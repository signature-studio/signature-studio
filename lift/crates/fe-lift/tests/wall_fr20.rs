//! Tests of decision 0147 (docs/decisions/0147-virtual-clock-and-concurrency.md, PROVISIONAL): the deterministic pseudo-preemption of the cooperative guest threads (the progress counter of runtime/fle_dispatch.h and
//! fle_progress_slice of runtime/fle_thread.c) and user32!PostThreadMessageW (runtime/fle_win.c).
//!
//! Expected values are DERIVED BY HAND, not read back from the C code: from the rule of the decision (one slice ends every 4096 indirect calls of the guest after the first CreateThread, the virtual clock moves 2,500
//! ticks = 0.25 ms per slice (decision 0279: 10,000 ticks = 1 ms until then), a slice ends a thread's turn only outside the callbacks of the runtime, the next ready thread in creation order runs, a waiting thread wakes at the first slice end that is at or after its
//! deadline) and from the Learn pages (PostThreadMessageW, About Messages and Message Queues, GetMessage, PostQuitMessage, Sleep, DwmFlush as chosen by decision 0122). Every number of the decision (4096, 0.25 ms, depth 1) is a CHOICE
//! WITHOUT A SOURCE (S-vclock-*): these tests pin the choices, not a Windows rule. Nothing is compared with a real Windows.
//!
//! 1. The response row (last row, own dll only, no pseudo-VA moved).
//! 2. The counter and the slices (driver with stub guest functions; real fle_thread.c, fle_win.c, fle_event.c, fle_sys.c ...).
//! 3. PostThreadMessageW.
//! 4. The same driver compiled at -O0 and -O2 gives the same lines (the counter does not depend on the optimisation).
//! 5. Source checks.

mod etxt_common;
mod rows_0147;
mod rows_0267;

use fe_lift::generate;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn rt_dir() -> PathBuf {
    manifest().join("../../runtime")
}
/// The runtime directory the driver is built from. FLE_FR20_RT (a copy of lift/runtime) overrides it: the mutation runs of the record change one file of such a copy and expect a test to fail.
fn src_dir() -> PathBuf {
    if let Ok(d) = std::env::var("FLE_FR20_RT") {
        return PathBuf::from(d);
    }
    rt_dir()
}
fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-wallFR20/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

// ---------------------------------------------------------------------------------------------
// 1. The response row
// ---------------------------------------------------------------------------------------------

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

/// A tiny PE whose import table names `imps` and whose code calls a `jmp [IAT]` thunk for each (the same shape as the images of tests/wall_fr8.rs).
fn build_imports(imps: &[(&str, &str)]) -> Vec<u8> {
    assert!(imps.len() <= 6);
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
    w32(&mut b, o + 124, 20 * (imps.len() as u32 + 1));
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
    for (k, (dll, name)) in imps.iter().enumerate() {
        let k32 = k as u32;
        let (oft, ft, dll_rva, hn_rva) = (0x2100 + 0x10 * k32, 0x2200 + 0x10 * k32, 0x2300 + 0x20 * k32, 0x2500 + 0x40 * k32);
        w32(&mut b, d + 20 * k, oft);
        w32(&mut b, d + 20 * k + 12, dll_rva);
        w32(&mut b, d + 20 * k + 16, ft);
        put(&mut b, dll_rva, format!("{dll}\0").as_bytes());
        put(&mut b, hn_rva, &[&[0u8, 0u8][..], name.as_bytes(), &[0u8]].concat());
        w64(&mut b, data_at(oft), hn_rva as u64);
        w64(&mut b, data_at(ft), hn_rva as u64);
    }
    let t = TEXT_RAW;
    let mut code: Vec<(u32, Vec<u8>)> = Vec::new();
    for k in 0..imps.len() as u32 {
        let at = 0x1000 + 5 * k;
        code.push((at, [&[0xe8][..], &rel(at + 5, 0x1040 + 0x10 * k)].concat()));
        code.push((0x1040 + 0x10 * k, [&[0xff, 0x25][..], &rel(0x1046 + 0x10 * k, 0x2200 + 0x10 * k)].concat()));
    }
    code.push((0x1000 + 5 * imps.len() as u32, vec![0xc3]));
    for (rva, bytes) in code {
        let o = t + (rva - TEXT_RVA) as usize;
        b[o..o + bytes.len()].copy_from_slice(&bytes);
    }
    b
}

fn gen(imps: &[(&str, &str)]) -> (fe_lift::Stats, String) {
    let img = build_imports(imps);
    let mut out = Vec::new();
    let st = generate(&img, &mut out).unwrap();
    (st, String::from_utf8(out).unwrap())
}

fn resp_rows(c: &str) -> Vec<(String, String, u64)> {
    let start = c.find("const fle_resp_entry fle_resp_table[] = {").expect("no response table");
    let end = start + c[start..].find("};").unwrap();
    c[start..end]
        .lines()
        .filter_map(|l| {
            let l = l.trim();
            let l = l.strip_prefix("{ \"")?;
            let (dll, rest) = l.split_once("\", \"")?;
            let (name, rest) = rest.split_once("\", UINT64_C(0x")?;
            let (va, _) = rest.split_once(')')?;
            Some((dll.to_string(), name.to_string(), u64::from_str_radix(va, 16).unwrap()))
        })
        .collect()
}

