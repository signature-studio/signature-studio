//! Tests for the virtual SystemParametersInfoW of runtime/fle_spi.c: user32!SystemParametersInfoW
//! (docs/decisions/0058-virtual-spi.md; sources: docs/investigations/spi-caller.md).
//!
//! Expected values are written here from the decision (the uiAction table of the Microsoft Learn page SystemParametersInfoW,
//! the rules of the page, and the choices of the decision), not read back from the C code. The one value answered,
//! SPI_GETWHEELSCROLLLINES = 3, is on the page ("The default value is 3"): official. Everything else stops. The host is Linux:
//! nothing is compared with a real Windows; no settings, registry, mouse, theme, font, clock, thread or process is read.
//!
//! The C sources of fle_spi.c / fle_spi.h come from runtime/ when fle_spi.c is there, otherwise from out-spi/work/
//! (development). FLE_SPI_SRC_DIR (a directory with fle_spi.c and fle_spi.h) overrides both: the mutation runs of the run
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
    if let Ok(d) = std::env::var("FLE_SPI_SRC_DIR") {
        return PathBuf::from(d);
    }
    if runtime_dir().join("fle_spi.c").exists() {
        runtime_dir()
    } else {
        manifest().join("../../out-spi/work")
    }
}
fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-spi/test");
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

