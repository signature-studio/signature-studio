//! Tests of decision 0171 (docs/decisions/0171-user-operation-walls.md, PROVISIONAL): the walls that stopped the engine at the user's first operations in the main window.
//!
//! 1. The virtual clipboard (runtime/fle_clip.c): user32!OpenClipboard, CloseClipboard, EmptyClipboard, IsClipboardFormatAvailable, CountClipboardFormats, EnumClipboardFormats, GetClipboardData,
//!    SetClipboardData. The clipboard starts empty (nothing is read from the host). Expected values are written here from the Microsoft Learn pages named in fle_clip.c (OpenClipboard, CloseClipboard,
//!    EmptyClipboard, IsClipboardFormatAvailable, CountClipboardFormats, EnumClipboardFormats, GetClipboardData, SetClipboardData, Synthesized Clipboard Formats) and from ERROR_CLIPBOARD_NOT_OPEN = 1418,
//!    not read back from the C code. The choices the pages leave open are the S-ia1-clip-N of the decision.
//! 2. The import rows of the decision (harness/seeds/ia1-imports.txt): `fe-lift --extra-imports` accepts the file, every row names a function that the runtime defines, and without the option the rows
//!    are not in the table (the generated C of the default invocation is the one of the earlier decisions).
//!
//! EVERY ANSWER IS PROVISIONAL. Nothing is compared with a real Windows. FLE_IA1_RT_DIR (a full copy of runtime/) replaces runtime/ for the mutation runs.

mod etxt_common;

