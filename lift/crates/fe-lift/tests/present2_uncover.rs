//! Tests of decision 0214 (docs/0214-present-path.md, PROVISIONAL): what the window manager does around a CHILD window that is moved, hidden, restacked or destroyed (runtime/fle_win.c: unc_begin / unc_end), and the dirty-rectangle notice
//! of ScrollWindowEx (runtime/fle_wall_ia2w.c). The engine's drag of a docked panel left the panel's old picture standing and its new place unpainted ('trails'): only a size change invalidated anything.
//!
//! Expected values are DERIVED BY HAND from Microsoft Learn, not read back from the C code:
//!   SetWindowPos  SWP_NOCOPYBITS: "Discards the entire contents of the client area. If this flag is not specified, the valid contents of the client area are saved and copied back into the client area after the window is
//!                 sized or repositioned."   SWP_NOREDRAW: "Does not redraw changes. If this flag is set, no repainting of any kind occurs. This applies to the client area, the nonclient area (including the title bar and scroll
//!                 bars), and any part of the parent window uncovered as a result of the window being moved. When this flag is set, the application must explicitly invalidate or redraw any parts of the window and parent
//!                 window that need redrawing."   MoveWindow bRepaint FALSE: the same.   "The WM_PAINT Message": "The system sends this message ... when changes to the window have altered the content of the client area".
//!   Window Features / Child Windows: a child is clipped to its parent's client area; "The child window is responsible for painting its client area after the move."
//! The update region of the model is ONE bounding rectangle per window (S-updregion), so an L-shaped uncovered area is reported by its bounding rectangle. Nothing is compared with a real Windows.

mod etxt_common;

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
    if let Ok(d) = std::env::var("FLE_P2_SRC_DIR") {
        return PathBuf::from(d);
    }
    rt_dir()
}
fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-present2/test/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

