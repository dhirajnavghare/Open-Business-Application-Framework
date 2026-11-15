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
        return json.loads(resp.read().decode("utf-8"))

def main():
    print("--- 1. Bootstrap Pharmacy Organization ---")
    slug = f"apex-pharmacy-{int(time.time())}"
    email = f"pharmacist-{int(time.time())}@apexmeds.com"
    bootstrap = post(f"{BASE_URL}/api/v1/admin/setup/preset", {
        "organization_name": "Apex Meds & Healthcare",
        "slug": slug,
        "preset": "pharmacy",
        "admin_email": email,
        "admin_name": "Dr. Sarah Pharmacist",
        "admin_password": "pharma_secure_pass"
    })
    print("Pharmacy Bootstrapped:", bootstrap["organization_name"], "Active modules:", bootstrap["active_modules"])

    print("\n--- 2. Login as Pharmacist ---")
    login = post(f"{BASE_URL}/api/v1/auth/login", {
        "email": email,
        "password": "pharma_secure_pass"
    })
    token = login["token"]
    print("Authenticated successfully.")

    print("\n--- 3. Catalog: Add Medicines with Batch & Prescription Metadata ---")
    med1 = post(f"{BASE_URL}/api/v1/catalog/products", {
        "sku": "MED-AMX-500",
        "name": "Amoxicillin 500mg Capsules (Strip of 10)",
        "price": 125.00,
        "cost_price": 70.00,
        "tax_rate": 0.12,
        "metadata": {
            "schedule": "H",
            "dosage": "500mg",
            "form": "capsule",
            "requires_prescription": True
        }
    }, token=token)
    print("Medicine 1 Created:", med1["name"], "| Price:", med1["price"], "| Tax Rate:", med1["tax_rate"])

    med2 = post(f"{BASE_URL}/api/v1/catalog/products", {
        "sku": "MED-PCM-650",
        "name": "Paracetamol 650mg Tablets (Strip of 15)",
        "price": 45.00,
        "cost_price": 20.00,
        "tax_rate": 0.12,
        "metadata": {
            "schedule": "OTC",
            "dosage": "650mg",
            "form": "tablet",
            "requires_prescription": False
        }
    }, token=token)
    print("Medicine 2 Created:", med2["name"], "| Price:", med2["price"])

    print("\n--- 4. Inventory: Record Stock Inward with Expiry Date ---")
    batch_in = post(f"{BASE_URL}/api/v1/inventory/movements", {
        "product_id": med1["id"],
        "movement_type": "RECEIPT",
        "quantity": 100.0,
        "batch_number": "BAT-2026-AMX99",
        "expiry_date": "2028-06-30T00:00:00Z",
        "notes": "Procured from Sun Pharma Distributor"
    }, token=token)
    print("Inward Stock:", batch_in["quantity"], "units | Batch:", batch_in["batch_number"], "| Expiry:", batch_in["expiry_date"])

    print("\n--- 5. Orders: Dispense Prescription to Patient ---")
    dispense = post(f"{BASE_URL}/api/v1/orders", {
        "customer_name": "Patient Emily Clark",
        "table_or_room": None,
        "items": [
            {"product_id": med1["id"], "quantity": 2.0, "notes": "Batch BAT-2026-AMX99 - Take with food"},
            {"product_id": med2["id"], "quantity": 1.0, "notes": "For fever"}
        ]
    }, token=token)
    print("Pharmacy Invoice Generated:", dispense["order_number"])
    print(f"Subtotal: {dispense['subtotal']} | 12% GST Tax: {dispense['tax_total']} | Grand Total: {dispense['grand_total']}")

    print("\n--- 6. Print Pharmacy Medical Receipt ---")
    receipt = get(f"{BASE_URL}/api/v1/billing/orders/{dispense['id']}/receipt", token=token)
    print(receipt["receipt_text"])

    print("PHARMACY DOMAIN VALIDATION SUCCESSFUL! 🩺💊")

if __name__ == "__main__":
    main()
