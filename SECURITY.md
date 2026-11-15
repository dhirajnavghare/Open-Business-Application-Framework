# Security Policy

The Open Business Application Framework (OBAF) team takes software security and data privacy seriously. As a foundational multi-tenant business operations and billing engine, maintaining uncompromising security standards is central to our mission.

---

## 🛡️ Supported Versions

Only the latest active minor release is officially supported with security patches:

| Version | Supported          |
| ------- | ------------------ |
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
- Stateless authentication uses JSON Web Tokens (JWT) signed with HMAC-SHA256.
- Token expiration (`exp`) is enforced.
- Revocation strategies can be configured via short TTLs or organization-level secret invalidation.

### 4. Webhook Delivery Authentication
- All outgoing webhooks dispatched to external subscribers include cryptographic signatures:
  - Header: `X-OBAF-Signature: sha256=<hmac_sha256_hex_digest>`
  - Header: `X-OBAF-Timestamp: <unix_epoch_seconds>`
  - Header: `X-OBAF-Event: <event_type>`
- Subscribers should verify the signature against their registered webhook secret before processing incoming payloads.

### 5. Financial Arithmetic Security
- OBAF strictly avoids IEEE 754 floating-point numbers (`f32`/`f64`) in all billing, tax, and ledger logic.
- We exclusively use `rust_decimal::Decimal` (128-bit fixed-point arithmetic) to prevent rounding exploits and fractional drift attacks.

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
