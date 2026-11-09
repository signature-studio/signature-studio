//! HLE entries (decision 0151, provisional, adopted as an opt-in by decision 0162; src/hle.rs, `GenOptions::hle`, `fe-lift --hle-entries`, runtime/fle_hle.h).
//!
//! An HLE entry names the first instruction of a lifted function and a host implementation `fle_hle_<name>()`; the generated C asks it first
//! and returns at once when it says it did everything. Synthetic PE (ImageBase 0x10000000, .text at RVA 0x1000, read + execute), written here:
//! - 0x1000 `main`: calls F1 and F2 (so both are function entries); never run
//! - 0x1020 F1: `mov eax,7; ret`
//! - 0x1040 F2: `mov eax,9; ret`
//!
//! Expected values come from what the instructions do and from the decision, not from the generated C: F1 returns 7 when the host did not take
//! the call, and leaves rax alone (the driver's 0xdeadbeef) when the host took it.
//!
//! 1. Generator: the guard is the only difference from the output without the entry (taking it out gives the old output byte for byte; no
//!    entry gives the old output byte for byte; no include line changes); a VA that is not a function entry is an error; the command line reads
//!    RVA and VA lines.
//! 2. Runtime, through a driver linked with every runtime source (fle_hle.c included) and the generated C: the host function decides.
//!
//! Outputs go under lift/out-hle/test.

mod etxt_common;

use fe_lift::hle::Entry;
use fe_lift::{generate, generate_full, GenOptions, SehOptions};
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

const IB: u64 = 0x1000_0000;
const LFANEW: usize = 0x80;
const OPT: usize = LFANEW + 24;
const SEC: usize = OPT + 240;
const TEXT_RVA: u32 = 0x1000;
const TEXT_RAW: usize = 0x400;
const FILE_LEN: usize = 0x800;
const SIZE_OF_IMAGE: u32 = 0x2000;

const F1: u32 = 0x1020;
const F2: u32 = 0x1040;

fn w16(b: &mut [u8], o: usize, v: u16) {
    b[o..o + 2].copy_from_slice(&v.to_le_bytes());
}
fn w32(b: &mut [u8], o: usize, v: u32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}
fn w64(b: &mut [u8], o: usize, v: u64) {
    b[o..o + 8].copy_from_slice(&v.to_le_bytes());
}
fn rel(next: u32, target: u32) -> [u8; 4] {
    ((target as i64 - next as i64) as i32).to_le_bytes()
}

fn build() -> Vec<u8> {
    let mut b = vec![0u8; FILE_LEN];
    w16(&mut b, 0, 0x5a4d);
    w32(&mut b, 0x3c, LFANEW as u32);
    w32(&mut b, LFANEW, 0x4550);
    let f = LFANEW + 4;
    w16(&mut b, f, 0x8664);
    w16(&mut b, f + 2, 1);
    w16(&mut b, f + 16, 240);
    w16(&mut b, f + 18, 0x2022);
    let o = OPT;
    w16(&mut b, o, 0x20b);
    w32(&mut b, o + 16, TEXT_RVA);
    w64(&mut b, o + 24, IB);
    w32(&mut b, o + 32, 0x400);
    w32(&mut b, o + 56, SIZE_OF_IMAGE);
    w32(&mut b, o + 60, 0x400);
    w16(&mut b, o + 68, 3);
    w32(&mut b, o + 108, 16);
    b[SEC..SEC + 5].copy_from_slice(b".text");
    w32(&mut b, SEC + 8, 0x400);
    w32(&mut b, SEC + 12, TEXT_RVA);
    w32(&mut b, SEC + 16, 0x400);
    w32(&mut b, SEC + 20, TEXT_RAW as u32);
    w32(&mut b, SEC + 36, 0x6000_0020);
    let call = |at: u32, to: u32| [&[0xe8][..], &rel(at + 5, to)].concat();
    let mut main = Vec::new();
    for (k, to) in [F1, F2].into_iter().enumerate() {
        main.extend(call(0x1000 + 5 * k as u32, to));
    }
    main.push(0xc3);
    let code: Vec<(u32, Vec<u8>)> = vec![
        (0x1000, main),
        (F1, vec![0xb8, 7, 0, 0, 0, 0xc3]), // mov eax,7; ret
        (F2, vec![0xb8, 9, 0, 0, 0, 0xc3]), // mov eax,9; ret
    ];
    for (rva, bytes) in code {
        let at = TEXT_RAW + (rva - TEXT_RVA) as usize;
        b[at..at + bytes.len()].copy_from_slice(&bytes);
    }
    b
}

