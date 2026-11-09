//! Decision 0155 (provisional, `--dead-flags`): flags liveness over the lifted closure.
//!
//! Every flag-setting instruction of the generated C writes six bytes of `FL` (fle_flags.h), and most of them are overwritten by the next such instruction
//! before anything reads them. With the flags read and written of each instruction from iced-x86 (`rflags_read`, `rflags_written`), a backward
//! dataflow over the nodes of every function (the same graph `traverse` builds) tells which writes are dead. An instruction none of whose flag writes is
//! live gets the helpers that compute the result only (`fle_nf_*` of fle_flags_nf.h) instead of those that also write FL.
//!
//! What is assumed (the rest is conservative):
//! - a `ret`, a jump out of the function (a tail call to a function entry is handled exactly: the flags live at the callee's entry), an instruction that is not
//!   decoded and anything that leaves the closure: all six flags are live (a Delphi helper may return its answer in the flags);
//! - a direct call of a function entry: the flags live at the callee's entry (interprocedural, to a fixed point) plus the flags live after the call (the callee may
//!   leave them alone);
//! - any other call (an import stub, an indirect call): all six flags are live (no assumption about the Windows x64 ABI is made; assuming that such a callee does
//!   not read the caller's flags would make 2.6% more flag writers dead in freetype, tables/ in decision 0155, and was left out);
//! - a shift or rotate whose count is not a non-zero immediate, and a repeated string instruction, may leave the flags alone: they are not a kill.

use crate::{is_near, successors, traverse, Node};
use fe_decode::{JumpTable, Resolved, Text};
use iced_x86::{FlowControl, Instruction, Mnemonic, OpKind, RflagsBits};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

pub const ALL: u32 = RflagsBits::OF | RflagsBits::SF | RflagsBits::ZF | RflagsBits::AF | RflagsBits::CF | RflagsBits::PF;

/// (gen, kill): the status flags the instruction reads, and the ones it surely overwrites.
pub fn use_kill(i: &Instruction) -> (u32, u32) {
    let gen = i.rflags_read() & ALL;
    let mut kill = i.rflags_written() & ALL;
    use Mnemonic::*;
    match i.mnemonic() {
        Shl | Shr | Sar | Sal | Rol | Ror | Rcl | Rcr | Shld | Shrd => {
            let last = i.op_count() - 1;
            let nonzero_imm = matches!(i.op_kind(last), OpKind::Immediate8 | OpKind::Immediate8_2nd) && {
                let sixty_four = i.op0_kind() == OpKind::Register && i.op0_register().size() == 8
                    || i.op0_kind() == OpKind::Memory && i.memory_size().size() == 8;
                (i.immediate(last) & if sixty_four { 0x3f } else { 0x1f }) != 0
            };
            if !nonzero_imm {
                kill = 0;
            }
        }
        _ => {}
    }
    if i.has_rep_prefix() || i.has_repne_prefix() {
        kill = 0;
    }
    (gen, kill)
}

struct FNode {
    va: u64,
    gen: u32,
    kill: u32,
    /// successors inside the function (indices)
    succ: Vec<usize>,
    /// flags live on leaving the function from here (ret, tail call, unknown), or 0
    exit: u32,
    /// the entry called here (a direct call of a function entry)
    callee: Option<u64>,
    /// the call is to something that is not a function entry of the closure
    unknown_call: bool,
    /// tail call target (jump to another entry)
    tail: Vec<u64>,
    wmask: u32,
}

pub struct Liveness {
    /// per function entry: the instructions whose flag writes are all dead
    pub dead: BTreeMap<u64, HashSet<u64>>,
    pub writers: usize,
    pub fully_dead: usize,
    pub partly_dead: usize,
    pub rounds: usize,
}

fn build(f: u64, text: &Text<'_>, entries: &BTreeSet<u64>, resolved: &BTreeMap<u64, Resolved>, tabs: &BTreeMap<u64, JumpTable>) -> Vec<FNode> {
    let nodes = traverse(f, text, entries, resolved, tabs);
    let idx: HashMap<u64, usize> = nodes.keys().enumerate().map(|(k, &va)| (va, k)).collect();
    let mut v = Vec::with_capacity(nodes.len());
    for (&va, node) in &nodes {
        let mut n = FNode { va, gen: 0, kill: 0, succ: Vec::new(), exit: 0, callee: None, unknown_call: false, tail: Vec::new(), wmask: 0 };
        match node {
            Node::Bad => n.exit = ALL,
            Node::Insn(i) => {
                let (g, k) = use_kill(i);
                n.gen = g;
                n.kill = k;
                n.wmask = i.rflags_written() & ALL;
                let mut ss = successors(i, va, resolved, tabs);
                match i.flow_control() {
                    FlowControl::Call => {
                        if is_near(i) {
                            let t = i.near_branch_target();
                            if entries.contains(&t) {
                                n.callee = Some(t);
                            } else {
                                n.unknown_call = true;
                            }
                        } else {
                            n.unknown_call = true;
                        }
                    }
                    FlowControl::IndirectCall => n.unknown_call = true,
                    FlowControl::Return | FlowControl::Interrupt | FlowControl::Exception => n.exit = ALL,
                    FlowControl::IndirectBranch => {
                        let any = resolved.contains_key(&va) || tabs.contains_key(&va);
                        if !any {
                            n.exit = ALL;
                        }
                    }
                    FlowControl::UnconditionalBranch | FlowControl::ConditionalBranch => {
                        if !is_near(i) {
                            n.exit = ALL;
                        }
                    }
                    _ => {}
                }
                ss.sort_unstable();
                ss.dedup();
                for s in ss {
                    if let Some(&k) = idx.get(&s) {
                        n.succ.push(k);
                    } else if s != f && entries.contains(&s) {
                        n.tail.push(s);
                    } else {
                        n.exit = ALL; // outside the traversal (not decoded, not an entry)
                    }
                }
            }
        }
        v.push(n);
    }
    v
}

