//! Decision 0270 (mt3w): the status flags that a function returns with, when they are the same on every path that returns.
//!
//! The C path keeps the flags in one global (`FL`), so a caller that reads a flag after `call g` reads whatever `g` left there. A direct-wasm function has no `FL`: its
//! flags are the operands that the defining instruction saved in locals. The caller of a function whose flags it reads therefore cannot be compiled (a `Call` is an `Opaque`
//! definition). The one idiom that does this in the real DLLs is the stack-cookie check of the MSVC runtime (`__security_check_cookie`: `cmp rcx,[cookie]; jne fail; rol rcx,16;
//! test cx,0xffff; jne fail2; ret`): its only `ret` follows the `jne` that was not taken, so ZF = 1 there, and the code that the compiler merged after the call reads it.
//!
//! `summary` runs a forward analysis over the function's blocks with a value per flag: known 0, known 1 or unknown. An instruction that writes a flag makes it unknown, except
//! the writes of known value (`test`/`and`/`or`/`xor`: CF = OF = 0; `xor r,r` and `sub r,r`: ZF = PF = 1, CF = SF = OF = 0; `clc`, `stc`); a `call` makes them unknown, except the
//! flags that the callee returns with (its own summary). The edges of a conditional jump refine the flag that it tests (`je`/`jne`: ZF; `jb`/`jae`: CF; `js`/`jns`: SF; `jo`/`jno`: OF;
//! `jp`/`jnp`: PF; `ja` taken and `jbe` not taken: CF = ZF = 0); an edge that contradicts a known value does not exist. At every `ret` the states are merged: a flag that has the same
//! known value on every `ret` is a known flag of the function.
//!
//! A path that does not reach a `ret` does not count: `ud2`/`int3`/`int n`, a call of a function that cannot return, and a call or jump of an import that the runtime declares
//! `noreturn` (`NORETURN_IMPORTS`: `fle_proc.h`/`fle_thread.h`; the unit test `noreturn_imports_match_the_runtime_headers` keeps the table equal to the headers). For the stack-cookie check this
//! is the failure path: `jmp __report_gsfailure`, which ends in `UnhandledExceptionFilter` and `TerminateProcess`, which stop the run (the runtime does not return from them).
//! A function with an unsupported terminator, a call or jump that is too deep to follow, or no known flag on its `ret`s has no summary (`Out::Unknown`), which is the old behaviour
//! (the reader makes the caller stay on the C path).

use crate::cfg::Cfg;
use crate::fx::*;
use crate::ir::{is_near, Fun, Term};
use iced_x86::{ConditionCode, Instruction, Mnemonic, OpKind};

/// The imports whose implementation in the runtime never returns (`__attribute__((noreturn))` in `lift/runtime/fle_proc.h` and `fle_thread.h`: the run is stopped there).
pub const NORETURN_IMPORTS: &[&str] = &[
    "ExitProcess",
    "TerminateProcess",
    "RaiseException",
    "RtlCaptureContext",
    "RtlLookupFunctionEntry",
    "RtlVirtualUnwind",
    "RtlUnwindEx",
    "RtlPcToFileHeader",
    "UnhandledExceptionFilter",
    "SetUnhandledExceptionFilter",
    "ExitThread",
];

/// `name` as fe-decode names an import (`KERNEL32.dll!ExitProcess`); the runtime resolves an import by the function name alone
pub fn noreturn_import(name: &str) -> bool {
    NORETURN_IMPORTS.contains(&name.rsplit('!').next().unwrap_or(name))
}

/// What a function does to the flags when it returns.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Out {
    /// no summary (an unsupported terminator, a call too deep to follow, or no flag is known on every `ret`)
    Unknown,
    /// no path reaches a `ret`
    Never,
    /// (mask, values) of the flags that every `ret` has with the same known value (bit k of `values` = the value of the flag with bit k of `mask`)
    Flags(u8, u8),
}

