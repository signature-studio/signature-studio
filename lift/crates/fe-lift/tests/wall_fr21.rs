//! Tests of decision 0148 (docs/decisions/0148-render-defects.md, PROVISIONAL): why parts of the engine's windows were not drawn, and what the runtime changed for it.
//!
//! 1. gdi32!GetClipBox of a memory DC answers the tightest rectangle around the DC's clipping region (cut by the bitmap), in logical units, with the type NULLREGION / SIMPLEREGION / COMPLEXREGION. Before decision
//!    0148 it answered the whole bitmap whatever the region was. Expected values are written here from the Learn page of GetClipBox ("the tightest bounding rectangle that can be drawn around the current visible
//!    area ... defined by the current clipping region", "in logical units", NULLREGION "Region is empty", SIMPLEREGION "Region is a single rectangle", COMPLEXREGION "Region is more than one rectangle") and from
//!    the rectangle arithmetic in the comment of each case; the choices without a source are the S-clipbox-... names of the decision. A MEMORY DC only: the screen / window DC still stops by name (wall_fr7.rs).
//! 2. The pattern of the engine that made the defect visible: each control of a window narrows the clip (IntersectClipRect), asks GetClipBox for the rectangle to draw into and draws there by itself (its own pixel
//!    code, not a GDI call). With a clip box that is the whole bitmap, a later control painted over the earlier ones. Here a stand-in does the same on a 32-bit DIB and the pixels are compared.
//! 3. The diagnostics added to the runtime for this study (opt-in, they read and change nothing the guest sees): FLE_RASTER_INIT_FILL (the first value of a window surface), FLE_RASTER_SRCDUMP (the source
//!    rectangle of a BitBlt to a window).
//! 4. The inputs: the VFS seed list vfs-seed-vis1.tsv (the list of decision 0144 + the operating-system fonts the guest asks for at C:\Windows\Fonts\), and the generated list of the API probe of the harness.
//!
//! EVERY ANSWER IS PROVISIONAL. Nothing is compared with a real Windows.

mod etxt_common;

use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn base_dir() -> PathBuf {
    manifest().join("../..")
}
/// The runtime directory. FLE_FR21_RT (a copy of lift/runtime) overrides it: the mutation runs of the run record change one place of such a copy and expect a test to fail.
fn rt_dir() -> PathBuf {
    if let Ok(d) = std::env::var("FLE_FR21_RT") {
        return PathBuf::from(d);
    }
    base_dir().join("runtime")
}
fn work_dir() -> PathBuf {
    let d = base_dir().join("out-wallFR21/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

fn clang_c(src: &Path, obj: &Path) {
    let cc = Command::new("clang")
        .args(["-O0", "-std=gnu11", "-Wall", "-Wextra", "-Werror", "-fPIE", "-c"])
        .arg("-I")
        .arg(rt_dir())
        .arg(src)
        .arg("-o")
        .arg(obj)
        .env("TMPDIR", work_dir().join("tmp"))
        .output()
        .expect("clang not found");
    assert!(cc.status.success(), "clang failed for {}: {}", src.display(), String::from_utf8_lossy(&cc.stderr));
}

fn build(tag: &str, files: &[&str]) -> Vec<PathBuf> {
    let dir = work_dir().join(etxt_common::per_process(tag));
    std::fs::create_dir_all(&dir).unwrap();
    files
        .iter()
        .map(|f| {
            let o = dir.join(f.replace(".c", ".o"));
            clang_c(&rt_dir().join(f), &o);
            o
        })
        .collect()
}

/// fle_gdi.c and the files it links with, WITHOUT fle_visrgn.c (its hooks are weak: the link is the one of every test that does not know decision 0142).
fn gdi_objs() -> &'static Vec<PathBuf> {
    static O: OnceLock<Vec<PathBuf>> = OnceLock::new();
    O.get_or_init(|| build("obj-fr21-gdi", &["fle_gdi.c", "fle_raster.c", "fle_gdc.c", "fle_imports.c", "fle_env.c", "fle_cursor.c"]))
}
/// fle_raster.c (the window surfaces) and fle_visrgn.c (the dump) with the clock of fle_sys.c.
fn vis_objs() -> &'static Vec<PathBuf> {
    static O: OnceLock<Vec<PathBuf>> = OnceLock::new();
    O.get_or_init(|| build("obj-fr21-vis", &["fle_raster.c", "fle_visrgn.c", "fle_sys.c", "fle_imports.c", "fle_env.c", "fle_time.c", "fle_x87ctl.c", "fle_mxcsr.c", "fle_x87.c", "fle_locale.c"]))
}

