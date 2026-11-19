//! Integer and system instructions: bsf, bsr, bts, btr, btc, rol, ror, xchg,
//! cmpxchg16b, jecxz/jrcxz, pause, lfence, mfence, sfence.
//!
//! Every body is plain C over `R`, `FL`, `fle_ld`, `fle_st`, `fle_sext` (runtime
//! headers only), so a generated file needs no extra runtime object.
//!
//! Flags (Intel SDM Vol. 2, order number 325383-093US, see docs/decisions/0023-int-more2.md):
//! - bsf, bsr: ZF = (source == 0). CF, OF, SF, AF are cleared. PF is left
//!   `FLE_UNDEF`: the SDM text (parity of the source) and the host (parity of the
//!   written index) disagree for a non-zero source, so reading it stops the run.
//!   With a zero source ZF = PF = 1 (SDM and host agree) and the destination is not written.
//! - bts, btr, btc: CF = the selected bit before the change. ZF unchanged. OF, SF, AF, PF undefined.
//! - rol, ror: nothing changes for a masked count of 0. Otherwise CF is set, and OF
//!   is set for a masked count of 1 and undefined for any other count. SF, ZF, AF, PF unchanged.
//! - xchg, pause, fences: no flags. cmpxchg16b: only ZF.
//!
//! LOCK, and the implicit lock of xchg with memory, are ignored: one guest thread
//! (decision 0013). jecxz and jrcxz test ECX or RCX and no flag; they are lifted as
//! a straight-line `if (...) goto` so that `lib.rs` needs no new control kind.
//! xgetbv is not lifted here: XCR0 is system-defined and the SDM gives no value (see cpuid.rs, decision 0047).
//! bswap: decision 0047.

use iced_x86::{Instruction, Mnemonic, OpKind, Register};
use std::collections::BTreeSet;

