//! Tests of decision 0144 (docs/decisions/0144-g8a-open-project.md, PROVISIONAL): how a file reaches the engine (runtime/fle_appfile.c), NATIVE side.
//!
//! What a cargo test can check without the lifted engine: harness/web/g8/g8_scenarios.c is built with the REAL runtime files (fle_appfile.c, fle_thread.c, fle_win.c, fle_input.c, fle_env.c ...) and a stub guest window
//! procedure, and each scenario's stdout is compared with the lines that are written below BY HAND from the Learn pages (WM_COPYDATA, COPYDATASTRUCT, SendMessage, GetCommandLineW, SHAddToRecentDocs, GetDpiForMonitor,
//! the UTF-8 and UTF-16 encodings) and from the choices of the decision (S-g8a-... names). Nothing is compared with a real Windows. Also: source checks (the new rows are appended at the END of the response table in a
//! fixed order, the hooks in the other runtime files are local, the new file has nothing that wasm cannot run).

mod etxt_common;
mod rows_0144;
mod rows_0147;
mod rows_0267;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
/// The runtime directory. FLE_FR19_RT (a copy of lift/runtime) overrides it: the mutation runs of the run record change a file of such a copy and expect a test to fail.
fn rt_dir() -> PathBuf {
    if let Ok(d) = std::env::var("FLE_FR19_RT") {
        return PathBuf::from(d);
    }
    manifest().join("../../runtime")
}
fn g8_dir() -> PathBuf {
    manifest().join("../../harness/web/g8")
}
fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-wallFR19/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

