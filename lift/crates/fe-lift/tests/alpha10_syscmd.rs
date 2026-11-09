//! Decision 0267 (alpha10): the system commands of DefWindowProcW (Learn WM_SYSCOMMAND: SC_MINIMIZE, SC_MAXIMIZE, SC_RESTORE become ShowWindow(SW_MINIMIZE / SW_MAXIMIZE / SW_RESTORE), SC_CLOSE becomes WM_CLOSE), the rectangle and
//! client of a minimized window ((-32000, -32000), 0 x 0; WM_SIZE SIZE_MINIMIZED with lParam 0) and the task bar of the host (fle_win_taskbar_buttons, fle_input_host_syscommand). The C tool harness/alpha10/syscmd_seq.c runs the runtime
//! and prints the messages and the state after each step; the same program on Wine 10.0 (UNOFFICIAL) gives the sequences asserted here.

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
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("alpha10_syscmd");
    std::fs::create_dir_all(&d).unwrap();
    d
}

const OBJS: [&str; 36] = [
    "fle_api2_call", "fle_appview", "fle_callback", "fle_class", "fle_comctl", "fle_cursor", "fle_env", "fle_event", "fle_gdc", "fle_gdi", "fle_host", "fle_imports", "fle_input", "fle_layered", "fle_locale", "fle_mxcsr",
    "fle_proc", "fle_raster", "fle_sys", "fle_thread", "fle_time", "fle_token", "fle_vfs", "fle_visrgn", "fle_vmap", "fle_vol", "fle_win", "fle_x87", "fle_x87ctl", "fle_wall_ia2", "fle_wall_ia2w", "fle_desk", "fle_dwm", "fle_monitor", "fle_sysmetrics", "fle_spi",
];

