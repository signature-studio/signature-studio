//! Inline hook points (decision 0105; src/hook.rs, `generate_with_hooks`, `fe-lift --hook-points`, runtime/fle_vmap.c).
//!
//! The guest hooks one of its own functions by overwriting its first 5 bytes with `jmp rel32` (FLEngine unit 314, QuickFontCache unit 162).
//! The lifted C does not read instructions from memory, so the function entry gets a guard: the 5 bytes of the image unchanged -> the body;
//! `e9 rel32` -> a tail dispatch to the target (what the CPU does: the stack and the registers are those of the entry); anything else -> a stop
//! by name. VirtualProtect answers for exactly the annotated 5-byte ranges of a registered image and stops for any other address of it.
//!
//! Synthetic PE (ImageBase 0x10000000, .text at RVA 0x1000, read + execute):
//! - 0x1000 `main`: calls the four functions below, so each is a function entry (never run)
//! - 0x1020 H1 (hook point): `mov eax,0x11; ret`            (the first 5 bytes are the `mov`)
//! - 0x1040 T1 (the guest's replacement): `lea rax,[rcx+0x22]; ret`
//! - 0x1060 H2 (hook point, the same page as H1): `push rbx; sub rsp,0x20; mov rbx,rcx; mov eax,0x33; add rsp,0x20; pop rbx; ret`
//! - 0x1080 N1 (not annotated): `mov eax,0x44; ret`
//!
//! Expected values are written from the semantics of `jmp rel32` (the target is the address after the 5 bytes plus the signed displacement), from
//! the Microsoft pages (VirtualProtect: the old protection is that of the first page; "Memory Protection Constants": protection belongs to whole
//! pages) and from the decision, not read back from the C code. That a read + execute section is PAGE_EXECUTE_READ (0x20) is an inference
//! (decision 0105). The host is Linux: nothing is compared with a real Windows.
//!
//! 1. Generator: the guard, the include, the table and the constructor are the only differences from the output without hook points, which
//!    is the old output byte for byte; the mistakes of the annotation stop the generator; the command line reads VA and RVA lines.
//! 2. Runtime, through a driver linked with every runtime source and the generated C: the image is mapped at its ImageBase and the driver
//!    writes `jmp rel32` and the original bytes the way the guest does.
//!
//! Outputs go under lift/out-hook/test.

mod etxt_common;

use fe_lift::{generate, generate_with_hooks};
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

const IB: u64 = 0x1000_0000;
const LFANEW: usize = 0x80;
const OPT: usize = LFANEW + 24;
const SEC: usize = OPT + 240;
const TEXT_RVA: u32 = 0x1000;
const TEXT_RAW: usize = 0x400;
const FILE_LEN: usize = 0x800;
const SIZE_OF_IMAGE: u32 = 0x2000;
/// The bytes of .text that the driver copies into its mapping of the image.
const TEXT_USED: usize = 0x90;

const H1: u32 = 0x1020;
const T1: u32 = 0x1040;
const H2: u32 = 0x1060;
const N1: u32 = 0x1080;
const H1_VA: u64 = IB + H1 as u64;
const H2_VA: u64 = IB + H2 as u64;

const RX: u32 = 0x6000_0020;

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

/// The synthetic PE. `h1` replaces the bytes of H1, `chars` is the characteristics of .text.
fn build_with(h1: Option<&[u8]>, chars: u32) -> Vec<u8> {
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
    w32(&mut b, o + 56, SIZE_OF_IMAGE);
    w32(&mut b, o + 60, 0x400);
    w16(&mut b, o + 68, 3);
    w32(&mut b, o + 108, 16);

    b[SEC..SEC + 5].copy_from_slice(b".text");
    w32(&mut b, SEC + 8, 0x400);
    w32(&mut b, SEC + 12, TEXT_RVA);
    w32(&mut b, SEC + 16, 0x400);
    w32(&mut b, SEC + 20, TEXT_RAW as u32);
    w32(&mut b, SEC + 36, chars);

    let call = |at: u32, to: u32| [&[0xe8][..], &rel(at + 5, to)].concat();
    let mut main = Vec::new();
    for (k, to) in [H1, T1, H2, N1].into_iter().enumerate() {
        main.extend(call(0x1000 + 5 * k as u32, to));
    }
    main.push(0xc3);
    let code: Vec<(u32, Vec<u8>)> = vec![
        (0x1000, main),
        (H1, h1.map(|x| x.to_vec()).unwrap_or_else(|| vec![0xb8, 0x11, 0x00, 0x00, 0x00, 0xc3])),
        (T1, vec![0x48, 0x8d, 0x41, 0x22, 0xc3]),
        (H2, vec![0x53, 0x48, 0x83, 0xec, 0x20, 0x48, 0x89, 0xcb, 0xb8, 0x33, 0x00, 0x00, 0x00, 0x48, 0x83, 0xc4, 0x20, 0x5b, 0xc3]),
        (N1, vec![0xb8, 0x44, 0x00, 0x00, 0x00, 0xc3]),
    ];
    for (rva, bytes) in code {
        let at = TEXT_RAW + (rva - TEXT_RVA) as usize;
        b[at..at + bytes.len()].copy_from_slice(&bytes);
    }
    b
}

