# Rust AArch32 Kernel

Rust 裸机 AArch32/ARMv7-A 内核，当前目标平台是 QEMU `virt` + `cortex-a15`。项目定位不是用户态调度器 demo，而是逐步发展为接近早期 Linux 设计的单体内核：真实异常入口、IRQ/SVC 上下文切换、MMU、用户态、VFS、initramfs、驱动和系统调用都在 CPU 模型上运行。

## 项目定位

当前内核优先保证这些基础机制真实可用：

1. 启动、异常向量、IRQ、SVC、fault handler 运行在 AArch32 特权模型上。
2. 调度器以 TCB、ready queue、sleep queue、wait queue 和 reaper 管理任务生命周期。
3. 默认启用 ARMv7-A short-descriptor MMU，使用 4 KiB 页映射内核、用户页和 MMIO。
4. 用户任务运行在 User mode，通过 SVC 调用内核服务。
5. 文件访问通过 VFS 和 fd 表，不再把用户程序、rootfs 或驱动路径写死在调度器里。
6. 平台设备优先来自 FDT，QEMU virt 固定地址只作为 fallback。

长期目标是做一个结构清晰、可调试、可演进的 Rust OS kernel。设计会参考 Linux 早期模型，但不会短期承诺完整 Linux ABI、完整 POSIX 或 ext4 全功能兼容。

## 当前能力

- 架构：AArch32 / ARMv7-A
- 机器：QEMU `virt`
- CPU：`cortex-a15`
- 串口：PL011，支持输出、RX/timeout 中断、canonical line discipline
- 中断：GICv2
- 时钟：ARM Generic Physical Timer
- 平台发现：FDT parser + QEMU virt fallback
- 调度：优先级 ready queue + Round-Robin 时间片 + 主动 `yield()` + 抢占 tick
- 任务状态：`Ready`、`Running`、`Sleeping`、`Blocked`、`Zombie`
- 等待机制：sleep queue、wait queue、console wait channel、父子进程 wait channel
- 资源回收：reaper 回收 kernel stack、用户地址空间、fd 表和 TCB slot
- 内存：buddy page allocator、物理页元数据、地址类型封装
- MMU：4 KiB small page、用户/内核权限、device memory MMIO、ASID 切换骨架
- 用户态：ELF32 ARM loader、独立用户地址空间、用户栈、SVC syscall
- syscall：`yield`、`sleep`、`block`、`wake`、`exit`、`read/write`、`readv/writev`、`open/close`、`wait/spawn/exec`、`brk/mmap/munmap/mprotect`、`fcntl/ioctl/stat/fstat/newfstatat/access`、`getpid/getppid/uname/time`
- 文件系统：VFS、只读内嵌 initrd、可写 ramfs、外部 cpio/tar initramfs 解包
- 驱动：UART、GIC、timer、VirtIO MMIO transport probe、virtio-blk probe 骨架、virtio-net 绑定骨架
- 网络：net interface 表、Ethernet/ARP/IPv4/ICMP/UDP 解析、ARP/ICMP 回复生成、接口统计和 TX/RX 队列
- 配置：Linux 风格 `Kconfig` / `.config` / `menuconfig`

## 文件系统兼容策略

后续文件系统需要做兼容，但要分层做，不应该一开始追求“完整兼容 Linux rootfs 可以直接运行”。

### 兼容目标

优先兼容的是 Linux/POSIX 的内核对象模型和语义边界：

1. fd 是进程资源，指向引用计数 `File` 对象。
2. `File` 保存 offset、flags、mode 和 inode/dentry 引用。
3. VFS 逐步形成 `super_block`、`inode`、`dentry`、`file`、`mount` 分层。
4. 路径解析支持 `.`、`..`、绝对路径、相对路径、symlink、mount crossing。
5. syscall 返回 Linux 风格负 errno。
6. `open/read/write/close/lseek/getdents/mkdir/unlink/rename/stat` 的行为尽量贴近 Linux。
7. 块设备接入 block layer，再由 buffer cache/page cache 服务文件系统。

