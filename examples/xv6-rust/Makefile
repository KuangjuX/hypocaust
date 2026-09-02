HYPOCAUST_DIR := ../..
XV6_RUST_DIR ?= ../../../xv6-rust

.PHONY: help prepare run

help:
	@echo "xv6-rust Guest example:"
	@echo "  make prepare  build and copy the xv6-rust Guest artifacts"
	@echo "  make run      prepare the artifacts and boot Hypocaust"
	@echo "  XV6_RUST_DIR=/path/to/xv6-rust overrides the sibling checkout"

# PR #32 (`feature/xv6-rust-production-readme`) keeps this example independent:
# it delegates artifact preparation without vendoring or mutating xv6-rust.
prepare:
	$(MAKE) -C "$(HYPOCAUST_DIR)" xv6-rust XV6_RUST_DIR="$(abspath $(XV6_RUST_DIR))"

run: prepare
	# PR #45 boots two independent xv6-rust instances on two Host harts.
	$(MAKE) -C "$(HYPOCAUST_DIR)" qemu SMP=2