fn build() -> Vec<u8> {
    build_with(None, RX)
}

fn lift(img: &[u8], hooks: &[u64]) -> Result<(fe_lift::Stats, String), String> {
    let mut out = Vec::new();
    let st = generate_with_hooks(img, &[], &[], hooks, &fe_decode::Options::default(), &mut out)?;
    Ok((st, String::from_utf8(out).expect("utf8")))
}

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn rt_dir() -> PathBuf {
    manifest().join("../../runtime")
}
fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-hook/test");
    std::fs::create_dir_all(d.join("tmp")).expect("create test output dir");
    d
}

// ---------------------------------------------------------------------------------------------
// 1. Generator
// ---------------------------------------------------------------------------------------------

/// The guard text of a hook point, written out here (not taken from src/hook.rs): the original 5 bytes of H1 are b8 11 00 00 00 and of H2
/// 53 48 83 ec 20, little-endian in a 64-bit constant.
const GUARD_H1: &str = "{ uint64_t fle_hk = fle_hook_target(UINT64_C(0x10001020), UINT64_C(0x11b8)); if (fle_hk) { fle_dispatch(fle_hk); return; } } ";
const GUARD_H2: &str = "{ uint64_t fle_hk = fle_hook_target(UINT64_C(0x10001060), UINT64_C(0x20ec834853)); if (fle_hk) { fle_dispatch(fle_hk); return; } } ";
const TABLE: &str = "/* inline hook points (decision 0105) */\n\
static const fle_hook_point fle_hook_points_10000000[] = {\n\
\x20   { UINT64_C(0x10001020), UINT64_C(0x11b8), 0x20 },\n\
\x20   { UINT64_C(0x10001060), UINT64_C(0x20ec834853), 0x20 },\n\
};\n\
static void __attribute__((constructor)) fle_hook_init_10000000(void)\n\
{\n\
\x20   fle_hook_register(UINT64_C(0x10000000), UINT64_C(0x2000), fle_hook_points_10000000, 2);\n\
}\n";

#[test]
fn the_guards_the_include_and_the_table_are_the_only_changes() {
    let (plain_st, plain) = lift(&build(), &[]).unwrap();
    let (st, c) = lift(&build(), &[H2_VA, H1_VA]).unwrap();
    assert_eq!(st.hook_points, 2);
    assert_eq!(plain_st.hook_points, 0);
    // the guard is inside the block of the first instruction of each hook point, in front of the instruction's own text
    assert!(c.contains(&format!("L_10001020: {{ {GUARD_H1}/* 0x10001020 Mov */ ")), "{c}");
    assert!(c.contains(&format!("L_10001060: {{ {GUARD_H2}/* 0x10001060 Push */ ")), "{c}");
    assert_eq!(c.matches("fle_hook_target(").count(), 2);
    // the table and the registration follow the last function, and the include precedes the globals
    assert!(c.ends_with(&format!("}}\n\n{TABLE}")), "{c}");
    assert!(c.contains("#include \"fle_dispatch.h\"\n#include \"fle_vmap.h\"\n#include \"fle_flags_nf.h\"\n\nuint64_t R[16];"), "{c}");
    // taking the three additions out gives the output without hook points, byte for byte (no other function changes)
    let back = c
        .replace("#include \"fle_vmap.h\"\n", "")
        .replace(GUARD_H1, "")
        .replace(GUARD_H2, "")
        .replace(TABLE, "");
    assert_eq!(back, plain);
}

#[test]
fn without_hook_points_the_output_is_the_old_output() {
    let mut old = Vec::new();
    let st0 = generate(&build(), &mut old).unwrap();
    let (st, c) = lift(&build(), &[]).unwrap();
    assert_eq!(c.as_bytes(), &old[..]);
    assert_eq!(st.hook_points, 0);
    assert_eq!(st.c_bytes, st0.c_bytes);
    assert!(!c.contains("fle_hook") && !c.contains("fle_vmap.h"), "{c}");
}

