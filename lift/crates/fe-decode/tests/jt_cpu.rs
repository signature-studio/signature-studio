//! Selector stubs (decision 0079): `movsxd I,[rip+sel]; lea B,[rip+table]; jmp [B+I*8]` resolved without an annotation.
//! The entry count is the run of DIR64-relocated slots that point into code. Synthetic PE32+ (ImageBase 0x1_4000_0000):
//!
//! | section | RVA    | content |
//! |---------|--------|---------|
//! | .text   | 0x1000 | S1 at 0x1000 (`movsxd rax; lea r10; jmp [r10+rax*8]`, table A), S2 at 0x1020 (`movsxd rcx; lea rdx; jmp [rdx+rcx*8]`, table B at 0x1060 in .text), targets T0..T3 at 0x1100+0x10k, main at 0x1200 |
//! | .data   | 0x2000 | selector A at 0x2000, selector B at 0x2004 (both -1), slot -1 at 0x2008, table A at 0x2010 (T0..T3), a relocated pointer to .data at 0x2030 |
//! | .reloc  | 0x3000 | DIR64 for the table slots |
//!
//! Table A has 4 entries: the slot after it (0x2030) is relocated but points into data. Table B has 3 entries
//! (T2, T0, T2): the slot after it (0x1078) holds T3 but is not relocated. Entry -1 (0x2008) is relocated and points
//! to code but is not part of the table. The real FLEngine_x64.dll has no such stub (`stub_tables` 0, `jumptabs` 4).
//! The real dsp_ippv2_x64.dll ($DSP_IPPV2_PE, else lift/out-ext/dsp_ippv2_x64.dll; skipped when absent): 434 stubs.

use fe_decode::{analyze, analyze_with_pdata_adjustors, Origin, Resolved, Select};
use fe_pe::Pe;
use std::path::PathBuf;

const IB: u64 = 0x1_4000_0000;
const LFANEW: usize = 0x80;
const OPT_SIZE: usize = 240;
const SEC_TABLE: usize = LFANEW + 24 + OPT_SIZE;
const FILE_LEN: usize = 0xc00;

fn w16(b: &mut [u8], o: usize, v: u16) {
    b[o..o + 2].copy_from_slice(&v.to_le_bytes());
}
fn w32(b: &mut [u8], o: usize, v: u32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}
fn w64(b: &mut [u8], o: usize, v: u64) {
    b[o..o + 8].copy_from_slice(&v.to_le_bytes());
}
fn rel(next: u32, target: u32) -> [u8; 4] {
    ((target as i64 - next as i64) as i32).to_le_bytes()
}

#[derive(Clone, Copy, Default)]
struct Variant {
    /// slot 0 of table A is not in the relocation directory
    unrelocated_a: bool,
    /// S1's jump has a displacement (`jmp [r10+rax*8+8]`): not the stub form
    disp: bool,
    /// main ends with a jump to the `lea` of S1
    jump_into_lea: bool,
    /// decision 0114: a third stub S3 (selector A, table C in .data with 2 entries T1, T3) whose slot -1 is built as the variant says; main calls it
    s3: S3,
}

/// How S3's lazy-initialisation path (the 16 bytes in front of the stub and the slot in front of table C) is built.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum S3 {
    /// no S3 at all
    #[default]
    Absent,
    /// slot -1 (0x2040) is relocated and holds S3 - 16 (0x1280); there are `call T0` and the 11-byte `nop word ptr [rax+rax]`
    Valid,
    /// as Valid, but slot -1 is not in the relocation directory
    Unrelocated,
    /// as Valid, but slot -1 holds T0 (0x1100), not S3 - 16
    WrongValue,
    /// as Valid, but the first byte at S3 - 16 is E9 (jmp rel32), not E8
    NotCall,
    /// as Valid, but the 11 bytes after the call are eleven 0x90
    WrongNop,
    /// as Valid, but the call goes to 0x3000 (the .reloc section, not code)
    CallOutside,
}

