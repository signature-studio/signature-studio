/* Checks the flag helpers in lift/runtime/fle_flags.h.
 *
 * Part 1 compares each helper with the host CPU (host_ref.c, pushfq).
 * Only flags the SDM defines for that operation are compared (see the table
 * in docs/milestones/M1-remaining-work.md section 2.3).
 *
 * Part 2 checks hand-computed values from the same table at boundaries. The
 * expected values here do not come from the host.
 *
 * Exit status is the number of failed checks (0 on success). Built and run by
 * tests/flags.rs. */
#include <stdio.h>
#include <stdint.h>
#include <string.h>

#include "../../../runtime/fle_flags.h"
#include "host_ref.c"

fle_eflags FL;

static long checks, fails;

static void fail_msg(const char *what, uint64_t a, uint64_t b, unsigned w, const char *detail) {
    fails++;
    if (fails <= 40) {
        fprintf(stderr, "FAIL %s w=%u a=%#llx b=%#llx %s\n", what, w, (unsigned long long)a,
                (unsigned long long)b, detail);
    }
}

/* Bits of the defined-flag mask. */
#define M_CF 0x01u
#define M_PF 0x02u
#define M_AF 0x04u
#define M_ZF 0x08u
#define M_SF 0x10u
#define M_OF 0x20u
#define M_ALL 0x3fu

/* Compare FL with the host EFLAGS word for the flags in mask. */
static void cmp_flags(const char *what, uint64_t a, uint64_t b, unsigned w, uint64_t hostfl, unsigned mask) {
    static const uint64_t hb[6] = {0x001, 0x004, 0x010, 0x040, 0x080, 0x800};
    static const unsigned mb[6] = {M_CF, M_PF, M_AF, M_ZF, M_SF, M_OF};
    const uint8_t got[6] = {FL.cf, FL.pf, FL.af, FL.zf, FL.sf, FL.of};
    static const char *nm[6] = {"CF", "PF", "AF", "ZF", "SF", "OF"};
    int i;
    checks++;
    for (i = 0; i < 6; i++) {
        if (!(mask & mb[i])) {
            continue;
        }
        uint8_t want = (hostfl & hb[i]) ? 1 : 0;
        if (got[i] != want) {
            char d[96];
            snprintf(d, sizeof d, "%s got=%u host=%u", nm[i], (unsigned)got[i], (unsigned)want);
            fail_msg(what, a, b, w, d);
            return;
        }
    }
}

static void cmp_res(const char *what, uint64_t a, uint64_t b, unsigned w, uint64_t got, uint64_t want) {
    uint64_t m = fle_mask(w);
    checks++;
    if ((got & m) != (want & m)) {
        char d[96];
        snprintf(d, sizeof d, "result got=%#llx host=%#llx", (unsigned long long)(got & m),
                 (unsigned long long)(want & m));
        fail_msg(what, a, b, w, d);
    }
}

static void set_init(unsigned cf_in) {
    FL.cf = (uint8_t)(cf_in & 1);
    FL.pf = FL.af = FL.zf = FL.sf = FL.of = 0;
}

static const char *alu_name[] = {"add", "adc", "sub", "sbb", "and", "or", "xor", "cmp", "test"};
static const char *un_name[] = {"inc", "dec", "neg", "not"};
static const char *sh_name[] = {"shl", "shr", "sar"};

/* Run one binary ALU op (0..8) through the helpers. */
static uint64_t run_bin(unsigned op, unsigned w, uint64_t a, uint64_t b, unsigned cf_in) {
    uint64_t m = fle_mask(w);
    set_init(cf_in);
    switch (op) {
    case 0: return fle_op_add(w, a, b, 0);
    case 1: return fle_op_add(w, a, b, (uint8_t)(FL.cf));
    case 2: return fle_op_sub(w, a, b, 0);
    case 3: return fle_op_sub(w, a, b, (uint8_t)(FL.cf));
    case 4: return fle_op_logic(w, (a & m) & (b & m));
    case 5: return fle_op_logic(w, (a & m) | (b & m));
    case 6: return fle_op_logic(w, (a & m) ^ (b & m));
    case 7: (void)fle_op_sub(w, a, b, 0); return a;
    default: (void)fle_op_logic(w, (a & m) & (b & m)); return a;
    }
}

