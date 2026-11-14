//! Tests of decision 0191 (docs/0191-user-operation-walls-2.md, PROVISIONAL): the runtime pieces of the second wave of user-operation walls (runtime/fle_wall_ia2.c, the hook in runtime/fle_locale.c, runtime/fle_load.c's diagnostics).
//!
//!  1. user32!GetKeyNameTextW (wall ia2-W18). Expected values are the strings of aKeyNames[] / aKeyNamesExt[] of Microsoft's sample layout kbdus.c (microsoft/Windows-driver-samples, input/layout/kbdus/kbdus.c, MS-PL;
//!     SHA-256 5a271ed2...f3fc3f96 of the file read), the Learn page's rules for lParam bits 16-23 (scan code), 24 (extended), 25 (do not care) and the return value (the length without the NUL), written here by hand.
//!  2. CompareStringW with the primary weights of Microsoft's "Windows 10 Sorting Weight Table.txt" (Download Center id=10921, SHA-256 81fcfa1e...df663f). The expected (SM, AW) values are lines of that file, quoted in the
//!     comments of each case; the hook was on only when the response table held the marker row "CompareStringW::ia2-sort-weights"; decision 0284 removed the row and the switch (docs/policies/no-options.md: every shipped configuration had the row), so it is always on.
//!
//! The C programs call the `fle_imp_*` functions the way the lifted code reaches them: arguments in R[1], R[2], R[8], R[9], the result in R[0]. EVERY ANSWER IS PROVISIONAL (nothing is compared with a real Windows).
//! FLE_IA2_RT_DIR (a full copy of runtime/) replaces runtime/ for the mutation runs.

mod etxt_common;

