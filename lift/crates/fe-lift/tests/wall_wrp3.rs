//! Tests of decision 0200 (docs/0200-engine-serum-output.md, PROVISIONAL): the per-thread implicit TLS of a third-party VST3 (wall W21 of decision 0190).
//!
//! Cause (measured with gdb on the native -O0 engine, same binary, one switch): Serum2.vst3 reads its implicit TLS block through `gs:[0x58][tls_index * 8]`. The harness (harness/two/wrp1_vst3.c)
//! made the block for the first thread only; the engine's mixer thread (guest thread 18 of 21) had slot 0 = 0, so the first IAudioProcessor::process stored through 0 + 0x18 (SIGSEGV). Microsoft Learn
//! (PE format, "The .tls Section"): "The system copies all of this data each time a thread is created"; since Windows Vista "The loader is capable of allocating TLS slots for such DLLs at load time";
//! the TLS callbacks get DLL_THREAD_ATTACH "for all but the first thread". The fix is the weak hook `fle_realmod_pre_thread(reason)` in runtime/fle_realmod.c (called at the start/end of every guest thread,
//! in that thread's context, before the DllMain notices and whatever the load state of the real modules) and its definition in harness/two/wrp1_vst3.c (opt-in: only the wrp1/wrp2/wrp3 native build links it).
//!
//! Expected values are written from the Learn sentences above and the choices of the decision (S-wrp3-tls-per-thread), not read back from the C code. Nothing is compared with a real Windows.
//!  1. runtime/fle_realmod.c: the hook is weak, is called first in the notify function, and a link that does not define it is unchanged (the DllMain notices of wall_fr8 are unchanged).
//!  2. A synthetic module with a TLS directory (and the PEB chain of wall W23: TEB+0x60 -> PEB+0x20 -> flags word, bit 31 clear, for the first thread and every new thread), the real harness/two/wrp1_vst3.c, the real fle_thread.c / fle_realmod.c: a thread has its own block (template + zero fill) whether it started before or after
//!     the module was loaded; the TLS callbacks run for DLL_THREAD_ATTACH / DETACH only while the module is loaded, before its DllMain.

mod etxt_common;

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
fn src_dir() -> PathBuf {
    if let Ok(d) = std::env::var("FLE_WRP3_SRC_DIR") {
        return PathBuf::from(d);
    }
    rt_dir()
}
fn harness_dir() -> PathBuf {
    if let Ok(d) = std::env::var("FLE_WRP3_HARNESS_DIR") {
        return PathBuf::from(d);
    }
    manifest().join("../../harness")
}
fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-wallwrp3/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

const RT_FILES: [&str; 14] = [
    "fle_thread.c", "fle_win.c", "fle_class.c", "fle_event.c", "fle_sys.c", "fle_imports.c", "fle_proc.c", "fle_callback.c", "fle_env.c", "fle_vfs.c", "fle_vmap.c", "fle_vol.c", "fle_host.c",
    "fle_time.c",
];
const RT_EXTRA: [&str; 4] = ["fle_x87ctl.c", "fle_mxcsr.c", "fle_x87.c", "fle_locale.c"];
const RT_LOADER: [&str; 6] = ["fle_load.c", "fle_realmod.c", "fle_modules.c", "fle_string.c", "fle_res.c", "fle_imagehlp.c"];

