#!/usr/bin/env python3
import json
import os
import urllib.error
import urllib.request

key = os.environ.get("XAI_API_KEY", "")
if not key:
    raise SystemExit("XAI_API_KEY not set")


def call(url, data=None):
    headers = {"Authorization": f"Bearer {key}", "Content-Type": "application/json"}
    body = json.dumps(data).encode() if data else None
    method = "POST" if data else "GET"
    req = urllib.request.Request(url, data=body, headers=headers, method=method)
    try:
        with urllib.request.urlopen(req, timeout=60) as resp:
            return resp.status, resp.read().decode()[:600]
    except urllib.error.HTTPError as exc:
        return exc.code, exc.read().decode()[:600]


print("models:", call("https://api.x.ai/v1/models"))
for model in [
    "grok-4.3",
    "grok-4",
    "grok-4-latest",
    "grok-3",
    "grok-3-latest",
    "grok-2-latest",
]:
    payload = {
        "model": model,
        "messages": [{"role": "user", "content": '{"action":"plan.only","reasoning":"probe"}'}],
        "response_format": {"type": "json_object"},
        "temperature": 0,
    }
    code, body = call("https://api.x.ai/v1/chat/completions", payload)
    print(f"chat {model}: {code}")
    print(body[:300])
    print("---")