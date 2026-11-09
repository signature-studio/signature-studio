//! Tests of the exception directory parser (Pe::runtime_functions, unwind_info, function_table,
//! function_starts, function_start_vas).
//!
//! Format source: "x64 exception handling" (Microsoft Learn,
//! https://learn.microsoft.com/en-us/cpp/build/exception-handling-x64): RUNTIME_FUNCTION is three
//! ULONG image-relative addresses (begin, end, unwind info); UNWIND_INFO has flags in the top 5 bits of
//! byte 0, the count of unwind codes in byte 2, an array of 2-byte codes padded to an even count, then
//! either the exception handler RVA or, with UNW_FLAG_CHAININFO, the parent RUNTIME_FUNCTION.
//! Part 1 builds a PE by hand. Part 2 checks the real FLEngine_x64.dll ($FLENGINE_PE, else
//! <repo root>/binaries/win64/FLEngine_x64.dll; same skip rules as real_dll.rs). The real-DLL numbers
//! were measured with an independent Python parser (lift/out-pdata/pdata_probe.py).

use fe_pe::{FunctionEntry, Pe, PeError, RuntimeFunction};
use std::path::PathBuf;

const LFANEW: usize = 0x80;
const OPT_SIZE: usize = 240;
const IMAGE_BASE: u64 = 0x40_0000;
const XDATA_RVA: u32 = 0x2000;
const XDATA_RAW: usize = 0x400;
const XDATA_LEN: usize = 0x200;
const PDATA_RVA: u32 = 0x3000;
const PDATA_RAW: usize = 0x600;
const PDATA_LEN: usize = 0x200;
const FILE_LEN: usize = PDATA_RAW + PDATA_LEN;

fn w16(b: &mut [u8], o: usize, v: u16) {
    b[o..o + 2].copy_from_slice(&v.to_le_bytes());
}
fn w32(b: &mut [u8], o: usize, v: u32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}
fn w64(b: &mut [u8], o: usize, v: u64) {
    b[o..o + 8].copy_from_slice(&v.to_le_bytes());
}
fn xd(rva: u32) -> usize {
    XDATA_RAW + (rva - XDATA_RVA) as usize
}

/// UNWIND_INFO header at `rva`: version 1, `flags`, prolog size 4, `count` codes (zero-filled).
/// Returns the file offset of the tail (after the codes, padded to an even count).
fn unwind(b: &mut [u8], rva: u32, flags: u8, count: u8) -> usize {
    let o = xd(rva);
    b[o] = 1 | (flags << 3);
    b[o + 1] = 4;
    b[o + 2] = count;
    b[o + 3] = 0;
    o + 4 + 2 * ((count as usize + 1) & !1)
}

fn rf(b: &mut [u8], at: usize, begin: u32, end: u32, unwind_rva: u32) {
    w32(b, at, begin);
    w32(b, at + 4, end);
    w32(b, at + 8, unwind_rva);
}

