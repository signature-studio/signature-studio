/* Conversion wrappers of generated C (decision 0154). Included at the end of fle_mxcsr_flags.h. fle_mxcsr_vec.h explains the
 * guard and the windows. Each wrapper tries the guarded fast path and otherwise calls the code of decision 0143 (the
 * *_slow functions defined in fle_mxcsr_flags.h), so a build without FLE_MX_VEC is the old code behind the same names.
 *
 * Exact conversions need no guard at all (no flag, no rounding, the result does not depend on MXCSR):
 *   CVTSI2SD from int32, CVTDQ2PD, CVTSS2SD and CVTPS2PD of a zero, a normal number or an infinity, CVTSI2SS and CVTDQ2PS
 *   of |i| <= 2^24, CVTSI2SD from int64 of |i| <= 2^53.
 * The others can be inexact: they need RC = 0 and PE set with PM masked (FLE_MXV_NP0), as the arithmetic does:
 *   CVTSD2SS and CVTPD2PS of zero or 2^-100 <= |x| <= 2^100 (the result is a binary32 normal number);
 *   CVTTSD2SI CVTTSS2SI CVTTPS2DQ CVTTPD2DQ (truncation) of zero or 2^-62 <= |x| < 2^31 (binary64: 2^-510 <= |x|; a 64-bit
 *   result: |x| < 2^51; the packed binary32 form: |x| <= 2^30);
 *   CVTSD2SI CVTSS2SI CVTPS2DQ CVTPD2DQ (round to nearest even) of zero or 2^-62 <= |x| <= 2^22 for the packed binary32 forms
 *   (rounded by x + 1.5 * 2^23 - 1.5 * 2^23), and |x| < 2^31 - 1 / 2^51 for the scalar forms (rounded by adding and
 *   subtracting 2^52 to |x|, as the code of decision 0143 does).
 */
#ifndef FLE_MXCSR_CVT_H
#define FLE_MXCSR_CVT_H

#ifdef FLE_MX_VEC

/* ---- float to float ---- */
FLE_MXV_WAI fle_xmm fle_mx_cvtss2sd(fle_xmm d, fle_xmm s) {
    if (FLE_MXV_CTL_OK(fle_mxcsr) && fle_mxv_os32(s.u32[0])) {
        FLE_MXVS(0, FLE_MXV_CVTSS2SD);
        d.f64[0] = (double)s.f32[0];
        return d;
    }
    FLE_MXVS(2, FLE_MXV_CVTSS2SD);
    return fle_mx_cvtss2sd_slow(d, s);
}

FLE_MXV_WAI fle_xmm fle_mx_cvtsd2ss(fle_xmm d, fle_xmm s) {
    if (FLE_MXV_NP0(fle_mxcsr) && fle_mxv_cw64(s.u64[0], 1023 - 100, 1023 + 100)) {
        FLE_MXVS(0, FLE_MXV_CVTSD2SS);
        d.f32[0] = (float)s.f64[0];
        return d;
    }
    FLE_MXVS(2, FLE_MXV_CVTSD2SS);
    return fle_mx_cvtsd2ss_slow(d, s);
}

FLE_MXV_WAI fle_xmm fle_mx_cvtps2pd(fle_xmm d, fle_xmm s) {
    if (FLE_MXV_CTL_OK(fle_mxcsr) && (fle_mxv_os32(s.u32[0]) & fle_mxv_os32(s.u32[1]))) {
        FLE_MXVS(0, FLE_MXV_CVTPS2PD);
        d.f64[0] = (double)s.f32[0];
        d.f64[1] = (double)s.f32[1];
        return d;
    }
    FLE_MXVS(2, FLE_MXV_CVTPS2PD);
    return fle_mx_cvtps2pd_slow(d, s);
}

FLE_MXV_WAI fle_xmm fle_mx_cvtpd2ps(fle_xmm d, fle_xmm s) {
    if (FLE_MXV_NP0(fle_mxcsr) && (fle_mxv_cw64(s.u64[0], 1023 - 100, 1023 + 100) & fle_mxv_cw64(s.u64[1], 1023 - 100, 1023 + 100))) {
        FLE_MXVS(0, FLE_MXV_CVTPD2PS);
        d.f32[0] = (float)s.f64[0];
        d.f32[1] = (float)s.f64[1];
        d.u64[1] = 0;
        return d;
    }
    FLE_MXVS(2, FLE_MXV_CVTPD2PS);
    return fle_mx_cvtpd2ps_slow(d, s);
}

