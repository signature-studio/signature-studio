//! SRW locks and condition variables in runtime/fle_sys.c and the response table (decision 0092, provisional).
//!
//! 1. Generator: the seven rows exist, follow GetModuleHandleA, keep every pseudo-VA, and both the `jmp [IAT]` thunk and the
//!    `call [IAT]` form call `fle_imp_*` (Blend2D releases a lock by `jmp [IAT]` in three places and calls everything else).
//! 2. Runtime state machine: round trip, first use of an unknown address, shared count, the stops, the other thread, the table limit.
//! 3. Wake, WakeAll: nothing happens, R[0] is kept. SleepConditionVariableSRW stops for every input and says the two values.
//! 4. The guest's bytes: the lock and the condition variable live in a PROT_NONE page while the runtime is called, so a read or a
//!    write by the runtime would be a SIGSEGV; the source of the new code has no guest memory access.
//! 5. GetProcAddress finds the rows and `fle_dispatch` answers like the direct call.
//! 6. The shape of the locks in Blend2D (static locks, `this + 0x30`, tail-jump Release), lifted from synthetic bytes.
//!
//! The expected values come from the decision and from the Microsoft pages it quotes, written again here; nothing is compared with a
//! real Windows (the host is Linux). The cases the pages leave open are the provisional ones of decision 0092 (a)-(d).

mod etxt_common;

use fe_lift::generate;
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::OnceLock;

const IB: u64 = 0x1_4000_0000;
const LFANEW: usize = 0x80;
const OPT: usize = LFANEW + 24;
const SEC: usize = OPT + 240;
const TEXT_RVA: u32 = 0x1000;
const TEXT_RAW: usize = 0x400;
const DATA_RVA: u32 = 0x2000;
const DATA_RAW: usize = 0x800;
const FILE_LEN: usize = 0x1000;

const NAMES: [&str; 7] = [
    "AcquireSRWLockExclusive",
    "ReleaseSRWLockExclusive",
    "AcquireSRWLockShared",
    "ReleaseSRWLockShared",
    "WakeConditionVariable",
    "WakeAllConditionVariable",
    "SleepConditionVariableSRW",
];

fn w16(b: &mut [u8], o: usize, v: u16) {
    b[o..o + 2].copy_from_slice(&v.to_le_bytes());
}
fn w32(b: &mut [u8], o: usize, v: u32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}
fn w64(b: &mut [u8], o: usize, v: u64) {
    b[o..o + 8].copy_from_slice(&v.to_le_bytes());
}
fn data_at(rva: u32) -> usize {
    DATA_RAW + (rva - DATA_RVA) as usize
}
fn put(b: &mut [u8], rva: u32, bytes: &[u8]) {
    let o = data_at(rva);
    b[o..o + bytes.len()].copy_from_slice(bytes);
}
fn rel(next: u32, target: u32) -> [u8; 4] {
    ((target as i64 - next as i64) as i32).to_le_bytes()
}
fn slot(k: usize) -> u32 {
    0x2200 + 8 * k as u32
}

/// A PE that imports `names` from kernel32.dll (IAT slot k at 0x2200 + 8 k; at most 12) and holds `code` at the given RVAs (entry 0x1000).
/// The part of .data from 0x2700 is free for the test's own static data (the locks).
fn pe(names: &[&str], code: &[(u32, Vec<u8>)]) -> Vec<u8> {
    assert!(names.len() <= 12);
    let mut b = vec![0u8; FILE_LEN];
    w16(&mut b, 0, 0x5a4d);
    w32(&mut b, 0x3c, LFANEW as u32);
    w32(&mut b, LFANEW, 0x4550);
    let f = LFANEW + 4;
    w16(&mut b, f, 0x8664);
    w16(&mut b, f + 2, 2);
    w16(&mut b, f + 16, 240);
    w16(&mut b, f + 18, 0x2022);
    let o = OPT;
    w16(&mut b, o, 0x20b);
    w32(&mut b, o + 16, TEXT_RVA);
    w64(&mut b, o + 24, IB);
    w32(&mut b, o + 32, 0x1000);
    w32(&mut b, o + 36, 0x200);
    w32(&mut b, o + 56, 0x3000);
    w32(&mut b, o + 60, 0x400);
    w16(&mut b, o + 68, 3);
    w32(&mut b, o + 108, 16);
    w32(&mut b, o + 120, DATA_RVA);
    w32(&mut b, o + 124, 40);

    b[SEC..SEC + 5].copy_from_slice(b".text");
    w32(&mut b, SEC + 8, 0x400);
    w32(&mut b, SEC + 12, TEXT_RVA);
    w32(&mut b, SEC + 16, 0x400);
    w32(&mut b, SEC + 20, TEXT_RAW as u32);
    w32(&mut b, SEC + 36, 0x6000_0020);
    let s1 = SEC + 40;
    b[s1..s1 + 5].copy_from_slice(b".data");
    w32(&mut b, s1 + 8, 0x800);
    w32(&mut b, s1 + 12, DATA_RVA);
    w32(&mut b, s1 + 16, 0x800);
    w32(&mut b, s1 + 20, DATA_RAW as u32);
    w32(&mut b, s1 + 36, 0xc000_0040);

    let d = data_at(DATA_RVA);
    w32(&mut b, d, 0x2100);
    w32(&mut b, d + 12, 0x2300);
    w32(&mut b, d + 16, 0x2200);
    put(&mut b, 0x2300, b"kernel32.dll\0");
    for (k, n) in names.iter().enumerate() {
        let name_rva = 0x2400 + 0x40 * k as u32;
        let mut hn = vec![0u8, 0u8];
        hn.extend_from_slice(n.as_bytes());
        hn.push(0);
        put(&mut b, name_rva, &hn);
        w64(&mut b, data_at(0x2100 + 8 * k as u32), name_rva as u64);
        w64(&mut b, data_at(slot(k)), name_rva as u64);
    }
    for (rva, bytes) in code {
        let o = TEXT_RAW + (*rva - TEXT_RVA) as usize;
        b[o..o + bytes.len()].copy_from_slice(bytes);
    }
    b
}

#[derive(Clone, Copy, PartialEq)]
enum Form {
    /// entry: `call thunk_k` (5 bytes); thunk k at 0x1100 + 0x10 k is `jmp [slot k]`
    Jmp,
    /// entry: `call [slot k]` (6 bytes)
    Call,
}

/// An entry that reaches every name once, in the given form.
fn entry_calls(names: &[&str], form: Form) -> Vec<(u32, Vec<u8>)> {
    let mut code = Vec::new();
    let step = if form == Form::Jmp { 5 } else { 6 };
    for k in 0..names.len() {
        let at = 0x1000 + step * k as u32;
        match form {
            Form::Jmp => {
                let th = 0x1100 + 0x10 * k as u32;
                code.push((at, [&[0xe8][..], &rel(at + 5, th)].concat()));
                code.push((th, [&[0xff, 0x25][..], &rel(th + 6, slot(k))].concat()));
            }
            Form::Call => code.push((at, [&[0xff, 0x15][..], &rel(at + 6, slot(k))].concat())),
        }
    }
    code.push((0x1000 + step * names.len() as u32, vec![0xc3]));
    code
}

