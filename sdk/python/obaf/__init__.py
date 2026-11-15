"""
Open Business Application Framework (OBAF) - Python SDK
High-performance headless business operations and billing client.
"""

from .client import ObafClient, ObafError, ObafAuthError

__version__ = "0.1.0"
__all__ = ["ObafClient", "ObafError", "ObafAuthError"]
