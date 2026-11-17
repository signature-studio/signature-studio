//! Packed SSE and SSE2 on the 128-bit XMM registers: aligned and unaligned
//! 128-bit moves; packed logic (PAND, PANDN, POR, PXOR, and the ps and pd logic
//! forms, which act on the same bits); PADDD, PSUBD, PCMPEQD, PMULHUW; PSHUFD;
//! PUNPCKL* and PUNPCKH*; CVTPS2DQ and CVTTPS2DQ; and MOVD and MOVQ between an
//! XMM register and a GPR or memory.
//!
//! Alignment (SDM Vol. 1, SSE memory operands, 既知の規則; not re-checked
//! against the manual in this work): MOVAPS, MOVAPD and MOVDQA, and the legacy
//! SSE packed instructions with a 128-bit memory operand, need a 16-byte aligned
//! address. The generated code calls `fle_misaligned` (abort) when it is not.
//! MOVUPS, MOVUPD and MOVDQU take any address. MOVD and MOVQ have no alignment
//! requirement.
//!
//! Register model (runtime/fle_sse.h): X[n] is XMMn. A packed instruction writes
//! the whole destination register. MOVD and MOVQ into an XMM register zero the
//! bits above the moved element.

use iced_x86::{Instruction, Mnemonic, OpKind, Register};

