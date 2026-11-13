//! Tests for the runtime's call of a guest function: runtime/fle_callback.c (docs/decisions/0064-guest-callback.md;
//! sources: docs/investigations/callback-caller.md).
//!
//! The mechanism is checked natively with a stub `fle_dispatch_table` (the stubs are C functions that read their arguments from
//! R[] and write R[0], like lifted code), and once against the real generator: the C that fe-lift emits for a `call` is run with
//! the same stack pointer, and the callee sees the same stack distance as through fle_call_guest.
//!
//! Expected values are written here from the Microsoft x64 calling convention (Learn: shadow store of 32 bytes owned by the
//! caller, rsp 16-byte aligned at the call, so 16n+8 inside the callee, result in rax) and from the shape of the generated call
//! (`R[4] -= 8; fle_st(R[4], 8, ret); f(); R[4] += 8;`), not read back from the C code.
//!
//! The C sources come from runtime/ when fle_callback.c is there, otherwise from out-cb/work/ (development). FLE_CB_SRC_DIR (a
//! directory with fle_callback.c and fle_callback.h) overrides both: the mutation runs of the run record use it.

mod etxt_common;

use fe_lift::generate;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn runtime_dir() -> PathBuf {
    manifest().join("../../runtime")
}
fn src_dir() -> PathBuf {
    if let Ok(d) = std::env::var("FLE_CB_SRC_DIR") {
        return PathBuf::from(d);
    }
    if runtime_dir().join("fle_callback.c").exists() {
        runtime_dir()
    } else {
        manifest().join("../../out-cb/work")
    }
}
fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-cb/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

fn clang_c(src: &Path, obj: &Path, strict: bool) {
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-Wall", "-Wextra", "-fPIE", "-c"]);
    if strict {
        cmd.arg("-Werror");
    }
    let cc = cmd
        .arg("-I")
        .arg(src_dir())
        .arg("-I")
        .arg(runtime_dir())
        .arg(src)
        .arg("-o")
        .arg(obj)
        .env("TMPDIR", work_dir().join("tmp"))
        .output()
        .expect("clang not found");
    assert!(cc.status.success(), "clang failed for {}: {}", src.display(), String::from_utf8_lossy(&cc.stderr));
}

struct Out {
    stdout: String,
    stderr: String,
    signal: Option<i32>,
    ok: bool,
}

fn exec(exe: &Path, args: &[&str]) -> Out {
    let r = Command::new(exe).args(args).env("TMPDIR", work_dir().join("tmp")).output().expect("run failed");
    Out {
        stdout: String::from_utf8_lossy(&r.stdout).to_string(),
        stderr: String::from_utf8_lossy(&r.stderr).to_string(),
        signal: r.status.signal(),
        ok: r.status.success(),
    }
}

/// The driver: fle_callback.c (warnings are errors) + fle_env.c (fle_stop) + fle_imports.c, and one main with stub guest functions.
const DRIVER: &str = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include "fle_runtime.h"
#include "fle_dispatch.h"
#include "fle_env.h"
#include "fle_callback.h"

uint64_t R[16];
static uint64_t stk[0x2000] __attribute__((aligned(4096)));          /* 64 KiB */
uint8_t fle_teb[0x2000] __attribute__((aligned(16)));                /* decision 0319: the TEB of the guest thread (fle_callback.c reads StackBase at +8 and StackLimit at +0x10); all zero = no stack registered */
#define E0 ((uint64_t)(uintptr_t)stk + 0x8000 + 8)                    /* an API's rsp at its entry: 16n+8, [E0] = return address */
#define TOP ((uint64_t)(uintptr_t)stk + sizeof stk)
#define RET0 UINT64_C(0xdead0000)
#define U(x) ((unsigned long long)(x))
#define PATTERN UINT64_C(0xA5A5A5A5A5A5A5A5)

static uint64_t g_sp, g_ret, g_sp_inner, g_ret_inner, g_depth_seen, g_outer_depth;
static unsigned g_calls;
static uint64_t g_args[4];

#define VA_ARGS   0x1000
#define VA_RET64  0x1010
#define VA_PROBE  0x1020
#define VA_STACK  0x1030
#define VA_RBX    0x1040
#define VA_UNBAL  0x1050
#define VA_POPX   0x1060
#define VA_SMASH  0x1070
#define VA_NEST   0x1080
#define VA_LEAF   0x1090
#define VA_REC    0x10a0
#define VA_STOP   0x10b0
#define VA_SAVED  0x10c0
#define VA_REG    0x10e0
static unsigned g_regn;
static void cb_reg(void) { R[g_regn] ^= 1; if (g_regn != 0) R[0] = 5; }

