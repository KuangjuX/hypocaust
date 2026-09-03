//! Guest-facing RISC-V Supervisor Binary Interface dispatch.
//!
//! Legacy extension IDs remain available for the xv6-rust example. PR #54
//! (`feature/sbi-v02-base-time`) adds the SBI v0.2 BASE and TIME contract used
//! by Linux without exposing the Host firmware implementation to a Guest.
//! PR #56 (`feature/sbi-srst-shutdown`) adds standards-based, VM-local poweroff.
//! PR #61 (`feature/sbi-dbcn-console`) implements the complete SBI 2.0 Debug
//! Console extension so Linux can emit early boot diagnostics without a UART.

pub const SBI_SET_TIMER: usize = 0;
pub const SBI_CONSOLE_PUTCHAR: usize = 1;
pub const SBI_CONSOLE_GETCHAR: usize = 2;
pub const SBI_CLEAR_IPI: usize = 3;
pub const SBI_SEND_IPI: usize = 4;
pub const SBI_REMOTE_FENCE_I: usize = 5;
pub const SBI_REMOTE_SFENCE_VMA: usize = 6;
pub const SBI_REMOTE_SFENCE_VMA_ASID: usize = 7;
pub const SBI_SHUTDOWN: usize = 8;

pub const SBI_EXT_BASE: usize = 0x10;
pub const SBI_EXT_TIME: usize = 0x5449_4d45;
pub const SBI_EXT_SRST: usize = 0x5352_5354;
pub const SBI_EXT_DBCN: usize = 0x4442_434e;

const SBI_BASE_GET_SPEC_VERSION: usize = 0;
const SBI_BASE_GET_IMPL_ID: usize = 1;
const SBI_BASE_GET_IMPL_VERSION: usize = 2;
const SBI_BASE_PROBE_EXTENSION: usize = 3;
const SBI_BASE_GET_MVENDORID: usize = 4;
const SBI_BASE_GET_MARCHID: usize = 5;
const SBI_BASE_GET_MIMPID: usize = 6;
const SBI_TIME_SET_TIMER: usize = 0;
const SBI_SRST_SYSTEM_RESET: usize = 0;
const SBI_SRST_RESET_TYPE_SHUTDOWN: usize = 0;
const SBI_SRST_RESET_TYPE_COLD_REBOOT: usize = 1;
const SBI_SRST_RESET_TYPE_WARM_REBOOT: usize = 2;
const SBI_SRST_RESET_REASON_NONE: usize = 0;
const SBI_SRST_RESET_REASON_SYSTEM_FAILURE: usize = 1;
const SBI_DBCN_CONSOLE_WRITE: usize = 0;
const SBI_DBCN_CONSOLE_READ: usize = 1;
const SBI_DBCN_CONSOLE_WRITE_BYTE: usize = 2;

const SBI_SUCCESS: usize = 0;
const SBI_ERR_NOT_SUPPORTED: usize = (-2isize) as usize;
pub(crate) const SBI_ERR_INVALID_PARAM: usize = (-3isize) as usize;
// PR #61 advertises SBI 2.0 because DBCN first became ratified in that
// specification. The encoding stores the major version above 24 minor bits.
const SBI_SPEC_VERSION_2_0: usize = 2 << 24;
// PR #54 uses a private, human-readable implementation identifier (`HYPO`).
// Linux treats this value as informational and does not branch on it.
const SBI_IMPL_ID_HYPOCAUST: usize = 0x4859_504f;
const SBI_IMPL_VERSION: usize = 0x0001_0000;

/// Host-side work requested by a successfully decoded Guest SBI call.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SbiAction {
    None,
    SetTimer(usize),
    /// PR #56 (`feature/sbi-srst-shutdown`) maps standard SRST shutdown onto
    /// the same VM-local terminal transition as the isolated legacy ABI.
    StopCurrentVm,
    /// PR #61 carries a DBCN request out of the pure ABI decoder. The trap
    /// layer performs the actual copy through the current VM's RAM capability.
    DebugConsoleWrite {
        num_bytes: usize,
        base_addr_lo: usize,
        base_addr_hi: usize,
    },
    DebugConsoleRead {
        num_bytes: usize,
        base_addr_lo: usize,
        base_addr_hi: usize,
    },
    DebugConsoleWriteByte(u8),
}

