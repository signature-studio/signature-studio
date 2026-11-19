//! Shared helper of tests/vfs.rs, reg.rs and env.rs (decision 0018): builds a small C program that calls the
//! runtime stubs the way the lifted code does (arguments in R[1], R[2], R[8], R[9] and on a fake stack, the
//! guest return address at [rsp]), links it with the virtual runtime and runs it. The host is Linux; nothing is
//! compared with a real Windows. Expected values in the tests are written from the Microsoft pages (cited in
//! docs/investigations/import-vfs-spec.md), not read back from the implementation.
#![allow(dead_code)]

use std::path::PathBuf;
use std::process::Command;

#[path = "../etxt_common/mod.rs"]
mod etxt_common;

pub struct Out {
    pub stdout: String,
    pub stderr: String,
    pub signal: Option<i32>,
    pub ok: bool,
}

pub fn runtime_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../runtime")
}

fn work_dir() -> PathBuf {
    let d = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../out-vfs/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

pub const PRELUDE: &str = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_modules.h"
#include "fle_vfs.h"
#include "fle_reg.h"
#include "fle_env.h"

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
#define CALL(f, ...) callv(f, (uint64_t[12]){ __VA_ARGS__ })
#define A(x) ((uint64_t)(uintptr_t)(x))
#define W(s) A(u##s)
#define NUL 0
static uint32_t LE(void) { fle_imp_GetLastError(); return (uint32_t)R[0]; }
static void SETLE(uint32_t v) { fle_set_last_error(v); }
static char asc_buf[8][1024];
static int asc_i;
static const char *AS(const uint16_t *s)
{
    char *o = asc_buf[asc_i++ & 7];
    size_t n = 0;
    while (s[n] && n < 1000) { o[n] = (char)s[n]; n++; }
    o[n] = 0;
    return o;
}
#define U(x) ((unsigned long long)(x))
"#;

pub fn run(name: &str, body: &str) -> Out {
    let rt = runtime_dir();
    let dir = work_dir();
    let uname = etxt_common::uniq(name);
    let c = dir.join(format!("{uname}.c"));
    let exe = dir.join(&uname);
    let src = format!("{PRELUDE}\nint main(void)\n{{\n{body}\n    return 0;\n}}\n");
    std::fs::write(&c, src).unwrap();
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fno-builtin", "-fPIE", "-I"]).arg(&rt);
    for f in ["fle_vfs.c", "fle_reg.c", "fle_env.c", "fle_imports.c", "fle_modules.c", "fle_string.c"] {
        cmd.arg(rt.join(f));
    }
    let cc = cmd.arg("-o").arg(&exe).arg(&c).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new(&exe).env("TMPDIR", dir.join("tmp")).output().expect("run failed");
    use std::os::unix::process::ExitStatusExt;
    Out {
        stdout: String::from_utf8_lossy(&r.stdout).to_string(),
        stderr: String::from_utf8_lossy(&r.stderr).to_string(),
        signal: r.status.signal(),
        ok: r.status.success(),
    }
}

/// Runs `body` and requires normal exit; returns the stdout lines.
pub fn lines(name: &str, body: &str) -> Vec<String> {
    let o = run(name, body);
    assert!(o.ok, "{name} did not exit normally (signal {:?}): {}\n{}", o.signal, o.stderr, o.stdout);
    o.stdout.lines().map(|s| s.to_string()).collect()
}

/// Each (label, C statements) must stop the run with SIGABRT, a "fle:" message, and no output after it.
/// `setup` is run first in every program (to create what the case needs).
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

/// UTF-16 C literal text for a Rust string, e.g. u"C:\\a" (backslashes doubled for C).
pub fn cw(s: &str) -> String {
    format!("W(\"{}\")", s.replace('\\', "\\\\"))
}

/// A `#define NAME 0x...ull` value read from a runtime header (to check the handle regions against each other).
pub fn header_const(file: &str, name: &str) -> u64 {
    let t = std::fs::read_to_string(runtime_dir().join(file)).unwrap();
    for l in t.lines() {
        let l = l.trim_start();
        if let Some(rest) = l.strip_prefix("#define ") {
            let mut it = rest.split_whitespace();
            if it.next() == Some(name) {
                let v = it.next().unwrap();
                let v = v.trim_start_matches("UINT64_C(").trim_end_matches(')');
                return u64::from_str_radix(v.trim_start_matches("0x"), 16).unwrap();
            }
        }
    }
    panic!("{name} not found in {file}");
}
