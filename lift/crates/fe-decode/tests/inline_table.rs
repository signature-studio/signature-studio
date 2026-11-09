//! Inline absolute-pointer tables (decision 0102 W3, provisional): `lea B,[rip+k]` right before `jmp qword [B+I*8+d]`, the table of absolute
//! code pointers following the jump in the function's own code (the shape of the Delphi `FillChar` tail; FLEngine's 0x40d253 is the same
//! idiom, annotated by hand since decision 0036). The table is the run of DIR64-relocated slots from the `lea` target that point into the
//! function; the lifted jump compares the address the jump really computes (`Select::Slot`).
//!
//! Synthetic PE32+ (ImageBase 0x1_4000_0000):
//!
//! | section | RVA    | content |
//! |---------|--------|---------|
//! | .text   | 0x1000 | F at 0x1000: `lea r10,[rip+5]` (0x1000, 7 bytes), `jmp [r10+rdx*8+0x40]` (0x1007, 5 bytes), the table at 0x100c (9 slots); targets T_k at 0x1080+0x10k (`mov eax,k; ret`); G at 0x1200 (a second function, the entry) |
//! | .xdata  | 0x2000 | UNWIND_INFO (version 1, no codes) |
//! | .pdata  | 0x3000 | F = [0x1000, 0x1200), G = [0x1200, 0x1210) |
//! | .reloc  | 0x4000 | DIR64 for the table slots |
//!
//! The real files (skipped when absent): FLEngine_x64.dll has the idiom once, annotated, so `inline_tables` stays 0 and `jumptabs` 4; the
//! QuickFontCache_x64.dll of the installer (lift/out-qfc/, not in the repository) has it at 0x40c163.

use fe_decode::{analyze_with_pdata_adjustors as analyze, Origin, Select};
use fe_pe::Pe;
use std::path::PathBuf;

const IB: u64 = 0x1_4000_0000;
const LFANEW: usize = 0x80;
const OPT_SIZE: usize = 240;
const SEC_TABLE: usize = LFANEW + 24 + OPT_SIZE;
const FILE_LEN: usize = 0xe00;

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
    /// slot 0 is not in the relocation directory
    unrelocated_first: bool,
    /// slot 5 points at G (another function): the run ends there
    foreign_slot5: bool,
    /// slot 7 is not in the relocation directory
    unrelocated_slot7: bool,
    /// the `lea` loads an address outside F (0x1300) instead of the end of the jump
    table_outside: bool,
    /// `mov r10,rcx` instead of the `lea` (7 bytes: `mov r10,rcx` + nops)
    no_lea: bool,
    /// scale 4 instead of 8
    scale4: bool,
}

