KERNEL := target/armv7a-none-eabi/debug/rust_scheduler_demo
QEMU ?= qemu-system-arm

.PHONY: build run clean

build:
	cargo build

run: build
	$(QEMU) -M virt,gic-version=2,virtualization=off,secure=off -cpu cortex-a15 -m 128M -nographic -serial mon:stdio -kernel $(KERNEL)

clean:
	cargo clean
