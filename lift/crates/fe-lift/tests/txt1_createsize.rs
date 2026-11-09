//! Decision 0219 (txt1): the messages a window procedure receives during CreateWindowExW (harness/txt1/create_seq.c; scaffold of harness/api2/win_seq.c).
//!
//! The expected lines are `harness/txt1/expected/create-seq-wine-10.0.txt`, the output of the same cases run against Wine 10.0's user32 (mingw build, headless; the command is in the header of
//! harness/txt1/create_seq.c, `API2_WINE`): a WS_CHILD or WS_POPUP window gets `WM_SIZE(0, cx, cy)` and `WM_MOVE(x, y)` right after WM_CREATE, a WS_OVERLAPPEDWINDOW window that is not visible does not, and a window whose
//! WM_CREATE returns -1 gets neither. Wine is the only source (no real Windows was asked). No Wine is needed to run this test. The lines that the runtime still lacks for other reasons (the activation messages of a
//! visible pop-up, WM_GETMINMAXINFO of an overlapped window, WM_SHOWWINDOW(0) when a visible child is destroyed, WM_NCDESTROY after a failed WM_CREATE) are `expected/known-gaps.txt`: each is a gap of the window model
//! that this decision does not touch.

use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::Command;

mod etxt_common;

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
    lift_dir().join("harness/txt1")
}
fn work() -> PathBuf {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("txt1");
    std::fs::create_dir_all(&d).unwrap();
    d
}

const OBJS: [&str; 28] = [
    "fle_api2_call", "fle_appview", "fle_callback", "fle_class", "fle_comctl", "fle_cursor", "fle_env", "fle_event", "fle_gdc", "fle_gdi", "fle_host", "fle_imports", "fle_input", "fle_layered", "fle_locale", "fle_mxcsr",
    "fle_proc", "fle_raster", "fle_sys", "fle_thread", "fle_time", "fle_token", "fle_vfs", "fle_visrgn", "fle_vmap", "fle_vol", "fle_win", "fle_x87",
];
const OBJS2: [&str; 3] = ["fle_x87ctl", "fle_wall_ia2", "fle_wall_ia2w"];

fn run_variant(tag: &str, defs: &[&str]) -> String {
    let dir = work().join(etxt_common::per_process(tag));
    std::fs::create_dir_all(&dir).unwrap();
    let mut objs = Vec::new();
    for n in OBJS.iter().chain(OBJS2.iter()) {
        let o = dir.join(format!("{n}.o"));
        let cc = Command::new("clang")
            .args(["-O0", "-std=gnu11", "-w", "-fPIE", "-c", "-I"])
            .arg(rt())
            .args(defs)
            .arg(rt().join(format!("{n}.c")))
            .arg("-o")
            .arg(&o)
            .output()
            .expect("clang not found");
        assert!(cc.status.success(), "clang failed for {n}: {}", String::from_utf8_lossy(&cc.stderr));
        objs.push(o);
    }
    let exe = dir.join("create_seq");
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fPIE", "-pthread", "-I"]).arg(rt()).arg(harness().join("create_seq.c"));
    for o in &objs {
        cmd.arg(o);
    }
    let cc = cmd.arg("-o").arg(&exe).arg("-lm").output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for create_seq: {}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new(&exe).output().expect("run failed");
    assert!(r.status.success(), "create_seq stopped (signal {:?}): {}", r.status.signal(), String::from_utf8_lossy(&r.stderr).lines().last().unwrap_or(""));
    String::from_utf8_lossy(&r.stdout).to_string()
}

fn expected(p: &str) -> Vec<String> {
    std::fs::read_to_string(harness().join("expected").join(p)).unwrap().lines().map(|s| s.to_string()).collect()
}

/// The lines of `wine` that are not in `rt`, in order; `rt` must be a subsequence of `wine` (the runtime sends nothing that Wine does not).
fn missing(wine: &[String], rt: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut j = 0;
    for w in wine {
        if j < rt.len() && &rt[j] == w {
            j += 1;
        } else {
            out.push(w.clone());
        }
    }
    assert_eq!(j, rt.len(), "the runtime sent a line (or an order) that Wine 10.0 does not: first unmatched {:?}", rt.get(j));
    out
}

#[test]
fn creation_messages_equal_wine_10_except_the_listed_gaps() {
    let out = run_variant("on", &[]);
    let rt_lines: Vec<String> = out.lines().map(|s| s.to_string()).collect();
    let wine = expected("create-seq-wine-10.0.txt");
    let gaps = expected("known-gaps.txt");
    assert_eq!(missing(&wine, &rt_lines), gaps, "the lines Wine sends and the runtime does not must be exactly the listed gaps");
    let sizes = rt_lines.iter().filter(|l| l.contains("WM_SIZE") || l.contains("WM_MOVE")).count();
    assert_eq!(sizes, wine.iter().filter(|l| l.contains("WM_SIZE") || l.contains("WM_MOVE")).count(), "every WM_SIZE / WM_MOVE of the Wine list");
    assert!(sizes >= 14, "the cases send at least 7 SIZE/MOVE pairs (got {sizes} lines)");
}

