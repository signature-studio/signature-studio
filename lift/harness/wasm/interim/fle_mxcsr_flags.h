/* wasm probe shim: software replacement for runtime/fle_mxcsr_flags.h (same names, same signatures).
 * The original runs each SSE FP instruction on the host CPU with inline asm (ldmxcsr / stmxcsr / addss ...),
 * which cannot be compiled for wasm. This version computes with C float/double (wasm f32/f64, round to nearest even)
 * and the SSE NaN rules already in fle_sse.h.
 *
 * KNOWN DIFFERENCES from the host-execution version (the reason this is a probe, not a fix):
 *   - the MXCSR status flags (IE DE ZE OE UE PE) are NEVER raised: fle_mxcsr is not updated by FP instructions;
 *   - the MXCSR control bits RC / FTZ / DAZ are ignored (always round-to-nearest, no flush) except ROUND* with imm bit 2;
 *   - the default NaN for an invalid operation is forced to the x86 "real indefinite" (0xFFC00000 / 0xFFF8000000000000);
 *     other NaN payload behaviour follows fle_sse.h, not the hardware.
 * Installed by lift/out-wasm/run.sh into lift/out-wasm/rt-wasm/fle_mxcsr_flags.h (a copy of the runtime). */
#ifndef FLE_MXCSR_FLAGS_H
#define FLE_MXCSR_FLAGS_H

#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define FLE_MX_CTL 0xE040u
#define FLE_MX_MASKS 0x1F80u
#define FLE_MX_FLAGS 0x003Fu

__attribute__((weak)) uint32_t fle_mxcsr = 0x1F80;

static inline fle_xmm fle_mx_m32(uint64_t a) {
    fle_xmm v;
    v.u64[0] = 0;
    v.u64[1] = 0;
    v.u32[0] = (uint32_t)fle_ld(a, 4);
    return v;
}

static inline fle_xmm fle_mx_m64(uint64_t a) {
    fle_xmm v;
    v.u64[0] = fle_ld(a, 8);
    v.u64[1] = 0;
    return v;
}

#define FLE_MX_QNAN32 0xFFC00000u
#define FLE_MX_QNAN64 UINT64_C(0xFFF8000000000000)

static inline uint32_t fle_mx_op32(uint32_t a, uint32_t b, int op) {
    float x = fle_f32(a), y = fle_f32(b), r = 0;
    if (fle_nan32(a)) return a | 0x00400000u;
    if (fle_nan32(b)) return b | 0x00400000u;
    switch (op) {
    case 0: r = x + y; break;
    case 1: r = x - y; break;
    case 2: r = x * y; break;
    default: r = x / y; break;
    }
    return isnan(r) ? FLE_MX_QNAN32 : fle_b32(r);
}

static inline uint64_t fle_mx_op64(uint64_t a, uint64_t b, int op) {
    double x = fle_f64(a), y = fle_f64(b), r = 0;
    if (fle_nan64(a)) return a | UINT64_C(0x0008000000000000);
    if (fle_nan64(b)) return b | UINT64_C(0x0008000000000000);
    switch (op) {
    case 0: r = x + y; break;
    case 1: r = x - y; break;
    case 2: r = x * y; break;
    default: r = x / y; break;
    }
    return isnan(r) ? FLE_MX_QNAN64 : fle_b64(r);
}

static inline uint32_t fle_mx_sq32(uint32_t b) {
    if (fle_nan32(b)) return b | 0x00400000u;
    return fle_b32(sqrtf(fle_f32(b))) == 0x7FC00000u ? FLE_MX_QNAN32 : fle_b32(sqrtf(fle_f32(b)));
}

static inline uint64_t fle_mx_sq64(uint64_t b) {
    if (fle_nan64(b)) return b | UINT64_C(0x0008000000000000);
    return fle_b64(sqrt(fle_f64(b))) == UINT64_C(0x7FF8000000000000) ? FLE_MX_QNAN64 : fle_b64(sqrt(fle_f64(b)));
}

/* scalar: lane 0 only, the rest of d is kept. packed: every lane. */
#define FLE_MX_DEF_SS(name, op) \
    static inline fle_xmm name(fle_xmm d, fle_xmm s) { d.u32[0] = fle_mx_op32(d.u32[0], s.u32[0], op); return d; }
