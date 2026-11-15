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
│   ├── obaf-core/        # Domain primitives: Tenant, Money, Security, Errors, Schema Validation
│   ├── obaf-database/    # SeaORM entities, multi-database pool, automated migration runner
│   └── obaf-api/         # Axum web server, REST handlers, WebSockets, background webhook dispatch
├── binaries/
│   ├── obaf-server/      # Standalone API server executable
│   └── obaf-cli/         # Developer CLI (`doctor`, `presets`, `token`)
├── config/
│   └── presets/          # 11 industry presets (Restaurant, Pharmacy, Gym, Laundry, etc.)
└── tests/                # End-to-end multi-domain test suites (Python & Rust)
```

---

## 🛠️ Setting Up Your Development Environment

### Prerequisites
- [Rust](https://rustup.rs/) (1.80 or higher)
- Git
- Python 3.10+ (for running end-to-end integration test suites)

### 1. Clone & Build
```bash
git clone https://github.com/dhirajnavghare/Open-Business-Application-Framework.git
cd Open-Business-Application-Framework

# Compile workspace
cargo build --workspace
```

### 2. Verify with CLI Doctor
```bash
cargo run -p obaf-cli -- doctor
```

### 3. Run the Development Server
```bash
cargo run -p obaf-server
```
Interactive Swagger UI documentation is available at [http://127.0.0.1:8080/swagger-ui](http://127.0.0.1:8080/swagger-ui).

---

## 🧪 Testing Guidelines

Before opening a pull request, all automated checks and tests must pass:

### 1. Formatting
```bash
cargo fmt --all -- --check
```

### 2. Clippy (Zero Warnings Allowed)
```bash
cargo clippy --workspace --all-targets -- -D warnings
```

### 3. Rust Unit and Integration Tests
```bash
cargo test --workspace
```

### 4. End-to-End Workflow Suites
In a separate terminal while `obaf-server` is running:
```bash
python3 tests/test_flow.py
python3 tests/test_pharmacy.py
python3 tests/test_services_gym_bookings.py
python3 tests/test_webhooks_and_invoices.py
```

---

## 📦 Adding a New Domain Preset

Presets allow new industries to bootstrap within seconds. To add a preset:
1. Create a new JSON file in `config/presets/<preset_name>.json`.
2. Ensure it defines:
   - `name`: Human-readable name.
   - `domain`: Industry identifier (e.g., `hotel_hospitality`, `retail`, `field_service`).
   - `default_currency`: 3-letter ISO code (e.g., `USD`, `INR`, `EUR`).
   - `tax_rates`: Default tax configurations (e.g. CGST, SGST, VAT).
   - `catalog_schema`: JSON Schema defining custom product/service metadata attributes.
   - `order_schema`: JSON Schema defining custom ticket/order metadata attributes.
   - `seed_products`: Initial catalog items to populate upon bootstrapping.
3. Test the preset using:
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
4. Ensure CI passes on all targets.
5. Submit the pull request with a clear description of the problem solved and testing steps verified.

Thank you for building the future of open-source business software! 🚀
