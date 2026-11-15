export interface ObafClientConfig {
  baseUrl?: string;
  token?: string;
  apiKey?: string;
}

export class ObafError extends Error {
  constructor(
    message: string,
    public statusCode?: number,
    public responseBody?: any
  ) {
    super(message);
    this.name = "ObafError";
  }
}

export class ObafClient {
  public baseUrl: string;
  public token?: string;
  public apiKey?: string;

  constructor(config: ObafClientConfig = {}) {
    this.baseUrl = (config.baseUrl || "http://127.0.0.1:8080").replace(/\/+$/, "");
    this.token = config.token;
    this.apiKey = config.apiKey;
  }

  private async request<T = any>(
    method: string,
    path: string,
    body?: any,
    customHeaders: Record<string, string> = {}
  ): Promise<T> {
    const url = `${this.baseUrl}${path}`;
    const headers: Record<string, string> = {
      "Content-Type": "application/json",
      Accept: "application/json",
      ...customHeaders,
    };

    if (this.token) {
      headers["Authorization"] = `Bearer ${this.token}`;
    }
    if (this.apiKey) {
      headers["X-API-Key"] = this.apiKey;
    }

    const res = await fetch(url, {
      method,
      headers,
      body: body ? JSON.stringify(body) : undefined,
    });

    const isJson = res.headers.get("content-type")?.includes("application/json");
    const data = isJson ? await res.json() : await res.text();

    if (!res.ok) {
      throw new ObafError(
        `HTTP ${res.status}: ${typeof data === "string" ? data : JSON.stringify(data)}`,
        res.status,
        data
      );
    }

    return data as T;
  }

  // --- Health & Discovery ---
  async health(): Promise<{ status: string; version: string; database: string }> {
    return this.request("GET", "/health");
  }

  async engineInfo(): Promise<any> {
    return this.request("GET", "/api/v1/meta/info");
  }

  async schemaDiscovery(): Promise<any> {
    return this.request("GET", "/api/v1/meta/schema");
  }

  // --- Authentication & Onboarding ---
  async bootstrap(payload: {
    organization_name: string;
    slug: string;
    preset: string;
    admin_email: string;
    admin_name: string;
    admin_password: string;
  }): Promise<any> {
    return this.request("POST", "/api/v1/admin/setup", payload);
  }

  async login(email: string, password: string): Promise<string> {
    const res = await this.request<{ token: string }>("POST", "/api/v1/auth/login", {
      email,
      password,
    });
    this.token = res.token;
    return res.token;
  }

  async me(): Promise<any> {
    return this.request("GET", "/api/v1/auth/me");
  }

  // --- Customers ---
  async createCustomer(payload: {
    name: string;
    phone?: string;
    email?: string;
    tax_id?: string;
    address?: string;
    metadata?: Record<string, any>;
  }): Promise<any> {
    return this.request("POST", "/api/v1/customers", payload);
  }

  async listCustomers(): Promise<any[]> {
    return this.request("GET", "/api/v1/customers");
  }

  async getCustomer(customerId: string): Promise<any> {
    return this.request("GET", `/api/v1/customers/${customerId}`);
  }

  async updateCustomer(
    customerId: string,
    payload: {
      name?: string;
      phone?: string;
      email?: string;
      tax_id?: string;
      address?: string;
      metadata?: Record<string, any>;
    }
  ): Promise<any> {
    return this.request("PATCH", `/api/v1/customers/${customerId}`, payload);
  }

  async deleteCustomer(customerId: string): Promise<any> {
    return this.request("DELETE", `/api/v1/customers/${customerId}`);
  }

  // --- Suppliers ---
  async createSupplier(payload: {
    name: string;
    contact_person?: string;
    phone?: string;
    email?: string;
    tax_id?: string;
    address?: string;
    metadata?: Record<string, any>;
  }): Promise<any> {
    return this.request("POST", "/api/v1/suppliers", payload);
  }

  async listSuppliers(): Promise<any[]> {
    return this.request("GET", "/api/v1/suppliers");
  }

  async getSupplier(supplierId: string): Promise<any> {
    return this.request("GET", `/api/v1/suppliers/${supplierId}`);
  }

