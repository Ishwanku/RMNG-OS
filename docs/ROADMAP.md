# RMNG-OS Roadmap

Development phases for the RMNG-OS kernel lab environment.

## Phase 1 — Environment & First Build ✅ COMPLETE

| Task | Status |
|------|--------|
| WSL2 Ubuntu 24.04 setup | ✅ |
| Build toolchain + ccache | ✅ |
| WSL tuning (`wsl.conf`, `.wslconfig`) | ✅ |
| Home directory structure | ✅ |
| Kernel source clone (shallow) | ✅ |
| Out-of-tree first full build | ✅ (`vmlinux` ~458 MB) |
| VS Code + WSL integration | ✅ |
| GitHub repo `RMNG-OS` published | ✅ |

**Outcome:** Reproducible WSL kernel development environment with documented tooling.

---

## Phase 2 — Active Development Workflow ✅ COMPLETE

Goal: Turn the environment into a daily-use kernel lab with faster iteration and slimmer builds.

### 2.1 Workspace Automation

| Task | Priority | Notes |
|------|----------|-------|
| `workspace-setup.sh` — wire symlinks | High | ✅ in repo |
| `status.sh` — one-command health check | High | ✅ in repo |
| `build.sh` — standardized make wrapper | High | ✅ in repo |
| Install `gh` in WSL for git push | Medium | Avoid hung pushes |
| Git credential helper via `gh auth login` | Medium | One-time in WSL |

### 2.2 Config Optimization

| Task | Priority | Notes |
|------|----------|-------|
| Generate slim config with `localmodconfig` | High | ✅ 8821 → 5498 lines, 19 modules |
| Document config diff vs full WSL config | Medium | ✅ `config/wsl-kernel.config.slim.example` |
| `make menuconfig` walkthrough | Medium | Document in `docs/config-guide.md` |

### 2.3 Rebuild Performance

| Task | Priority | Notes |
|------|----------|-------|
| Incremental rebuild benchmark | High | ✅ 176.39 s (see benchmarks/) |
| Single module build | High | ✅ `tun.ko` via `M=drivers/net` |
| Tune `JOBS` / ccache size | Low | Match 6 CPUs, 12 GB RAM |
| `make clean` vs full rebuild docs | Low | Disk management |

### 2.4 Kernel Experimentation

| Task | Priority | Notes |
|------|----------|-------|
| Build a single module (`make M=...`) | High | e.g. `drivers/char` |
| Apply a trivial patch (LOCALVERSION, printk) | High | Learn patch workflow |
| Kernel change journal in `docs/experiments/` | Medium | Log what you tried |
| Requirements & architecture docs | High | ✅ REQUIREMENTS.md, ARCHITECTURE.md, DECISIONS.md |

### 2.5 Repository & Docs

| Task | Priority | Notes |
|------|----------|-------|
| Update README with phase status | High | ✅ this session |
| Add `docs/ROADMAP.md` | High | ✅ this file |
| Add `docs/daily-workflow.md` | Medium | Common commands |
| GitHub topics + repo description polish | Low | |

---

## Phase 3 — Customization & RMNG Identity ✅ COMPLETE

Goal: Make the kernel build distinctly "RMNG" without forking the entire tree.

| Task | Status | Notes |
|------|--------|-------|
| Custom `CONFIG_LOCALVERSION="-rmng"` | ✅ | Set in `$KBUILD/.config` |
| RMNG boot banner patch | ✅ | `patches/0001-rmng-boot-banner.patch` |
| Track patches under `patches/` | ✅ | `series` + README |
| Scripted patch apply + rebuild | ✅ | `apply-patches.sh`, `rebuild-with-patches.sh` |
| Validation report | ✅ | `docs/experiments/phase3-validation-20260630.md` |

**Outcome:** Patched kernel builds with RMNG identity banner in `vmlinux`; rebuild ~252 s with warm ccache.

---

## Phase 4 — Advanced Kernel + Bare-Metal Boot (ADR-012)

| Task | Notes |
|------|-------|
| Boot custom kernel in WSL2 | Requires Microsoft WSL kernel build docs |
| Bare-metal boot timeline | **Locked: Phase 4** — initramfs, hardware drivers |
| eBPF / BTF experiments | Tools already built (pahole, dwarves) |
| GitHub Actions | Lint scripts only — no kernel CI (too heavy) |
| Cross-compile or module-only CI | Lightweight automation |

---

## Phase 5 — AI Agent Foundation ✅ COMPLETE

Goal: Implement Rust runtime with nervous-system / body separation. See [VISION.md](VISION.md), [ADR-009–012](DECISIONS.md).

**Locked:** Rust runtime · Hybrid local-first LLM · CLI-first · JSON intent boundary