struct Out {
    stdout: String,
    stderr: String,
    signal: Option<i32>,
    ok: bool,
}

const TOP_GDI: &str = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <unistd.h>
#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_gdc.h"
#include "fle_gdi.h"
#include "fle_cursor.h"

uint64_t R[16];
static uint64_t stk[32];

static uint64_t cN(void (*f)(void), const uint64_t *a)
{
    memset(stk, 0, sizeof stk);
    stk[0] = 0xdead0000u;
    for (int i = 4; i < 13; i++) stk[i + 1] = a[i];
    R[1] = a[0]; R[2] = a[1]; R[8] = a[2]; R[9] = a[3];
    R[4] = (uint64_t)(uintptr_t)stk;
    R[0] = 0xBADBADBADull;
    f();
    return R[0];
}
#define K(f, ...) cN((void (*)(void))(f), (uint64_t[13]){ __VA_ARGS__ })
#define U(x) ((unsigned long long)(x))
#define A(p) ((uint64_t)(uintptr_t)(p))
#define NEG(n) ((uint64_t)(int64_t)(n))
#define SCR() K(fle_imp_GetDC, 0)
#define MKDC() K(fle_imp_CreateCompatibleDC, 0)
#define SELO(dc, h) K(fle_imp_SelectObject, (dc), (h))

static uint8_t bmi[40 + 12];
static uint64_t bits_out;
static void p32(unsigned o, uint32_t v) { memcpy(bmi + o, &v, 4); }
/* a top-down 32-bit DIB section (a negative height) */
static uint64_t MKDIB(uint64_t dc, int32_t w, int32_t h)
{
    uint16_t one = 1, b = 32;
    memset(bmi, 0, sizeof bmi);
    p32(0, 40); p32(4, (uint32_t)w); p32(8, (uint32_t)(-h)); memcpy(bmi + 12, &one, 2); memcpy(bmi + 14, &b, 2); p32(16, 0);
    bits_out = 0;
    return K(fle_imp_CreateDIBSection, dc, A(bmi), 0, A(&bits_out), 0, 0);
}

static int32_t box[4];
/* GetClipBox and print: the label, the type, the rectangle */
static void CB(const char *label, uint64_t dc)
{
    box[0] = box[1] = box[2] = box[3] = 77;
    uint64_t t = K(fle_imp_GetClipBox, dc, A(box));
    printf("%s %llu %d %d %d %d\n", label, U(t), (int)box[0], (int)box[1], (int)box[2], (int)box[3]);
}
#define CLIP(dc, l, t, r, b) K(fle_imp_IntersectClipRect, (dc), NEG(l), NEG(t), NEG(r), NEG(b))
#define RGN(l, t, r, b) K(fle_imp_CreateRectRgn, NEG(l), NEG(t), NEG(r), NEG(b))
"#;

/// The C of the programs that use the window surfaces and the dump (no GDI call): the includes and the register file the runtime files expect.
const TOP_VIS: &str = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include "fle_runtime.h"
#include "fle_raster.h"
#include "fle_visrgn.h"

uint64_t R[16];
"#;