### 当前 rootfs 策略

当前使用的是 Linux initramfs 思路：

1. 外部 `newc` cpio 或 `ustar` tar 只是启动输入格式。
2. QEMU loader 把归一化后的 raw archive 放入 RAM。
3. 内核启动后把 archive 解包到可写 ramfs。
4. 运行期读写都发生在 VFS/ramfs inode 上。
5. 内嵌 initrd 作为 fallback，提供最小 `/bin/init` ELF。

已支持两个外部 rootfs 单独使用：

```bash
make defconfig DEFCONFIG=configs/qemu_virt_rootfs_cpio_defconfig
make run
```

```bash
make defconfig DEFCONFIG=configs/qemu_virt_rootfs_tar_defconfig
make run
```

默认路径：

```text
/home/tang/rust/rootfs.cpio.uboot
/home/tang/rust/rootfs.tar.gz
```

`scripts/prepare_initrd.py` 在宿主侧处理 uImage/gzip，把输入转换成内核可直接解析的 raw archive。内核不在早期启动路径里做 gzip 解压。

### 不作为近期目标

这些不建议作为短期目标：

1. 直接运行 Linux 发行版 rootfs。
2. 直接运行动态链接 busybox。
3. 完整 Linux syscall ABI。
4. 完整 ext4、权限、namespace、signal、procfs、sysfs。

原因是 Linux 用户态不是只有文件格式兼容，还依赖 ELF interpreter、动态链接器、`mmap/brk/fork/execve/wait4/ioctl/fcntl/stat`、signals、credentials、设备节点和大量 proc/sys 约定。当前更合理的路线是先稳定本内核 ABI 用户程序，再逐步扩展到能运行静态 busybox，最后再考虑更高等级 Linux 用户态兼容。

### 文件系统路线

建议路线：

1. 完善 ramfs：目录枚举、mkdir、unlink、rename、symlink、stat、lseek、truncate。
2. 完善 VFS：`File` 引用计数、共享 offset、close-on-exec、dup、dentry cache、mount table。
3. 接入 virtio-blk：vring、DMA buffer、request queue、完成中断、阻塞等待。
4. 建立 block layer：bio/request、buffer cache、page cache、脏页和 writeback。
5. 增加 FAT32 或 ext2：先只读，再读写。
6. 增加 ELF loader 的 argv/envp/auxv、interpreter、静态用户程序工具链。
7. 兼容静态 busybox 所需 syscall 子集。

## 构建和配置

安装 Rust 目标：

```bash
rustup target add armv7a-none-eabi
```

安装 QEMU：

```bash
sudo apt install qemu-system-arm
```

生成默认配置并构建：

```bash
make defconfig
make build
```

进入配置菜单：

```bash
make menuconfig
```

`.config` 是唯一配置真源。`make oldconfig` 会生成：

```text
include/generated/autoconf.mk
include/generated/autoconf.rs
```

Makefile 只保留统一入口，不再用 `run-dtb`、`run-virtio`、`run-no-mmu` 表示功能组合。需要调整功能时使用 `menuconfig` 或 defconfig。

关键配置：

- `CONFIG_MMU`：启用 ARMv7-A MMU；关闭后使用 no-MMU 调试路径。
- `CONFIG_USER`：启用用户态进程。
- `CONFIG_QEMU_FDT_LOADER`：由 QEMU 生成 DTB，并通过 loader 放入 RAM。
- `CONFIG_VIRTIO_MMIO`：枚举 VirtIO MMIO transport。
- `CONFIG_VIRTIO_BLK`：给 QEMU 挂载 virtio-blk 设备。
- `CONFIG_NET`：启用内核网络栈。
- `CONFIG_IPV4`、`CONFIG_NET_ARP`、`CONFIG_NET_ICMP`、`CONFIG_NET_UDP`：启用基础 IPv4 协议模块。
- `CONFIG_VIRTIO_NET`：给 QEMU 挂载 virtio-net 设备。
- `CONFIG_QEMU_NETDEV`：QEMU netdev 后端，例如 `user,id=net0`。
- `CONFIG_FS`：启用 VFS。
- `CONFIG_INITRD`：启用内嵌 initrd。
- `CONFIG_INITRD_EXTERNAL`：启用外部 rootfs。
- `CONFIG_INITRD_EXTERNAL_PATH`：外部 rootfs 路径。
- `CONFIG_BOOT_VERBOSE`：打开详细启动日志。
- `CONFIG_DEMO_KERNEL_TASKS`：打开早期 kernel task demo 输出。

