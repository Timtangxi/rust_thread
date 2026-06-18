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
- syscall：`yield`、`sleep`、`block`、`wake`、`exit`、`write`、`read`、`open`、`close`、`wait`、`spawn`、`exec`
- 文件系统：VFS、只读内嵌 initrd、可写 ramfs、外部 cpio/tar initramfs 解包
- 驱动：UART、GIC、timer、VirtIO MMIO transport probe、virtio-blk probe 骨架
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

## 近期路线

优先完成这些会显著提升内核完整性：

1. 用户异常处理：User mode data/prefetch abort 杀死当前进程，Kernel mode fault 才 panic。
2. fault-safe user copy：实现 exception table/fixup，坏用户指针返回 `-EFAULT`。
3. 用户程序构建链：把 `/bin/init` 从内嵌 byte array 改为独立用户 ELF 产物。
4. ELF loader：补 argv/envp/auxv、BSS 跨页校验、段权限、interpreter 识别。
5. fd 生命周期：引用计数 `File`、dup、close-on-exec、共享 offset、flags。
6. VFS：dentry cache、mount table、path resolver、`getdents/stat/lseek/mkdir/unlink/rename/symlink`。
7. virtio-blk：vring、DMA buffer、request/complete path、IRQ 唤醒等待任务。
8. block layer：统一块请求、buffer cache、page cache。
9. ext2 或 FAT32：先只读，再可写。
10. QEMU smoke test：自动等待串口关键文本并失败退出。

## 长线系统设计

### 执行模型

1. bootstrap 阶段关闭 IRQ，完成栈、`.bss`、串口、异常向量、页分配器、页表、GIC、timer 初始化。
2. 创建 idle、init、reaper、必要内核线程。
3. 打开 IRQ，进入统一调度路径。
4. 用户任务运行在 User mode，通过 SVC 进入内核。
5. IRQ handler 只做短路径处理：确认设备、搬运少量状态、唤醒 wait queue。
6. 耗时工作放到内核线程或 bottom half。
7. panic 路径避免普通锁，只做最小诊断输出。

### 进程模型

目标接近早期 Linux 的拆分：

```text
task_struct  -> 调度实体、上下文、kernel stack、状态、统计
mm_struct    -> 页表、ASID、用户段、用户栈、VMA/region
files_struct -> fd table、File 引用、close-on-exec 位图
file         -> offset、flags、mode、inode/dentry 引用
inode        -> 文件元数据和后端操作
dentry       -> 路径名缓存和父子关系
```

推荐演进顺序：

1. 维持一个进程一个线程，先把 `mm/files` 生命周期做准。
2. 实现 `spawn/exec/wait/exit` 的完整资源规则。
3. 增加 `fork()` 的完整地址空间复制。
4. 把 `fork()` 升级为 COW。
5. 引入线程组，共享 `mm/files` 引用计数。
6. 后续再考虑 signal、process group、session、credential。

### 内存模型

长期虚拟地址规划：

```text
0x00000000..0x7fffffff  用户空间
0x80000000..0xbfffffff  内核线性映射、内核堆
0xc0000000..0xdfffffff  设备 MMIO 映射
0xe0000000..0xffffffff  固定映射、向量、调试区
```

路线：

1. 当前保留 identity mapping 作为稳定启动路径。
2. 建立高地址内核链接和运行路径。
3. 用户页只存在于低地址用户空间。
4. 内核页对用户不可访问。
5. MMIO 始终映射为 device memory。
6. ASID 增加 generation 和回收策略。
7. 完善 TLB/cache 维护后再打开 D-cache。
8. 页分配器消费完整 FDT memory map 和 reserved-memory。

### 驱动模型

目标：

1. FDT 负责设备发现，驱动负责 bind/probe。
2. device manager 维护设备、driver、major/minor 或设备路径。
3. IRQ handler 使用明确的 ACK/EOI 顺序和内存屏障。
4. 阻塞 I/O 统一通过 wait queue。
5. VirtIO 成为 QEMU 下主要设备族，优先 block，再 console/gpu/net。

建议接口逐步收敛到：

```rust
trait Driver {
    fn name(&self) -> &'static str;
    fn probe(&self, device: &PlatformDevice) -> Result<DeviceId, Error>;
}

trait IrqHandler {
    fn handle_irq(&self, irq: u32);
}

trait BlockDevice {
    fn submit(&self, request: BlockRequest) -> Result<(), Error>;
}
```

早期可以继续用模块函数推进；设备数量增加后再固化 trait。

### 文件系统和程序加载

长期形态：

1. initramfs/ramfs 保证无块设备时能启动和写入。
2. virtio-blk 提供持久块设备。
3. block layer 统一请求队列和完成路径。
4. page cache 统一普通文件缓存。
5. VFS 通过 `super_block/inode/dentry/file/mount` 抽象后端。
6. ELF loader 从 VFS 读取文件，创建用户地址空间、用户栈、argv/envp/auxv。
7. 本内核 ABI 用户程序先稳定，再扩展 Linux 静态程序兼容。

Linux 兼容分级：

```text
L0  本内核 ABI 用户程序，支持基础 syscall 和 VFS
L1  静态 ELF 用户程序，支持 argv/envp/auxv、brk、mmap 子集
L2  静态 busybox 所需 syscall 子集
L3  动态 ELF interpreter、更多 ioctl/fcntl/stat/signal
L4  更完整 POSIX/Linux 用户态兼容
```

当前处于 L0，正在向 L1 准备。

### 同步和 Rust 安全边界

路线：

1. 引入 `SpinLock<T>`，单核阶段用关 IRQ 保护临界区。
2. 区分 IRQ-safe lock 和线程上下文 lock。
3. 引入 `OnceCell` 或初始化状态机替代裸 `static mut`。
4. 把页表、MMIO、上下文切换、用户拷贝等 `unsafe` 封装在底层模块。
5. 上层调度器、VFS、进程管理尽量暴露安全 API。
6. 为关键 `unsafe` 写清楚不变量：对齐、生命周期、CPU 模式、页表权限、MMIO 顺序。

### 测试和调试

路线：

1. 保留 `make readelf/nm/objdump/debug`。
2. 增加 QEMU smoke test，串口匹配 `fs:`、`ramfs-write-ok`、`user$`。
3. 将 ready queue、sleep queue、wait queue、buddy allocator、archive parser 抽成 host-testable 模块。
4. 增加异常测试配置，覆盖 undefined、data abort、prefetch abort、bad syscall。
5. CI 至少覆盖 `make defconfig && make build`、host tests、QEMU smoke。

## 已知限制

1. 外部 rootfs 里的 Linux busybox 当前只能作为文件读写样本，不能直接运行。
2. 没有完整 Linux syscall ABI、动态链接器、signal、`mmap/brk/fork/execve` 完整语义。
3. VFS 还没有完整 dentry cache、mount table、symlink、目录枚举和引用计数 file 对象。
4. virtio-blk 目前是 probe/初始化骨架，还没有块 I/O。
5. 用户指针拷贝是页表 preflight，还不是 Linux exception table fault fixup。
6. D-cache 暂时关闭。
7. 内核仍依赖 identity mapping 运行，高地址内核是下一阶段目标。
