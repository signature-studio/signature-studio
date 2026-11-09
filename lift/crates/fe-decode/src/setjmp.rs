//! The guest's own `setjmp` and where it returns a second time (decision 0100).
//!
//! FLEngine_x64.dll carries a register-saving `setjmp` and `longjmp` of its own (the Delphi runtime's start-up code, reading of the
//! bytes): `setjmp(buf)` stores the return address at `[buf]`, the stack pointer after the return at `[buf+8]`, rbx, rbp, rsi, rdi and
//! r12..r15 at `[buf+0x10..0x48]` and xmm6..xmm15 after them, and returns 0; `longjmp(buf, v)` loads them back, puts `v` in `eax` and ends
//! in `jmp [buf]` at the stack of the frame that called `setjmp`. The address that `jmp` goes to is the instruction after the `call` of
//! `setjmp`: the "landing". It lies in the middle of the calling function, so no function entry names it.
//!
//! This module finds the landings. A direct near `call` whose target is the shape below, optionally behind a `lea rcx,[rcx+d]; jmp rel32`
//! trampoline (the guest has one that passes the address of a field), has its return address as a landing. The shape (all of it is
//! checked on the decoded instructions, nothing else is looked at):
//! ```text
//! mov rax,[rsp]; lea rdx,[rsp+8]; mov [rcx],rax; mov [rcx+8],rdx;
//! mov [rcx+0x10],rbx; mov [rcx+0x18],rbp; mov [rcx+0x20],rsi; mov [rcx+0x28],rdi;
//! mov [rcx+0x30],r12; mov [rcx+0x38],r13; mov [rcx+0x40],r14; mov [rcx+0x48],r15;
//! ... (at most 24 more instructions) ... xor eax,eax; ret
//! ```
//! The landing is made a function entry (a seed), so that the code after the `setjmp` call is a function of its own that the runtime can
//! run a second time (`fe-lift`, `fle_landing` in `runtime/fle_dispatch.h`).

use crate::Text;
use iced_x86::{Decoder, DecoderOptions, FlowControl, Instruction, Mnemonic, OpKind, Register};
use std::collections::{BTreeMap, HashMap};

const SAVES: [(Register, i64); 8] = [
    (Register::RBX, 0x10),
    (Register::RBP, 0x18),
    (Register::RSI, 0x20),
    (Register::RDI, 0x28),
    (Register::R12, 0x30),
    (Register::R13, 0x38),
    (Register::R14, 0x40),
    (Register::R15, 0x48),
];
const TAIL_MAX: usize = 24;
const HOPS_MAX: usize = 2;

fn decode_at(text: &Text<'_>, va: u64) -> Option<Instruction> {
    let r = text.range_at(va)?;
    let off = (va - r.va) as usize;
    let end = (off + 15).min(r.bytes.len());
    let i = Decoder::with_ip(64, &r.bytes[off..end], va, DecoderOptions::NONE).decode();
    (!i.is_invalid()).then_some(i)
}

fn mem_rcx(i: &Instruction, disp: i64) -> bool {
    i.op_kind(0) == OpKind::Memory
        && i.memory_base() == Register::RCX
        && i.memory_index() == Register::None
        && i.segment_prefix() == Register::None
        && i.memory_displacement64() as i64 == disp
        && i.memory_size().size() == 8
}

fn is_store(i: &Instruction, disp: i64, reg: Register) -> bool {
    i.mnemonic() == Mnemonic::Mov && i.op_count() == 2 && mem_rcx(i, disp) && i.op_kind(1) == OpKind::Register && i.op1_register() == reg
}

