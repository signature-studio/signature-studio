//! Lowering of one function to wasm. Registers are locals, flags are recipes (see `flags`), memory is the imported linear memory at the
//! guest address (identity map, as in the C path), control flow is `block`/`loop`/`if`/`br`/`br_table` (see `cfg`).

use crate::cfg::Cfg;
use crate::flags::{self, opsize0, Def, FlagInfo, Kind, Src};
use crate::fx::*;
use crate::ir::{Fun, Term};
use crate::wasm::{Body, Code, Ty};
use iced_x86::{ConditionCode, Instruction, Mnemonic, OpKind, Register};
use std::collections::{HashMap, HashSet};

// decision 0259: x87 in the direct wasm = calls of the C runtime's own functions (a child module: it works on the lowering state `L`)
#[path = "x87.rs"]
pub mod x87;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Sig {
    /// Registers passed (RSP is always the first parameter and is not in the mask).
    pub inr: u16,
    /// Of `inr`, the registers whose value the function really needs (the rest is only forwarded to an indirect callee that may ignore it).
    pub ins: u16,
    /// Registers returned.
    pub out: u16,
    /// decision 0261: RSP after the function returns, relative to its value at entry (the return address slot): 0 for an ordinary function; k for a shared epilogue (`add rsp, k-8; pop; ret`) that a `jmp`
    /// reaches with the frame of another function. A caller adds k to its own RSP after the call (lifted C: `R[4] += 8` after the callee's `return`, so the C path needs nothing).
    pub rsp: i64,
}

impl Sig {
    pub fn params(&self) -> Vec<usize> {
        (0..16).filter(|&r| r != RSP && self.inr & (1 << r) != 0).collect()
    }
    pub fn results(&self) -> Vec<usize> {
        (0..16).filter(|&r| r != RSP && self.out & (1 << r) != 0).collect()
    }
    pub fn func_type(&self) -> crate::wasm::FuncType {
        let mut params = vec![Ty::I64];
        params.extend(self.params().iter().map(|_| Ty::I64));
        crate::wasm::FuncType { params, results: self.results().iter().map(|_| Ty::I64).collect() }
    }
}

pub struct Opts {
    /// Address of the thread environment block (GS-relative accesses).
    pub teb: Option<u32>,
}

pub struct Ctx<'a> {
    /// Hybrid mode with the C runtime (`--c-imports`): imports and indirect calls go to the C path (`c.fle_imp_<name>`, `c.fle_hyb_dispatch`) through R[]; `c_imp` maps an
    /// import name (without the DLL) to the index of its function import (in a dry run only the keys matter).
    pub c_mode: bool,
    pub c_imp: &'a HashMap<String, u32>,
    pub c_dispatch: u32,
    /// Hybrid mode: functions that stay on the C path (VA -> index of the import `cf_<va>`) and the address of the register array R[] of the C path.
    pub ext: &'a HashMap<u64, u32>,
    pub hybrid_r: u32,
    pub sigs: &'a HashMap<u64, Sig>,
    pub fidx: &'a HashMap<u64, u32>,
    pub opts: &'a Opts,
    /// Dry run: calls need no signature (the callees are recorded in the stats).
    pub dry: bool,
    /// Type index of the indirect-call signature (rsp, rcx, rdx, r8, r9) -> rax, and the first address of the code (table index = target - text_lo).
    pub ui_ty: u32,
    pub text_lo: u32,
    /// Function indices of the helpers `mulhu64(a, b) -> hi` and `mulhs64(a, b) -> hi`.
    pub mulhu: u32,
    pub mulhs: u32,
    /// Call sites of imported functions (instruction VA -> name) and the function index of each import `imp_<name>`.
    pub import_sites: &'a HashMap<u64, String>,
    pub import_idx: &'a HashMap<String, u32>,
    /// Scalar-double operations with the MXCSR window (fplow1's rule): the operands are normal numbers in a window, MXCSR is in the default
    /// state (the global `mx_ok`), else `mx_slow_f64(op, a, b)` (the conventional code). `mx_slow_f64` and `mx_slow_i64` are import indices.
    pub mx_window: bool,
    pub mx_slow_i64: u32,
    /// Census: keep going after an unsupported instruction and count it (dry run only).
    pub census: bool,
    /// Hybrid: address of X[] (0 = XMM registers do not cross function boundaries) and the exchange plan of each function (see `xabi`).
    pub hybrid_x: u32,
    pub xplans: &'a HashMap<u64, crate::xabi::XPlan>,
    /// Hybrid with the MXCSR window: address of the C path's `fle_mxcsr` and of the scratch buffer, and the function indices of `c.fle_hyb_mx(kind, imm)`, `c.fle_mxcsr_load(addr)`,
    /// `c.fle_mxcsr_store(addr)` (0 = not hybrid: MXCSR is the module's own global).
    pub mx_addr: u32,
    pub mx_buf: u32,
    pub mx_hyb: u32,
    pub mx_ld: u32,
    pub mx_st: u32,
    /// Hybrid: function indices of `c.fle_x87_load_cw(addr)` / `c.fle_x87_store_cw(addr)` (the control word is state of the C path: fle_x87.c); 0 = the module imports none
    pub x87_ld: u32,
    pub x87_st: u32,
    /// address of the crossing counters (0 = none)
    pub progress_addr: u32,
    /// Hybrid: function index of `c.fle_hyb_gs_ld8(off) -> i64` (the C path's answer for `mov r64, gs:[off]`; 0 = the TEB is read as memory at `opts.teb`)
    pub gs_ld: u32,
    /// Decision 0285: function index of the shared helper of the typed indirect call (rsp, rcx, rdx, r8, r9, rax, target) -> (done, rax); 0 = the module has none (the dispatch of the C path is the only way)
    pub ti_helper: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Frame {
    Trap,
    Block(usize),
    Loop(usize),
    If,
}

fn ty_of(size: usize) -> Ty {
    if size >= 8 {
        Ty::I64
    } else {
        Ty::I32
    }
}

fn mask_of(size: usize) -> u64 {
    match size {
        1 => 0xff,
        2 => 0xffff,
        4 => 0xffff_ffff,
        _ => u64::MAX,
    }
}

struct L<'a> {
    f: &'a Fun,
    cfg: &'a Cfg,
    fl: &'a FlagInfo,
    sig: Sig,
    cx: &'a Ctx<'a>,
    skip: &'a HashSet<u64>,
    c: Code,
    nparams: u32,
    locals: Vec<Ty>,
    reg_local: [u32; 16],
    xmm_local: [u32; 16],
    frames: Vec<Frame>,
    tmp_used: Vec<(Ty, u32)>,
    tmp_free: Vec<(Ty, u32)>,
    def_loc: HashMap<u32, (u32, u32)>,
    /// decision 0264: the slot of each flag definition (`locshare::assign`; empty = every definition owns its pair) and the pair of locals of each slot
    slot_of: Vec<u32>,
    slot_loc: HashMap<u32, (u32, u32)>,
    def_cin: HashMap<u32, u32>,
    mat_loc: [u32; 6],
    pending_rsp: i64,
    cur_va: u64,
    xp: Option<&'a crate::xabi::XPlan>,
    xsite: Option<(u16, u16)>,
    mx_guard: u32,
    /// decision 0181 (--mx-rc): RC (0..3) of the C path's MXCSR when PE is set and PM masked, else 4; NONE when the option is off
    mx_rcl: u32,
    mx_bypass: bool,
    src_ov: Option<u32>,
    fall: Option<usize>,
    /// Dispatch-loop mode (irreducible functions): the local that holds the next block, and the number of blocks.
    dispatch_pc: Option<u32>,
    stats: &'a mut LowerStats,
}

#[derive(Default, Debug, Clone)]
pub struct LowerStats {
    pub insns: usize,
    pub hi_checks: usize,
    pub mem_ops: usize,
    pub by_mnemonic: std::collections::BTreeMap<String, usize>,
    pub callees: Vec<u64>,
    pub indirect_calls: usize,
    pub import_calls: usize,
    pub switches: usize,
    pub switch_br_table: usize,
    pub dispatch_functions: usize,
    pub mx_checked_ops: usize,
    pub trapped_vex: usize,
    pub insn_errors: std::collections::BTreeMap<String, usize>,
}

type R<T> = Result<T, String>;

/// Test option (--mx-force-slow): every MXCSR window check fails, so the C path's own function runs for every floating-point instruction.
pub static MX_FORCE_SLOW: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Instruction forms whose window rule needs the guard (RC = 0, PE and PM set).
fn mx_guarded(m: Mnemonic) -> bool {
    use Mnemonic as M;
    matches!(m, M::Addss | M::Addsd | M::Subss | M::Subsd | M::Mulss | M::Mulsd | M::Divss | M::Divsd | M::Sqrtss | M::Sqrtsd | M::Sqrtpd | M::Cvtsd2ss | M::Cvtpd2ps | M::Cvttpd2dq | M::Cvtpd2dq | M::Cvttsd2si | M::Cvttss2si | M::Cvtsd2si | M::Cvtss2si)
        || matches!(m, M::Addps | M::Subps | M::Mulps | M::Addpd | M::Subpd | M::Mulpd | M::Cvttps2dq | M::Cvtps2dq | M::Roundss | M::Roundsd)
}

/// decision 0181 (--mx-packed): the SSE forms of `sse_x2`
fn is_sse_x2(m: Mnemonic) -> bool {
    use Mnemonic as M;
    matches!(
        m,
        M::Andnps | M::Andnpd | M::Pandn | M::Unpcklps | M::Unpckhps | M::Unpcklpd | M::Unpckhpd | M::Movlhps | M::Movhlps | M::Movlps | M::Movlpd | M::Movhps | M::Movhpd | M::Shufps | M::Shufpd
            | M::Pshufd | M::Pshuflw | M::Pshufhw | M::Punpckldq | M::Punpcklwd | M::Punpckhdq | M::Punpckhwd | M::Punpckhbw | M::Pcmpeqb | M::Pcmpeqw | M::Pcmpeqd | M::Pcmpgtb | M::Pcmpgtw
            | M::Pcmpgtd | M::Psrlw | M::Psraw | M::Psllw | M::Psrld | M::Psrad | M::Pslld | M::Packuswb | M::Packsswb | M::Packssdw | M::Pmuludq | M::Movmskps | M::Movmskpd | M::Pmovmskb
            | M::Pextrw
            | M::Psrlq | M::Psllq | M::Paddusb | M::Paddusw | M::Psubusb | M::Psubusw | M::Paddsb | M::Paddsw | M::Psubsb | M::Psubsw | M::Pmullw | M::Pmulhw | M::Pmulhuw | M::Pmaddwd
            | M::Pavgb | M::Pavgw | M::Pminub | M::Pmaxub | M::Pminsw | M::Pmaxsw | M::Psadbw | M::Pinsrw | M::Pshufb | M::Palignr | M::Pabsb | M::Pabsw | M::Pabsd
            | M::Movsldup | M::Movshdup
    )
}

fn splat_f64(v: f64) -> [u8; 16] {
    let mut b = [0u8; 16];
    b[..8].copy_from_slice(&v.to_le_bytes());
    b[8..].copy_from_slice(&v.to_le_bytes());
    b
}

const NONE: u32 = u32::MAX;

impl<'a> L<'a> {
    fn new_local(&mut self, t: Ty) -> u32 {
        self.locals.push(t);
        self.nparams + self.locals.len() as u32 - 1
    }

    fn tmp(&mut self, t: Ty) -> u32 {
        if let Some(p) = self.tmp_free.iter().position(|x| x.0 == t) {
            let x = self.tmp_free.swap_remove(p);
            self.tmp_used.push(x);
            return x.1;
        }
        let l = self.new_local(t);
        self.tmp_used.push((t, l));
        l
    }

    fn release_tmps(&mut self) {
        self.tmp_free.append(&mut self.tmp_used);
    }

    fn reg_loc(&mut self, r: usize) -> u32 {
        if self.reg_local[r] == NONE {
            self.reg_local[r] = self.new_local(Ty::I64);
        }
        self.reg_local[r]
    }

    fn xmm_loc(&mut self, r: usize) -> u32 {
        if self.xmm_local[r] == NONE {
            self.locals.push(Ty::V128);
            self.xmm_local[r] = self.nparams + self.locals.len() as u32 - 1;
        }
        self.xmm_local[r]
    }

    fn flush_rsp(&mut self) {
        if self.pending_rsp != 0 {
            let l = self.reg_loc(RSP);
            self.c.local_get(l);
            self.c.i64_const(self.pending_rsp);
            self.c.i64_add();
            self.c.local_set(l);
            self.pending_rsp = 0;
        }
    }

    /// Pushes the value of a general register at `size` bytes (i32 for 1/2/4, i64 for 8).
    fn get_reg(&mut self, r: Register) -> R<()> {
        let n = gpr(r).ok_or("not a GPR")?;
        if n == RSP {
            self.flush_rsp();
        }
        let l = self.reg_loc(n);
        self.c.local_get(l);
        if matches!(r, Register::AH | Register::CH | Register::DH | Register::BH) {
            self.c.i64_const(8);
            self.c.i64_shr_u();
            self.c.i32_wrap_i64();
            self.c.i32_const(0xff);
            self.c.i32_and();
            return Ok(());
        }
        match r.size() {
            8 => {}
            4 => self.c.i32_wrap_i64(),
            2 => {
                self.c.i32_wrap_i64();
                self.c.i32_const(0xffff);
                self.c.i32_and();
            }
            _ => {
                self.c.i32_wrap_i64();
                self.c.i32_const(0xff);
                self.c.i32_and();
            }
        }
        Ok(())
    }

    /// Writes the value on the stack (type by the register size) to a general register with the x86-64 rules (32-bit writes zero-extend,
    /// 8/16-bit writes merge).
    fn set_reg(&mut self, r: Register) -> R<()> {
        let n = gpr(r).ok_or("not a GPR")?;
        if n == RSP {
            self.flush_rsp();
        }
        let l = self.reg_loc(n);
        if matches!(r, Register::AH | Register::CH | Register::DH | Register::BH) {
            self.c.i64_extend_i32_u();
            self.c.i64_const(8);
            self.c.i64_shl();
            self.c.i64_const(0xff00);
            self.c.i64_and();
            self.c.local_get(l);
            self.c.i64_const(!0xff00i64);
            self.c.i64_and();
            self.c.i64_or();
            self.c.local_set(l);
            return Ok(());
        }
        match r.size() {
            8 => self.c.local_set(l),
            4 => {
                self.c.i64_extend_i32_u();
                self.c.local_set(l);
            }
            sz => {
                let m = mask_of(sz);
                self.c.i64_extend_i32_u();
                self.c.local_get(l);
                self.c.i64_const(!(m as i64));
                self.c.i64_and();
                self.c.i64_or();
                self.c.local_set(l);
            }
        }
        Ok(())
    }

    /// Branch to the trap when the i32 on the stack is not zero.
    fn trap_if(&mut self) {
        let d = self.trap_depth();
        self.c.br_if(d);
    }

    fn trap_depth(&self) -> u32 {
        (self.frames.len() - 1) as u32
    }

    /// After an i64 effective address is in local `a`: traps when bits 63:32 are not zero.
    fn hi_check(&mut self, a: u32) {
        self.stats.hi_checks += 1;
        self.c.local_get(a);
        self.c.i64_const(32);
        self.c.i64_shr_u();
        self.c.i32_wrap_i64();
        self.trap_if();
    }

    /// Pushes the i32 guest address of the memory operand and returns the constant offset for the memarg.
    fn mem_addr(&mut self, i: &Instruction) -> R<u32> {
        self.stats.mem_ops += 1;
        if i.memory_segment() == Register::GS && i.memory_base() == Register::None && i.memory_index() == Register::None {
            if let Some(t) = self.cx.opts.teb {
                let d = i.memory_displacement64();
                self.c.i32_const((t as u64).wrapping_add(d) as u32 as i32);
                return Ok(0);
            }
        }
        if matches!(i.memory_segment(), Register::FS | Register::GS) {
            return Err("FS/GS segment".into());
        }
        if i.is_ip_rel_memory_operand() {
            let a = i.ip_rel_memory_address();
            if a >> 32 != 0 {
                return Err("rip-relative address above 4 GiB".into());
            }
            self.c.i32_const(a as u32 as i32);
            return Ok(0);
        }
        let base = i.memory_base();
        let idx = i.memory_index();
        if (base != Register::None && base.size() != 8) || (idx != Register::None && idx.size() != 8) {
            return Err("32-bit addressing".into());
        }
        let disp = i.memory_displacement64() as i64;
        if base == Register::None && idx == Register::None {
            if (disp as u64) >> 32 != 0 {
                return Err("absolute address above 4 GiB".into());
            }
            self.c.i32_const(disp as u32 as i32);
            return Ok(0);
        }
        if idx == Register::None {
            let bn = gpr(base).ok_or("base is not a GPR")?;
            if bn == RSP {
                self.flush_rsp();
            }
            let l = self.reg_loc(bn);
            // bits 63:32 of every address that is not rsp-based are checked over the whole effective address: the C path's `fle_addr`
            let hi = bn != RSP;
            if hi && disp != 0 {
                let t = self.tmp(Ty::I64);
                self.c.local_get(l);
                self.c.i64_const(disp);
                self.c.i64_add();
                self.c.local_tee(t);
                self.c.i64_const(32);
                self.c.i64_shr_u();
                self.c.i32_wrap_i64();
                self.stats.hi_checks += 1;
                self.trap_if();
                self.c.local_get(t);
                self.c.i32_wrap_i64();
                return Ok(0);
            }
            if hi {
                self.hi_check(l);
            }
            self.c.local_get(l);
            self.c.i32_wrap_i64();
            if (0..(1i64 << 31)).contains(&disp) {
                return Ok(disp as u32);
            }
            self.c.i32_const(disp as i32);
            self.c.i32_add();
            return Ok(0);
        }
        // base + index*scale + disp in 64 bits, then the check on the sum
        let mut have = false;
        if base != Register::None {
            let bn = gpr(base).ok_or("base is not a GPR")?;
            if bn == RSP {
                self.flush_rsp();
            }
            let l = self.reg_loc(bn);
            self.c.local_get(l);
            have = true;
        }
        let inn = gpr(idx).ok_or("index is not a GPR")?;
        if inn == RSP {
            return Err("rsp as index".into());
        }
        let il = self.reg_loc(inn);
        self.c.local_get(il);
        let sc = i.memory_index_scale();
        if sc > 1 {
            self.c.i64_const(sc.trailing_zeros() as i64);
            self.c.i64_shl();
        }
        if have {
            self.c.i64_add();
        }
        if disp != 0 {
            self.c.i64_const(disp);
            self.c.i64_add();
        }
        let t = self.tmp(Ty::I64);
        self.c.local_tee(t);
        self.c.i64_const(32);
        self.c.i64_shr_u();
        self.c.i32_wrap_i64();
        self.stats.hi_checks += 1;
        self.trap_if();
        self.c.local_get(t);
        self.c.i32_wrap_i64();
        Ok(0)
    }

    /// i64 effective address (for lea), unchecked.
    fn lea_addr(&mut self, i: &Instruction) -> R<()> {
        if matches!(i.memory_segment(), Register::FS | Register::GS) {
            return Err("FS/GS segment".into());
        }
        if i.is_ip_rel_memory_operand() {
            self.c.i64_const(i.ip_rel_memory_address() as i64);
            return Ok(());
        }
        let base = i.memory_base();
        let idx = i.memory_index();
        if (base != Register::None && base.size() != 8) || (idx != Register::None && idx.size() != 8) {
            return Err("32-bit addressing".into());
        }
        let disp = i.memory_displacement64() as i64;
        let mut have = false;
        if base != Register::None {
            self.get_reg(base)?;
            have = true;
        }
        if idx != Register::None {
            self.get_reg(idx)?;
            let sc = i.memory_index_scale();
            if sc > 1 {
                self.c.i64_const(sc.trailing_zeros() as i64);
                self.c.i64_shl();
            }
            if have {
                self.c.i64_add();
            }
            have = true;
        }
        if disp != 0 || !have {
            self.c.i64_const(disp);
            if have {
                self.c.i64_add();
            }
        }
        Ok(())
    }

    fn load(&mut self, size: usize, off: u32) {
        match size {
            8 => self.c.i64_load(off),
            4 => self.c.i32_load(off),
            2 => self.c.i32_load16_u(off),
            _ => self.c.i32_load8_u(off),
        }
    }

    fn store(&mut self, size: usize, off: u32) {
        match size {
            8 => self.c.i64_store(off),
            4 => self.c.i32_store(off),
            2 => self.c.i32_store16(off),
            _ => self.c.i32_store8(off),
        }
    }

    /// Pushes operand `n` as a value of `size` bytes.
    fn get_op(&mut self, i: &Instruction, n: u32, size: usize) -> R<()> {
        match i.op_kind(n) {
            OpKind::Register => self.get_reg(i.op_register(n)),
            OpKind::Memory => {
                let off = self.mem_addr(i)?;
                self.load(size, off);
                Ok(())
            }
            OpKind::Immediate8 | OpKind::Immediate8to16 | OpKind::Immediate8to32 | OpKind::Immediate8to64 | OpKind::Immediate16 | OpKind::Immediate32
            | OpKind::Immediate32to64 | OpKind::Immediate64 => {
                let v = i.immediate(n);
                if size >= 8 {
                    self.c.i64_const(v as i64);
                } else {
                    self.c.i32_const((v & mask_of(size)) as u32 as i32);
                }
                Ok(())
            }
            _ => Err("operand kind".into()),
        }
    }

    fn mask_small(&mut self, size: usize) {
        if size == 1 {
            self.c.i32_const(0xff);
            self.c.i32_and();
        } else if size == 2 {
            self.c.i32_const(0xffff);
            self.c.i32_and();
        }
    }

    // ---- arithmetic helpers on the typed value stack ----
    fn op_add(&mut self, size: usize) {
        if size == 8 {
            self.c.i64_add()
        } else {
            self.c.i32_add()
        }
    }
    fn op_sub(&mut self, size: usize) {
        if size == 8 {
            self.c.i64_sub()
        } else {
            self.c.i32_sub()
        }
    }
    fn const_t(&mut self, size: usize, v: i64) {
        if size == 8 {
            self.c.i64_const(v)
        } else {
            self.c.i32_const(v as i32)
        }
    }
    fn eqz_t(&mut self, size: usize) {
        if size == 8 {
            self.c.i64_eqz()
        } else {
            self.c.i32_eqz()
        }
    }
    /// stack: value of type T -> bit (8*size-1) as i32 0/1
    fn sign_bit(&mut self, size: usize) {
        match size {
            8 => {
                self.c.i64_const(63);
                self.c.i64_shr_u();
                self.c.i32_wrap_i64();
            }
            4 => {
                self.c.i32_const(31);
                self.c.i32_shr_u();
            }
            s => {
                self.c.i32_const((s * 8 - 1) as i32);
                self.c.i32_shr_u();
                self.c.i32_const(1);
                self.c.i32_and();
            }
        }
    }
    /// stack: value of type T -> value sign-extended to its width in i32/i64 (for signed compares of 8/16 bits)
    fn sext_t(&mut self, size: usize) {
        match size {
            1 => self.c.i32_extend8_s(),
            2 => self.c.i32_extend16_s(),
            _ => {}
        }
    }

    // ---- flags ----
    fn def_locals(&mut self, id: u32) -> (u32, u32) {
        if let Some(&x) = self.def_loc.get(&id) {
            return x;
        }
        let w = self.fl.defs[id as usize].w as usize;
        let t = if self.fl.defs[id as usize].kind == Kind::BitTest { Ty::I32 } else { ty_of(w) };
        let s = self.slot_of.get(id as usize).copied().unwrap_or(crate::locshare::NO_SLOT);
        let (a, b) = match self.slot_loc.get(&s) {
            Some(&x) if s != crate::locshare::NO_SLOT => x,
            _ => {
                let a = self.new_local(t);
                let b = self.new_local(t);
                if s != crate::locshare::NO_SLOT {
                    self.slot_loc.insert(s, (a, b));
                }
                (a, b)
            }
        };
        self.def_loc.insert(id, (a, b));
        (a, b)
    }

    fn cin_local(&mut self, id: u32) -> u32 {
        if let Some(&l) = self.def_cin.get(&id) {
            return l;
        }
        let l = self.new_local(Ty::I32);
        self.def_cin.insert(id, l);
        l
    }

    fn mat_local(&mut self, flag: u8) -> u32 {
        let k = flag_index(flag);
        if self.mat_loc[k] == NONE {
            self.mat_loc[k] = self.new_local(Ty::I32);
        }
        self.mat_loc[k]
    }

    /// Pushes the result r (masked to the width) of the definition from its saved operands.
    fn push_r(&mut self, d: &Def, sa: u32, sb: u32, sc: u32) {
        let w = d.w as usize;
        match d.kind {
            Kind::Sub => {
                self.c.local_get(sa);
                self.c.local_get(sb);
                self.op_sub(w);
                self.mask_small(w);
            }
            Kind::Add => {
                self.c.local_get(sa);
                self.c.local_get(sb);
                self.op_add(w);
                self.mask_small(w);
            }
            Kind::Adc | Kind::Sbb => {
                let ci = sc;
                self.c.local_get(sa);
                self.c.local_get(sb);
                if d.kind == Kind::Adc {
                    self.op_add(w);
                } else {
                    self.op_sub(w);
                }
                self.c.local_get(ci);
                if w == 8 {
                    self.c.i64_extend_i32_u();
                }
                if d.kind == Kind::Adc {
                    self.op_add(w);
                } else {
                    self.op_sub(w);
                }
                self.mask_small(w);
            }
            Kind::Logic => self.c.local_get(sa),
            Kind::Inc => {
                self.c.local_get(sa);
                self.const_t(w, 1);
                self.op_add(w);
                self.mask_small(w);
            }
            Kind::Dec => {
                self.c.local_get(sa);
                self.const_t(w, 1);
                self.op_sub(w);
                self.mask_small(w);
            }
            Kind::Neg => {
                self.const_t(w, 0);
                self.c.local_get(sb);
                self.op_sub(w);
                self.mask_small(w);
            }
            Kind::Shl(_) | Kind::Shr(_) | Kind::Sar(_) | Kind::ShlV | Kind::ShrV | Kind::SarV => self.c.local_get(sb),
            Kind::BitTest | Kind::BitScan | Kind::FCmp | Kind::Dead | Kind::Opaque | Kind::Known(..) => unreachable!(),
        }
    }

    fn parity_of_r(&mut self, d: &Def, sa: u32, sb: u32, sc: u32) {
        self.push_r(d, sa, sb, sc);
        if d.w == 8 {
            self.c.i32_wrap_i64();
        }
        self.c.i32_const(0xff);
        self.c.i32_and();
        self.c.i32_popcnt();
        self.c.i32_const(1);
        self.c.i32_and();
        self.c.i32_eqz();
    }

