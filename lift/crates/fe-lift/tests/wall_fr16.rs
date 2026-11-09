//! Tests of decision 0140 (docs/decisions/0140-layered-windows.md, PROVISIONAL): the layered-window model of runtime/fle_layered.c (UpdateLayeredWindow, UpdateLayeredWindowIndirect, SetLayeredWindowAttributes,
//! GetLayeredWindowAttributes) on the real fle_win.c, fle_gdi.c, fle_raster.c, fle_thread.c and the files they work with.
//!
//! Expected values are written here from the Microsoft Learn pages named in the decision (fetched 2026-11-02, kept in docs/investigations/lay1/learn/ with index.tsv: UpdateLayeredWindow, UPDATELAYEREDWINDOWINFO,
//! SetLayeredWindowAttributes, GetLayeredWindowAttributes, BLENDFUNCTION, AlphaBlend, Window Features, Extended Window Styles) and from the choices of the decision (each choice without a source is named S-ulw-* in the
//! decision document), not read back from the C code. EVERY ANSWER IS PROVISIONAL; nothing is compared with a real Windows. A test that pins a choice without a source says so in its comment: it fixes the choice, not
//! a Windows rule.
//!
//! 1. Generator: the 4 rows of decision 0140 are the last rows, in the order given, and no pseudo-VA moved; each is answered under user32.dll only.
//! 2. GetProcAddress hands out the rows and the pseudo-VA reaches the model.
//! 3. The parameters: failures (invalid window, not layered, the "must be NULL" rules, the blend function) and the forms that stop by name.
//! 4. The pixels: ULW_ALPHA with and without AC_SRC_ALPHA, ULW_OPAQUE, ULW_COLORKEY, pptSrc, size and position, no messages, hdcSrc NULL.
//! 5. UpdateLayeredWindowIndirect: cbSize, ULW_EX_NORESIZE, prcDirty.
//! 6. SetLayeredWindowAttributes / GetLayeredWindowAttributes and the order rules of the Learn pages; the layering bit set again; destruction.

mod etxt_common;
mod rows_0132;
mod rows_0140;
mod rows_0144;
mod rows_0147;
mod rows_0267;

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
    if let Ok(d) = std::env::var("FLE_FR16_SRC_DIR") {
        return PathBuf::from(d);
    }
    rt_dir()
}
fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-wallFR16/test");
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


// ---------------------------------------------------------------------------------------------
// 2. The driver: the real fle_win.c, fle_class.c, fle_thread.c and the files they work with; a stub guest window procedure that logs every message
// ---------------------------------------------------------------------------------------------

