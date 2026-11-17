//! x86-64 decoding (iced-x86) and recursive-descent reachability over the
//! executable sections of a PE image (every section with IMAGE_SCN_MEM_EXECUTE, whatever its name;
//! decision 0074). Read-only: produces counts and per-function instruction lists; generates no code.
//! In the comments below `.text` means "the executable sections".
//!
//! Definitions (fixed here so the numbers are reproducible):
//! - seed: the PE entry point and every export. Exports whose VA lies outside
//!   `.text` are data, not code, and are recorded as skipped seeds.
//! - function: a seed, or the target of a direct `call` that lies in `.text`.
//! - block leader: a function entry, a direct branch target, or the
//!   fall-through address after a conditional branch.
//! - instruction: a unique VA reached by the traversal. An instruction is
//!   attributed to the first function whose traversal reaches it.
//! - Indirect calls/jumps are followed only when their target is resolved by
//!   `indirect::Resolver` (rip-relative table, register set by the preceding
//!   `lea`/`mov`). Import slots are named and not followed. Unresolved ones are
//!   counted and not followed. The raw counts below (`import_*`, `*_other`)
//!   count every indirect transfer, resolved or not.

pub mod adjustor;
pub mod indirect;
pub mod jumptab;
pub mod relocseeds;
pub mod seh;
pub mod setjmp;

pub use indirect::{How, Resolved};
pub use jumptab::{JumpTable, Options, Origin, Select, Spec};

use fe_pe::Pe;
use iced_x86::{Decoder, DecoderOptions, FlowControl, Instruction, OpKind};
use std::collections::{BTreeMap, BTreeSet, HashSet, VecDeque};

/// One contiguous run of code bytes at `va`.
#[derive(Debug, Clone, Copy)]
pub struct CodeRange<'a> {
    pub va: u64,
    pub bytes: &'a [u8],
}

impl<'a> CodeRange<'a> {
    pub fn contains(&self, va: u64) -> bool {
        va >= self.va && va - self.va < self.bytes.len() as u64
    }
}

/// The code of an image: one or more ranges in ascending VA order. `from_pe` takes every section that has
/// IMAGE_SCN_MEM_EXECUTE, whatever its name (decision 0074). A VA outside every range is out of code.
pub struct Text<'a> {
    ranges: Vec<CodeRange<'a>>,
}

pub enum Decoded {
    Ok(Instruction),
    Invalid,
    OutOfText,
}

impl<'a> Text<'a> {
    /// The executable sections of `pe` (`fe_pe::Section::is_executable`), ascending by VA. A section with no
    /// file bytes (`SizeOfRawData` 0) has nothing to decode and is left out; a `VirtualSize` above
    /// `SizeOfRawData` is cut at the file-backed bytes. No executable section is an error.
    pub fn from_pe(pe: &Pe<'a>) -> Result<Text<'a>, String> {
        let mut ranges = Vec::new();
        for s in pe.sections.iter().filter(|s| s.is_executable()) {
            if s.size_of_raw_data == 0 {
                continue;
            }
            let len = (s.virtual_size as usize).min(s.size_of_raw_data as usize);
            let bytes = pe
                .slice_at_rva(s.virtual_address, len)
                .map_err(|e| format!("{}: {e}", s.name))?;
            ranges.push(CodeRange { va: pe.rva_to_va(s.virtual_address), bytes });
        }
        if ranges.is_empty() {
            return Err("no executable section".into());
        }
        ranges.sort_by_key(|r| r.va);
        Ok(Text { ranges })
    }

    /// A single range (hand-made bytes in tests).
    pub fn single(va: u64, bytes: &'a [u8]) -> Text<'a> {
        Text { ranges: vec![CodeRange { va, bytes }] }
    }

    pub fn ranges(&self) -> &[CodeRange<'a>] {
        &self.ranges
    }

    /// The range that holds `va`.
    pub fn range_at(&self, va: u64) -> Option<&CodeRange<'a>> {
        self.ranges.iter().find(|r| r.contains(va))
    }

    /// `len` bytes at `va`, all inside one range.
    pub fn slice(&self, va: u64, len: usize) -> Option<&'a [u8]> {
        let r = self.range_at(va)?;
        let off = (va - r.va) as usize;
        r.bytes.get(off..off.checked_add(len)?)
    }

