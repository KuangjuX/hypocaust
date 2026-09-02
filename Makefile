TARGET := riscv64gc-unknown-none-elf
MODE := debug
KERNEL_ELF := target/$(TARGET)/$(MODE)/hypocaust
KERNEL_BIN := target/$(TARGET)/$(MODE)/hypocaust.bin

GDB ?= gdb-multiarch
QEMU ?= qemu-system-riscv64
OBJDUMP ?= rust-objdump --arch-name=riscv64
OBJCOPY ?= rust-objcopy --binary-architecture=riscv64

FS_IMG := fs.img
GUEST_KERNEL_ELF := guest_kernel
GUEST_KERNEL_FEATURE := --features embed_guest_kernel

# PR #28 (`feature/xv6-rust-production-readme`) treats xv6-rust as an
# independent Guest. Override this path when it is not checked out beside us.
XV6_RUST_DIR ?= ../xv6-rust
XV6_RUST_KERNEL_ELF := $(XV6_RUST_DIR)/kernel/target/$(TARGET)/$(MODE)/kernel
XV6_RUST_FS_IMG := $(XV6_RUST_DIR)/fs.img

# QEMU's bundled OpenSBI loads Hypocaust at its linked 0x80200000 entry point.
QEMUOPTS := -machine virt -m 3G -bios default -kernel $(KERNEL_ELF) -nographic
QEMUOPTS += -drive file=$(FS_IMG),if=none,format=raw,id=x0
QEMUOPTS += -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0

.PHONY: help build xv6-rust qemu qemu-xv6 qemu-gdb gdb debug asm clean check-xv6-rust

help:
	@echo "Hypocaust build targets:"
	@echo "  make qemu-xv6                    build xv6-rust, copy its artifacts, and boot"
	@echo "  make xv6-rust                    refresh guest_kernel and fs.img only"
	@echo "  make qemu                        boot using existing local guest artifacts"
	@echo "  make qemu-gdb                    wait for GDB on TCP port 1234"
	@echo "  make clean                       remove Hypocaust and copied guest artifacts"
	@echo "  XV6_RUST_DIR=/path/to/xv6-rust  override the default sibling checkout"

# PR #28 (`feature/xv6-rust-production-readme`) fails clearly instead of
# fetching or switching revisions in a developer-owned checkout.
check-xv6-rust:
	@test -f "$(XV6_RUST_DIR)/Makefile" || { \
		echo "error: xv6-rust was not found at $(XV6_RUST_DIR)" >&2; \
		echo "clone it as documented in README.md or set XV6_RUST_DIR" >&2; \
		exit 1; \
	}

# PR #28 (`feature/xv6-rust-production-readme`) builds xv6-rust's single-hart
# SBI payload and filesystem, then copies them into our build context.
xv6-rust: check-xv6-rust
	$(MAKE) -C "$(XV6_RUST_DIR)" fs.img
	$(MAKE) -C "$(XV6_RUST_DIR)" sbi
	@test -f "$(XV6_RUST_KERNEL_ELF)" || { echo "error: missing $(XV6_RUST_KERNEL_ELF)" >&2; exit 1; }
	@test -f "$(XV6_RUST_FS_IMG)" || { echo "error: missing $(XV6_RUST_FS_IMG)" >&2; exit 1; }
	cp "$(XV6_RUST_KERNEL_ELF)" "$(GUEST_KERNEL_ELF)"
	cp "$(XV6_RUST_FS_IMG)" "$(FS_IMG)"

$(GUEST_KERNEL_ELF):
	@echo "error: $(GUEST_KERNEL_ELF) is missing; run 'make xv6-rust' first" >&2
	@false

$(FS_IMG):
	@echo "error: $(FS_IMG) is missing; run 'make xv6-rust' first" >&2
	@false

build: $(GUEST_KERNEL_ELF) $(FS_IMG)
	cargo build $(GUEST_KERNEL_FEATURE)

$(KERNEL_BIN): build
	$(OBJCOPY) $(KERNEL_ELF) --strip-all -O binary $@

qemu: build
	$(QEMU) $(QEMUOPTS)

qemu-xv6: xv6-rust build
	$(QEMU) $(QEMUOPTS)

qemu-gdb: build
	$(QEMU) $(QEMUOPTS) -S -gdb tcp::1234

gdb: $(KERNEL_ELF)
	$(GDB) $(KERNEL_ELF)

debug: build
	@tmux new-session -d \
		"$(QEMU) $(QEMUOPTS) -S -gdb tcp::1234" && \
		tmux split-window -h "$(GDB) -ex 'file $(KERNEL_ELF)' -ex 'set arch riscv:rv64' -ex 'target remote localhost:1234'" && \
		tmux -2 attach-session -d

asm: build
	$(OBJDUMP) -d $(KERNEL_ELF) > hyper.S
	$(OBJDUMP) -d $(GUEST_KERNEL_ELF) > guest.S

clean:
	cargo clean
	rm -f $(GUEST_KERNEL_ELF) $(FS_IMG) hyper.S guest.S
