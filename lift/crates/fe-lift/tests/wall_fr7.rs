//! Tests of decision 0121 (docs/decisions/0121-screen-dc-and-fruity-walls.md): walls 18 to 20 of CreateFruityInstance.
//!   wall 18: the attributes of a screen DC (GetDC(NULL)) and LPtoDP in the metric modes; GetDeviceCaps(HORZSIZE, VERTSIZE) = 508, 286 (the PM's decision 1);
//!   wall 19: GetClassInfoW of QuickFontCache's own class name TSharedAnimTimerWindow (hInstance 0x1d00000);
//!   wall 20: CreateWindowExW of a message-only window (hWndParent HWND_MESSAGE).
//!
//! Expected values are written here from the Microsoft Learn pages named in the decision (Display Device Context Defaults, Mapping Modes and Translations, SetMapMode, LPtoDP,
//! GetDeviceCaps, CreateWindowEx, Window Features, [MS-WMF] MapMode Enumeration) and from integer arithmetic done in this file, not read back from the C code. EVERY ANSWER IS PROVISIONAL:
//! nothing is compared with a real Windows. The values that no page gives (508 and 286 mm, the rounding of LPtoDP, the state of a screen DC per GetDC handle) are choices of the
//! decision and are the only thing such a test pins down.
//!
//! 1. The GDI driver (fle_gdi.c, fle_gdc.c, ...): screen DC attributes, LPtoDP, what still stops.
//! 2. The window driver (fle_win.c, fle_class.c, ...): the class name of wall 19 and the message-only window of wall 20.
//! 3. The bytes of the real DLLs that the decision rests on (skipped when absent).

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
    if let Ok(d) = std::env::var("FLE_FR7_SRC_DIR") {
        return PathBuf::from(d);
    }
    rt_dir()
}
fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-wallFR7/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

fn clang_obj(src: &Path, obj: &Path, strict: bool) {
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-Wall", "-Wextra", "-fPIE", "-c"]);
    if strict {
        cmd.arg("-Werror");
    }
    let cc = cmd.arg("-I").arg(src_dir()).arg("-I").arg(rt_dir()).arg(src).arg("-o").arg(obj).env("TMPDIR", work_dir().join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {}: {}", src.display(), String::from_utf8_lossy(&cc.stderr));
}

/// The files of a driver: (name, taken from src_dir with -Werror, or from runtime/).
fn build_objs(tag: &str, files: &[(&str, bool)]) -> Vec<PathBuf> {
    let dir = work_dir().join(etxt_common::per_process(tag));
    std::fs::create_dir_all(&dir).unwrap();
    let mut v = Vec::new();
    for (name, ours) in files {
        let base = if *ours { src_dir() } else { rt_dir() };
        let o = dir.join(name.replace(".c", ".o"));
        clang_obj(&base.join(name), &o, *ours);
        v.push(o);
    }
    v
}

fn gdi_objs() -> &'static Vec<PathBuf> {
    static O: OnceLock<Vec<PathBuf>> = OnceLock::new();
    O.get_or_init(|| build_objs("obj-fr7-gdi", &[("fle_gdi.c", true), ("fle_raster.c", true), ("fle_gdc.c", true), ("fle_imports.c", false), ("fle_env.c", false), ("fle_cursor.c", false)]))
}
fn win_objs() -> &'static Vec<PathBuf> {
    static O: OnceLock<Vec<PathBuf>> = OnceLock::new();
    O.get_or_init(|| build_objs("obj-fr7-win", &[("fle_win.c", true), ("fle_class.c", true), ("fle_callback.c", false), ("fle_imports.c", false), ("fle_env.c", false)]))
}

struct Out {
    stdout: String,
    stderr: String,
    signal: Option<i32>,
    ok: bool,
}

fn run_with(objs: &[PathBuf], name: &str, prelude: &str, body: &str) -> Out {
    let dir = work_dir();
    let uname = etxt_common::uniq(name);
    let c = dir.join(format!("{uname}.c"));
    let exe = dir.join(&uname);
    std::fs::write(&c, format!("{prelude}\nint main(void)\n{{\n{body}\n    return 0;\n}}\n")).unwrap();
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fPIE", "-pthread", "-I"]).arg(src_dir()).arg("-I").arg(rt_dir());
    for o in objs {
        cmd.arg(o);
    }
    let cc = cmd.arg("-o").arg(&exe).arg(&c).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new(&exe).env("TMPDIR", dir.join("tmp")).output().expect("run failed");
    Out { stdout: String::from_utf8_lossy(&r.stdout).to_string(), stderr: String::from_utf8_lossy(&r.stderr).to_string(), signal: r.status.signal(), ok: r.status.success() }
}

// ---------------------------------------------------------------------------------------------
// 1. The GDI driver
// ---------------------------------------------------------------------------------------------

/// K(f, a, b, c, d) calls a runtime function like the guest does (rcx, rdx, r8, r9). STOP(label, statements) runs the statements in a forked child and prints
/// "label | signal | first line of stderr". A forked child has its own thread id, so the runtime must not have been called in the parent: every STOP does its own set-up.
const GDI_PRELUDE: &str = r##"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <unistd.h>
#include <pthread.h>
#include <sys/wait.h>
#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_gdc.h"
#include "fle_gdi.h"

uint64_t R[16];
static uint64_t stk[32];
static uint64_t kv(void (*f)(void), const uint64_t *a)
{
    memset(stk, 0, sizeof stk);
    stk[0] = 0xdead0000u;
    for (int i = 4; i < 9; i++) stk[i + 1] = a[i];    /* the fifth to the ninth argument: [rsp+0x28] ... (fle_arg) */
    R[1] = a[0]; R[2] = a[1]; R[8] = a[2]; R[9] = a[3];
    R[4] = (uint64_t)(uintptr_t)stk;
    R[0] = 0xBADBADBADull;
    f();
    return R[0];
}
#define K(f, ...) kv((void (*)(void))(f), (uint64_t[9]){ __VA_ARGS__ })
#define U(x) ((unsigned long long)(x))
#define A(p) ((uint64_t)(uintptr_t)(p))
#define NEG(n) ((uint64_t)(int64_t)(n))

static void pr(const char *label, uint64_t h)
{
    int32_t o[13] = { 0 };
    int k = fle_gdi_screen_attr(h, o);
    printf("%s %d |", label, k);
    for (int i = 0; i < 13; i++) printf(" %d", (int)o[i]);
    printf("\n");
}

