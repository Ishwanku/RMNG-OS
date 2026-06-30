# Dev integrations

Body-layer tool manifests for the kernel lab. Dispatched only via `rmng-core` after permission checks.

| Manifest | Tools |
|----------|-------|
| [kernel.json](kernel.json) | `kernel.status`, `kernel.build`, `kernel.apply_patches` |

Runtime implementation: `agents/rmng-core/src/tools/kernel.rs`
