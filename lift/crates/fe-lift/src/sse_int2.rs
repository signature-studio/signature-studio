//! Packed SSE2 integer instructions on the XMM registers (xmm forms only; the MMX forms
//! and the VEX/EVEX forms are not lifted here): PADDQ, PADDSB, PADDSW, PADDUSB, PADDUSW,
//! PSUBSB, PSUBSW, PSUBUSB, PSUBUSW, PMAXSW, PMINSW, PMULHW, PMULLW, PACKSSWB, PACKSSDW,
//! PSLLW/D/Q, PSRLW/D/Q, PSRAW/D (xmm, xmm/m128 and xmm, imm8), PSHUFLW and PSHUFHW.
//!
//! Semantics: Intel SDM Vol. 2 (325383-093US), the 128-bit Legacy SSE pseudo-code of each
//! instruction. The helpers are in runtime/fle_sse_int2.h. No flags change. A packed
//! instruction writes the whole destination register.
//!
//! Alignment: the SDM exception table for these instructions (Table 2-21, Type 4) lists
//! "#GP(0) Legacy SSE: Memory operand is not 16-byte aligned", and the shift instructions'
//! xmm/m128 count form is in the same class (Type 4 for the RM encoding). The 128-bit
//! memory operand is therefore read with the aligned load (`fle_ldxa`), which aborts on a
//! misaligned address, as sse_pack.rs and sse_more.rs do. Decision 0026.
//!
//! Shifts: the count operand is the low 64 bits (SDM LOGICAL_*_SHIFT_*, COUNT := COUNT_SRC[63:0]);
//! a logical shift with a count above width - 1 gives 0, an arithmetic shift clamps the
//! count (to 16 for words, 32 for doublewords; the result equals a shift by width - 1).

use iced_x86::{Instruction, Mnemonic, OpKind, Register};