static unsigned bin_mask(unsigned op) {
    if (op <= 3 || op == 7) {
        return M_ALL;
    }
    return M_ALL & ~M_AF;
}

/* Host result for a binary ALU op. */
static uint64_t host_bin_res(unsigned op, uint64_t a, uint64_t b, unsigned w, unsigned cf_in, uint64_t *fl) {
    static const unsigned hop[] = {0, 1, 2, 3, 4, 5, 6, 7, 8};
    host_out o = host_alu(hop[op], w, a, b, cf_in, 0);
    *fl = o.flags;
    return o.res;
}

static void test_bin(unsigned op, unsigned w, uint64_t a, uint64_t b, unsigned cf_in) {
    uint64_t hfl = 0;
    uint64_t hr = host_bin_res(op, a, b, w, cf_in, &hfl);
    uint64_t r = run_bin(op, w, a, b, cf_in);
    if (op != 7 && op != 8) {
        cmp_res(alu_name[op], a, b, w, r, hr);
    } else {
        /* cmp and test do not write a result. */
        checks++;
    }
    cmp_flags(alu_name[op], a, b, w, hfl, bin_mask(op));
}

static void test_un(unsigned op, unsigned w, uint64_t a, unsigned cf_in) {
    uint64_t m = fle_mask(w);
    host_out o = host_alu(9 + op, w, a, 0, cf_in, 0);
    unsigned mask;
    uint64_t r;
    set_init(cf_in);
    switch (op) {
    case 0: r = fle_op_inc(w, a); mask = M_ALL & ~M_CF; break;
    case 1: r = fle_op_dec(w, a); mask = M_ALL & ~M_CF; break;
    case 2: r = fle_op_neg(w, a); mask = M_ALL; break;
    default: r = fle_op_not(w, a); mask = M_ALL; break;
    }
    cmp_res(un_name[op], a, 0, w, r, o.res);
    /* INC/DEC keep CF: the host result has CF = cf_in, which we compare too. */
    if (op <= 1) {
        checks++;
        if (FL.cf != cf_in) {
            fail_msg(un_name[op], a, 0, w, "CF changed");
        }
    }
    cmp_flags(un_name[op], a, 0, w, o.flags, mask);
    (void)m;
}

/* Count rule: a shift with count 0 changes no flags. A count above the
 * width (8 and 16 bit only) is not defined, so those inputs are not run. */
static int shift_count_ok(unsigned w, uint64_t cnt) {
    unsigned c = (unsigned)(cnt & (w == 64 ? 0x3f : 0x1f));
    return !(c > w && c != 0);
}

static void test_shift(unsigned op, unsigned w, uint64_t a, uint64_t cnt, unsigned cf_in) {
    uint64_t m = fle_mask(w);
    unsigned c = (unsigned)(cnt & (w == 64 ? 0x3f : 0x1f));
    host_out o;
    uint64_t r;
    unsigned mask;
    if (!shift_count_ok(w, cnt)) {
        return;
    }
    o = host_alu(13 + op, w, a, 0, cf_in, cnt);
    set_init(cf_in);
    switch (op) {
    case 0: r = fle_op_shl(w, a, cnt); break;
    case 1: r = fle_op_shr(w, a, cnt); break;
    default: r = fle_op_sar(w, a, cnt); break;
    }
    cmp_res(sh_name[op], a, cnt, w, r, o.res);
    if (c == 0) {
        mask = M_ALL;
    } else if (c == 1) {
        mask = M_CF | M_PF | M_ZF | M_SF | M_OF;
    } else {
        mask = M_CF | M_PF | M_ZF | M_SF;
    }
    cmp_flags(sh_name[op], a, cnt, w, o.flags, mask);
    (void)m;
}

static void test_imul2(unsigned w, uint64_t a, uint64_t b, unsigned cf_in) {
    host_out o = host_alu(16, w, a, b, cf_in, 0);
    uint64_t r;
    set_init(cf_in);
    r = fle_op_imul(w, a, b);
    cmp_res("imul", a, b, w, r, o.res);
    cmp_flags("imul", a, b, w, o.flags, M_CF | M_OF);
}

