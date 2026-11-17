//! Call-site memory overrides (decision 0120; src/site_override.rs, `generate_with_overrides`, `fe-lift --site-overrides`, runtime `fle_site_store32`).
//!
//! A module may build a small structure on its stack with all zero, then call a function with a pointer to it; the environment wants one word of that
//! structure to be different (Blend2D's `Agg2D::attach`: `BLContextCreateInfo.flags` 0 -> 1, so that Blend2D uses the pipelines compiled into it and
//! does not generate machine code). The annotation names the instruction before which the word is stored. Synthetic PE (ImageBase 0x10000000, .text at
//! RVA 0x1000, read + execute), written here, not taken from the code under test:
//! - 0x1000 `main`: calls F1 and F2 (so both are function entries); never run
//! - 0x1020 F1 (the annotated function): `sub rsp,0x28; xor eax,eax; mov [rsp+0x20],eax; lea r8,[rsp+0x20]; call G; add rsp,0x28; ret`
//!   (the `call G` is the site: before it r8 points at a zero word; G stores the word it finds at [r8] into the global `OUT`)
//! - 0x1060 G: `mov eax,[r8]; ret`   (returns the word at [r8])
//! - 0x1080 F2 (not annotated, the same code as F1 with its own `call G`): used to show that only the annotated site is touched
//!
//! Expected values come from what the instructions do and from the decision, not from the generated C: F1 returns the word at [r8] when G runs, which is
//! 0 without the override and 1 with it.
//!
//! 1. Generator: the guard and the include are the only differences from the output without the override (taking them out gives the old output byte for
//!    byte); no override gives the old output byte for byte; a site that is not an instruction is an error; the command line reads RVA and VA lines.
//! 2. Runtime, through a driver linked with every runtime source and the generated C: F1 returns 1, F2 returns 0; a word that is neither the guest's value
//!    nor the new one stops by name; a word that already is the new value passes (a loop).
//!
//! Outputs go under lift/out-site/test.

mod etxt_common;

use fe_lift::site_override::{self, Site};
use fe_lift::{generate, generate_with_overrides};
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
const G: u32 = 0x1060;
const F2: u32 = 0x1080;
/// RVA of the `call G` inside F1: sub(4) xor(2) mov(4) lea(5) = 15 bytes after the start.
const SITE1: u32 = F1 + 15;
const SITE2: u32 = F2 + 15;

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

fn caller(at: u32) -> Vec<u8> {
    let mut v = vec![0x48, 0x83, 0xec, 0x28]; // sub rsp,0x28
    v.extend([0x31, 0xc0]); // xor eax,eax
    v.extend([0x89, 0x44, 0x24, 0x20]); // mov [rsp+0x20],eax
    v.extend([0x4c, 0x8d, 0x44, 0x24, 0x20]); // lea r8,[rsp+0x20]
    assert_eq!(v.len(), 15);
    v.push(0xe8);
    v.extend(rel(at + 15 + 5, G)); // call G
    v.extend([0x48, 0x83, 0xc4, 0x28, 0xc3]); // add rsp,0x28; ret
    v
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
    let code: Vec<(u32, Vec<u8>)> = vec![(0x1000, main), (F1, caller(F1)), (G, vec![0x41, 0x8b, 0x00, 0xc3]), (F2, caller(F2))];
    for (rva, bytes) in code {
        let at = TEXT_RAW + (rva - TEXT_RVA) as usize;
        b[at..at + bytes.len()].copy_from_slice(&bytes);
    }
    b
}

fn site(rva: u32, reg: usize, disp: i64, expect: u32, new: u32) -> Site {
    Site { va: IB + rva as u64, reg, disp, expect, new }
}

fn lift(img: &[u8], sites: &[Site]) -> Result<(fe_lift::Stats, String), String> {
    let mut out = Vec::new();
    let st = generate_with_overrides(img, &[], &[], &[], sites, &fe_decode::Options::default(), &mut out)?;
    Ok((st, String::from_utf8(out).expect("utf8")))
}

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn rt_dir() -> PathBuf {
    manifest().join("../../runtime")
}
fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-site/test");
    std::fs::create_dir_all(d.join("tmp")).expect("create test output dir");
    d
}

// ---------------------------------------------------------------------------------------------
// 1. Generator
// ---------------------------------------------------------------------------------------------