static void cb_args(void)
{
    g_calls++;
    g_args[0] = R[1]; g_args[1] = R[2]; g_args[2] = R[8]; g_args[3] = R[9];
    R[0] = R[1] ^ (R[2] << 1) ^ (R[8] << 2) ^ (R[9] << 3);
    R[1] = R[2] = R[8] = R[9] = 0xdead; R[10] = 7; R[11] = 9;         /* volatile registers: free to change */
}
static void cb_ret64(void) { R[0] = UINT64_C(0x1122334455667788); }
static void cb_probe(void)
{
    g_calls++;
    g_sp = R[4]; g_ret = fle_ld(R[4], 8);
    /* the callee owns the 32 bytes above its return address (the shadow space): it may write all of them */
    fle_st(R[4] + 8, 8, R[1]); fle_st(R[4] + 16, 8, R[2]); fle_st(R[4] + 24, 8, R[8]); fle_st(R[4] + 32, 8, R[9]);
    R[0] = 1;
}
/* a callee in the style of lifted code: push rbx / rbp frame, sub rsp, locals, add rsp, pop */
static void cb_stack(void)
{
    uint64_t i;
    R[4] -= 8; fle_st(R[4], 8, R[3]);
    R[4] -= 8; fle_st(R[4], 8, R[5]);
    R[5] = R[4];
    R[4] -= 0x400;
    for (i = 0; i < 0x400; i += 8) fle_st(R[4] + i, 8, 0x5151515151515151ull + i);
    R[3] = 0x1234;
    R[0] = fle_ld(R[4] + 0x10, 8);
    R[4] += 0x400;
    R[5] = fle_ld(R[4], 8); R[4] += 8;
    R[3] = fle_ld(R[4], 8); R[4] += 8;
}
static void cb_rbx(void) { R[3] = 1; R[0] = 5; }
static void cb_unbal(void) { R[4] -= 8; R[0] = 5; }
static void cb_popx(void) { R[4] += 8; R[0] = 5; }
static void cb_smash(void) { fle_st(R[4], 8, 0); R[0] = 5; }
static void cb_leaf(void)
{
    g_sp_inner = R[4]; g_ret_inner = fle_ld(R[4], 8); g_depth_seen = fle_callback_depth();
    R[0] = R[1] + R[2] + R[8] + R[9];
}
static void cb_nest(void)
{
    uint64_t outer_sp = R[4], r;
    g_sp = R[4]; g_ret = fle_ld(R[4], 8);
    g_outer_depth = fle_callback_depth();
    r = fle_call_guest(VA_LEAF, R[1] + 1, 20, 300, 4000);
    printf("inner-result %llx depth-in-inner %llu depth-after %u rsp-kept %d\n", U(r), U(g_depth_seen), fle_callback_depth(), (int)(R[4] == outer_sp));
    R[0] = r + 1;
}
static void cb_rec(void)       /* R[1] = how many more nested calls to make */
{
    uint64_t left = R[1];
    g_calls++;
    fprintf(stderr, "enter %u depth %u\n", g_calls, fle_callback_depth());
    if (left > 0) R[0] = fle_call_guest(VA_REC, left - 1, 0, 0, 0) + 1;
    else R[0] = 100;
}
static void cb_stop(void) { fle_stop("probe", "stop inside the callback"); }
static void cb_saved(void)     /* uses rbx, rbp, rsi, rdi, r12-r15 and restores them, like a lifted function with push/pop */
{
    unsigned i;
    static const unsigned regs[] = { 3, 5, 6, 7, 12, 13, 14, 15 };
    uint64_t sv[8];
    for (i = 0; i < 8; i++) { sv[i] = R[regs[i]]; R[regs[i]] = 0xbad0000 + i; }
    R[0] = 77;
    for (i = 0; i < 8; i++) R[regs[i]] = sv[i];
}

const fle_fn_entry fle_dispatch_table[] = {
    { VA_ARGS, cb_args }, { VA_RET64, cb_ret64 }, { VA_PROBE, cb_probe }, { VA_STACK, cb_stack }, { VA_RBX, cb_rbx },
    { VA_UNBAL, cb_unbal }, { VA_POPX, cb_popx }, { VA_SMASH, cb_smash }, { VA_NEST, cb_nest }, { VA_LEAF, cb_leaf },
    { VA_REC, cb_rec }, { VA_STOP, cb_stop }, { VA_SAVED, cb_saved }, { VA_REG, cb_reg },
};
const uint64_t fle_dispatch_count = sizeof fle_dispatch_table / sizeof fle_dispatch_table[0];

static void fill_pattern(void) { size_t i; for (i = 0; i < sizeof stk / 8; i++) stk[i] = PATTERN; }
static uint64_t u(const char *s) { return strtoull(s, 0, 0); }