fn run_src(name: &str, top: &str, objs: &[PathBuf], body: &str, env: &[(&str, &str)]) -> Out {
    let dir = work_dir();
    let uname = etxt_common::uniq(name);
    let c = dir.join(format!("{uname}.c"));
    let exe = dir.join(&uname);
    let src = format!("{top}\nint main(void)\n{{\n{body}\n    return 0;\n}}\n");
    std::fs::write(&c, src).unwrap();
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fPIE", "-pthread", "-I"]).arg(rt_dir());
    for o in objs {
        cmd.arg(o);
    }
    let cc = cmd.arg("-o").arg(&exe).arg(&c).arg("-lm").env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    let mut run = Command::new(&exe);
    run.env("TMPDIR", dir.join("tmp"));
    for (k, v) in env {
        run.env(k, v);
    }
    let r = run.output().expect("run failed");
    Out { stdout: String::from_utf8_lossy(&r.stdout).to_string(), stderr: String::from_utf8_lossy(&r.stderr).to_string(), signal: r.status.signal(), ok: r.status.success() }
}

fn lines(name: &str, objs: &[PathBuf], body: &str, env: &[(&str, &str)]) -> Vec<String> {
    let top = if objs == gdi_objs().as_slice() { TOP_GDI } else { TOP_VIS };
    let o = run_src(name, top, objs, body, env);
    assert!(o.ok, "{name} did not exit normally (signal {:?}): {}\n{}", o.signal, o.stderr, o.stdout);
    o.stdout.lines().map(|s| s.to_string()).collect()
}

// ---- 1. GetClipBox ----

const BOX_SETUP: &str = "    uint64_t sdc = SCR(), dc = MKDC();\n    uint64_t d = MKDIB(sdc, 30, 20);\n    SELO(dc, d);\n";

#[test]
fn get_clip_box_is_the_bounding_rectangle_of_the_clip_region_cut_by_the_bitmap() {
    // a 30 x 20 bitmap. Every rectangle below is half-open, in device = logical units (no origin yet). Types: NULLREGION 1, SIMPLEREGION 2, COMPLEXREGION 3.
    let l = lines(
        "fr21-box",
        gdi_objs(),
        &format!(
            r#"{BOX_SETUP}
    CB("none", dc);                                  /* no region: the bitmap */
    CLIP(dc, 5, 4, 15, 12);  CB("one", dc);          /* default region (the bitmap) and (5,4)-(15,12): that rectangle */
    CLIP(dc, 10, 0, 40, 8);  CB("narrowed", dc);     /* (5,4)-(15,12) and (10,0)-(40,8): x 10..15, y 4..8 */
    K(fle_imp_SelectClipRgn, dc, 0);  CB("removed", dc);   /* a NULL region removes the application-defined region: the bitmap again */
    /* a region of two rectangles: (2,2)-(8,8) and (20,10)-(26,18): the box is (2,2)-(26,18), COMPLEXREGION */
    uint64_t a = RGN(2, 2, 8, 8), b = RGN(20, 10, 26, 18), u = RGN(0, 0, 1, 1);
    K(fle_imp_CombineRgn, u, a, b, 2);
    K(fle_imp_SelectClipRgn, dc, u);  CB("two", dc);
    /* a region larger than the bitmap is cut by it: (-10,-10)-(100,100) -> (0,0)-(30,20) */
    uint64_t big = RGN(-10, -10, 100, 100);
    K(fle_imp_SelectClipRgn, dc, big);  CB("big", dc);
    /* a region that sticks out on one side: (-5,3)-(10,40) -> (0,3)-(10,20) */
    uint64_t side = RGN(-5, 3, 10, 40);
    K(fle_imp_SelectClipRgn, dc, side);  CB("side", dc);
    /* two pieces, one of them entirely outside the bitmap: (2,2)-(8,8) and (50,50)-(60,60): one piece is left, SIMPLEREGION */
    uint64_t far_ = RGN(50, 50, 60, 60), v = RGN(0, 0, 1, 1);
    K(fle_imp_CombineRgn, v, a, far_, 2);
    K(fle_imp_SelectClipRgn, dc, v);  CB("outside", dc);
    /* a region entirely outside the bitmap: nothing is left, NULLREGION (the rectangle is all zero: S-clipbox-null-rect) */
    uint64_t gone = RGN(40, 40, 50, 50);
    K(fle_imp_SelectClipRgn, dc, gone);  CB("gone", dc);
    K(fle_imp_SelectClipRgn, dc, 0);  CB("back", dc);
"#
        ),
        &[],
    );
    assert_eq!(
        l,
        [
            "none 2 0 0 30 20",
            "one 2 5 4 15 12",
            "narrowed 2 10 4 15 8",
            "removed 2 0 0 30 20",
            "two 3 2 2 26 18",
            "big 2 0 0 30 20",
            "side 2 0 3 10 20",
            "outside 2 2 2 8 8",
            "gone 1 0 0 0 0",
            "back 2 0 0 30 20",
        ]
    );
}