/// Written out here, not taken from src/site_override.rs: r8 is R[8]; the displacement 0 is `UINT64_C(0x0)`.
const GUARD1: &str = "{ fle_site_store32(UINT64_C(0x1000102f), R[8] + UINT64_C(0x0), UINT64_C(0x0), UINT64_C(0x1)); } ";

#[test]
fn the_guard_and_the_include_are_the_only_changes() {
    let (plain_st, plain) = lift(&build(), &[]).unwrap();
    let (st, c) = lift(&build(), &[site(SITE1, 8, 0, 0, 1)]).unwrap();
    assert_eq!(st.site_overrides, 1);
    assert_eq!(plain_st.site_overrides, 0);
    // the guard is inside the block of the instruction at the site, in front of the instruction's own text
    assert!(c.contains(&format!("L_1000102f: {{ {GUARD1}/* 0x1000102f Call */ ")), "{c}");
    assert_eq!(c.matches("fle_site_store32(").count(), 1);
    // the include follows the other three, before the globals
    assert!(c.contains("#include \"fle_dispatch.h\"\n#include \"fle_vmap.h\"\n#include \"fle_flags_nf.h\"\n\nuint64_t R[16];"), "{c}");
    // no function other than F1 changes
    let back = c.replace("#include \"fle_vmap.h\"\n", "").replace(GUARD1, "");
    assert_eq!(back, plain);
}

#[test]
fn without_a_site_the_output_is_the_old_output() {
    let mut old = Vec::new();
    let st0 = generate(&build(), &mut old).unwrap();
    let (st, c) = lift(&build(), &[]).unwrap();
    assert_eq!(c.as_bytes(), &old[..]);
    assert_eq!(st.site_overrides, 0);
    assert_eq!(st.c_bytes, st0.c_bytes);
    assert!(!c.contains("fle_site_store32") && !c.contains("fle_vmap.h"), "{c}");
}

#[test]
fn a_site_that_is_not_an_instruction_of_the_closure_is_an_error() {
    // the middle of an instruction, an address in no function, outside the image
    for va in [SITE1 + 1, 0x1100, 0x5000, 0] {
        let e = lift(&build(), &[site(va, 8, 0, 0, 1)]).unwrap_err();
        assert!(e.contains("not an instruction of the lifted closure"), "{va:#x}: {e}");
    }
    // the same override twice
    let s = site(SITE1, 8, 0, 0, 1);
    assert!(lift(&build(), &[s.clone(), s]).unwrap_err().contains("listed twice"));
}

#[test]
fn two_sites_in_two_functions_each_get_one_guard() {
    let (st, c) = lift(&build(), &[site(SITE2, 8, 0, 0, 1), site(SITE1, 8, 0, 0, 1)]).unwrap();
    assert_eq!(st.site_overrides, 2);
    assert_eq!(c.matches("fle_site_store32(").count(), 2);
    assert!(c.contains("fle_site_store32(UINT64_C(0x1000102f)") && c.contains("fle_site_store32(UINT64_C(0x1000108f)"));
}

fn cli(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_fe-lift")).args(args).output().expect("run fe-lift")
}

#[test]
fn the_command_line_reads_rva_and_va_lines_with_comments() {
    let dir = work_dir();
    let tag = etxt_common::uniq("site-cli");
    let pe = dir.join(format!("{tag}.dll"));
    let sites = dir.join(format!("{tag}-sites.txt"));
    let c = dir.join(format!("{tag}.c"));
    std::fs::write(&pe, build()).unwrap();
    std::fs::write(&sites, "# the synthetic module\n\nrva:0x102f store32 r8 0x0 0x1   # F1's call\n").unwrap();
    let o = cli(&["lift", pe.to_str().unwrap(), c.to_str().unwrap(), "--site-overrides", sites.to_str().unwrap()]);
    let out = String::from_utf8_lossy(&o.stdout).to_string();
    assert!(o.status.success(), "{out} {}", String::from_utf8_lossy(&o.stderr));
    assert!(out.contains("site_overrides         1"), "{out}");
    let text = std::fs::read_to_string(&c).unwrap();
    assert!(text.contains(GUARD1), "{text}");
    // the same site as a VA, with a displacement and the other register order: a different guard text
    std::fs::write(&sites, "0x1000102f store32 rsp+0x20 0 1\n").unwrap();
    let o = cli(&["lift", pe.to_str().unwrap(), c.to_str().unwrap(), "--site-overrides", sites.to_str().unwrap()]);
    assert!(o.status.success());
    assert!(std::fs::read_to_string(&c).unwrap().contains("fle_site_store32(UINT64_C(0x1000102f), R[4] + UINT64_C(0x20), UINT64_C(0x0), UINT64_C(0x1))"));
    // without the option: no statistics line, and the output is the library's output without a site
    let c2 = dir.join(format!("{tag}-plain.c"));
    let o = cli(&["lift", pe.to_str().unwrap(), c2.to_str().unwrap()]);
    assert!(o.status.success());
    assert!(!String::from_utf8_lossy(&o.stdout).contains("site_overrides"));
    assert_eq!(std::fs::read_to_string(&c2).unwrap(), lift(&build(), &[]).unwrap().1);
}