int main(int argc, char **argv)
{
    const char *t = argc > 1 ? argv[1] : "";
    (void)argc;
    fill_pattern();
    R[4] = E0;
    fle_st(E0, 8, RET0);

    if (!strcmp(t, "args")) {            /* argv[2..5] = rcx rdx r8 r9 */
        int i;
        uint64_t r;
        for (i = 0; i < 16; i++) R[i] = 0x7000 + (uint64_t)i;
        R[4] = E0;
        r = fle_call_guest(VA_ARGS, u(argv[2]), u(argv[3]), u(argv[4]), u(argv[5]));
        printf("ret %llx\nargs %llx %llx %llx %llx\nrsp-kept %d\ncalls %u\n", U(r), U(g_args[0]), U(g_args[1]), U(g_args[2]), U(g_args[3]), (int)(R[4] == E0), g_calls);
        printf("r0 %llx\n", U(R[0]));
    } else if (!strcmp(t, "ret64")) {
        printf("%llx\n", U(fle_call_guest(VA_RET64, 0, 0, 0, 0)));
    } else if (!strcmp(t, "sp")) {       /* argv[2] = delta subtracted from E0: entry rsp of any residue mod 16 */
        uint64_t e = E0 - u(argv[2]), r;
        R[4] = e;
        fle_st(e, 8, RET0);
        r = fle_call_guest(VA_PROBE, 0x11, 0x22, 0x33, 0x44);
        printf("entry %llx callee-sp %llx dist %llu mod16 %llu ret %llx rsp-kept %d result %llx\n", U(e), U(g_sp), U(e - g_sp), U(g_sp % 16), U(g_ret), (int)(R[4] == e), U(r));
        printf("ret-slot-of-api %llx\n", U(fle_ld(e, 8)));
    } else if (!strcmp(t, "above")) {    /* the callee uses 1 KiB of stack: nothing at or above the API's rsp may change */
        uint64_t a, bad = 0, r;
        r = fle_call_guest(VA_STACK, 1, 2, 3, 4);
        for (a = E0 + 8; a + 8 <= TOP; a += 8) if (fle_ld(a, 8) != PATTERN) bad++;
        printf("result %llx rsp-kept %d api-ret %llx changed-above %llu rbx %llx rbp %llx\n", U(r), (int)(R[4] == E0), U(fle_ld(E0, 8)), U(bad), U(R[3]), U(R[5]));
    } else if (!strcmp(t, "saved")) {
        unsigned i;
        uint64_t r;
        for (i = 0; i < 16; i++) R[i] = 0x9000 + i;
        R[4] = E0;
        r = fle_call_guest(VA_SAVED, 0, 0, 0, 0);
        printf("ret %llu regs", U(r));
        for (i = 0; i < 16; i++) if (i != 0 && i != 4) printf(" %llx", U(R[i]));
        printf("\n");
    } else if (!strcmp(t, "reg")) {      /* the callee flips bit 0 of R[argv[2]] */
        g_regn = (unsigned)u(argv[2]);
        fle_call_guest(VA_REG, 0, 0, 0, 0);
        printf("returned\n");
    } else if (!strcmp(t, "rbx")) { fle_call_guest(VA_RBX, 0, 0, 0, 0); printf("returned\n");
    } else if (!strcmp(t, "unbalanced")) { fle_call_guest(VA_UNBAL, 0, 0, 0, 0); printf("returned\n");
    } else if (!strcmp(t, "popextra")) { fle_call_guest(VA_POPX, 0, 0, 0, 0); printf("returned\n");
    } else if (!strcmp(t, "smash")) { fle_call_guest(VA_SMASH, 0, 0, 0, 0); printf("returned\n");
    } else if (!strcmp(t, "va0")) { fle_call_guest(0, 0, 0, 0, 0); printf("returned\n");
    } else if (!strcmp(t, "miss")) { fle_call_guest(u(argv[2]), 0, 0, 0, 0); printf("returned\n");
    } else if (!strcmp(t, "stop")) { fle_call_guest(VA_STOP, 0, 0, 0, 0); printf("returned\n");
    } else if (!strcmp(t, "nest")) {
        uint64_t r = fle_call_guest(VA_NEST, 0x10, 0, 0, 0);
        printf("outer-result %llx depth-in-outer %llu depth-end %u\n", U(r), U(g_outer_depth), fle_callback_depth());
        printf("outer-sp %llx inner-sp %llx gap %llu inner-mod16 %llu outer-ret %llx inner-ret %llx rsp-kept %d\n", U(g_sp), U(g_sp_inner), U(g_sp - g_sp_inner), U(g_sp_inner % 16), U(g_ret), U(g_ret_inner), (int)(R[4] == E0));
    } else if (!strcmp(t, "recstack")) {   /* decision 0319: the guest stack registered as [E0 - 20 KiB, TOP): 52 KiB, so the reserve is a quarter of it (13310 bytes) and the nesting stops by the stack, not by the count */
        uint64_t lo = E0 - 20 * 1024, hi = TOP;
        memcpy(fle_teb + 8, &hi, 8);
        memcpy(fle_teb + 0x10, &lo, 8);
        fle_call_guest(VA_REC, 1000, 0, 0, 0);
        printf("returned\n");
    } else if (!strcmp(t, "rec")) {      /* argv[2] = nested calls after the first one: levels = argv[2] + 1 */
        uint64_t r = fle_call_guest(VA_REC, u(argv[2]), 0, 0, 0);
        printf("result %llu calls %u depth-end %u rsp-kept %d\n", U(r), g_calls, fle_callback_depth(), (int)(R[4] == E0));
    } else {
        printf("unknown %s\n", t);
        return 3;
    }
    return 0;
}
"#;

