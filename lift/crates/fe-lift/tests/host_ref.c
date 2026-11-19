/* Host reference for the flag tests. Runs one operation on the host CPU and
 * reads EFLAGS with pushfq. Before the operation, EFLAGS is set to CF = cf_in
 * and every other status flag to 0, so an instruction that leaves a flag alone
 * is visible in the result.
 *
 * pushfq and popfq use the stack below rsp, so build with -mno-red-zone.
 * Requires x86_64. Included by flags_check.c only. */
#include <stdint.h>

typedef struct {
    uint64_t res;
    uint64_t hi;
    uint64_t flags;
} host_out;

/* CF 0x001, PF 0x004, AF 0x010, ZF 0x040, SF 0x080, OF 0x800 */
#define HOST_FLAG_BITS 0x8d5u

#if defined(__x86_64__)

#define HINIT "pushq %[i]\n\tpopfq\n\t"
#define HSAVE "pushfq\n\tpopq %[f]\n\t"

/* op: 0 add, 1 adc, 2 sub, 3 sbb, 4 and, 5 or, 6 xor, 7 cmp, 8 test,
 *     9 inc, 10 dec, 11 neg, 12 not, 13 shl, 14 shr, 15 sar,
 *     16 imul (2 operands), 17 mul (rdx:rax), 18 ucomiss, 19 ucomisd, 20 comisd.
 * w: 8, 16, 32, 64. cnt: shift count (only the low byte is used). */