fn build(v: Variant) -> Vec<u8> {
    let mut b = vec![0u8; FILE_LEN];
    w16(&mut b, 0, 0x5a4d);
    w32(&mut b, 0x3c, LFANEW as u32);
    w32(&mut b, LFANEW, 0x4550);
    let f = LFANEW + 4;
    w16(&mut b, f, 0x8664);
    w16(&mut b, f + 2, 4);
    w16(&mut b, f + 16, OPT_SIZE as u16);
    w16(&mut b, f + 18, 0x2022);
    let o = LFANEW + 24;
    w16(&mut b, o, 0x20b);
    w32(&mut b, o + 16, 0x1200);
    w64(&mut b, o + 24, IB);
    w32(&mut b, o + 32, 0x1000);
    w32(&mut b, o + 36, 0x200);
    w32(&mut b, o + 56, 0x5000);
    w32(&mut b, o + 60, 0x400);
    w16(&mut b, o + 68, 3);
    w32(&mut b, o + 108, 16);
    let section = |b: &mut Vec<u8>, i: usize, name: &[u8], rva: u32, size: u32, raw: u32, flags: u32| {
        let s = SEC_TABLE + 40 * i;
        b[s..s + name.len()].copy_from_slice(name);
        w32(b, s + 8, size);
        w32(b, s + 12, rva);
        w32(b, s + 16, size);
        w32(b, s + 20, raw);
        w32(b, s + 36, flags);
    };
    section(&mut b, 0, b".text", 0x1000, 0x400, 0x400, 0x6000_0020);
    section(&mut b, 1, b".xdata", 0x2000, 0x200, 0x800, 0x4000_0040);
    section(&mut b, 2, b".pdata", 0x3000, 0x200, 0xa00, 0x4000_0040);
    section(&mut b, 3, b".reloc", 0x4000, 0x200, 0xc00, 0x4200_0040);
    let text = |b: &mut Vec<u8>, rva: u32, bytes: &[u8]| {
        let at = 0x400 + (rva - 0x1000) as usize;
        b[at..at + bytes.len()].copy_from_slice(bytes);
    };
    // F
    let lea_target = if v.table_outside { 0x1300 } else { 0x100c };
    if v.no_lea {
        text(&mut b, 0x1000, &[0x49, 0x89, 0xca, 0x90, 0x90, 0x90, 0x90]); // mov r10,rcx; nop x4
    } else {
        text(&mut b, 0x1000, &[&[0x4c, 0x8d, 0x15][..], &rel(0x1007, lea_target)].concat());
    }
    if v.scale4 {
        text(&mut b, 0x1007, &[0x41, 0xff, 0x64, 0x92, 0x40]); // jmp [r10+rdx*4+0x40]
    } else {
        text(&mut b, 0x1007, &[0x41, 0xff, 0x64, 0xd2, 0x40]); // jmp [r10+rdx*8+0x40]
    }
    // the table: 9 slots, the tenth (0x1054) is not a pointer into F
    for k in 0..9u32 {
        let t = if v.foreign_slot5 && k == 5 { 0x1200 } else { 0x1080 + 0x10 * k };
        text(&mut b, 0x100c + 8 * k, &(IB + t as u64).to_le_bytes());
    }
    text(&mut b, 0x1054, &[0x90; 8]);
    for k in 0..9u32 {
        text(&mut b, 0x1080 + 0x10 * k, &[0xb8, k as u8, 0, 0, 0, 0xc3]); // mov eax,k; ret
    }
    // G
    text(&mut b, 0x1200, &[0xc3]);
    // .xdata
    b[0x800] = 1;
    // .pdata: F, G
    for (k, (begin, end)) in [(0x1000u32, 0x1200u32), (0x1200, 0x1210)].into_iter().enumerate() {
        w32(&mut b, 0xa00 + 12 * k, begin);
        w32(&mut b, 0xa00 + 12 * k + 4, end);
        w32(&mut b, 0xa00 + 12 * k + 8, 0x2000);
    }
    w32(&mut b, o + 112 + 8 * 3, 0x3000);
    w32(&mut b, o + 116 + 8 * 3, 24);
    // .reloc: page 0x1000, DIR64 for the slots (offsets 0x0c + 8k)
    let offs: Vec<u16> = (0..9u16)
        .filter(|&k| !(v.unrelocated_first && k == 0) && !(v.unrelocated_slot7 && k == 7))
        .map(|k| 0x0c + 8 * k)
        .collect();
    let n = (offs.len() + 1) & !1;
    w32(&mut b, 0xc00, 0x1000);
    w32(&mut b, 0xc04, 8 + 2 * n as u32);
    for (k, off) in offs.iter().enumerate() {
        w16(&mut b, 0xc08 + 2 * k, 0xa000 | off);
    }
    w32(&mut b, o + 112 + 8 * 5, 0x4000);
    w32(&mut b, o + 116 + 8 * 5, 8 + 2 * n as u32);
    b
}

const JMP: u64 = IB + 0x1007;

#[test]
fn the_table_is_the_run_of_relocated_pointers_into_the_function() {
    let img = build(Variant::default());
    let pe = Pe::parse(&img).unwrap();
    let a = analyze(&pe).unwrap();
    assert_eq!((a.closure.jumptabs, a.closure.inline_tables, a.closure.stub_tables, a.closure.msvc_tables), (1, 1, 0, 0), "{:?}", a.closure.jumptab_rejected);
    let t = &a.jumptabs[&JMP];
    assert_eq!(t.select, Select::Slot);
    assert_eq!(t.origin, Origin::Inline { table: IB + 0x100c });
    let want: Vec<(u64, u64)> = (0..9u64).map(|k| (IB + 0x100c + 8 * k, IB + 0x1080 + 0x10 * k)).collect();
    assert_eq!(t.cases, want);
    // every target is a decoded block of the closure, and the jump is no longer counted as unresolved
    for (_, tg) in &t.cases {
        assert!(a.insns.contains_key(tg), "{tg:#x}");
    }
    assert_eq!(a.closure.indirect.by_kind.get("jmp:inline_table"), Some(&1), "{:?}", a.closure.indirect.by_kind);
    assert_eq!(a.closure.indirect.by_kind.get("jmp:unresolved").copied().unwrap_or(0), 0, "{:?}", a.closure.indirect.by_kind);
    assert!(a.closure.jumptab_rejected.is_empty());
}

