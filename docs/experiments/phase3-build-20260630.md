# Phase 3 RMNG Kernel Build
Date: 2026-06-30T11:10:20+00:00

## Step 1: Apply patches
Set cache size limit to 10.0 GB
Kernel build environment ready:
  KSRC  = /home/saini/dev/kernel/linux   (kernel source)
  KBUILD= /home/saini/build/kernel (out-of-tree build dir)
  CCACHE= /home/saini/.ccache
=== RMNG-OS apply-patches ===
KSRC: /home/saini/dev/kernel/linux
Series: /home/saini/dev/projects/RMNG-OS/patches/series

--- Reset kernel source to clean state ---
make: Entering directory '/home/saini/dev/kernel/linux'
make: Leaving directory '/home/saini/dev/kernel/linux'

--- Applying patches ---
Applying: 0001-rmng-boot-banner.patch
patching file init/main.c

Patches applied successfully.
 init/main.c | 1 +
 1 file changed, 1 insertion(+)

## Step 2: Configure LOCALVERSION
make: Entering directory '/home/saini/dev/kernel/linux'
make[1]: Entering directory '/home/saini/build/kernel'
#
# No change to .config
#
make[1]: Leaving directory '/home/saini/build/kernel'
make: Leaving directory '/home/saini/dev/kernel/linux'
LOCALVERSION: CONFIG_LOCALVERSION="-rmng"
make: Entering directory '/home/saini/dev/kernel/linux'
make[1]: Entering directory '/home/saini/build/kernel'
7.1.0-rmng+
make[1]: Leaving directory '/home/saini/build/kernel'
make: Leaving directory '/home/saini/dev/kernel/linux'
kernel.release: 

## Step 3: Build
make: Entering directory '/home/saini/dev/kernel/linux'
make[1]: Entering directory '/home/saini/build/kernel'
  DESCEND objtool
  DESCEND bpf/resolve_btfids
  INSTALL libsubcmd_headers
  UPD     include/config/kernel.release
make[1]: Leaving directory '/home/saini/build/kernel'
make: Leaving directory '/home/saini/dev/kernel/linux'
nel/linux'
eaving directory '/home/saini/dev/kernel/linux'
o
  make[1]: Leaving directory '/home/saini/build/kernel'
