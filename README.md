<div align="center">

```
  ___  ____    _    _____ 
 / _ \| __ )  / \  |  ___|
| | | |  _ \ / _ \ | |_   
| |_| | |_) / ___ \|  _|  
 \___/|____/_/   \_\_|    
```

# Open Business Application Framework (OBAF)

**The ultra-fast, headless business operations, billing, and workflow engine built in Rust.**  
*Single binary. Multi-tenant. Multi-database. 100% API-driven.*

[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.80%2B-orange.svg)](https://www.rust-lang.org)
[![CI](https://github.com/dhirajnavghare/Open-Business-Application-Framework/actions/workflows/ci.yml/badge.svg)](https://github.com/dhirajnavghare/Open-Business-Application-Framework/actions)
[![OpenAPI](https://img.shields.io/badge/OpenAPI-3.1-brightgreen.svg)](http://localhost:8080/swagger-ui)
[![Docker](https://img.shields.io/badge/Docker-Multi--Stage-2496ED.svg)](Dockerfile)

</div>

---

## ⚡ Why OBAF?

Most open-source business and ERP platforms (Odoo, MedusaJS, ERPNext) require heavy runtimes (Node.js/Python), multiple database daemons, and gigabytes of memory just to idle.

**OBAF changes that:**
- **Zero-Dependency Dev Mode:** Runs instantly as a single binary on **SQLite** (`dev.db`). No Docker or external daemons required to start developing.
- **Enterprise-Ready Production:** Switches to **PostgreSQL** or **MySQL** with high-concurrency connection pools with a single environment variable change.
- **100% Headless Admin API:** The entire admin panel is an API. Comes with built-in **Schema Introspection** (`/api/v1/meta/schema`) so frontend developers can plug in [Refine.dev](https://refine.dev), Next.js, or Flutter without writing backend logic.
- **Microsecond Latency & Minimal Footprint:** Written in async Rust ([Axum](https://github.com/tokio-rs/axum) + [SeaORM](https://www.sea-ql.org/SeaORM/)). Compiles to an **~18MB static binary** that idles at **< 15MB RAM**.

---

## 🏛️ The 6 Universal Primitives

Instead of building hardcoded, isolated tables for each industry, OBAF abstracts 80% of business operations into 6 core primitives:

| Universal Primitive | Hotel / Restaurant | Pharmacy | Gym / Tiffin | Repair / Field Service |
| :--- | :--- | :--- | :--- | :--- |
| **Resource / Unit** | Table #4, Room 101 | Shelf / Rack | Membership Slot | Service Bay / Tech Slot |
| **Catalog Item** | Paneer Tikka, Room Service | Paracetamol 500mg | Monthly Gold Plan | Screen Repair, Labor Hour |
| **Customer / Party** | Guest / Diner | Patient / Customer | Member / Subscriber | Vehicle Owner / Client |
| **Order / Ticket** | Table Order + KOT | Retail Sale Invoice | Subscription / Check-in | Job Card / Service Ticket |
| **Status Workflow** | `Ordered → Cooking → Served` | `Prescribed → Dispensed` | `Active → Expired` | `Diagnosing → Ready` |
| **Ledger & Bill** | Split Bill + GST Invoice | GST Batch Invoice | Recurring Receipt | Advance + Final Invoice |

---

## 🚀 Quickstart in 30 Seconds

### Prerequisites
- [Rust](https://rustup.rs/) (1.80+)

### 1. Clone & Run
```bash
git clone https://github.com/dhirajnavghare/Open-Business-Application-Framework.git
cd Open-Business-Application-Framework

# Run with zero-config SQLite
cargo run -p obaf-server
```

You will see:
```text
  ___  ____    _    _____ 
 / _ \| __ )  / \  |  ___|
| | | |  _ \ / _ \ | |_   
| |_| | |_) / ___ \|  _|  
 \___/|____/_/   \_\_|    

 Open Business Application Framework (OBAF) v0.1.0
 High-Performance Headless Business & Billing Engine in Rust
 ────────────────────────────────────────────────────────────
 🚀 Server running at: http://127.0.0.1:8080
 📖 Swagger UI Docs:   http://127.0.0.1:8080/swagger-ui
 ⚡ OpenAPI JSON Spec: http://127.0.0.1:8080/api/v1/openapi.json
 🩺 Health Check:      http://127.0.0.1:8080/health
 ────────────────────────────────────────────────────────────
```

Open **[http://127.0.0.1:8080/swagger-ui](http://127.0.0.1:8080/swagger-ui)** in your browser to interactively test endpoints.

## 💻 Developer CLI (`obaf-cli`)

OBAF includes a standalone CLI binary for instant setup, database health checks, preset listing, and token generation:

```bash
# Verify environment, database connectivity, and migrations
cargo run -p obaf-cli -- doctor

# Inspect all 22 out-of-the-box business domain presets
cargo run -p obaf-cli -- presets

# Scaffold a new custom domain preset
cargo run -p obaf-cli -- new-preset dental_clinic

# Seed an instant demo tenant with products, customers, and admin token
cargo run -p obaf-cli -- seed restaurant_pos

# Export tenant data to formatted JSON
cargo run -p obaf-cli -- export products

# Generate a signed admin JWT token for terminal/curl debugging
cargo run -p obaf-cli -- token --role admin

# Backup and restore database
cargo run -p obaf-cli -- backup
cargo run -p obaf-cli -- restore --source data/backup_20261115.db
```

---

## 📦 Deployment Options

OBAF offers two fully documented, production-ready deployment topologies:

### Option 1: Single Deployment (Edge, Kiosk, or Standalone VPS)
Ideal for edge POS systems, branch stores, single-server VPS ($5 droplets), and embedded IoT appliances:
* **Architecture:** Single static binary or lightweight Docker container (~25MB, < 35MB RAM).
* **Storage:** Zero-dependency embedded SQLite with WAL (`Write-Ahead Logging`) mode.
* **Launch:**
  ```bash
  # Via Docker Compose (Recommended)
  docker compose up -d obaf-sqlite

  # Or via standard Docker run:
  docker run -d --name obaf-server -p 8080:8080 -v $(pwd)/data:/app/data ghcr.io/your-org/obaf:latest
  ```

### Option 2: Scalable Deployment (High-Availability Clustered Cloud)
Designed for high-throughput multi-tenant SaaS, enterprise retail chains, and high-concurrency billing engines:
* **Architecture:** Horizontally scaled stateless API pods behind L7 Load Balancer + Clustered PostgreSQL 16+ with connection pooling + Dedicated `obaf-worker` background daemons.
* **Auto-Scaling:** Kubernetes Horizontal Pod Autoscaler (HPA) and Pod Disruption Budgets (PDB).
* **Launch with Docker Compose:**
  ```bash
  # Launch Clustered PostgreSQL + Stateless API + Background Worker
  docker compose --profile postgres --profile worker up -d
  ```
* **Production Kubernetes (K8s):** Ready-to-apply manifests for `Deployment`, `Service`, `HPA`, `PDB`, `ConfigMap`, and `Secret` are provided in the [Production Deployment Runbook](docs/DEPLOYMENT_RUNBOOK.md).

For full details on PostgreSQL tuning, connection pool math, NGINX reverse proxying, Prometheus alerting, and disaster recovery, see [`docs/DEPLOYMENT_RUNBOOK.md`](docs/DEPLOYMENT_RUNBOOK.md).

---

## 🛠️ API & Admin Capabilities

### 1. One-Click Preset Bootstrapping
Initialize an organization and tenant with a pre-configured domain preset in one call:
```bash
curl -X POST http://127.0.0.1:8080/api/v1/admin/setup/preset \
  -H "Content-Type: application/json" \
  -d '{
    "organization_name": "Spice Bistro",
    "slug": "spice-bistro",
    "preset": "restaurant_pos",
    "admin_email": "admin@spicebistro.com",
    "admin_name": "Alex Chef"
  }'
```

### 2. Dynamic Schema Introspection (For Headless UIs)
```bash
curl -s http://127.0.0.1:8080/api/v1/meta/schema
```
Returns field definitions, input types, and permitted actions so your frontends (React/Next.js/Flutter) can render forms and tables dynamically.

### 3. Generate TypeScript SDK
Because OBAF publishes an OpenAPI 3.1 specification at `/api/v1/openapi.json`, frontend clients can be auto-generated in seconds:
```bash
npx @hey-api/openapi-ts -i http://localhost:8080/api/v1/openapi.json -o ./client
```

### 4. Reusable Business APIs

| Engine | Namespace | Key Capabilities |
| :--- | :--- | :--- |
| **Authentication & RBAC** | `/api/v1/auth` | Argon2id password hashing, JWT bearer tokens, role checks |
| **Customers & Parties** | `/api/v1/customers` | Patient/member/client profiles, GSTIN/tax IDs, addresses |
| **Catalog & Products** | `/api/v1/catalog` | Dynamic custom metadata, exact decimal pricing, categories |
| **Inventory & Batches** | `/api/v1/inventory` | Double-entry stock ledger, live balance calculation, FEFO expiry tracking |
| **Warehouses & Locations**| `/api/v1/inventory/locations` | Multi-location stockrooms, fulfillment centers, and warehouse codes |
| **Suppliers & Vendors** | `/api/v1/suppliers` | Vendor directory, procurement contact persons, tax IDs |
| **Procurement & POs** | `/api/v1/procurement` | Purchase order lifecycle (`DRAFT → SUBMITTED → RECEIVED`), auto-inward stock movements |
| **Orders & Workflows** | `/api/v1/orders` | Multi-item state machine with `Idempotency-Key` deduplication |
| **Payments & Settlements**| `/api/v1/payments` | Cash/Card/UPI transactions, refunds, idempotency, EOD settlement |
| **POS Cash Drawer** | `/api/v1/pos/shifts` | Cashier drawer shifts, opening floats, cash sales, refunds, expected cash & variance |
| **Bookings & Slots** | `/api/v1/bookings` | Table/room reservations, appointments, hall scheduling, check-in |
| **Subscriptions** | `/api/v1/subscriptions`| Recurring plans, pause/resume/cancel, attendance check-in |
| **Promotions & Coupons**| `/api/v1/promotions` | Promo codes, percentage/fixed discounts, order thresholds, max caps & atomic redemption |
| **Billing & Rules** | `/api/v1/billing` | GST & VAT, split bills, thermal receipts, HTML invoices, dynamic pricing rules |
| **Credit Notes** | `/api/v1/billing/credit-notes`| Immutable financial credit notes for returned merchandise and invoice corrections |
| **Multi-Currency** | `/api/v1/billing/exchange-rates`| Precision 128-bit decimal currency conversion across direct and reciprocal pairings |
| **Omni-Channel Search**| `/api/v1/search` | Fast unified search across products, orders, customers, and bookings |
| **Edge Offline Sync** | `/api/v1/sync` | Offline POS batch synchronization with monotonic versioning |
| **Webhooks Engine** | `/api/v1/webhooks` | Automated HTTP dispatch with HMAC-SHA256 signatures (`X-OBAF-Signature`) |
| **Real-time Event Hub** | `/api/v1/ws/events` | Axum WebSockets for live Kitchen Display (KDS) & cashier sync |
| **SaaS Multi-Tenancy** | `/api/v1/admin/tenants`| Organization quota metering, tenant database routing (`TenantPoolRouter`) |

---

## 📂 Repository Workspace Structure

```text
Open-Business-Application-Framework/
├── Cargo.toml                 # Cargo workspace definition
├── Dockerfile                 # Multi-stage release build (~25MB)
├── docker-compose.yml         # SQLite, PostgreSQL, and Worker profiles
├── config/
│   ├── default.toml           # Baseline config (host, port, DB URL)
│   └── presets/               # 22 industry domain setup seed templates
│       ├── restaurant_pos.json
│       ├── pharmacy.json
│       ├── clinic_opd.json
│       ├── coworking_space.json
│       ├── equipment_rental.json
│       ├── field_service.json
│       ├── gym_fitness.json
│       ├── laundry.json
│       ├── repair_shop.json
│       ├── cloud_kitchen_tiffin.json
│       ├── banquet_hall.json
│       ├── table_waitlist.json
│       ├── freelance_invoice.json
│       ├── pg_hostel.json
│       ├── visitor_pass.json
│       ├── salon_spa.json
│       ├── retail_grocery.json
│       ├── hotel_management.json
│       ├── education_coaching.json
│       ├── manufacturing_jobwork.json
│       ├── courier_fleet.json
│       └── dairy_milk_delivery.json
├── crates/
│   ├── obaf-core/             # Platform types, TtlCache, TokenBucket, CircuitBreaker, Decimal Money
│   ├── obaf-database/         # SeaORM multi-DB abstraction, migrations & TenantPoolRouter
│   └── obaf-api/              # Axum HTTP routes, Webhooks, Idempotency, Swagger UI, Admin APIs
├── binaries/
│   ├── obaf-server/           # Production API daemon with graceful SIGINT/SIGTERM shutdown
│   ├── obaf-cli/              # CLI tooling (`doctor`, `presets`, `seed`, `export`, `backup`)
│   └── obaf-worker/           # Dedicated background worker and scheduler daemon
├── sdk/
│   ├── python/                # Official zero-dependency Python Client SDK (`obaf-sdk`)
│   └── typescript/            # Official zero-dependency TypeScript Client SDK (`@obaf/sdk`)
├── examples/
│   └── community_extension/   # Reference module implementation (`ObafExtension`)
├── tests/                     # Multi-domain end-to-end Python test suites & benchmarks
├── docs/                      # Architectural specifications, ADRs & System Design Primer alignment
├── CONTRIBUTING.md            # Contributor guidelines, testing standards, and preset guidelines
├── SECURITY.md                # Vulnerability disclosure and multi-tenant security model
└── CHANGELOG.md               # Version history and milestone documentation
```

---

## 🗺️ Roadmap & Implemented Phases
- [x] **Phase 0: Project Definition** (Dual MIT/Apache-2.0 licensing, pinned `rust-toolchain.toml`, ADRs)
- [x] **Phase 1: Foundation** (Cargo Workspace, SeaORM Multi-DB, SQLite/Postgres pooling, Swagger UI, Preset bootstrap API)
- [x] **Phase 2: Platform Kernel** (JWT Authentication, Argon2id, Role-Based Access Control, Tenant Isolation Middleware)
- [x] **Phase 3: Core Business Engines** (Catalog, Inventory Stock Ledger, Order State Machine, Invoicing & GST/VAT Tax Engine)
- [x] **Phase 4: Financial & Operations** (Payments, refunds, split billing, subscriptions, resource bookings, audit logs)
- [x] **Phase 5: Domain Validation** (Validated with live tests for all 22 business domain presets)
- [x] **Phase 6: Extension Readiness** (`ObafExtension` trait, `ExtensionRegistry`, `LoyaltyRewardsExtension`, CLI scaffolding)
- [x] **Phase 7: Production Hardening** (Prometheus `/metrics`, security headers, 2,460+ req/s benchmarks, DB backup/restore)
- [x] **Phase 8: Domain Expansion** (Retail grocery, hotel reservation folio, manufacturing job cards, fleet dispatch)
- [x] **Phase 9: Advanced Platform Capabilities** (Offline edge sync, dynamic rules engine, tenant quota metering, `TenantPoolRouter`, `obaf-worker` daemon)
- [x] **Phase 10: Enterprise Commerce & Global Supply Chain** (Purchase orders & procurement lifecycle, promotions & coupons, multi-currency exchange, POS cash drawer shifts, credit notes, official Python & TypeScript SDKs)

---

## 🤝 Contributing & Developer Setup

Contributions are welcome! Please read our [Developer Setup Guide](docs/DEVELOPER_SETUP.md) and [CONTRIBUTING.md](CONTRIBUTING.md) to get started.

## 📄 License

Dual-licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE) or http://opensource.org/licenses/MIT)
at your option.
