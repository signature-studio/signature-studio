//! Hand-annotated jump tables (decision 0036). A `Spec` names one indirect `jmp`, the table it
//! reads, the number of entries, and the function the jump lives in. The ranges come from the
//! `cmp` / `ja` (or `jg`) guards in front of each jump, read from `objdump -d` of FLEngine_x64.dll
//! (see docs/investigations/indirect-jmp-run.md); nothing here recognises a pattern.
//!
//! `resolve` checks the annotation against the image before use:
//! - the bytes at `jmp_va` are the expected instruction and `jmp_va` lies in `func`;
//! - the table lies in `func` and every target lies in `func` and in `.text`.
//!
//! `verify` runs after the closure is complete: every target must be the start of a decoded
//! instruction, with no decoded instruction covering any of its bytes. A failed check drops the table
//! (the jump stays `Unresolved`) and is reported in `Closure::jumptab_rejected`.
//!
//! Selector stubs (decision 0079): `movsxd rax,[rip+sel]; lea r10,[rip+table]; jmp [r10+rax*8]` needs no
//! annotation. `recognize_stub` matches exactly those three adjacent instructions (7 bytes each); the entry
//! count of the table is not read from a guard (there is none) but from the base relocations:
//! `resolve_stub` takes the run of consecutive 8-byte slots from `table` that are IMAGE_REL_BASED_DIR64
//! and hold a VA in code. The lifted jump is a `switch` on the effective address, like `Select::Slot`, so
//! a selector outside the run stops in `fle_jumptab_miss`. Decision 0114 adds one more case, the index -1 (the selector's initial
//! value 0xffffffff): the relocated slot in front of the table that points to the stub's own lazy-initialisation `call` (`stub_init_slot`).

//! MSVC switch tables with a proven base (decision 0089): `[cmp idx,N; ja D;] [movzx eX,byte [B+idx+T1];]
//! mov eT,[B+X*4+T2]; add rT,B; jmp rT`, where B is proven to be the image base. `recognize_msvc` reads the
//! instruction forms and N from the decoded instructions in front of the jump, `resolve_msvc` reads the tables and
//! checks every target against the `.pdata` RUNTIME_FUNCTIONs of the function (chains included), `prove_base` is the
//! base proof (the simple condition of the decision, not loosened), `verify_msvc` checks that no branch lands between
//! the guard and the jump. The jump is lifted as a `switch` on the jump register (`Select::Target`).
//! Decision 0094 replaced the base proof by reaching definitions (`prove_base_reaching_defs`); the comparison-only proofs of decisions 0089 and 0090 are deleted (decision 0243).
//! Decision 0095 adds the swapped form `mov eX,[B+I*4+T2]; add B,rX; jmp B` (the base register is the jump register), whose N
//! comes from the guard's branch on the edge into the table block (`derive_swapped_guard`), with its own checks
//! (`verify_msvc_swapped`, `landings_in_swapped_idioms`).
//! Decision 0109 adds the scheduled form: the standard idiom `[movzx;] mov eT,[B+I*4+T2]; add rT,B; jmp rT` whose guard is not in
//! front of it because the compiler scheduled other instructions into the gap (`recognize_msvc_scheduled`). Its N comes from every
//! way into the idiom (`derive_scheduled_guard`: at most two, each a `cmp I,imm` and an unsigned branch; a loop-rotated switch has a
//! second guard at the bottom of the loop) and the same two post-closure checks as the swapped form.

use crate::{Decoded, Text};
use fe_pe::Pe;
use iced_x86::{ConditionCode, FlowControl, Instruction, InstructionInfoFactory, Mnemonic, OpAccess, OpKind, Register};
use std::collections::{BTreeMap, BTreeSet, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// 8-byte absolute VAs, entry `k` at `table + 8k`. The jump is `jmp [ea]`.
    Abs64,
    /// 4-byte signed offsets from `base`: target = base + entry. The jump is `jmp reg`.
    Rel32 { base: u64 },
}

#[derive(Debug, Clone, Copy)]
pub struct Spec {
    pub jmp_va: u64,
    /// The bytes of the jump instruction.
    pub code: &'static [u8],
    /// The function `[begin, end)` that contains the jump, its table and all targets.
    pub func: (u64, u64),
    /// VA of entry 0 of the table as read by this annotation.
    pub table: u64,
    pub count: usize,
    pub kind: Kind,
}

/// What the lifted `switch` compares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Select {
    /// The effective address of the memory operand; the cases are slot addresses.
    Slot,
    /// The value of the jump register; the cases are the target VAs.
    Target,
}

/// Where a `JumpTable` came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// A hand-written `Spec` (decision 0036).
    Annotated,
    /// A selector stub (decision 0079): the selector variable and the table, as read from the two
    /// instructions in front of the jump.
    Stub { sel_var: u64, table: u64 },
    /// An absolute-pointer table inside the function's own code, reached by `lea B,[rip+k]; jmp [B+I*8+d]` (decision 0102 W3):
    /// `table` is the address `lea` loads (the first slot of the run of relocated code pointers).
    Inline { table: u64 },
    /// An MSVC switch table whose base register is proven to be the image base (decision 0089): `n` is the
    /// upper bound of the index from `cmp idx,N; ja`, `t1` the RVA of the index byte table (two-stage form),
    /// `t2` the RVA of the 32-bit RVA table, `from` the instruction after the `ja` of the guard and `function` the
    /// begin RVA of the primary `.pdata` entry of the function that holds the jump.
    ///
    /// `swapped` (decision 0095): `Some(lo)` when the jump is `mov eX,[B+I*4+T2]; add B,rX; jmp B`. Then `from` is the first
    /// instruction of the table block (where the guard's branch lands), `guard_ja` the guard's conditional branch (`ja`,
    /// `jbe`, `jae` or `jb`), `guard_cmp` its `cmp I,imm` (they are not next to each other) and `lo` the first address of
    /// the range `[lo, jump]` that nothing but the guard's branch may enter (`lo` < `from` when padding `nop`s stand in front
    /// of the table block).
    ///
    /// `entry2` (decision 0109, scheduled form only): a second way into the run `[lo, jump]`, `(branch, address it lands on)`. The
    /// scheduled form of a loop-rotated switch has the guard at the bottom of the loop jump into the middle of the run (the
    /// aligned head of the loop); `from`, `guard_ja` and `guard_cmp` are then the guard with the lowest entry address.
    Msvc {
        n: u64,
        t1: Option<u32>,
        t2: u32,
        from: u64,
        guard_ja: u64,
        guard_cmp: u64,
        function: u32,
        swapped: Option<u64>,
        entry2: Option<(u64, u64)>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JumpTable {
    pub select: Select,
    /// `(case value, target VA)`, by ascending case value, without repeated case values.
    pub cases: Vec<(u64, u64)>,
    pub origin: Origin,
}

impl JumpTable {
    /// The `kind:how` key in `indirect::Stats::by_kind`.
    pub fn kind_key(&self) -> &'static str {
        match self.origin {
            Origin::Annotated => "jmp:table",
            Origin::Stub { .. } => "jmp:sel_table",
            Origin::Inline { .. } => "jmp:inline_table",
            Origin::Msvc { .. } => "jmp:msvc_table",
        }
    }
}

/// The four annotated jumps of FLEngine_x64.dll.
///
/// - 0x40d253 (fn 0x40d210): `jmp [r10+rdx*8+0x40]`, r10 = 0x40d258 (`lea r10,[rip+5]` just before).
///   `test rdx,rdx; jle ret` leaves rdx >= 1, `cmp rdx,0x20; jg` leaves rdx <= 0x20, `sub rdx,8; jg`
///   leaves rdx-8 <= 0. So the index is -7..0 and the slots are 0x40d260..0x40d298 (8 entries).
/// - 0x5e5eb6 (fn 0x5e5860): `add ecx,-0x45; cmp ecx,0x22; ja` gives 0..34, N = 35, table 0x5e6d44.
/// - 0x5e9ae8 (fn 0x5e8da0): `add ecx,-0x3f34; cmp ecx,0x1e; ja` gives 0..30, N = 31, table 0x5eb358.
/// - 0x5f1499 (fn 0x5f05f0): `cmp edx,5; ja` gives 0..5, N = 6, table 0x5f1664.
pub const FLENGINE: &[Spec] = &[
    Spec {
        jmp_va: 0x40d253,
        code: &[0x41, 0xff, 0x64, 0xd2, 0x40],
        func: (0x40d210, 0x40d31b),
        table: 0x40d260,
        count: 8,
        kind: Kind::Abs64,
    },
    Spec {
        jmp_va: 0x5e5eb6,
        code: &[0xff, 0xe1],
        func: (0x5e5860, 0x5e6dd0),
        table: 0x5e6d44,
        count: 35,
        kind: Kind::Rel32 { base: 0x5e6d44 },
    },
    Spec {
        jmp_va: 0x5e9ae8,
        code: &[0xff, 0xe1],
        func: (0x5e8da0, 0x5eb3d4),
        table: 0x5eb358,
        count: 31,
        kind: Kind::Rel32 { base: 0x5eb358 },
    },
    Spec {
        jmp_va: 0x5f1499,
        code: &[0xff, 0xe0],
        func: (0x5f05f0, 0x5f167c),
        table: 0x5f1664,
        count: 6,
        kind: Kind::Rel32 { base: 0x5f1664 },
    },
];

/// The annotated jump of ILTools_x64.dll rebased to 0x6000000 (decision 0104, a measurement base). The function is the same routine as
/// FLEngine's 0x40d210 (a fill routine; the bytes are equal except the rip-relative displacements and the table slots), so the
/// annotation has the same form as the first entry of `FLENGINE`: `jmp [r10+rdx*8+0x40]`, r10 = 0x600a5c8 (`lea r10,[rip+5]` just before),
/// index -7..0 from the same three guards (`test rdx,rdx; jle`, `cmp rdx,0x20; jg`, `sub rdx,8; jg`), slots 0x600a5d0..0x600a608.
/// Like `FLENGINE` it names VAs, so it applies to the image at that base only; at another base no jump is found at `jmp_va`.
pub const ILTOOLS_6000000: &[Spec] = &[Spec {
    jmp_va: 0x600a5c3,
    code: &[0x41, 0xff, 0x64, 0xd2, 0x40],
    func: (0x600a580, 0x600a68b),
    table: 0x600a5d0,
    count: 8,
    kind: Kind::Abs64,
}];

fn in_func(s: &Spec, va: u64) -> bool {
    va >= s.func.0 && va < s.func.1
}

/// Reads and checks the table of `s`. Errors say which check failed.
pub fn resolve(pe: &Pe<'_>, text: &Text<'_>, s: &Spec) -> Result<JumpTable, String> {
    if !in_func(s, s.jmp_va) {
        return Err("jump is outside the annotated function".into());
    }
    match text.decode(s.jmp_va) {
        Decoded::Ok(i) if i.flow_control() == FlowControl::IndirectBranch && i.len() == s.code.len() => {}
        _ => return Err("no indirect jmp of the annotated length at the jump address".into()),
    }
    match text.slice(s.jmp_va, s.code.len()) {
        Some(b) if b == s.code => {}
        _ => return Err("jump instruction bytes differ from the annotation".into()),
    }
    let esz: u64 = match s.kind {
        Kind::Abs64 => 8,
        Kind::Rel32 { .. } => 4,
    };
    let size = esz * s.count as u64;
    // decision 0181 (hyb2): a table that lies outside .text (read-only data: the table-relative switch tables of the third-party plug-in, `Spec`s made by fe-wasmgen --rel-sites) need not be
    // inside the annotated function; a table in .text must be, as before
    if s.count == 0 || (text.contains(s.table) && (!in_func(s, s.table) || !in_func(s, s.table + size - 1))) {
        return Err("table is not inside the annotated function".into());
    }
    let rva = pe.va_to_rva(s.table).map_err(|e| format!("table address: {e}"))?;
    let raw = pe.slice_at_rva(rva, size as usize).map_err(|e| format!("table bytes: {e}"))?;
    let mut cases: BTreeMap<u64, u64> = BTreeMap::new();
    for k in 0..s.count {
        let (case, target) = match s.kind {
            Kind::Abs64 => {
                let mut a = [0u8; 8];
                a.copy_from_slice(&raw[8 * k..8 * k + 8]);
                (s.table + 8 * k as u64, u64::from_le_bytes(a))
            }
            Kind::Rel32 { base } => {
                let mut a = [0u8; 4];
                a.copy_from_slice(&raw[4 * k..4 * k + 4]);
                let t = base.wrapping_add(i32::from_le_bytes(a) as i64 as u64);
                (t, t)
            }
        };
        if !text.contains(target) || !in_func(s, target) {
            return Err(format!("entry {k} target {target:#x} is outside the annotated function or .text"));
        }
        cases.insert(case, target);
    }
    let select = match s.kind {
        Kind::Abs64 => Select::Slot,
        Kind::Rel32 { .. } => Select::Target,
    };
    Ok(JumpTable { select, cases: cases.into_iter().collect(), origin: Origin::Annotated })
}

