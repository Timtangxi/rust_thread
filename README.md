# Rust AArch32 Round-Robin Kernel

这是一个真正运行在 CPU/QEMU 上的 Rust 裸机调度器，不是用户态 `std` demo。

当前目标：

- 架构：AArch32 / ARMv7-A
- 机器：QEMU `virt`
- CPU：`cortex-a15`
- 串口：PL011 `0x09000000`
- 中断控制器：GICv2
- 时钟源：ARM Generic Physical Timer，IRQ 30
- 调度策略：TCB + 优先级 ready queue 的抢占式/主动让出 Round-Robin
- 系统调用：SVC `yield`、`sleep`、`block`、`exit`

## 代码结构

- `src/main.rs`：内核入口、任务入口和 IRQ 分发。
- `src/arch/mod.rs`：体系结构相关模块入口。
- `src/arch/aarch32/boot.S`：启动代码、异常向量、IRQ 保存/恢复路径。
- `src/arch/aarch32/switch.S`：第一次进入任务时的上下文恢复。
- `src/arch/aarch32/context.rs`：AArch32 任务栈帧布局。
- `src/arch/aarch32/cpu.rs`：异常向量表安装等 CPU 操作。
- `src/kernel/mod.rs`：内核核心模块入口。
- `src/kernel/task.rs`：`TaskControlBlock`、任务状态、任务统计和任务栈。
- `src/kernel/queue.rs`：固定容量优先级 ready queue。
- `src/kernel/scheduler.rs`：Round-Robin 调度器、sleep/block/wake/exit 状态切换和延迟日志。
- `src/kernel/syscall.rs`：SVC syscall number 和任务侧调用封装。
- `src/kernel/memory.rs`：从 `__kernel_end` 开始的早期页分配器。
- `src/drivers/mod.rs`：驱动模块入口。
- `src/drivers/gic.rs`：QEMU virt 上的 GICv2 初始化。
- `src/drivers/timer.rs`：Generic Timer 周期 tick。
- `src/drivers/uart.rs`：PL011 串口输出。
- `src/print.rs`：`print!` / `println!` 串口宏。
- `linker.ld`：内核链接地址 `0x40000000`。

IRQ 和 SVC 发生时，汇编入口会保存当前任务的通用寄存器、异常返回 PC 和 SPSR；Rust 调度器只切换保存后的任务栈指针，然后汇编用 `rfe` 返回到下一个任务。

## 环境准备

安装 Rust 目标：

```bash
rustup target add armv7a-none-eabi
```

Ubuntu/Debian 安装 QEMU：

```bash
sudo apt update
sudo apt install qemu-system-arm
```

Arch Linux：

```bash
sudo pacman -S qemu-system-arm
```

Fedora：

```bash
sudo dnf install qemu-system-arm
```

## 构建

```bash
cargo build
```

生成的内核 ELF：

```text
target/armv7a-none-eabi/debug/rust_scheduler_demo
```

也可以检查入口地址：

```bash
llvm-readelf -h target/armv7a-none-eabi/debug/rust_scheduler_demo
```

应看到：

```text
Machine: ARM
Entry point address: 0x40000000
```

## 在 QEMU 运行

```bash
qemu-system-arm \
  -M virt,gic-version=2,virtualization=off,secure=off \
  -cpu cortex-a15 \
  -m 128M \
  -nographic \
  -serial mon:stdio \
  -kernel target/armv7a-none-eabi/debug/rust_scheduler_demo
```

或直接：

```bash
make run
```

退出 QEMU：

```text
Ctrl-a x
```

预期能看到类似输出：

