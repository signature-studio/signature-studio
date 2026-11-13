//! Tests of decision 0123 (docs/decisions/0123-window-model-and-walls.md, PROVISIONAL): the state-only window-management model of runtime/fle_win.c (R1..R10 of docs/investigations/detach-window.md section 3.2),
//! the messages it sends in the order of the Learn pages, the timers and WM_PAINT it makes from the virtual clock and the update regions, the per-thread focus / capture / active window, the VirtualQuery model of
//! runtime/fle_vmap.c, the GetSysColor table and GetFileAttributesW("") of the wall loop, the region objects of runtime/fle_gdi.c, the scroll bars, DeferWindowPos, and the thunk call of fle_dispatch_miss.
//!
//! Expected values are written here from the Microsoft Learn pages named in the decision (fetched 2026-10-31, kept in lift/out-fruity9/learn/ with index.tsv) and from the choices of the decision
//! (each choice without a source is named S-... in the decision document), not read back from the C code. EVERY ANSWER IS PROVISIONAL; nothing is compared with a real Windows. A test that pins a choice
//! without a source says so in its comment: it fixes the choice, not a Windows rule.
//!
//! 1. Generator: the 73 rows of decision 0123 are the last rows, in the order given, and no pseudo-VA moved.
//! 2. Window attributes R1..R10 and the creation messages.
//! 3. Position, size, z-order, TOPMOST, show, destroy (the message order).
//! 4. Focus, capture, active window (per thread), foreground (global).
//! 5. Timers and WM_PAINT made from the virtual clock and the update regions.
//! 6. Properties, scroll bars, DeferWindowPos, placement, menus.
//! 7. Regions, GetSysColor, GetFileAttributesW(""), VirtualQuery, FreeResource.
//! 8. The thunk call of fle_dispatch_miss, and the source checks.

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
    if let Ok(d) = std::env::var("FLE_FR9_SRC_DIR") {
        return PathBuf::from(d);
    }
    rt_dir()
}
fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-wallFR9/test");
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

/// The 73 rows of decision 0123 in the order given (decision 0015: append only). The first 49 are the window model (wall 24), then the rows that the later walls showed.
const NEW_ROWS: [(&str, &str); 73] = [
    ("user32.dll", "IsWindowVisible"),
    ("user32.dll", "ShowWindow"),
    ("user32.dll", "SetWindowPos"),
    ("user32.dll", "MoveWindow"),
    ("user32.dll", "GetWindowRect"),
    ("user32.dll", "GetClientRect"),
    ("user32.dll", "ClientToScreen"),
    ("user32.dll", "ScreenToClient"),
    ("user32.dll", "MapWindowPoints"),
    ("user32.dll", "EnableWindow"),
    ("user32.dll", "IsWindowEnabled"),
    ("user32.dll", "IsWindow"),
    ("user32.dll", "GetParent"),
    ("user32.dll", "GetWindow"),
    ("user32.dll", "SetParent"),
    ("user32.dll", "GetAncestor"),
    ("user32.dll", "GetWindowThreadProcessId"),
    ("user32.dll", "SetFocus"),
    ("user32.dll", "GetFocus"),
    ("user32.dll", "SetActiveWindow"),
    ("user32.dll", "GetActiveWindow"),
    ("user32.dll", "GetForegroundWindow"),
    ("user32.dll", "SetForegroundWindow"),
    ("user32.dll", "SetCapture"),
    ("user32.dll", "GetCapture"),
    ("user32.dll", "ReleaseCapture"),
    ("user32.dll", "WindowFromPoint"),
    ("user32.dll", "GetCursorPos"),
    ("user32.dll", "FindWindowW"),
    ("user32.dll", "UnregisterClassW"),
    ("user32.dll", "SetTimer"),
    ("user32.dll", "KillTimer"),
    ("user32.dll", "InvalidateRect"),
    ("user32.dll", "ValidateRect"),
    ("user32.dll", "UpdateWindow"),
    ("user32.dll", "BeginPaint"),
    ("user32.dll", "EndPaint"),
    ("user32.dll", "EnumWindows"),
    ("user32.dll", "DestroyWindow"),
    ("user32.dll", "IsChild"),
    ("user32.dll", "IsIconic"),
    ("user32.dll", "IsZoomed"),
    ("user32.dll", "GetDesktopWindow"),
    ("user32.dll", "GetTopWindow"),
    ("user32.dll", "BringWindowToTop"),
    ("user32.dll", "GetWindowTextW"),
    ("user32.dll", "EnumChildWindows"),
    ("user32.dll", "EnumThreadWindows"),
    ("user32.dll", "GetUpdateRect"),
    ("kernel32.dll", "FreeResource"),
    ("user32.dll", "IsWindowUnicode"),
    ("user32.dll", "GetClassNameW"),
    ("user32.dll", "SetPropW"),
    ("user32.dll", "GetPropW"),
    ("user32.dll", "RemovePropW"),
    ("user32.dll", "CallWindowProcW"),
    ("user32.dll", "SetMenu"),
    ("user32.dll", "GetMenu"),
    ("user32.dll", "DrawMenuBar"),
    ("comctl32.dll", "InitializeFlatSB"),
    ("comctl32.dll", "FlatSB_SetScrollProp"),
    ("comctl32.dll", "FlatSB_SetScrollInfo"),
    ("comctl32.dll", "FlatSB_GetScrollInfo"),
    ("comctl32.dll", "FlatSB_SetScrollPos"),
    ("comctl32.dll", "FlatSB_GetScrollPos"),
    ("user32.dll", "SetScrollInfo"),
    ("user32.dll", "GetScrollInfo"),
    ("user32.dll", "GetWindowPlacement"),
    ("user32.dll", "BeginDeferWindowPos"),
    ("user32.dll", "DeferWindowPos"),
    ("user32.dll", "EndDeferWindowPos"),
    ("user32.dll", "RedrawWindow"),
    ("gdi32.dll", "PtInRegion"),
];

