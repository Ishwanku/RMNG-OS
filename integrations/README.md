# Integrations (Phase A — placeholder)

Future home for **workflow domain adapters** that AI agents will call.

## Planned structure

```
integrations/
├── dev/          # git, build, kernel, containers
├── data/         # databases, files, notebooks
├── creative/     # docs, design, media
├── business/     # email, calendar, CRM
├── infra/        # cloud, deploy, monitoring
└── shared/       # auth, config, logging
```

## Adapter contract (draft)

Each integration should expose:

- `name` — unique identifier
- `tools[]` — callable actions with JSON schema
- `auth` — credential requirements
- `permissions` — what the agent may access

**Status:** Not implemented.

**Specs:** [REQUIREMENTS.md](../docs/REQUIREMENTS.md) (FR-L3-*) · [ARCHITECTURE.md](../docs/ARCHITECTURE.md) (Layer 3) · Complete [Layer 1](../docs/ROADMAP.md) first.