```text
rust aarch32 round-robin kernel
machine: qemu virt / cortex-a15 / armv7-a
memory: page allocator next=0x40020000 allocated_pages=0
timer: generic physical timer at 62500000 Hz
scheduler: states empty ready running sleeping blocked zombie
scheduler: task table (tasks=5, ready=5, ready_empty=false)
  id=0 pid=1 name=idle state=ready prio=0 slice=1 remain=1 wake=0 wait=0 stack=0x40020000+4p runtime=0 scheduled=0
  id=1 pid=2 name=init state=ready prio=1 slice=2 remain=2 wake=0 wait=0 stack=0x40024000+4p runtime=0 scheduled=0
  id=2 pid=3 name=shell state=ready prio=0 slice=1 remain=1 wake=0 wait=0 stack=0x40028000+4p runtime=0 scheduled=0
  id=3 pid=4 name=worker state=ready prio=1 slice=1 remain=1 wake=0 wait=0 stack=0x4002c000+4p runtime=0 scheduled=0
  id=4 pid=5 name=waker state=ready prio=2 slice=1 remain=1 wake=0 wait=0 stack=0x40030000+4p runtime=0 scheduled=0
scheduler: start task waker (pid=5, state=running, prio=2, slice=1 tick)
[waker] heartbeat 0
sleep 0000: waker -> init (ready=2, switches=1)
[init] tick work item 0
yield 0000: init -> worker (ready=2, switches=2)
[worker] background pass 0
tick 0001: worker -> init (ready=1, switches=3)
[init] tick work item 1
yield 0001: init -> shell (ready=1, switches=4)
[shell] prompt refresh 0
sleep 0001: shell -> idle (ready=0, switches=5)
```

## 后续发展路线

这个项目当前处在“最小内核骨架”阶段：已经能在 QEMU 的 ARMv7-A CPU 模型上启动，安装异常向量，初始化串口、GIC 和 Generic Timer，并通过 IRQ/SVC 进行真实上下文切换。接下来的目标不是继续堆 demo，而是逐步把它整理成具备进程、内存、驱动和文件系统边界的 Rust OS kernel。

### 当前基线

已经具备：

1. 裸机启动：`_start` 设置 SVC/IRQ 栈，清零 `.bss`，进入 Rust `kernel_main`。
2. 异常向量：已安装 IRQ、SVC、Undefined、Prefetch Abort、Data Abort、FIQ 等入口。
3. 上下文切换：IRQ 和 SVC 都保存通用寄存器、异常返回 PC、SPSR，并通过 `rfe` 返回到调度后的任务。
4. 调度器：TCB、优先级 ready queue、时间片、运行统计、主动让出和抢占式调度。
5. 任务状态：`Ready`、`Running`、`Sleeping`、`Blocked`、`Zombie`。
6. 系统调用：通过 SVC 实现 `yield`、`sleep`、`block`、`exit`。
7. 内核栈：每个任务有独立内核栈，栈内存从早期页分配器分配。
8. 基础驱动：PL011 串口、GICv2、ARM Generic Physical Timer。
9. 日志策略：调度事件进入 ring buffer，由任务上下文 flush，避免在 IRQ 中直接大量串口输出。

当前限制：

1. 任务仍运行在 SVC 特权模式，还没有真正用户态。
2. 没有 MMU，所有代码和任务共享同一物理地址空间。
3. 早期页分配器只能递增分配，不能释放。
4. `sleep` 当前仍扫描任务表，`block/wake` 还是简单 channel，不是完整等待队列。
5. `Zombie` 任务不会继续运行，但资源还没有被 reaper 回收。
6. 设备地址仍是 QEMU virt 固定地址，没有解析设备树。

### 阶段 1：调度器收尾

目标是把当前调度器从“可运行”推进到“可长期维护”。

建议任务：

1. 增加 reaper 内核任务，集中回收 `Zombie` 任务的 TCB、内核栈和后续地址空间资源。
2. 把 `sleep` 改为独立睡眠队列，按 `wake_at_tick` 排序，timer tick 只检查队首。
3. 把 `block/wake` 从单一整数 channel 扩展为等待队列对象，例如 `WaitQueue`。
4. 为 ready queue 增加更明确的优先级策略：固定优先级、动态优先级或多级反馈队列三选一。
5. 为每个任务记录更完整的统计信息：总运行 tick、最近运行 tick、上下文切换次数、阻塞次数、睡眠次数。
6. 把调度日志 ring buffer 独立为 `kernel::log`，记录丢失日志次数，避免满缓冲静默覆盖。

