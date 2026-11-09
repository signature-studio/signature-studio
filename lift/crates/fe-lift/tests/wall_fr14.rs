//! Tests of decision 0136 (docs/decisions/0136-input-roundtrip-and-timers.md, PROVISIONAL): the input round trip and the idle clock, NATIVE side.
//!
//! What a cargo test can check without the lifted engine, a browser or emscripten: harness/web/g3/g3_scenarios.c is built with the REAL runtime files (fle_input.c, fle_comctl.c, fle_win.c, fle_class.c,
//! fle_thread.c, fle_sys.c ...) and stub guest window procedures, and each scenario's stdout is compared with the sequence that is written below BY HAND from the Learn pages (WM_NCHITTEST, WM_SETCURSOR,
//! WM_MOUSEACTIVATE, WM_MOUSEMOVE, WM_LBUTTONDOWN, TRACKMOUSEEVENT, WM_MOUSEHOVER, WM_MOUSELEAVE, WM_KEYDOWN, WM_CHAR, GetKeyState, GetMessageTime, GetMessagePos, SetTimer / WM_TIMER) and from the choices
//! of the decision (S-... names). The layout of the scenarios: A = pop-up (100,100) 300 x 200 with a child (10,10) 100 x 50 (screen 110..210 x 110..160); B = pop-up (500,100) 200 x 200; C = pop-up (100,400) 100 x 100
//! of a class with CS_DBLCLKS; the origin of a window's client area is its position (no frame: decision 0123 S-nc-zero). Nothing is compared with a real Windows.
//! Also: source checks of the places that other files hook (fle_win.c, fle_thread.c), and that the new import rows are appended at the END of the response table.

mod etxt_common;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
/// The runtime directory. FLE_FR14_RT (a copy of lift/runtime) overrides it: the mutation runs of the run record change a file of such a copy and expect a test to fail.
fn rt_dir() -> PathBuf {
    if let Ok(d) = std::env::var("FLE_FR14_RT") {
        return PathBuf::from(d);
    }
    manifest().join("../../runtime")
}
fn g3_dir() -> PathBuf {
    manifest().join("../../harness/web/g3")
}
fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-wallFR14/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

const FILES: [&str; 22] = [
    "fle_thread.c", "fle_win.c", "fle_class.c", "fle_event.c", "fle_sys.c", "fle_imports.c", "fle_proc.c", "fle_callback.c", "fle_env.c", "fle_vfs.c", "fle_vmap.c", "fle_vol.c", "fle_host.c",
    "fle_time.c", "fle_x87ctl.c", "fle_mxcsr.c", "fle_x87.c", "fle_locale.c", "fle_input.c", "fle_comctl.c", "fle_cursor.c", "fle_token.c",
];

