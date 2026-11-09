//! Tests for the dataflow resolver: synthetic PE images (each case checked by
//! exact address and reason), and a real-DLL measurement that is skipped when the
//! DLL is absent (`$FLENGINE_DLL`, else `binaries/win64/FLEngine_x64.dll`).

use fe_decode::indirect::dataflow::{self, leaders, take_log, Class};
use fe_decode::{analyze, How, Resolved, Text};
use fe_pe::Pe;
use iced_x86::{FlowControl, Instruction, InstructionInfoFactory, Mnemonic, OpAccess, OpKind, Register};
use std::collections::{BTreeMap, BTreeSet, HashSet};

const IB: u64 = 0x1_4000_0000;
const LFANEW: usize = 0x80;
const OPT: usize = LFANEW + 24;
const SEC: usize = OPT + 240;
const TEXT_RVA: u32 = 0x1000;
const TEXT_RAW: usize = 0x400;
const DATA_RVA: u32 = 0x2000;
const DATA_RAW: usize = 0x800;
const FILE_LEN: usize = 0x1000;

fn w16(b: &mut [u8], o: usize, v: u16) {
    b[o..o + 2].copy_from_slice(&v.to_le_bytes());
}
fn w32(b: &mut [u8], o: usize, v: u32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}
fn w64(b: &mut [u8], o: usize, v: u64) {
    b[o..o + 8].copy_from_slice(&v.to_le_bytes());
}
fn data_at(rva: u32) -> usize {
    DATA_RAW + (rva - DATA_RVA) as usize
}
/// disp32 for a rip-relative operand whose instruction ends at `next`.
fn rel(next: u32, target: u32) -> [u8; 4] {
    ((target as i64 - next as i64) as i32).to_le_bytes()
}
fn cat(parts: &[&[u8]]) -> Vec<u8> {
    parts.concat()
}

