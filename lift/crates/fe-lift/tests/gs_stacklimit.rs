//! `mov r64, gs:[0x10]` (NT_TIB.StackLimit) and the `__chkstk` of dsp_ippv2_x64.dll (decision 0076,
//! docs/investigations/teb-peb-model.md section 2, option A).
//!
//! The real function: RVA 0x17daba0 of dsp_ippv2_x64.dll, 78 bytes (CHKSTK below; if the DLL is at $DSP_IPPV2_PE or
//! lift/out-ext/dsp_ippv2_x64.dll the bytes are read from it and must equal the constant). They are placed in a synthetic PE
//! and lifted by the generator itself (the test does not hand-edit the C), then:
//!
//! 1. Generator: the `gs:[0x10]` load is lifted (`fle_gs_ld8(UINT64_C(0x10))`), no `fle_unsupported`; a store to it, a
//!    32-bit load, gs:[0x8] and gs:[0x18] stay unsupported.
//! 2. Host CPU: the ORIGINAL bytes run on the host with the GS base at a fake TEB whose +0x10 is the StackLimit, the generated
//!    C runs with `fle_teb_set_stack` (the runtime's own StackLimit), on identical memory. Compared: r10, r11, rsp, the six
//!    status flags and every byte of the region. StackLimit is the real bottom of the stack. (a) the memory below the stack
//!    is writable (what wasm gives: no guard), (b) it is PROT_NONE (what the native harness gives next to nothing mapped):
//!    a frame that fits writes nothing; a frame that crosses the bottom writes one zero byte per page below it.
//! 3. wasm (emcc, node; flags of lift/harness/wasm/build.sh): the same generated C gives the same results as the native
//!    generated C for case (a). Skipped, with a message, when emcc or node is not installed.
//! 4. Runtime: fle_call_guest keeps the StackLimit (same stack), a read with no registered stack, with rsp outside it, and of
//!    gs:[0x8] stop by name, and fle_teb_set_stack rounds an unaligned low end up to a page.
//!
//! Outputs go under lift/out-w7/test.

mod etxt_common;

use fe_lift::{generate, lift_body};
use std::path::PathBuf;
use std::process::Command;

const IB: u64 = 0x1_4000_0000;
const LFANEW: usize = 0x80;
const OPT: usize = LFANEW + 24;
const SEC: usize = OPT + 240;
const TEXT_RVA: u32 = 0x1000;
const TEXT_RAW: usize = 0x400;
const FILE_LEN: usize = 0x800;

/// dsp_ippv2_x64.dll 0x17daba0..=0x17dabed (read from the DLL with a script, 2026-10-23).
const CHKSTK_HEX: &str = "4883ec104c8914244c895c24084d33db4c8d5424184c2bd04d0f42d3654c8b1c25100000004d3bd37316664181e200f04d8d9b00f0ffff41c603004d3bd375f04c8b14244c8b5c24084883c410c3";

fn chkstk() -> Vec<u8> {
    let v: Vec<u8> = (0..CHKSTK_HEX.len() / 2).map(|k| u8::from_str_radix(&CHKSTK_HEX[2 * k..2 * k + 2], 16).unwrap()).collect();
    assert_eq!(v.len(), 0x4e);
    assert_eq!(v[0x4d], 0xc3);
    v
}

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

/// .text at 0x1000: main (calls both, so both are function entries), 0x1040 = the real __chkstk bytes,
/// 0x10a0 = `mov r11, gs:[0x10]; mov rax, r11; ret` (a callee that reads StackLimit).
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
    w32(&mut b, o + 36, 0);
    w32(&mut b, o + 56, 0x2000);
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
    let mut main = call(0x1000, 0x1040);
    main.extend(call(0x1005, 0x10a0));
    main.push(0xc3);
    let cb = vec![0x65, 0x4c, 0x8b, 0x1c, 0x25, 0x10, 0x00, 0x00, 0x00, 0x4c, 0x89, 0xd8, 0xc3];
    for (rva, bytes) in [(0x1000u32, main), (0x1040, chkstk()), (0x10a0, cb)] {
        let o = TEXT_RAW + (rva - TEXT_RVA) as usize;
        b[o..o + bytes.len()].copy_from_slice(&bytes);
    }
    b
}

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn rt() -> PathBuf {
    crate_dir().join("../../runtime")
}
fn out_dir() -> PathBuf {
    let d = crate_dir().join("../../out-w7/test");
    std::fs::create_dir_all(d.join("tmp")).expect("create test output dir");
    d
}

