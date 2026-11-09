//! Tests of decision 0196 (docs/decisions/0196-handle-table-reuse.md, PROVISIONAL): tables of the runtime that counted what had ever been created, so that a long session stopped them (a limit that real
//! Windows does not have): SetWindowsHookExW / UnhookWindowsHookEx (the table counted every hook ever installed; menus install and remove two each), ... The guest is a C program linked with EVERY runtime
//! file (as tests/wall_fr11.rs does) that calls the `fle_imp_*` functions with the x64 argument registers. Expected values are written here from the Microsoft Learn pages named in the comments of
//! runtime/fle_win.c and from the choices S-tbl1-... of the decision, not read back from the C code. Nothing is compared with a real Windows.
//! FLE_TBL1_RT (a copy of lift/runtime) replaces runtime/ for the mutation runs.

mod etxt_common;

use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn rt_dir() -> PathBuf {
    if let Ok(d) = std::env::var("FLE_TBL1_RT") {
        return PathBuf::from(d);
    }
    manifest().join("../../runtime")
}
fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-tbl1/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

fn runtime_sources() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(rt_dir())
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            let n = p.file_name().unwrap().to_string_lossy().to_string();
            n.starts_with("fle_") && n.ends_with(".c")
        })
        .collect();
    v.sort();
    v
}