完成标准：

1. `exit()` 后任务资源可被回收。
2. 任务可以在 `Ready/Sleeping/Blocked/Zombie` 间稳定迁移。
3. timer tick 不再扫描所有任务。
4. 调度器模块不直接依赖具体驱动。

### 阶段 2：异常和诊断完善

目标是让内核出错时能定位问题，而不是只停在串口上一行 panic。

建议任务：

1. 拆分 IRQ、SVC、Undefined、Prefetch Abort、Data Abort、FIQ 的 Rust handler。
2. 为异常入口保存统一 `TrapFrame`，包含 `r0-r12`、`sp`、`lr`、返回 PC、SPSR。
3. Data Abort 时读取并打印 DFSR/DFAR，Prefetch Abort 时读取 IFSR/IFAR。
4. panic 时打印当前任务、任务状态、当前 tick、最近一次 syscall、异常类型。
5. SVC syscall 返回值写回保存的 `r0`，为后续 `read/write/open` 等 syscall 做准备。
6. 明确异常期间 IRQ 策略：先保持单核、异常处理中关闭 IRQ，后续再考虑嵌套中断。

完成标准：

1. 非法指令、坏地址访问、未知 syscall 都能打印可诊断信息。
2. syscall 可以向任务返回错误码。
3. 异常路径和普通调度路径共享清晰的 trap frame 定义。

### 阶段 3：物理内存和 MMU

目标是从“裸地址运行”过渡到有权限隔离和虚拟地址空间的内核。

建议任务：

1. 把 early bump allocator 扩展为支持释放的物理页分配器。
2. 建立页帧元数据，记录空闲、已用、保留、设备 MMIO 等状态。
3. 建立 ARMv7-A 一级页表，先做 identity mapping，确保打开 MMU 后行为不变。
4. 设置内核代码只读可执行，rodata 只读，data/bss 可读写。
5. 把 UART、GIC、timer 等 MMIO 映射为 device memory，禁止错误 cache 属性。
6. 打开 MMU、I-cache、D-cache，并验证串口和中断仍正常。
7. 增加内核虚拟地址布局文档，固定 kernel、device、user 的地址区间。

完成标准：

1. 打开 MMU 后 QEMU 仍能启动、打印、调度。
2. 空闲物理页可以分配和释放。
3. 设备 MMIO 映射属性明确，不依赖默认 cache 行为。

### 阶段 4：用户态和进程模型

目标是把当前“内核线程”推进为“进程 + 线程 + syscall”的基础模型。

建议任务：

1. 区分 `Process` 和 `Thread`：进程拥有地址空间和资源表，线程拥有执行上下文和栈。
2. 为任务拆分 kernel stack 和 user stack。
3. 构造用户态 CPSR、用户 PC、用户 SP，通过异常返回进入 User mode。
4. 增加用户程序加载接口，先从内嵌二进制或 initrd 加载。
5. 扩展 syscall：`write`、`read`、`exit`、`yield`、`sleep`、`spawn`。
6. 为 syscall 做用户指针检查，避免用户态传入任意内核地址。
7. 实现用户任务退出后的资源回收路径。

完成标准：

1. 普通任务运行在 User mode。
2. 用户任务只能通过 SVC 进入内核。
3. 用户任务崩溃不会破坏内核和其他任务。

### 阶段 5：驱动和设备模型

目标是从硬编码 QEMU 地址过渡到可扩展的驱动框架。

建议任务：

