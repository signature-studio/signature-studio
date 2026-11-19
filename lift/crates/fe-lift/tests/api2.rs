//! Tests for the first block of the systematic API coverage (decision 0210, api2; runtime/fle_api2_*.c, harness/api2/, harness/seeds/api2-imports.txt).
//!
//! The expected values are not read back from the C code: `harness/api2/expected/wine-10.0-enUS.txt` is the output of the same cases run against Wine 10.0's real APIs (mingw build, `build.sh wine`; UNOFFICIAL, Wine
//! is not Windows), the lines that differ on purpose are listed with their reason in `expected/known-divergence.txt`, and `expected/native-only.txt` is written by hand from the Microsoft Learn pages
//! (GetLargePageMinimum, CreateSymbolicLinkW, OutputDebugStringA, ShellExecuteA/ExW: Wine is not a reference for them). No Wine is needed to run this test.

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
    lift_dir().join("harness/api2")
}
fn work() -> PathBuf {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("api2");
    std::fs::create_dir_all(&d).unwrap();
    d
}

const OBJS: [&str; 13] = [
    "fle_api2_call", "fle_api2_str", "fle_api2_sys", "fle_api2_shell", "fle_locale", "fle_time", "fle_glob", "fle_imports", "fle_env", "fle_vmap", "fle_proc", "fle_sys", "fle_vfs",
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
            if n.starts_with("fle_api2_") {
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

fn build_run(name: &str, src: &Path, extra: &[&str]) -> Out {
    let exe = work().join(etxt_common::uniq(name));
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fPIE", "-pthread", "-I"]).arg(rt()).args(extra).arg(src);
    for o in objs() {
        cmd.arg(o);
    }
    let cc = cmd.arg("-o").arg(&exe).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new(&exe).output().expect("run failed");
    Out { stdout: String::from_utf8_lossy(&r.stdout).to_string(), stderr: String::from_utf8_lossy(&r.stderr).to_string(), signal: r.status.signal(), ok: r.status.success() }
}

fn rows() -> Vec<(String, String, String)> {
    let t = std::fs::read_to_string(lift_dir().join("harness/seeds/api2-imports.txt")).unwrap();
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
    let ship = std::fs::read_to_string(lift_dir().join("harness/seeds/ship-imports.txt")).unwrap();
    let h = std::fs::read_to_string(rt().join("fle_api2.h")).unwrap();
    let mut defs = String::new();
    for f in ["fle_api2_str.c", "fle_api2_sys.c", "fle_api2_shell.c"] {
        defs.push_str(&std::fs::read_to_string(rt().join(f)).unwrap());
    }
    let mut seen = BTreeSet::new();
    let rs = rows();
    assert_eq!(rs.len(), 14, "14 rows");
    for (dll, api, f) in rs {
        assert_eq!(f, format!("fle_imp_{api}"));
        assert!(seen.insert((dll.clone(), api.clone())), "{api}: repeated");
        assert!(!table.contains(&format!("\"{api}\", \"fle_imp_{api}\"")), "{api}: already in TABLE (it would change an existing row)");
        assert!(!ship.lines().any(|l| !l.starts_with('#') && l.split_whitespace().nth(1) == Some(api.as_str())), "{api}: already in ship-imports.txt");
        assert!(h.contains(&format!("void fle_imp_{api}(void);")), "{api}: not declared in fle_api2.h");
        assert!(defs.contains(&format!("void fle_imp_{api}(void)\n")), "{api}: not defined");
    }
}

fn read(p: &str) -> String {
    std::fs::read_to_string(harness().join(p)).unwrap()
}

#[test]
fn cases_equal_the_wine_output_except_the_listed_divergences() {
    let o = build_run("api2-cases", &harness().join("api2_cases.c"), &[]);
    assert!(o.ok, "the cases stopped (signal {:?}): {}\n{}", o.signal, o.stderr.lines().last().unwrap_or(""), o.stdout);
    let mut div = std::collections::BTreeMap::new();
    for l in read("expected/known-divergence.txt").lines().filter(|l| !l.starts_with('#') && !l.trim().is_empty()) {
        let f: Vec<&str> = l.splitn(3, '\t').collect();
        assert_eq!(f.len(), 3, "divergence row {l}");
        assert!(!f[2].is_empty(), "a divergence needs its reason");
        div.insert(f[0].to_string(), f[1].to_string());
    }
    let wine: Vec<String> = read("expected/wine-10.0-enUS.txt").lines().map(|s| s.to_string()).collect();
    let nat: Vec<String> = o.stdout.lines().filter(|l| !l.starts_with("NATIVE ")).map(|s| s.to_string()).collect();
    assert_eq!(wine.len(), nat.len(), "number of cases");
    let mut used = 0;
    for (w, n) in wine.iter().zip(nat.iter()) {
        let want = match div.get(w) {
            Some(x) => {
                used += 1;
                x
            }
            None => w,
        };
        assert_eq!(n, want, "case differs from Wine 10.0 and is not a listed divergence");
    }
    assert_eq!(used, div.len(), "every listed divergence is used");
    let only: Vec<String> = o.stdout.lines().filter(|l| l.starts_with("NATIVE ")).map(|s| s.to_string()).collect();
    let want: Vec<String> = read("expected/native-only.txt").lines().map(|s| s.to_string()).collect();
    assert_eq!(only, want, "native-only cases (expected values written from the Microsoft pages)");
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
uint64_t R[16];
int main(int argc, char **argv)
{
    int k = atoi(argv[1]);
    switch (k) {
    case 0: fle_api2_call(fle_imp_CreateSymbolicLinkW, 0, 0, 0, 0, 0, 0, 0, 0); break;
    case 1: fle_api2_call(fle_imp_CreateSymbolicLinkW, 1, 1, 2, 0, 0, 0, 0, 0); break;
    case 2: fle_api2_call(fle_imp_TzSpecificLocalTimeToSystemTime, 0, 0, 0, 0, 0, 0, 0, 0); break;
    case 3: fle_api2_call(fle_imp_ShellExecuteA, 0, 0, 0, 0, 0, 0, 0, 0); break;
    case 4: fle_api2_call(fle_imp_ShellExecuteExW, 0, 0, 0, 0, 0, 0, 0, 0); break;
    }
    printf("returned\n");
    return 0;
}
"#;
    let c = work().join(format!("{}.c", etxt_common::uniq("api2-stops")));
    std::fs::write(&c, src).unwrap();
    let exe = work().join(etxt_common::uniq("api2-stops"));
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fPIE", "-pthread", "-I"]).arg(rt()).arg(&c);
    for o in objs() {
        cmd.arg(o);
    }
    let cc = cmd.arg("-o").arg(&exe).output().expect("clang not found");
    assert!(cc.status.success(), "{}", String::from_utf8_lossy(&cc.stderr));
    for (k, api) in ["CreateSymbolicLinkW", "CreateSymbolicLinkW", "TzSpecificLocalTimeToSystemTime", "ShellExecuteA", "ShellExecuteExW"].iter().enumerate() {
        let r = Command::new(&exe).arg(k.to_string()).output().unwrap();
        let err = String::from_utf8_lossy(&r.stderr);
        assert_eq!(r.status.signal(), Some(6), "case {k} ({api}): not SIGABRT");
        assert!(err.contains(&format!("fle: {api}:")), "case {k}: no 'fle: {api}:' message: {err}");
        assert!(!String::from_utf8_lossy(&r.stdout).contains("returned"), "case {k}: returned instead of stopping");
    }
}

const WIN_OBJS: [&str; 30] = [
    "fle_api2_call", "fle_appview", "fle_callback", "fle_class", "fle_comctl", "fle_cursor", "fle_env", "fle_event", "fle_gdc", "fle_gdi", "fle_host", "fle_imports", "fle_input", "fle_layered", "fle_locale", "fle_mxcsr", "fle_proc",
    "fle_raster", "fle_sys", "fle_thread", "fle_time", "fle_token", "fle_vfs", "fle_visrgn", "fle_vmap", "fle_vol", "fle_wall_ia2", "fle_wall_ia2w", "fle_win", "fle_x87",
];

/// The update-region cases of harness/api2/win_upd.c against the REAL runtime window layer (the scaffold of harness/web/g3: a stub guest window procedure that calls DefWindowProcW), compared with Wine 10.0's output
/// (`expected/win-upd-wine-10.0.txt`, UNOFFICIAL). The two lines on which the runtime differs are GAPS of its window model (parent without WS_CLIPCHILDREN does not propagate an invalidation to its children), listed in
/// `expected/win-known-divergence.txt`; when the runtime is fixed the divergence is unused and this test asks for the entry to be deleted.
#[test]
fn window_update_region_cases_vs_wine_with_the_known_gaps() {
    let dir = work().join(etxt_common::per_process("winobj"));
    std::fs::create_dir_all(&dir).unwrap();
    let mut objs = Vec::new();
    for n in WIN_OBJS.iter().chain(["fle_x87ctl"].iter()) {
        let o = dir.join(format!("{n}.o"));
        let cc = Command::new("clang").args(["-O0", "-std=gnu11", "-w", "-fPIE", "-c", "-I"]).arg(rt()).arg(rt().join(format!("{n}.c"))).arg("-o").arg(&o).output().expect("clang not found");
        assert!(cc.status.success(), "clang failed for {n}: {}", String::from_utf8_lossy(&cc.stderr));
        objs.push(o);
    }
    let exe = work().join(etxt_common::uniq("win_upd"));
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fPIE", "-pthread", "-I"]).arg(rt()).arg(harness().join("win_upd.c"));
    for o in &objs {
        cmd.arg(o);
    }
    let cc = cmd.arg("-o").arg(&exe).arg("-lm").output().expect("clang not found");
    assert!(cc.status.success(), "{}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new(&exe).output().unwrap();
    assert!(r.status.success(), "the window cases stopped: {}", String::from_utf8_lossy(&r.stderr).lines().last().unwrap_or(""));
    let nat: Vec<String> = String::from_utf8_lossy(&r.stdout).lines().map(|s| s.to_string()).collect();
    let mut wine: Vec<String> = Vec::new();
    let all: Vec<String> = read("expected/win-upd-wine-10.0.txt").lines().map(|s| s.to_string()).collect();
    let mut i = 0;
    while i < all.len() {
        if all[i].starts_with("after ScrollWindowEx(") {
            i += 2; // the Wine-only pair (the runtime's ScrollWindowEx needs a drawing surface)
            continue;
        }
        wine.push(all[i].clone());
        i += 1;
    }
    let mut div = std::collections::BTreeMap::new();
    for l in read("expected/win-known-divergence.txt").lines().filter(|l| !l.starts_with('#') && !l.trim().is_empty()) {
        let f: Vec<&str> = l.splitn(3, '\t').collect();
        assert_eq!(f.len(), 3);
        div.insert(f[0].to_string(), f[1].to_string());
    }
    assert_eq!(wine.len(), nat.len(), "number of lines");
    let mut used = 0;
    for (w, n) in wine.iter().zip(nat.iter()) {
        let want = match div.get(w) {
            Some(x) => {
                used += 1;
                x
            }
            None => w,
        };
        assert_eq!(n, want, "differs from Wine 10.0 and is not a listed gap");
    }
    assert_eq!(used, div.len(), "a listed gap no longer appears: the runtime was fixed, delete the entry");
}
