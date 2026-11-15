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
