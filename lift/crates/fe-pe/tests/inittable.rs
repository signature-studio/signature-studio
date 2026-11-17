//! Tests of the A4 unit table reader (src/inittable.rs, docs/investigations/a4-survey-tool.md, decision 0060).
//!
//! 1. A synthetic PE with a table of 6 units that covers every kind (runnable, runnable with a repeated proc, empty in the file,
//!    empty in .bss, noproc, outside the image) and the entry's `lea rdx,[rip+disp]`.
//! 2. The real FLEngine_x64.dll ($FLENGINE_PE, else <repo root>/binaries/win64/FLEngine_x64.dll; with FLENGINE_PE set and the file
//!    missing the tests fail, with the default path missing they print SKIPPED and pass). The expected numbers were measured
//!    from the DLL bytes by an independent Python reader (942 / 895 / 46 / 1), not read back from this crate.

use fe_pe::inittable::{UnitKind, INIT_COUNT_VA, INIT_CONTEXT_VA, INIT_TABLE_HEADER_VA};
use fe_pe::{Pe, PeError};
use std::path::PathBuf;

const LFANEW: usize = 0x80;
const OPT_SIZE: usize = 240;
const NSEC: usize = 2;
const IMAGE_BASE: u64 = 0x1_4000_0000;
const TEXT_RAW: usize = 0x400;
const TEXT_RVA: u32 = 0x1000;
const TEXT_LEN: usize = 0x800;
const FILE_LEN: usize = TEXT_RAW + TEXT_LEN;

fn w16(b: &mut [u8], o: usize, v: u16) {
    b[o..o + 2].copy_from_slice(&v.to_le_bytes());
}
fn w32(b: &mut [u8], o: usize, v: u32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}
fn w64(b: &mut [u8], o: usize, v: u64) {
    b[o..o + 8].copy_from_slice(&v.to_le_bytes());
}
fn at(rva: u32) -> usize {
    TEXT_RAW + (rva - TEXT_RVA) as usize
}

/// .text (RVA 0x1000, 0x800 bytes, file-backed) and .bss (RVA 0x2000, 0x1000 virtual, no file bytes). Entry RVA 0x1000.
/// `count` and `units_rva` go into the header at RVA 0x1100; `lea_rva` is where the entry's `lea rdx` points.
fn build(count: u32, units_rva: u64, lea_target_rva: u32) -> Vec<u8> {
    let mut b = vec![0u8; FILE_LEN];
    w16(&mut b, 0, 0x5a4d);
    w32(&mut b, 0x3c, LFANEW as u32);
    w32(&mut b, LFANEW, 0x4550);
    let f = LFANEW + 4;
    w16(&mut b, f, 0x8664);
    w16(&mut b, f + 2, NSEC as u16);
    w16(&mut b, f + 16, OPT_SIZE as u16);
    w16(&mut b, f + 18, 0x2022);
    let o = LFANEW + 24;
    w16(&mut b, o, 0x20b);
    w32(&mut b, o + 16, 0x1000);
    w32(&mut b, o + 20, 0x1000);
    w64(&mut b, o + 24, IMAGE_BASE);
    w32(&mut b, o + 32, 0x1000);
    w32(&mut b, o + 36, 0x200);
    w32(&mut b, o + 56, 0x3000);
    w32(&mut b, o + 60, 0x400);
    w16(&mut b, o + 68, 2);
    w16(&mut b, o + 70, 0x160);
    w32(&mut b, o + 108, 16);
    let s0 = o + OPT_SIZE;
    b[s0..s0 + 5].copy_from_slice(b".text");
    w32(&mut b, s0 + 8, TEXT_LEN as u32);
    w32(&mut b, s0 + 12, TEXT_RVA);
    w32(&mut b, s0 + 16, TEXT_LEN as u32);
    w32(&mut b, s0 + 20, TEXT_RAW as u32);
    w32(&mut b, s0 + 36, 0x6000_0020);
    let s1 = s0 + 40;
    b[s1..s1 + 4].copy_from_slice(b".bss");
    w32(&mut b, s1 + 8, 0x1000);
    w32(&mut b, s1 + 12, 0x2000);
    w32(&mut b, s1 + 36, 0xc000_0000);

    // entry: nop; nop; lea rdx,[rip+disp32]; ret
    let e = at(0x1000);
    b[e] = 0x90;
    b[e + 1] = 0x90;
    b[e + 2..e + 5].copy_from_slice(&[0x48, 0x8d, 0x15]);
    let disp = lea_target_rva as i64 - (0x1002 + 7);
    w32(&mut b, e + 5, disp as i32 as u32);
    b[e + 9] = 0xc3;

    // header at RVA 0x1100
    w32(&mut b, at(0x1100), count);
    w64(&mut b, at(0x1108), units_rva);

    // units at RVA 0x1120
    let u = |i: u32| at(0x1120 + 16 * i);
    let va = |rva: u64| IMAGE_BASE + rva;
    w64(&mut b, u(0), va(0x1200));
    w64(&mut b, u(0) + 8, va(0x1300));
    w64(&mut b, u(1), va(0x1240));
    w64(&mut b, u(1) + 8, va(0x1301));
    w64(&mut b, u(2), 0);
    w64(&mut b, u(2) + 8, va(0x1302));
    w64(&mut b, u(3), va(0x1200));
    w64(&mut b, u(3) + 8, va(0x1303));
    w64(&mut b, u(4), va(0x2010));
    w64(&mut b, u(4) + 8, 0);
    w64(&mut b, u(5), 0x5000_0000);
    w64(&mut b, u(5) + 8, 0);
    // proc 0x1200 starts with non-zero bytes, proc 0x1240 with zeros
    w64(&mut b, at(0x1200), 0x0123_4567_89ab_cdef);
    b
}