1. 为 PL011 增加输入中断，支持接收字符和输入缓冲。
2. 建立 console 层，统一串口输入输出和日志输出。
3. 增加设备树 FDT 解析，不再硬编码 UART/GIC/timer 地址。
4. 增加 VirtIO MMIO 基础框架，优先实现 block 设备。
5. 为驱动定义统一接口：初始化、中断处理、读写、阻塞等待。
6. 明确 volatile MMIO、内存屏障和中断确认顺序。
7. 后续可加入 framebuffer 或 virtio-gpu，支持基本图形输出。

完成标准：

1. 串口输入可以唤醒阻塞任务。
2. 设备地址来自 FDT 或集中 platform 描述。
3. 至少一个 VirtIO 设备可以初始化并完成基本 I/O。

### 阶段 6：文件系统和程序加载

目标是让内核能从存储中加载用户程序，而不是把任务写死在内核里。

建议任务：

1. 增加 initrd 或 RAM disk，先使用只读 tar/cpio 格式。
2. 实现最小 VFS 抽象：inode、file、file descriptor。
3. 支持 `open/read/write/close` syscall。
4. 实现 ELF loader，把用户程序加载到独立地址空间。
5. 支持用户栈初始化，传入 argc/argv。
6. 后续增加 FAT32 或 ext2 只读支持，再考虑写支持。

完成标准：

1. 内核能加载一个独立用户 ELF。
2. 用户程序能通过 `write` 输出到 console。
3. 文件描述符和进程资源表开始成型。

### 阶段 7：同步原语和内核安全

目标是减少裸 `static mut` 和临界区混乱，为多模块内核打基础。

建议任务：

1. 实现 `SpinLock<T>`，单核阶段用关 IRQ 保护临界区。
2. 区分 IRQ-safe lock 和只能在线程上下文使用的 lock。
3. 增加 `OnceCell` 或初始化状态机，管理全局对象初始化。
4. 把调度器、页分配器、日志系统、设备队列封装成清晰所有权边界。
5. 逐步收缩 `unsafe` 范围，让上层模块使用安全 API。
6. 为关键 unsafe block 写明不变量，例如栈帧布局、MMIO 地址、页表对齐。

完成标准：

1. 全局可变状态有明确同步策略。
2. 大部分内核模块不需要直接写 `unsafe`。
3. panic/异常路径不会在持锁状态下死锁。

### 阶段 8：测试和调试

目标是让内核开发能被重复验证，而不是依赖手动看串口。

建议任务：

1. 增加 `make objdump`、`make readelf`、`make nm`，快速检查镜像布局。
2. 增加 `make debug`，用 QEMU `-S -s` 等待 `gdb-multiarch` 连接。
3. 增加 QEMU smoke test，启动后等待串口出现指定文本。
4. 把纯 Rust 数据结构抽成 host 可测试模块，例如 ready queue、位图分配器、等待队列。
5. 增加异常测试任务，触发 undefined instruction 和 data abort，验证诊断输出。
6. 后续接入 CI，至少保证 `cargo build --target armv7a-none-eabi` 和 host 单元测试不退化。

完成标准：

1. 本地一条命令能验证构建和 QEMU 基本启动。
2. 调度器、分配器、队列有 host 单元测试。
3. 关键异常路径有可重复触发的测试。

### 推荐近期里程碑

建议优先做这些，顺序尽量不要跳：

1. 做异常诊断：`TrapFrame`、当前任务、DFSR/DFAR、IFSR/IFAR、未知 syscall 错误码。
2. 做 reaper 和可释放页分配器：让 `exit()` 真正回收任务资源。
3. 做等待队列：替换简单 channel，并让串口输入能唤醒阻塞任务。
4. 做睡眠队列：timer tick 只检查即将到期的任务。
5. 做最小 MMU identity mapping：打开 MMU 后保持当前 demo 行为不变。
6. 拆分 kernel stack/user stack：为用户态入口做准备。
7. 做第一个用户态程序：只调用 `write/yield/exit`。

## 长线设计目标和方案

