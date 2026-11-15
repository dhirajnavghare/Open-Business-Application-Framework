import sys
from pathlib import Path

# Add sdk/python to path
sys.path.insert(0, str(Path(__file__).parent.parent / "sdk" / "python"))

from obaf import ObafClient, ObafError

def test_sdk_instantiation_and_offline():
    client = ObafClient(base_url="http://127.0.0.1:8080")
    assert client.base_url == "http://127.0.0.1:8080"
    assert client.token is None
    # Check new methods exist
    assert hasattr(client, "create_purchase_order")
    assert hasattr(client, "receive_purchase_order")
    assert hasattr(client, "create_coupon")
    assert hasattr(client, "validate_coupon")
    assert hasattr(client, "redeem_coupon")
    assert hasattr(client, "set_exchange_rate")
    assert hasattr(client, "convert_currency")
    print("✅ Python SDK loaded and instantiated successfully!")

if __name__ == "__main__":
    test_sdk_instantiation_and_offline()
