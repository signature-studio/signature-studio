//! Tests for CreateEventW, SetEvent, ResetEvent, WaitForSingleObject and the CloseHandle hook of runtime/fle_event.c
//! (docs/decisions/0050-event-sync.md; sources: docs/investigations/event-sync-run.md).
//!
//! Expected values are written here from the rules on the Microsoft pages (CreateEventW, SetEvent, ResetEvent,
//! WaitForSingleObject, WaitForSingleObjectEx, CloseHandle, System Error Codes), not read back from the C code. A value the
//! pages do not give is marked "assumption", "inference" or "choice" and is the only thing such a test pins down. The host is
//! Linux: nothing is compared with a real Windows; no host clock, thread or process is read.
//!
//! The C sources of fle_event.c / fle_event.h come from runtime/ when fle_event.c is there, otherwise from out-event/work/
//! (development). FLE_EVENT_SRC_DIR (a directory with fle_event.c and fle_event.h) overrides both: the mutation runs of
//! the run record use it. Files that already existed always come from runtime/.

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
    if let Ok(d) = std::env::var("FLE_EVENT_SRC_DIR") {
        return PathBuf::from(d);
    }
    if runtime_dir().join("fle_event.c").exists() {
        runtime_dir()
    } else {
        manifest().join("../../out-event/work")
    }
}
fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-event/test");
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

