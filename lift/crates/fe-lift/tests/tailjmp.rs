//! `jmp r64` with an unknown target as a tail dispatch (decision 0036, W1; src/tailjmp.rs).
//!
//! Synthetic PE (.text at 0x1000):
//! - 0x1000 `main`: calls the five functions below, so each is a function entry (never run)
//! - 0x1020 T1: `test rcx,rcx; jz skip; mov rax,rcx; jmp rax; skip: mov eax,0x99; ret`
//! - 0x1030 T2: `mov r11,rdx; jmp r11`
//! - 0x1040 T3: `jmp [rcx]` (memory form: a tail dispatch of the value read, decision 0100; decision 0036 left it unsupported)
//! - 0x1050 C1: `mov eax,0x11; ret`
//! - 0x1060 C2: `mov eax,0x22; ret`
//!
//! Checks, on the generated C built by clang -O0 and run:
//! - T1 / T2 reach the function whose VA is in the register (rcx / rdx -> r11), and not the one in rax / r11's
//!   neighbour registers (the registers hold the other function's VA on purpose);
//! - RSP and the stack are untouched (the guest return address stays on the stack, the callee's `ret` is `return;`);
//! - a register value that is not a function entry aborts in `fle_dispatch_miss`;
//! - `jmp [rcx]` reaches the function whose VA is in the slot that rcx points to, with the stack untouched, and a slot value that is not an
//!   entry aborts in `fle_dispatch_miss` (decision 0100);
//! - the generator counts the two register jumps as `jmp_ind:reg_dispatch` and in `register_jmps`, the memory jump as `jmp_ind:mem_dispatch`
//!   (not a register jump), and nothing is unsupported.
//!
//! Outputs go under lift/out-ijmp/test.

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

    let call = |at: u32, to: u32| [&[0xe8][..], &rel(at + 5, to)].concat();
    let mut main = Vec::new();
    for (k, to) in [0x1020u32, 0x1030, 0x1040, 0x1050, 0x1060].into_iter().enumerate() {
        main.extend(call(0x1000 + 5 * k as u32, to));
    }
    main.push(0xc3);
    let code: &[(u32, Vec<u8>)] = &[
        (0x1000, main),
        // test rcx,rcx; jz 0x102a; mov rax,rcx; jmp rax; 0x102a: mov eax,0x99; ret. The block after the jmp is
        // what runs if the lifted jmp falls through instead of returning.
        (0x1020, vec![0x48, 0x85, 0xc9, 0x74, 0x05, 0x48, 0x89, 0xc8, 0xff, 0xe0, 0xb8, 0x99, 0x00, 0x00, 0x00, 0xc3]),
        (0x1030, vec![0x49, 0x89, 0xd3, 0x41, 0xff, 0xe3]),
        (0x1040, vec![0xff, 0x21]),
        (0x1050, vec![0xb8, 0x11, 0x00, 0x00, 0x00, 0xc3]),
        (0x1060, vec![0xb8, 0x22, 0x00, 0x00, 0x00, 0xc3]),
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
    let d = crate_dir().join("../../out-ijmp/test");
    std::fs::create_dir_all(d.join("tmp")).expect("create test output dir");
    d
}

const MAIN_C: &str = r#"
#include <stdio.h>
#include <stdint.h>
#include <string.h>
extern uint64_t R[16];
void f_140001020(void);
void f_140001030(void);
void f_140001040(void);

int main(int argc, char **argv)
{
    static uint64_t stack[8];
    static uint64_t slot[2];
    const uint64_t C1 = UINT64_C(0x140001050), C2 = UINT64_C(0x140001060);
    const char *m = argc > 1 ? argv[1] : "";
    int i;
    for (i = 0; i < 8; i++) stack[i] = UINT64_C(0xfeed0000) + i;
    R[4] = (uint64_t)(uintptr_t)&stack[4];
    R[0] = 0xdeadbeef;
    if (!strcmp(m, "t1_c1")) { R[1] = C1; f_140001020(); }
    else if (!strcmp(m, "t1_c2")) { R[1] = C2; R[0] = C1; f_140001020(); }
    else if (!strcmp(m, "t2_c2")) { R[2] = C2; R[0] = C1; R[1] = C1; R[11] = C1; f_140001030(); }
    else if (!strcmp(m, "t2_c1")) { R[2] = C1; R[0] = C2; R[1] = C2; R[11] = C2; f_140001030(); }
    else if (!strcmp(m, "miss")) { R[1] = UINT64_C(0x1234); f_140001020(); }
    else if (!strcmp(m, "miss_mid")) { R[1] = C1 + 1; f_140001020(); }
    else if (!strcmp(m, "mem")) { R[1] = (uint64_t)(uintptr_t)&stack[0]; f_140001040(); }
    else if (!strcmp(m, "mem_c1")) { slot[0] = C1; slot[1] = C2; R[1] = (uint64_t)(uintptr_t)&slot[0]; f_140001040(); }
    else if (!strcmp(m, "mem_c2")) { slot[0] = C1; slot[1] = C2; R[0] = C1; R[1] = (uint64_t)(uintptr_t)&slot[1]; f_140001040(); }
    else return 3;
    {
        int ok = R[4] == (uint64_t)(uintptr_t)&stack[4];
        for (i = 0; i < 8; i++) ok = ok && stack[i] == UINT64_C(0xfeed0000) + i;
        printf("eax=%llx stack_ok=%d\n", (unsigned long long)(R[0] & 0xffffffff), ok);
    }
    return 0;
}
"#;

