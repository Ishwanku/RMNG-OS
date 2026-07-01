# End-to-End Workflow Example

**Research → memory → evaluation → execution → test** — a typical RMNG-OS integration sprint flow.

## 1. Research (fetch + GitHub)

```bash
rmng ask -a research-curator --session "$SID" \
  "search github issues tagged integration; fetch any linked RFC URLs"
```

MCP: `github.search_issues`, `fetch`, optional `markitdown.convert_to_markdown`.

## 2. Memory (Mem0, opt-in)

```bash
rmng ask -a web-researcher --session "$SID" \
  "remember: Sprint 22 focus is consolidation, not new features"
```

MCP: `mem0.add_memory` → later `mem0.search_memories`.

## 3. Evaluation (plan-only + skills)

```bash
rmng ask -a research-curator -s validate-output --session "$SID" \
  "evaluate whether the integration meets ADR-010 track rules"
```

Produces `plan.only` or structured validation without body execution.

## 4. Execution (git / sandbox)

```bash
rmng ask -a repo-keeper --session "$SID" "git diff and status"
# opt-in sandbox:
rmng ask -a repo-keeper --session "$SID" "run code to parse test output"
```

MCP: `git.*` or `e2b.run_code`.

## 5. Test (skills + E2B)

```bash
rmng ask -a repo-keeper -s run-tests --session "$SID" "run cargo test for agents crate"
```

Uses testing skills; may generate `e2b.run_code` or native tool intents.

## Verify along the way

```bash
rmng observe --cost
rmng audit verify --stats
rmng session show "$SID"
```

## Related docs

- [recommended-agent-setups.md](recommended-agent-setups.md)
- [testing-usage.md](testing-usage.md)
- [evaluation-usage.md](evaluation-usage.md)
- [memory-usage.md](memory-usage.md)
- [operations-usage.md](operations-usage.md)

## Multi-hop chains (Sprint 23)

Orchestrator emits `plan.only` with `metadata.handoff_chain`:

```json
{
  "action": "plan.only",
  "reasoning": "Delegate git check then execution.",
  "metadata": {
    "session_id": "<sid>",
    "handoff_chain": ["swarm-coordinator", "repo-keeper", "runtime-executor"],
    "chain_id": "<sid>"
  }
}
```

Specialist returns control with `handoff_return_to`:

```json
{
  "action": "plan.only",
  "reasoning": "Git status captured; returning summary.",
  "metadata": {
    "session_id": "<sid>",
    "handoff_return_to": "swarm-coordinator"
  }
}
```

CLI:

```bash
rmng session new
rmng ask --agent swarm-coordinator --session <sid> "delegate chain for git hygiene"
rmng ask --agent repo-keeper --session <sid> "report back to orchestrator"
rmng handoff --session <sid> --chain swarm-coordinator,repo-keeper,runtime-executor --prompt "explicit chain"
```

Session `shared_context.orchestration` tracks chain progress; `prompt_context` exposes it to subsequent LLM calls.