#[test]
fn a_hook_point_listed_twice_is_one_point() {
    let (st, c) = lift(&build(), &[H1_VA, H1_VA]).unwrap();
    assert_eq!(st.hook_points, 1);
    assert_eq!(c.matches("fle_hook_target(").count(), 1);
    assert!(c.contains("fle_hook_points_10000000, 1);"), "{c}");
}

#[test]
fn a_mistake_in_the_annotation_stops_the_generator() {
    // not a function entry: the middle of H1, an address in no function, an address outside the image
    for va in [H1_VA + 1, IB + 0x1100, IB + 0x5000, 0x20] {
        let e = lift(&build(), &[va]).unwrap_err();
        assert!(e.contains("not a function entry"), "{va:#x}: {e}");
    }
    // the first byte of the function is already e9 (an annotation of a function that is a jump, or of a hook that is already in the file)
    let jmp = [&[0xe9][..], &rel(H1 + 5, T1)].concat();
    let e = lift(&build_with(Some(&jmp[..]), RX), &[H1_VA]).unwrap_err();
    assert!(e.contains("already starts with e9"), "{e}");
    // the same file without the annotation lifts as before (the jump is an ordinary tail call)
    assert!(lift(&build_with(Some(&jmp[..]), RX), &[H2_VA]).is_ok());
    // a section that is writable, not readable or not executable is not the read + execute section that is modelled
    for chars in [0xe000_0020u32, 0x2000_0020, 0x4000_0020, 0xe000_0020 & !0x2000_0000] {
        let e = lift(&build_with(None, chars), &[H1_VA]).unwrap_err();
        // a section without IMAGE_SCN_MEM_EXECUTE has no code for the generator at all; the others reach the check of the hook point
        assert!(e.contains("read + execute") || e.contains("no executable section"), "{chars:#x}: {e}");
    }
    let e = lift(&build_with(None, 0xe000_0020), &[H1_VA]).unwrap_err();
    assert!(e.contains("characteristics 0xe0000020") && e.contains("read + execute"), "{e}");
}

fn cli(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_fe-lift")).args(args).output().expect("run fe-lift")
}

#[test]
fn the_command_line_reads_va_and_rva_lines_with_comments() {
    let dir = work_dir();
    let tag = etxt_common::uniq("hook-cli");
    let pe = dir.join(format!("{tag}.dll"));
    let hooks = dir.join(format!("{tag}-hooks.txt"));
    let c = dir.join(format!("{tag}.c"));
    std::fs::write(&pe, build()).unwrap();
    // a VA with 0x, an RVA, a comment after a number, a comment line, an empty line
    std::fs::write(&hooks, "# the hook points of the synthetic module\n\n0x10001020  # H1, a VA\nrva:0x1060\n").unwrap();
    let o = cli(&["lift", pe.to_str().unwrap(), c.to_str().unwrap(), "--hook-points", hooks.to_str().unwrap()]);
    let out = String::from_utf8_lossy(&o.stdout).to_string();
    assert!(o.status.success(), "{out} {}", String::from_utf8_lossy(&o.stderr));
    assert!(out.contains("hook_points            2"), "{out}");
    let text = std::fs::read_to_string(&c).unwrap();
    assert!(text.contains(GUARD_H1) && text.contains(GUARD_H2) && text.ends_with(TABLE), "{text}");
    // without the option: no hook line in the statistics, and the output is the library's output without hook points
    let c2 = dir.join(format!("{tag}-plain.c"));
    let o = cli(&["lift", pe.to_str().unwrap(), c2.to_str().unwrap()]);
    assert!(o.status.success());
    assert!(!String::from_utf8_lossy(&o.stdout).contains("hook_points"));
    assert_eq!(std::fs::read_to_string(&c2).unwrap(), lift(&build(), &[]).unwrap().1);
}

