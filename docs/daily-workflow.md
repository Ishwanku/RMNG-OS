# Daily Workflow

Common commands for day-to-day RMNG-OS kernel development.

## Start of Session

```bash
wsl -d Ubuntu-24.04          # from PowerShell, if needed
cd ~/dev/projects/RMNG-OS
~/scripts/rmng-status.sh     # verify kernel environment
rmngd --validate             # pre-flight: config, agents, dirs
rmng health --strict         # production liveness (daemon + circuits + budget)
rmng status                  # quick runtime summary
rmngd-status                 # alias — systemd user unit
```

## One-Time / Repeatable Setup

```bash
cd ~/dev/projects/RMNG-OS
./scripts/dev-environment-setup.sh   # dirs, ~/.rmng, shell snippets, rust check
./scripts/install-rmng.sh              # rmng + rmngd + allowlist + systemd
# Custom clone path:
# RMNG_PROJECT_ROOT=/path/to/RMNG-OS ./scripts/install-rmng.sh
./scripts/setup-dev-mcp.sh             # IDE MCP only (~/.config/rmng/)
./scripts/check-dev-prerequisites.sh   # report missing tools
```

## Build Environment

```bash
source ~/scripts/kernel-env.sh
# Sets: KSRC, KBUILD, CCACHE_DIR, CC, CXX
```

## Build Commands

```bash
# Full rebuild (6 jobs)
~/scripts/rmng-build.sh

# Specific target
~/scripts/rmng-build.sh modules
~/scripts/rmng-build.sh bzImage

# Single module directory
source ~/scripts/kernel-env.sh
make -C "$KSRC" O="$KBUILD" M=drivers/char -j6

# Clean artifacts (keeps .config)
make -C "$KSRC" O="$KBUILD" clean
```


## RMNG identity (Phase 3)

Apply RMNG patches and rebuild with branded LOCALVERSION:

```bash
cd ~/dev/projects/RMNG-OS
./scripts/rebuild-with-patches.sh
```

Apply patches only (no rebuild):

```bash
./scripts/apply-patches.sh
```

Verify RMNG banner in built kernel:

```bash
source ~/scripts/kernel-env.sh
strings "$KBUILD/vmlinux" | grep RMNG-OS
grep CONFIG_LOCALVERSION "$KBUILD/.config"
```

See `patches/README.md` for adding new patches to the series.

## Configuration

```bash
source ~/scripts/kernel-env.sh

# Interactive config editor
make -C "$KSRC" O="$KBUILD" menuconfig

# Slim config (only modules for loaded drivers)
make -C "$KSRC" O="$KBUILD" localmodconfig

# Sync example config to repo after changes
~/dev/projects/RMNG-OS/scripts/make-config-example.sh
```

## Monitoring

```bash
pgrep -c gcc                 # active compilers (0 = idle)
du -sh ~/build/kernel        # build dir size
ccache -s                    # cache statistics
ls -lh ~/build/kernel/vmlinux
```

## VS Code

```bash
code ~/dev/projects/RMNG-OS   # project tooling
code ~/dev/kernel/linux       # kernel source
```

## Git (RMNG-OS repo)

```bash
cd ~/dev/projects/RMNG-OS
git status
git add -p
git commit -m "Describe change"
git push origin main
```


## RMNG CLI (Phase 5)

```bash
cd ~/dev/projects/RMNG-OS/agents
cargo build

rmng status
rmng tools
rmngd &
rmng send -f schemas/kernel-status.intent.json
rmng run -f schemas/kernel-status.intent.json

# With Ollama running:
rmng ask "check kernel build status" --dry-run
rmng ask "check kernel build status"
```

## Dev MCP Servers (IDE Assistance)

MCP servers configured via `scripts/setup-dev-mcp.sh` are for **developer IDE assistance only** (Cursor, VS Code, etc.). They help you read files, inspect git history, fetch docs, and query GitHub during a coding session.

They are **not** part of the production execution path. RMNG-OS enforces nervous-system / body separation (ADR-010): the LLM and IDE reason; `rmngd` executes.

