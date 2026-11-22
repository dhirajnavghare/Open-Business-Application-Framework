# OBAF Multi-Tenancy & Authorization Design

## 1. Tenancy Model

OBAF enforces strict multi-tenant isolation at the application layer and database layer:

- **`TenantId`**: A strongly-typed 128-bit UUID identifier (`obaf_core::TenantId`) representing an Organization boundary.
- **`BranchId`**: Optional sub-division representing physical stores, clinic branches, or franchise outlets.

---

## 2. Request Tenant Context & Extraction

Every authenticated request carries a signed JWT token containing claims:
```json
{
  "sub": "user-uuid",
  "tenant_id": "organization-uuid",
  "role": "admin",
  "exp": 1794760139
}
```

The Axum extractor `AuthUser` automatically verifies the JWT signature and extracts `auth.tenant_id`. Every SeaORM database query must filter on `tenant_id.eq(auth.tenant_id.0)`:

```rust
let orders = order::Entity::find()
    .filter(order::Column::TenantId.eq(auth.tenant_id.0))
    .all(&state.db)
    .await?;
```

Cross-tenant access attempts immediately fail with empty results or `404 Not Found`.

---

## 3. Dedicated Database Routing (`TenantPoolRouter`)

For enterprise tenants requiring dedicated databases or physical data residency compliance:
- `TenantPoolRouter` maps `TenantId` to an isolated database connection pool.
- Standard tenants share the primary database cluster with table-level tenant isolation.

---

## 4. Platform Root Super Administrator

The platform has exactly one top-level Platform Super Administrator configured strictly via environment variables:
- **Identifier:** Well-known static UUID `SUPERADMIN_USER_ID = 00000000-0000-0000-0000-000000000001` with `TenantId(Uuid::nil())`.
- **Role:** `"superadmin"`
- **Universal Bypass:** Bypasses tenant isolation and module gating to manage all organizations, inspect system metrics, download database backups, and manage SSL certificates.
- **Dedicated Scope:** Restricts access to `/api/v1/superadmin/*` routes; standard tenant endpoints require tenant administrative credentials.

---

## 5. Dynamic API Module Gating

Super Admins can granularly select which of the 15 API engines each tenant organization can access (e.g. enabling `catalog` and `orders`, while disabling `bookings` or `pos`):
- **Storage:** Persisted as a JSON array of strings in the `active_modules` column of the `organization` table.
- **Middleware:** `module_gating_middleware` extracts the caller's `organization_id`, inspects the target URL namespace, and returns `403 Forbidden` (`{"code": "MODULE_DISABLED", "error": "Module Disabled"}`) if the organization has not been granted access to that module.
- **Instant Effect:** Module toggles take effect immediately across all nodes via in-memory TTL caching.

---

## 6. Real-Time Token Revocation & Session Blacklisting

- Both tenant users and Super Admins can invalidate active sessions via `POST /api/v1/auth/logout` and `POST /api/v1/superadmin/logout`.
- Revoked tokens are fingerprinted with SHA-256 and blacklisted in a high-performance in-memory `TtlCache<String, ()>` with a 24-hour expiration window.
- The `AuthUser` extractor checks each request against the blacklist, rejecting revoked tokens with `401 Unauthorized`.

---

## 7. Role-Based Access Control (RBAC) Hierarchy & Permissions

OBAF enforces zero-trust role-based authorization across all tenant APIs:

| Role | Scope | Key Capabilities & Boundaries |
|---|---|---|
| `superadmin` | Global Platform | Cross-tenant administration, module enablement, domain SSL configuration, database backups, system settings, global maintenance, and tenant impersonation. Blocked from registering via tenant auth. |
| `admin` | Tenant Boundary | Tenant configuration, user creation, API key management, audit trail inspection, credit note generation, report exports, and currency exchange rate updates. |
| `manager` | Tenant Boundary | Operational management: cashier shift close overrides, product catalog updates, inventory adjustments, and order processing. |
| `staff` | Tenant Boundary | Front-line staff operations: order fulfillment, appointment booking, inventory receipts, and customer updates. |
| `cashier` | Tenant Boundary | Point of sale cashier operations: opening shifts, executing cash/card sales, customer lookups, and closing **own** shifts. Cross-cashier shift closures strictly blocked (BOLA defense). |
| `waiter` | Tenant Boundary | Table ordering, menu consultation, order placement, and status tracking. |
| `viewer` | Tenant Boundary | Read-only access: querying catalog, orders, and customer lists. Strictly forbidden from mutating state across orders, catalog, bookings, subscriptions, and promotions. |

---

## 8. Tenant Provisioning Security & CWE-1188 Defenses

To eliminate insecure default credentials (CWE-1188 / OWASP A07) and prevent orphaned records:
- **Mandatory Password:** All onboarding endpoints (`POST /api/v1/superadmin/organizations`, `POST /api/v1/admin/setup/preset`, `POST /api/v1/admin/tenants`) enforce an explicit `admin_password` $\ge 8$ characters. Static default passwords (`admin123`, `AdminSecure123!`) are eliminated.
- **Pre-Creation Verification:** The system verifies `admin_email` format and uniqueness against existing users before inserting an organization, preventing orphaned records.
- **Atomic Rollback:** If tenant administrator record creation fails, the newly created organization record is automatically rolled back and deleted.
- **Anonymous Registration Lock:** Anonymous user registration (`POST /api/v1/auth/register`) is blocked once users exist for that organization, requiring tenant admin authentication.

