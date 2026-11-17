/* Directed-rounding fast path for SSE floating point (decision 0172). Included by fle_mxcsr_vec.h (the arithmetic functions and the
 * conversion functions at its end; the table and the trampoline are used by fle_mx_do2_slow_p in fle_mxcsr_flags.h).
 *
 * The problem. The fast paths of decisions 0143 and 0154 are for the guest rounding control RC = 0 (nearest even), the
 * only mode the host (C, SSE, WebAssembly) runs in. A guest that runs with RC = 1 (down), 2 (up) or 3 (toward zero) took
 * the integer implementation for every operation: Serum 2 sets MXCSR = 0xffc0 (FTZ DAZ RC = 3, all masked) at the start
 * of process and 0x1f80 at the end, and 75 to 87 percent of its process time was spent there.
 *
 * The idea. The host gives the nearest-even result s of an operation and an error term e = (exact result) - s
 * (an error-free transformation, or an exact remainder), so the sign of (exact - s) is known. The result for RC = 1
 * (down), 2 (up), 3 (zero) is either s or the neighbour of s on the side of the exact result: one step on the bit
 * pattern (the encoding is sign and magnitude, so +1 on the bits moves away from zero, -1 toward zero):
 *     RD: s - ulp if exact < s,   RU: s + ulp if exact > s,   RZ: toward zero if |exact| < |s|.
 * Nothing is rounded twice and no flag is computed: the guard is PE already set in the guest MXCSR and PM masked (RC is not part of
 * it: decision 0154's guard with RC = 0 dropped), so the sticky flags do not change, and the windows are those of decision 0154
 * (every operand zero or a normal number far from the exponent limits, so the exact result is zero or a normal number and no
 * IE DE ZE OE UE can be raised; FTZ and DAZ change only denormal operands and tiny results, which the windows exclude), except
 * that binary64 MUL DIV SQRT use 2^-400 .. 2^400 (below). Everything else takes the code of decision 0143, unchanged. The proof is
 * "inside the window, nothing but PE can happen and the directed result is s stepped by the sign of the error"; the tests compare
 * it with the instructions run on the host CPU (harness/fp1/mxfp_diff.c: rc, mxall).
 *
 * Call structure (no code at the call sites changes). fle_mx_do2_slow_p (the out-of-line entry of the sites, now a few instructions)
 * indexes the table fle_mxrc_tab[RC][op + 8 * f64 + 16 * packed]; each entry is one function per instruction and rounding mode in
 * which RC is a constant; it checks the windows and either writes the result or calls the integer implementation of the
 * translation unit (passed as a function pointer, a tail call). The functions and the table are weak definitions: one copy in a
 * program after linking.
 *
 * Error terms (all exact inside the windows, nearest even host arithmetic):
 *   a + b, a - b (binary32 and binary64)  Knuth's TwoSum: s = a + b, v = s - a, e = (a - (s - v)) + (b - v).
 *   a * b (binary64)                      Dekker's TwoProduct with Veltkamp's split (2^27 + 1): the operands are limited to
 *                                         2^-400 <= |x| <= 2^400 so that no partial product is denormal (that needs the sum of
 *                                         the two exponents >= -970).
 *   a * b (binary32)                      the double product (24 + 24 bits, exact); e = d - (double)(float)d.
 *   a / b (binary64)                      q = a / b; the remainder a - q * b = (a - p) - e with (p, e) = TwoProduct(q, b) is
 *                                         exactly representable; exact - q has the sign of the remainder times the sign of b.
 *   a / b (binary32)                      the remainder a - (double)q * (double)b in double (exact).
 *   sqrt(b)                               the remainder b - q * q the same way; exact - q has the sign of the remainder.
 *   conversions                           the rounded value and the source compared exactly in a wider type.
 * The zero sign: only a + b = +0 (the exact sum of operands of opposite sign, or +0 + -0) is -0 for RD; products, quotients and
 * roots keep the sign of the host result.
 *
 * (Decision 0243: the alternatives -DFLE_MX_NORC, -DFLE_MXR_DIRECT, -DFLE_MXR_STATIC are gone; this path and the trampoline are the only ones.)
 * Needs the vector extension and __builtin_elementwise_sqrt (clang) for the packed forms; with gcc the packed forms take the integer path.
 */
#ifndef FLE_MXCSR_RC_H
#define FLE_MXCSR_RC_H

#if defined(FLE_MX_VEC)
#define FLE_MX_RC 1

/* PE set, PM masked, and RC != 0 (decision 0154 has the same guard with RC = 0) */
#define FLE_MXR_ON(mx) (FLE_MXV_NP0X(mx) && ((mx) & 0x6000u))
#define FLE_MXR_RC(mx) (((mx) >> 13) & 3u)

#define FLE_MXR_AI static inline __attribute__((always_inline))
#define FLE_MXR_FN __attribute__((weak, noinline))

#ifdef FLE_MX_STATS
extern uint64_t fle_mx_rcstat[3][64];
#define FLE_MXRS(kind, id) (fle_mx_rcstat[(kind)][(id)]++)
#else
#define FLE_MXRS(kind, id) ((void)(kind), (void)(id))
#endif

#define FLE_MXR_S64 UINT64_C(0x8000000000000000)
#define FLE_MXR_S32 0x80000000u

FLE_MXR_AI double fle_mxr_d(uint64_t u) {
    double x;
    __builtin_memcpy(&x, &u, 8);
    return x;
}
FLE_MXR_AI uint64_t fle_mxr_ud(double x) {
    uint64_t u;
    __builtin_memcpy(&u, &x, 8);
    return u;
}
FLE_MXR_AI float fle_mxr_f(uint32_t u) {
    float x;
    __builtin_memcpy(&x, &u, 4);
    return x;
}
FLE_MXR_AI uint32_t fle_mxr_uf(float x) {
    uint32_t u;
    __builtin_memcpy(&u, &x, 4);
    return u;
}