fn entry(rva: u32, name: &str) -> Entry {
    Entry { va: IB + rva as u64, name: name.to_string() }
}

fn lift(img: &[u8], hle: &[Entry]) -> Result<(fe_lift::Stats, String), String> {
    let mut out = Vec::new();
    let gopt = GenOptions { hle: hle.to_vec(), ..Default::default() };
    let st = generate_full(img, &[], &[], &[], &[], &fe_decode::Options::default(), &SehOptions::default(), &gopt, &mut out)?;
    Ok((st, String::from_utf8(out).expect("utf8")))
}

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn rt_dir() -> PathBuf {
    manifest().join("../../runtime")
}
fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-hle/test");
    std::fs::create_dir_all(d.join("tmp")).expect("create test output dir");
    d
}

// ---------------------------------------------------------------------------------------------
// 1. Generator
// ---------------------------------------------------------------------------------------------

/// Written out here, not taken from src/hle.rs.
const GUARD1: &str = "{ extern int fle_hle_one(void); if (fle_hle_one()) return; } ";

#[test]
fn the_guard_is_the_only_change() {
    let (plain_st, plain) = lift(&build(), &[]).unwrap();
    let (st, c) = lift(&build(), &[entry(F1, "one")]).unwrap();
    assert_eq!(st.hle_entries, 1);
    assert_eq!(plain_st.hle_entries, 0);
    // the guard is inside the block of the first instruction, in front of the instruction's own text
    assert!(c.contains(&format!("L_10001020: {{ {GUARD1}/* 0x10001020 Mov */ ")), "{c}");
    assert_eq!(c.matches("fle_hle_").count(), 2);
    // nothing else changes: no include, no declaration
    assert_eq!(c.replace(GUARD1, ""), plain);
}

#[test]
fn without_an_entry_the_output_is_the_old_output() {
    let mut old = Vec::new();
    let st0 = generate(&build(), &mut old).unwrap();
    let (st, c) = lift(&build(), &[]).unwrap();
    assert_eq!(c.as_bytes(), &old[..]);
    assert_eq!(st.c_bytes, st0.c_bytes);
    assert!(!c.contains("fle_hle"), "{c}");
}

#[test]
fn an_entry_that_is_not_a_function_entry_is_an_error() {
    // the middle of F1, an address in no function, outside the image, and the second instruction of main
    for va in [F1 + 1, 0x1100, 0x5000, 0, 0x1005] {
        let e = lift(&build(), &[entry(va, "one")]).unwrap_err();
        assert!(e.contains("not a function entry of the lifted closure"), "{va:#x}: {e}");
    }
    // one function, two host implementations
    assert!(lift(&build(), &[entry(F1, "one"), entry(F1, "two")]).unwrap_err().contains("listed twice"));
}

#[test]
fn two_entries_in_two_functions_each_get_one_guard() {
    let (st, c) = lift(&build(), &[entry(F2, "two"), entry(F1, "one")]).unwrap();
    assert_eq!(st.hle_entries, 2);
    assert!(c.contains("L_10001020: { { extern int fle_hle_one(void); if (fle_hle_one()) return; } "));
    assert!(c.contains("L_10001040: { { extern int fle_hle_two(void); if (fle_hle_two()) return; } "));
}

fn cli(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_fe-lift")).args(args).output().expect("run fe-lift")
}