#[test]
fn system_commands_minimize_maximize_restore_and_the_task_bar_follow_the_contract() {
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
    let exe = dir.join("syscmd_seq");
    let mut cmd = Command::new("clang");
    cmd.args(["-O1", "-std=gnu11", "-w", "-fPIE", "-pthread", "-I"]).arg(rt()).args(defs).arg(lift_dir().join("harness/alpha10/syscmd_seq.c"));
    for o in &objs {
        cmd.arg(o);
    }
    let cc = cmd.arg("-o").arg(&exe).arg("-lm").output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for syscmd_seq: {}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new(&exe).output().expect("run failed");
    let out = String::from_utf8_lossy(&r.stdout).to_string();
    assert!(r.status.success(), "syscmd_seq failed: {out}");
    // a section = the lines under a heading (a line that does not start with two spaces)
    let section = |head: &str| -> Vec<String> {
        let mut it = out.lines().skip_while(|l| !l.starts_with(head));
        it.next().unwrap_or_else(|| panic!("no section {head}: {out}"));
        it.take_while(|l| l.starts_with("  ")).map(|l| l.trim().to_string()).collect()
    };
    let msgs = |s: &[String]| -> Vec<String> { s.iter().filter(|l| !l.starts_with("state:") && !l.starts_with("button") && !l.starts_with("queued") && !l.starts_with("refused")).map(|l| l.split(" flags=").next().unwrap().to_string()).collect() };
    let state = |s: &[String]| -> String { s.iter().find(|l| l.starts_with("state:")).cloned().unwrap_or_default() };

    // SC_MINIMIZE: DefWindowProc -> ShowWindow(SW_MINIMIZE); the window goes to (-32000, -32000) with a client of 0 x 0 and WM_SIZE carries SIZE_MINIMIZED and lParam 0 (Wine 10.0)
    let min = section("SendMessage(top, WM_SYSCOMMAND, SC_MINIMIZE):");
    let m = msgs(&min);
    assert_eq!(m[0], "top WM_SYSCOMMAND cmd=0xf020 lp=0", "{m:?}");
    assert!(m.iter().any(|l| l == "top WM_MOVE x=-32000 y=-32000"), "{m:?}");
    assert_eq!(m.last().unwrap(), "top WM_SIZE wp=1 cx=0 cy=0", "{m:?}");
    let pos = |name: &str| m.iter().position(|l| l.contains(name)).unwrap_or_else(|| panic!("no {name}: {m:?}"));
    assert!(pos("WM_WINDOWPOSCHANGING") < pos("WM_NCCALCSIZE") && pos("WM_NCCALCSIZE") < pos("WM_WINDOWPOSCHANGED") && pos("WM_WINDOWPOSCHANGED") < pos("WM_MOVE") && pos("WM_MOVE") < pos("WM_SIZE"), "{m:?}");
    let st = state(&min);
    assert!(st.contains("iconic=1") && st.contains("client=0x0") && st.contains("(-32000,-32000,") && st.contains("showCmd=2"), "{st}");
    // SC_RESTORE of the minimized window: back to 100,100 300 x 200, WM_SIZE SIZE_RESTORED
    let rst = section("SendMessage(top, WM_SYSCOMMAND, SC_RESTORE of the minimized window):");
    let m = msgs(&rst);
    assert_eq!(m[0], "top WM_SYSCOMMAND cmd=0xf120 lp=0", "{m:?}");
    assert!(m.iter().any(|l| l == "top WM_SIZE wp=0 cx=300 cy=200"), "{m:?}");
    let st = state(&rst);
    assert!(st.contains("iconic=0") && st.contains("window=(100,100,400,300)") && st.contains("client=300x200") && st.contains("showCmd=1"), "{st}");
    // SC_RESTORE of a normal window changes nothing
    let nrm = section("SendMessage(top, WM_SYSCOMMAND, SC_RESTORE of a normal window):");
    assert_eq!(msgs(&nrm), vec!["top WM_SYSCOMMAND cmd=0xf120 lp=0".to_string()], "{nrm:?}");
    // SC_MAXIMIZE: WM_GETMINMAXINFO first, then the size messages, WM_SIZE SIZE_MAXIMIZED
    let max = section("SendMessage(top, WM_SYSCOMMAND, SC_MAXIMIZE):");
    let m = msgs(&max);
    assert_eq!(m[1], "top WM_GETMINMAXINFO maxsize=1920,1080 maxpos=0,0", "{m:?}");
    assert_eq!(m.last().unwrap(), "top WM_SIZE wp=2 cx=1920 cy=1080", "{m:?}");
    assert!(state(&max).contains("zoomed=1"), "{}", state(&max));
    // a window that was maximized when it was minimized comes back maximized (SC_RESTORE of the minimized window)
    let back = section("SendMessage(top, WM_SYSCOMMAND, SC_RESTORE of the window that was maximized when it was minimized):");
    let m = msgs(&back);
    assert_eq!(m.last().unwrap(), "top WM_SIZE wp=2 cx=1920 cy=1080", "{m:?}");
    assert!(state(&back).contains("zoomed=1"), "{}", state(&back));
    // and SC_RESTORE of the maximized window gives the normal rectangle back
    let rmax = section("SendMessage(top, WM_SYSCOMMAND, SC_RESTORE of the maximized window):");
    assert_eq!(msgs(&rmax).last().unwrap(), "top WM_SIZE wp=0 cx=300 cy=200", "{rmax:?}");
    // DefWindowProc does not look at WS_MINIMIZEBOX / WS_MAXIMIZEBOX (Wine 10.0)
    assert!(state(&section("SendMessage(two, WM_SYSCOMMAND, SC_MINIMIZE of a window without WS_MINIMIZEBOX):")).contains("iconic=1"));
    assert!(state(&section("SendMessage(two, WM_SYSCOMMAND, SC_MAXIMIZE of a window without WS_MAXIMIZEBOX):")).contains("zoomed=1"));
    // SC_CLOSE is WM_CLOSE (and DefWindowProc destroys the window)
    let cl = section("SendMessage(two, WM_SYSCOMMAND, SC_CLOSE)");
    assert!(cl.iter().any(|l| l == "two WM_CLOSE") && cl.iter().any(|l| l == "two WM_DESTROY"), "{cl:?}");
    // the task bar of the host
    let h1 = section("H1 ");
    assert!(h1.iter().any(|l| l == "button 0: top iconic=0 zoomed=0"), "{h1:?}");
    let h2 = section("H2 ");
    assert!(h2.iter().any(|l| l == "button 0: top iconic=1 zoomed=0"), "{h2:?}");
    let h3 = section("H3 ");
    assert!(h3.iter().any(|l| l == "queued=1"), "{h3:?}");
    let st = state(&h3);
    assert!(st.contains("iconic=0") && st.contains("window=(100,100,400,300)") && st.contains("client=300x200"), "{st}");
    assert!(h3.iter().any(|l| l == "refused: cmd 0xF010 (SC_MOVE) -> 0; window 0 -> 0"), "{h3:?}");
    let n: usize = out.lines().filter(|l| l.trim_start().starts_with("top WM_") || l.trim_start().starts_with("two WM_")).count();
    assert!(n >= 40, "the program prints at least 40 message lines, got {n}");
}
