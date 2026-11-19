//! Status flags without a flags register. Every instruction that modifies flags is a definition; a forward reaching-definitions analysis
//! (per flag) tells each reader (jcc, setcc, cmovcc) which definition it sees. A reader that sees one definition is compiled from the
//! operands that the definition saved in locals (`cmp a,b; jl` becomes `a <s b`). A reader that sees several definitions of a flag reads
//! a local that each of them materialised. A flag that nothing reads costs nothing. The lazy-flags state of the C path
//! (`fle_op_*`, `fle_cc`, global `FL`) does not exist here.

use crate::cfg::Cfg;
use crate::fx::*;
use crate::ir::Fun;
use iced_x86::{Instruction, Mnemonic, OpKind};
use std::collections::{BTreeSet, HashMap};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Add,
    Sub,
    Logic,
    Inc,
    Dec,
    Neg,
    /// adc and sbb: the saved operands are a, b and the carry-in
    Adc,
    Sbb,
    Shl(u8),
    Shr(u8),
    Sar(u8),
    /// decision 0270 (mt3w): a shift by cl. The count (masked to 5 or 6 bits) is saved with the operand and the result; a count of 0 leaves every flag as it was (the definition is `weak`);
    /// CF, ZF, SF and PF are defined for counts 1..width (the C path's `fle_op_shl/shr/sar`: a larger count of an 8 or 16 bit operand stops the run there), OF is not (only a count of 1 defines it)
    ShlV,
    ShrV,
    SarV,
    /// bsf/bsr: ZF = (source == 0), the saved operand is the source; the other flags are undefined (not modelled)
    BitScan,
    /// bt/bts/btr/btc: CF is the bit, saved as an i32 0/1
    BitTest,
    /// ucomisd/comisd/ucomiss/comiss: the saved operands are the bit patterns of the two scalars (w = 8: double, w = 4: single); CF, ZF, PF from the comparison, OF = SF = 0
    FCmp,
    /// the flags of an instruction that is compiled as `unreachable` (a VEX path under TRAP_VEX): any value
    Dead,
    Opaque,
    /// decision 0270 (mt3w): the flags of a `call` of a function that returns with known flags (`flagout::summary`): (mask, values) of the known flags; no saved operands
    Known(u8, u8),
}

