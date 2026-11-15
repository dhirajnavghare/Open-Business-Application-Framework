# ADR 0003: In-Memory Event Streaming and Webhooks with Jitter Backoff

## Status
Accepted

## Context
Real-time screens (Kitchen Display Systems, cashier terminals) require instant notifications, while external integrations (Zapier, WhatsApp gateways) require reliable HTTP webhook delivery.

## Decision
1. **Tokio Broadcast Channels**:
   - Internal state transitions (`order.created`, `order.status_updated`, `payment.recorded`) publish typed JSON events to an in-memory broadcast channel (`tokio::sync::broadcast`).
   - WebSockets at `/api/v1/ws/events` stream filtered events to authenticated clients matching their `tenant_id`.
2. **Background Webhook Dispatcher**:
   - Asynchronously dispatches events to registered tenant HTTP webhooks without blocking main HTTP request threads.
   - Computes HMAC-SHA256 signatures (`X-Obaf-Signature-256`) using tenant webhook secrets.
3. **Full-Jitter Exponential Backoff & Dead-Letter Queue (DLQ)**:
   - Webhook retries follow full-jitter exponential backoff:
     $$\text{delay} = \min(\text{max\_backoff}, \text{base} \times 2^{\text{attempt}}) \pm \text{jitter}$$
   - Exhausted retries transition to `DEAD_LETTER` status for manual inspection and replay.

## Consequences
- Zero external broker dependency (RabbitMQ/Kafka) required for baseline operations.
- Ultra-low latency (<1ms) in-process notification dispatch.
