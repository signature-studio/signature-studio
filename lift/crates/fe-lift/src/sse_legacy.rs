//! Legacy SSE and SSE2 instructions on the XMM registers that dsp_ippv2_x64.dll uses and that were still
//! unsupported (decision 0075; xmm forms only, the MMX forms and the VEX/EVEX forms are not lifted here):
//! ORPS, ANDNPS, PCMPEQB, PCMPEQW, PCMPGTB, PCMPGTW, PADDB, PSUBB, PSUBQ, PMAXUB, PMINUB, PMADDWD,
//! PMULUDQ, PSRLDQ, PSLLDQ, PMOVMSKB, MOVMSKPS, MOVMSKPD, PEXTRW (xmm to r32/r64), PINSRW (r32/r64/m16 to xmm),
//! MOVNTPS, MOVNTPD, MOVNTDQ (m128, xmm), MOVNTI (m32/m64, r32/r64), RSQRTPS and RSQRTSS. Declared from sse_more.rs
//! with `#[path]`. RSQRTPS and RSQRTSS are approximations whose bits the SDM does not fix; they reproduce the host
//! (table and exhaustive check in runtime/fle_sse_int2.h and tests/rsqrt/).
//!
//! Semantics: Intel SDM Vol. 2, the 128-bit Legacy SSE pseudo-code of each instruction (read in the felixcloutier
//! transcription, which is unofficial). The helpers are in runtime/fle_sse_int2.h. No flags change. A packed
//! instruction writes the whole destination register, and the upper bits above 128 do not exist in this model.
//!
//! Alignment: ORPS, ANDNPS, PCMPEQx, PCMPGTx, PADDB, PSUBx, PMAXUB, PMINUB, PMADDWD and PMULUDQ point to
//! Table 2-21 (Type 4: "#GP(0) Legacy SSE: Memory operand is not 16-byte aligned"), and MOVNTPS, MOVNTPD and MOVNTDQ
//! to Table 2-18 (Type 1, same #GP), so their m128 operand uses the aligned access (`fle_ldxa`, `fle_stxa`), which
//! aborts on a misaligned address. PINSRW (m16) and MOVNTI have no alignment #GP (MOVNTI: only #AC), so `fle_ld` and
//! `fle_st` are used at any address. The host probe lift/out-w8/probe_align.c agrees for every form. The non-temporal
//! hint of MOVNT* has no architectural effect on the value stored.

use iced_x86::{Code, Instruction, Mnemonic, OpKind, Register};

use super::{dst, ea, need, src, xmm};

