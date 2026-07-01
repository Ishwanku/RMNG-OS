#!/usr/bin/env python3
"""Merge MCP server entries into ~/.rmng/mcp-allowlist.toml (stdlib only)."""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

try:
    import tomllib
except ImportError:
    import tomli as tomllib  # type: ignore


def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser(description="Add or update MCP server in allowlist")
    p.add_argument("--file", required=True, help="Path to mcp-allowlist.toml")
    p.add_argument("--server", required=True, help="Server key (e.g. github, git)")
    p.add_argument("--command", required=True, help="Executable (npx, uvx, ...)")
    p.add_argument("--args", nargs="+", default=[], help="Command arguments")
    p.add_argument("--tools", nargs="+", required=True, help="Allowed tool IDs")
    p.add_argument("--disable", action="store_true", help="Set enabled = false")
    return p.parse_args()


def load_servers(path: Path) -> dict:
    if not path.exists():
        return {}
    raw = path.read_text(encoding="utf-8")
    data = tomllib.loads(raw) if raw.strip() else {}
    return data.get("servers", {})


def format_toml(servers: dict) -> str:
    lines = [
        "# RMNG MCP production allowlist — rmngd proxy plane only",
        "# Managed by: scripts/register-mcp-tool.sh",
        "# ADR-014: explicit allowlist required before any MCP proxy executes",
        "# Never commit tokens or machine-specific secrets to the repo.",
        "",
    ]
    for name in sorted(servers.keys()):
        cfg = servers[name]
        lines.append(f"[servers.{name}]")
        lines.append(f"enabled = {'true' if cfg.get('enabled', True) else 'false'}")
        cmd = cfg["command"].replace("\\", "\\\\").replace('"', '\\"')
        lines.append(f'command = "{cmd}"')
        args = ", ".join(f'"{a}"' for a in cfg.get("args", []))
        lines.append(f"args = [{args}]")
        tools = ", ".join(f'"{t}"' for t in cfg.get("allowed_tools", []))
        lines.append(f"allowed_tools = [{tools}]")
        lines.append("")
    return "\n".join(lines)


def main() -> int:
    args = parse_args()
    path = Path(args.file).expanduser()
    path.parent.mkdir(parents=True, exist_ok=True)

    servers = load_servers(path)
    servers[args.server] = {
        "enabled": not args.disable,
        "command": args.command,
        "args": args.args,
        "allowed_tools": args.tools,
    }

    path.write_text(format_toml(servers), encoding="utf-8")
    print(f"Updated {path} — servers.{args.server} ({len(args.tools)} tools)")
    return 0


if __name__ == "__main__":
    sys.exit(main())