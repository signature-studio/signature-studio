//! x87 arithmetic, loads, stores, compares and constants (decision 0033). The state and
//! the f80 arithmetic are in runtime/fle_x87.c (header fle_x87.h): software extended
//! precision, bit-exact 80-bit registers, TOP and the tag word in `fle_x87`.
//!
//! F2XM1 and FSINCOS are correctly rounded soft implementations (decision 0043).
//!
//! Instruction forms that are not implemented (the transcendental FSIN, FCOS,
//! FPTAN, FPATAN, FYL2X, FYL2XP1, FPREM, FPREM1, FXTRACT and the state save/restore group)
//! return `None`, so they stay `mnemonic outside minimal set` and stop at `fle_unsupported`.

use iced_x86::{Instruction, Mnemonic, OpKind, Register};

type Lift = Result<(&'static str, String), String>;

const ADD: u32 = 0;
const SUB: u32 = 1;
const SUBR: u32 = 2;
const MUL: u32 = 3;
const DIV: u32 = 4;
const DIVR: u32 = 5;

pub fn lift(i: &Instruction) -> Option<Lift> {
    use Mnemonic::*;
    let m = i.mnemonic();
    Some(match m {
        Fld => fld(i),
        Fild => fild(i),
        Fst | Fstp => fst(i, m == Fstp),
        Fist | Fistp | Fisttp => fist(i, m != Fist, m == Fisttp),
        Fadd | Faddp | Fiadd => arith(i, ADD, "fadd", m == Faddp, m == Fiadd),
        Fsub | Fsubp | Fisub => arith(i, SUB, "fsub", m == Fsubp, m == Fisub),
        Fsubr | Fsubrp | Fisubr => arith(i, SUBR, "fsubr", m == Fsubrp, m == Fisubr),
        Fmul | Fmulp | Fimul => arith(i, MUL, "fmul", m == Fmulp, m == Fimul),
        Fdiv | Fdivp | Fidiv => arith(i, DIV, "fdiv", m == Fdivp, m == Fidiv),
        Fdivr | Fdivrp | Fidivr => arith(i, DIVR, "fdivr", m == Fdivrp, m == Fidivr),
        Fcom | Fcomp | Fcompp => com(i, m, false),
        Fucom | Fucomp | Fucompp => com(i, m, true),
        Ficom | Ficomp => ficom(i, m == Ficomp),
        Fcomi | Fcomip | Fucomi | Fucomip => comi(i, matches!(m, Fcomip | Fucomip), matches!(m, Fucomi | Fucomip)),
        Ftst => none(i, "ftst", "fle_x87_ftst();"),
        Fxam => none(i, "fxam", "fle_x87_fxam();"),
        Fxch => fxch(i),
        Fchs => none(i, "fchs", "fle_x87_fchs();"),
        Fabs => none(i, "fabs", "fle_x87_fabs();"),
        Fsqrt => none(i, "fsqrt", "fle_x87_fsqrt();"),
        Frndint => none(i, "frndint", "fle_x87_frndint();"),
        Fscale => none(i, "fscale", "fle_x87_fscale();"),
        F2xm1 => none(i, "f2xm1", "fle_x87_f2xm1();"),
        Fsincos => none(i, "fsincos", "fle_x87_fsincos();"),
        Fld1 => none(i, "fld1", "fle_x87_fld_const(0);"),
        Fldl2t => none(i, "fldl2t", "fle_x87_fld_const(1);"),
        Fldl2e => none(i, "fldl2e", "fle_x87_fld_const(2);"),
        Fldpi => none(i, "fldpi", "fle_x87_fld_const(3);"),
        Fldlg2 => none(i, "fldlg2", "fle_x87_fld_const(4);"),
        Fldln2 => none(i, "fldln2", "fle_x87_fld_const(5);"),
        Fldz => none(i, "fldz", "fle_x87_fld_const(6);"),
        Fcmovb | Fcmovbe | Fcmove | Fcmovnb | Fcmovnbe | Fcmovne | Fcmovnu | Fcmovu => fcmov(i, m),
        _ => return None,
    })
}

/// Prototypes of the runtime functions (fle_x87.h), declared in the block of each instruction
/// that calls them, like x87ctl.rs does, so that the generated C needs no extra include.
const PROTOS: &[(&str, &str)] = &[
    ("fle_x87_fld_m", "void fle_x87_fld_m(uint64_t, unsigned);"),
    ("fle_x87_fild", "void fle_x87_fild(uint64_t, unsigned);"),
    ("fle_x87_fld_st", "void fle_x87_fld_st(unsigned);"),
    ("fle_x87_fld_const", "void fle_x87_fld_const(unsigned);"),
    ("fle_x87_fst_m", "void fle_x87_fst_m(uint64_t, unsigned, unsigned);"),
    ("fle_x87_fst_st", "void fle_x87_fst_st(unsigned, unsigned);"),
    ("fle_x87_fist", "void fle_x87_fist(uint64_t, unsigned, unsigned, unsigned);"),
    ("fle_x87_arith_st", "void fle_x87_arith_st(unsigned, unsigned, unsigned, unsigned);"),
    ("fle_x87_arith_m", "void fle_x87_arith_m(unsigned, uint64_t, unsigned);"),
    ("fle_x87_arith_i", "void fle_x87_arith_i(unsigned, uint64_t, unsigned);"),
    ("fle_x87_com_st", "void fle_x87_com_st(unsigned, unsigned, unsigned);"),
    ("fle_x87_com_m", "void fle_x87_com_m(uint64_t, unsigned, unsigned);"),
    ("fle_x87_com_i", "void fle_x87_com_i(uint64_t, unsigned, unsigned);"),
    ("fle_x87_comi", "unsigned fle_x87_comi(unsigned, unsigned, unsigned);"),
    ("fle_x87_ftst", "void fle_x87_ftst(void);"),
    ("fle_x87_fxam", "void fle_x87_fxam(void);"),
    ("fle_x87_fxch", "void fle_x87_fxch(unsigned);"),
    ("fle_x87_fcmov", "void fle_x87_fcmov(unsigned, unsigned);"),
    ("fle_x87_fchs", "void fle_x87_fchs(void);"),
    ("fle_x87_fabs", "void fle_x87_fabs(void);"),
    ("fle_x87_fsqrt", "void fle_x87_fsqrt(void);"),
    ("fle_x87_frndint", "void fle_x87_frndint(void);"),
    ("fle_x87_fscale", "void fle_x87_fscale(void);"),
    ("fle_x87_f2xm1", "void fle_x87_f2xm1(void);"),
    ("fle_x87_fsincos", "void fle_x87_fsincos(void);"),
];

fn need(c: bool, msg: &str) -> Result<(), String> {
    if c {
        Ok(())
    } else {
        Err(msg.to_string())
    }
}

fn wrap(body: &str) -> String {
    let protos: Vec<&str> = PROTOS.iter().filter(|(n, _)| body.contains(&format!("{n}("))).map(|(_, p)| *p).collect();
    format!("{{ {} {body} }}", protos.join(" "))
}

fn none(i: &Instruction, h: &'static str, body: &str) -> Lift {
    need(i.op_count() == 0, "unexpected operand")?;
    Ok((h, wrap(body)))
}

/// Index of an ST register operand.
fn st_index(r: Register) -> Option<u32> {
    if r.is_st() {
        Some(r as u32 - Register::ST0 as u32)
    } else {
        None
    }
}

fn st_op(i: &Instruction, n: u32) -> Result<u32, String> {
    need(i.op_kind(n) == OpKind::Register, "operand must be an ST register")?;
    st_index(i.op_register(n)).ok_or_else(|| "operand must be an ST register".to_string())
}

fn is_mem(i: &Instruction, n: u32) -> bool {
    i.op_count() > n && i.op_kind(n) == OpKind::Memory
}

/// The memory operand: address expression and size in bytes.
fn mem(i: &Instruction) -> Result<(String, usize), String> {
    need(i.op_count() == 1 && i.op_kind(0) == OpKind::Memory, "operand must be memory")?;
    Ok((super::ea(i)?, i.memory_size().size()))
}

/// The ST(i) of FXCH / FCOM / FUCOM forms. iced lists ST(0) first for the two-operand forms
/// and nothing for the operandless ones (FCOMPP, FUCOMPP use ST(1)).
fn sti_operand(i: &Instruction) -> Result<u32, String> {
    match i.op_count() {
        0 => Ok(1),
        1 => st_op(i, 0),
        2 => {
            need(st_op(i, 0)? == 0, "first operand must be ST(0)")?;
            st_op(i, 1)
        }
        _ => Err("operand count".into()),
    }
}

fn fld(i: &Instruction) -> Lift {
    if is_mem(i, 0) {
        let (a, sz) = mem(i)?;
        need(matches!(sz, 4 | 8 | 10), "fld memory operand size")?;
        return Ok(("fld", wrap(&format!("fle_x87_fld_m({a}, {sz});"))));
    }
    need(i.op_count() == 1, "operand count")?;
    let s = st_op(i, 0)?;
    Ok(("fld", wrap(&format!("fle_x87_fld_st({s});"))))
}

fn fild(i: &Instruction) -> Lift {
    let (a, sz) = mem(i)?;
    need(matches!(sz, 2 | 4 | 8), "fild memory operand size")?;
    Ok(("fild", wrap(&format!("fle_x87_fild({a}, {sz});"))))
}

fn fst(i: &Instruction, pop: bool) -> Lift {
    let p = pop as u32;
    let h = if pop { "fstp" } else { "fst" };
    if is_mem(i, 0) {
        let (a, sz) = mem(i)?;
        need(matches!(sz, 4 | 8) || (pop && sz == 10), "fst memory operand size")?;
        return Ok((h, wrap(&format!("fle_x87_fst_m({a}, {sz}, {p});"))));
    }
    need(i.op_count() == 1, "operand count")?;
    let s = st_op(i, 0)?;
    Ok((h, wrap(&format!("fle_x87_fst_st({s}, {p});"))))
}

fn fist(i: &Instruction, pop: bool, trunc: bool) -> Lift {
    let (a, sz) = mem(i)?;
    need(matches!(sz, 2 | 4 | 8), "fist memory operand size")?;
    need(sz != 8 || pop, "fist m64 does not exist")?;
    let h = if trunc {
        "fisttp"
    } else if pop {
        "fistp"
    } else {
        "fist"
    };
    Ok((h, wrap(&format!("fle_x87_fist({a}, {sz}, {}, {});", pop as u32, trunc as u32))))
}

fn arith(i: &Instruction, op: u32, h: &'static str, pop: bool, int_mem: bool) -> Lift {
    if is_mem(i, 0) {
        need(i.op_count() == 1 && !pop, "memory form with a pop")?;
        let (a, sz) = mem(i)?;
        if int_mem {
            need(matches!(sz, 2 | 4), "integer memory operand size")?;
            return Ok((h, wrap(&format!("fle_x87_arith_i({op}, {a}, {sz});"))));
        }
        need(matches!(sz, 4 | 8), "float memory operand size")?;
        return Ok((h, wrap(&format!("fle_x87_arith_m({op}, {a}, {sz});"))));
    }
    need(!int_mem, "integer form without memory")?;
    // FADD ST(i), ST(0) | FADD ST(0), ST(i) | FADDP ST(i), ST(0) | FADDP (no operand = ST(1), ST(0))
    let (dst, src) = if i.op_count() == 0 {
        need(pop, "operandless form without a pop")?;
        (1, 0)
    } else {
        need(i.op_count() == 2, "operand count")?;
        (st_op(i, 0)?, st_op(i, 1)?)
    };
    need(dst == 0 || src == 0, "one operand must be ST(0)")?;
    Ok((h, wrap(&format!("fle_x87_arith_st({op}, {dst}, {src}, {});", pop as u32))))
}

fn com(i: &Instruction, m: Mnemonic, unordered: bool) -> Lift {
    let pops = match m {
        Mnemonic::Fcompp | Mnemonic::Fucompp => 2,
        Mnemonic::Fcomp | Mnemonic::Fucomp => 1,
        _ => 0,
    };
    let h = if unordered { "fucom" } else { "fcom" };
    let u = unordered as u32;
    if is_mem(i, 0) {
        need(!unordered && pops < 2, "memory form")?;
        let (a, sz) = mem(i)?;
        need(matches!(sz, 4 | 8), "fcom memory operand size")?;
        return Ok((h, wrap(&format!("fle_x87_com_m({a}, {sz}, {pops});"))));
    }
    let s = sti_operand(i)?;
    if pops == 2 {
        need(s == 1, "fcompp operand")?;
    }
    Ok((h, wrap(&format!("fle_x87_com_st({s}, {pops}, {u});"))))
}

fn ficom(i: &Instruction, pop: bool) -> Lift {
    let (a, sz) = mem(i)?;
    need(matches!(sz, 2 | 4), "ficom memory operand size")?;
    Ok(("ficom", wrap(&format!("fle_x87_com_i({a}, {sz}, {});", pop as u32))))
}

fn comi(i: &Instruction, pop: bool, unordered: bool) -> Lift {
    need(i.op_count() == 2, "operand count")?;
    need(st_op(i, 0)? == 0, "destination must be ST(0)")?;
    let s = st_op(i, 1)?;
    let body = format!(
        "unsigned c = fle_x87_comi({s}, {}, {}); FL.cf = c & 1; FL.pf = (c >> 1) & 1; FL.zf = (c >> 2) & 1; FL.af = 0; FL.sf = 0; FL.of = 0;",
        pop as u32, unordered as u32
    );
    Ok(("fcomi", wrap(&body)))
}

fn fxch(i: &Instruction) -> Lift {
    let s = sti_operand(i)?;
    Ok(("fxch", wrap(&format!("fle_x87_fxch({s});"))))
}

fn fcmov(i: &Instruction, m: Mnemonic) -> Lift {
    need(i.op_count() == 2, "operand count")?;
    need(st_op(i, 0)? == 0, "destination must be ST(0)")?;
    let s = st_op(i, 1)?;
    use Mnemonic::*;
    let cond = match m {
        Fcmovb => "fle_rd(FL.cf)",
        Fcmovnb => "!fle_rd(FL.cf)",
        Fcmove => "fle_rd(FL.zf)",
        Fcmovne => "!fle_rd(FL.zf)",
        Fcmovbe => "(fle_rd(FL.cf) || fle_rd(FL.zf))",
        Fcmovnbe => "!(fle_rd(FL.cf) || fle_rd(FL.zf))",
        Fcmovu => "fle_rd(FL.pf)",
        _ => "!fle_rd(FL.pf)",
    };
    Ok(("fcmov", wrap(&format!("fle_x87_fcmov({s}, (unsigned)({cond}));"))))
}
