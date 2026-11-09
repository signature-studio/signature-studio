//! Tests of decision 0125 (docs/decisions/0125-window-dc-and-walls.md, PROVISIONAL): walls 52 to 55 of CreateFruityInstance and the state-only DCs of a window.
//!   wall 52: gdi32 attribute calls with hdc NULL fail (the guest's TBitmap canvas without a DC calls SetWindowOrgEx(0, ...));
//!   the DCs of a window: user32!GetDC(hWnd), GetWindowDC, GetDCEx, ReleaseDC(hWnd, hDC), BeginPaint / EndPaint on the window's own DC (private, class and common DCs by the class style; one surface per top-level window);
//!   wall 53: Blend2D is told not to JIT in the FRUITY configuration (the annotation of decision 0120); wall 54: user32!LoadCursorW(hInstance, name) reads the RT_GROUP_CURSOR of the module by name or ordinal;
//!   wall 55 (stop condition (d)): the first drawing call at a DC whose bitmap keeps no pixels, and at a window DC, stops by name.
//!
//! Expected values are written here from the Microsoft Learn pages named in the decision (fetched 2026-10-31, kept in lift/out-fruity10/learn/ with index.tsv) and from the choices of the decision (each choice
//! without a source is named S-... in the decision document), not read back from the C code. EVERY ANSWER IS PROVISIONAL; nothing is compared with a real Windows. A test that pins a choice without a source says so
//! in its comment: it fixes the choice, not a Windows rule.
//!
//! 1. Generator: the 2 rows of decision 0125 are the last rows, no pseudo-VA moved.
//! 2. Wall 52: hdc NULL fails in the 14 attribute calls; any other non-DC still stops.
//! 3. The DCs of a window: GetDC / GetWindowDC / GetDCEx / ReleaseDC / BeginPaint.
//! 4. Wall 54: LoadCursorW with a module.
//! 5. Wall 55 and the first drawing call at a window DC.
//! 6. Source checks.

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
    if let Ok(d) = std::env::var("FLE_FR10_SRC_DIR") {
        return PathBuf::from(d);
    }
    rt_dir()
}
fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-wallFR10/test");
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


/// The 2 rows of decision 0125 in the order given (decision 0015: append only).
const NEW_ROWS: [(&str, &str); 2] = [("user32.dll", "GetWindowDC"), ("user32.dll", "GetDCEx")];

#[test]
fn the_2_new_rows_are_the_last_rows_in_the_order_given_and_no_pseudo_va_moved() {
    let (st, c) = gen(&GUEST_IMPORTS);
    let rows = resp_rows(&c);
    assert_eq!(rows.len(), st.resp_entries);
    // decision 0126 (old: `let n = rows.len();`) appended 33 more rows (tests/wall_fr11.rs): the 2 of decision 0125 end at n - 33.
    // decision 0129 (old: `rows.len() - 33`) appended 4 more rows (tests/wall_fr12.rs): they end at n - 37.
    // decision 0136: rows are appended at the END again (and decision 0132 appends more): the end of the group is found by the NAME of its last row, not by counting from the end of the table.
    let last = NEW_ROWS[NEW_ROWS.len() - 1];
    let n = rows.iter().position(|r| (r.0.as_str(), r.1.as_str()) == last).expect("the last row of the group") + 1;
    // decision 0015: append only. The row before the 2 is the last row of decision 0123, gdi32!PtInRegion; HEAD 4ff4f33 has 469 + 73 = 542 rows
    assert_eq!((rows[n - 3].0.as_str(), rows[n - 3].1.as_str()), ("gdi32.dll", "PtInRegion"));
    assert_eq!(n, 469 + 73 + 2, "the 542 rows of HEAD 4ff4f33 and the 2 of decision 0125");
    for (k, (dll, name)) in NEW_ROWS.iter().enumerate() {
        assert_eq!((rows[n - 2 + k].0.as_str(), rows[n - 2 + k].1.as_str()), (*dll, *name), "row {}", n - 2 + k);
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
        let (st, c) = gen(&[(dll, name), ("kernel32.dll", name), (dll, "SendMessageA"), ("kernel32.dll", "GetProcAddress")]);
        assert_eq!(st.handled[format!("jmp_imp:{name}").as_str()], 1, "{dll}!{name}");
        assert_eq!(c.matches(&format!("fle_imp_{name}(); return;")).count(), 1, "{dll}!{name}: only the thunk of its own dll is answered");
        for k in [1u64, 2] {
            let va = IB + 0x1040 + 0x10 * k;
            assert!(c.contains(&format!("L_{va:x}: {{ /* {va:#x} Jmp */ fle_unsupported(UINT64_C({va:#x})); }}")), "{dll}!{name}: thunk {k} stays unsupported");
        }
    }
}

