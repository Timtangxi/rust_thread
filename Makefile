KERNEL := target/armv7a-none-eabi/debug/rust_scheduler_demo
QEMU ?= qemu-system-arm
BUILD_DIR := build
CONFIG := .config
DEFCONFIG := configs/qemu_virt_defconfig
AUTOCONF_MK := include/generated/autoconf.mk
AUTOCONF_RS := include/generated/autoconf.rs
DTB := $(BUILD_DIR)/qemu-virt.dtb
INITRD_IMAGE := $(BUILD_DIR)/initrd.img
USERSPACE_DIR := userspace
USERSPACE_BUILD := $(USERSPACE_DIR)/build
USERSPACE_LIB := $(USERSPACE_BUILD)/libuserlib.rlib
USERSPACE_START := $(USERSPACE_BUILD)/start.o
USERSPACE_BINS := init shell ls cat echo
USERSPACE_ROOT := $(USERSPACE_BUILD)/rootfs
USERSPACE_ROOTFS := $(USERSPACE_BUILD)/rootfs.tar

-include $(AUTOCONF_MK)

CONFIG_MMU ?= n
CONFIG_QEMU_FDT_LOADER ?= n
CONFIG_VIRTIO_MMIO ?= n
CONFIG_VIRTIO_BLK ?= n
CONFIG_VIRTIO_NET ?= n
CONFIG_QEMU_MEMORY_MB ?= 128
CONFIG_QEMU_DTB_ADDR ?= 0x47f00000
CONFIG_QEMU_VIRTIO_BLK_PATH ?= /home/tang/rust/rootfs.ext4
CONFIG_QEMU_NETDEV ?= user,id=net0
CONFIG_INITRD_EXTERNAL ?= n
CONFIG_INITRD_EXTERNAL_PATH ?= /home/tang/rust/rootfs.cpio.uboot
CONFIG_INITRD_LOAD_ADDR ?= 0x46000000

QEMU_MACHINE := virt,gic-version=2,virtualization=off,secure=off
QEMU_BASE_ARGS := -M $(QEMU_MACHINE) -cpu cortex-a15 -m $(CONFIG_QEMU_MEMORY_MB)M -nographic -serial mon:stdio -kernel $(KERNEL)
QEMU_EXTRA_ARGS :=
QEMU_DTB_EXTRA_ARGS :=
CARGO_ARGS :=

ifeq ($(CONFIG_MMU),n)
CARGO_ARGS += --no-default-features
endif

ifeq ($(CONFIG_QEMU_FDT_LOADER),y)
QEMU_EXTRA_ARGS += -device loader,file=$(DTB),addr=$(CONFIG_QEMU_DTB_ADDR)
RUN_DEPS += dtb
endif

ifeq ($(CONFIG_INITRD_EXTERNAL),y)
QEMU_EXTRA_ARGS += -device loader,file=$(INITRD_IMAGE),addr=$(CONFIG_INITRD_LOAD_ADDR)
RUN_DEPS += initrd
endif

ifeq ($(CONFIG_VIRTIO_BLK),y)
QEMU_EXTRA_ARGS += -drive if=none,file=$(CONFIG_QEMU_VIRTIO_BLK_PATH),format=raw,id=blk0
QEMU_EXTRA_ARGS += -device virtio-blk-device,drive=blk0,bus=virtio-mmio-bus.0
QEMU_DTB_EXTRA_ARGS += -drive if=none,file=$(CONFIG_QEMU_VIRTIO_BLK_PATH),format=raw,id=blk0
QEMU_DTB_EXTRA_ARGS += -device virtio-blk-device,drive=blk0,bus=virtio-mmio-bus.0
endif

ifeq ($(CONFIG_VIRTIO_NET),y)
QEMU_EXTRA_ARGS += -netdev $(CONFIG_QEMU_NETDEV)
QEMU_EXTRA_ARGS += -device virtio-net-device,netdev=net0,bus=virtio-mmio-bus.1
QEMU_DTB_EXTRA_ARGS += -netdev $(CONFIG_QEMU_NETDEV)
QEMU_DTB_EXTRA_ARGS += -device virtio-net-device,netdev=net0,bus=virtio-mmio-bus.1
endif

.PHONY: all defconfig menuconfig oldconfig prepare autoconf build __build userspace userspace-rootfs dtb __dtb initrd __initrd run __run busybox-smoke debug __debug readelf nm objdump clean distclean

all: build

defconfig:
	mkdir -p $(BUILD_DIR) include/generated
	python3 scripts/kconfig.py defconfig $(DEFCONFIG)