/// SBI v0.2's `(error, value)` result plus work that must affect this vCPU.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SbiResponse {
    pub error: usize,
    pub value: usize,
    pub action: SbiAction,
}

impl SbiResponse {
    const fn success(value: usize) -> Self {
        Self {
            error: SBI_SUCCESS,
            value,
            action: SbiAction::None,
        }
    }

    const fn with_action(action: SbiAction) -> Self {
        Self {
            error: SBI_SUCCESS,
            value: 0,
            action,
        }
    }

    const fn not_supported() -> Self {
        Self {
            error: SBI_ERR_NOT_SUPPORTED,
            value: 0,
            action: SbiAction::None,
        }
    }

    const fn invalid_param() -> Self {
        Self {
            error: SBI_ERR_INVALID_PARAM,
            value: 0,
            action: SbiAction::None,
        }
    }
}

/// Decode a modern SBI call. `None` reserves extension IDs 0..=8 for the
/// existing legacy ABI, whose return convention differs from SBI v0.2.
///
/// PR #54 deliberately advertises only extensions implemented end-to-end.
/// After PR #61 that set is BASE, TIME, SRST, and DBCN; Linux probes for other
/// extensions receive zero until their VM-local operations exist.
pub fn dispatch_modern(
    extension_id: usize,
    function_id: usize,
    arguments: [usize; 6],
) -> Option<SbiResponse> {
    if extension_id <= SBI_SHUTDOWN {
        return None;
    }

    let response = match extension_id {
        SBI_EXT_BASE => dispatch_base(function_id, arguments[0]),
        SBI_EXT_TIME => match function_id {
            SBI_TIME_SET_TIMER => SbiResponse::with_action(SbiAction::SetTimer(arguments[0])),
            _ => SbiResponse::not_supported(),
        },
        SBI_EXT_SRST => dispatch_srst(function_id, arguments[0], arguments[1]),
        SBI_EXT_DBCN => dispatch_dbcn(function_id, arguments),
        _ => SbiResponse::not_supported(),
    };
    Some(response)
}

fn dispatch_base(function_id: usize, argument0: usize) -> SbiResponse {
    match function_id {
        SBI_BASE_GET_SPEC_VERSION => SbiResponse::success(SBI_SPEC_VERSION_2_0),
        SBI_BASE_GET_IMPL_ID => SbiResponse::success(SBI_IMPL_ID_HYPOCAUST),
        SBI_BASE_GET_IMPL_VERSION => SbiResponse::success(SBI_IMPL_VERSION),
        SBI_BASE_PROBE_EXTENSION => SbiResponse::success(usize::from(matches!(
            argument0,
            SBI_EXT_BASE | SBI_EXT_TIME | SBI_EXT_SRST | SBI_EXT_DBCN
        ))),
        // PR #54 cannot read M-mode identification CSRs while Hypocaust runs
        // in HS/S-mode. SBI permits zero when platform IDs are unavailable.
        SBI_BASE_GET_MVENDORID | SBI_BASE_GET_MARCHID | SBI_BASE_GET_MIMPID => {
            SbiResponse::success(0)
        }
        _ => SbiResponse::not_supported(),
    }
}

fn dispatch_dbcn(function_id: usize, arguments: [usize; 6]) -> SbiResponse {
    let action = match function_id {
        SBI_DBCN_CONSOLE_WRITE => SbiAction::DebugConsoleWrite {
            num_bytes: arguments[0],
            base_addr_lo: arguments[1],
            base_addr_hi: arguments[2],
        },
        SBI_DBCN_CONSOLE_READ => SbiAction::DebugConsoleRead {
            num_bytes: arguments[0],
            base_addr_lo: arguments[1],
            base_addr_hi: arguments[2],
        },
        // The SBI signature is uint8_t, so only the low byte is observable.
        SBI_DBCN_CONSOLE_WRITE_BYTE => SbiAction::DebugConsoleWriteByte(arguments[0] as u8),
        _ => return SbiResponse::not_supported(),
    };
    SbiResponse::with_action(action)
}

