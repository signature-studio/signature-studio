//! x87 in the direct wasm of the hybrid (decision 0259, mt3x). The x87 state (the 8 registers fle_x87_r[], TOP, the status, control and tag words, the exception flags) and all of the arithmetic (software 80-bit extended
//! precision, runtime/fle_x87.c) stay in the C path. An x87 instruction in a function of the direct wasm is a CALL of the same runtime function that the lifted C calls for it (fe-lift/src/x87.rs), with the same constant
//! arguments and, for a memory operand, the guest address; nothing is computed in wasm. The result is therefore the C path's result by construction (bit for bit, flags and exceptions included), and the state is one
//! state: a direct function and a C-path function that follow each other see the same stack, tag word and status word. No f64 anywhere.
//! Not here: FNSTCW / FLDCW (the control word: decision 0254), FCOMI / FUCOMI / FCMOV (they read or write EFLAGS: the flag recipes of flags.rs would need a new definition kind), the transcendental instructions the runtime does not
//! implement (FSIN, FCOS, FPTAN, FPATAN, FYL2X, FYL2XP1, FPREM, FPREM1, FXTRACT) and the state save/restore group. Those functions stay on the C path as before.
//! The module is a child of lower.rs (it uses the lowering state L); the hook in lower.rs is one match arm, and lib.rs imports the helpers a module uses (`Plan::helper`) next to the other C imports.

use super::{L, R};
use crate::wasm::Ty;
use iced_x86::{Instruction, Mnemonic, OpKind, Register};

/// The runtime functions of fle_x87.h / fle_x87ctl.h that the direct wasm calls: name, parameter types (unsigned = i32, uint64_t = i64), result types.
pub(crate) const HELPERS: &[(&str, &[Ty], &[Ty])] = &[
    ("fle_x87_fld_m", &[Ty::I64, Ty::I32], &[]),
    ("fle_x87_fild", &[Ty::I64, Ty::I32], &[]),
    ("fle_x87_fld_st", &[Ty::I32], &[]),
    ("fle_x87_fld_const", &[Ty::I32], &[]),
    ("fle_x87_fst_m", &[Ty::I64, Ty::I32, Ty::I32], &[]),
    ("fle_x87_fst_st", &[Ty::I32, Ty::I32], &[]),
    ("fle_x87_fist", &[Ty::I64, Ty::I32, Ty::I32, Ty::I32], &[]),
    ("fle_x87_arith_st", &[Ty::I32, Ty::I32, Ty::I32, Ty::I32], &[]),
    ("fle_x87_arith_m", &[Ty::I32, Ty::I64, Ty::I32], &[]),
    ("fle_x87_arith_i", &[Ty::I32, Ty::I64, Ty::I32], &[]),
    ("fle_x87_com_st", &[Ty::I32, Ty::I32, Ty::I32], &[]),
    ("fle_x87_com_m", &[Ty::I64, Ty::I32, Ty::I32], &[]),
    ("fle_x87_com_i", &[Ty::I64, Ty::I32, Ty::I32], &[]),
    ("fle_x87_ftst", &[], &[]),
    ("fle_x87_fxam", &[], &[]),
    ("fle_x87_fxch", &[Ty::I32], &[]),
    ("fle_x87_fchs", &[], &[]),
    ("fle_x87_fabs", &[], &[]),
    ("fle_x87_fsqrt", &[], &[]),
    ("fle_x87_frndint", &[], &[]),
    ("fle_x87_fscale", &[], &[]),
    ("fle_x87_f2xm1", &[], &[]),
    ("fle_x87_fsincos", &[], &[]),
    ("fle_x87_sw", &[], &[Ty::I32]),
    ("fle_x87_store_sw", &[Ty::I64], &[]),
    ("fle_x87_clex", &[], &[]),
    ("fle_x87_init", &[], &[]),
];

const ADD: u32 = 0;
const SUB: u32 = 1;
const SUBR: u32 = 2;
const MUL: u32 = 3;
const DIV: u32 = 4;
const DIVR: u32 = 5;

/// one argument of a helper call: a constant (unsigned), or the guest address of the instruction's memory operand (uint64_t)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum A {
    C(u32),
    Addr,
}

