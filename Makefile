TARGET		:= riscv64gc-unknown-none-elf
MODE		:= debug
KERNEL_ELF	:= target/$(TARGET)/$(MODE)/hypocaust
KERNEL_BIN	:= target/$(TARGET)/$(MODE)/hypocaust.bin
CPUS		:= 1

BOARD 		:= qemu

GDB			:= gdb-multiarch

FS_IMG 		:= fs.img

# 客户操作系统
GUEST_KERNEL_ELF	:= ./guest_kernel
# GUEST_KERNEL_BIN	:= minikernel/target/$(TARGET)/$(MODE)/minikernel.bin

GUEST_KERNEL_FEATURE:=$(if $(GUEST_KERNEL_ELF), --features embed_guest_kernel, )

OBJDUMP     := rust-objdump --arch-name=riscv64
OBJCOPY     := rust-objcopy --binary-architecture=riscv64

QEMU 		:= qemu-system-riscv64
BOOTLOADER	:= bootloader/rustsbi-qemu.bin

KERNEL_ENTRY_PA := 0x80200000

QEMUOPTS	= --machine virt -m 3G -bios $(BOOTLOADER) -nographic
QEMUOPTS	+=-device loader,file=$(KERNEL_BIN),addr=$(KERNEL_ENTRY_PA)
QEMUOPTS	+=-drive file=$(FS_IMG),if=none,format=raw,id=x0
QEMUOPTS	+=-device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0




$(GUEST_KERNEL_ELF):
	cd minikernel/user && cargo build --release
	cd minikernel && cargo build && cp target/$(TARGET)/$(MODE)/minikernel ../guest_kernel

# $(GUEST_KERNEL_BIN): $(GUEST_KERNEL_ELF)
# 	$(OBJCOPY) $(GUEST_KERNEL_ELF) --strip-all -O binary $@

build: $(GUEST_KERNEL_ELF) $(FS_IMG)
	cargo build $(GUEST_KERNEL_FEATURE)

$(KERNEL_BIN): build 
	$(OBJCOPY) $(KERNEL_ELF) --strip-all -O binary $@

	

qemu: $(KERNEL_BIN)
	$(QEMU) $(QEMUOPTS)

clean:
	cargo clean
	cd minikernel && cargo clean
	cd minikernel/user && cargo clean
	rm guest_kernel *.S $(FS_IMG)

qemu-gdb: $(KERNEL_ELF)
	$(QEMU) $(QEMUOPTS) -S -gdb tcp::1234

gdb: $(KERNEL_ELF)
	$(GDB) $(KERNEL_ELF)

debug: $(KERNEL_BIN)
	@tmux new-session -d \
		"$(QEMU) $(QEMUOPTS) -s -S" && \
		tmux split-window -h "$(GDB) -ex 'file $(KERNEL_ELF)' -ex 'set arch riscv:rv64' -ex 'target remote localhost:1234'" && \
		tmux -2 attach-session -d

asm:
	riscv64-unknown-elf-objdump -d target/riscv64gc-unknown-none-elf/debug/hypocaust > hyper.S 
	riscv64-unknown-elf-objdump -d guest_kernel > guest.S 


$(FS_IMG):
	cd minikernel && make fs-img 
	cp minikernel/user/target/$(TARGET)/release/fs.img ./
	# touch fs.img