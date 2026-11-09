//! Adjustor thunks that no `call` and no `.pdata` entry names (decision 0051).
//!
//! A Delphi interface method table points at a short thunk that corrects `Self` and jumps to the real
//! method: `add rcx, -0x10; jmp rel32`. The thunk is data-referenced only, and it lies outside every
//! `.pdata` range, so neither the `.pdata` seeds (decision 0031) nor the closure of direct calls reach it.
//!
//! A seed is taken only when all of these hold (each one is checked on the decoded bytes):
//! - a base relocation of type DIR64 (`Pe::dir64_relocation_vas`) marks an 8-byte slot whose value is the address;
//! - the address is in `.text`, is not a `.pdata` start and lies in no `.pdata` range (those are reached already);
//! - it decodes as 1..=3 register-adjusting instructions followed by a direct near `jmp`, nothing else:
//!   `add`/`sub` of a 32/64-bit register with an immediate, `lea r64, [base+disp]` (no index, not rip-relative,
//!   no segment) or `mov r64, [base+disp]` (same memory form);
//! - the whole thunk is at most 16 bytes and the jump target is a `.pdata` start in `.text` (a known function).
//!
//! A bare `jmp` (no adjusting instruction) is not taken: those are import and forwarding thunks.

use crate::Text;
use fe_pe::Pe;
use iced_x86::{Code, Decoder, DecoderOptions, FlowControl, Instruction, Mnemonic, OpKind, Register};
use std::collections::{BTreeSet, HashMap};

const MAX_PRE: usize = 3;
const MAX_BYTES: usize = 16;

/// Why a candidate address was not taken (for the survey tool and the tests).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reject {
    NotInText,
    IsPdataStart,
    InsidePdataRange,
    NotAThunk,
    /// Decision 0100: the shape decodes but the start has no proof of being an instruction boundary.
    NotAnchored,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Adjustor {
    pub va: u64,
    /// The `jmp` target.
    pub target: u64,
    /// Number of adjusting instructions before the `jmp`.
    pub pre: usize,
    pub len: usize,
    /// Mnemonic shape, for example `add+jmp`.
    pub shape: String,
}

fn is_imm(k: OpKind) -> bool {
    matches!(
        k,
        OpKind::Immediate8
            | OpKind::Immediate8_2nd
            | OpKind::Immediate16
            | OpKind::Immediate32
            | OpKind::Immediate64
            | OpKind::Immediate8to16
            | OpKind::Immediate8to32
            | OpKind::Immediate8to64
            | OpKind::Immediate32to64
    )
}

fn plain_base_mem(i: &Instruction, k: u32) -> bool {
    i.op_kind(k) == OpKind::Memory
        && i.memory_base() != Register::None
        && i.memory_base() != Register::RIP
        && i.memory_base() != Register::EIP
        && i.memory_index() == Register::None
        && i.segment_prefix() == Register::None
}

fn is_adjusting(i: &Instruction) -> Option<&'static str> {
    if i.op_count() != 2 || i.op_kind(0) != OpKind::Register {
        return None;
    }
    let r = i.op0_register();
    match i.mnemonic() {
        Mnemonic::Add | Mnemonic::Sub if (r.is_gpr64() || r.is_gpr32()) && is_imm(i.op_kind(1)) => {
            Some(if i.mnemonic() == Mnemonic::Add { "add" } else { "sub" })
        }
        Mnemonic::Lea if r.is_gpr64() && i.code() != Code::INVALID && plain_base_mem(i, 1) => Some("lea"),
        Mnemonic::Mov if r.is_gpr64() && plain_base_mem(i, 1) => Some("mov"),
        _ => None,
    }
}

