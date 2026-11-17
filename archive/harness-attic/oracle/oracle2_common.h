/* A5 oracle v2: pieces shared by native_drv2 and lift_drv2 (decision 0036). Differences from
 * oracle_common.h: the input carries an EFLAGS value, the output carries EFLAGS, MXCSR and the
 * x87 environment (fnstenv image), ORACLE_TRACE=1 single-steps the native call and reports the
 * executed instruction addresses plus a digest of (rip, GPR, XMM, status flags) at every step,
 * and ORACLE_PATCH=va:xor[,va:xor] XORs bytes of the mapped image (negative control without a
 * PE copy). Scratch fill modes 5 and 6 (pointer-structured data) are new. */
#ifndef ORACLE2_COMMON_H
#define ORACLE2_COMMON_H

#include <stddef.h>
#include <stdint.h>

#define OR_SCRATCH    UINT64_C(0x20000000)
#define OR_SCRATCH_SZ UINT64_C(0x100000)
#define OR_STACK      UINT64_C(0x30000000)
#define OR_STACK_SZ   UINT64_C(0x100000)
#define OR_STACK_TOP  (OR_STACK + UINT64_C(0xC0000))
#define OR_SENTINEL   UINT64_C(0xdead0000)
#define OR_TIMEOUT_S  2
#define OR_LIST       64
#define OR_MAX_STK    8
#define OR_MAX_SEC    32
#define OR_MAX_POKE   64
#define OR_TF         UINT64_C(0x100)
/* status flags CF PF AF ZF SF OF plus DF */
#define OR_FLMASK     UINT64_C(0xCD5)
#define OR_TRACE_SET  4096

/* gpr[] is in x86 encoding order (the lift R[]). Offsets used by ms_call2.S: xmm 128, rflags 384,
 * mxcsr 392, env 400 (28 bytes used by fnstenv: cw @0, sw @4, tw @8). */
typedef struct {
    uint64_t gpr[16];
    uint64_t xmm[16][2];
    uint64_t rflags;
    uint64_t mxcsr;
    uint8_t env[32];
    uint64_t flundef; /* lift only: flags the lifter models as undefined (bit positions of rflags) */
} oracle_cpu;

typedef struct {
    char id[64];
    uint64_t fn;
    uint32_t fill;
    uint64_t seed;
    uint64_t nstk;
    uint64_t stk[OR_MAX_STK];
    uint64_t npoke;
    uint64_t poke[OR_MAX_POKE][2]; /* 8-byte writes into scratch or a writable image section after the fill: addr, value */
    oracle_cpu in;
} oracle_case;

typedef struct {
    uint64_t va, size;
    int w, x;
} oracle_sec;

typedef struct {
    uint64_t base, size;
    uint64_t text_lo, text_hi;
    uint64_t wr_lo, wr_hi;
    int nsec;
    oracle_sec sec[OR_MAX_SEC];
} oracle_pe;

enum { OR_OK = 0, OR_FAULT = 1, OR_ABORT = 2, OR_TIMEOUT = 3, OR_NOENTRY = 4, OR_DIED = 5 };

extern char oracle_last_msg[256];
extern oracle_cpu oracle_ret_cpu;

typedef int (*oracle_exec_fn)(const oracle_case *c, const oracle_pe *pe, oracle_cpu *out);

int oracle_main(int argc, char **argv, const char *side, oracle_exec_fn exec);

#endif