const S3_START: u64 = IB + 0x1290;
const S3_JMP: u64 = IB + 0x129e;
const S3_INIT: u64 = IB + 0x1280;

fn build(v: Variant) -> Vec<u8> {
    let mut b = vec![0u8; FILE_LEN];
    w16(&mut b, 0, 0x5a4d);
    w32(&mut b, 0x3c, LFANEW as u32);
    w32(&mut b, LFANEW, 0x4550);
    let f = LFANEW + 4;
    w16(&mut b, f, 0x8664);
    w16(&mut b, f + 2, 3);
    w16(&mut b, f + 16, OPT_SIZE as u16);
    w16(&mut b, f + 18, 0x2022);
    let o = LFANEW + 24;
    w16(&mut b, o, 0x20b);
    w32(&mut b, o + 16, 0x1200);
    w64(&mut b, o + 24, IB);
    w32(&mut b, o + 32, 0x1000);
    w32(&mut b, o + 36, 0x200);
    w32(&mut b, o + 56, 0x4000);
    w32(&mut b, o + 60, 0x400);
    w16(&mut b, o + 68, 3);
    w32(&mut b, o + 108, 16);
    let section = |b: &mut Vec<u8>, i: usize, name: &[u8], rva: u32, raw: u32, flags: u32| {
        let s = SEC_TABLE + 40 * i;
        b[s..s + name.len()].copy_from_slice(name);
        w32(b, s + 8, 0x400);
        w32(b, s + 12, rva);
        w32(b, s + 16, 0x200);
        w32(b, s + 20, raw);
        w32(b, s + 36, flags);
    };
    section(&mut b, 0, b".text", 0x1000, 0x400, 0x6000_0020);
    section(&mut b, 1, b".data", 0x2000, 0x800, 0xc000_0040);
    section(&mut b, 2, b".reloc", 0x3000, 0xa00, 0x4200_0040);
    // .text is 0x400 long in the image but 0x200 in the file: make the file part the whole of it
    w32(&mut b, SEC_TABLE + 16, 0x400);

    let text = |b: &mut Vec<u8>, rva: u32, bytes: &[u8]| {
        let o = 0x400 + (rva - 0x1000) as usize;
        b[o..o + bytes.len()].copy_from_slice(bytes);
    };
    // S1: movsxd rax,[rip+selA]; lea r10,[rip+tabA]; jmp [r10+rax*8]
    text(&mut b, 0x1000, &[&[0x48, 0x63, 0x05][..], &rel(0x1007, 0x2000)].concat());
    text(&mut b, 0x1007, &[&[0x4c, 0x8d, 0x15][..], &rel(0x100e, 0x2010)].concat());
    if v.disp {
        text(&mut b, 0x100e, &[0x41, 0xff, 0x64, 0xc2, 0x08]);
    } else {
        text(&mut b, 0x100e, &[0x41, 0xff, 0x24, 0xc2]);
    }
    // S2: movsxd rcx,[rip+selB]; lea rdx,[rip+tabB]; jmp [rdx+rcx*8]
    text(&mut b, 0x1020, &[&[0x48, 0x63, 0x0d][..], &rel(0x1027, 0x2004)].concat());
    text(&mut b, 0x1027, &[&[0x48, 0x8d, 0x15][..], &rel(0x102e, 0x1060)].concat());
    text(&mut b, 0x102e, &[0xff, 0x24, 0xca]);
    // table B (in .text): T2, T0, T2, then T3 without a relocation
    for (k, t) in [0x1120u64, 0x1100, 0x1120, 0x1130].into_iter().enumerate() {
        text(&mut b, 0x1060 + 8 * k as u32, &(IB + t).to_le_bytes());
    }
    // targets: mov eax, 0x500+k ; ret
    for k in 0..4u32 {
        text(&mut b, 0x1100 + 0x10 * k, &[0xb8, k as u8, 0x05, 0, 0, 0xc3]);
    }
    // main: call S1; call S2; [call S3;] ret | jmp S1.lea
    text(&mut b, 0x1200, &[&[0xe8][..], &rel(0x1205, 0x1000)].concat());
    text(&mut b, 0x1205, &[&[0xe8][..], &rel(0x120a, 0x1020)].concat());
    if v.jump_into_lea {
        text(&mut b, 0x120a, &[&[0xe9][..], &rel(0x120f, 0x1007)].concat());
    } else if v.s3 != S3::Absent {
        text(&mut b, 0x120a, &[&[0xe8][..], &rel(0x120f, 0x1290)].concat());
        text(&mut b, 0x120f, &[0xc3]);
    } else {
        text(&mut b, 0x120a, &[0xc3]);
    }
    if v.s3 != S3::Absent {
        // S3 - 16: call T0 (or the variant's wrong form) + nop word ptr [rax+rax] (11 bytes); S3: movsxd rax,[rip+selA]; lea r10,[rip+tabC]; jmp [r10+rax*8]
        let call = match v.s3 {
            S3::NotCall => [&[0xe9][..], &rel(0x1285, 0x1100)].concat(),
            S3::CallOutside => [&[0xe8][..], &rel(0x1285, 0x3000)].concat(),
            _ => [&[0xe8][..], &rel(0x1285, 0x1100)].concat(),
        };
        text(&mut b, 0x1280, &call);
        let nop: [u8; 11] = if v.s3 == S3::WrongNop { [0x90; 11] } else { [0x66, 0x66, 0x66, 0x0f, 0x1f, 0x84, 0, 0, 0, 0, 0] };
        text(&mut b, 0x1285, &nop);
        text(&mut b, 0x1290, &[&[0x48, 0x63, 0x05][..], &rel(0x1297, 0x2000)].concat());
        text(&mut b, 0x1297, &[&[0x4c, 0x8d, 0x15][..], &rel(0x129e, 0x2048)].concat());
        text(&mut b, 0x129e, &[0x41, 0xff, 0x24, 0xc2]);
    }
    // .data
    let d = 0x800;
    w32(&mut b, d, 0xffff_ffff);
    w32(&mut b, d + 4, 0xffff_ffff);
    w64(&mut b, d + 8, IB + 0x1200);
    for k in 0..4u64 {
        w64(&mut b, d + 0x10 + 8 * k as usize, IB + 0x1100 + 0x10 * k);
    }
    w64(&mut b, d + 0x30, IB + 0x2000);
    if v.s3 != S3::Absent {
        w64(&mut b, d + 0x40, if v.s3 == S3::WrongValue { IB + 0x1100 } else { S3_INIT });
        w64(&mut b, d + 0x48, IB + 0x1110);
        w64(&mut b, d + 0x50, IB + 0x1130);
        w64(&mut b, d + 0x58, IB + 0x2000);
    }
    // .reloc: page 0x1000 (table B's three slots + pad), page 0x2000
    let r = 0xa00;
    w32(&mut b, r, 0x1000);
    w32(&mut b, r + 4, 16);
    for (k, off) in [0x60u16, 0x68, 0x70, 0].into_iter().enumerate() {
        w16(&mut b, r + 8 + 2 * k, if off == 0 { 0 } else { 0xa000 | off });
    }
    let mut offs: Vec<u16> = [0x08u16, 0x10, 0x18, 0x20, 0x28, 0x30].into_iter().filter(|&x| !(v.unrelocated_a && x == 0x10)).collect();
    if v.s3 != S3::Absent {
        // slot -1 of table C, its two entries, and a relocated pointer to .data after them (not code: the run ends there)
        offs.extend([0x40u16, 0x48, 0x50, 0x58].into_iter().filter(|&x| !(v.s3 == S3::Unrelocated && x == 0x40)));
    }
    let n = (offs.len() + 1) & !1;
    w32(&mut b, r + 16, 0x2000);
    w32(&mut b, r + 20, 8 + 2 * n as u32);
    for (k, off) in offs.iter().enumerate() {
        w16(&mut b, r + 24 + 2 * k, 0xa000 | off);
    }
    w32(&mut b, o + 112 + 8 * 5, 0x3000);
    w32(&mut b, o + 116 + 8 * 5, 16 + 8 + 2 * n as u32);
    b
}