use std::path::PathBuf;
use std::process::Command;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn rt() -> PathBuf {
    match std::env::var_os("FLE_IA1_RT_DIR") {
        Some(d) => PathBuf::from(d),
        None => manifest().join("../../runtime"),
    }
}
fn work() -> PathBuf {
    let d = manifest().join("../../out-ia1/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

const PRE: &str = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_clip.h"
uint64_t R[16];
#define U(x) ((unsigned long long)(x))
#include "fle_callback.h"
static uint32_t last_err;
void fle_set_last_error(uint32_t c) { last_err = c; }
/* the guest calls that the runtime makes (fle_call_guest): recorded, answered with 1 */
static uint64_t gcalls[64][3];
static unsigned ngcalls;
static void (*guest_hook)(void);
uint64_t fle_call_guest(uint64_t va, uint64_t a, uint64_t b, uint64_t c, uint64_t d) { (void)c; (void)d; if (ngcalls < 64) { gcalls[ngcalls][0] = va; gcalls[ngcalls][1] = a; gcalls[ngcalls][2] = b; } ngcalls++; if (guest_hook) guest_hook(); return 1; }
void fle_stop(const char *api, const char *why) { printf("STOP %s: %s\n", api, why); exit(3); }
"#;

/// Compiles `PRE + body` with fle_clip.c, runs it, returns (exit code, stdout lines).
fn run(name: &str, body: &str) -> (i32, Vec<String>) {
    run_with(name, body, &["fle_clip.c"])
}

/// Like `run`, linking the runtime files in `files` (besides the stubs of PRE).
fn run_with(name: &str, body: &str, files: &[&str]) -> (i32, Vec<String>) {
    let dir = work();
    let u = etxt_common::uniq(name);
    let c = dir.join(format!("{u}.c"));
    let exe = dir.join(&u);
    std::fs::write(&c, format!("{PRE}{body}")).unwrap();
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-Wall", "-Wextra", "-fPIE", "-pie"]).arg("-I").arg(rt()).arg(&c);
    for f in files {
        cmd.arg(rt().join(f));
    }
    let cc = cmd
        .arg("-o")
        .arg(&exe)
        .env("TMPDIR", dir.join("tmp"))
        .output()
        .expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new(&exe).output().expect("run failed");
    (r.status.code().unwrap_or(-1), String::from_utf8_lossy(&r.stdout).lines().map(|s| s.to_string()).collect())
}

const HELPERS: &str = r#"
static uint64_t call1(void (*f)(void), uint64_t a) { R[0] = 0xdead; R[1] = a; f(); return R[0]; }
static uint64_t call2(void (*f)(void), uint64_t a, uint64_t b) { R[0] = 0xdead; R[1] = a; R[2] = b; f(); return R[0]; }
static uint64_t call0(void (*f)(void)) { R[0] = 0xdead; f(); return R[0]; }
"#;

fn prog(main: &str) -> String {
    format!("{HELPERS}\nint main(void)\n{{\n{main}\n    return 0;\n}}\n")
}

#[test]
fn a_new_clipboard_is_empty() {
    // Learn IsClipboardFormatAvailable: zero when the format is not on the clipboard. CountClipboardFormats: zero for an empty clipboard.
    let (rc, l) = run(
        "clip-empty",
        &prog(
            r#"
    printf("text %llu\n", U(call1(fle_imp_IsClipboardFormatAvailable, 1)));
    printf("unicode %llu\n", U(call1(fle_imp_IsClipboardFormatAvailable, 13)));
    printf("hdrop %llu\n", U(call1(fle_imp_IsClipboardFormatAvailable, 15)));
    printf("registered %llu\n", U(call1(fle_imp_IsClipboardFormatAvailable, 0xC001)));
    printf("count %llu\n", U(call0(fle_imp_CountClipboardFormats)));
    printf("open %d\n", fle_clip_is_open());
"#,
        ),
    );
    assert_eq!(rc, 0);
    assert_eq!(l, ["text 0", "unicode 0", "hdrop 0", "registered 0", "count 0", "open 0"]);
}

#[test]
fn open_close_and_the_not_open_errors() {
    // Learn: CloseClipboard / EmptyClipboard / GetClipboardData / SetClipboardData / EnumClipboardFormats need an open clipboard; ERROR_CLIPBOARD_NOT_OPEN (1418).
    let (rc, l) = run(
        "clip-open",
        &prog(
            r#"
    printf("close_closed %llu err %u\n", U(call0(fle_imp_CloseClipboard)), last_err);
    last_err = 0;
    printf("empty_closed %llu err %u\n", U(call0(fle_imp_EmptyClipboard)), last_err);
    last_err = 0;
    printf("get_closed %llu err %u\n", U(call1(fle_imp_GetClipboardData, 13)), last_err);
    last_err = 0;
    printf("set_closed %llu err %u\n", U(call2(fle_imp_SetClipboardData, 13, 0x1000)), last_err);
    last_err = 0;
    printf("enum_closed %llu err %u\n", U(call1(fle_imp_EnumClipboardFormats, 0)), last_err);
    printf("open %llu\n", U(call1(fle_imp_OpenClipboard, 0x7F990008)));
    printf("is_open %d\n", fle_clip_is_open());
    printf("close %llu\n", U(call0(fle_imp_CloseClipboard)));
    printf("is_open %d\n", fle_clip_is_open());
"#,
        ),
    );
    assert_eq!(rc, 0);
    assert_eq!(
        l,
        ["close_closed 0 err 1418", "empty_closed 0 err 1418", "get_closed 0 err 1418", "set_closed 0 err 1418", "enum_closed 0 err 1418", "open 1", "is_open 1", "close 1", "is_open 0"]
    );
}

#[test]
fn open_fails_for_another_window_while_open() {
    // Learn OpenClipboard: "fails if another window has the clipboard open" (ERROR_ACCESS_DENIED 5 is the choice S-ia1-clip-1); after CloseClipboard another window can open it.
    let (rc, l) = run(
        "clip-other",
        &prog(
            r#"
    printf("a %llu\n", U(call1(fle_imp_OpenClipboard, 0x7F990008)));
    last_err = 0;
    printf("b %llu err %u\n", U(call1(fle_imp_OpenClipboard, 0x7F99000C)), last_err);
    printf("a_again %llu\n", U(call1(fle_imp_OpenClipboard, 0x7F990008)));
    printf("close %llu\n", U(call0(fle_imp_CloseClipboard)));
    printf("b2 %llu\n", U(call1(fle_imp_OpenClipboard, 0x7F99000C)));
"#,
        ),
    );
    assert_eq!(rc, 0);
    assert_eq!(l, ["a 1", "b 0 err 5", "a_again 1", "close 1", "b2 1"]);
}

#[test]
fn set_get_enum_count_and_empty() {
    let (rc, l) = run(
        "clip-set",
        &prog(
            r#"
    call1(fle_imp_OpenClipboard, 0x7F990008);
    printf("empty %llu\n", U(call0(fle_imp_EmptyClipboard)));
    printf("set_hdrop %llx\n", U(call2(fle_imp_SetClipboardData, 15, 0x5000)));
    printf("set_reg %llx\n", U(call2(fle_imp_SetClipboardData, 0xC001, 0x6000)));
    printf("count %llu\n", U(call0(fle_imp_CountClipboardFormats)));
    printf("avail_hdrop %llu\n", U(call1(fle_imp_IsClipboardFormatAvailable, 15)));
    printf("avail_reg %llu\n", U(call1(fle_imp_IsClipboardFormatAvailable, 0xC001)));
    printf("avail_other %llu\n", U(call1(fle_imp_IsClipboardFormatAvailable, 0xC002)));
    printf("get_hdrop %llx\n", U(call1(fle_imp_GetClipboardData, 15)));
    printf("get_reg %llx\n", U(call1(fle_imp_GetClipboardData, 0xC001)));
    printf("get_none %llx\n", U(call1(fle_imp_GetClipboardData, 0xC002)));
    last_err = 99;
    printf("enum0 %llu err %u\n", U(call1(fle_imp_EnumClipboardFormats, 0)), last_err);
    printf("enum15 %llu\n", U(call1(fle_imp_EnumClipboardFormats, 15)));
    last_err = 99;
    printf("enum_last %llu err %u\n", U(call1(fle_imp_EnumClipboardFormats, 0xC001)), last_err);
    printf("replace %llx\n", U(call2(fle_imp_SetClipboardData, 15, 0x5100)));
    printf("count %llu\n", U(call0(fle_imp_CountClipboardFormats)));
    printf("get_hdrop %llx\n", U(call1(fle_imp_GetClipboardData, 15)));
    printf("empty %llu\n", U(call0(fle_imp_EmptyClipboard)));
    printf("count %llu\n", U(call0(fle_imp_CountClipboardFormats)));
    printf("avail_hdrop %llu\n", U(call1(fle_imp_IsClipboardFormatAvailable, 15)));
    printf("get_hdrop %llx\n", U(call1(fle_imp_GetClipboardData, 15)));
    call0(fle_imp_CloseClipboard);
"#,
        ),
    );
    assert_eq!(rc, 0);
    assert_eq!(
        l,
        [
            "empty 1",
            "set_hdrop 5000",
            "set_reg 6000",
            "count 2",
            "avail_hdrop 1",
            "avail_reg 1",
            "avail_other 0",
            "get_hdrop 5000",
            "get_reg 6000",
            "get_none 0",
            "enum0 15 err 0",
            "enum15 49153",
            "enum_last 0 err 0",
            "replace 5100",
            "count 2",
            "get_hdrop 5100",
            "empty 1",
            "count 0",
            "avail_hdrop 0",
            "get_hdrop 0",
        ]
    );
}

#[test]
fn the_data_stays_after_close_and_text_formats_are_synthesized() {
    // Learn: the data stays on the clipboard after CloseClipboard. Learn, Synthesized Clipboard Formats: CF_UNICODETEXT (13) makes CF_TEXT (1) and CF_OEMTEXT (7) available; the three are counted.
    let (rc, l) = run(
        "clip-synth",
        &prog(
            r#"
    call1(fle_imp_OpenClipboard, 0);
    call0(fle_imp_EmptyClipboard);
    call2(fle_imp_SetClipboardData, 13, 0x7000);
    call0(fle_imp_CloseClipboard);
    printf("unicode %llu\n", U(call1(fle_imp_IsClipboardFormatAvailable, 13)));
    printf("text %llu\n", U(call1(fle_imp_IsClipboardFormatAvailable, 1)));
    printf("oem %llu\n", U(call1(fle_imp_IsClipboardFormatAvailable, 7)));
    printf("bitmap %llu\n", U(call1(fle_imp_IsClipboardFormatAvailable, 2)));
    printf("count %llu\n", U(call0(fle_imp_CountClipboardFormats)));
    call1(fle_imp_OpenClipboard, 0);
    printf("get_unicode %llx\n", U(call1(fle_imp_GetClipboardData, 13)));
    printf("get_text\n");
    call1(fle_imp_GetClipboardData, 1);
    printf("not reached\n");
"#,
        ),
    );
    assert_eq!(rc, 3, "a synthesized format is converted by the system: the runtime stops by name");
    assert_eq!(
        l,
        [
            "unicode 1",
            "text 1",
            "oem 1",
            "bitmap 0",
            "count 3",
            "get_unicode 7000",
            "get_text",
            "STOP GetClipboardData: a synthesized format (the system converts the data): not implemented"
        ]
    );
}

#[test]
fn a_delayed_rendering_entry_stops_by_name_when_read() {
    // Learn SetClipboardData: a NULL handle announces delayed rendering (WM_RENDERFORMAT to the owner); the format is available, reading it needs a message to the owner: not implemented.
    let (rc, l) = run(
        "clip-delayed",
        &prog(
            r#"
    call1(fle_imp_OpenClipboard, 0x7F990008);
    call0(fle_imp_EmptyClipboard);
    printf("set %llx\n", U(call2(fle_imp_SetClipboardData, 15, 0)));
    printf("avail %llu\n", U(call1(fle_imp_IsClipboardFormatAvailable, 15)));
    call1(fle_imp_GetClipboardData, 15);
    printf("not reached\n");
"#,
        ),
    );
    assert_eq!(rc, 3);
    assert_eq!(l[0], "set 0");
    assert_eq!(l[1], "avail 1");
    assert!(l[2].starts_with("STOP GetClipboardData: delayed rendering"), "{:?}", l[2]);
}

// ---- the OLE clipboard: the data object pointer only ----

#[test]
fn ole_clipboard_keeps_one_reference_and_answers_s_ok_or_s_false() {
    // Learn OleSetClipboard / OleIsCurrentClipboard / OleFlushClipboard: S_OK = 0, S_FALSE = 1; the object gets AddRef (IUnknown vtable slot 1) when it is placed and Release (slot 2) when it is replaced or flushed.
    let (rc, l) = run(
        "ole-clip",
        &prog(
            r#"
    uint64_t vt1[3] = { 0, 0xA1, 0xA2 }, vt2[3] = { 0, 0xB1, 0xB2 };
    uint64_t o1 = (uint64_t)(uintptr_t)vt1, o2 = (uint64_t)(uintptr_t)vt2;   /* an object is a pointer to its vtable pointer */
    uint64_t obj1 = (uint64_t)(uintptr_t)&o1, obj2 = (uint64_t)(uintptr_t)&o2;
    unsigned i;
    printf("none %llu\n", U(call1(fle_imp_OleIsCurrentClipboard, obj1)));
    printf("null %llu\n", U(call1(fle_imp_OleIsCurrentClipboard, 0)));
    printf("flush_empty %llu calls %u\n", U(call0(fle_imp_OleFlushClipboard)), ngcalls);
    printf("set1 %llu calls %u\n", U(call1(fle_imp_OleSetClipboard, obj1)), ngcalls);
    printf("cur1 %llu cur2 %llu\n", U(call1(fle_imp_OleIsCurrentClipboard, obj1)), U(call1(fle_imp_OleIsCurrentClipboard, obj2)));
    printf("set2 %llu\n", U(call1(fle_imp_OleSetClipboard, obj2)));
    printf("cur1 %llu cur2 %llu\n", U(call1(fle_imp_OleIsCurrentClipboard, obj1)), U(call1(fle_imp_OleIsCurrentClipboard, obj2)));
    printf("flush %llu\n", U(call0(fle_imp_OleFlushClipboard)));
    printf("cur2 %llu\n", U(call1(fle_imp_OleIsCurrentClipboard, obj2)));
    printf("set1 %llu\n", U(call1(fle_imp_OleSetClipboard, obj1)));
    printf("clear %llu\n", U(call1(fle_imp_OleSetClipboard, 0)));
    printf("cur1 %llu\n", U(call1(fle_imp_OleIsCurrentClipboard, obj1)));
    for (i = 0; i < ngcalls; i++) printf("call %llx this %s\n", U(gcalls[i][0]), gcalls[i][1] == obj1 ? "obj1" : gcalls[i][1] == obj2 ? "obj2" : "?");
"#,
        ),
    );
    assert_eq!(rc, 0);
    assert_eq!(
        l,
        [
            "none 1",
            "null 1",
            "flush_empty 0 calls 0",
            "set1 0 calls 1",
            "cur1 0 cur2 1",
            "set2 0",
            "cur1 1 cur2 0",
            "flush 0",
            "cur2 1",
            "set1 0",
            "clear 0",
            "cur1 1",
            "call a1 this obj1",
            "call b1 this obj2",
            "call a2 this obj1",
            "call b2 this obj2",
            "call a1 this obj1",
            "call a2 this obj1",
        ]
    );
}

#[test]
fn setting_the_ole_clipboard_empties_the_win32_clipboard() {
    // S-ia1-ole-1: OleSetClipboard empties the virtual Win32 clipboard (the formats of the data object are not copied onto it).
    let (rc, l) = run(
        "ole-empties",
        &prog(
            r#"
    uint64_t vt[3] = { 0, 0xA1, 0xA2 };
    uint64_t o = (uint64_t)(uintptr_t)vt, obj = (uint64_t)(uintptr_t)&o;
    call1(fle_imp_OpenClipboard, 0);
    call0(fle_imp_EmptyClipboard);
    call2(fle_imp_SetClipboardData, 15, 0x5000);
    call0(fle_imp_CloseClipboard);
    printf("before %llu\n", U(call0(fle_imp_CountClipboardFormats)));
    call1(fle_imp_OleSetClipboard, obj);
    printf("after %llu\n", U(call0(fle_imp_CountClipboardFormats)));
    printf("hdrop %llu\n", U(call1(fle_imp_IsClipboardFormatAvailable, 15)));
"#,
        ),
    );
    assert_eq!(rc, 0);
    assert_eq!(l, ["before 1", "after 0", "hdrop 0"]);
}

// ---- PathCleanupSpec (runtime/fle_shell.c) ----

fn w16(s: &str) -> String {
    let mut v: Vec<String> = s.encode_utf16().map(|u| u.to_string()).collect();
    v.push("0".into());
    format!("{{ {} }}", v.join(", "))
}

fn pcs(dir: Option<&str>, spec: &str) -> (i32, Vec<String>) {
    let d = dir.map(|d| format!("static uint16_t dir[] = {};\n", w16(d))).unwrap_or_default();
    let body = format!(
        "{d}static uint16_t spec[] = {};\nint main(void)\n{{\n    R[0] = 99; R[1] = {}; R[2] = (uint64_t)(uintptr_t)spec;\n    fle_imp_PathCleanupSpec();\n    printf(\"ret %llu\\n\", (unsigned long long)R[0]);\n    return 0;\n}}\n",
        w16(spec),
        if dir.is_some() { "(uint64_t)(uintptr_t)dir" } else { "0" }
    );
    run_with("pcs", &format!("#include \"fle_shell.h\"\n{body}"), &["fle_shell.c"])
}

#[test]
fn path_cleanup_spec_answers_zero_for_a_name_that_needs_no_change() {
    // Learn PathCleanupSpec: 0 = nothing changed. Only the unchanged case is answered (S-ia1-pcs-1).
    assert_eq!(pcs(None, "Sampler"), (0, vec!["ret 0".to_string()]));
    assert_eq!(pcs(Some("C:\\Users\\user\\Documents"), "My song 1.flp"), (0, vec!["ret 0".to_string()]));
}

#[test]
fn path_cleanup_spec_stops_by_name_when_a_character_would_have_to_change() {
    for spec in ["a:b", "a*b", "a?b", "a<b", "a>b", "a|b", "a\"b", "a/b", "a\\b"] {
        let (rc, l) = pcs(None, spec);
        assert_eq!(rc, 3, "{spec}");
        assert!(l[0].starts_with("STOP PathCleanupSpec: pszSpec holds an illegal character"), "{spec}: {l:?}");
    }
}

#[test]
fn path_cleanup_spec_stops_by_name_when_the_path_would_reach_max_path() {
    let long = "a".repeat(250);
    let (rc, l) = pcs(Some(&"d".repeat(20)), &long);
    assert_eq!(rc, 3);
    assert!(l[0].starts_with("STOP PathCleanupSpec: the path would reach MAX_PATH"), "{l:?}");
}

// ---- timer-queue timers (runtime/fle_timerq.c) with the thread functions replaced by recorders ----

const TQ_PRE: &str = r#"
#include "fle_timerq.h"
#include "fle_thread.h"
/* fle_thread_spawn / fle_thread_sleep recorders: a thread is made but not started; tq_run() runs the body of the n-th thread to its end on the calling stack, and every sleep is logged */
static void (*bodies[8])(uint64_t);
static uint64_t bargs[8];
static unsigned nspawn;
uint64_t fle_thread_spawn(void (*b)(uint64_t), uint64_t a) { bodies[nspawn] = b; bargs[nspawn] = a; return 0x7FFAA0000004ull + 4 * nspawn++; }
static uint32_t sleeps[32];
static unsigned nsleep;
static void (*sleep_hook)(void);   /* runs during a sleep: what another thread does while the timer thread is parked */
void fle_thread_sleep(const char *api, uint32_t ms) { (void)api; if (nsleep < 32) sleeps[nsleep++] = ms; if (sleep_hook) { void (*h)(void) = sleep_hook; sleep_hook = 0; h(); } }
static void tq_run(unsigned n) { bodies[n](bargs[n]); }
static uint64_t handle_cell;
static uint64_t fake_stack[16];
static void fs(void) { memset(fake_stack, 0, sizeof fake_stack); R[4] = (uint64_t)(uintptr_t)fake_stack; }
static uint64_t create(uint64_t cb, uint64_t param, uint32_t due, uint32_t period, uint32_t flags)
{
    uint64_t sp[16];
    R[0] = 0xdead; R[1] = (uint64_t)(uintptr_t)&handle_cell; R[2] = 0; R[8] = cb; R[9] = param;
    /* stack arguments 5, 6, 7 at [rsp + 0x28 ...]: R[4] points at a fake return slot of a fake stack */
    memset(sp, 0, sizeof sp);
    sp[5] = due; sp[6] = period; sp[7] = flags;
    R[4] = (uint64_t)(uintptr_t)sp;
    fle_imp_CreateTimerQueueTimer();
    return R[0];
}
static uint64_t del(uint64_t h, uint64_t ev) { R[0] = 0xdead; R[1] = 0; R[2] = h; R[8] = ev; fle_imp_DeleteTimerQueueTimer(); return R[0]; }
static uint64_t change(uint64_t h, uint32_t due, uint32_t period) { R[0] = 0xdead; R[1] = 0; R[2] = h; R[8] = due; R[9] = period; fle_imp_ChangeTimerQueueTimer(); return R[0]; }
static void print_sleeps(void) { unsigned i; printf("sleeps"); for (i = 0; i < nsleep; i++) printf(" %u", (unsigned)sleeps[i]); printf("\n"); }
"#;

fn tq(name: &str, main: &str) -> (i32, Vec<String>) {
    tq_top(name, "", main)
}

/// `top` is C at file scope (hooks), `main` the body of main.
fn tq_top(name: &str, top: &str, main: &str) -> (i32, Vec<String>) {
    run_with(name, &format!("{TQ_PRE}\n{top}\nint main(void)\n{{\n{main}\n    return 0;\n}}\n"), &["fle_timerq.c"])
}

#[test]
fn a_one_shot_timer_sleeps_the_due_time_and_calls_back_once_with_timer_fired_true() {
    // Learn CreateTimerQueueTimer: the callback WAITORTIMERCALLBACK(lpParameter, TimerOrWaitFired) is called when the timer expires; for a timer TimerOrWaitFired is TRUE; Period 0: "the timer is signaled once".
    let (rc, l) = tq(
        "tq-once",
        r#"
    printf("create %llu\n", U(create(0x6d86d0, 0x1234, 50, 0, 0)));
    printf("handle %llx live %u spawned %u\n", U(handle_cell), fle_timerq_live(), nspawn);
    tq_run(0);
    print_sleeps();
    printf("calls %u va %llx param %llx fired %llx\n", ngcalls, U(gcalls[0][0]), U(gcalls[0][1]), U(gcalls[0][2]));
    printf("delete %llu live %u\n", U(del(handle_cell, 0)), fle_timerq_live());
"#,
    );
    assert_eq!(rc, 0);
    assert_eq!(l, ["create 1", "handle 7ffab0000004 live 1 spawned 1", "sleeps 50", "calls 1 va 6d86d0 param 1234 fired 1", "delete 1 live 0"]);
}

#[test]
fn a_periodic_timer_sleeps_due_then_period_until_it_is_deleted_from_its_own_callback() {
    // Learn DeleteTimerQueueTimer: "If the error value is ERROR_IO_PENDING, it is not necessary to call this function again": deleting from the callback gives 0 and ERROR_IO_PENDING (997), and the timer ends.
    let (rc, l) = tq_top(
        "tq-periodic",
        r#"static unsigned n;
static void hook(void) { if (++n == 3) { last_err = 0; printf("del_in_cb %llu err %u\n", U(del(handle_cell, 0)), last_err); } }"#,
        r#"
    guest_hook = hook;
    create(0x6d86d0, 7, 10, 20, 0);
    tq_run(0);
    print_sleeps();
    printf("calls %u live %u\n", ngcalls, fle_timerq_live());
"#,
    );
    assert_eq!(rc, 0);
    assert_eq!(l, ["del_in_cb 0 err 997", "sleeps 10 20 20", "calls 3 live 0"]);
}

#[test]
fn wt_executeonlyonce_stops_a_periodic_timer_after_the_first_call() {
    // Learn CreateTimerQueueTimer, WT_EXECUTEONLYONCE (0x8): "The callback function is invoked only once for the thread".
    let (rc, l) = tq("tq-onlyonce", "    create(0x6d86d0, 7, 5, 20, 8);\n    tq_run(0);\n    print_sleeps();\n    printf(\"calls %u\\n\", ngcalls);\n");
    assert_eq!(rc, 0);
    assert_eq!(l, ["sleeps 5", "calls 1"]);
}

#[test]
fn change_from_outside_cancels_the_old_wait_and_starts_a_new_one() {
    // Learn ChangeTimerQueueTimer: DueTime is relative to now; the old due time no longer applies. The first thread is parked in its 100 ms sleep when the change comes; when it wakes it sees that it is obsolete and ends without a callback.
    let (rc, l) = tq_top(
        "tq-change",
        r#"static void during_sleep(void) { printf("change %llu spawned %u\n", U(change(handle_cell, 5, 0)), nspawn); }"#,
        r#"
    create(0x6d86d0, 7, 100, 0, 0);
    sleep_hook = during_sleep;
    tq_run(0);
    printf("old: calls %u\n", ngcalls);
    tq_run(1);
    print_sleeps();
    printf("calls %u\n", ngcalls);
"#,
    );
    assert_eq!(rc, 0);
    assert_eq!(l, ["change 1 spawned 2", "old: calls 0", "sleeps 100 5", "calls 1"]);
}

#[test]
fn change_from_the_callback_sets_the_next_wait() {
    // Learn ChangeTimerQueueTimer: "If you call this function from the callback, the timer is rescheduled": the next wait is the new DueTime, then the new Period.
    let (rc, l) = tq_top(
        "tq-change-cb",
        r#"static unsigned n;
static void hook(void) { ++n; if (n == 1) change(handle_cell, 3, 9); if (n == 3) del(handle_cell, 0); }"#,
        r#"
    guest_hook = hook;
    create(0x6d86d0, 7, 10, 20, 0);
    tq_run(0);
    print_sleeps();
    printf("calls %u\n", ngcalls);
"#,
    );
    assert_eq!(rc, 0);
    assert_eq!(l, ["sleeps 10 3 9", "calls 3"]);
}

#[test]
fn what_the_page_does_not_define_stops_by_name() {
    let cases: [(&str, &str, &str); 5] = [
        ("tq-stop-queue", "R[1] = (uint64_t)(uintptr_t)&handle_cell; R[2] = 0x1000; fle_imp_CreateTimerQueueTimer();", "STOP CreateTimerQueueTimer: TimerQueue is not NULL"),
        ("tq-stop-ph", "R[1] = 0; R[2] = 0; R[8] = 1; fle_imp_CreateTimerQueueTimer();", "STOP CreateTimerQueueTimer: phNewTimer is NULL"),
        ("tq-stop-cb", "R[1] = (uint64_t)(uintptr_t)&handle_cell; R[2] = 0; R[8] = 0; fle_imp_CreateTimerQueueTimer();", "STOP CreateTimerQueueTimer: Callback is NULL"),
        ("tq-stop-flag", "create(1, 1, 1, 1, 0x2000);", "STOP CreateTimerQueueTimer: Flags has a bit"),
        ("tq-stop-handle", "del(0x7FFAB0000044ull, 0);", "STOP DeleteTimerQueueTimer: Timer is not a handle"),
    ];
    for (name, code, want) in cases {
        let (rc, l) = tq(name, &format!("    fs();\n    {code}\n    printf(\"not reached\\n\");\n"));
        assert_eq!(rc, 3, "{name}: {l:?}");
        assert!(l[0].starts_with(want), "{name}: {l:?}");
    }
}

// ---- dwmapi (runtime/fle_dwm.c) ----

const DWM_PRE: &str = r#"
#include "fle_win.h"
#include "fle_dwm.h"
int fle_win_get(uint64_t hwnd, fle_win_view *v) { (void)v; return hwnd == 0x7F990004ull; }
/* decision 0244: the window table keeps the margins (fle_win.c); here a stub that remembers the last call */
static uint64_t kept_h; static int32_t kept[4];
void fle_win_set_frame_margins(uint64_t hwnd, const int32_t m[4]) { kept_h = hwnd; for (int k = 0; k < 4; k++) kept[k] = m[k]; }
/* decision 0302: fle_imp_DwmGetCompositionTimingInfo reads the virtual clock (fle_sys.c) and the refresh rate (fle_thread.c); tests/dwm_timing.rs runs it, these two are only here to link */
uint64_t fle_sys_clock_ticks(void) { return 0; }
void fle_sys_clock_advance_to(uint64_t ticks) { (void)ticks; }
"#;

fn dwm(name: &str, main: &str) -> (i32, Vec<String>) {
    run_with(name, &format!("{DWM_PRE}\nint main(void)\n{{\n{main}\n    return 0;\n}}\n"), &["fle_dwm.c"])
}

#[test]
fn dwm_composition_is_enabled_because_the_host_manifest_declares_windows_8() {
    // Learn DwmIsCompositionEnabled: S_OK; "As of Windows 8, DWM composition is always enabled. If an app declares Windows 8 compatibility in their manifest, this function will receive a value of TRUE through
    // pfEnabled": the manifest of FL64.exe lists supportedOS {4a2f28e3-...} (Windows 8) and {1f676c76-...} (Windows 8.1) (S-ia1-dwm-1).
    let (rc, l) = dwm(
        "dwm-enabled",
        r#"
    uint32_t cell = 0xdeadbeef;
    R[0] = 99; R[1] = (uint64_t)(uintptr_t)&cell;
    fle_imp_DwmIsCompositionEnabled();
    printf("hr %llu value %u\n", (unsigned long long)R[0], (unsigned)cell);
"#,
    );
    assert_eq!(rc, 0);
    assert_eq!(l, ["hr 0 value 1"]);
    let (rc, l) = dwm("dwm-null", "    R[1] = 0;\n    fle_imp_DwmIsCompositionEnabled();\n    printf(\"not reached\\n\");\n");
    assert_eq!(rc, 3);
    assert!(l[0].starts_with("STOP DwmIsCompositionEnabled: pfEnabled is NULL"), "{l:?}");
}

#[test]
fn dwm_extend_frame_keeps_the_margins_and_stops_by_name_for_what_the_page_does_not_define() {
    let (rc, l) = dwm(
        "dwm-extend",
        r#"
    int32_t margins[4] = { -1, 2, 3, 4 };
    R[0] = 99; R[1] = 0x7F990004ull; R[2] = (uint64_t)(uintptr_t)margins;
    fle_imp_DwmExtendFrameIntoClientArea();
    printf("hr %llu\n", (unsigned long long)R[0]);
    printf("kept %llx %d %d %d %d\n", (unsigned long long)kept_h, kept[0], kept[1], kept[2], kept[3]);   /* MARGINS: cxLeftWidth, cxRightWidth, cyTopHeight, cyBottomHeight */
"#,
    );
    assert_eq!(rc, 0);
    assert_eq!(l, ["hr 0", "kept 7f990004 -1 2 3 4"]);
    let (rc, l) = dwm("dwm-extend-badwin", "    int32_t m[4] = {0};\n    R[1] = 0x1234; R[2] = (uint64_t)(uintptr_t)m;\n    fle_imp_DwmExtendFrameIntoClientArea();\n");
    assert_eq!(rc, 3);
    assert!(l[0].starts_with("STOP DwmExtendFrameIntoClientArea: hWnd is not a window"), "{l:?}");
    let (rc, l) = dwm("dwm-extend-null", "    R[1] = 0x7F990004ull; R[2] = 0;\n    fle_imp_DwmExtendFrameIntoClientArea();\n");
    assert_eq!(rc, 3);
    assert!(l[0].starts_with("STOP DwmExtendFrameIntoClientArea: pMarInset is NULL"), "{l:?}");
}
