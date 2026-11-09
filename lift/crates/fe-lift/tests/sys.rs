//! Tests for the synchronisation, time, memory and system-information answers in runtime/fle_sys.c
//! (docs/decisions/0017-sync-time-memory.md; specification notes in docs/investigations/import-sys-spec.md).
//!
//! 1. Generator: the 15 rows are in the import table, the thunks call the runtime, and a generated unit
//!    links with fle_sys.c and runs.
//! 2. Virtual clock: the expected values come from a model written here from the rules of the decision
//!    (start value, Sleep adds ms, each read adds 10 ticks, 10 MHz, 32-bit wrap), not from the runtime.
//! 3. Critical sections: counts and re-entry follow the MSDN rules; every undefined case aborts.
//! 4. GetSystemInfo: offsets come from a struct mirror declared in the test program (natural alignment).
//! 5. VirtualAlloc, VirtualFree: committed memory is zero and writable, reserved memory faults, a decommit
//!    drops the data, the return values and error codes follow the MSDN pages; undefined cases abort.
//! 6. Process heap.
//!
//! The host is Linux: nothing is compared with a real Windows.

mod etxt_common;

use fe_lift::generate;
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::{Command, Output};

const IB: u64 = 0x1_4000_0000;
const LFANEW: usize = 0x80;
const OPT: usize = LFANEW + 24;
const SEC: usize = OPT + 240;
const TEXT_RVA: u32 = 0x1000;
const TEXT_RAW: usize = 0x400;
const DATA_RVA: u32 = 0x2000;
const DATA_RAW: usize = 0x800;
const FILE_LEN: usize = 0x1000;

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

/// A PE with kernel32.dll imports `names` (at most 16). Entry 0x1000 calls the thunks at 0x1100 + 0x10 * k,
/// each `jmp [IAT slot k]`.
fn build(names: &[&str]) -> Vec<u8> {
    assert!(names.len() <= 16);
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
        w64(&mut b, data_at(0x2200 + 8 * k as u32), name_rva as u64);
    }

    let t = TEXT_RAW;
    let mut code: Vec<(u32, Vec<u8>)> = Vec::new();
    for k in 0..names.len() {
        let at = 0x1000 + 5 * k as u32;
        let th = 0x1100 + 0x10 * k as u32;
        code.push((at, [&[0xe8][..], &rel(at + 5, th)].concat()));
        code.push((th, [&[0xff, 0x25][..], &rel(th + 6, 0x2200 + 8 * k as u32)].concat()));
    }
    code.push((0x1000 + 5 * names.len() as u32, vec![0xc3]));
    for (rva, bytes) in code {
        let o = t + (rva - TEXT_RVA) as usize;
        b[o..o + bytes.len()].copy_from_slice(&bytes);
    }
    b
}

fn dirs() -> (PathBuf, PathBuf) {
    let rt = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../runtime");
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../out-sys/test");
    std::fs::create_dir_all(dir.join("tmp")).unwrap();
    (rt, dir)
}

