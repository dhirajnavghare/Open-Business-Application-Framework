# OBAF Python SDK
*Official Python client library for the Open Business Application Framework (OBAF)*

---

## Installation

```bash
pip install obaf-sdk
# or install locally from source:
pip install -e sdk/python
```

The SDK is built with **zero external dependencies** using Python's standard library (`urllib`).

---

## Quickstart

```python
from obaf import ObafClient

# 1. Connect to OBAF API
client = ObafClient(base_url="http://127.0.0.1:8080")

# 2. Check health
health = client.health()
print(f"Server health: {health['status']}, database: {health['database']}")

# 3. Authenticate
client.login(email="owner@royalspice.com", password="SecurePassword123!")

# 4. Create an item
item = client.create_product(
    sku="PIZZA-MARG-01",
    name="Margherita Pizza",
    price="14.50",
    tax_rate="0.08"
)

# 5. Create order with Idempotency Key
order = client.create_order(
    customer_name="John Doe",
    table_or_room="Table 5",
    items=[{"product_id": item["id"], "quantity": 2}],
    idempotency_key="987e6543-e21b-12d3-a456-426614174000"
)
print(f"Order created: {order['order_number']}, Total: ${order['grand_total']}")
```

---

## Enterprise Workflows

### 📦 Procurement & Purchase Orders

```python
# 1. Create a supplier
supplier = client.create_supplier(
    name="Artisan Flour Mills",
    contact_person="Marco Rossi",
    email="marco@artisanflour.com"
)

# 2. Issue a Purchase Order
po = client.create_purchase_order(
    supplier_id=supplier["id"],
    items=[{
        "product_id": item["id"],
        "item_name": "Organic Type 00 Flour 25kg",
        "quantity_ordered": "100.00",
        "unit_cost": "18.50"
    }],
    notes="Pre-weekend replenishment"
)

# 3. Receive goods into inventory
received = client.receive_purchase_order(
    po_id=po["id"],
    items=[{
        "item_id": po["items"][0]["id"],
        "quantity_received": "100.00",
        "batch_number": "FLOUR-2026-B1"
    }]
)
print(f"PO Status: {received['status']}")  # RECEIVED
```

### 🎟️ Promotions & Coupon Redemption

```python
# Create a 20% off coupon code
client.create_coupon(
    code="SUMMER20",
    discount_type="PERCENTAGE",
    discount_value="0.20",
    min_order_amount="50.00",
    max_discount_amount="30.00",
    usage_limit=100
)

# Validate coupon against subtotal
validation = client.validate_coupon(code="SUMMER20", subtotal="100.00")
print(f"Valid: {validation['is_valid']}, Discount: ${validation['discount_amount']}")

# Redeem coupon atomically
redemption = client.redeem_coupon(code="SUMMER20", subtotal="100.00")
print(f"Final Subtotal: ${redemption['final_subtotal']}, Remaining Allowance: {redemption['remaining_usage']}")
```

### 🏧 POS Cash Drawer & Shift Management

```python
# Open shift with cash float
shift = client.open_shift(register_name="Front Counter Lane 1", opening_float="200.00")

# Record cash transactions during shift...

# Close shift and reconcile cash variance
closed = client.close_shift(
    shift_id=shift["id"],
    counted_cash="845.00",
    notes="Evening shift change"
)
print(f"Expected Cash: ${closed['expected_cash']}, Variance: ${closed['variance']}")
```

### 💱 Multi-Currency Conversion

```python
# Set exchange rate USD -> EUR = 0.92
client.set_exchange_rate(from_currency="USD", to_currency="EUR", rate="0.92")

# Convert currency with 128-bit decimal precision
result = client.convert_currency(amount="150.00", from_currency="USD", to_currency="EUR")
print(f"{result['original_amount']} USD = {result['converted_amount']} EUR")
```
