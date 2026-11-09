//! Decision 0245 (alpha2): a plain copy into a window surface that leaves the bytes as they were is not told to the compositor. harness/alpha2/wrote_seq.c runs the runtime itself (the production copy path) and
//! -Wl,--wrap=fle_desk_wrote: part 1 calls fle_r_blit between fle_r_changed_begin and fle_r_changed on surfaces (identical: 0; any pixel that differs, the first, the last, the fourth byte: 1 and the rows that differ; other
//! operations: unknown), part 2 draws into real windows (BitBlt) and counts the calls of fle_desk_wrote. The program prints one line per check and ends with `checks N failed 0`.

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
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("alpha2_changed");
    std::fs::create_dir_all(&d).unwrap();
    d
}

const OBJS: [&str; 34] = [
    "fle_api2_call", "fle_appview", "fle_callback", "fle_class", "fle_comctl", "fle_cursor", "fle_env", "fle_event", "fle_gdc", "fle_gdi", "fle_host", "fle_imports", "fle_input", "fle_layered", "fle_locale", "fle_mxcsr",
    "fle_proc", "fle_raster", "fle_sys", "fle_thread", "fle_time", "fle_token", "fle_vfs", "fle_visrgn", "fle_vmap", "fle_vol", "fle_win", "fle_x87", "fle_x87ctl", "fle_wall_ia2", "fle_wall_ia2w", "fle_desk", "fle_dwm", "fle_monitor",
];

#[test]
fn an_identical_copy_is_not_told_to_the_compositor_and_any_difference_is() {
    let dir = work().join(etxt_common::per_process("fast"));
    std::fs::create_dir_all(&dir).unwrap();
    let defs: [&str; 0] = [];
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
    let exe = dir.join("wrote_seq");
    let mut cmd = Command::new("clang");
    cmd.args(["-O1", "-std=gnu11", "-w", "-fPIE", "-pthread", "-I"]).arg(rt()).arg("-I").arg(lift_dir().join("harness/alpha1")).args(defs).arg(lift_dir().join("harness/alpha2/wrote_seq.c"));
    for o in &objs {
        cmd.arg(o);
    }
    let cc = cmd.arg("-Wl,--wrap=fle_desk_wrote").arg("-o").arg(&exe).arg("-lm").output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for wrote_seq: {}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new(&exe).output().expect("run failed");
    let out = String::from_utf8_lossy(&r.stdout).to_string();
    let fails: Vec<&str> = out.lines().filter(|l| l.starts_with("FAIL")).collect();
    assert!(r.status.success() && fails.is_empty(), "wrote_seq reported failures: {fails:?}");
    let last = out.lines().rev().find(|l| l.starts_with("checks ")).expect("no summary line");
    assert!(last.ends_with("failed 0"), "{last}");
    let n: u32 = last.split_whitespace().nth(1).unwrap().parse().unwrap();
    assert!(n >= 80, "the program makes at least 80 checks, got {n}");
}
