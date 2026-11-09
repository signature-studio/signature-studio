/* Guarded narrow-domain fast path for SSE floating point (decision 0154). Included by fle_mxcsr_flags.h, after the
 * per-lane fast path of decision 0143 and before the instruction wrappers.
 *
 * Idea. A whole instruction (all the lanes it uses) is decided at once. When the guest control state allows it (below)
 * and every lane it uses has operands that are zero or have a magnitude in a narrow window, the exact IEEE result is
 * a normal number (or an exact zero) for every lane and no status flag but PE can be raised; PE is already set in the
 * guest MXCSR and the precision exception is masked, so MXCSR does not change. Then the instruction is the host's
 * float/double operation on the whole 128-bit value (a vector_size(16) operation: SSE on x86, v128 on wasm with
 * -msimd128, scalar code on wasm without it), nothing else is written, and no flag is touched. Anything else takes
 * the code of decision 0143 (per-lane fast path, then the integer path), unchanged, out of line.
 *
 * The windows (the whole proof is "inside the window nothing but PE can happen and the host operation is the
 * correctly rounded result"):
 *   binary32: |x| = 0 or 2^-62 <= |x| <= 2^62.  binary64: |x| = 0 or 2^-510 <= |x| <= 2^510.
 *   For + and -: the exact sum is a multiple of ulp(2^-62) = 2^-85 (binary64: 2^-562), so it is 0 or at least 2^-85,
 *     far above the smallest normal 2^-126 (2^-1022), and at most 2^63: no overflow, no tiny result, no denormal.
 *   For *: |a*b| is 0 or in [2^-124, 2^124] (binary64: [2^-1020, 2^1020]): normal, no overflow, no underflow.
 *   For /: the divisor is nonzero and in the window, |a/b| is 0 or in [2^-124, 2^124]; no ZE, no IE (0/0 needs a zero
 *     divisor).
 *   For sqrt: the operand is +-0 (the result is the operand, no flag) or positive in the window (root in [2^-31, 2^31]).
 *   NaN fails every comparison, so a NaN lane is never accepted; an infinity or a denormal is outside the window.
 *   MIN and MAX raise no flag for zeros, normals and infinities (no NaN, no denormal): window = those, and RC, PE and
 *   the masks do not matter. The result is "a < b ? a : b" ("a > b ? a : b"), the second operand when equal (+0/-0).
 *   CMP (all 8 predicates): no flag for operands that are not NaN and not denormal (zero, normal, infinity); window =
 *   those; RC, PE and the masks do not matter; predicates 3 and 7 are constants then.
 *   Conversions: see each one.
 * RC = 0 (round to nearest even) is required wherever the result may be inexact. FTZ and DAZ do not matter: they
 * change only denormal operands and tiny results, which the windows exclude.
 * Guard for the operations whose result may be inexact: (MXCSR & 0x7020) == 0x1020: RC = 0, PE set, PM (bit 12) set.
 * Not here: a lane with PE not yet set (it is set by the first inexact operation through the slow code), RC != 0.
 *
 * (Decision 0243: -DFLE_MX_NOVEC and the operand trace -DFLE_MX_TRACE are gone.)
 */
#ifndef FLE_MXCSR_VEC_H
#define FLE_MXCSR_VEC_H

#if defined(FLE_MX_FAST) && (defined(__clang__) || defined(__GNUC__))
#define FLE_MX_VEC 1

/* The guard of the operations that can be inexact: the control bits of FLE_MX_FAST_CTL (RC)
 * must be 0 and PM (bit 12) and PE (bit 5) must be set. The operations that raise no flag in their window (MIN MAX CMP the exact conversions) only
 * look at the bits of FLE_MX_FAST_CTL that are not RC, so they are never refused for the control state. */
#define FLE_MXV_CTL (FLE_MX_FAST_CTL | 0x1020u)
#define FLE_MXV_OK 0x1020u /* RC = 0, PM set, PE set */
#define FLE_MXV_NP0(mx) (((mx) & FLE_MXV_CTL) == FLE_MXV_OK)
#define FLE_MXV_CTL_OK(mx) (((mx) & (FLE_MX_FAST_CTL & ~0x6000u)) == 0)
/* the guard of an operation whose result does not depend on RC but can be inexact (truncating conversions): PE set and PM masked (decision 0172) */
#define FLE_MXV_NP0X(mx) (((mx) & (0x1020u | (FLE_MX_FAST_CTL & ~0x6000u))) == 0x1020u)
#define FLE_MXV_TLO64 1u /* the lowest biased exponent of the truncating conversions: every normal number (decision 0172) */
#define FLE_MXV_TLO32 1u

