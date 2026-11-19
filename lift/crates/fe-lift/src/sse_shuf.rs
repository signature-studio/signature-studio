//! Floating-point SSE shuffles and half moves on the XMM registers (xmm forms only; the VEX
//! forms are other mnemonics and are not lifted here): SHUFPS, SHUFPD, UNPCKLPS, UNPCKHPS,
//! UNPCKLPD, UNPCKHPD (xmm, xmm/m128), MOVHLPS and MOVLHPS (xmm, xmm), and MOVLPS, MOVHPS,
//! MOVLPD, MOVHPD (xmm, m64 load and m64, xmm store).
//!
//! Semantics: Intel SDM Vol. 2 (325383-093US), the 128-bit Legacy SSE pseudo-code of each
//! instruction. The helpers are in runtime/fle_sse_shuf.h. No flags change. The lanes are
//! copied as bits (no floating-point operation, so no NaN quieting).
//!
//! Alignment (SDM "Other Exceptions" of each instruction): SHUFPS, SHUFPD and UNPCK* point
//! to Table 2-21 (Type 4), which has "#GP(0) Legacy SSE: Memory operand is not 16-byte
//! aligned", so the m128 operand is read with the aligned load (`fle_ldxa`, abort when
//! misaligned). MOVLPS, MOVHPS, MOVLPD and MOVHPD point to Table 2-22 (Type 5, "<16 Byte Mem
//! Arg"), which has no alignment #GP, so the 8-byte memory operand is accessed with
//! `fle_ld` / `fle_st` at any address. MOVHLPS and MOVLHPS are register only (ModRM.MOD = 11b).
//! Decision 0027.
//!
//! The 64-bit moves change only one quadword of the destination register: MOVLPS and MOVLPD
//! write bits 63:0, MOVHPS and MOVHPD write bits 127:64, MOVHLPS writes 63:0, MOVLHPS 127:64.

use iced_x86::{Instruction, Mnemonic, OpKind, Register};

type Lift = Result<(&'static str, String), String>;

pub fn lift(i: &Instruction) -> Option<Lift> {
    let r = match i.mnemonic() {
        Mnemonic::Shufps => shuf(i, "shufps", "fle_shufps"),
        Mnemonic::Shufpd => shuf(i, "shufpd", "fle_shufpd"),
        Mnemonic::Unpcklps => bin(i, "unpcklps", "fle_unpcklps"),
        Mnemonic::Unpckhps => bin(i, "unpckhps", "fle_unpckhps"),
        Mnemonic::Unpcklpd => bin(i, "unpcklpd", "fle_unpcklpd"),
        Mnemonic::Unpckhpd => bin(i, "unpckhpd", "fle_unpckhpd"),
        Mnemonic::Movhlps => regreg(i, "movhlps", "fle_movhlps"),
        Mnemonic::Movlhps => regreg(i, "movlhps", "fle_movlhps"),
        Mnemonic::Movlps => half(i, "movlps", 0),
        Mnemonic::Movlpd => half(i, "movlpd", 0),
        Mnemonic::Movhps => half(i, "movhps", 1),
        Mnemonic::Movhpd => half(i, "movhpd", 1),
        // Decision 0191: MOVDDUP, MOVSLDUP, MOVSHDUP (SSE3 moves).
        Mnemonic::Movddup => movddup(i),
        Mnemonic::Movsldup => dup32(i, "movsldup", "fle_movsldup"),
        Mnemonic::Movshdup => dup32(i, "movshdup", "fle_movshdup"),
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

/// SHUFPS and SHUFPD: dst = f(dst, src, imm8).
fn shuf(i: &Instruction, name: &'static str, f: &str) -> Lift {
    need(i.op_count() == 3 && i.op_kind(2) == OpKind::Immediate8, "shuffle operand form")?;
    let d = dst(i)?;
    Ok((name, format!("X[{d}] = {f}(X[{d}], {}, {:#x});", src(i, 1)?, i.immediate8())))
}

/// MOVDDUP xmm, xmm/m64 (decision 0191): the low quadword of the source into both quadwords of the destination. The memory operand is 8 bytes: no alignment rule (SDM Table 2-22, Type 5).
fn movddup(i: &Instruction) -> Lift {
    need(i.op_count() == 2, "operand count")?;
    let d = dst(i)?;
    let body = match i.op_kind(1) {
        OpKind::Register => format!("X[{d}] = fle_movddup(X[{}]);", xmm(i.op_register(1))?),
        OpKind::Memory => {
            need(i.memory_size().size() == 8, "memory operand must be 64 bits")?;
            format!("X[{d}] = fle_movddup_q(fle_ld({}, 8));", ea(i)?)
        }
        _ => return Err("operand kind".into()),
    };
    Ok(("movddup", body))
}

/// MOVSLDUP and MOVSHDUP xmm, xmm/m128 (decision 0191): the m128 operand is read with the aligned load (SDM Table 2-21, Type 4: #GP when not 16-byte aligned).
fn dup32(i: &Instruction, name: &'static str, f: &str) -> Lift {
    need(i.op_count() == 2, "operand count")?;
    let d = dst(i)?;
    Ok((name, format!("X[{d}] = {f}({});", src(i, 1)?)))
}

/// MOVHLPS and MOVLHPS: register to register only.
fn regreg(i: &Instruction, name: &'static str, f: &str) -> Lift {
    need(i.op_count() == 2, "operand count")?;
    need(i.op_kind(1) == OpKind::Register, "register form only")?;
    let d = dst(i)?;
    Ok((name, format!("X[{d}] = {f}(X[{d}], X[{}]);", xmm(i.op_register(1))?)))
}

/// MOVLPS, MOVLPD (q = 0) and MOVHPS, MOVHPD (q = 1): the 64-bit memory operand to or from
/// quadword `q` of the XMM register. The other quadword of a load destination is unchanged.
fn half(i: &Instruction, name: &'static str, q: usize) -> Lift {
    need(i.op_count() == 2, "operand count")?;
    need(i.memory_size().size() == 8, "memory operand must be 64 bits")?;
    let body = match (i.op_kind(0), i.op_kind(1)) {
        (OpKind::Register, OpKind::Memory) => format!("X[{}].u64[{q}] = fle_ld({}, 8);", dst(i)?, ea(i)?),
        (OpKind::Memory, OpKind::Register) => {
            format!("fle_st({}, 8, X[{}].u64[{q}]);", ea(i)?, xmm(i.op_register(1))?)
        }
        _ => return Err("move operand form".into()),
    };
    Ok((name, body))
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
