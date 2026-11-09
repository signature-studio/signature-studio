//! Tests for the user32 no-window answers (runtime/fle_user_nowin.c) and the winmm answers (runtime/fle_winmm.c),
//! decision docs/decisions/0030-apis-a9-a10.md.
//!
//! Expected values are written HERE from the Microsoft Learn pages (listed in the decision) and from independent
//! models (ASCII table, rectangle arithmetic in Rust), never read back from the C code. The numbers of the MMRESULT
//! codes are from the SDK headers (mingw-w64 copy) and windows-sys; they are written out here on purpose, so that a
//! wrong number in the header of the runtime is caught. The host is Linux: nothing is compared with a real Windows.
//! No table is checked for a fixed number of rows. The C sources come from runtime/ when they are there, otherwise
//! from out-a9/work/ (development).

mod etxt_common;

use fe_lift::generate;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

const MMSYSERR_NOERROR: u64 = 0;
const MMSYSERR_BADDEVICEID: u64 = 2;
const MMSYSERR_INVALHANDLE: u64 = 5;
const MIDIERR_NODEVICE: u64 = 68;
const TIMERR_NOERROR: u64 = 0;
const TIMERR_NOCANDO: u64 = 97;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn runtime_dir() -> PathBuf {
    manifest().join("../../runtime")
}
fn src_dir() -> PathBuf {
    if runtime_dir().join("fle_user_nowin.c").exists() && runtime_dir().join("fle_winmm.c").exists() {
        runtime_dir()
    } else {
        manifest().join("../../out-a9/work")
    }
}
fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-a9/test");
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

