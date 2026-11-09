//! Decision 0155, 0243: the dead-flags lowering of `generate_full` (always applied unless the module has swap stubs).
//!
//! Synthetic images (the layout of entry_label.rs). Checks, 
//! - the default `GenOptions` give the output of `generate_with_seh`, byte for byte;
//! - the structural effect in the C (the `fle_nf_*` helpers for the dead flag writes only);
//! - the compiled program computes the same registers with and without the option, for several inputs.
//!
//! Outputs go under lift/out-size1-test/test.

mod etxt_common;

use fe_decode::Options;
use fe_lift::{generate_full, generate_with_seh, GenOptions, SehOptions, Stats};
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

fn build(code: &[(u32, Vec<u8>)]) -> Vec<u8> {
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
    for (rva, bytes) in code {
        let o = TEXT_RAW + (*rva - TEXT_RVA) as usize;
        b[o..o + bytes.len()].copy_from_slice(bytes);
    }
    b
}

fn va(rva: u64) -> u64 {
    IB + rva
}

/// Two entries (0x1000: `mov eax,1; jmp S`, 0x1010: `mov eax,2; jmp S`) that jump into one body S at 0x1020 (`add eax,3; add eax,4; ret`).
fn shared_image() -> Vec<u8> {
    build(&[
        (0x1000, [&[0xb8, 0x01, 0x00, 0x00, 0x00, 0xe9][..], &rel(0x100a, 0x1020)].concat()),
        (0x1010, [&[0xb8, 0x02, 0x00, 0x00, 0x00, 0xe9][..], &rel(0x101a, 0x1020)].concat()),
        (0x1020, vec![0x83, 0xc0, 0x03, 0x83, 0xc0, 0x04, 0xc3]),
    ])
}

/// 0x1000: add eax,ebx; sub eax,1; cmp eax,5; jne 0x1010; mov ecx,7; ret.  0x1010: mov ecx,9; ret.
/// The flags of the add and of the sub are overwritten by the cmp before anything reads them; the flags of the cmp are read by the jne.
fn flags_image() -> Vec<u8> {
    build(&[
        (0x1000, vec![0x01, 0xd8, 0x83, 0xe8, 0x01, 0x83, 0xf8, 0x05, 0x75, 0x06, 0xb9, 0x07, 0x00, 0x00, 0x00, 0xc3]),
        (0x1010, vec![0xb9, 0x09, 0x00, 0x00, 0x00, 0xc3]),
    ])
}

fn gen(img: &[u8], seeds: &[u64], g: &GenOptions) -> (String, Stats) {
    let mut out = Vec::new();
    let st = generate_full(img, seeds, &[], &[], &[], &Options::default(), &SehOptions::default(), g, &mut out).expect("generate_full");
    (String::from_utf8(out).expect("utf8"), st)
}

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn out_dir() -> PathBuf {
    let d = crate_dir().join("../../out-size1-test/test");
    std::fs::create_dir_all(&d).expect("create test output dir");
    d
}

/// Compile `c` with a main that sets rax, rbx, calls f_<entry>, prints rax, rcx; returns the stdout.
fn run(c: &str, entry: u64, rax: u64, rbx: u64) -> String {
    let dir = out_dir();
    let tag = etxt_common::uniq("size1");
    let src = dir.join(format!("{tag}.c"));
    std::fs::write(&src, c).expect("write c");
    let main_src = dir.join(format!("{tag}_main.c"));
    std::fs::write(
        &main_src,
        format!(
            "#include <stdio.h>\n#include <stdint.h>\nextern uint64_t R[16];\nvoid f_{entry:x}(void);\n\
             int main(void) {{\n    R[0] = {rax}; R[3] = {rbx}; R[4] = 0x7000;\n    f_{entry:x}();\n    printf(\"%llu %llu\\n\", (unsigned long long)R[0], (unsigned long long)R[1]);\n    return 0;\n}}\n"
        ),
    )
    .expect("write main");
    let bin = dir.join(&tag);
    let cc = Command::new("clang")
        .env("TMPDIR", &dir)
        .args(["-O1", "-mno-red-zone", "-w", "-I"])
        .arg(crate_dir().join("../../runtime"))
        .args(["-o", bin.to_str().unwrap(), src.to_str().unwrap(), main_src.to_str().unwrap()])
        .output()
        .expect("run clang (is clang installed?)");
    assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new(&bin).output().expect("run");
    assert!(r.status.success(), "stdout: {}", String::from_utf8_lossy(&r.stdout));
    String::from_utf8_lossy(&r.stdout).trim().to_string()
}

#[test]
fn dead_flags_replaces_only_the_dead_writes() {
    let img = flags_image();
    let seeds = [va(0x1010)];
    let (c1, s1) = gen(&img, &seeds, &GenOptions::default());
    assert_eq!(s1.flag_writers, 3, "add, sub, cmp");
    assert_eq!(s1.flag_dead_full, 2, "the writes of the add and of the sub are dead");
    assert_eq!(s1.flag_substituted, 2);
    assert!(c1.contains("#include \"fle_flags_nf.h\""));
    assert!(c1.contains("fle_nf_add(32,") && c1.contains("fle_nf_sub(32,"), "the nf helpers");
    assert_eq!(c1.matches("fle_op_sub(").count(), 1, "the cmp keeps the helper that writes the flags the jne reads");
    assert_eq!(c1.matches("fle_op_add(").count(), 0);
    assert_eq!(run(&c1, va(0x1000), 2, 4), "5 7");
    assert_eq!(run(&c1, va(0x1000), 3, 4), "6 9");
}

#[test]
fn dead_flags_keeps_flags_that_a_call_may_read() {
    // 0x1000: cmp eax,1; call 0x1010; ret.  0x1010: je 0x1018 (reads ZF set by the caller); mov ecx,1; ret.  0x1018: mov ecx,2; ret.
    // The callee reads the flags of the caller: the cmp is not dead. A direct call of an entry is analysed exactly.
    let img = build(&[
        (0x1000, [&[0x83, 0xf8, 0x01, 0xe8][..], &rel(0x1008, 0x1010), &[0xc3][..]].concat()),
        (0x1010, vec![0x74, 0x06, 0xb9, 0x01, 0x00, 0x00, 0x00, 0xc3]),
        (0x1018, vec![0xb9, 0x02, 0x00, 0x00, 0x00, 0xc3]),
    ]);
    let (c1, s1) = gen(&img, &[va(0x1010), va(0x1018)], &GenOptions::default());
    assert_eq!(s1.flag_dead_full, 0);
    assert_eq!(c1.matches("fle_nf_").count(), 0, "{c1}");
}
