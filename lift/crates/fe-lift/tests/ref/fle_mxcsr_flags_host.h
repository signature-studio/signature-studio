/* SSE/SSE2/SSE4.1 floating-point operations with the MXCSR exception flags (docs/decisions/0038-mxcsr-flags.md).
 *
 * Header only. Each helper runs the instruction itself on the host CPU (inline asm), with the host MXCSR set
 * from the guest's fle_mxcsr for the duration of the one instruction:
 *   - control bits RC (14:13), FTZ (15), DAZ (6) are the guest's, so the result is the guest's;
 *   - all exception masks are set (0x1F80) and the status flags are clear, so the host never traps and the
 *     flags it sets after the instruction (stmxcsr) are exactly the ones this instruction raised;
 *   - the raised flags are OR-ed into fle_mxcsr (they are sticky, SDM Vol. 1, MXCSR);
 *   - if a raised flag is unmasked in the guest's MXCSR (bit 7+n clear), real hardware would deliver #XM
 *     (or #UD): the run stops with a named abort instead of continuing with a wrong state.
 * Result bits, NaN selection and QNaN conversion are the hardware's, so no NaN rule is re-implemented here.
 * The host MXCSR is put back to 0x1F80 after an instruction when the guest control bits are not the default.
 *
 * fle_mxcsr has a weak definition here so that a program built from a header-only runtime links; the strong one
 * is in runtime/fle_mxcsr.c.
 */
#ifndef FLE_HX_FLAGS_H
#define FLE_HX_FLAGS_H

#include <emmintrin.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define FLE_HX_CTL 0xE040u   /* RC, FTZ, DAZ */
#define FLE_HX_MASKS 0x1F80u /* IM DM ZM OM UM PM */
#define FLE_HX_FLAGS 0x003Fu /* IE DE ZE OE UE PE */

extern uint32_t fle_mxcsr; /* defined by the pure runtime header (weak) or runtime/fle_mxcsr.c */

static inline void fle_hx_unmasked(uint32_t raised) {
    fprintf(stderr, "fle: SSE floating-point exception unmasked in guest MXCSR (raised flags %#x, MXCSR %#x); #XM is not modelled\n",
            (unsigned)raised, (unsigned)fle_mxcsr);
    abort();
}

static inline void fle_hx_begin(void) {
    uint32_t v = (fle_mxcsr & FLE_HX_CTL) | FLE_HX_MASKS;
    __asm__ volatile("ldmxcsr %0" : : "m"(v) : "memory");
}

static inline void fle_hx_end(void) {
    uint32_t h;
    __asm__ volatile("stmxcsr %0" : "=m"(h) : : "memory");
    h &= FLE_HX_FLAGS;
    if (fle_mxcsr & FLE_HX_CTL) {
        uint32_t v = FLE_HX_MASKS;
        __asm__ volatile("ldmxcsr %0" : : "m"(v) : "memory");
    }
    if (h & ~((fle_mxcsr >> 7) & FLE_HX_FLAGS)) {
        fle_hx_unmasked(h);
    }
    fle_mxcsr |= h;
}

static inline __m128i fle_hx_v(fle_xmm a) {
    __m128i v;
    memcpy(&v, &a, sizeof v);
    return v;
}

static inline fle_xmm fle_hx_x(__m128i v) {
    fle_xmm a;
    memcpy(&a, &v, sizeof a);
    return a;
}

/* Memory operands narrower than 128 bits, as the low part of an XMM value. The rest is zero and unused. */
static inline fle_xmm fle_hx_m32(uint64_t a) {
    fle_xmm v;
    v.u64[0] = 0;
    v.u64[1] = 0;
    v.u32[0] = (uint32_t)fle_ld(a, 4);
    return v;
}

static inline fle_xmm fle_hx_m64(uint64_t a) {
    fle_xmm v;
    v.u64[0] = fle_ld(a, 8);
    v.u64[1] = 0;
    return v;
}