/* ---- integer to float ---- */
FLE_MXV_WAI fle_xmm fle_mx_cvtdq2pd(fle_xmm d, fle_xmm s) {
    if (!FLE_MXV_CTL_OK(fle_mxcsr)) {
        return fle_mx_cvtdq2pd_slow(d, s);
    }
    FLE_MXVS(0, FLE_MXV_CVTDQ2PD);
    d.f64[0] = (double)(int32_t)s.u32[0];
    d.f64[1] = (double)(int32_t)s.u32[1];
    return d;
}

FLE_MXV_WAI fle_xmm fle_mx_cvtdq2ps(fle_xmm d, fle_xmm s) {
    uint32_t k = 0x01000000u; /* |i| <= 2^24: i + 2^24 <= 2^25 as an unsigned number */
    if (FLE_MXV_NP0(fle_mxcsr) || (FLE_MXV_CTL_OK(fle_mxcsr) && ((s.u32[0] + k) <= 2 * k) & ((s.u32[1] + k) <= 2 * k) & ((s.u32[2] + k) <= 2 * k) & ((s.u32[3] + k) <= 2 * k))) {
        d.f32[0] = (float)(int32_t)s.u32[0];
        d.f32[1] = (float)(int32_t)s.u32[1];
        d.f32[2] = (float)(int32_t)s.u32[2];
        d.f32[3] = (float)(int32_t)s.u32[3];
        FLE_MXVS(0, FLE_MXV_CVTDQ2PS);
        return d;
    }
    FLE_MXVS(2, FLE_MXV_CVTDQ2PS);
    return fle_mx_cvtdq2ps_slow(d, s);
}

FLE_MXV_WAI fle_xmm fle_mx_cvtsi2sd32(fle_xmm d, int32_t v) {
    if (!FLE_MXV_CTL_OK(fle_mxcsr)) {
        return fle_mx_cvtsi2sd32_slow(d, v);
    }
    FLE_MXVS(0, FLE_MXV_CVTSI2SD32);
    d.f64[0] = (double)v;
    return d;
}

FLE_MXV_WAI fle_xmm fle_mx_cvtsi2sd64(fle_xmm d, int64_t v) {
    uint64_t k = UINT64_C(1) << 53;
    if (FLE_MXV_NP0(fle_mxcsr) || (FLE_MXV_CTL_OK(fle_mxcsr) && (uint64_t)v + k <= 2 * k)) {
        d.f64[0] = (double)v;
        FLE_MXVS(0, FLE_MXV_CVTSI2SD64);
        return d;
    }
    FLE_MXVS(2, FLE_MXV_CVTSI2SD64);
    return fle_mx_cvtsi2sd64_slow(d, v);
}

FLE_MXV_WAI fle_xmm fle_mx_cvtsi2ss32(fle_xmm d, int32_t v) {
    uint32_t k = 0x01000000u;
    if (FLE_MXV_NP0(fle_mxcsr) || (FLE_MXV_CTL_OK(fle_mxcsr) && (uint32_t)v + k <= 2 * k)) {
        d.f32[0] = (float)v;
        FLE_MXVS(0, FLE_MXV_CVTSI2SS32);
        return d;
    }
    FLE_MXVS(2, FLE_MXV_CVTSI2SS32);
    return fle_mx_cvtsi2ss32_slow(d, v);
}

FLE_MXV_WAI fle_xmm fle_mx_cvtsi2ss64(fle_xmm d, int64_t v) {
    uint64_t k = 0x01000000u;
    if (FLE_MXV_NP0(fle_mxcsr) || (FLE_MXV_CTL_OK(fle_mxcsr) && (uint64_t)v + k <= 2 * k)) {
        d.f32[0] = (float)v;
        FLE_MXVS(0, FLE_MXV_CVTSI2SS64);
        return d;
    }
    FLE_MXVS(2, FLE_MXV_CVTSI2SS64);
    return fle_mx_cvtsi2ss64_slow(d, v);
}

