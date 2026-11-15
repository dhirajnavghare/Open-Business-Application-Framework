import { ObafClient } from "../../sdk/typescript/dist/index.js";

declare const process: any;

async function run() {
  console.log("🚀 Starting OBAF TypeScript Quickstart Example...\n");

  const client = new ObafClient({ baseUrl: "http://127.0.0.1:8080" });

  // 1. Health & Discovery
  const health = await client.health();
  console.log("1. System Health:", health);

  // 2. Discover Meta Schema
  const schema = await client.schemaDiscovery();
  console.log(`2. Schema Discovered: ${schema.entities?.length ?? 21} foundational entities available.`);

  // 3. Bootstrap Organization
  const ts = Date.now();
  const tenant = await client.bootstrap({
    organization_name: `Quickstart Store ${ts}`,
    slug: `qs-store-${ts}`,
    preset: "retail_grocery",
    admin_email: `admin-${ts}@quickstart.com`,
    admin_name: "Quickstart Admin",
    admin_password: "Password123!",
  });
  console.log(`3. Tenant Bootstrapped: ${tenant.organization_name} (Preset: ${tenant.preset})`);

  // 4. Authenticate & Configure Currency
  await client.login(`admin-${ts}@quickstart.com`, "Password123!");
  await client.setExchangeRate({
    from_currency: "USD",
    to_currency: "EUR",
    rate: "0.92",
  });

  // 5. Multi-Currency Conversion
  const conversion = await client.convertCurrency({
    amount: "150.00",
    from_currency: "USD",
    to_currency: "EUR",
  });
  console.log("4. Currency Conversion (150 USD -> EUR):", conversion);

  console.log("\n✨ OBAF TypeScript SDK Client is ready to build modern frontends & microservices!");
}

run().catch((err) => {
  console.error("Quickstart Error:", err.message);
  if (typeof process !== "undefined") {
    process.exit(1);
  }
});
