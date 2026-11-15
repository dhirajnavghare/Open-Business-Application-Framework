# ADR 0001: Multi-Crate Modular Workspace and Pluggable Storage Architecture

## Status
Accepted

## Context
The Open Business Application Framework (OBAF) is designed as a reusable foundation for running diverse business operations (POS, clinics, hotels, subscriptions, salons) without monolithic entanglement or premature microservice complexity.

## Decision
1. **Cargo Virtual Workspace**: Segment the codebase into strictly bounded crates:
   - `crates/obaf-core`: Pure domain contracts, Money arithmetic, 128-bit decimal precision, JWT/Argon2id security, rules evaluation, extension traits.
   - `crates/obaf-database`: Unified connection pooling (`TenantPoolRouter`), SeaORM entities, migration execution.
   - `crates/obaf-api`: Axum 0.7 REST API, OpenAPI docs (Utoipa), WebSocket event bus, background HMAC webhook workers.
   - `binaries/obaf-server`: Standalone daemon with hierarchical configuration.
   - `binaries/obaf-cli`: Operational tooling (`doctor`, `presets`, `token`, `backup`, `restore`, `new-preset`).
2. **Database Portability**:
   - Zero-configuration local SQLite for edge and development deployments.
   - Clustered PostgreSQL 16+ for enterprise SaaS multi-tenant deployments.
3. **Strict Financial Precision**:
   - Forbid IEEE-754 floating-point numbers in financial arithmetic (`rust_decimal::Decimal`).

## Consequences
- Clean compile-time boundary enforcement with inward-pointing dependencies.
- Extensible via JSON presets and Rust community extensions (`ObafExtension`).
