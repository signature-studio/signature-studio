//! Minimal 1:1 translation of decoded x86-64 instructions to C.
//!
//! Scope: integer ALU with flags, shifts, mul/imul, setcc, cmov, mov, lea,
//! push, pop, ret, direct call, direct jmp, jcc. Everything else becomes
//! `fle_unsupported(VA)`.
//!
//! Model:
//! - registers: `uint64_t R[16]`, indexed by the 64-bit register number
//!   (RAX=0 .. R15=15). 32-bit writes zero-extend; 8/16-bit writes merge.
//! - memory: identity map. A guest VA is used directly as a host address
//!   (`fle_ld` / `fle_st`).
//! - flags: `fle_eflags FL`, updated eagerly by the helpers in `fle_flags.h`.
//!   A flag the SDM leaves undefined is set to `FLE_UNDEF`, and reading it aborts.
//!   jcc, setcc and cmov read FL through `fle_cc`.
//! - functions: `void f_<hexVA>(void)`. Each function is traversed on its own,
//!   so a block may be copied into several functions. A transfer to another
//!   function entry is a tail call (`f_T(); return;`).
//! - call: pushes the return VA, calls `f_T()`, then pops the return VA. `ret`
//!   is `return;` and does not pop.

use fe_decode::{Analysis, Decoded, Resolved, Text};
use fe_pe::Pe;
use iced_x86::{ConditionCode, FlowControl, Instruction, Mnemonic, OpKind, Register};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;

mod cpuid;
mod dispatch;
mod flaglive;
pub mod hle;
pub mod prune;
pub mod swap;
pub mod hook;
pub mod site_override;
pub mod imports_table;
mod indirect;
mod insn_int;
mod int_ext;
mod int_more;
mod int_more2;
mod jumptab;
mod mmx;
mod mxcsr;
mod sse_int2;
mod sse_more;
mod sse_pack;
mod sse_scalar;
mod sse_shuf;
mod sse_ssse3;
mod string;
mod tailjmp;
mod vex;
mod x87;
mod x87ctl;

const RSP: usize = 4;

#[derive(Debug, Default)]
pub struct Stats {
    pub closure_insns: usize,
    pub functions: usize,
    /// Instruction occurrences emitted, counting copies in several functions.
    pub emitted_insns: usize,
    pub bad_nodes: usize,
    pub tail_calls: usize,
    pub gotos: usize,
    pub handled: BTreeMap<&'static str, usize>,
    pub unsupported: BTreeMap<String, usize>,
    pub unsupported_total: usize,
    pub c_lines: usize,
    pub c_bytes: usize,
    /// Rows of `fle_resp_table` in the output (0 when GetProcAddress is not in the closure).
    pub resp_entries: usize,
    /// `jmp` with a register operand, by outcome: the handler name, or `unsupported`. One count per
    /// emitted occurrence (a block copied into several functions counts in each).
    pub register_jmps: BTreeMap<&'static str, usize>,
    /// Closure level: distinct `jmp reg` sites reached, and annotated jump tables accepted.
    pub register_jmp_sites: usize,
    pub jumptabs: usize,
    /// Of `jumptabs`, the MSVC switch tables with a proven base (decision 0089).
    pub msvc_tables: usize,
    pub jumptab_rejected: Vec<(u64, String)>,
    /// MSVC switch idioms (decision 0089) that were not resolved, with the first failing condition.
    pub msvc_rejected: Vec<(u64, String)>,
    /// Landings of the guest's `setjmp` (decision 0100) that are function entries: each is emitted as the landing wrapper.
    pub setjmp_landings: usize,
    /// Inline hook points (decision 0105): function entries that got the guard and are in the module's table (0 without `--hook-points`).
    pub hook_points: usize,
    /// Call-site memory overrides (decision 0120): sites of the overrides file, each with a guard in front of its instruction (0 without `--site-overrides`).
    pub site_overrides: usize,
    /// Decision 0162: HLE entries (function entries that got the guard of the host implementation; 0 without `--hle-entries`).
    pub hle_entries: usize,
    /// Decision 0160: swapped entries (function entries whose body is a stub for the host implementation; 0 without `--swap-entries`).
    pub swap_entries: usize,
    /// Decision 0168: pruned functions listed (`--prune-list`), of them stubs in the output (kept code reaches them), exports and seeds dropped, listed
    /// entries that a conditional branch of kept code targets (their bodies stay as blocks there); 0 without the option.
    pub pruned_listed: usize,
    pub pruned_stubs: usize,
    pub pruned_seeds_dropped: usize,
    pub pruned_cond_targets: usize,
    /// Decision 0132: functions that got the SEH entry wrapper (a landing of the guest's exception handler inside them) and the landings (function entries).
    pub seh_wrapped: usize,
    pub seh_landings: usize,
    /// Decision 0155 (`--outline-shared`): shared bodies made function entries of their own (0 without the option).
    pub outlined_heads: usize,
    /// Instructions of the closure that more than one function entry reaches, before and after the promotion (0 without the option).
    pub shared_insns_before: usize,
    pub shared_insns_after: usize,
    /// Heads that `--outline-shared` did not promote because they lie on a cycle of tail calls (their regions stay shared).
    pub outline_banned: usize,
    /// `--dead-flags`: instructions that write flags, of them with no live write, with some dead write only; those emitted with `fle_nf_*`; fixed-point rounds.
    pub flag_writers: usize,
    pub flag_dead_full: usize,
    pub flag_dead_part: usize,
    pub flag_substituted: usize,
    pub flag_rounds: usize,
}

/// Inputs of the generation that are data of the module (the lists of entries). Decision 0155, 0243: an instruction that more than one function entry reaches is emitted once, as an entry of its own
/// (outlining of shared bodies: the first instruction of every shared region becomes a function entry of the C output, not of the dispatch table, and the jumps into it are tail calls `f_H(); return;`),
/// and an instruction whose flag writes nothing reads is emitted with the `fle_nf_*` helpers of fle_flags_nf.h (flag liveness over the closure, flaglive.rs). Both always apply, except where the module
/// cannot take them: the swap stubs (`swap`) are not combined with either, and the SEH entry wrappers (`SehOptions`) are not combined with the outlining (a `_setjmp` in its own C frame cannot be a tail-call target).
#[derive(Debug, Default, Clone)]
pub struct GenOptions {
    /// Decision 0162 (`--hle-entries`; the mechanism is decision 0151's): function entries whose first instruction gets the guard of a host implementation (`hle`); empty (the default) leaves the output byte for byte as it was.
    pub hle: Vec<hle::Entry>,
    /// Decision 0160 (`--swap-entries`): function entries whose body is not lifted and whose callees are not followed; a stub calls the host implementation `fle_swap_<name>` (`swap`); empty (the default) leaves the output byte for byte as it was.
    pub swap: Vec<swap::Entry>,
    /// Decision 0168 (`--prune-list`): function entries that must not run; kept code that reaches one gets a stub that stops by name (`fle_pruned`), nothing
    /// else of them is generated; empty (the default) leaves the output byte for byte as it was.
    pub prune: Vec<prune::Entry>,
}

/// Decision 0132 (`--seh-handler`): the functions that have an except landing of the guest's exception handler (function start VA -> landings).
/// Each such function gets the entry wrapper (`fle_seh_enter`, a `_setjmp` in its own C frame, `fle_seh_land`; its body is `fb_<va>`), and the landings must be
/// function entries of the closure (the caller adds them to the seeds). The default (no function) leaves the output byte for byte as it was.
#[derive(Debug, Default, Clone)]
pub struct SehOptions {
    pub funcs: BTreeMap<u64, Vec<u64>>,
}