/// Compiles `src` with fle_sys.c and fle_imports.c (and `extra` runtime files).
fn clang(name: &str, src: &str) -> PathBuf {
    let (rt, dir) = dirs();
    let uname = etxt_common::uniq(name);
    let c = dir.join(format!("{uname}.c"));
    let exe = dir.join(&uname);
    std::fs::write(&c, src).unwrap();
    let cc = Command::new("clang")
        .args(["-O0", "-std=gnu11", "-Wall", "-Wextra", "-fPIE", "-pthread", "-I"])
        .arg(&rt)
        .arg(rt.join("fle_sys.c"))
        .arg(rt.join("fle_imports.c"))
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

const PRE: &str = r#"#include <stdio.h>
#include <stddef.h>
#include <string.h>
#include <stdlib.h>
#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_sys.h"

uint64_t R[16];
#define CALL(f, a, b, c, d) (R[1] = (uint64_t)(a), R[2] = (uint64_t)(b), R[8] = (uint64_t)(c), R[9] = (uint64_t)(d), R[0] = 0x99, f(), R[0])
static uint64_t le(void) { fle_imp_GetLastError(); return R[0]; }
#define P(name, v) printf("%s %llu\n", name, (unsigned long long)(v))
"#;

// ---- 1. Generator ----

const API_ROWS: [(&str, &str); 15] = [
    ("kernel32.dll", "Sleep"),
    ("kernel32.dll", "InitializeCriticalSection"),
    ("kernel32.dll", "EnterCriticalSection"),
    ("kernel32.dll", "LeaveCriticalSection"),
    ("kernel32.dll", "DeleteCriticalSection"),
    ("kernel32.dll", "GetTickCount"),
    ("winmm.dll", "timeGetTime"),
    ("kernel32.dll", "QueryPerformanceCounter"),
    ("kernel32.dll", "QueryPerformanceFrequency"),
    ("kernel32.dll", "GetSystemInfo"),
    ("kernel32.dll", "VirtualAlloc"),
    ("kernel32.dll", "VirtualFree"),
    ("kernel32.dll", "GetProcessHeap"),
    ("kernel32.dll", "HeapAlloc"),
    ("kernel32.dll", "HeapFree"),
];

#[test]
fn thunks_call_the_runtime() {
    let kernel: Vec<&str> = API_ROWS.iter().filter(|(d, _)| *d == "kernel32.dll").map(|(_, n)| *n).collect();
    assert_eq!(kernel.len(), 14);
    let img = build(&kernel);
    let mut out = Vec::new();
    let st = generate(&img, &mut out).unwrap();
    let c = String::from_utf8(out).unwrap();
    for name in &kernel {
        assert_eq!(st.handled[format!("jmp_imp:{name}").as_str()], 1, "{name}");
        assert!(c.contains(&format!("fle_imp_{name}(); return;")), "{name}");
    }
    let h = std::fs::read_to_string(dirs().0.join("fle_sys.h")).unwrap();
    for (_, name) in API_ROWS {
        assert!(h.contains(&format!("void fle_imp_{name}(void);")), "{name} is declared in fle_sys.h");
    }
}

#[test]
fn generated_unit_links_with_fle_sys_and_runs() {
    let img = build(&["Sleep", "GetTickCount", "GetProcessHeap"]);
    let mut out = Vec::new();
    generate(&img, &mut out).unwrap();
    let (rt, dir) = dirs();
    let tag = etxt_common::uniq("gen-sys");
    let gen = dir.join(format!("{tag}.c"));
    std::fs::write(&gen, &out).unwrap();
    // The entry f_140001000 calls the three thunks; ecx = 100 is the Sleep argument.
    let main = r#"#include <stdio.h>
#include "fle_runtime.h"
#include "fle_sys.h"
void f_140001000(void);
int main(void)
{
    static uint8_t stack[4096];
    uint64_t t0 = fle_sys_clock_ticks();
    R[4] = (uint64_t)(uintptr_t)stack + sizeof stack - 64;
    R[1] = 100;
    f_140001000();
    printf("%llu %llx\n", (unsigned long long)(fle_sys_clock_ticks() - t0), (unsigned long long)R[0]);
    return 0;
}
"#;
    let mainc = dir.join(format!("{tag}-main.c"));
    std::fs::write(&mainc, main).unwrap();
    let exe = dir.join(&tag);
    let cc = Command::new("clang")
        .args(["-O0", "-std=gnu11", "-w", "-fno-builtin", "-fPIE", "-I"])
        .arg(&rt)
        .arg(rt.join("fle_sys.c"))
        .arg(rt.join("fle_imports.c"))
        .arg(&gen)
        .arg(&mainc)
        .arg("-lm")
        .arg("-o")
        .arg(&exe)
        .env("TMPDIR", dir.join("tmp"))
        .output()
        .expect("clang not found");
    assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
    let l = lines(&run(&exe, &[]));
    // Sleep(100) adds 100 * 10000 ticks, GetTickCount's read adds 10; GetProcessHeap returns the token.
    assert_eq!(l, [format!("{} {:x}", 100 * 10_000 + 10, 0x7FF7_0000_0000u64)]);
}

/// With the real DLL: every row appears as a handled thunk (skipped when the DLL is absent).
#[test]
fn real_dll_handles_the_rows() {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../binaries/win64/FLEngine_x64.dll");
    let Ok(data) = std::fs::read(p) else { return };
    // The thunks are reached only from the 896-seed closure (lift/out-modules/seeds-896.txt, read only).
    let sp = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../out-modules/seeds-896.txt");
    let Ok(seeds_txt) = std::fs::read_to_string(sp) else { return };
    let seeds: Vec<u64> = seeds_txt.lines().filter(|l| !l.trim().is_empty()).map(|l| u64::from_str_radix(l.trim().trim_start_matches("0x"), 16).unwrap()).collect();
    assert_eq!(seeds.len(), 896);
    let mut out = Vec::new();
    let st = fe_lift::generate_with_seeds(&data, &seeds, &mut out).unwrap();
    // DeleteCriticalSection has no thunk in the closure (out-usage: 0 jmp, 0 call), so it is not counted.
    for (_, name) in API_ROWS.iter().filter(|(_, n)| *n != "DeleteCriticalSection") {
        assert!(st.handled.get(format!("jmp_imp:{name}").as_str()).copied().unwrap_or(0) >= 1, "{name}");
    }
    let c = String::from_utf8(out).unwrap();
    for (dll, name) in API_ROWS {
        assert!(c.contains(&format!("{{ \"{dll}\", \"{name}\", ")), "{name} in fle_resp_table");
    }
}

// ---- 2. Virtual clock ----

#[derive(Clone, Copy)]
enum Op {
    Qpc,
    Qpf,
    Tick,
    Time,
    Sleep(u32),
}

/// The rules of decision 0017, written again: ticks of 100 ns since a boot that is 3,600,000 ms old; a read
/// returns the clock then adds 10 ticks; Sleep adds ms * 10000; the frequency is 10,000,000.
fn model(ops: &[Op]) -> Vec<String> {
    let mut c: u64 = 3_600_000 * 10_000;
    let mut v = Vec::new();
    for (i, op) in ops.iter().enumerate() {
        match *op {
            Op::Qpc => {
                v.push(format!("{i} {c}"));
                c += 10;
            }
            Op::Qpf => v.push(format!("{i} 10000000")),
            Op::Tick | Op::Time => {
                v.push(format!("{i} {}", (c / 10_000) % (1u64 << 32)));
                c += 10;
            }
            Op::Sleep(ms) => {
                v.push(format!("{i} 153")); // VOID: R[0] keeps the caller's 0x99
                c += ms as u64 * 10_000;
            }
        }
    }
    v
}

fn clock_program(ops: &[Op]) -> String {
    let mut s = format!("{PRE}int main(void)\n{{\n    uint64_t b = 0;\n");
    for (i, op) in ops.iter().enumerate() {
        let line = match *op {
            Op::Qpc => format!("CALL(fle_imp_QueryPerformanceCounter, &b, 0, 0, 0); if (R[0] != 1) return 3; P(\"{i}\", b);"),
            Op::Qpf => format!("CALL(fle_imp_QueryPerformanceFrequency, &b, 0, 0, 0); if (R[0] != 1) return 3; P(\"{i}\", b);"),
            Op::Tick => format!("P(\"{i}\", (uint32_t)CALL(fle_imp_GetTickCount, 0, 0, 0, 0));"),
            Op::Time => format!("P(\"{i}\", (uint32_t)CALL(fle_imp_timeGetTime, 0, 0, 0, 0));"),
            Op::Sleep(ms) => format!("P(\"{i}\", CALL(fle_imp_Sleep, {ms}u, 0, 0, 0));"),
        };
        s.push_str(&format!("    {line}\n"));
    }
    s.push_str("    return 0;\n}\n");
    s
}

#[test]
fn clock_follows_the_model() {
    use Op::*;
    let ops = [
        Qpc, Qpc, Qpf, Tick, Sleep(1000), Tick, Qpc, Time, Sleep(0), Qpc, Sleep(250), Time, Tick, Qpc,
        // 49.71 days: wraps modulo 2^32 milliseconds.
        Sleep(0xFFFF_FFFE), Tick, Time, Qpc, Sleep(10), Tick,
    ];
    let exe = clang("clock-model", &clock_program(&ops));
    assert_eq!(lines(&run(&exe, &[])), model(&ops));
}

#[test]
fn clock_literals() {
    use Op::*;
    // Hand-computed: the first read is the start value, 3,600,000 ms = 36,000,000,000 ticks.
    let ops = [Qpc, Qpc, Tick, Sleep(5), Qpc];
    let exe = clang("clock-literals", &clock_program(&ops));
    assert_eq!(
        lines(&run(&exe, &[])),
        ["0 36000000000", "1 36000000010", "2 3600000", "3 153", "4 36000050030"]
    );
}

#[test]
fn counter_and_tick_count_agree() {
    // After any sequence of Sleep calls, QueryPerformanceCounter / QueryPerformanceFrequency (seconds)
    // equals GetTickCount (ms) / 1000, and timeGetTime equals GetTickCount to within the 10-tick reads.
    let src = format!(
        r#"{PRE}int main(void)
{{
    uint64_t q = 0, f = 0, i;
    static const unsigned ms[] = {{ 1, 17, 1000, 123456, 7 }};
    CALL(fle_imp_QueryPerformanceFrequency, &f, 0, 0, 0);
    for (i = 0; i < sizeof ms / sizeof ms[0]; i++) {{
        uint64_t t, w;
        CALL(fle_imp_Sleep, ms[i], 0, 0, 0);
        CALL(fle_imp_QueryPerformanceCounter, &q, 0, 0, 0);
        t = (uint32_t)CALL(fle_imp_GetTickCount, 0, 0, 0, 0);
        w = (uint32_t)CALL(fle_imp_timeGetTime, 0, 0, 0, 0);
        printf("%llu %llu %llu\n", (unsigned long long)(q * 1000 / f), (unsigned long long)t, (unsigned long long)w);
    }}
    return 0;
}}
"#
    );
    let exe = clang("clock-agree", &src);
    let mut total_ms = 0u64;
    let mut expected = Vec::new();
    for ms in [1u64, 17, 1000, 123456, 7] {
        total_ms += ms;
        let ms_now = 3_600_000 + total_ms;
        expected.push(format!("{ms_now} {ms_now} {ms_now}"));
    }
    assert_eq!(lines(&run(&exe, &[])), expected);
}

#[test]
fn clock_without_sleep_still_advances() {
    // A loop that waits for GetTickCount to change ends: each read adds 10 ticks (1 us), so 10,000 reads
    // add 1 ms.
    let src = format!(
        r#"{PRE}int main(void)
{{
    uint64_t first = CALL(fle_imp_GetTickCount, 0, 0, 0, 0), n = 1;
    while (CALL(fle_imp_GetTickCount, 0, 0, 0, 0) == first) n++;
    printf("%llu\n", (unsigned long long)n);
    return 0;
}}
"#
    );
    let exe = clang("clock-poll", &src);
    // Read k (k = 0, 1, ...) returns 36,000,000,000 + 10 k. The value reaches the next millisecond
    // (36,000,010,000) at k = 1000, so reads 0..=999 (1000 reads) see the first value.
    assert_eq!(lines(&run(&exe, &[])), ["1000"]);
}

// ---- Abort helpers ----

/// Runs `cases` (C statement lists) as case 0..n of one program; each must abort with SIGABRT, print a
/// message starting with "fle:", and not reach the line after the case.
fn expect_aborts(name: &str, cases: &[(&str, &str)]) {
    let mut s = format!("{PRE}int main(int argc, char **argv)\n{{\n    (void)argc;\n    switch (atoi(argv[1])) {{\n");
    for (i, (_, body)) in cases.iter().enumerate() {
        s.push_str(&format!("    case {i}: {{ {body} }} break;\n"));
    }
    s.push_str("    }\n    printf(\"returned\\n\");\n    return 0;\n}\n");
    let exe = clang(name, &s);
    for (i, (label, _)) in cases.iter().enumerate() {
        let o = run(&exe, &[&i.to_string()]);
        assert!(!String::from_utf8_lossy(&o.stdout).contains("returned"), "{label}: returned");
        assert!(String::from_utf8_lossy(&o.stderr).contains("fle:"), "{label}: {}", String::from_utf8_lossy(&o.stderr));
        assert_eq!(o.status.signal(), Some(6), "{label}");
    }
}

#[test]
fn sleep_infinite_and_null_pointers_abort() {
    expect_aborts(
        "abort-time",
        &[
            ("sleep-infinite", "CALL(fle_imp_Sleep, 0xFFFFFFFFu, 0, 0, 0);"),
            ("sleep-infinite-high-bits", "CALL(fle_imp_Sleep, 0x1FFFFFFFFull, 0, 0, 0);"),
            ("qpc-null", "CALL(fle_imp_QueryPerformanceCounter, 0, 0, 0, 0);"),
            ("qpf-null", "CALL(fle_imp_QueryPerformanceFrequency, 0, 0, 0, 0);"),
            ("systeminfo-null", "CALL(fle_imp_GetSystemInfo, 0, 0, 0, 0);"),
        ],
    );
}

#[test]
fn sleep_returns_nothing_and_a_finite_value_does_not_abort() {
    let src = format!(
        r#"{PRE}int main(void)
{{
    P("a", CALL(fle_imp_Sleep, 0, 0, 0, 0));
    P("b", CALL(fle_imp_Sleep, 0xFFFFFFFEu, 0, 0, 0));
    return 0;
}}
"#
    );
    let exe = clang("sleep-finite", &src);
    // VOID: R[0] is left as the caller had it (0x99 = 153).
    assert_eq!(lines(&run(&exe, &[])), ["a 153", "b 153"]);
}

// ---- 3. Critical sections ----

#[test]
fn critical_section_counts_and_reentry() {
    let src = format!(
        r#"{PRE}int main(void)
{{
    uint8_t cs[40], ref[40];
    int i;
    memset(cs, 0xAB, sizeof cs);
    memcpy(ref, cs, sizeof cs);
    CALL(fle_imp_InitializeCriticalSection, cs, 0, 0, 0);
    for (i = 0; i < 3; i++) CALL(fle_imp_EnterCriticalSection, cs, 0, 0, 0);
    for (i = 0; i < 3; i++) CALL(fle_imp_LeaveCriticalSection, cs, 0, 0, 0);
    /* Enter and Leave again after full release: the section is reusable. */
    CALL(fle_imp_EnterCriticalSection, cs, 0, 0, 0);
    CALL(fle_imp_LeaveCriticalSection, cs, 0, 0, 0);
    CALL(fle_imp_DeleteCriticalSection, cs, 0, 0, 0);
    P("object_untouched", memcmp(cs, ref, sizeof cs) == 0);
    P("r0_kept", R[0]);
    /* After Delete the object can be initialized again (MSDN: DeleteCriticalSection Remarks). */
    CALL(fle_imp_InitializeCriticalSection, cs, 0, 0, 0);
    CALL(fle_imp_EnterCriticalSection, cs, 0, 0, 0);
    CALL(fle_imp_LeaveCriticalSection, cs, 0, 0, 0);
    CALL(fle_imp_DeleteCriticalSection, cs, 0, 0, 0);
    /* Many live sections, at addresses that share low bits. */
    {{
        static uint8_t many[2000][64];
        for (i = 0; i < 2000; i++) CALL(fle_imp_InitializeCriticalSection, many[i], 0, 0, 0);
        for (i = 0; i < 2000; i++) CALL(fle_imp_EnterCriticalSection, many[i], 0, 0, 0);
        for (i = 1999; i >= 0; i--) CALL(fle_imp_LeaveCriticalSection, many[i], 0, 0, 0);
        for (i = 0; i < 2000; i++) CALL(fle_imp_DeleteCriticalSection, many[i], 0, 0, 0);
    }}
    P("done", 1);
    return 0;
}}
"#
    );
    let exe = clang("cs-ok", &src);
    // VOID functions: R[0] keeps the caller's 0x99 = 153.
    assert_eq!(lines(&run(&exe, &[])), ["object_untouched 1", "r0_kept 153", "done 1"]);
}

#[test]
fn critical_section_undefined_cases_abort() {
    let init = "uint8_t cs[40]; CALL(fle_imp_InitializeCriticalSection, cs, 0, 0, 0);";
    expect_aborts(
        "abort-cs",
        &[
            ("enter-uninitialized", "uint8_t cs[40]; CALL(fle_imp_EnterCriticalSection, cs, 0, 0, 0);"),
            ("leave-uninitialized", "uint8_t cs[40]; CALL(fle_imp_LeaveCriticalSection, cs, 0, 0, 0);"),
            ("delete-uninitialized", "uint8_t cs[40]; CALL(fle_imp_DeleteCriticalSection, cs, 0, 0, 0);"),
            ("initialize-twice", &format!("{init} CALL(fle_imp_InitializeCriticalSection, cs, 0, 0, 0);")),
            ("leave-without-enter", &format!("{init} CALL(fle_imp_LeaveCriticalSection, cs, 0, 0, 0);")),
            (
                "leave-more-than-enter",
                &format!("{init} CALL(fle_imp_EnterCriticalSection, cs, 0, 0, 0); CALL(fle_imp_EnterCriticalSection, cs, 0, 0, 0); CALL(fle_imp_LeaveCriticalSection, cs, 0, 0, 0); CALL(fle_imp_LeaveCriticalSection, cs, 0, 0, 0); CALL(fle_imp_LeaveCriticalSection, cs, 0, 0, 0);"),
            ),
            ("delete-while-owned", &format!("{init} CALL(fle_imp_EnterCriticalSection, cs, 0, 0, 0); CALL(fle_imp_DeleteCriticalSection, cs, 0, 0, 0);")),
            ("enter-after-delete", &format!("{init} CALL(fle_imp_DeleteCriticalSection, cs, 0, 0, 0); CALL(fle_imp_EnterCriticalSection, cs, 0, 0, 0);")),
            ("delete-twice", &format!("{init} CALL(fle_imp_DeleteCriticalSection, cs, 0, 0, 0); CALL(fle_imp_DeleteCriticalSection, cs, 0, 0, 0);")),
            ("initialize-null", "CALL(fle_imp_InitializeCriticalSection, 0, 0, 0, 0);"),
            ("enter-null", "CALL(fle_imp_EnterCriticalSection, 0, 0, 0, 0);"),
            ("leave-null", "CALL(fle_imp_LeaveCriticalSection, 0, 0, 0, 0);"),
            ("delete-null", "CALL(fle_imp_DeleteCriticalSection, 0, 0, 0, 0);"),
        ],
    );
}

#[test]
fn another_thread_stops_the_run() {
    // The section is "held by someone else" only if another thread exists; a call from a second thread
    // would have to wait. The runtime stops instead of waiting.
    let src = format!(
        r#"{PRE}#include <pthread.h>
static uint8_t cs[40];
static void *other(void *a) {{ (void)a; CALL(fle_imp_EnterCriticalSection, cs, 0, 0, 0); return NULL; }}
int main(void)
{{
    pthread_t t;
    CALL(fle_imp_InitializeCriticalSection, cs, 0, 0, 0);
    CALL(fle_imp_EnterCriticalSection, cs, 0, 0, 0);
    pthread_create(&t, NULL, other, NULL);
    pthread_join(t, NULL);
    printf("returned\n");
    return 0;
}}
"#
    );
    let exe = clang("cs-thread", &src);
    let o = run(&exe, &[]);
    assert_eq!(o.status.signal(), Some(6));
    assert!(String::from_utf8_lossy(&o.stderr).contains("fle: EnterCriticalSection called from tid"));
    assert!(!String::from_utf8_lossy(&o.stdout).contains("returned"));
}

// ---- 4. GetSystemInfo ----

#[test]
fn system_info_fields() {
    let src = format!(
        r#"{PRE}typedef struct {{
    union {{ uint32_t dwOemId; struct {{ uint16_t wProcessorArchitecture; uint16_t wReserved; }} s; }} u;
    uint32_t dwPageSize;
    void *lpMinimumApplicationAddress;
    void *lpMaximumApplicationAddress;
    uint64_t dwActiveProcessorMask;
    uint32_t dwNumberOfProcessors;
    uint32_t dwProcessorType;
    uint32_t dwAllocationGranularity;
    uint16_t wProcessorLevel;
    uint16_t wProcessorRevision;
}} SI;
int main(void)
{{
    SI si;
    memset(&si, 0xEE, sizeof si);
    CALL(fle_imp_GetSystemInfo, &si, 0, 0, 0);
    P("size", sizeof si);
    P("arch", si.u.s.wProcessorArchitecture);
    P("reserved", si.u.s.wReserved);
    P("page", si.dwPageSize);
    P("min_lt_max", (uint64_t)si.lpMinimumApplicationAddress < (uint64_t)si.lpMaximumApplicationAddress);
    P("min_aligned", ((uint64_t)si.lpMinimumApplicationAddress % si.dwAllocationGranularity) == 0);
    P("mask_bit0", si.dwActiveProcessorMask & 1);
    P("mask_bits", __builtin_popcountll(si.dwActiveProcessorMask) == (int)si.dwNumberOfProcessors);
    P("nproc", si.dwNumberOfProcessors);
    P("gran_multiple_of_page", si.dwAllocationGranularity % si.dwPageSize == 0);
    P("level", si.wProcessorLevel);
    P("revision", si.wProcessorRevision);
    P("type", si.dwProcessorType);
    P("page_is_host_page", si.dwPageSize == FLE_PAGE_SIZE);
    return 0;
}}
"#
    );
    let exe = clang("sysinfo", &src);
    // Architecture 9 = PROCESSOR_ARCHITECTURE_AMD64, processor type 8664 = PROCESSOR_AMD_X8664 (both on the
    // SYSTEM_INFO page). 48 bytes: the size of the struct on x64.
    assert_eq!(
        lines(&run(&exe, &[])),
        [
            "size 48", "arch 9", "reserved 0", "page 4096", "min_lt_max 1", "min_aligned 1", "mask_bit0 1", "mask_bits 1", "nproc 1",
            "gran_multiple_of_page 1", "level 0", "revision 0", "type 8664", "page_is_host_page 1",
        ]
    );
}

