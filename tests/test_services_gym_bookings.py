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
    print("--- 1. Bootstrap Gym & Fitness Organization ---")
    slug = f"iron-gym-{int(time.time())}"
    email = f"manager-{int(time.time())}@irongym.com"
    bootstrap = post(f"{BASE_URL}/api/v1/admin/setup/preset", {
        "organization_name": "Iron Elite Fitness Club",
        "slug": slug,
        "preset": "gym_fitness",
        "admin_email": email,
        "admin_name": "Coach Marcus",
        "admin_password": "gym_strong_pass"
    })
    print("Gym Bootstrapped:", bootstrap["organization_name"])

    print("\n--- 2. Login via Argon2id & JWT ---")
    login = post(f"{BASE_URL}/api/v1/auth/login", {
        "email": email,
        "password": "gym_strong_pass"
    })
    token = login["token"]
    print("Logged in as Coach Marcus.")

    print("\n--- 3. Customers: Register Gym Member ---")
    member = post(f"{BASE_URL}/api/v1/customers", {
        "name": "David Miller",
        "phone": "+1-555-0199",
        "email": "david.miller@example.com",
        "address": "452 Elm Street, Suite 3B",
        "metadata": {
            "emergency_contact": "Sarah Miller",
            "fitness_goal": "Hypertrophy & Strength",
            "medical_clearance": True
        }
    }, token=token)
    print("Member Profile Created:", member["name"], "| Phone:", member["phone"])

    print("\n--- 4. Subscriptions: Create Monthly Gold Plan ---")
    sub = post(f"{BASE_URL}/api/v1/subscriptions", {
        "customer_id": member["id"],
        "plan_name": "Gold Annual Membership (Unlimited Gym + Sauna)",
        "start_date": "2026-11-15T00:00:00Z",
        "end_date": "2027-11-14T23:59:59Z",
        "amount": 799.00,
        "billing_cycle": "ANNUAL",
        "metadata": {"locker_number": "L-42", "free_pt_sessions": 2}
    }, token=token)
    print("Subscription Active:", sub["plan_name"], "| Amount: $" + str(sub["amount"]), "| Status:", sub["status"])

    print("\n--- 5. Payments: Record Member Payment via UPI / Credit Card ---")
    pay = post(f"{BASE_URL}/api/v1/payments", {
        "customer_id": member["id"],
        "amount": 799.00,
        "payment_method": "CARD",
        "reference_number": "CARD-VISA-9942"
    }, token=token)
    print("Payment Recorded:", "$" + str(pay["amount"]), "via", pay["payment_method"], "| Status:", pay["status"])

    print("\n--- 6. Bookings: Reserve Personal Training Session / Studio Slot ---")
    booking = post(f"{BASE_URL}/api/v1/bookings", {
        "customer_id": member["id"],
        "resource_name": "Studio Bay #2 (Squat Rack & Olympic Platform)",
        "start_time": "2026-11-16T08:00:00Z",
        "end_time": "2026-11-16T09:30:00Z",
        "notes": "Leg Day with Coach Marcus"
    }, token=token)
    print("Slot Booked:", booking["resource_name"], "| Status:", booking["status"])

    print("\n--- 7. Payments: End-of-Day Settlement Summary ---")
    settlement = get(f"{BASE_URL}/api/v1/payments/settlement", token=token)
    print("Daily Settlement Summary:")
    print(f"  Total Revenue: ${settlement['total_revenue']}")
    print(f"  Card Volume:   ${settlement['card_total']}")
    print(f"  Cash Volume:   ${settlement['cash_total']}")
    print(f"  UPI Volume:    ${settlement['upi_total']}")
    print(f"  Transactions:  {settlement['total_transactions']}")

    print("\nGYM & SUBSCRIPTION DOMAIN VALIDATION SUCCESSFUL! 🏋️‍♂️💪")

if __name__ == "__main__":
    main()