#[test]
fn get_clip_box_is_in_logical_units_and_an_empty_box_is_not_moved_by_the_origins() {
    // Lx = Dx + WOx - VOx (MM_TEXT). The clip rectangle of IntersectClipRect is logical too: the same call under another window origin is another device rectangle.
    let l = lines(
        "fr21-box-org",
        gdi_objs(),
        &format!(
            r#"{BOX_SETUP}
    CLIP(dc, 5, 4, 15, 12);                          /* device (5,4)-(15,12) under the origins (0,0) */
    K(fle_imp_SetWindowOrgEx, dc, 3, 2, 0);  CB("wo", dc);       /* L = D + (3,2): (8,6)-(18,14) */
    K(fle_imp_SetViewportOrgEx, dc, 1, 1, 0);  CB("wovo", dc);   /* L = D + (3,2) - (1,1) = D + (2,1): (7,5)-(17,13) */
    /* the pattern of the engine: window origin = minus the position of the control, the control's own rectangle from (0,0) */
    K(fle_imp_SelectClipRgn, dc, 0);
    K(fle_imp_SetViewportOrgEx, dc, 0, 0, 0);
    K(fle_imp_SetWindowOrgEx, dc, NEG(-12), NEG(-3), 0);      /* WO = (-12,-3): L = D - (12,3) */
    CLIP(dc, 0, 0, 8, 6);                            /* logical (0,0)-(8,6) = device (12,3)-(20,9) */
    CB("ctl", dc);                                   /* the box in logical units again: (0,0)-(8,6) */
    /* an empty clip: the box is all zero whatever the origins are (not moved by them) */
    CLIP(dc, 100, 100, 110, 110);
    CB("empty", dc);
"#
        ),
        &[],
    );
    assert_eq!(l, ["wo 2 8 6 18 14", "wovo 2 7 5 17 13", "ctl 2 0 0 8 6", "empty 1 0 0 0 0"]);
}

#[test]
fn the_clip_box_follows_save_and_restore() {
    // the region is part of the state that SaveDC / RestoreDC keep (Learn SaveDC: "clipping region")
    let l = lines(
        "fr21-box-save",
        gdi_objs(),
        &format!(
            r#"{BOX_SETUP}
    CLIP(dc, 5, 4, 15, 12);  CB("a", dc);
    uint64_t s = K(fle_imp_SaveDC, dc);
    CLIP(dc, 6, 5, 9, 7);  CB("b", dc);
    K(fle_imp_RestoreDC, dc, NEG(-1));  CB("c", dc);
    (void)s;
"#
        ),
        &[],
    );
    assert_eq!(l, ["a 2 5 4 15 12", "b 2 6 5 9 7", "c 2 5 4 15 12"]);
}

#[test]
fn get_clip_box_keeps_its_old_stops() {
    // a NULL rectangle, another mapping mode and a screen DC stop by name (unchanged by decision 0148)
    for (label, stmt, word) in [
        ("null", "K(fle_imp_GetClipBox, dc, 0);", "lprect is NULL"),
        ("mode", "K(fle_imp_SetMapMode, dc, 7); K(fle_imp_GetClipBox, dc, A(box));", "not MM_TEXT"),
        ("screen", "K(fle_imp_GetClipBox, sdc, A(box));", "GetDC(NULL)"),
    ] {
        let o = run_src(&format!("fr21-stop-{label}"), TOP_GDI, gdi_objs(), &format!("{BOX_SETUP}\n    {stmt}\n"), &[]);
        assert_eq!(o.signal, Some(6), "{label}: must abort: {}{}", o.stdout, o.stderr);
        assert!(o.stderr.contains("fle: GetClipBox: ") && o.stderr.contains(word), "{label}: {}", o.stderr);
    }
}