fn driver() -> &'static PathBuf {
    static D: OnceLock<PathBuf> = OnceLock::new();
    D.get_or_init(|| {
        let dir = work_dir();
        let tag = etxt_common::per_process("cb-driver");
        let obj = dir.join(format!("{tag}-callback.o"));
        clang_c(&src_dir().join("fle_callback.c"), &obj, true);
        let main_c = dir.join(format!("{tag}-main.c"));
        std::fs::write(&main_c, DRIVER).unwrap();
        let exe = dir.join(&tag);
        let rt = runtime_dir();
        let cc = Command::new("clang")
            .args(["-O0", "-std=gnu11", "-w", "-fPIE", "-I"])
            .arg(src_dir())
            .arg("-I")
            .arg(&rt)
            .arg(&obj)
            .arg(rt.join("fle_env.c"))
            .arg(rt.join("fle_imports.c"))
            .arg("-o")
            .arg(&exe)
            .arg(&main_c)
            .env("TMPDIR", dir.join("tmp"))
            .output()
            .expect("clang not found");
        assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
        exe
    })
}

fn run(args: &[&str]) -> Out {
    exec(driver(), args)
}

fn lines(args: &[&str]) -> Vec<String> {
    let o = run(args);
    assert!(o.ok, "{args:?} did not exit normally (signal {:?}): {}\n{}", o.signal, o.stderr, o.stdout);
    o.stdout.lines().map(|s| s.to_string()).collect()
}

/// A run that must stop through fle_stop or fle_dispatch_miss: SIGABRT, `needle` on stderr, nothing on stdout.
fn stops(args: &[&str], needle: &str) -> String {
    let o = run(args);
    assert_eq!(o.signal, Some(6), "{args:?}: not SIGABRT: {} / {}", o.stderr, o.stdout);
    assert!(o.stderr.contains(needle), "{args:?}: no {needle:?} in {}", o.stderr);
    assert!(o.stdout.is_empty(), "{args:?}: output before the stop: {}", o.stdout);
    o.stderr
}

fn strip_comments(s: &str) -> String {
    let b: Vec<char> = s.chars().collect();
    let mut o = String::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == '/' && i + 1 < b.len() && b[i + 1] == '*' {
            i += 2;
            while i + 1 < b.len() && !(b[i] == '*' && b[i + 1] == '/') {
                i += 1;
            }
            i += 2;
        } else if b[i] == '/' && i + 1 < b.len() && b[i + 1] == '/' {
            while i < b.len() && b[i] != '\n' {
                i += 1;
            }
        } else {
            o.push(b[i]);
            i += 1;
        }
    }
    o
}

fn def_const(file: &str, name: &str) -> u64 {
    let t = std::fs::read_to_string(src_dir().join(file)).or_else(|_| std::fs::read_to_string(runtime_dir().join(file))).unwrap();
    let line = t.lines().find(|l| l.starts_with(&format!("#define {name} "))).unwrap_or_else(|| panic!("{name} not in {file}"));
    let rest = line[format!("#define {name} ").len()..].trim_start();
    if let Some(r) = rest.strip_prefix("UINT64_C(") {
        let tok = r.split(')').next().unwrap();
        return u64::from_str_radix(tok.trim_start_matches("0x"), 16).unwrap();
    }
    if let Some(r) = rest.strip_prefix("0x") {
        let tok: String = r.chars().take_while(|c| c.is_ascii_hexdigit()).collect();
        return u64::from_str_radix(&tok, 16).unwrap();
    }
    rest.split(|c: char| !c.is_ascii_digit()).next().unwrap().parse().unwrap()
}