    /// Pushes the flag (0/1) from a definition's saved operands.
    fn flag_expr(&mut self, id: u32, flag: u8) -> R<()> {
        let d = self.fl.defs[id as usize].clone();
        if let Kind::Known(_, v) = d.kind {
            // decision 0270 (mt3w): the flags a callee returns with are known (flagout.rs): no saved operands
            self.c.i32_const(((v & flag) != 0) as i32);
            return Ok(());
        }
        let (sa, sb) = self.def_locals(id);
        let sc = if matches!(d.kind, Kind::Adc | Kind::Sbb | Kind::ShlV | Kind::ShrV | Kind::SarV) { self.cin_local(id) } else { NONE };
        let w = d.w as usize;
        if d.kind == Kind::Dead {
            self.c.i32_const(0);
            return Ok(());
        }
        if d.kind == Kind::BitScan {
            // decision 0270 (mt3w): bsf/bsr define ZF only (the source was zero)
            if flag != ZF {
                return Err("flag recipe".into());
            }
            self.c.local_get(sa);
            self.eqz_t(w);
            return Ok(());
        }
        if d.kind == Kind::FCmp {
            // unordered = either operand is a NaN
            let dbl = w == 8;
            let unordered = |s: &mut Self| {
                for l in [sa, sb] {
                    s.c.local_get(l);
                    if dbl { s.c.raw(&[0xbf]) } else { s.c.raw(&[0xbe]) } // f64/f32.reinterpret
                    s.c.local_get(l);
                    if dbl { s.c.raw(&[0xbf]) } else { s.c.raw(&[0xbe]) }
                    if dbl { s.c.raw(&[0x62]) } else { s.c.raw(&[0x5c]) } // f64.ne / f32.ne
                }
                s.c.i32_or();
            };
            match flag {
                PF => unordered(self),
                ZF | CF => {
                    unordered(self);
                    self.c.local_get(sa);
                    if dbl { self.c.raw(&[0xbf]) } else { self.c.raw(&[0xbe]) }
                    self.c.local_get(sb);
                    if dbl { self.c.raw(&[0xbf]) } else { self.c.raw(&[0xbe]) }
                    // ZF: a == b, CF: a < b
                    match (flag, dbl) {
                        (ZF, true) => self.c.raw(&[0x61]),
                        (ZF, false) => self.c.raw(&[0x5b]),
                        (_, true) => self.c.raw(&[0x63]),
                        (_, false) => self.c.raw(&[0x5d]),
                    }
                    self.c.i32_or();
                }
                SF | OF => self.c.i32_const(0),
                _ => return Err("flag recipe".into()),
            }
            return Ok(());
        }
        let bits = (w * 8) as i64;
        match flag {
            ZF => {
                self.push_r(&d, sa, sb, sc);
                self.eqz_t(w);
            }
            SF => {
                self.push_r(&d, sa, sb, sc);
                self.sign_bit(w);
            }
            PF => self.parity_of_r(&d, sa, sb, sc),
            CF => match d.kind {
                Kind::Sub => {
                    self.c.local_get(sa);
                    self.c.local_get(sb);
                    if w == 8 {
                        self.c.i64_lt_u()
                    } else {
                        self.c.i32_lt_u()
                    }
                }
                Kind::Add => {
                    // carry out: the sum is below an operand (mod 2^w)
                    self.push_r(&d, sa, sb, sc);
                    self.c.local_get(sa);
                    if w == 8 {
                        self.c.i64_lt_u()
                    } else {
                        self.c.i32_lt_u()
                    }
                }
                Kind::Sbb => {
                    // a < b, or a == b with a borrow in
                    self.c.local_get(sa);
                    self.c.local_get(sb);
                    if w == 8 { self.c.i64_lt_u() } else { self.c.i32_lt_u() }
                    self.c.local_get(sa);
                    self.c.local_get(sb);
                    if w == 8 { self.c.i64_eq() } else { self.c.i32_eq() }
                    self.c.local_get(sc);
                    self.c.i32_and();
                    self.c.i32_or();
                }
                Kind::Adc => {
                    if w >= 4 {
                        // r < a, or r == a with a carry in
                        self.push_r(&d, sa, sb, sc);
                        self.c.local_get(sa);
                        if w == 8 { self.c.i64_lt_u() } else { self.c.i32_lt_u() }
                        self.push_r(&d, sa, sb, sc);
                        self.c.local_get(sa);
                        if w == 8 { self.c.i64_eq() } else { self.c.i32_eq() }
                        self.c.local_get(sc);
                        self.c.i32_and();
                        self.c.i32_or();
                    } else {
                        self.c.local_get(sa);
                        self.c.local_get(sb);
                        self.c.i32_add();
                        self.c.local_get(sc);
                        self.c.i32_add();
                        self.c.i32_const(bits as i32);
                        self.c.i32_shr_u();
                        self.c.i32_const(1);
                        self.c.i32_and();
                    }
                }
                Kind::Logic => self.c.i32_const(0),
                Kind::BitTest => self.c.local_get(sa),
                Kind::Neg => {
                    self.c.local_get(sb);
                    self.eqz_t(w);
                    self.c.i32_eqz();
                }
                Kind::Shl(c) => {
                    self.c.local_get(sa);
                    self.const_t(w, bits - c as i64);
                    if w == 8 {
                        self.c.i64_shr_u();
                        self.c.i64_const(1);
                        self.c.i64_and();
                        self.c.i32_wrap_i64();
                    } else {
                        self.c.i32_shr_u();
                        self.c.i32_const(1);
                        self.c.i32_and();
                    }
                }
                Kind::Shr(c) | Kind::Sar(c) => {
                    self.c.local_get(sa);
                    self.const_t(w, c as i64 - 1);
                    if w == 8 {
                        self.c.i64_shr_u();
                        self.c.i64_const(1);
                        self.c.i64_and();
                        self.c.i32_wrap_i64();
                    } else {
                        self.c.i32_shr_u();
                        self.c.i32_const(1);
                        self.c.i32_and();
                    }
                }
                Kind::ShlV | Kind::ShrV | Kind::SarV => {
                    // decision 0270: the last bit shifted out; the count (1..width here: the lowering of the shift stops for a larger one) is the saved `sc`
                    let (v, wide) = (d.kind, w == 8);
                    self.c.local_get(sa);
                    if v == Kind::ShlV {
                        self.c.i32_const(bits as i32);
                        self.c.local_get(sc);
                        self.c.i32_sub();
                    } else {
                        self.c.local_get(sc);
                        self.c.i32_const(1);
                        self.c.i32_sub();
                    }
                    if wide {
                        self.c.i64_extend_i32_u();
                        self.c.i64_shr_u();
                        self.c.i64_const(1);
                        self.c.i64_and();
                        self.c.i32_wrap_i64();
                    } else {
                        self.c.i32_shr_u();
                        self.c.i32_const(1);
                        self.c.i32_and();
                    }
                }
                _ => return Err("CF recipe".into()),
            },
            OF => match d.kind {
                Kind::Logic => self.c.i32_const(0),
                Kind::Sub | Kind::Neg | Kind::Dec | Kind::Sbb => {
                    // ((a ^ b) & (a ^ r)) >> (bits-1)
                    let (a_is_zero, b_is_one) = (d.kind == Kind::Neg, d.kind == Kind::Dec);
                    let geta = |s: &mut Self| {
                        if a_is_zero {
                            s.const_t(w, 0)
                        } else {
                            s.c.local_get(sa)
                        }
                    };
                    let getb = |s: &mut Self| {
                        if b_is_one {
                            s.const_t(w, 1)
                        } else if d.kind == Kind::Neg {
                            s.c.local_get(sb)
                        } else {
                            s.c.local_get(sb)
                        }
                    };
                    geta(self);
                    getb(self);
                    self.xor_t(w);
                    geta(self);
                    self.push_r(&d, sa, sb, sc);
                    self.xor_t(w);
                    self.and_t(w);
                    self.sign_bit(w);
                }
                Kind::Add | Kind::Inc | Kind::Adc => {
                    let b_is_one = d.kind == Kind::Inc;
                    // ((a ^ r) & (b ^ r)) >> (bits-1)
                    self.c.local_get(sa);
                    self.push_r(&d, sa, sb, sc);
                    self.xor_t(w);
                    if b_is_one {
                        self.const_t(w, 1);
                    } else {
                        self.c.local_get(sb);
                    }
                    self.push_r(&d, sa, sb, sc);
                    self.xor_t(w);
                    self.and_t(w);
                    self.sign_bit(w);
                }
                Kind::Shl(1) => {
                    // SF(r) xor CF
                    self.flag_expr(id, SF)?;
                    self.flag_expr(id, CF)?;
                    self.c.i32_xor();
                }
                Kind::Shr(1) => {
                    self.c.local_get(sa);
                    self.sign_bit(w);
                }
                Kind::Sar(1) => self.c.i32_const(0),
                _ => return Err("OF recipe".into()),
            },
            _ => return Err("flag recipe".into()),
        }
        Ok(())
    }

    fn xor_t(&mut self, w: usize) {
        if w == 8 {
            self.c.i64_xor()
        } else {
            self.c.i32_xor()
        }
    }
    fn and_t(&mut self, w: usize) {
        if w == 8 {
            self.c.i64_and()
        } else {
            self.c.i32_and()
        }
    }

    fn flag_from(&mut self, src: Src, flag: u8) -> R<()> {
        match src {
            Src::Def(d) => self.flag_expr(d, flag),
            Src::Mat => {
                let l = self.mat_local(flag);
                self.c.local_get(l);
                Ok(())
            }
        }
    }

    /// Pushes the condition as an i32 0/1.
    fn cond(&mut self, va: u64, cc: ConditionCode) -> R<()> {
        let srcs = *self.fl.reader.get(&va).ok_or("condition without flag source")?;
        let need = cc_flags(cc);
        // one definition for all flags: direct forms
        let mut single: Option<u32> = Option::None;
        let mut same = true;
        for k in 0..6 {
            if need & (1 << k) != 0 {
                match srcs[k] {
                    Some(Src::Def(d)) => {
                        if single.map_or(false, |x| x != d) {
                            same = false;
                        }
                        single = Some(d);
                    }
                    _ => same = false,
                }
            }
        }
        if same {
            if let Some(d) = single {
                if self.direct_cond(d, cc)? {
                    return Ok(());
                }
            }
        }
        let src = |f: u8| srcs[flag_index(f)].ok_or_else(|| "flag source missing".to_string());
        use ConditionCode::*;
        match cc {
            e => self.flag_from(src(ZF)?, ZF)?,
            ne => {
                self.flag_from(src(ZF)?, ZF)?;
                self.c.i32_eqz();
            }
            b => self.flag_from(src(CF)?, CF)?,
            ae => {
                self.flag_from(src(CF)?, CF)?;
                self.c.i32_eqz();
            }
            be | a => {
                self.flag_from(src(CF)?, CF)?;
                self.flag_from(src(ZF)?, ZF)?;
                self.c.i32_or();
                if cc == a {
                    self.c.i32_eqz();
                }
            }
            s => self.flag_from(src(SF)?, SF)?,
            ns => {
                self.flag_from(src(SF)?, SF)?;
                self.c.i32_eqz();
            }
            o => self.flag_from(src(OF)?, OF)?,
            no => {
                self.flag_from(src(OF)?, OF)?;
                self.c.i32_eqz();
            }
            p => self.flag_from(src(PF)?, PF)?,
            np => {
                self.flag_from(src(PF)?, PF)?;
                self.c.i32_eqz();
            }
            l | ge => {
                self.flag_from(src(SF)?, SF)?;
                self.flag_from(src(OF)?, OF)?;
                if cc == l {
                    self.c.i32_xor();
                } else {
                    self.c.i32_eq();
                }
            }
            le | g => {
                self.flag_from(src(SF)?, SF)?;
                self.flag_from(src(OF)?, OF)?;
                self.c.i32_xor();
                self.flag_from(src(ZF)?, ZF)?;
                self.c.i32_or();
                if cc == g {
                    self.c.i32_eqz();
                }
            }
            None => return Err("no condition".into()),
        }
        Ok(())
    }