/// Whether the function at `va` is the guest's `setjmp`: the shape of the module comment, reached directly or behind at most two
/// `lea rcx,[rcx+d]; jmp rel32` / `jmp rel32` hops.
pub fn is_setjmp_entry(text: &Text<'_>, va: u64) -> bool {
    let mut at = va;
    for _ in 0..=HOPS_MAX {
        let Some(first) = decode_at(text, at) else { return false };
        let jmp_at = match first.mnemonic() {
            Mnemonic::Mov => return setjmp_body(text, at),
            Mnemonic::Lea
                if first.op_kind(0) == OpKind::Register
                    && first.op0_register() == Register::RCX
                    && first.memory_base() == Register::RCX
                    && first.memory_index() == Register::None =>
            {
                first.next_ip()
            }
            Mnemonic::Jmp => at,
            _ => return false,
        };
        match decode_at(text, jmp_at) {
            Some(j) if j.mnemonic() == Mnemonic::Jmp && j.op_kind(0) == OpKind::NearBranch64 => at = j.near_branch_target(),
            _ => return false,
        }
    }
    false
}

fn setjmp_body(text: &Text<'_>, va: u64) -> bool {
    let mut at = va;
    let next = |at: &mut u64| -> Option<Instruction> {
        let i = decode_at(text, *at)?;
        *at = i.next_ip();
        Some(i)
    };
    let Some(i0) = next(&mut at) else { return false };
    let ok0 = i0.mnemonic() == Mnemonic::Mov
        && i0.op_count() == 2
        && i0.op_kind(0) == OpKind::Register
        && i0.op0_register() == Register::RAX
        && i0.op_kind(1) == OpKind::Memory
        && i0.memory_base() == Register::RSP
        && i0.memory_index() == Register::None
        && i0.memory_displacement64() == 0
        && i0.memory_size().size() == 8;
    let Some(i1) = next(&mut at) else { return false };
    let ok1 = i1.mnemonic() == Mnemonic::Lea
        && i1.op0_register() == Register::RDX
        && i1.memory_base() == Register::RSP
        && i1.memory_index() == Register::None
        && i1.memory_displacement64() == 8;
    let Some(i2) = next(&mut at) else { return false };
    let Some(i3) = next(&mut at) else { return false };
    if !(ok0 && ok1 && is_store(&i2, 0, Register::RAX) && is_store(&i3, 8, Register::RDX)) {
        return false;
    }
    for (reg, disp) in SAVES {
        let Some(i) = next(&mut at) else { return false };
        if !is_store(&i, disp, reg) {
            return false;
        }
    }
    let mut prev_xor = false;
    for _ in 0..TAIL_MAX {
        let Some(i) = next(&mut at) else { return false };
        if i.mnemonic() == Mnemonic::Ret {
            return prev_xor;
        }
        prev_xor = i.mnemonic() == Mnemonic::Xor
            && i.op_count() == 2
            && i.op_kind(0) == OpKind::Register
            && i.op_kind(1) == OpKind::Register
            && i.op0_register() == Register::EAX
            && i.op1_register() == Register::EAX;
        if matches!(i.flow_control(), FlowControl::UnconditionalBranch | FlowControl::ConditionalBranch | FlowControl::IndirectBranch | FlowControl::Call) {
            return false;
        }
    }
    false
}

/// A landing: the instruction after a `call` of the guest's `setjmp`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Landing {
    /// VA of the `call`.
    pub call_va: u64,
    /// The call target (the `setjmp` or its trampoline).
    pub callee: u64,
}

/// Every landing of the instructions `insns` (the closure of a run), keyed by its VA (the return address of the `call`).
pub fn landings(text: &Text<'_>, insns: &BTreeMap<u64, Instruction>) -> BTreeMap<u64, Landing> {
    let mut known: HashMap<u64, bool> = HashMap::new();
    let mut out = BTreeMap::new();
    for (&va, i) in insns {
        if i.mnemonic() != Mnemonic::Call || i.op_kind(0) != OpKind::NearBranch64 {
            continue;
        }
        let t = i.near_branch_target();
        if *known.entry(t).or_insert_with(|| is_setjmp_entry(text, t)) {
            out.insert(i.next_ip(), Landing { call_va: va, callee: t });
        }
    }
    out
}
