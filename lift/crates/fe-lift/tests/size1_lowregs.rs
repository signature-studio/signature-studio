//! Decision 0155 (provisional): the opt-in `-DFLE_LOWREGS` of runtime/fle_lowregs.h and harness/two/lowregs.py.
//!
//! A synthetic image is lifted, `lowregs.py` wraps the definitions of R and FL, and clang compiles the C twice: without the macro (R and FL are defined symbols, as always)
//! and with `-DFLE_LOWREGS` (no symbol R or FL is defined or referenced: every access is a constant address). A native run cannot map the fixed addresses (below
//! mmap_min_addr), so this test compiles and inspects the object only; the run is the engine check of the decision (node, wasm). Outputs go under lift/out-size1-test/test.

mod etxt_common;

use fe_lift::generate;
use std::path::PathBuf;
use std::process::Command;

const IB: u64 = 0x1_4000_0000;
const LFANEW: usize = 0x80;
const OPT: usize = LFANEW + 24;
const SEC: usize = OPT + 240;
const TEXT_RVA: u32 = 0x1000;
const TEXT_RAW: usize = 0x400;
const FILE_LEN: usize = 0x800;

fn w16(b: &mut [u8], o: usize, v: u16) {
    b[o..o + 2].copy_from_slice(&v.to_le_bytes());
}
fn w32(b: &mut [u8], o: usize, v: u32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}
fn w64(b: &mut [u8], o: usize, v: u64) {
    b[o..o + 8].copy_from_slice(&v.to_le_bytes());
}

/// add eax,ebx; sub eax,1; cmp eax,5; jne +6; mov ecx,7; ret; (+0x10) mov ecx,9; ret
fn image() -> Vec<u8> {
    let mut b = vec![0u8; FILE_LEN];
    w16(&mut b, 0, 0x5a4d);
    w32(&mut b, 0x3c, LFANEW as u32);
    w32(&mut b, LFANEW, 0x4550);
    let f = LFANEW + 4;
    w16(&mut b, f, 0x8664);
    w16(&mut b, f + 2, 1);
    w16(&mut b, f + 16, 240);
    w16(&mut b, f + 18, 0x2022);
    let o = OPT;
    w16(&mut b, o, 0x20b);
    w32(&mut b, o + 16, TEXT_RVA);
    w64(&mut b, o + 24, IB);
    w32(&mut b, o + 32, 0x400);
    w32(&mut b, o + 36, 0);
    w32(&mut b, o + 56, 0x2000);
    w32(&mut b, o + 60, 0x400);
    w16(&mut b, o + 68, 3);
    w32(&mut b, o + 108, 16);
    b[SEC..SEC + 5].copy_from_slice(b".text");
    w32(&mut b, SEC + 8, 0x400);
    w32(&mut b, SEC + 12, TEXT_RVA);
    w32(&mut b, SEC + 16, 0x400);
    w32(&mut b, SEC + 20, TEXT_RAW as u32);
    w32(&mut b, SEC + 36, 0x6000_0020);
    let code: &[(u32, Vec<u8>)] = &[
        (0x1000, vec![0x01, 0xd8, 0x83, 0xe8, 0x01, 0x83, 0xf8, 0x05, 0x75, 0x06, 0xb9, 0x07, 0x00, 0x00, 0x00, 0xc3]),
        (0x1010, vec![0xb9, 0x09, 0x00, 0x00, 0x00, 0xc3]),
    ];
    for (rva, bytes) in code {
        let o = TEXT_RAW + (*rva - TEXT_RVA) as usize;
        b[o..o + bytes.len()].copy_from_slice(bytes);
    }
    b
}

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn symbols(obj: &PathBuf) -> String {
    let o = Command::new("nm").arg(obj).output().expect("run nm");
    String::from_utf8_lossy(&o.stdout).to_string()
}

fn has_symbol(nm: &str, name: &str, kind: &[&str]) -> bool {
    nm.lines().any(|l| {
        let t: Vec<&str> = l.split_whitespace().collect();
        t.len() >= 2 && t[t.len() - 1] == name && kind.contains(&t[t.len() - 2])
    })
}

#[test]
fn lowregs_removes_the_symbols_r_and_fl_and_only_with_the_macro() {
    let dir = crate_dir().join("../../out-size1-test/test");
    std::fs::create_dir_all(&dir).expect("create test output dir");
    let tag = etxt_common::uniq("lowregs");
    let c = dir.join(format!("{tag}.c"));
    let mut out = Vec::new();
    generate(&image(), &mut out).expect("generate");
    std::fs::write(&c, &out).expect("write c");
    let rt = crate_dir().join("../../runtime");
    let py = crate_dir().join("../../harness/two/lowregs.py");
    let cc = |extra: &[&str], src: &PathBuf, obj: &PathBuf| {
        let r = Command::new("clang").env("TMPDIR", &dir).args(["-O1", "-w", "-c", "-I"]).arg(&rt).args(extra).arg("-o").arg(obj).arg(src).output().expect("run clang");
        assert!(r.status.success(), "clang failed: {}", String::from_utf8_lossy(&r.stderr));
    };
    // without lowregs.py and without the macro: the definitions are there (the old behaviour)
    let o0 = dir.join(format!("{tag}-plain.o"));
    cc(&[], &c, &o0);
    let nm0 = symbols(&o0);
    assert!(has_symbol(&nm0, "R", &["B", "D", "C"]) && has_symbol(&nm0, "FL", &["B", "D", "C"]), "R and FL are defined: {nm0}");
    // lowregs.py; without the macro the file means what it meant
    let w = Command::new("python3").arg(&py).arg(&c).output().expect("run lowregs.py");
    assert!(w.status.success(), "{}", String::from_utf8_lossy(&w.stderr));
    let o1 = dir.join(format!("{tag}-wrapped.o"));
    cc(&[], &c, &o1);
    let nm1 = symbols(&o1);
    assert!(has_symbol(&nm1, "R", &["B", "D", "C"]) && has_symbol(&nm1, "FL", &["B", "D", "C"]), "wrapped, no macro: R and FL are defined: {nm1}");
    // with the macro: no symbol R or FL at all, defined or undefined
    let o2 = dir.join(format!("{tag}-low.o"));
    cc(&["-DFLE_LOWREGS"], &c, &o2);
    let nm2 = symbols(&o2);
    assert!(!has_symbol(&nm2, "R", &["B", "D", "C", "U"]) && !has_symbol(&nm2, "FL", &["B", "D", "C", "U"]), "-DFLE_LOWREGS: no symbol R or FL: {nm2}");
    // the C text of the functions is the same
    let text = String::from_utf8(out).unwrap();
    assert!(std::fs::read_to_string(&c).unwrap().contains("#ifndef FLE_LOWREGS"), "{text}");
}
