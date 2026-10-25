"""
Automated unit and security tests for tokenized checkout.

# @trace SPEC-DEMO-AUTH-SVC, REQ-DEMO-AUTH
"""

import hmac
import hashlib
import time
import pytest
from src.checkout import validate_token


# @trace SPEC-DEMO-AUTH-SVC, REQ-DEMO-AUTH
def test_valid_checkout_token():
    secret = b"super-secret-key-12345"
    now = int(time.time())
    payload = f"cust_9876.{now}".encode("utf-8")
    sig = hmac.new(secret, payload, hashlib.sha256).hexdigest()
    token = f"cust_9876.{now}.{sig}"

    res = validate_token(token, secret)
    assert res["authenticated"] is True
    assert res["customer_id"] == "cust_9876"


# @trace SPEC-DEMO-AUTH-SVC, REQ-DEMO-AUTH
def test_tampered_token_rejected():
    secret = b"super-secret-key-12345"
    now = int(time.time())
    token = f"cust_9876.{now}.bad_signature"

    with pytest.raises(ValueError, match="Cryptographic token signature"):
        validate_token(token, secret)
