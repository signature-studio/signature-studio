//! MMX instructions on the mm registers (decision 0028). Lifted: EMMS, MOVD, MOVQ (mm with a
//! GPR, memory or mm), PXOR, PADDW, PSUBSW, PMULHW, PMULHUW, PACKUSWB, PUNPCKLBW, PSHUFW
//! (mm, mm/m64, imm8) and PSLLW (mm, mm/m64 and mm, imm8). The xmm and VEX forms of the same
//! mnemonics belong to sse_pack.rs, sse_more.rs and sse_int2.rs; this module returns `None`
//! for them. Other MMX mnemonics are not lifted and stay unsupported.
//!
//! Semantics: Intel SDM Vol. 2 (325383-093US), the "With 64-bit Operands" pseudo-code. The
//! helpers are in runtime/fle_mmx.h. State: `MM[n]` is MMn (independent of the x87 registers).
//! Every MMX instruction other than EMMS also sets x87 TOP to 0 and the tag word to 0
//! (valid); EMMS sets TOP to 0 and the tag word to FFFFH (SDM Vol. 3A 15.2, Table 15-2).
//! No flags change.
//!
//! Memory operands are 64 bits and have no alignment requirement (SDM Vol. 3B Table 25-7
//! lists only #AC, and only with alignment checking on at CPL 3), so `fle_ld` / `fle_st` are
//! used, as for MOVD and MOVQ in sse_pack.rs.

use iced_x86::{Instruction, Mnemonic, OpKind, Register};

