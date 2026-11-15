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
}
