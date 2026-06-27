# RMNG-OS — Full WSL Kernel Development Setup

Step-by-step guide for reproducing the development environment on a fresh WSL2 Ubuntu 24.04 install.

## 1. System Requirements

| Resource | Recommended |
|----------|-------------|
| Host RAM | 16 GB+ (allocate 12 GB to WSL) |
| Disk | 50 GB+ free for source + builds |
| CPUs | 6+ for parallel builds |

## 2. Folder Structure

```bash
mkdir -p ~/dev/{kernel,projects,tools}
mkdir -p ~/build/{kernel,out}
mkdir -p ~/scripts ~/src ~/dotfiles
```

| Path | Purpose |
|------|---------|
| `~/dev/kernel/linux` | Kernel source (git clone) |
| `~/build/kernel` | Out-of-tree build output |
| `~/scripts` | Helper scripts |
| `~/.ccache` | Compiler cache (auto-created) |

## 3. Install Build Dependencies

```bash
sudo apt update
sudo apt install -y \
  build-essential libncurses-dev bison flex libssl-dev libelf-dev \
  libiberty-dev libudev-dev bc rsync kmod cpio dwarves zstd \
  pkg-config ccache git python3 python3-pip
```

## 4. WSL Performance Tuning

### Inside WSL — `/etc/wsl.conf`

```bash
sudo cp config/wsl.conf.example /etc/wsl.conf
# Edit YOUR_USERNAME in the file
```

### Windows-side — `.wslconfig`

Copy `config/wslconfig.example` to `C:\Users\<you>\.wslconfig` and adjust memory/CPU for your host.

Apply both:

```powershell
wsl --shutdown
wsl -d Ubuntu-24.04
```

Verify:

```bash
free -h    # ~12 GB
nproc      # 6
```

## 5. Kernel Source

```bash
cd ~/dev/kernel
git clone --depth=1 https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git
```

For full git history later: `git -C linux fetch --unshallow`

## 6. Build Environment

```bash
cp scripts/kernel-env.sh ~/scripts/
chmod +x ~/scripts/kernel-env.sh
cat dotfiles/bashrc.ccache.snippet >> ~/.bashrc
```

## 7. Kernel Configuration

### Option A — Example config (from this repo)

```bash
source ~/scripts/kernel-env.sh
mkdir -p "$KBUILD"
cp config/wsl-kernel.config.example "$KBUILD/.config"
make -C "$KSRC" O="$KBUILD" olddefconfig
```

### Option B — Copy running WSL kernel config

```bash
zcat /proc/config.gz > "$KBUILD/.config"
make -C "$KSRC" O="$KBUILD" olddefconfig
```

### Option C — Interactive

```bash
make -C "$KSRC" O="$KBUILD" menuconfig
```

## 8. Compile

```bash
source ~/scripts/kernel-env.sh
make -C "$KSRC" O="$KBUILD" -j6
```

First build: 60–90 minutes. Rebuilds are faster with ccache.

## 9. Verify Build

```bash
ls -lh ~/build/kernel/vmlinux    # ~400–500 MB
pgrep -c gcc                     # 0 when done
ccache -s
```

## 10. VS Code Integration

```bash
cd ~/dev/kernel/linux
code .
```

## Troubleshooting

| Issue | Fix |
|-------|-----|
| `vmlinux` not found | Build still running; check `pgrep -c gcc` |
| Out of disk | `make O=~/build/kernel clean` removes artifacts |
| Build too large (~14 GB) | Normal with full config; use `localmodconfig` to slim |
| `sudo` password in scripts | Use `wsl -u root` for apt, or enter password interactively |

## Build Size Notes

A full WSL-derived config produces ~14 GB of build artifacts (drivers, modules, debug info). The final `vmlinux` is ~450 MB. Build artifacts stay in `~/build/kernel/` and are gitignored if you work inside this repo.