// ---------------------------------------------------------------------------------------------
// 2. The driver: the real runtime files and a stub guest window procedure that logs every message
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
        let dir = work_dir().join(etxt_common::per_process("obj-fr10"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut v = Vec::new();
        let ours = ["fle_thread.c", "fle_win.c", "fle_class.c", "fle_gdi.c", "fle_gdc.c", "fle_cursor.c", "fle_res.c", "fle_vmap.c", "fle_vfs.c", "fle_raster.c", "fle_monitor.c", "fle_ole.c", "fle_modules.c"];
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
// 2. Wall 52: an attribute call with hdc NULL fails (S-hdc-null)
// ---------------------------------------------------------------------------------------------

#[test]
fn the_14_attribute_calls_with_hdc_null_fail_with_zero_and_the_last_error_6_and_write_nothing() {
    // Learn SetWindowOrgEx: "If the function fails, the return value is zero." (the same sentence on the pages of the other 13); the page does not call a NULL hdc a failure and names no GetLastError value: the failure is a
    // READING and ERROR_INVALID_HANDLE (6, System Error Codes "The handle is invalid.") is the choice S-hdc-null. No POINT is written ("receives the previous origin": there is none).
    let l = lines(
        "fr10-hdc-null",
        r#"
    uint32_t pt[4] = { 0x11111111, 0x22222222, 0x33333333, 0x44444444 };
    struct { const char *n; void (*f)(void); } c[] = {
        { "MoveToEx", fle_imp_MoveToEx }, { "GetCurrentPositionEx", fle_imp_GetCurrentPositionEx }, { "SetBkMode", fle_imp_SetBkMode }, { "SetROP2", fle_imp_SetROP2 },
        { "SetMapMode", fle_imp_SetMapMode }, { "SetStretchBltMode", fle_imp_SetStretchBltMode }, { "GetStretchBltMode", fle_imp_GetStretchBltMode }, { "SetWindowOrgEx", fle_imp_SetWindowOrgEx },
        { "GetWindowOrgEx", fle_imp_GetWindowOrgEx }, { "SetViewportOrgEx", fle_imp_SetViewportOrgEx }, { "LPtoDP", fle_imp_LPtoDP }, { "SetBrushOrgEx", fle_imp_SetBrushOrgEx },
        { "GetBrushOrgEx", fle_imp_GetBrushOrgEx }, { "SaveDC", fle_imp_SaveDC }, { "RestoreDC", fle_imp_RestoreDC },
    };
    int k;
    for (k = 0; k < 15; k++) {
        uint64_t r;
        SETLE(0x1234);
        pt[0] = 0x11111111;
        /* every call that takes a POINT pointer gets one: the call must not write it. (rdx: x, or the POINT of the Get calls; r9: the POINT of the Set*OrgEx / MoveToEx calls) */
        R[4] -= 8; fle_st(R[4], 8, 0xAAAA0000);
        R[1] = 0; R[2] = (k == 1 || k == 8) ? A(pt) : 1; R[8] = 1; R[9] = A(pt);
        if (k == 10) { R[2] = A(pt); R[8] = 1; }
        c[k].f();
        R[4] += 8;
        r = R[0];
        printf("%s: ret %llu le %u pt %s\n", c[k].n, U(r), LE(), pt[0] == 0x11111111 ? "untouched" : "WRITTEN");
    }
"#,
    );
    let names = ["MoveToEx", "GetCurrentPositionEx", "SetBkMode", "SetROP2", "SetMapMode", "SetStretchBltMode", "GetStretchBltMode", "SetWindowOrgEx", "GetWindowOrgEx", "SetViewportOrgEx", "LPtoDP", "SetBrushOrgEx", "GetBrushOrgEx", "SaveDC", "RestoreDC"];
    assert_eq!(l.len(), 15, "{l:?}");
    for (line, n) in l.iter().zip(names) {
        assert_eq!(line, &format!("{n}: ret 0 le 6 pt untouched"));
    }
}

#[test]
fn a_handle_that_is_not_a_dc_and_not_null_still_stops_in_the_attribute_calls() {
    // only NULL is a failure: a number that this runtime never issued, a released DC and a bitmap handle are not defined by the pages and stop as before (decision 0111)
    for (label, setup) in [("never-issued", "uint64_t h = 0x1234;"), ("bitmap", "uint64_t h = callf(fle_imp_CreateBitmap, 2, 2, 1, 1);"), ("released", "uint64_t h = callf(fle_imp_GetDC, 0, 0, 0, 0); callf(fle_imp_ReleaseDC, 0, h, 0, 0);")] {
        aborts(&format!("fr10-notdc-{label}"), &format!("    {setup}\n    callf(fle_imp_SetWindowOrgEx, h, 1, 1, 0);"), "SetWindowOrgEx", "not a DC of this runtime");
    }
}

#[test]
fn the_guest_call_of_wall_52_is_a_set_window_org_with_hdc_zero_and_a_previous_origin_pointer() {
    // f_820440 at 0x8204ce: SetWindowOrgEx(0, -x, -y, &prev) and, at 0x82057f, SetWindowOrgEx(0, prev.x, prev.y, NULL): both with hdc 0 and the result not read (measured by gdb and read from the instructions)
    let l = lines(
        "fr10-wall52",
        r#"
    int32_t prev[2] = { 77, 88 };
    uint64_t a = callf(fle_imp_SetWindowOrgEx, 0, (uint32_t)-5, (uint32_t)-6, A(prev));
    uint64_t b = callf(fle_imp_SetWindowOrgEx, 0, 5, 6, 0);
    printf("%llu %llu prev %d %d\n", U(a), U(b), prev[0], prev[1]);
"#,
    );
    assert_eq!(l, ["0 0 prev 77 88"]);
}

// ---------------------------------------------------------------------------------------------
// 3. The DCs of a window (state only)
// ---------------------------------------------------------------------------------------------

#[test]
fn a_common_dc_is_new_on_every_call_is_released_by_releasedc_and_its_attributes_start_again() {
    // Learn GetDC: "for common DCs, GetDC assigns default attributes to the DC each time it is retrieved"; "After painting with a common DC, the ReleaseDC function must be called to release the DC"; "The number of DCs
    // is limited only by available memory". Display Device Context Defaults: "Changes to the attribute values of a common ... device context are not permanent. When an application releases these device contexts, the
    // current selections, such as mapping mode and clipping region, are lost". Handle values: S (the screen DC region, never reused).
    let l = lines(
        "fr10-common",
        r#"
    uint64_t w = cw(0x80, "w", POPUP | VIS, 10, 20, 300, 200, 0, 0, 0);
    uint64_t d1 = callf(fle_imp_GetDC, w, 0, 0, 0), d2 = callf(fle_imp_GetDC, w, 0, 0, 0);
    int32_t o[13];
    fle_gdc_info gi;
    printf("two calls two handles %d, both live %llu\n", d1 != d2 && d1 != 0 && d2 != 0, U(fle_gdc_live_count()));
    printf("info %d kind %u hwnd is w %d thread %u client %d x %d\n", fle_gdc_dc_info(d1, &gi), gi.kind, gi.hwnd == w, gi.thread, gi.cx, gi.cy);
    callf(fle_imp_SetMapMode, d1, 3, 0, 0);
    fle_gdi_screen_attr(d1, o);
    printf("map mode of d1 %d, of d2 %d\n", o[2], (fle_gdi_screen_attr(d2, o), o[2]));
    printf("release d1 %llu again %llu\n", U(callf(fle_imp_ReleaseDC, w, d1, 0, 0)), U(callf(fle_imp_ReleaseDC, w, d1, 0, 0)));
    printf("live %llu info of d1 %d of d2 %d\n", U(fle_gdc_live_count()), fle_gdc_dc_info(d1, &gi), fle_gdc_dc_info(d2, &gi));
    { uint64_t d3 = callf(fle_imp_GetDC, w, 0, 0, 0); fle_gdi_screen_attr(d3, o); printf("a new GetDC starts at MM_TEXT: %d, never reused %d\n", o[2], d3 != d1 && d3 != d2); }
"#,
    );
    assert_eq!(
        l,
        [
            "two calls two handles 1, both live 2",
            "info 1 kind 1 hwnd is w 1 thread 0 client 300 x 200",
            "map mode of d1 3, of d2 1",
            "release d1 1 again 0",
            "live 1 info of d1 0 of d2 1",
            "a new GetDC starts at MM_TEXT: 1, never reused 1",
        ]
    );
}

#[test]
fn a_private_dc_is_one_per_window_and_keeps_its_attributes_and_a_class_dc_is_one_per_class() {
    // Learn Window Class Styles: CS_OWNDC "Allocates a unique device context for each window in the class"; CS_CLASSDC "Allocates one device context to be shared by all windows in the class". Learn GetDC: "For class and
    // private DCs, GetDC leaves the previously assigned attributes unchanged"; "Class and private DCs do not have to be released". S-release-owndc: ReleaseDC of such a DC returns 1 and the DC stays (the page says
    // nothing of what it does to one).
    let l = lines(
        "fr10-own-class",
        r#"
    uint64_t a, b, c1, c2, d, e;
    int32_t o[13];
    uint64_t w1, w2, c, x;
    regclass_style(cls_own, 0x20);
    regclass_style(cls_cdc, 0x40);
    w1 = cw_cls(cls_own, 0x80, POPUP, 0, 0, 100, 50, 0);
    w2 = cw_cls(cls_own, 0x80, POPUP, 0, 0, 100, 50, 0);
    a = callf(fle_imp_GetDC, w1, 0, 0, 0);
    b = callf(fle_imp_GetDC, w1, 0, 0, 0);
    c1 = callf(fle_imp_GetDC, w2, 0, 0, 0);
    printf("private: same window same DC %d, other window another DC %d\n", a == b, a != c1);
    callf(fle_imp_SetMapMode, a, 3, 0, 0);
    printf("release: %llu, still live %d, attributes kept %d\n", U(callf(fle_imp_ReleaseDC, w1, a, 0, 0)), fle_gdc_is_live(a), (fle_gdi_screen_attr(callf(fle_imp_GetDC, w1, 0, 0, 0), o), o[2]));
    c = cw_cls(cls_cdc, 0x80, POPUP, 0, 0, 10, 10, 0);
    x = cw_cls(cls_cdc, 0x80, POPUP, 0, 0, 20, 20, 0);
    c2 = callf(fle_imp_GetDC, c, 0, 0, 0);
    d = callf(fle_imp_GetDC, x, 0, 0, 0);
    e = callf(fle_imp_GetDC, c, 0, 0, 0);
    printf("class: two windows one DC %d, same again %d, differs from the private ones %d\n", c2 == d, c2 == e, c2 != a && c2 != c1);
    { fle_gdc_info gi; fle_gdc_dc_info(a, &gi); printf("info kind %u, lid %u root %u\n", gi.kind, gi.lid, gi.root_lid); }
    printf("destroying w1 takes its private DC with it: %llu, live %d, the class DC stays %d\n", U(callf(fle_imp_DestroyWindow, w1, 0, 0, 0)), fle_gdc_is_live(a), fle_gdc_is_live(c2));
"#,
    );
    assert_eq!(l[0], "private: same window same DC 1, other window another DC 1");
    assert_eq!(l[1], "release: 1, still live 1, attributes kept 3");
    assert_eq!(l[2], "class: two windows one DC 1, same again 1, differs from the private ones 1");
    assert!(l[3].starts_with("info kind 2, lid "), "{}", l[3]);
    assert_eq!(l[4], "destroying w1 takes its private DC with it: 1, live 0, the class DC stays 1");
}

#[test]
fn a_child_window_draws_on_the_surface_of_its_top_level_window_and_an_owned_window_on_its_own() {
    // R10 of docs/investigations/detach-window.md: one drawing surface per top-level window (the logical id of the top-level ancestor, so that a re-created HWND does not orphan the surface: R5). Learn Window
    // Features: an owned window is a top-level window with its own ... (it is not a child): its surface is its own.
    let l = lines(
        "fr10-surface",
        r#"
    uint64_t top = cw(0x80, "t", POPUP | VIS, 0, 0, 400, 300, 0, 0, 0);
    uint64_t kid = cw(0, "k", CHILD | VIS, 5, 6, 100, 50, top, 1, 0);
    uint64_t kk = cw(0, "kk", CHILD | VIS, 1, 1, 10, 10, kid, 2, 0);
    uint64_t own = cw(0x80, "o", POPUP, 0, 0, 30, 30, top, 0, 0);
    fle_gdc_info a, b, c, d;
    uint64_t da = callf(fle_imp_GetDC, top, 0, 0, 0), db = callf(fle_imp_GetDC, kid, 0, 0, 0), dc = callf(fle_imp_GetDC, kk, 0, 0, 0), dd = callf(fle_imp_GetDC, own, 0, 0, 0);
    fle_gdc_dc_info(da, &a); fle_gdc_dc_info(db, &b); fle_gdc_dc_info(dc, &c); fle_gdc_dc_info(dd, &d);
    printf("top %u kid %u kk %u owned %u\n", a.root_lid, b.root_lid, c.root_lid, d.root_lid);
    printf("kid and grandchild draw on the top's surface %d %d, the owned window has its own %d\n", b.root_lid == a.lid, c.root_lid == a.lid, d.root_lid == d.lid && d.root_lid != a.lid);
    printf("sizes %d x %d, %d x %d, %d x %d\n", a.cx, a.cy, b.cx, b.cy, c.cx, c.cy);
"#,
    );
    assert_eq!(l[1], "kid and grandchild draw on the top's surface 1 1, the owned window has its own 1");
    assert_eq!(l[2], "sizes 400 x 300, 100 x 50, 10 x 10");
}

#[test]
fn getdc_and_releasedc_of_a_wrong_window_or_thread_follow_the_pages_failure_values() {
    // Learn GetDC: "If the function fails, the return value is NULL". An hWnd that is not a window of the table fails (1400, S-err: a choice from System Error Codes). Learn ReleaseDC: "If the DC was not released, the
    // return value is zero"; "ReleaseDC must be called from the same thread that called GetDC" (the page does not say what happens otherwise: S-release-thread, 0). S-release-mismatch: a hWnd that is not the window the
    // DC was got for gives 0 and the DC stays.
    let l = lines(
        "fr10-getdc-fail",
        r#"
    uint64_t w = cw(0x80, "w", POPUP | VIS, 0, 0, 10, 10, 0, 0, 0), v = cw(0x80, "v", POPUP | VIS, 0, 0, 10, 10, 0, 0, 0);
    uint64_t d;
    SETLE(0);
    printf("never issued: %llu le %u\n", U(callf(fle_imp_GetDC, 0x7F991000, 0, 0, 0)), LE());
    SETLE(0);
    printf("desktop window: %d le %u\n", callf(fle_imp_GetDC, FLE_DESKTOP_HANDLE, 0, 0, 0) != 0, LE());
    d = callf(fle_imp_GetDC, w, 0, 0, 0);
    printf("release with the wrong window: %llu, still live %d\n", U(callf(fle_imp_ReleaseDC, v, d, 0, 0)), fle_gdc_is_live(d));
    printf("release with NULL window: %llu, still live %d\n", U(callf(fle_imp_ReleaseDC, 0, d, 0, 0)), fle_gdc_is_live(d));
    printf("release: %llu\n", U(callf(fle_imp_ReleaseDC, w, d, 0, 0)));
    printf("release NULL dc: %llu\n", U(callf(fle_imp_ReleaseDC, w, 0, 0, 0)));
"#,
    );
    assert_eq!(l[0], "never issued: 0 le 1400");
    assert_eq!(l[1], "desktop window: 1 le 0");
    assert_eq!(l[2], "release with the wrong window: 0, still live 1");
    assert_eq!(l[3], "release with NULL window: 0, still live 1");
    assert_eq!(l[4], "release: 1");
    assert_eq!(l[5], "release NULL dc: 0");
}

#[test]
fn getwindowdc_is_a_common_dc_of_the_whole_window_and_getdcex_follows_its_flags() {
    // Learn GetWindowDC: "the device context for the entire window, including title bar, menus, and scroll bars" (S-windc-common: a new handle each call; the page does not mention class styles); hWnd NULL "the device
    // context for the entire screen". Learn GetDCEx: DCX_WINDOW "a DC that corresponds to the window rectangle rather than the client rectangle"; DCX_CACHE "a DC from the cache, rather than the OWNDC or CLASSDC
    // window. Essentially overrides CS_OWNDC and CS_CLASSDC"; DCX_NORESETATTRS "This flag is ignored"; DCX_INTERSECTRGN / DCX_EXCLUDERGN need the clip region (a stop here). The numbers are windows-sys's (UNOFFICIAL).
    let l = lines(
        "fr10-getwindowdc",
        r#"
    uint64_t w, o, a, b, c, d, e, f, g;
    fle_gdc_info i1, i2;
    regclass_style(cls_own, 0x20);
    w = cw(0x80, "w", POPUP | VIS, 0, 0, 50, 40, 0, 0, 0);
    o = cw_cls(cls_own, 0x80, POPUP, 0, 0, 20, 20, 0);
    a = callf(fle_imp_GetWindowDC, w, 0, 0, 0); b = callf(fle_imp_GetWindowDC, w, 0, 0, 0);
    fle_gdc_dc_info(a, &i1);
    printf("window dc: two handles %d, kind %u\n", a != b, i1.kind);
    c = callf(fle_imp_GetWindowDC, 0, 0, 0, 0); fle_gdc_dc_info(c, &i2);
    printf("NULL: the screen, kind %u hwnd %llu\n", i2.kind, U(i2.hwnd));
    d = callf(fle_imp_GetDCEx, o, 0, 0, 0); e = callf(fle_imp_GetDCEx, o, 0, 0, 0);
    printf("GetDCEx(0) on a CS_OWNDC window is its private DC %d (kind %u)\n", d == e, (fle_gdc_dc_info(d, &i1), i1.kind));
    f = callf(fle_imp_GetDCEx, o, 0, 2, 0);
    printf("DCX_CACHE overrides CS_OWNDC: another handle %d, kind %u\n", f != d, (fle_gdc_dc_info(f, &i1), i1.kind));
    g = callf(fle_imp_GetDCEx, o, 0, 1, 0);
    printf("DCX_WINDOW: kind %u\n", (fle_gdc_dc_info(g, &i1), i1.kind));
    g = callf(fle_imp_GetDCEx, o, 0, 0x10004 | 0x400, 0);
    printf("the guest's 0x10001 and the ignored/known bits do not stop: %d\n", g != 0);
    printf("ReleaseDC of a window DC: %llu\n", U(callf(fle_imp_ReleaseDC, w, a, 0, 0)));
"#,
    );
    assert_eq!(l[0], "window dc: two handles 1, kind 4");
    assert_eq!(l[1], "NULL: the screen, kind 0 hwnd 0");
    assert_eq!(l[2], "GetDCEx(0) on a CS_OWNDC window is its private DC 1 (kind 2)");
    assert_eq!(l[3], "DCX_CACHE overrides CS_OWNDC: another handle 1, kind 1");
    assert_eq!(l[4], "DCX_WINDOW: kind 4");
    assert_eq!(l[5], "the guest's 0x10001 and the ignored/known bits do not stop: 1");
    assert_eq!(l[6], "ReleaseDC of a window DC: 1");
}

#[test]
fn getdcex_with_a_clip_region_flag_stops() {
    // Learn GetDCEx: "If the value of flags is DCX_INTERSECTRGN or DCX_EXCLUDERGN, then the operating system assumes ownership of the region"; the clip region is not modelled (decision 0123 stops on every clip call)
    for (label, fl) in [("intersect", 0x80u32), ("exclude", 0x40)] {
        let o = aborts(&format!("fr10-getdcex-{label}"), &format!("    uint64_t w = cw(0x80, \"w\", POPUP, 0, 0, 5, 5, 0, 0, 0);\n    callf(fle_imp_GetDCEx, w, 0, {fl}, 0);"), "GetDCEx", "clip region");
        assert!(o.stderr.contains("not modelled"), "{label}: {}", o.stderr);
    }
}

#[test]
fn beginpaint_gives_the_dc_of_the_window_and_endpaint_releases_it() {
    // Learn BeginPaint: "a handle to a display device context for the specified window"; EndPaint "releases the display device context that BeginPaint retrieved". S-paintdc is now the window's own DC (decision 0123
    // had the DC of GetDC(NULL)). A CS_OWNDC window gets its private DC (EndPaint does not remove it).
    let l = lines(
        "fr10-beginpaint",
        r#"
    uint64_t w, o, dc, dc2;
    uint8_t ps[72];
    fle_gdc_info gi;
    regclass_style(cls_own, 0x20);
    w = cw(0x80, "w", POPUP, 0, 0, 80, 60, 0, 0, 0);
    o = cw_cls(cls_own, 0x80, POPUP, 0, 0, 20, 20, 0);
    callf(fle_imp_InvalidateRect, w, 0, 1, 0);
    dc = callf(fle_imp_BeginPaint, w, A(ps), 0, 0);
    fle_gdc_dc_info(dc, &gi);
    printf("BeginPaint: hdc in PAINTSTRUCT %d, kind %u, hwnd is w %d, live %llu\n", *(uint64_t *)ps == dc, gi.kind, gi.hwnd == w, U(fle_gdc_live_count()));
    printf("EndPaint %llu, live %llu\n", U(callf(fle_imp_EndPaint, w, A(ps), 0, 0)), U(fle_gdc_live_count()));
    callf(fle_imp_InvalidateRect, o, 0, 1, 0);
    dc = callf(fle_imp_BeginPaint, o, A(ps), 0, 0);
    callf(fle_imp_EndPaint, o, A(ps), 0, 0);
    callf(fle_imp_InvalidateRect, o, 0, 1, 0);
    dc2 = callf(fle_imp_BeginPaint, o, A(ps), 0, 0);
    callf(fle_imp_EndPaint, o, A(ps), 0, 0);
    fle_gdc_dc_info(dc, &gi);
    printf("CS_OWNDC window: the same DC both times %d, kind %u, still live %d\n", dc == dc2, gi.kind, fle_gdc_is_live(dc));
"#,
    );
    assert_eq!(l[0], "BeginPaint: hdc in PAINTSTRUCT 1, kind 1, hwnd is w 1, live 1");
    assert_eq!(l[1], "EndPaint 1, live 0");
    assert_eq!(l[2], "CS_OWNDC window: the same DC both times 1, kind 2, still live 1");
}

#[test]
fn a_common_dc_of_a_destroyed_window_stays_until_it_is_released() {
    // S-common-after-destroy (a choice without a source): Learn does not say what happens to a common DC when its window is destroyed before ReleaseDC. The DC stays live; ReleaseDC with the old hWnd (the window is
    // gone: its handle is never reused) releases it. A DestroyWindow does not release for the guest.
    let l = lines(
        "fr10-destroyed",
        r#"
    uint64_t w = cw(0x80, "w", POPUP, 0, 0, 5, 5, 0, 0, 0), d = callf(fle_imp_GetDC, w, 0, 0, 0);
    callf(fle_imp_DestroyWindow, w, 0, 0, 0);
    printf("live after DestroyWindow %d\n", fle_gdc_is_live(d));
    printf("release %llu, live %d\n", U(callf(fle_imp_ReleaseDC, w, d, 0, 0)), fle_gdc_is_live(d));
"#,
    );
    assert_eq!(l, ["live after DestroyWindow 1", "release 1, live 0"]);
}

// ---------------------------------------------------------------------------------------------
// 4. Wall 54: LoadCursorW(hInstance, name) with the cursors of a module
// ---------------------------------------------------------------------------------------------

/// A driver that maps the real FLEngine image (the loader of the harness) and calls LoadCursorW on it.
fn real_dll() -> Option<PathBuf> {
    let p = std::env::var("FLENGINE_DLL").map(PathBuf::from).unwrap_or_else(|_| manifest().join("../../../binaries/win64/FLEngine_x64.dll"));
    p.exists().then_some(p)
}

fn cursor_objs() -> &'static Vec<PathBuf> {
    static O: OnceLock<Vec<PathBuf>> = OnceLock::new();
    O.get_or_init(|| {
        let dir = work_dir().join(etxt_common::per_process("obj-fr10-cur"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut v = Vec::new();
        for (name, ours) in [
            ("fle_cursor.c", true), ("fle_res.c", true), ("fle_loadimage.c", true), ("fle_gdi.c", true), ("fle_raster.c", true), ("fle_gdc.c", true), ("fle_imports.c", false), ("fle_env.c", false), ("fle_modules.c", false),
            ("fle_realmod.c", false), ("fle_thread.c", false), ("fle_win.c", false), ("fle_class.c", false), ("fle_sys.c", false), ("fle_event.c", false), ("fle_proc.c", false), ("fle_callback.c", false),
            ("fle_vfs.c", false), ("fle_vmap.c", false), ("fle_vol.c", false), ("fle_host.c", false), ("fle_time.c", false), ("fle_x87ctl.c", false), ("fle_mxcsr.c", false), ("fle_x87.c", false), ("fle_locale.c", false),
        ] {
            let base = if ours { src_dir() } else { rt_dir() };
            let o = dir.join(name.replace(".c", ".o"));
            clang_c(&base.join(name), &o, false);
            v.push(o);
        }
        let image = dir.join("image.o");
        let cc = Command::new("clang")
            .args(["-O0", "-std=gnu11", "-w", "-fPIE", "-c", "-I"])
            .arg(rt_dir())
            .arg("-I")
            .arg(manifest().join("../../harness"))
            .arg(manifest().join("../../harness/image.c"))
            .arg("-o")
            .arg(&image)
            .env("TMPDIR", work_dir().join("tmp"))
            .output()
            .expect("clang not found");
        assert!(cc.status.success(), "clang failed for image.c: {}", String::from_utf8_lossy(&cc.stderr));
        v.push(image);
        v
    })
}

fn cursor_lines(name: &str, dll: &Path, body: &str) -> (bool, Option<i32>, String, String) {
    let dir = work_dir();
    let uname = etxt_common::uniq(name);
    let c = dir.join(format!("{uname}.c"));
    let exe = dir.join(&uname);
    let text = format!(
        r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <sys/resource.h>
#include "fle_runtime.h"
#include "fle_dispatch.h"
#include "fle_sse.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_gdc.h"
#include "fle_gdi.h"
#include "fle_cursor.h"
#include "fle_res.h"
#include "fle_loadimage.h"
#include "fle_x87ctl.h"
#include "fle_mxcsr.h"
#include "boot.h"
uint64_t R[16];
fle_eflags FL;
fle_xmm X[16];
const fle_fn_entry fle_dispatch_table[] = {{ {{ 0x2000, 0 }} }};
const uint64_t fle_dispatch_count = 1;
static uint64_t stk[0x400];
#define U(x) ((unsigned long long)(x))
static uint32_t LE(void) {{ fle_imp_GetLastError(); return (uint32_t)R[0]; }}
static void SETLE(uint32_t v) {{ fle_set_last_error(v); }}
static uint16_t NAME[8][64];
static unsigned NK;
static uint64_t S(const char *s) {{ uint16_t *p = NAME[NK++ % 8]; size_t i, n = strlen(s); for (i = 0; i < n; i++) p[i] = (uint8_t)s[i]; p[n] = 0; return (uint64_t)(uintptr_t)p; }}
static uint64_t LC(uint64_t inst, uint64_t name) {{ memset(stk, 0, sizeof stk); R[1] = inst; R[2] = name; R[4] = (uint64_t)(uintptr_t)stk; stk[0] = 0xdead0000u; fle_imp_LoadCursorW(); return R[0]; }}
__attribute__((constructor)) static void nocore(void) {{ struct rlimit r = {{ 0, 0 }}; setrlimit(RLIMIT_CORE, &r); }}
int main(int argc, char **argv)
{{
    harness_image img;
    char err[256];
    (void)argc;
    if (image_load(argv[1], &img, err, sizeof err) != 0) {{ fprintf(stderr, "load: %s\n", err); return 3; }}
    stk[0] = 0;
{body}
    return 0;
}}
"#
    );
    std::fs::write(&c, text).unwrap();
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fPIE", "-pthread", "-fno-builtin", "-I"]).arg(src_dir()).arg("-I").arg(rt_dir()).arg("-I").arg(manifest().join("../../harness"));
    for o in cursor_objs() {
        cmd.arg(o);
    }
    let cc = cmd.arg("-o").arg(&exe).arg(&c).arg("-lm").env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new(&exe).arg(dll).env("TMPDIR", dir.join("tmp")).output().expect("run failed");
    (r.status.success(), r.status.signal(), String::from_utf8_lossy(&r.stdout).to_string(), String::from_utf8_lossy(&r.stderr).to_string())
}

#[test]
fn loadcursorw_with_the_module_finds_the_named_cursor_resources_of_the_guest_and_gives_stable_handles() {
    // Learn LoadCursorW: "If hInstance is non-NULL, lpCursorName specifies the cursor resource either by name or ordinal"; "loads the cursor resource only if it has not been loaded; otherwise, it retrieves the handle to the
    // existing resource" (the same name gives the same handle). The 51 named RT_GROUP_CURSOR entries of FLEngine_x64.dll were counted from the PE resource directory with pefile (ADDPOINT ... VT_VERTSPLIT) and the
    // 7 ordinals 32761..32767. Handle values: S-loadcursor-handle (a choice: the page gives none).
    let Some(dll) = real_dll() else { return };
    let (ok, sig, out, err) = cursor_lines(
        "fr10-lc",
        &dll,
        r#"
    uint64_t a, b, c, d, e;
    a = LC(0x400000, S("VT_HEADERSPLIT")); b = LC(0x400000, S("VT_HEADERSPLIT")); c = LC(0x400000, S("vt_headersplit")); d = LC(0x400000, S("ADDPOINT")); e = LC(0x400000, S("VT_VERTSPLIT"));
    printf("same name same handle %d, case folded %d, other names differ %d %d, nonzero %d\n", a == b, a == c, a != d, d != e, a != 0 && d != 0 && e != 0);
    printf("shared form %d %d %d\n", fle_cursor_is_shared_handle(a), fle_cursor_is_shared_handle(d), fle_cursor_is_shared_handle(e));
    printf("ordinal 32762 is the handle LoadImageW gives: %d\n", LC(0x400000, 32762) == fle_loadimage_module_handle_of(32762));
    SETLE(0x1234);
    printf("a name that is not there: %llu le %u\n", U(LC(0x400000, S("NOSUCHCURSOR"))), LE());
    SETLE(0x1234);
    printf("the success leaves the last error: %llu le %u\n", U(LC(0x400000, S("MOVE")) != 0), LE());
    fflush(stdout);
    printf("an ordinal not there: %llu\n", U(LC(0x400000, 32649)));
"#,
    );
    // 32649 is a stop (no guest call loads another ordinal: "an ordinal other than 32762..32767")
    assert!(!ok && sig == Some(6), "{out} {err}");
    assert!(err.contains("fle: LoadCursorW: lpCursorName is an ordinal other than 32762..32767"), "{err}");
    let l: Vec<&str> = out.lines().collect();
    assert_eq!(l[0], "same name same handle 1, case folded 1, other names differ 1 1, nonzero 1");
    assert_eq!(l[1], "shared form 1 1 1");
    assert_eq!(l[2], "ordinal 32762 is the handle LoadImageW gives: 1");
    assert_eq!(l[3], "a name that is not there: 0 le 1814");
    assert_eq!(l[4], "the success leaves the last error: 1 le 4660");
}

#[test]
fn loadcursorw_with_null_hinstance_is_unchanged_and_other_modules_stop() {
    let Some(dll) = real_dll() else { return };
    let (ok, sig, out, err) = cursor_lines(
        "fr10-lc-stops",
        &dll,
        r#"
    printf("IDC_ARROW %d\n", LC(0, 32512) == fle_cursor_handle_of(32512));
    fflush(stdout);
    LC(0x1234, S("MOVE"));
"#,
    );
    assert!(!ok && sig == Some(6), "{out} {err}");
    assert_eq!(out.lines().next(), Some("IDC_ARROW 1"));
    assert!(err.contains("fle: LoadCursorW: ") && err.contains("not a module handle of this runtime"), "{err}");
}

#[test]
fn setcursor_takes_a_cursor_loaded_from_a_module_by_loadcursorw() {
    // Learn SetCursor: "The cursor must have been created by either the CreateCursor or the CreateIconIndirect function, or loaded by either the LoadCursor or the LoadImage function"
    let Some(dll) = real_dll() else { return };
    let (ok, _sig, out, err) = cursor_lines(
        "fr10-setcursor",
        &dll,
        r#"
    uint64_t h = LC(0x400000, S("VT_MOVEALL")), prev;
    R[1] = h; R[4] = (uint64_t)(uintptr_t)stk; stk[0] = 0xdead0000u; fle_imp_SetCursor(); prev = R[0];
    printf("SetCursor of the module cursor: previous %llu\n", U(prev));
    R[1] = h; fle_imp_DestroyCursor();
    printf("DestroyCursor of a shared cursor: %llu\n", U(R[0]));
"#,
    );
    assert!(ok, "{out} {err}");
    assert_eq!(out.lines().collect::<Vec<_>>(), ["SetCursor of the module cursor: previous 0", "DestroyCursor of a shared cursor: 1"]);
}

// ---------------------------------------------------------------------------------------------
// 5. Wall 55 and the first drawing call at a window DC (stop condition (d))
// ---------------------------------------------------------------------------------------------

#[test]
fn the_first_drawing_call_at_the_dc_of_a_window_stops_by_name_with_the_window_and_the_surface() {
    // decision 0126 (walls 55 to 102): old: "PM's decision 0125, stop condition (d): the first real drawing call whose destination is the DC of a window stops by name" (PatBlt and FillRect at a window DC stopped with
    // "DC of a WINDOW"); the pixel model draws on the surface of the top-level window now (fle_raster.c), so the two calls answer: they return 1 and the surface of the window has the pixels (tests/wall_fr11.rs and
    // tests/raster.rs pin the pixels and the evidence of the first non-background pixel). The stop (d) of decision 0125 is replaced by the pixel model, it no longer exists. This test now pins that the calls RETURN.
    for (label, call) in [
        ("patblt", "callf6(fle_imp_PatBlt, dc, 0, 0, 5, 5, 0xF00021)"),
        ("fillrect", "(rc[2] = 5, rc[3] = 5, callf(fle_imp_FillRect, dc, A(rc), br, 0))"),
    ] {
        let l = lines(
            &format!("fr10-draw-{label}"),
            &format!("    uint64_t w = cw(0x80, \"w\", POPUP | VIS, 0, 0, 40, 30, 0, 0, 0), dc = callf(fle_imp_GetDC, w, 0, 0, 0), br = callf(fle_imp_CreateSolidBrush, 0x112233, 0, 0, 0);\n    int32_t rc[4] = {{ 0, 0, 0, 0 }};\n    printf(\"ret %llu\\n\", (unsigned long long){call});"),
        );
        assert_eq!(l.last().map(String::as_str), Some("ret 1"), "{label}: {l:?}");
    }
}

#[test]
fn drawing_into_a_device_dependent_bitmap_of_a_memory_dc_still_stops_wall_55() {
    // decision 0126 (wall 55): old name and body: FillRect into a 9x9 colour device-dependent bitmap (CreateCompatibleBitmap) stopped ("it keeps no pixels (decision 0106)": the stop (d) of decision 0125). The pixel
    // model gives a DDB a pixel block (S-ddb-lazy, S-ddb-init), so the FillRect returns 1 and the bitmap holds the colour; the 9 x 9 pixels of 0xFFFFFF are the proof (tests/wall_fr11.rs checks the bytes).
    let l = lines(
        "fr10-wall55",
        r#"
    uint64_t sdc = callf(fle_imp_GetDC, 0, 0, 0, 0), m = callf(fle_imp_CreateCompatibleDC, sdc, 0, 0, 0), bmp = callf(fle_imp_CreateCompatibleBitmap, sdc, 9, 9, 0);
    uint64_t br = callf(fle_imp_CreateSolidBrush, 0xFFFFFF, 0, 0, 0);
    int32_t rc[4] = { 0, 0, 9, 9 };
    callf(fle_imp_SelectObject, m, bmp, 0, 0);
    printf("FillRect %llu\n", (unsigned long long)callf(fle_imp_FillRect, m, A(rc), br, 0));
"#,
    );
    assert_eq!(l.last().map(String::as_str), Some("FillRect 1"), "{l:?}");
}

#[test]
fn a_drawing_call_at_the_dc_of_the_entire_screen_keeps_its_old_stop() {
    // decision 0126 (old: PatBlt at the DC of GetDC(NULL) stopped with "is a DC of GetDC(NULL)"): the screen DC draws on the surface of the desktop (lid 0, 1920 x 1080) now, so the call returns 1
    // (tests/wall_fr11.rs). The name of the test is the old one (the file keeps the order of its tests).
    let l = lines("fr10-draw-screen", "    uint64_t sdc = callf(fle_imp_GetDC, 0, 0, 0, 0);\n    printf(\"ret %llu\\n\", (unsigned long long)callf6(fle_imp_PatBlt, sdc, 0, 0, 1, 1, 0xF00021));");
    assert_eq!(l.last().map(String::as_str), Some("ret 1"), "{l:?}");
}

// ---------------------------------------------------------------------------------------------
// 6. Source checks
// ---------------------------------------------------------------------------------------------

#[test]
fn the_window_dc_model_reads_no_host_state_and_draws_no_pixel_and_names_each_choice_without_a_source() {
    let gdc = strip_comments(&std::fs::read_to_string(src_dir().join("fle_gdc.c")).unwrap());
    for bad in [
        "getpid", "getenv", "environ", "fopen", "open(", "/proc", "/etc", "/home", "/dev", "/tmp", "clock_gettime", "gettimeofday", "time(", "nanosleep", "usleep", "sleep(", "select(", "<time.h>", "<sys/time.h>", "pthread",
        "<sys/", "<unistd.h>", "<signal.h>", "fork(", "syscall", "futex", "__asm__", "abort(", "X11", "DISPLAY", "wayland", "ioctl", "dlopen", "LoadLibrary", "setjmp", "longjmp", "fle_dispatch_table",
        "BitBlt", "StretchDIBits", "PatBlt", "FillRect", "TextOut", "SetPixel", "LineTo", "Rectangle(",
    ] {
        assert!(!gdc.contains(bad), "fle_gdc.c must not use {bad}");
    }
    // the guest thread comes from fle_thread.h (a weak reference), the window table from fle_win.h (a weak reference)
    assert!(gdc.contains("fle_thread_current") && gdc.contains("fle_win_dc_query") && gdc.contains("__attribute__((weak))"));
    // the DC of a window does not own any pixel: no allocation of a pixel block (the only malloc is the table of DC records)
    assert_eq!(gdc.matches("realloc(").count(), 1);
    assert!(!gdc.contains("calloc(") && !gdc.contains("malloc("));
    // the decision names each choice
    let c = std::fs::read_to_string(src_dir().join("fle_gdc.c")).unwrap();
    for s in ["S-windc-common", "S-owndc-life", "S-common-after-destroy", "S-release-mismatch", "S-release-thread", "S-release-owndc", "S-gdcex-parentdc", "S-gdcex-bits", "S-gdcex-window"] {
        assert!(c.contains(s), "fle_gdc.c must name the choice {s}");
    }
    let g = std::fs::read_to_string(src_dir().join("fle_gdi.c")).unwrap();
    assert!(g.contains("S-hdc-null"));
    let r = std::fs::read_to_string(src_dir().join("fle_res.c")).unwrap();
    assert!(r.contains("S-loadcursor-handle") && r.contains("S-loadcursor-missing"));
    // the window model asks the DC table for nothing but GetDC and ReleaseDC (BeginPaint / EndPaint, and the DC that RedrawWindow(ERASENOW / UPDATENOW) gives to WM_ERASEBKGND: two pairs, decision 0253; where they are is
    // checked in wall_fr9); the destroy hook is weak
    let win = strip_comments(&std::fs::read_to_string(src_dir().join("fle_win.c")).unwrap());
    assert!(win.contains("void fle_gdc_window_destroyed(uint64_t hwnd) __attribute__((weak));"));
    assert_eq!(win.matches("fle_imp_GetDC()").count(), 2);
    assert_eq!(win.matches("fle_imp_ReleaseDC()").count(), 2);
    // the header: plain C
    let h = strip_comments(&std::fs::read_to_string(src_dir().join("fle_gdc.h")).unwrap());
    for bad in ["asm", "__asm__", "<unistd.h>", "<sys/", "<pthread.h>", "<time.h>", "long "] {
        assert!(!h.contains(bad), "fle_gdc.h must not contain {bad}");
    }
}