/* ---- float to integer, scalar. x is the value as a double (a binary32 converts to double exactly). ---- */
#define FLE_MXV_CVT2SI(name, isd, out64, trunc, vid) \
    FLE_MXV_WAI uint64_t name(fle_xmm s) { \
        FLE_MXV_NOCONTRACT \
        if (((trunc) ? FLE_MXV_NP0X(fle_mxcsr) : FLE_MXV_NP0(fle_mxcsr)) && \
            ((isd) ? fle_mxv_cw64(s.u64[0], (trunc) ? FLE_MXV_TLO64 : 1023 - 510, 2046) : fle_mxv_cw32(s.u32[0], (trunc) ? FLE_MXV_TLO32 : 127 - 62, 254))) { \
            double x = (isd) ? s.f64[0] : (double)s.f32[0]; \
            if ((trunc) ? fle_mxv_f2i_trunc_ok(x, out64) : fle_mxv_f2i_rne_ok(x, out64)) { \
                int64_t i = (trunc) ? (int64_t)x : (int64_t)fle_mxv_rne(x); \
                FLE_MXVS(0, vid); \
                return (out64) ? (uint64_t)i : (uint64_t)(uint32_t)(int32_t)i; \
            } \
        } \
        FLE_MXVS(2, vid); \
        return name##_slow(s); \
    }

FLE_MXV_CVT2SI(fle_mx_cvtss2si32, 0, 0, 0, FLE_MXV_CVTSS2SI32)
FLE_MXV_CVT2SI(fle_mx_cvtss2si64, 0, 1, 0, FLE_MXV_CVTSS2SI64)
FLE_MXV_CVT2SI(fle_mx_cvtsd2si32, 1, 0, 0, FLE_MXV_CVTSD2SI32)
FLE_MXV_CVT2SI(fle_mx_cvtsd2si64, 1, 1, 0, FLE_MXV_CVTSD2SI64)
FLE_MXV_CVT2SI(fle_mx_cvttss2si32, 0, 0, 1, FLE_MXV_CVTTSS2SI32)
FLE_MXV_CVT2SI(fle_mx_cvttss2si64, 0, 1, 1, FLE_MXV_CVTTSS2SI64)
FLE_MXV_CVT2SI(fle_mx_cvttsd2si32, 1, 0, 1, FLE_MXV_CVTTSD2SI32)
FLE_MXV_CVT2SI(fle_mx_cvttsd2si64, 1, 1, 1, FLE_MXV_CVTTSD2SI64)

/* ---- float to integer, packed ---- */
FLE_MXV_AI fle_xmm fle_mxv_cvtps2dq_k(fle_xmm d, fle_xmm s, int trunc, int *ok) {
    FLE_MXV_NOCONTRACT
    int32_t r[4];
    float x[4];
    int k;
    *ok = 0;
    if (!(trunc ? FLE_MXV_NP0X(fle_mxcsr) : FLE_MXV_NP0(fle_mxcsr))) {
        return d;
    }
    for (k = 0; k < 4; k++) {
        uint32_t lo = trunc ? FLE_MXV_TLO32 : 127 - 62;
        x[k] = s.f32[k];
        uint32_t t = s.u32[k] & 0x7fffffffu;
        /* zero, or 2^-62 <= |x| <= 2^22 (rounding); truncation: any normal number up to 2^30 (decision 0172) */
        if (!(t == 0 || (t - (lo << 23) <= (uint32_t)(((127 + (trunc ? 30 : 22)) - lo) << 23)))) {
            return d;
        }
    }
    if (trunc) {
        for (k = 0; k < 4; k++) {
            r[k] = (int32_t)x[k];
        }
    } else {
        for (k = 0; k < 4; k++) {
            r[k] = (int32_t)((x[k] + 12582912.0f) - 12582912.0f);
        }
    }
    for (k = 0; k < 4; k++) {
        d.u32[k] = (uint32_t)r[k];
    }
    *ok = 1;
    return d;
}

FLE_MXV_WAI fle_xmm fle_mx_cvtps2dq(fle_xmm d, fle_xmm s) {
    int ok;
    fle_xmm r = fle_mxv_cvtps2dq_k(d, s, 0, &ok);
    FLE_MXVS(ok ? 0 : 2, FLE_MXV_CVTPS2DQ);
    return ok ? r : fle_mx_cvtps2dq_slow(d, s);
}

FLE_MXV_WAI fle_xmm fle_mx_cvttps2dq(fle_xmm d, fle_xmm s) {
    int ok;
    fle_xmm r = fle_mxv_cvtps2dq_k(d, s, 1, &ok);
    FLE_MXVS(ok ? 0 : 2, FLE_MXV_CVTTPS2DQ);
    return ok ? r : fle_mx_cvttps2dq_slow(d, s);
}

