//! Integer instructions: xadd, cmpxchg, bt (bit test) and unsigned div.
//!
//! Flags: xadd and cmpxchg set all six flags as ADD and CMP do. bt sets CF to the
//! selected bit, leaves ZF unchanged, and makes OF, SF, AF and PF undefined. div
//! leaves all six undefined. See docs/decisions/0013-more-insns.md.
//!
//! LOCK is ignored: FLEngine runs on one thread (see the decision record).
//! Divide errors (zero divisor, quotient out of range) stop the run through
//! fle_div_fault (runtime/fle_int_more.c), before any register is written.

use iced_x86::{Instruction, Mnemonic, OpKind, Register};

type Lift = Result<(&'static str, String), String>;

/// Lifts the instructions owned by this module. `None` leaves the instruction to `lib.rs`.
pub fn lift(i: &Instruction) -> Option<Lift> {
    match i.mnemonic() {
        Mnemonic::Xadd => Some(xadd(i)),
        Mnemonic::Cmpxchg => Some(cmpxchg(i)),
        Mnemonic::Bt => Some(bt(i)),
        Mnemonic::Div => Some(div(i)),
        _ => None,
    }
}

/// The destination of xadd and cmpxchg. A memory address is computed once, before
/// any register is written, so that a write to a base register does not move it.
struct Dest {
    decl: String,
    read: String,
    reg: Option<Register>,
    sz: usize,
}

impl Dest {
    fn new(i: &Instruction, sz: usize) -> Result<Dest, String> {
        if i.op_kind(0) == OpKind::Memory {
            Ok(Dest { decl: format!("uint64_t a = {};", ea(i)?), read: format!("fle_ld(a, {sz})"), reg: None, sz })
        } else {
            need(i.op_kind(0) == OpKind::Register, "destination operand kind")?;
            let r = i.op_register(0);
            Ok(Dest { decl: String::new(), read: reg_rd(r)?, reg: Some(r), sz })
        }
    }

    fn write(&self, v: &str) -> Result<String, String> {
        match self.reg {
            Some(r) => reg_wr(r, v),
            None => Ok(format!("fle_st(a, {}, {v});", self.sz)),
        }
    }
}

/// xadd r/m, r. TEMP := SRC + DEST; SRC := DEST; DEST := TEMP.
fn xadd(i: &Instruction) -> Lift {
    need(i.op_count() == 2 && i.op_kind(1) == OpKind::Register, "xadd operand form")?;
    let sz = opsize(i, 0)?;
    need(opsize(i, 1)? == sz, "xadd operand sizes")?;
    let w = sz * 8;
    let d = Dest::new(i, sz)?;
    if d.reg.is_none() {
        // A memory XADD is one atomic fetch-and-add (runtime/fle_rmw.h), with or without the LOCK prefix: guest threads may run it at the same time on the same word.
        let body = format!("{{ {} uint64_t sv = {}; uint64_t dv = fle_xadd(a, {sz}, sv); (void)fle_op_add({w}, dv, sv, 0); {} }}", d.decl, rd(i, 1, sz)?, reg_wr(i.op_register(1), "dv")?);
        return Ok(("xadd", body));
    }
    let body = format!(
        "{{ {} uint64_t dv = {}; uint64_t sv = {}; uint64_t sum = fle_op_add({w}, dv, sv, 0); {} {} }}",
        d.decl,
        d.read,
        rd(i, 1, sz)?,
        reg_wr(i.op_register(1), "dv")?,
        d.write("sum")?
    );
    Ok(("xadd", body))
}

/// cmpxchg r/m, r. Compares the accumulator with the destination (flags as CMP).
/// Equal: the destination gets the source. Otherwise the accumulator gets the
/// destination, and the destination is written back unchanged.
fn cmpxchg(i: &Instruction) -> Lift {
    need(i.op_count() == 2 && i.op_kind(1) == OpKind::Register, "cmpxchg operand form")?;
    let sz = opsize(i, 0)?;
    need(opsize(i, 1)? == sz, "cmpxchg operand sizes")?;
    let w = sz * 8;
    let acc = match sz {
        1 => Register::AL,
        2 => Register::AX,
        4 => Register::EAX,
        _ => Register::RAX,
    };
    let d = Dest::new(i, sz)?;
    if d.reg.is_none() {
        // A memory CMPXCHG is one atomic compare-and-swap (runtime/fle_rmw.h); the flags are those of CMP accumulator, old value; a failed exchange loads the accumulator.
        let body = format!("{{ {} uint64_t sv = {}; uint64_t acc = {}; uint64_t dv = fle_cas(a, {sz}, acc, sv); (void)fle_op_sub({w}, acc, dv, 0); if (acc != dv) {{ {} }} }}", d.decl, rd(i, 1, sz)?, reg_rd(acc)?, reg_wr(acc, "dv")?);
        return Ok(("cmpxchg", body));
    }
    let body = format!(
        "{{ {} uint64_t dv = {}; uint64_t sv = {}; uint64_t acc = {}; (void)fle_op_sub({w}, acc, dv, 0); if (acc == dv) {{ {} }} else {{ {} {} }} }}",
        d.decl,
        d.read,
        rd(i, 1, sz)?,
        reg_rd(acc)?,
        d.write("sv")?,
        reg_wr(acc, "dv")?,
        d.write("dv")?
    );
    Ok(("cmpxchg", body))
}

/// bt r/m, r and bt r/m, imm8 (no lock). CF gets the selected bit.
/// Register base: the offset is taken modulo the operand size.
/// Memory base with a register offset: the offset is signed; the byte at
/// EA + floor(offset / 8) holds the bit, at position offset mod 8.
/// Memory base with an immediate: the offset is taken modulo the operand size, at EA.
fn bt(i: &Instruction) -> Lift {
    need(i.op_count() == 2, "bt operand count")?;
    let sz = opsize(i, 0)?;
    need(matches!(sz, 2 | 4 | 8), "bt operand size")?;
    let w = sz * 8;
    let mask = w - 1;
    let mem = i.op_kind(0) == OpKind::Memory;
    let body = if i.op_kind(1) == OpKind::Register {
        need(opsize(i, 1)? == sz, "bt offset size")?;
        let off = rd(i, 1, sz)?;
        if mem {
            format!(
                "{{ void fle_bt_cf(uint8_t); int64_t o = fle_sext({w}, {off}); uint64_t a = {} + (uint64_t)(o >> 3); fle_bt_cf((uint8_t)((fle_ld(a, 1) >> ((uint64_t)o & 7)) & 1)); }}",
                ea(i)?
            )
        } else {
            format!(
                "{{ void fle_bt_cf(uint8_t); uint64_t v = {}; fle_bt_cf((uint8_t)((v >> (({off}) & {mask})) & 1)); }}",
                rd(i, 0, sz)?
            )
        }
    } else if is_imm(i.op_kind(1)) {
        let bit = i.immediate(1) & mask as u64;
        if mem {
            format!("{{ void fle_bt_cf(uint8_t); fle_bt_cf((uint8_t)((fle_ld({}, {sz}) >> {bit}) & 1)); }}", ea(i)?)
        } else {
            format!("{{ void fle_bt_cf(uint8_t); fle_bt_cf((uint8_t)((({}) >> {bit}) & 1)); }}", rd(i, 0, sz)?)
        }
    } else {
        return Err("bt offset operand".into());
    };
    Ok(("bt", body))
}

/// div r/m8, r/m16, r/m32, r/m64 (unsigned). The dividend is AX, DX:AX, EDX:EAX or
/// RDX:RAX. The quotient goes to AL, AX, EAX or RAX and the remainder to AH, DX, EDX or RDX.
fn div(i: &Instruction) -> Lift {
    need(i.op_count() == 1, "div operand count")?;
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
        "div",
        format!(
            "{{ uint64_t fle_div(unsigned, unsigned __int128, uint64_t, uint64_t *); uint64_t rem; uint64_t q = fle_div({w}, {dvd}, ({src}), &rem); {wr} }}"
        ),
    ))
}