## 运行

默认运行：

```bash
make run
```

退出 QEMU：

```text
Ctrl-a x
```

常用调试命令：

```bash
make readelf
make nm
make objdump
make debug
```

`make debug` 会让 QEMU 使用 `-S -s` 等待 GDB：

```bash
gdb-multiarch target/armv7a-none-eabi/debug/rust_scheduler_demo
target remote :1234
```

预期关键输出类似：

```text
rust aarch32 kernel
fs: initrd=true external=true format=cpio-newc files=191 builtin=1
fs: rootfs probe bin/busybox size=820900
[user-init] readback ramfs-write-ok
user$
```

如果使用 tar rootfs，格式会显示为 `tar-ustar`。当前 rootfs 中的 busybox 可作为文件系统兼容测试样本读取，但不能作为 Linux 动态 PIE 直接执行。

构建本内核 ABI 用户程序：

```bash
make userspace
make userspace-rootfs
```

产物位于 `userspace/build/`，包括 `/bin/init`、`shell`、`ls`、`cat`、`echo` 的 ELF32 ARM 静态用户程序，以及 `userspace/build/rootfs.tar`。如需用它作为外部 rootfs，可在 `make menuconfig` 中把 `CONFIG_INITRD_EXTERNAL_PATH` 改为 `userspace/build/rootfs.tar`。

启用 virtio-net：

```bash
make defconfig DEFCONFIG=configs/qemu_virt_net_defconfig
make run
```

当前网络设备绑定后会在详细启动日志中显示 `eth0`、MAC、IPv4、RX/TX 队列和统计。virtio-net vring DMA 收发还未完成，因此这一步用于验证 netdev 注册和协议栈入口，不等价于已经能从宿主 `ping` 通。

## 源码边界

关键目录：

```text
src/arch/aarch32/     CPU 模式、异常、上下文、MMU、cache、barrier
src/platform/         QEMU virt、FDT、initrd loader 描述
src/drivers/          UART、GIC、timer、VirtIO、设备接口骨架
src/kernel/           调度、任务、进程、syscall、内存、日志、console、等待队列
src/fs/               VFS、ramfs、initrd、archive parser
scripts/              kconfig、initrd 预处理
configs/              defconfig
```

长期边界：

1. `arch/` 只暴露 CPU 能力，不包含调度策略。
2. `platform/` 负责 board/FDT/device discovery，不实现设备协议。
3. `drivers/` 只处理设备寄存器、中断确认和 I/O 提交，不直接持有进程语义。
4. `kernel/` 保存调度、进程、syscall、同步、日志等通用内核机制。
5. `fs/` 只处理文件对象、路径、挂载和具体文件系统，不直接依赖 QEMU 地址。

## 分层路线图

后续按 L0-L8 推进。每层都要有可验证结果，避免只堆接口不闭环。当前代码已经覆盖 L0、L1，并部分进入 L2、L3、L4、L5；下一步应优先把 L2/L3 的生命周期语义补完整，再继续扩展 VFS 和驱动。

### L0：Boot

目标：

1. QEMU/Bootloader 能稳定进入 `_start`。
2. 建立 SVC/IRQ/Abort/Undefined/FIQ 栈。
3. 清零 `.bss`，保存 boot 参数、DTB 地址和 early initrd 范围。
4. 初始化 UART early console，panic 任意阶段都能输出。
5. 安装异常向量，区分 reset、undefined、svc、prefetch abort、data abort、irq、fiq。
6. 初始化 GIC、Generic Timer 和最小 IRQ 分发。
7. 建立 early page allocator、buddy allocator 和早期页表。
8. 打开 MMU/I-cache，保留 identity bootstrap 映射。