#[test]
fn the_run_ends_at_a_slot_that_is_not_relocated_or_points_out_of_the_function() {
    // a pointer to another function ends the run at 5 slots
    let img = build(Variant { foreign_slot5: true, ..Default::default() });
    let a = analyze(&Pe::parse(&img).unwrap()).unwrap();
    assert_eq!(a.jumptabs[&JMP].cases.len(), 5, "{:?}", a.closure.jumptab_rejected);
    // a slot without a relocation ends it at 7
    let img = build(Variant { unrelocated_slot7: true, ..Default::default() });
    let a = analyze(&Pe::parse(&img).unwrap()).unwrap();
    assert_eq!(a.jumptabs[&JMP].cases.len(), 7);
}

#[test]
fn a_table_without_a_relocated_first_slot_or_outside_the_function_is_not_recognised_and_nothing_is_reported() {
    for (label, v) in [
        ("first slot", Variant { unrelocated_first: true, ..Default::default() }),
        ("outside", Variant { table_outside: true, ..Default::default() }),
    ] {
        let img = build(v);
        let a = analyze(&Pe::parse(&img).unwrap()).unwrap();
        assert_eq!((a.closure.jumptabs, a.closure.inline_tables), (0, 0), "{label}");
        assert!(a.jumptabs.is_empty(), "{label}");
        // `lea B,[rip+k]; jmp [B+I*8+d]` is a common shape that is simply not this form: the other tests of the workspace assert that
        // `jumptab_rejected` stays empty for the real files, so nothing is reported here either
        assert!(a.closure.jumptab_rejected.is_empty(), "{label}: {:?}", a.closure.jumptab_rejected);
    }
}

#[test]
fn other_shapes_are_not_recognised() {
    // no `lea` right before the jump; scale 4
    for (label, v) in [("no lea", Variant { no_lea: true, ..Default::default() }), ("scale 4", Variant { scale4: true, ..Default::default() })] {
        let img = build(v);
        let a = analyze(&Pe::parse(&img).unwrap()).unwrap();
        assert_eq!((a.closure.jumptabs, a.closure.inline_tables), (0, 0), "{label}");
        assert!(a.closure.jumptab_rejected.is_empty(), "{label}: {:?}", a.closure.jumptab_rejected);
        assert!(a.jumptabs.is_empty(), "{label}");
    }
}

fn real(env: &str, default: &str) -> Option<Vec<u8>> {
    let p = std::env::var_os(env).map(PathBuf::from).unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(default));
    match std::fs::read(&p) {
        Ok(d) => Some(d),
        Err(e) => {
            eprintln!("SKIPPED: {}: {e}", p.display());
            None
        }
    }
}

#[test]
fn flengine_has_the_idiom_once_and_it_stays_annotated() {
    let Some(data) = real("FLENGINE_PE", "../../../binaries/win64/FLEngine_x64.dll") else { return };
    let pe = Pe::parse(&data).unwrap();
    let a = analyze(&pe).unwrap();
    assert_eq!((a.closure.jumptabs, a.closure.inline_tables), (4, 0), "{:?}", a.closure.jumptab_rejected);
    assert_eq!(a.jumptabs[&0x40d253].origin, Origin::Annotated);
}

#[test]
fn quickfontcache_has_the_idiom_at_0x40c163() {
    let Some(data) = real("QFC_PE", "../../out-qfc/quickfontcache_x64.dll") else { return };
    let pe = Pe::parse(&data).unwrap();
    let a = analyze(&pe).unwrap();
    // the table follows the jump (0x40c168) with 9 slots (index -8..0), as read from objdump: 0x40c168..0x40c1a8
    let t = &a.jumptabs[&0x40c163];
    assert_eq!(t.origin, Origin::Inline { table: 0x40c168 });
    assert_eq!(t.cases.len(), 9, "{:?}", t.cases);
    assert_eq!(t.cases[0].0, 0x40c168);
    assert!(a.closure.inline_tables >= 1);
}