enum Node {
    Insn(Instruction),
    /// Undecodable, or outside `.text`.
    Bad,
}

enum Ctl {
    /// Falls through to the next instruction.
    Seq,
    Ret,
    Jmp(u64),
    /// Target and condition (tttn, the low nibble of the Jcc opcode).
    Jcc(u64, u32),
}

struct Lifted {
    body: String,
    ctl: Ctl,
    handler: &'static str,
}

struct Cx<'a> {
    f: u64,
    nodes: &'a BTreeMap<u64, Node>,
    entries: &'a BTreeSet<u64>,
    /// Entries that are the landing of a call of the guest's `setjmp` (decision 0100).
    landings: &'a BTreeSet<u64>,
}

struct Out<'a, W: Write> {
    w: &'a mut W,
    lines: usize,
    bytes: usize,
}

impl<W: Write> Out<'_, W> {
    fn put(&mut self, s: &str) -> Result<(), String> {
        self.w.write_all(s.as_bytes()).map_err(|e| e.to_string())?;
        self.lines += s.matches('\n').count();
        self.bytes += s.len();
        Ok(())
    }
}

fn is_near(i: &Instruction) -> bool {
    matches!(i.op_kind(0), OpKind::NearBranch16 | OpKind::NearBranch32 | OpKind::NearBranch64)
}

fn is_imm(k: OpKind) -> bool {
    matches!(
        k,
        OpKind::Immediate8
            | OpKind::Immediate8to16
            | OpKind::Immediate8to32
            | OpKind::Immediate8to64
            | OpKind::Immediate16
            | OpKind::Immediate32
            | OpKind::Immediate32to64
            | OpKind::Immediate64
    )
}

fn need(c: bool, msg: &str) -> Result<(), String> {
    if c {
        Ok(())
    } else {
        Err(msg.to_string())
    }
}

fn reg_num(r: Register) -> Result<usize, String> {
    if !r.is_gpr() {
        return Err("non-GPR register".into());
    }
    Ok(r.full_register().number())
}

/// AH, CH, DH, BH (the REX-less 8-bit encodings 4..7): bits 8..15 of RAX, RCX, RDX, RBX (decision 0047).
fn is_high_byte(r: Register) -> bool {
    matches!(r, Register::AH | Register::CH | Register::DH | Register::BH)
}

fn reg_read(r: Register) -> Result<String, String> {
    let n = reg_num(r)?;
    if is_high_byte(r) {
        return Ok(format!("((uint8_t)(R[{n}] >> 8))"));
    }
    Ok(match r.size() {
        8 => format!("R[{n}]"),
        4 => format!("((uint32_t)R[{n}])"),
        2 => format!("((uint16_t)R[{n}])"),
        _ => format!("((uint8_t)R[{n}])"),
    })
}

fn reg_write(r: Register, v: &str) -> Result<String, String> {
    let n = reg_num(r)?;
    if is_high_byte(r) {
        return Ok(format!("R[{n}] = (R[{n}] & ~UINT64_C(0xff00)) | ((uint64_t)(uint8_t)({v}) << 8);"));
    }
    Ok(match r.size() {
        8 => format!("R[{n}] = {v};"),
        4 => format!("R[{n}] = (uint32_t)({v});"),
        2 => format!("R[{n}] = (R[{n}] & ~UINT64_C(0xffff)) | (uint16_t)({v});"),
        _ => format!("R[{n}] = (R[{n}] & ~UINT64_C(0xff)) | (uint8_t)({v});"),
    })
}

fn imm_lit(v: u64, size: usize) -> String {
    let m = if size >= 8 { u64::MAX } else { (1u64 << (size * 8)) - 1 };
    format!("UINT64_C({:#x})", v & m)
}

fn op_size(i: &Instruction, n: u32) -> Result<usize, String> {
    match i.op_kind(n) {
        OpKind::Register => {
            let r = i.op_register(n);
            reg_num(r)?;
            Ok(r.size())
        }
        OpKind::Memory => Ok(i.memory_size().size()),
        _ => Err("immediate operand has no size".into()),
    }
}

/// Effective address as a C expression. Never loads from memory.
fn ea(i: &Instruction) -> Result<String, String> {
    if matches!(i.memory_segment(), Register::FS | Register::GS) {
        return Err("FS/GS segment".into());
    }
    if i.is_ip_rel_memory_operand() {
        return Ok(format!("UINT64_C({:#x})", i.ip_rel_memory_address()));
    }
    let mut terms: Vec<String> = Vec::new();
    let base = i.memory_base();
    if base != Register::None {
        need(base.size() == 8, "32-bit addressing")?;
        terms.push(reg_read(base)?);
    }
    let idx = i.memory_index();
    if idx != Register::None {
        need(idx.size() == 8, "32-bit addressing")?;
        terms.push(format!("({}*{})", reg_read(idx)?, i.memory_index_scale()));
    }
    let d = i.memory_displacement64();
    if d != 0 || terms.is_empty() {
        terms.push(format!("UINT64_C({d:#x})"));
    }
    Ok(format!("({})", terms.join(" + ")))
}

fn op_read(i: &Instruction, n: u32, size: usize) -> Result<String, String> {
    let k = i.op_kind(n);
    if k == OpKind::Register {
        reg_read(i.op_register(n))
    } else if k == OpKind::Memory {
        Ok(format!("fle_ld({}, {size})", ea(i)?))
    } else if is_imm(k) {
        Ok(imm_lit(i.immediate(n), size))
    } else {
        Err("operand kind".into())
    }
}

fn op_write(i: &Instruction, n: u32, size: usize, v: &str) -> Result<String, String> {
    match i.op_kind(n) {
        OpKind::Register => reg_write(i.op_register(n), v),
        OpKind::Memory => Ok(format!("fle_st({}, {size}, {v});", ea(i)?)),
        _ => Err("operand kind".into()),
    }
}

/// Read-modify-write of operand 0. `f` takes the C expression for the old value
/// and returns the C expression for the new value. The `fle_op_*` helper it
/// calls also updates FL.
fn rmw_flags(i: &Instruction, sz: usize, f: &dyn Fn(&str) -> String) -> Result<String, String> {
    match i.op_kind(0) {
        OpKind::Register => {
            let d = i.op_register(0);
            let rd = reg_read(d)?;
            let w = reg_write(d, "t")?;
            Ok(format!("{{ uint64_t t = {}; {w} }}", f(&rd)))
        }
        OpKind::Memory => {
            let e = ea(i)?;
            if i.has_lock_prefix() {
                // A LOCK-prefixed read-modify-write is one atomic operation (runtime/fle_rmw.h): a compare-and-swap loop; the flags are those of the pass that swapped.
                return Ok(format!("{{ uint64_t a = {e}; uint64_t o, t; do {{ o = fle_ld(a, {sz}); t = {}; }} while (fle_cas(a, {sz}, o, t) != o); }}", f("o")));
            }
            Ok(format!("{{ uint64_t a = {e}; uint64_t t = {}; fle_st(a, {sz}, t); }}", f(&format!("fle_ld(a, {sz})"))))
        }
        _ => Err("operand kind".into()),
    }
}

/// Flags only (cmp, test). The result of `f` is discarded.
fn flags_only(i: &Instruction, sz: usize, f: &dyn Fn(&str) -> String) -> Result<String, String> {
    match i.op_kind(0) {
        OpKind::Register => Ok(format!("{{ (void)({}); }}", f(&reg_read(i.op_register(0))?))),
        OpKind::Memory => {
            let e = ea(i)?;
            Ok(format!("{{ uint64_t a = {e}; (void)({}); }}", f(&format!("fle_ld(a, {sz})"))))
        }
        _ => Err("operand kind".into()),
    }
}