fn clang_c(src: &Path, obj: &Path, strict: bool) {
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-Wall", "-Wextra", "-fPIE", "-c"]);
    if strict {
        cmd.arg("-Werror");
    }
    let cc = cmd.arg("-I").arg(src_dir()).arg("-I").arg(rt_dir()).arg(src).arg("-o").arg(obj).env("TMPDIR", work_dir().join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {}: {}", src.display(), String::from_utf8_lossy(&cc.stderr));
}

/// The runtime objects (the real fle_thread.c, fle_realmod.c ... from src_dir(), which is runtime/ unless FLE_WRP3_SRC_DIR points at a changed copy) and the harness file under test.
fn objs() -> &'static Vec<PathBuf> {
    static O: OnceLock<Vec<PathBuf>> = OnceLock::new();
    O.get_or_init(|| {
        let dir = work_dir().join(etxt_common::per_process("obj-wrp3"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut v = Vec::new();
        for name in RT_FILES.iter().chain(RT_EXTRA.iter()).chain(RT_LOADER.iter()) {
            let o = dir.join(name.replace(".c", ".o"));
            clang_c(&src_dir().join(name), &o, *name == "fle_realmod.c" || *name == "fle_thread.c");
            v.push(o);
        }
        let o = dir.join("wrp1_vst3.o");
        let cc = Command::new("clang")
            .args(["-O0", "-std=gnu11", "-w", "-fPIE", "-c", "-DWRP1_VST3_BASE=UINT64_C(0x10000000)", "-I"])
            .arg(src_dir())
            .arg("-I")
            .arg(rt_dir())
            .arg("-I")
            .arg(harness_dir())
            .arg(harness_dir().join("two/wrp1_vst3.c"))
            .arg("-o")
            .arg(&o)
            .env("TMPDIR", work_dir().join("tmp"))
            .output()
            .expect("clang not found");
        assert!(cc.status.success(), "clang failed for wrp1_vst3.c: {}", String::from_utf8_lossy(&cc.stderr));
        v.push(o);
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
void fle_imp_LoadLibraryW(void);

uint64_t R[16];
fle_eflags FL;
fle_xmm X[16];
static uint64_t stk[0x4000] __attribute__((aligned(4096)));          /* 128 KiB: the first thread's guest stack */
#define E0 ((uint64_t)(uintptr_t)stk + 0x10000 + 8)
#define RET0 UINT64_C(0xdead0000)
#define U(x) ((unsigned long long)(x))
#define A(x) ((uint64_t)(uintptr_t)(x))
#define INF 0xFFFFFFFFu

/* a call of an import answer as the lifted code makes it: the return address on the guest stack, arguments in R[] (no stack arguments) */
#define CALL4(f, a, b, c, d) do { R[1] = (a); R[2] = (b); R[8] = (c); R[9] = (d); R[4] -= 8; fle_st(R[4], 8, 0xAAAA0000); f(); R[4] += 8; } while (0)
/* with stack arguments 5 and 6: rsp is 16n+8 at the API's entry, [rsp+0x28] is argument 5 */
#define CALL6(f, a, b, c, d, e, g) do { uint64_t sp0_ = R[4]; R[4] = (R[4] - 0x40) & ~UINT64_C(15); R[4] -= 8; fle_st(R[4], 8, 0xAAAA0000); fle_st(R[4] + 0x28, 8, (e)); fle_st(R[4] + 0x30, 8, (g)); \
    R[1] = (a); R[2] = (b); R[8] = (c); R[9] = (d); f(); R[4] = sp0_; } while (0)
#define CALL5(f, a, b, c, d, e) CALL6(f, a, b, c, d, e, 0)
static uint32_t LE(void) { fle_imp_GetLastError(); return (uint32_t)R[0]; }
static void SETLE(uint32_t v) { fle_set_last_error(v); }
static uint64_t H_CREATE(uint64_t start, uint64_t param, uint32_t flags, uint64_t ptid) { CALL6(fle_imp_CreateThread, 0, 0, start, param, flags, ptid); return R[0]; }
static uint64_t H_EVENT(int manual, int init) { CALL4(fle_imp_CreateEventW, 0, manual, init, 0); return R[0]; }
static uint32_t W1(uint64_t h, uint32_t ms) { CALL4(fle_imp_WaitForSingleObject, h, ms, 0, 0); return (uint32_t)R[0]; }
static void SETEV(uint64_t h) { CALL4(fle_imp_SetEvent, h, 0, 0, 0); }
static void SLEEP(uint32_t ms) { CALL4(fle_imp_Sleep, ms, 0, 0, 0); }
static uint32_t EXITCODE(uint64_t h) { uint32_t c = 0xEEEE; CALL4(fle_imp_GetExitCodeThread, h, A(&c), 0, 0); return c; }

static int ord[64], on_;
static void note(int v) { if (on_ < 64) ord[on_++] = v; }
static void print_order(void) { int i; printf("order:"); for (i = 0; i < on_; i++) printf(" %d", ord[i]); printf("\n"); }
static void boot(void) { R[4] = E0; fle_st(E0, 8, RET0); fle_teb_set_stack(A(stk), A(stk) + sizeof stk); }
"##;

/// Builds and runs one program: the PRELUDE (guest-call helpers), `top` (dispatch table, stubs) and `body` (main), linked with the objects above. `env` is set for the run.
fn run_src(name: &str, top: &str, body: &str, env: &[(&str, &str)], use_harness_object: bool) -> Out {
    let dir = work_dir();
    let uname = etxt_common::uniq(name);
    let c = dir.join(format!("{uname}.c"));
    let exe = dir.join(&uname);
    let src = format!("{PRELUDE}\n{top}\nint main(void)\n{{\n    boot();\n{body}\n    return 0;\n}}\n");
    std::fs::write(&c, src).unwrap();
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fPIE", "-pthread", "-I"]).arg(src_dir()).arg("-I").arg(rt_dir());
    for o in objs() {
        if !use_harness_object && o.file_name().unwrap() == "wrp1_vst3.o" {
            continue;
        }
        cmd.arg(o);
    }
    let cc = cmd.arg("-o").arg(&exe).arg(&c).arg("-lm").env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    let mut run = Command::new(&exe);
    run.env("TMPDIR", dir.join("tmp"));
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

fn strip_comments(s: &str) -> String {
    let mut o = String::new();
    let b: Vec<char> = s.chars().collect();
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

// ---------------------------------------------------------------------------------------------
// 1. Source pins
// ---------------------------------------------------------------------------------------------

#[test]
fn the_pre_thread_hook_is_weak_and_runs_first() {
    let code = strip_comments(&std::fs::read_to_string(src_dir().join("fle_realmod.c")).unwrap());
    assert!(code.contains("void fle_realmod_pre_thread(uint32_t reason) __attribute__((weak));"), "the hook must be a weak declaration: a link that defines nothing is unchanged");
    let f = code.find("void fle_realmod_thread_notify(uint32_t reason)").unwrap();
    let body = &code[f..];
    let hook = body.find("if (fle_realmod_pre_thread) fle_realmod_pre_thread(reason);").expect("the call");
    let first = body.find("table_check(").unwrap();
    assert!(hook < first, "the TLS work is not a DllMain notice: it runs first, before anything else in the function");
    assert!(!body[..body.find("\n}\n").unwrap_or(body.len())].contains("getenv("), "decision 0243: FLE_THREAD_NO_DLL_NOTIFY is gone, the notices are always delivered");
    // fle_thread.c is not touched by this decision (tbl1 changes the thread table there): it still only calls the notify function
    let th = strip_comments(&std::fs::read_to_string(src_dir().join("fle_thread.c")).unwrap());
    assert!(!th.contains("fle_realmod_pre_thread"));
}

// ---------------------------------------------------------------------------------------------
// 2. A synthetic module with a TLS directory under the real harness/two/wrp1_vst3.c
// ---------------------------------------------------------------------------------------------

/// A 0x2000-byte PE image at 0x10000000 (DOS e_lfanew 0x80, "PE\0\0", PE32+, entry 0x1000 = the DllMain stub), data directory 9 (TLS) at RVA 0x200: template bytes AA BB CC DD 11 22 33 44 at RVA 0x300 (8 bytes),
/// zero fill 8, index at RVA 0x340, callbacks array at RVA 0x350 (one callback at VA 0x10001010, then NULL).
const TOP: &str = r#"
#include "fle_realmod.h"
#include <sys/mman.h>
#define BASE UINT64_C(0x10000000)
const fle_realmod_entry fle_realmod_table[] = { { "stubdll.dll", 0x10000000 } };
const uint64_t fle_realmod_count = 1;
const fle_resp_entry fle_resp_table[1];
const uint64_t fle_resp_count = 0;
void __real_fle_imp_FormatMessageW(void) {}
void __real_fle_imp_LockResource(void) {}
void wrp1_after_image_load(uint64_t base, uint64_t size);
void fle_realmod_pre_attach(uint64_t base, int idx);
extern uint64_t fle_tls_values[];

static char ev[64][48]; static unsigned nev;
static void rec(const char *what, unsigned reason)
{
    uint64_t *t = fle_thread_tls_values();
    uint64_t blk = t ? t[0] : fle_tls_values[0];
    if (nev < 64) snprintf(ev[nev++], 48, "%s %u thr %u blk %s", what, reason, fle_thread_current(), blk ? "yes" : "no");
}
static void dllmain(void) { rec("dllmain", (unsigned)R[2]); R[0] = 1; }
static void tlscb(void) { rec("tlscb", (unsigned)R[2]); R[0] = 0; }
#define VA_A 0x1000
static int peb_chain(const uint8_t *teb)
{
    uint64_t peb, params; uint32_t flags;
    if (!teb) return -1;
    memcpy(&peb, teb + 0x60, 8); if (!peb) return 0;
    memcpy(&params, (void *)(uintptr_t)(peb + 0x20), 8); if (!params) return 0;
    memcpy(&flags, (void *)(uintptr_t)(params + 8), 4);
    return (flags >> 31) == 0 ? 1 : 0;
}
static uint64_t seen_blk[4]; static unsigned seen_n;
static void th_a(void)
{
    uint64_t *t = fle_thread_tls_values();
    uint8_t *b = t ? (uint8_t *)(uintptr_t)t[0] : 0;
    char s[96] = "none";
    if (b) snprintf(s, sizeof s, "%02x%02x%02x%02x%02x%02x%02x%02x zero %02x%02x%02x%02x%02x%02x%02x%02x", b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7], b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15]);
    if (seen_n < 4) seen_blk[seen_n++] = (uint64_t)(uintptr_t)b;
    printf("th_a %s\n", s);
    printf("th_a_peb %d\n", peb_chain(fle_teb_cur));
    R[0] = 0;
}
const fle_fn_entry fle_dispatch_table[] = { { VA_A, th_a }, { 0x10001000, dllmain }, { 0x10001010, tlscb } };
const uint64_t fle_dispatch_count = 3;
"#;

const SETUP: &str = r#"
    uint8_t *img = mmap((void *)0x10000000, 0x2000, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED_NOREPLACE, -1, 0);
    static uint16_t nm[] = { 's','t','u','b','d','l','l','.','d','l','l', 0 };
    uint64_t th, th2;
    unsigned i;
    if (img == MAP_FAILED) { printf("mmap failed\n"); return 1; }
    img[0] = 'M'; img[1] = 'Z'; *(uint32_t *)(img + 0x3c) = 0x80; *(uint32_t *)(img + 0x80) = 0x4550;
    *(uint16_t *)(img + 0x80 + 24) = 0x20b; *(uint32_t *)(img + 0x80 + 24 + 16) = 0x1000;
    *(uint32_t *)(img + 0x80 + 24 + 112 + 9 * 8) = 0x200; *(uint32_t *)(img + 0x80 + 24 + 112 + 9 * 8 + 4) = 0x28;
    *(uint64_t *)(img + 0x200) = BASE + 0x300; *(uint64_t *)(img + 0x208) = BASE + 0x308; *(uint64_t *)(img + 0x210) = BASE + 0x340;
    *(uint64_t *)(img + 0x218) = BASE + 0x350; *(uint32_t *)(img + 0x220) = 8;
    { static const uint8_t t[8] = { 0xAA, 0xBB, 0xCC, 0xDD, 0x11, 0x22, 0x33, 0x44 }; memcpy(img + 0x300, t, 8); }
    *(uint64_t *)(img + 0x350) = 0x10001010;
    wrp1_after_image_load(BASE, 0x2000);                  /* the harness at start-up: the block of the first thread, the callbacks list */
"#;

#[test]
fn every_thread_has_its_own_block_with_the_template_before_and_after_the_module_is_loaded_and_the_callbacks_follow_the_load_state() {
    let body = format!(
        r#"{SETUP}
    printf("first thread block %s\n", fle_tls_values[0] ? "yes" : "no");
    printf("first_peb %d\n", peb_chain(fle_teb));
    th = H_CREATE(VA_A, 0, 0, 0);                         /* started BEFORE the module is loaded */
    W1(th, INF);
    CALL4(fle_imp_LoadLibraryW, A(nm), 0, 0, 0);          /* DLL_PROCESS_ATTACH: the TLS callback, then DllMain */
    th2 = H_CREATE(VA_A, 0, 0, 0);                        /* started AFTER */
    W1(th2, INF);
    printf("blocks distinct %d\n", seen_n == 2 && seen_blk[0] && seen_blk[1] && seen_blk[0] != seen_blk[1] && seen_blk[0] != fle_tls_values[0] && seen_blk[1] != fle_tls_values[0]);
    for (i = 0; i < nev; i++) printf("ev %s\n", ev[i]);
"#
    );
    let o = run_src("t-blocks", TOP, &body, &[], true);
    assert!(o.ok, "signal {:?}: {}\n{}", o.signal, o.stderr, o.stdout);
    // wall W23 (the PEB of the TEB): the TEB of the first thread and of every new thread has ProcessEnvironmentBlock -> ProcessParameters -> a flags word with bit 31 clear (Serum2's UCRT thread start reads it)
    assert!(o.stdout.lines().any(|x| x == "first_peb 1"), "{}", o.stdout);
    assert_eq!(o.stdout.lines().filter(|x| *x == "th_a_peb 1").count(), 2, "{}", o.stdout);
    let l: Vec<&str> = o.stdout.lines().filter(|x| !x.starts_with("wrp1:") && !x.contains("_peb ")).collect();
    assert_eq!(l[0], "first thread block yes");
    // thread 1 started before the load: it has the block (template, then the zero fill), and no callback or DllMain notice (the module was not loaded)
    assert_eq!(l[1], "th_a aabbccdd11223344 zero 0000000000000000");
    // the load: callback (reason 1) before DllMain (reason 1), in the first thread
    // thread 2 started after: the block, the callback with reason 2 BEFORE the DllMain with reason 2, then 3 and 3 at its end (callback before DllMain, the order of the attach)
    assert_eq!(l[2], "th_a aabbccdd11223344 zero 0000000000000000");
    assert_eq!(l[3], "blocks distinct 1");
    let ev: Vec<&str> = l[4..].to_vec();
    assert_eq!(
        ev,
        [
            "ev tlscb 1 thr 0 blk yes",
            "ev dllmain 1 thr 0 blk yes",
            "ev tlscb 2 thr 2 blk yes",
            "ev dllmain 2 thr 2 blk yes",
            "ev tlscb 3 thr 2 blk yes",
            "ev dllmain 3 thr 2 blk yes",
        ]
    );
}
