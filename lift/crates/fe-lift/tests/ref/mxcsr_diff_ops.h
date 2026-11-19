/* Operation table of the pure-C versus host-reference comparison (tests/ref/mxcsr_diff_main.c).
 * p_* call the portable runtime/fle_mxcsr_flags.h (fle_mx_*), h_* the host reference tests/ref/fle_mxcsr_flags_host.h
 * (fle_hx_*: the instruction itself on the CPU). Both have the same signatures. Counts are cases per operation at the
 * default scale. */
#define N_DEF 100000u

struct Op {
    const char *name;
    int sk;   /* 0 f32 lanes, 1 f64 lanes, 2 i32 lanes, 3 GPR source */
    int comi;
    unsigned n;
    void (*p)(fle_xmm *, fle_xmm, uint64_t *);
    void (*h)(fle_xmm *, fle_xmm, uint64_t *);
};

#define W_OP2(nm, sk) \
    static void p_##nm(fle_xmm *d, fle_xmm s, uint64_t *g) { (void)g; *d = fle_mx_##nm(*d, s); } \
    static void h_##nm(fle_xmm *d, fle_xmm s, uint64_t *g) { (void)g; *d = fle_hx_##nm(*d, s); }
#define E_OP2(nm, sk) { #nm, sk, 0, N_DEF, p_##nm, h_##nm },

#define OP2_LIST(X) \
    X(addss, 0) X(addsd, 1) X(addps, 0) X(addpd, 1) \
    X(subss, 0) X(subsd, 1) X(subps, 0) X(subpd, 1) \
    X(mulss, 0) X(mulsd, 1) X(mulps, 0) X(mulpd, 1) \
    X(divss, 0) X(divsd, 1) X(divps, 0) X(divpd, 1) \
    X(minss, 0) X(minsd, 1) X(minps, 0) X(minpd, 1) \
    X(maxss, 0) X(maxsd, 1) X(maxps, 0) X(maxpd, 1) \
    X(sqrtss, 0) X(sqrtsd, 1) X(sqrtps, 0) X(sqrtpd, 1) \
    X(cvtss2sd, 0) X(cvtsd2ss, 1) X(cvtps2pd, 0) X(cvtpd2ps, 1) \
    X(cvtdq2ps, 2) X(cvtdq2pd, 2) X(cvtps2dq, 0) X(cvttps2dq, 0) X(cvtpd2dq, 1) X(cvttpd2dq, 1)

OP2_LIST(W_OP2)

/* immediate forms: instruction mnemonic in upper case for the macro, constant imm */
#define W_IMM(nm, MAC, imm) \
    static void p_##nm##_##imm(fle_xmm *d, fle_xmm s, uint64_t *g) { (void)g; *d = FLE_MX_##MAC(*d, s, imm); } \
    static void h_##nm##_##imm(fle_xmm *d, fle_xmm s, uint64_t *g) { (void)g; *d = FLE_HX_##MAC(*d, s, imm); }
#define E_IMM(nm, sk, imm) { #nm "_" #imm, sk, 0, N_DEF, p_##nm##_##imm, h_##nm##_##imm },

#define CMP_IMMS(X, nm, MAC, sk) \
    X(nm, MAC, sk, 0) X(nm, MAC, sk, 1) X(nm, MAC, sk, 2) X(nm, MAC, sk, 3) \
    X(nm, MAC, sk, 4) X(nm, MAC, sk, 5) X(nm, MAC, sk, 6) X(nm, MAC, sk, 7)
#define ROUND_IMMS(X, nm, MAC, sk) \
    CMP_IMMS(X, nm, MAC, sk) \
    X(nm, MAC, sk, 8) X(nm, MAC, sk, 9) X(nm, MAC, sk, 10) X(nm, MAC, sk, 11) \
    X(nm, MAC, sk, 12) X(nm, MAC, sk, 13) X(nm, MAC, sk, 14) X(nm, MAC, sk, 15) \
    X(nm, MAC, sk, 16) X(nm, MAC, sk, 130) X(nm, MAC, sk, 245) X(nm, MAC, sk, 255)

#define WI(nm, MAC, sk, imm) W_IMM(nm, MAC, imm)
#define EI(nm, MAC, sk, imm) E_IMM(nm, sk, imm)