/// Longest run of table slots `resolve_stub` accepts. The tables of dsp_ippv2_x64.dll have 6, 7 or 9.
pub const STUB_MAX_ENTRIES: usize = 64;

/// The variable and the table named by a selector stub; `start` is the address of its `movsxd` (the stub's first instruction).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StubForm {
    pub sel_var: u64,
    pub table: u64,
    pub start: u64,
}

/// `jmp` at `va` is `jmp [B+I*8]` (no displacement) and the two instructions right before it, 7 bytes each, are
/// `movsxd I,[rip+sel_var]` (4-byte load) and `lea B,[rip+table]`, in that order. Nothing else matches.
pub fn recognize_stub(text: &Text<'_>, va: u64, jmp: &Instruction) -> Option<StubForm> {
    if jmp.mnemonic() != Mnemonic::Jmp
        || jmp.op_kind(0) != OpKind::Memory
        || jmp.is_ip_rel_memory_operand()
        || jmp.memory_size().size() != 8
        || jmp.memory_displacement64() != 0
        || jmp.memory_index_scale() != 8
        || matches!(jmp.memory_segment(), Register::FS | Register::GS)
    {
        return None;
    }
    let (base, idx) = (jmp.memory_base(), jmp.memory_index());
    if base == Register::None || idx == Register::None || base.size() != 8 || idx.size() != 8 || base == idx {
        return None;
    }
    let lea = match text.decode(va.checked_sub(7)?) {
        Decoded::Ok(i)
            if i.len() == 7
                && i.mnemonic() == Mnemonic::Lea
                && i.op0_kind() == OpKind::Register
                && i.op0_register() == base
                && i.op_kind(1) == OpKind::Memory
                && i.is_ip_rel_memory_operand() =>
        {
            i
        }
        _ => return None,
    };
    let mov = match text.decode(va.checked_sub(14)?) {
        Decoded::Ok(i)
            if i.len() == 7
                && i.mnemonic() == Mnemonic::Movsxd
                && i.op0_kind() == OpKind::Register
                && i.op0_register() == idx
                && i.op_kind(1) == OpKind::Memory
                && i.is_ip_rel_memory_operand()
                && i.memory_size().size() == 4 =>
        {
            i
        }
        _ => return None,
    };
    Some(StubForm { sel_var: mov.ip_rel_memory_address(), table: lea.ip_rel_memory_address(), start: va - 14 })
}

/// Reads the table of a recognised stub. Entry `k` is the 8 bytes at `table + 8k`; the entries are the run of
/// slots from `table` that are in `relocs` (the VAs of the DIR64 slots) and hold a VA inside code. The run
/// ends at the first slot that is not relocated or does not point into code. An empty run, or a run of
/// `STUB_MAX_ENTRIES` slots, is an error.
pub fn resolve_stub(pe: &Pe<'_>, text: &Text<'_>, relocs: &HashSet<u64>, f: &StubForm) -> Result<JumpTable, String> {
    let mut cases: Vec<(u64, u64)> = Vec::new();
    for k in 0..STUB_MAX_ENTRIES as u64 {
        let slot = f.table + 8 * k;
        if !relocs.contains(&slot) {
            break;
        }
        let rva = pe.va_to_rva(slot).map_err(|e| format!("table address: {e}"))?;
        let raw = pe.slice_at_rva(rva, 8).map_err(|e| format!("table bytes: {e}"))?;
        let mut a = [0u8; 8];
        a.copy_from_slice(raw);
        let target = u64::from_le_bytes(a);
        if !text.contains(target) {
            break;
        }
        cases.push((slot, target));
    }
    if cases.is_empty() {
        return Err("no relocated code pointer at the table address".into());
    }
    if cases.len() == STUB_MAX_ENTRIES {
        return Err(format!("table has {STUB_MAX_ENTRIES} or more relocated code pointers"));
    }
    if let Some(first) = stub_init_slot(pe, text, relocs, f) {
        cases.insert(0, first);
    }
    Ok(JumpTable { select: Select::Slot, cases, origin: Origin::Stub { sel_var: f.sel_var, table: f.table } })
}

/// The lazy-initialisation path of a selector stub (decision 0114): while the selector variable still holds its initial value
/// 0xffffffff (index -1), the hardware reads the slot in front of the table. In dsp_ippv2_x64.dll that slot is a relocated pointer
/// to the 16 bytes right before the stub, `call <init>` (E8 rel32) and an 11-byte `nop word ptr [rax+rax]` that falls into the
/// stub, which then reads the (now initialised) variable again. The slot is returned as a case only when ALL of these hold:
/// it is the slot at `table - 8` and a DIR64 relocation slot, its value is exactly `start - 16`, the 16 bytes there are `E8 rel32`
/// followed by the 11-byte nop `66 66 66 0F 1F 84 00 00 00 00 00`, and the call target is inside code. Any other content of
/// that slot (for example the last entry of the table in front, which is a different address) is not a case and the
/// index -1 stays outside the table (`fle_jumptab_miss`), as in decision 0079.
fn stub_init_slot(pe: &Pe<'_>, text: &Text<'_>, relocs: &HashSet<u64>, f: &StubForm) -> Option<(u64, u64)> {
    const NOP11: [u8; 11] = [0x66, 0x66, 0x66, 0x0f, 0x1f, 0x84, 0, 0, 0, 0, 0];
    let slot = f.table.checked_sub(8)?;
    let init = f.start.checked_sub(16)?;
    if !relocs.contains(&slot) {
        return None;
    }
    let raw = pe.slice_at_rva(pe.va_to_rva(slot).ok()?, 8).ok()?;
    if u64::from_le_bytes(raw.try_into().ok()?) != init {
        return None;
    }
    let code = text.slice(init, 16)?;
    if code[0] != 0xe8 || code[5..] != NOP11 {
        return None;
    }
    let rel = i32::from_le_bytes(code[1..5].try_into().ok()?);
    let callee = (init + 5).wrapping_add(rel as i64 as u64);
    if !text.contains(callee) {
        return None;
    }
    Some((slot, init))
}

/// After the closure: the `movsxd` and the `lea` in front of the stub's `jmp` were decoded as the sequential path
/// into it, and no block starts at the `lea` or at the `jmp` (a branch landing there would arrive with other
/// register values than the stub assumes).
pub fn verify_stub(jmp_va: u64, insns: &BTreeMap<u64, Instruction>, leaders: &HashSet<u64>) -> Result<(), String> {
    let (lea, mov) = (jmp_va - 7, jmp_va - 14);
    match (insns.get(&mov), insns.get(&lea)) {
        (Some(m), Some(l)) if m.mnemonic() == Mnemonic::Movsxd && l.mnemonic() == Mnemonic::Lea => {}
        _ => return Err("the movsxd and lea in front of the stub jump were not decoded in sequence".into()),
    }
    if leaders.contains(&lea) || leaders.contains(&jmp_va) {
        return Err("a block starts inside the stub (a branch lands between its instructions)".into());
    }
    Ok(())
}

/// The table named by an inline-table jump (decision 0102 W3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InlineForm {
    pub table: u64,
}

/// `jmp` at `va` is `jmp qword [B+I*8+d]` (any displacement, no segment override, B and I different 64-bit registers) and the 7
/// bytes right before it are `lea B,[rip+k]`: B is then a constant, the address the `lea` loads. This is the shape of the
/// `FillChar`/`Move` tails of the Delphi RTL, whose table of absolute code pointers follows the `jmp` in the code (FLEngine's
/// 0x40d253 is the same idiom, annotated by hand since decision 0036). The index is not read from a guard (it is a computed
/// value, here `size - 8` after several signed compares); `resolve_inline` takes the table from the relocations instead.
pub fn recognize_inline(va: u64, text: &Text<'_>, jmp: &Instruction) -> Option<InlineForm> {
    if jmp.mnemonic() != Mnemonic::Jmp
        || jmp.op_kind(0) != OpKind::Memory
        || jmp.is_ip_rel_memory_operand()
        || jmp.memory_size().size() != 8
        || jmp.memory_index_scale() != 8
        || matches!(jmp.memory_segment(), Register::FS | Register::GS)
    {
        return None;
    }
    let (base, idx) = (jmp.memory_base(), jmp.memory_index());
    if base == Register::None || idx == Register::None || base.size() != 8 || idx.size() != 8 || base == idx {
        return None;
    }
    match text.decode(va.checked_sub(7)?) {
        Decoded::Ok(i)
            if i.len() == 7
                && i.mnemonic() == Mnemonic::Lea
                && i.op0_kind() == OpKind::Register
                && i.op0_register() == base
                && i.op_kind(1) == OpKind::Memory
                && i.is_ip_rel_memory_operand() =>
        {
            Some(InlineForm { table: i.ip_rel_memory_address() })
        }
        _ => None,
    }
}

/// Reads the table of a recognised inline-table jump. Entry `k` is the 8 bytes at `table + 8k`; the entries are the run of slots
/// from `table` that are in `relocs` (the VAs of the DIR64 slots) and hold a VA inside the pieces of the `.pdata` function that
/// contains the jump (a computed jump into its own function; a pointer to another function ends the run, as does a slot
/// that is not relocated). `None` when this is not an inline table: the jump is not inside a `.pdata` function, the `lea` target is not
/// inside that function, or the run is empty or has `STUB_MAX_ENTRIES` slots. `lea B,[rip+k]; jmp [B+I*8+d]` is a common shape (a table
/// in `.rdata` of absolute pointers to other functions, say) that is simply not this form, so nothing is reported for it, unlike for an
/// annotated jump or a selector stub, which name the table themselves.
/// The lifted jump is a `switch` on the effective address (`Select::Slot`): an address outside the run stops in
/// `fle_jumptab_miss`, so a run longer than the guards allow only adds cases that are never taken.
pub fn resolve_inline(pe: &Pe<'_>, fm: &FuncMap, relocs: &HashSet<u64>, f: &InlineForm, jmp_va: u64) -> Option<JumpTable> {
    let jmp_rva = pe.va_to_rva(jmp_va).ok()?;
    let (_, pieces) = fm.function_at(jmp_rva)?;
    let inside = |va: u64| pieces.iter().any(|&(b, e)| va >= pe.rva_to_va(b) && va < pe.rva_to_va(e));
    if !inside(f.table) {
        return None;
    }
    let mut cases: Vec<(u64, u64)> = Vec::new();
    for k in 0..STUB_MAX_ENTRIES as u64 {
        let slot = f.table + 8 * k;
        if !relocs.contains(&slot) {
            break;
        }
        let Ok(rva) = pe.va_to_rva(slot) else { break };
        let Ok(raw) = pe.slice_at_rva(rva, 8) else { break };
        let mut a = [0u8; 8];
        a.copy_from_slice(raw);
        let target = u64::from_le_bytes(a);
        if !inside(target) {
            break;
        }
        cases.push((slot, target));
    }
    if cases.is_empty() || cases.len() == STUB_MAX_ENTRIES {
        return None;
    }
    Some(JumpTable { select: Select::Slot, cases, origin: Origin::Inline { table: f.table } })
}

/// After the closure: the `lea` in front of the jump was decoded in sequence into it. A block that starts at the `jmp` is not
/// a way to a wrong target (the switch compares the address the jump really computes), so it is not checked.
pub fn verify_inline(jmp_va: u64, insns: &BTreeMap<u64, Instruction>) -> Result<(), String> {
    match insns.get(&(jmp_va - 7)) {
        Some(l) if l.mnemonic() == Mnemonic::Lea => Ok(()),
        _ => Err("the lea in front of the inline-table jump was not decoded in sequence".into()),
    }
}