// ---- 1. Constants (written here from the Learn page and the decision) ----

#[test]
fn the_constants_are_the_decided_ones() {
    // Learn, x64 calling convention: "The caller must always allocate sufficient space to store four register parameters" (8 bytes each)
    assert_eq!(def_const("fle_callback.h", "FLE_CB_SHADOW_BYTES"), 32);
    // decision 0319 replaces the depth limit 16 of decision 0064 by the stack (the host and guest stack reserves) with a backstop count
    assert_eq!(def_const("fle_callback.h", "FLE_CB_MAX_DEPTH"), 256);
    assert_eq!(def_const("fle_callback.h", "FLE_CB_GUEST_RESERVE"), 64 * 1024);
    assert_eq!(def_const("fle_callback.h", "FLE_CB_HOST_RESERVE"), 512 * 1024);
    // the return address slot value is outside the image (0x400000..0x18C0000) and is no function entry
    let ra = def_const("fle_callback.h", "FLE_CB_RETURN_ADDRESS");
    assert_eq!(ra, 0xCB00_0000);
    assert!(ra >= 0x18C0000 || ra < 0x400000);
}

// ---- 2. Arguments, result ----

#[test]
fn four_arguments_reach_rcx_rdx_r8_r9_and_the_result_is_rax() {
    let (a, b, c, d) = (0x0102030405060708u64, 0x1111111111111111u64, 0xFFFF_FFFF_0000_0001u64, 0x8000_0000_0000_0000u64);
    let l = lines(&["args", &a.to_string(), &b.to_string(), &c.to_string(), &d.to_string()]);
    let want = a ^ (b << 1) ^ (c << 2) ^ (d << 3);
    assert_eq!(l[0], format!("ret {want:x}"));
    assert_eq!(l[1], format!("args {a:x} {b:x} {c:x} {d:x}"));
    assert_eq!(l[2], "rsp-kept 1");
    assert_eq!(l[3], "calls 1");
    // fle_call_guest returns rax, and R[0] is the same value (the callee's result is not overwritten by the runtime)
    assert_eq!(l[4], format!("r0 {want:x}"));
}

#[test]
fn all_64_bits_of_every_argument_and_of_the_result_are_kept() {
    assert_eq!(lines(&["ret64"]), ["1122334455667788"]);
    let l = lines(&["args", "0xFFFFFFFFFFFFFFFF", "0", "0", "0"]);
    assert_eq!(l[1], "args ffffffffffffffff 0 0 0");
}

// ---- 3. The stack ----

/// The largest x with x % 16 == 8 and x + 8 + 32 <= entry: the lowest-numbered... the highest callee rsp that leaves a 32-byte
/// shadow space between the callee's return address and the API's rsp, with rsp 16n+8 inside the callee (Learn: rsp 16-byte aligned
/// at the call instruction, the call pushes 8).
fn expected_callee_sp(entry: u64) -> u64 {
    let hi = entry - 40;
    let m = (hi - 8) % 16;
    hi - m
}

#[test]
fn the_callee_starts_at_16n_plus_8_with_a_shadow_space_below_the_apis_frame() {
    // the API's rsp at its entry is 16n+8 in practice; other residues must work too (the runtime aligns by itself)
    for delta in [0u64, 8, 4, 12, 1, 16, 24] {
        let l = lines(&["sp", &delta.to_string()]);
        let f: Vec<&str> = l[0].split(' ').collect();
        let entry = u64::from_str_radix(f[1], 16).unwrap();
        let sp = u64::from_str_radix(f[3], 16).unwrap();
        assert_eq!(sp, expected_callee_sp(entry), "delta {delta}");
        assert_eq!(sp % 16, 8, "delta {delta}: 16n+8 inside the callee");
        assert!(sp + 8 + 32 <= entry, "delta {delta}: the 32-byte shadow space ends at or below the API's rsp");
        // tight: no more than 15 bytes of slack
        assert!(entry - (sp + 40) < 16, "delta {delta}");
        assert!(l[0].contains(" ret cb000000 "), "delta {delta}: {}", l[0]);
        assert!(l[0].contains(" rsp-kept 1 "), "delta {delta}: {}", l[0]);
        // the API's own return address slot is untouched
        assert_eq!(l[1], "ret-slot-of-api dead0000", "delta {delta}");
    }
}

#[test]
fn the_usual_entry_16n_plus_8_gives_the_same_distance_as_a_generated_call() {
    // entry 16n+8: callee rsp = entry - 48 (see the generator test below: sub rsp,0x28 + call = 0x30)
    let l = lines(&["sp", "0"]);
    let f: Vec<&str> = l[0].split(' ').collect();
    assert_eq!(f[5], "48");
}

