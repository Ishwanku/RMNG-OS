# Integration Strategy — RMNG-OS

**Status:** Active · **Owner:** Principal Systems Architect  
**Aligns with:** ADR-009 (Rust runtime), ADR-010 (Nervous/Body), ADR-014 (native-first), ADR-015 (CoreIntent v2)

This document governs how **external open-source repositories** (MCP servers, agent frameworks, skill packs, orchestrators) may enter RMNG-OS **without breaking architectural sovereignty**.

---

## 1. Principle

> The LLM reasons. Rust executes. Nothing external runs commands directly.

Every integration must declare which **track** it uses. Mixing tracks without documentation is a governance violation.

---

## 2. Four Integration Tracks

| Track | Name | What enters RMNG-OS | Execution path | Example |
|-------|------|-------------------|----------------|---------|
| **1** | **Native Core** | Rust tool impl in `integrations/` + manifest | `tool.execute` → `PermissionGate` → native handler | `kernel.status`, `git.status` |
| **2** | **MCP Proxy Plane** | Allowlisted MCP server subprocess | `mcp.proxy` → `PermissionGate` → `rmng-mcp` → JSON-RPC child | `git.log` via `mcp-server-git` |
| **3** | **Nervous Context (Skills)** | Markdown skill in `skills/` | LLM reads skill; emits intents only — **no execution** | `skills/kernel-build/SKILL.md` |
| **4** | **Rejected / Deferred** | Nothing imported | N/A — recorded in `docs/integrations/` with rationale | Full LangChain runtime in-process |

### Track selection rules

| If the repo… | Track |
|--------------|-------|
| Is a small, auditable tool with stable I/O | **1 — Native Core** (preferred for production) |
| Is an MCP server with many tools, used sparingly | **2 — MCP Proxy** (allowlist + explicit tools) |
| Is documentation / workflow / prompt patterns | **3 — Skills** |
| Wants shell access, arbitrary code exec, or in-process Python agent loop | **4 — Rejected** (or defer until sandbox exists) |

---

## 3. Evaluation Framework

Score each candidate **1–5** per dimension. Minimum **3.5 average** and **no score below 3** on security to proceed.

### 3.1 Execution Plane Isolation

- Does it require running inside `rmngd` process? (**reject**)
- Can it be subprocess, allowlisted, or skill-only?
- Does it respect CoreIntent v2 (`tool.execute` | `mcp.proxy` | `plan.only`)?

### 3.2 Structural Determinism

- Are inputs/outputs schema-stable (JSON, TOML, typed CLI)?
- Can `PermissionGate` enumerate all capabilities upfront?
- Avoid integrations that discover tools dynamically without allowlist sync.

### 3.3 Zero-Trust Security

- No broad filesystem / network access by default
- No credential storage in repo — use `~/.rmng/` or OS keyring
- MCP: **explicit** `allowed_tools` per server (see `mcp-allowlist.toml`)
- Audit trail: every dispatch → `~/.rmng/logs/audit.jsonl`

### 3.4 Architectural Fit (ADR-010)

- Nervous system may **read** and **plan**
- Body may **execute** only after gate approval
- Integration must not blur IDE dev MCP (`~/.config/rmng/mcp-dev.json`) with production proxy (`~/.rmng/mcp-allowlist.toml`)

---

## 4. Governance Process

### Step 1 — Intake

Create `docs/integrations/<name>.md` from [integrations/TEMPLATE.md](integrations/TEMPLATE.md).

Record:
- Repository URL + license
- Proposed track (1–4)
- Evaluation scores
- Threat notes (prompt injection, path traversal, network egress)

### Step 2 — Review

Maintainer checks:
- [ ] ADR-009/010/014/015 compliance
- [ ] No vendored mega-deps into `agents/`
- [ ] Clear rollback (disable server / remove skill)

### Step 3 — Implement (by track)

| Track | Actions |
|-------|---------|
| **1** | Add `integrations/<domain>/<tool>.json`, Rust handler, schema, `PermissionGate` entry, tests |
| **2** | `./scripts/register-mcp-tool.sh …`, doc, restart `rmngd`, add example intent in `agents/schemas/` |
| **3** | Add `skills/<name>/SKILL.md`, index in `skills/INDEX.md` |
| **4** | Mark **Deferred** or **Rejected** in doc; no code changes |

### Step 4 — Verify

```bash
rmng status
rmng tools
# MCP track:
rmng send -f agents/schemas/mcp-git-log.intent.json
tail ~/.rmng/logs/audit.jsonl
```

### Step 5 — Operate

- Snapshot allowlist before changes: `~/.rmng/allowlists/*.bak` (register script does this)
- Review audit log weekly during active integration sprints

---

## 5. Directory Map

```
RMNG-OS/
├── integrations/          # Track 1 — native tool manifests + handlers
├── skills/                # Track 3 — nervous-system context
├── agents/rmng-mcp/       # Track 2 — MCP proxy implementation
├── ~/.rmng/
│   ├── mcp-allowlist.toml # Track 2 — production allowlist
│   └── logs/audit.jsonl   # All tracks — audit
├── ~/.config/rmng/
│   └── mcp-dev.json       # IDE-only MCP (NOT production)
└── docs/integrations/     # Per-repo evaluation docs
```

---

## 6. Anti-Patterns (Do Not)

| Anti-pattern | Why |
|--------------|-----|
| Vendoring awesome-mcp-servers lists | Unbounded attack surface |
| Letting IDE MCP configs drive `rmngd` | Breaks body/nervous separation |
| Python agent runtime inside `rmngd` | Violates ADR-009 |
| `allowed_tools = ["*"]` | Violates zero-trust |
| Skipping `docs/integrations/` doc | No rollback / no audit trail for humans |

---

## 7. When You Have a Repo List

1. Run intake template for each repo (parallel OK)
2. Assign track — default to **3** or **4** when unsure
3. Implement **one** Track-2 MCP at a time; prefer **Track-1** for hot-path tools
4. Never batch-allowlist tools without per-tool review

See also: [PLAN-AGENTS-MCP-SKILLS.md](PLAN-AGENTS-MCP-SKILLS.md) · [DECISIONS.md](DECISIONS.md) · [daily-workflow.md](daily-workflow.md)