//! Landing of the guest's setjmp (decision 0100, docs/decisions/0100-wall-lift-gaps.md; src/lib.rs `succ`, runtime/fle_dispatch.h
//! `fle_landing_*`): the code after a call of a register-saving setjmp runs a second time, in the C frame of the caller, when the guest's
//! longjmp (which ends in `jmp [buf]`, a dispatch to the middle of a function) is called from frames below it.
//!
//! Synthetic PE, .text at 0x1000, assembled from the listing below with `clang -c` (intel syntax) and `objcopy -O binary`; the bytes are
//! the ones of lift/out-wallL/asm/sj4.s. `setjmp_fn` and `longjmp_fn` have the shapes of the guest's own (FLEngine 0x412680 and 0x4126f0,
//! without the xmm registers); `tramp_setjmp` is the guest's trampoline (0x412760: `lea rcx,[rcx+d]; jmp setjmp`).
//!
//! Each program runs twice, on the lifted C and on the host CPU (the same bytes in an executable page, called with rcx = jmp_buf and
//! rdx = marks), and the results must be equal and equal to the values derived below:
//! - main: setjmp returns 0, rbx is clobbered, mid -> mid2 -> longjmp(buf, 7). The code after the call in mid and mid2 does not run (marks[7],
//!   marks[9] stay 0), nor the code after the call of mid in main (marks[1]). The landing runs with eax = 7, rbx = 0x1111 (restored),
//!   and the rsp that main had before the call of setjmp; main returns eax = 7.
//! - main2: no longjmp. setjmp returns 0, the normal path runs, main2 returns 0.
//! - main3: setjmp through the trampoline (buf+0x10), longjmp(7) lands, and the landing longjmps again with 9 to the same frame: two landings
//!   (marks[2] = 2), the last eax is 9 (marks[3] = 9, return 9).
//! - main4 (lifted only): the saved rsp is corrupted before the longjmp, so no live setjmp frame has the restored rsp: the run stops with
//!   the named message.
//!
//! The listing:
//!
//!     .intel_syntax noprefix
//!     .text
//!     .globl start
//!     .org 0x000
//! start:
//!     call main
//!     call main2
//!     call main3
//!     call main4
//!     ret
//!
//!     .org 0x100
//! main:
//!     push rbx
//!     push rdi
//!     sub rsp, 0x28
//!     mov rdi, rdx
//!     mov rbx, 0x1111
//!     mov qword ptr [rdi+0x28], rsp
//!     call setjmp_fn
//!     test eax, eax
//!     jne landed
//!     mov qword ptr [rdi], 1
//!     mov rbx, 0x2222
//!     mov edx, 7
//!     call mid
//!     mov qword ptr [rdi+8], 0x99
//!     jmp done
//! landed:
//!     mov dword ptr [rdi+0x10], eax
//!     mov qword ptr [rdi+0x18], rbx
//!     mov qword ptr [rdi+0x20], rsp
//! done:
//!     add rsp, 0x28
//!     pop rdi
//!     pop rbx
//!     ret
//!
//!     .org 0x200
//! mid:
//!     push rbx
//!     push rbp
//!     sub rsp, 0x38
//!     mov rbx, 0x3333
//!     mov rbp, 0x4444
//!     mov qword ptr [rdi+0x30], 1
//!     call mid2
//!     mov qword ptr [rdi+0x38], 0x99
//!     add rsp, 0x38
//!     pop rbp
//!     pop rbx
//!     ret
//!
//!     .org 0x280
//! mid2:
//!     sub rsp, 0x28
//!     mov qword ptr [rdi+0x40], 1
//!     call longjmp_fn
//!     mov qword ptr [rdi+0x48], 0x99
//!     add rsp, 0x28
//!     ret
//!
//!     .org 0x300
//! setjmp_fn:
//!     mov rax, qword ptr [rsp]
//!     lea rdx, [rsp+8]
//!     mov qword ptr [rcx], rax
//!     mov qword ptr [rcx+8], rdx
//!     mov qword ptr [rcx+0x10], rbx
//!     mov qword ptr [rcx+0x18], rbp
//!     mov qword ptr [rcx+0x20], rsi
//!     mov qword ptr [rcx+0x28], rdi
//!     mov qword ptr [rcx+0x30], r12
//!     mov qword ptr [rcx+0x38], r13
//!     mov qword ptr [rcx+0x40], r14
//!     mov qword ptr [rcx+0x48], r15
//!     xor eax, eax
//!     ret
//!
//!     .org 0x380
//! tramp_setjmp:
//!     lea rcx, [rcx+0x10]
//!     jmp setjmp_fn
//!
//!     .org 0x400
//! longjmp_fn:
//!     mov eax, edx
//!     mov rbx, qword ptr [rcx+0x10]
//!     mov rbp, qword ptr [rcx+0x18]
//!     mov rsi, qword ptr [rcx+0x20]
//!     mov rdi, qword ptr [rcx+0x28]
//!     mov r12, qword ptr [rcx+0x30]
//!     mov r13, qword ptr [rcx+0x38]
//!     mov r14, qword ptr [rcx+0x40]
//!     mov r15, qword ptr [rcx+0x48]
//!     mov rsp, qword ptr [rcx+8]
//!     jmp qword ptr [rcx]
//!
//!     .org 0x500
//! main2:
//!     push rbx
//!     push rdi
//!     sub rsp, 0x28
//!     mov rdi, rdx
//!     mov rbx, 0x1111
//!     call setjmp_fn
//!     test eax, eax
//!     jne done2
//!     mov qword ptr [rdi], 1
//!     mov rbx, 0x2222
//! done2:
//!     add rsp, 0x28
//!     pop rdi
//!     pop rbx
//!     ret
//!
//!     .org 0x600
//! main3:
//!     push rbx
//!     push rdi
//!     sub rsp, 0x28
//!     mov rdi, rdx
//!     mov rbx, 0x1111
//!     call tramp_setjmp
//!     test eax, eax
//!     jne landed3
//!     mov qword ptr [rdi], 1
//!     mov rbx, 0x2222
//!     mov edx, 7
//!     call longjmp_fn
//!     mov qword ptr [rdi+8], 0x99
//!     jmp done3
//! landed3:
//!     add dword ptr [rdi+0x10], 1
//!     cmp eax, 7
//!     jne done3
//!     mov edx, 9
//!     call longjmp_fn
//!     mov qword ptr [rdi+0x58], 0x99
//! done3:
//!     mov dword ptr [rdi+0x18], eax
//!     add rsp, 0x28
//!     pop rdi
//!     pop rbx
//!     ret
//!
//!     .org 0x700
//! main4:
//!     push rbx
//!     push rdi
//!     sub rsp, 0x28
//!     mov rdi, rdx
//!     call setjmp_fn
//!     test eax, eax
//!     jne done4
//!     sub qword ptr [rcx+8], 8
//!     mov edx, 1
//!     call longjmp_fn
//! done4:
//!     add rsp, 0x28
//!     pop rdi
//!     pop rbx
//!     ret

