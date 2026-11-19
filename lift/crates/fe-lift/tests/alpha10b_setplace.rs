//! Decision 0267 (alpha10b): GetWindowPlacement takes the SDK's length (44, FL passes it) and SetWindowPlacement exists: Learn SetWindowPlacement "Sets the show state and the restored, minimized, and maximized positions of the specified
//! window". The C tool harness/alpha10/setplace_seq.c (and syscmd_seq.c for the values GetWindowPlacement reports) runs the runtime and prints the messages and the state after each step; the same programs on Wine 10.0 (UNOFFICIAL) give
//! the states asserted here (the message flags and the activation order of the two differ as they do for ShowWindow: docs/0267).

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
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("alpha10b_setplace");
    std::fs::create_dir_all(&d).unwrap();
    d
}

const OBJS: [&str; 36] = [
    "fle_api2_call", "fle_appview", "fle_callback", "fle_class", "fle_comctl", "fle_cursor", "fle_env", "fle_event", "fle_gdc", "fle_gdi", "fle_host", "fle_imports", "fle_input", "fle_layered", "fle_locale", "fle_mxcsr",
    "fle_proc", "fle_raster", "fle_sys", "fle_thread", "fle_time", "fle_token", "fle_vfs", "fle_visrgn", "fle_vmap", "fle_vol", "fle_win", "fle_x87", "fle_x87ctl", "fle_wall_ia2", "fle_wall_ia2w", "fle_desk", "fle_dwm", "fle_monitor",
    "fle_sysmetrics", "fle_spi",
];

fn run(name: &str) -> String {
    let dir = work().join(etxt_common::per_process(name));
    std::fs::create_dir_all(&dir).unwrap();
    let defs = ["-DFLE_DESKTOP", "-DFLE_R_FAST_COPY=1"];
    let mut objs = Vec::new();
    for n in OBJS.iter() {
        let o = dir.join(format!("{n}.o"));
        let cc = Command::new("clang").args(["-O1", "-std=gnu11", "-w", "-fPIE", "-c", "-I"]).arg(rt()).args(defs).arg(rt().join(format!("{n}.c"))).arg("-o").arg(&o).output().expect("clang not found");
        assert!(cc.status.success(), "clang failed for {n}: {}", String::from_utf8_lossy(&cc.stderr));
        objs.push(o);
    }
    let exe = dir.join(name);
    let mut cmd = Command::new("clang");
    cmd.args(["-O1", "-std=gnu11", "-w", "-fPIE", "-pthread", "-I"]).arg(rt()).args(defs).arg(lift_dir().join(format!("harness/alpha10/{name}.c")));
    for o in &objs {
        cmd.arg(o);
    }
    let cc = cmd.arg("-o").arg(&exe).arg("-lm").output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new(&exe).output().expect("run failed");
    assert!(r.status.success(), "{name} failed");
    String::from_utf8_lossy(&r.stdout).to_string()
}

fn section(out: &str, head: &str) -> Vec<String> {
    let mut it = out.lines().skip_while(|l| !l.starts_with(head));
    it.next().unwrap_or_else(|| panic!("no section {head}: {out}"));
    it.take_while(|l| l.starts_with("  ")).map(|l| l.trim().to_string()).collect()
}
fn state(s: &[String]) -> String {
    s.iter().find(|l| l.starts_with("state:")).cloned().unwrap_or_default()
}