长期目标是把这个项目发展成一个结构清晰、可调试、可移植的教学型 Rust OS kernel。它不追求一开始就覆盖完整 Unix 语义，而是优先保证内核关键机制真实可靠：异常、中断、调度、地址空间、进程、驱动、文件系统和系统调用都运行在真实 CPU 模型上。

### 总体定位

设计定位：

1. 面向 ARMv7-A/AArch32，首先支持 QEMU `virt`，后续再抽象到更多 ARM board。
2. 采用单体内核架构，驱动、调度器、内存管理和文件系统都运行在内核态。
3. 优先单核正确性，后续再引入 SMP。
4. 以 Rust 为主要实现语言，汇编只保留在启动、异常入口、上下文恢复、少数 CPU 特权指令处。
5. 明确区分架构相关代码、平台设备代码、内核通用代码和用户程序。

不优先追求：

1. 完整 POSIX 兼容。
2. 高性能 SMP 调度。
3. 复杂图形栈。
4. 多架构同步支持。

这些可以作为更后期目标，不能压过内核基础机制的清晰性。

### 分层架构

建议长期目录结构：

```text
src/
  arch/
    aarch32/        # CPU 模式、异常向量、上下文、MMU、cache、barrier
  platform/
    qemu_virt/      # QEMU virt 地址布局、FDT、设备枚举
  drivers/
    uart/
    gic/
    timer/
    virtio/
  kernel/
    sched/
    task/
    sync/
    memory/
    syscall/
    log/
  mm/
    page_alloc.rs
    vmm.rs
    address_space.rs
  fs/
    vfs.rs
    initrd.rs
    tar.rs
    fat32.rs
  user/
    loader.rs
    elf.rs
```

核心原则：

1. `arch/` 只暴露 CPU 相关能力，不直接知道调度策略。
2. `drivers/` 不直接调度任务，只通过等待队列、IRQ handler 和内核同步原语交互。
3. `kernel/` 保存通用内核机制，例如任务、调度、syscall、日志和锁。
4. `mm/` 负责物理页、虚拟地址空间、页表和用户内存检查。
5. `fs/` 负责文件对象和路径，不直接操作具体块设备。

### 内核执行模型

长期执行模型：

1. 启动后只保留一个 bootstrap CPU 路径。
2. 内核初始化阶段关闭 IRQ，完成栈、`.bss`、串口、异常向量、GIC、timer、页分配器初始化。
3. 创建 idle 任务、init 任务和必要内核线程。
4. 打开 IRQ，进入调度器。
5. 普通用户任务运行在 User mode，通过 SVC 进入内核。
6. 中断只做短路径处理，把耗时工作交给内核线程或 bottom half。

关键约束：

1. IRQ handler 不做阻塞操作。
2. syscall 可以阻塞当前任务，但必须留下可调度任务，例如 idle。
3. panic 路径尽量不获取普通锁，只做最小诊断输出。
4. 所有进入调度器的路径必须有一致的上下文帧格式。

### 进程和线程模型

长期模型建议：

1. `Process` 表示资源容器，拥有地址空间、文件描述符表、当前工作目录、信号/退出状态等。
2. `Thread` 表示执行流，拥有寄存器上下文、kernel stack、user stack、调度状态和统计信息。
3. 一个进程可以先只支持一个线程，等地址空间和文件系统稳定后再扩展多线程。
4. 内核线程没有用户地址空间，只运行内核函数。
5. 用户线程必须通过 SVC 使用内核服务。

推荐最小 syscall 集：

```text
yield()
sleep(ticks)
exit(code)
write(fd, buf, len)
read(fd, buf, len)
open(path, flags)
close(fd)
spawn/process_create(...)
wait(pid)
```

实现顺序应先做 `write/yield/exit/sleep`，再做 `read/open/close/wait`。

### 内存设计

长期内存目标：

1. 物理页分配器支持分配和释放 4 KiB 页。
2. 内核有固定虚拟地址布局。
3. 每个用户进程有独立地址空间。
4. 用户页和内核页权限隔离。
5. MMIO 区域使用 device memory 属性。
6. 用户指针在 syscall 中必须检查。

