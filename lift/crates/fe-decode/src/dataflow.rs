//! Forward constant evaluation for indirect `call` / `jmp` targets that the
//! preceding-instruction rule in the parent module cannot see.
//!
//! Called only when that rule returns `Unresolved`. The scope is the straight-line
//! run from the block start to the branch. Each general register holds one value:
//! - `Imm(c)`: a constant (`lea` of a rip-relative address, `mov imm`, or folded
//!   arithmetic on constants).
//! - `Mem(c)`: the 8-byte file content read from a constant or table address.
//!   Only loads produce it; arithmetic on it does not.
//! - `Imp(slot)`: the content of an IAT slot (the callee has no code in the image).
//! - `Entry(r)`: the untouched value of register `r` at block start.
//! - `Unk { arg }`: anything else. `arg` is true when it depends on an argument
//!   register (`rcx`, `rdx`, `r8`, `r9`) at block start.
//!
//! File-content reads are trusted only while the run is "clean": no call and no
//! store to non-stack memory has happened since block start. After that, loads
//! give `Unk`.
//!
//! Rejected (left unresolved): a branch that is itself a direct branch/call
//! target, an instruction inside the run that is one (another path could enter
//! there with other values), and any control flow other than fall-through or
//! call inside the run.
//!
//! Assumptions (not verified on the real DLL): callee-saved registers survive
//! calls; a block entered only by a resolved indirect jump (not in `leaders`)
//! does not start inside another run; the linear sweep in `leaders` finds every
//! direct branch target.

use super::{How, Resolved};
use crate::{Decoded, Text};
use fe_pe::Pe;
use iced_x86::{Decoder, DecoderOptions, FlowControl, Instruction, InstructionInfoFactory, Mnemonic, OpAccess, OpKind, Register};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

const MAX_RUN: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// Target is a constant computed in the block (`lea`, `mov imm`, arithmetic).
    Arith,
    /// Target is read from file content: `[rip+X]`, or a load through a known address.
    Mem,
    /// Target depends on an argument register at block start (caller-dependent).
    Arg,
    /// Dataflow stopped at something not modelled, or the value is not known.
    Other,
    /// The branch is the block start, so nothing precedes it in the block.
    Entry,
}

