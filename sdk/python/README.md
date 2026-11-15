# OBAF Python SDK
*Official Python client library for the Open Business Application Framework (OBAF)*

---

## Installation

```bash
pip install obaf-sdk
# or install locally from source:
pip install -e sdk/python
```

The SDK is built with **zero external dependencies** using Python's standard library `urllib`.

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
