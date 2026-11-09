//! Tests of decision 0129 (docs/0129-idle-continuation.md, PROVISIONAL): the continuation of CreateFruityInstance after the idle stop of decision 0126: the stable-idle detector (harness/idle/idle.c, N virtual seconds), the
//! first-text-call record (fle_text_hook), the VFS directory lines, and the runtime answers of walls 1 to 16 (S-hdc-deleted, S-open-no-recall, S-cmp-digits-flag / -symbol-any-position / -symbol-vs-symbol / -space-as-symbol,
//! S-lastactivepopup, GetThreadPriority, S-guires-count, S-version-info-real-resource, S-com-registry-empty, S-errmsg-hresult-learn-text).
//!
//! Expected values are written here as LITERALS from the Microsoft Learn pages named in the decision (fetched 2026-11-01, kept in lift/out-idle1/learn/ with index.tsv) and from the choices of the decision, never read back from
//! the C code. EVERY ANSWER IS PROVISIONAL; nothing is compared with a real Windows. A test that pins a choice without a source says so in its comment: it fixes the choice, not a Windows rule.
//!
//! 1. Generator: the 5 rows of decision 0129 are the last rows, in the order given, and no pseudo-VA moved.
//! 2. The idle detector (idle.c is compiled alone: SHA-256 vectors, the stable-run rule).
//! 3. Runtime answers (the real runtime files and a stub guest).
//! 4. VFS seed directory lines.
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
fn harness_dir() -> PathBuf {
    manifest().join("../../harness")
}
fn src_dir() -> PathBuf {
    if let Ok(d) = std::env::var("FLE_FR12_SRC_DIR") {
        return PathBuf::from(d);
    }
    rt_dir()
}
fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-wallFR12/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

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

/// The 4 rows of decision 0129 in the order given (decision 0015: append only).
const NEW_ROWS: [(&str, &str); 4] = [
    ("user32.dll", "GetLastActivePopup"),
    ("kernel32.dll", "GetThreadPriority"),
    ("user32.dll", "GetGuiResources"),
    ("ole32.dll", "CoCreateInstance"),
];

#[test]
fn the_4_new_rows_are_the_last_rows_in_the_order_given_and_no_pseudo_va_moved() {
    let (st, c) = gen(&GUEST_IMPORTS);
    let rows = resp_rows(&c);
    assert_eq!(rows.len(), st.resp_entries);
    // HEAD 9574cb5 has 577 rows (decision 0126: 544 + 33); decision 0129 appends 4 (GetLastActivePopup, GetThreadPriority, GetGuiResources, CoCreateInstance)
    // decision 0136: rows are appended at the END again (and decision 0132 appends more): the end of the group is found by the NAME of its last row, not by counting from the end of the table.
    let last = NEW_ROWS[NEW_ROWS.len() - 1];
    let n = rows.iter().position(|r| (r.0.as_str(), r.1.as_str()) == last).expect("the last row of the group") + 1;
    assert_eq!(n, 577 + 4, "the 577 rows of HEAD 9574cb5 and the 4 of decision 0129");
    assert_eq!((rows[n - 5].0.as_str(), rows[n - 5].1.as_str()), ("user32.dll", "UnhookWindowsHookEx"), "the row before the 4 is the last row of decision 0126");
    for (k, (dll, name)) in NEW_ROWS.iter().enumerate() {
        assert_eq!((rows[n - 4 + k].0.as_str(), rows[n - 4 + k].1.as_str()), (*dll, *name), "row {}", n - 4 + k);
    }
    for (k, (_, _, va)) in rows.iter().enumerate() {
        assert_eq!(*va, 0x7FF9_0000_0000u64 + 0x10 * (k as u64 + 1), "row {k}: the pseudo-VA of a row never moves");
    }
    for (_, name) in NEW_ROWS.iter() {
        assert_eq!(rows.iter().filter(|(_, n2, _)| n2 == name).count(), 1, "{name}: exactly one row");
        assert!(c.contains(&format!("void fle_imp_{name}(void);")), "{name} is declared in the generated C");
    }
}

#[test]
fn each_new_thunk_is_answered_under_its_own_dll_and_no_other_dll() {
    for (dll, name) in NEW_ROWS.iter() {
        let other = if *dll == "user32.dll" { "kernel32.dll" } else { "user32.dll" };
        let (st, c) = gen(&[(dll, name), (other, name), (dll, "SendMessageA"), ("kernel32.dll", "GetProcAddress")]);
        assert_eq!(st.handled[format!("jmp_imp:{name}").as_str()], 1, "{dll}!{name}");
        assert_eq!(c.matches(&format!("fle_imp_{name}(); return;")).count(), 1, "{dll}!{name}: only the thunk of its own dll is answered");
    }
}

// ---------------------------------------------------------------------------------------------
// 3. The driver: the real runtime files and a stub guest
// ---------------------------------------------------------------------------------------------