#define IMM_FAMILIES(X) \
    CMP_IMMS(X, cmpss, CMPSS, 0) CMP_IMMS(X, cmpsd, CMPSD, 1) CMP_IMMS(X, cmpps, CMPPS, 0) CMP_IMMS(X, cmppd, CMPPD, 1) \
    ROUND_IMMS(X, roundss, ROUNDSS, 0) ROUND_IMMS(X, roundsd, ROUNDSD, 1) \
    ROUND_IMMS(X, roundps, ROUNDPS, 0) ROUND_IMMS(X, roundpd, ROUNDPD, 1)

IMM_FAMILIES(WI)

/* cvtsi2ss/sd: GPR source (sk 3), 32 and 64 bit */
#define W_SI2(nm, bits, T) \
    static void p_##nm##bits(fle_xmm *d, fle_xmm s, uint64_t *g) { (void)s; *d = fle_mx_##nm##bits(*d, (T)*g); } \
    static void h_##nm##bits(fle_xmm *d, fle_xmm s, uint64_t *g) { (void)s; *d = fle_hx_##nm##bits(*d, (T)*g); }
W_SI2(cvtsi2ss, 32, int32_t)
W_SI2(cvtsi2ss, 64, int64_t)
W_SI2(cvtsi2sd, 32, int32_t)
W_SI2(cvtsi2sd, 64, int64_t)

/* cvt[t]ss2si, cvt[t]sd2si: the result goes to *g */
#define W_2SI(nm, bits) \
    static void p_##nm##bits(fle_xmm *d, fle_xmm s, uint64_t *g) { (void)d; *g = fle_mx_##nm##bits(s); } \
    static void h_##nm##bits(fle_xmm *d, fle_xmm s, uint64_t *g) { (void)d; *g = fle_hx_##nm##bits(s); }
#define W_2SI_BOTH(nm) W_2SI(nm, 32) W_2SI(nm, 64)
W_2SI_BOTH(cvtss2si)
W_2SI_BOTH(cvtsd2si)
W_2SI_BOTH(cvttss2si)
W_2SI_BOTH(cvttsd2si)

#define W_COMI(nm) \
    static void p_##nm(fle_xmm *d, fle_xmm s, uint64_t *g) { (void)g; fle_mx_##nm(*d, s); } \
    static void h_##nm(fle_xmm *d, fle_xmm s, uint64_t *g) { (void)g; fle_hx_##nm(*d, s); }
W_COMI(ucomiss)
W_COMI(ucomisd)
W_COMI(comiss)
W_COMI(comisd)

static const struct Op OPS[] = {
    OP2_LIST(E_OP2)
    IMM_FAMILIES(EI)
    { "cvtsi2ss32", 3, 0, N_DEF, p_cvtsi2ss32, h_cvtsi2ss32 },
    { "cvtsi2ss64", 3, 0, N_DEF, p_cvtsi2ss64, h_cvtsi2ss64 },
    { "cvtsi2sd32", 3, 0, N_DEF, p_cvtsi2sd32, h_cvtsi2sd32 },
    { "cvtsi2sd64", 3, 0, N_DEF, p_cvtsi2sd64, h_cvtsi2sd64 },
    { "cvtss2si32", 0, 0, N_DEF, p_cvtss2si32, h_cvtss2si32 },
    { "cvtss2si64", 0, 0, N_DEF, p_cvtss2si64, h_cvtss2si64 },
    { "cvtsd2si32", 1, 0, N_DEF, p_cvtsd2si32, h_cvtsd2si32 },
    { "cvtsd2si64", 1, 0, N_DEF, p_cvtsd2si64, h_cvtsd2si64 },
    { "cvttss2si32", 0, 0, N_DEF, p_cvttss2si32, h_cvttss2si32 },
    { "cvttss2si64", 0, 0, N_DEF, p_cvttss2si64, h_cvttss2si64 },
    { "cvttsd2si32", 1, 0, N_DEF, p_cvttsd2si32, h_cvttsd2si32 },
    { "cvttsd2si64", 1, 0, N_DEF, p_cvttsd2si64, h_cvttsd2si64 },
    { "ucomiss", 0, 1, N_DEF, p_ucomiss, h_ucomiss },
    { "ucomisd", 1, 1, N_DEF, p_ucomisd, h_ucomisd },
    { "comiss", 0, 1, N_DEF, p_comiss, h_comiss },
    { "comisd", 1, 1, N_DEF, p_comisd, h_comisd },
};
