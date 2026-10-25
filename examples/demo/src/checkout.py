"""
Tokenized checkout authentication module.

# @trace SPEC-DEMO-AUTH-SVC, REQ-DEMO-AUTH
"""

import hmac
import hashlib
import time
from typing import Dict, Any


# @trace SPEC-DEMO-AUTH-SVC, REQ-DEMO-AUTH
def validate_token(token: str, secret_key: bytes, max_age_seconds: int = 300) -> Dict[str, Any]:
    """Validates an ephemeral checkout authentication token.
    
    Verifies HMAC-SHA256 signature and checks timestamp within allowed TTL window.
    """
    parts = token.split(".")
    if len(parts) != 3:
        raise ValueError("Malformed payment token structure")

    customer_id, timestamp_str, signature = parts
    try:
        timestamp = int(timestamp_str)
    except ValueError:
        raise ValueError("Invalid timestamp in payment token")

    now = int(time.time())
    if now - timestamp > max_age_seconds:
        raise ValueError(f"Token expired: age {now - timestamp}s exceeds TTL {max_age_seconds}s")

    payload = f"{customer_id}.{timestamp_str}".encode("utf-8")
    expected_sig = hmac.new(secret_key, payload, hashlib.sha256).hexdigest()

    if not hmac.compare_digest(signature, expected_sig):
        raise ValueError("Cryptographic token signature verification failed")

    return {
        "customer_id": customer_id,
        "timestamp": timestamp,
        "authenticated": True,
    }