/// Maps a condition code to the tttn nibble of Jcc / SETcc / CMOVcc.
fn cc_bits(c: ConditionCode) -> Result<u32, String> {
    Ok(match c {
        ConditionCode::o => 0x0,
        ConditionCode::no => 0x1,
        ConditionCode::b => 0x2,
        ConditionCode::ae => 0x3,
        ConditionCode::e => 0x4,
        ConditionCode::ne => 0x5,
        ConditionCode::be => 0x6,
        ConditionCode::a => 0x7,
        ConditionCode::s => 0x8,
        ConditionCode::ns => 0x9,
        ConditionCode::p => 0xa,
        ConditionCode::np => 0xb,
        ConditionCode::l => 0xc,
        ConditionCode::ge => 0xd,
        ConditionCode::le => 0xe,
        ConditionCode::g => 0xf,
        ConditionCode::None => return Err("no condition code (loop or jrcxz)".into()),
    })
}

fn is_setcc(m: Mnemonic) -> bool {
    matches!(
        m,
        Mnemonic::Seta
            | Mnemonic::Setae
            | Mnemonic::Setb
            | Mnemonic::Setbe
            | Mnemonic::Sete
            | Mnemonic::Setg
            | Mnemonic::Setge
            | Mnemonic::Setl
            | Mnemonic::Setle
            | Mnemonic::Setne
            | Mnemonic::Setno
            | Mnemonic::Setnp
            | Mnemonic::Setns
            | Mnemonic::Seto
            | Mnemonic::Setp
            | Mnemonic::Sets
    )
}

fn is_cmov(m: Mnemonic) -> bool {
    matches!(
        m,
        Mnemonic::Cmova
            | Mnemonic::Cmovae
            | Mnemonic::Cmovb
            | Mnemonic::Cmovbe
            | Mnemonic::Cmove
            | Mnemonic::Cmovg
            | Mnemonic::Cmovge
            | Mnemonic::Cmovl
            | Mnemonic::Cmovle
            | Mnemonic::Cmovne
            | Mnemonic::Cmovno
            | Mnemonic::Cmovnp
            | Mnemonic::Cmovns
            | Mnemonic::Cmovo
            | Mnemonic::Cmovp
            | Mnemonic::Cmovs
    )
}