/// what the call leaves on the stack
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Yields {
    Nothing,
    /// the 16 bits of the status word, for the low 16 bits of RAX (FNSTSW AX)
    Ax,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Plan {
    pub helper: &'static str,
    pub args: Vec<A>,
    pub ret: Yields,
}

/// the mnemonics this module takes over in the C-import mode
pub(crate) fn is_x87(m: Mnemonic) -> bool {
    use Mnemonic::*;
    matches!(
        m,
        Fld | Fild | Fst | Fstp | Fist | Fistp | Fisttp | Fadd | Faddp | Fiadd | Fsub | Fsubp | Fisub | Fsubr | Fsubrp | Fisubr | Fmul | Fmulp | Fimul | Fdiv | Fdivp | Fidiv | Fdivr | Fdivrp | Fidivr
            | Fcom | Fcomp | Fcompp | Fucom | Fucomp | Fucompp | Ficom | Ficomp | Ftst | Fxam | Fxch | Fchs | Fabs | Fsqrt | Frndint | Fscale | F2xm1 | Fsincos | Fld1 | Fldl2t | Fldl2e | Fldpi | Fldlg2 | Fldln2 | Fldz
            | Fnstsw | Fnclex | Fninit | Wait
    )
}

fn need(c: bool, msg: &str) -> R<()> {
    if c {
        Ok(())
    } else {
        Err(msg.to_string())
    }
}

fn st_op(i: &Instruction, n: u32) -> R<u32> {
    need(i.op_kind(n) == OpKind::Register, "x87: operand must be an ST register")?;
    let r = i.op_register(n);
    if r.is_st() {
        Ok(r as u32 - Register::ST0 as u32)
    } else {
        Err("x87: operand must be an ST register".into())
    }
}

fn is_mem(i: &Instruction, n: u32) -> bool {
    i.op_count() > n && i.op_kind(n) == OpKind::Memory
}

fn mem(i: &Instruction) -> R<usize> {
    need(i.op_count() == 1 && i.op_kind(0) == OpKind::Memory, "x87: operand must be memory")?;
    Ok(i.memory_size().size())
}

/// the ST(i) of the FXCH / FCOM / FUCOM forms: iced lists ST(0) first for the two-operand forms and nothing for the operandless ones (FCOMPP, FUCOMPP use ST(1))
fn sti_operand(i: &Instruction) -> R<u32> {
    match i.op_count() {
        0 => Ok(1),
        1 => st_op(i, 0),
        2 => {
            need(st_op(i, 0)? == 0, "x87: first operand must be ST(0)")?;
            st_op(i, 1)
        }
        _ => Err("x87: operand count".into()),
    }
}

fn plan1(helper: &'static str, args: Vec<A>) -> Plan {
    Plan { helper, args, ret: Yields::Nothing }
}

fn arith(i: &Instruction, op: u32, pop: bool, int_mem: bool) -> R<Plan> {
    if is_mem(i, 0) {
        need(i.op_count() == 1 && !pop, "x87: memory form with a pop")?;
        let sz = mem(i)? as u32;
        if int_mem {
            need(matches!(sz, 2 | 4), "x87: integer memory operand size")?;
            return Ok(plan1("fle_x87_arith_i", vec![A::C(op), A::Addr, A::C(sz)]));
        }
        need(matches!(sz, 4 | 8), "x87: float memory operand size")?;
        return Ok(plan1("fle_x87_arith_m", vec![A::C(op), A::Addr, A::C(sz)]));
    }
    need(!int_mem, "x87: integer form without memory")?;
    // FADD ST(i), ST(0) | FADD ST(0), ST(i) | FADDP ST(i), ST(0) | FADDP (no operand = ST(1), ST(0))
    let (dst, src) = if i.op_count() == 0 {
        need(pop, "x87: operandless form without a pop")?;
        (1, 0)
    } else {
        need(i.op_count() == 2, "x87: operand count")?;
        (st_op(i, 0)?, st_op(i, 1)?)
    };
    need(dst == 0 || src == 0, "x87: one operand must be ST(0)")?;
    Ok(plan1("fle_x87_arith_st", vec![A::C(op), A::C(dst), A::C(src), A::C(pop as u32)]))
}

/// The helper call of an x87 instruction, with the arguments the lifted C passes (fe-lift/src/x87.rs). Err: the form is not one of the supported ones and the function stays on the C path.
pub(crate) fn plan(i: &Instruction) -> R<Plan> {
    use Mnemonic::*;
    let m = i.mnemonic();
    let none = |h: &'static str| -> R<Plan> {
        need(i.op_count() == 0, "x87: unexpected operand")?;
        Ok(plan1(h, vec![]))
    };
    match m {
        Fld => {
            if is_mem(i, 0) {
                let sz = mem(i)? as u32;
                need(matches!(sz, 4 | 8 | 10), "x87: fld memory operand size")?;
                return Ok(plan1("fle_x87_fld_m", vec![A::Addr, A::C(sz)]));
            }
            need(i.op_count() == 1, "x87: operand count")?;
            Ok(plan1("fle_x87_fld_st", vec![A::C(st_op(i, 0)?)]))
        }
        Fild => {
            let sz = mem(i)? as u32;
            need(matches!(sz, 2 | 4 | 8), "x87: fild memory operand size")?;
            Ok(plan1("fle_x87_fild", vec![A::Addr, A::C(sz)]))
        }
        Fst | Fstp => {
            let pop = (m == Fstp) as u32;
            if is_mem(i, 0) {
                let sz = mem(i)? as u32;
                need(matches!(sz, 4 | 8) || (pop == 1 && sz == 10), "x87: fst memory operand size")?;
                return Ok(plan1("fle_x87_fst_m", vec![A::Addr, A::C(sz), A::C(pop)]));
            }
            need(i.op_count() == 1, "x87: operand count")?;
            Ok(plan1("fle_x87_fst_st", vec![A::C(st_op(i, 0)?), A::C(pop)]))
        }
        Fist | Fistp | Fisttp => {
            let (pop, trunc) = (m != Fist, m == Fisttp);
            let sz = mem(i)? as u32;
            need(matches!(sz, 2 | 4 | 8), "x87: fist memory operand size")?;
            need(sz != 8 || pop, "x87: fist m64 does not exist")?;
            Ok(plan1("fle_x87_fist", vec![A::Addr, A::C(sz), A::C(pop as u32), A::C(trunc as u32)]))
        }
        Fadd | Faddp | Fiadd => arith(i, ADD, m == Faddp, m == Fiadd),
        Fsub | Fsubp | Fisub => arith(i, SUB, m == Fsubp, m == Fisub),
        Fsubr | Fsubrp | Fisubr => arith(i, SUBR, m == Fsubrp, m == Fisubr),
        Fmul | Fmulp | Fimul => arith(i, MUL, m == Fmulp, m == Fimul),
        Fdiv | Fdivp | Fidiv => arith(i, DIV, m == Fdivp, m == Fidiv),
        Fdivr | Fdivrp | Fidivr => arith(i, DIVR, m == Fdivrp, m == Fidivr),
        Fcom | Fcomp | Fcompp | Fucom | Fucomp | Fucompp => {
            let unordered = matches!(m, Fucom | Fucomp | Fucompp);
            let pops = match m {
                Fcompp | Fucompp => 2,
                Fcomp | Fucomp => 1,
                _ => 0,
            };
            if is_mem(i, 0) {
                need(!unordered && pops < 2, "x87: memory form")?;
                let sz = mem(i)? as u32;
                need(matches!(sz, 4 | 8), "x87: fcom memory operand size")?;
                return Ok(plan1("fle_x87_com_m", vec![A::Addr, A::C(sz), A::C(pops)]));
            }
            let s = sti_operand(i)?;
            if pops == 2 {
                need(s == 1, "x87: fcompp operand")?;
            }
            Ok(plan1("fle_x87_com_st", vec![A::C(s), A::C(pops), A::C(unordered as u32)]))
        }
        Ficom | Ficomp => {
            let sz = mem(i)? as u32;
            need(matches!(sz, 2 | 4), "x87: ficom memory operand size")?;
            Ok(plan1("fle_x87_com_i", vec![A::Addr, A::C(sz), A::C((m == Ficomp) as u32)]))
        }
        Ftst => none("fle_x87_ftst"),
        Fxam => none("fle_x87_fxam"),
        Fxch => Ok(plan1("fle_x87_fxch", vec![A::C(sti_operand(i)?)])),
        Fchs => none("fle_x87_fchs"),
        Fabs => none("fle_x87_fabs"),
        Fsqrt => none("fle_x87_fsqrt"),
        Frndint => none("fle_x87_frndint"),
        Fscale => none("fle_x87_fscale"),
        F2xm1 => none("fle_x87_f2xm1"),
        Fsincos => none("fle_x87_fsincos"),
        Fld1 | Fldl2t | Fldl2e | Fldpi | Fldlg2 | Fldln2 | Fldz => {
            need(i.op_count() == 0, "x87: unexpected operand")?;
            let k = match m {
                Fld1 => 0,
                Fldl2t => 1,
                Fldl2e => 2,
                Fldpi => 3,
                Fldlg2 => 4,
                Fldln2 => 5,
                _ => 6,
            };
            Ok(plan1("fle_x87_fld_const", vec![A::C(k)]))
        }
        Fnstsw => {
            need(i.op_count() == 1, "x87: operand count")?;
            if i.op_kind(0) == OpKind::Register {
                need(i.op_register(0) == Register::AX, "x87: FNSTSW into a register other than AX")?;
                return Ok(Plan { helper: "fle_x87_sw", args: vec![], ret: Yields::Ax });
            }
            need(is_mem(i, 0) && mem(i)? == 2, "x87: FNSTSW memory operand size")?;
            Ok(plan1("fle_x87_store_sw", vec![A::Addr]))
        }
        Fnclex => none("fle_x87_clex"),
        Fninit => none("fle_x87_init"),
        _ => Err(format!("mnemonic {m:?}")),
    }
}

