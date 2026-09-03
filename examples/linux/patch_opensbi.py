#!/usr/bin/env python3
"""Prepare OpenSBI to hand deprivileged Guest instructions to Hypocaust.

PR #74 (`fix-bug/linux-opensbi-illegal-delegation`) changes only OpenSBI's
initial `medeleg` mask: bit 2 delegates illegal-instruction exceptions to Host
S-mode. Guest S-mode executes in U-mode without the RISC-V H extension, so its
privileged instructions are deliberately illegal and must reach Hypocaust.
"""

from __future__ import annotations

import os
from pathlib import Path
import sys


# OpenSBI v1.8.1's generic RV64 firmware constructs medeleg as 0x4b109. The
# patch changes the ADDI immediate to 0x10d, adding only bit 2. Including its
# following conditional branch makes accidental matches fail closed instead of
# corrupting an unrelated instruction.
ORIGINAL_MASK_CODE = bytes.fromhex("93 84 94 10 99 e3")
DELEGATING_MASK_CODE = bytes.fromhex("93 84 d4 10 99 e3")


def patch_firmware(source: Path, destination: Path) -> None:
    firmware = source.read_bytes()
    matches = firmware.count(ORIGINAL_MASK_CODE)
    if matches != 1:
        raise RuntimeError(
            "expected exactly one OpenSBI v1.8.1 delegation-mask sequence; "
            f"found {matches} in {source}"
        )

    patched = firmware.replace(ORIGINAL_MASK_CODE, DELEGATING_MASK_CODE, 1)
    destination.parent.mkdir(parents=True, exist_ok=True)
    temporary = destination.with_name(destination.name + ".tmp")
    temporary.write_bytes(patched)
    os.chmod(temporary, source.stat().st_mode)
    os.replace(temporary, destination)

    # PR #74 verifies both sides of the one-instruction transformation before
    # QEMU can consume the generated firmware.
    written = destination.read_bytes()
    if written.count(DELEGATING_MASK_CODE) != 1 or ORIGINAL_MASK_CODE in written:
        raise RuntimeError(f"failed to verify patched firmware {destination}")


def main() -> int:
    if len(sys.argv) != 3:
        print(f"usage: {sys.argv[0]} SOURCE DESTINATION", file=sys.stderr)
        return 2

    try:
        patch_firmware(Path(sys.argv[1]), Path(sys.argv[2]))
    except (OSError, RuntimeError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
