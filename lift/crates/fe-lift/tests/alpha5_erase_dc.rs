//! Decision 0253 (alpha5): the WM_ERASEBKGND that RedrawWindow(RDW_ERASENOW / RDW_UPDATENOW) sends carries a device context in wParam (Learn WM_ERASEBKGND: "wParam: A handle to the device context"). The C tool
//! harness/alpha5/erase_dc.c runs the runtime with a window procedure of the shape of FL's TEdgeBrowser (FillRect(wParam, ...)) and prints one line per check and ends with `checks N failed 0`.

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
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("alpha5_erase_dc");
    std::fs::create_dir_all(&d).unwrap();
    d
}

const OBJS: [&str; 34] = [
    "fle_api2_call", "fle_appview", "fle_callback", "fle_class", "fle_comctl", "fle_cursor", "fle_env", "fle_event", "fle_gdc", "fle_gdi", "fle_host", "fle_imports", "fle_input", "fle_layered", "fle_locale", "fle_mxcsr",
    "fle_proc", "fle_raster", "fle_sys", "fle_thread", "fle_time", "fle_token", "fle_vfs", "fle_visrgn", "fle_vmap", "fle_vol", "fle_win", "fle_x87", "fle_x87ctl", "fle_wall_ia2", "fle_wall_ia2w", "fle_desk", "fle_dwm", "fle_monitor",
];

#[test]
fn redraw_window_erase_message_has_a_dc() {
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
    let exe = dir.join("erase_dc");
    let mut cmd = Command::new("clang");
    cmd.args(["-O1", "-std=gnu11", "-w", "-fPIE", "-pthread", "-I"]).arg(rt()).arg("-I").arg(lift_dir().join("harness/alpha1")).args(defs).arg(lift_dir().join("harness/alpha5/erase_dc.c"));
    for o in &objs {
        cmd.arg(o);
    }
    let cc = cmd.arg("-o").arg(&exe).arg("-lm").output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for erase_dc: {}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new(&exe).output().expect("run failed");
    let out = String::from_utf8_lossy(&r.stdout).to_string();
    let fails: Vec<&str> = out.lines().filter(|l| l.starts_with("FAIL")).collect();
    assert!(r.status.success() && fails.is_empty(), "erase_dc reported failures: {fails:?}");
    let last = out.lines().rev().find(|l| l.starts_with("checks ")).expect("no summary line");
    assert!(last.ends_with("failed 0"), "{last}");
    let n: u32 = last.split_whitespace().nth(1).unwrap().parse().unwrap();
    assert!(n >= 13, "the program makes at least 13 checks, got {n}");
}
