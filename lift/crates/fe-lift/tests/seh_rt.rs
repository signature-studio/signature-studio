//! Decision 0132: the runtime half of the guest's structured exception handling, runtime/fle_seh.c (+ fle_callback.c), run natively on the REAL tables of
//! FLEngine_x64.dll ($FLENGINE_PE, else <repo root>/binaries/win64/FLEngine_x64.dll; skipped when absent): the driver maps the image at its base (0x400000) and
//! the functions of wall 16 are C stand-ins, at their real VAs, that lay their frames out exactly as the machine code of the real functions does (the prologues were
//! read from the DLL with capstone: `push rbp; sub rsp,N; mov rbp,rsp` and so on), call each other with the lifted convention (`R[4] -= 8; [R[4]] = ret; f(); R[4] += 8`)
//! and use the real return addresses (0x411e4d, 0x411e94, 0x5df56b, 0x5df582, 0x5df66f, 0x5df6e7, 0x63ae33). fle_seh.c has to find the frames from the real
//! UNWIND_INFO / .pdata of those functions. The one stand-in that is not the guest's code is the Delphi handler 0x4112c0: it is a C port of the reading of
//! docs/investigations/seh-model/sim_scopes.py (the same scope tables, the same class chain), because the lifted handler is not here (the handler runs for real only in the
//! FRUITY run of the decision; this test checks the OS side of the contract: the arguments it gets, the order of the calls, the registers and the stack at the landing).
//!
//! The expected values are written from the paper reproduction `sim_scopes.py` of docs/investigations/seh-model.md section 3.4 (the ControlPcs and landings of the two
//! raises of wall 16, the funclets 0x5df720 and 0x5df730 in table order), from the x64 exception handling page (EstablisherFrame = the frame register minus its offset or
//! the stack pointer, UNWIND_INFO codes undone in order) and from the C ABI of the lifted code (callee-saved registers: rbx rbp rsi rdi r12-r15).
//!
//! The stand-ins check what the unwinder must do for the guest to survive: rbx, rsi and rdi are clobbered by f_411d20 (it pushes them, as its UNWIND_INFO says), and the code after
//! the landing must see the values it had before; rax = the exception object; rbp = the EstablisherFrame; rsp the one of the call site; the frames above the landing are gone.

mod etxt_common;

use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn runtime_dir() -> PathBuf {
    manifest().join("../../runtime")
}
fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-seh/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

fn flengine() -> Option<PathBuf> {
    let explicit = std::env::var_os("FLENGINE_PE");
    let p = match &explicit {
        Some(p) => PathBuf::from(p),
        None => manifest().join("../../../binaries/win64/FLEngine_x64.dll"),
    };
    if p.exists() {
        Some(p)
    } else if explicit.is_some() {
        panic!("FLENGINE_PE={}: not found", p.display());
    } else {
        eprintln!("SKIPPED: {}: not found", p.display());
        None
    }
}

