import io
import json
import unittest
from unittest.mock import MagicMock, patch
import urllib.error

from obaf import ObafClient, ObafError


class TestObafClient(unittest.TestCase):
    def test_instantiation_and_normalization(self):
        client = ObafClient(base_url="http://127.0.0.1:8080/")
        self.assertEqual(client.base_url, "http://127.0.0.1:8080")
        self.assertEqual(client.timeout, 10.0)
        self.assertIsNone(client.token)
        self.assertIsNone(client.api_key)

        client_custom = ObafClient(
            base_url="http://localhost:9000",
            token="bearer-token-123",
            api_key="api-key-456",
            timeout_seconds=25.0,
        )
        self.assertEqual(client_custom.token, "bearer-token-123")
        self.assertEqual(client_custom.api_key, "api-key-456")
        self.assertEqual(client_custom.timeout, 25.0)

    def test_all_domain_methods_exist(self):
        client = ObafClient()
        required_methods = [
            "health", "engine_info", "schema_discovery", "bootstrap", "login", "me",
            "create_product", "list_products", "get_product", "update_product", "delete_product",
            "create_customer", "list_customers", "get_customer", "update_customer", "delete_customer",
            "create_supplier", "list_suppliers", "get_supplier", "update_supplier", "delete_supplier",
            "create_order", "list_orders", "record_payment", "daily_settlement",
            "create_stock_location", "list_stock_locations", "get_stock_location",
            "update_stock_location", "delete_stock_location", "record_stock_movement",
            "list_stock_movements", "get_stock_balance",
            "open_shift", "get_current_shift", "close_shift", "list_shifts",
            "issue_credit_note", "list_credit_notes",
            "create_purchase_order", "list_purchase_orders", "get_purchase_order",
            "receive_purchase_order", "cancel_purchase_order",
            "create_coupon", "list_coupons", "get_coupon", "update_coupon", "delete_coupon",
            "validate_coupon", "redeem_coupon", "set_exchange_rate", "list_exchange_rates", "convert_currency",
            "create_webhook", "list_webhooks", "get_webhook", "update_webhook", "delete_webhook", "test_webhook",
            "create_booking", "list_bookings", "get_booking", "update_booking_status", "delete_booking",
            "create_subscription", "list_subscriptions", "get_subscription",
            "update_subscription_status", "delete_subscription", "check_in_member",
            "get_dashboard_metrics", "get_kitchen_prep_sheet", "get_sales_by_payment_method",
            "search", "sync_batch"
        ]
        for m in required_methods:
            self.assertTrue(hasattr(client, m), f"Missing method {m}")
            self.assertTrue(callable(getattr(client, m)), f"Method {m} is not callable")

    def test_mock_request_dispatch_headers(self):
        client = ObafClient(
            base_url="http://127.0.0.1:8080",
            token="secret-jwt",
            api_key="secret-api-key",
        )
        mock_response = MagicMock()
        mock_response.read.return_value = json.dumps({"status": "healthy"}).encode("utf-8")
        mock_response.headers = {"Content-Type": "application/json"}
        mock_response.__enter__.return_value = mock_response

        with patch("urllib.request.urlopen", return_value=mock_response) as mock_urlopen:
            res = client.health()
            self.assertEqual(res, {"status": "healthy"})
            req = mock_urlopen.call_args[0][0]
            self.assertEqual(req.full_url, "http://127.0.0.1:8080/health")
            self.assertEqual(req.get_method(), "GET")
            self.assertEqual(req.get_header("Authorization"), "Bearer secret-jwt")
            self.assertEqual(req.get_header("X-api-key"), "secret-api-key")

    def test_mock_post_payload(self):
        client = ObafClient()
        mock_response = MagicMock()
        mock_response.read.return_value = json.dumps({"id": "prod-1", "name": "Aspirin"}).encode("utf-8")
        mock_response.headers = {"Content-Type": "application/json"}
        mock_response.__enter__.return_value = mock_response

        with patch("urllib.request.urlopen", return_value=mock_response) as mock_urlopen:
            res = client.create_product(sku="ASP-100", name="Aspirin", price=10.0)
            self.assertEqual(res["id"], "prod-1")
            req = mock_urlopen.call_args[0][0]
            self.assertEqual(req.get_method(), "POST")
            body = json.loads(req.data.decode("utf-8"))
            self.assertEqual(body["sku"], "ASP-100")
            self.assertEqual(body["name"], "Aspirin")

    def test_mock_error_handling(self):
        client = ObafClient()
        err_body = json.dumps({"error": "Item not found"}).encode("utf-8")
        http_err = urllib.error.HTTPError(
            url="http://127.0.0.1:8080/api/v1/catalog/products/fake",
            code=404,
            msg="Not Found",
            hdrs={"Content-Type": "application/json"},
            fp=io.BytesIO(err_body),
        )

        with patch("urllib.request.urlopen", side_effect=http_err):
            with self.assertRaises(ObafError) as ctx:
                client.get_product("fake")
            self.assertEqual(ctx.exception.status_code, 404)
            self.assertIn("Item not found", ctx.exception.message)


if __name__ == "__main__":
    unittest.main()
