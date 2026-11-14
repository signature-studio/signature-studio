//! Tests for the user32 keyboard-layout family found by the systematic sweep (decision 0215, sweep1; runtime/fle_sweep1_kbd.c, harness/sweep1/, harness/seeds/sweep1-imports.txt).
//!
//! The expected values are not read back from the C code: `harness/sweep1/expected/wine-10.0-kbd.txt` is the output of the same cases run against Wine 10.0's real APIs (mingw build, `build.sh wine`; Wine is
//! not Windows), the lines that differ on purpose are listed with their reason in `expected/known-divergence.txt`. No Wine is needed to run this test.

mod etxt_common;

use std::collections::BTreeSet;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn lift_dir() -> PathBuf {
    manifest().join("../..")
}
fn rt() -> PathBuf {
    lift_dir().join("runtime")
}
fn harness() -> PathBuf {
    lift_dir().join("harness/sweep1")
}
fn work() -> PathBuf {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("sweep1");
    std::fs::create_dir_all(&d).unwrap();
    d
}

const OBJS: [&str; 13] = [
    "fle_sweep1_kbd", "fle_api2_call", "fle_kbd", "fle_input", "fle_locale", "fle_time", "fle_glob", "fle_imports", "fle_env", "fle_vmap", "fle_proc", "fle_sys", "fle_vfs",
];