fn gen_c() -> String {
    let mut out = Vec::new();
    let st = generate(&build(), &mut out).expect("generate");
    assert_eq!(st.unsupported_total, 0, "{:?}", st.unsupported);
    String::from_utf8(out).unwrap()
}

#[test]
fn the_dll_bytes_are_the_constant() {
    let explicit = std::env::var_os("DSP_IPPV2_PE");
    let path = match &explicit {
        Some(p) => PathBuf::from(p),
        None => crate_dir().join("../../out-ext/dsp_ippv2_x64.dll"),
    };
    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(e) if explicit.is_some() => panic!("DSP_IPPV2_PE={}: {e}", path.display()),
        Err(e) => {
            eprintln!("SKIPPED: {}: {e} (the constant CHKSTK_HEX is used as it is)", path.display());
            return;
        }
    };
    let pe = fe_pe::Pe::parse(&data).expect("parse the PE");
    assert_eq!(pe.rva_to_va(0x17daba0), 0x1_8000_0000 + 0x17daba0);
    let real = pe.slice_at_rva(0x17daba0, 0x4e).expect("bytes of 0x17daba0");
    assert_eq!(real, &chkstk()[..], "the bytes of __chkstk in the DLL differ from CHKSTK_HEX");
}

#[test]
fn stacklimit_read_is_lifted_and_other_gs_forms_stay_unsupported() {
    let b = lift_body(&[0x65, 0x4c, 0x8b, 0x1c, 0x25, 0x10, 0x00, 0x00, 0x00], 0x1000).expect("mov r11, gs:[0x10]");
    assert!(b.contains("R[11] = fle_gs_ld8(UINT64_C(0x10));"), "{b}");
    let b = lift_body(&[0x65, 0x48, 0x8b, 0x04, 0x25, 0x10, 0x00, 0x00, 0x00], 0x1000).expect("mov rax, gs:[0x10]");
    assert!(b.contains("R[0] = fle_gs_ld8(UINT64_C(0x10));"), "{b}");
    let cases: [(&str, &[u8]); 7] = [
        ("mov gs:[0x10], rax (a store)", &[0x65, 0x48, 0x89, 0x04, 0x25, 0x10, 0x00, 0x00, 0x00]),
        ("mov eax, gs:[0x10] (32-bit)", &[0x65, 0x8b, 0x04, 0x25, 0x10, 0x00, 0x00, 0x00]),
        ("mov rax, gs:[0x8] (StackBase)", &[0x65, 0x48, 0x8b, 0x04, 0x25, 0x08, 0x00, 0x00, 0x00]),
        ("mov rax, gs:[0x18]", &[0x65, 0x48, 0x8b, 0x04, 0x25, 0x18, 0x00, 0x00, 0x00]),
        ("cmp r10, gs:[0x10] (ALU form)", &[0x65, 0x4c, 0x3b, 0x14, 0x25, 0x10, 0x00, 0x00, 0x00]),
        ("mov rax, fs:[0x10]", &[0x64, 0x48, 0x8b, 0x04, 0x25, 0x10, 0x00, 0x00, 0x00]),
        ("mov rax, gs:[0x60] (PEB: not modeled, decision 0076)", &[0x65, 0x48, 0x8b, 0x04, 0x25, 0x60, 0x00, 0x00, 0x00]),
    ];
    for (name, bytes) in cases {
        assert!(lift_body(bytes, 0x1000).is_err(), "{name} must stay unsupported");
    }
}