    /// Direct forms for compare-like definitions. Returns false when the kind/condition has no direct form.
    fn direct_cond(&mut self, id: u32, cc: ConditionCode) -> R<bool> {
        let d = self.fl.defs[id as usize].clone();
        if matches!(d.kind, Kind::Known(..)) {
            return Ok(false);
        }
        let (sa, sb) = self.def_locals(id);
        let w = d.w as usize;
        use ConditionCode::*;
        match d.kind {
            Kind::FCmp => {
                let dbl = w == 8;
                // a: CF = 0 and ZF = 0 -> a > b; ae: CF = 0 -> a >= b; b: not ae; be: not a  (an unordered pair has CF = ZF = 1)
                let (op_gt, op_ge): (u8, u8) = if dbl { (0x64, 0x66) } else { (0x5e, 0x60) };
                let (op, neg) = match cc {
                    a => (op_gt, false),
                    ae => (op_ge, false),
                    b => (op_ge, true),
                    be => (op_gt, true),
                    _ => return Ok(false),
                };
                self.c.local_get(sa);
                if dbl { self.c.raw(&[0xbf]) } else { self.c.raw(&[0xbe]) }
                self.c.local_get(sb);
                if dbl { self.c.raw(&[0xbf]) } else { self.c.raw(&[0xbe]) }
                self.c.raw(&[op]);
                if neg {
                    self.c.i32_eqz();
                }
                Ok(true)
            }
            Kind::Sub => {
                let signed = matches!(cc, l | ge | le | g);
                let unsigned = matches!(cc, b | ae | be | a | e | ne);
                if !(signed || unsigned) {
                    return Ok(false);
                }
                self.c.local_get(sa);
                if signed {
                    self.sext_t(w);
                }
                self.c.local_get(sb);
                if signed {
                    self.sext_t(w);
                }
                let c = &mut self.c;
                if w == 8 {
                    match cc {
                        e => c.i64_eq(),
                        ne => c.i64_ne(),
                        b => c.i64_lt_u(),
                        ae => c.i64_ge_u(),
                        be => c.i64_le_u(),
                        a => c.i64_gt_u(),
                        l => c.i64_lt_s(),
                        ge => c.i64_ge_s(),
                        le => c.i64_le_s(),
                        _ => c.i64_gt_s(),
                    }
                } else {
                    match cc {
                        e => c.i32_eq(),
                        ne => c.i32_ne(),
                        b => c.i32_lt_u(),
                        ae => c.i32_ge_u(),
                        be => c.i32_le_u(),
                        a => c.i32_gt_u(),
                        l => c.i32_lt_s(),
                        ge => c.i32_ge_s(),
                        le => c.i32_le_s(),
                        _ => c.i32_gt_s(),
                    }
                }
                Ok(true)
            }
            Kind::Logic => {
                match cc {
                    e => {
                        self.c.local_get(sa);
                        self.eqz_t(w);
                    }
                    ne => {
                        self.c.local_get(sa);
                        self.eqz_t(w);
                        self.c.i32_eqz();
                    }
                    s | l => {
                        self.c.local_get(sa);
                        self.sign_bit(w);
                    }
                    ns | ge => {
                        self.c.local_get(sa);
                        self.sign_bit(w);
                        self.c.i32_eqz();
                    }
                    b | o => self.c.i32_const(0),
                    ae | no => self.c.i32_const(1),
                    be => {
                        self.c.local_get(sa);
                        self.eqz_t(w);
                    }
                    a => {
                        self.c.local_get(sa);
                        self.eqz_t(w);
                        self.c.i32_eqz();
                    }
                    le => {
                        self.c.local_get(sa);
                        self.eqz_t(w);
                        self.c.local_get(sa);
                        self.sign_bit(w);
                        self.c.i32_or();
                    }
                    g => {
                        self.c.local_get(sa);
                        self.eqz_t(w);
                        self.c.local_get(sa);
                        self.sign_bit(w);
                        self.c.i32_or();
                        self.c.i32_eqz();
                    }
                    _ => return Ok(false),
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    // ---- instructions ----

    /// Stores the XMM locals in `mask` to X[] (the C path's XMM file) / loads them from it.
    fn xflush(&mut self, mask: u16) {
        for r in 0..16usize {
            if mask & (1 << r) != 0 {
                let l = self.xmm_loc(r);
                let o = self.c.reg_base(self.cx.hybrid_x + 16 * r as u32);
                self.c.local_get(l);
                self.c.v128_store(o);
            }
        }
    }

    fn xreload(&mut self, mask: u16) {
        for r in 0..16usize {
            if mask & (1 << r) != 0 {
                let l = self.xmm_loc(r);
                let o = self.c.reg_base(self.cx.hybrid_x + 16 * r as u32);
                self.c.v128_load(o);
                self.c.local_set(l);
            }
        }
    }

    /// Decision 0169: before a call into the C path (a C function, an import, the dispatch) the callee-saved registers the function holds are stored to R[] as well, because the C runtime
    /// reads the whole register file for the SEH unwinder and contexts (RaiseException, RtlVirtualUnwind), not only the registers the callee's summary says it reads. The callee-saved registers that this function holds in locals and that were not stored yet go to R[] too.
    fn sync_saved_regs(&mut self, already: &[usize]) {
        if self.cx.dry {
            return;
        }
        let r = self.cx.hybrid_r as i32;
        for rg in [3usize, 5, 6, 7, 12, 13, 14, 15] {
            if already.contains(&rg) || self.reg_local[rg] == NONE {
                continue;
            }
            let l = self.reg_local[rg];
            let o = self.c.reg_base((r + 8 * rg as i32) as u32);
            self.c.local_get(l);
            self.c.i64_store(o);
        }
    }

    fn call_to(&mut self, tgt: u64, retaddr: Option<u64>) -> R<()> {
        let site = if self.cx.dry { None } else { self.xsite.take() };
        if let Some((fl, _)) = site {
            self.xflush(fl);
        }
        self.call_to_inner(tgt, retaddr)?;
        if let Some((_, rl)) = site {
            self.xreload(rl);
        }
        if !self.cx.dry {
            self.mx_guard_refresh();
        }
        Ok(())
    }

    fn call_to_inner(&mut self, tgt: u64, retaddr: Option<u64>) -> R<()> {
        if self.cx.dry {
            self.stats.callees.push(tgt);
            return Ok(());
        }
        let sig = *self.cx.sigs.get(&tgt).ok_or_else(|| format!("callee {tgt:#x} has no signature"))?;
        let fi = *self.cx.fidx.get(&tgt).ok_or_else(|| format!("callee {tgt:#x} not compiled"))?;
        self.flush_rsp();
        let rl = self.reg_loc(RSP);
        if self.cx.ext.contains_key(&tgt) {
            // the callee stays on the C path: the registers it reads go to R[], R[4] points at the return address, the results come back from R[]
            let r = self.cx.hybrid_r as i32;
            let o_rsp = self.c.reg_base((r + 8 * RSP as i32) as u32);
            match retaddr {
                Some(next) => {
                    let t = self.tmp(Ty::I64);
                    self.c.local_get(rl);
                    self.c.i64_const(8);
                    self.c.i64_sub();
                    self.c.local_tee(t);
                    self.c.i32_wrap_i64();
                    self.c.i64_const(next as i64);
                    self.c.i64_store(0);
                    self.c.local_get(t);
                }
                None => self.c.local_get(rl),
            }
            self.c.i64_store(o_rsp);
            for rg in sig.params() {
                let l = self.reg_loc(rg);
                let o = self.c.reg_base((r + 8 * rg as i32) as u32);
                self.c.local_get(l);
                self.c.i64_store(o);
            }
            self.sync_saved_regs(&sig.params());
            self.c.call(fi);
            for rg in sig.results() {
                let l = self.reg_loc(rg);
                let o = self.c.reg_base((r + 8 * rg as i32) as u32);
                self.c.i64_load(o);
                self.c.local_set(l);
            }
            self.pending_rsp += sig.rsp;
            return Ok(());
        }
        match retaddr {
            Some(next) => {
                let t = self.tmp(Ty::I64);
                self.c.local_get(rl);
                self.c.i64_const(8);
                self.c.i64_sub();
                self.c.local_tee(t);
                self.c.i32_wrap_i64();
                self.c.i64_const(next as i64);
                self.c.i64_store(0);
                self.c.local_get(t);
            }
            None => self.c.local_get(rl),
        }
        for r in sig.params() {
            let l = self.reg_loc(r);
            self.c.local_get(l);
        }
        self.c.call(fi);
        let rs = sig.results();
        for &r in rs.iter().rev() {
            let l = self.reg_loc(r);
            self.c.local_set(l);
        }
        self.pending_rsp += sig.rsp;
        Ok(())
    }

    /// A call into the C runtime through its register array: R[4] = the return slot (a call pushes `ret` first), `ins` go to R[], `fi` is called (with `target` as its
    /// i64 argument when given), `outs` come back from R[].
    fn c_stub(&mut self, fi: u32, target: Option<u32>, ret: Option<u64>, ins: &[usize], outs: &[usize]) {
        let site = self.xsite.take();
        if let Some((fl, _)) = site {
            self.xflush(fl);
        }
        self.c_stub_inner(fi, target, ret, ins, outs);
        if let Some((_, rl)) = site {
            self.xreload(rl);
        }
        self.mx_guard_refresh();
    }

    fn c_stub_inner(&mut self, fi: u32, target: Option<u32>, ret: Option<u64>, ins: &[usize], outs: &[usize]) {
        self.flush_rsp();
        let rl = self.reg_loc(RSP);
        let r = self.cx.hybrid_r as i32;
        let o_rsp = self.c.reg_base((r + 8 * RSP as i32) as u32);
        match ret {
            Some(next) => {
                let t = self.tmp(Ty::I64);
                self.c.local_get(rl);
                self.c.i64_const(8);
                self.c.i64_sub();
                self.c.local_tee(t);
                self.c.i32_wrap_i64();
                self.c.i64_const(next as i64);
                self.c.i64_store(0);
                self.c.local_get(t);
            }
            None => self.c.local_get(rl),
        }
        self.c.i64_store(o_rsp);
        for &rg in ins {
            let l = self.reg_loc(rg);
            let o = self.c.reg_base((r + 8 * rg as i32) as u32);
            self.c.local_get(l);
            self.c.i64_store(o);
        }
        self.sync_saved_regs(ins);
        match target {
            Some(tt) => {
                self.c.local_get(tt);
                self.c.call(fi);
            }
            None => self.c.call(fi),
        }
        for &rg in outs {
            let l = self.reg_loc(rg);
            let o = self.c.reg_base((r + 8 * rg as i32) as u32);
            self.c.i64_load(o);
            self.c.local_set(l);
        }
    }

    /// Indirect call through the table of adapters (signature rsp, rcx, rdx, r8, r9 -> rax). The target is read before RSP changes. The table
    /// index is the target address minus the first code address; an address that is not a function with an adapter is a null entry: trap.
    /// Decision 0285: an indirect call of a direct function. The return address is written to the guest stack as for any call; the shared helper looks the target up in the table of adapters (no landing, no census hook, not a pseudo-VA, a function that
    /// fits the uniform indirect signature) and, for a hit, counts the call as `fle_dispatch` does and calls the typed body; a miss leaves everything as it was and the call takes the path of the C dispatch (the code of `c_stub`, as before). The callee
    /// reads nothing but rcx, rdx, r8, r9, rax and rsp, preserves the callee-saved registers and touches no XMM register, so the registers and the XMM locals of this function stay valid across the call and nothing is spilled to R[] / X[].
    fn typed_indirect(&mut self, tt: u32, ret: u64) {
        self.flush_rsp();
        let rl = self.reg_loc(RSP);
        let t = self.tmp(Ty::I64);
        self.c.local_get(rl);
        self.c.i64_const(8);
        self.c.i64_sub();
        self.c.local_tee(t);
        self.c.i32_wrap_i64();
        self.c.i64_const(ret as i64);
        self.c.i64_store(0);
        self.c.local_get(t);
        for r in [1usize, 2, 8, 9, 0] {
            let l = self.reg_loc(r);
            self.c.local_get(l);
        }
        self.c.local_get(tt);
        self.c.call(self.cx.ti_helper);
        let v = self.tmp(Ty::I64);
        self.c.local_set(v);
        self.c.if_();
        let ra = self.reg_loc(RAX);
        self.c.local_get(v);
        self.c.local_set(ra);
        self.mx_guard_refresh();
        self.c.else_();
        self.c_stub(self.cx.c_dispatch, Some(tt), Some(ret), &[1, 2, 8, 9, 0], &[0]);
        self.c.end();
    }

    fn call_indirect(&mut self, i: &Instruction, va: u64) -> R<()> {
        if let Some(name) = self.cx.import_sites.get(&va).cloned() {
            self.stats.import_calls += 1;
            if self.cx.c_mode {
                let key = name.rsplit('!').next().unwrap_or(&name).to_string();
                if !self.cx.c_imp.contains_key(&key) {
                    return Err(format!("import {key} is not modelled by the C runtime"));
                }
                if self.cx.dry {
                    return Ok(());
                }
                let fi = self.cx.c_imp[&key];
                self.c_stub(fi, None, Some(va + i.len() as u64), &[1, 2, 8, 9], &[0]);
                return Ok(());
            }
            if self.cx.dry {
                return Ok(());
            }
            let fi = *self.cx.import_idx.get(&name).ok_or("import not declared")?;
            self.flush_rsp();
            let rl = self.reg_loc(RSP);
            let t = self.tmp(Ty::I64);
            self.c.local_get(rl);
            self.c.i64_const(8);
            self.c.i64_sub();
            self.c.local_tee(t);
            self.c.i32_wrap_i64();
            self.c.i64_const((va + i.len() as u64) as i64);
            self.c.i64_store(0);
            self.c.local_get(t);
            for r in [1usize, 2, 8, 9] {
                let l = self.reg_loc(r);
                self.c.local_get(l);
            }
            self.c.call(fi);
            let ra = self.reg_loc(RAX);
            self.c.local_set(ra);
            return Ok(());
        }
        match i.op_kind(0) {
            OpKind::Register => {
                if i.op_register(0).size() != 8 {
                    return Err("indirect call register is not 64-bit".into());
                }
                self.get_reg(i.op_register(0))?;
            }
            OpKind::Memory => {
                if i.memory_size().size() != 8 {
                    return Err("indirect call memory operand is not 64-bit".into());
                }
                let off = self.mem_addr(i)?;
                self.load(8, off);
            }
            _ => return Err("indirect call operand".into()),
        }
        self.stats.indirect_calls += 1;
        if self.cx.dry {
            self.c.drop();
            return Ok(());
        }
        let tt = self.tmp(Ty::I64);
        if self.cx.c_mode {
            // no check of bits 63:32 here: the C path's dispatch also takes the addresses it hands out for its own functions (a GetProcAddress answer), and stops on an unknown one
            self.c.local_set(tt);
            if self.cx.ti_helper != 0 {
                self.typed_indirect(tt, va + i.len() as u64);
            } else {
                self.c_stub(self.cx.c_dispatch, Some(tt), Some(va + i.len() as u64), &[1, 2, 8, 9, 0], &[0]);
            }
            return Ok(());
        }
        self.c.local_tee(tt);
        self.c.i64_const(32);
        self.c.i64_shr_u();
        self.c.i32_wrap_i64();
        self.trap_if();
        self.flush_rsp();
        let rl = self.reg_loc(RSP);
        let t = self.tmp(Ty::I64);
        self.c.local_get(rl);
        self.c.i64_const(8);
        self.c.i64_sub();
        self.c.local_tee(t);
        self.c.i32_wrap_i64();
        self.c.i64_const((va + i.len() as u64) as i64);
        self.c.i64_store(0);
        self.c.local_get(t);
        for r in [1usize, 2, 8, 9, 0] {
            let l = self.reg_loc(r);
            self.c.local_get(l);
        }
        self.c.local_get(tt);
        self.c.i32_wrap_i64();
        self.c.i32_const(self.cx.text_lo as i32);
        self.c.i32_sub();
        self.c.call_indirect(self.cx.ui_ty, 0);
        let ra = self.reg_loc(RAX);
        self.c.local_set(ra);
        Ok(())
    }

    fn push_value_to_stack(&mut self) {
        // value of type i64 is on the wasm stack; store at new rsp
        self.flush_rsp();
        let rl = self.reg_loc(RSP);
        let t = self.tmp(Ty::I64);
        self.c.local_set(t);
        self.c.local_get(rl);
        self.c.i64_const(8);
        self.c.i64_sub();
        self.c.local_tee(rl);
        self.c.i32_wrap_i64();
        self.c.local_get(t);
        self.c.i64_store(0);
    }

    fn lower_insn(&mut self, va: u64, last: bool) -> R<()> {
        let i = *self.f.insn(va);
        self.stats.insns += 1;
        *self.stats.by_mnemonic.entry(format!("{:?}", i.mnemonic())).or_insert(0) += 1;
        use Mnemonic as M;
        let m = i.mnemonic();
        let flags_def = self.fl.def_of.get(&va).copied();
        if (i.has_lock_prefix() || (matches!(m, M::Xadd | M::Cmpxchg | M::Xchg) && (i.op_kind(0) == OpKind::Memory || i.op_kind(1) == OpKind::Memory))) {
            return Err("atomic read-modify-write stays on the C path (fle_cas, fle_xchg, fle_xadd)".into());
        }
        if crate::flags::is_vex(&i) && !matches!(m, M::Vucomisd | M::Vcomisd | M::Vucomiss | M::Vcomiss) {
            // the machine runs the SSE2 path: this path is never entered; if it is, the run stops here
            self.stats.trapped_vex += 1;
            self.c.unreachable();
            return Ok(());
        }
        if self.cx.gs_ld != 0 && i.memory_segment() == Register::GS {
            // the TEB of the C path is not memory that can be read (see fle_hyb_gs_ld8): only the 64-bit load from a fixed offset is lowered
            if m == M::Mov && i.op_kind(0) == OpKind::Register && i.op_register(0).size() == 8 && i.op_kind(1) == OpKind::Memory && i.memory_base() == Register::None && i.memory_index() == Register::None && i.memory_size().size() == 8 {
                self.c.i64_const(i.memory_displacement64() as i64);
                self.c.call(self.cx.gs_ld);
                return self.set_reg(i.op_register(0));
            }
            return Err("GS access other than a 64-bit load".into());
        }
        if self.mx_fid() && !self.mx_bypass {
            if let Some(r) = self.mx_form(&i) {
                return r;
            }
        }
        if self.cx.c_mode && x87::is_x87(m) {
            return self.x87_insn(&i);
        }
        match m {
            M::Ucomisd | M::Comisd | M::Ucomiss | M::Comiss | M::Vucomisd | M::Vcomisd | M::Vucomiss | M::Vcomiss => {
                let dbl = matches!(m, M::Ucomisd | M::Comisd | M::Vucomisd | M::Vcomisd);
                if i.op_kind(0) != OpKind::Register || !i.op_register(0).is_xmm() {
                    return Err("sse compare destination".into());
                }
                let Some(id) = flags_def else { return Ok(()) };
                if !self.fl.defs[id as usize].need_save || self.cx.dry {
                    return Ok(());
                }
                let (sa, sb) = self.def_locals(id);
                let xa = self.xmm_loc(i.op_register(0).number());
                self.c.local_get(xa);
                self.c.simd_lane(if dbl { 0x1d } else { 0x1b }, 0);
                self.c.local_set(sa);
                if i.op_kind(1) == OpKind::Register {
                    if !i.op_register(1).is_xmm() {
                        return Err("sse source register".into());
                    }
                    let xb = self.xmm_loc(i.op_register(1).number());
                    self.c.local_get(xb);
                    self.c.simd_lane(if dbl { 0x1d } else { 0x1b }, 0);
                } else {
                    let off = self.mem_addr(&i)?;
                    if dbl { self.c.i64_load(off) } else { self.c.i32_load(off) }
                }
                self.c.local_set(sb);
                if self.mx_fid() {
                    // zero, normal numbers: the comparison raises no flag; else the C path's function updates MXCSR (the flags come from the comparison above, as in the fast case)
                    // UCOMIS (not COMIS) raises IE only for a signaling NaN: a quiet NaN is as good as an ordinary operand (the idiom `ucomisd x, x; jp` on a cmpsd mask)
                    let quiet_ok = matches!(m, M::Ucomisd | M::Ucomiss | M::Vucomisd | M::Vucomiss);
                    for t in [sa, sb] {
                        self.mx_ord_chk(t, dbl);
                        if quiet_ok {
                            self.c.local_get(t);
                            if dbl {
                                self.c.i64_const(1);
                                self.c.i64_shl();
                                self.c.i64_const(0xfff0_0000_0000_0000u64 as i64);
                                self.c.i64_ge_u();
                            } else {
                                self.c.i32_const(1);
                                self.c.i32_shl();
                                self.c.i32_const(0xff80_0000u32 as i32);
                                self.c.i32_ge_u();
                            }
                            self.c.i32_or();
                        }
                    }
                    self.c.i32_and();
                    self.mx_force();
                    self.c.i32_eqz();
                    self.c.if_();
                    self.frames.push(Frame::If);
                    self.mx_put(0, |s| {
                        s.c.local_get(sa);
                        s.c.simd(if dbl { 0x12 } else { 0x11 });
                    });
                    self.mx_put(1, |s| {
                        s.c.local_get(sb);
                        s.c.simd(if dbl { 0x12 } else { 0x11 });
                    });
                    let kind = match m {
                        M::Ucomisd | M::Vucomisd => 25,
                        M::Comisd | M::Vcomisd => 26,
                        M::Ucomiss | M::Vucomiss => 27,
                        _ => 28,
                    };
                    self.mx_call(kind, 0);
                    self.frames.pop();
                    self.c.end();
                }
                let mat = self.fl.defs[id as usize].mat;
                self.materialize(Some(id), mat)
            }
            M::Nop | M::Fnop | M::Pause | M::Prefetchnta | M::Prefetcht0 | M::Prefetcht1 | M::Prefetcht2 | M::Prefetchw | M::Sfence | M::Mfence | M::Lfence | M::Cld => {
                Ok(())
            }
            m2 if is_sse_x2(m2) => self.sse_x2(&i),
            M::Movups | M::Movaps | M::Movdqa | M::Movdqu | M::Movapd | M::Movupd | M::Lddqu | M::Movsd | M::Movss | M::Addps | M::Subps | M::Mulps | M::Divps | M::Addpd
            | M::Subpd | M::Mulpd | M::Divpd | M::Addsd | M::Subsd | M::Mulsd | M::Divsd | M::Addss | M::Subss | M::Mulss | M::Divss | M::Xorps | M::Xorpd | M::Pxor
            | M::Andps | M::Andpd | M::Pand | M::Orps | M::Orpd | M::Por | M::Sqrtsd | M::Cvtss2sd | M::Cvtsd2ss | M::Cvtsi2sd | M::Cvtsi2ss => self.sse(&i),
            M::Movd | M::Movq | M::Movntps | M::Movntdq | M::Movntpd | M::Cvtdq2ps | M::Cvtdq2pd | M::Cvtps2pd | M::Cvtpd2ps | M::Cvttpd2dq | M::Cvtpd2dq | M::Sqrtpd | M::Sqrtps | M::Sqrtss
            | M::Maxsd | M::Minsd | M::Maxss | M::Minss | M::Maxpd | M::Minpd | M::Maxps | M::Minps | M::Paddb | M::Paddw | M::Paddd | M::Paddq | M::Psubb | M::Psubw | M::Psubd
            | M::Psubq | M::Psrlq | M::Psllq | M::Psrldq | M::Pslldq | M::Punpcklbw | M::Punpcklqdq | M::Punpckhqdq => self.sse_ext(&i),
            M::Cmpsd if i.op_kind(0) == OpKind::Register && i.op_register(0).is_xmm() => self.sse_ext(&i),
            M::Cvttsd2si => self.cvttsd2si(&i, false),
            M::Cvttss2si => self.cvttsd2si(&i, true),
            M::Stmxcsr if self.cx.mx_st != 0 => {
                let off = self.mem_addr(&i)?;
                self.c.i32_const(off as i32);
                self.c.i32_add();
                self.c.i64_extend_i32_u();
                self.c.call(self.cx.mx_st);
                Ok(())
            }
            M::Ldmxcsr if self.cx.mx_ld != 0 => {
                let off = self.mem_addr(&i)?;
                self.c.i32_const(off as i32);
                self.c.i32_add();
                self.c.i64_extend_i32_u();
                self.c.call(self.cx.mx_ld);
                self.mx_guard_refresh();
                Ok(())
            }
            M::Stmxcsr => {
                let off = self.mem_addr(&i)?;
                self.c.global_get(1);
                self.c.i32_store(off);
                Ok(())
            }
            M::Ldmxcsr => {
                // the value is kept in the global `mxcsr`; a reserved bit (31:16) is #GP. Only the window rule reads it (`mx_ok`); the SSE
                // operations do not apply a loaded rounding mode (as in the C path)
                let off = self.mem_addr(&i)?;
                self.c.i32_load(off);
                let t = self.tmp(Ty::I32);
                self.c.local_tee(t);
                self.c.i32_const(16);
                self.c.i32_shr_u();
                self.trap_if();
                self.c.local_get(t);
                self.c.global_set(1);
                self.c.local_get(t);
                self.c.i32_const(!0x3f);
                self.c.i32_and();
                self.c.i32_const(0x1f80);
                self.c.i32_eq();
                self.c.global_set(0);
                Ok(())
            }
            M::Fnstcw if self.cx.x87_st != 0 => {
                let off = self.mem_addr(&i)?;
                self.c.i32_const(off as i32);
                self.c.i32_add();
                self.c.i64_extend_i32_u();
                self.c.call(self.cx.x87_st);
                Ok(())
            }
            M::Fldcw if self.cx.x87_ld != 0 => {
                let off = self.mem_addr(&i)?;
                self.c.i32_const(off as i32);
                self.c.i32_add();
                self.c.i64_extend_i32_u();
                self.c.call(self.cx.x87_ld);
                Ok(())
            }
            M::Fnstcw | M::Fldcw if self.cx.c_mode => Err("x87 control word is state of the C path (fle_x87.c)".into()),
            // CPUID answers a constant that depends only on the leaf (runtime/fle_cpuid.c: leaf 0 -> eax 1; leaf >= 0x80000000 -> all zero; any other leaf -> edx = FLE_CPUID_LEAF1_EDX); tests/cpuid_hybrid.rs compares this with that function
            M::Cpuid => {
                let leaf = self.tmp(Ty::I32);
                self.get_reg(Register::EAX)?;
                self.c.local_set(leaf);
                self.c.local_get(leaf);
                self.c.i32_eqz();
                self.set_reg(Register::EAX)?;
                self.c.i32_const(0);
                self.set_reg(Register::EBX)?;
                self.c.i32_const(0);
                self.set_reg(Register::ECX)?;
                self.c.i32_const(CPUID_LEAF1_EDX as i32);
                self.c.i32_const(0);
                self.c.local_get(leaf);
                self.c.i32_const(1);
                self.c.i32_sub();
                self.c.i32_const(0x7fff_ffff);
                self.c.i32_lt_u();
                self.c.select();
                self.set_reg(Register::EDX)?;
                Ok(())
            }
            M::Fnstcw => {
                let off = self.mem_addr(&i)?;
                self.c.global_get(2);
                self.c.i32_store16(off);
                Ok(())
            }
            M::Fldcw => {
                let off = self.mem_addr(&i)?;
                self.c.i32_load16_u(off);
                self.c.global_set(2);
                Ok(())
            }
            M::Fnclex => Ok(()),
            M::Bswap => {
                let r = i.op_register(0);
                if r.size() == 8 {
                    self.get_reg(r)?;
                    let v = self.tmp(Ty::I64);
                    self.c.local_set(v);
                    // swap each half, then exchange the halves
                    for half in 0..2 {
                        let t = self.tmp(Ty::I32);
                        self.c.local_get(v);
                        if half == 0 {
                            self.c.i32_wrap_i64();
                        } else {
                            self.c.i64_const(32);
                            self.c.i64_shr_u();
                            self.c.i32_wrap_i64();
                        }
                        self.c.local_set(t);
                        self.c.local_get(t);
                        self.c.i32_const(24);
                        self.c.i32_shl();
                        self.c.local_get(t);
                        self.c.i32_const(8);
                        self.c.i32_shl();
                        self.c.i32_const(0xff0000);
                        self.c.i32_and();
                        self.c.i32_or();
                        self.c.local_get(t);
                        self.c.i32_const(8);
                        self.c.i32_shr_u();
                        self.c.i32_const(0xff00);
                        self.c.i32_and();
                        self.c.i32_or();
                        self.c.local_get(t);
                        self.c.i32_const(24);
                        self.c.i32_shr_u();
                        self.c.i32_or();
                        self.c.i64_extend_i32_u();
                        if half == 0 {
                            self.c.i64_const(32);
                            self.c.i64_shl();
                        }
                    }
                    self.c.i64_or();
                    return self.set_reg(r);
                }
                self.get_reg(r)?;
                {
                    let t = self.tmp(Ty::I32);
                    self.c.local_set(t);
                    self.c.local_get(t);
                    self.c.i32_const(24);
                    self.c.i32_shl();
                    self.c.local_get(t);
                    self.c.i32_const(8);
                    self.c.i32_shl();
                    self.c.i32_const(0xff0000);
                    self.c.i32_and();
                    self.c.i32_or();
                    self.c.local_get(t);
                    self.c.i32_const(8);
                    self.c.i32_shr_u();
                    self.c.i32_const(0xff00);
                    self.c.i32_and();
                    self.c.i32_or();
                    self.c.local_get(t);
                    self.c.i32_const(24);
                    self.c.i32_shr_u();
                    self.c.i32_or();
                    self.set_reg(r)
                }
            }
            M::Movnti => {
                let size = i.memory_size().size();
                let off = self.mem_addr(&i)?;
                self.get_op(&i, 1, size)?;
                self.store(size, off);
                Ok(())
            }
            M::Xchg if i.op_kind(0) == OpKind::Register && i.op_kind(1) == OpKind::Register && i.op_register(0) == i.op_register(1) => Ok(()),
            M::Rol | M::Ror => {
                let size = opsize0(&i) as usize;
                if size != 4 && size != 8 {
                    return Err("8/16-bit rotate".into());
                }
                if let Some(&id) = self.fl.def_of.get(&va) {
                    let d = &self.fl.defs[id as usize];
                    if d.need_save || d.mat != 0 {
                        return Err("rotate flags are read".into());
                    }
                }
                let imm = matches!(i.op_kind(1), OpKind::Immediate8);
                if !imm && !(i.op_kind(1) == OpKind::Register && i.op_register(1) == Register::CL) {
                    return Err("rotate count".into());
                }
                let dst_mem = i.op_kind(0) == OpKind::Memory;
                let mut off = 0;
                if dst_mem {
                    off = self.mem_addr(&i)?;
                    let t = self.tmp(Ty::I32);
                    self.c.local_tee(t);
                    self.c.local_get(t);
                    self.load(size, off);
                } else {
                    self.get_op(&i, 0, size)?;
                }
                if imm {
                    self.const_t(size, (i.immediate(1) & if size == 8 { 63 } else { 31 }) as i64);
                } else {
                    let l = self.reg_loc(RCX);
                    self.c.local_get(l);
                    if size == 4 {
                        self.c.i32_wrap_i64();
                    }
                }
                match (m, size == 8) {
                    (M::Rol, true) => self.c.i64_rotl(),
                    (M::Rol, false) => self.c.i32_rotl(),
                    (_, true) => self.c.i64_rotr(),
                    (_, false) => self.c.i32_rotr(),
                }
                if dst_mem {
                    self.store(size, off);
                    Ok(())
                } else {
                    self.set_reg(i.op_register(0))
                }
            }
            M::Mov => {
                let size = match i.op_kind(0) {
                    OpKind::Register => i.op_register(0).size(),
                    OpKind::Memory => i.memory_size().size(),
                    _ => return Err("mov dst".into()),
                };
                if i.op_kind(0) == OpKind::Memory {
                    let off = self.mem_addr(&i)?;
                    self.get_op(&i, 1, size)?;
                    self.store(size, off);
                } else {
                    // mov r, imm / r, r / r, [m]
                    if i.op_kind(1) == OpKind::Memory {
                        // a 32-bit register destination zero-extends; sizes match the operands
                    }
                    self.get_op(&i, 1, size)?;
                    self.set_reg(i.op_register(0))?;
                }
                Ok(())
            }
            M::Movzx | M::Movsx | M::Movsxd => {
                let ssz = match i.op_kind(1) {
                    OpKind::Register => i.op_register(1).size(),
                    OpKind::Memory => i.memory_size().size(),
                    _ => return Err("movx src".into()),
                };
                let dst = i.op_register(0);
                let dsz = dst.size();
                self.get_op(&i, 1, ssz)?;
                if m == M::Movzx {
                    if dsz == 8 {
                        self.c.i64_extend_i32_u();
                    }
                } else {
                    match ssz {
                        1 => self.c.i32_extend8_s(),
                        2 => self.c.i32_extend16_s(),
                        _ => {}
                    }
                    if dsz == 8 {
                        self.c.i64_extend_i32_s();
                    } else if dsz == 2 {
                        self.c.i32_const(0xffff);
                        self.c.i32_and();
                    }
                }
                self.set_reg(dst)
            }
            M::Cdq => {
                let l0 = self.reg_loc(RAX);
                self.c.local_get(l0);
                self.c.i32_wrap_i64();
                self.c.i32_const(31);
                self.c.i32_shr_s();
                self.c.i64_extend_i32_u();
                let l2 = self.reg_loc(RDX);
                self.c.local_set(l2);
                Ok(())
            }
            M::Cqo => {
                let l0 = self.reg_loc(RAX);
                self.c.local_get(l0);
                self.c.i64_const(63);
                self.c.i64_shr_s();
                let l2 = self.reg_loc(RDX);
                self.c.local_set(l2);
                Ok(())
            }
            M::Cdqe => {
                let l0 = self.reg_loc(RAX);
                self.c.local_get(l0);
                self.c.i64_extend32_s();
                self.c.local_set(l0);
                Ok(())
            }
            M::Cwde => {
                let l0 = self.reg_loc(RAX);
                self.c.local_get(l0);
                self.c.i32_wrap_i64();
                self.c.i32_extend16_s();
                self.c.i64_extend_i32_u();
                self.c.local_set(l0);
                Ok(())
            }
            M::Lea => {
                let dst = i.op_register(0);
                let sz = dst.size();
                if sz != 8 && sz != 4 {
                    return Err("lea destination size".into());
                }
                self.lea_addr(&i)?;
                if sz == 4 {
                    self.c.i32_wrap_i64();
                }
                self.set_reg(dst)
            }
            M::Add | M::Sub | M::And | M::Or | M::Xor | M::Cmp | M::Test | M::Adc | M::Sbb => self.alu(&i, va, flags_def),
            M::Xchg => self.xchg(&i),
            M::Bt | M::Bts | M::Btr | M::Btc => self.bit_test(&i, va, flags_def),
            M::Bsf | M::Bsr => self.bit_scan(&i, va, flags_def),
            M::Mul | M::Div | M::Idiv => self.muldiv(&i, va),
            M::Movsb | M::Movsw | M::Movsq | M::Stosb | M::Stosw | M::Stosd | M::Stosq | M::Scasb | M::Scasw | M::Scasd | M::Scasq | M::Cmpsb | M::Cmpsw | M::Cmpsq => {
                self.string_op(&i, va, flags_def)
            }
            M::Movsd if i.code() == iced_x86::Code::Movsd_m32_m32 => self.string_op(&i, va, flags_def),
            M::Inc | M::Dec | M::Neg | M::Not => self.unary(&i, va, flags_def),
            M::Shl | M::Sal | M::Shr | M::Sar => self.shift(&i, va, flags_def),
            M::Imul => {
                let n = i.op_count();
                if n == 1 {
                    return self.muldiv(&i, va);
                }
                let dst = i.op_register(0);
                let sz = dst.size();
                if sz < 2 {
                    return Err("8-bit imul".into());
                }
                if let Some(id) = flags_def {
                    if self.fl.defs[id as usize].need_save || self.fl.defs[id as usize].mat != 0 {
                        return Err("imul flags are read".into());
                    }
                }
                if n == 2 {
                    self.get_reg(dst)?;
                    self.get_op(&i, 1, sz)?;
                } else {
                    self.get_op(&i, 1, sz)?;
                    self.get_op(&i, 2, sz)?;
                }
                if sz == 8 {
                    self.c.i64_mul();
                } else {
                    self.c.i32_mul();
                    self.mask_small(sz);
                }
                self.set_reg(dst)
            }
            M::Push => {
                if self.skip.contains(&va) {
                    self.pending_rsp -= 8;
                    return Ok(());
                }
                match i.op_kind(0) {
                    OpKind::Register => {
                        if i.op_register(0).size() != 8 {
                            return Err("non-64-bit push".into());
                        }
                        self.get_reg(i.op_register(0))?;
                    }
                    OpKind::Memory => {
                        if i.memory_size().size() != 8 {
                            return Err("non-64-bit push".into());
                        }
                        let off = self.mem_addr(&i)?;
                        self.load(8, off);
                    }
                    k if flags::opsize0(&i) == 8 && matches!(k, OpKind::Immediate8to64 | OpKind::Immediate32to64 | OpKind::Immediate8 | OpKind::Immediate32) => {
                        self.c.i64_const(i.immediate(0) as i64);
                    }
                    _ => return Err("push operand".into()),
                }
                self.push_value_to_stack();
                Ok(())
            }
            M::Pop => {
                if self.skip.contains(&va) {
                    self.pending_rsp += 8;
                    return Ok(());
                }
                if i.op_kind(0) != OpKind::Register || i.op_register(0).size() != 8 {
                    return Err("pop operand".into());
                }
                let dst = i.op_register(0);
                if gpr(dst) == Some(RSP) {
                    return Err("pop rsp".into());
                }
                self.flush_rsp();
                let rl = self.reg_loc(RSP);
                self.c.local_get(rl);
                self.c.i32_wrap_i64();
                self.c.i64_load(0);
                self.set_reg(dst)?;
                self.c.local_get(rl);
                self.c.i64_const(8);
                self.c.i64_add();
                self.c.local_set(rl);
                Ok(())
            }
            M::Call => {
                self.xsite = self.xp.and_then(|p| p.call.get(&va).copied());
                if !crate::ir::is_near(&i) {
                    return self.call_indirect(&i, va);
                }
                let t = i.near_branch_target();
                self.call_to(t, Some(va + i.len() as u64))?;
                // decision 0270 (mt3w): the flags the callee returns with, for the readers that merge them with another definition
                if let Some(id) = flags_def {
                    let mat = self.fl.defs[id as usize].mat;
                    if mat != 0 {
                        self.materialize(Some(id), mat)?;
                    }
                }
                Ok(())
            }
            _ => {
                if let Some(cc) = setcc_cc(m) {
                    if i.op_kind(0) == OpKind::Memory {
                        let off = self.mem_addr(&i)?;
                        self.cond(va, cc)?;
                        self.store(1, off);
                    } else {
                        self.cond(va, cc)?;
                        self.set_reg(i.op_register(0))?;
                    }
                    return Ok(());
                }
                if let Some(cc) = cmov_cc(m) {
                    let dst = i.op_register(0);
                    let sz = dst.size();
                    if sz < 2 {
                        return Err("cmov 8-bit".into());
                    }
                    self.get_op(&i, 1, sz)?;
                    self.get_reg(dst)?;
                    self.cond(va, cc)?;
                    self.c.select();
                    return self.set_reg(dst);
                }
                let _ = last;
                Err(format!("mnemonic {m:?}"))
            }
        }
    }

    /// add sub and or xor cmp test
    fn alu(&mut self, i: &Instruction, va: u64, def: Option<u32>) -> R<()> {
        let m = i.mnemonic();
        let size = opsize0(i) as usize;
        let dst_mem = i.op_kind(0) == OpKind::Memory;
        let store_result = !matches!(m, Mnemonic::Cmp | Mnemonic::Test);
        let (need_save, mat) = match def {
            Some(id) => (self.fl.defs[id as usize].need_save, self.fl.defs[id as usize].mat),
            None => (false, 0),
        };
        let _ = va;
        if !store_result && !need_save {
            // flags nobody reads: the instruction has no effect
            return Ok(());
        }
        let (sa, sb) = match def {
            Some(id) if need_save => self.def_locals(id),
            _ => (NONE, NONE),
        };
        let kind = def.map(|id| self.fl.defs[id as usize].kind);
        // zeroing idiom
        let same_reg = i.op_kind(0) == OpKind::Register
            && i.op_kind(1) == OpKind::Register
            && i.op_register(0) == i.op_register(1)
            && matches!(m, Mnemonic::Xor);
        let mut off = 0;
        if dst_mem {
            off = self.mem_addr(i)?;
            if store_result {
                let t = self.tmp(Ty::I32);
                self.c.local_tee(t);
                self.c.local_get(t);
            }
            self.load(size, off);
        } else if !same_reg {
            self.get_op(i, 0, size)?;
        }
        if same_reg {
            self.const_t(size, 0);
        } else {
            if need_save && matches!(kind, Some(Kind::Sub) | Some(Kind::Add) | Some(Kind::Adc) | Some(Kind::Sbb)) {
                self.c.local_tee(sa);
            }
            self.get_op(i, 1, size)?;
            if need_save && matches!(kind, Some(Kind::Sub) | Some(Kind::Add) | Some(Kind::Adc) | Some(Kind::Sbb)) {
                self.c.local_tee(sb);
            }
            match m {
                Mnemonic::Add | Mnemonic::Adc => self.op_add(size),
                Mnemonic::Sub | Mnemonic::Cmp | Mnemonic::Sbb => self.op_sub(size),
                Mnemonic::And | Mnemonic::Test => self.and_t(size),
                Mnemonic::Or => {
                    if size == 8 {
                        self.c.i64_or()
                    } else {
                        self.c.i32_or()
                    }
                }
                _ => self.xor_t(size),
            }
            if matches!(m, Mnemonic::Adc | Mnemonic::Sbb) {
                let srcs = *self.fl.reader.get(&va).ok_or("adc/sbb without flag source")?;
                let src = srcs[flag_index(CF)].ok_or("adc/sbb without CF source")?;
                self.flag_from(src, CF)?;
                if need_save && matches!(kind, Some(Kind::Adc) | Some(Kind::Sbb)) {
                    let id = def.unwrap();
                    let sc = self.cin_local(id);
                    self.c.local_tee(sc);
                }
                if size == 8 {
                    self.c.i64_extend_i32_u();
                }
                if m == Mnemonic::Adc {
                    self.op_add(size)
                } else {
                    self.op_sub(size)
                }
            }
            if matches!(m, Mnemonic::Add | Mnemonic::Sub | Mnemonic::Cmp | Mnemonic::Adc | Mnemonic::Sbb) {
                self.mask_small(size);
            }
        }
        if need_save && kind == Some(Kind::Logic) {
            self.c.local_tee(sa);
        }
        if store_result {
            if dst_mem {
                self.store(size, off);
            } else {
                self.set_reg(i.op_register(0))?;
            }
        } else {
            self.c.drop();
        }
        self.materialize(def, mat)
    }

    /// XCHG reg, reg (decision 0169; no flags). XCHG with a memory operand is locked implicitly and stays on the C path (fle_xchg, runtime/fle_rmw.h), like every LOCK-prefixed instruction: see the refusal at the head of `insn`.
    fn xchg(&mut self, i: &Instruction) -> R<()> {
        if i.op_kind(0) != OpKind::Register || i.op_kind(1) != OpKind::Register {
            return Err("xchg form".into());
        }
        let size = opsize0(i) as usize;
        let ty = ty_of(size);
        let (tx, ty_) = (self.tmp(ty), self.tmp(ty));
        self.get_op(i, 0, size)?;
        self.c.local_set(tx);
        self.get_op(i, 1, size)?;
        self.c.local_set(ty_);
        self.c.local_get(ty_);
        self.set_reg(i.op_register(0))?;
        self.c.local_get(tx);
        self.set_reg(i.op_register(1))
    }

    fn materialize(&mut self, def: Option<u32>, mat: u8) -> R<()> {
        if let Some(id) = def {
            for k in 0..6 {
                if mat & (1 << k) != 0 {
                    let flag = 1u8 << k;
                    self.flag_expr(id, flag)?;
                    let l = self.mat_local(flag);
                    self.c.local_set(l);
                }
            }
        }
        Ok(())
    }

    /// inc dec neg not
    fn unary(&mut self, i: &Instruction, va: u64, def: Option<u32>) -> R<()> {
        let _ = va;
        let m = i.mnemonic();
        let size = opsize0(i) as usize;
        let dst_mem = i.op_kind(0) == OpKind::Memory;
        let (need_save, mat) = match def {
            Some(id) => (self.fl.defs[id as usize].need_save, self.fl.defs[id as usize].mat),
            None => (false, 0),
        };
        let (sa, sb) = match def {
            Some(id) if need_save => self.def_locals(id),
            _ => (NONE, NONE),
        };
        let mut off = 0;
        if dst_mem {
            off = self.mem_addr(i)?;
            let t = self.tmp(Ty::I32);
            self.c.local_tee(t);
            if m == Mnemonic::Neg {
                // 0 - x: the constant goes below the loaded value
                self.const_t(size, 0);
                self.c.local_get(t);
                self.load(size, off);
            } else {
                self.c.local_get(t);
                self.load(size, off);
            }
        } else if m == Mnemonic::Neg {
            self.const_t(size, 0);
            self.get_op(i, 0, size)?;
        } else {
            self.get_op(i, 0, size)?;
        }
        match m {
            Mnemonic::Inc | Mnemonic::Dec => {
                if need_save {
                    self.c.local_tee(sa);
                }
                self.const_t(size, 1);
                if m == Mnemonic::Inc {
                    self.op_add(size)
                } else {
                    self.op_sub(size)
                }
                self.mask_small(size);
            }
            Mnemonic::Neg => {
                if need_save {
                    self.c.local_tee(sb);
                }
                self.op_sub(size);
                self.mask_small(size);
            }
            _ => {
                self.const_t(size, -1);
                self.xor_t(size);
                self.mask_small(size);
            }
        }
        if dst_mem {
            self.store(size, off);
        } else {
            self.set_reg(i.op_register(0))?;
        }
        self.materialize(def, mat)
    }

    fn shift(&mut self, i: &Instruction, va: u64, def: Option<u32>) -> R<()> {
        let _ = va;
        let m = i.mnemonic();
        let size = opsize0(i) as usize;
        let dst_mem = i.op_kind(0) == OpKind::Memory;
        let (need_save, mat) = match def {
            Some(id) => (self.fl.defs[id as usize].need_save, self.fl.defs[id as usize].mat),
            None => (false, 0),
        };
        let kind = def.map(|id| self.fl.defs[id as usize].kind);
        let imm_count = i.op_count() == 1 || matches!(i.op_kind(1), OpKind::Immediate8);
        let cnt = if i.op_count() == 1 { 1u64 } else if imm_count { i.immediate(1) } else { 0 };
        let mask = if size == 8 { 63 } else { 31 };
        if imm_count && (cnt & mask) == 0 {
            return Ok(()); // count 0: no change at all
        }
        let var_live = !imm_count && matches!(kind, Some(Kind::ShlV | Kind::ShrV | Kind::SarV)) && need_save;
        if !imm_count {
            // cl: the flags depend on cl == 0 (decision 0270: a `weak` definition that keeps the older flags, the count saved with the operands)
            if let Some(id) = def {
                if (self.fl.defs[id as usize].need_save || self.fl.defs[id as usize].mat != 0) && !var_live {
                    return Err("shift by cl with live flags".into());
                }
            }
            if !(i.op_kind(1) == OpKind::Register && i.op_register(1) == Register::CL) {
                return Err("shift count operand".into());
            }
        }
        let (sa, sb) = match def {
            Some(id) if need_save => self.def_locals(id),
            _ => (NONE, NONE),
        };
        let mut off = 0;
        let mut t_addr = NONE;
        if dst_mem {
            off = self.mem_addr(i)?;
            t_addr = self.tmp(Ty::I32);
            self.c.local_tee(t_addr);
            self.c.local_get(t_addr);
            self.load(size, off);
        } else {
            self.get_op(i, 0, size)?;
        }
        if matches!(kind, Some(Kind::Shl(_)) | Some(Kind::Shr(_)) | Some(Kind::Sar(_)) | Some(Kind::ShlV) | Some(Kind::ShrV) | Some(Kind::SarV)) && need_save {
            self.c.local_tee(sa);
        }
        // sar on 8/16 bits needs the sign
        if m == Mnemonic::Sar {
            self.sext_t(size);
        }
        if imm_count {
            self.const_t(size, (cnt & mask) as i64);
        } else {
            let l = self.reg_loc(RCX);
            if var_live {
                // the masked count for the flags; the destination may be rcx itself
                let sc = self.cin_local(def.unwrap());
                self.c.local_get(l);
                self.c.i32_wrap_i64();
                self.c.i32_const(mask as i32);
                self.c.i32_and();
                self.c.local_set(sc);
            }
            self.c.local_get(l);
            if size == 8 {
                // i64 count
            } else {
                self.c.i32_wrap_i64();
            }
        }
        match (m, size == 8) {
            (Mnemonic::Shl | Mnemonic::Sal, true) => self.c.i64_shl(),
            (Mnemonic::Shl | Mnemonic::Sal, false) => self.c.i32_shl(),
            (Mnemonic::Shr, true) => self.c.i64_shr_u(),
            (Mnemonic::Shr, false) => self.c.i32_shr_u(),
            (_, true) => self.c.i64_shr_s(),
            (_, false) => self.c.i32_shr_s(),
        }
        self.mask_small(size);
        if need_save && matches!(kind, Some(Kind::Shl(_)) | Some(Kind::Shr(_)) | Some(Kind::Sar(_)) | Some(Kind::ShlV) | Some(Kind::ShrV) | Some(Kind::SarV)) {
            self.c.local_tee(sb);
        }
        if dst_mem {
            self.store(size, off);
        } else {
            self.set_reg(i.op_register(0))?;
        }
        if var_live {
            let sc = self.cin_local(def.unwrap());
            // a count of 0 changes no flag (the older definitions' materialized values stay); a count above the width of an 8 or 16 bit operand has undefined flags, where the C path stops (`fle_bad_count`)
            self.c.local_get(sc);
            self.c.if_();
            self.frames.push(Frame::If);
            if size < 4 {
                self.c.local_get(sc);
                self.c.i32_const((size * 8) as i32);
                self.c.i32_gt_u();
                self.c.if_();
                self.frames.push(Frame::If);
                self.c.unreachable();
                self.frames.pop();
                self.c.end();
            }
            self.materialize(def, mat)?;
            self.frames.pop();
            self.c.end();
            return Ok(());
        }
        self.materialize(def, mat)
    }

    /// decision 0270 (mt3w): bsf / bsr r, r/m. ZF = (source == 0), saved as the operand; the destination is written only for a nonzero source (the C path's `bscan` does not write it for 0). The other flags
    /// are undefined and not modelled: a reader of one stays on the C path.
    fn bit_scan(&mut self, i: &Instruction, va: u64, def: Option<u32>) -> R<()> {
        let _ = va;
        let size = opsize0(i) as usize;
        if i.op_count() != 2 || i.op_kind(0) != OpKind::Register || !matches!(size, 2 | 4 | 8) {
            return Err("bit scan operand form".into());
        }
        let (need_save, mat) = match def {
            Some(id) => (self.fl.defs[id as usize].need_save, self.fl.defs[id as usize].mat),
            None => (false, 0),
        };
        let wide = size == 8;
        let t = self.tmp(ty_of(size));
        self.get_op(i, 1, size)?;
        self.c.local_set(t);
        if let (Some(id), true) = (def, need_save) {
            let (sa, _) = self.def_locals(id);
            self.c.local_get(t);
            self.c.local_set(sa);
        }
        self.c.local_get(t);
        if wide {
            self.c.i64_eqz();
        } else {
            self.c.i32_eqz();
        }
        self.c.i32_eqz();
        self.c.if_();
        self.frames.push(Frame::If);
        if i.mnemonic() == Mnemonic::Bsf {
            self.c.local_get(t);
            if wide {
                self.c.i64_ctz();
            } else {
                self.c.i32_ctz();
            }
        } else if wide {
            self.c.i64_const(63);
            self.c.local_get(t);
            self.c.i64_clz();
            self.c.i64_sub();
        } else {
            self.c.i32_const(31);
            self.c.local_get(t);
            self.c.i32_clz();
            self.c.i32_sub();
        }
        self.set_reg(i.op_register(0))?;
        self.frames.pop();
        self.c.end();
        self.materialize(def, mat)
    }

    /// bt bts btr btc on a register, or on memory with an immediate bit index.
    fn bit_test(&mut self, i: &Instruction, va: u64, def: Option<u32>) -> R<()> {
        let _ = va;
        let m = i.mnemonic();
        let size = opsize0(i) as usize;
        if size < 2 {
            return Err("8-bit bit test".into());
        }
        let dst_mem = i.op_kind(0) == OpKind::Memory;
        let imm = i.op_kind(1) != OpKind::Register;
        if dst_mem && !imm {
            return Err("bit test on memory with a register bit offset".into());
        }
        let need_save = def.map_or(false, |id| self.fl.defs[id as usize].need_save);
        let mutate = m != Mnemonic::Bt;
        if !need_save && !mutate {
            return Ok(());
        }
        let sa = match def {
            Some(id) if need_save => self.def_locals(id).0,
            _ => NONE,
        };
        let t = ty_of(size);
        let ta = self.tmp(t);
        self.get_op(i, 0, size)?;
        self.c.local_set(ta);
        let bits = (size * 8) as u64;
        let push_idx = |s: &mut Self| -> R<()> {
            if imm {
                s.const_t(size, (i.immediate(1) & (bits - 1)) as i64);
            } else {
                s.get_op(i, 1, size)?;
                if size == 2 {
                    s.c.i32_const(15);
                    s.c.i32_and();
                }
            }
            Ok(())
        };
        if need_save {
            self.c.local_get(ta);
            push_idx(self)?;
            if size == 8 {
                self.c.i64_shr_u();
                self.c.i64_const(1);
                self.c.i64_and();
                self.c.i32_wrap_i64();
            } else {
                self.c.i32_shr_u();
                self.c.i32_const(1);
                self.c.i32_and();
            }
            self.c.local_set(sa);
        }
        if mutate {
            if dst_mem {
                let off = self.mem_addr(i)?;
                self.c.local_get(ta);
                self.bit_mask(m, size, &push_idx)?;
                self.store(size, off);
            } else {
                self.c.local_get(ta);
                self.bit_mask(m, size, &push_idx)?;
                self.set_reg(i.op_register(0))?;
            }
        }
        Ok(())
    }

    fn bit_mask(&mut self, m: Mnemonic, size: usize, push_idx: &dyn Fn(&mut Self) -> R<()>) -> R<()> {
        // 1 << idx
        self.const_t(size, 1);
        push_idx(self)?;
        if size == 8 {
            self.c.i64_shl();
        } else {
            self.c.i32_shl();
        }
        match m {
            Mnemonic::Bts => self.or_t(size),
            Mnemonic::Btc => self.xor_t(size),
            _ => {
                self.const_t(size, -1);
                self.xor_t(size);
                self.and_t(size);
            }
        }
        self.mask_small(size);
        Ok(())
    }

    fn or_t(&mut self, w: usize) {
        if w == 8 {
            self.c.i64_or()
        } else {
            self.c.i32_or()
        }
    }

    /// MOVS STOS SCAS CMPS (DF = 0 as the Win64 ABI requires at a function boundary), with REP / REPE / REPNE.
    fn string_op(&mut self, i: &Instruction, va: u64, def: Option<u32>) -> R<()> {
        use Mnemonic as M;
        let _ = va;
        let m = i.mnemonic();
        let n: usize = match m {
            M::Movsb | M::Stosb | M::Scasb | M::Cmpsb => 1,
            M::Movsw | M::Stosw | M::Scasw | M::Cmpsw => 2,
            M::Movsd | M::Stosd | M::Scasd => 4,
            _ => 8,
        };
        if matches!(i.memory_segment(), Register::FS | Register::GS) || (0..i.op_count()).any(|k| matches!(i.op_kind(k), OpKind::MemoryESEDI | OpKind::MemorySegESI)) {
            return Err("string instruction with a segment or 32-bit addressing".into());
        }
        let rep = i.has_rep_prefix();
        let repne = i.has_repne_prefix();
        let cmp = matches!(m, M::Scasb | M::Scasw | M::Scasd | M::Scasq | M::Cmpsb | M::Cmpsw | M::Cmpsq);
        if repne && !cmp {
            return Err("REPNE on movs/stos".into());
        }
        let (need_save, mat) = match def {
            Some(id) => (self.fl.defs[id as usize].need_save, self.fl.defs[id as usize].mat),
            None => (false, 0),
        };
        let (sa, sb) = match def {
            Some(id) if need_save => self.def_locals(id),
            _ if cmp && isrep_pre(i) => (self.tmp(ty_of(n)), self.tmp(ty_of(n))),
            _ => (NONE, NONE),
        };
        let need_cmp_vals = need_save || (cmp && isrep_pre(i));
        let rsi = self.reg_loc(6);
        let rdi = self.reg_loc(7);
        let rcx = self.reg_loc(RCX);
        let rax = self.reg_loc(RAX);
        let isrep = rep || repne;
        let bits = n as i64;
        // hi checks. A REP MOVS/STOS touches the whole range, so the start and end are checked once (only when RCX != 0, as no byte is touched
        // otherwise); a REPE/REPNE compare stops early, so its addresses are checked at every element (like the C path's fle_ld).
        let chk_range = |s: &mut Self, l: u32| {
            s.stats.hi_checks += 1;
            s.c.local_get(l);
            s.c.i64_const(32);
            s.c.i64_shr_u();
            s.c.i32_wrap_i64();
            s.c.local_get(l);
            s.c.local_get(rcx);
            if bits > 1 {
                s.c.i64_const(bits.trailing_zeros() as i64);
                s.c.i64_shl();
            }
            s.c.i64_add();
            s.c.i64_const(32);
            s.c.i64_shr_u();
            s.c.i32_wrap_i64();
            s.c.i32_or();
            s.c.local_get(rcx);
            s.c.i64_eqz();
            s.c.i32_eqz();
            s.c.i32_and();
            s.trap_if();
        };
        let chk = |s: &mut Self, l: u32, with_count: bool| {
            if with_count {
                chk_range(s, l);
            } else {
                s.stats.hi_checks += 1;
                s.c.local_get(l);
                s.c.i64_const(32);
                s.c.i64_shr_u();
                s.c.i32_wrap_i64();
                s.trap_if();
            }
        };
        let uses_si = matches!(m, M::Movsb | M::Movsw | M::Movsd | M::Movsq | M::Cmpsb | M::Cmpsw | M::Cmpsq);
        // stos with REP of bytes: one bulk fill
        if isrep && m == M::Stosb {
            chk(self, rdi, true);
            self.c.local_get(rdi);
            self.c.i32_wrap_i64();
            self.c.local_get(rax);
            self.c.i32_wrap_i64();
            self.c.local_get(rcx);
            self.c.i32_wrap_i64();
            self.c.memory_fill();
            self.c.local_get(rdi);
            self.c.local_get(rcx);
            self.c.i64_add();
            self.c.local_set(rdi);
            self.c.i64_const(0);
            self.c.local_set(rcx);
            return Ok(());
        }
        let ran = if isrep && cmp { self.tmp(Ty::I32) } else { NONE };
        if isrep {
            if !cmp {
                chk(self, rdi, true);
                if uses_si {
                    chk(self, rsi, true);
                }
            }
            if cmp {
                self.c.i32_const(0);
                self.c.local_set(ran);
            }
            self.c.block();
            self.frames.push(Frame::If);
            self.c.loop_();
            self.frames.push(Frame::If);
            self.c.local_get(rcx);
            self.c.i64_eqz();
            self.c.br_if(1);
            if cmp {
                chk(self, rdi, false);
                if uses_si {
                    chk(self, rsi, false);
                }
            }
        } else {
            chk(self, rdi, false);
            if uses_si {
                chk(self, rsi, false);
            }
        }
        // one element
        let ld = |s: &mut Self, l: u32| {
            s.c.local_get(l);
            s.c.i32_wrap_i64();
            s.load(n, 0);
        };
        match m {
            M::Movsb | M::Movsw | M::Movsd | M::Movsq => {
                self.c.local_get(rdi);
                self.c.i32_wrap_i64();
                ld(self, rsi);
                self.store(n, 0);
            }
            M::Stosb | M::Stosw | M::Stosd | M::Stosq => {
                self.c.local_get(rdi);
                self.c.i32_wrap_i64();
                self.c.local_get(rax);
                if n < 8 {
                    self.c.i32_wrap_i64();
                }
                self.store(n, 0);
            }
            M::Scasb | M::Scasw | M::Scasd | M::Scasq => {
                if need_cmp_vals {
                    self.c.local_get(rax);
                    if n < 8 {
                        self.c.i32_wrap_i64();
                        self.mask_small(n);
                    }
                    self.c.local_set(sa);
                    ld(self, rdi);
                    self.c.local_set(sb);
                }
            }
            _ => {
                if need_cmp_vals {
                    ld(self, rsi);
                    self.c.local_set(sa);
                    ld(self, rdi);
                    self.c.local_set(sb);
                }
            }
        }
        if uses_si {
            self.c.local_get(rsi);
            self.c.i64_const(bits);
            self.c.i64_add();
            self.c.local_set(rsi);
        }
        self.c.local_get(rdi);
        self.c.i64_const(bits);
        self.c.i64_add();
        self.c.local_set(rdi);
        if isrep {
            self.c.local_get(rcx);
            self.c.i64_const(1);
            self.c.i64_sub();
            self.c.local_set(rcx);
            if cmp {
                self.c.i32_const(1);
                self.c.local_set(ran);
                // REPE stops when ZF = 0, REPNE when ZF = 1
                self.c.local_get(sa);
                self.c.local_get(sb);
                if n == 8 {
                    if repne { self.c.i64_eq() } else { self.c.i64_ne() }
                } else if repne {
                    self.c.i32_eq()
                } else {
                    self.c.i32_ne()
                }
                self.c.br_if(1);
            }
            self.c.br(0);
            self.frames.pop();
            self.c.end();
            self.frames.pop();
            self.c.end();
            if cmp && mat != 0 {
                self.c.local_get(ran);
                self.c.if_();
                self.frames.push(Frame::If);
                self.materialize(def, mat)?;
                self.frames.pop();
                self.c.end();
            }
        } else {
            self.materialize(def, mat)?;
        }
        Ok(())
    }

    /// cvttsd2si r32/r64, xmm/m64: truncation toward zero; a value outside the integer range or a NaN gives the "integer indefinite"
    /// (0x80000000 / 0x8000000000000000), which wasm's trapping or saturating truncation does not, so those go to the slow import.
    fn cvttsd2si(&mut self, i: &Instruction, single: bool) -> R<()> {
        let dst = i.op_register(0);
        let wide = dst.size() == 8;
        let t = self.tmp(Ty::F64);
        if i.op_kind(1) == OpKind::Register {
            let s = self.xmm_loc(i.op_register(1).number());
            self.c.local_get(s);
            if single {
                self.c.simd_lane(0x1f, 0);
                self.c.f64_promote_f32();
            } else {
                self.c.simd_lane(0x21, 0);
            }
        } else {
            let off = self.mem_addr(i)?;
            if single {
                self.c.f32_load(off);
                self.c.f64_promote_f32();
            } else {
                self.c.f64_load(off);
            }
        }
        if self.cx.dry {
            self.c.drop();
            self.c.i32_const(0);
            if wide {
                self.c.i64_extend_i32_u();
            }
            return self.set_reg(dst);
        }
        self.c.local_set(t);
        // |x| < 2^31 (or 2^63) : biased exponent <= 1023+30 (1023+62) and not NaN/Inf; zero and denormals truncate to 0
        let lim = if wide { 1023 + 62 } else { 1023 + 30 };
        self.c.local_get(t);
        self.c.i64_reinterpret_f64();
        self.c.i64_const(1);
        self.c.i64_shl();
        self.c.i64_const(53);
        self.c.i64_shr_u();
        self.c.i64_const(lim + 1);
        self.c.i64_lt_u();
        self.c.if_ty(if wide { Ty::I64 } else { Ty::I32 });
        self.frames.push(Frame::If);
        self.c.local_get(t);
        if wide {
            self.c.i64_trunc_sat_f64_s();
        } else {
            self.c.i32_trunc_sat_f64_s();
        }
        self.c.else_();
        self.c.i32_const(if wide { 1 } else { 0 });
        self.c.local_get(t);
        self.c.f64_const(0.0);
        self.c.call(self.cx.mx_slow_i64);
        if !wide {
            self.c.i32_wrap_i64();
        }
        self.frames.pop();
        self.c.end();
        self.set_reg(dst)
    }

    /// A small SSE/SSE2 subset on XMM registers held in v128 locals: moves, packed and scalar float/double arithmetic, bitwise operations.
    /// MXCSR is not modelled (default rounding, no flags, denormals kept); no XMM value crosses a function boundary (checked in `xmm_check`).
    fn sse(&mut self, i: &Instruction) -> R<()> {
        use Mnemonic as M;
        let m = i.mnemonic();
        if m == M::Movsd && i.code() == iced_x86::Code::Movsd_m32_m32 {
            return self.string_op(i, 0, None);
        }
        if i.op_kind(0) != OpKind::Register && i.op_kind(0) != OpKind::Memory {
            return Err("sse operand".into());
        }
        let is_move = matches!(m, M::Movups | M::Movaps | M::Movdqa | M::Movdqu | M::Movapd | M::Movupd | M::Lddqu | M::Movsd | M::Movss);
        let scalar_d = matches!(m, M::Movsd | M::Addsd | M::Subsd | M::Mulsd | M::Divsd);
        let scalar_s = matches!(m, M::Movss | M::Addss | M::Subss | M::Mulss | M::Divss);
        // destination memory: store
        if i.op_kind(0) == OpKind::Memory {
            if !is_move || i.op_kind(1) != OpKind::Register || !i.op_register(1).is_xmm() {
                return Err("sse store form".into());
            }
            let x = self.xmm_loc(i.op_register(1).number());
            let off = self.mem_addr(i)?;
            self.c.local_get(x);
            if scalar_d {
                self.c.simd_lane(0x1d, 0);
                self.c.i64_store(off);
            } else if scalar_s {
                self.c.simd_lane(0x1b, 0); // i32x4.extract_lane
                self.c.i32_store(off);
            } else {
                self.c.v128_store(off);
            }
            return Ok(());
        }
        let dst = i.op_register(0);
        if !dst.is_xmm() {
            return Err("sse destination".into());
        }
        let d = self.xmm_loc(dst.number());
        let src_is_reg = i.op_kind(1) == OpKind::Register;
        if src_is_reg && !i.op_register(1).is_xmm() && m != M::Cvtsi2sd && m != M::Cvtsi2ss {
            return Err("sse source register".into());
        }
        // zeroing idioms
        if matches!(m, M::Xorps | M::Xorpd | M::Pxor) && src_is_reg && i.op_register(0) == i.op_register(1) {
            self.c.v128_const([0; 16]);
            self.c.local_set(d);
            return Ok(());
        }
        let src_x = if let Some(l) = self.src_ov { Some(l) } else if src_is_reg && i.op_register(1).is_xmm() { Some(self.xmm_loc(i.op_register(1).number())) } else { None };
        if is_move {
            if scalar_d || scalar_s {
                if !src_is_reg {
                    // load into the low lane, zero the rest
                    let off = self.mem_addr(i)?;
                    self.c.simd_mem(if scalar_d { 0x5d } else { 0x5c }, off);
                    self.c.local_set(d);
                } else {
                    // register to register: the low lane is replaced, the rest kept
                    self.c.local_get(d);
                    self.c.local_get(src_x.unwrap());
                    if scalar_d {
                        self.c.simd_lane(0x1d, 0);
                        self.c.simd_lane(0x1e, 0);
                    } else {
                        self.c.simd_lane(0x1b, 0);
                        self.c.simd_lane(0x1c, 0);
                    }
                    self.c.local_set(d);
                }
            } else if let Some(s) = src_x {
                self.c.local_get(s);
                self.c.local_set(d);
            } else {
                let off = self.mem_addr(i)?;
                self.c.v128_load(off);
                self.c.local_set(d);
            }
            return Ok(());
        }
        // arithmetic and bitwise
        if matches!(m, M::Sqrtsd | M::Cvtss2sd | M::Cvtsd2ss) {
            // low lane = f(source); the rest of the destination is kept
            self.c.local_get(d);
            let double_src = m != M::Cvtss2sd;
            match src_x {
                Some(s) => {
                    self.c.local_get(s);
                    self.c.simd_lane(if double_src { 0x21 } else { 0x1f }, 0);
                }
                None => {
                    let off = self.mem_addr(i)?;
                    if double_src { self.c.f64_load(off) } else { self.c.f32_load(off) }
                }
            }
            match m {
                M::Sqrtsd => self.c.f64_sqrt(),
                M::Cvtss2sd => self.c.f64_promote_f32(),
                _ => self.c.f32_demote_f64(),
            }
            self.c.simd_lane(if m == M::Cvtsd2ss { 0x20 } else { 0x22 }, 0);
            self.c.local_set(d);
            return Ok(());
        }
        if m == M::Cvtsi2ss {
            // decision 0169: the exact lowering (round to nearest even, MXCSR in the default state); the window version is mx_cvtsi2ss
            let sz = if i.op_kind(1) == OpKind::Register { i.op_register(1).size() } else { i.memory_size().size() };
            self.c.local_get(d);
            self.get_op(i, 1, sz)?;
            if sz == 8 {
                self.c.f32_convert_i64_s();
            } else {
                self.c.f32_convert_i32_s();
            }
            self.c.simd_lane(0x20, 0);
            self.c.local_set(d);
            return Ok(());
        }
        if m == M::Cvtsi2sd {
            let sz = if i.op_kind(1) == OpKind::Register { i.op_register(1).size() } else { i.memory_size().size() };
            self.c.local_get(d);
            self.get_op(i, 1, sz)?;
            if sz == 8 {
                self.c.f64_convert_i64_s();
            } else {
                self.c.f64_convert_i32_s();
            }
            self.c.simd_lane(0x22, 0);
            self.c.local_set(d);
            return Ok(());
        }
        if scalar_d || scalar_s {
            self.c.local_get(d);
            self.c.local_get(d);
            self.c.simd_lane(if scalar_d { 0x21 } else { 0x1f }, 0);
            match src_x {
                Some(s) => {
                    self.c.local_get(s);
                    self.c.simd_lane(if scalar_d { 0x21 } else { 0x1f }, 0);
                }
                None => {
                    let off = self.mem_addr(i)?;
                    if scalar_d {
                        self.c.f64_load(off);
                    } else {
                        self.c.f32_load(off);
                    }
                }
            }
            let op = match (m, scalar_d) {
                (M::Addsd, _) => 0xa0u8,
                (M::Subsd, _) => 0xa1,
                (M::Mulsd, _) => 0xa2,
                (M::Divsd, _) => 0xa3,
                (M::Addss, _) => 0x92,
                (M::Subss, _) => 0x93,
                (M::Mulss, _) => 0x94,
                _ => 0x95,
            };
            self.c.raw(&[op]);
            self.c.simd_lane(if scalar_d { 0x22 } else { 0x20 }, 0);
            self.c.local_set(d);
            return Ok(());
        }
        self.c.local_get(d);
        match src_x {
            Some(s) => self.c.local_get(s),
            None => {
                let off = self.mem_addr(i)?;
                self.c.v128_load(off);
            }
        }
        match m {
            M::Addps => self.c.simd(0xe4),
            M::Subps => self.c.simd(0xe5),
            M::Mulps => self.c.simd(0xe6),
            M::Divps => self.c.simd(0xe7),
            M::Addpd => self.c.simd(0xf0),
            M::Subpd => self.c.simd(0xf1),
            M::Mulpd => self.c.simd(0xf2),
            M::Divpd => self.c.simd(0xf3),
            M::Xorps | M::Xorpd | M::Pxor => self.c.simd(0x51),
            M::Andps | M::Andpd | M::Pand => self.c.simd(0x4e),
            M::Orps | M::Orpd | M::Por => self.c.simd(0x50),
            _ => return Err(format!("sse op {m:?}")),
        }
        self.c.local_set(d);
        Ok(())
    }

    /// i32 on the stack: the low lanes of the v128 locals `a` and `b` (a double each, or a single each) are both nonzero numbers whose biased exponent lies in an aligned window of 2^k
    /// exponents (f64: 767..1279 = 2^-256..2^256, f32: 95..159 = 2^-32..2^32): ((a<<1) - K | (b<<1) - K) >> (top bits) == 0, done with SIMD (no move of the bits to a general register).
    /// Zero, infinity, NaN and denormals fail it (the precise check then decides).
    fn mx_l1v(&mut self, a: u32, b: u32, dbl: bool) {
        self.c.local_get(a);
        self.c.local_get(b);
        if dbl {
            self.c.simd_shuffle([0, 1, 2, 3, 4, 5, 6, 7, 16, 17, 18, 19, 20, 21, 22, 23]);
            self.c.i32_const(1);
            self.c.simd(0xcb); // i64x2.shl
            let k = (767i64 << 53).to_le_bytes();
            let mut c = [0u8; 16];
            c[..8].copy_from_slice(&k);
            c[8..].copy_from_slice(&k);
            self.c.v128_const(c);
            self.c.simd(0xd1); // i64x2.sub
            self.c.i32_const(62);
            self.c.simd(0xcd); // i64x2.shr_u
        } else {
            self.c.simd_shuffle([0, 1, 2, 3, 16, 17, 18, 19, 0, 1, 2, 3, 16, 17, 18, 19]);
            self.c.i32_const(1);
            self.c.simd(0xab); // i32x4.shl
            let k = ((95i32) << 24).to_le_bytes();
            let mut c = [0u8; 16];
            for l in 0..4 {
                c[4 * l..4 * l + 4].copy_from_slice(&k);
            }
            self.c.v128_const(c);
            self.c.simd(0xb1); // i32x4.sub
            self.c.i32_const(30);
            self.c.simd(0xad); // i32x4.shr_u
        }
        self.c.simd(0x53); // v128.any_true
        self.c.i32_eqz();
    }

    /// after a window condition (i32 on the stack): the test option forces the slow path
    fn mx_force(&mut self) {
        if MX_FORCE_SLOW.load(std::sync::atomic::Ordering::Relaxed) {
            self.c.drop();
            self.c.i32_const(0);
        }
    }

    fn mx_fid(&self) -> bool {
        self.cx.mx_window && self.cx.mx_addr != 0
    }

    /// guard of the operations that can be inexact: RC = 0, PM and PE set, in the C path's MXCSR (recomputed after every call and ldmxcsr; a stale 0 is safe)
    fn mx_guard_refresh(&mut self) {
        if self.mx_guard == NONE {
            return;
        }
        self.c.i32_const(0);
        self.c.i32_load(self.cx.mx_addr);
        self.c.i32_const(0x7020);
        self.c.i32_and();
        self.c.i32_const(0x1020);
        self.c.i32_eq();
        self.c.local_set(self.mx_guard);
        if self.mx_rcl != NONE {
            // decision 0181: rcl = RC when PE is set and PM masked (the guard of the RC != 0 helpers), else 4 (C path); the local holds MXCSR on the way
            self.c.i32_const(0);
            self.c.i32_load(self.cx.mx_addr);
            self.c.local_set(self.mx_rcl);
            self.c.local_get(self.mx_rcl);
            self.c.i32_const(13);
            self.c.i32_shr_u();
            self.c.i32_const(3);
            self.c.i32_and();
            self.c.i32_const(4);
            self.c.local_get(self.mx_rcl);
            self.c.i32_const(0x1020);
            self.c.i32_and();
            self.c.i32_const(0x1020);
            self.c.i32_eq();
            self.c.select();
            self.c.local_set(self.mx_rcl);
        }
    }

    /// i32 on the stack: the bits in `t` (i64 for a double, i32 for a single) are zero (if `zero`) or have a biased exponent in [lo, hi]
    fn mx_win_chk(&mut self, t: u32, dbl: bool, lo: i64, hi: i64, zero: bool) {
        self.c.local_get(t);
        if dbl {
            self.c.i64_const(1);
            self.c.i64_shl();
            self.c.i64_const(lo << 53);
            self.c.i64_sub();
            self.c.i64_const((hi - lo + 1) << 53);
            self.c.i64_lt_u();
            if zero {
                self.c.local_get(t);
                self.c.i64_const(1);
                self.c.i64_shl();
                self.c.i64_eqz();
                self.c.i32_or();
            }
        } else {
            self.c.i32_const(1);
            self.c.i32_shl();
            self.c.i32_const((lo << 24) as i32);
            self.c.i32_sub();
            self.c.i32_const(((hi - lo + 1) << 24) as i32);
            self.c.i32_lt_u();
            if zero {
                self.c.local_get(t);
                self.c.i32_const(1);
                self.c.i32_shl();
                self.c.i32_eqz();
                self.c.i32_or();
            }
        }
    }

    /// zero, a normal number or an infinity (no NaN, no denormal): the operands for which a comparison, min/max and the exact conversions raise no flag
    fn mx_ord_chk(&mut self, t: u32, dbl: bool) {
        self.c.local_get(t);
        if dbl {
            self.c.i64_const(1);
            self.c.i64_shl();
            self.c.i64_const(1i64 << 53);
            self.c.i64_sub();
            self.c.i64_const((0xffe0_0000_0000_0000u64 - (1u64 << 53) + 1) as i64);
            self.c.i64_lt_u();
            self.c.local_get(t);
            self.c.i64_const(1);
            self.c.i64_shl();
            self.c.i64_eqz();
        } else {
            self.c.i32_const(1);
            self.c.i32_shl();
            self.c.i32_const(1 << 24);
            self.c.i32_sub();
            self.c.i32_const((0xff00_0000u32 - (1u32 << 24) + 1) as i32);
            self.c.i32_lt_u();
            self.c.local_get(t);
            self.c.i32_const(1);
            self.c.i32_shl();
            self.c.i32_eqz();
        }
        self.c.i32_or();
    }

    /// zero, or a positive number with a biased exponent in [lo, hi] (the operand of sqrt)
    fn mx_pos_chk(&mut self, t: u32, dbl: bool, lo: i64, hi: i64) {
        self.c.local_get(t);
        if dbl {
            self.c.i64_const(lo << 52);
            self.c.i64_sub();
            self.c.i64_const((hi - lo + 1) << 52);
            self.c.i64_lt_u();
            self.c.local_get(t);
            self.c.i64_const(1);
            self.c.i64_shl();
            self.c.i64_eqz();
        } else {
            self.c.i32_const((lo << 23) as i32);
            self.c.i32_sub();
            self.c.i32_const(((hi - lo + 1) << 23) as i32);
            self.c.i32_lt_u();
            self.c.local_get(t);
            self.c.i32_const(1);
            self.c.i32_shl();
            self.c.i32_eqz();
        }
        self.c.i32_or();
    }

    /// `slot`(0 = dst, 1 = src, 2 = result) of the scratch buffer of the C path := the v128 that `f` pushes
    fn mx_put(&mut self, slot: u32, f: impl FnOnce(&mut Self)) {
        self.c.i32_const(0);
        f(self);
        self.c.v128_store(self.cx.mx_buf + 16 * slot);
    }

    fn mx_call(&mut self, kind: i32, imm: i32) {
        self.c.i32_const(kind);
        self.c.i32_const(imm);
        self.c.call(self.cx.mx_hyb);
    }

    /// An SSE instruction under the MXCSR window of the hybrid (decision 0164): when the operands are in the window (and MXCSR is in the state the operation needs) the exact
    /// lowering of `sse`/`sse_ext`/`cvttsd2si` runs (no flag can change but the already-set PE), else the C path's own function for the instruction is called (`c.fle_hyb_mx`),
    /// which does what the generated C does: NaN and denormal rules, the MXCSR flags, rounding. Floating-point forms without such a rule are not lowered (the function stays on the C path).
    /// None = not a floating-point instruction with a rule here (the normal lowering).
    fn mx_form(&mut self, i: &Instruction) -> Option<R<()>> {
        use Mnemonic as M;
        let m = i.mnemonic();
        match m {
            M::Addps | M::Subps | M::Mulps | M::Divps | M::Addpd | M::Subpd | M::Mulpd | M::Divpd | M::Sqrtps | M::Maxpd | M::Minpd | M::Maxps | M::Minps => {
                return Some(self.mx_packed(i));
            }
            M::Roundss | M::Roundsd if i.op_kind(0) == OpKind::Register && i.op_register(0).is_xmm() => {
                return Some(self.mx_round(i));
            }
            M::Cvttps2dq if i.op_kind(0) == OpKind::Register && i.op_register(0).is_xmm() => {
                return Some(self.mx_cvttps2dq(i));
            }
            M::Cvtps2dq if i.op_kind(0) == OpKind::Register && i.op_register(0).is_xmm() => {
                return Some(self.mx_cvtps2dq(i));
            }
            M::Cmpps | M::Cmppd | M::Cmpss if i.op_kind(0) == OpKind::Register && i.op_register(0).is_xmm() => {
                return Some(self.mx_cmp(i));
            }
            M::Cvtdq2ps => return Some(self.mx_cvtdq2ps(i)),
            // decision 0252: the reciprocal / reciprocal square root approximations are a table of the hardware (the C path's fle_lg_rsqrt_f32 / fle_lg_rsqrtps / fle_i2_rcpps have it): always the C path's function,
            // which raises no MXCSR flag; ROUNDPS / ROUNDPD (the packed forms of mx_round): the C path's function, which may set PE
            M::Rsqrtps | M::Rsqrtss | M::Rcpps | M::Roundps | M::Roundpd if i.op_kind(0) == OpKind::Register && i.op_register(0).is_xmm() => {
                return Some(self.mx_c_only(i));
            }
            _ => {}
        }
        // (kind of the C function, operand is double, guard needed, window kind)
        enum W {
            Arith,    // add sub mul div: both lane-0 operands in the arithmetic window
            Sqrt,     // positive window of the source
            Ord,      // both lane-0 operands ordinary (zero or normal)
            CvtFrom,  // one lane-0 source with a window [lo, hi] (+ zero)
        }
        let (kind, dbl, guard, w): (i32, bool, bool, W) = match m {
            M::Addss => (0, false, true, W::Arith),
            M::Addsd => (1, true, true, W::Arith),
            M::Subss => (2, false, true, W::Arith),
            M::Subsd => (3, true, true, W::Arith),
            M::Mulss => (4, false, true, W::Arith),
            M::Mulsd => (5, true, true, W::Arith),
            M::Divss => (6, false, true, W::Arith),
            M::Divsd => (7, true, true, W::Arith),
            M::Sqrtss => (8, false, true, W::Sqrt),
            M::Sqrtsd => (9, true, true, W::Sqrt),
            M::Maxss => (10, false, false, W::Ord),
            M::Maxsd => (11, true, false, W::Ord),
            M::Minss => (12, false, false, W::Ord),
            M::Minsd => (13, true, false, W::Ord),
            M::Cmpsd if i.op_kind(0) == OpKind::Register && i.op_register(0).is_xmm() => (20, true, false, W::Ord),
            M::Cvtss2sd => (14, false, false, W::CvtFrom),
            M::Cvtsd2ss => (15, true, true, W::CvtFrom),
            M::Cvttsd2si | M::Cvttss2si | M::Cvtsd2si | M::Cvtss2si => return Some(self.mx_cvtt(i)),
            M::Cvtsi2sd => return self.mx_cvtsi2sd(i),
            M::Cvtsi2ss => return Some(self.mx_cvtsi2ss(i)),
            M::Cvtps2pd | M::Cvtpd2ps | M::Cvttpd2dq | M::Cvtpd2dq | M::Sqrtpd => return Some(self.mx_packed2(i)),
            _ => return None,
        };
        if i.op_kind(0) != OpKind::Register || !i.op_register(0).is_xmm() {
            return None;
        }
        let d = self.xmm_loc(i.op_register(0).number());
        let ty = if dbl { Ty::I64 } else { Ty::I32 };
        let src_dbl = if m == M::Cvtss2sd { false } else { dbl };
        let lane = |dbl: bool| if dbl { 0x1d } else { 0x1b };
        // the source is read once (a memory operand is addressed and checked once): a register operand is used in place, a memory operand goes to a temporary
        let sv = if i.op_kind(1) == OpKind::Register && i.op_register(1).is_xmm() {
            self.xmm_loc(i.op_register(1).number())
        } else {
            let t = self.tmp(Ty::V128);
            if let Err(e) = self.src_vec(i, 1, if src_dbl { 8 } else { 4 }) {
                return Some(Err(e));
            }
            self.c.local_set(t);
            t
        };
        let bs = self.tmp(if src_dbl { Ty::I64 } else { Ty::I32 });
        let (lo, hi) = if dbl { (513, 1533) } else { (65, 189) };
        let imm = if m == M::Cmpsd { i.immediate8() as i32 } else { 0 };
        if matches!(w, W::Arith | W::Ord) {
            // quick test for both operands at once (SIMD) -> the exact operation; else the precise test (zero allowed, the window of the C path) -> the exact operation; else the C path's function
            let bd = self.tmp(ty);
            let is_div = matches!(m, M::Divss | M::Divsd);
            let arith = matches!(w, W::Arith);
            let rc_wrap = self.mx_rcl != NONE && matches!(m, M::Addss | M::Addsd | M::Subss | M::Subsd | M::Mulss | M::Mulsd | M::Divss | M::Divsd);
            if rc_wrap {
                self.c.local_get(self.mx_guard);
                self.c.if_();
                self.frames.push(Frame::If);
            }
            self.mx_l1v(d, sv, dbl);
            if guard {
                self.c.local_get(self.mx_guard);
                self.c.i32_and();
            }
            self.mx_force();
            self.c.if_();
            self.frames.push(Frame::If);
            if let Err(e) = self.mx_fast(i, m, arith, dbl, d, sv) {
                return Some(Err(e));
            }
            self.c.else_();
            self.c.local_get(d);
            self.c.simd_lane(lane(dbl), 0);
            self.c.local_set(bd);
            self.c.local_get(sv);
            self.c.simd_lane(lane(dbl), 0);
            self.c.local_set(bs);
            if arith {
                self.mx_win_chk(bd, dbl, lo, hi, true);
                self.mx_win_chk(bs, dbl, lo, hi, !is_div);
            } else {
                self.mx_ord_chk(bd, dbl);
                self.mx_ord_chk(bs, dbl);
            }
            self.c.i32_and();
            if guard {
                self.c.local_get(self.mx_guard);
                self.c.i32_and();
            }
            self.mx_force();
            self.c.if_();
            self.frames.push(Frame::If);
            if let Err(e) = self.mx_fast(i, m, arith, dbl, d, sv) {
                return Some(Err(e));
            }
            self.c.else_();
            self.mx_slow(kind, imm, d, sv);
            self.frames.pop();
            self.c.end();
            self.frames.pop();
            self.c.end();
            if rc_wrap {
                // decision 0181: the guard of the RC = 0 window failed: RC != 0 with PE set and PM masked takes the helper, anything else the C path
                self.c.else_();
                self.mx_rc_scalar(m, kind, imm, d, sv, dbl);
                self.frames.pop();
                self.c.end();
            }
            self.stats.mx_checked_ops += 1;
            return Some(Ok(()));
        }
        self.c.local_get(sv);
        self.c.simd_lane(lane(src_dbl), 0);
        self.c.local_set(bs);
        match w {
            W::Sqrt => {
                self.mx_pos_chk(bs, dbl, lo, hi);
            }
            _ => {
                if m == M::Cvtss2sd {
                    self.mx_ord_chk(bs, false);
                } else {
                    self.mx_win_chk(bs, true, 923, 1123, true);
                }
            }
        }
        if guard {
            self.c.local_get(self.mx_guard);
            self.c.i32_and();
        }
        self.mx_force();
        self.c.if_();
        self.frames.push(Frame::If);
        if let Err(e) = self.mx_fast(i, m, false, dbl, d, sv) {
            return Some(Err(e));
        }
        self.c.else_();
        if self.mx_rcl != NONE && matches!(m, M::Cvtsd2ss | M::Sqrtsd | M::Sqrtss) {
            let (h, res64) = match m {
                M::Cvtsd2ss => (crate::rcgen::H_CVTSD2SS, false),
                M::Sqrtsd => (crate::rcgen::H_SQRT64, true),
                _ => (crate::rcgen::H_SQRT32, false),
            };
            self.mx_rc_unary(h, res64, kind, imm, d, sv, bs);
        } else {
            self.mx_slow(kind, imm, d, sv);
        }
        self.frames.pop();
        self.c.end();
        self.stats.mx_checked_ops += 1;
        Some(Ok(()))
    }

    /// decision 0181 (--mx-rc): CVTSD2SS, SQRTSD, SQRTSS under RC != 0 (PE set, PM masked): the helper of rcgen.rs on the bits of the source lane `bs`, else the C path's function
    fn mx_rc_unary(&mut self, h: usize, res64: bool, kind: i32, imm: i32, d: u32, sv: u32, bs: u32) {
        self.c.local_get(self.mx_rcl);
        self.c.i32_const(1);
        self.c.i32_sub();
        self.c.i32_const(3);
        self.c.i32_lt_u();
        self.c.if_();
        self.frames.push(Frame::If);
        let (r, ok) = (self.tmp(if res64 { Ty::I64 } else { Ty::I32 }), self.tmp(Ty::I32));
        self.c.local_get(bs);
        self.c.local_get(self.mx_rcl);
        self.c.call(crate::rcgen::helper(h));
        self.c.local_set(ok);
        self.c.local_set(r);
        self.c.local_get(ok);
        self.c.if_();
        self.c.local_get(d);
        self.c.local_get(r);
        self.c.simd_lane(if res64 { 0x1e } else { 0x1c }, 0);
        self.c.local_set(d);
        self.c.else_();
        self.mx_slow(kind, imm, d, sv);
        self.mx_guard_refresh();
        self.c.end();
        self.frames.pop();
        self.c.else_();
        self.mx_slow(kind, imm, d, sv);
        self.mx_guard_refresh();
        self.c.end();
    }

    /// the exact lowering of a scalar float instruction whose operands the window rule has accepted (`d` the destination register, `sv` the source as a v128)
    fn mx_fast(&mut self, i: &Instruction, m: Mnemonic, arith: bool, dbl: bool, d: u32, sv: u32) -> R<()> {
        use Mnemonic as M;
        if arith {
            let (op, rep): (u8, u32) = match m {
                M::Addsd => (0xa0, 0x22),
                M::Subsd => (0xa1, 0x22),
                M::Mulsd => (0xa2, 0x22),
                M::Divsd => (0xa3, 0x22),
                M::Addss => (0x92, 0x20),
                M::Subss => (0x93, 0x20),
                M::Mulss => (0x94, 0x20),
                _ => (0x95, 0x20),
            };
            let fl = if dbl { 0x21 } else { 0x1f };
            self.c.local_get(d);
            self.c.local_get(d);
            self.c.simd_lane(fl, 0);
            self.c.local_get(sv);
            self.c.simd_lane(fl, 0);
            self.c.raw(&[op]);
            self.c.simd_lane(rep, 0);
            self.c.local_set(d);
            Ok(())
        } else {
            self.mx_bypass = true;
            self.src_ov = Some(sv);
            let r = match m {
                M::Maxsd | M::Minsd | M::Maxss | M::Minss | M::Cmpsd | M::Sqrtss => self.sse_ext(i),
                _ => self.sse(i),
            };
            self.src_ov = None;
            self.mx_bypass = false;
            r
        }
    }

    /// the C path's own function for the instruction: operands in the scratch buffer, result back into `d`
    fn mx_slow(&mut self, kind: i32, imm: i32, d: u32, sv: u32) {
        self.mx_put(0, |s| s.c.local_get(d));
        self.mx_put(1, |s| s.c.local_get(sv));
        self.mx_call(kind, imm);
        self.c.i32_const(0);
        self.c.v128_load(self.cx.mx_buf + 32);
        self.c.local_set(d);
    }

    /// i32 on the stack: the guard (PE set, PM masked) for an operation whose result does not depend on the rounding control (`rcfree`: truncating conversions) when `--mx-rc`
    /// tracks RC, else the guard of the RC = 0 window
    fn mx_guard_rcfree(&mut self, rcfree: bool) {
        if rcfree && self.mx_rcl != NONE {
            self.c.local_get(self.mx_rcl);
            self.c.i32_const(4);
            self.c.i32_ne();
        } else {
            self.c.local_get(self.mx_guard);
        }
    }

    /// decision 0181 (--mx-packed): ROUNDSS / ROUNDSD with the rounding mode in the immediate (bit 2 clear): an ordinary source (no NaN, no denormal) -> f32/f64 nearest / floor / ceil / trunc
    /// into lane 0 (the rest of the destination is kept). Without the suppress bit (bit 3) PE can be raised: the guard (PE set, PM masked) is part of the condition. Anything else: the C
    /// path's function (kinds 57, 58).
    fn mx_round(&mut self, i: &Instruction) -> R<()> {
        use Mnemonic as M;
        let m = i.mnemonic();
        if i.op_kind(2) != OpKind::Immediate8 {
            return Err("round immediate".into());
        }
        let imm = i.immediate8() as i32;
        let dbl = m == M::Roundsd;
        let d = self.xmm_loc(i.op_register(0).number());
        let sv = self.tmp(Ty::V128);
        self.src_vec(i, 1, if dbl { 8 } else { 4 })?;
        self.c.local_set(sv);
        let kind = if dbl { 58 } else { 57 };
        if imm & 4 != 0 || imm & 0xf0 != 0 {
            self.mx_slow(kind, imm, d, sv);
            if self.mx_rcl != NONE {
                self.mx_guard_refresh();
            }
            self.stats.mx_checked_ops += 1;
            return Ok(());
        }
        let bs = self.tmp(if dbl { Ty::I64 } else { Ty::I32 });
        self.c.local_get(sv);
        self.c.simd_lane(if dbl { 0x1d } else { 0x1b }, 0);
        self.c.local_set(bs);
        self.mx_ord_chk(bs, dbl);
        if imm & 8 == 0 {
            self.mx_guard_rcfree(true);
            self.c.i32_and();
        }
        self.mx_force();
        self.c.if_();
        self.frames.push(Frame::If);
        self.c.local_get(d);
        self.c.local_get(bs);
        self.c.raw(&[if dbl { 0xbf } else { 0xbe }]);
        let op = match (dbl, imm & 3) {
            (true, 0) => 0x9e,
            (true, 1) => 0x9c,
            (true, 2) => 0x9b,
            (true, _) => 0x9d,
            (false, 0) => 0x90,
            (false, 1) => 0x8e,
            (false, 2) => 0x8d,
            (false, _) => 0x8f,
        };
        self.c.raw(&[op]);
        self.c.simd_lane(if dbl { 0x22 } else { 0x20 }, 0);
        self.c.local_set(d);
        self.c.else_();
        self.mx_slow(kind, imm, d, sv);
        if self.mx_rcl != NONE {
            self.mx_guard_refresh();
        }
        self.frames.pop();
        self.c.end();
        self.stats.mx_checked_ops += 1;
        Ok(())
    }

    /// decision 0181 (--mx-packed): CVTTPS2DQ: all lanes zero or a normal number below 2^31 in magnitude (the result is the truncation; PE is the only flag that can change and it is set) ->
    /// i32x4.trunc_sat_f32x4_s, else the C path's function (kind 56)
    fn mx_cvttps2dq(&mut self, i: &Instruction) -> R<()> {
        let d = self.xmm_loc(i.op_register(0).number());
        let sv = self.tmp(Ty::V128);
        self.src_vec(i, 1, 16)?;
        self.c.local_set(sv);
        let t = self.tmp(Ty::V128);
        crate::rcgen::window_mask(&mut self.c, false, sv, t, 1, 157);
        crate::rcgen::mask_all(&mut self.c, false);
        self.mx_guard_rcfree(true);
        self.c.i32_and();
        self.mx_force();
        self.c.if_();
        self.frames.push(Frame::If);
        self.c.local_get(sv);
        self.c.simd(0xf8);
        self.c.local_set(d);
        self.c.else_();
        self.mx_slow(56, 0, d, sv);
        if self.mx_rcl != NONE {
            self.mx_guard_refresh();
        }
        self.frames.pop();
        self.c.end();
        self.stats.mx_checked_ops += 1;
        Ok(())
    }

    /// CVTPS2DQ (decision 0252): the rounding follows MXCSR.RC. RC = 0 (round to nearest even), PE set, PM masked (the guard of the RC = 0 window; PE can then not change and no other flag can: the lanes are zero or
    /// normal numbers below 2^31 in magnitude, so nothing is out of range, a NaN or a denormal) -> f32x4.nearest, i32x4.trunc_sat_f32x4_s; anything else the C path's function (kind 59)
    fn mx_cvtps2dq(&mut self, i: &Instruction) -> R<()> {
        let d = self.xmm_loc(i.op_register(0).number());
        let sv = self.tmp(Ty::V128);
        self.src_vec(i, 1, 16)?;
        self.c.local_set(sv);
        let t = self.tmp(Ty::V128);
        crate::rcgen::window_mask(&mut self.c, false, sv, t, 1, 157);
        crate::rcgen::mask_all(&mut self.c, false);
        self.mx_guard_rcfree(false);
        self.c.i32_and();
        self.mx_force();
        self.c.if_();
        self.frames.push(Frame::If);
        self.c.local_get(sv);
        self.c.simd(0x6a);
        self.c.simd(0xf8);
        self.c.local_set(d);
        self.c.else_();
        self.mx_slow(59, 0, d, sv);
        if self.mx_rcl != NONE {
            self.mx_guard_refresh();
        }
        self.frames.pop();
        self.c.end();
        self.stats.mx_checked_ops += 1;
        Ok(())
    }

    /// decision 0252: RSQRTPS, RSQRTSS, RCPPS, ROUNDPS, ROUNDPD with no window of their own: the C path's function for the instruction (kinds 61..65 of `fle_hyb_mx`). ROUNDPS / ROUNDPD can set PE, so the
    /// guard is refreshed afterwards when RC is tracked.
    fn mx_c_only(&mut self, i: &Instruction) -> R<()> {
        use Mnemonic as M;
        let m = i.mnemonic();
        let d = self.xmm_loc(i.op_register(0).number());
        let sv = self.tmp(Ty::V128);
        self.src_vec(i, 1, if m == M::Rsqrtss { 4 } else { 16 })?;
        self.c.local_set(sv);
        let (kind, imm) = match m {
            M::Rsqrtps => (61, 0),
            M::Rsqrtss => (62, 0),
            M::Rcpps => (63, 0),
            M::Roundps | M::Roundpd => {
                if i.op_kind(2) != OpKind::Immediate8 {
                    return Err("round immediate".into());
                }
                (if m == M::Roundps { 64 } else { 65 }, i.immediate8() as i32)
            }
            _ => unreachable!(),
        };
        self.mx_slow(kind, imm, d, sv);
        if self.mx_rcl != NONE && matches!(m, M::Roundps | M::Roundpd) {
            self.mx_guard_refresh();
        }
        self.stats.mx_checked_ops += 1;
        Ok(())
    }

    /// i32 on the stack: every lane of the v128 locals `a` and `b` is zero or has a biased exponent in [lo, hi] (`ord`: zero, normal or infinite instead)
    fn mx_pk_ok(&mut self, a: u32, b: u32, dbl: bool, lo: u64, hi: u64, ord: bool) {
        let t = self.tmp(Ty::V128);
        for x in [a, b] {
            if ord {
                crate::rcgen::ord_mask(&mut self.c, dbl, x, t);
            } else {
                crate::rcgen::window_mask(&mut self.c, dbl, x, t, lo, hi);
            }
        }
        crate::rcgen::mask_and(&mut self.c);
        crate::rcgen::mask_all(&mut self.c, dbl);
    }

    /// decision 0181 (--mx-packed): packed SSE arithmetic under the MXCSR window of the hybrid. add/sub/mul: every lane of both operands in the window of the scalar rule (f32: 2^-62..2^62,
    /// f64: 2^-510..2^510, zero allowed) and the guard (RC = 0, PE set, PM masked) -> the plain wasm SIMD operation; RC != 0 with PE set and PM masked (--mx-rc) -> the helper of rcgen.rs;
    /// anything else the C path's function (kinds 40..52 of `fle_hyb_mx`). max/min: every lane ordinary (no NaN, no denormal) -> plain. div and sqrt: always the C path's function.
    fn mx_packed(&mut self, i: &Instruction) -> R<()> {
        use Mnemonic as M;
        let m = i.mnemonic();
        if i.op_kind(0) != OpKind::Register || !i.op_register(0).is_xmm() {
            return Err(format!("{m:?} destination"));
        }
        let d = self.xmm_loc(i.op_register(0).number());
        let sv = self.tmp(Ty::V128);
        self.src_vec(i, 1, 16)?;
        self.c.local_set(sv);
        let dbl = matches!(m, M::Addpd | M::Subpd | M::Mulpd | M::Divpd | M::Maxpd | M::Minpd);
        let kind = match m {
            M::Addps => 40,
            M::Addpd => 41,
            M::Subps => 42,
            M::Subpd => 43,
            M::Mulps => 44,
            M::Mulpd => 45,
            M::Divps => 46,
            M::Divpd => 47,
            M::Sqrtps => 48,
            M::Maxps => 49,
            M::Maxpd => 50,
            M::Minps => 51,
            _ => 52,
        };
        let refresh = self.mx_rcl != NONE;
        self.stats.mx_checked_ops += 1;
        if matches!(m, M::Divps | M::Divpd | M::Sqrtps) {
            self.mx_slow(kind, 0, d, sv);
            if refresh {
                self.mx_guard_refresh();
            }
            return Ok(());
        }
        let (lo, hi) = if dbl { (513, 1533) } else { (65, 189) };
        if matches!(m, M::Maxps | M::Maxpd | M::Minps | M::Minpd) {
            self.mx_pk_ok(d, sv, dbl, 0, 0, true);
            self.mx_force();
            self.c.if_();
            self.frames.push(Frame::If);
            self.mx_bypass = true;
            self.src_ov = Some(sv);
            let r = self.sse_ext(i);
            self.src_ov = None;
            self.mx_bypass = false;
            r?;
            self.c.else_();
            self.mx_slow(kind, 0, d, sv);
            self.frames.pop();
            self.c.end();
            return Ok(());
        }
        let plain = |s: &mut Self| -> R<()> {
            s.mx_bypass = true;
            s.src_ov = Some(sv);
            let r = s.sse(i);
            s.src_ov = None;
            s.mx_bypass = false;
            r
        };
        let sub = matches!(m, M::Subps | M::Subpd);
        if !refresh {
            self.mx_pk_ok(d, sv, dbl, lo, hi, false);
            self.c.local_get(self.mx_guard);
            self.c.i32_and();
            self.mx_force();
            self.c.if_();
            self.frames.push(Frame::If);
            plain(self)?;
            self.c.else_();
            self.mx_slow(kind, 0, d, sv);
            self.frames.pop();
            self.c.end();
            return Ok(());
        }
        // --mx-rc: rcl == 0 is the guard of the plain operation (RC = 0, PE set, PM masked)
        let h = match (m, dbl) {
            (M::Mulps, _) => crate::rcgen::H_MULPS,
            (M::Mulpd, _) => crate::rcgen::H_MULPD,
            (_, true) => crate::rcgen::H_ADDPD,
            _ => crate::rcgen::H_ADDPS,
        };
        self.c.local_get(self.mx_rcl);
        self.c.i32_eqz();
        self.c.if_();
        self.frames.push(Frame::If);
        self.mx_pk_ok(d, sv, dbl, lo, hi, false);
        self.mx_force();
        self.c.if_();
        self.frames.push(Frame::If);
        plain(self)?;
        self.c.else_();
        self.mx_slow(kind, 0, d, sv);
        self.mx_guard_refresh();
        self.frames.pop();
        self.c.end();
        self.c.else_();
        self.c.local_get(self.mx_rcl);
        self.c.i32_const(1);
        self.c.i32_sub();
        self.c.i32_const(3);
        self.c.i32_lt_u();
        self.c.if_();
        self.frames.push(Frame::If);
        let (r, ok) = (self.tmp(Ty::V128), self.tmp(Ty::I32));
        self.c.local_get(d);
        self.c.local_get(sv);
        if sub {
            self.c.v128_const(if dbl { crate::rcgen::splat64(1 << 63) } else { crate::rcgen::splat32(1 << 31) });
            self.c.simd(0x51);
        }
        self.c.local_get(self.mx_rcl);
        self.c.call(crate::rcgen::helper(h));
        self.c.local_set(ok);
        self.c.local_set(r);
        self.c.local_get(ok);
        self.c.if_();
        self.c.local_get(r);
        self.c.local_set(d);
        self.c.else_();
        self.mx_slow(kind, 0, d, sv);
        self.mx_guard_refresh();
        self.c.end();
        self.c.else_();
        self.mx_slow(kind, 0, d, sv);
        self.mx_guard_refresh();
        self.frames.pop();
        self.c.end();
        self.frames.pop();
        self.c.end();
        Ok(())
    }

    /// decision 0181 (--mx-packed): CMPPS / CMPPD / CMPSS with an immediate predicate 0..7. Ordinary operands (no NaN, no denormal: no flag can be raised) -> the SIMD compare, else the C path's
    /// function (kinds 53..55). The scalar form compares lane 0 and keeps the upper lanes of the destination.
    fn mx_cmp(&mut self, i: &Instruction) -> R<()> {
        use Mnemonic as M;
        let m = i.mnemonic();
        if i.op_kind(2) != OpKind::Immediate8 {
            return Err("cmp predicate".into());
        }
        let imm = i.immediate8() as i32;
        let pred = imm & 7;
        let d = self.xmm_loc(i.op_register(0).number());
        let sv = self.tmp(Ty::V128);
        self.src_vec(i, 1, match m {
            M::Cmpps | M::Cmppd => 16,
            _ => 4,
        })?;
        self.c.local_set(sv);
        let dbl = m == M::Cmppd;
        let kind = match m {
            M::Cmpps => 53,
            M::Cmppd => 54,
            _ => 55,
        };
        if m == M::Cmpss {
            let (bd, bs) = (self.tmp(Ty::I32), self.tmp(Ty::I32));
            self.c.local_get(d);
            self.c.simd_lane(0x1b, 0);
            self.c.local_set(bd);
            self.c.local_get(sv);
            self.c.simd_lane(0x1b, 0);
            self.c.local_set(bs);
            self.mx_ord_chk(bd, false);
            self.mx_ord_chk(bs, false);
            self.c.i32_and();
        } else {
            self.mx_pk_ok(d, sv, dbl, 0, 0, true);
        }
        self.mx_force();
        self.c.if_();
        self.frames.push(Frame::If);
        // the compare of all lanes as a mask
        let (eq, ne, lt, le) = if dbl { (0x47, 0x48, 0x49, 0x4b) } else { (0x41, 0x42, 0x43, 0x45) };
        let unord = |s: &mut Self| {
            s.c.local_get(d);
            s.c.local_get(d);
            s.c.simd(ne);
            s.c.local_get(sv);
            s.c.local_get(sv);
            s.c.simd(ne);
            s.c.simd(0x50);
        };
        if m == M::Cmpss {
            self.c.local_get(d);
        }
        match pred {
            0 | 1 | 2 | 4 | 5 | 6 => {
                self.c.local_get(d);
                self.c.local_get(sv);
                self.c.simd(match pred {
                    0 => eq,
                    1 | 5 => lt,
                    2 | 6 => le,
                    _ => ne,
                });
                if pred == 5 || pred == 6 {
                    self.c.simd(0x4d);
                }
            }
            3 => unord(self),
            _ => {
                unord(self);
                self.c.simd(0x4d);
            }
        }
        if m == M::Cmpss {
            self.c.simd_lane(0x1b, 0);
            self.c.simd_lane(0x1c, 0);
        }
        self.c.local_set(d);
        self.c.else_();
        self.mx_slow(kind, imm, d, sv);
        if self.mx_rcl != NONE {
            self.mx_guard_refresh();
        }
        self.frames.pop();
        self.c.end();
        self.stats.mx_checked_ops += 1;
        Ok(())
    }

    /// decision 0181 (--mx-rc): add/sub/mul scalar under RC != 0: the helper of `rcgen.rs` (window and arithmetic in wasm) when RC != 0 with PE set and PM masked (`mx_rcl` in 1..=3),
    /// else the C path's function. The helper returns (bits, 1) or (0, 0) outside its window. A subtraction passes the second operand with its sign flipped.
    fn mx_rc_scalar(&mut self, m: Mnemonic, kind: i32, imm: i32, d: u32, sv: u32, dbl: bool) {
        use Mnemonic as M;
        let h = match (m, dbl) {
            (M::Divsd, _) => crate::rcgen::H_DIV64,
            (M::Divss, _) => crate::rcgen::H_DIV32,
            (M::Mulsd, _) => crate::rcgen::H_MUL64,
            (M::Mulss, _) => crate::rcgen::H_MUL32,
            (_, true) => crate::rcgen::H_ADDSUB64,
            _ => crate::rcgen::H_ADDSUB32,
        };
        let sub = matches!(m, M::Subsd | M::Subss);
        self.c.local_get(self.mx_rcl);
        self.c.i32_const(1);
        self.c.i32_sub();
        self.c.i32_const(3);
        self.c.i32_lt_u();
        self.c.if_();
        self.frames.push(Frame::If);
        let ty = if dbl { Ty::I64 } else { Ty::I32 };
        let (r, ok) = (self.tmp(ty), self.tmp(Ty::I32));
        self.c.local_get(d);
        self.c.simd_lane(if dbl { 0x1d } else { 0x1b }, 0);
        self.c.local_get(sv);
        self.c.simd_lane(if dbl { 0x1d } else { 0x1b }, 0);
        if sub {
            if dbl {
                self.c.i64_const(i64::MIN);
                self.c.i64_xor();
            } else {
                self.c.i32_const(i32::MIN);
                self.c.i32_xor();
            }
        }
        self.c.local_get(self.mx_rcl);
        self.c.call(crate::rcgen::helper(h));
        self.c.local_set(ok);
        self.c.local_set(r);
        self.c.local_get(ok);
        self.c.if_();
        self.c.local_get(d);
        self.c.local_get(r);
        self.c.simd_lane(if dbl { 0x1e } else { 0x1c }, 0);
        self.c.local_set(d);
        self.c.else_();
        self.mx_slow(kind, imm, d, sv);
        self.mx_guard_refresh();
        self.c.end();
        self.frames.pop();
        self.c.else_();
        self.mx_slow(kind, imm, d, sv);
        self.mx_guard_refresh();
        self.c.end();
    }

    /// CVTTSD2SI / CVTTSS2SI r32/r64, xmm/m: truncation; the window needs RC = 0 and PE set (the result is exact or only PE)
    fn mx_cvtt(&mut self, i: &Instruction) -> R<()> {
        // decision 0169: also CVTSD2SI / CVTSS2SI (round to nearest even; RC = 0 is part of the guard). The window of the rounding forms stops one binade earlier than the truncating ones, so that
        // the C path's own limits (|x| < 2^31 - 1, |x| < 2^51: fle_mxv_f2i_rne_ok) are inside it; the values between go to the C function.
        let trunc = matches!(i.mnemonic(), Mnemonic::Cvttss2si | Mnemonic::Cvttsd2si);
        let single = matches!(i.mnemonic(), Mnemonic::Cvttss2si | Mnemonic::Cvtss2si);
        let dst = i.op_register(0);
        let wide = dst.size() == 8;
        let sv = self.tmp(Ty::V128);
        self.src_vec(i, 1, if single { 4 } else { 8 })?;
        self.c.local_set(sv);
        let bs = self.tmp(if single { Ty::I32 } else { Ty::I64 });
        self.c.local_get(sv);
        self.c.simd_lane(if single { 0x1b } else { 0x1d }, 0);
        self.c.local_set(bs);
        let (lo, hi) = match (single, wide) {
            (false, false) => (513, if trunc { 1053 } else { 1052 }),
            (false, true) => (513, if trunc { 1073 } else { 1072 }),
            (true, false) => (65, if trunc { 157 } else { 156 }),
            (true, true) => (65, if trunc { 177 } else { 176 }),
        };
        self.mx_win_chk(bs, !single, lo, hi, true);
        self.mx_guard_rcfree(trunc);
        self.c.i32_and();
        self.mx_force();
        self.c.if_ty(if wide { Ty::I64 } else { Ty::I32 });
        self.frames.push(Frame::If);
        // the exact lowering: truncate the lane
        if single {
            self.c.local_get(bs);
            self.c.raw(&[0xbe]); // f32.reinterpret_i32
            if !trunc {
                self.c.raw(&[0x90]); // f32.nearest (ties to even)
            }
            if wide {
                self.c.raw(&[0xfc, 0x04]); // i64.trunc_sat_f32_s
            } else {
                self.c.raw(&[0xfc, 0x00]); // i32.trunc_sat_f32_s
            }
        } else {
            self.c.local_get(bs);
            self.c.raw(&[0xbf]); // f64.reinterpret_i64
            if !trunc {
                self.c.raw(&[0x9e]); // f64.nearest (ties to even)
            }
            if wide {
                self.c.i64_trunc_sat_f64_s();
            } else {
                self.c.i32_trunc_sat_f64_s();
            }
        }
        self.c.else_();
        self.mx_put(1, |s| s.c.local_get(sv));
        self.mx_call(if !trunc { 35 + 2 * single as i32 + wide as i32 } else if single { if wide { 24 } else { 23 } } else if wide { 22 } else { 21 }, 0);
        self.c.i32_const(0);
        self.c.i64_load(self.cx.mx_buf + 32);
        if !wide {
            self.c.i32_wrap_i64();
        }
        self.frames.pop();
        self.c.end();
        self.stats.mx_checked_ops += 1;
        self.set_reg(dst)
    }

    /// CVTDQ2PS xmm, xmm/m128: exact (no flag) when every lane has |i| <= 2^24
    fn mx_cvtdq2ps(&mut self, i: &Instruction) -> R<()> {
        let d = self.xmm_loc(i.op_register(0).number());
        let sv = self.tmp(Ty::V128);
        self.src_vec(i, 1, 16)?;
        self.c.local_set(sv);
        self.c.local_get(sv);
        self.c.v128_const([0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1]);
        self.c.simd(0xae);
        let k = (1u32 << 25) + 1;
        let kb = k.to_le_bytes();
        let mut c = [0u8; 16];
        for l in 0..4 {
            c[4 * l..4 * l + 4].copy_from_slice(&kb);
        }
        self.c.v128_const(c);
        self.c.simd(0x3a);
        self.c.simd(0xa3);
        self.mx_force();
        self.c.if_();
        self.frames.push(Frame::If);
        self.c.local_get(sv);
        self.c.simd(0xfa);
        self.c.local_set(d);
        self.c.else_();
        self.mx_put(0, |s| s.c.local_get(d));
        self.mx_put(1, |s| s.c.local_get(sv));
        self.mx_call(32, 0);
        self.c.i32_const(0);
        self.c.v128_load(self.cx.mx_buf + 32);
        self.c.local_set(d);
        self.frames.pop();
        self.c.end();
        self.stats.mx_checked_ops += 1;
        Ok(())
    }

    /// CVTSI2SD xmm, r/m32 is exact; from 64 bits it is exact for |i| <= 2^53
    fn mx_cvtsi2sd(&mut self, i: &Instruction) -> Option<R<()>> {
        let sz = if i.op_kind(1) == OpKind::Register { i.op_register(1).size() } else { i.memory_size().size() };
        if sz != 8 {
            return None;
        }
        Some((|| {
            let d = self.xmm_loc(i.op_register(0).number());
            let v = self.tmp(Ty::I64);
            self.get_op(i, 1, 8)?;
            self.c.local_set(v);
            self.c.local_get(v);
            self.c.i64_const(1 << 53);
            self.c.i64_add();
            self.c.i64_const((1 << 54) + 1);
            self.c.i64_lt_u();
            self.mx_force();
            self.c.if_();
            self.frames.push(Frame::If);
            self.c.local_get(d);
            self.c.local_get(v);
            self.c.f64_convert_i64_s();
            self.c.simd_lane(0x22, 0);
            self.c.local_set(d);
            self.c.else_();
            self.mx_put(0, |s| s.c.local_get(d));
            self.mx_put(1, |s| {
                s.c.local_get(v);
                s.c.simd(0x12);
            });
            self.mx_call(30, 0);
            self.c.i32_const(0);
            self.c.v128_load(self.cx.mx_buf + 32);
            self.c.local_set(d);
            self.frames.pop();
            self.c.end();
            self.stats.mx_checked_ops += 1;
            Ok(())
        })())
    }

    /// CVTSI2SS xmm, r32/m32/r64/m64 (decision 0169). |v| <= 2^24 converts exactly (no rounding, no flag, whatever MXCSR says: the same range as the exact case of fle_mx_cvtsi2ss32/64 in
    /// runtime/fle_mxcsr_cvt.h); any other value goes to the C path's function (kinds 33 / 34 of fle_hyb_mx), which rounds and sets PE as the generated C does.
    fn mx_cvtsi2ss(&mut self, i: &Instruction) -> R<()> {
        let sz = if i.op_kind(1) == OpKind::Register { i.op_register(1).size() } else { i.memory_size().size() };
        let is64 = sz == 8;
        let d = self.xmm_loc(i.op_register(0).number());
        let v = self.tmp(if is64 { Ty::I64 } else { Ty::I32 });
        self.get_op(i, 1, sz)?;
        self.c.local_set(v);
        self.c.local_get(v);
        if is64 {
            self.c.i64_const(1 << 24);
            self.c.i64_add();
            self.c.i64_const((1 << 25) + 1);
            self.c.i64_lt_u();
        } else {
            self.c.i32_const(1 << 24);
            self.c.i32_add();
            self.c.i32_const((1 << 25) + 1);
            self.c.i32_lt_u();
        }
        self.mx_force();
        self.c.if_();
        self.frames.push(Frame::If);
        self.c.local_get(d);
        self.c.local_get(v);
        if is64 {
            self.c.f32_convert_i64_s();
        } else {
            self.c.f32_convert_i32_s();
        }
        self.c.simd_lane(0x20, 0);
        self.c.local_set(d);
        self.c.else_();
        self.mx_put(0, |s| s.c.local_get(d));
        self.mx_put(1, |s| {
            s.c.local_get(v);
            if !is64 {
                s.c.i64_extend_i32_s();
            }
            s.c.simd(0x12);
        });
        self.mx_call(if is64 { 34 } else { 33 }, 0);
        self.c.i32_const(0);
        self.c.v128_load(self.cx.mx_buf + 32);
        self.c.local_set(d);
        self.frames.pop();
        self.c.end();
        self.stats.mx_checked_ops += 1;
        Ok(())
    }

    /// CVTPS2PD, CVTPD2PS, CVTTPD2DQ, CVTPD2DQ (decision 0252: the guard is the RC = 0 window, the plain lowering rounds to nearest even), SQRTPD xmm, xmm/m128 (two lanes)
    fn mx_packed2(&mut self, i: &Instruction) -> R<()> {
        use Mnemonic as M;
        let m = i.mnemonic();
        let d = self.xmm_loc(i.op_register(0).number());
        let from_f32 = m == M::Cvtps2pd;
        let sv = self.tmp(Ty::V128);
        self.src_vec(i, 1, if from_f32 { 8 } else { 16 })?;
        self.c.local_set(sv);
        let ty = if from_f32 { Ty::I32 } else { Ty::I64 };
        let b0 = self.tmp(ty);
        let b1 = self.tmp(ty);
        self.c.local_get(sv);
        self.c.simd_lane(if from_f32 { 0x1b } else { 0x1d }, 0);
        self.c.local_set(b0);
        self.c.local_get(sv);
        self.c.simd_lane(if from_f32 { 0x1b } else { 0x1d }, 1);
        self.c.local_set(b1);
        for b in [b0, b1] {
            match m {
                M::Cvtps2pd => self.mx_ord_chk(b, false),
                M::Cvtpd2ps => self.mx_win_chk(b, true, 923, 1123, true),
                M::Cvttpd2dq | M::Cvtpd2dq => self.mx_win_chk(b, true, 513, 1053, true),
                _ => self.mx_pos_chk(b, true, 513, 1533),
            }
        }
        self.c.i32_and();
        if m != M::Cvtps2pd {
            self.mx_guard_rcfree(m == M::Cvttpd2dq);
            self.c.i32_and();
        }
        self.mx_force();
        self.c.if_();
        self.frames.push(Frame::If);
        self.mx_bypass = true;
        let r = if matches!(m, M::Cvttpd2dq | M::Cvtpd2dq) { self.sse_ext(i) } else { self.sse_or_ext(i) };
        self.mx_bypass = false;
        r?;
        self.c.else_();
        self.mx_put(0, |s| s.c.local_get(d));
        self.mx_put(1, |s| s.c.local_get(sv));
        let kind = match m {
            M::Cvtps2pd => 16,
            M::Cvtpd2ps => 17,
            M::Cvttpd2dq => 19,
            M::Cvtpd2dq => 60,
            _ => 31,
        };
        self.mx_call(kind, 0);
        self.c.i32_const(0);
        self.c.v128_load(self.cx.mx_buf + 32);
        self.c.local_set(d);
        self.frames.pop();
        self.c.end();
        self.stats.mx_checked_ops += 1;
        Ok(())
    }

    /// decision 0181 (--mx-packed): SSE forms that raise no floating-point flag and do not depend on the rounding mode: bitwise and-not, the unpack / shuffle / half-move forms, movmsk,
    /// integer compares, shifts by an immediate, pack, pmuludq, pextrw.
    fn sse_x2(&mut self, i: &Instruction) -> R<()> {
        use Mnemonic as M;
        let m = i.mnemonic();
        if i.op_kind(0) == OpKind::Memory {
            if !matches!(m, M::Movlps | M::Movlpd | M::Movhps | M::Movhpd) || i.op_kind(1) != OpKind::Register || !i.op_register(1).is_xmm() {
                return Err(format!("{m:?} store form"));
            }
            let x = self.xmm_loc(i.op_register(1).number());
            let off = self.mem_addr(i)?;
            self.c.local_get(x);
            self.c.simd_lane(0x1d, if matches!(m, M::Movhps | M::Movhpd) { 1 } else { 0 });
            self.c.i64_store(off);
            return Ok(());
        }
        if i.op_kind(0) != OpKind::Register {
            return Err("sse operand".into());
        }
        let dst = i.op_register(0);
        if dst.is_gpr() {
            if i.op_kind(1) != OpKind::Register || !i.op_register(1).is_xmm() {
                return Err(format!("{m:?} source"));
            }
            let s = self.xmm_loc(i.op_register(1).number());
            self.c.local_get(s);
            match m {
                M::Movmskps => self.c.simd(0xa4),
                M::Movmskpd => self.c.simd(0xc4),
                M::Pmovmskb => self.c.simd(0x64),
                M::Pextrw => self.c.simd_lane(0x19, i.immediate8() & 7),
                _ => return Err(format!("{m:?} to a general register")),
            }
            return self.set_reg(dst);
        }
        if !dst.is_xmm() {
            return Err("sse destination".into());
        }
        let d = self.xmm_loc(dst.number());
        let imm = i.immediate8();
        // lane picks of a 32-bit lane k (0..3) of the first / second operand of i8x16.shuffle
        let l32 = |second: bool, k: u8| -> [u8; 4] {
            let b = (if second { 16 } else { 0 }) + 4 * k;
            [b, b + 1, b + 2, b + 3]
        };
        let cat = |p: [[u8; 4]; 4]| -> [u8; 16] {
            let mut o = [0u8; 16];
            for (k, q) in p.iter().enumerate() {
                o[4 * k..4 * k + 4].copy_from_slice(q);
            }
            o
        };
        let bytes = |v: &[u8]| -> [u8; 16] {
            let mut o = [0u8; 16];
            o.copy_from_slice(v);
            o
        };
        match m {
            M::Andnps | M::Andnpd | M::Pandn => {
                self.src_vec(i, 1, 16)?;
                self.c.local_get(d);
                self.c.simd(0x4f);
                self.c.local_set(d);
            }
            M::Movlps | M::Movlpd | M::Movhps | M::Movhpd => {
                if i.op_kind(1) == OpKind::Register {
                    return Err(format!("{m:?} register form"));
                }
                self.c.local_get(d);
                let off = self.mem_addr(i)?;
                self.c.i64_load(off);
                self.c.simd_lane(0x1e, if matches!(m, M::Movhps | M::Movhpd) { 1 } else { 0 });
                self.c.local_set(d);
            }
            M::Movlhps | M::Movhlps => {
                if i.op_kind(1) != OpKind::Register {
                    return Err(format!("{m:?} memory form"));
                }
                self.c.local_get(d);
                self.src_vec(i, 1, 16)?;
                if m == M::Movlhps {
                    self.c.simd_shuffle([0, 1, 2, 3, 4, 5, 6, 7, 16, 17, 18, 19, 20, 21, 22, 23]);
                } else {
                    self.c.simd_shuffle([24, 25, 26, 27, 28, 29, 30, 31, 8, 9, 10, 11, 12, 13, 14, 15]);
                }
                self.c.local_set(d);
            }
            M::Unpcklps | M::Unpckhps | M::Unpcklpd | M::Unpckhpd | M::Punpckldq | M::Punpcklwd | M::Punpckhdq | M::Punpckhwd | M::Punpckhbw => {
                self.c.local_get(d);
                self.src_vec(i, 1, 16)?;
                let sh = match m {
                    M::Unpcklps | M::Punpckldq => cat([l32(false, 0), l32(true, 0), l32(false, 1), l32(true, 1)]),
                    M::Unpckhps | M::Punpckhdq => cat([l32(false, 2), l32(true, 2), l32(false, 3), l32(true, 3)]),
                    M::Unpcklpd => bytes(&[0, 1, 2, 3, 4, 5, 6, 7, 16, 17, 18, 19, 20, 21, 22, 23]),
                    M::Unpckhpd => bytes(&[8, 9, 10, 11, 12, 13, 14, 15, 24, 25, 26, 27, 28, 29, 30, 31]),
                    M::Punpcklwd => bytes(&[0, 1, 16, 17, 2, 3, 18, 19, 4, 5, 20, 21, 6, 7, 22, 23]),
                    M::Punpckhwd => bytes(&[8, 9, 24, 25, 10, 11, 26, 27, 12, 13, 28, 29, 14, 15, 30, 31]),
                    _ => bytes(&[8, 24, 9, 25, 10, 26, 11, 27, 12, 28, 13, 29, 14, 30, 15, 31]),
                };
                self.c.simd_shuffle(sh);
                self.c.local_set(d);
            }
            M::Shufps => {
                self.c.local_get(d);
                self.src_vec(i, 1, 16)?;
                let sh = cat([l32(false, imm & 3), l32(false, (imm >> 2) & 3), l32(true, (imm >> 4) & 3), l32(true, (imm >> 6) & 3)]);
                self.c.simd_shuffle(sh);
                self.c.local_set(d);
            }
            M::Shufpd => {
                self.c.local_get(d);
                self.src_vec(i, 1, 16)?;
                let lo = (imm & 1) * 8;
                let hi = 16 + ((imm >> 1) & 1) * 8;
                let mut sh = [0u8; 16];
                for k in 0..8 {
                    sh[k as usize] = lo + k;
                    sh[8 + k as usize] = hi + k;
                }
                self.c.simd_shuffle(sh);
                self.c.local_set(d);
            }
            M::Pshufd | M::Pshuflw | M::Pshufhw => {
                let t = self.tmp(Ty::V128);
                self.src_vec(i, 1, 16)?;
                self.c.local_set(t);
                self.c.local_get(t);
                self.c.local_get(t);
                let mut sh = [0u8; 16];
                match m {
                    M::Pshufd => {
                        for k in 0..4u8 {
                            let q = (imm >> (2 * k)) & 3;
                            for b in 0..4u8 {
                                sh[(4 * k + b) as usize] = 4 * q + b;
                            }
                        }
                    }
                    _ => {
                        let (w0, c0) = if m == M::Pshuflw { (0u8, 8u8) } else { (8u8, 0u8) };
                        for k in 0..4u8 {
                            let q = (imm >> (2 * k)) & 3;
                            sh[(w0 + 2 * k) as usize] = w0 + 2 * q;
                            sh[(w0 + 2 * k + 1) as usize] = w0 + 2 * q + 1;
                        }
                        for b in 0..8u8 {
                            sh[(c0 + b) as usize] = c0 + b;
                        }
                    }
                }
                self.c.simd_shuffle(sh);
                self.c.local_set(d);
            }
            // decision 0252: SSE3 MOVSLDUP / MOVSHDUP: the even / odd single of the source twice (no flag, no rounding)
            M::Movsldup | M::Movshdup => {
                let t = self.tmp(Ty::V128);
                self.src_vec(i, 1, 16)?;
                self.c.local_set(t);
                self.c.local_get(t);
                self.c.local_get(t);
                let o: u8 = if m == M::Movsldup { 0 } else { 4 };
                self.c.simd_shuffle([o, o + 1, o + 2, o + 3, o, o + 1, o + 2, o + 3, o + 8, o + 9, o + 10, o + 11, o + 8, o + 9, o + 10, o + 11]);
                self.c.local_set(d);
            }
            M::Pcmpeqb | M::Pcmpeqw | M::Pcmpeqd | M::Pcmpgtb | M::Pcmpgtw | M::Pcmpgtd => {
                self.c.local_get(d);
                self.src_vec(i, 1, 16)?;
                self.c.simd(match m {
                    M::Pcmpeqb => 0x23,
                    M::Pcmpeqw => 0x2d,
                    M::Pcmpeqd => 0x37,
                    M::Pcmpgtb => 0x27,
                    M::Pcmpgtw => 0x31,
                    _ => 0x3b,
                });
                self.c.local_set(d);
            }
            M::Psrlw | M::Psraw | M::Psllw | M::Psrld | M::Psrad | M::Pslld | M::Psrlq | M::Psllq => {
                let (w, op, arith) = match m {
                    M::Psrlq => (64u8, 0xcd, false),
                    M::Psllq => (64, 0xcb, false),
                    M::Psrlw => (16u8, 0x8d, false),
                    M::Psraw => (16, 0x8c, true),
                    M::Psllw => (16, 0x8b, false),
                    M::Psrld => (32, 0xad, false),
                    M::Psrad => (32, 0xac, true),
                    _ => (32, 0xab, false),
                };
                if i.op_kind(1) != OpKind::Immediate8 {
                    // the count is the low 64 bits of the source: above width - 1 the result is zero (arithmetic: the sign fill, i.e. a shift by width - 1)
                    let cnt = self.tmp(Ty::I64);
                    self.src_vec(i, 1, 16)?;
                    self.c.simd_lane(0x1d, 0);
                    self.c.local_set(cnt);
                    if arith {
                        self.c.local_get(d);
                        self.c.local_get(cnt);
                        self.c.i64_const((w - 1) as i64);
                        self.c.local_get(cnt);
                        self.c.i64_const((w - 1) as i64);
                        self.c.i64_le_u();
                        self.c.select();
                        self.c.i32_wrap_i64();
                        self.c.simd(op);
                    } else {
                        self.c.local_get(d);
                        self.c.local_get(cnt);
                        self.c.i32_wrap_i64();
                        self.c.simd(op);
                        self.c.v128_const([0; 16]);
                        self.c.local_get(cnt);
                        self.c.i64_const(w as i64);
                        self.c.i64_lt_u();
                        self.c.select();
                    }
                    self.c.local_set(d);
                    return Ok(());
                }
                if imm >= w && !arith {
                    self.c.v128_const([0; 16]);
                } else {
                    self.c.local_get(d);
                    self.c.i32_const(if imm >= w { (w - 1) as i32 } else { imm as i32 });
                    self.c.simd(op);
                }
                self.c.local_set(d);
            }
            M::Packuswb | M::Packsswb | M::Packssdw => {
                self.c.local_get(d);
                self.src_vec(i, 1, 16)?;
                self.c.simd(match m {
                    M::Packuswb => 0x66,
                    M::Packsswb => 0x65,
                    _ => 0x85,
                });
                self.c.local_set(d);
            }
            M::Pmuludq => {
                let k = [0xff, 0xff, 0xff, 0xff, 0, 0, 0, 0, 0xff, 0xff, 0xff, 0xff, 0, 0, 0, 0];
                self.c.local_get(d);
                self.c.v128_const(k);
                self.c.simd(0x4e);
                self.src_vec(i, 1, 16)?;
                self.c.v128_const(k);
                self.c.simd(0x4e);
                self.c.simd(0xd5);
                self.c.local_set(d);
            }
            // decision 0227 (packed integer forms; every one is a lane-wise integer function, so the wasm SIMD instruction of the same definition gives the same bits)
            M::Paddusb | M::Paddusw | M::Psubusb | M::Psubusw | M::Paddsb | M::Paddsw | M::Psubsb | M::Psubsw | M::Pmullw | M::Pmaddwd | M::Pavgb | M::Pavgw | M::Pminub | M::Pmaxub | M::Pminsw | M::Pmaxsw => {
                self.c.local_get(d);
                self.src_vec(i, 1, 16)?;
                self.c.simd(match m {
                    M::Paddusb => 0x70,
                    M::Paddusw => 0x90,
                    M::Psubusb => 0x73,
                    M::Psubusw => 0x93,
                    M::Paddsb => 0x6f,
                    M::Paddsw => 0x8f,
                    M::Psubsb => 0x72,
                    M::Psubsw => 0x92,
                    M::Pmullw => 0x95,
                    M::Pmaddwd => 0xba,
                    M::Pavgb => 0x7b,
                    M::Pavgw => 0x9b,
                    M::Pminub => 0x77,
                    M::Pmaxub => 0x79,
                    M::Pminsw => 0x96,
                    _ => 0x98,
                });
                self.c.local_set(d);
            }
            M::Pmulhw | M::Pmulhuw => {
                // the 32-bit products of the low and of the high four lanes (extmul), then the upper 16 bits of each
                let (lo, hi) = if m == M::Pmulhw { (0xbc, 0xbd) } else { (0xbe, 0xbf) };
                let s = self.tmp(Ty::V128);
                self.src_vec(i, 1, 16)?;
                self.c.local_set(s);
                self.c.local_get(d);
                self.c.local_get(s);
                self.c.simd(lo);
                self.c.local_get(d);
                self.c.local_get(s);
                self.c.simd(hi);
                self.c.simd_shuffle([2, 3, 6, 7, 10, 11, 14, 15, 18, 19, 22, 23, 26, 27, 30, 31]);
                self.c.local_set(d);
            }
            M::Psadbw => {
                // |a - b| per byte = (a -sat b) | (b -sat a); the pairwise sums of the bytes give four 32-bit sums, and the two sums of each 64-bit lane are added
                let s = self.tmp(Ty::V128);
                self.src_vec(i, 1, 16)?;
                self.c.local_set(s);
                self.c.local_get(d);
                self.c.local_get(s);
                self.c.simd(0x73);
                self.c.local_get(s);
                self.c.local_get(d);
                self.c.simd(0x73);
                self.c.simd(0x50);
                self.c.simd(0x7d);
                self.c.simd(0x7f);
                let t = self.tmp(Ty::V128);
                self.c.local_set(t);
                self.c.local_get(t);
                self.c.v128_const([0xff, 0xff, 0xff, 0xff, 0, 0, 0, 0, 0xff, 0xff, 0xff, 0xff, 0, 0, 0, 0]);
                self.c.simd(0x4e);
                self.c.local_get(t);
                self.c.i32_const(32);
                self.c.simd(0xcd);
                self.c.simd(0xce);
                self.c.local_set(d);
            }
            M::Pinsrw => {
                if i.op_kind(2) != OpKind::Immediate8 {
                    return Err("pinsrw count".into());
                }
                self.c.local_get(d);
                if i.op_kind(1) == OpKind::Memory {
                    let off = self.mem_addr(i)?;
                    self.c.i32_load16_u(off);
                } else {
                    if i.op_kind(1) != OpKind::Register || i.op_register(1).size() != 4 {
                        return Err("pinsrw source".into());
                    }
                    self.get_op(i, 1, 4)?;
                }
                self.c.simd_lane(0x1a, i.immediate8() & 7);
                self.c.local_set(d);
            }
            M::Pshufb => {
                // x86: a set bit 7 of the control byte gives 0, otherwise the low four bits pick the byte; swizzle gives 0 for an index above 15
                self.c.local_get(d);
                self.src_vec(i, 1, 16)?;
                self.c.v128_const([0x8f; 16]);
                self.c.simd(0x4e);
                self.c.simd(0x0e);
                self.c.local_set(d);
            }
            M::Palignr => {
                if i.op_kind(2) != OpKind::Immediate8 {
                    return Err("palignr count".into());
                }
                // the 32 bytes d:src shifted right by imm bytes, the low 16 kept (src is the low half)
                let n = i.immediate8() as usize;
                if n >= 32 {
                    self.c.v128_const([0; 16]);
                } else {
                    let mut sh = [0u8; 16];
                    if n < 16 {
                        self.src_vec(i, 1, 16)?;
                        self.c.local_get(d);
                        for k in 0..16usize {
                            sh[k] = (n + k) as u8;
                        }
                    } else {
                        self.c.local_get(d);
                        self.c.v128_const([0; 16]);
                        for k in 0..16usize {
                            let q = n - 16 + k;
                            sh[k] = if q < 16 { q as u8 } else { 16 };
                        }
                    }
                    self.c.simd_shuffle(sh);
                }
                self.c.local_set(d);
            }
            M::Pabsb | M::Pabsw | M::Pabsd => {
                self.src_vec(i, 1, 16)?;
                self.c.simd(match m {
                    M::Pabsb => 0x60,
                    M::Pabsw => 0x80,
                    _ => 0xa0,
                });
                self.c.local_set(d);
            }
            _ => return Err(format!("sse_x2 {m:?}")),
        }
        Ok(())
    }

    fn sse_or_ext(&mut self, i: &Instruction) -> R<()> {
        self.sse_ext(i)
    }

    /// Pushes the source operand `idx` of an SSE instruction as a v128: a register, or a memory operand of `width` bytes (zero-extended to 128 bits).
    fn src_vec(&mut self, i: &Instruction, idx: u32, width: u32) -> R<()> {
        if let (1, Some(l)) = (idx, self.src_ov) {
            self.c.local_get(l);
            return Ok(());
        }
        if i.op_kind(idx) == OpKind::Register {
            if !i.op_register(idx).is_xmm() {
                return Err("sse source register".into());
            }
            let s = self.xmm_loc(i.op_register(idx).number());
            self.c.local_get(s);
        } else {
            let off = self.mem_addr(i)?;
            match width {
                16 => self.c.v128_load(off),
                8 => self.c.simd_mem(0x5d, off),
                _ => self.c.simd_mem(0x5c, off),
            }
        }
        Ok(())
    }

    /// More SSE/SSE2 forms: MOVD/MOVQ, non-temporal stores, int/float conversions, sqrt/max/min, CMPSD, packed integer add/sub/shift/unpack. Floating-point
    /// exception flags and the rounding mode are not modelled here (as in `sse`).
    fn sse_ext(&mut self, i: &Instruction) -> R<()> {
        use Mnemonic as M;
        let m = i.mnemonic();
        if i.op_kind(0) == OpKind::Memory {
            if i.op_kind(1) != OpKind::Register || !i.op_register(1).is_xmm() {
                return Err("sse store form".into());
            }
            let x = self.xmm_loc(i.op_register(1).number());
            let off = self.mem_addr(i)?;
            self.c.local_get(x);
            match m {
                M::Movntps | M::Movntdq | M::Movntpd => self.c.v128_store(off),
                M::Movq => {
                    self.c.simd_lane(0x1d, 0);
                    self.c.i64_store(off);
                }
                M::Movd => {
                    self.c.simd_lane(0x1b, 0);
                    self.c.i32_store(off);
                }
                _ => return Err(format!("sse store {m:?}")),
            }
            return Ok(());
        }
        if i.op_kind(0) != OpKind::Register {
            return Err("sse operand".into());
        }
        let dst = i.op_register(0);
        if dst.is_gpr() {
            // movd r32, xmm / movq r64, xmm
            if !matches!(m, M::Movd | M::Movq) || i.op_kind(1) != OpKind::Register || !i.op_register(1).is_xmm() {
                return Err("sse to general register".into());
            }
            let s = self.xmm_loc(i.op_register(1).number());
            self.c.local_get(s);
            self.c.simd_lane(if m == M::Movq { 0x1d } else { 0x1b }, 0);
            return self.set_reg(dst);
        }
        if !dst.is_xmm() {
            return Err("sse destination".into());
        }
        let d = self.xmm_loc(dst.number());
        let src_gpr = i.op_kind(1) == OpKind::Register && i.op_register(1).is_gpr();
        match m {
            M::Movd | M::Movq => {
                let wide = m == M::Movq;
                self.c.v128_const([0; 16]);
                if src_gpr {
                    self.get_op(i, 1, if wide { 8 } else { 4 })?;
                } else if i.op_kind(1) == OpKind::Register {
                    let s = self.xmm_loc(i.op_register(1).number());
                    self.c.local_get(s);
                    self.c.simd_lane(if wide { 0x1d } else { 0x1b }, 0);
                } else {
                    let off = self.mem_addr(i)?;
                    if wide { self.c.i64_load(off) } else { self.c.i32_load(off) }
                }
                self.c.simd_lane(if wide { 0x1e } else { 0x1c }, 0);
                self.c.local_set(d);
            }
            M::Cvtdq2ps | M::Cvtdq2pd | M::Cvtps2pd | M::Cvtpd2ps | M::Sqrtpd | M::Sqrtps => {
                let (w, op) = match m {
                    M::Cvtdq2ps => (16, 0xfa),
                    M::Cvtdq2pd => (8, 0xfe),
                    M::Cvtps2pd => (8, 0x5f),
                    M::Cvtpd2ps => (16, 0x5e),
                    M::Sqrtpd => (16, 0xef),
                    _ => (16, 0xe3),
                };
                self.src_vec(i, 1, w)?;
                self.c.simd(op);
                self.c.local_set(d);
            }
            M::Cvttpd2dq => {
                // x86 gives 0x80000000 for a NaN or an out-of-range value, wasm's saturating truncation gives 0 / 0x7fffffff there
                let x = self.tmp(Ty::V128);
                self.src_vec(i, 1, 16)?;
                self.c.local_set(x);
                self.c.local_get(x);
                self.c.simd(0xfc);
                // in range: x > -2147483649.0 and x < 2147483648.0
                self.c.local_get(x);
                self.c.v128_const(splat_f64(-2147483649.0));
                self.c.simd(0x4a);
                self.c.local_get(x);
                self.c.v128_const(splat_f64(2147483648.0));
                self.c.simd(0x49);
                self.c.simd(0x4e);
                // the 64-bit lane masks to 32-bit lanes 0 and 1, lanes 2 and 3 are zero
                self.c.v128_const([0; 16]);
                self.c.simd_shuffle([0, 1, 2, 3, 8, 9, 10, 11, 16, 17, 18, 19, 16, 17, 18, 19]);
                let ok = self.tmp(Ty::V128);
                self.c.local_set(ok);
                // result = ok ? truncated : (lanes 0,1 = 0x80000000, lanes 2,3 = 0)
                self.c.v128_const([0, 0, 0, 0x80, 0, 0, 0, 0x80, 0, 0, 0, 0, 0, 0, 0, 0]);
                // stack: trunc, bad-const; bitselect(a=trunc, b=const, mask=ok) = (trunc & ok) | (const & ~ok)
                let cst = self.tmp(Ty::V128);
                self.c.local_set(cst);
                let tr = self.tmp(Ty::V128);
                self.c.local_set(tr);
                self.c.local_get(tr);
                self.c.local_get(cst);
                self.c.local_get(ok);
                self.c.simd(0x52);
                self.c.local_set(d);
            }
            M::Cvtpd2dq => {
                // decision 0252: rounding to nearest even (MXCSR.RC = 0, the only mode of the plain lowering; the MXCSR window of the hybrid guards it). x86 gives 0x80000000 for a NaN or when the rounded value
                // does not fit: x >= 2147483647.5 (it rounds to 2^31, a tie goes to the even one) or x < -2147483648.5 is invalid, -2147483648.5 itself rounds to -2^31 (even) and is valid
                // wasm's saturating truncation gives 0 / 0x7fffffff there
                let x = self.tmp(Ty::V128);
                self.src_vec(i, 1, 16)?;
                self.c.local_set(x);
                self.c.local_get(x);
                self.c.simd(0x94);
                self.c.simd(0xfc);
                // in range: x >= -2147483648.5 and x < 2147483647.5
                self.c.local_get(x);
                self.c.v128_const(splat_f64(-2147483648.5));
                self.c.simd(0x4c);
                self.c.local_get(x);
                self.c.v128_const(splat_f64(2147483647.5));
                self.c.simd(0x49);
                self.c.simd(0x4e);
                // the 64-bit lane masks to 32-bit lanes 0 and 1, lanes 2 and 3 are zero
                self.c.v128_const([0; 16]);
                self.c.simd_shuffle([0, 1, 2, 3, 8, 9, 10, 11, 16, 17, 18, 19, 16, 17, 18, 19]);
                let ok = self.tmp(Ty::V128);
                self.c.local_set(ok);
                // result = ok ? truncated : (lanes 0,1 = 0x80000000, lanes 2,3 = 0)
                self.c.v128_const([0, 0, 0, 0x80, 0, 0, 0, 0x80, 0, 0, 0, 0, 0, 0, 0, 0]);
                // stack: trunc, bad-const; bitselect(a=trunc, b=const, mask=ok) = (trunc & ok) | (const & ~ok)
                let cst = self.tmp(Ty::V128);
                self.c.local_set(cst);
                let tr = self.tmp(Ty::V128);
                self.c.local_set(tr);
                self.c.local_get(tr);
                self.c.local_get(cst);
                self.c.local_get(ok);
                self.c.simd(0x52);
                self.c.local_set(d);
            }
            M::Sqrtss => {
                self.c.local_get(d);
                self.src_vec(i, 1, 4)?;
                self.c.simd(0xe3);
                self.c.simd_lane(0x1f, 0);
                self.c.simd_lane(0x20, 0);
                self.c.local_set(d);
            }
            M::Maxsd | M::Minsd | M::Maxss | M::Minss => {
                let dbl = matches!(m, M::Maxsd | M::Minsd);
                let max = matches!(m, M::Maxsd | M::Maxss);
                self.c.local_get(d);
                // x86 max(a, b) = a > b ? a : b, which is wasm's pmax(b, a)
                self.src_vec(i, 1, if dbl { 8 } else { 4 })?;
                self.c.local_get(d);
                self.c.simd(match (dbl, max) {
                    (true, true) => 0xf7,
                    (true, false) => 0xf6,
                    (false, true) => 0xeb,
                    (false, false) => 0xea,
                });
                self.c.simd_lane(if dbl { 0x21 } else { 0x1f }, 0);
                self.c.simd_lane(if dbl { 0x22 } else { 0x20 }, 0);
                self.c.local_set(d);
            }
            M::Maxpd | M::Minpd | M::Maxps | M::Minps => {
                self.src_vec(i, 1, 16)?;
                self.c.local_get(d);
                self.c.simd(match m {
                    M::Maxpd => 0xf7,
                    M::Minpd => 0xf6,
                    M::Maxps => 0xeb,
                    _ => 0xea,
                });
                self.c.local_set(d);
            }
            M::Paddb | M::Paddw | M::Paddd | M::Paddq | M::Psubb | M::Psubw | M::Psubd | M::Psubq => {
                self.c.local_get(d);
                self.src_vec(i, 1, 16)?;
                self.c.simd(match m {
                    M::Paddb => 0x6e,
                    M::Paddw => 0x8e,
                    M::Paddd => 0xae,
                    M::Paddq => 0xce,
                    M::Psubb => 0x71,
                    M::Psubw => 0x91,
                    M::Psubd => 0xb1,
                    _ => 0xd1,
                });
                self.c.local_set(d);
            }
            M::Psrlq | M::Psllq => {
                if i.op_kind(1) != OpKind::Immediate8 {
                    return Err("vector shift by a register".into());
                }
                let n = i.immediate8() as i32;
                if n >= 64 {
                    self.c.v128_const([0; 16]);
                } else {
                    self.c.local_get(d);
                    self.c.i32_const(n);
                    self.c.simd(if m == M::Psrlq { 0xcd } else { 0xcb });
                }
                self.c.local_set(d);
            }
            M::Psrldq | M::Pslldq => {
                if i.op_kind(1) != OpKind::Immediate8 {
                    return Err("byte shift count".into());
                }
                let n = (i.immediate8() as usize).min(16);
                let mut sh = [0u8; 16];
                for k in 0..16usize {
                    sh[k] = if m == M::Psrldq {
                        if k + n < 16 { (k + n) as u8 } else { 16 }
                    } else if k >= n {
                        (k - n) as u8
                    } else {
                        16
                    };
                }
                self.c.local_get(d);
                self.c.v128_const([0; 16]);
                self.c.simd_shuffle(sh);
                self.c.local_set(d);
            }
            M::Punpcklbw | M::Punpcklqdq | M::Punpckhqdq => {
                self.c.local_get(d);
                self.src_vec(i, 1, 16)?;
                let mut sh = [0u8; 16];
                match m {
                    M::Punpcklbw => {
                        for k in 0..8usize {
                            sh[2 * k] = k as u8;
                            sh[2 * k + 1] = 16 + k as u8;
                        }
                    }
                    M::Punpcklqdq => {
                        for k in 0..8usize {
                            sh[k] = k as u8;
                            sh[8 + k] = 16 + k as u8;
                        }
                    }
                    _ => {
                        for k in 0..8usize {
                            sh[k] = 8 + k as u8;
                            sh[8 + k] = 24 + k as u8;
                        }
                    }
                }
                self.c.simd_shuffle(sh);
                self.c.local_set(d);
            }
            M::Cmpsd => {
                if i.op_kind(2) != OpKind::Immediate8 {
                    return Err("cmpsd predicate".into());
                }
                let pred = i.immediate8() & 7;
                let a = self.tmp(Ty::F64);
                let b = self.tmp(Ty::F64);
                self.c.local_get(d);
                self.c.simd_lane(0x21, 0);
                self.c.local_set(a);
                if let Some(l) = self.src_ov {
                    self.c.local_get(l);
                    self.c.simd_lane(0x21, 0);
                } else if i.op_kind(1) == OpKind::Register {
                    let s = self.xmm_loc(i.op_register(1).number());
                    self.c.local_get(s);
                    self.c.simd_lane(0x21, 0);
                } else {
                    let off = self.mem_addr(i)?;
                    self.c.f64_load(off);
                }
                self.c.local_set(b);
                self.c.local_get(d);
                self.c.i32_const(0);
                let unord = |s: &mut Self| {
                    s.c.local_get(a);
                    s.c.local_get(a);
                    s.c.raw(&[0x62]);
                    s.c.local_get(b);
                    s.c.local_get(b);
                    s.c.raw(&[0x62]);
                    s.c.i32_or();
                };
                match pred {
                    0 | 1 | 2 | 4 | 5 | 6 => {
                        self.c.local_get(a);
                        self.c.local_get(b);
                        self.c.raw(&[match pred {
                            0 => 0x61,
                            1 | 5 => 0x63,
                            2 | 6 => 0x65,
                            _ => 0x62,
                        }]);
                        if pred == 5 || pred == 6 {
                            self.c.i32_eqz();
                        }
                    }
                    3 => unord(self),
                    _ => {
                        unord(self);
                        self.c.i32_eqz();
                    }
                }
                self.c.i32_sub();
                self.c.i64_extend_i32_s();
                self.c.simd_lane(0x1e, 0);
                self.c.local_set(d);
            }
            _ => return Err(format!("sse op {m:?}")),
        }
        Ok(())
    }

    /// One-operand mul, imul, div, idiv (32 and 64 bits).
    fn muldiv(&mut self, i: &Instruction, va: u64) -> R<()> {
        let _ = va;
        let m = i.mnemonic();
        let size = opsize0(i) as usize;
        if size != 4 && size != 8 {
            return Err(format!("{m:?} {size}-byte"));
        }
        if let Some(&id) = self.fl.def_of.get(&va) {
            let d = &self.fl.defs[id as usize];
            if d.need_save || d.mat != 0 {
                return Err("flags of mul/div are read".into());
            }
        }
        let rax = self.reg_loc(RAX);
        let rdx = self.reg_loc(RDX);
        let src = self.tmp(Ty::I64);
        self.get_op(i, 0, size)?;
        if size == 4 {
            // extend: unsigned for mul/div, signed for imul/idiv
            if matches!(m, Mnemonic::Idiv | Mnemonic::Imul) {
                self.c.i64_extend_i32_s();
            } else {
                self.c.i64_extend_i32_u();
            }
        }
        self.c.local_set(src);
        match (m, size) {
            (Mnemonic::Mul | Mnemonic::Imul, 4) => {
                let p = self.tmp(Ty::I64);
                if m == Mnemonic::Mul {
                    self.c.local_get(rax);
                    self.c.i64_const(0xffff_ffff);
                    self.c.i64_and();
                } else {
                    self.c.local_get(rax);
                    self.c.i64_extend32_s();
                }
                self.c.local_get(src);
                self.c.i64_mul();
                self.c.local_tee(p);
                self.c.i64_const(0xffff_ffff);
                self.c.i64_and();
                self.c.local_set(rax);
                self.c.local_get(p);
                self.c.i64_const(32);
                self.c.i64_shr_u();
                self.c.i64_const(0xffff_ffff);
                self.c.i64_and();
                self.c.local_set(rdx);
            }
            (Mnemonic::Mul | Mnemonic::Imul, _) => {
                let lo = self.tmp(Ty::I64);
                self.c.local_get(rax);
                self.c.local_get(src);
                self.c.i64_mul();
                self.c.local_set(lo);
                self.c.local_get(rax);
                self.c.local_get(src);
                self.c.call(if m == Mnemonic::Mul { self.cx.mulhu } else { self.cx.mulhs });
                self.c.local_set(rdx);
                self.c.local_get(lo);
                self.c.local_set(rax);
            }
            (Mnemonic::Div, 4) | (Mnemonic::Idiv, 4) => {
                let n = self.tmp(Ty::I64);
                let q = self.tmp(Ty::I64);
                self.c.local_get(rdx);
                self.c.i64_const(32);
                self.c.i64_shl();
                self.c.local_get(rax);
                self.c.i64_const(0xffff_ffff);
                self.c.i64_and();
                self.c.i64_or();
                self.c.local_set(n);
                self.c.local_get(n);
                self.c.local_get(src);
                if m == Mnemonic::Div {
                    self.c.i64_div_u();
                } else {
                    self.c.i64_div_s();
                }
                self.c.local_tee(q);
                // the quotient must fit 32 bits
                if m == Mnemonic::Div {
                    self.c.i64_const(32);
                    self.c.i64_shr_u();
                    self.c.i64_eqz();
                    self.c.i32_eqz();
                } else {
                    self.c.local_get(q);
                    self.c.i64_extend32_s();
                    self.c.i64_ne();
                }
                self.trap_if();
                self.c.local_get(n);
                self.c.local_get(src);
                if m == Mnemonic::Div {
                    self.c.i64_rem_u();
                } else {
                    self.c.i64_rem_s();
                }
                self.c.i64_const(0xffff_ffff);
                self.c.i64_and();
                self.c.local_set(rdx);
                self.c.local_get(q);
                self.c.i64_const(0xffff_ffff);
                self.c.i64_and();
                self.c.local_set(rax);
            }
            (Mnemonic::Div, _) => {
                // 128/64 division only when the high half is zero; otherwise stop
                self.c.local_get(rdx);
                self.c.i64_eqz();
                self.c.i32_eqz();
                self.trap_if();
                let q = self.tmp(Ty::I64);
                self.c.local_get(rax);
                self.c.local_get(src);
                self.c.i64_div_u();
                self.c.local_set(q);
                self.c.local_get(rax);
                self.c.local_get(src);
                self.c.i64_rem_u();
                self.c.local_set(rdx);
                self.c.local_get(q);
                self.c.local_set(rax);
            }
            _ => {
                // idiv r64: the high half must be the sign of the low half
                self.c.local_get(rdx);
                self.c.local_get(rax);
                self.c.i64_const(63);
                self.c.i64_shr_s();
                self.c.i64_ne();
                self.trap_if();
                let q = self.tmp(Ty::I64);
                self.c.local_get(rax);
                self.c.local_get(src);
                self.c.i64_div_s();
                self.c.local_set(q);
                self.c.local_get(rax);
                self.c.local_get(src);
                self.c.i64_rem_s();
                self.c.local_set(rdx);
                self.c.local_get(q);
                self.c.local_set(rax);
            }
        }
        Ok(())
    }

    // ---- control flow ----

    fn depth_of(&self, f: Frame) -> R<u32> {
        let n = self.frames.len();
        for k in (0..n).rev() {
            if self.frames[k] == f {
                return Ok((n - 1 - k) as u32);
            }
        }
        Err(format!("no frame {f:?}"))
    }

    fn emit_return(&mut self) {
        if let Some(xp) = self.xp {
            self.xflush(xp.own);
        }
        self.emit_return_nf();
    }

    fn emit_return_nf(&mut self) {
        for r in self.sig.results() {
            let l = self.reg_loc(r);
            self.c.local_get(l);
        }
        self.c.return_();
    }

    fn body(&mut self, b: usize) -> R<()> {
        self.pending_rsp = 0;
        let insns = self.f.blocks[b].insns.clone();
        let n = insns.len();
        for (k, &va) in insns.iter().enumerate() {
            let i = self.f.insn(va);
            // the terminator instruction is handled by the caller
            let is_term = k + 1 == n && matches!(self.f.blocks[b].term, Term::Jcc { .. } | Term::Jcz { .. } | Term::Ret | Term::Switch { .. } | Term::Trap)
                || (k + 1 == n && i.mnemonic() == Mnemonic::Jmp);
            if is_term {
                break;
            }
            self.cur_va = va;
            let r = self.lower_insn(va, k + 1 == n);
            if let Err(e) = r {
                if self.cx.census {
                    *self.stats.insn_errors.entry(e).or_insert(0) += 1;
                    self.release_tmps();
                    continue;
                }
                return Err(e);
            }
            self.release_tmps();
        }
        Ok(())
    }

    fn branch_is_inline(&self, from: usize, to: usize) -> bool {
        !(self.cfg.num[to] <= self.cfg.num[from] || self.cfg.is_merge[to])
    }

    fn branch(&mut self, from: usize, to: usize) -> R<()> {
        if let Some(pc) = self.dispatch_pc {
            self.flush_rsp();
            self.c.i32_const(to as i32);
            self.c.local_set(pc);
            let d = self.depth_of(Frame::Loop(usize::MAX))?;
            self.c.br(d);
            return Ok(());
        }
        if self.cfg.num[to] <= self.cfg.num[from] {
            self.flush_rsp();
            let d = self.depth_of(Frame::Loop(to))?;
            self.c.br(d);
            Ok(())
        } else if self.cfg.is_merge[to] {
            self.flush_rsp();
            if self.fall == Some(to) {
                return Ok(());
            }
            let d = self.depth_of(Frame::Block(to))?;
            self.c.br(d);
            Ok(())
        } else {
            self.do_tree(to)
        }
    }

    fn terminator(&mut self, b: usize) -> R<()> {
        let term = self.f.blocks[b].term.clone();
        self.xsite = None;
        if matches!(term, Term::Tail(_) | Term::TailIndirect | Term::TailImport(_)) {
            self.xsite = self.xp.map(|p| (p.own, 0));
        }
        match term {
            Term::Ret => {
                // (a `ret imm16` is the callee's RSP offset k = imm16 (decision 0261): the callers add it, see `Sig::rsp`)
                self.emit_return();
                Ok(())
            }
            Term::Trap => {
                self.c.unreachable();
                Ok(())
            }
            Term::Unsup(e) => {
                if self.cx.census {
                    *self.stats.insn_errors.entry(format!("terminator: {e}")).or_insert(0) += 1;
                    return Ok(());
                }
                Err(e)
            }
            Term::TailIndirect => {
                let va = *self.f.blocks[b].insns.last().unwrap();
                let i = *self.f.insn(va);
                self.stats.indirect_calls += 1;
                match i.op_kind(0) {
                    OpKind::Register => {
                        if i.op_register(0).size() != 8 {
                            return Err("indirect jmp register is not 64-bit".into());
                        }
                        self.get_reg(i.op_register(0))?;
                    }
                    OpKind::Memory => {
                        if i.memory_size().size() != 8 {
                            return Err("indirect jmp memory operand is not 64-bit".into());
                        }
                        let off = self.mem_addr(&i)?;
                        self.load(8, off);
                    }
                    _ => return Err("indirect jmp operand".into()),
                }
                if self.cx.dry {
                    self.c.drop();
                    return Ok(());
                }
                let tt = self.tmp(Ty::I64);
                if self.cx.c_mode {
                    self.c.local_set(tt);
                    if self.cx.progress_addr != 0 && self.f.demoted.contains(&va) {
                        // fle_progress_count -= 1: the dispatch below counts, the C path's plain jump to this target did not
                        self.c.i32_const(0);
                        self.c.i32_const(0);
                        self.c.i64_load(self.cx.progress_addr);
                        self.c.i64_const(1);
                        self.c.i64_sub();
                        self.c.i64_store(self.cx.progress_addr);
                    }
                    self.c_stub(self.cx.c_dispatch, Some(tt), None, &[1, 2, 8, 9, 0], &[0]);
                    self.emit_return_nf();
                    return Ok(());
                }
                self.c.local_tee(tt);
                self.c.i64_const(32);
                self.c.i64_shr_u();
                self.c.i32_wrap_i64();
                self.trap_if();
                self.flush_rsp();
                let rl = self.reg_loc(RSP);
                self.c.local_get(rl);
                for r in [1usize, 2, 8, 9, 0] {
                    let l = self.reg_loc(r);
                    self.c.local_get(l);
                }
                self.c.local_get(tt);
                self.c.i32_wrap_i64();
                self.c.i32_const(self.cx.text_lo as i32);
                self.c.i32_sub();
                self.c.call_indirect(self.cx.ui_ty, 0);
                let ra = self.reg_loc(RAX);
                self.c.local_set(ra);
                self.emit_return_nf();
                Ok(())
            }
            Term::TailImport(name) => {
                self.stats.import_calls += 1;
                if self.cx.c_mode {
                    let key = name.rsplit('!').next().unwrap_or(&name).to_string();
                    if !self.cx.c_imp.contains_key(&key) {
                        return Err(format!("import {key} is not modelled by the C runtime"));
                    }
                    if self.cx.dry {
                        return Ok(());
                    }
                    let fi = self.cx.c_imp[&key];
                    self.c_stub(fi, None, None, &[1, 2, 8, 9], &[0]);
                    self.emit_return_nf();
                    return Ok(());
                }
                if self.cx.dry {
                    return Ok(());
                }
                let fi = *self.cx.import_idx.get(&name).ok_or("import not declared")?;
                self.flush_rsp();
                let rl = self.reg_loc(RSP);
                self.c.local_get(rl);
                for r in [1usize, 2, 8, 9] {
                    let l = self.reg_loc(r);
                    self.c.local_get(l);
                }
                self.c.call(fi);
                let ra = self.reg_loc(RAX);
                self.c.local_set(ra);
                self.emit_return_nf();
                Ok(())
            }
            Term::Tail(g) => {
                self.call_to(g, None)?;
                self.emit_return_nf();
                Ok(())
            }
            Term::Jmp(t) => {
                self.flush_rsp();
                self.branch(b, t)
            }
            Term::Switch { cases, slot } => {
                let va = *self.f.blocks[b].insns.last().unwrap();
                let i = *self.f.insn(va);
                self.flush_rsp();
                let sel = self.tmp(Ty::I64);
                if slot {
                    if i.op_kind(0) != OpKind::Memory || i.memory_size().size() != 8 {
                        return Err("jump table: operand is not a 64-bit memory operand".into());
                    }
                    self.lea_addr(&i)?;
                } else {
                    if i.op_kind(0) != OpKind::Register || i.op_register(0).size() != 8 {
                        return Err("jump table: operand is not a 64-bit register".into());
                    }
                    self.get_reg(i.op_register(0))?;
                }
                self.c.local_set(sel);
                let c0 = cases[0].0;
                if self.dispatch_pc.is_some() {
                    for &(cv, tb) in &cases {
                        self.c.local_get(sel);
                        self.c.i64_const(cv as i64);
                        self.c.i64_eq();
                        self.c.if_();
                        self.frames.push(Frame::If);
                        self.branch(b, tb)?;
                        self.frames.pop();
                        self.c.end();
                    }
                    self.c.unreachable();
                    return Ok(());
                }
                let regular = slot && cases.len() >= 3 && cases.iter().enumerate().all(|(k, c)| c.0 == c0 + 8 * k as u64);
                self.stats.switches += 1;
                if regular {
                    self.stats.switch_br_table += 1;
                    let t = self.tmp(Ty::I64);
                    self.c.local_get(sel);
                    self.c.i64_const(c0 as i64);
                    self.c.i64_sub();
                    self.c.local_tee(t);
                    self.c.i64_const(7);
                    self.c.i64_and();
                    self.c.i32_wrap_i64();
                    self.trap_if();
                    self.c.local_get(t);
                    self.c.i64_const(3);
                    self.c.i64_shr_u();
                    self.c.i64_const(cases.len() as i64);
                    self.c.i64_ge_u();
                    self.trap_if();
                    self.c.local_get(t);
                    self.c.i64_const(3);
                    self.c.i64_shr_u();
                    self.c.i32_wrap_i64();
                    let mut ds = Vec::new();
                    for &(_, tb) in &cases {
                        ds.push(self.br_depth(b, tb)?);
                    }
                    let dflt = self.trap_depth();
                    self.c.br_table(&ds, dflt);
                } else {
                    for &(cv, tb) in &cases {
                        self.c.local_get(sel);
                        self.c.i64_const(cv as i64);
                        self.c.i64_eq();
                        let d = self.br_depth(b, tb)?;
                        self.c.br_if(d);
                    }
                    self.c.unreachable();
                }
                Ok(())
            }
            Term::Jcc { cc, taken, fall } => self.jcond(b, Cnd::Cc(cc), taken, fall),
            Term::Jcz { wide, taken, fall } => self.jcond(b, Cnd::Z { wide, neg: false }, taken, fall),
        }
    }

    /// A conditional terminator (`jcc`, `jecxz`/`jrcxz`): `cnd` holds when the branch to `taken` is made.
    fn jcond(&mut self, b: usize, cnd: Cnd, taken: usize, fall: usize) -> R<()> {
        if self.dispatch_pc.is_some() {
            let va = *self.f.blocks[b].insns.last().unwrap();
            self.flush_rsp();
            self.cond_t(va, &cnd)?;
            self.c.if_();
            self.frames.push(Frame::If);
            self.branch(b, taken)?;
            self.c.else_();
            self.branch(b, fall)?;
            self.frames.pop();
            self.c.end();
            return Ok(());
        }
        let va = *self.f.blocks[b].insns.last().unwrap();
        self.flush_rsp();
        if taken == fall {
            return self.branch(b, taken);
        }
        let omit_fall = self.fall == Some(fall) && !(self.cfg.num[fall] <= self.cfg.num[b]);
        let omit_taken = self.fall == Some(taken) && !(self.cfg.num[taken] <= self.cfg.num[b]);
        let tk_inline = self.branch_is_inline(b, taken);
        let fl_inline = self.branch_is_inline(b, fall);
        let saved_fall = self.fall;
        if omit_fall {
            // only the taken arm needs code
            self.cond_t(va, &cnd)?;
            if tk_inline {
                self.c.if_();
                self.frames.push(Frame::If);
                self.do_tree(taken)?;
                self.frames.pop();
                self.c.end();
            } else {
                let d = self.br_depth(b, taken)?;
                self.c.br_if(d);
            }
            return Ok(());
        }
        if omit_taken {
            self.cond_t(va, &cnd.neg())?;
            if fl_inline {
                self.c.if_();
                self.frames.push(Frame::If);
                self.do_tree(fall)?;
                self.frames.pop();
                self.c.end();
            } else {
                let d = self.br_depth(b, fall)?;
                self.c.br_if(d);
            }
            return Ok(());
        }
        if !tk_inline {
            self.cond_t(va, &cnd)?;
            let d = self.br_depth(b, taken)?;
            self.c.br_if(d);
            return self.branch(b, fall);
        }
        if !fl_inline {
            self.cond_t(va, &cnd.neg())?;
            let d = self.br_depth(b, fall)?;
            self.c.br_if(d);
            return self.branch(b, taken);
        }
        self.cond_t(va, &cnd)?;
        self.c.if_();
        self.frames.push(Frame::If);
        self.fall = saved_fall;
        self.do_tree(taken)?;
        self.c.else_();
        self.do_tree(fall)?;
        self.frames.pop();
        self.c.end();
        Ok(())
    }

    /// pushes the i32 truth of `cnd`: a condition code reads the flags, `jecxz`/`jrcxz` compare (e)cx with zero
    fn cond_t(&mut self, va: u64, cnd: &Cnd) -> R<()> {
        match *cnd {
            Cnd::Cc(cc) => self.cond(va, cc),
            Cnd::Z { wide, neg } => {
                if wide {
                    self.get_reg(Register::RCX)?;
                    self.c.i64_eqz();
                } else {
                    self.get_reg(Register::ECX)?;
                    self.c.i32_eqz();
                }
                if neg {
                    self.c.i32_eqz();
                }
                Ok(())
            }
        }
    }

    fn br_depth(&self, from: usize, to: usize) -> R<u32> {
        if self.cfg.num[to] <= self.cfg.num[from] {
            self.depth_of(Frame::Loop(to))
        } else {
            self.depth_of(Frame::Block(to))
        }
    }

    fn do_tree(&mut self, x: usize) -> R<()> {
        let mut ys: Vec<usize> = self.cfg.children[x].iter().copied().filter(|&y| self.cfg.is_merge[y]).collect();
        ys.sort_by(|a, b| self.cfg.num[*b].cmp(&self.cfg.num[*a]));
        if self.cfg.is_header[x] {
            self.c.loop_();
            self.frames.push(Frame::Loop(x));
            let saved = self.fall;
            self.node_within(x, &ys)?;
            self.fall = saved;
            self.frames.pop();
            self.c.end();
            Ok(())
        } else {
            self.node_within(x, &ys)
        }
    }

    fn node_within(&mut self, x: usize, ys: &[usize]) -> R<()> {
        match ys.split_first() {
            Some((&y, rest)) => {
                self.c.block();
                self.frames.push(Frame::Block(y));
                let saved = self.fall;
                self.fall = Some(y);
                self.node_within(x, rest)?;
                self.fall = saved;
                self.frames.pop();
                self.c.end();
                self.do_tree(y)
            }
            None => {
                self.body(x)?;
                self.terminator(x)
            }
        }
    }
}

fn isrep_pre(i: &Instruction) -> bool {
    i.has_rep_prefix() || i.has_repne_prefix()
}

/// edx of every CPUID leaf from 1 to 0x7fffffff (runtime/fle_cpuid.c FLE_CPUID_LEAF1_EDX: FPU, CX8, CMOV, MMX, FXSR, SSE, SSE2)
const CPUID_LEAF1_EDX: u32 = (1 << 0) | (1 << 8) | (1 << 15) | (1 << 23) | (1 << 24) | (1 << 25) | (1 << 26);

/// The condition of a conditional terminator.
#[derive(Clone, Copy)]
enum Cnd {
    Cc(ConditionCode),
    /// `jrcxz` (wide) / `jecxz`: the branch is made when (r|e)cx is zero; neg: the negation
    Z { wide: bool, neg: bool },
}

impl Cnd {
    fn neg(self) -> Cnd {
        match self {
            Cnd::Cc(cc) => Cnd::Cc(negate(cc)),
            Cnd::Z { wide, neg } => Cnd::Z { wide, neg: !neg },
        }
    }
}

fn negate(cc: ConditionCode) -> ConditionCode {
    use ConditionCode as C;
    match cc {
        C::o => C::no,
        C::no => C::o,
        C::b => C::ae,
        C::ae => C::b,
        C::e => C::ne,
        C::ne => C::e,
        C::be => C::a,
        C::a => C::be,
        C::s => C::ns,
        C::ns => C::s,
        C::p => C::np,
        C::np => C::p,
        C::l => C::ge,
        C::ge => C::l,
        C::le => C::g,
        C::g => C::le,
        C::None => C::None,
    }
}

fn setcc_cc(m: Mnemonic) -> Option<ConditionCode> {
    use ConditionCode as C;
    Some(match m {
        Mnemonic::Seta => C::a,
        Mnemonic::Setae => C::ae,
        Mnemonic::Setb => C::b,
        Mnemonic::Setbe => C::be,
        Mnemonic::Sete => C::e,
        Mnemonic::Setg => C::g,
        Mnemonic::Setge => C::ge,
        Mnemonic::Setl => C::l,
        Mnemonic::Setle => C::le,
        Mnemonic::Setne => C::ne,
        Mnemonic::Setno => C::no,
        Mnemonic::Setnp => C::np,
        Mnemonic::Setns => C::ns,
        Mnemonic::Seto => C::o,
        Mnemonic::Setp => C::p,
        Mnemonic::Sets => C::s,
        _ => return None,
    })
}

fn cmov_cc(m: Mnemonic) -> Option<ConditionCode> {
    use ConditionCode as C;
    Some(match m {
        Mnemonic::Cmova => C::a,
        Mnemonic::Cmovae => C::ae,
        Mnemonic::Cmovb => C::b,
        Mnemonic::Cmovbe => C::be,
        Mnemonic::Cmove => C::e,
        Mnemonic::Cmovg => C::g,
        Mnemonic::Cmovge => C::ge,
        Mnemonic::Cmovl => C::l,
        Mnemonic::Cmovle => C::le,
        Mnemonic::Cmovne => C::ne,
        Mnemonic::Cmovno => C::no,
        Mnemonic::Cmovnp => C::np,
        Mnemonic::Cmovns => C::ns,
        Mnemonic::Cmovo => C::o,
        Mnemonic::Cmovp => C::p,
        Mnemonic::Cmovs => C::s,
        _ => return None,
    })
}

/// Lowers one function. The body has the parameters of `sig` (rsp, then the registers of `sig.inr`).
pub fn lower(f: &Fun, cfg: &Cfg, fl: &FlagInfo, sig: Sig, cx: &Ctx<'_>, skip: &HashSet<u64>, stats: &mut LowerStats) -> R<Body> {
    let r = lower_fn(f, cfg, fl, sig, cx, skip, stats);
    crate::wasm::TLS_LOCAL.with(|c| c.set(u32::MAX));   // decision 0234 (mt3): the cached `__tls_base` local belongs to the function just lowered
    r
}

fn lower_fn(f: &Fun, cfg: &Cfg, fl: &FlagInfo, sig: Sig, cx: &Ctx<'_>, skip: &HashSet<u64>, stats: &mut LowerStats) -> R<Body> {
    let params = sig.params();
    let mut l = L {
        f,
        cfg,
        fl,
        sig,
        cx,
        skip,
        c: Code::new(),
        nparams: 1 + params.len() as u32,
        locals: Vec::new(),
        reg_local: [NONE; 16],
        xmm_local: [NONE; 16],
        frames: vec![Frame::Trap],
        tmp_used: Vec::new(),
        tmp_free: Vec::new(),
        def_loc: HashMap::new(),
        slot_of: if crate::locshare::LOCSHARE_OFF.load(std::sync::atomic::Ordering::Relaxed) {
            Vec::new()
        } else {
            let reachable: Vec<bool> = cfg.num.iter().map(|&n| n != usize::MAX).collect();
            let (slots, _) = crate::locshare::assign(f, &cfg.preds, &reachable, fl, |id| if fl.defs[id as usize].kind == Kind::BitTest { Ty::I32 } else { ty_of(fl.defs[id as usize].w as usize) });
            if crate::locshare::LOCSHARE_VERIFY.load(std::sync::atomic::Ordering::Relaxed) {
                if let Err(e) = crate::locshare::verify(f, &cfg.preds, fl, &slots) {
                    panic!("locshare: {:#x}: {e}", f.entry);
                }
            }
            slots
        },
        slot_loc: HashMap::new(),
        def_cin: HashMap::new(),
        mat_loc: [NONE; 6],
        pending_rsp: 0,
        cur_va: 0,
        xp: if cx.hybrid_x != 0 { cx.xplans.get(&f.entry) } else { None },
        xsite: None,
        mx_guard: NONE,
        mx_rcl: NONE,
        mx_bypass: false,
        src_ov: None,
        fall: None,
        dispatch_pc: None,
        stats,
    };
    l.reg_local[RSP] = 0;
    for (k, &r) in params.iter().enumerate() {
        l.reg_local[r] = 1 + k as u32;
    }
    if crate::wasm::TLS_ON.with(|c| c.get()) != 0 && !cx.dry {
        // decision 0234 (mt3, B-prime stage 1): `__tls_base` does not change while an instance runs; one local per function keeps it in a register instead of a `global.get` at every access of the register page
        let b = l.new_local(Ty::I32);
        l.c.tls_base_get();
        l.c.local_set(b);
        crate::wasm::TLS_LOCAL.with(|c| c.set(b));
    }
    if let Some(xp) = l.xp {
        l.xreload(xp.entry);
    }
    if l.mx_fid() && f.nodes.values().any(|n| matches!(n, crate::ir::Node::Insn(i) if mx_guarded(i.mnemonic()))) {
        l.mx_guard = l.new_local(Ty::I32);
        l.mx_rcl = l.new_local(Ty::I32);
        l.mx_guard_refresh();
    }
    l.c.block();
    if cfg.reducible {
        l.do_tree(0)?;
    } else {
        // irreducible: one loop, one block per basic block, and a br_table on the number of the next block
        let n = f.blocks.len();
        let pc = l.new_local(Ty::I32);
        l.dispatch_pc = Some(pc);
        l.stats.dispatch_functions += 1;
        l.c.loop_();
        l.frames.push(Frame::Loop(usize::MAX));
        for k in (0..n).rev() {
            l.c.block();
            l.frames.push(Frame::Block(k));
        }
        l.c.local_get(pc);
        let targets: Vec<u32> = (0..n as u32).collect();
        l.c.br_table(&targets, (n - 1) as u32);
        for k in 0..n {
            l.frames.pop();
            l.c.end();
            l.body(k)?;
            l.terminator(k)?;
        }
        l.frames.pop();
        l.c.end();
    }
    l.c.end();
    l.c.unreachable();
    // run-length encode the locals
    let mut runs: Vec<(u32, Ty)> = Vec::new();
    for &t in &l.locals {
        match runs.last_mut() {
            Some((n, rt)) if *rt == t => *n += 1,
            _ => runs.push((1, t)),
        }
    }
    Ok(Body { locals: runs, code: l.c })
}
