//! Tests of decision 0171 (docs/decisions/0171-user-operation-walls.md, PROVISIONAL), the parts that need the window model: the system caret, ShowOwnedPopups, LogicalToPhysicalPoint /
//! PhysicalToLogicalPoint, SetKeyboardState, and the messages that DefWindowProcW answers with 0 (WM_CHANGEUISTATE, WM_UPDATEUISTATE, EM_CANUNDO). The guest is a C program linked with EVERY runtime
//! file (as tests/wall_fr11.rs does) that calls the `fle_imp_*` functions with the x64 argument registers. Expected values are written here from the Microsoft Learn pages named in the comments of
//! runtime/fle_win.c and fle_input.c, and from the choices S-ia1-... of the decision, not read back from the C code. Nothing is compared with a real Windows.
//! FLE_IA1W_RT (a copy of lift/runtime) replaces runtime/ for the mutation runs.

mod etxt_common;

use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn rt_dir() -> PathBuf {
    if let Ok(d) = std::env::var("FLE_IA1W_RT") {
        return PathBuf::from(d);
    }
    manifest().join("../../runtime")
}
fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-ia1/test");
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
        let dir = work_dir().join(etxt_common::per_process("obj-ia1w"));
        std::fs::create_dir_all(&dir).unwrap();
        let ours = ["fle_win.c", "fle_input.c", "fle_thread.c", "fle_seh.c", "fle_locale.c", "fle_clip.c", "fle_timerq.c", "fle_shell.c", "fle_dwm.c"];
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

#[test]
fn the_system_caret_is_created_hidden_and_hiding_is_cumulative() {
    // Learn CreateCaret: "The caret is initially hidden"; HideCaret: "Hiding is cumulative. If HideCaret has been called five times in a row, ShowCaret must be called five times to show the caret";
    // DestroyCaret: nonzero when the thread owns a caret; SetCaretPos: nonzero on success. The caret is state only (S-ia1-caret-1); a size of 0 is the border width, 1 pixel (S-ia1-caret-2).
    let l = lines(
        "caret",
        r#"
    int32_t x = -1, y = -1, hide = -1;
    uint64_t w1 = cw(POPUP | VISIBLE, 0, 0, 100, 100, 0), w2 = cw(POPUP | VISIBLE, 200, 0, 100, 100, 0);
    printf("none %d\n", fle_win_caret_state(&x, &y, &hide));
    printf("show_without %llu destroy_without %llu setpos_without %llu\n", U(K(fle_imp_ShowCaret, 0)), U(K(fle_imp_DestroyCaret)), U(K(fle_imp_SetCaretPos, 1, 2)));
    printf("create %llu\n", U(K(fle_imp_CreateCaret, w1, 0, 2, 16)));
    printf("state %d hide %d\n", fle_win_caret_state(&x, &y, &hide), hide);
    printf("setpos %llu\n", U(K(fle_imp_SetCaretPos, 5, 6)));
    fle_win_caret_state(&x, &y, &hide);
    printf("pos %d %d\n", x, y);
    printf("show %llu hide %d\n", U(K(fle_imp_ShowCaret, w1)), (fle_win_caret_state(&x, &y, &hide), hide));
    printf("hide %llu hide %llu hide_count %d\n", U(K(fle_imp_HideCaret, w1)), U(K(fle_imp_HideCaret, 0)), (fle_win_caret_state(&x, &y, &hide), hide));
    printf("show %llu hide_count %d\n", U(K(fle_imp_ShowCaret, w1)), (fle_win_caret_state(&x, &y, &hide), hide));
    printf("other_window_show %llu other_window_hide %llu\n", U(K(fle_imp_ShowCaret, w2)), U(K(fle_imp_HideCaret, w2)));
    printf("create_again_on_w2 %llu hide %d\n", U(K(fle_imp_CreateCaret, w2, 0, 0, 0)), (fle_win_caret_state(&x, &y, &hide), hide));
    printf("old_owner_show %llu\n", U(K(fle_imp_ShowCaret, w1)));
    printf("destroy %llu state %d destroy_again %llu\n", U(K(fle_imp_DestroyCaret)), fle_win_caret_state(&x, &y, &hide), U(K(fle_imp_DestroyCaret)));
    SETLE(0);
    printf("create_bad_window %llu err %u\n", U(K(fle_imp_CreateCaret, 0x1234, 0, 1, 1)), LE());
"#,
    );
    assert_eq!(
        l,
        [
            "none 0",
            "show_without 0 destroy_without 0 setpos_without 0",
            "create 1",
            "state 1 hide 1",
            "setpos 1",
            "pos 5 6",
            "show 1 hide 0",
            "hide 1 hide 1 hide_count 2",
            "show 1 hide_count 1",
            "other_window_show 0 other_window_hide 0",
            "create_again_on_w2 1 hide 1",
            "old_owner_show 0",
            "destroy 1 state 0 destroy_again 0",
            "create_bad_window 0 err 1400",
        ]
    );
}

