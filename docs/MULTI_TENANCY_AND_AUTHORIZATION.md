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

