/* Operation table of the fast-path differential test (decision 0143), included by each implementation file
 * (impl_new.c, impl_nofast.c, impl_old.c, impl_host.c) after it has defined
 *   PFX  the prefix of the functions   (fle_mx_ or fle_hx_)
 *   MPFX the prefix of the macros      (FLE_MX_ or FLE_HX_)
 *   TAG  the suffix of the exported table (new, nofast, old, host)
 * and included its runtime header. Every wrapper has the signature void f(xmm_t *d, const xmm_t *s, uint64_t *g):
 * d is the destination XMM (in/out), s the source, g a general register (source of CVTSI2S*, destination of
 * CVT[T]S*2SI). The list MXFP_OPS names the operations in the same order for every implementation. */
#ifndef MXFP_OPS_H
#define MXFP_OPS_H

#define MXFP_CAT_(a, b) a##b
#define MXFP_CAT(a, b) MXFP_CAT_(a, b)
#define FN(nm) MXFP_CAT(PFX, nm)
#define MC(nm) MXFP_CAT(MPFX, nm)

#define W_OP2(nm, sk, sid, lanes) \
    static void f_##nm(xmm_t *d, const xmm_t *s, uint64_t *g) { \
        fle_xmm dd, ss; (void)g; memcpy(&dd, d, 16); memcpy(&ss, s, 16); dd = FN(nm)(dd, ss); memcpy(d, &dd, 16); }
#define W_IMM(nm, MAC, imm) \
    static void f_##nm##_##imm(xmm_t *d, const xmm_t *s, uint64_t *g) { \
        fle_xmm dd, ss; (void)g; memcpy(&dd, d, 16); memcpy(&ss, s, 16); dd = MC(MAC)(dd, ss, imm); memcpy(d, &dd, 16); }
