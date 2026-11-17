//! Decision 0270b (mt3w): a function whose flags a caller on the C path reads after the call is not compiled direct. The wrapper `f_<va>` of a direct function carries registers only and writes no flag to `FL`; the lifted C of
//! a C-path caller reads `FL` after the call. The rule (flagread.rs, applied where the hybrid selection is made) is general: a callee is kept on the C path when, at any call site in a function that stays on the C path, a
//! status flag is read after the call before one is written; so is a function whose flags that callee hands on (a call followed by `ret`, a `jmp`). Tests (hybrid layout, the report of fe-wasmgen):
//!   (a) `call T; je` in a C-path caller: T stays on the C path;     (b) `call T; test eax,eax; je`: T is compiled;
//!   (c) `X: call T; ret` and a C-path `call X; je`: X and T stay;    (d) `X: jmp T` and a C-path `call X; je`: X and T stay;
//!   (e) the caller is compiled and reads the callee's flags from its summary (decision 0270): both are compiled;    (f) the caller is compiled but the read has no summary: the caller is not compiled, so T stays.
//! Nothing here names a function, address or module.
#![cfg(target_arch = "x86_64")]

use fe_wasmgen::{compile, Config};
use std::collections::{HashMap, HashSet};

fn w16(b: &mut [u8], o: usize, v: u16) {
    b[o..o + 2].copy_from_slice(&v.to_le_bytes());
}
fn w32(b: &mut [u8], o: usize, v: u32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}
fn w64(b: &mut [u8], o: usize, v: u64) {
    b[o..o + 8].copy_from_slice(&v.to_le_bytes());
}

fn pe(funcs: &[(u32, Vec<u8>)]) -> Vec<u8> {
    let mut b = vec![0u8; 0xc00];
    let (lf, opt) = (0x80usize, 0x80 + 24);
    let sec = opt + 240;
    w16(&mut b, 0, 0x5a4d);
    w32(&mut b, 0x3c, lf as u32);
    w32(&mut b, lf, 0x4550);
    let f = lf + 4;
    w16(&mut b, f, 0x8664);
    w16(&mut b, f + 2, 2);
    w16(&mut b, f + 16, 240);
    w16(&mut b, f + 18, 0x2022);
    w16(&mut b, opt, 0x20b);
    w32(&mut b, opt + 16, 0x1000);
    w64(&mut b, opt + 24, 0x400000);
    w32(&mut b, opt + 32, 0x1000);
    w32(&mut b, opt + 36, 0x200);
    w32(&mut b, opt + 56, 0x3000);
    w32(&mut b, opt + 60, 0x400);
    w16(&mut b, opt + 68, 3);
    w32(&mut b, opt + 108, 16);
    w32(&mut b, opt + 112 + 3 * 8, 0x2000);
    w32(&mut b, opt + 112 + 3 * 8 + 4, 12 * funcs.len() as u32);
    let mut section = |i: usize, name: &[u8], vs: u32, rva: u32, rs: u32, rp: u32, ch: u32| {
        let s = sec + 40 * i;
        b[s..s + name.len()].copy_from_slice(name);
        w32(&mut b, s + 8, vs);
        w32(&mut b, s + 12, rva);
        w32(&mut b, s + 16, rs);
        w32(&mut b, s + 20, rp);
        w32(&mut b, s + 36, ch);
    };
    section(0, b".text", 0x400, 0x1000, 0x400, 0x400, 0x60000020);
    section(1, b".pdata", 0x200, 0x2000, 0x400, 0x800, 0x40000040);
    for (n, (off, code)) in funcs.iter().enumerate() {
        let at = 0x400 + *off as usize;
        b[at..at + code.len()].copy_from_slice(code);
        let p = 0x800 + 12 * n;
        w32(&mut b, p, 0x1000 + off);
        w32(&mut b, p + 4, 0x1000 + off + code.len() as u32);
        w32(&mut b, p + 8, 0x2100);
    }
    b[0x900..0x904].copy_from_slice(&[1, 0, 0, 0]);
    b
}

fn conf(only: &[u64]) -> Config {
    let mut ci: HashMap<String, String> = HashMap::new();
    ci.insert("dummy".into(), "fle_imp_dummy".into());
    Config {
        hybrid_r: Some(0x2000),
        tls_ctx: Some(0x2000),
        hybrid_x: Some(0x2200),
        hybrid_mx: Some((0x3000, 0x3100)),
        c_imports: Some(ci),
        mem_import: Some(("c".into(), "memory".into())),
        teb: Some(0x1000),
        hybrid_only: Some(only.iter().copied().collect::<HashSet<u64>>()),
        ..Config::default()
    }
}

fn rel(from_next: usize, to: usize) -> [u8; 4] {
    (to as i32 - from_next as i32).to_le_bytes()
}

const BASE: u64 = 0x401000;
const T_AT: usize = 0x100;
const X_AT: usize = 0x180;

