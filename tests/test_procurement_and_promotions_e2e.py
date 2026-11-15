import sys
import time
from pathlib import Path

# Add sdk/python to path
sys.path.insert(0, str(Path(__file__).parent.parent / "sdk" / "python"))
from obaf import ObafClient

def run_procurement_and_promotions_e2e():
    client = ObafClient(base_url="http://127.0.0.1:8080")

    # 1. Health check
    health = client.health()
    assert health["status"] == "ok"
    print("1. Health OK:", health)

    # 2. Bootstrap new tenant
    ts = int(time.time())
    admin_email = f"admin_{ts}@supplyhub.com"
    admin_password = "SuperSecurePassword123!"
    setup = client.bootstrap(
        org_name=f"Supply Chain Hub {ts}",
        slug=f"supply-hub-{ts}",
        preset="retail_grocery",
        admin_email=admin_email,
        admin_name="Operations Lead",
        admin_password=admin_password,
    )
    print("2. Tenant Bootstrapped:", setup["organization_id"])
    token = client.login(admin_email, admin_password)
    client.token = token
    print("   Logged in. JWT token acquired.")

    # 3. Create Supplier
    supp = client.create_supplier(
        name="Global Beverage Bottlers",
        contact_person="Carlos Mendoza",
        email="carlos@beveragebottlers.com",
        phone="+1-555-444-2222",
        tax_id="VAT-BEV-99",
        address="Industrial Park, Sector 4"
    )
    supplier_id = supp["id"]
    print("3. Supplier Created:", supplier_id)

    # 4. Create Product
    prod = client.create_product(
        name="Artisanal Cold Brew 330ml",
        sku=f"COLD-BREW-{ts}",
        price="4.50",
        cost_price="1.80",
        tax_rate="0.05",
        metadata={"volume_ml": 330, "caffeine_mg": 180}
    )
    product_id = prod["id"]
    print("4. Product Created:", product_id)

    # 5. Create Purchase Order for 200 bottles
    po = client.create_purchase_order(
        supplier_id=supplier_id,
        items=[{
            "product_id": product_id,
            "item_name": "Artisanal Cold Brew 330ml",
            "quantity_ordered": "200.00",
            "unit_cost": "1.80"
        }],
        tax_rate="0.05",
        currency="USD",
        notes="Pre-summer stock order"
    )
    po_id = po["id"]
    po_item_id = po["items"][0]["id"]
    assert po["status"] == "SUBMITTED"
    assert float(po["subtotal"]) == 360.00
    assert float(po["total_amount"]) == 378.00
    print("5. Purchase Order Created:", po["po_number"], "Total:", po["total_amount"])

    # 6. Receive Goods (100 units partial)
    recv1 = client.receive_purchase_order(
        po_id=po_id,
        items=[{
            "item_id": po_item_id,
            "quantity_received": "100.00",
            "batch_number": "CB-BATCH-01"
        }]
    )
    assert recv1["status"] == "PARTIALLY_RECEIVED"
    print("6. Partial Delivery Received. Status:", recv1["status"])

    bal1 = client.get_stock_balance(product_id)
    assert float(bal1["on_hand_quantity"]) == 100.00
    print("   Stock Balance:", bal1["on_hand_quantity"])

    # 7. Receive remaining 100 units
    recv2 = client.receive_purchase_order(
        po_id=po_id,
        items=[{
            "item_id": po_item_id,
            "quantity_received": "100.00",
            "batch_number": "CB-BATCH-02"
        }]
    )
    assert recv2["status"] == "RECEIVED"
    print("7. Final Delivery Received. Status:", recv2["status"])

    bal2 = client.get_stock_balance(product_id)
    assert float(bal2["on_hand_quantity"]) == 200.00
    print("   Final Stock Balance:", bal2["on_hand_quantity"])

    # 8. Promotions: Create & Validate Coupon
    client.create_coupon(
        code="SUMMERDEAL",
        discount_type="PERCENTAGE",
        discount_value="0.15",
        min_order_amount="20.00",
        usage_limit=10
    )
    val = client.validate_coupon(code="summerdeal", subtotal="50.00")
    assert val["is_valid"] is True
    assert float(val["discount_amount"]) == 7.50
    assert float(val["final_subtotal"]) == 42.50
    print("8. Coupon Validated:", val["code"], "Discount:", val["discount_amount"], "Final Subtotal:", val["final_subtotal"])

    # 9. Redeem Coupon
    red = client.redeem_coupon(code="SUMMERDEAL", subtotal="50.00")
    assert red["used_count"] == 1
    assert red["remaining_usage"] == 9
    print("9. Coupon Redeemed! Used count:", red["used_count"], "Remaining:", red["remaining_usage"])

    # 10. Multi-Currency Conversion
    client.set_exchange_rate(from_currency="USD", to_currency="EUR", rate="0.92")
    conv = client.convert_currency(amount="100.00", from_currency="USD", to_currency="EUR")
    assert float(conv["converted_amount"]) == 92.00
    print("10. Multi-Currency Conversion:", conv["original_amount"], "USD =", conv["converted_amount"], "EUR")

    print("\n🎉 ALL PROCUREMENT, PROMOTIONS & MULTI-CURRENCY E2E CHECKS PASSED PERFECTLY!")

if __name__ == "__main__":
    run_procurement_and_promotions_e2e()