/* The directed result from the nearest-even bits sb and the error e = exact - s (sign and zero from its bit pattern eb; rc = 1 down, 2 up,
 * 3 toward zero, a constant in every caller so that only one arm remains). The bits move by one:
 *   toward zero (RZ): when e and s have opposite signs (e = 0 never steps): bits - 1
 *   down (RD): e < 0: s > 0 bits - 1, s < 0 bits + 1;   up (RU): e > 0: s > 0 bits + 1, s < 0 bits - 1. */
FLE_MXR_AI uint64_t fle_mxr_step64(uint64_t sb, uint64_t eb, unsigned rc) {
    int64_t nz = -(int64_t)((eb << 1) != 0), esg = (int64_t)eb >> 63, ssg = (int64_t)sb >> 63, delta;
    if (rc == 3) {
        delta = (esg ^ ssg) & nz;
    } else if (rc == 1) {
        delta = esg & nz & ((ssg & 2) - 1);
    } else {
        delta = ~esg & nz & (1 - (ssg & 2));
    }
    return sb + (uint64_t)delta;
}
/* the same for a binary32 s; the error is given by its sign mask (all ones if negative) and nonzero mask */
FLE_MXR_AI uint32_t fle_mxr_step32(uint32_t sb, int32_t esg, int32_t nz, unsigned rc) {
    int32_t ssg = (int32_t)sb >> 31, delta;
    if (rc == 3) {
        delta = (esg ^ ssg) & nz;
    } else if (rc == 1) {
        delta = esg & nz & ((ssg & 2) - 1);
    } else {
        delta = ~esg & nz & (1 - (ssg & 2));
    }
    return sb + (uint32_t)delta;
}
#define FLE_MXR_E32(eb) ((int32_t)(eb) >> 31), (-(int32_t)(((eb) << 1) != 0))   /* error of binary32 bits */
#define FLE_MXR_E64T32(eb) (-(int32_t)((eb) >> 63)), (-(int32_t)(((eb) << 1) != 0)) /* error of binary64 bits, for a binary32 s */

/* ---- windows ---- */
/* zero, or 2^-400 <= |x| <= 2^400 (binary64 products, quotients, roots) */
FLE_MXR_AI int fle_mxr_wm64(uint64_t u) {
    uint64_t t = u & UINT64_C(0x7fffffffffffffff);
    return (int)(((t - ((uint64_t)(1023 - 400) << 52)) <= (((uint64_t)(2 * 400)) << 52)) | (t == 0));
}
FLE_MXR_AI int fle_mxr_wm64nz(uint64_t u) {
    uint64_t t = u & UINT64_C(0x7fffffffffffffff);
    return (t - ((uint64_t)(1023 - 400) << 52)) <= (((uint64_t)(2 * 400)) << 52);
}
FLE_MXR_AI int fle_mxr_wm64sqrt(uint64_t u) { /* +-0, or 2^-400 <= x <= 2^400 */
    return (int)(((u - ((uint64_t)(1023 - 400) << 52)) <= (((uint64_t)(2 * 400)) << 52)) | ((u & UINT64_C(0x7fffffffffffffff)) == 0));
}

/* ---- scalar lanes on host float/double values (no trip through the integer registers except for the step). The caller has checked
 * the windows. ---- */
/* both operands: zero, or inside the window of ADD and SUB (2^-510 .. 2^510), as one vector test */
FLE_MXR_AI int fle_mxr_wab64(double a, double b) {
    fle_v2d ab = {a, b};
    return fle_mxv_all2(fle_mxv_w2(ab));
}
/* zero, or 2^-400 .. 2^400, for both operands; DIV: a may be zero, b not */
FLE_MXR_AI int fle_mxr_wm_ab64(double a, double b, int divisor) {
    fle_v2d ab = {a, b};
    fle_v2d x = fle_mxv_abs2(ab);
    fle_v2l inw = (x >= 0x1p-400) & (x <= 0x1p400), z = x == 0.0;
    fle_v2l zmask = divisor ? (fle_v2l){-1, 0} : (fle_v2l){-1, -1}; /* a zero is allowed, except as the divisor */
    return fle_mxv_all2(inw | (z & zmask));
}
FLE_MXR_AI int fle_mxr_wab32(float a, float b) {
    fle_v4f ab = {a, b, 1.0f, 1.0f};
    return fle_mxv_all4(fle_mxv_w4(ab));
}
FLE_MXR_AI int fle_mxr_wdiv32(float a, float b) {
    fle_v4f ab = {a, b, 1.0f, 1.0f};
    return fle_mxv_all4(fle_mxv_w4(ab) & ((fle_v4i){-1, 0, -1, -1} | fle_mxv_w4nz(ab)));
}

FLE_MXR_AI uint64_t fle_mxr_addsub64(double a, double b, unsigned rc) {
    FLE_MXV_NOCONTRACT
    double s = a + b, v = s - a, e = (a - (s - v)) + (b - v);
    uint64_t sb = fle_mxr_step64(fle_mxr_ud(s), fle_mxr_ud(e), rc);
    if (rc == 1) { /* an exact zero sum of operands of opposite sign is -0 for RD */
        sb |= (uint64_t)(-(int64_t)((sb << 1) == 0) & ((int64_t)(fle_mxr_ud(a) ^ fle_mxr_ud(b)) >> 63)) & FLE_MXR_S64;
    }
    return sb;
}