/// Synthetic PE32+: `.text` at RVA 0x1000, `.data` at 0x2000 with an import
/// (`k.dll!Fn`, IAT slot 0x2200) and a function-pointer slot at 0x2500 -> 0x1130.
///
/// `.text` (RVA):
/// - 0x1000 dispatcher: `call` 0x1020, 0x1040, 0x1050, 0x1090, 0x1200; `ret`.
/// - 0x1020 `mov rax,[rip+0x2500]; xor ecx,ecx; call rax; ret`.
///   Expect: call at 0x1029 -> 0x1130, `RegTable`.
/// - 0x1040 `call [rcx+8]; ret`. Expect: call at 0x1040 unresolved, `Arg`.
/// - 0x1050 `lea rax,[rip+0x1080]; nop; nop; call rax; ret`, with 0x1058 a leader
///   (jmp at 0x1070). Expect: call at 0x1059 unresolved, `interior_leader`.
/// - 0x1070 `jmp 0x1058` (only makes 0x1058 a leader, unreached).
/// - 0x1080 `ret` (target of the lea above).
/// - 0x1090 `lea rax,[rip+0x1080]; call 0x1040; call rax; ret`.
///   Expect: call at 0x109c unresolved, `unknown` (the direct call clobbers rax).
/// - 0x1200 `lea rax,[rip+0x1300]; add rax,0x10; call rax; ret`.
///   Expect: call at 0x120b -> 0x1310, `RegConst`.
/// - 0x1130 `ret`, 0x1310 `ret`.
fn build() -> Vec<u8> {
    let mut b = vec![0u8; FILE_LEN];
    w16(&mut b, 0, 0x5a4d);
    w32(&mut b, 0x3c, LFANEW as u32);
    w32(&mut b, LFANEW, 0x4550);
    let f = LFANEW + 4;
    w16(&mut b, f, 0x8664);
    w16(&mut b, f + 2, 2);
    w16(&mut b, f + 16, 240);
    w16(&mut b, f + 18, 0x2022);
    let o = OPT;
    w16(&mut b, o, 0x20b);
    w32(&mut b, o + 16, TEXT_RVA);
    w64(&mut b, o + 24, IB);
    w32(&mut b, o + 32, 0x1000);
    w32(&mut b, o + 36, 0x200);
    w32(&mut b, o + 56, 0x3000);
    w32(&mut b, o + 60, 0x400);
    w16(&mut b, o + 68, 3);
    w32(&mut b, o + 108, 16);
    w32(&mut b, o + 120, DATA_RVA);
    w32(&mut b, o + 124, 40);

    b[SEC..SEC + 5].copy_from_slice(b".text");
    w32(&mut b, SEC + 8, 0x400);
    w32(&mut b, SEC + 12, TEXT_RVA);
    w32(&mut b, SEC + 16, 0x400);
    w32(&mut b, SEC + 20, TEXT_RAW as u32);
    w32(&mut b, SEC + 36, 0x6000_0020);
    let s1 = SEC + 40;
    b[s1..s1 + 5].copy_from_slice(b".data");
    w32(&mut b, s1 + 8, 0x800);
    w32(&mut b, s1 + 12, DATA_RVA);
    w32(&mut b, s1 + 16, 0x800);
    w32(&mut b, s1 + 20, DATA_RAW as u32);
    w32(&mut b, s1 + 36, 0xc000_0040);

    let d = data_at(DATA_RVA);
    w32(&mut b, d, 0x2100);
    w32(&mut b, d + 12, 0x2300);
    w32(&mut b, d + 16, 0x2200);
    w64(&mut b, data_at(0x2100), 0x2400);
    w64(&mut b, data_at(0x2200), 0x2400);
    b[data_at(0x2300)..data_at(0x2300) + 6].copy_from_slice(b"k.dll\0");
    b[data_at(0x2402)..data_at(0x2402) + 3].copy_from_slice(b"Fn\0");
    w64(&mut b, data_at(0x2500), IB + 0x1130);

    let t = |rva: u32| TEXT_RAW + (rva - TEXT_RVA) as usize;
    let call = |from: u32, to: u32| cat(&[&[0xe8], &rel(from + 5, to)]);
    let lea_rip = |at: u32, to: u32| cat(&[&[0x48, 0x8d, 0x05], &rel(at + 7, to)]);
    let code: Vec<(u32, Vec<u8>)> = vec![
        (0x1000, cat(&[&call(0x1000, 0x1020), &call(0x1005, 0x1040), &call(0x100a, 0x1050), &call(0x100f, 0x1090), &call(0x1014, 0x1200), &[0xc3]])),
        (0x1020, cat(&[&[0x48, 0x8b, 0x05], &rel(0x1027, 0x2500), &[0x31, 0xc9, 0xff, 0xd0, 0xc3]])),
        (0x1040, vec![0xff, 0x51, 0x08, 0xc3]),
        (0x1050, cat(&[&lea_rip(0x1050, 0x1080), &[0x90, 0x90, 0xff, 0xd0, 0xc3]])),
        (0x1070, vec![0xeb, 0xe6, 0xc3]),
        (0x1080, vec![0xc3]),
        (0x1090, cat(&[&lea_rip(0x1090, 0x1080), &call(0x1097, 0x1040), &[0xff, 0xd0, 0xc3]])),
        (0x1130, vec![0xc3]),
        (0x1200, cat(&[&lea_rip(0x1200, 0x1300), &[0x48, 0x83, 0xc0, 0x10, 0xff, 0xd0, 0xc3]])),
        (0x1310, vec![0xc3]),
    ];
    for (rva, bytes) in &code {
        let o = t(*rva);
        b[o..o + bytes.len()].copy_from_slice(bytes);
    }
    b
}

fn resolved_at(a: &fe_decode::Analysis, va: u64) -> Resolved {
    a.resolved.get(&va).cloned().unwrap_or(Resolved::Unresolved)
}

#[test]
fn synthetic_cases_have_exact_outcomes() {
    let img = build();
    let pe = Pe::parse(&img).unwrap();
    let _ = take_log();
    let a = analyze(&pe).unwrap();
    let log = take_log();
    let by_va: BTreeMap<u64, &dataflow::Record> = log.iter().map(|r| (r.va, r)).collect();

    // Constant arithmetic after a lea: only the dataflow resolver sees the add.
    assert_eq!(resolved_at(&a, IB + 0x120b), Resolved::Code { target: IB + 0x1310, how: How::RegConst });
    assert_eq!(by_va[&(IB + 0x120b)].verdict.class, Class::Arith);

    // File-content read through a rip slot, then an indirect call through the register.
    assert_eq!(resolved_at(&a, IB + 0x1029), Resolved::Code { target: IB + 0x1130, how: How::RegTable });
    assert_eq!(by_va[&(IB + 0x1029)].verdict.class, Class::Mem);

    // Direct call target: a leader, so no register value is trusted here.
    assert_eq!(resolved_at(&a, IB + 0x1040), Resolved::Unresolved);
    assert_eq!(by_va[&(IB + 0x1040)].verdict.why, "branch_is_leader");

    // The run starts after the leader 0x1058: rax is unknown there, not the lea value.
    assert_eq!(resolved_at(&a, IB + 0x1059), Resolved::Unresolved);
    assert_eq!(by_va[&(IB + 0x1059)].verdict.why, "unknown");

    // A direct call between the definition and the branch clobbers rax.
    assert_eq!(resolved_at(&a, IB + 0x109c), Resolved::Unresolved);
    assert_eq!(by_va[&(IB + 0x109c)].verdict.why, "unknown");

    let total: usize = a.resolved.len();
    assert_eq!(total, 5, "indirect transfers in the synthetic image");
    let code = a.resolved.values().filter(|r| matches!(r, Resolved::Code { .. })).count();
    assert_eq!(code, 2);
}