  async updateSupplier(
    supplierId: string,
    payload: {
      name?: string;
      contact_person?: string;
      phone?: string;
      email?: string;
      tax_id?: string;
      address?: string;
      metadata?: Record<string, any>;
      is_active?: boolean;
    }
  ): Promise<any> {
    return this.request("PATCH", `/api/v1/suppliers/${supplierId}`, payload);
  }

  async deleteSupplier(supplierId: string): Promise<any> {
    return this.request("DELETE", `/api/v1/suppliers/${supplierId}`);
  }

  // --- Catalog ---
  async createProduct(payload: {
    sku: string;
    name: string;
    price: string;
    tax_rate?: string;
    cost_price?: string;
    metadata?: Record<string, any>;
  }): Promise<any> {
    return this.request("POST", "/api/v1/catalog/products", payload);
  }

  async listProducts(): Promise<any[]> {
    return this.request("GET", "/api/v1/catalog/products");
  }

  async getProduct(productId: string): Promise<any> {
    return this.request("GET", `/api/v1/catalog/products/${productId}`);
  }

  async updateProduct(
    productId: string,
    payload: {
      sku?: string;
      name?: string;
      price?: string;
      tax_rate?: string;
      cost_price?: string;
      metadata?: Record<string, any>;
      is_active?: boolean;
    }
  ): Promise<any> {
    return this.request("PATCH", `/api/v1/catalog/products/${productId}`, payload);
  }

  async deleteProduct(productId: string): Promise<any> {
    return this.request("DELETE", `/api/v1/catalog/products/${productId}`);
  }

  // --- Orders & Transactions ---
  async createOrder(
    payload: {
      customer_name?: string;
      table_or_room?: string;
      items: Array<{ product_id: string; quantity: number | string; notes?: string }>;
    },
    idempotencyKey?: string
  ): Promise<any> {
    const headers: Record<string, string> = {};
    if (idempotencyKey) {
      headers["Idempotency-Key"] = idempotencyKey;
    }
    return this.request("POST", "/api/v1/orders", payload, headers);
  }

  async listOrders(): Promise<any[]> {
    return this.request("GET", "/api/v1/orders");
  }

  // --- Payments & Settlement ---
  async recordPayment(
    payload: {
      amount: string;
      payment_method: string;
      order_id?: string;
      customer_id?: string;
      reference_number?: string;
    },
    idempotencyKey?: string
  ): Promise<any> {
    const headers: Record<string, string> = {};
    if (idempotencyKey) {
      headers["Idempotency-Key"] = idempotencyKey;
    }
    return this.request("POST", "/api/v1/payments", payload, headers);
  }

  async dailySettlement(): Promise<any> {
    return this.request("GET", "/api/v1/payments/settlement");
  }

  // --- Multi-Location Warehouses ---
  async createStockLocation(payload: {
    name: string;
    code: string;
    address?: string;
  }): Promise<any> {
    return this.request("POST", "/api/v1/inventory/locations", payload);
  }

  async listStockLocations(): Promise<any[]> {
    return this.request("GET", "/api/v1/inventory/locations");
  }

  async getStockLocation(locationId: string): Promise<any> {
    return this.request("GET", `/api/v1/inventory/locations/${locationId}`);
  }

  async updateStockLocation(
    locationId: string,
    payload: {
      name?: string;
      code?: string;
      address?: string;
      is_active?: boolean;
    }
  ): Promise<any> {
    return this.request("PATCH", `/api/v1/inventory/locations/${locationId}`, payload);
  }

  async deleteStockLocation(locationId: string): Promise<any> {
    return this.request("DELETE", `/api/v1/inventory/locations/${locationId}`);
  }

  // --- POS Cash Drawer & Shift Management ---
  async openShift(payload: {
    register_name: string;
    opening_float: string;
  }): Promise<any> {
    return this.request("POST", "/api/v1/pos/shifts/open", payload);
  }

  async getCurrentShift(): Promise<any> {
    return this.request("GET", "/api/v1/pos/shifts/current");
  }

  async closeShift(
    shiftId: string,
    payload: { counted_cash: string; notes?: string }
  ): Promise<any> {
    return this.request("POST", `/api/v1/pos/shifts/${shiftId}/close`, payload);
  }

  async listShifts(): Promise<any[]> {
    return this.request("GET", "/api/v1/pos/shifts");
  }