const RT_FILES: [&str; 26] = [
    "fle_thread.c", "fle_win.c", "fle_class.c", "fle_event.c", "fle_sys.c", "fle_imports.c", "fle_proc.c", "fle_callback.c", "fle_env.c", "fle_vfs.c", "fle_vmap.c", "fle_vol.c", "fle_host.c",
    "fle_time.c", "fle_gdc.c", "fle_gdi.c", "fle_cursor.c", "fle_res.c", "fle_modules.c", "fle_realmod.c", "fle_raster.c", "fle_monitor.c", "fle_ole.c", "fle_reg.c", "fle_visrgn.c", "fle_wall_ia2w.c",
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
        let dir = work_dir().join(etxt_common::per_process("obj-p2"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut v = Vec::new();
        let ours = ["fle_thread.c", "fle_win.c", "fle_visrgn.c", "fle_wall_ia2w.c", "fle_class.c", "fle_gdi.c", "fle_gdc.c", "fle_cursor.c", "fle_res.c", "fle_vmap.c", "fle_vfs.c", "fle_raster.c", "fle_monitor.c", "fle_ole.c", "fle_modules.c"];
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
#include "fle_raster.h"
#include "fle_visrgn.h"
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
    cmd.args(["-O0", "-std=gnu11", "-w", "-fPIE", "-pthread", "-Wl,--wrap=fle_raster_wrote", "-I"]).arg(src_dir()).arg("-I").arg(rt_dir());
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


const EXTRA_C: &str = r##"
static fle_rsurf *SF;
static int nwrote;
static int64_t wr[8][4];
void __wrap_fle_raster_wrote(const fle_rsurf *s, fle_rrect r, const char *api) { if (nwrote < 8) { wr[nwrote][0] = r.x0; wr[nwrote][1] = r.y0; wr[nwrote][2] = r.x1; wr[nwrote][3] = r.y1; } nwrote++; }
static uint32_t pat(int x, int y) { return 0xff000000u | ((uint32_t)x << 8) | (uint32_t)y; }
static uint32_t *PX(int x, int y) { return (uint32_t *)fle_r_row(SF, (uint32_t)y) + x; }
static void mksurf(uint64_t top, int w, int h) { uint64_t dc = callf(fle_imp_GetDC, top, 0, 0, 0); fle_gdc_info a; fle_gdc_dc_info(dc, &a); SF = fle_raster_window_surface(a.root_lid, w, h); }
static void fillpat(int x0, int y0, int x1, int y1) { int x, y; for (y = y0; y < y1; y++) for (x = x0; x < x1; x++) *PX(x, y) = pat(x, y); }
static void clean(uint64_t h) { callf(fle_imp_ValidateRect, h, 0, 0, 0); }
static void upd(const char *tag, uint64_t h) { memset(RC, 0, sizeof RC); uint64_t r = callf(fle_imp_GetUpdateRect, h, A(RC), 0, 0); printf("%s %d %d %d %d %d\n", tag, (int)r, RC[0], RC[1], RC[2], RC[3]); }
#define SW_HIDE_ 0
void fle_imp_ScrollWindowEx(void);
void fle_imp_RedrawWindow(void);
static int32_t IRC[4];
#define CLIPCH 0x02000000u
static void invrect(uint64_t h, int l, int t, int r, int b) { IRC[0] = l; IRC[1] = t; IRC[2] = r; IRC[3] = b; callf(fle_imp_InvalidateRect, h, A(IRC), 0, 0); }
"##;

// ---------------------------------------------------------------------------------------------
// decision 0214: the window manager's share of SetWindowPos / ShowWindow / DestroyWindow for a child window
// ---------------------------------------------------------------------------------------------

const KID_SETUP: &str = r#"
    uint64_t top = cw(0x80, "t", POPUP | VIS, 0, 0, 400, 300, 0, 0, 0);
    uint64_t kid = cw(0, "k", CHILD | VIS, 10, 20, 100, 50, top, 1, 0);
    mksurf(top, 400, 300);
    fillpat(10, 20, 110, 70);
    clean(top); clean(kid);
    nwrote = 0;
"#;

fn run_p2(name: &str, body: &str) -> Vec<String> {
    let top = top_with(EXTRA_C, &[]);
    let o = run_src(name, &top, &format!("{KID_SETUP}\n{body}"));
    assert!(o.ok, "{name} did not exit normally (signal {:?}): {}\n{}", o.signal, o.stderr, o.stdout);
    o.stdout.lines().map(|s| s.to_string()).collect()
}

#[test]
fn a_child_that_moves_without_overlap_copies_its_bits_and_leaves_the_old_place_invalid_in_the_parent() {
    // SetWindowPos(kid, x 100, y 120, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE): the valid contents of the client area are copied to the new place (Learn SWP_NOCOPYBITS), so the child has nothing to repaint;
    // the part of the parent that the move uncovers (the child's old rectangle (10,20)-(110,70)) is invalid in the parent (SWP_NOREDRAW: "any part of the parent window uncovered as a result of the window being moved").
    // the old pixels stay where they are until the parent paints; the pixel (10,20) was pat(10,20) = ff000a14 and (109,69) was ff006d45; the dirty rectangle of the copy is the new place (100,120)-(200,170).
    let l = run_p2("p2-move", r#"
    SWP(kid, 0, 100, 120, 0, 0, 0x1 | 0x4 | 0x10);
    upd("parent", top); upd("kid", kid);
    printf("new %08x %08x old %08x\n", *PX(100, 120), *PX(199, 169), *PX(10, 20));
    printf("wrote %d %lld %lld %lld %lld\n", nwrote, (long long)wr[0][0], (long long)wr[0][1], (long long)wr[0][2], (long long)wr[0][3]);
"#);
    assert_eq!(l[0], "parent 1 10 20 110 70");
    assert_eq!(l[1], "kid 0 0 0 0 0");
    assert_eq!(l[2], "new ff000a14 ff006d45 old ff000a14");
    assert_eq!(l[3], "wrote 1 100 120 200 170");
}

#[test]
fn an_overlapping_move_copies_through_the_overlap_and_only_the_strip_that_is_no_longer_covered_is_invalid() {
    // the child moves right by 30: old (10,20)-(110,70), new (40,20)-(140,70). Uncovered in the parent: x 10..40 (the new rectangle covers the rest of the old one) = (10,20)-(40,70).
    // The rows are copied as if through a buffer: the new (40,20) holds the old (10,20) and the new (139,20) the old (109,20) although the destination overlaps the source.
    let l = run_p2("p2-overlap", r#"
    SWP(kid, 0, 40, 20, 0, 0, 0x1 | 0x4 | 0x10);
    upd("parent", top); upd("kid", kid);
    printf("new %08x %08x %08x\n", *PX(40, 20), *PX(139, 20), *PX(75, 45));
"#);
    assert_eq!(l[0], "parent 1 10 20 40 70");
    assert_eq!(l[1], "kid 0 0 0 0 0");
    // new (75,45) holds the old (45,45) = pat(45,45) = ff002d2d
    assert_eq!(l[2], "new ff000a14 ff006d14 ff002d2d");
}

#[test]
fn swp_noredraw_copies_the_bits_but_invalidates_nothing_and_swp_nocopybits_invalidates_the_whole_child() {
    let a = run_p2("p2-noredraw", r#"
    SWP(kid, 0, 100, 120, 0, 0, 0x1 | 0x4 | 0x10 | 0x8);
    upd("parent", top); upd("kid", kid);
    printf("new %08x\n", *PX(100, 120));
"#);
    assert_eq!(a[0], "parent 0 0 0 0 0");
    assert_eq!(a[1], "kid 0 0 0 0 0");
    assert_eq!(a[2], "new ff000a14");
    // SWP_NOCOPYBITS: nothing is copied: the child's whole client area is invalid at the new place, the new place keeps what it had (0), the parent still gets the uncovered old rectangle
    let b = run_p2("p2-nocopy", r#"
    SWP(kid, 0, 100, 120, 0, 0, 0x1 | 0x4 | 0x10 | 0x100);
    upd("parent", top); upd("kid", kid);
    printf("new %08x wrote %d\n", *PX(100, 120), nwrote);
"#);
    assert_eq!(b[0], "parent 1 10 20 110 70");
    assert_eq!(b[1], "kid 1 0 0 100 50");
    assert_eq!(b[2], "new 00000000 wrote 0");
}

#[test]
fn movewindow_repaint_false_is_swp_noredraw_and_true_invalidates_the_uncovered_part() {
    let a = run_p2("p2-movewindow-false", r#"
    callf6(fle_imp_MoveWindow, kid, 100, 120, 100, 50, 0);
    upd("parent", top); upd("kid", kid);
"#);
    assert_eq!(a[0], "parent 0 0 0 0 0");
    assert_eq!(a[1], "kid 0 0 0 0 0");
    let b = run_p2("p2-movewindow-true", r#"
    callf6(fle_imp_MoveWindow, kid, 100, 120, 100, 50, 1);
    upd("parent", top); upd("kid", kid);
"#);
    assert_eq!(b[0], "parent 1 10 20 110 70");
    assert_eq!(b[1], "kid 0 0 0 0 0");
}

#[test]
fn hiding_or_destroying_a_child_invalidates_what_it_covered_in_the_parent() {
    let a = run_p2("p2-hide", r#"
    callf(fle_imp_ShowWindow, kid, SW_HIDE_, 0, 0);
    upd("parent", top);
"#);
    assert_eq!(a[0], "parent 1 10 20 110 70");
    let b = run_p2("p2-destroy", r#"
    callf(fle_imp_DestroyWindow, kid, 0, 0, 0);
    upd("parent", top);
"#);
    assert_eq!(b[0], "parent 1 10 20 110 70");
}

#[test]
fn the_window_that_was_covered_by_the_moved_sibling_and_the_parent_each_get_their_uncovered_part() {
    // A (20,20)-(120,120) below B (60,60)-(160,160) (a new child goes to the BOTTOM of the z-order: Learn CreateWindowExW Remarks; so B is created first and A second). B moves right by 200 (it stays inside the 400-wide parent): B's old rectangle is uncovered.
    // The part inside A, (60,60)-(120,120), is A's: in A's client coordinates (40,40)-(100,100). The rest of B's old rectangle belongs to the parent: the L-shape (120..160 x 60..160) + (60..120 x 120..160), bounding (60,60)-(160,160).
    let o = run_src("p2-siblings", &top_with(EXTRA_C, &[]), r#"
    uint64_t top = cw(0x80, "t", POPUP | VIS, 0, 0, 400, 300, 0, 0, 0);
    uint64_t b = cw(0, "b", CHILD | VIS, 60, 60, 100, 100, top, 2, 0);
    uint64_t a = cw(0, "a", CHILD | VIS, 20, 20, 100, 100, top, 1, 0);
    mksurf(top, 400, 300);
    clean(top); clean(a); clean(b);
    SWP(b, 0, 260, 60, 0, 0, 0x1 | 0x4 | 0x10);
    upd("parent", top); upd("a", a); upd("b", b);
"#);
    assert!(o.ok, "{}", o.stderr);
    let l: Vec<&str> = o.stdout.lines().collect();
    assert_eq!(l[0], "parent 1 60 60 160 160");
    assert_eq!(l[1], "a 1 40 40 100 100");
    assert_eq!(l[2], "b 0 0 0 0 0");
}

#[test]
fn a_top_level_window_that_moves_invalidates_nothing_in_itself() {
    // each top-level window has its own surface and the browser composes them (decision 0141): moving one needs no copy and no repaint
    let l = run_p2("p2-toplevel", r#"
    SWP(top, 0, 50, 60, 0, 0, 0x1 | 0x4 | 0x10);
    upd("top", top); upd("kid", kid);
    printf("wrote %d\n", nwrote);
"#);
    assert_eq!(l[0], "top 0 0 0 0 0");
    assert_eq!(l[1], "kid 0 0 0 0 0");
    assert_eq!(l[2], "wrote 0");
}

#[test]
fn scrollwindowex_tells_the_dirty_rectangle_about_the_pixels_it_moved() {
    // ScrollWindowEx(top, 0, 10, NULL, NULL, NULL, NULL, 0): the client area moves down by 10: the destination (0,10)-(400,300) is written (before decision 0214 only the repaint of the uncovered strip reached the page)
    let l = run_p2("p2-scroll", r#"
    callf8(fle_imp_ScrollWindowEx, top, 0, 10, 0, 0, 0, 0, 0);
    printf("wrote %d %lld %lld %lld %lld\n", nwrote, (long long)wr[0][0], (long long)wr[0][1], (long long)wr[0][2], (long long)wr[0][3]);
    printf("moved %08x\n", *PX(10, 30));
"#);
    assert_eq!(l[0], "wrote 1 0 10 400 300");
    // the pixel at (10,30) now holds the old (10,20) = ff000a14
    assert_eq!(l[1], "moved ff000a14");
}

#[test]
fn a_sibling_that_goes_to_the_bottom_uncovers_the_part_of_the_one_it_was_covering() {
    // B above A (a new child goes to the bottom, so B is created first). B (60,60)-(160,160) covers A (20,20)-(120,120) in (60,60)-(120,120). SetWindowPos(B, HWND_BOTTOM, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE): the
    // order of the two is reversed, A is now the upper one: the part of A that B hid, (60,60)-(120,120) = (40,40)-(100,100) in A's client coordinates, has to be painted (Learn SetWindowPos hWndInsertAfter HWND_BOTTOM;
    // 'The WM_PAINT Message': the system sends it when changes to the window have altered the content of the client area). B loses area and gains none; the parent is not touched.
    let o = run_src("p2-restack", &top_with(EXTRA_C, &[]), r#"
    uint64_t top = cw(0x80, "t", POPUP | VIS, 0, 0, 400, 300, 0, 0, 0);
    uint64_t b = cw(0, "b", CHILD | VIS, 60, 60, 100, 100, top, 2, 0);
    uint64_t a = cw(0, "a", CHILD | VIS, 20, 20, 100, 100, top, 1, 0);
    mksurf(top, 400, 300);
    clean(top); clean(a); clean(b);
    SWP(b, 1, 0, 0, 0, 0, 0x2 | 0x1 | 0x10);
    upd("parent", top); upd("a", a); upd("b", b);
"#);
    assert!(o.ok, "{}", o.stderr);
    let l: Vec<&str> = o.stdout.lines().collect();
    assert_eq!(l[0], "parent 0 0 0 0 0");
    assert_eq!(l[1], "a 1 40 40 100 100");
    assert_eq!(l[2], "b 0 0 0 0 0");
}

#[test]
fn a_grandchild_that_moves_uncovers_its_parent_and_not_the_top_level_window() {
    // top (0,0,400x300) > kid (10,20, 200x100) > gk (5,5, 50x30): gk's rectangle on the surface is (15,25)-(65,55). gk moves right by 100 inside kid: the vacated (5,5)-(55,35) is kid's (kid has no
    // WS_CLIPCHILDREN: the parent paints under its children, so the system gives the uncovered part to the window beneath, which is kid); top is covered by kid there and gets nothing; gk's bits are copied.
    let l = run_p2("p2-grandchild", r#"
    uint64_t kid2 = cw(0, "k2", CHILD | VIS, 10, 200, 200, 100, top, 7, 0);
    uint64_t gk = cw(0, "g", CHILD | VIS, 5, 5, 50, 30, kid2, 8, 0);
    clean(top); clean(kid); clean(kid2); clean(gk);
    SWP(gk, 0, 105, 5, 0, 0, 0x1 | 0x4 | 0x10);
    upd("top", top); upd("kid2", kid2); upd("gk", gk);
"#);
    assert_eq!(l[0], "top 0 0 0 0 0");
    assert_eq!(l[1], "kid2 1 5 5 55 35");
    assert_eq!(l[2], "gk 0 0 0 0 0");
}


// ---------------------------------------------------------------------------------------------
// decision 0214, second diff: GAP-U1 (decision 0210, api2). The cases of lift/harness/api2/win_upd.c with Wine 10.0's answers (expected/win-upd-wine-10.0.txt) and the rule of Learn RedrawWindow
// ("non-WS_CLIPCHILDREN windows are recursively validated or invalidated until a WS_CLIPCHILDREN window is encountered") and Window Features / Clipping.
// ---------------------------------------------------------------------------------------------

fn run_u1(name: &str, body: &str) -> Vec<String> {
    let o = run_src(name, &top_with(EXTRA_C, &[]), body);
    assert!(o.ok, "{name} did not exit normally (signal {:?}): {}\n{}", o.signal, o.stderr, o.stdout);
    o.stdout.lines().map(|s| s.to_string()).collect()
}

const U1_TREE: &str = r#"
    uint64_t p = cw(0x80, "p", POPUP | VIS | PSTYLE, 0, 0, 200, 200, 0, 0, 0);
    uint64_t c = cw(0, "c", CHILD | VIS, 20, 20, 100, 100, p, 1, 0);
    clean(p); clean(c);
"#;

#[test]
fn invalidating_a_parent_without_clipchildren_invalidates_the_children_it_covers_like_wine() {
    // Wine 10.0: parent (0,0,200,200) without WS_CLIPCHILDREN, child at (20,20) 100x100: Invalidate(parent) -> parent (0,0,200,200), child (0,0,100,100); Invalidate(child) -> parent nothing; Invalidate(parent,[30,30,60,60]) -> parent
    // (30,30,60,60), child (10,10,40,40). With WS_CLIPCHILDREN the child is not reached.
    let body = |pstyle: &str| format!("#define PSTYLE {pstyle}\n{U1_TREE}\n    callf(fle_imp_InvalidateRect, p, 0, 0, 0); upd(\"p\", p); upd(\"c\", c); clean(p); clean(c);\n    callf(fle_imp_InvalidateRect, c, 0, 0, 0); upd(\"p\", p); upd(\"c\", c); clean(p); clean(c);\n    invrect(p, 30, 30, 60, 60); upd(\"p\", p); upd(\"c\", c);\n#undef PSTYLE\n");
    let a = run_u1("p2-u1-none", &body("0"));
    assert_eq!(a, vec!["p 1 0 0 200 200", "c 1 0 0 100 100", "p 0 0 0 0 0", "c 1 0 0 100 100", "p 1 30 30 60 60", "c 1 10 10 40 40"]);
    let b = run_u1("p2-u1-clip", &body("CLIPCH"));
    assert_eq!(b, vec!["p 1 0 0 200 200", "c 0 0 0 0 0", "p 0 0 0 0 0", "c 1 0 0 100 100", "p 1 30 30 60 60", "c 0 0 0 0 0"]);
}

#[test]
fn the_descent_stops_below_a_window_that_has_clipchildren_and_skips_hidden_and_untouched_children() {
    // top (no CLIPCHILDREN) > kid A (20,20 100x100, no CLIPCHILDREN) > grandchild (10,10 50x50); kid B (20,20 100x100, WS_CLIPCHILDREN) > grandchild (10,10 50x50); a hidden kid (WS_VISIBLE missing) and a kid outside the rectangle.
    // Invalidate(top, [0,0,60,60]): top (0,0,60,60); A gets (0,0,40,40) and passes it on: its grandchild (10,10)-(40,40) of A's coordinates = (0,0,30,30); B is reached but, having WS_CLIPCHILDREN, passes nothing on.
    let l = run_u1("p2-u1-chain", r#"
    uint64_t p = cw(0x80, "p", POPUP | VIS, 0, 0, 300, 300, 0, 0, 0);
    uint64_t bb = cw(0, "b", CHILD | VIS | CLIPCH, 20, 20, 100, 100, p, 2, 0);
    uint64_t gb = cw(0, "gb", CHILD | VIS, 10, 10, 50, 50, bb, 3, 0);
    uint64_t aa = cw(0, "a", CHILD | VIS, 20, 20, 100, 100, p, 1, 0);
    uint64_t ga = cw(0, "ga", CHILD | VIS, 10, 10, 50, 50, aa, 4, 0);
    uint64_t hid = cw(0, "h", CHILD, 20, 20, 100, 100, p, 5, 0);
    uint64_t far = cw(0, "f", CHILD | VIS, 200, 200, 50, 50, p, 6, 0);
    clean(p); clean(aa); clean(ga); clean(bb); clean(gb); clean(hid); clean(far);
    invrect(p, 0, 0, 60, 60);
    upd("p", p); upd("a", aa); upd("ga", ga); upd("b", bb); upd("gb", gb); upd("hidden", hid); upd("far", far);
"#);
    assert_eq!(l, vec!["p 1 0 0 60 60", "a 1 0 0 40 40", "ga 1 0 0 30 30", "b 1 0 0 40 40", "gb 0 0 0 0 0", "hidden 0 0 0 0 0", "far 0 0 0 0 0"]);
}

#[test]
fn redrawwindow_invalidate_follows_the_same_default_and_rdw_nochildren_leaves_the_children_alone() {
    // RDW_INVALIDATE (0x1) without RDW_ALLCHILDREN / RDW_NOCHILDREN: the default of Learn RedrawWindow (children of a non-WS_CLIPCHILDREN window are reached); with RDW_NOCHILDREN (0x40; decision 0262: it was written 0x800, which is RDW_NOFRAME) they are not.
    let l = run_u1("p2-u1-redraw", r#"
#define PSTYLE 0
    uint64_t p = cw(0x80, "p", POPUP | VIS, 0, 0, 200, 200, 0, 0, 0);
    uint64_t c = cw(0, "c", CHILD | VIS, 20, 20, 100, 100, p, 1, 0);
    clean(p); clean(c);
    callf(fle_imp_RedrawWindow, p, 0, 0, 0x1u); upd("p", p); upd("c", c); clean(p); clean(c);
    callf(fle_imp_RedrawWindow, p, 0, 0, 0x1u | 0x40u); upd("p", p); upd("c", c);
"#);
    assert_eq!(l, vec!["p 1 0 0 200 200", "c 1 0 0 100 100", "p 1 0 0 200 200", "c 0 0 0 0 0"]);
}


// ---- decision 0240 (gpu1): a snapshot that cannot be complete must not silently drop windows ----
// UNC_MAX windows can touch the changed rectangle, and a visible region may need at most FLE_VISRGN_MAX (128) rectangles. Before this decision a window beyond the 160th, and a window whose region needed more rectangles,
// was left out of the snapshot / skipped at the end: the part of it that the move uncovered was never invalidated and the old picture stayed ('trail'). Now the over-estimate is used: every visible window of the tree that the
// old or the new rectangle touches is invalid in the part the union covers, and no bits are taken for valid (Learn SetWindowPos: 'any part of the parent window uncovered as a result of the window being moved').
fn nums(line: &str) -> Vec<i32> {
    line.split_whitespace().skip(1).map(|t| t.parse().unwrap()).collect()
}

#[test]
fn a_parent_whose_visible_region_needs_more_than_128_rectangles_still_gets_the_part_the_move_uncovered() {
    // 70 one-pixel-high children on a diagonal below the kid cut the parent's visible region (WS_CLIPCHILDREN rules: its rectangle minus the visible children) into more than 128 rectangles
    let l = run_p2("p2-region-overflow", r#"
    int i;
    for (i = 0; i < 70; i++) cw(0, "d", CHILD | VIS, 10 + 5 * i, 100 + 2 * i, 3, 1, top, 100 + i, 0);
    clean(top); clean(kid);
    nwrote = 0;
    SWP(kid, 0, 40, 20, 0, 0, 0x1 | 0x4 | 0x10);
    upd("parent", top); upd("kid", kid);
"#);
    let p = nums(&l[0]);
    assert_eq!(p[0], 1, "the parent must be invalid: {l:?}");
    assert!(p[1] <= 10 && p[2] <= 20 && p[3] >= 40 && p[4] >= 70, "the parent's update rectangle must contain the uncovered strip (10,20)-(40,70): {l:?}");
}

#[test]
fn a_window_beyond_the_160th_that_touches_the_move_still_gets_the_part_the_move_uncovered() {
    // 170 grandchildren of the kid touch the rectangle of the move; a sibling made after them (so with a later table index) at the bottom of the z-order lies under the kid and is uncovered when the kid moves away
    let l = run_p2("p2-window-cap", r#"
    int i;
    for (i = 0; i < 170; i++) cw(0, "g", CHILD | VIS, 1 + (i % 90), 1 + (i / 90) * 20, 4, 4, kid, 200 + i, 0);
    uint64_t under = cw(0, "s", CHILD | VIS, 10, 20, 100, 50, top, 999, 0);
    SWP(under, 1, 0, 0, 0, 0, 0x1 | 0x2 | 0x10);
    clean(top); clean(kid); clean(under);
    nwrote = 0;
    SWP(kid, 0, 210, 20, 0, 0, 0x1 | 0x4 | 0x10);
    upd("under", under); upd("kid", kid);
"#);
    let u = nums(&l[0]);
    assert_eq!(u[0], 1, "the sibling under the kid must be invalid where the kid was: {l:?}");
    assert!(u[1] <= 0 && u[2] <= 0 && u[3] >= 100 && u[4] >= 50, "its update rectangle must contain the uncovered (10,20)-(110,70) (client (0,0)-(100,50)): {l:?}");
}

#[test]
fn a_window_with_70_clipped_children_can_be_painted_inside_a_small_update_rectangle() {
    // decision 0240: the visible region of a WS_CLIPCHILDREN window with 70 one-pixel-high children on a diagonal needs more than FLE_VISRGN_MAX (128) rectangles, so the DC of a BeginPaint used to stop the engine ('the visible region of the
    // window has more rectangles than this runtime keeps') although the update rectangle (100,0)-(200,50) touches none of the children. The region is now made for the update rectangle alone: the pixel inside it is drawn, the one outside it
    // (and under no child) is not.
    let l = run_p2("p2-dc-region-within", r#"
    static uint8_t PS[72];
    int i;
    uint64_t w2 = cw(0x80, "w2", POPUP | VIS | CLIPCH, 500, 400, 400, 300, 0, 0, 0);
    for (i = 0; i < 70; i++) cw(0, "d", CHILD | VIS, 10 + 5 * i, 100 + 2 * i, 3, 1, w2, 300 + i, 0);
    mksurf(w2, 400, 300);
    clean(w2);
    invrect(w2, 100, 0, 200, 50);
    uint64_t hdc = callf(fle_imp_BeginPaint, w2, A(PS), 0, 0);
    callf(fle_imp_SetPixelV, hdc, 150, 20, 0x0000ff);
    callf(fle_imp_SetPixelV, hdc, 150, 200, 0x0000ff);
    callf(fle_imp_EndPaint, w2, A(PS), 0, 0);
    printf("in %08x out %08x\n", *PX(150, 20), *PX(150, 200));
"#);
    let f: Vec<&str> = l[0].split_whitespace().collect();
    assert_eq!(f[0], "in");
    assert_ne!(f[1], "00000000", "the pixel inside the update rectangle is drawn: {l:?}");
    assert_eq!(f[3], "00000000", "the pixel outside the update rectangle is not: {l:?}");
}