/// Decodes the thunk shape at `va` (a VA in `text`). `Ok` is the thunk, `Err(NotAThunk)` any other bytes.
pub fn decode_thunk(text: &Text<'_>, va: u64) -> Result<Adjustor, Reject> {
    let Some(r) = text.range_at(va) else {
        return Err(Reject::NotInText);
    };
    let off = (va - r.va) as usize;
    let end = (off + MAX_BYTES).min(r.bytes.len());
    let mut d = Decoder::with_ip(64, &r.bytes[off..end], va, DecoderOptions::NONE);
    let mut shape: Vec<&'static str> = Vec::new();
    loop {
        let i = d.decode();
        if i.is_invalid() {
            return Err(Reject::NotAThunk);
        }
        if i.flow_control() == FlowControl::UnconditionalBranch {
            let direct = matches!(i.op_kind(0), OpKind::NearBranch64 | OpKind::NearBranch32 | OpKind::NearBranch16);
            if !direct || shape.is_empty() {
                return Err(Reject::NotAThunk);
            }
            let len = (i.next_ip() - va) as usize;
            shape.push("jmp");
            return Ok(Adjustor { va, target: i.near_branch_target(), pre: shape.len() - 1, len, shape: shape.join("+") });
        }
        match is_adjusting(&i) {
            Some(s) if shape.len() < MAX_PRE => shape.push(s),
            _ => return Err(Reject::NotAThunk),
        }
    }
}

/// The `.pdata` function starts and ranges (chain entries included) for the "outside every range" test.
struct Pdata {
    starts: BTreeSet<u64>,
    ranges: Vec<(u64, u64)>,
}

impl Pdata {
    fn load(pe: &Pe<'_>) -> Result<Pdata, String> {
        let starts: BTreeSet<u64> = pe.function_start_vas().map_err(|e| format!("pdata: {e}"))?.into_iter().collect();
        let mut ranges: Vec<(u64, u64)> = pe
            .runtime_functions()
            .map_err(|e| format!("pdata: {e}"))?
            .iter()
            .map(|r| (pe.rva_to_va(r.begin_rva), pe.rva_to_va(r.end_rva)))
            .collect();
        ranges.sort_unstable();
        Ok(Pdata { starts, ranges })
    }

    fn inside_range(&self, va: u64) -> bool {
        let i = self.ranges.partition_point(|&(b, _)| b <= va);
        // ranges may nest or overlap in a hostile image: look back over every range that begins at or before va
        self.ranges[..i].iter().rev().take(64).any(|&(b, e)| b <= va && va < e)
    }
}

/// Every data-referenced adjustor thunk outside `.pdata`, ascending by VA.
pub fn adjustors(pe: &Pe<'_>) -> Result<Vec<Adjustor>, String> {
    let text = Text::from_pe(pe)?;
    let pd = Pdata::load(pe)?;
    if pd.starts.is_empty() {
        return Ok(Vec::new());
    }
    let mut cands: BTreeSet<u64> = BTreeSet::new();
    for slot in pe.dir64_relocation_vas().map_err(|e| format!("reloc: {e}"))? {
        let Ok(rva) = pe.va_to_rva(slot) else { continue };
        let Ok(b) = pe.slice_at_rva(rva, 8) else { continue };
        let v = u64::from_le_bytes(b.try_into().expect("8 bytes"));
        if text.contains(v) {
            cands.insert(v);
        }
    }
    let mut out = Vec::new();
    for v in cands {
        if pd.starts.contains(&v) || pd.inside_range(v) {
            continue;
        }
        if let Ok(a) = decode_thunk(&text, v) {
            if text.contains(a.target) && pd.starts.contains(&a.target) {
                out.push(a);
            }
        }
    }
    Ok(out)
}

/// Why `va` is not taken, or `Ok` with its thunk (single-address form of `adjustors`, for tests).
pub fn classify(pe: &Pe<'_>, va: u64) -> Result<Adjustor, Reject> {
    let text = Text::from_pe(pe).map_err(|_| Reject::NotInText)?;
    let pd = Pdata::load(pe).map_err(|_| Reject::NotAThunk)?;
    if !text.contains(va) {
        return Err(Reject::NotInText);
    }
    if pd.starts.contains(&va) {
        return Err(Reject::IsPdataStart);
    }
    if pd.inside_range(va) {
        return Err(Reject::InsidePdataRange);
    }
    let a = decode_thunk(&text, va)?;
    if text.contains(a.target) && pd.starts.contains(&a.target) {
        Ok(a)
    } else {
        Err(Reject::NotAThunk)
    }
}

