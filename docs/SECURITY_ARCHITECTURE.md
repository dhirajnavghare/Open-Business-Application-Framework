# OBAF Security Architecture & Threat Model

## 1. Threat Mitigation Overview

| Threat | Mitigation Strategy |
|---|---|
| **SQL Injection** | SeaORM strongly-typed query AST with parameterized SQL statements. Zero raw string interpolation. |
| **Cross-Tenant Data Leakage** | Context-driven tenant query filtering (`auth.tenant_id`) enforced across all database queries. |
| **Credential Compromise** | Argon2id password hashing with random salt and OWASP-compliant memory/time cost parameters. |
| **Token Tampering** | Cryptographically signed HS256 JWT tokens with 24-hour expiry and database-persisted high-entropy secrets. |
| **Session Invalidation / Stale Tokens** | Real-time SHA-256 fingerprint token revocation blacklist (`token_blacklist`) backed by memory-bounded `TtlCache`. |
| **Brute Force & Credential Stuffing** | Progressive account lockout (`LoginAttemptGuard`): 5 consecutive failures triggers 15-minute HTTP 429 lockout. |
| **User Enumeration (Timing Attacks)** | Argon2id dummy hash verification ensures constant ~50ms execution time even when the user account does not exist. |
| **Privilege Escalation** | Strict role-based access control (RBAC), dedicated `SUPERADMIN_USER_ID`, and intermediate MFA token privilege isolation. |
| **Server-Side Request Forgery (SSRF)** | Webhook dispatch engine validates destination IP addresses and rejects private (RFC 1918), link-local, and loopback ranges. |
| **Financial & Math Inversion Exploits** | Strict input boundary validation: quantities strictly greater than zero, payment amounts strictly greater than zero, and non-negative pricing. |
| **Clickjacking / MIME Sniffing** | Security headers middleware (`nosniff`, `DENY`, `HSTS`, `XSS protection`, `Permissions-Policy`, `CORP`, `COOP`, strict `CSP`). |
| **Replay Attacks (Sync & API)** | Idempotent transaction keys (`Idempotency-Key` header with `x-idempotent-replay`) and client action ID tracking. |
| **API Flooding / DoS / Rogue Tenants** | Token Bucket rate limiting middleware with `X-RateLimit-Remaining` and `Retry-After` (HTTP 429). |
| **Machine-to-Machine Spoofing** | SHA-256 hashed API key validation (`X-API-Key`). |
| **Tampered Webhook Payloads** | Cryptographic HMAC-SHA256 signature verification headers (`X-OBAF-Signature`). |
| **Production Default Password Vulnerability**| Fatal startup abort in production mode (`OBAF_ENV=production`) if default Super Admin password (`admin123`) is detected. |
| **Insecure Default Credentials (CWE-1188)**| Mandatory `admin_password` (minimum 8 characters) across all tenant onboarding endpoints (`/organizations`, `/setup/preset`, `/tenants`). All fallback defaults eliminated. |
| **Orphaned Organization Mitigation**| Pre-creation validation of administrator email format and duplicate checks before inserting organizations; automatic transactional rollback on user creation failure. |
| **Google SSO Audience Confusion**| Strict audience (`aud`) claim verification against configured Google client ID; unconfigured server returns `403 Forbidden` without external network calls; mock tokens gated strictly to debug/test builds. |
| **Broken Object Level Authorization (BOLA/IDOR)**| POS cashier shift closure restricted to the owning cashier or administrative override; cross-tenant usage metrics and billing records strictly isolated. |
| **Role Privilege Escalation (RBAC)**| Read-only `viewer` accounts blocked from mutating operations; anonymous registration locked down once users exist; superadmin role blocked from self-registration. |

---

## 2. Authentication & Identity Management

### 2.1 Dual-Engine Super Admin Authentication
- **Top-Level Root Admin:** Configured strictly via environment variables (`SUPERADMIN_EMAIL`, `SUPERADMIN_PASSWORD`).
- **Google SSO:** Direct integration with Google Identity Services (GIS). Cryptographically verifies Google ID tokens using Google's public JSON Web Key Sets (JWKS) via `jsonwebtoken` with cache-aside TTL key caching.
  - **Audience Confusion Defense:** Google SSO requires an explicitly configured `GOOGLE_CLIENT_ID`. If unconfigured, the endpoint immediately returns `403 Forbidden` without initiating external network calls.
  - **Strict Audience Matching:** The token's `aud` claim must strictly match the server's configured client ID (`401 Unauthorized` on mismatch or missing claim).
  - **Production Mock Isolation:** Development mock token bypasses (`mock_google_token:...`) are compiled exclusively within `#[cfg(any(test, debug_assertions))]` blocks, guaranteeing they are inactive in production release binaries.
