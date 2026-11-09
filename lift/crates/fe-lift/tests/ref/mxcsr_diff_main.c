/* Main of the pure-C versus host-reference comparison (included by mxcsr_diff.c after mxcsr_diff_ops.h, which has
 * the wrappers p_K for the portable runtime/fle_mxcsr_flags.h, h_K for the host reference
 * tests/ref/fle_mxcsr_flags_host.h, and the table OPS). Every case runs both and
 * compares XMM, the GPR value, all of MXCSR and (COMIS) ZF PF CF OF SF AF.
 *
 * Operands: fully random bit patterns plus classes that hit the edges: exponent 0/1/max, NaN payloads, quiet-bit
 * edges, mantissa edges, exact ties, exponents that make a product or quotient land on the underflow or overflow
 * boundary, near-equal operands (cancellation), and integer-conversion boundaries. RC, FTZ, DAZ are random per case,
 * and 1 case in 4 starts with sticky flags already set. All exceptions are masked (an unmasked flag aborts both).
 */
static uint64_t rs = 0x9e3779b97f4a7c15ull;

static uint64_t rnd(void) {
    rs ^= rs >> 12;
    rs ^= rs << 25;
    rs ^= rs >> 27;
    return rs * 0x2545f4914f6cdd1dull;
}

static unsigned rb(unsigned n) {
    return (unsigned)((rnd() >> 11) % n);
}

static uint64_t gen_mant(int mb) {
    uint64_t full = (UINT64_C(1) << mb) - 1, q = UINT64_C(1) << (mb - 1);
    switch (rb(12)) {
    case 0: return 0;
    case 1: return full;
    case 2: return 1;
    case 3: return q;
    case 4: return q - 1;
    case 5: return q | 1;
    case 6: return rnd() & full & (rnd() | rnd());
    case 7: { /* exact tie patterns: high bits random, then 1 and zeros */
        unsigned k = rb(mb);
        return ((rnd() & full) >> k << k | (UINT64_C(1) << k >> 1)) & full;
    }
    case 8: return (rnd() & full) >> rb(mb);
    default: return rnd() & full;
    }
}

/* one operand of format f64 (else binary32 in the low 32 bits) */
static uint64_t genv(int f64) {
    int mb = f64 ? 52 : 23, eb = f64 ? 11 : 8, bias = f64 ? 1023 : 127;
    uint64_t emax = (UINT64_C(1) << eb) - 1, sign = (uint64_t)(rnd() & 1) << (mb + eb), ex, bits;
    if (rb(10) == 0) {
        bits = rnd();
        return f64 ? bits : (bits & 0xffffffffull);
    }
    switch (rb(11)) {
    case 0: ex = 0; break;
    case 1: ex = 1 + rb(3); break;
    case 2: ex = emax - 1 - rb(3); break;
    case 3: ex = emax; break;
    case 4: ex = (uint64_t)bias - 3 + rb(80); break; /* conversion to integer boundaries and small numbers */
    case 5: ex = (uint64_t)bias + (rb(2) ? 23 : 52) - 2 + rb(5); break;
    case 6: ex = 1 + (rnd() % (emax - 1)); break;
    case 7: ex = (uint64_t)bias - 8 + rb(17); break;
    case 8: ex = (uint64_t)(bias + 31) - 2 + rb(5); break;
    case 9: ex = (uint64_t)(bias + 63) - 2 + rb(5); break;
    default: ex = rb(2) ? rb(4) : emax - rb(4); break;
    }
    bits = sign | (ex << mb) | gen_mant(mb);
    return bits;
}