    pub fn contains(&self, va: u64) -> bool {
        self.range_at(va).is_some()
    }

    pub fn decode(&self, va: u64) -> Decoded {
        let Some(r) = self.range_at(va) else { return Decoded::OutOfText };
        let off = (va - r.va) as usize;
        let mut d = Decoder::with_ip(64, &r.bytes[off..], va, DecoderOptions::NONE);
        let i = d.decode();
        if i.is_invalid() {
            Decoded::Invalid
        } else {
            Decoded::Ok(i)
        }
    }
}

#[derive(Debug, Clone)]
pub struct SkippedSeed {
    pub name: String,
    pub va: u64,
    pub reason: &'static str,
}

#[derive(Debug, Default)]
pub struct Closure {
    pub seeds_code: Vec<(String, u64)>,
    pub seeds_skipped: Vec<SkippedSeed>,
    pub functions: usize,
    pub blocks: usize,
    pub instructions: usize,
    pub decode_failures: usize,
    pub out_of_text_flow: usize,
    pub out_of_text_calls: usize,
    pub direct_calls: usize,
    pub direct_jumps: usize,
    pub conditional_branches: usize,
    pub returns: usize,
    pub import_calls: usize,
    pub import_jumps: usize,
    pub indirect_calls_other: usize,
    pub indirect_jumps_other: usize,
    pub far_transfers: usize,
    pub interrupts: usize,
    pub mnemonics: BTreeMap<String, usize>,
    pub fail_samples: Vec<u64>,
    /// Resolution results of indirect call/jmp, by kind (`call:rip_table`, ...).
    pub indirect: indirect::Stats,
    /// Annotated jump tables (`jumptab`) that were read and passed the checks.
    pub jumptabs: usize,
    /// Of `jumptabs`, the selector stubs of decision 0079 (no annotation; the table is read from the relocations).
    pub stub_tables: usize,
    /// Of `jumptabs`, the inline absolute-pointer tables of decision 0102 W3 (`lea B,[rip+k]; jmp [B+I*8+d]`; the table is read from
    /// the relocations).
    pub inline_tables: usize,
    /// Of `jumptabs`, the MSVC switch tables with a proven base (decision 0089).
    pub msvc_tables: usize,
    /// `jmp` with a register operand, whatever the outcome (resolved, table, unresolved).
    pub register_jumps: usize,
    /// Annotated jumps whose table was dropped: (jump VA, reason). Those jumps stay unresolved.
    pub jumptab_rejected: Vec<(u64, String)>,
    /// `jmp reg` sites with the MSVC switch idiom (`mov eT,[B+I*4+T]; add rT,B; jmp rT`) that were not resolved
    /// (decision 0089), with the first condition that failed. Those jumps stay unresolved.
    pub msvc_rejected: Vec<(u64, String)>,
    /// Decision 0168 (`prune` of `analyze_with_prune`): exports and extra seeds that were listed and dropped; conditional branches whose target is a listed
    /// function entry (the target is then a block of the branching function, not a stub: such a function is not pruned there).
    pub pruned_seeds_dropped: usize,
    pub prune_cond_targets: usize,
}

pub struct Analysis {
    pub closure: Closure,
    /// Decoded instructions, keyed by VA.
    pub insns: BTreeMap<u64, Instruction>,
    /// Function entry VA -> instruction VAs attributed to it, ascending.
    pub functions: BTreeMap<u64, Vec<u64>>,
    /// Indirect call/jmp VA -> resolution. Every reached indirect transfer has an entry.
    pub resolved: BTreeMap<u64, Resolved>,
    /// Indirect jmp VA -> its annotated jump table. The jump is `Unresolved` in `resolved`.
    pub jumptabs: BTreeMap<u64, JumpTable>,
}

