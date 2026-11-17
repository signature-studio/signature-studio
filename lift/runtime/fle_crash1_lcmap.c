/* Decision 0209 (crash1): the case table of LCMapStringW outside the range D. See fle_crash1_lcmap.h. */
#include <stddef.h>
#include "fle_crash1_lcmap.h"

struct fle_c1_run {
    uint32_t first;
    uint16_t count;
    uint8_t stride;
    int32_t up;
    int32_t lo;
};

static const struct fle_c1_run fle_c1_runs[] = {
#include "fle_crash1_case.inc"
};

#define FLE_C1_NRUNS (sizeof fle_c1_runs / sizeof fle_c1_runs[0])

unsigned fle_crash1_ucd_runs(void) { return (unsigned)FLE_C1_NRUNS; }

uint32_t fle_crash1_ucd_case(uint32_t cp, int upper)
{
    size_t lo = 0, hi = FLE_C1_NRUNS, i, back;

    if (cp > 0x10FFFFu || (cp >= 0xD800u && cp <= 0xDFFFu)) return cp;
    /* the last run whose first code point is <= cp */
    while (lo < hi) {
        size_t mid = lo + (hi - lo) / 2;
        if (fle_c1_runs[mid].first <= cp) lo = mid + 1;
        else hi = mid;
    }
    /* runs of stride 2 interleave with a neighbour that starts one code point later, so a few runs before the found one can still hold cp */
    for (i = lo, back = 0; i > 0 && back < 8; back++) {
        const struct fle_c1_run *r = &fle_c1_runs[--i];
        uint32_t d = cp - r->first;
        if (d < (uint32_t)r->count * r->stride && d % r->stride == 0) {
            int32_t delta = upper ? r->up : r->lo;
            return (uint32_t)((int32_t)cp + delta);
        }
    }
    return cp;
}

uint32_t fle_crash1_case(uint32_t cp, int upper, int linguistic)
{
    uint32_t m = fle_crash1_ucd_case(cp, upper);

    if (!linguistic && m != cp && fle_crash1_ucd_case(m, !upper) != cp) m = cp;
    return m;
}