  // --- Credit Notes ---
  async issueCreditNote(payload: {
    amount: string;
    tax_amount: string;
    reason: string;
    order_id?: string;
    customer_id?: string;
  }): Promise<any> {
    return this.request("POST", "/api/v1/billing/credit-notes", payload);
  }

  async listCreditNotes(): Promise<any[]> {
    return this.request("GET", "/api/v1/billing/credit-notes");
  }

  async getCreditNote(id: string): Promise<any> {
    return this.request("GET", `/api/v1/billing/credit-notes/${id}`);
  }

  // --- Search & Offline Sync ---
  async search(query: string): Promise<any> {
    return this.request("GET", `/api/v1/search?q=${encodeURIComponent(query)}`);
  }

  async syncBatch(clientVersion: number, items: any[]): Promise<any> {
    return this.request("POST", "/api/v1/sync/batch", {
      client_version: clientVersion,
      items,
    });
  }

  // --- Procurement & Purchase Orders ---
  async createPurchaseOrder(payload: {
    supplier_id: string;
    items: Array<{
      product_id: string;
      item_name: string;
      quantity_ordered: string;
      unit_cost: string;
    }>;
    expected_delivery_date?: string;
    currency?: string;
    tax_rate?: string;
    notes?: string;
  }): Promise<any> {
    return this.request("POST", "/api/v1/procurement/purchase-orders", payload);
  }

  async listPurchaseOrders(query?: { status?: string; supplier_id?: string }): Promise<any[]> {
    const params = new URLSearchParams();
    if (query?.status) params.append("status", query.status);
    if (query?.supplier_id) params.append("supplier_id", query.supplier_id);
    const qs = params.toString() ? `?${params.toString()}` : "";
    return this.request("GET", `/api/v1/procurement/purchase-orders${qs}`);
  }

  async getPurchaseOrder(id: string): Promise<any> {
    return this.request("GET", `/api/v1/procurement/purchase-orders/${id}`);
  }

  async receivePurchaseOrder(
    id: string,
    items: Array<{
      item_id: string;
      quantity_received: string;
      batch_number?: string;
      expiry_date?: string;
    }>
  ): Promise<any> {
    return this.request("POST", `/api/v1/procurement/purchase-orders/${id}/receive`, { items });
  }

  async cancelPurchaseOrder(id: string): Promise<any> {
    return this.request("POST", `/api/v1/procurement/purchase-orders/${id}/cancel`);
  }

  // --- Promotions & Coupons ---
  async createCoupon(payload: {
    code: string;
    discount_type: "PERCENTAGE" | "FIXED_AMOUNT";
    discount_value: string;
    min_order_amount?: string;
    max_discount_amount?: string;
    usage_limit?: number;
    valid_from?: string;
    valid_until?: string;
    is_active?: boolean;
  }): Promise<any> {
    return this.request("POST", "/api/v1/promotions/coupons", payload);
  }

  async listCoupons(activeOnly?: boolean): Promise<any[]> {
    const qs = activeOnly ? "?active_only=true" : "";
    return this.request("GET", `/api/v1/promotions/coupons${qs}`);
  }

  async getCoupon(code: string): Promise<any> {
    return this.request("GET", `/api/v1/promotions/coupons/${code}`);
  }

  async updateCoupon(
    code: string,
    payload: {
      discount_type?: string;
      discount_value?: string;
      min_order_amount?: string;
      max_discount_amount?: string;
      usage_limit?: number;
      valid_from?: string;
      valid_until?: string;
      is_active?: boolean;
    }
  ): Promise<any> {
    return this.request("PATCH", `/api/v1/promotions/coupons/${code}`, payload);
  }

  async deleteCoupon(code: string): Promise<any> {
    return this.request("DELETE", `/api/v1/promotions/coupons/${code}`);
  }

  async validateCoupon(code: string, subtotal: string): Promise<any> {
    return this.request("POST", "/api/v1/promotions/coupons/validate", {
      code,
      subtotal,
    });
  }

  async redeemCoupon(code: string, subtotal: string): Promise<any> {
    return this.request("POST", `/api/v1/promotions/coupons/${code}/redeem`, {
      subtotal,
    });
  }