#[test]
fn the_callee_may_write_its_shadow_space_and_use_a_kilobyte_of_stack_without_touching_the_callers_frame() {
    let l = lines(&["above"]);
    // cb_stack returns the 8 bytes at R[4]+0x10 of its 0x400-byte block: 0x5151515151515151 + 0x10
    // rbx and rbp: the callee pushed and popped them, so they are what the API had (0)
    assert_eq!(l[0], "result 5151515151515161 rsp-kept 1 api-ret dead0000 changed-above 0 rbx 0 rbp 0");
}

#[test]
fn rsp_is_restored_exactly_for_every_residue() {
    for delta in [0u64, 8, 4, 12] {
        let l = lines(&["sp", &delta.to_string()]);
        assert!(l[0].contains(" rsp-kept 1 "), "{delta}: {}", l[0]);
    }
}

// ---- 4. Registers ----

#[test]
fn a_callee_that_saves_and_restores_the_callee_saved_registers_leaves_them_as_they_were() {
    let l = lines(&["saved"]);
    // The test sets R[i] = 0x9000 + i. The callee-saved ones (rbx 3, rbp 5, rsi 6, rdi 7, r12-r15) come back unchanged. The volatile
    // r10, r11 are not touched by this callee, so they keep the test's values. rcx, rdx, r8, r9 hold what the runtime passed (0).
    let got: Vec<&str> = l[0].split(' ').collect();
    assert_eq!(&got[..3], ["ret", "77", "regs"]);
    // order of the printed registers: 1,2,3,5,6,7,8,9,10,11,12,13,14,15
    let regs: Vec<u64> = got[3..].iter().map(|s| u64::from_str_radix(s, 16).unwrap()).collect();
    assert_eq!(regs.len(), 14);
    let idx = [1u64, 2, 3, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
    for (k, &i) in idx.iter().enumerate() {
        let callee_saved = [3u64, 5, 6, 7, 12, 13, 14, 15].contains(&i);
        let arg = [1u64, 2, 8, 9].contains(&i);
        if callee_saved || !arg {
            assert_eq!(regs[k], 0x9000 + i, "R[{i}]");
        } else {
            assert_eq!(regs[k], 0, "R[{i}] holds the argument 0 the runtime passed");
        }
    }
}

#[test]
fn a_callee_that_changes_a_callee_saved_register_stops_the_run_by_name() {
    let e = stops(&["rbx"], "fle: fle_call_guest: the callback changed the callee-saved register rbx");
    assert!(!e.contains("returned"));
}

#[test]
fn each_callee_saved_register_is_checked_and_each_volatile_one_is_free() {
    // Learn: rbx, rbp, rdi, rsi, rsp, r12-r15 are nonvolatile; rax, rcx, rdx, r8-r11 are volatile. R[] index: rax 0, rcx 1, rdx 2, rbx 3,
    // rsp 4 (checked on its own), rbp 5, rsi 6, rdi 7, r8 8 .. r15 15.
    for (n, name) in [(3, "rbx"), (5, "rbp"), (6, "rsi"), (7, "rdi"), (12, "r12"), (13, "r13"), (14, "r14"), (15, "r15")] {
        stops(&["reg", &n.to_string()], &format!("fle: fle_call_guest: the callback changed the callee-saved register {name}\n"));
    }
    for n in [0, 1, 2, 8, 9, 10, 11] {
        assert_eq!(lines(&["reg", &n.to_string()]), ["returned"], "R[{n}] is volatile");
    }
}

#[test]
fn an_unbalanced_stack_stops_the_run_by_name() {
    stops(&["unbalanced"], "fle: fle_call_guest: the callback returned with R[4] not at its return address: the stack is unbalanced");
    stops(&["popextra"], "fle: fle_call_guest: the callback returned with R[4] not at its return address: the stack is unbalanced");
}

#[test]
fn a_changed_return_address_slot_stops_the_run_by_name() {
    stops(&["smash"], "fle: fle_call_guest: the callback changed its return address slot");
}

// ---- 5. Nesting ----

#[test]
fn a_callback_may_make_the_runtime_call_another_callback() {
    let l = lines(&["nest"]);
    // inner: R[1]+1 = 0x11, 20, 300, 4000 -> 0x11 + 20 + 300 + 4000 = 4337 = 0x10f1; the outer returns that + 1
    assert_eq!(l[0], "inner-result 10f1 depth-in-inner 2 depth-after 1 rsp-kept 1");
    assert_eq!(l[1], "outer-result 10f2 depth-in-outer 1 depth-end 0");
    let f: Vec<&str> = l[2].split(' ').collect();
    let outer = u64::from_str_radix(f[1], 16).unwrap();
    let inner = u64::from_str_radix(f[3], 16).unwrap();
    // the inner callee lies below the outer one, with room for the outer callee's frame in between: at least the outer return address,
    // the 32-byte shadow space of the inner call, and its own return address (all of it below the outer callee's rsp)
    assert!(inner + 8 + 32 <= outer, "inner {inner:#x} outer {outer:#x}");
    assert_eq!(inner % 16, 8, "inner callee rsp 16n+8");
    assert_eq!(outer % 16, 8, "outer callee rsp 16n+8");
    assert_eq!(f[5], (outer - inner).to_string());
    assert_eq!((f[7], f[9], f[11]), ("8", "cb000000", "cb000000"));
    assert_eq!(f[13], "1", "rsp restored at the end");
}

#[test]
fn nesting_of_exactly_the_limit_is_allowed_and_one_more_stops() {
    // levels = argv + 1; FLE_CB_MAX_DEPTH = 256, the backstop of decision 0319 (the build without a host stack query and without a registered guest stack)
    let l = lines(&["rec", "255"]);
    assert_eq!(l[0], "result 355 calls 256 depth-end 0 rsp-kept 1");
    let e = stops(&["rec", "256"], "fle: fle_call_guest: callback nesting is deeper than FLE_CB_MAX_DEPTH (256), the backstop against a runaway recursion: decision 0319");
    assert_eq!(e.matches("enter ").count(), 256, "{e}");
    // the depth is counted inside the callback: 1 in the first, 256 in the 256th (the limit); the 257th call stops
    assert!(e.contains("enter 256 depth 256") && !e.contains("enter 257"), "{e}");
}

#[test]
fn nesting_stops_when_the_guest_stack_is_used_up_before_the_count() {
    // decision 0319: Windows limits the nesting by the stack size only. The registered stack is 52 KiB, so the reserve is a quarter of it: (TOP - lo) / 4 = (65536 - 12296) / 4 = 13310 B;
    // ~48 bytes of stack per level leave room for about 150 levels, fewer than the 256 of the backstop.
    let e = stops(&["recstack"], "the guest stack is used up");
    assert!(e.contains("reserve 13310"), "{e}");
    let n = e.matches("enter ").count();
    assert!(n > 50 && n < 256, "{n} levels entered before the stop: {e}");
}

// ---- 6. Stops ----

#[test]
fn a_stop_inside_a_callback_stops_the_whole_run() {
    let e = stops(&["stop"], "fle: probe: stop inside the callback");
    assert!(!e.contains("returned"));
}

#[test]
fn address_zero_and_a_non_entry_stop_by_name() {
    stops(&["va0"], "fle: fle_call_guest: the callback address is 0");
    for va in ["0x1001", "0x4", "0x10d0", "0xCB000000", "0x7fffffffffffffff"] {
        stops(&["miss", va], "is not a known function entry");
    }
}

// ---- 7. Source checks ----

#[test]
fn source_is_plain_c_with_no_host_access_and_no_unwinding() {
    let code = strip_comments(&std::fs::read_to_string(src_dir().join("fle_callback.c")).unwrap());
    for bad in [
        "setjmp", "longjmp", "sigsetjmp", "siglongjmp", "<signal.h>", "signal(", "alarm", "getpid", "gettid", "pthread", "<unistd.h>", "<sys/", "fork(", "exec",
        "syscall", "fopen", "getenv", "clock_gettime", "gettimeofday", "time(", "asm", "__asm__", "__builtin_frame_address", "__builtin_return_address",
        "alloca", "malloc", "fle_host", "emscripten", "uintptr_t",
    ] {
        assert!(!code.contains(bad), "fle_callback.c must not use {bad}");
    }
    let b = code.as_bytes();
    let mut i = 0;
    while let Some(p) = code[i..].find("long") {
        let at = i + p;
        let before = at == 0 || !(b[at - 1].is_ascii_alphanumeric() || b[at - 1] == b'_');
        let after = at + 4 >= b.len() || !(b[at + 4].is_ascii_alphanumeric() || b[at + 4] == b'_');
        assert!(!(before && after), "fle_callback.c uses `long` near byte {at}");
        i = at + 4;
    }
    // the call goes through fle_dispatch only, and the whole-run stop through fle_stop only
    assert_eq!(code.matches("fle_dispatch(").count(), 1);
    assert!(code.contains("fle_stop(") && !code.contains("abort("));
    let h = strip_comments(&std::fs::read_to_string(src_dir().join("fle_callback.h")).unwrap());
    for bad in ["asm", "__asm__", "<unistd.h>", "<sys/", "<pthread.h>", "<setjmp.h>", "long "] {
        assert!(!h.contains(bad), "fle_callback.h must not contain {bad}");
    }
}

// ---- 8. The generated call: the same stack as fe-lift's `call` ----

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

/// A PE with the entry 0x1000: `sub rsp,0x28; call 0x1040; add rsp,0x28; ret` and 0x1040: `mov rax,rsp; ret` (returns its own rsp).
fn build_call_pe() -> Vec<u8> {
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
    put(&mut b, 0x2300, b"user32.dll\0");
    put(&mut b, 0x2400, b"\0\0GetKeyboardLayout\0");
    for (rva, v) in [(0x2100, 0x2400u64), (0x2200, 0x2400)] {
        w64(&mut b, data_at(rva), v);
    }
    let code: &[(u32, Vec<u8>)] = &[
        (0x1000, vec![0x48, 0x83, 0xec, 0x28]),
        (0x1004, [&[0xe8][..], &rel(0x1009, 0x1040)].concat()),
        (0x1009, vec![0x48, 0x83, 0xc4, 0x28, 0xc3]),
        (0x1040, vec![0x48, 0x89, 0xe0, 0xc3]),
    ];
    for (rva, bytes) in code {
        let o = TEXT_RAW + (rva - TEXT_RVA) as usize;
        b[o..o + bytes.len()].copy_from_slice(bytes);
    }
    b
}

fn all_runtime_sources() -> Vec<PathBuf> {
    let rt = runtime_dir();
    let mut v: Vec<PathBuf> = std::fs::read_dir(&rt)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            let n = p.file_name().unwrap().to_string_lossy().to_string();
            n.starts_with("fle_") && n.ends_with(".c") && n != "fle_callback.c"
        })
        .collect();
    v.sort();
    v
}

