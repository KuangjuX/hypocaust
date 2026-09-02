//! Guest-facing RISC-V Supervisor Binary Interface dispatch.
//!
//! Legacy extension IDs remain available for the xv6-rust example. PR #54
//! (`feature/sbi-v02-base-time`) adds the SBI v0.2 BASE and TIME contract used
//! by Linux without exposing the Host firmware implementation to a Guest.

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

const SBI_BASE_GET_SPEC_VERSION: usize = 0;
const SBI_BASE_GET_IMPL_ID: usize = 1;
const SBI_BASE_GET_IMPL_VERSION: usize = 2;
const SBI_BASE_PROBE_EXTENSION: usize = 3;
const SBI_BASE_GET_MVENDORID: usize = 4;
const SBI_BASE_GET_MARCHID: usize = 5;
const SBI_BASE_GET_MIMPID: usize = 6;
const SBI_TIME_SET_TIMER: usize = 0;

const SBI_SUCCESS: usize = 0;
const SBI_ERR_NOT_SUPPORTED: usize = (-2isize) as usize;
const SBI_SPEC_VERSION_0_2: usize = 2;
// PR #54 uses a private, human-readable implementation identifier (`HYPO`).
// Linux treats this value as informational and does not branch on it.
const SBI_IMPL_ID_HYPOCAUST: usize = 0x4859_504f;
const SBI_IMPL_VERSION: usize = 0x0001_0000;

/// Host-side work requested by a successfully decoded Guest SBI call.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SbiAction {
    None,
    SetTimer(usize),
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
}

/// Decode a modern SBI call. `None` reserves extension IDs 0..=8 for the
/// existing legacy ABI, whose return convention differs from SBI v0.2.
///
/// PR #54 deliberately advertises only extensions implemented end-to-end.
/// Linux may probe IPI, RFENCE, HSM, SRST, and DBCN, but receives zero until
/// the corresponding VM-local operation is added by a later PR.
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
        _ => SbiResponse::not_supported(),
    };
    Some(response)
}

fn dispatch_base(function_id: usize, argument0: usize) -> SbiResponse {
    match function_id {
        SBI_BASE_GET_SPEC_VERSION => SbiResponse::success(SBI_SPEC_VERSION_0_2),
        SBI_BASE_GET_IMPL_ID => SbiResponse::success(SBI_IMPL_ID_HYPOCAUST),
        SBI_BASE_GET_IMPL_VERSION => SbiResponse::success(SBI_IMPL_VERSION),
        SBI_BASE_PROBE_EXTENSION => SbiResponse::success(usize::from(matches!(
            argument0,
            SBI_EXT_BASE | SBI_EXT_TIME
        ))),
        // PR #54 cannot read M-mode identification CSRs while Hypocaust runs
        // in HS/S-mode. SBI permits zero when platform IDs are unavailable.
        SBI_BASE_GET_MVENDORID | SBI_BASE_GET_MARCHID | SBI_BASE_GET_MIMPID => {
            SbiResponse::success(0)
        }
        _ => SbiResponse::not_supported(),
    }
}

/// Exercise the ABI decoder before any Guest can issue an ecall.
pub fn self_test() {
    assert_eq!(
        dispatch_modern(SBI_EXT_BASE, SBI_BASE_GET_SPEC_VERSION, [0; 6]),
        Some(SbiResponse::success(SBI_SPEC_VERSION_0_2)),
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
            [0x5352_5354, 0, 0, 0, 0, 0],
        ),
        Some(SbiResponse::success(0)),
    );
    assert_eq!(
        dispatch_modern(SBI_EXT_TIME, SBI_TIME_SET_TIMER, [1234, 0, 0, 0, 0, 0]),
        Some(SbiResponse::with_action(SbiAction::SetTimer(1234))),
    );
    assert_eq!(dispatch_modern(SBI_SET_TIMER, 0, [0; 6]), None);
}