#define FLE_MX_DEF_SD(name, op) \
    static inline fle_xmm name(fle_xmm d, fle_xmm s) { d.u64[0] = fle_mx_op64(d.u64[0], s.u64[0], op); return d; }
#define FLE_MX_DEF_PS(name, op) \
    static inline fle_xmm name(fle_xmm d, fle_xmm s) { for (int k = 0; k < 4; k++) d.u32[k] = fle_mx_op32(d.u32[k], s.u32[k], op); return d; }
#define FLE_MX_DEF_PD(name, op) \
    static inline fle_xmm name(fle_xmm d, fle_xmm s) { for (int k = 0; k < 2; k++) d.u64[k] = fle_mx_op64(d.u64[k], s.u64[k], op); return d; }

FLE_MX_DEF_SS(fle_mx_addss, 0) FLE_MX_DEF_SD(fle_mx_addsd, 0) FLE_MX_DEF_PS(fle_mx_addps, 0) FLE_MX_DEF_PD(fle_mx_addpd, 0)
FLE_MX_DEF_SS(fle_mx_subss, 1) FLE_MX_DEF_SD(fle_mx_subsd, 1) FLE_MX_DEF_PS(fle_mx_subps, 1) FLE_MX_DEF_PD(fle_mx_subpd, 1)
FLE_MX_DEF_SS(fle_mx_mulss, 2) FLE_MX_DEF_SD(fle_mx_mulsd, 2) FLE_MX_DEF_PS(fle_mx_mulps, 2) FLE_MX_DEF_PD(fle_mx_mulpd, 2)
FLE_MX_DEF_SS(fle_mx_divss, 3) FLE_MX_DEF_SD(fle_mx_divsd, 3) FLE_MX_DEF_PS(fle_mx_divps, 3) FLE_MX_DEF_PD(fle_mx_divpd, 3)

static inline fle_xmm fle_mx_minss(fle_xmm d, fle_xmm s) { d.u32[0] = fle_minss(d.u32[0], s.u32[0]); return d; }
static inline fle_xmm fle_mx_maxss(fle_xmm d, fle_xmm s) { d.u32[0] = fle_maxss(d.u32[0], s.u32[0]); return d; }
static inline fle_xmm fle_mx_minsd(fle_xmm d, fle_xmm s) { d.u64[0] = fle_minsd(d.u64[0], s.u64[0]); return d; }
static inline fle_xmm fle_mx_maxsd(fle_xmm d, fle_xmm s) { d.u64[0] = fle_maxsd(d.u64[0], s.u64[0]); return d; }
static inline fle_xmm fle_mx_minps(fle_xmm d, fle_xmm s) { for (int k = 0; k < 4; k++) d.u32[k] = fle_minss(d.u32[k], s.u32[k]); return d; }
static inline fle_xmm fle_mx_maxps(fle_xmm d, fle_xmm s) { for (int k = 0; k < 4; k++) d.u32[k] = fle_maxss(d.u32[k], s.u32[k]); return d; }
static inline fle_xmm fle_mx_minpd(fle_xmm d, fle_xmm s) { for (int k = 0; k < 2; k++) d.u64[k] = fle_minsd(d.u64[k], s.u64[k]); return d; }
static inline fle_xmm fle_mx_maxpd(fle_xmm d, fle_xmm s) { for (int k = 0; k < 2; k++) d.u64[k] = fle_maxsd(d.u64[k], s.u64[k]); return d; }

static inline fle_xmm fle_mx_sqrtss(fle_xmm d, fle_xmm s) { d.u32[0] = fle_mx_sq32(s.u32[0]); return d; }
static inline fle_xmm fle_mx_sqrtsd(fle_xmm d, fle_xmm s) { d.u64[0] = fle_mx_sq64(s.u64[0]); return d; }
static inline fle_xmm fle_mx_sqrtps(fle_xmm d, fle_xmm s) { for (int k = 0; k < 4; k++) d.u32[k] = fle_mx_sq32(s.u32[k]); return d; }
static inline fle_xmm fle_mx_sqrtpd(fle_xmm d, fle_xmm s) { for (int k = 0; k < 2; k++) d.u64[k] = fle_mx_sq64(s.u64[k]); return d; }

