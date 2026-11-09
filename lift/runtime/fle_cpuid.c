/* CPUID response for fe-lift output. See fle_cpuid.h and docs/decisions/0009-cpuid.md.
 *
 * Basis (Intel SDM Vol. 2, CPUID, Table 3-8 and the text above it):
 * - Leaf 0: EAX = maximum basic input value. EBX:EDX:ECX = vendor string.
 * - Leaf 1: EAX version, EBX brand/APIC, ECX and EDX feature flags.
 * - An EAX above the maximum basic leaf returns the data of the highest basic leaf.
 * - CPUID clears the high 32 bits of RAX, RBX, RCX, RDX (Intel 64).
 *
 * Decided here (作業者の判断, not fixed by the SDM):
 * - Maximum basic leaf = 1. Vendor string (leaf 0 EBX, ECX, EDX) = 0: no vendor claimed.
 * - Leaf 1 EAX (family/model/stepping) = 0 and EBX = 0: no CPU model claimed.
 * - Leaf 1 EDX (decision 0118, PROVISIONAL; decision 0009 answered SSE and SSE2 only): the x86-64 psABI says that "Any program can expect that an
 *   x86-64 processor implements the baseline features mentioned in table" (CMOV, CX8, FPU, FXSR, MMX, OSFXSR, SCE, SSE, SSE2: the table, low-level-sys-info.tex);
 *   the ones that CPUID reports are EDX 0 FPU, 8 CX8 (CMPXCHG8B), 15 CMOV, 23 MMX, 24 FXSR, 25 SSE, 26 SSE2 (Intel SDM Vol. 1 Table 21-10).
 *   ECX = 0: no SSE3 or later, no AVX (the lifter has no VEX support), as in decisions 0032 and 0075. TSC (bit 4) and SEP (bit 11) are not in
 *   the baseline and stay 0.
 * - Basic leaves 2 through 0x7fffffff return the leaf 1 data (SDM rule above).
 * - Extended leaves (0x80000000 and above) return 0. The SDM does not fix the
 *   maximum extended leaf for this processor, so this is a decision, not confirmed. */
#include "fle_runtime.h"
#include "fle_cpuid.h"

#define FLE_CPUID_EDX_FPU   (UINT64_C(1) << 0)
#define FLE_CPUID_EDX_CX8   (UINT64_C(1) << 8)
#define FLE_CPUID_EDX_CMOV  (UINT64_C(1) << 15)
#define FLE_CPUID_EDX_MMX   (UINT64_C(1) << 23)
#define FLE_CPUID_EDX_FXSR  (UINT64_C(1) << 24)
#define FLE_CPUID_EDX_SSE   (UINT64_C(1) << 25)
#define FLE_CPUID_EDX_SSE2  (UINT64_C(1) << 26)
#define FLE_CPUID_LEAF1_EDX (FLE_CPUID_EDX_FPU | FLE_CPUID_EDX_CX8 | FLE_CPUID_EDX_CMOV | FLE_CPUID_EDX_MMX | FLE_CPUID_EDX_FXSR | \
                             FLE_CPUID_EDX_SSE | FLE_CPUID_EDX_SSE2)

void fle_cpuid(void) {
    uint32_t leaf = (uint32_t)R[0];
    uint64_t a = 0, b = 0, c = 0, d = 0;

    if (leaf == 0) {
        a = 1;
    } else if (leaf >= 0x80000000u) {
        a = b = c = d = 0;
    } else {
        d = FLE_CPUID_LEAF1_EDX;
    }
    R[0] = a;
    R[3] = b;
    R[1] = c;
    R[2] = d;
}

/* XGETBV (decision 0047). The SDM does not give an XCR0 value (it is set by the OS), and leaf 1 ECX = 0 above reports
 * no XSAVE or OSXSAVE (the SDM's #UD condition), so a guest that tests CPUID.1:ECX first never executes it. A run that
 * gets here stops by name. Only stdio and abort are used, so that fle_cpuid.c links alone. */
void fle_xgetbv_stop(uint64_t va) {
    fprintf(stderr, "fle: xgetbv at %#llx: XCR0 is not defined (CPUID leaf 1 ECX reports no OSXSAVE; the SDM gives no value)\n",
            (unsigned long long)va);
    abort();
}