/* How the code of decision 0143 is called when the fast path does not apply: with the operands and the result by value (the wasm code with -msimd128 is 3.4 times
 * faster on K1 that way: 1.38 against 4.75 ns/sample) or through pointers (native x86-64: K5 2.3 against 4.4 ns/sample, K1 and K6 about 10% better; wasm without
 * -msimd128: no difference). */
#if defined(__wasm_simd128__)
#define FLE_MXV_SLOWBYVAL 1
#else
#define FLE_MXV_SLOWBYVAL 0
#endif

/* helper functions are always inlined: they are constant-folded by the (constant) operation of each wrapper */
#define FLE_MXV_AI static inline __attribute__((always_inline))
/* the wrappers: the compiler decides */
#define FLE_MXV_WAI static inline

typedef float fle_v4f __attribute__((vector_size(16)));
typedef double fle_v2d __attribute__((vector_size(16)));
typedef int fle_v4i __attribute__((vector_size(16)));
typedef uint32_t fle_v4u __attribute__((vector_size(16)));
typedef long long fle_v2l __attribute__((vector_size(16)));
typedef uint64_t fle_v2u __attribute__((vector_size(16)));

#if defined(__clang__)
#define FLE_MXV_NOCONTRACT _Pragma("clang fp contract(off)")
#else
#define FLE_MXV_NOCONTRACT
#endif

#if defined(__has_builtin)
#if __has_builtin(__builtin_elementwise_sqrt)
#define FLE_MXV_SQRT 1
#endif
#endif

#define FLE_MXV_LO32 0x1p-62f
#define FLE_MXV_HI32 0x1p62f
#define FLE_MXV_LO64 0x1p-510
#define FLE_MXV_HI64 0x1p510
#define FLE_MXV_MIN32 0x1p-126f
#define FLE_MXV_MIN64 0x1p-1022

/* statistics for measurement (-DFLE_MX_STATS): [0] vector path taken, [1] not taken because the guard on PE and RC
 * failed, [2] not taken for another reason (a lane outside the window); index = op + 8 * f64 + 16 * packed (the cmp,
 * cvt and si2 classes have their own ids, see FLE_MXV_ID_* in the wrappers) */
#ifdef FLE_MX_STATS
extern uint64_t fle_mx_vstat[3][64];
#define FLE_MXVS(kind, id) (fle_mx_vstat[(kind)][(id)]++)
#else
#define FLE_MXVS(kind, id) ((void)(kind), (void)(id))
#endif

/* ---- bit casts of a fle_xmm ---- */
FLE_MXV_AI fle_v4f fle_mxv_ldf(const fle_xmm *x) {
    fle_v4f v;
    __builtin_memcpy(&v, x, 16);
    return v;
}
FLE_MXV_AI fle_v2d fle_mxv_ldd(const fle_xmm *x) {
    fle_v2d v;
    __builtin_memcpy(&v, x, 16);
    return v;
}
FLE_MXV_AI fle_xmm fle_mxv_stf(fle_v4f v) {
    fle_xmm x;
    __builtin_memcpy(&x, &v, 16);
    return x;
}
FLE_MXV_AI fle_xmm fle_mxv_std(fle_v2d v) {
    fle_xmm x;
    __builtin_memcpy(&x, &v, 16);
    return x;
}
FLE_MXV_AI fle_xmm fle_mxv_sti(fle_v4i v) {
    fle_xmm x;
    __builtin_memcpy(&x, &v, 16);
    return x;
}
FLE_MXV_AI fle_xmm fle_mxv_stl(fle_v2l v) {
    fle_xmm x;
    __builtin_memcpy(&x, &v, 16);
    return x;
}

/* all lanes of a comparison mask set */
FLE_MXV_AI int fle_mxv_all4(fle_v4i m) {
    uint64_t w[2];
    __builtin_memcpy(w, &m, 16);
    return (w[0] & w[1]) == UINT64_MAX;
}
FLE_MXV_AI int fle_mxv_all2(fle_v2l m) {
    uint64_t w[2];
    __builtin_memcpy(w, &m, 16);
    return (w[0] & w[1]) == UINT64_MAX;
}

