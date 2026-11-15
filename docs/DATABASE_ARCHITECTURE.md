# OBAF Database Architecture & Ledger Design

## 1. Multi-Engine Portability

OBAF is engineered for database portability across two primary database engines:
- **SQLite 3**: Zero-configuration, file-based embedded storage for development, edge POS, and single-business appliances (`sqlite://data/obaf.db?mode=rwc`).
- **PostgreSQL 16+**: High-concurrency clustering for multi-tenant SaaS deployments.

---

## 2. Automated Migration Runner

Migrations in `crates/obaf-database/src/migration.rs` use non-blocking DDL statements (`CREATE TABLE IF NOT EXISTS`) generated through SeaORM `Schema` builders:
- Forward-compatible additive changes.
- Safe for concurrent cluster nodes starting simultaneously.

---

## 3. Double-Entry Stock Ledger

Inventory balances are never modified via unchecked mutable updates. Instead, an append-only double-entry stock ledger (`stock_movements`) is maintained:

```
┌────────────────────────────────────────────────────────┐
│                    Stock Movement                      │
│   movement_type · quantity · product_id · tenant_id    │
└────────────────────────────────────────────────────────┘
  ▲ (+) RECEIPT/PURCHASE: Inward supplier delivery
  ▼ (-) SALE: Deducted automatically on order creation
  ▲ (+) RETURN: Credited automatically on order cancellation
  ▲/▼ ADJUSTMENT: Cycle count corrections
```

---

## 4. Immutable Audit Log

The `audit_logs` table stores an append-only record of administrative actions, status transitions, and payments:
- Columns: `id`, `tenant_id`, `actor_id`, `action`, `entity_type`, `entity_id`, `changes`, `ip_address`, `created_at`.
- Queryable via `GET /api/v1/admin/audit-logs`.