type Lift = Result<(&'static str, String), String>;

pub fn lift(i: &Instruction) -> Option<Lift> {
    if (0..i.op_count()).any(|k| i.op_kind(k) == OpKind::Register && i.op_register(k).is_mm()) {
        return None;
    }
    let r = match i.mnemonic() {
        Mnemonic::Paddq => bin(i, "paddq", "fle_i2_paddq"),
        Mnemonic::Paddsb => bin(i, "paddsb", "fle_i2_paddsb"),
        Mnemonic::Paddsw => bin(i, "paddsw", "fle_i2_paddsw"),
        Mnemonic::Paddusb => bin(i, "paddusb", "fle_i2_paddusb"),
        Mnemonic::Paddusw => bin(i, "paddusw", "fle_i2_paddusw"),
        Mnemonic::Psubsb => bin(i, "psubsb", "fle_i2_psubsb"),
        Mnemonic::Psubsw => bin(i, "psubsw", "fle_i2_psubsw"),
        Mnemonic::Psubusb => bin(i, "psubusb", "fle_i2_psubusb"),
        Mnemonic::Psubusw => bin(i, "psubusw", "fle_i2_psubusw"),
        Mnemonic::Pmaxsw => bin(i, "pmaxsw", "fle_i2_pmaxsw"),
        Mnemonic::Pminsw => bin(i, "pminsw", "fle_i2_pminsw"),
        Mnemonic::Pmulhw => bin(i, "pmulhw", "fle_i2_pmulhw"),
        Mnemonic::Pmullw => bin(i, "pmullw", "fle_i2_pmullw"),
        Mnemonic::Psubw => bin(i, "psubw", "fle_i2_psubw"),
        Mnemonic::Pcmpgtd => bin(i, "pcmpgtd", "fle_i2_pcmpgtd"),
        Mnemonic::Rcpps => bin(i, "rcpps", "fle_i2_rcpps"),
        Mnemonic::Packsswb => bin(i, "packsswb", "fle_i2_packsswb"),
        Mnemonic::Packssdw => bin(i, "packssdw", "fle_i2_packssdw"),
        Mnemonic::Psllw => shift(i, "psllw", "fle_i2_psllw"),
        Mnemonic::Pslld => shift(i, "pslld", "fle_i2_pslld"),
        Mnemonic::Psllq => shift(i, "psllq", "fle_i2_psllq"),
        Mnemonic::Psrlw => shift(i, "psrlw", "fle_i2_psrlw"),
        Mnemonic::Psrld => shift(i, "psrld", "fle_i2_psrld"),
        Mnemonic::Psrlq => shift(i, "psrlq", "fle_i2_psrlq"),
        Mnemonic::Psraw => shift(i, "psraw", "fle_i2_psraw"),
        Mnemonic::Psrad => shift(i, "psrad", "fle_i2_psrad"),
        Mnemonic::Pshuflw => shuf(i, "pshuflw", "fle_i2_pshuflw"),
        Mnemonic::Pshufhw => shuf(i, "pshufhw", "fle_i2_pshufhw"),
        _ => return None,
    };
    Some(r)
}

fn need(c: bool, msg: &str) -> Result<(), String> {
    if c {
        Ok(())
    } else {
        Err(msg.to_string())
    }
}

fn xmm(r: Register) -> Result<usize, String> {
    need(r.is_xmm() && r.number() < 16, "register is not XMM0-15")?;
    Ok(r.number())
}

fn dst(i: &Instruction) -> Result<usize, String> {
    need(i.op_kind(0) == OpKind::Register, "destination must be a register")?;
    xmm(i.op_register(0))
}

/// Source operand as a 128-bit C expression: an XMM register, or 16 bytes of memory (aligned).
fn src(i: &Instruction, n: u32) -> Result<String, String> {
    match i.op_kind(n) {
        OpKind::Register => Ok(format!("X[{}]", xmm(i.op_register(n))?)),
        OpKind::Memory => Ok(format!("fle_ldxa({})", ea(i)?)),
        _ => Err("operand kind".into()),
    }
}

/// Two-operand packed operation: dst = f(dst, src).
fn bin(i: &Instruction, name: &'static str, f: &str) -> Lift {
    need(i.op_count() == 2, "operand count")?;
    let d = dst(i)?;
    Ok((name, format!("X[{d}] = {f}(X[{d}], {});", src(i, 1)?)))
}

/// Shift by the low 64 bits of an XMM or m128 count, or by imm8.
fn shift(i: &Instruction, name: &'static str, f: &str) -> Lift {
    need(i.op_count() == 2, "operand count")?;
    let d = dst(i)?;
    let cnt = match i.op_kind(1) {
        OpKind::Immediate8 => format!("UINT64_C({:#x})", i.immediate8()),
        OpKind::Register | OpKind::Memory => format!("{}.u64[0]", src(i, 1)?),
        _ => return Err("operand kind".into()),
    };
    Ok((name, format!("X[{d}] = {f}(X[{d}], {cnt});")))
}

fn shuf(i: &Instruction, name: &'static str, f: &str) -> Lift {
    need(i.op_count() == 3 && i.op_kind(2) == OpKind::Immediate8, "shuffle operand form")?;
    let d = dst(i)?;
    Ok((name, format!("X[{d}] = {f}({}, {:#x});", src(i, 1)?, i.immediate8())))
}

/// Effective address as a C expression. Never loads from memory.
/// Same rules as sse_pack.rs: no FS/GS, and 64-bit addressing only.
fn ea(i: &Instruction) -> Result<String, String> {
    need(!matches!(i.memory_segment(), Register::FS | Register::GS), "FS/GS segment")?;
    if i.is_ip_rel_memory_operand() {
        return Ok(format!("UINT64_C({:#x})", i.ip_rel_memory_address()));
    }
    let mut terms: Vec<String> = Vec::new();
    let base = i.memory_base();
    if base != Register::None {
        need(base.size() == 8, "32-bit addressing")?;
        terms.push(format!("R[{}]", base.full_register().number()));
    }
    let idx = i.memory_index();
    if idx != Register::None {
        need(idx.size() == 8, "32-bit addressing")?;
        terms.push(format!("(R[{}]*{})", idx.full_register().number(), i.memory_index_scale()));
    }
    let d = i.memory_displacement64();
    if d != 0 || terms.is_empty() {
        terms.push(format!("UINT64_C({d:#x})"));
    }
    Ok(format!("({})", terms.join(" + ")))
}