// ---- 2. the pattern of the engine ----

#[test]
fn a_later_control_that_draws_into_its_clip_box_does_not_paint_over_the_earlier_ones() {
    // four controls side by side, 8 x 8 each, on a 32 x 8 top-down DIB. Each does what the engine's controls do: set the window origin to minus its own position, narrow the clip to its own rectangle (0,0)-(8,8),
    // ask GetClipBox which rectangle to draw into and write its colour (its own pixel code: no GDI call) into the DIB over exactly that rectangle, mapped back to the device (D = L - WO).
    // Expected: columns 0-7 hold control 1's colour, 8-15 control 2's, ... The defect of decision 0148 (the box was the whole bitmap) left the LAST control's colour everywhere.
    let l = lines(
        "fr21-controls",
        gdi_objs(),
        r#"
    uint64_t sdc = SCR(), dc = MKDC();
    uint64_t d = MKDIB(sdc, 32, 8);
    SELO(dc, d);
    uint32_t *px = (uint32_t *)(uintptr_t)bits_out;
    memset(px, 0, 32 * 8 * 4);
    for (int i = 0; i < 4; i++) {
        K(fle_imp_SelectClipRgn, dc, 0);
        K(fle_imp_SetWindowOrgEx, dc, NEG(-8 * i), 0, 0);   /* the control sits at x = 8 * i */
        CLIP(dc, 0, 0, 8, 8);
        int32_t r[4];
        K(fle_imp_GetClipBox, dc, A(r));
        for (int y = r[1]; y < r[3]; y++)
            for (int x = r[0]; x < r[2]; x++) {
                int dx = x + 8 * i, dy = y;                 /* device = logical - WO, with WO = -8 * i */
                if (dx >= 0 && dx < 32 && dy >= 0 && dy < 8) px[dy * 32 + dx] = 0x100u + (uint32_t)i;
            }
    }
    for (int y = 0; y < 8; y += 7) {
        printf("row %d:", y);
        for (int x = 0; x < 32; x += 4) printf(" %x", (unsigned)(px[y * 32 + x] & 0xff) | (px[y * 32 + x] ? 0x10u : 0));
        printf("\n");
    }
    unsigned n[5] = {0};
    for (int k = 0; k < 32 * 8; k++) n[px[k] ? (px[k] & 0xff) + 1 : 0]++;
    printf("counts none %u c1 %u c2 %u c3 %u c4 %u\n", n[0], n[1], n[2], n[3], n[4]);
"#,
        &[],
    );
    // two sample rows (every 4th column), then the number of pixels of each colour: 64 each, none left over
    assert_eq!(l[0], "row 0: 10 10 11 11 12 12 13 13");
    assert_eq!(l[1], "row 7: 10 10 11 11 12 12 13 13");
    assert_eq!(l[2], "counts none 0 c1 64 c2 64 c3 64 c4 64");
}

// ---- 3. the diagnostics ----

fn dump_dir(tag: &str) -> PathBuf {
    let d = work_dir().join(etxt_common::uniq(tag));
    std::fs::create_dir_all(&d).unwrap();
    d
}
fn files(d: &Path) -> Vec<(String, Vec<u8>)> {
    let mut v: Vec<(String, Vec<u8>)> = std::fs::read_dir(d).unwrap().map(|e| e.unwrap()).map(|e| (e.file_name().to_string_lossy().to_string(), std::fs::read(e.path()).unwrap())).collect();
    v.sort();
    v
}

const SRC_PROG: &str = r#"
    uint8_t bits[6 * 4 * 4];
    for (int y = 0; y < 4; y++) for (int x = 0; x < 6; x++) { uint8_t *p = bits + (y * 6 + x) * 4; p[0] = (uint8_t)(y * 16 + x); p[1] = (uint8_t)x; p[2] = (uint8_t)y; p[3] = 0; }
    fle_rsurf s; memset(&s, 0, sizeof s);
    s.bits = bits; s.width = 6; s.height = 4; s.stride = 24; s.bpp = 32;
    fle_rrect r = { 1, 1, 5, 3 };
    fle_visrgn_srcdump("BitBlt", 0x7f990040u, &s, r);       /* the wanted window */
    fle_visrgn_srcdump("BitBlt", 0x7f990044u, &s, r);       /* another window */
    fle_visrgn_srcdump("BitBlt", 0x7f990040u, &s, r);
    fle_visrgn_srcdump("BitBlt", 0x7f990040u, &s, r);