fn dispatch_srst(function_id: usize, reset_type: usize, reset_reason: usize) -> SbiResponse {
    if function_id != SBI_SRST_SYSTEM_RESET {
        return SbiResponse::not_supported();
    }
    if !matches!(
        reset_reason,
        SBI_SRST_RESET_REASON_NONE | SBI_SRST_RESET_REASON_SYSTEM_FAILURE
    ) {
        return SbiResponse::invalid_param();
    }

    match reset_type {
        SBI_SRST_RESET_TYPE_SHUTDOWN => SbiResponse::with_action(SbiAction::StopCurrentVm),
        // PR #56 does not claim reboot support until Hypocaust can reconstruct
        // RAM, devices, CSRs, and the vCPU boot context atomically.
        SBI_SRST_RESET_TYPE_COLD_REBOOT | SBI_SRST_RESET_TYPE_WARM_REBOOT => {
            SbiResponse::not_supported()
        }
        _ => SbiResponse::invalid_param(),
    }
}

/// Exercise the ABI decoder before any Guest can issue an ecall.
pub fn self_test() {
    assert_eq!(
        dispatch_modern(SBI_EXT_BASE, SBI_BASE_GET_SPEC_VERSION, [0; 6]),
        Some(SbiResponse::success(SBI_SPEC_VERSION_2_0)),
    );
    assert_eq!(
        dispatch_modern(
            SBI_EXT_BASE,
            SBI_BASE_PROBE_EXTENSION,
            [SBI_EXT_TIME, 0, 0, 0, 0, 0],
        ),
        Some(SbiResponse::success(1)),
    );
    assert_eq!(
        dispatch_modern(
            SBI_EXT_BASE,
            SBI_BASE_PROBE_EXTENSION,
            [SBI_EXT_DBCN, 0, 0, 0, 0, 0],
        ),
        Some(SbiResponse::success(1)),
    );
    assert_eq!(
        dispatch_modern(
            SBI_EXT_BASE,
            SBI_BASE_PROBE_EXTENSION,
            [SBI_EXT_SRST, 0, 0, 0, 0, 0],
        ),
        Some(SbiResponse::success(1)),
    );
    assert_eq!(
        dispatch_modern(SBI_EXT_TIME, SBI_TIME_SET_TIMER, [1234, 0, 0, 0, 0, 0]),
        Some(SbiResponse::with_action(SbiAction::SetTimer(1234))),
    );
    assert_eq!(dispatch_modern(SBI_SET_TIMER, 0, [0; 6]), None);
    assert_eq!(
        dispatch_modern(
            SBI_EXT_SRST,
            SBI_SRST_SYSTEM_RESET,
            [
                SBI_SRST_RESET_TYPE_SHUTDOWN,
                SBI_SRST_RESET_REASON_NONE,
                0,
                0,
                0,
                0,
            ],
        ),
        Some(SbiResponse::with_action(SbiAction::StopCurrentVm)),
    );
    assert_eq!(
        dispatch_modern(
            SBI_EXT_SRST,
            SBI_SRST_SYSTEM_RESET,
            [
                SBI_SRST_RESET_TYPE_COLD_REBOOT,
                SBI_SRST_RESET_REASON_NONE,
                0,
                0,
                0,
                0,
            ],
        ),
        Some(SbiResponse::not_supported()),
    );
    assert_eq!(
        dispatch_modern(
            SBI_EXT_DBCN,
            SBI_DBCN_CONSOLE_WRITE,
            [3, 0x8020_0000, 0, 0, 0, 0],
        ),
        Some(SbiResponse::with_action(SbiAction::DebugConsoleWrite {
            num_bytes: 3,
            base_addr_lo: 0x8020_0000,
            base_addr_hi: 0,
        })),
    );
    assert_eq!(
        dispatch_modern(
            SBI_EXT_DBCN,
            SBI_DBCN_CONSOLE_READ,
            [4, 0x8030_0000, 0, 0, 0, 0],
        ),
        Some(SbiResponse::with_action(SbiAction::DebugConsoleRead {
            num_bytes: 4,
            base_addr_lo: 0x8030_0000,
            base_addr_hi: 0,
        })),
    );
    assert_eq!(
        dispatch_modern(
            SBI_EXT_DBCN,
            SBI_DBCN_CONSOLE_WRITE_BYTE,
            [b'!' as usize, 0, 0, 0, 0, 0],
        ),
        Some(SbiResponse::with_action(SbiAction::DebugConsoleWriteByte(b'!'))),
    );
}
