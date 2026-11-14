//! Tests of decision 0127 (docs/decisions/0127-reproducible-fruity-and-pinned-answers.md, PROVISIONAL): they pin the answers that decision 0126 added (the 33 response rows 544 -> 577 and the new behaviour of the runtime)
//! to values that come from Microsoft Learn or from a named choice without a source (`S-...` of decision 0126). A test fails when the answer changes.
//!
//! Where the expected value is a Learn rule, the comment names the page and quotes the words (pages fetched 2026-11-01 by decision 0127's work; the URL of each page and the SHA-256 of what was fetched are in docs/decisions/0127-reproducible-fruity-and-pinned-answers.md). Where the value is a
//! choice without a source, the comment names it (S-...) and the test says "choice": it fixes the choice, not a Windows rule. Values are never read back from the C code of the runtime.
//!
//! The harness links EVERY runtime file (fle_*.c, sorted: a unit that links the whole response table must link all of them; modules.rs does the same) so that a new runtime file cannot make the link list stale (decision
//! 0126: tests that link only some fle_*.c needed fle_raster.c added). The guest is a C program that calls the `fle_imp_*` functions with the x64 argument registers (R[1] rcx, R[2] rdx, R[8] r8, R[9] r9, stack
//! arguments at [rsp+0x28]...).
//!
//! 1. The 33 rows (544 -> 577): names, order, no pseudo-VA moved, each thunk answered under its own dll only.
//! 2. Raster: the ROP3 codes through BitBlt/PatBlt, ROP2 through LineTo and Rectangle, FillRect, FrameRect, the pixel calls, StretchDIBits, GetDIBits/SetDIBits, MaskBlt, clipping.
//! 3. The environment answers: CompareStringW, SHGetFolderPathW, GetKeyState, ReadFile of 0 bytes, IsDebuggerPresent, MonitorFrom*, the RECT calls, FormatMessageW, DirectSoundEnumerateA, midiOutGetDevCapsW,
//!    RegCreateKeyExW, the cursor calls, SetThreadErrorMode, LockWindowUpdate, WM_SETREDRAW, menus, SLIST, hooks.

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
    if let Ok(d) = std::env::var("FLE_FR11_SRC_DIR") {
        return PathBuf::from(d);
    }
    rt_dir()
}
fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-wallFR11/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

// ---------------------------------------------------------------------------------------------
// 1. Generator: the 33 rows of decision 0126
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

/// The 33 rows of decision 0126 (imports_table.rs lines 544 -> 577 of the table, in the order of the table: append only, decision 0015). The names are those of the table; the dll is the one the row names.
const ROWS_0126: [(&str, &str); 33] = [
    ("gdi32.dll", "CreatePen"),
    ("user32.dll", "GetKeyState"),
    ("user32.dll", "GetAsyncKeyState"),
    ("user32.dll", "CheckMenuItem"),
    ("user32.dll", "EnableMenuItem"),
    ("user32.dll", "LockWindowUpdate"),
    ("kernel32.dll", "WaitForMultipleObjectsEx"),
    ("user32.dll", "GetPhysicalCursorPos"),
    ("user32.dll", "SetPhysicalCursorPos"),
    ("user32.dll", "SetCursorPos"),
    ("kernel32.dll", "FormatMessageW"),
    ("user32.dll", "GetCursorInfo"),
    ("dsound.dll", "DirectSoundEnumerateA"),
    ("kernel32.dll", "InterlockedPushEntrySList"),
    ("kernel32.dll", "InterlockedPopEntrySList"),
    ("user32.dll", "AdjustWindowRectEx"),
    ("wininet.dll", "InternetGetConnectedState"),
    ("shell32.dll", "ILCreateFromPathW"),
    ("shell32.dll", "SHChangeNotifyRegister"),
    ("shell32.dll", "SHChangeNotifyDeregister"),
    ("ole32.dll", "RegisterDragDrop"),
    ("ole32.dll", "RevokeDragDrop"),
    ("user32.dll", "GetUpdateRgn"),
    ("user32.dll", "ValidateRgn"),
    ("user32.dll", "InflateRect"),
    ("user32.dll", "EqualRect"),
    ("user32.dll", "PtInRect"),
    ("user32.dll", "SubtractRect"),
    ("user32.dll", "MonitorFromWindow"),
    ("user32.dll", "MonitorFromPoint"),
    ("user32.dll", "MonitorFromRect"),
    ("user32.dll", "SetWindowsHookExW"),
    ("user32.dll", "UnhookWindowsHookEx"),
];

#[test]
fn the_33_rows_of_decision_0126_are_the_last_rows_in_the_order_given_and_no_pseudo_va_moved() {
    let (st, c) = gen(&GUEST_IMPORTS);
    let rows = resp_rows(&c);
    assert_eq!(rows.len(), st.resp_entries);
    // decision 0129 (old: `let n = rows.len();` and `assert_eq!(n, 544 + 33)`): decision 0129 appended 4 more rows after these 33 (tests/wall_fr12.rs): the 33 end at rows.len() - 4
    // decision 0136: rows are appended at the END again (and decision 0132 appends more): the end of the group is found by the NAME of its last row, not by counting from the end of the table.
    let last = ROWS_0126[ROWS_0126.len() - 1];
    let n = rows.iter().position(|r| (r.0.as_str(), r.1.as_str()) == last).expect("the last row of the group") + 1;
    assert_eq!(n, 544 + 33, "the 544 rows of decision 0125's tree and the 33 of decision 0126");
    // decision 0015: append only. 544 rows at HEAD 0c39ffd (decision 0125 ended with gdi32!PtInRegion, user32!GetWindowDC, user32!GetDCEx), 544 + 33 = 577 at e2b3d50
    assert_eq!((rows[n - 34].0.as_str(), rows[n - 34].1.as_str()), ("user32.dll", "GetDCEx"), "the row before the 33 is the last of decision 0125");
    for (k, (dll, name)) in ROWS_0126.iter().enumerate() {
        assert_eq!((rows[n - 33 + k].0.as_str(), rows[n - 33 + k].1.as_str()), (*dll, *name), "row {}", n - 33 + k);
    }
    for (k, (_, _, va)) in rows.iter().enumerate() {
        assert_eq!(*va, 0x7FF9_0000_0000u64 + 0x10 * (k as u64 + 1), "row {k}: the pseudo-VA of a row never moves");
    }
    for (_, name) in ROWS_0126 {
        assert_eq!(rows.iter().filter(|(_, n2, _)| n2 == name).count(), 1, "{name}: exactly one row");
        assert!(c.contains(&format!("void fle_imp_{name}(void);")), "{name} is declared in the generated C");
    }
}

#[test]
fn each_of_the_33_thunks_is_answered_under_its_own_dll_and_no_other_dll() {
    for (dll, name) in ROWS_0126 {
        let (st, c) = gen(&[(dll, name), ("kernel32.dll", name), (dll, "SendMessageA"), ("kernel32.dll", "GetProcAddress")]);
        // the same name under kernel32 is a second import: for a kernel32 row both are answered; for any other dll only the one of the row
        let own = if dll == "kernel32.dll" { 2 } else { 1 };
        assert_eq!(st.handled[format!("jmp_imp:{name}").as_str()], own, "{dll}!{name}");
        assert_eq!(c.matches(&format!("fle_imp_{name}(); return;")).count(), own, "{dll}!{name}: answered only under the dll of its row");
        if dll != "kernel32.dll" {
            let va = IB + 0x1040 + 0x10;
            assert!(c.contains(&format!("L_{va:x}: {{ /* {va:#x} Jmp */ fle_unsupported(UINT64_C({va:#x})); }}")), "{dll}!{name}: the thunk of kernel32!{name} stays unsupported");
        }
    }
}

// ---------------------------------------------------------------------------------------------
// 2. The driver: every runtime file, and the call helpers
// ---------------------------------------------------------------------------------------------

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
    let cc = cmd.arg("-I").arg(src_dir()).arg("-I").arg(rt_dir()).arg(src).arg("-o").arg(obj).env("TMPDIR", work_dir().join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {}: {}", src.display(), String::from_utf8_lossy(&cc.stderr));
}

