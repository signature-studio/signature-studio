//! XMM registers across function boundaries in the hybrid (decision 0164): the C path keeps the XMM file in the array `X[]`; the compiled
//! functions keep it in v128 locals and exchange it with `X[]` at the boundary. A function loads the registers that are live into it from `X[]`
//! at entry, stores the registers it writes before every call that may read them and at every return, and reloads after a call the registers the
//! callee may have written and the function still needs. The sets come from a whole-program summary (live-in `xin`, may-write `xw`) of every
//! function, compiled or not.

use crate::cfg::Cfg;
use crate::fx::Eff;
use crate::ir::{Fun, Term};
use iced_x86::Mnemonic;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;

/// The exchange through X[] is on (hybrid with `--hybrid-x`); scalar operations then count as reading the old upper lanes of their destination.
pub static XMEM: AtomicBool = AtomicBool::new(false);

pub const ALLX: u16 = 0xffff;
/// An indirect callee follows the Windows x64 convention: it reads xmm0..3 and returns xmm0 (the volatile xmm1..5 are dead after the call; xmm6..15 are
/// preserved). A violation of this assumption is not detected (the assumption is documented in decision 0164).
pub const IND_IN: u16 = 0x000f;
pub const IND_W: u16 = 0x0001;
/// An import called through the C runtime (HLE): reads xmm0..3, may write the volatile registers.
pub const IMP_IN: u16 = 0x000f;
pub const IMP_W: u16 = 0x003f;

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct XSum {
    /// XMM registers read before written (live into the function), also by its callees.
    pub xin: u16,
    /// XMM registers written on some path, also by its callees.
    pub xw: u16,
}

#[derive(Clone, Default, Debug)]
pub struct XPlan {
    /// Registers loaded from X[] at entry.
    pub entry: u16,
    /// Registers written by the function itself: stored at every return.
    pub own: u16,
    /// Call instruction VA -> (registers stored before the call, registers reloaded after it).
    pub call: HashMap<u64, (u16, u16)>,
}

pub fn own_of(fun: &Fun, effs: &HashMap<u64, Eff>) -> u16 {
    let mut m = 0u16;
    for b in &fun.blocks {
        for va in &b.insns {
            m |= effs[va].xw;
        }
    }
    m
}

pub struct Env<'a> {
    pub sums: &'a HashMap<u64, XSum>,
    pub imports: &'a HashMap<u64, String>,
}

fn call_info(env: &Env, fun: &Fun, va: u64) -> (u16, u16) {
    let i = fun.insn(va);
    if env.imports.contains_key(&va) {
        return (IMP_IN, IMP_W);
    }
    if crate::ir::is_near(i) {
        return match env.sums.get(&i.near_branch_target()) {
            Some(s) => (s.xin, s.xw),
            None => (ALLX, ALLX),
        };
    }
    (IND_IN, IND_W)
}

/// Backward dataflow over the function. Returns (registers live into the function, registers written by the function and its callees, registers read by the function or by its
/// callees before they are written: the summary `xin`). The two live sets differ for registers that the function only passes through to a callee (a trampoline `sub rsp; call g;
/// add rsp; ret` that never touches xmm0 while g reads it): the function does not hold them (no entry load, no store), but its callers must store them before calling it.
/// With `plan`, records the store and reload sets of every call.
pub fn df(fun: &Fun, cfg: &Cfg, effs: &HashMap<u64, Eff>, own: u16, env: &Env, mut plan: Option<&mut XPlan>) -> (u16, u16, u16) {
    let n = fun.blocks.len();
    let mut live_in = vec![0u16; n];
    let mut pass_in = vec![0u16; n];
    let mut xw = own;
    let mut pass = 0;
    loop {
        let mut changed = false;
        for &b in cfg.rpo.iter().rev() {
            let bl = &fun.blocks[b];
            let (mut live, mut lp) = match &bl.term {
                Term::Ret => (own, own),
                Term::TailIndirect => (own, own | IND_IN),
                Term::TailImport(_) => (own, own | IMP_IN),
                Term::Tail(g) => (own, own | env.sums.get(g).map_or(ALLX, |s| s.xin)),
                _ => (bl.succs.iter().fold(0u16, |a, &x| a | live_in[x]), bl.succs.iter().fold(0u16, |a, &x| a | pass_in[x])),
            };
            match &bl.term {
                Term::Tail(g) => xw |= env.sums.get(g).map_or(ALLX, |s| s.xw),
                Term::TailImport(_) => xw |= IMP_W,
                Term::TailIndirect => xw |= IND_W,
                _ => {}
            }
            for &va in bl.insns.iter().rev() {
                let i = fun.insn(va);
                if i.mnemonic() == Mnemonic::Call {
                    let (xi, cw) = call_info(env, fun, va);
                    xw |= cw;
                    let reload = cw & live;
                    let flush = (xi | reload) & own;
                    if let Some(p) = plan.as_deref_mut() {
                        p.call.insert(va, (flush, reload));
                    }
                    live |= flush;
                    lp |= flush | xi;
                } else {
                    let e = effs[&va];
                    live = (live & !e.xd) | e.xr;
                    lp = (lp & !e.xd) | e.xr;
                }
            }
            if live != live_in[b] || lp != pass_in[b] {
                live_in[b] = live;
                pass_in[b] = lp;
                changed = true;
            }
        }
        pass += 1;
        if !changed || pass > 200 {
            break;
        }
    }
    (live_in[0], xw, pass_in[0])
}

/// Whole-program summaries of every function with a flow graph; the others (opaque) read and write everything.
pub fn summaries(funs: &HashMap<u64, (&Fun, &Cfg, &HashMap<u64, Eff>)>, imports: &HashMap<u64, String>) -> HashMap<u64, XSum> {
    let mut sums: HashMap<u64, XSum> = HashMap::new();
    let owns: HashMap<u64, u16> = funs.iter().map(|(&k, &(f, _, e))| (k, own_of(f, e))).collect();
    for (&k, &o) in &owns {
        sums.insert(k, XSum { xin: 0, xw: o });
    }
    let keys: Vec<u64> = {
        let mut v: Vec<u64> = funs.keys().copied().collect();
        v.sort();
        v
    };
    loop {
        let mut changed = false;
        for &k in &keys {
            let (f, c, e) = funs[&k];
            let env = Env { sums: &sums, imports };
            let (_, xw, xin) = df(f, c, e, owns[&k], &env, None);
            let ns = XSum { xin: xin | sums[&k].xin, xw: xw | sums[&k].xw };
            if ns != sums[&k] {
                sums.insert(k, ns);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    sums
}

pub fn plan(fun: &Fun, cfg: &Cfg, effs: &HashMap<u64, Eff>, sums: &HashMap<u64, XSum>, imports: &HashMap<u64, String>) -> XPlan {
    let own = own_of(fun, effs);
    let env = Env { sums, imports };
    let mut p = XPlan { own, ..Default::default() };
    let (entry, _, _) = df(fun, cfg, effs, own, &env, Some(&mut p));
    p.entry = entry;
    p
}
