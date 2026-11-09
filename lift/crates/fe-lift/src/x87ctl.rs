//! x87 control instructions: FNSTCW, FLDCW, FNSTSW, FNCLEX, FNINIT (and the
//! FSTCW, FSTSW, FCLEX, FINIT forms with the FWAIT prefix, and FWAIT itself).
//!
//! The x87 control, status and tag words are one struct in the runtime
//! (runtime/fle_x87ctl.c). FLDCW keeps the value the host CPU reads back,
//! `(v & 0x1F3F) | 0x0040`, and never aborts. x87 arithmetic is not lifted, so it
//! still stops at `fle_unsupported`. A precision or rounding control that differs
//! from the default is only stored.
//!
//! The status word never gets an exception flag, because nothing sets one. So the
//! "wait" forms have nothing to check and equal the "no-wait" forms, and FWAIT does nothing.

use iced_x86::{Instruction, Mnemonic, OpKind, Register};

type Lift = Result<(&'static str, String), String>;

pub fn lift(i: &Instruction) -> Option<Lift> {
    match i.mnemonic() {
        Mnemonic::Fnstcw | Mnemonic::Fstcw => Some(stcw(i)),
        Mnemonic::Fldcw => Some(ldcw(i)),
        Mnemonic::Fnstsw | Mnemonic::Fstsw => Some(stsw(i)),
        Mnemonic::Fnclex | Mnemonic::Fclex => Some(no_operands(i, "fnclex", "{ void fle_x87_clex(void); fle_x87_clex(); }")),
        Mnemonic::Fninit | Mnemonic::Finit => Some(no_operands(i, "fninit", "{ void fle_x87_init(void); fle_x87_init(); }")),
        Mnemonic::Wait => Some(no_operands(i, "fwait", "")),
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

/// The m16 operand as a C address expression.
fn m16(i: &Instruction) -> Result<String, String> {
    need(i.op_count() == 1 && i.op_kind(0) == OpKind::Memory, "operand must be m16")?;
    need(i.memory_size().size() == 2, "operand must be 16 bits")?;
    super::ea(i)
}

fn no_operands(i: &Instruction, handler: &'static str, body: &str) -> Lift {
    need(i.op_count() == 0, "unexpected operand")?;
    Ok((handler, body.to_string()))
}

fn stcw(i: &Instruction) -> Lift {
    let a = m16(i)?;
    Ok(("fnstcw", format!("{{ void fle_x87_store_cw(uint64_t); fle_x87_store_cw({a}); }}")))
}

fn ldcw(i: &Instruction) -> Lift {
    let a = m16(i)?;
    Ok(("fldcw", format!("{{ void fle_x87_load_cw(uint64_t); fle_x87_load_cw({a}); }}")))
}

fn stsw(i: &Instruction) -> Lift {
    need(i.op_count() == 1, "operand count")?;
    match i.op_kind(0) {
        OpKind::Register => {
            need(i.op_register(0) == Register::AX, "register operand must be AX")?;
            Ok((
                "fnstsw",
                "{ uint16_t fle_x87_sw(void); R[0] = (R[0] & ~UINT64_C(0xffff)) | fle_x87_sw(); }".to_string(),
            ))
        }
        _ => {
            let a = m16(i)?;
            Ok(("fnstsw", format!("{{ void fle_x87_store_sw(uint64_t); fle_x87_store_sw({a}); }}")))
        }
    }
}
