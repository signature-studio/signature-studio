//! Tests for the virtual system metrics of runtime/fle_sysmetrics.c: user32!GetSystemMetrics
//! (docs/decisions/0057-virtual-system-metrics.md; sources: docs/investigations/getsystemmetrics-caller.md).
//!
//! Expected values are written here from the decision (the index table of the Microsoft Learn page GetSystemMetrics, the
//! rules of the page, and the choices of the decision), not read back from the C code. A value the page does not give (the
//! mouse and the wheel present, the screen size, the number of monitors) is marked "choice" and is the only thing such a
//! test pins down. The host is Linux: nothing is compared with a real Windows; no display, mouse, DPI setting, theme, clock,
//! thread or process is read.
//!
//! The C sources of fle_sysmetrics.c / fle_sysmetrics.h come from runtime/ when fle_sysmetrics.c is there, otherwise from
//! out-sm/work/ (development). FLE_SM_SRC_DIR (a directory with fle_sysmetrics.c and fle_sysmetrics.h) overrides both: the
//! mutation runs of the run record use it. Files that already existed always come from runtime/.

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
    if let Ok(d) = std::env::var("FLE_SM_SRC_DIR") {
        return PathBuf::from(d);
    }
    if runtime_dir().join("fle_sysmetrics.c").exists() {
        runtime_dir()
    } else {
        manifest().join("../../out-sm/work")
    }
}
fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-sm/test");
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

