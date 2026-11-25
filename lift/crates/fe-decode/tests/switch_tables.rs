//! MSVC switch tables that the lift did not recognise in the real plug-in DLLs (issue #1413, decision 0430). The binaries are read
//! from `$SYTRUS_PE` / `$BALANCE_PE` / `$FLENGINE_PE`, else from the extraction tree of the repository (lift/out-app/host). With the
//! variable set and the file missing the test fails; with the default path missing it is skipped (as in adjustor.rs).
//!
//! Sytrus_x64.dll (ImageBase 0x400000, rebased to 0xc580000 in the engine; the RVA of the case in the report is 0x44f273):
//! - 0x44f271 (function 0x44f130): `cmp DWORD PTR [rsp+0x88],0x1e; ja; movsxd rax,DWORD PTR [rsp+0x88]; lea rcx,[rip+ImageBase];
//!   mov eax,[rcx+rax*4+0x4520d0]; add rax,rcx; jmp rax`. The guard compares the spilled index in memory; N = 0x1e.
//! - 0x55f666 (function containing 0x55f666): `cmp ecx,0x22; ja; lea rax,[rip+T]; movsxd rcx,[rax+rcx*4]; add rcx,rax; jmp rcx`
//!   (table-relative, T = 0x5604f4, N = 0x22).
//! - 0x563298: `cmp ecx,0x1e; ja; mov eax,-3; mov edi,1; movsxd rcx,[r15+rcx*4]; add rcx,r15; jmp rcx`
//!   (table-relative, r15 = lea 0x564b08 on every path, N = 0x1e, the guard is in front of the idiom with two instructions between).
//! Balance (the same two forms at 0x54a9b6 and 0x54e5e8) and FLEngine (its three annotated tables are unchanged) are checked too.

use fe_decode::{analyze_with_pdata, survey, Origin, Resolved, Select};
use fe_pe::Pe;
use std::path::PathBuf;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The bytes of the DLL named by `var`, else of `default` under the repository; `None` (skip) when neither exists.
fn load(var: &str, default: &str) -> Option<Vec<u8>> {
    let (path, strict) = match std::env::var(var) {
        Ok(p) => (PathBuf::from(p), true),
        Err(_) => (repo().join(default), false),
    };
    match std::fs::read(&path) {
        Ok(b) => Some(b),
        Err(e) if strict => panic!("{var}={}: {e}", path.display()),
        Err(_) => None,
    }
}

const SYTRUS: &str = "lift/out-app/host/lift/out-plugins/generators.sytrus.sytrus_x64.dll";
const BALANCE: &str = "lift/out-app/host/lift/out-plugins/effects.fruity-balance.fruity_balance_x64.dll";
const FLENGINE: &str = "lift/out-app/host/binaries/win64/FLEngine_x64.dll";

fn msvc(a: &fe_decode::Analysis, va: u64) -> (u64, bool, u64) {
    match a.jumptabs.get(&va).map(|t| (t.select, &t.origin)) {
        Some((Select::Target, Origin::Msvc { n, rel, .. })) => (*n, *rel, a.jumptabs[&va].cases.len() as u64),
        other => panic!("{va:#x}: not an MSVC table: {other:?}"),
    }
}

fn check_dll(data: &[u8]) -> (Pe<'_>, fe_decode::Analysis, survey::Summary) {
    let pe = Pe::parse(data).unwrap();
    let a = analyze_with_pdata(&pe).unwrap();
    let (_, s) = survey::report(&pe, &a).unwrap();
    (pe, a, s)
}

#[test]
fn sytrus_guard_in_memory_table_at_0x44f271_is_recognised() {
    let Some(data) = load("SYTRUS_PE", SYTRUS) else { return };
    let (pe, a, s) = check_dll(&data);
    let va = pe.rva_to_va(0x44f271);
    // N = 0x1e: 31 entries, the case 0x44f273 (RVA) is one of the targets
    let (n, rel, cases) = msvc(&a, va);
    assert_eq!((n, rel), (0x1e, false));
    assert!(a.jumptabs[&va].cases.iter().any(|&(_, t)| t == pe.rva_to_va(0x44f273)));
    assert!(matches!(a.jumptabs[&va].origin, Origin::Msvc { t2: 0x4520d0, .. }), "{:?}", a.jumptabs[&va].origin);
    assert!(cases > 0);
    assert_eq!(a.resolved[&va], Resolved::Unresolved);
    assert!(a.closure.msvc_rejected.iter().all(|(v, _)| *v != va), "{:?}", a.closure.msvc_rejected);
    assert_eq!(s.misrecognized, 0);
}

#[test]
fn sytrus_table_relative_movsxd_tables_are_recognised() {
    let Some(data) = load("SYTRUS_PE", SYTRUS) else { return };
    let (pe, a, s) = check_dll(&data);
    let x = pe.rva_to_va(0x15f666);
    assert_eq!(msvc(&a, x).0, 0x22);
    assert!(msvc(&a, x).1);
    let z = pe.rva_to_va(0x163298);
    assert_eq!(msvc(&a, z).0, 0x1e);
    assert!(msvc(&a, z).1);
    assert_eq!(s.unrecognized_table_shape, 0, "{s:?}");
    assert_eq!(s.misrecognized, 0, "{s:?}");
}

#[test]
fn balance_table_relative_movsxd_tables_are_recognised() {
    let Some(data) = load("BALANCE_PE", BALANCE) else { return };
    let (pe, a, s) = check_dll(&data);
    assert_eq!(msvc(&a, pe.rva_to_va(0x14a9b6)).0, 0x22);
    assert_eq!(msvc(&a, pe.rva_to_va(0x14e5e8)).0, 0x1e);
    assert_eq!(s.unrecognized_table_shape, 0, "{s:?}");
    assert_eq!(s.misrecognized, 0, "{s:?}");
}

#[test]
fn flengine_annotated_tables_are_unchanged_and_no_table_is_missed() {
    let Some(data) = load("FLENGINE_PE", FLENGINE) else { return };
    let (_, a, s) = check_dll(&data);
    assert_eq!(a.jumptabs.values().filter(|t| t.origin == Origin::Annotated).count(), 3);
    assert_eq!(s.unrecognized_table_shape, 0, "{s:?}");
    assert_eq!(s.misrecognized, 0, "{s:?}");
}
