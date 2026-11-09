//! Tests of the A4 survey report of the harness (lift/harness/a4.c, docs/investigations/a4-survey-tool.md, decision 0060).
//!
//! a4.c is compiled with a fake image (an mmap at 0x400000, as harness/image.c does), a hand-made InitContext (VA 0x15EAC90)
//! and unit table, and stubs for the trace hooks (`trace_fn_seen`, `trace_fn_on_stack`, ... that trace.c provides), so each
//! input fixes the output. The expected lines are written here from the definitions of the decision (InitCount is the index+1
//! written BEFORE the unit is called; started = runnable units with index < InitCount; completed = started - in_progress),
//! not read back from the C code. The real DLL run is checked by `lift/harness/a4-survey.sh`, not here.
//!
//! FLE_A4_SRC (a directory with a4.c and a4.h) replaces lift/harness/ for the mutation runs of the run record.

mod etxt_common;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn runtime_dir() -> PathBuf {
    manifest().join("../../runtime")
}
fn harness_dir() -> PathBuf {
    manifest().join("../../harness")
}
fn src_dir() -> PathBuf {
    match std::env::var("FLE_A4_SRC") {
        Ok(d) => PathBuf::from(d),
        Err(_) => harness_dir(),
    }
}
fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-surv/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

fn clang(args: &[&str], extra: &[&Path], out: &Path) {
    let r = Command::new("clang")
        .args(args)
        .arg("-I")
        .arg(src_dir())
        .arg("-I")
        .arg(harness_dir())
        .arg("-I")
        .arg(runtime_dir())
        .args(extra)
        .arg("-o")
        .arg(out)
        .env("TMPDIR", work_dir().join("tmp"))
        .output()
        .expect("clang not found");
    assert!(r.status.success(), "clang failed: {}", String::from_utf8_lossy(&r.stderr));
}

/// The fake guest. Image 0x400000..0x2000000. Header at 0x1000000, units at 0x1000100 (6 units), procs:
/// unit 0 A = 0x1100000, unit 1 empty proc 0x1200000 (zero bytes), unit 2 noproc, unit 3 B = 0x1100010, unit 4 C = 0x1100020,
/// unit 5 D = 0x1100030 (or, with `bad`, 0x50000000: outside the image). A..D are runnable (non-zero first 8 bytes) and are the
/// entries of the fake fle_dispatch_table (sorted by VA).
const PROG: &str = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <sys/mman.h>
#include "a4.h"
#include "fle_dispatch.h"
#include "fle_runtime.h"

uint64_t R[16];
static void fa(void) {}
static void fb(void) {}
static void fc(void) {}
static void fd(void) {}
const fle_fn_entry fle_dispatch_table[] = {
    { 0x1100000, fa }, { 0x1100010, fb }, { 0x1100020, fc }, { 0x1100030, fd },
};
const uint64_t fle_dispatch_count = 4;

static unsigned seen_mask, stack_mask, distinct = 7, calls = 99;
static int bit(void *fn) {
    if (fn == (void *)fa) return 0;
    if (fn == (void *)fb) return 1;
    if (fn == (void *)fc) return 2;
    return 3;
}
int trace_fn_seen(void *fn) { return (seen_mask >> bit(fn)) & 1; }
int trace_fn_on_stack(void *fn) { return (stack_mask >> bit(fn)) & 1; }
uint64_t trace_distinct_count(void) { return distinct; }
uint64_t trace_call_count(void) { return calls; }

#define A4_CTX 0x15EAC90ull
#define A4_COUNT 0x15EAC98ull
#define W64(a, v) (*(uint64_t *)(uintptr_t)(a) = (v))
#define W32(a, v) (*(uint32_t *)(uintptr_t)(a) = (v))