/* dst = op(dst, src). Scalar forms keep the upper lanes of dst (the instruction does that). */
#define FLE_HX_OP2(name, insn) \
    static inline fle_xmm name(fle_xmm d, fle_xmm s) \
    { \
        __m128i x = fle_hx_v(d), y = fle_hx_v(s); \
        fle_hx_begin(); \
        __asm__ volatile(insn " %1, %0" : "+x"(x) : "x"(y)); \
        fle_hx_end(); \
        return fle_hx_x(x); \
    }

FLE_HX_OP2(fle_hx_addss, "addss")
FLE_HX_OP2(fle_hx_addsd, "addsd")
FLE_HX_OP2(fle_hx_addps, "addps")
FLE_HX_OP2(fle_hx_addpd, "addpd")
FLE_HX_OP2(fle_hx_subss, "subss")
FLE_HX_OP2(fle_hx_subsd, "subsd")
FLE_HX_OP2(fle_hx_subps, "subps")
FLE_HX_OP2(fle_hx_subpd, "subpd")
FLE_HX_OP2(fle_hx_mulss, "mulss")
FLE_HX_OP2(fle_hx_mulsd, "mulsd")
FLE_HX_OP2(fle_hx_mulps, "mulps")
FLE_HX_OP2(fle_hx_mulpd, "mulpd")
FLE_HX_OP2(fle_hx_divss, "divss")
FLE_HX_OP2(fle_hx_divsd, "divsd")
FLE_HX_OP2(fle_hx_divps, "divps")
FLE_HX_OP2(fle_hx_divpd, "divpd")
FLE_HX_OP2(fle_hx_minss, "minss")
FLE_HX_OP2(fle_hx_minsd, "minsd")
FLE_HX_OP2(fle_hx_minps, "minps")
FLE_HX_OP2(fle_hx_minpd, "minpd")
FLE_HX_OP2(fle_hx_maxss, "maxss")
FLE_HX_OP2(fle_hx_maxsd, "maxsd")
FLE_HX_OP2(fle_hx_maxps, "maxps")
FLE_HX_OP2(fle_hx_maxpd, "maxpd")
FLE_HX_OP2(fle_hx_sqrtss, "sqrtss")
FLE_HX_OP2(fle_hx_sqrtsd, "sqrtsd")
FLE_HX_OP2(fle_hx_sqrtps, "sqrtps")
FLE_HX_OP2(fle_hx_sqrtpd, "sqrtpd")
FLE_HX_OP2(fle_hx_cvtss2sd, "cvtss2sd")
FLE_HX_OP2(fle_hx_cvtsd2ss, "cvtsd2ss")
FLE_HX_OP2(fle_hx_cvtps2pd, "cvtps2pd")
FLE_HX_OP2(fle_hx_cvtpd2ps, "cvtpd2ps")
FLE_HX_OP2(fle_hx_cvtdq2ps, "cvtdq2ps")
FLE_HX_OP2(fle_hx_cvtdq2pd, "cvtdq2pd")
FLE_HX_OP2(fle_hx_cvtps2dq, "cvtps2dq")
FLE_HX_OP2(fle_hx_cvttps2dq, "cvttps2dq")
FLE_HX_OP2(fle_hx_cvtpd2dq, "cvtpd2dq")
FLE_HX_OP2(fle_hx_cvttpd2dq, "cvttpd2dq")

/* CMPSS, CMPSD, CMPPS, CMPPD (imm8 predicate 0..7) and ROUNDSS, ROUNDSD, ROUNDPS, ROUNDPD (imm8): the immediate
 * must be a constant, so these are macros. Value is the new destination. */
#define FLE_HX_IMM2(insn, d, s, imm) \
    ({ \
        __m128i x_ = fle_hx_v(d), y_ = fle_hx_v(s); \
        fle_hx_begin(); \
        __asm__ volatile(insn " %2, %1, %0" : "+x"(x_) : "x"(y_), "i"(imm)); \
        fle_hx_end(); \
        fle_hx_x(x_); \
    })