// ---- 5. VirtualAlloc, VirtualFree ----

const MEM_COMMIT: u32 = 0x1000;
const MEM_RESERVE: u32 = 0x2000;
const MEM_DECOMMIT: u32 = 0x4000;
const MEM_RELEASE: u32 = 0x8000;
const MEM_TOP_DOWN: u32 = 0x10_0000;
const PAGE_READWRITE: u32 = 0x04;

#[test]
fn virtual_alloc_top_down_is_accepted() {
    // Decision 0052. The guest's own call is (NULL, size, MEM_COMMIT | MEM_TOP_DOWN = 0x101000, PAGE_READWRITE) at
    // 0x40779b. The native build does not choose the place (the host does), so only the results the page states are
    // checked: a non-NULL granule-aligned base, zero committed memory, the region is recorded, and a reserve-only
    // call can be committed afterwards.
    let src = format!(
        r#"{PRE}static int zero(const uint8_t *p, size_t n) {{ size_t i; for (i = 0; i < n; i++) if (p[i]) return 0; return 1; }}
#define VA(a, s, t, p) CALL(fle_imp_VirtualAlloc, a, s, t, p)
#define VF(a, s, t) CALL(fle_imp_VirtualFree, a, s, t, 0)
int main(void)
{{
    uint64_t n0 = fle_sys_region_count(), a, r, x;
    uint8_t *p;

    a = VA(0, 0x13fff0, {td} | {c}, {rw});
    P("a_nonnull", a != 0);
    P("a_granule", a % 65536);
    P("a_zero", zero((uint8_t *)a, 0x140000));
    p = (uint8_t *)a; p[0x13ffff] = 7; p[0] = 9;
    P("a_rw", p[0x13ffff] == 7 && p[0] == 9);
    P("regions_1", fle_sys_region_count() - n0);
    P("a_release", VF(a, 0, {rel}));
    P("regions_0", fle_sys_region_count() - n0);

    r = VA(0, 0x40000, {td} | {res}, {rw});
    P("r_nonnull", r != 0);
    P("r_granule", r % 65536);
    x = VA(r + 0x1000, 0x2000, {c}, {rw});
    P("commit_later", x == r + 0x1000);
    p = (uint8_t *)r; p[0x2fff] = 5;
    P("commit_later_rw", p[0x2fff]);
    P("r_release", VF(r, 0, {rel}));

    r = VA(0, 0x3000, {td} | {c} | {res}, {rw});
    P("both_nonnull", r != 0);
    p = (uint8_t *)r; p[0x2fff] = 1;
    P("both_rw", p[0x2fff]);
    P("both_release", VF(r, 0, {rel}));
    P("regions_back", fle_sys_region_count() - n0);
    return 0;
}}
"#,
        td = MEM_TOP_DOWN,
        c = MEM_COMMIT,
        res = MEM_RESERVE,
        rel = MEM_RELEASE,
        rw = PAGE_READWRITE,
    );
    let exe = clang("va-top-down", &src);
    assert_eq!(
        lines(&run(&exe, &[])),
        [
            "a_nonnull 1", "a_granule 0", "a_zero 1", "a_rw 1", "regions_1 1", "a_release 1", "regions_0 0", "r_nonnull 1", "r_granule 0",
            "commit_later 1", "commit_later_rw 5", "r_release 1", "both_nonnull 1", "both_rw 1", "both_release 1", "regions_back 0",
        ]
    );
}