/* ---- windows (vector forms): all-ones lane = inside ---- */
FLE_MXV_AI fle_v4f fle_mxv_abs4(fle_v4f x) {
    return (fle_v4f)((fle_v4u)x & 0x7fffffffu);
}
FLE_MXV_AI fle_v2d fle_mxv_abs2(fle_v2d x) {
    return (fle_v2d)((fle_v2u)x & UINT64_C(0x7fffffffffffffff));
}
/* zero, or 2^-62 <= |x| <= 2^62 */
FLE_MXV_AI fle_v4i fle_mxv_w4(fle_v4f x) {
    fle_v4f a = fle_mxv_abs4(x);
    return ((a >= FLE_MXV_LO32) & (a <= FLE_MXV_HI32)) | (a == 0.0f);
}
/* 2^-62 <= |x| <= 2^62 (nonzero) */
FLE_MXV_AI fle_v4i fle_mxv_w4nz(fle_v4f x) {
    fle_v4f a = fle_mxv_abs4(x);
    return (a >= FLE_MXV_LO32) & (a <= FLE_MXV_HI32);
}
/* +-0, or 2^-62 <= x <= 2^62 */
FLE_MXV_AI fle_v4i fle_mxv_w4sqrt(fle_v4f x) {
    fle_v4f a = fle_mxv_abs4(x);
    return ((x >= FLE_MXV_LO32) & (x <= FLE_MXV_HI32)) | (a == 0.0f);
}
/* zero, normal or infinite: not NaN, not denormal */
FLE_MXV_AI fle_v4i fle_mxv_o4(fle_v4f x) {
    fle_v4f a = fle_mxv_abs4(x);
    return (a >= FLE_MXV_MIN32) | (a == 0.0f);
}
FLE_MXV_AI fle_v2l fle_mxv_w2(fle_v2d x) {
    fle_v2d a = fle_mxv_abs2(x);
    return ((a >= FLE_MXV_LO64) & (a <= FLE_MXV_HI64)) | (a == 0.0);
}
FLE_MXV_AI fle_v2l fle_mxv_w2nz(fle_v2d x) {
    fle_v2d a = fle_mxv_abs2(x);
    return (a >= FLE_MXV_LO64) & (a <= FLE_MXV_HI64);
}
FLE_MXV_AI fle_v2l fle_mxv_w2sqrt(fle_v2d x) {
    fle_v2d a = fle_mxv_abs2(x);
    return ((x >= FLE_MXV_LO64) & (x <= FLE_MXV_HI64)) | (a == 0.0);
}
FLE_MXV_AI fle_v2l fle_mxv_o2(fle_v2d x) {
    fle_v2d a = fle_mxv_abs2(x);
    return (a >= FLE_MXV_MIN64) | (a == 0.0);
}

/* ---- scalar windows (bits) ---- */
FLE_MXV_AI int fle_mxv_ws32(uint32_t u) {
    uint32_t t = u & 0x7fffffffu;
    return (int)(((t - (uint32_t)((127 - 62) << 23)) <= (uint32_t)(((127 + 62) - (127 - 62)) << 23)) | (t == 0));
}
FLE_MXV_AI int fle_mxv_ws32nz(uint32_t u) {
    uint32_t t = u & 0x7fffffffu;
    return (t - (uint32_t)((127 - 62) << 23)) <= (uint32_t)(((127 + 62) - (127 - 62)) << 23);
}
FLE_MXV_AI int fle_mxv_ws32sqrt(uint32_t u) {
    return (int)(((u - (uint32_t)((127 - 62) << 23)) <= (uint32_t)(((127 + 62) - (127 - 62)) << 23)) | ((u & 0x7fffffffu) == 0));
}
FLE_MXV_AI int fle_mxv_os32(uint32_t u) { /* zero, normal, infinite */
    uint32_t t = u & 0x7fffffffu;
    return (int)(((t - 0x00800000u) <= 0x7f000000u) | (t == 0));
}
FLE_MXV_AI int fle_mxv_ws64(uint64_t u) {
    uint64_t t = u & UINT64_C(0x7fffffffffffffff);
    return (int)(((t - ((uint64_t)(1023 - 510) << 52)) <= (((uint64_t)(2 * 510)) << 52)) | (t == 0));
}
FLE_MXV_AI int fle_mxv_ws64nz(uint64_t u) {
    uint64_t t = u & UINT64_C(0x7fffffffffffffff);
    return (t - ((uint64_t)(1023 - 510) << 52)) <= (((uint64_t)(2 * 510)) << 52);
}
FLE_MXV_AI int fle_mxv_ws64sqrt(uint64_t u) {
    return (int)(((u - ((uint64_t)(1023 - 510) << 52)) <= (((uint64_t)(2 * 510)) << 52)) | ((u & UINT64_C(0x7fffffffffffffff)) == 0));
}
FLE_MXV_AI int fle_mxv_os64(uint64_t u) {
    uint64_t t = u & UINT64_C(0x7fffffffffffffff);
    return (int)(((t - UINT64_C(0x0010000000000000)) <= UINT64_C(0x7fe0000000000000)) | (t == 0));
}