fn good() -> Vec<u8> {
    build(6, IMAGE_BASE + 0x1120, 0x1100)
}

#[test]
fn every_kind_is_classified() {
    let img = good();
    let pe = Pe::parse(&img).unwrap();
    let t = pe.init_table(IMAGE_BASE + 0x1100).unwrap();
    assert_eq!(t.unit_count, 6);
    assert_eq!(t.units_va, IMAGE_BASE + 0x1120);
    let kinds: Vec<UnitKind> = t.units.iter().map(|u| u.kind).collect();
    assert_eq!(
        kinds,
        [
            UnitKind::Runnable,
            UnitKind::Empty,
            UnitKind::NoProc,
            UnitKind::Runnable,
            UnitKind::Empty,
            UnitKind::Unreadable
        ]
    );
    assert_eq!(t.count(UnitKind::Runnable), 2);
    assert_eq!(t.count(UnitKind::Empty), 2);
    assert_eq!(t.count(UnitKind::NoProc), 1);
    assert_eq!(t.count(UnitKind::Unreadable), 1);
    assert_eq!(t.distinct_procs(UnitKind::Runnable), 1, "units 0 and 3 share a proc");
    assert_eq!(t.units[0].proc_va, IMAGE_BASE + 0x1200);
    assert_eq!(t.units[0].second_va, IMAGE_BASE + 0x1300);
    assert_eq!(t.units[4].proc_va, IMAGE_BASE + 0x2010, ".bss is read as zero");
    assert_eq!(t.units[5].proc_va, 0x5000_0000);
}

#[test]
fn tsv_has_the_run_rank_of_runnable_units_only() {
    let img = good();
    let pe = Pe::parse(&img).unwrap();
    let t = pe.init_table(IMAGE_BASE + 0x1100).unwrap();
    let want = format!(
        "index\tproc_va\tkind\tsecond_va\trun_rank\n\
         0\t{a:#x}\trunnable\t{s0:#x}\t1\n\
         1\t{b:#x}\tempty\t{s1:#x}\t-\n\
         2\t0x0\tnoproc\t{s2:#x}\t-\n\
         3\t{a:#x}\trunnable\t{s3:#x}\t2\n\
         4\t{c:#x}\tempty\t0x0\t-\n\
         5\t0x50000000\tunreadable\t0x0\t-\n",
        a = IMAGE_BASE + 0x1200,
        b = IMAGE_BASE + 0x1240,
        c = IMAGE_BASE + 0x2010,
        s0 = IMAGE_BASE + 0x1300,
        s1 = IMAGE_BASE + 0x1301,
        s2 = IMAGE_BASE + 0x1302,
        s3 = IMAGE_BASE + 0x1303,
    );
    assert_eq!(t.to_tsv(), want);
}

#[test]
fn entry_lea_gives_the_header() {
    let img = good();
    let pe = Pe::parse(&img).unwrap();
    assert_eq!(pe.entry_header_reference(), Some((IMAGE_BASE + 0x1002, IMAGE_BASE + 0x1100)));
    // a backward displacement too (target below the lea)
    let img2 = build(6, IMAGE_BASE + 0x1120, 0x1000);
    let pe2 = Pe::parse(&img2).unwrap();
    assert_eq!(pe2.entry_header_reference(), Some((IMAGE_BASE + 0x1002, IMAGE_BASE + 0x1000)));
}

#[test]
fn entry_without_lea_has_no_header_reference() {
    let mut img = good();
    img[at(0x1002)] = 0x90; // the 48 8D 15 pattern is gone
    let pe = Pe::parse(&img).unwrap();
    assert_eq!(pe.entry_header_reference(), None);
}