type Lift = Result<(&'static str, String), String>;

pub fn lift(i: &Instruction) -> Option<Lift> {
    if (0..i.op_count()).any(|k| i.op_kind(k) == OpKind::Register && i.op_register(k).is_mm()) {
        return None;
    }
    let r = match i.mnemonic() {
        Mnemonic::Movaps => mov(i, "movaps", true),
        Mnemonic::Movapd => mov(i, "movapd", true),
        Mnemonic::Movdqa => mov(i, "movdqa", true),
        Mnemonic::Movups => mov(i, "movups", false),
        Mnemonic::Movupd => mov(i, "movupd", false),
        Mnemonic::Movdqu => mov(i, "movdqu", false),
        Mnemonic::Lddqu => lddqu(i),
        Mnemonic::Pand => bin(i, "pand", "fle_pand"),
        Mnemonic::Pandn => bin(i, "pandn", "fle_pandn"),
        Mnemonic::Por => bin(i, "por", "fle_por"),
        Mnemonic::Pxor => bin(i, "pxor", "fle_pxor"),
        Mnemonic::Andps => bin(i, "andps", "fle_pand"),
        Mnemonic::Andpd => bin(i, "andpd", "fle_pand"),
        Mnemonic::Andnpd => bin(i, "andnpd", "fle_pandn"),
        Mnemonic::Xorps => bin(i, "xorps", "fle_pxor"),
        Mnemonic::Xorpd => bin(i, "xorpd", "fle_pxor"),
        Mnemonic::Orpd => bin(i, "orpd", "fle_por"),
        Mnemonic::Paddd => bin(i, "paddd", "fle_paddd"),
        Mnemonic::Psubd => bin(i, "psubd", "fle_psubd"),
        Mnemonic::Pcmpeqd => bin(i, "pcmpeqd", "fle_pcmpeqd"),
        Mnemonic::Pmulhuw => bin(i, "pmulhuw", "fle_pmulhuw"),
        Mnemonic::Pshufd => pshufd(i),
        Mnemonic::Punpcklbw => unpack(i, "punpcklbw", 1, 0),
        Mnemonic::Punpcklwd => unpack(i, "punpcklwd", 2, 0),
        Mnemonic::Punpckldq => unpack(i, "punpckldq", 4, 0),
        Mnemonic::Punpcklqdq => unpack(i, "punpcklqdq", 8, 0),
        Mnemonic::Punpckhbw => unpack(i, "punpckhbw", 1, 1),
        Mnemonic::Punpckhwd => unpack(i, "punpckhwd", 2, 1),
        Mnemonic::Punpckhdq => unpack(i, "punpckhdq", 4, 1),
        Mnemonic::Punpckhqdq => unpack(i, "punpckhqdq", 8, 1),
        Mnemonic::Cvtps2dq => cvt(i, "cvtps2dq", "fle_cvtps2dq"),
        Mnemonic::Cvttps2dq => cvt(i, "cvttps2dq", "fle_cvttps2dq"),
        Mnemonic::Movd if has_xmm(i) => movdq(i, "movd"),
        Mnemonic::Movq if has_xmm(i) => movdq(i, "movq"),
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

fn has_xmm(i: &Instruction) -> bool {
    (0..i.op_count()).any(|k| i.op_kind(k) == OpKind::Register && i.op_register(k).is_xmm())
}

fn xmm(r: Register) -> Result<usize, String> {
    need(r.is_xmm() && r.number() < 16, "register is not XMM0-15")?;
    Ok(r.number())
}

fn gpr(r: Register) -> Result<usize, String> {
    need(r.is_gpr() && matches!(r.size(), 4 | 8), "GPR must be 32 or 64 bit")?;
    Ok(r.full_register().number())
}

fn dst(i: &Instruction) -> Result<usize, String> {
    need(i.op_kind(0) == OpKind::Register, "destination must be a register")?;
    xmm(i.op_register(0))
}

/// Source operand as a 128-bit C expression: an XMM register, or 16 bytes of memory.
/// Memory is read with the aligned load, because the legacy packed forms require alignment.
fn src(i: &Instruction, n: u32) -> Result<String, String> {
    match i.op_kind(n) {
        OpKind::Register => Ok(format!("X[{}]", xmm(i.op_register(n))?)),
        OpKind::Memory => Ok(format!("fle_ldxa({})", ea(i)?)),
        _ => Err("operand kind".into()),
    }
}

fn mov(i: &Instruction, name: &'static str, aligned: bool) -> Lift {
    need(i.op_count() == 2, "move operand count")?;
    let body = match (i.op_kind(0), i.op_kind(1)) {
        (OpKind::Register, OpKind::Register) => format!("X[{}] = X[{}];", dst(i)?, xmm(i.op_register(1))?),
        (OpKind::Register, OpKind::Memory) => {
            let f = if aligned { "fle_ldxa" } else { "fle_ldx" };
            format!("X[{}] = {f}({});", dst(i)?, ea(i)?)
        }
        (OpKind::Memory, OpKind::Register) => {
            let f = if aligned { "fle_stxa" } else { "fle_stx" };
            format!("{f}({}, X[{}]);", ea(i)?, xmm(i.op_register(1))?)
        }
        _ => return Err("move operand form".into()),
    };
    Ok((name, body))
}

/// LDDQU xmm, m128 (SSE3, F2 0F F0 /r; decision 0113 wall 3): DEST[127:0] := SRC[127:0], any alignment (SDM: "functionally similar to (V)MOVDQU
/// ymm/xmm, m256/m128 for loading from memory"). There is no register form (modrm.mod = 11 does not decode), and the VEX forms are other mnemonics.
fn lddqu(i: &Instruction) -> Lift {
    need(i.op_count() == 2 && i.op_kind(1) == OpKind::Memory, "lddqu operand form")?;
    Ok(("lddqu", format!("X[{}] = fle_ldx({});", dst(i)?, ea(i)?)))
}

/// Two-operand packed operation: dst = f(dst, src).
fn bin(i: &Instruction, name: &'static str, f: &str) -> Lift {
    need(i.op_count() == 2, "operand count")?;
    let d = dst(i)?;
    Ok((name, format!("X[{d}] = {f}(X[{d}], {});", src(i, 1)?)))
}

fn pshufd(i: &Instruction) -> Lift {
    need(i.op_count() == 3 && i.op_kind(2) == OpKind::Immediate8, "pshufd operand form")?;
    let d = dst(i)?;
    Ok(("pshufd", format!("X[{d}] = fle_pshufd({}, {:#x});", src(i, 1)?, i.immediate8())))
}

/// PUNPCKL* (hi = 0) and PUNPCKH* (hi = 1). `esz` is the element size in bytes.
fn unpack(i: &Instruction, name: &'static str, esz: u32, hi: u32) -> Lift {
    need(i.op_count() == 2, "operand count")?;
    let d = dst(i)?;
    Ok((name, format!("X[{d}] = fle_unpack(X[{d}], {}, {esz}, {hi});", src(i, 1)?)))
}

fn cvt(i: &Instruction, name: &'static str, f: &str) -> Lift {
    need(i.op_count() == 2, "operand count")?;
    let d = dst(i)?;
    Ok((name, format!("X[{d}] = {f}({});", src(i, 1)?)))
}

fn mem_size(i: &Instruction) -> Result<usize, String> {
    let sz = i.memory_size().size();
    need(matches!(sz, 4 | 8), "MOVD/MOVQ memory size must be 4 or 8")?;
    Ok(sz)
}

/// MOVD and MOVQ between XMM and a GPR or memory. The 32-bit form zero-extends
/// into the XMM register and the GPR.
fn movdq(i: &Instruction, name: &'static str) -> Lift {
    need(i.op_count() == 2, "movd/movq operand count")?;
    let body = if i.op_kind(0) == OpKind::Register && i.op_register(0).is_xmm() {
        let d = xmm(i.op_register(0))?;
        match i.op_kind(1) {
            OpKind::Register if i.op_register(1).is_xmm() => {
                format!("X[{d}].u64[0] = X[{}].u64[0]; X[{d}].u64[1] = 0;", xmm(i.op_register(1))?)
            }
            OpKind::Register => {
                let r = i.op_register(1);
                let n = gpr(r)?;
                let v = if r.size() == 4 { format!("(uint32_t)R[{n}]") } else { format!("R[{n}]") };
                format!("X[{d}].u64[0] = {v}; X[{d}].u64[1] = 0;")
            }
            OpKind::Memory => {
                let sz = mem_size(i)?;
                format!("X[{d}].u64[0] = fle_ld({}, {sz}); X[{d}].u64[1] = 0;", ea(i)?)
            }
            _ => return Err("operand kind".into()),
        }
    } else {
        need(i.op_kind(1) == OpKind::Register && i.op_register(1).is_xmm(), "movd/movq store source must be XMM")?;
        let s = xmm(i.op_register(1))?;
        match i.op_kind(0) {
            OpKind::Register => {
                let r = i.op_register(0);
                let n = gpr(r)?;
                let v = if r.size() == 4 { format!("(uint32_t)X[{s}].u64[0]") } else { format!("X[{s}].u64[0]") };
                format!("R[{n}] = {v};")
            }
            OpKind::Memory => {
                let sz = mem_size(i)?;
                format!("fle_st({}, {sz}, X[{s}].u64[0]);", ea(i)?)
            }
            _ => return Err("operand kind".into()),
        }
    };
    Ok((name, body))
}

/// Effective address as a C expression. Never loads from memory.
/// Same rules as sse_scalar.rs: no FS/GS, and 64-bit addressing only.
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