type Lift = Result<(&'static str, String), String>;

const TOUCH: &str = "fle_mmx_touch();";

pub fn lift(i: &Instruction) -> Option<Lift> {
    if i.mnemonic() == Mnemonic::Emms {
        return Some(emms(i));
    }
    if !(0..i.op_count()).any(|k| i.op_kind(k) == OpKind::Register && i.op_register(k).is_mm()) {
        return None;
    }
    let r = match i.mnemonic() {
        Mnemonic::Pxor => bin(i, "mmx_pxor", "fle_mmx_pxor"),
        Mnemonic::Paddw => bin(i, "mmx_paddw", "fle_mmx_paddw"),
        Mnemonic::Psubsw => bin(i, "mmx_psubsw", "fle_mmx_psubsw"),
        Mnemonic::Pmulhw => bin(i, "mmx_pmulhw", "fle_mmx_pmulhw"),
        Mnemonic::Pmulhuw => bin(i, "mmx_pmulhuw", "fle_mmx_pmulhuw"),
        Mnemonic::Packuswb => bin(i, "mmx_packuswb", "fle_mmx_packuswb"),
        Mnemonic::Punpcklbw => bin(i, "mmx_punpcklbw", "fle_mmx_punpcklbw"),
        Mnemonic::Psllw => shift(i, "mmx_psllw", "fle_mmx_psllw"),
        Mnemonic::Pshufw => pshufw(i),
        Mnemonic::Movd => mov(i, "mmx_movd", 4),
        Mnemonic::Movq => mov(i, "mmx_movq", 8),
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

fn emms(i: &Instruction) -> Lift {
    need(i.op_count() == 0, "emms operand count")?;
    Ok(("mmx_emms", "fle_mmx_emms();".to_string()))
}

fn mm(r: Register) -> Result<usize, String> {
    need(r.is_mm() && r.number() < 8, "register is not MM0-7")?;
    Ok(r.number())
}

fn gpr(r: Register, size: usize) -> Result<usize, String> {
    need(r.is_gpr() && r.size() == size, "GPR size does not match the operation")?;
    need(!matches!(r, Register::AH | Register::CH | Register::DH | Register::BH), "high-byte register")?;
    Ok(r.full_register().number())
}

fn dst(i: &Instruction) -> Result<usize, String> {
    need(i.op_kind(0) == OpKind::Register, "destination must be a register")?;
    mm(i.op_register(0))
}

/// Source operand as a 64-bit C expression: an MM register, or `msz` bytes of memory zero-extended
/// (no alignment rule). `msz` is 8, except PUNPCKLBW, whose memory form is m32 (SDM: "mm, mm/m32").
fn src_sz(i: &Instruction, n: u32, msz: usize) -> Result<String, String> {
    match i.op_kind(n) {
        OpKind::Register => Ok(format!("MM[{}]", mm(i.op_register(n))?)),
        OpKind::Memory => {
            need(i.memory_size().size() == msz, "memory operand size")?;
            Ok(format!("fle_ld({}, {msz})", ea(i)?))
        }
        _ => Err("operand kind".into()),
    }
}

fn src(i: &Instruction, n: u32) -> Result<String, String> {
    src_sz(i, n, 8)
}

/// Two-operand packed operation: dst = f(dst, src).
fn bin(i: &Instruction, name: &'static str, f: &str) -> Lift {
    need(i.op_count() == 2, "operand count")?;
    let d = dst(i)?;
    let msz = if i.mnemonic() == Mnemonic::Punpcklbw { 4 } else { 8 };
    Ok((name, format!("{TOUCH} MM[{d}] = {f}(MM[{d}], {});", src_sz(i, 1, msz)?)))
}

/// PSLLW by the 64-bit mm or m64 count, or by imm8.
fn shift(i: &Instruction, name: &'static str, f: &str) -> Lift {
    need(i.op_count() == 2, "operand count")?;
    let d = dst(i)?;
    let cnt = match i.op_kind(1) {
        OpKind::Immediate8 => format!("UINT64_C({:#x})", i.immediate8()),
        OpKind::Register | OpKind::Memory => src(i, 1)?,
        _ => return Err("operand kind".into()),
    };
    Ok((name, format!("{TOUCH} MM[{d}] = {f}(MM[{d}], {cnt});")))
}

fn pshufw(i: &Instruction) -> Lift {
    need(i.op_count() == 3 && i.op_kind(2) == OpKind::Immediate8, "pshufw operand form")?;
    let d = dst(i)?;
    Ok(("mmx_pshufw", format!("{TOUCH} MM[{d}] = fle_mmx_pshufw({}, {:#x});", src(i, 1)?, i.immediate8())))
}

/// MOVD (size 4) and MOVQ (size 8) between an MM register and a GPR, memory, or (MOVQ) another
/// MM register. A 4-byte value moved into an MM register is zero-extended to 64 bits; a 4-byte
/// value moved into a GPR zero-extends to 64 bits.
fn mov(i: &Instruction, name: &'static str, size: usize) -> Lift {
    need(i.op_count() == 2, "operand count")?;
    let is_mm = |n: u32| i.op_kind(n) == OpKind::Register && i.op_register(n).is_mm();
    let body = if is_mm(0) {
        let d = mm(i.op_register(0))?;
        match i.op_kind(1) {
            OpKind::Register if is_mm(1) => {
                need(size == 8, "movd between MM registers")?;
                format!("MM[{d}] = MM[{}];", mm(i.op_register(1))?)
            }
            OpKind::Register => {
                let n = gpr(i.op_register(1), size)?;
                if size == 4 {
                    format!("MM[{d}] = (uint32_t)R[{n}];")
                } else {
                    format!("MM[{d}] = R[{n}];")
                }
            }
            OpKind::Memory => {
                need(i.memory_size().size() == size, "memory operand size")?;
                format!("MM[{d}] = fle_ld({}, {size});", ea(i)?)
            }
            _ => return Err("operand kind".into()),
        }
    } else {
        need(is_mm(1), "source must be an MM register")?;
        let s = mm(i.op_register(1))?;
        match i.op_kind(0) {
            OpKind::Register => {
                let n = gpr(i.op_register(0), size)?;
                if size == 4 {
                    format!("R[{n}] = (uint32_t)MM[{s}];")
                } else {
                    format!("R[{n}] = MM[{s}];")
                }
            }
            OpKind::Memory => {
                need(i.memory_size().size() == size, "memory operand size")?;
                format!("fle_st({}, {size}, MM[{s}]);", ea(i)?)
            }
            _ => return Err("operand kind".into()),
        }
    };
    Ok((name, format!("{TOUCH} {body}")))
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
