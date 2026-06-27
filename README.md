# RMNG-OS

A personal Linux kernel development environment for **WSL2**, focused on compiling and experimenting with the Linux kernel from source, performance tuning, and clean development workflows.

This repository contains **tooling and configuration only** — not the Linux kernel source tree or build artifacts. Clone [torvalds/linux](https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git) separately.

## Features

- Out-of-tree kernel builds (keeps source tree clean)
- **ccache** integration for faster rebuilds
- WSL2 performance tuning (`wsl.conf` + `.wslconfig` templates)
- Sanitized baseline kernel config for WSL2 (see `config/`)
- VS Code + WSL workflow support

## Prerequisites

- Windows 10/11 with WSL2
- Ubuntu 24.04 LTS
- Build tools: `build-essential`, `flex`, `bison`, `libssl-dev`, `libelf-dev`, `libncurses-dev`, `bc`, `rsync`, `kmod`, `cpio`, `dwarves`, `zstd`, `ccache`

```bash
sudo apt update
sudo apt install -y build-essential libncurses-dev bison flex libssl-dev \
  libelf-dev bc rsync kmod cpio dwarves zstd libudev-dev libiberty-dev \
  pkg-config ccache git python3
```

## Quick Start

### 1. Clone this repository

```bash
mkdir -p ~/dev/projects
cd ~/dev/projects
git clone https://github.com/Ishwanku/RMNG-OS.git
```

### 2. Set up folder structure

```bash
mkdir -p ~/dev/kernel ~/build/kernel ~/scripts
cp ~/dev/projects/RMNG-OS/scripts/kernel-env.sh ~/scripts/
chmod +x ~/scripts/kernel-env.sh
```

### 3. Clone kernel source (separate repo)

```bash
cd ~/dev/kernel
git clone --depth=1 https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git
```

### 4. Optional — apply ccache to shell

```bash
cat ~/dev/projects/RMNG-OS/dotfiles/bashrc.ccache.snippet >> ~/.bashrc
```

### 5. Build the kernel (out-of-tree)

```bash
source ~/scripts/kernel-env.sh

# Use the example config as a starting point
cp ~/dev/projects/RMNG-OS/config/wsl-kernel.config.example "$KBUILD/.config"
make -C "$KSRC" O="$KBUILD" olddefconfig

# Compile (adjust -j to your CPU count)
make -C "$KSRC" O="$KBUILD" -j6
```

### 6. Verify

```bash
ls -lh ~/build/kernel/vmlinux
ccache -s
```

## Repository Structure

```
RMNG-OS/
├── README.md
├── LICENSE
├── .gitignore
├── scripts/
│   └── kernel-env.sh       # Build environment variables
├── config/
│   ├── wsl.conf.example    # WSL distro config (inside Ubuntu)
│   ├── wslconfig.example   # WSL2 limits (Windows-side)
│   └── wsl-kernel.config.example  # Sanitized kernel .config baseline
├── docs/
│   └── setup.md            # Full setup guide
└── dotfiles/
    └── bashrc.ccache.snippet
```

## WSL Optimization

Copy and adapt the example configs:

| File | Location | Purpose |
|------|----------|---------|
| `config/wsl.conf.example` | `/etc/wsl.conf` | systemd, automount tuning |
| `config/wslconfig.example` | `C:\Users\<you>\.wslconfig` | RAM, CPU, swap limits |

After changes: `wsl --shutdown` from PowerShell, then reopen Ubuntu.

## VS Code

```bash
cd ~/dev/kernel/linux
code .
```

Install the **WSL** extension and use **Reopen in WSL**.

## What Is NOT in This Repo

- Linux kernel source (`~/dev/kernel/linux`) — clone separately
- Build output (`~/build/kernel/`) — generated locally
- ccache data (`~/.ccache/`) — local cache

## License

MIT License — see [LICENSE](LICENSE).

**Note:** The Linux kernel source you compile separately is licensed under **GPLv2**. This repository's scripts and configs are MIT-licensed tooling only.