/* argv: mode seen_hex stack_hex count nunits ctxptr(0|1) bad(0|1) */
int main(int argc, char **argv) {
    (void)argc;
    const char *mode = argv[1];
    seen_mask = (unsigned)strtoul(argv[2], 0, 16);
    stack_mask = (unsigned)strtoul(argv[3], 0, 16);
    unsigned count = (unsigned)strtoul(argv[4], 0, 0), nunits = (unsigned)strtoul(argv[5], 0, 0);
    int ctxptr = atoi(argv[6]), bad = atoi(argv[7]);
    if (strcmp(mode, "noimage") == 0) { a4_report(); return 0; }
    void *m = mmap((void *)0x400000, 0x1c00000, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED_NOREPLACE, -1, 0);
    if (m == MAP_FAILED || (uintptr_t)m != 0x400000) { puts("mmap failed"); return 2; }
    a4_set_image(0x400000, 0x1c00000);
    /* runnable tests the whole 8 bytes: B has zero low dwords, C zero high dwords */
    W64(0x1100000, 0x1111111111111111ull); W64(0x1100010, 0x2222000000000000ull); W64(0x1100020, 0x3333); W64(0x1100030, 0x4444);
    W64(0x1200000, 0);
    W64(0x1000000, 0xdeadbeef00000000ull | nunits); /* UnitCount is a u32: the next 4 bytes are garbage */
    W64(0x1000008, 0x1000100);
    if (strcmp(mode, "hugecount") == 0) W64(0x1000000, 0xdeadbeef00000000ull | 0x20000);
    if (strcmp(mode, "unitsout") == 0) W64(0x1000008, 0x1ffffff0);
    W64(0x1000100 + 16 * 0, 0x1100000);
    W64(0x1000100 + 16 * 1, 0x1200000);
    W64(0x1000100 + 16 * 2, 0);
    W64(0x1000100 + 16 * 3, 0x1100010);
    W64(0x1000100 + 16 * 4, 0x1100020);
    W64(0x1100040, 0x5555); /* runnable bytes, but not an entry of fle_dispatch_table */
    {
        /* unit 5's proc: the modes of the edge tests (image is 0x400000..0x2000000) */
        uint64_t p5 = 0x1100030ull;
        if (bad) p5 = 0x50000000ull;
        else if (strcmp(mode, "nolift") == 0) p5 = 0x1100040ull;
        else if (strcmp(mode, "last8") == 0) p5 = 0x1fffff8ull;   /* the last 8 bytes of the image: inside */
        else if (strcmp(mode, "last7") == 0) p5 = 0x1fffff9ull;   /* 7 bytes left, 8 do not fit: outside */
        else if (strcmp(mode, "last4") == 0) p5 = 0x1fffffcull;   /* 8 bytes do not fit: outside */
        else if (strcmp(mode, "end") == 0) p5 = 0x2000000ull;     /* one past the image: outside */
        else if (strcmp(mode, "end1") == 0) p5 = 0x2000001ull;
        else if (strcmp(mode, "below") == 0) p5 = 0x3ffff8ull;    /* 8 bytes below ImageBase: outside */
        W64(0x1000100 + 16 * 5, p5);
    }
    if (strcmp(mode, "outside") == 0) W64(A4_CTX, 0x50000000ull);
    else W64(A4_CTX, ctxptr ? 0x1000000 : 0);
    W64(A4_COUNT, 0xdeadbeef00000000ull | count); /* InitCount is a u32 */
    if (strncmp(mode, "sampled", 7) == 0) {
        /* decision 0113: the trace hook samples while the context is set; the program's exit path then zeroes the header and the count */
        a4_sample();
        if (strcmp(mode, "sampled-down") == 0) { W64(A4_COUNT, 0xdeadbeef00000000ull | 2); a4_sample(); }   /* a smaller count later: the largest stays */
        W64(A4_CTX, 0);
        W64(A4_COUNT, 0xdeadbeef00000000ull);
        a4_sample();   /* a sample with the header zero changes nothing */
    }
    if (strcmp(mode, "unsampled-clear") == 0) { W64(A4_CTX, 0); W64(A4_COUNT, 0); }
    a4_report();
    return 0;
}
"#;

fn exe() -> &'static PathBuf {
    static E: OnceLock<PathBuf> = OnceLock::new();
    E.get_or_init(|| {
        let dir = work_dir();
        let u = etxt_common::per_process("a4-cli");
        let c = dir.join(format!("{u}.c"));
        let o = dir.join(format!("{u}.a4.o"));
        let x = dir.join(&u);
        std::fs::write(&c, PROG).unwrap();
        clang(&["-O1", "-std=gnu11", "-Wall", "-Wextra", "-Werror", "-fPIE", "-c", src_dir().join("a4.c").to_str().unwrap()], &[], &o);
        clang(&["-O0", "-std=gnu11", "-w", "-fPIE", "-pie"], &[&c, &o], &x);
        x
    })
}

