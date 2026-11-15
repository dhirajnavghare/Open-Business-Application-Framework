# @obaf/sdk - Official TypeScript SDK

Zero-dependency TypeScript/JavaScript client library for the Open Business Application Framework (OBAF). Built using modern standard `fetch` API for Node.js (18+), Bun, Deno, Next.js, and modern browsers.

---

## Installation

```bash
npm install @obaf/sdk
# or with pnpm:
pnpm add @obaf/sdk
```

---

## Quickstart

```typescript
import { ObafClient } from "@obaf/sdk";

// 1. Initialize client
const client = new ObafClient({ baseUrl: "http://127.0.0.1:8080" });

// 2. Health check
const health = await client.health();
console.log(`OBAF Status: ${health.status}, Database: ${health.database}`);

// 3. Login
await client.login("admin@example.com", "SecurePassword123!");

// 4. Create an item
const product = await client.createProduct({
  sku: "BURGER-01",
  name="Classic Cheeseburger",
  price: "9.99",
  tax_rate: "0.08"
});

// 5. Place order with idempotency protection
const order = await client.createOrder(
  {
    customer_name: "Sarah Connor",
    table_or_room: "Table 12",
    items: [{ product_id: product.id, quantity: 2 }]
  },
  "123e4567-e89b-12d3-a456-426614174000" // Idempotency Key
);

console.log(`Order placed: ${order.order_number}, Grand Total: $${order.grand_total}`);
```
