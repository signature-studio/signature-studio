/* VEX (AVX) moves and the FS/GS segment model for fe-lift output (vex.rs). Header only,
 * included at the end of fle_sse.h. Semantics: Intel SDM Vol. 2 (325383-093US), the
 * VMOVUPS, VMOVAPS, VMOVNTPS and VZEROUPPER pseudo-code. See docs/decisions/0032-vex-fsgs.md.
 *
 * State: YMMn is the pair (X[n], YH[n]): X[n] is bits 127:0 (the XMM register, shared with
 * the SSE helpers) and YH[n] is bits 255:128. Legacy SSE instructions write X only, so
 * YH is unchanged by them (SDM: "Bits (MAXVL-1:128) ... remain unchanged"). A VEX.128
 * instruction that writes a register zeroes YH of that register; a VEX.256 instruction
 * writes both halves. ZMM and the AVX-512 state above bit 255 are not modeled.
 * The SSE/AVX transition penalty state of real hardware is not modeled (it is not
 * architectural). */
#ifndef FLE_VEX_H
#define FLE_VEX_H

#include <stdint.h>

#include "fle_runtime.h"

/* Weak definition: every translation unit that includes this header carries the same
 * symbol, and the linker keeps one. */
fle_xmm YH[16] __attribute__((weak)) = { 0 };

static inline void fle_misaligned32(uint64_t a) {
    fprintf(stderr, "fle: misaligned 32-byte access at %#llx\n", (unsigned long long)a);
    abort();
}

/* A VMOVAPS or VMOVNTPS memory operand must be 16-byte (VEX.128) or 32-byte (VEX.256)
 * aligned, else #GP (SDM Table 2-18, Type 1: "VEX.256: Memory operand is not 32-byte
 * aligned. VEX.128: Memory operand is not 16-byte aligned."). VMOVUPS has no such check. */
static inline void fle_vcheck(uint64_t a, int wide, int aligned) {
    if (!aligned) return;
    if (wide) {
        if (a & 31) fle_misaligned32(a);
    } else {
        if (a & 15) fle_misaligned(a);
    }
}

/* Load xmm (wide = 0: DEST[127:0] := SRC, DEST[MAXVL-1:128] := 0) or ymm (wide = 1:
 * DEST[255:0] := SRC, upper zero) register d from memory at a. */
static inline void fle_vld(int d, uint64_t a, int wide, int aligned) {
    fle_xmm lo, hi;
    fle_vcheck(a, wide, aligned);
    lo = fle_ldx(a);
    if (wide) {
        hi = fle_ldx(a + 16);
    } else {
        hi.u64[0] = 0;
        hi.u64[1] = 0;
    }
    X[d] = lo;
    YH[d] = hi;
}

/* Store register s to memory at a: 128 bits, or 256 bits when wide. No register changes. */
static inline void fle_vst(uint64_t a, int s, int wide, int aligned) {
    fle_vcheck(a, wide, aligned);
    fle_stx(a, X[s]);
    if (wide) fle_stx(a + 16, YH[s]);
}

/* Register copy d := s (SDM "Load - and Register Copy" forms). */
static inline void fle_vcopy(int d, int s, int wide) {
    fle_xmm lo = X[s], hi;
    if (wide) {
        hi = YH[s];
    } else {
        hi.u64[0] = 0;
        hi.u64[1] = 0;
    }
    X[d] = lo;
    YH[d] = hi;
}

/* VZEROUPPER, 64-bit mode: bits 255:128 of YMM0-YMM15 := 0. Bits 127:0 are unchanged. */
static inline void fle_vzeroupper(void) {
    int k;
    for (k = 0; k < 16; k++) {
        YH[k].u64[0] = 0;
        YH[k].u64[1] = 0;
    }
}

/* GS segment (decision 0032). x64 Windows addresses the TEB through GS. The guest TEB is
 * the host buffer fle_teb (zero-filled). Only these quadwords are modeled (0x10 was added by
 * decision 0076, see below), and an access to any other GS offset stops the run:
 *   GS:[0x30]  NT_TIB.Self, the address of the TEB (winnt.h: NT_TIB64.Self at 0x30);
 *   GS:[0x58]  ThreadLocalStoragePointer (x64 TEB offset 0x58), here the address of
 *              fle_tls_values, the single-thread TlsAlloc slot table of fle_imports.c.
 * Both quadwords are also written into fle_teb, so that a guest that reads Self and then
 * loads through it sees the same values. FS has no model: a memory access through FS is
 * not lifted (vex.rs). */
