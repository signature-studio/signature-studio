//! Tests of decision 0124 (docs/decisions/0124-wasm-thread-switching.md, PROVISIONAL): the NATIVE side of the wasm thread switch.
//!
//! What a cargo test can check without emscripten or a browser (those are run by lift/harness/web/threads/compare_scen.sh and run_chrome_scen.mjs and recorded in the decision):
//!  1. the native build of thread_scenarios.c (the program that is also built for wasm) prints the event sequences that are written below by hand from the rules of decision 0122 (round robin in
//!     creation order, switches only in wait/yield APIs, the virtual clock moved to the smallest deadline, the 60 Hz boundary of DwmFlush), NOT read back from a run of the wasm build;
//!  2. the hooks in fle_thread.c leave the native build alone: fle_thread_wasm.c is an empty translation unit without -DFLE_THREAD_WASM, fle_thread.c compiled with and without the hook
//!     text gives the same object when the macro is not set (checked by compiling the clean-archive text if FLE_FR9T_CLEAN_RT names it), the macro on a native build is an #error;
//!  3. source checks of the parts that cannot run here (the JS transport has no scheduling policy; every switch saves the shadow stack pointer; a finished thread does not suspend).
//! Every expected value below is a choice of decision 0122 / a hand computation; nothing is compared with a real Windows.

mod etxt_common;

use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn rt_dir() -> PathBuf {
    manifest().join("../../runtime")
}
fn th_dir() -> PathBuf {
    manifest().join("../../harness/web/threads")
}
fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-wallFR9T/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

const FILES: [&str; 18] = [
    "fle_thread.c", "fle_win.c", "fle_class.c", "fle_event.c", "fle_sys.c", "fle_imports.c", "fle_proc.c", "fle_callback.c", "fle_env.c", "fle_vfs.c", "fle_vmap.c", "fle_vol.c", "fle_host.c",
    "fle_time.c", "fle_x87ctl.c", "fle_mxcsr.c", "fle_x87.c", "fle_locale.c",
];