结果：

```text
Hello Kernel
```

核心模块：

1. `arch/aarch32/boot.S`：CPU 模式切换、栈设置、`.bss` 清零、跳入 Rust。
2. `arch/aarch32/exception/`：异常入口和 fault 诊断。
3. `arch/aarch32/mmu.rs`：bootstrap 页表、TLB/cache/barrier。
4. `platform/fdt.rs`：DTB 扫描、RAM/MMIO/设备发现。
5. `kernel/memory.rs`：early reserve、buddy 初始化。
6. `drivers/uart.rs`、`drivers/gic.rs`、`drivers/timer.rs`：最小设备 bring-up。

设计边界：

1. `boot.S` 只做 CPU 必需动作，不放调度、内存策略和设备策略。
2. `kernel_main` 之前只允许 early console、early reserve、固定栈。
3. DTB、initrd、kernel image、页表、栈必须在 buddy 初始化前标记 reserved。
4. MMIO 必须用 device memory + XN 映射，普通 RAM 用 normal memory。
5. panic/fault 路径不依赖 heap、调度器或普通锁。

验收标准：

1. `make run` 稳定打印内核 banner、RAM/MMIO、timer 频率。
2. 未打开调度器前触发 panic 仍能输出异常类型和关键寄存器。
3. 打开 MMU 后 UART、GIC、timer 继续工作。
4. `make readelf` 显示入口地址和 linker 段布局符合预期。
5. no-MMU 配置仍能作为 bring-up fallback。

当前状态：

1. QEMU `virt` 可直接通过 `-kernel` 启动 AArch32 ELF。
2. 已初始化 PL011、GICv2、Generic Timer、异常向量。
3. 已启用 ARMv7-A short-descriptor MMU 和 4 KiB 页表。
4. 已有 buddy page allocator、panic/fault 诊断、串口输出。
5. FDT loader、外部 initrd loader、QEMU virt fallback 已接入配置系统。

下一步：

1. 把 kernel 切到高地址运行，保留 identity bootstrap。
2. 解析 FDT `reserved-memory`，让页分配器消费完整 memory map。
3. 把 linker symbols 明确拆成 `.text/.rodata/.data/.bss/.init` 权限域。
4. 增加 boot smoke test，匹配 banner、MMU、timer、panic 基础输出。

### L1：Kernel Core

目标：

1. 建立 `TaskControlBlock`、任务状态、任务统计和内核栈。
2. 实现抢占式 scheduler、ready queue、时间片和上下文切换。
3. Timer IRQ 能进入统一调度路径。
4. SVC `yield()` 和 timer tick 复用相同 reschedule 逻辑。
5. 建立 `SleepQueue`、`WaitQueue`、`Wakeup` 和 reaper。
6. 建立 `SpinLock<T>`、`Mutex`、`CondVar` 的内核同步基础。
7. 明确 IRQ 上下文和线程上下文的锁规则。

结果：

```text
Task A
Task B
Task C
```

核心结构：

```text
TaskControlBlock
  pid/tid
  name
  state
  priority/static_prio/dynamic_prio
  time_slice/remaining_ticks
  kernel_stack/user_stack
  context/trap_frame_sp
  wait_channel
  stats

RunQueue
  ready queues by priority
  current task
  idle task

WaitQueue
  sleepers
  blocked readers/writers
  parent waiters
```

设计边界：

1. 调度器不直接访问 UART、VirtIO 或文件系统；设备只能通过 wait queue 唤醒任务。
2. IRQ handler 不睡眠，不获取可能阻塞的锁。
3. `SpinLock<T>` 可用于 IRQ 上下文，`Mutex` 只能在线程上下文使用。
4. `yield/sleep/block/exit` 必须走同一状态迁移函数，避免状态分叉。
5. idle task 永远 runnable，保证 syscall/IRQ 后有可切换目标。

