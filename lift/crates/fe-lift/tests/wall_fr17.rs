//! Tests of decision 0142 (docs/decisions/0142-window-visible-region.md, PROVISIONAL): the visible region of a window's DC (runtime/fle_visrgn.c, fle_win.c `fle_win_dc_region`, fle_gdi.c, fle_gdc.c) and the
//! z-order requests of the guest that decide it.
//!
//! Expected values are written here from the Microsoft Learn pages named in the decision (Window Features / Clipping and Z-Order, Window Styles WS_CLIPCHILDREN and WS_CLIPSIBLINGS, GetDCEx and its DCX_*
//! flags, BeginPaint, Window Class Styles, SetWindowPos, CreateWindowEx) and from the rectangle arithmetic done in the comments of each test, not read back from the C code. A window is a child of a pop-up
//! window at (0, 0) of 200 x 200, so surface coordinates are the client coordinates of the pop-up. A colour is a COLORREF 0x00BBGGRR: red 0000ff, blue ff0000, green 00ff00. EVERY ANSWER IS PROVISIONAL: nothing
//! is compared with a real Windows. A test that pins a choice without a source says so in its comment (S-... of the decision).
//!
//! 1. The rectangle-set arithmetic of fle_visrgn.c and the clip flags of a DC.
//! 2. Siblings: WS_CLIPSIBLINGS, the z-order, hidden and WS_EX_TRANSPARENT siblings, GetDCEx(DCX_CLIPSIBLINGS), a DC that outlives a z-order change.
//! 3. Children and parents: WS_CLIPCHILDREN, DCX_CLIPCHILDREN, the parent's rectangle, a sibling above an ancestor, a minimized parent, DCX_PARENTCLIP.
//! 4. BeginPaint: the DC is clipped to the update region until EndPaint (a class DC too); GetPixel / SetPixel follow the visible region.
//! 5. The z-order requests: hWndInsertAfter, HWND_TOP / HWND_BOTTOM, BringWindowToTop, SWP_NOZORDER, ShowWindow, and the eight requests that the guest makes for its three work-panel windows.
//! 6. A link without fle_visrgn.c clips to the client rectangle only, as before decision 0142.

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
    if let Ok(d) = std::env::var("FLE_FR17_SRC_DIR") {
        return PathBuf::from(d);
    }
    rt_dir()
}
fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-wallFR17/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

const RT_FILES: [&str; 22] = [
    "fle_thread.c", "fle_win.c", "fle_class.c", "fle_event.c", "fle_sys.c", "fle_imports.c", "fle_proc.c", "fle_callback.c", "fle_env.c", "fle_vfs.c", "fle_vmap.c", "fle_vol.c", "fle_host.c",
    "fle_time.c", "fle_gdc.c", "fle_gdi.c", "fle_cursor.c", "fle_raster.c", "fle_monitor.c", "fle_ole.c", "fle_reg.c", "fle_visrgn.c",
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

fn build(tag: &str, skip: &[&str]) -> Vec<PathBuf> {
    let dir = work_dir().join(etxt_common::per_process(tag));
    std::fs::create_dir_all(&dir).unwrap();
    let mut v = Vec::new();
    let ours = ["fle_thread.c", "fle_win.c", "fle_class.c", "fle_gdi.c", "fle_gdc.c", "fle_raster.c", "fle_visrgn.c"];
    for name in RT_FILES.iter().chain(RT_EXTRA.iter()) {
        if skip.contains(name) {
            continue;
        }
        let is_ours = ours.contains(name);
        let base = if is_ours { src_dir() } else { rt_dir() };
        let o = dir.join(name.replace(".c", ".o"));
        clang_c(&base.join(name), &o, is_ours);
        v.push(o);
    }
    v
}
fn objs() -> &'static Vec<PathBuf> {
    static O: OnceLock<Vec<PathBuf>> = OnceLock::new();
    O.get_or_init(|| build("obj-fr17", &[]))
}
fn objs_without_visrgn() -> &'static Vec<PathBuf> {
    static O: OnceLock<Vec<PathBuf>> = OnceLock::new();
    O.get_or_init(|| build("obj-fr17-novis", &["fle_visrgn.c"]))
}

