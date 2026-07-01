# RMNG-OS Documentation Index

Central index for all project documentation. Start here when onboarding or sharing with collaborators / AI assistants.

## Quick links

| Document | Audience | Purpose |
|----------|----------|---------|
| [README.md](../README.md) | Everyone | Project entry point |
| [VISION.md](VISION.md) | Product / strategy | North star and layer model |
| [REQUIREMENTS.md](REQUIREMENTS.md) | Engineering | Functional & non-functional requirements |
| [ARCHITECTURE.md](ARCHITECTURE.md) | Engineering | Technical design and components |
| [ROADMAP.md](ROADMAP.md) | Planning | Phased delivery timeline |
| [PLAN-AGENTS-MCP-SKILLS.md](PLAN-AGENTS-MCP-SKILLS.md) | Architecture | Agents, skills, MCP adoption plan |
| [setup.md](setup.md) | Operators | WSL environment install guide |
| [daily-workflow.md](daily-workflow.md) | Developers | Day-to-day commands |
| [INTEGRATION-STRATEGY.md](INTEGRATION-STRATEGY.md) | Maintainers | **Future repo integration governance** |
| [integrations/](integrations/) | Maintainers | Per-repo evaluation docs |
| [experiments/phase3-validation-20260630.md](experiments/phase3-validation-20260630.md) | Kernel dev | Phase 3 RMNG identity validation |
| [DECISIONS.md](DECISIONS.md) | Maintainers | Architecture decision records |

## By role

### New contributor
1. [README.md](../README.md)
2. [setup.md](setup.md)
3. [daily-workflow.md](daily-workflow.md)

### Product / planning
1. [VISION.md](VISION.md)
2. [REQUIREMENTS.md](REQUIREMENTS.md)
3. [ROADMAP.md](ROADMAP.md)

### Kernel developer
1. [setup.md](setup.md)
2. [daily-workflow.md](daily-workflow.md)
3. [ARCHITECTURE.md](ARCHITECTURE.md) → Layer 1

### AI / agents
1. [VISION.md](VISION.md)
2. [REQUIREMENTS.md](REQUIREMENTS.md) → Layer 4–5
3. [ARCHITECTURE.md](ARCHITECTURE.md) → Agents & integrations
4. [INTEGRATION-STRATEGY.md](INTEGRATION-STRATEGY.md) → Adding OSS repos safely
5. [../agents/README.md](../agents/README.md)
6. [../integrations/README.md](../integrations/README.md)
7. [../skills/INDEX.md](../skills/INDEX.md)

### Adding a new open-source repo
1. Copy [integrations/TEMPLATE.md](integrations/TEMPLATE.md) → `integrations/<name>.md`
2. Score against [INTEGRATION-STRATEGY.md](INTEGRATION-STRATEGY.md) §3
3. Implement on the assigned track (1–4)
4. Register MCP (Track 2): `../scripts/register-mcp-tool.sh`

## Repository layout

```
RMNG-OS/
├── docs/           ← you are here
├── scripts/        ← automation
├── patches/        ← kernel patch series
├── config/         ← kernel & WSL templates
├── skills/         ← nervous-system skill guides
├── agents/         ← Rust runtime + definitions
└── integrations/   ← future workflow adapters
```

## Document status

| Document | Version | Last updated |
|----------|---------|--------------|
| REQUIREMENTS.md | 0.3 | 2026-06-30 |
| ARCHITECTURE.md | 0.2 | 2026-06-30 |
| benchmarks/phase2-validation-20260630.md | 1.0 | 2026-06-30 |
| VISION.md | 0.1 | 2026-06-27 |
| ROADMAP.md | 0.3 | 2026-06-30 |
| experiments/phase3-validation-20260630.md | 1.0 | 2026-06-30 |