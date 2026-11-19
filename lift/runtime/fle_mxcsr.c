/* MXCSR storage for fe-lift output. See fle_mxcsr.h. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fle_runtime.h"
#include "fle_mxcsr.h"

uint32_t fle_mxcsr = 0x1F80;

#ifdef FLE_MX_STATS
/* Measurement only (decision 0143; -DFLE_MX_STATS in a build that is not timed: a test instrumentation, the product defines it never): how often the fast path
 * of fle_mxcsr_flags.h applies per instruction class, which values LDMXCSR loads, and a sample of the operands. */
uint64_t fle_mx_stat[64][2];
uint64_t fle_mx_vstat[3][64];
uint64_t fle_mx_rcstat[3][64]; /* decision 0172: the directed-rounding path: [0] taken, [2] a lane outside its windows (ids as fle_mx_vstat) */
static const char *const fle_mx_stat_name[40] = {
    "addss", "subss", "mulss", "divss", "minss", "maxss", "sqrtss", "-", "addsd", "subsd", "mulsd", "divsd", "minsd", "maxsd", "sqrtsd", "-",
    "cvtss2sd", "cvtsd2ss", "cvtsi2ss32", "cvtsi2ss64", "cvtsi2sd32", "cvtsi2sd64", "-", "-",
    "cvtss2si32", "cvttss2si32", "cvtss2si64", "cvttss2si64", "cvtsd2si32", "cvttsd2si32", "cvtsd2si64", "cvttsd2si64",
    "ucomiss", "comiss", "ucomisd", "comisd", "-", "-", "-", "-"};
static uint32_t fle_mx_ld_seen[32];
static unsigned fle_mx_ld_n;

/* decision 0154: the vector fast path of fle_mxcsr_vec.h / fle_mxcsr_cvt.h. fle_mx_vstat[0] = taken, [1] = not taken because the guard on RC, PE and
 * PM failed (arithmetic only), [2] = not taken for another reason (an operand outside the windows). Ids: see the header. */
static const char *fle_mx_vstat_name(unsigned id, char *buf) {
    static const char *const ar[] = {"add", "sub", "mul", "div", "min", "max", "sqrt"};
    static const char *const cv[] = {"cvtss2sd", "cvtsd2ss", "cvtps2pd", "cvtpd2ps", "cvtdq2ps", "cvtdq2pd", "cvtps2dq", "cvttps2dq", "cvtpd2dq", "cvttpd2dq",
                                     "cvtsi2ss32", "cvtsi2ss64", "cvtsi2sd32", "cvtsi2sd64", "cvtss2si32", "cvtss2si64", "cvtsd2si32", "cvtsd2si64",
                                     "cvttss2si32", "cvttss2si64", "cvttsd2si32", "cvttsd2si64"};
    if (id < 32) {
        if ((id & 7u) > 6u) return 0;
        sprintf(buf, "%s%s%c", ar[id & 7u], (id & 16u) ? "p" : "s", (id & 8u) ? 'd' : 's');
        if (id & 16u) buf[strlen(buf) - 1] = (id & 8u) ? 'd' : 's';
        return buf;
    }
    if (id < 40) {
        sprintf(buf, "cmp%c%c", (id & 2u) ? 's' : 'p', (id & 4u) ? 'd' : 's');
        return buf;
    }
    if (id < 62) return cv[id - 40];
    return 0;
}

void fle_mx_vstats_dump(void) {
    unsigned i;
    char buf[32];
    fprintf(stderr, "fle-mx-vstats: begin\n");
    for (i = 0; i < 64; i++) {
        const char *nm;
        if (!(fle_mx_vstat[0][i] || fle_mx_vstat[1][i] || fle_mx_vstat[2][i])) continue;
        nm = fle_mx_vstat_name(i, buf);
        fprintf(stderr, "fle-mx-vstats: %-11s vec %llu guard_miss %llu window_miss %llu\n", nm ? nm : "?", (unsigned long long)fle_mx_vstat[0][i],
                (unsigned long long)fle_mx_vstat[1][i], (unsigned long long)fle_mx_vstat[2][i]);
    }
    fprintf(stderr, "fle-mx-vstats: end\n");
    for (i = 0; i < 64; i++) {
        const char *nm;
        if (!(fle_mx_rcstat[0][i] || fle_mx_rcstat[2][i])) continue;
        nm = fle_mx_vstat_name(i, buf);
        fprintf(stderr, "fle-mx-rcstats: %-11s rc %llu window_miss %llu\n", nm ? nm : "?", (unsigned long long)fle_mx_rcstat[0][i], (unsigned long long)fle_mx_rcstat[2][i]);
    }
}

void fle_mx_stats_dump(void) {
    unsigned i;
    fle_mx_vstats_dump();
    fprintf(stderr, "fle-mx-stats: begin\n");
    for (i = 0; i < 40; i++) {
        if (fle_mx_stat[i][0] || fle_mx_stat[i][1]) {
            fprintf(stderr, "fle-mx-stats: %-11s fast %llu slow %llu\n", fle_mx_stat_name[i], (unsigned long long)fle_mx_stat[i][1],
                    (unsigned long long)fle_mx_stat[i][0]);
        }
    }
    fprintf(stderr, "fle-mx-stats: end\n");
    fflush(stderr);
}

static void fle_mx_stats_atexit(void) {
    fle_mx_stats_dump();
}

__attribute__((constructor)) static void fle_mx_stats_init(void) {
    atexit(fle_mx_stats_atexit);
}
#endif


void fle_mxcsr_store(uint64_t a) {
    fle_st(a, 4, fle_mxcsr);
}

void fle_mxcsr_load(uint64_t a) {
    uint32_t v = (uint32_t)fle_ld(a, 4);
    if (v & 0xFFFF0000u) {
        fprintf(stderr, "fle: ldmxcsr reserved bits set (value %#x)\n", (unsigned)v);
        abort();
    }
#ifdef FLE_MX_STATS
    {
        unsigned i;
        for (i = 0; i < fle_mx_ld_n && fle_mx_ld_seen[i] != v; i++) {
        }
        if (i == fle_mx_ld_n && fle_mx_ld_n < 32) {
            fle_mx_ld_seen[fle_mx_ld_n++] = v;
            fprintf(stderr, "fle-mx-stats: ldmxcsr %#x (first time)\n", (unsigned)v);
        }
    }
#endif
    fle_mxcsr = v;
}