验收标准：

1. Timer IRQ 能打断 CPU-bound task，串口输出出现 A/B/C 交替。
2. `yield()` 主动让出后当前任务回到 ready queue 尾部。
3. `sleep(ticks)` 到期前不运行，到期后重新入队。
4. `block(channel)` 后不被调度，`wake(channel)` 后恢复运行。
5. `exit()` 后任务进入 `Zombie`，reaper 回收栈和 TCB。
6. IRQ 中不直接大量打印调度日志。

当前状态：

1. 已有 TCB、优先级 ready queue、sleep queue、wait queue、reaper。
2. IRQ/SVC 已保存 trap frame 并切换 kernel stack。
3. 已支持 `yield/sleep/block/wake/exit`。
4. 调度日志已延迟到任务上下文输出，IRQ 中避免大量串口打印。

下一步：

1. 引入 `SpinLock<T>` 和 `IrqSafeSpinLock<T>`。
2. 实现 `Mutex`、`CondVar`、可中断/不可中断 wait queue。
3. 把固定优先级调度升级为 MLFQ 或 Linux O(1) 风格 runqueue。
4. 增加 scheduler trace 和 host-testable queue 单元测试。
5. 为 panic 输出增加 runqueue 摘要、当前锁状态和最近调度事件。

### L2：Memory

目标：

1. Buddy Allocator 管理物理页。
2. Slab Allocator 管理小对象。
3. Kernel Heap 支持 `alloc`，但保持 `no_std`。
4. Page Table 提供统一 map/unmap/protect/query API。
5. Address Space 对应早期 `mm_struct`。
6. VMA/region 管理用户映射。
7. Page fault 支持 lazy allocation 和用户异常处理。

支持：

1. `mmap`。
2. 按需映射。
3. 用户空间页表。
4. 用户指针检查。
5. Page fault 驱动的 lazy allocation。
6. `brk`。
7. COW 前置页引用计数。

核心结构：

```text
PageFrame
  state
  order
  refcount
  flags

BuddyAllocator
  free_area[order]
  split/merge
  reserve/free

SlabCache
  object_size
  partial/full/free slabs

AddressSpace
  pgd/L1 root
  ASID
  VMA list
  owned pages
  owned page tables

VmArea
  start/end
  prot
  flags
  backing file/anon
```

地址空间规划：

```text
0x00000000..0x7fffffff  用户空间
0x80000000..0xbfffffff  内核线性映射、内核堆
0xc0000000..0xdfffffff  设备 MMIO 映射
0xe0000000..0xffffffff  固定映射、向量、调试区
```

设计边界：

1. 物理页分配器只处理页帧，不理解进程和文件。
2. Slab 只从 buddy 申请页，不直接操作平台 memory map。
3. AddressSpace 只持有页表和 VMA，不直接持有调度状态。
4. 用户页必须只映射在用户地址范围，内核页不得带 user permission。
5. `copy_from_user/copy_to_user` 先查权限，再执行 fault-safe 拷贝。
6. MMIO 映射不进入普通 heap，不参与 COW，不允许用户态直接访问。

验收标准：

1. buddy 分配/释放同 order 页后能正确合并。
2. slab 能反复分配释放 TCB/File/Inode 等小对象，无页泄漏。
3. 用户进程拥有独立页表，调度切换时 TTBR/ASID 正确切换。
4. 用户写只读页触发 fault，内核不会 silent corrupt。
5. 坏用户指针 syscall 返回 `-EFAULT`，不会导致 kernel fatal。
6. `mmap/brk` 能创建 VMA，首次访问再分配物理页。

当前状态：

1. 已有 buddy allocator、页帧元数据、地址类型封装。
2. 已有 L1/L2 页表、用户页权限、MMIO device mapping。
3. 用户进程已有独立地址空间和 ASID 切换骨架。
4. `copy_from_user/copy_to_user` 已做页表 preflight。

下一步：