fn gen(img: &[u8]) -> (fe_lift::Stats, String) {
    let mut out = Vec::new();
    let st = generate(img, &mut out).unwrap();
    (st, String::from_utf8(out).unwrap())
}

fn rt() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../runtime")
}

fn work() -> PathBuf {
    let d = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../out-srw/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

/// The rows of `fle_resp_table` in the generated C, in order: (name, pseudo-VA).
fn resp_rows(c: &str) -> Vec<(String, u64)> {
    let start = c.find("const fle_resp_entry fle_resp_table[] = {").unwrap();
    let end = c[start..].find("};").unwrap() + start;
    c[start..end]
        .lines()
        .filter(|l| l.trim_start().starts_with("{ \""))
        .map(|l| {
            let name = l.split('"').nth(3).unwrap().to_string();
            let va = u64::from_str_radix(l.split("UINT64_C(0x").nth(1).unwrap().split(')').next().unwrap(), 16).unwrap();
            (name, va)
        })
        .collect()
}

/// Every runtime/fle_*.c compiled once per process (the response rows can name any `fle_imp_*`).
fn runtime_objs() -> &'static Vec<PathBuf> {
    static O: OnceLock<Vec<PathBuf>> = OnceLock::new();
    O.get_or_init(|| {
        let dir = work().join(etxt_common::per_process("rt"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut srcs: Vec<PathBuf> = std::fs::read_dir(rt())
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| {
                let n = p.file_name().unwrap().to_string_lossy().to_string();
                n.starts_with("fle_") && n.ends_with(".c")
            })
            .collect();
        srcs.sort();
        let mut v = Vec::new();
        for s in srcs {
            let o = dir.join(s.file_name().unwrap().to_string_lossy().replace(".c", ".o"));
            let cc = Command::new("clang")
                .args(["-O0", "-std=gnu11", "-w", "-fno-builtin", "-fPIE", "-c", "-I"])
                .arg(rt())
                .arg(&s)
                .arg("-o")
                .arg(&o)
                .env("TMPDIR", work().join("tmp"))
                .output()
                .expect("clang not found");
            assert!(cc.status.success(), "clang failed for {}: {}", s.display(), String::from_utf8_lossy(&cc.stderr));
            v.push(o);
        }
        v
    })
}

/// Compiles the generated C (`gen_c`) and a main with every runtime object; `no_pie` for a main that maps the image's data page.
fn link_gen(name: &str, gen_c: &str, main_c: &str, no_pie: bool) -> PathBuf {
    let dir = work();
    let tag = etxt_common::uniq(name);
    let g = dir.join(format!("{tag}.c"));
    let m = dir.join(format!("{tag}-main.c"));
    let exe = dir.join(&tag);
    std::fs::write(&g, gen_c).unwrap();
    std::fs::write(&m, main_c).unwrap();
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fno-builtin", "-fPIE"]);
    if no_pie {
        cmd.arg("-no-pie");
    }
    cmd.arg("-I").arg(rt());
    for o in runtime_objs() {
        cmd.arg(o);
    }
    let cc = cmd.arg(&g).arg(&m).arg("-lm").arg("-o").arg(&exe).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    exe
}

// ---- 1. Generator ----

fn all_names() -> Vec<&'static str> {
    NAMES.iter().copied().chain(["GetProcAddress"]).collect()
}

#[test]
fn the_jmp_iat_thunks_call_the_runtime_and_the_rows_follow_get_module_handle_a() {
    let names = all_names();
    let (st, c) = gen(&pe(&names, &entry_calls(&names, Form::Jmp)));
    for n in NAMES {
        assert_eq!(st.handled[format!("jmp_imp:{n}").as_str()], 1, "{n}");
        assert!(c.contains(&format!("fle_imp_{n}(); return;")), "{n}");
        assert!(c.contains(&format!("void fle_imp_{n}(void);")), "{n} is declared");
    }
    assert_eq!(st.unsupported_total, 0, "{:?}", st.unsupported);

    // Every pseudo-VA is base + 0x10 * (position), so rows appended later move none of these; the rows that existed keep theirs.
    let rows = resp_rows(&c);
    assert_eq!(rows.len(), st.resp_entries);
    for (i, (name, va)) in rows.iter().enumerate() {
        assert_eq!(*va, 0x7ff9_0000_0000u64 + 0x10 * (i as u64 + 1), "row {i} {name}");
    }
    let pos = |n: &str| rows.iter().position(|(name, _)| name == n).unwrap_or_else(|| panic!("no row {n}"));
    assert!(c.contains("{ \"kernel32.dll\", \"GetModuleHandleW\", UINT64_C(0x7ff9000000c0), fle_imp_GetModuleHandleW }"));
    assert!(c.contains("{ \"kernel32.dll\", \"GetProcAddress\", UINT64_C(0x7ff9000000d0), fle_imp_GetProcAddress }"));
    // GetModuleHandleA was the last row before decision 0092 (decision 0091); its pseudo-VA did not move.
    let a = pos("GetModuleHandleA");
    assert_eq!(rows[a].1, 0x7ff9_0000_0000 + 0x10 * 261, "GetModuleHandleA kept the pseudo-VA it had");
    // The seven rows come right after it, in this order, once each, with their own pseudo-VAs.
    for (i, n) in NAMES.iter().enumerate() {
        assert_eq!(rows[a + 1 + i].0, *n, "row {} after GetModuleHandleA", i + 1);
        let va = 0x7ff9_0000_0000u64 + 0x10 * (a as u64 + 2 + i as u64);
        assert_eq!(rows[a + 1 + i].1, va);
        assert!(c.contains(&format!("{{ \"kernel32.dll\", \"{n}\", UINT64_C({va:#x}), fle_imp_{n} }}")), "{n} in fle_resp_table");
        assert!(c.contains(&format!("    {{ UINT64_C({va:#x}), fle_imp_{n} }},\n")), "{n} in fle_dispatch_table");
        assert_eq!(c.matches(&format!("\"{n}\"")).count(), 1, "{n} is in the table once");
    }
    assert!(c.contains(&format!("const uint64_t fle_dispatch_count = {};", st.functions + st.resp_entries)));
}

#[test]
fn the_call_iat_form_calls_the_runtime_with_the_call_shape() {
    let names = all_names();
    let (st, c) = gen(&pe(&names, &entry_calls(&names, Form::Call)));
    for (k, n) in NAMES.iter().enumerate() {
        assert_eq!(st.handled[format!("call_imp:{n}").as_str()], 1, "{n}");
        assert!(!st.handled.contains_key(format!("jmp_imp:{n}").as_str()), "{n}: no thunk in this image");
        let next = IB + 0x1000 + 6 * (k as u64 + 1);
        let body = format!("{{ void fle_imp_{n}(void); R[4] -= 8; fle_st(R[4], 8, UINT64_C({next:#x})); fle_imp_{n}(); R[4] += 8; }}");
        assert!(c.contains(&body), "{n}: {body}");
    }
    assert_eq!(st.unsupported_total, 0, "{:?}", st.unsupported);
}

// ---- 2. Runtime ----