#[test]
fn virtual_alloc_results() {
    // The guest's own calls: (NULL, 0x13fff0, MEM_COMMIT, PAGE_READWRITE) at 0x40769f and 0x10000 at 0x408847.
    let src = format!(
        r#"{PRE}static int zero(const uint8_t *p, size_t n) {{ size_t i; for (i = 0; i < n; i++) if (p[i]) return 0; return 1; }}
#define VA(a, s, t, p) CALL(fle_imp_VirtualAlloc, a, s, t, p)
#define VF(a, s, t) CALL(fle_imp_VirtualFree, a, s, t, 0)
int main(void)
{{
    uint64_t n0 = fle_sys_region_count(), a, r, x, f;
    uint8_t *p;

    /* NULL address, MEM_COMMIT only, as the guest calls it. */
    a = VA(0, 0x13fff0, {c}, {rw});
    P("a_nonnull", a != 0);
    P("a_granule", a % 65536);
    P("a_zero", zero((uint8_t *)a, 0x140000));
    p = (uint8_t *)a; p[0x13ffff] = 7; p[0] = 9;
    P("a_rw", p[0x13ffff] == 7 && p[0] == 9);
    P("regions_1", fle_sys_region_count() - n0);
    P("a_release", VF(a, 0, {rel}));
    P("regions_0", fle_sys_region_count() - n0);

    /* Reserve 4 granules, then commit parts. */
    r = VA(0, 0x40000, {res}, {rw});
    P("r_granule", r % 65536);
    x = VA(r + 0x1800, 1, {c}, {rw});
    P("commit_rounds_down", x == r + 0x1000);
    p = (uint8_t *)r;
    p[0x1000] = 0xAA; p[0x1fff] = 0xBB;
    P("commit_again_keeps_data", VA(r + 0x1000, 0x1000, {c}, {rw}) == r + 0x1000 && p[0x1000] == 0xAA && p[0x1fff] == 0xBB);
    P("commit_two_pages", VA(r + 0x2000, 0x1001, {c}, {rw}) == r + 0x2000);
    p[0x3000] = 1;
    P("third_page_writable", p[0x3000]);
    P("decommit", VF(r + 0x1000, 0x1000, {dec}));
    P("decommit_uncommitted_ok", VF(r + 0x20000, 0x1000, {dec}));
    P("recommit_zero", VA(r + 0x1000, 0x1000, {c}, {rw}) == r + 0x1000 && zero(p + 0x1000, 0x1000));
    p[0x2000] = 5;
    f = VF(r, 0, {dec});
    P("decommit_all", f);
    P("decommit_all_zero", VA(r, 0x40000, {c}, {rw}) == r && zero(p, 0x40000));

    fle_set_last_error(0);
    P("commit_past_end", VA(r + 0x40000, 0x1000, {c}, {rw}));
    P("commit_past_end_err", le());
    fle_set_last_error(0);
    P("commit_spanning_end", VA(r + 0x3f000, 0x2000, {c}, {rw}));
    P("commit_spanning_end_err", le());
    fle_set_last_error(0);
    P("reserve_reserved", VA(r, 0x1000, {res}, {rw}));
    P("reserve_reserved_err", le());
    fle_set_last_error(0);
    P("commit_unreserved_address", VA(0x500000000000ull, 0x1000, {c}, {rw}));
    P("commit_unreserved_address_err", le());

    fle_set_last_error(0);
    P("release_not_base", VF(r + 0x1000, 0, {rel}));
    P("release_not_base_err", le());
    fle_set_last_error(0);
    P("release_with_size", VF(r, 0x1000, {rel}));
    P("release_with_size_err", le());
    P("release", VF(r, 0, {rel}));
    P("regions_back", fle_sys_region_count() - n0);
    fle_set_last_error(0);
    P("release_twice", VF(r, 0, {rel}));
    P("release_twice_err", le());

    /* A fixed address: the reserve address is rounded down to the allocation granularity. */
    x = VA(0x500000012345ull, 0x1000, {res}, {rw});
    P("fixed_reserve_base", x);
    P("fixed_reserve_overlap", VA(0x500000010000ull, 0x10000, {res}, {rw}));
    P("fixed_commit", VA(0x500000011000ull, 0x1000, {c}, {rw}) == 0x500000011000ull);
    p = (uint8_t *)0x500000011000ull; p[5] = 3;
    P("fixed_rw", p[5]);
    P("fixed_release", VF(x, 0, {rel}));
    x = VA(0x500000020000ull, 0x3000, {res} | {c}, {rw});
    P("fixed_reserve_commit", x);
    p = (uint8_t *)x; p[0x2fff] = 1;
    P("fixed_reserve_commit_rw", p[0x2fff]);
    P("fixed_release_2", VF(x, 0, {rel}));
    return 0;
}}
"#,
        c = MEM_COMMIT,
        res = MEM_RESERVE,
        dec = MEM_DECOMMIT,
        rel = MEM_RELEASE,
        rw = PAGE_READWRITE,
    );
    let exe = clang("va-results", &src);
    let l = lines(&run(&exe, &[]));
    let got: std::collections::HashMap<String, u64> = l
        .iter()
        .map(|s| {
            let (k, v) = s.split_once(' ').unwrap();
            (k.to_string(), v.parse().unwrap())
        })
        .collect();
    let want: Vec<(&str, u64)> = vec![
        ("a_nonnull", 1),
        ("a_granule", 0),
        ("a_zero", 1),
        ("a_rw", 1),
        ("regions_1", 1),
        ("a_release", 1),
        ("regions_0", 0),
        ("r_granule", 0),
        ("commit_rounds_down", 1),
        ("commit_again_keeps_data", 1),
        ("commit_two_pages", 1),
        ("third_page_writable", 1),
        ("decommit", 1),
        ("decommit_uncommitted_ok", 1),
        ("recommit_zero", 1),
        ("decommit_all", 1),
        ("decommit_all_zero", 1),
        ("commit_past_end", 0),
        ("commit_past_end_err", 487),
        ("commit_spanning_end", 0),
        ("commit_spanning_end_err", 487),
        ("reserve_reserved", 0),
        ("reserve_reserved_err", 487),
        ("commit_unreserved_address", 0),
        ("commit_unreserved_address_err", 487),
        ("release_not_base", 0),
        ("release_not_base_err", 87),
        ("release_with_size", 0),
        ("release_with_size_err", 87),
        ("release", 1),
        ("regions_back", 0),
        ("release_twice", 0),
        ("release_twice_err", 87),
        ("fixed_reserve_base", 0x5000_0001_0000),
        ("fixed_reserve_overlap", 0),
        ("fixed_commit", 1),
        ("fixed_rw", 3),
        ("fixed_release", 1),
        ("fixed_reserve_commit", 0x5000_0002_0000),
        ("fixed_reserve_commit_rw", 1),
        ("fixed_release_2", 1),
    ];
    assert_eq!(l.len(), want.len(), "{l:?}");
    for (k, v) in want {
        assert_eq!(got[k], v, "{k}");
    }
}

