//! Decision 0280 (gpu7, PROVISIONAL): the scheduler hook by which the engine takes the page's input at a scheduler point instead of at its idle (runtime/fle_thread.c schedule_ex -> fle_input_ring_poll,
//! harness/fruity_idle.c). Native, the real runtime files, stub guest functions (harness/gpu7/ring_poll.c; the scaffold of harness/web/threads/thread_scenarios.c). Expected values are written here from the
//! design of the decision (the hook is a scheduler point; the records go through the SAME functions as the idle path's exports), not read back from the C code.
//!   1. the hook is called at scheduler points only (never with fewer than two threads) and at every switch after;
//!   2. an input queued by the hook reaches the thread that waits in GetMessageW at the very next switch (within 2 iterations of a Sleep(1) loop; at an idle it would be all 20);
//!   3. fle_input_ring_poll() of fruity_idle.c (cut out of the file between its markers, with fake imports) gives the same state as the same script applied by fle_input_host_*: queue, pointer, key states;
//!   4. the native fle_thread.o without the define has no hook (the native object of the tree without the hook is byte-identical: wall_fr9t.rs).

mod etxt_common;

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
    manifest().join("../../harness")
}
fn work() -> PathBuf {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("gpu7");
    std::fs::create_dir_all(&d).unwrap();
    d
}

const OBJS: [&str; 29] = [
    "fle_api2_call", "fle_appview", "fle_callback", "fle_class", "fle_comctl", "fle_cursor", "fle_env", "fle_event", "fle_gdc", "fle_gdi", "fle_host", "fle_imports", "fle_input", "fle_layered", "fle_locale", "fle_mxcsr",
    "fle_raster", "fle_sys", "fle_thread", "fle_time", "fle_token", "fle_vfs", "fle_visrgn", "fle_vmap", "fle_vol", "fle_wall_ia2", "fle_wall_ia2w", "fle_win", "fle_x87",
];

fn clang_obj(src: &PathBuf, obj: &PathBuf, extra: &[&str]) {
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fPIE", "-c", "-I"]).arg(rt()).args(extra).arg(src).arg("-o").arg(obj);
    let cc = cmd.env("TMPDIR", work()).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {}: {}", src.display(), String::from_utf8_lossy(&cc.stderr));
}

/// the text of fle_input_ring_poll() (and its imports' prototypes) between the markers of harness/fruity_idle.c
fn cut() -> String {
    let t = std::fs::read_to_string(harness().join("fruity_idle.c")).unwrap();
    let a = t.find("/* RING-POLL-BEGIN").expect("RING-POLL-BEGIN");
    let b = t.find("/* RING-POLL-END */").expect("RING-POLL-END");
    assert!(a < b);
    t[a..b].to_string()
}