#define FLE_HX_CMPSS(d, s, imm) FLE_HX_IMM2("cmpss", d, s, imm)
#define FLE_HX_CMPSD(d, s, imm) FLE_HX_IMM2("cmpsd", d, s, imm)
#define FLE_HX_CMPPS(d, s, imm) FLE_HX_IMM2("cmpps", d, s, imm)
#define FLE_HX_CMPPD(d, s, imm) FLE_HX_IMM2("cmppd", d, s, imm)
#define FLE_HX_ROUNDSS(d, s, imm) FLE_HX_IMM2("roundss", d, s, imm)
#define FLE_HX_ROUNDSD(d, s, imm) FLE_HX_IMM2("roundsd", d, s, imm)
#define FLE_HX_ROUNDPS(d, s, imm) FLE_HX_IMM2("roundps", d, s, imm)
#define FLE_HX_ROUNDPD(d, s, imm) FLE_HX_IMM2("roundpd", d, s, imm)

/* CVTSI2SS, CVTSI2SD: the upper lanes of dst are kept. */
#define FLE_HX_CVTSI2(name, insn, T) \
    static inline fle_xmm name(fle_xmm d, T v) \
    { \
        __m128i x = fle_hx_v(d); \
        fle_hx_begin(); \
        __asm__ volatile(insn " %1, %0" : "+x"(x) : "r"(v)); \
        fle_hx_end(); \
        return fle_hx_x(x); \
    }

FLE_HX_CVTSI2(fle_hx_cvtsi2ss32, "cvtsi2ss", int32_t)
FLE_HX_CVTSI2(fle_hx_cvtsi2ss64, "cvtsi2ss", int64_t)
FLE_HX_CVTSI2(fle_hx_cvtsi2sd32, "cvtsi2sd", int32_t)
FLE_HX_CVTSI2(fle_hx_cvtsi2sd64, "cvtsi2sd", int64_t)

/* CVTSS2SI, CVTSD2SI, CVTTSS2SI, CVTTSD2SI to a 32-bit (zero-extended) or 64-bit GPR value. */
#define FLE_HX_CVT2SI(name, insn, T, R) \
    static inline uint64_t name(fle_xmm s) \
    { \
        __m128i y = fle_hx_v(s); \
        T r; \
        fle_hx_begin(); \
        __asm__ volatile(insn " %1, %0" : "=r"(r) : "x"(y)); \
        fle_hx_end(); \
        return (R)r; \
    }

FLE_HX_CVT2SI(fle_hx_cvtss2si32, "cvtss2si", int32_t, uint32_t)
FLE_HX_CVT2SI(fle_hx_cvtss2si64, "cvtss2si", int64_t, uint64_t)
FLE_HX_CVT2SI(fle_hx_cvtsd2si32, "cvtsd2si", int32_t, uint32_t)
FLE_HX_CVT2SI(fle_hx_cvtsd2si64, "cvtsd2si", int64_t, uint64_t)
FLE_HX_CVT2SI(fle_hx_cvttss2si32, "cvttss2si", int32_t, uint32_t)
FLE_HX_CVT2SI(fle_hx_cvttss2si64, "cvttss2si", int64_t, uint64_t)
FLE_HX_CVT2SI(fle_hx_cvttsd2si32, "cvttsd2si", int32_t, uint32_t)
FLE_HX_CVT2SI(fle_hx_cvttsd2si64, "cvttsd2si", int64_t, uint64_t)

/* UCOMISS, UCOMISD, COMISS, COMISD: ZF, PF, CF from the instruction; OF, SF, AF cleared (SDM). */
#define FLE_HX_COMI(name, insn) \
    static inline void name(fle_xmm a, fle_xmm b) \
    { \
        __m128i x = fle_hx_v(a), y = fle_hx_v(b); \
        uint8_t c, p, z; \
        fle_hx_begin(); \
        __asm__ volatile(insn " %4, %3" : "=@ccc"(c), "=@ccp"(p), "=@ccz"(z) : "x"(x), "x"(y)); \
        fle_hx_end(); \
        FL.cf = c; \
        FL.pf = p; \
        FL.zf = z; \
        FL.of = 0; \
        FL.sf = 0; \
        FL.af = 0; \
    }

FLE_HX_COMI(fle_hx_ucomiss, "ucomiss")
FLE_HX_COMI(fle_hx_ucomisd, "ucomisd")
FLE_HX_COMI(fle_hx_comiss, "comiss")
FLE_HX_COMI(fle_hx_comisd, "comisd")

#endif