#[test]
fn virtual_alloc_protection_faults() {
    // Each program ends in a fault (SIGSEGV) at the marked access and prints "ok" before it.
    let cases: [(&str, String); 6] = [
        ("reserved-only", format!("r = VA(0, 0x10000, {MEM_RESERVE}, {PAGE_READWRITE}); puts(\"ok\"); fflush(stdout); *(volatile uint8_t *)r = 1;")),
        (
            "readonly",
            format!("r = VA(0, 0x1000, {}, 0x02); *(volatile uint8_t *)r; puts(\"ok\"); fflush(stdout); *(volatile uint8_t *)r = 1;", MEM_COMMIT),
        ),
        ("noaccess", format!("r = VA(0, 0x1000, {MEM_COMMIT}, 0x01); puts(\"ok\"); fflush(stdout); *(volatile uint8_t *)r;")),
        (
            "after-decommit",
            format!("r = VA(0, 0x1000, {MEM_COMMIT}, {PAGE_READWRITE}); *(volatile uint8_t *)r = 1; VF(r, 0x1000, {MEM_DECOMMIT}); puts(\"ok\"); fflush(stdout); *(volatile uint8_t *)r;"),
        ),
        (
            "after-release",
            format!("r = VA(0, 0x1000, {MEM_COMMIT}, {PAGE_READWRITE}); *(volatile uint8_t *)r = 1; VF(r, 0, {MEM_RELEASE}); puts(\"ok\"); fflush(stdout); *(volatile uint8_t *)r;"),
        ),
        (
            "uncommitted-page-in-reservation",
            format!("r = VA(0, 0x40000, {MEM_RESERVE}, {PAGE_READWRITE}); VA(r, 0x1000, {MEM_COMMIT}, {PAGE_READWRITE}); *(volatile uint8_t *)r = 1; puts(\"ok\"); fflush(stdout); *(volatile uint8_t *)(r + 0x1000) = 1;"),
        ),
    ];
    let mut s = format!(
        "{PRE}#define VA(a, s, t, p) CALL(fle_imp_VirtualAlloc, a, s, t, p)\n#define VF(a, s, t) CALL(fle_imp_VirtualFree, a, s, t, 0)\nint main(int argc, char **argv)\n{{\n    uint64_t r; (void)argc;\n    switch (atoi(argv[1])) {{\n"
    );
    for (i, (_, body)) in cases.iter().enumerate() {
        s.push_str(&format!("    case {i}: {{ {body} }} break;\n"));
    }
    s.push_str("    }\n    printf(\"no fault\\n\");\n    return 0;\n}\n");
    let exe = clang("va-faults", &s);
    for (i, (label, _)) in cases.iter().enumerate() {
        let o = run(&exe, &[&i.to_string()]);
        assert_eq!(String::from_utf8_lossy(&o.stdout), "ok\n", "{label}");
        assert_eq!(o.status.signal(), Some(11), "{label}");
    }
}

