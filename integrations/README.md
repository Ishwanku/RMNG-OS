# Integrations — The Body (Phase 5+)

**Status:** Specification locked · Implementation not started

## Role in the architecture (ADR-010)

`integrations/` is the **Body** — tool execution layer invoked **only** by the local Rust runtime after permission checks.

**Prohibited:** Direct LLM access to any integration, raw terminal, or unvalidated parameters.

## Contract (mandatory)

Every integration exposes:

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Unique identifier |
| `version` | semver | API version |
| `tools` | array | Callable actions with JSON Schema parameters |
| `auth` | object | Credential requirements |

### Dispatch flow

```
LLM → JSON intent → Rust runtime (validate + authorize) → integrations/ → result → runtime → user
```

## Planned structure

```
integrations/
├── dev/          # git, build, kernel, containers
├── data/         # databases, files, notebooks
├── creative/     # docs, design, media
├── business/     # email, calendar, CRM
├── infra/        # cloud, deploy, monitoring
└── shared/       # http_get, read_file (allowlisted)
```

## Specs

- [REQUIREMENTS.md](../docs/REQUIREMENTS.md) — FR-L3-*, FR-L3-09, FR-L3-10
- [ARCHITECTURE.md](../docs/ARCHITECTURE.md) — Layer 3
- [DECISIONS.md](../docs/DECISIONS.md) — ADR-010