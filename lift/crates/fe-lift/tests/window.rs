//! Tests for the virtual window table of runtime/fle_win.c: user32!CreateWindowExW, user32!SetWindowLongPtrW (GWLP_WNDPROC) and
//! user32!DefWindowProcW (docs/decisions/0071-virtual-windows.md, provisional; sources: docs/investigations/window-design.md), and
//! for fle_class_lookup (fle_class.c), the question CreateWindowExW asks of the class table of decision 0070.
//!
//! Expected values are written here from the decision (the rules on the Microsoft Learn pages CreateWindowEx, WM_NCCREATE,
//! WM_CREATE, WM_NCCALCSIZE, CREATESTRUCTW, SetWindowLongPtrW, DefWindowProcW, Interprocess communication, About Window Procedures,
//! and the choices of the decision), not read back from the C code. What the pages do not give (the handle values, the results of
//! DefWindowProcW, the RECT of WM_NCCALCSIZE, "no WM_GETMINMAXINFO", "no last error") is a choice of the decision and is the only
//! thing such a test pins down. The host is Linux: nothing is compared with a real Windows; no real window, display or message queue.
//! The guest window procedures are C stubs in a replacement `fle_dispatch_table` (the style of tests/callback.rs); the real
//! generator and the real DLL are used only in the last two sections.
//!
//! The C sources of fle_win.c / fle_win.h / fle_class.c / fle_class.h come from runtime/. FLE_WIN_SRC_DIR (a directory with those
//! four files) overrides that: the mutation runs of the run record use it. Every other file always comes from runtime/.

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
    if let Ok(d) = std::env::var("FLE_WIN_SRC_DIR") {
        return PathBuf::from(d);
    }
    runtime_dir()
}
fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-win/test");
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

