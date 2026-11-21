# Changelog

All notable changes to the Open Business Application Framework (OBAF) project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.2.0] - 2026-11-21

### 🛡️ Super Admin Control Plane & Bank-Grade Security Hardening

### Added
- **Embedded Super Admin Web Console**: Zero-dependency SPA at `/admin` and `/superadmin` for organization management, API module toggles, telemetry, database diagnostics, and backup downloads.
- **Dual-Engine Authentication**: Root administrator login via email/password or Google Identity Services (GIS) SSO with cryptographic JWKS caching, strict audience (`aud`) verification, and debug-only mock token isolation.
- **RFC 6238 TOTP Multi-Factor Authentication**: Two-factor authentication with 30s time steps, 6-digit codes, Argon2id-hashed emergency backup codes, and intermediate `superadmin_mfa_pending` privilege isolation.
- **Automated Let's Encrypt SSL & ACME Engine**: Domain DNS verification, ACME challenge token sanitization, and automated background SSL certificate renewal (`obaf-worker`).
- **Real-Time Token Revocation Blacklisting**: SHA-256 fingerprint token revocation (`token_blacklist`) backed by a bounded in-memory `TtlCache` for instantaneous logout enforcement.
- **Zero-Trust Role-Based Access Control (RBAC)**: Strict permission boundaries preventing read-only `viewer` accounts from mutating state, and restricting credit notes, reports, and currency rates to administrators.
- **BOLA / IDOR Cashier Shift Isolation**: Bounded cashier shift closures to the shift owner with administrative override support.
- **Comprehensive Denial-of-Service Defense**: Bounded memory limits on `TokenBucketRateLimiter` and `TtlCache` (50,000 max), 2MB request body ceiling, pagination limit clamping (200 max), and `CatchPanicLayer` resilience.

### Security
- **Elimination of Insecure Default Credentials (CWE-1188)**: Mandatory `admin_password` ($\ge 8$ chars) across all onboarding endpoints (`/organizations`, `/setup/preset`, `/tenants`). Removed hardcoded fallback passwords.
- **Orphaned Organization Mitigation**: Pre-creation email validation and uniqueness checks against existing users, with automatic transactional rollback upon user creation failure.
- **Google SSO Audience Confusion Defense**: Immediate rejection with `403 Forbidden` if Google Client ID is unconfigured, and strict token `aud` claim matching against the configured client ID.
- **Anonymous Registration Lockdown**: Anonymous user registration is blocked once tenant users exist, preventing unauthorized account takeover.

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
- **22 Domain Presets (`config/presets/`):**
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
  - Hair Salon & Luxury Spa (`salon_spa.json`)
  - Retail Supermarket & Grocery (`retail_grocery.json`)
  - Hotel & Hospitality Management (`hotel_management.json`)
  - Education Academy & Coaching (`education_coaching.json`)
  - Manufacturing & Job Work (`manufacturing_jobwork.json`)
  - Courier Fleet & Logistics Dispatch (`courier_fleet.json`)
  - Dairy Farm & Milk Delivery (`dairy_milk_delivery.json`)
- **System Design Primer Resiliency & Fault Tolerance:**
  - `obaf_core::TtlCache`: Thread-safe Cache-Aside store with TTL expiration and automatic LRU capacity eviction.
  - `obaf_core::TokenBucketRateLimiter`: High-throughput token bucket rate limiter with continuous fractional refill for traffic shaping and DoS mitigation.
  - `obaf_core::CircuitBreaker`: 3-state finite state machine (`Closed`, `Open`, `HalfOpen`) preventing cascading failures to downstream services.
  - **Idempotency Keys**: `Idempotency-Key` header inspection on `create_order` and `record_payment`, returning recorded responses with `X-Idempotent-Replay: true`.
  - **Distributed Tracing**: `X-Request-Id` correlation propagation across middleware, logging spans, and response headers.
  - **Graceful Lifecycle**: Zero-downtime rolling deploys via `tokio::signal` graceful connection draining on `SIGINT` and `SIGTERM`.