/// Built once: the tests run in parallel and share the output file.
fn build_exe(tag: &str) -> (PathBuf, String, fe_lift::Stats) {
    static ONCE: std::sync::OnceLock<(PathBuf, String, fe_lift::Stats)> = std::sync::OnceLock::new();
    let r = ONCE.get_or_init(|| build_exe_once(tag));
    (r.0.clone(), r.1.clone(), fe_lift::Stats::default())
}

fn build_exe_once(tag: &str) -> (PathBuf, String, fe_lift::Stats) {
    let mut out = Vec::new();
    let st = generate(&build(), &mut out).expect("generate");
    let c_text = String::from_utf8(out).expect("utf8");
    let dir = out_dir();
    let utag = etxt_common::uniq(tag);
    let src = dir.join(format!("{utag}.c"));
    std::fs::write(&src, &c_text).unwrap();
    let main_src = dir.join(format!("{utag}_main.c"));
    std::fs::write(&main_src, MAIN_C).unwrap();
    let bin = dir.join(&utag);
    let c = Command::new("clang")
        .env("TMPDIR", dir.join("tmp"))
        .args(["-O0", "-mno-red-zone", "-w", "-I"])
        .arg(crate_dir().join("../../runtime"))
        .args(["-o", bin.to_str().unwrap(), src.to_str().unwrap(), main_src.to_str().unwrap()])
        .output()
        .expect("run clang (is clang installed?)");
    assert!(c.status.success(), "clang failed: {}", String::from_utf8_lossy(&c.stderr));
    (bin, c_text, st)
}

#[test]
fn generator_counts_and_text() {
    let mut out = Vec::new();
    let st = generate(&build(), &mut out).expect("generate");
    let c = String::from_utf8(out).unwrap();
    assert_eq!(st.handled["jmp_ind:reg_dispatch"], 2, "{:?}", st.handled);
    assert_eq!(st.register_jmps["jmp_ind:reg_dispatch"], 2, "{:?}", st.register_jmps);
    assert_eq!(st.register_jmps.len(), 1, "{:?}", st.register_jmps);
    assert_eq!(st.register_jmp_sites, 2);
    assert!(c.contains("{ fle_dispatch(R[0]); return; }"), "T1");
    assert!(c.contains("{ fle_dispatch(R[11]); return; }"), "T2");
    // the memory form (decision 0100): the value is read before the dispatch; it is not a register jump
    assert_eq!(st.handled["jmp_ind:mem_dispatch"], 1, "{:?}", st.handled);
    assert!(!st.register_jmps.contains_key("jmp_ind:mem_dispatch"), "{:?}", st.register_jmps);
    assert!(st.unsupported.is_empty(), "{:?}", st.unsupported);
    assert_eq!(st.unsupported_total, 0);
    assert!(c.contains("{ fle_dispatch(fle_ld((R[1]), 8)); return; }"), "T3");
    assert!(!c.contains("fle_unsupported(UINT64_C("));
}

fn run(bin: &PathBuf, mode: &str) -> std::process::Output {
    Command::new(bin).arg(mode).output().expect("run")
}

#[test]
fn register_jmp_reaches_the_function_in_the_register_and_keeps_the_stack() {
    let (bin, _, _) = build_exe("tailjmp");
    for (mode, want) in [("t1_c1", "eax=11"), ("t1_c2", "eax=22"), ("t2_c2", "eax=22"), ("t2_c1", "eax=11")] {
        let r = run(&bin, mode);
        let o = String::from_utf8_lossy(&r.stdout).to_string();
        assert!(r.status.success(), "{mode}: {o} {}", String::from_utf8_lossy(&r.stderr));
        assert!(o.starts_with(want), "{mode}: {o}");
        assert!(o.contains("stack_ok=1"), "{mode}: {o}");
    }
}

#[test]
fn register_jmp_to_a_non_entry_aborts() {
    let (bin, _, _) = build_exe("tailjmp");
    for mode in ["miss", "miss_mid"] {
        let r = run(&bin, mode);
        assert!(!r.status.success(), "{mode} did not stop");
        let e = String::from_utf8_lossy(&r.stderr).to_string();
        assert!(e.contains("not a known function entry"), "{mode}: {e}");
        assert!(!String::from_utf8_lossy(&r.stdout).contains("eax="), "{mode} ran on");
    }
}

#[test]
fn memory_form_jmp_reaches_the_function_in_the_slot_and_keeps_the_stack() {
    let (bin, _, _) = build_exe("tailjmp");
    for (mode, want) in [("mem_c1", "eax=11"), ("mem_c2", "eax=22")] {
        let r = run(&bin, mode);
        let o = String::from_utf8_lossy(&r.stdout).to_string();
        assert!(r.status.success(), "{mode}: {o} {}", String::from_utf8_lossy(&r.stderr));
        assert!(o.starts_with(want), "{mode}: {o}");
        assert!(o.contains("stack_ok=1"), "{mode}: {o}");
    }
}

#[test]
fn memory_form_jmp_to_a_non_entry_aborts() {
    let (bin, _, _) = build_exe("tailjmp");
    let r = run(&bin, "mem");
    assert!(!r.status.success());
    let e = String::from_utf8_lossy(&r.stderr).to_string();
    assert!(e.contains("indirect call to 0xfeed0000 is not a known function entry"), "{e}");
    assert!(!String::from_utf8_lossy(&r.stdout).contains("eax="), "ran on");
}
