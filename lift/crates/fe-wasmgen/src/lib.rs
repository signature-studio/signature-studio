//! Decision 0157 (prototype): x86-64 to WebAssembly, directly, over the fe-decode closure.
//!
//! Pipeline: functions and instructions from fe-decode (unchanged) -> blocks (`ir`) -> dominators/loops (`cfg`) -> flag definitions and
//! readers (`flags`) -> a dry lowering that tells which functions are supported -> preserved registers and elidable saves (`pres`) ->
//! parameters/results of every function from interprocedural liveness -> wasm (`lower`, `wasm`).

pub mod cfg;
pub mod flagout;
pub mod flagread;
pub mod flags;
pub mod fx;
pub mod ir;
pub mod locshare;
pub mod lower;
pub mod pres;
pub mod rcgen;
pub mod wasm;
pub mod xabi;

use fe_decode::Text;
use fe_pe::Pe;
use fx::*;
use ir::Fun;
use lower::{Ctx, LowerStats, Opts, Sig};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use wasm::{Body, Code, FuncType, Module, Ty};

#[derive(Clone)]
pub struct Config {
    /// Adapters with the universal signature (16 registers in, 16 out) for these functions (None = none, Some(empty) = all closed ones). Test entry.
    pub export: Option<HashSet<u64>>,
    /// Compile only these functions and what they call (None = all). Test entry.
    pub roots: Option<HashSet<u64>>,
    /// Address of the thread environment block (GS-relative accesses become absolute); None = functions that use GS are not compiled.
    pub teb: Option<u32>,
    /// Emit the `name` section (f_<va>, imp_<name>, t_<va>). Used by the tests that read the module back.
    pub names: bool,
    /// Export every compiled function as f_<va> (keeps wasm-opt from removing them). Used by the tests that call the functions.
    pub export_funcs: bool,
    /// Test option: give an indirect-call adapter to every function that preserves the callee-saved registers, feeding 0 for the registers it reads
    /// beyond rcx/rdx/r8/r9 (normally such a function is not an indirect-call target and the call traps).
    pub adapt_all: bool,
    /// Census of unsupported instructions (counted per instruction in `Report::unsupported`). Test entry (the census tool).
    pub census: bool,
    /// Hybrid mode (decision 0157): the functions that this backend cannot lower are left to the C path, which keeps the registers in the
    /// array R[] at this address; calls into it go through R[] and the import `c.f_<va>`, and every compiled function is exported as `f_<va>` (void,
    /// registers in R[]) for the C path to call.
    pub hybrid_r: Option<u32>,
    /// Hybrid mode: compile only these functions (the others stay on the C path).
    pub hybrid_only: Option<HashSet<u64>>,
    /// Hybrid mode with the C runtime (decision 0164, OTT pilot): import name -> the symbol of the C function (`fle_imp_<name>`, or `__wrap_fle_imp_<name>` where the harness wraps it) for every import that exists. Imports of the compiled functions become
    /// stubs that call them through R[]; indirect calls call `fle_hyb_dispatch` of the C path (no adapters, no table); a function with another import is left to the C path.
    pub c_imports: Option<HashMap<String, String>>,
    /// The memory import (module, name); default env.memory. The C module of the pilot defines its memory: `c.memory`.
    pub mem_import: Option<(String, String)>,
    /// decision 0234 (`--tls-ctx OFFSET`; required by `hybrid_r` and requires it: the hybrid with the C runtime has one form; the imported memory is declared shared with a maximum of 32768 pages (2 GiB, the maximum of the C part's link), as the C part's is): the C path's R[], FL and X[] are the first 0x300 bytes of the C part's thread-local block `fle_ctx`, which is
    /// at this offset of its TLS segment (the link map): the register page is addressed as `global.get <the imported c.__tls_base>` + a memarg offset. R at `hybrid_r`, X at `hybrid_x` = `hybrid_r` + 0x200.
    pub tls_ctx: Option<u32>,
    /// Hybrid mode: address of the XMM file X[] of the C path; XMM registers cross function boundaries through it (see `xabi`).
    pub hybrid_x: Option<u32>,
    /// Hybrid mode: addresses of the C path's `fle_mxcsr` and of the scratch buffer `fle_hyb_buf` (3 x 16 bytes); the MXCSR window rule reads the C path's MXCSR and
    /// the slow path is the C path's own function (`c.fle_hyb_mx`).
    pub hybrid_mx: Option<(u32, u32)>,
    /// Hybrid (decision 0179): address of the C runtime's `fle_progress_count`. A resolved jmp that was demoted to the run-time lookup (the target is an opaque function that has to be reached through `fle_hyb_dispatch`) takes the
    /// count back by one before the dispatch, because the C path jumps to such a target without counting an indirect call of the guest (the count is the progress meter of the cooperative threads, 82-line identity).
    pub hybrid_progress: Option<u32>,
    /// Decision 0285 (`--hybrid-dispatch LIMIT LANDING HOOK`; needs `hybrid_progress` and `hybrid_x`): the addresses of the C runtime's `fle_progress_limit` (u64), `fle_landing_top` (a 32-bit pointer) and `fle_dispatch_hook` (a 32-bit function pointer).
    /// With them an indirect call of a direct function to a direct function that fits the uniform indirect signature (rsp, rcx, rdx, r8, r9, rax -> rax; see `indirect_fit_why`) goes through a table of adapters instead of `c.fle_hyb_dispatch`; a call that is
    /// not in the table, or made while a landing or the census hook is set, takes the path of `fle_hyb_dispatch` as before. The addresses are the data the C path's `fle_dispatch` reads; they are baked in like `hybrid_progress`.
    pub hybrid_dispatch: Option<(u32, u32, u32)>,
    /// Decision 0185: a function whose dry-run wasm body exceeds one of these limits (bytes of code, locals, bytes x locals) stays on the C path. Parameters (numbers), not switches: the cost limit bounds the memory of fe-wasmgen on the large hybrid lists (13 GB without it).
    pub max_fn_bytes: u64,
    pub max_fn_locals: u64,
    pub max_fn_cost: u64,
}

/// Decision 0185, 0243: the cost limit (bytes of code x locals) of the lists of the hybrid. The lists of the shipped hybrid were made with 30,000,000 and 50,000,000 is the limit of the 3,879-function list: all of its functions are below it. Decision 0264 (mt3z): 75,000,000. With the flag-definition locals shared (locshare.rs) the Liftoff memory of a large function no longer follows bytes x locals (Blend2D f_4776f60, 1.27 MB: 954 MB with 892 locals, 44 MB with 48; node, eager Liftoff compile), so the product is no longer the quantity the 50,000,000 was chosen against; the functions between 50 and 75 million after the sharing are f_4776f60 (60.4 million) and five dsp functions (68-71 million) that no run executes.
pub const MAX_FN_COST: u64 = 75_000_000;

impl Default for Config {
    fn default() -> Self {
        Config { export: None, roots: None, teb: None, names: false, export_funcs: false, adapt_all: false, census: false, hybrid_r: None, hybrid_only: None, c_imports: None, mem_import: None, tls_ctx: None, hybrid_x: None, hybrid_mx: None, hybrid_progress: None, hybrid_dispatch: None, max_fn_bytes: u64::MAX, max_fn_locals: u64::MAX, max_fn_cost: MAX_FN_COST }
    }
}

#[derive(Default, Debug)]
pub struct FnRow {
    pub va: u64,
    pub insns: usize,
    pub native_bytes: usize,
    pub status: String,
    pub wasm_bytes: usize,
    pub sig_in: u16,
    pub sig_out: u16,
    pub elided: u16,
    /// census: every distinct reason the function itself is not supported (empty = supported by itself; `calls a function that is not supported` is not one)
    pub direct: Vec<String>,
    /// census: entries this function calls directly or tail-jumps to
    pub callees: Vec<u64>,
    /// decision 0285 (phase 1, diagnostic only; hybrid mode): whether the function would fit the uniform indirect-call signature (rsp, rcx, rdx, r8, r9, rax -> rax):
    /// `fit`, or `nofit: <reasons>`. Empty outside hybrid mode and for functions that are not lowered.
    pub indirect: String,
}

#[derive(Default)]
pub struct Report {
    pub entries: usize,
    pub rows: Vec<FnRow>,
    /// indirect jmps that fe-decode resolved to an opaque function's entry (looked up at run time again)
    pub demoted_resolved_jumps: usize,
    /// the functions that have such a jump (they call the opaque function through the lookup of the slot, which counts as an indirect call of the guest; the C path jumps to it without counting)
    pub demoted_fns: Vec<u64>,
    pub stats: LowerStats,
    pub reasons: BTreeMap<String, (usize, usize)>,
    pub irreducible: usize,
    pub module_bytes: usize,
    pub code_bytes: usize,
    pub adapter_bytes: usize,
    pub sig_iterations: usize,
    pub pres_iterations: usize,
    pub nonconforming: usize,
    pub nonconf_reasons: BTreeMap<String, usize>,
    pub indirect_violations: usize,
    pub imports: usize,
    pub unbalanced: usize,
    pub noreturn: usize,
    /// census: unsupported instruction (reason) -> occurrences, in functions that passed the flag analysis
    pub unsupported: BTreeMap<String, usize>,
    pub indirect_adapters: usize,
    pub test_adapter_bytes: usize,
    /// decision 0285 (phase 1, diagnostic only; hybrid mode): the lowered functions that fit / do not fit the uniform indirect-call signature, the code bytes the standalone adapters of the fitting ones would take, and the reasons of the others
    pub indirect_fit: usize,
    pub indirect_nofit: usize,
    pub indirect_fit_adapter_bytes: usize,
    /// decision 0285: size of the table of adapters (slots), the buckets (slots that hold more than one function) and the largest bucket
    pub typed_slots: usize,
    pub typed_buckets: usize,
    pub typed_max_bucket: usize,
    pub indirect_nofit_reasons: BTreeMap<String, usize>,
    /// hybrid mode: functions left to the C path
    pub ext: usize,
    pub wrapper_bytes: usize,
    /// root reason -> (functions with it, functions that call them transitively counted per root [overlaps], instructions of those)
    pub blocked: BTreeMap<String, (usize, usize, usize)>,
}

