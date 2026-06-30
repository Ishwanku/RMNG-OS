# Integrations — The Body (Phase 5+)

**Status:** Dev/kernel manifest live · runtime in `rmng-core`

## Role (ADR-010)

Tool execution layer invoked **only** by the local Rust runtime after permission checks. LLMs emit JSON intents; they never call integrations directly.

## Dispatch flow

```
LLM → JSON intent → rmng-core (validate + authorize) → tools/ → result → audit log
```

## Layout

```
integrations/
├── dev/
│   ├── kernel.json    # kernel lab tools (live)
│   └── README.md
├── data/              # planned
├── creative/          # planned
└── shared/            # planned
```

## Live tools

See [dev/kernel.json](dev/kernel.json). Invoke via:

```bash
rmng run -f agents/schemas/kernel-status.intent.json
rmng send -f agents/schemas/kernel-status.intent.json   # via rmngd
```

## Specs

- [REQUIREMENTS.md](../docs/REQUIREMENTS.md)
- [ARCHITECTURE.md](../docs/ARCHITECTURE.md)
- [DECISIONS.md](../docs/DECISIONS.md) — ADR-010