/* Where the target has no 128-bit vector registers (WebAssembly without -msimd128) the vector code below is lowered
 * lane by lane with float comparisons; the same windows tested on the bit patterns of each lane (one subtraction and
 * one unsigned comparison per operand) are cheaper there. */
#if defined(__wasm__) && !defined(__wasm_simd128__)
#define FLE_MXV_LANES 1
#else
#define FLE_MXV_LANES 0
#endif

#if FLE_MXV_LANES
FLE_MXV_AI int fle_mxv_ps(int op, fle_xmm d, fle_xmm s, fle_xmm *out) {
    FLE_MXV_NOCONTRACT
    float r0, r1, r2, r3;
    int ok;
    float a0 = d.f32[0], a1 = d.f32[1], a2 = d.f32[2], a3 = d.f32[3];
    float b0 = s.f32[0], b1 = s.f32[1], b2 = s.f32[2], b3 = s.f32[3];
    switch (op) {
    case FLE_MXO_ADD:
        ok = fle_mxv_ws32(d.u32[0]) & fle_mxv_ws32(d.u32[1]) & fle_mxv_ws32(d.u32[2]) & fle_mxv_ws32(d.u32[3]) &
             fle_mxv_ws32(s.u32[0]) & fle_mxv_ws32(s.u32[1]) & fle_mxv_ws32(s.u32[2]) & fle_mxv_ws32(s.u32[3]);
        r0 = a0 + b0; r1 = a1 + b1; r2 = a2 + b2; r3 = a3 + b3;
        break;
    case FLE_MXO_SUB:
        ok = fle_mxv_ws32(d.u32[0]) & fle_mxv_ws32(d.u32[1]) & fle_mxv_ws32(d.u32[2]) & fle_mxv_ws32(d.u32[3]) &
             fle_mxv_ws32(s.u32[0]) & fle_mxv_ws32(s.u32[1]) & fle_mxv_ws32(s.u32[2]) & fle_mxv_ws32(s.u32[3]);
        r0 = a0 - b0; r1 = a1 - b1; r2 = a2 - b2; r3 = a3 - b3;
        break;
    case FLE_MXO_MUL:
        ok = fle_mxv_ws32(d.u32[0]) & fle_mxv_ws32(d.u32[1]) & fle_mxv_ws32(d.u32[2]) & fle_mxv_ws32(d.u32[3]) &
             fle_mxv_ws32(s.u32[0]) & fle_mxv_ws32(s.u32[1]) & fle_mxv_ws32(s.u32[2]) & fle_mxv_ws32(s.u32[3]);
        r0 = a0 * b0; r1 = a1 * b1; r2 = a2 * b2; r3 = a3 * b3;
        break;
    case FLE_MXO_DIV:
        ok = fle_mxv_ws32(d.u32[0]) & fle_mxv_ws32(d.u32[1]) & fle_mxv_ws32(d.u32[2]) & fle_mxv_ws32(d.u32[3]) &
             fle_mxv_ws32nz(s.u32[0]) & fle_mxv_ws32nz(s.u32[1]) & fle_mxv_ws32nz(s.u32[2]) & fle_mxv_ws32nz(s.u32[3]);
        r0 = a0 / b0; r1 = a1 / b1; r2 = a2 / b2; r3 = a3 / b3;
        break;
    case FLE_MXO_MIN:
        ok = fle_mxv_os32(d.u32[0]) & fle_mxv_os32(d.u32[1]) & fle_mxv_os32(d.u32[2]) & fle_mxv_os32(d.u32[3]) &
             fle_mxv_os32(s.u32[0]) & fle_mxv_os32(s.u32[1]) & fle_mxv_os32(s.u32[2]) & fle_mxv_os32(s.u32[3]);
        r0 = a0 < b0 ? a0 : b0; r1 = a1 < b1 ? a1 : b1; r2 = a2 < b2 ? a2 : b2; r3 = a3 < b3 ? a3 : b3;
        break;
    case FLE_MXO_MAX:
        ok = fle_mxv_os32(d.u32[0]) & fle_mxv_os32(d.u32[1]) & fle_mxv_os32(d.u32[2]) & fle_mxv_os32(d.u32[3]) &
             fle_mxv_os32(s.u32[0]) & fle_mxv_os32(s.u32[1]) & fle_mxv_os32(s.u32[2]) & fle_mxv_os32(s.u32[3]);
        r0 = a0 > b0 ? a0 : b0; r1 = a1 > b1 ? a1 : b1; r2 = a2 > b2 ? a2 : b2; r3 = a3 > b3 ? a3 : b3;
        break;
    default:
        ok = fle_mxv_ws32sqrt(s.u32[0]) & fle_mxv_ws32sqrt(s.u32[1]) & fle_mxv_ws32sqrt(s.u32[2]) & fle_mxv_ws32sqrt(s.u32[3]);
        r0 = (float)FLE_MXF_SQRT(b0); r1 = (float)FLE_MXF_SQRT(b1); r2 = (float)FLE_MXF_SQRT(b2); r3 = (float)FLE_MXF_SQRT(b3);
        break;
    }
    if (!ok) {
        return 0;
    }
    d.f32[0] = r0; d.f32[1] = r1; d.f32[2] = r2; d.f32[3] = r3;
    *out = d;
    return 1;
}