fn lift_insn(i: &Instruction, va: u64, entries: &BTreeSet<u64>, resolved: &BTreeMap<u64, Resolved>) -> Result<Lifted, String> {
    if let Some(r) = int_more2::lift(i, entries) {
        let (handler, body) = r?;
        return Ok(Lifted { body, ctl: Ctl::Seq, handler });
    }
    if i.flow_control() == FlowControl::ConditionalBranch {
        need(is_near(i), "non-near conditional branch")?;
        let cc = cc_bits(i.condition_code())?;
        return Ok(Lifted { body: String::new(), ctl: Ctl::Jcc(i.near_branch_target(), cc), handler: "jcc" });
    }
    let n = i.op_count();
    let m = i.mnemonic();
    if let Some(r) = insn_int::lift(i) {
        let (handler, body) = r?;
        return Ok(Lifted { body, ctl: Ctl::Seq, handler });
    }
    if let Some(r) = int_ext::lift(i) {
        let (handler, body) = r?;
        return Ok(Lifted { body, ctl: Ctl::Seq, handler });
    }
    if let Some(r) = int_more::lift(i) {
        let (handler, body) = r?;
        return Ok(Lifted { body, ctl: Ctl::Seq, handler });
    }
    if let Some(r) = sse_more::lift(i) {
        let (handler, body) = r?;
        return Ok(Lifted { body, ctl: Ctl::Seq, handler });
    }
    if let Some(r) = sse_scalar::lift(i) {
        let (handler, body) = r?;
        return Ok(Lifted { body, ctl: Ctl::Seq, handler });
    }
    if let Some(r) = sse_pack::lift(i) {
        let (handler, body) = r?;
        return Ok(Lifted { body, ctl: Ctl::Seq, handler });
    }
    if let Some(r) = sse_int2::lift(i) {
        let (handler, body) = r?;
        return Ok(Lifted { body, ctl: Ctl::Seq, handler });
    }
    if let Some(r) = sse_shuf::lift(i) {
        let (handler, body) = r?;
        return Ok(Lifted { body, ctl: Ctl::Seq, handler });
    }
    if let Some(r) = sse_ssse3::lift(i) {
        let (handler, body) = r?;
        return Ok(Lifted { body, ctl: Ctl::Seq, handler });
    }
    if let Some(r) = mmx::lift(i) {
        let (handler, body) = r?;
        return Ok(Lifted { body, ctl: Ctl::Seq, handler });
    }
    if let Some(r) = vex::lift(i) {
        let (handler, body) = r?;
        return Ok(Lifted { body, ctl: Ctl::Seq, handler });
    }
    if let Some(r) = string::lift(i) {
        let (handler, body) = r?;
        return Ok(Lifted { body, ctl: Ctl::Seq, handler });
    }
    if let Some(r) = mxcsr::lift(i) {
        let (handler, body) = r?;
        return Ok(Lifted { body, ctl: Ctl::Seq, handler });
    }
    if let Some(r) = cpuid::lift(i) {
        let (handler, body) = r?;
        return Ok(Lifted { body, ctl: Ctl::Seq, handler });
    }
    if let Some(r) = x87ctl::lift(i) {
        let (handler, body) = r?;
        return Ok(Lifted { body, ctl: Ctl::Seq, handler });
    }
    if let Some(r) = x87::lift(i) {
        let (handler, body) = r?;
        return Ok(Lifted { body, ctl: Ctl::Seq, handler });
    }
    let seq = |body: String, handler: &'static str| Ok(Lifted { body, ctl: Ctl::Seq, handler });
    match m {
        Mnemonic::Mov => {
            need(n == 2, "mov operand count")?;
            let sz = op_size(i, 0)?;
            let src = op_read(i, 1, sz)?;
            seq(op_write(i, 0, sz, &src)?, "mov")
        }
        Mnemonic::Add | Mnemonic::Adc | Mnemonic::Sub | Mnemonic::Sbb | Mnemonic::And | Mnemonic::Or | Mnemonic::Xor => {
            need(n == 2, "alu operand count")?;
            let sz = op_size(i, 0)?;
            let w = sz * 8;
            let src = op_read(i, 1, sz)?;
            let (h, body) = match m {
                Mnemonic::Add => ("add", rmw_flags(i, sz, &|o| format!("fle_op_add({w}, {o}, ({src}), 0)"))?),
                Mnemonic::Adc => {
                    ("adc", rmw_flags(i, sz, &|o| format!("fle_op_add({w}, {o}, ({src}), fle_rd(FL.cf))"))?)
                }
                Mnemonic::Sub => ("sub", rmw_flags(i, sz, &|o| format!("fle_op_sub({w}, {o}, ({src}), 0)"))?),
                Mnemonic::Sbb => {
                    ("sbb", rmw_flags(i, sz, &|o| format!("fle_op_sub({w}, {o}, ({src}), fle_rd(FL.cf))"))?)
                }
                Mnemonic::And => ("and", rmw_flags(i, sz, &|o| format!("fle_op_logic({w}, ({o}) & ({src}))"))?),
                Mnemonic::Or => ("or", rmw_flags(i, sz, &|o| format!("fle_op_logic({w}, ({o}) | ({src}))"))?),
                _ => ("xor", rmw_flags(i, sz, &|o| format!("fle_op_logic({w}, ({o}) ^ ({src}))"))?),
            };
            seq(body, h)
        }
        Mnemonic::Cmp | Mnemonic::Test => {
            need(n == 2, "cmp/test operand count")?;
            let sz = op_size(i, 0)?;
            let w = sz * 8;
            let src = op_read(i, 1, sz)?;
            if m == Mnemonic::Cmp {
                seq(flags_only(i, sz, &|o| format!("fle_op_sub({w}, {o}, ({src}), 0)"))?, "cmp")
            } else {
                seq(flags_only(i, sz, &|o| format!("fle_op_logic({w}, ({o}) & ({src}))"))?, "test")
            }
        }
        Mnemonic::Inc | Mnemonic::Dec | Mnemonic::Neg | Mnemonic::Not => {
            need(n == 1, "unary operand count")?;
            let sz = op_size(i, 0)?;
            let w = sz * 8;
            let (h, body) = match m {
                Mnemonic::Inc => ("inc", rmw_flags(i, sz, &|o| format!("fle_op_inc({w}, {o})"))?),
                Mnemonic::Dec => ("dec", rmw_flags(i, sz, &|o| format!("fle_op_dec({w}, {o})"))?),
                Mnemonic::Neg => ("neg", rmw_flags(i, sz, &|o| format!("fle_op_neg({w}, {o})"))?),
                _ => ("not", rmw_flags(i, sz, &|o| format!("fle_op_not({w}, {o})"))?),
            };
            seq(body, h)
        }
        Mnemonic::Shl | Mnemonic::Sal | Mnemonic::Shr | Mnemonic::Sar => {
            need(n == 1 || n == 2, "shift operand count")?;
            let sz = op_size(i, 0)?;
            let w = sz * 8;
            let cnt = if n == 2 { op_read(i, 1, 1)? } else { "1".to_string() };
            let (h, body) = match m {
                Mnemonic::Shl | Mnemonic::Sal => ("shl", rmw_flags(i, sz, &|o| format!("fle_op_shl({w}, {o}, ({cnt}))"))?),
                Mnemonic::Shr => ("shr", rmw_flags(i, sz, &|o| format!("fle_op_shr({w}, {o}, ({cnt}))"))?),
                _ => ("sar", rmw_flags(i, sz, &|o| format!("fle_op_sar({w}, {o}, ({cnt}))"))?),
            };
            seq(body, h)
        }
        Mnemonic::Imul => match n {
            2 => {
                need(i.op_kind(0) == OpKind::Register, "imul destination must be a register")?;
                let sz = op_size(i, 0)?;
                need(sz >= 2, "imul 8-bit form")?;
                let w = sz * 8;
                let src = op_read(i, 1, sz)?;
                seq(rmw_flags(i, sz, &|o| format!("fle_op_imul({w}, {o}, ({src}))"))?, "imul")
            }
            3 => {
                need(i.op_kind(0) == OpKind::Register, "imul destination must be a register")?;
                let sz = op_size(i, 0)?;
                need(sz >= 2, "imul 8-bit form")?;
                let w = sz * 8;
                let a = op_read(i, 1, sz)?;
                let b = op_read(i, 2, sz)?;
                seq(op_write(i, 0, sz, &format!("fle_op_imul({w}, ({a}), ({b}))"))?, "imul")
            }
            _ => Err("imul with one operand (implicit RDX:RAX) not modelled".into()),
        },
        Mnemonic::Mul => {
            need(n == 1, "mul operand count")?;
            let sz = op_size(i, 0)?;
            let w = sz * 8;
            let src = op_read(i, 0, sz)?;
            let body = match sz {
                1 => format!(
                    "{{ uint64_t lo; uint64_t hi = fle_op_mul({w}, R[0], ({src}), &lo); R[0] = (R[0] & ~UINT64_C(0xffff)) | (uint16_t)((hi << 8) | lo); }}"
                ),
                2 => format!(
                    "{{ uint64_t lo; uint64_t hi = fle_op_mul({w}, R[0], ({src}), &lo); R[0] = (R[0] & ~UINT64_C(0xffff)) | lo; R[2] = (R[2] & ~UINT64_C(0xffff)) | hi; }}"
                ),
                4 => format!(
                    "{{ uint64_t lo; uint64_t hi = fle_op_mul({w}, R[0], ({src}), &lo); R[0] = (uint32_t)lo; R[2] = (uint32_t)hi; }}"
                ),
                _ => format!("{{ uint64_t lo; uint64_t hi = fle_op_mul({w}, R[0], ({src}), &lo); R[0] = lo; R[2] = hi; }}"),
            };
            seq(body, "mul")
        }
        _ if is_setcc(m) => {
            need(n == 1 && op_size(i, 0)? == 1, "setcc operand form")?;
            let cc = cc_bits(i.condition_code())?;
            seq(op_write(i, 0, 1, &format!("(uint64_t)(fle_cc({cc}) != 0)"))?, "setcc")
        }
        _ if is_cmov(m) => {
            need(n == 2 && i.op_kind(0) == OpKind::Register, "cmov destination must be a register")?;
            let d = i.op_register(0);
            need(d.size() >= 2, "cmov 8-bit form")?;
            let cc = cc_bits(i.condition_code())?;
            let src = op_read(i, 1, d.size())?;
            let rd = reg_read(d)?;
            let w = reg_write(d, "t")?;
            seq(format!("{{ uint64_t s = {src}; uint64_t t = fle_cc({cc}) ? s : ({rd}); {w} }}"), "cmov")
        }
        Mnemonic::Lea => {
            need(n == 2 && i.op_kind(1) == OpKind::Memory, "lea operand form")?;
            let sz = op_size(i, 0)?;
            need(sz == 8 || sz == 4, "lea destination size")?;
            let e = ea(i)?;
            seq(op_write(i, 0, sz, &e)?, "lea")
        }
        Mnemonic::Push => {
            need(n == 1, "push operand count")?;
            let sz = match i.op_kind(0) {
                OpKind::Register => i.op_register(0).size(),
                OpKind::Memory => i.memory_size().size(),
                _ => 8,
            };
            need(sz == 8, "non-64-bit push")?;
            let v = op_read(i, 0, 8)?;
            seq(format!("{{ uint64_t t = {v}; R[{RSP}] -= 8; fle_st(R[{RSP}], 8, t); }}"), "push")
        }
        Mnemonic::Pop => {
            need(n == 1, "pop operand count")?;
            let sz = match i.op_kind(0) {
                OpKind::Register => i.op_register(0).size(),
                OpKind::Memory => i.memory_size().size(),
                _ => 0,
            };
            need(sz == 8, "non-64-bit pop")?;
            let w = op_write(i, 0, 8, "t")?;
            seq(format!("{{ uint64_t t = fle_ld(R[{RSP}], 8); R[{RSP}] += 8; {w} }}"), "pop")
        }
        Mnemonic::Ret => {
            // `ret imm16` (decisions 0102 W9 and 0103 D7): pops the return address, then imm16 more bytes of the stack. The caller's lifted
            // `call` already adds the 8 bytes of the return address after the callee returns, so the callee adds the imm16 part here.
            // `ret 0` (MSVC writes it for some functions) pops nothing more and is the same as `ret`, with the same handler.
            if n == 0 {
                Ok(Lifted { body: "return;".into(), ctl: Ctl::Ret, handler: "ret" })
            } else {
                need(n == 1 && i.op_kind(0) == OpKind::Immediate16, "ret operand")?;
                let pop = i.immediate16();
                if pop == 0 {
                    Ok(Lifted { body: "return;".into(), ctl: Ctl::Ret, handler: "ret" })
                } else {
                    Ok(Lifted { body: format!("R[{RSP}] += {pop}; return;"), ctl: Ctl::Ret, handler: "ret_imm16" })
                }
            }
        }
        Mnemonic::Call => {
            if !is_near(i) {
                return match indirect::call(i, va, entries, resolved) {
                    Err(e) if e == "indirect target unresolved" => dispatch::call(i, va),
                    r => r,
                };
            }
            let t = i.near_branch_target();
            need(entries.contains(&t), "call target is not a function entry")?;
            let next = va + i.len() as u64;
            seq(
                format!(
                    "{{ R[{RSP}] -= 8; fle_st(R[{RSP}], 8, UINT64_C({next:#x})); f_{t:x}(); R[{RSP}] += 8; }}"
                ),
                "call",
            )
        }
        Mnemonic::Jmp => {
            if !is_near(i) {
                if let Some(Resolved::Import { name }) = resolved.get(&va) {
                    if let Some((f, h)) = imports_table::lookup(name) {
                        return Ok(Lifted {
                            body: format!("{{ void {f}(void); {f}(); return; }}"),
                            ctl: Ctl::Ret,
                            handler: h,
                        });
                    }
                }
                return match indirect::jmp(va, resolved) {
                    Err(e) if e == "indirect target unresolved" => tailjmp::lift(i),
                    r => r,
                };
            }
            Ok(Lifted { body: String::new(), ctl: Ctl::Jmp(i.near_branch_target()), handler: "jmp" })
        }
        _ => Err("mnemonic outside minimal set".into()),
    }
}