const FILES: [&str; 23] = [
    "fle_thread.c", "fle_win.c", "fle_class.c", "fle_event.c", "fle_sys.c", "fle_imports.c", "fle_proc.c", "fle_callback.c", "fle_env.c", "fle_vfs.c", "fle_vmap.c", "fle_vol.c", "fle_host.c",
    "fle_time.c", "fle_x87ctl.c", "fle_mxcsr.c", "fle_x87.c", "fle_locale.c", "fle_input.c", "fle_comctl.c", "fle_cursor.c", "fle_token.c", "fle_appfile.c",
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
        let so = dir.join("g8_scenarios.o");
        clang_obj(&g8_dir().join("g8_scenarios.c"), &so, false);
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

fn filtered(stdout: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(stdout).lines().filter(|l| !l.starts_with("G2 evidence") && !l.starts_with("  window table") && !l.starts_with("    handle")).map(|s| s.to_string()).collect()
}
/// The stdout lines of a scenario (exit status 0), without the window-table evidence the window model prints, and without the closing "scenario NAME done" line.
fn run_env(name: &str, cmdline: Option<&str>) -> Vec<String> {
    let mut c = Command::new(scen_exe());
    c.arg(name).env_remove("FLE_CMDLINE");
    if let Some(v) = cmdline {
        c.env("FLE_CMDLINE", v);
    }
    let r = c.output().expect("run failed");
    assert_eq!(r.status.code(), Some(0), "{name}: {}", String::from_utf8_lossy(&r.stderr));
    let mut l = filtered(&r.stdout);
    assert_eq!(l.pop().as_deref(), Some(format!("scenario {name} done").as_str()), "{name}: the scenario ends");
    l
}
fn run(name: &str) -> Vec<String> {
    run_env(name, None)
}
/// A scenario that must stop: the process ends by a signal (abort), and stderr has `msg`; returns the stdout lines.
fn run_stop(name: &str, cmdline: Option<&str>, msg: &str) -> Vec<String> {
    let mut c = Command::new(scen_exe());
    c.arg(name).env_remove("FLE_CMDLINE");
    if let Some(v) = cmdline {
        c.env("FLE_CMDLINE", v);
    }
    let r = c.output().expect("run failed");
    assert_ne!(r.status.code(), Some(0), "{name} must stop");
    let e = String::from_utf8_lossy(&r.stderr).to_string() + &String::from_utf8_lossy(&r.stdout);
    assert!(e.contains(msg), "{name}: {msg:?} not in {e}");
    filtered(&r.stdout)
}

const DEFAULT_CMD: &str = "W: \"C:\\Program Files\\Image-Line\\FL Studio 21\\FLEngine_x64.dll\"";

#[test]
fn the_scenario_list_has_the_12_scenarios() {
    let r = Command::new(scen_exe()).arg("list").output().unwrap();
    let l: Vec<String> = String::from_utf8_lossy(&r.stdout).lines().map(|s| s.to_string()).collect();
    assert_eq!(l.len(), 12, "{l:?}");
    for must in ["cmdline_w", "cmdline_a", "open_no_window", "open_delivery", "open_other_thread", "open_errors", "defproc_copydata", "recentdocs_ok", "recentdocs_pidl", "dpi"] {
        assert!(l.iter().any(|x| x == must), "{must}");
    }
}

#[test]
fn without_fle_cmdline_the_command_line_is_the_quoted_virtual_dll_path_as_before_decision_0144() {
    // decision 0018 (fle_env.c): "<virtual path>" in quotes, no arguments; an unset and an EMPTY FLE_CMDLINE are the same thing
    assert_eq!(run("cmdline_w")[0], DEFAULT_CMD);
    assert_eq!(run_env("cmdline_w", Some(""))[0], DEFAULT_CMD);
    assert_eq!(run("cmdline_a")[0], "A: \"C:\\Program Files\\Image-Line\\FL Studio 21\\FLEngine_x64.dll\"");
}

#[test]
fn fle_cmdline_is_the_text_that_getcommandlinew_returns_as_utf16_and_getcommandlinea_as_ascii() {
    // Learn GetCommandLineW: the command-line string of the process (the quoted program, then the arguments); the text is UTF-8 in the environment and UTF-16 in the guest: a BMP character is one unit, a character
    // above U+FFFF (U+1F600) is the surrogate pair D83D DE00 (RFC 2781)
    let cmd = "\"C:\\a b\\FL64.exe\" \"C:\\caf\u{e9} \u{1F600}.flp\"";
    let l = run_env("cmdline_w", Some(cmd));
    assert_eq!(l[0], "W: \"C:\\a b\\FL64.exe\" \"C:\\caf\\u00E9 \\uD83D\\uDE00.flp\"");
    assert_eq!(l[1], "second call, same pointer: 1");
    // Learn GetCommandLineA: the conversion of the Unicode command line to the process code page; an ASCII text is the same text
    let a = run_env("cmdline_a", Some("\"C:\\x\\FL64.exe\" \"C:\\Users\\user\\Documents\\Basic.flp\""));
    assert_eq!(a[0], "A: \"C:\\x\\FL64.exe\" \"C:\\Users\\user\\Documents\\Basic.flp\"");
    // a character above 0x7F cannot be given as the 8-bit text without the code-page table: the run stops by name (S-g8a-cmdline-ascii)
    run_stop("cmdline_a", Some("\"C:\\caf\u{e9}.flp\""), "GetCommandLineA: FLE_CMDLINE has a character above 0x7F");
}

#[test]
fn opening_without_a_main_window_fails_with_minus_1_and_a_window_of_another_class_is_not_the_main_window() {
    assert_eq!(run("open_no_window"), ["open with no main window: -1", "main window handle: 0"]);
    assert_eq!(run("open_other_class"), ["open with a window of another class only: -1"]);
}

#[test]
fn the_copydata_is_sent_to_the_main_form_and_answered_only_when_its_thread_retrieves_messages() {
    // Learn WM_COPYDATA: wParam = the sending window (the host has none: 0), lParam = a COPYDATASTRUCT {dwData, cbData, lpData}; the engine's own sender f_117b930 puts 0xA (a path) there and the receiver f_11d1c80
    // takes 10 (decision 0144 reads dwData 10); cbData is (units + 1) * 2 including the UTF-16 NUL. Learn SendMessage: a message sent to a window of another thread "is processed only when the receiving thread
    // executes message retrieval code"; the result is the window procedure's. "C:\Users\user\Documents\Image-Line\FL Studio\Projects\Basic.flp" is 63 units: (63 + 1) * 2 = 128. "D:\second.flp" is 13: 28.
    let o = run("open_delivery");
    assert_eq!(
        o,
        [
            "main window found: 1",
            "request 1",
            "state(1) = 1 result 0",
            "(nothing is delivered until the window's thread retrieves messages)",
            "  main msg 4a WM_COPYDATA wp 0 dwData 10 cbData 128 lpData=C:\\Users\\user\\Documents\\Image-Line\\FL Studio\\Projects\\Basic.flp last_unit_is_nul 1",
            "state(1) = 2 result 1",
            "request 2",
            "state(2) = 1 result 0",
            "  main msg 4a WM_COPYDATA wp 0 dwData 10 cbData 28 lpData=D:\\second.flp last_unit_is_nul 1",
            "state(2) = 2 result 1",
            "state(1) = 2 result 1",
            "state(99) = 0 result 99",
        ]
    );
}

#[test]
fn the_copydata_runs_in_the_context_of_the_thread_that_owns_the_main_form() {
    // Learn SendMessage: "The thread that owns the window ... processes the message when it executes message retrieval code" - not the thread that happens to retrieve messages. The main form belongs to guest thread 1
    // (cooperative threads, decision 0122, thread 0 is the first); thread 0 retrieves messages and gets nothing; the window procedure runs when thread 1 does, and prints its own thread index.
    assert_eq!(
        run("open_other_thread"),
        [
            "thread 1: main form created",
            "main window found: 1",
            "request 1",
            "state(1) = 1 result 0",
            "(thread 0 retrieved messages: the window belongs to another thread, nothing is delivered to it)",
            "state(1) = 1 result 0",
            "  main msg 4a WM_COPYDATA wp 0 dwData 10 cbData 28 lpData=D:\\second.flp last_unit_is_nul 1 thread 1",
            "state(1) = 2 result 1",
        ]
    );
}

#[test]
fn a_path_with_non_ascii_characters_reaches_the_guest_as_utf16() {
    // "C:\caf" + U+00E9 + " " + U+1F600 + ".flp" = 6 + 1 + 1 + 2 + 4 = 14 units, +1 NUL = 15 units = 30 bytes; U+1F600 is the surrogate pair D83D DE00
    assert_eq!(run("open_unicode"), ["request 1", "  main msg 4a WM_COPYDATA wp 0 dwData 10 cbData 30 lpData=C:\\caf\\u00E9 \\uD83D\\uDE00.flp last_unit_is_nul 1"]);
}

#[test]
fn a_bad_path_and_a_full_queue_are_refused_with_their_own_numbers() {
    let o = run("open_errors");
    assert_eq!(&o[..5], ["empty: -2", "NULL: -2", "not UTF-8 (0xff): -2", "overlong 2-byte form of '/': -2", "a surrogate code point (ED A0 80): -2"]);
    // the limit: 520 units (+ the NUL = 521 units = 1042 bytes) is taken, 521 units are not
    assert!(o[5] == "520 units (the limit): 1", "{}", o[5]);
    assert!(o[6].starts_with("  main msg 4a WM_COPYDATA wp 0 dwData 10 cbData 1042 lpData=aaaa") && o[6].ends_with("aaaa last_unit_is_nul 1"), "{}", o[6]);
    assert_eq!(o[7], "521 units: -2");
    // 8 requests wait (FLE_APPFILE_MAX_PENDING), the 9th is refused; after a retrieval the slots are free again; the request numbers count on
    assert_eq!(&o[8..], ["8 pending, the last request number: 9", "a 9th while 8 wait: -3", "after the retrieval, a new request: 10"]);
}

#[test]
fn the_default_window_procedure_does_not_process_wm_copydata_and_answers_false() {
    // Learn WM_COPYDATA: "If the receiving application processes this message, it should return TRUE; otherwise, it should return FALSE" (S-g8a-copydata-default)
    assert_eq!(run("defproc_copydata"), ["DefWindowProcW(WM_COPYDATA) = 0"]);
}

#[test]
fn shaddtorecentdocs_takes_a_path_or_null_and_stops_by_name_for_the_forms_it_does_not_model() {
    assert_eq!(run("recentdocs_ok"), ["SHARD_PATHW returned", "SHARD_PATHA returned", "NULL item (clear all) returned"]);
    run_stop("recentdocs_pidl", None, "SHAddToRecentDocs: this SHARD form of the item is not modelled");
}

#[test]
fn getdpiformonitor_answers_96_for_the_one_virtual_monitor_and_e_invalidarg_for_anything_invalid() {
    // Learn GetDpiForMonitor: PROCESS_DPI_UNAWARE "96 because the app is unaware of any other scale factors"; "The values of *dpiX and *dpiY are identical"; E_INVALIDARG (0x80070057): "The handle, DPI type, or
    // pointers passed in are not valid"; MONITOR_DPI_TYPE is 0, 1, 2. The out values are left alone on a failure (7 is what the scenario put there).
    assert_eq!(
        run("dpi"),
        [
            "type 0: hr 0 dpi 96 96",
            "type 1: hr 0 dpi 96 96",
            "type 2: hr 0 dpi 96 96",
            "type 3: hr 80070057 dpi 7 7",
            "another monitor handle: hr 80070057 dpi 7 7",
            "NULL monitor: hr 80070057",
            "NULL dpiX: hr 80070057",
            "NULL dpiY: hr 80070057",
        ]
    );
}

#[test]
fn the_new_import_rows_are_appended_at_the_end_of_the_response_table_in_a_fixed_order() {
    // decision 0015: "append only"; the pseudo-VA of a row is its position. The last rows are those of decision 0144, right after the 4 of decision 0140 (tests/rows_0140)
    let src = std::fs::read_to_string(manifest().join("src/imports_table.rs")).unwrap();
    let rows: Vec<&str> = src.lines().filter(|l| l.trim_start().starts_with("(\"") && l.contains("\"fle_imp_")).collect();
    let k = rows_0144::ROWS_0144.len();
    let n = rows.len() - rows_0147::ROWS_0147.len() - rows_0267::ROWS_0267.len(); // decision 0147 appends 1 row (tests/rows_0147) after these, decision 0267 1 row (tests/rows_0267) after that
    for (i, (dll, name)) in rows_0144::ROWS_0144.iter().enumerate() {
        let want = format!("(\"{dll}\", \"{name}\", \"fle_imp_{name}\", \"jmp_imp:{name}\"),");
        assert_eq!(rows[n - k + i].trim(), want, "row {i} of decision 0144");
    }
    assert!(rows[n - k - 1].contains("\"GetLayeredWindowAttributes\""), "the row before the new ones is the last row of decision 0140");
    // every new row has its function in the new runtime file and its declaration in the header
    let c = std::fs::read_to_string(rt_dir().join("fle_appfile.c")).unwrap();
    let h = std::fs::read_to_string(rt_dir().join("fle_appfile.h")).unwrap();
    for (_, name) in rows_0144::ROWS_0144 {
        assert!(c.contains(&format!("void fle_imp_{name}(void)\n{{")), "{name} is defined in fle_appfile.c");
        assert!(h.contains(&format!("void fle_imp_{name}(void);")), "{name} is declared in fle_appfile.h");
    }
}

#[test]
fn the_hooks_in_the_other_runtime_files_are_local_and_the_new_file_is_what_wasm_can_run() {
    // fle_env.c: the two weak declarations and one use each; the default text of decision 0018 is still there
    let env = std::fs::read_to_string(rt_dir().join("fle_env.c")).unwrap();
    for must in [
        "const uint16_t *fle_appfile_cmdline_w(void) __attribute__((weak));",
        "const char *fle_appfile_cmdline_a(void) __attribute__((weak));",
        "if (cmd == NULL && fle_appfile_cmdline_w) cmd = (uint16_t *)(uintptr_t)fle_appfile_cmdline_w();",
        "if (cmd == NULL && fle_appfile_cmdline_a) {",
        "static const char q[] = \"\\\"\" FLE_VIRTUAL_DLL_PATH \"\\\"\";",
    ] {
        assert!(env.contains(must), "{must}");
    }
    // fle_win.c, fle_thread.c, fle_gdi.c, fle_gdc.c, fle_layered.c are not touched by decision 0144 (other work edits them)
    for f in ["fle_win.c", "fle_thread.c", "fle_gdi.c", "fle_gdc.c", "fle_layered.c", "fle_visrgn.c"] {
        if let Ok(s) = std::fs::read_to_string(rt_dir().join(f)) {
            assert!(!s.contains("fle_appfile"), "{f}");
        }
    }
    // fle_input.c: WM_COPYDATA in the default processing
    let inp = std::fs::read_to_string(rt_dir().join("fle_input.c")).unwrap();
    assert!(inp.contains("case 0x004Au:   /* WM_COPYDATA (decision 0144) */"));
    // the new file: no host thread-local, no file, no thread, no clock: plain C that emscripten compiles as it is (the export is EMSCRIPTEN_KEEPALIVE under __EMSCRIPTEN__ only)
    let c = std::fs::read_to_string(rt_dir().join("fle_appfile.c")).unwrap();
    for bad in ["__thread", "_Thread_local", "fopen", "fprintf(", "pthread", "unistd.h", "clock_gettime", "time(", "signal("] {
        assert!(!c.contains(bad), "fle_appfile.c must not use {bad}");
    }
    assert!(c.contains("#ifdef __EMSCRIPTEN__") && c.contains("EMSCRIPTEN_KEEPALIVE int fle_app_open(const char *path_utf8)"));
}