/// fle_sysmetrics.c (warnings are errors) and the existing files it links with: fle_stop (fle_env.c), GetLastError and
/// fle_set_last_error (fle_imports.c), and fle_gdc.c (the virtual display it must agree with).
fn objs() -> &'static Vec<PathBuf> {
    static O: OnceLock<Vec<PathBuf>> = OnceLock::new();
    O.get_or_init(|| {
        let dir = work_dir().join(etxt_common::per_process("obj-sm"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut v = Vec::new();
        for (name, ours) in [("fle_sysmetrics.c", true), ("fle_gdc.c", false), ("fle_imports.c", false), ("fle_env.c", false)] {
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
#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_gdc.h"
#include "fle_sysmetrics.h"

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
#define SM(idx) ((int32_t)(uint32_t)CALL(fle_imp_GetSystemMetrics, (idx)))
#define GETDC() CALL(fle_imp_GetDC, 0)
#define CAPS(dc, idx) ((int32_t)(uint32_t)CALL(fle_imp_GetDeviceCaps, (dc), (idx)))
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

/// One program for every index of a list: argv[1] is the index (any u64, the whole rcx), the output is "value <int>".
fn cli() -> &'static PathBuf {
    static E: OnceLock<PathBuf> = OnceLock::new();
    E.get_or_init(|| {
        compile_main(
            "sm-cli",
            "",
            r#"
    uint64_t idx = strtoull(argv[1], 0, 0);
    int32_t v = SM(idx);
    printf("value %d\n", v);
"#,
        )
    })
}

/// The result of GetSystemMetrics(idx): Ok(value), or Err(stderr) when the run stopped (SIGABRT, "fle: GetSystemMetrics").
fn ask(idx: u64) -> Result<i32, String> {
    let o = exec(cli(), &[&format!("{idx:#x}")]);
    if o.ok {
        let v = o.stdout.strip_prefix("value ").unwrap_or_else(|| panic!("odd output {}", o.stdout)).trim().parse().unwrap();
        return Ok(v);
    }
    assert_eq!(o.signal, Some(6), "index {idx:#x}: not SIGABRT: {}", o.stderr);
    assert!(o.stderr.contains("fle: GetSystemMetrics"), "index {idx:#x}: no 'fle: GetSystemMetrics' message: {}", o.stderr);
    assert!(o.stdout.is_empty(), "index {idx:#x}: output before the stop: {}", o.stdout);
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

/// The SM_* names and numbers of the Learn page GetSystemMetrics (fetched 2026-10-21): OFFICIAL, written here from the page.
const LEARN: &[(u64, &str)] = &[
    (0, "SM_CXSCREEN"), (1, "SM_CYSCREEN"), (2, "SM_CXVSCROLL"), (3, "SM_CYHSCROLL"), (4, "SM_CYCAPTION"), (5, "SM_CXBORDER"),
    (6, "SM_CYBORDER"), (7, "SM_CXFIXEDFRAME"), (8, "SM_CYFIXEDFRAME"), (9, "SM_CYVTHUMB"), (10, "SM_CXHTHUMB"), (11, "SM_CXICON"),
    (12, "SM_CYICON"), (13, "SM_CXCURSOR"), (14, "SM_CYCURSOR"), (15, "SM_CYMENU"), (16, "SM_CXFULLSCREEN"), (17, "SM_CYFULLSCREEN"),
    (18, "SM_CYKANJIWINDOW"), (19, "SM_MOUSEPRESENT"), (20, "SM_CYVSCROLL"), (21, "SM_CXHSCROLL"), (22, "SM_DEBUG"), (23, "SM_SWAPBUTTON"),
    (28, "SM_CXMIN"), (29, "SM_CYMIN"), (30, "SM_CXSIZE"), (31, "SM_CYSIZE"), (32, "SM_CXSIZEFRAME"), (33, "SM_CYSIZEFRAME"),
    (34, "SM_CXMINTRACK"), (35, "SM_CYMINTRACK"), (36, "SM_CXDOUBLECLK"), (37, "SM_CYDOUBLECLK"), (38, "SM_CXICONSPACING"),
    (39, "SM_CYICONSPACING"), (40, "SM_MENUDROPALIGNMENT"), (41, "SM_PENWINDOWS"), (42, "SM_DBCSENABLED"), (43, "SM_CMOUSEBUTTONS"),
    (44, "SM_SECURE"), (45, "SM_CXEDGE"), (46, "SM_CYEDGE"), (47, "SM_CXMINSPACING"), (48, "SM_CYMINSPACING"), (49, "SM_CXSMICON"),
    (50, "SM_CYSMICON"), (51, "SM_CYSMCAPTION"), (52, "SM_CXSMSIZE"), (53, "SM_CYSMSIZE"), (54, "SM_CXMENUSIZE"), (55, "SM_CYMENUSIZE"),
    (56, "SM_ARRANGE"), (57, "SM_CXMINIMIZED"), (58, "SM_CYMINIMIZED"), (59, "SM_CXMAXTRACK"), (60, "SM_CYMAXTRACK"),
    (61, "SM_CXMAXIMIZED"), (62, "SM_CYMAXIMIZED"), (63, "SM_NETWORK"), (67, "SM_CLEANBOOT"), (68, "SM_CXDRAG"), (69, "SM_CYDRAG"),
    (70, "SM_SHOWSOUNDS"), (71, "SM_CXMENUCHECK"), (72, "SM_CYMENUCHECK"), (73, "SM_SLOWMACHINE"), (74, "SM_MIDEASTENABLED"),
    (75, "SM_MOUSEWHEELPRESENT"), (76, "SM_XVIRTUALSCREEN"), (77, "SM_YVIRTUALSCREEN"), (78, "SM_CXVIRTUALSCREEN"),
    (79, "SM_CYVIRTUALSCREEN"), (80, "SM_CMONITORS"), (81, "SM_SAMEDISPLAYFORMAT"), (82, "SM_IMMENABLED"), (83, "SM_CXFOCUSBORDER"),
    (84, "SM_CYFOCUSBORDER"), (86, "SM_TABLETPC"), (87, "SM_MEDIACENTER"), (88, "SM_STARTER"), (89, "SM_SERVERR2"),
    (91, "SM_MOUSEHORIZONTALWHEELPRESENT"), (92, "SM_CXPADDEDBORDER"),
];
/// The page's higher entries (kept apart so that the array above stays a plain run): 94, 95 and the 0x1000.. group.
const LEARN_HIGH: [(u64, &str); 7] =
    [(94, "SM_DIGITIZER"), (95, "SM_MAXIMUMTOUCHES"), (0x1000, "SM_REMOTESESSION"), (0x2000, "SM_SHUTTINGDOWN"), (0x2001, "SM_REMOTECONTROL"), (0x2003, "SM_CONVERTIBLESLATEMODE"), (0x2004, "SM_SYSTEMDOCKED")];

fn learn_name(i: u64) -> Option<&'static str> {
    LEARN.iter().chain(LEARN_HIGH.iter()).find(|(n, _)| *n == i).map(|(_, s)| *s)
}

/// Answered: (index, value). 1920 and 1080 are the virtual display of decision 0053.
/// The icon sizes (11, 12, 49, 50) are the 96 DPI values of the Learn page "About Icons" (official); the mouse and the wheel are
/// choices.
/// (94, 0) is the answer of decision 0098 (PROVISIONAL, SM_DIGITIZER: no digitizer); (13, 32) and (14, 32) are those of decision 0106 (PROVISIONAL, Learn "About
/// Cursors", Cursor Sizes at < 144 DPI); (86, 0), (95, 0), (0x2003, 1) and (0x2004, 0) are those of decision 0107 (PROVISIONAL, the four indexes that the
/// same guest function asks next, see tests/wall_r2.rs); the other fourteen are those of decision 0057.
const ANSWERED: [(u64, i32); 21] = [
    (0, 1920), (1, 1080), (11, 32), (12, 32), (13, 32), (14, 32), (19, 1), (49, 16), (50, 16), (75, 1), (76, 0), (77, 0), (78, 1920), (79, 1080), (80, 1), (89, 0), (94, 0),
    (86, 0), (95, 0), (0x2003, 1), (0x2004, 0),
];
/// Used by the guest (static immediates of the 54 call sites, and the scroll bar table) and not decided: stop by name.
/// 94 (SM_DIGITIZER) was in this list until decision 0098 answered it (the census wall at unit 276); 13 and 14 until decision 0106 answered them (ILTools'
/// own initialisation); 86, 95, 0x2003 and 0x2004 until decision 0107 answered them (the next walls of the same unit as 94).
const UNDECIDED: [u64; 15] = [2, 3, 6, 7, 8, 10, 15, 20, 21, 32, 45, 46, 61, 68, 69];

// ---- 1. Table, header, constants ----

#[test]
fn the_row_is_in_the_table_and_declared_in_the_header() {
    let table = std::fs::read_to_string(manifest().join("src/imports_table.rs")).unwrap();
    let h = std::fs::read_to_string(src_dir().join("fle_sysmetrics.h")).unwrap();
    let row = "(\"user32.dll\", \"GetSystemMetrics\", \"fle_imp_GetSystemMetrics\", \"jmp_imp:GetSystemMetrics\")";
    assert_eq!(table.matches(row).count(), 1, "exactly one row in imports_table.rs");
    assert!(h.contains("void fle_imp_GetSystemMetrics(void)"));
    // SystemParametersInfoW was in this list until decision 0058 (runtime/fle_spi.c, tests/spi.rs) answered it: the guest's wheel
    // code (f_620880) calls it with uiAction 0x68 right after GetSystemMetrics(75), so this decision no longer holds it back
    // EnumDisplayMonitors was in this list until decision 0065 (runtime/fle_monitor.c, tests/monitor.rs) answered it: the guest's
    // monitor enumeration is the next stop after the keyboard layout and needs this decision's screen values, not the other way round.
    // decision 0126 (wall 100): old: "MonitorFromPoint" was in this list; it is a row now (the one monitor of decision 0065, runtime/fle_monitor.c; tests/wall_fr11.rs)
    for n in ["GetSystemMetricsForDpi", "GetDpiForWindow"] {
        assert!(!table.contains(&format!("\"{n}\"")), "{n} must not be answered by this decision");
    }
}

#[test]
fn the_index_numbers_are_the_learn_page_numbers() {
    // official: every number is a row of the Learn table (written above from the page, not read back from the header)
    for (n, v) in [
        ("CXSCREEN", 0u64),
        ("CYSCREEN", 1),
        ("CXICON", 11),
        ("CYICON", 12),
        ("CXCURSOR", 13),
        ("CYCURSOR", 14),
        ("MOUSEPRESENT", 19),
        ("CXSMICON", 49),
        ("CYSMICON", 50),
        ("MOUSEWHEELPRESENT", 75),
        ("XVIRTUALSCREEN", 76),
        ("YVIRTUALSCREEN", 77),
        ("CXVIRTUALSCREEN", 78),
        ("CYVIRTUALSCREEN", 79),
        ("CMONITORS", 80),
        ("SERVERR2", 89),
    ] {
        assert_eq!(def_const("fle_sysmetrics.h", &format!("FLE_SM_IDX_{n}")), v, "{n}");
        assert_eq!(learn_name(v), Some(format!("SM_{n}").as_str()), "{n}: the Learn table row");
    }
}

#[test]
fn the_decided_values() {
    // choices of decision 0057 (the page gives none of them)
    assert_eq!(def_const("fle_sysmetrics.h", "FLE_SM_MOUSEPRESENT"), 1);
    assert_eq!(def_const("fle_sysmetrics.h", "FLE_SM_MOUSEWHEELPRESENT"), 1);
    assert_eq!(def_const("fle_sysmetrics.h", "FLE_SM_VIRTUALSCREEN_ORIGIN"), 0);
    assert_eq!(def_const("fle_sysmetrics.h", "FLE_SM_CMONITORS"), 1);
    assert_eq!(def_const("fle_sysmetrics.h", "FLE_SM_SERVERR2"), 0);
    // Learn "About Icons", system icon sizes at 96 DPI (official); 96 is the DPI of the virtual display (decision 0053)
    assert_eq!(def_const("fle_sysmetrics.h", "FLE_SM_CXICON"), 32);
    assert_eq!(def_const("fle_sysmetrics.h", "FLE_SM_CYICON"), 32);
    assert_eq!(def_const("fle_sysmetrics.h", "FLE_SM_CXSMICON"), 16);
    assert_eq!(def_const("fle_sysmetrics.h", "FLE_SM_CYSMICON"), 16);
    // Learn "About Cursors", Cursor Sizes: 32x32 below 144 DPI at the default cursor size (decision 0106, PROVISIONAL)
    assert_eq!(def_const("fle_sysmetrics.h", "FLE_SM_CXCURSOR"), 32);
    assert_eq!(def_const("fle_sysmetrics.h", "FLE_SM_CYCURSOR"), 32);
    assert_eq!(def_const("fle_gdc.h", "FLE_GDC_LOGPIXELSX"), 96);
    assert_eq!(def_const("fle_gdc.h", "FLE_GDC_LOGPIXELSY"), 96);
    // the virtual display of decision 0053
    assert_eq!(def_const("fle_gdc.h", "FLE_GDC_HORZRES"), 1920);
    assert_eq!(def_const("fle_gdc.h", "FLE_GDC_VERTRES"), 1080);
}

#[test]
fn the_learn_table_in_this_file_has_no_repeated_number() {
    let mut all: Vec<u64> = LEARN.iter().chain(LEARN_HIGH.iter()).map(|(n, _)| *n).collect();
    all.sort();
    let n = all.len();
    all.dedup();
    assert_eq!(all.len(), n);
}

// ---- 2. The answers ----

#[test]
fn the_twenty_one_answers() {
    for (idx, v) in ANSWERED {
        assert_eq!(ask(idx), Ok(v), "{} ({idx})", learn_name(idx).unwrap());
    }
}

#[test]
fn the_screen_agrees_with_the_virtual_display_of_decision_0053() {
    // Learn: SM_CXSCREEN / SM_CYSCREEN are "the same value obtained by calling GetDeviceCaps(hdcPrimaryMonitor, HORZRES / VERTRES)";
    // the virtual screen of one monitor at the origin is the screen
    let l = lines(
        "sm-gdc",
        r#"
    uint64_t dc = GETDC();
    printf("%d %d\n", SM(0) == CAPS(dc, 8), SM(1) == CAPS(dc, 10));
    printf("%d %d\n", SM(78) == CAPS(dc, 8), SM(79) == CAPS(dc, 10));
    printf("%d %d %d\n", SM(76), SM(77), SM(80));
"#,
    );
    assert_eq!(l, ["1 1", "1 1", "0 0 1"]);
}

#[test]
fn only_the_low_dword_of_the_index_counts() {
    // GetSystemMetrics(int): a 32-bit argument; the guest may leave anything in the upper half of rcx
    assert_eq!(ask(0xAAAA_AAAA_0000_0013), Ok(1));
    assert_eq!(ask(0xFFFF_FFFF_0000_0001), Ok(1080));
    assert_eq!(ask(0x1_0000_0000), Ok(1920));
    assert!(ask(0x0000_0001_0000_0002).is_err(), "low dword 2 (SM_CXVSCROLL) is not decided");
}

#[test]
fn it_is_stateless_leaves_the_last_error_alone_and_keeps_the_return_address() {
    // Learn: on failure the return value is 0 and GetLastError gives no extended error information: the last error is not touched
    let l = lines(
        "sm-state",
        r#"
    int a, b;
    SETLE(0x1234);
    a = SM(0); b = SM(0);
    printf("same %d\n", a == b);
    printf("last error %x\n", LE());
    SM(19);
    printf("return address %llx\n", U(stk[0]));
    SETLE(0);
    SM(75);
    printf("last error %x\n", LE());
"#,
    );
    assert_eq!(l, ["same 1", "last error 1234", "return address dead0000", "last error 0"]);
}

// ---- 3. Stops ----

#[test]
fn indexes_the_guest_uses_without_a_decided_value_stop_by_name() {
    for idx in UNDECIDED {
        let name = learn_name(idx).unwrap_or_else(|| panic!("{idx} is not in the Learn table"));
        let e = ask(idx).expect_err(&format!("{name} must stop"));
        assert!(e.contains(&format!("index {idx} ({name})")), "{name}: {e}");
        assert!(e.contains("no value is decided"), "{name}: {e}");
    }
}

#[test]
fn every_other_index_stops_by_number() {
    // 4 SM_CYCAPTION, 5 SM_CXBORDER, 9, 16, 17, 18, 22, 23, 28..31, 33..44, 47, 48, 51..60, 62, 63, 67, 70..74, 81..88, 91, 92,
    // 0x1000 SM_REMOTESESSION, 0x2000 SM_SHUTTINGDOWN, 0x2001 SM_REMOTECONTROL, and numbers that are no index of the page.
    let mut idxs: Vec<u64> = vec![4, 5, 9, 16, 17, 18, 22, 23, 28, 29, 30, 31, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 47, 48, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 62, 63, 67, 70, 71, 72, 73, 74, 81, 82, 83, 84, 87, 88, 91, 92];
    idxs.extend([0x1000, 0x2000, 0x2001, 24, 25, 26, 27, 64, 65, 66, 85, 90, 93, 96, 97, 0x2002, 0x2005, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFFF]);
    for idx in idxs {
        assert!(!UNDECIDED.contains(&idx) && !ANSWERED.iter().any(|(i, _)| *i == idx), "{idx} is in a list of this test file");
        let e = ask(idx).expect_err(&format!("index {idx:#x} must stop"));
        assert!(e.contains(&format!("index {} is not answered", idx as u32)), "{idx:#x}: {e}");
    }
}

#[test]
fn the_environment_state_indexes_are_not_answered_because_the_guest_does_not_ask() {
    // SM_REMOTESESSION, SM_SHUTTINGDOWN, SM_REMOTECONTROL, SM_CONVERTIBLESLATEMODE-like state questions: only the ones the
    // guest's 54 call sites pass are answered (decision 0107: 0x2003, 0x2004) or in the UNDECIDED list; the three below are not passed and stop by number
    for (idx, name) in [(0x1000u64, "SM_REMOTESESSION"), (0x2000, "SM_SHUTTINGDOWN"), (0x2001, "SM_REMOTECONTROL")] {
        assert_eq!(learn_name(idx), Some(name));
        let e = ask(idx).expect_err(name);
        assert!(e.contains("is not answered"), "{name}: {e}");
    }
}

// ---- 4. Source checks ----

#[test]
fn source_uses_no_host_display_mouse_dpi_theme_window_clock_thread_process_or_file() {
    let code = strip_comments(&std::fs::read_to_string(src_dir().join("fle_sysmetrics.c")).unwrap());
    for bad in [
        "getpid", "getppid", "gettid", "getenv", "environ", "fopen", "open(", "stat(", "/proc", "/etc", "/home", "/dev", "/tmp", "exit(", "_exit",
        "clock_gettime", "gettimeofday", "time(", "clock(", "nanosleep", "usleep", "sleep(", "select(", "poll(", "<time.h>", "<sys/time.h>",
        "pthread", "<sys/", "<unistd.h>", "<signal.h>", "fork(", "exec", "syscall", "sched_", "futex", "asm", "__asm__", "uintptr_t", "abort(",
        "X11", "DISPLAY", "xrandr", "wayland", "fb0", "ioctl", "emscripten", "SDL", "screen.width", "devicePixelRatio", "input", "mouse0",
        "fle_host", "static int", "static uint64_t",
    ] {
        assert!(!code.contains(bad), "fle_sysmetrics.c must not use {bad}");
    }
    // `long` is not portable (32 bits on wasm32 and on Windows, 64 on Linux): fixed-width integers only
    let b = code.as_bytes();
    let mut i = 0;
    while let Some(p) = code[i..].find("long") {
        let at = i + p;
        let before = at == 0 || !(b[at - 1].is_ascii_alphanumeric() || b[at - 1] == b'_');
        let after = at + 4 >= b.len() || !(b[at + 4].is_ascii_alphanumeric() || b[at + 4] == b'_');
        assert!(!(before && after), "fle_sysmetrics.c uses `long` near byte {at}");
        i = at + 4;
    }
    // the screen is the virtual display by name, never a second copy of the numbers; fle_stop is the stop
    assert!(code.contains("FLE_GDC_HORZRES") && code.contains("FLE_GDC_VERTRES") && code.contains("fle_stop("));
    assert!(code.contains("#include \"fle_gdc.h\""));
    for n in ["1920", "1080"] {
        assert!(!code.contains(n), "fle_sysmetrics.c repeats {n}");
        let h = strip_comments(&std::fs::read_to_string(src_dir().join("fle_sysmetrics.h")).unwrap());
        assert!(!h.contains(n), "fle_sysmetrics.h repeats {n}");
    }
}

#[test]
fn header_is_plain_c() {
    let h = strip_comments(&std::fs::read_to_string(src_dir().join("fle_sysmetrics.h")).unwrap());
    for bad in ["asm", "__asm__", "<unistd.h>", "<sys/", "<pthread.h>", "<time.h>", "long "] {
        assert!(!h.contains(bad), "fle_sysmetrics.h must not contain {bad}");
    }
}

// ---- 5. Generator: the thunk calls the runtime ----

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

/// A PE that imports `dll`!GetSystemMetrics (slot 0x2200). Entry 0x1000: `mov ecx,0x13; call 0x1040; ret`; 0x1040 is the thunk.
fn build(dll: &str) -> Vec<u8> {
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
    put(&mut b, 0x2400, b"\0\0GetSystemMetrics\0");
    for (rva, v) in [(0x2100, 0x2400u64), (0x2200, 0x2400)] {
        w64(&mut b, data_at(rva), v);
    }

    let t = TEXT_RAW;
    let code: &[(u32, Vec<u8>)] = &[
        (0x1000, vec![0xb9, 0x13, 0x00, 0x00, 0x00]),
        (0x1005, [&[0xe8][..], &rel(0x100a, 0x1040)].concat()),
        (0x100a, vec![0xc3]),
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
    let img = build("user32.dll");
    let mut out = Vec::new();
    let st = generate(&img, &mut out).unwrap();
    let c = String::from_utf8(out).unwrap();
    assert_eq!(st.handled["jmp_imp:GetSystemMetrics"], 1);
    assert!(c.contains("fle_imp_GetSystemMetrics(); return;"));
    let va = IB + 0x1040;
    assert!(!c.contains(&format!("L_{va:x}: {{ /* {va:#x} Jmp */ fle_unsupported")), "{va:#x} still unsupported");
    assert_eq!(st.unsupported_total, 0, "{:?}", st.unsupported);
}

#[test]
fn the_dll_name_is_part_of_the_match() {
    // the same API name from another DLL is not this API: it stays unsupported
    let img = build("other.dll");
    let mut out = Vec::new();
    let st = generate(&img, &mut out).unwrap();
    let c = String::from_utf8(out).unwrap();
    assert!(!st.handled.contains_key("jmp_imp:GetSystemMetrics"));
    assert!(!c.contains("fle_imp_GetSystemMetrics"));
    let va = IB + 0x1040;
    assert!(c.contains(&format!("L_{va:x}: {{ /* {va:#x} Jmp */ fle_unsupported(UINT64_C({va:#x})); }}")));
}

// ---- 6. The bytes of the real DLL that the investigation rests on ----

fn real_dll() -> Option<Vec<u8>> {
    let p = std::env::var("FLENGINE_DLL").map(PathBuf::from).unwrap_or_else(|_| manifest().join("../../../binaries/win64/FLEngine_x64.dll"));
    std::fs::read(p).ok()
}

/// The file bytes at virtual address `va` (the section table is read here, not through fe-pe).
fn bytes_at(dll: &[u8], va: u64, n: usize) -> Vec<u8> {
    let le32 = |o: usize| u32::from_le_bytes(dll[o..o + 4].try_into().unwrap());
    let le16 = |o: usize| u16::from_le_bytes(dll[o..o + 2].try_into().unwrap());
    let lf = le32(0x3c) as usize;
    let nsec = le16(lf + 6) as usize;
    let osz = le16(lf + 20) as usize;
    let rva = (va - 0x400000) as u32;
    for i in 0..nsec {
        let o = lf + 24 + osz + 40 * i;
        let (vs, sva, rs, rp) = (le32(o + 8), le32(o + 12), le32(o + 16), le32(o + 20));
        if rva >= sva && rva < sva + vs.max(rs) {
            let off = rp as usize + (rva - sva) as usize;
            return dll[off..off + n].to_vec();
        }
    }
    panic!("va {va:#x} is in no section");
}

/// The 54 call sites of the thunk 0x424ee0 (objdump of the DLL), with the immediate of the `mov ecx,imm32` (or `xor ecx,ecx`)
/// before the call; None where the index is not an immediate (movzx ecx,di; mov ecx,[table+rcx*4]).
const SITES: [(u64, Option<u32>); 54] = [
    (0x454794, Some(0x59)), (0x5c55a9, Some(0xb)), (0x5c55bc, Some(0xc)), (0x5d0040, Some(0xc)), (0x5d0080, Some(0xc)),
    (0x61990c, Some(0x14)), (0x61991b, Some(0x14)), (0x619932, Some(0x15)), (0x62086d, Some(0x13)), (0x62088d, Some(0x4b)),
    (0x631847, Some(6)), (0x63187c, Some(6)), (0x6369f6, Some(0x15)), (0x636edf, Some(0xa)), (0x700ecd, Some(3)),
    (0x700efa, Some(0x15)), (0x806a33, Some(0x20)), (0x806a44, Some(0x20)), (0x80e377, Some(0x5e)), (0x80e3a4, Some(0x56)),
    (0x80e3b9, Some(0x2003)), (0x80e3ce, Some(0x2004)), (0x80e506, Some(0x5f)), (0x8150bb, Some(0xd)), (0x8150cb, Some(0xe)),
    (0x81c318, Some(0x44)), (0x81c325, Some(0x45)), (0x858f35, None), (0x858f5d, None), (0x859495, None), (0x859635, None),
    (0x85ad17, Some(0x31)), (0x85ad2f, Some(0x32)), (0x86221e, Some(0xf)), (0x865709, Some(1)), (0x865726, Some(0)),
    (0x865739, Some(0x4d)), (0x865759, Some(0x4c)), (0x865779, Some(0x4f)), (0x865799, Some(0x4e)), (0x8657e7, Some(0x50)),
    (0x866830, Some(0x3d)), (0x867efc, Some(0)), (0x867f08, Some(1)), (0x8698e0, Some(0)), (0x8698ec, Some(1)),
    (0x86bd5c, Some(0xe)), (0xa0efab, Some(2)), (0xa0efde, Some(2)), (0xa159cd, Some(7)), (0xa159da, Some(8)),
    (0xa15a03, Some(0x2d)), (0xa15a10, Some(0x2e)), (0xa2569f, Some(3)),
];

#[test]
fn the_guest_call_sites_pass_the_indexes_the_decision_lists() {
    let Some(dll) = real_dll() else { return };
    // the thunk: jmp [rip+0x154bfb2] -> the IAT slot 0x1970e98 (user32!GetSystemMetrics by pefile)
    assert_eq!(bytes_at(&dll, 0x424ee0, 6), [0xff, 0x25, 0xb2, 0xbf, 0x54, 0x01]);
    for (site, imm) in SITES {
        // `call rel32` that lands on the thunk
        let c = bytes_at(&dll, site, 5);
        assert_eq!(c[0], 0xe8, "{site:#x}: call");
        let target = (site as i64 + 5 + i32::from_le_bytes(c[1..5].try_into().unwrap()) as i64) as u64;
        assert_eq!(target, 0x424ee0, "{site:#x}: target");
        if let Some(i) = imm {
            let mov = bytes_at(&dll, site - 5, 5);
            let xor = bytes_at(&dll, site - 2, 2);
            let is_mov = mov[0] == 0xb9 && u32::from_le_bytes(mov[1..5].try_into().unwrap()) == i;
            let is_xor = i == 0 && xor == [0x33, 0xc9];
            assert!(is_mov || is_xor, "{site:#x}: index {i:#x}");
        }
    }
    // the two functions of the stop: f_620860 stores (GetSystemMetrics(19) != 0) in [rbx+0x10], f_620880 stores
    // (GetSystemMetrics(0x4b) != 0) in [rbx+0x28] and, when true, calls SystemParametersInfoW thunk 0x4255e0 with 0x68
    assert_eq!(bytes_at(&dll, 0x620868, 5), [0xb9, 0x13, 0, 0, 0]);
    assert_eq!(bytes_at(&dll, 0x620872, 8), [0x85, 0xc0, 0x0f, 0x95, 0xc0, 0x88, 0x43, 0x10]);
    assert_eq!(bytes_at(&dll, 0x620888, 5), [0xb9, 0x4b, 0, 0, 0]);
    assert_eq!(bytes_at(&dll, 0x620892, 8), [0x85, 0xc0, 0x0f, 0x95, 0xc0, 0x88, 0x43, 0x28]);
    assert_eq!(bytes_at(&dll, 0x62089e, 5), [0xb9, 0x68, 0, 0, 0]);
    // the index tables read by f_859470 (base 0x14082e0) and f_859610 (base 0x14082e8), `mov ecx,[base+rcx*4]` with rcx = the
    // byte [rbx+0x20]. The dwords from 0x14082e0 are 0x15 0x2 0x3 0x14 0x200 ...: f_859470 entries 0, 1 = 0x15 (SM_CXHSCROLL), 0x2
    // (SM_CXVSCROLL); f_859610 (base + 8) entries 0, 1 = 0x3 (SM_CYHSCROLL), 0x14 (SM_CYVSCROLL). The range of the byte is not
    // known: an entry 2 or more (0x200 is no SM_* number) would stop by number.
    let t = bytes_at(&dll, 0x14082e0, 16);
    let v: Vec<u32> = t.chunks(4).map(|c| u32::from_le_bytes(c.try_into().unwrap())).collect();
    assert_eq!(v, [0x15, 0x2, 0x3, 0x14]);
}

#[test]
fn every_static_index_of_the_guest_is_answered_or_stops_by_name() {
    // ties the call-site list above to the runtime: each immediate is in ANSWERED or UNDECIDED, and the C code does that
    let mut seen = std::collections::BTreeSet::new();
    for (_, imm) in SITES {
        if let Some(i) = imm {
            seen.insert(i as u64);
        }
    }
    // the two scroll bar-style tables of f_859470 / f_859610 give 0x15, 0x2, 0x3, 0x14 (checked against the DLL above)
    for i in [0x15u64, 2, 3, 0x14] {
        seen.insert(i);
    }
    assert_eq!(seen.len(), 36);
    for i in seen {
        let answered = ANSWERED.iter().find(|(n, _)| *n == i).map(|(_, v)| *v);
        match (answered, UNDECIDED.contains(&i)) {
            (Some(v), false) => assert_eq!(ask(i), Ok(v), "{i}"),
            (None, true) => assert!(ask(i).unwrap_err().contains("no value is decided"), "{i}"),
            other => panic!("index {i:#x} ({:?}): in {other:?} of the two lists", learn_name(i)),
        }
    }
}