FLE_MXV_AI int fle_mxv_pd(int op, fle_xmm d, fle_xmm s, fle_xmm *out) {
    FLE_MXV_NOCONTRACT
    double r0, r1;
    int ok;
    double a0 = d.f64[0], a1 = d.f64[1], b0 = s.f64[0], b1 = s.f64[1];
    switch (op) {
    case FLE_MXO_ADD: ok = fle_mxv_ws64(d.u64[0]) & fle_mxv_ws64(d.u64[1]) & fle_mxv_ws64(s.u64[0]) & fle_mxv_ws64(s.u64[1]); r0 = a0 + b0; r1 = a1 + b1; break;
    case FLE_MXO_SUB: ok = fle_mxv_ws64(d.u64[0]) & fle_mxv_ws64(d.u64[1]) & fle_mxv_ws64(s.u64[0]) & fle_mxv_ws64(s.u64[1]); r0 = a0 - b0; r1 = a1 - b1; break;
    case FLE_MXO_MUL: ok = fle_mxv_ws64(d.u64[0]) & fle_mxv_ws64(d.u64[1]) & fle_mxv_ws64(s.u64[0]) & fle_mxv_ws64(s.u64[1]); r0 = a0 * b0; r1 = a1 * b1; break;
    case FLE_MXO_DIV: ok = fle_mxv_ws64(d.u64[0]) & fle_mxv_ws64(d.u64[1]) & fle_mxv_ws64nz(s.u64[0]) & fle_mxv_ws64nz(s.u64[1]); r0 = a0 / b0; r1 = a1 / b1; break;
    case FLE_MXO_MIN: ok = fle_mxv_os64(d.u64[0]) & fle_mxv_os64(d.u64[1]) & fle_mxv_os64(s.u64[0]) & fle_mxv_os64(s.u64[1]); r0 = a0 < b0 ? a0 : b0; r1 = a1 < b1 ? a1 : b1; break;
    case FLE_MXO_MAX: ok = fle_mxv_os64(d.u64[0]) & fle_mxv_os64(d.u64[1]) & fle_mxv_os64(s.u64[0]) & fle_mxv_os64(s.u64[1]); r0 = a0 > b0 ? a0 : b0; r1 = a1 > b1 ? a1 : b1; break;
    default: ok = fle_mxv_ws64sqrt(s.u64[0]) & fle_mxv_ws64sqrt(s.u64[1]); r0 = FLE_MXF_SQRT(b0); r1 = FLE_MXF_SQRT(b1); break;
    }
    if (!ok) {
        return 0;
    }
    d.f64[0] = r0; d.f64[1] = r1;
    *out = d;
    return 1;
}
#else

