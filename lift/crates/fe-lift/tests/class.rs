//! Tests for the virtual window class table of runtime/fle_class.c: user32!RegisterClassW and user32!GetClassInfoW
//! (docs/decisions/0070-virtual-window-classes.md, provisional; sources: docs/investigations/getclassinfo-caller.md).
//!
//! Expected values are written here from the decision (the rules on the Microsoft Learn pages RegisterClassW, GetClassInfoW,
//! WNDCLASSW, About Window Classes, About Atom Tables, and the choices of the decision), not read back from the C code. A number
//! the pages do not give (the first atom 0xC000, the rule "names are compared unit for unit") is a choice of the decision and is
//! the only thing such a test pins down. The host is Linux: nothing is compared with a real Windows; no real class or window.
//!
//! The C sources of fle_class.c / fle_class.h come from runtime/. FLE_CLASS_SRC_DIR (a directory with fle_class.c and fle_class.h)
//! overrides that: the mutation runs of the run record use it. Files that already existed always come from runtime/.

mod etxt_common;

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
    if let Ok(d) = std::env::var("FLE_CLASS_SRC_DIR") {
        return PathBuf::from(d);
    }
    runtime_dir()
}
fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-cl/test");
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

/// fle_class.c (warnings are errors) and the files it links with: fle_imports.c (GetLastError / fle_set_last_error) and fle_env.c
/// (fle_stop, fle_w_icmp).
fn objs() -> &'static Vec<PathBuf> {
    static O: OnceLock<Vec<PathBuf>> = OnceLock::new();
    O.get_or_init(|| {
        let dir = work_dir().join(etxt_common::per_process("obj-class"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut v = Vec::new();
        for (name, ours) in [("fle_class.c", true), ("fle_imports.c", false), ("fle_env.c", false)] {
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
#include "fle_modules.h"
#include "fle_class.h"

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
#define A(x) ((uint64_t)(uintptr_t)(x))
static uint32_t LE(void) { fle_imp_GetLastError(); return (uint32_t)R[0]; }
static void SETLE(uint32_t v) { fle_set_last_error(v); }

/* a WNDCLASSW with a different non-zero value in every field, so that a shifted or short copy shows: style 3, proc, cbClsExtra 2,
   cbWndExtra 4, hInstance 0x400000, hIcon, hCursor, hbrBackground, menu name pointer, class name pointer */
static uint8_t wcb[72];
static uint16_t nmb[400];
static void p32(unsigned o, uint32_t v) { memcpy(wcb + o, &v, 4); }
static void p64(unsigned o, uint64_t v) { memcpy(wcb + o, &v, 8); }
static void mkwc(const char *name)
{
    size_t n = strlen(name), i;
    memset(wcb, 0, sizeof wcb);
    for (i = 0; i < n; i++) nmb[i] = (uint8_t)name[i];
    nmb[n] = 0;
    p32(0, 3); p64(8, 0x0000000000521000ull); p32(16, 2); p32(20, 4); p64(24, 0x400000);
    p64(32, 0x1111); p64(40, 0x2222); p64(48, 0x3333); p64(56, 0x4444); p64(64, A(nmb));
}
#define WCA A(wcb)
#define REG(name) (mkwc(name), CALL(fle_imp_RegisterClassW, WCA))
static uint8_t out[72];
static uint64_t getinfo(const char *name, uint64_t hinst)
{
    size_t n = strlen(name), i;
    static uint16_t q[400];
    for (i = 0; i < n; i++) q[i] = (uint8_t)name[i];
    q[n] = 0;
    return CALL(fle_imp_GetClassInfoW, hinst, A(q), A(out));
}
#define GETI(name) getinfo(name, 0x400000)
static void fill_out(uint8_t v) { memset(out, v, sizeof out); }
static int out_is(uint8_t v) { for (unsigned i = 0; i < sizeof out; i++) if (out[i] != v) return 0; return 1; }
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

/// Each (label, C statements) must stop the run with SIGABRT, a "fle: <api>" message that contains `word`, and no output after it.
/// `setup` runs first.
fn assert_aborts(prefix: &str, setup: &str, api: &str, word: &str, cases: &[(&str, &str)]) {
    for (label, stmts) in cases {
        let body = format!("{setup}\n    {stmts}\n    printf(\"returned %llu\\n\", U(R[0]));");
        let o = run(&format!("{prefix}-{label}"), &body);
        assert!(!o.ok, "{label}: expected an abort, got stdout {}", o.stdout);
        assert!(!o.stdout.contains("returned"), "{label}: returned instead of stopping");
        assert!(o.stderr.contains(&format!("fle: {api}: ")), "{label}: no 'fle: {api}' message: {}", o.stderr);
        assert!(o.stderr.contains(word), "{label}: message lacks {word:?}: {}", o.stderr);
        assert_eq!(o.signal, Some(6), "{label}: not SIGABRT");
    }
}

/// The value of `#define NAME <number>` (decimal, 0x.. hex, with an optional trailing `u`) in a runtime header.
fn def_const(file: &str, name: &str) -> u64 {
    let t = std::fs::read_to_string(src_dir().join(file)).or_else(|_| std::fs::read_to_string(runtime_dir().join(file))).unwrap();
    let line = t.lines().find(|l| l.starts_with(&format!("#define {name} "))).unwrap_or_else(|| panic!("{name} not in {file}"));
    let rest = line[format!("#define {name} ").len()..].trim_start();
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

#[test]
fn the_two_rows_are_in_the_table_and_declared_and_the_neighbours_are_not_answered() {
    let table = std::fs::read_to_string(manifest().join("src/imports_table.rs")).unwrap();
    let h = std::fs::read_to_string(src_dir().join("fle_class.h")).unwrap();
    for n in ["RegisterClassW", "GetClassInfoW"] {
        let row = format!("(\"user32.dll\", \"{n}\", \"fle_imp_{n}\", \"jmp_imp:{n}\")");
        assert_eq!(table.matches(&row).count(), 1, "{n}: exactly one row in imports_table.rs");
        assert!(h.contains(&format!("void fle_imp_{n}(void)")), "{n} is declared in fle_class.h");
    }
    // decision 0070 adds these two and no more: the other class APIs are not answered. CreateWindowExW and DefWindowProcW were in this
    // list until decision 0071 (runtime/fle_win.c answers them, see tests/window.rs): they were the next stop of the harness, and 0070
    // had said that CreateWindowExW is not answered "yet"
    // decision 0123 (old: "UnregisterClassW" and "GetClassNameW" were in this list): runtime/fle_class.c answers UnregisterClassW and runtime/fle_win.c GetClassNameW (tests/wall_fr9.rs).
    for n in [
        "RegisterClassExW", "GetClassInfoExW", "RegisterClassA", "RegisterClassExA", "GetClassInfoA",
        "GetClassInfoExA", "UnregisterClassA", "GetClassLongPtrW",
        // "SetClassLongPtrW" was in this list until decision 0119 (wall 9/15: runtime/fle_win.c answers it, tests/wall_fr6.rs); old: asserted absent
    ] {
        assert!(!table.contains(&format!("\"{n}\"")), "{n} must not be answered by this decision");
    }
}

#[test]
fn the_decided_constants() {
    // Learn WNDCLASSW on x64: UINT, 8 pointers and 2 ints in the order of the page, 8-byte aligned = 72
    assert_eq!(4 + 4 /* padding */ + 8 + 4 + 4 + 8 + 8 + 8 + 8 + 8 + 8, 72);
    assert_eq!(def_const("fle_class.h", "FLE_CLASS_WNDCLASSW_SIZE"), 72);
    // Learn WNDCLASSW: the maximum length of lpszClassName is 256; About Window Classes: extra memory at most 40 bytes
    assert_eq!(def_const("fle_class.h", "FLE_CLASS_NAME_MAX_UNITS"), 256);
    assert_eq!(def_const("fle_class.h", "FLE_CLASS_EXTRA_MAX"), 40);
    // Learn About Atom Tables: the values of string atoms are 0xC000 (MAXINTATOM) through 0xFFFF; the first atom is a choice
    assert_eq!(def_const("fle_class.h", "FLE_CLASS_ATOM_FIRST"), 0xC000);
    assert_eq!(def_const("fle_class.h", "FLE_CLASS_ATOM_LAST"), 0xFFFF);
}

// ---- 2. RegisterClassW ----

#[test]
fn register_returns_the_next_atom_counts_the_class_and_leaves_everything_else_alone() {
    let l = lines(
        "class-reg",
        r#"
    uint64_t a, b, c;
    SETLE(0x1234);
    printf("count0 %llu\n", U(fle_class_count()));
    a = REG("TPUtilWindow");
    printf("regs %d %d %d %d %d\n", R[1] == WCA, R[2] == 0, R[8] == 0, R[9] == 0, R[4] == A(stk));
    printf("return address %llx\n", U(stk[0]));
    b = REG("TTimerWindow");
    c = REG("TApplication");
    printf("atoms %llx %llx %llx\n", U(a), U(b), U(c));
    printf("count %llu\n", U(fle_class_count()));
    printf("last error %x\n", LE());
    printf("buffer kept %d\n", wcb[0] == 3 && wcb[8] == 0x00 && wcb[9] == 0x10);
"#,
    );
    assert_eq!(
        l,
        ["count0 0", "regs 1 1 1 1 1", "return address dead0000", "atoms c000 c001 c002", "count 3", "last error 1234", "buffer kept 1"]
    );
}

#[test]
fn the_window_procedure_is_kept_as_an_address_and_never_called() {
    // 0x521000 is not code: a call would crash; and the class buffer of the caller may change after the call (a copy is kept)
    let l = lines(
        "class-reg-copy",
        r#"
    uint64_t a;
    int i, same = 1;
    uint8_t first[72];
    a = REG("TApplication");
    memcpy(first, wcb, 72);
    memset(wcb, 0xEE, 72);
    memset(nmb, 0xEE, sizeof nmb);
    fill_out(0xAA);
    printf("atom %llx\n", U(a));
    printf("get %llu\n", U(GETI("TApplication")));
    for (i = 0; i < 72; i++) if (out[i] != first[i]) same = 0;
    printf("same %d\n", same);
"#,
    );
    // the name pointer is stored as it was (the table keeps the 72 bytes, the pointer fields are not followed)
    assert_eq!(l, ["atom c000", "get 1", "same 1"]);
}

#[test]
fn register_stops_on_every_form_the_pages_leave_open() {
    let setup = "static const char *ign;";
    let mk = |edit: &str| format!("mkwc(\"TApplication\"); {edit} CALL(fle_imp_RegisterClassW, WCA);");
    let cases: Vec<(String, String, &str)> = vec![
        ("null-pointer".into(), "CALL(fle_imp_RegisterClassW, 0);".into(), "lpWndClass is NULL"),
        ("null-proc".into(), mk("p64(8, 0);"), "lpfnWndProc is NULL"),
        ("hinst-null".into(), mk("p64(24, 0);"), "hInstance is not the lifted DLL"),
        ("hinst-other".into(), mk("p64(24, 0x400001);"), "hInstance is not the lifted DLL"),
        ("hinst-high".into(), mk("p64(24, 0x100400000ull);"), "hInstance is not the lifted DLL"),
        ("global-class".into(), mk("p32(0, 0x4000);"), "CS_GLOBALCLASS"),
        ("global-class-with-others".into(), mk("p32(0, 0x4003);"), "CS_GLOBALCLASS"),
        ("cls-extra-41".into(), mk("p32(16, 41);"), "outside 0..40"),
        ("cls-extra-negative".into(), mk("p32(16, 0xFFFFFFFF);"), "outside 0..40"),
        ("wnd-extra-41".into(), mk("p32(20, 41);"), "outside 0..40"),
        ("wnd-extra-negative".into(), mk("p32(20, 0x80000000);"), "outside 0..40"),
        ("wnd-extra-minus-one".into(), mk("p32(20, 0xFFFFFFFF);"), "outside 0..40"),
        ("cls-extra-minus-one".into(), mk("p32(16, 0xFFFFFFFF);"), "outside 0..40"),
        ("name-null".into(), mk("p64(64, 0);"), "atom or NULL"),
        ("name-atom".into(), mk("p64(64, 0xC001);"), "atom or NULL"),
        ("name-atom-max".into(), mk("p64(64, 0xFFFF);"), "atom or NULL"),
        ("name-empty".into(), mk("nmb[0] = 0;"), "empty"),
        ("name-non-ascii".into(), mk("nmb[3] = 0x0141;"), "above 0x7F"),
        ("name-non-ascii-high-byte".into(), mk("nmb[0] = 0x0154;"), "above 0x7F"),
    ];
    for (label, stmts, word) in &cases {
        assert_aborts("class-abort-reg", setup, "RegisterClassW", word, &[(label.as_str(), stmts.as_str())]);
    }
}

#[test]
fn the_smallest_value_above_16_bits_is_a_string_pointer_and_0xffff_is_an_atom() {
    // IS_INTRESOURCE: the high 48 bits are zero. 0x10000 is the smallest pointer value, 0xFFFF the largest atom. A page is mapped
    // at 0x10000 (the lowest address Linux allows by default, vm.mmap_min_addr = 65536) with a class name in it.
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
    let o = run_src(
        "class-low-pointer",
        top,
        r#"
    uint16_t *p = low_name("TApplication");
    uint64_t a;
    mkwc("TApplication");
    p64(64, A(p));
    a = CALL(fle_imp_RegisterClassW, WCA);
    printf("reg %llx\n", U(a));
    fill_out(0xAA);
    printf("get %llu\n", U(CALL(fle_imp_GetClassInfoW, 0x400000, A(p), A(out))));
    printf("same %d\n", out[0] == 3);
"#,
    );
    assert!(o.ok && o.stdout == "reg c000\nget 1\nsame 1\n", "{} {}", o.stdout, o.stderr);
    let o = run_src(
        "class-low-pointer-get-unknown",
        top,
        "    uint16_t *p = low_name(\"EDIT\");\n    CALL(fle_imp_GetClassInfoW, 0x400000, A(p), A(out));\n    printf(\"returned\\n\");",
    );
    assert_eq!(o.signal, Some(6), "{}", o.stderr);
    assert!(o.stderr.contains("is a system class"), "{}", o.stderr);
}

#[test]
fn the_extra_bytes_0_and_40_are_accepted_in_both_fields() {
    // About Window Classes: the call fails "if the amount of extra class memory requested is greater than 40 bytes" (and the same
    // for the window memory), so 0 and 40 are inside
    let l = lines(
        "class-reg-extra",
        r#"
    uint64_t a, b, c, d;
    mkwc("A0"); p32(16, 0); p32(20, 0); a = CALL(fle_imp_RegisterClassW, WCA);
    mkwc("A40"); p32(16, 40); p32(20, 40); b = CALL(fle_imp_RegisterClassW, WCA);
    mkwc("A40a"); p32(16, 40); p32(20, 0); c = CALL(fle_imp_RegisterClassW, WCA);
    mkwc("A40b"); p32(16, 0); p32(20, 40); d = CALL(fle_imp_RegisterClassW, WCA);
    printf("%llx %llx %llx %llx\n", U(a), U(b), U(c), U(d));
"#,
    );
    assert_eq!(l, ["c000 c001 c002 c003"]);
}

#[test]
fn a_name_that_is_not_registered_and_not_a_system_class_is_not_found_whatever_its_spelling() {
    // decision 0215 (sweep1; old: decision 0123 answered "not found" only for the seven spellings of the guest and for T + an upper-case letter, and stopped for every other name): Learn GetClassInfoW returns zero when no class
    // matches. The case of the spelling decides nothing either (no class is registered here, so there is nothing that differs only in case).
    let l = lines(
        "class-get-own-case",
        r#"
    printf("%llu %llu %llu %llu\n", U(GETI("tputilwindow")), U(GETI("XPUTILWINDOW")), U(GETI("ICMCALLBACKWND")), U(GETI("ttimerwindow")));
"#,
    );
    assert_eq!(l, ["0 0 0 0"]);
}

#[test]
fn an_all_upper_spelling_that_is_t_and_an_upper_case_letter_is_answered_not_found_by_the_delphi_name_rule_since_decision_0123() {
    let l = lines(
        "class-get-own-upper-delphi",
        r#"
    uint64_t a;
    a = GETI("TPUTILWINDOW");
    printf("%llu\n", U(a));
"#,
    );
    assert_eq!(l, ["0"]);
    // decision 0215: the name's shape is no rule any more: T followed by a lower-case letter is "not found" as well
    let l = lines("class-get-t-lower", "printf(\"%llu\\n\", U(GETI(\"TpUtilWindow\")));");
    assert_eq!(l, ["0"]);
}

#[test]
fn a_name_of_256_units_is_registered_and_one_of_257_stops() {
    let l = lines(
        "class-reg-256",
        r#"
    char n256[257];
    uint64_t a, b;
    memset(n256, 'x', 256); n256[256] = 0;
    a = REG(n256);
    n256[255] = 'y';
    b = REG(n256);
    printf("atoms %llx %llx\n", U(a), U(b));
    printf("get %llu\n", U(GETI(n256)));
    n256[255] = 'x';
    printf("get x %llu\n", U(GETI(n256)));
"#,
    );
    assert_eq!(l, ["atoms c000 c001", "get 1", "get x 1"]);
    assert_aborts(
        "class-abort-reg-257",
        "char n[260]; memset(n, 'x', 257); n[257] = 0;",
        "RegisterClassW",
        "longer than 256",
        &[("257", "mkwc(n); CALL(fle_imp_RegisterClassW, WCA);")],
    );
    assert_aborts(
        "class-abort-reg-300",
        "char n[400]; memset(n, 'x', 300); n[300] = 0;",
        "RegisterClassW",
        "longer than 256",
        &[("300", "mkwc(n); CALL(fle_imp_RegisterClassW, WCA);")],
    );
}

#[test]
fn a_second_registration_of_a_name_stops_and_a_name_that_differs_only_in_case_stops_too() {
    assert_aborts(
        "class-abort-dup",
        "",
        "RegisterClassW",
        "already registered",
        &[("same", "REG(\"TApplication\"); REG(\"TApplication\");")],
    );
    assert_aborts(
        "class-abort-case",
        "",
        "RegisterClassW",
        "differs only in case",
        &[("upper", "REG(\"TApplication\"); REG(\"TAPPLICATION\");"), ("lower", "REG(\"TApplication\"); REG(\"tapplication\");")],
    );
    // a name that is a prefix or an extension of a registered one is a different name
    let l = lines(
        "class-reg-prefix",
        r#"
    uint64_t a = REG("TApplication"), b = REG("TApplicatio"), c = REG("TApplicationX"), d = REG("TApplication2");
    printf("%llx %llx %llx %llx %llu\n", U(a), U(b), U(c), U(d), U(fle_class_count()));
"#,
    );
    assert_eq!(l, ["c000 c001 c002 c003 4"]);
}

#[test]
fn the_atoms_end_at_0xffff_and_the_next_registration_stops() {
    let o = run(
        "class-atoms",
        r#"
    char nm[16];
    uint64_t first = 0, last = 0, i;
    for (i = 0; i < 0x4000; i++) {
        sprintf(nm, "C%05llu", U(i));
        last = REG(nm);
        if (i == 0) first = last;
    }
    printf("first %llx last %llx count %llu\n", U(first), U(last), U(fle_class_count()));
    fflush(stdout);
    sprintf(nm, "C%05d", 99999);
    REG(nm);
    printf("returned %llx\n", U(R[0]));
"#,
    );
    assert!(o.stdout.starts_with("first c000 last ffff count 16384\n"), "{}", o.stdout);
    assert!(!o.stdout.contains("returned"), "{}", o.stdout);
    assert_eq!(o.signal, Some(6));
    assert!(o.stderr.contains("fle: RegisterClassW: ") && o.stderr.contains("used up"), "{}", o.stderr);
}

// ---- 3. GetClassInfoW ----

#[test]
fn get_class_info_copies_the_registered_72_bytes_and_returns_nonzero() {
    let l = lines(
        "class-get",
        r#"
    uint64_t a, r;
    uint32_t le;
    int i, same = 1;
    uint8_t keep[72];
    a = REG("TTimerWindow");
    memcpy(keep, wcb, 72);
    REG("TApplication");
    p32(0, 0x77);
    SETLE(0x5678);
    fill_out(0xAA);
    r = GETI("TTimerWindow");
    printf("ret %llu\n", U(r));
    printf("regs %d %d %d %d\n", R[1] == 0x400000, R[8] == A(out), R[9] == 0, R[4] == A(stk));
    printf("return address %llx\n", U(stk[0]));
    for (i = 0; i < 72; i++) if (out[i] != keep[i]) same = 0;
    printf("copy %d style %u proc %llx hinst %llx name %llx\n", same, (unsigned)out[0], U(*(uint64_t *)(out + 8)), U(*(uint64_t *)(out + 24)), U(*(uint64_t *)(out + 64)) == A(nmb));
    printf("last error %x\n", LE());
    fill_out(0xAA);
    r = GETI("TApplication");
    printf("second %llu style %u\n", U(r), (unsigned)out[0]);
    printf("count %llu\n", U(fle_class_count()));
"#,
    );
    assert_eq!(
        l,
        [
            "ret 1",
            "regs 1 1 1 1",
            "return address dead0000",
            "copy 1 style 3 proc 521000 hinst 400000 name 1",
            "last error 5678",
            "second 1 style 3",
            "count 2",
        ]
    );
}

#[test]
fn get_class_info_of_the_guests_own_names_before_they_are_registered_returns_zero_and_writes_nothing() {
    let l = lines(
        "class-get-own",
        r#"
    static const char *const own[] = { "TPUtilWindow", "TTimerWindow", "TLightweightLayeredControl", "TApplication",
                                       "VTPanningWindow", "TILRemoteWindow" };
    unsigned k;
    SETLE(0x4321);
    for (k = 0; k < 6; k++) {
        uint64_t r;
        fill_out(0xAA);
        r = GETI(own[k]);
        printf("%s ret %llu untouched %d regs %d %d\n", own[k], U(r), out_is(0xAA), R[4] == A(stk), stk[0] == 0xdead0000u);
    }
    printf("last error %x count %llu\n", LE(), U(fle_class_count()));
    REG("TApplication");
    fill_out(0xAA);
    printf("after register %llu\n", U(GETI("TApplication")));
    fill_out(0xAA);
    printf("others still 0 %llu %llu\n", U(GETI("TTimerWindow")), U(out_is(0xAA)));
"#,
    );
    let names = ["TPUtilWindow", "TTimerWindow", "TLightweightLayeredControl", "TApplication", "VTPanningWindow", "TILRemoteWindow"];
    let mut want: Vec<String> = names.iter().map(|n| format!("{n} ret 0 untouched 1 regs 1 1")).collect();
    want.push("last error 4321 count 0".into());
    want.push("after register 1".into());
    want.push("others still 0 0 1".into());
    assert_eq!(l, want);
}

#[test]
fn get_class_info_of_a_system_class_stops_the_run_and_any_other_unregistered_name_is_not_found() {
    // decision 0215 (old: every name that was neither one of the guest's seven static names nor `T` + an upper-case letter stopped): the system classes (About Window Classes, System Classes) are registered by the system, whose data
    // are not modelled: they stop. Every other unregistered name is "not found" (0), the buffer untouched.
    let names = [
        "EDIT", "Edit", "BUTTON", "Button", "COMBOBOX", "ComboBox", "ListBox", "STATIC", "ScrollBar", "MDIClient", "ComboLBox", "Message",
        "DDEMLEvent", "#32768", "#32769", "#32770", "#32771", "#32772",
    ];
    let cases: Vec<(String, String)> = names.iter().enumerate().map(|(i, n)| (format!("n{i}"), format!("GETI(\"{n}\");"))).collect();
    let refs: Vec<(&str, &str)> = cases.iter().map(|(l, c)| (l.as_str(), c.as_str())).collect();
    assert_aborts("class-abort-get-name", "fill_out(0xAA);", "GetClassInfoW", "is a system class", &refs);
    // the same names stop before any class exists and after the guest's classes exist
    assert_aborts(
        "class-abort-get-name-after",
        "REG(\"TApplication\"); REG(\"TTimerWindow\");",
        "GetClassInfoW",
        "is a system class",
        &[("edit", "GETI(\"EDIT\");"), ("button", "GETI(\"Button\");")],
    );
    let l = lines("class-get-other-names", "fill_out(0xAA); printf(\"%llu %llu %llu %llu %d\\n\", U(GETI(\"RICHEDIT20W\")), U(GETI(\"X\")), U(GETI(\"ICMCALLBACKWND\")), U(GETI(\"TpUtilWindow\")), out_is(0xAA));");
    assert_eq!(l, ["0 0 0 0 1"]);
    // a registered name of this module is found whatever it is (a name other than the guest's static ones, registered by the
    // guest itself at run time)
    let l = lines("class-get-dynamic", "REG(\"TCustomForm\"); fill_out(0xAA); printf(\"%llu %d\\n\", U(GETI(\"TCustomForm\")), out_is(0xAA));");
    assert_eq!(l, ["1 0"]);
}

#[test]
fn get_class_info_matches_names_unit_for_unit_and_stops_on_a_case_difference() {
    assert_aborts(
        "class-abort-get-case",
        "REG(\"TApplication\");",
        "GetClassInfoW",
        "differs only in case",
        &[("lower", "GETI(\"tapplication\");"), ("upper", "GETI(\"TAPPLICATION\");"), ("mixed", "GETI(\"TApplicatioN\");")],
    );
}

#[test]
fn get_class_info_stops_on_the_module_the_buffer_and_the_name() {
    let setup = "REG(\"TApplication\");";
    assert_aborts(
        "class-abort-get-hinst",
        setup,
        "GetClassInfoW",
        "hInstance is NULL",
        &[("null", "getinfo(\"TApplication\", 0);"), ("null-edit", "getinfo(\"EDIT\", 0);")],
    );
    assert_aborts(
        "class-abort-get-hinst-other",
        setup,
        "GetClassInfoW",
        "hInstance is not the lifted DLL",
        &[
            ("small", "getinfo(\"TApplication\", 1);"),
            ("next", "getinfo(\"TApplication\", 0x400001);"),
            ("module-token", "getinfo(\"TApplication\", 0x7FF800010000ull);"),
            ("high", "getinfo(\"TApplication\", 0x100400000ull);"),
        ],
    );
    assert_aborts("class-abort-get-out", setup, "GetClassInfoW", "lpWndClass is NULL", &[("null", "static uint16_t q[] = { 'T',0 }; CALL(fle_imp_GetClassInfoW, 0x400000, A(q), 0);")]);
    assert_aborts(
        "class-abort-get-atom",
        setup,
        "GetClassInfoW",
        "atom or NULL",
        &[
            ("null", "CALL(fle_imp_GetClassInfoW, 0x400000, 0, A(out));"),
            ("atom", "CALL(fle_imp_GetClassInfoW, 0x400000, 0xC000, A(out));"),
            ("atom-max", "CALL(fle_imp_GetClassInfoW, 0x400000, 0xFFFF, A(out));"),
        ],
    );
    assert_aborts(
        "class-abort-get-empty",
        setup,
        "GetClassInfoW",
        "empty",
        &[("empty", "static uint16_t q[] = { 0 }; CALL(fle_imp_GetClassInfoW, 0x400000, A(q), A(out));")],
    );
    assert_aborts(
        "class-abort-get-long",
        "static uint16_t q[300]; unsigned i; for (i = 0; i < 299; i++) q[i] = 'x';",
        "GetClassInfoW",
        "longer than 256",
        &[("257", "q[257] = 0; CALL(fle_imp_GetClassInfoW, 0x400000, A(q), A(out));")],
    );
    assert_aborts(
        "class-abort-get-non-ascii",
        setup,
        "GetClassInfoW",
        "above 0x7F",
        &[("n", "static uint16_t q[] = { 'T', 0x0141, 0 }; CALL(fle_imp_GetClassInfoW, 0x400000, A(q), A(out));")],
    );
}

#[test]
fn nothing_is_stored_before_a_stop() {
    // a RegisterClassW that stops adds nothing and a GetClassInfoW that stops writes nothing; the run ends, so what is checked is
    // what the process printed before the stop and the state a signal handler sees
    let o = run_src(
        "class-nothing-stored",
        "#include <signal.h>\n#include <unistd.h>\nstatic void h(int s) { (void)s; printf(\"count %llu out %d\\n\", U(fle_class_count()), out_is(0xAA)); fflush(stdout); _exit(7); }",
        "    signal(SIGABRT, h);\n    fill_out(0xAA);\n    REG(\"TApplication\");\n    mkwc(\"TTimerWindow\"); p32(16, 99);\n    CALL(fle_imp_RegisterClassW, WCA);",
    );
    assert_eq!(o.stdout, "count 1 out 1\n", "{}", o.stderr);
    let o = run_src(
        "class-nothing-written",
        "#include <signal.h>\n#include <unistd.h>\nstatic void h(int s) { (void)s; printf(\"count %llu out %d\\n\", U(fle_class_count()), out_is(0xAA)); fflush(stdout); _exit(7); }",
        "    signal(SIGABRT, h);\n    fill_out(0xAA);\n    REG(\"TApplication\");\n    GETI(\"EDIT\");",
    );
    assert_eq!(o.stdout, "count 1 out 1\n", "{}", o.stderr);
}

// ---- 4. Threads ----

#[test]
fn a_second_thread_stops_the_run_in_both_apis() {
    for (label, api, call) in [
        ("register", "RegisterClassW", "REG(\"TApplication\")"),
        ("getclassinfo", "GetClassInfoW", "GETI(\"TApplication\")"),
    ] {
        let o = run_src(
            &format!("class-thread-{label}"),
            &format!("static void *th(void *x) {{ (void)x; {call}; printf(\"returned-from-thread\\n\"); return 0; }}"),
            "    pthread_t t;\n    REG(\"TTimerWindow\");\n    pthread_create(&t, 0, th, 0);\n    pthread_join(t, 0);\n    printf(\"returned\\n\");",
        );
        assert!(!o.ok && !o.stdout.contains("returned"), "{label}: {}", o.stdout);
        assert_eq!(o.signal, Some(6), "{label}");
        assert!(o.stderr.contains(&format!("fle: {api}: ")) && o.stderr.contains("second thread"), "{label}: {}", o.stderr);
    }
}

// ---- 5. Source checks ----

#[test]
fn source_uses_no_host_window_class_module_clock_thread_process_or_file_and_never_calls_the_window_procedure() {
    let code = strip_comments(&std::fs::read_to_string(src_dir().join("fle_class.c")).unwrap());
    for bad in [
        "getpid", "getppid", "gettid", "getenv", "environ", "fopen", "open(", "stat(", "/proc", "/etc", "/home", "/dev", "/tmp", "exit(", "_exit",
        "clock_gettime", "gettimeofday", "time(", "clock(", "nanosleep", "usleep", "sleep(", "select(", "poll(", "<time.h>", "<sys/time.h>",
        "pthread", "<sys/", "<unistd.h>", "<signal.h>", "fork(", "exec", "syscall", "sched_", "futex", "asm", "__asm__", "uintptr_t", "abort(",
        "X11", "DISPLAY", "xrandr", "wayland", "fb0", "ioctl", "emscripten", "SDL", "dlopen", "LoadLibrary", "fle_load", "fle_res",
        "fle_call_guest", "fle_callback", "fle_dispatch", "fle_gdi", "fle_gdc",
        // "fle_set_last_error" was in this list until decision 0123 (UnregisterClassW sets 1411 and 1412: the codes of System Error Codes for the two failures the page names without a code)
    ] {
        assert!(!code.contains(bad), "fle_class.c must not use {bad}");
    }
    // `long` is not portable (32 bits on wasm32 and on Windows, 64 on Linux): fixed-width integers only
    let b = code.as_bytes();
    let mut i = 0;
    while let Some(p) = code[i..].find("long") {
        let at = i + p;
        let before = at == 0 || !(b[at - 1].is_ascii_alphanumeric() || b[at - 1] == b'_');
        let after = at + 4 >= b.len() || !(b[at + 4].is_ascii_alphanumeric() || b[at + 4] == b'_');
        assert!(!(before && after), "fle_class.c uses `long` near byte {at}");
        i = at + 4;
    }
    // the window procedure is read from the structure as a number and is not turned into a function pointer
    assert!(!code.contains("(void (*)") && !code.contains("WNDPROC"), "the window procedure must not be called");
    assert!(code.contains("fle_host_tid()") && code.contains("fle_stop(") && code.contains("fle_ld(") && code.contains("fle_st("));
    assert!(code.contains("#include \"fle_host.h\""));
    let h = strip_comments(&std::fs::read_to_string(src_dir().join("fle_class.h")).unwrap());
    for bad in ["asm", "__asm__", "<unistd.h>", "<sys/", "<pthread.h>", "<time.h>", "long "] {
        assert!(!h.contains(bad), "fle_class.h must not contain {bad}");
    }
}

// ---- 6. The bytes of the real DLL that the decision rests on (docs/investigations/getclassinfo-caller.md) ----

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
fn le64(d: &[u8], o: usize) -> u64 {
    u64::from_le_bytes(d[o..o + 8].try_into().unwrap())
}
/// (virtual size, rva, raw size, raw offset) of every section.
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
fn wstr_at(dll: &[u8], va: u64) -> String {
    let b = bytes_at(dll, va, 2 * 64);
    let mut s = String::new();
    for c in b.chunks(2) {
        let u = u16::from_le_bytes([c[0], c[1]]);
        if u == 0 {
            break;
        }
        s.push(char::from_u32(u as u32).unwrap());
    }
    s
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
            let e = le64(dll, t);
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
/// The VAs of every `call rel32` in .text whose target is `thunk` (a scan of the E8 bytes).
fn call_sites(dll: &[u8], thunk: u64) -> Vec<u64> {
    let (vs, sva, _, rp) = sections(dll)[0];
    let text = &dll[rp as usize..rp as usize + vs as usize];
    let mut found = Vec::new();
    for i in 0..text.len().saturating_sub(5) {
        if text[i] == 0xe8 {
            let rel = i32::from_le_bytes(text[i + 1..i + 5].try_into().unwrap()) as i64;
            let va = 0x400000 + sva as u64 + i as u64;
            if (va as i64 + 5 + rel) as u64 == thunk {
                found.push(va);
            }
        }
    }
    found
}

const GETCLASSINFO_THUNK: u64 = 0x424ba0;
const GETCLASSINFO_SLOT: u64 = 0x1970a90;
const REGISTERCLASS_THUNK: u64 = 0x425260;
const REGISTERCLASS_SLOT: u64 = 0x1970b98;

#[test]
fn the_two_thunks_jump_to_the_user32_slots_and_nothing_else_points_at_them() {
    let Some(dll) = real_dll() else { return };
    let imps = imports(&dll);
    for (slot, thunk, name) in [(GETCLASSINFO_SLOT, GETCLASSINFO_THUNK, "GetClassInfoW"), (REGISTERCLASS_SLOT, REGISTERCLASS_THUNK, "RegisterClassW")] {
        let named: Vec<_> = imps.iter().filter(|(_, _, s)| *s == slot).collect();
        assert_eq!(named.len(), 1);
        assert_eq!((named[0].0.to_lowercase(), named[0].1.as_str()), ("user32.dll".to_string(), name));
        let t = bytes_at(&dll, thunk, 6);
        assert_eq!(&t[..2], [0xff, 0x25], "{name}: jmp [rip+disp32]");
        assert_eq!((thunk as i64 + 6 + i32::from_le_bytes(t[2..6].try_into().unwrap()) as i64) as u64, slot, "{name}");
        // no absolute VA (or RVA) of the thunk or the slot is in the file as data: the direct call sites are all the references
        for va in [thunk, slot] {
            for pat in [(va as u32).to_le_bytes().to_vec(), va.to_le_bytes().to_vec(), ((va - 0x400000) as u32).to_le_bytes().to_vec()] {
                assert!(!dll.windows(pat.len()).any(|w| w == pat.as_slice()), "{name} {va:#x}: {pat:02x?}");
            }
        }
    }
    // the guest imports the other class APIs too; they are the ones this decision leaves unanswered
    let have: Vec<&str> = imps.iter().filter(|(d, n, _)| d.eq_ignore_ascii_case("user32.dll") && n.contains("Class")).map(|(_, n, _)| n.as_str()).collect();
    for n in ["GetClassInfoW", "RegisterClassW", "UnregisterClassW", "GetClassNameW"] {
        assert!(have.contains(&n), "{n} in {have:?}");
    }
    for n in ["GetClassInfoExW", "RegisterClassExW", "RegisterClassA", "GetClassInfoA"] {
        assert!(!have.contains(&n), "the guest does not import {n}");
    }
}

#[test]
fn the_guest_has_11_get_class_info_sites_and_8_register_class_sites() {
    let Some(dll) = real_dll() else { return };
    assert_eq!(
        call_sites(&dll, GETCLASSINFO_THUNK),
        [0x52a820, 0x611574, 0x61158e, 0x6115a0, 0x6115b4, 0x61199f, 0x6d83d6, 0x71ddaf, 0x867e95, 0xa1ba6f, 0xd706a6]
    );
    assert_eq!(call_sites(&dll, REGISTERCLASS_THUNK), [0x52a859, 0x6119f4, 0x6d8410, 0x71dde9, 0x81cfab, 0x867eb3, 0xa1baab, 0xd706e0]);
}

#[test]
fn the_static_classes_have_the_names_the_runtime_knows_and_a_window_procedure_and_no_class_style() {
    let Some(dll) = real_dll() else { return };
    // (the WNDCLASSW in .data, the name the guest registers, the window procedure: DefWindowProcW's thunk or a function of the DLL)
    let static_wcs: [(u64, &str, u64); 6] = [
        (0x13be810, "TPUtilWindow", 0x424920),
        (0x13f0648, "TTimerWindow", 0x6d8330),
        (0x13f5f20, "TLightweightLayeredControl", 0x71dd10),
        (0x1409cb8, "TApplication", 0x424920),
        (0x14d6b48, "VTPanningWindow", 0x424920),
        (0x151c970, "TILRemoteWindow", 0xd70600),
    ];
    for (va, name, proc) in static_wcs {
        let w = bytes_at(&dll, va, 72);
        assert_eq!(le32(&w, 0), 0, "{name}: style is 0 (no CS_GLOBALCLASS)");
        assert_eq!(le64(&w, 8), proc, "{name}: lpfnWndProc");
        assert_eq!((le32(&w, 16), le32(&w, 20)), (0, 0), "{name}: no extra bytes");
        assert_eq!(le64(&w, 24), 0, "{name}: hInstance is stored by the code before the call (the guest's HInstance, 0x400000)");
        assert_eq!(wstr_at(&dll, le64(&w, 64)), name);
    }
    // the seventh name (ICMCALLBACKWND) is in a WNDCLASSW built on the stack (f_81cf60, wc.lpszClassName = lea 0x81d014)
    assert_eq!(wstr_at(&dll, 0x81d014), "ICMCALLBACKWND");
    // DefWindowProcW is the thunk 0x424920 (the window procedure of three static classes is the system's)
    let imps = imports(&dll);
    let t = bytes_at(&dll, 0x424920, 6);
    let slot = (0x424920i64 + 6 + i32::from_le_bytes(t[2..6].try_into().unwrap()) as i64) as u64;
    let named: Vec<_> = imps.iter().filter(|(_, _, s)| *s == slot).collect();
    assert_eq!((named[0].0.to_lowercase(), named[0].1.as_str()), ("user32.dll".to_string(), "DefWindowProcW"));
}

#[test]
fn f_52a7f0_asks_for_its_class_then_registers_it_and_then_creates_the_window() {
    let Some(dll) = real_dll() else { return };
    let at = |va: u64, n: usize| bytes_at(&dll, va, n).iter().map(|x| format!("{x:02x}")).collect::<String>();
    // HInstance (0x15ec298) -> [0x13be828] (wc.hInstance) and rcx; rdx = wc.lpszClassName ([0x13be850]); r8 = &buffer; the call
    assert_eq!(at(0x52a800, 7), "488b05911a0c01");
    assert_eq!(at(0x52a807, 7), "4889051a40e900");
    assert_eq!(at(0x52a81c, 5), "4c8d4568e8");
    // found and the class's window procedure is the stub 0x424920 (DefWindowProcW): skip the registration; found with another
    // procedure: UnregisterClassW(name, HInstance) and register; not found: register
    assert_eq!(at(0x52a825, 13), "85c00f95c084c0740d488d0deb");
    assert_eq!(at(0x52a852, 12), "488d0db73fe900e802aaefff");
    // RegisterClassW's result is not tested: the next instruction is the CreateWindowExW set-up (ecx = 0x80, WS_EX_TOOLWINDOW)
    assert_eq!(at(0x52a85e, 5), "b980000000");
    // no read of the last error between the class calls and the window: none of the thunks of GetLastError / SetLastError (the IAT
    // slots 0x19713c0, 0x19713d0) is called in f_52a7f0 (0x52a7f0..0x52a8f4): every `call rel32` is listed
    let code = bytes_at(&dll, 0x52a7f0, 0x52a8f4 - 0x52a7f0);
    let mut calls = Vec::new();
    for i in 0..code.len() - 4 {
        if code[i] == 0xe8 {
            let rel = i32::from_le_bytes(code[i + 1..i + 5].try_into().unwrap()) as i64;
            let t = (0x52a7f0 + i as i64 + 5 + rel) as u64;
            if (0x424000..0x426000).contains(&t) {
                calls.push(t);
            }
        }
    }
    // GetClassInfoW, (UnregisterClassW), RegisterClassW, CreateWindowExW (0x425780 is the thunk of the window creation), SetWindowLongPtrW
    assert_eq!(calls, [0x424ba0, 0x425670, 0x425260, 0x425780, 0x425980]);
}
