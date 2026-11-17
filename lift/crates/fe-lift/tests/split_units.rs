//! `--split-bytes` / `--omit-list` (src/split.rs): the units are those of harness/two/split_bytes.py, then harness/two/lowregs.py, then
//! harness/hyb/strip_units.py, byte for byte, unit by unit. A synthetic image gives two functions (the entry and the jump target at +0x10,
//! seeded as a function); the budget 1 makes every function a unit of its own, a large budget one unit. Outputs go under lift/out-split-test.

mod etxt_common;

use fe_lift::split::{parse_omit, write_units, Split};
use std::collections::BTreeSet;
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

/// The image of size1_lowregs.rs: add eax,ebx; sub eax,1; cmp eax,5; jne +6; mov ecx,7; ret; (+0x10) mov ecx,9; ret
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

/// The units of the Python tools: split_bytes.py, lowregs.py on all of them, strip_units.py with the list.
fn python_units(full: &PathBuf, dir: &PathBuf, tag: &str, budget: usize, omit: &[u64]) -> Vec<(String, Vec<u8>)> {
    let py = |script: &str| crate_dir().join("../../harness").join(script);
    let prefix = dir.join(format!("{tag}-py"));
    let r = Command::new("python3").arg(py("two/split_bytes.py")).arg(full).arg(&prefix).arg(budget.to_string()).output().expect("run split_bytes.py");
    assert!(r.status.success(), "split_bytes.py: {}", String::from_utf8_lossy(&r.stderr));
    let units: Vec<PathBuf> = (0..).map(|n| dir.join(format!("{tag}-py-{n:02}.c"))).take_while(|p| p.exists()).collect();
    let r = Command::new("python3").arg(py("two/lowregs.py")).args(&units).output().expect("run lowregs.py");
    assert!(r.status.success(), "lowregs.py: {}", String::from_utf8_lossy(&r.stderr));
    let list = dir.join(format!("{tag}-omit.txt"));
    let text: String = omit.iter().map(|v| format!("0x{v:x}\n")).collect();
    std::fs::write(&list, text).expect("write omit list");
    let mut out = Vec::new();
    for u in &units {
        let stripped = u.with_extension("stripped.c");
        let s = Command::new("python3").arg(py("hyb/strip_units.py")).arg(u).arg(&list).arg(&stripped).output().expect("run strip_units.py");
        assert!(s.status.success(), "strip_units.py: {}", String::from_utf8_lossy(&s.stderr));
        let src = if stripped.exists() { stripped } else { u.clone() };
        out.push((u.file_name().unwrap().to_string_lossy().to_string(), std::fs::read(&src).expect("read unit")));
    }
    out
}

fn run_case(tag: &str, budget: usize, omit: &[u64]) {
    let dir = crate_dir().join("../../out-split-test/test");
    std::fs::create_dir_all(&dir).expect("create test output dir");
    let tag = etxt_common::uniq(tag);
    let mut out = Vec::new();
    fe_lift::generate_with_seeds(&image(), &[IB + 0x1010], &mut out).expect("generate");
    let text = String::from_utf8(out).expect("utf-8");
    let full = dir.join(format!("{tag}-full.c"));
    std::fs::write(&full, text.as_bytes()).expect("write c");
    let omit_set: BTreeSet<u64> = omit.iter().copied().collect();
    let prefix = dir.join(format!("{tag}-new"));
    let rep = write_units(&text, &Split { prefix: prefix.to_str().unwrap(), max_bytes: budget, omit: &omit_set }).expect("write_units");
    assert_eq!(rep.functions, 2, "two functions");
    assert_eq!(rep.omitted, omit.len());
    let expected = python_units(&full, &dir, &tag, budget, omit);
    assert_eq!(rep.parts, expected.len(), "number of units");
    for (name, bytes) in &expected {
        let got = std::fs::read(dir.join(name.replace("-py-", "-new-"))).expect("read new unit");
        assert!(got == *bytes, "unit {name} differs from split_bytes.py + lowregs.py + strip_units.py");
    }
    let tables = std::fs::read_to_string(format!("{}-tables.txt", prefix.display())).expect("tables");
    assert!(tables.starts_with("const fle_fn_entry fle_dispatch_table[]"), "{tables}");
    assert!(tables.contains("const uint64_t fle_dispatch_count = 2;"), "{tables}");
    assert!(!tables.contains("void f_"), "the tables file holds no function");
    for n in 0..rep.parts {
        let u = std::fs::read_to_string(format!("{}-{:02}.c", prefix.display(), n)).expect("unit");
        assert!(u.contains("#ifndef FLE_LOWREGS\nextern uint64_t R[16];\n#endif\n"), "R is declared in its guard");
        assert!(u.contains("#ifndef FLE_LOWREGS_X\nextern fle_xmm X[16];\n#endif\n"), "X is declared in its guard");
        assert!(!u.contains("fle_dispatch_table"), "the dispatch table is in no unit");
        for va in omit {
            assert!(!u.contains(&format!("void f_{va:x}(void)\n{{")), "the body of {va:#x} is left out");
            assert!(u.contains(&format!("void f_{va:x}(void);")), "the prototype of {va:#x} stays");
        }
    }
}

#[test]
fn units_are_the_python_units_one_function_per_unit() {
    run_case("one", 1, &[]);
}

#[test]
fn units_are_the_python_units_one_unit_with_an_omitted_body() {
    run_case("all", 1 << 30, &[IB + 0x1010]);
}

#[test]
fn units_are_the_python_units_one_function_per_unit_with_an_omitted_body() {
    run_case("omit", 1, &[IB + 0x1010]);
}

#[test]
fn omit_list_reads_hex_vas_and_skips_blank_lines() {
    let s = parse_omit("0x140001010\n\n  140001000  comment\n0xabc\n").expect("parse");
    assert_eq!(s.into_iter().collect::<Vec<_>>(), vec![0xabc, 0x140001000, 0x140001010]);
}
