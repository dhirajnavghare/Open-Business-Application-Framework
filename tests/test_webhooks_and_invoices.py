import urllib.request
import json
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
    with urllib.request.urlopen(req) as resp:
        return json.loads(resp.read().decode("utf-8"))

def get(url, token=None):
    req = urllib.request.Request(
        url,
        headers={"Authorization": f"Bearer {token}"} if token else {}
    )
    with urllib.request.urlopen(req) as resp:
        content_type = resp.headers.get("Content-Type", "")
        raw = resp.read()
        if "application/json" in content_type:
            return json.loads(raw.decode("utf-8"))
        else:
            return raw.decode("utf-8")

def main():
    print("--- 1. Bootstrap Repair Shop Preset ---")
    slug = f"speedy-repair-{int(time.time())}"
    email = f"lead-tech-{int(time.time())}@speedyrepair.com"
    bootstrap = post(f"{BASE_URL}/api/v1/admin/setup/preset", {
        "organization_name": "Speedy Tech & Auto Service",
        "slug": slug,
        "preset": "repair_shop",
        "admin_email": email,
        "admin_name": "Chief Technician Dave",
        "admin_password": "repair_secure_pass"
    })
    print("Repair Shop Bootstrapped with preset:", bootstrap["preset"])

    print("\n--- 2. Login ---")
    login = post(f"{BASE_URL}/api/v1/auth/login", {
        "email": email,
        "password": "repair_secure_pass"
    })
    token = login["token"]

    print("\n--- 3. Webhooks: Register Automation Webhook (WhatsApp / Zapier) ---")
    webhook = post(f"{BASE_URL}/api/v1/webhooks", {
        "url": "https://api.whatsapp-gateway.internal/webhook/obaf",
        "secret": "wh_sec_abc123xyz",
        "event_types": ["order.created", "order.status_updated", "stock.low"]
    }, token=token)
    print("Webhook Registered:", webhook["url"], "Event types:", webhook["event_types"])

    print("\n--- 4. Webhooks: Test Ping Trigger ---")
    ping = post(f"{BASE_URL}/api/v1/webhooks/{webhook['id']}/test", {}, token=token)
    print("Test Ping Response:", ping)

    print("\n--- 5. Catalog: Create Repair Spare Parts & Labor ---")
    labor = post(f"{BASE_URL}/api/v1/catalog/products", {
        "sku": "LABOR-01",
        "name": "Diagnostic & Labor Fee (2 Hours)",
        "price": 120.00,
        "cost_price": 40.00,
        "tax_rate": 0.18,
        "metadata": {"job_category": "electronics"}
    }, token=token)

    part = post(f"{BASE_URL}/api/v1/catalog/products", {
        "sku": "PART-SCR-X1",
        "name": "OLED Display Assembly Replacement",
        "price": 180.00,
        "cost_price": 95.00,
        "tax_rate": 0.18,
        "metadata": {"oem": True}
    }, token=token)

    print("\n--- 6. Orders: Create Repair Job Card ---")
    order = post(f"{BASE_URL}/api/v1/orders", {
        "customer_name": "Robert Downey",
        "table_or_room": "Bay #3 (Bench A)",
        "items": [
            {"product_id": labor["id"], "quantity": 1.0, "notes": "Screen flickering issue"},
            {"product_id": part["id"], "quantity": 1.0, "notes": "OEM replacement installed"}
        ]
    }, token=token)
    print("Job Card Order Created:", order["order_number"], "Grand Total: $" + str(order["grand_total"]))

    print("\n--- 7. Billing: Generate Printable HTML Invoice ---")
    html_invoice = get(f"{BASE_URL}/api/v1/billing/orders/{order['id']}/invoice.html", token=token)
    print(f"HTML Invoice successfully generated ({len(html_invoice)} bytes).")
    assert "<!DOCTYPE html>" in html_invoice
    assert "SPEEDY TECH & AUTO SERVICE" in html_invoice.upper()
    assert order["order_number"] in html_invoice
    assert "Grand Total" in html_invoice
    print("Preview snippet of HTML Invoice:")
    print("\n".join(html_invoice.splitlines()[35:45]))

    print("\nWEBHOOKS & HTML PRINTABLE INVOICE TEST PASSED! 📄⚡")

if __name__ == "__main__":
    main()