#define FLE_MXR_SPLIT 134217729.0 /* 2^27 + 1 */

FLE_MXR_AI uint64_t fle_mxr_mul64(double a, double b, unsigned rc) {
    FLE_MXV_NOCONTRACT
    double p = a * b;
    double ca = FLE_MXR_SPLIT * a, ah = ca - (ca - a), al = a - ah;
    double cb = FLE_MXR_SPLIT * b, bh = cb - (cb - b), bl = b - bh;
    double e = ((ah * bh - p) + ah * bl + al * bh) + al * bl;
    return fle_mxr_step64(fle_mxr_ud(p), fle_mxr_ud(e), rc);
}

FLE_MXR_AI uint64_t fle_mxr_div64(double a, double b, unsigned rc) {
    FLE_MXV_NOCONTRACT
    double q = a / b;
    double p = q * b;
    double cq = FLE_MXR_SPLIT * q, qh = cq - (cq - q), ql = q - qh;
    double cb = FLE_MXR_SPLIT * b, bh = cb - (cb - b), bl = b - bh;
    double e = ((qh * bh - p) + qh * bl + ql * bh) + ql * bl;
    double r = (a - p) - e;
    return fle_mxr_step64(fle_mxr_ud(q), fle_mxr_ud(r) ^ (fle_mxr_ud(b) & FLE_MXR_S64), rc);
}

FLE_MXR_AI uint64_t fle_mxr_sqrt64(double b, unsigned rc) {
    FLE_MXV_NOCONTRACT
    double q = FLE_MXF_SQRT(b);
    double p = q * q;
    double cq = FLE_MXR_SPLIT * q, qh = cq - (cq - q), ql = q - qh;
    double e = ((qh * qh - p) + qh * ql + ql * qh) + ql * ql;
    double r = (b - p) - e;
    return fle_mxr_step64(fle_mxr_ud(q), fle_mxr_ud(r), rc);
}

FLE_MXR_AI uint32_t fle_mxr_addsub32(float a, float b, unsigned rc) {
    FLE_MXV_NOCONTRACT
    float s = a + b, v = s - a, e = (a - (s - v)) + (b - v);
    uint32_t eb = fle_mxr_uf(e), sb = fle_mxr_step32(fle_mxr_uf(s), FLE_MXR_E32(eb), rc);
    if (rc == 1) {
        sb |= (uint32_t)(-(int32_t)((sb << 1) == 0) & ((int32_t)(fle_mxr_uf(a) ^ fle_mxr_uf(b)) >> 31)) & FLE_MXR_S32;
    }
    return sb;
}

FLE_MXR_AI uint32_t fle_mxr_mul32(float a, float b, unsigned rc) {
    FLE_MXV_NOCONTRACT
    double d = (double)a * (double)b;
    float s = (float)d;
    uint64_t eb = fle_mxr_ud(d - (double)s);
    return fle_mxr_step32(fle_mxr_uf(s), FLE_MXR_E64T32(eb), rc);
}

FLE_MXR_AI uint32_t fle_mxr_div32(float a, float b, unsigned rc) {
    FLE_MXV_NOCONTRACT
    float q = a / b;
    double r = (double)a - (double)q * (double)b;
    uint64_t eb = fle_mxr_ud(r) ^ ((uint64_t)fle_mxr_uf(b) << 32 & FLE_MXR_S64);
    return fle_mxr_step32(fle_mxr_uf(q), FLE_MXR_E64T32(eb), rc);
}

FLE_MXR_AI uint32_t fle_mxr_sqrt32(float b, unsigned rc) {
    FLE_MXV_NOCONTRACT
    float q = (float)FLE_MXF_SQRT(b);
    double r = (double)b - (double)q * (double)q;
    uint64_t eb = fle_mxr_ud(r);
    return fle_mxr_step32(fle_mxr_uf(q), FLE_MXR_E64T32(eb), rc);
}

/* ---- two binary64 lanes and four binary32 lanes at once (the vector extension: SSE2 on x86, v128 on wasm with -msimd128,
 * lane by lane elsewhere). Error terms and the step are the same formulas on vectors. ---- */
