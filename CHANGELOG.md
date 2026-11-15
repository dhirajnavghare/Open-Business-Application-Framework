# Changelog

All notable changes to the Open Business Application Framework (OBAF) project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.0] - 2026-11-15

### 🚀 Initial Public Release

Welcome to the foundational release of **Open Business Application Framework (OBAF)** — the ultra-fast, headless business operations, billing, and workflow engine in Rust!

### Added
- **Core Workspace Architecture:**
  - `crates/obaf-core`: Tenant contexts, ISO 4217 Currency, exact `Money` decimal arithmetic, unified error types, audit records, and Argon2id/JWT security primitives.
  - `crates/obaf-database`: Unified connection pooling supporting SQLite, PostgreSQL, and MySQL via SeaORM, with automatic table creation and migrations.
  - `crates/obaf-api`: High-performance asynchronous REST API built with Axum, featuring OpenAPI 3.1 generation and Swagger UI.
  - `binaries/obaf-server`: Self-contained server daemon with configuration management (`default.toml` + environment variable overrides).
  - `binaries/obaf-cli`: Developer companion tool with `doctor`, `presets`, and `token` subcommands.
- **Universal Business Engines:**
  - **Catalog Engine:** Products, services, bill-of-materials, tax rates, and dynamic schema validation via `jsonschema`.
  - **Inventory & Stock Ledger:** Append-only ledger supporting double-entry stock transactions, batch tracking, and FEFO expiration dating (ideal for pharmacies and perishables).
  - **Orders & State Machine:** Relational order lifecycle supporting multi-party split-bills, kitchen order tickets (KOT), item statuses (`pending`, `confirmed`, `in_progress`, `completed`, `cancelled`).
  - **Billing & Tax Engines:** Precision dual-tax split (CGST + SGST or single VAT/Sales Tax), penny reconciliation for multi-party splits, and HTML printable tax invoice rendering (`/api/v1/billing/orders/{id}/invoice.html`).
  - **Payments Engine:** Multi-tender checkout (Cash, Card, UPI, Bank Transfer) and End-Of-Day (EOD) financial shift reconciliation.
  - **Bookings & Reservations:** Unit scheduling, time slots, conflict prevention, and deposit tracking.
  - **Subscriptions & Memberships:** Recurring billing intervals (`monthly`, `yearly`), auto-renewal, and grace periods.
  - **Real-Time WebSockets & Webhooks:** Live broadcast channel (`/api/v1/ws/events`) and webhook management with cryptographic HMAC-SHA256 signatures.
  - **Headless Schema Introspection:** Dynamic metadata discovery endpoint (`/api/v1/meta/schema`) for auto-generating frontend UIs (Refine, React Admin, Flutter).
- **15 Domain Presets (`config/presets/`):**
  - Restaurant POS (`restaurant_pos.json`)
  - Pharmacy & Healthcare (`pharmacy.json`)
  - Outpatient Clinic & Doctor Practice (`clinic_opd.json`)
  - Co-Working & Shared Office Space (`coworking_space.json`)
  - Equipment & Heavy Machinery Rental (`equipment_rental.json`)
  - Field Service & Appliance Repair (`field_service.json`)
  - Gym & Fitness (`gym_fitness.json`)
  - Laundry & Dry Cleaning (`laundry.json`)
  - Electronics & Auto Repair (`repair_shop.json`)
  - Cloud Kitchen & Tiffin (`cloud_kitchen_tiffin.json`)
  - Banquet & Event Hall (`banquet_hall.json`)
  - Table Waitlist & Queuing (`table_waitlist.json`)
  - Freelance & Agency Invoicing (`freelance_invoice.json`)
  - PG & Student Hostel (`pg_hostel.json`)
  - Visitor Pass & Gate Security (`visitor_pass.json`)
- **Developer Experience & Infrastructure:**
  - Multi-stage `Dockerfile` producing an ultra-lightweight ~18-25MB scratch/Debian-slim container.
  - Zero-config local development: runs instantly on local SQLite without Docker or PostgreSQL daemons.
  - `docker-compose.yml` supporting instant multi-container deployment with PostgreSQL.
  - Complete CI workflow (`.github/workflows/ci.yml`) covering format, clippy, unit tests, and container builds.