type Lift = Result<(&'static str, String), String>;

/// Lifts the instructions owned by this module. `None` leaves the instruction to `lib.rs`.
pub fn lift(i: &Instruction, entries: &BTreeSet<u64>) -> Option<Lift> {
    match i.mnemonic() {
        Mnemonic::Bsf => Some(bscan(i, true)),
        Mnemonic::Bsr => Some(bscan(i, false)),
        Mnemonic::Bts => Some(bit_rmw(i, Bit::Set)),
        Mnemonic::Btr => Some(bit_rmw(i, Bit::Reset)),
        Mnemonic::Btc => Some(bit_rmw(i, Bit::Flip)),
        Mnemonic::Rol => Some(rotate(i, true)),
        Mnemonic::Ror => Some(rotate(i, false)),
        Mnemonic::Xchg => Some(xchg(i)),
        Mnemonic::Bswap => Some(bswap(i)),
        Mnemonic::Cmpxchg16b => Some(cmpxchg16b(i)),
        Mnemonic::Pause => Some(no_operands(i, "pause", "")),
        Mnemonic::Sfence => Some(no_operands(i, "sfence", FENCE)),
        Mnemonic::Lfence => Some(no_operands(i, "lfence", FENCE)),
        Mnemonic::Mfence => Some(no_operands(i, "mfence", FENCE)),
        Mnemonic::Jecxz | Mnemonic::Jrcxz => Some(jcxz(i, entries)),
        Mnemonic::Jcxz => Some(Err("jcxz (16-bit address size) is not valid in 64-bit mode".into())),
        _ => None,
    }
}

const FENCE: &str = "__atomic_thread_fence(__ATOMIC_SEQ_CST);";

fn no_operands(i: &Instruction, handler: &'static str, body: &str) -> Lift {
    super::need(i.op_count() == 0, "unexpected operand")?;
    Ok((handler, body.to_string()))
}

/// bsf / bsr r, r/m. The source is read before the destination is written.
fn bscan(i: &Instruction, forward: bool) -> Lift {
    super::need(i.op_count() == 2 && i.op_kind(0) == OpKind::Register, "bit scan operand form")?;
    let d = i.op_register(0);
    let sz = super::op_size(i, 0)?;
    super::need(matches!(sz, 2 | 4 | 8), "bit scan operand size")?;
    super::need(i.op_kind(1) == OpKind::Register || i.op_kind(1) == OpKind::Memory, "bit scan source operand")?;
    let src = super::op_read(i, 1, sz)?;
    let idx = if forward { "(uint64_t)__builtin_ctzll(s)" } else { "(uint64_t)(63 - __builtin_clzll(s))" };
    let wr = super::reg_write(d, "k")?;
    let name = if forward { "bsf" } else { "bsr" };
    Ok((
        name,
        format!(
            "{{ uint64_t s = {src}; FL.cf = FL.of = FL.sf = FL.af = 0; if (s == 0) {{ FL.zf = 1; FL.pf = 1; }} else {{ uint64_t k = {idx}; {wr} FL.zf = 0; FL.pf = FLE_UNDEF; }} }}"
        ),
    ))
}

#[derive(Clone, Copy)]
enum Bit {
    Set,
    Reset,
    Flip,
}

/// bts / btr / btc, with a register or an imm8 bit offset.
/// Register base: the offset is taken modulo the operand size.
/// Memory base with a register offset: the offset is signed and the bit is bit
/// (offset mod 8) of the byte at EA + floor(offset / 8). A byte access reaches the
/// same bit as the SDM's operand-size access.
/// Memory base with an imm8: the offset is taken modulo the operand size, at EA.
fn bit_rmw(i: &Instruction, op: Bit) -> Lift {
    super::need(i.op_count() == 2, "bit operand count")?;
    let sz = super::op_size(i, 0)?;
    super::need(matches!(sz, 2 | 4 | 8), "bit operand size")?;
    let w = sz * 8;
    let mask = (w - 1) as u64;
    let new = |v: &str, b: &str| -> String {
        match op {
            Bit::Set => format!("({v}) | (UINT64_C(1) << {b})"),
            Bit::Reset => format!("({v}) & ~(UINT64_C(1) << {b})"),
            Bit::Flip => format!("({v}) ^ (UINT64_C(1) << {b})"),
        }
    };
    let flags = "FL.pf = FL.af = FL.sf = FL.of = FLE_UNDEF;";
    let mem = i.op_kind(0) == OpKind::Memory;
    super::need(mem || i.op_kind(0) == OpKind::Register, "bit base operand")?;
    let body = if i.op_kind(1) == OpKind::Register {
        super::need(super::op_size(i, 1)? == sz, "bit offset size")?;
        let off = super::reg_read(i.op_register(1))?;
        if mem && i.has_lock_prefix() {
            // LOCK: the byte that holds the bit changes in one compare-and-swap (runtime/fle_rmw.h)
            format!(
                "{{ int64_t o = fle_sext({w}, {off}); uint64_t a = {} + (uint64_t)(o >> 3); unsigned b = (unsigned)((uint64_t)o & 7); uint64_t v, nv; do {{ v = fle_ld(a, 1); nv = {}; }} while (fle_cas(a, 1, v, nv) != v); FL.cf = (uint8_t)((v >> b) & 1); {flags} }}",
                super::ea(i)?,
                new("v", "b")
            )
        } else if mem {
            format!(
                "{{ int64_t o = fle_sext({w}, {off}); uint64_t a = {} + (uint64_t)(o >> 3); unsigned b = (unsigned)((uint64_t)o & 7); uint64_t v = fle_ld(a, 1); FL.cf = (uint8_t)((v >> b) & 1); fle_st(a, 1, {}); {flags} }}",
                super::ea(i)?,
                new("v", "b")
            )
        } else {
            let d = i.op_register(0);
            format!(
                "{{ unsigned b = (unsigned)(({off}) & {mask}); uint64_t v = {}; FL.cf = (uint8_t)((v >> b) & 1); {} {flags} }}",
                super::reg_read(d)?,
                super::reg_write(d, &new("v", "b"))?
            )
        }
    } else if super::is_imm(i.op_kind(1)) {
        let b = (i.immediate(1) & mask).to_string();
        if mem && i.has_lock_prefix() {
            format!(
                "{{ uint64_t a = {}; uint64_t v, nv; do {{ v = fle_ld(a, {sz}); nv = {}; }} while (fle_cas(a, {sz}, v, nv) != v); FL.cf = (uint8_t)((v >> {b}) & 1); {flags} }}",
                super::ea(i)?,
                new("v", &b)
            )
        } else if mem {
            format!(
                "{{ uint64_t a = {}; uint64_t v = fle_ld(a, {sz}); FL.cf = (uint8_t)((v >> {b}) & 1); fle_st(a, {sz}, {}); {flags} }}",
                super::ea(i)?,
                new("v", &b)
            )
        } else {
            let d = i.op_register(0);
            format!(
                "{{ uint64_t v = {}; FL.cf = (uint8_t)((v >> {b}) & 1); {} {flags} }}",
                super::reg_read(d)?,
                super::reg_write(d, &new("v", &b))?
            )
        }
    } else {
        return Err("bit offset operand".into());
    };
    let name = match op {
        Bit::Set => "bts",
        Bit::Reset => "btr",
        Bit::Flip => "btc",
    };
    Ok((name, body))
}

/// rol / ror with CL, imm8, or the implicit 1. The count is masked to 5 bits (6 for
/// a 64-bit operand); the rotation is by (masked count mod size). A register
/// destination is always written, so a 32-bit one is zero-extended even for count 0.
fn rotate(i: &Instruction, left: bool) -> Lift {
    super::need(i.op_count() == 2, "rotate operand count")?;
    let sz = super::op_size(i, 0)?;
    let w = sz * 8;
    let cnt = match i.op_kind(1) {
        OpKind::Register => {
            super::need(i.op_register(1) == Register::CL, "rotate count register must be CL")?;
            "((uint8_t)R[1])".to_string()
        }
        k if super::is_imm(k) => (i.immediate(1) & 0xff).to_string(),
        _ => return Err("rotate count operand".into()),
    };
    let cmask = if w == 64 { 0x3f } else { 0x1f };
    let m = if w == 64 { "UINT64_MAX".to_string() } else { format!("UINT64_C({:#x})", (1u64 << w) - 1) };
    let (decl, rd, wr) = match i.op_kind(0) {
        OpKind::Register => {
            let r = i.op_register(0);
            (String::new(), super::reg_read(r)?, super::reg_write(r, "r")?)
        }
        OpKind::Memory => (format!("uint64_t a = {};", super::ea(i)?), format!("fle_ld(a, {sz})"), format!("fle_st(a, {sz}, r);")),
        _ => return Err("rotate destination operand".into()),
    };
    let (rot, cf, of) = if left {
        (
            format!("((v << k) | (v >> ({w} - k))) & {m}"),
            "(r & 1)".to_string(),
            format!("(((r >> {}) & 1) ^ FL.cf)", w - 1),
        )
    } else {
        (
            format!("((v >> k) | (v << ({w} - k))) & {m}"),
            format!("((r >> {}) & 1)", w - 1),
            format!("(((r >> {}) & 1) ^ ((r >> {}) & 1))", w - 1, w - 2),
        )
    };
    Ok((
        if left { "rol" } else { "ror" },
        format!(
            "{{ unsigned c = (unsigned)({cnt}) & {cmask}; {decl} uint64_t v = {rd}; uint64_t r = v; if (c != 0) {{ unsigned k = c % {w}; if (k != 0) r = {rot}; FL.cf = (uint8_t){cf}; FL.of = c == 1 ? (uint8_t){of} : FLE_UNDEF; }} {wr} }}"
        ),
    ))
}

/// xchg r/m, r and xchg r, r/m (86, 87) and the short xchg rAX, r forms. Both
/// values are read before either is written; the memory address is computed once.
/// A 32-bit register write zero-extends, also when both operands are the same register.
fn xchg(i: &Instruction) -> Lift {
    super::need(i.op_count() == 2, "xchg operand count")?;
    let k0 = i.op_kind(0);
    let k1 = i.op_kind(1);
    if k0 == OpKind::Register && k1 == OpKind::Register {
        let (a, b) = (i.op_register(0), i.op_register(1));
        let sz = super::op_size(i, 0)?;
        super::need(super::op_size(i, 1)? == sz, "xchg operand sizes")?;
        let body = format!(
            "{{ uint64_t x = {}; uint64_t y = {}; {} {} }}",
            super::reg_read(a)?,
            super::reg_read(b)?,
            super::reg_write(a, "y")?,
            super::reg_write(b, "x")?
        );
        return Ok(("xchg", body));
    }
    let (reg, mem_first) = match (k0, k1) {
        (OpKind::Memory, OpKind::Register) => (i.op_register(1), true),
        (OpKind::Register, OpKind::Memory) => (i.op_register(0), false),
        _ => return Err("xchg operand form".into()),
    };
    let sz = super::op_size(i, if mem_first { 1 } else { 0 })?;
    super::need(i.memory_size().size() == sz, "xchg memory size")?;
    let body = format!(
        "{{ uint64_t a = {}; uint64_t y = {}; uint64_t x = fle_xchg(a, {sz}, y); {} }}",   // XCHG with memory is locked implicitly: one atomic exchange (runtime/fle_rmw.h)
        super::ea(i)?,
        super::reg_read(reg)?,
        super::reg_write(reg, "x")?
    );
    Ok(("xchg", body))
}

/// bswap r32 / r64 (decision 0047). SDM BSWAP: the byte order of the 32-bit or 64-bit register is reversed; no flags.
/// A 32-bit write zero-extends into the upper half. The 16-bit form is "undefined" in the SDM: not lifted.
fn bswap(i: &Instruction) -> Lift {
    super::need(i.op_count() == 1 && i.op_kind(0) == OpKind::Register, "bswap operand form")?;
    let r = i.op_register(0);
    let body = match r.size() {
        4 => super::reg_write(r, &format!("__builtin_bswap32((uint32_t){})", super::reg_read(r)?))?,
        8 => super::reg_write(r, &format!("__builtin_bswap64({})", super::reg_read(r)?))?,
        _ => return Err("bswap 16-bit form (result undefined in the SDM)".into()),
    };
    Ok(("bswap", body))
}

/// cmpxchg16b m128 (REX.W 0F C7 /1). RDX:RAX is compared with the 16 bytes; equal:
/// RCX:RBX is stored and ZF = 1; otherwise RDX:RAX gets the memory value and ZF = 0.
/// Other flags are unchanged. An address that is not 16-byte aligned is #GP in the
/// SDM: the run stops.
fn cmpxchg16b(i: &Instruction) -> Lift {
    super::need(i.op_count() == 1 && i.op_kind(0) == OpKind::Memory, "cmpxchg16b operand must be m128")?;
    let body = format!(
        "{{ uint64_t a = {}; if ((a & 15) != 0) {{ fprintf(stderr, \"fle: cmpxchg16b operand is not 16-byte aligned (#GP)\\n\"); abort(); }} uint64_t lo = R[0], hi = R[2]; int eq = fle_cmpxchg16b(a, &lo, &hi, R[3], R[1]); if (eq) {{ FL.zf = 1; }} else {{ R[0] = lo; R[2] = hi; FL.zf = 0; }} }}",
        super::ea(i)?
    );
    Ok(("cmpxchg16b", body))
}

/// jecxz / jrcxz rel8. Taken when ECX (RCX) is 0. The target is a label in the same
/// function, or another function entry (tail call), the same as for a Jcc.
fn jcxz(i: &Instruction, entries: &BTreeSet<u64>) -> Lift {
    super::need(i.op_count() == 1 && super::is_near(i), "jcxz operand must be a near target")?;
    let t = i.near_branch_target();
    let cond = if i.mnemonic() == Mnemonic::Jecxz { "(uint32_t)R[1] == 0" } else { "R[1] == 0" };
    let go = if entries.contains(&t) { format!("{{ f_{t:x}(); return; }}") } else { format!("goto L_{t:x};") };
    Ok((if i.mnemonic() == Mnemonic::Jecxz { "jecxz" } else { "jrcxz" }, format!("if ({cond}) {go} ")))
}