fn exes() -> &'static (PathBuf, PathBuf, PathBuf) {
    static E: OnceLock<(PathBuf, PathBuf, PathBuf)> = OnceLock::new();
    E.get_or_init(|| {
        let dir = work().join(etxt_common::per_process("obj"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut objs: Vec<PathBuf> = Vec::new();
        for n in OBJS.iter().chain(["fle_x87ctl"].iter()) {
            let o = dir.join(format!("{n}.o"));
            let extra: &[&str] = if *n == "fle_thread" { &["-DFLE_RING_POLL_HOOK=1"] } else { &[] };
            clang_obj(&rt().join(format!("{n}.c")), &o, extra);
            objs.push(o);
        }
        let cutf = dir.join("ring_poll_cut.inc");
        std::fs::write(&cutf, cut()).unwrap();
        let build = |name: &str, define: Option<&str>| -> PathBuf {
            let exe = work().join(etxt_common::uniq(name));
            let mut cmd = Command::new("clang");
            cmd.args(["-O0", "-std=gnu11", "-w", "-fPIE", "-pthread", "-I"]).arg(rt()).arg("-I").arg(&dir);
            if let Some(d) = define {
                cmd.arg(d);
            }
            cmd.arg(harness().join("gpu7/ring_poll.c"));
            for o in &objs {
                cmd.arg(o);
            }
            let cc = cmd.arg("-o").arg(&exe).arg("-lm").output().expect("clang not found");
            assert!(cc.status.success(), "{}", String::from_utf8_lossy(&cc.stderr));
            exe
        };
        // decision 0280 (the answer rule): the same scenario program with the tree's fle_thread.c compiled WITHOUT the hook define (the hook is never called): what the hook does when nothing is queued must be invisible
        let nohook = dir.join("fle_thread_nohook.o");
        clang_obj(&rt().join("fle_thread.c"), &nohook, &[]);
        let objs_nohook: Vec<PathBuf> = objs.iter().map(|o| if o.file_name().unwrap() == "fle_thread.o" { nohook.clone() } else { o.clone() }).collect();
        let exe_nohook = {
            let exe = work().join(etxt_common::uniq("ring_poll_nohook"));
            let mut cmd = Command::new("clang");
            cmd.args(["-O0", "-std=gnu11", "-w", "-fPIE", "-pthread", "-I"]).arg(rt()).arg("-I").arg(&dir).arg(harness().join("gpu7/ring_poll.c"));
            for o in &objs_nohook {
                cmd.arg(o);
            }
            let cc = cmd.arg("-o").arg(&exe).arg("-lm").output().expect("clang not found");
            assert!(cc.status.success(), "{}", String::from_utf8_lossy(&cc.stderr));
            exe
        };
        (build("ring_poll_hook", None), build("ring_poll_records", Some("-DRING_APPLY_RECORDS")), exe_nohook)
    })
}

fn run(exe: &PathBuf, scenario: &str) -> String {
    let r = Command::new(exe).arg(scenario).output().unwrap();
    assert!(r.status.success(), "{scenario}: {}", String::from_utf8_lossy(&r.stderr));
    String::from_utf8_lossy(&r.stdout).to_string()
}

#[test]
fn the_hook_is_called_at_scheduler_points_only_and_at_every_switch_after_the_second_thread_exists() {
    let o = run(&exes().0, "hook_counts");
    assert!(o.contains("before CreateThread: hook calls 0"), "{o}");
    assert!(o.contains("hook called at least once per switch"), "{o}");
    assert!(o.contains("calls with fewer than 2 threads 0"), "{o}");
}

#[test]
fn a_hook_with_nothing_queued_changes_neither_the_virtual_clock_nor_the_thread_order() {
    let with = run(&exes().0, "hook_counts");
    let without = run(&exes().2, "hook_counts");
    let tail = |o: &str| -> String { let l = o.lines().find(|l| l.starts_with("after the mixer-like thread ran")).unwrap_or_else(|| panic!("{o}")); l[l.find("mixer iteration").unwrap()..].to_string() };
    assert_eq!(tail(&with), tail(&without), "with the hook (nothing queued):\n{with}\nwithout:\n{without}");
    assert!(with.contains("hook called at least once per switch"), "{with}");
    assert!(without.contains("hook called too rarely"), "the build without the define never calls the hook: {without}");
}

#[test]
fn an_input_queued_by_the_hook_reaches_the_thread_in_getmessage_at_the_next_switch() {
    let o = run(&exes().0, "inject_wakes_ui");
    assert!(o.contains("window created"), "{o}");
    assert!(o.contains("the virtual clock was moved by 0 ticks"), "queueing an input at a scheduler point does not move the virtual clock (the engine is running; only an idle moves it): {o}");
    assert!(o.contains("the UI thread got WM_LBUTTONDOWN (0x201)"), "{o}");
    let line = o.lines().find(|l| l.starts_with("delay between")).expect("delay line");
    let n: i32 = line.split(": ").nth(1).unwrap().split(' ').next().unwrap().parse().unwrap();
    assert!((0..=2).contains(&n), "{o}");
}

#[test]
fn an_injected_input_is_settled_only_when_the_thread_that_handled_it_waits_in_its_message_loop_again() {
    let o = run(&exes().0, "inject_wakes_ui");
    let l = o.lines().find(|l| l.starts_with("settled (decision 0280, L1c)")).unwrap_or_else(|| panic!("{o}"));
    let num = |key: &str| -> i64 { let i = l.find(key).unwrap_or_else(|| panic!("{key} in {l}")) + key.len(); l[i..].trim_start().split(|c: char| !c.is_ascii_digit() && c != '-').next().unwrap().parse().unwrap() };
    assert_eq!(num("before the injection"), 1, "nothing is waiting before the first input: {l}");
    assert_eq!(num("right after it"), 0, "a queued event is not settled: {l}");
    assert_eq!(num("holds the message"), 0, "the thread that retrieved the message is running, not waiting: {l}");
    assert!(num("mixer iteration") >= 0, "once the UI thread is back in GetMessageW the state is settled: {l}");
}

#[test]
fn the_records_of_fruity_idle_give_the_same_state_as_the_same_events_applied_directly() {
    let direct = run(&exes().1, "apply_direct");
    let rec = run(&exes().1, "apply_records");
    let state = |s: &str| -> Vec<String> { s.lines().filter(|l| l.starts_with("before:") || l.starts_with("after:")).map(|l| l.to_string()).collect() };
    assert_eq!(state(&direct), state(&rec), "direct:\n{direct}\nrecords:\n{rec}");
    // the script has 14 records; 3 of them are refused by the functions themselves (a key with vk 0, an unknown mouse kind, a display of 10x10), the other 11 are queued; the result of each record is the one of the direct call
    let queued_direct = direct.lines().filter(|l| l.starts_with("record ") && l.ends_with("-> 1")).count();
    assert_eq!(queued_direct, 11, "{direct}");
    assert!(rec.contains("done_n 14 queued 11"), "{rec}");
    let pend = state(&rec).into_iter().find(|l| l.starts_with("after:")).unwrap();
    assert!(pend.contains("pending 11"), "{pend}");
}

#[test]
fn the_hook_exists_only_in_the_wasm_build_or_with_the_test_define() {
    let t = std::fs::read_to_string(rt().join("fle_thread.c")).unwrap();
    assert_eq!(t.matches("void fle_input_ring_poll(void) __attribute__((weak));").count(), 1);
    assert_eq!(t.matches("if (fle_input_ring_poll) fle_input_ring_poll();").count(), 1);
    for at in [t.find("void fle_input_ring_poll(void) __attribute__((weak));").unwrap(), t.find("if (fle_input_ring_poll) fle_input_ring_poll();").unwrap()] {
        let before = &t[..at];
        let guard = before.rfind("#if defined(__EMSCRIPTEN__) || defined(FLE_RING_POLL_HOOK)").expect("a guard before each use");
        assert!(before[guard..].find("#endif").is_none(), "the use is inside the guard");
    }
    // the call is the first statement of the scheduler loop, before poll_waiters()
    let call = t.find("if (fle_input_ring_poll) fle_input_ring_poll();").unwrap();
    let poll = t[call..].find("poll_waiters();").unwrap();
    assert!(poll < 600, "the hook comes right before poll_waiters()");
}