fn clang_c(src: &Path, obj: &Path, strict: bool) {
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-Wall", "-Wextra", "-fPIE", "-c"]);
    if strict {
        cmd.arg("-Werror");
    }
    let cc = cmd.arg("-I").arg(runtime_dir()).arg(src).arg("-o").arg(obj).env("TMPDIR", work_dir().join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {}: {}", src.display(), String::from_utf8_lossy(&cc.stderr));
}

/// fle_seh.c, fle_callback.c, fle_proc.c and the files that fle_proc.c links with (the five of tests/proc.rs), from runtime/ (warnings are errors), once per process.
fn objs() -> &'static Vec<PathBuf> {
    static O: OnceLock<Vec<PathBuf>> = OnceLock::new();
    O.get_or_init(|| {
        let dir = work_dir().join(etxt_common::per_process("obj-seh"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut v = Vec::new();
        for name in ["fle_seh.c", "fle_callback.c", "fle_proc.c", "fle_time.c", "fle_sys.c", "fle_imports.c", "fle_env.c"] {
            let o = dir.join(name.replace(".c", ".o"));
            clang_c(&runtime_dir().join(name), &o, true);
            v.push(o);
        }
        v
    })
}

/// The wrapper that fe-lift writes for a function with an except landing (tests/seh.rs pins the same text against the generator).
fn wrapper(va: u64) -> String {
    format!(
        "void f_{va:x}(void)\n{{\n    fle_seh_reg sr;\n    fle_seh_enter(&sr, UINT64_C({va:#x}));\n    if (FLE_SEH_SETJMP(&sr) == 0) {{ fb_{va:x}(); }} else {{ fle_seh_land(&sr); }}\n    fle_seh_leave(&sr);\n}}\n"
    )
}

const DRIVER_TOP: &str = r##"
#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <fcntl.h>
#include <unistd.h>
#include <sys/mman.h>
#include "fle_runtime.h"
#include "fle_sse.h"
#include "fle_dispatch.h"
#include "fle_seh.h"
#include "fle_callback.h"
#include "fle_x87ctl.h"
#include "fle_proc.h"

uint64_t R[16];
fle_eflags FL;
fle_xmm X[16];
fle_x87_state fle_x87 = { 0x37F, 0, 0xFFFF };

static unsigned g_thread;
unsigned fle_thread_current(void) { return g_thread; }
/* the harness's shadow stack depth (-finstrument-functions): +1 at a function entry and -1 at its normal exit; an exception skips the exits */
static size_t g_td;
size_t trace_depth(void) { return g_td; }
void trace_set_depth(size_t d) { g_td = d; }
#define ENTER() (g_td++)
#define LEAVE() (g_td--)

void fle_seh_set_images(const uint64_t *bases, unsigned n);

#define U(x) ((unsigned long long)(x))
#define IMG 0x400000ull
static char evlog[8192];
static void ev(const char *fmt, ...) __attribute__((format(printf, 1, 2)));
#include <stdarg.h>
static void ev(const char *fmt, ...)
{
    va_list ap; size_t n = strlen(evlog);
    va_start(ap, fmt); vsnprintf(evlog + n, sizeof evlog - n, fmt, ap); va_end(ap);
    strcat(evlog, "\n");
}

static uint32_t ld32(uint64_t a) { return (uint32_t)fle_ld(a, 4); }
static uint64_t ld64(uint64_t a) { return fle_ld(a, 8); }

static void map_image(const char *path)
{
    uint8_t hdr[0x1000];
    int fd = open(path, O_RDONLY);
    uint32_t lf, size, hsz; uint64_t ib; uint16_t ns, os; const uint8_t *pe; void *m; uint16_t i;
    if (fd < 0 || pread(fd, hdr, sizeof hdr, 0) < 0x400) { fprintf(stderr, "cannot read %s\n", path); exit(2); }
    memcpy(&lf, hdr + 0x3c, 4); pe = hdr + lf;
    memcpy(&ns, pe + 6, 2); memcpy(&os, pe + 20, 2);
    memcpy(&ib, pe + 24 + 24, 8); memcpy(&size, pe + 24 + 56, 4); memcpy(&hsz, pe + 24 + 60, 4);
    m = mmap((void *)(uintptr_t)ib, size, PROT_READ | PROT_WRITE | PROT_EXEC, MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED_NOREPLACE, -1, 0);
    if (m == MAP_FAILED || (uint64_t)(uintptr_t)m != ib) { fprintf(stderr, "mmap at the image base failed\n"); exit(2); }
    memcpy(m, hdr, hsz);
    for (i = 0; i < ns; i++) {
        const uint8_t *s = pe + 24 + os + 40u * i; uint32_t vs, va, rs, rp, span, copy;
        memcpy(&vs, s + 8, 4); memcpy(&va, s + 12, 4); memcpy(&rs, s + 16, 4); memcpy(&rp, s + 20, 4);
        span = vs ? vs : rs; copy = span < rs ? span : rs;
        if (copy && pread(fd, (uint8_t *)m + va, copy, rp) != (ssize_t)copy) { fprintf(stderr, "section read failed\n"); exit(2); }
    }
    close(fd);
}

/* ---- the guest stack and the lifted calling convention ---- */
static uint64_t stk[0x4000] __attribute__((aligned(4096)));
#define STK_TOP ((uint64_t)(uintptr_t)stk + sizeof stk - 0x100)
#define CALL(ret, f) do { R[4] -= 8; fle_st(R[4], 8, (ret)); f(); R[4] += 8; } while (0)
#define PUSH(r) do { R[4] -= 8; fle_st(R[4], 8, (r)); } while (0)
#define POP(r) do { (r) = fle_ld(R[4], 8); R[4] += 8; } while (0)

enum { M_NORMAL = 0, M_SEARCH_ONLY = 1 };
static int g_variant;       /* scenario switches, see main */
static uint64_t g_obj;      /* the exception object that the next raise passes */
static uint64_t g_est5df610, g_est63ade0, g_est5df570, g_landed;
static uint64_t objbuf[2][4];

void f_411d20(void); void f_411e80(void); void f_5df540(void); void f_5df570(void); void f_5df610(void); void fb_5df610(void);
void f_63ade0(void); void fb_63ade0(void); void f_5df673(void); void f_63ae4e(void); void f_5df720(void); void f_5df730(void); void f_4112c0(void);

/* kernel32!RaiseException as the import thunk calls it: rcx, rdx, r8, r9 and the guest return address at [R[4]] */
static uint64_t g_args[8];
static void raise_stub(void)
{
    R[1] = 0x0EEDFADE; R[2] = 1; R[8] = 8; R[9] = (uint64_t)(uintptr_t)g_args;
    if (g_variant == 8) fle_imp_RaiseException();   /* through fle_proc.c, as the import thunk calls it */
    else fle_seh_raise();
}

/* f_411d20: push rbp; push rdi; push rsi; push rbx; sub rsp,0x108; mov rbp,rsp (UNWIND_INFO: SET_FPREG, ALLOC_LARGE 0x108, PUSH rbx, rsi, rdi, rbp). It clobbers rbx, rsi, rdi. */
void f_411d20(void)
{
    ENTER();
    PUSH(R[5]); PUSH(R[7]); PUSH(R[6]); PUSH(R[3]);
    R[4] -= 0x108; R[5] = R[4];
    R[3] = 0xDEAD03; R[6] = 0xDEAD06; R[7] = 0xDEAD07;
    memset(g_args, 0, sizeof g_args);
    g_args[0] = 0x411e4d;                           /* args[0] = the address of the raise (the funnel's rsi); the value is not read by the handler */
    g_args[1] = g_obj;
    CALL(0x411e4d, raise_stub);
    R[4] = R[5] + 0x108; POP(R[3]); POP(R[6]); POP(R[7]); POP(R[5]);
    LEAVE();
}

/* f_411e80: push rbp; sub rsp,0x20; mov rbp,rsp */
void f_411e80(void)
{
    ENTER();
    PUSH(R[5]); R[4] -= 0x20; R[5] = R[4];
    CALL(0x411e94, f_411d20);
    R[4] = R[5] + 0x20; POP(R[5]);
    LEAVE();
}

/* f_5df540 and f_5df570: sub rsp,0x28 (no frame register) */
void f_5df540(void)
{
    fle_landing lj;
    fle_seh_reg fake;
    ENTER();
    R[4] -= 0x28;
    g_obj = (uint64_t)(uintptr_t)objbuf[0];
    fle_landing_push(&lj, 0x5df541, R[4]);                /* a decision-0100 landing registration of the C frame of this function: dropped with the frame by the unwind */
    if (g_variant == 7) fle_seh_enter(&fake, 0x5df610);   /* a NEWER registration of the same function with another entry rsp (a recursive instance): the unwinder must match the stack address too */
    CALL(0x5df56b, f_411e80);
    if (g_variant == 7) fle_seh_leave(&fake);
    fle_landing_pop(&lj);
    R[4] += 0x28;
    LEAVE();
}
void f_5df570(void)
{
    ENTER();
    R[4] -= 0x28;
    g_est5df570 = R[4];                                  /* its EstablisherFrame: no frame register, so the base of the fixed allocation = rsp in the body */
    CALL(0x5df582, f_5df540);
    R[4] += 0x28;
    LEAVE();
}

/* f_5df610: push rbp; add rsp,-0x80; mov rbp,rsp. try (0x5df63c..0x5df672): f_5df570; landing 0x5df673 raises a new exception from 0x5df6e2. */
void fb_5df610(void)
{
    ENTER();
    PUSH(R[5]); R[4] -= 0x80; R[5] = R[4];
    g_est5df610 = R[5];
    CALL(0x5df66f, f_5df570);
    R[4] = R[5] + 0x80; POP(R[5]);
    LEAVE();
}
void f_5df673(void)
{
    if (R[5] != g_est5df610) ev("landing 5df673 rbp WRONG %llx", U(R[5]));
    if (R[4] != R[5]) ev("landing 5df673 rsp WRONG %llx", U(R[4]));
    ev("landing 5df673 rax=%s", R[0] == (uint64_t)(uintptr_t)objbuf[0] ? "object1" : "WRONG");
    g_obj = (uint64_t)(uintptr_t)objbuf[1];
    CALL(0x5df6e7, f_411e80);                            /* the new exception (never returns) */
    R[4] = R[5] + 0x80; POP(R[5]);
}
/* the finally funclets of f_5df610 (scopes in table order): called by the handler with rcx = &EXCEPTION_POINTERS, rdx = the EstablisherFrame */
void f_5df720(void)
{
    ev("funclet 5df720 rdx=%s", R[2] == g_est5df610 ? "est(5df610)" : "WRONG");
    if (g_variant == 5) CALL(0x5df726, raise_stub);       /* a raise while the exception is being unwound */
}
void f_5df730(void)
{
    ev("funclet 5df730 rdx=%s", R[2] == g_est5df610 ? "est(5df610)" : "WRONG");
    if (g_variant == 6) g_thread = 5;                    /* the thread changed under the unwind */
}

/* f_63ade0: push rbp; sub rsp,0x50; mov rbp,rsp. try (0x63ae23..0x63ae4d): f_5df610; landing 0x63ae4e swallows and returns FALSE. */
void fb_63ade0(void)
{
    ENTER();
    PUSH(R[5]); R[4] -= 0x50; R[5] = R[4];
    g_est63ade0 = R[5];
    CALL(0x63ae33, f_5df610);
    R[4] = R[5] + 0x50; POP(R[5]);
    LEAVE();
}
void f_63ae4e(void)
{
    g_landed = 1;
    ev("landing 63ae4e rbp=%s rsp=%s rax=%s rbx=%llx rsi=%llx rdi=%llx r12=%llx", R[5] == g_est63ade0 ? "est(63ade0)" : "WRONG", R[4] == R[5] ? "=rbp" : "WRONG",
       R[0] == (uint64_t)(uintptr_t)objbuf[1] ? "object2" : "WRONG", U(R[3]), U(R[6]), U(R[7]), U(R[12]));
    R[0] = 0;                                            /* al = FALSE */
    R[4] = R[5] + 0x50; POP(R[5]);
}

__WRAPPERS__

/* ---- the handler 0x4112c0: a C port of sim_scopes.py (the reading of the Delphi handler), called by fle_seh.c with rcx = rec, rdx = EstablisherFrame, r8 = ctx, r9 = DC ---- */
static int pstr_eq(uint64_t a, uint64_t b) { unsigned n = (unsigned)fle_ld(a, 1), i; if (n != fle_ld(b, 1)) return 0; for (i = 1; i <= n; i++) if (fle_ld(a + i, 1) != fle_ld(b + i, 1)) return 0; return 1; }
static int class_matches(uint64_t cell, uint64_t obj_vmt)
{
    uint64_t want, cur;
    if (cell == 0) return 1;
    want = ld64(cell); cur = obj_vmt;
    while (cur) {
        uint64_t p;
        if (cur == want) return 1;
        if (ld32(cur - 0x80) == ld32(want - 0x80) && pstr_eq(ld64(cur - 0x88), ld64(want - 0x88))) return 1;
        p = ld64(cur - 0x78); cur = p ? ld64(p) : 0;
    }
    return 0;
}
static void call_unwind_ex(uint64_t est, uint64_t ip, uint64_t rec, uint64_t obj, uint64_t ctx)
{
    uint64_t s = R[4];
    fle_st(s + 0x20, 8, ctx); fle_st(s + 0x28, 8, 0);   /* arguments 5 and 6 above the shadow space */
    R[1] = est; R[2] = ip; R[8] = rec; R[9] = obj;
    R[4] -= 8; fle_st(R[4], 8, 0x4116ff);
    fle_seh_unwind_ex();
}
void f_4112c0(void)
{
    uint64_t rec = R[1], est = R[2], ctx = R[8], dc = R[9], saved_sp = R[4], pc, base, data, target, fva;
    uint32_t n, k, flags;
    R[4] -= 0x60;                                           /* the handler's own frame (sub rsp,0x128 in the real one) */
    pc = ld64(dc); base = ld64(dc + 8); data = ld64(dc + 0x38); target = ld64(dc + 0x20); fva = base + ld32(ld64(dc + 0x10));
    n = ld32(data); flags = ld32(rec + 4);
    if (g_variant == 1) { R[0] = 0; R[4] = saved_sp; return; }                    /* ExceptionContinueExecution */
    if (g_variant == 2) { R[0] = 3; R[4] = saved_sp; return; }                    /* ExceptionCollidedUnwind */
    if ((flags & 0x66) == 0) {
        for (k = 0; k < n; k++) {
            uint64_t sc = data + 4 + 16ull * k; uint32_t sb = ld32(sc), se = ld32(sc + 4), f2 = ld32(sc + 8), f3 = ld32(sc + 12);
            uint64_t obj = ld64(rec + 0x28), landing = 0;
            if (f2 == 0 || !(sb <= pc - base && pc - base < se)) continue;
            if (f2 == 2) landing = base + f3;
            else {
                uint32_t cnt = ld32(base + f2), j;
                for (j = 0; j < cnt && !landing; j++) {
                    uint32_t cell = ld32(base + f2 + 4 + 8ull * j), lnd = ld32(base + f2 + 8 + 8ull * j);
                    if (class_matches(cell ? base + cell : 0, ld64(obj))) landing = base + lnd;
                }
            }
            if (!landing) continue;
            ev("pass1 %llx pc=%llx matches -> %llx (nparams %u)", U(fva), U(pc), U(landing), ld32(rec + 0x18));
            if (ld32(rec + 0x18) > 7) fle_st(rec + 0x58, 8, est);                  /* the handler's own write of ExceptionInformation[7] */
            fle_st(rec + 4, 4, flags | 2);
            if (g_variant == 3) est += 8;                                           /* a TargetFrame that is not a frame */
            if (g_variant == 9) est = g_est5df570;                                  /* the TargetFrame is a frame without a frame register and without a handler: the unwinder must know its EstablisherFrame */
            if (g_variant == 4) g_thread = 5;                                      /* the thread changed before RtlUnwindEx */
            call_unwind_ex(est, landing, rec, obj, ctx);
        }
        ev("pass1 %llx pc=%llx continue", U(fva), U(pc));
        R[0] = 1; R[4] = saved_sp; return;
    }
    for (k = 0; k < n; k++) {
        uint64_t sc = data + 4 + 16ull * k; uint32_t sb = ld32(sc), se = ld32(sc + 4), f2 = ld32(sc + 8), f3 = ld32(sc + 12);
        if (!(sb <= pc - base && pc - base < se)) continue;
        if ((flags & 0x20) && sb <= target - base && target - base < se) break;       /* the scope that holds the landing: stop */
        if (f2 >= 2 && (flags & 0x20)) {                                                /* TARGET_UNWIND: the landing of this scope is recomputed and compared with TargetIp */
            uint64_t obj = ld64(rec + 0x28), landing = 0;
            if (f2 == 2) landing = base + f3;
            else {
                uint32_t cnt = ld32(base + f2), j;
                for (j = 0; j < cnt && !landing; j++) {
                    uint32_t cell = ld32(base + f2 + 4 + 8ull * j), lnd = ld32(base + f2 + 8 + 8ull * j);
                    if (class_matches(cell ? base + cell : 0, ld64(obj))) landing = base + lnd;
                }
            }
            if (landing == target) break;
        }
        if (f2 == 0) {
            R[1] = 0; R[2] = est;
            ev("pass2 pc=%llx funclet %llx", U(pc), U(base + f3));
            R[4] -= 8; fle_st(R[4], 8, 0x41189b);
            fle_dispatch(base + f3);
            R[4] += 8;
        }
    }
    ev("pass2 pc=%llx done%s", U(pc), (flags & 0x20) ? " (target)" : "");
    R[0] = 1; R[4] = saved_sp;
}

const fle_fn_entry fle_dispatch_table[] = {
    { 0x4112c0, f_4112c0 }, { 0x5df570, f_5df570 }, { 0x5df610, f_5df610 }, { 0x5df673, f_5df673 }, { 0x5df720, f_5df720 }, { 0x5df730, f_5df730 }, { 0x63ade0, f_63ade0 }, { 0x63ae4e, f_63ae4e },
};
const uint64_t fle_dispatch_count = sizeof fle_dispatch_table / sizeof fle_dispatch_table[0];

static const uint64_t BASES[1] = { IMG };
static void fresh(void)
{
    memset(stk, 0, sizeof stk); memset(evlog, 0, sizeof evlog);
    R[4] = STK_TOP; fle_seh_top = NULL; fle_seh_cur = NULL; g_landed = 0; g_thread = 0;
    objbuf[0][0] = ld64(0x5df54bull - 0x8b3);              /* the cell of EOleSysError's VMT, as in sim_scopes.py (raise 1) */
    objbuf[1][0] = ld64(0x5df6c4ull - 0xa2c);              /* (raise 2) */
    R[3] = 0xB1; R[5] = 0xB5; R[6] = 0xB2; R[7] = 0xB3; R[12] = 0xC12; R[13] = 0xC13; R[14] = 0xC14; R[15] = 0xC15;
}
static void callee_saved(const char *what)
{
    ev("%s: rbx=%llx rbp=%llx rsi=%llx rdi=%llx r12=%llx r13=%llx r14=%llx r15=%llx rsp=%s", what, U(R[3]), U(R[5]), U(R[6]), U(R[7]), U(R[12]), U(R[13]), U(R[14]), U(R[15]), R[4] == STK_TOP ? "entry" : "WRONG");
}

int main(int argc, char **argv)
{
    const char *sc = argc > 2 ? argv[2] : "wall16";
    if (argc < 3) return 2;
    map_image(argv[1]);
    fle_seh_set_images(BASES, 1);
    fresh();
    g_variant = argc > 3 ? atoi(argv[3]) : 0;
    if (!strcmp(sc, "wall16")) {
        R[4] -= 8; fle_st(R[4], 8, 0x1234);           /* the return address of f_63ade0: not an image address, never needed (the exception is caught inside) */
        f_63ade0();
        R[4] += 8;
        callee_saved("after f_63ade0");
        ev("rax=%llx landed=%llu registrations=%s landings=%s callback depth %u shadow depth %llu", U(R[0]), U(g_landed), fle_seh_top == NULL ? "none" : "LEFT", fle_landing_top == NULL ? "none" : "LEFT",
           fle_callback_depth(), U(g_td));
        ev("frames searched by the last pass 1: %u, handler calls of the last pass 2: %u", fle_seh_stat_frames(), fle_seh_stat_unwind_calls());
    } else if (!strcmp(sc, "unhandled")) {
        R[4] -= 8; fle_st(R[4], 8, 0);                 /* the bottom of the stack: return address 0 */
        f_5df570();
    } else if (!strcmp(sc, "callback")) {
        fle_call_guest(0x5df570, 0, 0, 0, 0);
    } else if (!strcmp(sc, "noreg")) {
        R[4] -= 8; fle_st(R[4], 8, 0x1234);
        f_63ade0_nowrap();
    } else if (!strcmp(sc, "nodispatch")) {
        R[1] = 0x1000; R[2] = 0x2000; R[8] = 0; R[9] = 0;
        fle_seh_unwind_ex();
    } else return 2;
    fputs(evlog, stdout);
    return 0;
}
"##;

struct Out {
    stdout: String,
    stderr: String,
    signal: Option<i32>,
    ok: bool,
}

fn run(scenario: &str, variant: u32) -> Option<Out> {
    let dll = flengine()?;
    let dir = work_dir();
    let name = etxt_common::uniq(&format!("seh-{scenario}-{variant}"));
    let c = dir.join(format!("{name}.c"));
    let exe = dir.join(&name);
    let mut wr = String::new();
    wr.push_str(&wrapper(0x5df610));
    wr.push_str(&wrapper(0x63ade0));
    // the 'noreg' scenario: f_63ade0 runs WITHOUT its wrapper, so the unwinder cannot find a registration for the target frame of the second exception
    wr.push_str("void f_63ade0_nowrap(void) { fb_63ade0(); }\n");
    let src = DRIVER_TOP.replace("__WRAPPERS__", &wr);
    std::fs::write(&c, src).unwrap();
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fPIE", "-I"]).arg(runtime_dir());
    for o in objs() {
        cmd.arg(o);
    }
    let cc = cmd.arg("-o").arg(&exe).arg(&c).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new(&exe).arg(&dll).arg(scenario).arg(variant.to_string()).env("TMPDIR", dir.join("tmp")).output().expect("run failed");
    let o = Out {
        stdout: String::from_utf8_lossy(&r.stdout).to_string(),
        stderr: String::from_utf8_lossy(&r.stderr).to_string(),
        signal: r.status.signal(),
        ok: r.status.success(),
    };
    let _ = std::fs::remove_file(&exe);
    let _ = std::fs::remove_file(&c);
    Some(o)
}

fn stops(scenario: &str, variant: u32, words: &[&str]) {
    let Some(o) = run(scenario, variant) else { return };
    assert!(!o.ok, "{scenario}/{variant}: did not stop: {}", o.stdout);
    assert_eq!(o.signal, Some(6), "{scenario}/{variant}: not SIGABRT: {}", o.stderr);
    let msg = o.stderr.lines().find(|l| l.starts_with("fle: ")).unwrap_or_else(|| panic!("{scenario}/{variant}: no fle: message: {}", o.stderr));
    for w in words {
        assert!(msg.contains(w), "{scenario}/{variant}: message lacks {w:?}: {msg}");
    }
}

#[test]
fn wall_16_two_raises_run_the_handler_funclets_and_landings_in_the_order_of_the_paper_reproduction() {
    let Some(o) = run("wall16", 0) else { return };
    assert!(o.ok, "{}\n{}", o.stderr, o.stdout);
    let l: Vec<&str> = o.stdout.lines().collect();
    // sim_scopes.py section 3.4: raise 1 is caught by `on EOleSysError` in f_5df610 (pc 0x5df66f, landing 0x5df673); the frames below it (f_411d20 .. f_5df570) have no handler;
    // the landing raises again (from 0x5df6e2, return 0x5df6e7); raise 2 passes f_5df610 (its handler says continue: the pc 0x5df6e7 is outside the `on` scope) and is caught by
    // `on Exception` in f_63ade0 (pc 0x63ae33, landing 0x63ae4e); in pass 2 the handler of f_5df610 runs the funclets 0x5df720 then 0x5df730 (table order), then the handler of f_63ade0 sees TARGET_UNWIND.
    let want = [
        "pass1 5df610 pc=5df66f matches -> 5df673 (nparams 8)",
        "pass2 pc=5df66f done (target)",
        "landing 5df673 rax=object1",
        "pass1 5df610 pc=5df6e7 continue",
        "pass1 63ade0 pc=63ae33 matches -> 63ae4e (nparams 8)",
        "pass2 pc=5df6e7 funclet 5df720",
        "funclet 5df720 rdx=est(5df610)",
        "pass2 pc=5df6e7 funclet 5df730",
        "funclet 5df730 rdx=est(5df610)",
        "pass2 pc=5df6e7 done",
        "pass2 pc=63ae33 done (target)",
    ];
    assert_eq!(l[..want.len()], want, "{l:?}");
    let rest = &l[want.len()..];
    assert_eq!(rest[0], "landing 63ae4e rbp=est(63ade0) rsp==rbp rax=object2 rbx=b1 rsi=b2 rdi=b3 r12=c12", "rbp, rsp, rax and the callee-saved registers that f_411d20 clobbered: {l:?}");
    assert_eq!(rest[1], "after f_63ade0: rbx=b1 rbp=b5 rsi=b2 rdi=b3 r12=c12 r13=c13 r14=c14 r15=c15 rsp=entry");
    // the C frames the unwind discarded took their bookkeeping with them: no registration, no decision-0100 landing, fle_call_guest's depth (the handler's calls were cut) and the shadow depth are back to 0
    assert_eq!(rest[2], "rax=0 landed=1 registrations=none landings=none callback depth 0 shadow depth 0");
    // raise 2 searched f_411d20, f_411e80, f_5df610, f_63ade0: 4 frames; its pass 2 called the handler for f_5df610 and for f_63ade0: 2 calls
    assert_eq!(rest[3], "frames searched by the last pass 1: 4, handler calls of the last pass 2: 2");
}

#[test]
fn an_exception_with_no_handler_in_any_frame_stops_by_name_with_the_frame_count() {
    stops("unhandled", 0, &["RaiseException", "exception code 0xeedfade", "flags 0x1", "8 argument(s)", "no frame-based handler took it", "4 frames searched"]);
}

#[test]
fn an_exception_that_reaches_a_host_callback_stops_by_name() {
    stops("callback", 0, &["RaiseException", "fle_call_guest", "host callback", "0xcb000000"]);
}

#[test]
fn a_target_frame_without_a_registration_stops_by_name_and_says_what_to_do() {
    stops("noreg", 0, &["RtlUnwindEx", "no landing registration", "--seh-handler", "0x63ae4e"]);
}

#[test]
fn a_newer_registration_of_the_same_function_at_another_stack_address_is_not_the_target() {
    let Some(o) = run("wall16", 7) else { return };
    assert!(o.ok, "{}\n{}", o.stderr, o.stdout);
    assert!(o.stdout.contains("rax=0 landed=1 registrations=none"), "{}", o.stdout);
}

#[test]
fn the_import_of_raise_exception_reaches_the_dispatcher_when_fle_seh_c_is_linked() {
    // fle_proc.c's fle_imp_RaiseException calls fle_seh_raise through a weak reference: the message is the one of the dispatcher, not the named stop of decision 0020 ("no structured exception handling is modelled")
    stops("unhandled", 8, &["RaiseException", "exception code 0xeedfade", "no frame-based handler took it", "4 frames searched"]);
    let Some(o) = run("unhandled", 8) else { return };
    assert!(!o.stderr.contains("no structured exception handling is modelled"), "{}", o.stderr);
}

#[test]
fn rtl_unwind_ex_outside_a_dispatch_stops_by_name() {
    stops("nodispatch", 0, &["RtlUnwindEx", "no exception being dispatched"]);
}

#[test]
fn the_dispositions_that_are_not_modelled_stop_by_name() {
    stops("wall16", 1, &["RaiseException", "ExceptionContinueExecution"]);
    stops("wall16", 2, &["RaiseException", "disposition 3"]);
}

#[test]
fn a_target_frame_that_is_not_a_frame_stops_by_name() {
    stops("wall16", 3, &["RtlUnwindEx", "passed TargetFrame"]);
}

#[test]
fn the_establisher_frame_of_a_function_without_a_frame_register_is_the_base_of_its_fixed_allocation() {
    // x64 exception handling page: "EstablisherFrame is the address of the base of the fixed stack allocation for this function"; f_5df570 has `sub rsp,0x28` and no frame register, so it is rsp in its body.
    // The handler names that frame as the target: the unwinder meets it (it has no registration, so it stops by name at that frame, not past it).
    stops("wall16", 9, &["RtlUnwindEx", "the function at 0x5df570", "no landing registration"]);
}

#[test]
fn a_raise_during_the_unwind_is_a_collided_exception_and_stops_by_name() {
    stops("wall16", 5, &["RaiseException", "another exception is being dispatched"]);
}

#[test]
fn a_thread_change_under_the_dispatch_or_the_unwind_is_detected_and_stops_by_name() {
    // variant 4: the thread id changes between pass 1 and RtlUnwindEx (the exception being dispatched belongs to the old thread)
    stops("wall16", 4, &["RtlUnwindEx", "another guest thread"]);
    // variant 6: it changes while the funclets run in pass 2; the registration of the target frame was made by the old thread: the jump would cross a thread switch
    stops("wall16", 6, &["RtlUnwindEx", "registration of the target frame belongs to another guest thread"]);
}

#[test]
fn the_per_thread_state_is_switched_with_the_thread_and_the_registrations_are_dropped_by_a_longjmp_of_decision_0100() {
    // source checks: the registration list head and the dispatch state are part of the context that fle_thread.c saves and loads at every switch, as fle_landing_top is
    let t = std::fs::read_to_string(runtime_dir().join("fle_thread.c")).unwrap();
    let save = &t[t.find("static void save_ctx").unwrap()..t.find("static void load_ctx").unwrap()];
    let load = &t[t.find("static void load_ctx").unwrap()..t.find("static void set_cur_pointers").unwrap()];
    for needle in ["c->seh = fle_seh_top;", "c->sehd = fle_seh_cur;"] {
        assert!(save.contains(needle), "save_ctx lacks {needle}");
    }
    for needle in ["fle_seh_top = c->seh;", "fle_seh_cur = c->sehd;"] {
        assert!(load.contains(needle), "load_ctx lacks {needle}");
    }
    // a decision-0100 landing remembers the registration list and gives it back when its longjmp drops the C frames in between
    let d = std::fs::read_to_string(runtime_dir().join("fle_dispatch.h")).unwrap();
    assert!(d.contains("l->seh = fle_seh_top;") && d.contains("fle_seh_top = l->seh;"));
}
