# Open Business Application Framework (OBAF) Architecture Blueprint

A headless, multi-domain business operations, billing, and workflow engine built in Rust. Designed to power diverse business models (Retail, Hospitality, Services, Rentals, Subscriptions) through high configurability, dynamic schemas, and pluggable workflows—eliminating the need to rewrite backends across domains.

---

## 1. Core Vision & Architectural Philosophy

### Universal Domain Abstraction
Rather than building domain-siloed tables (e.g., `medicines`, `hotel_rooms`, `laundry_tags`), the system is structured around **6 generic primitives**:

| Primitive | Description | Hotel / Restaurant | Pharmacy | Gym / Tiffin | Repair / Laundry |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Resource / Unit** | Physical or logical allocation unit | Table #4, Room 101 | Shelf / Rack / Batch | Membership Slot, Meal Plan | Service Bay, Laundry Tag |
| **Catalog Item** | Goods, services, or billable assets | Paneer Tikka, Room Service | Paracetamol 500mg | Monthly Gold Plan, Lunch Pack | Screen Replacement, Dry Clean |
| **Customer / Party** | Client, patient, guest, or subscriber | Walk-in Guest, Room Guest | Patient / Customer | Member, Subscriber | Vehicle Owner, Client |
| **Order / Ticket** | Transactional or operational record | Table Order + KOT | Retail Sale Invoice | Subscription / Check-in | Job Card, Laundry Ticket |
| **Status Workflow** | Finite state machine per order/ticket | `Ordered → Kitchen → Served` | `Prescribed → Billed → Dispensed` | `Active → Paused → Expired` | `Received → In-Progress → Ready` |
| **Ledger & Bill** | Financial settlement & tax document | Split Bill, Room Charge | GST Batch Invoice | Recurring Invoice, Receipt | Advance Payment + Final Bill |

---

## 2. Supported Domain Presets (22 Out-of-the-Box Targets)

### Hospitality, Food & Dining
1. **Restaurant POS (`restaurant_pos`):** Dine-in tables, Kitchen Order Tickets (KOT), split billing, daily cashier settlement.
2. **Hotel Management (`hotel_management`):** Room allocation, reservations, check-in/out folios, guest ID verification.
3. **Cloud Kitchen & Tiffin (`cloud_kitchen_tiffin`):** Daily meal planning, delivery pause/resume, monthly subscriptions, prep sheets.
4. **Event & Banquet Hall (`banquet_hall`):** Date availability, catering packages, advance deposits, settlement invoices.
5. **Table Reservation & Queue (`table_waitlist`):** Walk-in queue tracker, table assignment, live SMS/notification.

### Retail, Pharmacy & Inventory
6. **Pharmacy & Medical (`pharmacy`):** Batch numbers, FEFO expiry tracking, drug schedules, GST tax slabs.
7. **Retail Grocery & FMCG (`retail_grocery`):** Barcode POS checkout, unit conversions (kg/g/pack), low-stock reorder triggers.
8. **Dairy & Daily Delivery (`dairy_milk_delivery`):** Route delivery logs, recurring subscriptions, milk fat/snf testing.

### Health, Wellness & Services
9. **Clinic & Doctor OPD (`clinic_opd`):** Patient queuing, doctor consultation appointments, vitals recording, prescription slips.
10. **Salon, Spa & Wellness (`salon_spa`):** Stylist scheduling, chair reservations, service packages, tipping ledgers.
11. **Gym & Fitness Club (`gym_fitness`):** Membership plans, attendance check-ins, auto-renewals, automated receipts.
12. **Coworking Space (`coworking_space`):** Hot desk booking, meeting room credits, recurring monthly tenant billing.

### Service, Repair & Field Operations
13. **Repair Shop & Job Cards (`repair_shop`):** Electronics/vehicle job cards, spare parts consumption, technician notes.
14. **Field Service & HVAC (`field_service`):** Dispatch scheduling, on-site service reports, technician time tracking.
15. **Laundry & Dry-Cleaning (`laundry`):** Garment tag tracking, wash/iron/ready statuses, customer pickup alerts.
16. **Courier & Local Fleet (`courier_fleet`):** Parcel waybills, delivery proof signatures, dispatch manifest logs.