FLE_MXV_AI fle_xmm fle_mxv_cvtpd2dq_k(fle_xmm d, fle_xmm s, int trunc, int *ok) {
    FLE_MXV_NOCONTRACT
    *ok = 0;
    if ((trunc ? FLE_MXV_NP0X(fle_mxcsr) : FLE_MXV_NP0(fle_mxcsr)) && fle_mxv_cw64(s.u64[0], trunc ? FLE_MXV_TLO64 : 1023 - 510, 2046) &&
        fle_mxv_cw64(s.u64[1], trunc ? FLE_MXV_TLO64 : 1023 - 510, 2046)) {
        double x0 = s.f64[0], x1 = s.f64[1];
        if (trunc ? (fle_mxv_f2i_trunc_ok(x0, 0) & fle_mxv_f2i_trunc_ok(x1, 0)) : (fle_mxv_f2i_rne_ok(x0, 0) & fle_mxv_f2i_rne_ok(x1, 0))) {
            int64_t i0 = trunc ? (int64_t)x0 : (int64_t)fle_mxv_rne(x0), i1 = trunc ? (int64_t)x1 : (int64_t)fle_mxv_rne(x1);
            d.u32[0] = (uint32_t)(int32_t)i0;
            d.u32[1] = (uint32_t)(int32_t)i1;
            d.u64[1] = 0;
            *ok = 1;
        }
    }
    return d;
}

FLE_MXV_WAI fle_xmm fle_mx_cvtpd2dq(fle_xmm d, fle_xmm s) {
    int ok;
    fle_xmm r = fle_mxv_cvtpd2dq_k(d, s, 0, &ok);
    FLE_MXVS(ok ? 0 : 2, FLE_MXV_CVTPD2DQ);
    return ok ? r : fle_mx_cvtpd2dq_slow(d, s);
}

FLE_MXV_WAI fle_xmm fle_mx_cvttpd2dq(fle_xmm d, fle_xmm s) {
    int ok;
    fle_xmm r = fle_mxv_cvtpd2dq_k(d, s, 1, &ok);
    FLE_MXVS(ok ? 0 : 2, FLE_MXV_CVTTPD2DQ);
    return ok ? r : fle_mx_cvttpd2dq_slow(d, s);
}

#else /* !FLE_MX_VEC: the old code behind the same names */

#define FLE_MXC_FWD2(n) static inline fle_xmm fle_mx_##n(fle_xmm d, fle_xmm s) { return fle_mx_##n##_slow(d, s); }
FLE_MXC_FWD2(cvtss2sd)
FLE_MXC_FWD2(cvtsd2ss)
FLE_MXC_FWD2(cvtps2pd)
FLE_MXC_FWD2(cvtpd2ps)
FLE_MXC_FWD2(cvtdq2ps)
FLE_MXC_FWD2(cvtdq2pd)
FLE_MXC_FWD2(cvtps2dq)
FLE_MXC_FWD2(cvttps2dq)
FLE_MXC_FWD2(cvtpd2dq)
FLE_MXC_FWD2(cvttpd2dq)
static inline fle_xmm fle_mx_cvtsi2ss32(fle_xmm d, int32_t v) { return fle_mx_cvtsi2ss32_slow(d, v); }
static inline fle_xmm fle_mx_cvtsi2ss64(fle_xmm d, int64_t v) { return fle_mx_cvtsi2ss64_slow(d, v); }
static inline fle_xmm fle_mx_cvtsi2sd32(fle_xmm d, int32_t v) { return fle_mx_cvtsi2sd32_slow(d, v); }
static inline fle_xmm fle_mx_cvtsi2sd64(fle_xmm d, int64_t v) { return fle_mx_cvtsi2sd64_slow(d, v); }
#define FLE_MXC_FWD1(n) static inline uint64_t fle_mx_##n(fle_xmm s) { return fle_mx_##n##_slow(s); }
FLE_MXC_FWD1(cvtss2si32)
FLE_MXC_FWD1(cvtss2si64)
FLE_MXC_FWD1(cvtsd2si32)
FLE_MXC_FWD1(cvtsd2si64)
FLE_MXC_FWD1(cvttss2si32)
FLE_MXC_FWD1(cvttss2si64)
FLE_MXC_FWD1(cvttsd2si32)
FLE_MXC_FWD1(cvttsd2si64)

#endif /* FLE_MX_VEC */
#endif /* FLE_MXCSR_CVT_H */