/* ---- arithmetic, packed single (4 lanes) ---- */
FLE_MXV_AI int fle_mxv_ps(int op, fle_xmm d, fle_xmm s, fle_xmm *out) {
    FLE_MXV_NOCONTRACT
    fle_v4f a = fle_mxv_ldf(&d), b = fle_mxv_ldf(&s), r;
    fle_v4i m;
    switch (op) {
    case FLE_MXO_ADD: m = fle_mxv_w4(a) & fle_mxv_w4(b); r = a + b; break;
    case FLE_MXO_SUB: m = fle_mxv_w4(a) & fle_mxv_w4(b); r = a - b; break;
    case FLE_MXO_MUL: m = fle_mxv_w4(a) & fle_mxv_w4(b); r = a * b; break;
    case FLE_MXO_DIV: m = fle_mxv_w4(a) & fle_mxv_w4nz(b); r = a / b; break;
    case FLE_MXO_MIN:
    case FLE_MXO_MAX: {
        fle_v4i c = (op == FLE_MXO_MAX) ? (a > b) : (a < b);
        fle_v4i ia = (fle_v4i)a, ib = (fle_v4i)b;
        m = fle_mxv_o4(a) & fle_mxv_o4(b);
        r = (fle_v4f)((ia & c) | (ib & ~c));
        break;
    }
    default:
#ifdef FLE_MXV_SQRT
        m = fle_mxv_w4sqrt(b);
        r = __builtin_elementwise_sqrt(b);
        break;
#else
        return 0;
#endif
    }
    if (!fle_mxv_all4(m)) {
        return 0;
    }
    *out = fle_mxv_stf(r);
    return 1;
}

/* ---- arithmetic, packed double (2 lanes) ---- */
FLE_MXV_AI int fle_mxv_pd(int op, fle_xmm d, fle_xmm s, fle_xmm *out) {
    FLE_MXV_NOCONTRACT
    fle_v2d a = fle_mxv_ldd(&d), b = fle_mxv_ldd(&s), r;
    fle_v2l m;
    switch (op) {
    case FLE_MXO_ADD: m = fle_mxv_w2(a) & fle_mxv_w2(b); r = a + b; break;
    case FLE_MXO_SUB: m = fle_mxv_w2(a) & fle_mxv_w2(b); r = a - b; break;
    case FLE_MXO_MUL: m = fle_mxv_w2(a) & fle_mxv_w2(b); r = a * b; break;
    case FLE_MXO_DIV: m = fle_mxv_w2(a) & fle_mxv_w2nz(b); r = a / b; break;
    case FLE_MXO_MIN:
    case FLE_MXO_MAX: {
        fle_v2l c = (op == FLE_MXO_MAX) ? (a > b) : (a < b);
        fle_v2l ia = (fle_v2l)a, ib = (fle_v2l)b;
        m = fle_mxv_o2(a) & fle_mxv_o2(b);
        r = (fle_v2d)((ia & c) | (ib & ~c));
        break;
    }
    default:
#ifdef FLE_MXV_SQRT
        m = fle_mxv_w2sqrt(b);
        r = __builtin_elementwise_sqrt(b);
        break;
#else
        return 0;
#endif
    }
    if (!fle_mxv_all2(m)) {
        return 0;
    }
    *out = fle_mxv_std(r);
    return 1;
}

#endif /* FLE_MXV_LANES */

/* ---- arithmetic, scalar single and double: lane 0 only, the other lanes of d are kept ---- */
FLE_MXV_AI int fle_mxv_ss(int op, fle_xmm d, fle_xmm s, fle_xmm *out) {
    FLE_MXV_NOCONTRACT
    float a = d.f32[0], b = s.f32[0], r;
    int ok;
    switch (op) {
    case FLE_MXO_ADD: ok = fle_mxv_ws32(d.u32[0]) & fle_mxv_ws32(s.u32[0]); r = a + b; break;
    case FLE_MXO_SUB: ok = fle_mxv_ws32(d.u32[0]) & fle_mxv_ws32(s.u32[0]); r = a - b; break;
    case FLE_MXO_MUL: ok = fle_mxv_ws32(d.u32[0]) & fle_mxv_ws32(s.u32[0]); r = a * b; break;
    case FLE_MXO_DIV: ok = fle_mxv_ws32(d.u32[0]) & fle_mxv_ws32nz(s.u32[0]); r = a / b; break;
    case FLE_MXO_MIN: ok = fle_mxv_os32(d.u32[0]) & fle_mxv_os32(s.u32[0]); r = a < b ? a : b; break;
    case FLE_MXO_MAX: ok = fle_mxv_os32(d.u32[0]) & fle_mxv_os32(s.u32[0]); r = a > b ? a : b; break;
    default: ok = fle_mxv_ws32sqrt(s.u32[0]); r = (float)FLE_MXF_SQRT(b); break;
    }
    if (!ok) {
        return 0;
    }
    d.f32[0] = r;
    *out = d;
    return 1;
}