建议地址空间策略：

```text
0x00000000..0x7fffffff  用户空间
0x80000000..0xbfffffff  内核线性映射或内核堆
0xc0000000..0xdfffffff  设备 MMIO 映射
0xe0000000..0xffffffff  内核固定映射、向量、调试区
```

这只是长期方向，当前 QEMU 直接从 `0x40000000` 启动。真正启用 MMU 前，需要先保证 identity mapping 路径稳定，再切换到高地址内核布局。

### 驱动模型

长期驱动目标：

1. 设备来源从硬编码地址迁移到 FDT 解析。
2. 每类设备有统一 trait 或接口。
3. IRQ handler 只确认中断、搬运少量状态、唤醒等待队列。
4. 阻塞 I/O 通过等待队列挂起任务。
5. VirtIO 作为主要 QEMU 设备模型，优先支持 block 和 console。

推荐驱动接口方向：

```rust
trait Device {
    fn name(&self) -> &'static str;
    fn init(&self);
}

trait IrqHandler {
    fn handle_irq(&self);
}
```

早期可以不用复杂 trait，先通过模块函数推进；等设备数量增加后再统一接口。

### 文件系统和程序加载

长期目标：

1. 先有 initrd，保证没有块设备时也能加载用户程序。
2. 再实现 VirtIO block，支持从块设备读取文件系统。
3. VFS 提供统一 inode/file/file descriptor 抽象。
4. ELF loader 负责创建地址空间、映射代码段、数据段、用户栈。
5. 用户程序通过 syscall 使用文件描述符。

推荐顺序：

1. 内嵌 init 用户程序。
2. initrd + tar/cpio 只读文件系统。
3. ELF loader。
4. VirtIO block。
5. FAT32 或 ext2 只读。
6. 写支持和缓存层。

### 安全和 Rust 边界

长期安全目标：

1. `unsafe` 必须集中在架构、MMIO、页表、上下文切换和裸指针访问处。
2. 每个 `unsafe` 模块都需要写清楚不变量。
3. 上层调度器、VFS、进程管理尽量暴露安全 API。
4. 所有用户指针进入内核前必须检查地址范围和权限。
5. 内核锁需要明确是否允许在 IRQ 上下文使用。
6. panic 和异常路径必须避免二次崩溃。

长期可以引入：

1. `SpinLock<T>`
2. `IrqSafeSpinLock<T>`
3. `OnceCell<T>`
4. `NonNull` 包装裸指针
5. 地址类型封装，例如 `PhysAddr`、`VirtAddr`、`UserPtr<T>`

### 可移植性目标

第一阶段只支持 QEMU `virt`，但代码边界要为移植保留空间：

1. `arch/aarch32` 处理 CPU 架构。
2. `platform/qemu_virt` 处理设备地址、内存范围、FDT。
3. `drivers` 处理设备协议。
4. `kernel` 不应该依赖具体 board 地址。

未来可选方向：

1. ARMv7-A 其他开发板。
2. AArch64 版本。
3. RISC-V 版本。

不建议过早多架构化。等 MMU、用户态、VirtIO、VFS 稳定后再抽象多架构接口更稳。

### 长期完成标准

可以把以下结果作为“从调度器 demo 发展成 OS kernel”的判断标准：

1. 内核开启 MMU，并区分内核态和用户态。
2. 用户程序运行在 User mode，不能直接访问内核内存。
3. 用户程序通过 syscall 完成输出、睡眠、退出和文件读写。
4. 调度器可以运行多个用户进程和内核线程。
5. 串口和 VirtIO block 至少有一个中断驱动路径。
6. 内核能从 initrd 或文件系统加载 ELF 用户程序。
7. panic/异常能打印当前任务和关键寄存器。
8. QEMU smoke test 可以自动验证启动和基本 syscall。