const S1_JMP: u64 = IB + 0x100e;
const S2_JMP: u64 = IB + 0x102e;

#[test]
fn stub_tables_are_read_from_the_relocations() {
    let img = build(Variant::default());
    let pe = Pe::parse(&img).unwrap();
    let a = analyze(&pe).unwrap();
    assert!(a.closure.jumptab_rejected.is_empty(), "{:?}", a.closure.jumptab_rejected);
    assert_eq!((a.closure.jumptabs, a.closure.stub_tables), (2, 2));
    assert_eq!(a.resolved[&S1_JMP], Resolved::Unresolved);
    assert_eq!(a.closure.indirect.by_kind["jmp:sel_table"], 2);
    assert_eq!(a.closure.indirect.by_kind.get("jmp:unresolved").copied().unwrap_or(0), 0);

    let t1 = &a.jumptabs[&S1_JMP];
    assert_eq!(t1.select, Select::Slot);
    assert_eq!(t1.origin, Origin::Stub { sel_var: IB + 0x2000, table: IB + 0x2010 });
    let want1: Vec<(u64, u64)> = (0..4u64).map(|k| (IB + 0x2010 + 8 * k, IB + 0x1100 + 0x10 * k)).collect();
    assert_eq!(t1.cases, want1, "4 entries: not the slot before (0x2008), not the data pointer after (0x2030)");

    let t2 = &a.jumptabs[&S2_JMP];
    assert_eq!(t2.origin, Origin::Stub { sel_var: IB + 0x2004, table: IB + 0x1060 });
    assert_eq!(
        t2.cases,
        vec![(IB + 0x1060, IB + 0x1120), (IB + 0x1068, IB + 0x1100), (IB + 0x1070, IB + 0x1120)],
        "3 entries: the slot after (0x1078) holds T3 but is not relocated"
    );

    // main, S1, S2 and T0..T3 (every table entry is a function of the closure)
    assert_eq!(a.closure.functions, 7);
    for k in 0..4u64 {
        assert!(a.functions.contains_key(&(IB + 0x1100 + 0x10 * k)), "T{k}");
    }
}