/// VAs of `adjustors`, ascending.
pub fn adjustor_seeds(pe: &Pe<'_>) -> Result<Vec<u64>, String> {
    Ok(adjustors(pe)?.into_iter().map(|a| a.va).collect())
}

// ---- decision 0100: virtual-call thunks ----
//
// The same data-referenced, `.pdata`-less thunks, in the other shape the compiler emits for a virtual function that is
// reached through a secondary base: it corrects `this`, loads the vtable pointer and jumps through a slot,
// `add rcx, -0x10; mov rax, [rcx]; jmp [rax+0x10]`. Decision 0051 takes only a direct `jmp` to a `.pdata` start; this one has no static
// target, so the function it makes is the three instructions and nothing else (no new entry beyond itself).
//
// A seed is taken only when all of these hold:
// - a DIR64 relocation slot holds the address (as for 0051); the address is in `.text`, is not a `.pdata` start and lies in no `.pdata` range;
// - it decodes as 0..=2 `add`/`sub` of a 32/64-bit register with an immediate, then `mov r64, [base+disp]` (no index, not rip-relative, no
//   segment), then `jmp qword ptr [r64+disp]` with the register the `mov` wrote as base (no index, not rip-relative, no segment), nothing else,
//   in at most 16 bytes;
// - the start is an instruction boundary that does not rest on the decode alone (`anchored`): the byte before it is int3 padding, or a `.pdata`
//   range ends exactly there, or a thunk of either family (0051 or this one) ends exactly there and is itself anchored;
// - (guard, `vcall_seeds`) no function of the closure without these seeds reaches the address, so adding it as a function entry cannot turn
//   a `goto` of an existing function into a tail call: the bodies of the existing functions do not change.

const MAX_PRE_VCALL: usize = 2;
const MAX_ANCHOR_DEPTH: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VcallThunk {
    pub va: u64,
    pub len: usize,
    /// Number of `add`/`sub` instructions before the `mov`.
    pub pre: usize,
    /// For example `add+mov+jmp`.
    pub shape: String,
}

fn plain_mem64(i: &Instruction, k: u32) -> bool {
    plain_base_mem(i, k) && i.memory_size().size() == 8 && i.memory_base().is_gpr64()
}

/// Decodes the virtual-call thunk shape at `va`. `Ok` is the thunk, `Err(NotAThunk)` any other bytes.
pub fn decode_vcall_thunk(text: &Text<'_>, va: u64) -> Result<VcallThunk, Reject> {
    let Some(r) = text.range_at(va) else {
        return Err(Reject::NotInText);
    };
    let off = (va - r.va) as usize;
    let end = (off + MAX_BYTES).min(r.bytes.len());
    let mut d = Decoder::with_ip(64, &r.bytes[off..end], va, DecoderOptions::NONE);
    let mut shape: Vec<&'static str> = Vec::new();
    let mut loaded: Option<Register> = None;
    loop {
        let i = d.decode();
        if i.is_invalid() {
            return Err(Reject::NotAThunk);
        }
        match loaded {
            None => match i.mnemonic() {
                Mnemonic::Add | Mnemonic::Sub => match is_adjusting(&i) {
                    Some(s) if shape.len() < MAX_PRE_VCALL => shape.push(s),
                    _ => return Err(Reject::NotAThunk),
                },
                Mnemonic::Mov
                    if i.op_count() == 2
                        && i.op_kind(0) == OpKind::Register
                        && i.op0_register().is_gpr64()
                        && plain_mem64(&i, 1) =>
                {
                    loaded = Some(i.op0_register());
                    shape.push("mov");
                }
                _ => return Err(Reject::NotAThunk),
            },
            Some(reg) => {
                let ok = i.mnemonic() == Mnemonic::Jmp
                    && i.flow_control() == FlowControl::IndirectBranch
                    && i.op_kind(0) == OpKind::Memory
                    && plain_mem64(&i, 0)
                    && i.memory_base() == reg;
                if !ok {
                    return Err(Reject::NotAThunk);
                }
                let len = (i.next_ip() - va) as usize;
                shape.push("jmp");
                return Ok(VcallThunk { va, len, pre: shape.len() - 2, shape: shape.join("+") });
            }
        }
    }
}