const RT_FILES: [&str; 25] = [
    "fle_thread.c", "fle_win.c", "fle_class.c", "fle_event.c", "fle_sys.c", "fle_imports.c", "fle_proc.c", "fle_callback.c", "fle_env.c", "fle_vfs.c", "fle_vmap.c", "fle_vol.c", "fle_host.c",
    "fle_time.c", "fle_gdc.c", "fle_gdi.c", "fle_cursor.c", "fle_raster.c", "fle_monitor.c", "fle_ole.c", "fle_reg.c",
    "fle_layered.c", "fle_modules.c", "fle_realmod.c", "fle_load.c",
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
        let dir = work_dir().join(etxt_common::per_process("obj-fr16"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut v = Vec::new();
        let ours = ["fle_thread.c", "fle_win.c", "fle_class.c", "fle_gdi.c", "fle_vmap.c", "fle_vfs.c", "fle_raster.c", "fle_monitor.c", "fle_ole.c", "fle_layered.c"];
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
#include "fle_raster.h"
#include "fle_layered.h"
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
// 1. Generator
// ---------------------------------------------------------------------------------------------

#[test]
fn the_4_rows_of_decision_0140_are_the_last_rows_in_the_order_given_and_no_pseudo_va_moved() {
    let (st, c) = gen(&GUEST_IMPORTS);
    let rows = resp_rows(&c);
    assert_eq!(rows.len(), st.resp_entries);
    let k = rows_0140::ROWS_0140.len();
    let n = rows.len() - rows_0144::ROWS_0144.len() - rows_0147::ROWS_0147.len() - rows_0267::ROWS_0267.len(); // decision 0267 appends 1 row after those of 0147; decision 0147 appends 1 row after those of 0144; decision 0144 appends 2 rows (tests/wall_fr19.rs) after these
    // the rows before are the 3 of decision 0139 (tests/wall_fr15.rs), whose last row is dsound!DirectSoundEnumerateW; HEAD 93881c0 has 601 rows (598 of decision 0136 + 3), decision 0140 appends 4 (decision 0015: append only)
    assert_eq!(n, 601 + k, "the 601 rows of HEAD 93881c0 and the {k} of decision 0140");
    assert_eq!((rows[n - k - 1].0.as_str(), rows[n - k - 1].1.as_str()), ("dsound.dll", "DirectSoundEnumerateW"), "the row before the new ones is the last row of decision 0139");
    for (i, (dll, name)) in rows_0140::ROWS_0140.iter().enumerate() {
        assert_eq!((rows[n - k + i].0.as_str(), rows[n - k + i].1.as_str()), (*dll, *name), "row {}", n - k + i);
    }
    for (i, (_, _, va)) in rows.iter().enumerate() {
        assert_eq!(*va, 0x7FF9_0000_0000u64 + 0x10 * (i as u64 + 1), "row {i}: the pseudo-VA of a row never moves");
    }
    for (_, name) in rows_0140::ROWS_0140.iter() {
        assert_eq!(rows.iter().filter(|(_, n2, _)| n2 == name).count(), 1, "{name}: exactly one row");
        assert!(c.contains(&format!("void fle_imp_{name}(void);")), "{name} is declared in the generated C");
    }
}

#[test]
fn each_new_thunk_is_answered_under_its_own_dll_and_no_other_dll() {
    for (dll, name) in rows_0140::ROWS_0140 {
        let (st, c) = gen(&[(dll, name), ("kernel32.dll", name), (dll, "SendMessageA"), ("kernel32.dll", "GetProcAddress")]);
        assert_eq!(st.handled[format!("jmp_imp:{name}").as_str()], 1, "{dll}!{name}");
        assert_eq!(c.matches(&format!("fle_imp_{name}(); return;")).count(), 1, "{dll}!{name}: only the thunk of its own dll is answered");
    }
}

#[test]
fn the_apis_of_the_layered_family_that_the_guest_does_not_use_are_not_rows() {
    // the guest takes UpdateLayeredWindow, UpdateLayeredWindowIndirect and SetLayeredWindowAttributes with GetProcAddress (docs/decisions/0140-layered-windows.md); nothing else of the family is asked for
    let table = std::fs::read_to_string(manifest().join("src/imports_table.rs")).unwrap();
    for n in ["UpdateLayeredWindowEx", "SetLayeredWindowAttributesEx", "PrintWindow", "AlphaBlend", "GdiAlphaBlend", "TransparentBlt"] {
        assert!(!table.contains(&format!("\"{n}\"")), "{n} must not be a row of decision 0140");
    }
}

// ---------------------------------------------------------------------------------------------
// the driver for the layered model
// ---------------------------------------------------------------------------------------------

const LAY_PRE: &str = r##"
static uint64_t callf9(void (*f)(void), uint64_t a, uint64_t b, uint64_t c, uint64_t d, uint64_t e, uint64_t g, uint64_t h7, uint64_t h8, uint64_t h9)
{
    uint64_t sp0_ = R[4];
    R[4] = (R[4] - 0x80) & ~UINT64_C(15); R[4] -= 8; fle_st(R[4], 8, 0xAAAA0000);
    fle_st(R[4] + 0x28, 8, e); fle_st(R[4] + 0x30, 8, g); fle_st(R[4] + 0x38, 8, h7); fle_st(R[4] + 0x40, 8, h8); fle_st(R[4] + 0x48, 8, h9);
    R[1] = a; R[2] = b; R[8] = c; R[9] = d; f(); R[4] = sp0_;
    return R[0];
}
/* ULW(hwnd, hdcDst, pptDst, psize, hdcSrc, pptSrc, crKey, pblend, dwFlags) */
#define ULW(h, hd, pd, ps, hs, pp, key, pb, fl) callf9(fle_imp_UpdateLayeredWindow, h, hd, pd, ps, hs, pp, key, pb, fl)
/* UpdateLayeredWindowIndirect through a UPDATELAYEREDWINDOWINFO of 80 bytes */
static uint64_t ULWI_SZ(uint64_t h, uint32_t cb, uint64_t hd, uint64_t pd, uint64_t ps, uint64_t hs, uint64_t pp, uint32_t key, uint64_t pb, uint32_t fl, uint64_t dirty)
{
    static uint8_t info[96];
    memset(info, 0, sizeof info);
    memcpy(info, &cb, 4); memcpy(info + 8, &hd, 8); memcpy(info + 16, &pd, 8); memcpy(info + 24, &ps, 8); memcpy(info + 32, &hs, 8); memcpy(info + 40, &pp, 8);
    memcpy(info + 48, &key, 4); memcpy(info + 56, &pb, 8); memcpy(info + 64, &fl, 4); memcpy(info + 72, &dirty, 8);
    return callf(fle_imp_UpdateLayeredWindowIndirect, h, A(info), 0, 0);
}
#define ULWI(h, hd, pd, ps, hs, pp, key, pb, fl, dirty) ULWI_SZ(h, 80, hd, pd, ps, hs, pp, key, pb, fl, dirty)
#define WS_EX_LAYERED_ 0x00080000u
#define WS_EX_TOOLWINDOW_ 0x00000080u
#define LAY_EX (WS_EX_LAYERED_ | WS_EX_TOOLWINDOW_)
#define LAY_ST (POPUP | VIS)
/* a memory DC with a 32-bit top-down DIB section w x h selected into it; *bits is the pointer to the pixels (B, G, R, X per pixel) */
static uint64_t mk_dib(int w, int h, int bpp, uint8_t **bits)
{
    static uint8_t bmi[48];
    static uint64_t pbits;
    uint64_t dc, bmp;
    int32_t hh = -h;
    uint16_t pl = 1, bc = (uint16_t)bpp;
    uint32_t sz = 40, comp = 0;
    memset(bmi, 0, sizeof bmi);
    memcpy(bmi, &sz, 4); memcpy(bmi + 4, &w, 4); memcpy(bmi + 8, &hh, 4); memcpy(bmi + 12, &pl, 2); memcpy(bmi + 14, &bc, 2); memcpy(bmi + 16, &comp, 4);
    dc = callf(fle_imp_CreateCompatibleDC, 0, 0, 0, 0);
    bmp = callf6(fle_imp_CreateDIBSection, dc, A(bmi), 0, A(&pbits), 0, 0);
    callf(fle_imp_SelectObject, dc, bmp, 0, 0);
    *bits = (uint8_t *)(uintptr_t)pbits;
    return dc;
}
static uint8_t *surf_bytes(uint64_t h, uint32_t *w, uint32_t *hh)
{
    fle_win_dcq q;
    const fle_rsurf *s;
    if (!fle_win_dc_query(h, &q)) return NULL;
    s = fle_raster_surface_peek(q.root_lid);
    if (!s) return NULL;
    if (w) *w = s->width;
    if (hh) *hh = s->height;
    return s->bits;
}
static void pxs(uint64_t h, int n)
{
    uint32_t w, hh;
    uint8_t *b = surf_bytes(h, &w, &hh);
    int i;
    if (!b) { printf("nosurface\n"); return; }
    printf("surface %ux%u:", w, hh);
    for (i = 0; i < n; i++) printf(" %02x%02x%02x%02x", b[4 * i], b[4 * i + 1], b[4 * i + 2], b[4 * i + 3]);
    printf("\n");
}
static void wrect(uint64_t h)
{
    fle_win_view v;
    fle_win_get(h, &v);
    printf("window %d,%d %dx%d\n", v.x, v.y, v.cx, v.cy);
}
static void lay(uint64_t h)
{
    fle_layered_info li;
    if (!fle_layered_query(h, &li)) { printf("notlayered\n"); return; }
    printf("layer mode=%d alpha=%u key=%x flags=%x per_pixel=%u size=%ux%u updates=%llu\n", li.mode, li.alpha, li.colorkey, li.lwa_flags, li.per_pixel, li.width, li.height, U(li.updates));
}
static void set_px(uint8_t *b, int i, uint8_t B, uint8_t G, uint8_t Rr, uint8_t Xa) { b[4 * i] = B; b[4 * i + 1] = G; b[4 * i + 2] = Rr; b[4 * i + 3] = Xa; }
"##;

fn lay_lines(name: &str, body: &str) -> Vec<String> {
    lines_top(name, &top_with(LAY_PRE, &[]), body)
}

fn lay_aborts(name: &str, body: &str, api: &str, word: &str) -> Out {
    let o = run_src(name, &top_with(LAY_PRE, &[]), body);
    assert!(!o.ok, "{name}: expected an abort, got stdout {}", o.stdout);
    assert_eq!(o.signal, Some(6), "{name}: not SIGABRT: {}", o.stderr);
    assert!(o.stderr.contains(&format!("fle: {api}: ")), "{name}: no 'fle: {api}: ' message: {}", o.stderr);
    assert!(o.stderr.contains(word), "{name}: message lacks {word:?}: {}", o.stderr);
    o
}

// ---------------------------------------------------------------------------------------------
// 2. GetProcAddress and the pseudo-VA
// ---------------------------------------------------------------------------------------------

#[test]
fn getprocaddress_hands_out_the_four_rows_under_user32_and_the_pseudo_va_reaches_the_model() {
    let (_, c) = gen(&GUEST_IMPORTS);
    let rows = resp_rows(&c);
    let mut tab = String::new();
    let mut ent: Vec<(String, String)> = Vec::new();
    for (_, name) in rows_0140::ROWS_0140 {
        let (_, _, va) = rows.iter().find(|r| r.1 == name).unwrap().clone();
        tab.push_str(&format!("    {{ \"user32.dll\", \"{name}\", UINT64_C({va:#x}), fle_imp_{name} }},\n"));
        ent.push((format!("{va:#x}"), format!("fle_imp_{name}")));
    }
    let n = rows_0140::ROWS_0140.len();
    let extra = format!("{LAY_PRE}\nconst fle_resp_entry fle_resp_table[] = {{\n{tab}}};\nconst uint64_t fle_resp_count = {n};\n");
    let e: Vec<(&str, &str)> = ent.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    let top = top_with(&extra, &e);
    let body = r##"
    static uint16_t u32[] = { 'u','s','e','r','3','2','.','d','l','l',0 };
    uint64_t m, p;
    const char *names[4] = { "UpdateLayeredWindow", "UpdateLayeredWindowIndirect", "SetLayeredWindowAttributes", "GetLayeredWindowAttributes" };
    int i;
    m = callf(fle_imp_GetModuleHandleW, A(u32), 0, 0, 0);
    for (i = 0; i < 4; i++) {
        p = callf(fle_imp_GetProcAddress, m, A(names[i]), 0, 0);
        printf("%s %d\n", names[i], p != 0);
    }
    p = callf(fle_imp_GetProcAddress, m, A("UpdateLayeredWindowEx"), 0, 0);
    printf("missing %llu err %u\n", U(p), LE());
    /* the pseudo-VA of the first row, entered as the guest's `call [var]` does: a window handle that is not one fails with 1400 and a zero result */
    p = callf(fle_imp_GetProcAddress, m, A("UpdateLayeredWindow"), 0, 0);
    R[1] = 0x1234; R[2] = 0; R[8] = 0; R[9] = 0; R[4] -= 8; fle_st(R[4], 8, 0xAAAA0000); fle_dispatch(p); R[4] += 8;
    printf("via pseudo-VA: ret %llu err %u\n", U(R[0]), LE());
"##;
    let l = lines_top("lay_getproc", &top, body);
    assert_eq!(l[0..4], ["UpdateLayeredWindow 1", "UpdateLayeredWindowIndirect 1", "SetLayeredWindowAttributes 1", "GetLayeredWindowAttributes 1"]);
    assert_eq!(l[4], "missing 0 err 127", "a name without a row: NULL and ERROR_PROC_NOT_FOUND (decision 0015)");
    assert_eq!(l[5], "via pseudo-VA: ret 0 err 1400", "S-ulw-error-invalid-hwnd: ERROR_INVALID_WINDOW_HANDLE, the choice of SendMessageW");
}

// ---------------------------------------------------------------------------------------------
// 3. The parameters
// ---------------------------------------------------------------------------------------------

#[test]
fn failures_set_the_last_error_and_change_nothing() {
    // The pages name no error code (S-ulw-error-*): 1400 for a window that is not one, 87 for the rest.
    let l = lay_lines("lay_fail", r##"
    uint8_t *bits;
    uint64_t dc = mk_dib(2, 2, 32, &bits), h, plain;
    int32_t pt[2] = { 5, 6 }, sz[2] = { 2, 2 };
    uint8_t bf[4] = { 0, 0, 255, 1 };
    h = cw(LAY_EX, NULL, LAY_ST, 10, 20, 16, 16, 0, 0, 0);
    plain = cw(WS_EX_TOOLWINDOW_, NULL, LAY_ST, 10, 20, 16, 16, 0, 0, 0);
    set_px(bits, 0, 1, 2, 3, 4);
    SETLE(0); printf("badhwnd %llu %u\n", U(ULW(0x1234, 0, 0, 0, 0, 0, 0, 0, 0)), LE());
    SETLE(0); printf("notlayered %llu %u\n", U(ULW(plain, 0, 0, 0, 0, 0, 0, 0, 0)), LE());
    SETLE(0); printf("dst-without-src %llu %u\n", U(ULW(h, 0x7fa90300, 0, 0, 0, 0, 0, 0, 0)), LE());
    SETLE(0); printf("size-without-src %llu %u\n", U(ULW(h, 0, 0, A(sz), 0, 0, 0, 0, 0)), LE());
    SETLE(0); printf("alpha-without-blend %llu %u\n", U(ULW(h, 0, A(pt), A(sz), dc, 0, 0, 0, 2)), LE());
    bf[1] = 1; SETLE(0); printf("blendflags %llu %u\n", U(ULW(h, 0, A(pt), A(sz), dc, 0, 0, A(bf), 2)), LE()); bf[1] = 0;
    bf[0] = 1; SETLE(0); printf("blendop %llu %u\n", U(ULW(h, 0, A(pt), A(sz), dc, 0, 0, A(bf), 2)), LE()); bf[0] = 0;
    sz[0] = 0; SETLE(0); printf("zero-size %llu %u\n", U(ULW(h, 0, A(pt), A(sz), dc, 0, 0, A(bf), 2)), LE()); sz[0] = 2;
    sz[1] = -1; SETLE(0); printf("neg-size %llu %u\n", U(ULW(h, 0, A(pt), A(sz), dc, 0, 0, A(bf), 2)), LE()); sz[1] = 2;
    { uint8_t *b24; uint64_t dc24 = mk_dib(2, 2, 24, &b24); SETLE(0); printf("alpha-on-24bit %llu %u\n", U(ULW(h, 0, A(pt), A(sz), dc24, 0, 0, A(bf), 2)), LE()); }
    wrect(h); lay(h); printf("nosurface: %d\n", surf_bytes(h, NULL, NULL) == NULL);
"##);
    assert_eq!(
        l,
        [
            "badhwnd 0 1400",
            "notlayered 0 87",
            "dst-without-src 0 87",
            "size-without-src 0 87",
            "alpha-without-blend 0 87",
            "blendflags 0 87",
            "blendop 0 87",
            "zero-size 0 87",
            "neg-size 0 87",
            "alpha-on-24bit 0 87",
            "window 10,20 16x16",
            "layer mode=0 alpha=255 key=0 flags=0 per_pixel=0 size=0x0 updates=0",
            "nosurface: 1",
        ]
    );
}

#[test]
fn the_forms_the_pages_leave_open_stop_by_name() {
    let pre = r##"
    uint8_t *bits;
    uint64_t dc = mk_dib(2, 2, 32, &bits), h = cw(LAY_EX, NULL, LAY_ST, 10, 20, 16, 16, 0, 0, 0);
    int32_t pt[2] = { 5, 6 }, sz[2] = { 2, 2 }, ps[2] = { 0, 0 };
    uint8_t bf[4] = { 0, 0, 255, 1 };
"##;
    let cases: [(&str, &str, &str); 9] = [
        ("lay_stop_flagbits", "ULW(h, 0, A(pt), A(sz), dc, 0, 0, A(bf), 0x10);", "dwFlags has a bit that the page does not list"),
        ("lay_stop_noresize_plain", "ULW(h, 0, A(pt), A(sz), dc, 0, 0, A(bf), 0xa);", "ULW_EX_NORESIZE for UpdateLayeredWindow"),
        ("lay_stop_two_kinds", "ULW(h, 0, A(pt), A(sz), dc, 0, 0, A(bf), 0x3);", "more than one of ULW_COLORKEY"),
        ("lay_stop_nosrc_flags", "ULW(h, 0, A(pt), 0, 0, 0, 0, 0, 2);", "should be zero"),
        ("lay_stop_nosrc_ppt", "ULW(h, 0, A(pt), 0, 0, A(ps), 0, 0, 0);", "pptSrc \"should be NULL\""),
        ("lay_stop_kind0", "ULW(h, 0, A(pt), A(sz), dc, 0, 0, A(bf), 0);", "none of ULW_COLORKEY, ULW_ALPHA, ULW_OPAQUE is"),
        ("lay_stop_screen_src", "ULW(h, 0, A(pt), A(sz), 0x7fa90300, 0, 0, A(bf), 2);", "not a memory DC"),
        ("lay_stop_src_rect", "ps[0] = 1; ULW(h, 0, A(pt), A(sz), dc, A(ps), 0, A(bf), 2);", "not inside the bitmap"),
        ("lay_stop_afmt", "bf[3] = 3; ULW(h, 0, A(pt), A(sz), dc, 0, 0, A(bf), 2);", "AlphaFormat has a bit other than AC_SRC_ALPHA"),
    ];
    for (name, call, word) in cases {
        lay_aborts(name, &format!("{pre}    {call}\n"), "UpdateLayeredWindow", word);
    }
}

// ---------------------------------------------------------------------------------------------
// 4. The pixels
// ---------------------------------------------------------------------------------------------

#[test]
fn ulw_alpha_with_ac_src_alpha_keeps_the_premultiplied_bytes_moves_and_sizes_the_window_and_sends_no_message() {
    // Learn BLENDFUNCTION: AC_SRC_ALPHA = the bitmap has per-pixel alpha, premultiplied; SourceConstantAlpha is kept for the window (the Presenter multiplies it). Learn UpdateLayeredWindow: pptDst "the new screen position",
    // psize "the new size". S-ulw-no-messages: the window rectangle changes with no message sent to the procedure.
    let l = lay_lines("lay_alpha", r##"
    uint8_t *bits;
    uint64_t dc = mk_dib(2, 2, 32, &bits), h;
    int32_t pt[2] = { 959, 539 }, sz[2] = { 2, 2 };
    uint8_t bf[4] = { 0, 0, 200, 1 };
    int m0;
    h = cw(LAY_EX, NULL, LAY_ST, 10, 20, 16, 16, 0, 0, 0);
    set_px(bits, 0, 0x10, 0x20, 0x30, 0x40); set_px(bits, 1, 0, 0, 0, 0); set_px(bits, 2, 0xff, 0x80, 0x00, 0xff); set_px(bits, 3, 1, 2, 3, 4);
    lay(h);
    m0 = wn;
    printf("ret %llu\n", U(ULW(h, 0, A(pt), A(sz), dc, 0, 0, A(bf), 2)));
    printf("messages %d\n", wn - m0);
    wrect(h); pxs(h, 4); lay(h);
"##);
    assert_eq!(
        l,
        [
            "layer mode=0 alpha=255 key=0 flags=0 per_pixel=0 size=0x0 updates=0",
            "ret 1",
            "messages 0",
            "window 959,539 2x2",
            "surface 2x2: 10203040 00000000 ff8000ff 01020304",
            "layer mode=1 alpha=200 key=0 flags=0 per_pixel=1 size=2x2 updates=1",
        ]
    );
}

#[test]
fn ulw_alpha_without_ac_src_alpha_makes_every_pixel_opaque_and_ulw_opaque_ignores_the_blend_function() {
    // Learn BLENDFUNCTION: without AC_SRC_ALPHA the bitmap has no per-pixel alpha, SourceConstantAlpha alone applies. ULW_OPAQUE: "Draw an opaque layered window".
    let l = lay_lines("lay_noalpha", r##"
    uint8_t *bits;
    uint64_t dc = mk_dib(2, 1, 32, &bits), h;
    int32_t pt[2] = { 0, 0 }, sz[2] = { 2, 1 };
    uint8_t bf[4] = { 0, 0, 77, 0 };
    h = cw(LAY_EX, NULL, LAY_ST, 10, 20, 16, 16, 0, 0, 0);
    set_px(bits, 0, 0x10, 0x20, 0x30, 0x00); set_px(bits, 1, 0xaa, 0xbb, 0xcc, 0x55);
    printf("ret %llu\n", U(ULW(h, 0, A(pt), A(sz), dc, 0, 0, A(bf), 2)));
    pxs(h, 2); lay(h);
    printf("ret %llu\n", U(ULW(h, 0, A(pt), A(sz), dc, 0, 0, A(bf), 4)));
    pxs(h, 2); lay(h);
"##);
    assert_eq!(
        l,
        [
            "ret 1",
            "surface 2x1: 102030ff aabbccff",
            "layer mode=1 alpha=77 key=0 flags=0 per_pixel=1 size=2x1 updates=1",
            "ret 1",
            "surface 2x1: 102030ff aabbccff",
            "layer mode=1 alpha=255 key=0 flags=0 per_pixel=1 size=2x1 updates=2",
        ]
    );
}

#[test]
fn ulw_colorkey_makes_the_key_colour_transparent_and_compares_three_colour_bytes() {
    // Learn: ULW_COLORKEY "Use crKey as the transparency color". crKey is a COLORREF 0x00BBGGRR; the unused byte of a 32-bit DIB is not part of the colour. S-ulw-colorkey-baked: the key is applied to the pixels.
    let l = lay_lines("lay_key", r##"
    uint8_t *bits;
    uint64_t dc = mk_dib(3, 1, 32, &bits), h;
    int32_t pt[2] = { 0, 0 }, sz[2] = { 3, 1 };
    h = cw(LAY_EX, NULL, LAY_ST, 10, 20, 16, 16, 0, 0, 0);
    set_px(bits, 0, 0x30, 0x20, 0x10, 0x99);  /* R=0x10 G=0x20 B=0x30 -> COLORREF 0x00302010 */
    set_px(bits, 1, 0x31, 0x20, 0x10, 0x00);
    set_px(bits, 2, 0x30, 0x20, 0x10, 0x00);
    printf("ret %llu\n", U(ULW(h, 0, A(pt), A(sz), dc, 0, 0xff302010, 0, 1)));
    pxs(h, 3); lay(h);
"##);
    assert_eq!(l, ["ret 1", "surface 3x1: 00000000 312010ff 00000000", "layer mode=1 alpha=255 key=0 flags=0 per_pixel=1 size=3x1 updates=1"]);
}

#[test]
fn pptsrc_selects_the_source_rectangle_and_a_smaller_or_larger_psize_resizes_the_window_and_the_surface() {
    let l = lay_lines("lay_size", r##"
    uint8_t *bits;
    uint64_t dc = mk_dib(3, 2, 32, &bits), h;
    int32_t pt[2] = { 30, 40 }, sz[2] = { 2, 1 }, src[2] = { 1, 1 };
    uint8_t bf[4] = { 0, 0, 255, 1 };
    int i;
    h = cw(LAY_EX, NULL, LAY_ST, 10, 20, 16, 16, 0, 0, 0);
    for (i = 0; i < 6; i++) set_px(bits, i, (uint8_t)(i + 1), 0, 0, 0xff);
    printf("ret %llu\n", U(ULW(h, 0, A(pt), A(sz), dc, A(src), 0, A(bf), 2)));
    wrect(h); pxs(h, 2);
    sz[0] = 3; sz[1] = 2; src[0] = 0; src[1] = 0;
    printf("ret %llu\n", U(ULW(h, 0, 0, A(sz), dc, A(src), 0, A(bf), 2)));
    wrect(h); pxs(h, 6); lay(h);
"##);
    assert_eq!(
        l,
        [
            "ret 1",
            "window 30,40 2x1",
            "surface 2x1: 050000ff 060000ff",
            "ret 1",
            "window 30,40 3x2",
            "surface 3x2: 010000ff 020000ff 030000ff 040000ff 050000ff 060000ff",
            "layer mode=1 alpha=255 key=0 flags=0 per_pixel=1 size=3x2 updates=2",
        ]
    );
}

#[test]
fn with_a_null_hdcsrc_only_the_position_changes_and_the_pixels_and_the_attributes_stay() {
    // Learn: "If the current position is not changing, pptDst can be NULL", "If the shape and visual context of the window are not changing, hdcSrc can be NULL"; "If hdcSrc is NULL, hdcDst must be NULL" and "psize must be NULL".
    let l = lay_lines("lay_nosrc", r##"
    uint8_t *bits;
    uint64_t dc = mk_dib(1, 1, 32, &bits), h;
    int32_t pt[2] = { 30, 40 }, sz[2] = { 1, 1 }, mv[2] = { 100, 200 };
    uint8_t bf[4] = { 0, 0, 90, 1 };
    h = cw(LAY_EX, NULL, LAY_ST, 10, 20, 16, 16, 0, 0, 0);
    set_px(bits, 0, 9, 8, 7, 6);
    ULW(h, 0, A(pt), A(sz), dc, 0, 0, A(bf), 2);
    printf("ret %llu\n", U(ULW(h, 0, A(mv), 0, 0, 0, 0, 0, 0)));
    wrect(h); pxs(h, 1); lay(h);
"##);
    assert_eq!(l, ["ret 1", "window 100,200 1x1", "surface 1x1: 09080706", "layer mode=1 alpha=90 key=0 flags=0 per_pixel=1 size=1x1 updates=1"]);
}

#[test]
fn a_window_that_is_hidden_keeps_the_content_it_was_given() {
    // Learn Window Features: "the system stores the image of a layered window"; the content does not depend on the window being shown.
    let l = lay_lines("lay_hidden", r##"
    uint8_t *bits;
    uint64_t dc = mk_dib(1, 1, 32, &bits), h;
    int32_t pt[2] = { 0, 0 }, sz[2] = { 1, 1 };
    uint8_t bf[4] = { 0, 0, 255, 1 };
    h = cw(LAY_EX, NULL, POPUP, 10, 20, 16, 16, 0, 0, 0);   /* no WS_VISIBLE */
    set_px(bits, 0, 1, 2, 3, 4);
    printf("ret %llu\n", U(ULW(h, 0, A(pt), A(sz), dc, 0, 0, A(bf), 2)));
    pxs(h, 1);
"##);
    assert_eq!(l, ["ret 1", "surface 1x1: 01020304"]);
}

// ---------------------------------------------------------------------------------------------
// 5. UpdateLayeredWindowIndirect
// ---------------------------------------------------------------------------------------------

#[test]
fn the_indirect_form_reads_the_structure_of_80_bytes_and_does_what_the_direct_form_does() {
    let l = lay_lines("lay_indirect", r##"
    uint8_t *bits;
    uint64_t dc = mk_dib(2, 1, 32, &bits), h;
    int32_t pt[2] = { 959, 539 }, sz[2] = { 2, 1 }, bad[2] = { 3, 1 };
    uint8_t bf[4] = { 0, 0, 255, 1 };
    h = cw(LAY_EX, NULL, LAY_ST, 10, 20, 16, 16, 0, 0, 0);
    set_px(bits, 0, 0x10, 0x20, 0x30, 0x40); set_px(bits, 1, 0x50, 0x60, 0x70, 0x80);
    SETLE(0); printf("cbSize 79: %llu %u\n", U(ULWI_SZ(h, 79, 0, A(pt), A(sz), dc, 0, 0, A(bf), 2, 0)), LE());
    SETLE(0); printf("cbSize 81: %llu %u\n", U(ULWI_SZ(h, 81, 0, A(pt), A(sz), dc, 0, 0, A(bf), 2, 0)), LE());
    SETLE(0); printf("null info: %llu %u\n", U(callf(fle_imp_UpdateLayeredWindowIndirect, h, 0, 0, 0)), LE());
    printf("ret %llu\n", U(ULWI(h, 0, A(pt), A(sz), dc, 0, 0, A(bf), 2, 0)));
    wrect(h); pxs(h, 2); lay(h);
    /* ULW_EX_NORESIZE: "Force the UpdateLayeredWindowIndirect function to fail if the current window size does not match the size specified in the psize" */
    SETLE(0); printf("noresize mismatch: %llu %u\n", U(ULWI(h, 0, A(pt), A(bad), dc, 0, 0, A(bf), 2 | 8, 0)), LE());
    printf("noresize match: %llu\n", U(ULWI(h, 0, A(pt), A(sz), dc, 0, 0, A(bf), 2 | 8, 0)));
    wrect(h);
"##);
    assert_eq!(
        l,
        [
            "cbSize 79: 0 87",
            "cbSize 81: 0 87",
            "null info: 0 87",
            "ret 1",
            "window 959,539 2x1",
            "surface 2x1: 10203040 50607080",
            "layer mode=1 alpha=255 key=0 flags=0 per_pixel=1 size=2x1 updates=1",
            "noresize mismatch: 0 87",
            "noresize match: 1",
            "window 959,539 2x1",
        ]
    );
}

#[test]
fn a_dirty_rectangle_limits_the_update_to_that_area_of_the_source() {
    // Learn UPDATELAYEREDWINDOWINFO: prcDirty "The area to be updated. This parameter can be NULL. If it is non-NULL, only the area in this rectangle is updated from the source DC." The page gives no coordinate space: the
    // runtime reads the rectangle in the coordinates of the layered window (S-sweep1-ulw-1, decision 0215; Wine 10.0's driver intersects {0, 0, cx, cy} with prcDirty). Decision 0140 stopped by name here; this test replaces
    // the check of that stop by a check of the contract: what is inside the rectangle comes from the source, what is outside stays, an empty rectangle or one outside the window changes nothing.
    let l = lay_lines("lay_dirty", r##"
    uint8_t *bits;
    uint64_t dc = mk_dib(3, 1, 32, &bits), h;
    int32_t pt[2] = { 0, 0 }, sz[2] = { 3, 1 };
    int32_t rc1[4] = { 1, 0, 2, 1 }, rc2[4] = { 2, 0, 2, 1 }, rc3[4] = { 7, 7, 9, 9 }, rc4[4] = { -4, -4, 1, 1 }, rc5[4] = { 0, 0, 99, 99 }, rc6[4] = { 2, 1, 1, 0 };
    uint8_t bf[4] = { 0, 0, 255, 1 };
    h = cw(LAY_EX, NULL, LAY_ST, 10, 20, 16, 16, 0, 0, 0);
    set_px(bits, 0, 1, 1, 1, 1); set_px(bits, 1, 2, 2, 2, 2); set_px(bits, 2, 3, 3, 3, 3);
    printf("full: %llu\n", U(ULWI(h, 0, A(pt), A(sz), dc, 0, 0, A(bf), 2, 0))); pxs(h, 3);
    set_px(bits, 0, 9, 9, 9, 9); set_px(bits, 1, 8, 8, 8, 8); set_px(bits, 2, 7, 7, 7, 7);
    printf("dirty x 1..2: %llu\n", U(ULWI(h, 0, 0, 0, dc, 0, 0, A(bf), 2, A(rc1)))); pxs(h, 3);
    printf("dirty empty: %llu\n", U(ULWI(h, 0, 0, 0, dc, 0, 0, A(bf), 2, A(rc2)))); pxs(h, 3);
    printf("dirty outside the window: %llu\n", U(ULWI(h, 0, 0, 0, dc, 0, 0, A(bf), 2, A(rc3)))); pxs(h, 3);
    printf("dirty inverted: %llu\n", U(ULWI(h, 0, 0, 0, dc, 0, 0, A(bf), 2, A(rc6)))); pxs(h, 3);
    printf("dirty from -4 (clipped to the window): %llu\n", U(ULWI(h, 0, 0, 0, dc, 0, 0, A(bf), 2, A(rc4)))); pxs(h, 3);
    printf("dirty larger than the window: %llu\n", U(ULWI(h, 0, 0, 0, dc, 0, 0, A(bf), 2, A(rc5)))); pxs(h, 3);
"##);
    assert_eq!(
        l,
        [
            "full: 1",
            "surface 3x1: 01010101 02020202 03030303",
            "dirty x 1..2: 1",
            "surface 3x1: 01010101 08080808 03030303",
            "dirty empty: 1",
            "surface 3x1: 01010101 08080808 03030303",
            "dirty outside the window: 1",
            "surface 3x1: 01010101 08080808 03030303",
            "dirty inverted: 1",
            "surface 3x1: 01010101 08080808 03030303",
            "dirty from -4 (clipped to the window): 1",
            "surface 3x1: 09090909 08080808 03030303",
            "dirty larger than the window: 1",
            "surface 3x1: 09090909 08080808 07070707",
        ]
    );
}

// ---------------------------------------------------------------------------------------------
// 6. SetLayeredWindowAttributes, GetLayeredWindowAttributes and the order rules
// ---------------------------------------------------------------------------------------------

#[test]
fn set_then_get_layered_window_attributes_and_the_order_rules_of_the_pages() {
    let l = lay_lines("lay_slwa", r##"
    uint8_t *bits;
    uint64_t dc = mk_dib(1, 1, 32, &bits), h;
    int32_t pt[2] = { 0, 0 }, sz[2] = { 1, 1 };
    uint8_t bf[4] = { 0, 0, 255, 1 };
    uint32_t key = 0xdeadbeef, fl = 0xffff; uint8_t al = 7;
    h = cw(LAY_EX, NULL, LAY_ST, 10, 20, 16, 16, 0, 0, 0);
    set_px(bits, 0, 1, 2, 3, 4);
    /* GetLayeredWindowAttributes "can be called only if the application has previously called SetLayeredWindowAttributes" */
    SETLE(0); printf("get before set: %llu %u\n", U(callf(fle_imp_GetLayeredWindowAttributes, h, A(&key), A(&al), A(&fl))), LE());
    printf("set: %llu\n", U(callf(fle_imp_SetLayeredWindowAttributes, h, 0x00112233, 0x80, 3)));
    lay(h);
    printf("get: %llu key=%x alpha=%u flags=%x\n", U(callf(fle_imp_GetLayeredWindowAttributes, h, A(&key), A(&al), A(&fl))), key, al, fl);
    printf("get with NULL pointers: %llu\n", U(callf(fle_imp_GetLayeredWindowAttributes, h, 0, 0, 0)));
    /* "once SetLayeredWindowAttributes has been called for a layered window, subsequent UpdateLayeredWindow calls will fail until the layering style bit is cleared and set again" */
    SETLE(0); printf("ulw after slwa: %llu %u\n", U(ULW(h, 0, A(pt), A(sz), dc, 0, 0, A(bf), 2)), LE());
    printf("surface: %d\n", surf_bytes(h, NULL, NULL) != NULL);
    /* the layering bit cleared and set again (SetWindowLongPtrW GWL_EXSTYLE, -20) */
    callf(fle_imp_SetWindowLongPtrW, h, (uint64_t)(int64_t)-20, WS_EX_TOOLWINDOW_, 0); lay(h);
    callf(fle_imp_SetWindowLongPtrW, h, (uint64_t)(int64_t)-20, LAY_EX, 0); lay(h);
    printf("ulw after the bit is set again: %llu\n", U(ULW(h, 0, A(pt), A(sz), dc, 0, 0, A(bf), 2)));
    /* GetLayeredWindowAttributes "will fail if the layered window was setup with UpdateLayeredWindow" */
    SETLE(0); printf("get after ulw: %llu %u\n", U(callf(fle_imp_GetLayeredWindowAttributes, h, A(&key), A(&al), A(&fl))), LE());
    /* a window that is not layered */
    { uint64_t p = cw(WS_EX_TOOLWINDOW_, NULL, LAY_ST, 0, 0, 4, 4, 0, 0, 0); SETLE(0); printf("set on plain: %llu %u\n", U(callf(fle_imp_SetLayeredWindowAttributes, p, 0, 255, 2)), LE()); }
    SETLE(0); printf("set on bad hwnd: %llu %u\n", U(callf(fle_imp_SetLayeredWindowAttributes, 0x1234, 0, 255, 2)), LE());
    /* LWA_ALPHA only: the key stays 0 and flags are kept; LWA_COLORKEY only: the alpha is 255 */
    { uint64_t g = cw(LAY_EX, NULL, LAY_ST, 0, 0, 4, 4, 0, 0, 0);
      callf(fle_imp_SetLayeredWindowAttributes, g, 0x00aabbcc, 0x40, 2); lay(g);
      callf(fle_imp_SetLayeredWindowAttributes, g, 0x00aabbcc, 0x40, 1); lay(g); }
"##);
    assert_eq!(
        l,
        [
            "get before set: 0 87",
            "set: 1",
            "layer mode=2 alpha=128 key=112233 flags=3 per_pixel=0 size=0x0 updates=0",
            "get: 1 key=112233 alpha=128 flags=3",
            "get with NULL pointers: 1",
            "ulw after slwa: 0 87",
            "surface: 0",
            "notlayered",
            "layer mode=0 alpha=255 key=0 flags=0 per_pixel=0 size=0x0 updates=0",
            "ulw after the bit is set again: 1",
            "get after ulw: 0 87",
            "set on plain: 0 87",
            "set on bad hwnd: 0 1400",
            "layer mode=2 alpha=64 key=0 flags=2 per_pixel=0 size=0x0 updates=0",
            "layer mode=2 alpha=255 key=aabbcc flags=1 per_pixel=0 size=0x0 updates=0",
        ]
    );
}

#[test]
fn the_forms_of_slwa_that_the_pages_leave_open_stop_by_name() {
    for (name, call, word) in [
        ("lay_slwa_flags0", "callf(fle_imp_SetLayeredWindowAttributes, h, 0, 255, 0);", "dwFlags is not LWA_COLORKEY and/or LWA_ALPHA"),
        ("lay_slwa_flags4", "callf(fle_imp_SetLayeredWindowAttributes, h, 0, 255, 4);", "dwFlags is not LWA_COLORKEY and/or LWA_ALPHA"),
        (
            "lay_slwa_after_ulw",
            "{ uint8_t *bits; int32_t pt[2] = { 0, 0 }, sz[2] = { 1, 1 }; uint8_t bf[4] = { 0, 0, 255, 1 }; uint64_t dc = mk_dib(1, 1, 32, &bits); ULW(h, 0, A(pt), A(sz), dc, 0, 0, A(bf), 2); callf(fle_imp_SetLayeredWindowAttributes, h, 0, 255, 2); }",
            "set up with UpdateLayeredWindow",
        ),
    ] {
        lay_aborts(name, &format!("    uint64_t h = cw(LAY_EX, NULL, LAY_ST, 0, 0, 4, 4, 0, 0, 0);\n    {call}\n"), "SetLayeredWindowAttributes", word);
    }
}

#[test]
fn destroying_a_window_forgets_its_layer_state_and_a_child_layered_window_stops_by_name() {
    let l = lay_lines("lay_destroy", r##"
    uint8_t *bits;
    uint64_t dc = mk_dib(1, 1, 32, &bits), h;
    int32_t pt[2] = { 0, 0 }, sz[2] = { 1, 1 };
    uint8_t bf[4] = { 0, 0, 255, 1 };
    h = cw(LAY_EX, NULL, LAY_ST, 10, 20, 16, 16, 0, 0, 0);
    ULW(h, 0, A(pt), A(sz), dc, 0, 0, A(bf), 2);
    lay(h);
    callf(fle_imp_DestroyWindow, h, 0, 0, 0);
    lay(h);
    /* the state of a destroyed window is dropped: the table of 64 states is not used up by 70 windows that are made and destroyed one after the other */
    { int i; for (i = 0; i < 70; i++) { uint64_t g = cw(LAY_EX, NULL, LAY_ST, 0, 0, 4, 4, 0, 0, 0); ULW(g, 0, A(pt), A(sz), dc, 0, 0, A(bf), 2); callf(fle_imp_DestroyWindow, g, 0, 0, 0); } printf("70 windows\n"); }
"##);
    assert_eq!(l, ["layer mode=1 alpha=255 key=0 flags=0 per_pixel=1 size=1x1 updates=1", "notlayered", "70 windows"]);
    lay_aborts(
        "lay_child",
        r##"
    uint8_t *bits;
    uint64_t dc = mk_dib(1, 1, 32, &bits), top = cw(0, NULL, LAY_ST, 0, 0, 40, 40, 0, 0, 0), ch;
    int32_t pt[2] = { 0, 0 }, sz[2] = { 1, 1 };
    uint8_t bf[4] = { 0, 0, 255, 1 };
    ch = cw(WS_EX_LAYERED_, NULL, CHILD | VIS, 1, 1, 4, 4, top, 0, 0);
    ULW(ch, 0, A(pt), A(sz), dc, 0, 0, A(bf), 2);
"##,
        "UpdateLayeredWindow",
        "a child window with WS_EX_LAYERED",
    );
}