const GUEST_IMPORTS: [(&str, &str); 2] = [("kernel32.dll", "GetProcAddress"), ("kernel32.dll", "LoadLibraryW")];

#[test]
fn post_thread_message_w_is_the_last_row_after_the_rows_of_the_earlier_decisions_and_no_pseudo_va_moved() {
    let (st, c) = gen(&GUEST_IMPORTS);
    let rows = resp_rows(&c);
    assert_eq!(rows.len(), st.resp_entries);
    let k = rows_0147::ROWS_0147.len();
    let n = rows.len() - rows_0267::ROWS_0267.len(); // decision 0267 appends 1 row (tests/rows_0267) after this one
    // decision 0015: append only. HEAD 7b82424 ends with user32!GetLayeredWindowAttributes (decision 0140); once decision 0144's rows are in the tree they come between (the check is "last row", not "after row 605")
    for (i, (dll, name)) in rows_0147::ROWS_0147.iter().enumerate() {
        assert_eq!((rows[n - k + i].0.as_str(), rows[n - k + i].1.as_str()), (*dll, *name), "row {i} of decision 0147");
    }
    assert_ne!(rows[n - k - 1].1, "PostThreadMessageW", "exactly one row");
    for (i, (_, _, va)) in rows.iter().enumerate() {
        assert_eq!(*va, 0x7FF9_0000_0000u64 + 0x10 * (i as u64 + 1), "row {i}");
    }
    assert_eq!(rows.iter().filter(|(_, name, _)| name == "PostThreadMessageW").count(), 1);
    assert!(c.contains("void fle_imp_PostThreadMessageW(void);"));
    // and in the source table, right before the row of decision 0267
    let src = std::fs::read_to_string(manifest().join("src/imports_table.rs")).unwrap();
    let table: Vec<&str> = src.lines().filter(|l| l.trim_start().starts_with("(\"") && l.contains("\"fle_imp_")).collect();
    let last = table[table.len() - 1 - rows_0267::ROWS_0267.len()].trim().to_string();
    assert_eq!(last, "(\"user32.dll\", \"PostThreadMessageW\", \"fle_imp_PostThreadMessageW\", \"jmp_imp:PostThreadMessageW\"),");
}

#[test]
fn the_thunk_is_answered_under_user32_and_under_no_other_dll() {
    let (st, c) = gen(&[("user32.dll", "PostThreadMessageW"), ("kernel32.dll", "PostThreadMessageW"), ("user32.dll", "SendMessageA"), ("kernel32.dll", "GetProcAddress")]);
    assert_eq!(st.handled["jmp_imp:PostThreadMessageW"], 1);
    assert_eq!(c.matches("fle_imp_PostThreadMessageW(); return;").count(), 1, "only the thunk of user32 is answered");
    // the first thunk (0x1040) is user32!PostThreadMessageW; the second (0x1050) is kernel32!PostThreadMessageW, which does not exist and stays unsupported
    let va = IB + 0x1040;
    assert!(c.contains(&format!("L_{va:x}: {{ /* {va:#x} Jmp */ {{ void fle_imp_PostThreadMessageW(void); fle_imp_PostThreadMessageW(); return; }}}}")), "thunk 0 is answered");
    let va = IB + 0x1050;
    assert!(c.contains(&format!("L_{va:x}: {{ /* {va:#x} Jmp */ fle_unsupported(UINT64_C({va:#x})); }}")), "the same name under kernel32 stays unsupported");
}

// ---------------------------------------------------------------------------------------------
// 2. The driver
// ---------------------------------------------------------------------------------------------

const RT_FILES: [&str; 18] = [
    "fle_thread.c", "fle_win.c", "fle_class.c", "fle_event.c", "fle_sys.c", "fle_imports.c", "fle_proc.c", "fle_callback.c", "fle_env.c", "fle_vfs.c", "fle_vmap.c", "fle_vol.c", "fle_host.c",
    "fle_time.c", "fle_x87ctl.c", "fle_mxcsr.c", "fle_x87.c", "fle_locale.c",
];