/// Decision 0106: the committed hook-point file of ILTools_x64.dll (harness/hooks/iltools-hook-points.txt) against the real file. Skipped when the
/// DLL is not there (ILTOOLS_DLL, else lift/out-inventory/extracted/ILTools_x64.dll, a file that is not in the repository). The three points are
/// RVAs 0x178d0, 0x31e70, 0x31e90; their first 5 bytes are those of the three Delphi runtime functions that FLEngine's and QFC's files annotate
/// (55 56 53 c6 44 / 53 48 83 ec 20 / 55 48 83 ec 50), the generator accepts them (a closure entry, in a read + execute section, not `e9`) and
/// the statistics count three.
#[test]
fn the_iltools_hook_file_matches_the_real_dll() {
    let dll = std::env::var("ILTOOLS_DLL").map(PathBuf::from).unwrap_or_else(|_| manifest().join("../../out-inventory/extracted/ILTools_x64.dll"));
    let Ok(img) = std::fs::read(&dll) else {
        eprintln!("skipped: {} is not there", dll.display());
        return;
    };
    let hooks = manifest().join("../../harness/hooks/iltools-hook-points.txt");
    let text = std::fs::read_to_string(&hooks).unwrap();
    let rvas: Vec<u32> = text
        .lines()
        .filter_map(|l| l.split('#').next())
        .filter_map(|l| l.trim().strip_prefix("rva:"))
        .map(|h| u32::from_str_radix(h.trim().trim_start_matches("0x"), 16).unwrap())
        .collect();
    assert_eq!(rvas, [0x178d0, 0x31e70, 0x31e90]);
    // the bytes at the RVAs: the section table of the file maps RVA to file offset
    let le32 = |o: usize| u32::from_le_bytes(img[o..o + 4].try_into().unwrap());
    let nt = le32(0x3c) as usize;
    let nsec = u16::from_le_bytes(img[nt + 6..nt + 8].try_into().unwrap()) as usize;
    let opt_size = u16::from_le_bytes(img[nt + 20..nt + 22].try_into().unwrap()) as usize;
    let first = nt + 24 + opt_size;
    let at = |rva: u32| -> Vec<u8> {
        for k in 0..nsec {
            let s = first + 40 * k;
            let (vs, va, raw_size, raw) = (le32(s + 8), le32(s + 12), le32(s + 16), le32(s + 20));
            if rva >= va && rva < va + vs.max(raw_size) {
                let o = (raw + (rva - va)) as usize;
                return img[o..o + 5].to_vec();
            }
        }
        panic!("RVA {rva:#x} is in no section");
    };
    assert_eq!(at(0x178d0), [0x55, 0x56, 0x53, 0xc6, 0x44]);
    assert_eq!(at(0x31e70), [0x53, 0x48, 0x83, 0xec, 0x20]);
    assert_eq!(at(0x31e90), [0x55, 0x48, 0x83, 0xec, 0x50]);
    let dir = work_dir();
    let out = dir.join(format!("{}.c", etxt_common::uniq("hook-iltools")));
    let o = cli(&["lift", dll.to_str().unwrap(), out.to_str().unwrap(), "--pdata", "--hook-points", hooks.to_str().unwrap()]);
    let so = String::from_utf8_lossy(&o.stdout).to_string();
    assert!(o.status.success(), "{so} {}", String::from_utf8_lossy(&o.stderr));
    assert!(so.contains("hook_points            3"), "{so}");
    let _ = std::fs::remove_file(&out);
}

#[test]
fn the_command_line_stops_on_a_bad_file_and_on_a_point_that_is_not_an_entry() {
    let dir = work_dir();
    let tag = etxt_common::uniq("hook-cli-bad");
    let pe = dir.join(format!("{tag}.dll"));
    let c = dir.join(format!("{tag}.c"));
    std::fs::write(&pe, build()).unwrap();
    for (name, text, want) in [("syntax", Some("0x10001020\nrva:zz\n"), "line 2"), ("entry", Some("0x10001021\n"), "not a function entry"), ("missing", None, "")] {
        let hooks = dir.join(format!("{tag}-{name}.txt"));
        if let Some(t) = text {
            std::fs::write(&hooks, t).unwrap();
        }
        let o = cli(&["lift", pe.to_str().unwrap(), c.to_str().unwrap(), "--hook-points", hooks.to_str().unwrap()]);
        assert!(!o.status.success(), "{name}");
        let e = String::from_utf8_lossy(&o.stderr).to_string();
        assert!(e.starts_with("error: ") && e.contains(want), "{name}: {e}");
    }
}

// ---------------------------------------------------------------------------------------------
// 2. Runtime
// ---------------------------------------------------------------------------------------------

/// The driver: maps the image at its ImageBase, writes `jmp rel32` and the original bytes into it the way the guest does, and calls the lifted
/// functions and the runtime's VirtualProtect. /*PREFIX*/ is replaced by the constants and the bytes of the synthetic .text.
const DRIVER: &str = r####"#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <sys/mman.h>
#include "fle_runtime.h"
#include "fle_dispatch.h"
#include "fle_vmap.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_sys.h"
/*PREFIX*/

#define H1 (IB + 0x1020)
#define T1 (IB + 0x1040)
#define H2 (IB + 0x1060)
#define N1 (IB + 0x1080)
#define A(x) ((uint64_t)(uintptr_t)(x))
#define U(x) ((unsigned long long)(x))

