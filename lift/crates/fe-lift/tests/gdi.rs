//! Tests for the virtual GDI object table of runtime/fle_gdi.c: gdi32!CreatePalette, gdi32!DeleteObject, gdi32!GetStockObject and
//! user32!LoadIconW (docs/decisions/0054-virtual-gdi-objects.md; sources: docs/investigations/gdi-objects-caller.md).
//!
//! Expected values are written here from the decision (the rules on the Microsoft Learn pages CreatePalette, LOGPALETTE,
//! PALETTEENTRY, DeleteObject, GetStockObject, LoadIconW, About Icons, and the choices of the decision), not read back from the C
//! code. A number the pages do not give (a handle value, a stock index, palVersion) is marked "choice" or "unofficial" and is the
//! only thing such a test pins down. The host is Linux: nothing is compared with a real Windows; no display, resource file, clock,
//! thread or process is read.
//!
//! The C sources of fle_gdi.c / fle_gdi.h come from runtime/ when fle_gdi.c is there, otherwise from out-gdi/work/ (development).
//! FLE_GDI_SRC_DIR (a directory with fle_gdi.c and fle_gdi.h) overrides both: the mutation runs of the run record use it. Files
//! that already existed always come from runtime/.

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
    if let Ok(d) = std::env::var("FLE_GDI_SRC_DIR") {
        return PathBuf::from(d);
    }
    if runtime_dir().join("fle_gdi.c").exists() {
        runtime_dir()
    } else {
        manifest().join("../../out-gdi/work")
    }
}
fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-gdi/test");
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

