# External Integration Roadmap — RMNG-OS

**Status:** Active (Sprint 12+)  
**Governance:** [INTEGRATION-STRATEGY.md](INTEGRATION-STRATEGY.md) · ADR-019 · ADR-010

Shift from internal hardening to **controlled external integration** — multiply LLM capability via MCP, skills, and auditable tools without blurring Nervous/Body boundaries.

---


## Sprint status (Sprints 17–22)

| Sprint | Focus | Status |
|--------|-------|--------|
| 17 | E2B sandbox MCP | ✅ |
| 18 | Testing & validation workflows | ✅ |
| 19 | Production hardening (circuits, budgets) | ✅ |
| 20 | Resource metrics & observability | ✅ |
| 21 | Seccomp + capability dropping | ✅ |
| 22 | Consolidation & polish | ✅ |
| 23 | Multi-hop orchestration | ✅ |

## Executive Summary

The GitHub Repos analysis (130+ entries) clusters into:

| Category | Count (approx) | RMNG default track |
|----------|----------------|-------------------|
| Full agent runtimes (LangChain, AutoGen, CrewAI, Dify…) | ~25 | **Track 4** — patterns only |
| MCP servers & infra | ~35 | **Track 2** — primary ingestion path |
| Skill packs & methodology | ~30 | **Track 3** — curated adaptation |
| Observability / gateways (Langfuse, LiteLLM…) | ~20 | **Track 4** or reference |
| Vector DBs / RAG platforms | ~15 | **Track 4** — defer |
| IDE / chat UIs | ~15 | **Track 4** — out of scope |

**RMNG already ships:** `github-mcp-server`, `mcp-server-git`, 6 skills, L1–L4 agents.

---

## Phase A — Next 2–3 Sprints (High Value, Low Risk)