static inline fle_xmm fle_mx_cvtss2sd(fle_xmm d, fle_xmm s) { d.u64[0] = fle_cvtss2sd(s.u32[0]); return d; }
static inline fle_xmm fle_mx_cvtsd2ss(fle_xmm d, fle_xmm s) { d.u32[0] = fle_cvtsd2ss(s.u64[0]); return d; }
static inline fle_xmm fle_mx_cvtps2pd(fle_xmm d, fle_xmm s) {
    uint32_t a = s.u32[0], b = s.u32[1];
    (void)d;
    d.u64[0] = fle_cvtss2sd(a);
    d.u64[1] = fle_cvtss2sd(b);
    return d;
}
static inline fle_xmm fle_mx_cvtpd2ps(fle_xmm d, fle_xmm s) {
    (void)d;
    d.u32[0] = fle_cvtsd2ss(s.u64[0]);
    d.u32[1] = fle_cvtsd2ss(s.u64[1]);
    d.u64[1] = 0;
    return d;
}
static inline fle_xmm fle_mx_cvtdq2ps(fle_xmm d, fle_xmm s) { for (int k = 0; k < 4; k++) d.f32[k] = (float)(int32_t)s.u32[k]; return d; }
static inline fle_xmm fle_mx_cvtdq2pd(fle_xmm d, fle_xmm s) {
    int32_t a = (int32_t)s.u32[0], b = (int32_t)s.u32[1];
    d.f64[0] = (double)a;
    d.f64[1] = (double)b;
    return d;
}
static inline fle_xmm fle_mx_cvtps2dq(fle_xmm d, fle_xmm s) { (void)d; return fle_cvtps2dq(s); }
static inline fle_xmm fle_mx_cvttps2dq(fle_xmm d, fle_xmm s) { (void)d; return fle_cvttps2dq(s); }
static inline fle_xmm fle_mx_cvtpd2dq(fle_xmm d, fle_xmm s) {
    (void)d;
    d.u32[0] = (uint32_t)fle_cvtr32(s.f64[0]);
    d.u32[1] = (uint32_t)fle_cvtr32(s.f64[1]);
    d.u64[1] = 0;
    return d;
}
static inline fle_xmm fle_mx_cvttpd2dq(fle_xmm d, fle_xmm s) {
    (void)d;
    d.u32[0] = (uint32_t)fle_cvtt32(s.f64[0]);
    d.u32[1] = (uint32_t)fle_cvtt32(s.f64[1]);
    d.u64[1] = 0;
    return d;
}

static inline uint64_t fle_mx_cmp(double a, double b, int imm, int un) {
    int r;
    switch (imm & 7) {
    case 0: r = a == b; break;
    case 1: r = a < b; break;
    case 2: r = a <= b; break;
    case 3: r = un; break;
    case 4: r = !(a == b); break;
    case 5: r = !(a < b); break;
    case 6: r = !(a <= b); break;
    default: r = !un; break;
    }
    return r ? UINT64_MAX : 0;
}

#define FLE_MX_CMPSS(d, s, imm) \
    ({ fle_xmm d_ = (d), s_ = (s); \
       d_.u32[0] = (uint32_t)fle_mx_cmp((double)d_.f32[0], (double)s_.f32[0], (imm), fle_nan32(d_.u32[0]) || fle_nan32(s_.u32[0])); d_; })
#define FLE_MX_CMPSD(d, s, imm) \
    ({ fle_xmm d_ = (d), s_ = (s); \
       d_.u64[0] = fle_mx_cmp(d_.f64[0], s_.f64[0], (imm), fle_nan64(d_.u64[0]) || fle_nan64(s_.u64[0])); d_; })
#define FLE_MX_CMPPS(d, s, imm) \
    ({ fle_xmm d_ = (d), s_ = (s); \
       for (int k_ = 0; k_ < 4; k_++) d_.u32[k_] = (uint32_t)fle_mx_cmp((double)d_.f32[k_], (double)s_.f32[k_], (imm), fle_nan32(d_.u32[k_]) || fle_nan32(s_.u32[k_])); d_; })
#define FLE_MX_CMPPD(d, s, imm) \
    ({ fle_xmm d_ = (d), s_ = (s); \
       for (int k_ = 0; k_ < 2; k_++) d_.u64[k_] = fle_mx_cmp(d_.f64[k_], s_.f64[k_], (imm), fle_nan64(d_.u64[k_]) || fle_nan64(s_.u64[k_])); d_; })