mod etxt_common;

use fe_lift::generate_with_seeds;
use std::path::PathBuf;
use std::process::Command;

const IB: u64 = 0x1_4000_0000;
const LFANEW: usize = 0x80;
const OPT: usize = LFANEW + 24;
const SEC: usize = OPT + 240;
const TEXT_RVA: u32 = 0x1000;
const TEXT_RAW: usize = 0x400;
const TEXT_LEN: usize = 0x800;
const FILE_LEN: usize = 0xc00;

const CODE: &[(u32, &str)] = &[
        (0x0, "e8fb000000e8f6040000e8f1050000e8ec060000c3"),
        (0x100, "53574883ec284889d748c7c31111000048896728e8e701000085c0752248c7070100000048c7c322220000ba07000000e8cb00000048c7470899000000eb0b89471048895f18488967204883c4285f5bc3"),
        (0x200, "53554883ec3848c7c33333000048c7c54444000048c7473001000000e85f00000048c74738990000004883c4385d5bc3"),
        (0x280, "4883ec2848c7474001000000e86f01000048c74748990000004883c428c3"),
        (0x300, "488b0424488d54240848890148895108488959104889691848897120488979284c8961304c8969384c8971404c89794831c0c3"),
        (0x380, "488d4910e977ffffff"),
        (0x400, "89d0488b5910488b6918488b7120488b79284c8b61304c8b69384c8b71404c8b7948488b6108ff21"),
        (0x500, "53574883ec284889d748c7c311110000e8ebfdffff85c0750e48c7070100000048c7c3222200004883c4285f5bc3"),
        (0x600, "53574883ec284889d748c7c311110000e86bfdffff85c0752248c7070100000048c7c322220000ba07000000e8cffdffff48c7470899000000eb1b8347100183f8077512ba09000000e8b2fdffff48c74758990000008947184883c4285f5bc3"),
        (0x700, "53574883ec284889d7e8f2fbffff85c0750f4883690808ba01000000e8dffcffff4883c4285f5bc3"),
];