#[test]
fn the_command_line_reads_rva_and_va_lines_with_comments() {
    let dir = work_dir();
    let tag = etxt_common::uniq("hle-cli");
    let pe = dir.join(format!("{tag}.dll"));
    let list = dir.join(format!("{tag}-hle.txt"));
    let c = dir.join(format!("{tag}.c"));
    std::fs::write(&pe, build()).unwrap();
    std::fs::write(&list, "# the synthetic module\n\nrva:0x1020 one   # F1\n0x10001040 two\n").unwrap();
    let o = cli(&["lift", pe.to_str().unwrap(), c.to_str().unwrap(), "--hle-entries", list.to_str().unwrap()]);
    let out = String::from_utf8_lossy(&o.stdout).to_string();
    assert!(o.status.success(), "{out} {}", String::from_utf8_lossy(&o.stderr));
    assert!(out.contains("hle_entries            2"), "{out}");
    let text = std::fs::read_to_string(&c).unwrap();
    assert!(text.contains(GUARD1) && text.contains("fle_hle_two"), "{text}");
    // without the option: no statistics line, and the output is the library's output without an entry
    let c2 = dir.join(format!("{tag}-plain.c"));
    let o = cli(&["lift", pe.to_str().unwrap(), c2.to_str().unwrap()]);
    assert!(o.status.success());
    assert!(!String::from_utf8_lossy(&o.stdout).contains("hle_entries"));
    assert_eq!(std::fs::read_to_string(&c2).unwrap(), lift(&build(), &[]).unwrap().1);
}

#[test]
fn the_command_line_stops_on_a_bad_file_and_on_an_address_that_is_not_a_function_entry() {
    let dir = work_dir();
    let tag = etxt_common::uniq("hle-cli-bad");
    let pe = dir.join(format!("{tag}.dll"));
    let c = dir.join(format!("{tag}.c"));
    std::fs::write(&pe, build()).unwrap();
    for (name, text, want) in [
        ("syntax", Some("0x10001020 one\n0x10 a-b\n"), "line 2"),
        ("fields", Some("0x10001020\n"), "line 1"),
        ("entry", Some("0x10001021 one\n"), "not a function entry of the lifted closure"),
        ("missing", None, ""),
    ] {
        let list = dir.join(format!("{tag}-{name}.txt"));
        if let Some(t) = text {
            std::fs::write(&list, t).unwrap();
        }
        let o = cli(&["lift", pe.to_str().unwrap(), c.to_str().unwrap(), "--hle-entries", list.to_str().unwrap()]);
        assert!(!o.status.success(), "{name}");
        let e = String::from_utf8_lossy(&o.stderr).to_string();
        assert!(e.starts_with("error: ") && e.contains(want), "{name}: {e}");
    }
}

// ---------------------------------------------------------------------------------------------
// 2. Runtime
// ---------------------------------------------------------------------------------------------

fn driver_c() -> String {
    r####"#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include "fle_runtime.h"
#include "fle_dispatch.h"

void f_10001020(void);
void f_10001040(void);

#define U(x) ((unsigned long long)(x))
static uint64_t stk[16];
static int one_takes, one_calls, two_calls;

/* the host implementations named by the entries: one takes the call when asked to, two never does */
int fle_hle_one(void) { one_calls++; return one_takes; }
int fle_hle_two(void) { two_calls++; return 0; }

static void enter(void)
{
    int i;
    for (i = 0; i < 16; i++) stk[i] = UINT64_C(0xfeed0000) + i;
    R[4] = (uint64_t)(uintptr_t)&stk[12];
    R[0] = 0xdeadbeef;
}

int main(int argc, char **argv)
{
    const char *m = argc > 1 ? argv[1] : "";
    if (!strcmp(m, "declined")) {
        one_takes = 0; enter(); f_10001020(); printf("rax=%llx calls=%d\n", U(R[0]), one_calls);
    } else if (!strcmp(m, "taken")) {
        one_takes = 1; enter(); f_10001020(); printf("rax=%llx calls=%d rsp_same=%d\n", U(R[0]), one_calls, R[4] == (uint64_t)(uintptr_t)&stk[12]);
    } else if (!strcmp(m, "other-function")) {
        one_takes = 1; enter(); f_10001040(); printf("rax=%llx one=%d two=%d\n", U(R[0]), one_calls, two_calls);
    } else if (!strcmp(m, "each-call")) {
        one_takes = 1; enter(); f_10001020(); one_takes = 0; enter(); f_10001020(); one_takes = 1; enter(); f_10001020();
        printf("calls=%d last rax=%llx\n", one_calls, U(R[0]));
    } else {
        printf("no scenario %s\n", m);
        return 3;
    }
    return 0;
}
"####
    .to_string()
}

