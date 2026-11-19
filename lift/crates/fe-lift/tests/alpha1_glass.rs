//! Decision 0244 (alpha1): translucent windows. harness/alpha1/glass_seq.c runs the runtime itself (fle_win.c, fle_gdi.c, fle_raster.c, fle_dwm.c, fle_desk.c): a window whose frame is extended into the client
//! area (DwmExtendFrameIntoClientArea) keeps the alpha byte that a BitBlt from a 32-bit DIB carries, the compositor puts its extended-frame pixels over what is below as premultiplied ARGB, SetWindowRgn clips what is shown and what the
//! pointer finds, and an ordinary window stays opaque with no alpha byte. The expected pixels are literals made by tools/mk-expect.py (docs/investigations/alpha1/tools/) from the formula of Microsoft Learn (BLENDFUNCTION), in exact
//! rational arithmetic; the program prints one line per check and ends with `checks N failed 0`.

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
fn harness() -> PathBuf {
    lift_dir().join("harness/alpha1")
}
fn work() -> PathBuf {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("alpha1_glass");
    std::fs::create_dir_all(&d).unwrap();
    d
}

const OBJS: [&str; 34] = [
    "fle_api2_call", "fle_appview", "fle_callback", "fle_class", "fle_comctl", "fle_cursor", "fle_env", "fle_event", "fle_gdc", "fle_gdi", "fle_host", "fle_imports", "fle_input", "fle_layered", "fle_locale", "fle_mxcsr",
    "fle_proc", "fle_raster", "fle_sys", "fle_thread", "fle_time", "fle_token", "fle_vfs", "fle_visrgn", "fle_vmap", "fle_vol", "fle_win", "fle_x87", "fle_x87ctl", "fle_wall_ia2", "fle_wall_ia2w", "fle_desk", "fle_dwm", "fle_monitor",
];

fn run_variant(tag: &str, defs: &[&str]) -> String {
    let dir = work().join(etxt_common::per_process(tag));
    std::fs::create_dir_all(&dir).unwrap();
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
    let exe = dir.join("glass_seq");
    let mut cmd = Command::new("clang");
    cmd.args(["-O1", "-std=gnu11", "-w", "-fPIE", "-pthread", "-I"]).arg(rt()).args(defs).arg(harness().join("glass_seq.c"));
    for o in &objs {
        cmd.arg(o);
    }
    let cc = cmd.arg("-o").arg(&exe).arg("-lm").output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for glass_seq: {}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new(&exe).output().expect("run failed");
    let out = String::from_utf8_lossy(&r.stdout).to_string();
    assert!(r.status.success(), "glass_seq reported a failure:\n{}", out.lines().filter(|l| l.starts_with("FAIL")).collect::<Vec<_>>().join("\n"));
    out
}

fn check(out: &str) {
    let fails: Vec<&str> = out.lines().filter(|l| l.starts_with("FAIL")).collect();
    assert!(fails.is_empty(), "failed checks: {fails:?}");
    let last = out.lines().rev().find(|l| l.starts_with("checks ")).expect("no summary line");
    assert!(last.ends_with("failed 0"), "{last}");
    let n: u32 = last.split_whitespace().nth(1).unwrap().parse().unwrap();
    assert!(n >= 256, "the program makes at least 243 checks, got {n}");
}

#[test]
fn extended_frame_alpha_region_and_hit_test() {
    check(&run_variant("on", &[]));
}
