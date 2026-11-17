//! STMXCSR and LDMXCSR: store and load the MXCSR control and status register.
//!
//! MXCSR is one runtime variable (runtime/fle_mxcsr.c). STMXCSR writes its 32 bits
//! to m32. LDMXCSR reads m32 and aborts if a reserved bit (31:16) is set, which is
//! the #GP case in SDM Vol. 2 (LDMXCSR, Other Exceptions).
//!
//! Only the stored value changes. The SSE helpers in runtime/fle_sse.h still use
//! the default rounding and masking, so a loaded rounding mode is not applied to
//! arithmetic.

use iced_x86::{Instruction, Mnemonic, OpKind};

type Lift = Result<(&'static str, String), String>;

pub fn lift(i: &Instruction) -> Option<Lift> {
    match i.mnemonic() {
        Mnemonic::Stmxcsr => Some(stmxcsr(i)),
        Mnemonic::Ldmxcsr => Some(ldmxcsr(i)),
        _ => None,
    }
}

fn need(c: bool, msg: &str) -> Result<(), String> {
    if c {
        Ok(())
    } else {
        Err(msg.to_string())
    }
}

/// The m32 operand as a C address expression.
fn m32(i: &Instruction) -> Result<String, String> {
    need(i.op_count() == 1 && i.op_kind(0) == OpKind::Memory, "operand must be m32")?;
    need(i.memory_size().size() == 4, "operand must be 32 bits")?;
    super::ea(i)
}

fn stmxcsr(i: &Instruction) -> Lift {
    let a = m32(i)?;
    Ok(("stmxcsr", format!("{{ void fle_mxcsr_store(uint64_t); fle_mxcsr_store({a}); }}")))
}

fn ldmxcsr(i: &Instruction) -> Lift {
    let a = m32(i)?;
    Ok(("ldmxcsr", format!("{{ void fle_mxcsr_load(uint64_t); fle_mxcsr_load({a}); }}")))
}