"#;

// ---- 4. the inputs ----

fn read(p: &Path) -> String {
    std::fs::read_to_string(p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

#[test]
fn the_vis1_seed_is_the_decision_0144_list_plus_three_operating_system_fonts() {
    let seeds = base_dir().join("harness/seeds");
    let g8 = read(&seeds.join("vfs-seed-g8.tsv"));
    let v = read(&seeds.join("vfs-seed-vis1.tsv"));
    assert!(v.starts_with(&g8), "the new list starts with the whole list of decision 0144");
    let extra: Vec<&str> = v[g8.len()..].lines().collect();
    assert_eq!(extra.len(), 4, "{extra:?}");
    assert!(extra[0].starts_with('#') && extra[0].contains("S-font-arial") && extra[0].contains("S-font-tahoma") && extra[0].contains("S-font-lucon"), "{}", extra[0]);
    let want = [("arial.ttf", "LiberationSans-Regular.ttf"), ("tahoma.ttf", "DejaVuSans.ttf"), ("lucon.ttf", "DejaVuSansMono.ttf")];
    for (line, (guest, host)) in extra[1..].iter().zip(want) {
        let f: Vec<&str> = line.split('\t').collect();
        assert_eq!(f.len(), 3, "{line}");
        assert_eq!(f[0], format!("C:\\Windows\\Fonts\\{guest}"));
        // a path relative to the repository root (the lists hold no absolute host path) and a SHA-256
        assert_eq!(f[1], format!("lift/out-vis1/inputs/fonts/{host}"));
        assert!(f[2].len() == 64 && f[2].bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)), "{line}");
    }
    // the SHA-256 of the three files when they are on this machine (they are not in git): the lines of the list must be the files' hashes
    let root = std::env::var("FLE_FR21_SEED_ROOT").map(PathBuf::from).unwrap_or_else(|_| base_dir().join(".."));
    for line in &extra[1..] {
        let f: Vec<&str> = line.split('\t').collect();
        let p = root.join(f[1]);
        if p.exists() {
            let o = Command::new("sha256sum").arg(&p).output().unwrap();
            assert_eq!(String::from_utf8_lossy(&o.stdout).split_whitespace().next().unwrap(), f[2], "{}", p.display());
        }
    }
}

#[test]
fn the_api_probe_lists_agree_and_leave_out_the_five_names_of_the_g8_probe() {
    let h = base_dir().join("harness");
    let inc = read(&h.join("vis1/vis1_apis.inc"));
    let flags = read(&h.join("vis1/link-flags.txt"));
    let names: Vec<&str> = inc.lines().filter_map(|l| l.strip_prefix("V1API(").and_then(|r| r.strip_suffix(')'))).collect();
    let wrapped: Vec<&str> = flags.split_whitespace().filter_map(|f| f.strip_prefix("-Wl,--wrap=fle_imp_")).collect();
    assert_eq!(names.len(), wrapped.len());
    assert_eq!(names, wrapped, "one --wrap flag per V1API line, in the same order");
    assert!(names.len() > 500, "{}", names.len());
    let mut sorted = names.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), names.len(), "no name twice");
    for g8 in ["CreateFileW", "ReadFile", "CreateFileMappingW", "MapViewOfFile", "GetProcAddress"] {
        assert!(!names.contains(&g8), "{g8} is wrapped by g8_probe.c: two wrappers of one name do not link");
    }
    for must in ["BeginPaint", "EndPaint", "GetClipBox", "IntersectClipRect", "SelectClipRgn", "BitBlt", "DefWindowProcW", "RegisterClassW", "LoadLibraryA"] {
        assert!(names.contains(&must), "{must}");
    }
}
