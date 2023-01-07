TARGET		:= riscv64gc-unknown-none-elf
MODE		:= debug
KERNEL_ELF	:= target/$(TARGET)/$(MODE)/hypocaust
KERNEL_BIN	:= target/$(TARGET)/$(MODE)/hypocaust.bin
CPUS		:= 1

# 客户操作系统
GUEST_KERNEL_ELF	:= minikernel/target/$(TARGET)/$(MODE)/minikernel
GUEST_KERNEL_BIN	:= minikernel/target/$(TARGET)/$(MODE)/minikernel.bin

GUEST_KERNEL_FEATURE:=$(if $(GUEST_KERNEL_BIN), --features embed_guest_kernel, )

OBJDUMP     := rust-objdump --arch-name=riscv64
OBJCOPY     := rust-objcopy --binary-architecture=riscv64

QEMU 		:= qemu-system-riscv64
BOOTLOADER	:= bootloader/rustsbi-qemu.bin

KERNEL_ENTRY_PA := 0x80200000

QEMUOPTS	= -M 128m -machine virt -bios $(BOOTLOADER) -display none
QEMUOPTS	+=-kernel $(KERNEL_BIN) -initrd $(GUEST_KERNEL_BIN)
QEMUOPTS	+=-serial stdio


build: 
	cargo build $(GUEST_KERNEL_FEATURE)

$(KERNEL_BIN): build
	@$(OBJCOPY) $(KERNEL_ELF) --strip-all -O binary $@

$(GUEST_KERNEL_ELF):
	@cd minikernel
	@cargo build

$(GUEST_KERNEL_BIN): $(GUEST_KERNEL_ELF)
	@$(OBJCOPY) $(GUEST_KERNEL_ELF) --strip-all -O binary $@
	

qemu: $(KERNEL_BIN) $(GUEST_KERNEL_BIN)
	$(QEMU) $(QEMUOPTS)

clean:
	@cargo clean