#ifdef FLE_MXV_SQRT
FLE_MXR_AI fle_v2l fle_mxr_step2(fle_v2l sb, fle_v2l en, fle_v2l ep, unsigned rc) {
    fle_v2l ssg = sb >> 63, delta;
    if (rc == 3) {
        delta = (en & ~ssg) | (ep & ssg);
    } else if (rc == 1) {
        delta = en & ((ssg & 2) - 1);
    } else {
        delta = ep & (1 - (ssg & 2));
    }
    return sb + delta;
}
FLE_MXR_AI fle_v4i fle_mxr_step4(fle_v4i sb, fle_v4i en, fle_v4i ep, unsigned rc) {
    fle_v4i ssg = sb >> 31, delta;
    if (rc == 3) {
        delta = (en & ~ssg) | (ep & ssg);
    } else if (rc == 1) {
        delta = en & ((ssg & 2) - 1);
    } else {
        delta = ep & (1 - (ssg & 2));
    }
    return sb + delta;
}
FLE_MXR_AI fle_v2d fle_mxr_addsub2(fle_v2d a, fle_v2d b, unsigned rc) {
    FLE_MXV_NOCONTRACT
    fle_v2d s = a + b, v = s - a, e = (a - (s - v)) + (b - v);
    fle_v2l en = e < 0, ep = e > 0, sb = (fle_v2l)s;
    sb = fle_mxr_step2(sb, en, ep, rc);
    if (rc == 1) {
        fle_v2l sg = {INT64_MIN, INT64_MIN};
        sb |= ((sb << 1) == 0) & (((fle_v2l)a ^ (fle_v2l)b) >> 63) & sg;
    }
    return (fle_v2d)sb;
}
FLE_MXR_AI fle_v2d fle_mxr_mul2(fle_v2d a, fle_v2d b, unsigned rc) {
    FLE_MXV_NOCONTRACT
    fle_v2d p = a * b;
    fle_v2d ca = FLE_MXR_SPLIT * a, ah = ca - (ca - a), al = a - ah;
    fle_v2d cb = FLE_MXR_SPLIT * b, bh = cb - (cb - b), bl = b - bh;
    fle_v2d e = ((ah * bh - p) + ah * bl + al * bh) + al * bl;
    return (fle_v2d)fle_mxr_step2((fle_v2l)p, e < 0, e > 0, rc);
}
FLE_MXR_AI fle_v2d fle_mxr_div2(fle_v2d a, fle_v2d b, unsigned rc) {
    FLE_MXV_NOCONTRACT
    fle_v2d q = a / b, p = q * b;
    fle_v2d cq = FLE_MXR_SPLIT * q, qh = cq - (cq - q), ql = q - qh;
    fle_v2d cb = FLE_MXR_SPLIT * b, bh = cb - (cb - b), bl = b - bh;
    fle_v2d e = ((qh * bh - p) + qh * bl + ql * bh) + ql * bl;
    fle_v2d r = (a - p) - e;
    fle_v2l sg = {INT64_MIN, INT64_MIN};
    r = (fle_v2d)((fle_v2l)r ^ ((fle_v2l)b & sg));
    return (fle_v2d)fle_mxr_step2((fle_v2l)q, r < 0, r > 0, rc);
}
FLE_MXR_AI fle_v2d fle_mxr_sqrt2(fle_v2d b, unsigned rc) {
    FLE_MXV_NOCONTRACT
    fle_v2d q = __builtin_elementwise_sqrt(b), p = q * q;
    fle_v2d cq = FLE_MXR_SPLIT * q, qh = cq - (cq - q), ql = q - qh;
    fle_v2d e = ((qh * qh - p) + qh * ql + ql * qh) + ql * ql;
    fle_v2d r = (b - p) - e;
    return (fle_v2d)fle_mxr_step2((fle_v2l)q, r < 0, r > 0, rc);
}
FLE_MXR_AI fle_v4f fle_mxr_addsub4(fle_v4f a, fle_v4f b, unsigned rc) {
    FLE_MXV_NOCONTRACT
    fle_v4f s = a + b, v = s - a, e = (a - (s - v)) + (b - v);
    fle_v4i en = e < 0, ep = e > 0, sb = (fle_v4i)s;
    sb = fle_mxr_step4(sb, en, ep, rc);
    if (rc == 1) {
        fle_v4i sg = {INT32_MIN, INT32_MIN, INT32_MIN, INT32_MIN};
        sb |= ((sb << 1) == 0) & (((fle_v4i)a ^ (fle_v4i)b) >> 31) & sg;
    }
    return (fle_v4f)sb;
}
FLE_MXR_AI fle_v2d fle_mxr_ldd(const fle_xmm *x) {
    return fle_mxv_ldd(x);
}
/* zero, or 2^-400 <= |x| <= 2^400; the divisor: nonzero; the root: +-0 or 2^-400 <= x <= 2^400 */
FLE_MXR_AI fle_v2l fle_mxr_wm2(fle_v2d x) {
    fle_v2d a = fle_mxv_abs2(x);
    return ((a >= 0x1p-400) & (a <= 0x1p400)) | (a == 0.0);
}
FLE_MXR_AI fle_v2l fle_mxr_wm2nz(fle_v2d x) {
    fle_v2d a = fle_mxv_abs2(x);
    return (a >= 0x1p-400) & (a <= 0x1p400);
}
FLE_MXR_AI fle_v2l fle_mxr_wm2sqrt(fle_v2d x) {
    fle_v2d a = fle_mxv_abs2(x);
    return ((x >= 0x1p-400) & (x <= 0x1p400)) | (a == 0.0);
}
/* four binary32 lanes through two double lanes each (products, quotient remainders and root remainders are exact in double) */
typedef float fle_mxr_v2f __attribute__((vector_size(8)));
FLE_MXR_AI fle_v2d fle_mxr_lo(fle_v4f x) {
    return __builtin_convertvector(__builtin_shufflevector(x, x, 0, 1), fle_v2d);
}
FLE_MXR_AI fle_v2d fle_mxr_hi(fle_v4f x) {
    return __builtin_convertvector(__builtin_shufflevector(x, x, 2, 3), fle_v2d);
}
FLE_MXR_AI fle_v4f fle_mxr_pack(fle_v2d lo, fle_v2d hi) {
    return __builtin_shufflevector(__builtin_convertvector(lo, fle_mxr_v2f), __builtin_convertvector(hi, fle_mxr_v2f), 0, 1, 2, 3);
}
FLE_MXR_AI fle_v4i fle_mxr_mask4(fle_v2l lo, fle_v2l hi) { /* two masks of 64-bit lanes to one mask of 32-bit lanes */
    return __builtin_shufflevector((fle_v4i)lo, (fle_v4i)hi, 0, 2, 4, 6);
}
FLE_MXR_AI fle_v4f fle_mxr_mul4(fle_v4f a, fle_v4f b, unsigned rc) {
    FLE_MXV_NOCONTRACT
    fle_v2d plo = fle_mxr_lo(a) * fle_mxr_lo(b), phi = fle_mxr_hi(a) * fle_mxr_hi(b);
    fle_v4f s = fle_mxr_pack(plo, phi);
    fle_v2d elo = plo - fle_mxr_lo(s), ehi = phi - fle_mxr_hi(s);
    return (fle_v4f)fle_mxr_step4((fle_v4i)s, fle_mxr_mask4(elo < 0, ehi < 0), fle_mxr_mask4(elo > 0, ehi > 0), rc);
}
FLE_MXR_AI fle_v4f fle_mxr_div4(fle_v4f a, fle_v4f b, unsigned rc) {
    FLE_MXV_NOCONTRACT
    fle_v4f q = a / b;
    fle_v2d rlo = fle_mxr_lo(a) - fle_mxr_lo(q) * fle_mxr_lo(b), rhi = fle_mxr_hi(a) - fle_mxr_hi(q) * fle_mxr_hi(b);
    fle_v4i en = fle_mxr_mask4(rlo < 0, rhi < 0), ep = fle_mxr_mask4(rlo > 0, rhi > 0), bs = (fle_v4i)b >> 31;
    return (fle_v4f)fle_mxr_step4((fle_v4i)q, (en & ~bs) | (ep & bs), (ep & ~bs) | (en & bs), rc);
}
FLE_MXR_AI fle_v4f fle_mxr_sqrt4(fle_v4f b, unsigned rc) {
    FLE_MXV_NOCONTRACT
    fle_v4f q = __builtin_elementwise_sqrt(b);
    fle_v2d rlo = fle_mxr_lo(b) - fle_mxr_lo(q) * fle_mxr_lo(q), rhi = fle_mxr_hi(b) - fle_mxr_hi(q) * fle_mxr_hi(q);
    return (fle_v4f)fle_mxr_step4((fle_v4i)q, fle_mxr_mask4(rlo < 0, rhi < 0), fle_mxr_mask4(rlo > 0, rhi > 0), rc);
}
#endif /* FLE_MXV_SQRT */