#define W_SI2(nm, bits, T) \
    static void f_##nm##bits(xmm_t *d, const xmm_t *s, uint64_t *g) { \
        fle_xmm dd; (void)s; memcpy(&dd, d, 16); dd = FN(nm##bits)(dd, (T)*g); memcpy(d, &dd, 16); }
#define W_2SI(nm, bits) \
    static void f_##nm##bits(xmm_t *d, const xmm_t *s, uint64_t *g) { \
        fle_xmm ss; (void)d; memcpy(&ss, s, 16); *g = FN(nm##bits)(ss); }
#define W_COMI(nm) \
    static void f_##nm(xmm_t *d, const xmm_t *s, uint64_t *g) { \
        fle_xmm dd, ss; (void)g; memcpy(&dd, d, 16); memcpy(&ss, s, 16); FN(nm)(dd, ss); }

/* name, source kind (0 f32 lanes, 1 f64 lanes, 2 i32 lanes, 3 GPR source), stat id of the lane class, lanes */
#define OP2_LIST(X) \
    X(addss, 0, 0, 1) X(addsd, 1, 8, 1) X(addps, 0, 0, 4) X(addpd, 1, 8, 2) \
    X(subss, 0, 1, 1) X(subsd, 1, 9, 1) X(subps, 0, 1, 4) X(subpd, 1, 9, 2) \
    X(mulss, 0, 2, 1) X(mulsd, 1, 10, 1) X(mulps, 0, 2, 4) X(mulpd, 1, 10, 2) \
    X(divss, 0, 3, 1) X(divsd, 1, 11, 1) X(divps, 0, 3, 4) X(divpd, 1, 11, 2) \
    X(minss, 0, 4, 1) X(minsd, 1, 12, 1) X(minps, 0, 4, 4) X(minpd, 1, 12, 2) \
    X(maxss, 0, 5, 1) X(maxsd, 1, 13, 1) X(maxps, 0, 5, 4) X(maxpd, 1, 13, 2) \
    X(sqrtss, 0, 6, 1) X(sqrtsd, 1, 14, 1) X(sqrtps, 0, 6, 4) X(sqrtpd, 1, 14, 2) \
    X(cvtss2sd, 0, 16, 1) X(cvtsd2ss, 1, 17, 1) X(cvtps2pd, 0, 16, 2) X(cvtpd2ps, 1, 17, 2) \
    X(cvtdq2ps, 2, 18, 4) X(cvtdq2pd, 2, 20, 2) X(cvtps2dq, 0, 24, 4) X(cvttps2dq, 0, 25, 4) \
    X(cvtpd2dq, 1, 28, 2) X(cvttpd2dq, 1, 29, 2)

OP2_LIST(W_OP2)

#define WI(nm, MAC, sk, imm) W_IMM(nm, MAC, imm)
#define CMP_IMMS(X, nm, MAC, sk) \
    X(nm, MAC, sk, 0) X(nm, MAC, sk, 1) X(nm, MAC, sk, 2) X(nm, MAC, sk, 3) \
    X(nm, MAC, sk, 4) X(nm, MAC, sk, 5) X(nm, MAC, sk, 6) X(nm, MAC, sk, 7)
#define ROUND_IMMS(X, nm, MAC, sk) \
    CMP_IMMS(X, nm, MAC, sk) X(nm, MAC, sk, 8) X(nm, MAC, sk, 12) X(nm, MAC, sk, 13) X(nm, MAC, sk, 255)
#define IMM_FAMILIES(X) \
    CMP_IMMS(X, cmpss, CMPSS, 0) CMP_IMMS(X, cmpsd, CMPSD, 1) CMP_IMMS(X, cmpps, CMPPS, 0) CMP_IMMS(X, cmppd, CMPPD, 1) \
    ROUND_IMMS(X, roundss, ROUNDSS, 0) ROUND_IMMS(X, roundsd, ROUNDSD, 1) \
    ROUND_IMMS(X, roundps, ROUNDPS, 0) ROUND_IMMS(X, roundpd, ROUNDPD, 1)
IMM_FAMILIES(WI)

W_SI2(cvtsi2ss, 32, int32_t)
W_SI2(cvtsi2ss, 64, int64_t)
W_SI2(cvtsi2sd, 32, int32_t)
W_SI2(cvtsi2sd, 64, int64_t)
W_2SI(cvtss2si, 32)
W_2SI(cvtss2si, 64)
W_2SI(cvtsd2si, 32)
W_2SI(cvtsd2si, 64)
W_2SI(cvttss2si, 32)
W_2SI(cvttss2si, 64)
W_2SI(cvttsd2si, 32)
W_2SI(cvttsd2si, 64)
W_COMI(ucomiss)
W_COMI(ucomisd)
W_COMI(comiss)
W_COMI(comisd)
#ifdef MXFP_HAS_HADDPD
W_OP2(haddpd, 1, 8, 2)
#endif

#define E_OP2(nm, sk, sid, lanes) { #nm, sk, sid, lanes, 0, f_##nm },
#define EI(nm, MAC, sk, imm) { #nm "_" #imm, sk, -1, 0, 0, f_##nm##_##imm },
#define E_SI2(nm, bits, sk, sid) { #nm #bits, sk, sid, 1, 0, f_##nm##bits },
#define E_COMI(nm, sk, sid) { #nm, sk, sid, 1, 1, f_##nm },

const struct MxfpOp MXFP_CAT(MXFP_OPS_, TAG)[] = {
    OP2_LIST(E_OP2)
    IMM_FAMILIES(EI)
    E_SI2(cvtsi2ss, 32, 3, 18) E_SI2(cvtsi2ss, 64, 3, 19) E_SI2(cvtsi2sd, 32, 3, 20) E_SI2(cvtsi2sd, 64, 3, 21)
    E_SI2(cvtss2si, 32, 0, 24) E_SI2(cvtss2si, 64, 0, 26) E_SI2(cvtsd2si, 32, 1, 28) E_SI2(cvtsd2si, 64, 1, 30)
    E_SI2(cvttss2si, 32, 0, 25) E_SI2(cvttss2si, 64, 0, 27) E_SI2(cvttsd2si, 32, 1, 29) E_SI2(cvttsd2si, 64, 1, 31)
    E_COMI(ucomiss, 0, 32) E_COMI(ucomisd, 1, 34) E_COMI(comiss, 0, 33) E_COMI(comisd, 1, 35)
#ifdef MXFP_HAS_HADDPD
    { "haddpd", 1, 8, 2, 0, f_haddpd },
#endif
    { 0, 0, 0, 0, 0, 0 }
};
const unsigned MXFP_CAT(MXFP_N_, TAG) = (unsigned)(sizeof MXFP_CAT(MXFP_OPS_, TAG) / sizeof MXFP_CAT(MXFP_OPS_, TAG)[0]) - 1;

#endif
