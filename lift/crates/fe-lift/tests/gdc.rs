//! Tests for the virtual screen DC of runtime/fle_gdc.c: user32!GetDC(NULL), gdi32!GetDeviceCaps, user32!ReleaseDC
//! (docs/decisions/0053-virtual-screen-dc.md; sources: docs/investigations/getdc-caller.md).
//!
//! Expected values are written here from the decision (the rules on the Microsoft Learn pages GetDC, ReleaseDC, GetDeviceCaps,
//! and the fixed values of the virtual display), not read back from the C code. A number the pages do not give (the screen
//! size, the dots per inch, the colour depth, the handle value, the index numbers) is marked "choice" or "unofficial" and is the
//! only thing such a test pins down. The host is Linux: nothing is compared with a real Windows; no display, DPI setting,
//! window, clock, thread or process is read.
//!
//! The C sources of fle_gdc.c / fle_gdc.h come from runtime/ when fle_gdc.c is there, otherwise from out-gdc/work/
//! (development). FLE_GDC_SRC_DIR (a directory with fle_gdc.c and fle_gdc.h) overrides both: the mutation runs of the run
//! record use it. Files that already existed always come from runtime/.

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
    if let Ok(d) = std::env::var("FLE_GDC_SRC_DIR") {
        return PathBuf::from(d);
    }
    if runtime_dir().join("fle_gdc.c").exists() {
        runtime_dir()
    } else {
        manifest().join("../../out-gdc/work")
    }
}
fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-gdc/test");
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

