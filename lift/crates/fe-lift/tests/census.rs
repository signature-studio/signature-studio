//! Tests of the census harness (lift/harness/census/census.c, docs/decisions/0097-census-diagnostic.md).
//!
//! census.c and harness/wasm/trace_native.c are compiled with a fake guest: an mmap at 0x400000 that holds an InitContext (VA 0x15EAC90)
//! and a unit table, a loop that does what f_4120c0 does (write InitCount, push the return address 0x412129, `fle_dispatch(proc)`), and
//! unit functions that end in every way the census must handle: return, abort (a named stop), a null-page store, an endless loop
//! (wall time), an endless run of lifted-function entries (the entry limit), host stack overflow, SIGFPE, an unanswered import (a
//! `jmp [rip+rel32]` thunk), a call to an import pseudo-VA. The expected rows are written here from the definitions of the decision (what is
//! restored, what is recorded), not read back from the C code. The real DLL run is the one of lift/harness/census/run_census.sh, not here.

mod etxt_common;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn lift() -> PathBuf {
    manifest().join("../..")
}
fn work_dir() -> PathBuf {
    let d = lift().join("out-surv/test-census");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

fn clang(args: &[&str], files: &[&Path], out: &Path) {
    let l = lift();
    let r = Command::new("clang")
        .args(args)
        .arg("-I")
        .arg(l.join("runtime"))
        .arg("-I")
        .arg(l.join("harness"))
        .arg("-I")
        .arg(l.join("harness/census"))
        .args(files)
        .arg("-o")
        .arg(out)
        .env("TMPDIR", work_dir().join("tmp"))
        .output()
        .expect("clang not found");
    assert!(r.status.success(), "clang failed: {}", String::from_utf8_lossy(&r.stderr));
}

const PROG: &str = r#"
#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <sys/mman.h>
#include "a4.h"
#include "boot.h"
#include "census.h"
#include "fle_dispatch.h"
#include "fle_mxcsr.h"
#include "fle_runtime.h"
#include "fle_x87.h"
#include "fle_x87ctl.h"

uint64_t R[16];
fle_eflags FL;
fle_x87_state fle_x87 = { 0x037F, 0, 0xFFFF };
fle_f80 fle_x87_r[8];
uint8_t fle_x87_mmx_active;
uint64_t MM[8];
uint8_t fle_df;
uint32_t fle_mxcsr = 0x1F80;
unsigned fle_callback_depth(void) { return 0; }
const char *image_export_name(uint64_t va) { (void)va; return NULL; }
#define SLOT 0x1100400ull
const char *image_import_name(uint64_t va) { return va == SLOT ? "kernel32.dll!FakeApi" : NULL; }
void a4_report(void) { printf("a4_report (stub)\n"); }

static volatile int helper_calls;
static void helper(void) { helper_calls++; }
static void leaf(void) {}
static void rec(int n) { volatile char pad[4096]; pad[0] = (char)n; rec(n + 1); pad[1] = 0; }

static void u0(void) { R[3] = 0x1111; R[5] = 0x2222; R[0] = 5; }
static void u1(void) { R[3] = 0xdead; R[5] = 0xbeef; R[4] -= 64; fprintf(stderr, "fle: Foo: bar %d\n", 7); abort(); }
static void u2(void) { R[3] = 0xdead; *(volatile int *)0x10 = 1; }
static void u3(void) { R[4] -= 8; fle_st(R[4], 8, 0x412129); fle_dispatch(0x1100200); R[4] += 8; }
static void u4(void) { volatile unsigned long x = 0; for (;;) x++; }
static void u5(void) { for (;;) leaf(); }
static void u6(void) { rec(0); }
static void u7(void) {
    fle_x87.tw = 0x1234; fle_x87.cw = 0x1; fle_x87_mmx_active = 1; fle_mxcsr = 0x9999; MM[0] = 0x77; fle_df = 1;
    fprintf(stderr, "fle: X: y\n");
    abort();
}
static void u8(void) { volatile int z = 0; R[0] = (uint64_t)(100 / z); }
static void u9(void) { R[0] = 9; }
static void u12(void) { fle_unsupported(0x1100300); }
static void u13(void) { fle_dispatch(SLOT); }
static void u14(void) { fle_dispatch(0); }

const fle_fn_entry fle_dispatch_table[] = {
    { 0x1100000, u0 }, { 0x1100010, u1 }, { 0x1100020, u2 }, { 0x1100030, u3 }, { 0x1100040, u4 }, { 0x1100050, u5 },
    { 0x1100060, u6 }, { 0x1100070, u7 }, { 0x1100080, u8 }, { 0x1100090, u9 }, { 0x11000c0, u12 }, { 0x11000d0, u13 },
    { 0x11000e0, u14 }, { 0x1100200, helper }, { 0x1100210, leaf },
};
const uint64_t fle_dispatch_count = 15;

#define W64(a, v) (*(uint64_t *)(uintptr_t)(a) = (v))
#define W32(a, v) (*(uint32_t *)(uintptr_t)(a) = (v))
#define NUNITS 17
static const uint64_t procs[NUNITS] = {
    0x1100000, 0x1100010, 0x1100020, 0x1100030, 0x1100040, 0x1100050, 0x1100060, 0x1100070, 0x1100080, 0x1100090,
    0x1200000 /* 10: empty */, 0 /* 11: noproc */, 0x11000c0, 0x11000d0, 0x11000e0, 0x1100000 /* 15: u0 again */, 0x1100090 /* 16 */,
};
static uint64_t stack[4096];

int main(int argc, char **argv) {
    int on, depth0;
    (void)argc;
    void *m = mmap((void *)0x400000, 0x1c00000, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED_NOREPLACE, -1, 0);
    if (m == MAP_FAILED || (uintptr_t)m != 0x400000) { puts("mmap failed"); return 2; }
    for (int i = 0; i < 15; i++) W64(fle_dispatch_table[i].va, 0x1111111111111111ull); /* runnable: first 8 bytes not zero */
    W64(0x1000000, NUNITS);
    W64(0x1000008, 0x1000100);
    for (int i = 0; i < NUNITS; i++) W64(0x1000100 + 16 * i, procs[i]);
    W64(0x15EAC90, 0x1000000);
    /* the thunk of unit 12: jmp qword ptr [rip+rel32] to SLOT */
    { uint8_t *t = (uint8_t *)0x1100300; int32_t rel = (int32_t)(SLOT - (0x1100300 + 6)); t[0] = 0xff; t[1] = 0x25; memcpy(t + 2, &rel, 4); }
    setenv("FLE_CENSUS_UNIT_SECONDS", "0.3", 1);
    setenv("FLE_CENSUS_UNIT_CALLS", "1000", 1);
    trace_init();
    on = census_begin(0x400000, 0x1c00000);
    printf("census_begin=%d hook=%d\n", on, fle_dispatch_hook != NULL);
    R[4] = (uint64_t)(uintptr_t)&stack[4000];
    depth0 = (int)trace_depth();
    for (uint64_t i = 0; i < NUNITS; i++) { /* f_4120c0 */
        uint64_t proc, r4;
        fle_st(0x15EAC98, 4, i + 1);
        proc = fle_ld(0x1000100 + 16 * i, 8);
        R[3] = 0x33; R[5] = 0x55; R[0] = 0x77;
        fle_x87.cw = 0x037F; fle_x87.sw = 0; fle_x87.tw = 0xFFFF; fle_x87_mmx_active = 0; fle_mxcsr = 0x1F80; MM[0] = 0; fle_df = 0;
        r4 = R[4];
        if (proc != 0 && fle_ld(proc, 8) != 0) {
            R[4] -= 8;
            fle_st(R[4], 8, 0x412129);
            fle_dispatch(proc);
            R[4] += 8;
            printf("L %llu R0=%llx R3=%llx R5=%llx r4same=%d depth_same=%d cw=%x tw=%x mmx=%d mxcsr=%x mm0=%llx df=%d\n",
                   (unsigned long long)i, (unsigned long long)R[0], (unsigned long long)R[3], (unsigned long long)R[5], R[4] == r4,
                   (int)trace_depth() == depth0, fle_x87.cw, fle_x87.tw, fle_x87_mmx_active, fle_mxcsr, (unsigned long long)MM[0], fle_df);
        }
    }
    printf("helper_calls=%d\n", helper_calls);
    if (argc > 1 && strcmp(argv[1], "after") == 0) { /* a stop after the unit loop: outside every unit */
        fle_x87_mmx_active = 0;
        u0();
        fprintf(stderr, "fle: After: boom\n");
        abort();
    }
    if (on) census_end("returned from entry");
    trace_report("end of the fake guest");
    return 0;
}
"#;

fn exe() -> &'static PathBuf {
    static E: OnceLock<PathBuf> = OnceLock::new();
    E.get_or_init(|| {
        let dir = work_dir();
        let u = etxt_common::per_process("census-cli");
        let c = dir.join(format!("{u}.c"));
        let x = dir.join(&u);
        std::fs::write(&c, PROG).unwrap();
        let l = lift();
        let mut objs = Vec::new();
        for (name, src, instr) in [
            ("prog", c.clone(), true),
            ("census", l.join("harness/census/census.c"), false),
            ("trace", l.join("harness/wasm/trace_native.c"), false),
        ] {
            let o = dir.join(format!("{u}.{name}.o"));
            let mut a = vec!["-std=gnu11", "-O0", "-g", "-w", "-fPIE", "-c"];
            if instr {
                a.push("-finstrument-functions");
            }
            clang(&a, &[&src], &o);
            objs.push(o);
        }
        let refs: Vec<&Path> = objs.iter().map(|p| p.as_path()).collect();
        clang(&["-pie", "-Wl,--wrap=fprintf", "-Wl,--wrap=fputs", "-lm"], &refs, &x);
        x
    })
}