#[test]
fn the_73_new_rows_are_the_last_rows_in_the_order_given_and_no_pseudo_va_moved() {
    let (st, c) = gen(&GUEST_IMPORTS);
    let rows = resp_rows(&c);
    assert_eq!(rows.len(), st.resp_entries);
    let n = rows.len();
    // decision 0015: append only. The row before the 73 is the last row of decision 0122, user32!MsgWaitForMultipleObjects; the original 469 rows of HEAD 8c8131b end there.
    // Decision 0125 (old: the 73 were the LAST rows, `n - 73 + k`, and n was 469 + 73) appended 2 rows after them (tests/wall_fr10.rs): the 73 end at n - 2.
    // Decision 0126 (old: `rows.len() - 2`) appended 33 more rows (tests/wall_fr11.rs): the 73 end at n - 35.
    // Decision 0129 (old: `rows.len() - 2 - 33`) appended 4 more rows (tests/wall_fr12.rs): the 73 end at n - 39.
    // decision 0136: rows are appended at the END again: the end of the group is found by the NAME of its last row, not by counting from the end of the table.
    let last = NEW_ROWS[NEW_ROWS.len() - 1];
    let n = rows.iter().position(|r| (r.0.as_str(), r.1.as_str()) == last).expect("the last row of the group") + 1;
    assert_eq!((rows[n - 74].0.as_str(), rows[n - 74].1.as_str()), ("user32.dll", "MsgWaitForMultipleObjects"));
    assert_eq!(n, 469 + 73, "469 rows of HEAD 8c8131b and the 73 of decision 0123");
    for (k, (dll, name)) in NEW_ROWS.iter().enumerate() {
        assert_eq!((rows[n - 73 + k].0.as_str(), rows[n - 73 + k].1.as_str()), (*dll, *name), "row {}", n - 73 + k);
    }
    for (k, (_, _, va)) in rows.iter().enumerate() {
        assert_eq!(*va, 0x7FF9_0000_0000u64 + 0x10 * (k as u64 + 1), "row {k}: the pseudo-VA of a row never moves");
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
fn the_apis_that_stay_unanswered_by_decision_0123_are_not_rows() {
    // Learn is silent or the model is a large design: these have no row (decision 0123, "still stops"); SetWindowOrgEx is a row of decision 0111 that stops for hdc 0 (wall 52)
    let table = std::fs::read_to_string(manifest().join("src/imports_table.rs")).unwrap();
    // decision 0126 (wall 84): old: "AdjustWindowRectEx" was in this list; it is a row now (runtime/fle_win.c, S-adjustrect-nc-zero; tests/wall_fr11.rs)
    // decision 0147: old: "PostThreadMessageW" was in this list; it is a row now (runtime/fle_win.c; tests/wall_fr20.rs)
    for n in ["TerminateThread", "SendMessageA", "GetMessageA", "CreateWindowExA", "SetWindowLongW", "GetSystemMetrics_", "TrackPopupMenu"] {
        assert!(!table.contains(&format!("\"{n}\"")), "{n} must not be a row of decision 0123");
    }
}

// ---------------------------------------------------------------------------------------------
// 2. The driver: the real fle_win.c, fle_class.c, fle_thread.c and the files they work with; a stub guest window procedure that logs every message
// ---------------------------------------------------------------------------------------------

const RT_FILES: [&str; 21] = [
    "fle_thread.c", "fle_win.c", "fle_class.c", "fle_event.c", "fle_sys.c", "fle_imports.c", "fle_proc.c", "fle_callback.c", "fle_env.c", "fle_vfs.c", "fle_vmap.c", "fle_vol.c", "fle_host.c",
    "fle_time.c", "fle_gdc.c", "fle_gdi.c", "fle_cursor.c", "fle_raster.c", "fle_monitor.c", "fle_ole.c", "fle_reg.c",
];
const RT_EXTRA: [&str; 4] = ["fle_x87ctl.c", "fle_mxcsr.c", "fle_x87.c", "fle_locale.c"];

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
        let dir = work_dir().join(etxt_common::per_process("obj-fr9"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut v = Vec::new();
        let ours = ["fle_thread.c", "fle_win.c", "fle_class.c", "fle_gdi.c", "fle_vmap.c", "fle_vfs.c", "fle_raster.c", "fle_monitor.c", "fle_ole.c"];
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
#include "fle_vmap.h"
#include "fle_gdc.h"
#include "fle_gdi.h"
#include "fle_locale.h"
#include "fle_sysinfo.h"
#include "fle_x87ctl.h"
#include "fle_mxcsr.h"
void fle_imp_CloseHandle(void);
void fle_imp_LoadLibraryW(void);

uint64_t R[16];
fle_eflags FL;
fle_xmm X[16];
static uint64_t stk[0x4000] __attribute__((aligned(4096)));
#define E0 ((uint64_t)(uintptr_t)stk + 0x10000 + 8)
#define RET0 UINT64_C(0xdead0000)
#define U(x) ((unsigned long long)(x))
#define A(x) ((uint64_t)(uintptr_t)(x))
#define INF 0xFFFFFFFFu

#define CALL4(f, a, b, c, d) do { R[1] = (a); R[2] = (b); R[8] = (c); R[9] = (d); R[4] -= 8; fle_st(R[4], 8, 0xAAAA0000); f(); R[4] += 8; } while (0)
#define CALL6(f, a, b, c, d, e, g) do { uint64_t sp0_ = R[4]; R[4] = (R[4] - 0x40) & ~UINT64_C(15); R[4] -= 8; fle_st(R[4], 8, 0xAAAA0000); fle_st(R[4] + 0x28, 8, (e)); fle_st(R[4] + 0x30, 8, (g)); \
    R[1] = (a); R[2] = (b); R[8] = (c); R[9] = (d); f(); R[4] = sp0_; } while (0)
#define CALL5(f, a, b, c, d, e) CALL6(f, a, b, c, d, e, 0)
/* C(f, a, b, c, d) returns R[0] */
static uint64_t callf(void (*f)(void), uint64_t a, uint64_t b, uint64_t c, uint64_t d) { CALL4(f, a, b, c, d); return R[0]; }
#define C(f, ...) callf(f, __VA_ARGS__)
static uint64_t callf5(void (*f)(void), uint64_t a, uint64_t b, uint64_t c, uint64_t d, uint64_t e) { CALL5(f, a, b, c, d, e); return R[0]; }
static uint64_t callf6(void (*f)(void), uint64_t a, uint64_t b, uint64_t c, uint64_t d, uint64_t e, uint64_t g) { CALL6(f, a, b, c, d, e, g); return R[0]; }
/* the stack arguments 5, 6, 7 (and 8) of a call: [rsp+0x28], [rsp+0x30], [rsp+0x38], [rsp+0x40] at the callee's entry */
static uint64_t callf8(void (*f)(void), uint64_t a, uint64_t b, uint64_t c, uint64_t d, uint64_t e, uint64_t g, uint64_t h7, uint64_t h8)
{
    uint64_t sp0_ = R[4];
    R[4] = (R[4] - 0x60) & ~UINT64_C(15); R[4] -= 8; fle_st(R[4], 8, 0xAAAA0000);
    fle_st(R[4] + 0x28, 8, e); fle_st(R[4] + 0x30, 8, g); fle_st(R[4] + 0x38, 8, h7); fle_st(R[4] + 0x40, 8, h8);
    R[1] = a; R[2] = b; R[8] = c; R[9] = d; f(); R[4] = sp0_;
    return R[0];
}
/* SWP(h, after, x, y, cx, cy, flags) */
#define SWP(h, after, x, y, cx, cy, fl) callf8(fle_imp_SetWindowPos, h, after, (uint32_t)(x), (uint32_t)(y), (uint32_t)(cx), (uint32_t)(cy), fl, 0)
static uint32_t LE(void) { fle_imp_GetLastError(); return (uint32_t)R[0]; }
static void SETLE(uint32_t v) { fle_set_last_error(v); }
static uint64_t H_CREATE(uint64_t start, uint64_t param, uint32_t flags, uint64_t ptid) { return callf6(fle_imp_CreateThread, 0, 0, start, param, flags, ptid); }
static uint32_t W1(uint64_t h, uint32_t ms) { return (uint32_t)callf(fle_imp_WaitForSingleObject, h, ms, 0, 0); }
static void SLEEP(uint32_t ms) { CALL4(fle_imp_Sleep, ms, 0, 0, 0); }
static void boot(void) { R[4] = E0; fle_st(E0, 8, RET0); fle_teb_set_stack(A(stk), A(stk) + sizeof stk); }

/* the window procedure stub: logs (thread, hwnd, message, wParam, lParam) of every call; the creation messages are answered as Learn says (WM_NCCREATE 1, WM_NCCALCSIZE 0, WM_CREATE 0) and the rest by
 * the default processing (DefWindowProcW) unless `wp_eat` is set for the message; WM_WINDOWPOSCHANGING can be made to rewrite the WINDOWPOS (`wp_rewrite`) */
#define VA_WP 0x2000
typedef struct { uint64_t h, msg, wp, lp; unsigned thread; } wpent;
static wpent wl[512];
static int wn;
static uint32_t wp_eat_msg;
static int wp_rewrite;
static int32_t wp_new[4];
static uint64_t wp_result;
static void wndproc(void)
{
    uint64_t h = R[1], msg = R[2], wp = R[8], lp = R[9], sp = R[4];
    if (wn < 512) { wl[wn].h = h; wl[wn].msg = msg; wl[wn].wp = wp; wl[wn].lp = lp; wl[wn].thread = fle_thread_current(); }
    wn++;
    if (msg == 0x81) { R[0] = 1; return; }
    if (msg == 0x83) { R[0] = 0; return; }
    if (msg == 0x01) { R[0] = 0; return; }
    if (msg == 0x46 && wp_rewrite) {
        fle_st(lp + 16, 4, (uint32_t)wp_new[0]); fle_st(lp + 20, 4, (uint32_t)wp_new[1]); fle_st(lp + 24, 4, (uint32_t)wp_new[2]); fle_st(lp + 28, 4, (uint32_t)wp_new[3]);
        R[0] = 0; return;
    }
    if (wp_eat_msg && msg == wp_eat_msg) { R[0] = wp_result; return; }
    /* the default processing: DefWindowProcW(hWnd, Msg, wParam, lParam) as the guest's own handler would call it */
    R[1] = h; R[2] = msg; R[8] = wp; R[9] = lp;
    R[4] = sp - 8; fle_st(R[4], 8, 0xAAAA0001);
    fle_imp_DefWindowProcW();
    R[4] = sp;
}
const fle_fn_entry fle_dispatch_table_base[] = { { VA_WP, wndproc } };

static int cls_done;
static uint16_t cls_name[] = { 'T','P','U','t','i','l','W','i','n','d','o','w', 0 };
static void regclass(void)
{
    static uint8_t wc[72];
    uint64_t w = VA_WP, inst = 0x400000, nm = A(cls_name);
    if (cls_done) return;
    memset(wc, 0, sizeof wc); memcpy(wc + 8, &w, 8); memcpy(wc + 24, &inst, 8); memcpy(wc + 64, &nm, 8);
    CALL4(fle_imp_GetClassInfoW, 0x400000, A(cls_name), A(wc), 0);
    CALL4(fle_imp_RegisterClassW, A(wc), 0, 0, 0);
    cls_done = 1;
}
static uint16_t W16[8][128];
static unsigned W16k;
static uint64_t S(const char *s)
{
    uint16_t *p = W16[W16k++ % 8]; size_t i, n = strlen(s);
    for (i = 0; i < n; i++) p[i] = (uint8_t)s[i];
    p[n] = 0;
    return A(p);
}
/* CreateWindowExW(ex, class TPUtilWindow, title, style, x, y, w, h, parent, menu, hInstance 0x400000, param) */
static uint64_t cw(uint64_t ex, const char *title, uint64_t style, int32_t x, int32_t y, int32_t w, int32_t hh, uint64_t parent, uint64_t menu, uint64_t param)
{
    uint64_t sp0 = R[4], h;
    regclass();
    R[4] = (R[4] - 0x80) & ~UINT64_C(15); R[4] -= 8; fle_st(R[4], 8, 0xAAAA0000);
    fle_st(R[4] + 0x28, 8, (uint32_t)x); fle_st(R[4] + 0x30, 8, (uint32_t)y); fle_st(R[4] + 0x38, 8, (uint32_t)w); fle_st(R[4] + 0x40, 8, (uint32_t)hh);
    fle_st(R[4] + 0x48, 8, parent); fle_st(R[4] + 0x50, 8, menu); fle_st(R[4] + 0x58, 8, 0x400000); fle_st(R[4] + 0x60, 8, param);
    R[1] = ex; R[2] = A(cls_name); R[8] = title ? S(title) : 0; R[9] = style;
    fle_imp_CreateWindowExW();
    h = R[0];
    R[4] = sp0;
    return h;
}
#define POPUP 0x80000000u
#define CHILD 0x40000000u
#define VIS   0x10000000u
static int32_t RC[4];
static void rect_of(void (*f)(void), uint64_t h) { memset(RC, 0, sizeof RC); callf(f, h, A(RC), 0, 0); }
static void pr(const char *tag) { printf("%s %d %d %d %d\n", tag, RC[0], RC[1], RC[2], RC[3]); }
static void wlog(int from)
{
    int i;
    for (i = from; i < wn && i < 512; i++) printf(" m%x(%llx,%llx)", (unsigned)wl[i].msg, U(wl[i].wp), U(wl[i].lp));
    printf("\n");
}
static void wlog_h(int from)
{
    int i;
    for (i = from; i < wn && i < 512; i++) printf(" %llx:%x", U(wl[i].h & 0xffff), (unsigned)wl[i].msg);
    printf("\n");
}
"##;

fn top_with(extra: &str, entries: &[(&str, &str)]) -> String {
    // fle_dispatch searches the table by binary search: the rows must be sorted by VA (VA_WP is 0x2000; the entries given are 0x1000 and 0x52a570 in the tests that use them)
    let mut keyed: Vec<(u64, String)> = Vec::new();
    for (va, f) in entries {
        let n = if let Some(h) = va.strip_prefix("0x") { u64::from_str_radix(h, 16).unwrap() } else if *va == "VA_TARGET" { 0x52a570 } else { panic!("an entry VA must be a hex number or VA_TARGET: {va}") };
        keyed.push((n, format!("{{ {va}, {f} }}")));
    }
    keyed.push((0x2000, "{ VA_WP, wndproc }".to_string()));
    keyed.sort_by_key(|k| k.0);
    let rows: Vec<String> = keyed.into_iter().map(|k| k.1).collect();
    format!("{extra}\nconst fle_fn_entry fle_dispatch_table[] = {{ {} }};\nconst uint64_t fle_dispatch_count = {};\n", rows.join(", "), rows.len())
}

fn run_src(name: &str, top: &str, body: &str) -> Out {
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

/// A run with the stub window procedure (and no other guest function).
fn lines(name: &str, body: &str) -> Vec<String> {
    let o = run_src(name, &top_with("", &[]), body);
    assert!(o.ok, "{name} did not exit normally (signal {:?}): {}\n{}", o.signal, o.stderr, o.stdout);
    o.stdout.lines().map(|s| s.to_string()).collect()
}

fn lines_top(name: &str, top: &str, body: &str) -> Vec<String> {
    let o = run_src(name, top, body);
    assert!(o.ok, "{name} did not exit normally (signal {:?}): {}\n{}", o.signal, o.stderr, o.stdout);
    o.stdout.lines().map(|s| s.to_string()).collect()
}

/// The run must stop with SIGABRT and a "fle: <api>: " message that contains `word`.
fn aborts(name: &str, body: &str, api: &str, word: &str) -> Out {
    let o = run_src(name, &top_with("", &[]), body);
    assert!(!o.ok, "{name}: expected an abort, got stdout {}", o.stdout);
    assert_eq!(o.signal, Some(6), "{name}: not SIGABRT: {}", o.stderr);
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

// ---------------------------------------------------------------------------------------------
// 3. R1..R10, the creation, position, size, z-order and show: the messages in the order of the Learn pages
// ---------------------------------------------------------------------------------------------

#[test]
fn r1_parent_and_owner_are_two_fields_getparent_getancestor_and_gwlp_hwndparent_read_them_as_the_pages_say() {
    // Learn GetParent: a child window gives its parent; "a top-level window with the WS_POPUP style" gives the owner; GetAncestor(GA_PARENT) "does not include the owner"; GetWindowLongPtr GWLP_HWNDPARENT is "a handle
    // to the parent window, if there is one"; SetWindowLongPtr: "A window can have either a parent or an owner, or neither, but never both". S-getparent: GetParent is applied literally (an owner with WS_POPUP is not a
    // parent: the page's "owned window" rule gives the owner only when the window itself has WS_POPUP).
    let l = lines(
        "r1",
        r#"
    uint64_t top = cw(0x80, "main", POPUP, 10, 20, 300, 200, 0, 0, 0);
    uint64_t kid = cw(0, "kid", CHILD, 5, 6, 100, 50, top, 7, 0);
    uint64_t own = cw(0x80, "owned", POPUP, 0, 0, 10, 10, top, 0, 0);
    printf("kid parent is top %d, owned parent is top %d\n", callf(fle_imp_GetParent, kid, 0, 0, 0) == top, callf(fle_imp_GetParent, own, 0, 0, 0) == top);
    printf("ga_parent kid top %d, ga_parent owned %llx\n", callf(fle_imp_GetAncestor, kid, 1, 0, 0) == top, U(callf(fle_imp_GetAncestor, own, 1, 0, 0)));
    printf("ga_root kid top %d\n", callf(fle_imp_GetAncestor, kid, 2, 0, 0) == top);
    printf("gw_owner owned top %d, gw_owner kid %llx\n", callf(fle_imp_GetWindow, own, 4, 0, 0) == top, U(callf(fle_imp_GetWindow, kid, 4, 0, 0)));
    printf("gwlp_hwndparent kid top %d owned top %d top %llx\n", callf(fle_imp_GetWindowLongPtrW, kid, 0xfffffff8u, 0, 0) == top, callf(fle_imp_GetWindowLongPtrW, own, 0xfffffff8u, 0, 0) == top, U(callf(fle_imp_GetWindowLongPtrW, top, 0xfffffff8u, 0, 0)));
    printf("gwlp_id kid %llu top %llu\n", U(callf(fle_imp_GetWindowLongPtrW, kid, 0xfffffff4u, 0, 0)), U(callf(fle_imp_GetWindowLongPtrW, top, 0xfffffff4u, 0, 0)));
    printf("ischild kid-top %llu owned-top %llu\n", U(callf(fle_imp_IsChild, top, kid, 0, 0)), U(callf(fle_imp_IsChild, top, own, 0, 0)));
"#,
    );
    assert_eq!(
        l,
        [
            "kid parent is top 1, owned parent is top 0",
            "ga_parent kid top 1, ga_parent owned 7f990000",
            "ga_root kid top 1",
            "gw_owner owned top 1, gw_owner kid 0",
            "gwlp_hwndparent kid top 1 owned top 1 top 0",
            "gwlp_id kid 7 top 0",
            "ischild kid-top 1 owned-top 0",
        ]
    );
}

#[test]
fn r2_the_rectangle_is_parent_relative_and_the_screen_position_comes_from_the_parent_chain() {
    // Learn Window Features: "The coordinates of a top-level window are relative to the upper left corner of the screen; the coordinates of a child window are relative to the upper left corner of the parent window's client
    // area". GetWindowRect: screen coordinates. ClientToScreen / ScreenToClient / MapWindowPoints convert. S-nc: the non-client area is 0, so the client rectangle is the window rectangle (a choice without a source).
    let l = lines(
        "r2",
        r#"
    uint64_t top = cw(0x80, "main", POPUP, 100, 50, 300, 200, 0, 0, 0);
    uint64_t kid = cw(0, "kid", CHILD, 5, 6, 100, 40, top, 1, 0);
    uint64_t kk = cw(0, "kk", CHILD, 2, 3, 10, 10, kid, 2, 0);
    int32_t pt[2];
    rect_of(fle_imp_GetWindowRect, top); pr("top window rect");
    rect_of(fle_imp_GetWindowRect, kid); pr("kid window rect (screen)");
    rect_of(fle_imp_GetWindowRect, kk); pr("kk window rect (screen)");
    rect_of(fle_imp_GetClientRect, kid); pr("kid client rect");
    pt[0] = 0; pt[1] = 0; callf(fle_imp_ClientToScreen, kk, A(pt), 0, 0); printf("kk origin on screen %d %d\n", pt[0], pt[1]);
    pt[0] = 107; pt[1] = 59; callf(fle_imp_ScreenToClient, kid, A(pt), 0, 0); printf("screen 107,59 in kid %d %d\n", pt[0], pt[1]);
"#,
    );
    assert_eq!(l[0], "top window rect 100 50 400 250");
    assert_eq!(l[1], "kid window rect (screen) 105 56 205 96");
    assert_eq!(l[2], "kk window rect (screen) 107 59 117 69");
    assert_eq!(l[3], "kid client rect 0 0 100 40");
    assert_eq!(l[4], "kk origin on screen 107 59");
    assert_eq!(l[5], "screen 107,59 in kid 2 3");
}

#[test]
fn r2_moving_the_parent_moves_the_children_on_the_screen_and_a_child_move_changes_only_its_own_numbers() {
    let l = lines(
        "r2-move",
        r#"
    uint64_t top = cw(0x80, "main", POPUP, 100, 50, 300, 200, 0, 0, 0);
    uint64_t kid = cw(0, "kid", CHILD, 5, 6, 100, 40, top, 1, 0);
    /* SWP_NOZORDER 4 | SWP_NOACTIVATE 0x10 */
    printf("move parent %llu\n", U(SWP(top, 0, 400, 300, 300, 200, 0x14)));
    rect_of(fle_imp_GetWindowRect, kid); pr("kid on screen");
    rect_of(fle_imp_GetWindowRect, top); pr("top on screen");
    printf("move child %llu\n", U(SWP(kid, 0, 20, 30, 100, 40, 0x14)));
    rect_of(fle_imp_GetWindowRect, kid); pr("kid on screen");
    rect_of(fle_imp_GetClientRect, kid); pr("kid client");
"#,
    );
    assert_eq!(l, ["move parent 1", "kid on screen 405 306 505 346", "top on screen 400 300 700 500", "move child 1", "kid on screen 420 330 520 370", "kid client 0 0 100 40"]);
}

#[test]
fn r3_the_z_order_is_a_sibling_list_top_first_and_topmost_windows_come_before_the_others() {
    // Learn Window Features: "The system maintains the z-order in a single list. ... All topmost windows appear in the z-order before any non-topmost windows. A child window is grouped with its parent in z-order."
    // SetWindowPos: HWND_TOPMOST "Places the window above all non-topmost windows"; HWND_NOTOPMOST; HWND_BOTTOM; HWND_TOP. GetWindow GW_HWNDNEXT is the window below, GW_HWNDPREV above.
    let l = lines(
        "r3",
        r#"
    uint64_t a = cw(0, "a", POPUP, 0, 0, 10, 10, 0, 0, 0), b = cw(0, "b", POPUP, 0, 0, 10, 10, 0, 0, 0), c = cw(0, "c", POPUP, 0, 0, 10, 10, 0, 0, 0);
    uint64_t kid1 = cw(0, "k1", CHILD, 0, 0, 5, 5, a, 1, 0), kid2 = cw(0, "k2", CHILD, 0, 0, 5, 5, a, 2, 0);
    /* Learn CreateWindowExW (Remarks): a new child window goes to the bottom of the Z-order and a new top-level window to the top: top-level c, b, a; children of a: kid1, kid2 */
    printf("top is c %d\n", callf(fle_imp_GetTopWindow, 0, 0, 0, 0) == c);
    printf("c next is b %d, b next is a %d, a next %llx\n", callf(fle_imp_GetWindow, c, 2, 0, 0) == b, callf(fle_imp_GetWindow, b, 2, 0, 0) == a, U(callf(fle_imp_GetWindow, a, 2, 0, 0)));
    printf("a's top child is kid1 %d, kid1 next kid2 %d\n", callf(fle_imp_GetTopWindow, a, 0, 0, 0) == kid1, callf(fle_imp_GetWindow, kid1, 2, 0, 0) == kid2);
    /* a goes to the top; then topmost for b: b is above a. Learn GetWindow: "If the specified window is a topmost window, the handle identifies a topmost window": GW_HWNDNEXT of the only topmost window is NULL, and
       GW_HWNDFIRST of a is the highest NON-topmost window */
    SWP(a, 0, 0, 0, 0, 0, 0x13);                                  /* HWND_TOP, SWP_NOSIZE | SWP_NOMOVE | SWP_NOACTIVATE */
    printf("a at top %d\n", callf(fle_imp_GetTopWindow, 0, 0, 0, 0) == a);
    SWP(b, 0xFFFFFFFFFFFFFFFFull, 0, 0, 0, 0, 0x13);              /* HWND_TOPMOST */
    printf("b topmost bit %d, b at top %d, GW_HWNDNEXT of b %llx, GW_HWNDFIRST of a is a %d\n", (callf(fle_imp_GetWindowLongPtrW, b, 0xffffffecu, 0, 0) & 8) != 0, callf(fle_imp_GetTopWindow, 0, 0, 0, 0) == b, U(callf(fle_imp_GetWindow, b, 2, 0, 0)), callf(fle_imp_GetWindow, a, 0, 0, 0) == a);
    SWP(a, 0, 0, 0, 0, 0, 0x13);
    printf("a raised again: top is still b %d\n", callf(fle_imp_GetTopWindow, 0, 0, 0, 0) == b);
    /* HWND_BOTTOM: a topmost window placed at the bottom loses TOPMOST (Learn SetWindowPos) */
    SWP(b, 1, 0, 0, 0, 0, 0x13);
    printf("b bottom: topmost bit %d, last is b %d\n", (callf(fle_imp_GetWindowLongPtrW, b, 0xffffffecu, 0, 0) & 8) != 0, callf(fle_imp_GetWindow, a, 1, 0, 0) == b);
    /* HWND_NOTOPMOST on a window that is not topmost has no effect */
    SWP(c, 0xFFFFFFFFFFFFFFFEull, 0, 0, 0, 0, 0x13);
    printf("c after NOTOPMOST bit %d\n", (callf(fle_imp_GetWindowLongPtrW, c, 0xffffffecu, 0, 0) & 8) != 0);
    /* GW_HWNDFIRST / GW_HWNDLAST over the children */
    printf("kid2 first is kid1 %d, kid1 last is kid2 %d\n", callf(fle_imp_GetWindow, kid2, 0, 0, 0) == kid1, callf(fle_imp_GetWindow, kid1, 1, 0, 0) == kid2);
"#,
    );
    assert_eq!(
        l,
        [
            "top is c 1",
            "c next is b 1, b next is a 1, a next 0",
            "a's top child is kid1 1, kid1 next kid2 1",
            "a at top 1",
            "b topmost bit 1, b at top 1, GW_HWNDNEXT of b 0, GW_HWNDFIRST of a is a 1",
            "a raised again: top is still b 1",
            "b bottom: topmost bit 0, last is b 1",
            "c after NOTOPMOST bit 0",
            "kid2 first is kid1 1, kid1 last is kid2 1",
        ]
    );
}

#[test]
fn r4_ws_ex_topmost_is_set_by_setwindowpos_for_owned_windows_and_the_topmost_bit_is_not_taken_from_setwindowlongptr() {
    // Learn SetWindowPos: "When a non-topmost window is made topmost, its owned windows are also made topmost. Its owners, however, are not changed." and "When a topmost window is made non-topmost, its owners and its owned
    // windows are also made non-topmost windows." Extended Window Styles: WS_EX_TOPMOST "To add or remove this style, use the SetWindowPos function." (SetWindowLongPtrW(GWL_EXSTYLE) does not change the bit: S-style records the choice.)
    let l = lines(
        "r4",
        r#"
    uint64_t owner = cw(0, "owner", POPUP, 0, 0, 10, 10, 0, 0, 0);
    uint64_t owned = cw(0, "owned", POPUP, 0, 0, 10, 10, owner, 0, 0);
    uint64_t other = cw(0, "other", POPUP, 0, 0, 10, 10, 0, 0, 0);
    #define TM(h) ((callf(fle_imp_GetWindowLongPtrW, h, 0xffffffecu, 0, 0) & 8) != 0)
    SWP(owner, 0xFFFFFFFFFFFFFFFFull, 0, 0, 0, 0, 0x13);
    printf("owner topmost %d owned topmost %d other topmost %d\n", TM(owner), TM(owned), TM(other));
    SWP(owned, 0xFFFFFFFFFFFFFFFEull, 0, 0, 0, 0, 0x13);
    printf("owned made non-topmost: owned %d owner %d\n", TM(owned), TM(owner));
    SWP(owned, 0xFFFFFFFFFFFFFFFFull, 0, 0, 0, 0, 0x13);
    printf("owned topmost again: owned %d owner %d\n", TM(owned), TM(owner));
    /* SetWindowLongPtrW(GWL_EXSTYLE, 0) replaces the extended style as given (S-style: applied at once, nothing keeps the bit) */
    callf(fle_imp_SetWindowLongPtrW, other, 0xffffffecu, 0x80, 0);
    printf("other ex %llx\n", U(callf(fle_imp_GetWindowLongPtrW, other, 0xffffffecu, 0, 0)));
"#,
    );
    assert_eq!(l[0], "owner topmost 1 owned topmost 1 other topmost 0");
    assert_eq!(l[1], "owned made non-topmost: owned 0 owner 0");
    assert_eq!(l[2], "owned topmost again: owned 1 owner 0");
    assert_eq!(l[3], "other ex 80");
}

#[test]
fn r8_createwindow_sends_nccreate_nccalcsize_create_and_a_visible_window_gets_showwindow_then_the_position_messages() {
    // Learn CreateWindowExW: "sends WM_NCCREATE, WM_NCCALCSIZE, and WM_CREATE"; WM_SHOWWINDOW: "If a window has the WS_VISIBLE style when it is created, the window receives this message after it is created, but before it is displayed".
    // (The messages that the model sends after WM_CREATE for a window with WS_VISIBLE are a reading of the pages: the order is pinned here.)
    let l = lines(
        "r8-create",
        r#"
    int k0 = wn;
    uint64_t h = cw(0x80, "v", POPUP, 3, 4, 50, 60, 0, 0, 0);
    printf("hidden window:"); wlog(k0);
    k0 = wn;
    uint64_t v = cw(0x80, "v", POPUP | VIS, 3, 4, 50, 60, 0, 0, 0);
    printf("visible window:"); wlog(k0);
    printf("both live %d %d, visible %llu %llu\n", h != 0, v != 0, U(callf(fle_imp_IsWindowVisible, h, 0, 0, 0)), U(callf(fle_imp_IsWindowVisible, v, 0, 0, 0)));
"#,
    );
    // the lParam of WM_NCCREATE and WM_CREATE is the CREATESTRUCT (a stack address) and that of WM_NCCALCSIZE the RECT: only the messages and wParam are pinned; WM_CREATE's lParam is the same CREATESTRUCT as WM_NCCREATE's
    let parts: Vec<&str> = l[0].trim_start_matches("hidden window:").split_whitespace().collect();
    // decision 0219 (was 3 messages): a WS_POPUP / WS_CHILD window also gets WM_SIZE (5; SIZE_RESTORED, lParam = height 60 << 16 | width 50 = 0x3c0032) and WM_MOVE (3; lParam = y 4 << 16 | x 3 = 0x40003) after WM_CREATE
    // (Wine 10.0 measured, tests/txt1_createsize.rs; Learn does not list them: S-txt1-create-size)
    assert_eq!(parts.len(), 5, "{l:?}");
    assert!(parts[0].starts_with("m81(0,") && parts[1].starts_with("m83(0,") && parts[2].starts_with("m1(0,") && parts[3] == "m5(0,3c0032)" && parts[4] == "m3(0,40003)", "{l:?}");
    assert_eq!(parts[0].split(',').nth(1), parts[2].split(',').nth(1), "WM_NCCREATE and WM_CREATE get the same CREATESTRUCT");
    // a window with WS_VISIBLE gets WM_SHOWWINDOW (wParam TRUE, lParam 0) right after WM_CREATE and nothing else (Learn WM_SHOWWINDOW)
    let v: Vec<&str> = l[1].trim_start_matches("visible window:").split_whitespace().collect();
    assert_eq!(v.len(), 6, "{l:?}");   // decision 0219: was 4 (WM_SIZE and WM_MOVE come before WM_SHOWWINDOW, as in Wine 10.0)
    assert!(v[0].starts_with("m81(0,") && v[1].starts_with("m83(0,") && v[2].starts_with("m1(0,") && v[3] == "m5(0,3c0032)" && v[4] == "m3(0,40003)" && v[5] == "m18(1,0)", "{l:?}");
    assert_eq!(l[2], "both live 1 1, visible 0 1");
}

#[test]
fn r8_setwindowpos_sends_changing_then_nccalcsize_then_changed_then_move_and_size_and_the_guest_can_rewrite_the_windowpos() {
    // Learn SetWindowPos / WM_WINDOWPOSCHANGING: "While this message is being processed, modifying any of the values in WINDOWPOS affects the window's new size, position, or place in the Z order"; WM_WINDOWPOSCHANGED: "By
    // default, the DefWindowProc function sends the WM_SIZE and WM_MOVE messages to the window"; WM_NCCALCSIZE "is sent only when the window's size is being changed" (SetWindowPos). S-order: WM_NCCALCSIZE comes after
    // WM_WINDOWPOSCHANGING and before WM_WINDOWPOSCHANGED (MoveWindow's page lists the five messages and gives no order: a choice without a source). S-movesize (a choice without a source: the page gives no condition):
    // DefWindowProcW sends WM_MOVE when the WINDOWPOS flags lack SWP_NOMOVE and WM_SIZE when they lack SWP_NOSIZE, whether or not the numbers changed. WM_MOVE lParam = the client origin, WM_SIZE lParam = the client size.
    // decision 0132 (wall 21, S-swp-unchanged-geometry; old: "whether or not the numbers changed", the two `same numbers` rows below had m3 and m5): a call that leaves the position (size) as it is reports SWP_NOMOVE (SWP_NOSIZE) in the
    // WINDOWPOS of WM_WINDOWPOSCHANGED, so no WM_MOVE (WM_SIZE) follows. Learn: WM_SIZE "Sent to a window after its size has changed"; WM_MOVE "Sent after a window has been moved". The guest's Delphi control calls SetWindowPos with the size it
    // has from its WM_SIZE handler; the old rule entered that handler again at every call, 16 callbacks deep.
    let l = lines(
        "r8-pos",
        r#"
    uint64_t h = cw(0x80, "w", POPUP, 10, 20, 100, 80, 0, 0, 0);
    int k0;
    k0 = wn; SWP(h, 0, 30, 40, 100, 80, 0x14 | 1); printf("move, NOSIZE:"); wlog(k0);
    k0 = wn; SWP(h, 0, 30, 40, 120, 90, 0x14 | 2); printf("size, NOMOVE:"); wlog(k0);
    k0 = wn; SWP(h, 0, 5, 6, 130, 95, 0x14); printf("move and size:"); wlog(k0);
    k0 = wn; SWP(h, 0, 5, 6, 130, 95, 0x14); printf("same numbers:"); wlog(k0);
    k0 = wn; SWP(h, 0, 5, 6, 130, 95, 0x14 | 0x400); printf("same numbers, SWP_NOSENDCHANGING:"); wlog(k0);
    k0 = wn; SWP(h, 0, 0, 0, 0, 0, 0x14 | 1 | 2); printf("NOSIZE and NOMOVE:"); wlog(k0);
    /* the guest rewrites x, y, cx, cy in WM_WINDOWPOSCHANGING: the window gets the rewritten numbers */
    wp_rewrite = 1; wp_new[0] = 77; wp_new[1] = 88; wp_new[2] = 150; wp_new[3] = 160;
    SWP(h, 0, 1, 2, 3, 4, 0x14); wp_rewrite = 0;
    rect_of(fle_imp_GetWindowRect, h); pr("after rewrite");
"#,
    );
    let strip = |s: &str| -> String {
        // keep the message numbers and wParam; the lParam of WM_WINDOWPOSCHANGING/CHANGED/NCCALCSIZE is a stack address, WM_MOVE/WM_SIZE carry numbers
        s.split_whitespace()
            .map(|t| {
                let m = t.split('(').next().unwrap();
                if m == "m46" || m == "m47" || m == "m83" { m.to_string() } else { t.to_string() }
            })
            .collect::<Vec<_>>()
            .join(" ")
    };
    assert_eq!(strip(&l[0]), "move, NOSIZE: m46 m47 m3(0,28001e)");
    assert_eq!(strip(&l[1]), "size, NOMOVE: m46 m83 m47 m5(0,5a0078)");
    assert_eq!(strip(&l[2]), "move and size: m46 m83 m47 m3(0,60005) m5(0,5f0082)");
    // decision 0222 (replaces decision 0132's S-swp-unchanged-geometry for the calls that change nothing): a call that changes neither the size, the position nor the place in the z-order (and has no SWP_SHOWWINDOW /
    // SWP_HIDEWINDOW / SWP_FRAMECHANGED) sends WM_WINDOWPOSCHANGING but no WM_WINDOWPOSCHANGED. Learn WM_WINDOWPOSCHANGED: "Sent to a window whose size, position, or place in the Z order has changed as a result of a call to
    // the SetWindowPos function or another window-management function"; Wine 10.0 (win32u/window.c, fixup_swp_flags + set_window_pos) agrees (harness/api2/win_seq.c 'same position and size'). Built with -DFLE_SWP_ALWAYS_CHANGED the
    // old rows come back: "same numbers: m46 m47", "same numbers, SWP_NOSENDCHANGING: m47", "NOSIZE and NOMOVE: m46 m47".
    assert_eq!(strip(&l[3]), "same numbers: m46");
    assert_eq!(strip(&l[4]), "same numbers, SWP_NOSENDCHANGING:");
    assert_eq!(strip(&l[5]), "NOSIZE and NOMOVE: m46");
    assert_eq!(l[6], "after rewrite 77 88 227 248");
}

#[test]
fn r5_a_destroyed_window_keeps_its_handle_forever_and_a_recreated_window_gets_a_new_one_and_the_destroy_messages_go_parent_first() {
    // R5 (detach-window.md): VCL recreates a window to change its parent; the old HWND must fail (IsWindow 0, SendMessage 0 with 1400) and is never reused. Learn DestroyWindow: "The function first destroys child or owned
    // windows, and then it destroys the parent or owner window"; WM_DESTROY "is sent first to the window being destroyed and then to the child windows (if any) as they are destroyed"; WM_NCDESTROY "is sent after the child
    // windows have been destroyed". S-parentnotify: WM_PARENTNOTIFY is not sent (the page could not be read; Learn's DestroyWindow page says it is sent for a child without WS_EX_NOPARENTNOTIFY).
    let l = lines(
        "r5",
        r#"
    uint64_t top = cw(0x80, "top", POPUP, 0, 0, 50, 50, 0, 0, 0);
    uint64_t kid = cw(0, "kid", CHILD, 0, 0, 10, 10, top, 1, 0);
    uint64_t kk = cw(0, "kk", CHILD, 0, 0, 5, 5, kid, 2, 0);
    int k0 = wn;
    printf("destroy %llu\n", U(callf(fle_imp_DestroyWindow, top, 0, 0, 0)));
    printf("order:"); wlog_h(k0);
    printf("alive top %llu kid %llu kk %llu\n", U(callf(fle_imp_IsWindow, top, 0, 0, 0)), U(callf(fle_imp_IsWindow, kid, 0, 0, 0)), U(callf(fle_imp_IsWindow, kk, 0, 0, 0)));
    SETLE(0);
    printf("send to dead: %llu le %u\n", U(callf(fle_imp_SendMessageW, top, 0x400, 0, 0)), LE());
    uint64_t again = cw(0x80, "top", POPUP, 0, 0, 50, 50, 0, 0, 0);
    printf("recreated differs from all earlier: %d (%llx %llx %llx %llx)\n", again != top && again != kid && again != kk, U(top), U(kid), U(kk), U(again));
    printf("count %llu\n", U(fle_win_count()));
"#,
    );
    assert_eq!(l[0], "destroy 1");
    // the handles end in 4 (top), 8 (kid), c (kk): top: WM_DESTROY (2), then its child kid: WM_DESTROY, its child kk: WM_DESTROY + WM_NCDESTROY (0x82), kid: WM_NCDESTROY, then top: WM_NCDESTROY
    assert_eq!(l[1], "order: 4:2 8:2 c:2 c:82 8:82 4:82");
    assert_eq!(l[2], "alive top 0 kid 0 kk 0");
    assert_eq!(l[3], "send to dead: 0 le 1400");
    assert!(l[4].starts_with("recreated differs from all earlier: 1 "), "{l:?}");
    assert_eq!(l[5], "count 4");
}

#[test]
fn input_state_focus_active_and_capture_are_per_thread_and_the_foreground_window_is_global() {
    // Learn GetFocus: "the window with the keyboard focus for the current thread's message queue"; GetActiveWindow: "attached to the calling thread's message queue"; GetCapture: "associated with the current thread";
    // GetForegroundWindow: one in the system. A second thread has its own queue (AttachThreadInput is not modelled). S-foreground: the foreground window is NULL until a call activated a window (no input exists).
    let top = top_with(
        "static uint64_t w2; static uint64_t seen[8]; static void th_a(void) { seen[0] = callf(fle_imp_GetFocus, 0, 0, 0, 0); seen[1] = callf(fle_imp_GetActiveWindow, 0, 0, 0, 0); seen[2] = callf(fle_imp_GetCapture, 0, 0, 0, 0); seen[3] = callf(fle_imp_GetForegroundWindow, 0, 0, 0, 0); w2 = cw(0x80, \"t2\", POPUP | VIS, 0, 0, 20, 20, 0, 0, 0); seen[4] = callf(fle_imp_SetFocus, w2, 0, 0, 0); seen[5] = callf(fle_imp_GetFocus, 0, 0, 0, 0); seen[6] = callf(fle_imp_GetActiveWindow, 0, 0, 0, 0); R[0] = 0; }",
        &[("0x1000", "th_a")],
    );
    let l = lines_top(
        "input-per-thread",
        &top,
        r#"
    uint64_t w1 = cw(0x80, "t1", POPUP | VIS, 0, 0, 20, 20, 0, 0, 0), th;
    printf("start: focus %llx active %llx capture %llx fg %llx\n", U(callf(fle_imp_GetFocus, 0, 0, 0, 0)), U(callf(fle_imp_GetActiveWindow, 0, 0, 0, 0)), U(callf(fle_imp_GetCapture, 0, 0, 0, 0)), U(callf(fle_imp_GetForegroundWindow, 0, 0, 0, 0)));
    printf("SetFocus returns the previous (none): %llx\n", U(callf(fle_imp_SetFocus, w1, 0, 0, 0)));
    printf("thread 1: focus is w1 %d active is w1 %d\n", callf(fle_imp_GetFocus, 0, 0, 0, 0) == w1, callf(fle_imp_GetActiveWindow, 0, 0, 0, 0) == w1);
    printf("SetCapture returns the previous (none): %llx, capture is w1 %d\n", U(callf(fle_imp_SetCapture, w1, 0, 0, 0)), callf(fle_imp_GetCapture, 0, 0, 0, 0) == w1);
    /* S-foreground: SetFocus and the activation it causes do not make a foreground window (no input exists): NULL until SetForegroundWindow, SetActiveWindow or an activating SetWindowPos names one */
    printf("fg after SetFocus only: %llx\n", U(callf(fle_imp_GetForegroundWindow, 0, 0, 0, 0)));
    printf("SetForegroundWindow(w1): %llu, fg is w1 %d\n", U(callf(fle_imp_SetForegroundWindow, w1, 0, 0, 0)), callf(fle_imp_GetForegroundWindow, 0, 0, 0, 0) == w1);
    th = H_CREATE(0x1000, 0, 0, 0);
    W1(th, INF);
    printf("thread 2 sees: focus %llx active %llx capture %llx\n", U(seen[0]), U(seen[1]), U(seen[2]));
    printf("thread 2 after its own SetFocus: previous %llx, focus is w2 %d, active is w2 %d\n", U(seen[4]), seen[5] == w2, seen[6] == w2);
    printf("thread 1 unchanged: focus is w1 %d active is w1 %d capture is w1 %d\n", callf(fle_imp_GetFocus, 0, 0, 0, 0) == w1, callf(fle_imp_GetActiveWindow, 0, 0, 0, 0) == w1, callf(fle_imp_GetCapture, 0, 0, 0, 0) == w1);
    printf("foreground is global: thread 2 saw w1 %d, still w1 after thread 2's SetFocus %d\n", seen[3] == w1, callf(fle_imp_GetForegroundWindow, 0, 0, 0, 0) == w1);
    /* a thread cannot capture or focus another thread's window: SetCapture 0 and 1408; SetFocus 0 and 5 (S-ownerr: codes chosen from System Error Codes) */
    SETLE(0);
    printf("SetCapture on w2: %llu le %u\n", U(callf(fle_imp_SetCapture, w2, 0, 0, 0)), LE());
    SETLE(0);
    printf("SetFocus on w2: %llu le %u\n", U(callf(fle_imp_SetFocus, w2, 0, 0, 0)), LE());
    SETLE(0);
    printf("DestroyWindow on w2: %llu le %u\n", U(callf(fle_imp_DestroyWindow, w2, 0, 0, 0)), LE());
"#,
    );
    assert_eq!(l[0], "start: focus 0 active 0 capture 0 fg 0");
    assert_eq!(l[1], "SetFocus returns the previous (none): 0");
    assert_eq!(l[2], "thread 1: focus is w1 1 active is w1 1");
    assert_eq!(l[3], "SetCapture returns the previous (none): 0, capture is w1 1");
    assert_eq!(l[4], "fg after SetFocus only: 0");
    assert_eq!(l[5], "SetForegroundWindow(w1): 1, fg is w1 1");
    assert_eq!(l[6], "thread 2 sees: focus 0 active 0 capture 0");
    assert_eq!(l[7], "thread 2 after its own SetFocus: previous 0, focus is w2 1, active is w2 1");
    assert_eq!(l[8], "thread 1 unchanged: focus is w1 1 active is w1 1 capture is w1 1");
    assert_eq!(l[9], "foreground is global: thread 2 saw w1 1, still w1 after thread 2's SetFocus 1");
    assert_eq!(l[10], "SetCapture on w2: 0 le 1408");
    assert_eq!(l[11], "SetFocus on w2: 0 le 5");
    assert_eq!(l[12], "DestroyWindow on w2: 0 le 1408");
}

#[test]
fn focus_messages_go_kill_focus_to_the_old_window_then_set_focus_to_the_new_one_and_a_disabled_window_cannot_take_the_focus() {
    // Learn SetFocus: "This function sends a WM_KILLFOCUS message to the window that loses the keyboard focus and a WM_SETFOCUS message to the window that receives the keyboard focus. It also activates either the window
    // that receives the focus or the parent of the window that receives the focus."; "Extended error ERROR_INVALID_PARAMETER (0x57) means that window is in disabled state."; WM_SETFOCUS/WM_KILLFOCUS wParam: the other window.
    let l = lines(
        "focus-msgs",
        r#"
    uint64_t a = cw(0x80, "a", POPUP | VIS, 0, 0, 20, 20, 0, 0, 0), b = cw(0x80, "b", POPUP | VIS, 0, 0, 20, 20, 0, 0, 0);
    uint64_t dis = cw(0x80, "d", POPUP | VIS | 0x08000000, 0, 0, 20, 20, 0, 0, 0);
    int k0 = wn;
    callf(fle_imp_SetFocus, a, 0, 0, 0);
    k0 = wn;
    callf(fle_imp_SetFocus, b, 0, 0, 0);
    printf("a to b:"); for (int i = k0; i < wn; i++) printf(" %c:%x(%s)", wl[i].h == a ? 'a' : 'b', (unsigned)wl[i].msg, wl[i].wp == 0 ? "0" : wl[i].wp == a ? "a" : wl[i].wp == b ? "b" : "x"); printf("\n");
    SETLE(0);
    printf("SetFocus on a disabled window: %llu le %u, focus is still b %d\n", U(callf(fle_imp_SetFocus, dis, 0, 0, 0)), LE(), callf(fle_imp_GetFocus, 0, 0, 0, 0) == b);
    k0 = wn;
    callf(fle_imp_SetFocus, 0, 0, 0, 0);
    printf("SetFocus(NULL):"); for (int i = k0; i < wn; i++) printf(" %c:%x", wl[i].h == b ? 'b' : '?', (unsigned)wl[i].msg); printf(" focus %llx\n", U(callf(fle_imp_GetFocus, 0, 0, 0, 0)));
    /* disabling the window that has the focus takes the focus away (WM_CANCELMODE then WM_ENABLE, Learn EnableWindow) */
    callf(fle_imp_SetFocus, a, 0, 0, 0);
    k0 = wn;
    callf(fle_imp_EnableWindow, a, 0, 0, 0);
    printf("EnableWindow(a, FALSE):"); for (int i = k0; i < wn; i++) printf(" %x(%llx)", (unsigned)wl[i].msg, U(wl[i].wp)); printf(" focus %llx enabled %llu\n", U(callf(fle_imp_GetFocus, 0, 0, 0, 0)), U(callf(fle_imp_IsWindowEnabled, a, 0, 0, 0)));
"#,
    );
    // S-focusorder (a choice without a source: the page says SetFocus "also activates" and gives no order): the activation messages of the two top-level windows (0x86 WM_NCACTIVATE, 0x6 WM_ACTIVATE; the old window
    // first) come first, then WM_KILLFOCUS (0x8) to the old focus window with the new window as wParam, then WM_SETFOCUS (0x7) to the new one with the old window as wParam
    // decision 0222: the activated window is brought to the top of the z-order first (Learn Window Features: "The system positions the active window at the top of the z-order for windows of the same type"), by the
    // SetWindowPos(HWND_TOP, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE) that the system makes for it: b (below a) gets WM_WINDOWPOSCHANGING (0x46) and CHANGED (0x47) before the activation messages (S-activate-raise-order).
    //
    assert_eq!(l[0], "a to b: b:46(0) b:47(0) a:86(0) a:6(0) b:86(x) b:6(x) a:8(b) b:7(a)");
    assert_eq!(l[1], "SetFocus on a disabled window: 0 le 87, focus is still b 1");
    assert_eq!(l[2], "SetFocus(NULL): b:8 focus 0");
    // decision 0222: the window that loses the focus because it is disabled gets WM_KILLFOCUS (0x8, wParam 0 = no window receives the focus), between WM_CANCELMODE (0x1f) and WM_ENABLE (0xa): Learn Window Features, Disabled
    // Windows ("the window loses the keyboard focus in addition to being disabled. EnableWindow then sets the keyboard focus to NULL") and WM_KILLFOCUS ("Sent to a window immediately before it loses the keyboard focus");
    // Wine 10.0 sends it in that place (harness/api2/win_act.c scenario 11).
    assert_eq!(l[3], "EnableWindow(a, FALSE): 1f(0) 8(0) a(0) focus 0 enabled 0");
}

// ---------------------------------------------------------------------------------------------
// 5. Timers and WM_PAINT made from the virtual clock and the update regions
// ---------------------------------------------------------------------------------------------

#[test]
fn a_timer_posts_wm_timer_through_get_message_when_the_virtual_clock_has_reached_its_deadline_and_a_late_timer_yields_one_message() {
    // Learn SetTimer: WM_TIMER with the timer id as wParam; the elapsed time-out is the period, minimum USER_TIMER_MINIMUM 0x0000000A (10 ms); "WM_TIMER is a low-priority message ... GetMessage and PeekMessage post
    // this message only when no other higher-priority messages are in the thread's message queue" (WM_TIMER page). S-timer (a choice without a source): a timer that is late by several periods yields ONE WM_TIMER and its
    // next deadline is the retrieval time + the period. The virtual clock moves only by Sleep / waits (fle_sys.c): Sleep(n) advances it by n ms when no other thread is ready.
    let l = lines(
        "timer",
        r#"
    uint64_t h = cw(0x80, "t", POPUP, 0, 0, 10, 10, 0, 0, 0);
    uint8_t msg[48];
    uint64_t t0, id;
    id = callf(fle_imp_SetTimer, h, 7, 100, 0);
    printf("SetTimer returns %llu (the id given for a window timer)\n", U(id));
    printf("kill unknown id %llu le %u\n", U(callf(fle_imp_KillTimer, h, 99, 0, 0)), LE());
    printf("peek before the deadline: %llu\n", U(callf5(fle_imp_PeekMessageW, A(msg), 0, 0, 0, 1)));
    SLEEP(99);
    printf("peek at 99 ms: %llu\n", U(callf5(fle_imp_PeekMessageW, A(msg), 0, 0, 0, 1)));
    SLEEP(1);
    printf("peek at 100 ms: %llu msg %x wp %llu hwnd is h %d\n", U(callf5(fle_imp_PeekMessageW, A(msg), 0, 0, 0, 1)), (unsigned)*(uint32_t *)(msg + 8), U(*(uint64_t *)(msg + 16)), *(uint64_t *)msg == h);
    printf("peek again at once: %llu\n", U(callf5(fle_imp_PeekMessageW, A(msg), 0, 0, 0, 1)));
    /* late by 5 periods: ONE message, then none until the next period from the retrieval time */
    SLEEP(500);
    printf("late: first %llu, second %llu\n", U(callf5(fle_imp_PeekMessageW, A(msg), 0, 0, 0, 1)), U(callf5(fle_imp_PeekMessageW, A(msg), 0, 0, 0, 1)));
    SLEEP(99);
    printf("99 ms after the late one: %llu\n", U(callf5(fle_imp_PeekMessageW, A(msg), 0, 0, 0, 1)));
    SLEEP(1);
    printf("100 ms after the late one: %llu\n", U(callf5(fle_imp_PeekMessageW, A(msg), 0, 0, 0, 1)));
    printf("kill %llu, kill again %llu le %u\n", U(callf(fle_imp_KillTimer, h, 7, 0, 0)), U(callf(fle_imp_KillTimer, h, 7, 0, 0)), LE());
    SLEEP(1000);
    printf("after kill: %llu\n", U(callf5(fle_imp_PeekMessageW, A(msg), 0, 0, 0, 1)));
"#,
    );
    assert_eq!(l[0], "SetTimer returns 7 (the id given for a window timer)");
    assert_eq!(l[1], "kill unknown id 0 le 87");
    assert_eq!(l[2], "peek before the deadline: 0");
    assert_eq!(l[3], "peek at 99 ms: 0");
    assert_eq!(l[4], "peek at 100 ms: 1 msg 113 wp 7 hwnd is h 1");
    assert_eq!(l[5], "peek again at once: 0");
    assert_eq!(l[6], "late: first 1, second 0");
    assert_eq!(l[7], "99 ms after the late one: 0");
    assert_eq!(l[8], "100 ms after the late one: 1");
    assert_eq!(l[9], "kill 1, kill again 0 le 87");
    assert_eq!(l[10], "after kill: 0");
}

#[test]
fn a_timer_period_below_10_ms_is_10_ms_a_thread_timer_gets_a_generated_id_and_a_timer_is_gone_with_its_window() {
    // Learn SetTimer: "If uElapse is less than USER_TIMER_MINIMUM (0x0000000A), the timeout is set to USER_TIMER_MINIMUM"; hWnd NULL: the return value identifies the new timer, nIDEvent is ignored when it matches no timer.
    // S-timer-id: the generated ids count from 1 (no value is given). A timer of a destroyed window is removed (a choice without a source: otherwise WM_TIMER would go to a dead window).
    let l = lines(
        "timer-min",
        r#"
    uint8_t msg[48];
    uint64_t h = cw(0x80, "t", POPUP, 0, 0, 10, 10, 0, 0, 0);
    uint64_t id1 = callf(fle_imp_SetTimer, 0, 0, 1, 0), id2 = callf(fle_imp_SetTimer, 0, 0, 1, 0);
    printf("thread timer ids %llu %llu\n", U(id1), U(id2));
    callf(fle_imp_SetTimer, h, 5, 1000, 0);
    SLEEP(9);
    printf("at 9 ms: %llu\n", U(callf5(fle_imp_PeekMessageW, A(msg), 0, 0, 0, 1)));
    SLEEP(1);
    printf("at 10 ms: %llu hwnd %llx msg %x wp %llu\n", U(callf5(fle_imp_PeekMessageW, A(msg), 0, 0, 0, 1)), U(*(uint64_t *)msg), (unsigned)*(uint32_t *)(msg + 8), U(*(uint64_t *)(msg + 16)));
    printf("second thread timer: %llu\n", U(callf5(fle_imp_PeekMessageW, A(msg), 0, 0, 0, 1)));
    printf("kill the two thread timers: %llu %llu\n", U(callf(fle_imp_KillTimer, 0, id1, 0, 0)), U(callf(fle_imp_KillTimer, 0, id2, 0, 0)));
    callf(fle_imp_DestroyWindow, h, 0, 0, 0);
    SLEEP(2000);
    printf("after destroying the window: %llu\n", U(callf5(fle_imp_PeekMessageW, A(msg), 0, 0, 0, 1)));
"#,
    );
    assert_eq!(l[0], "thread timer ids 1 2");
    assert_eq!(l[1], "at 9 ms: 0");
    assert_eq!(l[2], "at 10 ms: 1 hwnd 0 msg 113 wp 1");
    assert_eq!(l[3], "second thread timer: 1");
    assert_eq!(l[4], "kill the two thread timers: 1 1");
    assert_eq!(l[5], "after destroying the window: 0");
}

#[test]
fn an_update_region_makes_wm_paint_after_the_posted_messages_and_begin_paint_validates_it() {
    // Learn InvalidateRect: the rectangle is added to the update region; "The system sends a WM_PAINT message to a window whenever its update region is not empty and there are no other messages in the application queue for
    // that window"; GetMessage order: sent, posted, input, sent again, WM_PAINT, WM_TIMER. BeginPaint: "fills the PAINTSTRUCT" (hdc, fErase, rcPaint) and validates the update region; EndPaint "always returns nonzero".
    // S-paintdc: the DC of BeginPaint is a screen DC from GetDC(NULL). S-updregion: the update region is kept as one bounding rectangle.
    let l = lines(
        "paint",
        r#"
    uint8_t msg[48], ps[72];
    uint64_t h = cw(0x80, "p", POPUP, 10, 10, 200, 100, 0, 0, 0), dc;
    int32_t r[4] = { 5, 6, 50, 60 }, got[4];
    memset(ps, 0xEE, sizeof ps);
    callf5(fle_imp_PeekMessageW, A(msg), 0, 0, 0, 1);
    printf("GetUpdateRect of a hidden window: %llu\n", U(callf(fle_imp_GetUpdateRect, h, A(got), 0, 0)));
    /* S-showpaint (a choice without a source): ShowWindow of a hidden window gives it its whole client area to paint (the first WM_PAINT after the show); a window created with WS_VISIBLE has none until something invalidates it */
    callf(fle_imp_ShowWindow, h, 5, 0, 0);
    printf("GetUpdateRect after show: %llu\n", U(callf(fle_imp_GetUpdateRect, h, A(got), 0, 0)));
    printf("rect %d %d %d %d\n", got[0], got[1], got[2], got[3]);
    callf(fle_imp_ValidateRect, h, 0, 0, 0);
    printf("after ValidateRect(NULL): %llu\n", U(callf(fle_imp_GetUpdateRect, h, A(got), 0, 0)));
    printf("InvalidateRect: %llu\n", U(callf(fle_imp_InvalidateRect, h, A(r), 1, 0)));
    callf(fle_imp_PostMessageW, h, 0x8001, 1, 2);
    printf("first the posted message: %llu msg %x\n", U(callf5(fle_imp_PeekMessageW, A(msg), 0, 0, 0, 1)), (unsigned)*(uint32_t *)(msg + 8));
    printf("then WM_PAINT: %llu msg %x hwnd is h %d\n", U(callf5(fle_imp_PeekMessageW, A(msg), 0, 0, 0, 1)), (unsigned)*(uint32_t *)(msg + 8), *(uint64_t *)msg == h);
    printf("WM_PAINT stays until it is painted: %llu\n", U(callf5(fle_imp_PeekMessageW, A(msg), 0, 0, 0, 1)));
    dc = callf(fle_imp_BeginPaint, h, A(ps), 0, 0);
    printf("BeginPaint returns the hdc of PAINTSTRUCT %d, rcPaint %d %d %d %d\n", dc == *(uint64_t *)ps && dc != 0, ((int32_t *)(ps + 12))[0], ((int32_t *)(ps + 12))[1], ((int32_t *)(ps + 12))[2], ((int32_t *)(ps + 12))[3]);
    printf("EndPaint: %llu\n", U(callf(fle_imp_EndPaint, h, A(ps), 0, 0)));
    printf("nothing left to paint: %llu\n", U(callf5(fle_imp_PeekMessageW, A(msg), 0, 0, 0, 1)));
    /* two invalidations are one bounding rectangle */
    { int32_t r1[4] = { 0, 0, 10, 10 }, r2[4] = { 100, 50, 120, 70 };
      callf(fle_imp_InvalidateRect, h, A(r1), 0, 0); callf(fle_imp_InvalidateRect, h, A(r2), 0, 0);
      callf(fle_imp_GetUpdateRect, h, A(got), 0, 0); printf("bounding %d %d %d %d\n", got[0], got[1], got[2], got[3]); }
"#,
    );
    assert_eq!(l[0], "GetUpdateRect of a hidden window: 0");
    assert_eq!(l[1], "GetUpdateRect after show: 1");
    assert_eq!(l[2], "rect 0 0 200 100");
    assert_eq!(l[3], "after ValidateRect(NULL): 0");
    assert_eq!(l[4], "InvalidateRect: 1");
    assert_eq!(l[5], "first the posted message: 1 msg 8001");
    assert_eq!(l[6], "then WM_PAINT: 1 msg f hwnd is h 1");
    assert_eq!(l[7], "WM_PAINT stays until it is painted: 1");
    assert_eq!(l[8], "BeginPaint returns the hdc of PAINTSTRUCT 1, rcPaint 5 6 50 60");
    assert_eq!(l[9], "EndPaint: 1");
    assert_eq!(l[10], "nothing left to paint: 0");
    assert_eq!(l[11], "bounding 0 0 120 70");
}

// ---------------------------------------------------------------------------------------------
// 6. Properties (R7), scroll bars, DeferWindowPos, placement, class name, menus
// ---------------------------------------------------------------------------------------------

#[test]
fn r7_the_property_list_is_per_window_a_set_replaces_a_get_reads_and_a_remove_returns_the_data() {
    // Learn SetPropW: adds the string/atom and the data handle to the list; GetPropW: "the associated data handle. Otherwise, the return value is NULL"; RemovePropW: "The return value identifies the specified data. If the data
    // cannot be found in the specified property list, the return value is NULL". S-prop (choices without a source): a second SetPropW of the same key replaces the data; string keys compare ASCII-case-insensitively as the global atom
    // table does; an atom key and a string key never match each other (GlobalAddAtom is not answered).
    let l = lines(
        "r7-prop",
        r##"
    uint64_t a = cw(0x80, "a", POPUP, 0, 0, 5, 5, 0, 0, 0), b = cw(0x80, "b", POPUP, 0, 0, 5, 5, 0, 0, 0);
    printf("get before: %llu\n", U(callf(fle_imp_GetPropW, a, S("Delphi"), 0, 0)));
    printf("set: %llu\n", U(callf(fle_imp_SetPropW, a, S("Delphi"), 0x1111, 0)));
    printf("get: %llx, on the other window %llx\n", U(callf(fle_imp_GetPropW, a, S("Delphi"), 0, 0)), U(callf(fle_imp_GetPropW, b, S("Delphi"), 0, 0)));
    printf("case-insensitive: %llx\n", U(callf(fle_imp_GetPropW, a, S("DELPHI"), 0, 0)));
    printf("replace: %llu then %llx\n", U(callf(fle_imp_SetPropW, a, S("delphi"), 0x2222, 0)), U(callf(fle_imp_GetPropW, a, S("Delphi"), 0, 0)));
    printf("atom key set %llu, get by atom %llx, by a string %llx\n", U(callf(fle_imp_SetPropW, a, 0xC123, 0x3333, 0)), U(callf(fle_imp_GetPropW, a, 0xC123, 0, 0)), U(callf(fle_imp_GetPropW, a, S("#"), 0, 0)));
    printf("remove: %llx, again %llx, get %llx\n", U(callf(fle_imp_RemovePropW, a, S("Delphi"), 0, 0)), U(callf(fle_imp_RemovePropW, a, S("Delphi"), 0, 0)), U(callf(fle_imp_GetPropW, a, S("Delphi"), 0, 0)));
    printf("atom key kept: %llx\n", U(callf(fle_imp_GetPropW, a, 0xC123, 0, 0)));
    callf(fle_imp_DestroyWindow, a, 0, 0, 0);
    SETLE(0);
    printf("set on a dead window: %llu le %u\n", U(callf(fle_imp_SetPropW, a, S("Delphi"), 1, 0)), LE());
"##,
    );
    assert_eq!(l[0], "get before: 0");
    assert_eq!(l[1], "set: 1");
    assert_eq!(l[2], "get: 1111, on the other window 0");
    assert_eq!(l[3], "case-insensitive: 1111");
    assert_eq!(l[4], "replace: 1 then 2222");
    assert_eq!(l[5], "atom key set 1, get by atom 3333, by a string 0");
    assert_eq!(l[6], "remove: 2222, again 0, get 0");
    assert_eq!(l[7], "atom key kept: 3333");
    assert_eq!(l[8], "set on a dead window: 0 le 1400");
}

#[test]
fn the_class_name_the_window_text_the_unicode_flag_and_the_window_font_are_answered_as_the_pages_say() {
    // Learn GetClassNameW: the class name, truncated to nMaxCount-1 characters, the count without the NUL; IsWindowUnicode: nonzero for a window created with a Unicode class; WM_SETTEXT / WM_GETTEXT / WM_GETTEXTLENGTH by
    // DefWindowProcW ("The DefWindowProc function sets and displays the window text"); WM_SETFONT keeps the handle and WM_GETFONT returns it (S-setfont: the page describes controls; a plain window keeps the number).
    let l = lines(
        "classname",
        r#"
    uint64_t h = cw(0x80, "Hello", POPUP, 0, 0, 5, 5, 0, 0, 0);
    uint16_t out[40];
    memset(out, 0xEE, sizeof out);
    printf("GetClassNameW: %llu\n", U(callf(fle_imp_GetClassNameW, h, A(out), 40, 0)));
    { char s[40]; int i; for (i = 0; i < 12; i++) s[i] = (char)out[i]; s[12] = 0; printf("name %s, NUL at 12: %d\n", s, out[12] == 0); }
    printf("truncated to 5: %llu, units %c%c%c%c NUL %d\n", U(callf(fle_imp_GetClassNameW, h, A(out), 5, 0)), (char)out[0], (char)out[1], (char)out[2], (char)out[3], out[4] == 0);
    printf("IsWindowUnicode %llu, dead/unknown %llu\n", U(callf(fle_imp_IsWindowUnicode, h, 0, 0, 0)), U(callf(fle_imp_IsWindowUnicode, 0x7F99FFF0, 0, 0, 0)));
    printf("WM_GETTEXTLENGTH %llu\n", U(callf(fle_imp_SendMessageW, h, 0xE, 0, 0)));
    memset(out, 0xEE, sizeof out);
    printf("WM_GETTEXT(20): %llu, units %c%c%c%c%c NUL %d\n", U(callf(fle_imp_SendMessageW, h, 0xD, 20, A(out))), (char)out[0], (char)out[1], (char)out[2], (char)out[3], (char)out[4], out[5] == 0);
    printf("WM_GETTEXT(3): %llu NUL at 2 %d\n", U(callf(fle_imp_SendMessageW, h, 0xD, 3, A(out))), out[2] == 0);
    printf("SetWindowTextW: %llu, GetWindowTextW: %llu\n", U(callf(fle_imp_SetWindowTextW, h, S("Renamed"), 0, 0)), U(callf(fle_imp_GetWindowTextW, h, A(out), 40, 0)));
    printf("WM_GETFONT before %llx\n", U(callf(fle_imp_SendMessageW, h, 0x31, 0, 0)));
    callf(fle_imp_SendMessageW, h, 0x30, 0x7FBA0042, 1);
    printf("WM_GETFONT after %llx\n", U(callf(fle_imp_SendMessageW, h, 0x31, 0, 0)));
"#,
    );
    assert_eq!(l[0], "GetClassNameW: 12");
    assert_eq!(l[1], "name TPUtilWindow, NUL at 12: 1");
    assert_eq!(l[2], "truncated to 5: 4, units TPUt NUL 1");
    assert_eq!(l[3], "IsWindowUnicode 1, dead/unknown 0");
    assert_eq!(l[4], "WM_GETTEXTLENGTH 5");
    assert_eq!(l[5], "WM_GETTEXT(20): 5, units Hello NUL 1");
    assert_eq!(l[6], "WM_GETTEXT(3): 2 NUL at 2 1");
    assert_eq!(l[7], "SetWindowTextW: 1, GetWindowTextW: 7");
    assert_eq!(l[8], "WM_GETFONT before 0");
    assert_eq!(l[9], "WM_GETFONT after 7fba0042");
}

#[test]
fn scroll_bars_keep_a_scrollinfo_with_the_range_checking_of_the_page_and_the_flat_ones_need_initializeflatsb() {
    // Learn SetScrollInfo: "The nPage member must specify a value from 0 to nMax - nMin + 1. The nPos member must specify a value between nMin and nMax - max(nPage - 1, 0). If either value is beyond its range, the function sets it
    // to a value that is just within the range."; the return value is the current position. Learn InitializeFlatSB: must be called before the other flat scroll bar functions. Comctl32 6.x does not support flat scroll bars: the
    // model answers success anyway (S-flatsb), and shares the standard bar's SCROLLINFO (S-sbshare); nTrackPos is the position (S-sbtrack).
    let l = lines(
        "scroll",
        r#"
    uint64_t h = cw(0x80, "s", POPUP, 0, 0, 100, 100, 0, 0, 0);
    uint32_t si[7], out[7];
    #define SIF_RANGE 1
    #define SIF_PAGE 2
    #define SIF_POS 4
    #define SIF_TRACK 0x10
    si[0] = 28; si[1] = SIF_RANGE | SIF_PAGE | SIF_POS; si[2] = 0; si[3] = 100; si[4] = 10; si[5] = 500; si[6] = 0;
    printf("SetScrollInfo pos 500: returns %d\n", (int)callf(fle_imp_SetScrollInfo, h, 1, A(si), 1));
    memset(out, 0, sizeof out); out[0] = 28; out[1] = SIF_RANGE | SIF_PAGE | SIF_POS | SIF_TRACK;
    printf("GetScrollInfo: %llu min %d max %d page %u pos %d track %d\n", U(callf(fle_imp_GetScrollInfo, h, 1, A(out), 0)), (int)out[2], (int)out[3], out[4], (int)out[5], (int)out[6]);
    si[1] = SIF_PAGE; si[4] = 1000;
    callf(fle_imp_SetScrollInfo, h, 1, A(si), 1);
    out[1] = SIF_PAGE | SIF_POS; callf(fle_imp_GetScrollInfo, h, 1, A(out), 0);
    printf("page 1000 is limited to nMax - nMin + 1: page %u pos %d\n", out[4], (int)out[5]);
    si[1] = SIF_POS; si[5] = (uint32_t)-7;
    printf("pos -7: returns %d\n", (int)callf(fle_imp_SetScrollInfo, h, 0, A(si), 1));
    out[0] = 28; out[1] = SIF_RANGE | SIF_PAGE | SIF_POS;
    callf(fle_imp_GetScrollInfo, h, 0, A(out), 0);
    printf("horizontal bar is separate: min %d max %d page %u pos %d\n", (int)out[2], (int)out[3], out[4], (int)out[5]);
    /* flat scroll bars */
    printf("FlatSB before Initialize: GetScrollPos %llu, SetScrollPos %llu\n", U(callf(fle_imp_FlatSB_GetScrollPos, h, 1, 0, 0)), U(callf(fle_imp_FlatSB_SetScrollPos, h, 1, 3, 1)));
    printf("InitializeFlatSB %llu\n", U(callf(fle_imp_InitializeFlatSB, h, 0, 0, 0)));
    printf("FlatSB_GetScrollPos shares the standard bar: %llu\n", U(callf(fle_imp_FlatSB_GetScrollPos, h, 1, 0, 0)));
    si[1] = SIF_RANGE | SIF_PAGE | SIF_POS; si[2] = 0; si[3] = 50; si[4] = 5; si[5] = 20;
    printf("FlatSB_SetScrollInfo returns the previous position: %d\n", (int)callf(fle_imp_FlatSB_SetScrollInfo, h, 1, A(si), 1));
    printf("FlatSB_SetScrollPos returns the previous position: %d, now %d\n", (int)callf(fle_imp_FlatSB_SetScrollPos, h, 1, 30, 1), (int)callf(fle_imp_FlatSB_GetScrollPos, h, 1, 0, 0));
    printf("FlatSB_SetScrollProp(WSB_PROP_VSTYLE 0x100, 1): %llu\n", U(callf(fle_imp_FlatSB_SetScrollProp, h, 0x100, 1, 1)));
"#,
    );
    // the upper limit of nPos is nMax - max(nPage - 1, 0) = 100 - 9 = 91 (Learn SetScrollInfo)
    assert_eq!(l[0], "SetScrollInfo pos 500: returns 91");
    assert_eq!(l[1], "GetScrollInfo: 1 min 0 max 100 page 10 pos 91 track 91");
    assert_eq!(l[2], "page 1000 is limited to nMax - nMin + 1: page 101 pos 0");
    assert_eq!(l[3], "pos -7: returns 0");
    assert_eq!(l[4], "horizontal bar is separate: min 0 max 0 page 0 pos 0");
    assert_eq!(l[5], "FlatSB before Initialize: GetScrollPos 0, SetScrollPos 0");
    assert_eq!(l[6], "InitializeFlatSB 1");
    assert_eq!(l[7], "FlatSB_GetScrollPos shares the standard bar: 0");
    assert_eq!(l[8], "FlatSB_SetScrollInfo returns the previous position: 0");
    assert_eq!(l[9], "FlatSB_SetScrollPos returns the previous position: 20, now 30");
    assert_eq!(l[10], "FlatSB_SetScrollProp(WSB_PROP_VSTYLE 0x100, 1): 1");
}

#[test]
fn show_scroll_bar_keeps_the_shown_state_of_the_standard_bars_and_stops_for_a_scroll_bar_control() {
    // decision 0132 (wall 20, PROVISIONAL). Learn (ShowScrollBar): "shows or hides the specified scroll bar"; wBar SB_BOTH (3) = "a window's standard horizontal and vertical scroll bars", SB_HORZ (0), SB_VERT (1), SB_CTL (2) = a scroll
    // bar control; bShow "If this parameter is TRUE, the scroll bar is shown; otherwise, it is hidden"; "If the function succeeds, the return value is nonzero. If the function fails, the return value is zero. To get extended error
    // information, call GetLastError". The state is kept per bar (S-showscrollbar-state: the window's style bits and client area are not changed; the SCROLLINFO of the bar is not touched). An invalid window is 0 with 1400
    // (the answer of SetScrollInfo here).
    let l = lines(
        "showscroll",
        r#"
    uint64_t h = cw(0x80, "s", POPUP, 0, 0, 100, 100, 0, 0, 0);
    uint32_t si[7], out[7];
    si[0] = 28; si[1] = 7; si[2] = 0; si[3] = 100; si[4] = 10; si[5] = 20; si[6] = 0;
    callf(fle_imp_SetScrollInfo, h, 1, A(si), 1);
    printf("initially hidden: %d %d\n", fle_win_scrollbar_shown(h, 0), fle_win_scrollbar_shown(h, 1));
    printf("show vertical: %llu -> %d %d\n", U(callf(fle_imp_ShowScrollBar, h, 1, 1, 0)), fle_win_scrollbar_shown(h, 0), fle_win_scrollbar_shown(h, 1));
    printf("hide vertical: %llu -> %d %d\n", U(callf(fle_imp_ShowScrollBar, h, 1, 0, 0)), fle_win_scrollbar_shown(h, 0), fle_win_scrollbar_shown(h, 1));
    printf("show both: %llu -> %d %d\n", U(callf(fle_imp_ShowScrollBar, h, 3, 1, 0)), fle_win_scrollbar_shown(h, 0), fle_win_scrollbar_shown(h, 1));
    printf("hide horizontal (bShow 0): %llu -> %d %d\n", U(callf(fle_imp_ShowScrollBar, h, 0, 0, 0)), fle_win_scrollbar_shown(h, 0), fle_win_scrollbar_shown(h, 1));
    printf("bShow is any nonzero 32-bit value: %llu -> %d\n", U(callf(fle_imp_ShowScrollBar, h, 0, 0xFFFFFFFF00000002ull, 0)), fle_win_scrollbar_shown(h, 0));
    printf("a high-order half alone is zero: %llu -> %d\n", U(callf(fle_imp_ShowScrollBar, h, 0, 0x100000000ull, 0)), fle_win_scrollbar_shown(h, 0));
    memset(out, 0, sizeof out); out[0] = 28; out[1] = 7;
    callf(fle_imp_GetScrollInfo, h, 1, A(out), 0);
    printf("the scroll info is untouched: min %d max %d page %u pos %d\n", (int)out[2], (int)out[3], out[4], (int)out[5]);
    SETLE(0);
    printf("a window that is not one: %llu le %u\n", U(callf(fle_imp_ShowScrollBar, 0x1234, 0, 1, 0)), LE());
"#,
    );
    assert_eq!(
        l,
        [
            "initially hidden: 0 0",
            "show vertical: 1 -> 0 1",
            "hide vertical: 1 -> 0 0",
            "show both: 1 -> 1 1",
            "hide horizontal (bShow 0): 1 -> 0 1",
            "bShow is any nonzero 32-bit value: 1 -> 1",
            "a high-order half alone is zero: 1 -> 0",
            "the scroll info is untouched: min 0 max 100 page 10 pos 20",
            "a window that is not one: 0 le 1400"
        ]
    );
    aborts("showscroll-ctl", "uint64_t h = cw(0x80, \"s\", POPUP, 0, 0, 100, 100, 0, 0, 0); callf(fle_imp_ShowScrollBar, h, 2, 1, 0);", "ShowScrollBar", "SB_CTL");
    aborts("showscroll-bad", "uint64_t h = cw(0x80, \"s\", POPUP, 0, 0, 100, 100, 0, 0, 0); callf(fle_imp_ShowScrollBar, h, 4, 1, 0);", "ShowScrollBar", "SB_BOTH (3)");
}

#[test]
fn defer_window_pos_collects_the_positions_returns_the_same_handle_and_end_applies_them_in_order_with_the_messages() {
    // Learn BeginDeferWindowPos / DeferWindowPos / EndDeferWindowPos: "All windows in a multiple-window-position structure must have the same parent"; "The handle returned by this function may differ from the handle passed";
    // EndDeferWindowPos "sends the WM_WINDOWPOSCHANGING and WM_WINDOWPOSCHANGED messages to each window identified in the internal structure". S-dwp (a choice without a source): the handle of the k-th structure is 0x7F970000 +
    // 4 * (k + 1), DeferWindowPos returns the same handle, and the windows are applied in the order they were deferred. Error codes (87 for a different parent, 6 for a bad handle) come from System Error Codes.
    let l = lines(
        "dwp",
        r#"
    uint64_t top = cw(0x80, "top", POPUP, 0, 0, 500, 500, 0, 0, 0);
    uint64_t a = cw(0, "a", CHILD, 0, 0, 10, 10, top, 1, 0), b = cw(0, "b", CHILD, 0, 0, 10, 10, top, 2, 0), other = cw(0x80, "o", POPUP, 0, 0, 10, 10, 0, 0, 0);
    uint64_t d = callf(fle_imp_BeginDeferWindowPos, 3, 0, 0, 0), d2;
    printf("Begin %llx\n", U(d));
    d2 = callf8(fle_imp_DeferWindowPos, d, a, 0, 100, 110, 20, 30, 0x14);
    printf("Defer a returns the same handle %d\n", d2 == d);
    callf8(fle_imp_DeferWindowPos, d, b, 0, 200, 210, 40, 50, 0x14);
    SETLE(0);
    printf("Defer a window with another parent: %llu le %u\n", U(callf8(fle_imp_DeferWindowPos, d, other, 0, 1, 1, 1, 1, 0x14)), LE());
    rect_of(fle_imp_GetWindowRect, a); pr("a before End");
    int k0 = wn;
    printf("End: %llu\n", U(callf(fle_imp_EndDeferWindowPos, d, 0, 0, 0)));
    printf("messages (hwnd:msg):"); for (int i = k0; i < wn; i++) if (wl[i].msg == 0x46 || wl[i].msg == 0x47) printf(" %c:%x", wl[i].h == a ? 'a' : wl[i].h == b ? 'b' : '?', (unsigned)wl[i].msg); printf("\n");
    rect_of(fle_imp_GetWindowRect, a); pr("a after End");
    rect_of(fle_imp_GetWindowRect, b); pr("b after End");
    SETLE(0);
    printf("End again (the structure is freed): %llu le %u\n", U(callf(fle_imp_EndDeferWindowPos, d, 0, 0, 0)), LE());
    printf("Begin again reuses the first handle: %d\n", callf(fle_imp_BeginDeferWindowPos, 1, 0, 0, 0) == d);
"#,
    );
    assert_eq!(l[0], "Begin 7f970004");
    assert_eq!(l[1], "Defer a returns the same handle 1");
    assert_eq!(l[2], "Defer a window with another parent: 0 le 87");
    assert_eq!(l[3], "a before End 0 0 10 10");
    assert_eq!(l[4], "End: 1");
    assert_eq!(l[5], "messages (hwnd:msg): a:46 a:47 b:46 b:47");
    assert_eq!(l[6], "a after End 100 110 120 140");
    assert_eq!(l[7], "b after End 200 210 240 260");
    assert_eq!(l[8], "End again (the structure is freed): 0 le 6");
    assert_eq!(l[9], "Begin again reuses the first handle: 1");
}

#[test]
fn get_window_placement_gives_the_restored_rectangle_and_the_show_command_and_the_menu_calls_keep_a_handle() {
    // Learn GetWindowPlacement: "The flags member ... retrieved by this function is always zero"; showCmd SW_SHOWMAXIMIZED / SW_SHOWMINIMIZED / SW_SHOWNORMAL; "fails if lpwndpl->length is not set correctly"; rcNormalPosition is the
    // restored position. Decision 0267 (alpha10b): the length is sizeof(WINDOWPLACEMENT) = 44 (the SDK header has rcDevice only for _MAC; FL passes 44); 60 (Learn's member list with rcDevice) fails like any other wrong length. S-placement: ptMinPosition and ptMaxPosition are (-1, -1) (the page says nothing for a window that never set them). SetMenu / GetMenu / DrawMenuBar: a NULL menu is the only one that is not a stop.
    let l = lines(
        "placement",
        r#"
    uint64_t h = cw(0x80, "p", POPUP | VIS, 10, 20, 300, 200, 0, 0, 0);
    uint32_t wpl[15];
    int32_t *r = (int32_t *)wpl;
    memset(wpl, 0, sizeof wpl); wpl[0] = 44;
    printf("normal: %llu flags %u show %u min %d %d max %d %d normal %d %d %d %d\n", U(callf(fle_imp_GetWindowPlacement, h, A(wpl), 0, 0)), wpl[1], wpl[2], r[3], r[4], r[5], r[6], r[7], r[8], r[9], r[10]);
    callf(fle_imp_ShowWindow, h, 3, 0, 0);
    SWP(h, 0, 0, 0, 1920, 1080, 0x14);
    memset(wpl, 0, sizeof wpl); wpl[0] = 44;
    callf(fle_imp_GetWindowPlacement, h, A(wpl), 0, 0);
    printf("maximized: show %u normal %d %d %d %d\n", wpl[2], r[7], r[8], r[9], r[10]);
    callf(fle_imp_ShowWindow, h, 2, 0, 0);
    memset(wpl, 0, sizeof wpl); wpl[0] = 44;
    callf(fle_imp_GetWindowPlacement, h, A(wpl), 0, 0);
    printf("minimized: show %u\n", wpl[2]);
    memset(wpl, 0, sizeof wpl); wpl[0] = 60; SETLE(0);
    printf("wrong length: %llu le %u\n", U(callf(fle_imp_GetWindowPlacement, h, A(wpl), 0, 0)), LE());
    printf("GetMenu %llu, SetMenu(NULL) %llu, DrawMenuBar %llu\n", U(callf(fle_imp_GetMenu, h, 0, 0, 0)), U(callf(fle_imp_SetMenu, h, 0, 0, 0)), U(callf(fle_imp_DrawMenuBar, h, 0, 0, 0)));
"#,
    );
    assert_eq!(l[0], "normal: 1 flags 0 show 1 min -1 -1 max -1 -1 normal 10 20 310 220");
    assert_eq!(l[2], "minimized: show 2");
    assert_eq!(l[3], "wrong length: 0 le 87");
    assert_eq!(l[4], "GetMenu 0, SetMenu(NULL) 1, DrawMenuBar 1");
    // a maximized window keeps its restored rectangle: the move to 1920 x 1080 while maximized is not the restored position
    assert!(l[1].starts_with("maximized: show 3 normal "), "{l:?}");
}

// ---------------------------------------------------------------------------------------------
// 7. VirtualQuery, GetFileAttributesW(""), FreeResource, GetSysColor, the region objects
// ---------------------------------------------------------------------------------------------

/// MEMORY_BASIC_INFORMATION on x64 (Learn MEMORY_BASIC_INFORMATION; offsets from the member types): BaseAddress 0, AllocationBase 8, AllocationProtect 16, RegionSize 24, State 32, Protect 36, Type 40 = 48 bytes.
const MBI: &str = r##"
static uint8_t mbi[48];
static void qv(uint64_t a)
{
    uint64_t r = callf(fle_imp_VirtualQuery, a, A(mbi), 48, 0);
    printf("query %llx -> %llu base %llx allocbase %llx allocprot %x size %llx state %x prot %x type %x\n", U(a), U(r), U(*(uint64_t *)mbi), U(*(uint64_t *)(mbi + 8)), *(uint32_t *)(mbi + 16), U(*(uint64_t *)(mbi + 24)), *(uint32_t *)(mbi + 32), *(uint32_t *)(mbi + 36), *(uint32_t *)(mbi + 40));
}
"##;

#[test]
fn virtual_query_answers_an_image_from_its_section_table_a_registered_range_and_free_for_everything_else() {
    // Learn VirtualQuery / MEMORY_BASIC_INFORMATION: State MEM_COMMIT 0x1000, MEM_FREE 0x10000; Type MEM_IMAGE 0x1000000, MEM_PRIVATE 0x20000; AllocationBase the base of the allocation; Protect from Memory Protection Constants
    // (PAGE_READONLY 2, PAGE_READWRITE 4, PAGE_EXECUTE_READ 0x20, PAGE_NOACCESS 1); for a free region "the information in the AllocationBase, AllocationProtect, Protect, and Type parameters is undefined" (here 0). The protection of
    // a section page comes from IMAGE_SCN_MEM_* (Learn: IMAGE_SECTION_HEADER.Characteristics gives the names). S-vq-headers: the headers are PAGE_READONLY; S-vq-heap: a host heap block is not tracked, so MEM_FREE.
    let l = lines_top(
        "vq",
        &top_with(&format!("{MBI}\nstatic uint8_t img[0x4000] __attribute__((aligned(4096)));\nstatic void mkimg(void) {{ uint32_t v; uint16_t w; memset(img, 0, sizeof img); w = 0x5A4D; memcpy(img, &w, 2); v = 0x80; memcpy(img + 0x3C, &v, 4); v = 0x4550; memcpy(img + 0x80, &v, 4); w = 2; memcpy(img + 0x86, &w, 2); w = 240; memcpy(img + 0x94, &w, 2); v = 0x4000; memcpy(img + 0x80 + 24 + 56, &v, 4); v = 0x400; memcpy(img + 0x80 + 24 + 60, &v, 4);\n  {{ uint8_t *s = img + 0x80 + 24 + 240; v = 0x1000; memcpy(s + 8, &v, 4); memcpy(s + 12, &v, 4); v = 0x60000020; memcpy(s + 36, &v, 4); s += 40; v = 0x1000; memcpy(s + 8, &v, 4); v = 0x2000; memcpy(s + 12, &v, 4); v = 0xC0000040; memcpy(s + 36, &v, 4); }} }}"), &[]),
        r#"
    mkimg();
    fle_vmap_register_image(A(img));
    { static uint8_t rng[0x3000] __attribute__((aligned(4096))); fle_vmap_register_range(A(rng), sizeof rng, 4, 0x20000); printf("range base %llx\n", U(A(rng))); qv(A(rng) + 0x1234); }
    qv(A(img));
    qv(A(img) + 0x1000 + 5);
    qv(A(img) + 0x2000);
    qv(A(img) + 0x3000);
    qv(0x10000);
    qv(0x20000000ull);
    qv(0x7FFFFFFF0000ull);
    printf("beyond the user address space: %llu le %u\n", U(callf(fle_imp_VirtualQuery, 0x800000000000ull, A(mbi), 48, 0)), LE());
"#,
    );
    let range_base = u64::from_str_radix(l[0].trim_start_matches("range base "), 16).unwrap();
    assert_eq!(l[1], format!("query {:x} -> 48 base {:x} allocbase {:x} allocprot 4 size 2000 state 1000 prot 4 type 20000", range_base + 0x1234, range_base + 0x1000, range_base));
    // "query <a> -> <n> base <b> allocbase <ab> allocprot <ap> size <s> state <st> prot <p> type <t>" as a map
    let f = |s: &str| -> std::collections::HashMap<String, String> {
        let t: Vec<&str> = s.split_whitespace().collect();
        let mut m = std::collections::HashMap::new();
        m.insert("query".to_string(), t[1].to_string());
        for k in ["base", "allocbase", "allocprot", "size", "state", "prot", "type"] {
            let at = t.iter().position(|x| *x == k).unwrap();
            m.insert(k.to_string(), t[at + 1].to_string());
        }
        m
    };
    // the image: headers (PAGE_READONLY 2), .text (PAGE_EXECUTE_READ 0x20: SCN_MEM_EXECUTE | READ), .data (PAGE_READWRITE 4)
    let hdr = f(&l[2]);
    let ib = hdr["query"].clone();
    assert_eq!((hdr["base"].as_str(), hdr["allocbase"].as_str(), hdr["allocprot"].as_str(), hdr["size"].as_str(), hdr["state"].as_str(), hdr["prot"].as_str(), hdr["type"].as_str()), (ib.as_str(), ib.as_str(), "2", "1000", "1000", "2", "1000000"));
    let text = f(&l[3]);
    assert_eq!((text["allocbase"].as_str(), text["allocprot"].as_str(), text["size"].as_str(), text["state"].as_str(), text["prot"].as_str(), text["type"].as_str()), (ib.as_str(), "2", "1000", "1000", "20", "1000000"), "{}", l[3]);
    let data = f(&l[4]);
    assert_eq!((data["allocbase"].as_str(), data["allocprot"].as_str(), data["size"].as_str(), data["state"].as_str(), data["prot"].as_str(), data["type"].as_str()), (ib.as_str(), "2", "1000", "1000", "4", "1000000"), "{}", l[4]);   // .data: VirtualSize 0x1000 (the section header of the test image)
    // a page of the image that no section covers (the page at +0x3000: S-vq-gap): committed, PAGE_NOACCESS (1), still MEM_IMAGE
    let gap = f(&l[5]);
    assert_eq!((gap["allocbase"].as_str(), gap["state"].as_str(), gap["prot"].as_str(), gap["type"].as_str()), (ib.as_str(), "1000", "1", "1000000"), "{}", l[5]);
    // three addresses that nothing tracks (a low one, one in the middle and one near the top of the user address space): free (state 0x10000), no allocation base, protect 0, type 0
    for i in [6, 7, 8] {
        let q = f(&l[i]);
        assert_eq!((q["allocbase"].as_str(), q["allocprot"].as_str(), q["state"].as_str(), q["prot"].as_str(), q["type"].as_str()), ("0", "0", "10000", "0", "0"), "{}", l[i]);
    }
    assert_eq!(l[9], "beyond the user address space: 0 le 87");
}

#[test]
fn region_objects_keep_rectangles_in_normal_form_and_the_combine_calls_follow_the_pages() {
    // Learn CreateRectRgn: "The region will be exclusive of the bottom and right edges"; GetRgnBox returns NULLREGION 1 / SIMPLEREGION 2 ("a single rectangle") / COMPLEXREGION 3 ("more than a single rectangle"), 0 for an invalid
    // region; CombineRgn RGN_AND 1, RGN_OR 2, RGN_XOR 3, RGN_DIFF 4, RGN_COPY 5; OffsetRgn returns the complexity and ERROR (0) leaves the region as it is; RectInRegion: "any part of the specified rectangle lies within the boundaries
    // of the region" and "The lower and right edges of the rectangle are not included"; EqualRgn: nonzero when the regions are the same (ERROR 0 for an invalid handle). S-region: the empty region's box is (0,0,0,0).
    let l = lines(
        "regions",
        r#"
    uint64_t a = callf(fle_imp_CreateRectRgn, 0, 0, 10, 10), b = callf(fle_imp_CreateRectRgn, 5, 5, 15, 15), d = callf(fle_imp_CreateRectRgn, 0, 0, 0, 0), e = callf(fle_imp_CreateRectRgn, 20, 20, 20, 25);
    int32_t box[4], rc[4] = { 9, 9, 12, 12 };
    #define BOX(h) (memset(box, 0x7F, sizeof box), callf(fle_imp_GetRgnBox, h, A(box), 0, 0))
    printf("a box type %llu: %d %d %d %d\n", U(BOX(a)), box[0], box[1], box[2], box[3]);
    printf("empty (a rectangle with no area) type %llu: %d %d %d %d\n", U(BOX(e)), box[0], box[1], box[2], box[3]);
    printf("PtInRegion a: (0,0) %llu (9,9) %llu (10,10) %llu (-1,0) %llu\n", U(callf(fle_imp_PtInRegion, a, 0, 0, 0)), U(callf(fle_imp_PtInRegion, a, 9, 9, 0)), U(callf(fle_imp_PtInRegion, a, 10, 10, 0)), U(callf(fle_imp_PtInRegion, a, (uint32_t)-1, 0, 0)));
    printf("RectInRegion (9,9)-(12,12) %llu, a rectangle that only touches the edge %llu\n", U(callf(fle_imp_RectInRegion, a, A(rc), 0, 0)), U((rc[0] = 10, rc[1] = 0, rc[2] = 20, rc[3] = 5, callf(fle_imp_RectInRegion, a, A(rc), 0, 0))));
    printf("OR: %llu", U(callf(fle_imp_CombineRgn, d, a, b, 2))); printf(" box type %llu: %d %d %d %d\n", U(BOX(d)), box[0], box[1], box[2], box[3]);
    printf("AND: %llu", U(callf(fle_imp_CombineRgn, d, a, b, 1))); printf(" box type %llu: %d %d %d %d\n", U(BOX(d)), box[0], box[1], box[2], box[3]);
    printf("DIFF: %llu", U(callf(fle_imp_CombineRgn, d, a, b, 4))); printf(" box type %llu: %d %d %d %d, (7,7) %llu (2,2) %llu\n", U(BOX(d)), box[0], box[1], box[2], box[3], U(callf(fle_imp_PtInRegion, d, 7, 7, 0)), U(callf(fle_imp_PtInRegion, d, 2, 2, 0)));
    printf("XOR: %llu", U(callf(fle_imp_CombineRgn, d, a, b, 3))); printf(" box type %llu: %d %d %d %d, (7,7) %llu (12,12) %llu\n", U(BOX(d)), box[0], box[1], box[2], box[3], U(callf(fle_imp_PtInRegion, d, 7, 7, 0)), U(callf(fle_imp_PtInRegion, d, 12, 12, 0)));
    printf("COPY: %llu, equal to a %llu, equal to b %llu\n", U(callf(fle_imp_CombineRgn, d, a, 0, 5)), U(callf(fle_imp_EqualRgn, d, a, 0, 0)), U(callf(fle_imp_EqualRgn, d, b, 0, 0)));
    printf("AND of disjoint regions: %llu\n", U((callf(fle_imp_CombineRgn, d, a, e, 1))));
    printf("OffsetRgn(b, 100, 200): %llu, box ", U(callf(fle_imp_OffsetRgn, b, 100, 200, 0))); BOX(b); printf("%d %d %d %d\n", box[0], box[1], box[2], box[3]);
    printf("SetRectRgn: %llu, box ", U(callf8(fle_imp_SetRectRgn, b, 1, 2, 3, 4, 0, 0, 0))); BOX(b); printf("%d %d %d %d\n", box[0], box[1], box[2], box[3]);
    printf("invalid handle: GetRgnBox %llu OffsetRgn %llu EqualRgn %llu PtInRegion %llu\n", U(callf(fle_imp_GetRgnBox, 0x12345, A(box), 0, 0)), U(callf(fle_imp_OffsetRgn, 0x12345, 1, 1, 0)), U(callf(fle_imp_EqualRgn, a, 0x12345, 0, 0)), U(callf(fle_imp_PtInRegion, 0x12345, 0, 0, 0)));
    printf("out of the 27-bit range: %llu\n", U(callf(fle_imp_CreateRectRgn, 0, 0, 0x08000000, 1)));
"#,
    );
    assert_eq!(l[0], "a box type 2: 0 0 10 10");
    assert_eq!(l[1], "empty (a rectangle with no area) type 1: 0 0 0 0");
    assert_eq!(l[2], "PtInRegion a: (0,0) 1 (9,9) 1 (10,10) 0 (-1,0) 0");
    assert_eq!(l[3], "RectInRegion (9,9)-(12,12) 1, a rectangle that only touches the edge 0");
    assert_eq!(l[4], "OR: 3 box type 3: 0 0 15 15");
    assert_eq!(l[5], "AND: 2 box type 2: 5 5 10 10");
    assert_eq!(l[6], "DIFF: 3 box type 3: 0 0 10 10, (7,7) 0 (2,2) 1");
    assert_eq!(l[7], "XOR: 3 box type 3: 0 0 15 15, (7,7) 0 (12,12) 1");
    assert_eq!(l[8], "COPY: 2, equal to a 1, equal to b 0");
    assert_eq!(l[9], "AND of disjoint regions: 1");
    assert_eq!(l[10], "OffsetRgn(b, 100, 200): 2, box 105 205 115 215");
    assert_eq!(l[11], "SetRectRgn: 1, box 1 2 3 4");
    assert_eq!(l[12], "invalid handle: GetRgnBox 0 OffsetRgn 0 EqualRgn 0 PtInRegion 0");
    assert_eq!(l[13], "out of the 27-bit range: 0");
}

#[test]
fn the_clip_region_calls_the_polygon_region_and_the_region_data_still_stop_by_name() {
    // decision 0123 answers eight region calls and leaves the clip regions of a DC, CreatePolygonRgn, GetRegionData and FrameRgn as named stops (the model has no clip state and no rasterizer)
    for (api, call) in [
        ("SelectClipRgn", "callf(fle_imp_SelectClipRgn, 0, 0, 0, 0);"),
        ("ExtSelectClipRgn", "callf(fle_imp_ExtSelectClipRgn, 0, 0, 5, 0);"),
        ("IntersectClipRect", "callf(fle_imp_IntersectClipRect, 0, 0, 0, 1);"),
        ("ExcludeClipRect", "callf(fle_imp_ExcludeClipRect, 0, 0, 0, 1);"),
        ("GetClipRgn", "callf(fle_imp_GetClipRgn, 0, 0, 0, 0);"),
        ("RectVisible", "callf(fle_imp_RectVisible, 0, 0, 0, 0);"),
        // decision 0126 (wall 94): old: ("GetRegionData", callf(fle_imp_GetRegionData, 0, 0, 0, 0)) stopped; it answers now (0 and ERROR_INVALID_HANDLE for an invalid region; tests/wall_fr11.rs)
        ("FrameRgn", "callf(fle_imp_FrameRgn, 0, 0, 0, 1);"),
    ] {
        let o = run_src(&format!("rgn-stop-{api}"), &top_with("", &[]), &format!("    {call}\n    printf(\"returned\\n\");"));
        assert!(!o.ok && !o.stdout.contains("returned") && o.signal == Some(6), "{api}: {} {}", o.stdout, o.stderr);
        // decision 0126 (old: every message ended with "(decision 0111)"): the clip-region calls and GetRegionData answer now (tests/wall_fr11.rs); called with hdc 0 / hrgn 0 they stop with their own message of decision 0126
        // ("hdc is not a DC of this runtime ... (decision 0126)") or, for GetRegionData, fail with 0 and the last error 6; only RectVisible and FrameRgn keep the old stop and its "(decision 0111)".
        assert!(o.stderr.contains(&format!("fle: {api}: ")) && (o.stderr.contains("(decision 0111)") || o.stderr.contains("(decision 0126)") || o.stderr.contains("not a DC of this runtime")), "{api}: {}", o.stderr);
    }
}

#[test]
fn get_file_attributes_of_the_empty_name_is_invalid_file_attributes_with_error_3_and_free_resource_returns_false() {
    // decision 0123 item 1 (S-22, a choice without a source: neither Learn page says what an empty name gives; the PM remembers 3): 0xFFFFFFFF and ERROR_PATH_NOT_FOUND (3). Learn FreeResource: "For modern versions of Windows
    // this function always returns FALSE" and "If the function succeeds, the return value is zero": the two are read together: 0, nothing is freed, the last error is not touched. fle_res.c needs the module table of the loader (not
    // linked here), so the function body is checked in the source and the generated thunk by the generator test above.
    let l = lines(
        "fileattr",
        r#"
    uint16_t empty[1] = { 0 };
    SETLE(0x5555);
    printf("empty name: %llx le %u\n", U(callf(fle_imp_GetFileAttributesW, A(empty), 0, 0, 0) & 0xFFFFFFFFu), LE());
"#,
    );
    assert_eq!(l[0], "empty name: ffffffff le 3");
    let res = strip_comments(&std::fs::read_to_string(src_dir().join("fle_res.c")).unwrap());
    let a = res.find("void fle_imp_FreeResource(void)").unwrap();
    let body = &res[a..a + res[a..].find("\n}\n").unwrap()];
    assert!(body.contains("R[0] = 0;") && !body.contains("fle_set_last_error") && !body.contains("fle_stop"), "{body}");
}

#[test]
fn get_sys_color_answers_all_31_indices_of_the_decided_table_and_zero_outside() {
    // decision 0123 item 2 (S-23: every value is a choice without a source: Windows 10/11 light-theme defaults from a forum page of registry defaults, not a Microsoft page; COLOR_HIGHLIGHT 0x00D77800 from a Windows 10 snippet;
    // index 25 has no element on the Learn page: 0; index 8 COLOR_WINDOWTEXT stays black, decision 0108); Learn GetSysColor: "If the nIndex parameter is out of range, the return value is zero". Pinned value by value in tests/gdi.rs
    // (get_sys_color_answers_the_table_of_indices_0_to_30_...); here only the properties that the model depends on.
    let l = lines(
        "syscolor",
        r#"
    int i, nonzero = 0, big = 0;
    for (i = 0; i < 31; i++) { uint64_t c = callf(fle_imp_GetSysColor, (uint64_t)i, 0, 0, 0); if (c) nonzero++; if (c > 0xFFFFFF) big++; }
    printf("nonzero entries %d, entries above 0x00FFFFFF %d\n", nonzero, big);
    printf("window %llx text %llx highlight %llx btnface %llx\n", U(callf(fle_imp_GetSysColor, 5, 0, 0, 0)), U(callf(fle_imp_GetSysColor, 8, 0, 0, 0)), U(callf(fle_imp_GetSysColor, 13, 0, 0, 0)), U(callf(fle_imp_GetSysColor, 15, 0, 0, 0)));
    printf("31 %llu -1 %llu 0x80000000 %llu\n", U(callf(fle_imp_GetSysColor, 31, 0, 0, 0)), U(callf(fle_imp_GetSysColor, (uint32_t)-1, 0, 0, 0)), U(callf(fle_imp_GetSysColor, 0x80000000u, 0, 0, 0)));
"#,
    );
    assert_eq!(l[0], "nonzero entries 23, entries above 0x00FFFFFF 0");
    assert_eq!(l[1], "window ffffff text 0 highlight d77800 btnface f0f0f0");
    assert_eq!(l[2], "31 0 -1 0 0x80000000 0");
}

// ---------------------------------------------------------------------------------------------
// 8. The thunk call of fle_dispatch_miss (wall 31) and the source checks
// ---------------------------------------------------------------------------------------------

#[test]
fn the_guests_own_indirect_call_of_a_makeobjectinstance_slot_runs_the_guest_function_with_r11_set_and_any_other_miss_still_stops() {
    // wall 31: the guest's code calls the slot (E8 rel32 -> `pop r11; jmp [rip+0]`) with `call [reg + disp]`. fle_dispatch_miss asks fle_win_try_thunk: a slot of the exact shape in a VirtualAlloc region is entered as the three
    // instructions do (r11 = slot + 5, the stack as at the slot's entry) and the guest function it jumps to is run through fle_dispatch. Any other address (not in a region, a region page that is not executable, bytes that differ)
    // stops as it always did ("indirect call to ... is not a known function entry" or the named reason of the thunk shape check). The region is a real VirtualAlloc of fle_sys.c (PAGE_EXECUTE_READWRITE, as MakeObjectInstance's).
    let top = top_with(
        "static uint64_t seen_r11, seen_a, seen_b, seen_rsp_word;\n\
         #define VA_TARGET 0x52a570\n\
         static void target(void) { seen_r11 = R[11]; seen_a = R[1]; seen_b = R[2]; seen_rsp_word = fle_ld(R[4], 8); R[0] = 0x4242; }\n\
         static uint64_t blk;\n\
         static uint64_t slot0(void) { return blk + 0x18; }\n\
         static void make_block(uint32_t prot) { static const uint8_t tpl[8] = { 0x41, 0x5b, 0xff, 0x25, 0, 0, 0, 0 }; uint64_t t = VA_TARGET; int32_t d; uint8_t *p;\n\
           R[4] -= 8; fle_st(R[4], 8, 0xAAAA0000); R[1] = 0; R[2] = 0x1000; R[8] = 0x3000; R[9] = 0x40; fle_imp_VirtualAlloc(); R[4] += 8; blk = R[0]; p = (uint8_t *)(uintptr_t)blk;\n\
           memset(p, 0, 0x1000); memcpy(p + 8, tpl, 8); memcpy(p + 0x10, &t, 8); p[0x18] = 0xe8; d = (int32_t)((int64_t)(blk + 8) - (int64_t)(slot0() + 5)); memcpy(p + 0x19, &d, 4);\n\
           if (prot != 0x40) { R[4] -= 8; fle_st(R[4], 8, 0xAAAA0000); R[1] = blk; R[2] = 0x1000; R[8] = prot; R[9] = A(&seen_a); fle_imp_VirtualProtect(); R[4] += 8; } }",
        &[("VA_TARGET", "target")],
    );
    let l = lines_top(
        "thunk-miss",
        &top,
        r#"
    make_block(0x40);
    /* the guest's `call [rax + 0x10]` with the slot address: the return address is on the stack, the arguments in rcx and rdx */
    R[1] = 0x1111; R[2] = 0x2222; R[4] -= 8; fle_st(R[4], 8, 0xBEEF0001);
    fle_dispatch(slot0());
    printf("returned %llx, r11 = slot + 5 %d, rcx %llx rdx %llx, [rsp] still the return address %d\n", U(R[0]), seen_r11 == slot0() + 5, U(seen_a), U(seen_b), seen_rsp_word == 0xBEEF0001);
"#,
    );
    assert_eq!(l[0], "returned 4242, r11 = slot + 5 1, rcx 1111 rdx 2222, [rsp] still the return address 1");
    for (label, body, word) in [
        ("not-in-a-region", "fle_dispatch(0x7FF012340000ull);", "is not a known function entry"),
        ("no-e8", "make_block(0x40); *(uint8_t *)(uintptr_t)(slot0()) = 0xe9; fle_dispatch(slot0());", "does not start with E8"),
        ("template-differs", "make_block(0x40); *(uint8_t *)(uintptr_t)(blk + 9) = 0x5a; fle_dispatch(slot0());", "pop r11; jmp [rip+0]"),
        ("page-not-executable", "make_block(0x04); fle_dispatch(slot0());", "not marked for running code"),
    ] {
        let o = run_src(&format!("thunk-miss-{label}"), &top, &format!("    R[4] -= 8; fle_st(R[4], 8, 0xBEEF0001);\n    {body}\n    printf(\"returned\\n\");"));
        assert!(!o.ok && !o.stdout.contains("returned") && o.signal == Some(6), "{label}: {} {}", o.stdout, o.stderr);
        assert!(o.stderr.contains(word), "{label}: {}", o.stderr);
    }
}

#[test]
fn source_of_the_window_model_reads_no_host_state_draws_no_pixel_and_names_each_choice_without_a_source() {
    let win = strip_comments(&std::fs::read_to_string(src_dir().join("fle_win.c")).unwrap());
    for bad in [
        "getpid", "getenv", "environ", "fopen", "open(", "/proc", "/etc", "/home", "/dev", "/tmp", "clock_gettime", "gettimeofday", "time(", "nanosleep", "usleep", "sleep(", "select(", "<time.h>", "<sys/time.h>", "pthread",
        "<sys/", "<unistd.h>", "<signal.h>", "fork(", "syscall", "futex", "__asm__", "abort(", "X11", "DISPLAY", "wayland", "ioctl", "dlopen", "LoadLibrary", "setjmp", "longjmp", "(void (*)", "fle_dispatch_table",
    ] {
        assert!(!win.contains(bad), "fle_win.c must not use {bad}");
    }
    // nothing is drawn: the window model writes no pixel and calls no drawing API of fle_gdi.c
    for pix in ["BitBlt", "StretchDIBits", "SetDIBitsToDevice", "CreateDIBSection", "PatBlt", "TextOut", "ExtTextOut", "SetPixel", "LineTo", "Rectangle("] {
        assert!(!win.contains(pix), "fle_win.c must not draw: {pix}");
    }
    // decision 0323 (1): the one drawing call of the window model is DefWindowProc(WM_ERASEBKGND) filling with the class background brush (Learn WM_ERASEBKGND / WNDCLASS.hbrBackground): the weak fle_imp_FillRect, declared once and
    // called once, and no other form of the word
    assert!(win.contains("void fle_imp_FillRect(void) __attribute__((weak));"));
    assert_eq!(win.matches("FillRect").count(), win.matches("fle_imp_FillRect").count(), "FillRect only as fle_imp_FillRect");
    assert_eq!(win.matches("fle_imp_FillRect()").count(), 1, "one call of FillRect");
    // the only GDI entries that the window model calls are GetDC and ReleaseDC, both through weak references, in two pairs: BeginPaint / EndPaint, and the DC that RedrawWindow (RDW_ERASENOW / RDW_UPDATENOW) gives to
    // WM_ERASEBKGND (paint_now_tree, decision 0253: Learn WM_ERASEBKGND "wParam: A handle to the device context")
    assert!(win.contains("void fle_imp_GetDC(void) __attribute__((weak));") && win.contains("void fle_imp_ReleaseDC(void) __attribute__((weak));"));
    assert_eq!(win.matches("fle_imp_GetDC()").count(), 2);
    assert_eq!(win.matches("fle_imp_ReleaseDC()").count(), 2);
    for (head, gets, releases) in [
        ("void fle_imp_BeginPaint(void)", 1, 0),
        ("void fle_imp_EndPaint(void)", 0, 1),
        // decision 0323: the DC of the WM_ERASEBKGND that RedrawWindow gives (GetDC ... ReleaseDC) moved from paint_now_tree into erase_now_one (which clips it to the update region)
        ("static void erase_now_one(const char *api, int64_t i)\n{", 1, 1),
        ("static void paint_now_tree(const char *api, int64_t i, uint32_t fl, int children)", 0, 0),
    ] {
        let at = win.find(head).unwrap_or_else(|| panic!("fle_win.c has no {head}"));
        let rest = &win[at..];
        let body = &rest[..rest.find("\n}\n").unwrap_or(rest.len())];
        assert_eq!(body.matches("fle_imp_GetDC()").count(), gets, "GetDC calls in {head}");
        assert_eq!(body.matches("fle_imp_ReleaseDC()").count(), releases, "ReleaseDC calls in {head}");
    }
    // the clock is the virtual one (fle_sys_clock_ticks, weak); the thread and the wake-up come from fle_thread.h
    assert!(win.contains("uint64_t fle_sys_clock_ticks(void) __attribute__((weak));"));
    // no message is injected except WM_TIMER and WM_PAINT (decision 0123, item 3): the synthesised messages are made in one function
    // decision 0126 (old: no `uint32_t qs` argument): the QS_* types that may be made (PM_QS_PAINT / PM_QS_POSTMESSAGE)
    let at = win.find("int fle_win_synth_message(unsigned t, uint64_t fh, uint32_t fmin, uint32_t fmax, int remove, uint32_t qs, fle_tmsg *out)\n{").unwrap();
    let body = &win[at..at + win[at..].find("\n}\n").unwrap()];
    for (m, n) in [("out->msg = FLE_WM_PAINT;", 1), ("out->msg = FLE_WM_TIMER;", 1)] {
        assert_eq!(body.matches(m).count(), n, "{m}");
    }
    assert_eq!(body.matches("out->msg =").count(), 2, "only WM_PAINT and WM_TIMER are made");
    // the one place that makes a message out of nothing else is WM_QUIT of PostQuitMessage / GetMessageW (decision 0122)
    assert_eq!(win.matches("m.msg = FLE_WM_QUIT_ID;").count(), 2);
    // the G2 detector reports once and the table report is written
    assert!(win.contains("static int g2_seen;") && win.contains("void fle_win_g2_report(FILE *f)"));
    // the scheduler hooks are the only additions of fle_thread.c that the window model needs: two weak calls
    let th = strip_comments(&std::fs::read_to_string(src_dir().join("fle_thread.c")).unwrap());
    assert!(th.contains("fle_win_next_timer_tick") && th.contains("fle_win_has_paint"));
    // the header declares every row of decision 0123 that lies in user32 / comctl32 as a function of fle_win.h
    let h = std::fs::read_to_string(src_dir().join("fle_win.h")).unwrap();
    for (dll, n) in NEW_ROWS {
        if dll == "user32.dll" || dll == "comctl32.dll" {
            assert!(h.contains(&format!("void fle_imp_{n}(void);")), "{n} is declared in fle_win.h");
        }
    }
    // the VirtualQuery model: images from the section table, ranges from the harness; no host call
    let vm = strip_comments(&std::fs::read_to_string(src_dir().join("fle_vmap.c")).unwrap());
    for bad in ["/proc", "mincore", "msync", "getpid", "fopen"] {
        assert!(!vm.contains(bad), "fle_vmap.c must not use {bad}");
    }
}

#[test]
fn a_window_call_that_the_model_does_not_cover_stops_by_name_with_the_reason() {
    // the stops that remain in the window model (decision 0123, "still stops"): each is "fle: <api>: <reason>" and SIGABRT, so no unknown form is answered by a made-up value
    aborts("stop-showwindow-default", "    uint64_t h = cw(0x80, \"w\", POPUP, 0, 0, 5, 5, 0, 0, 0); callf(fle_imp_ShowWindow, h, 10, 0, 0);", "ShowWindow", "SW_SHOWDEFAULT (10)");
    aborts("stop-setparent-message", "    uint64_t h = cw(0x80, \"w\", POPUP, 0, 0, 5, 5, 0, 0, 0); callf(fle_imp_SetParent, h, 0xFFFFFFFFFFFFFFFDull, 0, 0);", "SetParent", "HWND_MESSAGE");
    aborts("stop-gwl-extra", "    uint64_t h = cw(0x80, \"w\", POPUP, 0, 0, 5, 5, 0, 0, 0); callf(fle_imp_GetWindowLongPtrW, h, 0, 0, 0);", "GetWindowLongPtrW", "extra memory and DWLP_* are not modelled");
    aborts("stop-menu", "    uint64_t h = cw(0x80, \"w\", POPUP, 0, 0, 5, 5, 0, 0, 0); callf(fle_imp_SetMenu, h, 0x1234, 0, 0);", "SetMenu", "menus");
    aborts("stop-sb-ctl", "    uint64_t h = cw(0x80, \"w\", POPUP, 0, 0, 5, 5, 0, 0, 0); uint32_t si[7] = { 28, 1, 0, 0, 0, 0, 0 }; callf(fle_imp_SetScrollInfo, h, 2, A(si), 1);", "SetScrollInfo", "SB_CTL");
    // decision 0258 (alpha7): WM_GETMINMAXINFO with a valid MINMAXINFO is answered (0); the NULL lParam of this call is what still stops (old reason: "the MINMAXINFO defaults depend on the frame and the screen: not modelled")
    aborts("stop-defproc-minmax-null-lparam", "    uint64_t h = cw(0x80, \"w\", POPUP, 0, 0, 5, 5, 0, 0, 0); R[1] = h; R[2] = 0x24; R[8] = 0; R[9] = 0; R[4] -= 8; fle_st(R[4], 8, 1); fle_imp_DefWindowProcW();", "DefWindowProcW", "WM_GETMINMAXINFO with a NULL lParam");
}
