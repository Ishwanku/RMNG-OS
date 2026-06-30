# Daily Workflow

Common commands for day-to-day RMNG-OS kernel development.

## Start of Session

```bash
wsl -d Ubuntu-24.04          # from PowerShell, if needed
cd ~/dev/projects/RMNG-OS
~/scripts/rmng-status.sh     # verify environment
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

## Troubleshooting

| Problem | Solution |
|---------|----------|
| Out of disk | `make O=~/build/kernel clean` |
| Build too slow | Ensure ccache active: `which gcc` → `/usr/lib/ccache/gcc` |
| Wrong config | `cp config/wsl-kernel.config.example $KBUILD/.config && make olddefconfig` |
| `git push` hangs | Run `gh auth login` in WSL |