| Layer | Role | Examples |
|-------|------|----------|
| **Nervous** (IDE + LLM) | Read, plan, shape intents | MCP filesystem, git, fetch, github, memory |
| **Body** (`rmngd`) | Execute approved tools | `kernel.*`, `git.status` via `PermissionGate` |

**Production tool execution** always goes through:

```bash
rmng send -f schemas/kernel-status.intent.json
rmng ask "check kernel status"          # emits intent → rmngd
systemctl --user status rmngd           # daemon must be running
```

Do not route automated or unattended work through IDE MCP servers.

### Setup (one-time per machine)

```bash
cd ~/dev/projects/RMNG-OS
./scripts/setup-dev-mcp.sh
```

Writes `~/.config/rmng/mcp-dev.json` from `config/mcp-servers.wsl.example.json` with your `$USER` paths.

**Prerequisites:** Node.js (`npx`), `uv` (`uvx` for git MCP), `gh auth login` for GitHub MCP token.

Optional: merge entries into `~/.cursor/mcp.json` for Cursor. **Never commit** MCP config or tokens to the repo.

### Allowed paths (filesystem MCP)

| Path | Purpose |
|------|---------|
| `~/dev/projects/RMNG-OS` | Tooling, docs, patches, skills, agents |
| `~/dev/kernel/linux` | Kernel source (read; GPLv2, separate clone) |
| `~/build/kernel` | Out-of-tree build dir (read; do not commit artifacts) |

### Configured servers

| Server | Dev use | Production equivalent |
|--------|---------|----------------------|
| filesystem | Browse repo, docs, configs | Native tools + `kernel.status` |
| git | Rich git log/diff in IDE | `git.status` via `rmng send` |
| fetch | Kernel/WSL documentation lookup | Manual reference |
| github | Issues, PRs, Actions context | Planned `github.*` native tools |
| memory | Cross-session IDE notes | `~/.rmng/` runtime (Phase 7+) |

The Rust MCP bridge (`rmng-mcp`) is **Phase 6b** — production path uses `~/.rmng/mcp-allowlist.toml`.

## Production MCP Allowlist (rmngd)

```bash
# View / edit
mcp-allowlist                    # alias → ~/.rmng/mcp-allowlist.toml
cat ~/.rmng/mcp-allowlist.toml

# Register a new server (Track 2)
./scripts/register-mcp-tool.sh git uvx mcp-server-git \
  --repository ~/dev/projects/RMNG-OS --tools git.log

rmngd-restart
rmng send -f agents/schemas/mcp-git-log.intent.json
rmng-audit                       # tail audit log
```

Governance: [INTEGRATION-STRATEGY.md](INTEGRATION-STRATEGY.md) · per-repo docs in [integrations/](integrations/).

## Skills (Nervous Context — Track 3)

```bash
ls skills/
cat skills/INDEX.md
rmng ask "follow kernel-build skill" --skill kernel-build --dry-run
```

Skills inform the LLM; they never execute tools directly (ADR-010).

## ~/.rmng Layout

| Path | Purpose |
|------|---------|
| `config.toml` | BYO-LLM settings |
| `mcp-allowlist.toml` | Production MCP proxy allowlist |
| `logs/audit.jsonl` | Permission + dispatch audit |
| `sessions/` | Ephemeral session artifacts |
| `allowlists/` | Allowlist change backups |

## Troubleshooting

| Problem | Solution |
|---------|----------|
| Out of disk | `make O=~/build/kernel clean` |
| Build too slow | Ensure ccache active: `which gcc` → `/usr/lib/ccache/gcc` |
| Wrong config | `cp config/wsl-kernel.config.example $KBUILD/.config && make olddefconfig` |
| `git push` hangs | Run `gh auth login` in WSL |
| Concurrent rebuild / `ld: bad reloc` | Only one build at a time; `rebuild-with-patches.sh` uses flock. If stuck: `pgrep make`, wait, or `make O=$KBUILD clean` |