fn is_direct_near(i: &Instruction) -> bool {
    matches!(i.op_kind(0), OpKind::NearBranch16 | OpKind::NearBranch32 | OpKind::NearBranch64)
}

pub fn analyze(pe: &Pe<'_>) -> Result<Analysis, String> {
    analyze_with_seeds(pe, &[])
}

/// The function starts of the exception directory (`.pdata`, chained entries excluded, see
/// `fe_pe::Pe::function_starts`) that lie in `.text`, as VAs in ascending order.
pub fn pdata_seeds(pe: &Pe<'_>) -> Result<Vec<u64>, String> {
    let text = Text::from_pe(pe)?;
    let all = pe.function_start_vas().map_err(|e| format!("pdata: {e}"))?;
    Ok(all.into_iter().filter(|&v| text.contains(v)).collect())
}

/// The `.pdata` starts plus the data-referenced adjustor thunks outside `.pdata` (decision 0051).
pub fn pdata_and_adjustor_seeds(pe: &Pe<'_>) -> Result<Vec<u64>, String> {
    let mut v = pdata_seeds(pe)?;
    v.extend(adjustor::adjustor_seeds(pe)?);
    Ok(v)
}

/// `analyze_with_seeds` with `pdata_and_adjustor_seeds` as the extra seeds.
pub fn analyze_with_pdata_adjustors(pe: &Pe<'_>) -> Result<Analysis, String> {
    analyze_with_seeds(pe, &pdata_and_adjustor_seeds(pe)?)
}

/// `analyze_with_seeds` with `relocseeds::reloc_pdata_seeds` as the extra seeds.
pub fn analyze_with_reloc_seeds(pe: &Pe<'_>) -> Result<Analysis, String> {
    analyze_with_seeds(pe, &relocseeds::reloc_pdata_seeds(pe)?)
}

/// `analyze_with_seeds` with `pdata_seeds` as the extra seeds.
pub fn analyze_with_pdata(pe: &Pe<'_>) -> Result<Analysis, String> {
    analyze_with_seeds(pe, &pdata_seeds(pe)?)
}

/// Same as `analyze`, plus `extra` as additional code seeds. They are queued after
/// the entry and the exports, and are named `<seed:VA>`. With `extra` empty this is `analyze`.
pub fn analyze_with_seeds(pe: &Pe<'_>, extra: &[u64]) -> Result<Analysis, String> {
    analyze_with_specs(pe, extra, jumptab::FLENGINE)
}

/// Same as `analyze_with_seeds`, with `specs` as the annotated jump tables (`analyze_with_seeds` passes
/// `jumptab::FLENGINE`; an image without those jumps is not affected, because each spec is checked
/// against the bytes at its address). A jump that the resolver leaves unresolved and that `specs` names
/// has its table read and checked; the targets then enter the closure as blocks (decision 0036).
pub fn analyze_with_specs(pe: &Pe<'_>, extra: &[u64], specs: &[Spec]) -> Result<Analysis, String> {
    analyze_with_options(pe, extra, specs, &Options::default())
}

/// Same as `analyze_with_specs`, with `opts`. The default `Options` is what decisions 0036, 0079, 0089, 0090, 0094 and 0095 specify.
pub fn analyze_with_options(pe: &Pe<'_>, extra: &[u64], specs: &[Spec], opts: &Options) -> Result<Analysis, String> {
    analyze_with_swap(pe, extra, specs, opts, &BTreeSet::new())
}