/// fle_spi.c (warnings are errors) and the existing files it links with: fle_stop (fle_env.c), GetLastError and
/// fle_set_last_error (fle_imports.c).
fn objs() -> &'static Vec<PathBuf> {
    static O: OnceLock<Vec<PathBuf>> = OnceLock::new();
    O.get_or_init(|| {
        let dir = work_dir().join(etxt_common::per_process("obj-spi"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut v = Vec::new();
        for (name, ours) in [("fle_spi.c", true), ("fle_imports.c", false), ("fle_env.c", false)] {
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
#include "fle_spi.h"

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
#define SPI(a, u, p, i) CALL(fle_imp_SystemParametersInfoW, (a), (u), (p), (i))
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

/// One program for every call: argv = uiAction uiParam pvParam("null" | "buf") fWinIni; each a u64 (the whole register). The
/// buffer is three guard dwords and pvParam points at the middle one. Output "ret <hex> <d0> <d1> <d2>".
fn cli() -> &'static PathBuf {
    static E: OnceLock<PathBuf> = OnceLock::new();
    E.get_or_init(|| {
        compile_main(
            "spi-cli",
            "",
            r#"
    uint64_t action = strtoull(argv[1], 0, 0);
    uint64_t ui = strtoull(argv[2], 0, 0);
    uint64_t ini = strtoull(argv[4], 0, 0);
    uint32_t buf[3] = { 0xAAAAAAAAu, 0xBBBBBBBBu, 0xCCCCCCCCu };
    uint64_t pv = strcmp(argv[3], "null") == 0 ? 0 : (uint64_t)(uintptr_t)&buf[1];
    uint64_t r = SPI(action, ui, pv, ini);
    printf("ret %llx %x %x %x\n", U(r), buf[0], buf[1], buf[2]);
"#,
        )
    })
}

/// What SystemParametersInfoW(action, ui, pv, ini) did: Ok((return value, the three guard dwords)), or Err(stderr) when the run
/// stopped (SIGABRT, "fle: SystemParametersInfoW", nothing printed before the stop).
fn call(action: u64, ui: u64, pv_null: bool, ini: u64) -> Result<(u64, [u32; 3]), String> {
    let o = exec(cli(), &[&format!("{action:#x}"), &format!("{ui:#x}"), if pv_null { "null" } else { "buf" }, &format!("{ini:#x}")]);
    if o.ok {
        let t: Vec<&str> = o.stdout.trim().split(' ').collect();
        assert_eq!(t[0], "ret", "odd output {}", o.stdout);
        let h = |s: &str| u64::from_str_radix(s, 16).unwrap();
        return Ok((h(t[1]), [h(t[2]) as u32, h(t[3]) as u32, h(t[4]) as u32]));
    }
    assert_eq!(o.signal, Some(6), "action {action:#x}: not SIGABRT: {}", o.stderr);
    assert!(o.stderr.contains("fle: SystemParametersInfoW"), "action {action:#x}: no 'fle: SystemParametersInfoW' message: {}", o.stderr);
    assert!(o.stdout.is_empty(), "action {action:#x}: output before the stop: {}", o.stdout);
    Err(o.stderr)
}

/// The usual form of a get call: uiParam 0, pvParam a buffer, fWinIni 0.
fn ask(action: u64) -> Result<(u64, [u32; 3]), String> {
    call(action, 0, false, 0)
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

/// The SPI_* names and numbers of the Learn page SystemParametersInfoW (fetched 2026-10-21) that this file needs: OFFICIAL,
/// written here from the page's tables (the guest's eleven, and the neighbours the stop tests use).
const LEARN: &[(u64, &str)] = &[
    (0x0003, "SPI_GETMOUSE"), (0x0004, "SPI_SETMOUSE"), (0x001F, "SPI_GETICONTITLELOGFONT"), (0x0026, "SPI_GETDRAGFULLWINDOWS"),
    (0x0029, "SPI_GETNONCLIENTMETRICS"), (0x002A, "SPI_SETNONCLIENTMETRICS"), (0x0030, "SPI_GETWORKAREA"), (0x0048, "SPI_GETANIMATION"),
    (0x0049, "SPI_SETANIMATION"), (0x004A, "SPI_GETFONTSMOOTHING"), (0x004B, "SPI_SETFONTSMOOTHING"), (0x0068, "SPI_GETWHEELSCROLLLINES"),
    (0x0069, "SPI_SETWHEELSCROLLLINES"), (0x006C, "SPI_GETWHEELSCROLLCHARS"), (0x006D, "SPI_SETWHEELSCROLLCHARS"),
    (0x1016, "SPI_GETTOOLTIPANIMATION"), (0x1017, "SPI_SETTOOLTIPANIMATION"), (0x1018, "SPI_GETTOOLTIPFADE"), (0x1019, "SPI_SETTOOLTIPFADE"),
    (0x101A, "SPI_GETCURSORSHADOW"), (0x101B, "SPI_SETCURSORSHADOW"), (0x1042, "SPI_GETCLIENTAREAANIMATION"),
];

fn learn_name(i: u64) -> Option<&'static str> {
    LEARN.iter().find(|(n, _)| *n == i).map(|(_, s)| *s)
}

/// Answered: the only uiAction with a value on the page. SPI_GETWHEELSCROLLLINES, a UINT that receives the number of lines;
/// "The default value is 3" (official).
const WHEEL: u64 = 0x68;
/// Used by the guest (the immediates of the 22 call sites) and not answered, with whether the action sets a parameter:
/// stop by name. The set ones stop because this runtime keeps and writes no setting (decision 0058); the get ones because the
/// page gives no default value for them.
/// SPI_GETICONTITLELOGFONT (0x1F) and SPI_GETNONCLIENTMETRICS (0x29) were in this list until decision 0066 (PROVISIONAL values,
/// runtime/fle_spi.c): the guest's next stop after the monitor is f_8663b0 asking both, so they are answered now (section 3b) and
/// stop only for the argument forms the page does not define.
/// SPI_GETWORKAREA (0x30) was in this list until decision 0098 (runtime/fle_spi.c, tests/wall_r.rs) answered it with the rectangle of
/// rcWork of decision 0065: the guest's census wall at unit 275. The claim "no value is decided" is no longer true for it.
/// SPI_GETCURSORSHADOW (0x101A) was in this list until decision 0106 (runtime/fle_spi.c, the test below) answered it with FALSE (PROVISIONAL, no
/// default on the page): ILTools' own initialisation (CreateCursorFromFont) asks it. The claim "no value is decided" is no longer true for it.
// decision 0126 (wall 92): old: [(0x26, false), (0x48, false), (0x49, true), (0x4B, true), (0x1016, false), (0x1018, false)]; SPI_GETANIMATION (0x48) is answered now (S-spi-animation), see animation_call below.
// decision 0242 (pick1): SPI_SETANIMATION (0x49) is answered too (it keeps iMinAnimate for SPI_GETANIMATION), see spi_setanimation_keeps_iminanimate below.
const LISTED: [(u64, bool); 4] = [(0x26, false), (0x4B, true), (0x1016, false), (0x1018, false)];
/// Answered by decision 0126 (PROVISIONAL, S-spi-animation): SPI_GETANIMATION with an ANIMATIONINFO { cbSize 8, iMinAnimate }.
const ANIMATION: u64 = 0x48;
/// Answered by decision 0106 (PROVISIONAL): the cursor shadow, a BOOL (4 bytes).
const CURSORSHADOW: u64 = 0x101A;
/// Answered by decision 0098 (PROVISIONAL): the work area, a RECT (tests/wall_r.rs).
const WORKAREA: u64 = 0x30;
/// Answered by decision 0066 (the two font actions; see section 3b).
const ICONFONT: u64 = 0x1F;
const NCM: u64 = 0x29;

// ---- 1. Table, header, constants ----

#[test]
fn the_row_is_in_the_table_and_declared_in_the_header() {
    let table = std::fs::read_to_string(manifest().join("src/imports_table.rs")).unwrap();
    let h = std::fs::read_to_string(src_dir().join("fle_spi.h")).unwrap();
    let row = "(\"user32.dll\", \"SystemParametersInfoW\", \"fle_imp_SystemParametersInfoW\", \"jmp_imp:SystemParametersInfoW\")";
    assert_eq!(table.matches(row).count(), 1, "exactly one row in imports_table.rs");
    assert!(h.contains("void fle_imp_SystemParametersInfoW(void)"));
    // the guest imports no SystemParametersInfoA (the real DLL test checks that); the nearby APIs are not answered by this decision.
    // LoadCursorW was in this list until decision 0059 (runtime/fle_cursor.c, tests/cursor.rs) answered it: the guest's
    // LoadCursorW(NULL, IDC_ARROW) is the next stop after the wheel lines, and the claim "not answered" is no longer true.
    // GetKeyboardLayout was in this list until decision 0063 (runtime/fle_kbd.c, tests/kbd.rs) answered it: it is the next stop
    // after LoadImageW (f_8651a0 at 0x86521d), and the claim "not answered" is no longer true.
    // GetDoubleClickTime was in this list until decision 0136 (runtime/fle_input.c, tests/wall_fr14.rs) answered it with a row appended at the end of the table.
    for n in ["SystemParametersInfoA", "SystemParametersInfoForDpi"] {
        assert!(!table.contains(&format!("\"{n}\"")), "{n} must not be answered by this decision");
    }
}

#[test]
fn the_spi_number_is_the_learn_page_number() {
    // official: 0x0068 is the row SPI_GETWHEELSCROLLLINES of the Learn table (written above from the page, not read back)
    assert_eq!(def_const("fle_spi.h", "FLE_SPI_GETWHEELSCROLLLINES"), 0x68);
    assert_eq!(learn_name(0x68), Some("SPI_GETWHEELSCROLLLINES"));
    assert_eq!(WHEEL, 0x68);
}

#[test]
fn the_decided_value_is_the_pages_default() {
    // Learn, SPI_GETWHEELSCROLLLINES: "The default value is 3." (official)
    assert_eq!(def_const("fle_spi.h", "FLE_SPI_WHEELSCROLLLINES"), 3);
}

#[test]
fn the_learn_table_in_this_file_has_no_repeated_number_and_no_repeated_name() {
    let mut nums: Vec<u64> = LEARN.iter().map(|(n, _)| *n).collect();
    let mut names: Vec<&str> = LEARN.iter().map(|(_, s)| *s).collect();
    let n = nums.len();
    nums.sort();
    nums.dedup();
    names.sort();
    names.dedup();
    assert_eq!(nums.len(), n);
    assert_eq!(names.len(), n);
    for (idx, _) in LISTED {
        assert!(learn_name(idx).is_some(), "{idx:#x} is not in the table of this file");
    }
}

// ---- 2. The answer ----

#[test]
fn the_wheel_scroll_lines_are_three_and_the_call_succeeds() {
    // a UINT is written at pvParam (4 bytes: the guard dwords around it stay), the return value is TRUE
    assert_eq!(ask(WHEEL), Ok((1, [0xAAAA_AAAA, 3, 0xCCCC_CCCC])));
}

#[test]
fn the_cursor_shadow_is_false_in_the_buffer_and_its_argument_forms_stop() {
    // decision 0106 (PROVISIONAL, no default on the page): one BOOL (4 bytes) at pvParam, FALSE; the guard dwords stay; TRUE is returned
    assert_eq!(ask(CURSORSHADOW), Ok((1, [0xAAAA_AAAA, 0, 0xCCCC_CCCC])));
    assert_eq!(def_const("fle_spi.h", "FLE_SPI_GETCURSORSHADOW"), CURSORSHADOW);
    assert_eq!(def_const("fle_spi.h", "FLE_SPI_CURSORSHADOW"), 0);
    // the forms the page does not define stop by name (as for the other get actions answered by decisions 0098 and 0102)
    let e = call(CURSORSHADOW, 0, true, 0).expect_err("pvParam NULL");
    assert!(e.contains("SPI_GETCURSORSHADOW") && e.contains("pvParam NULL"), "{e}");
    let e = call(CURSORSHADOW, 1, false, 0).expect_err("uiParam");
    assert!(e.contains("SPI_GETCURSORSHADOW") && e.contains("uiParam not zero"), "{e}");
    let e = call(CURSORSHADOW, 0, false, 1).expect_err("fWinIni");
    assert!(e.contains("SPI_GETCURSORSHADOW") && e.contains("fWinIni not zero"), "{e}");
    // SPI_SETCURSORSHADOW (0x101B) is not used by the guest and stays unanswered
    let e = ask(0x101B).expect_err("SET");
    assert!(e.contains(&format!("uiAction {:#x} is not answered", 0x101Bu64)), "{e}");
}

#[test]
fn only_the_low_dword_of_each_uint_argument_counts() {
    // UINT arguments are 32 bits: the guest may leave anything in the upper half of rcx, rdx and r9
    assert_eq!(call(0xAAAA_AAAA_0000_0068, 0, false, 0), Ok((1, [0xAAAA_AAAA, 3, 0xCCCC_CCCC])));
    assert_eq!(call(WHEEL, 0xFFFF_FFFF_0000_0000, false, 0), Ok((1, [0xAAAA_AAAA, 3, 0xCCCC_CCCC])));
    assert_eq!(call(WHEEL, 0, false, 0x1234_5678_0000_0000), Ok((1, [0xAAAA_AAAA, 3, 0xCCCC_CCCC])));
    // and the low dword decides: a high bit set with a low dword that is not 0x68 is another action
    assert!(call(0x0000_0001_0000_0068 + 1, 0, false, 0).is_err());
    assert!(call(WHEEL, 0x0000_0000_0000_0001, false, 0).is_err(), "low dword of uiParam 1");
    assert!(call(WHEEL, 0, false, 0x0000_0000_0000_0001).is_err(), "low dword of fWinIni 1");
}

#[test]
fn it_is_stateless_leaves_the_last_error_alone_and_keeps_the_return_address() {
    // Learn: on failure GetLastError gives the extended error information; this call succeeds, so the last error is not touched
    let l = lines(
        "spi-state",
        r#"
    uint32_t a = 0xBBBBBBBBu, b = 0xBBBBBBBBu;
    uint64_t r1, r2;
    SETLE(0x1234);
    r1 = SPI(0x68, 0, (uint64_t)(uintptr_t)&a, 0);
    r2 = SPI(0x68, 0, (uint64_t)(uintptr_t)&b, 0);
    printf("same %d %d %d\n", a == b, r1 == r2, a == 3);
    printf("last error %x\n", LE());
    printf("return address %llx\n", U(stk[0]));
    SETLE(0);
    SPI(0x68, 0, (uint64_t)(uintptr_t)&a, 0);
    printf("last error %x\n", LE());
"#,
    );
    assert_eq!(l, ["same 1 1 1", "last error 1234", "return address dead0000", "last error 0"]);
}

#[test]
fn the_value_does_not_depend_on_what_was_in_the_buffer() {
    // the call writes the value; it does not add to or keep what was there
    let l = lines(
        "spi-overwrite",
        r#"
    uint32_t a = 0, b = 0xFFFFFFFFu, c = 7;
    SPI(0x68, 0, (uint64_t)(uintptr_t)&a, 0);
    SPI(0x68, 0, (uint64_t)(uintptr_t)&b, 0);
    SPI(0x68, 0, (uint64_t)(uintptr_t)&c, 0);
    printf("%u %u %u\n", a, b, c);
"#,
    );
    assert_eq!(l, ["3 3 3"]);
}

// ---- 3. Stops ----

#[test]
fn forms_the_page_does_not_define_stop_with_a_named_reason() {
    // pvParam NULL, uiParam not zero ("If not otherwise indicated, you must specify zero"), fWinIni for a get action
    let e = call(WHEEL, 0, true, 0).expect_err("pvParam NULL");
    assert!(e.contains("SPI_GETWHEELSCROLLLINES") && e.contains("pvParam NULL"), "{e}");
    for ui in [1u64, 4, 0xFFFF_FFFF] {
        let e = call(WHEEL, ui, false, 0).expect_err("uiParam");
        assert!(e.contains("SPI_GETWHEELSCROLLLINES") && e.contains("uiParam not zero"), "uiParam {ui:#x}: {e}");
    }
    // SPIF_UPDATEINIFILE, SPIF_SENDCHANGE (= SPIF_SENDWININICHANGE), both
    for ini in [1u64, 2, 3, 0xFFFF_FFFF] {
        let e = call(WHEEL, 0, false, ini).expect_err("fWinIni");
        assert!(e.contains("SPI_GETWHEELSCROLLLINES") && e.contains("fWinIni not zero"), "fWinIni {ini:#x}: {e}");
    }
}

#[test]
fn actions_the_guest_uses_without_a_decided_value_or_that_set_stop_by_name() {
    for (idx, set) in LISTED {
        let name = learn_name(idx).unwrap_or_else(|| panic!("{idx:#x} is not in the table of this file"));
        assert_eq!(name.starts_with("SPI_SET"), set, "{name}: the set flag follows the name");
        let e = ask(idx).expect_err(&format!("{name} must stop"));
        assert!(e.contains(&format!("uiAction {idx:#x} ({name})")), "{name}: {e}");
        if set {
            assert!(e.contains("sets a system parameter"), "{name}: {e}");
            assert!(!e.contains("no value is decided"), "{name}: {e}");
        } else {
            assert!(e.contains("no value is decided"), "{name}: {e}");
            assert!(!e.contains("sets a system parameter"), "{name}: {e}");
        }
        assert!(e.contains("decision 0058"), "{name}: {e}");
    }
}

#[test]
fn a_set_action_stops_whatever_the_arguments_and_nothing_is_written() {
    // the guest's own forms of the two set calls: SPI_SETFONTSMOOTHING(0, NULL, 0) and SPI_SETANIMATION(8, &ANIMATIONINFO, 0)
    // (decision 0242: SPI_SETANIMATION (0x49) is answered now, see the next test; SPI_SETFONTSMOOTHING stays a stop)
    for (action, ui, pv_null) in [(0x4Bu64, 0u64, true), (0x4B, 1, true)] {
        let e = call(action, ui, pv_null, 0).expect_err("set action");
        assert!(e.contains("sets a system parameter"), "{action:#x}: {e}");
    }
}

#[test]
fn spi_setanimation_keeps_iminanimate_for_spi_getanimation() {
    // decision 0242 (pick1). Learn SPI_SETANIMATION: pvParam points to an ANIMATIONINFO with the new parameters; cbSize and uiParam are sizeof(ANIMATIONINFO) = 8; nonzero on success. The value is kept in this process for SPI_GETANIMATION.
    let l = lines(
        "spi-setanim",
        r#"
    uint32_t s[2], g[2]; uint64_t r;
    g[0] = 8; g[1] = 0xFFFF; r = SPI(0x48, 8, (uint64_t)(uintptr_t)g, 0); printf("get0 %llu %u\n", U(r), g[1]);
    s[0] = 8; s[1] = 0; r = SPI(0x49, 8, (uint64_t)(uintptr_t)s, 0); printf("set0 %llu\n", U(r));
    g[0] = 8; g[1] = 0xFFFF; r = SPI(0x48, 8, (uint64_t)(uintptr_t)g, 0); printf("get1 %llu %u\n", U(r), g[1]);
    s[0] = 8; s[1] = 5; r = SPI(0x49, 8, (uint64_t)(uintptr_t)s, 3); printf("set1 %llu\n", U(r));
    g[0] = 8; g[1] = 0xFFFF; r = SPI(0x48, 8, (uint64_t)(uintptr_t)g, 0); printf("get2 %llu %u\n", U(r), g[1]);
"#,
    );
    assert_eq!(l, ["get0 1 1", "set0 1", "get1 1 0", "set1 1", "get2 1 1"]);
    // the forms the page leaves open stop by name
    for (ui, cb, pv_null, why) in [(0u64, 8u32, false, "uiParam not sizeof"), (8, 4, false, "cbSize not 8"), (8, 8, true, "pvParam NULL")] {
        let e = bufcall(0x49, ui, pv_null, Some(cb), 0xAA, 0, 8).expect_err(why);
        assert!(e.contains("SPI_SETANIMATION") && e.contains(why), "{ui} {cb}: {e}");
    }
}

#[test]
fn the_other_wheel_actions_are_not_answered() {
    // SPI_SETWHEELSCROLLLINES (0x69), SPI_GETWHEELSCROLLCHARS (0x6C: the page also says "The default value is 3"),
    // SPI_SETWHEELSCROLLCHARS (0x6D): the guest does not use them, so they stop by number
    for idx in [0x69u64, 0x6C, 0x6D] {
        let e = ask(idx).expect_err(learn_name(idx).unwrap());
        assert!(e.contains(&format!("uiAction {idx:#x} is not answered")), "{idx:#x}: {e}");
    }
}

#[test]
fn every_other_uiaction_stops_by_number() {
    let known: Vec<u64> = LISTED.iter().map(|(i, _)| *i).chain([WHEEL, ICONFONT, NCM, WORKAREA, CURSORSHADOW]).collect();
    // the neighbours of each used action, the first numbers, the group starts of the page, and wide values (an abort per number
    // is slow, so not every number of the page). 0x22 (SPI_SETICONTITLELOGFONT), 0x2A (SPI_SETNONCLIENTMETRICS), 0x2D and 0x2E
    // (SPI_GET/SETICONMETRICS) are the neighbours of the two font actions of decision 0066: they must still stop.
    let mut idxs: Vec<u64> = (0..=0x10).collect();
    idxs.extend([0x1E, 0x20, 0x22, 0x25, 0x27, 0x28, 0x2A, 0x2D, 0x2E, 0x2F, 0x31, 0x47, 0x4A, 0x4C, 0x67, 0x69, 0x6A, 0x6C, 0x6D, 0x90, 0xAE, 0xAF]);
    idxs.extend([0x1000, 0x1015, 0x1017, 0x1019, 0x101B, 0x1042, 0x2000, 0x2010]);
    idxs.extend([0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFFF, 0x1_0000_0001, 0xFFFF_FFFF_0000_0003]);
    for idx in idxs {
        if known.contains(&(idx & 0xFFFF_FFFF)) {
            continue;
        }
        let e = ask(idx).expect_err(&format!("uiAction {idx:#x} must stop"));
        assert!(e.contains(&format!("uiAction {:#x} is not answered", idx as u32)), "{idx:#x}: {e}");
    }
}

// ---- 3b. The two font actions (decision 0066, PROVISIONAL values) ----
//
// The structure sizes and offsets are written here from the Learn definitions of LOGFONTW (5 LONG, 8 BYTE, WCHAR[32] = 92) and
// NONCLIENTMETRICSW (the member order of the page), not read back from the C code. The values of the font are the decision's:
// lfHeight from the Learn formula -MulDiv(9, 96, 72) = -12 (the 9 pt and the face are not on a Learn page for Windows 10; the Fonts
// page gives them for Windows Vista and 7), the rest choices.

const LF_SIZE: usize = 92;
const NCM_FULL: usize = 0x1f8;
const NCM_OLD: usize = 0x1f4;
// member offsets of NONCLIENTMETRICSW in order: cbSize, iBorderWidth, iScrollWidth, iScrollHeight, iCaptionWidth, iCaptionHeight,
// lfCaptionFont, iSmCaptionWidth, iSmCaptionHeight, lfSmCaptionFont, iMenuWidth, iMenuHeight, lfMenuFont, lfStatusFont,
// lfMessageFont, iPaddedBorderWidth
const O_BORDER: usize = 4;
const O_CAPTION: usize = 0x18;
const O_SMCAPTION: usize = 0x7c;
const O_MENU: usize = 0xe0;
const O_STATUS: usize = 0x13c;
const O_MESSAGE: usize = 0x198;
const O_PADDED: usize = 0x1f4;

/// The LOGFONTW of the decision, field by field (little endian): the one font used for all five.
fn the_font() -> Vec<u8> {
    let mut f = vec![0u8; LF_SIZE];
    f[0..4].copy_from_slice(&(-12i32).to_le_bytes()); // lfHeight = -MulDiv(9, 96, 72)
    f[16..20].copy_from_slice(&400i32.to_le_bytes()); // lfWeight = FW_NORMAL
    f[23] = 1; // lfCharSet = DEFAULT_CHARSET
    // lfOutPrecision 0 (OUT_DEFAULT_PRECIS), lfClipPrecision 0 (CLIP_DEFAULT_PRECIS), lfPitchAndFamily 0 (DEFAULT_PITCH | FF_DONTCARE)
    // lfQuality 5 (CLEARTYPE_QUALITY): decision 0067 (was 0 in decision 0066). Unofficial: the bytes of the WindowMetrics registry
    // values of real Windows 10 systems (docs/decisions/0067-system-font-fields.md), the 11th byte of the 0x8c-byte tail of a LOGFONTW.
    f[26] = 5;
    for (i, c) in "Segoe UI".encode_utf16().enumerate() {
        f[28 + 2 * i..30 + 2 * i].copy_from_slice(&c.to_le_bytes());
    }
    f
}

/// One program for the buffer forms: argv = uiAction uiParam pvParam("null" | "buf") cbSize("none" | number written at pvParam) fill
/// fWinIni len. The buffer is a guard of 4 bytes, `len` bytes (all `fill`, then cbSize at the start), a guard of 4 bytes; pvParam
/// points after the first guard. Output "ret <hex>" and "bytes <hex>" (guards included).
fn bufcli() -> &'static PathBuf {
    static E: OnceLock<PathBuf> = OnceLock::new();
    E.get_or_init(|| {
        compile_main(
            "spi-bufcli",
            "",
            r#"
    uint64_t action = strtoull(argv[1], 0, 0), ui = strtoull(argv[2], 0, 0), ini = strtoull(argv[6], 0, 0);
    unsigned fill = (unsigned)strtoul(argv[5], 0, 0), len = (unsigned)strtoul(argv[7], 0, 0);
    uint8_t buf[1024];
    uint64_t pv, r;
    unsigned i;
    memset(buf, 0xC7, sizeof buf);
    memset(buf + 4, (int)fill, len);
    if (strcmp(argv[4], "none") != 0) { uint32_t cb = (uint32_t)strtoul(argv[4], 0, 0); memcpy(buf + 4, &cb, 4); }
    pv = strcmp(argv[3], "null") == 0 ? 0 : (uint64_t)(uintptr_t)(buf + 4);
    r = SPI(action, ui, pv, ini);
    printf("ret %llx\n", U(r));
    printf("bytes ");
    for (i = 0; i < len + 8; i++) printf("%02x", buf[i]);
    printf("\n");
"#,
        )
    })
}

/// (return value, the len + 8 bytes with the guards) or Err(stderr) of a stop.
fn bufcall(action: u64, ui: u64, pv_null: bool, cb: Option<u32>, fill: u8, ini: u64, len: usize) -> Result<(u64, Vec<u8>), String> {
    let cbs = cb.map(|c| format!("{c:#x}")).unwrap_or_else(|| "none".to_string());
    let o = exec(
        bufcli(),
        &[&format!("{action:#x}"), &format!("{ui:#x}"), if pv_null { "null" } else { "buf" }, &cbs, &format!("{fill:#x}"), &format!("{ini:#x}"), &len.to_string()],
    );
    if !o.ok {
        assert_eq!(o.signal, Some(6), "action {action:#x}: not SIGABRT: {}", o.stderr);
        assert!(o.stderr.contains("fle: SystemParametersInfoW"), "{}", o.stderr);
        assert!(o.stdout.is_empty(), "output before the stop: {}", o.stdout);
        return Err(o.stderr);
    }
    let l: Vec<&str> = o.stdout.lines().collect();
    let ret = u64::from_str_radix(l[0].strip_prefix("ret ").unwrap(), 16).unwrap();
    let hex = l[1].strip_prefix("bytes ").unwrap();
    let bytes = (0..hex.len() / 2).map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap()).collect();
    Ok((ret, bytes))
}

/// SPI_GETICONTITLELOGFONT with the guest's form (uiParam 0x5c, fWinIni 0) into a 92-byte buffer of `fill`.
fn font_call(ui: u64, fill: u8) -> Result<(u64, Vec<u8>), String> {
    bufcall(ICONFONT, ui, false, None, fill, 0, LF_SIZE)
}

/// SPI_GETNONCLIENTMETRICS with cbSize `cb` written at the start of a buffer of `cb` bytes of `fill` (the buffer the guest has),
/// and `ui` as uiParam. The returned bytes: guard (4), the structure (cb), guard (4).
fn ncm_call(cb: u32, ui: u64, fill: u8) -> Result<(u64, Vec<u8>), String> {
    bufcall(NCM, ui, false, Some(cb), fill, 0, cb as usize)
}

/// SPI_GETANIMATION with cbSize `cb` written at the start of an 8-byte buffer of `fill`, and `ui` as uiParam. The returned bytes: guard (4), the structure (8), guard (4).
fn animation_call(cb: u32, ui: u64, fill: u8) -> Result<(u64, Vec<u8>), String> {
    bufcall(ANIMATION, ui, false, Some(cb), fill, 0, 8)
}

#[test]
fn spi_getanimation_gives_an_animationinfo_with_minanimate_one_and_stops_on_the_forms_the_page_leaves_open() {
    // decision 0126, wall 92 (S-spi-animation, a choice without a source). Learn SPI_GETANIMATION: "pvParam must point to an ANIMATIONINFO structure that receives the information. Set the cbSize member of this
    // structure and the uiParam parameter to sizeof(ANIMATIONINFO)"; ANIMATIONINFO is { UINT cbSize; int iMinAnimate } = 8 bytes; "If this member is nonzero, minimize and restore animation is enabled; otherwise it
    // is disabled". The page gives no default: iMinAnimate is 0 (disabled). cbSize is left as the caller set it; the guard bytes before and after the structure are not touched.
    let (ret, b) = animation_call(8, 8, 0xAA).expect("the guest's form");
    assert_eq!(ret, 1);
    assert_eq!(&b[0..4], &[0xC7; 4], "the guard before (bufcall's guard byte is 0xC7)");
    assert_eq!(&b[4..8], &8u32.to_le_bytes(), "cbSize is the caller's");
    assert_eq!(&b[8..12], &1u32.to_le_bytes(), "iMinAnimate = 1 (decision 0242, S-pick1-spi-2; was 0)");
    assert_eq!(&b[12..16], &[0xC7; 4], "the guard after");
    for (cb, ui, why) in [(8u32, 0u64, "uiParam not sizeof(ANIMATIONINFO)"), (8, 4, "uiParam not sizeof(ANIMATIONINFO)"), (4, 8, "cbSize not 8"), (0, 8, "cbSize not 8")] {
        let e = animation_call(cb, ui, 0xAA).expect_err(why);
        assert!(e.contains("SPI_GETANIMATION") && e.contains(why.split(' ').next().unwrap()), "cb {cb} ui {ui}: {e}");
    }
    let e = bufcall(ANIMATION, 8, true, None, 0xAA, 0, 8).expect_err("pvParam NULL");
    assert!(e.contains("pvParam NULL"), "{e}");
    let e = bufcall(ANIMATION, 8, false, Some(8), 0xAA, 1, 8).expect_err("fWinIni");
    assert!(e.contains("fWinIni not zero"), "{e}");
}

#[test]
fn the_font_constants_are_the_decisions() {
    // official: 0x1F and 0x29 are the Learn rows SPI_GETICONTITLELOGFONT and SPI_GETNONCLIENTMETRICS
    assert_eq!(def_const("fle_spi.h", "FLE_SPI_GETICONTITLELOGFONT"), 0x1f);
    assert_eq!(def_const("fle_spi.h", "FLE_SPI_GETNONCLIENTMETRICS"), 0x29);
    assert_eq!((learn_name(0x1f), learn_name(0x29)), (Some("SPI_GETICONTITLELOGFONT"), Some("SPI_GETNONCLIENTMETRICS")));
    // computed from the Learn structure definitions
    assert_eq!(def_const("fle_spi.h", "FLE_SPI_LOGFONTW_SIZE"), 92);
    assert_eq!(def_const("fle_spi.h", "FLE_SPI_NCM_SIZE"), 0x1f8);
    assert_eq!(def_const("fle_spi.h", "FLE_SPI_NCM_SIZE_NO_PADDED"), 0x1f4);
    assert_eq!(4 + 5 * 4 + 92 + 2 * 4 + 92 + 2 * 4 + 3 * 92 + 4, NCM_FULL);
    assert_eq!(NCM_FULL - 4, NCM_OLD);
    for (n, v) in [
        ("FLE_SPI_NCM_BORDERWIDTH", O_BORDER),
        ("FLE_SPI_NCM_CAPTIONFONT", O_CAPTION),
        ("FLE_SPI_NCM_SMCAPTIONFONT", O_SMCAPTION),
        ("FLE_SPI_NCM_MENUFONT", O_MENU),
        ("FLE_SPI_NCM_STATUSFONT", O_STATUS),
        ("FLE_SPI_NCM_MESSAGEFONT", O_MESSAGE),
        ("FLE_SPI_NCM_PADDEDBORDER", O_PADDED),
    ] {
        assert_eq!(def_const("fle_spi.h", n), v as u64, "{n}");
    }
    // the member order gives the same offsets: caption at 4 + 5 ints, small caption after the caption font and two ints, ...
    assert_eq!(O_CAPTION, 4 + 5 * 4);
    assert_eq!(O_SMCAPTION, O_CAPTION + 92 + 2 * 4);
    assert_eq!(O_MENU, O_SMCAPTION + 92 + 2 * 4);
    assert_eq!(O_STATUS, O_MENU + 92);
    assert_eq!(O_MESSAGE, O_STATUS + 92);
    assert_eq!(O_PADDED, O_MESSAGE + 92);
    // official defaults: iBorderWidth 1 pixel, iPaddedBorderWidth 4 pixels
    assert_eq!(def_const("fle_spi.h", "FLE_SPI_BORDERWIDTH"), 1);
    assert_eq!(def_const("fle_spi.h", "FLE_SPI_PADDEDBORDERWIDTH"), 4);
    // provisional (decision 0066): 9 pt, FW_NORMAL, DEFAULT_CHARSET, and the zero precision/pitch fields; lfQuality is 5 since
    // decision 0067 (was 0): unofficial, the registry bytes of real Windows 10 systems
    assert_eq!(def_const("fle_spi.h", "FLE_SPI_FONT_POINTS"), 9);
    assert_eq!(def_const("fle_spi.h", "FLE_SPI_FONT_WEIGHT"), 400);
    assert_eq!(def_const("fle_spi.h", "FLE_SPI_FONT_CHARSET"), 1);
    for n in ["FLE_SPI_FONT_OUTPRECISION", "FLE_SPI_FONT_CLIPPRECISION", "FLE_SPI_FONT_PITCHANDFAMILY"] {
        assert_eq!(def_const("fle_spi.h", n), 0, "{n}");
    }
    assert_eq!(def_const("fle_spi.h", "FLE_SPI_FONT_QUALITY"), 5);
    let h = std::fs::read_to_string(src_dir().join("fle_spi.h")).unwrap();
    assert!(h.contains("#define FLE_SPI_FONT_FACE           \"Segoe UI\""));
    // lfHeight is the Learn formula with the DPI of decision 0053, not a second copy of -12
    let l = lines("spi-font-height", "    printf(\"%d %d\\n\", FLE_SPI_FONT_HEIGHT, -(FLE_SPI_FONT_POINTS * FLE_GDC_LOGPIXELSY / 72));");
    assert_eq!(l, ["-12 -12"]);
    assert!(h.contains("FLE_GDC_LOGPIXELSY"), "the height is derived from the DPI constant of decision 0053");
}

#[test]
fn the_icon_title_font_is_the_decisions_font_in_92_bytes_and_nothing_else_is_written() {
    // guest form: uiParam 0x5c, fWinIni 0. Guards of 4 bytes on both sides stay; the fill does not leak into the result.
    for fill in [0x00u8, 0xAA, 0x55, 0xFF] {
        let (ret, b) = font_call(0x5c, fill).unwrap();
        assert_eq!(ret, 1, "fill {fill:#x}");
        assert_eq!(&b[..4], [0xC7; 4], "guard before");
        assert_eq!(&b[4..4 + LF_SIZE], the_font().as_slice(), "fill {fill:#x}");
        assert_eq!(&b[4 + LF_SIZE..], [0xC7; 4], "guard after");
    }
    // spelled out: the fields of lfHeight, lfWeight, lfCharSet and the face, and the zero rest of the 32 face units
    let f = the_font();
    assert_eq!(i32::from_le_bytes(f[0..4].try_into().unwrap()), -12);
    assert_eq!(i32::from_le_bytes(f[16..20].try_into().unwrap()), 400);
    assert_eq!(f[23], 1);
    assert_eq!(&f[28..28 + 16], "Segoe UI".encode_utf16().flat_map(|c| c.to_le_bytes()).collect::<Vec<u8>>().as_slice());
    assert!(f[28 + 16..].iter().all(|&c| c == 0), "the 24 units after the 8 characters are zero (the terminator included)");
}

#[test]
fn the_non_client_metrics_of_cbsize_0x1f8_hold_the_four_fonts_the_two_page_defaults_and_zeros() {
    for fill in [0x00u8, 0xAA, 0x55, 0xFF] {
        let (ret, b) = ncm_call(0x1f8, 0, fill).unwrap();
        assert_eq!(ret, 1, "fill {fill:#x}");
        let s = &b[4..4 + NCM_FULL];
        assert_eq!(&b[..4], [0xC7; 4], "guard before");
        assert_eq!(&b[4 + NCM_FULL..], [0xC7; 4], "guard after");
        // cbSize is the guest's own value and is left as it was
        assert_eq!(u32::from_le_bytes(s[0..4].try_into().unwrap()), 0x1f8, "cbSize untouched");
        // iBorderWidth: Learn "The default is 1 pixel"; iPaddedBorderWidth: Learn "The default value is 4 pixels"
        assert_eq!(i32::from_le_bytes(s[O_BORDER..O_BORDER + 4].try_into().unwrap()), 1);
        assert_eq!(i32::from_le_bytes(s[O_PADDED..O_PADDED + 4].try_into().unwrap()), 4);
        // the four fonts the guest reads
        let f = the_font();
        for (name, o) in [("caption", O_CAPTION), ("menu", O_MENU), ("status", O_STATUS), ("message", O_MESSAGE)] {
            assert_eq!(&s[o..o + LF_SIZE], f.as_slice(), "{name} font, fill {fill:#x}");
        }
        // every other byte: written as 0, so the result never depends on what the buffer held (nothing is invented as a decided value)
        let mut want = vec![0u8; NCM_FULL];
        want[0..4].copy_from_slice(&0x1f8u32.to_le_bytes());
        want[O_BORDER..O_BORDER + 4].copy_from_slice(&1i32.to_le_bytes());
        want[O_PADDED..O_PADDED + 4].copy_from_slice(&4i32.to_le_bytes());
        for o in [O_CAPTION, O_MENU, O_STATUS, O_MESSAGE] {
            want[o..o + LF_SIZE].copy_from_slice(&f);
        }
        assert_eq!(s, want.as_slice(), "the whole structure, fill {fill:#x}");
    }
}

#[test]
fn the_non_client_metrics_of_cbsize_0x1f4_do_not_touch_the_padded_border_bytes() {
    // the 500-byte structure has no iPaddedBorderWidth: the four bytes at 0x1f4 are not part of it, so they stay (here: the last 4
    // bytes of a 504-byte buffer, and the guard after it)
    for fill in [0x00u8, 0xAA, 0xFF] {
        let (ret, b) = bufcall(NCM, 0, false, Some(0x1f4), fill, 0, NCM_FULL).unwrap();
        assert_eq!(ret, 1);
        let s = &b[4..4 + NCM_FULL];
        assert_eq!(u32::from_le_bytes(s[0..4].try_into().unwrap()), 0x1f4);
        assert_eq!(&s[O_PADDED..], [fill; 4], "bytes 0x1f4..0x1f8 untouched, fill {fill:#x}");
        assert_eq!(&b[4 + NCM_FULL..], [0xC7; 4]);
        assert_eq!(i32::from_le_bytes(s[O_BORDER..O_BORDER + 4].try_into().unwrap()), 1);
        let f = the_font();
        for o in [O_CAPTION, O_MENU, O_STATUS, O_MESSAGE] {
            assert_eq!(&s[o..o + LF_SIZE], f.as_slice());
        }
    }
    // and with a buffer of exactly 500 bytes (the guest's) the guard right after it is intact
    let (_, b) = ncm_call(0x1f4, 0, 0xAA).unwrap();
    assert_eq!(&b[4 + NCM_OLD..], [0xC7; 4]);
    assert_eq!(b.len(), 4 + NCM_OLD + 4);
}

#[test]
fn the_undecided_dimensions_and_the_small_caption_font_are_written_as_zero_and_nothing_else_is_zero_by_accident() {
    // iScrollWidth .. iCaptionHeight (8..0x18), iSmCaptionWidth/Height (0x74..0x7c), lfSmCaptionFont (0x7c..0xd8),
    // iMenuWidth/Height (0xd8..0xe0): no value is decided for them (the guest reads none of them as far as the static reading goes)
    let (_, b) = ncm_call(0x1f8, 0, 0xFF).unwrap();
    let s = &b[4..4 + NCM_FULL];
    for (name, a, e) in [("scroll/caption dims", 8usize, 0x18usize), ("small caption dims", 0x74, 0x7c), ("small caption font", 0x7c, 0xd8), ("menu dims", 0xd8, 0xe0)] {
        assert!(s[a..e].iter().all(|&c| c == 0), "{name} are zero");
    }
    // the regions that are not zero are exactly: cbSize, iBorderWidth, the four fonts, iPaddedBorderWidth
    let nonzero: Vec<usize> = (0..NCM_FULL).filter(|&i| s[i] != 0).collect();
    let in_font = |i: usize| [O_CAPTION, O_MENU, O_STATUS, O_MESSAGE].iter().any(|&o| i >= o && i < o + LF_SIZE);
    for i in nonzero {
        assert!(i < 4 || (O_BORDER..O_BORDER + 4).contains(&i) || in_font(i) || (O_PADDED..O_PADDED + 4).contains(&i), "byte {i:#x} is not zero");
    }
}

#[test]
fn the_font_actions_leave_the_last_error_alone_and_keep_the_return_address() {
    let l = lines(
        "spi-font-state",
        r#"
    uint8_t lf[92], ncm[504];
    uint64_t r1, r2;
    memset(lf, 0xAA, sizeof lf); memset(ncm, 0xAA, sizeof ncm);
    *(uint32_t *)ncm = 0x1f8;
    SETLE(0x1234);
    r1 = SPI(0x1f, 0x5c, (uint64_t)(uintptr_t)lf, 0);
    printf("lf %llu last error %x ra %llx\n", U(r1), LE(), U(stk[0]));
    r2 = SPI(0x29, 0, (uint64_t)(uintptr_t)ncm, 0);
    printf("ncm %llu last error %x ra %llx\n", U(r2), LE(), U(stk[0]));
    SETLE(0);
    SPI(0x1f, 0x5c, (uint64_t)(uintptr_t)lf, 0);
    SPI(0x29, 0, (uint64_t)(uintptr_t)ncm, 0);
    printf("last error %x\n", LE());
"#,
    );
    assert_eq!(l, ["lf 1 last error 1234 ra dead0000", "ncm 1 last error 1234 ra dead0000", "last error 0"]);
}

#[test]
fn only_the_low_dword_of_each_uint_argument_counts_for_the_font_actions() {
    assert_eq!(font_call(0xFFFF_FFFF_0000_005c, 0xAA).unwrap().1[4..4 + LF_SIZE], the_font()[..]);
    assert!(bufcall(0xAAAA_AAAA_0000_001F, 0x5c, false, None, 0xAA, 0, LF_SIZE).is_ok());
    assert!(bufcall(0x0000_0001_0000_001F, 0x5c, false, None, 0xAA, 0, LF_SIZE).is_ok(), "the high dword of uiAction is ignored");
    assert!(bufcall(ICONFONT, 0x5c, false, None, 0xAA, 0x1234_5678_0000_0000, LF_SIZE).is_ok(), "the high dword of fWinIni is ignored");
    assert!(bufcall(ICONFONT, 0x5c, false, None, 0xAA, 0x0000_0000_0000_0001, LF_SIZE).is_err(), "the low dword of fWinIni counts");
    assert!(bufcall(NCM, 0xFFFF_FFFF_0000_0000, false, Some(0x1f8), 0xAA, 0, NCM_FULL).is_ok(), "uiParam 0 in the low dword");
    assert!(bufcall(NCM, 0xFFFF_FFFF_0000_01f8, false, Some(0x1f8), 0xAA, 0, NCM_FULL).is_ok(), "uiParam 0x1f8 in the low dword");
}

#[test]
fn the_icon_title_font_stops_on_the_forms_the_page_does_not_define() {
    // Learn: "The uiParam parameter specifies the size of a LOGFONT structure, and the pvParam parameter must point to the LOGFONT
    // structure to fill in." The guest passes 0x5c. Not 0x5c, NULL, and a fWinIni (a set action's parameter) stop, with a name.
    for ui in [0u64, 1, 0x5b, 0x5d, 0x3c, 0x60, 0x1f8, 0xFFFF_FFFF] {
        let e = bufcall(ICONFONT, ui, false, None, 0xAA, 0, LF_SIZE).expect_err("uiParam");
        assert!(e.contains("SPI_GETICONTITLELOGFONT") && e.contains("uiParam not sizeof(LOGFONTW)"), "uiParam {ui:#x}: {e}");
    }
    let e = bufcall(ICONFONT, 0x5c, true, None, 0xAA, 0, LF_SIZE).expect_err("pvParam NULL");
    assert!(e.contains("SPI_GETICONTITLELOGFONT") && e.contains("pvParam NULL"), "{e}");
    for ini in [1u64, 2, 3, 0xFFFF_FFFF] {
        let e = bufcall(ICONFONT, 0x5c, false, None, 0xAA, ini, LF_SIZE).expect_err("fWinIni");
        assert!(e.contains("SPI_GETICONTITLELOGFONT") && e.contains("fWinIni not zero"), "fWinIni {ini:#x}: {e}");
    }
    // nothing is written before a stop: the checks come first (the stop leaves no output, so look at the order with a NULL pvParam
    // and a wrong uiParam: pvParam NULL is reported first)
    let e = bufcall(ICONFONT, 0, true, None, 0xAA, 0, LF_SIZE).expect_err("both");
    assert!(e.contains("pvParam NULL"), "{e}");
}

#[test]
fn the_non_client_metrics_stop_on_the_forms_the_page_does_not_define() {
    // Learn: "Set the cbSize member of this structure and the uiParam parameter to sizeof(NONCLIENTMETRICS)". cbSize is 0x1f8 or
    // 0x1f4 (the two sizes of the structure); anything else stops. uiParam: cbSize (the page) or 0 (the guest's form); anything
    // else stops. fWinIni and a NULL pvParam stop.
    for cb in [0u32, 4, 0x1f0, 0x1f5, 0x1f7, 0x1f9, 0x1fc, 0x200, 0x28c, 0xFFFF_FFFF] {
        let e = bufcall(NCM, 0, false, Some(cb), 0xAA, 0, NCM_FULL).expect_err("cbSize");
        assert!(e.contains("SPI_GETNONCLIENTMETRICS") && e.contains("cbSize not 0x1f8 or 0x1f4"), "cbSize {cb:#x}: {e}");
    }
    for (cb, ui) in [(0x1f8u64, 0x1f4u64), (0x1f8, 1), (0x1f8, 0x5c), (0x1f8, 0x1f9), (0x1f4, 0x1f8), (0x1f4, 0x5c), (0x1f4, 0xFFFF_FFFF)] {
        let e = bufcall(NCM, ui, false, Some(cb as u32), 0xAA, 0, NCM_FULL).expect_err("uiParam");
        assert!(e.contains("SPI_GETNONCLIENTMETRICS") && e.contains("uiParam neither 0"), "cbSize {cb:#x} uiParam {ui:#x}: {e}");
    }
    let e = bufcall(NCM, 0, true, None, 0xAA, 0, NCM_FULL).expect_err("pvParam NULL");
    assert!(e.contains("SPI_GETNONCLIENTMETRICS") && e.contains("pvParam NULL"), "{e}");
    for ini in [1u64, 2, 3, 0xFFFF_FFFF] {
        let e = bufcall(NCM, 0, false, Some(0x1f8), 0xAA, ini, NCM_FULL).expect_err("fWinIni");
        assert!(e.contains("SPI_GETNONCLIENTMETRICS") && e.contains("fWinIni not zero"), "fWinIni {ini:#x}: {e}");
    }
    // both accepted uiParam forms for both sizes
    for (cb, ui) in [(0x1f8u32, 0u64), (0x1f8, 0x1f8), (0x1f4, 0), (0x1f4, 0x1f4)] {
        assert_eq!(bufcall(NCM, ui, false, Some(cb), 0xAA, 0, NCM_FULL).unwrap().0, 1, "cbSize {cb:#x} uiParam {ui:#x}");
    }
}

#[test]
fn the_neighbours_of_the_font_actions_still_stop_by_number() {
    // SPI_SETICONTITLELOGFONT 0x22, SPI_SETNONCLIENTMETRICS 0x2A, SPI_GETICONMETRICS 0x2D, SPI_SETICONMETRICS 0x2E: not used by the
    // guest, so not answered
    for idx in [0x22u64, 0x2A, 0x2D, 0x2E] {
        let e = ask(idx).expect_err(learn_name(idx).unwrap_or("neighbour"));
        assert!(e.contains(&format!("uiAction {idx:#x} is not answered")), "{idx:#x}: {e}");
    }
}

#[test]
fn the_guests_font_calls_are_the_forms_the_runtime_accepts() {
    let Some(dll) = real_dll() else { return };
    // 0x866414..0x866424: `mov edx,[rbp+0x288]` (= 0x5c, stored at 0x866405: c7 85 88 02 00 00 5c 00 00 00), `lea r8,[rbp+0x22c]`,
    // `xor r9,r9`, call
    assert_eq!(bytes_at(&dll, 0x866405, 10), [0xc7, 0x85, 0x88, 0x02, 0x00, 0x00, 0x5c, 0x00, 0x00, 0x00]);
    assert_eq!(bytes_at(&dll, 0x866414, 16), [0x8b, 0x95, 0x88, 0x02, 0x00, 0x00, 0x4c, 0x8d, 0x85, 0x2c, 0x02, 0x00, 0x00, 0x4d, 0x33, 0xc9]);
    // 0x866485 (f_8663b0), 0x840037 (f_840020): `mov ecx,0x29; xor edx,edx; lea r8,[rbp+0x34 | rbp+0x28]; xor r9,r9` = uiParam 0
    assert_eq!(bytes_at(&dll, 0x866485, 11), [0xb9, 0x29, 0, 0, 0, 0x33, 0xd2, 0x4c, 0x8d, 0x45, 0x34]);
    assert_eq!(bytes_at(&dll, 0x840037, 11), [0xb9, 0x29, 0, 0, 0, 0x33, 0xd2, 0x4c, 0x8d, 0x45, 0x28]);
    // 0x991d28: cbSize is the immediate 0x1f8 (c7 45 40 f8 01 00 00), then `mov ecx,0x29; xor edx,edx; lea r8,[rbp+0x40]; xor r9,r9`
    assert_eq!(bytes_at(&dll, 0x991d28, 7), [0xc7, 0x45, 0x40, 0xf8, 0x01, 0x00, 0x00]);
    assert_eq!(bytes_at(&dll, 0x991d2f, 11), [0xb9, 0x29, 0, 0, 0, 0x33, 0xd2, 0x4c, 0x8d, 0x45, 0x40]);
    // f_8663b0 and f_840020 take cbSize from f_425a50: `cmp eax,6; jl; mov eax,0x1f8 | mov eax,0x1f4`
    assert_eq!(bytes_at(&dll, 0x425a59, 13), [0x83, 0xf8, 0x06, 0x7c, 0x07, 0xb8, 0xf8, 0x01, 0x00, 0x00, 0xeb, 0x05, 0xb8]);
    assert_eq!(bytes_at(&dll, 0x425a65, 5), [0xb8, 0xf4, 0x01, 0x00, 0x00]);
}

// ---- 4. Source checks ----

#[test]
fn source_uses_no_host_settings_registry_mouse_theme_font_window_clock_thread_process_or_file() {
    let code = strip_comments(&std::fs::read_to_string(src_dir().join("fle_spi.c")).unwrap());
    for bad in [
        "getpid", "getppid", "gettid", "getenv", "extern char **environ", "fopen", "open(", "stat(", "/proc", "/etc", "/home", "/dev", "/tmp", "exit(", "_exit",
        "clock_gettime", "gettimeofday", "time(", "clock(", "nanosleep", "usleep", "sleep(", "select(", "poll(", "<time.h>", "<sys/time.h>",
        "pthread", "<sys/", "<unistd.h>", "<signal.h>", "fork(", "exec", "syscall", "sched_", "futex", "asm", "__asm__", "uintptr_t", "abort(",
        "X11", "DISPLAY", "gsettings", "xinput", "libinput", "dconf", "wayland", "ioctl", "emscripten", "SDL", "devicePixelRatio", "mouse0",
        "fle_host", "static int", "static uint64_t", "static uint32_t n", "fle_set_last_error", "fle_reg", "fle_vfs",
    ] {
        assert!(!code.contains(bad), "fle_spi.c must not use {bad}");
    }
    // `long` is not portable (32 bits on wasm32 and on Windows, 64 on Linux): fixed-width integers only
    let b = code.as_bytes();
    let mut i = 0;
    while let Some(p) = code[i..].find("long") {
        let at = i + p;
        let before = at == 0 || !(b[at - 1].is_ascii_alphanumeric() || b[at - 1] == b'_');
        let after = at + 4 >= b.len() || !(b[at + 4].is_ascii_alphanumeric() || b[at + 4] == b'_');
        assert!(!(before && after), "fle_spi.c uses `long` near byte {at}");
        i = at + 4;
    }
    // the value is named, never a second copy of the number; fle_stop is the stop; the only write is fle_st
    assert!(code.contains("FLE_SPI_WHEELSCROLLLINES") && code.contains("fle_stop(") && code.contains("fle_st("));
    // five places write, all through fle_st (decision 0058: the 4 bytes of the wheel lines; decision 0066: put_bytes, which writes
    // the fonts and the zeroed undecided runs, and the three scalar writes iBorderWidth, the four zeroed dimension ints and
    // iPaddedBorderWidth). A raw store through a pointer would not be counted here, so none is allowed:
    // decision 0098 added SPI_GETWORKAREA: the four ints of the RECT (left, top, right, bottom), four more places, 9 in all;
    // decision 0102 W13 added SPI_GETFONTSMOOTHINGCONTRAST: the one UINT, 10 in all; decision 0103 added SPI_GETFONTSMOOTHINGTYPE and
    // SPI_GETFONTSMOOTHINGORIENTATION, one UINT each, 12 in all; decision 0106 added SPI_GETCURSORSHADOW, one BOOL, 13 in all;
    // decision 0126 added SPI_GETANIMATION: iMinAnimate, one int, 14 in all (old: 13)
    assert_eq!(code.matches("fle_st(").count(), 14, "writes go through fle_st only, at these fourteen places");
    for bad in ["memcpy", "memset", "*(uint", "*(int", "->"] {
        assert!(!code.contains(bad), "fle_spi.c must not store through {bad}");
    }
    // the font is made of the named constants of the header, not second copies of the numbers
    for n in ["FLE_SPI_FONT_HEIGHT", "FLE_SPI_FONT_WEIGHT", "FLE_SPI_FONT_FACE", "FLE_SPI_BORDERWIDTH", "FLE_SPI_PADDEDBORDERWIDTH"] {
        assert!(code.contains(n), "fle_spi.c uses {n}");
    }
}

#[test]
fn header_is_plain_c() {
    let h = strip_comments(&std::fs::read_to_string(src_dir().join("fle_spi.h")).unwrap());
    for bad in ["asm", "__asm__", "<unistd.h>", "<sys/", "<pthread.h>", "<time.h>", "long "] {
        assert!(!h.contains(bad), "fle_spi.h must not contain {bad}");
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

/// A PE that imports `dll`!`api` (slot 0x2200). Entry 0x1000: `mov ecx,0x68; call 0x1040; ret`; 0x1040 is the thunk.
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
        (0x1000, vec![0xb9, 0x68, 0x00, 0x00, 0x00]),
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
    let img = build("user32.dll", "SystemParametersInfoW");
    let mut out = Vec::new();
    let st = generate(&img, &mut out).unwrap();
    let c = String::from_utf8(out).unwrap();
    assert_eq!(st.handled["jmp_imp:SystemParametersInfoW"], 1);
    assert!(c.contains("fle_imp_SystemParametersInfoW(); return;"));
    let va = IB + 0x1040;
    assert!(!c.contains(&format!("L_{va:x}: {{ /* {va:#x} Jmp */ fle_unsupported")), "{va:#x} still unsupported");
    assert_eq!(st.unsupported_total, 0, "{:?}", st.unsupported);
}

#[test]
fn the_dll_name_and_the_api_name_are_part_of_the_match() {
    // the same API name from another DLL, and the ANSI name from user32, are not this API: they stay unsupported
    for (dll, api) in [("other.dll", "SystemParametersInfoW"), ("user32.dll", "SystemParametersInfoA"), ("user32.dll", "SystemParametersInfoForDpi")] {
        let img = build(dll, api);
        let mut out = Vec::new();
        let st = generate(&img, &mut out).unwrap();
        let c = String::from_utf8(out).unwrap();
        assert!(!st.handled.contains_key("jmp_imp:SystemParametersInfoW"), "{dll}!{api}");
        assert!(!c.contains("fle_imp_SystemParametersInfoW"), "{dll}!{api}");
        let va = IB + 0x1040;
        assert!(c.contains(&format!("L_{va:x}: {{ /* {va:#x} Jmp */ fle_unsupported(UINT64_C({va:#x})); }}")), "{dll}!{api}");
    }
}

// ---- 6. The bytes of the real DLL that the investigation rests on ----

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

/// The file offset of `rva` (the section table is read here, not through fe-pe).
fn rva_off(dll: &[u8], rva: u32) -> usize {
    let lf = le32(dll, 0x3c) as usize;
    let nsec = le16(dll, lf + 6) as usize;
    let osz = le16(dll, lf + 20) as usize;
    for i in 0..nsec {
        let o = lf + 24 + osz + 40 * i;
        let (vs, sva, rs, rp) = (le32(dll, o + 8), le32(dll, o + 12), le32(dll, o + 16), le32(dll, o + 20));
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

/// The 22 call sites of the three thunks of user32!SystemParametersInfoW, from the objdump of the DLL: (call site, thunk, the
/// immediate of the `mov ecx,imm32` before it, the address of that mov). Every site is the form
/// `mov ecx,uiAction; xor edx,edx | mov edx,..; lea/mov r8,pvParam | xor r8,r8; xor r9,r9; call thunk`.
const SITES: [(u64, u64, u32, u64); 22] = [
    (0x61b8bb, 0x4255d0, 0x1016, 0x61b8ad), (0x61b8d4, 0x4255d0, 0x1018, 0x61b8c6), (0x6208ac, 0x4255e0, 0x68, 0x62089e),
    (0x6209f5, 0x4255e0, 0x68, 0x6209e7), (0x7fe328, 0x4255d0, 0x4b, 0x7fe31b), (0x815514, 0x4255d0, 0x101a, 0x815503),
    (0x818c8e, 0x4255d0, 0x48, 0x818c7c), (0x818cd7, 0x4255d0, 0x49, 0x818cc6), (0x8190a6, 0x4255d0, 0x30, 0x819099),
    (0x840045, 0x4255d0, 0x29, 0x840037), (0x866424, 0x4255d0, 0x1f, 0x86640f), (0x866493, 0x4255d0, 0x29, 0x866485),
    (0x866d1a, 0x4255f0, 0x30, 0x866d0c), (0x86723c, 0x4255d0, 0x48, 0x86722d), (0x86728c, 0x4255d0, 0x49, 0x86727d),
    (0x8673a5, 0x4255f0, 0x30, 0x867398), (0x991d3d, 0x4255d0, 0x29, 0x991d2f), (0x9f4c93, 0x4255d0, 0x26, 0x9f4c85),
    (0xa0a57b, 0x4255d0, 0x68, 0xa0a56d), (0xa11c2f, 0x4255d0, 0x1016, 0xa11c21), (0xa11c4c, 0x4255d0, 0x1018, 0xa11c3e),
    (0x11c3cd8, 0x4255d0, 0x30, 0x11c3cc7),
];

#[test]
fn the_three_thunks_all_jump_to_the_same_slot_which_is_user32_systemparametersinfow() {
    let Some(dll) = real_dll() else { return };
    let imps = imports(&dll);
    let slot = 0x19710d0u64;
    let named: Vec<_> = imps.iter().filter(|(_, _, s)| *s == slot).collect();
    assert_eq!(named.len(), 1);
    assert_eq!((named[0].0.to_lowercase(), named[0].1.as_str()), ("user32.dll".to_string(), "SystemParametersInfoW"));
    for thunk in [0x4255d0u64, 0x4255e0, 0x4255f0] {
        let t = bytes_at(&dll, thunk, 6);
        assert_eq!(&t[..2], [0xff, 0x25], "{thunk:#x}: jmp [rip+disp32]");
        let target = (thunk as i64 + 6 + i32::from_le_bytes(t[2..6].try_into().unwrap()) as i64) as u64;
        assert_eq!(target, slot, "{thunk:#x}");
    }
    // the guest imports no SystemParametersInfoA, and no other function with the name
    let spi: Vec<_> = imps.iter().filter(|(_, n, _)| n.contains("SystemParametersInfo")).collect();
    assert_eq!(spi.len(), 1, "{spi:?}");
}

#[test]
fn nothing_else_in_the_file_points_at_the_thunks_or_the_slot() {
    let Some(dll) = real_dll() else { return };
    // no 4- or 8-byte absolute VA (or the 4-byte RVA) of a thunk or of the slot is in the file: no table of function pointers
    // holds them, so the 22 direct call sites are all the references (an indirect call through a computed address is not seen)
    for va in [0x4255d0u64, 0x4255e0, 0x4255f0, 0x19710d0] {
        for pat in [(va as u32).to_le_bytes().to_vec(), va.to_le_bytes().to_vec(), ((va - 0x400000) as u32).to_le_bytes().to_vec()] {
            assert!(!dll.windows(pat.len()).any(|w| w == pat.as_slice()), "{va:#x}: {pat:02x?}");
        }
    }
}

#[test]
fn the_guest_call_sites_pass_the_uiactions_the_decision_lists() {
    let Some(dll) = real_dll() else { return };
    for (site, thunk, imm, mov) in SITES {
        let c = bytes_at(&dll, site, 5);
        assert_eq!(c[0], 0xe8, "{site:#x}: call");
        let target = (site as i64 + 5 + i32::from_le_bytes(c[1..5].try_into().unwrap()) as i64) as u64;
        assert_eq!(target, thunk, "{site:#x}: target");
        let m = bytes_at(&dll, mov, 5);
        assert_eq!(m[0], 0xb9, "{site:#x}: mov ecx,imm32");
        assert_eq!(u32::from_le_bytes(m[1..5].try_into().unwrap()), imm, "{site:#x}: uiAction");
        // fWinIni: `xor r9,r9` (4d 33 c9) immediately before the call, at every site
        assert_eq!(bytes_at(&dll, site - 3, 3), [0x4d, 0x33, 0xc9], "{site:#x}: fWinIni = 0");
    }
    // the path to the stop: f_620880 stores (GetSystemMetrics(0x4b) != 0) in [rbx+0x28] and, when true, calls
    // SystemParametersInfoW with uiAction 0x68, uiParam 0 (xor edx,edx), pvParam = rbx+0x14 (lea r8,[rbx+0x14]), fWinIni 0
    // (xor r9,r9). f_6209a0 (edx == 0x68 after `sub edx,1; sub edx,0x67`) makes the same call.
    for mov in [0x62089eu64, 0x6209e7] {
        assert_eq!(bytes_at(&dll, mov, 14), [0xb9, 0x68, 0, 0, 0, 0x33, 0xd2, 0x4c, 0x8d, 0x43, 0x14, 0x4d, 0x33, 0xc9]);
    }
    // the receiver of the path to the stop is the 4 bytes at [obj+0x14]; the two bytes before are the flags f_620860 / f_620880 set
    assert_eq!(bytes_at(&dll, 0x62089a, 4), [0x84, 0xc0, 0x74, 0x13], "test al,al; je 0x6208b1 (skip the call when there is no wheel)");
}

#[test]
fn every_static_uiaction_of_the_guest_is_answered_or_stops_by_name() {
    // ties the call-site list above to the runtime: each immediate is the answered one or in LISTED, and the C code does that
    let mut seen = std::collections::BTreeSet::new();
    for (_, _, imm, _) in SITES {
        seen.insert(imm as u64);
    }
    assert_eq!(seen.len(), 11);
    for i in seen {
        match (i == WHEEL, LISTED.iter().any(|(n, _)| *n == i)) {
            (true, false) => assert_eq!(ask(i), Ok((1, [0xAAAA_AAAA, 3, 0xCCCC_CCCC])), "{i:#x}"),
            (false, true) => {
                let e = ask(i).unwrap_err();
                assert!(e.contains(&format!("uiAction {i:#x} ({})", learn_name(i).unwrap())), "{i:#x}: {e}");
            }
            // the two font actions (decision 0066) are answered with the guest's own forms (section 3b: 0x1f with uiParam 0x5c,
            // 0x29 with the guest's uiParam 0 and its cbSize); they need buffers of 92 and 504 bytes, not the 3 dwords of ask()
            (false, false) if i == ICONFONT => assert_eq!(font_call(0x5c, 0xAA).unwrap().0, 1, "{i:#x}"),
            (false, false) if i == NCM => assert_eq!(ncm_call(0x1f8, 0, 0xAA).unwrap().0, 1, "{i:#x}"),
            // decision 0098: SPI_GETWORKAREA writes a RECT (16 bytes), not the 3 dwords of ask(); tests/wall_r.rs answers it with the guest's form
            (false, false) if i == WORKAREA => {}
            // decision 0126: SPI_GETANIMATION needs an ANIMATIONINFO with cbSize 8 and uiParam 8 (spi_getanimation_gives_an_animationinfo_...), not the plain form of ask()
            (false, false) if i == ANIMATION => assert_eq!(animation_call(8, 8, 0xAA).unwrap().0, 1, "{i:#x}"),
            // decision 0106: SPI_GETCURSORSHADOW writes one BOOL (FALSE, PROVISIONAL), in the middle dword of ask()'s buffer
            (false, false) if i == CURSORSHADOW => assert_eq!(ask(i), Ok((1, [0xAAAA_AAAA, 0, 0xCCCC_CCCC])), "{i:#x}"),
            // decision 0242: SPI_SETANIMATION keeps iMinAnimate (spi_setanimation_keeps_iminanimate_for_spi_getanimation); the guest's form is cbSize 8, uiParam 8
            (false, false) if i == 0x49 => assert_eq!(bufcall(0x49, 8, false, Some(8), 0xAA, 0, 8).unwrap().0, 1, "{i:#x}"),
            other => panic!("uiAction {i:#x} ({:?}): in {other:?} of the lists", learn_name(i)),
        }
    }
}
