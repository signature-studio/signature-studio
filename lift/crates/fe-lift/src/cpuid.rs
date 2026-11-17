//! CPUID: the lifted body calls the runtime (runtime/fle_cpuid.c), which holds the response.
//!
//! The runtime reads EAX and ECX from R[0] and R[1] and writes EAX, EBX, ECX, EDX
//! to R[0], R[3], R[1], R[2]. The response does not depend on the inputs. See
//! docs/decisions/0009-cpuid.md for the value and its basis.

use iced_x86::{Instruction, Mnemonic};

type Lift = Result<(&'static str, String), String>;

pub fn lift(i: &Instruction) -> Option<Lift> {
    match i.mnemonic() {
        Mnemonic::Cpuid => Some(cpuid(i)),
        Mnemonic::Xgetbv => Some(xgetbv(i)),
        _ => None,
    }
}

/// XGETBV is a named stop, not an implementation (decision 0047): the SDM does not give an XCR0 value, and the
/// runtime CPUID (runtime/fle_cpuid.c) reports no OSXSAVE, so a guest that checks CPUID.1:ECX never reaches it.
/// Reaching it means the CPUID answer changed; the run stops with a message that names the instruction and the VA.
fn xgetbv(i: &Instruction) -> Lift {
    if i.op_count() != 0 {
        return Err("xgetbv operand count".into());
    }
    Ok(("xgetbv_stop", format!("{{ void fle_xgetbv_stop(uint64_t); fle_xgetbv_stop(UINT64_C({:#x})); }}", i.ip())))
}

fn cpuid(i: &Instruction) -> Lift {
    if i.op_count() != 0 {
        return Err("cpuid operand count".into());
    }
    Ok(("cpuid", "{ void fle_cpuid(void); fle_cpuid(); }".into()))
}