struct Run {
    code: Option<i32>,
    out: String,
    err: String,
}

fn run(mode: &str, census_file: Option<&Path>) -> Run {
    let mut c = Command::new(exe());
    c.arg(mode).env_remove("FLE_CENSUS");
    if let Some(f) = census_file {
        c.env("FLE_CENSUS", f);
    }
    let o = c.output().expect("run failed");
    Run { code: o.status.code(), out: String::from_utf8_lossy(&o.stdout).into(), err: String::from_utf8_lossy(&o.stderr).into() }
}

fn fresh(tag: &str) -> PathBuf {
    work_dir().join(format!("{}.tsv", etxt_common::uniq(tag)))
}

fn rows(path: &Path) -> Vec<Vec<String>> {
    let t = std::fs::read_to_string(path).unwrap();
    t.lines().filter(|l| !l.starts_with('#') && !l.starts_with("unit\t")).map(|l| l.split('\t').map(|s| s.to_string()).collect()).collect()
}

fn row(rs: &[Vec<String>], unit: u32) -> &Vec<String> {
    rs.iter().find(|r| r[0] == unit.to_string()).unwrap_or_else(|| panic!("no row for unit {unit}"))
}

// columns: 0 unit 1 proc 2 state 3 signal 4 fault_addr 5 stack_overflow 6 import 7 top 8 caller1 9 caller2 10 top_guest_return
//          11 entries 12 new_fns 13 ms 14 cb_depth_leak 15 message
fn census_run() -> (Run, Vec<Vec<String>>, String) {
    let f = fresh("census-all");
    let r = run("on", Some(&f));
    let rs = rows(&f);
    let t = std::fs::read_to_string(&f).unwrap();
    (r, rs, t)
}