static int pfd[2];
static pid_t spawn(void)
{
    pid_t p;
    fflush(stdout);
    fflush(stderr);
    if (pipe(pfd)) exit(2);
    p = fork();
    if (p == 0) { close(pfd[0]); dup2(pfd[1], 2); close(pfd[1]); }
    else close(pfd[1]);
    return p;
}
static void reap(const char *label, pid_t p)
{
    char buf[900], junk[256];
    ssize_t n = 0, k;
    int st = 0;
    char *nl;
    while ((k = read(pfd[0], n < (ssize_t)sizeof buf - 1 ? buf + n : junk, n < (ssize_t)sizeof buf - 1 ? sizeof buf - 1 - (size_t)n : sizeof junk)) > 0)
        if (n < (ssize_t)sizeof buf - 1) n += k;
    buf[n] = 0;
    nl = strchr(buf, '\n');
    if (nl) *nl = 0;
    close(pfd[0]);
    waitpid(p, &st, 0);
    printf("%s | %d | %s\n", label, WIFSIGNALED(st) ? WTERMSIG(st) : 0, buf);
}
#define STOP(label, ...) do { pid_t p_ = spawn(); if (p_ == 0) { __VA_ARGS__; _exit(0); } reap(label, p_); } while (0)
"##;

fn gdi_lines(name: &str, body: &str) -> Vec<String> {
    let o = run_with(gdi_objs(), name, GDI_PRELUDE, body);
    assert!(o.ok, "{name} did not exit normally (signal {:?}): {}\n{}", o.signal, o.stderr, o.stdout);
    o.stdout.lines().map(|s| s.to_string()).collect()
}

/// (label, statements run after `setup` in a child, a word of the message of fle_stop): every case must abort with SIGABRT and "fle: <api>: ..." containing the word.
fn gdi_stops(prefix: &str, setup: &str, api: &str, cases: &[(&str, &str, &str)]) {
    let mut body = String::new();
    for (label, stmts, _) in cases {
        body.push_str(&format!("    STOP(\"{label}\", {setup}\n    {stmts});\n"));
    }
    let l = gdi_lines(prefix, &body);
    assert_eq!(l.len(), cases.len(), "{prefix}: {l:?}");
    for (line, (label, _, word)) in l.iter().zip(cases) {
        let parts: Vec<&str> = line.splitn(3, " | ").collect();
        assert_eq!(parts.len(), 3, "{label}: {line}");
        assert_eq!(parts[0], *label, "{line}");
        assert_eq!(parts[1], "6", "{label}: not SIGABRT: {line}");
        assert!(parts[2].starts_with(&format!("fle: {api}: ")), "{label}: not a 'fle: {api}:' message: {line}");
        assert!(parts[2].contains(word), "{label}: message lacks {word:?}: {line}");
    }
}

const SCR: &str = "uint64_t sdc = K(fle_imp_GetDC, 0); int32_t q[4] = { 0, 0, 0, 0 };";

#[test]
fn the_attributes_of_a_screen_dc_start_at_the_display_defaults_and_belong_to_one_getdc_handle() {
    // Learn "Display Device Context Defaults": background mode OPAQUE (2), drawing mode R2_COPYPEN (13), mapping mode MM_TEXT (1), stretch mode BLACKONWHITE (1), current pen
    // position (0,0), brush origin (0,0), window and viewport origin (0,0); "When an application releases these device contexts, the current selections, such as mapping mode
    // and clipping region, are lost as the context is returned to the cache". The SetXxx calls return the previous value (SetMapMode page: "identifies the previous mapping mode").
    let l = gdi_lines(
        "fr7-screen-attr",
        r#"
    uint64_t d1 = K(fle_imp_GetDC, 0), d2, d3;
    int32_t p[2] = { 9, 9 };
    pr("d1 new", d1);
    { uint64_t m1 = K(fle_imp_SetMapMode, d1, 3), m2 = K(fle_imp_SetMapMode, d1, 3); printf("modes %llu %llu\n", U(m1), U(m2)); }
    { uint64_t b = K(fle_imp_SetBkMode, d1, 1), r = K(fle_imp_SetROP2, d1, 7), st = K(fle_imp_SetStretchBltMode, d1, 4), g = K(fle_imp_GetStretchBltMode, d1);
      printf("bk %llu rop2 %llu stretch %llu get %llu\n", U(b), U(r), U(st), U(g)); }
    { uint64_t mv = K(fle_imp_MoveToEx, d1, 5, 6, A(p)); printf("move %llu prev %d %d\n", U(mv), (int)p[0], (int)p[1]); }
    K(fle_imp_SetWindowOrgEx, d1, 7, 8, 0);
    K(fle_imp_SetViewportOrgEx, d1, 9, 10, 0);
    K(fle_imp_SetBrushOrgEx, d1, 3, 4, 0);
    pr("d1 set", d1);
    d2 = K(fle_imp_GetDC, 0);
    printf("d2 differs %d\n", d2 != d1);
    pr("d2 new", d2);
    { uint64_t rl = K(fle_imp_ReleaseDC, 0, d1); printf("release d1 %llu\n", U(rl)); }
    pr("d1 released", d1);
    d3 = K(fle_imp_GetDC, 0);
    pr("d3 new", d3);
    pr("d2 untouched", d2);
    /* the handle of a released DC is never reused (decision 0053), so the discard is seen by the number of states that the table holds, not by a second GetDC: the released DC's state is
     * gone (a new attribute call on the released handle stops, the DC is not live), and the state table has one state per live screen DC that an attribute call touched (d2, d3 = 2) */
    printf("states %u\n", fle_gdi_screen_states());
"#,
    );
    assert_eq!(
        l,
        [
            "d1 new 1 | 2 13 1 1 0 0 0 0 0 0 0 0 0",
            "modes 1 3",
            "bk 2 rop2 13 stretch 1 get 4",
            "move 1 prev 0 0",
            "d1 set 1 | 1 7 3 4 5 6 7 8 9 10 3 4 0",
            "d2 differs 1",
            "d2 new 1 | 2 13 1 1 0 0 0 0 0 0 0 0 0",
            "release d1 1",
            "d1 released 0 | 0 0 0 0 0 0 0 0 0 0 0 0 0",
            "d3 new 1 | 2 13 1 1 0 0 0 0 0 0 0 0 0",
            "d2 untouched 1 | 2 13 1 1 0 0 0 0 0 0 0 0 0",
            "states 2",
        ]
    );
}

#[test]
fn lptodp_of_a_screen_dc_in_mm_text_adds_the_origins() {
    // Mapping Modes and Translations: Dx = ((Lx - WOx) * VEx / WEx) + VOx with the extents 1 and 1 in MM_TEXT
    let l = gdi_lines(
        "fr7-screen-text",
        r#"
    uint64_t d = K(fle_imp_GetDC, 0);
    int32_t pt[4] = { 5, 6, -7, 8 };
    { uint64_t r = K(fle_imp_LPtoDP, d, A(pt), 2); printf("plain %llu %d %d %d %d\n", U(r), pt[0], pt[1], pt[2], pt[3]); }
    K(fle_imp_SetWindowOrgEx, d, 1, 2, 0);
    K(fle_imp_SetViewportOrgEx, d, 10, 20, 0);
    pt[0] = 5; pt[1] = 6;
    K(fle_imp_LPtoDP, d, A(pt), 1);
    printf("origins %d %d\n", pt[0], pt[1]);
    pt[0] = 134217727; pt[1] = -134217728; K(fle_imp_SetWindowOrgEx, d, 0, 0, 0); K(fle_imp_SetViewportOrgEx, d, 0, 0, 0);
    K(fle_imp_LPtoDP, d, A(pt), 1);
    printf("edge %d %d\n", pt[0], pt[1]);
"#,
    );
    assert_eq!(l, ["plain 1 5 6 -7 8", "origins 14 24", "edge 134217727 -134217728"]);
}