#[test]
fn generated_chkstk_has_the_gs_read_and_nothing_unsupported() {
    let c = gen_c();
    assert!(c.contains("R[11] = fle_gs_ld8(UINT64_C(0x10));"));
    assert!(!c.contains("fle_unsupported("));
}

/// 11 cases: (name, skew of the registered low end, rsp at the call, rax, below stays inside the 64 pages under the stack).
const MAIN_CHK: &str = r##"
#define _GNU_SOURCE
#include <setjmp.h>
#include <signal.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#ifndef __EMSCRIPTEN__
#include <sys/mman.h>
#include <sys/syscall.h>
#include <unistd.h>
#endif
#include "fle_runtime.h"
#include "fle_sse.h"

uint64_t fle_tls_values[8];
void f_140001040(void);

#define PAGE 4096ul
#define BELOW 64ul
#define STK 64ul
#define ULL(x) ((unsigned long long)(x))

typedef struct { const char *name; uint64_t skew, rsp_off, rax; int rw_ok; } kase;
static const kase K[] = {
    { "fit-small", 0, 0x20000, 0x1118, 1 },
    { "fit-largest-immediate-in-dll", 0, 0x20000, 0xa0b0, 1 },
    { "fit-exactly-to-bottom", 0, 0x2000, 0x2000, 1 },
    { "alloc-zero", 0, 0x2000, 0, 1 },
    { "fit-near-bottom", 0, 0x400, 0x10, 1 },
    { "straddle-1-byte", 0, 0x2000, 0x2001, 1 },
    { "straddle-1-page", 0, 0x2000, 0x3000, 1 },
    { "straddle-3-pages", 0, 0x2000, 0x5000, 1 },
    { "straddle-from-0x800", 0, 0x800, 0x1118, 1 },
    { "unaligned-registered-low-end", 0x10, 0x4000, 0x4800, 1 },
    { "borrow-huge", 0, 0x2000, 0xffffffffffffff00ull, 0 },
};
#define NK (sizeof K / sizeof K[0])

typedef struct { int fault; uint64_t r10, r11, rsp_off, flags; int64_t fault_page; } res_t;

static uint8_t pat(uint64_t i) { return (uint8_t)(1 + ((i * 131u >> 3) % 250)); }
static void fill(uint8_t *p) { uint64_t i; for (i = 0; i < (BELOW + STK) * PAGE; i++) p[i] = pat(i); }

static uint64_t limit_of(void) { uint64_t v; memcpy(&v, fle_teb + FLE_TEB_STACKLIMIT, 8); return v; }

#ifndef __EMSCRIPTEN__
static sigjmp_buf g_sj;
static volatile uint64_t g_sig_addr;
static void on_segv(int s, siginfo_t *si, void *u) { (void)s; (void)u; g_sig_addr = (uint64_t)(uintptr_t)si->si_addr; siglongjmp(g_sj, 1); }
static uint8_t fake_teb[4096] __attribute__((aligned(4096)));
static uint8_t *g_code;
#endif

