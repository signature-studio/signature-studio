//! Tests for kernel32!VerifyVersionInfoW (docs/decisions/0012-verifyversioninfow.md) and
//! kernel32!LocalFree (runtime/fle_imports.c).
//!
//! 1. VerifyVersionInfoW: no response is implemented (its result depends on the running OS,
//!    and the spec does not say which OS state FLEngine requires). The thunk must stay
//!    unsupported (fle_unsupported), and the import table must not list it.
//! 2. LocalFree: the thunk calls the runtime function, which is checked by the C test below.

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
const DATA_RVA: u32 = 0x2000;
const DATA_RAW: usize = 0x800;
const FILE_LEN: usize = 0x1000;

fn w16(b: &mut [u8], o: usize, v: u16) {
    b[o..o + 2].copy_from_slice(&v.to_le_bytes());
}
fn w32(b: &mut [u8], o: usize, v: u32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}
fn w64(b: &mut [u8], o: usize, v: u64) {
    b[o..o + 8].copy_from_slice(&v.to_le_bytes());
}
fn data_at(rva: u32) -> usize {
    DATA_RAW + (rva - DATA_RVA) as usize
}
fn put(b: &mut [u8], rva: u32, bytes: &[u8]) {
    let o = data_at(rva);
    b[o..o + bytes.len()].copy_from_slice(bytes);
}
fn rel(next: u32, target: u32) -> [u8; 4] {
    ((target as i64 - next as i64) as i32).to_le_bytes()
}

/// A PE with kernel32.dll!VerifyVersionInfoW only. Entry 0x1000 calls the IAT thunk at 0x1040.
fn build() -> Vec<u8> {
    build_with("VerifyVersionInfoW")
}

/// A PE with kernel32.dll!<name> only. Entry 0x1000 calls the IAT thunk at 0x1040.
fn build_with(name: &str) -> Vec<u8> {
    let mut b = vec![0u8; FILE_LEN];
    w16(&mut b, 0, 0x5a4d);
    w32(&mut b, 0x3c, LFANEW as u32);
    w32(&mut b, LFANEW, 0x4550);
    let f = LFANEW + 4;
    w16(&mut b, f, 0x8664);
    w16(&mut b, f + 2, 2);
    w16(&mut b, f + 16, 240);
    w16(&mut b, f + 18, 0x2022);
    let o = OPT;
    w16(&mut b, o, 0x20b);
    w32(&mut b, o + 16, TEXT_RVA);
    w64(&mut b, o + 24, IB);
    w32(&mut b, o + 32, 0x1000);
    w32(&mut b, o + 36, 0x200);
    w32(&mut b, o + 56, 0x3000);
    w32(&mut b, o + 60, 0x400);
    w16(&mut b, o + 68, 3);
    w32(&mut b, o + 108, 16);
    w32(&mut b, o + 120, DATA_RVA);
    w32(&mut b, o + 124, 40);

    b[SEC..SEC + 5].copy_from_slice(b".text");
    w32(&mut b, SEC + 8, 0x400);
    w32(&mut b, SEC + 12, TEXT_RVA);
    w32(&mut b, SEC + 16, 0x400);
    w32(&mut b, SEC + 20, TEXT_RAW as u32);
    w32(&mut b, SEC + 36, 0x6000_0020);
    let s1 = SEC + 40;
    b[s1..s1 + 5].copy_from_slice(b".data");
    w32(&mut b, s1 + 8, 0x800);
    w32(&mut b, s1 + 12, DATA_RVA);
    w32(&mut b, s1 + 16, 0x800);
    w32(&mut b, s1 + 20, DATA_RAW as u32);
    w32(&mut b, s1 + 36, 0xc000_0040);

    let d = data_at(DATA_RVA);
    w32(&mut b, d, 0x2100);
    w32(&mut b, d + 12, 0x2300);
    w32(&mut b, d + 16, 0x2200);
    put(&mut b, 0x2300, b"kernel32.dll\0");
    let mut nm = vec![0u8, 0];
    nm.extend_from_slice(name.as_bytes());
    nm.push(0);
    put(&mut b, 0x2400, &nm);
    w64(&mut b, data_at(0x2100), 0x2400);
    w64(&mut b, data_at(0x2200), 0x2400);

    let t = TEXT_RAW;
    let code: &[(u32, Vec<u8>)] = &[
        (0x1000, [&[0xe8][..], &rel(0x1005, 0x1040)].concat()),
        (0x1005, vec![0xc3]),
        (0x1040, [&[0xff, 0x25][..], &rel(0x1046, 0x2200)].concat()),
    ];
    for (rva, bytes) in code {
        let o = t + (rva - TEXT_RVA) as usize;
        b[o..o + bytes.len()].copy_from_slice(bytes);
    }
    b
}

