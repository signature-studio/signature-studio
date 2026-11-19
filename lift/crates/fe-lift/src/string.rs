//! String instructions: MOVS, STOS, LODS and SCAS with an optional REP, REPE
//! or REPNE prefix, and CLD and STD. Element sizes are 1, 2, 4 or 8 bytes.
//!
//! Rules (SDM Vol. 2: MOVS, STOS, LODS, SCAS, REP, CLD, STD):
//! - RSI and RDI change by the element size: up when DF = 0, down when DF = 1.
//! - REP repeats while RCX != 0 and decrements RCX per element. RCX = 0 does
//!   nothing.
//! - SCAS with REPE (F3) stops when ZF = 0. With REPNE (F2) it stops when ZF = 1.
//!   SCAS sets all six flags as CMP does (accumulator - element).
//! - MOVS, STOS and LODS leave the status flags unchanged.
//!
//! DF is `fle_df` (runtime/fle_string.h, defined in runtime/fle_string.c). It is
//! not a field of fle_eflags, because fle_flags.h has no DF. The generated body
//! declares it with `extern`, so the link must include fle_string.c.
//!
//! Not modelled, so they return Err: a 32-bit address (0x67), FS or GS segment,
//! and REPNE on MOVS, STOS or LODS (not checked against hardware here).

use iced_x86::{Code, Instruction, Mnemonic, OpKind, Register};

type Lift = Result<(&'static str, String), String>;

pub fn lift(i: &Instruction) -> Option<Lift> {
    let r = match i.mnemonic() {
        Mnemonic::Movsb => movs(i, 1),
        Mnemonic::Movsw => movs(i, 2),
        Mnemonic::Movsd if is_string_movsd(i) => movs(i, 4),
        Mnemonic::Movsq => movs(i, 8),
        Mnemonic::Stosb => stos(i, 1),
        Mnemonic::Stosw => stos(i, 2),
        Mnemonic::Stosd => stos(i, 4),
        Mnemonic::Stosq => stos(i, 8),
        Mnemonic::Lodsb => lods(i, 1),
        Mnemonic::Lodsd => lods(i, 4),
        Mnemonic::Lodsq => lods(i, 8),
        Mnemonic::Scasb => scas(i, 1),
        Mnemonic::Scasd => scas(i, 4),
        Mnemonic::Cld => Ok(("cld", "{ extern uint8_t fle_df; fle_df = 0; }".into())),
        Mnemonic::Std => Ok(("std", "{ extern uint8_t fle_df; fle_df = 1; }".into())),
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

/// MOVSD is also the SSE2 scalar move. The string form is the one whose code is
/// `Movsd_m32_m32` (decoded with the string operands ES:[RDI] and [RSI]).
fn is_string_movsd(i: &Instruction) -> bool {
    i.code() == Code::Movsd_m32_m32
}

#[derive(Clone, Copy, PartialEq)]
enum Rep {
    None,
    /// F3. For SCAS this is REPE.
    Rep,
    /// F2. Only SCAS is modelled with it (REPNE).
    Repne,
}

fn rep_of(i: &Instruction) -> Rep {
    if i.has_repne_prefix() {
        Rep::Repne
    } else if i.has_rep_prefix() {
        Rep::Rep
    } else {
        Rep::None
    }
}

/// Rejects 32-bit addressing (0x67) and FS/GS. Those forms are not modelled.
/// For string operands iced reports the address size in the operand kind:
/// `MemoryESRDI` and `MemorySegRSI` are 64-bit, `MemoryESEDI` and `MemorySegESI`
/// are 32-bit.
fn plain_addr(i: &Instruction) -> Result<(), String> {
    need(!matches!(i.memory_segment(), Register::FS | Register::GS), "FS/GS segment")?;
    for k in 0..i.op_count() {
        let bad = matches!(i.op_kind(k), OpKind::MemoryESEDI | OpKind::MemorySegESI);
        need(!bad, "32-bit string addressing")?;
    }
    Ok(())
}

/// Advances register `r` (RSI = 6, RDI = 7) by `n` bytes, direction from DF.
fn step(r: usize, n: u32) -> String {
    format!("R[{r}] = fle_df ? R[{r}] - {n} : R[{r}] + {n};")
}

/// Puts one element step inside the repeat loop for the prefix. The step is
/// `elem`, and the loop counter RCX is decremented after each element.
/// For SCAS, the loop also tests ZF after each element.
fn wrap(rep: Rep, scas: bool, elem: &str) -> Result<String, String> {
    let body = match (rep, scas) {
        (Rep::None, _) => elem.to_string(),
        (Rep::Rep, false) => format!("while (R[1] != 0) {{ {elem} R[1]--; }}"),
        (Rep::Rep, true) => format!("while (R[1] != 0) {{ {elem} R[1]--; if (!FL.zf) break; }}"),
        (Rep::Repne, true) => format!("while (R[1] != 0) {{ {elem} R[1]--; if (FL.zf) break; }}"),
        (Rep::Repne, false) => return Err("REPNE on movs/stos/lods".into()),
    };
    Ok(format!("{{ extern uint8_t fle_df; {body} }}"))
}

fn movs(i: &Instruction, n: u32) -> Lift {
    plain_addr(i)?;
    let elem = format!("fle_st(R[7], {n}, fle_ld(R[6], {n})); {} {}", step(6, n), step(7, n));
    Ok(("movs", wrap(rep_of(i), false, &elem)?))
}

fn stos(i: &Instruction, n: u32) -> Lift {
    plain_addr(i)?;
    let elem = format!("fle_st(R[7], {n}, R[0]); {}", step(7, n));
    Ok(("stos", wrap(rep_of(i), false, &elem)?))
}

/// LODS writes the accumulator with the register-write rules. 8- and 16-bit
/// keep the other bits, 32-bit zero-extends, 64-bit replaces.
fn lods(i: &Instruction, n: u32) -> Lift {
    plain_addr(i)?;
    need(rep_of(i) == Rep::None, "REP on lods (not modelled)")?;
    let acc = match n {
        1 => "R[0] = (R[0] & ~UINT64_C(0xff)) | (uint8_t)v;",
        4 => "R[0] = (uint32_t)v;",
        _ => "R[0] = v;",
    };
    let elem = format!("{{ uint64_t v = fle_ld(R[6], {n}); {} {acc} }}", step(6, n));
    Ok(("lods", wrap(rep_of(i), false, &elem)?))
}

/// SCAS: flags are those of CMP accumulator, element. RDI is advanced.
fn scas(i: &Instruction, n: u32) -> Lift {
    plain_addr(i)?;
    let elem = format!("(void)fle_op_sub({}, R[0], fle_ld(R[7], {n}), 0); {}", n * 8, step(7, n));
    Ok(("scas", wrap(rep_of(i), true, &elem)?))
}