#[test]
fn lptodp_of_a_screen_dc_in_mm_himetric_and_mm_lometric_uses_the_physical_size_of_the_virtual_display() {
    // MM_HIMETRIC (3: [MS-WMF] MapMode Enumeration; the guest's edx at 0x6f642d) is 0.01 mm per unit, y up; MM_LOMETRIC (2) is 0.1 mm. The window extent is the physical size of the
    // virtual display in those units (508 x 286 mm, decision 0121), the viewport extent the pixels (1920 and -1080). Expected values by hand, in integers, rounded to the nearest
    // (a half away from zero: a choice without a source; no half occurs among these points):
    //   HIMETRIC x: 1920 / 50800 per unit: 1000 -> 37.80 -> 38; 14 -> 0.53 -> 1; 13 -> 0.49 -> 0;  y: -1080 / 28600 per unit: -500 -> +18.88 -> 19; 14 -> -0.53 -> -1; 13 -> -0.49 -> 0; 26 -> -0.98 -> -1
    //   LOMETRIC x: 1920 / 5080: 1000 -> 377.95 -> 378;  y: -1080 / 2860: 500 -> -188.81 -> -189
    let l = gdi_lines(
        "fr7-screen-metric",
        r#"
    uint64_t d = K(fle_imp_GetDC, 0);
    int32_t pt[10] = { 1000, 0, -1000, -500, 1, 14, 13, 13, 14, 26 };
    int i;
    K(fle_imp_SetMapMode, d, 3);
    { uint64_t r = K(fle_imp_LPtoDP, d, A(pt), 5); printf("himetric ret %llu\n", U(r)); }
    for (i = 0; i < 5; i++) printf("p%d %d %d\n", i, pt[2 * i], pt[2 * i + 1]);
    K(fle_imp_SetMapMode, d, 2);
    pt[0] = 1000; pt[1] = 0; pt[2] = 0; pt[3] = 500;
    K(fle_imp_LPtoDP, d, A(pt), 2);
    printf("lometric %d %d %d %d\n", pt[0], pt[1], pt[2], pt[3]);
    K(fle_imp_SetMapMode, d, 3);
    K(fle_imp_SetWindowOrgEx, d, 100, 0, 0);
    K(fle_imp_SetViewportOrgEx, d, 50, 7, 0);
    pt[0] = 1100; pt[1] = 10;
    K(fle_imp_LPtoDP, d, A(pt), 1);
    printf("with origins %d %d\n", pt[0], pt[1]);
    pr("state", d);
"#,
    );
    assert_eq!(
        l,
        [
            "himetric ret 1",
            "p0 38 0",
            "p1 -38 19",
            "p2 0 -1",
            "p3 0 0",
            "p4 1 -1",
            "lometric 378 0 0 -189",
            "with origins 88 7",
            "state 1 | 2 13 3 1 0 0 100 0 50 7 0 0 0",
        ]
    );
}

#[test]
fn a_screen_dc_still_stops_where_the_objects_the_colours_the_visible_area_and_the_pixels_are_needed() {
    // PM's decision 1: attributes only (setting and querying); nothing is drawn. Selecting an object, the two colours, the clip box and the pixel operations are not modelled for a screen DC.
    gdi_stops(
        "fr7-screen-stops",
        SCR,
        "SelectObject",
        // decision 0126 (wall 55): old: ("select-brush", SelectObject of a solid brush into a DC of GetDC(NULL)) stopped ("GetDC(NULL)"); the screen DC keeps its own selected objects now (the default objects, like a memory DC), so a brush is selected
        &[],
    );
    // a bitmap into a screen DC is refused with NULL and does not stop (decisions 0108, 0111; unchanged by this decision)
    let l = gdi_lines("fr7-screen-select-bitmap", &format!("    {SCR}\n    {{ uint64_t b = K(fle_imp_CreateBitmap, 2, 2, 1, 1); uint64_t r = K(fle_imp_SelectObject, sdc, b); printf(\"select bitmap %llu\\n\", U(r)); }}"));
    assert_eq!(l, ["select bitmap 0"]);
    // decision 0126 (wall 55): old: SetTextColor, SetBkColor, GetClipBox, PatBlt and FillRect at a DC of GetDC(NULL) each stopped ("GetDC(NULL)"). SetTextColor and SetBkColor answer now (the screen DC has a state of its own,
    // tests/wall_fr11.rs); PatBlt and FillRect draw on the surface of the desktop, which the window table makes: THIS unit link has no fle_win.c, so they stop by name "no window table is linked" (the full runtime answers them:
    // tests/wall_fr10.rs a_drawing_call_at_the_dc_of_the_entire_screen_keeps_its_old_stop returns 1). GetClipBox of a screen DC keeps stopping by name.
    let l = gdi_lines("fr7-screen-answers", &format!("    {SCR}\n    printf(\"%llu %llu\\n\", U(K(fle_imp_SetTextColor, sdc, 5) != 0xFFFFFFFFu), U(K(fle_imp_SetBkColor, sdc, 5) != 0xFFFFFFFFu));"));
    assert_eq!(l, ["1 1"]);
    gdi_stops("fr7-screen-stops-clip", SCR, "GetClipBox", &[("clipbox", "K(fle_imp_GetClipBox, sdc, A(q))", "GetDC(NULL)")]);
    gdi_stops("fr7-screen-stops-patblt", SCR, "PatBlt", &[("patblt", "K(fle_imp_PatBlt, sdc, 0, 0, 1, 1, 0x42)", "no window table is linked")]);
    gdi_stops("fr7-screen-stops-fillrect", SCR, "FillRect", &[("fillrect", "int32_t rc[4] = { 0, 0, 1, 1 }; uint64_t br = K(fle_imp_CreateSolidBrush, 1); K(fle_imp_FillRect, sdc, A(rc), br)", "no window table is linked")]);
}