- **Background Worker & Distributed Architecture:**
  - `binaries/obaf-worker`: Standalone daemon for background reconciliation sweeps and scheduled jobs.
- **Enterprise Commerce & Global Supply Chain:**
  - **Procurement & PO Lifecycle:** Formal purchase orders (`purchase_order`, `purchase_order_item`), automatic inward stock movements (`type = "PURCHASE"`), and cancellation protection.
  - **Promotions & Coupons:** Persistent promotional coupons (`coupon`) with percentage/fixed discounts, order thresholds, max discount caps, usage allowances, and atomic redemption.
  - **Multi-Currency Precision Exchange:** Configurable exchange rates (`currency_exchange_rate`) supporting direct and reciprocal 128-bit decimal currency conversions (`/api/v1/billing/exchange-rates`, `/api/v1/billing/convert-currency`).
  - **POS Cash Drawer Shifts:** Cashier shift management (`pos_shift`) with opening float, cash sales, refunds, expected cash, and counted cash variance reconciliation.
  - **Immutable Credit Notes:** Formal financial credit note issuance (`credit_note`) for returned merchandise and billing adjustments.
  - **Multi-Location Warehouses:** Multi-location stockrooms, warehouses, and fulfillment centers (`stock_location`).
- **Official Client SDKs:**
  - `sdk/python`: Zero-dependency, typed Python SDK (`obaf-sdk`) using standard library `urllib`.
  - `sdk/typescript`: Zero-dependency, typed TypeScript SDK (`@obaf/sdk`) using standard `fetch` with full type declarations.
- **Omni-Channel Capabilities & CLI Tooling:**
  - `/api/v1/search`: Unified multi-entity search endpoint.
  - `/api/v1/inventory/balance/:product_id`: Real-time stock balance calculation.
  - M2M API Keys (`X-API-Key`) with SHA-256 hashing.
  - Suppliers directory (`/api/v1/suppliers`).
  - Extended CLI commands: `obaf seed`, `obaf export`, `obaf backup`, `obaf restore`, `obaf migrate`, `obaf new-preset`.
- **Developer Experience, AI Integrations & High-Scale Resilience:**
  - Standardized `llms.txt` specification at repository root and native `GET /llms.txt` HTTP endpoint for AI coding assistants.
  - Interactive 60-second guided terminal demonstration (`scripts/tour.py`, `scripts/tour.sh`, `make tour`).
  - 1-Click Cloud Workspace configuration for GitHub Codespaces and VS Code Remote Containers (`.devcontainer/devcontainer.json`).
  - Real-time system health and latency diagnostics command (`obaf-cli status` and `make status`).
  - Active Token Bucket rate limiting middleware in `obaf-api` injecting `X-RateLimit-Remaining` and returning `HTTP 429 Too Many Requests` with `Retry-After`.
  - Comprehensive launch playbook and community submission copy (`docs/LAUNCH_PLAYBOOK.md`).
- **Developer Experience & Infrastructure:**
  - Multi-stage `Dockerfile` producing an ultra-lightweight ~25MB unprivileged (`USER obaf`) container image.
  - Cross-platform `Makefile` with targets for `setup`, `dev`, `worker`, `doctor`, `presets`, `check`, `fmt`, `test`, `test-e2e`, `test-all`, `sdk-ts`, `sdk-py`, and `docker-*`.
  - Comprehensive `docs/DEVELOPER_SETUP.md` and `docs/DEPLOYMENT_RUNBOOK.md` covering single-node edge vs clustered HA deployment topologies.
  - Complete Webhooks lifecycle with `GET`, `PATCH`, and `DELETE /api/v1/webhooks/:id` endpoints.
  - Complete SDK bindings across both Python and TypeScript client libraries for Webhooks, Bookings, Subscriptions, and Reports.
  - Zero-config local development: runs instantly on local SQLite without Docker or PostgreSQL daemons.
  - `docker-compose.yml` supporting instant multi-container deployment with PostgreSQL and background worker.
  - Complete CI workflow (`.github/workflows/ci.yml`) covering format, clippy, unit tests, container builds, and SDK verification.