static uint64_t stk[8];

static void enter(void)
{
    int i;
    for (i = 0; i < 8; i++) stk[i] = UINT64_C(0xfeed0000) + i;
    R[4] = A(&stk[4]);
    R[0] = 0xdeadbeef;
}
/* rsp is where enter() put it; the slots at and above rsp (the guest return address and the caller's frame) are untouched; the slots below
 * rsp are untouched too, which a body that pushes would not leave (a hooked call must not run the body). */
static int rsp_ok(void) { return R[4] == A(&stk[4]); }
static int above_ok(void)
{
    int i, ok = 1;
    for (i = 4; i < 8; i++) ok = ok && stk[i] == UINT64_C(0xfeed0000) + i;
    return ok;
}
static int below_ok(void)
{
    int i, ok = 1;
    for (i = 0; i < 4; i++) ok = ok && stk[i] == UINT64_C(0xfeed0000) + i;
    return ok;
}
static void map_image(void)
{
    void *p = mmap((void *)(uintptr_t)IB, 0x2000, PROT_READ | PROT_WRITE | PROT_EXEC, MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED_NOREPLACE, -1, 0);
    if (p != (void *)(uintptr_t)IB) {
        fprintf(stderr, "cannot map the image at %#llx\n", U(IB));
        exit(77);
    }
    memcpy((uint8_t *)p + 0x1000, TEXT, sizeof TEXT);
}
static void put_jmp(uint64_t at, uint64_t to)
{
    uint8_t *p = (uint8_t *)(uintptr_t)at;
    int32_t rel = (int32_t)(to - (at + 5));
    p[0] = 0xe9;
    memcpy(p + 1, &rel, 4);
}
static void restore5(uint64_t va) { memcpy((void *)(uintptr_t)va, TEXT + (va - IB - 0x1000), 5); }
static uint64_t orig5(uint64_t va)
{
    uint64_t v = 0;
    memcpy(&v, TEXT + (va - IB - 0x1000), 5);
    return v;
}
/* The guest return address is at [rsp]; arguments in rcx (R[1]), rdx, r8, r9 (R[2], R[8], R[9]). */
static void call(void (*f)(void), uint64_t rcx)
{
    enter();
    R[1] = rcx;
    f();
}
static void show(const char *what)
{
    printf("%s rax=%llx rbx=%llx rsp_ok=%d above_ok=%d below_ok=%d\n", what, U(R[0]), U(R[3]), rsp_ok(), above_ok(), below_ok());
}
static uint64_t vp(uint64_t addr, uint64_t size, uint32_t prot, uint32_t *old)
{
    enter();
    R[1] = addr;
    R[2] = size;
    R[8] = prot;
    R[9] = A(old);
    fle_imp_VirtualProtect();
    return R[0];
}
/* VirtualProtect and print the result and the old protection it stored (the order of evaluation of printf's arguments is not defined). */
static void vpr(const char *what, uint64_t addr, uint64_t size, uint32_t prot)
{
    uint32_t old = 0xff;
    uint64_t r = vp(addr, size, prot, &old);
    printf("%s %llu old %x\n", what, U(r), old);
}
static uint32_t last_error(void)
{
    uint64_t r = R[0];
    uint32_t e;
    fle_imp_GetLastError();
    e = (uint32_t)R[0];
    R[0] = r;
    return e;
}