#[test]
fn lptodp_of_a_screen_dc_stops_for_the_other_modes_the_overflow_and_the_open_arguments() {
    gdi_stops(
        "fr7-screen-lptodp-stops",
        SCR,
        "LPtoDP",
        &[
            ("loenglish", "K(fle_imp_SetMapMode, sdc, 4); K(fle_imp_LPtoDP, sdc, A(q), 1)", "a screen DC answers MM_LOMETRIC and MM_HIMETRIC too"),
            ("twips", "K(fle_imp_SetMapMode, sdc, 6); K(fle_imp_LPtoDP, sdc, A(q), 1)", "not MM_TEXT"),
            ("isotropic", "K(fle_imp_SetMapMode, sdc, 7); K(fle_imp_LPtoDP, sdc, A(q), 1)", "not MM_TEXT"),
            ("anisotropic", "K(fle_imp_SetMapMode, sdc, 8); K(fle_imp_LPtoDP, sdc, A(q), 1)", "not MM_TEXT"),
            ("null", "K(fle_imp_SetMapMode, sdc, 3); K(fle_imp_LPtoDP, sdc, 0, 1)", "lppt is NULL"),
            ("count-zero", "K(fle_imp_SetMapMode, sdc, 3); K(fle_imp_LPtoDP, sdc, A(q), 0)", "c is not positive"),
            // 27 bits plus sign: -2^27 .. 2^27 - 1 after the conversion (decision 0111): HIMETRIC makes x * 1920 / 50800; the largest 32-bit x stays inside (81M), so use MM_TEXT's offset to overflow
            ("overflow", "K(fle_imp_SetViewportOrgEx, sdc, 134217727, 0, 0); q[0] = 5; K(fle_imp_LPtoDP, sdc, A(q), 1)", "exceeds 27 bits"),
        ],
    );
}

#[test]
fn set_map_mode_of_a_screen_dc_stops_for_a_value_outside_the_eight_modes() {
    gdi_stops(
        "fr7-screen-setmapmode-stops",
        SCR,
        "SetMapMode",
        &[("zero", "K(fle_imp_SetMapMode, sdc, 0)", "not one of the 8 mapping modes"), ("nine", "K(fle_imp_SetMapMode, sdc, 9)", "not one of the 8 mapping modes")],
    );
}

#[test]
fn the_save_and_restore_of_a_screen_dc_keep_its_mapping_mode_and_origins() {
    let l = gdi_lines(
        "fr7-screen-save",
        r#"
    uint64_t d = K(fle_imp_GetDC, 0);
    uint64_t s1 = K(fle_imp_SaveDC, d);
    K(fle_imp_SetMapMode, d, 3);
    K(fle_imp_SetBkMode, d, 1);
    K(fle_imp_SetWindowOrgEx, d, 4, 5, 0);
    pr("changed", d);
    { uint64_t r = K(fle_imp_RestoreDC, d, s1); printf("restore %llu\n", U(r)); }
    pr("restored", d);
    { uint64_t r = K(fle_imp_RestoreDC, d, 1); printf("restore again %llu\n", U(r)); }
"#,
    );
    assert_eq!(l, ["changed 1 | 1 13 3 1 0 0 4 5 0 0 0 0 1", "restore 1", "restored 1 | 2 13 1 1 0 0 0 0 0 0 0 0 0", "restore again 0"]);
}

#[test]
fn the_screen_dc_state_is_not_visible_through_memory_dc_inspection_and_a_memory_dc_is_unchanged() {
    // a memory DC made from a screen DC keeps the defaults of a new DC whatever the screen DC's state is (CreateCompatibleDC page: "all attributes are set to normal default values")
    let l = gdi_lines(
        "fr7-screen-memdc",
        r#"
    uint64_t d = K(fle_imp_GetDC, 0), m;
    int32_t o[13];
    K(fle_imp_SetMapMode, d, 3);
    m = K(fle_imp_CreateCompatibleDC, d);
    fle_gdi_dc_attr(m, o);
    printf("memdc map %d bk %d\n", (int)o[2], (int)o[0]);
    printf("screen as memdc inspection %d\n", fle_gdi_dc_attr(d, o));
    printf("memdc as screen inspection %d\n", fle_gdi_screen_attr(m, o));
    { uint64_t r = K(fle_imp_SetMapMode, m, 3); printf("memdc set %llu\n", U(r)); }
"#,
    );
    assert_eq!(l, ["memdc map 1 bk 2", "screen as memdc inspection 0", "memdc as screen inspection 0", "memdc set 1"]);
}

#[test]
fn the_guest_call_of_wall_18_gives_the_pixel_size_of_a_point_in_hundredths_of_a_millimetre() {
    // f_6f6410: hdc = GetDC(NULL); SetMapMode(hdc, 3); pt.y = -pt.y; LPtoDP(hdc, &pt, 1); ReleaseDC(NULL, hdc); the result is pt. f_6f6570 calls it with y = 0.
    // MM_HIMETRIC: x pixels = x * 1920 / 50800. A point of 2540 hundredths of a millimetre (25.4 mm = one inch) is 96 pixels at 96 dpi; y = 0 stays 0 (-0 is 0).
    let l = gdi_lines(
        "fr7-guest-call",
        r#"
    int32_t pt[2];
    uint64_t hdc, r;
    int k;
    static const int32_t xs[] = { 2540, 5080, 0, 100, 1, 26 };
    for (k = 0; k < 6; k++) {
        hdc = K(fle_imp_GetDC, 0);
        K(fle_imp_SetMapMode, hdc, 3);
        pt[0] = xs[k]; pt[1] = 0; pt[1] = -pt[1];
        r = K(fle_imp_LPtoDP, hdc, A(pt), 1);
        K(fle_imp_ReleaseDC, 0, hdc);
        printf("x %d -> %d, %d (ret %llu) live %llu\n", (int)xs[k], (int)pt[0], (int)pt[1], U(r), U(fle_gdc_live_count()));
    }
"#,
    );
    assert_eq!(l, ["x 2540 -> 96, 0 (ret 1) live 0", "x 5080 -> 192, 0 (ret 1) live 0", "x 0 -> 0, 0 (ret 1) live 0", "x 100 -> 4, 0 (ret 1) live 0", "x 1 -> 0, 0 (ret 1) live 0", "x 26 -> 1, 0 (ret 1) live 0"]);
}

// ---------------------------------------------------------------------------------------------
// 2. The window driver: wall 19 (the class name) and wall 20 (the message-only window)
// ---------------------------------------------------------------------------------------------

const WIN_PRELUDE: &str = r##"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <pthread.h>
#include "fle_runtime.h"
#include "fle_dispatch.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_modules.h"
#include "fle_realmod.h"
#include "fle_callback.h"
#include "fle_class.h"
#include "fle_sys.h"
#include "fle_win.h"

uint64_t R[16];
static uint64_t stk[0x2000] __attribute__((aligned(4096)));
#define E0 ((uint64_t)(uintptr_t)stk + 0x8000 + 8)
#define RET0 UINT64_C(0xdead0000)
#define BAD UINT64_C(0xBADBADBAD)
#define U(x) ((unsigned long long)(x))
#define A(x) ((uint64_t)(uintptr_t)(x))
#define QFC_BASE UINT64_C(0x1d00000)