#[derive(Clone, Debug)]
pub struct Def {
    pub va: u64,
    pub kind: Kind,
    /// Operand size in bytes.
    pub w: u8,
    pub modified: u8,
    pub defined: u8,
    /// Some reader builds its condition from this definition's saved operands.
    pub need_save: bool,
    /// Flags a reader needs from a local because several definitions reach it.
    pub mat: u8,
    /// A definition that may not execute (a REP string compare with RCX = 0): it does not kill the earlier definitions.
    pub weak: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Src {
    Def(u32),
    Mat,
}

#[derive(Default)]
pub struct FlagInfo {
    pub defs: Vec<Def>,
    pub def_of: HashMap<u64, u32>,
    /// reader VA -> source of each flag it reads (indexed by flag_index)
    pub reader: HashMap<u64, [Option<Src>; 6]>,
}

pub const ENTRY: u32 = u32::MAX;

pub fn opsize0(i: &Instruction) -> u8 {
    match i.mnemonic() {
        Mnemonic::Scasb | Mnemonic::Cmpsb => return 1,
        Mnemonic::Scasw | Mnemonic::Cmpsw => return 2,
        Mnemonic::Scasd => return 4,
        Mnemonic::Scasq | Mnemonic::Cmpsq => return 8,
        _ => {}
    }
    match i.op_kind(0) {
        OpKind::Register => i.op_register(0).size() as u8,
        OpKind::Memory => i.memory_size().size() as u8,
        _ => 8,
    }
}

/// Decision 0164 (OTT pilot): VEX/EVEX instructions are the CPUID-selected alternative paths of the MSVC code (AVX/FMA); the machine runs the SSE2 path. With this switch they are
/// compiled as `unreachable` (the compare instructions as what they are), so that a function that has such a path can be compiled.

pub fn is_vex(i: &Instruction) -> bool {
    use iced_x86::EncodingKind as E;
    matches!(i.code().encoding(), E::VEX | E::EVEX | E::XOP)
}

pub fn classify(i: &Instruction) -> Kind {
    let n = i.op_count();
    if is_vex(i) && !matches!(i.mnemonic(), Mnemonic::Vucomisd | Mnemonic::Vcomisd | Mnemonic::Vucomiss | Mnemonic::Vcomiss) {
        return Kind::Dead;
    }
    match i.mnemonic() {
        Mnemonic::Add | Mnemonic::Xadd => Kind::Add,
        Mnemonic::Sub | Mnemonic::Cmp | Mnemonic::Cmpxchg => Kind::Sub,
        Mnemonic::Adc => Kind::Adc,
        Mnemonic::Sbb => Kind::Sbb,
        Mnemonic::And | Mnemonic::Or | Mnemonic::Xor | Mnemonic::Test => Kind::Logic,
        Mnemonic::Inc => Kind::Inc,
        Mnemonic::Dec => Kind::Dec,
        Mnemonic::Neg => Kind::Neg,
        Mnemonic::Bt | Mnemonic::Bts | Mnemonic::Btr | Mnemonic::Btc => Kind::BitTest,
        Mnemonic::Bsf | Mnemonic::Bsr => Kind::BitScan,
        Mnemonic::Ucomisd | Mnemonic::Comisd | Mnemonic::Ucomiss | Mnemonic::Comiss | Mnemonic::Vucomisd | Mnemonic::Vcomisd | Mnemonic::Vucomiss | Mnemonic::Vcomiss => Kind::FCmp,
        Mnemonic::Scasb | Mnemonic::Scasw | Mnemonic::Scasd | Mnemonic::Scasq | Mnemonic::Cmpsb | Mnemonic::Cmpsw | Mnemonic::Cmpsq => Kind::Sub,
        Mnemonic::Shl | Mnemonic::Sal | Mnemonic::Shr | Mnemonic::Sar => {
            let cnt = if n == 1 {
                Some(1u8)
            } else if matches!(i.op_kind(1), OpKind::Immediate8 | OpKind::Immediate8to16 | OpKind::Immediate8to32 | OpKind::Immediate8to64) {
                Some(i.immediate(1) as u8)
            } else {
                None
            };
            match cnt {
                Some(c) => {
                    let mask = if opsize0(i) == 8 { 63 } else { 31 };
                    let c = c & mask;
                    let w = opsize0(i) * 8;
                    if c == 0 || c > w {
                        Kind::Opaque
                    } else {
                        match i.mnemonic() {
                            Mnemonic::Shr => Kind::Shr(c),
                            Mnemonic::Sar => Kind::Sar(c),
                            _ => Kind::Shl(c),
                        }
                    }
                }
                None if i.op_count() == 2 && i.op_kind(1) == OpKind::Register && i.op_register(1) == iced_x86::Register::CL => match i.mnemonic() {
                    Mnemonic::Shr => Kind::ShrV,
                    Mnemonic::Sar => Kind::SarV,
                    _ => Kind::ShlV,
                },
                None => Kind::Opaque,
            }
        }
        _ => Kind::Opaque,
    }
}

/// Which flags a recipe of this kind can produce.
pub fn recipe_flags(k: Kind) -> u8 {
    match k {
        Kind::Add | Kind::Sub | Kind::Neg | Kind::Adc | Kind::Sbb => CF | PF | ZF | SF | OF,
        Kind::Logic => CF | PF | ZF | SF | OF,
        Kind::Inc | Kind::Dec => PF | ZF | SF | OF,
        Kind::Shl(1) | Kind::Shr(1) | Kind::Sar(1) => CF | PF | ZF | SF | OF,
        Kind::Shl(_) | Kind::Shr(_) | Kind::Sar(_) | Kind::ShlV | Kind::ShrV | Kind::SarV => CF | PF | ZF | SF,
        Kind::BitTest => CF,
        Kind::BitScan => ZF,
        Kind::FCmp | Kind::Dead => CF | PF | ZF | SF | OF,
        Kind::Opaque => 0,
        Kind::Known(m, _) => m,
    }
}

type State = [BTreeSet<u32>; 6];

fn mn(f: &Fun, va: u64) -> String {
    if f.nodes.contains_key(&va) {
        format!("{:?}", f.insn(va).mnemonic())
    } else {
        "?".into()
    }
}

pub fn analyze(f: &Fun, cfg: &Cfg, fx: &mut Fx) -> Result<FlagInfo, String> {
    analyze_with(f, cfg, fx, &|_| None)
}

/// `known(callee)`: the flags that the function at `callee` returns with (decision 0270, `flagout::summary`)
pub fn analyze_with(f: &Fun, cfg: &Cfg, fx: &mut Fx, known: &dyn Fn(u64) -> Option<(u8, u8)>) -> Result<FlagInfo, String> {
    let mut info = FlagInfo::default();
    // definitions
    for b in &f.blocks {
        for &va in &b.insns {
            let i = f.insn(va);
            let e = fx.eff(i);
            let is_call = i.mnemonic() == Mnemonic::Call;
            if e.flags_w != 0 || is_call {
                let k_out = if is_call && matches!(i.op0_kind(), OpKind::NearBranch64) { known(i.near_branch_target()) } else { None };
                let kind = if let Some((m, v)) = k_out { Kind::Known(m, v) } else if is_call { Kind::Opaque } else { classify(i) };
                let (modified, defined) = if let Some((m, _)) = k_out { (ALL_FLAGS, m) } else if is_call { (ALL_FLAGS, 0) } else { (e.flags_w, e.flags_def) };
                let id = info.defs.len() as u32;
                info.def_of.insert(va, id);
                // a kind the recipes cover defines exactly the flags that the instruction writes
                let weak = !is_call && (i.has_rep_prefix() || i.has_repne_prefix() || matches!(kind, Kind::ShlV | Kind::ShrV | Kind::SarV));
                let w = if kind == Kind::FCmp { if matches!(i.mnemonic(), Mnemonic::Ucomisd | Mnemonic::Comisd | Mnemonic::Vucomisd | Mnemonic::Vcomisd) { 8 } else { 4 } } else { opsize0(i) };
                info.defs.push(Def { va, kind, w, modified, defined, need_save: false, mat: 0, weak });
            }
        }
    }
    let n = f.blocks.len();
    let mut ins: Vec<Option<State>> = vec![None; n];
    let mut entry: State = Default::default();
    for s in entry.iter_mut() {
        s.insert(ENTRY);
    }
    ins[0] = Some(entry);
    let step = |st: &mut State, va: u64, fx: &mut Fx, info: &FlagInfo, f: &Fun| {
        let i = f.insn(va);
        let _ = fx;
        if let Some(&id) = info.def_of.get(&va) {
            let d = &info.defs[id as usize];
            for k in 0..6 {
                if d.modified & (1 << k) != 0 {
                    if !d.weak {
                        st[k].clear();
                    }
                    st[k].insert(id);
                }
            }
        }
        let _ = i;
    };
    let mut work: Vec<usize> = cfg.rpo.clone();
    work.reverse();
    let mut on = vec![false; n];
    for &b in &work {
        on[b] = true;
    }
    while let Some(b) = work.pop() {
        on[b] = false;
        let Some(mut st) = ins[b].clone() else { continue };
        for &va in &f.blocks[b].insns {
            step(&mut st, va, fx, &info, f);
        }
        for &s in &f.blocks[b].succs {
            let changed = match &mut ins[s] {
                None => {
                    ins[s] = Some(st.clone());
                    true
                }
                Some(cur) => {
                    let mut ch = false;
                    for k in 0..6 {
                        for &x in &st[k] {
                            ch |= cur[k].insert(x);
                        }
                    }
                    ch
                }
            };
            if changed && !on[s] {
                on[s] = true;
                work.push(s);
            }
        }
    }
    // readers
    let mut need_save: Vec<u32> = Vec::new();
    let mut mat: Vec<(u32, u8)> = Vec::new();
    for (bi, b) in f.blocks.iter().enumerate() {
        let Some(mut st) = ins[bi].clone() else { continue };
        for &va in &b.insns {
            let i = f.insn(va);
            let e = fx.eff(i);
            let mut flags_r = e.flags_r;
            if i.mnemonic() == Mnemonic::Call {
                flags_r = 0;
            }
            if flags_r != 0 {
                let mut srcs: [Option<Src>; 6] = [None; 6];
                for k in 0..6 {
                    if flags_r & (1 << k) == 0 {
                        continue;
                    }
                    let flag = 1u8 << k;
                    if flag == AF {
                        return Err(format!("reads AF @ {va:#x}"));
                    }
                    let set = &st[k];
                    if set.len() == 1 {
                        let d = *set.iter().next().unwrap();
                        if d == ENTRY {
                            return Err(format!("reads a flag that is live into the function @ {va:#x}"));
                        }
                        let def = &info.defs[d as usize];
                        if def.kind == Kind::Opaque || def.defined & flag == 0 || recipe_flags(def.kind) & flag == 0 {
                            return Err(format!("flag read from a definition without recipe: {} {:?} flag {flag:#x} @ {va:#x}", mn(f, def.va), def.kind));
                        }
                        srcs[k] = Some(Src::Def(d));
                        need_save.push(d);
                    } else {
                        for &d in set {
                            if d == ENTRY {
                                return Err(format!("reads a flag that is live into the function on one path @ {va:#x}"));
                            }
                            let def = &info.defs[d as usize];
                            if def.kind == Kind::Opaque || def.defined & flag == 0 || recipe_flags(def.kind) & flag == 0 {
                                return Err(format!("flag merge from a definition without recipe: {} {:?} flag {flag:#x} @ {va:#x}", mn(f, def.va), def.kind));
                            }
                            mat.push((d, flag));
                        }
                        srcs[k] = Some(Src::Mat);
                    }
                }
                info.reader.insert(va, srcs);
            }
            step(&mut st, va, fx, &info, f);
        }
    }
    for d in need_save {
        info.defs[d as usize].need_save = true;
    }
    for (d, fl) in mat {
        info.defs[d as usize].mat |= fl;
        info.defs[d as usize].need_save = true;
    }
    Ok(info)
}
