//! Decision 0262 (alpha8): the move of a child window copies its bits and invalidates only what the move uncovered, also when a hidden window of the tree holds a stale update rectangle (S-0247-hidden), and RDW_NOCHILDREN is
//! 0x40 (RDW_NOFRAME 0x800). The C tool harness/alpha8/hidden_move.c runs the runtime on the window tree of the Mixer drag (the real rectangles), prints one line per check and ends with `checks N failed 0`.

mod etxt_common;

use std::path::PathBuf;
use std::process::Command;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn lift_dir() -> PathBuf {
    manifest().join("../..")
}
fn rt() -> PathBuf {
    lift_dir().join("runtime")
}
fn work() -> PathBuf {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("alpha8_hidden_move");
    std::fs::create_dir_all(&d).unwrap();
    d
}

const OBJS: [&str; 34] = [
    "fle_api2_call", "fle_appview", "fle_callback", "fle_class", "fle_comctl", "fle_cursor", "fle_env", "fle_event", "fle_gdc", "fle_gdi", "fle_host", "fle_imports", "fle_input", "fle_layered", "fle_locale", "fle_mxcsr",
    "fle_proc", "fle_raster", "fle_sys", "fle_thread", "fle_time", "fle_token", "fle_vfs", "fle_visrgn", "fle_vmap", "fle_vol", "fle_win", "fle_x87", "fle_x87ctl", "fle_wall_ia2", "fle_wall_ia2w", "fle_desk", "fle_dwm", "fle_monitor",
];

#[test]
fn a_hidden_window_does_not_stop_the_copy_of_a_moved_child_and_rdw_nochildren_is_0x40() {
    let dir = work().join(etxt_common::per_process("fast"));
    std::fs::create_dir_all(&dir).unwrap();
    let defs = ["-DFLE_DESKTOP", "-DFLE_R_FAST_COPY=1"];
    let mut objs = Vec::new();
    for n in OBJS.iter() {
        let o = dir.join(format!("{n}.o"));
        let cc = Command::new("clang")
            .args(["-O1", "-std=gnu11", "-w", "-fPIE", "-c", "-I"])
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
    let exe = dir.join("hidden_move");
    let mut cmd = Command::new("clang");
    cmd.args(["-O1", "-std=gnu11", "-w", "-fPIE", "-pthread", "-I"]).arg(rt()).args(defs).arg(lift_dir().join("harness/alpha8/hidden_move.c"));
    for o in &objs {
        cmd.arg(o);
    }
    let cc = cmd.arg("-o").arg(&exe).arg("-lm").output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for hidden_move: {}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new(&exe).output().expect("run failed");
    let out = String::from_utf8_lossy(&r.stdout).to_string();
    let fails: Vec<&str> = out.lines().filter(|l| l.starts_with("FAIL")).collect();
    assert!(r.status.success() && fails.is_empty(), "hidden_move reported failures: {fails:?}");
    let last = out.lines().rev().find(|l| l.starts_with("checks ")).expect("no summary line");
    assert!(last.ends_with("failed 0"), "{last}");
    let n: u32 = last.split_whitespace().nth(1).unwrap().parse().unwrap();
    assert!(n >= 20, "the program makes at least 20 checks, got {n}");
}