/* ---- one function per instruction and rounding mode: *r = op(*d, *s), the scalar forms keeping the upper lanes of *d. When
 * a lane is outside the windows the function calls fb (the integer implementation of this translation unit) with the
 * operation. The caller (fle_mx_do2_slow_p) has checked FLE_MXR_ON(fle_mxcsr). The functions are indexed by
 * fle_mxrc_tab[rc][op + 8 * f64 + 16 * packed]: with RC a constant in each of them the code is the straight line of one mode. ---- */
typedef void (*fle_mxr_fb_t)(int id, const fle_xmm *d, const fle_xmm *s, fle_xmm *r);
typedef void (*fle_mxr_fn_t)(const fle_xmm *d, const fle_xmm *s, fle_xmm *r, fle_mxr_fb_t fb);

#define FLE_MXR_MISS(ID, OP, F64, SCALAR) \
    do { \
        FLE_MXRS(2, ID); \
        fb(ID, d, s, r); \
        return; \
    } while (0)

#define FLE_MXR_3(DEF, NAME, ...) DEF(NAME##_1, 1, __VA_ARGS__) DEF(NAME##_2, 2, __VA_ARGS__) DEF(NAME##_3, 3, __VA_ARGS__)

#define FLE_MXR_S64DEF(FN, RC, ID, OP, WIN, CALL) \
    FLE_MXR_FN void FN(const fle_xmm *d, const fle_xmm *s, fle_xmm *r, fle_mxr_fb_t fb) { \
        double a = d->f64[0], b = s->f64[0]; \
        uint64_t ub = s->u64[0]; \
        (void)a; \
        (void)ub; \
        if (!(WIN)) FLE_MXR_MISS(ID, OP, 1, 1); \
        r->u64[0] = CALL(RC); \
        r->u64[1] = d->u64[1]; \
        FLE_MXRS(0, ID); \
    }
#define FLE_MXR_S32DEF(FN, RC, ID, OP, WIN, CALL) \
    FLE_MXR_FN void FN(const fle_xmm *d, const fle_xmm *s, fle_xmm *r, fle_mxr_fb_t fb) { \
        float a = d->f32[0], b = s->f32[0]; \
        uint32_t ub = s->u32[0]; \
        (void)a; \
        (void)ub; \
        if (!(WIN)) FLE_MXR_MISS(ID, OP, 0, 1); \
        r->u64[0] = (d->u64[0] & UINT64_C(0xffffffff00000000)) | CALL(RC); \
        r->u64[1] = d->u64[1]; \
        FLE_MXRS(0, ID); \
    }
#define FLE_MXR_ADD64(rc) fle_mxr_addsub64(a, b, rc)
#define FLE_MXR_SUB64(rc) fle_mxr_addsub64(a, -b, rc)
#define FLE_MXR_MUL64(rc) fle_mxr_mul64(a, b, rc)
#define FLE_MXR_DIV64(rc) fle_mxr_div64(a, b, rc)
#define FLE_MXR_SQRT64(rc) fle_mxr_sqrt64(b, rc)
#define FLE_MXR_ADD32(rc) fle_mxr_addsub32(a, b, rc)
#define FLE_MXR_SUB32(rc) fle_mxr_addsub32(a, -b, rc)
#define FLE_MXR_MUL32(rc) fle_mxr_mul32(a, b, rc)
#define FLE_MXR_DIV32(rc) fle_mxr_div32(a, b, rc)
#define FLE_MXR_SQRT32(rc) fle_mxr_sqrt32(b, rc)