#[test]
fn show_owned_popups_hides_and_restores_only_the_popups_it_hid() {
    // Learn ShowOwnedPopups: FALSE hides all visible pop-up windows owned by the window, TRUE shows the hidden ones; Remarks: "ShowOwnedPopups shows only windows hidden by a previous call to ShowOwnedPopups.
    // For example, if a pop-up window is hidden by using the ShowWindow function, subsequently calling ShowOwnedPopups with the fShow parameter set to TRUE does not cause the window to be shown."
    let l = lines(
        "sop",
        r#"
    uint64_t owner = cw(POPUP | VISIBLE, 0, 0, 100, 100, 0);
    uint64_t p1 = cw(POPUP | VISIBLE, 10, 10, 50, 50, owner), p2 = cw(POPUP | VISIBLE, 20, 20, 50, 50, owner);
    printf("visible %llu %llu\n", U(K(fle_imp_IsWindowVisible, p1)), U(K(fle_imp_IsWindowVisible, p2)));
    K(fle_imp_ShowWindow, p2, 0);
    printf("p2 hidden by ShowWindow: %llu\n", U(K(fle_imp_IsWindowVisible, p2)));
    printf("hide %llu\n", U(K(fle_imp_ShowOwnedPopups, owner, 0)));
    printf("after hide %llu %llu owner %llu\n", U(K(fle_imp_IsWindowVisible, p1)), U(K(fle_imp_IsWindowVisible, p2)), U(K(fle_imp_IsWindowVisible, owner)));
    printf("show %llu\n", U(K(fle_imp_ShowOwnedPopups, owner, 1)));
    printf("after show %llu %llu\n", U(K(fle_imp_IsWindowVisible, p1)), U(K(fle_imp_IsWindowVisible, p2)));
    SETLE(0);
    printf("bad_window %llu err %u\n", U(K(fle_imp_ShowOwnedPopups, 0x1234, 0)), LE());
"#,
    );
    assert_eq!(l, ["visible 1 1", "p2 hidden by ShowWindow: 0", "hide 1", "after hide 0 0 owner 1", "show 1", "after show 1 0", "bad_window 0 err 1400"]);
}

#[test]
fn logical_and_physical_points_are_the_same_on_the_one_96_dpi_monitor() {
    // Learn LogicalToPhysicalPoint / PhysicalToLogicalPoint: nonzero on success, the new coordinates are copied into the POINT; the process has no DPI scaling here (S-ia1-l2p-1): the point is unchanged.
    let l = lines(
        "l2p",
        r#"
    int32_t pt[2] = { 123, -45 };
    uint64_t w = cw(POPUP | VISIBLE, 0, 0, 100, 100, 0);
    printf("l2p %llu %d %d\n", U(K(fle_imp_LogicalToPhysicalPoint, w, A(pt))), pt[0], pt[1]);
    printf("p2l %llu %d %d\n", U(K(fle_imp_PhysicalToLogicalPoint, w, A(pt))), pt[0], pt[1]);
    SETLE(0);
    printf("bad %llu err %u\n", U(K(fle_imp_LogicalToPhysicalPoint, 0x1234, A(pt))), LE());
"#,
    );
    assert_eq!(l, ["l2p 1 123 -45", "p2l 1 123 -45", "bad 0 err 1400"]);
    aborts("l2p-null", "    uint64_t w = cw(POPUP | VISIBLE, 0, 0, 10, 10, 0);\n    K(fle_imp_LogicalToPhysicalPoint, w, 0);\n", "LogicalToPhysicalPoint", "lpPoint is NULL");
}