#[test]
fn zero_units_is_an_empty_table() {
    let img = build(0, IMAGE_BASE + 0x1120, 0x1100);
    let pe = Pe::parse(&img).unwrap();
    let t = pe.init_table(IMAGE_BASE + 0x1100).unwrap();
    assert!(t.units.is_empty());
    assert_eq!(t.to_tsv(), "index\tproc_va\tkind\tsecond_va\trun_rank\n");
}

#[test]
fn a_huge_unit_count_is_refused() {
    let img = build(0x10001, IMAGE_BASE + 0x1120, 0x1100);
    let pe = Pe::parse(&img).unwrap();
    assert!(matches!(pe.init_table(IMAGE_BASE + 0x1100), Err(PeError::LimitExceeded(_))));
    // the limit itself passes the count check and then fails on the table running past the section
    let img = build(0x10000, IMAGE_BASE + 0x1120, 0x1100);
    let pe = Pe::parse(&img).unwrap();
    assert!(!matches!(pe.init_table(IMAGE_BASE + 0x1100), Err(PeError::LimitExceeded(_))));
}

#[test]
fn a_table_that_runs_past_the_image_is_an_error_not_a_panic() {
    let img = build(6, IMAGE_BASE + 0x2fc0, 0x1100); // 6 * 16 bytes from 0x2fc0 end past 0x3000 / are not file-backed
    let pe = Pe::parse(&img).unwrap();
    assert!(pe.init_table(IMAGE_BASE + 0x1100).is_err());
    let img = build(6, 0x10, 0x1100); // units pointer below the image base
    let pe = Pe::parse(&img).unwrap();
    assert!(pe.init_table(IMAGE_BASE + 0x1100).is_err());
    let pe = Pe::parse(&img).unwrap();
    assert!(pe.init_table(IMAGE_BASE + 0x9000).is_err(), "header outside the image");
}

#[test]
fn the_fixed_addresses_are_the_ones_of_the_dll() {
    assert_eq!(INIT_CONTEXT_VA, 0x15EAC90);
    assert_eq!(INIT_COUNT_VA, INIT_CONTEXT_VA + 8);
    assert_eq!(INIT_TABLE_HEADER_VA, 0x13768B0);
}

fn load() -> Option<Vec<u8>> {
    let explicit = std::env::var_os("FLENGINE_PE");
    let path = match &explicit {
        Some(p) => PathBuf::from(p),
        None => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../binaries/win64/FLEngine_x64.dll"),
    };
    match std::fs::read(&path) {
        Ok(d) => Some(d),
        Err(e) if explicit.is_some() => panic!("FLENGINE_PE={}: {e}", path.display()),
        Err(e) => {
            eprintln!("SKIPPED: {}: {e}", path.display());
            None
        }
    }
}

#[test]
fn real_dll_counts() {
    let Some(d) = load() else { return };
    let pe = Pe::parse(&d).unwrap();
    assert_eq!(pe.entry_header_reference(), Some((0x1376858, 0x13768b0)), "DllMain's lea rdx at 0x1376858");
    let t = pe.init_table(0x13768b0).unwrap();
    assert_eq!(t.unit_count, 942);
    assert_eq!(t.units_va, 0x13768e0);
    assert_eq!(t.units.len(), 942);
    assert_eq!(t.count(UnitKind::Runnable), 895);
    assert_eq!(t.count(UnitKind::Empty), 46);
    assert_eq!(t.count(UnitKind::NoProc), 1);
    assert_eq!(t.count(UnitKind::Unreadable), 0);
    assert_eq!(t.distinct_procs(UnitKind::Runnable), 895);
}

#[test]
fn real_dll_known_units() {
    let Some(d) = load() else { return };
    let pe = Pe::parse(&d).unwrap();
    let t = pe.init_table(0x13768b0).unwrap();
    let u = |i: usize| (t.units[i].proc_va, t.units[i].kind);
    assert_eq!(u(0), (0x41e7e0, UnitKind::Runnable));
    assert_eq!(u(1), (0x41ce10, UnitKind::Runnable));
    assert_eq!(u(17), (0x455790, UnitKind::Runnable), "the unit that the decision 0046 run stopped in");
    assert_eq!(u(29), (0x13a9390, UnitKind::Empty));
    assert_eq!(u(114), (0x623840, UnitKind::Runnable), "the unit in progress at the LoadCursorW stop");
    assert_eq!(u(941), (0, UnitKind::NoProc));
    assert!(t.units.iter().filter(|u| u.kind == UnitKind::Empty).all(|u| u.proc_va == 0x13a9390), "all 46 empty units share one proc");
    // run_rank of unit 114: 111th runnable unit (29, 38, 39, 105 are empty before it: 114 + 1 - 4)
    let tsv = t.to_tsv();
    let line = tsv.lines().find(|l| l.starts_with("114\t")).unwrap();
    assert_eq!(line, "114\t0x623840\trunnable\t0x6237d0\t111");
    assert_eq!(tsv.lines().count(), 943);
}