#[test]
fn a_table_without_a_relocated_first_slot_is_rejected() {
    let img = build(Variant { unrelocated_a: true, ..Default::default() });
    let a = analyze(&Pe::parse(&img).unwrap()).unwrap();
    assert_eq!(a.closure.jumptabs, 1);
    assert!(!a.jumptabs.contains_key(&S1_JMP));
    assert!(a.jumptabs.contains_key(&S2_JMP));
    assert_eq!(a.closure.jumptab_rejected.len(), 1, "{:?}", a.closure.jumptab_rejected);
    assert_eq!(a.closure.jumptab_rejected[0].0, S1_JMP);
    assert!(a.closure.jumptab_rejected[0].1.contains("no relocated code pointer"), "{:?}", a.closure.jumptab_rejected);
    assert_eq!(a.closure.indirect.by_kind["jmp:unresolved"], 1);
}

#[test]
fn a_jump_with_a_displacement_is_not_a_stub() {
    let img = build(Variant { disp: true, ..Default::default() });
    let a = analyze(&Pe::parse(&img).unwrap()).unwrap();
    assert_eq!(a.closure.jumptabs, 1);
    assert!(!a.jumptabs.contains_key(&S1_JMP));
    assert!(a.closure.jumptab_rejected.is_empty(), "not matched, so not rejected: {:?}", a.closure.jumptab_rejected);
    assert_eq!(a.closure.indirect.by_kind["jmp:unresolved"], 1);
}