const PRE: &str = r#"#include <stdio.h>
#include <stddef.h>
#include <string.h>
#include <stdlib.h>
#include <sys/mman.h>
#include <pthread.h>
#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_sys.h"

uint64_t R[16];
#define CALL(f, a, b, c, d) (R[1] = (uint64_t)(a), R[2] = (uint64_t)(b), R[8] = (uint64_t)(c), R[9] = (uint64_t)(d), R[0] = 0x99, f(), R[0])
#define P(name, v) printf("%s %llu\n", name, (unsigned long long)(v))
#define AEX(l) CALL(fle_imp_AcquireSRWLockExclusive, l, 0, 0, 0)
#define REX(l) CALL(fle_imp_ReleaseSRWLockExclusive, l, 0, 0, 0)
#define ASH(l) CALL(fle_imp_AcquireSRWLockShared, l, 0, 0, 0)
#define RSH(l) CALL(fle_imp_ReleaseSRWLockShared, l, 0, 0, 0)
#define WAKE(c) CALL(fle_imp_WakeConditionVariable, c, 0, 0, 0)
#define WAKEALL(c) CALL(fle_imp_WakeAllConditionVariable, c, 0, 0, 0)
#define SLEEP(c, l, ms, fl) CALL(fle_imp_SleepConditionVariableSRW, c, l, ms, fl)

/* A page of 0xC3 bytes that nothing can read or write: the guest's lock and condition variable live here, so a read or a write by
 * the runtime is a SIGSEGV (signal 11), not the stop the tests expect (signal 6). */
