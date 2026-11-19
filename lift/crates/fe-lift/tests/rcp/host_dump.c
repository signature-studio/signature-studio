/* Test-only host reference (decision 0047): the host's RCPPS result mantissa for each of the 4096
 * indices (the top 12 bits of the source mantissa), measured at source exponent 127 (value in
 * [1, 2)); output exponent is 126. One decimal number per line: the 12 mantissa bits
 * (result bits [22:11]; the low 11 bits are 0, checked). Input of tests/rcp/gen_rcp_table.py. */
#include <stdint.h>
#include <stdio.h>
#include <xmmintrin.h>

int main(void) {
    for (uint32_t i = 0; i < 4096; i++) {
        __m128 v = _mm_castsi128_ps(_mm_cvtsi32_si128((int)((127u << 23) | (i << 11))));
        uint32_t o = (uint32_t)_mm_cvtsi128_si32(_mm_castps_si128(_mm_rcp_ps(v)));
        if ((o >> 23) != 126 || (o & 0x7ff)) {
            fprintf(stderr, "index %u: unexpected host result %#x\n", i, o);
            return 1;
        }
        printf("%u\n", (o >> 11) & 0xfff);
    }
    return 0;
}
