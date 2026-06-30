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
| [setup.md](setup.md) | Operators | WSL environment install guide |
| [daily-workflow.md](daily-workflow.md) | Developers | Day-to-day commands |
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

### AI / agents (future)
1. [VISION.md](VISION.md)
2. [REQUIREMENTS.md](REQUIREMENTS.md) → Layer 4–5
3. [ARCHITECTURE.md](ARCHITECTURE.md) → Agents & integrations
4. [../agents/README.md](../agents/README.md)
5. [../integrations/README.md](../integrations/README.md)

## Repository layout

```
RMNG-OS/
├── docs/           ← you are here
├── scripts/        ← automation
├── config/         ← kernel & WSL templates
├── agents/         ← future agent runtime
└── integrations/   ← future workflow adapters
```

## Document status

| Document | Version | Last updated |
|----------|---------|--------------|
| REQUIREMENTS.md | 0.1 | 2026-06-30 |
| ARCHITECTURE.md | 0.1 | 2026-06-30 |
| VISION.md | 0.1 | 2026-06-27 |
| ROADMAP.md | 0.2 | 2026-06-30 |