fn need(c: bool, msg: &str) -> Result<(), String> {
    if c {
        Ok(())
    } else {
        Err(msg.to_string())
    }
}

fn is_imm(k: OpKind) -> bool {
    matches!(
        k,
        OpKind::Immediate8
            | OpKind::Immediate8to16
            | OpKind::Immediate8to32
            | OpKind::Immediate8to64
            | OpKind::Immediate16
            | OpKind::Immediate32
            | OpKind::Immediate32to64
            | OpKind::Immediate64
    )
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

/// Register value as a C expression of the register's width (zero-extended).
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

/// Register write of the register's width. A 32-bit write zero-extends; 8- and
/// 16-bit writes keep the other bits. AH, BH, CH and DH are bits 8..15.
fn reg_wr(r: Register, v: &str) -> Result<String, String> {
    let n = reg_num(r)?;
    if is_high(r) {
        return Ok(format!("R[{n}] = (R[{n}] & ~UINT64_C(0xff00)) | ((uint64_t)(uint8_t)({v}) << 8);"));
    }
    Ok(match r.size() {
        8 => format!("R[{n}] = {v};"),
        4 => format!("R[{n}] = (uint32_t)({v});"),
        2 => format!("R[{n}] = (R[{n}] & ~UINT64_C(0xffff)) | (uint16_t)({v});"),
        _ => format!("R[{n}] = (R[{n}] & ~UINT64_C(0xff)) | (uint8_t)({v});"),
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

/// Operand value of `size` bytes as a C expression (zero-extended).
fn rd(i: &Instruction, n: u32, size: usize) -> Result<String, String> {
    match i.op_kind(n) {
        OpKind::Register => reg_rd(i.op_register(n)),
        OpKind::Memory => Ok(format!("fle_ld({}, {size})", ea(i)?)),
        _ => Err("operand kind".into()),
    }
}

/// Effective address as a C expression. Never loads from memory.
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