FLE_MXR_3(FLE_MXR_S64DEF, fle_mxrc_addsd, 8, FLE_MXO_ADD, fle_mxr_wab64(a, b), FLE_MXR_ADD64)
FLE_MXR_3(FLE_MXR_S64DEF, fle_mxrc_subsd, 9, FLE_MXO_SUB, fle_mxr_wab64(a, b), FLE_MXR_SUB64)
FLE_MXR_3(FLE_MXR_S64DEF, fle_mxrc_mulsd, 10, FLE_MXO_MUL, fle_mxr_wm_ab64(a, b, 0), FLE_MXR_MUL64)
FLE_MXR_3(FLE_MXR_S64DEF, fle_mxrc_divsd, 11, FLE_MXO_DIV, fle_mxr_wm_ab64(a, b, 1), FLE_MXR_DIV64)
FLE_MXR_3(FLE_MXR_S64DEF, fle_mxrc_sqrtsd, 14, FLE_MXO_SQRT, fle_mxr_wm64sqrt(ub), FLE_MXR_SQRT64)
FLE_MXR_3(FLE_MXR_S32DEF, fle_mxrc_addss, 0, FLE_MXO_ADD, fle_mxr_wab32(a, b), FLE_MXR_ADD32)
FLE_MXR_3(FLE_MXR_S32DEF, fle_mxrc_subss, 1, FLE_MXO_SUB, fle_mxr_wab32(a, b), FLE_MXR_SUB32)
FLE_MXR_3(FLE_MXR_S32DEF, fle_mxrc_mulss, 2, FLE_MXO_MUL, fle_mxr_wab32(a, b), FLE_MXR_MUL32)
FLE_MXR_3(FLE_MXR_S32DEF, fle_mxrc_divss, 3, FLE_MXO_DIV, fle_mxr_wdiv32(a, b), FLE_MXR_DIV32)
FLE_MXR_3(FLE_MXR_S32DEF, fle_mxrc_sqrtss, 6, FLE_MXO_SQRT, fle_mxv_ws32sqrt(ub), FLE_MXR_SQRT32)

#ifdef FLE_MXV_SQRT
#define FLE_MXR_V2DEF(FN, RC, ID, OP, WIN, CALL) \
    FLE_MXR_FN void FN(const fle_xmm *d, const fle_xmm *s, fle_xmm *r, fle_mxr_fb_t fb) { \
        fle_v2d a = fle_mxv_ldd(d), b = fle_mxv_ldd(s); \
        (void)a; \
        if (!fle_mxv_all2(WIN)) FLE_MXR_MISS(ID, OP, 1, 0); \
        *r = fle_mxv_std(CALL(RC)); \
        FLE_MXRS(0, ID); \
    }
#define FLE_MXR_ADD2(rc) fle_mxr_addsub2(a, b, rc)
#define FLE_MXR_SUB2(rc) fle_mxr_addsub2(a, (fle_v2d)((fle_v2l)b ^ (fle_v2l){INT64_MIN, INT64_MIN}), rc)
#define FLE_MXR_MUL2(rc) fle_mxr_mul2(a, b, rc)
#define FLE_MXR_DIV2(rc) fle_mxr_div2(a, b, rc)
#define FLE_MXR_SQRT2(rc) fle_mxr_sqrt2(b, rc)
FLE_MXR_3(FLE_MXR_V2DEF, fle_mxrc_addpd, 24, FLE_MXO_ADD, fle_mxv_w2(a) & fle_mxv_w2(b), FLE_MXR_ADD2)
FLE_MXR_3(FLE_MXR_V2DEF, fle_mxrc_subpd, 25, FLE_MXO_SUB, fle_mxv_w2(a) & fle_mxv_w2(b), FLE_MXR_SUB2)
FLE_MXR_3(FLE_MXR_V2DEF, fle_mxrc_mulpd, 26, FLE_MXO_MUL, fle_mxr_wm2(a) & fle_mxr_wm2(b), FLE_MXR_MUL2)
FLE_MXR_3(FLE_MXR_V2DEF, fle_mxrc_divpd, 27, FLE_MXO_DIV, fle_mxr_wm2(a) & fle_mxr_wm2nz(b), FLE_MXR_DIV2)
FLE_MXR_3(FLE_MXR_V2DEF, fle_mxrc_sqrtpd, 30, FLE_MXO_SQRT, fle_mxr_wm2sqrt(b), FLE_MXR_SQRT2)

#define FLE_MXR_V4DEF(FN, RC, ID, OP, WIN, CALL) \
    FLE_MXR_FN void FN(const fle_xmm *d, const fle_xmm *s, fle_xmm *r, fle_mxr_fb_t fb) { \
        fle_v4f a = fle_mxv_ldf(d), b = fle_mxv_ldf(s); \
        (void)a; \
        if (!fle_mxv_all4(WIN)) FLE_MXR_MISS(ID, OP, 0, 0); \
        *r = fle_mxv_stf(CALL(RC)); \
        FLE_MXRS(0, ID); \
    }
#define FLE_MXR_ADD4(rc) fle_mxr_addsub4(a, b, rc)
#define FLE_MXR_SUB4(rc) fle_mxr_addsub4(a, (fle_v4f)((fle_v4i)b ^ (fle_v4i){INT32_MIN, INT32_MIN, INT32_MIN, INT32_MIN}), rc)
#define FLE_MXR_MUL4(rc) fle_mxr_mul4(a, b, rc)
#define FLE_MXR_DIV4(rc) fle_mxr_div4(a, b, rc)
#define FLE_MXR_SQRT4(rc) fle_mxr_sqrt4(b, rc)
FLE_MXR_3(FLE_MXR_V4DEF, fle_mxrc_addps, 16, FLE_MXO_ADD, fle_mxv_w4(a) & fle_mxv_w4(b), FLE_MXR_ADD4)
FLE_MXR_3(FLE_MXR_V4DEF, fle_mxrc_subps, 17, FLE_MXO_SUB, fle_mxv_w4(a) & fle_mxv_w4(b), FLE_MXR_SUB4)
FLE_MXR_3(FLE_MXR_V4DEF, fle_mxrc_mulps, 18, FLE_MXO_MUL, fle_mxv_w4(a) & fle_mxv_w4(b), FLE_MXR_MUL4)
FLE_MXR_3(FLE_MXR_V4DEF, fle_mxrc_divps, 19, FLE_MXO_DIV, fle_mxv_w4(a) & fle_mxv_w4nz(b), FLE_MXR_DIV4)
FLE_MXR_3(FLE_MXR_V4DEF, fle_mxrc_sqrtps, 22, FLE_MXO_SQRT, fle_mxv_w4sqrt(b), FLE_MXR_SQRT4)
#define FLE_MXR_HAVE_VEC 1
#endif /* FLE_MXV_SQRT */