#[test]
fn local_free_thunk_calls_runtime() {
    let img = build_with("LocalFree");
    let mut out = Vec::new();
    let st = generate(&img, &mut out).unwrap();
    let c = String::from_utf8(out).unwrap();

    let lf = IB + 0x1040;

    assert_eq!(st.handled["jmp_imp:LocalFree"], 1);
    assert!(c.contains("fle_imp_LocalFree(); return;"));
    assert!(!c.contains(&format!("L_{lf:x}: {{ /* {lf:#x} Jmp */ fle_unsupported")));
}

/// LocalFree runtime (fle_imports.c): NULL is ignored and returns NULL; a LocalAlloc
/// pointer is freed and returns NULL. The C program reads only its own output.
#[test]
fn local_free_runtime_returns_null_and_frees() {
    let rt = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../runtime");
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../out-verify/test");
    std::fs::create_dir_all(dir.join("tmp")).unwrap();

    let src = "#include <stdio.h>\n#include \"fle_runtime.h\"\n#include \"fle_imports.h\"\n\n\
uint64_t R[16];\n\
int main(void)\n{\n\
    R[1] = 0; fle_imp_LocalFree(); printf(\"%llx\\n\", (unsigned long long)R[0]);\n\
    R[1] = (uint64_t)(uintptr_t)malloc(16); fle_imp_LocalFree(); printf(\"%llx\\n\", (unsigned long long)R[0]);\n\
    R[1] = 0x40; R[2] = 8; fle_imp_LocalAlloc(); \
    R[1] = R[0]; fle_imp_LocalFree(); printf(\"%llx\\n\", (unsigned long long)R[0]);\n\
    return 0;\n}\n";

    let tag = etxt_common::uniq("local-free");
    let c = dir.join(format!("{tag}.c"));
    let exe = dir.join(&tag);
    std::fs::write(&c, src).unwrap();
    let cc = Command::new("clang")
        .args(["-O0", "-std=gnu11", "-Wall", "-Wextra", "-fPIE", "-I"])
        .arg(&rt)
        .arg(rt.join("fle_imports.c"))
        .arg(rt.join("fle_string.c"))
        .arg("-o")
        .arg(&exe)
        .arg(&c)
        .env("TMPDIR", dir.join("tmp"))
        .output()
        .expect("clang not found");
    assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
    let run = Command::new(&exe).output().expect("run failed");
    assert!(run.status.success(), "C program failed");
    let got: Vec<String> = String::from_utf8(run.stdout).unwrap().lines().map(|l| l.trim().to_string()).collect();
    assert_eq!(got, vec!["0", "0", "0"]);
}

#[test]
fn verify_version_info_thunk_is_handled_since_decision_0014() {
    let img = build();
    let mut out = Vec::new();
    let st = generate(&img, &mut out).unwrap();
    let c = String::from_utf8(out).unwrap();

    // decision 0012 kept this thunk unsupported; decision 0014 (emulated OS) supersedes it.
    assert!(st.handled.contains_key("jmp_imp:VerifyVersionInfoW"));
    assert!(c.contains("fle_imp_VerifyVersionInfoW"));
    assert!(!st.unsupported.contains_key("Jmp: indirect through IAT slot (not lifted)"));
}