/// fle_gdi.c (warnings are errors) and the files it links with: fle_gdc.c (the virtual screen DC: the two handle regions are
/// tested against each other), fle_imports.c (GetLastError / fle_set_last_error) and fle_env.c (fle_stop).
fn objs() -> &'static Vec<PathBuf> {
    static O: OnceLock<Vec<PathBuf>> = OnceLock::new();
    O.get_or_init(|| {
        let dir = work_dir().join(etxt_common::per_process("obj-gdi"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut v = Vec::new();
        for (name, ours) in [("fle_gdi.c", true), ("fle_raster.c", true), ("fle_gdc.c", false), ("fle_imports.c", false), ("fle_env.c", false)] {
            let base = if ours { src_dir() } else { runtime_dir() };
            let o = dir.join(name.replace(".c", ".o"));
            clang_c(&base.join(name), &o, ours);
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

const PRELUDE: &str = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <pthread.h>
#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_gdc.h"
#include "fle_gdi.h"

uint64_t R[16];
static uint64_t stk[32];

/* a[0..3] go to rcx, rdx, r8, r9. */
static uint64_t callv(void (*f)(void), const uint64_t *a)
{
    memset(stk, 0, sizeof stk);
    stk[0] = 0xdead0000u;
    R[1] = a[0]; R[2] = a[1]; R[8] = a[2]; R[9] = a[3];
    R[4] = (uint64_t)(uintptr_t)stk;
    R[0] = 0xBADBADBADull;
    f();
    return R[0];
}
#define CALL(f, ...) callv((void (*)(void))(f), (uint64_t[12]){ __VA_ARGS__ })
#define U(x) ((unsigned long long)(x))
static uint32_t LE(void) { fle_imp_GetLastError(); return (uint32_t)R[0]; }
static void SETLE(uint32_t v) { fle_set_last_error(v); }

/* the guest's LOGPALETTE (call site 0x5c4eef): WORD palVersion = 0x300, WORD palNumEntries = 0x10, then the 16 entries copied from
   the static table at 0x13d8468: peRed, peGreen, peBlue, peFlags of each */
static const uint8_t guest_tbl[64] = {
    0, 0, 0, 0,        128, 0, 0, 0,     0, 128, 0, 0,     128, 128, 0, 0,
    0, 0, 128, 0,      128, 0, 128, 0,   0, 128, 128, 0,   128, 128, 128, 0,
    192, 192, 192, 0,  255, 0, 0, 0,     0, 255, 0, 0,     255, 255, 0, 0,
    0, 0, 255, 0,      255, 0, 255, 0,   0, 255, 255, 0,   255, 255, 255, 0 };
static uint8_t lp[4 + 4 * 256];
static void mk(uint16_t ver, uint16_t n)
{
    memset(lp, 0, sizeof lp);
    lp[0] = (uint8_t)(ver & 255); lp[1] = (uint8_t)(ver >> 8);
    lp[2] = (uint8_t)(n & 255); lp[3] = (uint8_t)(n >> 8);
    memcpy(lp + 4, guest_tbl, 64);
}
#define LPA ((uint64_t)(uintptr_t)lp)
#define MKPAL() (mk(0x300, 16), CALL(fle_imp_CreatePalette, LPA))
#define DEL(h) ((uint32_t)CALL(fle_imp_DeleteObject, (h)))
#define STOCK(i) CALL(fle_imp_GetStockObject, (i))
#define ICON() CALL(fle_imp_LoadIconW, 0, 32512)
#define A16(p) ((uint64_t)(uintptr_t)(p))
#define INREG(h) ((h) > FLE_GDI_HANDLE_BASE && (h) < FLE_GDI_HANDLE_BASE + FLE_GDI_REGION_SIZE)

/* a LOGFONTW of 92 bytes: byte i = seed + i (so that no two fields look alike and a shifted copy shows) */
static uint8_t lfb[92];
static void mklf(uint8_t seed) { for (unsigned i = 0; i < 92; i++) lfb[i] = (uint8_t)(seed + i); }
#define LFA ((uint64_t)(uintptr_t)lfb)
#define MKFONT() CALL(fle_imp_CreateFontIndirectW, LFA)
static void dumpfont(uint64_t h)
{
    uint8_t o[92];
    int k = fle_gdi_font_logfont(h, o);
    printf("logfont %d", k);
    if (k) { printf(" "); for (unsigned i = 0; i < 92; i++) printf("%02x", o[i]); }
    printf("\n");
}
"#;

fn run_src(name: &str, top: &str, body: &str) -> Out {
    let dir = work_dir();
    let uname = etxt_common::uniq(name);
    let c = dir.join(format!("{uname}.c"));
    let exe = dir.join(&uname);
    let src = format!("{PRELUDE}\n{top}\nint main(void)\n{{\n{body}\n    return 0;\n}}\n");
    std::fs::write(&c, src).unwrap();
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fPIE", "-pthread", "-I"]).arg(src_dir()).arg("-I").arg(runtime_dir());
    for o in objs() {
        cmd.arg(o);
    }
    let cc = cmd.arg("-o").arg(&exe).arg(&c).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new(&exe).env("TMPDIR", dir.join("tmp")).output().expect("run failed");
    Out {
        stdout: String::from_utf8_lossy(&r.stdout).to_string(),
        stderr: String::from_utf8_lossy(&r.stderr).to_string(),
        signal: r.status.signal(),
        ok: r.status.success(),
    }
}

fn run(name: &str, body: &str) -> Out {
    run_src(name, "", body)
}

fn lines(name: &str, body: &str) -> Vec<String> {
    let o = run(name, body);
    assert!(o.ok, "{name} did not exit normally (signal {:?}): {}\n{}", o.signal, o.stderr, o.stdout);
    o.stdout.lines().map(|s| s.to_string()).collect()
}

/// Each (label, C statements) must stop the run with SIGABRT, a "fle: <api>" message that contains every word of `words`, and
/// no output after it. `setup` runs first.
fn assert_aborts(prefix: &str, setup: &str, api: &str, words: &[&str], cases: &[(&str, &str)]) {
    for (label, stmts) in cases {
        let body = format!("{setup}\n    {stmts}\n    printf(\"returned %llu\\n\", U(R[0]));");
        let o = run(&format!("{prefix}-{label}"), &body);
        assert!(!o.ok, "{label}: expected an abort, got stdout {}", o.stdout);
        assert!(!o.stdout.contains("returned"), "{label}: returned instead of stopping");
        assert!(o.stderr.contains(&format!("fle: {api}")), "{label}: no 'fle: {api}' message: {}", o.stderr);
        for w in words {
            assert!(o.stderr.contains(w), "{label}: message lacks {w:?}: {}", o.stderr);
        }
        assert_eq!(o.signal, Some(6), "{label}: not SIGABRT");
    }
}

/// The value of `#define NAME <number>` (decimal, 0x.. hex, or UINT64_C(0x..)) in a runtime header.
fn def_const(file: &str, name: &str) -> u64 {
    let t = std::fs::read_to_string(src_dir().join(file)).or_else(|_| std::fs::read_to_string(runtime_dir().join(file))).unwrap();
    let line = t.lines().find(|l| l.starts_with(&format!("#define {name} "))).unwrap_or_else(|| panic!("{name} not in {file}"));
    let rest = line[format!("#define {name} ").len()..].trim_start();
    let rest = rest.strip_prefix("UINT64_C(").unwrap_or(rest);
    if let Some(h) = rest.strip_prefix("0x") {
        let tok: String = h.chars().take_while(|c| c.is_ascii_hexdigit()).collect();
        return u64::from_str_radix(&tok, 16).unwrap();
    }
    rest.split(|c: char| !c.is_ascii_digit()).next().unwrap().parse().unwrap()
}

/// C text without comments (strings are kept).
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

// ---- 1. Table, header, constants ----

const ROWS: [(&str, &str); 7] = [
    ("gdi32.dll", "CreatePalette"),
    ("gdi32.dll", "DeleteObject"),
    ("gdi32.dll", "GetStockObject"),
    ("user32.dll", "LoadIconW"),
    // decision 0066: the kind "font" (CreateFontIndirectW) is added to this table
    ("gdi32.dll", "CreateFontIndirectW"),
    // decision 0068: GetObjectW answers the created font only (section 10 below)
    ("gdi32.dll", "GetObjectW"),
    // decision 0106: CreateBitmap makes the kind "bitmap" (the monochrome cursor mask of ILTools; section 10b below)
    ("gdi32.dll", "CreateBitmap"),
];

#[test]
fn rows_are_in_the_table_and_declared_in_the_header() {
    let table = std::fs::read_to_string(manifest().join("src/imports_table.rs")).unwrap();
    let h = std::fs::read_to_string(src_dir().join("fle_gdi.h")).unwrap();
    for (dll, n) in ROWS {
        let row = format!("(\"{dll}\", \"{n}\", \"fle_imp_{n}\", \"jmp_imp:{n}\")");
        assert_eq!(table.matches(&row).count(), 1, "{n}: exactly one row in imports_table.rs");
        assert!(h.contains(&format!("void fle_imp_{n}(void)")), "{n} is declared in fle_gdi.h");
    }
    // the APIs that need a DC with selection state, a text system, a system palette or a resource loader are not answered
    // (decision 0054): no rows
    // (decision 0108 answers SelectObject, GetSysColor, SelectPalette, RealizePalette, CreateCompatibleDC, DeleteDC and CreateHalftonePalette: they were in this list
    // until then; section 12 below has their rows. Their neighbours below stay unanswered.)
    // (decision 0111 answers or stops by name GetTextMetricsW, GetSystemPaletteEntries, CreateSolidBrush, GetPaletteEntries and DestroyIcon: they were in this list until then; tests/gdi_bulk.rs has their rows)
    // decision 0126 (wall 55): old: "CreatePen" was in this list; it is a row now (runtime/fle_gdi.c: the pen of the pixel model, tests/wall_fr11.rs)
    for n in [
        "GetSysColorBrush",
        "LoadIconA",
        "CreateFontW", "CreateFontIndirectA", "CreateFontIndirectExW",
        "GetObjectA", "GetObject",
    ] {
        // "GetObjectW" was in this list until decision 0068 (runtime/fle_gdi.c, section 10 below): the guest's next stop after
        // CreateFontIndirectW is f_5c0bc0 calling it with c = 0x5c on the font it was given, so the claim "no row" is no longer
        // true. It answers the created font only; the palette, brush, pen, bitmap, DC, stock and deleted forms and every other c
        // stop by name (section 10). GetObjectA is not imported by the guest and stays unanswered.
        // "CreateFontIndirectW" was in this list until decision 0066 (runtime/fle_gdi.c, section 9 below): the guest's next stop after
        // the two SPI font actions is f_8663b0 calling it, with the kind "font" added to this table, so the claim "no row" is no
        // longer true. CreateFontW, the A and Ex forms and CreateHalftonePalette (the next stop) stay unanswered.
        // "LoadImageW" was in this list until decision 0061 (runtime/fle_loadimage.c, tests/loadimage.rs): it is answered there, for
        // the guest's one form only (IMAGE_CURSOR, 32x32, LR_SHARED), with cursor handles of fle_cursor.h / fle_loadimage.h, not
        // GDI objects of this table (a bitmap through LoadImageW still stops there), so the claim "no row" is no longer true.
        // "LoadCursorW" was in this list until decision 0059 (runtime/fle_cursor.c, tests/cursor.rs): it is answered there, in its
        // own table (a cursor handle is not a GDI object of this table), so the claim "no row" is no longer true.
        assert!(!table.contains(&format!("\"{n}\"")), "{n} must not be answered by this decision");
    }
}

#[test]
fn the_decided_constants() {
    // the stock indexes: unofficial (mingw-w64 wingdi.h: NULL_BRUSH 5, BLACK_PEN 7, SYSTEM_FONT 13); written here, not read back
    assert_eq!(def_const("fle_gdi.h", "FLE_GDI_STOCK_NULL_BRUSH"), 5);
    assert_eq!(def_const("fle_gdi.h", "FLE_GDI_STOCK_BLACK_PEN"), 7);
    assert_eq!(def_const("fle_gdi.h", "FLE_GDI_STOCK_SYSTEM_FONT"), 13);
    // Learn "About Icons": IDI_APPLICATION is MAKEINTRESOURCE(32512)
    assert_eq!(def_const("fle_gdi.h", "FLE_GDI_IDI_APPLICATION"), 32512);
    // choices of decision 0054: the palVersion and the count of the guest's LOGPALETTE
    assert_eq!(def_const("fle_gdi.h", "FLE_GDI_PALETTE_VERSION"), 0x300);
    assert_eq!(def_const("fle_gdi.h", "FLE_GDI_PALETTE_ENTRIES"), 16);
    // the slot plan: stock objects by index, then the icon, then created palettes
    assert_eq!(def_const("fle_gdi.h", "FLE_GDI_STOCK_SLOTS"), 32);
    assert_eq!(def_const("fle_gdi.h", "FLE_GDI_ICON_SLOT"), 32);
    assert_eq!(def_const("fle_gdi.h", "FLE_GDI_FIRST_CREATED_SLOT"), 33);
    // decision 0066: sizeof(LOGFONTW) = 5 LONG + 8 BYTE + 32 WCHAR (Learn LOGFONTW) = 92, and the kind number of a font
    assert_eq!(5 * 4 + 8 + 32 * 2, 92);
    assert_eq!(def_const("fle_gdi.h", "FLE_GDI_LOGFONTW_SIZE"), 92);
    assert_eq!(def_const("fle_gdi.h", "FLE_GDI_KIND_FONT"), 6);
}

#[test]
fn the_handle_region_overlaps_nothing_and_survives_32_bit_truncation() {
    let base = def_const("fle_gdi.h", "FLE_GDI_HANDLE_BASE");
    let size = def_const("fle_gdi.h", "FLE_GDI_REGION_SIZE");
    assert_eq!(base, 0x7FBA_0000);
    assert_eq!(size, 0x10_0000);
    // Learn, "Interprocess communication" (64-bit Windows): only the lower 32 bits of a GDI handle are significant, and it may be
    // truncated and sign-extended. Below 0x80000000 both give the same value.
    assert!(base + size <= 0x8000_0000);
    // above the image (ImageBase 0x400000, SizeOfImage 0x18C0000)
    assert!(base > 0x400000 + 0x18C0000);
    // not the region of the virtual screen DCs (decision 0053: 0x7FA90000 + 1 MiB)
    let gb = def_const("fle_gdc.h", "FLE_GDC_HANDLE_BASE");
    let gs = def_const("fle_gdc.h", "FLE_GDC_REGION_SIZE");
    assert_eq!((gb, gs), (0x7FA9_0000, 0x10_0000));
    assert!(base >= gb + gs || base + size <= gb, "the two regions overlap");
    // the other handle regions of the runtime are 0x7FF7.. and above
    assert!(base + size < 0x7FF0_0000_0000);
}

// ---- 2. CreatePalette ----

#[test]
fn a_palette_of_the_guests_table_gets_a_handle_and_keeps_its_entries() {
    let l = lines(
        "gdi-pal",
        r#"
    uint64_t p = MKPAL();
    printf("region %d aligned %d\n", INREG(p), (p % 4) == 0);
    printf("kind %d count %u\n", fle_gdi_kind(p), (unsigned)fle_gdi_palette_count(p));
    for (unsigned i = 0; i < 16; i++) printf("e%u %08x\n", i, (unsigned)fle_gdi_palette_entry(p, i));
    printf("out of range %u\n", (unsigned)fle_gdi_palette_entry(p, 16));
    printf("live %llu pal %llu\n", U(fle_gdi_live_count()), U(fle_gdi_palette_live_count()));
    printf("sign-extends %d\n", (uint64_t)(int64_t)(int32_t)(uint32_t)p == p);
"#,
    );
    // each entry is peRed | peGreen << 8 | peBlue << 16 | peFlags << 24 of the four bytes of the table
    let want = [
        "region 1 aligned 1", "kind 1 count 16", "e0 00000000", "e1 00000080", "e2 00008000", "e3 00008080", "e4 00800000", "e5 00800080", "e6 00808000",
        "e7 00808080", "e8 00c0c0c0", "e9 000000ff", "e10 0000ff00", "e11 0000ffff", "e12 00ff0000", "e13 00ff00ff", "e14 00ffff00", "e15 00ffffff",
        "out of range 0", "live 1 pal 1", "sign-extends 1",
    ];
    assert_eq!(l, want);
}

#[test]
fn the_flags_byte_and_the_entries_are_copied_not_referenced() {
    let l = lines(
        "gdi-pal-copy",
        r#"
    uint64_t p;
    mk(0x300, 16);
    lp[4 + 4 * 3 + 0] = 0x11; lp[4 + 4 * 3 + 1] = 0x22; lp[4 + 4 * 3 + 2] = 0x33; lp[4 + 4 * 3 + 3] = 0x44;   /* entry 3, peFlags 0x44 */
    p = CALL(fle_imp_CreatePalette, LPA);
    memset(lp, 0xEE, sizeof lp);    /* the guest's buffer is its stack: it changes after the call */
    printf("e3 %08x\n", (unsigned)fle_gdi_palette_entry(p, 3));
    printf("e0 %08x e15 %08x\n", (unsigned)fle_gdi_palette_entry(p, 0), (unsigned)fle_gdi_palette_entry(p, 15));
"#,
    );
    assert_eq!(l, ["e3 44332211", "e0 00000000 e15 00ffffff"]);
}

#[test]
fn every_palette_is_new_never_reused_and_the_last_error_is_left_alone() {
    let l = lines(
        "gdi-pal-handles",
        r#"
    uint64_t a, b, c;
    SETLE(0x1234);
    a = MKPAL(); b = MKPAL();
    printf("distinct %d region %d %d\n", a != b, INREG(a), INREG(b));
    printf("delete a %u live %llu\n", DEL(a), U(fle_gdi_live_count()));
    c = MKPAL();
    printf("not reused %d\n", c != a && c != b);
    printf("kinds %d %d %d\n", fle_gdi_kind(a), fle_gdi_kind(b), fle_gdi_kind(c));
    printf("last error %x\n", LE());
    printf("pal live %llu\n", U(fle_gdi_palette_live_count()));
"#,
    );
    assert_eq!(l, ["distinct 1 region 1 1", "delete a 1 live 1", "not reused 1", "kinds 0 1 1", "last error 1234", "pal live 2"]);
}

#[test]
fn a_logpalette_other_than_the_guests_form_stops_the_run() {
    assert_aborts(
        "gdi-abort-pal",
        "",
        "CreatePalette",
        &[],
        &[
            ("null", "CALL(fle_imp_CreatePalette, 0);"),
            ("version-0", "mk(0, 16); CALL(fle_imp_CreatePalette, LPA);"),
            ("version-0x301", "mk(0x301, 16); CALL(fle_imp_CreatePalette, LPA);"),
            ("version-0x200", "mk(0x200, 16); CALL(fle_imp_CreatePalette, LPA);"),
            ("count-0", "mk(0x300, 0); CALL(fle_imp_CreatePalette, LPA);"),
            ("count-1", "mk(0x300, 1); CALL(fle_imp_CreatePalette, LPA);"),
            ("count-15", "mk(0x300, 15); CALL(fle_imp_CreatePalette, LPA);"),
            ("count-17", "mk(0x300, 17); CALL(fle_imp_CreatePalette, LPA);"),
            ("count-256", "mk(0x300, 256); CALL(fle_imp_CreatePalette, LPA);"),
            ("count-65535", "mk(0x300, 65535); CALL(fle_imp_CreatePalette, LPA);"),
        ],
    );
    let o = run("gdi-abort-pal-text", "mk(0x300, 17); CALL(fle_imp_CreatePalette, LPA);");
    assert!(o.stderr.contains("palNumEntries is not 16"), "{}", o.stderr);
    let o = run("gdi-abort-pal-text2", "mk(0x301, 16); CALL(fle_imp_CreatePalette, LPA);");
    assert!(o.stderr.contains("palVersion is not 0x300"), "{}", o.stderr);
    let o = run("gdi-abort-pal-text3", "CALL(fle_imp_CreatePalette, 0);");
    assert!(o.stderr.contains("plpal is NULL"), "{}", o.stderr);
}

// ---- 3. DeleteObject ----

#[test]
fn delete_object_results_follow_the_page() {
    // DeleteObject page: nonzero when deleted; zero if the handle is not valid (an already deleted one, NULL)
    let l = lines(
        "gdi-delete",
        r#"
    uint64_t p = MKPAL();
    printf("first %u\n", DEL(p));
    printf("again %u\n", DEL(p));
    printf("null %u\n", DEL(0));
    printf("live %llu pal %llu kind %d\n", U(fle_gdi_live_count()), U(fle_gdi_palette_live_count()), fle_gdi_kind(p));
"#,
    );
    assert_eq!(l, ["first 1", "again 0", "null 0", "live 0 pal 0 kind 0"]);
}

#[test]
fn delete_object_of_what_the_pages_do_not_define_stops_the_run() {
    assert_aborts(
        "gdi-abort-delete",
        "uint64_t pen = STOCK(7); uint64_t ic = ICON(); uint64_t pal = MKPAL(); uint64_t dc = CALL(fle_imp_GetDC, 0);",
        "DeleteObject",
        &[],
        &[
            ("stock", "CALL(fle_imp_DeleteObject, pen);"),
            ("icon", "CALL(fle_imp_DeleteObject, ic);"),
            ("foreign", "CALL(fle_imp_DeleteObject, 0x1234);"),
            ("screen-dc", "CALL(fle_imp_DeleteObject, dc);"),
            ("event-region", "CALL(fle_imp_DeleteObject, 0x7FFA80000004ull);"),
            ("never-issued-in-region", "CALL(fle_imp_DeleteObject, FLE_GDI_HANDLE_BASE + 4 * 900);"),
            ("region-base", "CALL(fle_imp_DeleteObject, FLE_GDI_HANDLE_BASE);"),
            ("unaligned-in-region", "CALL(fle_imp_DeleteObject, pal + 1);"),
            ("region-end", "CALL(fle_imp_DeleteObject, FLE_GDI_HANDLE_BASE + FLE_GDI_REGION_SIZE);"),
            ("invalid-handle-value", "CALL(fle_imp_DeleteObject, 0xFFFFFFFFFFFFFFFFull);"),
        ],
    );
    let o = run("gdi-abort-delete-stock", "uint64_t pen = STOCK(7); CALL(fle_imp_DeleteObject, pen);");
    assert!(o.stderr.contains("stock object"), "{}", o.stderr);
    let o = run("gdi-abort-delete-icon", "uint64_t ic = ICON(); CALL(fle_imp_DeleteObject, ic);");
    assert!(o.stderr.contains("icon"), "{}", o.stderr);
}

// ---- 4. GetStockObject ----

#[test]
fn the_three_stock_objects_have_fixed_distinct_handles() {
    let l = lines(
        "gdi-stock",
        r#"
    uint64_t b = STOCK(5), p = STOCK(7), f = STOCK(13);
    printf("region %d %d %d\n", INREG(b), INREG(p), INREG(f));
    printf("distinct %d %d %d\n", b != p, b != f, p != f);
    printf("stable %d %d %d\n", STOCK(5) == b, STOCK(7) == p, STOCK(13) == f);
    printf("kinds %d %d %d\n", fle_gdi_kind(b), fle_gdi_kind(p), fle_gdi_kind(f));
    printf("live %llu pal %llu\n", U(fle_gdi_live_count()), U(fle_gdi_palette_live_count()));
    printf("not a palette %u\n", (unsigned)fle_gdi_palette_count(b));
    printf("sign-extends %d\n", (uint64_t)(int64_t)(int32_t)(uint32_t)f == f);
"#,
    );
    // kinds: 2 stock brush, 3 stock pen, 4 stock font (fle_gdi.h)
    assert_eq!(l, ["region 1 1 1", "distinct 1 1 1", "stable 1 1 1", "kinds 2 3 4", "live 3 pal 0", "not a palette 0", "sign-extends 1"]);
}

#[test]
fn only_the_low_dword_of_the_stock_index_counts() {
    // GetStockObject(int): the guest sets ecx; the upper half of rcx may hold anything
    let l = lines(
        "gdi-stock-dword",
        r#"
    uint64_t pen = STOCK(7);
    printf("%d\n", CALL(fle_imp_GetStockObject, 0xAAAAAAAA00000007ull) == pen);
"#,
    );
    assert_eq!(l, ["1"]);
}

#[test]
fn stock_indexes_the_guest_does_not_use_stop_by_number() {
    // decision 0126 (wall 55): old: 0..4 (the other brushes), 6 (WHITE_PEN) and 8 (NULL_PEN) were in this list and stopped; they answer now (the stock brushes and pens of the pixel model, S-stock-brush-colour; the numbers are the SDK's:
    // unofficial, tests/wall_fr11.rs). 9 (not an index), 10..12 the other fonts, 14 DEVICE_DEFAULT_FONT, 15 DEFAULT_PALETTE, 16 SYSTEM_FIXED_FONT, 17 DEFAULT_GUI_FONT, 18 DC_BRUSH, 19 DC_PEN, 20.. past the list, big and negative values still stop
    let idx = [9u32, 10, 11, 12, 14, 15, 16, 17, 18, 19, 20, 31, 32, 33, 0x7FFFFFFF, 0x80000000, 0xFFFFFFFF];
    let cases: Vec<(String, String)> = idx.iter().map(|i| (format!("idx-{i}"), format!("CALL(fle_imp_GetStockObject, {i});"))).collect();
    let refs: Vec<(&str, &str)> = cases.iter().map(|(l, c)| (l.as_str(), c.as_str())).collect();
    assert_aborts("gdi-abort-stock", "", "GetStockObject", &["is not answered"], &refs);
    let o = run("gdi-abort-stock-text", "CALL(fle_imp_GetStockObject, 17);");
    assert!(o.stderr.contains("index 17 is not answered"), "{}", o.stderr);
}

// ---- 5. LoadIconW ----

#[test]
fn the_application_icon_has_one_handle_for_every_call() {
    let l = lines(
        "gdi-icon",
        r#"
    uint64_t a, b, p, s;
    SETLE(0x1234);
    a = ICON(); b = ICON();
    p = MKPAL(); s = STOCK(7);
    printf("same %d region %d kind %d\n", a == b, INREG(a), fle_gdi_kind(a));
    printf("distinct %d %d\n", a != p, a != s);
    printf("live %llu pal %llu\n", U(fle_gdi_live_count()), U(fle_gdi_palette_live_count()));
    printf("last error %x\n", LE());
    printf("sign-extends %d\n", (uint64_t)(int64_t)(int32_t)(uint32_t)a == a);
"#,
    );
    // icon kind 5 (fle_gdi.h); live: icon + palette + stock pen
    assert_eq!(l, ["same 1 region 1 kind 5", "distinct 1 1", "live 3 pal 1", "last error 1234", "sign-extends 1"]);
}

#[test]
fn load_icon_of_anything_but_idi_application_from_no_module_stops_the_run() {
    assert_aborts(
        "gdi-abort-icon",
        "",
        "LoadIconW",
        &[],
        &[
            ("hinstance", "CALL(fle_imp_LoadIconW, 0x400000, 32512);"),
            ("hinstance-small", "CALL(fle_imp_LoadIconW, 1, 32512);"),
            ("idi-error", "CALL(fle_imp_LoadIconW, 0, 32513);"),
            ("idi-question", "CALL(fle_imp_LoadIconW, 0, 32514);"),
            ("idi-winlogo", "CALL(fle_imp_LoadIconW, 0, 32517);"),
            ("idi-shield", "CALL(fle_imp_LoadIconW, 0, 32518);"),
            ("zero", "CALL(fle_imp_LoadIconW, 0, 0);"),
            ("one", "CALL(fle_imp_LoadIconW, 0, 1);"),
            // decision 0069: a pointer to a string other than L"MAINICON" stops (the readable cases are in the tests below)
            ("name-pointer-other-string", "static const uint16_t s[] = { 'A', 'P', 'P', 0 }; CALL(fle_imp_LoadIconW, 0, A16(s));"),
        ],
    );
    // "high-bits-set" (0x100000000 + 32512) was in this list until decision 0069: a value above 16 bits is a string pointer now
    // (IS_INTRESOURCE is false), and an address that is not readable cannot be told from a pointer, so the case is not a stop test.
    let o = run("gdi-abort-icon-text1", "CALL(fle_imp_LoadIconW, 0x400000, 32512);");
    assert!(o.stderr.contains("hInstance is not NULL"), "{}", o.stderr);
    let o = run("gdi-abort-icon-text2", "CALL(fle_imp_LoadIconW, 0, 32513);");
    assert!(o.stderr.contains("lpIconName is not IDI_APPLICATION"), "{}", o.stderr);
}

// ---- 5b. LoadIconW(NULL, L"MAINICON") returns NULL (decision 0069, provisional) ----
//
// Learn LoadIconW: "If the function fails, the return value is NULL"; hInstance NULL means a predefined system icon and the page
// does not say what a string name returns (so NULL is a choice of the decision, as is "the last error is not touched").

const MAINICON_TOP: &str = r#"
#include <sys/mman.h>
#include <unistd.h>
static const uint16_t MAINICON_W[] = { 'M', 'A', 'I', 'N', 'I', 'C', 'O', 'N', 0 };
/* n units at the very end of a readable page, the next page not readable: a read past the last unit faults */
static uint16_t *at_page_end(const uint16_t *u, unsigned n)
{
    long ps = sysconf(_SC_PAGESIZE);
    uint8_t *m = mmap(0, 2 * ps, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    uint16_t *p = (uint16_t *)(m + ps - 2 * n);
    mprotect(m + ps, ps, PROT_NONE);
    memcpy(p, u, 2 * n);
    return p;
}
"#;

#[test]
fn mainicon_from_no_module_returns_null_and_changes_nothing() {
    let o = run_src(
        "gdi-mainicon-null",
        MAINICON_TOP,
        r#"
    uint64_t ic, r;
    uint64_t before_live = fle_gdi_live_count();
    SETLE(0x1234);
    r = CALL(fle_imp_LoadIconW, 0, A16(MAINICON_W));
    printf("ret %llu\n", U(r));
    printf("regs %d %d %d %d %d\n", R[1] == 0, R[2] == A16(MAINICON_W), R[8] == 0, R[9] == 0, R[4] == A16(stk));
    printf("return address %llx\n", U(stk[0]));
    printf("last error %x\n", LE());
    printf("live %d\n", fle_gdi_live_count() == before_live);
    ic = ICON();
    printf("idi still answered %d region %d kind %d\n", ic != 0, INREG(ic), fle_gdi_kind(ic));
    printf("null again %llu\n", U(CALL(fle_imp_LoadIconW, 0, A16(MAINICON_W))));
    printf("same icon %d\n", ICON() == ic);
    printf("live after %llu\n", U(fle_gdi_live_count()));
"#,
    );
    assert!(o.ok, "{}", o.stderr);
    let l: Vec<&str> = o.stdout.lines().collect();
    assert_eq!(
        l,
        [
            "ret 0",
            "regs 1 1 1 1 1",
            "return address dead0000",
            "last error 1234",
            "live 1",
            "idi still answered 1 region 1 kind 5",
            "null again 0",
            "same icon 1",
            "live after 1",
        ]
    );
}

#[test]
fn only_the_exact_utf16_mainicon_is_answered() {
    // each string differs from L"MAINICON" by one thing: case, a missing / extra / replaced unit, an empty name, a unit whose low
    // byte is right and whose high byte is not, a trailing blank
    let cases: [(&str, &str); 9] = [
        ("lower", "{ 'm','a','i','n','i','c','o','n',0 }"),
        ("short", "{ 'M','A','I','N','I','C','O',0 }"),
        ("long", "{ 'M','A','I','N','I','C','O','N','X',0 }"),
        ("blank", "{ 'M','A','I','N','I','C','O','N',' ',0 }"),
        ("replaced", "{ 'M','A','I','N','I','C','O','M',0 }"),
        ("empty", "{ 0 }"),
        ("high-byte", "{ 'M','A','I','N','I','C','O',0x104E,0 }"),
        ("first-high-byte", "{ 0x014D,'A','I','N','I','C','O','N',0 }"),
        ("ansi-order", "{ 'M'<<8,'A'<<8,'I'<<8,'N'<<8,'I'<<8,'C'<<8,'O'<<8,'N'<<8,0 }"),
    ];
    for (label, init) in cases {
        let o = run_src(
            &format!("gdi-mainicon-not-{label}"),
            MAINICON_TOP,
            &format!("    static const uint16_t s[] = {init};\n    CALL(fle_imp_LoadIconW, 0, A16(s));\n    printf(\"returned %llu\\n\", U(R[0]));"),
        );
        assert!(!o.ok && !o.stdout.contains("returned"), "{label}: {}", o.stdout);
        assert_eq!(o.signal, Some(6), "{label}");
        assert!(o.stderr.contains("fle: LoadIconW") && o.stderr.contains("other than L\"MAINICON\""), "{label}: {}", o.stderr);
    }
    // a NUL stops the read: units after it are not part of the name
    let o = run_src(
        "gdi-mainicon-after-nul",
        MAINICON_TOP,
        "    static const uint16_t s[] = { 'M','A','I','N','I','C','O','N',0,'X','Y' };\n    printf(\"ret %llu\\n\", U(CALL(fle_imp_LoadIconW, 0, A16(s))));",
    );
    assert!(o.ok && o.stdout == "ret 0\n", "{} {}", o.stdout, o.stderr);
}

#[test]
fn mainicon_with_a_module_handle_still_stops_on_the_module() {
    assert_aborts(
        "gdi-mainicon-hinst",
        "",
        "LoadIconW",
        &["hInstance is not NULL"],
        &[
            ("dll", "static const uint16_t s[] = { 'M','A','I','N','I','C','O','N',0 }; CALL(fle_imp_LoadIconW, 0x400000, A16(s));"),
            ("one", "static const uint16_t s[] = { 'M','A','I','N','I','C','O','N',0 }; CALL(fle_imp_LoadIconW, 1, A16(s));"),
            ("high", "static const uint16_t s[] = { 'M','A','I','N','I','C','O','N',0 }; CALL(fle_imp_LoadIconW, 0x100000000ull, A16(s));"),
        ],
    );
}

#[test]
fn the_name_is_read_in_at_most_16_units_and_never_past_the_nul() {
    // the exact name at the end of a page: a read past its NUL would fault (signal 11), not answer
    let o = run_src(
        "gdi-mainicon-page-end",
        MAINICON_TOP,
        "    uint16_t *p = at_page_end(MAINICON_W, 9);\n    printf(\"ret %llu\\n\", U(CALL(fle_imp_LoadIconW, 0, A16(p))));",
    );
    assert!(o.ok && o.stdout == "ret 0\n", "{} {}", o.stdout, o.stderr);
    // a name that is not MAINICON, ending in its NUL at the page end: stops by name, no fault
    let o = run_src(
        "gdi-mainicon-page-end-other",
        MAINICON_TOP,
        "    static const uint16_t s[] = { 'A','B','C',0 };\n    uint16_t *p = at_page_end(s, 4);\n    CALL(fle_imp_LoadIconW, 0, A16(p));\n    printf(\"returned\\n\");",
    );
    assert_eq!(o.signal, Some(6), "{}", o.stderr);
    assert!(o.stderr.contains("other than L\"MAINICON\""), "{}", o.stderr);
    // 16 units without a NUL at the page end: the 16 are read and the run stops on the length, the 17th unit is not touched
    let o = run_src(
        "gdi-mainicon-16-units",
        MAINICON_TOP,
        "    static const uint16_t s[16] = { 'M','A','I','N','I','C','O','N','M','A','I','N','I','C','O','N' };\n    uint16_t *p = at_page_end(s, 16);\n    CALL(fle_imp_LoadIconW, 0, A16(p));\n    printf(\"returned\\n\");",
    );
    assert_eq!(o.signal, Some(6), "{}", o.stderr);
    assert!(o.stderr.contains("longer than 16 UTF-16 units"), "{}", o.stderr);
    // 15 units and the NUL (16 units in all) at the page end: not MAINICON, stops by name (the NUL is the 16th unit read)
    let o = run_src(
        "gdi-mainicon-15-units",
        MAINICON_TOP,
        "    static const uint16_t s[16] = { 'M','A','I','N','I','C','O','N','M','A','I','N','I','C','O',0 };\n    uint16_t *p = at_page_end(s, 16);\n    CALL(fle_imp_LoadIconW, 0, A16(p));\n    printf(\"returned\\n\");",
    );
    assert_eq!(o.signal, Some(6), "{}", o.stderr);
    assert!(o.stderr.contains("other than L\"MAINICON\""), "{}", o.stderr);
    // a long name that begins like MAINICON: the length stop comes first, whatever the first units are
    let o = run_src(
        "gdi-mainicon-17-units",
        MAINICON_TOP,
        "    static const uint16_t s[18] = { 'M','A','I','N','I','C','O','N',1,1,1,1,1,1,1,1,1,0 };\n    CALL(fle_imp_LoadIconW, 0, A16(s));\n    printf(\"returned\\n\");",
    );
    assert_eq!(o.signal, Some(6), "{}", o.stderr);
    assert!(o.stderr.contains("longer than 16 UTF-16 units"), "{}", o.stderr);
}

#[test]
fn the_smallest_pointer_value_above_16_bits_is_a_string_and_0xffff_is_an_identifier() {
    // IS_INTRESOURCE: the high 48 bits are zero. A page is mapped at 0x10000 (vm.mmap_min_addr = 65536 allows it) with L"MAINICON".
    let top = r#"
#include <sys/mman.h>
static uint16_t *low_name(const char *s)
{
    uint16_t *p = mmap((void *)0x10000, 4096, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED_NOREPLACE, -1, 0);
    size_t i, n = strlen(s);
    if ((uint64_t)(uintptr_t)p != 0x10000) { printf("no low page\n"); exit(3); }
    for (i = 0; i < n; i++) p[i] = (uint8_t)s[i];
    p[n] = 0;
    return p;
}
"#;
    let o = run_src("gdi-mainicon-0x10000", top, "    uint16_t *p = low_name(\"MAINICON\");\n    printf(\"ret %llu\\n\", U(CALL(fle_imp_LoadIconW, 0, A16(p))));");
    assert!(o.ok && o.stdout == "ret 0\n", "{} {}", o.stdout, o.stderr);
    let o = run_src("gdi-other-0x10000", top, "    uint16_t *p = low_name(\"OTHER\");\n    CALL(fle_imp_LoadIconW, 0, A16(p));\n    printf(\"returned\\n\");");
    assert_eq!(o.signal, Some(6), "{}", o.stderr);
    assert!(o.stderr.contains("other than L\"MAINICON\""), "{}", o.stderr);
    // 0xFFFF is an identifier, not a string: not IDI_APPLICATION, so it stops by the identifier rule
    assert_aborts("gdi-abort-icon-0xffff", "", "LoadIconW", &["not IDI_APPLICATION"], &[("max-id", "CALL(fle_imp_LoadIconW, 0, 0xFFFF);")]);
}

#[test]
fn the_name_length_limit_is_the_decided_one() {
    assert_eq!(def_const("fle_gdi.h", "FLE_GDI_ICON_NAME_MAX_UNITS"), 16);
    assert_eq!("MAINICON".len(), 8);
}

#[test]
fn a_second_thread_stops_mainicon_too() {
    let o = run_src(
        "gdi-mainicon-thread",
        &format!("{MAINICON_TOP}\nstatic void *th(void *x) {{ (void)x; CALL(fle_imp_LoadIconW, 0, A16(MAINICON_W)); printf(\"returned-from-thread\\n\"); return 0; }}"),
        "    pthread_t t;\n    ICON();\n    pthread_create(&t, 0, th, 0);\n    pthread_join(t, 0);\n    printf(\"returned\\n\");",
    );
    assert!(!o.ok && !o.stdout.contains("returned"), "{}", o.stdout);
    assert_eq!(o.signal, Some(6));
    assert!(o.stderr.contains("fle: LoadIconW") && o.stderr.contains("second thread"), "{}", o.stderr);
}

// ---- 6. Together with the virtual screen DC, threads ----

#[test]
fn gdi_handles_and_screen_dc_handles_are_different_kinds() {
    let l = lines(
        "gdi-vs-gdc",
        r#"
    uint64_t dc = CALL(fle_imp_GetDC, 0), p = MKPAL(), s = STOCK(13), ic = ICON();
    printf("regions %d\n", FLE_GDI_HANDLE_BASE >= FLE_GDC_HANDLE_BASE + FLE_GDC_REGION_SIZE);
    printf("dc not gdi %d %d %d\n", INREG(dc), fle_gdi_kind(dc), dc != p && dc != s && dc != ic);
    printf("gdi not dc %d %d %d\n", fle_gdc_is_live(p), fle_gdc_is_live(s), fle_gdc_is_live(ic));
    printf("release dc %u\n", (unsigned)CALL(fle_imp_ReleaseDC, 0, dc));
    printf("delete p %u\n", DEL(p));
    printf("dc live %llu gdi live %llu\n", U(fle_gdc_live_count()), U(fle_gdi_live_count()));
"#,
    );
    assert_eq!(l, ["regions 1", "dc not gdi 0 0 1", "gdi not dc 0 0 0", "release dc 1", "delete p 1", "dc live 0 gdi live 2"]);
}

#[test]
fn a_gdi_handle_is_not_a_dc_for_release_dc() {
    // the screen DC table stops on a value it did not issue (decision 0053); the two tables do not share handles
    assert_aborts("gdi-abort-release-gdi", "uint64_t p = MKPAL();", "ReleaseDC", &[], &[("palette", "CALL(fle_imp_ReleaseDC, 0, p);")]);
}

#[test]
fn a_second_thread_stops_the_run() {
    for (label, api, call) in [
        ("createpalette", "CreatePalette", "CALL(fle_imp_CreatePalette, LPA)"),
        ("deleteobject", "DeleteObject", "CALL(fle_imp_DeleteObject, h)"),
        ("getstockobject", "GetStockObject", "CALL(fle_imp_GetStockObject, 7)"),
        ("loadiconw", "LoadIconW", "CALL(fle_imp_LoadIconW, 0, 32512)"),
        ("createfontindirectw", "CreateFontIndirectW", "CALL(fle_imp_CreateFontIndirectW, LFA)"),
        ("createbitmap", "CreateBitmap", "CALL(fle_imp_CreateBitmap, 8, 8, 1, 1)"),
        // decision 0068: the thread check comes first, whatever the handle is (h is the stock brush here)
        ("getobjectw", "GetObjectW", "CALL(fle_imp_GetObjectW, h, 92, LFA)"),
    ] {
        let o = run_src(
            &format!("gdi-thread-{label}"),
            &format!("static uint64_t h; static void *th(void *x) {{ (void)x; mk(0x300, 16); {call}; printf(\"returned-from-thread\\n\"); return 0; }}"),
            "    pthread_t t;\n    h = STOCK(5);\n    pthread_create(&t, 0, th, 0);\n    pthread_join(t, 0);\n    printf(\"returned\\n\");",
        );
        assert!(!o.ok && !o.stdout.contains("returned"), "{label}: {}", o.stdout);
        assert_eq!(o.signal, Some(6), "{label}");
        assert!(o.stderr.contains(&format!("fle: {api}")) && o.stderr.contains("second thread"), "{label}: {}", o.stderr);
    }
}

// ---- 9. Fonts: CreateFontIndirectW (decision 0066) ----
//
// Learn CreateFontIndirectW: returns "a handle to a logical font", NULL on failure (no GetLastError value is given); "When you no
// longer need the font, call the DeleteObject function". DeleteObject lists "font" among the objects it deletes: nonzero on
// success, zero if the handle is not valid or is selected into a DC (no selection exists in this runtime). The handle value, the
// slot plan (a font takes the next created slot, like a palette) and "every call a new handle" are choices of the decision.

#[test]
fn a_font_gets_a_handle_in_the_gdi_region_and_the_table_keeps_a_copy_of_the_92_bytes() {
    let l = lines(
        "gdi-font",
        r#"
    uint64_t f;
    mklf(0x10);
    f = MKFONT();
    printf("region %d aligned %d kind %d\n", INREG(f), (f % 4) == 0, fle_gdi_kind(f));
    printf("live %llu fonts %llu pal %llu\n", U(fle_gdi_live_count()), U(fle_gdi_font_live_count()), U(fle_gdi_palette_live_count()));
    printf("sign-extends %d\n", (uint64_t)(int64_t)(int32_t)(uint32_t)f == f);
    dumpfont(f);
    printf("slot %llu\n", U((f - FLE_GDI_HANDLE_BASE) / 4 - 1));
"#,
    );
    let want: String = (0..92u32).map(|i| format!("{:02x}", (0x10 + i) as u8)).collect();
    // the first created slot is 33 (fle_gdi.h: stock 0..31, icon 32, created from 33): a font and a palette share the sequence
    assert_eq!(l, ["region 1 aligned 1 kind 6", "live 1 fonts 1 pal 0", "sign-extends 1", &format!("logfont 1 {want}"), "slot 33"]);
}

#[test]
fn the_copy_is_taken_at_the_call_and_two_fonts_keep_their_own() {
    let l = lines(
        "gdi-font-copy",
        r#"
    uint64_t a, b;
    mklf(0x20); a = MKFONT();
    mklf(0x90); b = MKFONT();
    memset(lfb, 0xEE, sizeof lfb);      /* the guest's buffer is its stack: it changes after the call */
    printf("distinct %d region %d %d\n", a != b, INREG(a), INREG(b));
    dumpfont(a); dumpfont(b);
"#,
    );
    let hexs = |s: u8| (0..92u32).map(|i| format!("{:02x}", s.wrapping_add(i as u8))).collect::<String>();
    assert_eq!(l, ["distinct 1 region 1 1", &format!("logfont 1 {}", hexs(0x20)), &format!("logfont 1 {}", hexs(0x90))]);
}

#[test]
fn only_the_92_bytes_of_the_logfontw_are_read() {
    // bytes after the structure are not copied (they are the guest's), and a LOGFONTW right at the end of a buffer is readable
    let l = lines(
        "gdi-font-extent",
        r#"
    uint8_t big[96 + 4];
    uint64_t f;
    for (unsigned i = 0; i < sizeof big; i++) big[i] = (uint8_t)(0x40 + i);
    f = CALL(fle_imp_CreateFontIndirectW, (uint64_t)(uintptr_t)big);
    { uint8_t o[92]; fle_gdi_font_logfont(f, o);
      printf("first %02x last %02x\n", o[0], o[91]); }
"#,
    );
    assert_eq!(l, ["first 40 last 9b"]);
}

#[test]
fn the_face_name_is_kept_as_it_is_without_a_terminator_check_or_a_font_mapper() {
    // the page: lfFaceName is a null-terminated string of at most 32 TCHAR; the mapper runs when the font is selected, so a name
    // without a terminator, an empty one and a name that no font has are all kept (nothing here selects or maps)
    let l = lines(
        "gdi-font-face",
        r#"
    uint8_t o[92];
    uint64_t f;
    memset(lfb, 'A', sizeof lfb);      /* 32 units 0x4141: no terminator */
    f = MKFONT(); fle_gdi_font_logfont(f, o);
    printf("%02x%02x %02x%02x\n", o[28], o[29], o[90], o[91]);
    memset(lfb, 0, sizeof lfb);        /* all zero: height 0, empty face */
    f = MKFONT();
    printf("kind %d\n", fle_gdi_kind(f));
"#,
    );
    assert_eq!(l, ["4141 4141", "kind 6"]);
}

#[test]
fn a_font_never_reuses_a_slot_and_shares_the_sequence_with_palettes_and_leaves_the_last_error() {
    let l = lines(
        "gdi-font-seq",
        r#"
    uint64_t a, p, b, c;
    SETLE(0x1234);
    mklf(1);
    a = MKFONT(); p = MKPAL(); b = MKFONT();
    printf("slots %llu %llu %llu\n", U((a - FLE_GDI_HANDLE_BASE) / 4 - 1), U((p - FLE_GDI_HANDLE_BASE) / 4 - 1), U((b - FLE_GDI_HANDLE_BASE) / 4 - 1));
    printf("delete a %u fonts %llu\n", DEL(a), U(fle_gdi_font_live_count()));
    c = MKFONT();
    printf("not reused %d slot %llu\n", c != a && c != b && c != p, U((c - FLE_GDI_HANDLE_BASE) / 4 - 1));
    printf("kinds %d %d %d %d\n", fle_gdi_kind(a), fle_gdi_kind(p), fle_gdi_kind(b), fle_gdi_kind(c));
    printf("live %llu fonts %llu pal %llu\n", U(fle_gdi_live_count()), U(fle_gdi_font_live_count()), U(fle_gdi_palette_live_count()));
    printf("last error %x\n", LE());
    printf("ra %llx\n", U(stk[0]));
"#,
    );
    assert_eq!(
        l,
        ["slots 33 34 35", "delete a 1 fonts 1", "not reused 1 slot 36", "kinds 0 1 6 6", "live 3 fonts 2 pal 1", "last error 1234", "ra dead0000"]
    );
}

#[test]
fn delete_object_of_a_font_follows_the_page() {
    // the DeleteObject page lists fonts: nonzero when deleted; zero when the handle is no longer valid. The copy goes with it.
    let l = lines(
        "gdi-font-delete",
        r#"
    uint64_t f;
    mklf(7);
    f = MKFONT();
    printf("first %u\n", DEL(f));
    printf("again %u\n", DEL(f));
    printf("live %llu fonts %llu kind %d\n", U(fle_gdi_live_count()), U(fle_gdi_font_live_count()), fle_gdi_kind(f));
    dumpfont(f);
"#,
    );
    assert_eq!(l, ["first 1", "again 0", "live 0 fonts 0 kind 0", "logfont 0"]);
}

#[test]
fn a_stock_font_is_not_a_created_font_and_still_stops_on_delete() {
    // SYSTEM_FONT (13) is a stock object of this table: it has no LOGFONTW copy, and DeleteObject on it stops as before
    let l = lines(
        "gdi-font-stock",
        r#"
    uint64_t s = STOCK(13);
    printf("kind %d fonts %llu\n", fle_gdi_kind(s), U(fle_gdi_font_live_count()));
    dumpfont(s);
    dumpfont(0); dumpfont(FLE_GDI_HANDLE_BASE + 4 * 900);
"#,
    );
    assert_eq!(l, ["kind 4 fonts 0", "logfont 0", "logfont 0", "logfont 0"]);
    assert_aborts("gdi-abort-delete-stockfont", "uint64_t s = STOCK(13);", "DeleteObject", &["stock object"], &[("stock-font", "CALL(fle_imp_DeleteObject, s);")]);
}

#[test]
fn a_font_handle_is_not_a_palette_a_dc_or_a_cursor_and_the_other_tables_stop_on_it() {
    let l = lines(
        "gdi-font-kinds",
        r#"
    uint64_t f, p;
    mklf(3);
    f = MKFONT(); p = MKPAL();
    printf("palette count of a font %u of a palette %u\n", (unsigned)fle_gdi_palette_count(f), (unsigned)fle_gdi_palette_count(p));
    printf("logfont of a palette: "); dumpfont(p);
    printf("is a dc %d\n", fle_gdc_is_live(f));
"#,
    );
    assert_eq!(l, ["palette count of a font 0 of a palette 16", "logfont of a palette: logfont 0", "is a dc 0"]);
    assert_aborts("gdi-abort-release-font", "uint64_t f; mklf(3); f = MKFONT();", "ReleaseDC", &[], &[("font", "CALL(fle_imp_ReleaseDC, 0, f);")]);
}

#[test]
fn a_null_logfont_pointer_stops_the_run_and_creates_nothing() {
    assert_aborts(
        "gdi-abort-font",
        "",
        "CreateFontIndirectW",
        &["lplf is NULL"],
        &[("null", "CALL(fle_imp_CreateFontIndirectW, 0);")],
    );
    // a NULL stop creates nothing: it is the abort itself, so the only observable is that no output follows (assert_aborts); the
    // message names the page's silence on this form
    let o = run("gdi-abort-font-text", "CALL(fle_imp_CreateFontIndirectW, 0);");
    assert!(o.stderr.contains("the page does not define this call"), "{}", o.stderr);
}

#[test]
fn the_font_and_the_other_tables_use_regions_that_do_not_overlap() {
    // decision 0066 adds no region: the fonts use the handle region of this table (0x7FBA0000 + 1 MiB). The other virtual handle
    // regions of the runtime (written here from their headers' constants) must stay disjoint from it and from each other.
    let gdi = (def_const("fle_gdi.h", "FLE_GDI_HANDLE_BASE"), def_const("fle_gdi.h", "FLE_GDI_REGION_SIZE"));
    assert_eq!(gdi, (0x7FBA_0000, 0x10_0000));
    let mut regions = vec![
        ("gdc", def_const("fle_gdc.h", "FLE_GDC_HANDLE_BASE"), def_const("fle_gdc.h", "FLE_GDC_REGION_SIZE")),
        ("gdi", gdi.0, gdi.1),
        ("cursor", def_const("fle_cursor.h", "FLE_CURSOR_HANDLE_BASE"), def_const("fle_cursor.h", "FLE_CURSOR_REGION_SIZE")),
        ("loadimage", def_const("fle_loadimage.h", "FLE_LI_MODULE_HANDLE_BASE"), def_const("fle_loadimage.h", "FLE_LI_REGION_SIZE")),
        ("monitor", def_const("fle_monitor.h", "FLE_MONITOR_HANDLE_BASE"), def_const("fle_monitor.h", "FLE_MONITOR_REGION_SIZE")),
    ];
    regions.sort_by_key(|r| r.1);
    for w in regions.windows(2) {
        assert!(w[0].1 + w[0].2 <= w[1].1, "regions {} and {} overlap", w[0].0, w[1].0);
    }
    for (name, base, size) in &regions {
        assert!(base + size <= 0x8000_0000, "{name} is above 0x80000000 (32-bit truncation)");
    }
    // and in a run: a font handle, a palette, a stock object and the icon are all inside the gdi region and are not in another one
    let l = lines(
        "gdi-font-regions",
        r#"
    uint64_t f, p, s, ic;
    mklf(5);
    f = MKFONT(); p = MKPAL(); s = STOCK(13); ic = ICON();
    printf("all in the gdi region %d %d %d %d\n", INREG(f), INREG(p), INREG(s), INREG(ic));
    printf("none in the dc region %d\n", f >= FLE_GDC_HANDLE_BASE && f < FLE_GDC_HANDLE_BASE + FLE_GDC_REGION_SIZE);
"#,
    );
    assert_eq!(l, ["all in the gdi region 1 1 1 1", "none in the dc region 0"]);
}

// ---- 10. GetObjectW (decision 0068) ----
//
// Learn GetObjectW: "If hgdiobj is a handle to a font, the LOGFONT that is returned is the LOGFONT used to create the font"; with a
// valid pv the return value is "the number of bytes stored into the buffer", zero on failure. The page does not say what is stored
// for a c smaller or larger than the structure, and gives the byte count for a NULL pv. Answered here: a live created font,
// c == sizeof(LOGFONTW) = 0x5c (92, computed from the LOGFONTW page) and pv not NULL. Everything else stops by name, with a reason
// of its own for each form. The handle values, the slot plan and "the last error is left alone" are choices of the decisions.

const GETOBJ_REGS: &str = r#"
    R[3] = 0x3333333333333333ull; R[5] = 0x5555555555555555ull; R[6] = 0x6666666666666666ull; R[7] = 0x7777777777777777ull;
    R[12] = 0xCCCCCCCCCCCCCCCCull; R[13] = 0xDDDDDDDDDDDDDDDDull; R[14] = 0xEEEEEEEEEEEEEEEEull; R[15] = 0xFFFFFFFFFFFFFFFFull;
"#;

#[test]
fn the_created_font_is_answered_with_its_92_byte_copy_and_92() {
    let l = lines(
        "gdi-getobj-font",
        &format!(
            r#"{GETOBJ_REGS}
    uint8_t buf[4 + 92 + 4];
    uint64_t f, r;
    unsigned i;
    memset(buf, 0xC7, sizeof buf);
    mklf(0x30);
    f = MKFONT();
    SETLE(0x1234);
    R[3] = 0x3333333333333333ull;
    r = CALL(fle_imp_GetObjectW, f, 92, (uint64_t)(uintptr_t)(buf + 4));
    printf("ret %llu\n", U(r));
    printf("buf ");
    for (i = 0; i < sizeof buf; i++) printf("%02x", buf[i]);
    printf("\n");
    printf("callee-saved %d\n", R[3] == 0x3333333333333333ull && R[5] == 0x5555555555555555ull && R[6] == 0x6666666666666666ull &&
           R[7] == 0x7777777777777777ull && R[12] == 0xCCCCCCCCCCCCCCCCull && R[13] == 0xDDDDDDDDDDDDDDDDull &&
           R[14] == 0xEEEEEEEEEEEEEEEEull && R[15] == 0xFFFFFFFFFFFFFFFFull);
    printf("rsp %d ra %llx\n", R[4] == (uint64_t)(uintptr_t)stk, U(stk[0]));
    printf("args %d\n", R[1] == f && R[2] == 92 && R[8] == (uint64_t)(uintptr_t)(buf + 4));
    printf("last error %x\n", LE());
    printf("table copy still "); dumpfont(f);
"#
        ),
    );
    let font: String = (0..92u32).map(|i| format!("{:02x}", (0x30 + i) as u8)).collect();
    assert_eq!(
        l,
        [
            "ret 92".to_string(),
            format!("buf c7c7c7c7{font}c7c7c7c7"),
            "callee-saved 1".to_string(),
            "rsp 1 ra dead0000".to_string(),
            "args 1".to_string(),
            "last error 1234".to_string(),
            format!("table copy still logfont 1 {font}"),
        ]
    );
}

#[test]
fn each_font_gives_its_own_copy_and_the_table_is_not_changed_by_asking() {
    let l = lines(
        "gdi-getobj-two",
        r#"
    uint8_t b1[92], b2[92], b3[92];
    uint64_t a, p, b;
    unsigned i;
    mklf(0x20); a = MKFONT();
    p = MKPAL();
    mklf(0x90); b = MKFONT();
    printf("delete a %u\n", DEL(a));
    mklf(0x11); { uint64_t c = MKFONT(); (void)c; }
    printf("b %llu\n", U(CALL(fle_imp_GetObjectW, b, 92, (uint64_t)(uintptr_t)b2)));
    printf("b again %llu\n", U(CALL(fle_imp_GetObjectW, b, 0x5c, (uint64_t)(uintptr_t)b3)));
    printf("same %d\n", memcmp(b2, b3, 92) == 0);
    for (i = 0; i < 92; i++) b2[i] = 0xEE;      /* the guest's buffer changes: the table keeps its own copy */
    printf("first byte %02x\n", b3[0]);
    dumpfont(b);
    printf("fonts %llu pal %llu live %llu\n", U(fle_gdi_font_live_count()), U(fle_gdi_palette_live_count()), U(fle_gdi_live_count()));
    (void)b1; (void)p;
"#,
    );
    assert_eq!(
        l,
        [
            "delete a 1".to_string(),
            "b 92".to_string(),
            "b again 92".to_string(),
            "same 1".to_string(),
            "first byte 90".to_string(),
            format!("logfont 1 {}", (0..92u32).map(|i| format!("{:02x}", 0x90u8.wrapping_add(i as u8))).collect::<String>()),
            "fonts 2 pal 1 live 3".to_string(),
        ]
    );
}

#[test]
fn only_the_low_dword_of_rdx_is_c_and_only_92_bytes_are_written() {
    // GetObjectW(HANDLE, int c, LPVOID): the guest sets edx; the upper half of rdx may hold anything. A 92-byte buffer at the very
    // end of a 96-byte array leaves the 4 bytes before it alone; nothing after the 92 bytes is touched.
    let l = lines(
        "gdi-getobj-dword",
        r#"
    uint8_t big[100];
    uint64_t f;
    memset(big, 0xC7, sizeof big);
    mklf(0x55);
    f = MKFONT();
    printf("ret %llu\n", U(CALL(fle_imp_GetObjectW, f, 0xAAAAAAAA0000005Cull, (uint64_t)(uintptr_t)(big + 4))));
    printf("front %02x%02x%02x%02x first %02x last %02x back %02x%02x%02x%02x\n", big[0], big[1], big[2], big[3], big[4], big[95], big[96], big[97], big[98], big[99]);
"#,
    );
    assert_eq!(l, ["ret 92", "front c7c7c7c7 first 55 last b0 back c7c7c7c7"]);
}

#[test]
fn getobjectw_of_every_other_form_stops_with_a_reason_of_its_own() {
    // (label, setup, call, words the message must contain). h is the handle of the form, c and pv are the guest's (0x5c, a buffer).
    let forms: &[(&str, &str, &str, &[&str])] = &[
        ("stock-font", "uint64_t h = STOCK(13);", "CALL(fle_imp_GetObjectW, h, 92, BUF);", &["stock SYSTEM_FONT", "not decided"]),
        ("palette", "uint64_t h = MKPAL();", "CALL(fle_imp_GetObjectW, h, 4, BUF);", &["palette"]),
        ("palette-with-the-font-size", "uint64_t h = MKPAL();", "CALL(fle_imp_GetObjectW, h, 92, BUF);", &["palette"]),
        ("stock-brush", "uint64_t h = STOCK(5);", "CALL(fle_imp_GetObjectW, h, 16, BUF);", &["stock brush"]),
        ("stock-pen", "uint64_t h = STOCK(7);", "CALL(fle_imp_GetObjectW, h, 16, BUF);", &["stock pen"]),
        ("icon", "uint64_t h = ICON();", "CALL(fle_imp_GetObjectW, h, 32, BUF);", &["icon"]),
        ("deleted-font", "uint64_t h; mklf(1); h = MKFONT(); DEL(h);", "CALL(fle_imp_GetObjectW, h, 92, BUF);", &["was deleted"]),
        ("null-handle", "uint64_t h = 0;", "CALL(fle_imp_GetObjectW, h, 92, BUF);", &["was not issued"]),
        ("foreign-small", "uint64_t h = 0x1234;", "CALL(fle_imp_GetObjectW, h, 92, BUF);", &["was not issued"]),
        ("screen-dc", "uint64_t h = CALL(fle_imp_GetDC, 0);", "CALL(fle_imp_GetObjectW, h, 92, BUF);", &["was not issued"]),
        ("never-issued-in-region", "uint64_t h = FLE_GDI_HANDLE_BASE + 4 * 900;", "CALL(fle_imp_GetObjectW, h, 92, BUF);", &["was not issued"]),
        ("region-base", "uint64_t h = FLE_GDI_HANDLE_BASE;", "CALL(fle_imp_GetObjectW, h, 92, BUF);", &["was not issued"]),
        ("unaligned-in-region", "uint64_t h; mklf(1); h = MKFONT() + 1;", "CALL(fle_imp_GetObjectW, h, 92, BUF);", &["was not issued"]),
        ("invalid-handle-value", "uint64_t h = 0xFFFFFFFFFFFFFFFFull;", "CALL(fle_imp_GetObjectW, h, 92, BUF);", &["was not issued"]),
        ("pv-null", "uint64_t h; mklf(1); h = MKFONT();", "CALL(fle_imp_GetObjectW, h, 92, 0);", &["pv is NULL"]),
    ];
    let mut reasons: Vec<(String, String)> = Vec::new();
    for (label, setup, call, words) in forms {
        let body = format!("    uint8_t bufb[96];\n    #define BUF ((uint64_t)(uintptr_t)bufb)\n    {setup}\n    {call}\n    printf(\"returned %llu\\n\", U(R[0]));");
        let o = run(&format!("gdi-getobj-stop-{label}"), &body);
        assert!(!o.ok && !o.stdout.contains("returned"), "{label}: returned instead of stopping: {}", o.stdout);
        assert_eq!(o.signal, Some(6), "{label}: not SIGABRT");
        let msg = o.stderr.lines().find(|l| l.starts_with("fle: GetObjectW:")).unwrap_or_else(|| panic!("{label}: no 'fle: GetObjectW:' message: {}", o.stderr)).to_string();
        for w in *words {
            assert!(msg.contains(w), "{label}: message lacks {w:?}: {msg}");
        }
        reasons.push((label.to_string(), msg));
    }
    // c is not 0x5c: every number the guest's other 25 call sites pass (0x68, 0x20, 0x10, 0x4, 0x400, 0 for the size query), the
    // neighbours of 0x5c, and the extremes. The message names the value (the low DWORD).
    for c in [0u64, 1, 4, 0x10, 0x20, 0x5b, 0x5d, 0x68, 0x400, 0x5c0, 0x1000, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFFF, 0xFFFF_FFFF_0000_0020] {
        let body = format!("    uint8_t bufb[96];\n    uint64_t f; mklf(1); f = MKFONT();\n    CALL(fle_imp_GetObjectW, f, {c:#x}ull, (uint64_t)(uintptr_t)bufb);\n    printf(\"returned\\n\");");
        let o = run(&format!("gdi-getobj-stop-c-{c:x}"), &body);
        assert!(!o.ok && !o.stdout.contains("returned"), "c {c:#x}: returned instead of stopping: {}", o.stdout);
        assert_eq!(o.signal, Some(6), "c {c:#x}");
        assert!(o.stderr.contains(&format!("fle: GetObjectW: c is {:#x}, not sizeof(LOGFONTW) = 0x5c", c as u32)), "c {c:#x}: {}", o.stderr);
        if c == 0x20 {
            reasons.push(("c-not-0x5c".to_string(), o.stderr.lines().find(|l| l.starts_with("fle: GetObjectW:")).unwrap().replace("0x20", "0xN")));
        }
    }
    // the size query of the guest's call at 0x5c1739 (GetObjectW(h, 0, NULL)) stops on its c, which is checked first
    let o = run("gdi-getobj-stop-sizequery", "uint64_t f; mklf(1); f = MKFONT(); CALL(fle_imp_GetObjectW, f, 0, 0); printf(\"returned\\n\");");
    assert!(!o.ok && o.stderr.contains("c is 0x0, not sizeof(LOGFONTW)"), "{}", o.stderr);
    // every form has its own reason: one distinct message per family (the unissued handles share one on purpose, the palette
    // gives the same text for c = 4 and c = 92: the handle is checked before c)
    let mut distinct: Vec<&str> = reasons.iter().map(|(_, m)| m.as_str()).collect();
    distinct.sort();
    distinct.dedup();
    let families = ["stock SYSTEM_FONT", "palette", "stock brush", "stock pen", "icon", "was deleted", "was not issued", "pv is NULL", "not sizeof(LOGFONTW)"];
    assert_eq!(distinct.len(), families.len(), "{distinct:#?}");
    for fam in families {
        assert!(distinct.iter().any(|m| m.contains(fam)), "no message of the family {fam:?}");
    }
}

#[test]
fn nothing_is_stored_before_a_stop() {
    // a SIGABRT handler (fle_stop aborts) reports whether the 96-byte buffer still holds its 0xC7 fill
    let top = r#"
#include <signal.h>
#include <unistd.h>
static uint8_t gbuf[96];
static void onabort(int s) { unsigned i; char m[2] = { '1', '\n' }; (void)s; for (i = 0; i < sizeof gbuf; i++) if (gbuf[i] != 0xC7) m[0] = '0'; write(1, m, 2); _exit(0); }
"#;
    let cases: &[(&str, &str)] = &[
        ("c-0x20", "uint64_t h; mklf(1); h = MKFONT(); CALL(fle_imp_GetObjectW, h, 0x20, (uint64_t)(uintptr_t)gbuf);"),
        ("c-0x68", "uint64_t h; mklf(1); h = MKFONT(); CALL(fle_imp_GetObjectW, h, 0x68, (uint64_t)(uintptr_t)gbuf);"),
        ("palette", "uint64_t h = MKPAL(); CALL(fle_imp_GetObjectW, h, 92, (uint64_t)(uintptr_t)gbuf);"),
        ("stock-font", "uint64_t h = STOCK(13); CALL(fle_imp_GetObjectW, h, 92, (uint64_t)(uintptr_t)gbuf);"),
        ("deleted", "uint64_t h; mklf(1); h = MKFONT(); DEL(h); CALL(fle_imp_GetObjectW, h, 92, (uint64_t)(uintptr_t)gbuf);"),
        ("unissued", "CALL(fle_imp_GetObjectW, 0x1234, 92, (uint64_t)(uintptr_t)gbuf);"),
    ];
    for (label, stmts) in cases {
        let o = run_src(&format!("gdi-getobj-untouched-{label}"), top, &format!("    signal(SIGABRT, onabort);\n    memset(gbuf, 0xC7, sizeof gbuf);\n    {stmts}\n    printf(\"not reached\\n\");"));
        assert!(o.ok, "{label}: the handler did not run: {:?} {}", o.signal, o.stderr);
        assert_eq!(o.stdout, "1\n", "{label}: the buffer was written before the stop");
        assert!(o.stderr.contains("fle: GetObjectW:"), "{label}: {}", o.stderr);
    }
}

#[test]
fn getobjectw_adds_no_new_state_and_leaves_the_other_apis_alone() {
    // asking never moves the counts, and a DeleteObject of the asked font still works once, then the font is a stop
    let l = lines(
        "gdi-getobj-state",
        r#"
    uint8_t b[92];
    uint64_t f, p;
    mklf(0x44); f = MKFONT(); p = MKPAL();
    printf("before live %llu fonts %llu pal %llu\n", U(fle_gdi_live_count()), U(fle_gdi_font_live_count()), U(fle_gdi_palette_live_count()));
    CALL(fle_imp_GetObjectW, f, 92, (uint64_t)(uintptr_t)b);
    CALL(fle_imp_GetObjectW, f, 92, (uint64_t)(uintptr_t)b);
    printf("after  live %llu fonts %llu pal %llu kind %d\n", U(fle_gdi_live_count()), U(fle_gdi_font_live_count()), U(fle_gdi_palette_live_count()), fle_gdi_kind(f));
    printf("delete %u again %u\n", DEL(f), DEL(f));
    printf("palette %u entries\n", (unsigned)fle_gdi_palette_count(p));
"#,
    );
    assert_eq!(l, ["before live 2 fonts 1 pal 1", "after  live 2 fonts 1 pal 1 kind 6", "delete 1 again 0", "palette 16 entries"]);
    assert_aborts(
        "gdi-abort-getobj-after-delete",
        "uint64_t f; mklf(1); f = MKFONT(); uint8_t b[96]; CALL(fle_imp_DeleteObject, f);",
        "GetObjectW",
        &["was deleted"],
        &[("deleted", "CALL(fle_imp_GetObjectW, f, 92, (uint64_t)(uintptr_t)b);")],
    );
}

// ---- 10b. Bitmaps: CreateBitmap (decision 0106, PROVISIONAL) ----
//
// Learn CreateBitmap (read 2026-10-27): "creates a bitmap with the specified width, height, and color format (color planes and
// bits-per-pixel)"; lpBits "If this parameter is NULL, then the contents of the new bitmap are undefined"; the return value is a
// handle to a bitmap, NULL on failure; "If an application sets the nWidth or nHeight parameters to zero, CreateBitmap returns the handle
// to a 1-by-1 pixel, monochrome bitmap"; "When you no longer need the bitmap, call the DeleteObject function". The handle value and
// "every call a new handle" are choices of the decision (the same as for palettes and fonts). The pixels are not kept.

const BMP_TOP: &str = r#"
static uint64_t cb(uint64_t w, uint64_t h, uint64_t pl, uint64_t bp, uint64_t bits)
{
    memset(stk, 0, sizeof stk);
    stk[0] = 0xdead0000u;
    stk[5] = bits;                       /* [rsp+0x28]: the fifth argument, lpBits */
    R[1] = w; R[2] = h; R[8] = pl; R[9] = bp;
    R[4] = (uint64_t)(uintptr_t)stk;
    R[0] = 0xBADBADBADull;
    fle_imp_CreateBitmap();
    return R[0];
}
static void dumpbmp(uint64_t h)
{
    uint32_t o[4] = { 7, 7, 7, 7 };
    int k = fle_gdi_bitmap_info(h, o);
    printf("info %d %u %u %u %u\n", k, o[0], o[1], o[2], o[3]);
}
"#;

#[test]
fn a_bitmap_gets_a_handle_in_the_gdi_region_and_the_table_keeps_its_size_and_format() {
    let o = run_src(
        "gdi-bitmap",
        BMP_TOP,
        r#"
    uint64_t b, c;
    SETLE(0x1234);
    b = cb(32, 24, 1, 1, 0);
    printf("region %d aligned %d kind %d\n", INREG(b), (b % 4) == 0, fle_gdi_kind(b));
    printf("sign-extends %d\n", (uint64_t)(int64_t)(int32_t)(uint32_t)b == b);
    dumpbmp(b);
    printf("live %llu bitmaps %llu fonts %llu pal %llu\n", U(fle_gdi_live_count()), U(fle_gdi_bitmap_live_count()), U(fle_gdi_font_live_count()), U(fle_gdi_palette_live_count()));
    c = cb(32, 24, 1, 1, 0);
    printf("second differs %d slot %llu then %llu\n", c != b, U((b - FLE_GDI_HANDLE_BASE) / 4 - 1), U((c - FLE_GDI_HANDLE_BASE) / 4 - 1));
    printf("last error %u\n", LE());
"#,
    );
    assert!(o.ok, "{}\n{}", o.stderr, o.stdout);
    let l: Vec<&str> = o.stdout.lines().collect();
    // the first created slot is 33 (fle_gdi.h: stock 0..31, icon 32, created from 33); a bitmap shares the sequence with palettes and fonts
    assert_eq!(
        l,
        ["region 1 aligned 1 kind 7", "sign-extends 1", "info 1 32 24 1 1", "live 1 bitmaps 1 fonts 0 pal 0", "second differs 1 slot 33 then 34", "last error 4660"]
    );
}

#[test]
fn a_zero_width_or_height_gives_the_one_by_one_monochrome_bitmap_of_the_page() {
    let o = run_src(
        "gdi-bitmap-zero",
        BMP_TOP,
        r#"
    dumpbmp(cb(0, 0, 1, 1, 0));
    dumpbmp(cb(0, 16, 1, 1, 0));
    dumpbmp(cb(16, 0, 1, 1, 0));
    dumpbmp(cb(1, 1, 1, 1, 0));
"#,
    );
    assert!(o.ok, "{}\n{}", o.stderr, o.stdout);
    let l: Vec<&str> = o.stdout.lines().collect();
    assert_eq!(l, ["info 1 1 1 1 1", "info 1 1 1 1 1", "info 1 1 1 1 1", "info 1 1 1 1 1"]);
}

#[test]
fn only_the_low_dword_of_each_argument_counts_and_the_largest_size_below_two_gib_is_accepted() {
    let o = run_src(
        "gdi-bitmap-low",
        BMP_TOP,
        r#"
    /* int, int, UINT, UINT: the upper 32 bits of the registers are not part of the arguments */
    dumpbmp(cb(0xAAAAAAAA00000010ull, 0xBBBBBBBB00000008ull, 0xCCCCCCCC00000001ull, 0xDDDDDDDD00000001ull, 0));
    /* cj = (((w * 1 * 1 + 15) >> 4) << 1) * h = 0x0FFFFFFE * h for w = 0x7FFFFFF0: h = 8 gives 0x7FFFFFF0 (below 2^31), h = 9 gives 0x8FFFFFF2 */
    dumpbmp(cb(0x7FFFFFF0, 8, 1, 1, 0));
"#,
    );
    assert!(o.ok, "{}\n{}", o.stderr, o.stdout);
    let l: Vec<&str> = o.stdout.lines().collect();
    assert_eq!(l, ["info 1 16 8 1 1", "info 1 2147483632 8 1 1"]);
}

#[test]
fn delete_object_of_a_bitmap_follows_the_page() {
    let o = run_src(
        "gdi-bitmap-delete",
        BMP_TOP,
        r#"
    uint64_t b = cb(8, 8, 1, 1, 0), p;
    printf("before live %llu bitmaps %llu kind %d\n", U(fle_gdi_live_count()), U(fle_gdi_bitmap_live_count()), fle_gdi_kind(b));
    printf("delete %u again %u\n", DEL(b), DEL(b));
    printf("after live %llu bitmaps %llu kind %d\n", U(fle_gdi_live_count()), U(fle_gdi_bitmap_live_count()), fle_gdi_kind(b));
    dumpbmp(b);
    p = cb(8, 8, 1, 1, 0);
    printf("never reused %d\n", p != b);
"#,
    );
    assert!(o.ok, "{}\n{}", o.stderr, o.stdout);
    let l: Vec<&str> = o.stdout.lines().collect();
    assert_eq!(l, ["before live 1 bitmaps 1 kind 7", "delete 1 again 0", "after live 0 bitmaps 0 kind 0", "info 0 7 7 7 7", "never reused 1"]);
}

#[test]
fn a_bitmap_form_the_guest_does_not_use_stops_by_name_and_creates_nothing() {
    // (label, call, words of the message)
    let forms: &[(&str, &str, &[&str])] = &[
        ("negative-width", "cb(0xFFFFFFFFu, 8, 1, 1, 0)", &["negative"]),
        ("negative-height", "cb(8, 0x80000000u, 1, 1, 0)", &["negative"]),
        ("colour", "cb(8, 8, 1, 32, 0)", &["nPlanes and nBitCount"]),
        ("two-planes", "cb(8, 8, 2, 1, 0)", &["nPlanes and nBitCount"]),
        ("zero-format", "cb(8, 8, 0, 0, 0)", &["nPlanes and nBitCount"]),
        ("two-gib", "cb(0x7FFFFFF0, 9, 1, 1, 0)", &["2 GiB"]),
    ];
    for (label, call, words) in forms {
        let o = run_src("gdi-bitmap-stop", BMP_TOP, &format!("    {call};\n    printf(\"returned %llu\\n\", U(R[0]));"));
        assert!(!o.ok && !o.stdout.contains("returned"), "{label}: returned instead of stopping: {}", o.stdout);
        assert_eq!(o.signal, Some(6), "{label}: not SIGABRT");
        let msg = o.stderr.lines().find(|l| l.starts_with("fle: CreateBitmap:")).unwrap_or_else(|| panic!("{label}: no 'fle: CreateBitmap:' message: {}", o.stderr)).to_string();
        for w in *words {
            assert!(msg.contains(w), "{label}: message lacks {w:?}: {msg}");
        }
    }
}

#[test]
fn createbitmap_with_initial_bits_keeps_the_word_aligned_rows_of_the_page() {
    // decision 0132 (wall 18; old: lpBits != NULL stopped, "the guest passes NULL"). Learn CreateBitmap: lpBits "A pointer to an array of color data used to set the colors in a rectangle of pixels. Each scan line in the
    // rectangle must be word aligned (scan lines that are not word aligned must be padded with zeros). ... cj = (((nWidth * nPlanes * nBitCount + 15) >> 4) << 1) * nHeight". The bitmap is the monochrome one of this runtime;
    // the bytes are read back with GetBitmapBits (the same word-aligned layout). The 8-by-8 case is the guest's (the pattern bitmap of the Delphi graphics unit, args 8 8 1 1 and a pointer); 10 x 3 shows the padding
    // (2 bytes per row for 10 pixels: the second byte holds pixels 8 and 9 in its two most significant bits).
    let o = run_src(
        "gdi-bitmap-bits",
        BMP_TOP,
        r#"
    static uint8_t pat[16] = { 0xAA, 0, 0x55, 0, 0xAA, 0, 0x55, 0, 0xAA, 0, 0x55, 0, 0xAA, 0, 0x55, 0 };
    static uint8_t out[16];
    static uint8_t pat10[6] = { 0xFF, 0xC0, 0x00, 0x80, 0xFF, 0x40 };
    static uint8_t out10[6];
    uint64_t b, c;
    uint64_t i;
    b = cb(8, 8, 1, 1, (uint64_t)(uintptr_t)pat);
    R[1] = b; R[2] = 16; R[8] = (uint64_t)(uintptr_t)out; R[4] = (uint64_t)(uintptr_t)stk; stk[0] = 0xdead0000u;
    fle_imp_GetBitmapBits();
    printf("ret %llu\n", U(R[0]));
    for (i = 0; i < 16; i++) printf("%02x", out[i]);
    printf("\n");
    c = cb(10, 3, 1, 1, (uint64_t)(uintptr_t)pat10);
    R[1] = c; R[2] = 6; R[8] = (uint64_t)(uintptr_t)out10; R[4] = (uint64_t)(uintptr_t)stk; stk[0] = 0xdead0000u;
    fle_imp_GetBitmapBits();
    printf("ret %llu\n", U(R[0]));
    for (i = 0; i < 6; i++) printf("%02x", out10[i]);
    printf("\n");
    /* a zero size is the 1-by-1 bitmap and takes no bits (the pointer is not read) */
    b = cb(0, 5, 1, 1, 0x1000);
    dumpbmp(b);
"#,
    );
    assert!(o.ok, "{}\n{}", o.stderr, o.stdout);
    let l: Vec<&str> = o.stdout.lines().collect();
    assert_eq!(l, ["ret 16", "aa005500aa005500aa005500aa005500", "ret 6", "ffc00080ff40", "info 1 1 1 1 1"]);
}

#[test]
fn createpatternbrush_makes_a_brush_of_the_8x8_monochrome_bitmap_and_the_bitmap_stays_the_guests() {
    // decision 0132 (wall 19, PROVISIONAL). Learn (CreatePatternBrush): "creates a logical brush with the specified bitmap pattern"; the return value "identifies a logical brush"; "it can select that brush into any
    // device context by calling the SelectObject function"; "You can delete a pattern brush without affecting the associated bitmap by using the DeleteObject function". The guest's bitmap is the 8-by-8 monochrome
    // one of CreateBitmap with initial bits (wall 18). Every call makes a new handle (the choice of decisions 0106 and 0108); the brush counts as a brush (kind of CreateBrushIndirect).
    let l = w6_lines(
        "w6-patternbrush",
        r#"
    static uint8_t pat[16] = { 0xAA, 0, 0x55, 0, 0xAA, 0, 0x55, 0, 0xAA, 0, 0x55, 0, 0xAA, 0, 0x55, 0 };
    uint64_t bm = cN(fle_imp_CreateBitmap, 8, 8, 1, 1, (uint64_t)(uintptr_t)pat, 0, 0, 0, 0), solid = mkbrush(0, 1, 0), br, br2, dc = MKDC();
    br = C4(fle_imp_CreatePatternBrush, bm, 0, 0, 0);
    br2 = C4(fle_imp_CreatePatternBrush, bm, 0, 0, 0);
    printf("brush %d same kind as a solid brush %d distinct %d\n", br != 0, KIND(br) == KIND(solid), br != br2);
    SELO(dc, br);
    printf("selected %u\n", SELN(br));
    SELO(dc, solid);
    printf("deleted %u\n", (uint32_t)DEL(br));
    printf("the bitmap is still a bitmap: %d\n", KIND(bm) == 7);
    printf("deleted again %u\n", (uint32_t)DEL(br));
"#,
    );
    assert_eq!(l, ["brush 1 same kind as a solid brush 1 distinct 1", "selected 1", "deleted 1", "the bitmap is still a bitmap: 1", "deleted again 0"]);
    // the forms the guest does not use stop by name; a fill with the pattern brush stops by name (no pixel of a pattern fill is written)
    stops_with(
        "w6-patternbrush-stops",
        "static uint8_t pat[16] = { 0xAA, 0, 0x55, 0, 0xAA, 0, 0x55, 0, 0xAA, 0, 0x55, 0, 0xAA, 0, 0x55, 0 };\n    uint64_t bm = cN(fle_imp_CreateBitmap, 8, 8, 1, 1, (uint64_t)(uintptr_t)pat, 0, 0, 0, 0), big = MKBMP(9, 8), sdc = SCR(), col = CCB(sdc, 8, 8), br = C4(fle_imp_CreatePatternBrush, bm, 0, 0, 0), dc = MKDC();\n    SELO(dc, br);\n    (void)col; (void)big;",
        "CreatePatternBrush",
        &[
            ("null", "C4(fle_imp_CreatePatternBrush, 0, 0, 0, 0);", "hbm is not a live bitmap"),
            ("size", "C4(fle_imp_CreatePatternBrush, big, 0, 0, 0);", "not 8 by 8 pixels"),
            ("colour", "C4(fle_imp_CreatePatternBrush, col, 0, 0, 0);", "not 8 by 8 pixels, 1 plane, 1 bit"),
            ("a-brush", "C4(fle_imp_CreatePatternBrush, br, 0, 0, 0);", "hbm is not a live bitmap"),
        ],
    );
}

#[test]
fn a_fill_with_a_pattern_brush_stops_by_name_for_want_of_the_pixels_of_a_pattern_fill() {
    stops_with(
        "w6-patternbrush-fill",
        "static uint8_t pat[16] = { 0xAA, 0, 0x55, 0, 0xAA, 0, 0x55, 0, 0xAA, 0, 0x55, 0, 0xAA, 0, 0x55, 0 };\n    uint64_t bm = cN(fle_imp_CreateBitmap, 8, 8, 1, 1, (uint64_t)(uintptr_t)pat, 0, 0, 0, 0), br = C4(fle_imp_CreatePatternBrush, bm, 0, 0, 0), sdc = SCR(), mdc = MKDC(), t = CCB(sdc, 16, 16);\n    static uint32_t rc[4] = { 0, 0, 8, 8 };\n    SELO(mdc, t);\n    SELO(mdc, br);",
        "FillRect",
        &[("fillrect", "C4(fle_imp_FillRect, mdc, A16(rc), br, 0);", "pattern brush")],
    );
}

#[test]
fn getobjectw_of_a_bitmap_stops_with_a_reason_of_its_own() {
    let o = run_src(
        "gdi-bitmap-getobject",
        BMP_TOP,
        "    uint8_t bufb[96];\n    uint64_t b = cb(8, 8, 1, 1, 0);\n    CALL(fle_imp_GetObjectW, b, 24, (uint64_t)(uintptr_t)bufb);\n    printf(\"returned\\n\");",
    );
    assert!(!o.ok && !o.stdout.contains("returned"), "{}", o.stdout);
    assert_eq!(o.signal, Some(6));
    assert!(o.stderr.contains("fle: GetObjectW:") && o.stderr.contains("handle is a bitmap"), "{}", o.stderr);
}

// ---- 11. The bytes of the real DLL that the decision rests on (decision 0068, docs/investigations/getobjectw-caller.md) ----

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
/// (virtual size, rva, raw size, raw offset) of every section (the section table is read here, not through fe-pe).
fn sections(dll: &[u8]) -> Vec<(u32, u32, u32, u32)> {
    let lf = le32(dll, 0x3c) as usize;
    let nsec = le16(dll, lf + 6) as usize;
    let osz = le16(dll, lf + 20) as usize;
    (0..nsec).map(|i| lf + 24 + osz + 40 * i).map(|o| (le32(dll, o + 8), le32(dll, o + 12), le32(dll, o + 16), le32(dll, o + 20))).collect()
}
fn rva_off(dll: &[u8], rva: u32) -> usize {
    for (vs, sva, rs, rp) in sections(dll) {
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
fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}
/// Every import of the DLL as (dll name, function name, IAT slot VA).
fn imports(dll: &[u8]) -> Vec<(String, String, u64)> {
    let lf = le32(dll, 0x3c) as usize;
    let cstr = |rva: u32| -> String {
        let o = rva_off(dll, rva);
        let e = dll[o..].iter().position(|&c| c == 0).unwrap();
        String::from_utf8_lossy(&dll[o..o + e]).to_string()
    };
    let mut v = Vec::new();
    let mut d = rva_off(dll, le32(dll, lf + 24 + 112 + 8));
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

const GETOBJ_THUNK: u64 = 0x424280;
const GETOBJ_SLOT: u64 = 0x19719f0;

#[test]
fn the_thunk_jumps_to_the_slot_which_is_gdi32_getobjectw_and_nothing_else_points_at_either() {
    let Some(dll) = real_dll() else { return };
    let imps = imports(&dll);
    let named: Vec<_> = imps.iter().filter(|(_, _, s)| *s == GETOBJ_SLOT).collect();
    assert_eq!(named.len(), 1);
    assert_eq!((named[0].0.to_lowercase(), named[0].1.as_str()), ("gdi32.dll".to_string(), "GetObjectW"));
    let t = bytes_at(&dll, GETOBJ_THUNK, 6);
    assert_eq!(&t[..2], [0xff, 0x25], "jmp [rip+disp32]");
    assert_eq!((GETOBJ_THUNK as i64 + 6 + i32::from_le_bytes(t[2..6].try_into().unwrap()) as i64) as u64, GETOBJ_SLOT);
    // the guest imports no other GetObject: the A form and the ANSI/other spellings are not in its table
    let others: Vec<_> = imps.iter().filter(|(_, n, _)| n.contains("GetObject")).collect();
    assert_eq!(others.len(), 1, "{others:?}");
    // no 4- or 8-byte absolute VA (or the 4-byte RVA) of the thunk or the slot is anywhere in the file: no table of function
    // pointers holds them, so the direct call sites below are all the references (an indirect call through a computed address is
    // not seen)
    for va in [GETOBJ_THUNK, GETOBJ_SLOT] {
        for pat in [(va as u32).to_le_bytes().to_vec(), va.to_le_bytes().to_vec(), ((va - 0x400000) as u32).to_le_bytes().to_vec()] {
            assert!(!dll.windows(pat.len()).any(|w| w == pat.as_slice()), "{va:#x}: {pat:02x?}");
        }
    }
}

/// The 26 call sites (the address of the `call rel32` that targets the thunk), the c they pass, and the 16 bytes before the call.
/// c is `mov edx,imm32` (ba imm32) in 24 sites, `xor edx,edx` (33 d2) at 0x5c1739 (with `xor r8,r8`: the size query, c = 0 and
/// pv = NULL), and `mov edx,[rbp+0x44]` (8b 55 44) at 0x5c17dd (a variable).
const GETOBJ_SITES: [(u64, &str, &str); 26] = [
    (0x5c0c0f, "0x5c", "488b4d30ba5c0000004c8d8544010000"),
    (0x5c1739, "0 (xor edx,edx; r8 = 0)", "83010000488b8da800000033d24d33c0"),
    (0x5c1757, "0x10", "488b8da8000000ba100000004c8d4568"),
    (0x5c17dd, "variable [rbp+0x44]", "7890488b8da80000008b55444c8b4578"),
    (0x5c185e, "0x20", "488b8da8000000ba200000004c8d4548"),
    (0x5c1de4, "0x10", "85ed74794c89e9ba100000004c8d4520"),
    (0x5c4504, "0x20", "488b8de0000000ba200000004c8d4578"),
    (0x5c511c, "0x4", "85db744a4889d9ba040000004c8d452c"),
    (0x5c5893, "0x20", "c0000000ba200000004c8d8580000000"),
    (0x5c58ab, "0x20", "c8000000ba200000004c8d85a0000000"),
    (0x5c5b26, "0x68", "c7454800000000ba680000004c8d4528"),
    (0x5cb2e8, "0x68", "80010000ba680000004c8d85a8000000"),
    (0x5cbafe, "0x68", "20010000ba680000004c8b8598010000"),
    (0x5cbc23, "0x68", "20010000ba680000004c8b8398010000"),
    (0x5cbc60, "0x4", "004889d9ba040000004c8d852c040000"),
    (0x5cbd0d, "0x68", "488b8d10010000ba680000004c8d4578"),
    (0x5cd74f, "0x68", "007543488b4b10ba680000004c8d433c"),
    (0x5cd96a, "0x68", "488b8db0000000ba680000004c8d4548"),
    (0x5cdc03, "0x68", "19f6e3ff4889f1ba680000004c8d4528"),
    (0x5ce5d5, "0x68", "488b8d18010000ba680000004c8d4578"),
    (0x5ce8e9, "0x68", "488b8de8000000ba680000004c8d4550"),
    (0x5d043d, "0x20", "744190488b4d68ba200000004c8d4530"),
    (0x6b6800, "0x20", "2c4889d9ba200000004c8d8540040000"),
    (0x6ba762, "0x400", "004889c1ba000400004c8d8550080000"),
    (0x98b4f6, "0x20", "00010000ba200000004c8d85b0000000"),
    (0x9f248d, "0x68", "85c074244889c1ba680000004c8d4528"),
];

#[test]
fn the_26_call_sites_are_exactly_these_and_only_one_passes_c_0x5c() {
    let Some(dll) = real_dll() else { return };
    // every `call rel32` in .text whose target is the thunk (a scan of the E8 bytes: it also sees an E8 that is not an
    // instruction, none exists today)
    let (vs, sva, _, rp) = sections(&dll)[0];
    let text = &dll[rp as usize..rp as usize + vs as usize];
    let mut found = Vec::new();
    for i in 0..text.len().saturating_sub(5) {
        if text[i] == 0xe8 {
            let rel = i32::from_le_bytes(text[i + 1..i + 5].try_into().unwrap()) as i64;
            let va = 0x400000 + sva as u64 + i as u64;
            if (va as i64 + 5 + rel) as u64 == GETOBJ_THUNK {
                found.push(va);
            }
        }
    }
    assert_eq!(found, GETOBJ_SITES.iter().map(|s| s.0).collect::<Vec<_>>());
    for (site, c, pre) in GETOBJ_SITES {
        assert_eq!(hex(&bytes_at(&dll, site - 16, 16)), pre, "{site:#x} (c = {c})");
    }
    // the count of each c: the decision answers 0x5c (one site) and stops on all the rest (25 sites)
    let count = |s: &str| GETOBJ_SITES.iter().filter(|x| x.1 == s).count();
    assert_eq!((count("0x5c"), count("0x68"), count("0x20"), count("0x10"), count("0x4"), count("0x400")), (1, 11, 7, 2, 2, 1));
    assert_eq!(count("0 (xor edx,edx; r8 = 0)") + count("variable [rbp+0x44]"), 2);
    // the sizes: sizeof(LOGFONTW) = 5 x 4 + 8 + 32 x 2 = 92 = 0x5c, the c of the one answered site
    assert_eq!(5 * 4 + 8 + 32 * 2, 0x5c);
}

#[test]
fn f_5c0bc0_asks_for_0x5c_bytes_and_reads_only_these_fields_of_the_logfont() {
    let Some(dll) = real_dll() else { return };
    // lf is [rbp+0x144]: lfHeight +0, lfOrientation +0xc (0x150), lfWeight +0x10 (0x154), lfItalic +0x14 (0x158),
    // lfUnderline +0x15, lfStrikeOut +0x16, lfCharSet +0x17 (0x15b), lfPitchAndFamily +0x1b (0x15f), lfFaceName +0x1c (0x160)
    let lf = 0x144u32;
    assert_eq!([0x150 - lf, 0x154 - lf, 0x158 - lf, 0x159 - lf, 0x15a - lf, 0x15b - lf, 0x15f - lf, 0x160 - lf], [0xc, 0x10, 0x14, 0x15, 0x16, 0x17, 0x1b, 0x1c]);
    let at = |va: u64, n: usize| hex(&bytes_at(&dll, va, n));
    // hfont NULL: skip (test rax,rax; je 0x5c0d00); then GetObjectW(hfont, 0x5c, &lf); result 0: skip (test eax,eax; je 0x5c0d00)
    assert_eq!(at(0x5c0bf2, 13), "488b45304885c00f8401010000");
    assert_eq!(at(0x5c0bff, 30), "488b4d30ba5c0000004c8d8544010000e86c36e6ff85c00f84e40000008b");
    // lfHeight -> [obj+8]; lfWeight >= 0x2bc (700) -> bold flag [obj+0x11] |= 1; italic / underline / strikeout == 1 -> |= 2 / 4 / 8
    assert_eq!(at(0x5c0c1c, 13), "8b8544010000488b4d28894108");
    assert_eq!(at(0x5c0c29, 12), "81bd54010000bc0200007c08");
    assert_eq!(at(0x5c0c35, 8), "488b452880481101");
    assert_eq!(at(0x5c0c3d, 7), "80bd5801000001");
    assert_eq!(at(0x5c0c4e, 7), "80bd5901000001");
    assert_eq!(at(0x5c0c5f, 7), "80bd5a01000001");
    // lfCharSet -> [obj+0x12]; lfFaceName (0x20 units) -> f_414940 / f_41c570 / f_413940 -> [obj+0x13]
    assert_eq!(at(0x5c0c70, 15), "480fb6855b010000488b4d28884112");
    assert_eq!(at(0x5c0c7f, 22), "488d4d38488d956001000041b820000000e8ab3ce5ff");
    // lfPitchAndFamily & 0xf: 1 (FIXED_PITCH) -> [obj+0x10] = 2, 2 (VARIABLE_PITCH) -> 1, anything else (0 included) -> 0
    assert_eq!(at(0x5c0cb6, 22), "480fb6855f010000240f2c0184c074102c0184c07514");
    assert_eq!(at(0x5c0ccc, 10), "488b4528c6401001eb12");
    assert_eq!(at(0x5c0cd6, 10), "488b4528c6401002eb08");
    assert_eq!(at(0x5c0ce0, 8), "488b4528c6401000");
    // lfOrientation -> [obj+0xc]; lfWidth, lfEscapement, lfOutPrecision, lfClipPrecision and lfQuality are not read in this function
    assert_eq!(at(0x5c0cf3, 13), "8b8550010000488b4d2889410c");
    let code = bytes_at(&dll, 0x5c0bc0, 0x5c0d19 - 0x5c0bc0);
    for off in [0x148u32, 0x14c, 0x15c, 0x15d, 0x15e] {
        // [rbp+0x148] lfWidth, [rbp+0x14c] lfEscapement, [rbp+0x15c] lfOutPrecision, [rbp+0x15d] lfClipPrecision, [rbp+0x15e] lfQuality
        let pat = off.to_le_bytes();
        assert!(!code.windows(4).any(|w| w == pat), "[rbp+{off:#x}] is read in f_5c0bc0");
    }
}

// ---- 7. Source checks ----

#[test]
fn source_uses_no_host_display_resource_clock_thread_process_or_file() {
    let code = strip_comments(&std::fs::read_to_string(src_dir().join("fle_gdi.c")).unwrap());
    for bad in [
        "getpid", "getppid", "gettid", "getenv", "environ", "fopen", "open(", "stat(", "/proc", "/etc", "/home", "/dev", "/tmp", "exit(", "_exit",
        "clock_gettime", "gettimeofday", "time(", "clock(", "nanosleep", "usleep", "sleep(", "select(", "poll(", "<time.h>", "<sys/time.h>",
        "pthread", "<sys/", "<unistd.h>", "<signal.h>", "fork(", "exec", "syscall", "sched_", "futex", "asm", "__asm__", "uintptr_t", "abort(",
        "X11", "DISPLAY", "xrandr", "wayland", "fb0", "ioctl", "emscripten", "SDL", "dlopen", "LoadLibrary", "fle_load", "fle_res",
    ] {
        assert!(!code.contains(bad), "fle_gdi.c must not use {bad}");
    }
    // `long` is not portable (32 bits on wasm32 and on Windows, 64 on Linux): fixed-width integers only
    let b = code.as_bytes();
    let mut i = 0;
    while let Some(p) = code[i..].find("long") {
        let at = i + p;
        let before = at == 0 || !(b[at - 1].is_ascii_alphanumeric() || b[at - 1] == b'_');
        let after = at + 4 >= b.len() || !(b[at + 4].is_ascii_alphanumeric() || b[at + 4] == b'_');
        assert!(!(before && after), "fle_gdi.c uses `long` near byte {at}");
        i = at + 4;
    }
    // the host thread id comes from the portability layer (decision 0045), and fle_stop is the stop
    assert!(code.contains("fle_host_tid()") && code.contains("fle_stop("));
    assert!(code.contains("#include \"fle_host.h\""));
}

#[test]
fn header_is_plain_c() {
    let h = strip_comments(&std::fs::read_to_string(src_dir().join("fle_gdi.h")).unwrap());
    for bad in ["asm", "__asm__", "<unistd.h>", "<sys/", "<pthread.h>", "<time.h>", "long "] {
        assert!(!h.contains(bad), "fle_gdi.h must not contain {bad}");
    }
}

// ---- 8. Generator: the four thunks call the runtime ----

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

/// A PE that imports user32.dll!LoadIconW (slot 0x2200) and gdi32.dll!CreatePalette, DeleteObject, GetStockObject (slots 0x2240,
/// 0x2248, 0x2250); `gdi_dll` replaces the DLL name of the three gdi32 imports. Entry 0x1000 calls the thunks at 0x1040 (LoadIconW),
/// 0x1050 (CreatePalette), 0x1060 (DeleteObject), 0x1070 (GetStockObject).
fn build(gdi_dll: &str) -> Vec<u8> {
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

    // Descriptor 1: user32.dll (OFT 0x2100, Name 0x2300, FirstThunk 0x2200). Descriptor 2: the gdi DLL (OFT 0x2140, Name 0x2320,
    // FirstThunk 0x2240).
    let d = data_at(DATA_RVA);
    w32(&mut b, d, 0x2100);
    w32(&mut b, d + 12, 0x2300);
    w32(&mut b, d + 16, 0x2200);
    w32(&mut b, d + 20, 0x2140);
    w32(&mut b, d + 32, 0x2320);
    w32(&mut b, d + 36, 0x2240);
    put(&mut b, 0x2300, b"user32.dll\0");
    let mut gd = gdi_dll.as_bytes().to_vec();
    gd.push(0);
    put(&mut b, 0x2320, &gd);
    put(&mut b, 0x2400, b"\0\0LoadIconW\0");
    put(&mut b, 0x2420, b"\0\0CreatePalette\0");
    put(&mut b, 0x2440, b"\0\0DeleteObject\0");
    put(&mut b, 0x2460, b"\0\0GetStockObject\0");
    for (rva, v) in [
        (0x2100, 0x2400u64),
        (0x2200, 0x2400),
        (0x2140, 0x2420),
        (0x2148, 0x2440),
        (0x2150, 0x2460),
        (0x2240, 0x2420),
        (0x2248, 0x2440),
        (0x2250, 0x2460),
    ] {
        w64(&mut b, data_at(rva), v);
    }

    let t = TEXT_RAW;
    let code: &[(u32, Vec<u8>)] = &[
        (0x1000, [&[0xe8][..], &rel(0x1005, 0x1040)].concat()),
        (0x1005, [&[0xe8][..], &rel(0x100a, 0x1050)].concat()),
        (0x100a, [&[0xe8][..], &rel(0x100f, 0x1060)].concat()),
        (0x100f, [&[0xe8][..], &rel(0x1014, 0x1070)].concat()),
        (0x1014, vec![0xc3]),
        (0x1040, [&[0xff, 0x25][..], &rel(0x1046, 0x2200)].concat()),
        (0x1050, [&[0xff, 0x25][..], &rel(0x1056, 0x2240)].concat()),
        (0x1060, [&[0xff, 0x25][..], &rel(0x1066, 0x2248)].concat()),
        (0x1070, [&[0xff, 0x25][..], &rel(0x1076, 0x2250)].concat()),
    ];
    for (rva, bytes) in code {
        let o = t + (rva - TEXT_RVA) as usize;
        b[o..o + bytes.len()].copy_from_slice(bytes);
    }
    b
}

#[test]
fn the_four_thunks_call_the_runtime() {
    let img = build("gdi32.dll");
    let mut out = Vec::new();
    let st = generate(&img, &mut out).unwrap();
    let c = String::from_utf8(out).unwrap();

    for name in ["CreatePalette", "DeleteObject", "GetStockObject", "LoadIconW"] {
        assert_eq!(st.handled[format!("jmp_imp:{name}").as_str()], 1, "{name}");
        assert!(c.contains(&format!("fle_imp_{name}(); return;")), "{name}");
    }
    for va in [IB + 0x1040, IB + 0x1050, IB + 0x1060, IB + 0x1070] {
        assert!(!c.contains(&format!("L_{va:x}: {{ /* {va:#x} Jmp */ fle_unsupported")), "{va:#x} still unsupported");
    }
    assert_eq!(st.unsupported_total, 0, "{:?}", st.unsupported);
}

/// A PE that imports gdi32.dll!`api` (slot 0x2200); entry 0x1000 calls the thunk at 0x1040.
fn build_one(dll: &str, api: &str) -> Vec<u8> {
    let mut b = build("gdi32.dll");
    // rewrite descriptor 1 to import `dll`!`api` only: name at 0x2300, hint/name at 0x2400, slot 0x2200 (the second descriptor, the
    // one for the gdi names, is cut by zeroing it, which makes it the terminator)
    let d = data_at(DATA_RVA);
    for i in 20..40 {
        b[d + i] = 0;
    }
    put(&mut b, 0x2300, &[0u8; 32]);
    let mut nm = dll.as_bytes().to_vec();
    nm.push(0);
    put(&mut b, 0x2300, &nm);
    let mut hn = vec![0u8, 0u8];
    hn.extend_from_slice(api.as_bytes());
    hn.push(0);
    put(&mut b, 0x2400, &[0u8; 32]);
    put(&mut b, 0x2400, &hn);
    for (rva, v) in [(0x2100, 0x2400u64), (0x2200, 0x2400)] {
        w64(&mut b, data_at(rva), v);
    }
    b
}

#[test]
fn the_createfontindirectw_thunk_calls_the_runtime_and_the_names_are_part_of_the_match() {
    let img = build_one("gdi32.dll", "CreateFontIndirectW");
    let mut out = Vec::new();
    let st = generate(&img, &mut out).unwrap();
    let c = String::from_utf8(out).unwrap();
    assert_eq!(st.handled["jmp_imp:CreateFontIndirectW"], 1);
    assert!(c.contains("fle_imp_CreateFontIndirectW(); return;"));
    assert!(!c.contains(&format!("L_{:x}: {{ /* {:#x} Jmp */ fle_unsupported", IB + 0x1040, IB + 0x1040)));
    // another DLL, the ANSI name, the CreateFontW name and the Ex name are not this API
    for (dll, api) in [("other.dll", "CreateFontIndirectW"), ("gdi32.dll", "CreateFontIndirectA"), ("gdi32.dll", "CreateFontW"), ("gdi32.dll", "CreateFontIndirectExW")] {
        let img = build_one(dll, api);
        let mut out = Vec::new();
        let st = generate(&img, &mut out).unwrap();
        let c = String::from_utf8(out).unwrap();
        assert!(!st.handled.contains_key("jmp_imp:CreateFontIndirectW"), "{dll}!{api}");
        assert!(!c.contains("fle_imp_CreateFontIndirectW"), "{dll}!{api}");
        let va = IB + 0x1040;
        assert!(c.contains(&format!("L_{va:x}: {{ /* {va:#x} Jmp */ fle_unsupported(UINT64_C({va:#x})); }}")), "{dll}!{api}");
    }
}

#[test]
fn the_getobjectw_thunk_calls_the_runtime_and_the_names_are_part_of_the_match() {
    // decision 0068: gdi32.dll!GetObjectW only; another DLL, the A form and the Ex-less other spellings are not this API
    let img = build_one("gdi32.dll", "GetObjectW");
    let mut out = Vec::new();
    let st = generate(&img, &mut out).unwrap();
    let c = String::from_utf8(out).unwrap();
    assert_eq!(st.handled["jmp_imp:GetObjectW"], 1);
    assert!(c.contains("fle_imp_GetObjectW(); return;"));
    assert!(!c.contains(&format!("L_{:x}: {{ /* {:#x} Jmp */ fle_unsupported", IB + 0x1040, IB + 0x1040)));
    for (dll, api) in [("other.dll", "GetObjectW"), ("gdi32.dll", "GetObjectA"), ("gdi32.dll", "GetObject"), ("gdi32.dll", "GetObjectType")] {
        let img = build_one(dll, api);
        let mut out = Vec::new();
        let st = generate(&img, &mut out).unwrap();
        let c = String::from_utf8(out).unwrap();
        assert!(!st.handled.contains_key("jmp_imp:GetObjectW"), "{dll}!{api}");
        assert!(!c.contains("fle_imp_GetObjectW"), "{dll}!{api}");
        let va = IB + 0x1040;
        assert!(c.contains(&format!("L_{va:x}: {{ /* {va:#x} Jmp */ fle_unsupported(UINT64_C({va:#x})); }}")), "{dll}!{api}");
    }
}

#[test]
fn the_dll_name_is_part_of_the_match() {
    // the same API names from another DLL are not these APIs: they stay unsupported
    let img = build("other.dll");
    let mut out = Vec::new();
    let st = generate(&img, &mut out).unwrap();
    let c = String::from_utf8(out).unwrap();
    assert_eq!(st.handled["jmp_imp:LoadIconW"], 1);
    for name in ["CreatePalette", "DeleteObject", "GetStockObject"] {
        assert!(!st.handled.contains_key(format!("jmp_imp:{name}").as_str()), "{name}");
        assert!(!c.contains(&format!("fle_imp_{name}")), "{name}");
    }
    for va in [IB + 0x1050, IB + 0x1060, IB + 0x1070] {
        assert!(c.contains(&format!("L_{va:x}: {{ /* {va:#x} Jmp */ fle_unsupported(UINT64_C({va:#x})); }}")), "{va:#x}");
    }
}

// ---- 12. Memory DCs, DIB sections, brushes, selection (decision 0108, PROVISIONAL) ----
//
// Expected values come from the decision and the Learn pages it cites (CreateCompatibleDC, DeleteDC, SelectObject, SelectPalette, CreateCompatibleBitmap,
// CreateDIBSection, BITMAPINFOHEADER, GetDIBColorTable, SetDIBColorTable, SetTextColor, SetBkColor, CreateBrushIndirect, LOGBRUSH, CreateHalftonePalette,
// CreateIconIndirect, ICONINFO, GetObject, BITMAP, DeleteObject), not from the C code. A number the pages do not give (a handle, the default objects, the initial
// colours, the zero fill of a DIB, the colour of COLOR_WINDOWTEXT) is marked "choice" and is the only thing such a test pins down. The slot plan: stock 0..31,
// icon 32, created from 33 (fle_gdi.h); the first memory DC makes the four default objects (33..36), then itself (37) and its placeholder (38).

/// The 19 rows of decision 0108, in the order they are appended to imports_table.rs.
const ROWS_0108: [(&str, &str); 19] = [
    ("gdi32.dll", "CreateCompatibleDC"),
    ("gdi32.dll", "DeleteDC"),
    ("gdi32.dll", "SelectObject"),
    ("gdi32.dll", "SelectPalette"),
    ("gdi32.dll", "CreateCompatibleBitmap"),
    ("gdi32.dll", "CreateDIBSection"),
    ("gdi32.dll", "GetDIBColorTable"),
    ("gdi32.dll", "SetDIBColorTable"),
    ("gdi32.dll", "SetTextColor"),
    ("gdi32.dll", "SetBkColor"),
    ("gdi32.dll", "CreateBrushIndirect"),
    ("gdi32.dll", "CreateHalftonePalette"),
    ("user32.dll", "GetSysColor"),
    ("user32.dll", "CreateIconIndirect"),
    ("gdi32.dll", "BitBlt"),
    ("gdi32.dll", "PatBlt"),
    ("user32.dll", "FillRect"),
    ("gdi32.dll", "GetDIBits"),
    ("gdi32.dll", "RealizePalette"),
];

const W6_TOP: &str = r#"
static uint64_t cN(void (*f)(void), uint64_t a1, uint64_t a2, uint64_t a3, uint64_t a4, uint64_t a5, uint64_t a6, uint64_t a7, uint64_t a8, uint64_t a9)
{
    memset(stk, 0, sizeof stk);
    stk[0] = 0xdead0000u;
    stk[5] = a5; stk[6] = a6; stk[7] = a7; stk[8] = a8; stk[9] = a9;     /* [rsp+0x28]...: the fifth and later arguments */
    R[1] = a1; R[2] = a2; R[8] = a3; R[9] = a4;
    R[4] = (uint64_t)(uintptr_t)stk;
    R[0] = 0xBADBADBADull;
    f();
    return R[0];
}
#define C4(f, a, b, c, d) cN((void (*)(void))(f), (a), (b), (c), (d), 0, 0, 0, 0, 0)
#define MKDC() C4(fle_imp_CreateCompatibleDC, 0, 0, 0, 0)
#define DCOF(h) C4(fle_imp_CreateCompatibleDC, (h), 0, 0, 0)
#define DELDC(h) ((uint32_t)C4(fle_imp_DeleteDC, (h), 0, 0, 0))
#define SELO(dc, h) C4(fle_imp_SelectObject, (dc), (h), 0, 0)
#define SELP(dc, h, f) C4(fle_imp_SelectPalette, (dc), (h), (f), 0)
#define MKBMP(w, h) cN(fle_imp_CreateBitmap, (w), (h), 1, 1, 0, 0, 0, 0, 0)
#define CCB(dc, w, h) C4(fle_imp_CreateCompatibleBitmap, (dc), (w), (h), 0)
#define SCR() C4(fle_imp_GetDC, 0, 0, 0, 0)
#define HALFTONE(dc) C4(fle_imp_CreateHalftonePalette, (dc), 0, 0, 0)
#define SLOT(h) U(((h) - FLE_GDI_HANDLE_BASE) / 4 - 1)
#define DEF(i) fle_gdi_default_object(i)
#define SELN(h) ((unsigned)fle_gdi_selected_count(h))
#define KIND(h) fle_gdi_kind(h)
#define GOBJ(h, c, p) C4(fle_imp_GetObjectW, (h), (c), (p), 0)

static uint8_t lb[16];
static uint64_t mkbrush(uint32_t style, uint32_t color, uint64_t hatch)
{
    memset(lb, 0, sizeof lb);
    memcpy(lb, &style, 4);
    memcpy(lb + 4, &color, 4);
    memcpy(lb + 8, &hatch, 8);
    return C4(fle_imp_CreateBrushIndirect, A16(lb), 0, 0, 0);
}

static uint8_t bmi[40 + 4 * 256 + 16];
static uint64_t bits_out;
static void p32(unsigned o, uint32_t v) { memcpy(bmi + o, &v, 4); }
static void p16(unsigned o, uint16_t v) { memcpy(bmi + o, &v, 2); }
static void mkbmi(int32_t w, int32_t h, unsigned bpp, uint32_t comp, uint32_t clrused)
{
    memset(bmi, 0, sizeof bmi);
    p32(0, 40); p32(4, (uint32_t)w); p32(8, (uint32_t)h); p16(12, 1); p16(14, (uint16_t)bpp); p32(16, comp); p32(32, clrused);
}
static uint64_t DIB(uint64_t dc)
{
    bits_out = 0xEE;
    return cN(fle_imp_CreateDIBSection, dc, A16(bmi), 0, A16(&bits_out), 0, 0, 0, 0, 0);
}
static void dibinfo(uint64_t h)
{
    uint64_t bits = 0;
    uint32_t o[6] = { 7, 7, 7, 7, 7, 7 };
    int k = fle_gdi_dib_info(h, &bits, o);
    printf("dib %d %u %u %u %u %u %u bits %d\n", k, o[0], o[1], o[2], o[3], o[4], o[5], bits != 0 && bits == bits_out);
}
static int isdib(uint64_t h)
{
    uint64_t b = 0;
    uint32_t o[6];
    return fle_gdi_dib_info(h, &b, o);
}
static void bm32(const uint8_t *b, const char *label, uint64_t r)
{
    uint32_t t, w, h, wb;
    uint16_t pl, bp;
    uint64_t bits;
    memcpy(&t, b, 4); memcpy(&w, b + 4, 4); memcpy(&h, b + 8, 4); memcpy(&wb, b + 12, 4);
    memcpy(&pl, b + 16, 2); memcpy(&bp, b + 18, 2); memcpy(&bits, b + 24, 8);
    printf("%s: returns %llu type %u w %u h %u widthbytes %u planes %u bits %u bmBits %llu\n", label, U(r), t, w, h, wb, pl, bp, U(bits));
}
static void bmpinfo(uint64_t h)
{
    uint32_t o[4] = { 7, 7, 7, 7 };
    int k = fle_gdi_bitmap_info(h, o);
    printf("info %d %u %u %u %u format %d\n", k, o[0], o[1], o[2], o[3], fle_gdi_bitmap_format(h));
}

static uint8_t ii[32];
static uint64_t ICONI(uint32_t ficon, uint32_t hx, uint32_t hy, uint64_t mask, uint64_t color)
{
    memset(ii, 0, sizeof ii);
    memcpy(ii, &ficon, 4); memcpy(ii + 4, &hx, 4); memcpy(ii + 8, &hy, 4);
    memcpy(ii + 16, &mask, 8); memcpy(ii + 24, &color, 8);
    return C4(fle_imp_CreateIconIndirect, A16(ii), 0, 0, 0);
}
"#;

/// Each (label, C statements, word of the message) must stop the run with SIGABRT, a "fle: <api>:" message that contains `word`, and no output after it. `setup` runs
/// first.
fn stops_with(prefix: &str, setup: &str, api: &str, cases: &[(&str, &str, &str)]) {
    for (label, stmts, word) in cases {
        let body = format!("{setup}\n    {stmts}\n    printf(\"returned %llu\\n\", U(R[0]));");
        let o = run_src(&format!("{prefix}-{label}"), W6_TOP, &body);
        assert!(!o.ok, "{label}: expected an abort, got stdout {}", o.stdout);
        assert!(!o.stdout.contains("returned"), "{label}: returned instead of stopping");
        assert_eq!(o.signal, Some(6), "{label}: not SIGABRT: {}", o.stderr);
        let msg = o.stderr.lines().find(|l| l.starts_with(&format!("fle: {api}:"))).unwrap_or_else(|| panic!("{label}: no 'fle: {api}:' message: {}", o.stderr)).to_string();
        assert!(msg.contains(word), "{label}: message lacks {word:?}: {msg}");
    }
}

fn w6_lines(name: &str, body: &str) -> Vec<String> {
    let o = run_src(name, W6_TOP, body);
    assert!(o.ok, "{name} did not exit normally (signal {:?}): {}\n{}", o.signal, o.stderr, o.stdout);
    o.stdout.lines().map(|s| s.to_string()).collect()
}

/// The (dll, name) of every row of TABLE in imports_table.rs, in order.
fn table_rows() -> Vec<(String, String)> {
    let t = std::fs::read_to_string(manifest().join("src/imports_table.rs")).unwrap();
    let a = t.find("const TABLE").unwrap();
    let b = a + t[a..].find("\n];").unwrap();
    t[a..b]
        .lines()
        .filter_map(|l| {
            let l = l.trim();
            let l = l.strip_prefix("(\"")?;
            let (dll, rest) = l.split_once("\", \"")?;
            let (name, _) = rest.split_once("\", \"")?;
            Some((dll.to_string(), name.to_string()))
        })
        .collect()
}

#[test]
fn the_nineteen_rows_are_appended_in_order_after_the_older_rows_and_declared() {
    let rows = table_rows();
    let h = std::fs::read_to_string(src_dir().join("fle_gdi.h")).unwrap();
    let first = rows.iter().position(|(_, n)| n == "CreateCompatibleDC").expect("CreateCompatibleDC row");
    // the 19 rows follow one another in the decided order (decision 0015: rows are only appended, a pseudo-VA never moves)
    for (k, (dll, n)) in ROWS_0108.iter().enumerate() {
        assert_eq!(rows[first + k], (dll.to_string(), n.to_string()), "row {k}");
        assert_eq!(rows.iter().filter(|(d, m)| d.as_str() == *dll && m.as_str() == *n).count(), 1, "{n}: exactly one row");
        assert!(h.contains(&format!("void fle_imp_{n}(void)")), "{n} is declared in fle_gdi.h");
    }
    // the older rows of this file keep their places: every older row is before the first of the new ones
    for old in ["CreatePalette", "DeleteObject", "GetStockObject", "LoadIconW", "CreateFontIndirectW", "GetObjectW", "CreateBitmap", "EnumFontFamiliesExW", "GetDC", "GetDeviceCaps"] {
        let k = rows.iter().position(|(_, n)| n == old).unwrap_or_else(|| panic!("{old} row"));
        assert!(k < first, "{old} (row {k}) must stay before the first row of decision 0108 (row {first})");
    }
    // the pseudo-VA of a row is 0x7FF900000000 + 0x10 * (index + 1) (decision 0015): the first of the new rows is not at the VA of an older one
    assert!(first >= 311, "the table had 311 rows before decision 0108 (decision 0106); the new rows come after them");
    // the neighbours that are NOT answered by this decision: no row (decision 0111 gives DestroyCursor, GetPaletteEntries, StretchBlt, ExtTextOutW, CreateSolidBrush, SetDIBits and StretchDIBits rows: they were in this list until then)
    // decision 0126 (wall 55): old: "CreatePen" was in this list; it is a row now (appended at the end of the table: the order test above is about decision 0108's rows)
    for n in ["TextOutW", "GetCurrentObject"] {
        assert!(!rows.iter().any(|(_, m)| m == n), "{n} must not be answered by decision 0108");
    }
}

#[test]
fn the_constants_of_decision_0108() {
    assert_eq!(def_const("fle_gdi.h", "FLE_GDI_KIND_DC"), 8);
    assert_eq!(def_const("fle_gdi.h", "FLE_GDI_KIND_BRUSH"), 9);
    assert_eq!(def_const("fle_gdi.h", "FLE_GDI_KIND_DEFAULT"), 10);
    assert_eq!(def_const("fle_gdi.h", "FLE_GDI_KIND_ICONCUR"), 11);
    assert_eq!((def_const("fle_gdi.h", "FLE_GDI_BMP_MONO"), def_const("fle_gdi.h", "FLE_GDI_BMP_COLOR"), def_const("fle_gdi.h", "FLE_GDI_BMP_DIB")), (0, 1, 2));
    assert_eq!(
        (def_const("fle_gdi.h", "FLE_GDI_DEFAULT_BRUSH"), def_const("fle_gdi.h", "FLE_GDI_DEFAULT_PEN"), def_const("fle_gdi.h", "FLE_GDI_DEFAULT_FONT"), def_const("fle_gdi.h", "FLE_GDI_DEFAULT_PALETTE")),
        (0, 1, 2, 3)
    );
    // sizes from the member types on the Learn pages (x64): BITMAP = LONG x 4 + WORD x 2 + 4 padding + LPVOID
    assert_eq!(4 * 4 + 2 * 2 + 4 + 8, 32);
    assert_eq!(def_const("fle_gdi.h", "FLE_GDI_BITMAP_SIZE"), 32);
    // DIBSECTION = BITMAP 32 + BITMAPINFOHEADER 40 + DWORD x 3 (12) + 4 padding + HANDLE 8 + DWORD 4 + 4 padding
    assert_eq!(32 + 40 + 12 + 4 + 8 + 4 + 4, 104);
    assert_eq!(def_const("fle_gdi.h", "FLE_GDI_DIBSECTION_SIZE"), 104);
    // BITMAPINFOHEADER = DWORD + LONG x 2 + WORD x 2 + DWORD x 2 + LONG x 2 + DWORD x 2
    assert_eq!(4 + 4 * 2 + 2 * 2 + 4 * 2 + 4 * 2 + 4 * 2, 40);
    assert_eq!(def_const("fle_gdi.h", "FLE_GDI_BMIH_SIZE"), 40);
    // LOGBRUSH = UINT + COLORREF + ULONG_PTR; ICONINFO = BOOL + DWORD x 2 + 4 padding + HBITMAP x 2
    assert_eq!(4 + 4 + 8, 16);
    assert_eq!(def_const("fle_gdi.h", "FLE_GDI_LOGBRUSH_SIZE"), 16);
    assert_eq!(4 + 4 + 4 + 4 + 8 + 8, 32);
    assert_eq!(def_const("fle_gdi.h", "FLE_GDI_ICONINFO_SIZE"), 32);
    // Learn: BI_RGB 0, BI_BITFIELDS 3, DIB_RGB_COLORS 0, DIB_PAL_COLORS 1, BS_SOLID 0, BS_HOLLOW = BS_NULL 1 (written here from the pages)
    assert_eq!(def_const("fle_gdi.h", "FLE_GDI_BI_RGB"), 0);
    assert_eq!(def_const("fle_gdi.h", "FLE_GDI_BI_BITFIELDS"), 3);
    assert_eq!(def_const("fle_gdi.h", "FLE_GDI_DIB_RGB_COLORS"), 0);
    assert_eq!(def_const("fle_gdi.h", "FLE_GDI_DIB_PAL_COLORS"), 1);
    assert_eq!(def_const("fle_gdi.h", "FLE_GDI_BS_SOLID"), 0);
    assert_eq!(def_const("fle_gdi.h", "FLE_GDI_BS_HOLLOW"), 1);
    assert_eq!(def_const("fle_gdi.h", "FLE_GDI_BS_NULL"), 1);
    // choices (no page gives them): the initial colours of a memory DC and the one system colour
    assert_eq!(def_const("fle_gdi.h", "FLE_GDI_DC_TEXT_COLOR"), 0x0000_0000);
    assert_eq!(def_const("fle_gdi.h", "FLE_GDI_DC_BK_COLOR"), 0x00FF_FFFF);
    assert_eq!(def_const("fle_gdi.h", "FLE_GDI_COLOR_WINDOWTEXT"), 8);
    assert_eq!(def_const("fle_gdi.h", "FLE_GDI_COLOR_WINDOWTEXT_RGB"), 0);
}

// ---- 12a. CreateCompatibleDC, DeleteDC ----

#[test]
fn a_memory_dc_starts_with_the_placeholder_the_default_objects_and_the_initial_colours() {
    let l = w6_lines(
        "w6-dc",
        r#"
    uint64_t dc, dc2, ph, sel[5];
    uint32_t col[2];
    SETLE(0x1234);
    dc = MKDC();
    ph = fle_gdi_dc_placeholder(dc);
    printf("region %d aligned %d sign-extends %d\n", INREG(dc), (dc % 4) == 0, (uint64_t)(int64_t)(int32_t)(uint32_t)dc == dc);
    printf("slots dc %llu placeholder %llu defaults %llu %llu %llu %llu\n", SLOT(dc), SLOT(ph), SLOT(DEF(0)), SLOT(DEF(1)), SLOT(DEF(2)), SLOT(DEF(3)));
    printf("kinds dc %d placeholder %d defaults %d %d %d %d\n", KIND(dc), KIND(ph), KIND(DEF(0)), KIND(DEF(1)), KIND(DEF(2)), KIND(DEF(3)));
    printf("state %d\n", fle_gdi_dc_state(dc, sel, col));
    printf("selected %d %d %d %d %d\n", sel[0] == ph, sel[1] == DEF(0), sel[2] == DEF(1), sel[3] == DEF(2), sel[4] == DEF(3));
    printf("colours %08x %08x\n", (unsigned)col[0], (unsigned)col[1]);
    bmpinfo(ph);
    printf("placeholder selected in %u dc\n", SELN(ph));
    printf("live %llu dcs %llu bitmaps %llu\n", U(fle_gdi_live_count()), U(fle_gdi_dc_live_count()), U(fle_gdi_bitmap_live_count()));
    printf("last error %x\n", LE());
    dc2 = MKDC();
    printf("second: differs %d slots %llu %llu defaults shared %d selected-in %u\n", dc2 != dc, SLOT(dc2), SLOT(fle_gdi_dc_placeholder(dc2)), DEF(0) == sel[1], SELN(DEF(0)));
    printf("live %llu dcs %llu bitmaps %llu\n", U(fle_gdi_live_count()), U(fle_gdi_dc_live_count()), U(fle_gdi_bitmap_live_count()));
    printf("a DC is not a screen DC: %d %d\n", fle_gdc_is_live(dc), INREG(dc));
"#,
    );
    // the 4 default objects (33..36) are made with the first DC, then the DC (37) and its placeholder (38); the second DC is 39 with the placeholder 40
    assert_eq!(
        l,
        [
            "region 1 aligned 1 sign-extends 1",
            "slots dc 37 placeholder 38 defaults 33 34 35 36",
            "kinds dc 8 placeholder 7 defaults 10 10 10 10",
            "state 1",
            "selected 1 1 1 1 1",
            "colours 00000000 00ffffff",
            "info 1 1 1 1 1 format 0",
            "placeholder selected in 1 dc",
            "live 6 dcs 1 bitmaps 1",
            "last error 1234",
            "second: differs 1 slots 39 40 defaults shared 1 selected-in 2",
            "live 8 dcs 2 bitmaps 2",
            "a DC is not a screen DC: 0 1",
        ]
    );
}

#[test]
fn a_memory_dc_can_be_made_from_null_or_a_screen_dc_and_from_nothing_else() {
    let l = w6_lines(
        "w6-dc-arg",
        r#"
    uint64_t sdc = SCR(), a = DCOF(0), b = DCOF(sdc);
    printf("ok %d %d kinds %d %d\n", a != 0, b != 0, KIND(a), KIND(b));
    printf("screen dc still live %llu\n", U(fle_gdc_live_count()));
    printf("released %u\n", (unsigned)C4(fle_imp_ReleaseDC, 0, sdc, 0, 0));
"#,
    );
    assert_eq!(l, ["ok 1 1 kinds 8 8", "screen dc still live 1", "released 1"]);
    stops_with(
        "w6-abort-dc",
        "uint64_t dc = MKDC(), b = MKBMP(2, 2), dead = MKDC();\n    DELDC(dead);",
        "CreateCompatibleDC",
        &[
            ("memory-dc", "DCOF(dc);", "memory DC"),
            ("deleted-dc", "DCOF(dead);", "was deleted"),
            ("bitmap", "DCOF(b);", "neither NULL nor a DC"),
            ("never-issued", "DCOF(0x1234);", "neither NULL nor a DC"),
            ("region-base", "DCOF(FLE_GDI_HANDLE_BASE);", "neither NULL nor a DC"),
            ("event-region", "DCOF(0x7FFA80000004ull);", "neither NULL nor a DC"),
        ],
    );
}

#[test]
fn delete_dc_unselects_what_was_selected_and_does_not_delete_it() {
    let l = w6_lines(
        "w6-deletedc",
        r#"
    uint64_t dc = MKDC(), b = MKBMP(8, 8), ph = fle_gdi_dc_placeholder(dc);
    printf("select %d\n", SELO(dc, b) == ph);
    printf("selected: bitmap %u placeholder %u default brush %u\n", SELN(b), SELN(ph), SELN(DEF(0)));
    printf("delete %u\n", DELDC(dc));
    printf("after: dc kind %d dcs %llu bitmap kind %d selected %u placeholder kind %d default brush %u\n", KIND(dc), U(fle_gdi_dc_live_count()), KIND(b), SELN(b), KIND(ph), SELN(DEF(0)));
    printf("again %u null %u\n", DELDC(dc), DELDC(0));
    printf("the bitmap can be deleted now: %u\n", DEL(b));
    printf("live %llu bitmaps %llu\n", U(fle_gdi_live_count()), U(fle_gdi_bitmap_live_count()));
"#,
    );
    // live: only the 4 default objects remain
    assert_eq!(
        l,
        [
            "select 1",
            "selected: bitmap 1 placeholder 0 default brush 1",
            "delete 1",
            "after: dc kind 0 dcs 0 bitmap kind 7 selected 0 placeholder kind 0 default brush 0",
            "again 0 null 0",
            "the bitmap can be deleted now: 1",
            "live 4 bitmaps 0",
        ]
    );
}

#[test]
fn delete_object_on_a_memory_dc_deletes_it_like_delete_dc() {
    // Learn CreateCompatibleDC: "you can also call DeleteObject with the HDC to delete the DC"
    let l = w6_lines(
        "w6-deleteobject-dc",
        r#"
    uint64_t dc = MKDC(), b = MKBMP(4, 4);
    SELO(dc, b);
    printf("delete %u\n", DEL(dc));
    printf("again %u\n", DEL(dc));
    printf("dcs %llu bitmap selected %u\n", U(fle_gdi_dc_live_count()), SELN(b));
    printf("delete dc again as DeleteDC %u\n", DELDC(dc));
"#,
    );
    assert_eq!(l, ["delete 1", "again 0", "dcs 0 bitmap selected 0", "delete dc again as DeleteDC 0"]);
}

#[test]
fn delete_dc_of_what_the_page_leaves_open_stops_by_name() {
    stops_with(
        "w6-abort-deletedc",
        "uint64_t sdc = SCR(), b = MKBMP(2, 2);",
        "DeleteDC",
        &[
            ("screen-dc", "DELDC(sdc);", "must not delete one"),
            ("bitmap", "DELDC(b);", "not a DC of this runtime"),
            ("never-issued", "DELDC(0x1234);", "not a DC of this runtime"),
            ("stock", "DELDC(STOCK(7));", "not a DC of this runtime"),
        ],
    );
}

#[test]
fn a_memory_dc_is_not_a_screen_dc_for_release_dc_and_get_device_caps() {
    // the screen DC table (decision 0053) stops on a value it did not issue
    stops_with("w6-abort-release-memdc", "uint64_t dc = MKDC();", "ReleaseDC", &[("release", "C4(fle_imp_ReleaseDC, 0, dc, 0, 0);", "not issued by GetDC")]);
    // decision 0111 (g1-optional-getdevicecaps.diff): a memory DC answers BITSPIXEL and PLANES from its bitmap (the placeholder is monochrome: 1 and 1) and stops by name for the other indexes
    let l = w6_lines("w6-caps-memdc", "uint64_t dc = MKDC();\n    uint64_t a = C4(fle_imp_GetDeviceCaps, dc, 12, 0, 0), b = C4(fle_imp_GetDeviceCaps, dc, 14, 0, 0);\n    printf(\"bits %llu planes %llu\\n\", U(a), U(b));");
    assert_eq!(l, ["bits 1 planes 1"]);
    stops_with("w6-abort-caps-memdc", "uint64_t dc = MKDC();", "GetDeviceCaps", &[("horzres", "C4(fle_imp_GetDeviceCaps, dc, 8, 0, 0);", "index 8 of a memory DC")]);
}

// ---- 12b. SelectObject, SelectPalette, DeleteObject of a selected object ----

#[test]
fn select_object_returns_the_previous_bitmap_and_a_bitmap_is_in_one_dc_at_a_time() {
    let l = w6_lines(
        "w6-select-bitmap",
        r#"
    uint64_t dc = MKDC(), dc2 = MKDC(), ph = fle_gdi_dc_placeholder(dc), ph2 = fle_gdi_dc_placeholder(dc2), b = MKBMP(8, 8), b2 = MKBMP(4, 4), sel[5];
    uint32_t col[2];
    SETLE(0x1234);
    printf("first returns the placeholder %d\n", SELO(dc, b) == ph);
    printf("same again returns itself %d\n", SELO(dc, b) == b);
    printf("replace returns the old one %d\n", SELO(dc, b2) == b);
    printf("counts b %u b2 %u ph %u\n", SELN(b), SELN(b2), SELN(ph));
    printf("back to the placeholder returns b2 %d\n", SELO(dc, ph) == b2);
    printf("counts b %u b2 %u ph %u\n", SELN(b), SELN(b2), SELN(ph));
    SELO(dc, b);
    printf("selected in another DC: %llu\n", U(SELO(dc2, b)));
    fle_gdi_dc_state(dc2, sel, col);
    printf("dc2 unchanged %d, count of b %u\n", sel[0] == ph2, SELN(b));
    fle_gdi_dc_state(dc, sel, col);
    printf("dc holds b %d\n", sel[0] == b);
    printf("last error %x\n", LE());
"#,
    );
    assert_eq!(
        l,
        [
            "first returns the placeholder 1",
            "same again returns itself 1",
            "replace returns the old one 1",
            "counts b 0 b2 1 ph 0",
            "back to the placeholder returns b2 1",
            "counts b 0 b2 0 ph 1",
            "selected in another DC: 0",
            "dc2 unchanged 1, count of b 1",
            "dc holds b 1",
            "last error 1234",
        ]
    );
}

#[test]
fn select_object_selects_brushes_pens_and_fonts_and_returns_the_previous_one_of_the_type() {
    let l = w6_lines(
        "w6-select-gdiobj",
        r#"
    uint64_t dc = MKDC(), br = mkbrush(0, 0x00112233, 0), sb = STOCK(5), pen = STOCK(7), sf = STOCK(13), f, sel[5];
    uint32_t col[2];
    mklf(1); f = MKFONT();
    printf("brush returns the default %d\n", SELO(dc, br) == DEF(0));
    printf("stock brush returns the brush %d\n", SELO(dc, sb) == br);
    printf("pen returns the default %d\n", SELO(dc, pen) == DEF(1));
    printf("font returns the default %d\n", SELO(dc, f) == DEF(2));
    printf("stock font returns the font %d\n", SELO(dc, sf) == f);
    fle_gdi_dc_state(dc, sel, col);
    printf("state %d %d %d %d\n", sel[1] == sb, sel[2] == pen, sel[3] == sf, sel[4] == DEF(3));
    printf("counts brush %u stock brush %u font %u stock font %u pen %u\n", SELN(br), SELN(sb), SELN(f), SELN(sf), SELN(pen));
    printf("default font back returns the stock font %d\n", SELO(dc, DEF(2)) == sf);
    printf("default brush back returns the stock brush %d, default pen back returns the pen %d\n", SELO(dc, DEF(0)) == sb, SELO(dc, DEF(1)) == pen);
    printf("counts default brush %u pen %u font %u stock font %u\n", SELN(DEF(0)), SELN(DEF(1)), SELN(DEF(2)), SELN(sf));
"#,
    );
    assert_eq!(
        l,
        [
            "brush returns the default 1",
            "stock brush returns the brush 1",
            "pen returns the default 1",
            "font returns the default 1",
            "stock font returns the font 1",
            "state 1 1 1 1",
            "counts brush 0 stock brush 1 font 0 stock font 1 pen 1",
            "default font back returns the stock font 1",
            "default brush back returns the stock brush 1, default pen back returns the pen 1",
            "counts default brush 1 pen 1 font 1 stock font 0",
        ]
    );
}

#[test]
fn a_bitmap_into_a_screen_dc_is_refused_with_null_and_changes_nothing() {
    // Learn SelectObject: "Bitmaps can only be selected into memory DC's"; "If an error occurs ... the return value is NULL"
    let l = w6_lines(
        "w6-select-screen",
        r#"
    uint64_t sdc = SCR(), b = MKBMP(4, 4), r;
    r = SELO(sdc, b);
    printf("returns %llu selected %u\n", U(r), SELN(b));
"#,
    );
    assert_eq!(l, ["returns 0 selected 0"]);
}

#[test]
fn delete_object_of_a_selected_object_returns_zero_until_it_is_selected_out() {
    // Learn DeleteObject: zero "if the specified handle is not valid or is currently selected into a DC"
    let l = w6_lines(
        "w6-delete-selected",
        r#"
    uint64_t dc = MKDC(), b = MKBMP(8, 8), br = mkbrush(0, 1, 0), f, pal = MKPAL();
    mklf(1); f = MKFONT();
    SELO(dc, b); SELO(dc, br); SELO(dc, f); SELP(dc, pal, 0);
    printf("selected: %u %u %u %u\n", DEL(b), DEL(br), DEL(f), DEL(pal));
    printf("still live: %d %d %d %d\n", KIND(b) == 7, KIND(br) == 9, KIND(f) == 6, KIND(pal) == 1);
    SELO(dc, fle_gdi_dc_placeholder(dc)); SELO(dc, DEF(0)); SELO(dc, DEF(2)); SELP(dc, DEF(3), 0);
    printf("selected out: %u %u %u %u\n", DEL(b), DEL(br), DEL(f), DEL(pal));
    printf("gone: %d %d %d %d\n", KIND(b), KIND(br), KIND(f), KIND(pal));
    printf("again: %u %u %u %u\n", DEL(b), DEL(br), DEL(f), DEL(pal));
"#,
    );
    assert_eq!(l, ["selected: 0 0 0 0", "still live: 1 1 1 1", "selected out: 1 1 1 1", "gone: 0 0 0 0", "again: 0 0 0 0"]);
}

#[test]
fn select_object_of_what_the_page_leaves_open_stops_by_name() {
    stops_with(
        "w6-abort-select",
        "uint64_t dc = MKDC(), dead_dc = MKDC(), pal = MKPAL(), ic = ICON(), b = MKBMP(2, 2), db = MKBMP(2, 2), sdc = SCR();\n    DEL(db); DELDC(dead_dc);",
        "SelectObject",
        &[
            ("palette", "SELO(dc, pal);", "h is a palette"),
            ("default-palette", "SELO(dc, DEF(3));", "h is a palette"),
            ("icon", "SELO(dc, ic);", "icon or a cursor"),
            ("dc-as-h", "SELO(dc, dc);", "h is a DC"),
            ("null-h", "SELO(dc, 0);", "h was not issued"),
            ("never-issued-h", "SELO(dc, 0x1234);", "h was not issued"),
            ("deleted-bitmap", "SELO(dc, db);", "h was deleted"),
            // decision 0126 (wall 55): old: ("screen-dc-brush", SELO(sdc, STOCK(5))) stopped ("GetDC(NULL)"); a DC of GetDC(NULL) keeps its own selected objects now (default objects, like a memory DC), so a brush is selected and the previous one returned
            ("null-dc", "SELO(0, b);", "not a DC of this runtime"),
            ("bitmap-as-dc", "SELO(b, b);", "not a DC of this runtime"),
            ("deleted-dc", "SELO(dead_dc, b);", "hdc was deleted"),
        ],
    );
}

#[test]
fn select_palette_returns_the_previous_palette_and_ignores_bforcebkgd_for_a_memory_dc() {
    let l = w6_lines(
        "w6-select-palette",
        r#"
    uint64_t sdc = SCR(), dc = MKDC(), pal = MKPAL(), ht = HALFTONE(sdc), sel[5];
    uint32_t col[2];
    printf("pal returns the default %d\n", SELP(dc, pal, 1) == DEF(3));
    printf("halftone returns pal %d (bForceBkgd 0)\n", SELP(dc, ht, 0) == pal);
    printf("default returns halftone %d (bForceBkgd 1)\n", SELP(dc, DEF(3), 1) == ht);
    printf("same again returns itself %d\n", SELP(dc, DEF(3), 0) == DEF(3));
    fle_gdi_dc_state(dc, sel, col);
    printf("state %d, counts pal %u halftone %u default %u\n", sel[4] == DEF(3), SELN(pal), SELN(ht), SELN(DEF(3)));
    printf("delete %u %u\n", DEL(pal), DEL(ht));
"#,
    );
    assert_eq!(l, ["pal returns the default 1", "halftone returns pal 1 (bForceBkgd 0)", "default returns halftone 1 (bForceBkgd 1)", "same again returns itself 1", "state 1, counts pal 0 halftone 0 default 1", "delete 1 1"]);
}

#[test]
fn select_palette_of_what_the_page_leaves_open_stops_by_name() {
    stops_with(
        "w6-abort-selectpal",
        "uint64_t sdc = SCR(), dc = MKDC(), dead_dc = MKDC(), pal = MKPAL(), dp = MKPAL(), br = mkbrush(0, 0, 0);\n    DEL(dp); DELDC(dead_dc);",
        "SelectPalette",
        &[
            // decision 0126 (wall 55): old: ("screen-dc", SELP(sdc, pal, 0)) stopped ("GetDC(NULL)"); the screen DC has a palette state now
            // decision 0113 wall 7: a NULL hPal on a memory DC returns NULL (see select_palette_null_is_the_failure_value_and_changes_nothing); on a DC of GetDC(NULL) it still stops
            // decision 0126 (wall 55): old: ("null-palette-on-a-screen-dc", SELP(sdc, 0, 0)) stopped ("GetDC(NULL)"); the screen DC has a palette state now, and a NULL hPal returns NULL on it as on a memory DC (decision 0113 wall 7)
            ("brush-as-palette", "SELP(dc, br, 0);", "not a live palette"),
            ("deleted-palette", "SELP(dc, dp, 0);", "not a live palette"),
            ("dc-as-palette", "SELP(dc, dc, 0);", "not a live palette"),
            ("null-dc", "SELP(0, pal, 0);", "not a DC of this runtime"),
            ("deleted-dc", "SELP(dead_dc, pal, 0);", "was deleted"),
        ],
    );
}

#[test]
fn delete_object_of_the_placeholder_the_default_objects_and_an_icon_stops() {
    stops_with(
        "w6-abort-delete-0108",
        "uint64_t dc = MKDC(), m = MKBMP(8, 8), ic;\n    ic = ICONI(1, 0, 0, MKBMP(8, 16), 0);",
        "DeleteObject",
        &[
            ("placeholder", "DEL(fle_gdi_dc_placeholder(dc));", "placeholder bitmap"),
            ("default-brush", "DEL(DEF(0));", "default object"),
            ("default-palette", "DEL(DEF(3));", "default object"),
            ("created-icon", "DEL(ic);", "icon or a cursor"),
        ],
    );
}

#[test]
fn get_object_of_a_dc_a_brush_or_a_default_object_stops_as_a_kind_the_page_does_not_describe() {
    stops_with(
        "w6-abort-getobj-0108",
        "uint64_t dc = MKDC(), br = mkbrush(0, 1, 0);\n    uint8_t buf[96];",
        "GetObjectW",
        &[
            ("dc", "GOBJ(dc, 16, A16(buf));", "kind this table does not describe"),
            ("brush", "GOBJ(br, 16, A16(buf));", "kind this table does not describe"),
            ("default-brush", "GOBJ(DEF(0), 16, A16(buf));", "kind this table does not describe"),
        ],
    );
}

// ---- 12c. CreateCompatibleBitmap ----

#[test]
fn a_compatible_bitmap_follows_the_format_of_the_dc_it_is_made_for() {
    let l = w6_lines(
        "w6-ccb",
        r#"
    uint64_t sdc = SCR(), dc = MKDC(), c, m, from_mem_colour, from_mem_mono;
    SETLE(0x1234);
    c = CCB(sdc, 100, 50);
    printf("screen: "); bmpinfo(c);
    printf("memory DC with the placeholder: "); m = CCB(dc, 100, 50); bmpinfo(m);
    SELO(dc, c);
    printf("memory DC with a colour bitmap: "); from_mem_colour = CCB(dc, 10, 20); bmpinfo(from_mem_colour);
    SELO(dc, m);
    printf("memory DC with a monochrome bitmap: "); from_mem_mono = CCB(dc, 10, 20); bmpinfo(from_mem_mono);
    printf("zero width: "); bmpinfo(CCB(sdc, 0, 5));
    printf("zero height: "); bmpinfo(CCB(sdc, 5, 0));
    printf("low dwords only: "); bmpinfo(CCB(sdc, 0xAAAAAAAA00000007ull, 0xBBBBBBBB00000003ull));
    printf("largest below 2 GiB: "); bmpinfo(CCB(sdc, 0x10000000, 1));
    printf("region %d slots %llu %llu\n", INREG(c), SLOT(c), SLOT(m));
    printf("live bitmaps %llu\n", U(fle_gdi_bitmap_live_count()));
    printf("delete %u %u\n", DEL(from_mem_colour), DEL(from_mem_mono));
    printf("last error %x\n", LE());
"#,
    );
    // colour = the virtual display: 1 plane, 32 bits (fle_gdc.h); the first created slot is 33; here the DC (made first) takes the default objects 33..36, itself 37, its placeholder 38
    assert_eq!(
        l,
        [
            "screen: info 1 100 50 1 32 format 1",
            "memory DC with the placeholder: info 1 100 50 1 1 format 0",
            "memory DC with a colour bitmap: info 1 10 20 1 32 format 1",
            "memory DC with a monochrome bitmap: info 1 10 20 1 1 format 0",
            "zero width: info 1 1 1 1 1 format 0",
            "zero height: info 1 1 1 1 1 format 0",
            "low dwords only: info 1 7 3 1 32 format 1",
            "largest below 2 GiB: info 1 268435456 1 1 32 format 1",
            "region 1 slots 39 40",
            "live bitmaps 9",
            "delete 1 1",
            "last error 1234",
        ]
    );
}

#[test]
fn a_compatible_bitmap_of_what_the_page_leaves_open_stops_by_name() {
    stops_with(
        "w6-abort-ccb",
        "uint64_t sdc = SCR(), dc = MKDC(), dead_dc = MKDC(), d, b = MKBMP(2, 2);\n    DELDC(dead_dc);\n    mkbmi(4, 4, 32, 0, 0); d = DIB(sdc); SELO(dc, d);",
        "CreateCompatibleBitmap",
        &[
            ("dib-selected", "CCB(dc, 8, 8);", "DIB section is selected"),
            ("negative-width", "CCB(sdc, 0xFFFFFFFFu, 8);", "negative"),
            ("negative-height", "CCB(sdc, 8, 0x80000000u);", "negative"),
            ("two-gib", "CCB(sdc, 0x10000000, 2);", "2 GiB"),
            ("null-dc", "CCB(0, 8, 8);", "not a DC of this runtime"),
            ("deleted-dc", "CCB(dead_dc, 8, 8);", "was deleted"),
            ("bitmap-as-dc", "CCB(b, 8, 8);", "not a DC of this runtime"),
        ],
    );
}

// ---- 12d. CreateDIBSection and GetObjectW of a DIB section ----

#[test]
fn a_dib_section_gets_its_bits_a_handle_and_the_stride_of_the_page() {
    let l = w6_lines(
        "w6-dib",
        r#"
    uint64_t sdc = SCR(), h, h2, p1;
    uint8_t *bits;
    unsigned i, z = 1;
    SETLE(0x1234);
    mkbmi(5, -3, 32, 0, 0);
    h = DIB(sdc);
    p1 = bits_out;
    printf("region %d aligned %d kind %d format %d\n", INREG(h), (h % 4) == 0, KIND(h), fle_gdi_bitmap_format(h));
    dibinfo(h);
    bmpinfo(h);
    bits = (uint8_t *)(uintptr_t)p1;
    for (i = 0; i < 60; i++) if (bits[i]) z = 0;
    bits[0] = 0xAB; bits[59] = 0x5A;
    printf("zero filled %d, writable %d\n", z, bits[0] == 0xAB && bits[59] == 0x5A);
    mkbmi(3, 2, 24, 0, 0); h2 = DIB(sdc);
    printf("24 bits 3x2: "); dibinfo(h2);
    printf("distinct: handle %d bits %d, first still the same %d\n", h2 != h, bits_out != p1, bits[0] == 0xAB);
    mkbmi(4, 4, 32, 0, 0); printf("bottom-up 32 bits 4x4: "); dibinfo(DIB(sdc));
    mkbmi(3, 2, 16, 0, 0); printf("16 bits 3x2: "); dibinfo(DIB(sdc));
    mkbmi(5, -3, 32, 0, 0); p32(20, 60); printf("with biSizeImage 60: "); dibinfo(DIB(sdc));
    mkbmi(2, 2, 16, 3, 0); p32(40, 0xF800); p32(44, 0x07E0); p32(48, 0x001F); printf("565: "); dibinfo(DIB(sdc));
    mkbmi(2, 2, 16, 3, 0); p32(40, 0x7C00); p32(44, 0x03E0); p32(48, 0x001F); printf("555: "); dibinfo(DIB(sdc));
    mkbmi(2, 2, 32, 3, 0); p32(40, 0xFF0000); p32(44, 0xFF00); p32(48, 0xFF); printf("888: "); dibinfo(DIB(sdc));
    { uint64_t mdc = MKDC(), m; mkbmi(1, 1, 32, 0, 0); m = DIB(mdc); printf("from a memory DC: "); dibinfo(m); }
    printf("last error %x\n", LE());
"#,
    );
    assert_eq!(
        l,
        [
            "region 1 aligned 1 kind 7 format 2",
            "dib 1 5 3 32 0 20 0 bits 1",
            "info 1 5 3 1 32 format 2",
            "zero filled 1, writable 1",
            "24 bits 3x2: dib 1 3 2 24 0 12 0 bits 1",
            "distinct: handle 1 bits 1, first still the same 1",
            "bottom-up 32 bits 4x4: dib 1 4 4 32 0 16 0 bits 1",
            "16 bits 3x2: dib 1 3 2 16 0 8 0 bits 1",
            "with biSizeImage 60: dib 1 5 3 32 0 20 0 bits 1",
            "565: dib 1 2 2 16 3 4 0 bits 1",
            "555: dib 1 2 2 16 3 4 0 bits 1",
            "888: dib 1 2 2 32 3 8 0 bits 1",
            "from a memory DC: dib 1 1 1 32 0 4 0 bits 1",
            "last error 1234",
        ]
    );
}

#[test]
fn a_dib_section_of_8_bits_or_less_keeps_the_colour_table_of_its_bitmapinfo() {
    let l = w6_lines(
        "w6-dib-colours",
        r#"
    uint64_t sdc = SCR(), h;
    mkbmi(33, 1, 1, 0, 0); p32(40, 0x00000000); p32(44, 0x00FFFFFF);
    h = DIB(sdc); printf("1 bit: "); dibinfo(h);
    printf("colours %08x %08x %08x\n", (unsigned)fle_gdi_dib_color(h, 0), (unsigned)fle_gdi_dib_color(h, 1), (unsigned)fle_gdi_dib_color(h, 2));
    mkbmi(2, 2, 8, 0, 3); p32(40, 0x00112233); p32(44, 0x00445566); p32(48, 0x00778899);
    h = DIB(sdc); printf("8 bits, biClrUsed 3: "); dibinfo(h);
    printf("colours %08x %08x %08x %08x\n", (unsigned)fle_gdi_dib_color(h, 0), (unsigned)fle_gdi_dib_color(h, 1), (unsigned)fle_gdi_dib_color(h, 2), (unsigned)fle_gdi_dib_color(h, 3));
    mkbmi(1, 1, 4, 0, 0); h = DIB(sdc); printf("4 bits, biClrUsed 0: "); dibinfo(h);
    mkbmi(1, 1, 8, 0, 0); h = DIB(sdc); printf("8 bits, biClrUsed 0: "); dibinfo(h);
    mkbmi(1, 1, 24, 0, 0); h = DIB(sdc); printf("24 bits: "); dibinfo(h);
    mkbmi(1, 1, 8, 0, 256); h = DIB(sdc); printf("8 bits, biClrUsed 256: "); dibinfo(h);
"#,
    );
    assert_eq!(
        l,
        [
            "1 bit: dib 1 33 1 1 0 8 2 bits 1",
            "colours 00000000 00ffffff 00000000",
            "8 bits, biClrUsed 3: dib 1 2 2 8 0 4 3 bits 1",
            "colours 00112233 00445566 00778899 00000000",
            "4 bits, biClrUsed 0: dib 1 1 1 4 0 4 16 bits 1",
            "8 bits, biClrUsed 0: dib 1 1 1 8 0 4 256 bits 1",
            "24 bits: dib 1 1 1 24 0 4 0 bits 1",
            "8 bits, biClrUsed 256: dib 1 1 1 8 0 4 256 bits 1",
        ]
    );
}

#[test]
fn delete_object_of_a_dib_section_follows_the_page_and_releases_its_bits() {
    let l = w6_lines(
        "w6-dib-delete",
        r#"
    uint64_t sdc = SCR(), dc = MKDC(), h, h2;
    unsigned r1, r2, r3;
    mkbmi(2, 2, 32, 0, 0);
    h = DIB(sdc);
    SELO(dc, h);
    r1 = DEL(h);
    printf("selected: %u still a section %d\n", r1, isdib(h));
    SELO(dc, fle_gdi_dc_placeholder(dc));
    r2 = DEL(h);
    printf("selected out: %u section %d\n", r2, isdib(h));
    r3 = DEL(h);
    printf("again %u\n", r3);
    printf("bitmaps %llu\n", U(fle_gdi_bitmap_live_count()));
    mkbmi(2, 2, 32, 0, 0);
    h2 = DIB(sdc);
    printf("a new section is a new handle %d\n", h2 != h);
"#,
    );
    // live bitmaps: the placeholder of the DC (the section is gone)
    assert_eq!(l, ["selected: 0 still a section 1", "selected out: 1 section 0", "again 0", "bitmaps 1", "a new section is a new handle 1"]);
}

#[test]
fn get_object_of_a_dib_section_gives_the_bitmap_structure_for_32_bytes_and_stops_for_a_dibsection() {
    let l = w6_lines(
        "w6-dib-getobject",
        r#"
    uint64_t sdc = SCR(), h, r;
    uint32_t raw[28];
    uint8_t *buf = (uint8_t *)raw;
    uint32_t v;
    uint16_t w;
    uint64_t q;
    mkbmi(5, -3, 32, 0, 0);
    h = DIB(sdc);
    memset(raw, 0xC7, sizeof raw);
    r = GOBJ(h, 32, A16(buf));
    printf("returns %llu\n", U(r));
    memcpy(&v, buf, 4); printf("bmType %u\n", v);
    memcpy(&v, buf + 4, 4); printf("bmWidth %u\n", v);
    memcpy(&v, buf + 8, 4); printf("bmHeight %u\n", v);
    memcpy(&v, buf + 12, 4); printf("bmWidthBytes %u\n", v);
    memcpy(&w, buf + 16, 2); printf("bmPlanes %u\n", w);
    memcpy(&w, buf + 18, 2); printf("bmBitsPixel %u\n", w);
    memcpy(&v, buf + 20, 4); printf("padding %u\n", v);
    memcpy(&q, buf + 24, 8); printf("bmBits is the pointer the guest got %d\n", q == bits_out);
    printf("nothing past 32 bytes %d\n", buf[32] == 0xC7 && buf[111] == 0xC7);
"#,
    );
    assert_eq!(
        l,
        ["returns 32", "bmType 0", "bmWidth 5", "bmHeight 3", "bmWidthBytes 20", "bmPlanes 1", "bmBitsPixel 32", "padding 0", "bmBits is the pointer the guest got 1", "nothing past 32 bytes 1"]
    );
    stops_with(
        "w6-abort-dib-getobject",
        "uint64_t sdc = SCR(), h;\n    uint8_t buf[112];\n    mkbmi(5, -3, 32, 0, 0); h = DIB(sdc);",
        "GetObjectW",
        &[
            // decision 0113 wall 5: c == 104 (sizeof(DIBSECTION)) is answered now (get_object_of_a_dib_section_gives_a_dibsection below); a larger c still stops
            ("bigger", "GOBJ(h, 112, A16(buf));", "above sizeof(DIBSECTION)"),
            ("one-past", "GOBJ(h, 105, A16(buf));", "above sizeof(DIBSECTION)"),
            ("between", "GOBJ(h, 64, A16(buf));", "neither sizeof(BITMAP)"),
            ("bitmap-plus-one", "GOBJ(h, 33, A16(buf));", "neither sizeof(BITMAP)"),
            ("null-pv", "GOBJ(h, 32, 0);", "pv is NULL"),
            ("too-small", "GOBJ(h, 31, A16(buf));", "below sizeof(BITMAP)"),
        ],
    );
}

/// Decision 0113 wall 5 (provisional): GetObjectW(DIB section, 104, pv) stores a DIBSECTION. Expected bytes are written from the decision's table
/// (BITMAP as for c = 32; BITMAPINFOHEADER = the header the guest passed with biSizeImage = |h| * stride; masks; zeros elsewhere), not read back from the C.
/// Decision 0113 wall 7 (provisional, a choice without a source): SelectPalette(memory DC, NULL, f) returns NULL and changes nothing; the last error is left alone.
#[test]
fn select_palette_null_is_the_failure_value_and_changes_nothing() {
    let l = w6_lines(
        "c1-selectpalette-null",
        r#"
    uint64_t dc = MKDC(), pal = HALFTONE(dc), r;
    SETLE(0x1234);
    r = SELP(dc, 0, 0);
    printf("null returns %llu\n", U(r));
    r = SELP(dc, 0, 1);
    printf("null again (force) returns %llu\n", U(r));
    cN(fle_imp_GetLastError, 0, 0, 0, 0, 0, 0, 0, 0, 0);
    printf("last error %llx\n", U(R[0]));
    printf("the selected palette is still the default %d\n", SELP(dc, pal, 0) == DEF(3));
    printf("then the halftone palette is selected %d\n", SELP(dc, DEF(3), 0) == pal);
"#,
    );
    assert_eq!(l, ["null returns 0", "null again (force) returns 0", "last error 1234", "the selected palette is still the default 1", "then the halftone palette is selected 1"]);
}

#[test]
fn get_object_of_a_dib_section_gives_a_dibsection() {
    let l = w6_lines(
        "c1-dibsection",
        r#"
    uint64_t sdc = SCR(), h, r;
    uint32_t raw[32];
    uint8_t *buf = (uint8_t *)raw;
    uint32_t v;
    uint16_t w;
    uint64_t q;
    int i, nz;
    /* a top-down 32-bit BI_RGB DIB 5 x 3 with the resolution and colour counts the guest passed */
    mkbmi(5, -3, 32, 0, 0);
    p32(24, 3780); p32(28, 3781); p32(36, 9);
    h = DIB(sdc);
    memset(raw, 0xC7, sizeof raw);
    r = GOBJ(h, 104, A16(buf));
    printf("returns %llu\n", U(r));
#define F4(o) (memcpy(&v, buf + (o), 4), v)
    printf("bm %u %u %u %u\n", F4(0), F4(4), F4(8), F4(12));
    memcpy(&w, buf + 16, 2); printf("bmPlanes %u\n", w);
    memcpy(&w, buf + 18, 2); printf("bmBitsPixel %u\n", w);
    printf("bm padding %u\n", F4(20));
    memcpy(&q, buf + 24, 8); printf("bmBits is the pointer the guest got %d\n", q == bits_out);
    printf("bmih %u %d %d\n", F4(32), (int)F4(36), (int)F4(40));
    memcpy(&w, buf + 44, 2); printf("biPlanes %u\n", w);
    memcpy(&w, buf + 46, 2); printf("biBitCount %u\n", w);
    printf("biCompression %u biSizeImage %u\n", F4(48), F4(52));
    printf("pels %u %u clr %u %u\n", F4(56), F4(60), F4(64), F4(68));
    printf("bitfields %x %x %x\n", F4(72), F4(76), F4(80));
    memcpy(&q, buf + 88, 8); printf("dshSection %llu dsOffset %u pad %u %u\n", U(q), F4(96), F4(84), F4(100));
    for (nz = 0, i = 104; i < 128; i++) nz += buf[i] != 0xC7;
    printf("nothing past 104 bytes %d\n", nz == 0);
    /* a misaligned buffer fails as for c = 32 */
    memset(raw, 0xC7, sizeof raw);
    printf("misaligned %llu %d\n", U(GOBJ(h, 104, A16(buf + 2))), buf[2] == 0xC7 && buf[3] == 0xC7);
"#,
    );
    assert_eq!(
        l,
        [
            "returns 104",
            "bm 0 5 3 20",
            "bmPlanes 1",
            "bmBitsPixel 32",
            "bm padding 0",
            "bmBits is the pointer the guest got 1",
            "bmih 40 5 -3",
            "biPlanes 1",
            "biBitCount 32",
            "biCompression 0 biSizeImage 60",
            "pels 3780 3781 clr 0 9",
            "bitfields ff0000 ff00 ff",
            "dshSection 0 dsOffset 0 pad 0 0",
            "nothing past 104 bytes 1",
            "misaligned 0 1",
        ]
    );
    // a bottom-up 8-bit DIB keeps the sign and the colour count, has no masks; a 16-bit BI_RGB DIB is RGB 555; a BI_BITFIELDS 16-bit DIB gives its masks
    let l = w6_lines(
        "c1-dibsection-forms",
        r#"
    uint64_t sdc = SCR(), h;
    uint8_t buf[112];
    uint32_t v;
    uint16_t w;
#define F4(o) (memcpy(&v, buf + (o), 4), v)
    mkbmi(7, 2, 8, 0, 17);
    h = DIB(sdc);
    printf("ret8 %llu\n", U(GOBJ(h, 104, A16(buf))));
    memcpy(&w, buf + 18, 2);
    printf("8bit: bmWidthBytes %u bmBitsPixel %u biHeight %d biSizeImage %u clrused %u masks %x %x %x\n", F4(12), w, (int)F4(40), F4(52), F4(64), F4(72), F4(76), F4(80));
    mkbmi(5, 4, 16, 0, 0);
    h = DIB(sdc);
    printf("ret16 %llu\n", U(GOBJ(h, 104, A16(buf))));
    printf("16bit rgb: stride %u size %u masks %x %x %x\n", F4(12), F4(52), F4(72), F4(76), F4(80));
    mkbmi(5, 4, 16, 3, 0);
    p32(40, 0xF800); p32(44, 0x07E0); p32(48, 0x001F);
    h = DIB(sdc);
    printf("ret16bf %llu\n", U(GOBJ(h, 104, A16(buf))));
    printf("16bit bitfields: comp %u masks %x %x %x\n", F4(48), F4(72), F4(76), F4(80));
    mkbmi(2, 2, 1, 0, 0);
    h = DIB(sdc);
    printf("ret1 %llu\n", U(GOBJ(h, 104, A16(buf))));
    printf("1bit: stride %u size %u masks %x %x %x\n", F4(12), F4(52), F4(72), F4(76), F4(80));
"#,
    );
    assert_eq!(
        l,
        [
            "ret8 104",
            "8bit: bmWidthBytes 8 bmBitsPixel 8 biHeight 2 biSizeImage 16 clrused 17 masks 0 0 0",
            "ret16 104",
            "16bit rgb: stride 12 size 48 masks 7c00 3e0 1f",
            "ret16bf 104",
            "16bit bitfields: comp 3 masks f800 7e0 1f",
            "ret1 104",
            "1bit: stride 4 size 8 masks 0 0 0",
        ]
    );
}

/// A 24-bit BI_RGB DIB section: no page gives dsBitfields for it, so the DIBSECTION request stops by name; the BITMAP request still answers.
#[test]
fn get_object_of_a_24_bit_dib_section_asks_for_a_dibsection_stops() {
    stops_with(
        "c1-dibsection-24",
        "uint64_t sdc = SCR(), h;\n    uint8_t buf[112];\n    mkbmi(5, 3, 24, 0, 0); h = DIB(sdc);",
        "GetObjectW",
        &[("dibsection-24bit", "GOBJ(h, 104, A16(buf));", "24-bit BI_RGB DIB section")],
    );
    let l = w6_lines(
        "c1-dibsection-24-bitmap",
        "uint64_t sdc = SCR(), h;\n    uint8_t buf[112];\n    mkbmi(5, 3, 24, 0, 0); h = DIB(sdc);\n    printf(\"bitmap %llu\\n\", U(GOBJ(h, 32, A16(buf))));",
    );
    assert_eq!(l, ["bitmap 32"]);
}

#[test]
fn create_dib_section_forms_the_pages_leave_open_stop_by_name() {
    // (label, C statements after the common setup (a valid 32-bit header in bmi), word)
    let setup = "uint64_t sdc = SCR(), dc = MKDC(), dead_dc = MKDC(), b = MKBMP(2, 2);\n    DELDC(dead_dc);\n    mkbmi(4, 4, 32, 0, 0);";
    let cases: &[(&str, &str, &str)] = &[
        ("usage-pal-colors", "cN(fle_imp_CreateDIBSection, sdc, A16(bmi), 1, A16(&bits_out), 0, 0, 0, 0, 0);", "usage is 1"),
        ("usage-2", "cN(fle_imp_CreateDIBSection, sdc, A16(bmi), 2, A16(&bits_out), 0, 0, 0, 0, 0);", "usage is 2"),
        ("hsection", "cN(fle_imp_CreateDIBSection, sdc, A16(bmi), 0, A16(&bits_out), 0x1234, 0, 0, 0, 0);", "hSection is not NULL"),
        ("pbmi-null", "cN(fle_imp_CreateDIBSection, sdc, 0, 0, A16(&bits_out), 0, 0, 0, 0, 0);", "pbmi is NULL"),
        ("ppvbits-null", "cN(fle_imp_CreateDIBSection, sdc, A16(bmi), 0, 0, 0, 0, 0, 0, 0);", "ppvBits is NULL"),
        ("hdc-null", "cN(fle_imp_CreateDIBSection, 0, A16(bmi), 0, A16(&bits_out), 0, 0, 0, 0, 0);", "not a live DC"),
        ("hdc-deleted", "cN(fle_imp_CreateDIBSection, dead_dc, A16(bmi), 0, A16(&bits_out), 0, 0, 0, 0, 0);", "not a live DC"),
        ("hdc-bitmap", "cN(fle_imp_CreateDIBSection, b, A16(bmi), 0, A16(&bits_out), 0, 0, 0, 0, 0);", "not a live DC"),
        ("bisize-12", "p32(0, 12); DIB(sdc);", "biSize is 12"),
        ("bisize-108", "p32(0, 108); DIB(sdc);", "biSize is 108"),
        ("bisize-124", "p32(0, 124); DIB(sdc);", "biSize is 124"),
        ("planes-2", "p16(12, 2); DIB(sdc);", "biPlanes"),
        ("planes-0", "p16(12, 0); DIB(sdc);", "biPlanes"),
        ("width-0", "p32(4, 0); DIB(sdc);", "biWidth"),
        ("width-negative", "p32(4, 0xFFFFFFFFu); DIB(sdc);", "biWidth"),
        ("height-0", "p32(8, 0); DIB(sdc);", "biHeight is 0"),
        ("bitcount-0", "p16(14, 0); DIB(sdc);", "biBitCount is 0"),
        ("bitcount-2", "p16(14, 2); DIB(sdc);", "biBitCount is 2"),
        ("bitcount-15", "p16(14, 15); DIB(sdc);", "biBitCount is 15"),
        ("bitcount-48", "p16(14, 48); DIB(sdc);", "biBitCount is 48"),
        ("rle8", "mkbmi(4, 4, 8, 1, 0); DIB(sdc);", "biCompression is 1"),
        ("rle4", "mkbmi(4, 4, 4, 2, 0); DIB(sdc);", "biCompression is 2"),
        ("jpeg", "mkbmi(4, 4, 0, 4, 0); DIB(sdc);", "biBitCount is 0"),
        ("bitfields-24", "mkbmi(4, 4, 24, 3, 0); DIB(sdc);", "biCompression is 3"),
        ("bitfields-8", "mkbmi(4, 4, 8, 3, 0); DIB(sdc);", "biCompression is 3"),
        ("bitfields-odd-masks", "mkbmi(4, 4, 32, 3, 0); p32(40, 0xFF000000u); p32(44, 0xFF0000); p32(48, 0xFF00); DIB(sdc);", "masks"),
        ("bitfields-zero-masks", "mkbmi(4, 4, 16, 3, 0); DIB(sdc);", "masks"),
        ("clrused-257", "mkbmi(4, 4, 8, 0, 257); DIB(sdc);", "biClrUsed"),
        ("clrused-3-for-1-bit", "mkbmi(4, 4, 1, 0, 3); DIB(sdc);", "biClrUsed"),
        ("sizeimage-wrong", "p32(20, 17); DIB(sdc);", "biSizeImage"),
        ("size-2-gib", "mkbmi(0x10000000, 2, 32, 0, 0); DIB(sdc);", "2 GiB"),
        ("size-2-gib-negative-height", "mkbmi(0x10000000, -2, 32, 0, 0); DIB(sdc);", "2 GiB"),
    ];
    stops_with("w6-abort-dibsection", setup, "CreateDIBSection", cases);
}

// ---- 12e. GetDIBColorTable, SetDIBColorTable ----

#[test]
fn the_colour_table_of_the_dib_section_selected_into_a_dc_is_read_and_written_with_a_clamp() {
    let l = w6_lines(
        "w6-dib-table",
        r#"
    uint64_t sdc = SCR(), dc = MKDC(), h;
    uint32_t buf[300], nw[2] = { 0x00112233, 0x00445566 };
    unsigned i;
    mkbmi(8, 8, 1, 0, 0); p32(40, 0x00000000); p32(44, 0x00FFFFFF);
    h = DIB(sdc);
    SELO(dc, h);
    for (i = 0; i < 300; i++) buf[i] = 0xC7C7C7C7u;
    uint64_t r, r2, r3;
    r = C4(fle_imp_GetDIBColorTable, dc, 0, 256, A16(buf));
    printf("get 256 from 0: %llu, %08x %08x untouched %d\n", U(r), (unsigned)buf[0], (unsigned)buf[1], buf[2] == 0xC7C7C7C7u);
    r = C4(fle_imp_GetDIBColorTable, dc, 1, 1, A16(buf));
    printf("get 1 from 1: %llu, %08x\n", U(r), (unsigned)buf[0]);
    r = C4(fle_imp_GetDIBColorTable, dc, 2, 1, A16(buf));
    r2 = C4(fle_imp_GetDIBColorTable, dc, 0xFFFFFFFFu, 5, A16(buf));
    r3 = C4(fle_imp_GetDIBColorTable, dc, 0, 0, A16(buf));
    printf("get from 2: %llu, from 0xFFFFFFFF: %llu, zero entries: %llu\n", U(r), U(r2), U(r3));
    r = C4(fle_imp_GetDIBColorTable, dc, 0xAAAAAAAA00000001ull, 0xBBBBBBBB00000001ull, A16(buf));
    printf("low dwords only: %llu, %08x\n", U(r), (unsigned)buf[0]);
    r = C4(fle_imp_SetDIBColorTable, dc, 0, 2, A16(nw));
    printf("set 2 from 0: %llu, now %08x %08x\n", U(r), (unsigned)fle_gdi_dib_color(h, 0), (unsigned)fle_gdi_dib_color(h, 1));
    nw[0] = 0x00AAAAAA;
    r = C4(fle_imp_SetDIBColorTable, dc, 1, 5, A16(nw));
    printf("set 5 from 1 is clamped: %llu, now %08x %08x\n", U(r), (unsigned)fle_gdi_dib_color(h, 0), (unsigned)fle_gdi_dib_color(h, 1));
    r = C4(fle_imp_SetDIBColorTable, dc, 5, 1, A16(nw));
    printf("set from 5: %llu\n", U(r));
"#,
    );
    assert_eq!(
        l,
        [
            "get 256 from 0: 2, 00000000 00ffffff untouched 1",
            "get 1 from 1: 1, 00ffffff",
            "get from 2: 0, from 0xFFFFFFFF: 0, zero entries: 0",
            "low dwords only: 1, 00ffffff",
            "set 2 from 0: 2, now 00112233 00445566",
            "set 5 from 1 is clamped: 1, now 00112233 00aaaaaa",
            "set from 5: 0",
        ]
    );
}

#[test]
fn without_a_dib_section_of_8_bits_or_less_the_colour_table_calls_fail_with_zero() {
    let l = w6_lines(
        "w6-dib-table-zero",
        r#"
    uint64_t sdc = SCR(), dc = MKDC(), d24, ddb = MKBMP(4, 4);
    uint32_t buf[8];
    mkbmi(4, 4, 24, 0, 0); d24 = DIB(sdc);
    printf("placeholder: %llu\n", U(C4(fle_imp_GetDIBColorTable, dc, 0, 2, A16(buf))));
    SELO(dc, ddb);
    printf("device-dependent bitmap: %llu\n", U(C4(fle_imp_GetDIBColorTable, dc, 0, 2, A16(buf))));
    SELO(dc, d24);
    printf("24-bit DIB: get %llu set %llu\n", U(C4(fle_imp_GetDIBColorTable, dc, 0, 2, A16(buf))), U(C4(fle_imp_SetDIBColorTable, dc, 0, 2, A16(buf))));
    printf("screen DC: %llu %llu\n", U(C4(fle_imp_GetDIBColorTable, sdc, 0, 2, A16(buf))), U(C4(fle_imp_SetDIBColorTable, sdc, 0, 2, A16(buf))));
    printf("NULL: %llu %llu\n", U(C4(fle_imp_GetDIBColorTable, 0, 0, 2, A16(buf))), U(C4(fle_imp_SetDIBColorTable, 0, 0, 2, A16(buf))));
"#,
    );
    assert_eq!(l, ["placeholder: 0", "device-dependent bitmap: 0", "24-bit DIB: get 0 set 0", "screen DC: 0 0", "NULL: 0 0"]);
    stops_with(
        "w6-abort-dibtable",
        "uint64_t sdc = SCR(), dc = MKDC(), dead_dc = MKDC(), h, b = MKBMP(2, 2);\n    uint32_t buf[4];\n    DELDC(dead_dc);\n    mkbmi(4, 4, 1, 0, 0); h = DIB(sdc); SELO(dc, h);",
        "GetDIBColorTable",
        &[
            ("prgbq-null", "C4(fle_imp_GetDIBColorTable, dc, 0, 2, 0);", "prgbq is NULL"),
            ("deleted-dc", "C4(fle_imp_GetDIBColorTable, dead_dc, 0, 2, A16(buf));", "was deleted"),
            ("bitmap-as-dc", "C4(fle_imp_GetDIBColorTable, b, 0, 2, A16(buf));", "not a DC of this runtime"),
        ],
    );
    stops_with(
        "w6-abort-dibtable-set",
        "uint64_t sdc = SCR(), dc = MKDC(), h;\n    mkbmi(4, 4, 1, 0, 0); h = DIB(sdc); SELO(dc, h);",
        "SetDIBColorTable",
        &[("prgbq-null", "C4(fle_imp_SetDIBColorTable, dc, 0, 2, 0);", "prgbq is NULL")],
    );
}

// ---- 12f. SetTextColor, SetBkColor, GetSysColor ----

#[test]
fn the_text_and_background_colours_return_the_previous_colour_and_start_at_the_choices() {
    let l = w6_lines(
        "w6-colours",
        r#"
    uint64_t dc = MKDC(), dc2 = MKDC(), sel[5];
    uint32_t col[2];
    printf("first: text %08x bk %08x\n", (unsigned)C4(fle_imp_SetTextColor, dc, 0x00112233, 0, 0), (unsigned)C4(fle_imp_SetBkColor, dc, 0x00AABBCC, 0, 0));
    printf("second: text %08x bk %08x\n", (unsigned)C4(fle_imp_SetTextColor, dc, 0x00FFFFFF, 0, 0), (unsigned)C4(fle_imp_SetBkColor, dc, 0, 0, 0));
    fle_gdi_dc_state(dc, sel, col);
    printf("state text %08x bk %08x\n", (unsigned)col[0], (unsigned)col[1]);
    fle_gdi_dc_state(dc2, sel, col);
    printf("the other DC is not touched: text %08x bk %08x\n", (unsigned)col[0], (unsigned)col[1]);
    {
        uint64_t r = C4(fle_imp_SetTextColor, dc, 0xAAAAAAAA00001234ull, 0, 0);
        printf("low dword only: previous %08x upper half zero %d\n", (unsigned)r, (r >> 32) == 0);
    }
    fle_gdi_dc_state(dc, sel, col);
    printf("stored %08x\n", (unsigned)col[0]);
    SETLE(0x1234);
    C4(fle_imp_SetBkColor, dc, 5, 0, 0);
    printf("last error %x\n", LE());
"#,
    );
    assert_eq!(
        l,
        [
            "first: text 00000000 bk 00ffffff",
            "second: text 00112233 bk 00aabbcc",
            "state text 00ffffff bk 00000000",
            "the other DC is not touched: text 00000000 bk 00ffffff",
            "low dword only: previous 00ffffff upper half zero 1",
            "stored 00001234",
            "last error 1234",
        ]
    );
}

#[test]
fn a_colour_with_a_high_byte_or_a_dc_that_is_not_modelled_stops_by_name() {
    for (api, f) in [("SetTextColor", "fle_imp_SetTextColor"), ("SetBkColor", "fle_imp_SetBkColor")] {
        let cases: Vec<(String, String, &str)> = vec![
            // decision 0126 (wall 56): old: ("palette-rgb", 0x02FFFFFF) stopped ("high byte") because the page's PALETTERGB was not read; the high-order byte 2 is PALETTERGB, an ordinary RGB on this display (S-palettergb), so it is
            // answered (tests/wall_fr11.rs). The word of the message of the others is "high-order byte" (old: "high byte"): PALETTEINDEX 0x01 and every other high-order byte still stop.
            ("palette-index".to_string(), format!("C4({f}, dc, 0x01000000, 0, 0);"), "high-order byte"),
            ("top-byte".to_string(), format!("C4({f}, dc, 0xFF000000u, 0, 0);"), "high-order byte"),
            // decision 0126 (wall 55): old: ("screen-dc", SetTextColor / SetBkColor on a DC of GetDC(NULL)) stopped ("GetDC(NULL)"); the screen DC has a state of its own now (the default objects and colours), so it is answered

            ("null-dc".to_string(), format!("C4({f}, 0, 0, 0, 0);"), "not a DC of this runtime"),
            ("deleted-dc".to_string(), format!("C4({f}, dead_dc, 0, 0, 0);"), "was deleted"),
            ("bitmap-as-dc".to_string(), format!("C4({f}, b, 0, 0, 0);"), "not a DC of this runtime"),
        ];
        let refs: Vec<(&str, &str, &str)> = cases.iter().map(|(l, c, w)| (l.as_str(), c.as_str(), *w)).collect();
        stops_with(&format!("w6-abort-{api}"), "uint64_t sdc = SCR(), dc = MKDC(), dead_dc = MKDC(), b = MKBMP(2, 2);\n    DELDC(dead_dc);", api, &refs);
    }
}

/// Decision 0123 item 2 (old: `only_the_window_text_system_colour_is_answered_and_it_is_a_choice`: only index 8 answered (black) and every other index stopped): the indices 0..30 answer a table of the default
/// colours of Windows 10/11 with the light theme (COLORREF 0x00BBGGRR; every value is a choice without a source, S-colour), index 8 stays black (decision 0108), and any index below 0 or above 30 (read as a signed
/// 32-bit int) answers 0 as the Learn page says. The table is pinned value by value so that a changed value has to be a changed line here.
#[test]
fn get_sys_color_answers_the_table_of_indices_0_to_30_and_zero_for_any_other_index_since_decision_0123() {
    const TABLE: [u32; 31] = [
        0x00C8C8C8, 0x00000000, 0x00D1B499, 0x00DBCDBF, 0x00F0F0F0, 0x00FFFFFF, 0x00646464, 0x00000000, 0x00000000, 0x00000000, 0x00B4B4B4, 0x00FCF7F4, 0x00ABABAB, 0x00D77800, 0x00FFFFFF, 0x00F0F0F0,
        0x00A0A0A0, 0x006D6D6D, 0x00000000, 0x00000000, 0x00FFFFFF, 0x00696969, 0x00E3E3E3, 0x00000000, 0x00E1FFFF, 0x00000000, 0x00CC6600, 0x00EAD1B9, 0x00F2E4D7, 0x00FF9933, 0x00F0F0F0,
    ];
    let mut body = String::from("    uint64_t r;\n");
    for i in 0..31 {
        body.push_str(&format!("    r = C4(fle_imp_GetSysColor, {i}, 0, 0, 0); printf(\"%d %08x upper %d\\n\", {i}, (unsigned)r, (r >> 32) == 0);\n"));
    }
    body.push_str("    r = C4(fle_imp_GetSysColor, 0xAAAAAAAA00000008ull, 0, 0, 0); printf(\"low dword only: %llu upper half zero %d\\n\", U(r), (r >> 32) == 0);\n");
    for (k, i) in [31u64, 32, 100, 0xFFFF_FFFF, 0x8000_0000, 0x1_0000_001F].iter().enumerate() {
        body.push_str(&format!("    r = C4(fle_imp_GetSysColor, {i}ull, 0, 0, 0); printf(\"out {k}: %llu\\n\", U(r));\n"));
    }
    let l = w6_lines("w6-syscolor", &body);
    let mut want: Vec<String> = TABLE.iter().enumerate().map(|(i, c)| format!("{i} {c:08x} upper 1")).collect();
    want.push("low dword only: 0 upper half zero 1".to_string());
    for k in 0..6 {
        want.push(format!("out {k}: 0"));
    }
    assert_eq!(l, want);
    // COLOR_WINDOWTEXT stays black (decision 0108), and the 0x1_0000_001F case above is index 31 after the low dword: out of range
    assert_eq!(TABLE[8], 0);
    // the header and the source agree on the count
    assert_eq!(def_const("fle_gdi.h", "FLE_GDI_SYS_COLOR_COUNT"), 31);
}

// ---- 12g. CreateBrushIndirect, CreateHalftonePalette ----

#[test]
fn a_solid_or_hollow_brush_gets_a_handle_and_keeps_its_style_and_colour() {
    let l = w6_lines(
        "w6-brush",
        r#"
    uint64_t a, b, c, d;
    uint32_t o[2] = { 7, 7 };
    SETLE(0x1234);
    a = mkbrush(0, 0x00112233, 0x1122334455667788ull);
    printf("region %d aligned %d kind %d sign-extends %d\n", INREG(a), (a % 4) == 0, KIND(a), (uint64_t)(int64_t)(int32_t)(uint32_t)a == a);
    printf("info %d ", fle_gdi_brush_info(a, o)); printf("%u %08x\n", o[0], (unsigned)o[1]);
    b = mkbrush(1, 0x00ABCDEF, 5);
    printf("hollow ignores the colour: info %d ", fle_gdi_brush_info(b, o)); printf("%u %08x\n", o[0], (unsigned)o[1]);
    c = mkbrush(1, 0x7F000000, 0);
    printf("hollow with a high byte is fine: %d\n", c != 0);
    d = mkbrush(0, 0, 0);
    printf("distinct %d %d never the same slot %llu %llu %llu\n", a != b, b != d, SLOT(a), SLOT(b), SLOT(d));
    printf("live brushes %llu\n", U(fle_gdi_brush_live_count()));
    printf("delete %u\n", DEL(a));
    printf("again %u\n", DEL(a));
    printf("brushes %llu\n", U(fle_gdi_brush_live_count()));
    printf("last error %x\n", LE());
"#,
    );
    assert_eq!(
        l,
        [
            "region 1 aligned 1 kind 9 sign-extends 1",
            "info 1 0 00112233",
            "hollow ignores the colour: info 1 1 00000000",
            "hollow with a high byte is fine: 1",
            "distinct 1 1 never the same slot 33 34 36",
            "live brushes 4",
            "delete 1",
            "again 0",
            "brushes 3",
            "last error 1234",
        ]
    );
}

#[test]
fn a_brush_style_the_guest_does_not_use_or_a_palette_colour_stops_by_name() {
    stops_with(
        "w6-abort-brush",
        "",
        "CreateBrushIndirect",
        &[
            ("null", "C4(fle_imp_CreateBrushIndirect, 0, 0, 0, 0);", "plbrush is NULL"),
            ("hatched", "mkbrush(2, 0, 0);", "lbStyle is 2"),
            ("pattern", "mkbrush(3, 0, 0);", "lbStyle is 3"),
            ("dibpattern", "mkbrush(5, 0, 0);", "lbStyle is 5"),
            ("dibpatternpt", "mkbrush(6, 0, 0);", "lbStyle is 6"),
            ("pattern8x8", "mkbrush(7, 0, 0);", "lbStyle is 7"),
            ("dibpattern8x8", "mkbrush(8, 0, 0);", "lbStyle is 8"),
            ("big-style", "mkbrush(0xFFFFFFFFu, 0, 0);", "lbStyle is 4294967295"),
            // decision 0126: the word of the message is "high-order byte" (old: "high byte"); (wall 56, S-palettergb) old: ("solid-palette-rgb", 0x02112233) stopped; the high-order byte 2 is an ordinary RGB now
            ("solid-palette-index", "mkbrush(0, 0x01000005, 0);", "high-order byte"),
        ],
    );
}

#[test]
fn a_halftone_palette_is_an_opaque_palette_that_can_be_selected_and_deleted() {
    let l = w6_lines(
        "w6-halftone",
        r#"
    uint64_t sdc = SCR(), dc = MKDC(), a, b;
    a = HALFTONE(sdc);
    b = HALFTONE(dc);
    printf("region %d %d kinds %d %d distinct %d\n", INREG(a), INREG(b), KIND(a), KIND(b), a != b);
    printf("no count and no entries: %u %u %u\n", (unsigned)fle_gdi_palette_count(a), (unsigned)fle_gdi_palette_entry(a, 0), (unsigned)fle_gdi_palette_entry(a, 15));
    printf("palettes %llu\n", U(fle_gdi_palette_live_count()));
    printf("select returns the default %d\n", SELP(dc, a, 0) == DEF(3));
    printf("delete while selected %u\n", DEL(a));
    SELP(dc, b, 0);
    {
        unsigned da = DEL(a), db = DEL(b);
        printf("delete a %u b %u palettes %llu\n", da, db, U(fle_gdi_palette_live_count()));
    }
"#,
    );
    // a was selected out when b was selected, b is still selected
    assert_eq!(l, ["region 1 1 kinds 1 1 distinct 1", "no count and no entries: 0 0 0", "palettes 2", "select returns the default 1", "delete while selected 0", "delete a 1 b 0 palettes 1"]);
    // asking for the count or the entries of an opaque palette stops: the page says the entries are an implementation detail
    stops_with(
        "w6-abort-halftone",
        "uint64_t sdc = SCR(), a = HALFTONE(sdc);\n    uint8_t buf[16];",
        "GetObjectW",
        &[("count", "GOBJ(a, 2, A16(buf));", "handle is a palette")],
    );
    stops_with(
        "w6-abort-halftone-dc",
        "uint64_t dc = MKDC(), dead_dc = MKDC(), b = MKBMP(2, 2);\n    DELDC(dead_dc);",
        "CreateHalftonePalette",
        &[
            ("null-dc", "HALFTONE(0);", "not a DC of this runtime"),
            ("deleted-dc", "HALFTONE(dead_dc);", "was deleted"),
            ("bitmap-as-dc", "HALFTONE(b);", "not a DC of this runtime"),
        ],
    );
}

// ---- 12h. CreateIconIndirect ----

#[test]
fn an_icon_or_cursor_is_made_from_a_mask_and_a_colour_bitmap_and_the_bitmaps_stay_the_guests() {
    let l = w6_lines(
        "w6-icon",
        r#"
    uint64_t sdc = SCR(), col, mask, cur, ico, mono, ddbcol, ddbcur;
    uint32_t o[6];
    SETLE(0x1234);
    mkbmi(32, -32, 32, 0, 0); col = DIB(sdc);
    mask = MKBMP(32, 32);
    cur = ICONI(0, 5, 7, mask, col);
    printf("cursor: region %d aligned %d kind %d sign-extends %d\n", INREG(cur), (cur % 4) == 0, KIND(cur), (uint64_t)(int64_t)(int32_t)(uint32_t)cur == cur);
    printf("info %d ", fle_gdi_iconcur_info(cur, o)); printf("%u %u %u %u %u %u\n", o[0], o[1], o[2], o[3], o[4], o[5]);
    printf("the bitmaps stay: kinds %d %d selected %u %u\n", KIND(mask), KIND(col), SELN(mask), SELN(col));
    mono = MKBMP(32, 64);
    ico = ICONI(7, 1, 2, mono, 0);
    printf("monochrome icon (fIcon 7 is TRUE): info %d ", fle_gdi_iconcur_info(ico, o)); printf("%u %u %u %u %u %u\n", o[0], o[1], o[2], o[3], o[4], o[5]);
    ddbcol = CCB(sdc, 32, 32);
    ddbcur = ICONI(0, 0, 0, mask, ddbcol);
    printf("colour device-dependent bitmap: %d, distinct handles %d %d\n", ddbcur != 0, cur != ddbcur, cur != ico);
    printf("delete the guest's bitmaps %u %u %u\n", DEL(mask), DEL(col), DEL(mono));
    printf("last error %x\n", LE());
"#,
    );
    assert_eq!(
        l,
        [
            "cursor: region 1 aligned 1 kind 11 sign-extends 1",
            "info 1 0 5 7 32 32 1",
            "the bitmaps stay: kinds 7 7 selected 0 0",
            "monochrome icon (fIcon 7 is TRUE): info 1 1 1 2 32 64 0",
            "colour device-dependent bitmap: 1, distinct handles 1 1",
            "delete the guest's bitmaps 1 1 1",
            "last error 1234",
        ]
    );
}

#[test]
fn create_icon_indirect_of_what_the_page_leaves_open_stops_by_name() {
    stops_with(
        "w6-abort-icon-0108",
        "uint64_t sdc = SCR(), dc = MKDC(), mask = MKBMP(32, 32), col = CCB(sdc, 32, 32), gone = MKBMP(32, 32), c16 = CCB(sdc, 16, 16), colmask = CCB(sdc, 32, 32), odd = MKBMP(32, 31), sel_mask = MKBMP(32, 32), sel_col = CCB(sdc, 32, 32);\n    DEL(gone);\n    SELO(dc, sel_mask);",
        "CreateIconIndirect",
        &[
            ("null", "C4(fle_imp_CreateIconIndirect, 0, 0, 0, 0);", "piconinfo is NULL"),
            ("mask-null", "ICONI(0, 0, 0, 0, col);", "hbmMask is not a live bitmap"),
            ("mask-junk", "ICONI(0, 0, 0, 0x1234, col);", "hbmMask is not a live bitmap"),
            ("mask-deleted", "ICONI(0, 0, 0, gone, col);", "hbmMask is not a live bitmap"),
            ("mask-is-a-dc", "ICONI(0, 0, 0, dc, col);", "hbmMask is not a live bitmap"),
            ("mask-is-colour", "ICONI(0, 0, 0, colmask, col);", "monochrome"),
            ("mask-selected", "ICONI(0, 0, 0, sel_mask, 0);", "hbmMask is selected"),
            ("colour-junk", "ICONI(0, 0, 0, mask, 0x1234);", "hbmColor is not NULL and not a live bitmap"),
            ("colour-deleted", "ICONI(0, 0, 0, mask, gone);", "hbmColor is not NULL and not a live bitmap"),
            ("colour-other-size", "ICONI(0, 0, 0, mask, c16);", "differ in size"),
            ("odd-height-without-colour", "ICONI(1, 0, 0, odd, 0);", "odd"),
        ],
    );
    stops_with(
        "w6-abort-icon-sel-colour",
        "uint64_t sdc = SCR(), dc = MKDC(), mask = MKBMP(32, 32), sel_col = CCB(sdc, 32, 32);\n    SELO(dc, sel_col);",
        "CreateIconIndirect",
        &[("colour-selected", "ICONI(0, 0, 0, mask, sel_col);", "hbmColor is selected")],
    );
}

// ---- 12i. GetObjectW of a device-dependent bitmap ----

#[test]
fn get_object_of_a_device_dependent_bitmap_gives_the_bitmap_structure() {
    let l = w6_lines(
        "w6-ddb-getobject",
        r#"
    uint64_t sdc = SCR(), dc = MKDC(), m = MKBMP(8, 8), c = CCB(sdc, 5, 2), ph = fle_gdi_dc_placeholder(dc), r;
    uint32_t raw[16];
    uint8_t *buf = (uint8_t *)raw;
    unsigned i;
    memset(raw, 0xC7, sizeof raw);
    r = GOBJ(m, 32, A16(buf));
    bm32(buf, "monochrome 8x8", r);
    printf("beyond 32 bytes untouched %d\n", buf[32] == 0xC7 && buf[63] == 0xC7);
    memset(raw, 0xC7, sizeof raw);
    r = GOBJ(m, 0x68, A16(buf));
    printf("with c = sizeof(DIBSECTION) a device-dependent bitmap still stores 32 bytes: %llu, %d\n", U(r), buf[32] == 0xC7);
    memset(raw, 0xC7, sizeof raw);
    r = GOBJ(c, 32, A16(buf));
    bm32(buf, "colour 5x2", r);
    memset(raw, 0xC7, sizeof raw);
    r = GOBJ(ph, 32, A16(buf));
    bm32(buf, "placeholder", r);
    memset(raw, 0xC7, sizeof raw);
    r = GOBJ(m, 32, A16(buf + 1));
    for (i = 0; i < 40; i++) if (buf[i] != 0xC7) break;
    printf("not on a 4-byte boundary: returns %llu nothing stored %d\n", U(r), i == 40);
"#,
    );
    // widthbytes: ((w * planes * bits + 15) >> 4) << 1 (the CreateBitmap page): 8x1x1 -> 2; 5x1x32 -> (175 >> 4) << 1 = 20; 1x1x1 -> 2
    assert_eq!(
        l,
        [
            "monochrome 8x8: returns 32 type 0 w 8 h 8 widthbytes 2 planes 1 bits 1 bmBits 0",
            "beyond 32 bytes untouched 1",
            "with c = sizeof(DIBSECTION) a device-dependent bitmap still stores 32 bytes: 32, 1",
            "colour 5x2: returns 32 type 0 w 5 h 2 widthbytes 20 planes 1 bits 32 bmBits 0",
            "placeholder: returns 32 type 0 w 1 h 1 widthbytes 2 planes 1 bits 1 bmBits 0",
            "not on a 4-byte boundary: returns 0 nothing stored 1",
        ]
    );
    stops_with(
        "w6-abort-ddb-getobject",
        "uint64_t m = MKBMP(8, 8);\n    uint8_t buf[64];",
        "GetObjectW",
        &[("null-pv", "GOBJ(m, 32, 0);", "pv is NULL"), ("c-31", "GOBJ(m, 31, A16(buf));", "c is 0x1f, below sizeof(BITMAP) = 0x20"), ("c-0", "GOBJ(m, 0, A16(buf));", "c is 0x0, below sizeof(BITMAP)")],
    );
}

// ---- 12j. The five that stop by name ----

#[test]
fn bitblt_patblt_fillrect_getdibits_and_realizepalette_stop_by_name_with_their_arguments() {
    // decision 0126 (wall 55): old: stops_with("w6-abort-pixels", ..., "BitBlt", [("srccopy", BitBlt between two memory DCs with the 1x1 placeholder bitmaps, "rop 0xcc0020, 30 x 40 to (1, 2) from (3, 4)")]) stopped; a
    // device-dependent bitmap keeps pixels now, so the copy is made (the part outside the 1x1 source bitmap is not read: S-source-outside) and returns 1 (tests/wall_fr11.rs, tests/raster.rs). The call is no longer a stop.
    stops_with(
        "w6-abort-patblt",
        "uint64_t dc = MKDC();",
        "PatBlt",
        // decision 0126 (wall 55): old: ("whiteness", PatBlt(dc, 1, 2, 30, 40, WHITENESS) on a memory DC with the 1x1 placeholder bitmap) stopped ("rop 0xff0062, 30 x 40 at (1, 2)"); the bitmap keeps pixels now, WHITENESS
        // fills the part inside it and the call returns 1 (tests/wall_fr11.rs, tests/raster.rs). A rop that is not one of the 256 codes of the page still stops (pixel_stop).
        &[("not-a-ternary-code", "cN(fle_imp_PatBlt, dc, 1, 2, 30, 40, 0xFF0063, 0, 0, 0);", "rop 0xff0063, 30 x 40 at (1, 2)")],
    );
    stops_with(
        "w6-abort-fillrect",
        "uint64_t dc = MKDC();\n    uint32_t rc[4] = { 1, 2, 3, 4 };",
        "FillRect",
        &[("brush", "C4(fle_imp_FillRect, dc, A16(rc), 0x1234, 0);", "hbr 0x1234")],   // (a brush handle that was never issued still stops: unchanged by decision 0126)
    );
    stops_with(
        "w6-abort-getdibits",
        "uint64_t dc = MKDC(), b = MKBMP(8, 8);\n    uint8_t buf[64], info[64];",
        "GetDIBits",
        &[
            // decision 0126 (wall 55): old: the two forms stopped with "lpvBits given, start 3, cLines 5" and "lpvBits NULL, start 0, cLines 0" in the message because GetDIBits was a stop; GetDIBits answers now
            // (tests/wall_fr11.rs), so the message is that of a form it still stops on: `info` here is not a BITMAPINFOHEADER of size 40 (the buffer is uninitialised), and the bitmap `b` is the 8x8 DDB. The new text is
            // "GetDIBits of bitmap 0x... (start 3, cLines 5, lpvBits given, usage 0): biSize is not 40" (old words, new order)
            ("bits", "cN(fle_imp_GetDIBits, dc, b, 3, 5, A16(buf), A16(info), 0, 0, 0);", "start 3, cLines 5, lpvBits given"),
            ("query", "cN(fle_imp_GetDIBits, dc, b, 0, 0, 0, A16(info), 0, 0, 0);", "start 0, cLines 0, lpvBits NULL"),
        ],
    );
    stops_with("w6-abort-realize", "uint64_t dc = MKDC();", "RealizePalette", &[("memory-dc", "C4(fle_imp_RealizePalette, dc, 0, 0, 0);", "not given by the page")]);
}

// ---- 12j2. Handle lifetime ----

#[test]
fn the_handles_of_deleted_dcs_bitmaps_sections_and_brushes_are_never_reused() {
    // decision 0054 (a deleted slot is never reused) holds for the kinds of decision 0108 too
    let l = w6_lines(
        "w6-never-reused",
        r#"
    uint64_t sdc = SCR(), dc = MKDC(), dc2, b, b2, br, br2, d, d2, ph = fle_gdi_dc_placeholder(dc);
    b = MKBMP(4, 4); DEL(b); b2 = MKBMP(4, 4);
    br = mkbrush(0, 1, 0); DEL(br); br2 = mkbrush(0, 1, 0);
    mkbmi(2, 2, 32, 0, 0); d = DIB(sdc); DEL(d); d2 = DIB(sdc);
    DELDC(dc); dc2 = MKDC();
    printf("distinct %d %d %d %d\n", b2 != b, br2 != br, d2 != d, dc2 != dc);
    printf("slots increase %d %d %d %d\n", SLOT(b2) > SLOT(b), SLOT(br2) > SLOT(br), SLOT(d2) > SLOT(d), SLOT(dc2) > SLOT(dc));
    printf("the deleted handles stay invalid: bitmap %d brush %d section %d dc %d placeholder %d\n", KIND(b), KIND(br), KIND(d), KIND(dc), KIND(ph));
    printf("and are not the placeholder of the new dc: %d\n", fle_gdi_dc_placeholder(dc2) != ph);
    printf("a deleted handle is not deleted twice: %u %u %u\n", DEL(b), DEL(br), DEL(d));
"#,
    );
    assert_eq!(l, ["distinct 1 1 1 1", "slots increase 1 1 1 1", "the deleted handles stay invalid: bitmap 0 brush 0 section 0 dc 0 placeholder 0", "and are not the placeholder of the new dc: 1", "a deleted handle is not deleted twice: 0 0 0"]);
    stops_with(
        "w6-abort-deleted-handles",
        "uint64_t sdc = SCR(), dc = MKDC(), b = MKBMP(4, 4), br = mkbrush(0, 1, 0), f;\n    mklf(1); f = MKFONT();\n    DEL(b); DEL(br); DEL(f);",
        "SelectObject",
        &[("deleted-bitmap", "SELO(dc, b);", "h was deleted"), ("deleted-brush", "SELO(dc, br);", "h was deleted"), ("deleted-font", "SELO(dc, f);", "h was deleted")],
    );
}

// ---- 12k. Threads, the generator, the source ----

#[test]
fn a_second_thread_stops_the_new_calls_too_and_get_sys_color_has_no_thread_rule() {
    for (label, api, call) in [
        ("createcompatibledc", "CreateCompatibleDC", "C4(fle_imp_CreateCompatibleDC, 0, 0, 0, 0)"),
        ("deletedc", "DeleteDC", "C4(fle_imp_DeleteDC, 0, 0, 0, 0)"),
        ("selectobject", "SelectObject", "C4(fle_imp_SelectObject, 0, 0, 0, 0)"),
        ("selectpalette", "SelectPalette", "C4(fle_imp_SelectPalette, 0, 0, 0, 0)"),
        ("createcompatiblebitmap", "CreateCompatibleBitmap", "C4(fle_imp_CreateCompatibleBitmap, 0, 0, 0, 0)"),
        ("createdibsection", "CreateDIBSection", "C4(fle_imp_CreateDIBSection, 0, 0, 0, 0)"),
        ("getdibcolortable", "GetDIBColorTable", "C4(fle_imp_GetDIBColorTable, 0, 0, 0, 0)"),
        ("setdibcolortable", "SetDIBColorTable", "C4(fle_imp_SetDIBColorTable, 0, 0, 0, 0)"),
        ("settextcolor", "SetTextColor", "C4(fle_imp_SetTextColor, 0, 0, 0, 0)"),
        ("setbkcolor", "SetBkColor", "C4(fle_imp_SetBkColor, 0, 0, 0, 0)"),
        ("createbrushindirect", "CreateBrushIndirect", "C4(fle_imp_CreateBrushIndirect, 0, 0, 0, 0)"),
        ("createpatternbrush", "CreatePatternBrush", "C4(fle_imp_CreatePatternBrush, 0, 0, 0, 0)"),
        ("createhalftonepalette", "CreateHalftonePalette", "C4(fle_imp_CreateHalftonePalette, 0, 0, 0, 0)"),
        ("createiconindirect", "CreateIconIndirect", "C4(fle_imp_CreateIconIndirect, 0, 0, 0, 0)"),
        ("bitblt", "BitBlt", "C4(fle_imp_BitBlt, 0, 0, 0, 0)"),
        ("patblt", "PatBlt", "C4(fle_imp_PatBlt, 0, 0, 0, 0)"),
        ("fillrect", "FillRect", "C4(fle_imp_FillRect, 0, 0, 0, 0)"),
        ("getdibits", "GetDIBits", "C4(fle_imp_GetDIBits, 0, 0, 0, 0)"),
        ("realizepalette", "RealizePalette", "C4(fle_imp_RealizePalette, 0, 0, 0, 0)"),
    ] {
        let top = W6_TOP.to_string() + &format!("\nstatic void *th(void *x) {{ (void)x; {call}; printf(\"returned-from-thread\\n\"); return 0; }}");
        let o = run_src(
            &format!("w6-thread-{label}"),
            &top,
            "    pthread_t t;\n    STOCK(5);\n    pthread_create(&t, 0, th, 0);\n    pthread_join(t, 0);\n    printf(\"returned\\n\");",
        );
        assert!(!o.ok && !o.stdout.contains("returned"), "{label}: {}", o.stdout);
        assert_eq!(o.signal, Some(6), "{label}");
        assert!(o.stderr.contains(&format!("fle: {api}:")) && o.stderr.contains("second thread"), "{label}: {}", o.stderr);
    }
    // user32!GetSysColor keeps no state: no thread rule
    let top = W6_TOP.to_string() + "\nstatic void *th(void *x) { (void)x; printf(\"in-thread %llu\\n\", U(C4(fle_imp_GetSysColor, 8, 0, 0, 0))); return 0; }";
    let o = run_src("w6-thread-getsyscolor", &top, "    pthread_t t;\n    STOCK(5);\n    pthread_create(&t, 0, th, 0);\n    pthread_join(t, 0);");
    assert!(o.ok && o.stdout == "in-thread 0\n", "{} {}", o.stdout, o.stderr);
}

#[test]
fn the_thunks_of_the_nineteen_apis_call_the_runtime_and_the_dll_name_is_part_of_the_match() {
    for (dll, api) in ROWS_0108 {
        let img = build_one(dll, api);
        let mut out = Vec::new();
        let st = generate(&img, &mut out).unwrap();
        let c = String::from_utf8(out).unwrap();
        assert_eq!(st.handled[format!("jmp_imp:{api}").as_str()], 1, "{dll}!{api}");
        assert!(c.contains(&format!("fle_imp_{api}(); return;")), "{dll}!{api}");
        let va = IB + 0x1040;
        assert!(!c.contains(&format!("L_{va:x}: {{ /* {va:#x} Jmp */ fle_unsupported")), "{dll}!{api} still unsupported");
        // the other DLL of the pair, and the same name from an unrelated DLL, are not this API
        let other = if dll == "gdi32.dll" { "user32.dll" } else { "gdi32.dll" };
        for wrong in [other, "other.dll"] {
            let img = build_one(wrong, api);
            let mut out = Vec::new();
            let st = generate(&img, &mut out).unwrap();
            let c = String::from_utf8(out).unwrap();
            assert!(!st.handled.contains_key(format!("jmp_imp:{api}").as_str()), "{wrong}!{api}");
            assert!(!c.contains(&format!("fle_imp_{api}")), "{wrong}!{api}");
            assert!(c.contains(&format!("L_{va:x}: {{ /* {va:#x} Jmp */ fle_unsupported(UINT64_C({va:#x})); }}")), "{wrong}!{api}");
        }
    }
}

#[test]
fn the_neighbours_that_decision_0108_does_not_answer_stay_unsupported_thunks() {
    // (decision 0111 gives StretchBlt, GetPaletteEntries, DestroyCursor and SetBkMode rows: they were in this list until then)
    // decision 0126 (wall 55): old: ("gdi32.dll", "CreatePen") was here; it is a row now
    for (dll, api) in [("gdi32.dll", "GetCurrentObject"), ("gdi32.dll", "TextOutW"), ("gdi32.dll", "GetBkMode")] {
        let img = build_one(dll, api);
        let mut out = Vec::new();
        let st = generate(&img, &mut out).unwrap();
        let c = String::from_utf8(out).unwrap();
        assert!(!c.contains(&format!("fle_imp_{api}")), "{dll}!{api}");
        let va = IB + 0x1040;
        assert!(c.contains(&format!("L_{va:x}: {{ /* {va:#x} Jmp */ fle_unsupported(UINT64_C({va:#x})); }}")), "{dll}!{api}");
        assert_eq!(st.handled.keys().filter(|k| k.starts_with("jmp_imp:")).count(), 0, "{dll}!{api}");
    }
}

#[test]
fn the_source_of_decision_0108_pins_the_rules_that_the_c_tests_cannot_see() {
    let code = strip_comments(&std::fs::read_to_string(src_dir().join("fle_gdi.c")).unwrap());
    // decision 0126 (wall 55): old: "GetDIBits and RealizePalette still stop through fle_stop and touch no pixel": GetDIBits answers now (it reads a device-dependent bitmap into a DIB: it stops through pixel_stop for the forms
    // the page leaves open and writes the lines through fle_st; tests/wall_fr11.rs); RealizePalette still stops through fle_stop and touches no pixel
    {
        let a = code.find("void fle_imp_RealizePalette(void)").expect("RealizePalette");
        let end = a + code[a..].find("\n}\n").unwrap();
        let body = &code[a..end];
        assert!(body.contains("fle_stop(api"), "RealizePalette stops by name");
        assert!(!body.contains("fle_st(") && !body.contains("memcpy") && !body.contains("memset") && !body.contains("->bits"), "RealizePalette touches no pixel and stores nothing");
    }
    {
        let a = code.find("void fle_imp_GetDIBits(void)").expect("GetDIBits");
        let end = a + code[a..].find("\n}\n").unwrap();
        assert!(code[a..end].contains("pixel_stop(api") && code[a..end].contains("ddb_arg("), "GetDIBits stops by name through pixel_stop on the forms the page leaves open, and reads a DDB through ddb_arg");
    }
    // decision 0126 (old: BitBlt, PatBlt and FillRect "do not store through the guest memory interface" and "stop by name": the three write pixels now, into the surfaces of fle_raster.c, and PatBlt and FillRect keep one
    // pixel_stop for a rop or a brush the page does not fix; BitBlt hands its arguments to blit_call, which stops through pixel_stop). They still do not store through the guest memory interface (fle_st / memcpy / memset / ->bits).
    for api in ["BitBlt", "PatBlt", "FillRect"] {
        let a = code.find(&format!("void fle_imp_{api}(void)")).unwrap_or_else(|| panic!("{api}"));
        let end = a + code[a..].find("\n}\n").unwrap();
        let body = &code[a..end];
        assert!(body.contains("pixel_stop(api") || body.contains("blit_call(api"), "{api} stops by name");
        assert!(!body.contains("fle_st(") && !body.contains("memcpy") && !body.contains("memset") && !body.contains("->bits"), "{api} does not store through the guest memory interface");
    }
    // decision 0126 (old: `calloc(1, (size_t)bytes)` appeared once, in CreateDIBSection): the block of a device-dependent bitmap (S-ddb-lazy) is the second calloc of that form; the pixels are written by fle_raster.c, never by
    // indexing `bits` here
    assert_eq!(code.matches("calloc(1, (size_t)bytes)").count(), 2);
    assert!(!code.contains("->bits[") && !code.contains("*(uint8_t *)"), "the bits are never indexed or stored through by the runtime");
    // the screen DC table is the one that tells a screen DC from a memory DC (section 6 tests the two regions against each other)
    assert!(code.contains("#include \"fle_gdc.h\"") && code.contains("fle_gdc_is_live("));
}
