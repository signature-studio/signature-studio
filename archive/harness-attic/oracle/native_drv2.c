/* A5 oracle v2, native side: real machine code of FLEngine_x64.dll under the MS x64 ABI via
 * ms_call2.S. Records EFLAGS, MXCSR and the x87 environment at return. A case whose input
 * flags have TF set is single-stepped; the executed instruction addresses are reported.
 * Usage: native_drv2 CASES OUT PE   (env ORACLE_PATCH=va:xor[,va:xor])
 * ORACLE_RING_DIR=dir (with ORACLE_TRACE=1): the last OR_RING single-stepped states (rip and the 16 GPRs
 * before each instruction of the image) are written to dir/<case id>.ring when the call faults, aborts
 * or times out; used by a5f_repair.py to find which input made the faulting address. */
#define _GNU_SOURCE
#include <fcntl.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <ucontext.h>
#include <unistd.h>

#include "oracle2_common.h"

extern void oracle_ms_call(const oracle_cpu *in, uint64_t fn, uint64_t sp);
extern char oracle_ms_ret[];
uint64_t oracle_ret_target = OR_SENTINEL;

/* The return address the callee sees is OR_SENTINEL (as in lift_drv2). ORACLE_RETADDR=hex changes it
 * (page-aligned stub address; used to find functions whose result depends on the return address). */
static int map_ret_stub(void) {
    uint64_t at = getenv("ORACLE_RETADDR") ? strtoull(getenv("ORACLE_RETADDR"), NULL, 16) : OR_SENTINEL;
    uint8_t *p = mmap((void *)(uintptr_t)at, 4096, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED_NOREPLACE, -1, 0);
    uint64_t tgt = (uint64_t)(uintptr_t)oracle_ms_ret;
    if (p == MAP_FAILED || (uint64_t)(uintptr_t)p != at) { perror("mmap return stub"); return -1; }
    p[0] = 0xff; p[1] = 0x25; p[2] = p[3] = p[4] = p[5] = 0; /* jmp *0(%rip), target stored right after */
    memcpy(p + 6, &tgt, 8);
    if (mprotect(p, 4096, PROT_READ | PROT_EXEC) != 0) { perror("mprotect return stub"); return -1; }
    oracle_ret_target = at;
    return 0;
}

#define OR_RING 16384
typedef struct { uint64_t rip, gpr[16]; } ring_ent;
static ring_ent ring[OR_RING];
static uint64_t ring_n, ring_lo, ring_hi;
static char ring_path[600];
static struct sigaction old_sa[32];
static const int gidx[16] = {REG_RAX, REG_RCX, REG_RDX, REG_RBX, REG_RSP, REG_RBP, REG_RSI, REG_RDI,
                             REG_R8,  REG_R9,  REG_R10, REG_R11, REG_R12, REG_R13, REG_R14, REG_R15};

static void ring_trap(int sig, siginfo_t *si, void *uc) {
    ucontext_t *c = uc;
    uint64_t rip = (uint64_t)c->uc_mcontext.gregs[REG_RIP];
    ring_ent *e;
    (void)sig;
    (void)si;
    if (rip < ring_lo || rip >= ring_hi) return;
    e = &ring[ring_n % OR_RING];
    e->rip = rip;
    for (int i = 0; i < 16; i++) e->gpr[i] = (uint64_t)c->uc_mcontext.gregs[gidx[i]];
    ring_n++;
}

static void ring_dump(int sig, siginfo_t *si, void *uc) {
    int fd = open(ring_path, O_WRONLY | O_CREAT | O_TRUNC, 0644);
    if (fd >= 0) {
        uint64_t hdr[2] = {ring_n, ring_n < OR_RING ? ring_n : OR_RING};
        uint64_t first = ring_n % OR_RING;
        ssize_t r = write(fd, hdr, sizeof hdr);
        if (ring_n <= OR_RING) {
            r = write(fd, ring, ring_n * sizeof(ring_ent));
        } else {
            r = write(fd, ring + first, (OR_RING - first) * sizeof(ring_ent));
            r = write(fd, ring, first * sizeof(ring_ent));
        }
        (void)r;
        close(fd);
    }
    if (old_sa[sig].sa_flags & SA_SIGINFO) old_sa[sig].sa_sigaction(sig, si, uc);
    _exit(0);
}

static void ring_setup(const oracle_case *c, const oracle_pe *pe) {
    static const int sigs[] = {SIGSEGV, SIGBUS, SIGILL, SIGFPE, SIGABRT, SIGALRM};
    struct sigaction sa;
    const char *dir = getenv("ORACLE_RING_DIR");
    if (!dir) return;
    snprintf(ring_path, sizeof ring_path, "%s/%s.ring", dir, c->id);
    ring_lo = pe->base;
    ring_hi = pe->base + pe->size;
    memset(&sa, 0, sizeof sa);
    sa.sa_flags = SA_SIGINFO | SA_ONSTACK;
    sigemptyset(&sa.sa_mask);
    sa.sa_sigaction = ring_trap;
    sigaction(SIGTRAP, &sa, NULL);
    sa.sa_sigaction = ring_dump;
    for (unsigned i = 0; i < sizeof sigs / sizeof sigs[0]; i++) sigaction(sigs[i], &sa, &old_sa[sigs[i]]);
}

static int exec_native(const oracle_case *c, const oracle_pe *pe, oracle_cpu *out) {
    ring_setup(c, pe);
    oracle_ms_call(&c->in, c->fn, OR_STACK_TOP);
    *out = oracle_ret_cpu;
    out->flundef = 0;
    return 0;
}

int main(int argc, char **argv) {
    if (map_ret_stub() != 0) return 2;
    return oracle_main(argc, argv, "native", exec_native);
}
