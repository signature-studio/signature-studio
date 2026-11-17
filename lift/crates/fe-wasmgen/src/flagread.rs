//! Decision 0270b (mt3w): the callees whose status flags are read by a caller that stays on the C path.
//!
//! The C path keeps the flags in the global `FL` and the lifted C of a caller reads it after the call. A direct-wasm function has no `FL`, and its wrapper `f_<va>` (the entry from the C path) carries registers
//! only and writes no flag. A function that a C-path caller calls and whose flags that caller reads must therefore stay on the C path as well, or the caller reads the `FL` of before the call.
//! A caller that is compiled direct reads the flags of a callee from the callee's static summary (`flagout.rs`, decision 0270) or is not compiled, so only the sites in functions that stay on the C path count.
//!
//! `read_after_call` follows the instructions after every direct `call` of a C-path function (fall-through, direct `jmp`, both ways of a `jcc`/`jrcxz`) until one reads a status flag (the callee's flags are read),
//! one writes a flag, calls or jumps through an unknown target (the flags are not the callee's any more), or the function returns / jumps to another function without a write (the callee's flags are the function's own flags: the
//! callee of that call is a callee whose flags are read as well). A function that is reached by `jmp` from a function whose flags are read is one too.

use fe_decode::Analysis;
use iced_x86::{FlowControl, Mnemonic, OpKind};
use std::collections::{BTreeSet, HashSet};

/// iced's `RflagsBits` of the status flags: OF 0x1, SF 0x2, ZF 0x4, AF 0x8, CF 0x10, PF 0x20 (DF 0x40 is read by the string instructions and is not one of them)
const STATUS: u32 = 0x3f;

#[derive(Clone, Copy, PartialEq, Eq)]
enum After {
    Read,
    Passes,
    Neither,
}

/// What happens to the flags after the instruction at `from` (the one after a call).
fn after(a: &Analysis, f: u64, from: u64) -> After {
    let (mut seen, mut work, mut passes, mut steps) = (HashSet::new(), vec![from], false, 0usize);
    while let Some(mut p) = work.pop() {
        loop {
            steps += 1;
            if steps > 4000 || !seen.insert(p) {
                break;
            }
            let Some(n) = a.insns.get(&p) else { break };
            if n.rflags_read() & STATUS != 0 {
                return After::Read;
            }
            if (n.rflags_written() | n.rflags_cleared() | n.rflags_set() | n.rflags_undefined() | n.rflags_modified()) & STATUS != 0 {
                break;
            }
            let near = matches!(n.op0_kind(), OpKind::NearBranch64);
            match n.flow_control() {
                FlowControl::Next => p += n.len() as u64,
                FlowControl::UnconditionalBranch if near && n.mnemonic() == Mnemonic::Jmp => {
                    let t = n.near_branch_target();
                    if t != f && a.functions.contains_key(&t) {
                        // a jump to the entry of another function: its flags are the flags of this one
                        passes = true;
                        break;
                    }
                    p = t;
                }
                FlowControl::ConditionalBranch if near => {
                    work.push(n.near_branch_target());
                    p += n.len() as u64;
                }
                FlowControl::Return => {
                    passes = true;
                    break;
                }
                _ => break,
            }
        }
    }
    if passes {
        After::Passes
    } else {
        After::Neither
    }
}

/// The functions whose flags are read after a call by a function of `c_path`, directly or through the chain of calls and jumps that hand their flags on.
pub fn read_after_call(a: &Analysis, c_path: &HashSet<u64>) -> BTreeSet<u64> {
    let mut out: BTreeSet<u64> = BTreeSet::new();
    let mut work: Vec<u64> = Vec::new();
    let sites = |f: u64| -> Vec<(u64, u64)> {
        let mut v = Vec::new();
        for &va in a.functions.get(&f).into_iter().flatten() {
            let n = &a.insns[&va];
            if n.flow_control() == FlowControl::Call && matches!(n.op0_kind(), OpKind::NearBranch64) && a.functions.contains_key(&n.near_branch_target()) {
                v.push((va + n.len() as u64, n.near_branch_target()));
            }
        }
        v
    };
    // the sites in the functions of the C path
    for &f in a.functions.keys().filter(|f| c_path.contains(f)) {
        for (next, t) in sites(f) {
            if after(a, f, next) == After::Read && out.insert(t) {
                work.push(t);
            }
        }
    }
    // the flags of a callee are the flags of the function that returns them: its own callees and the functions it jumps to
    while let Some(x) = work.pop() {
        for (next, t) in sites(x) {
            if after(a, x, next) == After::Passes && out.insert(t) {
                work.push(t);
            }
        }
        for &va in a.functions.get(&x).into_iter().flatten() {
            let n = &a.insns[&va];
            let tgt = match n.flow_control() {
                FlowControl::UnconditionalBranch | FlowControl::ConditionalBranch if matches!(n.op0_kind(), OpKind::NearBranch64) => Some(n.near_branch_target()),
                FlowControl::IndirectBranch => match a.resolved.get(&va) {
                    Some(fe_decode::Resolved::Code { target, .. }) => Some(*target),
                    _ => None,
                },
                _ => None,
            };
            if let Some(t) = tgt {
                if t != x && a.functions.contains_key(&t) && out.insert(t) {
                    work.push(t);
                }
            }
        }
    }
    out
}
