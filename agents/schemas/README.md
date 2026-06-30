# Intent Schemas

Versioned JSON intents cross the nervous-system / body boundary (ADR-010).

| File | Purpose |
|------|---------|
| `intent.schema.json` | JSON Schema for all intents (v1) |
| `kernel-status.intent.json` | Example: run `kernel.status` tool |

Validate via CLI:

```bash
cargo run -p rmng-cli -- intent -f schemas/kernel-status.intent.json
```
