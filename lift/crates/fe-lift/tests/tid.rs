//! Tests of decision 0056 (docs/decisions/0056-virtual-thread-id.md): the thread id the guest sees from
//! kernel32!GetCurrentThreadId is the fixed virtual value FLE_EMULATED_TID = 0x1001 (4097), the same on native and on wasm,
//! whatever the host's real tid is. The host tid (fle_host_tid) stays for the single-thread rule (decision 0017) only.
//!
//! 1. The value: 4097, nonzero, stable, a 32-bit DWORD (the upper half of R[0] is 0), arguments not read.
//! 2. Not the host: it differs from the real Linux tid, is the same from another host thread (the extension point of the
//!    decision is a second guest thread, which is not built), and the same program built as emscripten (__EMSCRIPTEN__
//!    defined by hand, where fle_host_tid is the constant 1) prints the same value.
//! 3. The source of fle_imp_GetCurrentThreadId does not use the host tid, and the owner slots still do (0 is "not recorded").
//!
//! The expected value is written here from the decision (a setting of this environment: the Learn page gives no value), not
//! read back from the C code. FLE_TID_RT_DIR (a full copy of runtime/) replaces runtime/ for the mutation runs.

mod etxt_common;

use std::path::PathBuf;
use std::process::Command;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn rt() -> PathBuf {
    match std::env::var_os("FLE_TID_RT_DIR") {
        Some(d) => PathBuf::from(d),
        None => manifest().join("../../runtime"),
    }
}
fn work() -> PathBuf {
    let d = manifest().join("../../out-tid/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

const FILES: [&str; 8] = ["fle_sys.c", "fle_vmap.c", "fle_vfs.c", "fle_env.c", "fle_imports.c", "fle_host.c", "fle_proc.c", "fle_time.c"];

const PRE: &str = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <unistd.h>
#include <pthread.h>
#include <sys/syscall.h>
#include "fle_runtime.h"
#include "fle_host.h"
#include "fle_imports.h"
#ifdef __EMSCRIPTEN__
_Thread_local unsigned char fle_ctx[0x300] __attribute__((aligned(64), used));   /* the register block of the wasm build (fle_ctx_tls.c): R, FL and X live in it */
#else
uint64_t R[16];
#endif
#define U(x) ((unsigned long long)(x))
"#;

/// Compiles `PRE + body` with the runtime files, runs it, returns stdout lines. `defs` are extra clang flags.
fn run(name: &str, body: &str, defs: &[&str]) -> Vec<String> {
    let dir = work();
    let u = etxt_common::uniq(name);
    let c = dir.join(format!("{u}.c"));
    let exe = dir.join(&u);
    std::fs::write(&c, format!("{PRE}{body}")).unwrap();
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fPIE", "-pie", "-pthread"]);
    for d in defs {
        cmd.arg(d);
    }
    cmd.arg("-I").arg(rt()).arg(&c);
    for f in FILES {
        cmd.arg(rt().join(f));
    }
    let cc = cmd.arg("-o").arg(&exe).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new(&exe).output().expect("run failed");
    assert!(r.status.success(), "{name}: {:?}\n{}", r.status, String::from_utf8_lossy(&r.stderr));
    String::from_utf8_lossy(&r.stdout).lines().map(|s| s.to_string()).collect()
}

const MAIN_VALUE: &str = r#"
int main(void)
{
    uint64_t a, b;
    R[0] = 0x99; R[1] = 0x1234; R[2] = 0x5678; R[8] = 0x9abc; R[9] = 0xdef0;
    fle_imp_GetCurrentThreadId();
    a = R[0];
    R[0] = 0x99; R[1] = 0; R[2] = 0; R[8] = 0; R[9] = 0;
    fle_imp_GetCurrentThreadId();
    b = R[0];
    printf("value %llu\n", U(a));
    printf("same_with_other_arguments %d\n", a == b);
    printf("nonzero %d\n", a != 0);
    printf("upper_half_zero %d\n", (a >> 32) == 0);
    return 0;
}
"#;

#[test]
fn guest_thread_id_is_the_fixed_virtual_value() {
    let l = run("tid-value", MAIN_VALUE, &[]);
    assert_eq!(l, ["value 4097", "same_with_other_arguments 1", "nonzero 1", "upper_half_zero 1"]);
}

#[test]
fn guest_thread_id_is_not_the_host_tid() {
    let l = run(
        "tid-not-host",
        r#"
int main(void)
{
    long host = (long)syscall(SYS_gettid);
    fle_imp_GetCurrentThreadId();
    printf("guest %llu\n", U(R[0]));
    printf("host_tid_known %d\n", host > 0 && fle_host_tid() == host);
    printf("guest_is_not_host %d\n", R[0] != (uint64_t)(uint32_t)host && R[0] != (uint64_t)host);
    return 0;
}
"#,
        &[],
    );
    // the Linux tid of a test process is not 4097 in any realistic run, but the check above is on the value that
    // this very process has: the line is exact only for "guest"
    assert_eq!(l[0], "guest 4097");
    assert_eq!(l[1], "host_tid_known 1");
    assert_eq!(l[2], "guest_is_not_host 1", "a host whose tid happens to be 4097 would fail here: rerun");
}

#[test]
fn guest_thread_id_is_the_same_from_another_host_thread() {
    // The run has one guest thread (decision 0017); the virtual id does not depend on the host thread that runs the
    // call. A second guest thread would need its own value (decision 0056, extension point): this test then changes.
    let l = run(
        "tid-thread",
        r#"
static uint64_t other;
static long other_host;
static void *th(void *p)
{
    (void)p;
    other_host = (long)syscall(SYS_gettid);
    fle_imp_GetCurrentThreadId();
    other = R[0];
    return 0;
}
int main(void)
{
    pthread_t t;
    uint64_t mine;
    fle_imp_GetCurrentThreadId();
    mine = R[0];
    pthread_create(&t, 0, th, 0);
    pthread_join(t, 0);
    printf("main %llu other %llu\n", U(mine), U(other));
    printf("host_tids_differ %d\n", other_host != (long)syscall(SYS_gettid));
    return 0;
}
"#,
        &[],
    );
    assert_eq!(l, ["main 4097 other 4097", "host_tids_differ 1"]);
}

#[test]
fn native_and_emscripten_hosts_give_the_guest_the_same_value() {
    // fle_host.h with __EMSCRIPTEN__ defined by hand: fle_host_tid() is the constant 1 there (decision 0045), and the
    // guest value must not follow it. The two builds print the same lines.
    let native = run("tid-native", MAIN_VALUE, &[]);
    let wasm_like = run("tid-wasmlike", MAIN_VALUE, &["-D__EMSCRIPTEN__", "-DFLE_VMEM_ARENA"]);
    assert_eq!(native, wasm_like);
    assert_eq!(wasm_like[0], "value 4097");
    let host = run(
        "tid-wasmlike-host",
        r#"
int main(void)
{
    fle_imp_GetCurrentThreadId();
    printf("host %ld guest %llu\n", fle_host_tid(), U(R[0]));
    return 0;
}
"#,
        &["-D__EMSCRIPTEN__", "-DFLE_VMEM_ARENA"],
    );
    assert_eq!(host, ["host 1 guest 4097"]);
}

#[test]
fn guest_thread_id_does_not_change_between_runs() {
    // two processes, two different real tids (and pids): the same output
    let a = run("tid-run-a", MAIN_VALUE, &[]);
    let b = run("tid-run-b", MAIN_VALUE, &[]);
    assert_eq!(a, b);
}

fn strip_comments(s: &str) -> String {
    let b: Vec<char> = s.chars().collect();
    let (mut o, mut i) = (String::new(), 0);
    while i < b.len() {
        if b[i] == '/' && i + 1 < b.len() && b[i + 1] == '*' {
            i += 2;
            while i + 1 < b.len() && !(b[i] == '*' && b[i + 1] == '/') {
                i += 1;
            }
            i += 2;
        } else {
            o.push(b[i]);
            i += 1;
        }
    }
    o
}

#[test]
fn source_of_the_answer_does_not_use_the_host_tid_but_the_owner_checks_still_do() {
    let code = strip_comments(&std::fs::read_to_string(rt().join("fle_imports.c")).unwrap());
    let start = code.find("void fle_imp_GetCurrentThreadId(void)").expect("fle_imp_GetCurrentThreadId");
    let body = &code[start..code[start..].find("\n}\n").map(|e| start + e).unwrap()];
    for bad in ["fle_host_tid", "gettid", "SYS_gettid", "getpid", "pthread_self"] {
        assert!(!body.contains(bad), "fle_imp_GetCurrentThreadId must not use {bad}");
    }
    assert!(body.contains("FLE_EMULATED_TID"));
    // the owner slots keep the host tid (0 means "not recorded yet", so it must not be 0 on wasm either)
    assert!(code.contains("fle_host_tid()"), "the TLS owner check keeps using fle_host_tid()");
    let host = std::fs::read_to_string(rt().join("fle_host.h")).unwrap();
    assert!(host.contains("#define FLE_HOST_WASM_TID 1L"), "the wasm host tid is the constant 1 (never 0)");
}

#[test]
fn the_virtual_value_is_in_the_header_and_is_not_zero() {
    let h = std::fs::read_to_string(rt().join("fle_imports.h")).unwrap();
    let line = h.lines().find(|l| l.starts_with("#define FLE_EMULATED_TID")).expect("FLE_EMULATED_TID");
    let v = line.split_whitespace().nth(2).unwrap().trim_end_matches('u');
    let n = u64::from_str_radix(v.trim_start_matches("0x"), 16).unwrap();
    assert_eq!(n, 0x1001);
    assert!(n != 0 && n <= u32::MAX as u64);
}