1. 增加 slab allocator，服务 TCB、File、Inode、Dentry、VMA 等小对象。
2. 建立 kernel heap allocator，替代固定容量数组。
3. 引入 VMA/region 管理，为 `mmap/brk` 和 page fault 做准备。
4. 实现 fault-safe user copy，坏用户指针返回 `-EFAULT`。
5. 用户态 page fault 改成杀死当前进程或按 VMA 补页，内核态 fault 才 panic。
6. 为 buddy/slab/VMA 增加 host 单元测试和泄漏统计。

### L3：Process

目标：

1. `fork()`。
2. `exec()`。
3. `exit()`。
4. `wait()`。
5. `clone()`。

支持：

1. Process。
2. Thread。
3. 父子关系。
4. 退出码。
5. 资源引用计数。
6. 文件表继承。
7. 地址空间复制和后续 COW。
8. 用户异常转进程退出。

核心结构：

```text
task_struct
  scheduler state
  kernel stack
  trap frame
  signal/exit state

mm_struct
  AddressSpace
  VMA list
  refcount

files_struct
  fd table
  close-on-exec bitmap
  refcount

file
  inode/dentry
  offset
  flags
  refcount

process
  pid
  parent
  children
  thread group
  exit code
  zombie state
```

系统调用语义：

1. `fork()`：复制当前进程上下文，父进程返回 child pid，子进程返回 0。
2. `exec(path, argv, envp)`：替换当前 `mm_struct`，保留 pid，按 close-on-exec 关闭 fd。
3. `exit(code)`：关闭资源，进入 zombie，唤醒父进程。
4. `waitpid(pid, status, options)`：收割 zombie 子进程，必要时阻塞。
5. `clone(flags, stack)`：按 flags 共享或复制 `mm/files/sighand`。

设计边界：

1. `task_struct` 是调度实体，`process` 是资源和父子关系容器。
2. 内核线程没有用户 `mm_struct`，用户线程必须有有效 `mm_struct`。
3. `exec()` 失败不得破坏旧地址空间。
4. `exit()` 不能直接释放当前正在使用的 kernel stack，由 reaper 延迟回收。
5. fd、mm、file、inode 都需要引用计数，避免 fork/exit/wait 交错时悬垂。
6. 用户态 fault 默认变成当前进程异常退出，内核态 fault 才 panic。

验收标准：

1. `fork()` 后父子进程可独立运行，寄存器返回值符合约定。
2. `exec()` 后 pid 不变，用户 PC/SP/地址空间替换成功。
3. `waitpid()` 能拿到子进程 exit code，并释放 zombie。
4. `clone()` 可创建共享地址空间的线程，调度器按独立 task 调度。
5. 父进程退出时，孤儿进程能被 init/reaper 接管。
6. 连续 spawn/exit/wait 不泄漏 kernel stack、user pages、fd、process slot。

当前状态：

1. 已有 `ProcessId`、`ThreadId`、`ProcessTable`、用户地址空间和最小 fd 表。
2. 已支持 `spawn/exec/wait/exit` 的最小语义。
3. reaper 能回收 task、mm、files，并保留 zombie 退出状态给父进程收割。
4. 用户任务已经运行在 User mode，通过 SVC 进入内核。

下一步：

1. 把 `FileTable` 改成引用计数 `files_struct`。
2. 实现 `fork()`，先完整复制地址空间，再升级为 COW。
3. 实现 `clone()`，支持线程组共享 `mm/files`。
4. 补 `waitpid` flags、进程组、可中断睡眠和用户异常退出状态。
5. 为 `exec()` 补 close-on-exec、argv/envp/auxv 和用户栈初始布局。
6. 增加 init/reparent 规则，避免父进程退出后 zombie 无人收割。

### L4：VFS

统一接口：

1. `File`。
2. `Directory`。
3. `Inode`。
4. `Dentry`。
5. `Mount`。
6. `SuperBlock`。

支持：

1. RamFS。
2. DevFS。
3. ProcFS。
4. TmpFS。
5. FAT32。
6. EXT2。
7. EXT4。

设计要求：