static void test_mul(unsigned w, uint64_t a, uint64_t b, unsigned cf_in) {
    host_out o = host_alu(17, w, a, b, cf_in, 0);
    uint64_t lo = 0, hi;
    set_init(cf_in);
    hi = fle_op_mul(w, a, b, &lo);
    cmp_res("mul.lo", a, b, w, lo, o.res);
    cmp_res("mul.hi", a, b, w, hi, o.hi);
    cmp_flags("mul", a, b, w, o.flags, M_CF | M_OF);
}

static void test_comi(unsigned which, uint64_t a, uint64_t b) {
    host_out o = host_alu(18 + which, which == 0 ? 32 : 64, a, b, 0, 0);
    set_init(0);
    if (which == 0) {
        fle_op_ucomiss(a, b);
        cmp_flags("ucomiss", a, b, 32, o.flags, M_ALL);
    } else if (which == 1) {
        fle_op_ucomisd(a, b);
        cmp_flags("ucomisd", a, b, 64, o.flags, M_ALL);
    } else {
        fle_op_comisd(a, b);
        cmp_flags("comisd", a, b, 64, o.flags, M_ALL);
    }
}

/* Part 2: hand-computed from the SDM table. */
static void expect(const char *name, int ok) {
    checks++;
    if (!ok) {
        fails++;
        fprintf(stderr, "FAIL table %s\n", name);
    }
}

static void table_checks(void) {
    uint64_t r;
    set_init(0);
    r = fle_op_add(8, 0xff, 1, 0);
    expect("add8 ff+1 r", r == 0);
    expect("add8 ff+1 CF/ZF/PF/AF/SF/OF", FL.cf == 1 && FL.zf == 1 && FL.pf == 1 && FL.af == 1 && FL.sf == 0 && FL.of == 0);

    set_init(0);
    r = fle_op_add(8, 0x7f, 1, 0);
    expect("add8 7f+1 r", r == 0x80);
    expect("add8 7f+1 flags", FL.cf == 0 && FL.pf == 0 && FL.af == 1 && FL.zf == 0 && FL.sf == 1 && FL.of == 1);

    set_init(0);
    r = fle_op_sub(8, 0, 1, 0);
    expect("sub8 0-1 r", r == 0xff);
    expect("sub8 0-1 flags", FL.cf == 1 && FL.sf == 1 && FL.of == 0 && FL.zf == 0 && FL.pf == 1 && FL.af == 1);

    set_init(0);
    r = fle_op_sub(8, 0x80, 1, 0);
    expect("sub8 80-1 r", r == 0x7f);
    expect("sub8 80-1 flags", FL.cf == 0 && FL.sf == 0 && FL.of == 1);

    set_init(0);
    (void)fle_op_sub(8, 0, 0, 0);
    expect("cmp8 0,0 ZF", FL.zf == 1 && FL.cf == 0);

    set_init(1);
    r = fle_op_inc(8, 0xff);
    expect("inc8 ff with CF=1: r and CF kept", r == 0 && FL.cf == 1 && FL.zf == 1 && FL.of == 0);

    set_init(0);
    r = fle_op_neg(8, 0x80);
    expect("neg8 80", r == 0x80 && FL.cf == 1 && FL.of == 1 && FL.sf == 1);
    set_init(0);
    r = fle_op_neg(8, 0);
    expect("neg8 0", r == 0 && FL.cf == 0 && FL.zf == 1);

    set_init(0);
    r = fle_op_logic(8, 0xf0 & 0x0f);
    expect("and8 f0&0f", r == 0 && FL.zf == 1 && FL.pf == 1 && FL.sf == 0 && FL.cf == 0 && FL.of == 0);

    set_init(1);
    r = fle_op_shl(8, 0x81, 1);
    expect("shl8 81,1", r == 0x02 && FL.cf == 1 && FL.of == 1);
    set_init(1);
    r = fle_op_sar(8, 0x81, 1);
    expect("sar8 81,1", r == 0xc0 && FL.cf == 1 && FL.of == 0 && FL.sf == 1);
    set_init(0);
    r = fle_op_shr(8, 0x81, 1);
    expect("shr8 81,1", r == 0x40 && FL.cf == 1 && FL.of == 1);

    set_init(1);
    FL.zf = 1;
    r = fle_op_shl(8, 0x12, 0);
    expect("shl count 0 leaves all flags", r == 0x12 && FL.cf == 1 && FL.zf == 1 && FL.pf == 0);

    set_init(0);
    r = fle_op_imul(16, 0x4000, 2);
    expect("imul16 4000*2 overflow", (r & 0xffff) == 0x8000 && FL.cf == 1 && FL.of == 1);

    set_init(0);
    r = fle_op_imul(16, 0x0100, 0x0100);
    expect("imul16 0100*0100 overflow", (r & 0xffff) == 0 && FL.cf == 1 && FL.of == 1);

    set_init(0);
    {
        uint64_t lo = 0;
        uint64_t hi = fle_op_mul(32, 0x10000, 0x10000, &lo);
        expect("mul32 10000*10000 hi=1 lo=0", hi == 1 && lo == 0 && FL.cf == 1 && FL.of == 1);
    }

    set_init(0);
    fle_op_ucomisd(0x3ff0000000000000ULL, 0x7ff8000000000000ULL);
    expect("ucomisd 1.0 vs QNaN: ZF=PF=CF=1", FL.zf == 1 && FL.pf == 1 && FL.cf == 1 && FL.of == 0 && FL.sf == 0 && FL.af == 0);

    set_init(0);
    fle_op_comisd(0x3ff0000000000000ULL, 0x3ff0000000000000ULL);
    expect("comisd 1.0 vs 1.0: ZF=1 only", FL.zf == 1 && FL.pf == 0 && FL.cf == 0);

    set_init(0);
    fle_op_ucomiss(0x3f800000u, 0x40000000u);
    expect("ucomiss 1.0 < 2.0: CF=1", FL.cf == 1 && FL.zf == 0 && FL.pf == 0);
}

