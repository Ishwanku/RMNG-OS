# Agent Definitions (RMNG specialists)

YAML capability manifests for Phase 7 multi-agent routing. **Not** third-party agent packs.

| ID | File | Native tools | MCP |
|----|------|--------------|-----|
| `kernel-engineer` | [kernel-engineer.yaml](kernel-engineer.yaml) | `kernel.*` | — |
| `repo-keeper` | [repo-keeper.yaml](repo-keeper.yaml) | `git.*`, `github.pr_status` | `git:git.log` |

## Usage

```bash
export RMNG_PROJECT_ROOT=~/dev/projects/RMNG-OS
rmng ask --agent repo-keeper "check git status"
rmng ask --agent kernel-engineer "kernel status"
```

Router loads definitions here, activates agent skills (full `SKILL.md` on demand), and **narrows allowed tools before** dispatch to `rmngd`.

Runtime: `rmng-nervous` (`AgentRouter`) · definitions only — execution remains in `rmngd`.