FLE_MXV_AI int fle_mxv_sd(int op, fle_xmm d, fle_xmm s, fle_xmm *out) {
    FLE_MXV_NOCONTRACT
    double a = d.f64[0], b = s.f64[0], r;
    int ok;
    switch (op) {
    case FLE_MXO_ADD: ok = fle_mxv_ws64(d.u64[0]) & fle_mxv_ws64(s.u64[0]); r = a + b; break;
    case FLE_MXO_SUB: ok = fle_mxv_ws64(d.u64[0]) & fle_mxv_ws64(s.u64[0]); r = a - b; break;
    case FLE_MXO_MUL: ok = fle_mxv_ws64(d.u64[0]) & fle_mxv_ws64(s.u64[0]); r = a * b; break;
    case FLE_MXO_DIV: ok = fle_mxv_ws64(d.u64[0]) & fle_mxv_ws64nz(s.u64[0]); r = a / b; break;
    case FLE_MXO_MIN: ok = fle_mxv_os64(d.u64[0]) & fle_mxv_os64(s.u64[0]); r = a < b ? a : b; break;
    case FLE_MXO_MAX: ok = fle_mxv_os64(d.u64[0]) & fle_mxv_os64(s.u64[0]); r = a > b ? a : b; break;
    default: ok = fle_mxv_ws64sqrt(s.u64[0]); r = FLE_MXF_SQRT(b); break;
    }
    if (!ok) {
        return 0;
    }
    d.f64[0] = r;
    *out = d;
    return 1;
}

/* shared by the conversion wrappers (fle_mxcsr_cvt.h) and the directed-rounding code (fle_mxcsr_rc.h); moved here from fle_mxcsr_cvt.h (decision 0172) */
/* statistics ids of the vector counters (fle_mx_vstat): 0..31 arithmetic (op + 8 * f64 + 16 * packed), 32..39 CMP
 * (32 + 4 * f64 + 2 * scalar), then these */
enum {
    FLE_MXV_CVTSS2SD = 40, FLE_MXV_CVTSD2SS, FLE_MXV_CVTPS2PD, FLE_MXV_CVTPD2PS, FLE_MXV_CVTDQ2PS, FLE_MXV_CVTDQ2PD,
    FLE_MXV_CVTPS2DQ, FLE_MXV_CVTTPS2DQ, FLE_MXV_CVTPD2DQ, FLE_MXV_CVTTPD2DQ, FLE_MXV_CVTSI2SS32, FLE_MXV_CVTSI2SS64,
    FLE_MXV_CVTSI2SD32, FLE_MXV_CVTSI2SD64, FLE_MXV_CVTSS2SI32, FLE_MXV_CVTSS2SI64, FLE_MXV_CVTSD2SI32, FLE_MXV_CVTSD2SI64,
    FLE_MXV_CVTTSS2SI32, FLE_MXV_CVTTSS2SI64, FLE_MXV_CVTTSD2SI32, FLE_MXV_CVTTSD2SI64
};

/* scalar window of a binary64 for the conversions to binary32 and to integers */
FLE_MXV_AI int fle_mxv_cw64(uint64_t u, uint32_t elo, uint32_t ehi) { /* zero, or biased exponent in [elo, ehi] */
    uint64_t t = u & UINT64_C(0x7fffffffffffffff);
    return (int)(((uint32_t)(t >> 52) - elo <= ehi - elo) | (t == 0));
}
FLE_MXV_AI int fle_mxv_cw32(uint32_t u, uint32_t elo, uint32_t ehi) {
    uint32_t t = u & 0x7fffffffu;
    return (int)(((t >> 23) - elo <= ehi - elo) | (t == 0));
}

/* ---- float to integer, scalar. x is the value as a double (a binary32 converts to double exactly). ---- */
FLE_MXV_AI int fle_mxv_f2i_trunc_ok(double x, int out64) { /* |x| < 2^31 (2^51 for 64 bits) */
    return out64 ? (x > -2251799813685248.0 && x < 2251799813685248.0) : (x > -2147483648.0 && x < 2147483648.0);
}

FLE_MXV_AI int fle_mxv_f2i_rne_ok(double x, int out64) {
    return out64 ? (x > -2251799813685248.0 && x < 2251799813685248.0) : (x > -2147483647.0 && x < 2147483647.0);
}

FLE_MXV_AI double fle_mxv_rne(double x) { /* |x| < 2^51 */
    double a = x < 0 ? -x : x;
    double t = (a + 4503599627370496.0) - 4503599627370496.0;
    return x < 0 ? -t : t;
}