#[test]
fn a_generated_call_and_fle_call_guest_give_the_callee_the_same_stack() {
    let mut out = Vec::new();
    let st = generate(&build_call_pe(), &mut out).unwrap();
    let c = String::from_utf8(out).unwrap();
    assert_eq!(st.unsupported_total, 0, "{:?}", st.unsupported);
    // the shape of the generated call that fle_call_guest reproduces
    assert!(c.contains("{ R[4] -= 8; fle_st(R[4], 8, UINT64_C(0x140001009)); f_140001040(); R[4] += 8; }"), "call shape changed");
    let dir = work_dir();
    let tag = etxt_common::uniq("cb-gen");
    let gen_c = dir.join(format!("{tag}.c"));
    std::fs::write(&gen_c, &c).unwrap();
    let main = r#"#include <stdio.h>
#include "fle_runtime.h"
#include "fle_dispatch.h"
#include "fle_callback.h"
int main(void)
{
    static uint64_t stk[0x400] __attribute__((aligned(4096)));
    uint64_t e = (uint64_t)(uintptr_t)stk + 0x800 + 8, gen_sp, cb_sp, gen_r4, cb_r4;
    R[4] = e;
    fle_st(e, 8, 0xdead0000);
    fle_dispatch(UINT64_C(0x140001000));      /* the generated caller: sub rsp,0x28; call; add rsp,0x28; ret */
    gen_sp = R[0]; gen_r4 = R[4];
    R[4] = e;
    cb_sp = fle_call_guest(UINT64_C(0x140001040), 0, 0, 0, 0);
    cb_r4 = R[4];
    printf("gen %llu %llu\ncb %llu %llu\n", (unsigned long long)(e - gen_sp), (unsigned long long)(gen_sp % 16), (unsigned long long)(e - cb_sp), (unsigned long long)(cb_sp % 16));
    printf("r4 %d %d\n", (int)(gen_r4 == e), (int)(cb_r4 == e));
    return 0;
}
"#;
    let mainc = dir.join(format!("{tag}-main.c"));
    std::fs::write(&mainc, main).unwrap();
    let exe = dir.join(&tag);
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fno-builtin", "-fPIE", "-I"]).arg(src_dir()).arg("-I").arg(runtime_dir());
    for p in all_runtime_sources() {
        cmd.arg(p);
    }
    let cc = cmd
        .arg(src_dir().join("fle_callback.c"))
        .arg(&gen_c)
        .arg(&mainc)
        .arg("-lm")
        .arg("-o")
        .arg(&exe)
        .env("TMPDIR", dir.join("tmp"))
        .output()
        .expect("clang not found");
    assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
    let o = exec(&exe, &[]);
    assert!(o.ok, "{}\n{}", o.stdout, o.stderr);
    // a generated `sub rsp,0x28; call` puts the callee 0x30 = 48 bytes below the caller's entry rsp, at 16n+8; so does fle_call_guest
    assert_eq!(o.stdout.lines().collect::<Vec<_>>(), ["gen 48 8", "cb 48 8", "r4 1 1"]);
}