/// Whether the thunk of either family that ends at `va` exists, with the start anchored. `decode_thunk` is used for the direct family
/// without its target test: a predecessor only has to be code that is read from a sync point.
fn anchored(text: &Text<'_>, ends: &BTreeSet<u64>, va: u64, memo: &mut HashMap<u64, bool>, depth: usize) -> bool {
    if let Some(&m) = memo.get(&va) {
        return m;
    }
    let mut ok = text.slice(va.wrapping_sub(1), 1).is_some_and(|b| b[0] == 0xcc) || ends.contains(&va);
    if !ok && depth < MAX_ANCHOR_DEPTH {
        for len in 6..=MAX_BYTES as u64 {
            let s = va.wrapping_sub(len);
            let direct = decode_thunk(text, s).is_ok_and(|a| a.len as u64 == len);
            let virt = !direct && decode_vcall_thunk(text, s).is_ok_and(|a| a.len as u64 == len);
            if (direct || virt) && anchored(text, ends, s, memo, depth + 1) {
                ok = true;
                break;
            }
        }
    }
    memo.insert(va, ok);
    ok
}

fn pdata_ends(pd: &Pdata) -> BTreeSet<u64> {
    pd.ranges.iter().map(|&(_, e)| e).collect()
}

/// Every data-referenced virtual-call thunk outside `.pdata` with an anchored start, ascending by VA (before the guard of `vcall_seeds`).
pub fn vcall_thunks(pe: &Pe<'_>) -> Result<Vec<VcallThunk>, String> {
    let text = Text::from_pe(pe)?;
    let pd = Pdata::load(pe)?;
    if pd.starts.is_empty() {
        return Ok(Vec::new());
    }
    let ends = pdata_ends(&pd);
    let mut cands: BTreeSet<u64> = BTreeSet::new();
    for slot in pe.dir64_relocation_vas().map_err(|e| format!("reloc: {e}"))? {
        let Ok(rva) = pe.va_to_rva(slot) else { continue };
        let Ok(b) = pe.slice_at_rva(rva, 8) else { continue };
        let v = u64::from_le_bytes(b.try_into().expect("8 bytes"));
        if text.contains(v) {
            cands.insert(v);
        }
    }
    let mut memo: HashMap<u64, bool> = HashMap::new();
    let mut out: Vec<VcallThunk> = Vec::new();
    for v in cands {
        if pd.starts.contains(&v) || pd.inside_range(v) {
            continue;
        }
        let Ok(t) = decode_vcall_thunk(&text, v) else { continue };
        if out.last().is_some_and(|p| v < p.va + p.len as u64) {
            continue;
        }
        if anchored(&text, &ends, v, &mut memo, 0) {
            out.push(t);
        }
    }
    Ok(out)
}

/// Why `va` is not taken, or `Ok` with its thunk (single-address form of `vcall_thunks`, for tests; no guard).
pub fn classify_vcall(pe: &Pe<'_>, va: u64) -> Result<VcallThunk, Reject> {
    let text = Text::from_pe(pe).map_err(|_| Reject::NotInText)?;
    let pd = Pdata::load(pe).map_err(|_| Reject::NotAThunk)?;
    if !text.contains(va) {
        return Err(Reject::NotInText);
    }
    if pd.starts.contains(&va) {
        return Err(Reject::IsPdataStart);
    }
    if pd.inside_range(va) {
        return Err(Reject::InsidePdataRange);
    }
    let t = decode_vcall_thunk(&text, va)?;
    if anchored(&text, &pdata_ends(&pd), va, &mut HashMap::new(), 0) {
        Ok(t)
    } else {
        Err(Reject::NotAnchored)
    }
}

/// The seeds of `vcall_thunks` that the closure `base` does not reach. `base` is the closure of every other seed of the run (the `.pdata`
/// starts, the 0051 thunks, the seeds file), made with the specs and options the run itself will use. A thunk that is already an entry, or that
/// a function of that closure flows into, is left out.
pub fn vcall_seeds(pe: &Pe<'_>, base: &crate::Analysis) -> Result<Vec<u64>, String> {
    Ok(vcall_thunks(pe)?.into_iter().map(|t| t.va).filter(|v| !base.insns.contains_key(v)).collect())
}