1. 上层 syscall 只依赖 VFS，不直接依赖具体文件系统。
2. 新增文件系统不修改 `read/write/open/close/exec` 上层路径。
3. 目录、普通文件、设备文件、管道和 socket 都统一成 file operations。

当前状态：

1. 已有 VFS、ramfs、只读内嵌 initrd、archive parser。
2. 外部 cpio/tar rootfs 已可解包到可写 ramfs。
3. `open/read/write/close/spawn/exec` 已通过 VFS 访问 regular file。

下一步：

1. 引入 `File` 引用计数、共享 offset、flags、close-on-exec。
2. 增加 `Dentry` cache、`Mount` table、path resolver。
3. 支持 `getdents/stat/lseek/truncate/mkdir/unlink/rename/symlink`。
4. 增加 DevFS：`/dev/console`、`/dev/null`、`/dev/zero`、块设备节点。
5. 增加 ProcFS：`/proc/meminfo`、`/proc/mounts`、`/proc/<pid>`。
6. 有 block layer 后接 FAT32 或 EXT2，先只读再读写。

### L5：Driver Framework

驱动统一管理：

1. UART。
2. GPIO。
3. RTC。
4. Timer。
5. Block。
6. Input。
7. GPU。
8. Network。

支持：

1. VirtIO。
2. PCI。
3. USB。

当前状态：

1. UART/GIC/timer 已能从 FDT 或 fallback 初始化。
2. UART RX/timeout IRQ 已接入 console line discipline。
3. VirtIO MMIO transport 和 virtio-blk probe 骨架已存在。

下一步：

1. 建立 device manager，统一 `probe/bind/remove/irq` 生命周期。
2. 完成 virtio-blk vring、DMA buffer、request queue、完成中断。
3. 增加 block device trait，并接入 VFS/DevFS。
4. 扩展 FDT `ranges`、`interrupt-parent`、`interrupt-map`。
5. 后续再做 PCI/USB；在 QEMU ARM virt 上优先 VirtIO。

### L6：IPC

实现：

1. Pipe。
2. Socket。
3. Shared Memory。
4. Semaphore。
5. Mutex。
6. CondVar。
7. Event。

目标：

1. 进程之间能交换字节流、共享页和同步状态。
2. 阻塞 IPC 全部走 wait queue。
3. fd 表能同时管理普通文件、设备、pipe 和 socket。

路线：

1. 先实现 pipe：ring buffer + reader/writer wait queue。
2. 再实现匿名 shared memory：页引用计数 + 多进程映射。
3. 内核同步对象先支持 futex-like wait/wake 子集。
4. socket API 等网络栈 L7 可用后再完整接入。

### L7：Network

协议栈：

1. Ethernet。
2. ARP。
3. IPv4。
4. UDP。
5. TCP。
6. DNS。
7. DHCP。

接口：

1. BSD Socket。

最终能够：

1. `ping`。
2. `wget`。
3. `ssh`。

当前状态：

1. 已有固定容量 `net_device` 风格接口表，记录 `name/MAC/IPv4/MTU/state/stats`。
2. 已有 RX/TX packet queue，后续 virtio-net DMA 完成后可直接对接收发路径。
3. 已实现 Ethernet、ARP、IPv4、ICMP echo、UDP parser/writer。
4. 网络输入入口 `handle_rx()` 可分类以太网帧，ARP request 命中本机 IPv4 时生成 ARP reply，ICMP echo request 命中本机 IPv4 时生成 echo reply。
5. virtio-net probe/bind 已接入 VirtIO MMIO、device manager、GIC IRQ enable 和网络接口注册。
6. `configs/qemu_virt_net_defconfig` 可显式启用 QEMU virtio-net。

路线：