pub struct Compiled {
    pub wasm: Vec<u8>,
    pub report: Report,
    pub fidx: BTreeMap<u64, u32>,
}

struct F {
    fun: Fun,
    cfg: Option<cfg::Cfg>,
    fl: Option<flags::FlagInfo>,
    err: Option<String>,
    callees: Vec<u64>,
    closed: bool,
    direct: Vec<String>,
    /// hybrid mode: nothing is known about the function (undecodable, or RSP is not balanced at a return): calls to it are not compiled
    opaque: bool,
}

/// Direct callees and tail-jump targets of a function (census graph).
fn call_targets(fun: &Fun) -> Vec<u64> {
    let mut v: BTreeSet<u64> = BTreeSet::new();
    for b in &fun.blocks {
        if let ir::Term::Tail(g) = b.term {
            v.insert(g);
        }
        for &va in &b.insns {
            let i = fun.insn(va);
            if i.mnemonic() == iced_x86::Mnemonic::Call && ir::is_near(i) {
                v.insert(i.near_branch_target());
            }
        }
    }
    v.into_iter().collect()
}

/// Hybrid with the C runtime: an indirect call may be a call of a guard thunk (`call [__guard_dispatch_icall_fptr]` with the target in rax; `jmp rax`), so rax is read by it.
static INDIRECT_READS_RAX: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn first_word(s: &str) -> String {
    // normalise an error message to a reason class
    let mut t: String = s.chars().take(80).collect();
    if let Some(p) = t.find(" @ ") {
        t.truncate(p);
    }
    if let Some(p) = t.find(" at 0x") {
        t.truncate(p);
    }
    if let Some(p) = t.find("0x") {
        t.truncate(p);
    }
    t.trim().to_string()
}

const CALLEE_SAVED: u16 = (1 << 3) | (1 << 5) | (1 << 6) | (1 << 7) | (0xf << 12);
const ARG_REGS: u16 = (1 << 1) | (1 << 2) | (1 << 8) | (1 << 9);
const VOLATILE_NOT_RAX: u16 = ARG_REGS | (1 << 10) | (1 << 11);

struct Solution {
    pres: HashMap<u64, u16>,
    skip: HashMap<u64, HashSet<u64>>,
    elided: HashMap<u64, u16>,
    sigs: HashMap<u64, Sig>,
    /// functions with a value live across an indirect call in a register the callee may clobber
    violations: Vec<(u64, u16)>,
    unbalanced: Vec<u64>,
    noreturn: HashSet<u64>,
    pres_iterations: usize,
    sig_iterations: usize,
}

/// registers an ordinary function may change (rax, rcx, rdx, r8..r11)
const VOLATILE: u16 = (1 << 0) | (1 << 1) | (1 << 2) | (0xf << 8);

/// decision 0252: the signature of a function without a flow graph that a compiled caller calls as a stub (see `abi_helper`): it takes its arguments in rcx, rdx, r8, r9 (and rax), returns the volatile registers and preserves the others
fn nocfg_sig() -> Sig {
    Sig { inr: ARG_REGS | (1 << RAX), ins: ARG_REGS | (1 << RAX), out: VOLATILE, rsp: 0 }
}

fn solve(funs: &BTreeMap<u64, F>, closed: &[u64], nocfg: &BTreeSet<u64>, effs: &HashMap<u64, HashMap<u64, Eff>>, conf: &Config, fx: &mut Fx) -> Solution {
    // functions that never return (every path ends in a trap or in a call that does not return): an error handler. What they read of the
    // registers beyond their arguments (a context capture reads all of them) is not a value that matters, and nothing after the call runs.
    let mut noreturn: HashSet<u64> = closed.iter().copied().filter(|k| !funs[k].fun.blocks.iter().any(|b| matches!(b.term, ir::Term::Ret | ir::Term::TailImport(_) | ir::Term::TailIndirect))).collect();
    loop {
        let snapshot = noreturn.clone();
        noreturn.retain(|k| funs[k].fun.blocks.iter().all(|b| match b.term {
            ir::Term::Tail(g) => snapshot.contains(&g),
            _ => true,
        }));
        if noreturn.len() == snapshot.len() {
            break;
        }
    }
    // preserved registers: greatest fixpoint
    let mut pres: HashMap<u64, u16> = closed.iter().map(|&k| (k, ALL_REGS)).collect();
    for &k in nocfg {
        pres.insert(k, ALL_REGS & !VOLATILE);
    }
    // decision 0261: where RSP is after a function returns, relative to its entry value (`pres::PresResult::delta`): Some(0) for the ordinary function, Some(k) for a shared epilogue that is entered by a jmp
    // from the frame of another function (its callers and the functions that jump to it account for k), None: not a constant. Starts at Some(0) for every function and is recomputed until nothing changes (a function that jumps to a shared epilogue whose k is not known yet looks unbalanced in the first rounds).
    let mut rdelta: HashMap<u64, Option<i64>> = closed.iter().map(|&k| (k, Some(0))).collect();
    let mut pres_iterations = 0;
    loop {
        pres_iterations += 1;
        let mut updates: Vec<(u64, u16)> = Vec::new();
        let mut dupdates: Vec<(u64, Option<i64>)> = Vec::new();
        for &k in closed {
            let f = &funs[&k];
            let r = pres::analyze(&f.fun, f.cfg.as_ref().unwrap(), fx, &|g| if noreturn.contains(&g) { Some(ALL_REGS) } else { pres.get(&g).copied() }, &|_| 0, &|g| rdelta.get(&g).copied().unwrap_or(Some(0)), None);
            let np = r.pres & pres[&k];
            if np != pres[&k] {
                updates.push((k, np));
            }
            if r.delta != rdelta[&k] {
                dupdates.push((k, r.delta));
            }
        }
        let changed = !updates.is_empty() || !dupdates.is_empty();
        for (k, v) in updates {
            pres.insert(k, v);
        }
        for (k, v) in dupdates {
            rdelta.insert(k, v);
        }
        if !changed || pres_iterations > 60 {
            break;
        }
    }
    let mut unbalanced: Vec<u64> = Vec::new();
    for &k in closed {
        let f = &funs[&k];
        let r = pres::analyze(&f.fun, f.cfg.as_ref().unwrap(), fx, &|g| if noreturn.contains(&g) { Some(ALL_REGS) } else { pres.get(&g).copied() }, &|_| 0, &|g| rdelta.get(&g).copied().unwrap_or(Some(0)), None);
        // the RSP offset must be a constant that all exits agree on (and, once the fixpoint has settled, the same as the one the callers use)
        if r.delta.is_none() || r.delta != rdelta[&k] {
            unbalanced.push(k);
        }
    }
    let mut skip: HashMap<u64, HashSet<u64>> = HashMap::new();
    let mut elided: HashMap<u64, u16> = HashMap::new();
    let mut sigs: HashMap<u64, Sig> = closed.iter().map(|&k| (k, Sig { rsp: rdelta[&k].unwrap_or(0), ..Sig::default() })).collect();
    for &k in nocfg {
        sigs.insert(k, nocfg_sig());
    }
    let ext: u16 = 1 << RAX;
    let exported = |k: u64| conf.export.as_ref().map_or(false, |s| s.is_empty() || s.contains(&k)) || conf.hybrid_r.is_some();
    // hybrid: what a caller on the C path reads of the results is unknown: a function that such a caller calls returns every register it changes
    let called_from_c: HashSet<u64> = if conf.hybrid_r.is_some() { funs.values().filter(|f| !f.closed).flat_map(|f| call_targets(&f.fun)).collect() } else { HashSet::new() };
    let mut sig_iterations = 0;
    for _round in 0..8 {
        let mut new_skip: HashMap<u64, HashSet<u64>> = HashMap::new();
        let mut new_el: HashMap<u64, u16> = HashMap::new();
        for &k in closed {
            let f = &funs[&k];
            let mut ev = pres::Events::default();
            let _ = pres::analyze(
                &f.fun,
                f.cfg.as_ref().unwrap(),
                fx,
                &|g| if noreturn.contains(&g) { Some(ALL_REGS) } else { pres.get(&g).copied() },
                &|g| if noreturn.contains(&g) { ARG_REGS } else { sigs.get(&g).map_or(ALL_REGS, |s| s.inr) },
                &|g| rdelta.get(&g).copied().unwrap_or(Some(0)),
                Some(&mut ev),
            );
            let (regs, sk) = pres::elision(&ev, pres[&k]);
            new_skip.insert(k, sk);
            new_el.insert(k, regs);
        }
        let mut inner = 0;
        loop {
            inner += 1;
            let mut demand: HashMap<u64, u16> = HashMap::new();
            let mut new_in: HashMap<u64, (u16, u16)> = HashMap::new();
            for &k in closed {
                let f = &funs[&k];
                let (li, ls, calls, _) = liveness(f, &effs[&k], &sigs, &pres, &noreturn, sigs[&k].out, &new_skip[&k]);
                new_in.insert(k, (li & ALL_REGS, ls & ALL_REGS));
                for (g, la) in calls {
                    *demand.entry(g).or_insert(0) |= la;
                }
            }
            let mut changed = false;
            for &k in closed {
                let modm = !pres[&k] & ALL_REGS;
                let want = demand.get(&k).copied().unwrap_or(0) | if exported(k) { ext } else { 0 } | if called_from_c.contains(&k) { ALL_REGS } else { 0 }
                    // decision 0264 (mt3z): a function that leaves RSP above its entry value is a continuation entry of another frame; whoever enters it (a jmp the module does not show, a dispatch) expects the registers it pops
                    | if rdelta[&k] != Some(0) { ALL_REGS } else { 0 };
                let out = (modm & want & ALL_REGS) | sigs[&k].out;
                // an output that the function does not define on some path is live into it (the return reads it), so no extra term is needed
                let inr = new_in[&k].0 | sigs[&k].inr;
                let ins = (new_in[&k].1 | sigs[&k].ins) & inr;
                if out != sigs[&k].out || inr != sigs[&k].inr || ins != sigs[&k].ins {
                    sigs.insert(k, Sig { inr, ins, out, ..sigs[&k] });
                    changed = true;
                }
            }
            if !changed || inner > 80 {
                break;
            }
        }
        sig_iterations += inner;
        let stable = new_skip == skip;
        skip = new_skip;
        elided = new_el;
        if stable {
            break;
        }
    }
    let mut violations = Vec::new();
    for &k in closed {
        let f = &funs[&k];
        let (_, _, _, ind) = liveness(f, &effs[&k], &sigs, &pres, &noreturn, sigs[&k].out, &skip[&k]);
        let bad = ind.iter().fold(0u16, |a, &la| a | (la & VOLATILE_NOT_RAX));
        if bad != 0 {
            violations.push((k, bad));
        }
    }
    Solution { pres, skip, elided, sigs, violations, unbalanced, noreturn, pres_iterations, sig_iterations }
}

