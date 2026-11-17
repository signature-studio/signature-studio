//! Which registers does a function give back unchanged, and which register saves can be left out?
//!
//! A forward analysis keeps, for each register, whether it still holds its value at function entry (`Hold(r)`), a pointer into the frame
//! (`Rel(k)`: entry RSP + k), or something else, and for each stack slot below the entry RSP what it holds. `push rbx ... pop rbx` and
//! `mov [rsp+d], rbx ... mov rbx, [rsp+d]` pairs show up as a save and a restore of `Hold(rbx)`. At every `ret` the registers that still
//! hold their entry value are preserved. A register that is preserved and whose entry value is used only by those save/restore pairs
//! (never copied, compared, passed, or read from the slot by anything else, no tail jump while it is saved) needs neither a parameter
//! nor a result: its save and restore are not emitted ("elided"), and the caller keeps its value in its own local across the call.
//!
//! Assumption (checked by the differential tests, not by this analysis): a store through a pointer other than RSP/RBP-relative does not
//! overwrite a save slot, and nothing reads a save slot through such a pointer.

use crate::cfg::Cfg;
use crate::fx::*;
use crate::ir::{Fun, Term};
use iced_x86::{Instruction, Mnemonic, OpKind, Register};
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum V {
    Hold(u8),
    Rel(i64),
    /// decision 0261: a constant (`mov eax, 0x1020` before `call __chkstk; sub rsp, rax`): only so that a `sub rsp, reg` / `add rsp, reg` of a known constant keeps RSP a constant offset
    Const(i64),
    Other,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct St {
    pub regs: [V; 16],
    pub rsp: Option<i64>,
    pub slots: BTreeMap<i64, V>,
}

impl St {
    fn entry() -> St {
        let mut regs = [V::Other; 16];
        for r in 0..16 {
            regs[r] = V::Hold(r as u8);
        }
        St { regs, rsp: Some(0), slots: BTreeMap::new() }
    }
    fn join(&mut self, o: &St) -> bool {
        let before = self.clone();
        for r in 0..16 {
            if self.regs[r] != o.regs[r] {
                self.regs[r] = V::Other;
            }
        }
        if self.rsp != o.rsp {
            self.rsp = None;
        }
        self.slots.retain(|k, v| o.slots.get(k) == Some(v));
        *self != before
    }
}

#[derive(Default, Clone, Debug)]
pub struct Events {
    /// registers whose entry value is read by something other than a save
    pub uses_entry: u16,
    pub saves: Vec<(u64, usize)>,
    pub restores: Vec<(u64, usize)>,
    /// registers with a save slot alive at a tail jump
    pub tail_slots: u16,
}

#[derive(Default)]
pub struct PresResult {
    /// registers preserved at every return (RSP excluded)
    pub pres: u16,
    /// RSP is not known to equal its entry value at some `ret` or tail jump (a continuation entry in the middle of another function's frame,
    /// a callee that pops, an unanalysable RSP): the callers' RSP after the call would not be the one they had before it.
    pub unbalanced: bool,
    /// decision 0261: the RSP offset at every `ret` and tail jump (a tail jump adds the callee's own delta), when they all agree on one constant k: the function leaves RSP k bytes above its
    /// entry value (k = 0: balanced, the ordinary case; k != 0: a shared epilogue entered by a `jmp` from another frame, `add rsp, N; pop ...; ret`). None: the offset is not a constant
    /// (a variable `sub rsp, rax`) or two exits disagree.
    pub delta: Option<i64>,
    pub block_in: Vec<Option<St>>,
}

fn slot_of(st: &St, i: &Instruction) -> Option<i64> {
    if i.op_kind(1) != OpKind::Memory && i.op_kind(0) != OpKind::Memory {
        return None;
    }
    if i.is_ip_rel_memory_operand() || i.memory_index() != Register::None || matches!(i.memory_segment(), Register::FS | Register::GS) {
        return None;
    }
    let b = i.memory_base();
    let d = i.memory_displacement64() as i64;
    if b == Register::RSP {
        return st.rsp.map(|o| o + d);
    }
    if b.size() == 8 && b.is_gpr() {
        if let V::Rel(k) = st.regs[gpr(b)?] {
            return Some(k + d);
        }
    }
    None
}

fn mark_uses(st: &St, ev: &mut Events, mut reads: u16) {
    while reads != 0 {
        let r = reads.trailing_zeros() as usize;
        reads &= reads - 1;
        if let V::Hold(q) = st.regs[r] {
            ev.uses_entry |= 1 << q;
        }
    }
}

fn kill_slots(st: &mut St, off: i64, size: i64) {
    st.slots.retain(|&k, _| k + 8 <= off || k >= off + size);
}

/// One instruction.
fn step(st: &mut St, i: &Instruction, va: u64, fx: &mut Fx, pres_of: &dyn Fn(u64) -> Option<u16>, in_of: &dyn Fn(u64) -> u16, rsp_of: &dyn Fn(u64) -> Option<i64>, ev: &mut Events) {
    use Mnemonic as M;
    let e = fx.eff(i);
    let m = i.mnemonic();
    let write_regs = |st: &mut St, w: u16| {
        for r in 0..16 {
            if w & (1 << r) != 0 {
                st.regs[r] = V::Other;
            }
        }
    };
    match m {
        M::Nop | M::Fnop | M::Pause => {}
        M::Push => {
            let off = st.rsp.map(|o| o - 8);
            let mut val = V::Other;
            if i.op_kind(0) == OpKind::Register {
                if let Some(r) = gpr(i.op_register(0)) {
                    if r != RSP {
                        val = st.regs[r];
                        match val {
                            V::Hold(q) if q as usize == r => ev.saves.push((va, r)),
                            V::Hold(q) => ev.uses_entry |= 1 << q,
                            _ => {}
                        }
                    }
                }
            } else {
                mark_uses(st, ev, e.reads);
            }
            match off {
                Some(o) => {
                    kill_slots(st, o, 8);
                    st.slots.insert(o, val);
                    st.rsp = Some(o);
                }
                None => {
                    st.slots.clear();
                    st.rsp = None;
                }
            }
        }
        M::Pop => {
            if i.op_kind(0) == OpKind::Register {
                let r = gpr(i.op_register(0)).unwrap_or(RSP);
                let v = st.rsp.and_then(|o| st.slots.get(&o).copied()).unwrap_or(V::Other);
                if r != RSP {
                    match v {
                        V::Hold(q) if q as usize == r => ev.restores.push((va, r)),
                        V::Hold(q) => ev.uses_entry |= 1 << q,
                        _ => {}
                    }
                    st.regs[r] = v;
                }
            }
            if let Some(o) = st.rsp {
                st.slots.remove(&o);
                st.rsp = Some(o + 8);
            }
        }
        M::Call => {
            let direct = crate::ir::is_near(i);
            let (p, inn) = if direct {
                let t = i.near_branch_target();
                (pres_of(t), in_of(t))
            } else {
                (None, (1 << 1) | (1 << 2) | (1 << 8) | (1 << 9))
            };
            // the arguments are read
            let mut rd = inn | e.reads;
            if !direct {
                rd |= e.reads;
            }
            mark_uses(st, ev, rd & ALL_REGS);
            let keep = match p {
                Some(p) => p,
                // an indirect callee: the callee-saved registers of the Win64 ABI are assumed preserved (this is the one assumption at an unknown call)
                None => (1 << 3) | (1 << 5) | (1 << 6) | (1 << 7) | (0xf << 12),
            };
            for r in 0..16 {
                if r != RSP && keep & (1 << r) == 0 {
                    st.regs[r] = V::Other;
                }
            }
            if let Some(o) = st.rsp {
                kill_slots(st, o, 0x20);
            }
            // decision 0261: a callee that leaves RSP k bytes above its entry value (see `PresResult::delta`) moves the caller's RSP by k; an unknown k (None) loses it
            if direct {
                // (an unknown k is taken as 0, the assumption that was made for every callee before: a callee with a variable offset is not compiled, and a caller of it is left to the C path)
                st.rsp = st.rsp.map(|o| o + rsp_of(i.near_branch_target()).unwrap_or(0));
            }
        }
        M::Leave => {
            match st.regs[5] {
                V::Rel(k) => {
                    let v = st.slots.get(&k).copied().unwrap_or(V::Other);
                    match v {
                        V::Hold(q) if q == 5 => ev.restores.push((va, 5)),
                        V::Hold(q) => ev.uses_entry |= 1 << q,
                        _ => {}
                    }
                    st.regs[5] = v;
                    st.slots.remove(&k);
                    st.rsp = Some(k + 8);
                }
                _ => {
                    st.regs[5] = V::Other;
                    st.rsp = None;
                }
            }
        }
        M::Mov if i.op_count() == 2 => {
            // stack slot accesses and copies of RSP
            let slot = slot_of(st, i);
            if i.op_kind(0) == OpKind::Memory {
                let size = i.memory_size().size() as i64;
                match slot {
                    Some(s) => {
                        let src = if i.op_kind(1) == OpKind::Register { gpr(i.op_register(1)) } else { None };
                        kill_slots(st, s, size);
                        if size == 8 {
                            if let Some(r) = src.filter(|&r| r != RSP) {
                                let v = st.regs[r];
                                match v {
                                    V::Hold(q) if q as usize == r => ev.saves.push((va, r)),
                                    V::Hold(q) => ev.uses_entry |= 1 << q,
                                    _ => {}
                                }
                                st.slots.insert(s, v);
                            } else {
                                mark_uses(st, ev, e.reads);
                                st.slots.insert(s, V::Other);
                            }
                        } else {
                            mark_uses(st, ev, e.reads);
                        }
                    }
                    None => mark_uses(st, ev, e.reads),
                }
            } else if i.op_kind(0) == OpKind::Register {
                let d = i.op_register(0);
                let dn = gpr(d);
                if i.op_kind(1) == OpKind::Memory && slot.is_some() && d.size() == 8 && i.memory_size().size() == 8 {
                    let v = st.slots.get(&slot.unwrap()).copied().unwrap_or(V::Other);
                    let r = dn.unwrap();
                    match v {
                        V::Hold(q) if q as usize == r => ev.restores.push((va, r)),
                        V::Hold(q) => ev.uses_entry |= 1 << q,
                        _ => {}
                    }
                    if r == RSP {
                        st.rsp = None;
                    } else {
                        st.regs[r] = v;
                    }
                } else if i.op_kind(1) == OpKind::Register && d.size() == 8 && i.op_register(1) == Register::RSP && dn != Some(RSP) {
                    st.regs[dn.unwrap()] = match st.rsp {
                        Some(o) => V::Rel(o),
                        None => V::Other,
                    };
                } else if i.op_kind(1) == OpKind::Register && d == Register::RSP {
                    st.rsp = match gpr(i.op_register(1)).map(|r| st.regs[r]) {
                        Some(V::Rel(k)) => Some(k),
                        _ => None,
                    };
                } else {
                    mark_uses(st, ev, e.reads);
                    if let Some(n) = dn {
                        if n == RSP {
                            st.rsp = None;
                        } else {
                            write_regs(st, e.writes);
                            // decision 0261: `mov r32, imm` / `mov r64, imm` is a constant
                            if matches!(i.op_kind(1), OpKind::Immediate32 | OpKind::Immediate32to64 | OpKind::Immediate64) && d.size() >= 4 {
                                let v = i.immediate(1);
                                st.regs[n] = V::Const(if d.size() == 4 { (v as u32) as i64 } else { v as i64 });
                            }
                        }
                    }
                }
            } else {
                mark_uses(st, ev, e.reads);
                write_regs(st, e.writes);
            }
        }
        M::Lea => {
            let d = i.op_register(0);
            let dn = gpr(d);
            let val = if d.size() == 8 && !i.is_ip_rel_memory_operand() && i.memory_index() == Register::None {
                let b = i.memory_base();
                let disp = i.memory_displacement64() as i64;
                if b == Register::RSP {
                    st.rsp.map(|o| V::Rel(o + disp))
                } else {
                    gpr(b).and_then(|bn| if let V::Rel(k) = st.regs[bn] { Some(V::Rel(k + disp)) } else { None })
                }
            } else {
                None
            };
            let reads_rsp_base = i.memory_base() == Register::RSP;
            if !reads_rsp_base {
                mark_uses(st, ev, e.reads);
            }
            match (dn, val) {
                (Some(RSP), Some(V::Rel(k))) => st.rsp = Some(k),
                (Some(RSP), _) => st.rsp = None,
                (Some(n), Some(v)) => st.regs[n] = v,
                (Some(n), None) => st.regs[n] = V::Other,
                _ => {}
            }
        }
        M::Add | M::Sub if i.op_kind(0) == OpKind::Register && i.op_register(0) == Register::RSP => {
            let imm = if matches!(i.op_kind(1), OpKind::Immediate8to64 | OpKind::Immediate32to64 | OpKind::Immediate8 | OpKind::Immediate32) {
                Some(i.immediate(1) as i64)
            } else if i.op_kind(1) == OpKind::Register && i.op_register(1).size() == 8 {
                // decision 0261: `sub rsp, rax` after `mov eax, N; call __chkstk` (the probe preserves rax)
                gpr(i.op_register(1)).and_then(|r| if let V::Const(v) = st.regs[r] { Some(v) } else { None })
            } else {
                None
            };
            match (st.rsp, imm) {
                (Some(o), Some(v)) => st.rsp = Some(if m == M::Add { o + v } else { o - v }),
                _ => {
                    mark_uses(st, ev, e.reads);
                    st.rsp = None;
                }
            }
        }
        _ => {
            mark_uses(st, ev, e.reads);
            if fx.writes_rsp(i) {
                st.rsp = None;
            }
            // a store to the stack that is not a plain mov: forget the slots it may touch
            if i.op_kind(0) == OpKind::Memory {
                match slot_of(st, i) {
                    Some(s) => kill_slots(st, s, i.memory_size().size().max(1) as i64),
                    None => {}
                }
            }
            write_regs(st, e.writes);
        }
    }
}

/// Analysis of one function. `pres_of(g)`: the preserved set of a callee (None = unknown).
pub fn analyze(f: &Fun, cfg: &Cfg, fx: &mut Fx, pres_of: &dyn Fn(u64) -> Option<u16>, in_of: &dyn Fn(u64) -> u16, rsp_of: &dyn Fn(u64) -> Option<i64>, ev: Option<&mut Events>) -> PresResult {
    let n = f.blocks.len();
    let mut ins: Vec<Option<St>> = vec![None; n];
    ins[0] = Some(St::entry());
    let mut pres: u16 = ALL_REGS;
    let mut unbalanced = false;
    let mut exits: Vec<Option<i64>> = Vec::new();
    let mut dummy = Events::default();
    let ev = ev.unwrap_or(&mut dummy);
    // fixpoint over block entries (events are collected on the last pass only)
    let mut work: Vec<usize> = cfg.rpo.iter().rev().copied().collect();
    let mut on = vec![true; n];
    let mut scratch = Events::default();
    while let Some(b) = work.pop() {
        on[b] = false;
        let Some(mut st) = ins[b].clone() else { continue };
        for &va in &f.blocks[b].insns {
            step(&mut st, f.insn(va), va, fx, pres_of, in_of, rsp_of, &mut scratch);
        }
        for &s in &f.blocks[b].succs {
            let changed = match &mut ins[s] {
                None => {
                    ins[s] = Some(st.clone());
                    true
                }
                Some(cur) => cur.join(&st),
            };
            if changed && !on[s] {
                on[s] = true;
                work.push(s);
            }
        }
    }
    // final pass: results and events
    for (bi, b) in f.blocks.iter().enumerate() {
        let Some(mut st) = ins[bi].clone() else { continue };
        for &va in &b.insns {
            if matches!(b.term, Term::Ret) && Some(&va) == b.insns.last() {
                // `ret`: the state before it
                if st.rsp != Some(0) {
                    unbalanced = true;
                }
                // `ret imm16` pops the arguments as well: the callee leaves RSP imm16 bytes higher
                let pop = {
                    let ri = f.insn(va);
                    if ri.op_count() == 1 && ri.op_kind(0) == OpKind::Immediate16 { ri.immediate16() as i64 } else { 0 }
                };
                exits.push(st.rsp.map(|o| o + pop));
                for r in 0..16 {
                    if r != RSP && st.regs[r] != V::Hold(r as u8) {
                        pres &= !(1 << r);
                    }
                }
                break;
            }
            step(&mut st, f.insn(va), va, fx, pres_of, in_of, rsp_of, ev);
        }
        if matches!(&b.term, Term::TailImport(_) | Term::TailIndirect | Term::Tail(_)) && st.rsp != Some(0) {
            unbalanced = true;
        }
        match &b.term {
            // a tail jump into another function: that function's own delta is added to the offset it is entered with
            Term::Tail(g) => exits.push(st.rsp.map(|o| o + rsp_of(*g).unwrap_or(0))),
            // an import's stub or an indirect target is balanced
            Term::TailImport(_) | Term::TailIndirect => exits.push(if st.rsp == Some(0) { Some(0) } else { None }),
            _ => {}
        }
        if matches!(&b.term, Term::TailImport(_) | Term::TailIndirect) {
            let pg: u16 = (1 << 3) | (1 << 5) | (1 << 6) | (1 << 7) | (0xf << 12);
            for r in 0..16 {
                if r != RSP && !(st.regs[r] == V::Hold(r as u8) && pg & (1 << r) != 0) {
                    pres &= !(1 << r);
                }
            }
            for v in st.slots.values() {
                if let V::Hold(q) = v {
                    ev.tail_slots |= 1 << q;
                }
            }
            mark_uses(&st, ev, (1 << 1) | (1 << 2) | (1 << 8) | (1 << 9));
        }
        if let Term::Tail(g) = b.term {
            let pg = pres_of(g).unwrap_or(0);
            for r in 0..16 {
                if r != RSP && !(st.regs[r] == V::Hold(r as u8) && pg & (1 << r) != 0) {
                    pres &= !(1 << r);
                }
            }
            for v in st.slots.values() {
                if let V::Hold(q) = v {
                    ev.tail_slots |= 1 << q;
                }
            }
            // the tail callee's arguments are read
            mark_uses(&st, ev, in_of(g) & ALL_REGS);
        } else if let Some(&last) = b.insns.last() {
            // a block that ends in an ordinary instruction before a tail block is covered by the successor; a block whose terminator is a
            // jmp/jcc into a tail block carries its state through `ins` of the tail block
            let _ = last;
        }
    }
    let delta = match exits.first() {
        None => Some(0),
        Some(first) => {
            if first.is_some() && exits.iter().all(|e| e == first) {
                *first
            } else {
                None
            }
        }
    };
    PresResult { pres, unbalanced, delta, block_in: ins }
}

/// The elided registers and the instructions to skip, from the events of a function.
pub fn elision(ev: &Events, pres: u16) -> (u16, HashSet<u64>) {
    let mut regs: u16 = 0;
    for r in 0..16usize {
        if r == RSP || pres & (1 << r) == 0 {
            continue;
        }
        let has_save = ev.saves.iter().any(|s| s.1 == r);
        if !has_save {
            continue;
        }
        if ev.uses_entry & (1 << r) != 0 || ev.tail_slots & (1 << r) != 0 {
            continue;
        }
        regs |= 1 << r;
    }
    let mut skip = HashSet::new();
    for &(va, r) in ev.saves.iter().chain(ev.restores.iter()) {
        if regs & (1 << r) != 0 {
            skip.insert(va);
        }
    }
    (regs, skip)
}

pub type PresMap = HashMap<u64, u16>;
