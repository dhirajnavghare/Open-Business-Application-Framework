# OBAF System Architecture Specification

## 1. Executive Overview

The **Open Business Application Framework (OBAF)** is a modular, high-performance, headless business operations and billing engine written in Rust. It decouples core operational primitives (catalog, double-entry inventory, orders, billing, payments, bookings, subscriptions, webhooks) from industry-specific business rules through declarative JSON domain presets and compiled module extensions.

```
┌────────────────────────────────────────────────────────┐
│                   Admin & Client APIs                  │
│       REST Endpoints · WebSockets · OpenAPI Docs       │
└───────────────────────────┬────────────────────────────┘
                            │
┌───────────────────────────▼────────────────────────────┐
│                       obaf-api                         │
│   Axum 0.7 · Auth · Routes · Docs · Webhooks · Reports │
└─────────────┬────────────────────────────┬─────────────┘
              │                            │
┌─────────────▼──────────────┐ ┌───────────▼─────────────┐
│       obaf-database        │ │        obaf-core        │
│ SeaORM · Migrations · Pool │ │ Money · Security · Rules│
│   TenantPoolRouter         │ │  ObafExtension · Schemas│
└────────────────────────────┘ └─────────────────────────┘
```

---

## 2. Workspace & Crate Boundaries

OBAF follows strict inward-pointing dependencies to ensure compile-time boundary enforcement:

1. **`crates/obaf-core`**:
   - Zero database or web dependencies.
   - 128-bit fixed-point decimal arithmetic (`rust_decimal::Decimal`).
   - Tenant context primitives (`TenantId`, `BranchId`).
   - Dynamic JSON schema validation (`jsonschema`).
   - Argon2id password hashing and JWT token management.
   - Declarative pricing and business rules evaluation (`rules.rs`).
   - Open-source extension traits and registry (`extension.rs`).

2. **`crates/obaf-database`**:
   - Manages SeaORM entities and unified connection pooling.
   - Automated non-blocking schema migration runner (`run_migrations`).
   - `TenantPoolRouter`: Supports shared database clusters as well as dedicated per-tenant databases.

3. **`crates/obaf-api`**:
   - Axum 0.7 HTTP and WebSocket router with RFC 6455 30-second heartbeats.
   - Embedded Super Admin Single-Page Application (`/admin`, `/superadmin`).
   - Sensitive headers masking via `SetSensitiveRequestHeadersLayer` (`Authorization`, `Cookie`, `X-API-Key`, `X-OBAF-Signature`).
   - Dynamic API Module Gating middleware (`module_gating_middleware`).
   - Hardened defense-in-depth security headers (`nosniff`, `DENY`, `HSTS`, `CSP`, `CORP`, `COOP`, `Permissions-Policy`).
   - Let's Encrypt automated ACME HTTP-01 challenge responder (`/.well-known/acme-challenge/:token`).
   - Interactive OpenAPI/Swagger documentation (`ApiDoc`).
   - Sanitized database backup JSON downloads (`password_hash` redaction and secret filtering).
   - Standard Kubernetes health probes (`/health`, `/live`, `/ready`) and Prometheus metrics (`/metrics`).
   - Background HMAC-SHA256 webhook dispatcher with private IPv4 SSRF blocking.
   - Offline edge synchronization engine (`/api/v1/sync`).
   - Real-time SHA-256 cryptographic JWT token revocation store (`token_blacklist`).

4. **`binaries/obaf-server`**:
   - Production HTTP and WebSocket daemon.
   - Hierarchical configuration loader (`config/default.toml`, environment variables, `DATABASE_URL`).
   - Single-Root Super Administrator authentication with dual Google SSO and Argon2id password verification.
   - Production Credential Guard: Fatal startup abort in production mode if default password is in use.
   - Background Let's Encrypt automated SSL supervisor daemon (12-hour renewal interval).

5. **`binaries/obaf-cli`**:
   - Operational CLI tool (`obaf doctor`, `obaf presets`, `obaf token`, `obaf backup`, `obaf restore`, `obaf new-preset`, `obaf worker`).

6. **`binaries/obaf-worker`**:
   - Dedicated background worker daemon for periodic tenant maintenance sweeps and asynchronous queue processing.

7. **Client SDKs**:
   - `sdk/python`: Zero-dependency Python SDK (`obaf-sdk`).
   - `sdk/typescript`: Zero-dependency TypeScript SDK (`@obaf/sdk`).

---

## 3. Concurrency & Runtime Model

- Built on the multi-threaded **Tokio** runtime.
- Non-blocking asynchronous I/O across database pools, HTTP endpoints, and WebSockets.
- Request isolation with immutable shared application state (`Arc<AppState>`).
- Zero data-race guarantees enforced at compile time by Rust's type system.