fn w16(b: &mut [u8], o: usize, v: u16) {
    b[o..o + 2].copy_from_slice(&v.to_le_bytes());
}
fn w32(b: &mut [u8], o: usize, v: u32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}
fn w64(b: &mut [u8], o: usize, v: u64) {
    b[o..o + 8].copy_from_slice(&v.to_le_bytes());
}

fn unhex(h: &str) -> Vec<u8> {
    (0..h.len() / 2).map(|i| u8::from_str_radix(&h[2 * i..2 * i + 2], 16).unwrap()).collect()
}

fn text_bytes() -> Vec<u8> {
    let mut t = vec![0xccu8; TEXT_LEN];
    for (off, h) in CODE {
        let b = unhex(h);
        t[*off as usize..*off as usize + b.len()].copy_from_slice(&b);
    }
    t
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
    w32(&mut b, SEC + 8, TEXT_LEN as u32);
    w32(&mut b, SEC + 12, TEXT_RVA);
    w32(&mut b, SEC + 16, TEXT_LEN as u32);
    w32(&mut b, SEC + 20, TEXT_RAW as u32);
    w32(&mut b, SEC + 36, 0x6000_0020);
    b[TEXT_RAW..TEXT_RAW + TEXT_LEN].copy_from_slice(&text_bytes());
    b
}

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn out_dir() -> PathBuf {
    let d = crate_dir().join("../../out-wallL/test");
    std::fs::create_dir_all(d.join("tmp")).expect("create test output dir");
    d
}

fn c_array(name: &str, b: &[u8]) -> String {
    let v: Vec<String> = b.iter().map(|x| format!("{x}")).collect();
    format!("static const unsigned char {name}[{}] = {{ {} }};\n", b.len(), v.join(","))
}

const MAIN_C: &str = r#"
#include <stdio.h>
#include <stdint.h>
#include <string.h>
#include <sys/mman.h>
extern uint64_t R[16];
void f_140001100(void);
void f_140001500(void);
void f_140001600(void);
void f_140001700(void);

static uint64_t marks[16];
static uint64_t buf[64] __attribute__((aligned(16)));
static uint64_t gstack[1024] __attribute__((aligned(16)));

static void clear(void)
{
    memset(marks, 0, sizeof marks);
    memset(buf, 0, sizeof buf);
    memset(gstack, 0xa5, sizeof gstack);
}

static void show(const char *who, uint64_t rax)
{
    int i;
    printf("%s rax=%llx marks=", who, (unsigned long long)(rax & 0xffffffff));
    for (i = 0; i < 10; i++) {
        uint64_t m = marks[i];
        if (i == 4 || i == 5) m = 0; /* stack addresses differ between the runs: only their equality is shown */
        printf("%llx%s", (unsigned long long)m, i == 9 ? "" : ",");
    }
    printf(" rsp_eq=%d\n", marks[4] == marks[5]);
}

static uint64_t native_call(void *fn, uint64_t rcx, uint64_t rdx)
{
    uint64_t rax;
    __asm__ volatile("call *%[fn]"
                     : "=a"(rax), "+c"(rcx), "+d"(rdx)
                     : [fn] "r"(fn)
                     : "memory", "cc", "r8", "r9", "r10", "r11", "rsi", "rdi");
    return rax;
}