fn clang_obj(src: &Path, obj: &Path, strict: bool) {
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-fPIE", "-c"]);
    if strict {
        cmd.args(["-Wall", "-Wextra", "-Werror"]);
    } else {
        cmd.arg("-w");
    }
    let cc = cmd.arg("-I").arg(rt_dir()).arg(src).arg("-o").arg(obj).env("TMPDIR", work_dir().join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {}: {}", src.display(), String::from_utf8_lossy(&cc.stderr));
}

fn scen_exe() -> &'static PathBuf {
    static E: OnceLock<PathBuf> = OnceLock::new();
    E.get_or_init(|| {
        let dir = work_dir().join(etxt_common::per_process("scen"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut objs = Vec::new();
        for f in FILES {
            let o = dir.join(f.replace(".c", ".o"));
            clang_obj(&rt_dir().join(f), &o, true);
            objs.push(o);
        }
        let so = dir.join("g3_scenarios.o");
        clang_obj(&g3_dir().join("g3_scenarios.c"), &so, false);
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

/// The stdout lines of a scenario without the window-table evidence the window model prints to stdout/stderr and without the "    -> n" result lines of the default processing
/// (those are checked where they matter).
fn run(name: &str) -> Vec<String> {
    let r = Command::new(scen_exe()).arg(name).output().expect("run failed");
    assert_eq!(r.status.code(), Some(0), "{name}: {}", String::from_utf8_lossy(&r.stderr));
    String::from_utf8_lossy(&r.stdout).lines().filter(|l| !l.starts_with("G2 evidence") && !l.starts_with("  window table") && !l.starts_with("    handle")).map(|s| s.to_string()).collect()
}
/// Only the lines of the window procedures and of the pumps (no "->" lines, no SETCURSOR lines unless asked).
fn msgs(name: &str, keep_setcursor: bool) -> Vec<String> {
    run(name).into_iter().filter(|l| !l.starts_with("    -> ") && (keep_setcursor || !l.contains("SETCURSOR")) && !l.starts_with("scenario ")).collect()
}

#[test]
fn the_scenario_list_has_24_scenarios() {
    let r = Command::new(scen_exe()).arg("list").output().unwrap();
    let l: Vec<String> = String::from_utf8_lossy(&r.stdout).lines().map(|s| s.to_string()).collect();
    assert_eq!(l.len(), 24, "{l:?}");
    for must in ["move", "click", "capture", "track_leave", "track_hover", "track_hover_edge", "key_target", "key_dead_window", "post_dead_window", "token", "cmp_control", "keys", "syskeys", "timer_periodic", "idle_hook"] {
        assert!(l.iter().any(|x| x == must), "{must}");
    }
}

#[test]
fn a_move_is_a_hit_test_then_wm_setcursor_then_wm_mousemove_in_client_coordinates_of_the_deepest_window() {
    // Learn WM_NCHITTEST: sent to the window beneath the cursor, lParam = screen x, y; DefWindowProc answers HTCLIENT (1) for a point in the client area (S-nchittest-client).
    // Learn WM_SETCURSOR: wParam = the window, lParam low word = the hit-test result, high word = the mouse message (0x200); DefWindowProc passes it to the parent first (the child's parent A sees it too).
    // Learn WM_MOUSEMOVE: wParam MK_* (none down), lParam client x, y: the child's client origin is screen (110,110): (150,130) -> (40,20).
    let o = msgs("move", true);
    assert_eq!(
        o[..7],
        [
            "move to (150,130):",
            "pending 1",
            "  child msg 84 NCHITTEST x 150 y 130",
            "  child msg 20 SETCURSOR wp child hit 1 mouse 200",
            "  A msg 20 SETCURSOR wp child hit 1 mouse 200",
            "  child msg 200 wp 0 x 40 y 20",
            "pump 1",
        ]
    );
    let r = run("move");
    // the hit test and the default result lines: NCHITTEST -> 1 (HTCLIENT)
    assert!(r.iter().any(|l| l == "    -> 1"));
}

#[test]
fn consecutive_moves_are_one_wm_mousemove_at_the_last_point_and_getcursorpos_has_it() {
    // S-mousemove-coalesce (a choice: Windows combines consecutive moves; the pages read do not say how)
    let o = msgs("coalesce", false);
    assert_eq!(o, ["  A msg 84 NCHITTEST x 320 y 270", "  A msg 200 wp 0 x 220 y 170", "pump 1", "cursor 320 270"]);
}

#[test]
fn a_click_on_an_inactive_window_asks_mouseactivate_activates_with_wa_clickactive_and_posts_the_button_messages() {
    // Learn WM_MOUSEACTIVATE: wParam the top-level parent, lParam low = hit test (1), high = the mouse message (0x201); DefWindowProc: MA_ACTIVATE (1) for a top-level window (S-mouseactivate-default);
    // activation: WM_NCACTIVATE TRUE (0x86 wp 1), WM_ACTIVATE with WA_CLICKACTIVE = 2 (Learn WM_ACTIVATE); WM_SETCURSOR carries the button message; WM_LBUTTONDOWN wParam MK_LBUTTON (1);
    // the move before it has wParam 0, the release 0. GetKeyState follows the messages the thread read (Learn GetKeyState); GetAsyncKeyState the hardware.
    let o = msgs("click", true);
    assert_eq!(
        o,
        [
            "click on A at (300,250):",
            "GetKeyState(VK_LBUTTON) before the message: 0",
            "GetAsyncKeyState(VK_LBUTTON) after the host event: 8000",
            "  A msg 84 NCHITTEST x 300 y 250",
            "  A msg 20 SETCURSOR wp A hit 1 mouse 200",
            "  A msg 200 wp 0 x 200 y 150",
            "  A msg 84 NCHITTEST x 300 y 250",
            "  A msg 21 MOUSEACTIVATE top A hit 1 mouse 201",
            "  A msg 86 wp 1 lp 0",
            "  A msg 6 wp 2 lp 0",
            "  A msg 20 SETCURSOR wp A hit 1 mouse 201",
            "  A msg 201 wp 1 x 200 y 150",
            "pump 2",
            "GetKeyState(VK_LBUTTON) after: 8000",
            "  A msg 84 NCHITTEST x 300 y 250",
            "  A msg 202 wp 0 x 200 y 150",
            "pump 1",
            "GetKeyState(VK_LBUTTON) after the release: 0",
            "active A",
        ]
    );
}

#[test]
fn while_a_window_has_the_capture_it_gets_client_messages_with_coordinates_outside_its_client_area_and_no_wm_setcursor() {
    // Learn WM_NCHITTEST / WM_MOUSEMOVE / WM_LBUTTONDOWN: "If the mouse is captured, the message is sent/posted to the window that has captured the mouse"; WM_SETCURSOR: "Sent ... if the mouse causes the
    // cursor to move within a window and mouse input is not captured". B's origin is (500,100): (300,250) -> (-200,150). After ReleaseCapture B gets WM_CAPTURECHANGED (0x215, Learn) and A is hit again.
    let o = msgs("capture", true);
    assert_eq!(
        o,
        [
            "captured by B; click on A at (300,250):",
            "  B msg 84 NCHITTEST x 300 y 250",
            "  B msg 200 wp 0 x -200 y 150",
            "  B msg 84 NCHITTEST x 300 y 250",
            "  B msg 201 wp 1 x -200 y 150",
            "pump 2",
            "  B msg 215 wp 0 lp 0",
            "released",
            "  A msg 84 NCHITTEST x 300 y 250",
            "  A msg 202 wp 0 x 200 y 150",
            "pump 1",
        ]
    );
}

#[test]
fn a_disabled_window_gets_no_mouse_input_and_nothing_goes_to_the_window_beneath() {
    // S-disabled-drops (Learn WindowFromPoint: "does not retrieve a handle to a hidden or disabled window"); EnableWindow(FALSE) sends WM_CANCELMODE (0x1f) and WM_ENABLE (0xa) (decision 0123).
    let o = msgs("disabled", true);
    assert_eq!(o, ["  A msg 1f wp 0 lp 0", "  A msg a wp 0 lp 0", "A is disabled; click on A at (300,250):", "pump 0"]);
}

#[test]
fn a_second_press_in_time_and_place_on_a_cs_dblclks_class_is_wm_lbuttondblclk_and_a_late_one_is_not() {
    // Learn (About Mouse Input, double-click): class style CS_DBLCLKS; S-doubleclick-time 500 ms, rectangle 4 x 4. Second press 100 ms later: 0x203 with MK_LBUTTON; third after 600 ms: 0x201 again.
    let o: Vec<String> = msgs("dblclk", false).into_iter().filter(|l| l.contains("msg 20") && !l.contains("msg 200")).collect();
    assert_eq!(o, ["  C msg 201 wp 1 x 20 y 20", "  C msg 202 wp 0 x 20 y 20", "  C msg 203 wp 1 x 20 y 20", "  C msg 202 wp 0 x 20 y 20", "  C msg 201 wp 1 x 20 y 20", "  C msg 202 wp 0 x 20 y 20"]);
}

#[test]
fn an_httransparent_answer_drops_the_event() {
    // S-httransparent: the window beneath is not tried
    let o = msgs("hit_transparent", false);
    assert_eq!(o, ["B answers HTTRANSPARENT; move to (600,200):", "  B msg 84 NCHITTEST x 600 y 200", "pump 0"]);
}

#[test]
fn mouseactivate_answers_decide_whether_the_window_is_activated_and_whether_the_message_is_discarded() {
    // Learn WM_MOUSEACTIVATE: MA_ACTIVATEANDEAT (2) "Activates the window, and discards the mouse message"; MA_NOACTIVATE (3) "Does not activate the window, and does not discard the mouse message".
    let a = msgs("mouseactivate", false);
    assert_eq!(
        a,
        [
            "B answers MA_ACTIVATEANDEAT; press on B at (600,200):",
            "  B msg 84 NCHITTEST x 600 y 200",
            "  B msg 200 wp 0 x 100 y 100",
            "  B msg 84 NCHITTEST x 600 y 200",
            "  B msg 21 MOUSEACTIVATE top B hit 1 mouse 201",
            "  B msg 86 wp 1 lp 0",
            "  B msg 6 wp 2 lp 0",
            "pump 1",
            "active B",
            "  B msg 84 NCHITTEST x 600 y 200",
            "  B msg 202 wp 0 x 100 y 100",
        ]
    );
    let n = msgs("mouseactivate_no", false);
    assert_eq!(
        n,
        [
            "  A msg 86 wp 1 lp 0",
            "  A msg 6 wp 1 lp 0",
            "A is active; B answers MA_NOACTIVATE; press on B at (600,200):",
            "  B msg 84 NCHITTEST x 600 y 200",
            "  B msg 200 wp 0 x 100 y 100",
            "  B msg 84 NCHITTEST x 600 y 200",
            "  B msg 21 MOUSEACTIVATE top B hit 1 mouse 201",
            "  B msg 201 wp 1 x 100 y 100",
            "pump 2",
            "active A",
            "press on A (already active): MOUSEACTIVATE is not asked",
            "  A msg 84 NCHITTEST x 300 y 250",
            "  A msg 200 wp 1 x 200 y 150",
            "  A msg 84 NCHITTEST x 300 y 250",
            "  A msg 201 wp 1 x 200 y 150",
            "pump 2",
        ]
    );
}

#[test]
fn track_mouse_event_leave_is_posted_when_the_pointer_goes_to_another_window_and_at_once_when_it_is_not_over_the_window() {
    // Learn TRACKMOUSEEVENT: TME_LEAVE "If the mouse is not over the specified window or area, a leave notification is generated immediately and no further tracking is performed"; WM_MOUSELEAVE (0x2a3, wParam and
    // lParam 0) "All tracking requested by TrackMouseEvent is canceled when this message is generated". TME_QUERY fills the structure with the current tracking (flags 2 = TME_LEAVE, hover time 0).
    let o = msgs("track_leave", false);
    assert_eq!(
        o,
        [
            "  B msg 84 NCHITTEST x 600 y 200",
            "  B msg 200 wp 0 x 100 y 100",
            "pointer over B; TME_LEAVE on B:",
            "ret 1",
            "query: ret 1 flags 2 hwnd B hover 0",
            "pump (nothing yet) 0",
            "move to A (300,250):",
            "  A msg 84 NCHITTEST x 300 y 250",
            "  B msg 2a3 wp 0 lp 0",
            "  A msg 200 wp 0 x 200 y 150",
            "pump 2",
            "query after the leave: flags 0",
            "TME_LEAVE on B while the pointer is over A: leave at once",
            "  B msg 2a3 wp 0 lp 0",
            "pump 1",
        ]
    );
}

#[test]
fn track_mouse_event_hover_comes_after_the_hover_time_of_400_ms_stays_inside_the_rectangle_and_restarts_when_the_pointer_leaves_it() {
    // Learn TRACKMOUSEEVENT: HOVER_DEFAULT -> "initially the menu drop-down time, which is 400 milliseconds" (TME_QUERY returns the actual time-out, 400, "and not HOVER_DEFAULT"); WM_MOUSEHOVER (0x2a1) wParam MK_*
    // (none), lParam client x, y: (100,100); "Hover tracking stops when WM_MOUSEHOVER is generated"; the hover rectangle is the double-click rectangle (4 x 4, S-hover-defaults): a move of 2 px keeps the time, a move
    // of 20 px at +90 ms restarts it, so the hover comes at +190 ms (not +100 ms); lParam (122,121)... the pointer is at (622,221): client (122,121) = 0x79007a.
    let o = msgs("track_hover", false);
    assert_eq!(
        o,
        [
            "  B msg 84 NCHITTEST x 600 y 200",
            "  B msg 200 wp 0 x 100 y 100",
            "TME_HOVER with HOVER_DEFAULT on B at 3600000 ms:",
            "ret 1",
            "query: flags 1 hover 400",
            "399 ms later: pump 0",
            "  B msg 2a1 wp 0 lp 640064",
            "400 ms later (3600400 ms): pump 1",
            "query: flags 0 (hover tracking stopped)",
            "again, 100 ms hover; a move of 2 px stays inside the rectangle, a move of 20 px restarts the time:",
            "  B msg 84 NCHITTEST x 602 y 201",
            "  B msg 200 wp 0 x 102 y 101",
            "2 px at +60 ms: pump 1",
            "  B msg 84 NCHITTEST x 622 y 221",
            "  B msg 200 wp 0 x 122 y 121",
            "20 px at +90 ms: pump 1",
            "+189 ms (99 ms after the move): pump 0",
            "  B msg 2a1 wp 0 lp 79007a",
            "+190 ms (100 ms after the move): pump 1",
        ]
    );
}

#[test]
fn the_hover_rectangle_is_4_by_4_a_move_of_2_px_keeps_the_time_and_a_move_of_3_px_restarts_it() {
    // Learn TrackMouseEvent: the pointer "is considered to be hovering when it stays within a specified rectangle for a specified period of time"; the rectangle is the double-click rectangle, 4 x 4 around the
    // point where the time started (S-hover-defaults): (600,200) +- 2. A move to x 602 is on the edge and inside; x 603 is outside and the time restarts from that move (+70 ms): the first time-out (+100 ms)
    // runs out without a message, the restarted one comes at +170 ms with the client point (103,100) = 0x640067.
    let o = msgs("track_hover_edge", false);
    assert_eq!(
        o,
        [
            "  B msg 84 NCHITTEST x 600 y 200",
            "  B msg 200 wp 0 x 100 y 100",
            "hover 100 ms from the anchor (600,200); the rectangle is 4 x 4:",
            "  B msg 84 NCHITTEST x 602 y 200",
            "  B msg 200 wp 0 x 102 y 100",
            "2 px at +60 ms (on the edge, inside): pump 1",
            "  B msg 84 NCHITTEST x 603 y 200",
            "  B msg 200 wp 0 x 103 y 100",
            "3 px at +70 ms (outside, restarts at the new point): pump 1",
            "+100 ms (the first time-out has run out, the restarted one has not): pump 0",
            "+169 ms: pump 0",
            "  B msg 2a1 wp 0 lp 640067",
            "+170 ms (100 ms after the move): pump 1",
        ]
    );
}

#[test]
fn a_key_goes_to_the_focus_window_of_the_foreground_thread_and_not_to_its_active_window() {
    // Learn WM_KEYDOWN: "Posted to the window with the keyboard focus when a nonsystem key is pressed". The foreground window is A (active); the focus window is the child of A: the key messages and the character
    // message are for the child.
    let o = msgs("key_target", false);
    let i = o.iter().position(|l| l.starts_with("foreground A")).expect("the marker line");
    assert_eq!(
        o[i..],
        [
            "foreground A, focus the child of A: a key goes to the focus window, not to the active window:",
            "  child msg 100 wp 41 lp 1e0001",
            "  child msg 102 wp 61 lp 1e0001",
            "  child msg 101 wp 41 lp c01e0001",
            "pump 3",
        ]
    );
}

#[test]
fn a_key_up_that_was_made_for_a_window_destroyed_meanwhile_goes_to_the_window_that_has_the_focus_now() {
    // decision 0218 (ship3). Windows decides the window of a hardware message when the message is retrieved (Learn About Keyboard Input; WM_KEYUP: the focus window, else the active window). The key up of Enter
    // was made for the child while the key down was handled; the handling destroyed the child (a dialog closed by Enter). The key up must reach A (the active window), not the gone child.
    let o = run("key_dead_window");
    let i = o.iter().position(|l| l.starts_with("Enter down and up")).expect("the marker line");
    let t: Vec<&String> = o[i..].iter().filter(|l| l.starts_with("  retrieved") || l.starts_with("  peeked") || l.starts_with("DestroyWindow")).collect();
    assert_eq!(
        t,
        [
            "  retrieved 100 for the child 1",
            "  peeked 101 for the child 1",
            "DestroyWindow(child): ret 1",
            "  retrieved 101 for A 1 for the child 0",
        ]
    );
}

#[test]
fn the_posted_messages_of_a_destroyed_window_are_removed_and_those_of_another_window_are_delivered() {
    // decision 0218 (ship3): DestroyWindow removes the messages that are posted to the window (the classic page: "flushes the thread message queue"; Wine 10.0 queue_cleanup_window: UNOFFICIAL). A VCL box that Enter closes
    // would otherwise get the WM_CHAR that TranslateMessage posted to it before the key down destroyed it.
    let o = msgs("post_dead_window", false);
    let i = o.iter().position(|l| l.starts_with("posted WM_CHAR a")).expect("the marker line");
    assert_eq!(
        o[i..],
        [
            "posted WM_CHAR a to the child of A, b to A, c to the child of A; DestroyWindow(child):",
            "  child msg 2 wp 0 lp 0",
            "  child msg 82 wp 0 lp 0",
            "DestroyWindow(child): ret 1",
            "  A msg 102 wp 62 lp 1e0001",
            "pump 1",
        ]
    );
}

#[test]
fn the_elevation_check_of_the_guest_opens_a_token_gets_token_elevation_zero_and_closes_it_with_closehandle() {
    // Learn OpenProcessToken: nonzero on success, the token handle is closed with CloseHandle. Learn GetTokenInformation: ReturnLength "receives the number of bytes needed"; if it is larger than the buffer length "the function fails
    // and stores no data in the buffer". Learn TOKEN_ELEVATION: { DWORD TokenIsElevated } (zero = not elevated: S-token-not-elevated); the short-buffer error 122 is S-token-short-buffer; the second CloseHandle of a closed
    // handle fails with ERROR_INVALID_HANDLE 6 (as the other handle regions of the runtime).
    let o = msgs("token", false);
    assert_eq!(
        o,
        [
            "OpenProcessToken(current process, TOKEN_QUERY): ret 1",
            "token handle nonzero 1",
            "GetTokenInformation(TokenElevation, 4 bytes): ret 1 TokenIsElevated 0 ReturnLength 4",
            "GetTokenInformation(TokenElevation, 2 bytes): ret 0 ReturnLength 4 buffer eeeeeeee last error 122",
            "CloseHandle(token): ret 1",
            "CloseHandle(token) again: ret 0 last error 6",
        ]
    );
}

#[test]
fn compare_string_orders_a_white_space_control_unit_against_a_printable_ascii_unit() {
    // S-cmp-control (decision 0136, a CHOICE WITHOUT A SOURCE, same reading as S-cmp-general-punctuation of decision 0132): the first difference is a control unit 0x09 to 0x0d against a printable ASCII unit: the code points decide,
    // the control unit is the smaller. With NORM_IGNORECASE: 'O' folds to 'o' (0x6f), still above 0x0d. Equal strings are equal (2). Hand computation: CR (0x0d) < 'o' (0x6f): 1; reversed: 3.
    // Decision 0284 (A, on top of 0284c: the sort table is always on): against the SPACE the primary weights of Microsoft's table decide, not the code points: CR is SM 7 AW 9 and the space SM 7 AW 2 (wall_ia2.rs, the rows of the table), so CR is the greater: CR vs space 3, space vs CR 1.
    let o = msgs("cmp_control", false);
    assert_eq!(
        o,
        [
            "CompareStringW(CR vs 'O'): 1",
            "CompareStringW('O' vs CR): 3",
            "CompareStringW(LF vs 'O'): 1",
            "CompareStringW(TAB vs 'O'): 1",
            "CompareStringW(CR vs space): 3",
            "CompareStringW(space vs CR): 1",
            "CompareStringW(a CR b vs a o b): 1",
            "CompareStringW(CR vs CR): 2",
        ]
    );
}

#[test]
fn track_mouse_event_cancel_validates_its_arguments_and_removes_only_the_named_kind() {
    // Learn TRACKMOUSEEVENT: TME_CANCEL "should also specify the type of tracking that it wants to cancel"; a cbSize that is not the structure size and a window that is not live fail (0): S-tme-invalid
    // (ERROR_INVALID_PARAMETER 87 and ERROR_INVALID_WINDOW_HANDLE 1400 are choices).
    let o = msgs("track_cancel", false);
    assert_eq!(
        o[2..],
        ["query: flags 3 hover 100", "cancel hover: flags 2", "cancel leave: flags 0 hwnd -", "cbSize 16: ret 0 error 87", "invalid window: ret 0 error 1400"]
    );
}

#[test]
fn key_messages_have_the_lparam_of_learn_and_translatemessage_posts_the_character_of_the_us_layout() {
    // Learn WM_KEYDOWN: wParam the virtual-key code, lParam bits 0-15 repeat count (1), 16-23 scan code ('A' = 0x1e), 24 extended, 30 previous key state, 31 transition (0 for a down; the up message has 30 and 31 set:
    // 0xc01e0001). Learn WM_CHAR: posted when WM_KEYDOWN is translated by TranslateMessage; wParam the character code ('a' = 0x61, 'A' = 0x41 with Shift: the generic VK_SHIFT 0x10 is the wParam of the shift key itself).
    // An auto-repeat (the key is down when it goes down again) has bit 30 set (0x40300001) in the key message and in the character message. The right arrow has the extended-key bit 24 (0x14d0001).
    let o = msgs("keys", false);
    assert_eq!(
        o[3..],
        [
            "focus A; key down 'A' (65), up:",
            "  A msg 100 wp 41 lp 1e0001",
            "  A msg 102 wp 61 lp 1e0001",
            "pump 2",
            "  A msg 101 wp 41 lp c01e0001",
            "pump 1",
            "shift (160) down, 'A' down, 'A' up, shift up:",
            "  A msg 100 wp 10 lp 2a0001",
            "  A msg 100 wp 41 lp 1e0001",
            "  A msg 102 wp 41 lp 1e0001",
            "  A msg 101 wp 41 lp c01e0001",
            "  A msg 101 wp 10 lp c02a0001",
            "pump 5",
            "auto-repeat of 'B' (66) and the right arrow (39, extended):",
            "  A msg 100 wp 42 lp 300001",
            "  A msg 102 wp 62 lp 300001",
            "  A msg 100 wp 42 lp 40300001",
            "  A msg 102 wp 62 lp 40300001",
            "  A msg 101 wp 42 lp c0300001",
            "  A msg 100 wp 27 lp 14d0001",
            "  A msg 101 wp 27 lp c14d0001",
            "pump 7",
            "Space (32), Enter (13) and Escape (27), each down and up (the keys of the welcome wizard):",
            "  A msg 100 wp 20 lp 390001",
            "  A msg 102 wp 20 lp 390001",
            "  A msg 101 wp 20 lp c0390001",
            "  A msg 100 wp d lp 1c0001",
            "  A msg 102 wp d lp 1c0001",
            "  A msg 101 wp d lp c01c0001",
            "  A msg 100 wp 1b lp 10001",
            "  A msg 102 wp 1b lp 10001",
            "  A msg 101 wp 1b lp c0010001",
            "pump 9",
        ]
    );
}

#[test]
fn alt_f4_is_wm_syskeydown_with_the_context_bit_and_the_default_processing_closes_the_window() {
    // Learn WM_SYSKEYDOWN: posted "when the user presses the F10 key ... or holds down the ALT key and then presses another key"; lParam bit 29 (context code) is 1 while Alt is down: F4 = 0x203e0001.
    // S-altf4: the default processing makes WM_SYSCOMMAND SC_CLOSE (0xf060), whose default is WM_CLOSE (0x10), whose default is DestroyWindow (decision 0123): WM_DESTROY (2) to A, then the child, WM_NCDESTROY (0x82).
    let o = msgs("syskeys", false);
    assert_eq!(
        o[3..],
        [
            "alt (164) down, F4 (115) down, F4 up, alt up:",
            "  A msg 104 wp 12 lp 380001",
            "  A msg 104 wp 73 lp 203e0001",
            "  A msg 112 wp f060 lp 0",
            "  A msg 10 wp 0 lp 0",
            // decision 0222: DestroyWindow of the active window first hides it and the active (and focus) state is transferred to the next window (Learn Window Features, Window Destruction: "If the window being destroyed is
            // the active window, both the active and focus states are transferred to another window"): A gets WM_NCACTIVATE (0x86) / WM_ACTIVATE (6) FALSE / WA_INACTIVE, C (the next window in the z-order) TRUE / WA_ACTIVE.
            "  A msg 86 wp 0 lp 7f990010",
            "  A msg 6 wp 0 lp 7f990010",
            "  C msg 86 wp 1 lp 7f990004",
            "  C msg 6 wp 1 lp 7f990004",
            "  A msg 2 wp 0 lp 0",
            "  child msg 2 wp 0 lp 0",
            "  child msg 82 wp 0 lp 0",
            "  A msg 82 wp 0 lp 0",
            "pump 2",
            "IsWindow(A) after Alt+F4: 0",
        ]
    );
}

#[test]
fn getkeystate_follows_the_messages_getasynckeystate_the_hardware_and_getkeyboardstate_and_mapvirtualkey_answer_the_us_layout() {
    // Learn GetKeyState: bit 15 down, bit 0 toggled; "changes as a thread reads key messages from its message queue" (the async state is 0x8000 at once, the sync state only after pump); CapsLock down+up is toggled
    // (bit 0) and up: 1. VK_LCONTROL (0xa2) down sets VK_CONTROL (0x11) too. GetKeyboardState: 256 bytes, bit 7 down, bit 0 toggled. MapVirtualKeyW: 'A' -> scan 0x1e, scan 0x1e -> 'A', MAPVK_VK_TO_CHAR 'A' -> 0x41 (S-us-layout).
    let o = msgs("keystate", false);
    assert_eq!(
        o[3..],
        [
            "async VK_CONTROL 8000",
            "sync  VK_CONTROL before the message 0",
            "  A msg 100 wp 14 lp 3a0001",
            "  A msg 101 wp 14 lp c03a0001",
            "  A msg 100 wp 11 lp 1d0001",
            "sync  VK_CONTROL after 8000",
            "sync  VK_LCONTROL 8000, VK_RCONTROL 0",
            "sync  VK_CAPITAL 1",
            "GetKeyboardState ret 1: [0x11]=80 [0xA2]=80 [0x14]=1 [0x41]=0",
            "MapVirtualKeyW(65, VK_TO_VSC) 1e, (0x1E, VSC_TO_VK) 41, (65, VK_TO_CHAR) 41",
        ]
    );
}

#[test]
fn getmessagetime_and_getmessagepos_describe_the_last_retrieved_message() {
    // Learn GetMessageTime: the time the message was created; GetMessagePos: the cursor position for the last message (x low short, y high short): (600,200) = 0xc80258.
    let o = msgs("msgpos", false);
    assert_eq!(o, ["  B msg 84 NCHITTEST x 600 y 200", "peeked msg 200 time 3600000 pt 600,200", "GetMessageTime 3600000", "GetMessagePos c80258"]);
}

#[test]
fn a_window_timer_is_delivered_at_exactly_its_period_on_the_virtual_clock_when_the_host_moves_the_clock_to_each_deadline() {
    // Learn SetTimer: WM_TIMER every uElapse ms. The idle hook (decision 0136) is asked at every idle and answers "move the clock to the next deadline"; the virtual clock starts at 3,600,000 ms (decision 0017).
    // Hand computation: SetTimer at 3600000 with 100 ms -> deliveries at +100, +200, +300, +400, +500; each delivery is at its own deadline (late by 0): the intervals are 100 ms exactly.
    let o = msgs("timer_periodic", false);
    let d: Vec<&String> = o.iter().filter(|l| l.starts_with("WM_TIMER")).collect();
    assert_eq!(d.len(), 5);
    for (k, l) in d.iter().enumerate() {
        assert_eq!(**l, format!("WM_TIMER id 7 delivered at {} ms (+{})", 3_600_000 + 100 * (k + 1), 100 * (k + 1)));
    }
    let log: Vec<&String> = o.iter().filter(|l| l.starts_with("log #")).collect();
    for (k, l) in log.iter().enumerate() {
        assert_eq!(**l, format!("log #{k} id 7 period 100 due {0} delivered {0} late 0 ticks", 3_600_000 + 100 * (k + 1)));
    }
}

#[test]
fn the_idle_hook_is_given_the_next_deadline_and_the_scheduler_never_skips_it_even_when_the_host_asks_for_more() {
    // The scenario's hook always asks for the clock at +10,000 ms; fle_thread.c clamps the request to `next` (the timer's deadline): the timer of 250 ms is delivered at 250, 500, 750 and the hook is
    // called with now/next = (3600000, 3600250), (3600250, 3600500), (3600500, 3600750).
    let o = msgs("idle_hook", false);
    assert_eq!(
        o,
        [
            "SetTimer(A, 9, 250 ms) at 3600000 ms; the host asks for the clock at +10000 ms at every idle",
            "  idle now 3600000 next 3600250",
            "WM_TIMER id 9 at 3600250 ms",
            "  idle now 3600250 next 3600500",
            "WM_TIMER id 9 at 3600500 ms",
            "  idle now 3600500 next 3600750",
            "WM_TIMER id 9 at 3600750 ms",
            "idle calls 3",
        ]
    );
}

#[test]
fn the_new_import_rows_are_appended_at_the_end_of_the_response_table_in_a_fixed_order() {
    // decision 0015: "append only"; the pseudo-VA of a row is its position. The last 9 rows of decision 0136 are the end of TABLE, after decision 0132's InternetCloseHandle.
    let src = std::fs::read_to_string(manifest().join("src/imports_table.rs")).unwrap();
    let rows: Vec<&str> = src.lines().filter(|l| l.trim_start().starts_with("(\"") && l.contains("\"fle_imp_")).collect();
    // decision 0139 appends 3 rows after these (tests/rows_0139/mod.rs): the group ends at the row of GetTokenInformation, not at the end of the table
    let end = rows.iter().position(|l| l.contains("\"GetTokenInformation\"")).unwrap() + 1;
    let tail: Vec<String> = rows[end - 11..end].iter().map(|l| l.split('"').nth(3).unwrap().to_string()).collect();
    assert_eq!(tail, ["InternetCloseHandle", "_TrackMouseEvent", "GetMessageTime", "GetMessagePos", "GetMessageExtraInfo", "GetDoubleClickTime", "GetKeyboardState", "MapVirtualKeyW", "MessageBeep", "OpenProcessToken", "GetTokenInformation"]);
}

#[test]
fn the_hooks_in_the_other_runtime_files_are_local() {
    // fle_thread.c: msg_ready asks fle_input_has_event; the idle hook has the three-argument form; fle_win.c: DefWindowProcW's default case asks fle_input_defproc, fill_msg notes the message,
    // fle_win_synth_message asks fle_input_synth first, TranslateMessage asks fle_input_translate, WaitMessage looks at the synthesised message.
    let th = std::fs::read_to_string(rt_dir().join("fle_thread.c")).unwrap();
    assert!(th.contains("fle_input_has_event(t->idx)"));
    assert!(th.contains("int fle_thread_idle_hook(uint64_t now, uint64_t next, uint64_t *advance_to) __attribute__((weak));"));
    assert!(th.contains("if (have && adv > nx) adv = nx;"), "a deadline is never skipped");
    let w = std::fs::read_to_string(rt_dir().join("fle_win.c")).unwrap();
    for must in ["fle_input_defproc(api, h, msg, wp, lp, &res)", "fle_input_note_message(", "fle_input_synth(t, fh, fmin, fmax, remove, qs, out)", "fle_input_translate(\"TranslateMessage\", pmsg)", "fle_win_synth_message(fle_thread_current(), 0, 0, 0, 0, 0xFFFFu, &sm)"] {
        assert!(w.contains(must), "{must}");
    }
}