/// C body of one straight-line instruction at `va`. Used by the flag tests to run
/// the generated code. Branches and control transfers are rejected.
pub fn lift_body(bytes: &[u8], va: u64) -> Result<String, String> {
    let mut d = iced_x86::Decoder::with_ip(64, bytes, va, iced_x86::DecoderOptions::NONE);
    let i = d.decode();
    if i.is_invalid() {
        return Err("undecodable".into());
    }
    let l = lift_insn(&i, va, &BTreeSet::new(), &BTreeMap::new())?;
    match l.ctl {
        Ctl::Seq => Ok(l.body),
        _ => Err("control transfer".into()),
    }
}

fn traverse(
    f: u64,
    text: &Text<'_>,
    entries: &BTreeSet<u64>,
    resolved: &BTreeMap<u64, Resolved>,
    tabs: &BTreeMap<u64, fe_decode::JumpTable>,
) -> BTreeMap<u64, Node> {
    let boundary = |s: u64| s != f && entries.contains(&s);
    let mut nodes: BTreeMap<u64, Node> = BTreeMap::new();
    let mut work = vec![f];
    while let Some(start) = work.pop() {
        let mut va = start;
        loop {
            if nodes.contains_key(&va) || boundary(va) {
                break;
            }
            let i = match text.decode(va) {
                Decoded::Ok(i) => i,
                _ => {
                    nodes.insert(va, Node::Bad);
                    break;
                }
            };
            let next = va + i.len() as u64;
            let near = is_near(&i);
            let target = i.near_branch_target();
            match i.flow_control() {
                FlowControl::Next | FlowControl::XbeginXabortXend | FlowControl::IndirectCall => {
                    nodes.insert(va, Node::Insn(i));
                    va = next;
                }
                FlowControl::Call => {
                    nodes.insert(va, Node::Insn(i));
                    if !near {
                        break;
                    }
                    va = next;
                }
                FlowControl::UnconditionalBranch => {
                    nodes.insert(va, Node::Insn(i));
                    if near && !boundary(target) {
                        work.push(target);
                    }
                    break;
                }
                FlowControl::ConditionalBranch => {
                    nodes.insert(va, Node::Insn(i));
                    if near && !boundary(target) {
                        work.push(target);
                    }
                    if !boundary(next) {
                        work.push(next);
                    }
                    break;
                }
                FlowControl::IndirectBranch => {
                    nodes.insert(va, Node::Insn(i));
                    if let Some(Resolved::Code { target, .. }) = resolved.get(&va) {
                        if !boundary(*target) {
                            work.push(*target);
                        }
                    }
                    if let Some(t) = tabs.get(&va) {
                        for &(_, tg) in &t.cases {
                            if !boundary(tg) {
                                work.push(tg);
                            }
                        }
                    }
                    break;
                }
                FlowControl::Return | FlowControl::Interrupt | FlowControl::Exception => {
                    nodes.insert(va, Node::Insn(i));
                    break;
                }
            }
        }
    }
    nodes
}

/// Emission order of the blocks of `f`. The entry comes first, so the C body
/// starts at the function entry. The rest follow in ascending VA order.
/// The instructions a node of the traversal can go on to (the same edges as `traverse` follows, without the test for another entry).
fn successors(i: &Instruction, va: u64, resolved: &BTreeMap<u64, Resolved>, tabs: &BTreeMap<u64, fe_decode::JumpTable>) -> Vec<u64> {
    let next = va + i.len() as u64;
    let near = is_near(i);
    let target = i.near_branch_target();
    let mut v = Vec::new();
    match i.flow_control() {
        FlowControl::Next | FlowControl::XbeginXabortXend | FlowControl::IndirectCall => v.push(next),
        FlowControl::Call => {
            if near {
                v.push(next);
            }
        }
        FlowControl::UnconditionalBranch => {
            if near {
                v.push(target);
            }
        }
        FlowControl::ConditionalBranch => {
            if near {
                v.push(target);
            }
            v.push(next);
        }
        FlowControl::IndirectBranch => {
            if let Some(Resolved::Code { target, .. }) = resolved.get(&va) {
                v.push(*target);
            }
            if let Some(t) = tabs.get(&va) {
                for &(_, tg) in &t.cases {
                    v.push(tg);
                }
            }
        }
        FlowControl::Return | FlowControl::Interrupt | FlowControl::Exception => {}
    }
    v
}

/// Decision 0155 (`--outline-shared`). `traverse` gives every function entry all the instructions it reaches without passing another entry, so a body
/// that k entries reach is emitted k times. This returns the instructions to add to the entries so that no instruction is reached by two entries:
/// the first instruction of every shared region (a shared instruction with a predecessor that is not shared). Repeated until nothing is shared
/// (a shared region may itself be entered from two heads). A head that lies on a cycle of tail calls between functions is not promoted (see
/// `heads_on_tail_cycles`): its region stays shared. Also returns the number of shared instructions before the first and after the last round, and the number of heads
/// that were not promoted for that reason.
fn shared_heads(
    entries0: &BTreeSet<u64>,
    skip: &BTreeSet<u64>,
    text: &Text<'_>,
    resolved: &BTreeMap<u64, Resolved>,
    tabs: &BTreeMap<u64, fe_decode::JumpTable>,
) -> Result<(BTreeSet<u64>, usize, usize, usize), String> {
    use std::collections::HashMap;
    let mut banned: BTreeSet<u64> = BTreeSet::new();
    let mut first = 0usize;
    let mut first_set = false;
    for _outer in 0..64 {
        let mut entries = entries0.clone();
        let mut added: BTreeSet<u64> = BTreeSet::new();
        let mut shared_now;
        let mut rounds = 0;
        loop {
            rounds += 1;
            if rounds > 64 {
                return Err("outline-shared: no fixed point after 64 rounds".into());
            }
            let mut count: HashMap<u64, u32> = HashMap::new();
            for &f in &entries {
                if skip.contains(&f) {
                    continue;
                }
                for va in traverse(f, text, &entries, resolved, tabs).keys() {
                    *count.entry(*va).or_insert(0) += 1;
                }
            }
            shared_now = count.values().filter(|&&c| c > 1).count();
            if !first_set {
                first = shared_now;
                first_set = true;
            }
            if shared_now == 0 {
                break;
            }
            let mut heads: BTreeSet<u64> = BTreeSet::new();
            for &f in &entries {
                if skip.contains(&f) {
                    continue;
                }
                for (va, node) in &traverse(f, text, &entries, resolved, tabs) {
                    if count[va] > 1 {
                        continue;
                    }
                    if let Node::Insn(i) = node {
                        for s in successors(i, *va, resolved, tabs) {
                            if count.get(&s).copied().unwrap_or(0) > 1 && !entries.contains(&s) && !banned.contains(&s) {
                                heads.insert(s);
                            }
                        }
                    }
                }
            }
            if heads.is_empty() {
                break; // what is still shared hangs on banned heads
            }
            entries.extend(heads.iter().copied());
            added.extend(heads);
        }
        let bad = heads_on_tail_cycles(&entries, skip, &added, text, resolved, tabs);
        if bad.is_empty() {
            return Ok((added, first, shared_now, banned.len()));
        }
        banned.extend(bad);
    }
    Err("outline-shared: the heads on tail-call cycles were not all removed after 64 rounds".into())
}

