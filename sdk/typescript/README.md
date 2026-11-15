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
  name: "Classic Cheeseburger",
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

---

## Enterprise Workflows

### 📦 Procurement & Purchase Orders

```typescript
// 1. Create a Purchase Order
const po = await client.createPurchaseOrder({
  supplier_id: "c185ec43-85b6-4b2a-89f5-19e917d23f39",
  currency: "USD",
  items: [{
    product_id: product.id,
    item_name: "Classic Cheeseburger Buns 100pk",
    quantity_ordered: "50.00",
    unit_cost: "12.00"
  }],
  notes: "Urgent inventory replenishment"
});

// 2. Receive goods into stock
const received = await client.receivePurchaseOrder(po.id, [
  {
    item_id: po.items[0].id,
    quantity_received: "50.00",
    batch_number: "BATCH-2026-B1"
  }
]);
console.log(`PO Status: ${received.status}`); // RECEIVED
```

### 🎟️ Promotions & Coupon Redemption

```typescript
// Create a percentage discount coupon
await client.createCoupon({
  code: "WELCOME20",
  discount_type: "PERCENTAGE",
  discount_value: "0.20",
  min_order_amount: "30.00",
  usage_limit: 50
});

// Validate coupon applicability
const validation = await client.validateCoupon("WELCOME20", "60.00");
console.log(`Valid: ${validation.is_valid}, Discount: $${validation.discount_amount}`);

// Atomically redeem coupon
const redemption = await client.redeemCoupon("WELCOME20", "60.00");
console.log(`Final Subtotal: $${redemption.final_subtotal}, Remaining: ${redemption.remaining_usage}`);
```

### 🏧 POS Cash Drawer Shifts

```typescript
// Open register shift
const shift = await client.openShift({
  register_name: "Front Terminal 1",
  opening_float: "150.00"
});

// Reconcile and close shift
const closed = await client.closeShift(shift.id, {
  counted_cash: "720.00",
  notes: "End of morning shift"
});
console.log(`Expected Cash: $${closed.expected_cash}, Variance: $${closed.variance}`);
```

### 💱 Multi-Currency Conversion

```typescript
// Set exchange rate USD -> INR = 84.50
await client.setExchangeRate({
  from_currency: "USD",
  to_currency: "INR",
  rate: "84.50"
});

// Convert amount
const conversion = await client.convertCurrency({
  amount: "100.00",
  from_currency: "USD",
  to_currency: "INR"
});
console.log(`${conversion.original_amount} USD = ${conversion.converted_amount} INR`);
```
