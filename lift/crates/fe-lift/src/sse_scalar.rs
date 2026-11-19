//! Scalar single- and double-precision SSE: moves, arithmetic, min and max,
//! square root, conversions, and the ordered and unordered compares.
//!
//! Register model (runtime/fle_sse.h): X[n] is XMMn as a 128-bit union. An
//! arithmetic or conversion instruction writes only its destination element and
//! keeps the upper lanes. MOVSS and MOVSD between registers write only the low
//! element. MOVSS and MOVSD from memory zero the upper lanes. UCOMISS, UCOMISD,
//! COMISS and COMISD set ZF, PF and CF and clear OF, SF and AF (fle_flags.h).

use iced_x86::{Instruction, Mnemonic, OpKind, Register};

type Lift = Result<(&'static str, String), String>;

pub fn lift(i: &Instruction) -> Option<Lift> {
    let r = match i.mnemonic() {
        Mnemonic::Movss if has_xmm(i) => mov(i, 4),
        Mnemonic::Movsd if has_xmm(i) => mov(i, 8),
        Mnemonic::Addss => arith(i, "addss", 4, "fle_addss"),
        Mnemonic::Addsd => arith(i, "addsd", 8, "fle_addsd"),
        Mnemonic::Subss => arith(i, "subss", 4, "fle_subss"),
        Mnemonic::Subsd => arith(i, "subsd", 8, "fle_subsd"),
        Mnemonic::Mulss => arith(i, "mulss", 4, "fle_mulss"),
        Mnemonic::Mulsd => arith(i, "mulsd", 8, "fle_mulsd"),
        Mnemonic::Divss => arith(i, "divss", 4, "fle_divss"),
        Mnemonic::Divsd => arith(i, "divsd", 8, "fle_divsd"),
        Mnemonic::Minss => arith(i, "minss", 4, "fle_minss"),
        Mnemonic::Minsd => arith(i, "minsd", 8, "fle_minsd"),
        Mnemonic::Maxss => arith(i, "maxss", 4, "fle_maxss"),
        Mnemonic::Maxsd => arith(i, "maxsd", 8, "fle_maxsd"),
        Mnemonic::Sqrtss => sqrt(i, "sqrtss", 4),
        Mnemonic::Sqrtsd => sqrt(i, "sqrtsd", 8),
        Mnemonic::Cvtss2sd => cvt_ss2sd(i),
        Mnemonic::Cvtsd2ss => cvt_sd2ss(i),
        Mnemonic::Cvtsi2ss => cvtsi2(i, "cvtsi2ss", false),
        Mnemonic::Cvtsi2sd => cvtsi2(i, "cvtsi2sd", true),
        Mnemonic::Cvttss2si => cvt2si(i, "cvttss2si", 4, true),
        Mnemonic::Cvttsd2si => cvt2si(i, "cvttsd2si", 8, true),
        Mnemonic::Cvtss2si => cvt2si(i, "cvtss2si", 4, false),
        Mnemonic::Cvtsd2si => cvt2si(i, "cvtsd2si", 8, false),
        Mnemonic::Ucomiss => ucomi(i, "ucomiss", 4, "fle_op_ucomiss"),
        Mnemonic::Ucomisd => ucomi(i, "ucomisd", 8, "fle_op_ucomisd"),
        Mnemonic::Comiss => ucomi(i, "comiss", 4, "fle_op_ucomiss"),
        Mnemonic::Comisd => ucomi(i, "comisd", 8, "fle_op_comisd"),
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

fn dst_xmm(i: &Instruction) -> Result<usize, String> {
    need(i.op_kind(0) == OpKind::Register, "destination must be a register")?;
    xmm(i.op_register(0))
}

/// XMM element `sz` bytes wide (4 or 8) as a C expression.
fn xbits(k: usize, sz: usize) -> String {
    if sz == 4 {
        format!("X[{k}].u32[0]")
    } else {
        format!("X[{k}].u64[0]")
    }
}

/// Memory value of `sz` bytes (4 or 8), zero-extended, as a C expression.
fn mem_bits(e: String, sz: usize) -> String {
    if sz == 4 {
        format!("((uint32_t)fle_ld({e}, 4))")
    } else {
        format!("fle_ld({e}, 8)")
    }
}

/// Source operand (XMM low element or memory) of `sz` bytes.
fn fp_src(i: &Instruction, n: u32, sz: usize) -> Result<String, String> {
    match i.op_kind(n) {
        OpKind::Register => Ok(xbits(xmm(i.op_register(n))?, sz)),
        OpKind::Memory => Ok(mem_bits(ea(i)?, sz)),
        _ => Err("operand kind".into()),
    }
}

fn mov(i: &Instruction, sz: usize) -> Lift {
    need(i.op_count() == 2, "movss/movsd operand count")?;
    let name = if sz == 4 { "movss" } else { "movsd" };
    let body = match (i.op_kind(0), i.op_kind(1)) {
        (OpKind::Register, OpKind::Register) => {
            let (d, s) = (dst_xmm(i)?, xmm(i.op_register(1))?);
            if sz == 4 {
                format!("X[{d}].u32[0] = X[{s}].u32[0];")
            } else {
                format!("X[{d}].u64[0] = X[{s}].u64[0];")
            }
        }
        (OpKind::Register, OpKind::Memory) => {
            let d = dst_xmm(i)?;
            format!("X[{d}].u64[0] = {}; X[{d}].u64[1] = 0;", mem_bits(ea(i)?, sz))
        }
        (OpKind::Memory, OpKind::Register) => {
            let s = xmm(i.op_register(1))?;
            format!("fle_st({}, {sz}, {});", ea(i)?, xbits(s, sz))
        }
        _ => return Err("movss/movsd operand form".into()),
    };
    Ok((name, body))
}

/// Two-operand scalar arithmetic: dst.lo = f(dst.lo, src).
fn arith(i: &Instruction, name: &'static str, sz: usize, f: &str) -> Lift {
    need(i.op_count() == 2, "operand count")?;
    let d = dst_xmm(i)?;
    let s = fp_src(i, 1, sz)?;
    let body = if sz == 4 {
        format!("X[{d}].u32[0] = {f}(X[{d}].u32[0], {s});")
    } else {
        format!("X[{d}].u64[0] = {f}(X[{d}].u64[0], {s});")
    };
    Ok((name, body))
}

fn sqrt(i: &Instruction, name: &'static str, sz: usize) -> Lift {
    need(i.op_count() == 2, "operand count")?;
    let d = dst_xmm(i)?;
    let s = fp_src(i, 1, sz)?;
    let body = if sz == 4 {
        format!("X[{d}].u32[0] = fle_sqrtss({s});")
    } else {
        format!("X[{d}].u64[0] = fle_sqrtsd({s});")
    };
    Ok((name, body))
}

fn cvt_ss2sd(i: &Instruction) -> Lift {
    need(i.op_count() == 2, "operand count")?;
    let d = dst_xmm(i)?;
    let s = fp_src(i, 1, 4)?;
    Ok(("cvtss2sd", format!("X[{d}].u64[0] = fle_cvtss2sd({s});")))
}

fn cvt_sd2ss(i: &Instruction) -> Lift {
    need(i.op_count() == 2, "operand count")?;
    let d = dst_xmm(i)?;
    let s = fp_src(i, 1, 8)?;
    Ok(("cvtsd2ss", format!("X[{d}].u32[0] = fle_cvtsd2ss({s});")))
}

/// CVTSI2SS / CVTSI2SD. The integer is signed, 32 or 64 bits, from a GPR or memory.
fn cvtsi2(i: &Instruction, name: &'static str, dbl: bool) -> Lift {
    need(i.op_count() == 2, "operand count")?;
    let d = dst_xmm(i)?;
    let v = match i.op_kind(1) {
        OpKind::Register => {
            let r = i.op_register(1);
            let n = gpr(r)?;
            if r.size() == 4 {
                format!("(int32_t)((uint32_t)R[{n}])")
            } else {
                format!("(int64_t)R[{n}]")
            }
        }
        OpKind::Memory => {
            let sz = i.memory_size().size();
            need(sz == 4 || sz == 8, "cvtsi2 memory size")?;
            if sz == 4 {
                format!("(int32_t)fle_ld({}, 4)", ea(i)?)
            } else {
                format!("(int64_t)fle_ld({}, 8)", ea(i)?)
            }
        }
        _ => return Err("operand kind".into()),
    };
    let body = if dbl {
        format!("X[{d}].u64[0] = fle_b64((double)({v}));")
    } else {
        format!("X[{d}].u32[0] = fle_b32((float)({v}));")
    };
    Ok((name, body))
}

/// CVTTSS2SI, CVTTSD2SI (truncation), CVTSS2SI, CVTSD2SI (round by MXCSR RN).
/// The destination size is that of the GPR operand. `sz` is the source width.
fn cvt2si(i: &Instruction, name: &'static str, sz: usize, trunc: bool) -> Lift {
    need(i.op_count() == 2 && i.op_kind(0) == OpKind::Register, "destination must be a GPR")?;
    let n = gpr(i.op_register(0))?;
    let dsz = i.op_register(0).size();
    let s = fp_src(i, 1, sz)?;
    let v = if sz == 4 { format!("(double)fle_f32({s})") } else { format!("fle_f64({s})") };
    let f = match (trunc, dsz) {
        (true, 4) => "fle_cvtt32",
        (true, _) => "fle_cvtt64",
        (false, 4) => "fle_cvtr32",
        (false, _) => "fle_cvtr64",
    };
    Ok((name, format!("R[{n}] = {f}({v});")))
}

/// UCOMISS, UCOMISD, COMISS, COMISD. The flags are set by `f` (fle_flags.h).
fn ucomi(i: &Instruction, name: &'static str, sz: usize, f: &str) -> Lift {
    need(i.op_count() == 2, "operand count")?;
    let d = dst_xmm(i)?;
    let s = fp_src(i, 1, sz)?;
    Ok((name, format!("{f}(X[{d}].u64[0], {s});")))
}

/// Effective address as a C expression. Never loads from memory.
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
