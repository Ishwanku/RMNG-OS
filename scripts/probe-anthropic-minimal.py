#!/usr/bin/env python3
"""Minimal Anthropic probe — max 20 output tokens."""
import json
import os
import urllib.error
import urllib.request

key = os.environ.get("ANTHROPIC_API_KEY", "")
if not key:
    raise SystemExit("ANTHROPIC_API_KEY not set")

body = {
    "model": os.environ.get("ANTHROPIC_MODEL", "claude-3-5-haiku-20241022"),
    "max_tokens": 20,
    "messages": [{"role": "user", "content": "Reply with exactly: ok"}],
}
req = urllib.request.Request(
    "https://api.anthropic.com/v1/messages",
    data=json.dumps(body).encode(),
    headers={
        "x-api-key": key,
        "anthropic-version": "2023-06-01",
        "content-type": "application/json",
    },
    method="POST",
)
try:
    with urllib.request.urlopen(req, timeout=30) as resp:
        data = json.loads(resp.read().decode())
        text = data.get("content", [{}])[0].get("text", "")
        usage = data.get("usage", {})
        print(f"ok status={resp.status} reply={text!r} input={usage.get('input_tokens')} output={usage.get('output_tokens')}")
except urllib.error.HTTPError as exc:
    print(f"fail status={exc.code} body={exc.read().decode()[:300]}")
    raise SystemExit(1)