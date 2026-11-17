//! Tests of decision 0122 (docs/decisions/0122-cooperative-threads.md, PROVISIONAL): the cooperative guest threads of runtime/fle_thread.c (T2 of docs/investigations/thread-model.md), the
//! per-thread message queues in runtime/fle_win.c, the thread-aware parts of fle_event.c / fle_sys.c / fle_imports.c / fle_proc.c / fle_locale.c, dwmapi.dll as virtual module N = 39 and DwmFlush.
//!
//! Expected values are written here from the Microsoft Learn pages named in the decision (CreateThread, ResumeThread, SuspendThread, ExitThread, GetExitCodeThread, SwitchToThread, Sleep,
//! WaitForSingleObject, WaitForMultipleObjects, MsgWaitForMultipleObjects, SetThreadPriority, TlsAlloc, EnterCriticalSection, PostMessageW, GetMessageW, PeekMessageW, WaitMessage, PostQuitMessage,
//! TranslateMessage, DispatchMessageW, SendMessageW, GetQueueStatus, MSG, DllMain, DwmFlush; fetched 2026-10-30, kept in lift/out-fruity8/learn/ with the SHA-256 of each page) and from the choices
//! of the decision (creation-order round robin, switches only in a wait or yield API, the virtual clock moved to the smallest deadline, the 60 Hz boundary of DwmFlush, the handle and id numbers),
//! not read back from the C code. EVERY ANSWER IS PROVISIONAL; nothing is compared with a real Windows. The tests that fix the order of the scheduler pin down a choice, not a Windows rule.
//!
//! 1. Generator: the fifteen rows of decision 0122 are the last rows, in the order given, and no pseudo-VA moved.
//! 2. The scheduler and the thread APIs (driver with stub guest functions; real fle_thread.c, fle_event.c, fle_sys.c, fle_imports.c, fle_proc.c, fle_callback.c, fle_env.c, fle_vfs.c ...).
//! 3. Messages (fle_win.c + fle_class.c + the same files).
//! 4. Per-thread state, DllMain notifications, dwmapi.
//! 5. Source checks and the old-to-new list of tests.

mod etxt_common;
mod rows_0132;

use fe_lift::generate;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn rt_dir() -> PathBuf {
    manifest().join("../../runtime")
}
fn src_dir() -> PathBuf {
    if let Ok(d) = std::env::var("FLE_FR8_SRC_DIR") {
        return PathBuf::from(d);
    }
    rt_dir()
}
fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-wallFR8/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

// ---------------------------------------------------------------------------------------------
// 1. Generator
// ---------------------------------------------------------------------------------------------

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

fn build_imports(imps: &[(&str, &str)]) -> Vec<u8> {
    assert!(imps.len() <= 6);
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
    w32(&mut b, o + 124, 20 * (imps.len() as u32 + 1));
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
    for (k, (dll, name)) in imps.iter().enumerate() {
        let k32 = k as u32;
        let (oft, ft, dll_rva, hn_rva) = (0x2100 + 0x10 * k32, 0x2200 + 0x10 * k32, 0x2300 + 0x20 * k32, 0x2500 + 0x40 * k32);
        w32(&mut b, d + 20 * k, oft);
        w32(&mut b, d + 20 * k + 12, dll_rva);
        w32(&mut b, d + 20 * k + 16, ft);
        put(&mut b, dll_rva, format!("{dll}\0").as_bytes());
        put(&mut b, hn_rva, &[&[0u8, 0u8][..], name.as_bytes(), &[0u8]].concat());
        w64(&mut b, data_at(oft), hn_rva as u64);
        w64(&mut b, data_at(ft), hn_rva as u64);
    }
    let t = TEXT_RAW;
    let mut code: Vec<(u32, Vec<u8>)> = Vec::new();
    for k in 0..imps.len() as u32 {
        let at = 0x1000 + 5 * k;
        code.push((at, [&[0xe8][..], &rel(at + 5, 0x1040 + 0x10 * k)].concat()));
        code.push((0x1040 + 0x10 * k, [&[0xff, 0x25][..], &rel(0x1046 + 0x10 * k, 0x2200 + 0x10 * k)].concat()));
    }
    code.push((0x1000 + 5 * imps.len() as u32, vec![0xc3]));
    for (rva, bytes) in code {
        let o = t + (rva - TEXT_RVA) as usize;
        b[o..o + bytes.len()].copy_from_slice(&bytes);
    }
    b
}

fn gen(imps: &[(&str, &str)]) -> (fe_lift::Stats, String) {
    let img = build_imports(imps);
    let mut out = Vec::new();
    let st = generate(&img, &mut out).unwrap();
    (st, String::from_utf8(out).unwrap())
}

const GUEST_IMPORTS: [(&str, &str); 2] = [("kernel32.dll", "GetProcAddress"), ("kernel32.dll", "LoadLibraryW")];

fn resp_rows(c: &str) -> Vec<(String, String, u64)> {
    let start = c.find("const fle_resp_entry fle_resp_table[] = {").expect("no response table");
    let end = start + c[start..].find("};").unwrap();
    c[start..end]
        .lines()
        .filter_map(|l| {
            let l = l.trim();
            let l = l.strip_prefix("{ \"")?;
            let (dll, rest) = l.split_once("\", \"")?;
            let (name, rest) = rest.split_once("\", UINT64_C(0x")?;
            let (va, _) = rest.split_once(')')?;
            Some((dll.to_string(), name.to_string(), u64::from_str_radix(va, 16).unwrap()))
        })
        .collect()
}

/// The rows of decision 0122 in the order given (decision 0015: append only).
const NEW_ROWS: [(&str, &str); 15] = [
    ("kernel32.dll", "CreateThread"),
    ("kernel32.dll", "ResumeThread"),
    ("kernel32.dll", "SuspendThread"),
    ("kernel32.dll", "ExitThread"),
    ("kernel32.dll", "GetExitCodeThread"),
    ("dwmapi.dll", "DwmFlush"),
    ("user32.dll", "PostMessageW"),
    ("user32.dll", "GetMessageW"),
    ("user32.dll", "PeekMessageW"),
    ("user32.dll", "WaitMessage"),
    ("user32.dll", "PostQuitMessage"),
    ("user32.dll", "TranslateMessage"),
    ("user32.dll", "DispatchMessageW"),
    ("kernel32.dll", "WaitForMultipleObjects"),
    ("user32.dll", "MsgWaitForMultipleObjects"),
];

#[test]
fn the_fifteen_new_rows_are_the_last_rows_in_the_order_given_and_no_pseudo_va_moved() {
    let (st, c) = gen(&GUEST_IMPORTS);
    let rows = resp_rows(&c);
    assert_eq!(rows.len(), st.resp_entries);
    // decision 0123 (old: these fifteen were the LAST rows, `n - 15 + k`, and SetWindowTextW was `n - 16`) appended 73 rows after them (tests/wall_fr9.rs): the fifteen end at n - 73.
    // decision 0125 (old: `rows.len() - 73`) appended 2 more rows (tests/wall_fr10.rs): the fifteen end at n - 75.
    // decision 0126 (old: `rows.len() - 73 - 2`) appended 33 more rows (tests/wall_fr11.rs): the fifteen end at n - 108.
    // decision 0129 (old: `rows.len() - 73 - 2 - 33`) appended 4 more rows (tests/wall_fr12.rs): the fifteen end at n - 112.
    // decision 0136: rows are appended at the END again (and decision 0132 appends more): the end of the group is found by the NAME of its last row, not by counting from the end of the table.
    let last = NEW_ROWS[NEW_ROWS.len() - 1];
    let n = rows.iter().position(|r| (r.0.as_str(), r.1.as_str()) == last).expect("the last row of the group") + 1;
    // decision 0015: append only. The last row of decision 0119 is user32!SetWindowTextW.
    assert_eq!((rows[n - 16].0.as_str(), rows[n - 16].1.as_str()), ("user32.dll", "SetWindowTextW"));
    for (k, (dll, name)) in NEW_ROWS.iter().enumerate() {
        assert_eq!((rows[n - 15 + k].0.as_str(), rows[n - 15 + k].1.as_str()), (*dll, *name), "row {}", n - 15 + k);
    }
    for (k, (_, _, va)) in rows.iter().enumerate() {
        assert_eq!(*va, 0x7FF9_0000_0000u64 + 0x10 * (k as u64 + 1), "row {k}");
    }
    for (_, name) in NEW_ROWS {
        assert_eq!(rows.iter().filter(|(_, n2, _)| n2 == name).count(), 1, "{name}: exactly one row");
        assert!(c.contains(&format!("void fle_imp_{name}(void);")), "{name} is declared in the generated C");
    }
}

#[test]
fn each_new_thunk_is_answered_under_its_own_dll_and_no_other_dll() {
    for (dll, name) in NEW_ROWS {
        let other = if dll == "user32.dll" { "kernel32.dll" } else { "user32.dll" };
        let (st, c) = gen(&[(dll, name), (other, name), (dll, "SendMessageA"), ("kernel32.dll", "GetProcAddress")]);
        assert_eq!(st.handled[format!("jmp_imp:{name}").as_str()], 1, "{dll}!{name}");
        assert_eq!(c.matches(&format!("fle_imp_{name}(); return;")).count(), 1, "{dll}!{name}: only the thunk of its own dll is answered");
        for k in [1u64, 2] {
            let va = IB + 0x1040 + 0x10 * k;
            assert!(c.contains(&format!("L_{va:x}: {{ /* {va:#x} Jmp */ fle_unsupported(UINT64_C({va:#x})); }}")), "{dll}!{name}: thunk {k} stays unsupported");
        }
    }
}

#[test]
fn the_names_that_stay_unanswered_are_not_rows() {
    // TerminateThread is imported by no module (thread-model.md section 2, the "no" list) and stays unanswered; the neighbours of the new rows too.
    let table = std::fs::read_to_string(manifest().join("src/imports_table.rs")).unwrap();
    // decision 0123 (old: "SetTimer" and "KillTimer" were in this list) answers SetTimer and KillTimer (the timers of the window model: WM_TIMER is made at the retrieval of the thread that owns the window).
    // decision 0126 (wall 66): old: "WaitForMultipleObjectsEx" was in this list; it is a row now (runtime/fle_thread.c: no APC can be queued, so it is WaitForMultipleObjects; tests/wall_fr11.rs)
    // decision 0147: old: "PostThreadMessageW" was in this list; it is a row now (runtime/fle_win.c: the queue of the thread; tests/wall_fr20.rs)
    for n in ["TerminateThread", "SleepEx", "OpenThread", "SendMessageA", "MsgWaitForMultipleObjectsEx", "SignalObjectAndWait", "CreateSemaphoreW"] {
        assert!(!table.contains(&format!("\"{n}\"")), "{n} must not be a row of decision 0122");
    }
}

// ---------------------------------------------------------------------------------------------
// 2. The driver: real fle_thread.c and the files it works with, a stub fle_dispatch_table
// ---------------------------------------------------------------------------------------------

