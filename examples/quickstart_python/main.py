import sys
import time
from pathlib import Path

# Add sdk/python to path
sys.path.insert(0, str(Path(__file__).parent.parent.parent / "sdk" / "python"))

from obaf import ObafClient

def main():
    print("🚀 Starting OBAF Python Quickstart Example...\n")

    client = ObafClient(base_url="http://127.0.0.1:8080")

    # 1. Health & Discovery
    health = client.health()
    print("1. System Health:", health)

    # 2. Schema Discovery
    schema = client.schema_discovery()
    print(f"2. Schema Discovery: {len(schema.get('entities', []))} entities introspected.")

    # 3. Bootstrap Organization
    ts = int(time.time())
    tenant = client.bootstrap(
        organization_name=f"Quickstart Enterprise {ts}",
        slug=f"qs-enterprise-{ts}",
        preset="retail_grocery",
        admin_email=f"admin-{ts}@quickstart.com",
        admin_name="Quickstart Admin",
        password="Password123!",
    )
    print(f"3. Tenant Bootstrapped: {tenant.get('organization_name')} (Preset: {tenant.get('preset')})")

    # 4. Authenticate & Configure Currency Exchange
    client.login(f"admin-{ts}@quickstart.com", "Password123!")
    client.set_exchange_rate(from_currency="USD", to_currency="EUR", rate="0.92")

    # 5. Currency Conversion
    conversion = client.convert_currency(amount="250.00", from_currency="USD", to_currency="EUR")
    print("4. Currency Conversion (250 USD -> EUR):", conversion)

    print("\n✨ OBAF Python SDK Client is ready for AI agents, automations, and data pipelines!")

if __name__ == "__main__":
    main()
