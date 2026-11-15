# ADR 0002: Shared-Database Multi-Tenant Isolation and Hybrid Routing

## Status
Accepted

## Context
OBAF must support hundreds of organizations (tenants) operating on either shared infrastructure or dedicated databases for enterprise accounts with strict compliance requirements.

## Decision
1. **Strongly-Typed Identifiers**:
   - Define `TenantId(pub Uuid)` and `BranchId(pub Uuid)` in `obaf-core`.
   - Never trust client-supplied tenant identifiers in request payloads; extract tenant context strictly from verified JWT claims or cryptographic API keys (`X-API-Key`).
2. **Column-Level Isolation**:
   - Every business table (`customer`, `product`, `order`, `payment`, `booking`, `subscription`) enforces a mandatory `tenant_id` foreign key.
   - All queries filter by `tenant_id = auth.tenant_id`.
3. **Dedicated Enterprise Database Routing (`TenantPoolRouter`)**:
   - Maintain a shared default connection pool for standard tenants.
   - Support dynamic routing of enterprise tenants to isolated PostgreSQL instances via an in-memory pool router.

## Consequences
- Impossible for cross-tenant data leaks to occur under standard operations.
- Smooth upgrade path for enterprise customers requiring dedicated databases without code changes.
