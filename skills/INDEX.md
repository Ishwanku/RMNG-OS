# RMNG-OS Skills

Intent-shaping guides for the nervous system (BYO-LLM). **Do not execute tools directly** — emit JSON intents for `rmngd`.

| Skill | Path | Responsibility |
|-------|------|----------------|
| Kernel build | [kernel-build/SKILL.md](kernel-build/SKILL.md) | Patch application, rebuild workflows, ccache benchmarks, and `vmlinux` validation |
| Kernel config | [kernel-config/SKILL.md](kernel-config/SKILL.md) | `menuconfig`, slim vs full configs, `localmodconfig`, and config export scripts |
| Git workflow | [git-workflow/SKILL.md](git-workflow/SKILL.md) | Repo hygiene, commit standards, WSL push safety, and `gh` auth integration |
| GitHub workflow | [github-workflow/SKILL.md](github-workflow/SKILL.md) | PR status, `gh` context, and MCP `git.log` for commit history |
| Phase gates | [phase-gates/SKILL.md](phase-gates/SKILL.md) | Phase completion checks against `docs/ROADMAP.md` success criteria |

Format: [Agent Skills specification](https://agentskills.io).