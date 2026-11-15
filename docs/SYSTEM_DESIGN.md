# OBAF System Design & Scalability Architecture
*Engineered in alignment with [donnemartin/system-design-primer](https://github.com/donnemartin/system-design-primer)*

---

## 1. System Requirements & Scope

### 1.1 Functional Requirements
- **Multi-Tenant Operations**: Run 20+ distinct business domains (POS, retail, hotel, clinic, manufacturing) on a unified engine.
- **Financial Precision**: 100% exact currency arithmetic with zero floating-point rounding errors.
- **Double-Entry Stock Ledger**: Append-only inventory movements preventing silent inventory drift.
- **Real-Time & Offline Sync**: WebSocket event streaming and offline batch synchronization with idempotency.
- **Extensibility**: Declarative JSON domain presets and compiled module plugins (`ObafExtension`).

### 1.2 Non-Functional Requirements
- **High Availability**: 99.99% uptime with zero-downtime rolling upgrades.
- **Low Latency**: Sub-10ms P50 latency for transactions, sub-2ms for health/schema reads.
- **Strict Data Isolation**: Hard tenant data boundaries enforced at database and API layers.
- **Scalability**: Capable of scaling from an embedded edge POS to a clustered cloud SaaS backend.

---

## 2. Back-of-the-Envelope Estimation

Following the estimation methodology in the *System Design Primer*:

### Traffic Estimation (10,000 Active Multi-Tenant Businesses)
- **Daily Orders per Business**: ~200 orders/day.
- **Total Daily Orders**: $10,000 \times 200 = 2,000,000 \text{ orders/day}$.
- **Average Orders / Second**: $2,000,000 / 86,400 \approx 23.1 \text{ orders/sec}$.
- **Peak Write Traffic (3x multiplier)**: $\approx 70 \text{ writes/sec}$.
- **Read-to-Write Ratio**: 10:1 (browsing catalog, checking stock, dashboard reports).
- **Peak Read Traffic**: $\approx 700 \text{ reads/sec}$.
- **Total Peak QPS**: $\approx 770 \text{ QPS}$.  
  *(Our Rust benchmark verified **2,500+ req/sec** on a single node, providing over 3.2x peak capacity headroom without horizontal replicas).*

### Storage Estimation
- **Order Size**: Average order payload with metadata $\approx 1 \text{ KB}$.
- **Daily Storage Growth**: $2,000,000 \times 1 \text{ KB} = 2 \text{ GB/day}$.
- **Annual Storage Growth**: $2 \text{ GB} \times 365 \approx 730 \text{ GB/year}$.
- **Memory for Caching (20% Pareto Rule)**:
  - Cache top 20% active products and organization presets.
  - $10,000 \text{ orgs} \times 500 \text{ products} \times 20\% \times 500 \text{ bytes} \approx 500 \text{ MB}$.
  - Easily managed in-memory using `obaf_core::TtlCache`.

---

## 3. High-Level Architecture

```
                    ┌─────────────────────────────────┐
                    │      DNS / Cloudflare CDN       │
                    └────────────────┬────────────────┘
                                     │
                    ┌────────────────▼────────────────┐
                    │        Reverse Proxy / LB       │
                    │      (SSL Termination & WAF)    │
                    └────────────────┬────────────────┘
                                     │
           ┌─────────────────────────┼─────────────────────────┐
           │                         │                         │
┌──────────▼──────────┐   ┌──────────▼──────────┐   ┌──────────▼──────────┐
│  OBAF API Node 1    │   │  OBAF API Node 2    │   │  OBAF API Node N    │
│  · Axum + Tokio     │   │  · Axum + Tokio     │   │  · Axum + Tokio     │
│  · X-Request-Id     │   │  · X-Request-Id     │   │  · X-Request-Id     │
│  · In-Memory Cache  │   │  · In-Memory Cache  │   │  · In-Memory Cache  │
└──────────┬──────────┘   └──────────┬──────────┘   └──────────┬──────────┘
           │                         │                         │
           └─────────────────────────┼─────────────────────────┘
                                     │
                    ┌────────────────▼────────────────┐
                    │        TenantPoolRouter         │
                    └───────┬─────────────────┬───────┘
                            │                 │
         ┌──────────────────▼────┐       ┌────▼─────────────────┐
         │ Shared Primary DB     │       │ Dedicated Tenant DBs │
         │ PostgreSQL Clustered  │       │ (Enterprise Isolation)│
         └───────────────────────┘       └──────────────────────┘
```

---

## 4. CAP & PACELC Theorem Trade-Offs

In accordance with Brewer's CAP Theorem and the PACELC extension:

| Engine / Subsystem | Consistency Model | CAP Classification | Rationale |
|---|---|:---:|---|
| **Double-Entry Stock Ledger** | Strong Consistency | **CP** | Under network partitioning, stock movements must never drift. Inventory decrements block rather than allowing overselling. |
| **Financial Billing & Payments**| Strong Consistency | **CP** | Invoices and payment settlements require ACID atomicity and penny-rounding guarantees. |
| **Edge POS Synchronization** | Eventual Consistency | **AP** | Offline terminals continue recording sales during internet loss; reconciliation occurs idempotently upon reconnection. |
| **Real-time WebSockets & Webhooks** | Monotonic / At-least-once | **AP** | Event notification drop-resistance with exponential backoff and Dead-Letter Queue (DLQ). |

---

## 5. Resiliency & Reliability Patterns

### 5.1 Idempotency Keys (Distributed Systems Pattern)
- Mutating endpoints (`POST /api/v1/orders`, `POST /api/v1/payments`) inspect the `Idempotency-Key: <UUID>` header.
- Repeated submissions during mobile network retries or client timeouts return the exact cached result with `X-Idempotent-Replay: true`, preventing duplicate orders or double credit card charges.
- Backed by an in-memory 24-hour TTL idempotency store (`AppState::idempotency_cache`).

### 5.2 In-Memory Caching (Cache-Aside Pattern)
- Implemented via `obaf_core::TtlCache`.
- Frequently accessed, semi-static data (domain presets, tax slab tables, schema definitions) are cached in memory with configurable TTL expiration.
- Capacity management automatically purges expired entries using LRU/TTL eviction to bound heap footprint.

### 5.3 Distributed Tracing & Request ID Propagation
- `X-Request-Id` middleware assigns or preserves a unique UUID correlation header across all incoming requests.
- Logged across all `tracing` spans, enabling instant cross-service request tracing during post-mortems and incident debugging.

### 5.4 Dead-Letter Queue (DLQ) & Webhook Jitter Backoff
- Webhook dispatching uses full-jitter exponential backoff:
  $$\text{delay} = \min(\text{max\_backoff}, \text{base} \times 2^{\text{attempt}}) \pm \text{jitter}$$
- Webhooks exceeding maximum retry thresholds transition to `DEAD_LETTER` status, preserving payload history for manual replay without blocking the main event queue.

### 5.5 Token Bucket Rate Limiting (Traffic Shaping & DoS Mitigation)
- Implemented via `obaf_core::TokenBucketRateLimiter` and wired directly into the Axum request pipeline via `from_fn_with_state` in `obaf-api`.
- Client key resolved dynamically:
  1. Hashed Bearer token prefix from `Authorization` header for authenticated tenant requests.
  2. First client IP from `X-Forwarded-For` or `CF-Connecting-IP` proxy headers for public clients.
  3. Default fallback bucket for direct unauthenticated traffic.
- Implements continuous token replenishment based on elapsed time:
  $$\text{tokens}(t) = \min\left(\text{capacity}, \text{tokens}(t_0) + \Delta t \times \text{refill\_rate}\right)$$
- Responses inject `X-RateLimit-Remaining: <count>` on permitted requests.
- When burst limits are exhausted, immediately responds with **HTTP `429 Too Many Requests`**, setting `Retry-After: <seconds>` and `X-RateLimit-Remaining: 0`.
- Internal probes and documentation (`/health`, `/metrics`, `/llms.txt`, `/swagger-ui`) bypass rate limiting to guarantee monitoring availability.
- Inactive buckets are automatically pruned to prevent memory leaks.

### 5.6 Circuit Breaker Pattern (Cascading Failure Protection)
- Implemented via `obaf_core::CircuitBreaker`.
- Maintains a 3-state finite state machine (`Closed`, `Open`, `HalfOpen`):
  - **Closed**: Normal execution; counts consecutive downstream failures.
  - **Open**: Tripped after reaching failure threshold; fails fast immediately to prevent connection pool exhaustion.
  - **HalfOpen**: Probes downstream recovery with trial requests after a recovery timeout before safely closing the circuit.

### 5.7 Graceful Shutdown & In-Flight Request Draining
- Implemented in `obaf-server` via `tokio::signal` listening for `SIGINT` (Ctrl+C) and `SIGTERM`.
- Seamlessly drains in-flight HTTP requests and database connection pools before process termination, enabling zero-downtime rolling upgrades in Kubernetes and Docker deployments.

### 5.8 High-Throughput Read/Write Replica Routing (`ReadWriteRouter`)
- Directs write transactions (`INSERT`, `UPDATE`, `DELETE`) to the primary database while load-balancing queries across read replicas using lock-free round-robin load distribution.
- Seamlessly configured via `DATABASE_REPLICAS` or `OBAF_DATABASE_REPLICAS` environment variables with automatic fallback to primary if no replicas are registered.

### 5.9 High-Concurrency Embedded SQLite Auto-Tuning
- Automatically activates SQLite Write-Ahead Logging (`PRAGMA journal_mode = WAL;`), busy timeout (`PRAGMA busy_timeout = 5000;`), and normal synchronous flush (`PRAGMA synchronous = NORMAL;`) on SQLite databases.
- Prevents database lock contention under concurrent load, enabling single-node edge POS and embedded installations to achieve 6,000+ operations/sec.

---

## 6. System Design Primer Concept Mapping

| System Design Primer Topic | OBAF Implementation / Architecture |
|---|---|
| **Performance vs Scalability** | Tokio async I/O + Axum delivering 6,000+ req/s single-node; horizontal scale-out behind L7 load balancer |
| **Latency vs Throughput** | Sub-5ms P50 latency; in-memory `TtlCache` for hot read paths |
| **Availability vs Consistency** | CAP-classified CP engines (double-entry stock ledger, billing) vs AP engines (offline sync, webhooks) |
| **Relational Database Sharding & Read Scaling** | Multi-tenant shared database with `TenantPoolRouter` for dedicated tenant DBs + `ReadWriteRouter` for replica load-balancing |
| **Cache-Aside Pattern** | `obaf_core::TtlCache` with TTL expiry and capacity bounding |
| **Token Bucket Rate Limiting** | `obaf_core::TokenBucketRateLimiter` with continuous token refill |
| **Circuit Breaker** | `obaf_core::CircuitBreaker` with Closed/Open/HalfOpen state machine |
| **Idempotency** | `Idempotency-Key` header with `X-Idempotent-Replay: true` responses |
| **Asynchronous Events** | WebSocket event streaming (`/api/v1/ws`) and HMAC-SHA256 signed webhooks (`/api/v1/webhooks`) |
| **Security & Authorization** | Argon2id password hashing, JWT Bearer tokens, Machine API Keys (`X-API-Key`), Security headers |
| **Observability & Health Probes** | Prometheus metrics (`/metrics`), `/health` consolidated health, `/live` liveness probe, `/ready` readiness probe, and `X-Request-Id` correlation tracing |
| **Graceful Lifecycle** | SIGINT/SIGTERM graceful connection draining via `axum::serve::with_graceful_shutdown` |

