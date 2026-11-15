import io
import json
import sys
from pathlib import Path
from unittest.mock import MagicMock, patch
import urllib.error

# Add sdk/python to path
sys.path.insert(0, str(Path(__file__).parent.parent / "sdk" / "python"))

from obaf import ObafClient, ObafError


def test_sdk_instantiation_and_normalization():
    # Trailing slash stripping
    client1 = ObafClient(base_url="http://127.0.0.1:8080/")
    assert client1.base_url == "http://127.0.0.1:8080"
    assert client1.timeout == 10.0
    assert client1.token is None
    assert client1.api_key is None

    # Custom token and timeout
    client2 = ObafClient(
        base_url="http://localhost:9000",
        token="jwt-token-xyz",
        api_key="api-key-123",
        timeout_seconds=15.0,
    )
    assert client2.token == "jwt-token-xyz"
    assert client2.api_key == "api-key-123"
    assert client2.timeout == 15.0
    print("✅ test_sdk_instantiation_and_normalization passed")


def test_sdk_method_inventory():
    client = ObafClient()

    required_methods = [
        # System & Auth
        "health", "engine_info", "schema_discovery", "bootstrap", "login", "me",
        # Catalog
        "create_product", "list_products", "get_product", "update_product", "delete_product",
        # Customers
        "create_customer", "list_customers", "get_customer", "update_customer", "delete_customer",
        # Suppliers
        "create_supplier", "list_suppliers", "get_supplier", "update_supplier", "delete_supplier",
        # Orders & Payments
        "create_order", "list_orders", "record_payment", "daily_settlement",
        # Stock & Inventory Locations
        "create_stock_location", "list_stock_locations", "get_stock_location",
        "update_stock_location", "delete_stock_location", "record_stock_movement",
        "list_stock_movements", "get_stock_balance",
        # POS & Shifts
        "open_shift", "get_current_shift", "close_shift", "list_shifts",
        "issue_credit_note", "list_credit_notes",
        # Procurement
        "create_purchase_order", "list_purchase_orders", "get_purchase_order",
        "receive_purchase_order", "cancel_purchase_order",
        # Promotions & Multi-Currency
        "create_coupon", "list_coupons", "get_coupon", "update_coupon", "delete_coupon",
        "validate_coupon", "redeem_coupon", "set_exchange_rate", "list_exchange_rates", "convert_currency",
        # Webhooks
        "create_webhook", "list_webhooks", "get_webhook", "update_webhook", "delete_webhook", "test_webhook",
        # Bookings & Subscriptions
        "create_booking", "list_bookings", "get_booking", "update_booking_status", "delete_booking",
        "create_subscription", "list_subscriptions", "get_subscription",
        "update_subscription_status", "delete_subscription", "check_in_member",
        # Reports & Sync
        "get_dashboard_metrics", "get_kitchen_prep_sheet", "get_sales_by_payment_method",
        "search", "sync_batch"
    ]

    for m in required_methods:
        assert hasattr(client, m), f"ObafClient is missing required method: {m}"
        assert callable(getattr(client, m)), f"Attribute {m} is not callable"

    print(f"✅ test_sdk_method_inventory passed ({len(required_methods)} methods verified)")


def test_sdk_mock_request_dispatch():
    client = ObafClient(
        base_url="http://127.0.0.1:8080",
        token="jwt-token-mock",
        api_key="api-key-mock",
    )

    # Mock response
    mock_response = MagicMock()
    mock_response.read.return_value = json.dumps({"status": "healthy", "version": "0.1.0"}).encode("utf-8")
    mock_response.headers = {"Content-Type": "application/json"}
    mock_response.__enter__.return_value = mock_response

    with patch("urllib.request.urlopen", return_value=mock_response) as mock_urlopen:
        res = client.health()
        assert res == {"status": "healthy", "version": "0.1.0"}

        # Verify request parameters
        call_args = mock_urlopen.call_args
        req = call_args[0][0]
        assert req.full_url == "http://127.0.0.1:8080/health"
        assert req.get_method() == "GET"
        assert req.get_header("Authorization") == "Bearer jwt-token-mock"
        assert req.get_header("X-api-key") == "api-key-mock"

    print("✅ test_sdk_mock_request_dispatch passed")


def test_sdk_mock_post_payload_and_error():
    client = ObafClient(base_url="http://127.0.0.1:8080")

    # Mock HTTP 404 error
    err_body = json.dumps({"error": "Item not found"}).encode("utf-8")
    http_err = urllib.error.HTTPError(
        url="http://127.0.0.1:8080/api/v1/catalog/products/fake",
        code=404,
        msg="Not Found",
        hdrs={"Content-Type": "application/json"},
        fp=io.BytesIO(err_body),
    )

    with patch("urllib.request.urlopen", side_effect=http_err):
        try:
            client.get_product("fake")
            assert False, "Should have raised ObafError"
        except ObafError as e:
            assert e.status_code == 404
            assert "Item not found" in e.message

    print("✅ test_sdk_mock_post_payload_and_error passed")


if __name__ == "__main__":
    test_sdk_instantiation_and_normalization()
    test_sdk_method_inventory()
    test_sdk_mock_request_dispatch()
    test_sdk_mock_post_payload_and_error()
    print("🎉 All Python SDK automated tests passed!")
