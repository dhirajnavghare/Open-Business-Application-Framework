# OBAF API Design Standards & Conventions

## 1. URL Path Structure

All public REST endpoints follow standard versioning:
`/api/v1/<resource>`

### Core Resource Namespaces:
- **Authentication & Identity**: `/api/v1/auth/login`, `/api/v1/auth/register`, `/api/v1/auth/logout`, `/api/v1/auth/me`, `/api/v1/auth/keys`
- **Platform Super Administration**: `/api/v1/superadmin/*` (Login, Google SSO, Logout, TOTP MFA, Organizations, Audit Logs, Settings, SSL, Miracle Tools)
- **Customers & Parties**: `/api/v1/customers`
- **Suppliers & Procurement**: `/api/v1/suppliers`, `/api/v1/procurement/purchase-orders`
- **Catalog & Products**: `/api/v1/catalog/products`
- **Inventory & Stock Ledger**: `/api/v1/inventory/movements`, `/api/v1/inventory/balance/:product_id`, `/api/v1/inventory/locations`
- **Orders & State Machine**: `/api/v1/orders`
- **POS Cash Drawer & Shifts**: `/api/v1/pos/shifts`
- **Payments & Settlements**: `/api/v1/payments`, `/api/v1/payments/:id/refund`
- **Billing, Tax & Credit Notes**: `/api/v1/billing/calculate-tax`, `/api/v1/billing/apply-rules`, `/api/v1/billing/split`, `/api/v1/billing/receipt`, `/api/v1/billing/invoice-html`, `/api/v1/billing/credit-notes`, `/api/v1/billing/exchange-rates`, `/api/v1/billing/convert-currency`
- **Promotions & Coupons**: `/api/v1/promotions/coupons`
- **Bookings & Reservations**: `/api/v1/bookings`
- **Memberships & Subscriptions**: `/api/v1/subscriptions`
- **Webhooks Dispatcher**: `/api/v1/webhooks`
- **Real-Time WebSockets**: `/api/v1/ws/events`
- **Omni-Channel Global Search**: `/api/v1/search`
- **Offline Edge Synchronization**: `/api/v1/sync/push`, `/api/v1/sync/pull`
- **Introspection & Admin**: `/api/v1/meta/schema`, `/api/v1/meta/info`, `/api/v1/admin/setup/preset`, `/api/v1/admin/tenants`
- **Health & Observability**: `/health`, `/live`, `/ready`, `/metrics`
- **AI Agent Context & Documentation**: `/llms.txt`, `/swagger-ui`, `/api/v1/openapi.json`
- **Super Admin Single-Page Application**: `/admin`, `/superadmin`
- **Automated SSL ACME Responder**: `/.well-known/acme-challenge/:token`

---

## 2. HTTP Headers & Reliability Conventions

| Header | Direction | Description |
|---|---|---|
| `Authorization` | Request | `Bearer <JWT_TOKEN>` for authenticated tenant or superadmin operations |
| `X-API-Key` | Request | Machine-to-machine SHA-256 hashed API token |
| `x-request-id` | Both | Distributed tracing correlation ID (echoed back in response headers) |
| `Idempotency-Key` | Request | Unique client-generated UUID for deduplicating mutations (`/orders`, `/payments`) |
| `X-Idempotent-Replay` | Response | Set to `true` when a request was fulfilled from cached idempotent replay |
| `X-RateLimit-Remaining`| Response | Remaining token bucket quota for the calling client or tenant |
| `Retry-After` | Response | Recommended cooling duration in seconds when receiving HTTP `429 Too Many Requests` |
| `X-Content-Type-Options`| Response | Set to `nosniff` for MIME-sniffing prevention |
| `X-Frame-Options` | Response | Set to `DENY` for clickjacking mitigation |
| `X-XSS-Protection` | Response | Set to `1; mode=block` |
| `Strict-Transport-Security` | Response | `max-age=31536000; includeSubDomains` (HSTS) |
| `Content-Security-Policy` | Response | Hardened defense-in-depth CSP policy |
| `Permissions-Policy` | Response | Restricts camera, microphone, geolocation, and payment browser APIs |
| `Cross-Origin-Resource-Policy` | Response | Set to `same-origin` |
| `Cross-Origin-Opener-Policy` | Response | Set to `same-origin-allow-popups` |

> **Sensitive Headers Redaction (OWASP API3 / CWE-312):** Incoming `Authorization`, `Cookie`, `X-API-Key`, and `X-OBAF-Signature` headers are automatically classified as sensitive via `SetSensitiveRequestHeadersLayer` and masked with `[REDACTED]` in tracing logs and error output.
>
> **WebSocket Heartbeats (RFC 6455):** Connections to `/api/v1/ws/events` emit periodic 30-second `Message::Ping` frames and reply with `Message::Pong`, preventing reverse proxy idle drops and detecting stale client connections.

---

## 3. HTTP Status Code Conventions

| Status Code | Meaning | Usage |
|---|---|---|
| `200 OK` | Request succeeded | Standard response for GET, PUT, and action RPCs |
| `201 Created` | Resource created | Returned on successful POST creation |
| `400 Bad Request` | Malformed input | Invalid parameters, missing required fields (e.g. missing `admin_password`), negative quantities, or invalid domain names |
| `401 Unauthorized` | Missing / invalid token | Expired, revoked, or invalid JWT token |
| `403 Forbidden` | Access denied | User lacks required role (e.g. viewer attempting mutation), cross-cashier shift closure, unconfigured Google SSO, or tenant API module disabled (`MODULE_DISABLED`) |
| `404 Not Found` | Resource not found | Resource does not exist or belongs to another tenant |
| `409 Conflict` | Unique constraint conflict | Duplicate organization slug, admin user email, order number, or coupon code |
| `422 Unprocessable` | Schema validation error | Custom metadata failed preset JSON schema |
| `423 Locked` | Mutation frozen | Returned on POST/PUT/DELETE when Super Admin emergency lockdown is active |
| `429 Too Many Requests`| Rate limit / Lockout | Token bucket quota exhausted or anti-brute force account lockout triggered |
| `503 Service Unavailable`| Maintenance mode | Platform is in scheduled maintenance mode configured by Super Admin |

---

## 4. Query Filtering & Pagination

List endpoints support:
- `page`: 1-based page index (default: `1`)
- `page_size` / `limit`: items per page (default: `20` or `50`, hard ceiling clamped to `200` to prevent memory exhaustion DoS)
- `search`: case-insensitive substring matching on name, SKU, or email

---

## 5. OpenAPI Specification

OpenAPI 3.1 specification is auto-generated by **Utoipa** and served at:
- Swagger UI: `http://127.0.0.1:8080/swagger-ui`
- Raw JSON: `http://127.0.0.1:8080/api/v1/openapi.json`
- Static Export: `docs/openapi.json` (regenerated via `make openapi`)