#define FLE_MXR_ROW(rc) \
    [0 + 0] = fle_mxrc_addss_##rc, [1] = fle_mxrc_subss_##rc, [2] = fle_mxrc_mulss_##rc, [3] = fle_mxrc_divss_##rc, [6] = fle_mxrc_sqrtss_##rc, \
    [8] = fle_mxrc_addsd_##rc, [9] = fle_mxrc_subsd_##rc, [10] = fle_mxrc_mulsd_##rc, [11] = fle_mxrc_divsd_##rc, [14] = fle_mxrc_sqrtsd_##rc
#ifdef FLE_MXR_HAVE_VEC
#define FLE_MXR_ROWV(rc) \
    [16] = fle_mxrc_addps_##rc, [17] = fle_mxrc_subps_##rc, [18] = fle_mxrc_mulps_##rc, [19] = fle_mxrc_divps_##rc, [22] = fle_mxrc_sqrtps_##rc, \
    [24] = fle_mxrc_addpd_##rc, [25] = fle_mxrc_subpd_##rc, [26] = fle_mxrc_mulpd_##rc, [27] = fle_mxrc_divpd_##rc, [30] = fle_mxrc_sqrtpd_##rc
#else
#define FLE_MXR_ROWV(rc)
#endif
/* [rc][op + 8 * f64 + 16 * packed]; a null entry (MIN, MAX, or no vector extension) means the integer implementation */
__attribute__((weak)) const
fle_mxr_fn_t fle_mxrc_tab[4][32] = {{0}, {FLE_MXR_ROW(1), FLE_MXR_ROWV(1)}, {FLE_MXR_ROW(2), FLE_MXR_ROWV(2)}, {FLE_MXR_ROW(3), FLE_MXR_ROWV(3)}};

/* ---- conversions (rare in comparison with the arithmetic; the rounding control is a run-time value here). Each returns 0, writing
 * nothing, when an operand is outside its window. The caller has checked FLE_MXR_ON(fle_mxcsr). Truncating conversions (CVTT*) do not
 * depend on RC: they only need the guard on PE and PM, and take the vector-path code. ---- */

/* binary64 to binary32 (CVTSD2SS, CVTPD2PS): zero, or 2^-100 <= |x| < 2^101, so that the binary32 result is a normal number */
FLE_MXR_AI uint32_t fle_mxr_d2f(double x, unsigned rc) {
    FLE_MXV_NOCONTRACT
    float s = (float)x;
    uint64_t eb = fle_mxr_ud(x - (double)s); /* exact */
    return fle_mxr_step32(fle_mxr_uf(s), FLE_MXR_E64T32(eb), rc);
}
FLE_MXR_FN int fle_mxrc_cvtsd2ss(const fle_xmm *d, const fle_xmm *s, fle_xmm *r) {
    unsigned rc = FLE_MXR_RC(fle_mxcsr);
    if (!fle_mxv_cw64(s->u64[0], 1023 - 100, 1023 + 100)) {
        FLE_MXRS(2, FLE_MXV_CVTSD2SS);
        return 0;
    }
    *r = *d;
    r->u32[0] = fle_mxr_d2f(s->f64[0], rc);
    FLE_MXRS(0, FLE_MXV_CVTSD2SS);
    return 1;
}
FLE_MXR_FN int fle_mxrc_cvtpd2ps(const fle_xmm *d, const fle_xmm *s, fle_xmm *r) {
    unsigned rc = FLE_MXR_RC(fle_mxcsr);
    if (!(fle_mxv_cw64(s->u64[0], 1023 - 100, 1023 + 100) & fle_mxv_cw64(s->u64[1], 1023 - 100, 1023 + 100))) {
        FLE_MXRS(2, FLE_MXV_CVTPD2PS);
        return 0;
    }
    (void)d;
    r->u32[0] = fle_mxr_d2f(s->f64[0], rc);
    r->u32[1] = fle_mxr_d2f(s->f64[1], rc);
    r->u64[1] = 0;
    FLE_MXRS(0, FLE_MXV_CVTPD2PS);
    return 1;
}

/* integer to binary32 / binary64 (CVTSI2SS CVTSI2SD CVTDQ2PS): the exact ranges never get here; |v| < 2^62 for the 64-bit sources so that the
 * rounded value is an int64 again and the error v - (int64)s is exact */
