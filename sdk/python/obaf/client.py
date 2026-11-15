import json
import urllib.request
import urllib.error
from typing import Any, Dict, List, Optional

class ObafError(Exception):
    """Base exception for all OBAF client errors."""
    def __init__(self, message: str, status_code: Optional[int] = None, response_body: Any = None):
        super().__init__(message)
        self.status_code = status_code
        self.response_body = response_body

class ObafAuthError(ObafError):
    """Authentication or authorization failure."""
    pass

class ObafClient:
    """
    Official Python Client SDK for Open Business Application Framework (OBAF).
    Provides zero-dependency, idiomatic interaction with the OBAF REST API.
    """
    def __init__(
        self,
        base_url: str = "http://127.0.0.1:8080",
        token: Optional[str] = None,
        api_key: Optional[str] = None,
        timeout_seconds: float = 10.0,
    ):
        self.base_url = base_url.rstrip("/")
        self.token = token
        self.api_key = api_key
        self.timeout = timeout_seconds

    def _request(
        self,
        method: str,
        path: str,
        payload: Optional[Dict[str, Any]] = None,
        headers: Optional[Dict[str, str]] = None,
    ) -> Dict[str, Any]:
        url = f"{self.base_url}{path}"
        req_headers = {
            "Content-Type": "application/json",
            "Accept": "application/json",
        }
        if self.token:
            req_headers["Authorization"] = f"Bearer {self.token}"
        if self.api_key:
            req_headers["X-API-Key"] = self.api_key

        if headers:
            req_headers.update(headers)

        data = json.dumps(payload).encode("utf-8") if payload is not None else None
        req = urllib.request.Request(url, data=data, headers=req_headers, method=method)

        try:
            with urllib.request.urlopen(req, timeout=self.timeout) as resp:
                resp_bytes = resp.read()
                if not resp_bytes:
                    return {}
                return json.loads(resp_bytes.decode("utf-8"))
        except urllib.error.HTTPError as e:
            raw_err = e.read().decode("utf-8")
            try:
                parsed_err = json.loads(raw_err)
            except Exception:
                parsed_err = raw_err

            if e.code in (401, 403):
                raise ObafAuthError(f"HTTP {e.code}: {parsed_err}", status_code=e.code, response_body=parsed_err)
            raise ObafError(f"HTTP {e.code}: {parsed_err}", status_code=e.code, response_body=parsed_err)
        except urllib.error.URLError as e:
            raise ObafError(f"Connection failed to {url}: {e.reason}")

    # --- Health & Metadata ---
    def health(self) -> Dict[str, Any]:
        """Check system and database health."""
        return self._request("GET", "/health")

    def engine_info(self) -> Dict[str, Any]:
        """Discover engine metadata and available domain presets."""
        return self._request("GET", "/api/v1/meta/info")

    def schema_discovery(self) -> Dict[str, Any]:
        """Retrieve dynamic schemas for headless UI rendering."""
        return self._request("GET", "/api/v1/meta/schema")

    # --- Authentication & Organization Onboarding ---
    def bootstrap(
        self,
        org_name: str,
        slug: str,
        preset: str,
        admin_email: str,
        admin_name: str,
        admin_password: str,
    ) -> Dict[str, Any]:
        """Bootstrap a new tenant organization and primary administrator."""
        payload = {
            "organization_name": org_name,
            "slug": slug,
            "preset": preset,
            "admin_email": admin_email,
            "admin_name": admin_name,
            "admin_password": admin_password,
        }
        return self._request("POST", "/api/v1/admin/setup", payload=payload)

    def login(self, email: str, password: str) -> str:
        """Authenticate user credentials and store JWT token in client state."""
        payload = {"email": email, "password": password}
        resp = self._request("POST", "/api/v1/auth/login", payload=payload)
        token = resp.get("token")
        if token:
            self.token = token
        return token

    def me(self) -> Dict[str, Any]:
        """Get profile of current authenticated user."""
        return self._request("GET", "/api/v1/auth/me")

    # --- Catalog & Products ---
    def create_product(
        self,
        sku: str,
        name: str,
        price: str,
        tax_rate: str = "0.05",
        cost_price: str = "0.00",
        metadata: Optional[Dict[str, Any]] = None,
    ) -> Dict[str, Any]:
        """Create a new product or service item."""
        payload = {
            "sku": sku,
            "name": name,
            "price": price,
            "tax_rate": tax_rate,
            "cost_price": cost_price,
            "metadata": metadata or {},
        }
        return self._request("POST", "/api/v1/catalog/products", payload=payload)

    def list_products(self) -> List[Dict[str, Any]]:
        """List all active products in tenant catalog."""
        return self._request("GET", "/api/v1/catalog/products")

    # --- Orders & Transactions ---
    def create_order(
        self,
        items: List[Dict[str, Any]],
        customer_name: Optional[str] = None,
        table_or_room: Optional[str] = None,
        idempotency_key: Optional[str] = None,
    ) -> Dict[str, Any]:
        """Create a new order with optional idempotency key."""
        payload = {
            "customer_name": customer_name,
            "table_or_room": table_or_room,
            "items": items,
        }
        headers = {}
        if idempotency_key:
            headers["Idempotency-Key"] = idempotency_key
        return self._request("POST", "/api/v1/orders", payload=payload, headers=headers)

    def list_orders(self) -> List[Dict[str, Any]]:
        """List tenant orders."""
        return self._request("GET", "/api/v1/orders")

    # --- Payments & Settlements ---
    def record_payment(
        self,
        amount: str,
        payment_method: str,
        order_id: Optional[str] = None,
        customer_id: Optional[str] = None,
        reference_number: Optional[str] = None,
        idempotency_key: Optional[str] = None,
    ) -> Dict[str, Any]:
        """Record a payment transaction with idempotency protection."""
        payload = {
            "amount": amount,
            "payment_method": payment_method,
            "order_id": order_id,
            "customer_id": customer_id,
            "reference_number": reference_number,
        }
        headers = {}
        if idempotency_key:
            headers["Idempotency-Key"] = idempotency_key
        return self._request("POST", "/api/v1/payments", payload=payload, headers=headers)

    def daily_settlement(self) -> Dict[str, Any]:
        """Get daily payment and revenue settlement summary."""
        return self._request("GET", "/api/v1/payments/settlement")

    # --- Global Search ---
    def search(self, query: str) -> Dict[str, Any]:
        """Perform cross-entity global search."""
        encoded = urllib.parse.quote(query) if hasattr(urllib, "parse") else query
        return self._request("GET", f"/api/v1/search?q={encoded}")

    # --- Offline Edge Synchronization ---
    def sync_batch(self, client_version: int, items: List[Dict[str, Any]]) -> Dict[str, Any]:
        """Submit offline mutations for idempotent upstream batch reconciliation."""
        payload = {
            "client_version": client_version,
            "items": items,
        }
        return self._request("POST", "/api/v1/sync/batch", payload=payload)