### Rental, Education & Manufacturing
17. **Equipment Rental (`equipment_rental`):** Heavy equipment dispatch, security deposit holds, duration billing.
18. **Education & Coaching (`education_coaching`):** Student enrollment, batch schedules, course fee installments.
19. **Manufacturing Jobwork (`manufacturing_jobwork`):** Bill of Materials (BOM), raw material consumption, batch production runs.

### Professional & Property Management
20. **Freelance Invoice & Expense (`freelance_invoice`):** Billable hours, milestone billing, multi-currency invoices.
21. **PG & Hostel Rent (`pg_hostel`):** Room/bed allocation, electricity meter reading splits, rent receipts.
22. **Visitor & Gate Pass (`visitor_pass`):** Visitor badge generation, photo capture logs, check-in/out timestamps.

---

## 3. Technology Stack

- **Language:** Rust (2021 edition)
- **Web Framework:** `axum` 0.7 (high-performance, tower middleware, async Tokio ecosystem)
- **Database & ORM:** `sea-orm` 1.1 (Multi-database: PostgreSQL, MySQL, and SQLite with single entity definitions and migration engine)
- **Dynamic Field Validation:** `jsonschema` + `serde_json` (tenant-configurable custom attributes validated per tenant/business type)
- **Authentication & RBAC:** `jsonwebtoken` (JWT locked to HS256) + `argon2` (Argon2id password hashing) + RFC 6238 TOTP MFA
- **API Documentation:** `utoipa` + `utoipa-swagger-ui` (auto-generating OpenAPI 3.1 specifications)
- **Real-Time Communication:** `axum` WebSockets with RFC 6455 30s heartbeats (`/api/v1/ws/events`)
- **Document & Receipt Generation:** HTML printable invoices (`/invoice.html`) + ESC/POS thermal receipt formatting

---

## 4. Multi-Database & Container Strategy

Deployable in three distinct profiles via `DATABASE_URL`:
- **SQLite (`sqlite://data/app.db?mode=rwc`):** Zero-dependency, single-container mode for single-store offline setups, local shops, cafes with WAL mode.
- **PostgreSQL (`postgres://...`):** High-concurrency, multi-tenant cloud SaaS, enterprise branch networks with read replica round-robin routing.
- **MySQL / MariaDB (`mysql://...`):** Compatibility mode for standard enterprise hosting environments.

---

## 5. Modular Workspace Structure

```text
open-business-application-framework/
├── Cargo.toml                 # Cargo virtual workspace definition
├── Dockerfile                 # Multi-stage optimized Alpine container build
├── docker-compose.yml         # Container profiles (App + PostgreSQL / MySQL / SQLite)
├── config/
│   ├── default.toml           # Base configuration
│   └── presets/               # 22 Out-of-the-box industry domain presets
├── crates/
│   ├── obaf-core/             # Tenancy, Money, Auth/RBAC, dynamic schemas, events, rules
│   ├── obaf-database/         # Unified connection pool (TenantPoolRouter), SeaORM migrations
│   └── obaf-api/              # Axum HTTP & WebSocket routes, Super Admin SPA, OpenAPI 3.1
├── binaries/
│   ├── obaf-server/           # Production API daemon with embedded SPA & telemetry
│   ├── obaf-cli/              # CLI developer tool, preset generator, diagnostics, token utility
│   └── obaf-worker/           # Dedicated background worker daemon for sweeps and SSL renewal
└── sdk/
    ├── python/                # Official zero-dependency Python SDK (obaf-sdk)
    └── typescript/            # Official zero-dependency TypeScript SDK (@obaf/sdk)
```

---

## 6. Admin Panel Decoupling Strategy

1. **Bootstrap API (`POST /api/v1/setup/preset`):** One-click initialization with predefined presets (configuring status workflows, tax rates, custom JSON schemas).
2. **Schema & Config Discovery API (`GET /api/v1/meta/schema`):** Dynamic form and UI rendering instructions for decoupled frontends (React, Flutter, Next.js, Vue).
3. **Event Bus & Webhooks:** Emit transactional events (`order.created`, `stock.low`, `subscription.expiring`) to integrate external channels (WhatsApp, Email, Webhooks).