static uint8_t *region(int guard)
{
    uint8_t *p;
#ifdef __EMSCRIPTEN__
    p = aligned_alloc(4096, (BELOW + STK) * PAGE);
    (void)guard;
#else
    p = mmap(NULL, (BELOW + STK) * PAGE, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    if (p == MAP_FAILED) { perror("mmap"); exit(2); }
#endif
    if (!p) { perror("region"); exit(2); }
    fill(p);
#ifndef __EMSCRIPTEN__
    if (guard) mprotect(p, BELOW * PAGE, PROT_NONE);
#endif
    return p;
}

static void unregion(uint8_t *p)
{
#ifdef __EMSCRIPTEN__
    free(p);
#else
    munmap(p, (BELOW + STK) * PAGE);
#endif
}

/* the generated C, with the runtime's own StackLimit */
static res_t run_lifted(uint8_t *base, const kase *k)
{
    res_t r = { 0 };
    uint8_t *lo = base + BELOW * PAGE;
    R[0] = k->rax; R[10] = 0x1010; R[11] = 0x1111;
    memset(&FL, 0, sizeof FL);
    fle_teb_set_stack((uint64_t)(uintptr_t)lo + k->skew, (uint64_t)(uintptr_t)lo + STK * PAGE);
    R[4] = (uint64_t)(uintptr_t)lo + k->rsp_off - 8;
    fle_st(R[4], 8, UINT64_C(0xdead0000));
#ifndef __EMSCRIPTEN__
    if (sigsetjmp(g_sj, 1) != 0) {
        r.fault = 1;
        r.fault_page = (int64_t)(g_sig_addr - (uint64_t)(uintptr_t)lo) >> 12;
        return r;
    }
#endif
    f_140001040();
    r.r10 = R[10]; r.r11 = R[11]; r.rsp_off = R[4] + 8 - (uint64_t)(uintptr_t)lo;
    r.flags = (uint64_t)FL.cf << 0 | (uint64_t)FL.pf << 2 | (uint64_t)FL.af << 4 | (uint64_t)FL.zf << 6 | (uint64_t)FL.sf << 7 | (uint64_t)FL.of << 8;
    return r;
}

#ifndef __EMSCRIPTEN__
/* the original bytes on the host CPU: GS base = fake TEB whose +0x10 is the StackLimit the runtime holds */
static res_t run_native(uint8_t *base, const kase *k, uint64_t limit_off)
{
    res_t r = { 0 };
    uint8_t *lo = base + BELOW * PAGE;
    register uint64_t r10v asm("r10") = 0x1010;
    register uint64_t r11v asm("r11") = 0x1111;
    register uint64_t o13 asm("r13") = 0;
    register uint64_t fl asm("rcx") = 0;
    register uint64_t sp14 asm("r14") = (uint64_t)(uintptr_t)lo + k->rsp_off;
    register uint64_t code15 asm("r15") = (uint64_t)(uintptr_t)g_code;
    uint64_t rax_in = k->rax;
    *(uint64_t *)(fake_teb + 0x10) = (uint64_t)(uintptr_t)lo + limit_off;
    *(uint64_t *)(fake_teb + 0x8) = (uint64_t)(uintptr_t)lo + STK * PAGE;
    syscall(SYS_arch_prctl, 0x1001, (unsigned long)fake_teb);
    if (sigsetjmp(g_sj, 1) == 0) {
        r10v = 0x1010;
        r11v = 0x1111;
        __asm__ volatile(
            "mov %%rsp, %%r12\n\t"
            "mov %%r14, %%rsp\n\t"
            "call *%%r15\n\t"
            "mov %%rsp, %%r13\n\t"
            "mov %%r12, %%rsp\n\t"
            "seto %%dl\n\t"
            "lahf\n\t"
            "shr $8, %%eax\n\t"
            "movzx %%al, %%ecx\n\t"
            "movzx %%dl, %%edx\n\t"
            "shl $8, %%edx\n\t"
            "or %%edx, %%ecx\n\t"
            : "+r"(r10v), "+r"(r11v), "+a"(rax_in), "+r"(o13), "+r"(fl), "+r"(sp14), "+r"(code15)
            :
            : "rdx", "r12", "memory", "cc");
        r.r10 = r10v; r.r11 = r11v; r.rsp_off = o13 - (uint64_t)(uintptr_t)lo; r.flags = fl;
    } else {
        r.fault = 1;
        r.fault_page = (int64_t)(g_sig_addr - (uint64_t)(uintptr_t)lo) >> 12;
    }
    return r;
}
#endif

static void summarize(const uint8_t *base, const kase *k, char *buf, size_t n)
{
    uint64_t i, slot_lo = BELOW * PAGE + k->rsp_off - 0x18, slot_hi = BELOW * PAGE + k->rsp_off;
    unsigned below = 0, extra = 0;
    int64_t first = 0, last = 0;
    for (i = 0; i < (BELOW + STK) * PAGE; i++) {
        if (base[i] == pat(i)) continue;
        if (i >= slot_lo && i < slot_hi) continue;
        if (i < BELOW * PAGE) {
            if (below++ == 0) first = (int64_t)i - (int64_t)(BELOW * PAGE);
            last = (int64_t)i - (int64_t)(BELOW * PAGE);
        } else extra++;
    }
    snprintf(buf, n, "below=%u first=%lld last=%lld extra=%u", below, (long long)first, (long long)last, extra);
}

static int same_res(const res_t *a, const res_t *b)
{
    if (a->fault != b->fault) return 0;
    if (a->fault) return a->fault_page == b->fault_page;
    return a->r10 == b->r10 && a->r11 == b->r11 && a->rsp_off == b->rsp_off && (a->flags & 0x1d5) == (b->flags & 0x1d5);
}

static void line(const char *tag, const kase *k, const res_t *r, const char *sum)
{
    if (r->fault) printf("%s %s fault@page%lld\n", tag, k->name, (long long)r->fault_page);
    else printf("%s %s r10=%#llx r11=%#llx rsp_off=%#llx flags=%#llx %s\n", tag, k->name, ULL(r->r10), ULL(r->r11), ULL(r->rsp_off), ULL(r->flags & 0x1d5), sum);
}

int main(int argc, char **argv)
{
    const char *mode = argc > 1 ? argv[1] : "lifted-rw";
    size_t c;
    int fails = 0;
#ifndef __EMSCRIPTEN__
    static const uint8_t CODE[] = { @CODE@ };
    static char alt[65536];
    stack_t ss = { .ss_sp = alt, .ss_size = sizeof alt };
    struct sigaction sa;
    g_code = mmap(NULL, 4096, PROT_READ | PROT_WRITE | PROT_EXEC, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    memcpy(g_code, CODE, sizeof CODE);
    sigaltstack(&ss, NULL);
    memset(&sa, 0, sizeof sa);
    sa.sa_sigaction = on_segv; sa.sa_flags = SA_SIGINFO | SA_ONSTACK | SA_NODEFER;
    sigaction(SIGSEGV, &sa, NULL);
#endif
    if (strcmp(mode, "lifted-rw") == 0) {
        for (c = 0; c < NK; c++) {
            char sum[96];
            uint8_t *b;
            res_t r;
            if (!K[c].rw_ok) continue;
            b = region(0);
            r = run_lifted(b, &K[c]);
            summarize(b, &K[c], sum, sizeof sum);
            line("LIFT", &K[c], &r, sum);
            unregion(b);
        }
        return 0;
    }
#ifndef __EMSCRIPTEN__
    if (strcmp(mode, "native") == 0) {
        int guard;
        for (guard = 0; guard < 2; guard++) {
            for (c = 0; c < NK; c++) {
                uint8_t *a, *b;
                res_t rn, rl;
                char sum[96];
                int ok, mem_ok = 1;
                uint64_t i, slot = BELOW * PAGE + K[c].rsp_off;
                if (!guard && !K[c].rw_ok) continue;
                a = region(guard); b = region(guard);
                rl = run_lifted(b, &K[c]);
                rn = run_native(a, &K[c], limit_of() - (uint64_t)(uintptr_t)(b + BELOW * PAGE));
                for (i = 0; i < (BELOW + STK) * PAGE; i++) {
                    if (i >= slot - 8 && i < slot) continue;
                    if (guard && i < BELOW * PAGE) continue;
                    if (a[i] != b[i]) mem_ok = 0;
                }
                ok = same_res(&rn, &rl) && mem_ok;
                summarize(b, &K[c], sum, sizeof sum);
                if (guard) printf("%s %s hw=%s lifted=%s page=%lld %s\n", ok ? "SAME" : "DIFF", K[c].name, rn.fault ? "fault" : "ret", rl.fault ? "fault" : "ret", (long long)rl.fault_page, ok ? "" : "(mismatch)");
                else { printf("%s ", ok ? "HW-SAME" : "HW-DIFF"); line("LIFT", &K[c], &rl, sum); }
                if (!ok) fails++;
                unregion(a); unregion(b);
            }
        }
        return fails ? 1 : 0;
    }
#endif
    fprintf(stderr, "unknown mode %s\n", mode);
    return 3;
}
"##;

/// Needs the runtime (fle_callback.c, fle_env.c): fle_call_guest and the stop paths.
const MAIN_CB: &str = r##"
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include "fle_runtime.h"
#include "fle_sse.h"
#include "fle_callback.h"

void f_1400010a0(void);

#define SZ (UINT64_C(1) << 20)
#define ULL(x) ((unsigned long long)(x))

int main(int argc, char **argv)
{
    const char *m = argc > 1 ? argv[1] : "";
    uint8_t *stk = mmap(NULL, SZ, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS | MAP_NORESERVE, -1, 0);
    uint64_t lo = (uint64_t)(uintptr_t)stk, hi = lo + SZ;
    if (stk == MAP_FAILED) return 2;
    if (!strcmp(m, "nostack")) { R[4] = hi - 64; f_1400010a0(); return 0; }
    if (!strcmp(m, "badset")) { fle_teb_set_stack(lo, lo); return 0; }
    fle_teb_set_stack(lo, hi);
    if (!strcmp(m, "rsp_below")) { R[4] = lo - 0x100; f_1400010a0(); return 0; }
    if (!strcmp(m, "rsp_above")) { R[4] = hi + 0x100; f_1400010a0(); return 0; }
    if (!strcmp(m, "gs8")) { R[4] = hi - 64; (void)fle_gs_ld8(0x8); return 0; }
    if (!strcmp(m, "round")) {
        uint64_t base, limit;
        fle_teb_set_stack(lo + 0x10, hi);
        memcpy(&base, fle_teb + 8, 8); memcpy(&limit, fle_teb + 0x10, 8);
        printf("base_ok=%d limit_ok=%d\n", base == hi, limit == lo + 0x1000);
        return 0;
    }
    if (!strcmp(m, "callback")) {
        uint64_t r, self, via_self, base_self;
        R[4] = hi - 64;
        /* the callee reads gs:[0x10] while running on the callback's stack: the same stack as the caller (decision 0064) */
        r = fle_call_guest(UINT64_C(0x1400010a0), 0, 0, 0, 0);
        R[4] = hi - 64;
        self = fle_gs_ld8(0x30);
        memcpy(&via_self, (const void *)(uintptr_t)(self + 0x10), 8);
        memcpy(&base_self, (const void *)(uintptr_t)(self + 0x8), 8);
        printf("callback_limit_ok=%d via_self_ok=%d base_ok=%d rsp_restored=%d\n", r == lo, via_self == lo, base_self == hi, R[4] == hi - 64);
        return 0;
    }
    return 3;
}
"##;

fn clang(name: &str, src: &str, main_src: &str, extra: &[PathBuf]) -> PathBuf {
    let dir = out_dir();
    let u = etxt_common::uniq(name);
    let gc = dir.join(format!("{u}.c"));
    let mc = dir.join(format!("{u}_main.c"));
    let exe = dir.join(&u);
    std::fs::write(&gc, src).unwrap();
    std::fs::write(&mc, main_src).unwrap();
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-ffp-contract=off", "-fPIE", "-I"]).arg(rt()).arg("-o").arg(&exe).arg(&gc).arg(&mc);
    for f in extra {
        cmd.arg(f);
    }
    cmd.arg("-lm");
    let cc = cmd.env("TMPDIR", dir.join("tmp")).output().expect("run clang (is clang installed?)");
    assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
    exe
}

fn main_chk_src() -> String {
    let bytes: Vec<String> = chkstk().iter().map(|b| format!("{b:#04x}")).collect();
    MAIN_CHK.replace("@CODE@", &bytes.join(", "))
}

fn chk_exe() -> &'static (PathBuf, String) {
    static ONCE: std::sync::OnceLock<(PathBuf, String)> = std::sync::OnceLock::new();
    ONCE.get_or_init(|| {
        let c = gen_c();
        (clang("gs-chk", &c, &main_chk_src(), &[]), c)
    })
}

fn run_ok(exe: &PathBuf, mode: &str) -> String {
    let r = Command::new(exe).arg(mode).output().expect("run");
    let o = String::from_utf8_lossy(&r.stdout).to_string();
    assert!(r.status.success(), "{mode}: {o}{}", String::from_utf8_lossy(&r.stderr));
    o
}

fn lines_with<'a>(out: &'a str, prefix: &str) -> Vec<&'a str> {
    out.lines().filter(|l| l.starts_with(prefix)).collect()
}