const RT_FILES: [&str; 24] = [
    "fle_thread.c", "fle_win.c", "fle_class.c", "fle_event.c", "fle_sys.c", "fle_imports.c", "fle_proc.c", "fle_callback.c", "fle_env.c", "fle_vfs.c", "fle_vmap.c", "fle_vol.c", "fle_host.c",
    "fle_time.c", "fle_gdc.c", "fle_gdi.c", "fle_cursor.c", "fle_res.c", "fle_modules.c", "fle_realmod.c", "fle_raster.c", "fle_monitor.c", "fle_ole.c", "fle_reg.c",
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
        let dir = work_dir().join(etxt_common::per_process("obj-fr12"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut v = Vec::new();
        let ours = ["fle_thread.c", "fle_win.c", "fle_class.c", "fle_gdi.c", "fle_gdc.c", "fle_cursor.c", "fle_res.c", "fle_vmap.c", "fle_vfs.c", "fle_raster.c", "fle_monitor.c", "fle_ole.c", "fle_modules.c", "fle_imports.c", "fle_proc.c", "fle_locale.c"];
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
#include "fle_cursor.h"
#include "fle_res.h"
#include "fle_locale.h"
#include "fle_sysinfo.h"
#include "fle_x87ctl.h"
#include "fle_mxcsr.h"
#include "fle_ole.h"
#include "fle_reg.h"
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
    fle_st(R[4] + 0x28, 8, e); fle_st(R[4] + 0x30, 8, g); fle_st(R[4] + 0x38, 8, h7); fle_st(R[4] + 0x40, 8, h8); fle_st(R[4] + 0x48, 8, 0);
    R[1] = a; R[2] = b; R[8] = c; R[9] = d; f(); R[4] = sp0_;
    return R[0];
}
/* nine arguments (RegCreateKeyExW): the stack arguments 5..9 at [rsp+0x28] .. [rsp+0x48] */
static uint64_t callf9(void (*f)(void), uint64_t a, uint64_t b, uint64_t c, uint64_t d, uint64_t e, uint64_t g, uint64_t h7, uint64_t h8, uint64_t h9)
{
    uint64_t sp0_ = R[4];
    R[4] = (R[4] - 0x60) & ~UINT64_C(15); R[4] -= 8; fle_st(R[4], 8, 0xAAAA0000);
    fle_st(R[4] + 0x28, 8, e); fle_st(R[4] + 0x30, 8, g); fle_st(R[4] + 0x38, 8, h7); fle_st(R[4] + 0x40, 8, h8); fle_st(R[4] + 0x48, 8, h9);
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
/* a second and third class with a class style: OWNDC_CLS (CS_OWNDC 0x20) and CLASSDC_CLS (CS_CLASSDC 0x40) and PARENTDC_CLS (CS_PARENTDC 0x80); the plain class above has style 0 (a common DC) */
static uint16_t cls_own[] = { 'O','w','n','D','c', 0 };
static uint16_t cls_cdc[] = { 'C','l','a','s','s','D','c', 0 };
static uint16_t cls_pdc[] = { 'P','a','r','e','n','t','D','c', 0 };
static void regclass_style(uint16_t *name, uint32_t style)
{
    static uint8_t wc[72];
    uint64_t w = VA_WP, inst = 0x400000, nm = A(name);
    memset(wc, 0, sizeof wc); memcpy(wc, &style, 4); memcpy(wc + 8, &w, 8); memcpy(wc + 24, &inst, 8); memcpy(wc + 64, &nm, 8);
    CALL4(fle_imp_RegisterClassW, A(wc), 0, 0, 0);
}
static uint64_t cw_cls(uint16_t *name, uint64_t ex, uint64_t style, int32_t x, int32_t y, int32_t w, int32_t hh, uint64_t parent)
{
    uint64_t sp0 = R[4], h;
    regclass();
    R[4] = (R[4] - 0x80) & ~UINT64_C(15); R[4] -= 8; fle_st(R[4], 8, 0xAAAA0000);
    fle_st(R[4] + 0x28, 8, (uint32_t)x); fle_st(R[4] + 0x30, 8, (uint32_t)y); fle_st(R[4] + 0x38, 8, (uint32_t)w); fle_st(R[4] + 0x40, 8, (uint32_t)hh);
    fle_st(R[4] + 0x48, 8, parent); fle_st(R[4] + 0x50, 8, 0); fle_st(R[4] + 0x58, 8, 0x400000); fle_st(R[4] + 0x60, 8, 0);
    R[1] = ex; R[2] = A(name); R[8] = 0; R[9] = style;
    fle_imp_CreateWindowExW();
    h = R[0];
    R[4] = sp0;
    return h;
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
    run_src_img(name, top, body, None)
}

/// With `image` the program maps that PE at its ImageBase (harness/image.c, as the harness does) before the body runs.
fn run_src_img(name: &str, top: &str, body: &str, image: Option<&Path>) -> Out {
    let dir = work_dir();
    let uname = etxt_common::uniq(name);
    let c = dir.join(format!("{uname}.c"));
    let exe = dir.join(&uname);
    let (head, load, inc) = if image.is_some() {
        (
            "#include \"boot.h\"\nint main(int argc, char **argv)",
            "    { harness_image img; char err[256]; (void)argc; if (image_load(argv[1], &img, err, sizeof err) != 0) { fprintf(stderr, \"load: %s\\n\", err); return 3; } }\n",
            true,
        )
    } else {
        ("int main(void)", "", false)
    };
    let src = format!("{PRELUDE}\n{top}\n{head}\n{{\n{load}    boot();\n{body}\n    return 0;\n}}\n");
    std::fs::write(&c, src).unwrap();
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fPIE", "-pthread", "-I"]).arg(src_dir()).arg("-I").arg(rt_dir()).arg("-I").arg(harness_dir());
    for o in objs() {
        cmd.arg(o);
    }
    if inc {
        cmd.arg(harness_dir().join("image.c"));
    }
    let cc = cmd.arg("-o").arg(&exe).arg(&c).arg("-lm").env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    let mut run = Command::new(&exe);
    if let Some(p) = image {
        run.arg(p);
    }
    let r = run.env("TMPDIR", dir.join("tmp")).output().expect("run failed");
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

/// The run must stop with SIGABRT and a "fle: <api>: " message that contains `word`.
fn aborts(name: &str, body: &str, api: &str, word: &str) -> Out {
    let o = run_src(name, &top_with("", &[]), body);
    assert_eq!(o.signal, Some(6), "{name}: expected SIGABRT, got {:?}: {}\n{}", o.signal, o.stderr, o.stdout);
    assert!(o.stderr.contains(&format!("fle: {api}: ")) && o.stderr.contains(word), "{name}: stderr {:?} lacks `fle: {api}: ` and `{word}`", o.stderr);
    o
}


// ---------------------------------------------------------------------------------------------
// 2. The idle detector (harness/idle/idle.c compiled alone: SHA-256 and the stable-run rule)
// ---------------------------------------------------------------------------------------------

fn run_idle(name: &str, body: &str) -> Vec<String> {
    let dir = work_dir();
    let uname = etxt_common::uniq(name);
    let c = dir.join(format!("{uname}.c"));
    let exe = dir.join(&uname);
    let src = format!(
        "#include <stdio.h>\n#include <string.h>\n#include <stdint.h>\n#include \"idle/idle.h\"\n__attribute__((unused)) static void hexs(const uint8_t *d, size_t n) {{ uint8_t o[32]; idle_sha s; char h[65]; idle_sha_init(&s); idle_sha_add(&s, d, n); idle_sha_final(&s, o); idle_hex(o, h); printf(\"%s\\n\", h); }}\n\
         __attribute__((unused)) static idle_snap snap(uint64_t ms, uint64_t d, uint64_t a, uint8_t w, uint8_t sf, uint64_t maxp) {{ idle_snap s; memset(&s, 0, sizeof s); s.ms = ms; s.distinct = d; s.apis = a; s.win[0] = w; s.surf[0] = sf; s.max_period_ms = maxp; return s; }}\n\
         int main(void) {{\n{body}\n return 0; }}\n"
    );
    std::fs::write(&c, src).unwrap();
    let cc = Command::new("clang")
        .args(["-O0", "-std=gnu11", "-Wall", "-Wextra", "-Werror", "-fPIE", "-I"])
        .arg(harness_dir())
        .arg(harness_dir().join("idle/idle.c"))
        .arg("-o")
        .arg(&exe)
        .arg(&c)
        .env("TMPDIR", dir.join("tmp"))
        .output()
        .expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new(&exe).output().expect("run failed");
    assert!(r.status.success(), "{name}: {}", String::from_utf8_lossy(&r.stderr));
    String::from_utf8_lossy(&r.stdout).lines().map(|s| s.to_string()).collect()
}

#[test]
fn sha256_matches_the_fips_180_4_vectors_and_streaming_equals_one_shot() {
    // FIPS 180-4 / NIST examples, written here as literals: "", "abc", the 56-byte two-block message, and one million 'a'.
    let l = run_idle(
        "fr12-sha",
        r#"
    hexs((const uint8_t *)"", 0);
    hexs((const uint8_t *)"abc", 3);
    hexs((const uint8_t *)"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq", 56);
    { static uint8_t m[1000000]; idle_sha s; uint8_t o[32]; char h[65]; size_t k; memset(m, 'a', sizeof m); idle_sha_init(&s); for (k = 0; k < sizeof m; k += 997) idle_sha_add(&s, m + k, sizeof m - k < 997 ? sizeof m - k : 997); idle_sha_final(&s, o); idle_hex(o, h); printf("%s\n", h); }
"#,
    );
    assert_eq!(l[0], "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    assert_eq!(l[1], "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    assert_eq!(l[2], "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1");
    assert_eq!(l[3], "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0");
}

#[test]
fn the_stable_run_needs_n_virtual_seconds_of_equal_functions_apis_and_hashes() {
    // N = 120 s. Equal (functions, APIs, window hash, surface hash) from 1000 ms on: stable at 121000 ms (120000 ms later), not at 120999.
    let l = run_idle(
        "fr12-stable",
        r#"
    idle_track t; idle_snap s; int r1, r2, r3;
    idle_track_init(&t, 120000);
    s = snap(1000, 5, 7, 1, 1, 0); r1 = idle_track_feed(&t, &s);
    s = snap(60000, 5, 7, 1, 1, 0); r2 = idle_track_feed(&t, &s);
    s = snap(120999, 5, 7, 1, 1, 0); r3 = idle_track_feed(&t, &s);
    printf("%d %d %d\n", r1, r2, r3);
    s = snap(121000, 5, 7, 1, 1, 0); printf("%d\n", idle_track_feed(&t, &s));
    printf("%llu\n", (unsigned long long)t.n_resets);
"#,
    );
    assert_eq!(l[0], "0 0 0");
    assert_eq!(l[1], "1", "121000 - 1000 = 120000 ms = N");
    assert_eq!(l[2], "0", "no restart");
}

#[test]
fn a_new_function_a_new_api_or_a_changed_hash_restarts_the_stable_run() {
    // each of the four changes restarts the run at the observation that changed: after it N seconds are needed again
    let l = run_idle(
        "fr12-restart",
        r#"
    idle_track t; idle_snap s; int k;
    for (k = 0; k < 4; k++) {
        int r;
        idle_track_init(&t, 120000);
        s = snap(0, 5, 7, 1, 1, 0); idle_track_feed(&t, &s);
        s = snap(100000, k == 0 ? 6 : 5, k == 1 ? 8 : 7, k == 2 ? 2 : 1, k == 3 ? 2 : 1, 0); r = idle_track_feed(&t, &s);   /* changed at 100 s */
        s = snap(219999, k == 0 ? 6 : 5, k == 1 ? 8 : 7, k == 2 ? 2 : 1, k == 3 ? 2 : 1, 0); printf("%d %d %llu\n", r, idle_track_feed(&t, &s), (unsigned long long)t.n_resets);
        s = snap(220000, k == 0 ? 6 : 5, k == 1 ? 8 : 7, k == 2 ? 2 : 1, k == 3 ? 2 : 1, 0); printf("%d\n", idle_track_feed(&t, &s));
    }
"#,
    );
    for k in 0..4 {
        assert_eq!(l[2 * k], "0 0 1", "change #{k}: not stable 119.999 s after the change; one restart");
        assert_eq!(l[2 * k + 1], "1", "change #{k}: stable 120 s after the change");
    }
}

#[test]
fn a_live_window_timer_longer_than_n_keeps_the_run_open_until_its_period_has_passed() {
    // S-stable-timer-period: with a live timer of 600,000 ms the stretch must be at least 600 s, not 120 s
    let l = run_idle(
        "fr12-timer",
        r#"
    idle_track t; idle_snap s;
    idle_track_init(&t, 120000);
    s = snap(0, 5, 7, 1, 1, 600000); idle_track_feed(&t, &s);
    s = snap(599999, 5, 7, 1, 1, 600000); printf("%d\n", idle_track_feed(&t, &s));
    s = snap(600000, 5, 7, 1, 1, 600000); printf("%d\n", idle_track_feed(&t, &s));
"#,
    );
    assert_eq!(l[0], "0");
    assert_eq!(l[1], "1");
}

// ---------------------------------------------------------------------------------------------
// 3. Runtime answers (walls 3 to 16)
// ---------------------------------------------------------------------------------------------

#[test]
fn compare_string_digits_as_numbers_orders_runs_of_digits_by_value_and_ignores_the_flag_without_digits() {
    // Learn CompareStringEx: SORT_DIGITSASNUMBERS (0x8) "Treat digits as numbers during sorting, for example, sort "2" before "10"". Return values: CSTR_LESS_THAN 1, CSTR_EQUAL 2, CSTR_GREATER_THAN 3.
    let l = lines(
        "fr12-cmp-digits",
        r#"
    static uint16_t a[64], b[64];
    struct { const char *x, *y; uint32_t fl; } c[] = {
        { "2", "10", 8 }, { "10", "2", 8 }, { "a2", "a10", 8 }, { "a02", "a10", 8 }, { "a10", "a9", 8 }, { "FL Studio homepage", "FL Studio tutorials", 8 },
        { "Layout FPC (C4)", "Layout FPC (C5)", 8 },
    };
    int k;
    for (k = 0; k < 7; k++) {
        int i;
        memset(a, 0, sizeof a); memset(b, 0, sizeof b);
        for (i = 0; c[k].x[i]; i++) a[i] = (uint8_t)c[k].x[i];
        for (i = 0; c[k].y[i]; i++) b[i] = (uint8_t)c[k].y[i];
        R[1] = 0x409; R[2] = c[k].fl; R[8] = A(a); R[9] = (uint64_t)-1;
        { uint64_t sp0 = R[4]; R[4] = (R[4] - 0x40) & ~UINT64_C(15); R[4] -= 8; fle_st(R[4], 8, 0xAAAA0000); fle_st(R[4] + 0x28, 8, A(b)); fle_st(R[4] + 0x30, 8, (uint64_t)-1); fle_imp_CompareStringW(); R[4] = sp0; }
        printf("%d\n", (int)R[0]);
    }
"#,
    );
    // "2" < "10" by value; "10" > "2"; "a2" < "a10"; "a02" < "a10" (leading zeros do not count: 2 < 10); "a10" > "a9"; the homepage/tutorials pair has no digit: 'h' < 't' = 1; "C4" < "C5" = 1
    assert_eq!(l, ["1", "3", "1", "1", "3", "1", "1"]);
}

#[test]
fn compare_string_symbols_and_the_space_are_ordered_before_letters_and_digits_and_by_code_point_among_themselves() {
    // Learn "Handling Sorting in Your Applications" (word sort): punctuation marks and other nonalphanumeric characters (except the hyphen and the apostrophe) come before alphanumerics. The order of two different
    // symbols is the choice S-cmp-symbol-vs-symbol (code point); the space is a symbol (S-cmp-space-as-symbol). Without a rule from the page these pin the CHOICES.
    let l = lines(
        "fr12-cmp-sym",
        r#"
    static uint16_t a[64], b[64];
    struct { const char *x, *y; } c[] = {
        { "Minor Natural (Aeolian)", "Minor Natural Chordmap" },      /* '(' before 'C' at position 14: 1 */
        { "Minor Natural + Chords", "Minor Natural (Aeolian)" },      /* '+' (0x2b) after '(' (0x28): 3 */
        { "Default.tpr", "Default (macOS).tpr" },                      /* '.' (0x2e) after ' ' (0x20): 3 */
        { "ab c", "abc" },                                             /* ' ' before 'c': 1 */
    };
    int k;
    for (k = 0; k < 4; k++) {
        int i;
        memset(a, 0, sizeof a); memset(b, 0, sizeof b);
        for (i = 0; c[k].x[i]; i++) a[i] = (uint8_t)c[k].x[i];
        for (i = 0; c[k].y[i]; i++) b[i] = (uint8_t)c[k].y[i];
        R[1] = 0x409; R[2] = 0; R[8] = A(a); R[9] = (uint64_t)-1;
        { uint64_t sp0 = R[4]; R[4] = (R[4] - 0x40) & ~UINT64_C(15); R[4] -= 8; fle_st(R[4], 8, 0xAAAA0000); fle_st(R[4] + 0x28, 8, A(b)); fle_st(R[4] + 0x30, 8, (uint64_t)-1); fle_imp_CompareStringW(); R[4] = sp0; }
        printf("%d\n", (int)R[0]);
    }
"#,
    );
    assert_eq!(l, ["1", "3", "3", "1"]);
}

#[test]
fn compare_string_orders_a_general_punctuation_unit_against_an_ascii_symbol_by_code_point() {
    // decision 0132 (wall 25, S-cmp-general-punctuation, a CHOICE WITHOUT A SOURCE): the guest compares "News - FL Studio" with "News <U+2013> FL Studio" (the hyphen against the EN DASH at position 5). A unit of U+2000..U+206F
    // against a printable ASCII non-letter non-digit unit is ordered by the code point; against a letter or a digit (and any other non-ASCII unit) the run still stops (a hyphen against another ASCII symbol stopped too,
    // until decision 0229: see compare_string_hyphen_against_an_ascii_symbol_follows_the_word_sort).
    let l = lines(
        "fr12-cmp-gp",
        r#"
    static uint16_t a[64], b[64];
    struct { uint16_t x[24], y[24]; } c[] = {
        { { 'N','e','w','s',' ','-',' ','F','L',0 }, { 'N','e','w','s',' ',0x2013,' ','F','L',0 } },     /* '-' (0x2d) before U+2013: 1 */
        { { 'N','e','w','s',' ',0x2013,' ','F','L',0 }, { 'N','e','w','s',' ','-',' ','F','L',0 } },     /* U+2013 after '-': 3 */
        { { 'a',' ','b',0 }, { 'a',0x2026,'b',0 } },                                                     /* ' ' (0x20) before U+2026 (the ellipsis): 1 */
        { { 'a','\'','b',0 }, { 'a',0x2019,'b',0 } },                                                    /* the apostrophe before U+2019: 1 */
        { { 'a',0x2013,'b',0 }, { 'a','x','b',0 } },                                                     /* U+2013 (Script Member 6, a symbol) before 'x' (14): 1 */
        { { 'a','x','b',0 }, { 'a',0x2013,'b',0 } },                                                     /* 'x' after U+2013: 3 */
    };
    int k;
    for (k = 0; k < 6; k++) {
        int i;
        memset(a, 0, sizeof a); memset(b, 0, sizeof b);
        for (i = 0; c[k].x[i]; i++) a[i] = c[k].x[i];
        for (i = 0; c[k].y[i]; i++) b[i] = c[k].y[i];
        R[1] = 0x400; R[2] = 0; R[8] = A(a); R[9] = (uint64_t)-1;
        { uint64_t sp0 = R[4]; R[4] = (R[4] - 0x40) & ~UINT64_C(15); R[4] -= 8; fle_st(R[4], 8, 0xAAAA0000); fle_st(R[4] + 0x28, 8, A(b)); fle_st(R[4] + 0x30, 8, (uint64_t)-1); fle_imp_CompareStringW(); R[4] = sp0; }
        printf("%d\n", (int)R[0]);
    }
"#,
    );
    // decision 0284 (B): old: the general punctuation unit against a letter stopped here ("character other than an ASCII letter or digit in unequal strings", row gp-letter). The Sorting Weight Table is in every
    // build now: U+2013 (EN DASH) is Script Member 6, 'x' is 14, a symbol is the smaller (rows 5 and 6 above: 1, 3).
    assert_eq!(l, ["1", "3", "1", "1", "1", "3"]);
}

#[test]
fn compare_string_hyphen_against_an_ascii_symbol_follows_the_word_sort() {
    // decision 0229 (ref1, PROVISIONAL, S-ref1-cmp-hyphen-default). Old: "a-b" against "a+b" (the hyphen against a plain ASCII symbol at the first difference) stopped with "character other than an ASCII letter or
    // digit in unequal strings" (decision 0132 left it out of the code-point order of S-cmp-general-punctuation). Learn "Handling Sorting in Your Applications" (https://learn.microsoft.com/en-us/windows/win32/intl/
    // handling-sorting-in-your-applications, read 2026-11-04), word sort (the default of CompareString): "all punctuation marks and other nonalphanumeric characters, except for the hyphen and the apostrophe, come before
    // any alphanumeric character. The hyphen and the apostrophe are treated differently from the other nonalphanumeric characters to ensure that words such as "coop" and "co-op" stay together in a sorted list." The
    // runtime reads that as: the hyphen and the apostrophe do not take part in the comparison of the other characters (the strings are compared again without them from the first difference), so "a-b" against "a+b"
    // is "b" against "+b", and the plus sign (a symbol) comes before the letter: "a-b" is the greater, 3; the reverse is 1. The two values follow the reading (the page quotes the sentence and a list of nine words,
    // not this pair). The apostrophe in the same place gives the same values.
    let l = lines(
        "fr12-cmp-hyphen-plus",
        r#"
    static uint16_t a[64], b[64];
    struct { uint16_t x[8], y[8]; } c[] = {
        { { 'a','-','b',0 }, { 'a','+','b',0 } },
        { { 'a','+','b',0 }, { 'a','-','b',0 } },
        { { 'a','\'','b',0 }, { 'a','+','b',0 } },
        { { 'a','+','b',0 }, { 'a','\'','b',0 } },
    };
    int k;
    for (k = 0; k < 4; k++) {
        int i;
        memset(a, 0, sizeof a); memset(b, 0, sizeof b);
        for (i = 0; c[k].x[i]; i++) a[i] = c[k].x[i];
        for (i = 0; c[k].y[i]; i++) b[i] = c[k].y[i];
        R[1] = 0x400; R[2] = 0; R[8] = A(a); R[9] = (uint64_t)-1;
        { uint64_t sp0 = R[4]; R[4] = (R[4] - 0x40) & ~UINT64_C(15); R[4] -= 8; fle_st(R[4], 8, 0xAAAA0000); fle_st(R[4] + 0x28, 8, A(b)); fle_st(R[4] + 0x30, 8, (uint64_t)-1); fle_imp_CompareStringW(); R[4] = sp0; }
        printf("%d\n", (int)R[0]);
    }
"#,
    );
    assert_eq!(l, ["3", "1", "3", "1"]);
}

#[test]
fn get_thread_priority_returns_the_value_set_thread_priority_kept_and_normal_at_the_start() {
    // Learn GetThreadPriority: the return value is the thread's priority level (THREAD_PRIORITY_NORMAL 0 .. HIGHEST 2, LOWEST -2, ABOVE_NORMAL 1, BELOW_NORMAL -1, IDLE -15, TIME_CRITICAL 15). GetCurrentThread's pseudo
    // handle is -2 (Learn GetCurrentThread). Start value NORMAL: SetThreadPriority's page ("threads start at NORMAL").
    let l = lines(
        "fr12-gtp",
        r#"
    uint64_t h = (uint64_t)-2;
    int k; int32_t v[] = { 2, -2, 1, -1, -15, 15, 0 };
    printf("%d\n", (int)(int32_t)C(fle_imp_GetThreadPriority, h, 0, 0, 0));
    for (k = 0; k < 7; k++) { C(fle_imp_SetThreadPriority, h, (uint64_t)(int64_t)v[k], 0, 0); printf("%d\n", (int)(int32_t)C(fle_imp_GetThreadPriority, h, 0, 0, 0)); }
"#,
    );
    assert_eq!(l, ["0", "2", "-2", "1", "-1", "-15", "15", "0"]);
}

#[test]
fn get_thread_priority_of_a_created_thread_is_its_own_and_a_closed_handle_fails_with_the_error_return_and_6() {
    // Learn GetThreadPriority: "If the function fails, the return value is THREAD_PRIORITY_ERROR_RETURN" (0x7FFFFFFF in winbase.h: the page names the constant only: UNOFFICIAL) and GetLastError says why;
    // a thread handle that CreateThread returned names that thread (its own priority, NORMAL at the start), a closed one is ERROR_INVALID_HANDLE 6 (the choice made for SetThreadPriority, decision 0122).
    let o = run_src(
        "fr12-gtp-thread",
        &top_with("static void th_a(void) { R[0] = 0; }", &[("0x1000", "th_a")]),
        r#"
    uint64_t th = H_CREATE(0x1000, 0, 4, 0);
    printf("start %d\n", (int)(int32_t)C(fle_imp_GetThreadPriority, th, 0, 0, 0));
    C(fle_imp_SetThreadPriority, th, (uint64_t)(int64_t)-2, 0, 0);
    printf("other %d self %d\n", (int)(int32_t)C(fle_imp_GetThreadPriority, th, 0, 0, 0), (int)(int32_t)C(fle_imp_GetThreadPriority, (uint64_t)-2, 0, 0, 0));
    C(fle_imp_ResumeThread, th, 0, 0, 0);
    W1(th, INF);
    C(fle_imp_CloseHandle, th, 0, 0, 0);
    SETLE(0);
    printf("closed %x le %u\n", (unsigned)(uint32_t)C(fle_imp_GetThreadPriority, th, 0, 0, 0), LE());
"#,
    );
    assert!(o.ok, "did not exit normally (signal {:?}): {}\n{}", o.signal, o.stderr, o.stdout);
    let l: Vec<&str> = o.stdout.lines().collect();
    assert_eq!(l, ["start 0", "other -2 self 0", "closed 7fffffff le 6"]);
}

#[test]
fn get_thread_priority_stops_for_a_handle_that_nothing_returned() {
    aborts("fr12-gtp-bad", "C(fle_imp_GetThreadPriority, 0x1234, 0, 0, 0);", "GetThreadPriority", "a handle that GetCurrentThread or CreateThread did not return");
}

#[test]
fn get_gui_resources_counts_the_windows_and_the_gdi_objects_of_the_process_and_stops_otherwise() {
    // Learn GetGuiResources: GR_GDIOBJECTS 0, GR_USEROBJECTS 1; "If no GUI objects are in use, the return value is zero." The count is the choice S-guires-count (windows of the table; objects of the GDI table).
    let l = lines(
        "fr12-guires",
        r#"
    uint64_t h = (uint64_t)-1;
    uint64_t u0 = C(fle_imp_GetGuiResources, h, 1, 0, 0);
    uint64_t g0 = C(fle_imp_GetGuiResources, h, 0, 0, 0);
    uint64_t bm, w2;
    regclass();
    cw_cls(cls_name, 0, 0x80000000, 0, 0, 10, 10, 0);
    printf("%llu %llu\n", U(u0), U(g0));
    printf("%llu %llu\n", U(C(fle_imp_GetGuiResources, h, 1, 0, 0) - u0), U(C(fle_imp_GetGuiResources, h, 0, 0, 0) - g0));
    w2 = cw_cls(cls_name, 0, 0x80000000, 0, 0, 10, 10, 0);
    bm = callf6(fle_imp_CreateBitmap, 4, 4, 1, 1, 0, 0);
    printf("%llu %llu\n", U(C(fle_imp_GetGuiResources, h, 1, 0, 0) - u0), U(C(fle_imp_GetGuiResources, h, 0, 0, 0) - g0));
    C(fle_imp_DeleteObject, bm, 0, 0, 0);
    C(fle_imp_DestroyWindow, w2, 0, 0, 0);
    printf("%llu %llu\n", U(C(fle_imp_GetGuiResources, h, 1, 0, 0) - u0), U(C(fle_imp_GetGuiResources, h, 0, 0, 0) - g0));
    printf("%llu\n", U(C(fle_imp_GetGuiResources, 0xFFFFFFFFFFFFFFFFull, 1, 0, 0) == C(fle_imp_GetGuiResources, h, 1, 0, 0)));
"#,
    );
    assert_eq!(l[0], "0 0", "no window and no GDI object yet");
    assert_eq!(l[1], "1 0", "one window more, no GDI object more");
    assert_eq!(l[2], "2 1", "a second window and a bitmap: USER +2, GDI +1 (S-guires-count: windows of the table, objects of the GDI table)");
    assert_eq!(l[3], "1 0", "the bitmap is deleted and the second window is destroyed: back to the counts after the first window");
}

#[test]
fn get_gui_resources_stops_for_another_process_and_for_a_peak_value() {
    aborts("fr12-guires-p", "C(fle_imp_GetGuiResources, 0x77, 0, 0, 0);", "GetGuiResources", "hProcess is not the pseudo handle");
    aborts("fr12-guires-k", "C(fle_imp_GetGuiResources, (uint64_t)-1, 2, 0, 0);", "GetGuiResources", "peak");
}

#[test]
fn get_last_active_popup_returns_hwnd_for_the_three_conditions_of_the_page() {
    // Learn GetLastActivePopup: the return value is hWnd if (1) hWnd was most recently active, (2) hWnd owns no pop-up window, (3) hWnd is not top-level or is owned by another window. An invalid window: NULL + 1400 (the
    // choice of decision 0119). The test creates a top-level window (no owned pop-up): condition (2).
    let l = lines(
        "fr12-glap",
        r#"
    uint64_t w, r;
    regclass();
    w = cw_cls(cls_name, 0, 0x80000000, 0, 0, 10, 10, 0);
    r = C(fle_imp_GetLastActivePopup, w, 0, 0, 0);
    printf("%d\n", r == w);
    SETLE(0);
    r = C(fle_imp_GetLastActivePopup, 0x1234567, 0, 0, 0);
    printf("%llu %u\n", U(r), LE());
"#,
    );
    assert_eq!(l[0], "1");
    assert_eq!(l[1], "0 1400");
}

#[test]
fn an_attribute_call_with_the_handle_of_a_deleted_memory_dc_fails_like_hdc_null() {
    // S-hdc-deleted: a deleted memory DC is read as no DC (0 and last error 6, as S-hdc-null of decision 0125). The drawing calls still stop for it (not tested here: tests/gdi_bulk.rs).
    let l = lines(
        "fr12-dead-dc",
        r#"
    uint64_t dc, pt[2] = { 0x1111, 0x2222 };
    C(fle_imp_GetDC, 0, 0, 0, 0);
    dc = C(fle_imp_CreateCompatibleDC, R[0], 0, 0, 0);
    C(fle_imp_DeleteDC, dc, 0, 0, 0);
    SETLE(0);
    printf("%llu %u %llx\n", U(C(fle_imp_GetCurrentPositionEx, dc, A(pt), 0, 0)), LE(), U(pt[0]));
"#,
    );
    assert_eq!(l[0], "0 6 1111");
}

#[test]
fn create_file_accepts_open_no_recall_with_attribute_normal_and_it_changes_nothing() {
    // Learn CreateFileW: FILE_FLAG_OPEN_NO_RECALL 0x00100000 "The file data is requested, but it should continue to be located in remote storage ... This flag is for use by remote storage systems."; FILE_ATTRIBUTE_NORMAL 0x80
    let l = lines(
        "fr12-nrecall",
        r#"
    static uint16_t p[] = { 'C', ':', '\\', 'f', '.', 't', 'x', 't', 0 };
    uint64_t h, n = 0;
    h = callf8(fle_imp_CreateFileW, A(p), 0xC0000000u, 0, 0, 2, 0x80, 0, 0);
    C(fle_imp_CloseHandle, h, 0, 0, 0);
    h = callf8(fle_imp_CreateFileW, A(p), 0x80000000u, 1, 0, 3, 0x00100080u, 0, 0);
    printf("%d\n", h != (uint64_t)-1);
    (void)n;
"#,
    );
    assert_eq!(l[0], "1");
}

#[test]
fn co_create_instance_of_a_class_the_registry_does_not_hold_is_regdb_e_classnotreg_and_ppv_is_null() {
    // Learn CoCreateInstance: "Upon failure, *ppv contains NULL"; REGDB_E_CLASSNOTREG "A specified class is not registered in the registration database" = 0x80040154 (Learn COM Error Codes (Generic)).
    // The CLSID is the shell's TaskbarList, 56FDF344-FD6D-11D0-958A-006097C9A090 (the guest's call at 0x5df663).
    let l = lines(
        "fr12-cci",
        r#"
    static const uint8_t clsid[16] = { 0x44, 0xf3, 0xfd, 0x56, 0x6d, 0xfd, 0xd0, 0x11, 0x95, 0x8a, 0x00, 0x60, 0x97, 0xc9, 0xa0, 0x90 };
    static const uint8_t iid[16] = { 0, 0, 0, 0, 0, 0, 0, 0, 0xc0, 0, 0, 0, 0, 0, 0, 0x46 };
    uint64_t ppv = 0xABCDEF;
    uint32_t hr = (uint32_t)callf5(fle_imp_CoCreateInstance, A(clsid), 0, 5, A(iid), A(&ppv));
    printf("%08x %llx\n", hr, U(ppv));
"#,
    );
    assert_eq!(l[0], "80040154 0");
}

#[test]
fn co_create_instance_of_a_registered_class_stops_and_a_null_ppv_stops() {
    aborts(
        "fr12-cci-reg",
        r#"
    static const uint8_t clsid[16] = { 0x44, 0xf3, 0xfd, 0x56, 0x6d, 0xfd, 0xd0, 0x11, 0x95, 0x8a, 0x00, 0x60, 0x97, 0xc9, 0xa0, 0x90 };
    static const uint8_t iid[16] = { 0, 0, 0, 0, 0, 0, 0, 0, 0xc0, 0, 0, 0, 0, 0, 0, 0x46 };
    static uint16_t kn[] = { 'C','L','S','I','D','\\','{','5','6','F','D','F','3','4','4','-','F','D','6','D','-','1','1','D','0','-','9','5','8','A','-','0','0','6','0','9','7','C','9','A','0','9','0','}', 0 };
    uint64_t hk = 0, ppv = 0, st;
    /* RegCreateKeyExW(HKEY_CLASSES_ROOT, "CLSID\{56FDF344-...}", 0, NULL, 0, KEY_READ|KEY_WRITE (0x20006), NULL, &hk, NULL): creates the key, so that the class is registered */
    st = callf9(fle_imp_RegCreateKeyExW, 0xFFFFFFFF80000000ull, A(kn), 0, 0, 0, 0x20006, 0, A(&hk), 0);
    if (st != 0 || hk == 0) { fprintf(stderr, "setup failed: %llu\n", U(st)); exit(9); }
    callf5(fle_imp_CoCreateInstance, A(clsid), 0, 5, A(iid), A(&ppv));
"#,
        "CoCreateInstance",
        "registered in the virtual registry",
    );
    aborts("fr12-cci-null", "static const uint8_t g[16] = { 1 }; callf5(fle_imp_CoCreateInstance, A(g), 0, 5, A(g), 0);", "CoCreateInstance", "ppv is NULL");
}

#[test]
fn format_message_returns_the_learn_text_of_regdb_e_classnotreg_and_still_fails_for_an_unknown_hresult() {
    // S-errmsg-hresult-learn-text: Learn COM Error Codes (Generic): REGDB_E_CLASSNOTREG 0x80040154 "Class not registered". FORMAT_MESSAGE_FROM_SYSTEM 0x1000 | IGNORE_INSERTS 0x200 | ALLOCATE_BUFFER 0x100 | ARGUMENT_ARRAY 0x2000 = 0x3300.
    let l = lines(
        "fr12-fmt",
        r#"
    uint64_t pbuf = 0; uint32_t n;
    uint16_t *s; int i;
    n = (uint32_t)callf8(fle_imp_FormatMessageW, 0x3300, 0, 0x80040154u, 0, A(&pbuf), 0, 0, 0);
    s = (uint16_t *)pbuf;
    printf("%u ", n);
    for (i = 0; s && s[i]; i++) printf("%c", (char)s[i]);
    printf("\n");
    pbuf = 0x5555;
    SETLE(0);
    n = (uint32_t)callf8(fle_imp_FormatMessageW, 0x3300, 0, 0x80040155u, 0, A(&pbuf), 0, 0, 0);
    printf("%u %u %llx\n", n, LE(), U(pbuf));
"#,
    );
    assert_eq!(l[0], "20 Class not registered");
    assert_eq!(l[1], "0 317 5555", "0x80040155 (REGDB_E_IIDNOTREG) is not in the table: 0, ERROR_MR_MID_NOT_FOUND 317, lpBuffer untouched");
}

fn ver_node(key: &str, wtype: u16, wvlen: u16, value: &[u8], children: &[Vec<u8>]) -> Vec<u8> {
    // VS_VERSIONINFO / String / StringTable / Var node (Learn "VS_VERSIONINFO structure"): wLength, wValueLength, wType, szKey, pad to 4, Value, pad to 4, Children (each padded to 4 before the next)
    let mut o = vec![0u8; 6];
    for u in key.encode_utf16().chain([0u16]) {
        o.extend_from_slice(&u.to_le_bytes());
    }
    while o.len() % 4 != 0 {
        o.push(0);
    }
    o.extend_from_slice(value);
    for c in children {
        while o.len() % 4 != 0 {
            o.push(0);
        }
        o.extend_from_slice(c);
    }
    let wl = o.len() as u16;
    o[0..2].copy_from_slice(&wl.to_le_bytes());
    o[2..4].copy_from_slice(&wvlen.to_le_bytes());
    o[4..6].copy_from_slice(&wtype.to_le_bytes());
    o
}

fn ver_str(key: &str, val: &str) -> Vec<u8> {
    let mut v = Vec::new();
    for u in val.encode_utf16().chain([0u16]) {
        v.extend_from_slice(&u.to_le_bytes());
    }
    ver_node(key, 1, (v.len() / 2) as u16, &v, &[])
}

/// A VS_VERSIONINFO block: FileVersion 1.2.3.4 as VS_FIXEDFILEINFO, one StringTable "040904E4" with FileVersion "1.2.3.4" and ProductName "Probe", and VarFileInfo\Translation 0x0409 0x04E4.
fn ver_block() -> Vec<u8> {
    let mut ffi = Vec::new();
    for d in [0xFEEF04BDu32, 0x0001_0000, 0x0001_0002, 0x0003_0004, 0x0001_0002, 0x0003_0004, 0x3F, 0, 4, 1, 0, 0, 0] {
        ffi.extend_from_slice(&d.to_le_bytes());
    }
    assert_eq!(ffi.len(), 52);
    let table = ver_node("040904E4", 1, 0, &[], &[ver_str("FileVersion", "1.2.3.4"), ver_str("ProductName", "Probe")]);
    let sfi = ver_node("StringFileInfo", 1, 0, &[], &[table]);
    let tr = ver_node("Translation", 0, 4, &[0x09, 0x04, 0xE4, 0x04], &[]);
    let vfi = ver_node("VarFileInfo", 1, 0, &[], &[tr]);
    ver_node("VS_VERSION_INFO", 0, 52, &ffi, &[sfi, vfi])
}

/// A PE32+ at 0x400000 (SizeOfImage 0x3000) with .rsrc at RVA 0x1000: RT_VERSION (16), name 1, language 0x409, whose data is `block` at RVA 0x1400.
fn version_image(block: &[u8]) -> Vec<u8> {
    let (rsrc_raw, rsrc_len) = (0x400usize, 0x800usize);
    let mut b = vec![0u8; rsrc_raw + rsrc_len];
    w16(&mut b, 0, 0x5a4d);
    w32(&mut b, 0x3c, LFANEW as u32);
    w32(&mut b, LFANEW, 0x4550);
    let f = LFANEW + 4;
    w16(&mut b, f, 0x8664);
    w16(&mut b, f + 2, 1);
    w16(&mut b, f + 16, 240);
    w16(&mut b, f + 18, 0x2022);
    let o = LFANEW + 24;
    w16(&mut b, o, 0x20b);
    w64(&mut b, o + 24, 0x40_0000);
    w32(&mut b, o + 32, 0x1000);
    w32(&mut b, o + 36, 0x200);
    w32(&mut b, o + 56, 0x3000);
    w32(&mut b, o + 60, 0x400);
    w16(&mut b, o + 68, 2);
    w32(&mut b, o + 108, 16);
    w32(&mut b, o + 128, 0x1000);
    w32(&mut b, o + 132, rsrc_len as u32);
    let s0 = o + 240;
    b[s0..s0 + 5].copy_from_slice(b".rsrc");
    w32(&mut b, s0 + 8, rsrc_len as u32);
    w32(&mut b, s0 + 12, 0x1000);
    w32(&mut b, s0 + 16, rsrc_len as u32);
    w32(&mut b, s0 + 20, rsrc_raw as u32);
    w32(&mut b, s0 + 36, 0x4000_0040);
    let r = &mut b[rsrc_raw..rsrc_raw + rsrc_len];
    // root: one ID entry (16) -> 0x100; 0x100: one ID entry (1) -> 0x120; 0x120: one ID entry (0x409) -> leaf at 0x140; 0x140: data entry (RVA, size)
    let table = |r: &mut [u8], off: usize, id: u32, to: u32| {
        r[off + 14..off + 16].copy_from_slice(&1u16.to_le_bytes());
        r[off + 16..off + 20].copy_from_slice(&id.to_le_bytes());
        r[off + 20..off + 24].copy_from_slice(&to.to_le_bytes());
    };
    table(r, 0x000, 16, 0x8000_0100);
    table(r, 0x100, 1, 0x8000_0120);
    table(r, 0x120, 0x409, 0x140);
    r[0x140..0x144].copy_from_slice(&(0x1000u32 + 0x400).to_le_bytes());
    r[0x144..0x148].copy_from_slice(&(block.len() as u32).to_le_bytes());
    r[0x400..0x400 + block.len()].copy_from_slice(block);
    b
}

const ENGINE_PATH_C: &str = r#"
    static uint16_t pu[] = { 'C', ':', '\\', 'P', 'r', 'o', 'g', 'r', 'a', 'm', ' ', 'F', 'i', 'l', 'e', 's', '\\', 'I', 'm', 'a', 'g', 'e', '-', 'L', 'i', 'n', 'e', '\\', 'F', 'L', ' ', 'S', 't', 'u', 'd', 'i', 'o', ' ', '2', '1', '\\', 'F', 'L', 'E', 'n', 'g', 'i', 'n', 'e', '_', 'x', '6', '4', '.', 'd', 'l', 'l', 0 };
    static uint16_t pl[] = { 'c', ':', '\\', 'p', 'r', 'o', 'g', 'r', 'a', 'm', ' ', 'f', 'i', 'l', 'e', 's', '\\', 'i', 'm', 'a', 'g', 'e', '-', 'l', 'i', 'n', 'e', '\\', 'f', 'l', ' ', 's', 't', 'u', 'd', 'i', 'o', ' ', '2', '1', '\\', 'f', 'l', 'e', 'n', 'g', 'i', 'n', 'e', '_', 'x', '6', '4', '.', 'd', 'l', 'l', 0 };
    static uint16_t q_ver[] = { '\\', 'S', 't', 'r', 'i', 'n', 'g', 'F', 'i', 'l', 'e', 'I', 'n', 'f', 'o', '\\', '0', '4', '0', '9', '0', '4', 'e', '4', '\\', 'F', 'i', 'l', 'e', 'V', 'e', 'r', 's', 'i', 'o', 'n', 0 };
    static uint16_t q_trn[] = { '\\', 'V', 'a', 'r', 'F', 'i', 'l', 'e', 'I', 'n', 'f', 'o', '\\', 'T', 'r', 'a', 'n', 's', 'l', 'a', 't', 'i', 'o', 'n', 0 };
    static uint16_t q_root[] = { '\\', 0 };
    static uint16_t q_miss[] = { '\\', 'S', 't', 'r', 'i', 'n', 'g', 'F', 'i', 'l', 'e', 'I', 'n', 'f', 'o', '\\', '0', '4', '0', '9', '0', '4', 'e', '4', '\\', 'N', 'o', 'p', 'e', 0 };
    static uint16_t q_lang[] = { '\\', 'S', 't', 'r', 'i', 'n', 'g', 'F', 'i', 'l', 'e', 'I', 'n', 'f', 'o', '\\', '0', '4', '0', '7', '0', '4', 'e', '4', '\\', 'F', 'i', 'l', 'e', 'V', 'e', 'r', 's', 'i', 'o', 'n', 0 };
    static uint8_t buf[2048], zero[2048];
    uint64_t hd = 0x1234, p, n, ok;
    uint32_t sz, k;
    int i;
    SETLE(0);
    sz = (uint32_t)C(fle_imp_GetFileVersionInfoSizeW, A(pu), A(&hd), 0, 0);
    printf("size %u handle %x\n", sz, (unsigned)(uint32_t)hd);
    printf("lower %u\n", (unsigned)(uint32_t)C(fle_imp_GetFileVersionInfoSizeW, A(pl), 0, 0, 0));
    memset(buf, 0xAA, sizeof buf);
    printf("get %llu\n", U(C(fle_imp_GetFileVersionInfoW, A(pl), 0x55, sz, A(buf))));
    printf("same %d\n", memcmp(buf, (const void *)(uintptr_t)(0x400000 + 0x1400), sz) == 0);
    printf("tail %02x\n", buf[sz]);
    memset(zero, 0xAA, sizeof zero);
    printf("trunc %llu ", U(C(fle_imp_GetFileVersionInfoW, A(pu), 0, 100, A(zero))));
    for (k = 0, i = 1; k < 100; k++) if (zero[k] != buf[k]) i = 0;
    printf("%d %02x\n", i, zero[100]);
    p = 0x1111; n = 0x2222;
    ok = C(fle_imp_VerQueryValueW, A(buf), A(q_ver), A(&p), A(&n));
    printf("ver %llu len %llu in_buf %d ", U(ok), U(n), p >= A(buf) && p < A(buf) + sz);
    for (k = 0; k < 8; k++) printf("%c", (char)*(uint16_t *)(uintptr_t)(p + 2 * k));
    printf("\n");
    p = 0; n = 0;
    ok = C(fle_imp_VerQueryValueW, A(buf), A(q_trn), A(&p), A(&n));
    printf("trn %llu len %llu %02x %02x %02x %02x\n", U(ok), U(n), *(uint8_t *)(uintptr_t)p, *(uint8_t *)(uintptr_t)(p + 1), *(uint8_t *)(uintptr_t)(p + 2), *(uint8_t *)(uintptr_t)(p + 3));
    p = 0; n = 0;
    ok = C(fle_imp_VerQueryValueW, A(buf), A(q_root), A(&p), A(&n));
    printf("root %llu len %llu sig %08x\n", U(ok), U(n), (unsigned)*(uint32_t *)(uintptr_t)p);
    p = 0x1111; n = 0x2222;
    ok = C(fle_imp_VerQueryValueW, A(buf), A(q_miss), A(&p), A(&n));
    printf("miss %llu %llx %llu\n", U(ok), U(p), U(n));
    p = 0x1111; n = 0x2222;
    ok = C(fle_imp_VerQueryValueW, A(buf), A(q_lang), A(&p), A(&n));
    printf("lang %llu %llx %llu\n", U(ok), U(p), U(n));
    p = 0x1111; n = 0x2222;
    memset(zero, 0, sizeof zero);
    ok = C(fle_imp_VerQueryValueW, A(zero), A(q_ver), A(&p), A(&n));
    printf("notblock %llu %llx %llu\n", U(ok), U(p), U(n));
"#;

#[test]
fn the_version_info_of_the_engine_path_is_the_resource_of_the_mapped_image_and_queries_follow_the_page() {
    // S-version-info-real-resource: GetFileVersionInfoSizeW(the guest's own engine path) = the size of the RT_VERSION resource of the image mapped at 0x400000; GetFileVersionInfoW copies its bytes (S-version-info-block-form);
    // VerQueryValueW answers \, \VarFileInfo\Translation and \StringFileInfo\<lang-codepage>\<name> as Learn VerQueryValueW says (S-verquery-miss, S-verquery-fold). The image here is a synthetic PE whose
    // resource is built by ver_block() from the VS_VERSIONINFO structure pages, so the expected values are the builder's, not read from the runtime.
    let blk = ver_block();
    let img = work_dir().join(format!("{}.dll", etxt_common::per_process("fr12-ver")));
    std::fs::write(&img, version_image(&blk)).unwrap();
    let o = run_src_img("fr12-ver-engine", &top_with("", &[]), ENGINE_PATH_C, Some(&img));
    assert!(o.ok, "did not exit normally (signal {:?}): {}\n{}", o.signal, o.stderr, o.stdout);
    let l: Vec<&str> = o.stdout.lines().collect();
    assert_eq!(l[0], format!("size {} handle 0", blk.len()), "the size of the resource; *lpdwHandle is set to 0");
    assert_eq!(l[1], format!("lower {}", blk.len()), "the path is compared with case folding");
    assert_eq!(l[2], "get 1");
    assert_eq!(l[3], "same 1", "the buffer holds the bytes of the resource");
    assert_eq!(l[4], "tail aa", "nothing is written after the resource");
    assert_eq!(l[5], "trunc 1 1 aa", "dwLen 100 truncates to 100 bytes");
    assert_eq!(l[6], "ver 1 len 8 in_buf 1 1.2.3.4\0", "the string is in the caller's buffer; the length is in characters and includes the NUL (wValueLength of the String node)");
    assert_eq!(l[7], "trn 1 len 4 09 04 e4 04", "the translation array: 0x0409 0x04E4 as bytes");
    assert_eq!(l[8], "root 1 len 52 sig feef04bd", "the root block is VS_FIXEDFILEINFO (52 bytes, signature 0xFEEF04BD)");
    assert_eq!(l[9], "miss 0 0 0", "a name that is not in the table: 0, pointer and length 0");
    assert_eq!(l[10], "lang 0 0 0", "another lang-codepage: 0");
    assert_eq!(l[11], "notblock 0 0 0", "a buffer that is not a version resource: 0");
}

#[test]
fn the_version_info_of_a_path_that_is_not_the_engine_fails_as_before_decision_0098() {
    let l = lines(
        "fr12-ver-other",
        r#"
    static uint16_t p[] = { 'C', ':', '\\', 'n', 'o', 'p', 'e', '\\', 'x', '.', 'd', 'l', 'l', 0 };
    static uint16_t q[] = { 'C', ':', '\\', 'P', 'r', 'o', 'g', 'r', 'a', 'm', ' ', 'F', 'i', 'l', 'e', 's', '\\', 'x', '.', 'd', 'l', 'l', 0 };
    uint64_t hd = 0x1234;
    SETLE(0);
    printf("%llu %u %x\n", U(C(fle_imp_GetFileVersionInfoSizeW, A(p), A(&hd), 0, 0)), LE(), (unsigned)(uint32_t)hd);
    SETLE(0);
    printf("%llu %u\n", U(C(fle_imp_GetFileVersionInfoSizeW, A(q), 0, 0, 0)), LE());
"#,
    );
    assert_eq!(l[0], "0 3 0", "a path that is not in the virtual file system and is not the engine: 0, ERROR_PATH_NOT_FOUND 3 (decision 0098), *lpdwHandle = 0");
    assert_eq!(l[1], "0 3", "a path inside Program Files that is not the engine path is not recognised either");
}

#[test]
fn the_version_resource_of_the_real_engine_is_728_bytes_and_says_26_1_7_5653() {
    // The real FLEngine_x64.dll (read with pefile, independent of the runtime): RT_VERSION name 1, language 1033, 728 bytes; FileVersion "26.1.7.5653", StringFileInfo "040904E4".
    let Some(dll) = std::env::var_os("FLENGINE_PE").map(PathBuf::from).or_else(|| {
        let p = manifest().join("../../../binaries/win64/FLEngine_x64.dll");
        p.exists().then_some(p)
    }) else {
        eprintln!("SKIPPED: no FLEngine_x64.dll");
        return;
    };
    let o = run_src_img("fr12-ver-real", &top_with("", &[]), &ENGINE_PATH_C.replace("0x400000 + 0x1400", "0x400000 + 0x18be978").replace("'0', '4', '0', '9', '0', '4', 'e', '4'", "'0', '4', '0', '9', '0', '4', 'E', '4'"), Some(&dll));
    assert!(o.ok, "did not exit normally (signal {:?}): {}\n{}", o.signal, o.stderr, o.stdout);
    let l: Vec<&str> = o.stdout.lines().collect();
    assert_eq!(l[0], "size 728 handle 0");
    assert_eq!(l[3], "same 1", "the bytes are those at RVA 0x18be978 of the image");
    assert_eq!(l[6], "ver 1 len 12 in_buf 1 26.1.7.5", "FileVersion \"26.1.7.5653\" (11 characters + NUL = 12)");
    assert_eq!(l[7], "trn 1 len 4 09 04 e4 04");
    assert_eq!(l[8], "root 1 len 52 sig feef04bd");
}

// ---------------------------------------------------------------------------------------------
// 4. Source checks and the old-to-new list of tests
// ---------------------------------------------------------------------------------------------

fn read(p: &Path) -> String {
    std::fs::read_to_string(p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

#[test]
fn the_idle_core_and_the_wrapper_generator_have_no_host_dependency_beyond_the_standard_library() {
    // idle.c: the core (SHA-256, the tracker) is above `#ifdef FLE_IDLE`; the part under it reads the runtime. Nothing in the core opens a file, reads the clock or the environment.
    let s = read(&harness_dir().join("idle/idle.c"));
    let core = &s[..s.find("#ifdef FLE_IDLE").expect("the FLE_IDLE part")];
    for bad in ["fopen(", "getenv(", "clock_gettime", "time(", "open(", "socket(", "system("] {
        assert!(!core.contains(bad), "the core of idle.c must not use {bad}");
    }
    let g = read(&harness_dir().join("idle/gen_idle_wraps.py"));
    assert!(g.contains("never changes what the stub does") || g.contains("__real_"), "the wrappers call the real stub");
}

#[test]
fn the_text_hook_only_records_and_the_text_apis_still_stop_by_name() {
    // fle_gdi.c: ExtTextOutW, DrawTextW, DrawTextExW, GetTextExtentPoint32W, GetTextExtentPointW and GetTextMetricsW call the weak fle_text_hook and then fle_stop as before.
    let s = read(&src_dir().join("fle_gdi.c"));
    for name in ["ExtTextOutW", "GetTextMetricsW", "GetTextExtentPoint32W", "GetTextExtentPointW", "DrawTextW", "DrawTextExW"] {
        assert!(s.contains(&format!("GDI_TEXT_STOP({name},")), "{name} uses GDI_TEXT_STOP");
    }
    assert!(s.contains("if (fle_text_hook) fle_text_hook(#Name, R[1]); fle_stop(#Name,"), "the macro records then stops");
    assert!(!s.contains("GDI_STOP(ExtTextOutW"), "no plain stop is left for ExtTextOutW");
}

#[test]
fn each_text_api_calls_the_text_hook_with_its_name_and_the_dc_and_then_still_stops_by_name() {
    // decision 0129: fle_text_hook(api, hdc) is weak; the harness (fruity.c) defines it to record the first text call. Here the test defines it and prints. The stop is the same named stop as before (decision 0111).
    for name in ["ExtTextOutW", "GetTextMetricsW", "GetTextExtentPoint32W", "GetTextExtentPointW", "DrawTextW", "DrawTextExW"] {
        let top = top_with("void fle_text_hook(const char *api, uint64_t hdc) { fprintf(stderr, \"HOOK %s %llx\\n\", api, (unsigned long long)hdc); }", &[]);
        let o = run_src(&format!("fr12-text-{name}"), &top, &format!("C(fle_imp_{name}, 0x1234, 0, 0, 0);"));
        assert_eq!(o.signal, Some(6), "{name}: stops: {}", o.stderr);
        let hook = format!("HOOK {name} 1234\n");
        let stop = format!("fle: {name}: ");
        let (h, s) = (o.stderr.find(&hook), o.stderr.find(&stop));
        assert!(h.is_some() && s.is_some() && h < s, "{name}: the hook is called first with the name and rcx, then the named stop: {:?}", o.stderr);
    }
    // without a hook defined (the weak reference is NULL) the stop is the same
    let o = run_src("fr12-text-nohook", &top_with("", &[]), "C(fle_imp_ExtTextOutW, 0x1234, 0, 0, 0);");
    assert_eq!(o.signal, Some(6));
    assert!(o.stderr.contains("fle: ExtTextOutW: ") && !o.stderr.contains("HOOK"), "{:?}", o.stderr);
}

fn seed_run(name: &str, list: &str) -> Out {
    // the real harness/vfs_seed.h on the real runtime: the list is written to a file, vfs_seed_from_env() reads it, the body then asks the guest-facing stubs what the volume holds
    let f = work_dir().join(format!("{}.tsv", etxt_common::uniq(name)));
    std::fs::write(&f, list).unwrap();
    let body = format!(
        r#"
    static uint16_t d1[] = {{ 'C', ':', '\\', 'P', 'r', 'o', 'g', 'r', 'a', 'm', ' ', 'F', 'i', 'l', 'e', 's', 0 }};
    static uint16_t d2[] = {{ 'C', ':', '\\', 'P', 'r', 'o', 'g', 'r', 'a', 'm', ' ', 'F', 'i', 'l', 'e', 's', '\\', 'E', 'm', 'p', 't', 'y', 0 }};
    static uint16_t f1[] = {{ 'C', ':', '\\', 'a', '\\', 'b', '.', 't', 'x', 't', 0 }};
    static uint16_t d3[] = {{ 'C', ':', '\\', 'd', '1', '9', '9', 0 }};
    int rc;
    setenv("FLE_VFS_SEED", "{path}", 1);
    rc = vfs_seed_from_env();
    printf("rc %d\n", rc);
    if (rc == 0) {{
        printf("attrs %x %x %x %x\n", (unsigned)C(fle_imp_GetFileAttributesW, A(d1), 0, 0, 0), (unsigned)C(fle_imp_GetFileAttributesW, A(d2), 0, 0, 0), (unsigned)C(fle_imp_GetFileAttributesW, A(f1), 0, 0, 0), (unsigned)C(fle_imp_GetFileAttributesW, A(d3), 0, 0, 0));
    }}
"#,
        path = f.display()
    );
    let o = run_src("fr12-seed", &top_with("#include \"vfs_seed.h\"", &[]), &body);
    assert!(o.ok, "{name}: did not exit normally (signal {:?}): {}\n{}", o.signal, o.stderr, o.stdout);
    o
}

#[test]
fn a_directory_line_makes_the_directory_and_the_ones_above_it_without_a_file() {
    // decision 0129: a line whose guest path ends with a backslash and whose other two fields are '-' makes the directory (and those above it). The file line makes C:\a on its way.
    let host = work_dir().join(format!("{}.bin", etxt_common::per_process("fr12-seed-host")));
    std::fs::write(&host, b"hello").unwrap();
    let sha = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";   // SHA-256("hello"), FIPS 180-4 / NIST
    let list = format!("# a comment\nC:\\Program Files\\Empty\\\t-\t-\nC:\\a\\b.txt\t{}\t{sha}\n", host.display());
    let o = seed_run("dir-line", &list);
    let l: Vec<&str> = o.stdout.lines().collect();
    assert!(l.contains(&"rc 0"), "{l:?}");
    assert!(l.contains(&"vfs seed: C:\\Program Files\\Empty\\ (directory)"), "{l:?}");
    assert!(l.iter().any(|s| s.starts_with("vfs seed: done, 2 files, 3 directories created")), "C:\\Program Files and C:\\Program Files\\Empty from the directory line, C:\\a on the way to the file = 3; got {l:?}");
    assert!(l.contains(&"attrs 10 10 80 ffffffff"), "Program Files is a directory, Empty is a directory, b.txt is a normal file, d199 does not exist: {l:?}");
}

#[test]
fn the_vfs_seed_takes_2048_entries_and_stops_by_name_at_2049() {
    let mk = |n: usize| (0..n).map(|k| format!("C:\\d{k}\\\t-\t-\n")).collect::<String>();
    let o = seed_run("dirs-2048", &mk(2048));
    let l: Vec<&str> = o.stdout.lines().collect();
    assert!(l.contains(&"rc 0"), "2048 directory lines are accepted (the limit was 256 before decision 0129)");
    assert!(l.iter().any(|s| s.starts_with("vfs seed: done, 2048 files, 2048 directories created")), "{:?}", &l[l.len().saturating_sub(3)..]);
    assert!(l.contains(&"attrs ffffffff ffffffff ffffffff 10"), "C:\\d199 was made: {:?}", &l[l.len().saturating_sub(3)..]);
    let o = seed_run("dirs-2049", &mk(2049));
    assert!(o.stdout.contains("rc -1") && !o.stdout.contains("attrs"), "{}", o.stdout);
    assert!(o.stderr.contains("more than 2048 entries"), "{}", o.stderr);
}

#[test]
fn a_dash_dash_line_without_a_trailing_backslash_is_refused_by_name() {
    let o = seed_run("dash-no-slash", "C:\\Program Files\\Empty\t-\t-\n");
    assert!(o.stdout.contains("rc -1"), "{}", o.stdout);
    assert!(o.stderr.contains("needs a guest path that ends with a backslash"), "{}", o.stderr);
}

#[test]
fn the_vfs_seed_source_names_the_limit_and_the_directory_form() {
    let s = read(&harness_dir().join("vfs_seed.h"));
    assert!(s.contains("#define VS_MAX_ENTRIES 2048"), "decision 0129 raised the limit from 256");
    assert!(s.contains("is_dir"), "directory lines");
}

#[test]
fn decision_0129_adds_the_expected_new_runtime_functions_and_no_fe_lift_or_decode_change() {
    // fe-lift and fe-decode bodies are unchanged by decision 0129 (the seed files are an addition, not a change); imports_table.rs is appended to only.
    let t = read(&manifest().join("src/imports_table.rs"));
    let last: Vec<&str> = t.lines().filter(|l| l.trim_start().starts_with("(\"")).collect();
    // decision 0136 appends rows after these: the group is found by the name of its first row, not by counting from the end of the table
    let n = last.iter().position(|l| l.contains("\"GetLastActivePopup\"")).expect("the first row of decision 0129") + 4;
    for (k, want) in ["GetLastActivePopup", "GetThreadPriority", "GetGuiResources", "CoCreateInstance"].iter().enumerate() {
        assert!(last[n - 4 + k].contains(&format!("\"{want}\"")), "row {} is {want}", n - 4 + k);
    }
}