int main(int argc, char **argv)
{
    const char *m = argc > 1 ? argv[1] : "";
    unsigned off = !strcmp(m, "main") || !strcmp(m, "native_main") ? 0x100
                 : !strcmp(m, "main2") || !strcmp(m, "native_main2") ? 0x500
                 : !strcmp(m, "main3") || !strcmp(m, "native_main3") ? 0x600
                 : !strcmp(m, "main4") ? 0x700 : 0;
    if (!off) return 3;
    clear();
    if (!strncmp(m, "native_", 7)) {
        unsigned char *p = mmap(NULL, 4096, PROT_READ | PROT_WRITE | PROT_EXEC, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
        uint64_t rax;
        if (p == MAP_FAILED) return 4;
        memcpy(p, code, sizeof code);
        rax = native_call(p + off, (uint64_t)(uintptr_t)buf, (uint64_t)(uintptr_t)marks);
        show(m + 7, rax);
        return 0;
    }
    R[1] = (uint64_t)(uintptr_t)buf;
    R[2] = (uint64_t)(uintptr_t)marks;
    R[4] = (uint64_t)(uintptr_t)&gstack[1000];
    {
        uint64_t rsp0 = R[4];
        if (off == 0x100) f_140001100();
        else if (off == 0x500) f_140001500();
        else if (off == 0x600) f_140001600();
        else f_140001700();
        printf("rsp_back=%d\n", R[4] == rsp0);
    }
    show(m, R[0]);
    return 0;
}
"#;

fn build_exe_once(flags: &[&str], tag: &str) -> (PathBuf, String, fe_lift::Stats) {
    let pe_bytes = build();
    let pe = fe_pe::Pe::parse(&pe_bytes).expect("parse");
    let text = fe_decode::Text::from_pe(&pe).expect("text");
    let base = fe_decode::analyze_with_seeds(&pe, &[]).expect("analyze");
    let seeds: Vec<u64> = fe_decode::setjmp::landings(&text, &base.insns).into_keys().collect();
    let mut out = Vec::new();
    let st = generate_with_seeds(&pe_bytes, &seeds, &mut out).expect("generate");
    let c_text = String::from_utf8(out).expect("utf8");
    let dir = out_dir();
    let utag = etxt_common::uniq(tag);
    let src = dir.join(format!("{utag}.c"));
    std::fs::write(&src, &c_text).unwrap();
    let main_src = dir.join(format!("{utag}_main.c"));
    let code = text_bytes();
    std::fs::write(&main_src, format!("{}{}", c_array("code", &code), MAIN_C)).unwrap();
    let bin = dir.join(&utag);
    let c = Command::new("clang")
        .env("TMPDIR", dir.join("tmp"))
        .args(["-O0", "-mno-red-zone", "-w"])
        .args(flags)
        .arg("-I")
        .arg(crate_dir().join("../../runtime"))
        .args(["-o", bin.to_str().unwrap(), src.to_str().unwrap(), main_src.to_str().unwrap()])
        .output()
        .expect("run clang (is clang installed?)");
    assert!(c.status.success(), "clang failed: {}", String::from_utf8_lossy(&c.stderr));
    (bin, c_text, st)
}

/// Built once: the tests run in parallel and share the output file.
fn build_exe() -> (PathBuf, String, fe_lift::Stats) {
    static ONCE: std::sync::OnceLock<(PathBuf, String, fe_lift::Stats)> = std::sync::OnceLock::new();
    let r = ONCE.get_or_init(|| build_exe_once(&[], "setjmp_landing"));
    (r.0.clone(), r.1.clone(), fe_lift::Stats::default())
}

/// The same C built with -DFLE_LANDING_NOJUMP (the default under emscripten): no setjmp, no longjmp.
fn build_exe_nojump() -> PathBuf {
    static ONCE: std::sync::OnceLock<(PathBuf, String, fe_lift::Stats)> = std::sync::OnceLock::new();
    ONCE.get_or_init(|| build_exe_once(&["-DFLE_LANDING_NOJUMP"], "setjmp_landing_nojump")).0.clone()
}

fn run(bin: &PathBuf, mode: &str) -> std::process::Output {
    Command::new(bin).arg(mode).output().expect("run")
}

fn stdout_of(bin: &PathBuf, mode: &str) -> String {
    let r = run(bin, mode);
    let o = String::from_utf8_lossy(&r.stdout).to_string();
    assert!(r.status.success(), "{mode}: {o} {}", String::from_utf8_lossy(&r.stderr));
    o
}

fn last_line(o: &str) -> String {
    o.lines().last().unwrap().to_string()
}

#[test]
fn generator_makes_each_landing_an_entry_with_the_wrapper() {
    let (_, c, _) = build_exe();
    // main, main2, main3 and main4 each call a setjmp: four landings, each the instruction after the call
    assert_eq!(c.matches("fle_landing_push(&lj, ").count(), 4, "{}", c.lines().filter(|l| l.contains("fle_landing")).count());
    for l in [0x140001100u64 + 0x19, 0x140001500 + 0x15, 0x140001600 + 0x15, 0x140001700 + 0x0e] {
        assert!(c.contains(&format!("void f_{l:x}(void);")), "landing {l:#x} is a function");
        assert!(c.contains(&format!("fle_landing_push(&lj, UINT64_C({l:#x}), R[4]); if (FLE_LANDING_SETJMP(&lj) == 0) {{ f_{l:x}(); }} else {{ f_{l:x}(); }}")), "{l:#x}");
    }
    // the guest's longjmp ends in a memory jump, a dispatch of the saved address
    assert!(c.contains("{ fle_dispatch(fle_ld((R[1]), 8)); return; }"));
    assert!(!c.contains("fle_unsupported(UINT64_C("));
}

#[test]
fn stats_count_the_landings() {
    let pe_bytes = build();
    let pe = fe_pe::Pe::parse(&pe_bytes).unwrap();
    let text = fe_decode::Text::from_pe(&pe).unwrap();
    let base = fe_decode::analyze_with_seeds(&pe, &[]).unwrap();
    let l = fe_decode::setjmp::landings(&text, &base.insns);
    assert_eq!(l.keys().copied().collect::<Vec<_>>(), vec![0x140001100 + 0x19, 0x140001500 + 0x15, 0x140001600 + 0x15, 0x140001700 + 0x0e]);
    // the call through the trampoline is a call of setjmp too
    assert_eq!(l[&(0x140001600 + 0x15)].callee, 0x140001000 + 0x380);
    let mut out = Vec::new();
    let seeds: Vec<u64> = l.keys().copied().collect();
    let st = generate_with_seeds(&pe_bytes, &seeds, &mut out).unwrap();
    assert_eq!(st.setjmp_landings, 4);
    assert_eq!(st.unsupported_total, 0, "{:?}", st.unsupported);
    assert_eq!(st.bad_nodes, 0);
    // without the seeds nothing is an entry, so no wrapper (the generator does not guess)
    let mut out = Vec::new();
    let st = generate_with_seeds(&pe_bytes, &[], &mut out).unwrap();
    assert_eq!(st.setjmp_landings, 0);
}

#[test]
fn main_lands_with_the_value_the_restored_registers_and_the_discarded_frames_unrun() {
    let (bin, _, _) = build_exe();
    let o = stdout_of(&bin, "main");
    // marks: [0]=1 first return seen, [1]=0 not run, [2]=7 value at the landing, [3]=1111 rbx restored, [4],[5] shown as 0 (rsp_eq below),
    // [6]=1 mid ran, [7]=0 discarded, [8]=1 mid2 ran, [9]=0 discarded
    assert_eq!(last_line(&o), "main rax=7 marks=1,0,7,1111,0,0,1,0,1,0 rsp_eq=1", "{o}");
    assert!(o.contains("rsp_back=1"), "{o}");
}

#[test]
fn main2_without_a_longjmp_takes_the_normal_path() {
    let (bin, _, _) = build_exe();
    let o = stdout_of(&bin, "main2");
    assert_eq!(last_line(&o), "main2 rax=0 marks=1,0,0,0,0,0,0,0,0,0 rsp_eq=1", "{o}");
    assert!(o.contains("rsp_back=1"), "{o}");
}

#[test]
fn main3_lands_twice_in_the_same_frame_through_the_trampoline() {
    let (bin, _, _) = build_exe();
    let o = stdout_of(&bin, "main3");
    assert_eq!(last_line(&o), "main3 rax=9 marks=1,0,2,9,0,0,0,0,0,0 rsp_eq=1", "{o}");
    assert!(o.contains("rsp_back=1"), "{o}");
}

#[test]
fn the_lifted_runs_equal_the_host_runs() {
    let (bin, _, _) = build_exe();
    for m in ["main", "main2", "main3"] {
        let lifted = last_line(&stdout_of(&bin, m));
        let host = last_line(&stdout_of(&bin, &format!("native_{m}")));
        assert_eq!(lifted, host, "{m}");
    }
}

#[test]
fn a_longjmp_with_a_restored_rsp_that_no_live_setjmp_frame_has_stops_by_name() {
    let (bin, _, _) = build_exe();
    let r = run(&bin, "main4");
    assert!(!r.status.success());
    let e = String::from_utf8_lossy(&r.stderr).to_string();
    assert!(e.contains("the landing of a setjmp of the guest, with rsp"), "{e}");
    assert!(e.contains("which no live setjmp frame has"), "{e}");
    assert!(!String::from_utf8_lossy(&r.stdout).contains("rax="), "ran on");
}

