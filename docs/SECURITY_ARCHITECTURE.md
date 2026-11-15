# OBAF Security Architecture & Threat Model

## 1. Threat Mitigation Overview

| Threat | Mitigation Strategy |
|---|---|
| **SQL Injection** | SeaORM strongly-typed query AST with parameterized SQL statements. |
| **Cross-Tenant Data Leakage** | Context-driven tenant query filtering (`auth.tenant_id`) enforced in backend. |
| **Credential Compromise** | Argon2id password hashing with random salt and minimum cost parameters. |
| **Token Tampering** | Signed HS256 JWT tokens with 24-hour expiry and cryptographic secrets. |
| **Clickjacking / MIME Sniffing** | Security headers middleware (`nosniff`, `DENY`, `HSTS`, `XSS protection`). |
| **Replay Attacks (Sync)** | Idempotent transaction keys and client action ID tracking. |
| **API Flooding / DoS / Rogue Tenants** | Token Bucket rate limiting middleware with `X-RateLimit-Remaining` and `Retry-After` (HTTP 429). |
| **Machine-to-Machine Spoofing** | SHA-256 hashed API key validation (`X-API-Key`). |
| **Tampered Webhook Payloads** | Cryptographic HMAC-SHA256 signature verification headers. |

---

## 2. Cryptographic Standards

- **Password Hashing**: `Argon2id` (v19) via `argon2` Rust crate with memory cost: 19456 KiB, time cost: 2 iterations, parallelism: 1 lane.
- **Tokens**: `jsonwebtoken` with HS256 HMAC-SHA256 signatures. Secrets must exceed 32 characters in production environments.
- **Financial Precision**: 128-bit decimal arithmetic (`rust_decimal`) prevents floating-point rounding attacks and financial truncation vulnerabilities.
