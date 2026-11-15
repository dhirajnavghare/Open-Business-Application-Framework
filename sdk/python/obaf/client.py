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

    # --- Customers & Suppliers ---
    def create_customer(
        self,
        name: str,
        phone: Optional[str] = None,
        email: Optional[str] = None,
        metadata: Optional[Dict[str, Any]] = None,
    ) -> Dict[str, Any]:
        """Create a new customer profile."""
        payload = {
            "name": name,
            "phone": phone,
            "email": email,
            "metadata": metadata or {},
        }
        return self._request("POST", "/api/v1/customers", payload=payload)

    def list_customers(self) -> List[Dict[str, Any]]:
        """List tenant customers."""
        return self._request("GET", "/api/v1/customers")

    def get_customer(self, customer_id: str) -> Dict[str, Any]:
        """Get customer profile by ID."""
        return self._request("GET", f"/api/v1/customers/{customer_id}")

    def create_supplier(
        self,
        name: str,
        contact_person: Optional[str] = None,
        email: Optional[str] = None,
        phone: Optional[str] = None,
        tax_id: Optional[str] = None,
        address: Optional[str] = None,
        metadata: Optional[Dict[str, Any]] = None,
    ) -> Dict[str, Any]:
        """Create a vendor/supplier profile."""
        payload = {
            "name": name,
            "contact_person": contact_person,
            "email": email,
            "phone": phone,
            "tax_id": tax_id,
            "address": address,
            "metadata": metadata or {},
        }
        return self._request("POST", "/api/v1/suppliers", payload=payload)

    def list_suppliers(self) -> List[Dict[str, Any]]:
        """List tenant suppliers."""
        return self._request("GET", "/api/v1/suppliers")

    def get_supplier(self, supplier_id: str) -> Dict[str, Any]:
        """Get supplier profile by ID."""
        return self._request("GET", f"/api/v1/suppliers/{supplier_id}")

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

    # --- Multi-Location Warehouses & Inventory Ledger ---
    def create_stock_location(self, name: str, code: str, address: Optional[str] = None) -> Dict[str, Any]:
        """Create a physical stock location or warehouse."""
        payload = {"name": name, "code": code, "address": address}
        return self._request("POST", "/api/v1/inventory/locations", payload=payload)

    def list_stock_locations(self) -> List[Dict[str, Any]]:
        """List active stock locations."""
        return self._request("GET", "/api/v1/inventory/locations")

    def record_stock_movement(
        self,
        product_id: str,
        movement_type: str,
        quantity: str,
        batch_number: Optional[str] = None,
        expiry_date: Optional[str] = None,
        reference_id: Optional[str] = None,
        notes: Optional[str] = None,
    ) -> Dict[str, Any]:
        """Record an inventory ledger stock movement."""
        payload = {
            "product_id": product_id,
            "movement_type": movement_type,
            "quantity": quantity,
            "batch_number": batch_number,
            "expiry_date": expiry_date,
            "reference_id": reference_id,
            "notes": notes,
        }
        return self._request("POST", "/api/v1/inventory/movements", payload=payload)

    def list_stock_movements(self, product_id: Optional[str] = None) -> List[Dict[str, Any]]:
        """List inventory ledger stock movements."""
        qs = f"?product_id={product_id}" if product_id else ""
        return self._request("GET", f"/api/v1/inventory/movements{qs}")

    def get_stock_balance(self, product_id: str) -> Dict[str, Any]:
        """Get net on-hand stock balance calculated from ledger movements."""
        return self._request("GET", f"/api/v1/inventory/balance/{product_id}")

    # --- POS Cash Drawer & Shift Management ---
    def open_shift(self, register_name: str, opening_float: str) -> Dict[str, Any]:
        """Open a new cashier drawer shift with opening float."""
        payload = {"register_name": register_name, "opening_float": opening_float}
        return self._request("POST", "/api/v1/pos/shifts/open", payload=payload)

    def get_current_shift(self) -> Dict[str, Any]:
        """Get active shift details for current cashier."""
        return self._request("GET", "/api/v1/pos/shifts/current")

    def close_shift(self, shift_id: str, counted_cash: str, notes: Optional[str] = None) -> Dict[str, Any]:
        """Close cashier shift and reconcile counted cash variance."""
        payload = {"counted_cash": counted_cash, "notes": notes}
        return self._request("POST", f"/api/v1/pos/shifts/{shift_id}/close", payload=payload)

    def list_shifts(self) -> List[Dict[str, Any]]:
        """List historical POS shifts."""
        return self._request("GET", "/api/v1/pos/shifts")

    # --- Credit Notes ---
    def issue_credit_note(
        self,
        amount: str,
        tax_amount: str,
        reason: str,
        order_id: Optional[str] = None,
        customer_id: Optional[str] = None,
    ) -> Dict[str, Any]:
        """Issue an immutable financial credit note."""
        payload = {
            "amount": amount,
            "tax_amount": tax_amount,
            "reason": reason,
            "order_id": order_id,
            "customer_id": customer_id,
        }
        return self._request("POST", "/api/v1/billing/credit-notes", payload=payload)

    def list_credit_notes(self) -> List[Dict[str, Any]]:
        """List all credit notes."""
        return self._request("GET", "/api/v1/billing/credit-notes")

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

    # --- Procurement & Purchase Orders ---
    def create_purchase_order(
        self,
        supplier_id: str,
        items: List[Dict[str, Any]],
        expected_delivery_date: Optional[str] = None,
        currency: Optional[str] = "USD",
        tax_rate: Optional[str] = None,
        notes: Optional[str] = None,
    ) -> Dict[str, Any]:
        """Create a new purchase order with line items."""
        payload = {
            "supplier_id": supplier_id,
            "items": items,
            "expected_delivery_date": expected_delivery_date,
            "currency": currency,
            "tax_rate": tax_rate,
            "notes": notes,
        }
        return self._request("POST", "/api/v1/procurement/purchase-orders", payload=payload)

    def list_purchase_orders(
        self,
        status: Optional[str] = None,
        supplier_id: Optional[str] = None,
    ) -> List[Dict[str, Any]]:
        """List purchase orders with optional filtering."""
        params = []
        if status:
            params.append(f"status={status}")
        if supplier_id:
            params.append(f"supplier_id={supplier_id}")
        query_str = f"?{'&'.join(params)}" if params else ""
        return self._request("GET", f"/api/v1/procurement/purchase-orders{query_str}")

    def get_purchase_order(self, po_id: str) -> Dict[str, Any]:
        """Retrieve purchase order details by ID."""
        return self._request("GET", f"/api/v1/procurement/purchase-orders/{po_id}")

    def receive_purchase_order(
        self,
        po_id: str,
        items: List[Dict[str, Any]],
    ) -> Dict[str, Any]:
        """Receive goods against a purchase order and create inbound stock movements."""
        payload = {"items": items}
        return self._request("POST", f"/api/v1/procurement/purchase-orders/{po_id}/receive", payload=payload)

    def cancel_purchase_order(self, po_id: str) -> Dict[str, Any]:
        """Cancel an unfulfilled purchase order."""
        return self._request("POST", f"/api/v1/procurement/purchase-orders/{po_id}/cancel")

    # --- Promotions & Coupons ---
    def create_coupon(
        self,
        code: str,
        discount_type: str,
        discount_value: str,
        min_order_amount: Optional[str] = None,
        max_discount_amount: Optional[str] = None,
        usage_limit: Optional[int] = None,
        valid_from: Optional[str] = None,
        valid_until: Optional[str] = None,
        is_active: Optional[bool] = True,
    ) -> Dict[str, Any]:
        """Create a promotional coupon code."""
        payload = {
            "code": code,
            "discount_type": discount_type,
            "discount_value": discount_value,
            "min_order_amount": min_order_amount,
            "max_discount_amount": max_discount_amount,
            "usage_limit": usage_limit,
            "valid_from": valid_from,
            "valid_until": valid_until,
            "is_active": is_active,
        }
        return self._request("POST", "/api/v1/promotions/coupons", payload=payload)

    def list_coupons(self, active_only: Optional[bool] = None) -> List[Dict[str, Any]]:
        """List promotional coupons."""
        query_str = "?active_only=true" if active_only else ""
        return self._request("GET", f"/api/v1/promotions/coupons{query_str}")

    def validate_coupon(self, code: str, subtotal: str) -> Dict[str, Any]:
        """Validate coupon applicability and compute discount amounts."""
        payload = {"code": code, "subtotal": subtotal}
        return self._request("POST", "/api/v1/promotions/coupons/validate", payload=payload)

    def redeem_coupon(self, code: str, subtotal: str) -> Dict[str, Any]:
        """Redeem a coupon and atomically decrement remaining allowance."""
        payload = {"subtotal": subtotal}
        return self._request("POST", f"/api/v1/promotions/coupons/{code}/redeem", payload=payload)

    # --- Multi-Currency Exchange Rates & Conversion ---
    def set_exchange_rate(self, from_currency: str, to_currency: str, rate: str) -> Dict[str, Any]:
        """Configure or update an exchange rate between two ISO-4217 currencies."""
        payload = {
            "from_currency": from_currency,
            "to_currency": to_currency,
            "rate": rate,
        }
        return self._request("POST", "/api/v1/billing/exchange-rates", payload=payload)

    def list_exchange_rates(self) -> List[Dict[str, Any]]:
        """List configured currency exchange rates."""
        return self._request("GET", "/api/v1/billing/exchange-rates")

    def convert_currency(self, amount: str, from_currency: str, to_currency: str) -> Dict[str, Any]:
        """Convert a monetary amount using configured direct or reciprocal exchange rates."""
        payload = {
            "amount": amount,
            "from_currency": from_currency,
            "to_currency": to_currency,
        }
        return self._request("POST", "/api/v1/billing/convert-currency", payload=payload)

    # --- Webhooks Management ---
    def create_webhook(
        self,
        url: str,
        event_types: List[str],
        secret: Optional[str] = None,
    ) -> Dict[str, Any]:
        """Register a new event webhook endpoint."""
        payload = {
            "url": url,
            "event_types": event_types,
            "secret": secret,
        }
        return self._request("POST", "/api/v1/webhooks", payload=payload)

    def list_webhooks(self) -> List[Dict[str, Any]]:
        """List registered webhooks for the tenant."""
        return self._request("GET", "/api/v1/webhooks")

    def get_webhook(self, webhook_id: str) -> Dict[str, Any]:
        """Get details of a specific webhook."""
        return self._request("GET", f"/api/v1/webhooks/{webhook_id}")

    def update_webhook(
        self,
        webhook_id: str,
        url: Optional[str] = None,
        event_types: Optional[List[str]] = None,
        secret: Optional[str] = None,
        is_active: Optional[bool] = None,
    ) -> Dict[str, Any]:
        """Update a webhook's configuration or active state."""
        payload = {}
        if url is not None:
            payload["url"] = url
        if event_types is not None:
            payload["event_types"] = event_types
        if secret is not None:
            payload["secret"] = secret
        if is_active is not None:
            payload["is_active"] = is_active
        return self._request("PATCH", f"/api/v1/webhooks/{webhook_id}", payload=payload)

    def delete_webhook(self, webhook_id: str) -> Dict[str, Any]:
        """Delete a registered webhook."""
        return self._request("DELETE", f"/api/v1/webhooks/{webhook_id}")

    def test_webhook(self, webhook_id: str) -> Dict[str, Any]:
        """Dispatch a test ping payload to a registered webhook."""
        return self._request("POST", f"/api/v1/webhooks/{webhook_id}/test")

    # --- Bookings & Scheduling ---
    def create_booking(
        self,
        resource_id: str,
        start_time: str,
        end_time: str,
        booking_type: str,
        customer_id: Optional[str] = None,
        notes: Optional[str] = None,
    ) -> Dict[str, Any]:
        """Create a resource or appointment reservation."""
        payload = {
            "resource_id": resource_id,
            "start_time": start_time,
            "end_time": end_time,
            "booking_type": booking_type,
            "customer_id": customer_id,
            "notes": notes,
        }
        return self._request("POST", "/api/v1/bookings", payload=payload)

    def list_bookings(self, status: Optional[str] = None) -> List[Dict[str, Any]]:
        """List reservations with optional status filter."""
        query_str = f"?status={status}" if status else ""
        return self._request("GET", f"/api/v1/bookings{query_str}")

    def get_booking(self, booking_id: str) -> Dict[str, Any]:
        """Get booking details by ID."""
        return self._request("GET", f"/api/v1/bookings/{booking_id}")

    def update_booking_status(self, booking_id: str, status: str) -> Dict[str, Any]:
        """Update reservation state (e.g. CONFIRMED, CHECKED_IN, COMPLETED, CANCELLED)."""
        payload = {"status": status}
        return self._request("PATCH", f"/api/v1/bookings/{booking_id}/status", payload=payload)

    # --- Subscriptions & Memberships ---
    def create_subscription(
        self,
        customer_id: str,
        plan_name: str,
        amount: str,
        billing_interval: str = "monthly",
        start_date: Optional[str] = None,
        renewal_date: Optional[str] = None,
    ) -> Dict[str, Any]:
        """Create a recurring customer membership subscription."""
        payload = {
            "customer_id": customer_id,
            "plan_name": plan_name,
            "amount": amount,
            "billing_interval": billing_interval,
            "start_date": start_date,
            "renewal_date": renewal_date,
        }
        return self._request("POST", "/api/v1/subscriptions", payload=payload)

    def list_subscriptions(self, status: Optional[str] = None) -> List[Dict[str, Any]]:
        """List customer subscriptions with optional status filter."""
        query_str = f"?status={status}" if status else ""
        return self._request("GET", f"/api/v1/subscriptions{query_str}")

    def get_subscription(self, subscription_id: str) -> Dict[str, Any]:
        """Get subscription details by ID."""
        return self._request("GET", f"/api/v1/subscriptions/{subscription_id}")

    def update_subscription_status(self, subscription_id: str, status: str) -> Dict[str, Any]:
        """Update subscription status (e.g. ACTIVE, PAUSED, CANCELLED)."""
        payload = {"status": status}
        return self._request("PATCH", f"/api/v1/subscriptions/{subscription_id}/status", payload=payload)

    def check_in_member(self, subscription_id: str, notes: Optional[str] = None) -> Dict[str, Any]:
        """Record member physical attendance check-in against active subscription."""
        payload = {"notes": notes}
        return self._request("POST", f"/api/v1/subscriptions/{subscription_id}/checkin", payload=payload)

    # --- Reports & Analytics ---
    def get_dashboard_metrics(self) -> Dict[str, Any]:
        """Retrieve high-level operational metrics and revenue summary."""
        return self._request("GET", "/api/v1/reports/dashboard")

    def get_kitchen_prep_sheet(self) -> Dict[str, Any]:
        """Retrieve active kitchen prep sheet aggregating open pending order quantities."""
        return self._request("GET", "/api/v1/reports/kitchen-prep")

    def get_sales_by_payment_method(self) -> Dict[str, Any]:
        """Retrieve aggregated sales volume broken down by payment tender."""
        return self._request("GET", "/api/v1/reports/sales-by-payment-method")