type Lift = Result<(&'static str, String), String>;

pub fn lift(i: &Instruction) -> Option<Lift> {
    let r = match i.mnemonic() {
        Mnemonic::Orps => bin(i, "orps", "fle_por"),
        Mnemonic::Andnps => bin(i, "andnps", "fle_pandn"),
        Mnemonic::Pcmpeqb => bin(i, "pcmpeqb", "fle_lg_pcmpeqb"),
        Mnemonic::Pcmpeqw => bin(i, "pcmpeqw", "fle_lg_pcmpeqw"),
        Mnemonic::Pcmpgtb => bin(i, "pcmpgtb", "fle_lg_pcmpgtb"),
        Mnemonic::Pcmpgtw => bin(i, "pcmpgtw", "fle_lg_pcmpgtw"),
        Mnemonic::Paddb => bin(i, "paddb", "fle_lg_paddb"),
        Mnemonic::Psubb => bin(i, "psubb", "fle_lg_psubb"),
        Mnemonic::Psubq => bin(i, "psubq", "fle_lg_psubq"),
        Mnemonic::Pmaxub => bin(i, "pmaxub", "fle_lg_pmaxub"),
        Mnemonic::Pminub => bin(i, "pminub", "fle_lg_pminub"),
        Mnemonic::Pmaddwd => bin(i, "pmaddwd", "fle_lg_pmaddwd"),
        Mnemonic::Pmuludq => bin(i, "pmuludq", "fle_lg_pmuludq"),
        Mnemonic::Psrldq => byteshift(i, "psrldq", "fle_lg_psrldq"),
        Mnemonic::Pslldq => byteshift(i, "pslldq", "fle_lg_pslldq"),
        Mnemonic::Pmovmskb => mask(i, "pmovmskb", "fle_lg_pmovmskb"),
        Mnemonic::Movmskps => mask(i, "movmskps", "fle_lg_movmskps"),
        Mnemonic::Movmskpd => mask(i, "movmskpd", "fle_lg_movmskpd"),
        Mnemonic::Pextrw => pextrw(i),
        Mnemonic::Pinsrw => pinsrw(i),
        Mnemonic::Movntps => movnt(i, "movntps"),
        Mnemonic::Movntpd => movnt(i, "movntpd"),
        Mnemonic::Movntdq => movnt(i, "movntdq"),
        Mnemonic::Movnti => movnti(i),
        Mnemonic::Rsqrtps => rsqrtps(i),
        Mnemonic::Rsqrtss => rsqrtss(i),
        _ => return None,
    };
    Some(r)
}

fn gpr(r: Register) -> Result<usize, String> {
    need(r.is_gpr() && matches!(r.size(), 4 | 8), "GPR must be 32 or 64 bit")?;
    Ok(r.full_register().number())
}

/// Two-operand packed operation: dst = f(dst, src). The helpers are static inline in fle_sse.h.
fn bin(i: &Instruction, name: &'static str, f: &str) -> Lift {
    need(i.op_count() == 2, "operand count")?;
    let d = dst(i)?;
    Ok((name, format!("X[{d}] = {f}(X[{d}], {});", src(i, 1)?)))
}

/// PSRLDQ and PSLLDQ xmm, imm8: the count is in bytes; above 15 the result is 0 (the helper clamps to 16).
fn byteshift(i: &Instruction, name: &'static str, f: &str) -> Lift {
    need(i.op_count() == 2 && i.op_kind(1) == OpKind::Immediate8, "byte shift operand form")?;
    let d = dst(i)?;
    Ok((name, format!("X[{d}] = {f}(X[{d}], {}u);", i.immediate8())))
}

/// PMOVMSKB, MOVMSKPS and MOVMSKPD reg, xmm. The destination is a 32-bit or 64-bit register; the mask is at most
/// 16 bits, so a 32-bit write (which zero-extends) and a 64-bit write are the same value.
fn mask(i: &Instruction, name: &'static str, f: &str) -> Lift {
    need(i.op_count() == 2, "operand count")?;
    need(i.op_kind(0) == OpKind::Register && i.op_kind(1) == OpKind::Register, "mask operand form")?;
    let n = gpr(i.op_register(0))?;
    let s = xmm(i.op_register(1))?;
    Ok((name, format!("R[{n}] = {f}(X[{s}]);")))
}

/// PEXTRW reg, xmm, imm8 (SSE2, 66 0F C5). The `reg/m16` form (66 0F 3A 15) is SSE4.1, which CPUID does not claim.
fn pextrw(i: &Instruction) -> Lift {
    need(matches!(i.code(), Code::Pextrw_r32_xmm_imm8 | Code::Pextrw_r64_xmm_imm8), "PEXTRW is lifted only as 66 0F C5 (SSE2); the 66 0F 3A 15 form is SSE4.1")?;
    need(i.op_count() == 3 && i.op_kind(2) == OpKind::Immediate8, "pextrw operand form")?;
    let n = gpr(i.op_register(0))?;
    let s = xmm(i.op_register(1))?;
    Ok(("pextrw", format!("R[{n}] = fle_lg_pextrw(X[{s}], {}u);", i.immediate8())))
}

/// PINSRW xmm, r32/m16, imm8 (66 0F C4). A register source contributes its low 16 bits.
fn pinsrw(i: &Instruction) -> Lift {
    need(i.op_count() == 3 && i.op_kind(2) == OpKind::Immediate8, "pinsrw operand form")?;
    let d = dst(i)?;
    let w = match i.op_kind(1) {
        OpKind::Register => format!("R[{}]", gpr(i.op_register(1))?),
        OpKind::Memory => format!("fle_ld({}, 2)", ea(i)?),
        _ => return Err("operand kind".into()),
    };
    Ok(("pinsrw", format!("X[{d}] = fle_lg_pinsrw(X[{d}], {w}, {}u);", i.immediate8())))
}

/// MOVNTPS, MOVNTPD and MOVNTDQ m128, xmm: an aligned 16-byte store.
fn movnt(i: &Instruction, name: &'static str) -> Lift {
    need(i.op_count() == 2, "operand count")?;
    need(i.op_kind(0) == OpKind::Memory && i.op_kind(1) == OpKind::Register, "non-temporal store operand form")?;
    let s = xmm(i.op_register(1))?;
    Ok((name, format!("fle_stxa({}, X[{s}]);", ea(i)?)))
}

/// RSQRTPS xmm, xmm/m128 (aligned memory operand). The result bits are the host's (runtime/fle_sse_int2.h).
fn rsqrtps(i: &Instruction) -> Lift {
    need(i.op_count() == 2, "operand count")?;
    let d = dst(i)?;
    Ok(("rsqrtps", format!("X[{d}] = fle_lg_rsqrtps(X[{d}], {});", src(i, 1)?)))
}

/// RSQRTSS xmm, xmm/m32: only the low float of the destination changes. The m32 operand has no alignment rule.
fn rsqrtss(i: &Instruction) -> Lift {
    need(i.op_count() == 2, "operand count")?;
    let d = dst(i)?;
    let s = match i.op_kind(1) {
        OpKind::Register => format!("X[{}].u32[0]", xmm(i.op_register(1))?),
        OpKind::Memory => format!("(uint32_t)fle_ld({}, 4)", ea(i)?),
        _ => return Err("operand kind".into()),
    };
    Ok(("rsqrtss", format!("X[{d}].u32[0] = fle_lg_rsqrt_f32({s});")))
}

/// MOVNTI m32, r32 and m64, r64: a plain store of the register.
fn movnti(i: &Instruction) -> Lift {
    need(i.op_count() == 2, "operand count")?;
    need(i.op_kind(0) == OpKind::Memory && i.op_kind(1) == OpKind::Register, "movnti operand form")?;
    let r = i.op_register(1);
    let n = gpr(r)?;
    let sz = r.size();
    Ok(("movnti", format!("fle_st({}, {sz}, R[{n}]);", ea(i)?)))
}