/// T: xor eax,eax; ret          (returns with ZF = 1 and CF = SF = OF = 0)
fn code_t() -> Vec<u8> {
    vec![0x31, 0xc0, 0xc3]
}
/// `call <target>; <tail>` for a function at offset `at`
fn caller(at: usize, target: usize, tail: &[u8]) -> Vec<u8> {
    let mut c = vec![0xe8];
    c.extend_from_slice(&rel(at + 5, target));
    c.extend_from_slice(tail);
    c
}
/// je over `mov eax,1; ret`, then `mov eax,2; ret`
const JE: [u8; 15] = [0x74, 0x06, 0xb8, 0x01, 0x00, 0x00, 0x00, 0xc3, 0xb8, 0x02, 0x00, 0x00, 0x00, 0xc3, 0x90];
/// test eax,eax; then the same
fn test_je() -> Vec<u8> {
    let mut t = vec![0x85, 0xc0];
    t.extend_from_slice(&JE);
    t
}

fn status(comp: &fe_wasmgen::Compiled, k: u64) -> String {
    comp.report.rows.iter().find(|r| r.va == k).map(|r| r.status.clone()).unwrap_or_default()
}
const KEPT: &str = "flags are read after a call of it by a caller on the C path";

#[test]
fn a_callee_whose_flags_a_c_path_caller_reads_stays_on_the_c_path() {
    let c = caller(0, T_AT, &JE);
    let comp = compile(&pe(&[(0, c), (T_AT as u32, code_t())]), &[BASE, BASE + T_AT as u64], &[], &conf(&[BASE + T_AT as u64])).unwrap();
    assert!(status(&comp, BASE + T_AT as u64).contains(KEPT), "(a) T: {}", status(&comp, BASE + T_AT as u64));
    assert!(!status(&comp, BASE).contains(KEPT), "the caller is not the callee");
}

#[test]
fn a_callee_is_compiled_when_the_caller_writes_a_flag_before_it_reads_one() {
    let c = caller(0, T_AT, &test_je());
    let comp = compile(&pe(&[(0, c), (T_AT as u32, code_t())]), &[BASE, BASE + T_AT as u64], &[], &conf(&[BASE + T_AT as u64])).unwrap();
    assert_eq!(status(&comp, BASE + T_AT as u64), "ok", "(b) T");
}

#[test]
fn the_flags_that_a_callee_hands_on_keep_the_function_that_produced_them_on_the_c_path() {
    // (c) X: call T; ret       C-path caller: call X; je
    let x = caller(X_AT, T_AT, &[0xc3]);
    let c = caller(0, X_AT, &JE);
    let comp = compile(&pe(&[(0, c), (X_AT as u32, x), (T_AT as u32, code_t())]), &[BASE, BASE + X_AT as u64, BASE + T_AT as u64], &[], &conf(&[BASE + X_AT as u64, BASE + T_AT as u64])).unwrap();
    assert!(status(&comp, BASE + X_AT as u64).contains(KEPT), "(c) X: {}", status(&comp, BASE + X_AT as u64));
    assert!(status(&comp, BASE + T_AT as u64).contains(KEPT), "(c) T: {}", status(&comp, BASE + T_AT as u64));
    // (d) X: jmp T
    let mut x = vec![0xe9];
    x.extend_from_slice(&rel(X_AT + 5, T_AT));
    let c = caller(0, X_AT, &JE);
    let comp = compile(&pe(&[(0, c), (X_AT as u32, x), (T_AT as u32, code_t())]), &[BASE, BASE + X_AT as u64, BASE + T_AT as u64], &[], &conf(&[BASE + X_AT as u64, BASE + T_AT as u64])).unwrap();
    assert!(status(&comp, BASE + X_AT as u64).contains(KEPT), "(d) X: {}", status(&comp, BASE + X_AT as u64));
    assert!(status(&comp, BASE + T_AT as u64).contains(KEPT), "(d) T: {}", status(&comp, BASE + T_AT as u64));
}

#[test]
fn a_compiled_caller_reads_the_flags_of_a_compiled_callee_from_its_summary() {
    // (e) both selected: the caller is compiled (decision 0270: the callee's ZF = 1 is a constant), so no site is on the C path
    let c = caller(0, T_AT, &JE);
    let comp = compile(&pe(&[(0, c), (T_AT as u32, code_t())]), &[BASE, BASE + T_AT as u64], &[], &conf(&[BASE, BASE + T_AT as u64])).unwrap();
    assert_eq!(status(&comp, BASE), "ok", "(e) caller");
    assert_eq!(status(&comp, BASE + T_AT as u64), "ok", "(e) callee");
}

#[test]
fn a_caller_that_cannot_be_compiled_is_a_c_path_caller_even_when_it_is_selected() {
    // (f) the caller is selected but the flags it reads have no summary (the callee leaves them to a run-time `cmp`): the caller cannot be compiled, so it is a C-path caller and the callee stays
    let t = vec![0x48, 0x39, 0xd1, 0xc3];
    let c = caller(0, T_AT, &JE);
    let comp = compile(&pe(&[(0, c), (T_AT as u32, t)]), &[BASE, BASE + T_AT as u64], &[], &conf(&[BASE, BASE + T_AT as u64])).unwrap();
    assert!(!status(&comp, BASE).starts_with("ok"), "(f) caller: {}", status(&comp, BASE));
    assert!(status(&comp, BASE + T_AT as u64).contains(KEPT), "(f) callee: {}", status(&comp, BASE + T_AT as u64));
}