fn clang_c(src: &Path, obj: &Path, strict: bool) {
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-Wall", "-Wextra", "-fPIE", "-c"]);
    if strict {
        cmd.arg("-Werror");
    }
    let cc = cmd.arg("-I").arg(src_dir()).arg("-I").arg(rt_dir()).arg(src).arg("-o").arg(obj).env("TMPDIR", work_dir().join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {}: {}", src.display(), String::from_utf8_lossy(&cc.stderr));
}

fn objs() -> &'static Vec<PathBuf> {
    static O: OnceLock<Vec<PathBuf>> = OnceLock::new();
    O.get_or_init(|| {
        let dir = work_dir().join(etxt_common::per_process("obj-fr20"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut v = Vec::new();
        let ours = ["fle_thread.c", "fle_win.c"];
        for name in RT_FILES.iter() {
            let is_ours = ours.contains(name);
            let base = if is_ours { src_dir() } else { rt_dir() };
            let o = dir.join(name.replace(".c", ".o"));
            clang_c(&base.join(name), &o, is_ours);
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

const PRELUDE: &str = r##"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <stddef.h>
#include "fle_runtime.h"
#include "fle_sse.h"
#include "fle_dispatch.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_modules.h"
#include "fle_callback.h"
#include "fle_class.h"
#include "fle_sys.h"
#include "fle_proc.h"
#include "fle_event.h"
#include "fle_win.h"
#include "fle_thread.h"
#include "fle_vfs.h"
#include "fle_locale.h"
#include "fle_sysinfo.h"
#include "fle_x87ctl.h"
#include "fle_mxcsr.h"
void fle_imp_CloseHandle(void);

uint64_t R[16];
fle_eflags FL;
fle_xmm X[16];
static uint64_t stk[0x4000] __attribute__((aligned(4096)));          /* 128 KiB: the first thread's guest stack */
#define E0 ((uint64_t)(uintptr_t)stk + 0x10000 + 8)
#define RET0 UINT64_C(0xdead0000)
#define U(x) ((unsigned long long)(x))
#define A(x) ((uint64_t)(uintptr_t)(x))
#define INF 0xFFFFFFFFu

#define CALL4(f, a, b, c, d) do { R[1] = (a); R[2] = (b); R[8] = (c); R[9] = (d); R[4] -= 8; fle_st(R[4], 8, 0xAAAA0000); f(); R[4] += 8; } while (0)
#define CALL6(f, a, b, c, d, e, g) do { uint64_t sp0_ = R[4]; R[4] = (R[4] - 0x40) & ~UINT64_C(15); R[4] -= 8; fle_st(R[4], 8, 0xAAAA0000); fle_st(R[4] + 0x28, 8, (e)); fle_st(R[4] + 0x30, 8, (g)); \
    R[1] = (a); R[2] = (b); R[8] = (c); R[9] = (d); f(); R[4] = sp0_; } while (0)
#define CALL5(f, a, b, c, d, e) CALL6(f, a, b, c, d, e, 0)
static uint32_t LE(void) { fle_imp_GetLastError(); return (uint32_t)R[0]; }
static uint64_t H_CREATE(uint64_t start, uint64_t param, uint32_t flags, uint64_t ptid) { CALL6(fle_imp_CreateThread, 0, 0, start, param, flags, ptid); return R[0]; }
static uint64_t H_EVENT(int manual, int init) { CALL4(fle_imp_CreateEventW, 0, manual, init, 0); return R[0]; }
static uint32_t W1(uint64_t h, uint32_t ms) { CALL4(fle_imp_WaitForSingleObject, h, ms, 0, 0); return (uint32_t)R[0]; }
static void SETEV(uint64_t h) { CALL4(fle_imp_SetEvent, h, 0, 0, 0); }
static void SLEEP(uint32_t ms) { CALL4(fle_imp_Sleep, ms, 0, 0, 0); }
static uint32_t EXITCODE(uint64_t h) { uint32_t c = 0xEEEE; CALL4(fle_imp_GetExitCodeThread, h, A(&c), 0, 0); return c; }
static void boot(void) { R[4] = E0; fle_st(E0, 8, RET0); fle_teb_set_stack(A(stk), A(stk) + sizeof stk); }

/* the virtual clock since its origin, in ticks of 100 ns and in whole ms */
static uint64_t REL(void) { return fle_sys_clock_ticks() - FLE_CLOCK_START_MS * FLE_TICKS_PER_MS; }
static unsigned RELMS(void) { return (unsigned)(REL() / FLE_TICKS_PER_MS); }

/* guest work: n indirect calls of a function that does nothing; each is one unit of progress */
#define VA_NOP 0x1100
static void nop(void) { }
static void work(unsigned n) { unsigned i; for (i = 0; i < n; i++) fle_dispatch(VA_NOP); }

typedef struct { uint64_t hwnd; uint32_t msg; uint32_t pad; uint64_t wp, lp; uint32_t time; int32_t ptx, pty; uint32_t priv; } MSGX;
static MSGX mb_main, mb_b;
static uint32_t GET(MSGX *m) { CALL4(fle_imp_GetMessageW, A(m), 0, 0, 0); return (uint32_t)R[0]; }
static uint32_t PEEK(MSGX *m, uint32_t lo, uint32_t hi, uint32_t rm) { CALL5(fle_imp_PeekMessageW, A(m), 0, lo, hi, rm); return (uint32_t)R[0]; }
static uint32_t POSTT(uint32_t id, uint32_t msg, uint64_t wp, uint64_t lp) { CALL4(fle_imp_PostThreadMessageW, id, msg, wp, lp); return (uint32_t)R[0]; }
static void stats(const char *tag)
{
    fle_preempt_stats s;
    fle_thread_preempt_stats(&s);
    printf("%s: armed %d slices %llu deferred %llu switches %llu ticks %llu\n", tag, s.armed, U(s.slices), U(s.deferred), U(s.switches), U(s.ticks));
}
"##;

fn run_src_with(name: &str, top: &str, body: &str, opt: &str, env: &[(&str, &str)]) -> Out {
    let dir = work_dir();
    let uname = etxt_common::uniq(name);
    let c = dir.join(format!("{uname}.c"));
    let exe = dir.join(&uname);
    let src = format!("{PRELUDE}\n{top}\nint main(void)\n{{\n    boot();\n{body}\n    return 0;\n}}\n");
    std::fs::write(&c, src).unwrap();
    let mut cmd = Command::new("clang");
    cmd.args([opt, "-std=gnu11", "-w", "-fPIE", "-pthread", "-I"]).arg(src_dir()).arg("-I").arg(rt_dir());
    for o in objs() {
        cmd.arg(o);
    }
    let cc = cmd.arg("-o").arg(&exe).arg(&c).arg("-lm").env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    let mut run = Command::new(&exe);
    run.env("TMPDIR", dir.join("tmp")).env_remove("FLE_PREEMPT").env_remove("FLE_PREEMPT_DISPATCHES").env_remove("FLE_PREEMPT_TICKS");
    for (k, v) in env {
        run.env(k, v);
    }
    let r = run.output().expect("run failed");
    Out {
        stdout: String::from_utf8_lossy(&r.stdout).to_string(),
        stderr: String::from_utf8_lossy(&r.stderr).to_string(),
        signal: r.status.signal(),
        ok: r.status.success(),
    }
}

fn lines_with(name: &str, top: &str, body: &str, opt: &str, env: &[(&str, &str)]) -> Vec<String> {
    let o = run_src_with(name, top, body, opt, env);
    assert!(o.ok, "{name} did not exit normally (signal {:?}): {}\n{}", o.signal, o.stderr, o.stdout);
    o.stdout.lines().map(|s| s.to_string()).collect()
}

fn lines(name: &str, top: &str, body: &str) -> Vec<String> {
    lines_with(name, top, body, "-O0", &[])
}

const VA: &str = r#"
#define VA_A 0x1000
#define VA_B 0x1010
#define VA_C 0x1020
#define VA_OUTER 0x1040
#define VA_INNER 0x1050
"#;

/// The dispatch table must be sorted by VA (binary search).
fn table(entries: &[(&str, &str)]) -> String {
    let mut s = String::from("const fle_fn_entry fle_dispatch_table[] = {\n");
    for (va, f) in entries {
        s.push_str(&format!("    {{ {va}, {f} }},\n"));
    }
    s.push_str(&format!("    {{ VA_NOP, nop }},\n}};\nconst uint64_t fle_dispatch_count = {};\n", entries.len() + 1));
    s
}

// ---- 2a. before the first CreateThread nothing is armed ----

#[test]
fn before_the_first_create_thread_the_counter_counts_and_the_clock_does_not_move() {
    // The counter is one unit per indirect call that fle_dispatch finds; unarmed, the limit is UINT64_MAX and no slice ever ends: the virtual clock does not move (a run of one thread is what it was).
    let top = format!("{VA}\n{}", table(&[]));
    let l = lines(
        "p-unarmed",
        &top,
        r#"
    printf("limit-unarmed %d\n", fle_progress_limit == UINT64_MAX);
    work(10000);
    printf("count %llu clock %u ticks %llu\n", U(fle_progress_count), RELMS(), U(REL()));
    stats("stats");
"#,
    );
    assert_eq!(l, ["limit-unarmed 1", "count 10000 clock 0 ticks 0", "stats: armed 0 slices 0 deferred 0 switches 0 ticks 0"]);
}

// ---- 2b. slices ----

#[test]
fn a_slice_ends_every_4096_indirect_calls_moves_the_clock_a_quarter_ms_and_wakes_a_sleeper_at_the_slice_that_reaches_its_deadline() {
    // Hand derivation (decision 0279: a slice moves the clock 2,500 ticks = 0.25 ms; the times are printed in ticks). 100 calls before CreateThread (count 100). CreateThread arms: limit 100 + 4096 = 4196. The new thread B (flags 0)
    // is ready and unstarted. Slice 1 at count 4196: clock 2,500 ticks; B (the next ready thread) runs: its start is one more indirect call (fle_call_guest), it prints "tick 0 clock 2500" and Sleep(5): deadline
    // 2,500 + 50,000 = 52,500 ticks; main goes on. Slices 2..20 (5,000..50,000 ticks) find B waiting; slice 21 (52,500, the deadline itself) wakes B: "tick 1 clock 52500", Sleep(5) again (102,500). Slices 22..32: nothing
    // (the next deadline is slice 41). 32 slices in all; the 32nd is at count 100 + 4096 * 32 = 131172; main makes 100 + 4096 * 32 = 131172 calls + the start call of B: 131173 calls in all; 32 * 2,500 = 80,000 ticks = 8 ms.
    let top = format!(
        "{VA}\nstatic void th_b(void) {{ int i; for (i = 0; i < 4; i++) {{ printf(\"B tick %d clock %llu\\n\", i, U(REL())); SLEEP(5); }} R[0] = 0; }}\n{}",
        table(&[("VA_B", "th_b")])
    );
    let l = lines(
        "p-slices",
        &top,
        r#"
    work(100);
    H_CREATE(VA_B, 0, 0, 0);
    printf("armed: limit-minus-count %llu\n", U(fle_progress_limit - fle_progress_count));
    work(4096 * 32);
    printf("end count %llu clock %u ticks %llu\n", U(fle_progress_count), RELMS(), U(REL()));
    stats("stats");
"#,
    );
    assert_eq!(
        l,
        [
            "armed: limit-minus-count 4096",
            "B tick 0 clock 2500",
            "B tick 1 clock 52500",
            "end count 131173 clock 8 ticks 80000",
            "stats: armed 1 slices 32 deferred 0 switches 2 ticks 80000",
        ]
    );
}

#[test]
fn a_slice_inside_a_callback_of_the_runtime_is_skipped_and_the_clock_does_not_move() {
    // Hand derivation. B waits for an event forever. CreateThread arms at count 0 (limit 4096). Sleep(0) lets B start (1 call, count 1; it waits at once) and returns. fle_call_guest(OUTER) is call 2 (depth 1).
    // (Decision 0279: a slice moves the clock 2,500 ticks; the clock is printed in ticks.) OUTER: work(8192) = calls 3..8194: slice 1 at 4096 (clock 2500), slice 2 at 8192 (clock 5000). fle_call_guest(INNER) is call 8195 (depth 2); INNER work(12288) = 8196..20483: the limit passes
    // 12288, 16384, 20480 inside the callback: 3 skipped, no clock. Back in OUTER (depth 1), work(8192) = 20484..28675: slice 3 at 24576 (clock 7500), slice 4 at 28672 (clock 10000).
    let top = format!(
        "{VA}\nstatic uint64_t ev;\nstatic void th_b(void) {{ W1(ev, INF); R[0] = 0; }}\nstatic void outer(void) {{ work(8192); fle_call_guest(VA_INNER, 0, 0, 0, 0); work(8192); R[0] = 0; }}\n\
         static void inner(void) {{ printf(\"inner depth %u clock %llu\\n\", fle_callback_depth(), U(REL())); work(12288); printf(\"inner end clock %llu\\n\", U(REL())); R[0] = 0; }}\n{}",
        table(&[("VA_B", "th_b"), ("VA_OUTER", "outer"), ("VA_INNER", "inner")])
    );
    let l = lines(
        "p-callback",
        &top,
        r#"
    ev = H_EVENT(1, 0);
    H_CREATE(VA_B, 0, 0, 0);
    SLEEP(0);
    printf("count after start %llu\n", U(fle_progress_count));
    fle_call_guest(VA_OUTER, 0, 0, 0, 0);
    printf("end count %llu clock %llu\n", U(fle_progress_count), U(REL()));
    stats("stats");
"#,
    );
    assert_eq!(
        l,
        [
            "count after start 1",
            "inner depth 2 clock 5000",
            "inner end clock 5000",
            "end count 28675 clock 10000",
            "stats: armed 1 slices 4 deferred 3 switches 0 ticks 10000",
        ]
    );
}

#[test]
fn two_busy_threads_take_turns_one_slice_each_in_creation_order() {
    // Hand derivation (count 0 at CreateThread, limit 4096; decision 0279: slice n ends at clock 2,500 * n ticks and the clocks are printed in ticks: slice 2 = 5000, 3 = 7500, 4 = 10000, 5 = 12500, 6 = 15000). Main: work(4096) = calls 1..4096; the last one is slice 1 (clock 1 ms): B (ready, unstarted) runs, its start is call 4097, B: work(4096) = 4098..8193;
    // slice 2 at 8192 (clock 2 ms) is in B and gives the processor back to main (ready): main finishes work #1 and prints "M round 0 clock 2". Main work #2 = 8193..12288, slice 3 at 12288 (clock 3): to B, which
    // does the one call of its work #1 that is left (12289) and prints "B round 0 clock 3"; B work #2 = 12290..16385, slice 4 at 16384 (clock 4) back to main: "M round 1 clock 4"; main work #3 = 16385..20480,
    // slice 5 at 20480 (clock 5) to B: its last call of work #2 (20481): "B round 1 clock 5"; B work #3 = 20482..24577, slice 6 at 24576 (clock 6) to main: "M round 2 clock 6"; main waits for B, which makes its
    // last call (24577) and prints "B round 2 clock 6".
    let top = format!(
        "{VA}\nstatic void th_b(void) {{ int i; for (i = 0; i < 3; i++) {{ work(4096); printf(\"B round %d clock %llu\\n\", i, U(REL())); }} R[0] = 0; }}\n{}",
        table(&[("VA_B", "th_b")])
    );
    let body = r#"
    int i;
    uint64_t h = H_CREATE(VA_B, 0, 0, 0);
    for (i = 0; i < 3; i++) { work(4096); printf("M round %d clock %llu\n", i, U(REL())); }
    W1(h, INF);
    stats("stats");
"#;
    let want = [
        "M round 0 clock 5000",
        "B round 0 clock 7500",
        "M round 1 clock 10000",
        "B round 1 clock 12500",
        "M round 2 clock 15000",
        "B round 2 clock 15000",
        "stats: armed 1 slices 6 deferred 0 switches 6 ticks 15000",
    ];
    assert_eq!(lines("p-alternate", &top, body), want);
    // the counter and the slices do not depend on the optimisation of the code that makes the indirect calls (the lifted code of the engine is compiled at -O0 ... -O2 or -Oz)
    assert_eq!(lines_with("p-alternate-o2", &top, body, "-O2", &[]), want);
}

#[test]
fn a_ticker_thread_ticks_while_the_main_thread_computes_and_main_sees_the_ticks_before_it_goes_on() {
    // The shape of the logo: C ticks every 16 ms (Sleep(16)); main "initialises" for 400 slices (100 ms of virtual time at 2,500 ticks per slice, decision 0279) and only then asks for the end. Hand derivation: C first runs at slice 1 (2,500 ticks), then at the
    // slices whose clock reaches 162,500, 322,500, 482,500, 642,500, 802,500, 962,500 ticks (slices 65, 129, 193, 257, 321, 385; each Sleep(16) = 160,000 ticks): 7 ticks by the end of the 400 slices (the next one, slice 449, is later). Main sets the stop flag and waits for C: all threads wait, the clock goes to C's deadline 962,500 + 160,000 = 1,122,500 ticks = 112 ms and a quarter (the
    // exact-deadline jump of decision 0122 still applies when every thread waits), C ends, main goes on at 112.25 ms.
    let top = format!(
        "{VA}\nstatic int ticks, stop;\nstatic void th_c(void) {{ for (;;) {{ ticks++; SLEEP(16); if (stop) break; }} R[0] = 0; }}\n{}",
        table(&[("VA_C", "th_c")])
    );
    let body = r#"
    uint64_t h = H_CREATE(VA_C, 0, 0, 0);
    work(400 * 4096);
    printf("ticks-before-the-end %d clock %u\n", ticks, RELMS());
    stop = 1;
    printf("wait %u\n", W1(h, INF));
    printf("ticks %d clock %u exit %u\n", ticks, RELMS(), EXITCODE(h));
"#;
    // C wakes at 112.25 ms, finds the stop flag and leaves the loop before the `ticks++` of the next round: still 7 ticks (an earlier text of this test expected 8: a slip in the hand derivation, the loop does not tick after the wake-up)
    let want = ["ticks-before-the-end 7 clock 100", "wait 0", "ticks 7 clock 112 exit 0"];
    assert_eq!(lines("p-ticker", &top, body), want);
    assert_eq!(lines_with("p-ticker-o2", &top, body, "-O2", &[]), want);
}

#[test]
fn dwm_flush_wakes_at_the_first_slice_end_at_or_after_the_60_hz_boundary() {
    // DwmFlush waits for the first 60 Hz boundary strictly after now (decision 0122: ceil(k * 10^7 / 60) ticks). B starts at slice 1 (2,500 ticks; decision 0279): the boundary is 166,667 -> the slice ending at 167,500
    // (slice 67; slice 66 ends at 165,000); the next boundary after 167,500 is 333,334 -> slice 134 (335,000); then 500,000 -> slice 200 (500,000 itself).
    let top = format!(
        "{VA}\nstatic void th_b(void) {{ int i; for (i = 0; i < 3; i++) {{ fle_imp_DwmFlush(); printf(\"B frame %d at %llu\\n\", i, U(REL())); }} R[0] = 0; }}\n{}",
        table(&[("VA_B", "th_b")])
    );
    let l = lines(
        "p-dwm",
        &top,
        r#"
    H_CREATE(VA_B, 0, 0, 0);
    work(240 * 4096);
    printf("end clock %u\n", RELMS());
"#,
    );
    assert_eq!(l, ["B frame 0 at 167500", "B frame 1 at 335000", "B frame 2 at 500000", "end clock 60"]);
}

// ---------------------------------------------------------------------------------------------
// 3. PostThreadMessageW
// ---------------------------------------------------------------------------------------------

#[test]
fn post_thread_message_w_fails_with_1444_for_an_unknown_id_a_thread_without_a_queue_and_a_thread_that_ended() {
    // Learn PostThreadMessageW: "GetLastError returns ERROR_INVALID_THREAD_ID if idThread is not a valid thread identifier, or if the thread specified by idThread does not have a message queue" (1444).
    // The ids of the guest's threads are 0x1001 + the table index: main 0x1001, the first created thread 0x1002.
    let top = format!(
        "{VA}\nstatic uint64_t ready;\nstatic void th_b(void) {{ PEEK(&mb_b, 0x400, 0x400, 0); SETEV(ready); while (GET(&mb_b)) {{}} R[0] = 0; }}\n{}",
        table(&[("VA_B", "th_b")])
    );
    let l = lines(
        "m-fail",
        &top,
        r#"
    uint64_t h;
    uint32_t r;
    ready = H_EVENT(1, 0);
    r = POSTT(0x1001, 0x500, 1, 2); printf("to-main-before-its-queue %u le %u\n", r, LE());
    h = H_CREATE(VA_B, 0, 0, 0);
    r = POSTT(0x1002, 0x401, 1, 2); printf("to-B-before-it-ran %u le %u\n", r, LE());
    r = POSTT(0x7777, 0x401, 1, 2); printf("unknown-id %u le %u\n", r, LE());
    r = POSTT(0, 0x401, 1, 2); printf("id-0 %u le %u\n", r, LE());
    W1(ready, INF);
    r = POSTT(0x1002, 0x401, 1, 2); printf("to-B-with-a-queue %u\n", r);
    POSTT(0x1002, 0x12, 0, 0);
    W1(h, INF);
    r = POSTT(0x1002, 0x401, 1, 2); printf("to-B-after-it-ended %u le %u\n", r, LE());
"#,
    );
    assert_eq!(
        l,
        [
            "to-main-before-its-queue 0 le 1444",
            "to-B-before-it-ran 0 le 1444",
            "unknown-id 0 le 1444",
            "id-0 0 le 1444",
            "to-B-with-a-queue 1",
            "to-B-after-it-ended 0 le 1444",
        ]
    );
}

#[test]
fn a_posted_thread_message_has_hwnd_null_comes_in_order_and_wm_quit_comes_last_with_the_last_exit_code_and_ends_the_loop() {
    // Learn PostThreadMessage: the thread retrieves the message with GetMessage/PeekMessage and "The hwnd member of the returned MSG structure is NULL". Learn About Messages and Message Queues: WM_QUIT is "kept
    // in the queue and forwarded ... only when the queue contains no other messages" (so the 0x403 posted AFTER the WM_QUIT comes first); WM_QUIT: GetMessage returns zero. A second WM_QUIT replaces the
    // exit code (the choice S10 of decision 0122, as PostQuitMessage). B's thread function returns the wParam of WM_QUIT; GetExitCodeThread gives it; WaitForSingleObject(thread) is signalled.
    let top = format!(
        "{VA}\nstatic uint64_t ready;\n\
         static void th_b(void) {{ uint32_t r; PEEK(&mb_b, 0x400, 0x400, 0); SETEV(ready);\n\
           for (;;) {{ r = GET(&mb_b); printf(\"B got %u msg %x wp %llx lp %llx hwnd %llx\\n\", r, mb_b.msg, U(mb_b.wp), U(mb_b.lp), U(mb_b.hwnd)); if (!r) break; }}\n\
           R[0] = mb_b.wp; }}\n{}",
        table(&[("VA_B", "th_b")])
    );
    let l = lines(
        "m-order",
        &top,
        r#"
    uint64_t h;
    ready = H_EVENT(1, 0);
    h = H_CREATE(VA_B, 0, 0, 0);
    W1(ready, INF);
    uint32_t p1 = POSTT(0x1002, 0x401, 1, 2), p2 = POSTT(0x1002, 0x402, 3, 4), p3 = POSTT(0x1002, 0x12, 7, 9), p4 = POSTT(0x1002, 0x12, 5, 9), p5 = POSTT(0x1002, 0x403, 5, 6);
    printf("posted %u %u %u %u %u\n", p1, p2, p3, p4, p5);
    printf("wait %u\n", W1(h, INF));
    printf("exit-code %u\n", EXITCODE(h));
"#,
    );
    assert_eq!(
        l,
        [
            "posted 1 1 1 1 1",
            "B got 1 msg 401 wp 1 lp 2 hwnd 0",
            "B got 1 msg 402 wp 3 lp 4 hwnd 0",
            "B got 1 msg 403 wp 5 lp 6 hwnd 0",
            "B got 0 msg 12 wp 5 lp 0 hwnd 0",
            "wait 0",
            "exit-code 5",
        ]
    );
}

#[test]
fn a_thread_message_to_the_calling_thread_is_peeked_with_hwnd_null_and_the_queue_limit_is_10000() {
    // Learn PostThreadMessage: "There is a limit of 10,000 posted messages per message queue" and ERROR_NOT_ENOUGH_QUOTA (1816) when it is hit. The calling thread's own queue exists after its first PeekMessage.
    let top = format!("{VA}\n{}", table(&[]));
    let l = lines(
        "m-self-quota",
        &top,
        r#"
    unsigned i, ok = 0;
    uint32_t r;
    PEEK(&mb_main, 0, 0, 0);
    for (i = 0; i < 10000; i++) ok += POSTT(0x1001, 0x500, i, 0);
    r = POSTT(0x1001, 0x500, 10000, 0);
    printf("accepted %u 10001st %u le %u\n", ok, r, LE());
    r = PEEK(&mb_main, 0, 0, 1);
    printf("peek %u msg %x wp %llx hwnd %llx\n", r, mb_main.msg, U(mb_main.wp), U(mb_main.hwnd));
    r = POSTT(0x1001, 0x500, 10000, 0);
    printf("after-one-removed %u\n", r);
"#,
    );
    assert_eq!(l, ["accepted 10000 10001st 0 le 1816", "peek 1 msg 500 wp 0 hwnd 0", "after-one-removed 1"]);
}

#[test]
fn the_thread_of_the_logo_ends_when_it_is_told_to_quit_the_whole_shape_of_decision_0147() {
    // The shape of the engine's end of the logo (perf3's what-if made real): a thread draws every 16 ms in a message loop driven by a timer-like Sleep thread; the main thread tells it to quit with
    // PostThreadMessageW(WM_QUIT) and waits for it; the thread's loop ends, its function returns, the wait is signalled. Derivation: main computes 80 slices (20 ms at 2,500 ticks per slice, decision 0279); K posts at slices 1 and 65 (2,500 and 162,500 ticks; 2 draws), main posts the
    // quit at 20 ms and waits; T is blocked in GetMessage: the quit makes it ready at once, it ends at clock 20 ms (nothing waits for a deadline).
    let top = format!(
        "{VA}\nstatic uint64_t ready; static int draws;\n\
         static void th_t(void) {{ PEEK(&mb_b, 0x400, 0x400, 0); SETEV(ready); for (;;) {{ if (!GET(&mb_b)) break; draws++; }} R[0] = 0; }}\n\
         static void th_k(void) {{ int i; for (i = 0; i < 100; i++) {{ if (POSTT(0x1002, 0x8ffc, 0, 0) != 1) break; SLEEP(16); }} R[0] = 0; }}\n{}",
        table(&[("VA_A", "th_t"), ("VA_B", "th_k")])
    );
    let l = lines(
        "m-logo",
        &top,
        r#"
    uint64_t t, k;
    ready = H_EVENT(1, 0);
    t = H_CREATE(VA_A, 0, 0, 0);
    W1(ready, INF);
    k = H_CREATE(VA_B, 0, 0, 0);
    work(80 * 4096);
    printf("draws-before-the-end %d clock %u\n", draws, RELMS());
    printf("quit-posted %u\n", POSTT(0x1002, 0x12, 0, 0));
    printf("wait-t %u\n", W1(t, INF));
    printf("draws %d clock %u exit %u\n", draws, RELMS(), EXITCODE(t));
    W1(k, INF);
    printf("the poster stopped when the target ended\n");
"#,
    );
    // k posts at 0.25 ms (draw 1), 16.25 ms (draw 2); main's 80 slices end at 20 ms with 2 draws
    assert_eq!(l[0], "draws-before-the-end 2 clock 20");
    assert_eq!(l[1], "quit-posted 1");
    assert_eq!(l[2], "wait-t 0");
    assert_eq!(l[3], "draws 2 clock 20 exit 0");
    assert_eq!(l[4], "the poster stopped when the target ended");
}

// ---------------------------------------------------------------------------------------------
// 5. Source checks
// ---------------------------------------------------------------------------------------------

#[test]
fn the_progress_counter_is_in_the_one_inline_function_that_every_indirect_call_ends_in() {
    let h = std::fs::read_to_string(rt_dir().join("fle_dispatch.h")).unwrap();
    assert!(h.contains("__attribute__((weak)) uint64_t fle_progress_count = 0;"));
    assert!(h.contains("__attribute__((weak)) uint64_t fle_progress_limit = UINT64_MAX;"));
    assert_eq!(h.matches("++fle_progress_count").count(), 1, "one increment, in fle_dispatch");
    assert_eq!(h.matches("&& fle_progress_slice) fle_progress_slice();").count(), 1, "one call, in fle_dispatch");
    // the generated code is not changed: fe-lift emits no reference to the counter
    let lift = std::fs::read_to_string(manifest().join("src/lib.rs")).unwrap();
    assert!(!lift.contains("fle_progress"));
    // fle_progress_slice is the only caller of the clock in fle_thread.c besides the waits
    let t = std::fs::read_to_string(rt_dir().join("fle_thread.c")).unwrap();
    assert_eq!(t.matches("void fle_progress_slice(void)").count(), 1);
    assert!(t.contains("fle_callback_depth() > FLE_PREEMPT_MAX_DEPTH"));
    let th = std::fs::read_to_string(rt_dir().join("fle_thread.h")).unwrap();
    assert!(th.contains("#define FLE_PREEMPT_SLICE_DISPATCHES 4096u"));
    assert!(th.contains("#define FLE_PREEMPT_SLICE_TICKS      2500u"));
    assert!(th.contains("#define FLE_PREEMPT_MAX_DEPTH        1u"));
}