/// Sections: .xdata (RVA 0x2000) and .pdata (RVA 0x3000). Exception directory = .pdata, `n` entries.
///
/// xdata layout:
///   0x2000  primary, no handler, 0 codes
///   0x2010  primary, EHANDLER, 3 codes (padded to 4), handler RVA 0x1234
///   0x2030  primary, UHANDLER|EHANDLER, 1 code, handler RVA 0x4321
///   0x2040  chained (2 codes), parent = RF(0x1100, 0x1180, 0x2000)
///   0x2060  chained (0 codes), parent = RF(0x1100, 0x1180, 0x2000)
///   0x2070  chained (1 code), parent = RF(0x1200, 0x1240, 0x2090)   [multiple shrink-wrapping]
///   0x2090  chained (0 codes), parent = RF(0x1100, 0x1180, 0x2000)
///   0x20c0  chained (0 codes), parent = RF(0x1300, 0x1340, 0x20c0)  [cycle]
fn build(n: usize, patch: impl FnOnce(&mut [u8])) -> Vec<u8> {
    let mut b = vec![0u8; FILE_LEN];
    w16(&mut b, 0, 0x5a4d);
    w32(&mut b, 0x3c, LFANEW as u32);
    w32(&mut b, LFANEW, 0x4550);
    let f = LFANEW + 4;
    w16(&mut b, f, 0x8664);
    w16(&mut b, f + 2, 2);
    w16(&mut b, f + 16, OPT_SIZE as u16);
    w16(&mut b, f + 18, 0x2022);
    let o = LFANEW + 24;
    w16(&mut b, o, 0x20b);
    w64(&mut b, o + 24, IMAGE_BASE);
    w32(&mut b, o + 32, 0x1000);
    w32(&mut b, o + 36, 0x200);
    w32(&mut b, o + 56, 0x4000);
    w32(&mut b, o + 60, 0x400);
    w16(&mut b, o + 68, 2);
    w32(&mut b, o + 108, 16);
    w32(&mut b, o + 112 + 8 * 3, PDATA_RVA);
    w32(&mut b, o + 116 + 8 * 3, (12 * n) as u32);
    let s0 = o + OPT_SIZE;
    b[s0..s0 + 6].copy_from_slice(b".xdata");
    w32(&mut b, s0 + 8, XDATA_LEN as u32);
    w32(&mut b, s0 + 12, XDATA_RVA);
    w32(&mut b, s0 + 16, XDATA_LEN as u32);
    w32(&mut b, s0 + 20, XDATA_RAW as u32);
    w32(&mut b, s0 + 36, 0x4000_0040);
    let s1 = s0 + 40;
    b[s1..s1 + 6].copy_from_slice(b".pdata");
    w32(&mut b, s1 + 8, PDATA_LEN as u32);
    w32(&mut b, s1 + 12, PDATA_RVA);
    w32(&mut b, s1 + 16, PDATA_LEN as u32);
    w32(&mut b, s1 + 20, PDATA_RAW as u32);
    w32(&mut b, s1 + 36, 0x4000_0040);

    unwind(&mut b, 0x2000, 0, 0);
    let t = unwind(&mut b, 0x2010, 1, 3);
    w32(&mut b, t, 0x1234);
    let t = unwind(&mut b, 0x2030, 3, 1);
    w32(&mut b, t, 0x4321);
    let t = unwind(&mut b, 0x2040, 4, 2);
    rf(&mut b, t, 0x1100, 0x1180, 0x2000);
    let t = unwind(&mut b, 0x2060, 4, 0);
    rf(&mut b, t, 0x1100, 0x1180, 0x2000);
    let t = unwind(&mut b, 0x2070, 4, 1);
    rf(&mut b, t, 0x1200, 0x1240, 0x2090);
    let t = unwind(&mut b, 0x2090, 4, 0);
    rf(&mut b, t, 0x1100, 0x1180, 0x2000);
    let t = unwind(&mut b, 0x20c0, 4, 0);
    rf(&mut b, t, 0x1300, 0x1340, 0x20c0);
    patch(&mut b);
    b
}

fn set_pdata(b: &mut [u8], entries: &[(u32, u32, u32)]) {
    for (i, e) in entries.iter().enumerate() {
        rf(b, PDATA_RAW + 12 * i, e.0, e.1, e.2);
    }
}

fn r(begin: u32, end: u32, unwind: u32) -> RuntimeFunction {
    RuntimeFunction { begin_rva: begin, end_rva: end, unwind_rva: unwind }
}

#[test]
fn runtime_functions_are_read_in_file_order() {
    let img = build(3, |b| set_pdata(b, &[(0x1200, 0x1240, 0x2000), (0x1000, 0x1010, 0x2010), (0x1100, 0x1180, 0x2000)]));
    let pe = Pe::parse(&img).unwrap();
    assert_eq!(
        pe.runtime_functions().unwrap(),
        vec![r(0x1200, 0x1240, 0x2000), r(0x1000, 0x1010, 0x2010), r(0x1100, 0x1180, 0x2000)]
    );
}

#[test]
fn no_exception_directory_is_empty() {
    let mut img = build(0, |_| {});
    let o = LFANEW + 24;
    w32(&mut img, o + 112 + 8 * 3, 0);
    let pe = Pe::parse(&img).unwrap();
    assert!(pe.runtime_functions().unwrap().is_empty());
    assert!(pe.function_starts().unwrap().is_empty());
}

#[test]
fn directory_size_must_be_a_multiple_of_12() {
    let mut img = build(2, |b| set_pdata(b, &[(0x1000, 0x1010, 0x2000), (0x1100, 0x1180, 0x2000)]));
    w32(&mut img, LFANEW + 24 + 116 + 8 * 3, 25);
    let pe = Pe::parse(&img).unwrap();
    assert!(matches!(pe.runtime_functions(), Err(PeError::Malformed(_))));
}

#[test]
fn directory_outside_the_file_is_an_error() {
    let mut img = build(2, |_| {});
    w32(&mut img, LFANEW + 24 + 116 + 8 * 3, 12 * 100);
    let pe = Pe::parse(&img).unwrap();
    assert!(matches!(pe.runtime_functions(), Err(PeError::Truncated { .. })));
}