#define FLE_TEB_SIZE 0x2000u
#define FLE_TEB_SELF 0x30u
#define FLE_TEB_TLSPTR 0x58u
/* Decision 0076: NT_TIB.StackBase (0x8) and NT_TIB.StackLimit (0x10), the top and the bottom of the stack the guest is
 * running on. The values are not made up: they are the bounds of the memory the harness allocated for that stack, given by
 * fle_teb_set_stack. GS:[0x10] can be read by a lifted `mov r64, gs:[0x10]` (dsp_ippv2_x64.dll __chkstk). GS:[0x8] has no
 * lifted read; it is only present in fle_teb for a guest that reads it through Self. */
#define FLE_TEB_STACKBASE 0x8u
#define FLE_TEB_STACKLIMIT 0x10u
#define FLE_TEB_PAGE 0x1000u

uint8_t fle_teb[FLE_TEB_SIZE] __attribute__((weak, aligned(16))) = { 0 };
extern uint64_t fle_tls_values[];
/* Decision 0122 (cooperative threads, fle_thread.c): the TEB and the TlsAlloc slot table of the RUNNING guest thread when it is not the first one. NULL (the default, and the only value
 * in a program that creates no thread) means fle_teb and fle_tls_values, as before. fle_thread.c sets them at every switch. */
uint8_t *fle_teb_cur __attribute__((weak)) = 0;
uint64_t *fle_tls_cur __attribute__((weak)) = 0;

/* Registers the guest stack [lo, hi): StackBase := hi, StackLimit := lo rounded UP to a page boundary. The rounding keeps
 * StackLimit inside the allocation and a multiple of 4096, which the __chkstk probe loop needs to end (it stops only when its
 * pointer is equal to the target page; docs/investigations/teb-peb-model.md section 2.2). For an mmap'ed stack lo is already
 * a page boundary and nothing changes. Call it every time the guest's R[4] moves to another stack (the harness boot does).
 * An empty or inverted range stops the run. */
__attribute__((weak)) void fle_teb_set_stack(uint64_t lo, uint64_t hi) {
    uint64_t limit = (lo + (FLE_TEB_PAGE - 1)) & ~(uint64_t)(FLE_TEB_PAGE - 1);
    if (lo == 0 || limit < lo || limit >= hi) {
        fprintf(stderr, "fle: fle_teb_set_stack: the stack [%#llx, %#llx) has no whole page\n", (unsigned long long)lo, (unsigned long long)hi);
        abort();
    }
    memcpy(fle_teb + FLE_TEB_STACKBASE, &hi, sizeof hi);
    memcpy(fle_teb + FLE_TEB_STACKLIMIT, &limit, sizeof limit);
}

static inline uint64_t fle_gs_ld8(uint64_t off) {
    uint8_t *teb = fle_teb_cur ? fle_teb_cur : fle_teb;
    uint64_t self = (uint64_t)(uintptr_t)teb;
    uint64_t tls = (uint64_t)(uintptr_t)(fle_tls_cur ? fle_tls_cur : fle_tls_values);
    memcpy(teb + FLE_TEB_SELF, &self, sizeof self);
    memcpy(teb + FLE_TEB_TLSPTR, &tls, sizeof tls);
    if (off == FLE_TEB_SELF) return self;
    if (off == FLE_TEB_TLSPTR) return tls;
    if (off == FLE_TEB_STACKLIMIT) {
        uint64_t base, limit;
        memcpy(&base, teb + FLE_TEB_STACKBASE, sizeof base);
        memcpy(&limit, teb + FLE_TEB_STACKLIMIT, sizeof limit);
        if (limit == 0) {
            fprintf(stderr, "fle: GS:[0x10] (StackLimit) read, but no guest stack is registered (fle_teb_set_stack)\n");
            abort();
        }
        /* A read while rsp is outside the registered stack means the guest moved to a stack nobody registered (a missed
         * update): the answer would be the limit of another stack. */
        if (R[4] < limit || R[4] > base) {
            fprintf(stderr, "fle: GS:[0x10] (StackLimit) read with rsp=%#llx outside the registered stack [%#llx, %#llx]\n",
                    (unsigned long long)R[4], (unsigned long long)limit, (unsigned long long)base);
            abort();
        }
        return limit;
    }
    fprintf(stderr, "fle: GS:[%#llx] is not modeled (only 0x30, 0x58 and 0x10)\n", (unsigned long long)off);
    abort();
}

#endif
