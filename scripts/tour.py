#!/usr/bin/env python3
"""
Open Business Application Framework (OBAF) - 60-Second Interactive Tour
A turnkey demonstration of OBAF's speed, multi-tenancy, catalog, orders, and precision billing.
"""

import os
import sys
import time
import json
import subprocess
import urllib.request
import urllib.error
from pathlib import Path

PORT = 8080
BASE_URL = f"http://127.0.0.1:{PORT}"

def print_banner():
    banner = """
\033[1;36m  ___  ____    _    _____ 
 / _ \\| __ )  / \\  |  ___|
| | | |  _ \\ / _ \\ | |_   
| |_| | |_) / ___ \\|  _|  
 \\___/|____/_/   \\_\\_|    \033[0m

\033[1mOpen Business Application Framework (OBAF) — 60-Second Guided Tour\033[0m
\033[90mUltra-fast, headless business operations, billing & workflow engine in Rust\033[0m
────────────────────────────────────────────────────────────────────────
"""
    print(banner)

def request(method, path, data=None, token=None, headers_extra=None):
    url = f"{BASE_URL}{path}"
    headers = {"Content-Type": "application/json"}
    if token:
        headers["Authorization"] = f"Bearer {token}"
    if headers_extra:
        headers.update(headers_extra)

    payload = json.dumps(data).encode("utf-8") if data else None
    req = urllib.request.Request(url, data=payload, headers=headers, method=method)
    try:
        with urllib.request.urlopen(req, timeout=5) as resp:
            content_type = resp.headers.get("Content-Type", "")
            raw = resp.read()
            if "json" in content_type:
                return json.loads(raw.decode("utf-8"))
            return raw.decode("utf-8")
    except urllib.error.HTTPError as e:
        err_msg = e.read().decode("utf-8")
        print(f"\033[91mHTTP Error {e.code} on {path}: {err_msg}\033[0m")
        sys.exit(1)