impl<'a> L<'a> {
    /// The lowering of an x87 instruction in the C-import mode: the arguments, then the call. FWAIT is nothing (as in the lifted C).
    pub(super) fn x87_insn(&mut self, i: &Instruction) -> R<()> {
        if i.mnemonic() == Mnemonic::Wait {
            return Ok(());
        }
        let p = plan(i)?;
        let idx = *self.cx.c_imp.get(p.helper).ok_or_else(|| format!("x87 helper {} is not imported", p.helper))?;
        // the guest address first (it may need temporaries and the stack pointer), then the call's arguments in order
        let addr = if p.args.contains(&A::Addr) {
            let off = self.mem_addr(i)?;
            self.c.i32_const(off as i32);
            self.c.i32_add();
            self.c.i64_extend_i32_u();
            let t = self.tmp(Ty::I64);
            self.c.local_set(t);
            Some(t)
        } else {
            None
        };
        for a in &p.args {
            match *a {
                A::C(v) => self.c.i32_const(v as i32),
                A::Addr => self.c.local_get(addr.expect("an address argument has its temporary")),
            }
        }
        self.c.call(idx);
        if p.ret == Yields::Ax {
            self.c.i32_const(0xffff);
            self.c.i32_and();
            self.set_reg(Register::AX)?;
        }
        Ok(())
    }
}