fn objs() -> &'static Vec<PathBuf> {
    static O: OnceLock<Vec<PathBuf>> = OnceLock::new();
    O.get_or_init(|| {
        let dir = work_dir().join(etxt_common::per_process("obj-a9"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut v = Vec::new();
        for (name, ours) in [("fle_user_nowin.c", true), ("fle_winmm.c", true), ("fle_sys.c", false), ("fle_imports.c", false), ("fle_env.c", false)] {
            let base = if ours { src_dir() } else { runtime_dir() };
            let o = dir.join(name.replace(".c", ".o"));
            clang_c(&base.join(name), &o, ours);
            v.push(o);
        }
        v
    })
}

struct Out {
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
#include <limits.h>
#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_sys.h"
#include "fle_user_nowin.h"
#include "fle_winmm.h"

uint64_t R[16];
static uint64_t stk[32];

/* a[0..3] go to rcx, rdx, r8, r9; a[4..11] are arguments 5..12 at [rsp + 0x28 + 8 * k]. */
static uint64_t callv(void (*f)(void), const uint64_t *a)
{
    int i;
    memset(stk, 0, sizeof stk);
    stk[0] = 0xdead0000u;
    R[1] = a[0]; R[2] = a[1]; R[8] = a[2]; R[9] = a[3];
    for (i = 4; i < 12; i++) stk[i + 1] = a[i];
    R[4] = (uint64_t)(uintptr_t)stk;
    R[0] = 0xBADBADBADull;
    f();
    return R[0];
}
#define CALL(f, ...) callv((void (*)(void))(f), (uint64_t[12]){ __VA_ARGS__ })
#define A(x) ((uint64_t)(uintptr_t)(x))
#define W(s) A(u##s)
#define U(x) ((unsigned long long)(x))
static uint32_t LE(void) { fle_imp_GetLastError(); return (uint32_t)R[0]; }
static void SETLE(uint32_t v) { fle_set_last_error(v); }
static void SETW(uint16_t *d, const char *s) { size_t i = 0; do d[i] = (uint8_t)s[i]; while (s[i++]); }
static void PW(const uint16_t *s, size_t n) { size_t i; for (i = 0; i < n; i++) printf("%04x ", s[i]); printf("\n"); }
static void PRC(const int32_t *r) { printf("%d %d %d %d\n", r[0], r[1], r[2], r[3]); }
"#;

fn build_prog(name: &str, src: &str) -> PathBuf {
    let dir = work_dir();
    let uname = etxt_common::uniq(&format!("a9-{name}"));
    let c = dir.join(format!("{uname}.c"));
    let exe = dir.join(&uname);
    std::fs::write(&c, src).unwrap();
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fPIE", "-I"]).arg(src_dir()).arg("-I").arg(runtime_dir());
    for o in objs() {
        cmd.arg(o);
    }
    let cc = cmd.arg("-o").arg(&exe).arg(&c).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    exe
}

fn exec(exe: &Path, args: &[&str]) -> Out {
    let r = Command::new(exe).args(args).env("TMPDIR", work_dir().join("tmp")).output().expect("run failed");
    Out {
        stdout: String::from_utf8_lossy(&r.stdout).to_string(),
        stderr: String::from_utf8_lossy(&r.stderr).to_string(),
        signal: r.status.signal(),
        ok: r.status.success(),
    }
}

fn run(name: &str, body: &str) -> Out {
    let exe = build_prog(name, &format!("{PRELUDE}\nint main(void)\n{{\n{body}\n    return 0;\n}}\n"));
    exec(&exe, &[])
}

fn lines(name: &str, body: &str) -> Vec<String> {
    let o = run(name, body);
    assert!(o.ok, "{name} did not exit normally (signal {:?}): {}", o.signal, o.stderr);
    o.stdout.lines().map(|s| s.to_string()).collect()
}

/// One program per group; every case runs in its own process (`argv[1]` selects it) after `setup`, and must stop
/// through fle_stop: "fle:" on stderr, SIGABRT, and the line "returned" never printed.
fn assert_aborts(prefix: &str, setup: &str, cases: &[(&str, &str)]) {
    let mut sw = String::new();
    for (k, (_, stmts)) in cases.iter().enumerate() {
        sw.push_str(&format!("    case {k}: {{ {stmts} }} break;\n"));
    }
    let src = format!(
        "{PRELUDE}\nint main(int argc, char **argv)\n{{\n    int k = atoi(argv[1]);\n    {setup}\n    switch (k) {{\n{sw}    }}\n    printf(\"returned %llu\\n\", U(R[0]));\n    return 0;\n}}\n"
    );
    let exe = build_prog(&format!("{prefix}-abort"), &src);
    for (k, (label, _)) in cases.iter().enumerate() {
        let o = exec(&exe, &[&k.to_string()]);
        assert!(!o.ok, "{label}: expected an abort, got stdout {}", o.stdout);
        assert!(!o.stdout.contains("returned"), "{label}: returned instead of stopping");
        assert!(o.stderr.contains("fle:"), "{label}: no fle: message: {}", o.stderr);
        assert_eq!(o.signal, Some(6), "{label}: not SIGABRT");
    }
}

// ---------------------------------------------------------------------------------------------------------------
// user32: CharLowerW, CharUpperW, CharLowerBuffW, CharUpperBuffW, CharNextW
// ---------------------------------------------------------------------------------------------------------------

#[test]
fn char_case_string_is_converted_in_place_and_ascii_only() {
    // ASCII table: 'A'..'Z' = 0x41..0x5A, 'a'..'z' = 0x61..0x7A; '@' 0x40, '[' 0x5B, '`' 0x60, '{' 0x7B are not letters.
    // "iI" maps to "ii" and "II": the pages say i and I map the same way in every language (no Turkish rule).
    let l = lines(
        "char-str",
        r#"
    uint16_t b[32]; uint64_t r;
    SETW(b, "@[`{AZaz iI 019");
    r = CALL(fle_imp_CharLowerW, A(b));
    printf("%d\n", r == A(b)); PW(b, 15);
    SETW(b, "@[`{AZaz iI 019");
    r = CALL(fle_imp_CharUpperW, A(b));
    printf("%d\n", r == A(b)); PW(b, 15);
    SETW(b, "");
    r = CALL(fle_imp_CharUpperW, A(b));
    printf("%d %04x\n", r == A(b), b[0]);
"#,
    );
    let hex = |s: &str| s.chars().map(|c| format!("{:04x} ", c as u32)).collect::<String>();
    assert_eq!(l[0], "1");
    assert_eq!(l[1], hex("@[`{azaz ii 019"));
    assert_eq!(l[2], "1");
    assert_eq!(l[3], hex("@[`{AZAZ II 019"));
    assert_eq!(l[4], "1 0000");
}

#[test]
fn char_case_single_character_is_returned_in_the_low_word() {
    // Page: when the high-order word of the argument is zero the low word is one character; the converted character
    // comes back with a zero high-order word.
    let l = lines(
        "char-one",
        r#"
    printf("%llx\n", U(CALL(fle_imp_CharLowerW, 0x41)));
    printf("%llx\n", U(CALL(fle_imp_CharLowerW, 0x5A)));
    printf("%llx\n", U(CALL(fle_imp_CharLowerW, 0x61)));
    printf("%llx\n", U(CALL(fle_imp_CharUpperW, 0x7A)));
    printf("%llx\n", U(CALL(fle_imp_CharUpperW, 0x61)));
    printf("%llx\n", U(CALL(fle_imp_CharUpperW, 0x31)));
    printf("%llx\n", U(CALL(fle_imp_CharUpperW, 0x40)));
    printf("%llx\n", U(CALL(fle_imp_CharLowerW, 0x5B)));
    printf("%llx\n", U(CALL(fle_imp_CharUpperW, 0x69)));
    printf("%llx\n", U(CALL(fle_imp_CharLowerW, 0x49)));
"#,
    );
    assert_eq!(l, ["61", "7a", "61", "5a", "41", "31", "40", "5b", "49", "69"]);
}

#[test]
fn char_buff_converts_exactly_the_count_including_nul_and_returns_it() {
    let l = lines(
        "char-buff",
        r#"
    uint16_t b[16]; uint64_t r;
    /* ten units with an embedded NUL: "ABCabc" NUL "XYz": all ten are examined (the page) */
    SETW(b, "ABCabc"); b[6] = 0; b[7] = 'X'; b[8] = 'Y'; b[9] = 'z'; b[10] = 'Q';
    r = CALL(fle_imp_CharUpperBuffW, A(b), 10);
    printf("%llu\n", U(r)); PW(b, 11);
    SETW(b, "ABCDEF");
    r = CALL(fle_imp_CharLowerBuffW, A(b), 4);
    printf("%llu\n", U(r)); PW(b, 6);
    /* units beyond the count are not examined, so a non-ASCII unit there is fine */
    SETW(b, "AB"); b[2] = 0xE9; b[3] = 0;
    r = CALL(fle_imp_CharLowerBuffW, A(b), 2);
    printf("%llu\n", U(r)); PW(b, 3);
    /* a count of zero examines nothing */
    r = CALL(fle_imp_CharUpperBuffW, 0, 0);
    printf("%llu\n", U(r));
    /* the upper 32 bits of rdx are not part of a DWORD count */
    SETW(b, "abc");
    r = CALL(fle_imp_CharUpperBuffW, A(b), 0xFFFFFFFF00000002ull);
    printf("%llu\n", U(r)); PW(b, 3);
"#,
    );
    let hex = |units: &[u32]| units.iter().map(|c| format!("{c:04x} ")).collect::<String>();
    assert_eq!(l[0], "10");
    assert_eq!(l[1], hex(&[0x41, 0x42, 0x43, 0x41, 0x42, 0x43, 0, 0x58, 0x59, 0x5A, 0x51]));
    assert_eq!(l[2], "4");
    assert_eq!(l[3], hex(&[0x61, 0x62, 0x63, 0x64, 0x45, 0x46]));
    assert_eq!(l[4], "2");
    assert_eq!(l[5], hex(&[0x61, 0x62, 0xE9]));
    assert_eq!(l[6], "0");
    assert_eq!(l[7], "2");
    assert_eq!(l[8], hex(&[0x41, 0x42, 0x63]));
}

#[test]
fn char_case_stops_on_non_ascii_and_null() {
    assert_aborts(
        "char",
        "uint16_t b[16];",
        &[
            ("lower string with e-acute", "SETW(b, \"ab\"); b[2] = 0xE9; b[3] = 0; CALL(fle_imp_CharLowerW, A(b));"),
            ("upper string with e-acute", "SETW(b, \"ab\"); b[2] = 0xE9; b[3] = 0; CALL(fle_imp_CharUpperW, A(b));"),
            ("upper single A-grave", "CALL(fle_imp_CharUpperW, 0xC0);"),
            ("lower single 0x80", "CALL(fle_imp_CharLowerW, 0x80);"),
            ("lower single 0xFFFF", "CALL(fle_imp_CharLowerW, 0xFFFF);"),
            ("buff upper with non-ASCII inside the count", "SETW(b, \"ab\"); b[2] = 0x3B1; b[3] = 0; CALL(fle_imp_CharUpperBuffW, A(b), 3);"),
            ("buff lower NULL with count", "CALL(fle_imp_CharLowerBuffW, 0, 3);"),
        ],
    );
}

#[test]
fn char_next_steps_one_unit_over_plain_ascii_and_stays_on_nul() {
    let l = lines(
        "char-next",
        r#"
    uint16_t b[8]; uint64_t r;
    SETW(b, "abc");
    r = CALL(fle_imp_CharNextW, A(b));      printf("%llu\n", U(r - A(b)));
    r = CALL(fle_imp_CharNextW, A(b + 2));  printf("%llu\n", U(r - A(b)));   /* 'c' then NUL: points at the NUL */
    r = CALL(fle_imp_CharNextW, A(b + 3));  printf("%llu\n", U(r - A(b)));   /* NUL: returned unchanged */
    SETW(b, "\\x"); b[0] = '\\'; b[1] = 0;
    r = CALL(fle_imp_CharNextW, A(b));      printf("%llu\n", U(r - A(b)));
    SETW(b, "a b"); r = CALL(fle_imp_CharNextW, A(b)); printf("%llu\n", U(r - A(b)));
    b[0] = 0x0D; b[1] = 'x'; b[2] = 0;
    r = CALL(fle_imp_CharNextW, A(b));      printf("%llu\n", U(r - A(b)));
"#,
    );
    // Offsets are in bytes: 2 per UTF-16 unit.
    assert_eq!(l, ["2", "6", "6", "2", "2", "2"]);
}

#[test]
fn char_next_stops_where_the_page_does_not_decide() {
    assert_aborts(
        "charnext",
        "uint16_t b[8];",
        &[
            ("NULL", "CALL(fle_imp_CharNextW, 0);"),
            ("first unit non-ASCII", "b[0] = 0xE9; b[1] = 0; CALL(fle_imp_CharNextW, A(b));"),
            ("a + combining acute (the page: advances two)", "b[0] = 'a'; b[1] = 0x0301; b[2] = 0; CALL(fle_imp_CharNextW, A(b));"),
            ("a + high surrogate", "b[0] = 'a'; b[1] = 0xD83D; b[2] = 0; CALL(fle_imp_CharNextW, A(b));"),
            ("CR LF", "b[0] = 0x0D; b[1] = 0x0A; b[2] = 0; CALL(fle_imp_CharNextW, A(b));"),
        ],
    );
}

// ---------------------------------------------------------------------------------------------------------------
// user32: RECT arithmetic
// ---------------------------------------------------------------------------------------------------------------

type Rc = [i32; 4];

fn empty(r: Rc) -> bool {
    r[2] <= r[0] || r[3] <= r[1] // IsRectEmpty page
}
fn model_intersect(a: Rc, b: Rc) -> (Rc, u64) {
    let q = [a[0].max(b[0]), a[1].max(b[1]), a[2].min(b[2]), a[3].min(b[3])];
    if empty(q) {
        ([0; 4], 0) // IntersectRect page: an empty rectangle with all coordinates zero, return value zero
    } else {
        (q, 1)
    }
}
fn model_union(a: Rc, b: Rc) -> (Rc, u64) {
    // UnionRect page: the smallest rectangle containing both; an empty rectangle's dimensions are ignored. Both empty (decision 0144, S-g8a-unionrect-empty): (0, 0, 0, 0) and zero.
    if empty(a) && empty(b) {
        return ([0; 4], 0);
    }
    let q = if empty(a) {
        b
    } else if empty(b) {
        a
    } else {
        [a[0].min(b[0]), a[1].min(b[1]), a[2].max(b[2]), a[3].max(b[3])]
    };
    (q, if empty(q) { 0 } else { 1 })
}
fn rc_line(r: Rc) -> String {
    format!("{} {} {} {}", r[0], r[1], r[2], r[3])
}

#[test]
fn set_rect_and_offset_rect() {
    let l = lines(
        "rect-set",
        r#"
    int32_t r[4]; uint64_t x;
    memset(r, 0x7F, sizeof r);
    x = CALL(fle_imp_SetRect, A(r), 1, 2, 3, 4);                       printf("%llu\n", U(x)); PRC(r);
    x = CALL(fle_imp_SetRect, A(r), (uint32_t)-5, (uint32_t)-6, 7, (uint32_t)-8); printf("%llu\n", U(x)); PRC(r);
    /* the arguments are 32-bit: the upper halves of the registers and of the stack slot are not part of them */
    x = CALL(fle_imp_SetRect, A(r), 0xAAAAAAAA00000009ull, 0xBBBBBBBB0000000Aull, 0xCCCCCCCC0000000Bull, 0xDDDDDDDD0000000Cull);
    printf("%llu\n", U(x)); PRC(r);
    r[0] = 1; r[1] = 2; r[2] = 3; r[3] = 4;
    x = CALL(fle_imp_OffsetRect, A(r), 10, (uint32_t)-20);             printf("%llu\n", U(x)); PRC(r);
    r[0] = INT_MIN + 5; r[1] = 0; r[2] = INT_MAX - 3; r[3] = 0;
    x = CALL(fle_imp_OffsetRect, A(r), (uint32_t)-5, 3);               printf("%llu\n", U(x)); PRC(r);
    r[0] = 1; r[1] = 2; r[2] = 3; r[3] = 4;
    x = CALL(fle_imp_OffsetRect, A(r), 0, 0);                          printf("%llu\n", U(x)); PRC(r);
"#,
    );
    assert_eq!(l[0], "1");
    assert_eq!(l[1], "1 2 3 4");
    assert_eq!(l[2], "1");
    assert_eq!(l[3], "-5 -6 7 -8");
    assert_eq!(l[4], "1");
    assert_eq!(l[5], "9 10 11 12");
    assert_eq!(l[6], "1");
    assert_eq!(l[7], "11 -18 13 -16");
    assert_eq!(l[8], "1");
    assert_eq!(l[9], format!("{} 3 {} 3", i32::MIN, i32::MAX - 3 - 5 + 0));
    assert_eq!(l[10], "1");
    assert_eq!(l[11], "1 2 3 4");
}

#[test]
fn intersect_rect_known_cases() {
    let l = lines(
        "rect-int",
        r#"
    int32_t a[4], b[4], d[4]; uint64_t x;
    /* overlap */
    a[0]=0;a[1]=0;a[2]=10;a[3]=10; b[0]=5;b[1]=5;b[2]=15;b[3]=15; memset(d, 0x5A, sizeof d);
    x = CALL(fle_imp_IntersectRect, A(d), A(a), A(b)); printf("%llu\n", U(x)); PRC(d);
    /* edges that only touch do not intersect: all zero is stored, the result is zero (the page) */
    b[0]=10;b[1]=0;b[2]=20;b[3]=10; memset(d, 0x5A, sizeof d);
    x = CALL(fle_imp_IntersectRect, A(d), A(a), A(b)); printf("%llu\n", U(x)); PRC(d);
    /* far apart */
    b[0]=20;b[1]=20;b[2]=30;b[3]=30; memset(d, 0x5A, sizeof d);
    x = CALL(fle_imp_IntersectRect, A(d), A(a), A(b)); printf("%llu\n", U(x)); PRC(d);
    /* destination is the first source (the lifted code does this) */
    b[0]=5;b[1]=5;b[2]=15;b[3]=15;
    x = CALL(fle_imp_IntersectRect, A(a), A(a), A(b)); printf("%llu\n", U(x)); PRC(a);
    /* destination is the second source, and both sources are the same rectangle */
    a[0]=0;a[1]=0;a[2]=10;a[3]=10; b[0]=2;b[1]=3;b[2]=4;b[3]=20;
    x = CALL(fle_imp_IntersectRect, A(b), A(a), A(b)); printf("%llu\n", U(x)); PRC(b);
    x = CALL(fle_imp_IntersectRect, A(d), A(a), A(a)); printf("%llu\n", U(x)); PRC(d);
    /* an empty source (zero width) inside the other one */
    b[0]=3;b[1]=3;b[2]=3;b[3]=8;
    x = CALL(fle_imp_IntersectRect, A(d), A(a), A(b)); printf("%llu\n", U(x)); PRC(d);
"#,
    );
    let want: Vec<(Rc, u64)> = vec![
        model_intersect([0, 0, 10, 10], [5, 5, 15, 15]),
        model_intersect([0, 0, 10, 10], [10, 0, 20, 10]),
        model_intersect([0, 0, 10, 10], [20, 20, 30, 30]),
        model_intersect([0, 0, 10, 10], [5, 5, 15, 15]),
        model_intersect([0, 0, 10, 10], [2, 3, 4, 20]),
        model_intersect([0, 0, 10, 10], [0, 0, 10, 10]),
        model_intersect([0, 0, 10, 10], [3, 3, 3, 8]),
    ];
    // Hand-written anchors, so the model itself is pinned.
    assert_eq!(want[0], ([5, 5, 10, 10], 1));
    assert_eq!(want[1], ([0, 0, 0, 0], 0));
    assert_eq!(want[4], ([2, 3, 4, 10], 1));
    assert_eq!(want[5], ([0, 0, 10, 10], 1));
    assert_eq!(want[6], ([0, 0, 0, 0], 0));
    for (k, (rc, ret)) in want.iter().enumerate() {
        assert_eq!(l[2 * k], ret.to_string(), "case {k} result");
        assert_eq!(l[2 * k + 1], rc_line(*rc), "case {k} rectangle");
    }
}

#[test]
fn union_rect_known_cases() {
    let l = lines(
        "rect-union",
        r#"
    int32_t a[4], b[4], d[4]; uint64_t x;
    a[0]=0;a[1]=0;a[2]=10;a[3]=10; b[0]=5;b[1]=5;b[2]=15;b[3]=15; memset(d, 0x5A, sizeof d);
    x = CALL(fle_imp_UnionRect, A(d), A(a), A(b)); printf("%llu\n", U(x)); PRC(d);
    /* disjoint: still the bounding box */
    b[0]=20;b[1]=30;b[2]=25;b[3]=40;
    x = CALL(fle_imp_UnionRect, A(d), A(a), A(b)); printf("%llu\n", U(x)); PRC(d);
    /* one source empty (all zero): its dimensions are ignored */
    a[0]=0;a[1]=0;a[2]=0;a[3]=0; b[0]=3;b[1]=4;b[2]=5;b[3]=6;
    x = CALL(fle_imp_UnionRect, A(d), A(a), A(b)); printf("%llu\n", U(x)); PRC(d);
    /* the other source empty, and inverted (right < left) */
    a[0]=1;a[1]=1;a[2]=2;a[3]=2; b[0]=7;b[1]=7;b[2]=3;b[3]=3;
    x = CALL(fle_imp_UnionRect, A(d), A(a), A(b)); printf("%llu\n", U(x)); PRC(d);
    /* destination is the first source, as in the lifted code */
    a[0]=0;a[1]=0;a[2]=10;a[3]=10; b[0]=-5;b[1]=2;b[2]=3;b[3]=30;
    x = CALL(fle_imp_UnionRect, A(a), A(a), A(b)); printf("%llu\n", U(x)); PRC(a);
"#,
    );
    let want: Vec<(Rc, u64)> = vec![
        model_union([0, 0, 10, 10], [5, 5, 15, 15]),
        model_union([0, 0, 10, 10], [20, 30, 25, 40]),
        model_union([0, 0, 0, 0], [3, 4, 5, 6]),
        model_union([1, 1, 2, 2], [7, 7, 3, 3]),
        model_union([0, 0, 10, 10], [-5, 2, 3, 30]),
    ];
    assert_eq!(want[0], ([0, 0, 15, 15], 1));
    assert_eq!(want[1], ([0, 0, 25, 40], 1));
    assert_eq!(want[2], ([3, 4, 5, 6], 1));
    assert_eq!(want[3], ([1, 1, 2, 2], 1));
    assert_eq!(want[4], ([-5, 0, 10, 30], 1));
    for (k, (rc, ret)) in want.iter().enumerate() {
        assert_eq!(l[2 * k], ret.to_string(), "case {k} result");
        assert_eq!(l[2 * k + 1], rc_line(*rc), "case {k} rectangle");
    }
}

#[test]
fn rect_functions_match_the_model_on_a_grid_of_small_values() {
    // Deterministic pseudo-random cases (a fixed LCG) over -4..=4, so that empty, inverted, touching and nested
    // rectangles all occur. Union of two empty rectangles is excluded (the runtime stops there; tested apart).
    // splitmix64 (a fixed, well-mixed generator; a plain LCG's low bits made the grid lopsided).
    let mut s: u64 = 0x2545F4914F6CDD1D;
    let mut next = |m: u64| {
        s = s.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = s;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        (z ^ (z >> 31)) % m
    };
    // A rectangle is a corner in -4..=4 plus a width and a height in -1..=7: proper, empty (width or height 0) and inverted occur.
    let rect = |next: &mut dyn FnMut(u64) -> u64| {
        let (l, t) = (next(9) as i32 - 4, next(9) as i32 - 4);
        let (w, h) = (next(9) as i32 - 1, next(9) as i32 - 1);
        [l, t, l + w, t + h]
    };
    let mut cases: Vec<(Rc, Rc)> = Vec::new();
    while cases.len() < 4000 {
        let a = rect(&mut next);
        let b = rect(&mut next);
        cases.push((a, b));
    }
    let mut table = String::new();
    for (a, b) in &cases {
        table.push_str(&format!("    {{ {{ {}, {}, {}, {} }}, {{ {}, {}, {}, {} }} }},\n", a[0], a[1], a[2], a[3], b[0], b[1], b[2], b[3]));
    }
    let body = format!(
        r#"
    static const int32_t T[][2][4] = {{
{table}    }};
    size_t i;
    for (i = 0; i < sizeof T / sizeof T[0]; i++) {{
        int32_t a[4], b[4], d[4]; uint64_t x;
        memcpy(a, T[i][0], 16); memcpy(b, T[i][1], 16); memset(d, 0x5A, 16);
        x = CALL(fle_imp_IntersectRect, A(d), A(a), A(b));
        printf("I %llu ", U(x)); PRC(d);
        /* decision 0144 (S-g8a-unionrect-empty): two empty sources give (0, 0, 0, 0) and zero now; the case stopped before and was skipped here */
        memset(d, 0x5A, 16);
        x = CALL(fle_imp_UnionRect, A(d), A(a), A(b));
        printf("U %llu ", U(x)); PRC(d);
        /* sources are not modified */
        if (memcmp(a, T[i][0], 16) || memcmp(b, T[i][1], 16)) printf("SOURCE CHANGED\n");
    }}
"#
    );
    let l = lines("rect-grid", &body);
    assert_eq!(l.len(), 2 * cases.len());
    let mut inter_nonempty = 0;
    let mut inter_empty = 0;
    for (k, (a, b)) in cases.iter().enumerate() {
        let (ir, iret) = model_intersect(*a, *b);
        assert_eq!(l[2 * k], format!("I {iret} {}", rc_line(ir)), "intersect {a:?} {b:?}");
        if iret == 1 {
            inter_nonempty += 1
        } else {
            inter_empty += 1
        }
        let (ur, uret) = model_union(*a, *b);
        assert_eq!(l[2 * k + 1], format!("U {uret} {}", rc_line(ur)), "union {a:?} {b:?}");
    }
    // The grid must exercise both outcomes for the test to mean something.
    assert!(inter_nonempty > 200 && inter_empty > 200, "grid is lopsided: {inter_nonempty} / {inter_empty}");
}

#[test]
fn rect_functions_stop_on_null_and_on_undefined_cases() {
    assert_aborts(
        "rect",
        "int32_t a[4] = {0, 0, 10, 10}, b[4] = {5, 5, 15, 15}, e[4] = {0, 0, 0, 0}, d[4], m[4] = {INT_MAX, 0, INT_MAX, 0};",
        &[
            ("SetRect NULL", "CALL(fle_imp_SetRect, 0, 1, 2, 3, 4);"),
            ("IntersectRect dst NULL", "CALL(fle_imp_IntersectRect, 0, A(a), A(b));"),
            ("IntersectRect src1 NULL", "CALL(fle_imp_IntersectRect, A(d), 0, A(b));"),
            ("IntersectRect src2 NULL", "CALL(fle_imp_IntersectRect, A(d), A(a), 0);"),
            ("UnionRect dst NULL", "CALL(fle_imp_UnionRect, 0, A(a), A(b));"),
            ("UnionRect src1 NULL", "CALL(fle_imp_UnionRect, A(d), 0, A(b));"),
            ("UnionRect src2 NULL", "CALL(fle_imp_UnionRect, A(d), A(a), 0);"),
            ("OffsetRect NULL", "CALL(fle_imp_OffsetRect, 0, 1, 1);"),
            ("OffsetRect x overflow", "CALL(fle_imp_OffsetRect, A(m), 1, 0);"),
            ("OffsetRect y overflow", "m[1] = INT_MIN; m[3] = INT_MIN; CALL(fle_imp_OffsetRect, A(m), 0, (uint32_t)-1);"),
        ],
    );
}

// ---------------------------------------------------------------------------------------------------------------
// user32: RegisterWindowMessageW, RegisterClipboardFormatW, GetClipboardFormatNameW
// ---------------------------------------------------------------------------------------------------------------

#[test]
fn registered_values_start_at_0xc000_and_the_same_name_gives_the_same_value() {
    let l = lines(
        "reg",
        r#"
    printf("%llx\n", U(CALL(fle_imp_RegisterWindowMessageW, W("TaskbarCreated"))));
    printf("%llx\n", U(CALL(fle_imp_RegisterWindowMessageW, W("commdlg_help"))));
    printf("%llx\n", U(CALL(fle_imp_RegisterWindowMessageW, W("TaskbarCreated"))));   /* same string, same value */
    printf("%llx\n", U(CALL(fle_imp_RegisterClipboardFormatW, W("Rich Text Format"))));
    printf("%llx\n", U(CALL(fle_imp_RegisterClipboardFormatW, W("rich text format")))); /* case-insensitive (the page) */
    printf("%llx\n", U(CALL(fle_imp_RegisterClipboardFormatW, W("CSV"))));
    printf("%llx\n", U(CALL(fle_imp_RegisterWindowMessageW, W("commdlg_help"))));
"#,
    );
    // One counter hands the values out; a message and a clipboard format never share one.
    assert_eq!(l, ["c000", "c001", "c000", "c002", "c002", "c003", "c001"]);
}

#[test]
fn registered_values_stay_inside_0xc000_0xffff_and_the_range_ends_with_a_stop() {
    let body = r#"
    uint16_t n[16]; unsigned i; uint64_t v = 0, first = 0;
    for (i = 0; i < 0x4000; i++) {
        char s[16]; sprintf(s, "N%u", i); SETW(n, s);
        v = CALL(fle_imp_RegisterWindowMessageW, A(n));
        if (i == 0) first = v;
        if (v < 0xC000 || v > 0xFFFF) { printf("OUT OF RANGE %llx\n", U(v)); return 1; }
    }
    printf("%llx %llx\n", U(first), U(v));
    SETW(n, "N0"); printf("%llx\n", U(CALL(fle_imp_RegisterWindowMessageW, A(n))));
    fflush(stdout);
    SETW(n, "one-too-many"); CALL(fle_imp_RegisterWindowMessageW, A(n));
    printf("returned\n");
"#;
    let o = run("reg-range", body);
    assert!(!o.ok);
    assert_eq!(o.signal, Some(6), "stderr: {}", o.stderr);
    assert!(o.stderr.contains("fle: RegisterWindowMessageW:"), "stderr: {}", o.stderr);
    let l: Vec<&str> = o.stdout.lines().collect();
    assert_eq!(l, ["c000 ffff", "c000"], "the 16384 identifiers 0xC000..=0xFFFF are all handed out, and a known name still answers");
}

#[test]
fn clipboard_format_name_returns_the_registered_name_and_its_length() {
    let l = lines(
        "clipname",
        r#"
    uint16_t b[64]; uint64_t f, r;
    f = CALL(fle_imp_RegisterClipboardFormatW, W("Standard MIDI File"));
    memset(b, 0xEE, sizeof b);
    r = CALL(fle_imp_GetClipboardFormatNameW, f, A(b), 64);
    printf("%llu %04x %04x %04x\n", U(r), b[0], b[17], b[18]);     /* 18 characters, then the NUL */
    /* exactly enough room for the characters and the NUL */
    memset(b, 0xEE, sizeof b);
    r = CALL(fle_imp_GetClipboardFormatNameW, f, A(b), 19);
    printf("%llu %04x\n", U(r), b[18]);
    /* a predefined format (CF_TEXT = 1) and a value that was never registered: zero, with an error */
    SETLE(0);
    r = CALL(fle_imp_GetClipboardFormatNameW, 1, A(b), 64);
    printf("%llu %u\n", U(r), LE());
    SETLE(0);
    r = CALL(fle_imp_GetClipboardFormatNameW, 0xC0FF, A(b), 64);
    printf("%llu %u\n", U(r), LE());
    SETLE(0);
    r = CALL(fle_imp_GetClipboardFormatNameW, 0, A(b), 64);
    printf("%llu %u\n", U(r), LE());
    /* the upper half of rcx is not part of a UINT */
    memset(b, 0xEE, sizeof b);
    r = CALL(fle_imp_GetClipboardFormatNameW, 0x1234567800000000ull | f, A(b), 64);
    printf("%llu\n", U(r));
"#,
    );
    // "Standard MIDI File": S=0x53, ..., e=0x65 (last character); 'S' at index 0.
    assert_eq!(l[0], "18 0053 0065 0000");
    assert_eq!(l[1], "18 0000");
    assert_eq!(l[2], "0 87");
    assert_eq!(l[3], "0 87");
    assert_eq!(l[4], "0 87");
    assert_eq!(l[5], "18");
}

#[test]
fn register_functions_stop_where_the_pages_are_silent() {
    assert_aborts(
        "regabort",
        "uint16_t b[300]; uint64_t m, c; m = CALL(fle_imp_RegisterWindowMessageW, W(\"Msg\")); c = CALL(fle_imp_RegisterClipboardFormatW, W(\"Clip\"));",
        &[
            ("RegisterWindowMessageW NULL", "CALL(fle_imp_RegisterWindowMessageW, 0);"),
            ("RegisterClipboardFormatW NULL", "CALL(fle_imp_RegisterClipboardFormatW, 0);"),
            ("RegisterWindowMessageW empty", "CALL(fle_imp_RegisterWindowMessageW, W(\"\"));"),
            ("RegisterClipboardFormatW empty", "CALL(fle_imp_RegisterClipboardFormatW, W(\"\"));"),
            ("RegisterWindowMessageW non-ASCII", "SETW(b, \"a\"); b[1] = 0xE9; b[2] = 0; CALL(fle_imp_RegisterWindowMessageW, A(b));"),
            ("RegisterClipboardFormatW non-ASCII", "SETW(b, \"a\"); b[1] = 0xE9; b[2] = 0; CALL(fle_imp_RegisterClipboardFormatW, A(b));"),
            ("RegisterWindowMessageW name longer than the limit", "{ unsigned i; for (i = 0; i < 300; i++) b[i] = 'x'; b[299] = 0; CALL(fle_imp_RegisterWindowMessageW, A(b)); }"),
            ("RegisterWindowMessageW same name, different case", "CALL(fle_imp_RegisterWindowMessageW, W(\"MSG\"));"),
            ("GetClipboardFormatNameW buffer NULL", "CALL(fle_imp_GetClipboardFormatNameW, c, 0, 64);"),
            ("GetClipboardFormatNameW cchMaxCount zero", "CALL(fle_imp_GetClipboardFormatNameW, c, A(b), 0);"),
            ("GetClipboardFormatNameW cchMaxCount negative", "CALL(fle_imp_GetClipboardFormatNameW, c, A(b), (uint32_t)-1);"),
            ("GetClipboardFormatNameW buffer too small (truncation is not defined by an answer here)", "CALL(fle_imp_GetClipboardFormatNameW, c, A(b), 2);"),
            ("GetClipboardFormatNameW buffer holds the characters but not the NUL", "CALL(fle_imp_GetClipboardFormatNameW, c, A(b), 4);"),
            ("GetClipboardFormatNameW on a registered window message", "CALL(fle_imp_GetClipboardFormatNameW, m, A(b), 64);"),
        ],
    );
}

// ---------------------------------------------------------------------------------------------------------------
// winmm
// ---------------------------------------------------------------------------------------------------------------

#[test]
fn midi_has_no_devices() {
    let l = lines(
        "midi-count",
        r#"
    printf("%llu\n", U(CALL(fle_imp_midiInGetNumDevs)));
    printf("%llu\n", U(CALL(fle_imp_midiOutGetNumDevs)));
"#,
    );
    // Pages: "A return value of zero means that there are no devices (not that there is no error)."
    assert_eq!(l, ["0", "0"]);
}

#[test]
fn midi_dev_caps_report_a_bad_device_id_for_every_identifier_and_do_not_touch_the_buffer() {
    let l = lines(
        "midi-caps",
        r#"
    uint8_t b[0x54]; unsigned i; uint64_t ids[] = { 0, 1, 7, 0x7FFFFFFF, 0x80000000u };
    for (i = 0; i < sizeof ids / sizeof ids[0]; i++) {
        uint64_t r1, r2;
        memset(b, 0xA5, sizeof b);
        r1 = CALL(fle_imp_midiInGetDevCapsW, ids[i], A(b), 0x2C);
        r2 = CALL(fle_imp_midiOutGetDevCapsW, ids[i], A(b), 0x54);
        printf("%llu %llu %02x %02x\n", U(r1), U(r2), b[0], b[0x53]);
    }
    /* (UINT)-1 is not an input identifier; as a 32-bit value and as a sign-extended 64-bit value */
    printf("%llu\n", U(CALL(fle_imp_midiInGetDevCapsW, 0xFFFFFFFFu, A(b), 0x2C)));
    printf("%llu\n", U(CALL(fle_imp_midiInGetDevCapsW, 0xFFFFFFFFFFFFFFFFull, A(b), 0x2C)));
"#,
    );
    for line in &l[0..5] {
        assert_eq!(*line, format!("{MMSYSERR_BADDEVICEID} {MMSYSERR_BADDEVICEID} a5 a5"));
    }
    assert_eq!(l[5], MMSYSERR_BADDEVICEID.to_string());
    assert_eq!(l[6], MMSYSERR_BADDEVICEID.to_string());
}

#[test]
fn midi_dev_caps_and_error_text_stop_where_the_pages_are_silent() {
    assert_aborts(
        "midicaps",
        "uint8_t b[0x54]; uint16_t t[256];",
        &[
            ("midiInGetDevCapsW pointer NULL", "CALL(fle_imp_midiInGetDevCapsW, 0, 0, 0x2C);"),
            ("midiOutGetDevCapsW pointer NULL", "CALL(fle_imp_midiOutGetDevCapsW, 0, 0, 0x54);"),
            ("midiInGetDevCapsW size zero", "CALL(fle_imp_midiInGetDevCapsW, 0, A(b), 0);"),
            ("midiOutGetDevCapsW size zero", "CALL(fle_imp_midiOutGetDevCapsW, 0, A(b), 0);"),
            // decision 0126 (wall 78, S-midi-mapper-nodriver): old: midiOutGetDevCapsW(MIDI_MAPPER) stopped here (both the 32-bit and the sign-extended form); new: it returns MMSYSERR_NODRIVER (6), see
            // midi_out_mapper_is_nodriver_and_the_structure_is_not_written below. midiInGetDevCapsW with (UINT)-1 is not the mapper (an input has none) and still gives BADDEVICEID (the test above).
            ("midiInGetDevCapsW identifier above 32 bits (a handle?)", "CALL(fle_imp_midiInGetDevCapsW, 0x100000000ull, A(b), 0x2C);"),
            ("midiOutGetErrorTextW with a buffer", "CALL(fle_imp_midiOutGetErrorTextW, 2, A(t), 256);"),
            ("midiOutGetErrorTextW BADERRNUM range", "CALL(fle_imp_midiOutGetErrorTextW, 9999, A(t), 256);"),
            ("midiOutGetErrorTextW NULL buffer, nonzero length", "CALL(fle_imp_midiOutGetErrorTextW, 2, 0, 256);"),
        ],
    );
}

#[test]
fn midi_out_mapper_is_nodriver_and_the_structure_is_not_written() {
    // decision 0126, wall 78 (S-midi-mapper-nodriver, a choice without a source): the guest asks midiOutGetDevCapsW(MIDI_MAPPER) on a machine without a MIDI output device; the page lists MMSYSERR_NODRIVER
    // ("The driver is not installed.") and says "The MIDI_MAPPER constant is also a valid device identifier"; the number 6 is from the SDK headers (unofficial). The mapper is not a number in the device range, so
    // BADDEVICEID (2) is not the answer for it. Both the 32-bit form 0xFFFFFFFF and the sign-extended form (movsxd in the lifted code) are the mapper; the input function has no mapper.
    let l = lines(
        "midi-mapper",
        r#"
    uint8_t b[0x54];
    memset(b, 0xA5, sizeof b);
    printf("%llu %02x %02x\n", U(CALL(fle_imp_midiOutGetDevCapsW, 0xFFFFFFFFu, A(b), 0x54)), b[0], b[0x53]);
    printf("%llu %02x %02x\n", U(CALL(fle_imp_midiOutGetDevCapsW, 0xFFFFFFFFFFFFFFFFull, A(b), 0x54)), b[0], b[0x53]);
    printf("%llu\n", U(CALL(fle_imp_midiInGetDevCapsW, 0xFFFFFFFFu, A(b), 0x2C)));
"#,
    );
    assert_eq!(l[0], "6 a5 a5");
    assert_eq!(l[1], "6 a5 a5");
    assert_eq!(l[2], MMSYSERR_BADDEVICEID.to_string());
}

#[test]
fn midi_error_text_with_zero_length_copies_nothing() {
    let l = lines(
        "midi-text0",
        r#"
    uint16_t t[4] = { 0xEEEE, 0xEEEE, 0xEEEE, 0xEEEE };
    printf("%llu %04x\n", U(CALL(fle_imp_midiOutGetErrorTextW, 2, A(t), 0)), t[0]);
    printf("%llu\n", U(CALL(fle_imp_midiOutGetErrorTextW, 9999, 0, 0)));
"#,
    );
    assert_eq!(l[0], format!("{MMSYSERR_NOERROR} eeee"));
    assert_eq!(l[1], MMSYSERR_NOERROR.to_string());
}

#[test]
fn midi_out_open_fails_without_a_device_and_leaves_the_handle_alone() {
    let l = lines(
        "midi-open",
        r#"
    uint64_t h = 0x1234, r;
    /* CALLBACK_FUNCTION = 0x30000 as the lifted code passes it */
    r = CALL(fle_imp_midiOutOpen, A(&h), 0, 0x400000, 0, 0x30000);  printf("%llu %llx\n", U(r), U(h));
    r = CALL(fle_imp_midiOutOpen, A(&h), 3, 0, 0, 0);               printf("%llu %llx\n", U(r), U(h));
    r = CALL(fle_imp_midiOutOpen, A(&h), 0xFFFFFFFFu, 0, 0, 0);     printf("%llu %llx\n", U(r), U(h));
    r = CALL(fle_imp_midiOutOpen, A(&h), 0xFFFFFFFFu, 0, 0, 0x10000); printf("%llu\n", U(r));
    r = CALL(fle_imp_midiOutOpen, A(&h), 0xFFFFFFFFu, 0, 0, 0x20000); printf("%llu\n", U(r));
    r = CALL(fle_imp_midiOutOpen, A(&h), 0xFFFFFFFFu, 0, 0, 0x30000); printf("%llu\n", U(r));
    r = CALL(fle_imp_midiOutOpen, A(&h), 0xFFFFFFFFu, 0, 0, 0x50000); printf("%llu\n", U(r));
    /* only the low 32 bits of rdx are the identifier */
    r = CALL(fle_imp_midiOutOpen, A(&h), 0xABCD0000FFFFFFFFull, 0, 0, 0); printf("%llu\n", U(r));
"#,
    );
    assert_eq!(l[0], format!("{MMSYSERR_BADDEVICEID} 1234"));
    assert_eq!(l[1], format!("{MMSYSERR_BADDEVICEID} 1234"));
    assert_eq!(l[2], format!("{MIDIERR_NODEVICE} 1234"));
    for line in &l[3..8] {
        assert_eq!(*line, MIDIERR_NODEVICE.to_string());
    }
}

#[test]
fn midi_out_open_and_short_msg_stop_or_fail_as_the_pages_say() {
    assert_aborts(
        "midiopen",
        "uint64_t h = 0;",
        &[
            ("phmo NULL", "CALL(fle_imp_midiOutOpen, 0, 0, 0, 0, 0);"),
            ("flag outside the callback types", "CALL(fle_imp_midiOutOpen, A(&h), 0, 0, 0, 0x1);"),
            ("callback type 0x40000 is not on the page", "CALL(fle_imp_midiOutOpen, A(&h), 0, 0, 0, 0x40000);"),
            ("callback type 0x70000", "CALL(fle_imp_midiOutOpen, A(&h), 0, 0, 0, 0x70000);"),
        ],
    );
    let l = lines(
        "midi-short",
        r#"
    printf("%llu\n", U(CALL(fle_imp_midiOutShortMsg, 0, 0x007F3C90)));
    printf("%llu\n", U(CALL(fle_imp_midiOutShortMsg, 0x1234, 0)));
    printf("%llu\n", U(CALL(fle_imp_midiOutShortMsg, 0xFFFFFFFFFFFFFFFFull, 0x90)));
"#,
    );
    // No handle was ever handed out, so every handle is invalid (page: MMSYSERR_INVALHANDLE).
    assert_eq!(l, [MMSYSERR_INVALHANDLE.to_string(), MMSYSERR_INVALHANDLE.to_string(), MMSYSERR_INVALHANDLE.to_string()]);
}

#[test]
fn time_period_calls_count_matched_pairs_and_range_check() {
    let l = lines(
        "time-period",
        r#"
    printf("%u %u\n", fle_winmm_period_outstanding(), fle_winmm_period_lowest());
    printf("%llu\n", U(CALL(fle_imp_timeBeginPeriod, 5)));
    printf("%llu\n", U(CALL(fle_imp_timeBeginPeriod, 5)));
    printf("%llu\n", U(CALL(fle_imp_timeBeginPeriod, 2)));
    printf("%u %u\n", fle_winmm_period_outstanding(), fle_winmm_period_lowest());
    printf("%llu\n", U(CALL(fle_imp_timeEndPeriod, 2)));
    printf("%u %u\n", fle_winmm_period_outstanding(), fle_winmm_period_lowest());
    printf("%llu\n", U(CALL(fle_imp_timeEndPeriod, 5)));
    printf("%u %u\n", fle_winmm_period_outstanding(), fle_winmm_period_lowest());
    printf("%llu\n", U(CALL(fle_imp_timeEndPeriod, 5)));
    printf("%u %u\n", fle_winmm_period_outstanding(), fle_winmm_period_lowest());
    /* out of range: TIMERR_NOCANDO and nothing recorded (the page: "if the resolution specified in uPeriod is out of range") */
    printf("%llu\n", U(CALL(fle_imp_timeBeginPeriod, 0)));
    printf("%llu\n", U(CALL(fle_imp_timeBeginPeriod, 1000001)));
    printf("%llu\n", U(CALL(fle_imp_timeBeginPeriod, 0xFFFFFFFFu)));
    printf("%llu\n", U(CALL(fle_imp_timeEndPeriod, 0)));
    printf("%llu\n", U(CALL(fle_imp_timeEndPeriod, 1000001)));
    printf("%u\n", fle_winmm_period_outstanding());
    /* the edges of the range */
    printf("%llu %llu\n", U(CALL(fle_imp_timeBeginPeriod, 1)), U(CALL(fle_imp_timeBeginPeriod, 1000000)));
    printf("%u %u\n", fle_winmm_period_outstanding(), fle_winmm_period_lowest());
    printf("%llu %llu\n", U(CALL(fle_imp_timeEndPeriod, 1000000)), U(CALL(fle_imp_timeEndPeriod, 1)));
    printf("%u\n", fle_winmm_period_outstanding());
    /* only the low 32 bits of rcx are the period */
    printf("%llu\n", U(CALL(fle_imp_timeBeginPeriod, 0xAAAAAAAA00000007ull)));
    printf("%u\n", fle_winmm_period_lowest());
    printf("%llu\n", U(CALL(fle_imp_timeEndPeriod, 7)));
"#,
    );
    assert_eq!(
        l,
        [
            "0 0",
            "0",
            "0",
            "0",
            "3 2",
            "0",
            "2 5",
            "0",
            "1 5",
            "0",
            "0 0",
            "97",
            "97",
            "97",
            "97",
            "97",
            "0",
            "0 0",
            "2 1",
            "0 0",
            "0",
            "0",
            "7",
            "0"
        ]
    );
}

#[test]
fn time_period_end_without_begin_and_too_many_periods_stop() {
    assert_aborts(
        "period",
        "",
        &[
            ("timeEndPeriod without a matching begin", "CALL(fle_imp_timeEndPeriod, 5);"),
            ("timeEndPeriod of a period that was begun and already ended", "CALL(fle_imp_timeBeginPeriod, 5); CALL(fle_imp_timeEndPeriod, 5); CALL(fle_imp_timeEndPeriod, 5);"),
            ("more different periods than slots", "{ unsigned i; for (i = 1; i <= FLE_WINMM_PERIOD_SLOTS + 1; i++) CALL(fle_imp_timeBeginPeriod, i); }"),
            ("timeGetDevCaps with a size that is not TIMECAPS", "uint32_t c[2]; CALL(fle_imp_timeGetDevCaps, A(c), 4);"),
            ("timeGetDevCaps with size zero", "uint32_t c[2]; CALL(fle_imp_timeGetDevCaps, A(c), 0);"),
            ("timeGetDevCaps with a larger size", "uint32_t c[4]; CALL(fle_imp_timeGetDevCaps, A(c), 16);"),
        ],
    );
}

#[test]
fn time_get_dev_caps_fills_timecaps_and_fails_for_null() {
    let l = lines(
        "time-caps",
        r#"
    uint32_t c[3] = { 0xEEEEEEEE, 0xEEEEEEEE, 0xEEEEEEEE };
    printf("%llu %u %u %08x\n", U(CALL(fle_imp_timeGetDevCaps, A(c), 8)), c[0], c[1], c[2]);
    printf("%llu\n", U(CALL(fle_imp_timeGetDevCaps, 0, 8)));
    /* the reported range is the range that timeBeginPeriod accepts */
    printf("%llu %llu\n", U(CALL(fle_imp_timeBeginPeriod, c[0])), U(CALL(fle_imp_timeBeginPeriod, c[1])));
    printf("%llu %llu\n", U(CALL(fle_imp_timeBeginPeriod, c[0] - 1)), U(CALL(fle_imp_timeBeginPeriod, c[1] + 1)));
"#,
    );
    // TIMECAPS is two UINT (wPeriodMin, wPeriodMax), 8 bytes; the values are settings of this environment (decision 0030).
    assert_eq!(l[0], format!("{TIMERR_NOERROR} 1 1000000 eeeeeeee"));
    assert_eq!(l[1], TIMERR_NOCANDO.to_string());
    assert_eq!(l[2], format!("{TIMERR_NOERROR} {TIMERR_NOERROR}"));
    assert_eq!(l[3], format!("{TIMERR_NOCANDO} {TIMERR_NOCANDO}"));
}

#[test]
fn timer_resolution_calls_do_not_move_the_virtual_clock_that_time_get_time_reads() {
    let l = lines(
        "time-clock",
        r#"
    uint32_t c[2]; uint64_t t0, t1, t2, t3;
    t0 = fle_sys_clock_ticks();
    CALL(fle_imp_timeBeginPeriod, 1);
    CALL(fle_imp_timeGetDevCaps, A(c), 8);
    CALL(fle_imp_timeEndPeriod, 1);
    CALL(fle_imp_midiOutGetNumDevs);
    CALL(fle_imp_midiInGetNumDevs);
    CALL(fle_imp_midiOutShortMsg, 0, 0);
    t1 = fle_sys_clock_ticks();
    printf("%d\n", t0 == t1);
    /* timeGetTime is the clock of fle_sys.c in milliseconds (decision 0017); Sleep adds exactly the interval */
    {
        uint64_t a = CALL(fle_imp_timeGetTime);
        uint64_t before = fle_sys_clock_ticks();
        CALL(fle_imp_Sleep, 250);
        t2 = fle_sys_clock_ticks();
        printf("%d\n", t2 - before == 250 * FLE_TICKS_PER_MS);
        t3 = CALL(fle_imp_timeGetTime);
        printf("%llu\n", U(t3 - a));
    }
    CALL(fle_imp_timeBeginPeriod, 1);
    {
        uint64_t u = fle_sys_clock_ticks(); CALL(fle_imp_timeEndPeriod, 1); printf("%d\n", u == fle_sys_clock_ticks());
    }
"#,
    );
    assert_eq!(l[0], "1", "timer-resolution and MIDI calls moved the clock");
    assert_eq!(l[1], "1");
    assert_eq!(l[2], "250", "timeGetTime advanced by the Sleep interval (a read itself adds only 10 ticks = 0 ms)");
    assert_eq!(l[3], "1");
}

// ---------------------------------------------------------------------------------------------------------------
// Generator: the thunks reach the runtime, and every named function is declared and defined
// ---------------------------------------------------------------------------------------------------------------

const USER32: &[&str] = &[
    "CharLowerW",
    "CharLowerBuffW",
    "CharUpperW",
    "CharUpperBuffW",
    "CharNextW",
    "SetRect",
    "IntersectRect",
    "OffsetRect",
    "UnionRect",
    "RegisterWindowMessageW",
    "RegisterClipboardFormatW",
    "GetClipboardFormatNameW",
];
const WINMM: &[&str] = &[
    "midiInGetNumDevs",
    "midiOutGetNumDevs",
    "midiInGetDevCapsW",
    "midiOutGetDevCapsW",
    "midiOutGetErrorTextW",
    "midiOutOpen",
    "midiOutShortMsg",
    "timeBeginPeriod",
    "timeEndPeriod",
    "timeGetDevCaps",
];

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

/// A PE importing `user32.dll` (USER32) and `winmm.dll` (WINMM); the entry at 0x1000 calls one thunk per import,
/// each thunk is `jmp [IAT slot]` at 0x1100 + 0x10 * k (the calls take 5 * 22 bytes from 0x1000, so the thunks start later).
fn build_pe() -> Vec<u8> {
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
    w32(&mut b, o + 124, 60);

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

    // Descriptors at 0x2000 (user32: OFT 0x2100, IAT 0x2200; winmm: OFT 0x2300, IAT 0x2400); DLL names at 0x2060,
    // 0x2070; hint/name entries at 0x2480 + 0x20 * k.
    let d = data_at(DATA_RVA);
    w32(&mut b, d, 0x2100);
    w32(&mut b, d + 12, 0x2060);
    w32(&mut b, d + 16, 0x2200);
    w32(&mut b, d + 20, 0x2300);
    w32(&mut b, d + 32, 0x2070);
    w32(&mut b, d + 36, 0x2400);
    put(&mut b, 0x2060, b"user32.dll\0");
    put(&mut b, 0x2070, b"winmm.dll\0");
    let mut slots: Vec<u32> = Vec::new();
    for (k, name) in USER32.iter().chain(WINMM.iter()).enumerate() {
        let (oft, iat) = if k < USER32.len() { (0x2100 + 8 * k as u32, 0x2200 + 8 * k as u32) } else { (0x2300 + 8 * (k - USER32.len()) as u32, 0x2400 + 8 * (k - USER32.len()) as u32) };
        let name_rva = 0x2480 + 0x20 * k as u32;
        let mut hn = vec![0u8, 0u8];
        hn.extend_from_slice(name.as_bytes());
        hn.push(0);
        put(&mut b, name_rva, &hn);
        w64(&mut b, data_at(oft), name_rva as u64);
        w64(&mut b, data_at(iat), name_rva as u64);
        slots.push(iat);
    }
    let t = TEXT_RAW;
    let mut code: Vec<(u32, Vec<u8>)> = Vec::new();
    for (k, iat) in slots.iter().enumerate() {
        let at = 0x1000 + 5 * k as u32;
        let th = 0x1100 + 0x10 * k as u32;
        code.push((at, [&[0xe8][..], &rel(at + 5, th)].concat()));
        code.push((th, [&[0xff, 0x25][..], &rel(th + 6, *iat)].concat()));
    }
    code.push((0x1000 + 5 * slots.len() as u32, vec![0xc3]));
    for (rva, bytes) in code {
        let o = t + (rva - TEXT_RVA) as usize;
        b[o..o + bytes.len()].copy_from_slice(&bytes);
    }
    b
}

#[test]
fn every_thunk_of_the_two_groups_calls_its_runtime_function() {
    let img = build_pe();
    let mut out = Vec::new();
    let st = generate(&img, &mut out).unwrap();
    let c = String::from_utf8(out).unwrap();
    for name in USER32.iter().chain(WINMM.iter()) {
        let key = format!("jmp_imp:{name}");
        assert_eq!(st.handled.get(key.as_str()).copied(), Some(1), "{name} is not answered through the table");
        assert!(c.contains(&format!("fle_imp_{name}(); return;")), "{name}: no call to the runtime in the generated C");
    }
    assert!(!st.unsupported.keys().any(|k| k.contains("IAT slot")), "some thunk stayed unsupported: {:?}", st.unsupported);
}

#[test]
fn every_function_of_the_table_is_declared_and_defined_in_the_runtime() {
    // A function named in the table but missing (or misspelled) in the runtime would fail at link time of every
    // generated program; this test takes the address of all of them.
    let mut decl = String::new();
    let mut tab = String::new();
    for name in USER32.iter().chain(WINMM.iter()) {
        decl.push_str(&format!("void fle_imp_{name}(void);\n"));
        tab.push_str(&format!("    (void (*)(void))fle_imp_{name},\n"));
    }
    let src = format!("#include <stdint.h>\n#include <stdio.h>\nuint64_t R[16];\n{decl}\nstatic void (*const T[])(void) = {{\n{tab}}};\nint main(void) {{ unsigned i, n = 0; for (i = 0; i < sizeof T / sizeof T[0]; i++) if (T[i]) n++; printf(\"%u %u\\n\", n, (unsigned)(sizeof T / sizeof T[0])); return 0; }}\n");
    let exe = build_prog("linked", &src);
    let o = exec(&exe, &[]);
    assert!(o.ok, "{}", o.stderr);
    let n: Vec<&str> = o.stdout.split_whitespace().collect();
    assert_eq!(n[0], n[1]);
    assert_eq!(n[1], (USER32.len() + WINMM.len()).to_string());
}