int main(int argc, char **argv)
{
    const char *m = argc > 1 ? argv[1] : "";
    uint32_t old = 0xff;
    int i;

    map_image();
    R[3] = 0xabc;
    if (!strcmp(m, "plain")) {
        call(f_10001020, 5); show("h1");
        call(f_10001060, 7); show("h2");
        call(f_10001080, 9); show("n");
    } else if (!strcmp(m, "hooked")) {
        put_jmp(H1, T1);
        call(f_10001020, 5); show("h1");
        put_jmp(H2, T1);
        call(f_10001060, 0x100); show("h2");
    } else if (!strcmp(m, "dispatch")) {
        put_jmp(H1, T1);
        enter(); R[1] = 5; fle_dispatch(H1); show("h1");
    } else if (!strcmp(m, "restored")) {
        put_jmp(H1, T1);
        call(f_10001020, 5); show("hooked");
        restore5(H1);
        call(f_10001020, 5); show("restored");
    } else if (!strcmp(m, "rehook")) {
        for (i = 0; i < 3; i++) {
            put_jmp(H2, T1);
            call(f_10001060, 1); show("on");
            restore5(H2);
            call(f_10001060, 1); show("off");
        }
    } else if (!strcmp(m, "target-not-entry")) {
        put_jmp(H1, H1 + 1);
        call(f_10001020, 5);
        printf("returned\n");
    } else if (!strcmp(m, "garbage")) {
        memset((void *)(uintptr_t)H1, 0xcc, 5);
        call(f_10001020, 5);
        printf("returned\n");
    } else if (!strcmp(m, "partial")) {
        *(uint8_t *)(uintptr_t)(H1 + 4) = 1;
        call(f_10001020, 5);
        printf("returned\n");
    } else if (!strcmp(m, "target-zero")) {
        put_jmp(H1, 0);
        call(f_10001020, 5);
        printf("returned\n");
    } else if (!strcmp(m, "unguarded")) {
        put_jmp(N1, T1);
        call(f_10001080, 9); show("n");
    } else if (!strcmp(m, "target-direct")) {
        uint64_t t;
        t = fle_hook_target(H1, orig5(H1)); printf("plain %llx\n", U(t));
        put_jmp(H1, T1); t = fle_hook_target(H1, orig5(H1)); printf("forward %d\n", t == T1);
        put_jmp(H2, H1); t = fle_hook_target(H2, orig5(H2)); printf("backward %d\n", t == H1);
        put_jmp(H2, H2 - 0x40000000); t = fle_hook_target(H2, orig5(H2)); printf("far-back %d\n", t == H2 - 0x40000000);
        put_jmp(H2, H2 + 0x40000000); t = fle_hook_target(H2, orig5(H2)); printf("far-fwd %d\n", t == H2 + 0x40000000);
    } else if (!strcmp(m, "vp-old")) {
        vpr("a", H1, 5, 0x40);
        vpr("b", H1, 5, 0x40);
        vpr("c", H1, 5, 0x20);
        vpr("d", H1, 5, 0x20);
    } else if (!strcmp(m, "vp-shared-page")) {
        vpr("h1", H1, 5, 0x40);
        vpr("h2", H2, 5, 0x40);
    } else if (!strcmp(m, "vp-sizes")) {
        for (i = 1; i <= 5; i++) printf("size %d -> %llu\n", i, U(vp(H1, (uint64_t)i, 0x40, &old)));
    } else if (!strcmp(m, "vp-size6")) {
        vp(H1, 6, 0x40, &old);
        printf("returned\n");
    } else if (!strcmp(m, "vp-null-old")) {
        uint64_t r = vp(H1, 5, 0x40, NULL);
        printf("r %llu le %u\n", U(r), last_error());
    } else if (!strcmp(m, "vp-nonhook")) {
        vp(N1, 5, 0x40, &old);
        printf("returned\n");
    } else if (!strcmp(m, "vp-inside")) {
        vp(H1 + 1, 4, 0x40, &old);
        printf("returned\n");
    } else if (!strcmp(m, "vp-whole-text")) {
        vp(IB + 0x1000, 0x400, 0x40, &old);
        printf("returned\n");
    } else if (!strcmp(m, "vp-header")) {
        vp(IB, 5, 0x40, &old);
        printf("returned\n");
    } else if (!strcmp(m, "vp-nonhook-null-old")) {
        vp(N1, 5, 0x40, NULL);
        printf("returned\n");
    } else if (!strcmp(m, "vp-last-byte")) {
        vp(IB + 0x1fff, 1, 0x40, &old);
        printf("returned\n");
    } else if (!strcmp(m, "vp-after-image")) {
        vp(IB + 0x2000, 1, 0x40, &old);
        printf("returned\n");
    } else if (!strcmp(m, "vp-outside")) {
        vp(A(&old), 4, 0x40, &old);
        printf("returned\n");
    } else if (!strcmp(m, "vp-region")) {
        uint64_t base;
        enter(); R[1] = 0; R[2] = 0x2000; R[8] = 0x3000; R[9] = 0x04; fle_imp_VirtualAlloc();
        base = R[0];
        printf("alloc %d\n", base != 0);
        vpr("vp", base, 0x1000, 0x02);
    } else {
        printf("no scenario %s\n", m);
        return 3;
    }
    return 0;
}
"####;

/// The constants of the synthetic program and its .text bytes (the same bytes build() writes into the PE).
fn prefix() -> String {
    let img = build();
    let bytes: Vec<String> = img[TEXT_RAW..TEXT_RAW + TEXT_USED].iter().map(|b| format!("0x{b:02x}")).collect();
    let mut s = format!("#define IB UINT64_C({IB:#x})\n");
    for off in [H1, T1, H2, N1] {
        s.push_str(&format!("void f_{:x}(void);\n", IB + off as u64));
    }
    s.push_str(&format!("static const unsigned char TEXT[] = {{ {} }};\n", bytes.join(", ")));
    s
}