fn clang_c(src: &Path, obj: &Path, strict: bool) {
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-Wall", "-Wextra", "-fPIE", "-c"]);
    if strict {
        cmd.arg("-Werror");
    }
    let cc = cmd.arg("-I").arg(rt_dir()).arg(src).arg("-o").arg(obj).env("TMPDIR", work_dir().join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {}: {}", src.display(), String::from_utf8_lossy(&cc.stderr));
}

/// Every runtime file, compiled once per process; the files of decision 0171 with -Werror.
fn objs() -> &'static Vec<PathBuf> {
    static O: OnceLock<Vec<PathBuf>> = OnceLock::new();
    O.get_or_init(|| {
        let dir = work_dir().join(etxt_common::per_process("obj-tbl1"));
        std::fs::create_dir_all(&dir).unwrap();
        let ours = ["fle_win.c", "fle_thread.c"];
        let mut v = Vec::new();
        for f in runtime_sources() {
            let name = f.file_name().unwrap().to_string_lossy().to_string();
            let o = dir.join(name.replace(".c", ".o"));
            clang_c(&f, &o, ours.contains(&name.as_str()));
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
#include "fle_runtime.h"
#include "fle_sse.h"
#include "fle_dispatch.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_callback.h"
#include "fle_class.h"
#include "fle_win.h"
#include "fle_thread.h"
#include "fle_timerq.h"
#include "fle_event.h"
#include "fle_sys.h"
#include "fle_vfs.h"
#include "fle_input.h"
#include "fle_x87ctl.h"
#include "fle_mxcsr.h"

uint64_t R[16];
fle_eflags FL;
fle_xmm X[16];
static uint64_t stk[0x4000] __attribute__((aligned(4096)));
#define E0 ((uint64_t)(uintptr_t)stk + 0x10000 + 8)
#define RET0 UINT64_C(0xdead0000)
#define U(x) ((unsigned long long)(x))
#define A(x) ((uint64_t)(uintptr_t)(x))

/* callv(f, n, args): the first four arguments in rcx, rdx, r8, r9, the fifth and later at [rsp+0x28] ... of the callee's entry (rsp points at the return address) */
static uint64_t callv(void (*f)(void), int n, const uint64_t *a)
{
    uint64_t sp0 = R[4];
    int i;
    R[4] = (R[4] - 0x100) & ~UINT64_C(15);
    R[4] -= 8;
    fle_st(R[4], 8, 0xAAAA0000);
    for (i = 4; i < n; i++) fle_st(R[4] + 0x28 + 8u * (unsigned)(i - 4), 8, a[i]);
    R[1] = a[0]; R[2] = a[1]; R[8] = a[2]; R[9] = a[3];
    R[0] = 0xBADBADBADull;
    f();
    R[4] = sp0;
    return R[0];
}
#define K(f, ...) callv((void (*)(void))(f), 4, (uint64_t[16]){ __VA_ARGS__ })
static uint32_t LE(void) { fle_imp_GetLastError(); return (uint32_t)R[0]; }
static void SETLE(uint32_t v) { fle_set_last_error(v); }
static void boot(void) { R[4] = E0; fle_st(E0, 8, RET0); fle_teb_set_stack(A(stk), A(stk) + sizeof stk); }

static uint16_t W16[16][260];
static unsigned W16k;
static uint64_t S(const char *s)
{
    uint16_t *p = W16[W16k++ % 16]; size_t i, n = strlen(s);
    for (i = 0; i < n; i++) p[i] = (uint8_t)s[i];
    p[n] = 0;
    return A(p);
}

#define VA_WP 0x2000
static void wndproc(void)
{
    uint64_t h = R[1], msg = R[2], wp = R[8], lp = R[9], sp = R[4];
    if (msg == 0x81) { R[0] = 1; return; }
    if (msg == 0x83) { R[0] = 0; return; }
    if (msg == 0x01) { R[0] = 0; return; }
    R[1] = h; R[2] = msg; R[8] = wp; R[9] = lp;
    R[4] = sp - 8; fle_st(R[4], 8, 0xAAAA0001);
    fle_imp_DefWindowProcW();
    R[4] = sp;
}
static int cls_done;
static uint16_t cls_name[] = { 'T','I','A','1','W', 0 };
static void regclass(void)
{
    static uint8_t wc[72];
    uint64_t w = VA_WP, inst = 0x400000, nm = A(cls_name);
    if (cls_done) return;
    memset(wc, 0, sizeof wc); memcpy(wc + 8, &w, 8); memcpy(wc + 24, &inst, 8); memcpy(wc + 64, &nm, 8);
    K(fle_imp_RegisterClassW, A(wc));
    cls_done = 1;
}
/* CreateWindowExW(ex, class, title, style, x, y, w, h, parent, menu, hInstance 0x400000, param) */
static uint64_t cw(uint64_t style, int32_t x, int32_t y, int32_t w, int32_t hh, uint64_t parent)
{
    uint64_t sp0 = R[4], h;
    regclass();
    R[4] = (R[4] - 0x80) & ~UINT64_C(15); R[4] -= 8; fle_st(R[4], 8, 0xAAAA0000);
    fle_st(R[4] + 0x28, 8, (uint32_t)x); fle_st(R[4] + 0x30, 8, (uint32_t)y); fle_st(R[4] + 0x38, 8, (uint32_t)w); fle_st(R[4] + 0x40, 8, (uint32_t)hh);
    fle_st(R[4] + 0x48, 8, parent); fle_st(R[4] + 0x50, 8, 0); fle_st(R[4] + 0x58, 8, 0x400000); fle_st(R[4] + 0x60, 8, 0);
    R[1] = 0; R[2] = A(cls_name); R[8] = S("w"); R[9] = style;
    fle_imp_CreateWindowExW();
    h = R[0];
    R[4] = sp0;
    return h;
}
#define POPUP 0x80000000u
#define VISIBLE 0x10000000u
"##;

fn run_src(name: &str, body: &str) -> Out {
    run_top(name, "", "", 0, body)
}

/// `top` is C at file scope (guest function stubs), `entries` more rows of the dispatch table (VA > 0x2000, ascending), `n_extra` their number.
fn run_top(name: &str, top: &str, entries: &str, n_extra: usize, body: &str) -> Out {
    let dir = work_dir();
    let uname = etxt_common::uniq(name);
    let c = dir.join(format!("{uname}.c"));
    let exe = dir.join(&uname);
    let src = format!("{PRELUDE}\n{top}\nconst fle_fn_entry fle_dispatch_table[] = {{ {{ VA_WP, wndproc }}{entries} }};\nconst uint64_t fle_dispatch_count = {};\nint main(void)\n{{\n    boot();\n{body}\n    return 0;\n}}\n", 1 + n_extra);
    std::fs::write(&c, src).unwrap();
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fPIE", "-pthread", "-I"]).arg(rt_dir());
    for o in objs() {
        cmd.arg(o);
    }
    let cc = cmd.arg("-o").arg(&exe).arg(&c).arg("-lm").env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new(&exe).env("TMPDIR", dir.join("tmp")).output().expect("run failed");
    Out { stdout: String::from_utf8_lossy(&r.stdout).to_string(), stderr: String::from_utf8_lossy(&r.stderr).to_string(), signal: r.status.signal(), ok: r.status.success() }
}

fn lines(name: &str, body: &str) -> Vec<String> {
    let o = run_src(name, body);
    assert!(o.ok, "{name} did not exit normally (signal {:?}): {}\n{}", o.signal, o.stderr, o.stdout);
    o.stdout.lines().map(|s| s.to_string()).collect()
}

/// The run must stop with SIGABRT and a "fle: <api>: " message that contains `word`.
fn aborts(name: &str, body: &str, api: &str, word: &str) {
    let o = run_src(name, body);
    assert!(!o.ok, "{name}: expected an abort, got stdout {}", o.stdout);
    assert_eq!(o.signal, Some(6), "{name}: not SIGABRT: {}", o.stderr);
    assert!(o.stderr.contains(&format!("fle: {api}: ")), "{name}: no 'fle: {api}' message: {}", o.stderr);
    assert!(o.stderr.contains(word), "{name}: message lacks {word:?}: {}", o.stderr);
}

// ---- SetWindowsHookExW / UnhookWindowsHookEx (decision 0196, part 1) ----
// Learn UnhookWindowsHookEx: "Removes a hook procedure installed in a hook chain"; nonzero on success, zero on failure. The runtime table held at most 64 hooks EVER INSTALLED (hook_count was a total): the
// engine's pop-up windows install a WH_CALLWNDPROC hook when a menu opens and remove it when the menu closes, so the 65th install stopped the run after about 32 menus. Now at most 64 hooks may be ALIVE.
// The handle of the n-th install is 0x7FFA0000 + 4 * n (unchanged: wall_fr11.rs), a removed hook's handle is refused (returns 0) until the values are issued again after 65,536 installs (S-tbl1-hook-span).

const HOOK_PROCS: &str = r#"
#define VA_HA 0x2100
#define VA_HB 0x2110
#define VA_HC 0x2120
#define VA_HD 0x2130
static char order[16];
static unsigned norder;
static void call_next(void)
{
    uint64_t n = R[1], wp = R[2], lp = R[8], sp = R[4];
    R[1] = 0; R[2] = n; R[8] = wp; R[9] = lp;
    R[4] = sp - 8; fle_st(R[4], 8, 0xAAAA0002);
    fle_imp_CallNextHookEx();
    R[4] = sp;
}
static void hook_a(void) { order[norder++] = 'A'; call_next(); R[0] = 0; }
static void hook_b(void) { order[norder++] = 'B'; call_next(); R[0] = 0; }
static void hook_c(void) { order[norder++] = 'C'; call_next(); R[0] = 0; }
static void hook_d(void) { order[norder++] = 'D'; call_next(); R[0] = 0; }
"#;
const HOOK_ENTRIES: &str = ", { VA_HA, hook_a }, { VA_HB, hook_b }, { VA_HC, hook_c }, { VA_HD, hook_d }";

#[test]
fn a_removed_hook_gives_its_place_back_so_that_a_hundred_installs_one_after_the_other_all_succeed() {
    let l = lines(
        "tbl1-hook-100",
        r#"
    unsigned i, ok = 1, gone_refused = 1, handles_ok = 1;
    uint64_t first = 0, h, prev = 0;
    for (i = 1; i <= 300; i++) {
        h = K(fle_imp_SetWindowsHookExW, 4, 0x809740, 0, 0x1001);
        if (h == 0) ok = 0;
        if (h != 0x7FFA0000ull + 4ull * i) handles_ok = 0;
        if (i == 1) first = h;
        if (K(fle_imp_UnhookWindowsHookEx, h) != 1) ok = 0;
        if (K(fle_imp_UnhookWindowsHookEx, h) != 0) gone_refused = 0;
        if (i > 1 && K(fle_imp_UnhookWindowsHookEx, first) != 0) gone_refused = 0;
        prev = h;
    }
    printf("300 installs and removals ok %u handles 4*n %u removed handles refused %u live %u last %llx\n", ok, handles_ok, gone_refused, fle_win_hook_live(), U(prev));
"#,
    );
    assert_eq!(l, ["300 installs and removals ok 1 handles 4*n 1 removed handles refused 1 live 0 last 7ffa04b0"], "far more than 64 hooks installed over the run, never more than one alive");
}

#[test]
fn sixty_four_live_hooks_are_allowed_and_only_the_sixty_fifth_alive_at_once_stops() {
    let l = lines(
        "tbl1-hook-64",
        r#"
    unsigned i;
    uint64_t h[64], again;
    for (i = 0; i < 64; i++) h[i] = K(fle_imp_SetWindowsHookExW, 7, 0x809740, 0, 0x1001);
    printf("64 alive: live %u first %llx last %llx\n", fle_win_hook_live(), U(h[0]), U(h[63]));
    printf("remove one: %llu live %u\n", U(K(fle_imp_UnhookWindowsHookEx, h[10])), fle_win_hook_live());
    again = K(fle_imp_SetWindowsHookExW, 7, 0x809740, 0, 0x1001);
    printf("install again: handle %llx live %u (a value that is not one of the 64 alive)\n", U(again), fle_win_hook_live());
    for (i = 0; i < 64; i++) if (i != 10 && h[i] == again) printf("DUPLICATE HANDLE\n");
"#,
    );
    assert_eq!(l, ["64 alive: live 64 first 7ffa0004 last 7ffa0100", "remove one: 1 live 63", "install again: handle 7ffa0104 live 64 (a value that is not one of the 64 alive)"]);
    aborts(
        "tbl1-hook-65",
        "    unsigned i;\n    for (i = 0; i < 65; i++) K(fle_imp_SetWindowsHookExW, 7, 0x809740, 0, 0x1001);\n",
        "SetWindowsHookExW",
        "more than 64 hooks are alive",
    );
}

#[test]
fn the_handle_of_a_removed_hook_is_refused_until_the_values_come_round_and_a_value_held_by_a_live_hook_is_never_issued_again() {
    // S-tbl1-hook-span: the handle values run 0x7FFA0004 .. 0x7FFA0004 + 4 * 65535 and then start again at 0x7FFA0004; a value that a live hook holds is skipped.
    let l = lines(
        "tbl1-hook-span",
        r#"
    unsigned i, dup = 0, wrapped = 0;
    uint64_t keep = K(fle_imp_SetWindowsHookExW, 4, 0x809740, 0, 0x1001), h, old, second_value = 0;
    old = K(fle_imp_SetWindowsHookExW, 4, 0x809740, 0, 0x1001);
    K(fle_imp_UnhookWindowsHookEx, old);
    printf("keep %llx old %llx stale unhook %llu\n", U(keep), U(old), U(K(fle_imp_UnhookWindowsHookEx, old)));
    for (i = 0; i < 65600; i++) {
        h = K(fle_imp_SetWindowsHookExW, 4, 0x809740, 0, 0x1001);
        if (h == keep) dup++;
        if (h == old) wrapped++;
        K(fle_imp_UnhookWindowsHookEx, h);
    }
    printf("duplicates of the live hook's handle %u, the removed handle issued again %u\n", dup, wrapped);
    printf("live hook still removable %llu, then again %llu live %u\n", U(K(fle_imp_UnhookWindowsHookEx, keep)), U(K(fle_imp_UnhookWindowsHookEx, keep)), fle_win_hook_live());
"#,
    );
    assert_eq!(
        l,
        [
            "keep 7ffa0004 old 7ffa0008 stale unhook 0",
            "duplicates of the live hook's handle 0, the removed handle issued again 1",
            "live hook still removable 1, then again 0 live 0",
        ]
    );
}

#[test]
fn the_wh_callwndproc_chain_is_newest_first_by_install_order_not_by_the_place_in_the_table() {
    // Learn SetWindowsHookEx: "the hook procedure is added to the start of the hook chain" for the thread: newest first (decision 0171). A place freed by an earlier removal is taken by a later install; the order
    // of the chain is the order of the installs: C, then B, then A even though C sits in a place before the others.
    let o = run_top(
        "tbl1-hook-order",
        HOOK_PROCS,
        HOOK_ENTRIES,
        4,
        r#"
    unsigned i;
    uint64_t w = cw(POPUP | VISIBLE, 0, 0, 100, 100, 0), ha, hb, hc, hd, fill[61];
    ha = K(fle_imp_SetWindowsHookExW, 4, VA_HA, 0, 0x1001);
    hb = K(fle_imp_SetWindowsHookExW, 4, VA_HB, 0, 0x1001);
    hc = K(fle_imp_SetWindowsHookExW, 4, VA_HC, 0, 0x1001);
    for (i = 0; i < 61; i++) fill[i] = K(fle_imp_SetWindowsHookExW, 7, 0x809740, 0, 0x1001);
    printf("64 alive %u\n", fle_win_hook_live());
    K(fle_imp_SendMessageW, w, 0x405, 0, 0); order[norder] = 0; printf("A B C installed in this order: %s\n", order); norder = 0;
    K(fle_imp_UnhookWindowsHookEx, hb);
    hd = K(fle_imp_SetWindowsHookExW, 4, VA_HD, 0, 0x1001);
    printf("B removed, D installed (takes the place B left): live %u\n", fle_win_hook_live());
    K(fle_imp_SendMessageW, w, 0x405, 0, 0); order[norder] = 0; printf("chain %s\n", order); norder = 0;
    K(fle_imp_UnhookWindowsHookEx, hd);
    K(fle_imp_SendMessageW, w, 0x405, 0, 0); order[norder] = 0; printf("D removed: chain %s\n", order); norder = 0;
"#,
    );
    assert!(o.ok, "{:?} {}\n{}", o.signal, o.stderr, o.stdout);
    let l: Vec<&str> = o.stdout.lines().collect();
    assert_eq!(l, ["64 alive 64", "A B C installed in this order: CBA", "B removed, D installed (takes the place B left): live 64", "chain DCA", "D removed: chain CA"]);
}

// ---- CreateThread and the timer-queue threads (decision 0196, part 2) ----
// Learn CreateThread: the thread object stays until the thread has ended AND every handle to it is closed (CloseHandle: "the thread object ... removed when the last handle is closed and the thread has terminated").
// The table of the runtime held 256 threads EVER CREATED: every timer-queue timer that fires (the engine's FILE menu starts one each time it opens) uses one place for good, so after about 240 menus the
// timers could not be made ("no thread could be made for the timer"), and a guest CreateThread returned NULL with ERROR_NOT_ENOUGH_MEMORY. Now, once the table is full, the place of the thread that ended
// longest ago is used again if nobody can name it: its handle was closed, or it is a timer-queue thread (whose handle the guest never sees). A closed handle stays refused (S-tbl1-thread-handle).

const THREAD_TOP: &str = r#"
#define VA_T 0x2100
#define VA_CB 0x2110
static unsigned ran, cb_ran;
static uint64_t last_tid;
static void thread_fn(void) { ran++; fle_imp_GetCurrentThreadId(); last_tid = R[0]; R[0] = 7; }
static void cb_fn(void) { cb_ran++; R[0] = 0; }
static uint64_t K6(void (*f)(void), uint64_t a, uint64_t b, uint64_t c, uint64_t d, uint64_t e, uint64_t g) { uint64_t v[6] = { a, b, c, d, e, g }; return callv(f, 6, v); }
static uint64_t mk(unsigned flags) { return K6(fle_imp_CreateThread, 0, 0, VA_T, 0, flags, 0); }
static uint64_t waitinf(uint64_t h) { return (uint32_t)K(fle_imp_WaitForSingleObject, h, 0xFFFFFFFFull); }
"#;
const THREAD_ENTRIES: &str = ", { VA_T, thread_fn }, { VA_CB, cb_fn }";

#[test]
fn six_hundred_threads_made_run_to_the_end_and_closed_one_after_another_all_get_a_place() {
    let o = run_top(
        "tbl1-threads-600",
        THREAD_TOP,
        THREAD_ENTRIES,
        2,
        r#"
    unsigned i, failed = 0, bad_wait = 0, same_order = 1;
    uint64_t h, first = 0, last = 0, prev = 0;
    for (i = 1; i <= 600; i++) {
        h = mk(0);
        if (h == 0) { failed++; continue; }
        if (i == 1) first = h;
        if (i <= 255 && h != 0x7FFAA0000000ull + 4ull * (i + 1)) same_order = 0;
        if (waitinf(h) != 0) bad_wait++;
        if (K(fle_imp_CloseHandle, h) != 1) bad_wait++;
        last = h; prev = h;
    }
    printf("600 threads: failed %u bad wait/close %u ran %u first %llx last %llx\n", failed, bad_wait, ran, U(first), U(last));
    printf("the handles of the first 255 are the old ones %u\n", same_order);
    SETLE(0);
    printf("a closed handle: wait %llx close %llu err %u\n", U(waitinf(first)), U(K(fle_imp_CloseHandle, first)), LE());
    SETLE(0);
    printf("the last closed handle: wait %llx close %llu err %u\n", U(waitinf(prev)), U(K(fle_imp_CloseHandle, prev)), LE());
"#,
    );
    assert!(o.ok, "{:?} {}\n{}", o.signal, o.stderr, o.stdout);
    let l: Vec<&str> = o.stdout.lines().collect();
    assert_eq!(l[0], "600 threads: failed 0 bad wait/close 0 ran 600 first 7ffaa0000008 last 7ffaa0000964");
    assert_eq!(l[1], "the handles of the first 255 are the old ones 1");
    assert_eq!(l[2], "a closed handle: wait ffffffff close 0 err 6");
    assert_eq!(l[3], "the last closed handle: wait ffffffff close 0 err 6");
}

#[test]
fn a_full_table_of_threads_that_are_alive_or_whose_handles_are_open_still_refuses_the_next_thread() {
    let o = run_top(
        "tbl1-threads-full",
        THREAD_TOP,
        THREAD_ENTRIES,
        2,
        r#"
    unsigned i, made = 0;
    uint64_t h[300], extra;
    for (i = 0; i < 300; i++) { h[i] = mk(0); if (h[i]) made++; }
    printf("made %u of 300 (the first thread is the table's place 0)\n", made);
    SETLE(0);
    printf("one more: %llu err %u\n", U(mk(0)), LE());
    for (i = 0; i < 255; i++) waitinf(h[i]);
    SETLE(0);
    printf("all 255 ended, none closed: %llu err %u\n", U(mk(0)), LE());
    K(fle_imp_CloseHandle, h[100]);
    extra = mk(0);
    printf("one closed: new thread %s, it differs from the closed handle %d, new wait %llx\n", extra ? "made" : "NOT made", extra != h[100], U(waitinf(extra)));
    printf("the closed handle is refused: wait %llx\n", U(waitinf(h[100])));
    printf("an open one still answers: wait %llx exit code ran %u\n", U(waitinf(h[7])), ran);
"#,
    );
    assert!(o.ok, "{:?} {}\n{}", o.signal, o.stderr, o.stdout);
    let l: Vec<&str> = o.stdout.lines().collect();
    assert_eq!(l[0], "made 255 of 300 (the first thread is the table's place 0)");
    assert_eq!(l[1], "one more: 0 err 8");
    assert_eq!(l[2], "all 255 ended, none closed: 0 err 8");
    assert_eq!(l[3], "one closed: new thread made, it differs from the closed handle 1, new wait 0");
    assert_eq!(l[4], "the closed handle is refused: wait ffffffff");
    assert!(l[5].starts_with("an open one still answers: wait 0 "), "{}", l[5]);
}

#[test]
fn timer_queue_timers_that_fire_do_not_use_up_the_thread_table_six_hundred_one_shot_timers_all_run() {
    // Learn CreateTimerQueueTimer / DeleteTimerQueueTimer: a timer fires once for Period 0 and its handle may be deleted afterwards. The runtime runs each timer on a thread of its own (decision 0171).
    let o = run_top(
        "tbl1-timers-600",
        THREAD_TOP,
        THREAD_ENTRIES,
        2,
        r#"
    unsigned i, failed = 0;
    static uint64_t ph;
    for (i = 0; i < 600; i++) {
        uint64_t v[7] = { A(&ph), 0, VA_CB, 0, 5, 0, 0 };
        if (callv(fle_imp_CreateTimerQueueTimer, 7, v) != 1) failed++;
        K(fle_imp_Sleep, 20);
        if (K(fle_imp_DeleteTimerQueueTimer, 0, ph, 0) != 1) failed++;
    }
    printf("600 one-shot timers: failed %u callbacks %u threads %u\n", failed, cb_ran, fle_thread_count());
"#,
    );
    assert!(o.ok, "{:?} {}\n{}", o.signal, o.stderr, o.stdout);
    let l: Vec<&str> = o.stdout.lines().collect();
    assert_eq!(l[0], "600 one-shot timers: failed 0 callbacks 600 threads 256");
}

#[test]
fn the_place_of_the_thread_that_finished_longest_ago_is_used_first() {
    // S-tbl1-thread-order: first in, first out (a thread id is FLE_EMULATED_TID 0x1001 + the table place, decision 0056). Threads 10 and 200 (places 10 and 200) are closed after all ran; thread 10 finished first.
    let o = run_top(
        "tbl1-threads-order",
        THREAD_TOP,
        THREAD_ENTRIES,
        2,
        r#"
    unsigned i;
    uint64_t h[300], n1, n2;
    for (i = 0; i < 255; i++) h[i] = mk(0);
    for (i = 0; i < 255; i++) waitinf(h[i]);
    K(fle_imp_CloseHandle, h[199]);   /* place 200 */
    K(fle_imp_CloseHandle, h[9]);     /* place 10: closed later, finished earlier */
    n1 = mk(0); waitinf(n1);
    printf("first new thread runs with id %llx (place 10 = 0x%x)\n", U(last_tid), 0x1001 + 10);
    n2 = mk(0); waitinf(n2);
    printf("second new thread runs with id %llx (place 200 = 0x%x)\n", U(last_tid), 0x1001 + 200);
"#,
    );
    assert!(o.ok, "{:?} {}\n{}", o.signal, o.stderr, o.stdout);
    let l: Vec<&str> = o.stdout.lines().collect();
    assert_eq!(l, ["first new thread runs with id 100b (place 10 = 0x100b)", "second new thread runs with id 10c9 (place 200 = 0x10c9)"]);
}