struct Out {
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
#include "fle_visrgn.h"
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
static uint32_t paint_col;      /* WM_PAINT of the stub: BeginPaint, fill 1000 x 1000 with this colour (the DC clip does the cutting), EndPaint; 0 = the default processing */
static int paint_n;
static void wndproc(void)
{
    uint64_t h = R[1], msg = R[2], wp = R[8], lp = R[9], sp = R[4];
    if (msg == 0x0F && paint_col) {
        static uint8_t ps_[80];
        int32_t rc_[4] = { 0, 0, 1000, 1000 };
        uint64_t dc_, br_;
        paint_n++;
        R[4] = sp - 8; fle_st(R[4], 8, 0xAAAA0002);
        dc_ = callf(fle_imp_BeginPaint, h, A(ps_), 0, 0);
        br_ = callf(fle_imp_CreateSolidBrush, paint_col, 0, 0, 0);
        callf(fle_imp_FillRect, dc_, A(rc_), br_, 0);
        callf(fle_imp_EndPaint, h, A(ps_), 0, 0);
        R[4] = sp; R[0] = 0; return;
    }
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
static uint16_t cls_own[] = { 'T','O','w','n','D','C', 0 };
static uint16_t cls_par[] = { 'T','P','a','r','D','C', 0 };
static uint16_t *use_cls = cls_name;
static void regclass(void)
{
    static uint8_t wc[72];
    uint64_t w = VA_WP, inst = 0x400000, nm = A(cls_name);
    uint32_t own_style = 0x20;
    if (cls_done) return;
    memset(wc, 0, sizeof wc); memcpy(wc + 8, &w, 8); memcpy(wc + 24, &inst, 8); memcpy(wc + 64, &nm, 8);
    CALL4(fle_imp_GetClassInfoW, 0x400000, A(cls_name), A(wc), 0);
    CALL4(fle_imp_RegisterClassW, A(wc), 0, 0, 0);
    /* a second class whose style is CS_OWNDC (0x20): its windows have ONE private DC */
    nm = A(cls_own);
    memset(wc, 0, sizeof wc); memcpy(wc, &own_style, 4); memcpy(wc + 8, &w, 8); memcpy(wc + 24, &inst, 8); memcpy(wc + 64, &nm, 8);
    CALL4(fle_imp_GetClassInfoW, 0x400000, A(cls_own), A(wc), 0);
    CALL4(fle_imp_RegisterClassW, A(wc), 0, 0, 0);
    /* a third class whose style is CS_PARENTDC (0x80) */
    own_style = 0x80; nm = A(cls_par);
    memset(wc, 0, sizeof wc); memcpy(wc, &own_style, 4); memcpy(wc + 8, &w, 8); memcpy(wc + 24, &inst, 8); memcpy(wc + 64, &nm, 8);
    CALL4(fle_imp_GetClassInfoW, 0x400000, A(cls_par), A(wc), 0);
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
    R[1] = ex; R[2] = A(use_cls); R[8] = title ? S(title) : 0; R[9] = style;
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

#define CLIPSIB 0x04000000u
#define CLIPCH  0x02000000u
#define RED   0x0000ffu
#define BLUE  0xff0000u
#define GREEN 0x00ff00u
#define NMNS  0x13u   /* SWP_NOSIZE | SWP_NOMOVE | SWP_NOACTIVATE */
#define NMN   0x03u   /* SWP_NOSIZE | SWP_NOMOVE (the window is activated) */
static uint64_t TOP(int x, int y, int w, int h) { use_cls = cls_name; return cw(0, "top", POPUP | VIS, x, y, w, h, 0, 0, 0); }
static uint64_t KID(uint64_t par, uint64_t extra, uint64_t ex, int x, int y, int w, int h) { use_cls = cls_name; return cw(ex, "kid", CHILD | VIS | extra, x, y, w, h, par, 0, 0); }
static uint64_t KIDO(uint64_t par, uint64_t extra, int x, int y, int w, int h) { uint64_t r; use_cls = cls_own; r = cw(0, "own", CHILD | VIS | extra, x, y, w, h, par, 0, 0); use_cls = cls_name; return r; }
static uint64_t KIDP(uint64_t par, uint64_t extra, int x, int y, int w, int h) { uint64_t r; use_cls = cls_par; r = cw(0, "par", CHILD | VIS | extra, x, y, w, h, par, 0, 0); use_cls = cls_name; return r; }
static uint32_t LIDOF(uint64_t h) { fle_win_dcq q; fle_win_dc_query(h, &q); return q.root_lid; }
static uint32_t PIX(uint32_t lid, int x, int y)
{
    const fle_rsurf *s = fle_raster_surface_peek(lid);
    return (s && x >= 0 && y >= 0 && (uint32_t)x < s->width && (uint32_t)y < s->height) ? fle_r_get(s, (uint32_t)x, (uint32_t)y) : 0xEEEEEEu;
}
static void FILLDC(uint64_t dc, int l, int t, int r, int b, uint32_t col)
{
    int32_t rc[4] = { l, t, r, b };
    uint64_t br = callf(fle_imp_CreateSolidBrush, col, 0, 0, 0);
    callf(fle_imp_FillRect, dc, A(rc), br, 0);
}
static void FILLW(uint64_t hwnd, int l, int t, int r, int b, uint32_t col)
{
    uint64_t dc = callf(fle_imp_GetDC, hwnd, 0, 0, 0);
    FILLDC(dc, l, t, r, b, col);
    callf(fle_imp_ReleaseDC, hwnd, dc, 0, 0);
}
static void FILLX(uint64_t hwnd, uint32_t dcx, int l, int t, int r, int b, uint32_t col)
{
    uint64_t dc = callf(fle_imp_GetDCEx, hwnd, 0, dcx, 0);
    FILLDC(dc, l, t, r, b, col);
    callf(fle_imp_ReleaseDC, hwnd, dc, 0, 0);
}
#define SHOW(label, v) printf("%s %06x\n", label, (unsigned)(v))
static void ORD(const char *label, uint64_t parent, const uint64_t *hs, const char *names, int n)
{
    char buf[16];
    int k = 0;
    uint64_t h = callf(fle_imp_GetTopWindow, parent, 0, 0, 0);
    while (h && k < 15) {
        char c = '?';
        for (int i = 0; i < n; i++) if (hs[i] == h) c = names[i];
        buf[k++] = c;
        h = callf(fle_imp_GetWindow, h, 2, 0, 0);
    }
    buf[k] = 0;
    printf("%s %s\n", label, buf);
}
"##;

fn run_src(name: &str, objs: &[PathBuf], body: &str) -> Out {
    let dir = work_dir();
    let uname = etxt_common::uniq(name);
    let c = dir.join(format!("{uname}.c"));
    let exe = dir.join(&uname);
    let src = format!("{PRELUDE}\nconst fle_fn_entry fle_dispatch_table[] = {{ {{ VA_WP, wndproc }} }};\nconst uint64_t fle_dispatch_count = 1;\nint main(void)\n{{\n    boot();\n{body}\n    return 0;\n}}\n");
    std::fs::write(&c, src).unwrap();
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fPIE", "-pthread", "-I"]).arg(src_dir()).arg("-I").arg(rt_dir());
    for o in objs {
        cmd.arg(o);
    }
    let cc = cmd.arg("-o").arg(&exe).arg(&c).arg("-lm").env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new(&exe).env("TMPDIR", dir.join("tmp")).output().expect("run failed");
    Out { stdout: String::from_utf8_lossy(&r.stdout).to_string(), stderr: String::from_utf8_lossy(&r.stderr).to_string(), signal: r.status.signal(), ok: r.status.success() }
}

fn lines_with(name: &str, o: &[PathBuf], body: &str) -> Vec<String> {
    let r = run_src(name, o, body);
    assert!(r.ok, "{name} did not exit normally (signal {:?}): {}\n{}", r.signal, r.stderr, r.stdout);
    r.stdout.lines().map(|s| s.to_string()).collect()
}
fn lines(name: &str, body: &str) -> Vec<String> {
    lines_with(name, objs(), body)
}
fn check(name: &str, body: &str, want: &[&str]) {
    let got = lines(name, body);
    assert_eq!(got, want, "{name}");
}

// ---------------------------------------------------------------------------------------------
// 1. The arithmetic and the flags
// ---------------------------------------------------------------------------------------------

#[test]
fn the_rectangle_set_arithmetic_keeps_the_area_of_the_set_difference_and_the_intersection() {
    // A 10 x 10 square minus a 3 x 3 hole in the middle: 91 points in 4 pieces (above 3 x 10... the pieces are the rows above and below and the two side pieces of the hole's rows); minus a strip
    // that spans the width: the two rows above and below (2 pieces, no side pieces). The area is counted by testing every point of a 12 x 12 grid, which is independent of the piece layout.
    let body = r#"
    fle_visrgn v;
    int n, x, y;
    fle_visrgn_set_rect(&v, (fle_rrect){ 0, 0, 10, 10 });
    printf("set %u\n", v.n);
    printf("sub %d\n", fle_visrgn_subtract(&v, (fle_rrect){ 3, 3, 6, 6 }));
    for (n = 0, y = -1; y < 11; y++) for (x = -1; x < 11; x++) n += fle_visrgn_contains(&v, x, y);
    printf("area %d pieces %u\n", n, v.n);
    printf("hole %d %d edge %d %d\n", fle_visrgn_contains(&v, 3, 3), fle_visrgn_contains(&v, 5, 5), fle_visrgn_contains(&v, 6, 6), fle_visrgn_contains(&v, 2, 3));
    { fle_rrect b = fle_visrgn_bounds(&v); printf("bounds %d %d %d %d\n", (int)b.x0, (int)b.y0, (int)b.x1, (int)b.y1); }
    fle_visrgn_set_rect(&v, (fle_rrect){ 0, 0, 10, 10 });
    fle_visrgn_subtract(&v, (fle_rrect){ -5, 4, 15, 6 });
    for (n = 0, y = -1; y < 11; y++) for (x = -1; x < 11; x++) n += fle_visrgn_contains(&v, x, y);
    printf("strip area %d pieces %u\n", n, v.n);
    fle_visrgn_subtract(&v, (fle_rrect){ -5, -5, 15, 4 });
    printf("minus top rows %u %d\n", v.n, fle_visrgn_equal_rect(&v, (fle_rrect){ 0, 6, 10, 10 }));
    fle_visrgn_intersect_rect(&v, (fle_rrect){ 2, 0, 5, 8 });
    printf("intersect %u %d\n", v.n, fle_visrgn_equal_rect(&v, (fle_rrect){ 2, 6, 5, 8 }));
    fle_visrgn_subtract(&v, (fle_rrect){ 0, 0, 100, 100 });
    printf("all gone %u\n", v.n);
    { fle_rrect b = fle_visrgn_bounds(&v); printf("empty bounds %d\n", (int)(b.x1 - b.x0)); }
    /* two holes that together cut a 10 x 10 slot out of a 30 x 10 strip: the pieces that touch with the same width are one piece (2 pieces, not 4) */
    fle_visrgn_set_rect(&v, (fle_rrect){ 0, 0, 30, 10 });
    fle_visrgn_subtract(&v, (fle_rrect){ 10, 0, 20, 5 });
    printf("after the first hole %u\n", v.n);
    fle_visrgn_subtract(&v, (fle_rrect){ 10, 5, 20, 10 });
    for (n = 0, y = 0; y < 10; y++) for (x = 0; x < 30; x++) n += fle_visrgn_contains(&v, x, y);
    printf("merged %u area %d\n", v.n, n);
    "#;
    check("fr17-arith", body, &[
        "set 1",
        "sub 1",
        "area 91 pieces 4",
        "hole 0 0 edge 1 1",
        "bounds 0 0 10 10",
        "strip area 80 pieces 2",
        "minus top rows 1 1",
        "intersect 1 1",
        "all gone 0",
        "empty bounds 0",
        "after the first hole 3",
        "merged 2 area 200",
    ]);
}

#[test]
fn two_regions_intersect_piece_by_piece_and_a_region_that_would_need_too_many_pieces_is_refused_unchanged() {
    // a 600 x 1 strip with one-pixel holes cannot be held in FLE_VISRGN_MAX (128) pieces: the call that needs the 129th piece returns 0 and leaves the region as it was
    let body = r#"
    fle_visrgn a, b;
    int i, ok = 1;
    unsigned before = 0;
    fle_visrgn_set_rect(&a, (fle_rrect){ 0, 0, 10, 10 });
    fle_visrgn_set_rect(&b, (fle_rrect){ 5, 5, 20, 20 });
    printf("meet %d %u %d\n", fle_visrgn_intersect(&a, &b), a.n, fle_visrgn_equal_rect(&a, (fle_rrect){ 5, 5, 10, 10 }));
    fle_visrgn_set_rect(&a, (fle_rrect){ 0, 0, 600, 1 });
    for (i = 0; i < 130 && ok; i++) {
        before = a.n;
        ok = fle_visrgn_subtract(&a, (fle_rrect){ 4 * i + 1, 0, 4 * i + 2, 1 });
        if (!ok) printf("refused at hole %d, pieces still %u (was %u)\n", i, a.n, before);
    }
    printf("max %u\n", (unsigned)FLE_VISRGN_MAX);
    "#;
    let got = lines("fr17-limits", body);
    assert_eq!(got[0], "meet 1 1 1");
    // holes at x = 1, 5, 9, ...: each hole splits a piece, so k holes leave k + 1 pieces; 127 holes leave 128 (the most it holds); the 128th hole (i = 127) would need 129 pieces and is refused
    assert_eq!(got[1], "refused at hole 127, pieces still 128 (was 128)", "{got:?}");
    assert_eq!(got[2], "max 128");
}

#[test]
fn the_clip_flags_of_a_dc_follow_the_styles_and_the_getdcex_flags() {
    let body = r#"
    /* from_getdcex, dcx, class style, window style */
    printf("getdc none %u\n", fle_visrgn_dc_flags(0, 0, 0, 0));
    printf("getdc sib %u\n", fle_visrgn_dc_flags(0, 0, 0, CLIPSIB));
    printf("getdc both %u\n", fle_visrgn_dc_flags(0, 0, 0, CLIPSIB | CLIPCH));
    printf("getdc parentdc class %u\n", fle_visrgn_dc_flags(0, 0, 0x80, 0));
    printf("ex 0 with styles %u\n", fle_visrgn_dc_flags(1, 0, 0, CLIPSIB | CLIPCH));
    printf("ex sib %u\n", fle_visrgn_dc_flags(1, 0x10, 0, 0));
    printf("ex children %u\n", fle_visrgn_dc_flags(1, 0x08, 0, 0));
    printf("ex parentclip %u\n", fle_visrgn_dc_flags(1, 0x20, 0, 0));
    printf("ex usestyle %u\n", fle_visrgn_dc_flags(1, 0x10000, 0, CLIPSIB | CLIPCH));
    printf("ex window only %u\n", fle_visrgn_dc_flags(1, 0x01, 0, CLIPSIB | CLIPCH));
    "#;
    // FLE_VR_CLIPCHILDREN 1, FLE_VR_CLIPSIBLINGS 2, FLE_VR_PARENTCLIP 4
    check("fr17-flags", body, &["getdc none 0", "getdc sib 2", "getdc both 3", "getdc parentdc class 4", "ex 0 with styles 0", "ex sib 2", "ex children 1", "ex parentclip 4", "ex usestyle 3", "ex window only 0"]);
}

// ---------------------------------------------------------------------------------------------
// 2. Siblings
// ---------------------------------------------------------------------------------------------

/// A (10,10)-(110,110) above B (60,60)-(160,160), both children of a 200 x 200 pop-up (a child is created at the BOTTOM of the z-order: Learn CreateWindowEx, so the window created first is above). A and B overlap in
/// (60,60)-(110,110). `a_style` / `b_style` / `a_ex` are the extra style bits of the windows; `body` runs with `p a b lid` declared.
fn two(a_style: &str, b_style: &str, a_ex: &str, body: &str) -> String {
    format!("uint64_t p = TOP(0, 0, 200, 200); uint64_t a = KID(p, {a_style}, {a_ex}, 10, 10, 100, 100); uint64_t b = KID(p, {b_style}, 0, 60, 60, 100, 100); uint32_t lid = LIDOF(p);\n{body}")
}
const SAMPLES: &str = r#"SHOW("a only", PIX(lid, 20, 20)); SHOW("both", PIX(lid, 70, 70)); SHOW("b only", PIX(lid, 130, 130)); SHOW("none", PIX(lid, 5, 5));"#;

#[test]
fn a_window_with_ws_clipsiblings_does_not_draw_where_a_sibling_above_it_lies() {
    // both have WS_CLIPSIBLINGS; A is above B: B's DC excludes A's rectangle, A's DC excludes nothing (nothing is above A). Learn WS_CLIPSIBLINGS: "clips all other overlapping child windows out of the region of
    // the child window to be updated"; GetDCEx DCX_CLIPSIBLINGS: "Excludes the visible regions of all sibling windows above the window". Drawn A first, then B: the overlap stays A's red.
    let body = two("CLIPSIB", "CLIPSIB", "0", &format!("FILLW(a, 0, 0, 100, 100, RED); FILLW(b, 0, 0, 100, 100, BLUE); {SAMPLES}"));
    check("fr17-sib-both", &body, &["a only 0000ff", "both 0000ff", "b only ff0000", "none 000000"]);
    // the other order gives the same picture (B drawn first, A over it)
    let body = two("CLIPSIB", "CLIPSIB", "0", &format!("FILLW(b, 0, 0, 100, 100, BLUE); FILLW(a, 0, 0, 100, 100, RED); {SAMPLES}"));
    check("fr17-sib-both-rev", &body, &["a only 0000ff", "both 0000ff", "b only ff0000", "none 000000"]);
}

#[test]
fn without_ws_clipsiblings_a_child_draws_in_the_client_area_of_its_neighbour() {
    // Learn WS_CLIPSIBLINGS: "If WS_CLIPSIBLINGS is not specified and child windows overlap, it is possible, when drawing within the client area of a child window, to draw within the client area of a neighboring child window".
    // B lacks the style: its fill covers the overlap although A is above it.
    let body = two("CLIPSIB", "0", "0", &format!("FILLW(a, 0, 0, 100, 100, RED); FILLW(b, 0, 0, 100, 100, BLUE); {SAMPLES}"));
    check("fr17-sib-b-free", &body, &["a only 0000ff", "both ff0000", "b only ff0000", "none 000000"]);
    // A lacks the style but nothing is above A: the style of the window that DRAWS decides, not that of the neighbour (B clipped, A over)
    let body = two("0", "CLIPSIB", "0", &format!("FILLW(b, 0, 0, 100, 100, BLUE); FILLW(a, 0, 0, 100, 100, RED); {SAMPLES}"));
    check("fr17-sib-a-free", &body, &["a only 0000ff", "both 0000ff", "b only ff0000", "none 000000"]);
}

#[test]
fn moving_a_window_to_the_top_changes_which_sibling_is_clipped_and_a_dc_got_before_follows() {
    // SetWindowPos(B, HWND_TOP) puts B above A (Learn SetWindowPos: HWND_TOP "Places the window at the top of the Z order"). Now A's DC excludes B's rectangle: the overlap is B's blue.
    let body = two("CLIPSIB", "CLIPSIB", "0", &format!("SWP(b, 0, 0, 0, 0, 0, NMNS); FILLW(a, 0, 0, 100, 100, RED); FILLW(b, 0, 0, 100, 100, BLUE); {SAMPLES}"));
    check("fr17-sib-swap", &body, &["a only 0000ff", "both ff0000", "b only ff0000", "none 000000"]);
    // a DC that was got BEFORE the change draws with the visible region of the moment of the drawing call (the system recomputes the visible region of the DCs when the z-order changes): S-region-at-draw
    let body = two("CLIPSIB", "CLIPSIB", "0", &format!("uint64_t dca = callf(fle_imp_GetDC, a, 0, 0, 0); SWP(b, 0, 0, 0, 0, 0, NMNS); FILLDC(dca, 0, 0, 100, 100, RED); FILLW(b, 0, 0, 100, 100, BLUE); {SAMPLES}"));
    check("fr17-sib-old-dc", &body, &["a only 0000ff", "both ff0000", "b only ff0000", "none 000000"]);
    // and back: A to the top again, B clipped
    let body = two("CLIPSIB", "CLIPSIB", "0", &format!("SWP(b, 0, 0, 0, 0, 0, NMNS); SWP(a, 0, 0, 0, 0, 0, NMNS); FILLW(a, 0, 0, 100, 100, RED); FILLW(b, 0, 0, 100, 100, BLUE); {SAMPLES}"));
    check("fr17-sib-swap-back", &body, &["a only 0000ff", "both 0000ff", "b only ff0000", "none 000000"]);
}

#[test]
fn a_hidden_sibling_and_a_ws_ex_transparent_sibling_take_nothing_out() {
    // Learn Window Features: a hidden window cannot display output, so it covers nothing; Extended Window Styles WS_EX_TRANSPARENT: "The window should not be painted until siblings beneath the window ... have
    // been painted": the siblings beneath must be able to draw there (S-ex-transparent)
    let body = two("CLIPSIB", "CLIPSIB", "0", &format!("callf(fle_imp_ShowWindow, a, 0, 0, 0); FILLW(b, 0, 0, 100, 100, BLUE); {SAMPLES}"));
    check("fr17-sib-hidden", &body, &["a only 000000", "both ff0000", "b only ff0000", "none 000000"]);
    let body = two("CLIPSIB", "CLIPSIB", "0", &format!("callf(fle_imp_ShowWindow, a, 0, 0, 0); callf(fle_imp_ShowWindow, a, 5, 0, 0); FILLW(a, 0, 0, 100, 100, RED); FILLW(b, 0, 0, 100, 100, BLUE); {SAMPLES}"));
    check("fr17-sib-shown-again", &body, &["a only 0000ff", "both 0000ff", "b only ff0000", "none 000000"]);
    let body = two("CLIPSIB", "CLIPSIB", "0x20", &format!("FILLW(a, 0, 0, 100, 100, RED); FILLW(b, 0, 0, 100, 100, BLUE); {SAMPLES}"));
    check("fr17-sib-transparent", &body, &["a only 0000ff", "both ff0000", "b only ff0000", "none 000000"]);
}

#[test]
fn getdcex_clips_siblings_by_its_flags_and_by_the_style_only_with_the_usestyle_bit() {
    // DCX_CLIPSIBLINGS 0x10 on a window without the style: clipped
    let body = two("CLIPSIB", "0", "0", &format!("FILLW(a, 0, 0, 100, 100, RED); FILLX(b, 0x10, 0, 0, 100, 100, BLUE); {SAMPLES}"));
    check("fr17-ex-sib-flag", &body, &["a only 0000ff", "both 0000ff", "b only ff0000", "none 000000"]);
    // GetDCEx with flags 0 on a window WITH the style: the flags are literal, nothing excluded
    let body = two("CLIPSIB", "CLIPSIB", "0", &format!("FILLW(a, 0, 0, 100, 100, RED); FILLX(b, 0, 0, 0, 100, 100, BLUE); {SAMPLES}"));
    check("fr17-ex-zero", &body, &["a only 0000ff", "both ff0000", "b only ff0000", "none 000000"]);
    // DCX_USESTYLE 0x10000 (UNOFFICIAL value, S-dcx-usestyle): the style applies
    let body = two("CLIPSIB", "CLIPSIB", "0", &format!("FILLW(a, 0, 0, 100, 100, RED); FILLX(b, 0x10000, 0, 0, 100, 100, BLUE); {SAMPLES}"));
    check("fr17-ex-usestyle", &body, &["a only 0000ff", "both 0000ff", "b only ff0000", "none 000000"]);
    // GetWindowDC (the whole window; the non-client area is 0 here) follows the style like GetDC
    let body = two("CLIPSIB", "CLIPSIB", "0", &format!("{{ uint64_t dc = callf(fle_imp_GetWindowDC, b, 0, 0, 0); FILLW(a, 0, 0, 100, 100, RED); FILLDC(dc, 0, 0, 100, 100, BLUE); callf(fle_imp_ReleaseDC, b, dc, 0, 0); }} {SAMPLES}"));
    check("fr17-windowdc", &body, &["a only 0000ff", "both 0000ff", "b only ff0000", "none 000000"]);
}

// ---------------------------------------------------------------------------------------------
// 3. Children, parents, ancestors
// ---------------------------------------------------------------------------------------------

#[test]
fn ws_clipchildren_keeps_the_parents_drawing_out_of_its_visible_children() {
    // P2 (10,10)-(160,160) with a child C at (20,20) of 50 x 50 = surface (30,30)-(80,80). Learn WS_CLIPCHILDREN: "Excludes the area occupied by child windows when drawing occurs within the parent window".
    let sample = r#"SHOW("p2 only", PIX(lid, 100, 100)); SHOW("under c", PIX(lid, 50, 50)); SHOW("outside", PIX(lid, 170, 170));"#;
    let mk = |st: &str, extra: &str| format!("uint64_t p = TOP(0, 0, 200, 200); uint64_t p2 = KID(p, {st}, 0, 10, 10, 150, 150); uint64_t c = KID(p2, 0, 0, 20, 20, 50, 50); uint32_t lid = LIDOF(p); {extra}");
    check("fr17-clipch", &mk("CLIPCH", &format!("FILLW(p2, 0, 0, 150, 150, GREEN); {sample}")), &["p2 only 00ff00", "under c 000000", "outside 000000"]);
    check("fr17-noclipch", &mk("0", &format!("FILLW(p2, 0, 0, 150, 150, GREEN); {sample}")), &["p2 only 00ff00", "under c 00ff00", "outside 000000"]);
    // DCX_CLIPCHILDREN 0x08 on a window without the style
    check("fr17-ex-clipch", &mk("0", &format!("FILLX(p2, 0x08, 0, 0, 150, 150, GREEN); {sample}")), &["p2 only 00ff00", "under c 000000", "outside 000000"]);
    // a hidden child takes nothing out
    check("fr17-clipch-hidden", &mk("CLIPCH", &format!("callf(fle_imp_ShowWindow, c, 0, 0, 0); FILLW(p2, 0, 0, 150, 150, GREEN); {sample}")), &["p2 only 00ff00", "under c 00ff00", "outside 000000"]);
}

#[test]
fn a_child_is_kept_inside_its_parents_client_rectangle() {
    // Learn Window Features / Positioning: "No part of a child window ever appears outside the borders of its parent window". C (20,20) of 100 x 100 in P2 (10,10) of 50 x 50: P2's client is surface (10,10)-(60,60),
    // C's rectangle (30,30)-(130,130); the DC of C reaches (30,30)-(60,60) only.
    let body = "uint64_t p = TOP(0, 0, 200, 200); uint64_t p2 = KID(p, 0, 0, 10, 10, 50, 50); uint64_t c = KID(p2, 0, 0, 20, 20, 100, 100); uint32_t lid = LIDOF(p); FILLW(c, 0, 0, 100, 100, GREEN); SHOW(\"in\", PIX(lid, 40, 40)); SHOW(\"beyond parent\", PIX(lid, 80, 80)); SHOW(\"left of c\", PIX(lid, 20, 20));";
    check("fr17-parent-bounds", body, &["in 00ff00", "beyond parent 000000", "left of c 000000"]);
}

#[test]
fn a_sibling_above_an_ancestor_with_ws_clipsiblings_covers_its_descendants_too() {
    // Y (70,70)-(170,170) is created first, so it is above X (10,10)-(130,130); Z is a child of X at (40,40) of 60 x 60 = surface (50,50)-(110,110); Z and Y overlap in (70,70)-(110,110). Learn Window Features /
    // Z-Order: "A child window is grouped with its parent in z-order": what covers X covers Z. X has WS_CLIPSIBLINGS: its visible region excludes Y, and Z lives inside it (S-ancestor-siblings).
    let mk = |xst: &str| format!("uint64_t p = TOP(0, 0, 200, 200); uint64_t y = KID(p, 0, 0, 70, 70, 100, 100); uint64_t x = KID(p, {xst}, 0, 10, 10, 120, 120); uint64_t z = KID(x, 0, 0, 40, 40, 60, 60); uint32_t lid = LIDOF(p); FILLW(z, 0, 0, 60, 60, GREEN); SHOW(\"z only\", PIX(lid, 60, 60)); SHOW(\"z under y\", PIX(lid, 90, 90));");
    check("fr17-ancestor", &mk("CLIPSIB"), &["z only 00ff00", "z under y 000000"]);
    // X without the style: nothing excludes Y from Z's region
    check("fr17-ancestor-free", &mk("0"), &["z only 00ff00", "z under y 00ff00"]);
}

#[test]
fn a_window_with_a_minimized_ancestor_has_an_empty_visible_region() {
    // S-minimized-empty: Learn Window Features, Minimized: "the system reduces a minimized window to the size of its taskbar button"; the client area is not shown
    // decision 0267 (alpha10): a minimized top-level window is 0 x 0 (Wine 10.0), so it has no surface to make; the surface the first fill made while the window was restored (blue, read back as ff0000) keeps its pixels and the fill while it is minimized (green) draws nothing. Before alpha10 the minimized window kept its rectangle, a surface was made by the refused fill and the pixel read 000000.
    let body = "uint64_t p = TOP(0, 0, 200, 200); uint64_t k = KID(p, 0, 0, 10, 10, 100, 100); uint32_t lid = LIDOF(p); FILLW(k, 0, 0, 100, 100, BLUE); callf(fle_imp_ShowWindow, p, 2, 0, 0); FILLW(k, 0, 0, 100, 100, GREEN); SHOW(\"minimized\", PIX(lid, 50, 50)); callf(fle_imp_ShowWindow, p, 9, 0, 0); FILLW(k, 0, 0, 100, 100, GREEN); SHOW(\"restored\", PIX(lid, 50, 50));";
    check("fr17-minimized", body, &["minimized ff0000", "restored 00ff00"]);
}

#[test]
fn dcx_parentclip_gives_the_visible_region_of_the_parent_with_the_origin_of_the_window() {
    // Learn GetDCEx DCX_PARENTCLIP: "Uses the visible region of the parent window ... The origin is set to the upper-left corner of the window identified by hWnd". B (60,60) of 100 x 100: a fill at (-30,-30)-(-10,-10)
    // of its DC is the surface rectangle (30,30)-(50,50), inside the parent (the pop-up, 0..200) and outside B.
    let body = "uint64_t p = TOP(0, 0, 200, 200); uint64_t b = KID(p, CLIPSIB, 0, 60, 60, 100, 100); uint32_t lid = LIDOF(p); FILLX(b, 0x20, -30, -30, -10, -10, BLUE); SHOW(\"outside b\", PIX(lid, 40, 40)); FILLX(b, 0, -30, -30, -10, -10, RED); SHOW(\"without the flag\", PIX(lid, 40, 40)); SHOW(\"inside parent only\", PIX(lid, 150, 20));";
    check("fr17-parentclip", body, &["outside b ff0000", "without the flag ff0000", "inside parent only 000000"]);
}

#[test]
fn a_window_of_a_cs_parentdc_class_gets_the_visible_region_of_its_parent() {
    // Learn Window Class Styles, CS_PARENTDC: "Sets the clipping rectangle of the child window to that of the parent window so that the child can draw on the parent" (S-parentdc-region: read as the visible region
    // of the parent, DCX_PARENTCLIP). The child at (60,60) of 100 x 100 fills (-30,-30)-(-10,-10) of its DC = the surface rectangle (30,30)-(50,50), outside the child and inside the pop-up.
    let body = "uint64_t p = TOP(0, 0, 200, 200); uint64_t b = KIDP(p, CLIPSIB, 60, 60, 100, 100); uint64_t c = KID(p, CLIPSIB, 0, 60, 60, 100, 100); uint32_t lid = LIDOF(p); FILLW(b, -30, -30, -10, -10, BLUE); SHOW(\"parentdc outside the child\", PIX(lid, 40, 40)); FILLW(c, -30, -30, -10, -10, RED); SHOW(\"common dc, the same fill\", PIX(lid, 40, 40)); FILLW(b, 0, 0, 100, 100, GREEN); SHOW(\"inside the child\", PIX(lid, 100, 100));";
    check("fr17-parentdc-class", body, &["parentdc outside the child ff0000", "common dc, the same fill ff0000", "inside the child 00ff00"]);
}

// ---------------------------------------------------------------------------------------------
// 4. BeginPaint, GetPixel, SetPixel
// ---------------------------------------------------------------------------------------------

#[test]
fn the_dc_of_beginpaint_is_clipped_to_the_update_region_and_a_later_getdc_is_not() {
    // W at (10,10) of 100 x 100 = surface (10,10)-(110,110); the stub's WM_PAINT fills 1000 x 1000 through the DC of BeginPaint. InvalidateRect (20,20)-(40,40) in client coordinates = surface (30,30)-(50,50).
    // Learn BeginPaint: "automatically sets the clipping region of the device context to exclude any area outside the update region"
    let body = r#"
    int32_t rc[4] = { 20, 20, 40, 40 };
    uint64_t p = TOP(0, 0, 200, 200), w = KID(p, 0, 0, 10, 10, 100, 100);
    uint32_t lid = LIDOF(p);
    paint_col = BLUE;
    callf(fle_imp_InvalidateRect, w, A(rc), 0, 0);
    callf(fle_imp_UpdateWindow, w, 0, 0, 0);
    printf("paints %d\n", paint_n);
    SHOW("in update", PIX(lid, 35, 35));
    SHOW("window outside update", PIX(lid, 60, 60));
    SHOW("outside window", PIX(lid, 5, 5));
    FILLW(w, 0, 0, 100, 100, RED);
    SHOW("getdc after endpaint", PIX(lid, 60, 60));
    paint_col = GREEN;
    callf(fle_imp_InvalidateRect, w, 0, 0, 0);
    callf(fle_imp_UpdateWindow, w, 0, 0, 0);
    SHOW("whole client invalid", PIX(lid, 105, 105));
    "#;
    check("fr17-beginpaint", body, &["paints 1", "in update ff0000", "window outside update 000000", "outside window 000000", "getdc after endpaint 0000ff", "whole client invalid 00ff00"]);
}

#[test]
fn the_private_dc_of_a_cs_owndc_window_is_clipped_by_beginpaint_only_until_endpaint() {
    let body = r#"
    int32_t rc[4] = { 20, 20, 40, 40 };
    uint64_t p = TOP(0, 0, 200, 200), w = KIDO(p, 0, 10, 10, 100, 100), dc1, dc2;
    uint32_t lid = LIDOF(p);
    dc1 = callf(fle_imp_GetDC, w, 0, 0, 0);
    paint_col = BLUE;
    callf(fle_imp_InvalidateRect, w, A(rc), 0, 0);
    callf(fle_imp_UpdateWindow, w, 0, 0, 0);
    SHOW("in update", PIX(lid, 35, 35));
    SHOW("outside update", PIX(lid, 60, 60));
    dc2 = callf(fle_imp_GetDC, w, 0, 0, 0);
    printf("same dc %d\n", dc1 == dc2);
    FILLDC(dc2, 0, 0, 100, 100, RED);
    SHOW("owndc after endpaint", PIX(lid, 60, 60));
    "#;
    check("fr17-owndc", body, &["in update ff0000", "outside update 000000", "same dc 1", "owndc after endpaint 0000ff"]);
}

#[test]
fn getpixel_and_setpixel_refuse_a_point_outside_the_visible_region() {
    // the point (70,70) of the surface is in A and B; B's DC excludes A (A above B, both WS_CLIPSIBLINGS). B's client coordinates of the surface point (70,70) are (10,10); (110,110) is (50,50), in B only.
    // Learn SetPixel: "The function fails if the pixel coordinates lie outside of the current clipping region" (-1); GetPixel: CLR_INVALID (0xffffffff) outside the clipping region.
    let body = two("CLIPSIB", "CLIPSIB", "0", r#"
    uint64_t dc = callf(fle_imp_GetDC, b, 0, 0, 0);
    printf("set hole %x\n", (unsigned)callf(fle_imp_SetPixel, dc, 10, 10, 0x0000ff));
    printf("pixel hole %x\n", (unsigned)PIX(lid, 70, 70));
    printf("set free %x\n", (unsigned)callf(fle_imp_SetPixel, dc, 50, 50, 0x0000ff));
    printf("pixel free %x\n", (unsigned)PIX(lid, 110, 110));
    printf("get hole %x\n", (unsigned)callf(fle_imp_GetPixel, dc, 10, 10, 0));
    printf("get free %x\n", (unsigned)callf(fle_imp_GetPixel, dc, 50, 50, 0));
    "#);
    check("fr17-pixel", &body, &["set hole ffffffff", "pixel hole 0", "set free ff", "pixel free ff", "get hole ffffffff", "get free ff"]);
}

// ---------------------------------------------------------------------------------------------
// 5. The z-order requests
// ---------------------------------------------------------------------------------------------

#[test]
fn set_window_pos_places_a_window_directly_below_hwndinsertafter_and_the_other_z_requests() {
    // A B C are children of one parent: a child is created at the bottom (Learn CreateWindowEx), so the order from the top is A B C. SetWindowPos hWndInsertAfter is "the window to precede the positioned window in
    // the Z order": the window goes directly BELOW it. The list is top first; the order is that of GetTopWindow followed by GetWindow(GW_HWNDNEXT) = "the window below" (Learn GetWindow).
    let body = r#"
    uint64_t p = TOP(0, 0, 200, 200), hs[3];
    hs[0] = KID(p, 0, 0, 0, 0, 20, 20); hs[1] = KID(p, 0, 0, 0, 0, 20, 20); hs[2] = KID(p, 0, 0, 0, 0, 20, 20);
    ORD("created", p, hs, "ABC", 3);
    SWP(hs[0], hs[2], 0, 0, 0, 0, NMNS);   ORD("A after C", p, hs, "ABC", 3);
    SWP(hs[2], 1, 0, 0, 0, 0, NMNS);       ORD("C to HWND_BOTTOM", p, hs, "ABC", 3);
    SWP(hs[0], 0, 0, 0, 0, 0, NMNS);       ORD("A to HWND_TOP", p, hs, "ABC", 3);
    callf(fle_imp_BringWindowToTop, hs[2], 0, 0, 0);   ORD("BringWindowToTop C", p, hs, "ABC", 3);
    SWP(hs[1], hs[0], 0, 0, 0, 0, NMNS | 0x04);        ORD("SWP_NOZORDER ignores hWndInsertAfter", p, hs, "ABC", 3);
    callf(fle_imp_ShowWindow, hs[0], 5, 0, 0);          ORD("ShowWindow SW_SHOW on a child", p, hs, "ABC", 3);
    callf(fle_imp_ShowWindow, hs[1], 4, 0, 0);          ORD("ShowWindow SW_SHOWNOACTIVATE", p, hs, "ABC", 3);
    SWP(hs[1], hs[2], 0, 0, 0, 0, NMN);                 ORD("B after C, activating", p, hs, "ABC", 3);
    "#;
    check("fr17-z-basic", body, &[
        "created ABC",
        "A after C BCA",
        "C to HWND_BOTTOM BAC",
        "A to HWND_TOP ABC",
        "BringWindowToTop C CAB",
        "SWP_NOZORDER ignores hWndInsertAfter CAB",
        "ShowWindow SW_SHOW on a child CAB",
        "ShowWindow SW_SHOWNOACTIVATE CAB",
        "B after C, activating CBA",
    ]);
}

#[test]
fn the_eight_z_order_requests_of_the_guest_for_its_channel_rack_playlist_and_mixer_leave_the_mixer_at_the_bottom() {
    // The requests in the order the guest made them (measured with FLE_RASTER_TRACE on the final configuration, lids 16 = Channel rack R, 53 = Playlist P, 64 = Mixer M, children of the work panel lid 15; hand-computed
    // lists, top first):
    //   create R                      -> R
    //   SetWindowPos(R, HWND_TOP, flags 0x13)       -> R
    //   SetWindowPos(R, HWND_BOTTOM, flags 0x03)    -> R
    //   create P (child: bottom)      -> R P
    //   SetWindowPos(P, HWND_TOP, 0x13)             -> P R
    //   create M (child: bottom)      -> P R M
    //   SetWindowPos(M, hWndInsertAfter = P, 0x13)  -> P M R
    //   SetWindowPos(R, HWND_TOP, 0x03)             -> R P M
    //   SetWindowPos(P, HWND_TOP, 0x03)             -> P R M
    //   SetWindowPos(R, HWND_TOP, 0x03)             -> R P M
    let body = r#"
    uint64_t p = TOP(0, 0, 200, 200), hs[3] = { 0, 0, 0 };
    hs[0] = KID(p, 0, 0, 0, 0, 20, 20);                      ORD("create R", p, hs, "RPM", 3);
    SWP(hs[0], 0, 0, 0, 0, 0, NMNS);                         ORD("R top", p, hs, "RPM", 3);
    SWP(hs[0], 1, 0, 0, 0, 0, NMN);                          ORD("R bottom", p, hs, "RPM", 3);
    hs[1] = KID(p, 0, 0, 0, 0, 20, 20);                      ORD("create P", p, hs, "RPM", 3);
    SWP(hs[1], 0, 0, 0, 0, 0, NMNS);                         ORD("P top", p, hs, "RPM", 3);
    hs[2] = KID(p, 0, 0, 0, 0, 20, 20);                      ORD("create M", p, hs, "RPM", 3);
    SWP(hs[2], hs[1], 0, 0, 0, 0, NMNS);                     ORD("M after P", p, hs, "RPM", 3);
    SWP(hs[0], 0, 0, 0, 0, 0, NMN);                          ORD("R top again", p, hs, "RPM", 3);
    SWP(hs[1], 0, 0, 0, 0, 0, NMN);                          ORD("P top again", p, hs, "RPM", 3);
    SWP(hs[0], 0, 0, 0, 0, 0, NMN);                          ORD("R top last", p, hs, "RPM", 3);
    "#;
    check("fr17-z-guest", body, &[
        "create R R",
        "R top R",
        "R bottom R",
        "create P RP",
        "P top PR",
        "create M PRM",
        "M after P PMR",
        "R top again RPM",
        "P top again PRM",
        "R top last RPM",
    ]);
}

// ---------------------------------------------------------------------------------------------
// 6. Without fle_visrgn.c
// ---------------------------------------------------------------------------------------------

#[test]
fn a_link_without_fle_visrgn_clips_a_window_dc_to_the_client_rectangle_only() {
    // the references to fle_visrgn.c are weak: tests that link only some runtime files (wall_fr7, wall_fr9, ...) see the clip of decision 0126 (S-clip-rects): B's fill covers the overlap although A is above it
    let body = format!("{}", two("CLIPSIB", "CLIPSIB", "0", &format!("FILLW(a, 0, 0, 100, 100, RED); FILLW(b, 0, 0, 100, 100, BLUE); {SAMPLES}")));
    let got = lines_with("fr17-novis", objs_without_visrgn(), &body);
    assert_eq!(got, ["a only 0000ff", "both ff0000", "b only ff0000", "none 000000"]);
}
