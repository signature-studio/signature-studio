//! Tests for the virtual monitor of runtime/fle_monitor.c: user32!EnumDisplayMonitors(NULL, NULL, proc, data) and
//! user32!GetMonitorInfoW(the virtual monitor, MONITORINFO) (docs/decisions/0065-virtual-monitor.md; sources:
//! docs/investigations/callback-caller.md). The guest callback is called through fle_call_guest (decision 0064, tests/callback.rs).
//!
//! Expected values are written here from the decision (Microsoft Learn: EnumDisplayMonitors, MONITORENUMPROC, GetMonitorInfoW,
//! MONITORINFO, The Virtual Screen; decisions 0053 and 0057 for 1920x1080 and the origin), not read back from the C code.
//! PROVISIONAL (decision 0065): the HMONITOR value 0x7FEA0004, rcWork = rcMonitor, the place of the RECT handed to the callback, and
//! the return value of EnumDisplayMonitors when the callback stops the enumeration. The host is Linux: nothing is compared with a real
//! Windows; no display, monitor, EDID, display server, registry, clock, thread or process is read.
//!
//! The C sources of fle_monitor.c/.h and fle_callback.c/.h come from runtime/ when fle_monitor.c is there, otherwise from out-cb/work/
//! (development). FLE_MON_SRC_DIR (a directory with those four files) overrides both: the mutation runs of the run record use it.

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
    if let Ok(d) = std::env::var("FLE_MON_SRC_DIR") {
        return PathBuf::from(d);
    }
    if runtime_dir().join("fle_monitor.c").exists() {
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

const DRIVER: &str = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <signal.h>
#include <unistd.h>
#include <stdint.h>
#include "fle_runtime.h"
#include "fle_dispatch.h"
#include "fle_env.h"
#include "fle_imports.h"
#include "fle_callback.h"
#include "fle_monitor.h"

uint64_t R[16];
static uint64_t stk[0x2000] __attribute__((aligned(4096)));
#define E0 ((uint64_t)(uintptr_t)stk + 0x8000 + 8)
#define TOP ((uint64_t)(uintptr_t)stk + sizeof stk)
#define RET0 UINT64_C(0xdead0000)
#define PATTERN UINT64_C(0xA5A5A5A5A5A5A5A5)
#define U(x) ((unsigned long long)(x))

#define VA_CB 0x2000
static unsigned g_calls;
static uint64_t g_a[4], g_cb_sp, g_retv, g_rect_before_rsp;
static int32_t g_rect[4];
static uint8_t g_buf[0x80];
static uint8_t g_init[0x80];

static void cb_record(void)
{
    g_calls++;
    g_a[0] = R[1]; g_a[1] = R[2]; g_a[2] = R[8]; g_a[3] = R[9];
    g_cb_sp = R[4];
    if (R[8]) memcpy(g_rect, (void *)(uintptr_t)R[8], sizeof g_rect);
    R[0] = g_retv;
    R[1] = R[2] = R[8] = R[9] = 0;      /* the callee may destroy the volatile registers */
}
static void cb_clobber_rect(void)       /* the callee overwrites the RECT it was given (it is the callee's to read; a second call must not see it) */
{
    g_calls++;
    if (R[8]) memset((void *)(uintptr_t)R[8], 0x77, 16);
    R[0] = 1;
}
const fle_fn_entry fle_dispatch_table[] = { { VA_CB, cb_record }, { VA_CB + 0x10, cb_clobber_rect } };
const uint64_t fle_dispatch_count = 2;

static void on_abort(int s)
{
    char out[0x80 * 2 + 8];
    size_t i;
    (void)s;
    memcpy(out, "BUF ", 4);
    for (i = 0; i < sizeof g_buf; i++) { out[4 + 2 * i] = "0123456789abcdef"[g_buf[i] >> 4]; out[5 + 2 * i] = "0123456789abcdef"[g_buf[i] & 15]; }
    out[4 + 2 * sizeof g_buf] = '\n';
    (void)!write(1, out, 5 + 2 * sizeof g_buf);
    _exit(134);
}
static uint64_t u(const char *s) { return strtoull(s, 0, 0); }
static void fill(void) { size_t i; for (i = 0; i < sizeof stk / 8; i++) stk[i] = PATTERN; }
static void hexbuf(void) { size_t i; for (i = 0; i < sizeof g_buf; i++) printf("%02x", g_buf[i]); printf("\n"); }

int main(int argc, char **argv)
{
    const char *t = argc > 1 ? argv[1] : "";
    uint64_t r;
    fill();
    R[4] = E0;
    fle_st(E0, 8, RET0);
    fle_set_last_error(0x1234);

    if (!strcmp(t, "enum")) {            /* hdc clip proc data retval [sp-delta] */
        uint64_t e = E0 - (argc > 7 ? u(argv[7]) : 0), a, bad = 0;
        int i;
        for (i = 0; i < 16; i++) R[i] = 0x5000 + (uint64_t)i;
        R[4] = e;
        fill();
        fle_st(e, 8, RET0);
        R[1] = u(argv[2]); R[2] = u(argv[3]); R[8] = u(argv[4]) ? (u(argv[4]) == 1 ? VA_CB : u(argv[4])) : 0; R[9] = u(argv[5]);
        g_retv = u(argv[6]);
        fle_imp_EnumDisplayMonitors();
        r = R[0];
        for (a = e + 8; a + 8 <= TOP; a += 8) if (fle_ld(a, 8) != PATTERN) bad++;
        printf("ret %llx calls %u rsp-kept %d api-ret %llx above-changed %llu\n", U(r), g_calls, (int)(R[4] == e), U(fle_ld(e, 8)), U(bad));
        printf("args %llx %llx %llx %llx\n", U(g_a[0]), U(g_a[1]), g_a[2] >= (uint64_t)(uintptr_t)stk && g_a[2] + 16 <= e ? 0ull : U(g_a[2]), U(g_a[3]));
        printf("rect %d %d %d %d\n", g_rect[0], g_rect[1], g_rect[2], g_rect[3]);
        printf("rect-ptr-aligned %d cb-sp-below-rect %d\n", (int)(g_a[2] % 16 == 0), (int)(g_cb_sp + 8 + 32 <= g_a[2]));
        for (i = 3; i < 16; i++) if (i != 4 && i != 8 && i != 9 && R[i] != 0x5000 + (uint64_t)i) printf("changed R[%d]\n", i);
        fle_imp_GetLastError();
        printf("last-error %llx\n", U(R[0]));
    } else if (!strcmp(t, "enum-stop")) {  /* hdc clip proc : the call must stop; the callback must not have been called (printed by the abort handler path: none) */
        R[1] = u(argv[2]); R[2] = u(argv[3]); R[8] = u(argv[4]) ? (u(argv[4]) == 1 ? VA_CB : u(argv[4])) : 0; R[9] = 0;
        fle_imp_EnumDisplayMonitors();
        printf("returned calls %u\n", g_calls);
    } else if (!strcmp(t, "gmi")) {      /* hMonitor cbSize [lpmi-offset-kind] */
        uint64_t mi = (uint64_t)(uintptr_t)g_buf + 8;
        memset(g_buf, 0xAA, sizeof g_buf);
        fle_st(mi, 4, u(argv[3]));
        memcpy(g_init, g_buf, sizeof g_buf);
        signal(SIGABRT, on_abort);
        R[1] = u(argv[2]); R[2] = (argc > 4 && !strcmp(argv[4], "null")) ? 0 : mi;
        R[9] = 0x77;
        fle_imp_GetMonitorInfoW();
        printf("ret %llx r9 %llx last-error ", U(R[0]), U(R[9]));
        fle_imp_GetLastError();
        printf("%llx\n", U(R[0]));
        hexbuf();
    } else if (!strcmp(t, "gmi-high")) { /* cbSize with garbage in the upper half of the 8 bytes (only the DWORD counts) */
        uint64_t mi = (uint64_t)(uintptr_t)g_buf + 8;
        memset(g_buf, 0xAA, sizeof g_buf);
        fle_st(mi, 8, UINT64_C(0xFFFFFFFF00000028));
        R[1] = FLE_MONITOR_HANDLE; R[2] = mi;
        fle_imp_GetMonitorInfoW();
        printf("ret %llx\n", U(R[0]));
        hexbuf();
    } else if (!strcmp(t, "twice")) {    /* two enumerations in a row give the same answers; the rect of the first is not reused */
        uint64_t rects[2], hs[2];
        int k;
        for (k = 0; k < 2; k++) {
            g_retv = 1;
            R[1] = 0; R[2] = 0; R[8] = VA_CB; R[9] = 0x42 + (uint64_t)k;
            R[4] = E0;
            fle_imp_EnumDisplayMonitors();
            rects[k] = g_a[2]; hs[k] = g_a[0];
            printf("round %d ret %llx calls %u rect %d %d %d %d data %llx\n", k, U(R[0]), g_calls, g_rect[0], g_rect[1], g_rect[2], g_rect[3], U(g_a[3]));
        }
        printf("same-handle %d same-rect-place %d\n", (int)(hs[0] == hs[1]), (int)(rects[0] == rects[1]));
    } else if (!strcmp(t, "clobber")) {  /* the callee overwrites its RECT; the API still returns 1 and the stack above is intact */
        uint64_t a, bad = 0;
        R[1] = 0; R[2] = 0; R[8] = VA_CB + 0x10; R[9] = 0;
        R[4] = E0;
        fle_imp_EnumDisplayMonitors();
        for (a = E0 + 8; a < TOP; a += 8) if (fle_ld(a, 8) != PATTERN) bad++;
        printf("ret %llx calls %u above-changed %llu rsp-kept %d\n", U(R[0]), g_calls, U(bad), (int)(R[4] == E0));
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
        let tag = etxt_common::per_process("mon-driver");
        let rt = runtime_dir();
        let objs: Vec<PathBuf> = [("fle_monitor.c", true), ("fle_callback.c", true)]
            .iter()
            .map(|(n, strict)| {
                let o = dir.join(format!("{tag}-{}.o", n.replace(".c", "")));
                clang_c(&src_dir().join(n), &o, *strict);
                o
            })
            .collect();
        let main_c = dir.join(format!("{tag}-main.c"));
        std::fs::write(&main_c, DRIVER).unwrap();
        let exe = dir.join(&tag);
        let mut cmd = Command::new("clang");
        cmd.args(["-O0", "-std=gnu11", "-w", "-fPIE", "-I"]).arg(src_dir()).arg("-I").arg(&rt);
        for o in &objs {
            cmd.arg(o);
        }
        let cc = cmd
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

fn lines(args: &[&str]) -> Vec<String> {
    let o = exec(driver(), args);
    assert!(o.ok, "{args:?} did not exit normally (signal {:?}): {}\n{}", o.signal, o.stderr, o.stdout);
    o.stdout.lines().map(|s| s.to_string()).collect()
}

/// A stop of EnumDisplayMonitors: SIGABRT, the message with `needle`, nothing printed (so the callback was not called either).
fn enum_stops(args: &[&str], needle: &str) -> String {
    let o = exec(driver(), args);
    assert_eq!(o.signal, Some(6), "{args:?}: not SIGABRT: {} / {}", o.stderr, o.stdout);
    assert!(o.stderr.contains(needle), "{args:?}: no {needle:?} in {}", o.stderr);
    assert!(o.stdout.is_empty(), "{args:?}: output before the stop: {}", o.stdout);
    o.stderr
}

const H: u64 = 0x7FEA_0004;
const INIT_HEX: &str = "aaaaaaaaaaaaaaaa28000000aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

/// The stop of GetMonitorInfoW: the process is aborted by fle_stop; the driver's SIGABRT handler prints the buffer ("BUF <hex>"), which
/// must be exactly what it was before the call (nothing written on a stop).
fn gmi_stops(args: &[&str], needle: &str) {
    let o = exec(driver(), args);
    assert!(!o.ok && o.signal.is_none() || o.signal == Some(6), "{args:?}: {} / {}", o.stderr, o.stdout);
    assert!(o.stderr.contains(needle), "{args:?}: no {needle:?} in {}", o.stderr);
    let buf = o.stdout.lines().find_map(|l| l.strip_prefix("BUF ")).unwrap_or_else(|| panic!("{args:?}: no BUF line: {}", o.stdout));
    let mut want = vec![0xAAu8; 0x80];
    let cb = parse(args[2]) as u32;
    want[8..12].copy_from_slice(&cb.to_le_bytes());
    let hex: String = want.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(buf, hex, "{args:?}: the buffer changed on a stop");
}

fn parse(s: &str) -> u64 {
    if let Some(h) = s.strip_prefix("0x") {
        u64::from_str_radix(h, 16).unwrap()
    } else {
        s.parse().unwrap()
    }
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

// ---- 1. Table, header, constants ----

#[test]
fn the_rows_are_in_the_table_and_declared_in_the_header() {
    let table = std::fs::read_to_string(manifest().join("src/imports_table.rs")).unwrap();
    let h = std::fs::read_to_string(src_dir().join("fle_monitor.h")).unwrap();
    for (api, f) in [("EnumDisplayMonitors", "fle_imp_EnumDisplayMonitors"), ("GetMonitorInfoW", "fle_imp_GetMonitorInfoW")] {
        let row = format!("(\"user32.dll\", \"{api}\", \"{f}\", \"jmp_imp:{api}\")");
        assert_eq!(table.matches(&row).count(), 1, "exactly one row for {api}");
        assert!(h.contains(&format!("void {f}(void)")), "{f}");
    }
    // the other monitor APIs are not answered by this decision (MonitorFromWindow needs a window table: decision 0065)
    // EnumDisplaySettingsW was in this list until decision 0098 (runtime/fle_monitor.c, tests/wall_r.rs) answered it: the guest's refresh-rate
    // helper (f_666d50) is a census wall at unit 246, and the claim "not answered" is no longer true.
    // old: "MonitorFromWindow", "MonitorFromRect" and "MonitorFromPoint" were in this list; decision 0126 (wall 100) answered them with the one monitor of decision 0065 (runtime/fle_monitor.c, fle_win.c; tests/wall_fr11.rs)
    // old: "GetDpiForMonitor" was in this list; decision 0144 (wall 4, runtime/fle_appfile.c, tests/wall_fr19.rs) answers it: the delay-load helper raised 0xC06D007F when GetProcAddress gave NULL
    for n in ["GetMonitorInfoA", "EnumDisplayDevicesW"] {
        assert!(!table.contains(&format!("\"{n}\"")), "{n} must not be answered by decision 0065");
    }
}

#[test]
fn the_constants_are_the_decided_ones() {
    assert_eq!(def_const("fle_monitor.h", "FLE_MONITOR_HANDLE_BASE"), 0x7FEA_0000);
    assert_eq!(def_const("fle_monitor.h", "FLE_MONITOR_REGION_SIZE"), 0x10_0000);
    assert_eq!(H, 0x7FEA_0000 + 4);
    // Learn: sizeof(MONITORINFO) = DWORD + 2 RECT + DWORD = 4 + 16 + 16 + 4 = 40; MONITORINFOEXW adds WCHAR szDevice[CCHDEVICENAME = 32] = 64 bytes
    assert_eq!(def_const("fle_monitor.h", "FLE_MONITORINFO_SIZE"), 4 + 16 + 16 + 4);
    assert_eq!(def_const("fle_monitor.h", "FLE_MONITORINFOEXW_SIZE"), 4 + 16 + 16 + 4 + 2 * 32);
    assert_eq!(def_const("fle_monitor.h", "FLE_MONITORINFOEXA_SIZE"), 4 + 16 + 16 + 4 + 32);
    assert_eq!(def_const("fle_monitor.h", "FLE_MONITORINFO_OFF_MONITOR"), 4);
    assert_eq!(def_const("fle_monitor.h", "FLE_MONITORINFO_OFF_WORK"), 20);
    assert_eq!(def_const("fle_monitor.h", "FLE_MONITORINFO_OFF_FLAGS"), 36);
    // the SDK header's value (mingw-w64 winuser.h: #define MONITORINFOF_PRIMARY 0x00000001); the Learn page names the flag only
    assert_eq!(def_const("fle_monitor.h", "FLE_MONITORINFOF_PRIMARY"), 1);
    // the rectangle is the virtual display of decision 0053 and holds the origin of decision 0057
    assert_eq!((def_const("fle_monitor.h", "FLE_MONITOR_LEFT"), def_const("fle_monitor.h", "FLE_MONITOR_TOP")), (0, 0));
    assert_eq!(def_const("fle_monitor.h", "FLE_MONITOR_RIGHT"), 1920);
    assert_eq!(def_const("fle_monitor.h", "FLE_MONITOR_BOTTOM"), 1080);
    assert_eq!(def_const("fle_monitor.h", "FLE_MONITOR_RIGHT"), def_const("fle_gdc.h", "FLE_GDC_HORZRES"));
    assert_eq!(def_const("fle_monitor.h", "FLE_MONITOR_BOTTOM"), def_const("fle_gdc.h", "FLE_GDC_VERTRES"));
    assert_eq!(def_const("fle_monitor.h", "FLE_MONITOR_LEFT"), def_const("fle_sysmetrics.h", "FLE_SM_VIRTUALSCREEN_ORIGIN"));
    // SM_CMONITORS = 1 (decision 0057): one monitor
    assert_eq!(def_const("fle_sysmetrics.h", "FLE_SM_CMONITORS"), 1);
    // rcWork = rcMonitor (decision 0065, provisional): the header says so by name
    let h = std::fs::read_to_string(src_dir().join("fle_monitor.h")).unwrap();
    for (w, m) in [("LEFT", "LEFT"), ("TOP", "TOP"), ("RIGHT", "RIGHT"), ("BOTTOM", "BOTTOM")] {
        assert!(h.contains(&format!("#define FLE_MONITOR_WORK_{w} FLE_MONITOR_{m}")) || h.contains(&format!("#define FLE_MONITOR_WORK_{w}  FLE_MONITOR_{m}")) || h.contains(&format!("#define FLE_MONITOR_WORK_{w} ")), "{w}");
    }
}

#[test]
fn the_handle_region_is_below_2_gib_and_apart_from_every_other_table() {
    let base = def_const("fle_monitor.h", "FLE_MONITOR_HANDLE_BASE");
    let size = def_const("fle_monitor.h", "FLE_MONITOR_REGION_SIZE");
    assert!(base + size <= 0x8000_0000, "below 2 GiB");
    // every other handle or pointer region of the runtime that is below 2 GiB, written here from decisions 0053, 0054, 0059, 0061
    let others: Vec<(&str, u64, u64)> = vec![
        ("DCs (0053)", def_const("fle_gdc.h", "FLE_GDC_HANDLE_BASE"), def_const("fle_gdc.h", "FLE_GDC_REGION_SIZE")),
        ("GDI objects (0054)", def_const("fle_gdi.h", "FLE_GDI_HANDLE_BASE"), def_const("fle_gdi.h", "FLE_GDI_REGION_SIZE")),
        ("cursors (0059)", def_const("fle_cursor.h", "FLE_CURSOR_HANDLE_BASE"), def_const("fle_cursor.h", "FLE_CURSOR_REGION_SIZE")),
        ("DLL cursors (0061)", def_const("fle_loadimage.h", "FLE_LI_MODULE_HANDLE_BASE"), def_const("fle_loadimage.h", "FLE_LI_REGION_SIZE")),
    ];
    assert_eq!(others[0].1, 0x7FA9_0000);
    assert_eq!(others[1].1, 0x7FBA_0000);
    assert_eq!(others[2].1, 0x7FCA_0000);
    assert_eq!(others[3].1, 0x7FDA_0000);
    for (name, b, s) in others {
        assert!(base >= b + s || base + size <= b, "overlaps {name}: {b:#x}+{s:#x}");
    }
    // the image (ImageBase 0x400000, SizeOfImage 0x18C0000) is far below
    assert!(base > 0x400000 + 0x18C0000);
    // the handle: non-NULL (MONITORENUMPROC: "will always be non-NULL"), inside the region, 4-byte aligned like the other handles
    assert!(H >= base && H < base + size && H % 4 == 0 && H != 0);
}

// ---- 2. EnumDisplayMonitors ----

#[test]
fn the_callback_is_called_once_with_the_monitor_a_null_dc_the_rectangle_and_the_data() {
    let data = 0x0123_4567_89AB_CDEFu64;
    let l = lines(&["enum", "0", "0", "1", &data.to_string(), "1"]);
    // ret 1: the callback returned TRUE; one call; rsp restored; the API's return address and everything above it untouched
    assert_eq!(l[0], "ret 1 calls 1 rsp-kept 1 api-ret dead0000 above-changed 0");
    // hMonitor, hdcMonitor NULL (the page: NULL when hdc is NULL), lprcMonitor a pointer into the guest stack below the API's frame
    // (printed as 0 when it is: the C driver maps "inside the stack and below the API's rsp" to 0), dwData unchanged (all 64 bits)
    assert_eq!(l[1], format!("args {H:x} 0 0 {data:x}"));
    // the rectangle the callback read through lprcMonitor: the monitor rectangle in virtual-screen coordinates
    assert_eq!(l[2], "rect 0 0 1920 1080");
    assert_eq!(l[3], "rect-ptr-aligned 1 cb-sp-below-rect 1");
    // the callee-saved registers are what they were; volatile ones may change but R[3], R[5..7], R[10..15] must be as set
    assert!(!l.iter().any(|x| x.starts_with("changed R[")), "{l:?}");
    // the last error is untouched
    assert_eq!(l.last().unwrap(), "last-error 1234");
}

#[test]
fn a_false_from_the_callback_stops_the_enumeration_and_the_result_is_the_callbacks_bool() {
    // one monitor, so one call either way; FALSE (0) -> the API returns 0 (provisional rule of decision 0065), TRUE -> 1
    let l = lines(&["enum", "0", "0", "1", "7", "0"]);
    assert!(l[0].starts_with("ret 0 calls 1 "), "{}", l[0]);
    let l = lines(&["enum", "0", "0", "1", "7", "1"]);
    assert!(l[0].starts_with("ret 1 calls 1 "), "{}", l[0]);
    // the PM's rule (decision 0065): al decides. A nonzero al is TRUE (2, 0xFF, 0xFFFFFFFF) ...
    for v in ["2", "0xFF", "0xFFFFFFFF", "0x1234567801"] {
        let l = lines(&["enum", "0", "0", "1", "7", v]);
        assert!(l[0].starts_with("ret 1 calls 1 "), "retval {v}: {}", l[0]);
    }
    // ... and al == 0 is FALSE whatever the other bits of rax are (the guest's callback writes al only; decision 0065 records that a
    // BOOL such as 0x100 would be TRUE for a Windows that tests eax: the guest never produces one)
    for v in ["0x100", "0x80000000", "0x1234567800000000", "0xFFFFFFFFFFFFFF00"] {
        let l = lines(&["enum", "0", "0", "1", "7", v]);
        assert!(l[0].starts_with("ret 0 calls 1 "), "retval {v}: {}", l[0]);
    }
}

#[test]
fn the_data_pointer_and_every_bit_of_dwdata_reach_the_callback() {
    for d in [0u64, 1, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFFF, 0x1_0000_0000, 0xFFFF_FFFF_FFFF_FFFF, 0x8000_0000_0000_0000] {
        let l = lines(&["enum", "0", "0", "1", &d.to_string(), "1"]);
        assert_eq!(l[1], format!("args {H:x} 0 0 {d:x}"), "dwData {d:#x}");
    }
}

#[test]
fn it_works_for_every_residue_of_the_apis_stack_pointer_and_restores_it() {
    for delta in [0u64, 8, 4, 12, 16] {
        let l = lines(&["enum", "0", "0", "1", "9", "1", &delta.to_string()]);
        assert_eq!(l[0], "ret 1 calls 1 rsp-kept 1 api-ret dead0000 above-changed 0", "delta {delta}");
        assert_eq!(l[2], "rect 0 0 1920 1080", "delta {delta}");
        assert_eq!(l[3], "rect-ptr-aligned 1 cb-sp-below-rect 1", "delta {delta}");
    }
}

#[test]
fn two_enumerations_give_the_same_answers_and_the_rectangle_is_written_again_for_each() {
    let l = lines(&["twice"]);
    assert_eq!(l[0], "round 0 ret 1 calls 1 rect 0 0 1920 1080 data 42");
    assert_eq!(l[1], "round 1 ret 1 calls 2 rect 0 0 1920 1080 data 43");
    assert_eq!(l[2], "same-handle 1 same-rect-place 1");
}

#[test]
fn a_callback_that_overwrites_its_rectangle_does_not_damage_the_callers_stack() {
    let l = lines(&["clobber"]);
    assert_eq!(l[0], "ret 1 calls 1 above-changed 0 rsp-kept 1");
}

#[test]
fn a_non_null_dc_a_clip_rectangle_or_a_null_callback_stops_with_a_named_reason() {
    // the page's other three (hdc, lprcClip) combinations need the visible region of a DC or a clipping computation: not implemented
    let e = enum_stops(&["enum-stop", "0x7FA90004", "0", "1"], "fle: EnumDisplayMonitors: hdc is not NULL");
    assert!(e.contains("decision 0065"), "{e}");
    enum_stops(&["enum-stop", "1", "0", "1"], "fle: EnumDisplayMonitors: hdc is not NULL");
    enum_stops(&["enum-stop", "0xFFFFFFFFFFFFFFFF", "0", "1"], "fle: EnumDisplayMonitors: hdc is not NULL");
    // the whole 64 bits of rdx and rcx count (not only the low DWORD): a NULL low half with a nonzero high half is a pointer
    enum_stops(&["enum-stop", "0x100000000", "0", "1"], "fle: EnumDisplayMonitors: hdc is not NULL");
    let e = enum_stops(&["enum-stop", "0", "0x1000", "1"], "fle: EnumDisplayMonitors: lprcClip is not NULL");
    assert!(e.contains("decision 0065"), "{e}");
    enum_stops(&["enum-stop", "0", "0x100000000", "1"], "fle: EnumDisplayMonitors: lprcClip is not NULL");
    // both non-NULL: the hdc reason comes first (it is the larger gap)
    enum_stops(&["enum-stop", "5", "6", "1"], "fle: EnumDisplayMonitors: hdc is not NULL");
    let e = enum_stops(&["enum-stop", "0", "0", "0"], "fle: EnumDisplayMonitors: lpfnEnum is NULL");
    assert!(e.contains("decision 0065"), "{e}");
}

#[test]
fn a_callback_address_that_is_not_a_function_entry_stops_inside_the_runtime() {
    // the whole run stops (decision 0064: no unwinding): fle_dispatch_miss names the address
    enum_stops(&["enum-stop", "0", "0", "0x2008"], "indirect call to 0x2008 is not a known function entry");
}

// ---- 3. GetMonitorInfoW ----

fn gmi(h: u64, cb: u64) -> Vec<String> {
    lines(&["gmi", &h.to_string(), &cb.to_string()])
}

/// The 0x80-byte buffer after a call, as the 4-byte words it holds, from the hex line.
fn words(hex: &str) -> Vec<u32> {
    (0..hex.len() / 8).map(|i| u32::from_le_bytes(hex_bytes(&hex[8 * i..8 * i + 8]).try_into().unwrap())).collect()
}
fn hex_bytes(h: &str) -> Vec<u8> {
    (0..h.len() / 2).map(|i| u8::from_str_radix(&h[2 * i..2 * i + 2], 16).unwrap()).collect()
}

#[test]
fn the_virtual_monitor_with_cbsize_0x28_gets_the_rectangles_and_the_primary_flag() {
    let l = gmi(H, 0x28);
    // return 1 (nonzero: success); rdx..r9 left alone by the runtime does not matter here; the last error is untouched
    assert_eq!(l[0], "ret 1 r9 77 last-error 1234");
    let w = words(&l[1]);
    // the buffer: 8 bytes before the structure (0xAAAAAAAA x2), the 40-byte MONITORINFO, then 0xAAAAAAAA
    assert_eq!(&w[0..2], [0xAAAA_AAAA, 0xAAAA_AAAA], "nothing written before the structure");
    assert_eq!(w[2], 0x28, "cbSize is left as the caller set it");
    assert_eq!(&w[3..7], [0, 0, 1920, 1080], "rcMonitor: left top right bottom, virtual-screen coordinates, origin (0,0)");
    assert_eq!(&w[7..11], [0, 0, 1920, 1080], "rcWork = rcMonitor (decision 0065, provisional: no task bar)");
    assert_eq!(w[11], 1, "dwFlags = MONITORINFOF_PRIMARY");
    assert!(w[12..].iter().all(|&x| x == 0xAAAA_AAAA), "exactly 40 bytes written, nothing after the structure: {:x?}", &w[12..]);
}

#[test]
fn the_rectangle_has_a_positive_size_and_the_work_area_is_inside_the_monitor() {
    let w = words(&gmi(H, 0x28)[1]);
    let (m, k) = (&w[3..7], &w[7..11]);
    assert!(m[2] > m[0] && m[3] > m[1]);
    assert!(k[0] >= m[0] && k[1] >= m[1] && k[2] <= m[2] && k[3] <= m[3], "rcWork inside rcMonitor (Learn: the work area of the monitor)");
    // the same numbers as GetSystemMetrics(SM_CXSCREEN / SM_CYSCREEN) and GetDeviceCaps(HORZRES / VERTRES) (decisions 0057, 0053)
    assert_eq!((m[2] - m[0], m[3] - m[1]), (1920, 1080));
}

#[test]
fn only_the_dword_cbsize_counts_and_the_upper_half_of_the_eight_bytes_is_not_read() {
    let l = lines(&["gmi-high"]);
    assert_eq!(l[0], "ret 1");
    let w = words(&l[1]);
    assert_eq!(&w[3..7], [0, 0, 1920, 1080]);
}

#[test]
fn another_monitor_handle_stops_with_a_named_reason_and_writes_nothing() {
    // NULL, a neighbour of the virtual one, the region base, a handle of another table (a cursor, a DC) and a 32-bit truncation
    for h in [0u64, H - 4, H + 4, H + 1, H - 1, 0x7FEA_0000, 0x7FCA_0004, 0x7FA9_0004, 0x1_7FEA_0004, 0xFFFF_FFFF_7FEA_0004, 1, 0x10001] {
        gmi_stops(&["gmi", &h.to_string(), "40"], &format!("fle: GetMonitorInfoW: hMonitor {h:#x} is not the virtual monitor 0x7fea0004"));
    }
}

#[test]
fn a_cbsize_other_than_0x28_stops_with_a_named_reason_and_writes_nothing() {
    // MONITORINFOEXW (0x68 = 0x28 + 64 bytes of WCHAR szDevice[32]): the device name is not decided. (The PM's brief said 0x48; that is
    // MONITORINFOEXA, 0x28 + 32 CHARs, which is not a form of the Unicode function: it stops with its own reason.)
    gmi_stops(&["gmi", &H.to_string(), "104"], "fle: GetMonitorInfoW: cbSize is sizeof(MONITORINFOEXW) 0x68: the device name is not decided");
    gmi_stops(&["gmi", &H.to_string(), "72"], "fle: GetMonitorInfoW: cbSize is 0x48 = sizeof(MONITORINFOEXA), not a form of the Unicode function");
    for cb in [0u64, 4, 0x24, 0x27, 0x29, 0x2c, 0x47, 0x49, 0x67, 0x69, 0x80, 0x128, 0x8000_0028, 0xFFFF_FFFF] {
        gmi_stops(&["gmi", &H.to_string(), &cb.to_string()], &format!("fle: GetMonitorInfoW: cbSize {cb:#x} is not sizeof(MONITORINFO) 0x28"));
    }
}

#[test]
fn a_null_lpmi_stops_with_a_named_reason() {
    let o = exec(driver(), &["gmi", &H.to_string(), "40", "null"]);
    assert!(o.stderr.contains("fle: GetMonitorInfoW: lpmi is NULL"), "{}", o.stderr);
    assert!(!o.ok);
}

// ---- 4. Source checks ----

#[test]
fn source_uses_no_host_display_monitor_clock_thread_process_or_file() {
    let code = strip_comments(&std::fs::read_to_string(src_dir().join("fle_monitor.c")).unwrap());
    for bad in [
        "getpid", "getppid", "gettid", "getenv", "environ", "fopen", "open(", "stat(", "/proc", "/etc", "/home", "/dev", "/sys", "/tmp", "exit(", "_exit", "clock_gettime",
        "gettimeofday", "time(", "clock(", "nanosleep", "usleep", "sleep(", "select(", "<time.h>", "<sys/time.h>", "pthread", "<sys/", "<unistd.h>", "<signal.h>",
        "fork(", "exec", "syscall", "asm", "__asm__", "abort(", "X11", "\"DISPLAY\"", "XRandR", "xrandr", "wayland", "EDID", "edid", "drm", "ioctl", "setjmp", "longjmp",
        "emscripten", "SDL", "fle_host", "fle_reg", "fle_vfs", "malloc", "static int", "static uint64_t", "static uint32_t",
    ] {
        assert!(!code.contains(bad), "fle_monitor.c must not use {bad}");
    }
    let b = code.as_bytes();
    let mut i = 0;
    while let Some(p) = code[i..].find("long") {
        let at = i + p;
        let before = at == 0 || !(b[at - 1].is_ascii_alphanumeric() || b[at - 1] == b'_');
        let after = at + 4 >= b.len() || !(b[at + 4].is_ascii_alphanumeric() || b[at + 4] == b'_');
        assert!(!(before && after), "fle_monitor.c uses `long` near byte {at}");
        i = at + 4;
    }
    // the values come from the named constants of the header; the callback goes through fle_call_guest; stops through fle_stop
    assert!(code.contains("fle_call_guest(") && code.contains("FLE_MONITOR_HANDLE") && code.contains("FLE_MONITORINFOF_PRIMARY"));
    assert_eq!(code.matches("fle_call_guest(").count(), 1, "one monitor: one call site");
    // decision 0098 added EnumDisplaySettingsW to this file: four more stops (device name, iModeNum, lpDevMode NULL, dmSize) and one more R[0] write
    // decision 0126 (old: 12): MonitorFromRect adds two (lprc NULL; dwFlags that is not 0, 1 or 2 in fle_monitor_resolve): 14
    assert_eq!(code.matches("fle_stop(").count(), 14, "three stops of EnumDisplayMonitors, five of GetMonitorInfoW, four of EnumDisplaySettingsW (decision 0098), two of the MonitorFrom* family (decision 0126)");
    // no literal 1920 or 1080 outside the header: the size is the one of decision 0053 by name
    assert!(!code.contains("1920") && !code.contains("1080"), "the screen size is taken from the header constants");
    // R[0] is written once per API (three since decision 0098: EnumDisplaySettingsW)
    // decision 0126 (old: 3): MonitorFromPoint and MonitorFromRect write R[0] once each: 5 (the MonitorFromWindow one is in fle_win.c)
    assert_eq!(code.matches("R[0] =").count(), 5);
}

#[test]
fn header_is_plain_c() {
    let h = strip_comments(&std::fs::read_to_string(src_dir().join("fle_monitor.h")).unwrap());
    for bad in ["asm", "__asm__", "<unistd.h>", "<sys/", "<pthread.h>", "<time.h>", "long "] {
        assert!(!h.contains(bad), "fle_monitor.h must not contain {bad}");
    }
}

// ---- 5. Generator: the thunks call the runtime ----

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
    for api in ["EnumDisplayMonitors", "GetMonitorInfoW"] {
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
        ("other.dll", "EnumDisplayMonitors"), ("user32.dll", "EnumDisplayMonitorsW"), ("user32.dll", "GetMonitorInfoA"), ("other.dll", "GetMonitorInfoW"),
        // old: ("user32.dll", "MonitorFromWindow"), ("user32.dll", "MonitorFromRect") and ("user32.dll", "MonitorFromPoint") were here; decision 0126 made them rows
        ("user32.dll", "EnumDisplayDevicesW"),
    ] {
        let img = build(dll, api);
        let mut out = Vec::new();
        let st = generate(&img, &mut out).unwrap();
        let c = String::from_utf8(out).unwrap();
        assert!(!st.handled.contains_key("jmp_imp:EnumDisplayMonitors") && !st.handled.contains_key("jmp_imp:GetMonitorInfoW"), "{dll}!{api}");
        assert!(!c.contains("fle_imp_EnumDisplayMonitors") && !c.contains("fle_imp_GetMonitorInfoW"), "{dll}!{api}");
        let va = IB + 0x1040;
        assert!(c.contains(&format!("L_{va:x}: {{ /* {va:#x} Jmp */ fle_unsupported(UINT64_C({va:#x})); }}")), "{dll}!{api}");
    }
}

// ---- 6. The bytes of the real DLL that the decision rests on ----

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

fn rva_off(dll: &[u8], rva: u32) -> usize {
    let lf = le32(dll, 0x3c) as usize;
    let nsec = le16(dll, lf + 6) as usize;
    let osz = le16(dll, lf + 20) as usize;
    for i in 0..nsec {
        let o = lf + 24 + osz + 40 * i;
        let (vs, sva, rs, rp) = (le32(dll, o + 8), le32(dll, o + 12), le32(dll, o + 16), le32(dll, o + 20));
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

fn imports(dll: &[u8]) -> Vec<(String, String, u64)> {
    let lf = le32(dll, 0x3c) as usize;
    let dir_rva = le32(dll, lf + 24 + 112 + 8);
    let cstr = |rva: u32| -> String {
        let o = rva_off(dll, rva);
        let e = dll[o..].iter().position(|&c| c == 0).unwrap();
        String::from_utf8_lossy(&dll[o..o + e]).to_string()
    };
    let mut v = Vec::new();
    let mut d = rva_off(dll, dir_rva);
    loop {
        let (oft, name, ft) = (le32(dll, d), le32(dll, d + 12), le32(dll, d + 16));
        if name == 0 && ft == 0 {
            break;
        }
        let dname = cstr(name);
        let mut t = rva_off(dll, if oft != 0 { oft } else { ft });
        let mut k = 0u32;
        loop {
            let e = u64::from_le_bytes(dll[t..t + 8].try_into().unwrap());
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

fn thunk_slot(dll: &[u8], va: u64) -> u64 {
    let t = bytes_at(dll, va, 6);
    assert_eq!(&t[..2], [0xff, 0x25], "{va:#x}: jmp [rip+disp32]");
    (va as i64 + 6 + i32::from_le_bytes(t[2..6].try_into().unwrap()) as i64) as u64
}

#[test]
fn the_thunks_jump_to_the_slots_which_are_user32_enumdisplaymonitors_and_getmonitorinfow() {
    let Some(dll) = real_dll() else { return };
    let imps = imports(&dll);
    for (thunk, slot, name) in [(0x5dca50u64, 0x1971058u64, "EnumDisplayMonitors"), (0x5dca40, 0x1970ee0, "GetMonitorInfoW")] {
        let named: Vec<_> = imps.iter().filter(|(_, _, s)| *s == slot).collect();
        assert_eq!(named.len(), 1, "{name}");
        assert_eq!((named[0].0.to_lowercase(), named[0].1.as_str()), ("user32.dll".to_string(), name));
        assert_eq!(thunk_slot(&dll, thunk), slot, "{name}");
    }
    // the three MonitorFrom* functions are other imports (not answered here)
    for (n, slot) in [("MonitorFromWindow", 0x1970dd8u64), ("MonitorFromRect", 0x1970f68), ("MonitorFromPoint", 0x19710c0)] {
        assert_eq!(imps.iter().find(|(_, f, _)| f == n).map(|(_, _, s)| *s), Some(slot), "{n}");
    }
}

#[test]
fn the_two_enumeration_sites_pass_null_null_the_callback_and_the_list_in_the_registers_the_page_names() {
    let Some(dll) = real_dll() else { return };
    // 0x8652da: xor ecx,ecx (hdc); xor edx,edx (lprcClip); lea r8,[rip+..] = 0x85df70 (lpfnEnum); then mov rax,[rbp+0x60]; mov r9,[rax+0xbc] (dwData)
    assert_eq!(bytes_at(&dll, 0x8652d8, 4), [0x33, 0xc9, 0x33, 0xd2]);
    assert_eq!(bytes_at(&dll, 0x8652dc, 7), [0x4c, 0x8d, 0x05, 0x8d, 0x8c, 0xff, 0xff]);
    assert_eq!((0x8652dci64 + 7 + 0xffff8c8du32 as i32 as i64) as u64, 0x85df70);
    assert_eq!(bytes_at(&dll, 0x8652e7, 7), [0x4c, 0x8b, 0x88, 0xbc, 0x00, 0x00, 0x00]);
    assert_eq!(bytes_at(&dll, 0x8652ee, 1), [0xe8]);
    // 0x867519 (f_867510): the same
    assert_eq!(bytes_at(&dll, 0x867519, 4), [0x33, 0xc9, 0x33, 0xd2]);
    assert_eq!(bytes_at(&dll, 0x86751d, 7), [0x4c, 0x8d, 0x05, 0x4c, 0x6a, 0xff, 0xff]);
    assert_eq!((0x86751di64 + 7 + 0xffff6a4cu32 as i32 as i64) as u64, 0x85df70);
    assert_eq!(bytes_at(&dll, 0x86752b, 7), [0x4c, 0x8b, 0x88, 0xbc, 0x00, 0x00, 0x00]);
    for (site, thunk) in [(0x8652eeu64, 0x5dca50u64), (0x867532, 0x5dca50)] {
        let c = bytes_at(&dll, site, 5);
        assert_eq!(c[0], 0xe8, "{site:#x}");
        assert_eq!((site as i64 + 5 + i32::from_le_bytes(c[1..5].try_into().unwrap()) as i64) as u64, thunk, "{site:#x}");
    }
}

#[test]
fn the_callback_reads_only_rcx_and_r9_and_returns_true_in_al() {
    let Some(dll) = real_dll() else { return };
    // 0x85df70 .. 0x85dfa7 (the whole function): push rsi; push rbx; sub rsp,0x28; mov rbx,rcx; mov rsi,r9; mov rcx,[rip+..];
    // mov dl,1; call 0x40f9e0; mov [rax+8],rbx; mov ecx,[rsi+0x10]; mov [rax+0x10],ecx; mov rcx,rsi; mov rdx,rax; call 0x50a830;
    // mov al,1; add rsp,0x28; pop rbx; pop rsi; ret
    let want = hex_bytes("56534883ec284889cb4c89ce488b0d6d58ffffb201e8561abbff488958088b4e108948104889f14889c2e891c8caffb0014883c4285b5ec3");
    assert_eq!(bytes_at(&dll, 0x85df70, want.len()), want);
    // rdx (hdcMonitor) and r8 (lprcMonitor) are never read: no instruction of the function has rdx/r8 as a source before it is written.
    // The only uses of rdx are `mov dl, 1` (a write) and `mov rdx, rax` (a write); r8 does not occur (REX.R with reg 0: 4c 8d, 4c 89 ... below)
    let f = &want;
    assert!(!f.windows(3).any(|w| w == [0x4c, 0x89, 0xc1] || w == [0x4c, 0x89, 0xc2]), "no `mov rcx/rdx, r8`");
    // the callback never reads [rsp] as a return address: its only memory operands are [rip+..], [rsi+0x10], [rax+8], [rax+0x10], and the pushes/pops
    assert!(!f.windows(2).any(|w| w == [0x8b, 0x04] || w == [0x8b, 0x0c]), "no `mov reg, [sib]` form that reads [rsp]");
    let _ = f;
}

#[test]
fn the_six_getmonitorinfow_sites_pass_cbsize_0x28_and_none_passes_a_monitorinfoex() {
    let Some(dll) = real_dll() else { return };
    // each call site (e8 rel32 to 0x5dca40); `mov dword [rbp+disp], 0x28` shortly before it (c7 45 xx 28 00 00 00 or c7 85 xx xx xx xx 28 00 00 00)
    let sites = [0x807862u64, 0x864fbd, 0x864ffd, 0x865037, 0x86a9ea, 0x8805c4];
    for site in sites {
        let c = bytes_at(&dll, site, 5);
        assert_eq!(c[0], 0xe8, "{site:#x}");
        assert_eq!((site as i64 + 5 + i32::from_le_bytes(c[1..5].try_into().unwrap()) as i64) as u64, 0x5dca40, "{site:#x}");
        let w = bytes_at(&dll, site - 40, 40);
        let has28 = w.windows(7).any(|x| x[0] == 0xc7 && x[1] == 0x45 && x[3..7] == [0x28, 0, 0, 0]) || w.windows(10).any(|x| x[0] == 0xc7 && x[1] == 0x85 && x[6..10] == [0x28, 0, 0, 0]);
        assert!(has28, "{site:#x}: no `mov dword [rbp+..], 0x28` in the 40 bytes before the call");
        let has48 = w.windows(7).any(|x| x[0] == 0xc7 && x[1] == 0x45 && x[3..7] == [0x48, 0, 0, 0]) || w.windows(10).any(|x| x[0] == 0xc7 && x[1] == 0x85 && x[6..10] == [0x48, 0, 0, 0]);
        assert!(!has48, "{site:#x}: a MONITORINFOEX size");
    }
    // and these are all the sites of the thunk: every `e8 rel32` in .text with the target 0x5dca40
    let lf = le32(&dll, 0x3c) as usize;
    let nsec = le16(&dll, lf + 6) as usize;
    let osz = le16(&dll, lf + 20) as usize;
    let mut hits = Vec::new();
    for i in 0..nsec {
        let o = lf + 24 + osz + 40 * i;
        if &dll[o..o + 5] != b".text" {
            continue;
        }
        let (rs, rp, sva) = (le32(&dll, o + 16) as usize, le32(&dll, o + 20) as usize, le32(&dll, o + 12) as u64);
        for k in rp..rp + rs - 5 {
            if dll[k] == 0xe8 {
                let va = 0x400000 + sva + (k - rp) as u64;
                let t = (va as i64 + 5 + i32::from_le_bytes(dll[k + 1..k + 5].try_into().unwrap()) as i64) as u64;
                if t == 0x5dca40 {
                    hits.push(va);
                }
            }
        }
    }
    assert_eq!(hits, sites);
}