#[test]
fn set_window_placement_moves_the_window_then_shows_it_and_get_window_placement_takes_44() {
    let out = run("setplace_seq");
    // a hidden window with SW_HIDE: moved and sized to the restored rectangle (also when hidden), still hidden
    let h = section(&out, "SetWindowPlacement(top, SW_HIDE, the rectangle of the form)");
    assert!(h.iter().any(|l| l == "returned 1"), "{h:?}");
    assert!(h.iter().any(|l| l.starts_with("top WM_MOVE x=-5 y=-6")) && h.iter().any(|l| l == "top WM_SIZE wp=0 cx=1930 cy=1092"), "{h:?}");
    let st = state(&h);
    assert!(st.contains("visible=0") && st.contains("window=(-5,-6,1925,1086)") && st.contains("normal=(-5,-6,1925,1086)") && st.contains("showCmd=1"), "{st}");
    // and ShowWindow(SW_SHOWNORMAL) then shows it where it was put
    let s = section(&out, "ShowWindow(top, SW_SHOWNORMAL) after that");
    assert!(state(&s).contains("visible=1") && state(&s).contains("window=(-5,-6,1925,1086)"), "{s:?}");
    // a visible window: SW_SHOWNORMAL moves it; SW_SHOWMAXIMIZED stores the rectangle and maximizes; SW_SHOWNORMAL of the maximized window restores to the new rectangle
    let n = section(&out, "SetWindowPlacement(two, SW_SHOWNORMAL, a new rectangle)");
    assert!(state(&n).contains("window=(10,20,310,220)"), "{n:?}");
    let m = section(&out, "SetWindowPlacement(two, SW_SHOWMAXIMIZED, a new rectangle)");
    assert!(m.iter().any(|l| l == "top WM_GETMINMAXINFO maxsize=1920,1080 maxpos=0,0") || m.iter().any(|l| l == "two WM_GETMINMAXINFO maxsize=1920,1080 maxpos=0,0"), "{m:?}");
    assert!(state(&m).contains("zoomed=1") && state(&m).contains("normal=(30,40,330,240)"), "{m:?}");
    let r = section(&out, "SetWindowPlacement(two, SW_SHOWNORMAL of the maximized window)");
    assert!(state(&r).contains("zoomed=0") && state(&r).contains("window=(50,60,350,260)"), "{r:?}");
    // minimize, restore, WPF_RESTORETOMAXIMIZED
    let mi = section(&out, "SetWindowPlacement(two, SW_SHOWMINIMIZED)");
    assert!(state(&mi).contains("iconic=1") && state(&mi).contains("showCmd=2") && state(&mi).contains("minpos=(-32000,-32000)") && state(&mi).contains("normal=(50,60,350,260)"), "{mi:?}");
    let re = section(&out, "SetWindowPlacement(two, SW_SHOWNORMAL of the minimized window)");
    assert!(state(&re).contains("iconic=0") && state(&re).contains("window=(70,80,370,280)"), "{re:?}");
    let rm = section(&out, "SetWindowPlacement(two, SW_SHOWMINIMIZED with WPF_RESTORETOMAXIMIZED)");
    assert!(state(&rm).contains("iconic=1") && state(&rm).contains("placement: flags=2"), "{rm:?}");
    let rz = section(&out, "SetWindowPlacement(two, SW_SHOWNORMAL of that minimized window (WPF_RESTORETOMAXIMIZED is not set))");
    assert!(state(&rz).contains("zoomed=1") && state(&rz).contains("iconic=0"), "{rz:?}");
    let hd = section(&out, "SetWindowPlacement(two, SW_HIDE of the visible window)");
    assert!(state(&hd).contains("visible=0"), "{hd:?}");
    // hidden windows with a show state: shown at the rectangle, maximized, minimized
    let a = section(&out, "SetWindowPlacement(top, a hidden window, SW_SHOWNORMAL)");
    assert!(state(&a).contains("visible=1") && state(&a).contains("window=(5,6,205,106)"), "{a:?}");
    let b = section(&out, "SetWindowPlacement(top, a hidden window, SW_SHOWMAXIMIZED)");
    assert!(state(&b).contains("visible=1") && state(&b).contains("window=(0,0,1920,1080)") && state(&b).contains("normal=(5,6,205,106)"), "{b:?}");
    let c = section(&out, "SetWindowPlacement(top, a hidden window, SW_SHOWMINIMIZED)");
    assert!(state(&c).contains("iconic=1") && state(&c).contains("normal=(5,6,205,106)"), "{c:?}");
    // a length that is not sizeof(WINDOWPLACEMENT) = 44 fails
    let w = section(&out, "SetWindowPlacement with a wrong length (60)");
    assert!(w.iter().any(|l| l == "returned 0"), "{w:?}");
    // GetWindowPlacement with length 44: the values (syscmd_seq prints them through the 44-byte structure)
    let sc = run("syscmd_seq");
    let pl = section(&sc, "SendMessage(top, WM_SYSCOMMAND, SC_MINIMIZE):");
    let st = state(&pl);
    assert!(st.contains("placement: flags=0 showCmd=2 minpos=(-32000,-32000) maxpos=(-1,-1) normal=(100,100,400,300)"), "{st}");
    let mx = section(&sc, "SendMessage(top, WM_SYSCOMMAND, SC_MINIMIZE of the maximized window):");
    assert!(state(&mx).contains("placement: flags=2 showCmd=2"), "{}", state(&mx));
    let first = sc.lines().find(|l| l.contains("state:")).unwrap();
    assert!(first.contains("showCmd=1 minpos=(-1,-1) maxpos=(-1,-1) normal=(100,100,400,300)"), "{first}");
}
