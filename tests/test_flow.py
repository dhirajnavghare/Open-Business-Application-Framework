import urllib.request
import urllib.error
import json
import sys
import time

BASE_URL = "http://127.0.0.1:8080"

def post(url, data, token=None):
    req = urllib.request.Request(
        url,
        data=json.dumps(data).encode("utf-8"),
        headers={
            "Content-Type": "application/json",
            **({"Authorization": f"Bearer {token}"} if token else {})
        },
        method="POST"
    )
    try:
        with urllib.request.urlopen(req) as resp:
            return json.loads(resp.read().decode("utf-8"))
    except urllib.error.HTTPError as e:
        print("HTTP ERROR:", e.code, e.read().decode("utf-8"))
        raise

def patch(url, data, token=None):
    req = urllib.request.Request(
        url,
        data=json.dumps(data).encode("utf-8"),
        headers={
            "Content-Type": "application/json",
            **({"Authorization": f"Bearer {token}"} if token else {})
        },
        method="PATCH"
    )
    with urllib.request.urlopen(req) as resp:
        return json.loads(resp.read().decode("utf-8"))

def get(url, token=None):
    req = urllib.request.Request(
        url,
        headers={"Authorization": f"Bearer {token}"} if token else {}
    )
    with urllib.request.urlopen(req) as resp:
        return json.loads(resp.read().decode("utf-8"))

def main():
    print("--- 1. Bootstrap Organization & Preset ---")
    ts = int(time.time())
    slug = f"royal-spice-{ts}"
    email = f"owner-{ts}@royalspice.com"
    bootstrap = post(f"{BASE_URL}/api/v1/admin/setup/preset", {
        "organization_name": "Royal Spice Bistro",
        "slug": slug,
        "preset": "restaurant_pos",
        "admin_email": email,
        "admin_name": "Owner Anita",
        "admin_password": "secretpassword123"
    })
    print("Bootstrap response:", json.dumps(bootstrap, indent=2))

    print("\n--- 2. Login via Argon2id & JWT ---")
    login = post(f"{BASE_URL}/api/v1/auth/login", {
        "email": email,
        "password": "secretpassword123"
    })
    token = login["token"]
    print(f"Logged in as {login['full_name']} ({login['role']}). Token: {token[:20]}...")

    print("\n--- 3. Verify Authenticated Profile (/api/v1/auth/me) ---")
    me = get(f"{BASE_URL}/api/v1/auth/me", token=token)
    print("Profile:", json.dumps(me, indent=2))

    print("\n--- 4. Catalog Engine: Create Menu Items ---")
    p1 = post(f"{BASE_URL}/api/v1/catalog/products", {
        "sku": "FOOD-001",
        "name": "Paneer Tikka Masala",
        "price": 280.00,
        "cost_price": 100.00,
        "tax_rate": 0.05,
        "metadata": {"spice": "medium", "course": "main"}
    }, token=token)
    print("Product 1:", p1["name"], "| Price:", p1["price"])

    p2 = post(f"{BASE_URL}/api/v1/catalog/products", {
        "sku": "FOOD-002",
        "name": "Garlic Butter Naan",
        "price": 60.00,
        "cost_price": 15.00,
        "tax_rate": 0.05,
        "metadata": {"course": "bread"}
    }, token=token)
    print("Product 2:", p2["name"], "| Price:", p2["price"])

    print("\n--- 5. Inventory Engine: Record Batch Stock ---")
    mov = post(f"{BASE_URL}/api/v1/inventory/movements", {
        "product_id": p1["id"],
        "movement_type": "RECEIPT",
        "quantity": 40.0,
        "batch_number": "BATCH-202611A",
        "notes": "Fresh procurement batch"
    }, token=token)
    print("Stock Movement recorded:", mov["movement_type"], "Qty:", mov["quantity"], "Batch:", mov["batch_number"])

    print("\n--- 6. Orders Engine: Create Table Order ---")
    order = post(f"{BASE_URL}/api/v1/orders", {
        "customer_name": "Rohan & Family",
        "table_or_room": "Table #04",
        "items": [
            {"product_id": p1["id"], "quantity": 2.0, "notes": "Extra crispy"},
            {"product_id": p2["id"], "quantity": 4.0, "notes": "Hot & fresh"}
        ]
    }, token=token)
    print("Order Created:", order["order_number"], "Status:", order["status"])
    print(f"Subtotal: {order['subtotal']} | Tax: {order['tax_total']} | Grand Total: {order['grand_total']}")

    print("\n--- 7. Workflow State Transition: Send to Kitchen ---")
    status_update = patch(f"{BASE_URL}/api/v1/orders/{order['id']}/status", {
        "status": "COOKING"
    }, token=token)
    print("Order Status Updated:", status_update)

    print("\n--- 8. Billing Engine: Split Bill across 3 Diners ---")
    split = post(f"{BASE_URL}/api/v1/billing/split-bill", {
        "total_amount": float(order["grand_total"]),
        "num_parties": 3
    })
    print("Split Bill Breakdown:")
    for share in split["shares"]:
        print(f"  Party #{share['party_index']}: ${share['share_amount']}")

    print("\n--- 9. Billing Engine: Generate Thermal Printable Receipt ---")
    receipt = get(f"{BASE_URL}/api/v1/billing/orders/{order['id']}/receipt", token=token)
    print(receipt["receipt_text"])

    print("ALL TESTS PASSED SUCCESSFULLY! 🚀")

if __name__ == "__main__":
    main()
