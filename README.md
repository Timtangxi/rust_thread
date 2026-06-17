# Rust AArch32 Round-Robin Kernel

这是一个真正运行在 CPU/QEMU 上的 Rust 裸机调度器，不是用户态 `std` demo。

当前目标：

- 架构：AArch32 / ARMv7-A
- 机器：QEMU `virt`
- CPU：`cortex-a15`
- 串口：PL011 `0x09000000`
- 中断控制器：GICv2
- 时钟源：ARM Generic Physical Timer，IRQ 30
- 调度策略：固定任务表的时间片轮转 Round-Robin

## 代码结构

- `src/boot.S`：启动代码、异常向量、IRQ 保存/恢复路径。
- `src/switch.S`：第一次进入任务时的上下文恢复。
- `src/main.rs`：内核入口、任务入口和 IRQ 分发。
- `src/scheduler.rs`：Round-Robin 调度器。
- `src/task.rs`：任务表和每个任务独立内核栈。
- `src/context.rs`：AArch32 任务栈帧布局。
- `src/gic.rs`：QEMU virt 上的 GICv2 初始化。
- `src/timer.rs`：Generic Timer 周期 tick。
- `src/uart.rs`：PL011 串口输出。
- `linker.ld`：内核链接地址 `0x40000000`。

IRQ 发生时，汇编入口会保存当前任务的通用寄存器、异常返回 PC 和 SPSR；Rust 调度器只切换保存后的任务栈指针，然后汇编用 `rfe` 返回到下一个任务。

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
timer: generic physical timer at 62500000 Hz
scheduler: start task init (1)
[init] tick work item 0
tick 0001: init -> shell
[shell] prompt refresh 0
tick 0002: shell -> worker
[worker] background pass 0
tick 0003: worker -> init
```

## 后续扩展方向

1. 把固定任务表换成内核分配器管理的 TCB。
2. 增加任务状态：`Sleeping`、`Blocked`、`Zombie`。
3. 加入系统调用/SVC，让任务主动让出 CPU。
4. 加入 MMU 页表，把内核空间和用户空间分开。
5. 将任务切到 User/System mode，并建立异常返回到用户态的路径。
6. 增加优先级、睡眠队列和 tickless timer。