static uint32_t LE(void) { fle_imp_GetLastError(); return (uint32_t)R[0]; }
static void SETLE(uint32_t v) { fle_set_last_error(v); }

/* QuickFontCache is a real module at 0x1d00000 of the measured build: fle_class_hinst_known accepts its base (a strong definition for this driver, the table is the link's) */
int fle_realmod_find(const char *name) { (void)name; return -1; }
int fle_realmod_find_handle(uint64_t h) { return h == QFC_BASE ? 0 : -1; }
int fle_sys_region_at(uint64_t addr, fle_sys_region_view *out) { (void)addr; (void)out; return 0; }

static uint64_t S(const char *s)
{
    static uint16_t b[8][400];
    static unsigned k;
    uint16_t *p = b[k++ % 8];
    size_t i, n = strlen(s);
    for (i = 0; i < n; i++) p[i] = (uint8_t)s[i];
    p[n] = 0;
    return A(p);
}

#define VA_REC 0x2000
typedef struct { uint64_t h, msg, wp, lp; } ent;
static ent lg[64];
static unsigned ln;
static void cb_rec(void)
{
    ent *e = &lg[ln++];
    e->h = R[1]; e->msg = R[2]; e->wp = R[8]; e->lp = R[9];
    R[0] = e->msg == 0x81 ? 1 : 0;
}
const fle_fn_entry fle_dispatch_table[] = { { VA_REC, cb_rec } };
const uint64_t fle_dispatch_count = 1;

static uint64_t callv(void (*f)(void), uint64_t a, uint64_t b, uint64_t c, uint64_t d)
{
    uint64_t keep4 = R[4], r;
    memset(stk, 0, 0x100);
    stk[0] = RET0;
    R[1] = a; R[2] = b; R[8] = c; R[9] = d;
    R[4] = A(stk);
    R[0] = BAD;
    f();
    r = R[0];
    R[4] = keep4;
    return r;
}
#define CALL(f, ...) callv((void (*)(void))(f), __VA_ARGS__)

static uint8_t wcb[72];
static uint64_t REGW(uint64_t hinst, const char *name, uint64_t proc)
{
    uint32_t v = 0;
    uint64_t nm = S(name);
    memset(wcb, 0, sizeof wcb);
    memcpy(wcb + 0, &v, 4); memcpy(wcb + 8, &proc, 8);
    memcpy(wcb + 24, &hinst, 8); memcpy(wcb + 64, &nm, 8);
    return CALL(fle_imp_RegisterClassW, A(wcb), 0, 0, 0);
}

/* the guest's call at 0x1e85a18 as QuickFontCache makes it: CreateWindowExW(0, name, "", 0, 0, 0, 0, 0, HWND_MESSAGE, NULL, hInstance, NULL) */
static uint64_t cw_raw(uint64_t ex, uint64_t cls, uint64_t name, uint64_t style, uint64_t x, uint64_t y, uint64_t w, uint64_t h,
                       uint64_t parent, uint64_t menu, uint64_t hinst, uint64_t param)
{
    uint64_t save4 = R[4], call_sp = (R[4] - 0x80) & ~UINT64_C(15), in_sp = call_sp - 8, r;
    fle_st(call_sp + 0x20, 8, x); fle_st(call_sp + 0x28, 8, y); fle_st(call_sp + 0x30, 8, w); fle_st(call_sp + 0x38, 8, h);
    fle_st(call_sp + 0x40, 8, parent); fle_st(call_sp + 0x48, 8, menu); fle_st(call_sp + 0x50, 8, hinst); fle_st(call_sp + 0x58, 8, param);
    fle_st(in_sp, 8, RET0);
    R[1] = ex; R[2] = cls; R[8] = name; R[9] = style; R[4] = in_sp; R[0] = BAD;
    fle_imp_CreateWindowExW();
    r = R[0];
    if (R[4] != in_sp) printf("R4-NOT-RESTORED\n");
    R[4] = save4;
    return r;
}
static uint16_t empty[1] = { 0 };
#define CW_MSG(cls) cw_raw(0, S(cls), A(empty), 0, 0, 0, 0, 0, FLE_HWND_MESSAGE, 0, QFC_BASE, 0)
"##;

fn win_lines(name: &str, body: &str) -> Vec<String> {
    let o = run_with(win_objs(), name, WIN_PRELUDE, &format!("    R[4] = E0;\n{body}"));
    assert!(o.ok, "{name} did not exit normally (signal {:?}): {}\n{}", o.signal, o.stderr, o.stdout);
    o.stdout.lines().map(|s| s.to_string()).collect()
}

/// Each (label, statements) must stop the run with SIGABRT, a "fle: <api>: " message that contains `word`, and no output after it.
fn win_aborts(prefix: &str, setup: &str, api: &str, word: &str, cases: &[(&str, &str)]) {
    for (label, stmts) in cases {
        let body = format!("    R[4] = E0;\n{setup}\n    {stmts}\n    printf(\"returned %llu\\n\", U(R[0]));");
        let o = run_with(win_objs(), &format!("{prefix}-{label}"), WIN_PRELUDE, &body);
        assert!(!o.ok, "{label}: expected an abort, got stdout {}", o.stdout);
        assert!(!o.stdout.contains("returned"), "{label}: returned instead of stopping");
        assert!(o.stderr.contains(&format!("fle: {api}: ")), "{label}: no 'fle: {api}' message: {}", o.stderr);
        assert!(o.stderr.contains(word), "{label}: message lacks {word:?}: {}", o.stderr);
        assert_eq!(o.signal, Some(6), "{label}: not SIGABRT");
    }
}

#[test]
fn get_class_info_of_the_shared_anim_timer_class_is_not_found_before_the_guest_registers_it_and_found_after() {
    // wall 19: QuickFontCache's AcquireSharedAnimTimer asks GetClassInfoW(0x1d00000, "TSharedAnimTimerWindow") and registers the class itself when it is not found (0x1e85981..0x1e859c0)
    let l = win_lines(
        "fr7-class",
        r#"
    static uint8_t out[72];
    uint64_t r, atom;
    memset(out, 0xAA, sizeof out);
    SETLE(0x4321);
    r = CALL(fle_imp_GetClassInfoW, QFC_BASE, S("TSharedAnimTimerWindow"), A(out), 0);
    printf("first %llu untouched %d le %x count %llu\n", U(r), out[0] == 0xAA && out[71] == 0xAA, LE(), U(fle_class_count()));
    atom = REGW(QFC_BASE, "TSharedAnimTimerWindow", 0x1e858e0);
    printf("register %llx count %llu\n", U(atom), U(fle_class_count()));
    r = CALL(fle_imp_GetClassInfoW, QFC_BASE, S("TSharedAnimTimerWindow"), A(out), 0);
    { uint64_t proc; memcpy(&proc, out + 8, 8); printf("second %llu proc %llx\n", U(r), U(proc)); }
    /* the class is the one of hInstance 0x1d00000: the lifted DLL's hInstance does not see it */
    memset(out, 0xAA, sizeof out);
    r = CALL(fle_imp_GetClassInfoW, 0x400000, S("TSharedAnimTimerWindow"), A(out), 0);
    printf("other hinst %llu\n", U(r));
"#,
    );
    assert_eq!(l, ["first 0 untouched 1 le 4321 count 0", "register c000 count 1", "second 1 proc 1e858e0", "other hinst 0"]);
}