- **Email & Password:** Supports raw password configuration or pre-hashed Argon2id hashes in environment variables.

### 2.2 Multi-Factor Authentication (TOTP 2FA)
- Super Admin accounts support RFC 6238 Time-based One-Time Password (TOTP) two-factor authentication.
- Cryptographic TOTP secret generated using SHA-1 / HMAC with 30-second time steps and 6-digit verification codes.
- Pre-generated single-use emergency recovery backup codes stored with Argon2 hashing.
- **Intermediate Token Isolation:** When MFA is active, initial login returns a temporary token with role `superadmin_mfa_pending`. All standard API endpoints reject tokens with this role with `401 Unauthorized`. Only `POST /api/v1/superadmin/mfa-verify` accepts this temporary token to issue the final full-privilege session token.

### 2.3 Instant Token Revocation Blacklist
JWT tokens are inherently stateless. To provide immediate session revocation upon logout:
- Endpoints: `POST /api/v1/auth/logout` (Tenant User) and `POST /api/v1/superadmin/logout` (Super Admin).
- When invoked, the caller's JWT token is extracted, converted to a SHA-256 hex fingerprint, and stored in a thread-safe `TtlCache<String, ()>` with an automatic 24-hour expiration window.
- The `AuthUser` extractor checks each incoming request against this blacklist in $O(1)$ time, immediately returning `401 Unauthorized` if the token has been revoked.

### 2.4 Anti-Brute Force, Credential Stuffing & User Enumeration
- **Progressive Lockout (`LoginAttemptGuard`):** Tracks failed attempts keyed by `login:<email>:<client_ip>`. After 5 consecutive failures, subsequent requests are rejected immediately with `429 Too Many Requests` for 15 minutes.
- **Timing Equalization:** In standard authentication handlers, non-existent users trigger an immediate failure, creating a measurable ~50ms difference compared to password hashing for existing users. OBAF executes an Argon2id verification against a precomputed dummy hash on non-existent users, equalizing server response time and eliminating user enumeration side-channels.

### 2.5 Zero-Trust Role-Based Access Control (RBAC) & BOLA / IDOR Defense
OBAF implements a hierarchical role matrix: `superadmin` > `admin` > `manager` > `staff` > `cashier` > `viewer`.
- **Read-Only Viewer Lockdown:** Accounts with role `viewer` are strictly forbidden from mutating state across orders, catalog, customers, booking check-ins/check-outs, subscription status updates, and promotions (`403 Forbidden`).
- **POS Cashier Shift Isolation (BOLA Defense):** Cashiers can only manage and close their own shifts (`pos/shifts/:id/close`). Attempting to close another cashier's shift returns `403 Forbidden`, while tenant administrators or managers possess operational override capability.
- **Tenant Self-Registration Takeover Prevention:** Anonymous user registration (`POST /api/v1/auth/register`) is permitted only during initial tenant bootstrapping. Once tenant users exist, registration requires tenant admin or superadmin authorization.
- **Superadmin Role Gating:** The `superadmin` role cannot be registered via tenant API endpoints under any circumstances (`403 Forbidden`).
- **Administrative Operation Gating:** Financial report exports, credit notes, currency exchange rates, and webhooks are strictly gated to administrative privileges.

### 2.6 Tenant Provisioning & Default Credential Defense (CWE-1188)
- **Mandatory Administrator Password:** All organization and tenant creation endpoints (`POST /api/v1/superadmin/organizations`, `POST /api/v1/admin/setup/preset`, and `POST /api/v1/admin/tenants`) enforce an explicit `admin_password` of at least 8 characters. Hardcoded fallback passwords (`"admin123"`, `"AdminSecure123!"`) are completely removed.
- **Pre-Creation Email Validation & Uniqueness:** Administrator email format and uniqueness are verified against existing users prior to organization database insertion, preventing orphaned organization records.
- **Automatic Rollback:** If user record creation fails, the newly inserted organization record is automatically rolled back and deleted.

---

## 3. Financial & Business Logic Integrity

- **128-Bit Fixed-Point Arithmetic:** Financial balances, line items, and totals use `rust_decimal::Decimal` (128-bit fixed-point), eliminating floating-point rounding errors.
- **Penny-Perfect Reconciliation:** Split bill and payment distribution algorithms strictly reconcile rounding pennies into the final participant share to guarantee exact mathematical balance.
- **Input Bounds Validation:**
  - Order line items enforce `quantity > Decimal::ZERO` and limit batch requests to 500 items, preventing inventory-reversal exploits.
  - Payment records enforce `amount > Decimal::ZERO`.
  - Product catalog records enforce non-negative values (`>= 0`) for price, cost price, and tax rate.