#[test]
fn the_command_line_stops_on_a_bad_file_and_on_a_site_that_is_not_an_instruction() {
    let dir = work_dir();
    let tag = etxt_common::uniq("site-cli-bad");
    let pe = dir.join(format!("{tag}.dll"));
    let c = dir.join(format!("{tag}.c"));
    std::fs::write(&pe, build()).unwrap();
    for (name, text, want) in [
        ("syntax", Some("0x1000102f store32 r8 0 1\n0x10 store32 eax 0 1\n"), "line 2"),
        ("entry", Some("0x10001030 store32 r8 0 1\n"), "not an instruction of the lifted closure"),
        ("same", Some("0x1000102f store32 r8 5 5\n"), "would change nothing"),
        ("missing", None, ""),
    ] {
        let sites = dir.join(format!("{tag}-{name}.txt"));
        if let Some(t) = text {
            std::fs::write(&sites, t).unwrap();
        }
        let o = cli(&["lift", pe.to_str().unwrap(), c.to_str().unwrap(), "--site-overrides", sites.to_str().unwrap()]);
        assert!(!o.status.success(), "{name}");
        let e = String::from_utf8_lossy(&o.stderr).to_string();
        assert!(e.starts_with("error: ") && e.contains(want), "{name}: {e}");
    }
}

// ---------------------------------------------------------------------------------------------
// 2. Runtime
// ---------------------------------------------------------------------------------------------

/// The driver header: a guest stack and the includes. The scenarios are in `driver_c`; /*PREFIX*/ holds the function declarations.
const DRIVER: &str = r####"#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include "fle_runtime.h"
#include "fle_dispatch.h"
#include "fle_vmap.h"
#include "fle_env.h"
/*PREFIX*/

#define U(x) ((unsigned long long)(x))
static uint64_t stk[16];

static void enter(void)
{
    int i;
    for (i = 0; i < 16; i++) stk[i] = UINT64_C(0xfeed0000) + i;
    R[4] = (uint64_t)(uintptr_t)&stk[12];
    R[0] = 0xdeadbeef;
}
"####;

fn driver_c() -> String {
    // F1 starts at 0x10001020 and F2 at 0x10001080; both end with `ret`, the word G read from [r8] is in rax.
    let main = r####"
    if (!strcmp(m, "f1")) {
        enter(); f_10001020(); printf("f1 rax=%llx\n", U(R[0]));
    } else if (!strcmp(m, "f2")) {
        enter(); f_10001080(); printf("f2 rax=%llx\n", U(R[0]));
    } else if (!strcmp(m, "f1-twice")) {
        enter(); f_10001020(); printf("first rax=%llx\n", U(R[0]));
        enter(); f_10001020(); printf("second rax=%llx\n", U(R[0]));
    } else if (!strcmp(m, "direct-expect")) {
        w = 0; fle_site_store32(0x1234, (uint64_t)(uintptr_t)&w, 0, 1); printf("w=%u\n", w);
    } else if (!strcmp(m, "direct-already")) {
        w = 1; fle_site_store32(0x1234, (uint64_t)(uintptr_t)&w, 0, 1); printf("w=%u\n", w);
    } else if (!strcmp(m, "direct-other")) {
        w = 7; fle_site_store32(0x1234, (uint64_t)(uintptr_t)&w, 0, 1); printf("returned w=%u\n", w);
    } else if (!strcmp(m, "direct-neighbours")) {
        uint32_t a[3] = {0x11111111, 0, 0x33333333};
        fle_site_store32(0x1234, (uint64_t)(uintptr_t)&a[1], 0, 0xdeadbeef);
        printf("%x %x %x\n", a[0], a[1], a[2]);
    } else if (!strcmp(m, "direct-high")) {
        w = 0xfffffffe; fle_site_store32(0x1234, (uint64_t)(uintptr_t)&w, 0xfffffffe, 0xffffffff); printf("w=%x\n", w);
    } else {
        printf("no scenario %s\n", m);
        return 3;
    }
    return 0;
}
"####;
    format!("{DRIVER}\nint main(int argc, char **argv)\n{{\n    const char *m = argc > 1 ? argv[1] : \"\";\n    uint32_t w;\n{main}")
}

