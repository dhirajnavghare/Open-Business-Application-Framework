import sys
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
    print(f"2. Schema Discovery: {len(schema.get('entities', []))} entities introspection complete.")

    # 3. Currency Conversion
    conversion = client.convert_currency("USD", "EUR", 250.0)
    print("3. Currency Conversion (250 USD -> EUR):", conversion)

    print("\n✨ OBAF Python SDK Client is ready for AI agents, automations, and data pipelines!")

if __name__ == "__main__":
    main()
