/* A5 oracle: pieces shared by the native driver and the lift driver. Each driver is a
 * separate PIE executable. It maps FLEngine_x64.dll at ImageBase (harness/image.c, so the
 * IAT slots hold their own slot VA on both sides), runs each case of a case file in a forked
 * child, and writes one JSON line per case. Not part of the generated output.
 * See docs/decisions/0035-a5-oracle.md. */
#ifndef ORACLE_COMMON_H
#define ORACLE_COMMON_H

#include <stddef.h>
#include <stdint.h>

#define OR_SCRATCH    UINT64_C(0x20000000)
#define OR_SCRATCH_SZ UINT64_C(0x100000)
#define OR_STACK      UINT64_C(0x30000000)
#define OR_STACK_SZ   UINT64_C(0x100000)
/* rsp at the call instruction (16-byte aligned). [S, S+32) is the shadow space, stack
 * arguments start at S+32, the return address slot is S-8. */
#define OR_STACK_TOP  (OR_STACK + UINT64_C(0xC0000))
#define OR_SENTINEL   UINT64_C(0xdead0000)
#define OR_TIMEOUT_S  2
#define OR_LIST       64
#define OR_MAX_STK    8
#define OR_MAX_SEC    32

/* gpr[] is in x86 encoding order: rax rcx rdx rbx rsp rbp rsi rdi r8..r15 (the lift R[]). */
typedef struct {
    uint64_t gpr[16];
    uint64_t xmm[16][2];
} oracle_cpu;

typedef struct {
    char id[64];
    uint64_t fn;
    uint32_t fill;
    uint64_t seed;
    uint64_t nstk;
    uint64_t stk[OR_MAX_STK];
    oracle_cpu in;
} oracle_case;

typedef struct {
    uint64_t va, size;
    int w, x;
} oracle_sec;

typedef struct {
    uint64_t base, size;
    uint64_t text_lo, text_hi;
    uint64_t wr_lo, wr_hi; /* VA hull of the sections with IMAGE_SCN_MEM_WRITE, page rounded */
    int nsec;
    oracle_sec sec[OR_MAX_SEC];
} oracle_pe;

enum { OR_OK = 0, OR_FAULT = 1, OR_ABORT = 2, OR_TIMEOUT = 3, OR_NOENTRY = 4, OR_DIED = 5 };

/* The lift driver's fprintf/fputs wrappers store the runtime's last message here. */
extern char oracle_last_msg[256];
extern oracle_cpu oracle_ret_cpu;

/* exec: runs in the child, after memory is prepared. Returns 0 when the function returned
 * (out filled), 1 when this side has no entry for c->fn. */
typedef int (*oracle_exec_fn)(const oracle_case *c, const oracle_pe *pe, oracle_cpu *out);

int oracle_main(int argc, char **argv, const char *side, oracle_exec_fn exec);

#endif