#[test]
fn unwind_info_primary_without_handler() {
    let img = build(0, |_| {});
    let pe = Pe::parse(&img).unwrap();
    let u = pe.unwind_info(0x2000).unwrap();
    assert_eq!((u.version, u.flags, u.size_of_prolog, u.count_of_codes), (1, 0, 4, 0));
    assert_eq!((u.handler_rva, u.chained), (None, None));
}

#[test]
fn unwind_info_handler_follows_the_padded_code_array() {
    let img = build(0, |_| {});
    let pe = Pe::parse(&img).unwrap();
    // 3 codes are padded to 4 slots, so the handler is at +4 +8
    let u = pe.unwind_info(0x2010).unwrap();
    assert_eq!((u.flags, u.count_of_codes), (1, 3));
    assert_eq!(u.handler_rva, Some(0x1234));
    assert_eq!(u.chained, None);
    // 1 code is padded to 2 slots
    let u = pe.unwind_info(0x2030).unwrap();
    assert_eq!(u.flags, 3);
    assert_eq!(u.handler_rva, Some(0x4321));
}

#[test]
fn unwind_info_chained_reads_the_parent_entry() {
    let img = build(0, |_| {});
    let pe = Pe::parse(&img).unwrap();
    let u = pe.unwind_info(0x2040).unwrap();
    assert_eq!((u.flags, u.count_of_codes), (4, 2));
    assert_eq!(u.chained, Some(r(0x1100, 0x1180, 0x2000)));
    assert_eq!(u.handler_rva, None);
    // zero codes: the parent entry follows the 4-byte header directly
    let u = pe.unwind_info(0x2060).unwrap();
    assert_eq!(u.chained, Some(r(0x1100, 0x1180, 0x2000)));
}

#[test]
fn unwind_info_rejects_chain_with_handler_flag() {
    let img = build(0, |b| b[xd(0x2060)] = 1 | ((4 | 1) << 3));
    let pe = Pe::parse(&img).unwrap();
    assert!(matches!(pe.unwind_info(0x2060), Err(PeError::Malformed(_))));
}

#[test]
fn unwind_info_rejects_odd_address_and_unmapped_address() {
    let img = build(0, |_| {});
    let pe = Pe::parse(&img).unwrap();
    assert!(matches!(pe.unwind_info(0x2001), Err(PeError::Malformed(_))));
    assert!(pe.unwind_info(0x9000).is_err());
}

#[test]
fn chained_entries_are_not_function_starts() {
    // 0x1000 primary; 0x1100 is a chained piece of 0x1000? No: the chained RF below names 0x1100 as the
    // parent, so the entries (0x1500, 0x2040) and (0x1600, 0x2060) both resolve to primary 0x1100.
    let img = build(4, |b| {
        set_pdata(
            b,
            &[
                (0x1000, 0x1010, 0x2010),
                (0x1100, 0x1180, 0x2000),
                (0x1500, 0x1520, 0x2040),
                (0x1600, 0x1620, 0x2060),
            ],
        )
    });
    let pe = Pe::parse(&img).unwrap();
    let t = pe.function_table().unwrap();
    assert_eq!(t.len(), 4);
    assert!(!t[0].is_chained());
    assert!(!t[1].is_chained());
    assert!(t[2].is_chained() && t[3].is_chained());
    assert_eq!(t[2].parent, Some(r(0x1100, 0x1180, 0x2000)));
    assert_eq!(t[2].primary, r(0x1100, 0x1180, 0x2000));
    assert_eq!(t[0].primary, t[0].function);
    assert_eq!(pe.function_starts().unwrap(), vec![0x1000, 0x1100]);
    assert_eq!(pe.function_start_vas().unwrap(), vec![IMAGE_BASE + 0x1000, IMAGE_BASE + 0x1100]);
}

#[test]
fn multi_level_chain_resolves_to_the_primary() {
    // 0x2070 chains to RF(0x1200, ..., 0x2090), which chains to RF(0x1100, ..., 0x2000), which is primary.
    let img = build(1, |b| set_pdata(b, &[(0x1400, 0x1420, 0x2070)]));
    let pe = Pe::parse(&img).unwrap();
    let t = pe.function_table().unwrap();
    assert_eq!(t[0].parent, Some(r(0x1200, 0x1240, 0x2090)));
    assert_eq!(t[0].primary, r(0x1100, 0x1180, 0x2000));
    assert_eq!(pe.function_starts().unwrap(), vec![0x1100]);
}

