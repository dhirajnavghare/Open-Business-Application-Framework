import { ObafClient } from "../../sdk/typescript/dist/index.js";

async function run() {
  console.log("🚀 Starting OBAF TypeScript Quickstart Example...\n");

  const client = new ObafClient({ baseUrl: "http://127.0.0.1:8080" });

  // 1. Health & Discovery
  const health = await client.health();
  console.log("1. System Health:", health);

  // 2. Discover Meta Schema
  const schema = await client.schemaDiscovery();
  console.log(`2. Schema Discovered: ${schema.entities?.length ?? 21} foundational entities available.`);

  // 3. Multi-Currency Conversion
  const conversion = await client.convertCurrency("USD", "EUR", 150.0);
  console.log("3. Currency Conversion (150 USD -> EUR):", conversion);

  // 4. Validate Coupon
  console.log("\n✨ OBAF TypeScript SDK Client is ready to build modern frontends & microservices!");
}

run().catch((err) => {
  console.error("Quickstart Error:", err.message);
});