/// fle_win.c and fle_class.c (warnings are errors) and the files they link with: fle_callback.c, fle_imports.c (GetLastError /
/// fle_set_last_error), fle_env.c (fle_stop, fle_w_icmp).
fn objs() -> &'static Vec<PathBuf> {
    static O: OnceLock<Vec<PathBuf>> = OnceLock::new();
    O.get_or_init(|| {
        let dir = work_dir().join(etxt_common::per_process("obj-win"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut v = Vec::new();
        for (name, ours) in [("fle_win.c", true), ("fle_class.c", true), ("fle_callback.c", false), ("fle_imports.c", false), ("fle_env.c", false)] {
            let base = if ours { src_dir() } else { runtime_dir() };
            let o = dir.join(name.replace(".c", ".o"));
            clang_c(&base.join(name), &o, ours);
            v.push(o);
        }
        v
    })
}

pub struct Out {
    stdout: String,
    stderr: String,
    signal: Option<i32>,
    ok: bool,
}

const PRELUDE: &str = r##"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <stddef.h>
#include <pthread.h>
#include "fle_runtime.h"
#include "fle_dispatch.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_modules.h"
#include "fle_callback.h"
#include "fle_class.h"
#include "fle_win.h"

uint64_t R[16];
static uint64_t stk[0x2000] __attribute__((aligned(4096)));          /* 64 KiB */
#define E0 ((uint64_t)(uintptr_t)stk + 0x8000 + 8)
#define TOP ((uint64_t)(uintptr_t)stk + sizeof stk)
#define RET0 UINT64_C(0xdead0000)
#define PATTERN UINT64_C(0xA5A5A5A5A5A5A5A5)
#define BAD UINT64_C(0xBADBADBAD)
#define U(x) ((unsigned long long)(x))
#define A(x) ((uint64_t)(uintptr_t)(x))
#define H1 UINT64_C(0x7F990004)

/* CREATESTRUCTW as the C compiler lays it out for the types of the Learn page (LP64 has the 8-byte pointers of Win64) */
typedef struct { void *a; void *b; void *c; void *d; int cy, cx, y, x; int32_t style; const uint16_t *name; const uint16_t *cls; uint32_t ex; } CS;
_Static_assert(sizeof(CS) == FLE_CREATESTRUCTW_SIZE, "sizeof CREATESTRUCTW");
_Static_assert(offsetof(CS, a) == FLE_CS_OFF_CREATEPARAMS && offsetof(CS, b) == FLE_CS_OFF_HINSTANCE && offsetof(CS, c) == FLE_CS_OFF_MENU &&
               offsetof(CS, d) == FLE_CS_OFF_PARENT && offsetof(CS, cy) == FLE_CS_OFF_CY && offsetof(CS, cx) == FLE_CS_OFF_CX &&
               offsetof(CS, y) == FLE_CS_OFF_Y && offsetof(CS, x) == FLE_CS_OFF_X && offsetof(CS, style) == FLE_CS_OFF_STYLE &&
               offsetof(CS, name) == FLE_CS_OFF_NAME && offsetof(CS, cls) == FLE_CS_OFF_CLASS && offsetof(CS, ex) == FLE_CS_OFF_EXSTYLE,
               "offsets of CREATESTRUCTW");

static uint32_t LE(void) { fle_imp_GetLastError(); return (uint32_t)R[0]; }
static void SETLE(uint32_t v) { fle_set_last_error(v); }
static void fill(void) { size_t i; for (i = 0; i < sizeof stk / 8; i++) stk[i] = PATTERN; }

/* a few ASCII strings as UTF-16, rotating buffers so that several can be alive */
static uint64_t S(const char *s)
{
    static uint16_t b[8][400];
    static unsigned k;
    uint16_t *p = b[k++ % 8];
    size_t i, n = strlen(s);
    for (i = 0; i < n; i++) p[i] = (uint8_t)s[i];
    p[n] = 0;
    return A(p);
}

/* ---- the guest: window procedures and the DefWindowProcW thunk, as stubs (the style of tests/callback.rs) ---- */
#define VA_REC   0x2000
#define VA_REC2  0x2010
#define VA_SETP  0x2020
#define VA_NEST  0x2030
#define VA_DEF   0x424920            /* the guest's DefWindowProcW thunk (a class window procedure of three guest classes) */

typedef struct {
    int tag;
    uint64_t h, msg, wp, lp, sp;
    unsigned depth;
    int inw;
    fle_win_view v;
    uint8_t cs[80];
    uint8_t rect[16];
    uint64_t ret;
} ent;
static ent lg[512];
static unsigned ln;
static uint64_t rv[3] = { 1, 0, 0 };            /* what VA_REC answers to WM_NCCREATE, WM_NCCALCSIZE, WM_CREATE */
static uint64_t rv2[3] = { 1, 0, 0 };           /* the same for VA_REC2 */
static uint64_t g_prev, g_inner, g_esp_top, g_above;
static int g_nest_once;

static void logit(int tag, uint64_t ret)
{
    ent *e = &lg[ln++];
    memset(e, 0, sizeof *e);
    e->tag = tag; e->h = R[1]; e->msg = R[2]; e->wp = R[8]; e->lp = R[9]; e->sp = R[4]; e->depth = fle_callback_depth();
    e->inw = fle_win_get(e->h, &e->v);
    if ((e->msg == 0x81 || e->msg == 0x01) && e->lp) memcpy(e->cs, (void *)(uintptr_t)e->lp, 80);
    if (e->msg == 0x83 && e->lp) memcpy(e->rect, (void *)(uintptr_t)e->lp, 16);
    e->ret = ret;
}
static uint64_t pick(const uint64_t *v)
{
    return R[2] == 0x81 ? v[0] : R[2] == 0x83 ? v[1] : R[2] == 0x01 ? v[2] : 0;
}
/* a real window procedure writes into its shadow space (the 32 bytes above its return address) */
static void scribble(void)
{
    fle_st(R[4] + 8, 8, 0x5151515151515151ull); fle_st(R[4] + 16, 8, 0x5252525252525252ull);
    fle_st(R[4] + 24, 8, 0x5353535353535353ull); fle_st(R[4] + 32, 8, 0x5454545454545454ull);
}
static void cb_rec(void)  { uint64_t r = pick(rv);  logit(1, r); scribble(); R[0] = r; R[1] = R[2] = R[8] = R[9] = 0xdead; }
static void cb_rec2(void) { uint64_t r = pick(rv2); logit(2, r); scribble(); R[0] = r; }
static uint64_t cw_raw(uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t);
static void cb_setp(void)                         /* the class window procedure of the guest's TLightweightLayeredControl, in short */
{
    uint64_t h = R[1], m = R[2], r = pick(rv);
    logit(7, r);
    if (m == 0x81) {
        R[1] = h; R[2] = 0xFFFFFFFCu; R[8] = VA_REC2; R[9] = 0;
        fle_imp_SetWindowLongPtrW();
        g_prev = R[0];
    }
    R[0] = r;
}
static void cb_nest(void)                         /* creates a second window while the first one is being created */
{
    uint64_t r = pick(rv);
    logit(3, r);
    if (R[2] == 0x81 && g_nest_once) {
        g_nest_once = 0;
        g_inner = cw_raw(0x80, S("TTimerWindow"), 0, 0x80000000u, 0, 0, 0, 0, 0, 0, 0x400000, 0);
    }
    R[0] = r;
}
static void cb_def(void)                          /* the thunk 0x424920: jmp [DefWindowProcW] */
{
    logit(9, 0);
    fle_imp_DefWindowProcW();
    lg[ln - 1].ret = R[0];
}
const fle_fn_entry fle_dispatch_table[] = { { VA_REC, cb_rec }, { VA_REC2, cb_rec2 }, { VA_SETP, cb_setp }, { VA_NEST, cb_nest }, { VA_DEF, cb_def } };
const uint64_t fle_dispatch_count = 5;

/* ---- the class table: RegisterClassW of a WNDCLASSW with a different value in every field ---- */
static uint64_t callv(void (*f)(void), uint64_t a, uint64_t b, uint64_t c, uint64_t d)
{
    uint64_t keep4 = R[4], r;
    memset(stk, 0, 0x100);
    stk[0] = RET0;
    R[1] = a; R[2] = b; R[8] = c; R[9] = d;
    R[4] = A(stk);
    R[0] = BAD;
    f();
    r = R[0];
    R[4] = keep4;
    return r;
}
static uint8_t wcb[72];
static void p32(unsigned o, uint32_t v) { memcpy(wcb + o, &v, 4); }
static void p64(unsigned o, uint64_t v) { memcpy(wcb + o, &v, 8); }
static uint64_t REGP(const char *name, uint64_t proc)
{
    memset(wcb, 0, sizeof wcb);
    p32(0, 3); p64(8, proc); p32(16, 2); p32(20, 4); p64(24, 0x400000);
    p64(32, 0x1111); p64(40, 0x2222); p64(48, 0x3333); p64(56, 0x4444); p64(64, S(name));
    return callv(fle_imp_RegisterClassW, A(wcb), 0, 0, 0);
}

/* ---- CreateWindowExW with the 12 arguments in place: 4 in registers, 8 in the slots at [rsp+0x20 ..] of the call ---- */
static uint64_t cw_raw(uint64_t ex, uint64_t cls, uint64_t name, uint64_t style, uint64_t x, uint64_t y, uint64_t w, uint64_t h,
                       uint64_t parent, uint64_t menu, uint64_t hinst, uint64_t param)
{
    uint64_t save4 = R[4], call_sp = (R[4] - 0x80) & ~UINT64_C(15), in_sp = call_sp - 8, r, a;
    uint64_t before[0x1000];
    unsigned n = 0, i, top_level = fle_callback_depth() == 0;
    fle_st(call_sp + 0x20, 8, x); fle_st(call_sp + 0x28, 8, y); fle_st(call_sp + 0x30, 8, w); fle_st(call_sp + 0x38, 8, h);
    fle_st(call_sp + 0x40, 8, parent); fle_st(call_sp + 0x48, 8, menu); fle_st(call_sp + 0x50, 8, hinst); fle_st(call_sp + 0x58, 8, param);
    fle_st(in_sp, 8, RET0);
    if (top_level) {
        g_esp_top = in_sp;
        for (a = in_sp; a < TOP && n < 0x1000; a += 8) before[n++] = fle_ld(a, 8);
    }
    R[1] = ex; R[2] = cls; R[8] = name; R[9] = style; R[4] = in_sp; R[0] = BAD;
    fle_imp_CreateWindowExW();
    r = R[0];
    if (R[4] != in_sp) printf("R4-NOT-RESTORED\n");
    if (top_level) {
        g_above = 0;
        for (a = in_sp, i = 0; a < TOP && i < n; a += 8, i++) if (fle_ld(a, 8) != before[i]) g_above++;
    }
    R[4] = save4;
    return r;
}
#define CWX(ex, cls, name, style, x, y, w, h, parent, menu, hinst, param) cw_raw(ex, cls, name, style, x, y, w, h, parent, menu, hinst, param)
/* the call of the guest at 0x52a8be */
#define CW_OK(cls) cw_raw(0x80, S(cls), A(empty), 0x80000000u, 0, 0, 0, 0, 0, 0, 0x400000, 0)
#define CW_LIVE(cls) (cw_raw(0x80, S(cls), A(empty), 0x80000000u, 0, 0, 0, 0, 0, 0, 0x400000, 0) != 0)
static uint16_t empty[1] = { 0 };

#define CALL(f, ...) callv((void (*)(void))(f), __VA_ARGS__)
#define SWL(h, idx, val) CALL(fle_imp_SetWindowLongPtrW, h, idx, val, 0)
#define DEF(h, msg, wp, lp) CALL(fle_imp_DefWindowProcW, h, msg, wp, lp)

static void pw(uint64_t h)
{
    fle_win_view v;
    if (!fle_win_get(h, &v)) { printf("win %llx none\n", U(h)); return; }
    printf("win %llx wndproc=%llx hinst=%llx style=%x ex=%x atom=%x live=%d\n", U(h), U(v.wndproc), U(v.hinst), (unsigned)v.style, (unsigned)v.exstyle, (unsigned)v.atom, v.live);
}
static void dump_log(void)
{
    unsigned i;
    for (i = 0; i < ln; i++) {
        ent *e = &lg[i];
        const CS *c = (const CS *)e->cs;
        const int32_t *rc = (const int32_t *)e->rect;
        printf("m%u tag=%d h=%llx msg=%llx wp=%llx depth=%u inw=%d live=%d wndproc=%llx ret=%llx\n", i, e->tag, U(e->h), U(e->msg), U(e->wp), e->depth, e->inw, e->v.live, U(e->v.wndproc), U(e->ret));
        if (e->msg == 0x81 || e->msg == 0x01) {
            uint32_t pad1, pad2;
            memcpy(&pad1, e->cs + 52, 4); memcpy(&pad2, e->cs + 76, 4);
            printf("cs%u create=%llx inst=%llx menu=%llx parent=%llx cy=%d cx=%d y=%d x=%d style=%x pad1=%x pad2=%x ex=%x\n", i, U(A(c->a)), U(A(c->b)), U(A(c->c)), U(A(c->d)), c->cy, c->cx, c->y, c->x, (unsigned)c->style, pad1, pad2, (unsigned)c->ex);
        }
        if (e->msg == 0x83) printf("rect%u %d %d %d %d\n", i, rc[0], rc[1], rc[2], rc[3]);
    }
}
"##;

fn run_src(name: &str, top: &str, body: &str) -> Out {
    let dir = work_dir();
    let uname = etxt_common::uniq(name);
    let c = dir.join(format!("{uname}.c"));
    let exe = dir.join(&uname);
    let src = format!("{PRELUDE}\n{top}\nint main(void)\n{{\n    R[4] = E0;\n{body}\n    return 0;\n}}\n");
    std::fs::write(&c, src).unwrap();
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fPIE", "-pthread", "-I"]).arg(src_dir()).arg("-I").arg(runtime_dir());
    for o in objs() {
        cmd.arg(o);
    }
    let cc = cmd.arg("-o").arg(&exe).arg(&c).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new(&exe).env("TMPDIR", dir.join("tmp")).output().expect("run failed");
    Out {
        stdout: String::from_utf8_lossy(&r.stdout).to_string(),
        stderr: String::from_utf8_lossy(&r.stderr).to_string(),
        signal: r.status.signal(),
        ok: r.status.success(),
    }
}

fn run(name: &str, body: &str) -> Out {
    run_src(name, "", body)
}

fn lines(name: &str, body: &str) -> Vec<String> {
    let o = run(name, body);
    assert!(o.ok, "{name} did not exit normally (signal {:?}): {}\n{}", o.signal, o.stderr, o.stdout);
    o.stdout.lines().map(|s| s.to_string()).collect()
}

/// Each (label, C statements) must stop the run with SIGABRT, a "fle: <api>: " message that contains `word`, and no output after it.
/// `setup` runs first.
fn assert_aborts(prefix: &str, setup: &str, api: &str, word: &str, cases: &[(&str, &str)]) {
    for (label, stmts) in cases {
        let body = format!("{setup}\n    {stmts}\n    printf(\"returned %llu\\n\", U(R[0]));");
        let o = run(&format!("{prefix}-{label}"), &body);
        assert!(!o.ok, "{label}: expected an abort, got stdout {}", o.stdout);
        assert!(!o.stdout.contains("returned"), "{label}: returned instead of stopping");
        assert!(o.stderr.contains(&format!("fle: {api}: ")), "{label}: no 'fle: {api}' message: {}", o.stderr);
        assert!(o.stderr.contains(word), "{label}: message lacks {word:?}: {}", o.stderr);
        assert_eq!(o.signal, Some(6), "{label}: not SIGABRT");
    }
}

/// The value of `#define NAME UINT64_C(0x...)` or `#define NAME <number>` in a runtime header.
fn header_const(file: &str, name: &str) -> u64 {
    let t = std::fs::read_to_string(src_dir().join(file)).or_else(|_| std::fs::read_to_string(runtime_dir().join(file))).unwrap();
    let key = format!("#define {name} ");
    let line = t.lines().find(|l| l.starts_with(&key)).unwrap_or_else(|| panic!("{name} not in {file}"));
    let mut rest = line[key.len()..].trim_start();
    if let Some(r) = rest.strip_prefix("UINT64_C(") {
        rest = r;
    }
    if let Some(h) = rest.strip_prefix("0x").or_else(|| rest.strip_prefix("0X")) {
        let tok: String = h.chars().take_while(|c| c.is_ascii_hexdigit()).collect();
        return u64::from_str_radix(&tok, 16).unwrap();
    }
    rest.split(|c: char| !c.is_ascii_digit()).next().unwrap().parse().unwrap()
}

/// C text without comments (strings are kept).
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

fn want(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

// ---- 1. Table, header, constants ----

#[test]
fn the_three_rows_are_in_the_table_and_declared_and_the_neighbours_are_not_answered() {
    let table = std::fs::read_to_string(manifest().join("src/imports_table.rs")).unwrap();
    let h = std::fs::read_to_string(src_dir().join("fle_win.h")).unwrap();
    for n in ["CreateWindowExW", "SetWindowLongPtrW", "DefWindowProcW"] {
        let row = format!("(\"user32.dll\", \"{n}\", \"fle_imp_{n}\", \"jmp_imp:{n}\")");
        assert_eq!(table.matches(&row).count(), 1, "{n}: exactly one row in imports_table.rs");
        assert!(h.contains(&format!("void fle_imp_{n}(void)")), "{n} is declared in fle_win.h");
    }
    // decision 0071 adds these three and no more: the rest of user32's windows are the next stops (docs/investigations/window-design.md section 8)
    // Decision 0119 (wall 9..17) answers SendMessageW, GetWindowLongPtrW and SetWindowTextW (and SetClassLongPtrW, GetSystemMenu, DeleteMenu, wtsapi32 and uxtheme rows): they are
    // no longer in this list of names that decision 0071 leaves out (old: asserted absent; new: tests/wall_fr6.rs asserts each of them present exactly once).
    // Decision 0122 answers PostMessageW, GetMessageW, PeekMessageW, DispatchMessageW and TranslateMessage (and WaitMessage, PostQuitMessage, CreateThread ...): they leave this list of names
    // that decision 0071 leaves out (old: asserted absent; new: tests/wall_fr8.rs asserts each of them present exactly once, in the order of the decision's rows).
    // decision 0123 (old: "DestroyWindow", "UnregisterClassW", "ShowWindow", "CallWindowProcW", "SetTimer", "KillTimer", "IsWindow", "SetParent", "SetWindowPos" and "EnableWindow" were in this list): the window model answers them
    // (tests/wall_fr9.rs asserts each of them present exactly once)
    for n in [
        "SendMessageA",
        "CreateWindowExA", "DefWindowProcA", "SetWindowLongPtrA", "SetWindowLongW", "SetWindowLongA", "GetWindowLongW", "RegisterClassExW",
    ] {
        assert!(!table.contains(&format!("\"{n}\"")), "{n} must not be answered by this decision");
    }
}

#[test]
fn the_decided_constants() {
    assert_eq!(header_const("fle_win.h", "FLE_WIN_HANDLE_BASE"), 0x7F99_0000);
    assert_eq!(header_const("fle_win.h", "FLE_WIN_REGION_SIZE"), 0x10_0000);
    assert_eq!(header_const("fle_win.h", "FLE_WIN_HANDLE_STEP"), 4);
    // Learn CreateWindowEx / WM_NCCREATE / WM_CREATE / WM_NCCALCSIZE
    assert_eq!(header_const("fle_win.h", "FLE_WM_CREATE"), 0x0001);
    assert_eq!(header_const("fle_win.h", "FLE_WM_NCCREATE"), 0x0081);
    assert_eq!(header_const("fle_win.h", "FLE_WM_NCCALCSIZE"), 0x0083);
    // Learn Window Styles / Extended Window Styles
    assert_eq!(header_const("fle_win.h", "FLE_WS_POPUP"), 0x8000_0000);
    assert_eq!(header_const("fle_win.h", "FLE_WS_EX_TOOLWINDOW"), 0x80);
    // Learn CREATESTRUCTW on x64: the C compiler's layout of the fields of the page (the driver has a _Static_assert for each offset)
    assert_eq!(header_const("fle_win.h", "FLE_CREATESTRUCTW_SIZE"), 80);
    for (n, v) in [
        ("CREATEPARAMS", 0u64), ("HINSTANCE", 8), ("MENU", 16), ("PARENT", 24), ("CY", 32), ("CX", 36), ("Y", 40), ("X", 44), ("STYLE", 48), ("NAME", 56),
        ("CLASS", 64), ("EXSTYLE", 72),
    ] {
        assert_eq!(header_const("fle_win.h", &format!("FLE_CS_OFF_{n}")), v, "{n}");
    }
}

#[test]
fn the_handle_region_overlaps_no_other_virtual_handle_region_and_is_below_2_gib() {
    let base = header_const("fle_win.h", "FLE_WIN_HANDLE_BASE");
    let size = header_const("fle_win.h", "FLE_WIN_REGION_SIZE");
    assert!(base + size <= 0x8000_0000, "below 2 GiB: truncation and sign extension give one value");
    assert!(base > 0x1000_0000, "far above the image");
    // (file, base, size) of the regions below 4 GiB (decisions 0053, 0054, 0059, 0061, 0065)
    let others = [
        ("fle_gdc.h", "FLE_GDC_HANDLE_BASE", "FLE_GDC_REGION_SIZE"),
        ("fle_gdi.h", "FLE_GDI_HANDLE_BASE", "FLE_GDI_REGION_SIZE"),
        ("fle_cursor.h", "FLE_CURSOR_HANDLE_BASE", "FLE_CURSOR_REGION_SIZE"),
        ("fle_loadimage.h", "FLE_LI_MODULE_HANDLE_BASE", "FLE_LI_REGION_SIZE"),
        ("fle_monitor.h", "FLE_MONITOR_HANDLE_BASE", "FLE_MONITOR_REGION_SIZE"),
        // decision 0119: the window menus (GetSystemMenu), a region of its own directly before the window handles
        ("fle_win.h", "FLE_MENU_HANDLE_BASE", "FLE_MENU_REGION_SIZE"),
    ];
    let mut named: Vec<String> = Vec::new();
    for (f, b, s) in others {
        let (ob, os) = (header_const(f, b), header_const(f, s));
        assert!(ob + os <= 0x8000_0000, "{b}");
        assert!(base + size <= ob || ob + os <= base, "the window region [{base:#x}, {:#x}) overlaps {b} [{ob:#x}, {:#x})", base + size, ob + os);
        named.push(b.to_string());
    }
    // every other `_BASE UINT64_C(...)` of the runtime headers is above 4 GiB, or is one of the five above: a new low region is noticed
    for e in std::fs::read_dir(runtime_dir()).unwrap() {
        let p = e.unwrap().path();
        if p.extension().map_or(true, |x| x != "h") {
            continue;
        }
        for l in std::fs::read_to_string(&p).unwrap().lines() {
            let Some(rest) = l.strip_prefix("#define ") else { continue };
            let Some((name, val)) = rest.split_once(' ') else { continue };
            if !name.ends_with("_BASE") || name == "FLE_WIN_HANDLE_BASE" || name == "FLE_MODULE_HANDLE_BASE_UNUSED" {
                continue;
            }
            let val = val.trim_start();
            let Some(h) = val.strip_prefix("UINT64_C(0x") else { continue };
            let tok: String = h.chars().take_while(|c| c.is_ascii_hexdigit()).collect();
            let v = u64::from_str_radix(&tok, 16).unwrap();
            assert!(v >= 1 << 32 || named.iter().any(|n| n == name), "{name} = {v:#x} is a region below 4 GiB that the window region was not checked against");
        }
    }
}

// ---- 2. CreateWindowExW: the messages, their order, their arguments ----

#[test]
fn create_window_sends_nccreate_nccalcsize_create_in_this_order_with_the_decided_arguments_and_returns_the_handle() {
    let l = lines(
        "win-create",
        r#"
    uint64_t atom = REGP("TPUtilWindow", VA_REC), h, cls;
    uint64_t esp;
    fill();
    SETLE(0x1234);
    cls = S("TPUtilWindow");
    h = cw_raw(0x80, cls, A(empty), 0x80000000u, 0, 0, 0, 0, 0, 0, 0x400000, 0);
    printf("atom %llx\n", U(atom));
    printf("h %llx count %llu msgs %u depth %u\n", U(h), U(fle_win_count()), ln, fle_callback_depth());
    printf("last error %x above-changed %llu r4 %d\n", LE(), U(g_above), R[4] == E0);
    pw(h);
    dump_log();
    esp = g_esp_top;
    printf("lp same %d cs bytes same %d\n", lg[0].lp == lg[2].lp, memcmp(lg[0].cs, lg[2].cs, 80) == 0);
    printf("cs below frame %d aligned %d rect below frame %d aligned %d\n", lg[0].lp + 80 <= esp, lg[0].lp % 16 == 0, lg[1].lp + 16 <= esp, lg[1].lp % 16 == 0);
    printf("rect below cs %d callee below cs %d %d %d\n", lg[1].lp >= lg[0].lp + 80, lg[0].sp + 40 <= lg[0].lp, lg[1].sp + 40 <= lg[1].lp, lg[2].sp + 40 <= lg[2].lp);
    printf("name ptr %d class ptr %d\n", *(uint64_t *)(lg[0].cs + 56) == A(empty), *(uint64_t *)(lg[0].cs + 64) == cls);
"#,
    );
    assert_eq!(
        l,
        want(&[
            "atom c000",
            "h 7f990004 count 1 msgs 5 depth 0",   // decision 0219: a WS_POPUP window also gets WM_SIZE and WM_MOVE after WM_CREATE (was msgs 3)
            "last error 1234 above-changed 0 r4 1",
            "win 7f990004 wndproc=2000 hinst=400000 style=80000000 ex=80 atom=c000 live=1",
            "m0 tag=1 h=7f990004 msg=81 wp=0 depth=1 inw=1 live=1 wndproc=2000 ret=1",
            "cs0 create=0 inst=400000 menu=0 parent=0 cy=0 cx=0 y=0 x=0 style=80000000 pad1=0 pad2=0 ex=80",
            "m1 tag=1 h=7f990004 msg=83 wp=0 depth=1 inw=1 live=1 wndproc=2000 ret=0",
            "rect1 0 0 0 0",
            "m2 tag=1 h=7f990004 msg=1 wp=0 depth=1 inw=1 live=1 wndproc=2000 ret=0",
            "cs2 create=0 inst=400000 menu=0 parent=0 cy=0 cx=0 y=0 x=0 style=80000000 pad1=0 pad2=0 ex=80",
            "m3 tag=1 h=7f990004 msg=5 wp=0 depth=1 inw=1 live=1 wndproc=2000 ret=0",   // decision 0219: WM_SIZE (SIZE_RESTORED), lParam 0 = a 0 x 0 window
            "m4 tag=1 h=7f990004 msg=3 wp=0 depth=1 inw=1 live=1 wndproc=2000 ret=0",   // decision 0219: WM_MOVE at (0, 0)
            "lp same 1 cs bytes same 1",
            "cs below frame 1 aligned 1 rect below frame 1 aligned 1",
            "rect below cs 1 callee below cs 1 1 1",
            "name ptr 1 class ptr 1",
        ])
    );
}

#[test]
fn the_window_is_in_the_table_before_the_first_message_and_the_handles_count_up_by_4_and_are_never_reused() {
    let l = lines(
        "win-handles",
        r#"
    uint64_t h[3];
    REGP("TPUtilWindow", VA_REC);
    h[0] = CW_OK("TPUtilWindow"); h[1] = CW_OK("TPUtilWindow"); h[2] = CW_OK("TPUtilWindow");
    printf("%llx %llx %llx count %llu\n", U(h[0]), U(h[1]), U(h[2]), U(fle_win_count()));
    pw(h[0]); pw(h[1]); pw(h[2]); pw(0); pw(0x7F990000); pw(0x7F990002); pw(0x7F99000C + 4); pw(0x7F990004 + 0x100000000ull);
    printf("first message of each: %d %d %d\n", lg[0].h == h[0] && lg[0].inw, lg[5].h == h[1] && lg[5].inw, lg[10].h == h[2] && lg[10].inw);   /* decision 0219: five calls per window (was 3), so the second and third windows start at calls 5 and 10 */
"#,
    );
    assert_eq!(
        l,
        want(&[
            "7f990004 7f990008 7f99000c count 3",
            "win 7f990004 wndproc=2000 hinst=400000 style=80000000 ex=80 atom=c000 live=1",
            "win 7f990008 wndproc=2000 hinst=400000 style=80000000 ex=80 atom=c000 live=1",
            "win 7f99000c wndproc=2000 hinst=400000 style=80000000 ex=80 atom=c000 live=1",
            "win 0 none",
            "win 7f990000 none",
            "win 7f990002 none",
            "win 7f990010 none",
            "win 17f990004 none",
            "first message of each: 1 1 1",
        ])
    );
}

#[test]
fn the_low_dwords_of_the_register_and_stack_slots_are_what_counts_for_the_int_and_dword_arguments() {
    // Win64: a DWORD or int argument leaves the upper half of its register / 8-byte slot undefined (the guest stores x, y, cx, cy with
    // 4-byte `mov DWORD PTR [rsp+0x20], 0`); HWND, HMENU, HINSTANCE and LPVOID are 64-bit and are compared whole
    let l = lines(
        "win-low-dwords",
        r#"
    uint64_t h;
    REGP("TPUtilWindow", VA_REC);
    h = cw_raw(0xDEADBEEF00000080ull, S("TPUtilWindow"), 0, 0xDEADBEEF80000000ull, 0xDEADBEEF00000000ull, 0xCAFE000000000000ull,
               0x1234567800000000ull, 0xFFFFFFFF00000000ull, 0, 0, 0x400000, 0);
    printf("h %llx\n", U(h));
    dump_log();
"#,
    );
    assert_eq!(l[0], "h 7f990004");
    assert!(l.contains(&"cs0 create=0 inst=400000 menu=0 parent=0 cy=0 cx=0 y=0 x=0 style=80000000 pad1=0 pad2=0 ex=80".to_string()), "{l:?}");
    assert!(l.contains(&"rect1 0 0 0 0".to_string()), "{l:?}");
}

#[test]
fn a_null_or_empty_window_name_is_the_same_and_the_name_pointer_is_passed_through() {
    let l = lines(
        "win-name",
        r#"
    uint64_t a, b;
    REGP("TPUtilWindow", VA_REC);
    a = cw_raw(0x80, S("TPUtilWindow"), 0, 0x80000000u, 0, 0, 0, 0, 0, 0, 0x400000, 0);
    b = cw_raw(0x80, S("TPUtilWindow"), A(empty), 0x80000000u, 0, 0, 0, 0, 0, 0, 0x400000, 0);
    printf("%llx %llx name0 %llx name1 %d\n", U(a), U(b), U(*(uint64_t *)(lg[0].cs + 56)), *(uint64_t *)(lg[5].cs + 56) == A(empty));   /* decision 0219: each window has five calls now (the first window's calls are 0..4) */
"#,
    );
    assert_eq!(l, ["7f990004 7f990008 name0 0 name1 1"]);
}

#[test]
fn nccreate_false_gives_null_and_stops_the_creation_after_one_message() {
    let l = lines(
        "win-nccreate-false",
        r#"
    uint64_t h, h2;
    REGP("TPUtilWindow", VA_REC);
    rv[0] = 0;
    SETLE(0x2222);
    h = CW_OK("TPUtilWindow");
    printf("h %llx msgs %u count %llu last error %x r4 %d\n", U(h), ln, U(fle_win_count()), LE(), R[4] == E0);
    pw(0x7F990004);
    printf("msg %llx\n", U(lg[0].msg));
    rv[0] = 1;
    h2 = CW_OK("TPUtilWindow");
    printf("next %llx msgs %u count %llu\n", U(h2), ln, U(fle_win_count()));
    pw(0x7F990004); pw(0x7F990008);
"#,
    );
    assert_eq!(
        l,
        want(&[
            "h 0 msgs 1 count 1 last error 2222 r4 1",
            "win 7f990004 wndproc=2000 hinst=400000 style=80000000 ex=80 atom=c000 live=0",
            "msg 81",
            "next 7f990008 msgs 6 count 2",   // decision 0219: 1 (the failed window) + 5 (NCCREATE, NCCALCSIZE, CREATE, SIZE, MOVE); was 4
            "win 7f990004 wndproc=2000 hinst=400000 style=80000000 ex=80 atom=c000 live=0",
            "win 7f990008 wndproc=2000 hinst=400000 style=80000000 ex=80 atom=c000 live=1",
        ])
    );
}

#[test]
fn create_minus_one_gives_null_after_all_three_messages_and_sends_nothing_else() {
    let l = lines(
        "win-create-m1",
        r#"
    uint64_t h;
    REGP("TPUtilWindow", VA_REC);
    rv[2] = UINT64_MAX;
    SETLE(0x3333);
    h = CW_OK("TPUtilWindow");
    printf("h %llx msgs %u count %llu last error %x r4 %d\n", U(h), ln, U(fle_win_count()), LE(), R[4] == E0);
    printf("order %llx %llx %llx\n", U(lg[0].msg), U(lg[1].msg), U(lg[2].msg));
    pw(0x7F990004);
"#,
    );
    assert_eq!(
        l,
        want(&[
            "h 0 msgs 3 count 1 last error 3333 r4 1",
            "order 81 83 1",
            "win 7f990004 wndproc=2000 hinst=400000 style=80000000 ex=80 atom=c000 live=0",
        ])
    );
}

/// Decision 0123, choice S-wmcreate (old: `a_window_procedure_result_that_the_pages_give_no_meaning_to_stops_the_run`: a WM_NCCREATE result other than 0 and 1 and a WM_CREATE result other than 0 and -1 stopped the run): the guest's
/// own handler of the class TLightweightLayeredControl (wall 43) returns 1 for WM_CREATE, so a nonzero WM_NCCREATE result continues like TRUE and a WM_CREATE result other than the whole 64-bit -1 continues like zero. A WM_NCCALCSIZE
/// result other than zero still stops (the page says the application should return zero and the guest does).
#[test]
fn a_window_procedure_result_that_the_pages_give_no_meaning_to_continues_except_for_nccalcsize_since_decision_0123() {
    let setup = "REGP(\"TPUtilWindow\", VA_REC);";
    let l = lines(
        "win-result-continues",
        &format!(
            "{setup}\n    rv[0] = 2; printf(\"nccreate 2 live %d\\n\", CW_LIVE(\"TPUtilWindow\"));\n    rv[0] = 1; rv[2] = 1; printf(\"create 1 live %d\\n\", CW_LIVE(\"TPUtilWindow\"));\n    rv[2] = 0xFFFFFFFFull; printf(\"create low -1 live %d\\n\", CW_LIVE(\"TPUtilWindow\"));\n    rv[2] = UINT64_MAX; printf(\"create -1 live %d\\n\", CW_LIVE(\"TPUtilWindow\"));"
        ),
    );
    assert_eq!(l, ["nccreate 2 live 1", "create 1 live 1", "create low -1 live 1", "create -1 live 0"]);
    assert_aborts(
        "win-abort-nccalcsize-result",
        setup,
        "CreateWindowExW",
        "WM_NCCALCSIZE (wParam FALSE) returned",
        &[
            ("one", "rv[1] = 1; CW_OK(\"TPUtilWindow\");"),
            ("redraw", "rv[1] = 0x300; CW_OK(\"TPUtilWindow\");"),
            ("high", "rv[1] = 0x100000000ull; CW_OK(\"TPUtilWindow\");"),
            ("minus1", "rv[1] = UINT64_MAX; CW_OK(\"TPUtilWindow\");"),
        ],
    );
}

#[test]
fn a_class_that_the_guest_registers_itself_and_has_not_registered_yet_makes_the_creation_fail_with_null() {
    let l = lines(
        "win-class-not-yet",
        r#"
    uint64_t h;
    SETLE(0x4444);
    h = CW_OK("TPUtilWindow");
    printf("h %llx count %llu msgs %u last error %x r4 %d\n", U(h), U(fle_win_count()), ln, LE(), R[4] == E0);
    REGP("TPUtilWindow", VA_REC);
    h = CW_OK("TPUtilWindow");
    printf("after register %llx count %llu msgs %u\n", U(h), U(fle_win_count()), ln);
    h = CW_OK("TApplication");
    printf("other %llx count %llu msgs %u\n", U(h), U(fle_win_count()), ln);
"#,
    );
    assert_eq!(
        l,
        want(&[
            "h 0 count 0 msgs 0 last error 4444 r4 1",
            "after register 7f990004 count 1 msgs 5",   // decision 0219: was msgs 3 (WM_SIZE and WM_MOVE follow WM_CREATE of a WS_POPUP window)
            "other 0 count 1 msgs 5",
        ])
    );
}

#[test]
fn the_window_procedure_may_replace_itself_in_wm_nccreate_and_the_next_messages_go_to_the_new_one() {
    // the guest's TLightweightLayeredControl (0x71dd10): in WM_NCCREATE it calls SetWindowLongPtrW(hwnd, GWLP_WNDPROC, ...) and returns 1
    let l = lines(
        "win-self-replace",
        r#"
    uint64_t h;
    REGP("TLightweightLayeredControl", VA_SETP);
    h = CW_OK("TLightweightLayeredControl");
    printf("h %llx prev %llx msgs %u\n", U(h), U(g_prev), ln);
    pw(h);
    dump_log();
"#,
    );
    assert_eq!(l[0], "h 7f990004 prev 2020 msgs 5");   // decision 0219: was 3 (+ WM_SIZE, WM_MOVE)
    assert_eq!(l[1], "win 7f990004 wndproc=2010 hinst=400000 style=80000000 ex=80 atom=c000 live=1");
    assert!(l.contains(&"m0 tag=7 h=7f990004 msg=81 wp=0 depth=1 inw=1 live=1 wndproc=2020 ret=1".to_string()), "{l:?}");
    assert!(l.contains(&"m1 tag=2 h=7f990004 msg=83 wp=0 depth=1 inw=1 live=1 wndproc=2010 ret=0".to_string()), "{l:?}");
    assert!(l.contains(&"m2 tag=2 h=7f990004 msg=1 wp=0 depth=1 inw=1 live=1 wndproc=2010 ret=0".to_string()), "{l:?}");
}

#[test]
fn a_window_procedure_may_create_another_window_and_the_stack_stays_intact() {
    let l = lines(
        "win-nested",
        r#"
    uint64_t h;
    REGP("TPUtilWindow", VA_NEST);
    REGP("TTimerWindow", VA_REC);
    g_nest_once = 1;
    h = CW_OK("TPUtilWindow");
    printf("outer %llx inner %llx count %llu msgs %u depth %u r4 %d above %llu\n", U(h), U(g_inner), U(fle_win_count()), ln, fle_callback_depth(), R[4] == E0, U(g_above));
    { unsigned i; for (i = 0; i < ln; i++) printf("m%u tag=%d h=%llx msg=%llx depth=%u\n", i, lg[i].tag, U(lg[i].h), U(lg[i].msg), lg[i].depth); }
    printf("outer cs intact %d\n", memcmp(lg[0].cs, lg[7].cs, 80) == 0);   /* decision 0219: the outer WM_CREATE is call 7 now (was 5) */
    pw(h); pw(g_inner);
"#,
    );
    assert_eq!(
        l,
        want(&[
            "outer 7f990004 inner 7f990008 count 2 msgs 10 depth 0 r4 1 above 0",   // decision 0219: was msgs 6; each window now also gets WM_SIZE (5) and WM_MOVE (3) after WM_CREATE
            "m0 tag=3 h=7f990004 msg=81 depth=1",
            "m1 tag=1 h=7f990008 msg=81 depth=2",
            "m2 tag=1 h=7f990008 msg=83 depth=2",
            "m3 tag=1 h=7f990008 msg=1 depth=2",
            "m4 tag=1 h=7f990008 msg=5 depth=2",
            "m5 tag=1 h=7f990008 msg=3 depth=2",
            "m6 tag=3 h=7f990004 msg=83 depth=1",
            "m7 tag=3 h=7f990004 msg=1 depth=1",
            "m8 tag=3 h=7f990004 msg=5 depth=1",
            "m9 tag=3 h=7f990004 msg=3 depth=1",
            "outer cs intact 1",
            "win 7f990004 wndproc=2030 hinst=400000 style=80000000 ex=80 atom=c000 live=1",
            "win 7f990008 wndproc=2000 hinst=400000 style=80000000 ex=80 atom=c001 live=1",
        ])
    );
}

#[test]
fn the_guests_own_window_procedure_is_the_defwindowproc_thunk_and_the_three_messages_get_1_0_0() {
    // the call at 0x52a8be: class TPUtilWindow, lpfnWndProc 0x424920 (the thunk of user32!DefWindowProcW)
    let l = lines(
        "win-guest-def",
        r#"
    uint64_t h;
    REGP("TPUtilWindow", VA_DEF);
    SETLE(0x5555);
    h = CW_OK("TPUtilWindow");
    printf("h %llx last error %x count %llu\n", U(h), LE(), U(fle_win_count()));
    dump_log();
"#,
    );
    assert_eq!(l[0], "h 7f990004 last error 5555 count 1");
    for (i, (msg, ret)) in [(0x81, 1), (0x83, 0), (0x01, 0)].iter().enumerate() {
        let want = format!("m{i} tag=9 h=7f990004 msg={msg:x} wp=0 depth=1 inw=1 live=1 wndproc=424920 ret={ret:x}");
        assert!(l.contains(&want), "{want}\n{l:?}");
    }
}

#[test]
fn the_class_window_procedure_must_be_a_function_entry_and_a_value_that_is_not_stops_by_name() {
    // Delphi's MakeObjectInstance gives SetWindowLongPtrW the address of a block the guest wrote itself (docs/investigations/
    // window-design.md section 3.3): kept as a number, called only by a later message, which this table does not send
    let o = run("win-nonentry-class", "REGP(\"TPUtilWindow\", 0x12345); CW_OK(\"TPUtilWindow\"); printf(\"returned\\n\");");
    assert!(!o.ok && !o.stdout.contains("returned"), "{}", o.stdout);
    assert_eq!(o.signal, Some(6));
    assert!(o.stderr.contains("fle: indirect call to 0x12345 is not a known function entry"), "{}", o.stderr);
}

// ---- 3. CreateWindowExW: the forms that stop ----

fn cwc(over: &[(usize, &str)]) -> String {
    let mut a: Vec<String> = ["0x80", "S(\"TPUtilWindow\")", "A(empty)", "0x80000000u", "0", "0", "0", "0", "0", "0", "0x400000", "0"].iter().map(|s| s.to_string()).collect();
    for (i, v) in over {
        a[*i] = v.to_string();
    }
    format!("cw_raw({});", a.join(", "))
}

fn cw_cases(over: &[(&str, usize, &str)]) -> Vec<(String, String)> {
    over.iter().map(|(l, i, v)| (l.to_string(), cwc(&[(*i, v)]))).collect()
}

fn aborts_cw(prefix: &str, setup: &str, word: &str, cases: &[(&str, usize, &str)]) {
    let c = cw_cases(cases);
    let refs: Vec<(&str, &str)> = c.iter().map(|(l, s)| (l.as_str(), s.as_str())).collect();
    assert_aborts(prefix, setup, "CreateWindowExW", word, &refs);
}

const REG_SETUP: &str = "REGP(\"TPUtilWindow\", VA_REC);";

/// Decision 0123 (old: `only_the_tool_window_style_pair_is_answered_other_extended_styles_and_styles_stop`: dwExStyle other than WS_EX_TOOLWINDOW and dwStyle other than WS_POPUP / 0x84ca0000 stopped the run): every style and
/// extended style is answered and kept as given (the page of CreateWindowExW lists the values; the window model of decision 0123 follows WS_VISIBLE, WS_CHILD and WS_DISABLED). The forms that still stop are the ones the pages leave open.
#[test]
fn every_style_and_extended_style_is_kept_as_given_since_decision_0123() {
    let l = lines(
        "win-styles-kept",
        r#"
    REGP("TPUtilWindow", VA_REC);
    uint64_t a = cw_raw(0, S("TPUtilWindow"), A(empty), 0x00CF0000u, 0, 0, 0, 0, 0, 0, 0x400000, 0);
    uint64_t b = cw_raw(0x81, S("TPUtilWindow"), A(empty), 0x80000001u, 0, 0, 0, 0, 0, 0, 0x400000, 0);
    uint64_t c = cw_raw(0x08000080, S("TPUtilWindow"), A(empty), 0x80000000u, 0, 0, 0, 0, 0, 0, 0x400000, 0);
    fle_win_view v;
    fle_win_get(a, &v); printf("a style %x ex %x live %d\n", (unsigned)v.style, (unsigned)v.exstyle, v.live);
    fle_win_get(b, &v); printf("b style %x ex %x live %d\n", (unsigned)v.style, (unsigned)v.exstyle, v.live);
    fle_win_get(c, &v); printf("c style %x ex %x live %d\n", (unsigned)v.style, (unsigned)v.exstyle, v.live);
"#,
    );
    assert_eq!(l, ["a style cf0000 ex 0 live 1", "b style 80000001 ex 81 live 1", "c style 80000000 ex 8000080 live 1"]);
}

/// Decision 0123 (old: `only_a_zero_position_and_size_is_answered_cw_usedefault_and_any_other_value_stop`): the position and size are kept as given, in the coordinates of the Learn page of CreateWindowExW (screen
/// coordinates for a pop-up or overlapped window, the parent's client coordinates for a child); CW_USEDEFAULT is valid only for an overlapped window: for a pop-up window x, y and the size become 0 (Learn); for an overlapped
/// window the default is the choice S-default of the decision (x = y = 0 and the rest of the virtual screen).
#[test]
fn the_position_and_size_are_kept_and_cw_usedefault_follows_the_page_since_decision_0123() {
    let l = lines(
        "win-rect-kept",
        r#"
    REGP("TPUtilWindow", VA_REC);
    uint64_t a = cw_raw(0x80, S("TPUtilWindow"), A(empty), 0x80000000u, 1, 2, 3, 4, 0, 0, 0x400000, 0);
    uint64_t b = cw_raw(0x80, S("TPUtilWindow"), A(empty), 0x80000000u, 0x80000000u, 0, 0x80000000u, 0, 0, 0, 0x400000, 0);
    uint64_t c = cw_raw(0x80, S("TPUtilWindow"), A(empty), 0x00CF0000u, 0x80000000u, 0, 0x80000000u, 0, 0, 0, 0x400000, 0);
    fle_win_view v;
    fle_win_get(a, &v); printf("a %d %d %d %d\n", v.x, v.y, v.cx, v.cy);
    fle_win_get(b, &v); printf("b %d %d %d %d\n", v.x, v.y, v.cx, v.cy);
    fle_win_get(c, &v); printf("c %d %d %d %d\n", v.x, v.y, v.cx, v.cy);
"#,
    );
    assert_eq!(l, ["a 1 2 3 4", "b 0 0 0 0", "c 0 0 1920 1080"]);
}

/// Decision 0123 (old: `a_parent_a_menu_an_instance_a_creation_parameter_and_a_window_text_stop`: a window handle as hWndParent, a non-NULL hMenu, a creation parameter and a window name stopped the run): a live window as hWndParent makes
/// the new window its child (WS_CHILD) or owned (otherwise); the window name is kept (up to 256 units); lpParam goes to CREATESTRUCT.lpCreateParams. What still stops: an hMenu for a window that is not a child, an hInstance that is not
/// the lifted DLL, a name longer than 256 units; a window handle that is not live fails with NULL and 1400.
#[test]
fn a_parent_an_owner_a_window_name_and_a_creation_parameter_are_answered_since_decision_0123() {
    let l = lines(
        "win-parent-owner",
        r#"
    REGP("TPUtilWindow", VA_REC);
    uint64_t top = cw_raw(0x80, S("TPUtilWindow"), S("main"), 0x80000000u, 10, 20, 300, 200, 0, 0, 0x400000, 0);
    uint64_t child = cw_raw(0, S("TPUtilWindow"), S("kid"), 0x40000000u, 5, 6, 100, 50, top, 7, 0x400000, 0x1234);
    uint64_t owned = cw_raw(0x80, S("TPUtilWindow"), A(empty), 0x80000000u, 0, 0, 0, 0, top, 0, 0x400000, 0);
    uint64_t bad;
    SETLE(0);
    bad = cw_raw(0x80, S("TPUtilWindow"), A(empty), 0x80000000u, 0, 0, 0, 0, 0x7F99FFF0ull, 0, 0x400000, 0);
    printf("top %d child %d owned %d\n", top != 0, child != 0, owned != 0);
    printf("bad %llx le %u\n", U(bad), LE());
    R[1] = child; fle_imp_GetParent(); printf("parent of child is top %d\n", R[0] == top);
    R[1] = owned; fle_imp_GetParent(); printf("parent of owned (WS_POPUP, owner top has WS_POPUP) %llx\n", U(R[0]));
    R[1] = owned; R[2] = 4; fle_imp_GetWindow(); printf("owner of owned is top %d\n", R[0] == top);
    R[1] = child; R[2] = 1; fle_imp_GetAncestor(); printf("ga_parent of child is top %d\n", R[0] == top);
"#,
    );
    assert_eq!(l[0], "top 1 child 1 owned 1");
    assert_eq!(l[1], "bad 0 le 1400");
    assert_eq!(l[2], "parent of child is top 1");
    assert_eq!(l[4], "owner of owned is top 1");
    assert_eq!(l[5], "ga_parent of child is top 1");
    aborts_cw("win-abort-menu", REG_SETUP, "hMenu is not NULL", &[("one", 9, "1"), ("pointer-like", 9, "0x7F990004ull"), ("high", 9, "0x100000000ull")]);
    // the word is the tail of the message of fle_win.c itself: fle_class_lookup checks hInstance too (with another message), so a
    // check of the same value in both places must still be seen to come from CreateWindowExW first
    aborts_cw(
        "win-abort-hinst",
        REG_SETUP,
        "hInstance is not the lifted DLL (0x400000): not answered (decision 0071)",
        &[("null", 10, "0"), ("one", 10, "1"), ("next", 10, "0x400001"), ("module-token", 10, "0x7FF800010000ull"), ("high", 10, "0x100400000ull")],
    );
}

#[test]
fn a_class_name_that_is_not_a_string_of_the_table_stops_by_name() {
    let setup = "REGP(\"TPUtilWindow\", VA_REC); static uint16_t big[300]; static uint16_t na[] = { 'T', 0x141, 0 }; { unsigned i; for (i = 0; i < 299; i++) big[i] = 'x'; big[257] = 0; }";
    aborts_cw("win-abort-class-atom", setup, "atom or NULL", &[("atom", 1, "0xC000"), ("atom-max", 1, "0xFFFF"), ("null", 1, "0"), ("atom-1", 1, "1")]);
    aborts_cw("win-abort-class-empty", setup, "empty", &[("empty", 1, "A(empty)")]);
    aborts_cw("win-abort-class-long", setup, "longer than 256", &[("257", 1, "A(big)")]);
    aborts_cw("win-abort-class-ascii", setup, "above 0x7F", &[("n", 1, "A(na)")]);
    aborts_cw(
        "win-abort-class-unknown",
        setup,
        "is a system class",
        // decision 0215 (sweep1): only the system classes stop; every other unregistered name is "not found" (next assertion)
        &[("edit", 1, "S(\"EDIT\")"), ("button", 1, "S(\"BUTTON\")"), ("mdiclient", 1, "S(\"MDICLIENT\")")],
    );
    let l = lines(
        "win-class-unknown-not-found",
        &format!(
            r#"
    {setup}
    uint64_t r;
    r = cw_raw(0x80, S("RICHEDIT20W"), 0, 0x80000000u, 0, 0, 0, 0, 0, 0, 0x400000, 0); printf("richedit %llu le %u\n", U(r), LE());
    r = cw_raw(0x80, S("ICMCALLBACKWND"), 0, 0x80000000u, 0, 0, 0, 0, 0, 0, 0x400000, 0); printf("icm %llu le %u\n", U(r), LE());
    r = cw_raw(0x80, S("X"), 0, 0x80000000u, 0, 0, 0, 0, 0, 0, 0x400000, 0); printf("x %llu le %u\n", U(r), LE());
"#
        ),
    );
    // the creation answers NULL as it already did for a `T` + upper-case class name; the last error is left alone (a real system sets ERROR_CANNOT_FIND_WND_CLASS 1407: a request for fle_win.c, decision 0215 R12)
    assert_eq!(l, ["richedit 0 le 0", "icm 0 le 0", "x 0 le 0"]);
    aborts_cw("win-abort-class-case", setup, "differs from a registered one only in case", &[("lower", 1, "S(\"tputilwindow\")"), ("upper", 1, "S(\"TPUTILWINDOW\")")]);
}

#[test]
fn nothing_is_stored_and_no_message_is_sent_before_a_stop() {
    let top = "#include <signal.h>\n#include <unistd.h>\nstatic void hnd(int s) { (void)s; printf(\"count %llu msgs %u\\n\", U(fle_win_count()), ln); fflush(stdout); _exit(7); }";
    // decision 0123 (old: "style" (dwStyle 0), "parent" (hWndParent 1) and "param" (lpParam 1) were in this list): those are answered; the forms that still stop before anything is stored are a system class name and an hMenu
    // for a window that is not a child, and an hInstance that is not the lifted DLL
    for (label, call) in [
        ("class", cwc(&[(1, "S(\"EDIT\")")])),
        ("menu", cwc(&[(9, "1")])),
        ("hinst", cwc(&[(10, "1")])),
    ] {
        let o = run_src(&format!("win-nothing-{label}"), top, &format!("    signal(SIGABRT, hnd);\n    REGP(\"TPUtilWindow\", VA_REC);\n    CW_OK(\"TPUtilWindow\");\n    {call}"));
        assert_eq!(o.stdout, "count 1 msgs 5\n", "{label}: {}", o.stderr);   // decision 0219: was msgs 3
    }
}

// ---- 4. SetWindowLongPtrW ----

#[test]
fn set_window_long_ptr_replaces_the_window_procedure_and_returns_the_previous_one() {
    let l = lines(
        "win-swl",
        r#"
    uint64_t h, a, b, c;
    REGP("TPUtilWindow", VA_REC);
    h = CW_OK("TPUtilWindow");
    SETLE(0x6666);
    memset(stk, 0, 0x100);
    a = SWL(h, 0xFFFFFFFCull, VA_REC2);
    printf("a %llx\n", U(a));
    pw(h);
    b = SWL(h, 0xDEADBEEFFFFFFFFCull, 0x7FFF12345678ull);
    printf("b %llx\n", U(b));
    pw(h);
    c = SWL(h, (uint64_t)(int64_t)-4, VA_REC);
    printf("c %llx last error %x\n", U(c), LE());
    pw(h);
    printf("count %llu msgs %u\n", U(fle_win_count()), ln);
    memset(stk, 0, 0x100);
    R[1] = h; R[2] = (uint64_t)(int64_t)-4; R[8] = VA_NEST; R[9] = 0x77; R[10] = 0x88; R[11] = 0x99; R[4] = A(stk); stk[0] = RET0;
    fle_imp_SetWindowLongPtrW();
    printf("regs r9 %llx r4 %d ret-slot %llx r0 %llx\n", U(R[9]), R[4] == A(stk), U(stk[0]), U(R[0]));
"#,
    );
    assert_eq!(
        l,
        want(&[
            "a 2000",
            "win 7f990004 wndproc=2010 hinst=400000 style=80000000 ex=80 atom=c000 live=1",
            "b 2010",
            "win 7f990004 wndproc=7fff12345678 hinst=400000 style=80000000 ex=80 atom=c000 live=1",
            "c 7fff12345678 last error 6666",
            "win 7f990004 wndproc=2000 hinst=400000 style=80000000 ex=80 atom=c000 live=1",
            "count 1 msgs 5",   // decision 0219: was msgs 3
            "regs r9 77 r4 1 ret-slot dead0000 r0 2000",
        ])
    );
}

#[test]
fn set_window_long_ptr_stops_for_every_index_but_gwlp_wndproc_and_for_a_null_procedure() {
    let setup = "REGP(\"TPUtilWindow\", VA_REC); { uint64_t h = CW_OK(\"TPUtilWindow\"); (void)h; }";
    let idx = |l: &str, i: i64| (l.to_string(), format!("SWL(H1, (uint64_t)(int64_t){i}, 5);"));
    let cases: Vec<(String, String)> = vec![
        // decision 0119 (wall 17): GWL_EXSTYLE (-20) and GWLP_USERDATA (-21) are answered now (old: stop; new: tests/wall_fr6.rs pins their answers)
        // decision 0123 (wall 34; old: "gwl-style" -16, "gwlp-id" -12, "gwlp-hwndparent" -8 and "gwlp-hinstance" -6 were in this list): answered (tests/wall_fr9.rs pins their answers)
        idx("extra-0", 0),
        idx("extra-8", 8),
        idx("dwlp-user", 16),
        idx("minus-3", -3),
        idx("minus-5", -5),
        ("high-set-low-other".to_string(), "SWL(H1, 0xFFFFFFFF00000000ull, 5);".to_string()),
        ("positive-4".to_string(), "SWL(H1, 4, 5);".to_string()),
        ("unsigned-fffffffc-as-u32-1".to_string(), "SWL(H1, 0xFFFFFFFCull + 0x100000000ull * 0 + 1, 5);".to_string()),
    ];
    let refs: Vec<(&str, &str)> = cases.iter().map(|(l, s)| (l.as_str(), s.as_str())).collect();
    assert_aborts("win-abort-swl-index", setup, "SetWindowLongPtrW", "is not GWLP_WNDPROC (-4), GWL_EXSTYLE", &refs);
    assert_aborts("win-abort-swl-null", setup, "SetWindowLongPtrW", "dwNewLong is 0", &[("null", "SWL(H1, (uint64_t)(int64_t)-4, 0);")]);
}

#[test]
fn set_window_long_ptr_and_def_window_proc_stop_for_a_handle_that_is_not_a_live_window_of_the_table() {
    let setup = "REGP(\"TPUtilWindow\", VA_REC); CW_OK(\"TPUtilWindow\"); rv[0] = 0; CW_OK(\"TPUtilWindow\"); rv[0] = 1;";
    let bad = [
        ("null", "0"),
        ("never-issued-next", "0x7F99000C"),
        ("base", "0x7F990000"),
        ("misaligned", "0x7F990006"),
        ("below-region", "0x7F98FFFC"),
        ("above-region", "0x7FA90004"),
        ("high-bits-only-low-matches", "0x17F990004ull"),
        ("sign-extended", "0xFFFFFFFF7F990004ull"),
        ("dc-handle", "0x7FA90004"),
        ("gdi-handle", "0x7FBA0004"),
    ];
    // Decision 0119 (PM's decision 1: a CHOICE WITHOUT A SOURCE): SetWindowLongPtrW for a handle that is not a live window of the table answers 0 and the last error 1400
    // (old, decision 0071: stop by name, "is not a window of this table"); tests/wall_fr6.rs pins the new answer for the other APIs, the one here is pinned below.
    for (label, h) in bad {
        let l = lines(&format!("win-swl-handle-{label}"), &format!("{setup}\n    SETLE(0x3131);\n    printf(\"r %llu le %x msgs %u\\n\", U(SWL({h}, (uint64_t)(int64_t)-4, VA_REC2)), LE(), ln);"));
        // 1400 = 0x578. msgs 6 (measured; decision 0219: was 4): the first window got its five messages (NCCREATE, NCCALCSIZE, CREATE, SIZE, MOVE), the second one only WM_NCCREATE (answered 0: the creation failed, decision 0071); the call sends none
        assert_eq!(l, want(&["r 0 le 578 msgs 6"]), "{label}");
    }
    let def: Vec<(String, String)> = bad.iter().map(|(l, h)| (l.to_string(), format!("DEF({h}, 0x81, 0, 1);"))).collect();
    let refs: Vec<(&str, &str)> = def.iter().map(|(l, s)| (l.as_str(), s.as_str())).collect();
    assert_aborts("win-abort-def-handle", setup, "DefWindowProcW", "is not a window of this table", &refs);
    // the window whose creation failed (index 1, handle 0x7F990008) is a handle that was issued and is not live
    // decision 0119: a window whose creation failed answers 0 and 1400 for SetWindowLongPtrW too (old: stop "creation failed"); DefWindowProcW still stops for it (below)
    let l = lines("win-swl-dead", &format!("{setup}\n    SETLE(0x3131);\n    printf(\"r %llu le %x\\n\", U(SWL(0x7F990008, (uint64_t)(int64_t)-4, VA_REC2)), LE());"));
    assert_eq!(l, want(&["r 0 le 578"]));
    assert_aborts("win-abort-def-dead", setup, "DefWindowProcW", "creation failed", &[("dead", "DEF(0x7F990008, 0x81, 0, 1);")]);
}

// ---- 5. DefWindowProcW ----

#[test]
fn def_window_proc_answers_nccreate_create_and_nccalcsize_with_1_0_0_and_leaves_the_rect() {
    let l = lines(
        "win-def",
        r#"
    uint64_t h, r1, r2, r3, r4, r5;
    int32_t rc[4] = { 10, 20, 30, 40 };
    int32_t rc2[4] = { 10, 20, 30, 40 };
    uint8_t csb[80];
    REGP("TPUtilWindow", VA_REC);
    h = CW_OK("TPUtilWindow");
    SETLE(0x7777);
    memset(csb, 0x5A, sizeof csb);
    r1 = DEF(h, 0x81, 0, A(csb));
    r2 = DEF(h, 0x81, 0x1234567812345678ull, A(csb));
    r3 = DEF(h, 0x01, 0, A(csb));
    r4 = DEF(h, 0x01, 0xFFFFFFFFFFFFFFFFull, A(csb));
    r5 = DEF(h, 0x83, 0, A(rc));
    printf("%llx %llx %llx %llx %llx\n", U(r1), U(r2), U(r3), U(r4), U(r5));
    printf("rect same %d cs same %d\n", memcmp(rc, rc2, 16) == 0, csb[0] == 0x5A && csb[79] == 0x5A);
    printf("high garbage in msg: %llx %llx\n", U(DEF(h, 0xDEADBEEF00000081ull, 0, A(csb))), U(DEF(h, 0xDEADBEEF00000001ull, 0, A(csb))));
    printf("last error %x count %llu msgs %u\n", LE(), U(fle_win_count()), ln);
    memset(stk, 0, 0x100); stk[0] = RET0; R[4] = A(stk); R[1] = h; R[2] = 0x81; R[8] = 0; R[9] = A(csb); R[10] = 0x55; R[11] = 0x66;
    fle_imp_DefWindowProcW();
    printf("r4 %d ret-slot %llx r10 %llx r11 %llx\n", R[4] == A(stk), U(stk[0]), U(R[10]), U(R[11]));
"#,
    );
    assert_eq!(
        l,
        want(&[
            "1 1 0 0 0",
            "rect same 1 cs same 1",
            "high garbage in msg: 1 0",
            "last error 7777 count 1 msgs 5",   // decision 0219: was msgs 3
            "r4 1 ret-slot dead0000 r10 55 r11 66",
        ])
    );
}

#[test]
fn def_window_proc_answers_zero_for_a_message_whose_learn_page_names_no_default_processing() {
    // decision 0222: tables/defwindowproc-messages.tsv (class B / C, generated into runtime/fle_defproc_zero.h) and WM_CHILDACTIVATE (0x22) / WM_PARENTNOTIFY (0x210), which the window manager sends. Learn DefWindowProc: "The
    // return value is the result of the message processing and depends on the message"; Wine 10.0's default_window_proc answers 0 for a message it has no case for. The low 32 bits are the message (decision 0123).
    let l = lines(
        "win-def-zero",
        r#"
    uint64_t h;
    REGP("TPUtilWindow", VA_REC);
    h = CW_OK("TPUtilWindow");
    printf("%llx %llx %llx %llx %llx\n", U(DEF(h, 0x0, 0, 0)), U(DEF(h, 0x113, 5, 0)), U(DEF(h, 0x22, 0, 0)), U(DEF(h, 0x210, 0x201, 0x10020)), U(DEF(h, 0x100000113ull, 5, 0)));
"#,
    );
    assert_eq!(l, ["0 0 0 0 0"]);
}

#[test]
fn def_window_proc_stops_for_every_other_message_a_true_nccalcsize_and_a_null_lparam() {
    let setup = "REGP(\"TPUtilWindow\", VA_REC); { uint64_t h = CW_OK(\"TPUtilWindow\"); (void)h; } { static uint8_t buf[80]; (void)buf; }";
    let msgs: Vec<(String, String)> = [
        // decision 0222: WM_NULL (0) and WM_TIMER (0x113) are no longer in this list: their Learn pages name no default processing of DefWindowProc, so it answers 0 (the next test pins that). WM_GETICON (0x7F) and
        // WM_QUERYOPEN (0x13) have a page that describes one (class A of tables/defwindowproc-messages.tsv) and are not implemented: they still stop by name.
        ("wm-queryopen", 0x13u64),
        // decision 0123 (old: 0x02 WM_DESTROY, 0x03 WM_MOVE, 0x05 WM_SIZE, 0x0F WM_PAINT, 0x10 WM_CLOSE, 0x14 WM_ERASEBKGND, 0x18 WM_SHOWWINDOW, 0x46 WM_WINDOWPOSCHANGING, 0x47 WM_WINDOWPOSCHANGED, 0x82
        // WM_NCDESTROY were in this list (0x113 WM_TIMER still stops); 0x8FFC, 0x8000, 0x7FFFFFFF and 0xFFFFFFFF too: the messages from WM_USER up are application messages, answered 0 (S-privmsg)): the default
        // processing of those messages is answered (tests/wall_fr9.rs pins each of them)
        ("wm-nchittest", 0x84),
        ("wm-geticon", 0x7F),
        // decision 0123 (old: 0x1_0000_0002): the message is the low 32 bits, which are WM_DESTROY (2) and answered now; decision 0222 (old: 0x1_0000_0113, WM_TIMER): 0x1_0000_007F has the low half WM_GETICON, which still stops
        ("wm-geticon-plus-2^32-low-is-other", 0x1_0000_007F),
    ]
    .iter()
    .map(|(l, m)| (l.to_string(), format!("DEF(H1, {m:#x}ull, 0, A(wcb));")))
    .collect();
    let refs: Vec<(&str, &str)> = msgs.iter().map(|(l, s)| (l.as_str(), s.as_str())).collect();
    assert_aborts("win-abort-def-msg", setup, "DefWindowProcW", "the default processing is not answered", &refs);
    // decision 0258 (alpha7): DefWindowProcW(WM_GETMINMAXINFO) leaves the defaults the message carries and answers 0 (it stopped before: "the MINMAXINFO defaults depend on the frame and the screen: not modelled"; harness/alpha7/showwin.c
    // pins the answer); only a NULL lParam stops, with its own text (Learn WM_GETMINMAXINFO: lParam "A pointer to a MINMAXINFO structure")
    assert_aborts("win-abort-def-minmax", setup, "DefWindowProcW", "MINMAXINFO", &[("wm-getminmaxinfo-null-lparam", "DEF(H1, 0x24, 0, 0);")]);
    assert_aborts(
        "win-abort-def-form",
        setup,
        "DefWindowProcW",
        "WM_",
        &[
            ("nccalcsize-true", "DEF(H1, 0x83, 1, A(wcb));"),
            ("nccalcsize-nonzero-wp", "DEF(H1, 0x83, 0x100000000ull, A(wcb));"),
            ("nccalcsize-null", "DEF(H1, 0x83, 0, 0);"),
            ("nccreate-null", "DEF(H1, 0x81, 0, 0);"),
            ("create-null", "DEF(H1, 0x01, 0, 0);"),
        ],
    );
}

// ---- 6. fle_class_lookup (decision 0071 adds it to fle_class.c; the class table is decision 0070) ----

#[test]
fn the_class_lookup_gives_the_atom_and_the_window_procedure_of_a_registered_class_and_zero_for_the_guests_own_unregistered_names() {
    let l = lines(
        "win-lookup",
        r#"
    uint16_t at = 0xBEEF;
    uint64_t wp = 0x1111;
    int r;
    r = fle_class_lookup("T", S("TTimerWindow"), 0x400000, &at, &wp);
    printf("unregistered own: %d at %x wp %llx\n", r, at, U(wp));
    REGP("TApplication", VA_REC);
    REGP("TTimerWindow", VA_REC2);
    r = fle_class_lookup("T", S("TTimerWindow"), 0x400000, &at, &wp);
    printf("registered: %d at %x wp %llx\n", r, at, U(wp));
    r = fle_class_lookup("T", S("TApplication"), 0x400000, &at, &wp);
    printf("first: %d at %x wp %llx count %llu\n", r, at, U(wp), U(fle_class_count()));
    printf("last error untouched %d\n", LE() == 0);
"#,
    );
    assert_eq!(
        l,
        want(&["unregistered own: 0 at beef wp 1111", "registered: 1 at c001 wp 2010", "first: 1 at c000 wp 2000 count 2", "last error untouched 1"])
    );
}

#[test]
fn the_class_lookup_stops_for_another_module_and_for_every_name_the_class_table_does_not_answer() {
    assert_aborts(
        "win-lookup-stop",
        "REGP(\"TApplication\", VA_REC); uint16_t at; uint64_t wp;",
        "T",
        "",
        &[
            ("hinst-null", "fle_class_lookup(\"T\", S(\"TApplication\"), 0, &at, &wp);"),
            ("hinst-other", "fle_class_lookup(\"T\", S(\"TApplication\"), 0x400001, &at, &wp);"),
            ("name-null", "fle_class_lookup(\"T\", 0, 0x400000, &at, &wp);"),
            ("name-atom", "fle_class_lookup(\"T\", 0xC000, 0x400000, &at, &wp);"),
            ("name-case", "fle_class_lookup(\"T\", S(\"tapplication\"), 0x400000, &at, &wp);"),
            ("name-edit", "fle_class_lookup(\"T\", S(\"EDIT\"), 0x400000, &at, &wp);"),
        ],
    );
}

// ---- 7. Threads ----

#[test]
fn a_second_thread_stops_the_run_in_all_three_apis() {
    for (label, api, call) in [
        ("create", "CreateWindowExW", "CW_OK(\"TTimerWindow\")"),
        ("setlong", "SetWindowLongPtrW", "SWL(H1, (uint64_t)(int64_t)-4, VA_REC2)"),
        ("def", "DefWindowProcW", "DEF(H1, 0x81, 0, 1)"),
    ] {
        let o = run_src(
            &format!("win-thread-{label}"),
            &format!("static void *th(void *x) {{ (void)x; {call}; printf(\"returned-from-thread\\n\"); return 0; }}"),
            "    pthread_t t;\n    REGP(\"TPUtilWindow\", VA_REC);\n    CW_OK(\"TPUtilWindow\");\n    pthread_create(&t, 0, th, 0);\n    pthread_join(t, 0);\n    printf(\"returned\\n\");",
        );
        assert!(!o.ok && !o.stdout.contains("returned"), "{label}: {}", o.stdout);
        assert_eq!(o.signal, Some(6), "{label}");
        assert!(o.stderr.contains(&format!("fle: {api}: ")) && o.stderr.contains("second thread"), "{label}: {}", o.stderr);
    }
}

#[test]
fn a_second_thread_stops_the_class_lookup_called_directly() {
    // fle_class_lookup is a public function of fle_class.c: the single-thread rule of that file holds for it on its own, not only
    // through CreateWindowExW (which has its own check first)
    let o = run_src(
        "win-thread-lookup",
        "static void *th(void *x) { uint16_t a; uint64_t w; (void)x; fle_class_lookup(\"T\", S(\"TApplication\"), 0x400000, &a, &w); printf(\"returned-from-thread\\n\"); return 0; }",
        "    pthread_t t;\n    REGP(\"TApplication\", VA_REC);\n    pthread_create(&t, 0, th, 0);\n    pthread_join(t, 0);\n    printf(\"returned\\n\");",
    );
    assert!(!o.ok && !o.stdout.contains("returned"), "{}", o.stdout);
    assert_eq!(o.signal, Some(6));
    assert!(o.stderr.contains("fle: T: ") && o.stderr.contains("second thread"), "{}", o.stderr);
}

// ---- 8. Source checks ----

#[test]
fn source_uses_no_host_window_display_queue_clock_thread_process_or_file_and_calls_the_window_procedure_only_through_fle_call_guest() {
    let code = strip_comments(&std::fs::read_to_string(src_dir().join("fle_win.c")).unwrap());
    for bad in [
        "getpid", "getppid", "gettid", "getenv", "environ", "fopen", "open(", "stat(", "/proc", "/etc", "/home", "/dev", "/tmp", "exit(", "_exit",
        "clock_gettime", "gettimeofday", "time(", "clock(", "nanosleep", "usleep", "sleep(", "select(", "<time.h>", "<sys/time.h>", "pthread", "<sys/",
        "<unistd.h>", "<signal.h>", "fork(", "exec", "syscall", "sched_", "futex", "asm", "__asm__", "abort(", "X11", "DISPLAY", "xrandr",
        "wayland", "fb0", "ioctl", "emscripten", "SDL", "dlopen", "LoadLibrary", "fle_load_", "fle_res_", "GetLastError", "setjmp",
        "longjmp", "fle_dispatch_table", "(void (*)",
    ] {
        assert!(!code.contains(bad), "fle_win.c must not use {bad}");
    }
    // decision 0323 (1) (old: "uintptr_t" was in the list above): def_erase_background (DefWindowProc(WM_ERASEBKGND) filling with the class background brush) hands the address of its own 16-byte RECT, a local array, to
    // GetClipBox / FillRect as a guest address (guest addresses are host addresses in this runtime: fle_addr is the identity, or the high-32 check): the one use of uintptr_t in fle_win.c
    assert_eq!(code.matches("uintptr_t").count(), 1);
    let at = code.find("uintptr_t").unwrap();
    assert!(code[..at].rfind("static uint64_t def_erase_background(").is_some_and(|f| !code[f..at].contains("\n}\n")), "the one uintptr_t is inside def_erase_background");
    // decision 0123 (old: "fle_dispatch(" was in the list above): wall 31 adds exactly one call, in fle_win_try_thunk (the guest's own `call [reg+disp]` of the MakeObjectInstance slot, reached from fle_dispatch_miss)
    assert_eq!(code.matches("fle_dispatch(").count(), 1);
    let at = code.find("fle_dispatch(").unwrap();
    assert!(code[..at].rfind("int fle_win_try_thunk(uint64_t va)").is_some_and(|f| !code[f..at].contains("\n}\n")), "the one fle_dispatch( call is inside fle_win_try_thunk");
    // Decision 0119 (PM's decision 1) lets fle_win.c set the last error, in one place only: invalid_window() answers 1400 (ERROR_INVALID_WINDOW_HANDLE) for a hWnd that is not a live
    // window. Decision 0071 forbade the word altogether (old: no fle_set_last_error in fle_win.c; new: exactly one call, with the 1400 constant).
    // Decision 0122 adds one more place: PostMessageW answers ERROR_NOT_ENOUGH_QUOTA (1816) when the queue holds 10,000 messages (Learn PostMessage). Old: exactly one call (invalid_window()).
    // Decision 0123 (old: exactly 2 calls): the window model sets the last error where a page gives a failure: 32 calls (counted by tools of the wall loop), each with a named constant of fle_win.h or one of the two
    // System Error Codes numbers that have no constant (1406 ERROR_TLW_WITH_WSCHILD, 8 ERROR_NOT_ENOUGH_MEMORY). The set of arguments is pinned; a new place needs a new line here.
    // Decision 0126 (old: 32): two more places, found by comparing the clean tree and this tree function by function: fle_imp_LockWindowUpdate (FLE_ERROR_INVALID_WINDOW_HANDLE_W) and fle_imp_GetCursorInfo (FLE_ERROR_INVALID_PARAMETER_W); the set of arguments does not change: 34
    // Decision 0129 (old: 34): one more place: fle_imp_GetLastActivePopup (an invalid window: FLE_ERROR_INVALID_WINDOW_HANDLE_W, already in the pinned set of arguments below): 35.
    // Decision 0147 (old: 35): fle_imp_PostThreadMessageW sets the last error in two places (FLE_ERROR_INVALID_THREAD_ID: an unknown thread or a thread without a queue; FLE_ERROR_NOT_ENOUGH_QUOTA_W: a full queue, already in the
    // pinned set), so the set of arguments gets FLE_ERROR_INVALID_THREAD_ID: 37.
    // Decision 0267 (old: 37): fle_imp_SetWindowPlacement sets FLE_ERROR_INVALID_PARAMETER_W (already in the pinned set) for a length other than the 44 bytes of WINDOWPLACEMENT: 38.
    assert_eq!(code.matches("fle_set_last_error(").count(), 38);
    let mut args: Vec<String> = Vec::new();
    let mut from = 0;
    while let Some(p) = code[from..].find("fle_set_last_error(") {
        let s = from + p + "fle_set_last_error(".len();
        let e = s + code[s..].find(')').unwrap();
        let a = code[s..e].to_string();
        if !args.contains(&a) {
            args.push(a);
        }
        from = e;
    }
    args.sort();
    assert_eq!(
        args,
        [
            "1406u", "8u", "FLE_ERROR_ACCESS_DENIED_W", "FLE_ERROR_INVALID_PARAMETER_W", "FLE_ERROR_INVALID_THREAD_ID", "FLE_ERROR_INVALID_WINDOW_HANDLE", "FLE_ERROR_INVALID_WINDOW_HANDLE_W",
            "FLE_ERROR_NOT_ENOUGH_QUOTA_W", "FLE_ERROR_WINDOW_OF_OTHER_THREAD", "FLE_W32_ERR_INVALID_HANDLE",
        ]
    );
    assert!(code.contains("fle_set_last_error(FLE_ERROR_INVALID_WINDOW_HANDLE)"));
    assert!(code.contains("fle_set_last_error(FLE_ERROR_NOT_ENOUGH_QUOTA_W)"));
    // `long` is not portable (32 bits on wasm32 and on Windows, 64 on Linux): fixed-width integers only
    let b = code.as_bytes();
    let mut i = 0;
    while let Some(p) = code[i..].find("long") {
        let at = i + p;
        let before = at == 0 || !(b[at - 1].is_ascii_alphanumeric() || b[at - 1] == b'_');
        let after = at + 4 >= b.len() || !(b[at + 4].is_ascii_alphanumeric() || b[at + 4] == b'_');
        assert!(!(before && after), "fle_win.c uses `long` near byte {at}");
        i = at + 4;
    }
    // exactly the three messages, one call site each, in this order, through fle_call_guest; the class table through fle_class_lookup
    // Decision 0119: every window-procedure call goes through call_wndproc (the three sends of CreateWindowExW, SendMessageW, SetWindowTextW), which has the two sites of
    // fle_call_guest (an image function entry directly, and the thunk of MakeObjectInstance after the shape check). Old: 3 call sites of fle_call_guest, in CreateWindowExW.
    // Decision 0123 (old: exactly 2): call_wndproc keeps its 2 sites; the other 4 are the timer procedure of a WM_TIMER message with a callback (Learn SetTimer: TIMERPROC) and the three callbacks of EnumWindows,
    // EnumChildWindows and EnumThreadWindows (WNDENUMPROC, Learn), each called with the handle and lParam
    // Decision 0171 (old: 6): +2, the WH_CALLWNDPROC hook procedure (callwndproc_hooks) and the next procedure of the chain (CallNextHookEx)
    // Decision 0323 (3) (old: 8): +1, the WH_MOUSE / WH_KEYBOARD hook procedure that the input retrieval calls (fle_win_input_call_hooks; Learn MouseProc / KeyboardProc)
    assert_eq!(code.matches("fle_call_guest(").count(), 9);
    let wndproc_sites = code.matches("return fle_call_guest(proc, h, msg, wp, lp)").count() + code.matches("return fle_call_guest(jump, h, msg, wp, lp)").count();
    assert_eq!(wndproc_sites, 2, "call_wndproc");
    let (a, b2, c) = (code.find("FLE_WM_NCCREATE, 0").unwrap(), code.find("FLE_WM_NCCALCSIZE, 0").unwrap(), code.find("FLE_WM_CREATE, 0").unwrap());
    assert!(a < b2 && b2 < c, "WM_NCCREATE, WM_NCCALCSIZE, WM_CREATE in this order");
    assert_eq!(code.matches("fle_class_lookup(").count(), 1);
    assert!(code.contains("fle_host_tid()") && code.contains("fle_stop(") && code.contains("fle_ld(") && code.contains("fle_st("));
    assert!(code.contains("#include \"fle_host.h\"") && code.contains("#include \"fle_callback.h\"") && code.contains("#include \"fle_class.h\""));
    // Decision 0123 (old: no message other than the three was sent: WM_SIZE, WM_MOVE, WM_DESTROY, WM_NCDESTROY, WM_SHOWWINDOW and WM_WINDOWPOS* were forbidden in fle_win.c): the window model sends them in the order of the
    // Learn pages (tests/wall_fr9.rs pins the order). WM_PARENTNOTIFY stays unsent (S-parentnotify: the Learn page of the message is not available, so the choice is recorded in the decision document).
    for sent in ["WM_SIZE", "WM_MOVE", "WM_DESTROY", "WM_NCDESTROY", "WM_SHOWWINDOW", "WM_WINDOWPOS"] {
        assert!(code.contains(sent), "fle_win.c sends {sent} since decision 0123");
    }
    assert!(!code.contains("WM_PARENTNOTIFY"), "S-parentnotify: WM_PARENTNOTIFY is not sent");
    let h = strip_comments(&std::fs::read_to_string(src_dir().join("fle_win.h")).unwrap());
    for bad in ["asm", "__asm__", "<unistd.h>", "<sys/", "<pthread.h>", "<time.h>", "long "] {
        assert!(!h.contains(bad), "fle_win.h must not contain {bad}");
    }
    // fle_class.c: the lookup does not call the window procedure either
    let cc = strip_comments(&std::fs::read_to_string(src_dir().join("fle_class.c")).unwrap());
    // Decision 0123 (old: no fle_set_last_error either): UnregisterClassW (wall 24) sets the two codes of System Error Codes, 1411 ERROR_CLASS_DOES_NOT_EXIST and 1412 ERROR_CLASS_HAS_WINDOWS, in those two places only
    assert!(!cc.contains("fle_call_guest") && !cc.contains("fle_dispatch"));
    assert_eq!(cc.matches("fle_set_last_error(").count(), 2);
    assert!(cc.contains("fle_set_last_error(FLE_ERROR_CLASS_DOES_NOT_EXIST)") && cc.contains("fle_set_last_error(FLE_ERROR_CLASS_HAS_WINDOWS)"));
}

// ---- 9. Generator: the thunks call the runtime ----

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

/// A PE that imports `dll`!`api` (slot 0x2200). Entry 0x1000: `xor ecx,ecx; call 0x1040; ret`; 0x1040 is the thunk.
fn build(dll: &str, api: &str) -> Vec<u8> {
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
    let mut nm = dll.as_bytes().to_vec();
    nm.push(0);
    put(&mut b, 0x2300, &nm);
    let mut hn = vec![0u8, 0u8];
    hn.extend_from_slice(api.as_bytes());
    hn.push(0);
    put(&mut b, 0x2400, &hn);
    for (rva, v) in [(0x2100, 0x2400u64), (0x2200, 0x2400)] {
        w64(&mut b, data_at(rva), v);
    }
    let t = TEXT_RAW;
    let code: &[(u32, Vec<u8>)] = &[
        (0x1000, vec![0x33, 0xc9]),
        (0x1002, [&[0xe8][..], &rel(0x1007, 0x1040)].concat()),
        (0x1007, vec![0xc3]),
        (0x1040, [&[0xff, 0x25][..], &rel(0x1046, 0x2200)].concat()),
    ];
    for (rva, bytes) in code {
        let o = t + (rva - TEXT_RVA) as usize;
        b[o..o + bytes.len()].copy_from_slice(bytes);
    }
    b
}

#[test]
fn the_thunks_call_the_runtime() {
    for api in ["CreateWindowExW", "SetWindowLongPtrW", "DefWindowProcW"] {
        let img = build("user32.dll", api);
        let mut out = Vec::new();
        let st = generate(&img, &mut out).unwrap();
        let c = String::from_utf8(out).unwrap();
        assert_eq!(st.handled[format!("jmp_imp:{api}").as_str()], 1, "{api}");
        assert!(c.contains(&format!("fle_imp_{api}(); return;")), "{api}");
        let va = IB + 0x1040;
        assert!(!c.contains(&format!("L_{va:x}: {{ /* {va:#x} Jmp */ fle_unsupported")), "{api}: still unsupported");
        assert_eq!(st.unsupported_total, 0, "{api}: {:?}", st.unsupported);
    }
}

#[test]
fn the_dll_name_and_the_api_name_are_part_of_the_match() {
    for (dll, api) in [
        ("other.dll", "CreateWindowExW"), ("user32.dll", "CreateWindowExA"), ("user32.dll", "CreateWindowW"), ("other.dll", "SetWindowLongPtrW"),
        ("user32.dll", "SetWindowLongPtrA"), ("user32.dll", "SetWindowLongW"), ("user32.dll", "GetWindowLongW"), ("other.dll", "DefWindowProcW"),
        // decision 0123 (old: ("user32.dll", "DestroyWindow") and ("user32.dll", "CallWindowProcW") were here: both are answered now; the wrong-DLL spelling takes their place)
        ("user32.dll", "DefWindowProcA"), ("other.dll", "DestroyWindow"), ("user32.dll", "SendMessageA"), ("other.dll", "CallWindowProcW"),
    ] {
        let img = build(dll, api);
        let mut out = Vec::new();
        let st = generate(&img, &mut out).unwrap();
        let c = String::from_utf8(out).unwrap();
        for n in ["CreateWindowExW", "SetWindowLongPtrW", "DefWindowProcW"] {
            assert!(!st.handled.contains_key(format!("jmp_imp:{n}").as_str()) && !c.contains(&format!("fle_imp_{n}")), "{dll}!{api}");
        }
        let va = IB + 0x1040;
        assert!(c.contains(&format!("L_{va:x}: {{ /* {va:#x} Jmp */ fle_unsupported(UINT64_C({va:#x})); }}")), "{dll}!{api}");
    }
}

// ---- 10. The bytes of the real DLL that the decision rests on (docs/investigations/window-design.md) ----

fn real_dll() -> Option<Vec<u8>> {
    let p = std::env::var("FLENGINE_DLL").map(PathBuf::from).unwrap_or_else(|_| manifest().join("../../../binaries/win64/FLEngine_x64.dll"));
    std::fs::read(p).ok()
}
fn le32(d: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(d[o..o + 4].try_into().unwrap())
}
fn le16(d: &[u8], o: usize) -> u16 {
    u16::from_le_bytes(d[o..o + 2].try_into().unwrap())
}
fn le64(d: &[u8], o: usize) -> u64 {
    u64::from_le_bytes(d[o..o + 8].try_into().unwrap())
}
fn sections(dll: &[u8]) -> Vec<(u32, u32, u32, u32)> {
    let lf = le32(dll, 0x3c) as usize;
    let nsec = le16(dll, lf + 6) as usize;
    let osz = le16(dll, lf + 20) as usize;
    (0..nsec).map(|i| lf + 24 + osz + 40 * i).map(|o| (le32(dll, o + 8), le32(dll, o + 12), le32(dll, o + 16), le32(dll, o + 20))).collect()
}
fn rva_off(dll: &[u8], rva: u32) -> usize {
    for (vs, sva, rs, rp) in sections(dll) {
        if rva >= sva && rva < sva + vs.max(rs) {
            return rp as usize + (rva - sva) as usize;
        }
    }
    panic!("rva {rva:#x} is in no section");
}
fn bytes_at(dll: &[u8], va: u64, n: usize) -> Vec<u8> {
    let off = rva_off(dll, (va - 0x400000) as u32);
    dll[off..off + n].to_vec()
}
fn hex_at(dll: &[u8], va: u64, n: usize) -> String {
    bytes_at(dll, va, n).iter().map(|x| format!("{x:02x}")).collect()
}
fn imports(dll: &[u8]) -> Vec<(String, String, u64)> {
    let lf = le32(dll, 0x3c) as usize;
    let cstr = |rva: u32| -> String {
        let o = rva_off(dll, rva);
        let e = dll[o..].iter().position(|&c| c == 0).unwrap();
        String::from_utf8_lossy(&dll[o..o + e]).to_string()
    };
    let mut v = Vec::new();
    let mut d = rva_off(dll, le32(dll, lf + 24 + 112 + 8));
    loop {
        let (oft, name, ft) = (le32(dll, d), le32(dll, d + 12), le32(dll, d + 16));
        if name == 0 && ft == 0 {
            break;
        }
        let dname = cstr(name);
        let mut t = rva_off(dll, if oft != 0 { oft } else { ft });
        let mut k = 0u32;
        loop {
            let e = le64(dll, t);
            if e == 0 {
                break;
            }
            let fname = if e >> 63 != 0 { format!("#{}", e & 0xFFFF) } else { cstr(e as u32 + 2) };
            v.push((dname.clone(), fname, 0x400000 + ft as u64 + 8 * k as u64));
            t += 8;
            k += 1;
        }
        d += 20;
    }
    v
}
fn call_sites(dll: &[u8], thunk: u64) -> Vec<u64> {
    let (vs, sva, _, rp) = sections(dll)[0];
    let text = &dll[rp as usize..rp as usize + vs as usize];
    let mut found = Vec::new();
    for i in 0..text.len().saturating_sub(5) {
        if text[i] == 0xe8 {
            let rel = i32::from_le_bytes(text[i + 1..i + 5].try_into().unwrap()) as i64;
            let va = 0x400000 + sva as u64 + i as u64;
            if (va as i64 + 5 + rel) as u64 == thunk {
                found.push(va);
            }
        }
    }
    found
}
fn thunk_slot(dll: &[u8], thunk: u64) -> u64 {
    let t = bytes_at(dll, thunk, 6);
    assert_eq!(&t[..2], [0xff, 0x25], "jmp [rip+disp32] at {thunk:#x}");
    (thunk as i64 + 6 + i32::from_le_bytes(t[2..6].try_into().unwrap()) as i64) as u64
}

#[test]
fn the_three_thunks_jump_to_the_user32_slots_and_the_guest_calls_them_where_the_decision_says() {
    let Some(dll) = real_dll() else { return };
    let imps = imports(&dll);
    for (thunk, name, nsites) in [(0x425780u64, "CreateWindowExW", 9usize), (0x425980, "SetWindowLongPtrW", 43), (0x424920, "DefWindowProcW", 22)] {
        let slot = thunk_slot(&dll, thunk);
        let named: Vec<_> = imps.iter().filter(|(_, _, s)| *s == slot).collect();
        assert_eq!(named.len(), 1, "{name}");
        assert_eq!((named[0].0.to_lowercase(), named[0].1.as_str()), ("user32.dll".to_string(), name));
        assert_eq!(call_sites(&dll, thunk).len(), nsites, "{name}: the number of `call rel32` sites");
    }
    assert_eq!(call_sites(&dll, 0x425780), [0x425805, 0x52a8be, 0x611d09, 0x6327fb, 0x6328ce, 0x71e133, 0x85fb6c, 0x867f7f, 0xa1bb35]);
    // the APIs this decision does not answer are imported by the guest and are the next stops (window-design.md section 8)
    let have: Vec<&str> = imps.iter().filter(|(d, _, _)| d.eq_ignore_ascii_case("user32.dll")).map(|(_, n, _)| n.as_str()).collect();
    for n in ["GetWindowLongPtrW", "DestroyWindow", "SendMessageW", "PostMessageW", "UnregisterClassW", "GetMessageW", "DispatchMessageW", "ShowWindow", "CallWindowProcW"] {
        assert!(have.contains(&n), "{n}");
    }
    // the E8 byte scan (the way tests/class.rs counts); the objdump text of lift/objdump-text.txt stops at 0x11fb4ae and misses the tail
    assert_eq!(call_sites(&dll, 0x425970).len(), 66, "GetWindowLongPtrW");
    assert_eq!(call_sites(&dll, 0x424990).len(), 8, "DestroyWindow");
    assert_eq!(call_sites(&dll, 0x425210).len(), 139, "PostMessageW");
    assert_eq!([0x425380u64, 0x425390, 0x4253b0].iter().map(|t| call_sites(&dll, *t).len()).sum::<usize>(), 216, "SendMessageW");
}

#[test]
fn the_call_at_0x52a8be_is_the_hidden_tool_window_of_size_zero_with_no_parent_and_the_result_goes_to_setwindowlongptr_with_gwl_wndproc() {
    let Some(dll) = real_dll() else { return };
    let h = |va: u64, n: usize| hex_at(&dll, va, n);
    // ecx = 0x80 (WS_EX_TOOLWINDOW); rdx = [0x13be850] (the class name); r8 = L"" at 0x52a8f4; r9d = 0x80000000 (WS_POPUP)
    assert_eq!(h(0x52a85e, 5), "b980000000");
    assert_eq!(h(0x52a863, 7), "488b15e63fe900");
    assert_eq!(h(0x52a86a, 7), "4c8d0583000000");
    assert_eq!(h(0x52a871, 6), "41b900000080");
    assert_eq!(hex_at(&dll, 0x52a8f4, 2), "0000");
    // [rsp+0x20], +0x28, +0x30, +0x38 (x, y, cx, cy): 4-byte stores of 0; +0x40 (parent), +0x48 (menu): 8-byte stores of 0
    assert_eq!(h(0x52a877, 8), "c744242000000000");
    assert_eq!(h(0x52a87f, 8), "c744242800000000");
    assert_eq!(h(0x52a887, 8), "c744243000000000");
    assert_eq!(h(0x52a88f, 8), "c744243800000000");
    assert_eq!(h(0x52a897, 9), "48c744244000000000");
    assert_eq!(h(0x52a8a0, 9), "48c744244800000000");
    // [rsp+0x50] = HInstance ([0x15ec298]); [rsp+0x58] (lpParam) = 0; the call
    assert_eq!(h(0x52a8a9, 7), "488b05e8190c01");
    assert_eq!(h(0x52a8b0, 5), "4889442450");
    assert_eq!(h(0x52a8b5, 9), "48c744245800000000");
    assert_eq!(h(0x52a8be, 1), "e8");
    // after the call: rsi = hwnd; if [rbx] (the TMethod's Code) != 0: rcx = f_52a5b0(rbx); SetWindowLongPtrW(rsi, -4, rax); return rsi
    assert_eq!(h(0x52a8c3, 3), "4889c6");
    assert_eq!(h(0x52a8c6, 4), "48833b00");
    assert_eq!(h(0x52a8ca, 2), "7418");
    assert_eq!(h(0x52a8cc, 3), "4889d9");
    assert_eq!(h(0x52a8cf, 1), "e8");
    assert_eq!(h(0x52a8d4, 3), "4889f1");
    assert_eq!(h(0x52a8d7, 5), "bafcffffff");
    assert_eq!(h(0x52a8dc, 3), "4989c0");
    assert_eq!(h(0x52a8df, 1), "e8");
    assert_eq!(h(0x52a8e4, 3), "4889f0");
    // the call at 0x52a8cf is f_52a5b0 and the one at 0x52a8df is the thunk of SetWindowLongPtrW
    let target = |va: u64| -> u64 {
        let r = bytes_at(&dll, va + 1, 4);
        (va as i64 + 5 + i32::from_le_bytes(r.try_into().unwrap()) as i64) as u64
    };
    assert_eq!(target(0x52a8be), 0x425780);
    assert_eq!(target(0x52a8cf), 0x52a5b0);
    assert_eq!(target(0x52a8df), 0x425980);
    // the class of that call: name at [0x13be850] = L"TPUtilWindow", lpfnWndProc = the DefWindowProcW thunk (0x424920) (tests/class.rs)
    assert_eq!(le64(&bytes_at(&dll, 0x13be850, 8), 0), 0x52a7c8);
}

#[test]
fn the_nine_calls_of_createwindowexw_have_the_shapes_the_decision_lists_and_only_one_is_the_answered_form() {
    let Some(dll) = real_dll() else { return };
    // each of the other forms differs from the answered one in at least one argument; the evidence is the instruction that sets it
    let h = |va: u64, n: usize| hex_at(&dll, va, n);
    // 0xa1bb35 (VTPanningWindow): cx = cy = 0x20 (c7 44 24 30 20 00 00 00 / c7 44 24 38 20 ..)
    assert_eq!(h(0xa1bb02, 8), "c744243020000000");
    assert_eq!(h(0xa1bb0a, 8), "c744243820000000");
    // 0x867f7f (TApplication): dwStyle r9d = 0x84ca0000
    assert_eq!(h(0x867f1c, 6), "41b90000ca84");
    // 0x71e133 (TLightweightLayeredControl): r8d (style of the call's third register slot, here the window name register) = 0x080800ac
    assert_eq!(h(0x71e0d2, 6), "41b8ac000808");
    // 0x85fb6c (MDICLIENT): r9d = 0x56330001, ecx = 0x200
    assert_eq!(h(0x85fb1c, 5), "b900020000");
    assert_eq!(h(0x85fb2b, 6), "41b901003356");
    // 0x6d8468 and 0xd70738 reach the thunk at 0x425790 with hWndParent = -3 (HWND_MESSAGE): mov QWORD PTR [rsp+0x38], 0xfffffffffffffffd
    assert_eq!(h(0x6d8441, 9), "48c7442438fdffffff");
    assert_eq!(h(0xd70711, 9), "48c7442438fdffffff");
}

#[test]
fn the_guests_window_procedures_of_the_classes_the_decision_names() {
    let Some(dll) = real_dll() else { return };
    let h = |va: u64, n: usize| hex_at(&dll, va, n);
    // 0x71dd10 (TLightweightLayeredControl): `cmp edx,0x81; jne`; ...; call SetWindowLongPtrW thunk with edx = -4; `mov eax,1`; else jmp to DefWindowProcW
    assert_eq!(h(0x71dd14, 6), "81fa81000000");
    assert_eq!(h(0x71dd2a, 5), "bafcffffff");
    assert_eq!(h(0x71dd37, 5), "b801000000");
    assert_eq!(h(0x71dd3e, 1), "e8");
    // 0x6d8330 (TTimerWindow): `cmp rdx,0x8ffc` and, otherwise, call DefWindowProcW (0x424920)
    assert_eq!(h(0x6d8336, 7), "4881fafc8f0000");
    // 0xd70600 (TILRemoteWindow): `cmp rdx,0x8ffd`
    assert_eq!(h(0xd70609, 7), "4881fafd8f0000");
    // MakeObjectInstance (0x52a5b0): VirtualAlloc(NULL, 0x1000, 0x1000, 0x40): xor ecx,ecx; mov edx,0x1000; mov r8d,0x1000; mov r9d,0x40
    assert_eq!(h(0x52a5ca, 2), "33c9");
    assert_eq!(h(0x52a5cc, 5), "ba00100000");
    assert_eq!(h(0x52a5d1, 6), "41b800100000");
    assert_eq!(h(0x52a5d7, 6), "41b940000000");
    // and the 8 bytes `pop r11; jmp [rip+0]` that it copies into the block (0x13be804)
    assert_eq!(h(0x13be804, 8), "415bff2500000000");
}

