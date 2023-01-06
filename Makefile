TARGET		:= riscv64gc-unknown-none-elf
MODE		:= debug
KERNEL_ELF	:= target/$(TARGET)/$(MODE)/hypocaust
KERNEL_BIN	:= target/$(TARGET)/$(MODE)/hypocaust.bin
CPUS		:= 1

OBJDUMP     := rust-objdump --arch-name=riscv64
OBJCOPY     := rust-objcopy --binary-architecture=riscv64

QEMU 		:= qemu-system-riscv64
BOOTLOADER	:= bootloader/rustsbi-qemu.bin

KERNEL_ENTRY_PA := 0x80200000

QEMUOPTS	= -M 128m -machine virt -bios $(BOOTLOADER) -display none
QEMUOPTS	+=-device loader,file=$(KERNEL_BIN),addr=$(KERNEL_ENTRY_PA)
QEMUOPTS	+=-serial stdio


build: 
	@cargo build

$(KERNEL_BIN): build
	@$(OBJCOPY) $(KERNEL_ELF) --strip-all -O binary $@

qemu: $(KERNEL_BIN)
	$(QEMU) $(QEMUOPTS)

clean:
	@cargo clean