/// Every target is the start of a decoded instruction and no decoded instruction overlaps it.
pub fn verify(t: &JumpTable, insns: &BTreeMap<u64, Instruction>) -> Result<(), String> {
    verify_except(t, insns, &|_| false)
}

/// Same as `verify`, with the targets for which `stubbed` answers true left out: a function entry whose body is not decoded on purpose (a pruned function,
/// decision 0168) has no instruction at its address; the table still reaches it as a call of the stub that stops the run (it is a function entry of the closure).
pub fn verify_except(t: &JumpTable, insns: &BTreeMap<u64, Instruction>, stubbed: &dyn Fn(u64) -> bool) -> Result<(), String> {
    for &(_, tg) in &t.cases {
        if stubbed(tg) {
            continue;
        }
        let i = insns.get(&tg).ok_or_else(|| format!("target {tg:#x} is not a decoded instruction start"))?;
        if let Some((&pva, p)) = insns.range(..tg).next_back() {
            if pva + p.len() as u64 > tg {
                return Err(format!("target {tg:#x} lies inside the instruction at {pva:#x}"));
            }
        }
        if let Some((&nva, _)) = insns.range(tg + 1..).next() {
            if nva < tg + i.len() as u64 {
                return Err(format!("an instruction at {nva:#x} starts inside the instruction at target {tg:#x}"));
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// MSVC switch tables with a proven base (decision 0089)
// ---------------------------------------------------------------------------------------------

/// Largest `N + 1` (entries of the index range) taken from a `cmp idx,N; ja` guard.
pub const MSVC_MAX_ENTRIES: u64 = 4096;

/// Most instructions (`cdqe`, `lea B,[rip+d]`, padding `nop`; decision 0093) allowed between the `ja` and the table idiom.
pub const MSVC_SKIP: usize = 2;

/// Upper bound of the byte length from the `cmp` of the guard to the `jmp`: cmp 8, ja 6, `MSVC_SKIP` x 15 (the longest
/// x86 instruction, a padding `nop`; decision 0093), movzx 10, mov 10, add 4, jmp 3.
pub const MSVC_IDIOM_MAX: u64 = 8 + 6 + 15 * MSVC_SKIP as u64 + 10 + 10 + 4 + 3;

/// Most instructions allowed between the table load `mov eX,[B+I*4+T2]` and the `jmp B` of the swapped form (decision 0095),
/// in both gaps together (the ones before and after the `add B,rX`). A separate limit from `MSVC_SKIP`; the one known form
/// has the two instructions `add rcx,r8; mov rax,r11` after the `add`.
pub const MSVC_SWAP_SKIP: usize = 2;

/// Most instructions `derive_scheduled_guard` (decision 0109) walks back from the first instruction of the idiom to the guard.
pub const MSVC_SCHED_WALK: usize = 16;

/// Most guards (ways into the idiom, each with its own `cmp I,imm` and branch) of one scheduled idiom (decision 0109): the one in
/// front of the idiom and, in a loop-rotated switch, the one at the bottom of the loop.
pub const MSVC_SCHED_GUARDS: usize = 2;

/// Options of the closure that are not part of the image.
#[derive(Debug, Clone, Copy, Default)]
pub struct Options {}

/// The `.pdata` RUNTIME_FUNCTIONs with their chains resolved (`fe_pe::Pe::function_table`): a function is the primary
/// RUNTIME_FUNCTION and every entry chained to it.
pub struct FuncMap {
    /// (begin_rva, end_rva, primary begin_rva), ascending
    entries: Vec<(u32, u32, u32)>,
    pieces: BTreeMap<u32, Vec<(u32, u32)>>,
}

impl FuncMap {
    pub fn from_pe(pe: &Pe<'_>) -> Result<FuncMap, String> {
        let table = pe.function_table().map_err(|e| format!(".pdata: {e}"))?;
        let mut entries: Vec<(u32, u32, u32)> =
            table.iter().map(|e| (e.function.begin_rva, e.function.end_rva, e.primary.begin_rva)).collect();
        entries.sort_unstable();
        entries.dedup();
        let mut pieces: BTreeMap<u32, Vec<(u32, u32)>> = BTreeMap::new();
        for &(b, e, p) in &entries {
            pieces.entry(p).or_default().push((b, e));
        }
        Ok(FuncMap { entries, pieces })
    }

    /// The function that has a RUNTIME_FUNCTION containing `rva`: (primary begin RVA, all its `[begin, end)` pieces).
    pub fn function_at(&self, rva: u32) -> Option<(u32, &[(u32, u32)])> {
        let k = self.entries.partition_point(|e| e.0 <= rva).checked_sub(1)?;
        let &(b, e, p) = self.entries.get(k)?;
        if rva >= b && rva < e {
            Some((p, self.pieces.get(&p)?.as_slice()))
        } else {
            None
        }
    }
}

/// Linear sweep of the pieces of one function (ascending). Bytes that do not decode (switch tables inside the range) are
/// skipped.
pub struct Sweep {
    insns: Vec<Instruction>,
    pieces: Vec<(u64, u64)>,
}

impl Sweep {
    pub fn build(pe: &Pe<'_>, text: &Text<'_>, pieces: &[(u32, u32)]) -> Result<Sweep, String> {
        let mut insns = Vec::new();
        let mut ranges = Vec::new();
        for &(b, e) in pieces {
            let (va, end) = (pe.rva_to_va(b), pe.rva_to_va(e));
            let bytes = text
                .slice(va, (end - va) as usize)
                .ok_or_else(|| format!("function piece [{va:#x}, {end:#x}) is not inside the executable sections"))?;
            let mut d = iced_x86::Decoder::with_ip(64, bytes, va, iced_x86::DecoderOptions::NONE);
            while d.can_decode() {
                let i = d.decode();
                if !i.is_invalid() {
                    insns.push(i);
                }
            }
            ranges.push((va, end));
        }
        insns.sort_by_key(|i| i.ip());
        Ok(Sweep { insns, pieces: ranges })
    }

    fn index_of(&self, va: u64) -> Option<usize> {
        self.insns.binary_search_by_key(&va, |i| i.ip()).ok()
    }

    pub fn covers(&self, va: u64) -> bool {
        self.pieces.iter().any(|&(b, e)| va >= b && va < e)
    }

    pub fn is_start(&self, va: u64) -> bool {
        self.index_of(va).is_some()
    }

    /// The `[begin, end)` VA ranges of the pieces of the function.
    pub fn pieces(&self) -> &[(u64, u64)] {
        &self.pieces
    }
}

/// The instruction forms and the guard in front of a `jmp reg` (decision 0089, 1(b), 1(c)).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MsvcForm {
    /// First instruction of the idiom (`movzx` of the two-stage form, else the `mov` from the RVA table).
    pub first: u64,
    /// The instruction after the `ja` of the guard: the run from here to the jump must have no other entry.
    pub from: u64,
    /// The `ja` and the `cmp` of the guard (the closure must have decoded them as such).
    pub guard_ja: u64,
    pub guard_cmp: u64,
    /// B: the register that `add rT,B` adds.
    pub base: Register,
    /// Upper bound of the index from `cmp idx,N; ja`.
    pub n: u64,
    /// `[B + idx + T1]` of the index byte table (two-stage form).
    pub t1: Option<u32>,
    /// `[B + X*4 + T2]` of the 32-bit RVA table.
    pub t2: u32,
    /// `Some` for the swapped form (decision 0095). Then `n`, `from`, `guard_ja` and `guard_cmp` are **placeholders (0)**:
    /// the guard is not in front of the idiom, `recognize_msvc` reads only the closure's instructions in front of the
    /// jump, and `resolve_msvc` derives them from the CFG of the function's sweep (`derive_swapped_guard`) and writes
    /// them into the table's `Origin::Msvc`. `guard_ja` is then the conditional branch of the guard, `from` the
    /// first instruction of the table block (the branch's target, or the instruction after the branch when the table
    /// block is its fall-through side). Decision 0109: also `Some` for the scheduled form (`SwapForm::scheduled`), whose guard is
    /// derived the same way, from all the ways into the idiom (`derive_scheduled_guard`).
    pub swapped: Option<SwapForm>,
}

/// What `recognize_msvc` found of the swapped form `mov eX,[B+I*4+T2]; add B,rX; jmp B` (decision 0095).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwapForm {
    /// The table load `mov eX,[B+I*4+T2]`.
    pub load: u64,
    /// The `add B,rX` that turns B from the image base into the target.
    pub add: u64,
    /// I, the index register of the load.
    pub index: Register,
    /// Decision 0109: the form is the standard `add rT,B` one with a scheduled guard (`recognize_msvc_scheduled`). Then `load` is the
    /// first instruction of the idiom (the `movzx` of the two-stage form), `add` the `add rT,B` and `index` the register the first
    /// instruction indexes with (the one that the guard must bound, possibly through a sign extension).
    pub scheduled: bool,
}

/// The one instruction of the closure that ends exactly at `at` (`insns` holds what the closure decoded going forward).
/// `Ok(None)`: none. `Err`: two overlapping decodings end there.
fn prev_insn(insns: &BTreeMap<u64, Instruction>, at: u64) -> Result<Option<&Instruction>, String> {
    let mut it = insns.range(at.saturating_sub(15)..at).map(|(_, i)| i).filter(|i| i.next_ip() == at);
    let first = it.next();
    if it.next().is_some() {
        return Err(format!("two overlapping instructions of the closure end at {at:#x}"));
    }
    Ok(first)
}

fn is_near_direct(i: &Instruction) -> bool {
    matches!(i.op_kind(0), OpKind::NearBranch16 | OpKind::NearBranch32 | OpKind::NearBranch64)
}

/// A padding `nop` (decision 0093): `Mnemonic::Nop` (`90`, `66 90` = `xchg ax,ax`, the multi-byte `0f 1f /0` with `66`
/// prefixes) that, by what `iced_x86` reports, writes no register, writes no memory and modifies no flag. `pause`
/// (`f3 90`), `xchg` of other registers (also `87 c0`, which zero-extends rax), `int3`, `ud2` and the reserved-nop opcodes
/// (`0f 19`..`0f 1e`) have other mnemonics and are not padding.
pub fn is_padding_nop(i: &Instruction) -> bool {
    if i.mnemonic() != Mnemonic::Nop {
        return false;
    }
    let writes = |a: OpAccess| matches!(a, OpAccess::Write | OpAccess::CondWrite | OpAccess::ReadWrite | OpAccess::ReadCondWrite);
    let mut factory = InstructionInfoFactory::new();
    let info = factory.info(i);
    i.rflags_modified() == 0
        && !info.used_memory().iter().any(|m| writes(m.access()))
        && !info.used_registers().iter().any(|r| writes(r.access()))
}

fn disp_u32(i: &Instruction) -> Option<u32> {
    u32::try_from(i.memory_displacement64() as i64).ok()
}

/// `mov eT,[B + I*4 + T]` (no segment override, no RIP base): returns (I, T).
fn table_load(i: &Instruction, rt: Register, b: Register) -> Option<(Register, u32)> {
    if i.mnemonic() != Mnemonic::Mov
        || i.op_count() != 2
        || i.op0_kind() != OpKind::Register
        || i.op0_register().size() != 4
        || i.op0_register().full_register() != rt
        || i.op_kind(1) != OpKind::Memory
        || i.memory_size().size() != 4
        || i.segment_prefix() != Register::None
        || i.is_ip_rel_memory_operand()
        || i.memory_base() != b
        || i.memory_index_scale() != 4
    {
        return None;
    }
    let idx = i.memory_index();
    if !idx.is_gpr64() || idx == b || idx == Register::RSP {
        return None;
    }
    Some((idx, disp_u32(i)?))
}

/// `movzx eX,byte [B + I0 + T1]` with X = `x` (either order of the two registers): returns (I0, T1).
fn byte_load(i: &Instruction, x: Register, b: Register) -> Option<(Register, u32)> {
    if i.mnemonic() != Mnemonic::Movzx
        || i.op_count() != 2
        || i.op0_kind() != OpKind::Register
        || i.op0_register().size() != 4
        || i.op0_register().full_register() != x
        || i.op_kind(1) != OpKind::Memory
        || i.memory_size().size() != 1
        || i.segment_prefix() != Register::None
        || i.is_ip_rel_memory_operand()
        || i.memory_index_scale() != 1
    {
        return None;
    }
    let (r1, r2) = (i.memory_base(), i.memory_index());
    if !r1.is_gpr64() || !r2.is_gpr64() || r1 == r2 {
        return None;
    }
    let other = if r1 == b {
        r2
    } else if r2 == b {
        r1
    } else {
        return None;
    };
    if other == Register::RSP {
        return None;
    }
    Some((other, disp_u32(i)?))
}

/// `jmp rT` at `va` that is the end of an MSVC switch idiom (decision 0089, 1(b), 1(c)):
///
/// ```text
///   cmp idx,N ; ja default                         ; idx = I0, 32- or 64-bit, N an immediate
///   [cdqe] [lea B,[rip+d]] [nop]                   ; optional, index-preserving (see MSVC_SKIP)
///   [movzx eX, byte [B + I0 + T1]]                 ; (c2) only
///   mov eT, dword [B + I*4 + T2]                   ; I = I0 (c1) or X (c2)
///   add rT, B
///   jmp rT
/// ```
///
/// The instructions are the ones the closure decoded going forward (`insns`, which already holds the `jmp`): each step
/// back takes the one instruction that ends where the next begins, so there is no guessing about instruction lengths. The
/// `ja` ends a block of the closure; its fall-through is where the closure continued, so the `ja` and the `cmp` before it
/// were decoded earlier. `Ok(None)`: no `add rT,B` with a `mov eT,[B+I*4+T]` before it (not this form, nothing to
/// report). `Err`: the table form is there but N is not derived (or the closure's decoding is ambiguous or missing).
/// `verify_msvc` later checks that no block starts inside the idiom.
///
/// This is the form of decisions 0089 and 0093 (`add rT,B`). `recognize_msvc` tries it first and, when it finds no table
/// idiom (`Ok(None)`), the swapped form of decision 0095 (`recognize_msvc_swapped`); a result of the first (also `Err`)
/// is never changed by the second.
pub fn recognize_msvc_standard(insns: &BTreeMap<u64, Instruction>, va: u64, jmp: &Instruction) -> Result<Option<MsvcForm>, String> {
    if jmp.mnemonic() != Mnemonic::Jmp || jmp.op_kind(0) != OpKind::Register || !jmp.op0_register().is_gpr64() {
        return Ok(None);
    }
    let rt = jmp.op0_register();
    let Some(add) = prev_insn(insns, va)? else { return Ok(None) };
    if !(add.mnemonic() == Mnemonic::Add
        && add.op_count() == 2
        && add.op0_kind() == OpKind::Register
        && add.op0_register() == rt
        && add.op1_kind() == OpKind::Register
        && add.op1_register().is_gpr64()
        && add.op1_register() != rt)
    {
        return Ok(None);
    }
    let b = add.op1_register();
    let Some(ld) = prev_insn(insns, add.ip())? else { return Ok(None) };
    let Some((idx, t2)) = table_load(ld, rt, b) else { return Ok(None) };
    // (c2) when the instruction before the load is the byte load of the index; otherwise (c1)
    let (first, i0, t1) = match prev_insn(insns, ld.ip())? {
        Some(zx) => match byte_load(zx, idx, b) {
            Some((i0, t1)) => (zx.ip(), i0, Some(t1)),
            None => (ld.ip(), idx, None),
        },
        None => (ld.ip(), idx, None),
    };
    // between the `ja` and the idiom only instructions that leave the checked index as it is: `lea B,[rip+d]` (writes B
    // only; B is not the index) and `cdqe` (the index is in rax and, after `cmp .., N; ja` with N < 4096, its value is
    // below 2^31, which `cdqe` keeps), and a padding `nop` (decision 0093: changes no register, flag or memory). At most
    // MSVC_SKIP of them. Anything else: N is not derived.
    let mut cur = first;
    let mut skipped = 0;
    let ja = loop {
        let Some(i) = prev_insn(insns, cur)? else {
            return Err(format!("N not derived: the closure decoded nothing before {cur:#x}"));
        };
        if i.mnemonic() == Mnemonic::Ja && is_near_direct(i) {
            break i;
        }
        let neutral = (i.mnemonic() == Mnemonic::Cdqe && i0 == Register::RAX)
            || (i.mnemonic() == Mnemonic::Lea
                && i.op0_kind() == OpKind::Register
                && i.op0_register() == b
                && i.op_kind(1) == OpKind::Memory
                && i.is_ip_rel_memory_operand())
            || is_padding_nop(i);
        if !neutral {
            return Err("N not derived: no `ja` before the table idiom (only `cdqe`, `lea B,[rip+d]` and `nop` may sit between)".into());
        }
        skipped += 1;
        if skipped > MSVC_SKIP {
            return Err("N not derived: too many instructions between the `ja` and the table idiom".into());
        }
        cur = i.ip();
    };
    let Some(cmp) = prev_insn(insns, ja.ip())? else {
        return Err("N not derived: the closure decoded nothing before the `ja`".into());
    };
    if !(cmp.mnemonic() == Mnemonic::Cmp
        && cmp.op_count() == 2
        && cmp.op0_kind() == OpKind::Register
        && matches!(cmp.op0_register().size(), 4 | 8)
        && cmp.op0_register().full_register() == i0
        && matches!(
            cmp.op1_kind(),
            OpKind::Immediate8 | OpKind::Immediate8to32 | OpKind::Immediate8to64 | OpKind::Immediate32 | OpKind::Immediate32to64
        ))
    {
        return Err("N not derived: no `cmp idx,imm` right before the `ja`".into());
    }
    let imm = cmp.immediate(1);
    let n = if cmp.op0_register().size() == 4 { imm & 0xffff_ffff } else { imm };
    if n >= MSVC_MAX_ENTRIES {
        return Err(format!("N not derived: the bound {n:#x} allows {MSVC_MAX_ENTRIES} or more entries"));
    }
    Ok(Some(MsvcForm { first, from: ja.next_ip(), guard_ja: ja.ip(), guard_cmp: cmp.ip(), base: b, n, t1, t2, swapped: None }))
}

/// `jmp rT` at `va` is the end of an MSVC switch idiom of either form: the one of decisions 0089 and 0093 or, if that finds
/// no table idiom, the swapped form of decision 0095. `Ok(None)`: neither (nothing to report). `Err`: a table form is
/// there but N is not derived or a condition on its instructions fails. The closure calls the two forms itself
/// (`recognize_msvc_standard`, `recognize_msvc_swapped`) because the swapped form belongs to the base proof by reaching
/// definitions only (the base proof of decision 0094).
pub fn recognize_msvc(insns: &BTreeMap<u64, Instruction>, va: u64, jmp: &Instruction) -> Result<Option<MsvcForm>, String> {
    match recognize_msvc_standard(insns, va, jmp)? {
        Some(f) => Ok(Some(f)),
        None => recognize_msvc_swapped(insns, va, jmp),
    }
}

/// The standard table idiom `[movzx eX,byte [B+I0+T1];] mov eT,[B+I*4+T2]; add rT,B; jmp rT` read backwards from `jmp rT` at `va`,
/// **without** a guard in front of it (decision 0109). It is the first half of `recognize_msvc_standard` (the same instruction
/// forms, the same adjacency), kept as a copy so that function stays as decisions 0089 and 0093 left it. The caller tries it when
/// `recognize_msvc_standard` found the idiom but could not derive N (`Err`): the compiler scheduled other instructions between the
/// `cmp; ja` and the idiom, or between the `cmp` and the `ja`, or the idiom has more than one way in. The result is a
/// `MsvcForm` with `swapped: Some(SwapForm { scheduled: true, .. })` and the placeholders `n`, `from`, `guard_ja`, `guard_cmp`
/// of 0: `resolve_msvc` derives them from the CFG of the function (`derive_scheduled_guard`). `Ok(None)`: no standard idiom.
pub fn recognize_msvc_scheduled(insns: &BTreeMap<u64, Instruction>, va: u64, jmp: &Instruction) -> Result<Option<MsvcForm>, String> {
    if jmp.mnemonic() != Mnemonic::Jmp || jmp.op_kind(0) != OpKind::Register || !jmp.op0_register().is_gpr64() {
        return Ok(None);
    }
    let rt = jmp.op0_register();
    let Some(add) = prev_insn(insns, va)? else { return Ok(None) };
    if !(add.mnemonic() == Mnemonic::Add
        && add.op_count() == 2
        && add.op0_kind() == OpKind::Register
        && add.op0_register() == rt
        && add.op1_kind() == OpKind::Register
        && add.op1_register().is_gpr64()
        && add.op1_register() != rt)
    {
        return Ok(None);
    }
    let b = add.op1_register();
    let Some(ld) = prev_insn(insns, add.ip())? else { return Ok(None) };
    let Some((idx, t2)) = table_load(ld, rt, b) else { return Ok(None) };
    let (first, i0, t1) = match prev_insn(insns, ld.ip())? {
        Some(zx) => match byte_load(zx, idx, b) {
            Some((i0, t1)) => (zx.ip(), i0, Some(t1)),
            None => (ld.ip(), idx, None),
        },
        None => (ld.ip(), idx, None),
    };
    Ok(Some(MsvcForm {
        first,
        from: 0,
        guard_ja: 0,
        guard_cmp: 0,
        base: b,
        n: 0,
        t1,
        t2,
        swapped: Some(SwapForm { load: first, add: add.ip(), index: i0, scheduled: true }),
    }))
}

fn writes(a: OpAccess) -> bool {
    matches!(a, OpAccess::Write | OpAccess::CondWrite | OpAccess::ReadWrite | OpAccess::ReadCondWrite)
}

/// Whether `i` writes the register `r` (any part of it; implicit writes included, as `iced_x86` reports them).
fn writes_gpr(info: &mut InstructionInfoFactory, i: &Instruction, r: Register) -> bool {
    info.info(i).used_registers().iter().any(|u| u.register().is_gpr() && u.register().full_register() == r && writes(u.access()))
}

/// An instruction that may sit inside the swapped idiom (decision 0095): it does not change the control flow
/// (`FlowControl::Next`), reads no flag, writes no memory and neither reads nor writes any of the registers `regs`
/// (full registers; implicit uses included).
fn swap_gap_ok(info: &mut InstructionInfoFactory, i: &Instruction, regs: &[Register]) -> bool {
    if i.flow_control() != FlowControl::Next || i.rflags_read() != 0 {
        return false;
    }
    let inf = info.info(i);
    if inf.used_memory().iter().any(|m| writes(m.access())) {
        return false;
    }
    !inf.used_registers().iter().any(|u| u.register().is_gpr() && regs.contains(&u.register().full_register()))
}

/// The swapped form of decision 0095, read backwards from `jmp B` at `va` (B = the jump register):
///
/// ```text
///   mov eX, dword [B + I*4 + T2]       ; the load (X is neither B nor I)
///   [instructions that touch none of B, X, I]      ; together with the ones below at most MSVC_SWAP_SKIP
///   add B, rX                          ; B becomes the target
///   [instructions that do not touch B]
///   jmp B
/// ```
///
/// "Instructions that ..." do not change the control flow, read no flag and write no memory (`swap_gap_ok`). After the `add`
/// X and I are free (their values are used up: `add rcx,r8; mov rax,r11` of the one known form reads I and writes X); before
/// it B, X and I are untouched. The guard (N) is not looked for here: it is derived by `resolve_msvc` (`MsvcForm::swapped`).
/// `Ok(None)`: the walk back from the jump does not reach an `add B,rX` and a `mov eX,[B+I*4+T2]` within the limit; `Err`: the
/// load and the `add` are there but an instruction between them is not allowed.
pub fn recognize_msvc_swapped(insns: &BTreeMap<u64, Instruction>, va: u64, jmp: &Instruction) -> Result<Option<MsvcForm>, String> {
    if jmp.mnemonic() != Mnemonic::Jmp || jmp.op_kind(0) != OpKind::Register || !jmp.op0_register().is_gpr64() {
        return Ok(None);
    }
    let b = jmp.op0_register();
    if b == Register::RSP {
        return Ok(None);
    }
    let mut info = InstructionInfoFactory::new();
    let mut after: Vec<&Instruction> = Vec::new();
    let mut cur = va;
    let add = loop {
        let Some(p) = prev_insn(insns, cur)? else { return Ok(None) };
        if p.mnemonic() == Mnemonic::Add
            && p.op_count() == 2
            && p.op0_kind() == OpKind::Register
            && p.op0_register() == b
            && p.op1_kind() == OpKind::Register
            && p.op1_register().is_gpr64()
            && p.op1_register() != b
            && p.op1_register() != Register::RSP
        {
            break p;
        }
        if after.len() >= MSVC_SWAP_SKIP || !swap_gap_ok(&mut info, p, &[b]) {
            return Ok(None);
        }
        after.push(p);
        cur = p.ip();
    };
    let x = add.op1_register();
    let mut between: Vec<&Instruction> = Vec::new();
    cur = add.ip();
    let (ld, idx, t2) = loop {
        let Some(p) = prev_insn(insns, cur)? else { return Ok(None) };
        if let Some((idx, t2)) = table_load(p, x, b) {
            break (p, idx, t2);
        }
        if after.len() + between.len() >= MSVC_SWAP_SKIP {
            return Ok(None);
        }
        between.push(p);
        cur = p.ip();
    };
    if idx == x {
        return Ok(None);
    }
    if let Some(g) = between.iter().find(|g| !swap_gap_ok(&mut info, g, &[b, x, idx])) {
        return Err(format!(
            "the instruction at {:#x} between the table load and the `add {b:?},{x:?}` is not allowed in the swapped idiom (it changes the control flow, reads a flag, writes memory, or reads or writes {b:?}, {x:?} or {idx:?}; decision 0095)",
            g.ip()
        ));
    }
    Ok(Some(MsvcForm {
        first: ld.ip(),
        from: 0,
        guard_ja: 0,
        guard_cmp: 0,
        base: b,
        n: 0,
        t1: None,
        t2,
        swapped: Some(SwapForm { load: ld.ip(), add: add.ip(), index: idx, scheduled: false }),
    }))
}

/// The guard that `derive_swapped_guard` read from the CFG of the sweep.
struct SwapGuard {
    cmp: u64,
    br: u64,
    /// first instruction of the table block
    from: u64,
    /// first address of the range that only the guard's branch may enter (padding `nop`s in front of the block included)
    lo: u64,
    n: u64,
}

fn negate_cc(cc: ConditionCode) -> Option<ConditionCode> {
    match cc {
        ConditionCode::a => Some(ConditionCode::be),
        ConditionCode::be => Some(ConditionCode::a),
        ConditionCode::ae => Some(ConditionCode::b),
        ConditionCode::b => Some(ConditionCode::ae),
        _ => None,
    }
}

/// Whether control can continue from `i` to the instruction after it.
fn falls_through(i: &Instruction) -> bool {
    matches!(
        i.flow_control(),
        FlowControl::Next | FlowControl::Call | FlowControl::IndirectCall | FlowControl::ConditionalBranch | FlowControl::XbeginXabortXend
    )
}

/// N of the swapped form (decision 0095, item 2), from the CFG of the linear sweep of the function: the basic block of the
/// table load has exactly one predecessor, a conditional branch whose `cmp I,imm` precedes it in its own block, and the edge
/// into the table block is the side where I <= N (unsigned): the `ja` not taken, the `jbe` taken, the `jae` not taken or the
/// `jb` taken (the `ConditionCode` of the branch decides; `jb imm` gives N = imm - 1). Simple forms only:
///
/// - (a) the table block is the straight-line run of `FlowControl::Next` instructions that no branch of the sweep targets
///   (but its first); its predecessors are the direct branches to its first instruction and the instruction before it that
///   falls through, where a padding `nop` (`is_padding_nop`) that no branch targets and that follows an instruction
///   that does not fall through is not a predecessor. Exactly one, and the function entry is none of them;
/// - (b) in the block of the branch, the last instruction that writes the flags is `cmp I,imm` and nothing between it and the
///   branch writes I;
/// - (c) nothing from the table block's first instruction to the load writes I.
///
/// Anything else is `N not derived`. Branches that the sweep does not see (an unresolved indirect jump of the function that
/// lands there) are not predecessors here; the closure's own sources are checked after it (`landings_in_swapped_idioms`).
fn derive_swapped_guard(sw: &Sweep, entry_va: u64, s: &SwapForm) -> Result<SwapGuard, String> {
    let nd = |why: String| -> Result<SwapGuard, String> { Err(format!("N not derived (swapped form, decision 0095): {why}")) };
    let ins = &sw.insns;
    let load = sw.index_of(s.load).ok_or_else(|| format!("the table load {:#x} does not decode in the sweep", s.load))?;
    let idx = s.index;
    let mut info = InstructionInfoFactory::new();
    // direct branches and direct calls: a call into the table block is a way in that is not the guard's branch
    let enters = |i: &Instruction| {
        matches!(i.flow_control(), FlowControl::UnconditionalBranch | FlowControl::ConditionalBranch | FlowControl::Call) && is_near_direct(i)
    };
    let mut branch_targets: HashSet<u64> = HashSet::new();
    for i in ins {
        if enters(i) {
            branch_targets.insert(i.near_branch_target());
        }
    }
    // `k` falls straight into `k + 1`
    let straight = |k: usize| k + 1 < ins.len() && ins[k].flow_control() == FlowControl::Next && ins[k].next_ip() == ins[k + 1].ip();

    // (a) the table block: back from the load while the instruction before falls straight into it and nothing branches to it
    let mut p = load;
    while p > 0 && straight(p - 1) && !branch_targets.contains(&ins[p].ip()) {
        p -= 1;
    }
    let sip = ins[p].ip();
    if ins[p..=load].iter().any(|i| i.ip() == entry_va) {
        return nd(format!("the block of the table load contains the function entry {entry_va:#x}, a way in that is not the guard's branch"));
    }
    // predecessors: (index of the source, whether the edge is the branch's target side)
    let mut edges: Vec<(usize, bool)> = Vec::new();
    for (k, i) in ins.iter().enumerate() {
        if enters(i) && i.near_branch_target() == sip {
            edges.push((k, true));
        }
    }
    let mut q = p;
    // `q` ends at the first padding `nop` in front of the block that no branch targets (or at `p`)
    while q > 0 && ins[q - 1].next_ip() == ins[q].ip() {
        let pr = &ins[q - 1];
        if is_padding_nop(pr) && !branch_targets.contains(&pr.ip()) {
            q -= 1;
            continue;
        }
        if falls_through(pr) {
            edges.push((q - 1, false));
        }
        break;
    }
    if edges.len() != 1 {
        return nd(format!("the block of the table load at {sip:#x} has {} predecessors; it needs exactly one, the guard's conditional branch", edges.len()));
    }
    let (bri, taken) = edges[0];
    let br = &ins[bri];
    if br.flow_control() != FlowControl::ConditionalBranch || !is_near_direct(br) {
        return nd(format!("the only predecessor of the block of the table load at {sip:#x}, the `{:?}` at {:#x}, is not a conditional branch", br.mnemonic(), br.ip()));
    }
    let cc = br.condition_code();
    let side = if taken { Some(cc) } else { negate_cc(cc) };
    let below = match side {
        Some(ConditionCode::be) => false,
        Some(ConditionCode::b) => true,
        _ => {
            return nd(format!(
                "the edge of the `{:?}` at {:#x} into the table block ({}) is not an unsigned I <= N side (`ja` not taken, `jbe` taken, `jae` not taken, `jb` taken)",
                br.mnemonic(),
                br.ip(),
                if taken { "taken" } else { "not taken" }
            ))
        }
    };

    // (b) the block of the branch: the last instruction that writes the flags is `cmp I,imm`; nothing after it writes I
    let mut j = bri;
    let cmp = loop {
        if j == 0 || !straight(j - 1) || branch_targets.contains(&ins[j].ip()) {
            return nd(format!("no `cmp {idx:?},imm` sets the flags of the `{:?}` at {:#x} in its own block", br.mnemonic(), br.ip()));
        }
        j -= 1;
        let i = &ins[j];
        if i.mnemonic() == Mnemonic::Cmp
            && i.op_count() == 2
            && i.op0_kind() == OpKind::Register
            && matches!(i.op0_register().size(), 4 | 8)
            && i.op0_register().full_register() == idx
            && matches!(
                i.op1_kind(),
                OpKind::Immediate8 | OpKind::Immediate8to32 | OpKind::Immediate8to64 | OpKind::Immediate32 | OpKind::Immediate32to64
            )
        {
            break i;
        }
        if i.rflags_modified() != 0 {
            return nd(format!("the flags of the `{:?}` at {:#x} are last written by the `{:?}` at {:#x}, not by a `cmp {idx:?},imm`", br.mnemonic(), br.ip(), i.mnemonic(), i.ip()));
        }
        if writes_gpr(&mut info, i, idx) {
            return nd(format!("{idx:?} is written at {:#x}, between the `cmp` and the `{:?}` at {:#x}", i.ip(), br.mnemonic(), br.ip()));
        }
    };
    // (c) from the first instruction of the table block to the load
    if let Some(w) = ins[p..load].iter().find(|i| writes_gpr(&mut info, i, idx)) {
        return nd(format!("{idx:?} is written at {:#x}, between the guard's branch and the table load", w.ip()));
    }
    let imm = cmp.immediate(1);
    let imm = if cmp.op0_register().size() == 4 { imm & 0xffff_ffff } else { imm };
    let n = if below {
        match imm.checked_sub(1) {
            Some(n) => n,
            None => return nd("`jb 0` leaves no index".into()),
        }
    } else {
        imm
    };
    if n >= MSVC_MAX_ENTRIES {
        return nd(format!("the bound {n:#x} allows {MSVC_MAX_ENTRIES} or more entries"));
    }
    Ok(SwapGuard { cmp: cmp.ip(), br: br.ip(), from: sip, lo: ins[q].ip(), n })
}

/// One way into a scheduled idiom (decision 0109): an unsigned conditional branch and the `cmp I,imm` that decides it.
#[derive(Clone, Copy)]
struct SchedEntry {
    cmp: u64,
    br: u64,
    /// the address the edge lands on: the branch's target, or the instruction after the branch
    at: u64,
    /// the largest value of the index on this edge
    n: u64,
}

/// What `derive_scheduled_guard` read from the CFG.
struct SchedGuard {
    /// the largest of the bounds of the entries
    n: u64,
    /// the first address of the run that only the guards' branches may enter (padding `nop`s and the other instructions between
    /// the guard and the idiom are inside it)
    lo: u64,
    /// the entry with the lowest address, and the second one if the idiom has two
    first: SchedEntry,
    second: Option<SchedEntry>,
}

/// A step that keeps the value of the checked index: `cdqe` (rax) and `movsxd rD,eS` (D may differ from S). For a 32-bit compare
/// `cmp eS,N; ja` (N < 4096, so eS < 2^31) and for a 64-bit compare, the sign extension of eS is the value that was compared.
/// `cur` is the register that must hold the checked value after the instruction; the result is the register that held it before
/// (`None`: `i` is not such a step). Any other write to `cur` ends the derivation (a copy by `mov`, an offset by `lea` or `sub`).
fn sext_source(i: &Instruction, cur: Register) -> Option<Register> {
    match i.mnemonic() {
        Mnemonic::Cdqe if cur == Register::RAX => Some(Register::RAX),
        Mnemonic::Movsxd
            if i.op_count() == 2
                && i.op0_kind() == OpKind::Register
                && i.op0_register() == cur
                && i.op1_kind() == OpKind::Register
                && i.op1_register().size() == 4 =>
        {
            Some(i.op1_register().full_register())
        }
        _ => None,
    }
}

/// The `cmp I,imm` that decides the conditional branch `ins[kb]` on the edge into the idiom (`taken`: the edge is the branch's
/// target; else its fall-through), as `derive_swapped_guard` reads it (decision 0095, item 2): the edge must be the unsigned
/// `I <= N` side (`ja` not taken, `jbe` taken, `jae` not taken, `jb` taken: `jb imm` gives N = imm - 1); the instructions from the
/// `cmp` to the branch are one straight run that no branch enters after the `cmp`; the last flag writer in it is
/// `cmp reg,imm` with `reg` the checked register (`cur`, followed back over sign extensions); no other instruction in it changes `cur`.
/// Other instructions in the run (stores, `lea`, `mov` of other registers) are allowed (decision 0109). `at` of the result is 0
/// (the caller knows it).
fn sched_find_cmp(
    ins: &[Instruction],
    targets: &BTreeMap<u64, Vec<usize>>,
    info: &mut InstructionInfoFactory,
    kb: usize,
    taken: bool,
    mut cur: Register,
) -> Result<SchedEntry, String> {
    let br = &ins[kb];
    let cc = br.condition_code();
    let side = if taken { Some(cc) } else { negate_cc(cc) };
    let below = match side {
        Some(ConditionCode::be) => false,
        Some(ConditionCode::b) => true,
        _ => {
            return Err(format!(
                "the edge of the `{:?}` at {:#x} into the idiom ({}) is not an unsigned I <= N side (`ja` not taken, `jbe` taken, `jae` not taken, `jb` taken)",
                br.mnemonic(),
                br.ip(),
                if taken { "taken" } else { "not taken" }
            ))
        }
    };
    let mut j = kb;
    loop {
        if j == 0
            || ins[j - 1].flow_control() != FlowControl::Next
            || ins[j - 1].next_ip() != ins[j].ip()
            || targets.contains_key(&ins[j].ip())
        {
            return Err(format!("no `cmp {cur:?},imm` sets the flags of the `{:?}` at {:#x} in its own block", br.mnemonic(), br.ip()));
        }
        j -= 1;
        let i = &ins[j];
        if i.mnemonic() == Mnemonic::Cmp
            && i.op_count() == 2
            && i.op0_kind() == OpKind::Register
            && matches!(i.op0_register().size(), 4 | 8)
            && i.op0_register().full_register() == cur
            && matches!(
                i.op1_kind(),
                OpKind::Immediate8 | OpKind::Immediate8to32 | OpKind::Immediate8to64 | OpKind::Immediate32 | OpKind::Immediate32to64
            )
        {
            let imm = i.immediate(1);
            let imm = if i.op0_register().size() == 4 { imm & 0xffff_ffff } else { imm };
            let n = if below {
                match imm.checked_sub(1) {
                    Some(n) => n,
                    None => return Err("`jb 0` leaves no index".into()),
                }
            } else {
                imm
            };
            if n >= MSVC_MAX_ENTRIES {
                return Err(format!("the bound {n:#x} allows {MSVC_MAX_ENTRIES} or more entries"));
            }
            return Ok(SchedEntry { cmp: i.ip(), br: br.ip(), at: 0, n });
        }
        if i.rflags_modified() != 0 {
            return Err(format!(
                "the flags of the `{:?}` at {:#x} are last written by the `{:?}` at {:#x}, not by a `cmp {cur:?},imm`",
                br.mnemonic(),
                br.ip(),
                i.mnemonic(),
                i.ip()
            ));
        }
        if writes_gpr(info, i, cur) {
            match sext_source(i, cur) {
                Some(src) => cur = src,
                None => {
                    return Err(format!(
                        "{cur:?} is written at {:#x} (`{:?}`), between the `cmp` and the `{:?}` at {:#x}",
                        i.ip(),
                        i.mnemonic(),
                        br.mnemonic(),
                        br.ip()
                    ))
                }
            }
        }
    }
}

/// N of the scheduled form (decision 0109) from the CFG of the linear sweep of the function: **every way into the idiom** is the
/// edge of an unsigned conditional branch whose `cmp I,imm` bounds the index, and N is the largest of the bounds.
///
/// Walk back from the first instruction of the idiom (`s.load`) one instruction at a time while the instruction before falls into
/// the one at hand (a straight run, `FlowControl::Next`); at each instruction, every direct branch that targets it must be a
/// conditional branch (`sched_find_cmp` reads its bound); the walk ends at an instruction whose predecessor by fall-through is a
/// conditional branch (the fall-through edge is the other entry, read the same way), is not there, or does not fall through (`jmp`,
/// `ret`: nothing enters from before). Any other predecessor (a `call`, an unconditional `jmp`, the function entry) is a way in
/// that bounds nothing: `N not derived`. The instructions on the way between a guard and the idiom may be anything that does not
/// change the checked index: a store, a `lea` or `mov` of another register, a `nop`, the `lea B,[ImageBase]`; a write to the
/// checked register must be a sign extension (`sext_source`). `s.index` is the register that the first instruction of the idiom
/// indexes with. At most `MSVC_SCHED_GUARDS` distinct entries and `MSVC_SCHED_WALK` instructions.
///
/// Branches that the sweep does not see (an unresolved indirect jump of the function that lands inside) are not entries here; the
/// closure's own sources are checked after it (`landings_in_swapped_idioms`, with the second entry allowed).
fn derive_scheduled_guard(sw: &Sweep, entry_va: u64, s: &SwapForm) -> Result<SchedGuard, String> {
    let ins = &sw.insns;
    let mut p =
        sw.index_of(s.load).ok_or_else(|| format!("the first instruction of the idiom {:#x} does not decode in the sweep", s.load))?;
    let mut info = InstructionInfoFactory::new();
    // direct branches and direct calls by target: a call into the idiom is a way in that is not a guard
    let mut targets: BTreeMap<u64, Vec<usize>> = BTreeMap::new();
    for (k, i) in ins.iter().enumerate() {
        if matches!(i.flow_control(), FlowControl::UnconditionalBranch | FlowControl::ConditionalBranch | FlowControl::Call)
            && is_near_direct(i)
        {
            targets.entry(i.near_branch_target()).or_default().push(k);
        }
    }
    let mut cur = s.index;
    let mut entries: Vec<SchedEntry> = Vec::new();
    let mut lo = ins[p].ip();
    let mut done = false;
    for _ in 0..MSVC_SCHED_WALK {
        let at = ins[p].ip();
        if at == entry_va {
            return Err(sched_nd(format!("the walk reaches the function entry {entry_va:#x}, a way in that bounds nothing")));
        }
        lo = lo.min(at);
        for &kb in targets.get(&at).map(Vec::as_slice).unwrap_or(&[]) {
            let b = &ins[kb];
            if b.flow_control() != FlowControl::ConditionalBranch {
                return Err(sched_nd(format!(
                    "the `{:?}` at {:#x} enters the idiom at {at:#x} and is not a conditional branch",
                    b.mnemonic(),
                    b.ip()
                )));
            }
            let mut e = sched_find_cmp(ins, &targets, &mut info, kb, true, cur).map_err(sched_nd)?;
            e.at = at;
            entries.push(e);
        }
        if p == 0 || ins[p - 1].next_ip() != at || !falls_through(&ins[p - 1]) {
            done = true;
            break;
        }
        let pr = &ins[p - 1];
        match pr.flow_control() {
            FlowControl::ConditionalBranch => {
                let mut e = sched_find_cmp(ins, &targets, &mut info, p - 1, false, cur).map_err(sched_nd)?;
                e.at = at;
                entries.push(e);
                done = true;
                break;
            }
            FlowControl::Next => {
                if writes_gpr(&mut info, pr, cur) {
                    match sext_source(pr, cur) {
                        Some(src) => cur = src,
                        None => {
                            return Err(sched_nd(format!(
                                "{cur:?} is written at {:#x} (`{:?}`) on the way into the idiom and that is not a sign extension of the checked index",
                                pr.ip(),
                                pr.mnemonic()
                            )))
                        }
                    }
                }
                p -= 1;
            }
            _ => {
                return Err(sched_nd(format!(
                    "the `{:?}` at {:#x} falls into the idiom and is not a conditional branch",
                    pr.mnemonic(),
                    pr.ip()
                )))
            }
        }
    }
    if !done {
        return Err(sched_nd(format!("the walk back from the idiom does not end in {MSVC_SCHED_WALK} instructions")));
    }
    entries.sort_by_key(|e| (e.at, e.br));
    entries.dedup_by_key(|e| (e.br, e.at));
    if entries.is_empty() {
        return Err(sched_nd("no way into the idiom is a guard"));
    }
    if entries.len() > MSVC_SCHED_GUARDS {
        return Err(sched_nd(format!("{} ways into the idiom; at most {MSVC_SCHED_GUARDS} are read", entries.len())));
    }
    let n = entries.iter().map(|e| e.n).max().expect("not empty");
    Ok(SchedGuard { n, lo, first: entries[0], second: entries.get(1).copied() })
}

/// The reason text of `derive_scheduled_guard`; `lib.rs` recognises the prefix `N not derived` to keep the older reason of
/// `recognize_msvc_standard` for a site that stays rejected for lack of an N.
fn sched_nd(why: impl std::fmt::Display) -> String {
    format!("N not derived (scheduled form, decision 0109): {why}")
}

fn volatile(r: Register) -> bool {
    matches!(r, Register::RAX | Register::RCX | Register::RDX | Register::R8 | Register::R9 | Register::R10 | Register::R11)
}

/// The edges of the CFG of a sweep (decision 0090; decision 0094 uses the same ones): fall-through (a call falls through), direct
/// branches inside the sweep (a direct branch out of the pieces ends the path), and, at `jmp`, its own table targets. An
/// indirect jump other than `jmp` has no edge here (the reaching definitions add their virtual node on top of this).
fn cfg_succ(sw: &Sweep, k: usize, jmp: usize, targets: &[u64]) -> Vec<usize> {
    let i = &sw.insns[k];
    let next = sw.index_of(i.next_ip());
    let direct = if is_near_direct(i) { sw.index_of(i.near_branch_target()) } else { None };
    match i.flow_control() {
        FlowControl::Next | FlowControl::Call | FlowControl::IndirectCall | FlowControl::XbeginXabortXend => next.into_iter().collect(),
        FlowControl::UnconditionalBranch => direct.into_iter().collect(),
        FlowControl::ConditionalBranch => direct.into_iter().chain(next).collect(),
        FlowControl::IndirectBranch if k == jmp => targets.iter().filter_map(|&t| sw.index_of(t)).collect(),
        _ => Vec::new(),
    }
}

/// What defines B for `prove_base_reaching_defs`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum DefKind {
    /// `lea B,[rip+d]` with rip+d = ImageBase
    Lea,
    /// any other write to B (also a `lea` of another address)
    Write,
    /// a call, when B is caller-saved: the callee may change it
    Call,
}

#[derive(Clone, Copy)]
struct Def {
    kind: DefKind,
    /// the definition replaces the earlier ones (a write of the whole register); false for a conditional or partial write
    kills: bool,
}

/// The base proof of decision 0094: the definitions of B that reach the jump at `jmp_va` are all `lea B,[rip+d]`
/// with rip+d = ImageBase, and the function entry's own (undefined) B does not reach it.
///
/// Forward data flow over the CFG of `cfg_succ` (fall-through, direct branches inside the sweep, the jump's own table
/// `targets`). A definition is named by the index of its instruction; `ENTRY` stands for B at the function entry. The
/// definitions are the writes to B that `iced_x86` reports (explicit and implicit, whole sweep, chained pieces included; a
/// `push B` is not a write, a `pop B` is not one when the function has a `push B`) and, when B is caller-saved (rax, rcx,
/// rdx, r8-r11), every call (the callee may change B); callee-saved registers are assumed preserved by calls (Win64), as in
/// `dataflow.rs`. A call kills the definitions before it, so only a call on a path from a `lea` to the jump (or from the
/// entry) can reach the jump: a call before the `lea` on every path, or on a branch that never leads to the jump, is
/// harmless, and a call that does lie between the `lea` and the jump leaves `Call` as the reaching definition and rejects.
///
/// Every indirect jump of the function other than this one (`jmp reg`, `jmp [mem]`; a resolved switch included) is one
/// virtual node, node `n`: the jump's out-state flows into it, and its in-state flows into **every** instruction of the
/// function. The virtual node is a single set, the union of the definitions that reach those jumps, merged into the input of
/// all instructions: no per-edge work, and a definition that is not `lea` and reaches any other indirect jump therefore
/// reaches the jump too (it can land anywhere), so the proof fails. The landing anywhere over-estimates (the real targets are
/// in the jump's own table), which is the safe side.
///
/// `site_va` is the instruction at whose input the definitions are asked for: the jump (`site_va == jmp_va`, the form of
/// decisions 0089 and 0093) or, for the swapped form (decision 0095), the table load `mov eX,[B+I*4+T2]` in front of the
/// `add B,rX` that makes B the target. The data flow, its edges and the virtual node are the same; the jump is still the node
/// whose successors are its own table's targets and that is not a source of the virtual node. Everything that reaches the load
/// reaches it with B = ImageBase or the proof fails; the `add` and the instructions after it cannot change that (nothing can
/// enter the idiom after the load: `verify_msvc_swapped`, `landings_in_swapped_idioms`), and a definition that comes back
/// to the load through the jump's own table or an indirect jump (the `add` included) is a non-`lea` definition that reaches it
/// and rejects.
fn prove_base_reaching_defs(
    pe: &Pe<'_>,
    sw: &Sweep,
    entry_va: u64,
    b: Register,
    jmp_va: u64,
    targets: &[u64],
    site_va: u64,
) -> Result<(), String> {
    const ENTRY: usize = usize::MAX;
    let n = sw.insns.len();
    let entry = sw.index_of(entry_va).ok_or_else(|| format!("the function entry {entry_va:#x} does not decode in the sweep"))?;
    let jmp = sw.index_of(jmp_va).ok_or_else(|| format!("the jump {jmp_va:#x} does not decode in the sweep"))?;
    let site = sw.index_of(site_va).ok_or_else(|| format!("the instruction {site_va:#x} does not decode in the sweep"))?;
    let site_name = if site_va == jmp_va { "the jump" } else { "the table load of the swapped form" };
    let caller_saved = volatile(b);
    let is_push = |i: &Instruction| i.mnemonic() == Mnemonic::Push && i.op0_kind() == OpKind::Register && i.op0_register() == b;
    // a `pop B` is a restore (not a write) only when the function also has a `push B` that saved it
    let saved = sw.insns.iter().any(is_push);
    let mut info = InstructionInfoFactory::new();
    let mut defs: Vec<Option<Def>> = vec![None; n];
    for (k, i) in sw.insns.iter().enumerate() {
        if is_push(i) || (saved && i.mnemonic() == Mnemonic::Pop && i.op0_kind() == OpKind::Register && i.op0_register() == b) {
            continue;
        }
        if caller_saved && matches!(i.flow_control(), FlowControl::Call | FlowControl::IndirectCall) {
            defs[k] = Some(Def { kind: DefKind::Call, kills: true });
            continue;
        }
        let (mut wrote, mut kills) = (false, false);
        for u in info.info(i).used_registers() {
            if u.register().is_gpr() && u.register().full_register() == b {
                match u.access() {
                    // a 32-bit write zero-extends: the whole register is replaced; an 8- or 16-bit write keeps the rest
                    OpAccess::Write | OpAccess::ReadWrite => {
                        wrote = true;
                        kills |= u.register().size() >= 4;
                    }
                    OpAccess::CondWrite | OpAccess::ReadCondWrite => wrote = true,
                    _ => {}
                }
            }
        }
        if wrote {
            let is_lea = i.mnemonic() == Mnemonic::Lea
                && i.op0_kind() == OpKind::Register
                && i.op0_register() == b
                && i.op_kind(1) == OpKind::Memory
                && i.is_ip_rel_memory_operand()
                && i.ip_rel_memory_address() == pe.optional.image_base;
            defs[k] = Some(Def { kind: if is_lea { DefKind::Lea } else { DefKind::Write }, kills });
        }
    }

    type State = Option<BTreeSet<usize>>;
    fn merge(inn: &mut [State], work: &mut Vec<usize>, s: usize, set: &BTreeSet<usize>) {
        if let Some(old) = inn[s].as_mut() {
            if !set.is_subset(old) {
                old.extend(set.iter().copied());
                work.push(s);
            }
        } else {
            inn[s] = Some(set.clone());
            work.push(s);
        }
    }
    // inn[k]: the definitions that reach the start of instruction k (None: not reached); `virt`: the one set of the virtual
    // node (the union of what reaches the other indirect jumps). It is merged into every instruction only when the work list
    // is empty (`virt_dirty`), so a growing set costs one pass over the instructions per growth of the whole flow, not one
    // per change.
    let mut inn: Vec<State> = vec![None; n];
    let mut work: Vec<usize> = Vec::new();
    let mut virt: BTreeSet<usize> = BTreeSet::new();
    let mut virt_dirty = false;
    merge(&mut inn, &mut work, entry, &BTreeSet::from([ENTRY]));
    loop {
        let Some(k) = work.pop() else {
            if !virt_dirty {
                break;
            }
            virt_dirty = false;
            for j in 0..n {
                merge(&mut inn, &mut work, j, &virt);
            }
            continue;
        };
        let cur = inn[k].clone().expect("a queued node has a state");
        let out = match defs[k] {
            None => cur,
            Some(Def { kills: true, .. }) => BTreeSet::from([k]),
            Some(Def { kills: false, .. }) => {
                let mut s = cur;
                s.insert(k);
                s
            }
        };
        for s in cfg_succ(sw, k, jmp, targets) {
            merge(&mut inn, &mut work, s, &out);
        }
        if k != jmp && sw.insns[k].flow_control() == FlowControl::IndirectBranch && !out.is_subset(&virt) {
            virt.extend(out.iter().copied());
            virt_dirty = true;
        }
    }

    let reaching = match inn[site].as_ref() {
        Some(d) if !d.is_empty() => d,
        _ => return Err(format!("{site_name} is not reached from the function entry {entry_va:#x} on the CFG, so no definition of {b:?} is known to reach it (decision 0094)")),
    };
    let first = |kind: DefKind| reaching.iter().copied().filter(|&d| d != ENTRY).find(|&d| defs[d].is_some_and(|x| x.kind == kind));
    if let Some(d) = first(DefKind::Write) {
        let i = &sw.insns[d];
        let what = if i.mnemonic() == Mnemonic::Lea { "a `lea` of an address other than the ImageBase".to_string() } else { format!("`{:?}`", i.mnemonic()) };
        return Err(format!(
            "the definition of {b:?} at {:#x} ({what}) reaches {site_name} (directly, or through another indirect jump of the function, which lands anywhere) and is not `lea {b:?},[rip+d]` with rip+d = ImageBase (decision 0094)",
            i.ip()
        ));
    }
    if let Some(d) = first(DefKind::Call) {
        return Err(format!("the call at {:#x} lies on a path from a definition of {b:?} to {site_name} and {b:?} is caller-saved (decision 0094)", sw.insns[d].ip()));
    }
    if reaching.iter().all(|&d| d == ENTRY) {
        return Err(format!("no definition of {b:?} reaches {site_name}: {b:?} is never written on any path from the function entry {entry_va:#x} to it (decision 0094)"));
    }
    if reaching.contains(&ENTRY) {
        return Err(format!(
            "{b:?} is not defined on every path from the function entry {entry_va:#x} to {site_name}: a path arrives with the value of the caller (decision 0094; every reaching definition must be `lea {b:?},[rip+d]` = ImageBase)"
        ));
    }
    Ok(())
}

/// The base proof of decisions 0089, 1(a) and 0094: the definitions of B that reach the jump are all `lea B,[rip+d]` with rip+d = ImageBase
/// (`prove_base_reaching_defs`). `verify_msvc` adds the checks that rest on the closure. `targets` is the own table of the jump (an edge set for the CFG).
pub fn prove_base(pe: &Pe<'_>, sw: &Sweep, entry_va: u64, b: Register, jmp_va: u64, targets: &[u64]) -> Result<(), String> {
    if !b.is_gpr64() || b == Register::RSP {
        return Err("the base register is not a general 64-bit register".into());
    }
    prove_base_reaching_defs(pe, sw, entry_va, b, jmp_va, targets, jmp_va)
}

/// Reads the tables of `f` and returns the distinct target VAs (ImageBase + RVA) in ascending order, or why they are not
/// all inside the function `pieces`, in an executable section and at a decodable address (decision 0089, 1(c), 1(d)).
fn read_msvc_targets(pe: &Pe<'_>, text: &Text<'_>, fm: &FuncMap, pieces: &[(u32, u32)], f: &MsvcForm) -> Result<Vec<u64>, String> {
    let count = match f.t1 {
        Some(t1) => {
            let bytes = pe.slice_at_rva(t1, f.n as usize + 1).map_err(|e| format!("index byte table: {e}"))?;
            *bytes.iter().max().expect("N + 1 bytes") as usize + 1
        }
        None => f.n as usize + 1,
    };
    let raw = pe.slice_at_rva(f.t2, 4 * count).map_err(|e| format!("RVA table ({count} entries): {e}"))?;
    let rvas: std::collections::BTreeSet<u32> = raw.chunks_exact(4).map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect();
    let mut outside: Vec<String> = Vec::new();
    for &r in &rvas {
        let va = pe.rva_to_va(r);
        if pieces.iter().any(|&(b, e)| r >= b && r < e) && text.contains(va) {
            if !matches!(text.decode(va), Decoded::Ok(_)) {
                return Err(format!("target {va:#x} does not decode"));
            }
            continue;
        }
        let what = match fm.function_at(r) {
            Some((p, _)) => format!("in the function at {:#x}", pe.rva_to_va(p)),
            None => "in no .pdata function".to_string(),
        };
        outside.push(format!("{va:#x} ({what}{})", if text.contains(va) { "" } else { ", not executable" }));
    }
    if !outside.is_empty() {
        let shown: Vec<&str> = outside.iter().take(8).map(String::as_str).collect();
        return Err(format!(
            "target outside the function: {} of {} distinct target(s): {}{}",
            outside.len(),
            rvas.len(),
            shown.join(", "),
            if outside.len() > 8 { ", ..." } else { "" }
        ));
    }
    Ok(rvas.into_iter().map(|r| pe.rva_to_va(r)).collect())
}

/// The tables of a scheduled switch (decision 0109) must lie in sections without IMAGE_SCN_MEM_WRITE (or in the headers, which the
/// loader maps read-only). The scheduled form lets stores stand between the guard and the table load; the tables are read from
/// the file image (as in every MSVC switch since decision 0089), so a table that the program can write could hold other values at
/// run time. `resolve_msvc` calls it after the tables were read, so each range is known to lie in one file-backed section.
fn scheduled_tables_are_read_only(pe: &Pe<'_>, f: &MsvcForm) -> Result<(), String> {
    let mut tables = Vec::new();
    if let Some(t1) = f.t1 {
        tables.push(("index byte table", t1));
    }
    tables.push(("RVA table", f.t2));
    for (what, rva) in tables {
        if (rva as u64) < pe.optional.size_of_headers as u64 {
            continue;
        }
        if let Some(s) = pe.sections.iter().find(|s| s.contains_rva(rva)) {
            if s.is_writable() {
                return Err(format!(
                    "the {what} at {:#x} lies in the writable section `{}`: the scheduled form (decision 0109) allows stores in the gap, so the table could change at run time",
                    pe.rva_to_va(rva),
                    s.name
                ));
            }
        }
    }
    Ok(())
}

/// Resolves a recognised MSVC switch (decision 0089): the function of the jump, then the base proof (1(a)), then the tables
/// and their targets (1(c), 1(d)). The first failing condition in that order is the error. On success the table is a
/// `Select::Target` switch over the target VAs, and the sweep of the function is left in `sweeps` for `verify_msvc`.
/// The lifted switch compares the value the jump register really holds, so a wrong base or entry count can only end in
/// `fle_jumptab_miss`, never in a jump to another place.
pub fn resolve_msvc(
    pe: &Pe<'_>,
    text: &Text<'_>,
    fm: &FuncMap,
    sweeps: &mut BTreeMap<u32, Sweep>,
    f: &MsvcForm,
    jmp_va: u64,
) -> Result<JumpTable, String> {
    let jmp_rva = pe.va_to_rva(jmp_va).map_err(|e| e.to_string())?;
    let (primary, pieces) = fm.function_at(jmp_rva).ok_or("the jump is not inside a .pdata function")?;
    let entry_va = pe.rva_to_va(primary);
    if !pieces.iter().any(|&(b, e)| primary >= b && primary < e) {
        return Err("the function entry is not inside one of its .pdata pieces".into());
    }
    if !sweeps.contains_key(&primary) {
        sweeps.insert(primary, Sweep::build(pe, text, pieces)?);
    }
    let sw = &sweeps[&primary];
    // the swapped form (decision 0095): N comes from the guard's branch on the edge into the table block, read from the CFG of
    // the sweep; the base is proven at the table load (the `add B,rX` after it is part of the idiom). The scheduled form
    // (decision 0109) reads N from every way into the idiom (`derive_scheduled_guard`) and is proven the same way
    let (form, swapped, site, entry2) = match f.swapped {
        None => (*f, None, jmp_va, None),
        Some(s) => {
            if s.scheduled {
                let g = derive_scheduled_guard(sw, entry_va, &s)?;
                let f1 = g.first;
                (
                    MsvcForm { n: g.n, from: f1.at, guard_ja: f1.br, guard_cmp: f1.cmp, ..*f },
                    Some(g.lo),
                    s.load,
                    g.second.map(|e| (e.br, e.at)),
                )
            } else {
                let g = derive_swapped_guard(sw, entry_va, &s)?;
                (MsvcForm { n: g.n, from: g.from, guard_ja: g.br, guard_cmp: g.cmp, ..*f }, Some(g.lo), s.load, None)
            }
        }
    };
    let targets = read_msvc_targets(pe, text, fm, pieces, &form);
    if swapped.is_some() {
        prove_base_reaching_defs(pe, sw, entry_va, form.base, jmp_va, targets.as_deref().unwrap_or(&[]), site)?;
    } else {
        prove_base(pe, sw, entry_va, form.base, jmp_va, targets.as_deref().unwrap_or(&[]))?;
    }
    let targets = targets?;
    // decision 0109: the gap of a scheduled idiom may hold stores, so the tables it reads must be where the program cannot change them
    if f.swapped.is_some_and(|s| s.scheduled) {
        scheduled_tables_are_read_only(pe, &form)?;
    }
    if let Some(t) = targets.iter().find(|&&t| !sw.is_start(t)) {
        return Err(format!("target {t:#x} is not an instruction start on the linear sweep of the function"));
    }
    Ok(JumpTable {
        select: Select::Target,
        cases: targets.into_iter().map(|t| (t, t)).collect(),
        origin: Origin::Msvc {
            n: form.n,
            t1: form.t1,
            t2: form.t2,
            from: form.from,
            guard_ja: form.guard_ja,
            guard_cmp: form.guard_cmp,
            function: primary,
            swapped,
            entry2,
        },
    })
}

/// After the closure, for a table from `resolve_msvc`:
/// - the instructions from `from` (the one after the guard's `ja`) to the jump were decoded in sequence and no block
///   starts inside them after the first (a branch landing there would arrive without the guard having run);
/// - every instruction the closure decoded inside the function's pieces starts where the linear sweep, on which the base
///   proof rests, has an instruction;
/// - the closure attributed the jump to the entry of its own function (the entry block, with the `lea`, ran first), and no
///   other closure function starts inside the function's pieces (such an entry could reach the jump without it).
pub fn verify_msvc(
    jmp_va: u64,
    from: u64,
    guard: (u64, u64), // (ja, cmp)
    entry_va: u64,
    insns: &BTreeMap<u64, Instruction>,
    leaders: &HashSet<u64>,
    functions: &BTreeMap<u64, Vec<u64>>,
    sw: &Sweep,
) -> Result<(), String> {
    if !functions.get(&entry_va).is_some_and(|v| v.contains(&jmp_va)) {
        return Err(format!("the jump was first reached from another closure function, not from the function entry {entry_va:#x}"));
    }
    match (insns.get(&guard.0), insns.get(&guard.1)) {
        (Some(ja), Some(cmp)) if ja.mnemonic() == Mnemonic::Ja && ja.next_ip() == from && cmp.mnemonic() == Mnemonic::Cmp && cmp.next_ip() == guard.0 => {}
        _ => return Err("the closure did not decode the guard `cmp; ja` in front of the switch idiom as recognised".into()),
    }
    let mut va = from;
    loop {
        let i = insns.get(&va).ok_or_else(|| format!("the instruction at {va:#x} in front of the jump was not decoded in sequence"))?;
        if va != from && leaders.contains(&va) {
            return Err(format!("a block starts at {va:#x}, inside the switch idiom (a branch lands between its instructions)"));
        }
        if va == jmp_va {
            break;
        }
        va += i.len() as u64;
        if va > jmp_va {
            return Err("the idiom instructions do not end at the jump".into());
        }
    }
    for &(b, e) in &sw.pieces {
        if let Some((&bad, _)) = insns.range(b..e).find(|(&k, _)| !sw.is_start(k)) {
            return Err(format!("the closure decoded an instruction at {bad:#x} that is not on the linear sweep of its function"));
        }
        if let Some((&other, _)) = functions.range(b..e).find(|(&k, _)| k != entry_va) {
            return Err(format!("another closure function starts at {other:#x} inside the function {entry_va:#x}"));
        }
    }
    Ok(())
}

/// `verify_msvc` for the swapped form (decision 0095), after the closure. The guard is not next to the table block: `br` (the
/// conditional branch) and `cmp` are the instructions `derive_swapped_guard` found, `from` the first instruction of the table
/// block and `lo` the first address of the run that only the guard's branch may enter (`lo` < `from` when padding `nop`s
/// stand in front of the block). Checked:
/// - the closure decoded `cmp` and `br` as such, and `br` leads into the run: its target is `from`, or the instruction after
///   it is `lo` (the fall-through side);
/// - the instructions from there (`from`, or `lo` for the fall-through side) to the jump were decoded in sequence and no block
///   starts at any of them after the first (a branch landing between them would arrive without the guard having run);
/// - the same two checks on the function as `verify_msvc`: every instruction the closure decoded inside the pieces starts where
///   the linear sweep has one, and no other closure function starts inside the pieces.
pub fn verify_msvc_swapped(
    jmp_va: u64,
    from: u64,
    lo: u64,
    guard: (u64, u64), // (conditional branch, cmp)
    entry_va: u64,
    insns: &BTreeMap<u64, Instruction>,
    leaders: &HashSet<u64>,
    functions: &BTreeMap<u64, Vec<u64>>,
    sw: &Sweep,
) -> Result<(), String> {
    verify_msvc_swapped_entries(jmp_va, from, lo, guard, None, entry_va, insns, leaders, functions, sw)
}

/// `verify_msvc_swapped` with a second way into the run (decision 0109): `extra` is the address that the second guard's branch
/// lands on (`Origin::Msvc::entry2`), the head of the loop in a loop-rotated switch. A block starts there by design, so it is
/// not an error; every other block start inside the run still is.
pub fn verify_msvc_swapped_entries(
    jmp_va: u64,
    from: u64,
    lo: u64,
    guard: (u64, u64), // (conditional branch, cmp)
    extra: Option<u64>,
    entry_va: u64,
    insns: &BTreeMap<u64, Instruction>,
    leaders: &HashSet<u64>,
    functions: &BTreeMap<u64, Vec<u64>>,
    sw: &Sweep,
) -> Result<(), String> {
    if !functions.get(&entry_va).is_some_and(|v| v.contains(&jmp_va)) {
        return Err(format!("the jump was first reached from another closure function, not from the function entry {entry_va:#x}"));
    }
    let start = match (insns.get(&guard.0), insns.get(&guard.1)) {
        (Some(br), Some(cmp))
            if br.flow_control() == FlowControl::ConditionalBranch
                && is_near_direct(br)
                && (br.near_branch_target() == from || br.next_ip() == lo)
                && cmp.mnemonic() == Mnemonic::Cmp
                && cmp.ip() < br.ip() =>
        {
            if br.near_branch_target() == from {
                from
            } else {
                lo
            }
        }
        _ => return Err("the closure did not decode the guard `cmp` and conditional branch in front of the swapped switch idiom as recognised".into()),
    };
    let mut va = start;
    loop {
        let i = insns.get(&va).ok_or_else(|| format!("the instruction at {va:#x} in front of the jump was not decoded in sequence"))?;
        if va != start && leaders.contains(&va) && Some(va) != extra {
            return Err(format!("a block starts at {va:#x}, inside the swapped switch idiom (a branch lands between its instructions)"));
        }
        if va == jmp_va {
            break;
        }
        va += i.len() as u64;
        if va > jmp_va {
            return Err("the idiom instructions do not end at the jump".into());
        }
    }
    for &(b, e) in &sw.pieces {
        if let Some((&bad, _)) = insns.range(b..e).find(|(&k, _)| !sw.is_start(k)) {
            return Err(format!("the closure decoded an instruction at {bad:#x} that is not on the linear sweep of its function"));
        }
        if let Some((&other, _)) = functions.range(b..e).find(|(&k, _)| k != entry_va) {
            return Err(format!("another closure function starts at {other:#x} inside the function {entry_va:#x}"));
        }
    }
    Ok(())
}

/// The one place a function with an MSVC table can be entered from outside its entry: a branch, a resolved indirect jump
/// or a table case of an instruction that lies outside the function's pieces and targets an address inside them other
/// than the entry. `protected` is `(begin, end, function)` per piece (ascending by `begin`); `sources` is every
/// `(source VA, target VA)` pair the closure knows. Returns, per function, the first offending pair. Such a function
/// can reach its switch without running the `lea` of its entry block, so its tables are dropped.
pub fn foreign_entries(
    protected: &[(u64, u64, u32, u64)],
    sources: impl Iterator<Item = (u64, u64)>,
) -> BTreeMap<u32, (u64, u64)> {
    let find = |va: u64| -> Option<&(u64, u64, u32, u64)> {
        let k = protected.partition_point(|p| p.0 <= va).checked_sub(1)?;
        protected.get(k).filter(|p| va < p.1)
    };
    let mut out = BTreeMap::new();
    for (src, tgt) in sources {
        let Some(&(_, _, f, entry)) = find(tgt) else { continue };
        if tgt == entry || out.contains_key(&f) {
            continue;
        }
        if find(src).map(|p| p.2) != Some(f) {
            out.insert(f, (src, tgt));
        }
    }
    out
}

/// `landings_in_idioms` for the swapped form (decision 0095). `ranges` is `(lo, jmp, branch, from)` per table (`Origin::Msvc`
/// with `swapped: Some(lo)`, `guard_ja` as the branch and `from`, the first instruction of the table block): the only way
/// into `[lo, jmp]` is the guard's own conditional `branch` to `from` (when `from` is its target side; the fall-through side
/// is not a source). A source `(src, tgt)` with `lo <= tgt <= jmp` that is not `(branch, from)` is an entry that skips the
/// guard (padding `nop`s in front of the table block are inside the range: a branch to one of them runs into the block
/// without the guard). Returns, per jump, the first offending pair.
pub fn landings_in_swapped_idioms(ranges: &[(u64, u64, u64, u64)], sources: impl Iterator<Item = (u64, u64)>) -> BTreeMap<u64, (u64, u64)> {
    let mut sorted: Vec<(u64, u64, u64, u64)> = ranges.to_vec();
    sorted.sort_unstable();
    let longest = sorted.iter().map(|r| r.1 - r.0).max().unwrap_or(0);
    // a jump can have two ways in (decision 0109: the scheduled form of a loop-rotated switch): two tuples with the same `lo` and
    // jump, one per guard. A source is allowed when any tuple of that jump names it; with one tuple per jump this is the old rule.
    let allowed: HashSet<(u64, u64, u64)> = ranges.iter().map(|r| (r.1, r.2, r.3)).collect();
    let mut out = BTreeMap::new();
    for (src, tgt) in sources {
        let k = sorted.partition_point(|r| r.0 <= tgt);
        for r in sorted[..k].iter().rev().take_while(|r| tgt - r.0 <= longest) {
            if tgt <= r.1 && !allowed.contains(&(r.1, src, tgt)) && !out.contains_key(&r.1) {
                out.insert(r.1, (src, tgt));
            }
        }
    }
    out
}

/// Branches that land inside a switch idiom: for each table `(jump VA, first VA that must not be entered, jump VA)`, a
/// source `(src, tgt)` with `tgt` in `(guard cmp, jump]` is an entry that skips the guard (the fall-through of the guard's
/// `ja` is not a source, it is the only way in). `ranges` is `(after, jmp)` per table: an instruction start `a` is inside
/// when `after < a <= jmp`. Returns, per jump, the first offending pair.
pub fn landings_in_idioms(ranges: &[(u64, u64)], sources: impl Iterator<Item = (u64, u64)>) -> BTreeMap<u64, (u64, u64)> {
    let mut sorted: Vec<(u64, u64)> = ranges.to_vec();
    sorted.sort_unstable();
    let mut out = BTreeMap::new();
    for (src, tgt) in sources {
        // an idiom is a few dozen bytes long (MSVC_IDIOM_MAX), so only the ranges that start just before `tgt` can hold it
        let k = sorted.partition_point(|r| r.0 < tgt);
        for r in sorted[..k].iter().rev().take_while(|r| tgt - r.0 <= MSVC_IDIOM_MAX) {
            if tgt <= r.1 && !out.contains_key(&r.1) {
                out.insert(r.1, (src, tgt));
            }
        }
    }
    out
}