---

## 4. Network Boundary & SSRF Prevention

- **Webhook Destination Validation:** When registering or testing outgoing webhooks (`POST /api/v1/webhooks`), the URL destination host is resolved and verified:
  - Private IPv4 ranges (RFC 1918: `10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`) are blocked.
  - Link-local addresses (`169.254.0.0/16`, including AWS/GCP cloud metadata endpoints) are blocked.
  - Loopback addresses (`127.0.0.0/8`, `localhost`) are blocked.
- **ACME Challenge Sanitization:** `GET /.well-known/acme-challenge/:token` strictly validates that challenge tokens match `[a-zA-Z0-9_-]{8,128}`, preventing path traversal and CRLF injection.
- **FQDN Domain Validation:** Domain names configured for SSL must strictly conform to RFC 1123, rejecting shell metacharacters, null bytes, and traversal tokens.

---

## 5. Defense-in-Depth HTTP Security Headers

Every HTTP response from `obaf-server` is hardened with security headers:
- `Content-Security-Policy`: Restricts scripts, styles, fonts, objects, and connect endpoints.
- `Permissions-Policy`: Restricts unauthorized browser API access (`geolocation=(), camera=(), microphone=(), payment=(), usb=()`).
- `Cross-Origin-Resource-Policy`: `same-origin` to prevent cross-origin data extraction.
- `Cross-Origin-Opener-Policy`: `same-origin-allow-popups` to isolate browsing contexts while permitting Google SSO popups.
- `X-Content-Type-Options`: `nosniff` prevents MIME confusion attacks.
- `X-Frame-Options`: `DENY` prevents clickjacking.
- `X-XSS-Protection`: `1; mode=block` enables legacy browser XSS filters.
- `Strict-Transport-Security`: `max-age=31536000; includeSubDomains` enforces HTTPS.

---

## 6. Cryptographic Standards

- **Password Hashing**: `Argon2id` (v19) via `argon2` Rust crate with memory cost: 19,456 KiB, time cost: 2 iterations, parallelism: 1 lane.
- **JWT Signing & Algorithm Locking**: `jsonwebtoken` explicitly locked to `Algorithm::HS256` for both signing and verification. Algorithms such as `none`, `HS384`, `HS512`, or asymmetric algorithms are cryptographically rejected to prevent JWT Algorithm Confusion attacks. Cryptographic signing secrets are automatically generated and persisted in the database on first boot if not explicitly provided.
- **Constant-Time Comparison**: All sensitive comparisons (backup recovery codes, API key hashes, and webhook signatures) use constant-time byte-by-byte XOR comparison (`constant_time_compare`) to eliminate side-channel timing analysis.
- **Webhook Signatures**: Outgoing webhooks include an `X-OBAF-Signature` header computed as `HMAC-SHA256(payload, webhook_secret)`.
- **API Keys**: Stored as SHA-256 hex digests; plaintext keys are displayed to users only once upon creation.

---

## 7. Resilience, Anti-Tampering & Denial-of-Service Defense

- **Host Header Injection / Poisoning Defense**: The HTTP middleware strictly validates `Host` and `X-Forwarded-Host` headers against allowable RFC character sets (`[a-zA-Z0-9.:\-\[\]]`) with length limits ($\le 255$ chars), rejecting malformed headers, CRLF injection, and cache poisoning attempts with `400 Bad Request`.
- **Panic Resilience Shield (`CatchPanicLayer`)**: The Axum HTTP pipeline is protected by Tower's `CatchPanicLayer`, which intercepts unhandled panics, unwinds gracefully, logs the event internally, and returns a sanitized HTTP 500 status rather than crashing worker threads or aborting connections.
- **Database Error Masking & Information Disclosure Prevention**: Database driver errors and constraint violations are logged internally via `tracing::error!` and masked into generic error messages for client responses, preventing internal SQL schemas, database versions, and table structures from being leaked to attackers.
- **Bounded Resource Ceilings**:
  - `TokenBucketRateLimiter`: Max 50,000 buckets with automatic LRU and idle eviction.
  - `TtlCache`: Hard maximum capacity enforcement to eliminate unbounded memory growth.
  - Request Body Limits: 2MB hard ceiling via `DefaultBodyLimit::max(2 * 1024 * 1024)`.
  - Query Pagination Clamping: All list endpoints clamp user-requested limits to a maximum of 200 records.
  - WebSocket Lifecycles: 64KB max message size and active disconnect monitoring.