FLE_MXR_AI uint32_t fle_mxr_i2f32(int64_t v, unsigned rc) {
    float s = (float)v;
    int64_t err = v - (int64_t)s;
    return fle_mxr_step32(fle_mxr_uf(s), (int32_t)(err >> 63), -(int32_t)(err != 0), rc);
}
FLE_MXR_AI uint64_t fle_mxr_i2f64(int64_t v, unsigned rc) {
    double s = (double)v;
    int64_t err = v - (int64_t)s;
    uint64_t sb = fle_mxr_ud(s), delta;
    int64_t ssg = (int64_t)sb >> 63, esg = err >> 63, nz = -(int64_t)(err != 0);
    if (rc == 3) {
        delta = (uint64_t)((esg ^ ssg) & nz);
    } else if (rc == 1) {
        delta = (uint64_t)(esg & nz & ((ssg & 2) - 1));
    } else {
        delta = (uint64_t)(~esg & nz & (1 - (ssg & 2)));
    }
    return sb + delta;
}
/* in64 = 0: a 32-bit source; f64 selects the destination. *out holds the bits of the result. */
FLE_MXR_FN int fle_mxrc_cvtsi2f(int f64, int in64, int64_t v, uint64_t *out) {
    unsigned rc = FLE_MXR_RC(fle_mxcsr);
    int id = f64 ? (in64 ? FLE_MXV_CVTSI2SD64 : FLE_MXV_CVTSI2SD32) : (in64 ? FLE_MXV_CVTSI2SS64 : FLE_MXV_CVTSI2SS32);
    if (in64 && (uint64_t)v + (UINT64_C(1) << 62) >= (UINT64_C(1) << 63)) {
        FLE_MXRS(2, id);
        return 0;
    }
    *out = f64 ? fle_mxr_i2f64(v, rc) : fle_mxr_i2f32(v, rc);
    FLE_MXRS(0, id);
    return 1;
}
FLE_MXR_FN int fle_mxrc_cvtdq2ps(const fle_xmm *d, const fle_xmm *s, fle_xmm *r) {
    unsigned rc = FLE_MXR_RC(fle_mxcsr);
    (void)d;
    r->u32[0] = fle_mxr_i2f32((int32_t)s->u32[0], rc);
    r->u32[1] = fle_mxr_i2f32((int32_t)s->u32[1], rc);
    r->u32[2] = fle_mxr_i2f32((int32_t)s->u32[2], rc);
    r->u32[3] = fle_mxr_i2f32((int32_t)s->u32[3], rc);
    FLE_MXRS(0, FLE_MXV_CVTDQ2PS);
    return 1;
}

/* binary to integer with the rounding of RC (CVTSS2SI CVTSD2SI CVTPS2DQ CVTPD2DQ): zero, or any normal number with |x| < 2^31 - 1 (32-bit
 * result) / 2^51 (64-bit result); the result is floor (RD), ceil (RU) or truncation (RZ) of x, computed from the truncation and a comparison. */
FLE_MXR_AI int64_t fle_mxr_f2i(double x, unsigned rc) {
    int64_t t = (int64_t)x;
    double tf = (double)t;
    return rc == 1 ? t - (int64_t)(x < tf) : (rc == 2 ? t + (int64_t)(x > tf) : t);
}
FLE_MXR_FN int fle_mxrc_cvt2si(int isd, int out64, uint64_t bits, uint64_t *out) {
    unsigned rc = FLE_MXR_RC(fle_mxcsr);
    int id = isd ? (out64 ? FLE_MXV_CVTSD2SI64 : FLE_MXV_CVTSD2SI32) : (out64 ? FLE_MXV_CVTSS2SI64 : FLE_MXV_CVTSS2SI32);
    double x;
    if (isd ? !fle_mxv_cw64(bits, 1, 2046) : !fle_mxv_cw32((uint32_t)bits, 1, 254)) {
        FLE_MXRS(2, id);
        return 0;
    }
    x = isd ? fle_mxr_d(bits) : (double)fle_mxr_f((uint32_t)bits);
    if (!fle_mxv_f2i_rne_ok(x, out64)) {
        FLE_MXRS(2, id);
        return 0;
    }
    {
        int64_t i = fle_mxr_f2i(x, rc);
        *out = out64 ? (uint64_t)i : (uint64_t)(uint32_t)(int32_t)i;
    }
    FLE_MXRS(0, id);
    return 1;
}
FLE_MXR_FN int fle_mxrc_cvtps2dq(const fle_xmm *d, const fle_xmm *s, fle_xmm *r) {
    unsigned rc = FLE_MXR_RC(fle_mxcsr);
    int k;
    (void)d;
    for (k = 0; k < 4; k++) {
        if (!fle_mxv_cw32(s->u32[k], 1, 127 + 30)) {
            FLE_MXRS(2, FLE_MXV_CVTPS2DQ);
            return 0;
        }
    }
    for (k = 0; k < 4; k++) {
        r->u32[k] = (uint32_t)(int32_t)fle_mxr_f2i((double)s->f32[k], rc);
    }
    FLE_MXRS(0, FLE_MXV_CVTPS2DQ);
    return 1;
}
FLE_MXR_FN int fle_mxrc_cvtpd2dq(const fle_xmm *d, const fle_xmm *s, fle_xmm *r) {
    unsigned rc = FLE_MXR_RC(fle_mxcsr);
    (void)d;
    if (!(fle_mxv_cw64(s->u64[0], 1, 2046) & fle_mxv_cw64(s->u64[1], 1, 2046) & fle_mxv_f2i_rne_ok(s->f64[0], 0) & fle_mxv_f2i_rne_ok(s->f64[1], 0))) {
        FLE_MXRS(2, FLE_MXV_CVTPD2DQ);
        return 0;
    }
    r->u32[0] = (uint32_t)(int32_t)fle_mxr_f2i(s->f64[0], rc);
    r->u32[1] = (uint32_t)(int32_t)fle_mxr_f2i(s->f64[1], rc);
    r->u64[1] = 0;
    FLE_MXRS(0, FLE_MXV_CVTPD2DQ);
    return 1;
}

#endif /* FLE_MX_VEC */
#endif /* FLE_MXCSR_RC_H */