| Priority | Repository | Track | Exposure | Effort | Risk | Status |
|----------|------------|-------|----------|--------|------|--------|
| A1 | [modelcontextprotocol/servers](https://github.com/modelcontextprotocol/servers) (`fetch`) | 2 | `mcp.proxy` → read-only URL fetch | S | Low | **Active** |
| A2 | [microsoft/playwright-mcp](https://github.com/microsoft/playwright-mcp) | 2 | `mcp.proxy` → DOM/a11y tree navigation | M | Med | **Active** (Sprint 14 E2E) |
| A3 | [microsoft/markitdown](https://github.com/microsoft/markitdown) | 2 | `mcp.proxy` → doc→markdown for context | M | Low | **Active** |
| A4 | [obra/superpowers](https://github.com/obra/superpowers) | 3 | `skills/tdd-discipline` — phase-gated TDD | S | Low | **Active** |
| A5 | [anthropics/skills](https://github.com/anthropics/skills) | 3 | Selective skill adaptation (spec format) | S | Low | **Active** |
| A6 | [github/github-mcp-server](https://github.com/github/github-mcp-server) | 2 | Read-only issue tools (`list_issues`, `get_issue`) | S | Low | **Active** (Sprint 14) |
| A7 | [mcp-server-git](https://github.com/modelcontextprotocol/servers/tree/main/src/git) | 2 | Expand tools (`git.diff`, `git.status`) | S | Low | **Active** (Sprint 14) |

### Phase A rejections (documented, not wired)

| Repository | Track | Reason |
|------------|-------|--------|
| [upstash/context7](https://github.com/upstash/context7) | 4 | ContextCrush / indirect prompt injection (doc analysis) |
| [punkpeye/awesome-mcp-servers](https://github.com/punkpeye/awesome-mcp-servers) | 4 | Unbounded surface — use as catalog only |
| [langchain-ai/langchain](https://github.com/langchain-ai/langchain) | 4 | In-process agent loop replaces RMNG nervous |

---

## Phase B — Medium Risk / Higher Reward (Sprints 14–16)

| Repository | Track | Value | Effort | Risk |
|------------|-------|-------|--------|------|
| [mem0ai/mem0](https://github.com/mem0ai/mem0) | 2 + 3 | Long-term memory via Mem0 MCP | M | Med | **Active** (Sprint 15) |
| [BerriAI/litellm](https://github.com/litellm/litellm) | 4 | Reference for gateway routing — RMNG has native providers | L | Med |
| [langfuse/langfuse](https://github.com/langfuse/langfuse) | 4 | External trace UI — complement audit.jsonl | M | Low |
| [promptfoo/promptfoo](https://github.com/promptfoo/promptfoo) | 3 | Rubric eval patterns in skills | M | Low | **Active** (Sprint 16) |
| [e2b-dev/E2B](https://github.com/e2b-dev/e2b) | 2 | Sandboxed code exec MCP (`run_code`) | L | High | **Active** (Sprint 17, opt-in) |
| [ChromeDevTools/chrome-devtools-mcp](https://github.com/ChromeDevTools/chrome-devtools-mcp) | 2 | Deep browser debug — after Playwright stable | M | Med |
| `kernel.status` native expansion | 1 | Hot-path kernel ops — small Rust handlers | M | Low |

---

## Phase C — Long-Term / Experimental

| Repository | Track | Notes |
|------------|-------|-------|
| [microsoft/autogen](https://github.com/microsoft/autogen) | 4 | Multi-agent conversation patterns → L4 skill only |
| [crewAIInc/crewAI](https://github.com/crewAIInc/crewAI) | 4 | Role-based crews → map to RMNG handoff chains |
| [geekan/MetaGPT](https://github.com/geekan/MetaGPT) | 4 | SOP emulation — skill extraction only |
| [All-Hands-AI/OpenHands](https://github.com/All-Hands-AI/OpenHands) | 4 | Full autonomous dev — Docker sandbox prerequisite |
| [langgenius/dify](https://github.com/langgenius/dify) | 4 | Competing orchestration plane |
| [infiniflow/ragflow](https://github.com/infiniflow/ragflow) | 4 | Heavy RAG stack — defer |
| [agentgateway/agentgateway](https://github.com/agentgateway/agentgateway) | 4 | MCP gateway — evaluate when multi-tenant |

---

## Top 12 Highest-Value Repositories (Ranked)

1. **modelcontextprotocol/servers** (fetch, git) — canonical MCP; extends body without Rust churn  
2. **microsoft/playwright-mcp** — DOM-first web agent; complements research-curator  
3. **github/github-mcp-server** — already partial; expand for issue/PR intelligence  
4. **obra/superpowers** — anti-drift TDD methodology → Track 3  
5. **anthropics/skills** — agentskills.io format alignment  
6. **microsoft/markitdown** — token-efficient document ingestion  
7. **mem0ai/mem0** — session memory beyond `shared_context`  
8. **promptfoo/promptfoo** — nervous prompt regression testing  
9. **e2b-dev/E2B** — sandboxed execution for future code tools  
10. **openai/openai-agents-python** — handoff/guardrail patterns (Track 3/4 reference)  
11. **agentskills/agentskills** — skill spec compliance  
12. **langfuse/langfuse** — optional external dashboard for audit/cost

---

## Sprint 12 First Batch Status

| Item | Track | Deliverable | Status |
|------|-------|-------------|--------|
| MCP Fetch | 2 | E2E tests + session write-back + isolation | ✅ Complete (Sprint 13) |
| Markitdown MCP | 2 | allowlist + doc-ingestion skill + example intent | ✅ Complete (Sprint 13) |
| web-researcher agent | L3 | 4 skills + budget + fetch/markitdown scope | ✅ Complete (Sprint 13) |
| Playwright MCP | 2 | allowlist + doc (opt-in) | ✅ Started (Sprint 12) |
| TDD discipline skill | 3 | `skills/tdd-discipline/SKILL.md` | ✅ Started |
| Context7 | 4 | rejection doc | ✅ Documented |

---

## How to Continue

1. **One MCP server per sprint** — register, test, audit, document  
2. **Two skills per sprint** — adapt external methodology, never vend wholesale  
3. **Agent YAML updates** — wire new MCP tools to L3 agents with least privilege  
4. **E2E test** — `agents/tests/mcp_e2e.rs` pattern for each new server  
5. **Never batch-allowlist** — explicit `allowed_tools` per server

See [integrations/README.md](integrations/README.md) for per-repo intake records.

## Sprint 14 Status

| Item | Track | Deliverable | Status |
|------|-------|-------------|--------|
| Playwright MCP | 2 | opt-in E2E + isolation + browser-researcher | ✅ Complete |
| GitHub MCP expand | 2 | list_issues, get_issue; create_issue removed | ✅ Complete |
| Git MCP expand | 2 | git.diff, git.status + repo-keeper scope | ✅ Complete |
| Markitdown live E2E | 2 | rmngd full loop + session write-back | ✅ Complete |
| Usage docs | — | browser-research, code-workflow guides | ✅ Complete |
## Sprint 15 Status

| Item | Track | Deliverable | Status |
|------|-------|-------------|--------|
| Mem0 MCP | 2 | add/search/get/delete + session write-back | ✅ Complete |
| memory-management skill | 3 | hygiene + privacy guidance | ✅ Complete |
| Agent memory scope | L3 | research-curator, web-researcher, repo-keeper | ✅ Complete |
| Usage docs | — | mem0-mcp.md, memory-usage.md | ✅ Complete |
## Sprint 16 Status

| Item | Track | Deliverable | Status |
|------|-------|-------------|--------|
| self-critique skill | 3 | llm-rubric adapted plan.only | ✅ Complete |
| output-validation skill | 3 | threshold + deterministic checks | ✅ Complete |
| improvement-loop skill | 3 | composable with memory/research | ✅ Complete |
| promptfoo patterns | 3 | intake doc; no full framework | ✅ Complete |
| Agent evaluation scope | L3 | 4 agents + budget limits | ✅ Complete |

## Sprint 17 Status

| Item | Track | Deliverable | Status |
|------|-------|-------------|--------|
| E2B MCP | 2 | run_code + opt-in E2E + isolation | ✅ Complete |
| code-execution skill | 3 | safety + testing patterns | ✅ Complete |
| Agent sandbox scope | L3 | repo-keeper, research-curator | ✅ Complete |
| Usage docs | — | e2b-mcp.md, sandbox-usage.md | ✅ Complete |

## Sprint 18 Status

| Item | Track | Deliverable | Status |
|------|-------|-------------|--------|
| run-tests skill | 3 | E2B harness composition + guards | ✅ Complete |
| validate-output skill | 3 | Sandbox result gate (plan.only) | ✅ Complete |
| test-coverage-check skill | 3 | Breadth rubric | ✅ Complete |
| regression-check skill | 3 | Session/Mem0 baseline compare | ✅ Complete |
| Agent testing scope | L3 | repo-keeper, research-curator | ✅ Complete |
| Usage docs | — | testing-usage.md | ✅ Complete |

## Sprint 19 Status

| Item | Track | Deliverable | Status |
|------|-------|-------------|--------|
| Persistent circuit breaker | Core | atomic save + cross-process reload | ✅ Complete |
| Per-agent observability | CLI | observe --cost/json breakdowns | ✅ Complete |
| Audit verify stats | CLI | CI-friendly --stats/--json | ✅ Complete |
| Per-profile budgets | Core | profile-scoped spend + governance report | ✅ Complete |
| Operations docs | — | operations-usage.md | ✅ Complete |

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