| Task | Notes |
|------|-------|
| `agents/` runtime scaffold | 🔄 Rust workspace (`rmng-core`, `rmng-cli`, `rmngd`) |
| `integrations/` adapter layout | 🔄 `integrations/dev/kernel.json` |
| Ollama nervous-system adapter | 🔄 `rmng-nervous` + `rmng ask` |
| External API adapter (pluggable) | OpenAI/Anthropic — intents only |
| `rmng` CLI + `rmngd` daemon | ✅ `run`, `send`, IPC socket |
| Permission gate + audit log | ✅ Gate + `~/.rmng/logs/audit.jsonl` |
| `gh auth login` in WSL | Git push from Ubuntu |

## Phase 6 — Skills, MCP & Integrations ✅ ACTIVE
### Agent integration sprints (Sprints 17–22) ✅

| Sprint | Deliverable |
|--------|-------------|
| 17 | E2B `run_code` MCP (opt-in) |
| 18 | Testing skills + workflow E2E |
| 19 | Persistent circuit breaker, budget governance |
| 20 | Per-process MCP resource metrics |
| 21 | Seccomp profiles + cap drop for high-risk MCP |
| 22 | Docs polish, test coverage, observe schema v1 |

Docs: `docs/integrations/end-to-end-workflow.md`, `recommended-agent-setups.md`



**Plan:** [PLAN-AGENTS-MCP-SKILLS.md](PLAN-AGENTS-MCP-SKILLS.md) · **ADR:** [ADR-014](DECISIONS.md)

| Sub-phase | Goal |
|-----------|------|
| **6a** | Skills (`skills/`) + dev MCP template + `setup-dev-mcp.sh` |
| **6b** | `rmng-mcp` bridge (rust-sdk) + allowlist |
| **6c** | `rmng ask --skill` nervous-system integration |

## Phase 6 — Workflow Integrations

| Domain | Priority |
|--------|----------|
| Development (git, build, kernel) | High — natural extension of today |
| Data & files | Medium |
| Cloud & infra | Medium |
| Creative & business | Later |

## Phase 7 — Agent Orchestration (IN PROGRESS)

Multi-agent routing, layer model, session store, swarm handoffs.

| Sprint | Status | Deliverables |
|--------|--------|--------------|
| **Sprint 1** | ✅ | IntegrationRegistry, IntentValidator, Audit v2, MCP lifecycle |
| **Sprint 2** | ✅ | Agent definitions, router, `rmng ask --agent`, `rmng observe`, progressive skills |
| **Sprint 3** | ✅ | L1–L4 layer model, session store, layer-aware router, ADR-017 |
| **Sprint 4a** | ✅ | Shared context in prompts, `rmng handoff`, daemon E2E tests, light ingestion |
| **Sprint 4b** | ✅ | Tool result write-back to `shared_context`, multi-hop `--chain` handoffs, `session prune`, collaboration E2E |
| **Sprint 4c** | ✅ | Live LLM session orchestration prompts, Ollama guidance, MCP `search_issues` E2E, session TTL on load |
| **Sprint 5** | ✅ | Pluggable LLM providers (Ollama, Grok, OpenAI, Anthropic, Google, OpenAI-compat family) |
| **Sprint 6** | ✅ | Autonomous handoff (`metadata.handoff_to`), JSON auto-retry, provider matrix, production hardening |
| **Sprint 7** | ✅ | Per-agent LLM (`llm_profile` / `model` in agent YAML), live model discovery (`rmng llm models --live`), multi-hop `handoff_chain`, expanded matrix + error classification, generation params |
| **Sprint 8** | ✅ | Provider fallback chains (`llm_fallback`), handoff pre-validation, per-session LLM observability in `rmng observe`, `rmng llm sync-catalog`, expanded matrix providers |
| **Sprint 9** | ✅ | Token/cost telemetry, circuit breaker + exponential backoff, fallback E2E tests, `sync-catalog --apply`, session-less fallback + audit telemetry |
| **Sprint 10** | ✅ | Tamper-evident audit (hash chain v3), MCP subprocess isolation (cgroup/rlimit), deeper `rmng observe`, schema version enforcement, ADR-020 |
| **Sprint 11** | ✅ | Editable catalog pricing, `rmng observe --cost` rollups, persistent circuit breaker, budget warn/deny, `rmng audit verify`, ADR-021 |
| **Sprint 12** | ✅ | Controlled external integration — MCP fetch/playwright, skills batch, integration roadmap, per-agent budgets |
| **Sprint 13** | ✅ | Web & research capability — fetch E2E, markitdown MCP, web-researcher skills, usage docs |
| **Sprint 14** | ✅ | Browser + code workflow — Playwright opt-in E2E, GitHub/Git MCP expand, markitdown live E2E |
| **Sprint 15** | ✅ | Memory & long-term context — Mem0 MCP, memory-management skill, agent scopes |
| **Sprint 16** | ✅ | Evaluation & self-improvement — critique/validation/loop skills, promptfoo patterns |
| **Sprint 17** | ✅ | Safe code execution — E2B sandbox MCP, code-execution skill, agent scopes |
| **Sprint 18** | ✅ | Testing workflows — run-tests, validate-output, coverage, regression skills |
| **Sprint 19** | ✅ | Production hardening — persistent circuits, observability, audit CI, profile budgets |

