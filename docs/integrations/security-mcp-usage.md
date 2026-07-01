# MCP Security & Isolation (Sprint 21)

Hardening for Track 2 MCP subprocesses: seccomp profiles, capability dropping, and per-server isolation.

## Risk levels

| Server | Risk | Why | Recommended seccomp |
|--------|------|-----|------------------------|
| `git`, `github`, `fetch`, `markitdown` | **Low** | Read-only, no browser/sandbox | Off or `basic` |
| `mem0` | **Medium** | External API; memory writes | `basic` + `no_new_privs` |
| `playwright` | **High** | Full browser, web egress, DOM injection | `playwright` + caps drop |
| `e2b` | **High** | Remote code execution API (cloud sandbox) | `e2b` + caps drop |

High-risk servers are **opt-in** (`enabled = false` in examples). Enable only with explicit isolation blocks.

## Seccomp profiles

Profiles are **blocklists** (default allow, deny dangerous syscalls). Configurable per server — not always-on globally.

| Profile | Blocks | Compatibility |
|---------|--------|-----------------|
| `basic` | mount, module load, bpf, namespaces, keyring, time skew, … | Most Node MCP servers |
| `e2b` | Same as `basic` | Thin `npx @e2b/mcp-server` client |
| `playwright` | Smaller blocklist (allows namespaces/ptrace for Chromium) | `@playwright/mcp` + browser children |

Set in `~/.rmng/mcp-allowlist.toml`:

```toml
[servers.playwright.isolation]
seccomp_profile = "playwright"
drop_capabilities = true
no_new_privs = true

[servers.e2b.isolation]
seccomp_profile = "e2b"
drop_capabilities = true
```

Values: `off` / omit = no seccomp. Requires **Linux or WSL2**.

## Capability dropping

`drop_capabilities = true` clears all capabilities in `pre_exec` after `no_new_privs`. Logged in audit for high-risk MCP calls:

```
seccomp=Some("playwright") applied=true caps_dropped=true risk=high
```

## Security vs compatibility

| Setting | Safer | More compatible |
|---------|-------|-----------------|
| Global `seccomp_profile = "basic"` | Blocks kernel abuse | May break unusual MCP servers |
| Per-server seccomp | Targeted hardening | Recommended default |
| `drop_capabilities` on low-risk | Minimal cap exposure | Rare edge cases with setuid helpers |
| Playwright `pids_max = 128` | Limits fork bombs | Lower if OOM on small hosts |

**Backward compatible:** omitting `seccomp_profile` and `drop_capabilities` leaves Sprint 10 behavior unchanged.

## Audit & observe

```bash
rmng observe                    # isolation line shows seccomp + cap_drop defaults
tail ~/.rmng/logs/audit.jsonl   # MCP entries include seccomp/caps for high-risk
```

## References

- [operations-usage.md](operations-usage.md)
- [playwright-mcp.md](playwright-mcp.md)
- [e2b-mcp.md](e2b-mcp.md)
- [mcp-allowlist.toml.example](../../config/mcp-allowlist.toml.example)