static inline double fle_mx_rnd(double x, int imm) {
    int m = (imm & 4) ? (int)((fle_mxcsr >> 13) & 3) : (imm & 3);
    if (!(fabs(x) < 0x1p52)) return x;
    switch (m) {
    case 0: return fle_rne(x);
    case 1: return floor(x);
    case 2: return ceil(x);
    default: return trunc(x);
    }
}
#define FLE_MX_ROUNDSS(d, s, imm) \
    ({ fle_xmm d_ = (d), s_ = (s); d_.f32[0] = (float)fle_mx_rnd((double)s_.f32[0], (imm)); d_; })
#define FLE_MX_ROUNDSD(d, s, imm) \
    ({ fle_xmm d_ = (d), s_ = (s); d_.f64[0] = fle_mx_rnd(s_.f64[0], (imm)); d_; })
#define FLE_MX_ROUNDPS(d, s, imm) \
    ({ fle_xmm d_ = (d), s_ = (s); for (int k_ = 0; k_ < 4; k_++) d_.f32[k_] = (float)fle_mx_rnd((double)s_.f32[k_], (imm)); d_; })
#define FLE_MX_ROUNDPD(d, s, imm) \
    ({ fle_xmm d_ = (d), s_ = (s); for (int k_ = 0; k_ < 2; k_++) d_.f64[k_] = fle_mx_rnd(s_.f64[k_], (imm)); d_; })

static inline fle_xmm fle_mx_cvtsi2ss32(fle_xmm d, int32_t v) { d.f32[0] = (float)v; return d; }
static inline fle_xmm fle_mx_cvtsi2ss64(fle_xmm d, int64_t v) { d.f32[0] = (float)v; return d; }
static inline fle_xmm fle_mx_cvtsi2sd32(fle_xmm d, int32_t v) { d.f64[0] = (double)v; return d; }
static inline fle_xmm fle_mx_cvtsi2sd64(fle_xmm d, int64_t v) { d.f64[0] = (double)v; return d; }

static inline uint64_t fle_mx_cvtss2si32(fle_xmm s) { return fle_cvtr32((double)s.f32[0]); }
static inline uint64_t fle_mx_cvtss2si64(fle_xmm s) { return fle_cvtr64((double)s.f32[0]); }
static inline uint64_t fle_mx_cvtsd2si32(fle_xmm s) { return fle_cvtr32(s.f64[0]); }
static inline uint64_t fle_mx_cvtsd2si64(fle_xmm s) { return fle_cvtr64(s.f64[0]); }
static inline uint64_t fle_mx_cvttss2si32(fle_xmm s) { return fle_cvtt32((double)s.f32[0]); }
static inline uint64_t fle_mx_cvttss2si64(fle_xmm s) { return fle_cvtt64((double)s.f32[0]); }
static inline uint64_t fle_mx_cvttsd2si32(fle_xmm s) { return fle_cvtt32(s.f64[0]); }
static inline uint64_t fle_mx_cvttsd2si64(fle_xmm s) { return fle_cvtt64(s.f64[0]); }

static inline void fle_mx_comi_set(double a, double b, int un) {
    FL.of = 0;
    FL.sf = 0;
    FL.af = 0;
    if (un) {
        FL.zf = 1; FL.pf = 1; FL.cf = 1;
    } else if (a > b) {
        FL.zf = 0; FL.pf = 0; FL.cf = 0;
    } else if (a < b) {
        FL.zf = 0; FL.pf = 0; FL.cf = 1;
    } else {
        FL.zf = 1; FL.pf = 0; FL.cf = 0;
    }
}
static inline void fle_mx_ucomiss(fle_xmm a, fle_xmm b) { fle_mx_comi_set((double)a.f32[0], (double)b.f32[0], fle_nan32(a.u32[0]) || fle_nan32(b.u32[0])); }
static inline void fle_mx_comiss(fle_xmm a, fle_xmm b) { fle_mx_comi_set((double)a.f32[0], (double)b.f32[0], fle_nan32(a.u32[0]) || fle_nan32(b.u32[0])); }
static inline void fle_mx_ucomisd(fle_xmm a, fle_xmm b) { fle_mx_comi_set(a.f64[0], b.f64[0], fle_nan64(a.u64[0]) || fle_nan64(b.u64[0])); }
static inline void fle_mx_comisd(fle_xmm a, fle_xmm b) { fle_mx_comi_set(a.f64[0], b.f64[0], fle_nan64(a.u64[0]) || fle_nan64(b.u64[0])); }

#endif
