# ADR-019: Licensing and Layering Philosophy

**Date:** 2026-07-01  
**Status:** **Accepted**  
**Supersedes:** ADR-007 (MIT license for RMNG-OS tooling)  
**Related:** ADR-003 (kernel not in repo), ADR-010 (nervous/body), ADR-017 (multi-level agents), [INTEGRATION-STRATEGY.md](../INTEGRATION-STRATEGY.md)

---

## Context

### Current state

RMNG-OS has evolved from a kernel lab repository into a **multi-layer AI operating environment**:

| Layer | Contents | Prior license stance |
|-------|----------|----------------------|
| Kernel patches | `patches/` applied to upstream Linux | Implicitly GPLv2 (Linux upstream) |
| Core runtime | `agents/` (`rmng-core`, `rmng-nervous`, `rmng-cli`, `rmngd`, `rmng-mcp`) | MIT (ADR-007) |
| Tooling & config | `scripts/`, `config/`, `skills/`, `integrations/` | MIT (ADR-007) |
| Documentation | `docs/` | MIT (ADR-007) |
| External OSS | MCP servers, LLM APIs, Ollama, etc. | Third-party licenses (consumed, not relicensed) |

ADR-007 chose MIT for all RMNG-OS tooling to maximize reuse and simplicity during early scaffolding.

### Problems with MIT for the core runtime

1. **Loss of control** — MIT allows anyone to copy, modify, sublicense, and sell the agent runtime, nervous system, and orchestration logic without contribution back or attribution beyond the license notice.
2. **Strategic misalignment** — RMNG-OS is building a **personal AI-first OS** with proprietary orchestration, session semantics, and permission gates. Permissive licensing treats the runtime as disposable infrastructure rather than core IP.
3. **Confusion with Linux** — The project follows a **Torvalds-style layering model** (kernel ≠ userspace ≠ intelligence). Linux is **GPLv2**, not MIT. Keeping RMNG tooling MIT while kernel work is GPLv2 sends mixed signals about what is “open” vs “controlled.”
4. **Integration boundary blur** — Consuming MIT MCP servers (Track 2) does not require relicensing RMNG-OS as MIT. A clear proprietary core + bounded OSS consumption is architecturally cleaner.

### Torvalds-style inspiration (not relicensing Linux)

