# Contributing to Open Business Application Framework (OBAF)

Thank you for your interest in contributing to **Open Business Application Framework (OBAF)**! OBAF is an ultra-fast, headless business operations, billing, and workflow engine engineered in Rust. We strive to be the definitive modern backend for business applications worldwide.

---

## 🎯 Architectural Principles & Commitments

When contributing to OBAF, keep these non-negotiables in mind:

1. **Headless & Decoupled by Design:** OBAF is strictly a backend engine. We do not bundle client-side JS or HTML frameworks. All UI is driven via our REST endpoints, WebSockets, and Schema Discovery API (`/api/v1/meta/schema`).
2. **Zero-Float Precision Arithmetic:** Never use `f32` or `f64` for currency, pricing, taxes, or stock units. Always use `rust_decimal::Decimal` and the `Money` domain primitive in `obaf-core`.
3. **Multi-Tenant Isolation First:** Every business entity belongs to a `tenant_id` (organization). No endpoint should ever leak or mutate records across tenant boundaries.
4. **Zero-Friction Contributor Onboarding:** Default local development must always work out of the box with zero external dependencies via SQLite. PostgreSQL and MySQL are available for production configurations.
5. **No Untyped JSON Blobs for Core Relational Data:** Relational models are strongly typed via SeaORM. Industry-specific customizations go through validated `metadata` fields checked against dynamic JSON Schemas using `jsonschema`.

---

## 🗂️ Workspace Architecture

```
.
├── crates/
│   ├── obaf-core/        # Domain primitives: Tenant, Money, Security, Errors, Schema Validation, Cache, RateLimiter
│   ├── obaf-database/    # SeaORM entities (21 tables), multi-database pool, TenantPoolRouter, migrations
│   └── obaf-api/         # Axum web server, REST handlers, OpenAPI docs, WebSockets, background webhook dispatch
├── binaries/
│   ├── obaf-server/      # Standalone API server executable with graceful SIGINT/SIGTERM drain
│   ├── obaf-cli/         # Developer CLI (`doctor`, `presets`, `token`, `backup`, `restore`, `seed`, `worker`)
│   └── obaf-worker/      # Dedicated background worker and scheduler daemon
├── config/
│   └── presets/          # 22 industry presets (Restaurant, Pharmacy, Gym, Hotel, Retail, etc.)
├── sdk/
│   ├── python/           # Official zero-dependency Python Client SDK (`obaf-sdk`)
│   └── typescript/       # Official zero-dependency TypeScript Client SDK (`@obaf/sdk`)
├── examples/
│   └── community_extension/ # Reference compiled Rust extension module (`ObafExtension`)
└── tests/                # End-to-end multi-domain test suites & benchmarks (Python & Rust)
```

---

## 🛠️ Setting Up Your Development Environment

For complete details on environment prerequisites, editors, and quickstart commands, see the [Developer Setup Guide](docs/DEVELOPER_SETUP.md). You can also run `make setup` to automatically initialize your environment.

### Prerequisites
- [Rust](https://rustup.rs/) (1.80 or higher) with `rustfmt` and `clippy` (`rustup component add rustfmt clippy`)
- Git
- Python 3.9+ (for running end-to-end integration test suites)
- Node.js 18+ (optional, for building or contributing to the TypeScript SDK)

### 1. Clone & Build
```bash
git clone https://github.com/dhirajnavghare/Open-Business-Application-Framework.git
cd Open-Business-Application-Framework

# Compile entire workspace
cargo build --workspace
```

### 2. Verify with CLI Doctor
```bash
cargo run -p obaf-cli -- doctor
```
This verifies:
- Local database connection (`sqlite://data/obaf.db?mode=rwc`)
- Table migration integrity (21 foundational entities)
- Configuration preset templates (22 industry presets)

### 3. Run the Development Server
```bash
cargo run -p obaf-server
```
Interactive Swagger UI documentation is immediately available at [http://127.0.0.1:8080/swagger-ui](http://127.0.0.1:8080/swagger-ui), and OpenAPI 3.1 JSON specification at [http://127.0.0.1:8080/api/v1/openapi.json](http://127.0.0.1:8080/api/v1/openapi.json).

### 4. Run the Background Worker (Optional)
```bash
cargo run -p obaf-worker -- --interval-secs 60
# Or run a single sweep:
cargo run -p obaf-worker -- --once
```

---

## 🧪 Testing Guidelines

Before opening a pull request, all automated checks and tests must pass:

### 1. Formatting
```bash
cargo fmt --all --check
```

### 2. Clippy (Zero Warnings Allowed)
```bash
cargo clippy --workspace --all-targets -- -D warnings
```

### 3. Rust Unit, Contract, and Integration Tests
```bash
cargo test --workspace --all-targets
```

### 4. End-to-End Workflow Test Suites
In a separate terminal while `obaf-server` is running on port 8080:
```bash
# Core Restaurant POS & Bill Splitting Flow
python3 tests/test_flow.py

# Pharmacy Expiry & FEFO Batch Dispensing
python3 tests/test_pharmacy.py

# Gym Memberships, Payments & Slot Bookings
python3 tests/test_services_gym_bookings.py

# Retail Barcode Checkout & Hotel Room Folios
python3 tests/test_retail_and_hotel.py

# Webhooks & HTML Printable Invoices
python3 tests/test_webhooks_and_invoices.py

# Procurement POs, Coupons & Multi-Currency Exchange
python3 tests/test_procurement_and_promotions_e2e.py

# Official Python SDK Client Tests
python3 tests/test_python_sdk.py

# Concurrency & Load Benchmark
python3 tests/benchmark_concurrency.py
```

### 5. TypeScript SDK Build & Typecheck (Optional)
```bash
cd sdk/typescript
npx -p typescript tsc
```

---

## 📦 Adding a New Domain Preset

Presets allow new industries to bootstrap within seconds without writing backend database schemas. To add a preset:

1. Scaffold using the CLI:
   ```bash
   cargo run -p obaf-cli -- new-preset boutique_jewelry --title "Boutique Jewelry Studio"
   ```
2. Customize the generated `config/presets/boutique_jewelry.json`:
   - `active_modules`: Array of enabled modules (`catalog`, `inventory`, `orders`, `billing`, etc.)
   - `workflow_states`: Ordered lifecycle states for tickets/orders.
   - `default_tax_slabs`: Standard regional GST/VAT categories.
   - `custom_fields`: JSON Schema defining domain-specific custom attributes.
3. Verify your new preset:
   ```bash
   cargo run -p obaf-cli -- presets
   ```

---

## 🔀 Pull Request Process

1. Fork the repository and create a branch from `main`:
   ```bash
   git checkout -b feat/your-feature-name
   ```
2. Keep commits concise and meaningful (following Conventional Commits e.g. `feat:`, `fix:`, `docs:`, `perf:`).
3. Update OpenAPI schemas in `crates/obaf-api/src/docs.rs` whenever adding new endpoints or request/response structs.
4. Ensure CI passes on all targets:
   ```bash
   cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
   ```
5. Submit the pull request with a clear description of the problem solved and testing steps verified.

Thank you for building the future of open-source business software! 🚀
