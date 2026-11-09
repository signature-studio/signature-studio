//! Tests for the virtual standard cursor of runtime/fle_cursor.c: user32!LoadCursorW(NULL, IDC_ARROW)
//! (docs/decisions/0059-virtual-cursors.md; sources: docs/investigations/loadcursor-caller.md).
//!
//! Expected values are written here from the decision (the tables of the Microsoft Learn pages About Cursors, LoadCursorW and
//! IS_INTRESOURCE, and the choices of the decision), not read back from the C code. The identifier IDC_ARROW = 32512 is on the
//! About Cursors page: official. The handle value (0x7FCA0000 + 4) is a PROVISIONAL choice of the decision (no page gives a value).
//! The host is Linux: nothing is compared with a real Windows; no cursor, theme, mouse setting, display, resource, clock, thread or
//! process is read.
//!
//! The C sources of fle_cursor.c / fle_cursor.h come from runtime/ when fle_cursor.c is there, otherwise from out-cur/work/
//! (development). FLE_CUR_SRC_DIR (a directory with fle_cursor.c and fle_cursor.h) overrides both: the mutation runs of the run
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
    if let Ok(d) = std::env::var("FLE_CUR_SRC_DIR") {
        return PathBuf::from(d);
    }
    if runtime_dir().join("fle_cursor.c").exists() {
        runtime_dir()
    } else {
        manifest().join("../../out-cur/work")
    }
}
fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-cur/test");
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