fn driver() -> &'static PathBuf {
    static D: OnceLock<PathBuf> = OnceLock::new();
    D.get_or_init(|| {
        let dir = work_dir();
        let tag = etxt_common::per_process("site-driver");
        let (_, c) = lift(&build(), &[site(SITE1, 8, 0, 0, 1)]).unwrap();
        let gen_c = dir.join(format!("{tag}-gen.c"));
        let main_c = dir.join(format!("{tag}-main.c"));
        std::fs::write(&gen_c, &c).unwrap();
        let prefix = format!("void f_{:x}(void);\nvoid f_{:x}(void);\n", IB + F1 as u64, IB + F2 as u64);
        std::fs::write(&main_c, driver_c().replace("/*PREFIX*/", &prefix)).unwrap();
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

struct Out {
    stdout: String,
    stderr: String,
    signal: Option<i32>,
    code: Option<i32>,
}

fn run(scenario: &str) -> Out {
    let r = Command::new(driver()).arg(scenario).output().expect("run failed");
    Out {
        stdout: String::from_utf8_lossy(&r.stdout).to_string(),
        stderr: String::from_utf8_lossy(&r.stderr).to_string(),
        signal: r.status.signal(),
        code: r.status.code(),
    }
}

fn lines(scenario: &str) -> Vec<String> {
    let o = run(scenario);
    assert!(o.signal.is_none() && o.code == Some(0), "{scenario}: did not exit normally (signal {:?}, code {:?}): {}\n{}", o.signal, o.code, o.stderr, o.stdout);
    o.stdout.lines().map(|s| s.to_string()).collect()
}

#[test]
fn the_annotated_call_sees_the_new_word_and_the_other_function_the_old_one() {
    // G returns the word at [r8]: the guest wrote 0, the override makes it 1 just before F1's call; F2 has no override
    assert_eq!(lines("f1"), ["f1 rax=1"]);
    assert_eq!(lines("f2"), ["f2 rax=0"]);
}

#[test]
fn a_second_run_of_the_site_with_the_word_already_changed_passes() {
    // each run of F1 builds the word again (xor; mov), so the guest's value is there every time: the same answer twice
    assert_eq!(lines("f1-twice"), ["first rax=1", "second rax=1"]);
    // the word already holds the new value (a loop that reaches the instruction again): left as it is
    assert_eq!(lines("direct-already"), ["w=1"]);
    assert_eq!(lines("direct-expect"), ["w=1"]);
}

#[test]
fn only_four_bytes_change_and_the_full_32_bit_values_are_compared() {
    assert_eq!(lines("direct-neighbours"), ["11111111 deadbeef 33333333"]);
    assert_eq!(lines("direct-high"), ["w=ffffffff"]);
}

#[test]
fn a_word_that_is_neither_the_guests_value_nor_the_new_one_stops_by_name() {
    let o = run("direct-other");
    assert_eq!(o.signal, Some(6), "not SIGABRT (code {:?}): {}", o.code, o.stderr);
    for s in ["fle:", "site override", "site 0x1234", "is 0x7", "expects 0 ", "0x1 (already overridden)", "decision 0120"] {
        assert!(o.stderr.contains(s), "message lacks {s:?}: {}", o.stderr);
    }
    assert!(!o.stdout.contains("returned"), "returned instead of stopping");
}

#[test]
fn the_site_override_helpers_do_not_leak_into_the_hook_point_tables() {
    // a unit with only a site override has the include but no hook table and no constructor
    let (_, c) = lift(&build(), &[site(SITE1, 8, 0, 0, 1)]).unwrap();
    assert!(!c.contains("fle_hook_register") && !c.contains("constructor"), "{c}");
    let _ = site_override::REGS;
}
