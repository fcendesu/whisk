#!/usr/bin/env python3
"""Operator-side token for the loopback-only development SFU. Never ship in a client."""
import base64
import hashlib
import hmac
import json
import time
import uuid


def encode(value):
    return base64.urlsafe_b64encode(value).rstrip(b"=")


def token():
    now = int(time.time())
    header = encode(json.dumps({"alg": "HS256", "typ": "JWT"}).encode())
    claims = encode(json.dumps({
        "iss": "devkey", "sub": f"whisk-probe-{uuid.uuid4()}",
        "nbf": now - 5, "exp": now + 120,
        "video": {"roomJoin": True, "room": "whisk-m0-smoke",
                  "canPublish": False, "canSubscribe": True,
                  "canPublishData": False},
    }).encode())
    signed = header + b"." + claims
    return (signed + b"." + encode(hmac.new(b"secret", signed, hashlib.sha256).digest())).decode()


if __name__ == "__main__":
    print(token())
