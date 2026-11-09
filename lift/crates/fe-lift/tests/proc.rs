//! Tests for the process and thread answers in runtime/fle_proc.c (docs/decisions/0020-process-and-time.md; sources
//! and what they do not say: docs/investigations/import-proctime-spec.md).
//!
//! Expected values are written here from the rules on the Microsoft pages (GetCurrentProcess, SetLastError,
//! SetErrorMode, SetThreadErrorMode, SetPriorityClass, SetThreadPriority, SwitchToThread, OutputDebugStringW,
//! ExitProcess, TerminateProcess, RaiseException, System Error Codes), not read back from the C code. A value the
//! pages do not give is marked "assumption" or "inference" and is the only thing such a test pins down.
//! The host is Linux: nothing is compared with a real Windows. No host process id, clock or environment is read.
//!
//! The C sources come from runtime/ when fle_proc.c is there, otherwise from out-proctime/work/ (development).

mod etxt_common;

use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn runtime_dir() -> PathBuf {
    manifest().join("../../runtime")
}
fn src_dir() -> PathBuf {
    if runtime_dir().join("fle_proc.c").exists() {
        runtime_dir()
    } else {
        manifest().join("../../out-proctime/work")
    }
}
fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-proctime/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

fn clang_c(src: &Path, obj: &Path, strict: bool) {
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-Wall", "-Wextra", "-fPIE", "-c"]);
    if strict {
        cmd.arg("-Werror");
    }
    let cc = cmd
        .arg("-I")
        .arg(src_dir())
        .arg("-I")
        .arg(runtime_dir())
        .arg(src)
        .arg("-o")
        .arg(obj)
        .env("TMPDIR", work_dir().join("tmp"))
        .output()
        .expect("clang not found");
    assert!(cc.status.success(), "clang failed for {}: {}", src.display(), String::from_utf8_lossy(&cc.stderr));
}