#[test]
fn every_way_a_unit_can_end_is_recorded_and_the_guest_loop_goes_on() {
    let (r, rs, _t) = census_run();
    assert_eq!(r.code, Some(0), "stdout: {}\nstderr: {}", r.out, r.err);
    assert!(r.out.contains("census_begin=1 hook=1"));
    // unit -> (state, signal)
    for (u, st, sig) in [
        (0, "ok", "-"), (1, "stop", "SIGABRT"), (2, "fault", "SIGSEGV"), (3, "ok", "-"), (4, "hang", "SIGALRM"), (5, "hang", "CALLS"),
        (6, "fault", "SIGSEGV"), (7, "stop", "SIGABRT"), (8, "fault", "SIGFPE"), (9, "ok", "-"), (12, "stop", "SIGABRT"),
        (13, "stop", "SIGABRT"), (14, "stop", "SIGABRT"), (15, "ok", "-"), (16, "ok", "-"),
    ] {
        let x = row(&rs, u);
        assert_eq!((x[2].as_str(), x[3].as_str()), (st, sig), "unit {u}: {x:?}");
    }
    assert_eq!(row(&rs, 10)[2], "empty");
    assert_eq!(row(&rs, 11)[2], "noproc");
    assert_eq!(rs.len(), 17, "one row per unit");
    // the loop reached the last unit (the units after every kind of failure ran)
    assert!(r.out.contains("L 16 R0=9 "), "{}", r.out);
}

