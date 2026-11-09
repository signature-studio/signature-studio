//! SSE/SSE2/SSE4.1 floating-point instructions with MXCSR exception flags (decision 0038).
//!
//! Every instruction here is emitted as a call to a helper in runtime/fle_mxcsr_flags.h. Since decision 0044 the
//! helper is portable C (integer arithmetic on the bit patterns, no host FP unit, no asm): it applies the guest's RC,
//! FTZ and DAZ, computes the result with the x86 NaN rules, and ORs the raised status flags (IE DE ZE OE UE PE) into
//! `fle_mxcsr` (decision 0038 ran the instruction on the host CPU; the names and signatures did not change).
//!
//! Instructions: ADD SUB MUL DIV MIN MAX SQRT (ps pd ss sd), CMP (ps pd ss sd), COMISS COMISD UCOMISS UCOMISD,
//! ROUND (ps pd ss sd), CVTSS2SD CVTSD2SS CVTPS2PD CVTPD2PS CVTDQ2PS CVTDQ2PD CVTPS2DQ CVTTPS2DQ CVTPD2DQ
//! CVTTPD2DQ, CVTSI2SS CVTSI2SD CVTSS2SI CVTSD2SI CVTTSS2SI CVTTSD2SI. Not here: RCPPS, RSQRTPS (the result is an
//! implementation-specific approximation, not reproducible across CPUs), MMX forms (CVTPI2PS ...), HADDPS, HSUB*, DPPS.
//! Decision 0106 adds HADDPD (SSE3; xmm, xmm/m128 only): the guest's dsp_ippv2 `ippsSum_32f` code path ends in `haddpd xmm4, xmm4` when ILTools'
//! initialisation reaches FontCache_AcquireFont. The other SSE3 mnemonics stay unsupported (decision 0075).
//! Declared in sse_more.rs with `#[path]` so that lib.rs is not touched; sse_more::lift calls `lift` first.

use iced_x86::{Instruction, Mnemonic, OpKind, Register};