fn clang_obj(src: &Path, obj: &Path, strict: bool, extra: &[&str]) {
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-fPIE", "-c"]);
    if strict {
        cmd.args(["-Wall", "-Wextra", "-Werror"]);
    } else {
        cmd.arg("-w");
    }
    cmd.args(extra);
    let cc = cmd.arg("-I").arg(rt_dir()).arg(src).arg("-o").arg(obj).env("TMPDIR", work_dir().join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {}: {}", src.display(), String::from_utf8_lossy(&cc.stderr));
}

/// The native scenario program, built once per process.
fn scen_exe() -> &'static PathBuf {
    static E: OnceLock<PathBuf> = OnceLock::new();
    E.get_or_init(|| {
        let dir = work_dir().join(etxt_common::per_process("scen"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut objs = Vec::new();
        for f in FILES {
            let o = dir.join(f.replace(".c", ".o"));
            clang_obj(&rt_dir().join(f), &o, true, &[]);
            objs.push(o);
        }
        let so = dir.join("thread_scenarios.o");
        clang_obj(&th_dir().join("thread_scenarios.c"), &so, false, &[]);
        objs.push(so);
        let exe = dir.join("scen");
        let mut cmd = Command::new("clang");
        cmd.arg("-pie").arg("-o").arg(&exe);
        for o in &objs {
            cmd.arg(o);
        }
        let l = cmd.arg("-lm").env("TMPDIR", work_dir().join("tmp")).output().expect("clang not found");
        assert!(l.status.success(), "link failed: {}", String::from_utf8_lossy(&l.stderr));
        exe
    })
}

struct Run {
    out: Vec<String>,
    err: Vec<String>,
    code: Option<i32>,
    signal: Option<i32>,
}

fn run(args: &[&str]) -> Run {
    let r = Command::new(scen_exe()).args(args).output().expect("run failed");
    Run {
        out: String::from_utf8_lossy(&r.stdout).lines().map(|s| s.to_string()).collect(),
        err: String::from_utf8_lossy(&r.stderr).lines().map(|s| s.to_string()).collect(),
        code: r.status.code(),
        signal: r.status.signal(),
    }
}

fn ok(name: &str) -> Vec<String> {
    let r = run(&[name]);
    assert_eq!(r.code, Some(0), "{name}: {:?} {:?}", r.signal, r.err);
    r.out
}

/// The `fle-thread:` lines of stderr without the clock: "switch a b" / "create i f" / "start i" / "exit i code".
fn trace(r: &Run) -> Vec<String> {
    r.err.iter().filter_map(|l| l.strip_prefix("fle-thread: ")).map(|l| l.split(" clock=").next().unwrap().to_string()).collect()
}

// ---------------------------------------------------------------------------------------------
// 1. Event sequences of the scenario program (hand computed from the rules of decision 0122)
// ---------------------------------------------------------------------------------------------

#[test]
fn the_scenario_list_has_38_scenarios_of_which_three_stop() {
    let r = run(&["list"]);
    assert_eq!(r.code, Some(0));
    assert_eq!(r.out.len(), 38, "{:?}", r.out);  // 36 of decision 0122 + priority_yield and priority_slice (decision 0234, mt3)
    assert_eq!(r.out.iter().filter(|l| l.ends_with("(stops)")).count(), 3);
    for must in ["ids", "round_robin", "timer_thread", "layered_thread", "landing", "ping_pong", "many_threads", "dwm_flush", "deep_locals", "stack_churn", "stop_deadlock", "priority_yield", "priority_slice"] {
        assert!(r.out.iter().any(|l| l.starts_with(must)), "{must}");
    }
}

#[test]
fn round_robin_goes_in_creation_order_a_b_c_a_b_c_a_b_c() {
    // main creates A, B, C and waits for A's handle: A runs first (it is the next ready thread after main in creation order), then B, then C, each yielding twice: 1 2 3 4 5 6 7 8 9.
    let o = ok("round_robin");
    assert_eq!(o[0], "order: 1 2 3 4 5 6 7 8 9");
}

#[test]
fn the_clock_moves_to_the_smallest_deadline_and_one_set_event_releases_the_first_waiter() {
    // wall_fr8's hand computation: T1 sleeps 5 ms, T2 waits for the event, nobody ready: the clock moves +5 ms = 50,000 ticks; T1 sets the event, ends; T2 gets it.
    let o = ok("clock_order");
    assert_eq!(o[0], "order: 1 10 20 11 12 21 2 3");
    assert_eq!(o[1], "elapsed-ticks 50000");
    assert_eq!(o[2], "codes 7 8");
}

#[test]
fn a_thread_made_with_flags_0_does_not_run_before_the_creator_waits_or_yields() {
    let o = ok("flags0");
    assert_eq!(o[..4], ["resume-flags0 0", "ran-before-wait 2", "after-sleep0 4", "order: 1 2 11 12"]);
}

#[test]
fn ids_handles_and_suspend_counts_follow_decision_0122() {
    let o = ok("ids");
    assert_eq!(o[..5], ["main 1001", "h1 7ffaa0000008 tid 1002", "h2 7ffaa000000c tid 1003", "start-not-yet 0", "order: 4098 4099"]);
    assert_eq!(ok("suspended")[..6], ["ran-suspended 0", "code 259", "resume 1", "ran-at-resume 0", "wait 0", "ran 1 code 5"]);
    assert_eq!(ok("suspend_count")[..5], ["suspend 1", "r1 2", "ran-with-count-1 0", "r2 1", "ran 1"]);
}

#[test]
fn dwm_flush_waits_for_the_next_60_hz_boundary_with_and_without_another_thread() {
    // boundaries ceil(k * 10^7 / 60): 166667, 333334, 500000, 666667, 833334, 1000000 ticks after the virtual boot (36,000,000,000 ticks = 3,600 s = frame 216,000 exactly, so the first call
    // ends at frame 216,001 = 36,000,166,667)
    let o = ok("dwm_flush");
    assert_eq!(
        o[..6],
        [
            "frame 0: 36000000000 -> 36000166667",
            "frame 1: 36000166667 -> 36000333334",
            "frame 2: 36000333334 -> 36000500000",
            "frame with a sleeping thread 0: 36000500000 -> 36000666667",
            "frame with a sleeping thread 1: 36000666667 -> 36000833334",
            "frame with a sleeping thread 2: 36000833334 -> 36001000000"
        ]
    );
}

#[test]
fn a_timer_thread_of_the_qfc_shape_ticks_every_second_frame_and_the_main_thread_gets_the_messages_in_order() {
    // TTimerThread shape: DwmFlush (to the next 1/60 s boundary) then Sleep(2 ms), then a message to the owner window and a call of the subscriber. The k-th tick is at
    // boundary(k) + 2 ms: 3600000 + 16.67 -> +2 = 3600018 (the first message is read at 18 ms: the boundary 166,667 ticks = 16.67 ms plus 2 ms = 18.67 ms, shown as the ms floor 18),
    // then every 1/60 s + 2 ms ... : 18, 35, 52, 68, 85, 102 (floor of (k * 16.667 + 2)): hand computed as floor(k*16.6667)+2 with k = 1..6 -> 18, 35, 52, 68, 85, 102.
    let o = ok("timer_thread");
    let ticks: Vec<&String> = o.iter().filter(|l| l.starts_with("tick message")).collect();
    assert_eq!(ticks.len(), 6);
    let want = [(0, 18), (1, 35), (2, 52), (3, 68), (4, 85), (5, 102)];
    for (i, (wp, ms)) in want.iter().enumerate() {
        assert_eq!(*ticks[i], format!("tick message 8ffc wp {wp} at {} ms", 3_600_000 + ms), "tick {i}");
    }
    assert!(o.iter().any(|l| l == "order: 301 302 303 304 305 306"), "the subscriber ran once per tick, in order: {o:?}");
    assert!(o.iter().any(|l| l.starts_with("exit code 99")));
}

#[test]
fn a_layered_control_thread_owns_its_window_and_handles_posted_and_sent_messages_then_quits() {
    let o = ok("layered_thread");
    assert!(o.iter().any(|l| l == "post 1 1"));
    assert!(o.iter().any(|l| l == "send result 9003"), "SendMessageW to a window of another thread returns the window procedure's value 0x1000 + 0x8003: {o:?}");
    assert!(o.iter().any(|l| l == "wait 0 code 77"));
    // every window procedure call of the layered window happens in thread 1 (the owner); depth 2 = inside DispatchMessageW's callback of a thread that itself runs inside fle_call_guest
    for l in o.iter().filter(|l| l.contains("msg 8001") || l.contains("msg 8002") || l.contains("msg 8003") || l.contains("msg 8010")) {
        assert!(l.starts_with("  wndproc thread 1 "), "{l}");
    }
    assert!(o.iter().any(|l| l == "order: 101 102 116 203"), "{o:?}");
}

#[test]
fn a_switch_inside_a_window_procedure_keeps_the_callback_depth_and_the_landing_frames_of_each_thread() {
    let o = ok("callback_depth");
    assert!(o.iter().any(|l| l == "  wndproc thread 0 back from Sleep(3) at depth 1"), "{o:?}");
    assert!(o.iter().any(|l| l == "order: 100 11 21"));
    // four threads (including the first) are each between their own setjmp and longjmp at the same time; every one lands in its own frame with its own value:
    // note 1000 + id (before), switches, 2000 + id (after the longjmp), then id * 100 + 1 (the volatile local plus one): ids 0..3 -> (1000, 2000, 1), (1001, 2001, 101), ...
    let l = ok("landing");
    assert_eq!(l[0], "order: 1000 1001 1002 1003 2000 1 2001 101 2002 201 2003 301");
}

#[test]
fn the_spawn_scenario_lets_a_thread_create_a_thread_and_wait_for_it() {
    let o = ok("spawn");
    // thread 1 creates thread 2 (id 0x1003 = 4099 noted by the child), notes 50, waits for it; main notes 52 last
    assert_eq!(o[0], "order: 50 4099 51 52");
}

#[test]
fn forty_threads_that_mix_sleep_zero_sleep_and_switch_to_thread_give_the_same_hash_every_run() {
    let a = ok("many_threads");
    let b = ok("many_threads");
    assert_eq!(a, b, "decision 0122: the schedule is deterministic");
    assert!(a[0].starts_with("hash ") && a[0].contains("sum 780"), "sum of the exit codes 0..39 = 780: {a:?}");
}

#[test]
fn the_scheduler_trace_on_stderr_is_the_same_in_two_runs_and_counts_one_switch_per_transfer() {
    let a = run(&["ping_pong"]);
    let b = run(&["ping_pong"]);
    assert_eq!(trace(&a), trace(&b));
    let sw = trace(&a).iter().filter(|l| l.starts_with("switch ")).count();
    // 2000 rounds, two transfers per round (main -> thread when main waits, thread -> main when it waits), plus the first start and the end: 4003 stderr lines = create + switches + start + exit
    assert_eq!(a.err.len(), 4003, "{}", a.err.len());
    assert!(sw >= 4000 && sw <= 4001, "{sw}");
}

#[test]
fn the_three_stops_end_by_name_with_the_thread_table_and_exit_134() {
    let d = run(&["stop_deadlock"]);
    assert_eq!(d.signal, Some(6));
    assert!(d.err.iter().any(|l| l.starts_with("fle: scheduler: deadlock: no thread can run")), "{:?}", d.err);
    assert!(d.err.iter().any(|l| l.starts_with("threads: 2 (running index 1)")) && d.err.iter().any(|l| l.contains("thread 1: id 0x1002")));
    let b = run(&["stop_bad_start"]);
    assert_eq!(b.signal, Some(6));
    assert!(b.err.iter().any(|l| l == "fle: indirect call to 0x5555 is not a known function entry"), "{:?}", b.err);
    let t = run(&["stop_in_thread"]);
    assert_eq!(t.signal, Some(6));
    assert!(t.err.iter().any(|l| l == "fle: ResumeThread: thread handle value was never returned by CreateThread"), "{:?}", t.err);
}

// ---------------------------------------------------------------------------------------------
// 2. The hooks leave the native build alone
// ---------------------------------------------------------------------------------------------

fn clean_rt() -> Option<PathBuf> {
    std::env::var("FLE_FR9T_CLEAN_RT").ok().map(PathBuf::from).filter(|p| p.join("fle_thread.c").exists())
}

#[test]
fn fle_thread_wasm_c_is_an_empty_translation_unit_on_native() {
    let dir = work_dir().join(etxt_common::per_process("empty"));
    std::fs::create_dir_all(&dir).unwrap();
    let o = dir.join("w.o");
    clang_obj(&rt_dir().join("fle_thread_wasm.c"), &o, true, &[]);
    let nm = Command::new("nm").arg(&o).output().expect("nm not found");
    assert!(String::from_utf8_lossy(&nm.stdout).trim().is_empty(), "no symbol without FLE_THREAD_WASM: {}", String::from_utf8_lossy(&nm.stdout));
}

/// Decision 0126 (old: the comparison was skipped unless FLE_FR9T_CLEAN_RT named the runtime/ of "HEAD 8c8131b" and, when it was set, FAILED: the object of 8c8131b is not the object of the tree, because decision 0123 changed
/// fle_thread.c (docs/investigations/verify-0122-0123-0124/verification.md 3.4)). The baseline of this test is now the text that has no 0124 hook and has the 0123 hunks: the preprocessor output of fle_thread.c with no macro
/// set, which is exactly what a native build compiles. Two parts: (1) the 0124 hook text (every #if/#ifdef over FLE_THREAD_WASM and __EMSCRIPTEN__) must disappear in the preprocessing of a native build, so the object of
/// the tree is the object of a tree that has none of it (compared by compiling the preprocessed text of the tree against a copy with the hook text removed by `clang -E` of the SAME file with `-U FLE_THREAD_WASM
/// -U __EMSCRIPTEN__`: the two objects must be byte-identical, and they are the same compile); (2) when FLE_FR9T_CLEAN_RT names a runtime/ whose fle_thread.c is the pre-0124 text plus the 0123 hunks, the object of the
/// tree equals that object (compiled from the same relative file name: the object holds the path of its source, so two files in two directories are compiled under one name). Without the variable part (2) prints SKIPPED.
#[test]
fn the_native_fle_thread_object_equals_the_one_of_the_tree_without_the_hooks() {
    let dir = work_dir().join(etxt_common::per_process("cmp"));
    std::fs::create_dir_all(&dir).unwrap();
    // (1) no macro set: the tree's fle_thread.c compiled as it is and the same text with FLE_THREAD_WASM and __EMSCRIPTEN__ explicitly undefined are the same object
    let (a, b) = (dir.join("a.o"), dir.join("b.o"));
    clang_obj(&rt_dir().join("fle_thread.c"), &a, true, &[]);
    clang_obj(&rt_dir().join("fle_thread.c"), &b, true, &["-UFLE_THREAD_WASM", "-U__EMSCRIPTEN__"]);
    assert_eq!(std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap(), "-UFLE_THREAD_WASM changes nothing in a native build");
    // the hook is only text: with the macro undefined, no symbol of fle_thread_wasm.h is referenced by the object
    let nm = Command::new("nm").arg(&a).output().expect("nm not found");
    assert!(!String::from_utf8_lossy(&nm.stdout).contains("fle_thread_wasm"), "the native object has no reference to the wasm hook: {}", String::from_utf8_lossy(&nm.stdout));
    // (2) the clean baseline (pre-0124 text + 0123 hunks), when the caller names it
    let Some(clean) = clean_rt() else {
        eprintln!("SKIPPED: FLE_FR9T_CLEAN_RT is not set: the comparison with the pre-0124 + 0123 text is not made (part (1) above was made; the decision records the cmp of the work)");
        return;
    };
    // both files are compiled under the same relative name from two directories, so the path that the object records is the same
    let (da, db) = (dir.join("tree"), dir.join("base"));
    std::fs::create_dir_all(&da).unwrap();
    std::fs::create_dir_all(&db).unwrap();
    std::fs::copy(rt_dir().join("fle_thread.c"), da.join("fle_thread.c")).unwrap();
    std::fs::copy(clean.join("fle_thread.c"), db.join("fle_thread.c")).unwrap();
    let (oa, ob) = (dir.join("tree.o"), dir.join("base.o"));
    for (d, o) in [(&da, &oa), (&db, &ob)] {
        let cc = Command::new("clang").current_dir(d).args(["-O0", "-std=gnu11", "-Wall", "-Wextra", "-Werror", "-fPIE", "-c", "-I"]).arg(&rt_dir()).arg("fle_thread.c").arg("-o").arg(o).env("TMPDIR", work_dir().join("tmp")).output().unwrap();
        assert!(cc.status.success(), "{}", String::from_utf8_lossy(&cc.stderr));
    }
    assert_eq!(std::fs::read(&oa).unwrap(), std::fs::read(&ob).unwrap(), "the native object of fle_thread.c must not change by the 0124 hook");
}

// ---------------------------------------------------------------------------------------------
// 3. Source checks of what only wasm runs
// ---------------------------------------------------------------------------------------------

fn code_of(p: &Path) -> String {
    let s = std::fs::read_to_string(p).unwrap();
    let b: Vec<char> = s.chars().collect();
    let (mut o, mut i) = (String::new(), 0);
    while i < b.len() {
        if b[i] == '/' && i + 1 < b.len() && b[i + 1] == '*' {
            i += 2;
            while i + 1 < b.len() && !(b[i] == '*' && b[i + 1] == '/') {
                i += 1;
            }
            i += 2;
        } else if b[i] == '/' && i + 1 < b.len() && b[i + 1] == '/' {
            while i < b.len() && b[i] != '\n' {
                i += 1;
            }
        } else {
            o.push(b[i]);
            i += 1;
        }
    }
    o
}

#[test]
fn every_wasm_switch_saves_the_shadow_stack_pointer_and_checks_the_canary_before_it_leaves_the_thread() {
    let c = code_of(&rt_dir().join("fle_thread_wasm.c"));
    let t = c.find("void fle_thread_wasm_transfer(").unwrap();
    let body = &c[t..];
    let (sp, chk, sw, ex) = (body.find("emscripten_stack_get_current()").unwrap(), body.find("check_region(from").unwrap(), body.find("fle_wasm_thread_switch(from").unwrap(), body.find("fle_wasm_thread_exit(to").unwrap());
    assert!(sp < chk && chk < ex && ex < sw, "order: save the pointer, check the canary, (finished thread: unwind), else suspend");
    // the JS side restores it for the thread it hands over to, in the two places a hand-over starts or resumes a thread
    let js = code_of(&th_dir().join("fle_thread_lib.js"));
    assert_eq!(js.matches("stackRestore(").count(), 2, "one for a new thread (spNew), one for a resumed thread (S.sp[to])");
    assert!(js.contains("stackRestore(spNew)") && js.contains("stackRestore(S.sp[to])"));
}

#[test]
fn the_js_transport_holds_no_scheduling_policy() {
    // decision 0124 item 1: the choice of the next thread is made in fle_thread.c; the JS file may not contain a thread selection loop, a priority, a clock or a queue of ready threads
    let js = code_of(&th_dir().join("fle_thread_lib.js"));
    for w in ["ready", "priority", "deadline", "Date.now", "performance.now", "setTimeout", "setInterval", "Math.random", "round"] {
        assert!(!js.contains(w), "fle_thread_lib.js must not contain {w:?}");
    }
    // the hand-over is queued (never run inside the import call) and a finished thread ends by a throw, not by a suspension
    assert!(js.contains("queueMicrotask(() => S.handover(to, spNew))"));
    assert!(js.contains("throw { fleThreadExit: true"));
}

#[test]
fn fle_thread_c_hooks_are_conditional_and_the_makecontext_path_is_the_default() {
    let c = std::fs::read_to_string(rt_dir().join("fle_thread.c")).unwrap();
    assert!(c.contains("swapcontext(&from->uc, &t->uc);"), "the native switch is still swapcontext");
    assert!(c.contains("makecontext(&t->uc, (void (*)(void))thread_main, 0);"));
    // every use of the wasm functions is under FLE_THREAD_WASM
    let code = code_of(&rt_dir().join("fle_thread.c"));
    for f in ["fle_thread_wasm_transfer(", "fle_thread_wasm_stack_alloc(", "fle_thread_wasm_stack_free(", "fle_thread_wasm_entry_cur("] {
        for (i, _) in code.match_indices(f) {
            let before = &code[..i];
            let open = before.rfind("#ifdef __EMSCRIPTEN__").or_else(|| before.rfind("#elif defined(__EMSCRIPTEN__)")).or_else(|| before.rfind("#else"));
            assert!(open.is_some(), "{f} is used outside an __EMSCRIPTEN__ conditional (decision 0243: the wasm switch is the target)");
        }
    }
}