/// The promoted heads that lie on a cycle of tail transfers (a jump to another function entry) of two or more functions. A loop that was a `goto` inside one
/// function would be a recursion of C calls there (nothing guarantees a tail call in C): `--outline-shared` does not promote such a head (its region stays shared).
fn heads_on_tail_cycles(
    entries: &BTreeSet<u64>,
    skip: &BTreeSet<u64>,
    added: &BTreeSet<u64>,
    text: &Text<'_>,
    resolved: &BTreeMap<u64, Resolved>,
    tabs: &BTreeMap<u64, fe_decode::JumpTable>,
) -> Vec<u64> {
    use std::collections::HashMap;
    let ids: HashMap<u64, usize> = entries.iter().enumerate().map(|(k, &e)| (e, k)).collect();
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); ids.len()];
    for &f in entries {
        if skip.contains(&f) {
            continue;
        }
        for (va, node) in &traverse(f, text, entries, resolved, tabs) {
            if let Node::Insn(i) = node {
                for s in successors(i, *va, resolved, tabs) {
                    if s != f && entries.contains(&s) {
                        adj[ids[&f]].push(ids[&s]);
                    }
                }
            }
        }
    }
    // Tarjan, iterative
    let n = adj.len();
    let (mut index, mut low, mut on, mut comp) = (vec![usize::MAX; n], vec![0usize; n], vec![false; n], vec![usize::MAX; n]);
    let (mut stack, mut next, mut ncomp) = (Vec::new(), 0usize, 0usize);
    let mut csize: Vec<usize> = Vec::new();
    for root in 0..n {
        if index[root] != usize::MAX {
            continue;
        }
        let mut work: Vec<(usize, usize)> = vec![(root, 0)];
        while let Some(&(v, k)) = work.last() {
            if k == 0 && index[v] == usize::MAX {
                index[v] = next;
                low[v] = next;
                next += 1;
                stack.push(v);
                on[v] = true;
            }
            if k < adj[v].len() {
                work.last_mut().unwrap().1 += 1;
                let w = adj[v][k];
                if index[w] == usize::MAX {
                    work.push((w, 0));
                } else if on[w] {
                    low[v] = low[v].min(index[w]);
                }
            } else {
                work.pop();
                if let Some(&(p, _)) = work.last() {
                    low[p] = low[p].min(low[v]);
                }
                if low[v] == index[v] {
                    let mut size = 0;
                    while let Some(w) = stack.pop() {
                        on[w] = false;
                        comp[w] = ncomp;
                        size += 1;
                        if w == v {
                            break;
                        }
                    }
                    csize.push(size);
                    ncomp += 1;
                }
            }
        }
    }
    added.iter().copied().filter(|h| csize[comp[ids[h]]] > 1).collect()
}

fn emit_order(f: u64, nodes: &BTreeMap<u64, Node>) -> Vec<u64> {
    let mut keys = Vec::with_capacity(nodes.len());
    if nodes.contains_key(&f) {
        keys.push(f);
    }
    keys.extend(nodes.keys().copied().filter(|&va| va != f));
    keys
}

fn succ(cx: &Cx<'_>, s: u64, next_emitted: Option<u64>, st: &mut Stats) -> String {
    if s != cx.f && cx.entries.contains(&s) {
        st.tail_calls += 1;
        if cx.landings.contains(&s) {
            // The code after a call of the guest's setjmp runs twice when the guest longjmps back: the second time from the same C
            // frame, after the runtime has unwound the frames of the longjmp (fle_landing_try in fle_dispatch.h). R[4] is the stack the
            // guest's longjmp restores. No C local is used across the setjmp: the guest state is in R, FL and X.
            return format!(
                "{{ fle_landing lj; fle_landing_push(&lj, UINT64_C({s:#x}), R[4]); if (FLE_LANDING_SETJMP(&lj) == 0) {{ f_{s:x}(); }} else {{ f_{s:x}(); }} fle_landing_pop(&lj); return; }} "
            );
        }
        return format!("{{ f_{s:x}(); return; }} ");
    }
    if next_emitted == Some(s) {
        return String::new();
    }
    if cx.nodes.contains_key(&s) {
        st.gotos += 1;
        return format!("goto L_{s:x}; ");
    }
    st.unsupported_total += 1;
    *st.unsupported.entry("<internal: successor not in function>".into()).or_insert(0) += 1;
    format!("fle_unsupported(UINT64_C({s:#x})); ")
}

fn emit_ctl(cx: &Cx<'_>, va: u64, i: &Instruction, l: Lifted, next_emitted: Option<u64>, st: &mut Stats) -> String {
    let next = va + i.len() as u64;
    match l.ctl {
        Ctl::Seq => format!("{}{}", l.body, succ(cx, next, next_emitted, st)),
        Ctl::Ret => l.body,
        Ctl::Jmp(t) => format!("{}{}", l.body, succ(cx, t, None, st)),
        Ctl::Jcc(t, cc) => {
            let taken = succ(cx, t, None, st);
            format!("if (fle_cc({cc})) {{ {taken}}} {}", succ(cx, next, next_emitted, st))
        }
    }
}

/// Translate the closure of the PE's entry and exports to C. Writes the
/// translation unit to `w` and returns the counts.
pub fn generate<W: Write>(data: &[u8], w: &mut W) -> Result<Stats, String> {
    generate_with_seeds(data, &[], w)
}

/// Same as `generate`, with `extra` as additional closure seeds (see `fe_decode::analyze_with_seeds`).
pub fn generate_with_seeds<W: Write>(data: &[u8], extra: &[u64], w: &mut W) -> Result<Stats, String> {
    generate_with_specs(data, extra, fe_decode::jumptab::FLENGINE, w)
}

/// Same as `generate_with_seeds`, with `specs` as the annotated jump tables (decision 0036).
pub fn generate_with_specs<W: Write>(data: &[u8], extra: &[u64], specs: &[fe_decode::Spec], w: &mut W) -> Result<Stats, String> {
    generate_with_options(data, extra, specs, &fe_decode::Options::default(), w)
}

/// Same as `generate_with_specs`, with `opts` (`fe_decode::Options`; the default is what the decisions specify).
pub fn generate_with_options<W: Write>(
    data: &[u8],
    extra: &[u64],
    specs: &[fe_decode::Spec],
    opts: &fe_decode::Options,
    w: &mut W,
) -> Result<Stats, String> {
    generate_with_hooks(data, extra, specs, &[], opts, w)
}

