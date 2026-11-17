//! Shared helper of tests/vol.rs and tests/vmap.rs (decision 0024): builds a small C program that calls the runtime
//! stubs the way the lifted code does (arguments in R[1], R[2], R[8], R[9] and on a fake stack, the guest return
//! address at [rsp]), links it with the runtime files and runs it. The host is Linux; nothing is compared with a real
//! Windows. Expected values in the tests are written from the Microsoft pages (cited in
//! docs/investigations/apis-a3-a5-a13-run.md), not read back from the implementation.
//!
//! The C sources of the new unit come from runtime/ when fle_vmap.c is there, otherwise from out-apis/work/
//! (development). Files that already existed always come from runtime/.
#![allow(dead_code)]

use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

#[path = "../etxt_common/mod.rs"]
mod etxt_common;

pub struct Out {
    pub stdout: String,
    pub stderr: String,
    pub signal: Option<i32>,
    pub ok: bool,
}

pub fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn runtime_dir() -> PathBuf {
    manifest().join("../../runtime")
}
pub fn src_dir() -> PathBuf {
    if runtime_dir().join("fle_vmap.c").exists() {
        runtime_dir()
    } else {
        manifest().join("../../out-apis/work")
    }
}
pub fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-apis/test");
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

/// The new files (warnings are errors) and the existing files they link with.
pub fn objs() -> &'static Vec<PathBuf> {
    static O: OnceLock<Vec<PathBuf>> = OnceLock::new();
    O.get_or_init(|| {
        let dir = work_dir().join(etxt_common::per_process("obj-vv"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut v = Vec::new();
        for (name, ours) in [
            ("fle_vol.c", true),
            ("fle_vmap.c", true),
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

pub const PRELUDE: &str = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <setjmp.h>
#include <signal.h>
#include <pthread.h>
#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_sys.h"
#include "fle_vfs.h"
#include "fle_proc.h"
#include "fle_vol.h"
#include "fle_vmap.h"

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

/* A memory fault inside TRY(...) is caught, so that a test can show that a protection is enforced by the host. */
static sigjmp_buf jb;
static void on_segv(int s) { (void)s; siglongjmp(jb, 1); }
static void install_segv(void) { struct sigaction sa; memset(&sa, 0, sizeof sa); sa.sa_handler = on_segv; sa.sa_flags = SA_NODEFER; sigaction(SIGSEGV, &sa, 0); }
#define FAULTS(stmt) ({ volatile int faulted = 0; if (sigsetjmp(jb, 1) == 0) { stmt; } else { faulted = 1; } faulted; })
"#;

pub fn run_src(name: &str, top: &str, body: &str) -> Out {
    let dir = work_dir();
    let uname = etxt_common::uniq(name);
    let c = dir.join(format!("{uname}.c"));
    let exe = dir.join(&uname);
    let src = format!("{PRELUDE}\n{top}\nint main(void)\n{{\n    install_segv();\n{body}\n    return 0;\n}}\n");
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

pub fn run(name: &str, body: &str) -> Out {
    run_src(name, "", body)
}

/// Runs `body` and requires normal exit; returns the stdout lines.
pub fn lines(name: &str, body: &str) -> Vec<String> {
    let o = run(name, body);
    assert!(o.ok, "{name} did not exit normally (signal {:?}): {}\n{}", o.signal, o.stderr, o.stdout);
    o.stdout.lines().map(|s| s.to_string()).collect()
}

/// Each (label, C statements) must stop the run with SIGABRT, a "fle:" message and no output after it. `setup` runs first.
pub fn assert_aborts(prefix: &str, setup: &str, cases: &[(&str, &str)]) {
    for (label, stmts) in cases {
        let body = format!("{setup}\n    {stmts}\n    printf(\"returned %llu\\n\", U(R[0]));");
        let o = run(&format!("{prefix}-{label}"), &body);
        assert!(!o.ok, "{label}: expected an abort, got stdout {}", o.stdout);
        assert!(!o.stdout.contains("returned"), "{label}: returned instead of stopping");
        assert!(o.stderr.contains("fle:"), "{label}: no fle: message: {}", o.stderr);
        assert_eq!(o.signal, Some(6), "{label}: not SIGABRT");
    }
}

/// A C expression for a guest UTF-16 string (the extra parentheses keep the braces of the literal inside one macro argument).
pub fn cw(s: &str) -> String {
    let mut v: Vec<String> = s.encode_utf16().map(|u| u.to_string()).collect();
    v.push("0".into());
    format!("A(((const uint16_t[]){{ {} }}))", v.join(", "))
}

/// The value of `#define NAME UINT64_C(...)` in a runtime header (new headers come from src_dir).
pub fn header_const(file: &str, name: &str) -> u64 {
    let t = std::fs::read_to_string(src_dir().join(file)).or_else(|_| std::fs::read_to_string(runtime_dir().join(file))).unwrap();
    let line = t.lines().find(|l| l.contains(&format!("#define {name} "))).unwrap_or_else(|| panic!("{name} not in {file}"));
    let tok = line.split("UINT64_C(").nth(1).unwrap().split(')').next().unwrap();
    u64::from_str_radix(tok.trim_start_matches("0x"), 16).unwrap()
}

/// C text without comments (strings are kept).
pub fn strip_comments(s: &str) -> String {
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