/// the helper names a module needs for the x87 instructions among `insns` (the real lowering registers exactly these as C imports)
pub(crate) fn helpers_of<'i>(insns: impl Iterator<Item = &'i Instruction>) -> std::collections::BTreeSet<&'static str> {
    insns.filter(|i| is_x87(i.mnemonic())).filter_map(|i| plan(i).ok()).map(|p| p.helper).collect()
}

/// the (parameter, result) types of a helper
pub(crate) fn signature(name: &str) -> Option<(&'static [Ty], &'static [Ty])> {
    HELPERS.iter().find(|(n, _, _)| *n == name).map(|&(_, p, r)| (p, r))
}

/// The call that the direct wasm makes for the first instruction of `bytes`, as text for the tests: "fle_x87_fld_m(ADDR,8)" (a uint64_t parameter is the guest address of the memory operand and is written ADDR), plus "->AX" when
/// the result goes to AX. Ok("") for FWAIT. Err: the direct wasm does not take the instruction.
pub fn call_text(bytes: &[u8]) -> Result<String, String> {
    let mut d = iced_x86::Decoder::with_ip(64, bytes, 0x401000, iced_x86::DecoderOptions::NONE);
    let i = d.decode();
    if i.is_invalid() {
        return Err("undecodable".into());
    }
    if !is_x87(i.mnemonic()) {
        return Err(format!("not an x87 mnemonic: {:?}", i.mnemonic()));
    }
    if i.mnemonic() == Mnemonic::Wait {
        return Ok(String::new());
    }
    let p = plan(&i)?;
    let (params, _) = signature(p.helper).expect("a planned helper has a signature");
    let args: Vec<String> = p.args.iter().zip(params.iter()).map(|(a, t)| match (a, t) {
        (A::Addr, Ty::I64) => "ADDR".to_string(),
        (A::C(v), Ty::I32) => v.to_string(),
        _ => "MISMATCH".to_string(),
    }).collect();
    assert_eq!(args.len(), params.len(), "the arguments of {} are its parameters", p.helper);
    Ok(format!("{}({}){}", p.helper, args.join(","), if p.ret == Yields::Ax { "->AX" } else { "" }))
}

/// the names of all helpers (for the tests)
pub fn helper_names() -> Vec<&'static str> {
    HELPERS.iter().map(|(n, _, _)| *n).collect()
}