/// The a4 lines (with the "a4: " prefix) of one run: mode, seen mask, stack mask, count, nunits, ctx pointer set, bad unit 5.
fn run(mode: &str, seen: u32, stack: u32, count: u32, nunits: u32, ctxptr: bool, bad: bool) -> Vec<String> {
    let o = Command::new(exe())
        .args([mode, &format!("{seen:x}"), &format!("{stack:x}"), &count.to_string(), &nunits.to_string(), if ctxptr { "1" } else { "0" }, if bad { "1" } else { "0" }])
        .output()
        .expect("run failed");
    assert!(o.status.success(), "{mode}: exit {:?}: {}{}", o.status, String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    String::from_utf8_lossy(&o.stdout).lines().map(|s| s.to_string()).collect()
}

fn ok(seen: u32, stack: u32, count: u32) -> Vec<String> {
    run("ok", seen, stack, count, 6, true, false)
}

// bit 0 = A (unit 0), bit 1 = B (unit 3), bit 2 = C (unit 4), bit 3 = D (unit 5)
const A: u32 = 1;
const B: u32 = 2;
const C: u32 = 4;
const D: u32 = 8;

#[test]
fn in_progress_unit() {
    // InitCount 4: units 0..3 reached. A started and returned, B (unit 3) is on the call stack.
    let l = ok(A | B, B, 4);
    assert_eq!(l[0], "a4: init_count=4 of 6, last_unit_proc=0x1100010");
    assert_eq!(l[1], "a4: last_unit index=3 run_rank=2 of 4 proc=0x1100010 state=in_progress");
    assert_eq!(l[2], "a4: runnable_started=2 completed=1 in_progress=1 of 4 runnable");
    assert_eq!(l[3], "a4: unit[init_count-1] proc=0x1100010; units_outside_image=0");
    assert_eq!(l[4], "a4: entry_check started_but_not_entered=0 runnable_entered_beyond_init_count=0");
    assert_eq!(l[5], "a4: all_units_done=no functions_distinct=7 function_entries=99");
    assert_eq!(l.len(), 6);
}

#[test]
fn init_count_on_a_noproc_unit_counts_the_previous_runnable_unit_as_last() {
    // InitCount 3: the loop reached index 2 (noproc: nothing was called). The last started runnable unit is A (index 0), returned.
    let l = ok(A, 0, 3);
    assert_eq!(l[0], "a4: init_count=3 of 6, last_unit_proc=0x1100000");
    assert_eq!(l[1], "a4: last_unit index=0 run_rank=1 of 4 proc=0x1100000 state=returned");
    assert_eq!(l[2], "a4: runnable_started=1 completed=1 in_progress=0 of 4 runnable");
    assert_eq!(l[3], "a4: unit[init_count-1] proc=0x0; units_outside_image=0");
}

#[test]
fn init_count_on_an_empty_unit() {
    // InitCount 2: index 1 is the empty unit (skipped by the loop). Started = {A}.
    let l = ok(A, 0, 2);
    assert_eq!(l[1], "a4: last_unit index=0 run_rank=1 of 4 proc=0x1100000 state=returned");
    assert_eq!(l[2], "a4: runnable_started=1 completed=1 in_progress=0 of 4 runnable");
    assert_eq!(l[3], "a4: unit[init_count-1] proc=0x1200000; units_outside_image=0");
}

#[test]
fn nothing_started() {
    let l = ok(0, 0, 0);
    assert_eq!(l[0], "a4: init_count=0 of 6, last_unit_proc=0x0");
    assert_eq!(l[1], "a4: last_unit index=0 run_rank=0 of 4 proc=0x0 state=none");
    assert_eq!(l[2], "a4: runnable_started=0 completed=0 in_progress=0 of 4 runnable");
    assert_eq!(l[3], "a4: unit[init_count-1] proc=0x0; units_outside_image=0");
    assert!(l[5].starts_with("a4: all_units_done=no "));
}

#[test]
fn entry_check_counts_started_units_that_were_never_entered() {
    // C (unit 4) is started by InitCount 5 but was not entered; B was.
    let l = ok(A | B, 0, 5);
    assert_eq!(l[2], "a4: runnable_started=3 completed=3 in_progress=0 of 4 runnable");
    assert_eq!(l[4], "a4: entry_check started_but_not_entered=1 runnable_entered_beyond_init_count=0");
}

#[test]
fn entry_check_counts_units_entered_beyond_init_count() {
    // D was entered (e.g. as a callee of an earlier unit) although InitCount has not reached unit 5.
    let l = ok(A | B | D, 0, 4);
    assert_eq!(l[4], "a4: entry_check started_but_not_entered=0 runnable_entered_beyond_init_count=1");
    // and not for a unit that is started
    let l = ok(A | B | C | D, 0, 6);
    assert_eq!(l[4], "a4: entry_check started_but_not_entered=0 runnable_entered_beyond_init_count=0");
}

#[test]
fn all_units_done_needs_the_full_count_and_no_unit_in_progress() {
    let l = ok(A | B | C | D, 0, 6);
    assert_eq!(l[0], "a4: init_count=6 of 6, last_unit_proc=0x1100030");
    assert_eq!(l[1], "a4: last_unit index=5 run_rank=4 of 4 proc=0x1100030 state=returned");
    assert_eq!(l[2], "a4: runnable_started=4 completed=4 in_progress=0 of 4 runnable");
    assert_eq!(l[3], "a4: unit[init_count-1] proc=0x1100030; units_outside_image=0");
    assert_eq!(l[5], "a4: all_units_done=yes functions_distinct=7 function_entries=99");
    // InitCount 6 but the last unit D is still running: not done, completed 3
    let l = ok(A | B | C | D, D, 6);
    assert_eq!(l[2], "a4: runnable_started=4 completed=3 in_progress=1 of 4 runnable");
    assert_eq!(l[5], "a4: all_units_done=no functions_distinct=7 function_entries=99");
    // InitCount 5 of 6 with everything returned: not done
    let l = ok(A | B | C, 0, 5);
    assert_eq!(l[5], "a4: all_units_done=no functions_distinct=7 function_entries=99");
}

#[test]
fn an_earlier_unit_on_the_stack_is_not_in_progress_for_the_last_one() {
    // B (the last started unit) returned; A is on the stack for some other reason: in_progress concerns the last unit only.
    let l = ok(A | B, A, 4);
    assert_eq!(l[1], "a4: last_unit index=3 run_rank=2 of 4 proc=0x1100010 state=returned");
    assert_eq!(l[2], "a4: runnable_started=2 completed=2 in_progress=0 of 4 runnable");
}

#[test]
fn init_count_above_the_unit_count_is_not_a_crash() {
    let l = ok(A | B | C | D, 0, 100);
    assert_eq!(l[0], "a4: init_count=100 of 6, last_unit_proc=0x1100030");
    assert_eq!(l[3], "a4: unit[init_count-1] proc=0x0; units_outside_image=0");
    assert_eq!(l[5], "a4: all_units_done=no functions_distinct=7 function_entries=99");
}

#[test]
fn a_unit_outside_the_image_is_counted_and_not_read() {
    let l = run("ok", A | B | C, 0, 6, 6, true, true);
    assert_eq!(l[2], "a4: runnable_started=3 completed=3 in_progress=0 of 3 runnable");
    assert_eq!(l[3], "a4: unit[init_count-1] proc=0x50000000; units_outside_image=1");
}

#[test]
fn a_started_unit_that_is_not_a_lifted_function_counts_as_not_entered() {
    // unit 5's proc is runnable in the image but has no entry in fle_dispatch_table: it cannot have been entered.
    let l = run("nolift", A | B | C | D, 0, 6, 6, true, false);
    assert_eq!(l[1], "a4: last_unit index=5 run_rank=4 of 4 proc=0x1100040 state=returned");
    assert_eq!(l[2], "a4: runnable_started=4 completed=4 in_progress=0 of 4 runnable");
    assert_eq!(l[4], "a4: entry_check started_but_not_entered=1 runnable_entered_beyond_init_count=0");
}

#[test]
fn the_edges_of_the_image() {
    // the last 8 bytes of the image are readable: the unit is empty (zero bytes), not outside
    let l = run("last8", A | B | C, 0, 6, 6, true, false);
    assert_eq!(l[2], "a4: runnable_started=3 completed=3 in_progress=0 of 3 runnable");
    assert_eq!(l[3], "a4: unit[init_count-1] proc=0x1fffff8; units_outside_image=0");
    // everything that does not fit 8 bytes inside the image is outside (and is not read: the run would fault)
    for (mode, p5) in [("last7", "0x1fffff9"), ("last4", "0x1fffffc"), ("end", "0x2000000"), ("end1", "0x2000001"), ("below", "0x3ffff8")] {
        let l = run(mode, A | B | C, 0, 6, 6, true, false);
        assert_eq!(l[2], "a4: runnable_started=3 completed=3 in_progress=0 of 3 runnable", "{mode}");
        assert_eq!(l[3], format!("a4: unit[init_count-1] proc={p5}; units_outside_image=1"), "{mode}");
    }
}

#[test]
fn context_not_yet_written() {
    let l = run("ok", 0, 0, 0, 6, false, false);
    assert_eq!(l, ["a4: init_count=0 of ?, InitContext+0 is 0 (InitUnits not reached)"]);
}

#[test]
fn bad_context_is_reported_not_followed() {
    let l = run("outside", 0, 0, 3, 6, true, false);
    assert_eq!(l, ["a4: init_count=3 of ?, InitContext+0=0x50000000 is outside the image"]);
    let l = run("hugecount", 0, 0, 3, 6, true, false);
    assert_eq!(l, ["a4: init_count=3 of ?, bad table header 0x1000000 (count 131072, units 0x1000100)"]);
    let l = run("unitsout", 0, 0, 3, 6, true, false);
    assert_eq!(l, ["a4: init_count=3 of ?, bad table header 0x1000000 (count 6, units 0x1ffffff0)"]);
}

const NOTE: &str = "a4: note: InitContext+0 is 0 now (the program's exit path zeroed it; the unit finalizers were not called); the table 0x1000000 and init_count=";

/// Decision 0113: when the header is zero at the report but a non-zero header was sampled during the run, the sampled table and the largest sampled InitCount are used, with a note
/// line in front; the other lines are the ones of a stop with that state (here: all six positions reached, nothing on the stack: done).
#[test]
fn a_header_zeroed_by_the_exit_path_is_replaced_by_the_sampled_one() {
    let l = run("sampled", A | B | C | D, 0, 6, 6, true, false);
    assert_eq!(l[0], format!("{NOTE}6 are the last non-zero header and the largest InitCount sampled at lifted function entries"));
    assert_eq!(l[1], "a4: init_count=6 of 6, last_unit_proc=0x1100030");
    assert_eq!(l[2], "a4: last_unit index=5 run_rank=4 of 4 proc=0x1100030 state=returned");
    assert_eq!(l[3], "a4: runnable_started=4 completed=4 in_progress=0 of 4 runnable");
    assert_eq!(l[4], "a4: unit[init_count-1] proc=0x1100030; units_outside_image=0");
    assert_eq!(l[5], "a4: entry_check started_but_not_entered=0 runnable_entered_beyond_init_count=0");
    assert_eq!(l[6], "a4: all_units_done=yes functions_distinct=7 function_entries=99");
    assert_eq!(l.len(), 7);
    // the largest count stays when a later sample is smaller; a partial count is reported as partial
    let l = run("sampled-down", A | B, 0, 4, 6, true, false);
    assert_eq!(l[0], format!("{NOTE}4 are the last non-zero header and the largest InitCount sampled at lifted function entries"));
    assert_eq!(l[1], "a4: init_count=4 of 6, last_unit_proc=0x1100010");
    assert_eq!(l[6], "a4: all_units_done=no functions_distinct=7 function_entries=99");
    // nothing sampled while the context was zero: the old one-line report
    let l = run("unsampled-clear", 0, 0, 6, 6, true, false);
    assert_eq!(l, ["a4: init_count=0 of ?, InitContext+0 is 0 (InitUnits not reached)"]);
    // a header that is non-zero at the report is not replaced by the sampled one (no sampling in this mode, so no note either)
    let l = ok(A | B | C | D, 0, 6);
    assert!(!l[0].starts_with("a4: note:"), "{l:?}");
}

#[test]
fn no_image_means_a_note_and_no_read() {
    let l = run("noimage", 0, 0, 0, 6, true, false);
    assert_eq!(l, ["a4: image not loaded, nothing to read"]);
}
