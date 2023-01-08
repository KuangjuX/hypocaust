TARGET		:= riscv64gc-unknown-none-elf
MODE		:= debug
KERNEL_ELF	:= target/$(TARGET)/$(MODE)/hypocaust
KERNEL_BIN	:= target/$(TARGET)/$(MODE)/hypocaust.bin
CPUS		:= 1

BOARD 		:= qemu

# 客户操作系统
GUEST_KERNEL_ELF	:= ./guest_kernel
# GUEST_KERNEL_BIN	:= minikernel/target/$(TARGET)/$(MODE)/minikernel.bin

GUEST_KERNEL_FEATURE:=$(if $(GUEST_KERNEL_ELF), --features embed_guest_kernel, )

OBJDUMP     := rust-objdump --arch-name=riscv64
OBJCOPY     := rust-objcopy --binary-architecture=riscv64

QEMU 		:= qemu-system-riscv64
BOOTLOADER	:= bootloader/rustsbi-qemu.bin

KERNEL_ENTRY_PA := 0x80200000

QEMUOPTS	= -M 3G -machine virt -bios $(BOOTLOADER) -display none
QEMUOPTS	+=-kernel $(KERNEL_BIN) -initrd $(GUEST_KERNEL_ELF)
QEMUOPTS	+=-serial stdio




$(GUEST_KERNEL_ELF):
	cd minikernel && cargo build && cp target/$(TARGET)/$(MODE)/minikernel ../guest_kernel

# $(GUEST_KERNEL_BIN): $(GUEST_KERNEL_ELF)
# 	$(OBJCOPY) $(GUEST_KERNEL_ELF) --strip-all -O binary $@

build: $(GUEST_KERNEL_ELF)
	cargo build $(GUEST_KERNEL_FEATURE)

$(KERNEL_BIN): build 
	$(OBJCOPY) $(KERNEL_ELF) --strip-all -O binary $@

	

qemu: $(KERNEL_BIN)
	$(QEMU) $(QEMUOPTS)

clean:
	cargo clean
	cd minikernel && cargo clean
	rm guest_kernel