use std::path::PathBuf;
use std::process::Command;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn rt() -> PathBuf {
    match std::env::var_os("FLE_IA2_RT_DIR") {
        Some(d) => PathBuf::from(d),
        None => manifest().join("../../runtime"),
    }
}
fn work() -> PathBuf {
    let d = manifest().join("../../out-ia2/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

fn run(name: &str, files: &[&str], src: &str) -> (i32, Vec<String>, String) {
    let dir = work();
    let u = etxt_common::uniq(name);
    let c = dir.join(format!("{u}.c"));
    let exe = dir.join(&u);
    std::fs::write(&c, src).unwrap();
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-Wall", "-Wextra", "-Wno-unused-function", "-Wno-unused-parameter", "-fPIE", "-pie"]).arg("-I").arg(rt()).arg(&c);
    for f in files {
        cmd.arg(rt().join(f));
    }
    let cc = cmd.arg("-o").arg(&exe).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new(&exe).output().expect("run failed");
    let _ = std::fs::remove_file(&exe);
    (r.status.code().unwrap_or(-1), String::from_utf8_lossy(&r.stdout).lines().map(|s| s.to_string()).collect(), String::from_utf8_lossy(&r.stderr).to_string())
}

const KEY_PRE: &str = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_wall_ia2.h"
uint64_t R[16];
void fle_stop(const char *api, const char *why) { printf("STOP %s: %s\n", api, why); fflush(stdout); exit(3); }
static uint16_t buf[64];
/* lParam = scan << 16 | ext << 24 | dontcare << 25; the program prints the name and the return value */
static void key(uint32_t scan, int ext, int dontcare, int cch)
{
    int i;
    memset(buf, 0xAA, sizeof buf);
    R[1] = (uint64_t)(((uint32_t)scan << 16) | ((uint32_t)ext << 24) | ((uint32_t)dontcare << 25)); R[2] = (uint64_t)(uintptr_t)buf; R[8] = (uint64_t)cch; R[0] = 0xDEAD;
    fle_imp_GetKeyNameTextW();
    printf("%02x%s%s ret %llu '", scan, ext ? "e" : "", dontcare ? "d" : "", (unsigned long long)R[0]);
    for (i = 0; buf[i]; i++) putchar(buf[i] < 0x80 ? buf[i] : '?');
    printf("' nul-after %d\n", buf[i] == 0);
}
"#;

#[test]
fn get_key_name_text_answers_the_names_of_the_us_layout() {
    let src = format!(
        "{KEY_PRE}{}",
        r#"
int main(void)
{
    setvbuf(stdout, NULL, _IONBF, 0);
    key(0x1d, 0, 0, 256);   /* Ctrl */
    key(0x1f, 0, 0, 256);   /* S: the key's own character */
    key(0x02, 0, 0, 256);   /* 1 */
    key(0x0c, 0, 0, 256);   /* - */
    key(0x29, 0, 0, 256);   /* ` */
    key(0x3b, 0, 0, 256);   /* F1 */
    key(0x57, 0, 0, 256);   /* F11 */
    key(0x39, 0, 0, 256);   /* Space */
    key(0x52, 0, 0, 256);   /* the engine asks scan<<16 for VK_INSERT: not extended -> "Num 0" in the table */
    key(0x52, 1, 0, 256);   /* Insert */
    key(0x1d, 1, 0, 256);   /* Right Ctrl */
    key(0x1d, 1, 1, 256);   /* do not care: Ctrl */
    key(0x36, 0, 1, 256);   /* do not care: Shift */
    key(0x38, 1, 1, 256);   /* do not care: Alt */
    key(0x4b, 1, 0, 256);   /* Left */
    key(0x51, 1, 0, 256);   /* Page Down */
    key(0x5b, 1, 0, 256);   /* Left Windows */
    key(0x1c, 1, 0, 256);   /* Num Enter */
    key(0x1f, 0, 0, 2);     /* exactly one character and the NUL fit */
    return 0;
}
"#
    );
    let (rc, out, err) = run("ia2-key-names", &["fle_wall_ia2.c"], &src);
    assert_eq!(rc, 0, "{out:?} {err}");
    assert_eq!(
        out,
        [
            "1d ret 4 'Ctrl' nul-after 1",
            "1f ret 1 'S' nul-after 1",
            "02 ret 1 '1' nul-after 1",
            "0c ret 1 '-' nul-after 1",
            "29 ret 1 '`' nul-after 1",
            "3b ret 2 'F1' nul-after 1",
            "57 ret 3 'F11' nul-after 1",
            "39 ret 5 'Space' nul-after 1",
            "52 ret 5 'Num 0' nul-after 1",
            "52e ret 6 'Insert' nul-after 1",
            "1de ret 10 'Right Ctrl' nul-after 1",
            "1ded ret 4 'Ctrl' nul-after 1",
            "36d ret 5 'Shift' nul-after 1",
            "38ed ret 3 'Alt' nul-after 1",
            "4be ret 4 'Left' nul-after 1",
            "51e ret 9 'Page Down' nul-after 1",
            "5be ret 12 'Left Windows' nul-after 1",
            "1ce ret 9 'Num Enter' nul-after 1",
            "1f ret 1 'S' nul-after 1",
        ]
    );
}

#[test]
fn get_key_name_text_stops_by_name_where_the_tables_and_the_page_give_nothing() {
    for (label, body, expect) in [
        ("unknown", "key(0x7a, 0, 0, 256);", "STOP GetKeyNameTextW: the scan code / extended flag has no name"),
        ("small", "key(0x1d, 0, 0, 4);", "STOP GetKeyNameTextW: cchSize cannot hold the key name and its NUL"),
        ("null", "R[1] = 0x1f0000; R[2] = 0; R[8] = 256; fle_imp_GetKeyNameTextW();", "STOP GetKeyNameTextW: lpString is NULL"),
    ] {
        let src = format!("{KEY_PRE}int main(void) {{ setvbuf(stdout, NULL, _IONBF, 0); {body} printf(\"not reached\\n\"); return 0; }}\n");
        let (rc, out, err) = run(&format!("ia2-key-stop-{label}"), &["fle_wall_ia2.c"], &src);
        assert_eq!(rc, 3, "{label}: {out:?} {err}");
        assert!(out.last().unwrap().starts_with(expect), "{label}: {out:?}");
    }
}

/// CompareStringW through fle_locale.c and fle_wall_ia2.c (decision 0284: no marker row, no stub response table).
fn cmp_run(name: &str, cases: &str) -> (i32, Vec<String>, String) {
    let src = format!(
        r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_locale.h"
#include "fle_modules.h"
#include "fle_wall_ia2.h"
uint64_t R[16];
static uint64_t st[16];
void fle_stop(const char *api, const char *why) {{ printf("STOP %s: %s\n", api, why); fflush(stdout); exit(3); }}
#define P(...) ((uint64_t)(uintptr_t)(__VA_ARGS__))
static uint16_t *W(const char *s)   /* UTF-8 -> UTF-16 (BMP only) */
{{
    uint16_t *d = calloc(strlen(s) + 1, 2); size_t n = 0; const unsigned char *u = (const unsigned char *)s;
    while (*u) {{ uint32_t c = *u++; if (c >= 0xE0) {{ c = ((c & 0xF) << 12) | ((u[0] & 0x3F) << 6) | (u[1] & 0x3F); u += 2; }} else if (c >= 0xC0) {{ c = ((c & 0x1F) << 6) | (u[0] & 0x3F); u++; }} d[n++] = (uint16_t)c; }}
    return d;
}}
static void cmp(const char *label, const char *a, const char *b, uint32_t flags)
{{
    uint16_t *x = W(a), *y = W(b); int nx = 0, ny = 0;
    while (x[nx]) nx++;
    while (y[ny]) ny++;
    R[1] = 0x409; R[2] = flags; R[8] = P(x); R[9] = (uint64_t)(int64_t)nx; st[5] = P(y); st[6] = (uint64_t)(int64_t)ny; R[4] = P(st); R[0] = 0xDEAD;
    fle_imp_CompareStringW();
    printf("%s %llu\n", label, (unsigned long long)R[0]);
}}
int main(void) {{ setvbuf(stdout, NULL, _IONBF, 0); {cases} return 0; }}
"#
    );
    run(name, &["fle_locale.c", "fle_imports.c", "fle_wall_ia2.c"], &src)
}

#[test]
fn compare_string_orders_units_by_the_primary_weights_of_microsofts_table() {
    // Lines of Windows 10 Sorting Weight Table.txt (default table; columns CP, SM, AW, DW, CW):
    //   0x0020 SM 7 AW 2 | 0x0041 'A' SM 14 AW 2 CW 18 | 0x0061 'a' SM 14 AW 2 CW 2 | 0x0030 '0' SM 13 AW 3 | 0x007a 'z' SM 14 AW 169
    //   0x3042 HIRAGANA A SM 3 AW 2 CW 39 | 0x30a2 KATAKANA A SM 3 AW 2 CW 7 | 0x30ab KATAKANA KA SM 3 AW 10 | 0x4e00 SM 5 AW 60 | 0x65e5 SM 5 AW 84 | 0x0410 CYRILLIC A SM 16 AW 2 | 0x03b1 GREEK ALPHA SM 15 AW 2
    // The primary weight is (SM, AW) compared as numbers; kana (SM 3) < ideographs (5) < symbols (6-12) < digits (13) < Latin (14) < Greek (15) < Cyrillic (16).
    let cases = r#"
    cmp("kana-vs-ascii", "\u3042", "UNSORTED", 0);
    cmp("ascii-vs-kana", "UNSORTED", "\u3042", 0);
    cmp("kana-vs-space", "\u3042", " x", 0);
    cmp("kana-vs-digit", "\u3042", "0", 0);
    cmp("han-vs-kana", "\u4e00", "\u3042", 0);
    cmp("kana-a-vs-ka", "\u3042", "\u30ab", 0);
    cmp("ka-vs-a", "\u30ab", "\u30a2", 0);
    cmp("greek-vs-latin", "\u03b1", "z", 0);
    cmp("cyrillic-vs-greek", "\u0410", "\u03b1", 0);
    cmp("prefix-kana", "\u3042\u3042", "\u3042\u30ab", 0);
    cmp("han-vs-latin", "\u65e5", "a", 0);
"#
    .replace("\\u3042", "\u{3042}")
    .replace("\\u30ab", "\u{30ab}")
    .replace("\\u30a2", "\u{30a2}")
    .replace("\\u4e00", "\u{4e00}")
    .replace("\\u03b1", "\u{03b1}")
    .replace("\\u0410", "\u{0410}")
    .replace("\\u65e5", "\u{65e5}");
    let (rc, out, err) = cmp_run("ia2-cmp-weights", &cases);
    assert_eq!(rc, 0, "{out:?} {err}");
    assert_eq!(
        out,
        [
            "kana-vs-ascii 1", "ascii-vs-kana 3", "kana-vs-space 1", "kana-vs-digit 1", "han-vs-kana 3", "kana-a-vs-ka 1", "ka-vs-a 3", "greek-vs-latin 3", "cyrillic-vs-greek 3", "prefix-kana 1", "han-vs-latin 1",
        ]
    );
}

/// Decision 0284 (A), on top of 0284c (the table is always on): two units of the ASCII symbol classes (and the white-space controls against the space) are ordered by the primary weights (SM, AW) of Microsoft's table, not by their code points. The expected values are
/// read from the rows of Windows10-SortingWeightTable.txt (SHA-256 81fcfa1e...; default table), not from the runtime: for example `+` is SM 8 AW 3 and `,` is SM 7 AW 47, so `+` is the greater although 0x2b < 0x2c; TAB is SM 7 AW 5 and the space
/// SM 7 AW 2, so TAB is the greater. 53 printable pairs and 5 TAB..CR-against-space pairs whose order differs from the code-point order, then 12 pairs where the two orders agree. 1 = CSTR_LESS_THAN, 3 = CSTR_GREATER_THAN.
#[test]
fn compare_string_orders_the_ascii_symbols_by_the_table_and_not_by_the_code_point() {
    let cases = r#"
    cmp("u2b-u2c", "\x2b", "\x2c", 0);
    cmp("u2b-u2e", "\x2b", "\x2e", 0);
    cmp("u2b-u2f", "\x2b", "\x2f", 0);
    cmp("u2b-u3a", "\x2b", "\x3a", 0);
    cmp("u2b-u3b", "\x2b", "\x3b", 0);
    cmp("u2b-u3f", "\x2b", "\x3f", 0);
    cmp("u2b-u40", "\x2b", "\x40", 0);
    cmp("u2b-u5b", "\x2b", "\x5b", 0);
    cmp("u2b-u5c", "\x2b", "\x5c", 0);
    cmp("u2b-u5d", "\x2b", "\x5d", 0);
    cmp("u2b-u5e", "\x2b", "\x5e", 0);
    cmp("u2b-u5f", "\x2b", "\x5f", 0);
    cmp("u2b-u60", "\x2b", "\x60", 0);
    cmp("u2b-u7b", "\x2b", "\x7b", 0);
    cmp("u2b-u7c", "\x2b", "\x7c", 0);
    cmp("u2b-u7d", "\x2b", "\x7d", 0);
    cmp("u2b-u7e", "\x2b", "\x7e", 0);
    cmp("u3c-u3f", "\x3c", "\x3f", 0);
    cmp("u3c-u40", "\x3c", "\x40", 0);
    cmp("u3c-u5b", "\x3c", "\x5b", 0);
    cmp("u3c-u5c", "\x3c", "\x5c", 0);
    cmp("u3c-u5d", "\x3c", "\x5d", 0);
    cmp("u3c-u5e", "\x3c", "\x5e", 0);
    cmp("u3c-u5f", "\x3c", "\x5f", 0);
    cmp("u3c-u60", "\x3c", "\x60", 0);
    cmp("u3c-u7b", "\x3c", "\x7b", 0);
    cmp("u3c-u7c", "\x3c", "\x7c", 0);
    cmp("u3c-u7d", "\x3c", "\x7d", 0);
    cmp("u3c-u7e", "\x3c", "\x7e", 0);
    cmp("u3d-u3f", "\x3d", "\x3f", 0);
    cmp("u3d-u40", "\x3d", "\x40", 0);
    cmp("u3d-u5b", "\x3d", "\x5b", 0);
    cmp("u3d-u5c", "\x3d", "\x5c", 0);
    cmp("u3d-u5d", "\x3d", "\x5d", 0);
    cmp("u3d-u5e", "\x3d", "\x5e", 0);
    cmp("u3d-u5f", "\x3d", "\x5f", 0);
    cmp("u3d-u60", "\x3d", "\x60", 0);
    cmp("u3d-u7b", "\x3d", "\x7b", 0);
    cmp("u3d-u7c", "\x3d", "\x7c", 0);
    cmp("u3d-u7d", "\x3d", "\x7d", 0);
    cmp("u3d-u7e", "\x3d", "\x7e", 0);
    cmp("u3e-u3f", "\x3e", "\x3f", 0);
    cmp("u3e-u40", "\x3e", "\x40", 0);
    cmp("u3e-u5b", "\x3e", "\x5b", 0);
    cmp("u3e-u5c", "\x3e", "\x5c", 0);
    cmp("u3e-u5d", "\x3e", "\x5d", 0);
    cmp("u3e-u5e", "\x3e", "\x5e", 0);
    cmp("u3e-u5f", "\x3e", "\x5f", 0);
    cmp("u3e-u60", "\x3e", "\x60", 0);
    cmp("u3e-u7b", "\x3e", "\x7b", 0);
    cmp("u3e-u7c", "\x3e", "\x7c", 0);
    cmp("u3e-u7d", "\x3e", "\x7d", 0);
    cmp("u3e-u7e", "\x3e", "\x7e", 0);
    cmp("u09-u20", "\x09", "\x20", 0);
    cmp("u0a-u20", "\x0a", "\x20", 0);
    cmp("u0b-u20", "\x0b", "\x20", 0);
    cmp("u0c-u20", "\x0c", "\x20", 0);
    cmp("u0d-u20", "\x0d", "\x20", 0);
    cmp("u21-u22", "\x21", "\x22", 0);
    cmp("u21-u23", "\x21", "\x23", 0);
    cmp("u21-u24", "\x21", "\x24", 0);
    cmp("u21-u25", "\x21", "\x25", 0);
    cmp("u21-u26", "\x21", "\x26", 0);
    cmp("u21-u28", "\x21", "\x28", 0);
    cmp("u21-u29", "\x21", "\x29", 0);
    cmp("u21-u2a", "\x21", "\x2a", 0);
    cmp("u21-u2b", "\x21", "\x2b", 0);
    cmp("u21-u2c", "\x21", "\x2c", 0);
    cmp("u21-u2e", "\x21", "\x2e", 0);
    cmp("u21-u2f", "\x21", "\x2f", 0);
"#;
    let (rc, out, err) = cmp_run("ia2-cmp-ascii-symbols", cases);
    assert_eq!(rc, 0, "{out:?} {err}");
    assert_eq!(
        out,
        [
            "u2b-u2c 3", "u2b-u2e 3", "u2b-u2f 3", "u2b-u3a 3", "u2b-u3b 3", "u2b-u3f 3",
            "u2b-u40 3", "u2b-u5b 3", "u2b-u5c 3", "u2b-u5d 3", "u2b-u5e 3", "u2b-u5f 3",
            "u2b-u60 3", "u2b-u7b 3", "u2b-u7c 3", "u2b-u7d 3", "u2b-u7e 3", "u3c-u3f 3",
            "u3c-u40 3", "u3c-u5b 3", "u3c-u5c 3", "u3c-u5d 3", "u3c-u5e 3", "u3c-u5f 3",
            "u3c-u60 3", "u3c-u7b 3", "u3c-u7c 3", "u3c-u7d 3", "u3c-u7e 3", "u3d-u3f 3",
            "u3d-u40 3", "u3d-u5b 3", "u3d-u5c 3", "u3d-u5d 3", "u3d-u5e 3", "u3d-u5f 3",
            "u3d-u60 3", "u3d-u7b 3", "u3d-u7c 3", "u3d-u7d 3", "u3d-u7e 3", "u3e-u3f 3",
            "u3e-u40 3", "u3e-u5b 3", "u3e-u5c 3", "u3e-u5d 3", "u3e-u5e 3", "u3e-u5f 3",
            "u3e-u60 3", "u3e-u7b 3", "u3e-u7c 3", "u3e-u7d 3", "u3e-u7e 3", "u09-u20 3",
            "u0a-u20 3", "u0b-u20 3", "u0c-u20 3", "u0d-u20 3", "u21-u22 1", "u21-u23 1",
            "u21-u24 1", "u21-u25 1", "u21-u26 1", "u21-u28 1", "u21-u29 1", "u21-u2a 1",
            "u21-u2b 1", "u21-u2c 1", "u21-u2e 1", "u21-u2f 1",
        ]
    );
}

#[test]
fn compare_string_leaves_the_pairs_the_table_does_not_decide_to_the_stop() {
    // equal primary weights (hiragana A / katakana A: SM 3 AW 2 both; only the case weight differs: 39 / 7) -> the old code's stop
    let hira_kata = "cmp(\"x\", \"\u{3042}\", \"\u{30a2}\", 0);";
    let (rc, out, err) = cmp_run("ia2-cmp-tie", hira_kata);
    assert_ne!(rc, 0, "{out:?}");
    assert!(err.contains("CompareStringW: character other than an ASCII letter or digit in unequal strings"), "{err}");
    // two ideographs of the same script member (SM 5): their AW is not carried in this build -> the old code's stop
    let han = "cmp(\"x\", \"\u{4e00}\", \"\u{65e5}\", 0);";
    let (rc, out, err) = cmp_run("ia2-cmp-han", han);
    assert_ne!(rc, 0, "{out:?}");
    assert!(err.contains("CompareStringW: character other than an ASCII letter or digit in unequal strings"), "{err}");
    // a non-spacing mark (SM 1: U+0301) has no primary weight
    let mark = "cmp(\"x\", \"\u{0301}\", \"a\", 0);";
    let (rc, out, err) = cmp_run("ia2-cmp-mark", mark);
    assert_ne!(rc, 0, "{out:?}");
    assert!(err.contains("CompareStringW"), "{err}");
    // (decision 0284: the hyphen against a unit above U+007F is no longer left to the stop: see compare_string_takes_the_hyphen_out_against_any_unit below)
}

#[test]
fn compare_string_takes_the_hyphen_out_against_any_unit() {
    // Decision 0284. Learn "Handling Sorting in Your Applications": the hyphen and the apostrophe are "treated differently from the other nonalphanumeric characters to ensure that words such as "coop" and "co-op" stay
    // together"; the rule of decision 0178/0229 (they do not take part in the comparison of the other units; the string with more of them is the greater when the rest is equal) does not depend on the class of the OTHER
    // unit, so it now also applies to a unit above U+007F or a control unit (before: only against printable ASCII). Hand derivation: "-" against HIRAGANA A: without the hyphen the first string is empty, the second is not:
    // empty < non-empty -> 1. "-x" against ESC "x": the tails are "x" and ESC "x"; ESC (SM 6) < x (SM 14) -> the second tail is the smaller -> the first string is the greater -> 3.
    // "a-" against "a" + HIRAGANA A: tails "" and HIRAGANA A -> 1. "-" against "-": equal strings -> 2 (unchanged).
    let cases = r#"
    cmp("hyphen-vs-kana", "-", "\u3042", 0);
    cmp("kana-vs-hyphen", "\u3042", "-", 0);
    cmp("hyphen-x-vs-esc-x", "-x", "\033x", 0);
    cmp("a-hyphen-vs-a-kana", "a-", "a\u3042", 0);
    cmp("apostrophe-vs-esc", "'", "\033", 0);
    cmp("equal", "-", "-", 0);
    "#
    .replace("\\u3042", "\u{3042}");
    let (rc, out, err) = cmp_run("ia2-cmp-hyphen-any", &cases);
    assert_eq!(rc, 0, "{out:?} {err}");
    assert_eq!(out, ["hyphen-vs-kana 1", "kana-vs-hyphen 3", "hyphen-x-vs-esc-x 3", "a-hyphen-vs-a-kana 1", "apostrophe-vs-esc 1", "equal 2"]);
}

#[test]
fn compare_string_orders_the_ascii_control_units_by_the_primary_weights_of_the_table() {
    // Decision 0284. Lines of Windows 10 Sorting Weight Table.txt (default table; columns CP, SM, AW, DW, CW; SHA-256 81fcfa1e...):
    //   0x0007 BEL SM 6 AW 9 | 0x0009 TAB SM 7 AW 5 | 0x000A LF SM 7 AW 6 | 0x000D CR SM 7 AW 9 | 0x001B ESC SM 6 AW 24 | 0x007F DEL SM 6 AW 29 | 'O' 0x004F SM 14 | 0x3042 HIRAGANA A SM 3 AW 2 | 0x0000 NUL: 0 0 0 0 = no weight
    // The primary weight is (SM, AW) compared as numbers. The guest compares the character of a key with the accelerator letter of a button of its error window (decision 0136; the Escape key is U+001B).
    // ESC (SM 6) < 'O' (SM 14) -> 1; the reverse -> 3. LF (7,6) < CR (7,9) -> 1. BEL (6,9) < ESC (6,24) -> the pair (ESC, BEL) -> 3. DEL (6,29) > ESC (6,24) -> 3. TAB (SM 7) > ESC (SM 6) -> 3.
    // ESC (SM 6) against HIRAGANA A (SM 3): the kana is the smaller -> 3. A control unit in the common prefix does not decide: "a ESC b" against "a ESC c" -> 1 (b < c).
    let cases = r#"
    cmp("esc-vs-O", "\033", "O", 1);
    cmp("O-vs-esc", "O", "\033", 1);
    cmp("lf-vs-cr", "\n", "\r", 0);
    cmp("cr-vs-lf", "\r", "\n", 0);
    cmp("esc-vs-bel", "\033", "\007", 0);
    cmp("del-vs-esc", "\177", "\033", 0);
    cmp("tab-vs-esc", "\t", "\033", 0);
    cmp("esc-vs-kana", "\033", "\u3042", 0);
    cmp("prefix-esc", "a\033b", "a\033c", 0);
    "#
    .replace("\\u3042", "\u{3042}");
    let (rc, out, err) = cmp_run("ia2-cmp-ctl", &cases);
    assert_eq!(rc, 0, "{out:?} {err}");
    assert_eq!(out, ["esc-vs-O 1", "O-vs-esc 3", "lf-vs-cr 1", "cr-vs-lf 3", "esc-vs-bel 3", "del-vs-esc 3", "tab-vs-esc 3", "esc-vs-kana 3", "prefix-esc 1"]);
}

#[test]
fn a_stop_of_compare_string_names_the_two_strings_the_flags_and_the_first_difference() {
    // Decision 0284 (diagnostic only). The first line of the message is the one of before; the second line holds what was compared: lcid, flags, lengths, both strings (a unit that is not printable ASCII, and the
    // quote, as \uXXXX), the first position where the case-folded units differ and the units there, and the guest return address (here the harness's stack slot holds 0). The pair that stops is a unit with no
    // weight in the table (U+00AD SOFT HYPHEN: "0 0 0 0" = no weight, "effectively ignored by NLS") against 'O'.
    let (rc, out, err) = cmp_run("ia2-cmp-diag", "cmp(\"x\", \"\u{00ad}[\\\"q\", \"O\\\\x\", 1);");
    assert_ne!(rc, 0, "{out:?}");
    assert!(err.contains("fle: CompareStringW: character other than an ASCII letter or digit in unequal strings (not defined by the MSDN page or outside the emulated locale; not implemented)\n"), "{err}");
    assert!(err.contains("fle: CompareStringW: lcid=0x409 flags=0x1 len1=4 len2=3 string1=\"\\u00AD[\\u0022q\" string2=\"O\\x\" first-difference=0 units=U+00AD/U+004F guest-return-address=0x0\n"), "{err}");
    // a long string is cut at 160 units and says how many are left: 170 'a' and SOFT HYPHEN (171 units) against 170 'a' and 'O' (171 units): the first difference is at 170
    let a = "a".repeat(170);
    let (rc, out, err) = cmp_run("ia2-cmp-diag-long", &format!("cmp(\"x\", \"{a}\u{00ad}\", \"{a}O\", 1);"));
    assert_ne!(rc, 0, "{out:?}");
    let shown = "a".repeat(160);
    assert!(err.contains(&format!("len1=171 len2=171 string1=\"{shown}\"...(+11 units) string2=\"{shown}\"...(+11 units) first-difference=170 units=U+00AD/U+004F guest-return-address=0x0\n")), "{err}");
}

#[test]
fn a_unit_with_no_weight_in_the_table_still_stops_by_name() {
    // decision 0284 (old: "without the marker row compare_string is as before": a kana against UNSORTED stopped without the row; the row is gone and the kana is answered, see the first test). What stays a stop:
    // U+00AD SOFT HYPHEN has no weight (0 0 0 0) in the table, so no primary weight decides it.
    let nowt = "cmp(\"nowt\", \"\u{00ad}\", \"UNSORTED\", 0);";
    let (rc, out, err) = cmp_run("ia2-cmp-noweight", nowt);
    assert_ne!(rc, 0, "{out:?}");
    assert!(out.is_empty(), "{out:?}");
    assert!(err.contains("CompareStringW: character other than an ASCII letter or digit in unequal strings"), "{err}");
}

#[test]
fn a_non_ascii_unit_in_the_common_prefix_is_accepted_only_with_a_table_entry() {
    // "\u3042x" against "\u3042y": the common prefix holds a kana (in the table), the first difference is x against y (ASCII letters)
    let cases = "cmp(\"prefix\", \"\u{3042}x\", \"\u{3042}y\", 0);";
    let (rc, out, err) = cmp_run("ia2-cmp-prefix-on", cases);
    assert_eq!((rc, out.clone()), (0, vec!["prefix 1".to_string()]), "{err}");
    // decision 0284 (old: the same strings without the marker row stopped): a unit that is NOT in the table, U+FEFF ZERO WIDTH NO-BREAK SPACE (0 0 0 0, no weight), in the common prefix still stops
    let (rc, out, err) = cmp_run("ia2-cmp-prefix-noweight", "cmp(\"prefix\", \"\u{feff}x\", \"\u{feff}y\", 0);");
    assert_ne!(rc, 0, "{out:?}");
    assert!(err.contains("before the first difference"), "{err}");
}

const SCROLL_PRE: &str = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_win.h"
#include "fle_raster.h"
#include "fle_gdi.h"
#include "fle_wall_ia2.h"
uint64_t R[16];
void fle_stop(const char *api, const char *why) { printf("STOP %s: %s\n", api, why); fflush(stdout); exit(3); }
/* stubs of the window table and the pixel model: an 8 x 8 BGRX client area at the origin of its surface; pixel (x, y) holds y * 8 + x */
#define W 8
#define H 8
static uint32_t pix[W * H];
static fle_rsurf surf;
static int dc_ok = 1;
static uint32_t last_err;
static int inval_calls; static int32_t inval[4]; static uint64_t inval_erase, inval_hwnd;
int fle_win_dc_query(uint64_t hwnd, fle_win_dcq *out) { if (!dc_ok) return 0; memset(out, 0, sizeof *out); out->cx = W; out->cy = H; out->root_lid = 14; out->root_cx = W; out->root_cy = H; out->clip_l = 0; out->clip_t = 0; out->clip_r = W; out->clip_b = H; (void)hwnd; return 1; }
const fle_rsurf *fle_raster_surface_peek(uint32_t lid) { (void)lid; return &surf; }
int fle_r_direct(const fle_rsurf *s) { (void)s; return 1; }
uint8_t *fle_r_row(const fle_rsurf *s, uint32_t y) { return s->bits + (size_t)y * s->stride; }
void fle_imp_InvalidateRect(void) { inval_calls++; inval_hwnd = R[1]; inval_erase = R[8]; if (R[2]) memcpy(inval, (const void *)(uintptr_t)R[2], 16); R[0] = 1; }
void fle_set_last_error(uint32_t e) { last_err = e; }
static int rgn_calls; static int32_t rgn[5];
/* decision 0319: the window model (fle_win.c) is not linked here: it "cannot answer" and the former simple path of fle_wall_ia2w.c answers, which is what these tests check */
int fle_win_scroll(const char *api, uint64_t hwnd, int32_t dx, int32_t dy, const int32_t *scroll, const int32_t *clip, uint32_t flags, fle_win_scroll_result *res) { (void)api; (void)hwnd; (void)dx; (void)dy; (void)scroll; (void)clip; (void)flags; (void)res; return 0; }
void fle_imp_SetRectRgn(void) { rgn_calls++; rgn[0] = (int32_t)R[1]; rgn[1] = (int32_t)R[2]; rgn[2] = (int32_t)R[8]; rgn[3] = (int32_t)R[9]; rgn[4] = (int32_t)((uint64_t *)(uintptr_t)R[4])[5]; R[0] = 1; }
static int rgnbox_kind = 2; static int32_t rgnbox[4] = { 1, 2, 5, 6 };
void fle_imp_GetRgnBox(void) { if (rgnbox_kind) memcpy((void *)(uintptr_t)R[2], rgnbox, 16); R[0] = (uint64_t)rgnbox_kind; }
static uint64_t st[16];
#define P(x) ((uint64_t)(uintptr_t)(x))
static void reset(void) { int i; for (i = 0; i < W * H; i++) pix[i] = (uint32_t)i; inval_calls = 0; memset(inval, 0, sizeof inval); inval_erase = 99; }
/* ScrollWindowEx(hwnd, dx, dy, prcScroll, prcClip, hrgnUpdate, prcUpdate, flags) */
static void scroll(int dx, int dy, const int32_t *rs, const int32_t *rc, uint64_t hrgn, int32_t *ru, uint32_t flags)
{
    R[1] = 0x7f990000; R[2] = (uint64_t)(int64_t)dx; R[8] = (uint64_t)(int64_t)dy; R[9] = rs ? P(rs) : 0; st[5] = rc ? P(rc) : 0; st[6] = hrgn; st[7] = ru ? P(ru) : 0; st[8] = flags; R[4] = P(st); R[0] = 0xDEAD;
    fle_imp_ScrollWindowEx();
}
static void show(const char *label, int ret, const int32_t *ru)
{
    int y;
    printf("%s ret %d", label, ret);
    if (ru) printf(" update (%d,%d,%d,%d)", ru[0], ru[1], ru[2], ru[3]);
    printf(" inval %d", inval_calls);
    if (inval_calls) printf(" (%d,%d,%d,%d) erase %llu", inval[0], inval[1], inval[2], inval[3], (unsigned long long)inval_erase);
    printf(" col0:");
    for (y = 0; y < H; y++) printf(" %u", pix[y * W]);
    printf("\n");
}
"#;

#[test]
fn scroll_window_ex_moves_the_bits_and_reports_the_uncovered_area() {
    let src = format!(
        "{SCROLL_PRE}{}",
        r#"
int main(void)
{
    int32_t ru[4], clip[4] = { 0, 0, 8, 4 };
    surf.bits = (uint8_t *)pix; surf.width = W; surf.height = H; surf.stride = W * 4; surf.bpp = 32;
    setvbuf(stdout, NULL, _IONBF, 0);
    reset(); scroll(0, 2, NULL, NULL, 0, ru, 2); show("down2", (int)R[0], ru);         /* SW_INVALIDATE */
    reset(); scroll(0, -3, NULL, NULL, 0, ru, 6); show("up3", (int)R[0], ru);          /* SW_INVALIDATE | SW_ERASE */
    reset(); scroll(2, 1, NULL, NULL, 0, ru, 2); show("diag", (int)R[0], ru);          /* an L-shaped uncovered area */
    reset(); scroll(0, 2, NULL, NULL, 0, ru, 0); show("noflags", (int)R[0], ru);       /* no invalidation without SW_INVALIDATE / SW_ERASE */
    reset(); scroll(0, 0, NULL, NULL, 0, ru, 2); show("zero", (int)R[0], ru);
    reset(); scroll(0, 2, NULL, clip, 0, ru, 2); show("clip", (int)R[0], ru);          /* the clip rectangle limits what is drawn */
    reset(); scroll(0, 2, NULL, NULL, 0, NULL, 2); show("noupdate", (int)R[0], NULL);
    reset(); scroll(0, 20, NULL, NULL, 0, ru, 2); show("far", (int)R[0], ru);
    reset(); scroll(0, 2, NULL, NULL, 0x1234, ru, 2); show("hrgn", (int)R[0], ru); printf("setrectrgn %d (%#x,%d,%d,%d,%d)\n", rgn_calls, rgn[0], rgn[1], rgn[2], rgn[3], rgn[4]);          /* scrolled out of the area: everything is uncovered */
    dc_ok = 0; reset(); scroll(0, 2, NULL, NULL, 0, ru, 2); printf("badwnd ret %d err %u\n", (int)R[0], last_err);
    return 0;
}
"#
    );
    let (rc, out, err) = run("ia2-scroll", &["fle_wall_ia2w.c"], &src);
    assert_eq!(rc, 0, "{out:?} {err}");
    // The Learn page: SIMPLEREGION 2 (a rectangular invalidated region), COMPLEXREGION 3, NULLREGION 1; the uncovered area is returned in prcUpdate; bits move by (dx, dy); with SW_ERASE the invalidation asks for the erase.
    assert_eq!(
        out,
        [
            "down2 ret 2 update (0,0,8,2) inval 1 (0,0,8,2) erase 0 col0: 0 8 0 8 16 24 32 40",
            "up3 ret 2 update (0,5,8,8) inval 1 (0,5,8,8) erase 1 col0: 24 32 40 48 56 40 48 56",
            "diag ret 3 update (0,0,8,8) inval 1 (0,0,8,8) erase 0 col0: 0 8 16 24 32 40 48 56",
            "noflags ret 2 update (0,0,8,2) inval 0 col0: 0 8 0 8 16 24 32 40",
            "zero ret 1 update (0,0,0,0) inval 0 col0: 0 8 16 24 32 40 48 56",
            "clip ret 2 update (0,0,8,2) inval 1 (0,0,8,2) erase 0 col0: 0 8 0 8 32 40 48 56",
            "noupdate ret 2 inval 1 (0,0,8,2) erase 0 col0: 0 8 0 8 16 24 32 40",
            "far ret 2 update (0,0,8,8) inval 1 (0,0,8,8) erase 0 col0: 0 8 16 24 32 40 48 56",
            "hrgn ret 2 update (0,0,8,2) inval 1 (0,0,8,2) erase 0 col0: 0 8 0 8 16 24 32 40",
            "setrectrgn 1 (0x1234,0,0,8,2)",
            "badwnd ret 0 err 1400",
        ]
    );
}

#[test]
fn scroll_window_ex_stops_by_name_where_the_page_leaves_the_behaviour_open() {
    for (label, call, expect) in [
        ("children", "scroll(0, 2, NULL, NULL, 0, ru, 1);", "STOP ScrollWindowEx: SW_SCROLLCHILDREN"),
        ("negrect", "{ int32_t bad[4] = { 5, 5, 2, 2 }; scroll(0, 2, bad, NULL, 0, ru, 2); }", "STOP ScrollWindowEx: prcScroll has a negative size"),
    ] {
        let src = format!("{SCROLL_PRE}int main(void) {{ int32_t ru[4]; surf.bits = (uint8_t *)pix; surf.width = W; surf.height = H; surf.stride = W * 4; surf.bpp = 32; setvbuf(stdout, NULL, _IONBF, 0); reset(); {call} printf(\"not reached\\n\"); return 0; }}\n");
        let (rc, out, err) = run(&format!("ia2-scroll-stop-{label}"), &["fle_wall_ia2w.c"], &src);
        assert_eq!(rc, 3, "{label}: {out:?} {err}");
        assert!(out.last().unwrap().starts_with(expect), "{label}: {out:?}");
    }
}

#[test]
fn do_drag_drop_answers_cancel_and_leaves_the_effect_alone() {
    let src = r#"
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_wall_ia2.h"
uint64_t R[16];
void fle_stop(const char *api, const char *why) { printf("STOP %s: %s\n", api, why); fflush(stdout); exit(3); }
int main(void)
{
    uint32_t eff = 0x1234; int dummy;
    setvbuf(stdout, NULL, _IONBF, 0);
    R[1] = (uint64_t)(uintptr_t)&dummy; R[2] = (uint64_t)(uintptr_t)&dummy; R[8] = 3; R[9] = (uint64_t)(uintptr_t)&eff; R[0] = 0xDEAD;
    fle_imp_DoDragDrop();
    printf("ret %#llx effect %#x\n", (unsigned long long)R[0], eff);
    R[2] = 0;
    fle_imp_DoDragDrop();
    printf("not reached\n");
    return 0;
}
"#;
    let (rc, out, err) = run("ia2-dragdrop", &["fle_wall_ia2.c"], src);
    assert_eq!(rc, 3, "{out:?} {err}");
    // DRAGDROP_S_CANCEL 0x00040101; "The pdwEffect parameter is set only if the operation is not canceled".
    assert_eq!(out[0], "ret 0x40101 effect 0x1234");
    assert!(out[1].starts_with("STOP DoDragDrop: pDropSource is NULL"), "{out:?}");
}

/// LCMapStringW and the LoadLibraryW path list through fle_locale.c / fle_wall_ia2.c (decision 0284: no marker row, no stub response table).
fn lcmap_run(name: &str, body: &str) -> (i32, Vec<String>, String) {
    let src = format!(
        r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_locale.h"
#include "fle_modules.h"
#include "fle_wall_ia2.h"
uint64_t R[16];
static uint64_t st[16];
void fle_stop(const char *api, const char *why) {{ printf("STOP %s: %s\n", api, why); fflush(stdout); exit(3); }}
#define P(...) ((uint64_t)(uintptr_t)(__VA_ARGS__))
/* LCMapStringW(LCID 0x409, flags, src, n, dst, cchDest 64); prints the result units */
static void lcmap(const char *label, uint32_t flags, const uint16_t *s, int n)
{{
    uint16_t d[64]; int i, r;
    memset(d, 0xAA, sizeof d);
    R[1] = 0x409; R[2] = flags; R[8] = P(s); R[9] = (uint64_t)(int64_t)n; st[5] = P(d); st[6] = 64; R[4] = P(st); R[0] = 0xDEAD;
    fle_imp_LCMapStringW();
    r = (int)R[0];
    printf("%s ret %d:", label, r);
    for (i = 0; i < r; i++) printf(" %04x", d[i]);
    printf("\n");
}}
int main(void) {{ setvbuf(stdout, NULL, _IONBF, 0); {body} return 0; }}
"#
    );
    run(name, &["fle_locale.c", "fle_imports.c", "fle_wall_ia2.c"], &src)
}

#[test]
fn lc_map_string_maps_private_use_units_to_themselves() {
    // "Kick" + U+E5A0 + " (" + "I" + ")" + U+E5FF with LCMAP_LOWERCASE | LCMAP_LINGUISTIC_CASING (0x01000100), the engine's call (decision 0191 wall W25). Microsoft's Windows 8 Upper Case Mapping Table lists no unit of U+E000..U+F8FF.
    let body = r#"
    static const uint16_t a[] = { 'K', 'I', 'c', 'k', 0xE5A0, ' ', '(', 'I', ')', 0xE5FF };
    lcmap("linguistic-lower", 0x01000100u, a, 10);
    lcmap("plain-lower", 0x100u, a, 10);
    lcmap("linguistic-upper", 0x01000200u, a, 10);
"#;
    let (rc, out, err) = lcmap_run("ia2-lcmap-on", body);
    assert_eq!(rc, 0, "{out:?} {err}");
    assert_eq!(
        out,
        [
            "linguistic-lower ret 10: 006b 0069 0063 006b e5a0 0020 0028 0069 0029 e5ff",
            "plain-lower ret 10: 006b 0069 0063 006b e5a0 0020 0028 0069 0029 e5ff",
            "linguistic-upper ret 10: 004b 0049 0043 004b e5a0 0020 0028 0049 0029 e5ff",
        ]
    );
    // a unit that the table maps and that is outside the old range (U+0391 GREEK CAPITAL ALPHA) is still a stop
    let (rc, out, err) = lcmap_run("ia2-lcmap-on-greek", "static const uint16_t a[] = { 0x0391 }; lcmap(\"x\", 0x100u, a, 1);");
    assert_ne!(rc, 0, "{out:?}");
    assert!(err.contains("outside the range D"), "{err}");
}

#[test]
fn the_relative_python_module_is_not_found() {
    // decision 0284 (old: "only with the marker": without the row the same names were not answered, the row is gone)
    let prog = r#"
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include "fle_runtime.h"
#include "fle_modules.h"
#include "fle_wall_ia2.h"
uint64_t R[16];
void fle_stop(const char *api, const char *why) { printf("STOP %s: %s\n", api, why); fflush(stdout); exit(3); }
static int t(const char *s) { uint16_t w[64]; size_t n = 0; while (s[n]) { w[n] = (unsigned char)s[n]; n++; } return fle_ia2_path_absent(w, n); }
int main(void) {
    printf("exact %d upper %d other %d nopath %d\n", t("Python\\Python312.dll"), t("PYTHON\\PYTHON312.DLL"), t("Python\\Python311.dll"), t("python312.dll"));
    return 0;
}
"#;
    let (rc, out, err) = run("ia2-pypath", &["fle_wall_ia2.c"], prog);
    assert_eq!((rc, out), (0, vec!["exact 1 upper 1 other 0 nopath 0".to_string()]), "{err}");
}

const INI_PRE: &str = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_vfs.h"
#include "fle_wall_ia2.h"
uint64_t R[16];
void fle_stop(const char *api, const char *why) { printf("STOP %s: %s\n", api, why); fflush(stdout); exit(3); }
static uint32_t last_err = 99;
void fle_set_last_error(uint32_t e) { last_err = e; }
/* stubs of the virtual volume: one file "C:\Windows\a.ini" (and a file "C:\x\u.url") */
static const char *ini_text = "[InternetShortcut]\r\nURL=https://x.test/a\r\nShowCommand=3\r\n[Other]\r\nkey = 'quoted'\r\nK2=  spaced value  \r\n; comment\r\n  [ Third ]  \r\nz=1\r\n";
static char opened[100];
static char written[4000]; static size_t written_n; static int write_opens;
void fle_imp_CreateFileW(void)
{
    const uint16_t *w = (const uint16_t *)(uintptr_t)R[1]; size_t i;
    for (i = 0; w[i] && i < 99; i++) opened[i] = (char)w[i];
    opened[i] = 0;
    if (R[2] == 0x40000000u) { write_opens++; R[0] = 0x200; return; }                    /* GENERIC_WRITE: always works */
    R[0] = (!strcmp(opened, "C:\\Windows\\a.ini") || !strcmp(opened, "C:\\x\\a.ini")) ? 0x100 : UINT64_MAX;
}
void fle_imp_CloseHandle(void) { R[0] = 1; }
void fle_imp_WriteFile(void) { memcpy(written, (const void *)(uintptr_t)R[2], (size_t)R[8]); written_n = (size_t)R[8]; *(uint32_t *)(uintptr_t)R[9] = (uint32_t)R[8]; R[0] = 1; }
int fle_vfs_file_info(const char *api, uint64_t h, const uint8_t **data, uint64_t *size, unsigned *can_read, unsigned *can_write) { (void)api; (void)h; *data = (const uint8_t *)ini_text; *size = strlen(ini_text); *can_read = 1; *can_write = 0; return 1; }
static uint16_t *W(const char *s) { size_t n = strlen(s), i; uint16_t *d = calloc(n + 1, 2); for (i = 0; i < n; i++) d[i] = (unsigned char)s[i]; return d; }
static uint16_t out[64];
static uint64_t st[16];
#define P(x) ((uint64_t)(uintptr_t)(x))
/* GetPrivateProfileStringW(app, key, def, out, nSize, file) */
static void gp(const char *label, const char *app, const char *key, const char *def, uint32_t nsize, const char *file)
{
    int i, n;
    memset(out, 0xAA, sizeof out);
    R[1] = app ? P(W(app)) : 0; R[2] = key ? P(W(key)) : 0; R[8] = def ? P(W(def)) : 0; R[9] = P(out); st[5] = nsize; st[6] = P(W(file)); R[4] = P(st); last_err = 99; R[0] = 0xDEAD;
    fle_imp_GetPrivateProfileStringW();
    n = (int)R[0];
    printf("%s ret %d err %u opened '%s':", label, n, last_err, opened);
    for (i = 0; i < n + 2 && i < 40; i++) { if (out[i] == 0) printf(" |"); else if (out[i] == 0xAAAA) printf(" ??"); else printf(" %c", out[i]); }
    printf("\n");
}
"#;

#[test]
fn get_private_profile_string_follows_the_page() {
    let src = format!(
        "{INI_PRE}{}",
        r#"
int main(void)
{
    setvbuf(stdout, NULL, _IONBF, 0);
    gp("url", "internetshortcut", "url", "d", 64, "C:\\x\\a.ini");                 /* case-insensitive, first match */
    gp("quoted", "OTHER", "KEY", "d", 64, "C:\\x\\a.ini");                        /* the quotation marks are discarded */
    gp("spaced", "Other", "K2", "d", 64, "C:\\x\\a.ini");                         /* blanks around the value are not part of it */
    gp("default", "Other", "nokey", "def  ", 64, "C:\\x\\a.ini");                 /* the default, trailing blanks stripped */
    gp("nulldef", "Other", "nokey", NULL, 64, "C:\\x\\a.ini");                    /* NULL default = "" */
    gp("nosection", "Nope", "url", "dd", 64, "C:\\x\\a.ini");
    gp("trunc", "InternetShortcut", "URL", "d", 5, "C:\\x\\a.ini");               /* ret = nSize - 1 */
    gp("relative", "InternetShortcut", "URL", "d", 64, "a.ini");                  /* no path: the Windows directory */
    gp("missing", "InternetShortcut", "URL", "dflt", 64, "C:\\x\\none.ini");      /* not found: the default and error 2 */
    gp("sections", NULL, NULL, "d", 64, "C:\\x\\a.ini");
    gp("keys", "InternetShortcut", NULL, "d", 64, "C:\\x\\a.ini");
    gp("sections-trunc", NULL, NULL, "d", 6, "C:\\x\\a.ini");                     /* the last string is cut, two NULs, ret = nSize - 2 */
    return 0;
}
"#
    );
    let (rc, out, err) = run("ia2-ini", &["fle_wall_ia2v.c"], &src);
    assert_eq!(rc, 0, "{out:?} {err}");
    assert_eq!(
        out,
        [
            "url ret 16 err 99 opened 'C:\\x\\a.ini': h t t p s : / / x . t e s t / a | ??",
            "quoted ret 6 err 99 opened 'C:\\x\\a.ini': q u o t e d | ??",
            "spaced ret 12 err 99 opened 'C:\\x\\a.ini': s p a c e d   v a l u e | ??",
            "default ret 3 err 99 opened 'C:\\x\\a.ini': d e f | ??",
            "nulldef ret 0 err 99 opened 'C:\\x\\a.ini': | ??",
            "nosection ret 2 err 99 opened 'C:\\x\\a.ini': d d | ??",
            "trunc ret 4 err 99 opened 'C:\\x\\a.ini': h t t p | ??",
            "relative ret 16 err 99 opened 'C:\\Windows\\a.ini': h t t p s : / / x . t e s t / a | ??",
            "missing ret 4 err 2 opened 'C:\\x\\none.ini': d f l t | ??",
            "sections ret 29 err 99 opened 'C:\\x\\a.ini': I n t e r n e t S h o r t c u t | O t h e r | T h i r d | | |",
            "keys ret 16 err 99 opened 'C:\\x\\a.ini': U R L | S h o w C o m m a n d | | |",
            "sections-trunc ret 4 err 99 opened 'C:\\x\\a.ini': I n t e | |",
        ]
    );
}

#[test]
fn message_box_and_shell_execute_answer_without_showing_or_starting_anything() {
    let src = r#"
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_wall_ia2.h"
uint64_t R[16];
void fle_stop(const char *api, const char *why) { printf("STOP %s: %s\n", api, why); fflush(stdout); exit(3); }
static uint16_t *W(const char *s) { size_t n = strlen(s), i; uint16_t *d = calloc(n + 1, 2); for (i = 0; i < n; i++) d[i] = (unsigned char)s[i]; return d; }
#define P(x) ((uint64_t)(uintptr_t)(x))
static uint64_t stk[4] = { 0x1234 };
static void mb(uint32_t type) { R[4] = P(stk); R[1] = 0; R[2] = P(W("Hello")); R[8] = P(W("Cap")); R[9] = type; fle_imp_MessageBoxW(); printf("type %#x -> %d\n", type, (int)R[0]); }
int main(void)
{
    setvbuf(stdout, NULL, _IONBF, 0);
    mb(0x0); mb(0x1); mb(0x2); mb(0x3); mb(0x4); mb(0x5); mb(0x6);          /* MB_OK ... MB_CANCELTRYCONTINUE: the first button */
    mb(0x104); mb(0x203); mb(0x103); mb(0x301);                                /* a second / third button as default (0x103: Yes/No/Cancel with the 2nd, No); a default beyond the buttons is the first */
    mb(0x30); mb(0x1040);                                                       /* icon and modality flags do not matter */
    R[1] = 0; R[2] = P(W("open")); R[8] = P(W("https://example.test/")); R[9] = 0; st: ;
    fle_imp_ShellExecuteW();
    printf("shellexecute -> %d\n", (int)R[0]);
    R[8] = 0;
    fle_imp_ShellExecuteW();
    printf("not reached\n");
    return 0;
}
"#
    .replace("#include <stdint.h>\n#include \"fle_runtime.h\"", "#include <stdint.h>\n#include <string.h>\n#include \"fle_runtime.h\"")
    .replace(" st: ;", "");
    let (rc, out, err) = run("ia2-msgbox", &["fle_wall_ia2.c"], &src);
    assert_eq!(rc, 3, "{out:?} {err}");
    // Learn MessageBox: IDOK 1, IDCANCEL 2, IDABORT 3, IDRETRY 4, IDIGNORE 5, IDYES 6, IDNO 7, IDTRYAGAIN 10; the buttons of each type in the order shown; MB_DEFBUTTON2 0x100, MB_DEFBUTTON3 0x200
    assert_eq!(
        out,
        [
            "type 0 -> 1", "type 0x1 -> 1", "type 0x2 -> 3", "type 0x3 -> 6", "type 0x4 -> 6", "type 0x5 -> 4", "type 0x6 -> 2",
            "type 0x104 -> 7", "type 0x203 -> 2", "type 0x103 -> 7", "type 0x301 -> 1",
            "type 0x30 -> 1", "type 0x1040 -> 1",
            "shellexecute -> 31",
            "STOP ShellExecuteW: lpFile is NULL: the page gives no behaviour",
        ]
    );
    // the one-line records on stderr name the choice
    assert!(err.contains("MessageBoxW NOT SHOWN") && err.contains("caller 0x1234, caption \"Cap\", text \"Hello\""), "{err}");
    assert!(err.contains("ShellExecuteW NOT CARRIED OUT") && err.contains("file \"https://example.test/\""), "{err}");
}

#[test]
fn write_private_profile_string_edits_the_text_per_the_page() {
    let src = format!(
        "{INI_PRE}{}",
        r#"
static void put(const char *label, const char *in, const char *app, const char *key, const char *str)
{
    uint8_t outb[2000]; size_t n, i;
    n = fle_ia2_ini_put((const uint8_t *)in, strlen(in), W(app), key ? W(key) : NULL, str ? W(str) : NULL, outb, sizeof outb);
    printf("%s [", label);
    for (i = 0; i < n; i++) { if (outb[i] == '\r') printf("\\r"); else if (outb[i] == '\n') printf("\\n"); else putchar(outb[i]); }
    printf("]\n");
}
int main(void)
{
    setvbuf(stdout, NULL, _IONBF, 0);
    put("replace", "[A]\r\nk=1\r\nj=2\r\n[B]\r\nk=3\r\n", "a", "K", "new");                  /* the first key of the first section of that name, case-insensitive; the key's spelling of THIS call */
    put("append-key", "[A]\r\nk=1\r\n\r\n[B]\r\nk=3\r\n", "A", "z", "9");                  /* after the last non-empty line of the section */
    put("new-section", "[A]\r\nk=1\r\n", "C", "x", "y");                                   /* at the end after one empty line */
    put("delete-key", "[A]\r\nk=1\r\nj=2\r\n", "A", "k", NULL);
    put("delete-section", "[A]\r\nk=1\r\n[B]\r\nk=3\r\n[C]\r\nq=4\r\n", "B", NULL, NULL);
    put("empty-file", "", "S", "k", "v");
    put("no-section-delete", "[A]\r\nk=1\r\n", "Z", "k", NULL);                           /* nothing to delete: unchanged */
    put("lf-only", "[A]\nk=1\n", "A", "j", "2");                                           /* the existing lines are kept, a new line has CR LF */
    put("other-section-key", "[A]\r\nk=1\r\n[B]\r\nj=2\r\n", "B", "k", "7");                /* a key of another section is not touched */
    put("no-final-eol", "[A]\r\nk=1", "A", "j", "2");
    return 0;
}
"#
    );
    // the program above prints with \\r / \\n as text; the expectations are what the page and the choices (S-ia2-ini-write) give
    let (rc, out, err) = run("ia2-iniput", &["fle_wall_ia2v.c"], &src);
    assert_eq!(rc, 0, "{out:?} {err}");
    assert_eq!(
        out,
        [
            "replace [[A]\\r\\nK=new\\r\\nj=2\\r\\n[B]\\r\\nk=3\\r\\n]",
            "append-key [[A]\\r\\nk=1\\r\\nz=9\\r\\n\\r\\n[B]\\r\\nk=3\\r\\n]",
            "new-section [[A]\\r\\nk=1\\r\\n\\r\\n[C]\\r\\nx=y\\r\\n]",
            "delete-key [[A]\\r\\nj=2\\r\\n]",
            "delete-section [[A]\\r\\nk=1\\r\\n[C]\\r\\nq=4\\r\\n]",
            "empty-file [[S]\\r\\nk=v\\r\\n]",
            "no-section-delete [[A]\\r\\nk=1\\r\\n]",
            "lf-only [[A]\\r\\nk=1\\r\\nj=2\\r\\n]",
            "other-section-key [[A]\\r\\nk=1\\r\\n[B]\\r\\nj=2\\r\\nk=7\\r\\n]",
            "no-final-eol [[A]\\r\\nk=1\\r\\nj=2\\r\\n]",
        ]
    );
}

#[test]
fn write_private_profile_string_reads_the_file_and_writes_it_back() {
    let src = format!(
        "{INI_PRE}{}",
        r#"
static uint64_t st2[16];
static int wp(const char *app, const char *key, const char *str, const char *file)
{
    R[1] = app ? P(W(app)) : 0; R[2] = key ? P(W(key)) : 0; R[8] = str ? P(W(str)) : 0; R[9] = file ? P(W(file)) : 0; R[4] = P(st2); written_n = 0; write_opens = 0; R[0] = 0xDEAD;
    fle_imp_WritePrivateProfileStringW();
    return (int)R[0];
}
static void dump(void) { size_t i; for (i = 0; i < written_n; i++) { if (written[i] == '\r') printf("\\r"); else if (written[i] == '\n') printf("\\n"); else putchar(written[i]); } printf("\n"); }
int main(void)
{
    setvbuf(stdout, NULL, _IONBF, 0);
    printf("ret %d writes %d path '%s'\n", wp("Other", "key", "changed", "C:\\x\\a.ini"), write_opens, opened);
    dump();
    printf("ret %d writes %d path '%s'\n", wp("Visits", "n", "1", "new.ini"), write_opens, opened);   /* a new file in the Windows directory */
    dump();
    printf("flush ret %d writes %d\n", wp(0, 0, 0, "C:\\x\\a.ini"), write_opens);   /* the setup recipe of the page: the first three NULL */
    printf("flush-all ret %d writes %d\n", wp(0, 0, 0, 0), write_opens);
    return 0;
}
"#
    );
    let (rc, out, err) = run("ia2-iniwrite", &["fle_wall_ia2v.c"], &src);
    assert_eq!(rc, 0, "{out:?} {err}");
    assert_eq!(out[0], "ret 1 writes 1 path 'C:\\x\\a.ini'");
    assert_eq!(out[1], "[InternetShortcut]\\r\\nURL=https://x.test/a\\r\\nShowCommand=3\\r\\n[Other]\\r\\nkey=changed\\r\\nK2=  spaced value  \\r\\n; comment\\r\\n  [ Third ]  \\r\\nz=1\\r\\n");
    assert_eq!(out[2], "ret 1 writes 1 path 'C:\\Windows\\new.ini'");
    assert_eq!(out[3], "[Visits]\\r\\nn=1\\r\\n");
    assert_eq!(out[4], "flush ret 0 writes 0");
    assert_eq!(out[5], "flush-all ret 0 writes 0");
}

#[test]
fn get_time_zone_information_for_year_gives_the_one_zone_of_the_machine() {
    let src = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_wall_ia2.h"
uint64_t R[16];
void fle_stop(const char *api, const char *why) { printf("STOP %s: %s\n", api, why); fflush(stdout); exit(3); }
static int tz_calls;
/* the zone of fle_time.c is stubbed: GetTimeZoneInformation(ptzi) writes a marker into the structure */
void fle_imp_GetTimeZoneInformation(void) { tz_calls++; *(uint32_t *)(uintptr_t)R[1] = 0xABCD; R[0] = 0; }
int main(void)
{
    uint32_t tzi[44];
    setvbuf(stdout, NULL, _IONBF, 0);
    memset(tzi, 0, sizeof tzi);
    R[1] = 2026; R[2] = 0; R[8] = (uint64_t)(uintptr_t)tzi; R[0] = 0xDEAD;
    fle_imp_GetTimeZoneInformationForYear();
    printf("ret %llu calls %d tzi0 %#x\n", (unsigned long long)R[0], tz_calls, tzi[0]);
    R[1] = 2026; R[2] = 0x1234; R[8] = (uint64_t)(uintptr_t)tzi;
    fle_imp_GetTimeZoneInformationForYear();
    printf("not reached\n");
    return 0;
}
"#;
    let (rc, out, err) = run("ia2-tzyear", &["fle_wall_ia2t.c"], src);
    assert_eq!(rc, 3, "{out:?} {err}");
    assert_eq!(out[0], "ret 1 calls 1 tzi0 0xabcd");
    assert!(out[1].starts_with("STOP GetTimeZoneInformationForYear: pdtzi is not NULL"), "{out:?}");
}

#[test]
fn invalidate_rgn_adds_the_bounding_box_of_the_region() {
    let src = format!(
        "{SCROLL_PRE}{}",
        r#"
static void irgn(const char *label, uint64_t hrgn, uint64_t erase)
{
    reset(); R[1] = 0x7f990000; R[2] = hrgn; R[8] = erase; R[0] = 0xDEAD;
    fle_imp_InvalidateRgn();
    printf("%s ret %llu inval %d", label, (unsigned long long)R[0], inval_calls);
    if (inval_calls) printf(" (%d,%d,%d,%d) erase %llu", inval[0], inval[1], inval[2], inval[3], (unsigned long long)inval_erase);
    printf("\n");
}
int main(void)
{
    setvbuf(stdout, NULL, _IONBF, 0);
    irgn("null", 0, 1);                       /* NULL region: the whole client area (InvalidateRect with a NULL rectangle) */
    rgnbox_kind = 2; irgn("rect", 0x1234, 1); /* the bounding box of the region */
    rgnbox_kind = 3; irgn("complex", 0x1234, 0);
    rgnbox_kind = 1; irgn("empty", 0x1234, 1);/* an empty region adds nothing; the return value is nonzero anyway */
    rgnbox_kind = 0; irgn("invalid", 0x1234, 0);
    printf("not reached\n");
    return 0;
}
"#
    );
    let (rc, out, err) = run("ia2-invrgn", &["fle_wall_ia2w.c"], &src);
    assert_eq!(rc, 3, "{out:?} {err}");
    assert_eq!(
        &out[..4],
        ["null ret 1 inval 1 (0,0,0,0) erase 1", "rect ret 1 inval 1 (1,2,5,6) erase 1", "complex ret 1 inval 1 (1,2,5,6) erase 0", "empty ret 1 inval 0"]
    );
    assert!(out[4].starts_with("STOP InvalidateRgn: hRgn is not a live region"), "{out:?}");
}