struct Out {
    stdout: String,
    stderr: String,
    signal: Option<i32>,
    code: Option<i32>,
}

fn driver() -> &'static PathBuf {
    static D: OnceLock<PathBuf> = OnceLock::new();
    D.get_or_init(|| {
        let dir = work_dir();
        let tag = etxt_common::per_process("hook-driver");
        let (_, c) = lift(&build(), &[H1_VA, H2_VA]).unwrap();
        let gen_c = dir.join(format!("{tag}-gen.c"));
        let main_c = dir.join(format!("{tag}-main.c"));
        std::fs::write(&gen_c, &c).unwrap();
        std::fs::write(&main_c, DRIVER.replace("/*PREFIX*/", &prefix())).unwrap();
        let rt = rt_dir();
        let mut srcs: Vec<PathBuf> = std::fs::read_dir(&rt)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| {
                let n = p.file_name().unwrap().to_string_lossy().to_string();
                n.starts_with("fle_") && n.ends_with(".c")
            })
            .collect();
        srcs.sort();
        let exe = dir.join(&tag);
        let mut cmd = Command::new("clang");
        cmd.args(["-O0", "-std=gnu11", "-w", "-fno-builtin", "-fPIE", "-I"]).arg(&rt);
        for p in &srcs {
            cmd.arg(p);
        }
        let cc = cmd.arg(&gen_c).arg(&main_c).arg("-lm").arg("-o").arg(&exe).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
        assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
        exe
    })
}

fn run(scenario: &str) -> Out {
    let r = Command::new(driver()).arg(scenario).output().expect("run failed");
    Out {
        stdout: String::from_utf8_lossy(&r.stdout).to_string(),
        stderr: String::from_utf8_lossy(&r.stderr).to_string(),
        signal: r.status.signal(),
        code: r.status.code(),
    }
}

fn lines(scenario: &str) -> Vec<String> {
    let o = run(scenario);
    assert!(o.signal.is_none() && o.code == Some(0), "{scenario}: did not exit normally (signal {:?}, code {:?}): {}\n{}", o.signal, o.code, o.stderr, o.stdout);
    o.stdout.lines().map(|s| s.to_string()).collect()
}

/// The scenario must stop the run by name: SIGABRT, a "fle:" message with every `must_say`, and nothing printed after the stop.
fn aborts(scenario: &str, must_say: &[&str]) -> Out {
    let o = run(scenario);
    assert!(o.signal.is_some() || o.code != Some(0), "{scenario}: expected an abort, got stdout {}", o.stdout);
    assert_eq!(o.signal, Some(6), "{scenario}: not SIGABRT (code {:?}): {}", o.code, o.stderr);
    assert!(o.stderr.contains("fle:"), "{scenario}: no fle: message: {}", o.stderr);
    for s in must_say {
        assert!(o.stderr.contains(s), "{scenario}: message lacks {s:?}: {}", o.stderr);
    }
    assert!(!o.stdout.contains("returned"), "{scenario}: returned instead of stopping");
    o
}

/// rax, rbx and the three stack checks of one call (see the driver): rsp is where it was, the slots at and above it are untouched, the slots
/// below it are untouched (a body that pushes leaves 0 there).
fn row(what: &str, rax: u64, below: u8) -> String {
    format!("{what} rax={rax:x} rbx=abc rsp_ok=1 above_ok=1 below_ok={below}")
}

#[test]
fn before_the_hook_the_body_runs() {
    assert_eq!(lines("plain"), [row("h1", 0x11, 1), row("h2", 0x33, 0), row("n", 0x44, 1)]);
}

#[test]
fn a_jmp_rel32_in_the_image_is_a_tail_dispatch_that_keeps_the_stack_and_the_registers() {
    // rcx is the argument: T1 returns rcx + 0x22, so the registers of the entry reached the target. below_ok=1 for H2: the push of its body did not run.
    assert_eq!(lines("hooked"), [row("h1", 5 + 0x22, 1), row("h2", 0x100 + 0x22, 1)]);
    // the same through fle_dispatch (an indirect call of the guest to the hook point)
    assert_eq!(lines("dispatch"), [row("h1", 5 + 0x22, 1)]);
}