/// The callbacks of the analysis: the summary of the function at a code address (a direct `call`, or a `jmp` to its entry), and the name of the import that the instruction at an address
/// calls through its IAT slot.
pub struct Env<'a> {
    pub callee: &'a mut dyn FnMut(u64, &mut Fx) -> Out,
    pub import_at: &'a dyn Fn(u64) -> Option<String>,
}

const UNK: u8 = 2;
type St = [u8; 6];

fn add_ret(at_ret: &mut Option<St>, st: &St) {
    match at_ret {
        None => *at_ret = Some(*st),
        Some(r) => {
            join(r, st);
        }
    }
}

fn idx(flag: u8) -> usize {
    flag.trailing_zeros() as usize
}

fn join(a: &mut St, b: &St) -> bool {
    let mut ch = false;
    for k in 0..6 {
        if a[k] != b[k] && a[k] != UNK {
            a[k] = UNK;
            ch = true;
        }
    }
    ch
}

fn of_out(m: u8, v: u8) -> St {
    let mut o = [UNK; 6];
    for k in 0..6 {
        if m & (1 << k) != 0 {
            o[k] = (v >> k) & 1;
        }
    }
    o
}

fn same_reg_zeroing(i: &Instruction) -> bool {
    matches!(i.mnemonic(), Mnemonic::Xor | Mnemonic::Sub) && i.op_count() == 2 && i.op0_kind() == OpKind::Register && i.op1_kind() == OpKind::Register && i.op0_register() == i.op1_register()
}

/// The effect of one instruction on the state; `false`: the instruction does not return (a call of a function or an import that cannot return).
fn step(st: &mut St, va: u64, i: &Instruction, fx: &mut Fx, env: &mut Env) -> bool {
    if i.mnemonic() == Mnemonic::Call {
        if is_near(i) {
            match (env.callee)(i.near_branch_target(), fx) {
                Out::Never => return false,
                Out::Flags(m, v) => *st = of_out(m, v),
                Out::Unknown => *st = [UNK; 6],
            }
        } else if (env.import_at)(va).map_or(false, |n| noreturn_import(&n)) {
            return false;
        } else {
            *st = [UNK; 6];
        }
        return true;
    }
    let w = fx.eff(i).flags_w;
    if w == 0 {
        return true;
    }
    for k in 0..6 {
        if w & (1 << k) != 0 {
            st[k] = UNK;
        }
    }
    let set = |st: &mut St, flag: u8, v: u8| {
        if w & flag != 0 {
            st[idx(flag)] = v;
        }
    };
    match i.mnemonic() {
        Mnemonic::And | Mnemonic::Or | Mnemonic::Xor | Mnemonic::Test => {
            set(st, CF, 0);
            set(st, OF, 0);
            if same_reg_zeroing(i) {
                set(st, ZF, 1);
                set(st, PF, 1);
                set(st, SF, 0);
            }
        }
        Mnemonic::Sub if same_reg_zeroing(i) => {
            set(st, CF, 0);
            set(st, OF, 0);
            set(st, ZF, 1);
            set(st, PF, 1);
            set(st, SF, 0);
        }
        Mnemonic::Clc => set(st, CF, 0),
        Mnemonic::Stc => set(st, CF, 1),
        _ => {}
    }
    true
}

/// The state on the edge of `jcc cc` that is taken (`taken`) or falls through; `None`: the edge cannot be followed (a known flag contradicts it).
fn refine(mut st: St, cc: ConditionCode, taken: bool) -> Option<St> {
    let put = |flag: u8, v: u8, st: &mut St| -> bool {
        let k = idx(flag);
        if st[k] != UNK && st[k] != v {
            return false;
        }
        st[k] = v;
        true
    };
    let t = taken as u8;
    use ConditionCode as C;
    let ok = match cc {
        C::e => put(ZF, t, &mut st),
        C::ne => put(ZF, 1 - t, &mut st),
        C::b => put(CF, t, &mut st),
        C::ae => put(CF, 1 - t, &mut st),
        C::s => put(SF, t, &mut st),
        C::ns => put(SF, 1 - t, &mut st),
        C::o => put(OF, t, &mut st),
        C::no => put(OF, 1 - t, &mut st),
        C::p => put(PF, t, &mut st),
        C::np => put(PF, 1 - t, &mut st),
        C::a if taken => put(CF, 0, &mut st) && put(ZF, 0, &mut st),
        C::be if !taken => put(CF, 0, &mut st) && put(ZF, 0, &mut st),
        _ => true,
    };
    if ok {
        Some(st)
    } else {
        None
    }
}

