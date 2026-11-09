//! Tests of decision 0222 (docs/0222-window-manager-fidelity.md): z-order, activation, mouse activation, focus and close of top-level, owned and child windows. The cases of harness/api2/win_act.c are run against the REAL
//! runtime window layer (the scaffold of harness/web/g3: a stub guest window procedure that calls DefWindowProcW) and compared, step by step, with the output of the same cases on Wine 10.0 (mingw build, headless Wine:
//! `harness/api2/build.sh act WORKDIR`), kept in `expected/win-act-wine-10.0.txt` (UNOFFICIAL: Wine is a reference, not Windows). No Wine is needed to run this test.
//!   - `expected/win-act-runtime-0222.txt` pins what the runtime says, block by block (a change of the window layer shows here);
//!   - a step on which the runtime differs from Wine must be listed in `expected/win-act-known-divergence.txt` with its reason (a reading of Microsoft Learn that Wine's null display driver does not follow, a Wine-only
//!     behaviour the runtime does not copy, or a gap); a listed step that no longer differs asks for the entry to be deleted, as the other Wine comparisons of harness/api2 do.
//! Lines of windows the test does not know yet ('?', the window being created) and WM_NCHITTEST lines (Wine asks again after every window change; the runtime only for pointer input) are left out on both sides, and the
//! blocks '(setup)' and '(cleanup)' are not compared.

mod etxt_common;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn rt() -> PathBuf {
    manifest().join("../../runtime")
}
fn harness() -> PathBuf {
    manifest().join("../../harness/api2")
}
fn work() -> PathBuf {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("win2");
    std::fs::create_dir_all(&d).unwrap();
    d
}

const WIN_OBJS: [&str; 30] = [
    "fle_api2_call", "fle_appview", "fle_callback", "fle_class", "fle_comctl", "fle_cursor", "fle_env", "fle_event", "fle_gdc", "fle_gdi", "fle_host", "fle_imports", "fle_input", "fle_layered", "fle_locale", "fle_mxcsr", "fle_proc",
    "fle_raster", "fle_sys", "fle_thread", "fle_time", "fle_token", "fle_vfs", "fle_visrgn", "fle_vmap", "fle_vol", "fle_wall_ia2", "fle_wall_ia2w", "fle_win", "fle_x87",
];

fn read(p: &str) -> String {
    std::fs::read_to_string(harness().join(p)).unwrap_or_else(|e| panic!("{p}: {e}"))
}

type Blocks = Vec<(String, Vec<String>)>;

/// The blocks of an output of win_act.c: key "N: step title" and the lines of the step. '(setup)' and '(cleanup)' are not kept.
fn blocks(text: &str) -> Blocks {
    let mut out: Blocks = Vec::new();
    let mut sc = 0u32;
    let mut cur: Option<usize> = None;
    for raw in text.lines() {
        let ln = raw.trim_end_matches('\r');
        if let Some(rest) = ln.strip_prefix("#### scenario ") {
            sc = rest.split(':').next().unwrap().trim().parse().unwrap();
            cur = None;
            continue;
        }
        if !ln.is_empty() && !ln.starts_with(' ') && !ln.starts_with('=') && ln.ends_with(':') {
            let title = &ln[..ln.len() - 1];
            if title == "(cleanup)" || title == "(setup)" {
                cur = None;
            } else {
                out.push((format!("{sc}: {title}"), Vec::new()));
                cur = Some(out.len() - 1);
            }
            continue;
        }
        if ln.starts_with("  ?") || ln.contains("WM_NCHITTEST") {
            continue;
        }
        if let Some(i) = cur {
            out[i].1.push(ln.to_string());
        }
    }
    out
}

fn render(b: &Blocks) -> String {
    let mut s = String::new();
    for (k, v) in b {
        s.push_str(&format!("@@ {k}\n"));
        for l in v {
            s.push_str(l);
            s.push('\n');
        }
    }
    s
}

fn runtime_output() -> &'static String {
    static O: OnceLock<String> = OnceLock::new();
    O.get_or_init(|| {
        let dir = work().join(etxt_common::per_process("actobj"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut objs = Vec::new();
        for n in WIN_OBJS.iter().chain(["fle_x87ctl"].iter()) {
            let o = dir.join(format!("{n}.o"));
            let cc = Command::new("clang").args(["-O0", "-std=gnu11", "-w", "-fPIE", "-c", "-I"]).arg(rt()).arg(rt().join(format!("{n}.c"))).arg("-o").arg(&o).output().expect("clang not found");
            assert!(cc.status.success(), "clang failed for {n}: {}", String::from_utf8_lossy(&cc.stderr));
            objs.push(o);
        }
        let exe = work().join(etxt_common::uniq("win_act"));
        let mut cmd = Command::new("clang");
        cmd.args(["-O0", "-std=gnu11", "-w", "-fPIE", "-pthread", "-I"]).arg(rt()).arg(harness().join("win_act.c"));
        for o in &objs {
            cmd.arg(o);
        }
        let cc = cmd.arg("-o").arg(&exe).arg("-lm").output().expect("clang not found");
        assert!(cc.status.success(), "{}", String::from_utf8_lossy(&cc.stderr));
        let r = Command::new(&exe).output().unwrap();
        assert!(r.status.success(), "the window cases stopped: {}", String::from_utf8_lossy(&r.stderr).lines().last().unwrap_or(""));
        String::from_utf8_lossy(&r.stdout).to_string()
    })
}

#[test]
fn runtime_output_is_the_pinned_one() {
    let now = render(&blocks(runtime_output()));
    let pinned = read("expected/win-act-runtime-0222.txt");
    if now != pinned {
        let a: Vec<&str> = now.lines().collect();
        let b: Vec<&str> = pinned.lines().collect();
        let k = a.iter().zip(b.iter()).position(|(x, y)| x != y).unwrap_or(a.len().min(b.len()));
        panic!("the runtime's output changed from expected/win-act-runtime-0222.txt at line {}: now {:?}, pinned {:?}", k + 1, a.get(k), b.get(k));
    }
}

#[test]
fn every_step_equals_wine_10_0_except_the_listed_divergences() {
    let rtb = blocks(runtime_output());
    let wine = blocks(&read("expected/win-act-wine-10.0.txt"));
    let mut div: BTreeMap<String, String> = BTreeMap::new();
    for l in read("expected/win-act-known-divergence.txt").lines().filter(|l| !l.starts_with('#') && !l.trim().is_empty()) {
        let f: Vec<&str> = l.splitn(3, '\t').collect();
        assert_eq!(f.len(), 3, "a divergence line is: key TAB class TAB reason: {l}");
        div.insert(f[0].to_string(), f[2].to_string());
    }
    assert!(!wine.is_empty());
    let mut used = 0usize;
    for (k, wl) in &wine {
        let rl = rtb.iter().find(|(rk, _)| rk == k).map(|(_, v)| v).unwrap_or_else(|| panic!("the runtime has no step {k}"));
        if wl == rl {
            assert!(!div.contains_key(k), "step {k} now equals Wine: delete its entry from expected/win-act-known-divergence.txt");
        } else {
            assert!(div.contains_key(k), "step {k} differs from Wine 10.0 and is not a listed divergence:\n wine:    {wl:?}\n runtime: {rl:?}");
            used += 1;
        }
    }
    assert_eq!(used, div.len(), "a listed divergence names a step that does not exist");
}
