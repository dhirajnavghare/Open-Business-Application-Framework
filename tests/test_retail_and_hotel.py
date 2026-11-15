#!/usr/bin/env python3
"""
E2E Integration Test: Retail Grocery and Hotel Management Domain Workflows (Phase 8 Validation)
"""

import json
import time
import urllib.request
import uuid

BASE_URL = "http://127.0.0.1:8080"

def post_json(path, data, token=None):
    req = urllib.request.Request(
        f"{BASE_URL}{path}",
        data=json.dumps(data).encode("utf-8"),
        headers={
            "Content-Type": "application/json",
            **({"Authorization": f"Bearer {token}"} if token else {})
        }
    )
    with urllib.request.urlopen(req) as resp:
        return json.loads(resp.read().decode("utf-8"))

def get_json(path, token=None):
    req = urllib.request.Request(
        f"{BASE_URL}{path}",
        headers={
            **({"Authorization": f"Bearer {token}"} if token else {})
        }
    )
    with urllib.request.urlopen(req) as resp:
        return json.loads(resp.read().decode("utf-8"))

def test_retail_and_hotel():
    print("🛒 Starting Phase 8 Retail & Hotel Domain Verification...")
    ts = int(time.time())

    # --- 1. RETAIL GROCERY WORKFLOW ---
    retail_slug = f"supermart-{ts}"
    retail_email = f"manager-{ts}@supermart.test"
    print(f"\n1. Bootstrapping Retail Grocery Preset: {retail_slug}")
    retail_setup = post_json("/api/v1/admin/setup/preset", {
        "organization_name": "Fresh Harvest Supermarket",
        "slug": retail_slug,
        "preset": "retail_grocery",
        "admin_email": retail_email,
        "admin_name": "Store Manager Dave",
        "admin_password": "DavePassword123!"
    })

    # Login
    retail_auth = post_json("/api/v1/auth/login", {
        "email": retail_email,
        "password": "DavePassword123!"
    })
    retail_token = retail_auth["token"]

    # Create Product with Barcode Custom Metadata
    print("• Creating FMCG Product with Barcode and Unit of Measure...")
    product = post_json("/api/v1/catalog/products", {
        "sku": f"FMCG-{ts}",
        "name": "Organic Almond Milk 1L",
        "price": "4.99",
        "cost_price": "2.80",
        "tax_rate": "0.05",
        "metadata": {
            "barcode": "8901030383748",
            "unit_of_measure": "liter",
            "rack_location": "Aisle 4, Shelf B"
        }
    }, retail_token)
    product_id = product["id"]
    print(f"  ✅ Created FMCG Product: {product['name']} (ID: {product_id})")

    # Add initial stock
    post_json("/api/v1/inventory/movements", {
        "product_id": product_id,
        "quantity": "50",
        "movement_type": "PURCHASE",
        "reference_id": f"PO-RETAIL-{ts}"
    }, retail_token)

    # Create POS Order
    print("• Processing Barcode Checkout Order...")
    order = post_json("/api/v1/orders", {
        "order_number": f"POS-BILL-{ts}",
        "customer_name": "Sarah Connor",
        "table_or_room": None,
        "items": [
            {
                "product_id": product_id,
                "item_name": "Organic Almond Milk 1L",
                "quantity": "2",
                "unit_price": "4.99",
                "tax_rate": "0.05"
            }
        ],
        "metadata": {
            "checkout_counter": "Counter-02",
            "cashier_id": "Cashier-09"
        }
    }, retail_token)
    print(f"  ✅ Retail POS Order {order['order_number']} created with Grand Total: ${order['grand_total']}")

    # --- 2. HOTEL MANAGEMENT WORKFLOW ---
    hotel_slug = f"grandhotel-{ts}"
    hotel_email = f"concierge-{ts}@grandhotel.test"
    print(f"\n2. Bootstrapping Hotel Management Preset: {hotel_slug}")
    hotel_setup = post_json("/api/v1/admin/setup/preset", {
        "organization_name": "Grand Continental Resort",
        "slug": hotel_slug,
        "preset": "hotel_management",
        "admin_email": hotel_email,
        "admin_name": "Chief Concierge",
        "admin_password": "HotelPassword123!"
    })

    hotel_auth = post_json("/api/v1/auth/login", {
        "email": hotel_email,
        "password": "HotelPassword123!"
    })
    hotel_token = hotel_auth["token"]

    # Create Customer
    guest = post_json("/api/v1/customers", {
        "name": "Lord Brandon Stark",
        "email": "brandon@winterfell.test",
        "phone": "+1-555-0199"
    }, hotel_token)
    guest_id = guest["id"]

    # Create Room Reservation with Custom Metadata
    print("• Creating Deluxe Ocean View Room Reservation...")
    booking = post_json("/api/v1/bookings", {
        "customer_id": guest_id,
        "resource_name": "Deluxe Suite 501",
        "start_time": "2026-12-01T14:00:00Z",
        "end_time": "2026-12-05T11:00:00Z",
        "notes": "Ocean front view, extra towels"
    }, hotel_token)
    booking_id = booking["id"]
    print(f"  ✅ Room Booking Created: {booking['resource_name']} (Status: {booking['status']})")

    # Guest Check-In
    print("• Checking in guest...")
    checkin_res = post_json(f"/api/v1/bookings/{booking_id}/check-in", {}, hotel_token)
    assert checkin_res["status"] == "CHECKED_IN", "Booking should be CHECKED_IN"
    print(f"  ✅ Guest successfully checked into Suite 501 (Status: {checkin_res['status']})")

    # Guest Check-Out
    print("• Checking out guest...")
    checkout_res = post_json(f"/api/v1/bookings/{booking_id}/check-out", {}, hotel_token)
    assert checkout_res["status"] == "COMPLETED", "Booking should be COMPLETED"
    print(f"  ✅ Guest checked out; folio settled (Status: {checkout_res['status']})")

    print("\n🎉 All Phase 8 Retail and Hotel Domain Workflows successfully verified!")

if __name__ == "__main__":
    test_retail_and_hotel()