menuconfig:
	mkdir -p $(BUILD_DIR) include/generated
	python3 scripts/kconfig.py menuconfig

oldconfig: prepare
	python3 scripts/kconfig.py oldconfig

prepare:
	mkdir -p $(BUILD_DIR) include/generated
	@if [ ! -f $(CONFIG) ]; then python3 scripts/kconfig.py defconfig $(DEFCONFIG); fi

build: oldconfig
	$(MAKE) __build

__build:
	cargo build $(CARGO_ARGS)

userspace:
	mkdir -p $(USERSPACE_BUILD)
	rustc --target armv7a-none-eabi --edition=2024 --crate-type=rlib --crate-name userlib -C panic=abort -C opt-level=z $(USERSPACE_DIR)/lib/lib.rs -o $(USERSPACE_LIB)
	rustc --target armv7a-none-eabi --edition=2024 -C panic=abort -C opt-level=z --extern userlib=$(USERSPACE_LIB) --emit=obj $(USERSPACE_DIR)/lib/start.rs -o $(USERSPACE_START)
	@for bin in $(USERSPACE_BINS); do \
		rustc --target armv7a-none-eabi --edition=2024 -C panic=abort -C opt-level=z -C relocation-model=static -C link-arg=-T$(USERSPACE_DIR)/linker.ld -C link-arg=--nmagic --extern userlib=$(USERSPACE_LIB) $(USERSPACE_DIR)/bin/$$bin.rs -C link-arg=$(USERSPACE_START) -o $(USERSPACE_BUILD)/$$bin; \
	done

userspace-rootfs: userspace
	rm -rf $(USERSPACE_ROOT)
	mkdir -p $(USERSPACE_ROOT)/bin $(USERSPACE_ROOT)/dev $(USERSPACE_ROOT)/proc $(USERSPACE_ROOT)/tmp
	cp $(USERSPACE_BUILD)/init $(USERSPACE_ROOT)/bin/init
	cp $(USERSPACE_BUILD)/shell $(USERSPACE_ROOT)/bin/shell
	cp $(USERSPACE_BUILD)/ls $(USERSPACE_ROOT)/bin/ls
	cp $(USERSPACE_BUILD)/cat $(USERSPACE_ROOT)/bin/cat
	cp $(USERSPACE_BUILD)/echo $(USERSPACE_ROOT)/bin/echo
	tar -C $(USERSPACE_ROOT) -cf $(USERSPACE_ROOTFS) .

dtb: oldconfig
	$(MAKE) __dtb

__dtb:
	$(QEMU) -M $(QEMU_MACHINE),dumpdtb=$(DTB) -cpu cortex-a15 -m $(CONFIG_QEMU_MEMORY_MB)M -nographic $(QEMU_DTB_EXTRA_ARGS)

initrd: oldconfig
	$(MAKE) __initrd

__initrd:
	python3 scripts/prepare_initrd.py $(CONFIG_INITRD_EXTERNAL_PATH) $(INITRD_IMAGE)

run: oldconfig
	$(MAKE) __run

__run: __build $(RUN_DEPS)
	$(QEMU) $(QEMU_BASE_ARGS) $(QEMU_EXTRA_ARGS)

busybox-smoke:
	$(MAKE) defconfig DEFCONFIG=configs/qemu_virt_defconfig
	python3 scripts/kconfig.py oldconfig
	sed -i 's/# CONFIG_BUSYBOX_SMOKE is not set/CONFIG_BUSYBOX_SMOKE=y/' $(CONFIG)
	python3 scripts/kconfig.py oldconfig
	timeout 30s $(MAKE) __run 2>&1 | tee $(BUILD_DIR)/busybox-smoke.log
	grep -E "BusyBox|Usage: busybox" $(BUILD_DIR)/busybox-smoke.log

debug: oldconfig
	$(MAKE) __debug

__debug: __build $(RUN_DEPS)
	$(QEMU) $(QEMU_BASE_ARGS) $(QEMU_EXTRA_ARGS) -S -s

readelf: build
	llvm-readelf -h -S $(KERNEL)

nm: build
	llvm-nm -n $(KERNEL)

objdump: build
	llvm-objdump -d $(KERNEL)

clean:
	cargo clean
	rm -rf $(BUILD_DIR)
	rm -rf $(USERSPACE_BUILD)

distclean: clean
	rm -f $(CONFIG) $(AUTOCONF_MK) $(AUTOCONF_RS)
