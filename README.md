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

# Inspect all 15 out-of-the-box business presets
cargo run -p obaf-cli -- presets

# Generate a signed admin JWT token for terminal/curl debugging
cargo run -p obaf-cli -- token --role admin
```

---

## 📦 Docker Deployment

### Run with SQLite (Zero-Dependency)
```bash
docker compose up obaf-sqlite
```

### Run with PostgreSQL (High-Concurrency Profile)
```bash
docker compose --profile postgres up
```

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
| **Inventory & Batches** | `/api/v1/inventory` | Double-entry stock ledger, batch numbers, FEFO expiry tracking |
| **Orders & Workflows** | `/api/v1/orders` | Multi-item state machine (`CONFIRMED → COOKING → READY`) |
| **Payments & Settlements**| `/api/v1/payments` | Cash/Card/UPI transactions, End-of-Day revenue reconciliation |
| **Bookings & Slots** | `/api/v1/bookings` | Table/room reservations, appointments, hall scheduling |
| **Subscriptions** | `/api/v1/subscriptions`| Gym/tiffin/coworking recurring plans, pause/resume/cancel |
| **Billing & Taxes** | `/api/v1/billing` | GST (CGST/SGST/IGST) & VAT, split bills, thermal receipts, printable HTML invoices |
| **Webhooks Engine** | `/api/v1/webhooks` | Automated HTTP dispatch with HMAC-SHA256 signatures (`X-OBAF-Signature`) |
| **Real-time Event Hub** | `/api/v1/ws/events` | Axum WebSockets for live Kitchen Display (KDS) & cashier sync |

---

## 📂 Repository Workspace Structure

```text
Open-Business-Application-Framework/
├── Cargo.toml                 # Cargo workspace definition
├── Dockerfile                 # Multi-stage distroless/alpine build (~18MB)
├── docker-compose.yml         # SQLite and PostgreSQL profiles
├── config/
│   ├── default.toml           # Baseline config (host, port, DB URL)
│   └── presets/               # 15 industry domain setup seed templates
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
│       └── visitor_pass.json
├── crates/
│   ├── obaf-core/             # Platform types (TenantId, Money, Precision Decimal, Errors, Audit)
│   ├── obaf-database/         # SeaORM multi-DB abstraction & automatic schema migrations
│   └── obaf-api/              # Axum HTTP routes, Webhooks, Swagger UI, Admin introspection APIs
├── binaries/
│   ├── obaf-server/           # Standalone server daemon
│   └── obaf-cli/              # Developer companion CLI (`doctor`, `presets`, `token`)
├── tests/                     # Multi-domain end-to-end Python test suites
├── CONTRIBUTING.md            # Contributor guidelines, testing standards, and preset guidelines
├── SECURITY.md                # Vulnerability disclosure and multi-tenant security model
└── CHANGELOG.md               # Version history and milestone documentation
```

---

## 🗺️ Roadmap & Implemented Engines
- [x] **Phase 1: Foundation** (Cargo Workspace, SeaORM Multi-DB, SQLite/Postgres pooling, Swagger UI, Preset bootstrap API)
- [x] **Phase 2: Platform Kernel** (JWT Authentication, Argon2id, Role-Based Access Control, Tenant Isolation Middleware)
- [x] **Phase 3: Core Business Engines** (Catalog & Dynamic Schemas, Inventory Stock Ledger, Order State Machine, Invoicing & GST/VAT Tax Engine)
- [x] **Phase 4: Domain Validation** (Validated with live tests for all 11 business domain presets)
- [x] **Phase 5: Real-Time Event Hub** (Axum WebSockets on `/api/v1/ws/events` for live KDS & cashier sync)
- [x] **Phase 6: Webhooks & Tooling** (Asynchronous HTTP Webhook dispatcher with HMAC-SHA256 signatures, `obaf-cli` doctor & presets tool, native tenant data isolation test suite)

---

## 🤝 Contributing

Contributions are welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) to get started.

## 📄 License

Dual-licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE) or http://opensource.org/licenses/MIT)
at your option.