static __attribute__((unused)) uint8_t *dead_page(void)
{
    uint8_t *p = mmap(NULL, 4096, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    if (p == MAP_FAILED) exit(2);
    memset(p, 0xC3, 4096);
    mprotect(p, 4096, PROT_NONE);
    return p;
}
"#;

fn clang(name: &str, src: &str) -> PathBuf {
    let dir = work();
    let uname = etxt_common::uniq(name);
    let c = dir.join(format!("{uname}.c"));
    let exe = dir.join(&uname);
    std::fs::write(&c, src).unwrap();
    let cc = Command::new("clang")
        .args(["-O0", "-std=gnu11", "-Wall", "-Wextra", "-Wno-unused-value", "-fPIE", "-pthread", "-I"])
        .arg(rt())
        .arg(rt().join("fle_sys.c"))
        .arg(rt().join("fle_imports.c"))
        .arg("-o")
        .arg(&exe)
        .arg(&c)
        .env("TMPDIR", dir.join("tmp"))
        .output()
        .expect("clang not found");
    assert!(cc.status.success(), "clang failed: {}\n{}", String::from_utf8_lossy(&cc.stderr), name);
    exe
}

fn run(exe: &PathBuf, args: &[&str]) -> Output {
    Command::new(exe).args(args).output().expect("run failed")
}

fn lines(o: &Output) -> Vec<String> {
    assert!(o.status.success(), "{}\n{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    String::from_utf8_lossy(&o.stdout).lines().map(|s| s.to_string()).collect()
}

/// Runs `cases` (label, C statements, texts the stop message must hold) as case 0..n of one program. Each must stop the run with abort
/// (signal 6), print a message that starts with "fle: " and holds every text, and not reach the line after the case.
fn expect_stops(name: &str, cases: &[(&str, &str, &[&str])]) {
    let mut s = format!("{PRE}int main(int argc, char **argv)\n{{\n    (void)argc;\n    switch (atoi(argv[1])) {{\n");
    for (i, (_, body, _)) in cases.iter().enumerate() {
        s.push_str(&format!("    case {i}: {{ {body} }} break;\n"));
    }
    s.push_str("    }\n    printf(\"returned\\n\");\n    return 0;\n}\n");
    let exe = clang(name, &s);
    for (i, (label, _, say)) in cases.iter().enumerate() {
        let o = run(&exe, &[&i.to_string()]);
        let err = String::from_utf8_lossy(&o.stderr).to_string();
        assert!(!String::from_utf8_lossy(&o.stdout).contains("returned"), "{label}: returned");
        assert_eq!(o.status.signal(), Some(6), "{label}: {err}");
        assert!(err.starts_with("fle: "), "{label}: {err}");
        for t in *say {
            assert!(err.contains(t), "{label}: `{t}` is not in `{err}`");
        }
    }
}

#[test]
fn an_exclusive_round_trip_and_the_first_use_of_an_unknown_address() {
    let src = format!(
        "{PRE}{}",
        r#"int main(void)
{
    uint8_t zero[8] = {0}, junk[8], ref0[8], refj[8];
    int i;
    memset(junk, 0xAB, 8);
    memcpy(refj, junk, 8);
    memcpy(ref0, zero, 8);
    for (i = 0; i < 3; i++) {
        AEX(zero); P("r0a", R[0]);
        REX(zero); P("r0r", R[0]);
        /* an address nobody registered is an unlocked lock whatever the guest's bytes are (the runtime does not read them) */
        AEX(junk);
        REX(junk);
    }
    P("zero_same", memcmp(zero, ref0, 8) == 0);
    P("junk_same", memcmp(junk, refj, 8) == 0);
    return 0;
}
"#
    );
    let exe = clang("srw-round", &src);
    // VOID: R[0] stays what the caller had (0x99 = 153).
    let mut want = Vec::new();
    for _ in 0..3 {
        want.push("r0a 153".to_string());
        want.push("r0r 153".to_string());
    }
    want.push("zero_same 1".to_string());
    want.push("junk_same 1".to_string());
    assert_eq!(lines(&run(&exe, &[])), want);
}

#[test]
fn shared_acquisitions_are_counted_and_released_one_by_one() {
    let src = format!(
        "{PRE}{}",
        r#"static uint8_t many[1000][8];
int main(void)
{
    uint8_t l[8], other[8];
    int i;
    /* three shared holders, released one by one; only after the third is the lock free for an exclusive acquisition */
    ASH(l); ASH(l); ASH(l);
    RSH(l); RSH(l); RSH(l);
    AEX(l); REX(l);
    /* the same lock again in the other order */
    AEX(l); REX(l);
    ASH(l); RSH(l);
    ASH(l); ASH(l); RSH(l); RSH(l);
    AEX(l); REX(l);
    /* two locks do not affect each other: one held exclusively while another is shared */
    AEX(l); ASH(other); ASH(other); RSH(other); RSH(other); REX(l);
    /* many shared holders on one lock */
    for (i = 0; i < 1000; i++) ASH(l);
    for (i = 0; i < 1000; i++) RSH(l);
    AEX(l); REX(l);
    /* many locks, each shared once */
    for (i = 0; i < 1000; i++) ASH(many[i]);
    for (i = 999; i >= 0; i--) RSH(many[i]);
    for (i = 0; i < 1000; i++) { AEX(many[i]); REX(many[i]); }
    P("done", 1);
    return 0;
}
"#
    );
    let exe = clang("srw-shared", &src);
    assert_eq!(lines(&run(&exe, &[])), ["done 1"]);
}

#[test]
fn acquire_stops_where_the_page_says_deadlock_or_where_no_one_could_release() {
    expect_stops(
        "srw-acquire-stops",
        &[
            ("excl-over-excl", "uint8_t l[8]; AEX(l); AEX(l);", &["fle: AcquireSRWLockExclusive on a lock", "already held exclusively", "cannot be acquired recursively (deadlock)"]),
            ("shared-over-excl", "uint8_t l[8]; AEX(l); ASH(l);", &["fle: AcquireSRWLockShared on a lock", "held exclusively", "no other thread can release it"]),
            ("excl-over-shared", "uint8_t l[8]; ASH(l); AEX(l);", &["fle: AcquireSRWLockExclusive on a lock", "held in shared mode (1 acquisitions)", "cannot upgrade to exclusive"]),
            ("excl-over-two-shared", "uint8_t l[8]; ASH(l); ASH(l); AEX(l);", &["fle: AcquireSRWLockExclusive on a lock", "held in shared mode (2 acquisitions)"]),
            ("excl-after-one-of-two-shared-released", "uint8_t l[8]; ASH(l); ASH(l); RSH(l); AEX(l);", &["held in shared mode (1 acquisitions)"]),
            ("excl-after-excl-released-and-taken-again", "uint8_t l[8]; AEX(l); REX(l); AEX(l); AEX(l);", &["already held exclusively"]),
            ("shared-after-shared-released-then-excl", "uint8_t l[8]; ASH(l); RSH(l); AEX(l); ASH(l);", &["fle: AcquireSRWLockShared on a lock", "held exclusively"]),
        ],
    );
}

#[test]
fn release_stops_when_the_lock_is_not_held_in_that_mode() {
    let nh = "that is not held (never acquired, or already released)";
    expect_stops(
        "srw-release-stops",
        &[
            ("rel-excl-unregistered", "uint8_t l[8]; REX(l);", &["fle: ReleaseSRWLockExclusive on a lock", nh, "the page does not give the result"]),
            ("rel-excl-empty-after-use", "uint8_t l[8]; AEX(l); REX(l); REX(l);", &["fle: ReleaseSRWLockExclusive on a lock", nh]),
            ("rel-excl-on-shared", "uint8_t l[8]; ASH(l); REX(l);", &["fle: ReleaseSRWLockExclusive on a lock", "held in shared mode (1 acquisitions)", "the mode does not match"]),
            ("rel-excl-on-two-shared", "uint8_t l[8]; ASH(l); ASH(l); REX(l);", &["held in shared mode (2 acquisitions)"]),
            ("rel-shared-unregistered", "uint8_t l[8]; RSH(l);", &["fle: ReleaseSRWLockShared on a lock", nh, "the page does not give the result"]),
            ("rel-shared-empty-after-use", "uint8_t l[8]; ASH(l); RSH(l); RSH(l);", &["fle: ReleaseSRWLockShared on a lock", nh]),
            ("rel-shared-more-than-acquired", "uint8_t l[8]; ASH(l); ASH(l); RSH(l); RSH(l); RSH(l);", &["fle: ReleaseSRWLockShared on a lock", nh]),
            ("rel-shared-on-excl", "uint8_t l[8]; AEX(l); RSH(l);", &["fle: ReleaseSRWLockShared on a lock", "held exclusively", "the mode does not match"]),
            ("rel-other-lock", "uint8_t l[8], m[8]; AEX(l); REX(m);", &["fle: ReleaseSRWLockExclusive on a lock", nh]),
        ],
    );
}

#[test]
fn a_null_pointer_stops_by_name() {
    expect_stops(
        "srw-null",
        &[
            ("acq-excl-null", "AEX(0);", &["fle: AcquireSRWLockExclusive(NULL) is not defined"]),
            ("rel-excl-null", "REX(0);", &["fle: ReleaseSRWLockExclusive(NULL) is not defined"]),
            ("acq-shared-null", "ASH(0);", &["fle: AcquireSRWLockShared(NULL) is not defined"]),
            ("rel-shared-null", "RSH(0);", &["fle: ReleaseSRWLockShared(NULL) is not defined"]),
            ("wake-null", "WAKE(0);", &["fle: WakeConditionVariable(NULL) is not defined"]),
            ("wakeall-null", "WAKEALL(0);", &["fle: WakeAllConditionVariable(NULL) is not defined"]),
        ],
    );
}

/// Another thread stops the run by the rule of decision 0017 (the same function, fle_sys_check_thread), for every one of the seven names.
#[test]
fn another_thread_stops_the_run_for_every_name() {
    let src = format!(
        "{PRE}{}",
        r#"static uint8_t l[8], cv[8];
static int which;
static void *other(void *a)
{
    (void)a;
    switch (which) {
    case 0: AEX(l); break;
    case 1: REX(l); break;
    case 2: ASH(l); break;
    case 3: RSH(l); break;
    case 4: WAKE(cv); break;
    case 5: WAKEALL(cv); break;
    case 6: SLEEP(cv, l, 100, 0); break;
    }
    return NULL;
}
int main(int argc, char **argv)
{
    pthread_t t;
    (void)argc;
    which = atoi(argv[1]);
    AEX(l); REX(l);   /* the first call is from this thread: it is the owner */
    pthread_create(&t, NULL, other, NULL);
    pthread_join(t, NULL);
    printf("returned\n");
    return 0;
}
"#
    );
    let exe = clang("srw-thread", &src);
    for (i, n) in NAMES.iter().enumerate() {
        let o = run(&exe, &[&i.to_string()]);
        let err = String::from_utf8_lossy(&o.stderr).to_string();
        assert_eq!(o.status.signal(), Some(6), "{n}: {err}");
        assert!(err.starts_with(&format!("fle: {n} called from tid")), "{n}: {err}");
        assert!(!String::from_utf8_lossy(&o.stdout).contains("returned"), "{n}");
    }
}

/// A lock table that holds 32,768 locks at once (half of its 65,536 slots, the limit of the critical-section table) and stops at the
/// next one; entries of released locks are reused, and releasing in another order does not lose a lock whose probe passed a released one.
#[test]
fn the_table_limit_stops_and_released_entries_are_reused() {
    let src = format!(
        "{PRE}{}",
        r#"static uint8_t many[32769][8];
int main(int argc, char **argv)
{
    int i, j;
    if (argc > 1) {
        /* one more than the limit */
        for (i = 0; i < 32769; i++) AEX(many[i]);
        printf("returned\n");
        return 0;
    }
    /* the limit itself, then everything released in a scrambled order (7919 is odd, so i -> i * 7919 mod 2^15 is a permutation) */
    for (i = 0; i < 32768; i++) AEX(many[i]);
    for (j = 0; j < 32768; j++) REX(many[(j * 7919) % 32768]);
    /* the table is empty again: all of them can be taken once more, now shared twice */
    for (i = 0; i < 32768; i++) ASH(many[i]);
    for (i = 0; i < 32768; i++) ASH(many[i]);
    for (i = 0; i < 32768; i++) RSH(many[i]);
    for (i = 0; i < 32768; i++) RSH(many[i]);
    /* released half-way: the half that is still held must still be found (a released entry in a probe run must not hide the rest),
     * and the released half must be free (a stop here would mean a released lock is still found) */
    for (i = 0; i < 20000; i++) AEX(many[i]);
    for (j = 0; j < 10000; j++) REX(many[(j * 7919) % 20000]);          /* 7919 is coprime to 20000 */
    for (j = 0; j < 10000; j++) AEX(many[(j * 7919) % 20000]);          /* free again */
    for (j = 10000; j < 20000; j++) REX(many[(j * 7919) % 20000]);      /* the other half is still held */
    for (j = 0; j < 10000; j++) REX(many[(j * 7919) % 20000]);
    P("done", 1);
    return 0;
}
"#
    );
    let exe = clang("srw-table", &src);
    assert_eq!(lines(&run(&exe, &[])), ["done 1"]);
    let o = run(&exe, &["over"]);
    let err = String::from_utf8_lossy(&o.stderr).to_string();
    assert_eq!(o.status.signal(), Some(6), "{err}");
    assert!(err.starts_with("fle: more than 32768 SRW locks held at once"), "{err}");
    assert!(!String::from_utf8_lossy(&o.stdout).contains("returned"));
}

/// Locks whose addresses have the same hash (a probe run of 8 entries): releasing one in the middle must not hide the ones behind it,
/// a released entry is reused, and releasing the last one of a run leaves the rest findable. The runtime does not dereference a lock
/// address, so the program uses plain numbers for them; the hash below is the table's (`fle_cs_hash`). If the hash were changed the
/// numbers would stop colliding and this test would pass without testing anything, so it checks that they collide at the start.
#[test]
fn released_entries_in_a_probe_run_do_not_hide_the_locks_behind_them() {
    let chain = r#"static uint64_t hsh(uint64_t a) { return (uint32_t)(((a >> 3) * UINT64_C(0x9E3779B97F4A7C15)) >> 48) & 0xFFFF; }
static uint64_t c[8];
static void find_chain(void)
{
    uint64_t a, t = hsh(0x100000);
    int n = 0;
    for (a = 0x100000; n < 8; a += 8) if (hsh(a) == t) c[n++] = a;
}
"#;
    let ok = format!(
        "{PRE}{chain}{}",
        r#"int main(void)
{
    int i;
    find_chain();
    for (i = 1; i < 8; i++) if (hsh(c[i]) != hsh(c[0])) { printf("no collision\n"); return 2; }
    /* 1: a run of 8; the middle ones released first, then the others */
    for (i = 0; i < 8; i++) AEX(c[i]);
    REX(c[1]); REX(c[2]); REX(c[5]); REX(c[6]);
    REX(c[0]); REX(c[3]); REX(c[4]); REX(c[7]);
    /* 2: the last one of a run released first (its entry and the released ones before it can be dropped), the others stay findable */
    for (i = 0; i < 4; i++) AEX(c[i]);
    REX(c[1]); REX(c[2]); REX(c[3]);
    REX(c[0]);
    /* 3: a released entry in the middle is reused, and the lock behind it is still found */
    for (i = 0; i < 4; i++) AEX(c[i]);
    REX(c[1]);
    AEX(c[1]);
    REX(c[3]); REX(c[1]); REX(c[0]); REX(c[2]);
    /* 4: shared holders in a run, released in another order */
    for (i = 0; i < 4; i++) { ASH(c[i]); ASH(c[i]); }
    RSH(c[0]); RSH(c[0]); RSH(c[1]);
    RSH(c[3]); RSH(c[3]); RSH(c[2]); RSH(c[2]); RSH(c[1]);
    P("done", 1);
    return 0;
}
"#
    );
    let exe = clang("srw-probe-run", &ok);
    assert_eq!(lines(&run(&exe, &[])), ["done 1"]);

    // A lock that was released and taken again is one entry, not two: a second Acquire of it stops; one that was released is free.
    let stops = [
        ("reacquired-twice", "for (i = 0; i < 4; i++) AEX(c[i]); REX(c[1]); AEX(c[1]); AEX(c[1]);", "already held exclusively"),
        ("released-is-free-but-the-run-behind-is-held", "for (i = 0; i < 4; i++) AEX(c[i]); REX(c[1]); AEX(c[3]);", "already held exclusively"),
        ("release-twice-in-a-run", "for (i = 0; i < 4; i++) AEX(c[i]); REX(c[1]); REX(c[1]);", "that is not held"),
    ];
    for (label, body, say) in stops {
        let src = format!("{PRE}{chain}int main(void)\n{{\n    int i;\n    find_chain();\n    {body}\n    printf(\"returned\\n\");\n    return 0;\n}}\n");
        let exe = clang(&format!("srw-probe-{label}"), &src);
        let o = run(&exe, &[]);
        let err = String::from_utf8_lossy(&o.stderr).to_string();
        assert_eq!(o.status.signal(), Some(6), "{label}: {err}");
        assert!(err.starts_with("fle: ") && err.contains(say), "{label}: {err}");
        assert!(!String::from_utf8_lossy(&o.stdout).contains("returned"), "{label}");
    }
}

// ---- 3. Wake, WakeAll, Sleep ----

#[test]
fn wake_and_wake_all_do_nothing_and_keep_r0() {
    let src = format!(
        "{PRE}{}",
        r#"int main(void)
{
    uint8_t l[8] = {1,2,3,4,5,6,7,8}, cv[8] = {9,10,11,12,13,14,15,16};
    uint8_t rl[8], rcv[8];
    memcpy(rl, l, 8);
    memcpy(rcv, cv, 8);
    /* no waiter ever exists (one thread): both return at once with R[0] as the caller had it (0x99 = 153), and keep no signal */
    P("wake", WAKE(cv));
    P("wakeall", WAKEALL(cv));
    P("wake2", WAKE(cv));
    P("wakeall2", WAKEALL(cv));
    /* a lock held around a Wake stays held and is released as usual: the Wake did not touch the lock state */
    AEX(l);
    P("wake_in_lock", WAKE(cv));
    P("wakeall_in_lock", WAKEALL(cv));
    REX(l);
    ASH(l);
    P("wake_in_shared", WAKE(cv));
    RSH(l);
    AEX(l); REX(l);
    P("l_same", memcmp(l, rl, 8) == 0);
    P("cv_same", memcmp(cv, rcv, 8) == 0);
    return 0;
}
"#
    );
    let exe = clang("srw-wake", &src);
    assert_eq!(
        lines(&run(&exe, &[])),
        ["wake 153", "wakeall 153", "wake2 153", "wakeall2 153", "wake_in_lock 153", "wakeall_in_lock 153", "wake_in_shared 153", "l_same 1", "cv_same 1"]
    );
}

#[test]
fn sleep_condition_variable_stops_for_every_input_and_says_the_two_values() {
    let held = "uint8_t *p = dead_page(); AEX(p);";
    let ex = |tail: &str| format!("{held} {tail}");
    let sh = |tail: &str| format!("uint8_t *p = dead_page(); ASH(p); {tail}");
    let pre = "fle: SleepConditionVariableSRW(ConditionVariable ";
    let cases: Vec<(&str, String, Vec<&str>)> = vec![
        ("infinite", ex("SLEEP(p + 16, p, 0xFFFFFFFFu, 0);"), vec![pre, "dwMilliseconds 4294967295, Flags 0): the lock is held exclusively; INFINITE never times out", "no other thread can call WakeConditionVariable or WakeAllConditionVariable"]),
        ("infinite-high-bits-ignored", ex("SLEEP(p + 16, p, 0x1FFFFFFFFull, 0x100000000ull);"), vec![pre, "dwMilliseconds 4294967295, Flags 0)", "INFINITE never times out"]),
        ("finite-100", ex("SLEEP(p + 16, p, 100, 0);"), vec![pre, "dwMilliseconds 100, Flags 0): the lock is held exclusively; a finite time-out needs the virtual clock", "decision 0050 C", "ERROR_TIMEOUT value (not on the page)"]),
        ("finite-1", ex("SLEEP(p + 16, p, 1, 0);"), vec![pre, "dwMilliseconds 1, Flags 0)", "a finite time-out needs the virtual clock"]),
        ("finite-max-minus-1", ex("SLEEP(p + 16, p, 0xFFFFFFFEu, 0);"), vec![pre, "dwMilliseconds 4294967294, Flags 0)", "a finite time-out needs the virtual clock"]),
        ("finite-high-bits-ignored", ex("SLEEP(p + 16, p, 0x100000064ull, 0);"), vec![pre, "dwMilliseconds 100, Flags 0)"]),
        ("zero", ex("SLEEP(p + 16, p, 0, 0);"), vec![pre, "dwMilliseconds 0, Flags 0): the lock is held exclusively; dwMilliseconds 0 returns at once with FALSE and ERROR_TIMEOUT", "whether the lock is held again on a time-out is not given"]),
        ("shared-flag-on-shared-lock", sh("SLEEP(p + 16, p, 5000, 1);"), vec![pre, "dwMilliseconds 5000, Flags 1): the lock is held in shared mode; a finite time-out"]),
        ("shared-flag-infinite", sh("SLEEP(p + 16, p, 0xFFFFFFFFu, 1);"), vec![pre, "dwMilliseconds 4294967295, Flags 1): the lock is held in shared mode; INFINITE"]),
        ("flags-not-interpreted", ex("SLEEP(p + 16, p, 7, 0x80000001u);"), vec![pre, "dwMilliseconds 7, Flags 2147483649)"]),
        ("exclusive-flag-on-shared-lock", sh("SLEEP(p + 16, p, 0, 0);"), vec![pre, "Flags 0): the lock is held in shared mode"]),
        ("lock-never-acquired", "uint8_t *p = dead_page(); SLEEP(p + 16, p, 0xFFFFFFFFu, 0);".to_string(), vec![pre, "dwMilliseconds 4294967295, Flags 0): the lock is not held (never acquired, or released)", "the page says the behavior is undefined"]),
        ("lock-released", "uint8_t *p = dead_page(); AEX(p); REX(p); SLEEP(p + 16, p, 100, 0);".to_string(), vec![pre, "dwMilliseconds 100, Flags 0): the lock is not held", "undefined"]),
        ("lock-null", "uint8_t *p = dead_page(); SLEEP(p + 16, 0, 100, 0);".to_string(), vec![pre, "SRWLock 0, dwMilliseconds 100, Flags 0): the lock is not held"]),
        ("cv-null-with-held-lock", ex("SLEEP(0, p, 100, 0);"), vec!["fle: SleepConditionVariableSRW(ConditionVariable 0, SRWLock ", "dwMilliseconds 100, Flags 0)"]),
        // a Wake before the Sleep leaves no signal: the Sleep stops as it does without it (the stop cannot tell the two apart, but it
        // shows that a Wake does not make a Sleep return)
        ("sleep-after-wakes", ex("WAKE(p + 16); WAKEALL(p + 16); SLEEP(p + 16, p, 0, 0);"), vec![pre, "dwMilliseconds 0, Flags 0)", "returns at once with FALSE and ERROR_TIMEOUT"]),
    ];
    let cases: Vec<(&str, &str, &[&str])> = cases.iter().map(|(l, b, s)| (*l, b.as_str(), s.as_slice())).collect();
    expect_stops("srw-sleep", &cases);
}

// ---- 4. The guest's bytes ----

/// The lock and the condition variable are in a page that cannot be read or written while the runtime is called. Every call of the seven
/// names that is not a stop returns, and the bytes are what the guest wrote after the page is readable again.
#[test]
fn the_runtime_never_reads_or_writes_the_guests_lock_or_condition_variable() {
    let src = format!(
        "{PRE}{}",
        r#"int main(void)
{
    uint8_t *pg = mmap(NULL, 4096, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    uint8_t ref[64];
    int i;
    if (pg == MAP_FAILED) return 2;
    for (i = 0; i < 64; i++) pg[i] = (uint8_t)(0x40 + i * 3);
    memcpy(ref, pg, 64);
    mprotect(pg, 4096, PROT_NONE);          /* from here a read or a write of these bytes by the runtime is a SIGSEGV */
    /* locks at pg + 0, +8, +32; condition variables at pg + 16, +24 */
    AEX(pg); REX(pg);
    ASH(pg + 8); ASH(pg + 8); RSH(pg + 8); RSH(pg + 8);
    AEX(pg + 32);
    P("wake", WAKE(pg + 16));
    P("wakeall", WAKEALL(pg + 16));
    P("wake2", WAKE(pg + 24));
    P("wakeall2", WAKEALL(pg + 24));
    REX(pg + 32);
    AEX(pg); ASH(pg + 8); REX(pg); RSH(pg + 8);
    mprotect(pg, 4096, PROT_READ);
    P("bytes_same", memcmp(pg, ref, 64) == 0);
    return 0;
}
"#
    );
    let exe = clang("srw-bytes", &src);
    assert_eq!(lines(&run(&exe, &[])), ["wake 153", "wakeall 153", "wake2 153", "wakeall2 153", "bytes_same 1"]);
}

/// The code that this decision added to fle_sys.c touches no guest memory (no fle_ld, fle_st, fle_addr, no memcpy, memset) and reads
/// nothing of the host (the words of the other source tests). The comments are cut first: they quote the pages.
#[test]
fn the_new_code_touches_no_guest_memory_and_reads_nothing_from_the_host() {
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
            } else {
                o.push(b[i]);
                i += 1;
            }
        }
        o
    }
    let raw = std::fs::read_to_string(rt().join("fle_sys.c")).unwrap();
    // from the `/*` that opens the section's comment to the `/*` that opens the next section's, so that every comment is cut whole
    let at = raw.find("/* ---- Slim reader/writer locks and condition variables").expect("the section of decision 0092");
    let end = at + raw[at..].find("/* ---- GetSystemInfo ----").expect("the next section");
    let code = strip_comments(&raw[at..end]);
    assert!(code.contains("fle_imp_SleepConditionVariableSRW") && code.contains("fle_imp_ReleaseSRWLockShared"), "the cut holds the code");
    for bad in [
        "fle_ld(", "fle_st(", "fle_addr", "fle_addr_opt", "memcpy", "memset", "memmove", "(void *)", "(uint8_t *)", "(uint64_t *)", "(uint32_t *)",
        "getenv(", "environ[", "getpid", "sysconf", "get_nprocs", "/proc", "/sys/", "cpuid", "clock_gettime", "fopen", "open(", "pthread", "nanosleep", "usleep(", "time(",
    ] {
        assert!(!code.contains(bad), "{bad}");
    }
    // The runtime's own table is the only state: the entry points read R[1] (the address as a key) and, for Sleep, R[2], R[8], R[9].
    for needle in ["fle_sys_check_thread(api)", "fle_sys_check_thread(\"SleepConditionVariableSRW\")"] {
        assert!(code.contains(needle), "{needle}");
    }
}