/// live-in of every node of one function under the current entry flags `fin`; returns (live_in, live_out).
fn solve(ns: &[FNode], fin: &HashMap<u64, u32>) -> (Vec<u32>, Vec<u32>) {
    let n = ns.len();
    let mut li = vec![0u32; n];
    let mut lo = vec![0u32; n];
    let mut changed = true;
    while changed {
        changed = false;
        for k in (0..n).rev() {
            let nd = &ns[k];
            let mut out = nd.exit;
            for &s in &nd.succ {
                out |= li[s];
            }
            for t in &nd.tail {
                out |= fin.get(t).copied().unwrap_or(ALL);
            }
            let mut inn = nd.gen | (out & !nd.kill);
            if let Some(c) = nd.callee {
                inn |= fin.get(&c).copied().unwrap_or(ALL);
            } else if nd.unknown_call {
                inn |= ALL;
            }
            if out != lo[k] || inn != li[k] {
                lo[k] = out;
                li[k] = inn;
                changed = true;
            }
        }
    }
    (li, lo)
}

pub fn analyse(
    entries: &BTreeSet<u64>,
    skip: &BTreeSet<u64>,
    text: &Text<'_>,
    resolved: &BTreeMap<u64, Resolved>,
    tabs: &BTreeMap<u64, JumpTable>,
) -> Result<Liveness, String> {
    let mut funcs: Vec<(u64, Vec<FNode>)> = entries.iter().filter(|f| !skip.contains(f)).map(|&f| (f, build(f, text, entries, resolved, tabs))).collect();
    let mut fin: HashMap<u64, u32> = entries.iter().map(|&f| (f, 0)).collect();
    let mut rounds = 0;
    loop {
        rounds += 1;
        let mut changed = false;
        for (f, ns) in &funcs {
            if ns.is_empty() {
                continue;
            }
            let (li, _) = solve(ns, &fin);
            // the entry node is the one with va == f
            let k = ns.iter().position(|n| n.va == *f).unwrap_or(0);
            let v = li[k];
            if fin[f] != v {
                fin.insert(*f, v);
                changed = true;
            }
        }
        if !changed {
            break;
        }
        if rounds > 10_000 {
            return Err("dead-flags: the flags live at the function entries did not reach a fixed point".into());
        }
    }
    let mut res = Liveness { dead: BTreeMap::new(), writers: 0, fully_dead: 0, partly_dead: 0, rounds };
    for (f, ns) in funcs.drain(..) {
        let (_, lo) = solve(&ns, &fin);
        let mut set = HashSet::new();
        for (k, nd) in ns.iter().enumerate() {
            if nd.wmask == 0 {
                continue;
            }
            res.writers += 1;
            // the flags the instruction may write against the flags live after it
            if (nd.wmask & lo[k]) == 0 {
                res.fully_dead += 1;
                set.insert(nd.va);
            } else if (nd.wmask & !lo[k]) != 0 {
                res.partly_dead += 1;
            }
        }
        if !set.is_empty() {
            res.dead.insert(f, set);
        }
    }
    Ok(res)
}

/// The helpers that compute the result and leave `FL` alone (runtime/fle_flags_nf.h), for an instruction whose flag writes are all dead.
/// Returns None for a body that has a flag writer other than these (it is then emitted as before).
pub fn nf_subst(body: &str) -> Option<String> {
    const SUBST: [(&str, &str); 10] = [
        ("fle_op_add(", "fle_nf_add("),
        ("fle_op_sub(", "fle_nf_sub("),
        ("fle_op_logic(", "fle_nf_logic("),
        ("fle_op_inc(", "fle_nf_inc("),
        ("fle_op_dec(", "fle_nf_dec("),
        ("fle_op_neg(", "fle_nf_neg("),
        ("fle_op_shl(", "fle_nf_shl("),
        ("fle_op_shr(", "fle_nf_shr("),
        ("fle_op_sar(", "fle_nf_sar("),
        ("fle_op_imul(", "fle_nf_imul("),
    ];
    let mut s = body.to_string();
    let mut any = false;
    for (a, b) in SUBST {
        if s.contains(a) {
            s = s.replace(a, b);
            any = true;
        }
    }
    if !any {
        return None;
    }
    // any other writer of FL left in the body: leave the whole instruction alone
    let mut t = s.replace("fle_rd(FL.cf)", "");
    for w in ["FL.", "fle_op_", "fle_bt_cf", "fle_imul1", "fle_idiv", "fle_div", "fle_set_comi", "fle_szp", "fle_undef_all", "fle_mx_", "fle_x87", "fle_mxcsr", "fle_cc("] {
        if t.contains(w) {
            return None;
        }
    }
    t.clear();
    Some(s)
}
