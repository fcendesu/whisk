#!/usr/bin/env python3
"""Operator-side token for the loopback-only development SFU. Never ship in a client."""
import base64
import hashlib
import hmac
import json
import time
import uuid
import argparse


def encode(value):
    return base64.urlsafe_b64encode(value).rstrip(b"=")


def token(publish=False, room="whisk-m0-smoke"):
    if not room.strip():
        raise ValueError("room must not be blank")
    now = int(time.time())
    header = encode(json.dumps({"alg": "HS256", "typ": "JWT"}).encode())
    claims = encode(json.dumps({
        "iss": "devkey", "sub": f"whisk-probe-{uuid.uuid4()}",
        "nbf": now - 5, "exp": now + 120,
        "video": {"roomJoin": True, "room": room,
                  "canPublish": publish, "canSubscribe": True,
                  "canPublishData": False},
    }).encode())
    signed = header + b"." + claims
    return (signed + b"." + encode(hmac.new(b"secret", signed, hashlib.sha256).digest())).decode()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--publisher", action="store_true", help="allow media publishing in the chosen local fixture room")
    parser.add_argument("--room", default="whisk-m0-smoke", help="operator-selected fixture room")
    args = parser.parse_args()
    print(token(publish=args.publisher, room=args.room))