make: Leaving directory '/home/saini/dev/kernel/linux'
.a
  CC      kernel/module/main.o
  AR      kernel/module/built-in.a
  AR      kernel/built-in.a
  CC      drivers/base/firmware_loader/main.o
  CC      drivers/gpu/drm/vmwgfx/vmwgfx_drv.o
  AR      drivers/base/firmware_loader/built-in.a
  AR      drivers/base/built-in.a
  AR      drivers/gpu/drm/vmwgfx/built-in.a
  AR      drivers/gpu/drm/built-in.a
  AR      drivers/gpu/built-in.a
  AR      drivers/built-in.a
  AR      built-in.a
  AR      built-in-fixup.a
  COPY    vmlinux.a
  LD      vmlinux.o
  MODPOST Module.symvers
  UPD     include/generated/utsversion.h
  CC      init/version-timestamp.o
  KSYMS   .tmp_vmlinux0.kallsyms.S
  AS      .tmp_vmlinux0.kallsyms.o
  LD      .tmp_vmlinux1
  BTF     .tmp_vmlinux1
  NM      .tmp_vmlinux1.syms
  KSYMS   .tmp_vmlinux1.kallsyms.S
  AS      .tmp_vmlinux1.kallsyms.o
  LD      .tmp_vmlinux2
  NM      .tmp_vmlinux2.syms
  KSYMS   .tmp_vmlinux2.kallsyms.S
  AS      .tmp_vmlinux2.kallsyms.o
  LD      vmlinux.unstripped
  BTFIDS  vmlinux.unstripped
  NM      System.map
  SORTTAB vmlinux.unstripped
  OBJCOPY vmlinux
  GEN     modules.builtin.modinfo
  GEN     modules.builtin
  CC      arch/x86/boot/version.o
  CC [M]  .module-common.o
  LD [M]  arch/x86/kvm/kvm.ko
  LD [M]  arch/x86/kvm/kvm-intel.ko
  LD [M]  arch/x86/kvm/kvm-amd.ko
  LD [M]  fs/autofs/autofs4.ko
  LD [M]  drivers/acpi/ac.ko
  BTF [M] drivers/acpi/ac.ko
  BTF [M] fs/autofs/autofs4.ko
  VOFFSET arch/x86/boot/compressed/../voffset.h
  BTF [M] arch/x86/kvm/kvm-amd.ko
  BTF [M] arch/x86/kvm/kvm-intel.ko
  BTF [M] arch/x86/kvm/kvm.ko
  LD [M]  drivers/acpi/battery.ko
  BTF [M] drivers/acpi/battery.ko
  LD [M]  drivers/net/tun.ko
  BTF [M] drivers/net/tun.ko
  OBJCOPY arch/x86/boot/compressed/vmlinux.bin
  RELOCS  arch/x86/boot/compressed/vmlinux.relocs
  LD [M]  drivers/powercap/intel_rapl_common.ko
  BTF [M] drivers/powercap/intel_rapl_common.ko
  LD [M]  drivers/powercap/intel_rapl_msr.ko
  BTF [M] drivers/powercap/intel_rapl_msr.ko
  CC      arch/x86/boot/compressed/kaslr.o
  LD [M]  net/802/psnap.ko
  BTF [M] net/802/psnap.ko
  LD [M]  net/802/stp.ko
  BTF [M] net/802/stp.ko
  LD [M]  net/sched/sch_fq_codel.ko
  BTF [M] net/sched/sch_fq_codel.ko
  LD [M]  net/ipv4/netfilter/ip_tables.ko
  BTF [M] net/ipv4/netfilter/ip_tables.ko
  LD [M]  net/llc/llc.ko
  BTF [M] net/llc/llc.ko
  LD [M]  net/tls/tls.ko
  LD [M]  net/bridge/bridge.ko
  BTF [M] net/tls/tls.ko
  BTF [M] net/bridge/bridge.ko
  LD [M]  net/bridge/br_netfilter.ko
  LD [M]  virt/lib/irqbypass.ko
  BTF [M] net/bridge/br_netfilter.ko
  BTF [M] virt/lib/irqbypass.ko
  CC      arch/x86/boot/compressed/misc.o
  GZIP    arch/x86/boot/compressed/vmlinux.bin.gz
  MKPIGGY arch/x86/boot/compressed/piggy.S
  AS      arch/x86/boot/compressed/piggy.o
  LD      arch/x86/boot/compressed/vmlinux
  ZOFFSET arch/x86/boot/zoffset.h
  OBJCOPY arch/x86/boot/vmlinux.bin
  AS      arch/x86/boot/header.o
  LD      arch/x86/boot/setup.elf
  OBJCOPY arch/x86/boot/setup.bin
  BUILD   arch/x86/boot/bzImage
Kernel: arch/x86/boot/bzImage is ready  (#9)
make[1]: Leaving directory '/home/saini/build/kernel'
make: Leaving directory '/home/saini/dev/kernel/linux'
Elapsed: 252.20s

## Step 4: Verify
-rwxr-xr-x 1 saini saini 440M Jun 30 11:14 /home/saini/build/kernel/vmlinux
Linux version  (saini@DEVIL) (gcc (Ubuntu 13.3.0-6ubuntu2~24.04.1) 13.3.0, GNU ld (GNU Binutils for Ubuntu) 2.42) # SMP PREEMPT_DYNAMIC 
Linux version  (saini@DEVIL) (gcc (Ubuntu 13.3.0-6ubuntu2~24.04.1) 13.3.0, GNU ld (GNU Binutils for Ubuntu) 2.42) #9 SMP PREEMPT_DYNAMIC Tue Jun 30 11:12:59 UTC 2026
6RMNG-OS: kernel identity active - foundation layer ready
kernel.release file: n/a
patched source:  init/main.c | 1 +
 1 file changed, 1 insertion(+)