#[test]
fn set_keyboard_state_writes_the_table_that_get_key_state_reads() {
    // Learn SetKeyboardState: "Copies an array of keyboard key states into the calling thread's keyboard input-state table. This is the same table accessed by the GetKeyboardState and GetKeyState functions."
    // GetKeyState: high-order bit 1 = down, low-order bit 1 = toggled (a SHORT). GetAsyncKeyState is the hardware state: not touched (S-ia1-key-1).
    let l = lines(
        "setkbd",
        r#"
    static uint8_t st[256], back[256];
    unsigned k, same = 1;
    st[0x10] = 0x80; st[0x14] = 0x01; st[0x41] = 0x81;
    printf("before shift %llx caps %llx\n", U(K(fle_imp_GetKeyState, 0x10)), U(K(fle_imp_GetKeyState, 0x14)));
    printf("set %llu\n", U(K(fle_imp_SetKeyboardState, A(st))));
    printf("after shift %llx caps %llx a %llx ctrl %llx\n", U(K(fle_imp_GetKeyState, 0x10)) & 0xffff, U(K(fle_imp_GetKeyState, 0x14)) & 0xffff, U(K(fle_imp_GetKeyState, 0x41)) & 0xffff, U(K(fle_imp_GetKeyState, 0x11)) & 0xffff);
    printf("get %llu\n", U(K(fle_imp_GetKeyboardState, A(back))));
    for (k = 0; k < 256; k++) if (st[k] != back[k]) same = 0;
    printf("round trip %u\n", same);
    printf("async shift %llx\n", U(K(fle_imp_GetAsyncKeyState, 0x10)) & 0xffff);
"#,
    );
    assert_eq!(l, ["before shift 0 caps 0", "set 1", "after shift 8000 caps 1 a 8001 ctrl 0", "get 1", "round trip 1", "async shift 0"]);
    aborts("setkbd-null", "    K(fle_imp_SetKeyboardState, 0);\n", "SetKeyboardState", "lpKeyState is NULL");
}

#[test]
fn defwindowproc_answers_zero_for_the_ui_state_and_edit_undo_messages_and_still_stops_for_others() {
    // Learn WM_CHANGEUISTATE (0x127) and WM_UPDATEUISTATE (0x128): "If an application processes this message, it should return zero"; EM_CANUNDO (0xC6): an edit control message, a window that is not an edit
    // control has no default processing for it (S-ia1-defproc-1). Any other message below WM_USER that decision 0071 does not list still stops by name.
    let l = lines(
        "defproc",
        r#"
    uint64_t w = cw(POPUP | VISIBLE, 0, 0, 100, 100, 0);
    printf("0x127 %llu 0x128 %llu 0xc6 %llu\n", U(K(fle_imp_DefWindowProcW, w, 0x127, 0x10001, 0)), U(K(fle_imp_DefWindowProcW, w, 0x128, 1, 0)), U(K(fle_imp_DefWindowProcW, w, 0xC6, 0, 0)));
"#,
    );
    assert_eq!(l, ["0x127 0 0x128 0 0xc6 0"]);
    aborts("defproc-other", "    uint64_t w = cw(POPUP | VISIBLE, 0, 0, 10, 10, 0);\n    K(fle_imp_DefWindowProcW, w, 0xC7, 0, 0);\n", "DefWindowProcW", "message 0xc7");
}

const HOOK_TOP: &str = r#"
#define VA_HOOK_OLD 0x2100
#define VA_HOOK_NEW 0x2200
static int calls_old, calls_new, chain;
static uint64_t seen_ncode[4], seen_wp[4], seen_cwp_lp[4], seen_cwp_wp[4], seen_cwp_msg[4], seen_cwp_hwnd[4];
static void rec(unsigned k)
{
    seen_ncode[k] = R[1]; seen_wp[k] = R[2];
    seen_cwp_lp[k] = fle_ld(R[8], 8); seen_cwp_wp[k] = fle_ld(R[8] + 8, 8); seen_cwp_msg[k] = fle_ld(R[8] + 16, 4); seen_cwp_hwnd[k] = fle_ld(R[8] + 24, 8);
}
/* the older hook: records the call and returns 7 */
static void hook_old(void) { rec(0); calls_old++; R[0] = 7; }
/* the newer hook: records, and calls CallNextHookEx when `chain` is set (hhk is ignored), then returns what CallNextHookEx returned */
static void hook_new(void)
{
    uint64_t n = R[1], wp = R[2], lp = R[8], sp = R[4], res = 0;
    rec(1); calls_new++;
    if (chain) {
        R[1] = 0; R[2] = n; R[8] = wp; R[9] = lp;
        R[4] = sp - 8; fle_st(R[4], 8, 0xAAAA0002);
        fle_imp_CallNextHookEx();
        R[4] = sp;
        res = R[0];
    }
    R[0] = res;
}
"#;