/* second operand related to the first */
static uint64_t genb(int f64, uint64_t a) {
    int mb = f64 ? 52 : 23, eb = f64 ? 11 : 8, bias = f64 ? 1023 : 127;
    uint64_t emax = (UINT64_C(1) << eb) - 1, full = f64 ? UINT64_MAX : 0xffffffffull;
    int64_t ea = (int64_t)((a >> mb) & emax), t;
    uint64_t sign = (uint64_t)(rnd() & 1) << (mb + eb);
    switch (rb(14)) {
    case 0: return (a + rb(5) - 2) & full;                 /* a few ulps away */
    case 1: return (a ^ (UINT64_C(1) << (mb + eb))) & full; /* -a */
    case 2: return ((a ^ (UINT64_C(1) << (mb + eb))) + rb(5) - 2) & full;
    case 3: t = ea - rb(60); break;                         /* alignment distance */
    case 4: t = ea + rb(60); break;
    case 5: t = 1 + bias - ea + (int64_t)rb(5) - 2; break;  /* product at the underflow boundary */
    case 6: t = (int64_t)emax - 1 + bias - ea + (int64_t)rb(5) - 2; break; /* product at the overflow boundary */
    case 7: t = ea + bias - 1 + (int64_t)rb(5) - 2; break;  /* quotient at the underflow boundary */
    case 8: t = ea + bias - ((int64_t)emax - 1) + (int64_t)rb(5) - 2; break; /* quotient at the overflow boundary */
    case 9: t = ea + (int64_t)rb(5) - 2; break;
    default: return genv(f64);
    }
    if (t < 0) t = 0;
    if (t > (int64_t)emax) t = (int64_t)emax;
    return (sign | ((uint64_t)t << mb) | gen_mant(mb)) & full;
}

static uint32_t geni32(void) {
    switch (rb(8)) {
    case 0: return (uint32_t)rnd();
    case 1: return rb(40) - 20;
    case 2: return (uint32_t)((int32_t)(rb(2) ? 1 : -1) * (int32_t)((1u << 24) + rb(9) - 4));
    case 3: return rb(2) ? 0x7fffffffu - rb(4) : 0x80000000u + rb(4);
    case 4: return (uint32_t)rnd() >> rb(32);
    case 5: return (uint32_t)((int32_t)((uint32_t)rnd() >> rb(32)) * (rb(2) ? -1 : 1));
    default: return (uint32_t)rnd() >> rb(31) << rb(8);
    }
}

static uint64_t geni64(void) {
    switch (rb(8)) {
    case 0: return rnd();
    case 1: return rb(40) - 20;
    case 2: return (uint64_t)((int64_t)(rb(2) ? 1 : -1) * ((INT64_C(1) << 53) + (int64_t)rb(9) - 4));
    case 3: return rb(2) ? UINT64_C(0x7fffffffffffffff) - rb(4) : UINT64_C(0x8000000000000000) + rb(4);
    case 4: return rnd() >> rb(64);
    case 5: return (uint64_t)((int64_t)(rnd() >> rb(64)) * (rb(2) ? -1 : 1));
    case 6: return (uint64_t)(int64_t)(int32_t)geni32();
    default: return rnd() >> rb(63) << rb(12);
    }
}

static uint64_t fl_bits(void) {
    return ((uint64_t)FL.cf << 0) | ((uint64_t)FL.pf << 1) | ((uint64_t)FL.af << 2) | ((uint64_t)FL.zf << 3) |
           ((uint64_t)FL.sf << 4) | ((uint64_t)FL.of << 5);
}

uint64_t R[16];
fle_eflags FL;
fle_xmm X[16];

/* FNV-1a over the outputs of every case of an operation. Native (host reference) and wasm (pure C only: the
 * reference is built from the pure header there, see out-mxsoft/wasm) print comparable "HASH" lines. */
static uint64_t hash_mix(uint64_t h, uint64_t v) {
    int i;
    for (i = 0; i < 8; i++) {
        h ^= (v >> (8 * i)) & 0xff;
        h *= UINT64_C(0x100000001b3);
    }
    return h;
}