type Lift = Result<(&'static str, String), String>;

/// Width of the source operand when it is in memory.
#[derive(Clone, Copy)]
enum Src {
    S32,
    S64,
    P128,
}

pub fn lift(i: &Instruction) -> Option<Lift> {
    use Mnemonic::*;
    use Src::*;
    let r = match i.mnemonic() {
        Addss => op2(i, "addss", "fle_mx_addss", S32),
        Addsd => op2(i, "addsd", "fle_mx_addsd", S64),
        Addps => op2(i, "addps", "fle_mx_addps", P128),
        Addpd => op2(i, "addpd", "fle_mx_addpd", P128),
        Haddpd => op2(i, "haddpd", "fle_mx_haddpd", P128),
        // Decision 0191: HADDPS, ADDSUBPS, ADDSUBPD (SSE3; the ones that the census of the engine and its 5 DLLs holds; same helper pattern as HADDPD). HSUBPS, HSUBPD, LDDQU have 0 sites and stay unsupported.
        Haddps => op2(i, "haddps", "fle_mx_haddps", P128),
        Addsubps => op2(i, "addsubps", "fle_mx_addsubps", P128),
        Addsubpd => op2(i, "addsubpd", "fle_mx_addsubpd", P128),
        Subss => op2(i, "subss", "fle_mx_subss", S32),
        Subsd => op2(i, "subsd", "fle_mx_subsd", S64),
        Subps => op2(i, "subps", "fle_mx_subps", P128),
        Subpd => op2(i, "subpd", "fle_mx_subpd", P128),
        Mulss => op2(i, "mulss", "fle_mx_mulss", S32),
        Mulsd => op2(i, "mulsd", "fle_mx_mulsd", S64),
        Mulps => op2(i, "mulps", "fle_mx_mulps", P128),
        Mulpd => op2(i, "mulpd", "fle_mx_mulpd", P128),
        Divss => op2(i, "divss", "fle_mx_divss", S32),
        Divsd => op2(i, "divsd", "fle_mx_divsd", S64),
        Divps => op2(i, "divps", "fle_mx_divps", P128),
        Divpd => op2(i, "divpd", "fle_mx_divpd", P128),
        Minss => op2(i, "minss", "fle_mx_minss", S32),
        Minsd => op2(i, "minsd", "fle_mx_minsd", S64),
        Minps => op2(i, "minps", "fle_mx_minps", P128),
        Minpd => op2(i, "minpd", "fle_mx_minpd", P128),
        Maxss => op2(i, "maxss", "fle_mx_maxss", S32),
        Maxsd => op2(i, "maxsd", "fle_mx_maxsd", S64),
        Maxps => op2(i, "maxps", "fle_mx_maxps", P128),
        Maxpd => op2(i, "maxpd", "fle_mx_maxpd", P128),
        Sqrtss => op2(i, "sqrtss", "fle_mx_sqrtss", S32),
        Sqrtsd => op2(i, "sqrtsd", "fle_mx_sqrtsd", S64),
        Sqrtps => op2(i, "sqrtps", "fle_mx_sqrtps", P128),
        Sqrtpd => op2(i, "sqrtpd", "fle_mx_sqrtpd", P128),
        Cvtss2sd => op2(i, "cvtss2sd", "fle_mx_cvtss2sd", S32),
        Cvtsd2ss => op2(i, "cvtsd2ss", "fle_mx_cvtsd2ss", S64),
        Cvtps2pd => op2(i, "cvtps2pd", "fle_mx_cvtps2pd", S64),
        Cvtpd2ps => op2(i, "cvtpd2ps", "fle_mx_cvtpd2ps", P128),
        Cvtdq2ps => op2(i, "cvtdq2ps", "fle_mx_cvtdq2ps", P128),
        Cvtdq2pd => op2(i, "cvtdq2pd", "fle_mx_cvtdq2pd", S64),
        Cvtps2dq => op2(i, "cvtps2dq", "fle_mx_cvtps2dq", P128),
        Cvttps2dq => op2(i, "cvttps2dq", "fle_mx_cvttps2dq", P128),
        Cvtpd2dq => op2(i, "cvtpd2dq", "fle_mx_cvtpd2dq", P128),
        Cvttpd2dq => op2(i, "cvttpd2dq", "fle_mx_cvttpd2dq", P128),
        Cmpss if is_xmm_form(i) => imm2(i, "cmpss", "FLE_MX_CMPSS", S32, 7),
        Cmpsd if is_xmm_form(i) => imm2(i, "cmpsd", "FLE_MX_CMPSD", S64, 7),
        Cmpps => imm2(i, "cmpps", "FLE_MX_CMPPS", P128, 7),
        Cmppd => imm2(i, "cmppd", "FLE_MX_CMPPD", P128, 7),
        Roundss => imm2(i, "roundss", "FLE_MX_ROUNDSS", S32, 255),
        Roundsd => imm2(i, "roundsd", "FLE_MX_ROUNDSD", S64, 255),
        Roundps => imm2(i, "roundps", "FLE_MX_ROUNDPS", P128, 255),
        Roundpd => imm2(i, "roundpd", "FLE_MX_ROUNDPD", P128, 255),
        Ucomiss => comi(i, "ucomiss", "fle_mx_ucomiss", S32),
        Ucomisd => comi(i, "ucomisd", "fle_mx_ucomisd", S64),
        Comiss => comi(i, "comiss", "fle_mx_comiss", S32),
        Comisd => comi(i, "comisd", "fle_mx_comisd", S64),
        Cvtsi2ss => cvtsi2(i, "cvtsi2ss", "fle_mx_cvtsi2ss"),
        Cvtsi2sd => cvtsi2(i, "cvtsi2sd", "fle_mx_cvtsi2sd"),
        Cvtss2si => cvt2si(i, "cvtss2si", "fle_mx_cvtss2si", S32),
        Cvtsd2si => cvt2si(i, "cvtsd2si", "fle_mx_cvtsd2si", S64),
        Cvttss2si => cvt2si(i, "cvttss2si", "fle_mx_cvttss2si", S32),
        Cvttsd2si => cvt2si(i, "cvttsd2si", "fle_mx_cvttsd2si", S64),
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

/// CMPSD and CMPSS also name string instructions (memory operands only, no immediate).
fn is_xmm_form(i: &Instruction) -> bool {
    i.op_count() == 3 && i.op_kind(0) == OpKind::Register && i.op_register(0).is_xmm()
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

/// Source operand n as an fle_xmm C expression: the register, or memory of the given width (packed: aligned).
fn src(i: &Instruction, n: u32, w: Src) -> Result<String, String> {
    match i.op_kind(n) {
        OpKind::Register => Ok(format!("X[{}]", xmm(i.op_register(n))?)),
        OpKind::Memory => {
            let a = crate::ea(i)?;
            Ok(match w {
                Src::S32 => format!("fle_mx_m32({a})"),
                Src::S64 => format!("fle_mx_m64({a})"),
                Src::P128 => format!("fle_ldxa({a})"),
            })
        }
        _ => Err("operand kind".into()),
    }
}

/// dst = f(dst, src).
fn op2(i: &Instruction, name: &'static str, f: &str, w: Src) -> Lift {
    need(i.op_count() == 2, "operand count")?;
    let d = dst(i)?;
    Ok((name, format!("X[{d}] = {f}(X[{d}], {});", src(i, 1, w)?)))
}

/// dst = MACRO(dst, src, imm8). `max` is the largest immediate accepted.
fn imm2(i: &Instruction, name: &'static str, m: &str, w: Src, max: u32) -> Lift {
    need(i.op_count() == 3 && i.op_kind(2) == OpKind::Immediate8, "imm8 form")?;
    let imm = i.immediate8() as u32;
    need(imm <= max, "immediate outside the defined range")?;
    let d = dst(i)?;
    Ok((name, format!("X[{d}] = {m}(X[{d}], {}, {imm});", src(i, 1, w)?)))
}

fn comi(i: &Instruction, name: &'static str, f: &str, w: Src) -> Lift {
    need(i.op_count() == 2, "operand count")?;
    let d = dst(i)?;
    Ok((name, format!("{f}(X[{d}], {});", src(i, 1, w)?)))
}

/// CVTSI2SS, CVTSI2SD: signed 32- or 64-bit integer from a GPR or memory.
fn cvtsi2(i: &Instruction, name: &'static str, f: &str) -> Lift {
    need(i.op_count() == 2, "operand count")?;
    let d = dst(i)?;
    let (v, bits) = match i.op_kind(1) {
        OpKind::Register => {
            let r = i.op_register(1);
            let n = gpr(r)?;
            if r.size() == 4 {
                (format!("(int32_t)((uint32_t)R[{n}])"), 32)
            } else {
                (format!("(int64_t)R[{n}]"), 64)
            }
        }
        OpKind::Memory => {
            let sz = i.memory_size().size();
            need(sz == 4 || sz == 8, "cvtsi2 memory size")?;
            if sz == 4 {
                (format!("(int32_t)fle_ld({}, 4)", crate::ea(i)?), 32)
            } else {
                (format!("(int64_t)fle_ld({}, 8)", crate::ea(i)?), 64)
            }
        }
        _ => return Err("operand kind".into()),
    };
    Ok((name, format!("X[{d}] = {f}{bits}(X[{d}], {v});")))
}

/// CVTSS2SI, CVTSD2SI, CVTTSS2SI, CVTTSD2SI. The destination size is that of the GPR operand.
fn cvt2si(i: &Instruction, name: &'static str, f: &str, w: Src) -> Lift {
    need(i.op_count() == 2 && i.op_kind(0) == OpKind::Register, "destination must be a GPR")?;
    let n = gpr(i.op_register(0))?;
    let bits = i.op_register(0).size() * 8;
    Ok((name, format!("R[{n}] = {f}{bits}({});", src(i, 1, w)?)))
}