[Linus Torvalds’ public repositories](https://github.com/torvalds?tab=repositories) illustrate a consistent pattern:

- **Linux kernel** — GPLv2; copyleft when distributed; intelligence stays out of kernel space.
- **Userspace applications** — Separate projects with their own licenses (often GPL for apps like Subsurface).
- **Small utilities** — Focused, auditable tools; not monolithic agent frameworks in the kernel tree.

RMNG-OS adopts the **layering discipline**, not a blanket “open everything” license:

```text
┌─────────────────────────────────────────────────────────────┐
│  L4–L3 — Orchestration & domain agents (userspace)          │
│  Proprietary RMNG runtime — reasoning via nervous system    │
├─────────────────────────────────────────────────────────────┤
│  L2 — rmngd, PermissionGate, audit, MCP proxy (Body)        │
│  Proprietary — execution policy is core IP                    │
├─────────────────────────────────────────────────────────────┤
│  L1 — Kernel patches, modules, lab scripts                  │
│  GPLv2 when derived from / distributed with Linux           │
├─────────────────────────────────────────────────────────────┤
│  External OSS — MCP servers, LLM SDKs, Ollama               │
│  Original licenses — subprocess / API boundaries (Track 2)    │
└─────────────────────────────────────────────────────────────┘
```

**Invariant (ADR-010):** LLM intelligence lives in userspace (nervous system). The kernel provides resources and policy hooks only. Licensing mirrors this split.

---

## Decision

Adopt a **split licensing model**:

| Component | Path(s) | License |
|-----------|---------|---------|
| **Core runtime** | `agents/` (all crates), including `rmngd`, nervous system, CLI, MCP proxy implementation | **Proprietary — All Rights Reserved** |
| **Kernel patches** | `patches/` | **GPLv2** (see [LICENSE.kernel-patches](../../LICENSE.kernel-patches)) |
| **Scripts, docs, config, skills, integrations manifests** | `scripts/`, `docs/`, `config/`, `skills/`, `integrations/` | **Proprietary** (same as core) |
| **External tools & dependencies** | npm MCP servers, crates.io deps, API clients | **Their original licenses** — used via integration tracks; no relicensing |

### Consumption vs publication

| Action | Allowed | Notes |
|--------|---------|-------|
| Run MIT/Apache MCP server as subprocess | ✅ | Track 2 — allowlisted; RMNG core stays proprietary |
| Link to crates.io / use OSS libraries | ✅ | Dependency licenses apply to those libs only |
| Publish RMNG agent runtime under MIT | ❌ | Supersedes ADR-007 |
| Distribute modified kernel without source | ❌ | GPLv2 obligations apply to kernel binaries |

---

## Consequences

### Positive

- ✅ **Core IP protected** — orchestration, session store, permission gate, and nervous/body boundary remain under owner control.
- ✅ **Linux-aligned kernel stance** — patches documented as GPLv2; consistent with upstream `torvalds/linux`.
- ✅ **Clear integration story** — external OSS enters via [INTEGRATION-STRATEGY.md](../INTEGRATION-STRATEGY.md) tracks; core license unchanged.
- ✅ **Torvalds-style layering** — kernel thin, intelligence in userspace, proprietary runtime as the “distribution you control.”

### Trade-offs

- ⚠️ **No casual reuse** — third parties cannot legally fork and ship the RMNG runtime without permission.
- ⚠️ **GitHub visibility** — a public repo with proprietary LICENSE still shows source; consider private repo or selective publication for true confidentiality.
- ⚠️ **Contributor agreements** — future contributors need explicit CLA or assignment if core remains proprietary.
- ⚠️ **Cargo.toml metadata** — workspace `license` field updated to `LicenseRef-RMNG-Proprietary` (metadata only).

### Distribution impact

| Artifact | Distribution note |
|----------|-------------------|
| RMNG-OS git repo (tooling + agents) | Proprietary — personal / authorized use only unless otherwise licensed |
| Built `rmng` / `rmngd` binaries | Same — no implied open-source grant |
| Kernel with RMNG patches | GPLv2 — offer corresponding source for patched kernel if you distribute binaries |
| MCP tool subprocesses | Governed by each server’s license (e.g. MIT for many MCP reference servers) |

---

## Migration steps

No runtime code changes required. Documentation and legal files only:

1. ✅ Replace root [LICENSE](../../LICENSE) with proprietary terms.
2. ✅ Add [LICENSE.kernel-patches](../../LICENSE.kernel-patches) for `patches/`.
3. ✅ Update [README.md](../../README.md), [VISION.md](../VISION.md), [INTEGRATION-STRATEGY.md](../INTEGRATION-STRATEGY.md).
4. ✅ Mark ADR-007 superseded in [DECISIONS.md](../DECISIONS.md); link to this ADR.
5. ✅ Update `agents/Cargo.toml` workspace `license` field (metadata).
6. ✅ Update [REQUIREMENTS.md](../REQUIREMENTS.md) constraint C-05.
7. 🔲 Decide GitHub repo visibility (public read-only vs private).
8. 🔲 Audit git history — prior MIT commits remain MIT for those snapshots; new license applies going forward (consider `LICENSE-change` note in release).

---

## References

- [Linux kernel — GPLv2](https://github.com/torvalds/linux)
- ADR-007 (superseded): MIT license for RMNG-OS tooling
- ADR-010: Nervous system / body separation
- [INTEGRATION-STRATEGY.md](../INTEGRATION-STRATEGY.md): Track 1–4 integration boundaries