See [ADR-017](decisions/ADR-017-multi-level-agent-architecture.md) · [ADR-020](decisions/ADR-020-linux-aligned-runtime-hardening.md) · [ADR-021](decisions/ADR-021-cost-governance.md) · [INTEGRATION-ROADMAP.md](INTEGRATION-ROADMAP.md).

---

## Immediate Next Actions (Start Here)

```bash
# 1. Wire workspace
cd ~/dev/projects/RMNG-OS
./scripts/workspace-setup.sh

# 2. Check status
~/scripts/rmng-status.sh

# 3. Slim config (done — or re-run)
~/dev/projects/RMNG-OS/scripts/slim-config.sh

# 4. Rebuild with slim config and measure ccache
~/scripts/rmng-build.sh

# 5. Open project in VS Code
code ~/dev/projects/RMNG-OS
```

---

## Success Criteria by Phase

| Phase | Done when |
|-------|-----------|
| **1** | `vmlinux` exists, repo on GitHub | ✅ |
| **2** | Slim config builds, ccache rebuild < 5 min, daily scripts work |
| **3** | Custom patch applies cleanly and rebuilds | ✅ |
| **4** | Optional advanced goal achieved |
### Sprint 21 — Security Hardening & Subprocess Isolation ✅
- Seccomp profiles (`basic`, `playwright`, `e2b`) per MCP server
- Capability dropping in pre_exec; audit logging for high-risk tools
- Tighter config examples; [security-mcp-usage.md](integrations/security-mcp-usage.md)
### Sprint 20 — Resource Metrics & Observability ✅
- Per-subprocess peak RSS + CPU via `wait4` in `rmng-mcp`
- `resource_rollup` in `rmng observe` (text + JSON)
- Audit/session fields: `mcp_peak_rss_kb`, `mcp_cpu_time_ms`

### Sprint 22 — Consolidation & Polish ✅
- [end-to-end-workflow.md](integrations/end-to-end-workflow.md), [recommended-agent-setups.md](integrations/recommended-agent-setups.md)
- Integration tests: resource metrics (real subprocess), security isolation E2E, Mem0 deny gate
- `rmng observe --json` schema v1 (`schema_version`, `generated_at`, `resource_rollup`)
- Integrations index cleanup
### Sprint 23 — Multi-hop Orchestration & Autonomous Behavior ✅
- `handoff_chain` + `handoff_return_to` in `plan.only` metadata; router executes full chains
- Upward return to L4 orchestrator (feedback loop) with session tool-result summary
- `shared_context.orchestration` tracks chain progress; audit events per hop
- E2E: autonomous chain, return-to-orchestrator, chain state in prompt context
### Sprint 24 — Live LLM orchestration hardening ✅
- Stronger chain/return prompt guidance + orchestration_prompt module
- Robust parse_core_intent (JSON extraction, comma-chain normalize)
- `rmng ask --auto-continue` partial dispatch loop
- Chain hop failure recording + audit
- live_llm_chain_e2e (Groq/Grok); orchestration-usage.md
### Sprint 25 — Auto-continue foundation ✅
- `AutoContinueLoop`, chain continuation session state, CLI `--auto-continue`
- Chain error recovery, hop failure policies, parser hardening
### Sprint 26 — Daemon auto-continue ✅
- `DaemonOrchestrator`, `orchestration.continue` IPC, background post-dispatch continue
### Sprint 27 — Production-safe auto-continue ✅
- Per-session continuation mutex, timeout finalization, socket E2E tests
### Sprint 28 — Production readiness & operational polish ✅
- `ReadinessReport`, `rmngd --validate`, `rmng health`, improved `observe --json`
- systemd `ExecStartPre`, install validation, ops documentation
### Sprint 29 — Health consistency & hardening ✅
- `rmng health --require-daemon` and `--strict` exit semantics
- Health JSON schema v2 with `failures` array
- Integration tests for `rmng health --json` and `rmngd --validate`
- Configurable systemd paths via `rmngd.service.in` + `RMNG_PROJECT_ROOT`
- Install script skips restart on validate ERROR; docs sync
### Sprint 30 — Live LLM chain reliability ✅
- Orchestration prompt tuning: few-shot examples, error recovery hints, provider-specific guidance
- Parser hardening: semicolon chains, empty array filter, invalid chain drop + warn
- Live chain E2E tests (Groq, Grok, Ollama) with strict HandoffChain assertion
- Chain emission matrix tests; [live-llm-orchestration.md](integrations/live-llm-orchestration.md)