#[test]
fn wh_callwndproc_hooks_see_sent_messages_newest_first_and_the_chain_goes_through_call_next_hook_ex() {
    // Learn CallWndProc callback: nCode HC_ACTION (0), wParam nonzero when the message was sent by the current thread, lParam points to a CWPSTRUCT {lParam, wParam, message, hwnd}; the hook procedure should call
    // CallNextHookEx, which "passes the hook information to the next hook procedure in the current hook chain" (hhk ignored); the most recently installed hook is called first.
    let o = run_top(
        "hook-chain",
        HOOK_TOP,
        ", { VA_HOOK_OLD, hook_old }, { VA_HOOK_NEW, hook_new }",
        2,
        r#"
    uint64_t w = cw(POPUP | VISIBLE, 0, 0, 100, 100, 0), h_old, h_new, r;
    h_old = K(fle_imp_SetWindowsHookExW, 4, VA_HOOK_OLD, 0, 0x1001);
    h_new = K(fle_imp_SetWindowsHookExW, 4, VA_HOOK_NEW, 0, 0x1001);
    printf("installed %d\n", h_old != 0 && h_new != 0 && h_old != h_new);
    r = K(fle_imp_SendMessageW, w, 0x405, 7, 9);
    printf("no chain: new %d old %d result %llu\n", calls_new, calls_old, U(r));
    printf("new saw ncode %llu sent_by_current %llu msg %llx wp %llu lp %llu hwnd_is_w %d\n", U(seen_ncode[1]), U(seen_wp[1]), U(seen_cwp_msg[1]), U(seen_cwp_wp[1]), U(seen_cwp_lp[1]), seen_cwp_hwnd[1] == w);
    chain = 1;
    r = K(fle_imp_SendMessageW, w, 0x406, 3, 4);
    printf("chain: new %d old %d\n", calls_new, calls_old);
    printf("old saw ncode %llu sent_by_current %llu msg %llx wp %llu lp %llu hwnd_is_w %d\n", U(seen_ncode[0]), U(seen_wp[0]), U(seen_cwp_msg[0]), U(seen_cwp_wp[0]), U(seen_cwp_lp[0]), seen_cwp_hwnd[0] == w);
    printf("unhook new %llu\n", U(K(fle_imp_UnhookWindowsHookEx, h_new)));
    K(fle_imp_SendMessageW, w, 0x407, 0, 0);
    printf("after unhook of the newer: new %d old %d\n", calls_new, calls_old);
    K(fle_imp_UnhookWindowsHookEx, h_old);
    K(fle_imp_SendMessageW, w, 0x408, 0, 0);
    printf("after both: new %d old %d\n", calls_new, calls_old);
"#,
    );
    assert!(o.ok, "{:?} {}\n{}", o.signal, o.stderr, o.stdout);
    let l: Vec<&str> = o.stdout.lines().collect();
    assert_eq!(
        l,
        [
            "installed 1",
            "no chain: new 1 old 0 result 0",
            "new saw ncode 0 sent_by_current 1 msg 405 wp 7 lp 9 hwnd_is_w 1",
            "chain: new 2 old 1",
            "old saw ncode 0 sent_by_current 1 msg 406 wp 3 lp 4 hwnd_is_w 1",
            "unhook new 1",
            "after unhook of the newer: new 2 old 2",
            "after both: new 2 old 2",
        ]
    );
}

#[test]
fn call_next_hook_ex_outside_a_hook_procedure_stops_by_name() {
    aborts("hook-outside", "    K(fle_imp_CallNextHookEx, 0, 0, 0, 0);\n", "CallNextHookEx", "called outside a hook procedure");
}