/// Same as `generate_with_options`, with `hooks` as the inline hook points (decision 0105): VAs of function entries of the closure whose
/// first 5 bytes the guest may overwrite with `jmp rel32`. Each gets a guard in front of its first instruction, and the unit gets the table
/// of the points and its registration (see `hook`). A VA that is not a function entry, a first byte that already is `e9`, a section that
/// is not read + execute, or two points less than 5 bytes apart is an error. With no hook point the output is the one of
/// `generate_with_options`, byte for byte.
pub fn generate_with_hooks<W: Write>(
    data: &[u8],
    extra: &[u64],
    specs: &[fe_decode::Spec],
    hooks: &[u64],
    opts: &fe_decode::Options,
    w: &mut W,
) -> Result<Stats, String> {
    generate_with_overrides(data, extra, specs, hooks, &[], opts, w)
}

/// Same as `generate_with_hooks`, with `sites` as the call-site memory overrides (decision 0120; `site_override`): in front of each listed
/// instruction (in every function that holds a copy of it) the unit calls `fle_site_store32`. A site that is not an instruction of the
/// closure is an error. With no site the output is the one of `generate_with_hooks`, byte for byte.
pub fn generate_with_overrides<W: Write>(
    data: &[u8],
    extra: &[u64],
    specs: &[fe_decode::Spec],
    hooks: &[u64],
    sites: &[site_override::Site],
    opts: &fe_decode::Options,
    w: &mut W,
) -> Result<Stats, String> {
    generate_with_seh(data, extra, specs, hooks, sites, opts, &SehOptions::default(), w)
}

/// Same as `generate_with_overrides`, with `seh` (decision 0132; see `SehOptions`). With no SEH function the output is the one of
/// `generate_with_overrides`, byte for byte.
pub fn generate_with_seh<W: Write>(
    data: &[u8],
    extra: &[u64],
    specs: &[fe_decode::Spec],
    hooks: &[u64],
    sites: &[site_override::Site],
    opts: &fe_decode::Options,
    seh: &SehOptions,
    w: &mut W,
) -> Result<Stats, String> {
    generate_full(data, extra, specs, hooks, sites, opts, seh, &GenOptions::default(), w)
}