#[test]
fn chkstk_matches_the_host_cpu_with_the_real_stack_bottom_as_stacklimit() {
    let (exe, _) = chk_exe();
    let o = run_ok(exe, "native");
    assert!(!o.contains("DIFF"), "{o}");
    // (a) memory below the stack writable: 10 cases, hardware and generated C agree on registers, flags, memory
    let rw: Vec<&str> = lines_with(&o, "HW-SAME ");
    assert_eq!(rw.len(), 10, "{o}");
    let want = |name: &str, tail: &str| {
        let l = rw.iter().find(|l| l.contains(&format!(" LIFT {name} "))).unwrap_or_else(|| panic!("{name}: {o}"));
        assert!(l.ends_with(tail), "{name}: {l}");
    };
    // a frame that fits: nothing is written anywhere, the stack pointer comes back
    for n in ["fit-small", "fit-largest-immediate-in-dll", "fit-exactly-to-bottom", "alloc-zero", "fit-near-bottom"] {
        want(n, "below=0 first=0 last=0 extra=0");
    }
    // a frame across the bottom: one zero byte per page below the limit, at limit-0x1000, -0x2000 ... (fail-open)
    want("straddle-1-byte", "below=1 first=-4096 last=-4096 extra=0");
    want("straddle-1-page", "below=1 first=-4096 last=-4096 extra=0");
    want("straddle-3-pages", "below=3 first=-12288 last=-4096 extra=0");
    want("straddle-from-0x800", "below=1 first=-4096 last=-4096 extra=0");
    // the registered low end was lo+0x10, StackLimit = lo+0x1000: the first probe is lo+0 (inside the stack's pages)
    want("unaligned-registered-low-end", "below=1 first=-4096 last=-4096 extra=1");
    // (b) PROT_NONE below the stack: fits return, every crossing faults at the first page below
    let g: Vec<&str> = lines_with(&o, "SAME ");
    assert_eq!(g.len(), 11, "{o}");
    for l in &g {
        let fits = ["fit-small", "fit-largest", "fit-exactly", "alloc-zero", "fit-near"].iter().any(|n| l.contains(&format!(" {n}")));
        if fits {
            assert!(l.contains("hw=ret lifted=ret"), "{l}");
        } else {
            assert!(l.contains("hw=fault lifted=fault page=-1"), "{l}");
        }
    }
}