// ---- 5. GetProcAddress and fle_dispatch ----

#[test]
fn get_proc_address_finds_the_rows_and_dispatch_answers_like_the_direct_call() {
    let names = all_names();
    let (st, c) = gen(&pe(&names, &entry_calls(&names, Form::Jmp)));
    let rows = resp_rows(&c);
    assert_eq!(rows.len(), st.resp_entries);
    let main = r#"#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "fle_runtime.h"
#include "fle_dispatch.h"
#include "fle_modules.h"
#include "fle_imports.h"
#include "fle_sys.h"

static const char *names[7] = { "AcquireSRWLockExclusive", "ReleaseSRWLockExclusive", "AcquireSRWLockShared", "ReleaseSRWLockShared",
                                "WakeConditionVariable", "WakeAllConditionVariable", "SleepConditionVariableSRW" };
static uint8_t lk[8], cv[8];

#define D(i, a, b, c, d) (R[1] = (uint64_t)(a), R[2] = (uint64_t)(b), R[8] = (uint64_t)(c), R[9] = (uint64_t)(d), R[0] = 0x99, fle_dispatch(va[i]), R[0])

int main(int argc, char **argv)
{
    static const uint16_t k[] = { 'k','e','r','n','e','l','3','2', 0 };
    uint64_t k32, va[7];
    int i;
    R[1] = (uint64_t)(uintptr_t)k; fle_imp_GetModuleHandleW(); k32 = R[0];
    for (i = 0; i < 7; i++) {
        R[1] = k32; R[2] = (uint64_t)(uintptr_t)names[i]; fle_imp_GetProcAddress(); va[i] = R[0];
        printf("va %s %llx\n", names[i], (unsigned long long)va[i]);
    }
    /* the exact spelling only: another case is not found (NULL and ERROR_PROC_NOT_FOUND, 127), as for every other name */
    R[1] = k32; R[2] = (uint64_t)(uintptr_t)"acquiresrwlockexclusive"; fle_set_last_error(5); fle_imp_GetProcAddress();
    printf("lower %llx\n", (unsigned long long)R[0]);
    fle_imp_GetLastError(); printf("le %llu\n", (unsigned long long)R[0]);
    if (argc > 1) {
        D(0, lk, 0, 0, 0);
        D(6, cv, lk, 7, 0);   /* Sleep through the pseudo-VA: stops */
        printf("returned\n");
        return 0;
    }
    printf("acq_ex %llu\n", (unsigned long long)D(0, lk, 0, 0, 0));
    printf("rel_ex %llu\n", (unsigned long long)D(1, lk, 0, 0, 0));
    printf("acq_sh %llu\n", (unsigned long long)D(2, lk, 0, 0, 0));
    printf("acq_sh2 %llu\n", (unsigned long long)D(2, lk, 0, 0, 0));
    printf("rel_sh %llu\n", (unsigned long long)D(3, lk, 0, 0, 0));
    printf("rel_sh2 %llu\n", (unsigned long long)D(3, lk, 0, 0, 0));
    printf("wake %llu\n", (unsigned long long)D(4, cv, 0, 0, 0));
    printf("wakeall %llu\n", (unsigned long long)D(5, cv, 0, 0, 0));
    /* the table state is shared with the direct calls: taken through the pseudo-VA, released by the direct function */
    D(0, lk, 0, 0, 0);
    R[1] = (uint64_t)(uintptr_t)lk; fle_imp_ReleaseSRWLockExclusive();
    printf("returned\n");
    return 0;
}
"#;
    let exe = link_gen("srw-gpa", &c, main, false);
    let o = run(&exe, &[]);
    let got = lines(&o);
    let mut want: Vec<String> = Vec::new();
    for n in NAMES {
        let va = rows.iter().find(|(name, _)| name == n).unwrap().1;
        want.push(format!("va {n} {va:x}"));
    }
    want.push("lower 0".to_string());
    want.push("le 127".to_string());
    for l in ["acq_ex", "rel_ex", "acq_sh", "acq_sh2", "rel_sh", "rel_sh2", "wake", "wakeall"] {
        want.push(format!("{l} 153")); // VOID: R[0] as the caller had it
    }
    want.push("returned".to_string());
    assert_eq!(got, want);

    let o = run(&exe, &["sleep"]);
    let err = String::from_utf8_lossy(&o.stderr).to_string();
    assert_eq!(o.status.signal(), Some(6), "{err}");
    assert!(err.contains("fle: SleepConditionVariableSRW(ConditionVariable ") && err.contains("dwMilliseconds 7, Flags 0): the lock is held exclusively"), "{err}");
    assert!(!String::from_utf8_lossy(&o.stdout).contains("returned"));
}

