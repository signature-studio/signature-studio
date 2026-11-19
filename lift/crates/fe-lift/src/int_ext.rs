//! Sign extension (movsx, cbw, cwde, cdqe, cdq, cqo), signed divide (idiv) and
//! the one-operand imul (RDX:RAX form).
//!
//! Flags: movsx and the sign-extension group leave the flags unchanged. idiv
//! leaves all six undefined. One-operand imul sets CF and OF as the SDM says and
//! leaves the other four undefined. See docs/decisions/0010-int-ext.md.
//!
//! Divide errors (zero divisor, quotient out of range) stop the run through
//! fle_div_fault (runtime/fle_int_ext.c), before any register is written.

use iced_x86::{Instruction, Mnemonic, OpKind, Register};

type Lift = Result<(&'static str, String), String>;

/// Lifts the instructions owned by this module. `None` leaves the instruction to `lib.rs`.
pub fn lift(i: &Instruction) -> Option<Lift> {
    match i.mnemonic() {
        Mnemonic::Movsx => Some(movsx(i)),
        Mnemonic::Cbw => Some(Ok(("cbw", "R[0] = (R[0] & ~UINT64_C(0xffff)) | (uint16_t)(int16_t)(int8_t)(uint8_t)R[0];".into()))),
        Mnemonic::Cwde => Some(Ok(("cwde", "R[0] = (uint32_t)(int32_t)(int16_t)(uint16_t)R[0];".into()))),
        Mnemonic::Cdqe => Some(Ok(("cdqe", "R[0] = (uint64_t)(int64_t)(int32_t)(uint32_t)R[0];".into()))),
        Mnemonic::Cdq => Some(Ok(("cdq", "R[2] = (uint32_t)(0u - ((uint32_t)R[0] >> 31));".into()))),
        Mnemonic::Cqo => Some(Ok(("cqo", "R[2] = UINT64_C(0) - (R[0] >> 63);".into()))),
        Mnemonic::Idiv => Some(idiv(i)),
        Mnemonic::Imul if i.op_count() == 1 => Some(imul1(i)),
        _ => None,
    }
}

/// movsx r16/r32/r64, r/m8 or r/m16. The source is sign-extended to the destination width.
fn movsx(i: &Instruction) -> Lift {
    need(i.op_count() == 2 && i.op_kind(0) == OpKind::Register, "movsx operand form")?;
    let d = i.op_register(0);
    need(matches!(d.size(), 2 | 4 | 8), "movsx destination size")?;
    let v = match opsize(i, 1)? {
        1 => format!("(uint64_t)(int64_t)(int8_t)(uint8_t)({})", rd(i, 1, 1)?),
        2 => format!("(uint64_t)(int64_t)(int16_t)(uint16_t)({})", rd(i, 1, 2)?),
        _ => return Err("movsx source size".into()),
    };
    Ok(("movsx", reg_wr(d, &v)?))
}

/// idiv r/m8, r/m16, r/m32, r/m64. The dividend is AX, DX:AX, EDX:EAX or RDX:RAX.
/// The quotient goes to AL, AX, EAX or RAX and the remainder to AH, DX, EDX or RDX.
fn idiv(i: &Instruction) -> Lift {
    need(i.op_count() == 1, "idiv operand count")?;
    let sz = opsize(i, 0)?;
    let w = sz * 8;
    let src = rd(i, 0, sz)?;
    let dvd = match sz {
        1 => "(unsigned __int128)(uint16_t)R[0]",
        2 => "(((unsigned __int128)(uint16_t)R[2] << 16) | (uint16_t)R[0])",
        4 => "(((unsigned __int128)(uint32_t)R[2] << 32) | (uint32_t)R[0])",
        _ => "(((unsigned __int128)R[2] << 64) | R[0])",
    };
    let wr = match sz {
        1 => "R[0] = (R[0] & ~UINT64_C(0xffff)) | ((rem & 0xff) << 8) | (q & 0xff);",
        2 => "R[0] = (R[0] & ~UINT64_C(0xffff)) | q; R[2] = (R[2] & ~UINT64_C(0xffff)) | rem;",
        4 => "R[0] = (uint32_t)q; R[2] = (uint32_t)rem;",
        _ => "R[0] = q; R[2] = rem;",
    };
    Ok((
        "idiv",
        format!(
            "{{ uint64_t fle_idiv(unsigned, unsigned __int128, uint64_t, uint64_t *); uint64_t rem; uint64_t q = fle_idiv({w}, {dvd}, ({src}), &rem); {wr} }}"
        ),
    ))
}

/// imul r/m8, r/m16, r/m32, r/m64 (one operand). The product goes to AX, DX:AX, EDX:EAX or RDX:RAX.
fn imul1(i: &Instruction) -> Lift {
    need(i.op_count() == 1, "imul operand count")?;
    let sz = opsize(i, 0)?;
    let src = rd(i, 0, sz)?;
    let body = match sz {
        1 => format!(
            "{{ uint64_t fle_imul1(unsigned, uint64_t, uint64_t, uint64_t *); uint64_t lo; uint64_t hi = fle_imul1(8, R[0], ({src}), &lo); R[0] = (R[0] & ~UINT64_C(0xffff)) | ((hi & 0xff) << 8) | (lo & 0xff); }}"
        ),
        2 => format!(
            "{{ uint64_t fle_imul1(unsigned, uint64_t, uint64_t, uint64_t *); uint64_t lo; uint64_t hi = fle_imul1(16, R[0], ({src}), &lo); R[0] = (R[0] & ~UINT64_C(0xffff)) | lo; R[2] = (R[2] & ~UINT64_C(0xffff)) | hi; }}"
        ),
        4 => format!(
            "{{ uint64_t fle_imul1(unsigned, uint64_t, uint64_t, uint64_t *); uint64_t lo; uint64_t hi = fle_imul1(32, R[0], ({src}), &lo); R[0] = (uint32_t)lo; R[2] = (uint32_t)hi; }}"
        ),
        _ => format!(
            "{{ uint64_t fle_imul1(unsigned, uint64_t, uint64_t, uint64_t *); uint64_t lo; uint64_t hi = fle_imul1(64, R[0], ({src}), &lo); R[0] = lo; R[2] = hi; }}"
        ),
    };
    Ok(("imul1", body))
}

fn need(c: bool, msg: &str) -> Result<(), String> {
    if c {
        Ok(())
    } else {
        Err(msg.to_string())
    }
}

fn reg_num(r: Register) -> Result<usize, String> {
    if !r.is_gpr() {
        return Err("non-GPR register".into());
    }
    Ok(r.full_register().number())
}

fn is_high(r: Register) -> bool {
    matches!(r, Register::AH | Register::BH | Register::CH | Register::DH)
}

fn reg_rd(r: Register) -> Result<String, String> {
    let n = reg_num(r)?;
    if is_high(r) {
        return Ok(format!("((uint8_t)(R[{n}] >> 8))"));
    }
    Ok(match r.size() {
        8 => format!("R[{n}]"),
        4 => format!("((uint32_t)R[{n}])"),
        2 => format!("((uint16_t)R[{n}])"),
        _ => format!("((uint8_t)R[{n}])"),
    })
}

/// Destination write of width 2, 4 or 8. A 32-bit write zero-extends. An 8-bit destination is not used here.
fn reg_wr(r: Register, v: &str) -> Result<String, String> {
    let n = reg_num(r)?;
    Ok(match r.size() {
        8 => format!("R[{n}] = {v};"),
        4 => format!("R[{n}] = (uint32_t)({v});"),
        2 => format!("R[{n}] = (R[{n}] & ~UINT64_C(0xffff)) | (uint16_t)({v});"),
        _ => return Err("8-bit destination".into()),
    })
}

fn opsize(i: &Instruction, n: u32) -> Result<usize, String> {
    match i.op_kind(n) {
        OpKind::Register => {
            let r = i.op_register(n);
            reg_num(r)?;
            Ok(r.size())
        }
        OpKind::Memory => Ok(i.memory_size().size()),
        _ => Err("immediate operand has no size".into()),
    }
}

/// Source operand value as a C expression of width `size` (zero-extended).
fn rd(i: &Instruction, n: u32, size: usize) -> Result<String, String> {
    match i.op_kind(n) {
        OpKind::Register => reg_rd(i.op_register(n)),
        OpKind::Memory => Ok(format!("fle_ld({}, {size})", ea(i)?)),
        _ => Err("operand kind".into()),
    }
}

/// Effective address as a C expression. Never loads from memory. 32-bit addressing (0x67) is not modelled here.
fn ea(i: &Instruction) -> Result<String, String> {
    if matches!(i.memory_segment(), Register::FS | Register::GS) {
        return Err("FS/GS segment".into());
    }
    if i.is_ip_rel_memory_operand() {
        return Ok(format!("UINT64_C({:#x})", i.ip_rel_memory_address()));
    }
    let mut terms: Vec<String> = Vec::new();
    let base = i.memory_base();
    if base != Register::None {
        need(base.size() == 8, "32-bit addressing")?;
        terms.push(reg_rd(base)?);
    }
    let idx = i.memory_index();
    if idx != Register::None {
        need(idx.size() == 8, "32-bit addressing")?;
        terms.push(format!("({}*{})", reg_rd(idx)?, i.memory_index_scale()));
    }
    let d = i.memory_displacement64();
    if d != 0 || terms.is_empty() {
        terms.push(format!("UINT64_C({d:#x})"));
    }
    Ok(format!("({})", terms.join(" + ")))
}
