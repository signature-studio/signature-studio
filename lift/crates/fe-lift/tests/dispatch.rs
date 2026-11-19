//! Indirect call dispatch (`fle_dispatch`). Checks that
//! - a call to a table VA runs that function (C, clang -O0, synthetic table),
//! - a value that is not in the table aborts (same C program),
//! - the generated C for an unresolved `call rbx` uses `fle_dispatch` and the table
//!   contains every function entry (Rust, synthetic PE),
//! - the generated translation unit compiles with clang -O0.
//!
//! Outputs go under lift/out-dispatch/test (not out-uA).

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
fn rel(next: u32, target: u32) -> [u8; 4] {
    ((target as i64 - next as i64) as i32).to_le_bytes()
}

/// One `.text` section at 0x1000. `call 0x1010` makes 0x1010 a function entry.
/// `0x1005: call rbx` is the indirect call under test. Its target is not set
/// anywhere, so the static resolver should leave it unresolved.
fn build() -> Vec<u8> {
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
        (0x1000, [&[0xe8][..], &rel(0x1005, 0x1010)].concat()),
        (0x1005, vec![0xff, 0xd3]),
        (0x1007, vec![0xc3]),
        (0x1010, vec![0xc3]),
    ];
    for (rva, bytes) in code {
        let o = TEXT_RAW + (rva - TEXT_RVA) as usize;
        b[o..o + bytes.len()].copy_from_slice(bytes);
    }
    b
}

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn out_dir() -> PathBuf {
    let d = crate_dir().join("../../out-dispatch/test");
    std::fs::create_dir_all(&d).expect("create test output dir");
    d
}

/// Synthetic table with two entries (0x10 and 0x20) and a dispatch harness.
/// argv[1] == "miss" calls 0x30, which is not in the table.
const HARNESS: &str = r#"
#include "fle_runtime.h"
#include "fle_dispatch.h"
#include <string.h>

static int hits_a, hits_b;
static void f_10(void) { hits_a++; }
static void f_20(void) { hits_b++; }

const fle_fn_entry fle_dispatch_table[] = {
    { UINT64_C(0x10), f_10 },
    { UINT64_C(0x20), f_20 },
};
const uint64_t fle_dispatch_count = 2;

uint64_t R[16];

int main(int argc, char **argv) {
    if (argc > 1 && strcmp(argv[1], "miss") == 0) {
        fle_dispatch(UINT64_C(0x30));
        printf("not aborted\n");
        return 0;
    }
    fle_dispatch(UINT64_C(0x20));
    fle_dispatch(UINT64_C(0x10));
    fle_dispatch(UINT64_C(0x20));
    printf("hits_a %d hits_b %d\n", hits_a, hits_b);
    return hits_a == 1 && hits_b == 2 ? 0 : 1;
}
"#;

fn clang(args: &[&str]) -> std::process::Output {
    Command::new("clang")
        .args(["-O0", "-mno-red-zone", "-w", "-I"])
        .arg(crate_dir().join("../../runtime"))
        .args(args)
        .output()
        .expect("run clang (is clang installed?)")
}

#[test]
fn dispatch_calls_table_entry_and_aborts_on_miss() {
    let tag = etxt_common::uniq("dispatch_harness");
    let src = out_dir().join(format!("{tag}.c"));
    std::fs::write(&src, HARNESS).expect("write harness");
    let bin = out_dir().join(&tag);
    let c = clang(&["-o", bin.to_str().unwrap(), src.to_str().unwrap()]);
    assert!(c.status.success(), "clang failed: {}", String::from_utf8_lossy(&c.stderr));

    let ok = Command::new(&bin).output().expect("run harness");
    assert!(ok.status.success(), "harness failed: {}", String::from_utf8_lossy(&ok.stdout));
    assert!(String::from_utf8_lossy(&ok.stdout).contains("hits_a 1 hits_b 2"));

    let miss = Command::new(&bin).arg("miss").output().expect("run harness miss");
    assert!(!miss.status.success(), "miss did not stop the program");
    assert!(
        String::from_utf8_lossy(&miss.stderr).contains("not a known function entry"),
        "stderr: {}",
        String::from_utf8_lossy(&miss.stderr)
    );
    assert!(!String::from_utf8_lossy(&miss.stdout).contains("not aborted"));
}

#[test]
fn unresolved_indirect_call_uses_dispatch_and_table_has_entries() {
    let img = build();
    let mut out = Vec::new();
    let st = generate(&img, &mut out).expect("generate");
    let c = String::from_utf8(out).expect("utf8");

    assert_eq!(st.handled["call_ind:dispatch"], 1, "handled: {:?}", st.handled);
    assert!(!st.unsupported.contains_key("Call: indirect target unresolved"));
    assert!(c.contains("fle_dispatch(t);"));
    assert!(c.contains("#include \"fle_dispatch.h\""));
    assert!(c.contains(&format!("{{ UINT64_C({:#x}), f_{:x} }},", IB + 0x1000, IB + 0x1000)));
    assert!(c.contains(&format!("{{ UINT64_C({:#x}), f_{:x} }},", IB + 0x1010, IB + 0x1010)));
    assert!(c.contains("const uint64_t fle_dispatch_count = 2;"));
}

#[test]
fn generated_unit_compiles_with_clang_o0() {
    let img = build();
    let mut out = Vec::new();
    generate(&img, &mut out).expect("generate");
    let tag = etxt_common::uniq("synthetic_lift");
    let src = out_dir().join(format!("{tag}.c"));
    std::fs::write(&src, &out).expect("write synthetic_lift.c");
    let obj = out_dir().join(format!("{tag}.o"));
    let r = clang(&["-c", "-o", obj.to_str().unwrap(), src.to_str().unwrap()]);
    assert!(r.status.success(), "clang failed: {}", String::from_utf8_lossy(&r.stderr));
}
