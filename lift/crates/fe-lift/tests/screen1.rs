//! Tests for the size of the virtual screen as a value of the run (decision 0212, runtime/fle_screen.h): FLE_MONITOR_W / FLE_MONITOR_H / FLE_SCREEN_DPI in the engine's environment.
//!
//! The answers of user32!GetSystemMetrics (SM_CXSCREEN, SM_CYSCREEN, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN) and gdi32!GetDeviceCaps (HORZRES, VERTRES, HORZSIZE, VERTSIZE, LOGPIXELSX, LOGPIXELSY) are the
//! two files that name the screen by FLE_GDC_HORZRES / FLE_GDC_VERTRES; the rest of the runtime uses the same macros (decision 0212 section 2). Expected values are written here from the rule (HORZSIZE =
//! pixels * 254 / (dpi * 10), rounded to the nearest), not read back from the C code. Without the variables every answer is the constant of decisions 0053 and 0057 (1920 x 1080, 508 x 286 mm, 96 DPI):
//! that is what the other test files pin down; the first test here pins it down once more through the environment-reading path. Nothing is compared with a real Windows.

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
fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-screen1-test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

fn clang_c(src: &Path, obj: &Path) {
    let cc = Command::new("clang")
        .args(["-O0", "-std=gnu11", "-Wall", "-Wextra", "-Werror", "-fPIE", "-c"])
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

/// The two files that answer the screen size by the macros, and the two they link with (fle_stop; GetLastError).
fn objs() -> &'static Vec<PathBuf> {
    static O: OnceLock<Vec<PathBuf>> = OnceLock::new();
    O.get_or_init(|| {
        let dir = work_dir().join(etxt_common::per_process("obj-screen1"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut v = Vec::new();
        for name in ["fle_gdc.c", "fle_sysmetrics.c", "fle_imports.c", "fle_env.c"] {
            let o = dir.join(name.replace(".c", ".o"));
            clang_c(&runtime_dir().join(name), &o);
            v.push(o);
        }
        v
    })
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
#include "fle_sysmetrics.h"
#include "fle_screen.h"

uint64_t R[16];
static uint64_t stk[32];

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
#define SM(i) ((int32_t)(uint32_t)CALL(fle_imp_GetSystemMetrics, (i)))
#define CAPS(dc, idx) ((int32_t)(uint32_t)CALL(fle_imp_GetDeviceCaps, (dc), (idx)))

/* one line: the four screen metrics, then HORZRES VERTRES HORZSIZE VERTSIZE LOGPIXELSX LOGPIXELSY of the screen DC */
static void report(void)
{
    uint64_t dc = CALL(fle_imp_GetDC, 0);
    printf("%d %d %d %d | %d %d %d %d %d %d\n", SM(0), SM(1), SM(78), SM(79), CAPS(dc, 8), CAPS(dc, 10), CAPS(dc, 4), CAPS(dc, 6), CAPS(dc, 88), CAPS(dc, 90));
}
"#;

fn run(name: &str, env: &[(&str, &str)], body: &str) -> Vec<String> {
    let dir = work_dir();
    let uname = etxt_common::uniq(name);
    let c = dir.join(format!("{uname}.c"));
    let exe = dir.join(&uname);
    std::fs::write(&c, format!("{PRELUDE}\nint main(void)\n{{\n{body}\n    return 0;\n}}\n")).unwrap();
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fPIE", "-pthread", "-I"]).arg(runtime_dir());
    for o in objs() {
        cmd.arg(o);
    }
    let cc = cmd.arg("-o").arg(&exe).arg(&c).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    let mut r = Command::new(&exe);
    r.env_remove("FLE_MONITOR_W").env_remove("FLE_MONITOR_H").env_remove("FLE_SCREEN_DPI").env("TMPDIR", dir.join("tmp"));
    for (k, v) in env {
        r.env(k, v);
    }
    let o = r.output().expect("run failed");
    assert!(o.status.success(), "{name} did not exit normally (signal {:?}): {}", o.status.signal(), String::from_utf8_lossy(&o.stderr));
    String::from_utf8_lossy(&o.stdout).lines().map(|s| s.to_string()).collect()
}

/// pixels * 25.4 / dpi millimetres, rounded to the nearest, in integers (pixels * 254 / (dpi * 10)): the rule of decision 0121 for any DPI.
fn mm(px: u32, dpi: u32) -> u32 {
    (px * 254 + dpi * 5) / (dpi * 10)
}

#[test]
fn without_the_variables_every_answer_is_the_constant_of_decisions_0053_and_0057() {
    // 1920 x 1080, 508 x 286 mm (285.75 rounded), 96 DPI
    assert_eq!((mm(1920, 96), mm(1080, 96)), (508, 286));
    assert_eq!(run("screen1-default", &[], "    report();"), ["1920 1080 1920 1080 | 1920 1080 508 286 96 96"]);
}

#[test]
fn the_variables_set_the_screen_everywhere_the_two_files_answer_it() {
    let (w, h) = (2400u32, 1300u32);
    let want = format!("{w} {h} {w} {h} | {w} {h} {} {} 96 96", mm(w, 96), mm(h, 96));
    assert_eq!(want, "2400 1300 2400 1300 | 2400 1300 635 344 96 96");
    assert_eq!(run("screen1-2400", &[("FLE_MONITOR_W", "2400"), ("FLE_MONITOR_H", "1300")], "    report();"), [want]);
}

#[test]
fn the_dpi_is_96_whatever_the_environment_says_and_the_physical_size_follows_it() {
    // decision 0243: the page has no DPI choice and the runtime reads no FLE_SCREEN_DPI: 96 (100%), and the millimetres follow the size
    let want = format!("3240 1923 3240 1923 | 3240 1923 {} {} 96 96", mm(3240, 96), mm(1923, 96));
    assert_eq!(run("screen1-3240", &[("FLE_MONITOR_W", "3240"), ("FLE_MONITOR_H", "1923")], "    report();"), [want]);
    assert_eq!(run("screen1-dpi-ignored", &[("FLE_SCREEN_DPI", "192")], "    report();"), [format!("1920 1080 1920 1080 | 1920 1080 {} {} 96 96", mm(1920, 96), mm(1080, 96))]);
    assert_eq!((mm(1920, 96), mm(1080, 96)), (508, 286));
}

#[test]
fn a_value_that_is_not_a_whole_number_of_decimal_digits_or_is_out_of_range_is_ignored_dimension_by_dimension() {
    let d = "1920 1080 1920 1080 | 1920 1080 508 286 96 96";
    for (w, h) in [("abc", "abc"), ("", ""), ("2400 ", "1300 "), ("-2400", "-1300"), ("2400.5", "1300.5"), ("0x960", "0x514"), ("639", "479"), ("7681", "4321"), ("99999999999", "99999999999")] {
        assert_eq!(run("screen1-bad", &[("FLE_MONITOR_W", w), ("FLE_MONITOR_H", h)], "    report();"), [d], "W={w:?} H={h:?}");
    }
    // each dimension is judged alone: a good width with a bad height keeps the good width
    assert_eq!(run("screen1-half", &[("FLE_MONITOR_W", "2400"), ("FLE_MONITOR_H", "abc")], "    report();"), [format!("2400 1080 2400 1080 | 2400 1080 {} 286 96 96", mm(2400, 96))]);
    // the limits themselves are accepted; leading zeros are digits
    assert_eq!(run("screen1-min", &[("FLE_MONITOR_W", "640"), ("FLE_MONITOR_H", "480")], "    report();"), [format!("640 480 640 480 | 640 480 {} {} 96 96", mm(640, 96), mm(480, 96))]);
    assert_eq!(run("screen1-max", &[("FLE_MONITOR_W", "7680"), ("FLE_MONITOR_H", "4320")], "    report();"), [format!("7680 4320 7680 4320 | 7680 4320 {} {} 96 96", mm(7680, 96), mm(4320, 96))]);
    assert_eq!(run("screen1-zeros", &[("FLE_MONITOR_W", "02400"), ("FLE_MONITOR_H", "01300")], "    report();"), [format!("2400 1300 2400 1300 | 2400 1300 {} {} 96 96", mm(2400, 96), mm(1300, 96))]);
}

#[test]
fn a_change_while_the_engine_runs_is_seen_by_every_answer_and_a_size_outside_the_limits_changes_nothing() {
    let body = r#"
    report();
    printf("set %d\n", fle_screen_set(1000, 700));
    report();
    printf("set %d\n", fle_screen_set(639, 700));
    printf("set %d\n", fle_screen_set(1000, 4321));
    printf("set %d\n", fle_screen_set(7681, 4320));
    report();
    printf("set %d\n", fle_screen_set(640, 480));
    report();
"#;
    let got = run("screen1-set", &[("FLE_MONITOR_W", "2400"), ("FLE_MONITOR_H", "1300")], body);
    assert_eq!(
        got,
        [
            format!("2400 1300 2400 1300 | 2400 1300 {} {} 96 96", mm(2400, 96), mm(1300, 96)),
            "set 1".to_string(),
            format!("1000 700 1000 700 | 1000 700 {} {} 96 96", mm(1000, 96), mm(700, 96)),
            "set 0".to_string(),
            "set 0".to_string(),
            "set 0".to_string(),
            format!("1000 700 1000 700 | 1000 700 {} {} 96 96", mm(1000, 96), mm(700, 96)),
            "set 1".to_string(),
            format!("640 480 640 480 | 640 480 {} {} 96 96", mm(640, 96), mm(480, 96)),
        ]
    );
}

#[test]
fn the_header_reads_only_the_engines_own_variables_and_nothing_of_the_host_display() {
    let h = std::fs::read_to_string(runtime_dir().join("fle_screen.h")).unwrap();
    let mut code = String::new();
    let mut in_comment = false;
    for l in h.lines() {
        let mut s = l.to_string();
        if in_comment {
            if let Some(p) = s.find("*/") {
                s = s[p + 2..].to_string();
                in_comment = false;
            } else {
                continue;
            }
        }
        while let Some(p) = s.find("/*") {
            if let Some(q) = s[p..].find("*/") {
                s.replace_range(p..p + q + 2, "");
            } else {
                s.truncate(p);
                in_comment = true;
            }
        }
        code.push_str(&s);
        code.push('\n');
    }
    let names: Vec<&str> = code.match_indices("getenv(").map(|(i, _)| &code[i..i + 40]).collect();
    // getenv is called with a parameter in fle_screen_dim only (decision 0243: the DPI and the flag readers are gone); the names are the literal arguments
    assert_eq!(names.len(), 1, "{names:?}");
    for lit in ["\"FLE_MONITOR_W\"", "\"FLE_MONITOR_H\""] {
        assert!(code.contains(lit), "{lit}");
    }
    for bad in ["X11", "DISPLAY", "xrandr", "wayland", "/dev", "/sys", "/proc", "ioctl", "fb0", "devicePixelRatio", "screen.width", "emscripten", "getpid", "clock_gettime", "fopen", "pthread"] {
        assert!(!code.contains(bad), "fle_screen.h must not contain {bad}");
    }
}