1. 完成 virtio-net vring DMA：描述符、avail/used ring、RX buffer refill、TX complete reclaim。
2. 把驱动 RX 中断接到 `net::handle_rx()`，把 TX queue 接到 virtqueue kick。
3. 增加 ARP cache、路由表和邻居解析，避免每次发送都广播。
4. 增加 UDP socket：端口绑定、接收队列、阻塞 `read/write`、`poll` 前置。
5. 实现 DHCP client 和 DNS resolver。
6. 实现 TCP 最小状态机，支持 connect/read/write/close。
7. 通过 socket fd 接入 VFS file operations，补 `socket/bind/connect/listen/accept/send/recv`。

### L8：Userspace

支持：

1. ELF。
2. 动态链接可后续。
3. libc。
4. Rust std 可选。

应用：

1. Shell。
2. `ls`。
3. `cat`。
4. `echo`。
5. `top`。
6. `ps`。

当前状态：

1. 已能加载最小 ELF32 ARM 用户程序。
2. 已有 builtin user shell 走 `read(0)` 和 `write(1)`。
3. ELF loader 已支持 `ET_EXEC`、`ET_DYN`、`PT_LOAD`、`PT_PHDR`、`PT_INTERP`。
4. 动态 ELF 带 `PT_INTERP` 时，内核会从 VFS 查找解释器并映射到独立基址，入口切换到解释器。
5. 用户初始栈已按 Linux 风格放置 `argc/argv/envp/auxv`，包含 `AT_PHDR`、`AT_PHENT`、`AT_PHNUM`、`AT_PAGESZ`、`AT_BASE`、`AT_ENTRY`。
6. syscall 已补 `readv/writev`、`fcntl`、`ioctl`、`access`、`fstat`、`newfstatat`、`getpid/getppid`、`uname`、`gettimeofday/clock_gettime`。
7. 地址空间已补 `brk`、匿名 `mmap`、fd-backed `mmap`、`MAP_FIXED`、部分区间 `munmap`、实际 PTE 权限更新的 `mprotect`。
8. 已补 `/proc/self/exe`、`/dev/urandom`、`/tmp`、基础 credentials 和 umask。
9. 已建立独立 Rust no_std userspace support crate 和用户程序构建链，可生成 `/bin/init`、shell、`ls/cat/echo` 的本内核 ABI ELF。
10. 外部 rootfs 中的 Linux busybox 目前仍主要作为文件系统兼容测试样本；运行动态 Linux 用户态还需要继续补 Linux syscall ABI、signal、完整 VFS 和动态链接器约定。

下一步：

1. 增加 Linux ARM EABI syscall 入口兼容层，支持 `r7=nr, r0..r6=args`，同时保留本内核 ABI。
2. 补 `execve(argv/envp)`、`wait4`、`dup/dup2/dup3`、`pipe2`、`getcwd/chdir`、`readlink`、`poll/select`、`rt_sigaction/rt_sigprocmask` 最小语义。
3. 把 VFS `Stat` 升级为 Linux `stat/statx` 兼容布局，补真实 inode mode、nlink、uid/gid、mtime/ctime。
4. 建立共享 fd offset、file refcount、close-on-exec bitmap，靠近 Linux `files_struct + file` 模型。
5. 支持静态 busybox 的最小 syscall 子集，并加入 QEMU smoke test 自动运行 `/bin/busybox --help`。
6. 后续再实现 ELF 动态链接器需要的 TLS、auxv 完整字段、`set_tid_address`、`futex`、robust list、更多 `/proc`/`/sys` 约定。
7. 最后推进动态 busybox/musl，再考虑 glibc。

## 已知限制

1. 外部 rootfs 里的 Linux busybox 当前只能作为文件读写样本，不能直接运行；本轮新增的是本内核 ABI 用户程序构建链。
2. 没有完整 Linux syscall ABI、signal、`fork/execve/wait4` 完整语义和动态链接器运行环境。
3. VFS 还没有完整 dentry cache、mount table、symlink 解析和引用计数 file 对象。
4. virtio-blk 目前是 probe/初始化骨架，还没有块 I/O。
5. 用户指针拷贝是页表 preflight，还不是 Linux exception table fault fixup。
6. D-cache 暂时关闭。
7. 内核仍依赖 identity mapping 运行，高地址内核是下一阶段目标。
