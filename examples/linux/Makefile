HYPOCAUST_DIR := ../..
ALPINE_VERSION ?= 3.24.1
ALPINE_RELEASE_BASE ?= https://dl-cdn.alpinelinux.org/alpine/v3.24/releases/riscv64
ALPINE_ISO_NAME := alpine-standard-$(ALPINE_VERSION)-riscv64.iso
ALPINE_CACHE_DIR ?= $(HYPOCAUST_DIR)/target/alpine-$(ALPINE_VERSION)-riscv64
ALPINE_ISO := $(ALPINE_CACHE_DIR)/$(ALPINE_ISO_NAME)
ALPINE_CHECKSUM := $(ALPINE_ISO).sha256
ALPINE_EXTRACT_DIR := $(ALPINE_CACHE_DIR)/iso
ALPINE_KERNEL := $(ALPINE_EXTRACT_DIR)/boot/vmlinuz-lts
ALPINE_INITRD := $(ALPINE_EXTRACT_DIR)/boot/initramfs-lts
ALPINE_EXTRACT_STAMP := $(ALPINE_EXTRACT_DIR)/.verified-extract
QEMU ?= qemu-system-riscv64
QEMU_DATA_DIRS := $(shell $(QEMU) -L help 2>/dev/null)
QEMU_OPENSBI ?= $(firstword $(foreach dir,$(QEMU_DATA_DIRS),$(wildcard $(dir)/opensbi-riscv64-generic-fw_dynamic.bin)))
HYPOCAUST_OPENSBI := $(ALPINE_CACHE_DIR)/opensbi-riscv64-hypocaust-fw_dynamic.bin

.PHONY: help download verify prepare prepare-firmware run clean

help:
	@echo "Alpine Linux Guest example:"
	@echo "  make download  fetch and verify the pinned Alpine ISO"
	@echo "  make prepare   install ignored kernel/initramfs/disk inputs"
	@echo "  make prepare-firmware  delegate Guest privileged traps to Hypocaust"
	@echo "  make run       prepare and boot two Linux initramfs shells"
	@echo "  make clean     remove only the cached Alpine/example artifacts"

# PR #73 (`feature/linux-initramfs-shell-example`) downloads immutable release
# inputs into target/ rather than committing Linux binaries or a Guest submodule.
download: $(ALPINE_ISO) $(ALPINE_CHECKSUM)

$(ALPINE_CACHE_DIR):
	mkdir -p "$@"

$(ALPINE_ISO): | $(ALPINE_CACHE_DIR)
	curl --fail --location --output "$@.tmp" \
		"$(ALPINE_RELEASE_BASE)/$(ALPINE_ISO_NAME)"
	mv "$@.tmp" "$@"

$(ALPINE_CHECKSUM): | $(ALPINE_CACHE_DIR)
	curl --fail --location --output "$@.tmp" \
		"$(ALPINE_RELEASE_BASE)/$(ALPINE_ISO_NAME).sha256"
	mv "$@.tmp" "$@"

# PR #73 accepts either GNU sha256sum or the shasum tool shipped by macOS and
# fails closed before extracting a corrupt or substituted Guest artifact.
verify: download
	@if command -v sha256sum >/dev/null 2>&1; then \
		cd "$(ALPINE_CACHE_DIR)" && sha256sum --check "$(ALPINE_ISO_NAME).sha256"; \
	elif command -v shasum >/dev/null 2>&1; then \
		cd "$(ALPINE_CACHE_DIR)" && shasum -a 256 --check "$(ALPINE_ISO_NAME).sha256"; \
	else \
		echo "error: sha256sum or shasum is required" >&2; \
		exit 1; \
	fi

$(ALPINE_EXTRACT_STAMP): verify
	mkdir -p "$(ALPINE_EXTRACT_DIR)"
	bsdtar -xf "$(ALPINE_ISO)" -C "$(ALPINE_EXTRACT_DIR)" \
		boot/vmlinuz-lts boot/initramfs-lts
	touch "$@"

# PR #73 turns Alpine's compressed lts image into Linux's directly bootable
# Image input and gives each VM a distinct placeholder block backend.
prepare: $(ALPINE_EXTRACT_STAMP)
	gzip -dc "$(ALPINE_KERNEL)" > "$(HYPOCAUST_DIR)/guest_kernel.tmp"
	mv "$(HYPOCAUST_DIR)/guest_kernel.tmp" "$(HYPOCAUST_DIR)/guest_kernel"
	cp "$(ALPINE_INITRD)" "$(HYPOCAUST_DIR)/linux_initrd.tmp"
	mv -f "$(HYPOCAUST_DIR)/linux_initrd.tmp" "$(HYPOCAUST_DIR)/linux_initrd"
	dd if=/dev/zero of="$(HYPOCAUST_DIR)/fs-vm0.img" bs=1048576 count=16
	dd if=/dev/zero of="$(HYPOCAUST_DIR)/fs-vm1.img" bs=1048576 count=16

# PR #74 (`fix-bug/linux-opensbi-illegal-delegation`) patches a private copy of
# QEMU's firmware; the package-managed original is never modified in place.
prepare-firmware:
	@test -n "$(QEMU_OPENSBI)" || { \
		echo "error: QEMU's RISC-V OpenSBI firmware was not found" >&2; \
		echo "set QEMU_OPENSBI=/path/to/opensbi-riscv64-generic-fw_dynamic.bin" >&2; \
		exit 1; \
	}
	python3 patch_opensbi.py "$(QEMU_OPENSBI)" "$(HYPOCAUST_OPENSBI)"

run: prepare prepare-firmware
	# PR #73 enables the Linux-only embedded initramfs and boots two VM-local shells.
	$(MAKE) -C "$(HYPOCAUST_DIR)" qemu SMP=2 \
		BIOS="$(abspath $(HYPOCAUST_OPENSBI))" \
		GUEST_KERNEL_FEATURE="--features linux_guest"

clean:
	# PR #73 removes only known files and empty directories, even when callers
	# override ALPINE_CACHE_DIR; it never recursively deletes a supplied path.
	rm -f "$(ALPINE_KERNEL)" "$(ALPINE_INITRD)" "$(ALPINE_EXTRACT_STAMP)" \
		"$(ALPINE_ISO)" "$(ALPINE_CHECKSUM)" "$(HYPOCAUST_OPENSBI)"
	rmdir "$(ALPINE_EXTRACT_DIR)/boot" "$(ALPINE_EXTRACT_DIR)" \
		"$(ALPINE_CACHE_DIR)" 2>/dev/null || true
	rm -f "$(HYPOCAUST_DIR)/guest_kernel" "$(HYPOCAUST_DIR)/linux_initrd" \
		"$(HYPOCAUST_DIR)/fs-vm0.img" "$(HYPOCAUST_DIR)/fs-vm1.img"