/* directed rounding (RC != 0) with the same guard on PE and PM, decision 0172 */
#include "fle_mxcsr_rc.h"

/* One instruction: the vector/scalar fast path when the guard and the windows allow, else the code of decision 0143. */
FLE_MX_SLOW fle_xmm fle_mx_do2_slow(int op, int f64, int scalar, fle_xmm d, fle_xmm s);
FLE_MX_SLOW void fle_mx_do2_slow_p(int id, const fle_xmm *d, const fle_xmm *s, fle_xmm *r);
FLE_MX_SLOW void fle_mx_do2_slow_id_h(int id, const fle_xmm *d, const fle_xmm *s, fle_xmm *r);

FLE_MXV_AI fle_xmm fle_mxv_op2(int op, int f64, int scalar, fle_xmm d, fle_xmm s) {
    uint32_t mx = fle_mxcsr;
    int id = op + 8 * f64 + 16 * !scalar;
    fle_xmm r;
    if (((op == FLE_MXO_MIN || op == FLE_MXO_MAX) && FLE_MXV_CTL_OK(mx)) || ((op != FLE_MXO_MIN && op != FLE_MXO_MAX) && FLE_MXV_NP0(mx))) {
        if (scalar ? (f64 ? fle_mxv_sd(op, d, s, &r) : fle_mxv_ss(op, d, s, &r)) : (f64 ? fle_mxv_pd(op, d, s, &r) : fle_mxv_ps(op, d, s, &r))) {
            FLE_MXVS(0, id);
            return r;
        }
        FLE_MXVS(2, id);
    } else {
        FLE_MXVS(1, id);
    }
#if FLE_MXV_SLOWBYVAL
    return fle_mx_do2_slow(op, f64, scalar, d, s);
#else
    fle_mx_do2_slow_p(id, &d, &s, &r);
    return r;
#endif
}

/* ---- CMPSS CMPSD CMPPS CMPPD: no flag for operands that are not NaN and not denormal; the result is a mask ---- */
FLE_MXV_AI int fle_mxv_cmp4(fle_xmm d, fle_xmm s, unsigned imm, int scalar, fle_xmm *out) {
    FLE_MXV_NOCONTRACT
    fle_v4f a = fle_mxv_ldf(&d), b = fle_mxv_ldf(&s);
    fle_v4i m, c;
    if (scalar ? !(fle_mxv_os32(d.u32[0]) & fle_mxv_os32(s.u32[0])) : !fle_mxv_all4(fle_mxv_o4(a) & fle_mxv_o4(b))) {
        return 0;
    }
    switch (imm & 7u) {
    case 0: c = (a == b); break;
    case 1: c = (a < b); break;
    case 2: c = (a <= b); break;
    case 3: c = (fle_v4i){0, 0, 0, 0}; break;
    case 4: c = (a != b); break;
    case 5: c = (a >= b); break;
    case 6: c = (a > b); break;
    default: c = (fle_v4i){-1, -1, -1, -1}; break;
    }
    if (scalar) {
        d.u32[0] = (uint32_t)c[0];
        *out = d;
    } else {
        m = c;
        *out = fle_mxv_sti(m);
    }
    return 1;
}

FLE_MXV_AI int fle_mxv_cmp2(fle_xmm d, fle_xmm s, unsigned imm, int scalar, fle_xmm *out) {
    FLE_MXV_NOCONTRACT
    fle_v2d a = fle_mxv_ldd(&d), b = fle_mxv_ldd(&s);
    fle_v2l c;
    if (scalar ? !(fle_mxv_os64(d.u64[0]) & fle_mxv_os64(s.u64[0])) : !fle_mxv_all2(fle_mxv_o2(a) & fle_mxv_o2(b))) {
        return 0;
    }
    switch (imm & 7u) {
    case 0: c = (a == b); break;
    case 1: c = (a < b); break;
    case 2: c = (a <= b); break;
    case 3: c = (fle_v2l){0, 0}; break;
    case 4: c = (a != b); break;
    case 5: c = (a >= b); break;
    case 6: c = (a > b); break;
    default: c = (fle_v2l){-1, -1}; break;
    }
    if (scalar) {
        d.u64[0] = (uint64_t)c[0];
        *out = d;
    } else {
        *out = fle_mxv_stl(c);
    }
    return 1;
}

#endif /* FLE_MX_VEC */
#endif /* FLE_MXCSR_VEC_H */