pub fn summary(f: &Fun, cfg: &Cfg, fx: &mut Fx, env: &mut Env) -> Out {
    let n = f.blocks.len();
    let mut ins: Vec<Option<St>> = vec![None; n];
    ins[0] = Some([UNK; 6]);
    let mut work: Vec<usize> = cfg.rpo.clone();
    work.reverse();
    let mut on = vec![false; n];
    for &b in &work {
        on[b] = true;
    }
    let mut at_ret: Option<St> = None;
    while let Some(b) = work.pop() {
        on[b] = false;
        let Some(mut st) = ins[b] else { continue };
        let mut live = true;
        for &va in &f.blocks[b].insns {
            if !step(&mut st, va, f.insn(va), fx, env) {
                live = false;
                break;
            }
        }
        if !live {
            continue;
        }
        let mut out: Vec<(usize, St)> = Vec::new();
        match &f.blocks[b].term {
            Term::Jmp(t) => out.push((*t, st)),
            Term::Jcc { cc, taken, fall } => {
                if let Some(s) = refine(st, *cc, true) {
                    out.push((*taken, s));
                }
                if let Some(s) = refine(st, *cc, false) {
                    out.push((*fall, s));
                }
            }
            Term::Jcz { taken, fall, .. } => {
                out.push((*taken, st));
                out.push((*fall, st));
            }
            Term::Switch { cases, .. } => {
                for &(_, t) in cases {
                    out.push((t, st));
                }
            }
            Term::Ret => add_ret(&mut at_ret, &st),
            Term::Trap => {}
            Term::Tail(t) => match (env.callee)(*t, fx) {
                Out::Unknown => return Out::Unknown,
                Out::Never => {}
                Out::Flags(m, v) => add_ret(&mut at_ret, &of_out(m, v)),
            },
            Term::TailImport(name) => {
                if !noreturn_import(name) {
                    return Out::Unknown;
                }
            }
            _ => return Out::Unknown,
        }
        for (s, st) in out {
            let changed = match &mut ins[s] {
                None => {
                    ins[s] = Some(st);
                    true
                }
                Some(cur) => join(cur, &st),
            };
            if changed && !on[s] {
                on[s] = true;
                work.push(s);
            }
        }
    }
    let Some(r) = at_ret else { return Out::Never };
    let (mut mask, mut vals) = (0u8, 0u8);
    for k in 0..6 {
        if r[k] != UNK {
            mask |= 1 << k;
            vals |= r[k] << k;
        }
    }
    if mask == 0 {
        Out::Unknown
    } else {
        Out::Flags(mask, vals)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `NORETURN_IMPORTS` is the list of the imports that the runtime declares `noreturn` in `fle_proc.h` (the kernel32 requests to end the process and the stops by name) and `fle_thread.h` (`ExitThread`):
    /// the list and the headers must agree, so that an import that comes to return is taken out of the list.
    #[test]
    fn noreturn_imports_match_the_runtime_headers() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtime");
        let mut declared: Vec<String> = Vec::new();
        for h in ["fle_proc.h", "fle_thread.h"] {
            let text = std::fs::read_to_string(dir.join(h)).expect(h);
            for l in text.lines() {
                if let Some(rest) = l.strip_prefix("void fle_imp_") {
                    if let Some((name, tail)) = rest.split_once("(void)") {
                        if tail.contains("__attribute__((noreturn))") {
                            declared.push(name.to_string());
                        }
                    }
                }
            }
        }
        declared.sort();
        let mut listed: Vec<String> = NORETURN_IMPORTS.iter().map(|s| s.to_string()).collect();
        listed.sort();
        assert_eq!(declared, listed);
    }
}
