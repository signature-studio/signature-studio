//! Decision 0190 (wrp2, PROVISIONAL, additive): the SSSE3 integer instructions that Elastique_x64.dll executes without asking CPUID (xmm forms, legacy encoding):
//! PALIGNR xmm, xmm/m128, imm8; PABSB, PABSW, PABSD xmm, xmm/m128; PSHUFB xmm, xmm/m128. The MMX (mm register) forms are not lifted here (they stop by name as before).
//!
//! Semantics: Intel SDM Vol. 2 pseudo-code of each instruction (128-bit Legacy SSE form). The helpers are in runtime/fle_ssse3.h (header only, included at the end of fle_sse.h). No flags change.
//! Alignment: all four instructions point to Table 2-21 (Type 4) "Exceptions Type 4": #GP(0) when the m128 operand is not 16-byte aligned, so the memory operand is read with the aligned load (`fle_ldxa`).
//! The other SSE3 instructions of Elastique (MOVDDUP, MOVSHDUP, MOVSLDUP, HADDPS, ADDSUBPS, ADDSUBPD) are another worker's addition to sse_shuf.rs (decision 0191) and are not here.

use iced_x86::{Instruction, Mnemonic, OpKind, Register};

type Lift = Result<(&'static str, String), String>;

pub fn lift(i: &Instruction) -> Option<Lift> {
    let r = match i.mnemonic() {
        Mnemonic::Palignr => palignr(i),
        Mnemonic::Pabsb => unary(i, "pabsb", "fle_pabsb"),
        Mnemonic::Pabsw => unary(i, "pabsw", "fle_pabsw"),
        Mnemonic::Pabsd => unary(i, "pabsd", "fle_pabsd"),
        Mnemonic::Pshufb => pshufb(i),
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
    need(r.is_xmm() && r.number() < 16, "register is not XMM0-15 (MMX form)")?;
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

/// PABSB, PABSW, PABSD: dst = |src| lane by lane (the most negative value stays).
fn unary(i: &Instruction, name: &'static str, f: &str) -> Lift {
    need(i.op_count() == 2, "operand count")?;
    let d = dst(i)?;
    Ok((name, format!("X[{d}] = {f}({});", src(i, 1)?)))
}

/// PSHUFB: dst byte k = 0 when mask byte bit 7 is set, else the dst byte selected by mask bits 3:0 (mask = the source).
fn pshufb(i: &Instruction) -> Lift {
    need(i.op_count() == 2, "operand count")?;
    let d = dst(i)?;
    Ok(("pshufb", format!("X[{d}] = fle_pshufb(X[{d}], {});", src(i, 1)?)))
}

/// PALIGNR: the 32-byte concatenation dst:src shifted right by imm8 bytes, low 16 bytes kept (imm8 > 31: zero).
fn palignr(i: &Instruction) -> Lift {
    need(i.op_count() == 3 && i.op_kind(2) == OpKind::Immediate8, "palignr operand form")?;
    let d = dst(i)?;
    Ok(("palignr", format!("X[{d}] = fle_palignr(X[{d}], {}, {:#x});", src(i, 1)?, i.immediate8())))
}

/// Effective address as a C expression. Never loads from memory. Same rules as sse_shuf.rs: no FS/GS, and 64-bit addressing only.
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