/// fle_event.c (warnings are errors) and the existing files it links with (the CloseHandle of fle_vfs.c; fle_vol.c and
/// fle_vmap.c are the two other users of the close hooks).
fn objs() -> &'static Vec<PathBuf> {
    static O: OnceLock<Vec<PathBuf>> = OnceLock::new();
    O.get_or_init(|| {
        let dir = work_dir().join(etxt_common::per_process("obj-event"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut v = Vec::new();
        for (name, ours) in [
            ("fle_event.c", true),
            ("fle_vol.c", false),
            ("fle_vmap.c", false),
            ("fle_vfs.c", false),
            ("fle_sys.c", false),
            ("fle_proc.c", false),
            ("fle_imports.c", false),
            ("fle_env.c", false),
        ] {
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
#include "fle_vfs.h"
#include "fle_proc.h"
#include "fle_vol.h"
#include "fle_vmap.h"
#include "fle_event.h"

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
static void SETLE(uint32_t v) { fle_set_last_error(v); }
#define INVH 0xFFFFFFFFFFFFFFFFull
#define CREATE(manual, initial) CALL(fle_imp_CreateEventW, 0, (manual), (initial), 0)
#define SET(h) CALL(fle_imp_SetEvent, (h))
#define RESET(h) CALL(fle_imp_ResetEvent, (h))
#define WAIT(h, ms) ((uint32_t)CALL(fle_imp_WaitForSingleObject, (h), (ms)))
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

/// Each (label, C statements) must stop the run with SIGABRT, a "fle: <api>" message and no output after it. `setup` runs first.
fn assert_aborts(prefix: &str, setup: &str, api: &str, cases: &[(&str, &str)]) {
    for (label, stmts) in cases {
        let body = format!("{setup}\n    {stmts}\n    printf(\"returned %llu\\n\", U(R[0]));");
        let o = run(&format!("{prefix}-{label}"), &body);
        assert!(!o.ok, "{label}: expected an abort, got stdout {}", o.stdout);
        assert!(!o.stdout.contains("returned"), "{label}: returned instead of stopping");
        assert!(o.stderr.contains(&format!("fle: {api}")), "{label}: no 'fle: {api}' message: {}", o.stderr);
        assert_eq!(o.signal, Some(6), "{label}: not SIGABRT");
    }
}

fn header_const(file: &str, name: &str) -> u64 {
    let t = std::fs::read_to_string(src_dir().join(file)).or_else(|_| std::fs::read_to_string(runtime_dir().join(file))).unwrap();
    let line = t.lines().find(|l| l.contains(&format!("#define {name} "))).unwrap_or_else(|| panic!("{name} not in {file}"));
    let tok = line.split("UINT64_C(").nth(1).unwrap().split(')').next().unwrap();
    u64::from_str_radix(tok.trim_start_matches("0x"), 16).unwrap()
}

/// C text without comments (strings are kept).
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

// ---- 1. Table, header, handle region ----

const API: [&str; 4] = ["CreateEventW", "SetEvent", "ResetEvent", "WaitForSingleObject"];

#[test]
fn rows_are_in_the_table_and_declared_in_the_header() {
    let table = std::fs::read_to_string(manifest().join("src/imports_table.rs")).unwrap();
    let h = std::fs::read_to_string(src_dir().join("fle_event.h")).unwrap();
    for n in API {
        let row = format!("(\"kernel32.dll\", \"{n}\", \"fle_imp_{n}\", \"jmp_imp:{n}\")");
        assert_eq!(table.matches(&row).count(), 1, "{n}: exactly one row in imports_table.rs");
        assert!(h.contains(&format!("void fle_imp_{n}(void)")), "{n} is declared in fle_event.h");
    }
    // CloseHandle already has its one row (it asks the close hooks, one of which is this file's)
    let close = "(\"kernel32.dll\", \"CloseHandle\", \"fle_imp_CloseHandle\", \"jmp_imp:CloseHandle\")";
    assert_eq!(table.matches(close).count(), 1);
    // WaitForSingleObjectEx is not imported by the DLL (out-sync/counts.txt: 0 thunks, 0 sites) and is not answered
    assert!(!table.contains("WaitForSingleObjectEx"));
}

#[test]
fn the_event_region_is_the_one_decided_and_overlaps_nothing() {
    let base = header_const("fle_event.h", "FLE_EVENT_HANDLE_BASE");
    let size = header_const("fle_event.h", "FLE_EVENT_REGION_SIZE");
    assert_eq!(base, 0x7FFA_8000_0000, "decision 0050 (PM answer 3 of the survey)");
    assert_eq!(size, 0x1000_0000, "the size of every other handle region");
    let fsz = header_const("fle_vfs.h", "FLE_HANDLE_REGION_SIZE");
    let mut others: Vec<(String, u64, u64)> = vec![
        ("vol".into(), header_const("fle_vol.h", "FLE_VOL_HANDLE_BASE"), header_const("fle_vol.h", "FLE_VOL_REGION_SIZE")),
        ("vmap".into(), header_const("fle_vmap.h", "FLE_VMAP_HANDLE_BASE"), header_const("fle_vmap.h", "FLE_VMAP_REGION_SIZE")),
        ("glob".into(), header_const("fle_glob.h", "FLE_GLOBAL_HANDLE_BASE"), header_const("fle_glob.h", "FLE_GLOBAL_REGION_SIZE")),
    ];
    for n in ["FLE_VFS_FILE_BASE", "FLE_VFS_FIND_BASE", "FLE_REG_KEY_BASE"] {
        others.push((n.into(), header_const("fle_vfs.h", n), fsz));
    }
    // 0x7FFA30000000 and 0x7FFA40000000 were left free by decision 0024 for other work: they are not used here either
    others.push(("reserved-30".into(), 0x7FFA_3000_0000, 0x1000_0000));
    others.push(("reserved-40".into(), 0x7FFA_4000_0000, 0x1000_0000));
    // the regions named in the task: 0x7FFA00000000 .. 0x7FFA70000000
    assert_eq!(others.iter().filter(|o| o.1 >= 0x7FFA_0000_0000 && o.1 <= 0x7FFA_7000_0000).count(), others.len());
    for (n, b, s) in &others {
        assert!(base >= b + s || *b >= base + size, "event region overlaps {n}");
    }
    // above the heap token, the module handles and the response pseudo-VAs, inside the user address space
    let heap = 0x7FF7_0000_0000u64;
    let mods_end = 0x7FF8_0000_0000u64 + 0x10000 * 34;
    let resp_end = 0x7FF9_0000_0000u64 + 0x10 * 5000;
    assert!(base > heap + 0x1000_0000 && base > mods_end && base > resp_end);
    assert!(base + size < (1u64 << 47));
}

// ---- 2. The four kinds ----

#[test]
fn the_four_kinds_follow_the_pages() {
    let l = lines(
        "event-four-kinds",
        r#"
    uint64_t h;
    /* manual-reset, initially signaled: stays signaled until ResetEvent (CreateEventW, SetEvent pages) */
    h = CREATE(1, 1);
    printf("mi created %d\n", h != 0);
    printf("mi w1 %x\n", WAIT(h, 0));
    printf("mi w2 %x\n", WAIT(h, 0));
    printf("mi w3 %x\n", WAIT(h, 5000));
    printf("mi reset %llu\n", U(RESET(h)));
    printf("mi w4 %x\n", WAIT(h, 0));
    printf("mi set %llu\n", U(SET(h)));
    printf("mi w5 %x\n", WAIT(h, 0));
    printf("mi w6 %x\n", WAIT(h, 0));
    /* manual-reset, initially nonsignaled */
    h = CREATE(1, 0);
    printf("m0 w1 %x\n", WAIT(h, 0));
    printf("m0 w2 %x\n", WAIT(h, 0));
    printf("m0 set %llu\n", U(SET(h)));
    printf("m0 w3 %x\n", WAIT(h, 0));
    printf("m0 w4 %x\n", WAIT(h, 0));
    printf("m0 set again %llu\n", U(SET(h)));
    printf("m0 w5 %x\n", WAIT(h, 0));
    printf("m0 reset %llu\n", U(RESET(h)));
    printf("m0 reset again %llu\n", U(RESET(h)));
    printf("m0 w6 %x\n", WAIT(h, 0));
    /* auto-reset, initially signaled: the first wait consumes the signal */
    h = CREATE(0, 1);
    printf("ai w1 %x\n", WAIT(h, 0));
    printf("ai w2 %x\n", WAIT(h, 0));
    printf("ai set %llu\n", U(SET(h)));
    printf("ai w3 %x\n", WAIT(h, 1000));
    printf("ai w4 %x\n", WAIT(h, 0));
    /* auto-reset, initially nonsignaled; with no thread waiting the state stays signaled (SetEvent page), and a set on a set event has no effect */
    h = CREATE(0, 0);
    printf("a0 w1 %x\n", WAIT(h, 0));
    printf("a0 set %llu\n", U(SET(h)));
    printf("a0 set again %llu\n", U(SET(h)));
    printf("a0 w2 %x\n", WAIT(h, 0));
    printf("a0 w3 %x\n", WAIT(h, 0));
    printf("a0 set %llu\n", U(SET(h)));
    printf("a0 reset %llu\n", U(RESET(h)));
    printf("a0 w4 %x\n", WAIT(h, 0));
"#,
    );
    assert_eq!(
        l,
        [
            "mi created 1",
            "mi w1 0",
            "mi w2 0",
            "mi w3 0",
            "mi reset 1",
            "mi w4 102",
            "mi set 1",
            "mi w5 0",
            "mi w6 0",
            "m0 w1 102",
            "m0 w2 102",
            "m0 set 1",
            "m0 w3 0",
            "m0 w4 0",
            "m0 set again 1",
            "m0 w5 0",
            "m0 reset 1",
            "m0 reset again 1",
            "m0 w6 102",
            "ai w1 0",
            "ai w2 102",
            "ai set 1",
            "ai w3 0",
            "ai w4 102",
            "a0 w1 102",
            "a0 set 1",
            "a0 set again 1",
            "a0 w2 0",
            "a0 w3 102",
            "a0 set 1",
            "a0 reset 1",
            "a0 w4 102",
        ]
    );
}

#[test]
fn a_bool_is_a_32_bit_value() {
    // the guest passes TRUE as edx = 0xFFFFFFFF (or edx = 1) and a BOOL is a DWORD: garbage in the high half of the register
    // does not count (inference from the type), and any nonzero low half is TRUE (the pages say "TRUE" / "FALSE" for the BOOL)
    let l = lines(
        "event-bool-width",
        r#"
    uint64_t h;
    /* bManualReset = 0xFFFFFFFF: manual */
    h = CREATE(0xFFFFFFFFull, 0);
    SET(h); WAIT(h, 0); printf("minus1 manual %x\n", WAIT(h, 0));
    /* bManualReset = 0x100000000: low half 0, so auto-reset */
    h = CREATE(0x100000000ull, 0);
    SET(h); WAIT(h, 0); printf("high-only auto %x\n", WAIT(h, 0));
    /* bInitialState = 0x100000000: low half 0, so not signaled; 0x80000000 is nonzero: signaled */
    h = CREATE(1, 0x100000000ull); printf("init high-only %x\n", WAIT(h, 0));
    h = CREATE(1, 0x80000000ull); printf("init 0x80000000 %x\n", WAIT(h, 0));
    /* the timeout of a wait is a DWORD too: 0x100000000 is 0 (returns at once), not INFINITE */
    h = CREATE(1, 0); printf("wait high-only %x\n", WAIT(h, 0x100000000ull));
"#,
    );
    assert_eq!(l, ["minus1 manual 0", "high-only auto 102", "init high-only 102", "init 0x80000000 0", "wait high-only 102"]);
}

// ---- 3. Handles, last error, clock ----

#[test]
fn handles_are_in_the_region_distinct_never_reused_and_leave_the_last_error_alone() {
    let base = header_const("fle_event.h", "FLE_EVENT_HANDLE_BASE");
    let l = lines(
        "event-handles",
        &format!(
            r#"
    uint64_t h1, h2, h3, ticks;
    SETLE(41);
    h1 = CREATE(1, 0);
    printf("h1 %llx le %u\n", U(h1 - {base}ull), LE());
    h2 = CREATE(0, 1);
    printf("h2 %llx le %u\n", U(h2 - {base}ull), LE());
    printf("count %llu\n", U(fle_event_handle_count()));
    printf("state %d %d\n", fle_event_state(h1), fle_event_state(h2));
    /* the other functions leave the last error alone on success */
    SETLE(77);
    SET(h1); RESET(h1); WAIT(h2, 0); WAIT(h1, 0);
    printf("le after ops %u\n", LE());
    /* a closed handle value is not issued again */
    CALL(fle_imp_CloseHandle, h1);
    h3 = CREATE(1, 1);
    printf("h3 %llx distinct %d\n", U(h3 - {base}ull), h3 != h1 && h3 != h2);
    printf("count %llu state %d\n", U(fle_event_handle_count()), fle_event_state(h1));
    /* none of these moves the virtual clock (decision 0050) */
    ticks = fle_sys_clock_ticks();
    CREATE(0, 0); SET(h2); RESET(h2); WAIT(h3, 0); WAIT(h3, 1000); WAIT(h2, 0); CALL(fle_imp_CloseHandle, h3);
    printf("clock moved %d\n", fle_sys_clock_ticks() != ticks);
"#,
            base = base
        ),
    );
    assert_eq!(l, ["h1 4 le 41", "h2 8 le 41", "count 2", "state 0 1", "le after ops 77", "h3 c distinct 1", "count 2 state -1", "clock moved 0"]);
}

// ---- 4. Wait results ----

#[test]
fn wait_results() {
    let l = lines(
        "event-wait",
        r#"
    uint64_t m = CREATE(1, 0), a = CREATE(0, 0);
    SETLE(0);
    /* not signaled, time-out 0: WAIT_TIMEOUT at once (page), the state is not touched */
    printf("m0 %x a0 %x\n", WAIT(m, 0), WAIT(a, 0));
    SET(m); SET(a);
    /* signaled: WAIT_OBJECT_0 whatever the time-out, INFINITE included (the page: returns when the object is signaled) */
    printf("m inf %x\n", WAIT(m, 0xFFFFFFFFu));
    printf("m 1 %x\n", WAIT(m, 1));
    printf("m 0 %x\n", WAIT(m, 0));
    printf("a inf %x\n", WAIT(a, 0xFFFFFFFFu));
    /* the auto-reset event was reset by that wait (CreateEventW page) */
    printf("a after %x state %d\n", WAIT(a, 0), fle_event_state(a));
    printf("m state %d\n", fle_event_state(m));
    printf("le %u\n", LE());
"#,
    );
    assert_eq!(l, ["m0 102 a0 102", "m inf 0", "m 1 0", "m 0 0", "a inf 0", "a after 102 state 0", "m state 1", "le 0"]);
}

#[test]
fn invalid_handles_fail_with_error_invalid_handle() {
    // SetEvent / ResetEvent: 0 and a last error; WaitForSingleObject: WAIT_FAILED (0xFFFFFFFF) and a last error. The pages
    // do not name the code for an invalid handle: 6 (ERROR_INVALID_HANDLE, System Error Codes) is an assumption, as in fle_vfs.c.
    let l = lines(
        "event-invalid",
        r#"
    uint64_t h = CREATE(1, 1), r;
    CALL(fle_imp_CloseHandle, h);
    SETLE(0); r = SET(0);   printf("set null %llu le %u\n", U(r), LE());
    SETLE(0); r = RESET(0); printf("reset null %llu le %u\n", U(r), LE());
    SETLE(0); printf("wait null %x le %u\n", WAIT(0, 0), LE());
    SETLE(0); r = SET(h);   printf("set closed %llu le %u\n", U(r), LE());
    SETLE(0); r = RESET(h); printf("reset closed %llu le %u\n", U(r), LE());
    SETLE(0); printf("wait closed %x le %u\n", WAIT(h, 0), LE());
    SETLE(0); printf("wait closed inf %x le %u\n", WAIT(h, 0xFFFFFFFFu), LE());
"#,
    );
    assert_eq!(
        l,
        ["set null 0 le 6", "reset null 0 le 6", "wait null ffffffff le 6", "set closed 0 le 6", "reset closed 0 le 6", "wait closed ffffffff le 6", "wait closed inf ffffffff le 6"]
    );
}

// ---- 5. CloseHandle ----

#[test]
fn close_handle_closes_events_and_reuse_after_close_fails() {
    let l = lines(
        "event-close",
        r#"
    uint64_t h = CREATE(0, 1), g = CREATE(1, 0), r;
    SETLE(9);
    r = CALL(fle_imp_CloseHandle, h); printf("close %llu le %u count %llu\n", U(r), LE(), U(fle_event_handle_count()));
    /* the other event is untouched */
    printf("g state %d set %llu wait %x\n", fle_event_state(g), U(SET(g)), WAIT(g, 0));
    /* a second close of the same handle fails with 0 and ERROR_INVALID_HANDLE (page: no code named; 6 is an inference) */
    SETLE(0); r = CALL(fle_imp_CloseHandle, h); printf("again %llu le %u count %llu\n", U(r), LE(), U(fle_event_handle_count()));
    SETLE(0); r = CALL(fle_imp_CloseHandle, h); printf("third %llu le %u\n", U(r), LE());
    /* every use of the closed handle fails */
    SETLE(0); r = SET(h); printf("set %llu le %u\n", U(r), LE());
    SETLE(0); printf("wait %x le %u\n", WAIT(h, 0), LE());
    /* the live one still works, and closing it makes the count 0 */
    printf("g wait %x\n", WAIT(g, 0));
    r = CALL(fle_imp_CloseHandle, g); printf("close g %llu count %llu\n", U(r), U(fle_event_handle_count()));
"#,
    );
    assert_eq!(
        l,
        [
            "close 1 le 9 count 1",
            "g state 0 set 1 wait 0",
            "again 0 le 6 count 1",
            "third 0 le 6",
            "set 0 le 6",
            "wait ffffffff le 6",
            "g wait 0",
            "close g 1 count 0",
        ]
    );
}

#[test]
fn the_close_hook_leaves_other_handles_and_the_other_hooks_alone() {
    // the event hook is the third of at most four registered hooks (vol, vmap, event): all three work together, and the
    // file handles of fle_vfs.c, NULL and a closed value still behave as before
    let l = lines(
        "event-hooks",
        r#"
    uint64_t pp = CALL(fle_imp_GetCurrentProcess, 0), d = 0, m, e, f, r;
    CALL(fle_imp_DuplicateHandle, pp, pp, pp, A(&d), 0, 0, 2);            /* fle_vol.c registers its hook */
    m = CALL(fle_imp_CreateFileMappingW, INVH, 0, 0x04, 0, 0x1000, 0);    /* fle_vmap.c registers its hook */
    e = CREATE(1, 0);                                                      /* this file registers its hook */
    f = CALL(fle_imp_CreateFileW, A(((const uint16_t[]){ 'C', ':', '\\', 'e', 'v', '.', 'b', 'i', 'n', 0 })), 0xC0000000u, 0, 0, 2, 0x80, 0);
    printf("issued %d %d %d %d\n", d != 0, m != 0, e != 0, f != INVH);
    r = CALL(fle_imp_CloseHandle, e); printf("event %llu\n", U(r));
    r = CALL(fle_imp_CloseHandle, d); printf("dup %llu\n", U(r));
    r = CALL(fle_imp_CloseHandle, m); printf("map %llu\n", U(r));
    r = CALL(fle_imp_CloseHandle, f); printf("file %llu\n", U(r));
    SETLE(0); r = CALL(fle_imp_CloseHandle, 0); printf("null %llu le %u\n", U(r), LE());
    SETLE(0); r = CALL(fle_imp_CloseHandle, f); printf("file again %llu le %u\n", U(r), LE());
    SETLE(0); r = CALL(fle_imp_CloseHandle, e); printf("event again %llu le %u\n", U(r), LE());
"#,
    );
    assert_eq!(
        l,
        ["issued 1 1 1 1", "event 1", "dup 1", "map 1", "file 1", "null 0 le 6", "file again 0 le 6", "event again 0 le 6"]
    );
}

#[test]
fn the_runtime_sources_use_at_most_four_close_hooks() {
    // fle_vfs.c has room for 8 hooks (a ninth stops the run with "too many handle hooks"; 4 until decision 0136): count the registrations in runtime/
    let mut n = 0;
    let mut files = vec![];
    for e in std::fs::read_dir(runtime_dir()).unwrap() {
        let p = e.unwrap().path();
        if p.extension().is_some_and(|x| x == "c") {
            files.push(p);
        }
    }
    let ev = src_dir().join("fle_event.c");
    if !files.iter().any(|p| p.file_name() == ev.file_name()) {
        files.push(ev);
    }
    for p in files {
        let code = strip_comments(&std::fs::read_to_string(&p).unwrap());
        n += code.matches("fle_vfs_add_close_hook(").count() - code.matches("void fle_vfs_add_close_hook(").count();
    }
    // decision 0122: fle_thread.c registers the fourth hook (the thread handles of CreateThread): the room of fle_vfs.c (4) was then full, a fifth stopped the run
    // (old: 3 = vol, vmap and event one each). Decision 0136: fle_token.c registers the fifth hook (the token handles of OpenProcessToken) and fle_vfs.c has room for 8.
    // decision 0183: fle_wall_wrp1.c registers the sixth hook (the job handles of CreateJobObjectW)
    assert_eq!(n, 6, "vol, vmap, event, thread, token and job register one hook each");
    assert!(n <= 8);
}

// ---- 6. Stops ----

#[test]
fn create_with_attributes_or_a_name_stops_the_run() {
    assert_aborts(
        "event-abort-create",
        "static uint16_t nm[] = { 'L', 'o', 'c', 'a', 'l', '\\\\', 'x', 0 };",
        "CreateEventW",
        &[
            ("attrs", "CALL(fle_imp_CreateEventW, 8, 1, 0, 0);"),
            ("name", "CALL(fle_imp_CreateEventW, 0, 1, 0, A(nm));"),
            ("attrs-and-name", "CALL(fle_imp_CreateEventW, 8, 0, 1, A(nm));"),
            ("empty-name", "CALL(fle_imp_CreateEventW, 0, 0, 0, A(nm + 7));"),
        ],
    );
}

#[test]
fn wait_that_would_block_stops_the_run() {
    // INFINITE on a nonsignaled event never returns: no other thread exists (Sleep(INFINITE) stops for the same reason).
    // A finite nonzero time-out on a nonsignaled event would be WAIT_TIMEOUT only after the interval: decision 0050 stops.
    assert_aborts(
        "event-abort-wait",
        "uint64_t m = CREATE(1, 0), a = CREATE(0, 0), c = CREATE(0, 1); WAIT(c, 0);",
        "WaitForSingleObject",
        &[
            ("inf-manual", "WAIT(m, 0xFFFFFFFFu);"),
            ("inf-auto", "WAIT(a, 0xFFFFFFFFu);"),
            ("inf-auto-consumed", "WAIT(c, 0xFFFFFFFFu);"),
            ("inf-after-reset", "SET(m); RESET(m); WAIT(m, 0xFFFFFFFFu);"),
            ("finite-1", "WAIT(m, 1);"),
            ("finite-100", "WAIT(a, 100);"),
            ("finite-max-1", "WAIT(m, 0xFFFFFFFEu);"),
            ("finite-high-garbage", "WAIT(m, 0x1000000064ull);"),
            ("inf-high-garbage", "WAIT(m, 0xAAAAAAAAFFFFFFFFull);"),
        ],
    );
}

#[test]
fn handles_that_are_not_events_stop_the_run() {
    let setup = "uint64_t e = CREATE(1, 0); uint64_t pp = CALL(fle_imp_GetCurrentProcess, 0), t = CALL(fle_imp_GetCurrentThread, 0);\n    uint64_t f = CALL(fle_imp_CreateFileW, A(((const uint16_t[]){ 'C', ':', '\\\\', 'x', '.', 'b', 0 })), 0xC0000000u, 0, 0, 2, 0x80, 0);";
    for api in ["SetEvent", "ResetEvent", "WaitForSingleObject"] {
        let call = |h: &str| match api {
            "SetEvent" => format!("CALL(fle_imp_SetEvent, {h});"),
            "ResetEvent" => format!("CALL(fle_imp_ResetEvent, {h});"),
            _ => format!("CALL(fle_imp_WaitForSingleObject, {h}, 0);"),
        };
        let cases: Vec<(String, String)> = [
            ("foreign", "0x1234"),
            ("invalid-handle-value", "INVH"),
            ("pseudo-process", "pp"),
            ("pseudo-thread", "t"),
            ("vfs-file", "f"),
            ("never-issued-in-region", "FLE_EVENT_HANDLE_BASE + 4 * 900"),
            ("region-base", "FLE_EVENT_HANDLE_BASE"),
            ("unaligned-in-region", "e + 1"),
            ("region-end", "FLE_EVENT_HANDLE_BASE + FLE_EVENT_REGION_SIZE"),
            ("vmap-region", "FLE_VMAP_HANDLE_BASE + 4"),
        ]
        .iter()
        .map(|(l, h)| (l.to_string(), call(h)))
        .collect();
        let refs: Vec<(&str, &str)> = cases.iter().map(|(l, c)| (l.as_str(), c.as_str())).collect();
        assert_aborts(&format!("event-abort-{}", api.to_lowercase()), setup, api, &refs);
    }
    // CloseHandle of a value in the event region that was never issued stops too, and it is this file's hook that stops it
    // (not fle_vfs.c's "not issued by the virtual file system": the hook claims the whole region)
    let setup = "CREATE(1, 0);";
    for (label, v) in [
        ("never-issued", "FLE_EVENT_HANDLE_BASE + 4 * 900"),
        ("unaligned", "FLE_EVENT_HANDLE_BASE + 5"),
        ("region-last", "FLE_EVENT_HANDLE_BASE + FLE_EVENT_REGION_SIZE - 4"),
    ] {
        let o = run(&format!("event-abort-close-{label}"), &format!("{setup}\n    CALL(fle_imp_CloseHandle, {v});\n    printf(\"returned\\n\");"));
        assert!(!o.ok && !o.stdout.contains("returned"), "{label}");
        assert_eq!(o.signal, Some(6), "{label}");
        assert!(o.stderr.contains("fle: CloseHandle") && o.stderr.contains("not issued by this runtime"), "{label}: {}", o.stderr);
    }
    // just outside the region (the base value itself, and the first value past the end) the hook says nothing: fle_vfs.c
    // answers with its own stop for a value it did not issue
    for (label, v) in [("base", "FLE_EVENT_HANDLE_BASE"), ("end", "FLE_EVENT_HANDLE_BASE + FLE_EVENT_REGION_SIZE")] {
        let o = run(&format!("event-abort-close-outside-{label}"), &format!("CREATE(1, 0);\n    CALL(fle_imp_CloseHandle, {v});\n    printf(\"returned\\n\");"));
        assert!(!o.ok && !o.stdout.contains("returned"), "{label}");
        assert!(o.stderr.contains("fle: CloseHandle") && !o.stderr.contains("not issued by this runtime"), "{label}: {}", o.stderr);
    }
    // DuplicateHandle of an event is not implemented: fle_vol.c stops on a handle it did not issue
    assert_aborts("event-abort-dup", "uint64_t pp = CALL(fle_imp_GetCurrentProcess, 0), e = CREATE(1, 0), d = 0;", "DuplicateHandle", &[("dup-event", "CALL(fle_imp_DuplicateHandle, pp, e, pp, A(&d), 0, 0, 2);")]);
}

#[test]
fn a_second_thread_stops_the_run() {
    for (label, call) in [
        ("create", "CALL(fle_imp_CreateEventW, 0, 1, 0, 0)"),
        ("set", "CALL(fle_imp_SetEvent, h)"),
        ("reset", "CALL(fle_imp_ResetEvent, h)"),
        ("wait", "CALL(fle_imp_WaitForSingleObject, h, 0)"),
        ("close", "CALL(fle_imp_CloseHandle, h)"),
    ] {
        let api = match label {
            "create" => "CreateEventW",
            "set" => "SetEvent",
            "reset" => "ResetEvent",
            "wait" => "WaitForSingleObject",
            _ => "CloseHandle",
        };
        let o = run_src(
            &format!("event-thread-{label}"),
            &format!("static uint64_t h; static void *th(void *x) {{ (void)x; {call}; printf(\"returned-from-thread\\n\"); return 0; }}"),
            "    pthread_t t;\n    h = CREATE(1, 1);\n    pthread_create(&t, 0, th, 0);\n    pthread_join(t, 0);\n    printf(\"returned\\n\");",
        );
        assert!(!o.ok && !o.stdout.contains("returned"), "{label}: {}", o.stdout);
        assert_eq!(o.signal, Some(6), "{label}");
        assert!(o.stderr.contains(&format!("fle: {api}")) && o.stderr.contains("second thread"), "{label}: {}", o.stderr);
    }
}

// ---- 7. Source checks ----

#[test]
fn source_uses_no_host_clock_thread_process_or_file() {
    let code = strip_comments(&std::fs::read_to_string(src_dir().join("fle_event.c")).unwrap());
    for bad in [
        "getpid", "getppid", "gettid", "getenv", "environ", "fopen", "open(", "stat(", "/proc", "/etc", "/home", "/dev", "/tmp", "exit(", "_exit",
        "clock_gettime", "gettimeofday", "time(", "clock(", "nanosleep", "usleep", "sleep(", "select(", "poll(", "<time.h>", "<sys/time.h>",
        "pthread", "<sys/", "<unistd.h>", "<signal.h>", "fork(", "exec", "syscall", "sched_", "futex", "asm", "__asm__", "uintptr_t", "printf(", "abort(",
    ] {
        assert!(!code.contains(bad), "fle_event.c must not use {bad}");
    }
    // `long` is not portable (32 bits on wasm32 and on Windows, 64 on Linux): fixed-width integers only
    let b = code.as_bytes();
    let mut i = 0;
    while let Some(p) = code[i..].find("long") {
        let at = i + p;
        let before = at == 0 || !(b[at - 1].is_ascii_alphanumeric() || b[at - 1] == b'_');
        let after = at + 4 >= b.len() || !(b[at + 4].is_ascii_alphanumeric() || b[at + 4] == b'_');
        assert!(!(before && after), "fle_event.c uses `long` near byte {at}");
        i = at + 4;
    }
    // the host thread id comes from the portability layer (decision 0045), and fle_stop is the stop
    assert!(code.contains("fle_host_tid()") && code.contains("fle_stop("));
    assert!(code.contains("#include \"fle_host.h\""));
}

#[test]
fn header_is_plain_c() {
    let h = strip_comments(&std::fs::read_to_string(src_dir().join("fle_event.h")).unwrap());
    for bad in ["asm", "__asm__", "<unistd.h>", "<sys/", "<pthread.h>", "<time.h>", "long "] {
        assert!(!h.contains(bad), "fle_event.h must not contain {bad}");
    }
}