#[test]
fn wasm_gives_the_same_results_as_native_for_the_generated_chkstk() {
    let emcc = PathBuf::from("/opt/emsdk/upstream/emscripten/emcc");
    let have_node = Command::new("node").arg("--version").output().map(|o| o.status.success()).unwrap_or(false);
    if !emcc.exists() || !have_node {
        eprintln!("SKIPPED: emcc ({}) or node is not installed", emcc.display());
        return;
    }
    let (exe, c) = chk_exe();
    let native = lines_with(&run_ok(exe, "native"), "HW-SAME LIFT ").iter().map(|l| l.trim_start_matches("HW-SAME ").to_string()).collect::<Vec<_>>();
    assert_eq!(native.len(), 10);
    let dir = out_dir();
    let u = etxt_common::uniq("gs-chk-wasm");
    let gc = dir.join(format!("{u}.c"));
    let mc = dir.join(format!("{u}_main.c"));
    let js = dir.join(format!("{u}.js"));
    // decision 0243: in a wasm build R, FL and X are the thread-local block fle_ctx (the generated C of the wasm stages is wrapped by lowregs.py); here the definitions of the generated unit are replaced by that block
    let mut cw = String::new();
    for l in c.lines() {
        if l == "uint64_t R[16];" {
            cw.push_str("_Thread_local unsigned char fle_ctx[0x300] __attribute__((aligned(64), used));\n");
        } else if l.starts_with("fle_eflags FL") || l == "extern fle_eflags FL;" || l == "extern uint64_t R[16];" || l.starts_with("fle_xmm X") || l == "extern fle_xmm X[16];" {
            continue;
        } else {
            cw.push_str(l);
            cw.push('\n');
        }
    }
    std::fs::write(&gc, cw).unwrap();
    std::fs::write(&mc, main_chk_src()).unwrap();
    let path = format!("/opt/emsdk:/opt/emsdk/upstream/emscripten:{}", std::env::var("PATH").unwrap_or_default());
    let cc = Command::new(&emcc)
        .args(["-O0", "-std=gnu11", "-w", "-fno-builtin", "-ffp-contract=off", "-I"])
        .arg(rt())
        .args(["-sALLOW_MEMORY_GROWTH=1", "-sSTACK_SIZE=8MB", "-sENVIRONMENT=node", "-sEXIT_RUNTIME=1", "-o"])
        .arg(&js)
        .arg(&gc)
        .arg(&mc)
        .arg(rt().join("fle_host.c")) // fle_addr_fault: wasm32 builds check guest addresses (fle_runtime.h)
        .env("PATH", path)
        .env("TMPDIR", dir.join("tmp"))
        .env("EM_CACHE", crate_dir().join("../../out-w7/emcache"))
        .output()
        .expect("run emcc");
    assert!(cc.status.success(), "emcc failed: {}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new("node").arg(&js).arg("lifted-rw").output().expect("run node");
    let o = String::from_utf8_lossy(&r.stdout).to_string();
    assert!(r.status.success(), "{o}{}", String::from_utf8_lossy(&r.stderr));
    let wasm: Vec<String> = lines_with(&o, "LIFT ").iter().map(|s| s.to_string()).collect();
    assert_eq!(wasm, native, "wasm differs from native");
    // wasm has no guard page: the crossing frames wrote their zero bytes below the stack and went on (fail-open)
    assert!(wasm.iter().any(|l| l.contains("straddle-3-pages") && l.ends_with("below=3 first=-12288 last=-4096 extra=0")), "{o}");
}

#[test]
fn runtime_stop_paths_and_callback_keep_the_stack_limit() {
    let dir = out_dir();
    // fle_imports.c defines fle_tls_values and fle_set_last_error (fle_env.c needs it); fle_host.c has the address-fault stop
    let mut sources: Vec<PathBuf> = ["fle_callback.c", "fle_env.c", "fle_imports.c", "fle_host.c"].iter().map(|f| rt().join(f)).collect();
    sources.sort();
    let c = gen_c();
    let exe = clang("gs-cb", &c, MAIN_CB, &sources);
    let run = |m: &str| Command::new(&exe).arg(m).env("TMPDIR", dir.join("tmp")).output().expect("run");
    let o = run("callback");
    let so = String::from_utf8_lossy(&o.stdout).to_string();
    assert!(o.status.success(), "{so}{}", String::from_utf8_lossy(&o.stderr));
    assert_eq!(so.trim(), "callback_limit_ok=1 via_self_ok=1 base_ok=1 rsp_restored=1");
    let o = run("round");
    assert_eq!(String::from_utf8_lossy(&o.stdout).trim(), "base_ok=1 limit_ok=1", "{}", String::from_utf8_lossy(&o.stderr));
    for (mode, msg) in [
        ("nostack", "no guest stack is registered"),
        ("badset", "has no whole page"),
        ("rsp_below", "outside the registered stack"),
        ("rsp_above", "outside the registered stack"),
        ("gs8", "GS:[0x8] is not modeled"),
    ] {
        let o = run(mode);
        let e = String::from_utf8_lossy(&o.stderr).to_string();
        assert!(!o.status.success(), "{mode} did not stop");
        assert!(e.contains(msg), "{mode}: {e}");
    }
}