#[test]
fn an_abandoned_unit_gets_rax_0_and_its_registers_and_the_fpu_state_back() {
    let (r, _rs, _t) = census_run();
    let line = |u: u32| r.out.lines().find(|l| l.starts_with(&format!("L {u} "))).unwrap_or_else(|| panic!("no line for unit {u}: {}", r.out)).to_string();
    // abandoned units: rax = 0, rbx/rbp as before the call (0x33, 0x55), rsp as before, the shadow stack depth as before
    for u in [1, 2, 4, 5, 6, 7, 8, 12, 13, 14] {
        let l = line(u);
        assert!(l.contains(" R0=0 R3=33 R5=55 r4same=1 depth_same=1 "), "unit {u}: {l}");
    }
    // unit 7 corrupted the x87 control/tag words, the MMX flag, MXCSR, MM0 and the direction flag before it stopped: all back
    assert!(line(7).contains(" cw=37f tw=ffff mmx=0 mxcsr=1f80 mm0=0 df=0"), "{}", line(7));
    // a unit that returns keeps what it did (the census does not touch the registers of a completed unit), rsp as before
    assert!(line(0).contains(" R0=5 R3=1111 R5=2222 r4same=1 depth_same=1 "), "{}", line(0));
}

#[test]
fn the_record_of_a_stop_has_the_message_the_top_function_and_the_import() {
    let (_r, rs, _t) = census_run();
    let u1 = row(&rs, 1);
    assert_eq!(u1[15], "fle: Foo: bar 7");
    assert_eq!(u1[7], "0x1100010", "top of the shadow stack is the unit function: {u1:?}");
    assert_eq!(u1[6], "-");
    // an unsupported instruction at a `jmp [rip+rel32]` thunk names the import behind the slot
    let u12 = row(&rs, 12);
    assert_eq!(u12[15], "fle: unsupported instruction at 0x1100300");
    assert_eq!(u12[6], "kernel32.dll!FakeApi");
    // a call to an import pseudo-VA (fle_dispatch_miss) names it too
    let u13 = row(&rs, 13);
    assert_eq!(u13[15], "fle: indirect call to 0x1100400 is not a known function entry");
    assert_eq!(u13[6], "kernel32.dll!FakeApi");
    // a null function pointer is a dispatch miss to 0
    assert_eq!(row(&rs, 14)[15], "fle: indirect call to 0 is not a known function entry");
}