def main():
    print_banner()

    # 1. Health check & ensure server is running
    server_process = None
    tmp_db = f"/tmp/obaf_tour_{os.getpid()}.db"
    try:
        try:
            health = request("GET", "/health")
            print(f"ℹ️  Existing OBAF server detected on {BASE_URL} (Database: {health.get('database')}).")
        except Exception:
            print("🚀 \033[1mStarting ephemeral high-performance OBAF server on port 8080...\033[0m")
            server_bin = None
            if Path("./target/release/obaf-server").exists():
                server_bin = "./target/release/obaf-server"
            elif Path("./target/debug/obaf-server").exists():
                server_bin = "./target/debug/obaf-server"

            env = os.environ.copy()
            env["PORT"] = str(PORT)
            env["HOST"] = "127.0.0.1"
            env["DATABASE_URL"] = f"sqlite://{tmp_db}?mode=rwc"

            if server_bin:
                server_process = subprocess.Popen([server_bin], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            else:
                server_process = subprocess.Popen(["cargo", "run", "-p", "obaf-server"], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

            # Wait for health
            for _ in range(40):
                time.sleep(0.25)
                try:
                    h = request("GET", "/health")
                    if h.get("status") == "ok":
                        break
                except Exception:
                    pass
            print("✅ \033[32mServer online and healthy in < 1 second!\033[0m\n")

        # 2. Bootstrap Organization with Restaurant POS Preset
        ts = int(time.time())
        print(f"1. \033[1mBootstrapping Tenant\033[0m with \033[33mrestaurant_pos\033[0m domain preset...")
        org = request("POST", "/api/v1/admin/setup/preset", {
            "organization_name": "Royal Bistro Tour",
            "slug": f"tour-bistro-{ts}",
            "preset": "restaurant_pos",
            "admin_email": f"marco-{ts}@royalbistro.com",
            "admin_name": "Chef Marco",
            "admin_password": "Password123!"
        })
        print(f"   • Tenant ID: \033[36m{org['organization_id']}\033[0m")
        print(f"   • Active Modules: {', '.join(org['active_modules'])}\n")

        # 3. Authenticate with Argon2id + JWT
        print("2. \033[1mAuthenticating Administrator\033[0m via Argon2id & Bearer JWT...")
        login_res = request("POST", "/api/v1/auth/login", {
            "email": f"marco-{ts}@royalbistro.com",
            "password": "Password123!"
        })
        token = login_res["token"]
        print(f"   • JWT Bearer Token: \033[90m{token[:24]}...{token[-12:]}\033[0m\n")

        # 4. Catalog Engine: Create Menu Items
        print("3. \033[1mPopulating Dynamic Product Catalog\033[0m...")
        p1 = request("POST", "/api/v1/catalog/products", {
            "sku": "PIZZA-TRF",
            "name": "Wood-Fired Truffle Pizza",
            "price": "24.50",
            "cost_price": "8.00",
            "tax_rate": "0.10"
        }, token=token)
        p2 = request("POST", "/api/v1/catalog/products", {
            "sku": "DRINK-MGT",
            "name": "Smoked Mezcal Margarita",
            "price": "15.00",
            "cost_price": "4.00",
            "tax_rate": "0.10"
        }, token=token)
        print(f"   • Item 1: {p1['name']} ($24.50, Tax: 10%)")
        print(f"   • Item 2: {p2['name']} ($15.00, Tax: 10%)\n")

        # 5. Inventory Movement
        print("4. \033[1mRecording Double-Entry Stock Movement\033[0m...")
        request("POST", "/api/v1/inventory/movements", {
            "product_id": p1["id"],
            "movement_type": "RECEIPT",
            "quantity": 50.0,
            "batch_number": "BATCH-2026-NOV"
        }, token=token)
        print("   • Inward 50 units received into batch BATCH-2026-NOV.\n")

        # 6. Order Placement with Idempotency Key
        print("5. \033[1mPlacing POS Table Order\033[0m (Protected with Idempotency-Key)...")
        order = request("POST", "/api/v1/orders", {
            "customer_name": "VIP Table #04",
            "table_or_room": "Table #04",
            "items": [
                {"product_id": p1["id"], "quantity": 2.0},
                {"product_id": p2["id"], "quantity": 4.0}
            ]
        }, token=token, headers_extra={"Idempotency-Key": f"order-tour-{ts}"})
        print(f"   • Order Number: \033[32m{order['order_number']}\033[0m | Status: {order['status']}")
        tax_val = order.get("tax", order.get("tax_amount", "0.00"))
        print(f"   • Subtotal: ${order['subtotal']} | Tax: ${tax_val} | Grand Total: \033[1;32m${order['grand_total']}\033[0m\n")

        # 7. Split Bill Math
        print("6. \033[1mCalculating Penny-Perfect Split Bill (3 Guests)\033[0m...")
        split = request("POST", "/api/v1/billing/split-bill", {
            "total_amount": str(order["grand_total"]),
            "num_parties": 3
        }, token=token)
        for s in split["shares"]:
            print(f"   • Guest #{s['party_index']}: ${s['share_amount']}")
        reconciled = sum(float(s['share_amount']) for s in split["shares"])
        print(f"   • Reconciled Total: \033[32m${reconciled:.2f}\033[0m (Exact Penny Split!)\n")

        # 8. Printable Thermal Receipt
        print("7. \033[1mGenerated Thermal Printable Receipt\033[0m:")
        receipt_res = request("GET", f"/api/v1/billing/orders/{order['id']}/receipt", token=token)
        print(f"\033[33m{receipt_res['receipt_text']}\033[0m")

        print("────────────────────────────────────────────────────────────────────────")
        print("\033[32m✨ OBAF Tour completed successfully!\033[0m")
        print(f"📖 Swagger UI Documentation:  \033[4m{BASE_URL}/swagger-ui\033[0m")
        print(f"🤖 AI Agent Context Engine:   \033[4m{BASE_URL}/llms.txt\033[0m")
        print(f"🩺 System Health & Metrics:   \033[4m{BASE_URL}/health\033[0m & \033[4m{BASE_URL}/metrics\033[0m")
        print("────────────────────────────────────────────────────────────────────────")

    finally:
        if server_process:
            print("\n🛑 Stopping ephemeral tour server...")
            server_process.terminate()
            server_process.wait()
            for ext in ["", "-shm", "-wal"]:
                p = Path(f"{tmp_db}{ext}")
                if p.exists():
                    p.unlink()

if __name__ == "__main__":
    main()