int main(int argc, char **argv) {
    unsigned scale = argc > 1 ? (unsigned)atoi(argv[1]) : 100000u;
    unsigned long long total = 0, bad_total = 0;
    unsigned o, c, i;
    for (o = 0; o < sizeof OPS / sizeof OPS[0]; o++) {
        const struct Op *op = &OPS[o];
        unsigned n = (unsigned)((unsigned long long)op->n * scale / 100000u), bad = 0, newflag = 0, ctl = 0;
        uint32_t raised = 0;
        uint64_t hp = UINT64_C(0xcbf29ce484222325), hh = UINT64_C(0xcbf29ce484222325);
        const char *s = op->name;
        rs = 0x9e3779b97f4a7c15ull;
        while (*s) {
            rs = (rs ^ (unsigned char)*s++) * 0x100000001b3ull;
        }
        for (i = 0; i < 8; i++) rnd();
        for (c = 0; c < n; c++) {
            fle_xmm d, s0, dp, dh;
            uint64_t g = 0, gp, gh, fp = 0, fh = 0;
            uint32_t mx = 0x1f80u | ((uint32_t)rb(4) << 13) | ((uint32_t)rb(2) << 15) | ((uint32_t)rb(2) << 6), mp, mh;
            if (rb(4) == 0) mx |= (uint32_t)rb(64);
            if (op->sk == 1) {
                for (i = 0; i < 2; i++) {
                    d.u64[i] = genv(1);
                    s0.u64[i] = genb(1, d.u64[i]);
                }
            } else if (op->sk == 0) {
                for (i = 0; i < 4; i++) {
                    d.u32[i] = (uint32_t)genv(0);
                    s0.u32[i] = (uint32_t)genb(0, d.u32[i]);
                }
            } else {
                for (i = 0; i < 4; i++) {
                    d.u32[i] = (uint32_t)rnd();
                    s0.u32[i] = geni32();
                }
                if (op->sk == 3) {
                    g = geni64();
                }
            }
            FL.cf = FL.pf = FL.af = FL.zf = FL.sf = FL.of = FLE_UNDEF;
            dh = d;
            gh = g;
            fle_mxcsr = mx;
            op->h(&dh, s0, &gh);
            mh = fle_mxcsr;
            if (op->comi) fh = fl_bits();
            FL.cf = FL.pf = FL.af = FL.zf = FL.sf = FL.of = FLE_UNDEF;
            dp = d;
            gp = g;
            fle_mxcsr = mx;
            op->p(&dp, s0, &gp);
            mp = fle_mxcsr;
            if (op->comi) fp = fl_bits();
            raised |= mh & 0x3f;
            hp = hash_mix(hash_mix(hash_mix(hash_mix(hash_mix(hp, dp.u64[0]), dp.u64[1]), gp), mp), fp);
            hh = hash_mix(hash_mix(hash_mix(hash_mix(hash_mix(hh, dh.u64[0]), dh.u64[1]), gh), mh), fh);
            if ((mh ^ mx) & 0x3f) newflag++;
            if (mx & 0xe040u) ctl++;
            if (memcmp(&dp, &dh, sizeof dp) || gp != gh || mp != mh || fp != fh) {
                bad++;
                if (bad <= 5) {
                    printf("MISMATCH %s case %u mx_in %#x\n  d  %016llx %016llx\n  s  %016llx %016llx g %016llx\n"
                           "  pure %016llx %016llx g %016llx mx %#x fl %#llx\n  host %016llx %016llx g %016llx mx %#x fl %#llx\n",
                           op->name, c, mx, (unsigned long long)d.u64[1], (unsigned long long)d.u64[0],
                           (unsigned long long)s0.u64[1], (unsigned long long)s0.u64[0], (unsigned long long)g,
                           (unsigned long long)dp.u64[1], (unsigned long long)dp.u64[0], (unsigned long long)gp, mp,
                           (unsigned long long)fp, (unsigned long long)dh.u64[1], (unsigned long long)dh.u64[0],
                           (unsigned long long)gh, mh, (unsigned long long)fh);
                }
            }
        }
        printf("OP %s cases %u mismatches %u raised_union %#x cases_new_flag %u nondefault_ctl %u\n", op->name, n, bad,
               (unsigned)raised, newflag, ctl);
        printf("HASH %s %u pure %016llx ref %016llx\n", op->name, n, (unsigned long long)hp, (unsigned long long)hh);
        total += n;
        bad_total += bad;
    }
    printf("TOTAL cases %llu mismatches %llu ops %u\n", total, bad_total, (unsigned)(sizeof OPS / sizeof OPS[0]));
    return bad_total ? 1 : 0;
}