#[test]
fn virtual_alloc_undefined_cases_abort() {
    let va = |t: String, p: String, s: &str| format!("CALL(fle_imp_VirtualAlloc, 0, {s}, {t}, {p});");
    let rw = PAGE_READWRITE.to_string();
    let c = MEM_COMMIT.to_string();
    let vf = |a: &str, s: &str, t: u32| format!("CALL(fle_imp_VirtualFree, {a}, {s}, {t}, 0);");
    expect_aborts(
        "abort-va",
        &[
            ("type-0", &va("0".into(), rw.clone(), "0x1000")),
            ("mem-reset", &va("0x80000".into(), rw.clone(), "0x1000")),
            ("mem-reset-undo", &va("0x1000000".into(), rw.clone(), "0x1000")),
            ("mem-large-pages", &va(format!("0x20000000 | {c} | {MEM_RESERVE}"), rw.clone(), "0x1000")),
            ("mem-physical", &va(format!("0x400000 | {MEM_RESERVE}"), rw.clone(), "0x1000")),
            ("mem-top-down-alone", &va(format!("{MEM_TOP_DOWN}"), rw.clone(), "0x1000")),
            ("mem-top-down-with-address", &format!("CALL(fle_imp_VirtualAlloc, 0x10000000, 0x1000, {MEM_TOP_DOWN} | {c}, {rw});")),
            ("mem-top-down-with-write-watch", &va(format!("{MEM_TOP_DOWN} | 0x200000 | {MEM_RESERVE}"), rw.clone(), "0x1000")),
            ("mem-top-down-with-reset", &va(format!("{MEM_TOP_DOWN} | 0x80000"), rw.clone(), "0x1000")),
            ("mem-top-down-unknown-bit", &va(format!("{MEM_TOP_DOWN} | {c} | 0x10000000"), rw.clone(), "0x1000")),
            ("mem-write-watch", &va(format!("0x200000 | {MEM_RESERVE}"), rw.clone(), "0x1000")),
            ("size-0", &va(c.clone(), rw.clone(), "0")),
            ("size-huge", &va(c.clone(), rw.clone(), "0x7fffffffffffffff")),
            ("protect-0", &va(c.clone(), "0".into(), "0x1000")),
            ("protect-writecopy", &va(c.clone(), "0x08".into(), "0x1000")),
            ("protect-execute-writecopy", &va(c.clone(), "0x80".into(), "0x1000")),
            ("protect-guard", &va(c.clone(), "0x104".into(), "0x1000")),
            ("protect-nocache", &va(c.clone(), "0x204".into(), "0x1000")),
            ("protect-two-values", &va(c.clone(), "0x06".into(), "0x1000")),
            ("free-type-0", &vf("0", "0", 0)),
            ("free-decommit-and-release", &vf("0", "0", MEM_DECOMMIT | MEM_RELEASE)),
            ("free-release-coalesce", &vf("0", "0", MEM_RELEASE | 1)),
            ("free-release-preserve-placeholder", &vf("0", "0", MEM_RELEASE | 2)),
            ("free-decommit-outside", &vf("0x123450000", "0x1000", MEM_DECOMMIT)),
            (
                "free-decommit-size-0-not-base",
                &format!("{{ uint64_t r = CALL(fle_imp_VirtualAlloc, 0, 0x10000, {c}, {rw}); CALL(fle_imp_VirtualFree, r + 0x1000, 0, {MEM_DECOMMIT}, 0); }}"),
            ),
            (
                "free-decommit-past-region",
                &format!("{{ uint64_t r = CALL(fle_imp_VirtualAlloc, 0, 0x10000, {c}, {rw}); CALL(fle_imp_VirtualFree, r + 0xf000, 0x2000, {MEM_DECOMMIT}, 0); }}"),
            ),
        ],
    );
}