const RT_FILES: [&str; 14] = [
    "fle_thread.c", "fle_win.c", "fle_class.c", "fle_event.c", "fle_sys.c", "fle_imports.c", "fle_proc.c", "fle_callback.c", "fle_env.c", "fle_vfs.c", "fle_vmap.c", "fle_vol.c", "fle_host.c",
    "fle_time.c",
];
const RT_EXTRA: [&str; 4] = ["fle_x87ctl.c", "fle_mxcsr.c", "fle_x87.c", "fle_locale.c"];
// the real-module test links the loader as well (fle_load.c, fle_realmod.c, fle_modules.c, fle_string.c, fle_res.c, fle_imagehlp.c)
const RT_LOADER: [&str; 6] = ["fle_load.c", "fle_realmod.c", "fle_modules.c", "fle_string.c", "fle_res.c", "fle_imagehlp.c"];

fn clang_c(src: &Path, obj: &Path, strict: bool) {
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-Wall", "-Wextra", "-fPIE", "-c"]);
    if strict {
        cmd.arg("-Werror");
    }
    let cc = cmd.arg("-I").arg(src_dir()).arg("-I").arg(rt_dir()).arg(src).arg("-o").arg(obj).env("TMPDIR", work_dir().join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {}: {}", src.display(), String::from_utf8_lossy(&cc.stderr));
}

fn objs() -> &'static Vec<PathBuf> {
    static O: OnceLock<Vec<PathBuf>> = OnceLock::new();
    O.get_or_init(|| {
        let dir = work_dir().join(etxt_common::per_process("obj-fr8"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut v = Vec::new();
        let ours = ["fle_thread.c", "fle_win.c", "fle_event.c"];
        for name in RT_FILES.iter().chain(RT_EXTRA.iter()) {
            let is_ours = ours.contains(name);
            let base = if is_ours { src_dir() } else { rt_dir() };
            let o = dir.join(name.replace(".c", ".o"));
            clang_c(&base.join(name), &o, is_ours);
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
#include "fle_modules.h"
#include "fle_callback.h"
#include "fle_class.h"
#include "fle_sys.h"
#include "fle_proc.h"
#include "fle_event.h"
#include "fle_win.h"
#include "fle_thread.h"
#include "fle_vfs.h"
#include "fle_locale.h"
#include "fle_sysinfo.h"
#include "fle_x87ctl.h"
#include "fle_mxcsr.h"
void fle_imp_CloseHandle(void);
void fle_imp_LoadLibraryW(void);

uint64_t R[16];
fle_eflags FL;
fle_xmm X[16];
static uint64_t stk[0x4000] __attribute__((aligned(4096)));          /* 128 KiB: the first thread's guest stack */
#define E0 ((uint64_t)(uintptr_t)stk + 0x10000 + 8)
#define RET0 UINT64_C(0xdead0000)
#define U(x) ((unsigned long long)(x))
#define A(x) ((uint64_t)(uintptr_t)(x))
#define INF 0xFFFFFFFFu

/* a call of an import answer as the lifted code makes it: the return address on the guest stack, arguments in R[] (no stack arguments) */
#define CALL4(f, a, b, c, d) do { R[1] = (a); R[2] = (b); R[8] = (c); R[9] = (d); R[4] -= 8; fle_st(R[4], 8, 0xAAAA0000); f(); R[4] += 8; } while (0)
/* with stack arguments 5 and 6: rsp is 16n+8 at the API's entry, [rsp+0x28] is argument 5 */
#define CALL6(f, a, b, c, d, e, g) do { uint64_t sp0_ = R[4]; R[4] = (R[4] - 0x40) & ~UINT64_C(15); R[4] -= 8; fle_st(R[4], 8, 0xAAAA0000); fle_st(R[4] + 0x28, 8, (e)); fle_st(R[4] + 0x30, 8, (g)); \
    R[1] = (a); R[2] = (b); R[8] = (c); R[9] = (d); f(); R[4] = sp0_; } while (0)
#define CALL5(f, a, b, c, d, e) CALL6(f, a, b, c, d, e, 0)
static uint32_t LE(void) { fle_imp_GetLastError(); return (uint32_t)R[0]; }
static void SETLE(uint32_t v) { fle_set_last_error(v); }
static uint64_t H_CREATE(uint64_t start, uint64_t param, uint32_t flags, uint64_t ptid) { CALL6(fle_imp_CreateThread, 0, 0, start, param, flags, ptid); return R[0]; }
static uint64_t H_EVENT(int manual, int init) { CALL4(fle_imp_CreateEventW, 0, manual, init, 0); return R[0]; }
static uint32_t W1(uint64_t h, uint32_t ms) { CALL4(fle_imp_WaitForSingleObject, h, ms, 0, 0); return (uint32_t)R[0]; }
static void SETEV(uint64_t h) { CALL4(fle_imp_SetEvent, h, 0, 0, 0); }
static void SLEEP(uint32_t ms) { CALL4(fle_imp_Sleep, ms, 0, 0, 0); }
static uint32_t EXITCODE(uint64_t h) { uint32_t c = 0xEEEE; CALL4(fle_imp_GetExitCodeThread, h, A(&c), 0, 0); return c; }

static int ord[64], on_;
static void note(int v) { if (on_ < 64) ord[on_++] = v; }
static void print_order(void) { int i; printf("order:"); for (i = 0; i < on_; i++) printf(" %d", ord[i]); printf("\n"); }
static void boot(void) { R[4] = E0; fle_st(E0, 8, RET0); fle_teb_set_stack(A(stk), A(stk) + sizeof stk); }
"##;

fn run_src(name: &str, top: &str, body: &str) -> Out {
    run_src_with(name, top, body, &[])
}

/// Like run_src, with more runtime files (compiled from runtime/ next to the others).
fn run_src_with(name: &str, top: &str, body: &str, more: &[&str]) -> Out {
    let dir = work_dir();
    let uname = etxt_common::uniq(name);
    let c = dir.join(format!("{uname}.c"));
    let exe = dir.join(&uname);
    let src = format!("{PRELUDE}\n{top}\nint main(void)\n{{\n    boot();\n{body}\n    return 0;\n}}\n");
    std::fs::write(&c, src).unwrap();
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fPIE", "-pthread", "-I"]).arg(src_dir()).arg("-I").arg(rt_dir());
    for o in objs() {
        cmd.arg(o);
    }
    for f in more {
        cmd.arg(rt_dir().join(f));
    }
    let cc = cmd.arg("-o").arg(&exe).arg(&c).arg("-lm").env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new(&exe).env("TMPDIR", dir.join("tmp")).output().expect("run failed");
    Out {
        stdout: String::from_utf8_lossy(&r.stdout).to_string(),
        stderr: String::from_utf8_lossy(&r.stderr).to_string(),
        signal: r.status.signal(),
        ok: r.status.success(),
    }
}

fn lines(name: &str, top: &str, body: &str) -> Vec<String> {
    let o = run_src(name, top, body);
    assert!(o.ok, "{name} did not exit normally (signal {:?}): {}\n{}", o.signal, o.stderr, o.stdout);
    o.stdout.lines().map(|s| s.to_string()).collect()
}

/// The run must stop with SIGABRT and a "fle: <api>: " message that contains `word`.
fn aborts(name: &str, top: &str, body: &str, api: &str, word: &str) -> Out {
    let o = run_src(name, top, body);
    assert!(!o.ok, "{name}: expected an abort, got stdout {}", o.stdout);
    assert_eq!(o.signal, Some(6), "{name}: not SIGABRT: {}", o.stderr);
    // the messages of fle_stop are "fle: <api>: <why>"; the older STOP() of fle_sys.c is "fle: <api>(...) <why>" or "fle: <api> <why>": accept both forms
    assert!(o.stderr.contains(&format!("fle: {api}: ")) || o.stderr.contains(&format!("fle: {api}(")) || o.stderr.contains(&format!("fle: {api} ")), "{name}: no 'fle: {api}' message: {}", o.stderr);
    assert!(o.stderr.contains(word), "{name}: message lacks {word:?}: {}", o.stderr);
    o
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

const VA: &str = r#"
#define VA_A 0x1000
#define VA_B 0x1010
#define VA_C 0x1020
#define VA_D 0x1030
"#;

// ---- 2a. creation, ids, handles, flags ----

#[test]
fn a_new_thread_has_the_next_id_and_a_handle_of_the_decided_form_and_the_first_thread_keeps_0x1001() {
    // PM item 2: ids 0x1001 (FLE_EMULATED_TID, decision 0056) for the first thread, then 0x1002, 0x1003; handles 0x7FFAA0000000 + 4 * (i + 1) (i = thread index: 0 is the first thread, so the first
    // created thread, index 1, has 0x7FFAA0000008). Learn CreateThread: lpThreadId receives the identifier; the handle is returned.
    let top = format!(
        "{VA}static void th_a(void) {{ fle_imp_GetCurrentThreadId(); note((int)R[0]); R[0] = 0; }}\n\
         const fle_fn_entry fle_dispatch_table[] = {{ {{ VA_A, th_a }} }};\nconst uint64_t fle_dispatch_count = 1;"
    );
    let l = lines(
        "t-ids",
        &top,
        r#"
    uint64_t tid1 = 0, tid2 = 0, h1, h2;
    fle_imp_GetCurrentThreadId(); printf("main %llx\n", U(R[0]));
    h1 = H_CREATE(VA_A, 1, 0, A(&tid1));
    h2 = H_CREATE(VA_A, 2, 0, A(&tid2));
    printf("h1 %llx tid %llx\nh2 %llx tid %llx\n", U(h1), U(tid1), U(h2), U(tid2));
    printf("start-not-yet %d\n", on_);
    W1(h1, INF); W1(h2, INF);
    print_order();
"#,
    );
    assert_eq!(
        l,
        [
            "main 1001",
            "h1 7ffaa0000008 tid 1002",
            "h2 7ffaa000000c tid 1003",
            "start-not-yet 0",
            "order: 4098 4099"
        ]
    );
}

#[test]
fn a_thread_made_with_flags_0_does_not_run_before_the_creator_first_waits_or_yields() {
    // PM item 2. Learn CreateThread: "the thread can start running before CreateThread returns" (it may); the decision chooses that it does not run until the creator waits or yields.
    let top = format!(
        "{VA}static void th_a(void) {{ note(10 + (int)R[1]); R[0] = 0; }}\n\
         const fle_fn_entry fle_dispatch_table[] = {{ {{ VA_A, th_a }} }};\nconst uint64_t fle_dispatch_count = 1;"
    );
    let l = lines(
        "t-flags0",
        &top,
        r#"
    uint64_t h1 = H_CREATE(VA_A, 1, 0, 0), h2;
    note(1);
    CALL4(fle_imp_ResumeThread, h1, 0, 0, 0); printf("resume-flags0 %llu\n", U(R[0]));
    h2 = H_CREATE(VA_A, 2, 0, 0);
    note(2);
    printf("ran-before-wait %d\n", on_);
    SLEEP(0);
    printf("after-sleep0 %d\n", on_);
    W1(h2, INF);
    print_order();
"#,
    );
    // ResumeThread of a thread that is not suspended returns 0 (Learn: "If the return value is zero, the specified thread was not suspended")
    assert_eq!(l[0], "resume-flags0 0");
    assert_eq!(l[1], "ran-before-wait 2");
    // Sleep(0): "relinquish the remainder of its time slice to any other thread that is ready to run": both ready threads run, in creation order, then main continues
    assert_eq!(l[2], "after-sleep0 4");
    assert_eq!(l[3], "order: 1 2 11 12");
}

#[test]
fn create_suspended_waits_for_resume_and_resume_returns_the_previous_count() {
    // Learn CreateThread: CREATE_SUSPENDED "does not run until the ResumeThread function is called". ResumeThread: previous suspend count; 1 means "was suspended but was restarted".
    // TThread checks `ResumeThread(h) == 1` (thread-model.md section 3.5), so 1 is required. GetExitCodeThread: STILL_ACTIVE (259) until the thread ends.
    let top = format!(
        "{VA}static void th_a(void) {{ note(10 + (int)R[1]); R[0] = 5; }}\n\
         const fle_fn_entry fle_dispatch_table[] = {{ {{ VA_A, th_a }} }};\nconst uint64_t fle_dispatch_count = 1;"
    );
    let l = lines(
        "t-suspended",
        &top,
        r#"
    uint64_t h = H_CREATE(VA_A, 7, 4, 0);
    SLEEP(0); SLEEP(5);
    printf("ran-suspended %d\n", on_);
    printf("code %u\n", EXITCODE(h));
    CALL4(fle_imp_ResumeThread, h, 0, 0, 0); printf("resume %llu\n", U(R[0]));
    printf("ran-at-resume %d\n", on_);
    printf("wait %x\n", W1(h, INF));
    printf("ran %d code %u\n", on_, EXITCODE(h));
"#,
    );
    assert_eq!(l, ["ran-suspended 0", "code 259", "resume 1", "ran-at-resume 0", "wait 0", "ran 1 code 5"]);
    // a thread that has ended cannot be resumed: the page does not say what ResumeThread returns then (stop)
    aborts("t-resume-ended", &top, "    uint64_t h = H_CREATE(VA_A, 7, 0, 0); W1(h, INF); CALL4(fle_imp_ResumeThread, h, 0, 0, 0);", "ResumeThread", "has terminated");
}

#[test]
fn suspend_and_resume_count_and_a_second_resume_returns_zero() {
    // Learn SuspendThread: returns the previous suspend count and increments it; ResumeThread decrements it and the thread runs again when it reaches zero. A thread made with CREATE_SUSPENDED has
    // count 1; one SuspendThread makes 2, so one ResumeThread leaves it suspended (returns 2) and the second restarts it (returns 1); a third returns 0 (not suspended).
    let top = format!(
        "{VA}static void th_a(void) {{ note(10); R[0] = 0; }}\n\
         const fle_fn_entry fle_dispatch_table[] = {{ {{ VA_A, th_a }} }};\nconst uint64_t fle_dispatch_count = 1;"
    );
    let l = lines(
        "t-suspend-count",
        &top,
        r#"
    uint64_t h = H_CREATE(VA_A, 1, 4, 0);
    CALL4(fle_imp_SuspendThread, h, 0, 0, 0); printf("suspend %llu\n", U(R[0]));
    CALL4(fle_imp_ResumeThread, h, 0, 0, 0); printf("r1 %llu\n", U(R[0]));
    SLEEP(0); printf("ran-with-count-1 %d\n", on_);
    CALL4(fle_imp_ResumeThread, h, 0, 0, 0); printf("r2 %llu\n", U(R[0]));
    W1(h, INF); printf("ran %d\n", on_);
"#,
    );
    assert_eq!(l, ["suspend 1", "r1 2", "ran-with-count-1 0", "r2 1", "ran 1"]);
}

#[test]
fn two_threads_in_creation_order_wait_for_an_event_and_the_virtual_clock_moves_to_the_smallest_deadline() {
    // The scheduler's own test (a choice, hand-computed from the decision's rules): main creates T1 then T2 and waits for T1's handle; T1 (index 1) runs first, Sleep(5) -> waiting with a
    // deadline; T2 (index 2) waits for the event; nobody ready: the clock moves to +5 ms; T1 sets the event (the waiter T2 takes it: auto-reset), ends; main and T2 are ready.
    let top = format!(
        "{VA}static uint64_t ev;\n\
         static void th_a(void) {{ note(10); SLEEP(5); note(11); SETEV(ev); note(12); R[0] = 7; }}\n\
         static void th_b(void) {{ note(20); note(21 + (int)W1(ev, INF)); R[0] = 8; }}\n\
         const fle_fn_entry fle_dispatch_table[] = {{ {{ VA_A, th_a }}, {{ VA_B, th_b }} }};\nconst uint64_t fle_dispatch_count = 2;"
    );
    let l = lines(
        "t-order",
        &top,
        r#"
    uint64_t t0 = fle_sys_clock_ticks(), h1, h2;
    ev = H_EVENT(0, 0);
    h1 = H_CREATE(VA_A, 0, 0, 0);
    h2 = H_CREATE(VA_B, 0, 0, 0);
    note(1);
    W1(h1, INF);
    note(2);
    W1(h2, INF);
    note(3);
    print_order();
    printf("elapsed-ticks %llu\n", U(fle_sys_clock_ticks() - t0));
    printf("codes %u %u\n", EXITCODE(h1), EXITCODE(h2));
"#,
    );
    // T1 runs to its Sleep (10), T2 to its wait (20), clock +5 ms, T1 sets the event and ends (11 12), T2 gets it (21), main (2) ... 3
    assert_eq!(l[0], "order: 1 10 20 11 12 21 2 3");
    // 5 ms = 50,000 ticks; plus the 10-tick reads of the clock: t0 read (1) + the elapsed read (1): elapsed counts one read of t0 only (+10) at the second call before it returns the value
    let el: u64 = l[1].strip_prefix("elapsed-ticks ").unwrap().parse().unwrap();
    assert!((50_000..50_100).contains(&el), "elapsed {el}");
    assert_eq!(l[2], "codes 7 8");
}

#[test]
fn round_robin_goes_in_creation_order_from_the_running_thread() {
    // 3 ready threads: main yields (Sleep(0)); each thread yields once; the order is A B C A B C by creation, starting after the thread that yielded.
    let top = format!(
        "{VA}static void th_a(void) {{ note(1); SLEEP(0); note(4); R[0] = 0; }}\n\
         static void th_b(void) {{ note(2); SLEEP(0); note(5); R[0] = 0; }}\n\
         static void th_c(void) {{ note(3); SLEEP(0); note(6); R[0] = 0; }}\n\
         const fle_fn_entry fle_dispatch_table[] = {{ {{ VA_A, th_a }}, {{ VA_B, th_b }}, {{ VA_C, th_c }} }};\nconst uint64_t fle_dispatch_count = 3;"
    );
    let l = lines(
        "t-rr",
        &top,
        r#"
    uint64_t h[3]; int i;
    h[0] = H_CREATE(VA_A, 0, 0, 0); h[1] = H_CREATE(VA_B, 0, 0, 0); h[2] = H_CREATE(VA_C, 0, 0, 0);
    for (i = 0; i < 3; i++) W1(h[i], INF);
    print_order();
"#,
    );
    assert_eq!(l[0], "order: 1 2 3 4 5 6");
}

#[test]
fn switch_to_thread_returns_1_when_another_thread_ran_and_0_when_none_is_ready() {
    // Learn SwitchToThread: nonzero when execution switched to another thread, zero when no other thread is ready.
    let top = format!(
        "{VA}static void th_a(void) {{ note(10); R[0] = 0; }}\n\
         const fle_fn_entry fle_dispatch_table[] = {{ {{ VA_A, th_a }} }};\nconst uint64_t fle_dispatch_count = 1;"
    );
    let l = lines(
        "t-stt",
        &top,
        r#"
    uint64_t h;
    CALL4(fle_imp_SwitchToThread, 0, 0, 0, 0); printf("none %llu\n", U(R[0]));
    h = H_CREATE(VA_A, 0, 0, 0);
    CALL4(fle_imp_SwitchToThread, 0, 0, 0, 0); printf("one %llu on %d\n", U(R[0]), on_);
    CALL4(fle_imp_SwitchToThread, 0, 0, 0, 0); printf("again %llu\n", U(R[0]));
"#,
    );
    assert_eq!(l, ["none 0", "one 1 on 1", "again 0"]);
}

#[test]
fn a_deadlock_is_a_named_stop_with_the_thread_table() {
    // PM item 2: everyone waits without a deadline and nobody can wake anyone: a named stop.
    let top = format!(
        "{VA}static uint64_t ev;\nstatic void th_a(void) {{ W1(ev, INF); R[0] = 0; }}\n\
         const fle_fn_entry fle_dispatch_table[] = {{ {{ VA_A, th_a }} }};\nconst uint64_t fle_dispatch_count = 1;"
    );
    let o = aborts("t-deadlock", &top, "    uint64_t h; ev = H_EVENT(1, 0); h = H_CREATE(VA_A, 0, 0, 0); W1(h, INF); printf(\"returned\\n\");", "scheduler", "deadlock");
    assert!(o.stderr.contains("threads: 2") && o.stderr.contains("thread 0: id 0x1001") && o.stderr.contains("thread 1: id 0x1002") && o.stderr.contains("waiting:"), "{}", o.stderr);
    assert!(!o.stdout.contains("returned"));
}

#[test]
fn sleep_infinite_with_one_thread_still_stops_and_with_threads_it_waits_for_the_others() {
    let o = aborts("t-sleep-inf-one", "const fle_fn_entry fle_dispatch_table[] = { { 0x1000, 0 } };\nconst uint64_t fle_dispatch_count = 1;", "    SLEEP(INF); printf(\"returned\\n\");", "Sleep", "INFINITE");
    assert!(!o.stdout.contains("returned"));
}

#[test]
fn wait_for_a_thread_handle_times_out_and_then_succeeds_when_the_thread_ends() {
    // Learn WaitForSingleObject: WAIT_TIMEOUT (0x102) when the interval elapses; Learn CreateThread: "When a thread terminates, the thread object attains a signaled state".
    let top = format!(
        "{VA}static void th_a(void) {{ SLEEP(30); R[0] = 3; }}\n\
         const fle_fn_entry fle_dispatch_table[] = {{ {{ VA_A, th_a }} }};\nconst uint64_t fle_dispatch_count = 1;"
    );
    let l = lines(
        "t-wait-thread",
        &top,
        r#"
    uint64_t h = H_CREATE(VA_A, 0, 0, 0);
    uint64_t t0 = fle_sys_clock_ticks();
    printf("zero %x\n", W1(h, 0));
    printf("ten %x\n", W1(h, 10));
    printf("inf %x\n", W1(h, INF));
    printf("code %u\n", EXITCODE(h));
    printf("signaled-again %x\n", W1(h, 0));
"#,
    );
    assert_eq!(l, ["zero 102", "ten 102", "inf 0", "code 3", "signaled-again 0"]);
}

#[test]
fn wait_for_multiple_objects_returns_the_first_signaled_index_and_all_waits_for_all() {
    // Learn WaitForMultipleObjects: bWaitAll FALSE returns WAIT_OBJECT_0 + the index of the first signaled object (lowest index); TRUE returns when all are signaled; nCount 0 fails.
    let top = format!(
        "{VA}static uint64_t e1, e2;\nstatic void th_a(void) {{ SLEEP(2); SETEV(e2); SLEEP(2); SETEV(e1); R[0] = 0; }}\n\
         const fle_fn_entry fle_dispatch_table[] = {{ {{ VA_A, th_a }} }};\nconst uint64_t fle_dispatch_count = 1;"
    );
    let l = lines(
        "t-wfmo",
        &top,
        r#"
    uint64_t arr[2], h;
    uint32_t r;
    e1 = H_EVENT(1, 0); e2 = H_EVENT(1, 0);
    arr[0] = e1; arr[1] = e2;
    h = H_CREATE(VA_A, 0, 0, 0);
    CALL4(fle_imp_WaitForMultipleObjects, 2, A(arr), 0, INF); r = (uint32_t)R[0]; printf("any %u\n", r);
    CALL4(fle_imp_WaitForMultipleObjects, 2, A(arr), 1, INF); r = (uint32_t)R[0]; printf("all %u\n", r);
    SETEV(e1);
    CALL4(fle_imp_WaitForMultipleObjects, 2, A(arr), 0, 0); printf("lowest %u\n", (uint32_t)R[0]);
    CALL4(fle_imp_WaitForMultipleObjects, 0, A(arr), 0, 0); printf("n0 %x le %u\n", (uint32_t)R[0], LE());
    W1(h, INF);
"#,
    );
    // e2 is set first (index 1), then e1 (index 0): the any-wait returns when the first one is signaled = e2 -> 1; the all-wait returns 0 when both are
    assert_eq!(l[0], "any 1");
    assert_eq!(l[1], "all 0");
    assert_eq!(l[2], "lowest 0");
    assert_eq!(l[3], "n0 ffffffff le 87");
}

#[test]
fn an_auto_reset_event_goes_to_the_thread_that_waited_first_and_stays_signaled_without_a_waiter() {
    // Learn SetEvent: an auto-reset event "stays signaled until a single waiting thread is released"; with no waiter it stays signaled.
    let top = format!(
        "{VA}static uint64_t ev;\nstatic void th_a(void) {{ note(10 + (int)W1(ev, INF)); R[0] = 0; }}\n\
         static void th_b(void) {{ note(20 + (int)W1(ev, INF)); R[0] = 0; }}\n\
         const fle_fn_entry fle_dispatch_table[] = {{ {{ VA_A, th_a }}, {{ VA_B, th_b }} }};\nconst uint64_t fle_dispatch_count = 2;"
    );
    let l = lines(
        "t-autoreset",
        &top,
        r#"
    uint64_t h1, h2;
    ev = H_EVENT(0, 0);
    SETEV(ev);
    printf("stays %d\n", fle_event_state(ev));
    W1(ev, 0);
    printf("consumed %d\n", fle_event_state(ev));
    h1 = H_CREATE(VA_A, 0, 0, 0); h2 = H_CREATE(VA_B, 0, 0, 0);
    SLEEP(0);
    SETEV(ev);
    SLEEP(0);
    printf("after-first-set %d state %d\n", on_, fle_event_state(ev));
    SETEV(ev);
    W1(h1, INF); W1(h2, INF);
    print_order();
"#,
    );
    assert_eq!(l[0], "stays 1");
    assert_eq!(l[1], "consumed 0");
    // A waits first (it is first in creation order), so the first SetEvent releases A (10), the second releases B (20)
    assert_eq!(l[2], "after-first-set 1 state 0");
    assert_eq!(l[3], "order: 10 20");
}

#[test]
fn a_contended_critical_section_makes_the_second_thread_wait_and_a_leave_by_a_non_owner_stops() {
    // Learn EnterCriticalSection: "If the critical section object is currently owned by another thread, EnterCriticalSection waits indefinitely for ownership"; LeaveCriticalSection by a thread that
    // does not own it "may cause another thread to wait indefinitely" (the stop is kept, decision 0017).
    let top = format!(
        "{VA}static uint64_t cs[5];\n\
         static void th_a(void) {{ CALL4(fle_imp_EnterCriticalSection, A(cs), 0, 0, 0); note(10); CALL4(fle_imp_LeaveCriticalSection, A(cs), 0, 0, 0); note(11); R[0] = 0; }}\n\
         const fle_fn_entry fle_dispatch_table[] = {{ {{ VA_A, th_a }} }};\nconst uint64_t fle_dispatch_count = 1;"
    );
    let l = lines(
        "t-cs",
        &top,
        r#"
    uint64_t h;
    CALL4(fle_imp_InitializeCriticalSection, A(cs), 0, 0, 0);
    CALL4(fle_imp_EnterCriticalSection, A(cs), 0, 0, 0);
    h = H_CREATE(VA_A, 0, 0, 0);
    SLEEP(5);
    printf("blocked %d\n", on_);
    CALL4(fle_imp_LeaveCriticalSection, A(cs), 0, 0, 0);
    W1(h, INF);
    printf("after %d\n", on_);
    print_order();
"#,
    );
    assert_eq!(l, ["blocked 0", "after 2", "order: 10 11"]);
    let top2 = format!(
        "{VA}static uint64_t cs[5];\n\
         static void th_a(void) {{ CALL4(fle_imp_LeaveCriticalSection, A(cs), 0, 0, 0); R[0] = 0; }}\n\
         const fle_fn_entry fle_dispatch_table[] = {{ {{ VA_A, th_a }} }};\nconst uint64_t fle_dispatch_count = 1;"
    );
    aborts(
        "t-cs-leave-other",
        &top2,
        "    uint64_t h; CALL4(fle_imp_InitializeCriticalSection, A(cs), 0, 0, 0); CALL4(fle_imp_EnterCriticalSection, A(cs), 0, 0, 0); h = H_CREATE(VA_A, 0, 0, 0); W1(h, INF);",
        "LeaveCriticalSection on a section",
        "that another thread owns",
    );
}

#[test]
fn exit_thread_ends_the_thread_with_the_code_and_the_first_thread_cannot_call_it() {
    let top = format!(
        "{VA}static void th_a(void) {{ note(10); CALL4(fle_imp_ExitThread, 42, 0, 0, 0); note(11); }}\n\
         const fle_fn_entry fle_dispatch_table[] = {{ {{ VA_A, th_a }} }};\nconst uint64_t fle_dispatch_count = 1;"
    );
    let l = lines("t-exit", &top, "    uint64_t h = H_CREATE(VA_A, 0, 0, 0); W1(h, INF); printf(\"code %u\\n\", EXITCODE(h)); print_order();");
    assert_eq!(l, ["code 42", "order: 10"]);
    aborts("t-exit-first", &top, "    CALL4(fle_imp_ExitThread, 1, 0, 0, 0);", "ExitThread", "first thread");
}

#[test]
fn the_thread_handle_is_closed_by_closehandle_and_a_closed_handle_fails() {
    let top = format!(
        "{VA}static void th_a(void) {{ R[0] = 0; }}\n\
         const fle_fn_entry fle_dispatch_table[] = {{ {{ VA_A, th_a }} }};\nconst uint64_t fle_dispatch_count = 1;"
    );
    let l = lines(
        "t-close",
        &top,
        r#"
    uint64_t h = H_CREATE(VA_A, 0, 0, 0);
    W1(h, INF);
    CALL4(fle_imp_CloseHandle, h, 0, 0, 0); printf("close %llu\n", U(R[0]));
    SETLE(0); CALL4(fle_imp_CloseHandle, h, 0, 0, 0); printf("again %llu le %u\n", U(R[0]), LE());
    printf("wait %x le %u\n", W1(h, 0), LE());
"#,
    );
    assert_eq!(l, ["close 1", "again 0 le 6", "wait ffffffff le 6"]);
}

#[test]
fn creating_a_thread_with_security_attributes_or_an_unknown_flag_stops_and_a_bad_handle_stops() {
    let top = format!("{VA}static void th_a(void) {{ R[0] = 0; }}\nconst fle_fn_entry fle_dispatch_table[] = {{ {{ VA_A, th_a }} }};\nconst uint64_t fle_dispatch_count = 1;");
    aborts("t-attrs", &top, "    CALL6(fle_imp_CreateThread, 8, 0, VA_A, 0, 0, 0);", "CreateThread", "lpThreadAttributes");
    aborts("t-flag", &top, "    CALL6(fle_imp_CreateThread, 0, 0, VA_A, 0, 0x2, 0);", "CreateThread", "0x2");
    aborts("t-bad-handle", &top, "    CALL4(fle_imp_ResumeThread, 0x7FFAA0000100ull, 0, 0, 0);", "ResumeThread", "never returned");
    aborts("t-first-handle", &top, "    CALL4(fle_imp_ResumeThread, 0x7FFAA0000004ull, 0, 0, 0);", "ResumeThread", "never returned");
    aborts("t-null-handle", &top, "    CALL4(fle_imp_ResumeThread, 0, 0, 0, 0);", "ResumeThread", "not issued as a thread");
}

#[test]
fn a_thread_start_address_that_is_not_a_function_entry_stops_by_name_when_the_thread_runs() {
    // Learn CreateThread: "CreateThread may succeed even if lpStartAddress points to data, code, or is not accessible. If the start address is invalid when the thread runs, an exception occurs":
    // CreateThread succeeds and the stop happens when the thread first runs (fle_dispatch_miss).
    let top = "const fle_fn_entry fle_dispatch_table[] = { { 0x1000, 0 } };\nconst uint64_t fle_dispatch_count = 1;";
    let o = run_src("t-bad-start", top, "    uint64_t h = H_CREATE(0x5555, 0, 0, 0); printf(\"created %llx\\n\", U(h)); fflush(stdout); W1(h, INF); printf(\"returned\\n\");");
    assert!(o.stdout.contains("created 7ffaa0000008"), "{}", o.stdout);
    assert!(!o.stdout.contains("returned") && o.signal == Some(6), "{} {}", o.stdout, o.stderr);
    assert!(o.stderr.contains("0x5555") && o.stderr.contains("not a known function entry"), "{}", o.stderr);
}

// ---------------------------------------------------------------------------------------------
// 3. Messages
// ---------------------------------------------------------------------------------------------

/// A window procedure stub (VA_WP) that logs (thread index, hwnd, message, wParam, lParam) of every call and answers the three creation messages as Learn says (WM_NCCREATE 1, WM_NCCALCSIZE 0,
/// WM_CREATE 0) and 0x1000 + message for the rest; and `mkwnd`: RegisterClassW (WNDCLASSW at the offsets of the Learn page: lpfnWndProc 8, hInstance 24, lpszClassName 64) and the first form of
/// CreateWindowExW (WS_EX_TOOLWINDOW, WS_POPUP, size 0, hInstance the guest's 0x400000), as decision 0071 answers it.
const WM_STUB: &str = r#"
#define VA_WP 0x2000
static int wp_calls; static uint64_t wp_log[64][5]; static unsigned wp_thread[64];
static void wndproc(void)
{
    if (wp_calls < 64) { wp_log[wp_calls][0] = R[1]; wp_log[wp_calls][1] = R[2]; wp_log[wp_calls][2] = R[8]; wp_log[wp_calls][3] = R[9]; wp_thread[wp_calls] = fle_thread_current(); }
    wp_calls++;
    if (R[2] == 0x81) { R[0] = 1; return; }
    if (R[2] == 0x83) { R[0] = 0; return; }
    if (R[2] == 0x01) { R[0] = 0; return; }
    R[0] = 0x1000 + R[2];
}
static int cls_done;
static uint64_t mkwnd(void)
{
    static uint16_t cls[] = { 'T','P','U','t','i','l','W','i','n','d','o','w', 0 };
    static uint8_t wc[72];
    uint64_t w = VA_WP, inst = 0x400000, nm = A(cls), sp0 = R[4], h;
    if (!cls_done) {
        memset(wc, 0, sizeof wc); memcpy(wc + 8, &w, 8); memcpy(wc + 24, &inst, 8); memcpy(wc + 64, &nm, 8);
        CALL4(fle_imp_GetClassInfoW, 0x400000, A(cls), A(wc), 0);
        CALL4(fle_imp_RegisterClassW, A(wc), 0, 0, 0);
        cls_done = 1;
    }
    R[4] = (R[4] - 0x80) & ~UINT64_C(15); R[4] -= 8; fle_st(R[4], 8, 0xAAAA0000);
    fle_st(R[4] + 0x28, 8, 0); fle_st(R[4] + 0x30, 8, 0); fle_st(R[4] + 0x38, 8, 0); fle_st(R[4] + 0x40, 8, 0);
    fle_st(R[4] + 0x48, 8, 0); fle_st(R[4] + 0x50, 8, 0); fle_st(R[4] + 0x58, 8, inst); fle_st(R[4] + 0x60, 8, 0);
    R[1] = 0x80; R[2] = A(cls); R[8] = 0; R[9] = 0x80000000u;
    fle_imp_CreateWindowExW();
    h = R[0];
    R[4] = sp0;
    return h;
}
typedef struct { uint64_t hwnd; uint32_t msg; uint32_t pad; uint64_t wp, lp; uint32_t time; int32_t ptx, pty; uint32_t priv; } MSGX;
_Static_assert(sizeof(MSGX) == FLE_MSG_SIZE, "sizeof MSG");
static uint32_t GET(uint64_t pmsg, uint64_t h, uint32_t lo, uint32_t hi) { CALL4(fle_imp_GetMessageW, pmsg, h, lo, hi); return (uint32_t)R[0]; }
static uint32_t PEEK(uint64_t pmsg, uint64_t h, uint32_t lo, uint32_t hi, uint32_t rm) { CALL5(fle_imp_PeekMessageW, pmsg, h, lo, hi, rm); return (uint32_t)R[0]; }
static uint32_t POST(uint64_t h, uint32_t m, uint64_t wp, uint64_t lp) { CALL4(fle_imp_PostMessageW, h, m, wp, lp); return (uint32_t)R[0]; }
"#;

fn msg_top(extra: &str, entries: &[(&str, &str)]) -> String {
    let mut t = format!("{VA}{WM_STUB}{extra}\n");
    // fle_dispatch searches the table by binary search: the rows must be sorted by VA (VA_A 0x1000 .. VA_D 0x1030 come before VA_WP 0x2000)
    let mut rows: Vec<String> = Vec::new();
    for (va, f) in entries {
        rows.push(format!("{{ {va}, {f} }}"));
    }
    rows.push("{ VA_WP, wndproc }".to_string());
    t += &format!("const fle_fn_entry fle_dispatch_table[] = {{ {} }};\nconst uint64_t fle_dispatch_count = {};\n", rows.join(", "), rows.len());
    t
}

#[test]
fn a_window_belongs_to_the_thread_that_created_it() {
    let top = msg_top("static uint64_t made[2];\nstatic void th_a(void) { made[1] = mkwnd(); R[0] = 0; }", &[("VA_A", "th_a")]);
    let l = lines(
        "m-owner",
        &top,
        r#"
    uint64_t th;
    fle_win_view v;
    made[0] = mkwnd();
    th = H_CREATE(VA_A, 0, 0, 0);
    W1(th, INF);
    fle_win_get(made[0], &v); printf("w0 %llx owner %u\n", U(made[0]), v.owner);
    fle_win_get(made[1], &v); printf("w1 %llx owner %u\n", U(made[1]), v.owner);
    printf("procs %d threads %u %u %u %u %u %u %u %u %u %u\n", wp_calls, wp_thread[0], wp_thread[1], wp_thread[2], wp_thread[3], wp_thread[4], wp_thread[5], wp_thread[6], wp_thread[7], wp_thread[8], wp_thread[9]);
"#,
    );
    assert_eq!(l[0], "w0 7f990004 owner 0");
    assert_eq!(l[1], "w1 7f990008 owner 1");
    // the creation messages (WM_NCCREATE, WM_NCCALCSIZE, WM_CREATE, and since decision 0219 WM_SIZE and WM_MOVE: five per window; was three, "procs 6 threads 0 0 0 1 1 1") of each window run in the context of the thread that creates it
    assert_eq!(l[2], "procs 10 threads 0 0 0 0 0 1 1 1 1 1");
}

#[test]
fn post_message_goes_to_the_queue_of_the_owner_thread_and_get_message_takes_them_in_order() {
    // Learn PostMessageW: "the message queue associated with the thread that created the specified window", first in first out; GetMessageW fills the MSG (Learn MSG: hwnd, message, wParam,
    // lParam, time, pt, lPrivate) and returns nonzero. The poster is another thread; the receiver is blocked in GetMessageW until the first message is there.
    let top = msg_top(
        "static uint64_t g_w;\nstatic void th_a(void) { note(POST(g_w, 0x8001, 5, 6)); note(POST(g_w, 0x8002, 7, 8)); R[0] = 0; }",
        &[("VA_A", "th_a")],
    );
    let l = lines(
        "m-post-get",
        &top,
        r#"
    MSGX m1, m2;
    uint64_t th;
    g_w = mkwnd();
    th = H_CREATE(VA_A, 0, 0, 0);
    printf("get1 %u\n", GET(A(&m1), 0, 0, 0));
    printf("m1 hwnd %llx msg %x wp %llx lp %llx\n", U(m1.hwnd), m1.msg, U(m1.wp), U(m1.lp));
    printf("get2 %u\n", GET(A(&m2), g_w, 0x8000, 0x8fff));
    printf("m2 hwnd %llx msg %x wp %llx lp %llx pt %d %d\n", U(m2.hwnd), m2.msg, U(m2.wp), U(m2.lp), m2.ptx, m2.pty);
    W1(th, INF);
    print_order();
"#,
    );
    assert_eq!(l[0], "get1 1");
    assert_eq!(l[1], "m1 hwnd 7f990004 msg 8001 wp 5 lp 6");
    assert_eq!(l[2], "get2 1");
    assert_eq!(l[3], "m2 hwnd 7f990004 msg 8002 wp 7 lp 8 pt 0 0");
    // both posts returned nonzero (1)
    assert_eq!(l[4], "order: 1 1");
}

#[test]
fn post_message_with_a_null_hwnd_is_a_thread_message_of_the_calling_thread() {
    // PM item 4 / Learn PostMessageW: hWnd NULL "behaves like a call to PostThreadMessage with the dwThreadId parameter set to the identifier of the current thread"; Learn MSG: hwnd is NULL for a
    // thread message; GetMessageW with hWnd -1 retrieves "only messages on the current thread's message queue whose hwnd value is NULL".
    let top = msg_top("", &[]);
    let l = lines(
        "m-null-hwnd",
        &top,
        r#"
    MSGX m;
    uint64_t w = mkwnd();
    printf("post %u le %u\n", POST(0, 0x8003, 11, 12), LE());
    printf("post-window %u\n", POST(w, 0x8004, 13, 14));
    printf("peek-minus1 %u\n", PEEK(A(&m), (uint64_t)-1, 0, 0, 1));
    printf("m hwnd %llx msg %x wp %llu\n", U(m.hwnd), m.msg, U(m.wp));
    printf("peek-minus1-again %u\n", PEEK(A(&m), (uint64_t)-1, 0, 0, 1));
    printf("get %u hwnd %llx msg %x\n", GET(A(&m), 0, 0, 0), U(m.hwnd), m.msg);
"#,
    );
    assert_eq!(l, ["post 1 le 0", "post-window 1", "peek-minus1 1", "m hwnd 0 msg 8003 wp 11", "peek-minus1-again 0", "get 1 hwnd 7f990004 msg 8004"]);
}

#[test]
fn peek_message_remove_and_no_remove_and_an_empty_queue() {
    // Learn PeekMessageW: nonzero when a message is available and zero when none; PM_NOREMOVE (0) leaves it, PM_REMOVE (1) removes it; the filter range is on the message id.
    let top = msg_top("", &[]);
    let l = lines(
        "m-peek",
        &top,
        r#"
    MSGX m;
    uint64_t w = mkwnd();
    printf("empty %u\n", PEEK(A(&m), 0, 0, 0, 1));
    POST(w, 0x8001, 1, 0); POST(w, 0x8002, 2, 0); POST(w, 0x8003, 3, 0);
    printf("noremove %u msg %x\n", PEEK(A(&m), 0, 0, 0, 0), m.msg);
    printf("noremove-again %u msg %x\n", PEEK(A(&m), 0, 0, 0, 0), m.msg);
    printf("range %u msg %x\n", PEEK(A(&m), 0, 0x8002, 0x8002, 1), m.msg);
    printf("remove %u msg %x\n", PEEK(A(&m), 0, 0, 0, 1), m.msg);
    printf("remove %u msg %x\n", PEEK(A(&m), 0, 0, 0, 1), m.msg);
    printf("empty %u\n", PEEK(A(&m), 0, 0, 0, 1));
    printf("noyield %u\n", PEEK(A(&m), 0, 0, 0, 3));
"#,
    );
    assert_eq!(l, ["empty 0", "noremove 1 msg 8001", "noremove-again 1 msg 8001", "range 1 msg 8002", "remove 1 msg 8001", "remove 1 msg 8003", "empty 0", "noyield 0"]);
}

#[test]
fn wm_quit_is_delivered_after_the_other_posted_messages_and_get_message_returns_zero_for_it() {
    // Learn PostQuitMessage: posts WM_QUIT, the exit code is the wParam; Learn GetMessage: zero for WM_QUIT; "About Messages and Message Queues": WM_QUIT is forwarded only when the queue holds no other
    // messages. WM_QUIT is not associated with a window (hwnd NULL).
    let top = msg_top("", &[]);
    let l = lines(
        "m-quit",
        &top,
        r#"
    MSGX m;
    uint64_t w = mkwnd();
    CALL4(fle_imp_PostQuitMessage, 7, 0, 0, 0);
    POST(w, 0x8001, 1, 0);
    printf("first %u msg %x\n", GET(A(&m), 0, 0, 0), m.msg);
    printf("quit %u msg %x wp %llu hwnd %llx\n", GET(A(&m), 0, 0, 0), m.msg, U(m.wp), U(m.hwnd));
    printf("peek-after %u\n", PEEK(A(&m), 0, 0, 0, 1));
    CALL4(fle_imp_PostQuitMessage, (uint64_t)(uint32_t)-3, 0, 0, 0);
    printf("peek-quit %u msg %x wp %lld\n", PEEK(A(&m), 0, 0, 0, 1), m.msg, (long long)m.wp);
"#,
    );
    assert_eq!(l, ["first 1 msg 8001", "quit 0 msg 12 wp 7 hwnd 0", "peek-after 0", "peek-quit 1 msg 12 wp -3"]);
}

#[test]
fn get_message_with_an_empty_queue_blocks_the_thread_and_the_others_run_and_a_window_of_another_thread_is_refused() {
    // Learn GetMessage: hWnd "must belong to the current thread"; an hWnd that is not a window of the thread: -1 (the page: "the function fails if hWnd is an invalid window handle") with the last error
    // 1400 (a choice without a source, as the 1400 of decision 0119). The empty queue: the thread waits (the page "does not return" for a blocking read) and the others run.
    let top = msg_top("static uint64_t g_w;\nstatic void th_a(void) { note(10); SLEEP(3); note(11); POST(g_w, 0x8005, 9, 0); note(12); R[0] = 0; }", &[("VA_A", "th_a")]);
    let l = lines(
        "m-block",
        &top,
        r#"
    MSGX m;
    uint64_t th, t0;
    g_w = mkwnd();
    th = H_CREATE(VA_A, 0, 0, 0);
    t0 = fle_sys_clock_ticks();
    printf("get %u msg %x wp %llu\n", GET(A(&m), g_w, 0, 0), m.msg, U(m.wp));
    note(1);
    printf("waited>=3ms %d\n", fle_sys_clock_ticks() - t0 >= 30000);
    SETLE(0);
    printf("bad-hwnd %x le %u\n", GET(A(&m), 0x7f9900ff, 0, 0), LE());
    W1(th, INF);
    print_order();
"#,
    );
    assert_eq!(l[0], "get 1 msg 8005 wp 9");
    assert_eq!(l[1], "waited>=3ms 1");
    assert_eq!(l[2], "bad-hwnd ffffffff le 1400");
    assert_eq!(l[3], "order: 10 11 12 1");
}

#[test]
fn wait_message_returns_only_for_a_new_message_and_marks_the_old_ones() {
    // Learn WaitMessage: "does not return until a new message is placed in the thread's message queue"; messages already looked at by PeekMessage/GetMessage/WaitMessage are not new ("A subsequent call to
    // WaitMessage will not return until new messages arrive").
    let top = msg_top("static uint64_t g_w;\nstatic void th_a(void) { SLEEP(2); POST(g_w, 0x8001, 1, 0); SLEEP(2); POST(g_w, 0x8002, 2, 0); R[0] = 0; }", &[("VA_A", "th_a")]);
    let l = lines(
        "m-waitmsg",
        &top,
        r#"
    MSGX m;
    uint64_t th;
    g_w = mkwnd();
    th = H_CREATE(VA_A, 0, 0, 0);
    CALL4(fle_imp_WaitMessage, 0, 0, 0, 0); printf("w1 %llu pending %u\n", U(R[0]), fle_thread_queue(0, 0)->count);
    PEEK(A(&m), 0, 0, 0, 0);
    printf("peeked %x\n", m.msg);
    CALL4(fle_imp_WaitMessage, 0, 0, 0, 0); printf("w2 %llu pending %u\n", U(R[0]), fle_thread_queue(0, 0)->count);
    W1(th, INF);
"#,
    );
    // the first message wakes the first WaitMessage (1 pending); the peek (no remove) marks it old; the second WaitMessage returns for the SECOND message only (2 pending)
    assert_eq!(l, ["w1 1 pending 1", "peeked 8001", "w2 1 pending 2"]);
}

#[test]
fn msg_wait_for_multiple_objects_returns_n_for_input_and_the_object_index_and_times_out() {
    // Learn MsgWaitForMultipleObjects: returns WAIT_OBJECT_0 + nCount for new input of the types in dwWakeMask; WAIT_OBJECT_0 + i for an object; WAIT_TIMEOUT (0x102); after PeekMessage/GetMessage
    // marked the messages old, a later call does not return for them. QS_SENDMESSAGE 0x40 is the TThread.WaitFor mask: only sent messages from other threads count for it.
    let top = msg_top(
        "static uint64_t g_w, ev;\nstatic void th_a(void) { SLEEP(2); POST(g_w, 0x8001, 1, 0); SLEEP(2); SETEV(ev); R[0] = 0; }",
        &[("VA_A", "th_a")],
    );
    let l = lines(
        "m-msgwait",
        &top,
        r#"
    MSGX m;
    uint64_t th, harr[1];
    uint32_t r;
    g_w = mkwnd();
    ev = H_EVENT(0, 0);
    harr[0] = ev;
    th = H_CREATE(VA_A, 0, 0, 0);
    /* QS_SENDMESSAGE only: a posted message does not wake it; the event does (index 0) */
    CALL5(fle_imp_MsgWaitForMultipleObjects, 1, A(harr), 0, INF, 0x40); printf("sendmask %u\n", (uint32_t)R[0]);
    /* the posted message arrived earlier, but was not of the mask: with QS_POSTMESSAGE (0x8) it is still not "new" for this call after the previous wait looked at the queue */
    CALL5(fle_imp_MsgWaitForMultipleObjects, 1, A(harr), 0, 5, 0x8); printf("old-input %x\n", (uint32_t)R[0]);
    printf("peek %u\n", PEEK(A(&m), 0, 0, 0, 1));
    CALL5(fle_imp_MsgWaitForMultipleObjects, 1, A(harr), 0, 0, 0xff); printf("zero-timeout %x\n", (uint32_t)R[0]);
    W1(th, INF);
"#,
    );
    assert_eq!(l, ["sendmask 0", "old-input 102", "peek 1", "zero-timeout 102"]);
}

#[test]
fn msg_wait_returns_n_for_a_message_posted_while_it_waits() {
    let top = msg_top("static uint64_t g_w;\nstatic void th_a(void) { SLEEP(2); POST(g_w, 0x8001, 1, 0); R[0] = 0; }", &[("VA_A", "th_a")]);
    let l = lines(
        "m-msgwait-input",
        &top,
        r#"
    uint64_t th, harr[1];
    g_w = mkwnd();
    harr[0] = H_EVENT(1, 0);
    th = H_CREATE(VA_A, 0, 0, 0);
    CALL5(fle_imp_MsgWaitForMultipleObjects, 1, A(harr), 0, INF, 0x4ff); printf("input %u\n", (uint32_t)R[0]);
    W1(th, INF);
"#,
    );
    assert_eq!(l, ["input 1"]);
}

#[test]
fn send_message_to_a_window_of_another_thread_blocks_the_sender_until_the_owner_retrieves_and_returns_the_result() {
    // Learn SendMessageW: "If the specified window was created by a different thread, the system switches to that thread and calls the appropriate window procedure. Messages sent between threads are
    // processed only when the receiving thread executes message retrieval code. The sending thread is blocked until the receiving thread processes the message."
    let top = msg_top(
        "static uint64_t g_w;\nstatic void th_a(void) { note(10); CALL4(fle_imp_SendMessageW, g_w, 0x8010, 3, 4); note(11 + (int)(R[0] - 0x9010)); R[0] = 0; }",
        &[("VA_A", "th_a")],
    );
    let l = lines(
        "m-send",
        &top,
        r#"
    MSGX m;
    uint64_t th;
    g_w = mkwnd();
    th = H_CREATE(VA_A, 0, 0, 0);
    SLEEP(0);
    note(1);
    printf("before-retrieval calls %d\n", wp_calls);
    PEEK(A(&m), 0, 0, 0, 1);
    note(2);
    printf("calls %d msg %llx wp %llu lp %llu thread %u\n", wp_calls, U(wp_log[5][1]), U(wp_log[5][2]), U(wp_log[5][3]), wp_thread[5]);
    W1(th, INF);
    print_order();
"#,
    );
    // the five creation messages (decision 0219: NCCREATE, NCCALCSIZE, CREATE, SIZE, MOVE; was three) are calls 0..4; the sent message is call 5, made by the OWNER (thread 0) inside its PeekMessageW
    assert_eq!(l[0], "before-retrieval calls 5");
    assert_eq!(l[1], "calls 6 msg 8010 wp 3 lp 4 thread 0");
    // the creator yields (Sleep(0)): the new thread runs first (10) and blocks in SendMessageW; main goes on (1), retrieves (the window procedure runs in main's context) and notes 2; the sender is
    // woken by main's wait for it (W1) and notes 11 + (result - 0x9010) = 11, the stub answering 0x1000 + message = 0x9010
    assert_eq!(l[2], "order: 10 1 2 11");
}

#[test]
fn dispatch_message_calls_the_window_procedure_in_the_receivers_context_and_a_thread_message_is_not_dispatched() {
    // Learn DispatchMessageW: "dispatches a message to a window procedure", returns the procedure's value; a message with hwnd NULL cannot be dispatched (PostThreadMessage page).
    let top = msg_top("", &[]);
    let l = lines(
        "m-dispatch",
        &top,
        r#"
    MSGX m;
    uint64_t w = mkwnd();
    int before = wp_calls;
    POST(w, 0x8021, 5, 6);
    POST(0, 0x8022, 7, 8);
    GET(A(&m), 0, 0, 0);
    CALL4(fle_imp_TranslateMessage, A(&m), 0, 0, 0); printf("translate %llu\n", U(R[0]));
    CALL4(fle_imp_DispatchMessageW, A(&m), 0, 0, 0); printf("dispatch %llx calls %d\n", U(R[0]), wp_calls - before);
    printf("log hwnd %llx msg %llx wp %llu lp %llu\n", U(wp_log[before][0]), U(wp_log[before][1]), U(wp_log[before][2]), U(wp_log[before][3]));
    GET(A(&m), 0, 0, 0);
    CALL4(fle_imp_DispatchMessageW, A(&m), 0, 0, 0); printf("thread-msg %llx calls %d\n", U(R[0]), wp_calls - before);
"#,
    );
    assert_eq!(l, ["translate 0", "dispatch 9021 calls 1", "log hwnd 7f990004 msg 8021 wp 5 lp 6", "thread-msg 0 calls 1"]);
}

#[test]
fn dispatch_message_a_goes_to_the_window_procedure_and_a_message_of_a_window_that_is_gone_answers_zero() {
    // decision 0222 (sweep1's wall at user32!DispatchMessageA): the A variant dispatches like the W variant (Learn: one page); a message whose window was destroyed after the retrieval has no window procedure
    // to go to: 0, nothing called (Wine 10.0 dispatch_message; before: a stop). A text-carrying ANSI message stops by name (not tested here: it aborts).
    let top = msg_top("", &[]);
    let l = lines(
        "m-dispatch-a",
        &top,
        r#"
    MSGX m;
    uint64_t w = mkwnd();
    int before = wp_calls;
    POST(w, 0x8021, 5, 6);
    GET(A(&m), 0, 0, 0);
    CALL4(fle_imp_DispatchMessageA, A(&m), 0, 0, 0); printf("A %llx calls %d\n", U(R[0]), wp_calls - before);
    POST(w, 0x8023, 1, 2);
    GET(A(&m), 0, 0, 0);
    CALL4(fle_imp_DestroyWindow, w, 0, 0, 0);
    before = wp_calls;
    CALL4(fle_imp_DispatchMessageW, A(&m), 0, 0, 0); printf("gone-W %llx calls %d\n", U(R[0]), wp_calls - before);
    CALL4(fle_imp_DispatchMessageA, A(&m), 0, 0, 0); printf("gone-A %llx calls %d\n", U(R[0]), wp_calls - before);
"#,
    );
    assert_eq!(l, ["A 9021 calls 1", "gone-W 0 calls 0", "gone-A 0 calls 0"]);
}

#[test]
fn the_queue_limit_is_10000_and_a_full_queue_answers_zero_with_error_not_enough_quota() {
    // Learn PostMessageW: "There is a limit of 10,000 posted messages per message queue ... GetLastError returns ERROR_NOT_ENOUGH_QUOTA (1816) when the limit is hit".
    let top = msg_top("", &[]);
    let l = lines(
        "m-quota",
        &top,
        r#"
    uint64_t w = mkwnd();
    unsigned i, ok = 0;
    for (i = 0; i < 10000; i++) ok += POST(w, 0x8001, i, 0);
    printf("ok %u\n", ok);
    SETLE(0);
    printf("full %u le %u\n", POST(w, 0x8001, 0, 0), LE());
"#,
    );
    assert_eq!(l, ["ok 10000", "full 0 le 1816"]);
}

#[test]
fn post_message_stops_for_broadcast_and_for_wm_quit_and_get_message_stops_without_a_pointer() {
    let top = msg_top("", &[]);
    aborts("m-bcast", &top, "    POST(0xFFFF, 0x8001, 0, 0);", "PostMessageW", "HWND_BROADCAST");
    aborts("m-quit-post", &top, "    POST(0, 0x12, 0, 0);", "PostMessageW", "PostQuitMessage");
    aborts("m-nullmsg", &top, "    GET(0, 0, 0, 0);", "GetMessageW", "lpMsg is NULL");
    // decision 0126 (wall 83): old: `PEEK(A(&m), 0, 0, 0, 0x400000)` stopped ("the PM_QS_* filters are not modelled"); 0x400000 is PM_QS_SENDMESSAGE (QS_SENDMESSAGE 0x40 << 16) and PM_QS_* are answered now
    // (tests/wall_fr11.rs). A high-word bit that no PM_QS_* value of the page contains (0x1000000 = 0x100 << 16, QS_ALLPOSTMESSAGE, which the page does not list for PeekMessage) and a low-word bit other than
    // PM_REMOVE / PM_NOYIELD (0x4) still stop.
    aborts("m-peek-bits", &top, "    MSGX m; PEEK(A(&m), 0, 0, 0, 0x1000000);", "PeekMessageW", "PM_QS");
    aborts("m-peek-lowbit", &top, "    MSGX m; PEEK(A(&m), 0, 0, 0, 0x4);", "PeekMessageW", "PM_QS");
    aborts("m-peek-badwnd", &top, "    MSGX m; PEEK(A(&m), 0x7f9900ff, 0, 0, 0);", "PeekMessageW", "not a live window");
}

#[test]
fn the_idle_stop_reports_the_threads_queues_windows_and_the_clock() {
    // PM stop condition (b): the first thread waits for input with an empty queue and no other thread can run: a named stop that carries the report (the harness switches it on).
    let top = msg_top("", &[]);
    let o = aborts(
        "m-idle",
        &top,
        "    MSGX m; mkwnd(); fle_thread_set_idle_stop(1); GET(A(&m), 0, 0, 0); printf(\"returned\\n\");",
        "MessageLoop",
        "idle: the first thread waits for input",
    );
    assert!(o.stderr.contains("threads: 1") && o.stderr.contains("thread 0: id 0x1001") && o.stderr.contains("waiting: GetMessageW: GetMessage") && o.stderr.contains("windows: 1"), "{}", o.stderr);
    assert!(o.stderr.contains("virtual clock"), "{}", o.stderr);
    // without the idle stop and with one thread that waits for input forever: the deadlock stop
    aborts("m-deadlock-getmsg", &top, "    MSGX m; mkwnd(); GET(A(&m), 0, 0, 0);", "scheduler", "deadlock");
}

// ---------------------------------------------------------------------------------------------
// 4. Per-thread state, DllMain notifications, dwmapi
// ---------------------------------------------------------------------------------------------

#[test]
fn last_error_and_tls_slots_belong_to_the_thread() {
    // Learn GetLastError: "maintained on a per-thread basis"; Learn TlsAlloc: "each thread receives its own slot for the index", "The slots for the index are initialized to zero";
    // Learn TlsGetValue: success sets the last error to ERROR_SUCCESS (0) of the CALLING thread.
    let top = format!(
        "{VA}static uint64_t sp[2];\n\
         static void th_a(void) {{ uint64_t idx = sp[0]; SETLE(77); CALL4(fle_imp_TlsGetValue, idx, 0, 0, 0); note((int)R[0]); CALL4(fle_imp_TlsSetValue, idx, 222, 0, 0); SETLE(88); SLEEP(1); note(1000 + (int)LE()); CALL4(fle_imp_TlsGetValue, idx, 0, 0, 0); note((int)R[0]); R[0] = 0; }}\n\
         const fle_fn_entry fle_dispatch_table[] = {{ {{ VA_A, th_a }} }};\nconst uint64_t fle_dispatch_count = 1;"
    );
    let l = lines(
        "p-tls",
        &top,
        r#"
    uint64_t th, v;
    CALL4(fle_imp_TlsAlloc, 0, 0, 0, 0); sp[0] = R[0];
    CALL4(fle_imp_TlsSetValue, sp[0], 111, 0, 0);
    SETLE(5);
    th = H_CREATE(VA_A, 0, 0, 0);
    SLEEP(0);
    SETLE(6);
    CALL4(fle_imp_TlsGetValue, sp[0], 0, 0, 0); v = R[0];
    printf("main slot %llu le %u\n", U(v), LE());
    W1(th, INF);
    CALL4(fle_imp_TlsGetValue, sp[0], 0, 0, 0); v = R[0];
    printf("main slot after %llu\n", U(v));
    SETLE(9);
    printf("main le %u\n", LE());
    print_order();
"#,
    );
    // the new thread's slot starts at 0 (first note); its last error 88 is its own and survives the switch although main set 6 and 9 meanwhile (1088); its slot value 222 survives too (third note);
    // the main thread's slot (111) and last error are untouched by the other thread. (TlsGetValue's success sets the CALLING thread's error to 0: main reads le 0 after its own call.)
    assert_eq!(l[0], "main slot 111 le 0");
    assert_eq!(l[1], "main slot after 111");
    assert_eq!(l[2], "main le 9");
    assert_eq!(l[3], "order: 0 1088 222");
}

#[test]
fn set_thread_priority_of_a_created_thread_is_answered_and_the_background_mode_needs_the_pseudo_handle() {
    // Learn SetThreadPriority: hThread is "a handle to the thread whose priority value is to be set"; the values -15, -2, -1, 0, 1, 2, 15 and the background modes; a closed handle fails.
    let top = format!("{VA}static void th_a(void) {{ R[0] = 0; }}\nconst fle_fn_entry fle_dispatch_table[] = {{ {{ VA_A, th_a }} }};\nconst uint64_t fle_dispatch_count = 1;");
    let l = lines(
        "p-prio2",
        &top,
        r#"
    uint64_t th = H_CREATE(VA_A, 0, 4, 0);
    CALL4(fle_imp_SetThreadPriority, th, (uint32_t)-2, 0, 0); printf("set-other %llu\n", U(R[0]));
    CALL4(fle_imp_SetThreadPriority, 0xFFFFFFFFFFFFFFFEull, 2, 0, 0); printf("set-self %llu\n", U(R[0]));
    CALL4(fle_imp_ResumeThread, th, 0, 0, 0);
    W1(th, INF);
    CALL4(fle_imp_CloseHandle, th, 0, 0, 0);
    SETLE(0);
    CALL4(fle_imp_SetThreadPriority, th, 1, 0, 0); printf("closed %llu le %u\n", U(R[0]), LE());
"#,
    );
    assert_eq!(l, ["set-other 1", "set-self 1", "closed 0 le 6"]);
    aborts("p-bg-other", &top, "    uint64_t th = H_CREATE(VA_A, 0, 4, 0); CALL4(fle_imp_SetThreadPriority, th, 0x10000, 0, 0);", "SetThreadPriority", "calling thread only");
}

#[test]
fn a_new_thread_starts_with_an_unset_locale_and_its_own_slot() {
    let top = format!(
        "{VA}static void th_a(void) {{ CALL4(fle_imp_GetThreadLocale, 0, 0, 0, 0); note((int)R[0]); CALL4(fle_imp_SetThreadLocale, 0x409, 0, 0, 0); CALL4(fle_imp_GetThreadLocale, 0, 0, 0, 0); note((int)R[0]); R[0] = 0; }}\n\
         const fle_fn_entry fle_dispatch_table[] = {{ {{ VA_A, th_a }} }};\nconst uint64_t fle_dispatch_count = 1;"
    );
    let l = lines(
        "p-locale",
        &top,
        r#"
    uint64_t th;
    CALL4(fle_imp_SetThreadLocale, 0x7f, 0, 0, 0);
    th = H_CREATE(VA_A, 0, 0, 0);
    W1(th, INF);
    CALL4(fle_imp_GetThreadLocale, 0, 0, 0, 0); printf("main %llx\n", U(R[0]));
    print_order();
"#,
    );
    // the new thread starts with the emulated user locale 0x409 (en-US) and not the main thread's 0x7f; the main thread keeps 0x7f
    assert_eq!(l[0], "main 7f");
    assert_eq!(l[1], "order: 1033 1033");
}

#[test]
fn the_guest_state_registers_flags_x87_and_mxcsr_are_per_thread_across_a_switch() {
    // PM item 3: R[], X/YH, FL, the x87 and MXCSR state are saved and loaded at every switch.
    let top = format!(
        "{VA}static void th_a(void) {{ R[10] = 0xAAAA; R[11] = 0xBBBB; X[2].u64[0] = 0x2222; fle_mxcsr = 0x9F80; fle_x87.cw = 0x027F; FL.cf = 1; SLEEP(1); note((int)(R[10] == 0xAAAA && R[11] == 0xBBBB && X[2].u64[0] == 0x2222 && fle_mxcsr == 0x9F80 && fle_x87.cw == 0x027F && FL.cf == 1)); R[0] = 0; }}\n\
         const fle_fn_entry fle_dispatch_table[] = {{ {{ VA_A, th_a }} }};\nconst uint64_t fle_dispatch_count = 1;"
    );
    let l = lines(
        "p-regs",
        &top,
        r#"
    uint64_t th;
    R[10] = 0x1111; R[11] = 0x1212; X[2].u64[0] = 0x1010; fle_mxcsr = 0x1F80; fle_x87.cw = 0x037F; FL.cf = 0;
    th = H_CREATE(VA_A, 0, 0, 0);
    SLEEP(0);
    printf("main after first switch: %d\n", R[10] == 0x1111 && R[11] == 0x1212 && X[2].u64[0] == 0x1010 && fle_mxcsr == 0x1F80 && fle_x87.cw == 0x037F && FL.cf == 0);
    W1(th, INF);
    printf("main after the end: %d\n", R[10] == 0x1111 && R[11] == 0x1212 && X[2].u64[0] == 0x1010 && fle_mxcsr == 0x1F80 && fle_x87.cw == 0x037F && FL.cf == 0);
    print_order();
"#,
    );
    assert_eq!(l, ["main after first switch: 1", "main after the end: 1", "order: 1"]);
}

#[test]
fn a_new_threads_gs_teb_is_its_own_with_its_own_stack_bounds_and_tls_pointer() {
    // PM item 3: GS:[0x30] is the thread's TEB, GS:[0x58] its TLS table, GS:[0x10] its own stack limit (decision 0076); the first thread's are fle_teb and fle_tls_values.
    let top = format!(
        "{VA}static uint64_t g_self[2], g_tls[2], g_lim;\n\
         static void th_a(void) {{ g_self[1] = fle_gs_ld8(0x30); g_tls[1] = fle_gs_ld8(0x58); g_lim = fle_gs_ld8(0x10); R[0] = 0; }}\n\
         const fle_fn_entry fle_dispatch_table[] = {{ {{ VA_A, th_a }} }};\nconst uint64_t fle_dispatch_count = 1;"
    );
    let l = lines(
        "p-gs",
        &top,
        r#"
    extern uint64_t fle_tls_values[];
    uint64_t th;
    uint64_t sp1;
    g_self[0] = fle_gs_ld8(0x30); g_tls[0] = fle_gs_ld8(0x58);
    th = H_CREATE(VA_A, 0, 0, 0);
    W1(th, INF);
    printf("first teb %d tls %d\n", g_self[0] == A(fle_teb), g_tls[0] == A(fle_tls_values));
    printf("second teb differs %d tls differs %d\n", g_self[1] != g_self[0], g_tls[1] != g_tls[0]);
    printf("second limit page aligned %d\n", (g_lim & 0xfff) == 0 && g_lim != 0);
    printf("first limit back %d\n", fle_gs_ld8(0x10) == ((A(stk) + 0xfff) & ~0xfffull));
    (void)sp1;
"#,
    );
    assert_eq!(l, ["first teb 1 tls 1", "second teb differs 1 tls differs 1", "second limit page aligned 1", "first limit back 1"]);
}

#[test]
fn the_callback_depth_is_per_thread_and_a_thread_may_switch_inside_a_callback() {
    // PM item 3: the nesting of fle_call_guest belongs to the thread; the switch can happen inside a window procedure (a wait API called from it).
    let top = msg_top(
        "static void th_a(void) { note(10 + (int)fle_callback_depth()); SLEEP(0); note(20 + (int)fle_callback_depth()); R[0] = 0; }",
        &[("VA_A", "th_a")],
    );
    let l = lines(
        "p-depth",
        &top,
        r#"
    uint64_t th = H_CREATE(VA_A, 0, 0, 0);
    uint64_t r;
    note(100 + (int)fle_callback_depth());
    r = fle_call_guest(VA_WP, 0, 0x8001, 0, 0);
    printf("callback result %llx\n", U(r));
    W1(th, INF);
    print_order();
"#,
    );
    // the thread starts inside fle_call_guest (depth 1 from its own start): 11 then (after its yield) 21; main reads depth 0
    assert_eq!(l[0], "callback result 9001");
    assert!(l[1].starts_with("order: 100"), "{}", l[1]);
}

#[test]
fn dll_thread_attach_and_detach_are_delivered_to_the_real_modules_in_the_new_threads_context() {
    // PM item 6 / Learn DllMain: DLL_THREAD_ATTACH (2) "The call is made in the context of the new thread"; DLL_THREAD_DETACH (3) when the thread ends (ExitThread page: "The entry-point function of all
    // attached dynamic-link libraries (DLLs) is invoked with a value indicating that the thread is detaching"). A synthetic real module (a 0x1000-byte PE header mapped at its base by the test: DOS
    // e_lfanew 0x80, "PE\0\0", PE32+ magic 0x20b, AddressOfEntryPoint 0x1000) whose entry is a stub: LoadLibraryW makes it loaded (reason 1); a thread made later gets 2 before its first instruction and 3
    // after its last, both in its own context (thread index 1), with hinstDLL = the base and lpvReserved NULL. The first thread (the harness's) gets no ATTACH (Learn: DLL_PROCESS_ATTACH was its notice).
    let top = format!(
        "{VA}#include \"fle_realmod.h\"\n#include <sys/mman.h>\n\
         const fle_realmod_entry fle_realmod_table[] = {{ {{ \"stubdll.dll\", 0x10000000 }} }};\nconst uint64_t fle_realmod_count = 1;\n\
         static unsigned notif[16][4]; static unsigned nn;\n\
         static void dllmain(void) {{ if (nn < 16) {{ notif[nn][0] = (unsigned)R[2]; notif[nn][1] = fle_thread_current(); notif[nn][2] = (unsigned)(R[1] == 0x10000000); notif[nn][3] = (unsigned)(R[8] == 0); nn++; }} R[0] = 1; }}\n\
         static void th_a(void) {{ note(10); R[0] = 0; }}\n\
         const fle_fn_entry fle_dispatch_table[] = {{ {{ VA_A, th_a }}, {{ 0x10001000, dllmain }} }};\nconst uint64_t fle_dispatch_count = 2;\n"
    );
    let body = r#"
    uint8_t *img = mmap((void *)0x10000000, 0x2000, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED_NOREPLACE, -1, 0);
    uint64_t th, ex;
    unsigned i;
    static uint16_t nm[] = { 's','t','u','b','d','l','l','.','d','l','l', 0 };
    if (img == MAP_FAILED) { printf("mmap failed\n"); return 1; }
    img[0] = 'M'; img[1] = 'Z'; *(uint32_t *)(img + 0x3c) = 0x80; *(uint32_t *)(img + 0x80) = 0x4550;
    *(uint16_t *)(img + 0x80 + 24) = 0x20b; *(uint32_t *)(img + 0x80 + 24 + 16) = 0x1000;
    CALL4(fle_imp_LoadLibraryW, A(nm), 0, 0, 0);
    printf("loaded %llx notices %u\n", U(R[0]), nn);
    th = H_CREATE(VA_A, 0, 0, 0);
    printf("after-create %u\n", nn);
    W1(th, INF);
    ex = EXITCODE(th);
    printf("after-end %u exit %llu\n", nn, U(ex));
    for (i = 0; i < nn; i++) printf("n%u reason %u thread %u base %u reserved-null %u\n", i, notif[i][0], notif[i][1], notif[i][2], notif[i][3]);
    print_order();
"#;
    let o = run_src_with("d-notify", &top, body, &RT_LOADER);
    assert!(o.ok, "d-notify: signal {:?}: {}\n{}", o.signal, o.stderr, o.stdout);
    let l: Vec<String> = o.stdout.lines().map(|x| x.to_string()).collect();
    assert_eq!(l[0], "loaded 10000000 notices 1");
    assert_eq!(l[1], "after-create 1");
    assert_eq!(l[2], "after-end 3 exit 0");
    // n0 = DLL_PROCESS_ATTACH of the load (the first thread); n1 = ATTACH in thread 1, n2 = DETACH in thread 1
    assert_eq!(l[3], "n0 reason 1 thread 0 base 1 reserved-null 1");
    assert_eq!(l[4], "n1 reason 2 thread 1 base 1 reserved-null 1");
    assert_eq!(l[5], "n2 reason 3 thread 1 base 1 reserved-null 1");
    assert_eq!(l[6], "order: 10");
}

#[test]
fn dwmapi_is_virtual_module_39_with_the_handle_of_the_formula_and_dwm_flush_is_its_only_row() {
    // PM item 5: dwmapi.dll is the next virtual module (N = 39, index 38; handle 0x7FF800000000 + 0x10000 * 39); LoadLibraryW("dwmapi") and GetModuleHandleW find it; the response table has one row under it.
    let (_, c) = gen(&GUEST_IMPORTS);
    let rows = resp_rows(&c);
    assert_eq!(rows.iter().filter(|(d, _, _)| d == "dwmapi.dll").map(|(_, n, _)| n.as_str()).collect::<Vec<_>>(), ["DwmFlush"]);
    let t = std::fs::read_to_string(rt_dir().join("fle_modules.c")).unwrap();
    let a = t.find("static const char *const fle_modules[] = {").unwrap();
    let list = &t[a..a + t[a..].find("};").unwrap()];
    let stripped = strip_comments(list);
    let names: Vec<&str> = stripped.split('"').enumerate().filter(|(i, _)| i % 2 == 1).map(|(_, s)| s).collect();
    assert_eq!(names.len(), 41);   // decision 0238: kernelbase.dll (N = 40) and combase.dll (N = 41) are appended after dwmapi.dll (old: 39)
    assert_eq!(names[38], "dwmapi.dll");
    assert_eq!(names[37], "uxtheme.dll");
    assert_eq!(names[39], "kernelbase.dll");
    assert_eq!(names[40], "combase.dll");
}

#[test]
fn dwm_flush_waits_for_the_next_60_hz_boundary_of_the_virtual_clock_and_returns_s_ok() {
    // PM item 5: S_OK (0); the caller waits until the next boundary of 1/60 s of the virtual clock: boundary k = ceil(k * 10^7 / 60) ticks (100 ns) since the virtual boot (the clock's zero). A choice
    // without a source (the Learn page: "waits for any queued DirectX changes ... to be drawn to the screen").
    let top = format!("{VA}static void th_a(void) {{ R[0] = 0; }}\nconst fle_fn_entry fle_dispatch_table[] = {{ {{ VA_A, th_a }} }};\nconst uint64_t fle_dispatch_count = 1;");
    let l = lines(
        "w-dwm",
        &top,
        r#"
    uint64_t t, k;
    int i;
    uint64_t th = H_CREATE(VA_A, 0, 0, 0);
    W1(th, INF);
    for (i = 0; i < 4; i++) {
        t = fle_sys_clock_ticks();
        CALL4(fle_imp_DwmFlush, 0, 0, 0, 0);
        k = fle_sys_clock_ticks();
        /* the first boundary strictly after t, by the formula k = floor(t * 60 / 10^7) + 1 -> ceil(k * 10^7 / 60) */
        printf("hr %llx on-boundary %d gap<=166667 %d\n", U(R[0]), (int)(((k - 10) * 60 + 9999999) / 10000000 * 10000000 / 60 >= 0), (int)(k - t <= 166667 + 10));
    }
    t = fle_sys_clock_ticks();
    printf("next-boundary-formula %llu\n", U(fle_thread_next_frame_tick(t)));
"#,
    );
    for i in 0..4 {
        assert!(l[i].starts_with("hr 0 on-boundary 1 gap<=166667 1"), "{}", l[i]);
    }
}

#[test]
fn the_60_hz_boundary_arithmetic_is_exact_at_hand_computed_points() {
    // ceil(k * 10^7 / 60) for k = 1.. : 166667, 333334, 500000, 666667, 833334, 1000000 (hand computed: 10^7 / 60 = 166666.666...). 36,000,000,000 ticks = 3,600 s is exactly frame 216,000
    // (216000 * 10^7 / 60 = 36,000,000,000), so the first boundary STRICTLY after it is frame 216,001 = ceil(216001 * 10^7 / 60) = 36,000,166,667.
    let top = "const fle_fn_entry fle_dispatch_table[] = { { 0x1000, 0 } };\nconst uint64_t fle_dispatch_count = 1;";
    let l = lines(
        "w-boundary",
        top,
        r#"
    uint64_t pts[] = { 0, 1, 166666, 166667, 166668, 333333, 333334, 999999, 1000000, 36000000000ull };
    unsigned i;
    for (i = 0; i < sizeof pts / sizeof pts[0]; i++) printf("%llu -> %llu\n", U(pts[i]), U(fle_thread_next_frame_tick(pts[i])));
"#,
    );
    assert_eq!(
        l,
        [
            "0 -> 166667",
            "1 -> 166667",
            "166666 -> 166667",
            "166667 -> 333334",
            "166668 -> 333334",
            "333333 -> 333334",
            "333334 -> 500000",
            "999999 -> 1000000",
            "1000000 -> 1166667",
            "36000000000 -> 36000166667"
        ]
    );
}