#[test]
fn a_branch_into_the_middle_of_the_stub_rejects_it() {
    let img = build(Variant { jump_into_lea: true, ..Default::default() });
    let a = analyze(&Pe::parse(&img).unwrap()).unwrap();
    assert_eq!(a.closure.jumptabs, 1);
    assert_eq!(a.closure.stub_tables, 1);
    assert!(!a.jumptabs.contains_key(&S1_JMP));
    assert_eq!(a.closure.jumptab_rejected.len(), 1, "{:?}", a.closure.jumptab_rejected);
    assert!(a.closure.jumptab_rejected[0].1.contains("a block starts inside the stub"), "{:?}", a.closure.jumptab_rejected);
    assert_eq!(a.closure.indirect.by_kind["jmp:unresolved"], 1);
    assert_eq!(a.closure.indirect.by_kind["jmp:sel_table"], 1);
}

// ---- decision 0114: the lazy-initialisation slot (index -1) of a selector stub ---------------------------------------------------

fn s3_table(v: S3) -> (fe_decode::Analysis, Vec<(u64, u64)>) {
    let img = build(Variant { s3: v, ..Default::default() });
    let a = analyze(&Pe::parse(&img).unwrap()).unwrap();
    let cases = a.jumptabs.get(&S3_JMP).unwrap_or_else(|| panic!("no table at S3: {:?}", a.closure.jumptab_rejected)).cases.clone();
    (a, cases)
}

/// Slot -1 is a case when it is a relocated slot that holds S3 - 16, and the 16 bytes there are `call rel32` into code and the 11-byte nop.
#[test]
fn the_init_slot_is_a_case_when_every_condition_holds() {
    let (a, cases) = s3_table(S3::Valid);
    assert!(a.closure.jumptab_rejected.is_empty(), "{:?}", a.closure.jumptab_rejected);
    assert_eq!(a.closure.stub_tables, 3);
    assert_eq!(cases, vec![(IB + 0x2040, S3_INIT), (IB + 0x2048, IB + 0x1110), (IB + 0x2050, IB + 0x1130)], "slot -1, T1, T3; not the relocated pointer to data after them");
    assert_eq!(a.jumptabs[&S3_JMP].origin, Origin::Stub { sel_var: IB + 0x2000, table: IB + 0x2048 });
    // the init path is a function of the closure (S3 - 16 = `call T0; nop`, which falls into S3), next to main, S1, S2, S3, T0..T3
    assert!(a.functions.contains_key(&S3_INIT));
    assert!(a.functions.contains_key(&S3_START));
    assert_eq!(a.closure.functions, 9);
}

#[test]
fn the_init_slot_is_not_a_case_when_a_condition_fails() {
    for (v, what) in [
        (S3::Unrelocated, "slot -1 is not a DIR64 relocation slot"),
        (S3::WrongValue, "slot -1 holds T0, not S3 - 16"),
        (S3::NotCall, "the byte at S3 - 16 is E9, not E8"),
        (S3::WrongNop, "the 11 bytes after the call are not the nop"),
        (S3::CallOutside, "the call target is not in code"),
    ] {
        let (a, cases) = s3_table(v);
        assert_eq!(cases, vec![(IB + 0x2048, IB + 0x1110), (IB + 0x2050, IB + 0x1130)], "{what}: only the two table entries");
        assert!(a.closure.jumptab_rejected.is_empty(), "{what}: {:?}", a.closure.jumptab_rejected);
        assert!(!a.functions.contains_key(&S3_INIT), "{what}: S3 - 16 is not made a function");
    }
}

/// S1 and S2 of the default image keep slot -1 out: S1's slot -1 (0x2008) is relocated and holds main, not S1 - 16 (and S1 - 16 is before .text).
#[test]
fn a_relocated_code_pointer_in_front_of_the_table_that_is_not_the_init_path_stays_out() {
    let a = analyze(&Pe::parse(&build(Variant::default())).unwrap()).unwrap();
    assert_eq!(a.jumptabs[&S1_JMP].cases.len(), 4);
    assert_eq!(a.jumptabs[&S1_JMP].cases[0].0, IB + 0x2010);
}