fn objs() -> &'static Vec<PathBuf> {
    static O: OnceLock<Vec<PathBuf>> = OnceLock::new();
    O.get_or_init(|| {
        let dir = work().join(etxt_common::per_process("obj"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut v = Vec::new();
        for n in OBJS {
            let o = dir.join(format!("{n}.o"));
            let mut cmd = Command::new("clang");
            cmd.args(["-O0", "-std=gnu11", "-Wall", "-Wextra", "-fPIE", "-c", "-I"]).arg(rt());
            if n.starts_with("fle_sweep1_") {
                cmd.arg("-Werror");
            } else {
                cmd.arg("-w");
            }
            let cc = cmd.arg(rt().join(format!("{n}.c"))).arg("-o").arg(&o).output().expect("clang not found");
            assert!(cc.status.success(), "clang failed for {n}: {}", String::from_utf8_lossy(&cc.stderr));
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

fn build_run(name: &str, src: &Path, args: &[&str]) -> Out {
    let exe = work().join(etxt_common::uniq(name));
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fPIE", "-pthread", "-I"]).arg(rt()).arg(src);
    for o in objs() {
        cmd.arg(o);
    }
    let cc = cmd.arg("-o").arg(&exe).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new(&exe).args(args).output().expect("run failed");
    Out { stdout: String::from_utf8_lossy(&r.stdout).to_string(), stderr: String::from_utf8_lossy(&r.stderr).to_string(), signal: r.status.signal(), ok: r.status.success() }
}

fn rows() -> Vec<(String, String, String)> {
    let t = std::fs::read_to_string(lift_dir().join("harness/seeds/sweep1-imports.txt")).unwrap();
    t.lines()
        .map(|l| l.split('#').next().unwrap().trim())
        .filter(|l| !l.is_empty())
        .map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            assert_eq!(f.len(), 3, "row {l}");
            (f[0].to_string(), f[1].to_string(), f[2].to_string())
        })
        .collect()
}

#[test]
fn rows_are_new_declared_and_defined() {
    let table = std::fs::read_to_string(manifest().join("src/imports_table.rs")).unwrap();
    let mut earlier = String::new();
    for f in ["ship2-imports.txt", "dec1-imports.txt", "api2-imports.txt", "act1-imports.txt", "crash1-imports.txt"] {
        if let Ok(t) = std::fs::read_to_string(lift_dir().join("harness/seeds").join(f)) {
            earlier.push_str(&t);
        }
    }
    let h = std::fs::read_to_string(rt().join("fle_sweep1.h")).unwrap();
    let defs = std::fs::read_to_string(rt().join("fle_sweep1_kbd.c")).unwrap();
    let mut seen = BTreeSet::new();
    let rs = rows();
    assert_eq!(rs.len(), 18, "18 rows");
    for (dll, api, f) in rs {
        assert_eq!(f, format!("fle_imp_{api}"));
        assert!(seen.insert((dll.clone(), api.clone())), "{api}: repeated");
        assert!(!table.contains(&format!("\"{api}\", \"fle_imp_{api}\"")), "{api}: already in TABLE (it would change an existing row)");
        assert!(!earlier.lines().any(|l| !l.starts_with('#') && l.split_whitespace().nth(1) == Some(api.as_str())), "{api}: already in an earlier rows file");
        assert!(h.contains(&format!("void fle_imp_{api}(void);")), "{api}: not declared in fle_sweep1.h");
        assert!(defs.contains(&format!("void fle_imp_{api}(void)\n")), "{api}: not defined");
    }
}

fn read(p: &str) -> String {
    std::fs::read_to_string(harness().join(p)).unwrap()
}

#[test]
fn cases_equal_the_wine_output_except_the_listed_divergences() {
    let o = build_run("sweep1-cases", &harness().join("sweep1_cases.c"), &[]);
    assert!(o.ok, "the cases stopped (signal {:?}): {}\n{}", o.signal, o.stderr.lines().last().unwrap_or(""), o.stdout);
    let mut div = std::collections::BTreeMap::new();
    for l in read("expected/known-divergence.txt").lines().filter(|l| !l.starts_with('#') && !l.trim().is_empty()) {
        let f: Vec<&str> = l.splitn(2, '\t').collect();
        assert_eq!(f.len(), 2, "divergence row {l}");
        assert!(!f[1].is_empty(), "a divergence needs its reason");
        div.insert(f[0].to_string(), f[1].to_string());
    }
    let wine: Vec<String> = read("expected/wine-10.0-kbd.txt").lines().map(|s| s.to_string()).collect();
    let nat: Vec<String> = o.stdout.lines().map(|s| s.to_string()).collect();
    assert_eq!(wine.len(), nat.len(), "number of cases");
    let mut used = BTreeSet::new();
    for (w, n) in wine.iter().zip(nat.iter()) {
        if let Some(k) = div.keys().find(|k| w.contains(k.as_str())) {
            used.insert(k.clone());
            continue;
        }
        assert_eq!(n, w, "case differs from Wine 10.0 and is not a listed divergence");
    }
    assert_eq!(used.len(), div.len(), "every listed divergence is used");
    // the native output is pinned too (the divergent lines included): they are the values written from the Microsoft pages
    let want: Vec<String> = read("expected/native-kbd.txt").lines().map(|s| s.to_string()).collect();
    assert_eq!(nat, want, "native output (expected values written from the Microsoft pages for the divergent lines)");
}

/// Inputs on which the pages give no behaviour stop by name (fle_stop), they are not turned into a success or a made-up failure.
#[test]
fn undefined_inputs_stop_by_name() {
    let src = r#"
#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include "fle_runtime.h"
#include "fle_api2.h"
#include "fle_sweep1.h"
uint64_t R[16];
void fle_win_input_activate(void) {} void fle_win_input_active(void) {} void fle_win_input_capture_any(void) {} void fle_win_input_hit(void) {} void fle_win_input_key_target(void) {}
void fle_win_input_send(void) {} void fle_win_input_view(void) {} void fle_win_pointer_get(void) {} void fle_win_pointer_set(void) {} void fle_win_host_pointer_set(void) {} void fle_win_pointer_reclamp(void) {}
void fle_win_input_toplevels(void) {} void fle_win_input_remaximize(void) {} void fle_win_input_showwindow(void) {}
void fle_win_input_call_hooks(void) {} void fle_win_input_hit_list(void) {}
int main(int argc, char **argv)
{
    int k = atoi(argv[1]);
    switch (k) {
    case 0: fle_api2_call(fle_imp_GetKeyboardLayoutNameW, 0, 0, 0, 0, 0, 0, 0, 0); break;
    case 1: fle_api2_call(fle_imp_GetKeyboardLayoutList, (uint64_t)(int64_t)-1, 0, 0, 0, 0, 0, 0, 0); break;
    case 2: fle_api2_call(fle_imp_GetKeyboardLayoutList, 1, 0, 0, 0, 0, 0, 0, 0); break;
    case 3: fle_api2_call(fle_imp_LoadKeyboardLayoutW, 0, 0, 0, 0, 0, 0, 0, 0); break;
    case 4: fle_api2_call(fle_imp_LoadKeyboardLayoutW, (uint64_t)(uintptr_t)L"00000409", 0x4, 0, 0, 0, 0, 0, 0); break;
    case 5: fle_api2_call(fle_imp_ActivateKeyboardLayout, 1, 0x4, 0, 0, 0, 0, 0, 0); break;
    case 6: fle_api2_call(fle_imp_ToUnicode, 0x41, 0, 0, 0, 4, 0, 0, 0); break;
    case 7: fle_api2_call(fle_imp_ToAscii, 0x41, 0, 0, 0, 0, 0, 0, 0); break;
    case 8: fle_api2_call(fle_imp_ToUnicodeEx, 0x41, 0, 0, 0, 4, 0, 0x12345678, 0); break;
    case 9: fle_api2_call(fle_imp_VkKeyScanExW, 0x41, 0x12345678, 0, 0, 0, 0, 0, 0); break;
    case 10: fle_api2_call(fle_imp_MapVirtualKeyExW, 0x41, 0, 0x12345678, 0, 0, 0, 0, 0); break;
    case 11: fle_api2_call(fle_imp_ToAsciiEx, 0x41, 0, 0, 0, 0, 0x12345678, 0, 0); break;
    }
    printf("returned\n");
    return 0;
}
"#;
    let c = work().join(format!("{}.c", etxt_common::uniq("sweep1-stops")));
    std::fs::write(&c, src).unwrap();
    let exe = work().join(etxt_common::uniq("sweep1-stops"));
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fPIE", "-pthread", "-I"]).arg(rt()).arg(&c);
    for o in objs() {
        cmd.arg(o);
    }
    let cc = cmd.arg("-o").arg(&exe).output().expect("clang not found");
    assert!(cc.status.success(), "{}", String::from_utf8_lossy(&cc.stderr));
    for (k, api) in ["GetKeyboardLayoutNameW", "GetKeyboardLayoutList", "GetKeyboardLayoutList", "LoadKeyboardLayoutW", "LoadKeyboardLayoutW", "ActivateKeyboardLayout", "ToUnicode", "ToAscii", "ToUnicodeEx", "VkKeyScanExW", "MapVirtualKeyExW", "ToAsciiEx"].iter().enumerate() {
        let r = Command::new(&exe).arg(k.to_string()).output().unwrap();
        let err = String::from_utf8_lossy(&r.stderr);
        assert_eq!(r.status.signal(), Some(6), "case {k} ({api}): not SIGABRT");
        assert!(err.contains(&format!("fle: {api}:")), "case {k}: no 'fle: {api}:' message: {err}");
        assert!(!String::from_utf8_lossy(&r.stdout).contains("returned"), "case {k}: returned instead of stopping");
    }
}

/// The rows must also be defined once in the default build's table sense: the file lists exactly the functions of fle_sweep1.h (no declared function without a row, except the shared table accessor).
#[test]
fn header_and_rows_agree() {
    let h = std::fs::read_to_string(rt().join("fle_sweep1.h")).unwrap();
    let mut declared: BTreeSet<String> = h.lines().filter_map(|l| l.strip_prefix("void fle_imp_").and_then(|r| r.split('(').next()).map(|s| s.to_string())).collect();
    declared.remove("MapVirtualKeyW"); // declared here only because fle_input.c does not export a prototype; its row is an earlier one
    let rowed: BTreeSet<String> = rows().into_iter().map(|r| r.1).collect();
    assert_eq!(declared, rowed);
}
