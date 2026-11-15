import test from "node:test";
import assert from "node:assert/strict";
import { ObafClient, ObafError } from "../dist/index.js";

test("ObafClient - Initialization and URL Normalization", () => {
  const client1 = new ObafClient({ baseUrl: "http://127.0.0.1:8080/" });
  assert.equal(client1.baseUrl, "http://127.0.0.1:8080");

  const client2 = new ObafClient({ baseUrl: "http://127.0.0.1:8080", token: "jwt-token-123" });
  assert.equal(client2.token, "jwt-token-123");
  assert.equal(client2.timeoutMs, 30000);

  const client3 = new ObafClient({ baseUrl: "http://localhost:9000", timeoutMs: 5000 });
  assert.equal(client3.timeoutMs, 5000);
});

test("ObafClient - Verification of all Domain Methods", () => {
  const client = new ObafClient({ baseUrl: "http://127.0.0.1:8080" });

  const requiredMethods = [
    // System & Auth
    "health", "engineInfo", "schemaDiscovery", "bootstrap", "login", "me",
    // Catalog
    "createProduct", "listProducts", "getProduct", "updateProduct", "deleteProduct",
    // Customers
    "createCustomer", "listCustomers", "getCustomer", "updateCustomer", "deleteCustomer",
    // Suppliers
    "createSupplier", "listSuppliers", "getSupplier", "updateSupplier", "deleteSupplier",
    // Orders & Billing
    "createOrder", "listOrders", "recordPayment", "dailySettlement",
    // Inventory Locations
    "createStockLocation", "listStockLocations", "getStockLocation", "updateStockLocation", "deleteStockLocation",
    // POS & Shifts
    "openShift", "getCurrentShift", "closeShift", "listShifts", "issueCreditNote", "listCreditNotes", "getCreditNote",
    // Procurement
    "createPurchaseOrder", "listPurchaseOrders", "getPurchaseOrder", "receivePurchaseOrder", "cancelPurchaseOrder",
    // Promotions
    "createCoupon", "listCoupons", "getCoupon", "updateCoupon", "deleteCoupon", "validateCoupon", "redeemCoupon",
    // Currency
    "setExchangeRate", "listExchangeRates", "convertCurrency",
    // Webhooks
    "createWebhook", "listWebhooks", "getWebhook", "updateWebhook", "deleteWebhook", "testWebhook",
    // Bookings & Subscriptions
    "createBooking", "listBookings", "getBooking", "updateBookingStatus", "deleteBooking",
    "createSubscription", "listSubscriptions", "getSubscription", "updateSubscriptionStatus", "deleteSubscription",
    "checkInMember",
    // Reports & Search
    "getDashboardMetrics", "getKitchenPrepSheet", "getSalesByPaymentMethod", "search", "syncBatch"
  ];

  for (const method of requiredMethods) {
    assert.equal(
      typeof client[method],
      "function",
      `Expected client to implement method '${method}'`
    );
  }
});

test("ObafClient - Mock Request Dispatch, Query Params, and Headers", async () => {
  const originalFetch = globalThis.fetch;
  let capturedUrl = "";
  let capturedOptions = null;

  try {
    globalThis.fetch = async (url, options) => {
      capturedUrl = url.toString();
      capturedOptions = options;
      return {
        ok: true,
        status: 200,
        headers: new Headers({ "content-type": "application/json" }),
        json: async () => ({ status: "success", data: [{ id: "prod-1", name: "Paracetamol" }] })
      };
    };

    const client = new ObafClient({ baseUrl: "http://127.0.0.1:8080", token: "secret-bearer-jwt" });

    const result = await client.search("Paracetamol");

    assert.equal(capturedUrl, "http://127.0.0.1:8080/api/v1/search?q=Paracetamol");
    assert.equal(capturedOptions.method, "GET");
    assert.equal(capturedOptions.headers["Authorization"], "Bearer secret-bearer-jwt");
    assert.equal(capturedOptions.headers["Content-Type"], "application/json");
    assert.equal(result.status, "success");
    assert.equal(result.data.length, 1);
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test("ObafClient - Mock Post Request Body Serialization", async () => {
  const originalFetch = globalThis.fetch;
  let capturedBody = null;

  try {
    globalThis.fetch = async (url, options) => {
      capturedBody = JSON.parse(options.body);
      return {
        ok: true,
        status: 201,
        headers: new Headers({ "content-type": "application/json" }),
        json: async () => ({ id: "cust-1", name: "Acme Corp" })
      };
    };

    const client = new ObafClient({ baseUrl: "http://127.0.0.1:8080" });
    const payload = { name: "Acme Corp", email: "acme@example.com", phone: "+1234567890" };
    const res = await client.createCustomer(payload);

    assert.deepEqual(capturedBody, payload);
    assert.equal(res.id, "cust-1");
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test("ObafClient - Error Response Handling and ObafError Propagation", async () => {
  const originalFetch = globalThis.fetch;

  try {
    globalThis.fetch = async () => {
      return {
        ok: false,
        status: 404,
        headers: new Headers({ "content-type": "application/json" }),
        json: async () => ({ error: "Product not found" }),
        text: async () => JSON.stringify({ error: "Product not found" })
      };
    };

    const client = new ObafClient({ baseUrl: "http://127.0.0.1:8080" });

    await assert.rejects(
      async () => {
        await client.getProduct("non-existent-id");
      },
      (err) => {
        assert.ok(err instanceof ObafError);
        assert.equal(err.statusCode, 404);
        assert.ok(err.message.includes("Product not found"));
        return true;
      }
    );
  } finally {
    globalThis.fetch = originalFetch;
  }
});