#[test]
fn faults_hangs_and_the_stack_overflow_are_told_apart() {
    let (_r, rs, _t) = census_run();
    let u2 = row(&rs, 2);
    assert_eq!(u2[4], "0x10", "fault address of the null-page store: {u2:?}");
    assert_eq!(u2[5], "0");
    assert!(u2[15].starts_with("SIGSEGV fault address 0x10"), "{}", u2[15]);
    let u6 = row(&rs, 6);
    assert_eq!(u6[5], "1", "host stack overflow is marked: {u6:?}");
    assert!(u6[15].contains("(host stack)"), "{}", u6[15]);
    // the endless loop: the wall time limit (0.3 s), the endless entries: the entry limit (1000)
    let u4 = row(&rs, 4);
    assert!(u4[15].starts_with("wall time limit of the unit"), "{}", u4[15]);
    let u5 = row(&rs, 5);
    assert!(u5[15].starts_with("function-entry limit of the unit (1000)"), "{}", u5[15]);
    let ms: f64 = u4[13].parse().unwrap();
    assert!((250.0..5000.0).contains(&ms), "the hang took {ms} ms");
    assert_eq!(row(&rs, 8)[3], "SIGFPE");
}

#[test]
fn a_nested_dispatch_inside_a_unit_is_not_a_unit() {
    // unit 3 pushes 0x412129 itself and calls helper through fle_dispatch: it must run (in_unit) and must not become a row
    let (r, rs, _t) = census_run();
    assert!(r.out.contains("helper_calls=1"), "{}", r.out);
    assert_eq!(rs.iter().filter(|x| x[0] == "3").count(), 1);
    assert_eq!(rs.len(), 17);
}

#[test]
fn the_file_ends_with_the_counts_and_says_it_is_not_a4_progress() {
    let (r, _rs, t) = census_run();
    assert!(t.starts_with("# census (decision 0097): NOT A4 progress."), "{t}");
    assert!(t.contains("#end\treturned from entry\t"), "{t}");
    assert!(t.contains("#counts\tunits=17 ok=5 stop=5 fault=3 hang=2 empty=1 noproc=1 unreadable=0 not_reached=0\n"), "{t}");
    assert!(r.out.contains("census: the result is NOT A4 progress (decision 0097)"), "{}", r.out);
    assert!(r.out.contains("not A4 progress"), "{}", r.out);
}

#[test]
fn without_the_variable_nothing_changes() {
    let r = run("on", None);
    // the hook stays NULL and the first named stop ends the run the way it always did: SIGABRT, reported by trace.c, exit code 134
    assert!(r.out.contains("census_begin=0 hook=0"), "{}", r.out);
    assert_eq!(r.code, Some(134), "stdout: {}\nstderr: {}", r.out, r.err);
    assert!(r.out.contains("== stop: SIGABRT (runtime abort, see runtime message)"), "{}", r.out);
    assert!(r.out.contains("runtime message: fle: Foo: bar 7"), "{}", r.out);
    assert!(!r.out.contains("census:"), "{}", r.out);
}

#[test]
fn an_existing_output_file_is_never_overwritten() {
    let f = fresh("census-exists");
    std::fs::write(&f, "keep me\n").unwrap();
    let r = run("on", Some(&f));
    assert_eq!(r.code, Some(2), "stdout: {}\nstderr: {}", r.out, r.err);
    assert!(r.err.contains("an existing file is never overwritten"), "{}", r.err);
    assert_eq!(std::fs::read_to_string(&f).unwrap(), "keep me\n");
}

#[test]
fn a_stop_outside_every_unit_is_written_down_and_then_ends_the_run_as_before() {
    let f = fresh("census-after");
    let r = run("after", Some(&f));
    // trace.c's handler reports and exits with 134, as without the census
    assert_eq!(r.code, Some(134), "stdout: {}\nstderr: {}", r.out, r.err);
    assert!(r.out.contains("runtime message: fle: After: boom"), "{}", r.out);
    let t = std::fs::read_to_string(&f).unwrap();
    assert!(t.contains("#end\tstop outside a unit\tfle: After: boom\t-\t"), "{t}");
    assert!(t.contains("#counts\tunits=17 ok=5 stop=5 fault=3 hang=2 empty=1 noproc=1 unreadable=0 not_reached=0\n"), "{t}");
    assert!(r.out.contains("census: end (stop outside a unit)"), "{}", r.out);
}