static host_out host_alu(unsigned op, unsigned w, uint64_t a, uint64_t b, unsigned cf_in, uint64_t cnt) {
    uint64_t x = a, y = b, dx = 0, fl = 0;
    uint64_t init = UINT64_C(0x2) | (uint64_t)(cf_in & 1);
    unsigned char c8 = (unsigned char)cnt;
    host_out o = {0, 0, 0};

    if (op == 18) {
        __asm__ volatile(HINIT "movd %k[a], %%xmm0\n\t"
                               "movd %k[b], %%xmm1\n\t"
                               "ucomiss %%xmm1, %%xmm0\n\t" HSAVE
                         : [f] "=&r"(fl)
                         : [a] "r"(x), [b] "r"(y), [i] "r"(init)
                         : "cc", "xmm0", "xmm1");
        o.flags = fl & HOST_FLAG_BITS;
        return o;
    }
    if (op == 19 || op == 20) {
        if (op == 19) {
            __asm__ volatile(HINIT "movq %[a], %%xmm0\n\t"
                                   "movq %[b], %%xmm1\n\t"
                                   "ucomisd %%xmm1, %%xmm0\n\t" HSAVE
                             : [f] "=&r"(fl)
                             : [a] "r"(x), [b] "r"(y), [i] "r"(init)
                             : "cc", "xmm0", "xmm1");
        } else {
            __asm__ volatile(HINIT "movq %[a], %%xmm0\n\t"
                                   "movq %[b], %%xmm1\n\t"
                                   "comisd %%xmm1, %%xmm0\n\t" HSAVE
                             : [f] "=&r"(fl)
                             : [a] "r"(x), [b] "r"(y), [i] "r"(init)
                             : "cc", "xmm0", "xmm1");
        }
        o.flags = fl & HOST_FLAG_BITS;
        return o;
    }

    if (op == 17) {
        switch (w) {
        case 8:
            __asm__ volatile(HINIT "mulb %b[b]\n\t" HSAVE
                             : [a] "+a"(x), [d] "=&d"(dx), [f] "=&r"(fl)
                             : [b] "r"(y), [i] "r"(init) : "cc");
            o.res = x & 0xff;
            o.hi = (x >> 8) & 0xff;
            break;
        case 16:
            __asm__ volatile(HINIT "mulw %w[b]\n\t" HSAVE
                             : [a] "+a"(x), [d] "=&d"(dx), [f] "=&r"(fl)
                             : [b] "r"(y), [i] "r"(init) : "cc");
            o.res = x & 0xffff;
            o.hi = dx & 0xffff;
            break;
        case 32:
            __asm__ volatile(HINIT "mull %k[b]\n\t" HSAVE
                             : [a] "+a"(x), [d] "=&d"(dx), [f] "=&r"(fl)
                             : [b] "r"(y), [i] "r"(init) : "cc");
            o.res = x & 0xffffffffu;
            o.hi = dx & 0xffffffffu;
            break;
        default:
            __asm__ volatile(HINIT "mulq %q[b]\n\t" HSAVE
                             : [a] "+a"(x), [d] "=&d"(dx), [f] "=&r"(fl)
                             : [b] "r"(y), [i] "r"(init) : "cc");
            o.res = x;
            o.hi = dx;
            break;
        }
        o.flags = fl & HOST_FLAG_BITS;
        return o;
    }

    if (op == 16) {
        switch (w) {
        case 16:
            __asm__ volatile(HINIT "imulw %w[b], %w[a]\n\t" HSAVE
                             : [a] "+r"(x), [f] "=&r"(fl)
                             : [b] "r"(y), [i] "r"(init) : "cc");
            break;
        case 32:
            __asm__ volatile(HINIT "imull %k[b], %k[a]\n\t" HSAVE
                             : [a] "+r"(x), [f] "=&r"(fl)
                             : [b] "r"(y), [i] "r"(init) : "cc");
            break;
        default:
            __asm__ volatile(HINIT "imulq %q[b], %q[a]\n\t" HSAVE
                             : [a] "+r"(x), [f] "=&r"(fl)
                             : [b] "r"(y), [i] "r"(init) : "cc");
            break;
        }
        o.res = x;
        o.flags = fl & HOST_FLAG_BITS;
        return o;
    }

    if (op <= 8) {
#define HBIN(MN, SUF_B, SUF_W, SUF_L, SUF_Q)                                                   \
    switch (w) {                                                                              \
    case 8:                                                                                   \
        __asm__ volatile(HINIT MN "b %b[b], %b[a]\n\t" HSAVE                                  \
                         : [a] "+r"(x), [f] "=&r"(fl) : [b] "r"(y), [i] "r"(init) : "cc");   \
        break;                                                                                \
    case 16:                                                                                  \
        __asm__ volatile(HINIT MN "w %w[b], %w[a]\n\t" HSAVE                                  \
                         : [a] "+r"(x), [f] "=&r"(fl) : [b] "r"(y), [i] "r"(init) : "cc");   \
        break;                                                                                \
    case 32:                                                                                  \
        __asm__ volatile(HINIT MN "l %k[b], %k[a]\n\t" HSAVE                                  \
                         : [a] "+r"(x), [f] "=&r"(fl) : [b] "r"(y), [i] "r"(init) : "cc");   \
        break;                                                                                \
    default:                                                                                  \
        __asm__ volatile(HINIT MN "q %q[b], %q[a]\n\t" HSAVE                                  \
                         : [a] "+r"(x), [f] "=&r"(fl) : [b] "r"(y), [i] "r"(init) : "cc");   \
        break;                                                                                \
    }
        switch (op) {
        case 0: HBIN("add", 0, 0, 0, 0) break;
        case 1: HBIN("adc", 0, 0, 0, 0) break;
        case 2: HBIN("sub", 0, 0, 0, 0) break;
        case 3: HBIN("sbb", 0, 0, 0, 0) break;
        case 4: HBIN("and", 0, 0, 0, 0) break;
        case 5: HBIN("or", 0, 0, 0, 0) break;
        case 6: HBIN("xor", 0, 0, 0, 0) break;
        case 7: HBIN("cmp", 0, 0, 0, 0) break;
        default: HBIN("test", 0, 0, 0, 0) break;
        }
#undef HBIN
        o.res = x;
        o.flags = fl & HOST_FLAG_BITS;
        return o;
    }

    if (op <= 12) {
#define HUN(MN)                                                                               \
    switch (w) {                                                                              \
    case 8:                                                                                   \
        __asm__ volatile(HINIT MN "b %b[a]\n\t" HSAVE                                         \
                         : [a] "+r"(x), [f] "=&r"(fl) : [i] "r"(init) : "cc");               \
        break;                                                                                \
    case 16:                                                                                  \
        __asm__ volatile(HINIT MN "w %w[a]\n\t" HSAVE                                         \
                         : [a] "+r"(x), [f] "=&r"(fl) : [i] "r"(init) : "cc");               \
        break;                                                                                \
    case 32:                                                                                  \
        __asm__ volatile(HINIT MN "l %k[a]\n\t" HSAVE                                         \
                         : [a] "+r"(x), [f] "=&r"(fl) : [i] "r"(init) : "cc");               \
        break;                                                                                \
    default:                                                                                  \
        __asm__ volatile(HINIT MN "q %q[a]\n\t" HSAVE                                         \
                         : [a] "+r"(x), [f] "=&r"(fl) : [i] "r"(init) : "cc");               \
        break;                                                                                \
    }
        switch (op) {
        case 9: HUN("inc") break;
        case 10: HUN("dec") break;
        case 11: HUN("neg") break;
        default: HUN("not") break;
        }
#undef HUN
        o.res = x;
        o.flags = fl & HOST_FLAG_BITS;
        return o;
    }

    /* 13 shl, 14 shr, 15 sar: count in CL. */
#define HSH(MN)                                                                               \
    switch (w) {                                                                              \
    case 8:                                                                                   \
        __asm__ volatile(HINIT MN "b %%cl, %b[a]\n\t" HSAVE                                   \
                         : [a] "+r"(x), [f] "=&r"(fl) : "c"(c8), [i] "r"(init) : "cc");      \
        break;                                                                                \
    case 16:                                                                                  \
        __asm__ volatile(HINIT MN "w %%cl, %w[a]\n\t" HSAVE                                   \
                         : [a] "+r"(x), [f] "=&r"(fl) : "c"(c8), [i] "r"(init) : "cc");      \
        break;                                                                                \
    case 32:                                                                                  \
        __asm__ volatile(HINIT MN "l %%cl, %k[a]\n\t" HSAVE                                   \
                         : [a] "+r"(x), [f] "=&r"(fl) : "c"(c8), [i] "r"(init) : "cc");      \
        break;                                                                                \
    default:                                                                                  \
        __asm__ volatile(HINIT MN "q %%cl, %q[a]\n\t" HSAVE                                   \
                         : [a] "+r"(x), [f] "=&r"(fl) : "c"(c8), [i] "r"(init) : "cc");      \
        break;                                                                                \
    }
    switch (op) {
    case 13: HSH("shl") break;
    case 14: HSH("shr") break;
    default: HSH("sar") break;
    }
#undef HSH
    o.res = x;
    o.flags = fl & HOST_FLAG_BITS;
    return o;
}

#else

static host_out host_alu(unsigned op, unsigned w, uint64_t a, uint64_t b, unsigned cf_in, uint64_t cnt) {
    host_out o = {0, 0, 0};
    (void)op; (void)w; (void)a; (void)b; (void)cf_in; (void)cnt;
    return o;
}

#endif