#[test]
fn get_class_info_of_a_system_class_stops_by_name_and_any_other_unregistered_name_is_not_found() {
    // decision 0215 (sweep1): a system class (EDIT ...) is registered by the system and stops (not modelled); every other unregistered name is "not found", whatever its shape or case (Learn GetClassInfoW: zero when no
    // class matches). Old (decision 0123): only T + an upper-case letter and the seven names of the guest were answered, the other names stopped.
    let setup = "static uint8_t out[72];";
    win_aborts(
        "fr7-class-stop",
        setup,
        "GetClassInfoW",
        "is a system class",
        &[("edit", "CALL(fle_imp_GetClassInfoW, QFC_BASE, S(\"EDIT\"), A(out), 0);")],
    );
    let l = win_lines(
        "fr7-class-delphi",
        r#"
    static uint8_t out[72];
    printf("%llu %llu %llu %llu %llu\n", U(CALL(fle_imp_GetClassInfoW, QFC_BASE, S("TSharedAnimTimerWindo"), A(out), 0)), U(CALL(fle_imp_GetClassInfoW, QFC_BASE, S("TSharedAnimTimerWindowX"), A(out), 0)),
           U(CALL(fle_imp_GetClassInfoW, QFC_BASE, S("XSharedAnimTimerWindo"), A(out), 0)), U(CALL(fle_imp_GetClassInfoW, QFC_BASE, S("XSharedAnimTimerWindowX"), A(out), 0)), U(CALL(fle_imp_GetClassInfoW, QFC_BASE, S("tsharedanimtimerwindow"), A(out), 0)));
"#,
    );
    assert_eq!(l, ["0 0 0 0 0"]);
    // a base that is not a module of the build still stops with the hInstance message
    win_aborts("fr7-class-hinst", setup, "GetClassInfoW", "hInstance is not the lifted DLL", &[("other-base", "CALL(fle_imp_GetClassInfoW, 0x1d10000, S(\"TSharedAnimTimerWindow\"), A(out), 0);")]);
}

#[test]
fn a_message_only_window_is_created_with_the_three_messages_and_is_marked_message_only() {
    // wall 20: CreateWindowExW(0, "TSharedAnimTimerWindow", "", 0, 0, 0, 0, 0, HWND_MESSAGE, NULL, 0x1d00000, NULL). Learn: "To create a message-only window, supply HWND_MESSAGE"; the three
    // messages as for every window (CreateWindowEx: "sends WM_NCCREATE, WM_NCCALCSIZE, and WM_CREATE"); a message-only window "is not visible".
    let l = win_lines(
        "fr7-msgonly",
        r#"
    uint64_t h;
    fle_win_view v;
    REGW(QFC_BASE, "TSharedAnimTimerWindow", VA_REC);
    SETLE(0x1234);
    h = CW_MSG("TSharedAnimTimerWindow");
    printf("h %llx count %llu msgs %u le %x\n", U(h), U(fle_win_count()), ln, LE());
    printf("m %llx %llx %llx\n", U(lg[0].msg), U(lg[1].msg), U(lg[2].msg));
    fle_win_get(h, &v);
    printf("style %x ex %x rect %d %d %d %d msgonly %d live %d hinst %llx text_len %u\n", (unsigned)v.style, (unsigned)v.exstyle, (int)v.x, (int)v.y, (int)v.cx, (int)v.cy, v.msgonly, v.live, U(v.hinst), (unsigned)v.text_len);
    { uint64_t cs = lg[0].lp; uint64_t par, hi; memcpy(&par, (void *)(uintptr_t)(cs + 24), 8); memcpy(&hi, (void *)(uintptr_t)(cs + 8), 8); printf("createstruct parent %llx hinstance %llx\n", U(par), U(hi)); }
    /* a second one gets the next handle, and an ordinary tool window of the table is not message-only */
    { uint64_t h2 = CW_MSG("TSharedAnimTimerWindow"); printf("h2 %llx\n", U(h2)); }
"#,
    );
    assert_eq!(
        l,
        [
            "h 7f990004 count 1 msgs 3 le 1234",
            "m 81 83 1",
            "style 0 ex 0 rect 0 0 0 0 msgonly 1 live 1 hinst 1d00000 text_len 0",
            "createstruct parent fffffffffffffffd hinstance 1d00000",
            "h2 7f990008",
        ]
    );
}

#[test]
fn a_message_only_window_gets_get_window_long_ptr_answers_but_not_the_parent() {
    let l = win_lines(
        "fr7-msgonly-long",
        r#"
    uint64_t h, r;
    REGW(QFC_BASE, "TSharedAnimTimerWindow", VA_REC);
    h = CW_MSG("TSharedAnimTimerWindow");
    r = CALL(fle_imp_GetWindowLongPtrW, h, 0xffffffecu, 0, 0); printf("exstyle %llx\n", U(r));
    r = CALL(fle_imp_GetWindowLongPtrW, h, 0xfffffff0u, 0, 0); printf("style %llx\n", U(r));
    r = CALL(fle_imp_GetWindowLongPtrW, h, 0xfffffffau, 0, 0); printf("hinstance %llx\n", U(r));
    r = CALL(fle_imp_SetWindowLongPtrW, h, 0xfffffffcu, 0x5555, 0); printf("wndproc was %llx\n", U(r));
    r = CALL(fle_imp_GetWindowLongPtrW, h, 0xfffffffcu, 0, 0); printf("wndproc now %llx\n", U(r));
"#,
    );
    assert_eq!(l, ["exstyle 0", "style 0", "hinstance 1d00000", "wndproc was 2000", "wndproc now 5555"]);
    win_aborts(
        "fr7-msgonly-parent",
        "uint64_t h; REGW(QFC_BASE, \"TSharedAnimTimerWindow\", VA_REC); h = CW_MSG(\"TSharedAnimTimerWindow\");",
        "GetWindowLongPtrW",
        "message-only window",
        &[("hwndparent", "CALL(fle_imp_GetWindowLongPtrW, h, 0xfffffff8u, 0, 0);")],
    );
}

