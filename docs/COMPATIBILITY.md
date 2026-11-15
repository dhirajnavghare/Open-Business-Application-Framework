# Open Business Application Framework (OBAF) Compatibility & Stability Policy

This document defines the stability guarantees, versioning rules, and backward-compatibility guidelines for OBAF platform components, HTTP APIs, and community extensions.

---

## 1. Semantic Versioning Specification

OBAF strictly adheres to [Semantic Versioning 2.0.0](https://semver.org/) across its public contracts:

- **Major (`X.0.0`)**: Incompatible API contract changes, breaking database migrations without automated transitions, or removal of deprecated module hooks.
- **Minor (`0.Y.0` / `X.Y.0`)**: Backward-compatible new features, new REST/WebSocket endpoints, new domain presets, and additive schema fields.
- **Patch (`0.0.Z` / `X.Y.Z`)**: Backward-compatible bug fixes, security patches, performance optimizations, and documentation updates.

---

## 2. Public API Stability (`/api/v1/*`)

All endpoints under the `/api/v1/` route prefix are governed by strict backward-compatibility rules:

### 2.1 Backward-Compatible Changes (Permitted in Minor/Patch Releases)
- Adding new optional request fields.
- Adding new fields to response JSON payloads.
- Adding new HTTP routes and WebSocket event channels.
- Adding new query parameters with sensible defaults.
- Adding new domain presets under `config/presets/`.

### 2.2 Breaking Changes (Forbidden within a Major Version)
- Removing or renaming existing endpoints.
- Renaming existing JSON request or response fields.
- Changing field data types (e.g. converting a string to an integer, or float to decimal).
- Making previously optional request fields mandatory.
- Changing HTTP status codes for existing success/error paths.

### 2.3 Deprecation Policy
When an API capability is superseded:
1. It is marked as deprecated in OpenAPI specifications and release notes.
2. The response will include standard HTTP `Deprecation` and `Sunset` headers.
3. The deprecated behavior is retained for at least one minor release cycle before scheduled removal in the next major version.

---

## 3. Database Schema & Migration Stability

To support zero-downtime rolling upgrades and independent deployments:
1. **Additive Schema Modifications:** New tables, optional columns (`nullable`), and indexes are permitted in minor migrations.
2. **Never Drop in a Single Step:** Column or table deletions require a two-phase deprecation:
   - Phase 1: Mark column as deprecated and stop reading from it.
   - Phase 2: Drop column in the subsequent major version after data migration.
3. **Database Portability:** All foundational schema definitions in `crates/obaf-database/src/migration.rs` must execute cleanly across both SQLite (for zero-config local operations) and PostgreSQL (for production clustering).

---

## 4. Extension & Module Contract Stability (`obaf-core::extension`)

Extensions compiled against `obaf-core` rely on stable Rust traits:
- The `ObafExtension` trait methods (`manifest`, `on_init`, `on_event`) are stable within the `0.1.x` release series.
- New lifecycle hooks added to `ObafExtension` provide default no-op implementations to prevent breaking existing third-party modules.
- Domain event schemas (e.g. `order.created`, `payment.refunded`) carry an explicit `schema_version: u32` in their envelope.
