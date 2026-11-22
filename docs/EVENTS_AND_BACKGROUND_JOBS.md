# OBAF Events & Background Automation Design

## 1. Domain Event Architecture

All critical state transitions publish a versioned `DomainEvent` envelope:

```rust
pub struct DomainEvent<T> {
    pub id: Uuid,
    pub event_type: String,
    pub schema_version: u32,
    pub tenant_id: TenantId,
    pub timestamp: DateTime<Utc>,
    pub payload: T,
}
```

Standard domain event types:
- `order.created`, `order.status_updated`, `order.completed`, `order.cancelled`
- `payment.recorded`, `payment.refunded`
- `stock.low`, `stock.movement_recorded`
- `purchase_order.created`, `purchase_order.received`
- `pos.shift_opened`, `pos.shift_closed`
- `coupon.redeemed`
- `booking.created`, `booking.checked_in`, `booking.completed`
- `subscription.renewed`, `subscription.checked_in`

---

## 2. In-Memory Event Bus & WebSocket Streaming

1. Handlers publish events into a Tokio `broadcast::channel`.
2. Clients subscribe to real-time events via WebSockets at `/api/v1/ws/events?token=<JWT>`.
3. Events are filtered so each client receives only events matching its `tenant_id` (Super Admin receives global events).
4. **RFC 6455 Heartbeat Keep-Alive:** The server emits periodic 30-second `Message::Ping` frames and responds to client `Message::Ping` frames with `Message::Pong`, preventing reverse proxy idle drops and detecting stale clients.
5. **Frame & Message Limits:** Maximum frame size 16KB, maximum message size 64KB for denial-of-service resilience.

---

## 3. Webhook Delivery & HMAC-SHA256 Signatures

Background workers dispatch HTTP webhooks asynchronously to external automation endpoints (WhatsApp, Zapier, accounting software):
- Each request carries headers:
  - `X-OBAF-Signature`: `sha256=<hex_hmac>` calculated over the JSON payload using the webhook secret.
  - `X-OBAF-Event`: The event type string.
  - `X-OBAF-Timestamp`: Epoch timestamp of dispatch.
- **SSRF Network Boundary:** Destination URLs are resolved and verified; requests to private RFC 1918, link-local, and loopback IPs are blocked.
- Automatic retries with exponential backoff on HTTP 5xx or connection timeouts.

---

## 4. Standalone Worker Daemon (`obaf-worker`)

The standalone background daemon (`binaries/obaf-worker`) runs independently of API servers:
- Executes periodic tenant maintenance sweeps and scheduled tasks without blocking user-facing HTTP request threads.
- Supports `--interval-secs` and `--once` execution modes with graceful `SIGINT`/`SIGTERM` signal draining.