#[test]
fn leaders_include_branch_targets_and_fallthrough() {
    let img = build();
    let pe = Pe::parse(&img).unwrap();
    let text = Text::from_pe(&pe).unwrap();
    let l = leaders(&text);
    assert!(l.contains(&(IB + 0x1058)), "target of jmp at 0x1070");
    assert!(l.contains(&(IB + 0x1040)), "target of call at 0x1005 (direct call target)");
    assert!(!l.contains(&(IB + 0x1057)));
}

#[test]
fn stores_to_slots_before_branch_are_not_trusted() {
    // Same as the 0x1020 case, but with a store to non-stack memory in between:
    // the slot content may have changed at run time, so it must not be used.
    let mut img = build();
    let t = |rva: u32| TEXT_RAW + (rva - TEXT_RVA) as usize;
    // 0x1020: mov rax,[rip+0x2500] (7) ; mov [rdx],rcx (48 89 0a, 3) ; xor ecx,ecx ; call rax
    let bytes = cat(&[&[0x48, 0x8b, 0x05], &rel(0x1027, 0x2500), &[0x48, 0x89, 0x0a, 0x31, 0xc9, 0xff, 0xd0, 0xc3]]);
    img[t(0x1020)..t(0x1020) + bytes.len()].copy_from_slice(&bytes);
    let pe = Pe::parse(&img).unwrap();
    let _ = take_log();
    let a = analyze(&pe).unwrap();
    let log = take_log();
    // 0x1020 mov (7) + mov [rdx],rcx (3) + xor (2) = 0x102c for the call.
    // The store is not modelled, so rax is unknown (not the slot content).
    let rec = log.iter().find(|r| r.va == IB + 0x102c).expect("call after store");
    assert_eq!(rec.verdict.why, "unknown");
    assert_eq!(resolved_at(&a, IB + 0x102c), Resolved::Unresolved);
}

