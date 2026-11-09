//! Packed SSE and SSE2 arithmetic on XMM registers: PACKUSWB, PADDW, ADDPS,
//! SUBPS, MULPS, MINPS and MAXPS (xmm, xmm/m128 forms). The MMX forms are not lifted.
//!
//! Register model and alignment as in sse_pack.rs: a packed instruction writes the
//! whole destination register; a 128-bit legacy SSE memory operand is read with the
//! aligned load (the 16-byte rule is reused from sse_pack.rs and not re-checked
//! here). No flags change. Each float lane follows the scalar helper of the same
//! operation in runtime/fle_sse.h (NaN, MIN and MAX rules from the SDM pseudo-code).

use iced_x86::{Instruction, Mnemonic, OpKind, Register};

// Decision 0038: SSE floating point with MXCSR flags, and PREFETCHh. Declared here so that lib.rs stays untouched.
#[path = "prefetch.rs"]
mod prefetch;
#[path = "sse_fp_flags.rs"]
mod sse_fp_flags;
// Decision 0075: legacy SSE/SSE2 forms used by dsp_ippv2_x64.dll (ORPS, PSRLDQ, PMOVMSKB, ...).
#[path = "sse_legacy.rs"]
mod sse_legacy;

type Lift = Result<(&'static str, String), String>;

pub fn lift(i: &Instruction) -> Option<Lift> {
    if (0..i.op_count()).any(|k| i.op_kind(k) == OpKind::Register && i.op_register(k).is_mm()) {
        return None;
    }
    if let Some(r) = sse_fp_flags::lift(i) {
        return Some(r);
    }
    if let Some(r) = prefetch::lift(i) {
        return Some(r);
    }
    if let Some(r) = sse_legacy::lift(i) {
        return Some(r);
    }
    let r = match i.mnemonic() {
        Mnemonic::Packuswb => bin(i, "packuswb", "fle_packuswb"),
        Mnemonic::Paddw => bin(i, "paddw", "fle_paddw"),
        Mnemonic::Addps => bin(i, "addps", "fle_addps"),
        Mnemonic::Subps => bin(i, "subps", "fle_subps"),
        Mnemonic::Mulps => bin(i, "mulps", "fle_mulps"),
        Mnemonic::Minps => bin(i, "minps", "fle_minps"),
        Mnemonic::Maxps => bin(i, "maxps", "fle_maxps"),
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

/// Two-operand packed operation: dst = f(dst, src). The prototype is declared in the block.
fn bin(i: &Instruction, name: &'static str, f: &str) -> Lift {
    need(i.op_count() == 2, "operand count")?;
    let d = dst(i)?;
    Ok((name, format!("{{ fle_xmm {f}(fle_xmm, fle_xmm); X[{d}] = {f}(X[{d}], {}); }}", src(i, 1)?)))
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