fn read_if_present(env: &str, default: &str) -> Option<Vec<u8>> {
    let explicit = std::env::var_os(env);
    let path = match &explicit {
        Some(p) => PathBuf::from(p),
        None => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(default),
    };
    match std::fs::read(&path) {
        Ok(d) => Some(d),
        Err(e) if explicit.is_some() => panic!("{env}={}: {e}", path.display()),
        Err(e) => {
            eprintln!("SKIPPED: {}: {e}", path.display());
            None
        }
    }
}

#[test]
fn flengine_has_no_stub() {
    let Some(data) = read_if_present("FLENGINE_PE", "../../../binaries/win64/FLEngine_x64.dll") else { return };
    let pe = Pe::parse(&data).unwrap();
    let a = analyze_with_pdata_adjustors(&pe).unwrap();
    assert_eq!(a.closure.stub_tables, 0);
    assert_eq!(a.closure.jumptabs, 4, "only the four annotated tables of decision 0036");
    assert!(a.closure.jumptab_rejected.is_empty(), "{:?}", a.closure.jumptab_rejected);
}

#[test]
fn dsp_ippv2_has_434_stubs() {
    let Some(data) = read_if_present("DSP_IPPV2_PE", "../../out-ext/dsp_ippv2_x64.dll") else { return };
    let pe = Pe::parse(&data).unwrap();
    let a = analyze_with_pdata_adjustors(&pe).unwrap();
    assert!(a.closure.jumptab_rejected.is_empty(), "{:?}", a.closure.jumptab_rejected);
    assert_eq!(a.closure.stub_tables, 434);
    // 434 stubs + 8 MSVC switches with a proven base: 1 by decision 0089 (0x1817c69eb in the file's base, `f_1817c6990`), 4
    // more by the reaching definitions of decision 0094 (0x37b25bd, 0x37cc80b, 0x37d9670, 0x37d986a at the base 0x2000000),
    // 1 by the swapped form of decision 0095 (0x37cd3eb, `f_37cd2c0`) and 2 by the scheduled form of decision 0109 (0x37b5cea,
    // 0x37b6554; predicted by an independent scan, not yet run)
    assert_eq!(a.closure.msvc_tables, 8);
    assert_eq!(a.closure.jumptabs, 442);
    let mut by_var = std::collections::BTreeMap::new();
    let mut by_len = std::collections::BTreeMap::new();
    for t in a.jumptabs.values().filter(|t| !matches!(t.origin, Origin::Msvc { .. })) {
        let Origin::Stub { sel_var, .. } = t.origin else { panic!("annotated table in dsp_ippv2") };
        *by_var.entry(sel_var).or_insert(0usize) += 1;
        *by_len.entry(t.cases.len()).or_insert(0usize) += 1;
    }
    assert_eq!(by_var, [(0x1_8254_1000u64, 377), (0x1_8254_10a0, 20), (0x1_8254_f270, 4), (0x1_8255_d050, 33)].into_iter().collect());
    // decision 0114: every table has one more case than before, the index -1 (the stub's own `call <init>`): 6/7/9 entries became 7/8/10
    assert_eq!(by_len, [(7usize, 376), (8, 57), (10, 1)].into_iter().collect());
    // the other unresolved jumps stay unresolved (classified in decision 0079, not implemented). `jmp:unresolved`
    // also counts the `jmp reg` that fe-lift lifts as a tail dispatch (decision 0036, W1): 366 of the 549.
    let memory_form = a
        .resolved
        .iter()
        .filter(|(va, r)| {
            **r == Resolved::Unresolved
                && a.insns[*va].flow_control() == iced_x86::FlowControl::IndirectBranch
                && a.insns[*va].op_kind(0) == iced_x86::OpKind::Memory
        })
        .count();
    // 549 before decision 0089; the MSVC switches moved register-form jumps out of `jmp:unresolved` (1 by 0089, 4 by 0094, 1 by 0095)
    assert_eq!(a.closure.indirect.by_kind["jmp:unresolved"], 541);
    assert_eq!(a.closure.indirect.by_kind["jmp:msvc_table"], 8);
    assert_eq!(memory_form - a.closure.stub_tables, 183, "memory-form jumps still unresolved after the 434 stubs");
}
