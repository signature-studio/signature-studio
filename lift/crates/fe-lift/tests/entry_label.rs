//! Function entry label. Checks that, when the entry of a function is not the
//! lowest-VA block of its body, the generated C still
//! - starts the function body with the entry label (`L_<entry>:`),
//! - runs the entry block first (C, clang -O0, runtime check),
//! - compiles with clang -O0.
//!
//! The synthetic function at 0x1010 is called from 0x1000. Its body is
//! 0x1010 (`mov eax,2; jmp 0x1008`) and 0x1008 (`add eax,3; ret`). The entry
//! 0x1010 is above 0x1008, so a VA-ordered body would start at 0x1008.
//!
//! Outputs go under lift/out-entry/test.

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
        (0x1005, vec![0xc3, 0xcc, 0xcc]),
        (0x1008, vec![0x83, 0xc0, 0x03, 0xc3]),
        (0x1010, [&[0xb8, 0x02, 0x00, 0x00, 0x00, 0xe9][..], &rel(0x101a, 0x1008)].concat()),
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
    let d = crate_dir().join("../../out-entry/test");
    std::fs::create_dir_all(&d).expect("create test output dir");
    d
}

fn va(rva: u64) -> u64 {
    IB + rva
}

fn body_start(c: &str, f: u64) -> String {
    let head = format!("void f_{f:x}(void)\n{{\n");
    let i = c.find(&head).unwrap_or_else(|| panic!("no definition of f_{f:x}"));
    c[i + head.len()..].lines().next().unwrap_or("").to_string()
}

#[test]
fn entry_label_starts_function_body() {
    let mut out = Vec::new();
    let st = generate(&build(), &mut out).expect("generate");
    let c = String::from_utf8(out).expect("utf8");

    assert_eq!(st.functions, 2, "functions: {}", st.functions);
    assert!(st.gotos >= 1, "expected a goto from 0x1010 to 0x1008");
    let first = body_start(&c, va(0x1010));
    assert!(first.starts_with(&format!("L_{:x}: ", va(0x1010))), "first line of f_1010: {first}");
    let first = body_start(&c, va(0x1000));
    assert!(first.starts_with(&format!("L_{:x}: ", va(0x1000))), "first line of f_1000: {first}");
    assert!(c.contains(&format!("goto L_{:x}; ", va(0x1008))));
}

#[test]
fn entry_block_runs_first_and_compiles() {
    let mut out = Vec::new();
    generate(&build(), &mut out).expect("generate");
    let dir = out_dir();
    let tag = etxt_common::uniq("entry_label");
    let src = dir.join(format!("{tag}.c"));
    std::fs::write(&src, &out).expect("write entry_label.c");
    let main_src = dir.join(format!("{tag}_main.c"));
    std::fs::write(
        &main_src,
        format!(
            "#include <stdio.h>\n#include <stdint.h>\nextern uint64_t R[16];\nvoid f_{e:x}(void);\n\
             int main(void) {{\n    R[0] = 0;\n    f_{e:x}();\n    printf(\"%llu\\n\", (unsigned long long)R[0]);\n    return R[0] == 5 ? 0 : 1;\n}}\n",
            e = va(0x1010)
        ),
    )
    .expect("write entry_label_main.c");

    let bin = dir.join(&tag);
    let c = Command::new("clang")
        .env("TMPDIR", &dir)
        .args(["-O0", "-mno-red-zone", "-w", "-I"])
        .arg(crate_dir().join("../../runtime"))
        .args(["-o", bin.to_str().unwrap(), src.to_str().unwrap(), main_src.to_str().unwrap()])
        .output()
        .expect("run clang (is clang installed?)");
    assert!(c.status.success(), "clang failed: {}", String::from_utf8_lossy(&c.stderr));

    let r = Command::new(&bin).output().expect("run entry_label");
    assert!(r.status.success(), "stdout: {}", String::from_utf8_lossy(&r.stdout));
    assert_eq!(String::from_utf8_lossy(&r.stdout).trim(), "5");
}