/// fle_gdc.c (warnings are errors) and the two existing files it links with (fle_stop, and GetLastError / fle_set_last_error).
fn objs() -> &'static Vec<PathBuf> {
    static O: OnceLock<Vec<PathBuf>> = OnceLock::new();
    O.get_or_init(|| {
        let dir = work_dir().join(etxt_common::per_process("obj-gdc"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut v = Vec::new();
        for (name, ours) in [("fle_gdc.c", true), ("fle_imports.c", false), ("fle_env.c", false)] {
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
#define GETDC() CALL(fle_imp_GetDC, 0)
#define RELEASE(dc) ((uint32_t)CALL(fle_imp_ReleaseDC, 0, (dc)))
#define CAPS(dc, idx) ((int32_t)(uint32_t)CALL(fle_imp_GetDeviceCaps, (dc), (idx)))
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

/// The value of `#define NAME <number>` (decimal, or UINT64_C(0x..)) in a runtime header.
fn def_const(file: &str, name: &str) -> u64 {
    let t = std::fs::read_to_string(src_dir().join(file)).or_else(|_| std::fs::read_to_string(runtime_dir().join(file))).unwrap();
    let line = t.lines().find(|l| l.starts_with(&format!("#define {name} "))).unwrap_or_else(|| panic!("{name} not in {file}"));
    let rest = line[format!("#define {name} ").len()..].trim_start();
    if let Some(r) = rest.strip_prefix("UINT64_C(") {
        let tok = r.split(')').next().unwrap();
        return u64::from_str_radix(tok.trim_start_matches("0x"), 16).unwrap();
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

const ROWS: [(&str, &str); 3] = [("user32.dll", "GetDC"), ("user32.dll", "ReleaseDC"), ("gdi32.dll", "GetDeviceCaps")];

#[test]
fn rows_are_in_the_table_and_declared_in_the_header() {
    let table = std::fs::read_to_string(manifest().join("src/imports_table.rs")).unwrap();
    let h = std::fs::read_to_string(src_dir().join("fle_gdc.h")).unwrap();
    for (dll, n) in ROWS {
        let row = format!("(\"{dll}\", \"{n}\", \"fle_imp_{n}\", \"jmp_imp:{n}\")");
        assert_eq!(table.matches(&row).count(), 1, "{n}: exactly one row in imports_table.rs");
        assert!(h.contains(&format!("void fle_imp_{n}(void)")), "{n} is declared in fle_gdc.h");
    }
    // the APIs that need a window or a DC with selection state are not answered (decision 0053): no rows. GetStockObject,
    // CreatePalette and LoadIconW were in this list until decision 0054 (runtime/fle_gdi.c, tests/gdi.rs) answered them.
    // GetSystemMetrics was in this list until decision 0057 (runtime/fle_sysmetrics.c, tests/sysmetrics.rs) answered it.
    // (CreateCompatibleDC and SelectObject were in this list until decision 0108 answered them: runtime/fle_gdi.c, tests/gdi.rs section 12)
    // (GetTextMetricsW and GetSystemPaletteEntries were in this list until decision 0111 gave them rows that stop by name: runtime/fle_gdi.c, tests/gdi_bulk.rs)
    // (GetDCEx and GetWindowDC were in this list until decision 0125 gave them rows: runtime/fle_gdc.c, tests/wall_fr10.rs; old: no row for either)
    for n in ["GetDCEx", "GetWindowDC"] {
        assert_eq!(table.matches(&format!("\"{n}\"")).count(), 1, "{n} is a row of decision 0125");
    }
}

#[test]
fn the_decided_values_and_index_numbers() {
    // choices of decision 0053 (the Learn pages give none of them)
    assert_eq!(def_const("fle_gdc.h", "FLE_GDC_HORZRES"), 1920);
    assert_eq!(def_const("fle_gdc.h", "FLE_GDC_VERTRES"), 1080);
    assert_eq!(def_const("fle_gdc.h", "FLE_GDC_BITSPIXEL"), 32);
    assert_eq!(def_const("fle_gdc.h", "FLE_GDC_PLANES"), 1);
    assert_eq!(def_const("fle_gdc.h", "FLE_GDC_LOGPIXELSX"), 96);
    assert_eq!(def_const("fle_gdc.h", "FLE_GDC_LOGPIXELSY"), 96);
    assert_eq!(def_const("fle_gdc.h", "FLE_GDC_SIZEPALETTE"), 0);
    // index numbers: unofficial (mingw-w64 wingdi.h, windows-sys on docs.rs, the same in ReactOS and Wine); written here, not read
    // back from the header
    for (n, v) in [("HORZSIZE", 4), ("VERTSIZE", 6), ("HORZRES", 8), ("VERTRES", 10), ("BITSPIXEL", 12), ("PLANES", 14), ("LOGPIXELSX", 88), ("LOGPIXELSY", 90), ("SIZEPALETTE", 104)] {
        assert_eq!(def_const("fle_gdc.h", &format!("FLE_GDC_IDX_{n}")), v, "{n}");
    }
}

#[test]
fn the_handle_region_overlaps_nothing_and_survives_32_bit_truncation() {
    let base = def_const("fle_gdc.h", "FLE_GDC_HANDLE_BASE");
    let size = def_const("fle_gdc.h", "FLE_GDC_REGION_SIZE");
    assert_eq!(base, 0x7FA9_0000);
    assert_eq!(size, 0x10_0000);
    // Learn, "Interprocess communication" (64-bit Windows): only the lower 32 bits of a GDI handle are significant, and it may be
    // truncated and sign-extended. Below 0x80000000 both give the same value.
    assert!(base + size <= 0x8000_0000);
    // above the image (ImageBase 0x400000, SizeOfImage 0x18C0000)
    assert!(base > 0x400000 + 0x18C0000);
    // the other handle regions of the runtime are 0x7FF7.. and above
    assert!(base + size < 0x7FF0_0000_0000);
}

#[test]
fn the_values_are_the_decided_ones_for_the_seven_indexes() {
    let l = lines(
        "gdc-values",
        r#"
    uint64_t dc = GETDC();
    printf("dc %d\n", dc != 0);
    printf("horzres %d\n", CAPS(dc, 8));
    printf("vertres %d\n", CAPS(dc, 10));
    printf("bitspixel %d\n", CAPS(dc, 12));
    printf("planes %d\n", CAPS(dc, 14));
    printf("logpixelsx %d\n", CAPS(dc, 88));
    printf("logpixelsy %d\n", CAPS(dc, 90));
    printf("sizepalette %d\n", CAPS(dc, 104));
    /* the same answer on every call and on another DC */
    uint64_t dc2 = GETDC();
    printf("again %d %d\n", CAPS(dc, 8), CAPS(dc2, 8));
"#,
    );
    assert_eq!(
        l,
        ["dc 1", "horzres 1920", "vertres 1080", "bitspixel 32", "planes 1", "logpixelsx 96", "logpixelsy 96", "sizepalette 0", "again 1920 1920"]
    );
}

#[test]
fn only_the_low_dword_of_the_index_counts() {
    // GetDeviceCaps(HDC, int): a 32-bit argument; the guest may leave anything in the upper half of rdx
    let l = lines(
        "gdc-index-dword",
        r#"
    uint64_t dc = GETDC();
    printf("%d %d\n", (int32_t)(uint32_t)CALL(fle_imp_GetDeviceCaps, dc, 0xAAAAAAAA00000008ull), (int32_t)(uint32_t)CALL(fle_imp_GetDeviceCaps, dc, 0xFFFFFFFF00000058ull));
"#,
    );
    assert_eq!(l, ["1920 96"]);
}

// ---- 2. GetDC / ReleaseDC ----

#[test]
fn handles_are_in_the_region_distinct_never_reused_and_leave_the_last_error_alone() {
    let l = lines(
        "gdc-handles",
        r#"
    uint64_t a, b, c;
    SETLE(0x1234);
    a = GETDC(); b = GETDC();
    printf("region %d %d\n", a > FLE_GDC_HANDLE_BASE && a < FLE_GDC_HANDLE_BASE + FLE_GDC_REGION_SIZE, b > FLE_GDC_HANDLE_BASE && b < FLE_GDC_HANDLE_BASE + FLE_GDC_REGION_SIZE);
    printf("distinct %d aligned %d\n", a != b, (a % 4) == 0);
    printf("live %llu\n", U(fle_gdc_live_count()));
    printf("release %u\n", RELEASE(a));
    printf("live %llu\n", U(fle_gdc_live_count()));
    c = GETDC();
    printf("not reused %d\n", c != a && c != b);
    printf("sign-extends %d\n", (uint64_t)(int64_t)(int32_t)(uint32_t)a == a);
    printf("last error %x\n", LE());
    printf("release b, c %u %u\n", RELEASE(b), RELEASE(c));
    printf("live %llu\n", U(fle_gdc_live_count()));
"#,
    );
    assert_eq!(
        l,
        ["region 1 1", "distinct 1 aligned 1", "live 2", "release 1", "live 1", "not reused 1", "sign-extends 1", "last error 1234", "release b, c 1 1", "live 0"]
    );
}

#[test]
fn release_results_follow_the_page() {
    // ReleaseDC page: 1 when the DC was released, 0 when it was not
    let l = lines(
        "gdc-release",
        r#"
    uint64_t dc = GETDC();
    printf("first %u\n", RELEASE(dc));
    printf("again %u\n", RELEASE(dc));
    printf("null %u\n", RELEASE(0));
    printf("live %llu is_live %d\n", U(fle_gdc_live_count()), fle_gdc_is_live(dc));
"#,
    );
    assert_eq!(l, ["first 1", "again 0", "null 0", "live 0 is_live 0"]);
}

#[test]
fn get_dc_with_a_window_stops_the_run() {
    assert_aborts(
        "gdc-abort-getdc-hwnd",
        "",
        "GetDC",
        &["hWnd is not NULL", "window"],
        &[("small", "CALL(fle_imp_GetDC, 0x10001);"), ("pointer-like", "CALL(fle_imp_GetDC, 0x7FF700001000ull);"), ("high-bit-only", "CALL(fle_imp_GetDC, 0x100000000ull);"), ("invalid-handle-value", "CALL(fle_imp_GetDC, 0xFFFFFFFFFFFFFFFFull);")],
    );
}

#[test]
fn release_dc_with_a_window_or_a_foreign_handle_stops_the_run() {
    assert_aborts(
        "gdc-abort-release",
        "uint64_t dc = GETDC();",
        "ReleaseDC",
        &[],
        &[
            ("hwnd", "CALL(fle_imp_ReleaseDC, 0x10001, dc);"),
            ("foreign", "CALL(fle_imp_ReleaseDC, 0, 0x1234);"),
            ("event-region", "CALL(fle_imp_ReleaseDC, 0, 0x7FFA80000004ull);"),
            ("never-issued-in-region", "CALL(fle_imp_ReleaseDC, 0, FLE_GDC_HANDLE_BASE + 4 * 900);"),
            ("region-base", "CALL(fle_imp_ReleaseDC, 0, FLE_GDC_HANDLE_BASE);"),
            ("unaligned-in-region", "CALL(fle_imp_ReleaseDC, 0, dc + 1);"),
            ("region-end", "CALL(fle_imp_ReleaseDC, 0, FLE_GDC_HANDLE_BASE + FLE_GDC_REGION_SIZE);"),
            ("invalid-handle-value", "CALL(fle_imp_ReleaseDC, 0, 0xFFFFFFFFFFFFFFFFull);"),
        ],
    );
}

// ---- 3. GetDeviceCaps stops ----

#[test]
fn indexes_the_guest_does_not_use_stop_by_number() {
    // 0 DRIVERVERSION, 2 TECHNOLOGY, 16 NUMBRUSHES, 18 NUMPENS, 22 NUMFONTS, 24 NUMCOLORS, 36 CLIPCAPS, 38 RASTERCAPS,
    // 40..44 ASPECT*, 106 NUMRESERVED, 108 COLORRES, 116 VREFRESH, 120 SHADEBLENDCAPS, 5 (not an index), a negative value.
    let cases: Vec<(String, String)> = [0u32, 2, 16, 18, 22, 24, 36, 38, 40, 42, 44, 106, 108, 116, 120, 5, 9, 13, 87, 89, 91, 103, 105, 0x7FFFFFFF, 0x80000000, 0xFFFFFFFF]
        .iter()
        .map(|i| (format!("idx-{i}"), format!("CALL(fle_imp_GetDeviceCaps, dc, {i});")))
        .collect();
    let refs: Vec<(&str, &str)> = cases.iter().map(|(l, c)| (l.as_str(), c.as_str())).collect();
    assert_aborts("gdc-abort-index", "uint64_t dc = GETDC();", "GetDeviceCaps", &["is not answered"], &refs);
    // the message names the number (as an unsigned value)
    let o = run("gdc-abort-index-text", "uint64_t dc = GETDC(); CALL(fle_imp_GetDeviceCaps, dc, 38);");
    assert!(o.stderr.contains("index 38 is not answered"), "{}", o.stderr);
}

#[test]
fn horzsize_and_vertsize_are_derived_from_the_virtual_display_at_96_dpi() {
    // decision 0121 (PROVISIONAL; the PM's decision 1; was: "stop by name, not decided"). The guest asks both (call sites 0x5c8d3a, 0x5c8df8, ...). Learn GetDeviceCaps: HORZSIZE "Width, in
    // millimeters, of the physical screen", VERTSIZE "Height, in millimeters, of the physical screen"; no value. The values are derived, not read from the host: 1920 px * 25.4 mm/inch /
    // 96 px/inch = 508 mm exactly; 1080 px * 25.4 / 96 = 285.75 mm, rounded to the nearest millimetre = 286 (the rounding is a choice without a source). Expected values written here
    // from that arithmetic, in integers (so that this test does not use the C constants it checks): 1920 * 254 = 487680 = 508 * 960; 1080 * 254 = 274320 = 285.75 * 960.
    assert_eq!(1920u64 * 254 % (96 * 10), 0, "508 is exact");
    assert_eq!(1920u64 * 254 / (96 * 10), 508);
    assert_eq!(1080u64 * 254 * 100 / (96 * 10), 28575, "285.75 mm in hundredths");
    let l = lines(
        "gdc-size",
        r#"
    uint64_t dc = GETDC();
    printf("horzsize %d vertsize %d\n", CAPS(dc, 4), CAPS(dc, 6));
    printf("with the pixel sizes: %d x %d px at %d x %d dpi\n", CAPS(dc, 8), CAPS(dc, 10), CAPS(dc, 88), CAPS(dc, 90));
    RELEASE(dc);
    dc = GETDC();
    printf("new dc: %d %d\n", CAPS(dc, 4), CAPS(dc, 6));
"#,
    );
    assert_eq!(l, ["horzsize 508 vertsize 286", "with the pixel sizes: 1920 x 1080 px at 96 x 96 dpi", "new dc: 508 286"]);
    // the two numbers of the header agree with the arithmetic above
    assert_eq!(def_const("fle_gdc.h", "FLE_GDC_HORZSIZE"), 508);
    assert_eq!(def_const("fle_gdc.h", "FLE_GDC_VERTSIZE"), 286);
    // a released DC and NULL still stop for these indexes, as for every index (the DC is not live)
    assert_aborts("gdc-abort-size-released", "uint64_t dc = GETDC(); RELEASE(dc);", "GetDeviceCaps", &["hdc is not a live DC"], &[("horz", "CALL(fle_imp_GetDeviceCaps, dc, 4);"), ("vert", "CALL(fle_imp_GetDeviceCaps, dc, 6);")]);
    assert_aborts("gdc-abort-size-null", "", "GetDeviceCaps", &["hdc is not a live DC"], &[("horz", "CALL(fle_imp_GetDeviceCaps, 0, 4);"), ("vert", "CALL(fle_imp_GetDeviceCaps, 0, 6);")]);
}

#[test]
fn caps_on_a_dc_that_is_not_live_stops_the_run() {
    assert_aborts(
        "gdc-abort-caps-hdc",
        "uint64_t dc = GETDC(); RELEASE(dc);",
        "GetDeviceCaps",
        &["hdc is not a live DC"],
        &[
            ("released", "CALL(fle_imp_GetDeviceCaps, dc, 8);"),
            ("null", "CALL(fle_imp_GetDeviceCaps, 0, 8);"),
            ("foreign", "CALL(fle_imp_GetDeviceCaps, 0x1234, 8);"),
            ("never-issued-in-region", "CALL(fle_imp_GetDeviceCaps, FLE_GDC_HANDLE_BASE + 4 * 900, 8);"),
            ("invalid-handle-value", "CALL(fle_imp_GetDeviceCaps, 0xFFFFFFFFFFFFFFFFull, 8);"),
        ],
    );
}

#[test]
fn a_second_thread_stops_the_run() {
    for (label, api, call) in [
        ("getdc", "GetDC", "CALL(fle_imp_GetDC, 0)"),
        ("release", "ReleaseDC", "CALL(fle_imp_ReleaseDC, 0, dc)"),
        ("caps", "GetDeviceCaps", "CALL(fle_imp_GetDeviceCaps, dc, 8)"),
    ] {
        let o = run_src(
            &format!("gdc-thread-{label}"),
            &format!("static uint64_t dc; static void *th(void *x) {{ (void)x; {call}; printf(\"returned-from-thread\\n\"); return 0; }}"),
            "    pthread_t t;\n    dc = GETDC();\n    pthread_create(&t, 0, th, 0);\n    pthread_join(t, 0);\n    printf(\"returned\\n\");",
        );
        assert!(!o.ok && !o.stdout.contains("returned"), "{label}: {}", o.stdout);
        assert_eq!(o.signal, Some(6), "{label}");
        assert!(o.stderr.contains(&format!("fle: {api}")) && o.stderr.contains("second thread"), "{label}: {}", o.stderr);
    }
}

// ---- 4. Source checks ----

#[test]
fn source_uses_no_host_display_window_clock_thread_process_or_file() {
    let code = strip_comments(&std::fs::read_to_string(src_dir().join("fle_gdc.c")).unwrap());
    for bad in [
        "getpid", "getppid", "gettid", "getenv", "environ", "fopen", "open(", "stat(", "/proc", "/etc", "/home", "/dev", "/tmp", "exit(", "_exit",
        "clock_gettime", "gettimeofday", "time(", "clock(", "nanosleep", "usleep", "sleep(", "select(", "poll(", "<time.h>", "<sys/time.h>",
        "pthread", "<sys/", "<unistd.h>", "<signal.h>", "fork(", "exec", "syscall", "sched_", "futex", "asm", "__asm__", "uintptr_t", "abort(",
        "X11", "DISPLAY", "xrandr", "wayland", "fb0", "ioctl", "emscripten", "SDL", "screen.width", "devicePixelRatio",
    ] {
        assert!(!code.contains(bad), "fle_gdc.c must not use {bad}");
    }
    // `long` is not portable (32 bits on wasm32 and on Windows, 64 on Linux): fixed-width integers only
    let b = code.as_bytes();
    let mut i = 0;
    while let Some(p) = code[i..].find("long") {
        let at = i + p;
        let before = at == 0 || !(b[at - 1].is_ascii_alphanumeric() || b[at - 1] == b'_');
        let after = at + 4 >= b.len() || !(b[at + 4].is_ascii_alphanumeric() || b[at + 4] == b'_');
        assert!(!(before && after), "fle_gdc.c uses `long` near byte {at}");
        i = at + 4;
    }
    // the host thread id comes from the portability layer (decision 0045), and fle_stop is the stop
    assert!(code.contains("fle_host_tid()") && code.contains("fle_stop("));
    assert!(code.contains("#include \"fle_host.h\""));
}

#[test]
fn header_is_plain_c() {
    let h = strip_comments(&std::fs::read_to_string(src_dir().join("fle_gdc.h")).unwrap());
    for bad in ["asm", "__asm__", "<unistd.h>", "<sys/", "<pthread.h>", "<time.h>", "long "] {
        assert!(!h.contains(bad), "fle_gdc.h must not contain {bad}");
    }
}

// ---- 5. Generator: the three thunks call the runtime ----

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

/// A PE that imports user32.dll!GetDC, user32.dll!ReleaseDC (slots 0x2200, 0x2208) and gdi32.dll!GetDeviceCaps (slot 0x2240);
/// `other` replaces the DLL name of the GetDeviceCaps import. Entry 0x1000 calls the thunks at 0x1040, 0x1050, 0x1060.
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
    put(&mut b, 0x2400, b"\0\0GetDC\0");
    put(&mut b, 0x2420, b"\0\0ReleaseDC\0");
    put(&mut b, 0x2440, b"\0\0GetDeviceCaps\0");
    for (rva, v) in [(0x2100, 0x2400u64), (0x2108, 0x2420), (0x2200, 0x2400), (0x2208, 0x2420), (0x2140, 0x2440), (0x2240, 0x2440)] {
        w64(&mut b, data_at(rva), v);
    }

    let t = TEXT_RAW;
    let code: &[(u32, Vec<u8>)] = &[
        (0x1000, [&[0xe8][..], &rel(0x1005, 0x1040)].concat()),
        (0x1005, [&[0xe8][..], &rel(0x100a, 0x1050)].concat()),
        (0x100a, [&[0xe8][..], &rel(0x100f, 0x1060)].concat()),
        (0x100f, vec![0xc3]),
        (0x1040, [&[0xff, 0x25][..], &rel(0x1046, 0x2200)].concat()),
        (0x1050, [&[0xff, 0x25][..], &rel(0x1056, 0x2208)].concat()),
        (0x1060, [&[0xff, 0x25][..], &rel(0x1066, 0x2240)].concat()),
    ];
    for (rva, bytes) in code {
        let o = t + (rva - TEXT_RVA) as usize;
        b[o..o + bytes.len()].copy_from_slice(bytes);
    }
    b
}

#[test]
fn the_three_thunks_call_the_runtime() {
    let img = build("gdi32.dll");
    let mut out = Vec::new();
    let st = generate(&img, &mut out).unwrap();
    let c = String::from_utf8(out).unwrap();

    for name in ["GetDC", "ReleaseDC", "GetDeviceCaps"] {
        assert_eq!(st.handled[format!("jmp_imp:{name}").as_str()], 1, "{name}");
        assert!(c.contains(&format!("fle_imp_{name}(); return;")), "{name}");
    }
    for (va, _) in [(IB + 0x1040, 0), (IB + 0x1050, 0), (IB + 0x1060, 0)] {
        assert!(!c.contains(&format!("L_{va:x}: {{ /* {va:#x} Jmp */ fle_unsupported")), "{va:#x} still unsupported");
    }
    assert_eq!(st.unsupported_total, 0, "{:?}", st.unsupported);
}

#[test]
fn the_dll_name_is_part_of_the_match() {
    // the same API name from another DLL is not this API: it stays unsupported
    let img = build("other.dll");
    let mut out = Vec::new();
    let st = generate(&img, &mut out).unwrap();
    let c = String::from_utf8(out).unwrap();
    assert_eq!(st.handled["jmp_imp:GetDC"], 1);
    assert_eq!(st.handled["jmp_imp:ReleaseDC"], 1);
    assert!(!st.handled.contains_key("jmp_imp:GetDeviceCaps"));
    assert!(!c.contains("fle_imp_GetDeviceCaps"));
    let va = IB + 0x1060;
    assert!(c.contains(&format!("L_{va:x}: {{ /* {va:#x} Jmp */ fle_unsupported(UINT64_C({va:#x})); }}")));
}