impl Class {
    pub fn as_str(self) -> &'static str {
        match self {
            Class::Arith => "arith",
            Class::Mem => "mem",
            Class::Arg => "arg",
            Class::Other => "other",
            Class::Entry => "entry",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verdict {
    pub resolved: Resolved,
    pub class: Class,
    /// Why it stopped or how it resolved, for measurement.
    pub why: String,
}

/// One call of `resolve`, kept for measurement. Drained by `take_log`.
#[derive(Debug, Clone)]
pub struct Record {
    pub block: u64,
    pub va: u64,
    pub is_call: bool,
    pub verdict: Verdict,
}

thread_local! {
    static LOG: RefCell<Vec<Record>> = const { RefCell::new(Vec::new()) };
}

/// Returns and clears the records made on this thread since the last call.
/// `resolve` appends one record per call, so callers that do not drain it keep
/// growing memory.
pub fn take_log() -> Vec<Record> {
    LOG.with(|l| std::mem::take(&mut *l.borrow_mut()))
}

/// Direct branch and call targets, and the fall-through after a conditional
/// branch, found by a linear sweep of `.text` (each executable range on its own, in ascending VA order).
pub fn leaders(text: &Text) -> BTreeSet<u64> {
    let mut out = BTreeSet::new();
    for r in text.ranges() {
        let mut d = Decoder::with_ip(64, r.bytes, r.va, DecoderOptions::NONE);
        while d.can_decode() {
            let i = d.decode();
            if i.is_invalid() {
                continue;
            }
            match i.flow_control() {
                FlowControl::UnconditionalBranch | FlowControl::Call
                    if matches!(i.op_kind(0), OpKind::NearBranch16 | OpKind::NearBranch32 | OpKind::NearBranch64) =>
                {
                    out.insert(i.near_branch_target());
                }
                FlowControl::ConditionalBranch => {
                    out.insert(i.near_branch_target());
                    out.insert(i.next_ip());
                }
                _ => {}
            }
        }
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum V {
    Imm(u64),
    Mem(u64),
    Imp(u64),
    Entry(usize),
    Unk { arg: bool },
}

type Env = [V; 16];

/// Index of a 64-bit general register (0..16), or `None` for anything else.
fn gidx(r: Register) -> Option<usize> {
    use Register::*;
    match r.full_register() {
        RAX => Some(0),
        RCX => Some(1),
        RDX => Some(2),
        RBX => Some(3),
        RSP => Some(4),
        RBP => Some(5),
        RSI => Some(6),
        RDI => Some(7),
        R8 => Some(8),
        R9 => Some(9),
        R10 => Some(10),
        R11 => Some(11),
        R12 => Some(12),
        R13 => Some(13),
        R14 => Some(14),
        R15 => Some(15),
        _ => Option::None,
    }
}

fn is_arg(idx: usize) -> bool {
    matches!(idx, 1 | 2 | 8 | 9)
}

fn caller_saved(idx: usize) -> bool {
    matches!(idx, 0 | 1 | 2 | 8 | 9 | 10 | 11)
}

fn arg_of(v: V) -> bool {
    match v {
        V::Entry(i) => is_arg(i),
        V::Unk { arg } => arg,
        _ => false,
    }
}

fn entry_env() -> Env {
    let mut e = [V::Unk { arg: false }; 16];
    for (i, slot) in e.iter_mut().enumerate() {
        *slot = V::Entry(i);
    }
    e
}

fn read8(pe: &Pe, va: u64) -> Option<u64> {
    let rva = pe.va_to_rva(va).ok()?;
    let b = pe.slice_at_rva(rva, 8).ok()?;
    let mut a = [0u8; 8];
    a.copy_from_slice(b);
    Some(u64::from_le_bytes(a))
}

fn writes(a: OpAccess) -> bool {
    matches!(a, OpAccess::Write | OpAccess::CondWrite | OpAccess::ReadWrite | OpAccess::ReadCondWrite)
}

fn reads(a: OpAccess) -> bool {
    matches!(a, OpAccess::Read | OpAccess::CondRead | OpAccess::ReadWrite | OpAccess::ReadCondWrite)
}

/// Effective address of a memory operand.
enum Addr {
    /// `addr` is known. `from_mem` is true when a register holding file content
    /// (not a constant) took part.
    Known { addr: u64, from_mem: bool },
    /// Depends on an unknown register. `arg` is the argument flag.
    Unknown { arg: bool },
}

fn address(i: &Instruction, env: &Env) -> Addr {
    if i.is_ip_rel_memory_operand() {
        return Addr::Known { addr: i.ip_rel_memory_address(), from_mem: false };
    }
    let mut addr = i.memory_displacement64();
    let mut from_mem = false;
    for (reg, scale) in [(i.memory_base(), 1u64), (i.memory_index(), i.memory_index_scale() as u64)] {
        if reg == Register::None {
            continue;
        }
        let Some(g) = gidx(reg) else { return Addr::Unknown { arg: false } };
        match env[g] {
            V::Imm(c) => addr = addr.wrapping_add(c.wrapping_mul(scale)),
            V::Mem(c) => {
                addr = addr.wrapping_add(c.wrapping_mul(scale));
                from_mem = true;
            }
            other => return Addr::Unknown { arg: arg_of(other) },
        }
    }
    Addr::Known { addr, from_mem }
}

/// The value a memory operand (size 8) holds: a file-content read, an import slot,
/// or unknown.
fn load(pe: &Pe, imports: &BTreeMap<u64, String>, i: &Instruction, env: &Env, clean: bool) -> V {
    if i.memory_size().size() != 8 {
        return V::Unk { arg: false };
    }
    match address(i, env) {
        Addr::Unknown { arg } => V::Unk { arg },
        Addr::Known { addr, .. } => {
            if !clean {
                return V::Unk { arg: false };
            }
            if imports.contains_key(&addr) {
                return V::Imp(addr);
            }
            match read8(pe, addr) {
                Some(c) => V::Mem(c),
                None => V::Unk { arg: false },
            }
        }
    }
}

/// Value of source operand `n`.
fn src(pe: &Pe, imports: &BTreeMap<u64, String>, i: &Instruction, env: &Env, clean: bool, n: u32) -> V {
    match i.op_kind(n) {
        OpKind::Register => {
            let r = i.op_register(n);
            match gidx(r) {
                Some(g) if r.size() == 8 => env[g],
                _ => V::Unk { arg: false },
            }
        }
        OpKind::Memory => load(pe, imports, i, env, clean),
        OpKind::Immediate8to64 | OpKind::Immediate32to64 | OpKind::Immediate64 => V::Imm(i.immediate(n)),
        _ => V::Unk { arg: false },
    }
}

/// Folds `op` only for two constants. Anything else is unknown; a file-content
/// value never changes form under arithmetic.
fn fold(op: fn(u64, u64) -> u64, a: V, b: V) -> V {
    match (a, b) {
        (V::Imm(x), V::Imm(y)) => V::Imm(op(x, y)),
        _ => V::Unk { arg: arg_of(a) || arg_of(b) },
    }
}

/// True when the instruction writes a memory operand that is not stack-based.
fn stores_outside_stack(i: &Instruction, info: &mut InstructionInfoFactory) -> bool {
    info.info(i).used_memory().iter().any(|m| {
        writes(m.access()) && !matches!(m.base(), Register::RSP | Register::ESP)
    })
}

/// Evaluates `run` in order from `env` and returns the register values after it.
/// `clean` must be false when the whole run contains a call or a non-stack store.
/// `Err` when the run has control flow other than fall-through or call.
fn eval_run(pe: &Pe, imports: &BTreeMap<u64, String>, run: &[(u64, Instruction)], mut env: Env, clean: bool) -> Result<Env, String> {
    let mut info = InstructionInfoFactory::new();
    for (_, p) in run.iter() {
        match p.flow_control() {
            FlowControl::Next => {}
            FlowControl::Call | FlowControl::IndirectCall => {
                for (g, slot) in env.iter_mut().enumerate() {
                    if caller_saved(g) {
                        *slot = V::Unk { arg: false };
                    }
                }
                continue;
            }
            _ => return Err("control_flow_in_run".into()),
        }
        if exec(pe, imports, p, &mut env, clean).is_none() {
            // Generic rule: every GPR written by `p` becomes unknown. It keeps the
            // argument flag of the GPRs `p` reads.
            let used = info.info(p).used_registers().to_vec();
            let arg = used
                .iter()
                .filter(|u| reads(u.access()))
                .filter_map(|u| gidx(u.register()))
                .any(|g| arg_of(env[g]));
            for u in &used {
                if writes(u.access()) {
                    if let Some(g) = gidx(u.register()) {
                        env[g] = V::Unk { arg };
                    }
                }
            }
        }
    }
    Ok(env)
}

/// Explicitly modelled register writes. `None` means "not modelled": the caller
/// applies the generic rule.
fn exec(pe: &Pe, imports: &BTreeMap<u64, String>, p: &Instruction, env: &mut Env, clean: bool) -> Option<()> {
    if p.op_count() == 0 || p.op_kind(0) != OpKind::Register {
        return None;
    }
    let d = p.op_register(0);
    let g = gidx(d)?;
    if d.size() != 8 {
        return None;
    }
    let v = match p.mnemonic() {
        Mnemonic::Lea => {
            if p.op_kind(1) != OpKind::Memory {
                return None;
            }
            match address(p, env) {
                Addr::Known { addr, from_mem: false } => V::Imm(addr),
                Addr::Known { from_mem: true, .. } => V::Unk { arg: false },
                Addr::Unknown { arg } => V::Unk { arg },
            }
        }
        Mnemonic::Mov => src(pe, imports, p, env, clean, 1),
        Mnemonic::Add => fold(u64::wrapping_add, env[g], src(pe, imports, p, env, clean, 1)),
        Mnemonic::Sub => {
            if p.op_kind(1) == OpKind::Register && p.op_register(1).full_register() == d.full_register() {
                V::Imm(0)
            } else {
                fold(u64::wrapping_sub, env[g], src(pe, imports, p, env, clean, 1))
            }
        }
        Mnemonic::Xor => {
            if p.op_kind(1) == OpKind::Register && p.op_register(1).full_register() == d.full_register() {
                V::Imm(0)
            } else {
                fold(|a, b| a ^ b, env[g], src(pe, imports, p, env, clean, 1))
            }
        }
        Mnemonic::And => fold(|a, b| a & b, env[g], src(pe, imports, p, env, clean, 1)),
        Mnemonic::Or => fold(|a, b| a | b, env[g], src(pe, imports, p, env, clean, 1)),
        Mnemonic::Shl if p.op_kind(1) == OpKind::Immediate8 => {
            fold(|a, b| a.wrapping_shl((b & 63) as u32), env[g], V::Imm(p.immediate(1)))
        }
        Mnemonic::Imul if p.op_count() == 3 => {
            let k = p.immediate(2);
            fold(u64::wrapping_mul, src(pe, imports, p, env, clean, 1), V::Imm(k))
        }
        _ => return None,
    };
    env[g] = v;
    Some(())
}

fn verdict(why: &str, class: Class) -> Verdict {
    Verdict { resolved: Resolved::Unresolved, class, why: why.into() }
}

fn class_of(v: V) -> Class {
    match v {
        V::Imm(_) => Class::Arith,
        V::Mem(_) | V::Imp(_) => Class::Mem,
        V::Entry(g) if is_arg(g) => Class::Arg,
        V::Unk { arg: true } => Class::Arg,
        _ => Class::Other,
    }
}

/// The branch target is the 8-byte content at `slot`, read at the branch.
fn slot_target(pe: &Pe, text: &Text, imports: &BTreeMap<u64, String>, slot: u64, clean: bool) -> Verdict {
    if !clean {
        return verdict("slot_after_store_or_call", Class::Mem);
    }
    if let Some(name) = imports.get(&slot) {
        return Verdict { resolved: Resolved::Import { name: name.clone() }, class: Class::Mem, why: "import".into() };
    }
    match read8(pe, slot) {
        Some(t) if text.contains(t) => {
            Verdict { resolved: Resolved::Code { target: t, how: How::RegTable }, class: Class::Mem, why: "table".into() }
        }
        Some(_) => verdict("table_not_code", Class::Mem),
        None => verdict("table_unreadable", Class::Mem),
    }
}

/// Turns the branch operand's value into a verdict.
///
/// Register form (`call reg`): the target is the register's value.
/// Memory form (`call [reg+disp]`): the target is the content at `value + disp`.
fn judge(pe: &Pe, text: &Text, imports: &BTreeMap<u64, String>, env: &Env, i: &Instruction, clean: bool) -> Verdict {
    if i.op_kind(0) == OpKind::Memory {
        if i.is_ip_rel_memory_operand() {
            return slot_target(pe, text, imports, i.ip_rel_memory_address(), clean);
        }
        if i.memory_index() != Register::None {
            return verdict("mem_form_index", Class::Other);
        }
        let Some(g) = gidx(i.memory_base()) else { return verdict("mem_form_base", Class::Other) };
        let disp = i.memory_displacement64();
        return match env[g] {
            V::Imm(c) | V::Mem(c) => slot_target(pe, text, imports, c.wrapping_add(disp), clean),
            other => verdict("mem_form_unknown_base", class_of(other)),
        };
    }
    if i.op_kind(0) != OpKind::Register {
        return verdict("not_reg_or_mem", Class::Other);
    }
    let Some(g) = gidx(i.op_register(0)) else { return verdict("not_gpr", Class::Other) };
    match env[g] {
        V::Imm(c) if text.contains(c) => {
            Verdict { resolved: Resolved::Code { target: c, how: How::RegConst }, class: Class::Arith, why: "const".into() }
        }
        V::Imm(_) => verdict("const_not_code", Class::Arith),
        V::Mem(c) if text.contains(c) => {
            Verdict { resolved: Resolved::Code { target: c, how: How::RegTable }, class: Class::Mem, why: "file_content".into() }
        }
        V::Mem(_) => verdict("content_not_code", Class::Mem),
        V::Imp(slot) => match imports.get(&slot) {
            Some(name) => Verdict { resolved: Resolved::Import { name: name.clone() }, class: Class::Mem, why: "import".into() },
            None => verdict("import_unnamed", Class::Mem),
        },
        other => verdict(if arg_of(other) { "arg_dependent" } else { "unknown" }, class_of(other)),
    }
}

/// Decoded instructions from `block` up to `va`, with their addresses. `None` if
/// `va` is not reached by decoding within `MAX_RUN` instructions.
fn run(text: &Text, block: u64, va: u64) -> Option<Vec<(u64, Instruction)>> {
    let mut out = Vec::new();
    let mut p = block;
    while p != va {
        if out.len() >= MAX_RUN {
            return None;
        }
        match text.decode(p) {
            Decoded::Ok(i) => {
                let len = i.len() as u64;
                out.push((p, i));
                p += len;
            }
            _ => return None,
        }
    }
    Some(out)
}

fn decide(pe: &Pe, text: &Text, imports: &BTreeMap<u64, String>, leaders: &BTreeSet<u64>, block: u64, va: u64, i: &Instruction) -> Verdict {
    if leaders.contains(&va) {
        return verdict("branch_is_leader", Class::Other);
    }
    if block == va {
        let base = match i.op_kind(0) {
            OpKind::Register => Some(i.op_register(0)),
            OpKind::Memory if !i.is_ip_rel_memory_operand() => Some(i.memory_base()),
            _ => None,
        };
        let arg = base.and_then(gidx).is_some_and(is_arg);
        return verdict("block_start", if arg { Class::Arg } else { Class::Entry });
    }
    let Some(run) = run(text, block, va) else { return verdict("run_undecodable", Class::Other) };
    // Start after the last leader inside the run: every path into the branch
    // passes through that leader, so its values are the only ones that matter.
    // Registers are unknown there (`Entry` of the start is replaced by `Unk`).
    let cut = run.iter().rposition(|(a, _)| leaders.contains(a));
    let (env, scope) = match cut {
        None => (entry_env(), &run[..]),
        Some(k) => (unknown_env(), &run[k..]),
    };
    let mut info = InstructionInfoFactory::new();
    let clean = scope.iter().all(|(_, p)| {
        !matches!(p.flow_control(), FlowControl::Call | FlowControl::IndirectCall) && !stores_outside_stack(p, &mut info)
    });
    match eval_run(pe, imports, scope, env, clean) {
        Ok(env) => judge(pe, text, imports, &env, i, clean),
        Err(why) => verdict(&why, Class::Other),
    }
}

fn unknown_env() -> Env {
    [V::Unk { arg: false }; 16]
}

/// Resolves the indirect branch `i` at `va` in the block that starts at `block`.
/// `leaders` must come from `leaders(text)`.
pub fn resolve(
    pe: &Pe,
    text: &Text,
    imports: &BTreeMap<u64, String>,
    leaders: &BTreeSet<u64>,
    block: u64,
    va: u64,
    i: &Instruction,
) -> Verdict {
    let v = decide(pe, text, imports, leaders, block, va, i);
    LOG.with(|l| {
        l.borrow_mut().push(Record { block, va, is_call: i.flow_control() == FlowControl::IndirectCall, verdict: v.clone() })
    });
    v
}