/// decision 0252: is `fun` (which has no flow graph) an ordinary function of the Windows x64 convention for its callers? The compiled caller of a stub passes rcx, rdx, r8, r9 and rax and takes the volatile registers
/// back; the other registers stay in its locals (the callee preserves them) and r10 / r11 are not passed. So the function must (1) read, before it writes them, only rsp, rcx, rdx, r8, r9, rax and the callee-saved
/// registers that it pushes (to restore them) and (2) write no callee-saved register that it has not pushed before (in address order; the function is small and straight enough that this is the order of execution
/// where it matters). Anything else (a custom convention: an implicit r10 / r11 / rbx input) stays opaque. The reads of XMM registers are not a condition: the XMM file travels through X[] (xabi).
fn abi_helper(fun: &ir::Fun, fx: &mut Fx) -> bool {
    use iced_x86::Mnemonic as M;
    let ok_in: u16 = ARG_REGS | (1 << RAX);
    let mut written: u16 = 0;
    let mut pushed: u16 = 0;
    for n in fun.nodes.values() {
        let ir::Node::Insn(i) = n else { return false };
        if i.mnemonic() == M::Push && i.op_kind(0) == iced_x86::OpKind::Register {
            if let Some(r) = fx::gpr(i.op_register(0)) {
                if CALLEE_SAVED & (1 << r) != 0 {
                    pushed |= 1 << r;
                    continue;
                }
            }
        }
        let e = fx.eff(i);
        if e.reads & !(ok_in | written) != 0 {
            return false;
        }
        if e.writes & CALLEE_SAVED & !pushed != 0 {
            return false;
        }
        written |= e.writes | e.defs;
    }
    true
}