/// Same as `analyze_with_options`, with `swap` as the entries of functions that the host implements (decision 0160): each is a function entry
/// of the closure (it is a seed) whose body is not decoded and whose callees are not followed; a direct jump to one is a tail call to it.
/// With `swap` empty this is `analyze_with_options`.
pub fn analyze_with_swap(pe: &Pe<'_>, extra: &[u64], specs: &[Spec], opts: &Options, swap: &BTreeSet<u64>) -> Result<Analysis, String> {
    analyze_with_prune(pe, extra, specs, opts, swap, &BTreeSet::new(), &BTreeSet::new())
}

/// Same as `analyze_with_swap`, with `prune` as the entries of functions that must not run (decision 0168): a function in `prune` is a function entry
/// of the closure only when kept code reaches it (a direct call or jump, a resolved indirect call or jump, a case of a selector stub or of a jump table);
/// it is not a seed (the export, the `extra` seed and the entry point are dropped when listed; the entry point is an error), its body is not decoded and
/// nothing behind it is followed, so a function that only a pruned function reaches is not in the closure.
///
/// `keep_unresolved` (decision 0168) are the VAs of table jumps that stay unresolved whatever the checks after the closure say (they become
/// `fle_dispatch` jumps). The checks look at the decoded code around a table (an instruction that overlaps a target, a block that starts inside a stub), so
/// with a smaller closure a jump that the unpruned closure rejects can pass; this set is the rejected jumps of the unpruned lift, and it keeps the
/// dispatch (and the guest's pseudo-preemption count, decision 0147) as it was. With both sets empty this is `analyze_with_swap`.
pub fn analyze_with_prune(
    pe: &Pe<'_>,
    extra: &[u64],
    specs: &[Spec],
    _opts: &Options,   // no member any more (decision 0243: the base proof is one); kept so that the callers do not change
    swap: &BTreeSet<u64>,
    prune: &BTreeSet<u64>,
    keep_unresolved: &BTreeSet<u64>,
) -> Result<Analysis, String> {
    let spec_at: BTreeMap<u64, &Spec> = specs.iter().map(|s| (s.jmp_va, s)).collect();
    let mut jumptabs: BTreeMap<u64, JumpTable> = BTreeMap::new();
    let text = Text::from_pe(pe)?;
    let iat: HashSet<u64> = pe
        .imports()
        .map_err(|e| format!("imports: {e}"))?
        .iter()
        .flat_map(|i| i.entries.iter())
        .map(|e| e.iat_va)
        .collect();
    let resolver = indirect::Resolver::new(pe, &text)?;
    let mut resolved: BTreeMap<u64, Resolved> = BTreeMap::new();
    // DIR64 slot VAs, read on the first selector stub (decision 0079)
    let mut relocs: Option<HashSet<u64>> = None;
    // `.pdata` functions and their linear sweeps, for the MSVC switch tables (decision 0089); built on first use
    let mut funcmap: Option<Result<jumptab::FuncMap, String>> = None;
    let mut sweeps: BTreeMap<u32, jumptab::Sweep> = BTreeMap::new();

    let mut cl = Closure::default();
    // (function VA, Some(name) for a seed, None for a direct call target)
    let mut fn_queue: VecDeque<(u64, Option<String>)> = VecDeque::new();

    let entry_va = pe.rva_to_va(pe.optional.address_of_entry_point);
    if prune.contains(&entry_va) {
        return Err(format!("prune list: the entry point {entry_va:#x} is listed"));
    }
    fn_queue.push_back((entry_va, Some("<entry>".to_string())));
    for e in pe.exports().map_err(|e| format!("exports: {e}"))? {
        let name = e.name.clone().unwrap_or_else(|| format!("#{}", e.ordinal));
        if prune.contains(&e.va) {
            cl.pruned_seeds_dropped += 1;
        } else if text.contains(e.va) {
            fn_queue.push_back((e.va, Some(name)));
        } else {
            cl.seeds_skipped.push(SkippedSeed { name, va: e.va, reason: "export outside .text (data)" });
        }
    }
    for &v in extra {
        if prune.contains(&v) {
            cl.pruned_seeds_dropped += 1;
            continue;
        }
        fn_queue.push_back((v, Some(format!("<seed:{v:#x}>"))));
    }
    for &v in swap {
        fn_queue.push_back((v, None));
    }

    let mut insns: BTreeMap<u64, Instruction> = BTreeMap::new();
    let mut functions: BTreeMap<u64, Vec<u64>> = BTreeMap::new();
    let mut fn_seen: HashSet<u64> = HashSet::new();
    let mut block_seen: HashSet<u64> = HashSet::new();

    while let Some((f, seed)) = fn_queue.pop_front() {
        if !fn_seen.insert(f) {
            continue;
        }
        if !text.contains(f) {
            cl.out_of_text_calls += 1;
            continue;
        }
        if let Some(name) = seed {
            cl.seeds_code.push((name, f));
        }
        functions.entry(f).or_default();
        if swap.contains(&f) || prune.contains(&f) {
            continue;
        }
        let mut blocks: VecDeque<u64> = VecDeque::from([f]);
        while let Some(b) = blocks.pop_front() {
            if !block_seen.insert(b) {
                continue;
            }
            cl.blocks += 1;
            let mut va = b;
            let mut prev: Option<Instruction> = None;
            loop {
                if insns.contains_key(&va) {
                    break;
                }
                let i = match text.decode(va) {
                    Decoded::Ok(i) => i,
                    Decoded::OutOfText => {
                        cl.out_of_text_flow += 1;
                        break;
                    }
                    Decoded::Invalid => {
                        cl.decode_failures += 1;
                        if cl.fail_samples.len() < 32 {
                            cl.fail_samples.push(va);
                        }
                        break;
                    }
                };
                let len = i.len() as u64;
                cl.instructions += 1;
                *cl.mnemonics.entry(format!("{:?}", i.mnemonic())).or_insert(0) += 1;
                functions.get_mut(&f).expect("function entry inserted").push(va);
                insns.insert(va, i.clone());
                let before = prev.replace(i.clone());

                let next = va + len;
                match i.flow_control() {
                    FlowControl::Next | FlowControl::XbeginXabortXend => {
                        va = next;
                    }
                    FlowControl::UnconditionalBranch => {
                        if is_direct_near(&i) {
                            cl.direct_jumps += 1;
                            if swap.contains(&i.near_branch_target()) || prune.contains(&i.near_branch_target()) {
                                fn_queue.push_back((i.near_branch_target(), None));
                            } else {
                                blocks.push_back(i.near_branch_target());
                            }
                        } else {
                            cl.far_transfers += 1;
                        }
                        break;
                    }
                    FlowControl::ConditionalBranch => {
                        cl.conditional_branches += 1;
                        if prune.contains(&i.near_branch_target()) {
                            cl.prune_cond_targets += 1;
                        }
                        blocks.push_back(i.near_branch_target());
                        blocks.push_back(next);
                        break;
                    }
                    FlowControl::Call => {
                        if is_direct_near(&i) {
                            cl.direct_calls += 1;
                            fn_queue.push_back((i.near_branch_target(), None));
                        } else {
                            cl.far_transfers += 1;
                        }
                        va = next;
                    }
                    FlowControl::IndirectCall => {
                        if import_slot(&i, &iat) {
                            cl.import_calls += 1;
                        } else {
                            cl.indirect_calls_other += 1;
                        }
                        let r = resolver.resolve(b, va, &i, before.as_ref());
                        cl.indirect.record("call", &r);
                        if let Resolved::Code { target, .. } = &r {
                            fn_queue.push_back((*target, None));
                        }
                        resolved.insert(va, r);
                        va = next;
                    }
                    FlowControl::IndirectBranch => {
                        if import_slot(&i, &iat) {
                            cl.import_jumps += 1;
                        } else {
                            cl.indirect_jumps_other += 1;
                        }
                        if i.op_kind(0) == OpKind::Register {
                            cl.register_jumps += 1;
                        }
                        let r = resolver.resolve(b, va, &i, before.as_ref());
                        cl.indirect.record("jmp", &r);
                        if let Resolved::Code { target, .. } = &r {
                            if prune.contains(target) {
                                fn_queue.push_back((*target, None));
                            } else {
                                blocks.push_back(*target);
                            }
                        }
                        if r == Resolved::Unresolved {
                            if let Some(s) = spec_at.get(&va) {
                                match jumptab::resolve(pe, &text, s) {
                                    Ok(t) => {
                                        cl.indirect.move_kind("jmp:unresolved", "jmp:table");
                                        cl.jumptabs += 1;
                                        for &(_, tg) in &t.cases {
                                            blocks.push_back(tg);
                                        }
                                        jumptabs.insert(va, t);
                                    }
                                    Err(e) => cl.jumptab_rejected.push((va, e)),
                                }
                            } else if let Some(form) = jumptab::recognize_stub(&text, va, &i) {
                                if relocs.is_none() {
                                    relocs = Some(pe.dir64_relocation_vas().map_err(|e| format!("relocations: {e}"))?.into_iter().collect());
                                }
                                match jumptab::resolve_stub(pe, &text, relocs.as_ref().expect("set above"), &form) {
                                    Ok(t) => {
                                        cl.indirect.move_kind("jmp:unresolved", t.kind_key());
                                        cl.jumptabs += 1;
                                        cl.stub_tables += 1;
                                        for &(_, tg) in &t.cases {
                                            fn_queue.push_back((tg, None));
                                        }
                                        jumptabs.insert(va, t);
                                    }
                                    Err(e) => cl.jumptab_rejected.push((va, e)),
                                }
                            } else if let Some(form) = jumptab::recognize_inline(va, &text, &i) {
                                if relocs.is_none() {
                                    relocs = Some(pe.dir64_relocation_vas().map_err(|e| format!("relocations: {e}"))?.into_iter().collect());
                                }
                                let fm = funcmap.get_or_insert_with(|| jumptab::FuncMap::from_pe(pe));
                                // not an inline table (no `.pdata`, the table is not in the function, no relocated pointers into it): nothing
                                // is reported, the jump stays unresolved as before
                                let t = match fm {
                                    Ok(fm) => jumptab::resolve_inline(pe, fm, relocs.as_ref().expect("set above"), &form, va),
                                    Err(_) => None,
                                };
                                if let Some(t) = t {
                                    cl.indirect.move_kind("jmp:unresolved", t.kind_key());
                                    cl.jumptabs += 1;
                                    cl.inline_tables += 1;
                                    for &(_, tg) in &t.cases {
                                        blocks.push_back(tg);
                                    }
                                    jumptabs.insert(va, t);
                                }
                            } else if i.op_kind(0) == OpKind::Register {
                                // the swapped form (decision 0095) is tried only when the standard form finds no table idiom, and
                                // only with the base proof by reaching definitions: the comparison-only proofs of decisions 0089
                                // and 0090 leave such a site as they always did (not recognised, not reported)
                                //
                                // decision 0109: a standard idiom whose N the closure's own walk back could not derive (`Err`: other
                                // instructions between the `cmp; ja` and the idiom, or between the `cmp` and the `ja`, or a second way
                                // into the idiom) gets a second reading with the guard taken from the CFG of the function
                                // (`recognize_msvc_scheduled`, resolved by `resolve_msvc`). `strict` keeps the first reason: a site that
                                // stays rejected for lack of an N reports that one, as before, followed by the reason of the second reading
                                let mut strict: Option<String> = None;
                                let found = match jumptab::recognize_msvc_standard(&insns, va, &i) {
                                    Ok(None) => jumptab::recognize_msvc_swapped(&insns, va, &i),
                                    Err(e) => match jumptab::recognize_msvc_scheduled(&insns, va, &i) {
                                        Ok(Some(form)) => {
                                            strict = Some(e);
                                            Ok(Some(form))
                                        }
                                        _ => Err(e),
                                    },
                                    Ok(Some(form)) => Ok(Some(form)),
                                };
                                match found {
                                    Ok(None) => {}
                                    Err(e) => cl.msvc_rejected.push((va, e)),
                                    Ok(Some(form)) => {
                                        let fm = funcmap.get_or_insert_with(|| jumptab::FuncMap::from_pe(pe));
                                        let t = match fm {
                                            Ok(fm) => jumptab::resolve_msvc(pe, &text, fm, &mut sweeps, &form, va),
                                            Err(e) => Err(e.clone()),
                                        };
                                        match t {
                                            Ok(t) => {
                                                cl.indirect.move_kind("jmp:unresolved", t.kind_key());
                                                cl.jumptabs += 1;
                                                cl.msvc_tables += 1;
                                                for &(_, tg) in &t.cases {
                                                    blocks.push_back(tg);
                                                }
                                                jumptabs.insert(va, t);
                                            }
                                            Err(e) => {
                                                let e = match strict {
                                                    Some(first) if e.starts_with("N not derived") => format!("{first} [{e}]"),
                                                    _ => e,
                                                };
                                                cl.msvc_rejected.push((va, e));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        resolved.insert(va, r);
                        break;
                    }
                    FlowControl::Return => {
                        cl.returns += 1;
                        break;
                    }
                    FlowControl::Interrupt | FlowControl::Exception => {
                        cl.interrupts += 1;
                        break;
                    }
                }
            }
        }
    }

    // Targets pushed through fn_queue that lie outside .text were counted above;
    // fall-through and branch targets outside .text were counted in out_of_text_flow.
    // A register-based resolution is valid only if the instruction before the
    // branch is on every path into it. Drop the resolutions whose branch lies on
    // a block leader, and any leader reached by a path that skips the register set.
    for (&va, r) in resolved.iter_mut() {
        if let Resolved::Code { how: how @ (How::RegConst | How::RegTable), .. } = *r {
            if block_seen.contains(&va) {
                let kind = if insns[&va].flow_control() == FlowControl::IndirectCall { "call" } else { "jmp" };
                cl.indirect.demote(kind, how);
                *r = Resolved::Unresolved;
            }
        }
    }

    // MSVC tables (decision 0089): is there a way into the function other than its entry, or into the switch idiom other
    // than through the guard? Both are branch sources whose target lies in a protected range.
    let (foreign, landings) = {
        let mut protected: Vec<(u64, u64, u32, u64)> = Vec::new();
        let mut idioms: Vec<(u64, u64)> = Vec::new();
        // the swapped form (decision 0095): (first address only the guard's branch may enter, jump, the guard's branch)
        let mut swapped_idioms: Vec<(u64, u64, u64, u64)> = Vec::new();
        let mut seen: HashSet<u32> = HashSet::new();
        for (&va, t) in &jumptabs {
            if let jumptab::Origin::Msvc { function, guard_cmp, guard_ja, from, swapped, entry2, .. } = t.origin {
                match swapped {
                    Some(lo) => {
                        swapped_idioms.push((lo, va, guard_ja, from));
                        // decision 0109: the second guard of a loop-rotated switch is a second way into the same run
                        if let Some((branch2, at2)) = entry2 {
                            swapped_idioms.push((lo, va, branch2, at2));
                        }
                    }
                    None => idioms.push((guard_cmp, va)),
                }
                if seen.insert(function) {
                    let entry = pe.rva_to_va(function);
                    protected.extend(sweeps[&function].pieces().iter().map(|&(b, e)| (b, e, function, entry)));
                }
            }
        }
        protected.sort_unstable();
        if protected.is_empty() {
            (BTreeMap::new(), BTreeMap::new())
        } else {
            let mut sources: Vec<(u64, u64)> = Vec::new();
            for (&va, i) in &insns {
                if matches!(i.flow_control(), FlowControl::UnconditionalBranch | FlowControl::ConditionalBranch) && is_direct_near(i) {
                    sources.push((va, i.near_branch_target()));
                }
            }
            for (&va, r) in &resolved {
                if let Resolved::Code { target, .. } = r {
                    if insns[&va].flow_control() == FlowControl::IndirectBranch {
                        sources.push((va, *target));
                    }
                }
            }
            for (&va, t) in &jumptabs {
                sources.extend(t.cases.iter().map(|&(_, tg)| (va, tg)));
            }
            let mut landings = jumptab::landings_in_idioms(&idioms, sources.iter().copied());
            landings.extend(jumptab::landings_in_swapped_idioms(&swapped_idioms, sources.iter().copied()));
            (jumptab::foreign_entries(&protected, sources.iter().copied()), landings)
        }
    };

    let bad: Vec<(u64, String)> = jumptabs
        .iter()
        .filter_map(|(&va, t)| {
            if keep_unresolved.contains(&va) {
                return Some((va, "kept unresolved (prune list, class U: the unpruned lift rejects this jump)".to_string()));
            }
            let stub = match t.origin {
                jumptab::Origin::Stub { .. } => jumptab::verify_stub(va, &insns, &block_seen),
                jumptab::Origin::Annotated => Ok(()),
                jumptab::Origin::Inline { .. } => jumptab::verify_inline(va, &insns),
                jumptab::Origin::Msvc { from, guard_ja, guard_cmp, function, swapped, entry2, .. } => {
                    let entry = pe.rva_to_va(function);
                    match (foreign.get(&function), landings.get(&va)) {
                        (Some(&(src, tgt)), _) => {
                            Err(format!("the instruction at {src:#x}, outside the function {entry:#x}, jumps to {tgt:#x} inside it (not its entry)"))
                        }
                        (None, Some(&(src, tgt))) => {
                            Err(format!("the branch at {src:#x} lands at {tgt:#x}, inside the switch idiom (the guard `cmp; ja` is skipped)"))
                        }
                        (None, None) if swapped.is_some() => jumptab::verify_msvc_swapped_entries(
                            va,
                            from,
                            swapped.expect("checked"),
                            (guard_ja, guard_cmp),
                            entry2.map(|e| e.1),
                            entry,
                            &insns,
                            &block_seen,
                            &functions,
                            &sweeps[&function],
                        ),
                        (None, None) => jumptab::verify_msvc(va, from, (guard_ja, guard_cmp), entry, &insns, &block_seen, &functions, &sweeps[&function]),
                    }
                }
            };
            stub.and_then(|()| jumptab::verify_except(t, &insns, &|tg| prune.contains(&tg))).err().map(|e| (va, e))
        })
        .collect();
    for (va, e) in bad {
        if let Some(t) = jumptabs.remove(&va) {
            if matches!(t.origin, jumptab::Origin::Stub { .. }) {
                cl.stub_tables -= 1;
            }
            if matches!(t.origin, jumptab::Origin::Inline { .. }) {
                cl.inline_tables -= 1;
            }
            cl.indirect.move_kind(t.kind_key(), "jmp:unresolved");
            if matches!(t.origin, jumptab::Origin::Msvc { .. }) {
                cl.msvc_tables -= 1;
                cl.jumptabs -= 1;
                cl.msvc_rejected.push((va, e));
                continue;
            }
        }
        cl.jumptabs -= 1;
        cl.jumptab_rejected.push((va, e));
    }

    cl.functions = functions.len();
    cl.seeds_code.sort_by_key(|(_, v)| *v);
    for v in functions.values_mut() {
        v.sort_unstable();
    }
    Ok(Analysis { closure: cl, insns, functions, resolved, jumptabs })
}

fn import_slot(i: &Instruction, iat: &HashSet<u64>) -> bool {
    i.op_kind(0) == OpKind::Memory && i.is_ip_rel_memory_operand() && iat.contains(&i.ip_rel_memory_address())
}