#[test]
fn the_message_only_form_stops_for_any_other_style_name_parent_or_argument() {
    let setup = "REGW(QFC_BASE, \"TSharedAnimTimerWindow\", VA_REC);";
    let msg = |over: &[(usize, &str)]| -> String {
        let mut a: Vec<String> = ["0", "S(\"TSharedAnimTimerWindow\")", "A(empty)", "0", "0", "0", "0", "0", "FLE_HWND_MESSAGE", "0", "QFC_BASE", "0"].iter().map(|s| s.to_string()).collect();
        for (i, v) in over {
            a[*i] = v.to_string();
        }
        format!("cw_raw({});", a.join(", "))
    };
    let cases: Vec<(&str, String, &str)> = vec![
        ("tool-extended-style", msg(&[(0, "0x80")]), "a message-only window (hWndParent HWND_MESSAGE) with dwExStyle"),
        ("style-popup", msg(&[(3, "0x80000000u")]), "a message-only window (hWndParent HWND_MESSAGE) with dwExStyle"),
        ("style-visible", msg(&[(3, "0x10000000")]), "a message-only window (hWndParent HWND_MESSAGE) with dwExStyle"),
        // decision 0123 (old: ("x"), ("y"), ("width"), ("height") "position/size", ("param") "lpParam is not NULL" and ("name") "lpWindowName is not empty" stopped): the position and size are kept as given, lpParam goes
        // to CREATESTRUCT.lpCreateParams and the name is kept (answered, next assertion); a menu, another hInstance and a class that is not registered still stop
        ("menu", msg(&[(9, "1")]), "hMenu is not NULL"),
        ("hinst", msg(&[(10, "0")]), "hInstance is not the lifted DLL"),
        ("system-class-name", msg(&[(1, "S(\"EDIT\")")]), "is a system class"),
    ];
    for (label, stmt, word) in &cases {
        win_aborts("fr7-msgonly-stop", setup, "CreateWindowExW", word, &[(label, stmt.as_str())]);
    }
    let l = win_lines(
        "fr7-msgonly-answered",
        &format!(
            r#"
    {setup}
    fle_win_view v;
    uint64_t h;
    h = {x}; fle_win_get(h, &v); printf("x %d %d %d %d\n", v.x, v.y, v.cx, v.cy);
    h = {y}; fle_win_get(h, &v); printf("y %d %d %d %d\n", v.x, v.y, v.cx, v.cy);
    h = {w}; fle_win_get(h, &v); printf("w %d %d %d %d\n", v.x, v.y, v.cx, v.cy);
    h = {hh}; fle_win_get(h, &v); printf("h %d %d %d %d\n", v.x, v.y, v.cx, v.cy);
    h = {name}; fle_win_get(h, &v); printf("name len %u msgonly %d\n", (unsigned)v.text_len, v.msgonly);
    h = {param}; printf("param live %d\n", h != 0);
"#,
            x = msg(&[(4, "1")]).trim_end_matches(';'),
            y = msg(&[(5, "2")]).trim_end_matches(';'),
            w = msg(&[(6, "3")]).trim_end_matches(';'),
            hh = msg(&[(7, "4")]).trim_end_matches(';'),
            name = msg(&[(2, "S(\"x\")")]).trim_end_matches(';'),
            param = msg(&[(11, "1")]).trim_end_matches(';'),
        ),
    );
    assert_eq!(l, ["x 1 0 0 0", "y 0 2 0 0", "w 0 0 3 0", "h 0 0 0 4", "name len 1 msgonly 1", "param live 1"]);
    // the near misses of the parent: HWND_MESSAGE is exactly (HWND)-3; the neighbours are not live windows of the table, so the creation fails with NULL and 1400 (decision 0123, old: they stopped with "hWndParent is not NULL and not HWND_MESSAGE")
    let l = win_lines(
        "fr7-msgonly-parent-neighbours",
        &format!(
            r#"
    {setup}
    uint64_t r;
    r = cw_raw(0x80, S("TSharedAnimTimerWindow"), A(empty), 0x80000000u, 0, 0, 0, 0, 0xFFFFFFFFFFFFFFFEull, 0, QFC_BASE, 0); printf("minus-2 %llu le %u\n", U(r), LE());
    r = cw_raw(0x80, S("TSharedAnimTimerWindow"), A(empty), 0x80000000u, 0, 0, 0, 0, 0xFFFFFFFFFFFFFFFCull, 0, QFC_BASE, 0); printf("minus-4 %llu le %u\n", U(r), LE());
    r = cw_raw(0x80, S("TSharedAnimTimerWindow"), A(empty), 0x80000000u, 0, 0, 0, 0, 0xFFFFFFFDull, 0, QFC_BASE, 0); printf("low-half %llu le %u\n", U(r), LE());
    r = cw_raw(0x80, S("TSharedAnimTimerWindow"), A(empty), 0x80000000u, 0, 0, 0, 0, 1, 0, QFC_BASE, 0); printf("one %llu le %u\n", U(r), LE());
"#
        ),
    );
    assert_eq!(l, ["minus-2 0 le 1400", "minus-4 0 le 1400", "low-half 0 le 1400", "one 0 le 1400"]);
}

#[test]
fn a_message_only_window_of_an_unregistered_guest_class_name_fails_with_null_and_leaves_the_table_alone() {
    // as for the first form (decision 0071): a class of the guest's own names that is not registered yet makes CreateWindowExW fail (NULL), the table untouched
    let l = win_lines(
        "fr7-msgonly-unregistered",
        r#"
    uint64_t h;
    SETLE(0x7777);
    h = CW_MSG("TSharedAnimTimerWindow");
    printf("h %llx count %llu le %x\n", U(h), U(fle_win_count()), LE());
"#,
    );
    assert_eq!(l, ["h 0 count 0 le 7777"]);
}

// ---------------------------------------------------------------------------------------------
// 3. The bytes of the real DLLs that the decision rests on
// ---------------------------------------------------------------------------------------------

fn dll_bytes(var: &str, default: &str) -> Option<Vec<u8>> {
    let explicit = std::env::var_os(var);
    let path = match &explicit {
        Some(p) => PathBuf::from(p),
        None => manifest().join(default),
    };
    match std::fs::read(&path) {
        Ok(b) => Some(b),
        Err(e) if explicit.is_some() => panic!("{var}={}: {e}", path.display()),
        Err(e) => {
            eprintln!("SKIPPED: {}: {e}", path.display());
            None
        }
    }
}
fn le32(d: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(d[o..o + 4].try_into().unwrap())
}
fn le16(d: &[u8], o: usize) -> u16 {
    u16::from_le_bytes(d[o..o + 2].try_into().unwrap())
}
/// The bytes at an RVA of a PE file on disk (headers and sections by their raw data).
fn hex_rva(dll: &[u8], rva: u32, n: usize) -> String {
    let lf = le32(dll, 0x3c) as usize;
    let nsec = le16(dll, lf + 6) as usize;
    let osz = le16(dll, lf + 20) as usize;
    for i in 0..nsec {
        let o = lf + 24 + osz + 40 * i;
        let (vs, sva, rs, rp) = (le32(dll, o + 8), le32(dll, o + 12), le32(dll, o + 16), le32(dll, o + 20));
        if rva >= sva && rva < sva + vs.max(rs) {
            let p = rp as usize + (rva - sva) as usize;
            return dll[p..p + n].iter().map(|x| format!("{x:02x}")).collect();
        }
    }
    panic!("rva {rva:#x} is in no section");
}