/// Hybrid mode: a function that calls (or tail-jumps to) an opaque function is not compiled either (it stays on the C path); a call to any other
/// function that stays on the C path is a stub call.
fn propagate_hybrid(funs: &mut BTreeMap<u64, F>) {
    loop {
        let mut changed = false;
        let opaque: HashSet<u64> = funs.iter().filter(|(_, f)| f.opaque).map(|(&k, _)| k).collect();
        let known: HashSet<u64> = funs.keys().copied().collect();
        for f in funs.values_mut() {
            if !f.closed {
                continue;
            }
            // a callee that is not a known function entry cannot be called through a stub either
            let mut bad = f.callees.iter().any(|c| opaque.contains(c) || !known.contains(c));
            for b in &f.fun.blocks {
                if let ir::Term::Tail(g) = b.term {
                    bad |= opaque.contains(&g) || !known.contains(&g);
                }
            }
            if bad {
                f.closed = false;
                f.err = Some("calls a function that is opaque".to_string());
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
}

/// Marks as not closed every function that calls (or tail-jumps to) something that is not closed; with `roots`, also everything the roots
/// do not reach.
fn propagate_closed(funs: &mut BTreeMap<u64, F>, roots: &Option<HashSet<u64>>) {
    loop {
        let mut changed = false;
        let ok: HashSet<u64> = funs.iter().filter(|(_, f)| f.closed).map(|(&k, _)| k).collect();
        for f in funs.values_mut() {
            if !f.closed {
                continue;
            }
            if f.callees.iter().any(|c| !ok.contains(c)) {
                f.closed = false;
                f.err = Some("calls a function that is not supported".to_string());
                changed = true;
                continue;
            }
            for b in &f.fun.blocks {
                if let ir::Term::Tail(g) = b.term {
                    if !ok.contains(&g) {
                        f.closed = false;
                        f.err = Some("tail-jumps to a function that is not supported".to_string());
                        changed = true;
                        break;
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }
    if let Some(roots) = roots {
        let mut keep: HashSet<u64> = HashSet::new();
        let mut work: Vec<u64> = roots.iter().copied().filter(|r| funs.get(r).map_or(false, |f| f.closed)).collect();
        while let Some(x) = work.pop() {
            if keep.insert(x) {
                work.extend(funs[&x].callees.iter().copied());
                for b in &funs[&x].fun.blocks {
                    if let ir::Term::Tail(g) = b.term {
                        work.push(g);
                    }
                }
            }
        }
        for (k, f) in funs.iter_mut() {
            if f.closed && !keep.contains(k) {
                f.closed = false;
                f.err = Some("not in the selection".to_string());
            }
        }
    }
}

/// decision 0270 (mt3w): what the function at `g` does to the flags when it returns (`flagout::summary`); the summaries of the functions that it calls or jumps to are computed on the way,
/// down to `FLAGOUT_DEPTH` levels and for functions of at most `FLAGOUT_BLOCKS` blocks (the stack-cookie check and the failure handler it jumps to have 6 and 4 blocks)
const FLAGOUT_DEPTH: usize = 4;
const FLAGOUT_BLOCKS: usize = 64;

#[allow(clippy::too_many_arguments)]
fn flag_out_of(g: u64, depth: usize, text: &Text, entries: &BTreeSet<u64>, a: &fe_decode::Analysis, demote: &HashSet<u64>, fx: &mut Fx, memo: &mut HashMap<u64, flagout::Out>, busy: &mut HashSet<u64>) -> flagout::Out {
    if let Some(o) = memo.get(&g) {
        return *o;
    }
    if !entries.contains(&g) || depth == 0 || !busy.insert(g) {
        return flagout::Out::Unknown;
    }
    let nodes = ir::traverse(g, text, entries, &a.resolved, &a.jumptabs);
    let gf = ir::build(g, nodes, entries, a, demote);
    let o = if gf.blocks.len() > FLAGOUT_BLOCKS || gf.nodes.values().any(|n| matches!(n, ir::Node::Bad)) {
        flagout::Out::Unknown
    } else {
        match cfg::analyze(&gf.blocks) {
            Err(_) => flagout::Out::Unknown,
            Ok(gc) => {
                let import_at = |va: u64| if let Some(fe_decode::Resolved::Import { name }) = a.resolved.get(&va) { Some(name.clone()) } else { None };
                let mut callee = |t: u64, fx: &mut Fx| flag_out_of(t, depth - 1, text, entries, a, demote, fx, memo, busy);
                flagout::summary(&gf, &gc, fx, &mut flagout::Env { callee: &mut callee, import_at: &import_at })
            }
        }
    };
    busy.remove(&g);
    memo.insert(g, o);
    o
}

pub fn compile(data: &[u8], extra: &[u64], specs: &[fe_decode::Spec], conf: &Config) -> Result<Compiled, String> {
    let pe = Pe::parse(data).map_err(|e| e.to_string())?;
    let text = Text::from_pe(&pe)?;
    let a = fe_decode::analyze_with_options(&pe, extra, specs, &fe_decode::Options::default())?;
    let entries: BTreeSet<u64> = a.functions.keys().copied().collect();
    let mut rep = Report::default();
    INDIRECT_READS_RAX.store(conf.c_imports.is_some(), std::sync::atomic::Ordering::Relaxed);
    xabi::XMEM.store(conf.hybrid_x.is_some(), std::sync::atomic::Ordering::Relaxed);
    wasm::NIMPG.with(|c| c.set(0));
    wasm::TLS_LOCAL.with(|c| c.set(u32::MAX));
    match (conf.tls_ctx, conf.hybrid_r) {   // decision 0234 (mt3, B-prime stage 1): the register page of the C path is thread-local
        (Some(off), Some(rb)) => {
            wasm::TLS_RB.with(|c| c.set(rb));
            wasm::TLS_OFF.with(|c| c.set(off));
            wasm::TLS_ON.with(|c| c.set(1));
        }
        (Some(_), None) => return Err("--tls-ctx needs --hybrid-r".into()),
        (None, Some(_)) => return Err("--hybrid-r needs --tls-ctx".into()),
        (None, None) => wasm::TLS_ON.with(|c| c.set(0)),
    }
    rep.entries = entries.len();
    let mut fx = Fx::new();
    let mut funs: BTreeMap<u64, F> = BTreeMap::new();
    let mk = |e: u64, demote: &HashSet<u64>, fx: &mut Fx, rep: &mut Report| -> F {
        let nodes = ir::traverse(e, &text, &entries, &a.resolved, &a.jumptabs);
        let fun = ir::build(e, nodes, &entries, &a, demote);
        let mut err = None;
        let mut cfgo = None;
        let mut flo = None;
        if fun.nodes.values().any(|n| matches!(n, ir::Node::Bad)) {
            err = Some("undecodable or out-of-text instruction".to_string());
        } else {
            match cfg::analyze(&fun.blocks) {
                Err(e) => err = Some(e),
                Ok(c) => match flags::analyze(&fun, &c, fx).or_else(|e| {
                    // decision 0270 (mt3w): a flag read after a call: the callees that return with known flags (flagout.rs)
                    if !e.contains("Call Opaque") {
                        return Err(e);
                    }
                    let mut memo: HashMap<u64, flagout::Out> = HashMap::new();
                    let mut busy: HashSet<u64> = HashSet::new();
                    for g in call_targets(&fun) {
                        flag_out_of(g, FLAGOUT_DEPTH, &text, &entries, &a, demote, fx, &mut memo, &mut busy);
                    }
                    let known: HashMap<u64, (u8, u8)> = memo.iter().filter_map(|(g, o)| if let flagout::Out::Flags(m, v) = o { Some((*g, (*m, *v))) } else { None }).collect();
                    flags::analyze_with(&fun, &c, fx, &|g| known.get(&g).copied())
                }) {
                    Err(e) => err = Some(e),
                    Ok(f) => {
                        if !c.reducible {
                            rep.irreducible += 1;
                        }
                        flo = Some(f);
                        cfgo = Some(c);
                    }
                },
            }
        }
        let direct: Vec<String> = err.iter().cloned().collect();
        let opaque = cfgo.is_none();
        F { fun, cfg: cfgo, fl: flo, err, callees: vec![], closed: false, direct, opaque }
    };
    let none: HashSet<u64> = HashSet::new();
    for &e in &entries {
        let f = mk(e, &none, &mut fx, &mut rep);
        funs.insert(e, f);
    }
    // A jmp that fe-decode resolved to the entry of a function that this backend cannot analyse (opaque) cannot be a tail call of it; it goes back to the lookup of the slot's value at run time
    // (`TailIndirect`, the C path's dispatch in hybrid mode): the engine's IPP dispatch stubs (`mov rax,[slot]; jmp rax`) are of that kind.
    {
        let opaque: HashSet<u64> = funs.iter().filter(|(_, f)| f.opaque).map(|(&k, _)| k).collect();
        let mut redo: Vec<(u64, HashSet<u64>)> = Vec::new();
        for (&e, f) in &funs {
            let mut d: HashSet<u64> = HashSet::new();
            for b in &f.fun.blocks {
                if let ir::Term::Jmp(t) = b.term {
                    if let (Some(&last), ir::Term::Tail(g)) = (b.insns.last(), &f.fun.blocks[t].term) {
                        if f.fun.insn(last).flow_control() == iced_x86::FlowControl::IndirectBranch && opaque.contains(g) {
                            d.insert(last);
                        }
                    }
                }
            }
            if !d.is_empty() {
                redo.push((e, d));
            }
        }
        rep.demoted_resolved_jumps = redo.iter().map(|(_, d)| d.len()).sum();
        rep.demoted_fns = redo.iter().map(|(e, _)| *e).collect();
        for (e, d) in redo {
            let f = mk(e, &d, &mut fx, &mut rep);
            funs.insert(e, f);
        }
    }
    let text_lo = text.ranges().first().map(|r| r.va).unwrap_or(0);
    let text_hi = text.ranges().last().map(|r| r.va + r.bytes.len() as u64).unwrap_or(0);
    if text_hi >> 32 != 0 {
        return Err("code above 4 GiB".into());
    }
    let mut module = Module::default();
    if wasm::TLS_ON.with(|c| c.get()) != 0 {
        module.import_global("c", "__tls_base");
    }
    if let Some((m, n)) = &conf.mem_import {
        if conf.tls_ctx.is_some() { module.import_memory_shared(m, n, 1, 32768); } else { module.import_memory(m, n, 1, None); }
    } else {
        let name = if conf.hybrid_r.is_some() { "memory" } else { "mem" };
        if conf.tls_ctx.is_some() { module.import_memory_shared("env", name, 1, 32768); } else { module.import_memory("env", name, 1, None); }
    }
    let ui = FuncType { params: vec![Ty::I64; 6], results: vec![Ty::I64] };
    let ui_ty = module.type_index(ui.clone());
    let import_sites: HashMap<u64, String> = a
        .resolved
        .iter()
        .filter_map(|(&va, r)| if let fe_decode::Resolved::Import { name } = r { Some((va, name.clone())) } else { None })
        .collect();
    let no_names: HashMap<String, u32> = HashMap::new();
    let mut effs: HashMap<u64, HashMap<u64, Eff>> = HashMap::new();
    for (&k, f) in &funs {
        if f.cfg.is_none() || (f.fl.is_none() && conf.hybrid_r.is_none()) {
            continue;
        }
        let mut m = HashMap::new();
        for b in &f.fun.blocks {
            for &va in &b.insns {
                m.insert(va, fx.eff(f.fun.insn(va)));
            }
        }
        effs.insert(k, m);
    }
    // dry lowering: which functions are supported by themselves
    let opts = Opts { teb: conf.teb };
    let empty_sigs: HashMap<u64, Sig> = HashMap::new();
    let empty_idx: HashMap<u64, u32> = HashMap::new();
    let no_ext: HashMap<u64, u32> = HashMap::new();
    let c_mode = conf.c_imports.is_some();
    let mut c_imp_dry: HashMap<String, u32> = conf.c_imports.iter().flat_map(|m| m.keys()).map(|n| (n.clone(), 0u32)).collect();
    if c_mode {
        // decision 0259: the x87 helpers of the C runtime (any of them may be asked for in the dry run; the real module imports the ones that are used)
        for (n, _, _) in lower::x87::HELPERS {
            c_imp_dry.insert((*n).to_string(), 0);
        }
    }
    let no_xplans: HashMap<u64, xabi::XPlan> = HashMap::new();
    let dry = Ctx { c_mode, c_imp: &c_imp_dry, c_dispatch: 0, ext: &no_ext, hybrid_r: 0, sigs: &empty_sigs, fidx: &empty_idx, opts: &opts, dry: true, ui_ty, text_lo: text_lo as u32, mulhu: 0, mulhs: 0, import_sites: &import_sites, import_idx: &no_names, mx_window: c_mode && conf.hybrid_mx.is_some(), mx_slow_i64: 0, census: conf.census, hybrid_x: 0, xplans: &no_xplans, mx_addr: if conf.hybrid_mx.is_some() && c_mode { 1 } else { 0 }, mx_buf: 0, mx_hyb: 0, mx_ld: 0, mx_st: 0, x87_ld: if c_mode { 1 } else { 0 }, x87_st: if c_mode { 1 } else { 0 }, progress_addr: 0, gs_ld: if c_mode { 1 } else { 0 }, ti_helper: 0 };
    let no_skip: HashSet<u64> = HashSet::new();
    // decision 0185: a function whose dry-run wasm body exceeds the limits of the Config (bytes, locals, bytes x locals) stays on the C path
    let (lim_bytes, lim_locals, lim_cost) = (conf.max_fn_bytes, conf.max_fn_locals, conf.max_fn_cost);
    let keys: Vec<u64> = funs.keys().copied().collect();
    for &k in &keys {
        let f = funs.get_mut(&k).unwrap();
        if f.err.is_some() {
            continue;
        }
        let mut st = LowerStats::default();
        match lower::lower(&f.fun, f.cfg.as_ref().unwrap(), f.fl.as_ref().unwrap(), Sig::default(), &dry, &no_skip, &mut st) {
            Ok(body) if st.insn_errors.is_empty() && {
                let (nb, nl) = (body.code.len() as u64, body.locals.iter().map(|r| r.0 as u64).sum::<u64>());
                nb > lim_bytes || nl > lim_locals || nb * nl > lim_cost
            } =>
            {
                let (nb, nl) = (body.code.len(), body.locals.iter().map(|r| r.0 as u64).sum::<u64>());
                let e = format!("over the size limit (hybrid): {} wasm bytes, {} locals", nb, nl);
                f.direct = vec![e.clone()];
                f.err = Some(e);
            }
            Ok(_) if !st.insn_errors.is_empty() => {
                for (e, n) in &st.insn_errors {
                    *rep.unsupported.entry(e.clone()).or_insert(0) += n;
                }
                f.err = Some(st.insn_errors.keys().next().unwrap().clone());
                f.direct = st.insn_errors.keys().cloned().collect();
            }
            Ok(_) => match if conf.hybrid_x.is_some() { Ok(()) } else { xmm_check(f, &effs[&k]) } {
                Ok(()) => f.callees = st.callees.clone(),
                Err(e) => {
                    f.direct = vec![e.clone()];
                    f.err = Some(e)
                }
            },
            Err(e) => {
                f.direct = vec![e.clone()];
                f.err = Some(e)
            }
        }
    }
    if let Some(only) = &conf.hybrid_only {
        for (k, f) in funs.iter_mut() {
            if f.err.is_none() && !only.contains(k) {
                f.err = Some("not selected for the new backend (hybrid)".to_string());
            }
        }
    }
    // decision 0270b (mt3w): a function whose flags a caller on the C path reads after the call stays on the C path (the wrapper of a direct function writes no flag to `FL`; flagread.rs)
    if conf.hybrid_r.is_some() {
        loop {
            let c_path: HashSet<u64> = funs.iter().filter(|(_, f)| f.err.is_some()).map(|(&k, _)| k).collect();
            let hit: Vec<u64> = flagread::read_after_call(&a, &c_path).into_iter().filter(|t| funs.get(t).map_or(false, |f| f.err.is_none())).collect();
            if hit.is_empty() {
                break;
            }
            for t in hit {
                let f = funs.get_mut(&t).unwrap();
                f.err = Some("flags are read after a call of it by a caller on the C path (they do not cross the C wrapper)".to_string());
                f.direct = vec![f.err.clone().unwrap()];
            }
        }
    }
    for f in funs.values_mut() {
        f.closed = f.err.is_none();
    }
    let hybrid = conf.hybrid_r.is_some();
    // decision 0252 (what decision 0181 had as the opt-in `--opaque-stub`): a function whose flow analysis fails only because a flag is used in a way that has no recipe (`shr r, cl` followed by `jae` or `adc`:
    // the flags depend on cl == 0; a flag merged from such a definition; AF) has no flow graph, so it was opaque and every caller stayed on the C path. It is an ordinary function of the C path all the same: a compiled
    // caller calls it as a stub (the registers of the Windows x64 convention; every XMM passed and returned). A function that reads a flag that is live INTO it, or that cannot be decoded, stays opaque: its caller's flags
    // would have to be handed over. So does one that is not an ordinary function of the convention (`abi_helper`).
    let mut nocfg: BTreeSet<u64> = BTreeSet::new();
    if hybrid {
        for (k, f) in funs.iter_mut() {
            if f.opaque && f.cfg.is_none() && f.err.as_deref().map_or(false, |e| e.starts_with("flag read from a definition without recipe") || e.starts_with("flag merge from a definition without recipe") || e.starts_with("reads AF")) && abi_helper(&f.fun, &mut fx) {
                f.opaque = false;
                nocfg.insert(*k);
            }
        }
    }
    if hybrid {
        propagate_hybrid(&mut funs);
    } else {
        propagate_closed(&mut funs, &conf.roots);
    }
    // which root reasons block the most functions (by reverse reachability over the direct call graph)
    {
        let mut callers: HashMap<u64, Vec<u64>> = HashMap::new();
        for (&k, f) in &funs {
            for b in &f.fun.blocks {
                if let ir::Term::Tail(g) = b.term {
                    callers.entry(g).or_default().push(k);
                }
                for &va in &b.insns {
                    let i = f.fun.insn(va);
                    if i.mnemonic() == iced_x86::Mnemonic::Call && ir::is_near(i) {
                        callers.entry(i.near_branch_target()).or_default().push(k);
                    }
                }
            }
        }
        let mut blocked: BTreeMap<String, (usize, usize, usize)> = BTreeMap::new();
        for (&k, f) in &funs {
            let Some(e) = &f.err else { continue };
            if e.starts_with("calls a function") || e.starts_with("tail-jumps") || e.starts_with("not in the selection") {
                continue;
            }
            let mut seen: HashSet<u64> = HashSet::new();
            let mut work = vec![k];
            while let Some(x) = work.pop() {
                if seen.insert(x) {
                    if let Some(cs) = callers.get(&x) {
                        work.extend(cs.iter().copied());
                    }
                }
            }
            let ins: usize = seen.iter().map(|x| funs[x].fun.nodes.len()).sum();
            let ent = blocked.entry(first_word(e)).or_insert((0, 0, 0));
            ent.0 += 1;
            ent.1 += seen.len();
            ent.2 += ins;
        }
        rep.blocked = blocked;
    }
    let sol = loop {
        let closed: Vec<u64> = funs.iter().filter(|(_, f)| if hybrid { !f.opaque && f.cfg.is_some() } else { f.closed }).map(|(&k, _)| k).collect();
        let sol = solve(&funs, &closed, &nocfg, &effs, conf, &mut fx);
        rep.pres_iterations += sol.pres_iterations;
        rep.sig_iterations += sol.sig_iterations;
        rep.indirect_violations = sol.violations.len();
        if !sol.unbalanced.is_empty() {
            for v in &sol.unbalanced {
                let f = funs.get_mut(v).unwrap();
                f.closed = false;
                f.opaque = true;
                f.err = Some("RSP at a return or tail jump is not its entry value (a continuation entry inside another frame, or a callee that pops)".to_string());
                f.direct = vec![f.err.clone().unwrap()];
                rep.unbalanced += 1;
            }
            if hybrid {
                propagate_hybrid(&mut funs);
            } else {
                propagate_closed(&mut funs, &conf.roots);
            }
            continue;
        }
        break sol;
    };
    let closed: Vec<u64> = funs.iter().filter(|(_, f)| f.closed).map(|(&k, _)| k).collect();
    rep.noreturn = sol.noreturn.len();
    let (pres, skip, elided, sigs) = (&sol.pres, &sol.skip, &sol.elided, &sol.sigs);
    let exported = |k: u64| conf.export.as_ref().map_or(false, |s| s.is_empty() || s.contains(&k));
    let mut xplans: HashMap<u64, xabi::XPlan> = HashMap::new();
    let mut xsums: HashMap<u64, xabi::XSum> = HashMap::new();
    if conf.hybrid_x.is_some() {
        let mut m: HashMap<u64, (&ir::Fun, &cfg::Cfg, &HashMap<u64, Eff>)> = HashMap::new();
        for (&k, f) in &funs {
            if let (Some(c), Some(e)) = (f.cfg.as_ref(), effs.get(&k)) {
                m.insert(k, (&f.fun, c, e));
            }
        }
        let mut sums = xabi::summaries(&m, &import_sites);
        for &k in funs.keys() {
            if !m.contains_key(&k) {
                sums.insert(k, xabi::XSum { xin: xabi::ALLX, xw: xabi::ALLX });
            }
        }
        for (&k, x) in &sums {
            xsums.insert(k, *x);
        }
        for &k in &closed {
            let (f, c, e) = m[&k];
            xplans.insert(k, xabi::plan(f, c, e, &sums, &import_sites));
        }
    }
    // decision 0285: which of the lowered functions fit the uniform indirect-call signature (the predicate of the standalone adapters below, plus: no XMM register read or written, also by the callees; no stack popped by the callee)
    let fit_why = |k: u64| -> Vec<String> {
        let mut why: Vec<String> = Vec::new();
        let (real_in, _, _, _) = liveness(&funs[&k], &effs[&k], sigs, pres, &sol.noreturn, 0, &skip[&k]);
        let fits = ARG_REGS | (1 << RAX);
        if real_in & !fits != 0 {
            why.push(format!("reads {:#06x}", real_in & !fits));
        }
        if pres[&k] & CALLEE_SAVED != CALLEE_SAVED {
            why.push(format!("does not preserve {:#06x}", !pres[&k] & CALLEE_SAVED));
        }
        if sigs[&k].rsp != 0 {
            why.push("pops the stack".to_string());
        }
        match xsums.get(&k) {
            Some(x) if x.xin != 0 || x.xw != 0 => why.push(format!("xmm in {:#06x} written {:#06x}", x.xin, x.xw)),
            Some(_) => {}
            None => why.push("xmm unknown (no hybrid-x summary)".to_string()),
        }
        why
    };
    let fit_map: HashMap<u64, Vec<String>> = if c_mode { closed.iter().map(|&k| (k, fit_why(k))).collect() } else { HashMap::new() };
    let typed_cfg: Option<(u32, u32, u32)> = conf.hybrid_dispatch.filter(|_| c_mode && conf.hybrid_progress.is_some() && conf.hybrid_x.is_some());
    let mut fit: Vec<u64> = closed.iter().copied().filter(|k| fit_map.get(k).map_or(false, |w| w.is_empty())).collect();
    fit.sort();
    // a module whose lowered functions hold no indirect call gets nothing (no helper, no adapters, no table)
    let n_ind_sites: usize = closed
        .iter()
        .map(|k| funs[k].fun.nodes.iter().filter(|(va, n)| matches!(n, ir::Node::Insn(i) if i.mnemonic() == iced_x86::Mnemonic::Call && !ir::is_near(i) && !import_sites.contains_key(*va))).count())
        .sum();
    let typed = typed_cfg.is_some() && !fit.is_empty() && n_ind_sites > 0;
    // imports used by the functions that are compiled (a call or tail jump through a resolved IAT slot), then the helpers
    let mut import_idx: HashMap<String, u32> = HashMap::new();
    let mut c_imp: HashMap<String, u32> = HashMap::new();
    {
        let mut names: BTreeSet<String> = BTreeSet::new();
        for &k in &closed {
            for &va in funs[&k].fun.nodes.keys() {
                if let Some(n) = import_sites.get(&va) {
                    names.insert(n.clone());
                }
            }
        }
        for n in names {
            if c_mode {
                let key = n.rsplit('!').next().unwrap_or(&n).to_string();
                if !c_imp.contains_key(&key) {
                    let sym = conf.c_imports.as_ref().and_then(|m| m.get(&key)).cloned().unwrap_or_else(|| format!("fle_imp_{key}"));
                    let idx = module.import_func("c", &sym, FuncType { params: vec![], results: vec![] });
                    c_imp.insert(key, idx);
                }
                continue;
            }
            let idx = module.import_func("env", &format!("imp_{n}"), FuncType { params: vec![Ty::I64; 5], results: vec![Ty::I64] });
            import_idx.insert(n, idx);
        }
    }
    if c_mode {
        // decision 0259: the C runtime's x87 functions that the instructions of the compiled functions call (imported only by a module that has such instructions)
        let used = lower::x87::helpers_of(closed.iter().flat_map(|k| funs[k].fun.nodes.values()).filter_map(|n| match n {
            ir::Node::Insn(i) => Some(i),
            _ => None,
        }));
        for name in used {
            let (params, results) = lower::x87::signature(name).expect("a planned helper has a signature");
            let idx = module.import_func("c", name, FuncType { params: params.to_vec(), results: results.to_vec() });
            c_imp.insert(name.to_string(), idx);
        }
    }
    let c_dispatch = if c_mode { module.import_func("c", "fle_hyb_dispatch", FuncType { params: vec![Ty::I64], results: vec![] }) } else { 0 };
    let gs_ld = if c_mode { module.import_func("c", "fle_hyb_gs_ld8", FuncType { params: vec![Ty::I64], results: vec![Ty::I64] }) } else { 0 };
    let ti_slice = if typed { module.import_func("c", "fle_hyb_slice", FuncType { params: vec![], results: vec![] }) } else { 0 };
    let mx_cfg = if c_mode { conf.hybrid_mx } else { None };
    let (mut mx_hyb, mut mx_ld, mut mx_st) = (0, 0, 0);
    if conf.hybrid_mx.is_some() && c_mode {
        // MXCSR of the C path: ldmxcsr/stmxcsr and the slow path of the window go to its functions
        mx_ld = module.import_func("c", "fle_mxcsr_load", FuncType { params: vec![Ty::I64], results: vec![] });
        mx_st = module.import_func("c", "fle_mxcsr_store", FuncType { params: vec![Ty::I64], results: vec![] });
        if mx_cfg.is_some() {
            mx_hyb = module.import_func("c", "fle_hyb_mx", FuncType { params: vec![Ty::I32, Ty::I32], results: vec![] });
        }
    }
    // the x87 control word (fnstcw / fldcw) is state of the C path (fle_x87.c): the direct code calls its two helpers, imported only when a function of the module has one of the instructions
    let (mut x87_ld, mut x87_st) = (0, 0);
    if c_mode && closed.iter().any(|k| funs[k].fun.nodes.values().any(|n| matches!(n, ir::Node::Insn(i) if matches!(i.mnemonic(), iced_x86::Mnemonic::Fnstcw | iced_x86::Mnemonic::Fldcw)))) {
        x87_ld = module.import_func("c", "fle_x87_load_cw", FuncType { params: vec![Ty::I64], results: vec![] });
        x87_st = module.import_func("c", "fle_x87_store_cw", FuncType { params: vec![Ty::I64], results: vec![] });
    }
    rep.imports = import_idx.len();
    // hybrid mode: the functions that stay on the C path are called through `cf_<va>` (void; registers in R[])
    let hybrid_r = conf.hybrid_r.unwrap_or(0);
    let used_ext: HashSet<u64> = funs.values().filter(|f| f.closed).flat_map(|f| call_targets(&f.fun)).collect();
    let ext: Vec<u64> = if hybrid { funs.iter().filter(|(k, f)| !f.opaque && !f.closed && used_ext.contains(k)).map(|(&k, _)| k).collect() } else { vec![] };
    let mut ext_idx: HashMap<u64, u32> = HashMap::new();
    for &k in &ext {
        let idx = module.import_func("c", &format!("f_{k:x}"), FuncType { params: vec![], results: vec![] });
        ext_idx.insert(k, idx);
    }
    rep.ext = ext.len();
    // MXCSR window / x86 integer-indefinite helpers (host functions) and the global that says MXCSR is in the default state
    let need_cvtt = closed.iter().any(|k| funs[k].fun.nodes.values().any(|n| matches!(n, ir::Node::Insn(i) if matches!(i.mnemonic(), iced_x86::Mnemonic::Cvttsd2si | iced_x86::Mnemonic::Cvttss2si))));
    let mut mx_slow_i64 = 0;
    let need_mx_helpers = (c_mode && conf.hybrid_mx.is_some()) || need_cvtt;
    // (the helpers used to be host imports; in the pilot the module is merged with the C path and has no host, so they are functions of the module: see below)
    // globals: 0 = mx_ok (MXCSR in the state the window rule assumes), 1 = MXCSR, 2 = x87 control word
    module.globals.push((1, Some("mx_ok".into())));
    module.globals.push((0x1f80, Some("mxcsr".into())));
    module.globals.push((0x27f, Some("fpucw".into())));
    let h2 = FuncType { params: vec![Ty::I64, Ty::I64], results: vec![Ty::I64] };
    let mulhu = module.add_func(h2.clone(), mulhu_body(false));
    let mulhs = module.add_func(h2, mulhu_body(true));
    if need_mx_helpers {
        // mx_slow_i64(op, a, _): cvttsd2si with the x86 integer indefinite (op 0: 32 bit, 1: 64 bit)
        let mut c = Code::new();
        let cmp = |c: &mut Code, lo: f64, hi: f64| {
            c.local_get(1);
            c.f64_const(lo);
            c.raw(&[0x66]); // f64.ge
            c.local_get(1);
            c.f64_const(hi);
            c.raw(&[0x63]); // f64.lt
            c.raw(&[0x71]); // i32.and
        };
        c.local_get(0);
        c.if_ty(Ty::I64);
        cmp(&mut c, -9223372036854775808.0, 9223372036854775808.0);
        c.if_ty(Ty::I64);
        c.local_get(1);
        c.raw(&[0xb0]); // i64.trunc_f64_s
        c.else_();
        c.i64_const(i64::MIN);
        c.end();
        c.else_();
        cmp(&mut c, -2147483648.0, 2147483648.0);
        c.if_ty(Ty::I64);
        c.local_get(1);
        c.raw(&[0xaa, 0xad]); // i32.trunc_f64_s, i64.extend_i32_u
        c.else_();
        c.i64_const(0x8000_0000);
        c.end();
        c.end();
        mx_slow_i64 = module.add_func(FuncType { params: vec![Ty::I32, Ty::F64, Ty::F64], results: vec![Ty::I64] }, Body { locals: vec![], code: c });
    }
    // decision 0181: the directed-rounding helpers (hybrid mode)
    if c_mode {
        rcgen::install(&mut module);
    }
    // decision 0285: the shared helper of the typed indirect call: landing / hook / pseudo-VA checks, then the table of adapters (hashed by the target)
    let ut_ft = FuncType { params: vec![Ty::I64; 7], results: vec![Ty::I32, Ty::I64] };
    let mut ti_k = 0u32;
    let ti_helper = if typed {
        let (_lim, landing, hook) = typed_cfg.unwrap();
        ti_k = (8 * fit.len() as u32).max(16).next_power_of_two().trailing_zeros().min(16);
        let ut_idx = module.type_index(ut_ft.clone());
        let mut c = Code::new();
        for a in [landing, hook] {
            c.i32_const(0);
            c.i32_load(a);
            c.if_();
            c.i32_const(0);
            c.i64_const(0);
            c.return_();
            c.end();
        }
        c.local_get(6);
        c.i64_const(32);
        c.i64_shr_u();
        c.i64_eqz();
        c.i32_eqz();
        c.if_();
        c.i32_const(0);
        c.i64_const(0);
        c.return_();
        c.end();
        for l in 0..7 {
            c.local_get(l);
        }
        c.local_get(6);
        c.i32_wrap_i64();
        c.i32_const(text_lo as i32);
        c.i32_sub();
        c.i32_const(0x9E37_79B1u32 as i32);
        c.i32_mul();
        c.i32_const((32 - ti_k) as i32);
        c.i32_shr_u();
        c.call_indirect(ut_idx, 0);
        module.add_func(ut_ft.clone(), Body { locals: vec![], code: c })
    } else {
        0
    };
    let n_helpers = module.funcs.len() as u32;
    let base_idx = module.n_imported_funcs + n_helpers;
    let mut fidx: HashMap<u64, u32> = HashMap::new();
    for (n, &k) in closed.iter().enumerate() {
        fidx.insert(k, n as u32 + base_idx);
    }
    for (&k, &i) in &ext_idx {
        fidx.insert(k, i);
    }
    let cx = Ctx { c_mode, c_imp: &c_imp, c_dispatch, ext: &ext_idx, hybrid_r, sigs, fidx: &fidx, opts: &opts, dry: false, ui_ty, text_lo: text_lo as u32, mulhu, mulhs, import_sites: &import_sites, import_idx: &import_idx, mx_window: c_mode && conf.hybrid_mx.is_some(), mx_slow_i64, census: false, hybrid_x: conf.hybrid_x.unwrap_or(0), xplans: &xplans, mx_addr: mx_cfg.map_or(0, |m| m.0), mx_buf: mx_cfg.map_or(0, |m| m.1), mx_hyb, mx_ld, mx_st, x87_ld, x87_st, progress_addr: conf.hybrid_progress.unwrap_or(0), gs_ld, ti_helper };
    let mut rows: Vec<FnRow> = Vec::new();
    for &k in &closed {
        let f = &funs[&k];
        let mut st = LowerStats::default();
        let body = lower::lower(&f.fun, f.cfg.as_ref().unwrap(), f.fl.as_ref().unwrap(), sigs[&k], &cx, &skip[&k], &mut st);
        match body {
            Ok(b) => {
                let size = b.code.len();
                rep.stats.insns += st.insns;
                rep.stats.hi_checks += st.hi_checks;
                rep.stats.mem_ops += st.mem_ops;
                rep.stats.indirect_calls += st.indirect_calls;
                rep.stats.switches += st.switches;
                rep.stats.mx_checked_ops += st.mx_checked_ops;
                rep.stats.trapped_vex += st.trapped_vex;
                rep.stats.switch_br_table += st.switch_br_table;
                for (m, n) in st.by_mnemonic {
                    *rep.stats.by_mnemonic.entry(m).or_insert(0) += n;
                }
                let idx = module.add_func(sigs[&k].func_type(), b);
                debug_assert_eq!(idx, fidx[&k]);
                if conf.names {
                    module.names.push((idx, format!("f_{k:x}")));
                }
                rows.push(FnRow {
                    va: k,
                    insns: f.fun.nodes.len(),
                    native_bytes: f.fun.bytes(),
                    status: "ok".into(),
                    wasm_bytes: size,
                    sig_in: sigs[&k].inr,
                    sig_out: sigs[&k].out,
                    elided: elided[&k],
                    direct: vec![],
                    callees: vec![],
                    indirect: String::new(),
                });
            }
            Err(e) => return Err(format!("internal: closed function {k:#x} failed in the real lowering: {e}")),
        }
    }
    rep.code_bytes = module.bodies[n_helpers as usize..].iter().map(|b| b.code.len() + 1).sum();
    if typed {
        // decision 0285: one adapter per fitting function (key check, progress count and slice as fle_dispatch does after a lookup that hit, then the typed body), a bucket for the functions that share a slot, `miss` for the empty slots
        let (lim, _landing, _hook) = typed_cfg.unwrap();
        let pa = conf.hybrid_progress.unwrap();
        let n_before = module.bodies.len();
        let miss = {
            let mut c = Code::new();
            c.i32_const(0);
            c.i64_const(0);
            module.add_func(ut_ft.clone(), Body { locals: vec![], code: c })
        };
        let mut adapters: HashMap<u64, u32> = HashMap::new();
        for &k in &fit {
            let sg = sigs[&k];
            let res = sg.results();
            let cl = 7 + res.len() as u32;
            let mut c = Code::new();
            c.local_get(6);
            c.i64_const(k as i64);
            c.i64_ne();
            c.if_();
            c.i32_const(0);
            c.i64_const(0);
            c.return_();
            c.end();
            c.i32_const(0);
            c.i32_const(0);
            c.i64_load(pa);
            c.i64_const(1);
            c.i64_add();
            c.local_tee(cl);
            c.i64_store(pa);
            c.local_get(cl);
            c.i32_const(0);
            c.i64_load(lim);
            c.i64_ge_u();
            c.if_();
            c.call(ti_slice);
            c.end();
            c.i32_const(1);
            c.local_get(0);
            for r in sg.params() {
                match r {
                    1 => c.local_get(1),
                    2 => c.local_get(2),
                    8 => c.local_get(3),
                    9 => c.local_get(4),
                    0 => c.local_get(5),
                    _ => c.i64_const(0),
                }
            }
            c.call(fidx[&k]);
            for j in (0..res.len()).rev() {
                c.local_set(7 + j as u32);
            }
            match res.iter().position(|&r| r == RAX) {
                Some(j) => c.local_get(7 + j as u32),
                None => c.local_get(5),
            }
            let idx = module.add_func(ut_ft.clone(), Body { locals: vec![(res.len() as u32 + 1, Ty::I64)], code: c });
            if conf.names {
                module.names.push((idx, format!("ia_{k:x}")));
            }
            adapters.insert(k, idx);
        }
        let m = 1usize << ti_k;
        let slot = |va: u64| -> usize { (((va as u32).wrapping_sub(text_lo as u32)).wrapping_mul(0x9E37_79B1) >> (32 - ti_k)) as usize };
        let mut slots: Vec<Vec<u64>> = vec![Vec::new(); m];
        for &k in &fit {
            slots[slot(k)].push(k);
        }
        let mut table: Vec<u32> = vec![miss; m];
        let mut n_buckets = 0usize;
        let mut max_bucket = 0usize;
        for (si, ks) in slots.iter().enumerate() {
            match ks.len() {
                0 => {}
                1 => table[si] = adapters[&ks[0]],
                n => {
                    let mut c = Code::new();
                    for &k in ks {
                        for l in 0..7 {
                            c.local_get(l);
                        }
                        c.call(adapters[&k]);
                        c.local_set(8);
                        c.local_set(7);
                        c.local_get(7);
                        c.if_();
                        c.local_get(7);
                        c.local_get(8);
                        c.return_();
                        c.end();
                    }
                    c.i32_const(0);
                    c.i64_const(0);
                    table[si] = module.add_func(ut_ft.clone(), Body { locals: vec![(1, Ty::I32), (1, Ty::I64)], code: c });
                    n_buckets += 1;
                    max_bucket = max_bucket.max(n);
                }
            }
        }
        module.table_size = Some(m as u32);
        module.elems = vec![(0, table)];
        rep.indirect_adapters = fit.len();
        rep.adapter_bytes = module.bodies[n_before..].iter().map(|b| b.code.len() + 1).sum::<usize>() + 3 * m;
        rep.typed_slots = m;
        rep.typed_buckets = n_buckets;
        rep.typed_max_bucket = max_bucket;
    }
    // decision 0285: the report column `indirect` and the counts of the fitting functions (the predicate above); in a build without the typed call the counts are a diagnostic only
    if c_mode {
        let mut fit_bytes = 0usize;
        for r in rows.iter_mut() {
            let why = &fit_map[&r.va];
            if why.is_empty() {
                let sg = sigs[&r.va];
                fit_bytes += 16 + 2 * sg.params().len() + 2 * sg.results().len();
                rep.indirect_fit += 1;
                r.indirect = "fit".to_string();
            } else {
                rep.indirect_nofit += 1;
                for w in why {
                    let key = w.split(' ').take(2).collect::<Vec<_>>().join(" ");
                    *rep.indirect_nofit_reasons.entry(key).or_insert(0) += 1;
                }
                r.indirect = format!("nofit: {}", why.join("; "));
            }
        }
        rep.indirect_fit_adapter_bytes = fit_bytes;
    }
    // adapters of the indirect-call signature for the functions that fit it (reads only rcx, rdx, r8, r9; preserves the callee-saved registers)
    let n_core = module.funcs.len();
    let _ = n_core;
    let mut elems: Vec<(u32, Vec<u32>)> = Vec::new();
    let mut n_adapters = 0;
    for &k in closed.iter().filter(|_| !c_mode) {
        let sg = sigs[&k];
        let (real_in, _, _, _) = liveness(&funs[&k], &effs[&k], sigs, pres, &sol.noreturn, 0, &skip[&k]);
        // the adapter passes rax as well: a function that reads rax only (a partial write such as `sete al` merges into the caller's value) fits
        let fits = ARG_REGS | (1 << RAX);
        if (real_in & !fits != 0 && !conf.adapt_all) || pres[&k] & CALLEE_SAVED != CALLEE_SAVED {
            rep.nonconforming += 1;
            if real_in & !fits != 0 {
                *rep.nonconf_reasons.entry(format!("reads {:#06x}", real_in & !fits)).or_insert(0) += 1;
            }
            if pres[&k] & CALLEE_SAVED != CALLEE_SAVED {
                *rep.nonconf_reasons.entry(format!("does not preserve {:#06x}", !pres[&k] & CALLEE_SAVED)).or_insert(0) += 1;
            }
            continue;
        }
        let res = sg.results();
        let mut c = Code::new();
        c.local_get(0);
        for r in sg.params() {
            match r {
                1 => c.local_get(1),
                2 => c.local_get(2),
                8 => c.local_get(3),
                9 => c.local_get(4),
                0 => c.local_get(5),
                _ => c.i64_const(0),
            }
        }
        c.call(fidx[&k]);
        for j in (0..res.len()).rev() {
            c.local_set(6 + j as u32);
        }
        match res.iter().position(|&r| r == RAX) {
            Some(j) => c.local_get(6 + j as u32),
            None => c.local_get(5),
        }
        let locals = if res.is_empty() { vec![] } else { vec![(res.len() as u32, Ty::I64)] };
        let idx = module.add_func(ui.clone(), Body { locals, code: c });
        if conf.names {
            module.names.push((idx, format!("ia_{k:x}")));
        }
        elems.push(((k - text_lo) as u32, vec![idx]));
        n_adapters += 1;
    }
    // hybrid: an indirect call whose target stays on the C path: rsp, rcx, rdx, r8, r9, rax go to R[], rax comes back
    for &k in ext.iter().filter(|_| !c_mode) {
        let r = hybrid_r as i32;
        let mut c = Code::new();
        for (li, rg) in [(0u32, RSP), (1, 1), (2, 2), (3, 8), (4, 9), (5, RAX)] {
            let o = c.reg_base((r + 8 * rg as i32) as u32);
            c.local_get(li);
            c.i64_store(o);
        }
        c.call(ext_idx[&k]);
        let o = c.reg_base(r as u32);
        c.i64_load(o);
        let idx = module.add_func(ui.clone(), Body { locals: vec![], code: c });
        if conf.names {
            module.names.push((idx, format!("ia_{k:x}")));
        }
        elems.push(((k - text_lo) as u32, vec![idx]));
        n_adapters += 1;
    }
    rep.indirect_adapters += n_adapters;
    rep.adapter_bytes += module.bodies[n_core..].iter().map(|b| b.code.len() + 1).sum::<usize>();
    if !elems.is_empty() {
        module.table_size = Some((text_hi - text_lo) as u32);
        module.elems = elems;
    }
    // test adapters with the universal signature
    let mut exports: Vec<(String, u32)> = Vec::new();
    let n_before_test = module.funcs.len();
    if conf.export.is_some() {
        let u = FuncType { params: vec![Ty::I64; 16], results: vec![Ty::I64; 16] };
        for &k in &closed {
            if !exported(k) {
                continue;
            }
            let sg = sigs[&k];
            let mut c = Code::new();
            c.local_get(RSP as u32);
            for r in sg.params() {
                c.local_get(r as u32);
            }
            c.call(fidx[&k]);
            for r in sg.results().into_iter().rev() {
                c.local_set(r as u32);
            }
            for r in 0..16 {
                c.local_get(r);
            }
            let idx = module.add_func(u.clone(), Body { locals: vec![], code: c });
            if conf.names {
                module.names.push((idx, format!("t_{k:x}")));
            }
            exports.push((format!("t_{k:x}"), idx));
        }
    }
    rep.test_adapter_bytes = module.bodies[n_before_test..].iter().map(|b| b.code.len() + 1).sum();
    // hybrid: `w_<va>` (void, registers in R[]) for the C path to call a compiled function
    if hybrid {
        let n0 = module.funcs.len();
        let r = hybrid_r as i32;
        for &k in &closed {
            let sg = sigs[&k];
            let res = sg.results();
            let mut c = Code::new();
            // the entry rsp is kept in the last local: a stub call inside the function leaves R[4] at the callee's return slot, and the C caller
            // expects R[4] unchanged after the call
            let rsp_local = res.len() as u32;
            // the callee-saved registers in R[] belong to the C caller: a stub call inside this function writes the function's own (temporary) values
            // there, so they are saved here and put back at the end
            // decision 0264 (mt3z): not the callee-saved registers that the function itself leaves modified (`res`): a continuation entry (a shared epilogue, k != 0) pops them from the frame of the function that jumped to it,
            // and that value, not the one it was entered with, is what the lifted C leaves in R[]
            let cs: Vec<usize> = (0..16).filter(|&x| (CALLEE_SAVED & (1 << x)) != 0 && x != RSP && !res.contains(&x)).collect();
            let tls = wasm::TLS_ON.with(|x| x.get()) != 0;
            let tb = res.len() as u32 + 1 + cs.len() as u32;   // decision 0234 (mt3): with --tls-ctx the wrapper keeps `__tls_base` in one more local
            if tls {
                c.tls_base_get();
                c.local_set(tb);
                wasm::TLS_LOCAL.with(|x| x.set(tb));
            }
            for (n, &x) in cs.iter().enumerate() {
                let o = c.reg_base((r + 8 * x as i32) as u32);
                c.i64_load(o);
                c.local_set(rsp_local + 1 + n as u32);
            }
            let o = c.reg_base((r + 8 * RSP as i32) as u32);
            c.i64_load(o);
            c.local_tee(rsp_local);
            for rg in sg.params() {
                let o = c.reg_base((r + 8 * rg as i32) as u32);
                c.i64_load(o);
            }
            c.call(fidx[&k]);
            for j in (0..res.len()).rev() {
                c.local_set(j as u32);
            }
            for (j, &rg) in res.iter().enumerate() {
                let o = c.reg_base((r + 8 * rg as i32) as u32);
                c.local_get(j as u32);
                c.i64_store(o);
            }
            let o = c.reg_base((r + 8 * RSP as i32) as u32);
            c.local_get(rsp_local);
            if sg.rsp != 0 {
                // decision 0261: a function that leaves RSP above its entry value (a shared epilogue): the lifted C's `ret` does not pop, so R[4] at the return is entry + k
                c.i64_const(sg.rsp);
                c.i64_add();
            }
            c.i64_store(o);
            for (n, &x) in cs.iter().enumerate() {
                let o = c.reg_base((r + 8 * x as i32) as u32);
                c.local_get(rsp_local + 1 + n as u32);
                c.i64_store(o);
            }
            let mut locals = vec![(res.len() as u32 + 1 + cs.len() as u32, Ty::I64)];
            if tls {
                locals.push((1, Ty::I32));
                wasm::TLS_LOCAL.with(|x| x.set(u32::MAX));
            }
            let idx = module.add_func(FuncType { params: vec![], results: vec![] }, Body { locals, code: c });
            exports.push((format!("f_{k:x}"), idx));
        }
        rep.wrapper_bytes = module.bodies[n0..].iter().map(|b| b.code.len() + 1).sum();
    }
    for (n, i) in exports {
        module.exports.push((n, 0, i));
    }
    if conf.export_funcs {
        for &k in &closed {
            module.exports.push((format!("f_{k:x}"), 0, fidx[&k]));
        }
    }
    let wasm = module.encode();
    rep.module_bytes = wasm.len();
    let mut all_rows: Vec<FnRow> = Vec::new();
    let mut closed_rows: HashMap<u64, FnRow> = rows.into_iter().map(|r| (r.va, r)).collect();
    for (&k, f) in &funs {
        if let Some(mut r) = closed_rows.remove(&k) {
            if conf.census {
                r.callees = call_targets(&f.fun);
            }
            all_rows.push(r);
        } else {
            let reason = f.err.clone().unwrap_or_else(|| "?".into());
            let c = rep.reasons.entry(first_word(&reason)).or_insert((0, 0));
            c.0 += 1;
            c.1 += f.fun.nodes.len();
            let mut direct = f.direct.clone();
            if direct.is_empty() && !reason.starts_with("calls a function") && !reason.starts_with("tail-jumps") {
                direct.push(reason.clone());
            }
            all_rows.push(FnRow { va: k, insns: f.fun.nodes.len(), native_bytes: f.fun.bytes(), status: reason, direct, callees: if conf.census { call_targets(&f.fun) } else { vec![] }, sig_in: sigs.get(&k).map_or(0, |g| g.inr), sig_out: sigs.get(&k).map_or(0, |g| g.out), ..Default::default() });
        }
    }
    rep.rows = all_rows;
    let fidx_out: BTreeMap<u64, u32> = fidx.into_iter().collect();
    Ok(Compiled { wasm, report: rep, fidx: fidx_out })
}

/// Backward liveness of the registers of one function. Returns the registers live at entry, for each direct call or tail call the registers
/// live after it, and for each indirect call the registers live after it.
///
/// Two sets are kept. `s` is the strict liveness: a register is live when a later instruction reads the value that is in it now. `w` also
/// counts the argument registers (rcx, rdx, r8, r9) of an indirect call, which the unknown callee may or may not read: they have to be
/// passed on when they come from the caller (a forwarding wrapper), but they do not demand an output of a callee that modified them
/// (the value would be garbage) and they are not a value that lives across an indirect call. Entry parameters come from `w`; the demand on
/// callees and the check of values across indirect calls come from `s`.
fn liveness(
    f: &F,
    effs: &HashMap<u64, Eff>,
    sigs: &HashMap<u64, Sig>,
    pres: &HashMap<u64, u16>,
    noreturn: &HashSet<u64>,
    out_f: u16,
    skip: &HashSet<u64>,
) -> (u16, u16, Vec<(u64, u16)>, Vec<u16>) {
    let n = f.fun.blocks.len();
    let cfg = f.cfg.as_ref().unwrap();
    let mut in_s = vec![0u16; n];
    let mut in_w = vec![0u16; n];
    let mut calls: Vec<(u64, u16)> = Vec::new();
    let mut ind: Vec<u16> = Vec::new();
    let sig_of = |g: u64| sigs.get(&g).copied().unwrap_or_default();
    let mod_of = |g: u64| !pres.get(&g).copied().unwrap_or(0) & ALL_REGS;
    let mut changed = true;
    while changed {
        changed = false;
        calls.clear();
        ind.clear();
        for &b in cfg.rpo.iter().rev() {
            let bl = &f.fun.blocks[b];
            let (mut s, mut w) = match &bl.term {
                ir::Term::Ret => (out_f, out_f),
                ir::Term::Tail(g) => {
                    let sg = sig_of(*g);
                    let vs = (out_f & !sg.out) | sg.ins;
                    let vw = (out_f & !sg.out) | sg.inr;
                    calls.push((*g, out_f));
                    if vs != in_s[b] || vw != in_w[b] {
                        in_s[b] = vs;
                        in_w[b] = vw;
                        changed = true;
                    }
                    continue;
                }
                ir::Term::TailImport(_) | ir::Term::TailIndirect => {
                    let tr = if matches!(bl.term, ir::Term::TailIndirect) { effs[bl.insns.last().unwrap()].reads } else { 0 };
                    let vs = (out_f & !(1 << RAX)) | tr;
                    let vw = vs | ARG_REGS | if INDIRECT_READS_RAX.load(std::sync::atomic::Ordering::Relaxed) { 1 << RAX } else { 0 };
                    ind.push(0);
                    if vs != in_s[b] || vw != in_w[b] {
                        in_s[b] = vs;
                        in_w[b] = vw;
                        changed = true;
                    }
                    continue;
                }
                _ => bl.succs.iter().fold((0u16, 0u16), |a, &x| (a.0 | in_s[x], a.1 | in_w[x])),
            };
            for &va in bl.insns.iter().rev() {
                let i = f.fun.insn(va);
                if i.mnemonic() == iced_x86::Mnemonic::Call && ir::is_near(i) {
                    let g = i.near_branch_target();
                    let sg = sig_of(g);
                    if noreturn.contains(&g) {
                        s = sg.ins & ARG_REGS;
                        w = sg.inr & ARG_REGS;
                        continue;
                    }
                    calls.push((g, s));
                    s = (s & !sg.out) | sg.ins;
                    w = (w & !mod_of(g) & !sg.out) | sg.inr;
                    continue;
                }
                if i.mnemonic() == iced_x86::Mnemonic::Call {
                    ind.push(s);
                    let kill = (1 << RAX) | (1 << 10) | (1 << 11) | ARG_REGS;
                    s = (s & !kill) | effs[&va].reads;
                    w = (w & !kill) | effs[&va].reads | ARG_REGS | if INDIRECT_READS_RAX.load(std::sync::atomic::Ordering::Relaxed) { 1 << RAX } else { 0 };
                    continue;
                }
                let mut e = effs[&va];
                if skip.contains(&va) {
                    if i.mnemonic() == iced_x86::Mnemonic::Push {
                        e.reads = 0;
                    } else {
                        e.defs = 0;
                    }
                }
                s = (s & !e.defs) | e.reads;
                w = (w & !e.defs) | e.reads;
            }
            if s != in_s[b] || w != in_w[b] {
                in_s[b] = s;
                in_w[b] = w;
                changed = true;
            }
        }
    }
    (in_w[0], in_s[0], calls, ind)
}

pub fn native_len(data: &[u8]) -> usize {
    data.len()
}

/// High 64 bits of a 64x64-bit product (Hacker's Delight 8-2), unsigned or signed.
fn mulhu_body(signed: bool) -> Body {
    // params: 0 = a, 1 = b; locals: 2 alo, 3 ahi, 4 blo, 5 bhi, 6 t, 7 w1
    let mut c = Code::new();
    let m = 0xffff_ffffi64;
    c.local_get(0);
    c.i64_const(m);
    c.i64_and();
    c.local_set(2);
    c.local_get(0);
    c.i64_const(32);
    c.i64_shr_u();
    c.local_set(3);
    c.local_get(1);
    c.i64_const(m);
    c.i64_and();
    c.local_set(4);
    c.local_get(1);
    c.i64_const(32);
    c.i64_shr_u();
    c.local_set(5);
    // t = alo*blo
    c.local_get(2);
    c.local_get(4);
    c.i64_mul();
    c.local_set(6);
    // t = ahi*blo + (t >> 32)
    c.local_get(3);
    c.local_get(4);
    c.i64_mul();
    c.local_get(6);
    c.i64_const(32);
    c.i64_shr_u();
    c.i64_add();
    c.local_set(6);
    // w1 = t & m ; (w2 = t >> 32 stays in t)
    c.local_get(6);
    c.i64_const(m);
    c.i64_and();
    c.local_set(7);
    c.local_get(6);
    c.i64_const(32);
    c.i64_shr_u();
    c.local_set(6);
    // t3 = alo*bhi + w1
    c.local_get(2);
    c.local_get(5);
    c.i64_mul();
    c.local_get(7);
    c.i64_add();
    c.local_set(7);
    // hi = ahi*bhi + w2 + (t3 >> 32)
    c.local_get(3);
    c.local_get(5);
    c.i64_mul();
    c.local_get(6);
    c.i64_add();
    c.local_get(7);
    c.i64_const(32);
    c.i64_shr_u();
    c.i64_add();
    if signed {
        // hi - ((a >> 63) & b) - ((b >> 63) & a)
        c.local_get(0);
        c.i64_const(63);
        c.i64_shr_s();
        c.local_get(1);
        c.i64_and();
        c.i64_sub();
        c.local_get(1);
        c.i64_const(63);
        c.i64_shr_s();
        c.local_get(0);
        c.i64_and();
        c.i64_sub();
    }
    Body { locals: vec![(6, Ty::I64)], code: c }
}

/// XMM registers are v128 locals of the function and no XMM value crosses a function boundary: none may be live into the function or live
/// across a call (a float argument, a float result, a callee-saved xmm6..15 kept across a call). The C path keeps the XMM file in a global
/// (`X[]`), so those functions need the full interprocedural treatment (parameters/results of type v128 in the signatures) that the
/// prototype does not have.
fn xmm_check(f: &F, effs: &HashMap<u64, Eff>) -> Result<(), String> {
    let n = f.fun.blocks.len();
    let cfg = f.cfg.as_ref().unwrap();
    let mut live_in = vec![0u16; n];
    let mut changed = true;
    let mut bad: Option<String> = None;
    while changed {
        changed = false;
        for &b in cfg.rpo.iter().rev() {
            let bl = &f.fun.blocks[b];
            let mut live = bl.succs.iter().fold(0u16, |a, &x| a | live_in[x]);
            for &va in bl.insns.iter().rev() {
                let i = f.fun.insn(va);
                if i.mnemonic() == iced_x86::Mnemonic::Call && live != 0 {
                    bad = Some(format!("an XMM register is live across a call @ {va:#x}"));
                }
                let e = effs[&va];
                live = (live & !e.xd) | e.xr;
            }
            if live != live_in[b] {
                live_in[b] = live;
                changed = true;
            }
        }
    }
    if live_in[0] != 0 {
        return Err("an XMM register is live into the function".to_string());
    }
    match bad {
        Some(e) => Err(e),
        None => Ok(()),
    }
}