#[test]
fn writing_the_original_bytes_back_returns_to_the_body_and_the_hook_can_be_set_again() {
    assert_eq!(lines("restored"), [row("hooked", 5 + 0x22, 1), row("restored", 0x11, 1)]);
    // the patch / unpatch / patch sequence of the guest's hook object, three times, on the function with a frame
    let mut want = Vec::new();
    for _ in 0..3 {
        want.push(row("on", 1 + 0x22, 1));
        want.push(row("off", 0x33, 0));
    }
    assert_eq!(lines("rehook"), want);
}

#[test]
fn a_jmp_to_an_address_that_is_not_a_function_entry_stops_in_dispatch() {
    aborts("target-not-entry", &["indirect call to 0x10001021 is not a known function entry"]);
}

#[test]
fn bytes_that_are_neither_the_original_nor_jmp_rel32_stop_by_name() {
    // cc cc cc cc cc: not the original, not e9
    aborts("garbage", &["inline hook", "hook point 0x10001020", "cc cc cc cc cc", "b8 11 00 00 00", "decision 0105"]);
    // only the last byte changed
    aborts("partial", &["hook point 0x10001020", "b8 11 00 00 01"]);
    // a jmp rel32 to address 0 is not taken for "no hook"
    aborts("target-zero", &["hook point 0x10001020", "address 0"]);
}

#[test]
fn the_displacement_is_signed_and_counted_from_the_end_of_the_jmp() {
    // plain: 0 (no hook). forward: T1. backward (H2 -> H1), 1 GiB back and 1 GiB forward: the 32-bit displacement is sign-extended.
    assert_eq!(lines("target-direct"), ["plain 0", "forward 1", "backward 1", "far-back 1", "far-fwd 1"]);
}

#[test]
fn a_function_that_is_not_annotated_has_no_guard_and_virtualprotect_is_what_stops_it() {
    // the limit of the design: a jmp written over N1 is not seen by the lifted code (N1 is not a hook point)
    assert_eq!(lines("unguarded"), [row("n", 0x44, 1)]);
    // ... and the guest cannot get there: making N1 writable is the stop
    aborts("vp-nonhook", &["VirtualProtect", "inline hook at an address that is not an annotated hook point (decision 0105)", "address 0x10001080", "size 5", "new protection 0x40"]);
}

#[test]
fn virtualprotect_of_a_hook_point_returns_the_protection_of_the_page_and_records_the_new_one() {
    // the section is read + execute: 0x20. The page keeps what the guest set (0x40) until it sets another: the second call returns 0x40.
    assert_eq!(lines("vp-old"), ["a 1 old 20", "b 1 old 40", "c 1 old 40", "d 1 old 20"]);
    // H1 and H2 are in one page: protection belongs to whole pages, so the second hook point sees what the first call set
    assert_eq!(lines("vp-shared-page"), ["h1 1 old 20", "h2 1 old 40"]);
    // one to five bytes at the hook point
    assert_eq!(lines("vp-sizes"), ["size 1 -> 1", "size 2 -> 1", "size 3 -> 1", "size 4 -> 1", "size 5 -> 1"]);
    // lpflOldProtect NULL fails (the page); ERROR_INVALID_PARAMETER is the code the rest of fle_vmap.c uses for it (assumption)
    assert_eq!(lines("vp-null-old"), ["r 0 le 87"]);
}

#[test]
fn virtualprotect_in_a_registered_image_outside_the_hook_points_stops_by_name() {
    let say = |extra: &[&'static str]| -> Vec<&'static str> {
        let mut v = vec!["VirtualProtect", "not an annotated hook point (decision 0105)", "image base 0x10000000"];
        v.extend_from_slice(extra);
        v
    };
    aborts("vp-inside", &say(&["address 0x10001021", "size 4"]));
    aborts("vp-whole-text", &say(&["address 0x10001000", "size 1024"]));
    aborts("vp-header", &say(&["address 0x10000000", "size 5"]));
    aborts("vp-last-byte", &say(&["address 0x10001fff", "size 1"]));
    // a NULL lpflOldProtect does not make an unannotated address a plain failure
    aborts("vp-nonhook-null-old", &["not an annotated hook point"]);
    // a hook point with more than the 5 bytes of a jmp rel32
    aborts("vp-size6", &["VirtualProtect", "range of 6 bytes at the hook point 0x10001020", "5 bytes of jmp rel32"]);
}

#[test]
fn virtualprotect_outside_the_image_is_as_before() {
    aborts("vp-outside", &["VirtualProtect", "address is not in a VirtualAlloc region (the runtime does not track other memory)"]);
    // the first address after the image
    aborts("vp-after-image", &["address is not in a VirtualAlloc region"]);
    // a VirtualAlloc region still works with an image registered
    assert_eq!(lines("vp-region"), ["alloc 1", "vp 1 old 4"]);
}