#[test]
fn chain_cycle_is_a_limit_error_not_a_hang() {
    let img = build(1, |b| set_pdata(b, &[(0x1400, 0x1420, 0x20c0)]));
    let pe = Pe::parse(&img).unwrap();
    assert_eq!(pe.function_table(), Err(PeError::LimitExceeded("unwind info chain length")));
}

#[test]
fn function_starts_are_sorted_and_deduplicated() {
    let img = build(4, |b| {
        set_pdata(
            b,
            &[(0x1300, 0x1310, 0x2000), (0x1000, 0x1010, 0x2000), (0x1300, 0x1310, 0x2000), (0x1100, 0x1110, 0x2000)],
        )
    });
    let pe = Pe::parse(&img).unwrap();
    assert_eq!(pe.function_starts().unwrap(), vec![0x1000, 0x1100, 0x1300]);
}

#[test]
fn function_entry_is_chained_only_with_a_parent() {
    let f = r(0x10, 0x20, 0x30);
    let e = FunctionEntry { function: f, parent: None, primary: f };
    assert!(!e.is_chained());
    let e = FunctionEntry { function: f, parent: Some(r(1, 2, 3)), primary: r(1, 2, 3) };
    assert!(e.is_chained());
}

#[test]
fn truncated_images_do_not_panic() {
    let img = build(4, |b| {
        set_pdata(b, &[(0x1000, 0x1010, 0x2010), (0x1100, 0x1180, 0x2000), (0x1500, 0x1520, 0x2040), (0x1600, 0x1620, 0x2060)])
    });
    for len in [0usize, 0x100, 0x3ff, 0x400, 0x500, 0x600, 0x610, 0x7ff] {
        if let Ok(pe) = Pe::parse(&img[..len]) {
            let _ = pe.runtime_functions();
            let _ = pe.function_table();
            let _ = pe.function_starts();
        }
    }
}

// ---- real DLL ----

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
fn real_dll_runtime_functions() {
    let Some(d) = load() else { return };
    let pe = Pe::parse(&d).unwrap();
    assert_eq!(pe.data_dirs[3].rva, 0x1607000);
    assert_eq!(pe.data_dirs[3].size, 0xa9620);
    let rfs = pe.runtime_functions().unwrap();
    assert_eq!(rfs.len(), 57_816);
    assert_eq!(rfs[0], r(0x7380, 0x73ba, 0xfa50f8));
    assert_eq!(rfs[1], r(0x74d0, 0x751f, 0xfa5cc0));
    assert_eq!(*rfs.last().unwrap(), r(0xf768a0, 0xf768ae, 0x11dfb18));
    // all inside .text (RVA 0x1000, size 0xfa34b0), strictly ascending, no overlap, none empty
    let text_end = 0x1000 + 0xfa34b0;
    assert!(rfs.iter().all(|e| e.begin_rva >= 0x1000 && e.end_rva <= text_end && e.begin_rva < e.end_rva));
    assert!(rfs.windows(2).all(|w| w[0].begin_rva < w[1].begin_rva));
    assert!(rfs.windows(2).all(|w| w[0].end_rva <= w[1].begin_rva));
    let uw: std::collections::BTreeSet<u32> = rfs.iter().map(|e| e.unwind_rva).collect();
    assert_eq!(uw.len(), 57_816);
}

#[test]
fn real_dll_unwind_flags_and_no_chains() {
    let Some(d) = load() else { return };
    let pe = Pe::parse(&d).unwrap();
    let t = pe.function_table().unwrap();
    assert_eq!(t.len(), 57_816);
    let mut by_flags = std::collections::BTreeMap::new();
    for e in &t {
        let u = pe.unwind_info(e.function.unwind_rva).unwrap();
        assert_eq!(u.version, 1);
        *by_flags.entry(u.flags).or_insert(0usize) += 1;
    }
    assert_eq!(by_flags.into_iter().collect::<Vec<_>>(), vec![(0u8, 46_979usize), (2, 8_021), (3, 2_816)]);
    assert!(t.iter().all(|e| !e.is_chained() && e.primary == e.function));
}

#[test]
fn real_dll_function_starts() {
    let Some(d) = load() else { return };
    let pe = Pe::parse(&d).unwrap();
    let s = pe.function_starts().unwrap();
    assert_eq!(s.len(), 57_816);
    assert_eq!(s[0], 0x7380);
    assert_eq!(*s.last().unwrap(), 0xf768a0);
    let v = pe.function_start_vas().unwrap();
    assert_eq!(v[0], 0x400000 + 0x7380);
}