// ---- 6. Process heap ----

#[test]
fn process_heap() {
    let src = format!(
        r#"{PRE}int main(void)
{{
    uint64_t h = CALL(fle_imp_GetProcessHeap, 0, 0, 0, 0), a, b, i, ok = 1;
    uint8_t *p;
    P("heap_is_token", h == FLE_PROCESS_HEAP);
    P("heap_stable", CALL(fle_imp_GetProcessHeap, 0, 0, 0, 0) == h);
    a = CALL(fle_imp_HeapAlloc, h, 0, 100, 0);
    P("alloc_nonnull", a != 0);
    P("alloc_16_aligned", a % 16);
    p = (uint8_t *)a; memset(p, 0xFF, 100);
    P("free_ok", CALL(fle_imp_HeapFree, h, 0, a, 0));
    b = CALL(fle_imp_HeapAlloc, h, 0x8, 100, 0);
    p = (uint8_t *)b;
    for (i = 0; i < 100; i++) if (p[i]) ok = 0;
    P("zero_memory", ok);
    P("free_ok_2", CALL(fle_imp_HeapFree, h, 1, b, 0));
    P("free_null", CALL(fle_imp_HeapFree, h, 0, 0, 0));
    a = CALL(fle_imp_HeapAlloc, h, 0x1 | 0x8, 0, 0);
    P("alloc_zero_bytes_nonnull", a != 0);
    CALL(fle_imp_HeapFree, h, 0, a, 0);
    /* A LocalAlloc block can be freed by HeapFree: both are host malloc blocks. */
    a = CALL(fle_imp_LocalAlloc, 0x40, 32, 0, 0);
    P("local_block_free", CALL(fle_imp_HeapFree, h, 0, a, 0));
    P("big", CALL(fle_imp_HeapAlloc, h, 0x4, 1u << 20, 0) != 0);
    return 0;
}}
"#
    );
    let exe = clang("heap", &src);
    assert_eq!(
        lines(&run(&exe, &[])),
        [
            "heap_is_token 1", "heap_stable 1", "alloc_nonnull 1", "alloc_16_aligned 0", "free_ok 1", "zero_memory 1", "free_ok_2 1",
            "free_null 1", "alloc_zero_bytes_nonnull 1", "local_block_free 1", "big 1",
        ]
    );
}

#[test]
fn process_heap_undefined_cases_abort() {
    expect_aborts(
        "abort-heap",
        &[
            ("alloc-unknown-heap", "CALL(fle_imp_HeapAlloc, 0x1234, 0, 16, 0);"),
            ("alloc-null-heap", "CALL(fle_imp_HeapAlloc, 0, 0, 16, 0);"),
            ("free-unknown-heap", "CALL(fle_imp_HeapFree, 0x1234, 0, 0, 0);"),
            ("alloc-unknown-flag", "CALL(fle_imp_HeapAlloc, FLE_PROCESS_HEAP, 0x10, 16, 0);"),
            ("free-unknown-flag", "CALL(fle_imp_HeapFree, FLE_PROCESS_HEAP, 0x8, 0, 0);"),
            ("alloc-fails-with-exceptions-flag", "CALL(fle_imp_HeapAlloc, FLE_PROCESS_HEAP, 0x4, 0x7fffffffffffffffull, 0);"),
        ],
    );
}
