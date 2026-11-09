//! Tests of decision 0209 (docs/0209-crash-fixes.md, PROVISIONAL): the walls behind the stops that the client's browser reported.
//!
//!  1. kernel32!LCMapStringW (runtime/fle_locale.c with runtime/fle_crash1_lcmap.c): the Learn pages of LCMapStringW and LCMapStringEx (read 2026-11-04) define the mapping, the sizes and the errors; the case table outside the
//!     range D of decision 0087 is the Unicode Character Database 16.0.0 simple case mapping (UNOFFICIAL for Windows: no Microsoft page gives it). The expected values are written here from the pages and from the UCD, not read
//!     back from the implementation. The first test compares the run table with every code point of lift/runtime/test/case_ucd16_simple.tsv (the generator's own output) for all 0x110000 code points.
//!
//! The C programs call the `fle_imp_*` functions the way the lifted code reaches them: arguments in R[1], R[2], R[8], R[9] and on a fake stack, the result in R[0]. EVERY ANSWER IS PROVISIONAL (nothing is compared with a real Windows).

mod etxt_common;

use std::path::PathBuf;
use std::process::Command;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn rt() -> PathBuf {
    match std::env::var_os("FLE_CRASH1_RT_DIR") {
        Some(d) => PathBuf::from(d),
        None => manifest().join("../../runtime"),
    }
}
fn work() -> PathBuf {
    let d = manifest().join("../../out-crash1/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

fn run(name: &str, files: &[&str], src: &str, args: &[String]) -> (i32, Vec<String>, String) {
    run_with(name, files, src, args, &[], &[])
}

/// `env`: extra environment variables of the run; `cflags`: extra clang flags (for example -DFLE_SOFT_STOP)
fn run_with(name: &str, files: &[&str], src: &str, args: &[String], env: &[(&str, &str)], cflags: &[&str]) -> (i32, Vec<String>, String) {
    let dir = work();
    let u = etxt_common::uniq(name);
    let c = dir.join(format!("{u}.c"));
    let exe = dir.join(&u);
    std::fs::write(&c, src).unwrap();
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-Wall", "-Wextra", "-Wno-unused-function", "-Wno-unused-parameter", "-fPIE", "-pie"]).args(cflags).arg("-I").arg(rt()).arg(&c);
    for f in files {
        cmd.arg(rt().join(f));
    }
    let cc = cmd.arg("-o").arg(&exe).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    let mut r = Command::new(&exe);
    r.args(args).env_remove("FLE_SOFT_STOP");
    for (k, v) in env {
        r.env(k, v);
    }
    let r = r.output().expect("run failed");
    let _ = std::fs::remove_file(&exe);
    (r.status.code().unwrap_or(-1), String::from_utf8_lossy(&r.stdout).lines().map(|s| s.to_string()).collect(), String::from_utf8_lossy(&r.stderr).to_string())
}

#[test]
fn ucd_case_table_matches_every_code_point_of_the_generators_list() {
    let tsv = rt().join("test/case_ucd16_simple.tsv");
    let src = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include "fle_crash1_lcmap.h"
int main(int argc, char **argv)
{
    static uint32_t up[0x110000], lo[0x110000];
    static unsigned char has[0x110000];
    char line[1024];
    unsigned long cp, u, l, bad = 0, listed = 0, i;
    FILE *f = fopen(argv[1], "r");
    if (!f) { printf("no list\n"); return 2; }
    for (cp = 0; cp < 0x110000; cp++) { up[cp] = lo[cp] = (uint32_t)cp; }
    while (fgets(line, sizeof line, f)) {
        if (line[0] == '#') continue;
        if (sscanf(line, "%lx %lx %lx", &cp, &u, &l) != 3) { printf("bad line %s", line); return 2; }
        up[cp] = (uint32_t)u; lo[cp] = (uint32_t)l; has[cp] = 1; listed++;
    }
    fclose(f);
    for (i = 0; i < 0x110000; i++) {
        uint32_t gu = fle_crash1_ucd_case((uint32_t)i, 1), gl = fle_crash1_ucd_case((uint32_t)i, 0);
        if (gu != up[i] || gl != lo[i]) { if (bad < 10) printf("MISMATCH %05lx upper %05x want %05x lower %05x want %05x\n", i, gu, up[i], gl, lo[i]); bad++; }
    }
    /* beyond the code space and the surrogates are not code points: unchanged */
    if (fle_crash1_ucd_case(0x110000, 1) != 0x110000 || fle_crash1_ucd_case(0xFFFFFFFFu, 0) != 0xFFFFFFFFu) { printf("beyond\n"); bad++; }
    if (fle_crash1_ucd_case(0xD801, 1) != 0xD801 || fle_crash1_ucd_case(0xDC00, 0) != 0xDC00) { printf("surrogate\n"); bad++; }
    printf("listed %lu runs %u mismatches %lu\n", listed, fle_crash1_ucd_runs(), bad);
    return bad ? 1 : 0;
}
"#;
    let (rc, out, err) = run("crash1-ucd-table", &["fle_crash1_lcmap.c"], src, &[tsv.to_string_lossy().to_string()]);
    assert_eq!(rc, 0, "{out:?} {err}");
    let last = out.last().unwrap();
    assert!(last.starts_with("listed 2933 runs 392 mismatches 0"), "{last}");
    // a few rows of the Unicode data written by hand: ASCII, Latin Extended-A pairs, Greek, Cyrillic, Georgian (Mtavruli), Deseret (outside the BMP), Adlam
    for row in ["00041\t00041\t00061", "00130\t00130\t00069", "00131\t00049\t00131", "003A3\t003A3\t003C3", "00410\t00410\t00430", "010D0\t01C90\t010D0", "10400\t10400\t10428", "1E900\t1E900\t1E922"] {
        let tsv_text = std::fs::read_to_string(rt().join("test/case_ucd16_simple.tsv")).unwrap();
        assert!(tsv_text.lines().any(|l| l == row), "row {row} is not in the list");
    }
}

const PRE: &str = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_locale.h"
uint64_t R[16];
static uint64_t st[16];
void fle_stop(const char *api, const char *why) { printf("STOP %s: %s\n", api, why); fflush(stdout); exit(3); }
#define P(...) ((uint64_t)(uintptr_t)(__VA_ARGS__))
/* LCMapStringW(lcid, flags, src, cchSrc, dst, cchDest); prints "<ret> u" (the last error was left as the harness set it) or "<ret> e<last error>" and, when dst is not NULL and the call returned > 0, the first `show` units of dst in hex */
static void map(uint32_t lcid, uint32_t flags, const uint16_t *src, int cs, uint16_t *dst, int cd, int show)
{
    int i;
    R[1] = lcid; R[2] = flags; R[8] = P(src); R[9] = (uint64_t)(int64_t)cs; st[5] = P(dst); st[6] = (uint64_t)(int64_t)cd; R[4] = P(st);
    fle_set_last_error(0xAAAA); R[0] = 0xDEAD;
    fle_imp_LCMapStringW();
    { uint64_t r = R[0]; fle_imp_GetLastError(); if (R[0] == 0xAAAA) printf("%llu u", (unsigned long long)r); else printf("%llu e%llu", (unsigned long long)r, (unsigned long long)R[0]); }
    if (dst && show > 0) { printf(" |"); for (i = 0; i < show; i++) printf(" %x", dst[i]); }
    printf("\n");
}
static uint16_t out[256];
static void clr(void) { int i; for (i = 0; i < 256; i++) out[i] = 0x5555; }
"#;

fn prog(body: &str) -> String {
    format!("{PRE}\nint main(void)\n{{\n    setvbuf(stdout, NULL, _IONBF, 0);\n{body}\n    return 0;\n}}\n")
}

fn lcmap(name: &str, body: &str) -> (i32, Vec<String>, String) {
    run(name, &["fle_locale.c", "fle_imports.c", "fle_crash1_lcmap.c"], &prog(body), &[])
}

/// The call the engine made when the client clicked a folder of the Browser: LCMapStringW(0x409, LCMAP_LINGUISTIC_CASING | LCMAP_LOWERCASE, "/-106/32768/Sampler", 30, dst, 30): the string is 19 units, the call says 30, so the
/// NUL and the 10 units after it are part of the source whatever they hold. Before decision 0209 this stopped at the first unit of 0x80 or above.
#[test]
fn the_browser_folder_call_maps_all_cchsrc_units_and_does_not_stop() {
    let (rc, out, err) = lcmap(
        "crash1-browser",
        r#"
    static const uint16_t s[30] = { '/', '-', '1', '0', '6', '/', '3', '2', '7', '6', '8', '/', 'S', 'a', 'm', 'p', 'l', 'e', 'r', 0,
                                    0x00C4, 0x0100, 0x0391, 0x0416, 0x10A0, 0x20AC, 0xE000, 0xD801, 0xDC00, 0xFFFF };
    clr(); map(0x409, 0x01000100, s, 30, out, 30, 30);
    clr(); map(0x409, 0x01000200, s, 30, out, 30, 30);
    clr(); map(0x409, 0x00000100, s, 30, out, 30, 30);
    clr(); map(0x409, 0x01000100, s, 30, 0, 0, 0);
"#,
    );
    assert_eq!(rc, 0, "{out:?} {err}");
    // lower: ASCII lowered; the NUL maps to itself; U+00C4 -> U+00E4 (the Windows 8 table row, range D); U+0100 -> U+0101; U+0391 -> U+03B1; U+0416 -> U+0436; U+10A0 (Georgian capital An) -> U+2D00; U+20AC, U+E000 and U+FFFF
    // have no mapping; the pair D801 DC00 (Deseret capital long I, U+10400) -> U+10428 = D801 DC28
    let lower = "30 u | 2f 2d 31 30 36 2f 33 32 37 36 38 2f 73 61 6d 70 6c 65 72 0 e4 101 3b1 436 2d00 20ac e000 d801 dc28 ffff";
    let upper = "30 u | 2f 2d 31 30 36 2f 33 32 37 36 38 2f 53 41 4d 50 4c 45 52 0 c4 100 391 416 10a0 20ac e000 d801 dc00 ffff";
    assert_eq!(out, vec![lower, upper, lower, "30 u"], "flags 0x01000100, 0x01000200, 0x100 and the size query");
}

/// The sizes and the errors of the Learn pages: cchSrc negative = NUL-terminated and the NUL is mapped and counted; cchDest 0 returns the size; a buffer that is too small is ERROR_INSUFFICIENT_BUFFER (122); a NULL buffer with
/// a size is the same ("incorrectly set to NULL"); a source of NULL or of size 0, or a negative cchDest, is ERROR_INVALID_PARAMETER (87); flag bits that no page defines, and LINGUISTIC_CASING without a case, are
/// ERROR_INVALID_FLAGS (1004). The destination may be the source ("can use the same buffer").
#[test]
fn lcmapstringw_sizes_errors_and_in_place() {
    let (rc, out, err) = lcmap(
        "crash1-sizes",
        r#"
    static const uint16_t a[] = { 'A', 0x00C9, 0x0391, 0 };           /* Latin, range D, Greek, NUL */
    static uint16_t buf[8] = { 'A', 0x00C9, 0x0391, 0 };
    clr(); map(0x409, 0x100, a, -1, out, 4, 4);          /* NUL-terminated: 3 units and the NUL = 4 */
    clr(); map(0x409, 0x100, a, -7, out, 4, 4);          /* any negative value */
    clr(); map(0x409, 0x100, a, -1, 0, 0, 0);            /* the size */
    clr(); map(0x409, 0x100, a, 3, 0, 0, 0);
    clr(); map(0x409, 0x100, a, 3, out, 3, 3);           /* exactly the source without the NUL */
    clr(); map(0x409, 0x100, a, 4, out, 3, 0);           /* the NUL is in cchSrc, so cchDest must have room for it */
    clr(); map(0x409, 0x100, a, 3, out, 2, 0);
    clr(); map(0x409, 0x100, a, 3, 0, 3, 0);             /* NULL buffer with a size */
    map(0x409, 0x100, 0, -1, out, 4, 0);                 /* source NULL */
    map(0x409, 0x100, a, 0, out, 4, 0);                  /* cchSrc 0 */
    map(0x409, 0x100, a, -1, out, -1, 0);                /* cchDest negative */
    map(0x409, 0x40000000 | 0x100, a, -1, out, 4, 0);   /* bit 30: no page defines it */
    map(0x409, 0x01000000, a, -1, out, 4, 0);            /* LCMAP_LINGUISTIC_CASING alone */
    map(0x409, 0x100, buf, 3, buf, 3, 3);                /* in place */
    map(0x409, 0x200, buf, 3, buf, 3, 3);                /* and back */
"#,
    );
    assert_eq!(rc, 0, "{out:?} {err}");
    assert_eq!(
        out,
        vec![
            "4 u | 61 e9 3b1 0", "4 u | 61 e9 3b1 0", "4 u", "3 u", "3 u | 61 e9 3b1", "0 e122", "0 e122", "0 e122", "0 e87", "0 e87", "0 e87", "0 e1004", "0 e1004", "3 u | 61 e9 3b1",
            "3 u | 41 c9 391",
        ]
    );
}

/// "Use linguistic rules for casing, instead of file system rules (default)": without LCMAP_LINGUISTIC_CASING a letter is mapped only where the second letter maps back (U+0130, U+0131, U+017F, U+03C2, the Kelvin sign, U+1E9E,
/// the titlecase digraphs are left alone); with it they are all mapped; and U+00B5 (range D, a row of the Windows 8 table without a mapping) is mapped to U+039C only with it. Wine 10.0 gives the same answers for all
/// 0x110000 code points and the four flag sets (wine/diff-lines.txt of decision 0209 is empty).
#[test]
fn lcmapstringw_file_system_rules_versus_linguistic_casing() {
    let (rc, out, err) = lcmap(
        "crash1-linguistic",
        r#"
    static const uint16_t a[] = { 0x0130, 0x0131, 0x017F, 0x03C2, 0x212A, 0x1E9E, 0x01C5, 0x00B5, 'i', 'I' };
    clr(); map(0x409, 0x100, a, 10, out, 10, 10);
    clr(); map(0x409, 0x01000100, a, 10, out, 10, 10);
    clr(); map(0x409, 0x200, a, 10, out, 10, 10);
    clr(); map(0x409, 0x01000200, a, 10, out, 10, 10);
"#,
    );
    assert_eq!(rc, 0, "{out:?} {err}");
    assert_eq!(
        out,
        vec![
            // lower, file system rules: only 'I' -> 'i' (the others have no second letter that maps back)
            "10 u | 130 131 17f 3c2 212a 1e9e 1c5 b5 69 69",
            // lower, linguistic: U+0130 -> 'i', Kelvin -> 'k', U+1E9E -> sharp s, U+01C5 -> U+01C6; U+0131, U+017F, U+03C2, U+00B5 have no lower case
            "10 u | 69 131 17f 3c2 6b df 1c6 b5 69 69",
            // upper, file system rules: only 'i' -> 'I'
            "10 u | 130 131 17f 3c2 212a 1e9e 1c5 b5 49 49",
            // upper, linguistic: U+0131 -> 'I', U+017F -> 'S', U+03C2 -> U+03A3, U+01C5 -> U+01C4, U+00B5 -> U+039C
            "10 u | 130 49 53 3a3 212a 1e9e 1c4 39c 49 49",
        ]
    );
}

/// LCMAP_BYTEREV (the page: "if the application passes in 0x3450 0x4822, the result is 0x5034 0x2248"), alone and with a case flag (the case is mapped, then the bytes are reversed: a choice, S-crash1-3).
#[test]
fn lcmapstringw_byterev() {
    let (rc, out, err) = lcmap(
        "crash1-byterev",
        r#"
    static const uint16_t a[] = { 0x3450, 0x4822 };
    static const uint16_t b[] = { 'A', 0xD801, 0xDC00 };
    clr(); map(0x409, 0x800, a, 2, out, 2, 2);
    clr(); map(0x409, 0x800 | 0x100, b, 3, out, 3, 3);
    clr(); map(0x409, 0x800 | 0x200, b, 3, out, 3, 3);
    clr(); map(0x409, 0x800, a, 2, 0, 0, 0);
"#,
    );
    assert_eq!(rc, 0, "{out:?} {err}");
    assert_eq!(out, vec!["2 u | 5034 2248", "3 u | 6100 1d8 28dc", "3 u | 4100 1d8 dc", "2 u"]);
}

/// Surrogates: a pair is one code point (Deseret U+10400 <-> U+10428, Adlam U+1E900 <-> U+1E922), an unpaired high or low surrogate and a high surrogate at the end of the counted source are copied.
#[test]
fn lcmapstringw_supplementary_planes_and_lone_surrogates() {
    let (rc, out, err) = lcmap(
        "crash1-surrogates",
        r#"
    static const uint16_t d[] = { 0xD801, 0xDC00, 0xD801, 0xDC28, 0xD83A, 0xDD00, 0xD801 };   /* U+10400, U+10428, U+1E900 (Adlam capital Alif), a high surrogate alone */
    static const uint16_t e[] = { 0xDC00, 0xD801, 'x', 0xD801 };                               /* a low surrogate first; a high surrogate before a plain unit; a high surrogate last */
    clr(); map(0x409, 0x100, d, 7, out, 7, 7);
    clr(); map(0x409, 0x200, d, 7, out, 7, 7);
    clr(); map(0x409, 0x200, e, 4, out, 4, 4);
    clr(); map(0x409, 0x01000100, d, 6, out, 6, 6);
"#,
    );
    assert_eq!(rc, 0, "{out:?} {err}");
    assert_eq!(
        out,
        vec![
            "7 u | d801 dc28 d801 dc28 d83a dd22 d801",
            "7 u | d801 dc00 d801 dc00 d83a dd00 d801",
            "4 u | dc00 d801 58 d801",
            "6 u | d801 dc28 d801 dc28 d83a dd22",
        ]
    );
}

/// Flags that this runtime still cannot do stop by name, as before (the Learn page defines them; the only data of decision 0209 is the case table): title case, the sort key, kana, width, the Chinese forms, NORM_*, the hash.
/// A link without runtime/fle_crash1_lcmap.c behaves as before for a unit of 0x80 or above outside D.
#[test]
fn lcmapstringw_unimplemented_flags_stop_by_name_and_the_old_stop_remains_without_the_table() {
    for (label, flags) in [("title", 0x300u32), ("sortkey", 0x400), ("hiragana", 0x100000), ("katakana", 0x200000), ("halfwidth", 0x400000), ("fullwidth", 0x800000), ("simplified", 0x2000000), ("traditional", 0x4000000), ("ignorecase", 1), ("hash", 0x40000)] {
        let body = format!("    static const uint16_t a[] = {{ 'A', 0 }};\n    clr(); map(0x409, 0x{flags:x}, a, -1, out, 4, 2);\n");
        let (rc, out, err) = lcmap(&format!("crash1-unimpl-{label}"), &body);
        // fle_locale_stop prints "fle: <api>: <why> (...)" and aborts (SIGABRT: no exit code)
        assert_eq!(rc, -1, "{label}: {out:?} {err}");
        assert!(err.contains("fle: LCMapStringW: map flags other than a single LCMAP_LOWERCASE or LCMAP_UPPERCASE"), "{label}: {err}");
    }
    let src = prog("    static const uint16_t a[] = { 0x0391, 0 };\n    clr(); map(0x409, 0x01000100, a, -1, out, 4, 2);\n");
    // not the table of decision 0209 in the link: fle_locale.c's weak reference is unresolved, and the old stop is what it says
    let (rc, out, err) = run("crash1-no-table", &["fle_locale.c", "fle_imports.c"], &src, &[]);
    assert_eq!(rc, -1, "{out:?} {err}");
    assert!(err.contains("fle: LCMapStringW: character of 0x80 or above: Unicode case table not on the MSDN pages"), "{err}");
}

// ---- 2. shell32's file drag-and-drop family (runtime/fle_crash1_drag.c) ----

const DRAG_PRE: &str = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_win.h"
#include "fle_crash1_drag.h"
uint64_t R[16];
void fle_stop(const char *api, const char *why) { printf("STOP %s: %s\n", api, why); fflush(stdout); exit(3); }
#define P(...) ((uint64_t)(uintptr_t)(__VA_ARGS__))
/* fle_win.c is not linked: the window table is a stub that records what DragAcceptFiles asked of it */
static uint64_t live_hwnd = 0x7f990004ull; static uint32_t ex = 0x00010040u; static int calls;
int fle_win_set_exstyle_bits(uint64_t hwnd, uint32_t mask, uint32_t value) { calls++; if (hwnd != live_hwnd) return 0; ex = (ex & ~mask) | (value & mask); return 1; }
static void rel(uint64_t h) { printf("released\n"); }
/* a DROPFILES block: pFiles 20, pt (x, y), fNC, fWide, then the list */
static uint8_t blk[512] __attribute__((aligned(8)));
static void mk(int32_t x, int32_t y, uint32_t fnc, uint32_t wide, const char *const *names)
{
    uint32_t h[5]; size_t at = 20; unsigned i;
    memset(blk, 0xEE, sizeof blk);
    h[0] = 20; h[1] = (uint32_t)x; h[2] = (uint32_t)y; h[3] = fnc; h[4] = wide; memcpy(blk, h, 20);
    for (i = 0; names[i]; i++) { const char *s = names[i]; size_t k; for (k = 0; ; k++) { if (wide) { uint16_t u = (uint16_t)(unsigned char)s[k]; memcpy(blk + at, &u, 2); at += 2; } else blk[at++] = (uint8_t)s[k]; if (!s[k]) break; } }
    if (wide) { uint16_t z = 0; memcpy(blk + at, &z, 2); } else blk[at] = 0;
}
static uint16_t out[64];
static void q(uint64_t h, uint32_t idx, uint16_t *buf, uint32_t cch)
{
    int i;
    R[1] = h; R[2] = idx; R[8] = P(buf); R[9] = cch; R[0] = 0xDEAD;
    fle_imp_DragQueryFileW();
    printf("%llu", (unsigned long long)R[0]);
    if (buf && cch > 0 && idx != 0xFFFFFFFFu) { printf(" '"); for (i = 0; buf[i] && i < 63; i++) putchar(buf[i] < 0x80 ? buf[i] : '?'); printf("'"); }
    printf("\n");
}
"#;

fn drag(name: &str, body: &str) -> (i32, Vec<String>, String) {
    let src = format!("{DRAG_PRE}\nint main(void)\n{{\n    setvbuf(stdout, NULL, _IONBF, 0);\n{body}\n    return 0;\n}}\n");
    run(name, &["fle_crash1_drag.c"], &src, &[])
}

/// DragQueryFileW over a DROPFILES block of three names (Learn): 0xFFFFFFFF = the count; an index = the name copied with its NUL, the return value the characters copied without the NUL; lpszFile NULL = the size without the NUL;
/// a buffer that is too small copies what fits and the NUL; an index past the end, a buffer of 0 characters and a handle that was not registered return 0 (the page does not say; a reading of Wine and ReactOS).
#[test]
fn drag_query_file_follows_the_page_over_a_registered_drop() {
    let (rc, out, err) = drag(
        "crash1-dragqueryfile",
        r#"
    static const char *const names[] = { "C:\\a.txt", "D:\\music\\b b.mid", "E:\\c", 0 };
    mk(7, 9, 0, 1, names);
    printf("registered %d\n", fle_drop_register(P(blk), rel));
    q(P(blk), 0xFFFFFFFFu, 0, 0);                 /* the count */
    q(P(blk), 0xFFFFFFFFu, out, 64);              /* the count, and the buffer is not touched */
    q(P(blk), 0, 0, 0);                           /* size of name 0 */
    q(P(blk), 1, 0, 0);
    q(P(blk), 2, 0, 0);
    q(P(blk), 0, out, 64);
    q(P(blk), 1, out, 64);
    q(P(blk), 2, out, 64);
    q(P(blk), 1, out, 6);                         /* room for 5 characters and the NUL */
    q(P(blk), 1, out, 1);                         /* room for the NUL only */
    q(P(blk), 1, out, 0);                         /* no room at all */
    q(P(blk), 3, out, 64);                        /* past the end */
    q(P(blk), 3, 0, 0);
    q(P(blk) + 8, 0, out, 64);                    /* not a registered handle */
    q(0, 0xFFFFFFFFu, 0, 0);
"#,
    );
    assert_eq!(rc, 0, "{out:?} {err}");
    assert_eq!(
        out,
        vec![
            "registered 1",
            "3",
            "3",
            "8",
            "16",
            "4",
            "8 'C:\\a.txt'",
            "16 'D:\\music\\b b.mid'",
            "4 'E:\\c'",
            "5 'D:\\mu'",
            "0 ''",   // cch 1: only the NUL
            "0",      // cch 0: nothing
            "0 ''",   // past the end
            "0",      // past the end, size query
            "0 ''",   // not a registered handle
            "0",      // handle 0
        ]
    );
}

/// An ANSI list (fWide 0) is read byte by byte for ASCII names; a byte of 0x80 or above stops by name (the code page conversion of the list is not done). DragQueryPoint and DragFinish: the point is returned in all cases, TRUE only
/// when the drop was in the client area (fNC FALSE); an unregistered handle is FALSE and leaves the POINT alone; DragFinish releases the handle through its owner and the handle is gone.
#[test]
fn drag_query_point_and_finish_and_the_ansi_list() {
    let (rc, out, err) = drag(
        "crash1-dragpoint",
        r#"
    static const char *const names[] = { "C:\\x", "y", 0 };
    int32_t pt[2];
    mk(-12, 345, 0, 0, names);
    fle_drop_register(P(blk), rel);
    q(P(blk), 0xFFFFFFFFu, 0, 0);
    q(P(blk), 1, out, 64);
    pt[0] = pt[1] = 0x7777; R[1] = P(blk); R[2] = P(pt); R[0] = 0xDEAD; fle_imp_DragQueryPoint(); printf("client: %llu (%d, %d)\n", (unsigned long long)R[0], pt[0], pt[1]);
    mk(1000, 2000, 1, 1, names);
    pt[0] = pt[1] = 0x7777; R[1] = P(blk); R[2] = P(pt); R[0] = 0xDEAD; fle_imp_DragQueryPoint(); printf("nonclient: %llu (%d, %d)\n", (unsigned long long)R[0], pt[0], pt[1]);
    pt[0] = pt[1] = 0x7777; R[1] = P(blk) + 8; R[2] = P(pt); R[0] = 0xDEAD; fle_imp_DragQueryPoint(); printf("unregistered: %llu (%d, %d)\n", (unsigned long long)R[0], pt[0], pt[1]);
    R[1] = P(blk); fle_imp_DragFinish();
    printf("live after finish %d\n", fle_drop_is_live(P(blk)));
    R[1] = P(blk); fle_imp_DragFinish();           /* a second finish and a handle never registered: nothing */
    R[1] = 0; fle_imp_DragFinish();
    q(P(blk), 0xFFFFFFFFu, 0, 0);
    { static const char *const hi[] = { "caf\xe9", 0 }; mk(0, 0, 0, 0, hi); fle_drop_register(P(blk), 0); q(P(blk), 0, out, 64); }
"#,
    );
    assert_eq!(rc, 3, "the last call stops by name: {out:?} {err}");
    assert_eq!(out[..7], ["2", "1 'y'", "client: 1 (-12, 345)", "nonclient: 0 (1000, 2000)", "unregistered: 0 (30583, 30583)", "released", "live after finish 0"]);
    assert_eq!(out[7], "0");
    assert!(out[8].starts_with("STOP DragQueryFileW: an ANSI file list with a byte of 0x80 or above"), "{out:?}");
}

/// DragAcceptFiles (Learn: void; "Registers whether a window accepts dropped files"; TRUE accepts, FALSE discontinues): it sets and clears WS_EX_ACCEPTFILES (0x10) of the window; a handle that is not a live window changes
/// nothing and is not an error; the other extended style bits are untouched; BOOL is the low 32 bits of the argument.
#[test]
fn drag_accept_files_sets_and_clears_ws_ex_acceptfiles_and_ignores_a_handle_that_is_not_a_window() {
    let (rc, out, err) = drag(
        "crash1-dragaccept",
        r#"
    R[1] = live_hwnd; R[2] = 1; fle_imp_DragAcceptFiles(); printf("TRUE: ex %x\n", ex);
    R[1] = live_hwnd; R[2] = 7; fle_imp_DragAcceptFiles(); printf("any nonzero: ex %x\n", ex);
    R[1] = live_hwnd; R[2] = 0; fle_imp_DragAcceptFiles(); printf("FALSE: ex %x\n", ex);
    R[1] = live_hwnd; R[2] = 0; fle_imp_DragAcceptFiles(); printf("FALSE again: ex %x\n", ex);
    R[1] = 0x1234; R[2] = 1; fle_imp_DragAcceptFiles(); printf("not a window: ex %x calls %d\n", ex, calls);
    R[1] = live_hwnd; R[2] = 0x100000000ull; fle_imp_DragAcceptFiles(); printf("only the high half set: ex %x\n", ex);
"#,
    );
    assert_eq!(rc, 0, "{out:?} {err}");
    assert_eq!(out, vec!["TRUE: ex 10050", "any nonzero: ex 10050", "FALSE: ex 10040", "FALSE again: ex 10040", "not a window: ex 10040 calls 5", "only the high half set: ex 10040"]);
}

// ---- 3. the same table for user32's CharUpper / CharLower family, the range D agrees with the helper, the rows of the seeds file ----

/// CharUpperW / CharLowerW (a single character when the high word of the argument is zero, else a NUL-terminated string converted in place) and CharUpperBuffW / CharLowerBuffW answer a unit of 0x80 or above from the same table,
/// by file system rules, per UTF-16 unit (a surrogate unit is not changed: these functions know no pairs). Before decision 0209 they stopped by name (8 of the stops that the ia1/ia2 operations recorded). Wine 10.0 gives the same
/// answers for all 65,536 units and the four functions (wine/charcase_wine.txt, 4,672 changed units, and wine/charcase_rt.txt are equal).
#[test]
fn char_upper_lower_family_uses_the_same_table_per_utf16_unit() {
    let src = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_user_nowin.h"
uint64_t R[16];
#define P(...) ((uint64_t)(uintptr_t)(__VA_ARGS__))
static void one(void (*f)(void), uint32_t u) { R[1] = u; f(); printf("%04x -> %04x\n", u, (unsigned)R[0]); }
int main(void)
{
    static uint16_t s[] = { 'a', 0x00E9, 0x03B1, 0x0131, 0xD801, 0xDC28, 0x0416, 0 };
    static uint16_t t[] = { 'A', 0x00C9, 0x0391, 0x0130, 0x03A3, 0x03C2, 0xD801, 0xDC00 };
    int i;
    setvbuf(stdout, NULL, _IONBF, 0);
    one(fle_imp_CharUpperW, 0xE9); one(fle_imp_CharUpperW, 0x3B1); one(fle_imp_CharUpperW, 0x131); one(fle_imp_CharUpperW, 0x20AC);
    one(fle_imp_CharLowerW, 0xC9); one(fle_imp_CharLowerW, 0x3A3); one(fle_imp_CharLowerW, 0x130); one(fle_imp_CharLowerW, 0x1C5);
    R[1] = P(s); fle_imp_CharUpperW(); printf("string returns the pointer %d:", R[0] == P(s)); for (i = 0; s[i]; i++) printf(" %x", s[i]); printf("\n");
    R[1] = P(t); R[2] = 8; fle_imp_CharLowerBuffW(); printf("lower buf %llu:", (unsigned long long)R[0]); for (i = 0; i < 8; i++) printf(" %x", t[i]); printf("\n");
    R[1] = P(t); R[2] = 8; fle_imp_CharUpperBuffW(); printf("upper buf %llu:", (unsigned long long)R[0]); for (i = 0; i < 8; i++) printf(" %x", t[i]); printf("\n");
    return 0;
}
"#;
    let (rc, out, err) = run("crash1-charcase", &["fle_user_nowin.c", "fle_crash1_lcmap.c", "fle_imports.c", "fle_env.c"], src, &[]);
    assert_eq!(rc, 0, "{out:?} {err}");
    assert_eq!(
        out,
        vec![
            // upper: e-acute, alpha; dotless i has no pair under file system rules; the euro sign has no case
            "00e9 -> 00c9", "03b1 -> 0391", "0131 -> 0131", "20ac -> 20ac",
            // lower: E-acute, capital sigma, dotted capital I (no pair), the titlecase digraph (no pair)
            "00c9 -> 00e9", "03a3 -> 03c3", "0130 -> 0130", "01c5 -> 01c5",
            "string returns the pointer 1: 41 c9 391 131 d801 dc28 416",
            "lower buf 8: 61 e9 3b1 130 3c3 3c2 d801 dc00",
            "upper buf 8: 41 c9 391 130 3a3 3c2 d801 dc00",
        ]
    );
}

/// The range D of decision 0087 keeps its own rows (the Windows 8 table: fle_locale.c) for file system rules; the helper of this decision (round trip of the UCD) gives the same answer for all 284 units, so the two tables cannot
/// disagree about a unit that either could answer. The mapping of LCMapStringW is compared with fle_crash1_case() for flags 0x100 and 0x200.
#[test]
fn the_windows_8_rows_of_the_range_d_and_the_ucd_round_trip_rule_agree_on_every_unit_of_d() {
    let src = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_locale.h"
#include "fle_crash1_lcmap.h"
uint64_t R[16];
static uint64_t st[16];
void fle_stop(const char *api, const char *why) { printf("STOP %s: %s\n", api, why); exit(3); }
int main(void)
{
    static const uint16_t high[28] = { 0x0152, 0x0153, 0x0160, 0x0161, 0x0178, 0x017D, 0x017E, 0x0191, 0x0192, 0x02C6, 0x02DC, 0x2013, 0x2014, 0x2018, 0x2019, 0x201A, 0x201C, 0x201D, 0x201E, 0x2020, 0x2021, 0x2022, 0x2026, 0x2030, 0x2039, 0x203A, 0x20AC, 0x2122 };
    unsigned i, n = 0, bad = 0, f;
    for (i = 0; i < 256 + 28; i++) {
        uint16_t u = i < 256 ? (uint16_t)i : high[i - 256];
        for (f = 0; f < 2; f++) {
            uint16_t s[1], d[1] = { 0x5555 };
            uint32_t want = fle_crash1_case(u, f, 0);
            s[0] = u;
            R[1] = 0x409; R[2] = f ? 0x200 : 0x100; R[8] = (uint64_t)(uintptr_t)s; R[9] = 1; st[5] = (uint64_t)(uintptr_t)d; st[6] = 1; R[4] = (uint64_t)(uintptr_t)st;
            fle_imp_LCMapStringW();
            n++;
            if (d[0] != want) { if (bad < 5) printf("DIFF %04x %s: rows %04x helper %04x\n", u, f ? "upper" : "lower", d[0], want); bad++; }
        }
    }
    printf("units %u differences %u\n", n, bad);
    return bad != 0;
}
"#;
    let (rc, out, err) = run("crash1-d-agree", &["fle_locale.c", "fle_imports.c", "fle_crash1_lcmap.c"], src, &[]);
    assert_eq!(rc, 0, "{out:?} {err}");
    assert_eq!(out.last().unwrap(), "units 568 differences 0");
}

/// The rows of seeds/crash1-imports.txt name runtime functions that exist, once each, in the format of decision 0156; and the engine's own import table has no row for these four yet (so the rows are what makes them reachable).
#[test]
fn the_crash1_import_rows_name_existing_runtime_functions() {
    let rows = std::fs::read_to_string(rt().join("../harness/seeds/crash1-imports.txt")).unwrap();
    let mut n = 0;
    let mut all_c = String::new();
    for e in std::fs::read_dir(rt()).unwrap() {
        let p = e.unwrap().path();
        if p.extension().map(|x| x == "c").unwrap_or(false) {
            all_c.push_str(&std::fs::read_to_string(&p).unwrap());
        }
    }
    for l in rows.lines().filter(|l| !l.trim().is_empty() && !l.starts_with('#')) {
        let f: Vec<&str> = l.split_whitespace().collect();
        assert_eq!(f.len(), 3, "{l}");
        assert!(f[0] == "shell32.dll" || f[0] == "user32.dll", "{l}");
        assert_eq!(f[2], format!("fle_imp_{}", f[1]));
        assert!(all_c.contains(&format!("void {}(void)\n{{", f[2])) || all_c.contains(&format!("void {}(void) {{", f[2])), "{} is not defined in runtime/*.c", f[2]);
        n += 1;
    }
    assert_eq!(n, 18, "4 shell32 rows (the file drag-and-drop family) and 14 user32 rows (the menu family)");
}


// ---- 4. soft stops (runtime/fle_crash1_soft.c, opt-in) ----

const SOFT_BODY: &str = r#"
    static const uint16_t a[] = { 'A', 0 };
    int i;
    /* an unimplemented flag (title case) four times, then other APIs of the family */
    for (i = 0; i < 5; i++) { clr(); map(0x409, 0x300, a, -1, out, 4, 0); }
    R[1] = 932; R[0] = 0xDEAD; fle_set_last_error(0xAAAA); fle_imp_IsValidCodePage(); { uint64_t r = R[0]; fle_imp_GetLastError(); printf("IsValidCodePage(932): %llu last error %llx\n", (unsigned long long)r, (unsigned long long)R[0]); }
    { static uint8_t info[20]; R[1] = 932; R[2] = 0; R[8] = P(info); R[0] = 0xDEAD; fle_set_last_error(0xAAAA); fle_imp_GetCPInfo(); { uint64_t r = R[0]; fle_imp_GetLastError(); printf("GetCPInfo(932): %llu last error %llu\n", (unsigned long long)r, (unsigned long long)R[0]); } }
    printf("counts: LCMapStringW %llu IsValidCodePage %llu GetCPInfo %llu CompareStringW %llu; apis %u\n", (unsigned long long)fle_soft_count("LCMapStringW"), (unsigned long long)fle_soft_count("IsValidCodePage"), (unsigned long long)fle_soft_count("GetCPInfo"), (unsigned long long)fle_soft_count("CompareStringW"), fle_soft_apis());
"#;

fn soft_prog(body: &str) -> String {
    prog(body).replace("#include \"fle_locale.h\"", "#include \"fle_locale.h\"\n#include \"fle_crash1_soft.h\"")
}

// ---- 5. user32's menu family against Wine 10.0 ----

/// The same 156 calls on Wine 10.0 (x86_64-w64-mingw32-gcc + wine; the file is runtime/test/menu_battery_wine10.txt, made by wine/menu_battery.exe) and on this runtime (runtime/test/menu_battery.inc built with -x c -DFLE_RUNTIME) print the same lines,
/// the last errors included: creation, insertion by position and by command (also into the submenus: Wine finds a command id in a submenu), the half-made item that a failed InsertMenuItemW leaves, GetMenuItemInfoW for every mask,
/// the strings (size queries and truncation), GetMenuState, GetMenuItemID, GetSubMenu, SetMenuItemInfoW (the depth loop of the submenu hierarchy, a bad handle), CheckMenuItem / EnableMenuItem / DeleteMenu on these menus, RemoveMenu, DestroyMenu.
/// Wine is not Windows (a reading): where the Learn pages are silent (the last errors, the position rules) this is Wine's behaviour, UNOFFICIAL.
#[test]
fn the_menu_family_prints_exactly_what_wine_10_prints() {
    let battery = rt().join("test/menu_battery.inc");
    let dir = work();
    let u = etxt_common::uniq("crash1-menu-battery");
    let exe = dir.join(&u);
    let mut cmd = Command::new("clang");
    cmd.args(["-O1", "-std=gnu11", "-w", "-fPIE", "-pie", "-DFLE_RUNTIME", "-x", "c"]).arg("-I").arg(rt()).arg(&battery);
    for f in ["fle_crash1_menu.c", "fle_imports.c", "fle_env.c"] {
        cmd.arg(rt().join(f));
    }
    let cc = cmd.arg("-o").arg(&exe).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new(&exe).output().expect("run failed");
    let _ = std::fs::remove_file(&exe);
    let got = String::from_utf8_lossy(&r.stdout).to_string();
    let want = std::fs::read_to_string(rt().join("test/menu_battery_wine10.txt")).unwrap();
    assert_eq!(got.lines().count(), 156, "{}", String::from_utf8_lossy(&r.stderr));
    for (i, (g, w)) in got.lines().zip(want.lines()).enumerate() {
        assert_eq!(g, w, "line {}", i + 1);
    }
    assert_eq!(got.lines().count(), want.lines().count());
}