/// Same as `generate_with_seh`, with `gopt` (decision 0155; see `GenOptions`). With the default options the output is the one of `generate_with_seh`, byte for byte.
pub fn generate_full<W: Write>(
    data: &[u8],
    extra: &[u64],
    specs: &[fe_decode::Spec],
    hooks: &[u64],
    sites: &[site_override::Site],
    opts: &fe_decode::Options,
    seh: &SehOptions,
    gopt: &GenOptions,
    w: &mut W,
) -> Result<Stats, String> {
    let pe = Pe::parse(data).map_err(|e| e.to_string())?;
    let text = Text::from_pe(&pe)?;
    swap::check_unique(&gopt.swap)?;
    let dead_flags = gopt.swap.is_empty();
    let outline_shared = gopt.swap.is_empty() && seh.funcs.is_empty();
    let swap_set: BTreeSet<u64> = gopt.swap.iter().map(|e| e.va).collect();
    for e in &gopt.swap {
        if !text.contains(e.va) {
            return Err(format!("swap entry {:#x} is not in .text", e.va));
        }
    }
    prune::check_unique(&gopt.prune)?;
    // class U rows are table jumps that stay unresolved (see fe_decode::analyze_with_prune); all other rows are pruned functions
    let prune_set: BTreeSet<u64> = gopt.prune.iter().filter(|e| e.class != "U").map(|e| e.va).collect();
    let keep_unresolved: BTreeSet<u64> = gopt.prune.iter().filter(|e| e.class == "U").map(|e| e.va).collect();
    for e in &gopt.prune {
        if !text.contains(e.va) {
            return Err(format!("prune entry {:#x} is not in .text", e.va));
        }
        if swap_set.contains(&e.va) {
            return Err(format!("prune entry {:#x} is also a swap entry", e.va));
        }
    }
    let a: Analysis = fe_decode::analyze_with_prune(&pe, extra, specs, opts, &swap_set, &prune_set, &keep_unresolved)?;
    let mut entries: BTreeSet<u64> = a.functions.keys().copied().collect();
    let table_entries: BTreeSet<u64> = entries.clone();
    // the pruned functions that kept code reaches: stubs in the output, not walked by the analyses below
    let prune_stubs: BTreeSet<u64> = prune_set.intersection(&entries).copied().collect();
    let mut outlined = (0usize, 0usize, 0usize, 0usize);
    if outline_shared {
        let (heads, before, after, banned) = shared_heads(&entries, &prune_stubs, &text, &a.resolved, &a.jumptabs)?;
        outlined = (heads.len(), before, after, banned);
        entries.extend(heads);
    }
    let flive = if dead_flags { Some(flaglive::analyse(&entries, &prune_stubs, &text, &a.resolved, &a.jumptabs)?) } else { None };
    let mut st = Stats {
        closure_insns: a.closure.instructions,
        functions: entries.len(),
        register_jmp_sites: a.closure.register_jumps,
        jumptabs: a.closure.jumptabs,
        msvc_tables: a.closure.msvc_tables,
        jumptab_rejected: a.closure.jumptab_rejected.clone(),
        msvc_rejected: a.closure.msvc_rejected.clone(),
        outlined_heads: outlined.0,
        shared_insns_before: outlined.1,
        shared_insns_after: outlined.2,
        outline_banned: outlined.3,
        flag_writers: flive.as_ref().map_or(0, |r| r.writers),
        flag_dead_full: flive.as_ref().map_or(0, |r| r.fully_dead),
        flag_dead_part: flive.as_ref().map_or(0, |r| r.partly_dead),
        flag_rounds: flive.as_ref().map_or(0, |r| r.rounds),
        ..Default::default()
    };
    let landings: BTreeSet<u64> = fe_decode::setjmp::landings(&text, &a.insns).into_keys().filter(|l| entries.contains(l)).collect();
    st.setjmp_landings = landings.len();
    for (f, ls) in &seh.funcs {
        if !entries.contains(f) {
            return Err(format!("SEH: the function {f:#x} with an except landing is not a function entry of the closure (lift with --pdata)"));
        }
        for l in ls {
            if !entries.contains(l) {
                return Err(format!("SEH: the landing {l:#x} of {f:#x} is not a function entry of the closure (it must be a seed)"));
            }
        }
    }
    st.seh_wrapped = seh.funcs.len();
    st.seh_landings = seh.funcs.values().map(Vec::len).sum();
    let mut found = Vec::new();
    for &va in hooks {
        let rva = va.wrapping_sub(pe.optional.image_base);
        let section = pe
            .sections
            .iter()
            .find(|s| {
                let span = if s.virtual_size != 0 { s.virtual_size } else { s.size_of_raw_data };
                rva >= s.virtual_address as u64 && rva < s.virtual_address as u64 + span as u64
            })
            .map(|s| s.characteristics);
        found.push(hook::check_point(va, text.slice(va, hook::HOOK_LEN), entries.contains(&va), section)?);
    }
    let hook_points = hook::finish(found)?;
    st.hook_points = hook_points.len();
    let guards: BTreeMap<u64, String> = hook_points.iter().map(|p| (p.va, hook::guard(p))).collect();
    site_override::check_unique(sites)?;
    let mut site_guards: BTreeMap<u64, String> = BTreeMap::new();
    for s in sites {
        if !a.insns.contains_key(&s.va) {
            return Err(format!("site override {:#x} is not an instruction of the lifted closure (lift with --pdata, or seed the function with --seeds)", s.va));
        }
        site_guards.entry(s.va).or_default().push_str(&site_override::guard(s));
    }
    let mut site_emitted: BTreeMap<u64, usize> = BTreeMap::new();
    st.site_overrides = sites.len();
    hle::check_unique(&gopt.hle)?;
    let mut hle_guards: BTreeMap<u64, String> = BTreeMap::new();
    for e in &gopt.hle {
        if !entries.contains(&e.va) {
            return Err(format!("HLE entry {:#x} is not a function entry of the lifted closure (lift with --pdata, or seed the function with --seeds)", e.va));
        }
        hle_guards.insert(e.va, hle::guard(e));
    }
    st.hle_entries = gopt.hle.len();
    st.swap_entries = gopt.swap.len();
    st.pruned_listed = prune_set.len();
    st.pruned_stubs = prune_stubs.len();
    st.pruned_seeds_dropped = a.closure.pruned_seeds_dropped;
    st.pruned_cond_targets = a.closure.prune_cond_targets;
    for e in &gopt.hle {
        if prune_set.contains(&e.va) {
            return Err(format!("HLE entry {:#x} is in the prune list", e.va));
        }
    }
    for p in &hook_points {
        if prune_set.contains(&p.va) {
            return Err(format!("hook point {:#x} is in the prune list", p.va));
        }
    }
    let mut o = Out { w, lines: 0, bytes: 0 };

    o.put("/* generated by fe-lift. Do not edit. */\n")?;
    o.put("#include \"fle_runtime.h\"\n#include \"fle_sse.h\"\n#include \"fle_dispatch.h\"\n")?;
    if !hook_points.is_empty() || !sites.is_empty() {
        o.put(hook::INCLUDE)?;
    }
    if !seh.funcs.is_empty() {
        o.put("#include \"fle_seh.h\"\n")?;
    }
    if flive.is_some() {
        o.put("#include \"fle_flags_nf.h\"\n")?;
    }
    if !gopt.prune.is_empty() {
        o.put("#include \"fle_prune.h\"\n")?;
    }
    o.put("\nuint64_t R[16];\nfle_eflags FL = { FLE_UNDEF, FLE_UNDEF, FLE_UNDEF, FLE_UNDEF, FLE_UNDEF, FLE_UNDEF };\nfle_xmm X[16];\n\n")?;
    for &f in &entries {
        o.put(&format!("void f_{f:x}(void);\n"))?;
        if seh.funcs.contains_key(&f) {
            o.put(&format!("__attribute__((no_instrument_function)) void fb_{f:x}(void);\n"))?;
        }
    }
    o.put("\n")?;
    let resp = imports_table::resp_wanted(a.resolved.values().filter_map(|r| match r {
        Resolved::Import { name } => Some(name.as_str()),
        _ => None,
    }));
    if resp {
        o.put(&imports_table::dispatch_and_resp_tables(&table_entries))?;
        st.resp_entries = imports_table::resp_count();
    } else {
        o.put(&dispatch::table(&table_entries))?;
    }

    for &f in &entries {
        if let Some(e) = gopt.swap.iter().find(|e| e.va == f) {
            o.put(&swap::stub(e))?;
            continue;
        }
        if prune_stubs.contains(&f) {
            o.put(&prune::stub(f))?;
            continue;
        }
        let nodes = traverse(f, &text, &entries, &a.resolved, &a.jumptabs);
        let cx = Cx { f, nodes: &nodes, entries: &entries, landings: &landings };
        if seh.funcs.contains_key(&f) {
            // Decision 0132: the entry of a function with an except landing. The C frame of this wrapper is the one the unwinder (fle_seh.c, RtlUnwindEx)
            // jumps back to; the landing runs in it as an ordinary function (fle_seh_land). The body is fb_<va>, not instrumented, so the wrapper is the one entry.
            o.put(&format!(
                "void f_{f:x}(void)\n{{\n    fle_seh_reg sr;\n    fle_seh_enter(&sr, UINT64_C({f:#x}));\n    if (FLE_SEH_SETJMP(&sr) == 0) {{ fb_{f:x}(); }} else {{ fle_seh_land(&sr); }}\n    fle_seh_leave(&sr);\n}}\n\n__attribute__((no_instrument_function)) void fb_{f:x}(void)\n{{\n"
            ))?;
        } else {
            o.put(&format!("void f_{f:x}(void)\n{{\n"))?;
        }
        let dead_here = flive.as_ref().and_then(|r| r.dead.get(&f));
        let keys = emit_order(f, &nodes);
        for (k, &va) in keys.iter().enumerate() {
            let next_emitted = keys.get(k + 1).copied();
            // The guard of an inline hook point (decision 0105) is part of the block of the entry instruction, so a jump back to the entry runs it.
            let mut pre = String::new();
            if va == f {
                pre.push_str(guards.get(&f).map(String::as_str).unwrap_or(""));
                // Decision 0162: the guard of an HLE entry, after the inline-hook guard (a hooked function is redirected first).
                pre.push_str(hle_guards.get(&f).map(String::as_str).unwrap_or(""));
            }
            // The guard of a call-site memory override (decision 0120): in the block of the instruction at the site, in every function that holds a copy.
            if let Some(g) = site_guards.get(&va) {
                pre.push_str(g);
                *site_emitted.entry(va).or_insert(0) += 1;
            }
            let line = match &nodes[&va] {
                Node::Bad => {
                    st.bad_nodes += 1;
                    st.unsupported_total += 1;
                    *st.unsupported.entry("<undecodable or outside .text>".into()).or_insert(0) += 1;
                    format!("L_{va:x}: {{ {pre}fle_unsupported(UINT64_C({va:#x})); }}\n")
                }
                Node::Insn(i) => {
                    st.emitted_insns += 1;
                    let m = format!("{:?}", i.mnemonic());
                    let lifted = match a.jumptabs.get(&va) {
                        Some(t) => jumptab::lift(i, va, t, &cx, &mut st),
                        None => lift_insn(i, va, &entries, &a.resolved),
                    };
                    let reg_jmp = m == "Jmp" && i.op_kind(0) == OpKind::Register;
                    match lifted {
                        Ok(mut l) => {
                            if dead_here.map_or(false, |d| d.contains(&va)) {
                                if let Some(b) = flaglive::nf_subst(&l.body) {
                                    l.body = b;
                                    st.flag_substituted += 1;
                                }
                            }
                            if reg_jmp {
                                *st.register_jmps.entry(l.handler).or_insert(0) += 1;
                            }
                            *st.handled.entry(l.handler).or_insert(0) += 1;
                            let body = emit_ctl(&cx, va, i, l, next_emitted, &mut st);
                            format!("L_{va:x}: {{ {pre}/* {va:#x} {m} */ {body}}}\n")
                        }
                        Err(reason) => {
                            if reg_jmp {
                                *st.register_jmps.entry("unsupported").or_insert(0) += 1;
                            }
                            st.unsupported_total += 1;
                            *st.unsupported.entry(format!("{m}: {reason}")).or_insert(0) += 1;
                            format!("L_{va:x}: {{ {pre}/* {va:#x} {m} */ fle_unsupported(UINT64_C({va:#x})); }}\n")
                        }
                    }
                }
            };
            o.put(&line)?;
        }
        o.put("}\n\n")?;
    }
    if !hook_points.is_empty() {
        o.put(&hook::tail(pe.optional.image_base, pe.optional.size_of_image as u64, &hook_points))?;
    }
    for s in sites {
        if site_emitted.get(&s.va).copied().unwrap_or(0) == 0 {
            return Err(format!("site override {:#x}: the instruction is in the closure but in no emitted function (nothing was guarded)", s.va));
        }
    }

    st.c_lines = o.lines;
    st.c_bytes = o.bytes;
    Ok(st)
}