  // --- Multi-Currency Exchange Rates & Conversion ---
  async setExchangeRate(payload: {
    from_currency: string;
    to_currency: string;
    rate: string;
  }): Promise<any> {
    return this.request("POST", "/api/v1/billing/exchange-rates", payload);
  }

  async listExchangeRates(): Promise<any[]> {
    return this.request("GET", "/api/v1/billing/exchange-rates");
  }

  async convertCurrency(payload: {
    amount: string;
    from_currency: string;
    to_currency: string;
  }): Promise<any> {
    return this.request("POST", "/api/v1/billing/convert-currency", payload);
  }

  // --- Webhooks Management ---
  async createWebhook(payload: {
    url: string;
    event_types: string[];
    secret?: string;
  }): Promise<any> {
    return this.request("POST", "/api/v1/webhooks", payload);
  }

  async listWebhooks(): Promise<any[]> {
    return this.request("GET", "/api/v1/webhooks");
  }

  async getWebhook(webhookId: string): Promise<any> {
    return this.request("GET", `/api/v1/webhooks/${webhookId}`);
  }

  async updateWebhook(
    webhookId: string,
    payload: {
      url?: string;
      event_types?: string[];
      secret?: string;
      is_active?: boolean;
    }
  ): Promise<any> {
    return this.request("PATCH", `/api/v1/webhooks/${webhookId}`, payload);
  }

  async deleteWebhook(webhookId: string): Promise<any> {
    return this.request("DELETE", `/api/v1/webhooks/${webhookId}`);
  }

  async testWebhook(webhookId: string): Promise<any> {
    return this.request("POST", `/api/v1/webhooks/${webhookId}/test`);
  }

  // --- Bookings & Scheduling ---
  async createBooking(payload: {
    resource_id: string;
    start_time: string;
    end_time: string;
    booking_type: string;
    customer_id?: string;
    notes?: string;
  }): Promise<any> {
    return this.request("POST", "/api/v1/bookings", payload);
  }

  async listBookings(status?: string): Promise<any[]> {
    const qs = status ? `?status=${status}` : "";
    return this.request("GET", `/api/v1/bookings${qs}`);
  }

  async getBooking(bookingId: string): Promise<any> {
    return this.request("GET", `/api/v1/bookings/${bookingId}`);
  }

  async updateBookingStatus(bookingId: string, status: string): Promise<any> {
    return this.request("PATCH", `/api/v1/bookings/${bookingId}/status`, { status });
  }

  async deleteBooking(bookingId: string): Promise<any> {
    return this.request("DELETE", `/api/v1/bookings/${bookingId}`);
  }

  // --- Subscriptions & Memberships ---
  async createSubscription(payload: {
    customer_id: string;
    plan_name: string;
    amount: string;
    billing_interval?: string;
    start_date?: string;
    renewal_date?: string;
  }): Promise<any> {
    return this.request("POST", "/api/v1/subscriptions", payload);
  }

  async listSubscriptions(status?: string): Promise<any[]> {
    const qs = status ? `?status=${status}` : "";
    return this.request("GET", `/api/v1/subscriptions${qs}`);
  }

  async getSubscription(subscriptionId: string): Promise<any> {
    return this.request("GET", `/api/v1/subscriptions/${subscriptionId}`);
  }

  async updateSubscriptionStatus(subscriptionId: string, status: string): Promise<any> {
    return this.request("PATCH", `/api/v1/subscriptions/${subscriptionId}/status`, { status });
  }

  async deleteSubscription(subscriptionId: string): Promise<any> {
    return this.request("DELETE", `/api/v1/subscriptions/${subscriptionId}`);
  }

  async checkInMember(subscriptionId: string, notes?: string): Promise<any> {
    return this.request("POST", `/api/v1/subscriptions/${subscriptionId}/checkin`, { notes });
  }

  // --- Reports & Analytics ---
  async getDashboardMetrics(): Promise<any> {
    return this.request("GET", "/api/v1/reports/dashboard");
  }

  async getKitchenPrepSheet(): Promise<any> {
    return this.request("GET", "/api/v1/reports/kitchen-prep");
  }

  async getSalesByPaymentMethod(): Promise<any> {
    return this.request("GET", "/api/v1/reports/sales-by-payment-method");
  }
}