/// fle_proc.c and fle_time.c (from src_dir, warnings are errors) plus the files they link with.
fn objs() -> &'static Vec<PathBuf> {
    static O: OnceLock<Vec<PathBuf>> = OnceLock::new();
    O.get_or_init(|| {
        let dir = work_dir().join(etxt_common::per_process("obj-proc"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut v = Vec::new();
        for (name, ours) in [("fle_proc.c", true), ("fle_time.c", true), ("fle_sys.c", false), ("fle_imports.c", false), ("fle_env.c", false)] {
            let base = if ours { src_dir() } else { runtime_dir() };
            let o = dir.join(name.replace(".c", ".o"));
            clang_c(&base.join(name), &o, ours);
            v.push(o);
        }
        v
    })
}

pub struct Out {
    stdout: String,
    stderr: String,
    signal: Option<i32>,
    ok: bool,
}

const PRELUDE: &str = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <pthread.h>
#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_sys.h"
#include "fle_proc.h"

uint64_t R[16];
static uint64_t stk[32];

/* a[0..3] go to rcx, rdx, r8, r9; a[4..11] are arguments 5..12 at [rsp + 0x28 + 8 * k]. */
static uint64_t callv(void (*f)(void), const uint64_t *a)
{
    int i;
    memset(stk, 0, sizeof stk);
    stk[0] = 0xdead0000u;
    R[1] = a[0]; R[2] = a[1]; R[8] = a[2]; R[9] = a[3];
    for (i = 4; i < 12; i++) stk[i + 1] = a[i];
    R[4] = (uint64_t)(uintptr_t)stk;
    R[0] = 0xBADBADBADull;
    f();
    return R[0];
}
#define CALL(f, ...) callv((void (*)(void))(f), (uint64_t[12]){ __VA_ARGS__ })
#define A(x) ((uint64_t)(uintptr_t)(x))
#define U(x) ((unsigned long long)(x))
static uint32_t LE(void) { fle_imp_GetLastError(); return (uint32_t)R[0]; }
"#;

fn run_src(name: &str, top: &str, body: &str) -> Out {
    let dir = work_dir();
    let uname = etxt_common::uniq(name);
    let c = dir.join(format!("{uname}.c"));
    let exe = dir.join(&uname);
    let src = format!("{PRELUDE}\n{top}\nint main(void)\n{{\n{body}\n    return 0;\n}}\n");
    std::fs::write(&c, src).unwrap();
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fPIE", "-pthread", "-I"]).arg(src_dir()).arg("-I").arg(runtime_dir());
    for o in objs() {
        cmd.arg(o);
    }
    let cc = cmd.arg("-o").arg(&exe).arg(&c).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new(&exe).env("TMPDIR", dir.join("tmp")).output().expect("run failed");
    Out {
        stdout: String::from_utf8_lossy(&r.stdout).to_string(),
        stderr: String::from_utf8_lossy(&r.stderr).to_string(),
        signal: r.status.signal(),
        ok: r.status.success(),
    }
}

fn run(name: &str, body: &str) -> Out {
    run_src(name, "", body)
}

fn lines(name: &str, body: &str) -> Vec<String> {
    let o = run(name, body);
    assert!(o.ok, "{name} did not exit normally (signal {:?}): {}\n{}", o.signal, o.stderr, o.stdout);
    o.stdout.lines().map(|s| s.to_string()).collect()
}

/// Each (label, C statements) must stop the run with SIGABRT, a "fle:" message and no output after it.
fn assert_aborts(prefix: &str, setup: &str, cases: &[(&str, &str)]) {
    for (label, stmts) in cases {
        let body = format!("{setup}\n    {stmts}\n    printf(\"returned %llu\\n\", U(R[0]));");
        let o = run(&format!("{prefix}-{label}"), &body);
        assert!(!o.ok, "{label}: expected an abort, got stdout {}", o.stdout);
        assert!(!o.stdout.contains("returned"), "{label}: returned instead of stopping");
        assert!(o.stderr.contains("fle:"), "{label}: no fle: message: {}", o.stderr);
        assert_eq!(o.signal, Some(6), "{label}: not SIGABRT");
    }
}

// ---- 1. Table, header, generator ----

const A1: [&str; 13] = [
    "GetCurrentProcess",
    "GetCurrentProcessId",
    "GetCurrentThread",
    "SetLastError",
    "SetErrorMode",
    "SetThreadErrorMode",
    "SetPriorityClass",
    "SetThreadPriority",
    "SwitchToThread",
    "OutputDebugStringW",
    "ExitProcess",
    "TerminateProcess",
    "RaiseException",
];

#[test]
fn rows_are_in_the_table_and_declared_in_the_header() {
    let table = std::fs::read_to_string(manifest().join("src/imports_table.rs")).unwrap();
    let h = std::fs::read_to_string(src_dir().join("fle_proc.h")).unwrap();
    for n in A1 {
        let row = format!("(\"kernel32.dll\", \"{n}\", \"fle_imp_{n}\", \"jmp_imp:{n}\")");
        assert_eq!(table.matches(&row).count(), 1, "{n}: exactly one row in imports_table.rs");
        assert!(h.contains(&format!("void fle_imp_{n}(void)")), "{n} is declared in fle_proc.h");
    }
}

/// With the real DLL: every API of the unit appears as a handled thunk in the 896-seed closure, and in
/// fle_resp_table (skipped when the DLL or the seed list is absent).
#[test]
fn real_dll_handles_the_rows() {
    let p = manifest().join("../../../binaries/win64/FLEngine_x64.dll");
    let Ok(data) = std::fs::read(p) else { return };
    let Ok(seeds_txt) = std::fs::read_to_string(manifest().join("../../out-vfs/seeds-896.txt")) else { return };
    let seeds: Vec<u64> =
        seeds_txt.lines().filter(|l| !l.trim().is_empty()).map(|l| u64::from_str_radix(l.trim().trim_start_matches("0x"), 16).unwrap()).collect();
    assert_eq!(seeds.len(), 896);
    let mut out = Vec::new();
    let st = fe_lift::generate_with_seeds(&data, &seeds, &mut out).unwrap();
    let c = String::from_utf8(out).unwrap();
    for n in A1 {
        assert!(st.handled.get(format!("jmp_imp:{n}").as_str()).copied().unwrap_or(0) >= 1, "{n} is a handled thunk");
        assert!(c.contains(&format!("{{ \"kernel32.dll\", \"{n}\", ")), "{n} in fle_resp_table");
        assert!(c.contains(&format!("fle_imp_{n}(); return;")), "{n}: the thunk calls the runtime");
    }
}

// ---- 2. Handles and ids ----

#[test]
fn pseudo_handles_and_process_id() {
    let l = lines(
        "proc-handles",
        r#"
    uint64_t p1 = CALL(fle_imp_GetCurrentProcess, 0), p2 = CALL(fle_imp_GetCurrentProcess, 0);
    uint64_t t1 = CALL(fle_imp_GetCurrentThread, 0), t2 = CALL(fle_imp_GetCurrentThread, 0);
    uint64_t i1 = CALL(fle_imp_GetCurrentProcessId, 0), i2 = CALL(fle_imp_GetCurrentProcessId, 0);
    printf("proc %llx %d\n", U(p1), p1 == p2);
    printf("thread_same %d distinct_from_process %d\n", t1 == t2, t1 != p1);
    printf("thread_value %llx\n", U(t1));
    printf("pid_same %d nonzero %d fits32 %d\n", i1 == i2, i1 != 0, i1 <= 0xFFFFFFFFull);
"#,
    );
    assert_eq!(
        l,
        [
            // MSDN GetCurrentProcess: "currently (HANDLE)-1"
            "proc ffffffffffffffff 1",
            "thread_same 1 distinct_from_process 1",
            // assumption: the GetCurrentThread page gives no value; -2 is the commonly known one, not confirmed
            "thread_value fffffffffffffffe",
            // GetCurrentProcessId returns a DWORD that identifies the process "until the process terminates"
            "pid_same 1 nonzero 1 fits32 1",
        ]
    );
}

// ---- 3. SetLastError ----

#[test]
fn set_last_error_round_trips_any_32_bit_value() {
    let values: [u32; 7] = [0, 5, 87, 0x20000000 /* bit 29: application-defined */, 0x80000000, 0xC06D0057, 0xFFFFFFFF];
    let mut body = String::new();
    for v in values {
        body.push_str(&format!("    CALL(fle_imp_SetLastError, {v}u); printf(\"%u\\n\", LE());\n"));
    }
    // only the low 32 bits of rcx are the DWORD
    body.push_str("    CALL(fle_imp_SetLastError, 0x1234567800000057ull); printf(\"%u\\n\", LE());\n");
    // SetLastError is VOID: it must not touch the return register
    body.push_str("    R[0] = 0x1111; R[1] = 9; fle_imp_SetLastError(); printf(\"%llx\\n\", U(R[0]));\n");
    let l = lines("proc-lasterror", &body);
    let mut want: Vec<String> = values.iter().map(|v| v.to_string()).collect();
    want.push("87".into());
    want.push("1111".into());
    assert_eq!(l, want);
}

// ---- 4. Error mode ----

const SEM_ALIGN: u32 = 0x4;

/// The rule of the SetErrorMode page: the return value is the previous mode; once SEM_NOALIGNMENTFAULTEXCEPT is
/// set, an attempt to clear it is ignored.
fn error_mode_model(seq: &[u32]) -> Vec<u32> {
    let mut cur = 0u32; // 0: "the system default" (the mode before any call)
    let mut out = Vec::new();
    for &m in seq {
        out.push(cur);
        let mut n = m;
        if cur & SEM_ALIGN != 0 {
            n |= SEM_ALIGN;
        }
        cur = n;
    }
    out
}

#[test]
fn set_error_mode_returns_the_previous_mode_and_keeps_the_alignment_bit() {
    let seq: [u32; 12] = [0, 1, 2, 0x8000, 0x8003, 0, 1, 4, 0, 1, 0x8002, 0];
    let mut body = String::new();
    for m in seq {
        body.push_str(&format!("    printf(\"%llx\\n\", U(CALL(fle_imp_SetErrorMode, {m}u)));\n"));
    }
    let l = lines("proc-errmode", &body);
    let want: Vec<String> = error_mode_model(&seq).iter().map(|v| format!("{v:x}")).collect();
    assert_eq!(l, want);
    // a hand-checked tail of the model, so the model itself is pinned: ... 4 (set), 0 (clear ignored -> 4), 1 -> 4|1
    assert_eq!(&want[7..10], ["1", "4", "4"]);
}

#[test]
fn set_thread_error_mode() {
    // Page: BOOL; lpOldMode (may be NULL) receives the thread's previous mode; values 0, 1, 2, 0x8000.
    // "A thread inherits the error mode of the process": the first previous mode is the process mode (reading).
    let l = lines(
        "proc-thrmode",
        r#"
    uint32_t old = 0xAAAAAAAAu;
    uint64_t r;
    CALL(fle_imp_SetErrorMode, 1);
    r = CALL(fle_imp_SetThreadErrorMode, 2, A(&old)); printf("a %llu %u\n", U(r), old);
    r = CALL(fle_imp_SetThreadErrorMode, 0x8001, A(&old)); printf("b %llu %u\n", U(r), old);
    r = CALL(fle_imp_SetThreadErrorMode, 0, 0); printf("c %llu\n", U(r));
    r = CALL(fle_imp_SetThreadErrorMode, 1, A(&old)); printf("d %llu %u\n", U(r), old);
    /* a later process-wide change does not rewrite the thread's own mode */
    CALL(fle_imp_SetErrorMode, 2);
    r = CALL(fle_imp_SetThreadErrorMode, 0, A(&old)); printf("e %llu %u\n", U(r), old);
    /* the page does not say the 64-bit value of rcx matters: only the DWORD */
    r = CALL(fle_imp_SetThreadErrorMode, 0xFFFFFFFF00000002ull, A(&old)); printf("f %llu %u\n", U(r), old);
"#,
    );
    assert_eq!(l, ["a 1 1", "b 1 2", "c 1", "d 1 0", "e 1 1", "f 1 0"]);
}

#[test]
fn error_mode_values_outside_the_page_stop_the_run() {
    assert_aborts(
        "proc-abort-errmode",
        "",
        &[
            ("seterrormode-bit4-extra", "CALL(fle_imp_SetErrorMode, 0x10);"),
            ("seterrormode-bit16", "CALL(fle_imp_SetErrorMode, 0x10000);"),
            // decision 0126 (wall 67, S-threaderrormode-alignment): old: ("setthread-alignment-bit", SetThreadErrorMode(4, 0)) stopped; SEM_NOALIGNMENTFAULTEXCEPT 4 is accepted now (tests/wall_fr11.rs)
            ("setthread-bit4-extra", "CALL(fle_imp_SetThreadErrorMode, 0x20, 0);"),
        ],
    );
}

// ---- 5. Priorities ----

#[test]
fn set_priority_class() {
    // Classes on the page: ABOVE_NORMAL 0x8000, BELOW_NORMAL 0x4000, HIGH 0x80, IDLE 0x40, NORMAL 0x20,
    // REALTIME 0x100; background begin 0x100000 / end 0x200000 (current process only). ERROR_PROCESS_MODE_ALREADY_BACKGROUND
    // 402 and ERROR_PROCESS_MODE_NOT_BACKGROUND 403 are in System Error Codes.
    let classes = [0x8000u32, 0x4000, 0x80, 0x40, 0x20, 0x100];
    let mut body = String::from("    uint64_t r;\n");
    for c in classes {
        body.push_str(&format!("    r = CALL(fle_imp_SetPriorityClass, 0xFFFFFFFFFFFFFFFFull, {c}); printf(\"{c:#x} %llu\\n\", U(r));\n"));
    }
    body.push_str(
        r#"
    r = CALL(fle_imp_SetPriorityClass, 0xFFFFFFFFFFFFFFFFull, 0x200000); printf("end-first %llu %u\n", U(r), LE());
    r = CALL(fle_imp_SetPriorityClass, 0xFFFFFFFFFFFFFFFFull, 0x100000); printf("begin %llu\n", U(r));
    r = CALL(fle_imp_SetPriorityClass, 0xFFFFFFFFFFFFFFFFull, 0x100000); printf("begin-again %llu %u\n", U(r), LE());
    r = CALL(fle_imp_SetPriorityClass, 0xFFFFFFFFFFFFFFFFull, 0x200000); printf("end %llu\n", U(r));
    r = CALL(fle_imp_SetPriorityClass, 0xFFFFFFFFFFFFFFFFull, 0x200000); printf("end-again %llu %u\n", U(r), LE());
"#,
    );
    let l = lines("proc-prioclass", &body);
    let mut want: Vec<String> = classes.iter().map(|c| format!("{c:#x} 1")).collect();
    want.extend(["end-first 0 403", "begin 1", "begin-again 0 402", "end 1", "end-again 0 403"].map(String::from));
    assert_eq!(l, want);
}

#[test]
fn set_thread_priority() {
    // Values on the page: -15 IDLE, -2 LOWEST, -1 BELOW_NORMAL, 0 NORMAL, 1 ABOVE_NORMAL, 2 HIGHEST, 15
    // TIME_CRITICAL; background begin 0x10000 / end 0x20000 (current thread only); -7..-3 and 3..6 only for a
    // REALTIME_PRIORITY_CLASS process. ERROR_THREAD_MODE_ALREADY_BACKGROUND 400, ERROR_THREAD_MODE_NOT_BACKGROUND 401.
    let l = lines(
        "proc-threadprio",
        r#"
    const uint64_t th = 0xFFFFFFFFFFFFFFFEull; /* assumption: the GetCurrentThread value, see fle_proc.h */
    uint64_t r;
    uint64_t h = CALL(fle_imp_GetCurrentThread, 0);
    int vals[] = { -15, -2, -1, 0, 1, 2, 15 };
    int i;
    for (i = 0; i < 7; i++) {
        r = CALL(fle_imp_SetThreadPriority, h, (uint32_t)vals[i]);
        printf("%d %llu\n", vals[i], U(r));
    }
    r = CALL(fle_imp_SetThreadPriority, h, 0x20000); printf("end-first %llu %u\n", U(r), LE());
    r = CALL(fle_imp_SetThreadPriority, h, 0x10000); printf("begin %llu\n", U(r));
    r = CALL(fle_imp_SetThreadPriority, h, 0x10000); printf("begin-again %llu %u\n", U(r), LE());
    r = CALL(fle_imp_SetThreadPriority, h, 0x20000); printf("end %llu\n", U(r));
    r = CALL(fle_imp_SetThreadPriority, h, 0x20000); printf("end-again %llu %u\n", U(r), LE());
    /* the fine steps need a REALTIME class: set it through the process API, then they are accepted */
    CALL(fle_imp_SetPriorityClass, CALL(fle_imp_GetCurrentProcess, 0), 0x100);
    { int rt[] = { -7, -6, -5, -4, -3, 3, 4, 5, 6 };
      for (i = 0; i < 9; i++) { r = CALL(fle_imp_SetThreadPriority, h, (uint32_t)rt[i]); printf("rt%d %llu\n", rt[i], U(r)); } }
    (void)th;
"#,
    );
    let mut want: Vec<String> = [-15i32, -2, -1, 0, 1, 2, 15].iter().map(|v| format!("{v} 1")).collect();
    want.extend(["end-first 0 401", "begin 1", "begin-again 0 400", "end 1", "end-again 0 401"].map(String::from));
    want.extend([-7i32, -6, -5, -4, -3, 3, 4, 5, 6].iter().map(|v| format!("rt{v} 1")));
    assert_eq!(l, want);
}

#[test]
fn priority_requests_outside_the_page_stop_the_run() {
    assert_aborts(
        "proc-abort-prio",
        "",
        &[
            ("class-foreign-handle", "CALL(fle_imp_SetPriorityClass, 0x1234, 0x20);"),
            ("class-null-handle", "CALL(fle_imp_SetPriorityClass, 0, 0x20);"),
            ("class-unlisted-value", "CALL(fle_imp_SetPriorityClass, 0xFFFFFFFFFFFFFFFFull, 0x1);"),
            ("class-two-flags", "CALL(fle_imp_SetPriorityClass, 0xFFFFFFFFFFFFFFFFull, 0x60);"),
            ("thread-foreign-handle", "CALL(fle_imp_SetThreadPriority, 0x1234, 0);"),
            ("thread-process-handle", "CALL(fle_imp_SetThreadPriority, CALL(fle_imp_GetCurrentProcess, 0), 0);"),
            ("thread-unlisted-value", "CALL(fle_imp_SetThreadPriority, CALL(fle_imp_GetCurrentThread, 0), 7);"),
            ("thread-realtime-step-in-normal-class", "CALL(fle_imp_SetThreadPriority, CALL(fle_imp_GetCurrentThread, 0), 0xFFFFFFFDu);"),
            ("thread-unlisted-negative", "CALL(fle_imp_SetThreadPriority, CALL(fle_imp_GetCurrentThread, 0), 0xFFFFFFF0u);"),
        ],
    );
}

// ---- 6. SwitchToThread, OutputDebugStringW ----

#[test]
fn switch_to_thread_reports_that_no_other_thread_is_ready() {
    // Page: zero when "there are no other threads ready to execute". There is one thread.
    let l = lines(
        "proc-switch",
        r#"
    uint64_t nz = 0; int i;
    for (i = 0; i < 1000000; i++) nz += CALL(fle_imp_SwitchToThread, 0);
    printf("nonzero-results %llu\n", U(nz));
"#,
    );
    assert_eq!(l, ["nonzero-results 0"]);
}

#[test]
fn switch_to_thread_spin_limit_stops_the_run() {
    // The limit (1,000,000 calls) is a runtime choice, not from a page: a loop that waits for a thread that does
    // not exist must end in a stop, not spin forever.
    let o = run("proc-switch-limit", "    int i; for (i = 0; i < 1000001; i++) CALL(fle_imp_SwitchToThread, 0);\n    printf(\"returned\\n\");");
    assert!(!o.ok && !o.stdout.contains("returned"));
    assert_eq!(o.signal, Some(6));
    assert!(o.stderr.contains("fle: SwitchToThread"), "{}", o.stderr);
}

#[test]
fn output_debug_string_does_nothing_and_does_not_read_the_string() {
    // Page: with no debugger and no active system debugger it "does nothing"; VOID. The argument may be NULL
    // (optional). The pointer is not dereferenced here, so even an invalid one must not fault.
    let l = lines(
        "proc-debugstring",
        r#"
    static const uint16_t s[] = { 'h', 'i', 0 };
    uint64_t r;
    r = CALL(fle_imp_OutputDebugStringW, A(s)); printf("%llx\n", U(r));
    r = CALL(fle_imp_OutputDebugStringW, 0); printf("%llx\n", U(r));
    r = CALL(fle_imp_OutputDebugStringW, 1); printf("%llx\n", U(r));
    printf("le %u\n", LE());
"#,
    );
    // VOID: R[0] still holds what the harness put there before the call
    assert_eq!(l, ["badbadbad", "badbadbad", "badbadbad", "le 0"]);
}

// ---- 7. Requests to end the process stop the run (decision 0020) ----

#[test]
fn exit_process_stops_the_run_and_names_the_exit_code() {
    for (code, text) in [(0u64, "exit code 0"), (1, "exit code 1"), (0xFFFF_FFFF, "exit code 4294967295"), (0x1_0000_0003, "exit code 3")] {
        let o = run(&format!("proc-exit-{code:x}"), &format!("    CALL(fle_imp_ExitProcess, {code}ull);\n    printf(\"returned\\n\");"));
        assert!(!o.ok, "ExitProcess({code}) must not end the host process normally");
        assert!(!o.stdout.contains("returned"), "ExitProcess returned");
        assert_eq!(o.signal, Some(6), "SIGABRT, not a silent exit");
        assert!(o.stderr.starts_with("fle: ExitProcess: "), "{}", o.stderr);
        assert!(o.stderr.contains(text), "{} should contain {text}", o.stderr);
    }
}

#[test]
fn terminate_process_stops_the_run() {
    let o = run("proc-terminate-self", "    CALL(fle_imp_TerminateProcess, 0xFFFFFFFFFFFFFFFFull, 7);\n    printf(\"returned\\n\");");
    assert!(!o.ok && !o.stdout.contains("returned"));
    assert_eq!(o.signal, Some(6));
    assert!(o.stderr.starts_with("fle: TerminateProcess: "), "{}", o.stderr);
    assert!(o.stderr.contains("exit code 7"), "{}", o.stderr);
    // a handle that GetCurrentProcess did not return: no other process exists here
    let o = run("proc-terminate-other", "    CALL(fle_imp_TerminateProcess, 0x1234, 7);\n    printf(\"returned\\n\");");
    assert!(!o.ok && !o.stdout.contains("returned"));
    assert_eq!(o.signal, Some(6));
    assert!(o.stderr.contains("0x1234") && o.stderr.contains("not the current process"), "{}", o.stderr);
}

#[test]
fn raise_exception_stops_the_run_and_names_the_exception() {
    // 0xC06D0057 is the code the guest raises at 0x41da18 (mov ecx,0xc06d0057 before the call to the thunk)
    let o = run(
        "proc-raise",
        "    static uint64_t args[2] = { 1, 2 };\n    CALL(fle_imp_RaiseException, 0xC06D0057u, 1, 2, A(args));\n    printf(\"returned\\n\");",
    );
    assert!(!o.ok && !o.stdout.contains("returned"));
    assert_eq!(o.signal, Some(6));
    assert!(o.stderr.starts_with("fle: RaiseException: "), "{}", o.stderr);
    assert!(o.stderr.contains("0xc06d0057"), "{}", o.stderr);
    assert!(o.stderr.contains("flags 0x1") && o.stderr.contains("2 argument"), "{}", o.stderr);
}

// ---- 8. Single thread, no host state ----

#[test]
fn a_second_thread_stops_the_run() {
    let o = run_src(
        "proc-thread",
        "static void *th(void *x) { (void)x; CALL(fle_imp_GetCurrentProcess, 0); printf(\"returned-from-thread\\n\"); return 0; }",
        "    pthread_t t;\n    CALL(fle_imp_GetCurrentProcess, 0);\n    pthread_create(&t, 0, th, 0);\n    pthread_join(t, 0);\n    printf(\"returned\\n\");",
    );
    assert!(!o.ok && !o.stdout.contains("returned"));
    assert_eq!(o.signal, Some(6));
    assert!(o.stderr.contains("fle: GetCurrentProcess") && o.stderr.contains("second thread"), "{}", o.stderr);
}

/// C text without comments and string literals' contents are kept; only /* */ and // are dropped.
fn strip_comments(s: &str) -> String {
    let b: Vec<char> = s.chars().collect();
    let mut o = String::new();
    let mut i = 0;
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
fn source_uses_no_host_process_time_environment_or_file_state() {
    let code = strip_comments(&std::fs::read_to_string(src_dir().join("fle_proc.c")).unwrap());
    // (SYS_gettid is allowed: it only names the owner thread for the single-thread rule and is never returned)
    for bad in ["getpid", "getppid", "clock_gettime", "gettimeofday", "getenv", "environ", "fopen", "open(", "<time.h>", "sched_yield", "nanosleep", "usleep", "exit(", "_exit"] {
        assert!(!code.contains(bad), "fle_proc.c must not use {bad}");
    }
}