/// Every runtime file is compiled once per process. The files that decision 0126 added or changed are built with -Werror from `src_dir()` (FLE_FR11_SRC_DIR: the mutation runs of decision 0127 use it).
fn objs() -> &'static Vec<PathBuf> {
    static O: OnceLock<Vec<PathBuf>> = OnceLock::new();
    O.get_or_init(|| {
        let dir = work_dir().join(etxt_common::per_process("obj-fr11"));
        std::fs::create_dir_all(&dir).unwrap();
        let ours = [
            "fle_raster.c", "fle_gdi.c", "fle_win.c", "fle_monitor.c", "fle_user_nowin.c", "fle_vfs.c", "fle_env.c", "fle_locale.c", "fle_proc.c", "fle_imports.c", "fle_reg.c", "fle_winmm.c", "fle_dsound.c",
            "fle_wininet.c", "fle_spi.c", "fle_ole.c", "fle_sys.c", "fle_thread.c", "fle_load.c", "fle_modules.c",
        ];
        let mut v = Vec::new();
        for f in runtime_sources() {
            let name = f.file_name().unwrap().to_string_lossy().to_string();
            let is_ours = ours.contains(&name.as_str());
            let base = if is_ours { src_dir() } else { rt_dir() };
            let src = if base.join(&name).exists() { base.join(&name) } else { f.clone() };
            let o = dir.join(name.replace(".c", ".o"));
            clang_c(&src, &o, is_ours);
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
#include "fle_cursor.h"
#include "fle_res.h"
#include "fle_locale.h"
#include "fle_sysinfo.h"
#include "fle_monitor.h"
#include "fle_user_nowin.h"
#include "fle_winmm.h"
#include "fle_dsound.h"
#include "fle_wininet.h"
#include "fle_ole.h"
#include "fle_reg.h"
#include "fle_raster.h"
#include "fle_spi.h"
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
#define NEG(n) ((uint64_t)(int64_t)(n))

/* callN(f, n, args...): the first four arguments in rcx, rdx, r8, r9, the fifth and later at [rsp+0x28] ... of the callee's entry (rsp points at the return address) */
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
/* K(f, args...): up to four arguments (registers only); KN(f, n, args...): n arguments, the fifth and later on the stack */
#define K(f, ...) callv((void (*)(void))(f), 4, (uint64_t[16]){ __VA_ARGS__ })
#define KN(f, n, ...) callv((void (*)(void))(f), (n), (uint64_t[16]){ __VA_ARGS__ })
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
static void wtoa(uint64_t p, char *out, size_t cap)
{
    size_t i;
    for (i = 0; i + 1 < cap; i++) { uint16_t c = *(uint16_t *)(uintptr_t)(p + 2 * i); if (!c) break; out[i] = (char)c; }
    out[i] = 0;
}

/* a DIB section (32 bit top-down) of w x h pixels and a memory DC that has it selected; pixels are read back with PX */
static uint8_t bmi[40 + 12];
static uint64_t bits_out;
static void p32(unsigned o, uint32_t v) { memcpy(bmi + o, &v, 4); }
static uint64_t MKDC(void) { return K(fle_imp_CreateCompatibleDC, 0); }
static uint64_t MKDIB(int32_t w, int32_t h, unsigned bpp)
{
    uint16_t one = 1, b = (uint16_t)bpp;
    uint64_t scr = K(fle_imp_GetDC, 0);
    memset(bmi, 0, sizeof bmi);
    /* BITMAPINFOHEADER (Learn): "If biHeight is negative, the bitmap is a top-down DIB with the origin at the upper left corner": the height is given negative, so device row y is memory row y */
    p32(0, 40); p32(4, (uint32_t)w); p32(8, (uint32_t)(-h)); memcpy(bmi + 12, &one, 2); memcpy(bmi + 14, &b, 2); p32(16, 0);
    bits_out = 0;
    return KN(fle_imp_CreateDIBSection, 6, scr, A(bmi), 0, A(&bits_out), 0, 0);
}
static uint8_t *BITS(uint64_t h)
{
    uint64_t b = 0;
    uint32_t o[6];
    return fle_gdi_dib_info(h, &b, o) ? (uint8_t *)(uintptr_t)b : NULL;
}
/* the colour of pixel (x, y) of a 32-bit top-down DIB of width w as 0x00RRGGBB (stored B, G, R, unused) */
static uint32_t PX(uint64_t dib, int w, int x, int y)
{
    const uint8_t *p = BITS(dib) + 4 * ((size_t)y * (size_t)w + (size_t)x);
    return (uint32_t)p[2] << 16 | (uint32_t)p[1] << 8 | p[0];
}
static uint64_t PXRAW(uint64_t dib, int w, int x, int y)
{
    const uint8_t *p = BITS(dib) + 4 * ((size_t)y * (size_t)w + (size_t)x);
    return (uint64_t)p[0] | (uint64_t)p[1] << 8 | (uint64_t)p[2] << 16 | (uint64_t)p[3] << 24;
}
static void FILLALL(uint64_t dib, int w, int h, int v) { memset(BITS(dib), v, (size_t)w * (size_t)h * 4); }
static uint64_t SOLID(uint32_t c) { return K(fle_imp_CreateSolidBrush, c); }
static uint32_t rcv[4];
static uint64_t RCT(int32_t l, int32_t t, int32_t r, int32_t b) { rcv[0] = (uint32_t)l; rcv[1] = (uint32_t)t; rcv[2] = (uint32_t)r; rcv[3] = (uint32_t)b; return A(rcv); }

/* the window procedure stub: every message gets the creation answers (WM_NCCREATE 1, WM_NCCALCSIZE 0, WM_CREATE 0) and otherwise the default processing, as the guest's own handler would call DefWindowProcW */
#define VA_WP 0x2000
static uint32_t wp_count[0x400];
static uint64_t last_ctx_target, last_ctx_wp, last_ctx_lp;
static void wndproc(void)
{
    uint64_t h = R[1], msg = R[2], wp = R[8], lp = R[9], sp = R[4];
    if (msg < 0x400) wp_count[msg]++;
    if (msg == 0x007B) { last_ctx_target = h; last_ctx_wp = wp; last_ctx_lp = lp; }
    if (msg == 0x81) { R[0] = 1; return; }
    if (msg == 0x83) { R[0] = 0; return; }
    if (msg == 0x01) { R[0] = 0; return; }
    R[1] = h; R[2] = msg; R[8] = wp; R[9] = lp;
    R[4] = sp - 8; fle_st(R[4], 8, 0xAAAA0001);
    fle_imp_DefWindowProcW();
    R[4] = sp;
}
/* the IUnknown methods of a guest drop target (vtable slots 1 and 2): they only count their calls and remember the object they were called with */
#define VA_ADDREF  0x3000
#define VA_RELEASE 0x3010
static uint32_t addref_calls, release_calls;
static uint64_t last_unknown_this;
static void addref_stub(void) { addref_calls++; last_unknown_this = R[1]; R[0] = 2; }
static void release_stub(void) { release_calls++; last_unknown_this = R[1]; R[0] = 1; }
static int cls_done;
static uint16_t cls_name[] = { 'T','P','U','t','i','l','W','i','n','d','o','w', 0 };
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
static uint64_t cw(uint64_t ex, const char *title, uint64_t style, int32_t x, int32_t y, int32_t w, int32_t hh, uint64_t parent)
{
    uint64_t sp0 = R[4], h;
    regclass();
    R[4] = (R[4] - 0x80) & ~UINT64_C(15); R[4] -= 8; fle_st(R[4], 8, 0xAAAA0000);
    fle_st(R[4] + 0x28, 8, (uint32_t)x); fle_st(R[4] + 0x30, 8, (uint32_t)y); fle_st(R[4] + 0x38, 8, (uint32_t)w); fle_st(R[4] + 0x40, 8, (uint32_t)hh);
    fle_st(R[4] + 0x48, 8, parent); fle_st(R[4] + 0x50, 8, 0); fle_st(R[4] + 0x58, 8, 0x400000); fle_st(R[4] + 0x60, 8, 0);
    R[1] = ex; R[2] = A(cls_name); R[8] = title ? S(title) : 0; R[9] = style;
    fle_imp_CreateWindowExW();
    h = R[0];
    R[4] = sp0;
    return h;
}
#define POPUP 0x80000000u
#define CHILD 0x40000000u
#define VIS   0x10000000u
/* the colour (0x00RRGGBB) of pixel (x, y) of the drawing surface of the top-level window with the logical id lid (the surface is made by the first call that draws or reads) */
static uint32_t SPX(uint32_t lid, int x, int y)
{
    const fle_rsurf *s = fle_raster_surface_peek(lid);
    const uint8_t *p;
    if (!s || (uint32_t)x >= s->width || (uint32_t)y >= s->height) return 0xFFFFFFFFu;
    p = fle_r_row(s, (uint32_t)y) + 4 * (size_t)x;
    return (uint32_t)p[2] << 16 | (uint32_t)p[1] << 8 | p[0];
}
"##;

fn run_src(name: &str, body: &str) -> Out {
    let dir = work_dir();
    let uname = etxt_common::uniq(name);
    let c = dir.join(format!("{uname}.c"));
    let exe = dir.join(&uname);
    let src = format!("{PRELUDE}\nconst fle_fn_entry fle_dispatch_table[] = {{ {{ VA_WP, wndproc }}, {{ VA_ADDREF, addref_stub }}, {{ VA_RELEASE, release_stub }} }};\nconst uint64_t fle_dispatch_count = 3;\nint main(void)\n{{\n    boot();\n{body}\n    return 0;\n}}\n");
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
    assert!(o.stderr.contains(&format!("fle: {api}: ")) || o.stderr.contains(&format!("fle: {api}(")) || o.stderr.contains(&format!("fle: {api} ")), "{name}: no 'fle: {api}' message: {}", o.stderr);
    assert!(o.stderr.contains(word), "{name}: message lacks {word:?}: {}", o.stderr);
}


// ---------------------------------------------------------------------------------------------
// 3. Raster: what the drawing calls write
// ---------------------------------------------------------------------------------------------
//
// Common set-up of the programs below: a 32-bit top-down DIB section of 4 x 4 pixels selected into a memory DC. A 32-bit DIB pixel is stored B, G, R, unused. `PX` reads 0x00RRGGBB. The colours are COLORREF values 0x00BBGGRR
// as the pages define them (COLORREF: "0x00bbggrr").

#[test]
fn bitblt_applies_the_ternary_code_of_the_page_to_pattern_source_and_destination() {
    // Learn "Ternary Raster Operations": the 256 codes are Boolean functions of P (pattern = the brush), S (source) and D (destination); the table of the page gives for each code the reverse Polish expression:
    // SRCCOPY 0x00CC0020 = S, SRCPAINT 0x00EE0086 = DSo, SRCAND 0x008800C6 = DSa, SRCINVERT 0x00660046 = DSx, SRCERASE 0x00440328 = SDna, NOTSRCCOPY 0x00330008 = Sn, NOTSRCERASE 0x001100A6 = DSon,
    // MERGECOPY 0x00C000CA = PSa, MERGEPAINT 0x00BB0226 = DSno, PATCOPY 0x00F00021 = P, PATPAINT 0x00FB0A09 = DPSnoo, PATINVERT 0x005A0049 = DPx, DSTINVERT 0x00550009 = Dn, BLACKNESS 0x00000042 = 0, WHITENESS 0x00FF0062 = 1.
    // Operands (COLORREF -> 0xRRGGBB): the source pixel is COLORREF 0x00F0CC11 (R 0x11, G 0xCC, B 0xF0 -> 0x11CCF0), the destination pixel COLORREF 0x00AA3355 (R 0x55, G 0x33, B 0xAA -> 0x5533AA), the brush COLORREF
    // 0x00123456 (R 0x56, G 0x34, B 0x12 -> 0x563412). Every colour bit plane is combined alike, so each result is the Boolean function applied to the three 24-bit values.
    let l = lines(
        "fr11-rop3",
        r#"
    uint64_t sdib = MKDIB(4, 4, 32), ddib = MKDIB(4, 4, 32), sdc = MKDC(), ddc = MKDC(), br = SOLID(0x00123456);
    struct { const char *n; uint32_t rop; } t[] = {
        {"SRCCOPY", 0x00CC0020}, {"SRCPAINT", 0x00EE0086}, {"SRCAND", 0x008800C6}, {"SRCINVERT", 0x00660046}, {"SRCERASE", 0x00440328}, {"NOTSRCCOPY", 0x00330008},
        {"NOTSRCERASE", 0x001100A6}, {"MERGECOPY", 0x00C000CA}, {"MERGEPAINT", 0x00BB0226}, {"PATCOPY", 0x00F00021}, {"PATPAINT", 0x00FB0A09}, {"PATINVERT", 0x005A0049},
        {"DSTINVERT", 0x00550009}, {"BLACKNESS", 0x00000042}, {"WHITENESS", 0x00FF0062}, {"DPSxnon", 0x00140869},
    };
    unsigned i;
    K(fle_imp_SelectObject, sdc, sdib);
    K(fle_imp_SelectObject, ddc, ddib);
    K(fle_imp_SelectObject, ddc, br);
    for (i = 0; i < sizeof t / sizeof t[0]; i++) {
        uint64_t r;
        FILLALL(sdib, 4, 4, 0);
        FILLALL(ddib, 4, 4, 0);
        { uint8_t *s = BITS(sdib), *d = BITS(ddib); s[0] = 0xF0; s[1] = 0xCC; s[2] = 0x11; d[0] = 0xAA; d[1] = 0x33; d[2] = 0x55; }
        r = KN(fle_imp_BitBlt, 9, ddc, 0, 0, 1, 1, sdc, 0, 0, t[i].rop);
        printf("%s ret %llu %06x\n", t[i].n, U(r), PX(ddib, 4, 0, 0));
    }
"#,
    );
    // P = 0x563412, S = 0x11CCF0, D = 0x5533AA (0xRRGGBB); the results are the page's postfix expressions applied to the 24-bit values (a = and, o = or, x = xor, n = not)
    let (p, s, d) = (0x563412u32, 0x11CCF0u32, 0x5533AAu32);
    let m = 0xFFFFFFu32;
    let want: Vec<(&str, u32)> = vec![
        ("SRCCOPY", s),                                   // S
        ("SRCPAINT", d | s),                              // DSo
        ("SRCAND", d & s),                                // DSa
        ("SRCINVERT", d ^ s),                             // DSx
        ("SRCERASE", s & !d & m),                         // SDna
        ("NOTSRCCOPY", !s & m),                           // Sn
        ("NOTSRCERASE", !(d | s) & m),                    // DSon
        ("MERGECOPY", p & s),                             // PSa
        ("MERGEPAINT", d | (!s & m)),                     // DSno
        ("PATCOPY", p),                                   // P
        ("PATPAINT", d | (p | (!s & m))),                 // DPSnoo
        ("PATINVERT", d ^ p),                             // DPx
        ("DSTINVERT", !d & m),                            // Dn
        ("BLACKNESS", 0),                                 // 0
        ("WHITENESS", m),                                 // 1
        ("DPSxnon", !(d | (!(p ^ s) & m)) & m),           // D P S x n o n: not(D or not(P xor S))
    ];
    for (k, (name, v)) in want.iter().enumerate() {
        assert_eq!(l[k], format!("{name} ret 1 {v:06x}"), "{name}");
    }
    assert_eq!(l.len(), want.len());
}

#[test]
fn rop2_mix_modes_combine_pen_and_destination_by_the_page_for_lines_and_rectangle_outlines() {
    // Learn "Binary Raster Operations" (SetROP2): the 16 modes with the Boolean function of the pen P and the destination D: R2_BLACK 1 = 0, R2_NOTMERGEPEN 2 = DPon, R2_MASKNOTPEN 3 = DPna, R2_NOTCOPYPEN 4 = Pn,
    // R2_MASKPENNOT 5 = PDna, R2_NOT 6 = Dn, R2_XORPEN 7 = DPx, R2_NOTMASKPEN 8 = DPan, R2_MASKPEN 9 = DPa, R2_NOTXORPEN 10 = DPxn, R2_NOP 11 = D, R2_MERGENOTPEN 12 = DPno, R2_COPYPEN 13 = P, R2_MERGEPENNOT 14 = PDno,
    // R2_MERGEPEN 15 = DPo, R2_WHITE 16 = 1 (the page lists the names and functions; the numbers 1 to 16 are the SDK's, UNOFFICIAL). Pen COLORREF 0x00123456 (R 0x56 G 0x34 B 0x12 -> 0x563412), destination 0x5533AA.
    // LineTo: the pen draws "from the current position up to, but not including, the specified point" (LineTo page), so a line from (0,0) to (1,0) writes exactly the pixel (0,0).
    let l = lines(
        "fr11-rop2",
        r#"
    uint64_t dib = MKDIB(4, 4, 32), dc = MKDC(), pen = K(fle_imp_CreatePen, 0, 1, 0x00123456);
    unsigned m;
    K(fle_imp_SelectObject, dc, dib);
    K(fle_imp_SelectObject, dc, pen);
    for (m = 1; m <= 16; m++) {
        uint64_t ret;
        FILLALL(dib, 4, 4, 0);
        { uint8_t *d = BITS(dib); d[0] = 0xAA; d[1] = 0x33; d[2] = 0x55; }
        K(fle_imp_SetROP2, dc, m);
        K(fle_imp_MoveToEx, dc, 0, 0, 0);
        ret = K(fle_imp_LineTo, dc, 1, 0);
        printf("%u ret %llu px0 %06x px1 %06x\n", m, U(ret), PX(dib, 4, 0, 0), PX(dib, 4, 1, 0));
    }
"#,
    );
    let (p, d, mk) = (0x563412u32, 0x5533AAu32, 0xFFFFFFu32);
    let want: [u32; 16] = [
        0,                         // 1 R2_BLACK: 0
        !(d | p) & mk,             // 2 R2_NOTMERGEPEN: DPon
        d & !p & mk,               // 3 R2_MASKNOTPEN: DPna
        !p & mk,                   // 4 R2_NOTCOPYPEN: Pn
        p & !d & mk,               // 5 R2_MASKPENNOT: PDna
        !d & mk,                   // 6 R2_NOT: Dn
        d ^ p,                     // 7 R2_XORPEN: DPx
        !(d & p) & mk,             // 8 R2_NOTMASKPEN: DPan
        d & p,                     // 9 R2_MASKPEN: DPa
        !(d ^ p) & mk,             // 10 R2_NOTXORPEN: DPxn
        d,                         // 11 R2_NOP: D
        d | (!p & mk),             // 12 R2_MERGENOTPEN: DPno
        p,                         // 13 R2_COPYPEN: P
        p | (!d & mk),             // 14 R2_MERGEPENNOT: PDno
        d | p,                     // 15 R2_MERGEPEN: DPo
        mk,                        // 16 R2_WHITE: 1
    ];
    assert_eq!(l.len(), 16);
    for (k, v) in want.iter().enumerate() {
        // pixel (1,0) is the end point: not drawn, it stays 0 (the destination was zero there)
        assert_eq!(l[k], format!("{} ret 1 px0 {:06x} px1 000000", k + 1, v), "R2 mode {}", k + 1);
    }
}

#[test]
fn lineto_includes_the_start_excludes_the_end_and_moves_the_current_position() {
    // Learn LineTo: "draws a line from the current position up to, but not including, the specified point" and "If LineTo succeeds, the current position is set to the specified ending point"; MoveToEx: lppt "receives the
    // previous current position". Display Device Context Defaults: "Current pen position (0,0)". The pen is a 1-pixel solid pen (CreatePen: "If nWidth is zero, the pen is a single pixel wide") of COLORREF 0x000000FF
    // (red 0xFF). The line from (1,1) to (4,1) writes (1,1), (2,1), (3,1): three pixels. The line is horizontal here; the 45 degree line is in the next test.
    let l = lines(
        "fr11-lineto",
        r#"
    uint64_t dib = MKDIB(6, 6, 32), dc = MKDC(), pen = K(fle_imp_CreatePen, 0, 0, 0x000000FF);
    int32_t prev[2] = { 77, 77 };
    int x, y, n = 0;
    K(fle_imp_SelectObject, dc, dib);
    K(fle_imp_SelectObject, dc, pen);
    FILLALL(dib, 6, 6, 0);
    K(fle_imp_MoveToEx, dc, 1, 1, A(prev));
    printf("prev before the first move %d %d\n", prev[0], prev[1]);
    printf("lineto ret %llu\n", U(K(fle_imp_LineTo, dc, 4, 1)));
    K(fle_imp_MoveToEx, dc, 0, 5, A(prev));
    printf("prev after the line %d %d\n", prev[0], prev[1]);
    for (y = 0; y < 6; y++) for (x = 0; x < 6; x++) if (PX(dib, 6, x, y)) { printf("px %d %d %06x\n", x, y, PX(dib, 6, x, y)); n++; }
    printf("pixels %d\n", n);
"#,
    );
    assert_eq!(l[0], "prev before the first move 0 0");
    assert_eq!(l[1], "lineto ret 1");
    assert_eq!(l[2], "prev after the line 4 1");
    assert_eq!(&l[3..7], ["px 1 1 ff0000", "px 2 1 ff0000", "px 3 1 ff0000", "pixels 3"]);
    assert_eq!(l.len(), 7);
}

#[test]
fn lineto_draws_a_45_degree_line_and_a_vertical_one_from_the_start_up_to_but_not_including_the_end() {
    // Learn LineTo: "draws a line from the current position up to, but not including, the specified point". Which pixels GDI picks between two points that are not on a row, a column or a 45 degree diagonal is not on the page
    // (S-line-dda, choice: the integer Bresenham line of the longer axis); the three cases below are the ones that every rule of a one-pixel line gives alike, so they pin the end-point rule and the direction and not the
    // choice: (0,0) to (3,3) writes (0,0), (1,1), (2,2); (5,5) to (2,2) (towards the origin) writes (5,5), (4,4), (3,3); (1,1) to (1,4) writes (1,1), (1,2), (1,3).
    let l = lines(
        "fr11-lineto-diag",
        r#"
    uint64_t dib = MKDIB(6, 6, 32), dc = MKDC(), pen = K(fle_imp_CreatePen, 0, 0, 0x000000FF);
    int x, y;
    K(fle_imp_SelectObject, dc, dib);
    K(fle_imp_SelectObject, dc, pen);
    FILLALL(dib, 6, 6, 0);
    K(fle_imp_MoveToEx, dc, 0, 0, 0); K(fle_imp_LineTo, dc, 3, 3);
    K(fle_imp_MoveToEx, dc, 5, 5, 0); K(fle_imp_LineTo, dc, 2, 2);
    K(fle_imp_MoveToEx, dc, 1, 1, 0); K(fle_imp_LineTo, dc, 1, 4);
    for (y = 0; y < 6; y++) { for (x = 0; x < 6; x++) putchar(PX(dib, 6, x, y) ? 'R' : '.'); putchar('\n'); }
"#,
    );
    // rows y = 0..5: (0,0); (1,1) from the first and the third line; (2,2) and (1,2); (1,3), (3,3), (4,4)... the second line writes (5,5), (4,4), (3,3)
    assert_eq!(l, ["R.....", ".R....", ".RR...", ".R.R..", "....R.", ".....R"]);
}

#[test]
fn rectangle_draws_the_outline_with_the_pen_and_fills_the_inside_with_the_brush_without_the_bottom_and_right_edge() {
    // Learn Rectangle: "The rectangle is outlined by using the current pen and filled by using the current brush"; "The rectangle that is drawn excludes the bottom and right edges"; "If a PS_NULL pen is used, the
    // dimensions of the rectangle are 1 pixel less in height and 1 pixel less in width". Rectangle(1,1,5,4) with a red 1-pixel pen and a blue brush: x in [1,5), y in [1,4): the outline is the border of that set, the
    // inside [2,4) x [2,3) is blue (a 1-pixel pen takes a 1-pixel border: a reading of "excludes the bottom and right edges"). With the PS_NULL pen (style 5: PS_NULL, the page's list) the set is [1,4) x [1,3), all
    // brush. A rectangle with right <= left draws nothing (S-rectangle-empty, choice) and returns 1 here.
    let l = lines(
        "fr11-rectangle",
        r#"
    uint64_t dib = MKDIB(8, 8, 32), dc = MKDC(), pen = K(fle_imp_CreatePen, 0, 1, 0x000000FF), nullpen = K(fle_imp_CreatePen, 5, 0, 0), br = SOLID(0x00FF0000);
    int x, y, n;
    K(fle_imp_SelectObject, dc, dib);
    K(fle_imp_SelectObject, dc, pen);
    K(fle_imp_SelectObject, dc, br);
    FILLALL(dib, 8, 8, 0);
    printf("ret %llu\n", U(KN(fle_imp_Rectangle, 5, dc, 1, 1, 5, 4)));
    for (y = 0; y < 8; y++) { for (x = 0; x < 8; x++) { uint32_t v = PX(dib, 8, x, y); putchar(v == 0 ? '.' : v == 0xFF0000 ? 'R' : v == 0x0000FF ? 'B' : '?'); } putchar('\n'); }
    FILLALL(dib, 8, 8, 0);
    K(fle_imp_SelectObject, dc, nullpen);
    printf("nullpen ret %llu\n", U(KN(fle_imp_Rectangle, 5, dc, 1, 1, 5, 4)));
    for (y = 0; y < 8; y++) { for (x = 0; x < 8; x++) { uint32_t v = PX(dib, 8, x, y); putchar(v == 0 ? '.' : v == 0xFF0000 ? 'R' : v == 0x0000FF ? 'B' : '?'); } putchar('\n'); }
    FILLALL(dib, 8, 8, 0);
    printf("empty ret %llu\n", U(KN(fle_imp_Rectangle, 5, dc, 3, 3, 3, 6)));
    n = 0; for (y = 0; y < 8; y++) for (x = 0; x < 8; x++) if (PX(dib, 8, x, y)) n++;
    printf("empty pixels %d\n", n);
"#,
    );
    // COLORREF 0x000000FF is red 0xFF -> PX 0xFF0000 'R'; the brush COLORREF 0x00FF0000 is blue 0xFF -> PX 0x0000FF 'B'. So the pen is 'R' and the brush 'B'.
    assert_eq!(
        l,
        [
            "ret 1",
            "........",
            ".RRRR...",
            ".RBBR...",
            ".RRRR...",
            "........",
            "........",
            "........",
            "........",
            "nullpen ret 1",
            "........",
            ".BBB....",
            ".BBB....",
            "........",
            "........",
            "........",
            "........",
            "........",
            "empty ret 1",
            "empty pixels 0",
        ]
    );
}

#[test]
fn fillrect_fills_the_half_open_rectangle_with_a_brush_and_ignores_the_rop2_and_framerect_draws_a_one_unit_border() {
    // Learn FillRect: "This function includes the left and top borders, but excludes the right and bottom borders" (the system colour + 1 form of hbr, whose colour value is a choice of decisions 0108 and 0123, is pinned by
    // gdi_bulk.rs). Learn FrameRect: "draws a border around the specified rectangle by using the specified brush. The width and height of the border are always one logical unit";
    // "If the bottom member of the RECT structure is less than the top member, or if the right member is less than the left member, the function does not draw the rectangle". FillRect is PATCOPY and ignores the DC's
    // ROP2 (S-fillrect-copy, choice): the DC is put in R2_BLACK first and the colour still arrives.
    let l = lines(
        "fr11-fillrect",
        r#"
    uint64_t dib = MKDIB(6, 6, 32), dc = MKDC(), br = SOLID(0x000000FF);
    int x, y;
    K(fle_imp_SelectObject, dc, dib);
    K(fle_imp_SetROP2, dc, 1);
    FILLALL(dib, 6, 6, 0);
    printf("fill ret %llu\n", U(K(fle_imp_FillRect, dc, RCT(1, 1, 4, 3), br)));
    for (y = 0; y < 6; y++) { for (x = 0; x < 6; x++) putchar(PX(dib, 6, x, y) == 0xFF0000 ? 'R' : PX(dib, 6, x, y) == 0 ? '.' : '?'); putchar('\n'); }
    FILLALL(dib, 6, 6, 0);
    printf("frame ret %llu\n", U(K(fle_imp_FrameRect, dc, RCT(1, 1, 5, 4), br)));
    for (y = 0; y < 6; y++) { for (x = 0; x < 6; x++) putchar(PX(dib, 6, x, y) == 0xFF0000 ? 'R' : PX(dib, 6, x, y) == 0 ? '.' : '?'); putchar('\n'); }
    FILLALL(dib, 6, 6, 0);
    printf("inverted frame ret %llu\n", U(K(fle_imp_FrameRect, dc, RCT(4, 4, 2, 2), br)));
    { int n = 0; for (y = 0; y < 6; y++) for (x = 0; x < 6; x++) if (PX(dib, 6, x, y)) n++; printf("inverted pixels %d\n", n); }
"#,
    );
    assert_eq!(
        l,
        [
            "fill ret 1",
            "......",
            ".RRR..",
            ".RRR..",
            "......",
            "......",
            "......",
            "frame ret 1",
            "......",
            ".RRRR.",
            ".R..R.",
            ".RRRR.",
            "......",
            "......",
            "inverted frame ret 1",
            "inverted pixels 0",
        ]
    );
}

#[test]
fn set_pixel_get_pixel_and_the_clipping_region_of_the_dib() {
    // Learn SetPixel: "the return value is the RGB value that the function sets the pixel to ... If the function fails, the return value is -1"; "The function fails if the pixel coordinates lie outside of the current
    // clipping region"; ERROR_INVALID_PARAMETER is the code the page lists. SetPixelV: the return is nonzero on success, zero on failure. GetPixel: "If the pixel is outside of the current clipping region, the return value
    // is CLR_INVALID (0xFFFFFFFF defined in Wingdi.h)". The pixel is stored as the COLORREF 0x00BBGGRR the guest passed (a 24-bit colour is exactly representable on a 32-bit surface: a reading).
    let l = lines(
        "fr11-pixel",
        r#"
    uint64_t dib = MKDIB(4, 4, 32), dc = MKDC();
    K(fle_imp_SelectObject, dc, dib);
    FILLALL(dib, 4, 4, 0);
    printf("set %08llx\n", U((uint32_t)K(fle_imp_SetPixel, dc, 1, 2, 0x00332211)));
    printf("get %08llx px %06x\n", U((uint32_t)K(fle_imp_GetPixel, dc, 1, 2)), PX(dib, 4, 1, 2));
    printf("setv %llu px %06x\n", U(K(fle_imp_SetPixelV, dc, 3, 3, 0x00665544)), PX(dib, 4, 3, 3));
    SETLE(0);
    printf("set outside %08llx le %u\n", U((uint32_t)K(fle_imp_SetPixel, dc, 4, 0, 0x00FFFFFF)), LE());
    printf("setv outside %llu\n", U(K(fle_imp_SetPixelV, dc, 0, 4, 0x00FFFFFF)));
    printf("get outside %08llx %08llx\n", U((uint32_t)K(fle_imp_GetPixel, dc, 4, 0)), U((uint32_t)K(fle_imp_GetPixel, dc, 0, NEG(-1))));
    printf("unused byte %llx\n", U(PXRAW(dib, 4, 1, 2) >> 24));
"#,
    );
    // COLORREF 0x00332211: R 0x11, G 0x22, B 0x33 -> PX 0x112233; 0x00665544: R 0x44 G 0x55 B 0x66 -> 0x445566
    assert_eq!(
        l,
        [
            "set 00332211",
            "get 00332211 px 112233",
            "setv 1 px 445566",
            "set outside ffffffff le 87",
            "setv outside 0",
            "get outside ffffffff ffffffff",
            "unused byte 0",
        ]
    );
}

#[test]
fn stretch_blt_enlarges_by_the_source_pixel_of_each_destination_pixel_and_mirrors_when_the_signs_differ() {
    // Learn StretchBlt: "stretches or compresses the source bitmap in memory and then copies the result to the destination rectangle"; "StretchBlt creates a mirror image of a bitmap if the signs of the nWidthSrc and
    // nWidthDest parameters or if the nHeightSrc and nHeightDest parameters differ. If nWidthSrc and nWidthDest have different signs, the function creates a mirror image of the bitmap along the x-axis".
    // A 2 x 1 source (pixels A = 0x00000011 -> PX 0x110000, B = 0x00000022 -> PX 0x220000) stretched to 4 x 1: the pixel i of the destination takes the source column floor(i * 2 / 4) (S-stretch-groups, choice: the page
    // names no source pixel): A A B B. Mirrored: the destination x is 4 and the width -4 (a negative width extends to the left: the rectangle is [0, 4)), the result is B B A A.
    let l = lines(
        "fr11-stretch",
        r#"
    uint64_t s = MKDIB(2, 1, 32), d = MKDIB(4, 1, 32), sdc = MKDC(), ddc = MKDC();
    int x;
    K(fle_imp_SelectObject, sdc, s);
    K(fle_imp_SelectObject, ddc, d);
    { uint8_t *p = BITS(s); p[0] = 0x00; p[1] = 0x00; p[2] = 0x11; p[4] = 0x00; p[5] = 0x00; p[6] = 0x22; }
    FILLALL(d, 4, 1, 0);
    printf("enlarge ret %llu\n", U(KN(fle_imp_StretchBlt, 11, ddc, 0, 0, 4, 1, sdc, 0, 0, 2, 1, 0x00CC0020)));
    for (x = 0; x < 4; x++) printf("%06x ", PX(d, 4, x, 0));
    printf("\n");
    FILLALL(d, 4, 1, 0);
    printf("mirror ret %llu\n", U(KN(fle_imp_StretchBlt, 11, ddc, 4, 0, NEG(-4), 1, sdc, 0, 0, 2, 1, 0x00CC0020)));
    for (x = 0; x < 4; x++) printf("%06x ", PX(d, 4, x, 0));
    printf("\n");
"#,
    );
    assert_eq!(l[0], "enlarge ret 1");
    assert_eq!(l[1].trim(), "110000 110000 220000 220000");
    assert_eq!(l[2], "mirror ret 1");
    assert_eq!(l[3].trim(), "220000 220000 110000 110000");
}

#[test]
fn stretch_blt_shrink_modes_and_the_stretch_mode_of_the_dc() {
    // Learn SetStretchBltMode: BLACKONWHITE (the initial mode: Display Device Context Defaults "Stretch mode BLACKONWHITE"), WHITEONBLACK, COLORONCOLOR, HALFTONE; STRETCH_ANDSCANS "Same as BLACKONWHITE", STRETCH_ORSCANS
    // "Same as WHITEONBLACK", STRETCH_DELETESCANS "Same as COLORONCOLOR"; "the return value is the previous stretching mode". For colours the page says BLACKONWHITE "Performs a Boolean AND operation using the color values for the eliminated and existing pixels", WHITEONBLACK "Performs a Boolean OR operation
    // using the color values for the eliminated and existing pixels", COLORONCOLOR "Deletes the pixels. This mode deletes all eliminated lines of pixels without trying to preserve their information", HALFTONE "Maps pixels
    // from the source rectangle into blocks of pixels in the destination rectangle. The average color over the destination block of pixels approximates the color of the source pixels" (the arithmetic mean per channel is the
    // choice S-stretch-halftone). Two source pixels 0x00F0F0F0 and 0x000F0FF0 (PX values) shrunk to one destination pixel: AND = 0x000000 .. computed below by
    // channel; OR; COLORONCOLOR keeps one source pixel (S-stretch-groups: the centre pixel of the group: the second of two, floor(1 * 2 / 2)? for a group of two the pixel taken is the one under the centre of the group);
    // HALFTONE = the mean per channel (0xF0 + 0x0F) / 2 = 0x77 etc.
    let l = lines(
        "fr11-shrink",
        r#"
    uint64_t s = MKDIB(2, 1, 32), d = MKDIB(1, 1, 32), sdc = MKDC(), ddc = MKDC();
    unsigned mode;
    K(fle_imp_SelectObject, sdc, s);
    K(fle_imp_SelectObject, ddc, d);
    { uint8_t *p = BITS(s); p[0] = 0xF0; p[1] = 0xF0; p[2] = 0xF0; p[4] = 0x0F; p[5] = 0xFF; p[6] = 0x0F; }
    printf("initial mode %llu\n", U(K(fle_imp_SetStretchBltMode, ddc, 2)));
    for (mode = 1; mode <= 4; mode++) {
        uint64_t prev = K(fle_imp_SetStretchBltMode, ddc, mode);
        FILLALL(d, 1, 1, 0);
        KN(fle_imp_StretchBlt, 11, ddc, 0, 0, 1, 1, sdc, 0, 0, 2, 1, 0x00CC0020);
        printf("mode %u prev %llu px %06x\n", mode, U(prev), PX(d, 1, 0, 0));
    }
"#,
    );
    // pixel 0 = PX 0xF0F0F0 (B F0, G F0, R F0), pixel 1 = bytes B 0x0F, G 0xFF, R 0x0F -> PX 0x0FFF0F
    assert_eq!(l[0], "initial mode 1", "BLACKONWHITE (1) is the mode of a new DC");
    assert_eq!(l[1], format!("mode 1 prev 2 px {:06x}", 0xF0F0F0u32 & 0x0FFF0Fu32), "BLACKONWHITE ANDs the merged pixels");
    assert_eq!(l[2], format!("mode 2 prev 1 px {:06x}", 0xF0F0F0u32 | 0x0FFF0Fu32), "WHITEONBLACK ORs the merged pixels");
    assert_eq!(l[3], "mode 3 prev 2 px 0fff0f", "COLORONCOLOR deletes the pixels that are not used: the one under the centre of the group stays");
    assert_eq!(l[4], format!("mode 4 prev 3 px {:02x}{:02x}{:02x}", (0xF0 + 0x0F) / 2, (0xF0 + 0xFF) / 2, (0xF0 + 0x0F) / 2), "HALFTONE: the mean of each channel");
}

#[test]
fn stretch_dibits_copies_the_scan_lines_of_a_dib_to_the_target_and_returns_the_number_of_scan_lines() {
    // Learn StretchDIBits: "copies the color data for a rectangle of pixels in a DIB ... to the specified destination rectangle"; the return value is "the number of scan lines copied. Note that this value can be negative
    // for mirrored content" and "If the function fails, or no scan lines are copied, the return value is 0". Learn Device-Independent Bitmaps: "A bottom-up DIB, in which the origin lies at the lower-left corner", "If the height
    // of a DIB, as indicated by the Height member of the bitmap information header structure, is a positive value, it is a bottom-up DIB; if the height is a negative value, it is a top-down DIB". The source is a 2 x 2 32-bit BI_RGB DIB with a POSITIVE height (bottom-up): its first memory row is the bottom image row. Bytes (B G R X):
    // memory row 0 = pixels P0 = (0x01,0x02,0x03) and P1 = (0x04,0x05,0x06), memory row 1 = P2 = (0x07,0x08,0x09), P3 = (0x0A,0x0B,0x0C). In image terms the top row is P2 P3 and the bottom row is P0 P1.
    // Copied 1:1 to (0,0) of a top-down target: target row 0 = P2 P3, row 1 = P0 P1. The return value is 2 (two scan lines): S-stretchdibits-return (choice) says DestHeight.
    let l = lines(
        "fr11-sdib",
        r#"
    uint64_t d = MKDIB(2, 2, 32), ddc = MKDC();
    uint8_t hdr[40 + 12], img[16] = { 1,2,3,0, 4,5,6,0, 7,8,9,0, 10,11,12,0 };
    uint64_t r;
    int x, y;
    K(fle_imp_SelectObject, ddc, d);
    FILLALL(d, 2, 2, 0);
    memset(hdr, 0, sizeof hdr);
    { uint32_t v = 40; memcpy(hdr, &v, 4); v = 2; memcpy(hdr + 4, &v, 4); memcpy(hdr + 8, &v, 4); { uint16_t one = 1, b = 32; memcpy(hdr + 12, &one, 2); memcpy(hdr + 14, &b, 2); } }
    r = KN(fle_imp_StretchDIBits, 13, ddc, 0, 0, 2, 2, 0, 0, 2, 2, A(img), A(hdr), 0, 0x00CC0020);
    printf("ret %lld\n", (long long)(int32_t)r);
    for (y = 0; y < 2; y++) { for (x = 0; x < 2; x++) printf("%06x ", PX(d, 2, x, y)); printf("\n"); }
"#,
    );
    // PX is 0xRRGGBB: P2 (B 7, G 8, R 9) -> 090807, P3 -> 0c0b0a, P0 (B 1, G 2, R 3) -> 030201, P1 -> 060504
    assert_eq!(l[0], "ret 2");
    assert_eq!(l[1].trim(), "090807 0c0b0a");
    assert_eq!(l[2].trim(), "030201 060504");
}

#[test]
fn get_dibits_and_set_dibits_and_the_bitmap_bits_roundtrip_a_device_dependent_bitmap() {
    // Learn SetDIBits: "sets the pixels in a compatible bitmap (DDB) using the color data found in the specified DIB"; the return value is "the number of scan lines copied"; hbm "must not be selected into a device context";
    // "The origin for bottom-up DIBs is the lower-left corner of the bitmap; the origin for top-down DIBs is the upper-left corner". Learn GetDIBits: "retrieves the bits of the specified compatible bitmap and copies them into
    // a buffer as a DIB"; "If the lpvBits parameter is non-NULL and the function succeeds, the return value is the number of scan lines copied from the bitmap"; "If the lpvBits parameter is NULL and GetDIBits successfully fills
    // the BITMAPINFO structure, the return value is nonzero" (this test does not call GetDIBits with a NULL lpvBits: only the non-NULL form is pinned here). The DDB of CreateCompatibleBitmap(screen) is a colour bitmap whose bits are 32-bit BGRX (S-ddb-format, choice). A top-down 2 x 2 24-bit... the
    // source here is a 32-bit top-down DIB so that no row padding enters: 4 pixels (B G R 0): (1,2,3) (4,5,6) / (7,8,9) (10,11,12). The same four pixels must come back through a GetDIBits of 32 bits top-down; a SELECTED
    // bitmap fails with 0 and ERROR_INVALID_PARAMETER 87 (S-selected-ddb, choice: the page says "must not be selected").
    let l = lines(
        "fr11-dibits",
        r#"
    uint64_t scr = K(fle_imp_GetDC, 0), bmp = K(fle_imp_CreateCompatibleBitmap, scr, 2, 2), dc = MKDC();
    uint8_t hdr[40 + 12], in[16] = { 1,2,3,0, 4,5,6,0, 7,8,9,0, 10,11,12,0 }, out[16];
    uint64_t r;
    unsigned i;
    memset(hdr, 0, sizeof hdr); memset(out, 0xEE, sizeof out);
    { uint32_t v = 40; memcpy(hdr, &v, 4); v = 2; memcpy(hdr + 4, &v, 4); v = (uint32_t)-2; memcpy(hdr + 8, &v, 4); { uint16_t one = 1, b = 32; memcpy(hdr + 12, &one, 2); memcpy(hdr + 14, &b, 2); } }
    r = KN(fle_imp_SetDIBits, 7, scr, bmp, 0, 2, A(in), A(hdr), 0);
    printf("setdibits ret %llu\n", U(r));
    r = KN(fle_imp_GetDIBits, 7, scr, bmp, 0, 2, A(out), A(hdr), 0);
    printf("getdibits ret %llu\n", U(r));
    for (i = 0; i < 16; i++) printf("%02x", out[i]);
    printf("\n");
    K(fle_imp_SelectObject, dc, bmp);
    SETLE(0);
    r = KN(fle_imp_GetDIBits, 7, scr, bmp, 0, 2, A(out), A(hdr), 0);
    printf("selected ret %llu le %u\n", U(r), LE());
"#,
    );
    assert_eq!(l[0], "setdibits ret 2");
    assert_eq!(l[1], "getdibits ret 2");
    assert_eq!(l[2], "01020300040506000708090 00a0b0c00".replace(' ', ""));
    assert_eq!(l[3], "selected ret 0 le 87");
}

#[test]
fn maskblt_applies_the_foreground_code_where_the_mask_is_one_and_the_background_code_where_it_is_zero() {
    // Learn MaskBlt: "A value of 1 in the mask specified by hbmMask indicates that the foreground raster operation code specified by dwRop should be applied at that location. A value of 0 in the mask indicates that the
    // background raster operation code ... should be applied"; dwRop: "The background raster operation code is stored in the high-order byte of the high-order word of this value; the foreground raster operation code is
    // stored in the low-order byte of the high-order word of this value; the low-order word of this value is ignored" (MAKEROP4(fore, back) = ((back << 8) & 0xFF000000) | (fore)); "If the mask bitmap is not a monochrome
    // bitmap, an error occurs"; "The MaskBlt function uses device-dependent bitmaps"; "If no mask bitmap is supplied, this function behaves exactly like BitBlt, using the foreground raster operation code". Foreground SRCCOPY (index
    // 0xCC), background BLACKNESS (index 0x00): rop = 0x00CC0000 | (0x00 << 24) -> 0x00CC0000 as the high word is (back << 8 | fore) = 0x00CC. A 1 x 2 destination, source colour 0x00F0CC11 (PX 0x11CCF0) in both rows; the
    // mask is a 1 x 2 monochrome DDB with bit 1 in row 0 and bit 0 in row 1: row 0 = SRCCOPY = 0x11CCF0, row 1 = BLACKNESS = 0. The mask bits are set with SetBitmapBits: one 16-bit word per row, the leftmost pixel in the
    // most significant bit (Learn CreateBitmap: each scan line is word aligned; Bitmap Storage: "The index of the RGBQUAD for the color of the left-most pixel is the first n bits in the color-index array": said for a DIB; applied to the DDB mask as a reading).
    let l = lines(
        "fr11-maskblt",
        r#"
    uint64_t s = MKDIB(1, 2, 32), d = MKDIB(1, 2, 32), sdc = MKDC(), ddc = MKDC(), mask = K(fle_imp_CreateBitmap, 1, 2, 1, 1);
    uint8_t bits[4] = { 0x80, 0x00, 0x00, 0x00 };
    uint64_t r;
    K(fle_imp_SelectObject, sdc, s);
    K(fle_imp_SelectObject, ddc, d);
    printf("setbits ret %llu\n", U(K(fle_imp_SetBitmapBits, mask, 4, A(bits))));
    { uint8_t *p = BITS(s); p[0] = 0xF0; p[1] = 0xCC; p[2] = 0x11; p[4] = 0xF0; p[5] = 0xCC; p[6] = 0x11; }
    FILLALL(d, 1, 2, 0x55);
    r = KN(fle_imp_MaskBlt, 12, ddc, 0, 0, 1, 2, sdc, 0, 0, mask, 0, 0, 0x00CC0000);
    printf("ret %llu row0 %06x row1 %06x\n", U(r), PX(d, 1, 0, 0), PX(d, 1, 0, 1));
"#,
    );
    assert_eq!(l[0], "setbits ret 4");
    assert_eq!(l[1], "ret 1 row0 11ccf0 row1 000000");
}

#[test]
fn the_clipping_region_calls_return_the_region_complexity_and_clip_the_drawing() {
    // Learn SelectClipRgn: the return value "specifies the region's complexity": NULLREGION (1) "Region is empty", SIMPLEREGION (2) "Region is a single rectangle", COMPLEXREGION (3) "more than one rectangle", ERROR (0);
    // "To remove a device-context's clipping region, specify a NULL region handle". IntersectClipRect: "The lower and right-most edges of the given rectangle are excluded from the clipping region"; ExcludeClipRect:
    // "The lower and right edges of the specified rectangle are not excluded from the clipping region" (the page's sentence is read as: the excluded set is the half-open rectangle, S-exclude-edges, choice).
    // GetClipRgn: "If the function succeeds and there is no clipping region for the given device context, the return value is zero. If ... there is a clipping region ... 1". The numbers of the complexity are
    // the SDK's (ERROR 0, NULLREGION 1, SIMPLEREGION 2, COMPLEXREGION 3: UNOFFICIAL). Drawing is clipped to the intersection: a FillRect of the whole 4 x 4 DIB under IntersectClipRect(1,1,3,3) paints [1,3) x [1,3).
    let l = lines(
        "fr11-clip",
        r#"
    uint64_t dib = MKDIB(4, 4, 32), dc = MKDC(), br = SOLID(0x000000FF), rgn = K(fle_imp_CreateRectRgn, 0, 0, 2, 2);
    int x, y;
    K(fle_imp_SelectObject, dc, dib);
    FILLALL(dib, 4, 4, 0);
    printf("getcliprgn none %llu\n", U(K(fle_imp_GetClipRgn, dc, rgn)));
    printf("intersect ret %llu\n", U(KN(fle_imp_IntersectClipRect, 5, dc, 1, 1, 3, 3)));
    printf("getcliprgn set %llu\n", U(K(fle_imp_GetClipRgn, dc, rgn)));
    K(fle_imp_FillRect, dc, RCT(0, 0, 4, 4), br);
    for (y = 0; y < 4; y++) { for (x = 0; x < 4; x++) putchar(PX(dib, 4, x, y) ? 'R' : '.'); putchar('\n'); }
    printf("exclude ret %llu\n", U(KN(fle_imp_ExcludeClipRect, 5, dc, 1, 1, 2, 3)));
    FILLALL(dib, 4, 4, 0);
    K(fle_imp_FillRect, dc, RCT(0, 0, 4, 4), br);
    for (y = 0; y < 4; y++) { for (x = 0; x < 4; x++) putchar(PX(dib, 4, x, y) ? 'R' : '.'); putchar('\n'); }
    printf("remove ret %llu getcliprgn %llu\n", U(K(fle_imp_SelectClipRgn, dc, 0)), U(K(fle_imp_GetClipRgn, dc, rgn)));
    printf("select empty %llu\n", U(K(fle_imp_SelectClipRgn, dc, K(fle_imp_CreateRectRgn, 1, 1, 1, 1))));
    printf("select rect %llu\n", U(K(fle_imp_SelectClipRgn, dc, rgn)));
"#,
    );
    assert_eq!(l[0], "getcliprgn none 0");
    assert_eq!(l[1], "intersect ret 2");
    assert_eq!(l[2], "getcliprgn set 1");
    assert_eq!(&l[3..7], ["....", ".RR.", ".RR.", "...."]);
    // [1,3) x [1,3) minus the half-open [1,2) x [1,3) leaves the column x = 2 of rows 1 and 2: a single rectangle
    assert_eq!(l[7], "exclude ret 2");
    assert_eq!(&l[8..12], ["....", "..R.", "..R.", "...."]);
    assert_eq!(l[12], "remove ret 2 getcliprgn 0");
    assert_eq!(l[13], "select empty 1");
    assert_eq!(l[14], "select rect 2");
}

#[test]
fn the_stock_objects_are_the_colours_of_the_pages_or_the_choice_of_the_decision() {
    // Learn GetStockObject: BLACK_BRUSH "Black brush", WHITE_BRUSH "White brush", WHITE_PEN "White pen", BLACK_PEN "Black pen", NULL_BRUSH "Null brush (equivalent to HOLLOW_BRUSH)", NULL_PEN "Null pen. The null pen
    // draws nothing"; GRAY_BRUSH "Gray brush", LTGRAY_BRUSH "Light gray brush", DKGRAY_BRUSH "Dark gray brush" (the page gives no grey value: S-stock-brush-colour, choice: LTGRAY 0xC0C0C0, GRAY 0x808080, DKGRAY 0x404040).
    // The indices are the SDK's (WHITE_BRUSH 0, LTGRAY_BRUSH 1, GRAY_BRUSH 2, DKGRAY_BRUSH 3, BLACK_BRUSH 4, NULL_BRUSH 5, WHITE_PEN 6, BLACK_PEN 7, NULL_PEN 8: UNOFFICIAL). Selecting a stock brush and calling
    // PatBlt(PATCOPY) writes its colour; the NULL brush (S-hollow-fill, choice) writes nothing and the call still returns 1; WHITE_PEN draws white with LineTo.
    let l = lines(
        "fr11-stock",
        r#"
    uint64_t dib = MKDIB(4, 4, 32), dc = MKDC();
    int i;
    K(fle_imp_SelectObject, dc, dib);
    for (i = 0; i <= 5; i++) {
        uint64_t ret;
        FILLALL(dib, 4, 4, 0x11);
        K(fle_imp_SelectObject, dc, K(fle_imp_GetStockObject, (uint64_t)i));
        ret = KN(fle_imp_PatBlt, 6, dc, 0, 0, 1, 1, 0x00F00021);
        printf("brush %d ret %llu px %06x\n", i, U(ret), PX(dib, 4, 0, 0));
    }
    FILLALL(dib, 4, 4, 0);
    K(fle_imp_SelectObject, dc, K(fle_imp_GetStockObject, 6));
    K(fle_imp_MoveToEx, dc, 0, 0, 0);
    K(fle_imp_LineTo, dc, 2, 0);
    printf("white pen px0 %06x\n", PX(dib, 4, 0, 0));
    FILLALL(dib, 4, 4, 0x11);
    K(fle_imp_SelectObject, dc, K(fle_imp_GetStockObject, 8));
    K(fle_imp_MoveToEx, dc, 0, 0, 0);
    K(fle_imp_LineTo, dc, 2, 0);
    printf("null pen px0 %06x\n", PX(dib, 4, 0, 0));
    printf("same handle %d\n", K(fle_imp_GetStockObject, 4) == K(fle_imp_GetStockObject, 4));
"#,
    );
    assert_eq!(l[0], "brush 0 ret 1 px ffffff");
    assert_eq!(l[1], "brush 1 ret 1 px c0c0c0");
    assert_eq!(l[2], "brush 2 ret 1 px 808080");
    assert_eq!(l[3], "brush 3 ret 1 px 404040");
    assert_eq!(l[4], "brush 4 ret 1 px 000000");
    assert_eq!(l[5], "brush 5 ret 1 px 111111", "NULL_BRUSH (HOLLOW) paints nothing: the destination keeps 0x11");
    assert_eq!(l[6], "white pen px0 ffffff");
    assert_eq!(l[7], "null pen px0 111111", "the null pen draws nothing");
    assert_eq!(l[8], "same handle 1");
}

// ---------------------------------------------------------------------------------------------
// 4. The environment answers
// ---------------------------------------------------------------------------------------------

#[test]
fn compare_string_w_orders_a_prefix_before_the_longer_string_and_skips_equal_units_of_a_common_prefix() {
    // Learn CompareStringEx (the page CompareStringW refers to for the return values): "CSTR_LESS_THAN. The string indicated by lpString1 is less in lexical value than the string indicated by lpString2.",
    // "CSTR_EQUAL ... equivalent in lexical value", "CSTR_GREATER_THAN ... greater". The page names the three results and gives no numbers; the values 1 (CSTR_LESS_THAN), 2 (CSTR_EQUAL) and 3 (CSTR_GREATER_THAN) are the SDK's winnls.h numbers
    // (UNOFFICIAL; the table of decision 0126 uses them). A string that is a prefix of the other: the shorter is the smaller (S-cmp-prefix, choice: the page gives no rule). A common prefix that holds a space (S-cmp-equal-prefix, choice) or a hyphen
    // (S-cmp-equal-hyphen, choice) does not decide the order: the first different pair does. Flags 0 and locale 0x409 (en-US) as the guest calls it.
    let l = lines(
        "fr11-compare",
        r#"
#define CMP(a, b) (unsigned)(K(fle_imp_CompareStringW, 0x409, 0, S(a), NEG(-1)) , R[0])
    static const struct { const char *a, *b; } t[] = {
        { "ab", "abc" }, { "abc", "ab" }, { ".FLMS", ".FLM" }, { ".FLM", ".FLMS" }, { "_", "_a" }, { "", "a" },
        { "Test message processing", "Test program expiration" }, { "Test program expiration", "Test message processing" },
        { "a b", "a c" }, { "a c", "a b" }, { "a-b", "a-c" }, { "a-c", "a-b" },
        { "c:\\users\\user\\documents\\image-line\\dow", "c:\\users\\user\\documents\\image-line\\fl s" },
        { "abc", "abc" },
    };
    unsigned i;
    for (i = 0; i < sizeof t / sizeof t[0]; i++) {
        uint64_t r = KN(fle_imp_CompareStringW, 6, 0x409, 0, S(t[i].a), NEG(-1), S(t[i].b), NEG(-1));
        printf("%u %llu\n", i, U(r));
    }
"#,
    );
    let want = [1, 3, 3, 1, 1, 1, 1, 3, 1, 3, 1, 3, 1, 2];
    assert_eq!(l.len(), want.len());
    for (k, w) in want.iter().enumerate() {
        assert_eq!(l[k], format!("{k} {w}"), "pair {k}");
    }
}

#[test]
fn sh_get_folder_path_w_gives_the_path_of_each_csidl_and_fails_for_the_others() {
    // Learn CSIDL: CSIDL_WINDOWS "The Windows directory or SYSROOT ... A typical path is C:\Windows." (a REAL value of the page); CSIDL_PROGRAM_FILES "A typical path is C:\Program Files." (page); CSIDL_PROFILE "The user's
    // profile folder. A typical path is C:\Users\username"; the user name of this environment is "user" (decision 0107, a choice). CSIDL_LOCAL_APPDATA: the page gives the Windows XP path "C:\Documents and Settings\username\
    // Local Settings\Application Data" and the environment answers C:\Users\user\AppData\Local (S-localappdata-path, choice). CSIDL_COMMON_APPDATA 0x23: C:\ProgramData (S-commonappdata, choice). CSIDL_PERSONAL 0x05:
    // C:\Users\user\Documents (S-profile-personal, choice). CSIDL_PROGRAM_FILESX86 0x2a: C:\Program Files (S-programfiles, choice; a real machine says "(x86)"). CSIDL_PROGRAM_FILES_COMMON 0x2b and COMMONX86 0x2c:
    // C:\Program Files\Common Files (S-programfiles-common, choice; the page says "C:\Program Files\Common"). The csidl numbers are from shlobj.h (UNOFFICIAL: the page lists names). Learn SHGetFolderPathW: S_OK (0)
    // on success; "The returned path does not include a trailing backslash". An unlisted csidl (0x25 SYSTEM) stops the run by name.
    let l = lines(
        "fr11-csidl",
        r#"
    static const uint32_t ids[] = { 0x24, 0x1c, 0x23, 0x28, 0x05, 0x26, 0x2a, 0x2b, 0x2c, 0x14, 0x1a };
    unsigned i;
    for (i = 0; i < sizeof ids / sizeof ids[0]; i++) {
        static uint16_t buf[300];
        char out[300];
        memset(buf, 0, sizeof buf);
        { uint64_t r = KN(fle_imp_SHGetFolderPathW, 5, 0, ids[i], 0, 0, A(buf)); wtoa(A(buf), out, sizeof out); printf("%02x hr %llu %s\n", ids[i], U(r), out); }
    }
"#,
    );
    let want = [
        ("24", "C:\\Windows"),
        ("1c", "C:\\Users\\user\\AppData\\Local"),
        ("23", "C:\\ProgramData"),
        ("28", "C:\\Users\\user"),
        ("05", "C:\\Users\\user\\Documents"),
        ("26", "C:\\Program Files"),
        ("2a", "C:\\Program Files"),
        ("2b", "C:\\Program Files\\Common Files"),
        ("2c", "C:\\Program Files\\Common Files"),
        ("14", "C:\\Windows\\Fonts"),
        ("1a", "C:\\Users\\user\\AppData\\Roaming"),
    ];
    assert_eq!(l.len(), want.len());
    for (k, (id, path)) in want.iter().enumerate() {
        assert_eq!(l[k], format!("{id} hr 0 {path}"), "csidl 0x{id}");
    }
    aborts("fr11-csidl-unlisted", "static uint16_t buf[300]; KN(fle_imp_SHGetFolderPathW, 5, 0, 0x25, 0, 0, A(buf));", "SHGetFolderPathW", "csidl 0x25 is not answered");
}

#[test]
fn get_key_state_and_get_async_key_state_say_every_key_is_up_and_untoggled() {
    // Learn GetKeyState: "If the high-order bit is 1, the key is down; otherwise, it is up. If the low-order bit is 1, the key is toggled ... The key is off and untoggled if the low-order bit is 0." The environment has no input:
    // every key is up and untoggled, the result is 0 for any key (S-no-keys, choice). The same for GetAsyncKeyState. A letter by its ASCII value, VK_SHIFT 0x10, VK_CONTROL 0x11, VK_MENU 0x12, VK_CAPITAL 0x14 (a toggle key
    // that the page names: CAPS LOCK is untoggled), VK_LSHIFT 0xA0, a digit '0' 0x30. The numbers are the SDK's (UNOFFICIAL).
    let l = lines(
        "fr11-keys",
        r#"
    static const uint32_t keys[] = { 0x41, 0x30, 0x10, 0x11, 0x12, 0x14, 0xA0, 0x01, 0x7B, 0xFF };
    unsigned i;
    for (i = 0; i < sizeof keys / sizeof keys[0]; i++)
        printf("%02x %llx %llx\n", keys[i], U(K(fle_imp_GetKeyState, keys[i]) & 0xFFFF), U(K(fle_imp_GetAsyncKeyState, keys[i]) & 0xFFFF));
"#,
    );
    let keys = ["41", "30", "10", "11", "12", "14", "a0", "01", "7b", "ff"];
    assert_eq!(l.len(), keys.len());
    for (k, key) in keys.iter().enumerate() {
        assert_eq!(l[k], format!("{key} 0 0"), "virtual key 0x{key}");
    }
}

#[test]
fn read_file_of_zero_bytes_follows_the_two_rules_of_the_page_at_and_before_the_end_of_the_file() {
    // Learn ReadFile: "If a read operation on a file begins at or beyond the end of the file, then the read operation fails with the error ERROR_HANDLE_EOF. If a read operation on a file begins before the end of the file, but
    // the read operation extends past the end of the file, then the read operation succeeds, and the number of bytes read is the number of bytes that were read before the end of file was reached"; "ReadFile sets this value
    // [lpNumberOfBytesRead] to zero before doing any work or error checking"; ERROR_HANDLE_EOF is 38 (System Error Codes: "Reached the end of the file."). The page does not define nNumberOfBytesToRead = 0: the two rules are
    // applied to a zero-byte read as they stand (S-readfile-zero, choice): at or beyond the end it fails with 38, before the end it succeeds with 0 bytes read. File "C:\z.bin" holds the 4 bytes "abcd".
    let l = lines(
        "fr11-readzero",
        r#"
    uint64_t h = KN(fle_imp_CreateFileW, 7, S("C:\\z.bin"), 0xC0000000u, 7, 0, 2, 0x80, 0);
    uint32_t n = 99;
    static char buf[8];
    uint64_t ok;
    n = 99; ok = KN(fle_imp_WriteFile, 5, h, A("abcd"), 4, A(&n), 0); printf("write %llu %u\n", U(ok), n);
    K(fle_imp_SetFilePointer, h, 0, 0, 0);
    n = 99; SETLE(0); ok = KN(fle_imp_ReadFile, 5, h, A(buf), 0, A(&n), 0); printf("start ok %llu n %u le %u\n", U(ok), n, LE());
    K(fle_imp_SetFilePointer, h, 3, 0, 0);
    n = 99; SETLE(0); ok = KN(fle_imp_ReadFile, 5, h, A(buf), 0, A(&n), 0); printf("before end ok %llu n %u le %u\n", U(ok), n, LE());
    K(fle_imp_SetFilePointer, h, 4, 0, 0);
    n = 99; SETLE(0); ok = KN(fle_imp_ReadFile, 5, h, A(buf), 0, A(&n), 0); printf("at end ok %llu n %u le %u\n", U(ok), n, LE());
    K(fle_imp_SetFilePointer, h, 9, 0, 0);
    n = 99; SETLE(0); ok = KN(fle_imp_ReadFile, 5, h, A(buf), 0, A(&n), 0); printf("beyond end ok %llu n %u le %u\n", U(ok), n, LE());
    K(fle_imp_SetFilePointer, h, 4, 0, 0);
    n = 99; SETLE(0); ok = KN(fle_imp_ReadFile, 5, h, A(buf), 2, A(&n), 0); printf("one byte at end ok %llu n %u le %u\n", U(ok), n, LE());
"#,
    );
    assert_eq!(l[0], "write 1 4");
    assert_eq!(l[1], "start ok 1 n 0 le 0");
    assert_eq!(l[2], "before end ok 1 n 0 le 0");
    assert_eq!(l[3], "at end ok 0 n 0 le 38");
    assert_eq!(l[4], "beyond end ok 0 n 0 le 38");
    assert_eq!(l[5], "one byte at end ok 1 n 0 le 0"); // decision 0243: a read of more than 0 bytes at the end is TRUE with 0 bytes read (Learn "Testing for the End of a File"); the zero-byte requests above keep 38
}

#[test]
fn is_debugger_present_says_no_debugger_is_attached() {
    // Learn IsDebuggerPresent: "If the current process is running in the context of a debugger, the return value is nonzero. If the current process is not running in the context of a debugger, the return value is zero."
    // This virtual process has no debugger (S-no-debugger, choice: the definition of the environment): 0, also as a BOOL in the 32-bit eax of the guest.
    let l = lines("fr11-debugger", "printf(\"%llu %llu\\n\", U(K(fle_imp_IsDebuggerPresent, 0)), U(R[0] >> 32));");
    assert_eq!(l, ["0 0"]);
}

#[test]
fn monitor_from_window_point_and_rect_answer_the_one_monitor_by_the_flags_of_the_pages() {
    // Learn MonitorFromPoint: "If the point is contained by a display monitor, the return value is an HMONITOR handle to that display monitor. If the point is not contained by a display monitor, the return value depends on
    // the value of dwFlags": MONITOR_DEFAULTTONULL 0x00000000 "Returns NULL", MONITOR_DEFAULTTOPRIMARY 0x00000001 "Returns a handle to the primary display monitor", MONITOR_DEFAULTTONEAREST 0x00000002 "Returns a handle to
    // the display monitor that is nearest to the point" (the numbers are on this page). MonitorFromWindow: "If the window intersects one or more display monitor rectangles, the return value is an HMONITOR handle to the
    // display monitor that has the largest area of intersection with the window. If the window does not intersect a display monitor, the return value depends on the value of dwFlags." The environment has one monitor
    // (0, 0, 1920, 1080) (decision 0065), its handle is 0x7FEA0004 (decision 0065, a choice). A point is contained when left <= x < right and top <= y < bottom (S-monitor-point-halfopen, choice: the PtInRect rule); a rectangle
    // intersects when the area of the intersection is not zero (S-monitor-area, choice). A window handle that is not a live window answers "no intersection" by dwFlags (S-monitor-invalid-window, choice). A dwFlags other than
    // 0, 1, 2 stops (the pages list no other).
    let l = lines(
        "fr11-monitor",
        r#"
    const uint64_t MON = 0x7FEA0004;
    struct { int32_t x, y; } pts[] = { {0, 0}, {1919, 1079}, {1920, 500}, {500, 1080}, {-1, 0}, {0, -1} };
    struct { int32_t l, t, r, b; } rcs[] = { {100, 100, 200, 200}, {1919, 1079, 2000, 2000}, {1920, 0, 2000, 100}, {-50, -50, 0, 0}, {-50, -50, 1, 1}, {10, 10, 10, 50}, {2000, 2000, 2100, 2100} };
    unsigned i, f;
    for (i = 0; i < sizeof pts / sizeof pts[0]; i++) {
        uint64_t pt = (uint64_t)(uint32_t)pts[i].x | ((uint64_t)(uint32_t)pts[i].y << 32);
        printf("pt %d %d:", pts[i].x, pts[i].y);
        for (f = 0; f < 3; f++) printf(" %llx", U(K(fle_imp_MonitorFromPoint, pt, f)));
        printf("\n");
    }
    for (i = 0; i < sizeof rcs / sizeof rcs[0]; i++) {
        printf("rc %d:", (int)i);
        for (f = 0; f < 3; f++) printf(" %llx", U(K(fle_imp_MonitorFromRect, RCT(rcs[i].l, rcs[i].t, rcs[i].r, rcs[i].b), f)));
        printf("\n");
    }
    printf("badwin:");
    for (f = 0; f < 3; f++) printf(" %llx", U(K(fle_imp_MonitorFromWindow, 0x12345678, f)));
    printf("\n");
    printf("null window:");
    for (f = 0; f < 3; f++) printf(" %llx", U(K(fle_imp_MonitorFromWindow, 0, f)));
    printf("\n");
    printf("mon is 7fea0004 %d\n", MON == 0x7FEA0004);
"#,
    );
    let n = "0";
    let m = "7fea0004";
    // a point inside the monitor gives the handle for every dwFlags; a point outside gives NULL for DEFAULTTONULL (0) and the handle for the other two
    assert_eq!(l[0], format!("pt 0 0: {m} {m} {m}"));
    assert_eq!(l[1], format!("pt 1919 1079: {m} {m} {m}"));
    assert_eq!(l[2], format!("pt 1920 500: {n} {m} {m}"), "the right edge is outside");
    assert_eq!(l[3], format!("pt 500 1080: {n} {m} {m}"), "the bottom edge is outside");
    assert_eq!(l[4], format!("pt -1 0: {n} {m} {m}"));
    assert_eq!(l[5], format!("pt 0 -1: {n} {m} {m}"));
    assert_eq!(l[6], format!("rc 0: {m} {m} {m}"));
    assert_eq!(l[7], format!("rc 1: {m} {m} {m}"), "a rectangle that crosses the corner intersects");
    assert_eq!(l[8], format!("rc 2: {n} {m} {m}"), "a rectangle that begins at the right edge has no area in the monitor");
    assert_eq!(l[9], format!("rc 3: {n} {m} {m}"), "a rectangle that ends at the left top corner has no area in the monitor");
    assert_eq!(l[10], format!("rc 4: {m} {m} {m}"), "one pixel inside is an area");
    assert_eq!(l[11], format!("rc 5: {n} {m} {m}"), "an empty rectangle has no area (S-monitor-area)");
    assert_eq!(l[12], format!("rc 6: {n} {m} {m}"));
    assert_eq!(l[13], format!("badwin: {n} {m} {m}"));
    assert_eq!(l[14], format!("null window: {n} {m} {m}"));
    assert_eq!(l[15], "mon is 7fea0004 1");
    aborts("fr11-monitor-flags", "K(fle_imp_MonitorFromPoint, 0, 3);", "MonitorFromPoint", "dwFlags 0x3 is not MONITOR_DEFAULTTONULL");
}

#[test]
fn the_rect_calls_follow_the_pages_inflate_equal_pt_in_and_subtract() {
    // Learn InflateRect: "adds -dx units to the left end and dx to the right end of the rectangle and -dy units to the top and dy to the bottom. The dx and dy parameters are signed values; positive values increase the width
    // and height, and negative values decrease them" (the page's parameter text: "This parameter must be negative to decrease the width"); nonzero on success. EqualRect: nonzero when "the two rectangles are identical" ("compares
    // the coordinates of their upper-left and lower-right corners"), "does not treat empty rectangles as equal if their coordinates are different". PtInRect: "A point is within a rectangle if it lies on the left or top side
    // or is within all four sides. A point on the right or bottom side is considered outside the rectangle"; "The rectangle must be normalized ... If the rectangle is not normalized, a point is never considered inside of the
    // rectangle." The POINT is passed by value (8 bytes: x low, y high). SubtractRect: "only subtracts the rectangle specified by lprcSrc2 from the rectangle specified by lprcSrc1 when the rectangles intersect completely in
    // either the x- or y-direction. For example, if *lprcSrc1 has the coordinates (10,10,100,100) and *lprcSrc2 has the coordinates (50,50,150,150), the function sets the coordinates of the rectangle pointed to by
    // lprcDst to (10,10,100,100). If *lprcSrc1 has the coordinates (10,10,100,100) and *lprcSrc2 has the coordinates (50,10,150,150), however, ... (10,10,50,100)"; "If the resulting rectangle is empty, the return value is
    // zero. If the resulting rectangle is not empty, the return value is nonzero". The rule for the other cases (a strip in the middle, both edges, no intersection, empty operands) is the choice S-subtractrect-reading.
    let l = lines(
        "fr11-rects",
        r#"
    int32_t a[4], b[4], d[4];
    uint64_t r;
    unsigned i;
#define SET(v, l, t, r_, b_) do { v[0] = l; v[1] = t; v[2] = r_; v[3] = b_; } while (0)
#define PT(x, y) ((uint64_t)(uint32_t)(x) | ((uint64_t)(uint32_t)(y) << 32))
    SET(a, 10, 20, 30, 40);
    r = K(fle_imp_InflateRect, A(a), 5, 3, 0);
    printf("inflate ret %llu %d %d %d %d\n", U(r), a[0], a[1], a[2], a[3]);
    r = K(fle_imp_InflateRect, A(a), NEG(-5), NEG(-3), 0);
    printf("deflate ret %llu %d %d %d %d\n", U(r), a[0], a[1], a[2], a[3]);
    r = K(fle_imp_InflateRect, A(a), NEG(-20), 0, 0);
    printf("deflate past ret %llu %d %d %d %d\n", U(r), a[0], a[1], a[2], a[3]);
    SET(a, 1, 2, 3, 4); SET(b, 1, 2, 3, 4);
    printf("equal same %llu\n", U(K(fle_imp_EqualRect, A(a), A(b), 0, 0)));
    b[3] = 5;
    printf("equal differs %llu\n", U(K(fle_imp_EqualRect, A(a), A(b), 0, 0)));
    SET(a, 5, 5, 5, 5); SET(b, 6, 6, 6, 6);
    printf("equal empty different %llu\n", U(K(fle_imp_EqualRect, A(a), A(b), 0, 0)));
    SET(a, 10, 10, 20, 20);
    printf("ptinrect:");
    { static const int32_t p[][2] = { {10,10}, {19,19}, {20,15}, {15,20}, {20,20}, {9,15}, {15,9}, {15,15} };
      for (i = 0; i < 8; i++) printf(" %llu", U(K(fle_imp_PtInRect, A(a), PT(p[i][0], p[i][1])))); }
    printf("\n");
    SET(a, 20, 10, 10, 20);
    printf("ptinrect not normalized %llu %llu\n", U(K(fle_imp_PtInRect, A(a), PT(15, 15))), U(K(fle_imp_PtInRect, A(a), PT(10, 10))));
    SET(a, 10, 20, 20, 10);
    printf("ptinrect not normalized y %llu\n", U(K(fle_imp_PtInRect, A(a), PT(15, 15))));
    SET(a, 10, 10, 100, 100); SET(b, 50, 50, 150, 150); SET(d, 0, 0, 0, 0);
    r = K(fle_imp_SubtractRect, A(d), A(a), A(b));
    printf("subtract page1 ret %llu %d %d %d %d\n", U(r), d[0], d[1], d[2], d[3]);
    SET(b, 50, 10, 150, 150);
    r = K(fle_imp_SubtractRect, A(d), A(a), A(b));
    printf("subtract page2 ret %llu %d %d %d %d\n", U(r), d[0], d[1], d[2], d[3]);
    SET(b, 0, 10, 60, 100);
    r = K(fle_imp_SubtractRect, A(d), A(a), A(b));
    printf("subtract left strip ret %llu %d %d %d %d\n", U(r), d[0], d[1], d[2], d[3]);
    SET(b, 0, 0, 200, 200);
    r = K(fle_imp_SubtractRect, A(d), A(a), A(b));
    printf("subtract covers all ret %llu %d %d %d %d\n", U(r), d[0], d[1], d[2], d[3]);
    SET(b, 200, 200, 300, 300);
    r = K(fle_imp_SubtractRect, A(d), A(a), A(b));
    printf("subtract disjoint ret %llu %d %d %d %d\n", U(r), d[0], d[1], d[2], d[3]);
"#,
    );
    assert_eq!(l[0], "inflate ret 1 5 17 35 43");
    assert_eq!(l[1], "deflate ret 1 10 20 30 40");
    assert_eq!(l[2], "deflate past ret 1 30 20 10 40", "the arithmetic is stored as it comes: left + 20, right - 20");
    assert_eq!(l[3], "equal same 1");
    assert_eq!(l[4], "equal differs 0");
    assert_eq!(l[5], "equal empty different 0");
    // (10,10)-(20,20): left/top inside, right/bottom outside
    assert_eq!(l[6].trim(), "ptinrect: 1 1 0 0 0 0 0 1");
    assert_eq!(l[7], "ptinrect not normalized 0 0");
    assert_eq!(l[8], "ptinrect not normalized y 0");
    assert_eq!(l[9], "subtract page1 ret 1 10 10 100 100");
    assert_eq!(l[10], "subtract page2 ret 1 10 10 50 100", "(10,10,100,100) minus (50,10,150,150) is (10,10,50,100)");
    assert_eq!(l[11], "subtract left strip ret 1 60 10 100 100", "S-subtractrect-reading: the left strip goes");
    assert_eq!(l[12], "subtract covers all ret 0 10 10 10 100", "S-subtractrect-reading: both edges covered: the empty rectangle, return 0");
    assert_eq!(l[13], "subtract disjoint ret 1 10 10 100 100");
}

// ---------------------------------------------------------------------------------------------
// 5. Windows: the surface of a window, WM_SETREDRAW, LockWindowUpdate, menus, the pointer, hooks, lists
// ---------------------------------------------------------------------------------------------

#[test]
fn a_visible_window_draws_on_its_surface_and_a_window_that_is_not_visible_discards_the_drawing() {
    // R10 of docs/investigations/detach-window.md (one drawing surface per top-level window; decision 0126, the choice S-surface-init: a new surface is all 0) and S-draw-hidden (choice: a window or a parent without
    // WS_VISIBLE has an empty visible region, the drawing is discarded; the page LockWindowUpdate says of an empty visible region that "the called function returns a device context with a visible region that is empty"). Learn
    // GetDC: "retrieves a handle to a device context (DC) for the client area of a specified window". The client area of a window at (10, 20) of size 40 x 30: its origin in the surface of the window is (0, 0) (the
    // non-client area is 0 here: decision 0123). FillRect of (0,0,4,2) in the client DC paints the surface pixels [0,4) x [0,2). A child at (5,6) in the client area of its parent draws at (5,6) of the PARENT's surface.
    // The PM's G2 rule: the first non-background pixel is a pixel with a nonzero colour byte.
    let l = lines(
        "fr11-surface",
        r#"
    uint64_t top = cw(0x80, "t", POPUP | VIS, 10, 20, 40, 30, 0);
    uint64_t kid = cw(0, "k", CHILD | VIS, 5, 6, 10, 10, top);
    uint64_t hid = cw(0x80, "h", POPUP, 0, 0, 20, 20, 0);
    uint64_t dtop = K(fle_imp_GetDC, top), dkid = K(fle_imp_GetDC, kid), dhid = K(fle_imp_GetDC, hid);
    uint64_t br = SOLID(0x000000FF);
    fle_gdc_info gt, gk, gh;
    fle_gdc_dc_info(dtop, &gt); fle_gdc_dc_info(dkid, &gk); fle_gdc_dc_info(dhid, &gh);
    printf("fill top ret %llu\n", U(K(fle_imp_FillRect, dtop, RCT(0, 0, 4, 2), br)));
    printf("top px (0,0) %06x (3,1) %06x (4,0) %06x (0,2) %06x\n", SPX(gt.root_lid, 0, 0), SPX(gt.root_lid, 3, 1), SPX(gt.root_lid, 4, 0), SPX(gt.root_lid, 0, 2));
    printf("fill kid ret %llu\n", U(K(fle_imp_FillRect, dkid, RCT(0, 0, 2, 2), br)));
    printf("kid px on the parent's surface (5,6) %06x (6,7) %06x (7,6) %06x (4,6) %06x\n", SPX(gk.root_lid, 5, 6), SPX(gk.root_lid, 6, 7), SPX(gk.root_lid, 7, 6), SPX(gk.root_lid, 4, 6));
    printf("kid surface is the top's %d\n", gk.root_lid == gt.root_lid);
    printf("fill hidden ret %llu\n", U(K(fle_imp_FillRect, dhid, RCT(0, 0, 4, 4), br)));
    printf("hidden px (0,0) %06x discarded %llu\n", SPX(gh.root_lid, 0, 0) == 0xFFFFFFFFu ? 0u : SPX(gh.root_lid, 0, 0), U(fle_raster_discarded_count()));
"#,
    );
    assert_eq!(l[0], "fill top ret 1");
    // COLORREF 0x000000FF is red 0xFF: 0xFF0000 in 0x00RRGGBB
    assert_eq!(l[1], "top px (0,0) ff0000 (3,1) ff0000 (4,0) 000000 (0,2) 000000");
    assert_eq!(l[2], "fill kid ret 1");
    assert_eq!(l[3], "kid px on the parent's surface (5,6) ff0000 (6,7) ff0000 (7,6) 000000 (4,6) 000000");
    assert_eq!(l[4], "kid surface is the top's 1");
    assert_eq!(l[5], "fill hidden ret 1", "a discarded drawing call still succeeds");
    assert_eq!(l[6], "hidden px (0,0) 000000 discarded 1", "nothing reached the surface of the window that is not visible; the call was counted once");
}

#[test]
fn wm_setredraw_false_discards_the_drawing_of_the_window_and_its_children_until_true() {
    // Learn WM_SETREDRAW: "If this parameter [wParam] is TRUE, then the content can be redrawn after a change. If this parameter is FALSE, then the content can't be redrawn after a change"; "Your application should
    // return 0 if it processes this message" (DefWindowProc returns 0 here: S-setredraw-default, choice). While the state is FALSE the window and its children are drawn on with an empty visible region (S-setredraw-region,
    // choice): the call returns 1 and no pixel changes. WM_SETREDRAW is 0x000B (the page's message number). The page also says that "Passing a WM_SETREDRAW message to the DefWindowProc function removes the WS_VISIBLE style
    // from the window when wParam is set to FALSE": THE RUNTIME DOES NOT DO THAT (decision 0126 keeps WS_VISIBLE; found by decision 0127's tests, recorded as a deviation and not changed): IsWindowVisible stays TRUE. The
    // test pins what the runtime does, and says so.
    let l = lines(
        "fr11-setredraw",
        r#"
    uint64_t top = cw(0x80, "t", POPUP | VIS, 0, 0, 40, 30, 0);
    uint64_t kid = cw(0, "k", CHILD | VIS, 2, 2, 10, 10, top);
    uint64_t dtop = K(fle_imp_GetDC, top), dkid = K(fle_imp_GetDC, kid), br = SOLID(0x000000FF);
    fle_gdc_info gt;
    fle_gdc_dc_info(dtop, &gt);
    printf("send false ret %llu\n", U(K(fle_imp_SendMessageW, top, 0x0B, 0, 0)));
    printf("visible after false (the page says FALSE; the runtime keeps TRUE) %llu\n", U(K(fle_imp_IsWindowVisible, top)));
    K(fle_imp_FillRect, dtop, RCT(0, 0, 4, 4), br);
    printf("top px after false %06x\n", SPX(gt.root_lid, 0, 0) == 0xFFFFFFFFu ? 0u : SPX(gt.root_lid, 0, 0));
    K(fle_imp_FillRect, dkid, RCT(0, 0, 2, 2), br);
    printf("child px after false %06x\n", SPX(gt.root_lid, 2, 2) == 0xFFFFFFFFu ? 0u : SPX(gt.root_lid, 2, 2));
    printf("send true ret %llu\n", U(K(fle_imp_SendMessageW, top, 0x0B, 1, 0)));
    K(fle_imp_FillRect, dtop, RCT(0, 0, 4, 4), br);
    printf("top px after true %06x\n", SPX(gt.root_lid, 0, 0));
    K(fle_imp_FillRect, dkid, RCT(0, 0, 2, 2), br);
    printf("child px after true %06x\n", SPX(gt.root_lid, 2, 2));
"#,
    );
    assert_eq!(l[0], "send false ret 0");
    assert_eq!(l[1], "visible after false (the page says FALSE; the runtime keeps TRUE) 1");
    assert_eq!(l[2], "top px after false 000000");
    assert_eq!(l[3], "child px after false 000000");
    assert_eq!(l[4], "send true ret 0");
    assert_eq!(l[5], "top px after true ff0000");
    assert_eq!(l[6], "child px after true ff0000");
}

#[test]
fn lock_window_update_locks_one_window_discards_its_drawing_and_unlocking_invalidates_the_attempted_area() {
    // Learn LockWindowUpdate: "Only one window can be locked at a time"; "If this parameter is NULL, drawing in the locked window is enabled"; "If the function succeeds, the return value is nonzero. If the function fails, the
    // return value is zero, indicating that an error occurred or another window was already locked"; "If an application with a locked window (or any locked child windows) calls the GetDC, GetDCEx, or BeginPaint function,
    // the called function returns a device context with a visible region that is empty"; "If an application attempts to draw within a locked window, the system records the extent of the attempted operation in a bounding
    // rectangle. When the window is unlocked, the system invalidates the area within this bounding rectangle, forcing an eventual WM_PAINT message ... If no drawing has occurred while the window updates were locked, no area
    // is invalidated"; "does not make the specified window invisible and does not clear the WS_VISIBLE style bit". Choices: LockWindowUpdate(NULL) returns 1 although nothing is locked (S-lock-null); locking the window that is
    // already locked returns 0 (S-lock-twice); the invalidated extent is the whole client area (S-lock-extent); a window that is not a live window: 0 with ERROR_INVALID_WINDOW_HANDLE 1400 ("Invalid window handle."; System
    // Error Codes; the choice of that code for an invalid window is S-err of decision 0123). GetUpdateRect (Learn): "the smallest rectangle that completely encloses the update region".
    let l = lines(
        "fr11-lock",
        r#"
    uint64_t a = cw(0x80, "a", POPUP | VIS, 0, 0, 40, 30, 0), b = cw(0x80, "b", POPUP | VIS, 0, 0, 40, 30, 0);
    uint64_t da = K(fle_imp_GetDC, a), br = SOLID(0x000000FF);
    fle_gdc_info ga;
    int32_t rc[4];
    fle_gdc_dc_info(da, &ga);
    memset(rc, 0x55, sizeof rc);
    printf("update rect before: %llu\n", U(K(fle_imp_GetUpdateRect, a, A(rc), 0)));
    SETLE(0);
    printf("unlock with nothing locked %llu\n", U(K(fle_imp_LockWindowUpdate, 0)));
    printf("lock a %llu\n", U(K(fle_imp_LockWindowUpdate, a)));
    printf("lock b while a is locked %llu\n", U(K(fle_imp_LockWindowUpdate, b)));
    printf("lock a again %llu\n", U(K(fle_imp_LockWindowUpdate, a)));
    printf("a is still visible %llu\n", U(K(fle_imp_IsWindowVisible, a)));
    printf("fill ret %llu\n", U(K(fle_imp_FillRect, da, RCT(0, 0, 4, 4), br)));
    printf("px while locked %06x\n", SPX(ga.root_lid, 0, 0) == 0xFFFFFFFFu ? 0u : SPX(ga.root_lid, 0, 0));
    printf("update rect while locked: %llu\n", U(K(fle_imp_GetUpdateRect, a, A(rc), 0)));
    printf("unlock %llu\n", U(K(fle_imp_LockWindowUpdate, 0)));
    memset(rc, 0x55, sizeof rc);
    printf("update rect after unlock: %llu %d %d %d %d\n", U(K(fle_imp_GetUpdateRect, a, A(rc), 0)), rc[0], rc[1], rc[2], rc[3]);
    K(fle_imp_FillRect, da, RCT(0, 0, 4, 4), br);
    printf("px after unlock %06x\n", SPX(ga.root_lid, 0, 0));
    printf("lock a, nothing drawn: %llu, unlock %llu\n", U(K(fle_imp_LockWindowUpdate, b)), U(K(fle_imp_LockWindowUpdate, 0)));
    printf("update rect of b after an unlock without drawing: %llu\n", U(K(fle_imp_GetUpdateRect, b, A(rc), 0)));
    SETLE(0);
    printf("lock a bad window %llu le %u\n", U(K(fle_imp_LockWindowUpdate, 0x1234)), LE());
"#,
    );
    // the first GetUpdateRect (nothing drawn yet): a new window has an update region only if the window model made one at creation, which the runtime does for a visible window; the value is not part of this test: only
    // that the answers after the lock differ in the way the page says. It is printed and compared for the later lines.
    assert_eq!(l[1], "unlock with nothing locked 1");
    assert_eq!(l[2], "lock a 1");
    assert_eq!(l[3], "lock b while a is locked 0");
    assert_eq!(l[4], "lock a again 0");
    assert_eq!(l[5], "a is still visible 1");
    assert_eq!(l[6], "fill ret 1", "a drawing call at a locked window still succeeds");
    assert_eq!(l[7], "px while locked 000000", "nothing reaches the surface of a locked window");
    assert_eq!(l[9], "unlock 1");
    assert_eq!(l[10], "update rect after unlock: 1 0 0 40 30", "the unlock invalidates the whole client area (S-lock-extent)");
    assert_eq!(l[11], "px after unlock ff0000", "after the unlock drawing reaches the surface again");
    assert_eq!(l[12], "lock a, nothing drawn: 1, unlock 1");
    assert_eq!(l[13], "update rect of b after an unlock without drawing: 0", "no drawing while locked: no area is invalidated");
    assert_eq!(l[14], "lock a bad window 0 le 1400");
}

#[test]
fn the_pointer_calls_keep_one_position_and_get_cursor_info_reports_it_with_the_cursor_state() {
    // Learn GetCursorPos: "retrieves the position of the mouse cursor, in screen coordinates"; SetCursorPos: "Moves the cursor to the specified screen coordinates. If the new coordinates are not within the screen rectangle
    // set by the most recent ClipCursor function call, the system automatically adjusts the coordinates so that the cursor stays within the rectangle"; "Returns nonzero if successful"; GetPhysicalCursorPos: "Retrieves the
    // position of the cursor in physical coordinates", SetPhysicalCursorPos the counterpart. The runtime keeps ONE position that starts at (0, 0) (S-cursor-start, choice), clamps it to the virtual screen [0,1920) x
    // [0,1080) (S-cursor-screen, choice: the screen as the rectangle) and gives the same position in physical and logical coordinates (S-physical-cursor, choice: no DPI virtualisation). Learn CURSORINFO: cbSize "The caller must
    // set this to sizeof(CURSORINFO)"; { DWORD cbSize; DWORD flags; HCURSOR hCursor; POINT ptScreenPos } is 24 bytes on x64 (4 + 4 + 8 + 8: computed from the member types); flags: "0 The cursor is hidden. CURSOR_SHOWING
    // 0x00000001 The cursor is showing."; a cbSize other than 24 fails: 0 with ERROR_INVALID_PARAMETER 87 (S-cursorinfo-size, choice: the page names no code). Learn ShowCursor: "The cursor is displayed only if the display
    // count is greater than or equal to 0. If a mouse is installed, the initial display count is 0" and "If bShow is FALSE, the display count is decremented by one": after ShowCursor(FALSE) the count is -1: hidden.
    let l = lines(
        "fr11-pointer",
        r#"
    int32_t pt[2], ci[6];
    uint64_t r;
    memset(pt, 0x55, sizeof pt);
    r = K(fle_imp_GetCursorPos, A(pt)); printf("start %llu %d %d\n", U(r), pt[0], pt[1]);
    r = K(fle_imp_SetCursorPos, 100, 200); printf("set %llu\n", U(r));
    K(fle_imp_GetCursorPos, A(pt)); printf("get %d %d\n", pt[0], pt[1]);
    K(fle_imp_GetPhysicalCursorPos, A(pt)); printf("physical %d %d\n", pt[0], pt[1]);
    r = K(fle_imp_SetPhysicalCursorPos, 300, 400); printf("setphys %llu\n", U(r));
    K(fle_imp_GetCursorPos, A(pt)); printf("get after setphys %d %d\n", pt[0], pt[1]);
    K(fle_imp_SetCursorPos, NEG(-5), 5000); K(fle_imp_GetCursorPos, A(pt)); printf("clamped low/high %d %d\n", pt[0], pt[1]);
    K(fle_imp_SetCursorPos, 5000, NEG(-5)); K(fle_imp_GetCursorPos, A(pt)); printf("clamped high/low %d %d\n", pt[0], pt[1]);
    K(fle_imp_SetCursorPos, 1919, 1079); K(fle_imp_GetCursorPos, A(pt)); printf("last pixel %d %d\n", pt[0], pt[1]);
    K(fle_imp_SetCursorPos, 1920, 1080); K(fle_imp_GetCursorPos, A(pt)); printf("one past %d %d\n", pt[0], pt[1]);
    memset(ci, 0, sizeof ci); ci[0] = 24;
    SETLE(0);
    r = K(fle_imp_GetCursorInfo, A(ci)); printf("info %llu size %d flags %d cursor %llx pos %d %d\n", U(r), ci[0], ci[1], U(*(uint64_t *)(ci + 2)), ci[4], ci[5]);
    K(fle_imp_ShowCursor, 0);
    memset(ci, 0, sizeof ci); ci[0] = 24;
    r = K(fle_imp_GetCursorInfo, A(ci)); printf("hidden %llu flags %d\n", U(r), ci[1]);
    K(fle_imp_ShowCursor, 1);
    memset(ci, 0, sizeof ci); ci[0] = 20;
    SETLE(0); r = K(fle_imp_GetCursorInfo, A(ci)); printf("bad size %llu le %u\n", U(r), LE());
"#,
    );
    assert_eq!(l[0], "start 1 0 0");
    assert_eq!(l[1], "set 1");
    assert_eq!(l[2], "get 100 200");
    assert_eq!(l[3], "physical 100 200");
    assert_eq!(l[4], "setphys 1");
    assert_eq!(l[5], "get after setphys 300 400");
    assert_eq!(l[6], "clamped low/high 0 1079");
    assert_eq!(l[7], "clamped high/low 1919 0");
    assert_eq!(l[8], "last pixel 1919 1079");
    assert_eq!(l[9], "one past 1919 1079");
    assert_eq!(l[10], "info 1 size 24 flags 1 cursor 0 pos 1919 1079");
    assert_eq!(l[11], "hidden 1 flags 0");
    assert_eq!(l[12], "bad size 0 le 87");
}

#[test]
fn the_virtual_pointer_keeps_set_cursor_pos_as_a_relative_offset() {
    // Decision 0319 O20: Windows's pointer is a virtual cursor. SetCursorPos moves it, and the host
    // (browser) pointer's absolute motion is relative to that new place: the guest position is the
    // host's latest absolute position plus an offset, clamped to the screen. With no offset this is
    // the same behaviour as before.
    let l = lines(
        "fr11-virtual-pointer",
        r#"
    int32_t pt[2], ci[6];
    void fle_win_host_pointer_set(int32_t, int32_t);
    void fle_win_pointer_reclamp(void);
    K(fle_imp_SetCursorPos, 0, 0);
    fle_win_host_pointer_set(10, 10);
    K(fle_imp_GetCursorPos, A(pt)); printf("host 10 10 -> %d %d\n", pt[0], pt[1]);
    K(fle_imp_SetCursorPos, 100, 200);
    K(fle_imp_GetCursorPos, A(pt)); printf("set -> %d %d\n", pt[0], pt[1]);
    fle_win_host_pointer_set(20, 10);
    K(fle_imp_GetCursorPos, A(pt)); printf("host 20 10 -> %d %d\n", pt[0], pt[1]);
    fle_win_host_pointer_set(20, 20);
    K(fle_imp_GetCursorPos, A(pt)); printf("host 20 20 -> %d %d\n", pt[0], pt[1]);
    memset(ci, 0, sizeof ci); ci[0] = 24;
    K(fle_imp_GetCursorInfo, A(ci)); printf("info pos %d %d\n", ci[4], ci[5]);
    fle_win_host_pointer_set(450, 1079);
    K(fle_imp_SetPhysicalCursorPos, 450, 17);
    K(fle_imp_GetCursorPos, A(pt)); printf("wrap set -> %d %d\n", pt[0], pt[1]);
    fle_win_host_pointer_set(450, 20);
    K(fle_imp_GetCursorPos, A(pt)); printf("host 450 20 -> %d %d\n", pt[0], pt[1]);
    fle_win_host_pointer_set(450, 21);
    K(fle_imp_GetCursorPos, A(pt)); printf("host 450 21 -> %d %d\n", pt[0], pt[1]);
    K(fle_imp_SetCursorPos, 1920, 1080);
    K(fle_imp_GetCursorPos, A(pt)); printf("clamped -> %d %d\n", pt[0], pt[1]);
    fle_win_host_pointer_set(21, 20);
    K(fle_imp_GetCursorPos, A(pt)); printf("host 21 20 -> %d %d\n", pt[0], pt[1]);
    fle_win_host_pointer_set(22, 20);
    K(fle_imp_GetCursorPos, A(pt)); printf("host 22 20 -> %d %d\n", pt[0], pt[1]);
    fle_screen_set(1024, 768);
    fle_win_pointer_reclamp();
    K(fle_imp_GetCursorPos, A(pt)); printf("shrunk -> %d %d\n", pt[0], pt[1]);
    fle_win_host_pointer_set(23, 21);
    K(fle_imp_GetCursorPos, A(pt)); printf("host 23 21 -> %d %d\n", pt[0], pt[1]);
"#,
    );
    assert_eq!(l[0], "host 10 10 -> 10 10");
    assert_eq!(l[1], "set -> 100 200");
    assert_eq!(l[2], "host 20 10 -> 20 10");
    assert_eq!(l[3], "host 20 20 -> 20 20");
    assert_eq!(l[4], "info pos 20 20");
    assert_eq!(l[5], "wrap set -> 450 17");
    assert_eq!(l[6], "host 450 20 -> 450 20");
    assert_eq!(l[7], "host 450 21 -> 450 21");
    assert_eq!(l[8], "clamped -> 1919 1079");
    assert_eq!(l[9], "host 21 20 -> 21 20");
    assert_eq!(l[10], "host 22 20 -> 22 20");
    assert_eq!(l[11], "shrunk -> 22 20");
    assert_eq!(l[12], "host 23 21 -> 23 21");
}

#[test]
fn def_window_proc_generates_wm_context_menu_from_right_button_up() {
    // Learn WM_CONTEXTMENU: "DefWindowProc generates the WM_CONTEXTMENU message when it processes the
    // WM_RBUTTONUP or WM_NCRBUTTONUP message"; wParam is the handle of the window in which the button was
    // released (a child forwards to the parent), lParam the cursor position in screen coordinates.
    let l = lines(
        "fr11-contextmenu",
        r#"
    uint64_t top = cw(0x80, "t", POPUP | VIS, 10, 20, 40, 30, 0);
    uint64_t kid = cw(0, "k", CHILD | VIS, 5, 6, 10, 10, top);
    uint64_t r;
    wp_count[0x007B] = 0; last_ctx_target = 0; last_ctx_wp = 0; last_ctx_lp = 0;
    r = K(fle_imp_DefWindowProcW, kid, 0x0205, 2, ((uint64_t)2 << 16) | 1);
    printf("child rbuttonup r %llu ctx %u target %s wparam %s lparam %llx\n", U(r), wp_count[0x007B], last_ctx_target == top ? "top" : "other", last_ctx_wp == kid ? "kid" : "other", U(last_ctx_lp));
    wp_count[0x007B] = 0; last_ctx_target = 0; last_ctx_wp = 0; last_ctx_lp = 0;
    r = K(fle_imp_DefWindowProcW, top, 0x00A5, 1, ((uint64_t)40 << 16) | 30);
    printf("top ncrbuttonup r %llu ctx %u target %s wparam %s lparam %llx\n", U(r), wp_count[0x007B], last_ctx_target == top ? "top" : "other", last_ctx_wp == top ? "top" : "other", U(last_ctx_lp));
"#,
    );
    assert_eq!(l[0], "child rbuttonup r 0 ctx 1 target top wparam kid lparam 1c0010");
    assert_eq!(l[1], "top ncrbuttonup r 0 ctx 1 target top wparam top lparam 28001e");
}

#[test]
fn set_thread_error_mode_returns_the_previous_mode_and_accepts_the_alignment_fault_bit() {
    // Learn SetThreadErrorMode: dwNewMode "can be one or more of the following values": 0 "Use the system default", SEM_FAILCRITICALERRORS 0x0001, SEM_NOGPFAULTERRORBOX 0x0002, SEM_NOOPENFILEERRORBOX 0x8000;
    // lpOldMode "If the function succeeds, this parameter is set to the thread's previous error mode. This parameter can be NULL"; "If the function succeeds, the return value is nonzero". SEM_NOALIGNMENTFAULTEXCEPT 0x0004
    // is not in the page's list of values; the guest passes it (decision 0126 wall 67) and the runtime accepts it like the others (S-threaderrormode-alignment, choice). The old mode of a thread starts at 0.
    let l = lines(
        "fr11-errmode",
        r#"
    uint32_t old = 77;
    uint64_t r;
    r = K(fle_imp_SetThreadErrorMode, 4, A(&old)); printf("set 4 %llu old %u\n", U(r), old);
    r = K(fle_imp_SetThreadErrorMode, 0x8001, A(&old)); printf("set 8001 %llu old %u\n", U(r), old);
    r = K(fle_imp_SetThreadErrorMode, 0, A(&old)); printf("set 0 %llu old %u\n", U(r), old);
    r = K(fle_imp_SetThreadErrorMode, 2, 0); printf("set 2 without old %llu\n", U(r));
    r = K(fle_imp_SetThreadErrorMode, 0, A(&old)); printf("set 0 %llu old %u\n", U(r), old);
"#,
    );
    assert_eq!(l[0], "set 4 1 old 0");
    assert_eq!(l[1], "set 8001 1 old 4");
    assert_eq!(l[2], "set 0 1 old 32769");
    assert_eq!(l[3], "set 2 without old 1");
    assert_eq!(l[4], "set 0 1 old 2");
}

#[test]
fn reg_create_key_ex_w_drops_one_trailing_backslash_and_creates_the_keys_of_the_path() {
    // Learn RegCreateKeyExW: "The RegCreateKeyEx function creates all missing keys in the specified path"; "If lpSubKey is a pointer to an empty string, phkResult receives a new handle to the key specified by hKey"; the
    // return value is ERROR_SUCCESS (0) on success; lpdwDisposition receives REG_CREATED_NEW_KEY 0x1 or REG_OPENED_EXISTING_KEY 0x2 (the page's table). The page says nothing of a backslash at the end of the path:
    // "Software\Image-Line\Shared\" with ONE trailing backslash names the key "Software\Image-Line\Shared" (S-regcreate-trailing-backslash, choice): creating "...\Shared\" after "...\Shared" opens the existing key
    // (disposition 2), and the other way round. HKEY_CURRENT_USER is 0xFFFFFFFF80000001 (the SDK's winreg.h: UNOFFICIAL, the number of the other registry tests).
    let l = lines(
        "fr11-regtrail",
        r#"
    uint64_t h = 0, st;
    uint32_t disp = 77;
    st = KN(fle_imp_RegCreateKeyExW, 9, 0xFFFFFFFF80000001ull, S("Software\\Image-Line\\Shared\\"), 0, 0, 0, 0xF003F, 0, A(&h), A(&disp));
    printf("create with backslash %llu disp %u\n", U(st), disp);
    K(fle_imp_RegCloseKey, h);
    st = KN(fle_imp_RegCreateKeyExW, 9, 0xFFFFFFFF80000001ull, S("Software\\Image-Line\\Shared"), 0, 0, 0, 0xF003F, 0, A(&h), A(&disp));
    printf("create without %llu disp %u\n", U(st), disp);
    K(fle_imp_RegCloseKey, h);
    st = KN(fle_imp_RegCreateKeyExW, 9, 0xFFFFFFFF80000001ull, S("Software\\Image-Line\\Shared\\"), 0, 0, 0, 0xF003F, 0, A(&h), A(&disp));
    printf("again with backslash %llu disp %u\n", U(st), disp);
    K(fle_imp_RegCloseKey, h);
    st = KN(fle_imp_RegCreateKeyExW, 9, 0xFFFFFFFF80000001ull, S("Software\\Image-Line\\Other\\"), 0, 0, 0, 0xF003F, 0, A(&h), A(&disp));
    printf("a new key with backslash %llu disp %u\n", U(st), disp);
    K(fle_imp_RegCloseKey, h);
    st = KN(fle_imp_RegOpenKeyExW, 5, 0xFFFFFFFF80000001ull, S("Software\\Image-Line\\Other"), 0, 0x20019, A(&h));
    printf("the key without the backslash exists %llu\n", U(st));
"#,
    );
    assert_eq!(l[0], "create with backslash 0 disp 1");
    assert_eq!(l[1], "create without 0 disp 2");
    assert_eq!(l[2], "again with backslash 0 disp 2");
    assert_eq!(l[3], "a new key with backslash 0 disp 1");
    assert_eq!(l[4], "the key without the backslash exists 0");
    aborts("fr11-regtrail-double", "uint64_t h; KN(fle_imp_RegCreateKeyExW, 9, 0xFFFFFFFF80000001ull, S(\"Software\\\\X\\\\\\\\\"), 0, 0, 0, 0xF003F, 0, A(&h), 0);", "RegCreateKeyExW", "");
}

#[test]
fn reg_open_key_ex_w_takes_the_wow64_view_bits_as_a_choice_of_view_and_refuses_both() {
    // Learn Accessing an Alternate Registry View: "KEY_WOW64_64KEY 0x0100 Access a 64-bit key from either a 32-bit or 64-bit application. KEY_WOW64_32KEY 0x0200 Access a 32-bit key from either a 32-bit or 64-bit
    // application"; "Either KEY_WOW64_32KEY or KEY_WOW64_64KEY can be specified. If both flags are specified, the function fails with ERROR_INVALID_PARAMETER" (87: System Error Codes) on the functions RegCreateKeyEx,
    // RegDeleteKeyEx and RegOpenKeyEx; (Registry Key Security and Access Rights: KEY_WOW64_64KEY "Indicates that an application on 64-bit Windows should operate on the 64-bit registry view. This flag is ignored by 32-bit
    // Windows"). The environment has one view: either bit is accepted as a choice of view and the key opens as without it (S-wow64-views, choice: the page describes two views of a real machine); the rule that BOTH bits
    // are an error with 87 is the page's (the environment follows it). HKEY_CURRENT_USER 0xFFFFFFFF80000001; HKCU\Software does not exist in a new registry (reg.rs): create it first so that the open has a key to find.
    let l = lines(
        "fr11-wow64",
        r#"
    uint64_t h = 0, k = 0, st;
    uint32_t disp;
    st = KN(fle_imp_RegCreateKeyExW, 9, 0xFFFFFFFF80000001ull, S("Software\\W"), 0, 0, 0, 0xF003F, 0, A(&k), A(&disp));
    K(fle_imp_RegCloseKey, k);
    st = KN(fle_imp_RegOpenKeyExW, 5, 0xFFFFFFFF80000001ull, S("Software\\W"), 0, 0x20019, A(&h)); printf("plain %llu\n", U(st)); K(fle_imp_RegCloseKey, h);
    st = KN(fle_imp_RegOpenKeyExW, 5, 0xFFFFFFFF80000001ull, S("Software\\W"), 0, 0x20019 | 0x100, A(&h)); printf("64key %llu\n", U(st)); K(fle_imp_RegCloseKey, h);
    st = KN(fle_imp_RegOpenKeyExW, 5, 0xFFFFFFFF80000001ull, S("Software\\W"), 0, 0x20019 | 0x200, A(&h)); printf("32key %llu\n", U(st)); K(fle_imp_RegCloseKey, h);
    h = 0x1234;
    st = KN(fle_imp_RegOpenKeyExW, 5, 0xFFFFFFFF80000001ull, S("Software\\W"), 0, 0x20019 | 0x300, A(&h)); printf("both %llu\n", U(st));
"#,
    );
    assert_eq!(l[0], "plain 0");
    assert_eq!(l[1], "64key 0");
    assert_eq!(l[2], "32key 0");
    assert_eq!(l[3], "both 87");
}

#[test]
fn format_message_w_gives_the_text_of_the_page_for_a_system_error_and_fails_by_the_codes_of_the_page() {
    // Learn FormatMessageW: FORMAT_MESSAGE_FROM_SYSTEM 0x1000, FORMAT_MESSAGE_IGNORE_INSERTS 0x200, FORMAT_MESSAGE_ALLOCATE_BUFFER 0x100: "the function allocates a buffer ... and places a pointer to the allocated buffer at the
    // address specified by lpBuffer"; "If the function succeeds, the return value is the number of TCHARs stored in the output buffer, excluding the terminating null character. If the function fails, the return value is
    // zero". The texts are those of Learn System Error Codes (0-499): 2 "The system cannot find the file specified.", 3 "The system cannot find the path specified.", 5 "Access is denied.", 6 "The handle is invalid.", 87 "The
    // parameter is incorrect.", 122 "The data area passed to a system call is too small.", 38 "Reached the end of the file.", 1400 is NOT in 0-499 and has no text here. The text is stored without a line break
    // (S-errmsg-crlf, choice: the page's text exactly). A buffer that is too small: 0 and ERROR_INSUFFICIENT_BUFFER 122 ("The data area passed to a system call is too small."); a message number that the page does not
    // list: 0 and ERROR_MR_MID_NOT_FOUND 317 ("The system cannot find message text for message number 0x%1 in the message file for %2."): the codes the pages name.
    let l = lines(
        "fr11-fmtmsg",
        r#"
    static uint16_t buf[200];
    static const uint32_t ids[] = { 2, 3, 5, 6, 38, 87, 122, 0, 1 };
    unsigned i;
    uint64_t r;
    char out[300];
    uint64_t p = 0;
    for (i = 0; i < sizeof ids / sizeof ids[0]; i++) {
        memset(buf, 0, sizeof buf);
        r = KN(fle_imp_FormatMessageW, 7, 0x1200, 0, ids[i], 0, A(buf), 200, 0);
        wtoa(A(buf), out, sizeof out);
        printf("id %u n %llu [%s]\n", ids[i], U(r), out);
    }
    r = KN(fle_imp_FormatMessageW, 7, 0x1300, 0, 5, 0, A(&p), 0, 0);
    wtoa(p, out, sizeof out);
    printf("alloc n %llu [%s] ptr %d\n", U(r), out, p != 0);
    SETLE(0);
    r = KN(fle_imp_FormatMessageW, 7, 0x1200, 0, 5, 0, A(buf), 5, 0);
    printf("small n %llu le %u\n", U(r), LE());
    SETLE(0);
    r = KN(fle_imp_FormatMessageW, 7, 0x1200, 0, 1400, 0, A(buf), 200, 0);
    printf("unlisted n %llu le %u\n", U(r), LE());
    memset(buf, 0, sizeof buf);
    SETLE(0);
    r = KN(fle_imp_FormatMessageW, 7, 0x1200, 0, 5, 0, A(buf), 17, 0);
    printf("one unit short (the text has 17 characters, the NUL needs the 18th) n %llu le %u\n", U(r), LE());
    r = KN(fle_imp_FormatMessageW, 7, 0x1200, 0, 5, 0, A(buf), 18, 0);
    wtoa(A(buf), out, sizeof out);
    printf("exact fit n %llu [%s]\n", U(r), out);
"#,
    );
    let want = [
        ("2", "The system cannot find the file specified."),
        ("3", "The system cannot find the path specified."),
        ("5", "Access is denied."),
        ("6", "The handle is invalid."),
        ("38", "Reached the end of the file."),
        ("87", "The parameter is incorrect."),
        ("122", "The data area passed to a system call is too small."),
        ("0", "The operation completed successfully."),
        ("1", "Incorrect function."),
    ];
    for (k, (id, text)) in want.iter().enumerate() {
        assert_eq!(l[k], format!("id {id} n {} [{text}]", text.len()), "message {id}");
    }
    assert_eq!(l[9], "alloc n 17 [Access is denied.] ptr 1");
    assert_eq!(l[10], "small n 0 le 122");
    assert_eq!(l[11], "unlisted n 0 le 317");
    assert_eq!(l[12], "one unit short (the text has 17 characters, the NUL needs the 18th) n 0 le 122");
    assert_eq!(l[13], "exact fit n 17 [Access is denied.]");
}

#[test]
fn direct_sound_enumerate_a_calls_the_callback_and_a_null_callback_is_invalid_param() {
    // decision 0139 (old: S-ds-none, "the callback is never called and the result is DS_OK"): the virtual machine has one playback device and DirectSoundEnumerateA calls the guest's callback for it (tests/wall_fr15.rs runs the
    // whole enumeration). The callback pointer here is a guest address (0x12af280 as the guest passes it) that is not in the dispatch table of this test, so the call stops by name. A NULL callback is DSERR_INVALIDPARAM
    // (the DirectSoundEnumerate page: "If it fails, the return value may be DSERR_INVALIDPARAM"; E_INVALIDARG = 0x80070057, the DirectSound Return Values by Number table) instead of the old stop.
    aborts("fr11-dsound", "K(fle_imp_DirectSoundEnumerateA, 0x12af280, 0);", "indirect call to 0x12af280", "is not a known function entry");
    let l = lines("fr11-dsound-null", "printf(\"ret %llx\\n\", U(K(fle_imp_DirectSoundEnumerateA, 0, 0)));");
    assert_eq!(l, ["ret 80070057"]);
}

#[test]
fn midi_out_get_dev_caps_w_gives_nodriver_for_the_mapper_and_baddeviceid_for_a_number() {
    // Learn midiOutGetDevCapsW: "The device identifier ... varies from zero to one less than the number of devices present. The MIDI_MAPPER constant is also a valid device identifier"; the errors the page lists:
    // MMSYSERR_BADDEVICEID "The specified device identifier is out of range.", MMSYSERR_INVALPARAM "The specified pointer or structure is invalid.", MMSYSERR_NODRIVER "The driver is not installed."; MMSYSERR_NOERROR 0.
    // The numbers are those of mmsystem.h (UNOFFICIAL): MMSYSERR_BADDEVICEID 2, MMSYSERR_NODRIVER 6; MIDI_MAPPER is (UINT)-1. With no MIDI output device a number is out of range: 2. The mapper is a driver that is not
    // installed: 6 (S-midi-mapper-nodriver, choice: the page does not say which error the mapper gives when no device exists). The structure is not written (the buffer keeps its bytes).
    let l = lines(
        "fr11-midi",
        r#"
    uint8_t caps[0x54];
    uint64_t r;
    memset(caps, 0xAB, sizeof caps);
    r = K(fle_imp_midiOutGetDevCapsW, 0, A(caps), 0x54); printf("device 0 %llu\n", U(r));
    r = K(fle_imp_midiOutGetDevCapsW, 5, A(caps), 0x54); printf("device 5 %llu\n", U(r));
    r = K(fle_imp_midiOutGetDevCapsW, 0xFFFFFFFFull, A(caps), 0x54); printf("mapper 32-bit %llu\n", U(r));
    r = K(fle_imp_midiOutGetDevCapsW, 0xFFFFFFFFFFFFFFFFull, A(caps), 0x54); printf("mapper sign-extended %llu\n", U(r));
    { unsigned i, same = 1; for (i = 0; i < sizeof caps; i++) if (caps[i] != 0xAB) same = 0; printf("caps untouched %u\n", same); }
    r = K(fle_imp_midiInGetDevCapsW, 0xFFFFFFFFull, A(caps), 0x54); printf("midi in mapper %llu\n", U(r));
"#,
    );
    assert_eq!(l[0], "device 0 2");
    assert_eq!(l[1], "device 5 2");
    assert_eq!(l[2], "mapper 32-bit 6");
    assert_eq!(l[3], "mapper sign-extended 6");
    assert_eq!(l[4], "caps untouched 1");
    assert_eq!(l[5], "midi in mapper 2", "the choice is for the OUTPUT mapper only: a MIDI input device number is out of range");
}

#[test]
fn wait_for_multiple_objects_ex_is_the_wait_of_wait_for_multiple_objects_whatever_alertable_says() {
    // Learn WaitForMultipleObjectsEx: "bWaitAll: If this parameter is TRUE, the function returns when the state of all objects in the lpHandles array is set to signaled. If FALSE, the function returns when the state of any
    // one of the objects is set to signaled. In the latter case, the return value indicates the object whose state caused the function to return"; "WAIT_OBJECT_0 to (WAIT_OBJECT_0 + nCount-1)": "the return value minus
    // WAIT_OBJECT_0 indicates the lpHandles array index of the object that satisfied the wait. If more than one object became signaled during the call, this is the array index of the signaled object with the smallest index
    // value"; WAIT_TIMEOUT 0x00000102L "The time-out interval elapsed, the conditions specified by the bWaitAll parameter were not satisfied, and no completion routines are queued" (the number is on the page; the SDK's numbers: WAIT_OBJECT_0 0, WAIT_TIMEOUT 0x102, UNOFFICIAL); bAlertable
    // "If this parameter is TRUE and the thread is in the waiting state, the function returns when the system queues an I/O completion routine or APC": no APC can be queued here (no ReadFileEx, WriteFileEx, QueueUserAPC
    // row), so bAlertable changes nothing (S-alertable-no-apc, choice). Two manual-reset events: e0 not signalled, e1 signalled. Any (FALSE): index 1; all (TRUE) with a timeout of 0: WAIT_TIMEOUT.
    let l = lines(
        "fr11-waitex",
        r#"
    uint64_t e0 = K(fle_imp_CreateEventW, 0, 1, 0, 0), e1 = K(fle_imp_CreateEventW, 0, 1, 1, 0), h[2];
    uint64_t r;
    h[0] = e0; h[1] = e1;
    r = KN(fle_imp_WaitForMultipleObjectsEx, 5, 2, A(h), 0, 0, 0); printf("any alertable 0 -> %llx\n", U(r));
    r = KN(fle_imp_WaitForMultipleObjectsEx, 5, 2, A(h), 0, 0, 1); printf("any alertable 1 -> %llx\n", U(r));
    r = KN(fle_imp_WaitForMultipleObjectsEx, 5, 2, A(h), 1, 0, 0); printf("all alertable 0 -> %llx\n", U(r));
    r = KN(fle_imp_WaitForMultipleObjectsEx, 5, 2, A(h), 1, 0, 1); printf("all alertable 1 -> %llx\n", U(r));
    K(fle_imp_SetEvent, e0);
    r = KN(fle_imp_WaitForMultipleObjectsEx, 5, 2, A(h), 1, 0, 1); printf("all signalled -> %llx\n", U(r));
    r = KN(fle_imp_WaitForMultipleObjectsEx, 5, 2, A(h), 0, 0, 1); printf("both signalled, any -> %llx\n", U(r));
    r = K(fle_imp_WaitForMultipleObjects, 2, A(h), 0, 0); printf("same call without Ex -> %llx\n", U(r));
"#,
    );
    assert_eq!(l[0], "any alertable 0 -> 1");
    assert_eq!(l[1], "any alertable 1 -> 1");
    assert_eq!(l[2], "all alertable 0 -> 102");
    assert_eq!(l[3], "all alertable 1 -> 102");
    assert_eq!(l[4], "all signalled -> 0");
    assert_eq!(l[5], "both signalled, any -> 0", "the smallest index of the signaled objects");
    assert_eq!(l[6], "same call without Ex -> 0");
}

#[test]
fn check_menu_item_and_enable_menu_item_of_a_menu_that_does_not_exist_return_uint_max() {
    // Learn CheckMenuItem: "The return value specifies the previous state of the menu item (either MF_CHECKED or MF_UNCHECKED). If the menu item does not exist, the return value is UINT_MAX" (0xFFFFFFFF). Learn
    // EnableMenuItem: "The return value specifies the previous state of the menu item (it is either MF_DISABLED, MF_ENABLED, or MF_GRAYED). If the menu item does not exist, the return value is -1" (0xFFFFFFFF). The guest
    // asks about item 0x75 of the menu NULL: the menu does not exist, so the item does not exist. A handle that is not a menu of the runtime is the same. MF_CHECKED is 0x08 (the SDK's number: UNOFFICIAL). The last
    // error is left alone (the page names none).
    let l = lines(
        "fr11-menu",
        r#"
    SETLE(0);
    printf("check NULL menu %08llx\n", U((uint32_t)K(fle_imp_CheckMenuItem, 0, 0x75, 0x08)));
    printf("enable NULL menu %08llx\n", U((uint32_t)K(fle_imp_EnableMenuItem, 0, 0x75, 0)));
    printf("check stray menu %08llx\n", U((uint32_t)K(fle_imp_CheckMenuItem, 0x1234, 0x75, 0x08)));
    printf("enable stray menu %08llx\n", U((uint32_t)K(fle_imp_EnableMenuItem, 0x1234, 0x75, 1)));
    printf("by position %08llx\n", U((uint32_t)K(fle_imp_CheckMenuItem, 0, 0, 0x08 | 0x400)));
    printf("le %u\n", LE());
"#,
    );
    assert_eq!(l, ["check NULL menu ffffffff", "enable NULL menu ffffffff", "check stray menu ffffffff", "enable stray menu ffffffff", "by position ffffffff", "le 0"]);
}

#[test]
fn adjust_window_rect_ex_returns_the_rectangle_as_it_is_because_the_non_client_area_is_zero() {
    // Learn AdjustWindowRectEx: "Calculates the required size of the window rectangle, based on the desired size of the client rectangle"; nonzero on success. The page gives no frame, caption or menu-bar size, and does not
    // say what a NULL rectangle does. The windows of this environment have a non-client area of 0 (decision 0123: the client rectangle is the window rectangle), so the window rectangle for a desired client rectangle is that
    // rectangle itself, whatever the style, the menu flag and the extended style (S-adjustrect-nc-zero and S-adjustrect-style-unchecked, choices: withdrawn when a non-client area with sizes from a source is modelled).
    let l = lines(
        "fr11-adjust",
        r#"
    int32_t rc[4] = { 10, 20, 300, 200 };
    uint64_t r;
    r = K(fle_imp_AdjustWindowRectEx, A(rc), 0x00CF0000, 1, 0x100); printf("overlapped window with menu %llu %d %d %d %d\n", U(r), rc[0], rc[1], rc[2], rc[3]);
    r = K(fle_imp_AdjustWindowRectEx, A(rc), 0x80000000u, 0, 0); printf("popup %llu %d %d %d %d\n", U(r), rc[0], rc[1], rc[2], rc[3]);
    r = K(fle_imp_AdjustWindowRectEx, A(rc), 0, 0, 0); printf("style 0 %llu %d %d %d %d\n", U(r), rc[0], rc[1], rc[2], rc[3]);
"#,
    );
    assert_eq!(l[0], "overlapped window with menu 1 10 20 300 200");
    assert_eq!(l[1], "popup 1 10 20 300 200");
    assert_eq!(l[2], "style 0 1 10 20 300 200");
    aborts("fr11-adjust-null", "K(fle_imp_AdjustWindowRectEx, 0, 0, 0, 0);", "AdjustWindowRectEx", "lpRect is NULL");
}

#[test]
fn internet_get_connected_state_says_there_is_no_connection_and_sets_no_flag() {
    // Learn InternetGetConnectedState: "Returns TRUE if there is an active modem or a LAN Internet connection, or FALSE if there is no Internet connection, or if all possible Internet connections are not currently active";
    // "This parameter [lpdwFlags] may return a valid flag even when the function returns FALSE"; dwReserved "is reserved and must be 0" (a nonzero value: the page gives no behaviour: the run stops). The virtual machine has
    // no Internet connection (S-inet-offline, choice: the definition of the environment): FALSE and the flags 0 (no modem, no LAN, no proxy, not in offline mode). The flags are written even when the call fails (a NULL
    // pointer is allowed: nothing is written).
    let l = lines(
        "fr11-inet",
        r#"
    uint32_t flags = 0x55555555;
    printf("ret %llu flags %x\n", U(K(fle_imp_InternetGetConnectedState, A(&flags), 0)), flags);
    printf("null flags %llu\n", U(K(fle_imp_InternetGetConnectedState, 0, 0)));
"#,
    );
    assert_eq!(l, ["ret 0 flags 0", "null flags 0"]);
    aborts("fr11-inet-reserved", "uint32_t f; K(fle_imp_InternetGetConnectedState, A(&f), 1);", "InternetGetConnectedState", "dwReserved is not 0");
}

#[test]
fn the_singly_linked_list_calls_push_and_pop_the_guests_entries_and_return_the_previous_first_item() {
    // Learn InterlockedPushEntrySList: "The return value is the previous first item in the list. If the list was previously empty, the return value is NULL." InterlockedPopEntrySList: "a pointer to the item removed from
    // the list. If the list is empty, the return value is NULL". SLIST_ENTRY is { SLIST_ENTRY *Next }: Push writes the previous first item into the entry's Next (offset 0), Pop reads the first entry's Next to make it the new
    // first. The list head must have been initialised by InitializeSListHead. Entries are 16-byte aligned (MEMORY_ALLOCATION_ALIGNMENT on x64: the SDK's value, UNOFFICIAL: the page says the item "must be aligned on a
    // MEMORY_ALLOCATION_ALIGNMENT boundary"); an unaligned entry stops (S-slist, choice: the state of the list is kept outside the SLIST_HEADER).
    let l = lines(
        "fr11-slist",
        r#"
    static uint64_t head[2] __attribute__((aligned(16)));
    static uint64_t e[3][2] __attribute__((aligned(16)));
    uint64_t r;
    K(fle_imp_InitializeSListHead, A(head));
    r = K(fle_imp_InterlockedPopEntrySList, A(head)); printf("pop empty %llx\n", U(r));
    r = K(fle_imp_InterlockedPushEntrySList, A(head), A(e[0])); printf("push 0 -> previous first %llx\n", U(r));
    r = K(fle_imp_InterlockedPushEntrySList, A(head), A(e[1])); printf("push 1 -> previous first is e0 %d\n", r == A(e[0]));
    r = K(fle_imp_InterlockedPushEntrySList, A(head), A(e[2])); printf("push 2 -> previous first is e1 %d\n", r == A(e[1]));
    printf("next of e2 is e1 %d, next of e1 is e0 %d, next of e0 is 0 %d\n", e[2][0] == A(e[1]), e[1][0] == A(e[0]), e[0][0] == 0);
    r = K(fle_imp_InterlockedPopEntrySList, A(head)); printf("pop gives e2 %d\n", r == A(e[2]));
    r = K(fle_imp_InterlockedPopEntrySList, A(head)); printf("pop gives e1 %d\n", r == A(e[1]));
    r = K(fle_imp_InterlockedPopEntrySList, A(head)); printf("pop gives e0 %d\n", r == A(e[0]));
    r = K(fle_imp_InterlockedPopEntrySList, A(head)); printf("pop empty again %llx\n", U(r));
"#,
    );
    assert_eq!(
        l,
        [
            "pop empty 0",
            "push 0 -> previous first 0",
            "push 1 -> previous first is e0 1",
            "push 2 -> previous first is e1 1",
            "next of e2 is e1 1, next of e1 is e0 1, next of e0 is 0 1",
            "pop gives e2 1",
            "pop gives e1 1",
            "pop gives e0 1",
            "pop empty again 0",
        ]
    );
    aborts("fr11-slist-unaligned", "static uint64_t head[2] __attribute__((aligned(16))); static uint8_t e[64] __attribute__((aligned(16))); K(fle_imp_InitializeSListHead, A(head)); K(fle_imp_InterlockedPushEntrySList, A(head), A(e) + 8);", "InterlockedPushEntrySList", "not aligned on 16 bytes");
}

#[test]
fn il_create_from_path_w_and_the_shell_change_notify_calls_follow_the_pages_and_the_choices_of_the_decision() {
    // Learn ILCreateFromPathW: "Returns the ITEMIDLIST structure associated with a specified file path"; "Call ILFree to release the ITEMIDLIST when you are finished with it"; the page names no failure value. A path of the
    // virtual file system gives a live ITEMIDLIST (the form of its bytes is the runtime's own: S-il-path-form, choice); a path that the virtual file system does not hold gives NULL (S-il-missing-null, choice).
    // Learn SHChangeNotifyRegister: "Registers a window to receive notifications from the file system or Shell"; "Returns a positive integer registration ID. Returns 0 if out of memory or in response to invalid
    // parameters"; fSources: SHCNRF_ShellLevel 0x0002, SHCNRF_NewDelivery 0x8000 (the numbers are on the page). The IDs are 1, 2, 3 (S-shchange-id, choice: the page says only "positive"). A registration is recorded and
    // never delivers (S-shchange-no-events, choice). SHChangeNotifyDeregister: "Returns TRUE if the specified client was found and removed; otherwise FALSE". ILFree: "pidl can be NULL" and frees a list of the shell
    // allocator.
    let l = lines(
        "fr11-il",
        r#"
    uint64_t dir = KN(fle_imp_CreateDirectoryW, 2, S("C:\\ildir"), 0);
    uint64_t pidl, pidl2, w = cw(0x80, "w", POPUP | VIS, 0, 0, 20, 20, 0);
    uint64_t ent[2];
    uint64_t id1, id2, id3;
    printf("dir %llu\n", U(dir));
    pidl = K(fle_imp_ILCreateFromPathW, S("C:\\ildir"));
    printf("existing path gives a list %d\n", pidl != 0);
    printf("missing path gives NULL %llx\n", U(K(fle_imp_ILCreateFromPathW, S("C:\\no\\such\\path"))));
    pidl2 = K(fle_imp_ILCreateFromPathW, S("C:\\ildir"));
    printf("two lists for one path are two blocks %d\n", pidl2 != 0 && pidl2 != pidl);
    ent[0] = pidl; ent[1] = 0;
    id1 = KN(fle_imp_SHChangeNotifyRegister, 6, w, 0x8003, 0x7FFFFFFF, 0x401, 1, A(ent));
    id2 = KN(fle_imp_SHChangeNotifyRegister, 6, w, 0x8003, 0x7FFFFFFF, 0x402, 1, A(ent));
    id3 = KN(fle_imp_SHChangeNotifyRegister, 6, w, 0x0002, 0x7FFFFFFF, 0x403, 1, A(ent));
    printf("ids %llu %llu %llu live %u\n", U(id1), U(id2), U(id3), fle_ole_shchange_live());
    printf("bad source bits %llu\n", U(KN(fle_imp_SHChangeNotifyRegister, 6, w, 0x0004, 0x7FFFFFFF, 0x404, 1, A(ent))));
    printf("no level bit %llu\n", U(KN(fle_imp_SHChangeNotifyRegister, 6, w, 0x8000, 0x7FFFFFFF, 0x404, 1, A(ent))));
    printf("window that is not live %llu\n", U(KN(fle_imp_SHChangeNotifyRegister, 6, 0x1234, 0x8003, 0x7FFFFFFF, 0x404, 1, A(ent))));
    { uint64_t nent[2], idn, live0 = fle_ole_shchange_live(); nent[0] = 0; nent[1] = 0; idn = KN(fle_imp_SHChangeNotifyRegister, 6, w, 0x8003, 0x7FFFFFFF, 0x405, 1, A(nent));
      printf("NULL pidl id %llu live %u deregister %llu live %u\n", U(idn), fle_ole_shchange_live() - (unsigned)live0, U(K(fle_imp_SHChangeNotifyDeregister, idn)), fle_ole_shchange_live() - (unsigned)live0); }
    printf("deregister 2 %llu again %llu unknown %llu zero %llu live %u\n", U(K(fle_imp_SHChangeNotifyDeregister, id2)), U(K(fle_imp_SHChangeNotifyDeregister, id2)), U(K(fle_imp_SHChangeNotifyDeregister, 99)), U(K(fle_imp_SHChangeNotifyDeregister, 0)), fle_ole_shchange_live());
    printf("deregister 1 %llu 3 %llu live %u\n", U(K(fle_imp_SHChangeNotifyDeregister, id1)), U(K(fle_imp_SHChangeNotifyDeregister, id3)), fle_ole_shchange_live());
    K(fle_imp_ILFree, 0);
    K(fle_imp_ILFree, pidl);
    K(fle_imp_ILFree, pidl2);
    printf("freed\n");
"#,
    );
    assert_eq!(l[0], "dir 1");
    assert_eq!(l[1], "existing path gives a list 1");
    assert_eq!(l[2], "missing path gives NULL 0");
    assert_eq!(l[3], "two lists for one path are two blocks 1");
    assert_eq!(l[4], "ids 1 2 3 live 3");
    assert_eq!(l[5], "bad source bits 0");
    assert_eq!(l[6], "no level bit 0");
    assert_eq!(l[7], "window that is not live 0");
    // anim1 (decision 0225): a NULL pidl registers like any other (Wine 10.0 changenotify.c: ILClone(NULL), a positive id; Learn is silent): the 4th id, one live entry, removed again.
    assert_eq!(l[8], "NULL pidl id 4 live 1 deregister 1 live 0");
    assert_eq!(l[9], "deregister 2 1 again 0 unknown 0 zero 0 live 2");
    assert_eq!(l[10], "deregister 1 1 3 1 live 0");
    assert_eq!(l[11], "freed");
}

#[test]
fn register_drag_drop_checks_the_window_the_ole_initialisation_and_a_second_registration_and_calls_add_ref_and_release() {
    // Learn RegisterDragDrop: S_OK on success; DRAGDROP_E_INVALIDHWND "Invalid handle returned in the hwnd parameter", DRAGDROP_E_ALREADYREGISTERED "The specified window has already been registered as a drop target",
    // E_OUTOFMEMORY "Insufficient memory for the operation"; "If you use CoInitialize or CoInitializeEx instead of OleInitialize to initialize COM, RegisterDragDrop will always return an E_OUTOFMEMORY error"; "The RegisterDragDrop function
    // also calls the IUnknown::AddRef method on the IDropTarget pointer". RevokeDragDrop: S_OK; DRAGDROP_E_NOTREGISTERED, DRAGDROP_E_INVALIDHWND; "This function calls the IUnknown::Release method for your drop target
    // interface". The HRESULT numbers are the SDK's winerror.h (UNOFFICIAL): DRAGDROP_E_NOTREGISTERED 0x80040100, DRAGDROP_E_ALREADYREGISTERED 0x80040101, DRAGDROP_E_INVALIDHWND 0x80040104, E_OUTOFMEMORY 0x8007000E.
    // The order of the checks (invalid window, then the OLE state, then a second registration) is the choice S-dragdrop-order; "OleInitialize was called" is a state of the whole process (S-dragdrop-ole-global, choice).
    // AddRef is vtable slot 1 and Release slot 2 of the guest's object (the order of IUnknown: QueryInterface, AddRef, Release); the test's guest object has them at 0x3000 and 0x3010 of its dispatch table, and each call
    // is counted. Only CoInitialize is called first: RegisterDragDrop must still say E_OUTOFMEMORY.
    let l = lines(
        "fr11-dragdrop",
        r#"
    uint64_t w = cw(0x80, "w", POPUP | VIS, 0, 0, 20, 20, 0), w2 = cw(0x80, "w2", POPUP | VIS, 0, 0, 20, 20, 0);
    static uint64_t vt[3], obj[1];
    vt[0] = 0; vt[1] = VA_ADDREF; vt[2] = VA_RELEASE; obj[0] = A(vt);
    printf("not initialised %08llx addref %u\n", U((uint32_t)K(fle_imp_RegisterDragDrop, w, A(obj))), addref_calls);
    printf("invalid window %08llx\n", U((uint32_t)K(fle_imp_RegisterDragDrop, 0x1234, A(obj))));
    printf("revoke invalid window %08llx\n", U((uint32_t)K(fle_imp_RevokeDragDrop, 0x1234)));
    printf("revoke not registered %08llx\n", U((uint32_t)K(fle_imp_RevokeDragDrop, w)));
    K(fle_imp_CoInitialize, 0);
    printf("CoInitialize only %08llx addref %u\n", U((uint32_t)K(fle_imp_RegisterDragDrop, w, A(obj))), addref_calls);
    K(fle_imp_OleInitialize, 0);
    printf("registered %08llx addref %u this is the object %d live %u\n", U((uint32_t)K(fle_imp_RegisterDragDrop, w, A(obj))), addref_calls, last_unknown_this == A(obj), fle_ole_dragdrop_live());
    printf("second registration %08llx addref %u live %u\n", U((uint32_t)K(fle_imp_RegisterDragDrop, w, A(obj))), addref_calls, fle_ole_dragdrop_live());
    printf("another window %08llx addref %u live %u\n", U((uint32_t)K(fle_imp_RegisterDragDrop, w2, A(obj))), addref_calls, fle_ole_dragdrop_live());
    printf("revoke %08llx release %u live %u\n", U((uint32_t)K(fle_imp_RevokeDragDrop, w)), release_calls, fle_ole_dragdrop_live());
    printf("revoke again %08llx release %u\n", U((uint32_t)K(fle_imp_RevokeDragDrop, w)), release_calls);
    printf("register again after the revoke %08llx addref %u\n", U((uint32_t)K(fle_imp_RegisterDragDrop, w, A(obj))), addref_calls);
    printf("a null target stops after this line\n");
"#,
    );
    assert_eq!(l[0], "not initialised 8007000e addref 0", "a live window, without OleInitialize: E_OUTOFMEMORY, AddRef not called");
    assert_eq!(l[1], "invalid window 80040104", "DRAGDROP_E_INVALIDHWND wins over the state of OLE");
    assert_eq!(l[2], "revoke invalid window 80040104");
    assert_eq!(l[3], "revoke not registered 80040100");
    assert_eq!(l[4], "CoInitialize only 8007000e addref 0");
    assert_eq!(l[5], "registered 00000000 addref 1 this is the object 1 live 1");
    assert_eq!(l[6], "second registration 80040101 addref 1 live 1");
    assert_eq!(l[7], "another window 00000000 addref 2 live 2");
    assert_eq!(l[8], "revoke 00000000 release 1 live 1");
    assert_eq!(l[9], "revoke again 80040100 release 1");
    assert_eq!(l[10], "register again after the revoke 00000000 addref 3");
    aborts("fr11-dragdrop-null", "uint64_t w = cw(0x80, \"w\", POPUP | VIS, 0, 0, 20, 20, 0); K(fle_imp_OleInitialize, 0); K(fle_imp_RegisterDragDrop, w, 0);", "RegisterDragDrop", "pDropTarget is NULL");
}

#[test]
fn set_windows_hook_ex_w_records_a_mouse_or_keyboard_hook_of_this_process_and_never_calls_it() {
    // Learn SetWindowsHookExW: "Installs an application-defined hook procedure into a hook chain"; WH_KEYBOARD 2 "monitors keystroke messages", WH_MOUSE 7 "monitors mouse messages" (the numbers are on the page);
    // dwThreadId "the identifier of the thread with which the hook procedure is to be associated"; "If the dwThreadId parameter is zero or specifies the identifier of a thread created by a different process, the lpfn
    // parameter must point to a hook procedure in a DLL. Otherwise, lpfn can point to a hook procedure in the code associated with the current process"; hmod "must be set to NULL if the dwThreadId parameter specifies a
    // thread created by the current process"; the result is the handle or NULL. UnhookWindowsHookEx: nonzero on success, zero on failure. The runtime accepts WH_MOUSE and WH_KEYBOARD for a thread of this process (the first
    // thread's id is 0x1001) with hmod NULL and lpfn not NULL, records the hook and NEVER CALLS it (S-hook-no-events, choice: no mouse or keyboard message exists here); the handle is 0x7FFA0000 + 4 * (n + 1) for the n-th
    // hook (a choice: the page gives no value); every other hook type, dwThreadId 0 and a thread of no live process stop; UnhookWindowsHookEx of a handle that is not a live hook returns 0.
    let l = lines(
        "fr11-hook",
        r#"
    uint64_t h1 = K(fle_imp_SetWindowsHookExW, 7, 0x809740, 0, 0x1001), h2 = K(fle_imp_SetWindowsHookExW, 2, 0x809750, 0, 0x1001);
    printf("handles %llx %llx live %u\n", U(h1), U(h2), fle_win_hook_live());
    printf("unhook 1 %llu again %llu bogus %llu live %u\n", U(K(fle_imp_UnhookWindowsHookEx, h1)), U(K(fle_imp_UnhookWindowsHookEx, h1)), U(K(fle_imp_UnhookWindowsHookEx, 0x1234)), fle_win_hook_live());
    printf("unhook 2 %llu live %u\n", U(K(fle_imp_UnhookWindowsHookEx, h2)), fle_win_hook_live());
"#,
    );
    assert_eq!(l[0], "handles 7ffa0004 7ffa0008 live 2");
    assert_eq!(l[1], "unhook 1 1 again 0 bogus 0 live 1");
    assert_eq!(l[2], "unhook 2 1 live 0");
    // decision 0171 (wall ia1-W14): old: idHook 4 (WH_CALLWNDPROC) stopped here ("idHook 4 is not WH_MOUSE"); it is installed and called for sent messages now (tests/wall_ia1w.rs); WH_CBT (5) still stops
    aborts("fr11-hook-cbt", "K(fle_imp_SetWindowsHookExW, 5, 0x809740, 0, 0x1001);", "SetWindowsHookExW", "idHook 5 is not WH_MOUSE");
    aborts("fr11-hook-global", "K(fle_imp_SetWindowsHookExW, 7, 0x809740, 0, 0);", "SetWindowsHookExW", "dwThreadId is 0");
    aborts("fr11-hook-hmod", "K(fle_imp_SetWindowsHookExW, 7, 0x809740, 0x400000, 0x1001);", "SetWindowsHookExW", "hmod is not NULL");
}

#[test]
fn get_update_rgn_and_validate_rgn_copy_and_remove_the_update_rectangle_of_the_window() {
    // Learn GetUpdateRgn: "retrieves the update region of a window by copying it into the specified region. The coordinates of the update region are relative to the upper-left corner of the window (that is, they are
    // client coordinates)"; the return value is the complexity of the region: NULLREGION "Region is empty", SIMPLEREGION "Region is a single rectangle", ERROR "An error occurred" (1, 2, 0: the SDK's numbers, UNOFFICIAL).
    // ValidateRgn: "validates the client area within a region by removing the region from the current update region of the specified window"; "If this parameter [hRgn] is NULL, the entire client area is removed"; nonzero
    // on success. The update region of a window is ONE bounding rectangle here (S-updregion of decision 0123, choice): ValidateRgn empties it when the region covers all of it and leaves it otherwise (S-updregion). InvalidateRect makes the
    // rectangle (Learn InvalidateRect: "adds a rectangle to the specified window's update region").
    let l = lines(
        "fr11-updrgn",
        r#"
    uint64_t w = cw(0x80, "w", POPUP | VIS, 0, 0, 100, 80, 0);
    uint64_t rgn = K(fle_imp_CreateRectRgn, 0, 0, 1, 1), cover = K(fle_imp_CreateRectRgn, 0, 0, 100, 80), part = K(fle_imp_CreateRectRgn, 0, 0, 10, 10);
    int32_t box[4] = { 0, 0, 0, 0 };
    int32_t ir[4] = { 10, 20, 50, 60 };
    uint64_t r;
    K(fle_imp_ValidateRgn, w, 0);
    r = K(fle_imp_GetUpdateRgn, w, rgn, 0); K(fle_imp_GetRgnBox, rgn, A(box)); printf("empty: %llu box %d %d %d %d\n", U(r), box[0], box[1], box[2], box[3]);
    K(fle_imp_InvalidateRect, w, A(ir), 0);
    r = K(fle_imp_GetUpdateRgn, w, rgn, 0); K(fle_imp_GetRgnBox, rgn, A(box)); printf("after InvalidateRect: %llu box %d %d %d %d\n", U(r), box[0], box[1], box[2], box[3]);
    printf("validate a part: %llu\n", U(K(fle_imp_ValidateRgn, w, part)));
    r = K(fle_imp_GetUpdateRgn, w, rgn, 0); printf("after the part: %llu\n", U(r));
    printf("validate the cover: %llu\n", U(K(fle_imp_ValidateRgn, w, cover)));
    r = K(fle_imp_GetUpdateRgn, w, rgn, 0); printf("after the cover: %llu\n", U(r));
    K(fle_imp_InvalidateRect, w, A(ir), 0);
    printf("validate NULL region: %llu\n", U(K(fle_imp_ValidateRgn, w, 0)));
    r = K(fle_imp_GetUpdateRgn, w, rgn, 0); printf("after NULL: %llu\n", U(r));
    printf("bad region: %llu\n", U(K(fle_imp_GetUpdateRgn, w, 0x1234, 0)));
"#,
    );
    assert_eq!(l[0], "empty: 1 box 0 0 0 0");
    assert_eq!(l[1], "after InvalidateRect: 2 box 10 20 50 60");
    assert_eq!(l[2], "validate a part: 1");
    assert_eq!(l[3], "after the part: 2", "a partial removal leaves the one rectangle as it is (S-updregion)");
    assert_eq!(l[4], "validate the cover: 1");
    assert_eq!(l[5], "after the cover: 1");
    assert_eq!(l[6], "validate NULL region: 1");
    assert_eq!(l[7], "after NULL: 1");
    assert_eq!(l[8], "bad region: 0", "ERROR (0): hrgn is not a region");
}

#[test]
fn set_bk_color_takes_a_palettergb_specifier_as_a_plain_rgb_value_and_returns_the_previous_colour() {
    // Learn SetBkColor: "the return value specifies the previous background color as a COLORREF value"; "If the function fails, the return value is CLR_INVALID". Learn PALETTERGB: "returns a palette-relative red, green,
    // blue (RGB) specifier consisting of 2 in the high-order byte and an RGB value in the three low-order bytes"; "If an output device does not support a system palette, the system uses the palette-relative RGB as though
    // it were a conventional RGB value returned by the RGB macro". This environment's display is not a palette device, so the specifier is its three low bytes (S-palettergb, choice: the page's "does not support a system
    // palette" is read as this display: SIZEPALETTE 0); the guest passes 0x02FF00FF. The colour is what a later BitBlt from a monochrome source uses (S-mono-convert, choice: the zero bits of a monochrome source take the
    // text colour, the one bits the background colour of the DC with colour). PALETTEINDEX (high byte 1) still stops.
    let l = lines(
        "fr11-palettergb",
        r#"
    uint64_t dib = MKDIB(2, 1, 32), dc = MKDC(), sdc = MKDC(), mono = K(fle_imp_CreateBitmap, 2, 1, 1, 1);
    uint8_t bits[2] = { 0x40, 0x00 };
    K(fle_imp_SelectObject, dc, dib);
    K(fle_imp_SelectObject, sdc, mono);
    K(fle_imp_SetBitmapBits, mono, 2, A(bits));
    printf("first SetBkColor returns the initial colour %llx\n", U((uint32_t)K(fle_imp_SetBkColor, dc, 0x02FF00FF)));
    printf("previous colour is the RGB value %llx\n", U((uint32_t)K(fle_imp_SetBkColor, dc, 0x00112233)));
    K(fle_imp_SetBkColor, dc, 0x02FF00FF);
    printf("the colour set last was kept without the specifier byte: previous is %llx\n", U((uint32_t)K(fle_imp_SetBkColor, dc, 0x02FF00FF)));
    K(fle_imp_SetTextColor, dc, 0x000000FF);
    FILLALL(dib, 2, 1, 0x77);
    printf("blit ret %llu\n", U(KN(fle_imp_BitBlt, 9, dc, 0, 0, 2, 1, sdc, 0, 0, 0x00CC0020)));
    printf("zero bit takes the text colour %06x, one bit takes the background colour %06x\n", PX(dib, 2, 0, 0), PX(dib, 2, 1, 0));
"#,
    );
    // bits 0x40 = 01000000: pixel 0 is 0, pixel 1 is 1. Text colour COLORREF 0x000000FF = R 0xFF -> PX 0xFF0000; background (PALETTERGB(0xFF,0x00,0xFF) as 0x02FF00FF read as 0x00FF00FF: COLORREF R 0xFF, G 0x00, B 0xFF)
    // -> PX 0xFF00FF
    assert_eq!(l[1], "previous colour is the RGB value ff00ff", "the previous colour is the specifier's three low bytes");
    assert_eq!(l[2], "the colour set last was kept without the specifier byte: previous is ff00ff", "the specifier byte 2 is not part of the stored colour (S-palettergb)");
    assert_eq!(l[4], "zero bit takes the text colour ff0000, one bit takes the background colour ff00ff");
}

#[test]
fn compare_string_w_skips_a_space_or_a_hyphen_that_both_strings_have_at_the_same_place() {
    // The guest compares "Test message processing" with "Test program expiration" (the common prefix "Test " holds a space) and two paths that share "image-line\" (a hyphen). Learn Handling Sorting in Your
    // Applications: the sorting functions "evaluate strings character by character"; the word sort puts punctuation and other non-alphanumerics before alphanumerics and treats the hyphen and the apostrophe differently
    // (so that "coop" and "co-op" stay together). The page says nothing about a unit that BOTH strings have at the same place: that such a unit does not decide the order, and the first different pair does, is the choice
    // S-cmp-equal-prefix (space) and S-cmp-equal-hyphen (hyphen). The first different pair here is "m" against "p": m < p, so CSTR_LESS_THAN (1). A control character in the common prefix (0x01) is answered by the Sorting Weight Table (decision 0284, B).
    let l = lines(
        "fr11-compare-equalprefix",
        r#"
    printf("space prefix %llu %llu\n", U(KN(fle_imp_CompareStringW, 6, 0x409, 0, S("Test message processing"), NEG(-1), S("Test program expiration"), NEG(-1))), U(KN(fle_imp_CompareStringW, 6, 0x409, 0, S("Test program expiration"), NEG(-1), S("Test message processing"), NEG(-1))));
    printf("hyphen prefix %llu %llu\n", U(KN(fle_imp_CompareStringW, 6, 0x409, 0, S("c:\\users\\user\\image-line\\dow"), NEG(-1), S("c:\\users\\user\\image-line\\fl s"), NEG(-1))), U(KN(fle_imp_CompareStringW, 6, 0x409, 0, S("c:\\users\\user\\image-line\\fl s"), NEG(-1), S("c:\\users\\user\\image-line\\dow"), NEG(-1))));
    printf("digits after a space %llu\n", U(KN(fle_imp_CompareStringW, 6, 0x409, 0, S("a 1"), NEG(-1), S("a 2"), NEG(-1))));
    printf("ignore case %llu\n", U(KN(fle_imp_CompareStringW, 6, 0x409, 1, S("Test Message"), NEG(-1), S("TEST PROGRAM"), NEG(-1))));
"#,
    );
    assert_eq!(l[0], "space prefix 1 3");
    assert_eq!(l[1], "hyphen prefix 1 3");
    assert_eq!(l[2], "digits after a space 1");
    assert_eq!(l[3], "ignore case 1");
    // decision 0284 (B): old: a control character (0x01) in the common prefix stopped by name ("before the first difference"). The Sorting Weight Table gives U+0001 a weight (Script Member 6, Alphabetic Weight 3, a
    // symbol of the same weight on both sides), it is in every build now, so the first different pair decides: "b" against "c" (14/9 against 14/10): CSTR_LESS_THAN, and the reverse.
    let l = lines(
        "fr11-compare-control",
        "static uint16_t a[] = { 'a', 1, 'b', 0 }, b[] = { 'a', 1, 'c', 0 }; printf(\"control prefix %llu %llu\\n\", U(KN(fle_imp_CompareStringW, 6, 0x409, 0, A(a), NEG(-1), A(b), NEG(-1))), U(KN(fle_imp_CompareStringW, 6, 0x409, 0, A(b), NEG(-1), A(a), NEG(-1))));",
    );
    assert_eq!(l[0], "control prefix 1 3");
}

#[test]
fn create_pen_makes_a_one_pixel_pen_for_width_zero_makes_wide_dashed_pens_solid_and_stops_outside_the_styles() {
    // Learn CreatePen: "If nWidth is zero, the pen is a single pixel wide, regardless of the current transformation"; "CreatePen returns a pen with the specified width but with the PS_SOLID style if you specify a width
    // greater than one for the following styles: PS_DASH, PS_DOT, PS_DASHDOT, PS_DASHDOTDOT"; the styles PS_SOLID 0, PS_DASH 1, PS_DOT 2, PS_DASHDOT 3, PS_DASHDOTDOT 4, PS_NULL 5, PS_INSIDEFRAME 6 (the page's list; the
    // numbers are the SDK's: UNOFFICIAL); the return value is a handle to the pen, NULL on failure. The pen colour is a COLORREF. A pen of width above 1 draws nothing here yet: its pixels are not on the pages (S-pen-width,
    // choice: it stops by name at the first drawing call); a dashed pen of width 1 stops the same way (S-pen-dashes). So the pen that CreatePen made is observed by what LineTo does with it: a solid pen of width 0 and a
    // PS_DASH pen of width 3 (which became PS_SOLID of width 3) are told apart by the stop of the wide one; a style above 6 stops in CreatePen itself.
    let l = lines(
        "fr11-createpen",
        r#"
    uint64_t dib = MKDIB(4, 1, 32), dc = MKDC(), p0 = K(fle_imp_CreatePen, 0, 0, 0x00000011), pinside = K(fle_imp_CreatePen, 6, 0, 0x00000022), pnull = K(fle_imp_CreatePen, 5, 0, 0x00000033);
    uint64_t h = K(fle_imp_CreatePen, 0, 1, 0x00000044);
    printf("handles are pens %d %d %d %d\n", p0 != 0, pinside != 0, pnull != 0, h != 0);
    printf("all different %d\n", p0 != pinside && pinside != pnull && pnull != h && p0 != pnull);
    K(fle_imp_SelectObject, dc, dib);
    FILLALL(dib, 4, 1, 0);
    K(fle_imp_SelectObject, dc, p0); K(fle_imp_MoveToEx, dc, 0, 0, 0); K(fle_imp_LineTo, dc, 1, 0);
    K(fle_imp_SelectObject, dc, pinside); K(fle_imp_MoveToEx, dc, 1, 0, 0); K(fle_imp_LineTo, dc, 2, 0);
    K(fle_imp_SelectObject, dc, pnull); K(fle_imp_MoveToEx, dc, 2, 0, 0); K(fle_imp_LineTo, dc, 3, 0);
    printf("width 0 pen %06x, inside-frame pen %06x, null pen %06x\n", PX(dib, 4, 0, 0), PX(dib, 4, 1, 0), PX(dib, 4, 2, 0));
"#,
    );
    // COLORREF 0x00000011 is red 0x11 -> 0x110000; 0x22 -> 0x220000; the null pen draws nothing
    assert_eq!(l[0], "handles are pens 1 1 1 1");
    assert_eq!(l[1], "all different 1");
    assert_eq!(l[2], "width 0 pen 110000, inside-frame pen 220000, null pen 000000");
    aborts("fr11-createpen-wide", "uint64_t dib = MKDIB(4, 1, 32), dc = MKDC(), p = K(fle_imp_CreatePen, 0, 3, 0x11); K(fle_imp_SelectObject, dc, dib); K(fle_imp_SelectObject, dc, p); K(fle_imp_MoveToEx, dc, 0, 0, 0); K(fle_imp_LineTo, dc, 2, 0);", "LineTo", "wider than one pixel");
    aborts("fr11-createpen-dash-wide", "uint64_t dib = MKDIB(4, 1, 32), dc = MKDC(), p = K(fle_imp_CreatePen, 1, 3, 0x11); K(fle_imp_SelectObject, dc, dib); K(fle_imp_SelectObject, dc, p); K(fle_imp_MoveToEx, dc, 0, 0, 0); K(fle_imp_LineTo, dc, 2, 0);", "LineTo", "wider than one pixel");
    aborts("fr11-createpen-dash", "uint64_t dib = MKDIB(4, 1, 32), dc = MKDC(), p = K(fle_imp_CreatePen, 1, 1, 0x11); K(fle_imp_SelectObject, dc, dib); K(fle_imp_SelectObject, dc, p); K(fle_imp_MoveToEx, dc, 0, 0, 0); K(fle_imp_LineTo, dc, 2, 0);", "LineTo", "dashed or dotted");
    aborts("fr11-createpen-style7", "K(fle_imp_CreatePen, 7, 0, 0);", "CreatePen", "iStyle is 7");
}

// The 244 messages of the Learn page "System Error Codes (0-499)" (https://learn.microsoft.com/en-us/windows/win32/debug/system-error-codes--0-499-, fetched 2026-11-01 by decision 0127's work, SHA-256 of the HTML in
// logs/learn-index.tsv: 106e611e44c8...): each entry of the page is `ERROR_NAME`, `number (0xhex)` and the text; these are the number and the text of every one of the 244 entries, taken from the page and not from
// fle_errmsg_table.h. (Decision 0126's generator found 241 of the 244: it did not recognise the entries WAIT_TIMEOUT 258 (no ERROR_ prefix) and ERROR_ITERATED_DATA_EXCEEDS_64k 194 / ERROR_AUTODATASEG_EXCEEDS_64k 199 (a
// lower-case k), so those 3 numbers were missing and the 3 texts before them held the next entry's name, number and text: 193, 198, 255. Decision 0127 fixed the table; this test found it.) %1, %2, %3 stay as they are (FORMAT_MESSAGE_IGNORE_INSERTS: "Insert sequences in the message definition such as %1 are to be ignored and passed through to the output buffer unchanged").
const ERRMSG_PAGE: [(u32, &str); 244] = [
    (0, "The operation completed successfully."),
    (1, "Incorrect function."),
    (2, "The system cannot find the file specified."),
    (3, "The system cannot find the path specified."),
    (4, "The system cannot open the file."),
    (5, "Access is denied."),
    (6, "The handle is invalid."),
    (7, "The storage control blocks were destroyed."),
    (8, "Not enough memory resources are available to process this command."),
    (9, "The storage control block address is invalid."),
    (10, "The environment is incorrect."),
    (11, "An attempt was made to load a program with an incorrect format."),
    (12, "The access code is invalid."),
    (13, "The data is invalid."),
    (14, "Not enough storage is available to complete this operation."),
    (15, "The system cannot find the drive specified."),
    (16, "The directory cannot be removed."),
    (17, "The system cannot move the file to a different disk drive."),
    (18, "There are no more files."),
    (19, "The media is write protected."),
    (20, "The system cannot find the device specified."),
    (21, "The device is not ready."),
    (22, "The device does not recognize the command."),
    (23, "Data error (cyclic redundancy check)."),
    (24, "The program issued a command but the command length is incorrect."),
    (25, "The drive cannot locate a specific area or track on the disk."),
    (26, "The specified disk or diskette cannot be accessed."),
    (27, "The drive cannot find the sector requested."),
    (28, "The printer is out of paper."),
    (29, "The system cannot write to the specified device."),
    (30, "The system cannot read from the specified device."),
    (31, "A device attached to the system is not functioning."),
    (32, "The process cannot access the file because it is being used by another process."),
    (33, "The process cannot access the file because another process has locked a portion of the file."),
    (34, "The wrong diskette is in the drive. Insert %2 (Volume Serial Number: %3) into drive %1."),
    (36, "Too many files opened for sharing."),
    (38, "Reached the end of the file."),
    (39, "The disk is full."),
    (50, "The request is not supported."),
    (51, "Windows cannot find the network path. Verify that the network path is correct and the destination computer is not busy or turned off. If Windows still cannot find the network path, contact your network administrator."),
    (52, "You were not connected because a duplicate name exists on the network. If joining a domain, go to System in Control Panel to change the computer name and try again. If joining a workgroup, choose another workgroup name."),
    (53, "The network path was not found."),
    (54, "The network is busy."),
    (55, "The specified network resource or device is no longer available."),
    (56, "The network BIOS command limit has been reached."),
    (57, "A network adapter hardware error occurred."),
    (58, "The specified server cannot perform the requested operation."),
    (59, "An unexpected network error occurred."),
    (60, "The remote adapter is not compatible."),
    (61, "The printer queue is full."),
    (62, "Space to store the file waiting to be printed is not available on the server."),
    (63, "Your file waiting to be printed was deleted."),
    (64, "The specified network name is no longer available."),
    (65, "Network access is denied."),
    (66, "The network resource type is not correct."),
    (67, "The network name cannot be found."),
    (68, "The name limit for the local computer network adapter card was exceeded."),
    (69, "The network BIOS session limit was exceeded."),
    (70, "The remote server has been paused or is in the process of being started."),
    (71, "No more connections can be made to this remote computer at this time because there are already as many connections as the computer can accept."),
    (72, "The specified printer or disk device has been paused."),
    (80, "The file exists."),
    (82, "The directory or file cannot be created."),
    (83, "Fail on INT 24."),
    (84, "Storage to process this request is not available."),
    (85, "The local device name is already in use."),
    (86, "The specified network password is not correct."),
    (87, "The parameter is incorrect."),
    (88, "A write fault occurred on the network."),
    (89, "The system cannot start another process at this time."),
    (100, "Cannot create another system semaphore."),
    (101, "The exclusive semaphore is owned by another process."),
    (102, "The semaphore is set and cannot be closed."),
    (103, "The semaphore cannot be set again."),
    (104, "Cannot request exclusive semaphores at interrupt time."),
    (105, "The previous ownership of this semaphore has ended."),
    (106, "Insert the diskette for drive %1."),
    (107, "The program stopped because an alternate diskette was not inserted."),
    (108, "The disk is in use or locked by another process."),
    (109, "The pipe has been ended."),
    (110, "The system cannot open the device or file specified."),
    (111, "The file name is too long."),
    (112, "There is not enough space on the disk."),
    (113, "No more internal file identifiers available."),
    (114, "The target internal file identifier is incorrect."),
    (117, "The IOCTL call made by the application program is not correct."),
    (118, "The verify-on-write switch parameter value is not correct."),
    (119, "The system does not support the command requested."),
    (120, "This function is not supported on this system."),
    (121, "The semaphore timeout period has expired."),
    (122, "The data area passed to a system call is too small."),
    (123, "The filename, directory name, or volume label syntax is incorrect."),
    (124, "The system call level is not correct."),
    (125, "The disk has no volume label."),
    (126, "The specified module could not be found."),
    (127, "The specified procedure could not be found."),
    (128, "There are no child processes to wait for."),
    (129, "The %1 application cannot be run in Win32 mode."),
    (130, "Attempt to use a file handle to an open disk partition for an operation other than raw disk I/O."),
    (131, "An attempt was made to move the file pointer before the beginning of the file."),
    (132, "The file pointer cannot be set on the specified device or file."),
    (133, "A JOIN or SUBST command cannot be used for a drive that contains previously joined drives."),
    (134, "An attempt was made to use a JOIN or SUBST command on a drive that has already been joined."),
    (135, "An attempt was made to use a JOIN or SUBST command on a drive that has already been substituted."),
    (136, "The system tried to delete the JOIN of a drive that is not joined."),
    (137, "The system tried to delete the substitution of a drive that is not substituted."),
    (138, "The system tried to join a drive to a directory on a joined drive."),
    (139, "The system tried to substitute a drive to a directory on a substituted drive."),
    (140, "The system tried to join a drive to a directory on a substituted drive."),
    (141, "The system tried to SUBST a drive to a directory on a joined drive."),
    (142, "The system cannot perform a JOIN or SUBST at this time."),
    (143, "The system cannot join or substitute a drive to or for a directory on the same drive."),
    (144, "The directory is not a subdirectory of the root directory."),
    (145, "The directory is not empty."),
    (146, "The path specified is being used in a substitute."),
    (147, "Not enough resources are available to process this command."),
    (148, "The path specified cannot be used at this time."),
    (149, "An attempt was made to join or substitute a drive for which a directory on the drive is the target of a previous substitute."),
    (150, "System trace information was not specified in your CONFIG.SYS file, or tracing is disallowed."),
    (151, "The number of specified semaphore events for DosMuxSemWait is not correct."),
    (152, "DosMuxSemWait did not execute; too many semaphores are already set."),
    (153, "The DosMuxSemWait list is not correct."),
    (154, "The volume label you entered exceeds the label character limit of the target file system."),
    (155, "Cannot create another thread."),
    (156, "The recipient process has refused the signal."),
    (157, "The segment is already discarded and cannot be locked."),
    (158, "The segment is already unlocked."),
    (159, "The address for the thread ID is not correct."),
    (160, "One or more arguments are not correct."),
    (161, "The specified path is invalid."),
    (162, "A signal is already pending."),
    (164, "No more threads can be created in the system."),
    (167, "Unable to lock a region of a file."),
    (170, "The requested resource is in use."),
    (171, "Device's command support detection is in progress."),
    (173, "A lock request was not outstanding for the supplied cancel region."),
    (174, "The file system does not support atomic changes to the lock type."),
    (180, "The system detected a segment number that was not correct."),
    (182, "The operating system cannot run %1."),
    (183, "Cannot create a file when that file already exists."),
    (186, "The flag passed is not correct."),
    (187, "The specified system semaphore name was not found."),
    (188, "The operating system cannot run %1."),
    (189, "The operating system cannot run %1."),
    (190, "The operating system cannot run %1."),
    (191, "Cannot run %1 in Win32 mode."),
    (192, "The operating system cannot run %1."),
    (193, "%1 is not a valid Win32 application."),
    (194, "The operating system cannot run %1."),
    (195, "The operating system cannot run %1."),
    (196, "The operating system cannot run this application program."),
    (197, "The operating system is not presently configured to run this application."),
    (198, "The operating system cannot run %1."),
    (199, "The operating system cannot run this application program."),
    (200, "The code segment cannot be greater than or equal to 64K."),
    (201, "The operating system cannot run %1."),
    (202, "The operating system cannot run %1."),
    (203, "The system could not find the environment option that was entered."),
    (205, "No process in the command subtree has a signal handler."),
    (206, "The filename or extension is too long."),
    (207, "The ring 2 stack is in use."),
    (208, "The global filename characters, * or ?, are entered incorrectly or too many global filename characters are specified."),
    (209, "The signal being posted is not correct."),
    (210, "The signal handler cannot be set."),
    (212, "The segment is locked and cannot be reallocated."),
    (214, "Too many dynamic-link modules are attached to this program or dynamic-link module."),
    (215, "Cannot nest calls to LoadModule."),
    (216, "This version of %1 is not compatible with the version of Windows you're running. Check your computer's system information and then contact the software publisher."),
    (217, "The image file %1 is signed, unable to modify."),
    (218, "The image file %1 is strong signed, unable to modify."),
    (220, "This file is checked out or locked for editing by another user."),
    (221, "The file must be checked out before saving changes."),
    (222, "The file type being saved or retrieved has been blocked."),
    (223, "The file size exceeds the limit allowed and cannot be saved."),
    (224, "Access Denied. Before opening files in this location, you must first add the web site to your trusted sites list, browse to the web site, and select the option to login automatically."),
    (225, "Operation did not complete successfully because the file contains a virus or potentially unwanted software."),
    (226, "This file contains a virus or potentially unwanted software and cannot be opened. Due to the nature of this virus or potentially unwanted software, the file has been removed from this location."),
    (229, "The pipe is local."),
    (230, "The pipe state is invalid."),
    (231, "All pipe instances are busy."),
    (232, "The pipe is being closed."),
    (233, "No process is on the other end of the pipe."),
    (234, "More data is available."),
    (240, "The session was canceled."),
    (254, "The specified extended attribute name was invalid."),
    (255, "The extended attributes are inconsistent."),
    (258, "The wait operation timed out."),
    (259, "No more data is available."),
    (266, "The copy functions cannot be used."),
    (267, "The directory name is invalid."),
    (275, "The extended attributes did not fit in the buffer."),
    (276, "The extended attribute file on the mounted file system is corrupt."),
    (277, "The extended attribute table file is full."),
    (278, "The specified extended attribute handle is invalid."),
    (282, "The mounted file system does not support extended attributes."),
    (288, "Attempt to release mutex not owned by caller."),
    (298, "Too many posts were made to a semaphore."),
    (299, "Only part of a ReadProcessMemory or WriteProcessMemory request was completed."),
    (300, "The oplock request is denied."),
    (301, "An invalid oplock acknowledgment was received by the system."),
    (302, "The volume is too fragmented to complete this operation."),
    (303, "The file cannot be opened because it is in the process of being deleted."),
    (304, "Short name settings may not be changed on this volume due to the global registry setting."),
    (305, "Short names are not enabled on this volume."),
    (306, "The security stream for the given volume is in an inconsistent state. Please run CHKDSK on the volume."),
    (307, "A requested file lock operation cannot be processed due to an invalid byte range."),
    (308, "The subsystem needed to support the image type is not present."),
    (309, "The specified file already has a notification GUID associated with it."),
    (310, "An invalid exception handler routine has been detected."),
    (311, "Duplicate privileges were specified for the token."),
    (312, "No ranges for the specified operation were able to be processed."),
    (313, "Operation is not allowed on a file system internal file."),
    (314, "The physical resources of this disk have been exhausted."),
    (315, "The token representing the data is invalid."),
    (316, "The device does not support the command feature."),
    (317, "The system cannot find message text for message number 0x%1 in the message file for %2."),
    (318, "The scope specified was not found."),
    (319, "The Central Access Policy specified is not defined on the target machine."),
    (320, "The Central Access Policy obtained from Active Directory is invalid."),
    (321, "The device is unreachable."),
    (322, "The target device has insufficient resources to complete the operation."),
    (323, "A data integrity checksum error occurred. Data in the file stream is corrupt."),
    (324, "An attempt was made to modify both a KERNEL and normal Extended Attribute (EA) in the same operation."),
    (326, "Device does not support file-level TRIM."),
    (327, "The command specified a data offset that does not align to the device's granularity/alignment."),
    (328, "The command specified an invalid field in its parameter list."),
    (329, "An operation is currently in progress with the device."),
    (330, "An attempt was made to send down the command via an invalid path to the target device."),
    (331, "The command specified a number of descriptors that exceeded the maximum supported by the device."),
    (332, "Scrub is disabled on the specified file."),
    (333, "The storage device does not provide redundancy."),
    (334, "An operation is not supported on a resident file."),
    (335, "An operation is not supported on a compressed file."),
    (336, "An operation is not supported on a directory."),
    (337, "The specified copy of the requested data could not be read."),
    (350, "No action was taken as a system reboot is required."),
    (351, "The shutdown operation failed."),
    (352, "The restart operation failed."),
    (353, "The maximum number of sessions has been reached."),
    (400, "The thread is already in background processing mode."),
    (401, "The thread is not in background processing mode."),
    (402, "The process is already in background processing mode."),
    (403, "The process is not in background processing mode."),
    (487, "Attempt to access invalid address."),
];

#[test]
fn format_message_w_gives_the_text_of_every_one_of_the_244_entries_of_the_system_error_codes_page() {
    let mut body = String::from("    static uint16_t buf[400];\n    char out[800];\n    uint64_t r;\n");
    for (n, _) in ERRMSG_PAGE.iter() {
        body += &format!("    memset(buf, 0, sizeof buf); r = KN(fle_imp_FormatMessageW, 7, 0x1200, 0, {n}, 0, A(buf), 400, 0); wtoa(A(buf), out, sizeof out); printf(\"%u|%llu|%s\\n\", {n}u, U(r), out);\n");
    }
    let l = lines("fr11-errmsg-page", &body);
    assert_eq!(l.len(), ERRMSG_PAGE.len());
    for (k, (n, t)) in ERRMSG_PAGE.iter().enumerate() {
        assert_eq!(l[k], format!("{n}|{}|{t}", t.len()), "message number {n}");
    }
    // a number that the page does not list: 0 and ERROR_MR_MID_NOT_FOUND (317); the page lists 0..=487 with gaps: 2000 is outside
    let l2 = lines("fr11-errmsg-gap", "static uint16_t buf[100]; SETLE(0); printf(\"%llu %u\\n\", U(KN(fle_imp_FormatMessageW, 7, 0x1200, 0, 2000, 0, A(buf), 100, 0)), LE());");
    assert_eq!(l2, ["0 317"]);
}

#[test]
fn an_empty_file_name_fails_in_get_full_path_name_w_and_get_file_attributes_ex_w_with_the_chosen_codes() {
    // Learn GetFullPathNameW: "If the function fails for any other reason, the return value is zero. To get extended error information, call GetLastError"; the page gives no rule for an empty lpFileName. The runtime
    // answers 0 with ERROR_INVALID_NAME 123 ("The filename, directory name, or volume label syntax is incorrect": System Error Codes) and leaves the buffer alone (S-fullpath-empty, choice: the guest's Delphi
    // ExpandFileName("") reads the 0 as an empty expanded name). Learn GetFileAttributesExW: "If the function fails, the return value is zero (0). To get extended error information, call GetLastError"; the page does not say what
    // an empty name gives: 0 with ERROR_PATH_NOT_FOUND 3 ("The system cannot find the path specified.") like GetFileAttributesW of decision 0123 (S-empty-attrs-ex, choice); the output structure is left as it is. A non-empty
    // name is not an error: GetFullPathNameW("C:\x") gives the 4 characters of "C:\x" (Learn: "The return value is the length of the string copied to lpBuffer, in TCHARs, not including the terminating null character").
    let l = lines(
        "fr11-emptyname",
        r#"
    static uint16_t buf[300];
    static uint8_t data[36];
    uint64_t r;
    char out[300];
    memset(buf, 0x55, sizeof buf); memset(data, 0x66, sizeof data);
    SETLE(0);
    r = K(fle_imp_GetFullPathNameW, S(""), 300, A(buf), 0);
    printf("fullpath empty %llu le %u buffer untouched %d\n", U(r), LE(), buf[0] == 0x5555);
    SETLE(0);
    r = KN(fle_imp_GetFileAttributesExW, 3, S(""), 0, A(data));
    printf("attrs-ex empty %llu le %u data untouched %d\n", U(r), LE(), data[0] == 0x66 && data[35] == 0x66);
    SETLE(0);
    r = K(fle_imp_GetFullPathNameW, S("C:\\x"), 300, A(buf), 0);
    wtoa(A(buf), out, sizeof out);
    printf("fullpath C:\\x %llu [%s]\n", U(r), out);
"#,
    );
    assert_eq!(l[0], "fullpath empty 0 le 123 buffer untouched 1");
    assert_eq!(l[1], "attrs-ex empty 0 le 3 data untouched 1");
    assert_eq!(l[2], "fullpath C:\\x 4 [C:\\x]");
}