// ---- 6. The shape of the locks in Blend2D ----

/// Three functions of the shape the DLL has, lifted and run:
///  - 0x1040 (like f_18019bec0): the static lock 0x2700; Acquire by `call [IAT]`, Release by `jmp [IAT]` (a tail jump, 3 sites in the DLL);
///  - 0x1080 (like f_18019caa0, the stop of decision 0091): `lea rbx,[this]; lea rcx,[rbx + 0x30]` of the static object 0x2740, Acquire and
///    Release by `call [IAT]`;
///  - 0x10c0 (like f_1800a9bb0): shared Acquire/Release, then an exclusive pair around Wake and WakeAll of a static condition variable.
/// The entry runs each of them twice, so a Release that did not happen would stop the second Acquire. The data page of the image is
/// mapped PROT_NONE during the run: the lifted code and the runtime never read or write the lock bytes (the DLL writes zero to a lock
/// once, in its `.xc` initializer; this test has no such write).
#[test]
fn the_static_lock_shapes_of_blend2d_run_and_never_touch_the_lock_bytes() {
    let names = ["AcquireSRWLockExclusive", "ReleaseSRWLockExclusive", "AcquireSRWLockShared", "ReleaseSRWLockShared", "WakeConditionVariable", "WakeAllConditionVariable"];
    let (acq, rel_, acq_s, rel_s, wake, wake_all) = (slot(0), slot(1), slot(2), slot(3), slot(4), slot(5));
    let lea_rcx = |at: u32, target: u32| (at, [&[0x48, 0x8d, 0x0d][..], &rel(at + 7, target)].concat());
    let call = |at: u32, s: u32| (at, [&[0xff, 0x15][..], &rel(at + 6, s)].concat());
    let jmp = |at: u32, s: u32| (at, [&[0xff, 0x25][..], &rel(at + 6, s)].concat());
    let (l1, obj, cv, l3) = (0x2700u32, 0x2740u32, 0x2780u32, 0x27a0u32);
    let mut code: Vec<(u32, Vec<u8>)> = Vec::new();
    // entry: f_a, f_a, f_b, f_b, f_c, f_c, ret
    for (k, f) in [0x1040u32, 0x1040, 0x1080, 0x1080, 0x10c0, 0x10c0].iter().enumerate() {
        let at = 0x1000 + 5 * k as u32;
        code.push((at, [&[0xe8][..], &rel(at + 5, *f)].concat()));
    }
    code.push((0x101e, vec![0xc3]));
    // f_a
    code.push((0x1040, vec![0x48, 0x83, 0xec, 0x28]));
    code.push(lea_rcx(0x1044, l1));
    code.push(call(0x104b, acq));
    code.push(lea_rcx(0x1051, l1));
    code.push((0x1058, vec![0x48, 0x83, 0xc4, 0x28]));
    code.push(jmp(0x105c, rel_));
    // f_b
    code.push((0x1080, vec![0x53, 0x48, 0x83, 0xec, 0x20]));
    code.push((0x1085, [&[0x48, 0x8d, 0x1d][..], &rel(0x108c, obj)].concat()));
    code.push((0x108c, vec![0x48, 0x8d, 0x4b, 0x30]));
    code.push(call(0x1090, acq));
    code.push((0x1096, vec![0x48, 0x8d, 0x4b, 0x30]));
    code.push(call(0x109a, rel_));
    code.push((0x10a0, vec![0x48, 0x83, 0xc4, 0x20, 0x5b, 0xc3]));
    // f_c
    code.push((0x10c0, vec![0x48, 0x83, 0xec, 0x28]));
    code.push(lea_rcx(0x10c4, l3));
    code.push(call(0x10cb, acq_s));
    code.push(lea_rcx(0x10d1, l3));
    code.push(call(0x10d8, rel_s));
    code.push(lea_rcx(0x10de, l3));
    code.push(call(0x10e5, acq));
    code.push(lea_rcx(0x10eb, cv));
    code.push(call(0x10f2, wake));
    code.push(lea_rcx(0x10f8, cv));
    code.push(call(0x10ff, wake_all));
    code.push(lea_rcx(0x1105, l3));
    code.push((0x110c, vec![0x48, 0x83, 0xc4, 0x28]));
    code.push(jmp(0x1110, rel_));

    let (st, c) = gen(&pe(&names, &code));
    // the Acquire sites are `call [IAT]`, the three tail-jump Releases (here 2: f_a and f_c) are `jmp [IAT]`
    assert_eq!(st.handled["call_imp:AcquireSRWLockExclusive"], 3, "{:?}", st.handled);
    assert_eq!(st.handled["call_imp:ReleaseSRWLockExclusive"], 1);
    assert_eq!(st.handled["jmp_imp:ReleaseSRWLockExclusive"], 2);
    assert_eq!(st.handled["call_imp:AcquireSRWLockShared"], 1);
    assert_eq!(st.handled["call_imp:ReleaseSRWLockShared"], 1);
    assert_eq!(st.handled["call_imp:WakeConditionVariable"], 1);
    assert_eq!(st.handled["call_imp:WakeAllConditionVariable"], 1);
    assert_eq!(st.unsupported_total, 0, "{:?}", st.unsupported);

    let main = r#"#include <stdio.h>
#include <string.h>
#include <sys/mman.h>
#include "fle_runtime.h"

extern void f_140001000(void);
static uint64_t stk[64];

int main(void)
{
    /* the loaded image has its data at 0x140002000; the lock bytes are at 0x140002700 (static lock), 0x140002770 (this + 0x30), 0x1400027a0 */
    uint8_t *p = mmap((void *)0x140002000, 0x1000, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED_NOREPLACE, -1, 0);
    uint8_t ref[0x100];
    int i;
    if (p != (void *)0x140002000) { printf("mmap failed\n"); return 2; }
    for (i = 0; i < 0x100; i++) p[0x700 + i] = (uint8_t)(0x11 + i);
    memcpy(ref, p + 0x700, 0x100);
    mprotect(p, 0x1000, PROT_NONE);
    stk[56] = 0xdead0000ull;
    R[4] = (uint64_t)(uintptr_t)&stk[56];
    R[3] = 0x1234;
    f_140001000();
    printf("rsp_kept %d\n", R[4] == (uint64_t)(uintptr_t)&stk[56]);
    printf("rbx_kept %llx\n", (unsigned long long)R[3]);
    printf("guest_ret_untouched %llx\n", (unsigned long long)stk[56]);
    /* and once more: no lock was left held */
    f_140001000();
    mprotect(p, 0x1000, PROT_READ);
    printf("bytes_same %d\n", memcmp(p + 0x700, ref, 0x100) == 0);
    printf("returned\n");
    return 0;
}
"#;
    let exe = link_gen("srw-shape", &c, main, true);
    let o = run(&exe, &[]);
    assert_eq!(
        lines(&o),
        ["rsp_kept 1", "rbx_kept 1234", "guest_ret_untouched dead0000", "bytes_same 1", "returned"],
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
}

/// The same entry, with the lock left held by a function that does not release it: the next Acquire of that lock stops by name.
#[test]
fn a_lock_a_guest_function_leaves_held_stops_the_next_acquire() {
    let names = ["AcquireSRWLockExclusive"];
    let lea_rcx = |at: u32, target: u32| (at, [&[0x48, 0x8d, 0x0d][..], &rel(at + 7, target)].concat());
    let call = |at: u32, s: u32| (at, [&[0xff, 0x15][..], &rel(at + 6, s)].concat());
    let mut code: Vec<(u32, Vec<u8>)> = vec![(0x1000, [&[0xe8][..], &rel(0x1005, 0x1040)].concat()), (0x1005, vec![0xc3])];
    code.push((0x1040, vec![0x48, 0x83, 0xec, 0x28]));
    code.push(lea_rcx(0x1044, 0x2700));
    code.push(call(0x104b, slot(0)));
    code.push((0x1051, vec![0x48, 0x83, 0xc4, 0x28, 0xc3]));
    let (_, c) = gen(&pe(&names, &code));
    let main = r#"#include <stdio.h>
#include "fle_runtime.h"
extern void f_140001000(void);
static uint64_t stk[64];
int main(void)
{
    stk[56] = 0xdead0000ull;
    R[4] = (uint64_t)(uintptr_t)&stk[56];
    f_140001000();
    printf("first returned\n"); fflush(stdout);
    f_140001000();
    printf("second returned\n");
    return 0;
}
"#;
    let exe = link_gen("srw-held", &c, main, true);
    let o = run(&exe, &[]);
    let err = String::from_utf8_lossy(&o.stderr).to_string();
    assert_eq!(o.status.signal(), Some(6), "{err}");
    assert_eq!(String::from_utf8_lossy(&o.stdout).lines().collect::<Vec<_>>(), ["first returned"]);
    assert!(err.starts_with("fle: AcquireSRWLockExclusive on a lock 0x140002700 that is already held exclusively"), "{err}");
}
