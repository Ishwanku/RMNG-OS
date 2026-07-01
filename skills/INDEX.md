# RMNG-OS Skills

Intent-shaping guides for the nervous system (BYO-LLM). **Do not execute tools directly** — emit JSON intents for `rmngd`.

| Skill | Path | Responsibility |
|-------|------|----------------|
| Kernel build | [kernel-build/SKILL.md](kernel-build/SKILL.md) | Patch application, rebuild workflows, ccache benchmarks, and `vmlinux` validation |
| Kernel config | [kernel-config/SKILL.md](kernel-config/SKILL.md) | `menuconfig`, slim vs full configs, `localmodconfig`, and config export scripts |
| Git workflow | [git-workflow/SKILL.md](git-workflow/SKILL.md) | Repo hygiene, commit standards, WSL push safety, and `gh` auth integration |
| GitHub workflow | [github-workflow/SKILL.md](github-workflow/SKILL.md) | PR status, `gh` context, and MCP `git.log` for commit history |
| Phase gates | [phase-gates/SKILL.md](phase-gates/SKILL.md) | Phase completion checks against `docs/ROADMAP.md` success criteria |
| TDD discipline | [tdd-discipline/SKILL.md](tdd-discipline/SKILL.md) | Red-green-refactor phase gates (superpowers-adapted) |
| Browser research | [browser-research/SKILL.md](browser-research/SKILL.md) | Opt-in Playwright MCP (DOM navigation) |
| Web research | [web-research/SKILL.md](web-research/SKILL.md) | MCP fetch for live URLs |
| Doc ingestion | [doc-ingestion/SKILL.md](doc-ingestion/SKILL.md) | Markitdown MCP for PDF/Office files |
| Doc summarization | [doc-summarization/SKILL.md](doc-summarization/SKILL.md) | Session-aware synthesis of tool results |
| Source evaluation | [source-evaluation/SKILL.md](source-evaluation/SKILL.md) | Integration track scoring for external repos |
| Self-critique | [self-critique/SKILL.md](self-critique/SKILL.md) | LLM-as-judge rubric (plan.only) |
| Output validation | [output-validation/SKILL.md](output-validation/SKILL.md) | Deterministic + threshold gate |
| Improvement loop | [improvement-loop/SKILL.md](improvement-loop/SKILL.md) | Bounded recall-critique-validate cycle |
| Memory management | [memory-management/SKILL.md](memory-management/SKILL.md) | Mem0 long-term memory hygiene and privacy |
| Code execution | [code-execution/SKILL.md](code-execution/SKILL.md) | E2B sandbox run_code — opt-in verification |
| MCP integration | [mcp-integration/SKILL.md](mcp-integration/SKILL.md) | Track 2 MCP proxy usage and allowlist rules |
| Spec compliance | [spec-compliance/SKILL.md](spec-compliance/SKILL.md) | agentskills.io + CoreIntent v2 alignment |
| Research workflow | [research-workflow/SKILL.md](research-workflow/SKILL.md) | Research and documentation curation |

Format: [Agent Skills specification](https://agentskills.io).