/* Boundary values for width w, and random ones. */
static uint64_t bval(unsigned w, unsigned i) {
    uint64_t m = fle_mask(w), s = UINT64_C(1) << (w - 1);
    static const uint64_t base[] = {0, 1, 2, 0x0f, 0x55, 0xaa, 0x7f, 0x80, 0xfe, 0xff};
    switch (i) {
    case 0: return 0;
    case 1: return 1;
    case 2: return m;
    case 3: return m - 1;
    case 4: return s;
    case 5: return s - 1;
    case 6: return (s + 1) & m;
    case 7: return 2;
    case 8: return 0x5555555555555555ULL & m;
    case 9: return 0xaaaaaaaaaaaaaaaaULL & m;
    default: return base[(i - 10) % 10] & m;
    }
}

#define NB 20
#define NR 1500

static uint64_t rng = 0x9e3779b97f4a7c15ULL;
static uint64_t xr(void) {
    rng ^= rng << 13;
    rng ^= rng >> 7;
    rng ^= rng << 17;
    return rng;
}

static void run_all(void) {
    static const unsigned widths[] = {8, 16, 32, 64};
    unsigned wi, op, cf, i, j, k;
    for (wi = 0; wi < 4; wi++) {
        unsigned w = widths[wi];
        for (cf = 0; cf < 2; cf++) {
            for (op = 0; op < 9; op++) {
                for (i = 0; i < NB; i++) {
                    for (j = 0; j < NB; j++) {
                        test_bin(op, w, bval(w, i), bval(w, j), cf);
                    }
                }
                for (k = 0; k < NR; k++) {
                    test_bin(op, w, xr() & fle_mask(w), xr() & fle_mask(w), cf);
                }
            }
            for (op = 0; op < 4; op++) {
                for (i = 0; i < NB; i++) {
                    test_un(op, w, bval(w, i), cf);
                }
                for (k = 0; k < NR; k++) {
                    test_un(op, w, xr() & fle_mask(w), cf);
                }
            }
            for (op = 0; op < 3; op++) {
                static const unsigned cnts[] = {0, 1, 2, 3, 7, 8, 9, 15, 16, 17, 31, 32, 33, 63, 64, 65, 255};
                unsigned ci;
                for (i = 0; i < NB; i++) {
                    for (ci = 0; ci < sizeof cnts / sizeof cnts[0]; ci++) {
                        test_shift(op, w, bval(w, i), cnts[ci], cf);
                    }
                }
                for (k = 0; k < NR; k++) {
                    test_shift(op, w, xr() & fle_mask(w), xr() % 70, cf);
                }
            }
            if (w >= 16) {
                for (i = 0; i < NB; i++) {
                    for (j = 0; j < NB; j++) {
                        test_imul2(w, bval(w, i), bval(w, j), cf);
                    }
                }
                for (k = 0; k < NR; k++) {
                    test_imul2(w, xr() & fle_mask(w), xr() & fle_mask(w), cf);
                }
            }
            for (i = 0; i < NB; i++) {
                for (j = 0; j < NB; j++) {
                    test_mul(w, bval(w, i), bval(w, j), cf);
                }
            }
            for (k = 0; k < NR; k++) {
                test_mul(w, xr() & fle_mask(w), xr() & fle_mask(w), cf);
            }
        }
    }
    {
        static const uint64_t fv[] = {
            0x00000000u, 0x80000000u, 0x3f800000u, 0x40000000u, 0x7f800000u, 0xff800000u,
            0x7fc00000u, 0x7f800001u, 0x00000001u, 0xbf800000u,
        };
        static const uint64_t dv[] = {
            0x0000000000000000ULL, 0x8000000000000000ULL, 0x3ff0000000000000ULL, 0x4000000000000000ULL,
            0x7ff0000000000000ULL, 0xfff0000000000000ULL, 0x7ff8000000000000ULL, 0x7ff0000000000001ULL,
            0x0000000000000001ULL, 0xbff0000000000000ULL,
        };
        unsigned a, b;
        for (a = 0; a < 10; a++) {
            for (b = 0; b < 10; b++) {
                test_comi(0, fv[a], fv[b]);
                test_comi(1, dv[a], dv[b]);
                test_comi(2, dv[a], dv[b]);
            }
        }
        for (k = 0; k < NR; k++) {
            test_comi(1, xr(), xr());
            test_comi(2, xr(), xr());
        }
    }
}

