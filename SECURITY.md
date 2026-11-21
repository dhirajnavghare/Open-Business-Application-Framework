# Security Policy

The Open Business Application Framework (OBAF) team takes software security and data privacy seriously. As a foundational multi-tenant business operations and billing engine, maintaining uncompromising security standards is central to our mission.

---

## 🛡️ Supported Versions

Only the latest active minor release is officially supported with security patches:

| Version | Supported          |
| ------- | ------------------ |
| 0.2.x   | :white_check_mark: |
| 0.1.x   | :white_check_mark: |
| < 0.1   | :x:                |

---

## 🔒 Security Architecture & Boundaries

OBAF enforces defense-in-depth across the following boundaries:

### 1. Multi-Tenant Isolation
- All business records (`products`, `orders`, `customers`, `payments`, `bookings`, `subscriptions`, `webhooks`) are scoped by `tenant_id`.
- The `AuthUser` extractor enforces tenant scoping at the HTTP request layer.
- Tenant context is verified on all database queries and mutations. Requests attempting to access or mutate resources outside the authenticated tenant boundary are rejected with `401 Unauthorized` or `404 Not Found`.

### 2. Password Hashing & Key Derivation
- Passwords are never stored in plaintext.
- We use **Argon2id** (`argon2 = 0.5`) with per-user cryptographic salts (`OsRng`) and memory-hard parameter limits to safeguard against GPU/ASIC cracking.

### 3. API Authentication & Token Lifecycle
- Stateless authentication uses JSON Web Tokens (JWT) signed with HMAC-SHA256 locked to `Algorithm::HS256`.
- Token expiration (`exp`) is enforced.
- **Real-Time Token Revocation:** Immediate session invalidation via `POST /api/v1/auth/logout` and `POST /api/v1/superadmin/logout`, adding token SHA-256 fingerprints to an in-memory `TtlCache` blacklist.
- **MFA Privilege Isolation:** Multi-factor authentication requires intermediate `superadmin_mfa_pending` token verification before full platform privileges are granted.

### 4. Webhook Delivery Authentication
- All outgoing webhooks dispatched to external subscribers include cryptographic signatures:
  - Header: `X-OBAF-Signature: sha256=<hmac_sha256_hex_digest>`
  - Header: `X-OBAF-Timestamp: <unix_epoch_seconds>`
  - Header: `X-OBAF-Event: <event_type>`
- Subscribers should verify the signature against their registered webhook secret before processing incoming payloads.

### 5. Financial Arithmetic Security
- OBAF strictly avoids IEEE 754 floating-point numbers (`f32`/`f64`) in all billing, tax, and ledger logic.
- We exclusively use `rust_decimal::Decimal` (128-bit fixed-point arithmetic) to prevent rounding exploits and fractional drift attacks.

### 6. Zero-Trust RBAC & Default Credential Defenses (CWE-1188)
- **Mandatory Administrator Passwords:** All tenant creation routes require an explicit password of $\ge 8$ characters. Static default credentials (`admin123`, `AdminSecure123!`) are eliminated.
- **Zero-Trust Role Gating:** Read-only `viewer` accounts are blocked from mutating state across orders, catalog, bookings, subscriptions, and promotions.
- **BOLA Shift Isolation:** Cashier shift closures are strictly locked to the shift owner or authorized administrators.

### 7. Sensitive Headers Redaction & Backup Sanitization (OWASP API3 / CWE-312)
- **Trace Redaction:** `SetSensitiveRequestHeadersLayer` automatically redacts `Authorization`, `Cookie`, `X-API-Key`, and `X-OBAF-Signature` headers with `[REDACTED]` in tracing logs and error output.
- **Backup Sanitization:** Database backups downloaded from `/api/v1/superadmin/database/backup` automatically sanitize user `password_hash` to `[REDACTED]` and filter out MFA secrets (`superadmin_totp_secret`, recovery codes).

### 8. RFC 6455 WebSocket Heartbeats & DoS Defense
- **Heartbeat Pings:** Real-time event streams at `/api/v1/ws` emit periodic 30-second heartbeats (`Message::Ping`) and handle `Message::Pong`, preventing proxy timeouts, detecting dead clients, and mitigating ghost connection leaks.

---

## 🚨 Reporting a Vulnerability

If you discover a security vulnerability in OBAF, please **do NOT report it in public GitHub issues**.

Instead, please report it via one of the following channels:

1. **GitHub Private Vulnerability Reporting:**  
   Navigate to the repository's **Security** tab and click **"Report a vulnerability"**.
2. **Direct Security Contact:**  
   Send an email to `security@obaf.org` (or contact repository maintainers directly).

### What to Include:
- A detailed description of the vulnerability.
- Steps to reproduce or proof-of-concept (PoC) code/script.
- Affected components (e.g. `obaf-core`, `obaf-api`, specific endpoints).
- Potential impact and threat model.

### Disclosure Timeline:
- **Initial Response:** Within 48 hours of receipt.
- **Triage & Assessment:** Within 5 business days.
- **Fix & Advisory Release:** Coordinated disclosure after patch release and verification.