#[test]
fn the_guests_screen_dc_call_of_wall_18_asks_mm_himetric_and_the_point_is_converted_in_place() {
    let Some(dll) = dll_bytes("FLENGINE_DLL", "../../../binaries/win64/FLEngine_x64.dll") else { return };
    let h = |va: u64, n: usize| hex_rva(&dll, (va - 0x400000) as u32, n);
    // f_6f6410 at 0x6f6410: GetDC(NULL) (xor ecx,ecx; call), SetMapMode(hdc, 3) (mov edx,3), pt.y = -pt.y (neg dword ptr [rbp+0x24]), LPtoDP(hdc, &pt, 1) (mov r8d,1), ReleaseDC(NULL, hdc)
    assert_eq!(h(0x6f6420, 2), "33c9");
    assert_eq!(h(0x6f642d, 5), "ba03000000");
    assert_eq!(h(0x6f643f, 4), "f75d2448");
    assert_eq!(h(0x6f6449, 6), "41b801000000");
    assert_eq!(h(0x6f6454, 2), "33c9");
    // the number 3 is MM_HIMETRIC in the numbering of [MS-WMF] / [MS-EMF] "MapMode Enumeration" (MM_TEXT 1, MM_LOMETRIC 2, MM_HIMETRIC 3): written here, not read from the C header
    let map_modes = [("MM_TEXT", 1u32), ("MM_LOMETRIC", 2), ("MM_HIMETRIC", 3), ("MM_LOENGLISH", 4), ("MM_HIENGLISH", 5), ("MM_TWIPS", 6), ("MM_ISOTROPIC", 7), ("MM_ANISOTROPIC", 8)];
    assert_eq!(map_modes.iter().find(|(_, v)| *v == 3).unwrap().0, "MM_HIMETRIC");
    let hdr = std::fs::read_to_string(src_dir().join("fle_gdi.h")).unwrap();
    for (n, v) in [("FLE_GDI_MM_TEXT", 1), ("FLE_GDI_MM_LOMETRIC", 2), ("FLE_GDI_MM_HIMETRIC", 3), ("FLE_GDI_MM_ANISOTROPIC", 8)] {
        let line = hdr.lines().find(|l| l.starts_with(&format!("#define {n} "))).unwrap_or_else(|| panic!("{n}"));
        let num: u32 = line[format!("#define {n} ").len()..].trim_start().split(|c: char| !c.is_ascii_digit()).next().unwrap().parse().unwrap();
        assert_eq!(num, v, "{n}");
    }
}

#[test]
fn the_guests_calls_of_walls_19_and_20_in_quickfontcache_have_the_arguments_the_decision_lists() {
    let Some(dll) = dll_bytes("QFC_PE", "../../out-qfc/quickfontcache_x64.dll") else { return };
    let h = |rva: u32, n: usize| hex_rva(&dll, rva, n);
    // image base 0x400000 for the original file; the module is rebased to 0x1d00000 in the measured build: the RVAs are the same
    // 0x1e85981: lea r8,[rsp+0x68]; call GetClassInfoW (the wrapper at 0x1d1e010); the class name is the UTF-16 string at 0x1e85922 = "TSharedAnimTimerWindow" (RVA 0x185922)
    assert_eq!(h(0x185981, 4), "4c8d4424");
    let name: String = "TSharedAnimTimerWindow".encode_utf16().flat_map(|c| c.to_le_bytes()).chain([0, 0]).map(|b| format!("{b:02x}")).collect();
    assert_eq!(h(0x185922, 46), name);
    // the call at 0x1e85a18 is the wrapper of CreateWindowExW: ecx (dwExStyle) = 0 (xor r8/r9 and the stack zeroes), dwStyle 0, X Y W H 0, hWndParent = -3 (mov qword ptr [rsp+0x38], 0xfffffffffffffffd),
    // hMenu 0, lpWindowName = L"" (RVA 0x185a2c: two zero bytes), lpParam 0
    assert_eq!(h(0x1859d3, 6), "4d33c04d33c9");
    assert_eq!(h(0x1859d9, 8), "c744242000000000");
    assert_eq!(h(0x1859f1, 9), "48c7442438fdffffff");
    assert_eq!(h(0x185a2c, 2), "0000");
    assert_eq!(h(0x185a18, 1), "e8");
    // the wrapper 0x1d1e130 passes its arguments to the thunk at 0x1d1e120 (a `jmp [IAT]`), with eax = 0 as the first argument (xor eax,eax; ... mov ecx,eax)
    assert_eq!(h(0x1e120, 2), "ff25");
    assert_eq!(h(0x1e130, 5), "534883ec60");
    assert_eq!(h(0x1e135, 2), "33c0");
    // the next wall (21): CreateThread with CREATE_SUSPENDED (flags 4, and 0x10004 with STACK_SIZE_PARAM_IS_A_RESERVATION): the start routine is the Delphi thread procedure at 0x1df0ee0
    assert_eq!(h(0x6d80, 2), "ff25");
    assert_eq!(h(0xf11cf, 8), "c744242004000000");
    assert_eq!(h(0xf119c, 8), "c744242004000100");
}

#[test]
fn the_sources_of_decision_0121_read_nothing_of_the_host_and_make_no_message_queue_apart_from_decision_0122() {
    let strip = |s: &str| -> String {
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
    };
    for f in ["fle_gdc.c", "fle_win.c", "fle_class.c"] {
        let t = strip(&std::fs::read_to_string(src_dir().join(f)).unwrap());
        for bad in ["XOpenDisplay", "getenv(", "gettimeofday", "clock_gettime", "fopen(", "open(", "GetDpi", "sysconf(", "XGetWindowAttributes"] {
            assert!(!t.contains(bad), "{f} must not use {bad}");
        }
    }
    // wall 18 and 20 add no message queue (decision 0121). Decision 0122 adds the queue in fle_win.c (tests/wall_fr8.rs), so the names PostMessageW, GetMessageW, PeekMessageW, WaitMessage,
    // DispatchMessageW, TranslateMessage and PostQuitMessage are allowed there now (old: none of them); MsgWaitForMultipleObjects stays in fle_thread.c.
    let win = strip(&std::fs::read_to_string(src_dir().join("fle_win.c")).unwrap());
    for w in ["MsgWaitForMultipleObjects"] {
        assert!(!win.contains(w), "{w}: belongs to fle_thread.c (decision 0122)");
    }
}