/// fle_cursor.c (warnings are errors) and the files it links with: fle_gdi.c and fle_gdc.c (the other two handle tables: the
/// regions and the "not issued by this table" stops are tested against each other), fle_imports.c (GetLastError /
/// fle_set_last_error) and fle_env.c (fle_stop).
fn objs() -> &'static Vec<PathBuf> {
    static O: OnceLock<Vec<PathBuf>> = OnceLock::new();
    O.get_or_init(|| {
        let dir = work_dir().join(etxt_common::per_process("obj-cursor"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut v = Vec::new();
        for (name, ours) in [("fle_cursor.c", true), ("fle_gdi.c", false), ("fle_raster.c", false), ("fle_gdc.c", false), ("fle_imports.c", false), ("fle_env.c", false)] {
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
#include <sys/resource.h>
#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_gdc.h"
#include "fle_gdi.h"
#include "fle_cursor.h"

uint64_t R[16];
static uint64_t stk[32];

/* a stop is an abort(): no core file for it (it only slows the many stop tests down) */
__attribute__((constructor)) static void nocore(void)
{
    struct rlimit r = { 0, 0 };
    setrlimit(RLIMIT_CORE, &r);
}

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
#define CUR(inst, name) CALL(fle_imp_LoadCursorW, (inst), (name))
"#;

fn compile_main(name: &str, top: &str, body: &str) -> PathBuf {
    let dir = work_dir();
    let uname = etxt_common::uniq(name);
    let c = dir.join(format!("{uname}.c"));
    let exe = dir.join(&uname);
    let src = format!("{PRELUDE}\n{top}\nint main(int argc, char **argv)\n{{\n    (void)argc; (void)argv;\n{body}\n    return 0;\n}}\n");
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
    let exe = compile_main(name, "", body);
    exec(&exe, &[])
}

fn lines(name: &str, body: &str) -> Vec<String> {
    let o = run(name, body);
    assert!(o.ok, "{name} did not exit normally (signal {:?}): {}\n{}", o.signal, o.stderr, o.stdout);
    o.stdout.lines().map(|s| s.to_string()).collect()
}

/// One program for every call: argv = hInstance lpCursorName; each a u64 (the whole register). Output "ret <hex>".
fn cli() -> &'static PathBuf {
    static E: OnceLock<PathBuf> = OnceLock::new();
    E.get_or_init(|| {
        compile_main(
            "cursor-cli",
            "",
            r#"
    uint64_t inst = strtoull(argv[1], 0, 0);
    uint64_t name = strtoull(argv[2], 0, 0);
    uint64_t r = CUR(inst, name);
    printf("ret %llx\n", U(r));
"#,
        )
    })
}

/// What LoadCursorW(inst, name) did: Ok(return value), or Err(stderr) when the run stopped (SIGABRT, "fle: LoadCursorW", nothing
/// printed before the stop).
fn call(inst: u64, name: u64) -> Result<u64, String> {
    let o = exec(cli(), &[&format!("{inst:#x}"), &format!("{name:#x}")]);
    if o.ok {
        let t: Vec<&str> = o.stdout.trim().split(' ').collect();
        assert_eq!(t[0], "ret", "odd output {}", o.stdout);
        return Ok(u64::from_str_radix(t[1], 16).unwrap());
    }
    assert_eq!(o.signal, Some(6), "inst {inst:#x} name {name:#x}: not SIGABRT: {}", o.stderr);
    assert!(o.stderr.contains("fle: LoadCursorW"), "inst {inst:#x} name {name:#x}: no 'fle: LoadCursorW' message: {}", o.stderr);
    assert!(o.stdout.is_empty(), "inst {inst:#x} name {name:#x}: output before the stop: {}", o.stdout);
    Err(o.stderr)
}

/// The value of `#define NAME <number>` (decimal, or UINT64_C(0x..), or 0x..) in a runtime header.
fn def_const(file: &str, name: &str) -> u64 {
    let t = std::fs::read_to_string(src_dir().join(file)).or_else(|_| std::fs::read_to_string(runtime_dir().join(file))).unwrap();
    let line = t.lines().find(|l| l.starts_with(&format!("#define {name} "))).unwrap_or_else(|| panic!("{name} not in {file}"));
    let rest = line[format!("#define {name} ").len()..].trim_start();
    if let Some(r) = rest.strip_prefix("UINT64_C(") {
        let tok = r.split(')').next().unwrap();
        return u64::from_str_radix(tok.trim_start_matches("0x"), 16).unwrap();
    }
    if let Some(r) = rest.strip_prefix("0x") {
        let tok: String = r.chars().take_while(|c| c.is_ascii_hexdigit()).collect();
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

/// The standard cursors of the Learn page About Cursors (fetched 2026-10-21): OFFICIAL, written here from the page's table
/// "IDC_xxx MAKEINTRESOURCE(n)". IDC_ARROW is the one that is answered.
const LEARN_IDC: &[(u64, &str)] = &[
    (32512, "IDC_ARROW"), (32513, "IDC_IBEAM"), (32514, "IDC_WAIT"), (32515, "IDC_CROSS"), (32516, "IDC_UPARROW"), (32642, "IDC_SIZENWSE"),
    (32643, "IDC_SIZENESW"), (32644, "IDC_SIZEWE"), (32645, "IDC_SIZENS"), (32646, "IDC_SIZEALL"), (32648, "IDC_NO"), (32649, "IDC_HAND"),
    (32650, "IDC_APPSTARTING"), (32651, "IDC_HELP"), (32671, "IDC_PIN"), (32672, "IDC_PERSON"),
];
/// The page's second table: "additional cursors that do not have identifiers defined in WinUser.h": 32631 and 32652..=32663.
const LEARN_EXTRA: &[u64] = &[32631, 32652, 32653, 32654, 32655, 32656, 32657, 32658, 32659, 32660, 32661, 32662, 32663];

const ARROW: u64 = 32512;
/// The decided handle (decision 0059, PROVISIONAL): base 0x7FCA0000 + 4 * (idc - 32512 + 1).
const HANDLE: u64 = 0x7FCA_0004;

// ---- 1. Table, header, constants ----

#[test]
fn the_row_is_in_the_table_and_declared_in_the_header() {
    let table = std::fs::read_to_string(manifest().join("src/imports_table.rs")).unwrap();
    let h = std::fs::read_to_string(src_dir().join("fle_cursor.h")).unwrap();
    let row = "(\"user32.dll\", \"LoadCursorW\", \"fle_imp_LoadCursorW\", \"jmp_imp:LoadCursorW\")";
    assert_eq!(table.matches(row).count(), 1, "exactly one row in imports_table.rs");
    assert!(h.contains("void fle_imp_LoadCursorW(void)"));
    // the nearby cursor and image APIs are not answered by this decision (the investigation counts their call sites only)
    // ("LoadImageW" was in this list until decision 0061: runtime/fle_loadimage.c answers it, see tests/loadimage.rs)
    // ("RegisterClassW" and "GetClassInfoW" were in this list until decision 0070: runtime/fle_class.c answers them, see tests/class.rs)
    // ("SetCursor", "GetCursor", "DestroyCursor" and "ShowCursor" were in this list until decision 0111: runtime/fle_gdi.c, tests/gdi_bulk.rs)
    for n in [
        "LoadCursorA", "LoadCursorFromFileW", "LoadImageA", "CopyImage", "CopyCursor",
        "ClipCursor", "GetClassLongPtrW",
        // old: "GetCursorInfo" and "SetCursorPos" were in this list; decision 0126 answered them (wall 77 GetCursorInfo, wall 74 SetCursorPos: runtime/fle_win.c, tests/wall_fr11.rs); new: not asserted absent
        // "SetClassLongPtrW" was in this list until decision 0119 (wall 9 / 15: runtime/fle_win.c answers it, tests/wall_fr6.rs); old: asserted absent
    ] {
        assert!(!table.contains(&format!("\"{n}\"")), "{n} must not be answered by this decision");
    }
}

#[test]
fn idc_arrow_is_the_learn_number() {
    // official: About Cursors, IDC_ARROW = MAKEINTRESOURCE(32512) (written above from the page, not read back)
    assert_eq!(LEARN_IDC[0], (ARROW, "IDC_ARROW"));
    assert_eq!(def_const("fle_cursor.h", "FLE_CURSOR_IDC_ARROW"), 32512);
    assert_eq!(ARROW, 0x7F00, "the guest's immediate `mov edx,0x7f00`");
}

#[test]
fn the_learn_tables_in_this_file_have_no_repeated_number_and_no_repeated_name() {
    let mut nums: Vec<u64> = LEARN_IDC.iter().map(|(n, _)| *n).chain(LEARN_EXTRA.iter().copied()).collect();
    let n = nums.len();
    nums.sort();
    nums.dedup();
    assert_eq!(nums.len(), n);
    let mut names: Vec<&str> = LEARN_IDC.iter().map(|(_, s)| *s).collect();
    let m = names.len();
    names.sort();
    names.dedup();
    assert_eq!(names.len(), m);
    assert_eq!((LEARN_IDC.len(), LEARN_EXTRA.len()), (16, 13));
}

#[test]
fn the_handle_region_is_decided_below_2_gib_and_apart_from_the_other_two_tables() {
    // decision 0059 (provisional): base 0x7FCA0000, 1 MiB
    let base = def_const("fle_cursor.h", "FLE_CURSOR_HANDLE_BASE");
    let size = def_const("fle_cursor.h", "FLE_CURSOR_REGION_SIZE");
    assert_eq!(base, 0x7FCA_0000);
    assert_eq!(size, 0x10_0000);
    // Learn "Interprocess communication": only the lower 32 bits are significant, truncation / sign extension are safe -> < 2^31
    assert!(base + size <= 0x8000_0000);
    assert!(HANDLE > base && HANDLE < base + size);
    // not overlapping the regions of the virtual GDI objects (0054) and the virtual screen DCs (0053)
    let (gb, gs) = (def_const("fle_gdi.h", "FLE_GDI_HANDLE_BASE"), def_const("fle_gdi.h", "FLE_GDI_REGION_SIZE"));
    let (cb, cs) = (def_const("fle_gdc.h", "FLE_GDC_HANDLE_BASE"), def_const("fle_gdc.h", "FLE_GDC_REGION_SIZE"));
    assert!(base >= gb + gs || base + size <= gb, "overlaps the GDI object region");
    assert!(base >= cb + cs || base + size <= cb, "overlaps the screen DC region");
    // above the image (ImageBase 0x400000, SizeOfImage 0x18c0000)
    assert!(base > 0x400000 + 0x18c0000);
}

// ---- 2. The answer ----

#[test]
fn idc_arrow_returns_the_decided_handle() {
    assert_eq!(call(0, ARROW), Ok(HANDLE));
}

#[test]
fn the_handle_is_the_same_on_every_call_and_does_not_depend_on_other_calls() {
    // Learn LoadCursorW: "loads the cursor resource only if it has not been loaded; otherwise, it retrieves the handle to the
    // existing resource". The guest also compares a new result with the one it stored (0x865aeb: cmp rax,[rbx+0x154]).
    let l = lines(
        "cursor-same",
        r#"
    uint64_t a = CUR(0, 32512), b, c, i;
    uint64_t icon = CALL(fle_imp_LoadIconW, 0, 32512);
    b = CUR(0, 32512);
    uint64_t dc = CALL(fle_imp_GetDC, 0);
    c = CUR(0, 32512);
    for (i = 0; i < 1000; i++) if (CUR(0, 32512) != a) { printf("changed at %llu\n", U(i)); return 1; }
    printf("%llx %llx %llx\n", U(a), U(b), U(c));
    printf("icon differs %d dc differs %d\n", icon != a, dc != a);
"#,
    );
    assert_eq!(l, [format!("{HANDLE:x} {HANDLE:x} {HANDLE:x}"), "icon differs 1 dc differs 1".to_string()]);
}

#[test]
fn the_inspection_functions_agree_with_the_call() {
    let l = lines(
        "cursor-inspect",
        r#"
    uint64_t h = CUR(0, 32512);
    printf("%llx %llx\n", U(fle_cursor_handle_of(32512)), U(h));
    printf("%u\n", fle_cursor_idc_of(h));
    printf("%u %u %u %u\n", fle_cursor_idc_of(0), fle_cursor_idc_of(h + 4), fle_cursor_idc_of(h - 4), fle_cursor_idc_of(h + 1));
    printf("%llx %llx %llx\n", U(fle_cursor_handle_of(32513)), U(fle_cursor_handle_of(0)), U(fle_cursor_handle_of(32511)));
"#,
    );
    assert_eq!(l, [format!("{HANDLE:x} {HANDLE:x}"), "32512".to_string(), "0 0 0 0".to_string(), "0 0 0".to_string()]);
}

#[test]
fn it_is_stateless_leaves_the_last_error_alone_and_keeps_the_return_address() {
    // Learn: on failure GetLastError gives the extended error information; this call succeeds, so the last error is not touched
    let l = lines(
        "cursor-state",
        r#"
    uint64_t r1, r2;
    SETLE(0x1234);
    r1 = CUR(0, 32512);
    r2 = CUR(0, 32512);
    printf("same %d\n", r1 == r2);
    printf("last error %x\n", LE());
    printf("return address %llx\n", U(stk[0]));
    SETLE(0);
    CUR(0, 32512);
    printf("last error %x\n", LE());
"#,
    );
    assert_eq!(l, ["same 1", "last error 1234", "return address dead0000", "last error 0"]);
}

#[test]
fn the_answer_does_not_depend_on_the_other_argument_registers() {
    // only rcx and rdx are arguments; r8 and r9 hold whatever the guest left there
    let l = lines(
        "cursor-regs",
        r#"
    uint64_t r = CALL(fle_imp_LoadCursorW, 0, 32512, 0x1122334455667788ull, 0x99aabbccddeeff00ull);
    printf("%llx\n", U(r));
"#,
    );
    assert_eq!(l, [format!("{HANDLE:x}")]);
}

// ---- 3. Stops ----

#[test]
fn a_non_null_hinstance_stops_with_a_named_reason() {
    // 1: a small value; 0x400000: the guest's module (the three dynamic call sites pass [0x15ec298] = the image base);
    // 0x7FF800000000: a virtual module handle; 0x1_0000_0000: only the upper half set (the whole 64 bits of rcx count)
    for inst in [1u64, 0x400000, 0x7FF8_0000_0000, 0x1_0000_0000, u64::MAX] {
        let e = call(inst, ARROW).expect_err("hInstance not NULL");
        assert!(e.contains("hInstance is not NULL") && e.contains("decision 0059"), "{inst:#x}: {e}");
        assert!(e.contains("only IDC_ARROW is answered"), "{inst:#x}: {e}");
    }
}

#[test]
fn a_name_pointer_stops_with_a_named_reason() {
    // Learn IS_INTRESOURCE: an identifier has all bits except the least 16 zero; otherwise it is typically a pointer to a string.
    // The guest's three dynamic sites pass a pointer to a wide string ("VT_HEADERSPLIT", "VT_VERTSPLIT", "VT_MOVEN", "VT_MOVES").
    for name in [0x1_0000u64, 0x7FF7_0000_1000, 0x1_0000_0000 + ARROW, 0x10000 + ARROW, u64::MAX] {
        let e = call(0, name).expect_err("pointer name");
        assert!(e.contains("lpCursorName is a pointer to a name") && e.contains("decision 0059"), "{name:#x}: {e}");
    }
    // with a module as well: hInstance is reported first
    let e = call(0x400000, 0x7FF7_0000_1000).expect_err("module + pointer");
    assert!(e.contains("hInstance is not NULL"), "{e}");
}

#[test]
fn every_other_standard_cursor_stops_by_number() {
    // the 15 other identifiers of About Cursors
    for (id, name) in LEARN_IDC.iter().skip(1) {
        let e = call(0, *id).expect_err(name);
        assert!(e.contains(&format!("MAKEINTRESOURCE({id}) is not IDC_ARROW (32512)")) && e.contains("decision 0059"), "{name}: {e}");
    }
}

#[test]
fn the_additional_cursors_and_other_numbers_stop_by_number() {
    // the page's second table, the neighbours of IDC_ARROW, the obsolete IDC_SIZE (32641) and IDC_ICON (32647) that the pages
    // here do not list, the ends of the 16-bit range (an abort per number is slow, so not every number)
    let mut ids: Vec<u64> = LEARN_EXTRA.to_vec();
    ids.extend([0, 1, 2, 0x7EFF, 0x7F01, 32511, 32641, 32647, 32761, 32767, 32768, 0xFFFE, 0xFFFF]);
    for id in ids {
        let e = call(0, id).expect_err(&format!("{id}"));
        assert!(e.contains(&format!("MAKEINTRESOURCE({id}) is not IDC_ARROW (32512)")), "{id}: {e}");
    }
}

// ---- 4. Source checks ----

#[test]
fn source_uses_no_host_cursor_theme_mouse_display_resource_clock_thread_process_or_file() {
    let code = strip_comments(&std::fs::read_to_string(src_dir().join("fle_cursor.c")).unwrap());
    for bad in [
        "getpid", "getppid", "gettid", "getenv", "extern char **environ", "fopen", "open(", "stat(", "/proc", "/etc", "/home", "/dev", "/tmp", "exit(", "_exit",
        "clock_gettime", "gettimeofday", "time(", "clock(", "nanosleep", "usleep", "sleep(", "select(", "poll(", "<time.h>", "<sys/time.h>",
        "pthread", "<sys/", "<unistd.h>", "<signal.h>", "fork(", "exec", "syscall", "sched_", "futex", "asm", "__asm__", "uintptr_t", "abort(",
        "X11", "DISPLAY", "XCursor", "xcursor", "XCURSOR", "gsettings", "xinput", "libinput", "dconf", "wayland", "ioctl", "emscripten", "SDL",
        "devicePixelRatio", "mouse0", "fle_host", "fle_set_last_error", "fle_reg", "fle_vfs", "fle_gdi", "fle_gdc", "fle_res", "fle_load",
        "malloc", "realloc", "free(",
    ] {
        assert!(!code.contains(bad), "fle_cursor.c must not use {bad}");
    }
    // stateless: no static variable other than the constant name string (a handle is a pure function of the identifier)
    // (decision 0061 added the constant table of the 13 identifiers and a static helper function; the rule it stands for is "no
    // mutable static variable", so a `static` line that is a function (has a parenthesis) or a `const` object is allowed)
    for line in code.lines().filter(|l| l.starts_with("static ")) {
        let function = line.contains('(') && !line.contains('=') && !line.contains(';');
        assert!(function || line.contains("const "), "fle_cursor.c must be stateless (mutable static: {line})");
    }
    // `long` is not portable (32 bits on wasm32 and on Windows, 64 on Linux): fixed-width integers only
    let b = code.as_bytes();
    let mut i = 0;
    while let Some(p) = code[i..].find("long") {
        let at = i + p;
        let before = at == 0 || !(b[at - 1].is_ascii_alphanumeric() || b[at - 1] == b'_');
        let after = at + 4 >= b.len() || !(b[at + 4].is_ascii_alphanumeric() || b[at + 4] == b'_');
        assert!(!(before && after), "fle_cursor.c uses `long` near byte {at}");
        i = at + 4;
    }
    // the identifier and the base are named, never a second copy of the number; fle_stop is the stop; nothing is written
    assert!(code.contains("FLE_CURSOR_IDC_ARROW") && code.contains("FLE_CURSOR_HANDLE_BASE") && code.contains("fle_stop("));
    // decision 0059: no guest memory is read or written by LoadCursorW. Decision 0098 added GetClipCursor to this file: it writes the four ints of the RECT
    // it is given (through fle_st, nothing else) and reads nothing
    assert!(!code.contains("fle_ld("), "no guest memory is read");
    assert_eq!(code.matches("fle_st(").count(), 4, "the only writes are the four ints of GetClipCursor's RECT (decision 0098)");
    // (the stop message prints the identifier as text, so only the comparisons and the base are checked)
    assert!(!code.contains("== 32512") && !code.contains("!= 32512") && !code.contains("0x7FCA"), "the numbers are in the header only");
}

#[test]
fn header_is_plain_c() {
    let h = strip_comments(&std::fs::read_to_string(src_dir().join("fle_cursor.h")).unwrap());
    for bad in ["asm", "__asm__", "<unistd.h>", "<sys/", "<pthread.h>", "<time.h>", "long "] {
        assert!(!h.contains(bad), "fle_cursor.h must not contain {bad}");
    }
}

// ---- 5. The other handle tables do not take a cursor handle ----

#[test]
fn a_cursor_handle_is_not_a_gdi_object_or_a_dc() {
    // decision 0059: its own table and region; the GDI table (0054) and the DC table (0053) stop on it as "not issued"
    for (name, body, needle) in [
        ("cursor-delobj", "uint64_t h = CUR(0, 32512); CALL(fle_imp_DeleteObject, h);", "fle: DeleteObject"),
        ("cursor-relDC", "uint64_t h = CUR(0, 32512); CALL(fle_imp_ReleaseDC, 0, h);", "fle: ReleaseDC"),
    ] {
        let o = run(name, body);
        assert_eq!(o.signal, Some(6), "{name}: {}", o.stderr);
        assert!(o.stderr.contains(needle), "{name}: {}", o.stderr);
        assert!(o.stderr.contains("not issued"), "{name}: {}", o.stderr);
    }
    // and the cursor table does not answer a GDI handle either
    let l = lines(
        "cursor-gdi-not-cursor",
        r#"
    uint64_t pen = CALL(fle_imp_GetStockObject, 7), icon = CALL(fle_imp_LoadIconW, 0, 32512);
    printf("%u %u %u\n", fle_cursor_idc_of(pen), fle_cursor_idc_of(icon), fle_cursor_idc_of(CUR(0, 32512)));
"#,
    );
    assert_eq!(l, ["0 0 32512"]);
}

// ---- 6. Generator: the thunk calls the runtime ----

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

/// A PE that imports `dll`!`api` (slot 0x2200). Entry 0x1000: `xor ecx,ecx; mov edx,0x7f00; call 0x1040; ret`; 0x1040 is the thunk.
fn build(dll: &str, api: &str) -> Vec<u8> {
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

    // Descriptor 1: the DLL (OFT 0x2100, Name 0x2300, FirstThunk 0x2200). Descriptor 2 is the zero terminator.
    let d = data_at(DATA_RVA);
    w32(&mut b, d, 0x2100);
    w32(&mut b, d + 12, 0x2300);
    w32(&mut b, d + 16, 0x2200);
    let mut nm = dll.as_bytes().to_vec();
    nm.push(0);
    put(&mut b, 0x2300, &nm);
    let mut hn = vec![0u8, 0u8];
    hn.extend_from_slice(api.as_bytes());
    hn.push(0);
    put(&mut b, 0x2400, &hn);
    for (rva, v) in [(0x2100, 0x2400u64), (0x2200, 0x2400)] {
        w64(&mut b, data_at(rva), v);
    }

    let t = TEXT_RAW;
    let code: &[(u32, Vec<u8>)] = &[
        (0x1000, vec![0x33, 0xc9, 0xba, 0x00, 0x7f, 0x00, 0x00]),
        (0x1007, [&[0xe8][..], &rel(0x100c, 0x1040)].concat()),
        (0x100c, vec![0xc3]),
        (0x1040, [&[0xff, 0x25][..], &rel(0x1046, 0x2200)].concat()),
    ];
    for (rva, bytes) in code {
        let o = t + (rva - TEXT_RVA) as usize;
        b[o..o + bytes.len()].copy_from_slice(bytes);
    }
    b
}

#[test]
fn the_thunk_calls_the_runtime() {
    let img = build("user32.dll", "LoadCursorW");
    let mut out = Vec::new();
    let st = generate(&img, &mut out).unwrap();
    let c = String::from_utf8(out).unwrap();
    assert_eq!(st.handled["jmp_imp:LoadCursorW"], 1);
    assert!(c.contains("fle_imp_LoadCursorW(); return;"));
    let va = IB + 0x1040;
    assert!(!c.contains(&format!("L_{va:x}: {{ /* {va:#x} Jmp */ fle_unsupported")), "{va:#x} still unsupported");
    assert_eq!(st.unsupported_total, 0, "{:?}", st.unsupported);
}

#[test]
fn the_dll_name_and_the_api_name_are_part_of_the_match() {
    // the same API name from another DLL, and the neighbouring cursor and image APIs of user32, are not this API: they stay unsupported
    for (dll, api) in [
        // ("user32.dll", "LoadImageW") was here until decision 0061 answered it (tests/loadimage.rs has its own DLL-name test)
        ("other.dll", "LoadCursorW"), ("user32.dll", "LoadCursorA"), ("user32.dll", "LoadCursorFromFileW"), ("user32.dll", "LoadImageA"),
        ("user32.dll", "ClipCursor"),   // old: ("user32.dll", "GetCursorInfo") was here too; decision 0126 (wall 77) made it a row: its thunk is no longer `fle_unsupported`   // ("user32.dll", "SetCursor") and "DestroyCursor" were here until decision 0111 answered them
    ] {
        let img = build(dll, api);
        let mut out = Vec::new();
        let st = generate(&img, &mut out).unwrap();
        let c = String::from_utf8(out).unwrap();
        assert!(!st.handled.contains_key("jmp_imp:LoadCursorW"), "{dll}!{api}");
        assert!(!c.contains("fle_imp_LoadCursorW"), "{dll}!{api}");
        let va = IB + 0x1040;
        assert!(c.contains(&format!("L_{va:x}: {{ /* {va:#x} Jmp */ fle_unsupported(UINT64_C({va:#x})); }}")), "{dll}!{api}");
    }
}

// ---- 7. The bytes of the real DLL that the investigation rests on ----

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

/// The section table: (name, virtual address, virtual size, raw offset, raw size), read here, not through fe-pe.
fn sections(dll: &[u8]) -> Vec<(String, u32, u32, u32, u32)> {
    let lf = le32(dll, 0x3c) as usize;
    let nsec = le16(dll, lf + 6) as usize;
    let osz = le16(dll, lf + 20) as usize;
    (0..nsec)
        .map(|i| {
            let o = lf + 24 + osz + 40 * i;
            let name = String::from_utf8_lossy(&dll[o..o + 8]).trim_end_matches('\0').to_string();
            (name, le32(dll, o + 12), le32(dll, o + 8), le32(dll, o + 20), le32(dll, o + 16))
        })
        .collect()
}

/// The file offset of `rva`.
fn rva_off(dll: &[u8], rva: u32) -> usize {
    for (_, sva, vs, rp, rs) in sections(dll) {
        if rva >= sva && rva < sva + vs.max(rs) {
            return rp as usize + (rva - sva) as usize;
        }
    }
    panic!("rva {rva:#x} is in no section");
}

/// The file bytes at virtual address `va` (ImageBase 0x400000).
fn bytes_at(dll: &[u8], va: u64, n: usize) -> Vec<u8> {
    let off = rva_off(dll, (va - 0x400000) as u32);
    dll[off..off + n].to_vec()
}

/// Every import of the DLL as (dll name, function name, IAT slot VA). Ordinal imports have the name "#<ordinal>".
fn imports(dll: &[u8]) -> Vec<(String, String, u64)> {
    let lf = le32(dll, 0x3c) as usize;
    let dir_rva = le32(dll, lf + 24 + 112 + 8);
    let cstr = |rva: u32| -> String {
        let o = rva_off(dll, rva);
        let e = dll[o..].iter().position(|&c| c == 0).unwrap();
        String::from_utf8_lossy(&dll[o..o + e]).to_string()
    };
    let mut v = Vec::new();
    let mut d = rva_off(dll, dir_rva);
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

/// The virtual addresses of every `call rel32` (e8) in .text whose target is `target` (a byte scan of the whole section: the
/// objdump of the investigation stops at 0x11fb4af, the section goes to 0x13768a0).
fn text_calls(dll: &[u8], target: u64) -> Vec<u64> {
    let (_, sva, vs, rp, rs) = sections(dll).into_iter().find(|s| s.0 == ".text").unwrap();
    let n = vs.min(rs) as usize;
    let t = &dll[rp as usize..rp as usize + n];
    let va0 = 0x400000u64 + sva as u64;
    let mut v = Vec::new();
    for i in 0..n.saturating_sub(5) {
        if t[i] == 0xe8 {
            let r = i32::from_le_bytes(t[i + 1..i + 5].try_into().unwrap()) as i64;
            if (va0 + i as u64 + 5).wrapping_add(r as u64) == target {
                v.push(va0 + i as u64);
            }
        }
    }
    v
}

/// The 7 call sites of the thunk 0x4250e0: (call site, hInstance static NULL?)
const SITES: [(u64, bool); 7] =
    [(0x6117b5, true), (0x865a01, true), (0x865ae6, true), (0x86618f, true), (0x9f2927, false), (0x9f2959, false), (0xa0e3db, false)];

#[test]
fn the_thunk_jumps_to_the_slot_which_is_user32_loadcursorw() {
    let Some(dll) = real_dll() else { return };
    let imps = imports(&dll);
    let slot = 0x1970e70u64;
    let named: Vec<_> = imps.iter().filter(|(_, _, s)| *s == slot).collect();
    assert_eq!(named.len(), 1);
    assert_eq!((named[0].0.to_lowercase(), named[0].1.as_str()), ("user32.dll".to_string(), "LoadCursorW"));
    let lc: Vec<_> = imps.iter().filter(|(_, n, _)| n.contains("LoadCursor")).collect();
    assert_eq!(lc.len(), 1, "the guest imports no LoadCursorA / LoadCursorFromFile*: {lc:?}");
    let thunk = 0x4250e0u64;
    let t = bytes_at(&dll, thunk, 6);
    assert_eq!(&t[..2], [0xff, 0x25], "jmp [rip+disp32]");
    assert_eq!((thunk as i64 + 6 + i32::from_le_bytes(t[2..6].try_into().unwrap()) as i64) as u64, slot);
    // no other thunk jumps to the slot (every `jmp [rip+disp]` of .text that lands on it is this one)
    let (_, sva, vs, rp, rs) = sections(&dll).into_iter().find(|s| s.0 == ".text").unwrap();
    let n = vs.min(rs) as usize;
    let text = &dll[rp as usize..rp as usize + n];
    let va0 = 0x400000u64 + sva as u64;
    let mut jmps = Vec::new();
    for i in 0..n - 6 {
        if text[i] == 0xff && text[i + 1] == 0x25 {
            let r = i32::from_le_bytes(text[i + 2..i + 6].try_into().unwrap()) as i64;
            if (va0 + i as u64 + 6).wrapping_add(r as u64) == slot {
                jmps.push(va0 + i as u64);
            }
        }
    }
    assert_eq!(jmps, [thunk]);
}

#[test]
fn nothing_else_in_the_file_points_at_the_thunk_or_the_slot() {
    let Some(dll) = real_dll() else { return };
    // no 4- or 8-byte absolute VA (or the 4-byte RVA) of the thunk or of the slot is in the file: no table of function pointers
    // holds them, so the direct call sites are all the references (an indirect call through a computed address is not seen)
    for va in [0x4250e0u64, 0x1970e70] {
        for pat in [(va as u32).to_le_bytes().to_vec(), va.to_le_bytes().to_vec(), ((va - 0x400000) as u32).to_le_bytes().to_vec()] {
            assert!(!dll.windows(pat.len()).any(|w| w == pat.as_slice()), "{va:#x}: {pat:02x?}");
        }
    }
}

#[test]
fn the_guest_has_exactly_seven_call_sites_and_no_jump_to_the_thunk() {
    let Some(dll) = real_dll() else { return };
    let found = text_calls(&dll, 0x4250e0);
    let want: Vec<u64> = SITES.iter().map(|s| s.0).collect();
    assert_eq!(found, want);
}

#[test]
fn four_sites_pass_null_and_idc_arrow_and_three_pass_the_module_and_a_name() {
    let Some(dll) = real_dll() else { return };
    let mut statics = 0;
    for (site, is_static) in SITES {
        if is_static {
            // `xor ecx,ecx; mov edx,0x7f00` immediately before the call: hInstance NULL, lpCursorName MAKEINTRESOURCE(0x7f00 = 32512)
            assert_eq!(bytes_at(&dll, site - 7, 7), [0x33, 0xc9, 0xba, 0x00, 0x7f, 0x00, 0x00], "{site:#x}");
            statics += 1;
        } else {
            // `mov rcx,[rip+disp32]; mov rdx,rax; call` : hInstance = the global 0x15ec298, lpCursorName = the pointer a call returned
            let b = bytes_at(&dll, site - 10, 10);
            assert_eq!(&b[..3], [0x48, 0x8b, 0x0d], "{site:#x}: mov rcx,[rip+disp32]");
            let disp = i32::from_le_bytes(b[3..7].try_into().unwrap()) as i64;
            assert_eq!((site as i64 - 10 + 7 + disp) as u64, 0x15ec298, "{site:#x}: the global is 0x15ec298");
            assert_eq!(&b[7..], [0x48, 0x89, 0xc2], "{site:#x}: mov rdx,rax");
        }
    }
    assert_eq!(statics, 4);
    assert_eq!(ARROW, 0x7f00);
    // the global is stored from a register once (the only `mov [0x15ec298],reg` of the objdump), by the DllMain-like f_41d8f0 from
    // its third argument (r8 = the module handle, the entry's rcx); other writers (immediate stores) were not searched
    assert_eq!(bytes_at(&dll, 0x41d919, 7), [0x48, 0x89, 0x3d, 0x78, 0xe9, 0x1c, 0x01], "mov [0x15ec298],rdi");
    assert_eq!(bytes_at(&dll, 0x41d8ff, 3), [0x4c, 0x89, 0xc7], "mov rdi,r8");
}

fn wide(dll: &[u8], va: u64) -> String {
    let off = rva_off(dll, (va - 0x400000) as u32);
    let u: Vec<u16> = dll[off..].chunks(2).map(|c| u16::from_le_bytes([c[0], c[1]])).take_while(|&c| c != 0).collect();
    String::from_utf16(&u).unwrap()
}

#[test]
fn the_three_dynamic_names_are_the_vt_cursors_of_the_guests_own_resources() {
    let Some(dll) = real_dll() else { return };
    // the strings passed at 0x9f2927 / 0x9f2959 / 0xa0e3db (lea rcx,[rip+..] before the call that returns the pointer)
    assert_eq!(wide(&dll, 0x9f2b20), "VT_HEADERSPLIT");
    assert_eq!(wide(&dll, 0x9f2b4c), "VT_VERTSPLIT");
    assert_eq!(wide(&dll, 0xa0e578), "VT_MOVEN");
    assert_eq!(wide(&dll, 0xa0e598), "VT_MOVES");
    // these are names, so the runtime stops on them as pointers (and on the non-NULL hInstance first)
    let e = call(0x400000, 0x7FF7_0000_1000).unwrap_err();
    assert!(e.contains("hInstance is not NULL"), "{e}");
}

#[test]
fn the_result_is_stored_compared_and_passed_on_the_way_the_investigation_reads_it() {
    let Some(dll) = real_dll() else { return };
    // f_8659f0 (the stop of the harness): call LoadCursorW(NULL, 0x7f00) at 0x865a01, then `mov [rbx+0x154],rax`
    assert_eq!(bytes_at(&dll, 0x865a06, 7), [0x48, 0x89, 0x83, 0x54, 0x01, 0x00, 0x00]);
    // f_865a80: the new result is compared with [rbx+0x154] (0x865aeb: cmp rax,[rbx+0x154]; je 0x865b11): the same IDC_ARROW must
    // give the same handle, otherwise the guest takes the DestroyCursor path (0x865b0c) on a shared cursor
    assert_eq!(bytes_at(&dll, 0x865aeb, 9), [0x48, 0x3b, 0x83, 0x54, 0x01, 0x00, 0x00, 0x74, 0x1d]);
    // f_866170: with index 0 and no cursor given, [rbx+0x154] = LoadCursorW(NULL, IDC_ARROW) again (0x866194)
    assert_eq!(bytes_at(&dll, 0x866194, 7), [0x48, 0x89, 0x83, 0x54, 0x01, 0x00, 0x00]);
    // f_611670: the result goes into the field at +0x58 of a structure whose +0x30 is 0xb (CS_VREDRAW | CS_HREDRAW | CS_DBLCLKS)
    // and whose +0x48 is the module handle: WNDCLASS.hCursor (WNDCLASSW: style 0, hInstance 24, hCursor 40, from +0x30)
    assert_eq!(bytes_at(&dll, 0x6117a7, 7), [0xc7, 0x40, 0x30, 0x0b, 0x00, 0x00, 0x00]);
    assert_eq!(bytes_at(&dll, 0x6117ba, 8), [0x48, 0x8b, 0x4d, 0x30, 0x48, 0x89, 0x41, 0x58]);
    assert_eq!(0x30 + 40, 0x58);
    assert_eq!(0x30 + 24, 0x48);
}

#[test]
fn the_next_stop_is_loadimagew_with_shared_standard_cursors_and_the_guests_resources() {
    let Some(dll) = real_dll() else { return };
    let imps = imports(&dll);
    let li: Vec<_> = imps.iter().filter(|(_, n, _)| n == "LoadImageW").collect();
    assert_eq!(li.len(), 1);
    assert_eq!(li[0].2, 0x1971090);
    assert_eq!(text_calls(&dll, 0x425100), [0x865a55], "one call site of the thunk 0x425100");
    // LoadImageW(hInst, name, IMAGE_CURSOR = 2, cx = 32, cy = 32, LR_SHARED = 0x8000): r8d = 2, r9d = 0x20, [rsp+0x20] = 0x20, [rsp+0x28] = 0x8000
    assert_eq!(
        bytes_at(&dll, 0x865a39, 28),
        [0x41, 0xb8, 2, 0, 0, 0, 0x41, 0xb9, 0x20, 0, 0, 0, 0xc7, 0x44, 0x24, 0x20, 0x20, 0, 0, 0, 0xc7, 0x44, 0x24, 0x28, 0x00, 0x80, 0, 0]
    );
    // the 21 identifiers of the loop (esi = -22 ..= -2; the table at 0x1409038, 8 bytes each; the Learn numbers written above):
    // hInstance is NULL for 14 of them (all standard IDC_ numbers of About Cursors) and the module for 7 (-21 and -17 ..= -12)
    let want: [u64; 21] = [
        32646, 32649, 32651, 32650, 32648, 32762, 32763, 32764, 32765, 32767, 32766, 32514, 32516, 32644, 32642, 32645, 32643, 32646, 32513, 32515, 32512,
    ];
    let tab = bytes_at(&dll, 0x1409038, 21 * 8);
    for (i, w) in want.iter().enumerate() {
        assert_eq!(u64::from_le_bytes(tab[8 * i..8 * i + 8].try_into().unwrap()), *w, "entry {i}");
    }
    let module = |esi: i32| (-17..=-12).contains(&esi) || esi == -21;
    let mut null_ids = Vec::new();
    for (i, w) in want.iter().enumerate() {
        let esi = -22 + i as i32;
        if !module(esi) {
            assert!(LEARN_IDC.iter().any(|(n, _)| n == w), "{w}: a standard IDC_ number");
            null_ids.push(*w);
        }
    }
    assert_eq!(null_ids.len(), 14);
    assert_eq!((0..21).filter(|i| module(-22 + *i as i32)).count(), 7);
    // the module's ones: six of them are 32762..=32767, RT_GROUP_CURSOR ids of the DLL's own resources (the directory lists the
    // names ADDPOINT .. VT_VERTSPLIT and the numbers 32761 ..= 32767); the seventh, esi = -21 with 32649 (IDC_HAND) and the
    // module, is not an id of that directory (read from the PE resource directory with pefile; whether the guest meant it, not known)
    let has = |s: &str| {
        let pat: Vec<u8> = s.encode_utf16().flat_map(|c| c.to_le_bytes()).collect();
        dll.windows(pat.len()).any(|w| w == pat.as_slice())
    };
    for n in ["VT_HEADERSPLIT", "VT_VERTSPLIT", "VT_MOVEN", "VT_MOVES"] {
        assert!(has(n), "{n}");
    }
}