/// Real DLL: counts only, no expected values. Skipped when the file is absent.
#[test]
fn real_dll_breakdown() {
    let path = std::env::var("FLENGINE_DLL")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../binaries/win64/FLEngine_x64.dll"));
    let Ok(data) = std::fs::read(&path) else {
        eprintln!("skip: {} not found", path.display());
        return;
    };
    let pe = Pe::parse(&data).unwrap();
    let text = Text::from_pe(&pe).unwrap();
    let lead = leaders(&text);
    let _ = take_log();
    let a = analyze(&pe).unwrap();
    let log = take_log();

    let mut dec: BTreeMap<String, usize> = BTreeMap::new();
    for r in &log {
        let k = match &r.verdict.resolved {
            Resolved::Code { how, .. } => format!("code:{}", how.as_str()),
            Resolved::Import { .. } => "import".into(),
            Resolved::Unresolved => "unresolved".into(),
        };
        *dec.entry(format!("{} {k} [{}]", if r.is_call { "call" } else { "jmp" }, r.verdict.why)).or_insert(0) += 1;
    }
    eprintln!("dataflow decisions {}", log.len());
    for (k, v) in &dec {
        eprintln!("  {v:>7}  {k}");
    }

    let iat: HashSet<u64> = pe.imports().unwrap().iter().flat_map(|i| i.entries.iter()).map(|e| e.iat_va).collect();
    let mut info = InstructionInfoFactory::new();
    let mut cls: BTreeMap<String, usize> = BTreeMap::new();
    let mut root_detail: BTreeMap<String, usize> = BTreeMap::new();
    let mut total = 0usize;
    for (&va, r) in &a.resolved {
        if *r != Resolved::Unresolved {
            continue;
        }
        total += 1;
        let i = &a.insns[&va];
        let kind = if i.flow_control() == FlowControl::IndirectCall { "call" } else { "jmp" };
        let c = if i.op_kind(0) == OpKind::Memory && i.is_ip_rel_memory_operand() {
            if iat.contains(&i.ip_rel_memory_address()) { "c:iat" } else { "c:rip_slot" }.to_string()
        } else {
            let base = if i.op_kind(0) == OpKind::Register { i.op_register(0) } else { i.memory_base() };
            let body = block_body(&a.insns, &lead, va);
            let rt = root(&body, base, &mut info, 0);
            *root_detail.entry(format!("{rt:?}")).or_insert(0) += 1;
            class_of(&rt).to_string()
        };
        *cls.entry(format!("{kind} {c}")).or_insert(0) += 1;
    }
    eprintln!("unresolved after resolver: {total}");
    for (k, v) in &cls {
        eprintln!("  {v:>7}  {k}");
    }
    eprintln!("roots (register branches):");
    for (k, v) in &root_detail {
        eprintln!("  {v:>7}  {k}");
    }
    assert_eq!(total, a.resolved.values().filter(|r| **r == Resolved::Unresolved).count());
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Root {
    Const,
    Slot,
    PtrLoad(Box<Root>),
    Arg,
    Call,
    Other(String),
}

fn writes(a: OpAccess) -> bool {
    matches!(a, OpAccess::Write | OpAccess::CondWrite | OpAccess::ReadWrite | OpAccess::ReadCondWrite)
}

/// Root of the register's definition inside `body`, following `mov reg,reg` and
/// pointer loads (`mov reg,[reg2+d]`) back to where the first value came from.
fn root(body: &[&Instruction], r: Register, info: &mut InstructionInfoFactory, depth: usize) -> Root {
    if depth > 16 {
        return Root::Other("depth".into());
    }
    let mut reg = r.full_register();
    let mut end = body.len();
    loop {
        let Some(k) = body[..end].iter().rposition(|d| info.info(d).used_registers().iter().any(|u| writes(u.access()) && u.register().full_register() == reg)) else {
            return if matches!(reg, Register::RCX | Register::RDX | Register::R8 | Register::R9) { Root::Arg } else { Root::Other("entry_non_arg".into()) };
        };
        let d = body[k];
        if matches!(d.flow_control(), FlowControl::Call | FlowControl::IndirectCall) {
            return Root::Call;
        }
        let dst_reg = d.op_count() > 0 && d.op_kind(0) == OpKind::Register;
        return match d.mnemonic() {
            Mnemonic::Lea if d.is_ip_rel_memory_operand() => Root::Const,
            Mnemonic::Mov if dst_reg && matches!(d.op_kind(1), OpKind::Immediate32to64 | OpKind::Immediate64) => Root::Const,
            Mnemonic::Mov if dst_reg && d.op_kind(1) == OpKind::Register => {
                reg = d.op_register(1).full_register();
                end = k;
                continue;
            }
            Mnemonic::Mov if dst_reg && d.op_kind(1) == OpKind::Memory && d.is_ip_rel_memory_operand() => Root::Slot,
            Mnemonic::Mov if dst_reg && d.op_kind(1) == OpKind::Memory => {
                let b = d.memory_base();
                if b == Register::None || d.memory_index() != Register::None {
                    return Root::Other("load_no_base".into());
                }
                Root::PtrLoad(Box::new(root(&body[..k], b, info, depth + 1)))
            }
            m => Root::Other(format!("{m:?}")),
        };
    }
}

/// Class (a, b, c, d) of a chain root.
/// - a: the register is a constant (`lea rip`, `mov imm`).
/// - b: the register is an argument (or a pointer loaded from an argument).
/// - c: the register is read from a rip-relative slot, or loaded through one
///   constant pointer (`mov reg,[rip+X]` then `call [reg+d]`).
/// - d: anything else, including a second level of pointer loads.
fn class_of(r: &Root) -> &'static str {
    match r {
        Root::Const => "a",
        Root::Slot => "c",
        Root::Arg => "b",
        Root::PtrLoad(inner) => match **inner {
            Root::Const | Root::Slot => "c",
            Root::Arg => "b",
            _ => "d",
        },
        _ => "d",
    }
}

fn block_body<'a>(insns: &'a BTreeMap<u64, Instruction>, leaders: &BTreeSet<u64>, va: u64) -> Vec<&'a Instruction> {
    let mut start = va;
    while !leaders.contains(&start) {
        match insns.range(..start).next_back() {
            Some((&p, pi)) if pi.next_ip() == start => start = p,
            _ => break,
        }
    }
    insns.range(start..va).map(|(_, x)| x).collect()
}
