---
name: phase-gates
description: RMNG-OS phase completion checks — evaluate whether a roadmap phase is done using docs/ROADMAP.md success criteria and experiment evidence. Use before declaring a phase complete.
---

# Phase Gates Skill

## When to use

- Before marking a roadmap phase complete
- Writing validation reports in `docs/experiments/`
- Reviewing whether deliverables match `docs/ROADMAP.md` success criteria
- Planning the next phase after confirming exit conditions

**Source of truth:** `docs/ROADMAP.md` — especially the **Success Criteria by Phase** table.

## How to evaluate a phase

1. Read the phase section and success criteria in `docs/ROADMAP.md`
2. Run concrete checks (commands, file existence, benchmarks)
3. Record evidence in `docs/experiments/phase<N>-validation-<date>.md`
4. Update `docs/ROADMAP.md` task checkboxes only when evidence exists

Do not mark a phase complete on intent alone — require reproducible checks.

## Phase gate reference

### Phase 1 — Environment & First Build ✅

| Check | Command / evidence |
|-------|-------------------|
| `vmlinux` exists | `ls -lh ~/build/kernel/vmlinux` |
| Repo on GitHub | `gh repo view Ishwanku/RMNG-OS` |
| WSL toolchain | `~/scripts/rmng-status.sh` passes |

### Phase 2 — Active Development Workflow

| Check | Command / evidence |
|-------|-------------------|
| Slim config builds | `~/scripts/rmng-build.sh` succeeds with slim `.config` |
| ccache rebuild < 5 min | See `docs/benchmarks/` (target: incremental under 300 s) |
| Daily scripts work | `workspace-setup.sh`, `status.sh`, `build.sh` run without error |

### Phase 3 — RMNG Identity ✅

| Check | Command / evidence |
|-------|-------------------|
| Patch applies cleanly | `./scripts/apply-patches.sh` exit 0 |
| Rebuild with patches | `./scripts/rebuild-with-patches.sh` exit 0 |
| RMNG banner in kernel | `strings "$KBUILD/vmlinux" \| grep RMNG-OS` |
| LOCALVERSION | `grep CONFIG_LOCALVERSION "$KBUILD/.config"` → `"-rmng"` |
| Validation report | `docs/experiments/phase3-validation-*.md` exists |

### Phase 4 — Advanced Kernel + Bare-Metal

| Check | Command / evidence |
|-------|-------------------|
| Optional advanced goal | Document chosen goal (WSL custom boot, eBPF, etc.) |
| Evidence | Experiment log + reproducible steps |

### Phase 5 — AI Agent Foundation

| Check | Command / evidence |
|-------|-------------------|
| Rust tests pass | `cd agents && cargo test` |
| Daemon running | `systemctl --user status rmngd` active |
| Permission gate | Unknown tools denied; audit at `~/.rmng/logs/audit.jsonl` |
| IPC works | `rmng send -f schemas/kernel-status.intent.json` returns JSON |
| BYO-LLM boundary | `rmng ask "..." --dry-run` emits intent only; no direct shell |

### Phase 6a — Skills + Dev MCP

| Check | Command / evidence |
|-------|-------------------|
| Four skills present | `skills/kernel-build`, `kernel-config`, `git-workflow`, `phase-gates` |
| Dev MCP config | `~/.config/rmng/mcp-dev.json` from `setup-dev-mcp.sh` |
| Production path unchanged | Tool execution still via `rmng send` / `rmngd` only |

## Example validation report structure

```markdown
# Phase N Validation — YYYY-MM-DD

## Criteria (from ROADMAP)
- [ ] criterion 1
- [ ] criterion 2

## Evidence
\`\`\`bash
# commands run and output summary
\`\`\`

## Result
PASS / FAIL — notes
```

## Tool intents (via rmngd)

| Intent tool | When |
|-------------|------|
| `kernel.status` | Phase 1–4 kernel environment checks |
| `git.status` | Repo cleanliness before phase sign-off |
| `kernel.build` | Rebuild verification after phase work |

## Constraints

- Phase gates are **evaluative** — this skill shapes how to assess completion, not how to skip steps
- Cross-check `docs/REQUIREMENTS.md` and `docs/DECISIONS.md` (ADRs) when a phase touches architecture
- Phase 6b+ (MCP bridge, multi-agent) have their own gates in `docs/PLAN-AGENTS-MCP-SKILLS.md`

## Validation

- Experiment file exists for the phase being closed
- ROADMAP success criteria row can be marked with evidence cited
- No criterion marked complete without a reproducible check