fn driver() -> &'static PathBuf {
    static D: OnceLock<PathBuf> = OnceLock::new();
    D.get_or_init(|| {
        let dir = work_dir();
        let tag = etxt_common::per_process("hle-driver");
        let (_, c) = lift(&build(), &[entry(F1, "one"), entry(F2, "two")]).unwrap();
        let gen_c = dir.join(format!("{tag}-gen.c"));
        let main_c = dir.join(format!("{tag}-main.c"));
        std::fs::write(&gen_c, &c).unwrap();
        std::fs::write(&main_c, driver_c()).unwrap();
        let rt = rt_dir();
        let mut srcs: Vec<PathBuf> = std::fs::read_dir(&rt)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| {
                let n = p.file_name().unwrap().to_string_lossy().to_string();
                n.starts_with("fle_") && n.ends_with(".c")
            })
            .collect();
        srcs.sort();
        assert!(srcs.iter().any(|p| p.ends_with("fle_hle.c")), "runtime/fle_hle.c is missing");
        let exe = dir.join(&tag);
        let mut cmd = Command::new("clang");
        cmd.args(["-O0", "-std=gnu11", "-w", "-fno-builtin", "-fPIE", "-I"]).arg(&rt);
        for p in &srcs {
            cmd.arg(p);
        }
        let cc = cmd.arg(&gen_c).arg(&main_c).arg("-lm").arg("-o").arg(&exe).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
        assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
        exe
    })
}

fn lines(scenario: &str) -> Vec<String> {
    let r = Command::new(driver()).arg(scenario).output().expect("run failed");
    assert!(r.status.signal().is_none() && r.status.code() == Some(0), "{scenario}: {:?} {}", r.status, String::from_utf8_lossy(&r.stderr));
    String::from_utf8_lossy(&r.stdout).lines().map(|s| s.to_string()).collect()
}

#[test]
fn a_host_that_declines_leaves_the_lifted_body_to_run() {
    // mov eax,7 sets rax = 7 (the 32-bit write clears the upper half)
    assert_eq!(lines("declined"), ["rax=7 calls=1"]);
}

#[test]
fn a_host_that_takes_the_call_makes_the_function_return_at_once() {
    // the body did not run (rax is the driver's value), the stack pointer is the one at the entry
    assert_eq!(lines("taken"), ["rax=deadbeef calls=1 rsp_same=1"]);
}

#[test]
fn the_guard_is_in_the_annotated_function_only_and_asks_at_every_call() {
    // F2 is annotated with a host that declines: its body runs (rax = 9); F1's host is not asked
    assert_eq!(lines("other-function"), ["rax=9 one=0 two=1"]);
    // taken, declined, taken: the last call is taken, so rax keeps the driver's value; the host was asked three times
    assert_eq!(lines("each-call"), ["calls=3 last rax=deadbeef"]);
}

#[test]
fn the_runtime_guard_of_the_dsp_function_is_silent_when_nothing_calls_it() {
    // the runtime's own guard (fle_hle_warp_bilin) is linked into this driver too, but no entry names it: it is never called, so with FLE_HLE=0, unset or `check`
    // the run prints no statistics line (the statistics are printed only after a guard was called)
    for env in [None, Some("0"), Some("check")] {
        let mut c = Command::new(driver());
        c.arg("declined");
        match env {
            Some(v) => c.env("FLE_HLE", v),
            None => c.env_remove("FLE_HLE"),
        };
        let r = c.output().unwrap();
        assert!(r.status.success());
        assert!(!String::from_utf8_lossy(&r.stderr).contains("fle_hle:"), "no statistics line without a call: {env:?}");
    }
}