/* Condition codes (SDM Vol. 2 Jcc table), checked against the formula for every
 * combination of CF, ZF, SF, OF, PF. */
static void cc_checks(void) {
    unsigned cc, bits;
    for (bits = 0; bits < 32; bits++) {
        unsigned cf = bits & 1, pf = (bits >> 1) & 1, zf = (bits >> 2) & 1, sf = (bits >> 3) & 1, of = (bits >> 4) & 1;
        FL.cf = (uint8_t)cf; FL.pf = (uint8_t)pf; FL.af = 0;
        FL.zf = (uint8_t)zf; FL.sf = (uint8_t)sf; FL.of = (uint8_t)of;
        for (cc = 0; cc < 16; cc++) {
            int want = 0, got;
            switch (cc) {
            case 0x0: want = of; break;
            case 0x1: want = !of; break;
            case 0x2: want = cf; break;
            case 0x3: want = !cf; break;
            case 0x4: want = zf; break;
            case 0x5: want = !zf; break;
            case 0x6: want = cf || zf; break;
            case 0x7: want = !cf && !zf; break;
            case 0x8: want = sf; break;
            case 0x9: want = !sf; break;
            case 0xa: want = pf; break;
            case 0xb: want = !pf; break;
            case 0xc: want = sf != of; break;
            case 0xd: want = sf == of; break;
            case 0xe: want = zf || (sf != of); break;
            default: want = !zf && (sf == of); break;
            }
            got = fle_cc(cc) != 0;
            expect("cc table", got == want);
        }
    }
}

int main(void) {
    run_all();
    table_checks();
    cc_checks();
    printf("checks %ld\nfailures %ld\n", checks, fails);
    return fails > 0 ? 1 : 0;
}
