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

## 2. Supported Domain Presets (Out-of-the-box Targets)

### Retail & Small Business Management
1. **Pharmacy & Medical Store:** Batch tracking, expiry tracking, low-stock alerts, GST billing.
2. **Gym & Fitness Club:** Membership plans, recurring renewals, daily check-ins, automated receipts.
3. **Laundry & Dry-Cleaning:** Tag number tracking, wash/iron/ready statuses, customer SMS/alerts, thermal receipts.
4. **Repair Shop & Service Center:** Electronic/vehicle job cards, spare parts consumption, technician assignments, invoice generation.

### Food, Hospitality & Events
5. **Hotel / Restaurant Order & POS:** Real-time table/room orders, Kitchen Order Tickets (KOT), split billing, daily revenue settlement.
6. **Cloud Kitchen & Tiffin Subscription:** Daily meal planning, delivery pause/resume, monthly billing, kitchen prep sheets.
7. **Event & Banquet Hall Booking:** Calendar availability, catering packages, advance deposits, settlement invoices.
8. **Table Reservation & Waitlist Queue:** Walk-in queue tracker, table assignment, live notification.

### Freelance, Office & Operations
9. **Freelancer Invoice & Expense:** Client management, billable hour tracking, expense ledger, PDF invoices.
10. **PG / Hostel Rent & Utility Splitter:** Room allocation, electricity meter reading logs, split utility calculation, tenant receipts.
11. **Visitor & Gate Pass Management:** Visitor log, badge/pass printing, check-in/out tracking.

---

## 3. Technology Stack

- **Language:** Rust (2021/2024 edition)
- **Web Framework:** `axum` (high-performance, tower middleware, async ecosystem)
- **Database & ORM:** `sea-orm` (Multi-database: PostgreSQL, MySQL, and SQLite with single entity definitions and migration engine)
- **Dynamic Field Validation:** `jsonschema` + `serde_json` (tenant-configurable custom attributes validated per tenant/business type)
- **Authentication & RBAC:** `jsonwebtoken` (JWT) + `argon2` password hashing (Role permissions: SuperAdmin, Admin, Manager, Cashier, Technician/Kitchen)
- **API Documentation:** `utoipa` + `utoipa-swagger-ui` (auto-generating OpenAPI 3.0 specs for seamless frontend client generation)
- **Real-Time Communication:** `axum` WebSockets / SSE (live updates for KOT, waitlists, job cards)
- **Document & Receipt Generation:** PDF generation (`printpdf` or `typst`) + ESC/POS thermal receipt formatting

---

## 4. Multi-Database & Container Strategy

Deployable in three distinct profiles via `DATABASE_URL`:
- **SQLite (`sqlite://data/app.db?mode=rwc`):** Zero-dependency, single-container mode for single-store offline setups, local shops, cafes.
- **PostgreSQL (`postgres://...`):** High-concurrency, multi-tenant cloud SaaS, enterprise branch networks.
- **MySQL / MariaDB (`mysql://...`):** Compatibility mode for standard hosting environments.

---

## 5. Modular Workspace Structure

```text
open-business-application-framework/
├── Cargo.toml                 # Cargo workspace definition
├── docker-compose.yml         # Container profiles (App + Postgres / MySQL / SQLite)
├── Dockerfile                 # Multi-stage distroless/alpine build
├── config/
│   ├── default.toml           # Base configuration
│   └── presets/               # Business domain setup seed templates
│       ├── restaurant_pos.json
│       ├── pharmacy.json
│       ├── gym_fitness.json
│       └── repair_shop.json
└── crates/
    ├── core-engine/           # Auth, RBAC, Multi-tenancy, Dynamic schemas, Events
    ├── catalog-inventory/     # Items, Variants, Batches, Expiry, Units, Stock alerts
    ├── order-workflow/        # Orders, Tickets, Configurable State Machines, KOT/Queues
    ├── booking-subscription/  # Resources/Units, Calendars, Time slots, Subscriptions
    ├── billing-tax/           # Tax engines (GST/VAT slabs), Split bills, PDF/Thermal printing
    └── api-server/            # Axum HTTP routes, WebSockets, OpenAPI/Swagger docs
```

---

## 6. Admin Panel Decoupling Strategy

1. **Bootstrap API (`POST /api/v1/setup/preset`):** One-click initialization with predefined presets (configuring status workflows, tax rates, custom JSON schemas).
2. **Schema & Config Discovery API (`GET /api/v1/meta/schema`):** Dynamic form and UI rendering instructions for decoupled frontends (React, Flutter, Next.js, Vue).
3. **Event Bus & Webhooks:** Emit transactional events (`order.created`, `stock.low`, `subscription.expiring`) to integrate external channels (WhatsApp, Email, Webhooks).
