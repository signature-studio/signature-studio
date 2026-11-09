/* alpha1 (decision 0244): how a GPU blend can equal the compositor's integer formula (fle_desk.c over_premul). Exhaustive over c, a, d in 0..255 (c up to 255 even when above a: the guest must not make such a pixel, but the compositor clamps it)
 * and SourceConstantAlpha in {255, 128, 77}. The reference is the integer function copied from fle_desk.c; the candidates are float32 evaluations a GLSL ES 3.00 shader can do:
 *   F1  floor(c + d * (255 - a) / 255 + 0.5)             with the premultiplied source scaled first by sca/255 (the formula of the page, in the order the page writes it)
 *   F2  floor(c + d * (1.0 - a / 255.0) + 0.5)            the same with the complement computed as 1 - a/255 first
 *   F3  the fixed-function path: (src, a) written with blendFunc(ONE, ONE_MINUS_SRC_ALPHA) on normalised 8-bit values and each result rounded to nearest (a MODEL: the rounding of real hardware is implementation-defined and differs between drivers)
 * usage: gl-parity  -> prints a table. */
#include <stdio.h>
#include <stdint.h>
#include <math.h>
#include <stdlib.h>
static uint32_t ref(uint32_t c, uint32_t a, uint32_t d, uint32_t sca)
{
    uint32_t k = 65025u - a * sca, v = (c * sca * 255u + d * k + 32512u) / 65025u;
    return v > 255u ? 255u : v;
}
static uint32_t f1(uint32_t c, uint32_t a, uint32_t d, uint32_t sca)
{
    float cs = (float)c * (float)sca / 255.0f, as = (float)a * (float)sca / 255.0f;      /* Src * SCA / 255 in all four channels */
    float x = cs + (float)d * (255.0f - as) / 255.0f;
    float r = floorf(x + 0.5f);
    return r > 255.0f ? 255u : (uint32_t)r;
}
static uint32_t f2(uint32_t c, uint32_t a, uint32_t d, uint32_t sca)
{
    float cs = (float)c * (float)sca / 255.0f, as = (float)a * (float)sca / 255.0f;
    float x = cs + (float)d * (1.0f - as / 255.0f);
    float r = floorf(x + 0.5f);
    return r > 255.0f ? 255u : (uint32_t)r;
}
static uint32_t f3(uint32_t c, uint32_t a, uint32_t d, uint32_t sca)
{
    /* the source is first written as a normalised 8-bit value (Src * SCA / 255 rounded), then blended in normalised floats and rounded to nearest */
    float cs = roundf((float)c * (float)sca / 255.0f) / 255.0f, as = roundf((float)a * (float)sca / 255.0f) / 255.0f;
    float x = (cs + (1.0f - as) * ((float)d / 255.0f)) * 255.0f;
    float r = roundf(x);
    return r > 255.0f ? 255u : (uint32_t)r;
}
int main(void)
{
    static const uint32_t scas[] = { 255, 128, 77 };
    printf("%-6s %-4s %12s %12s %12s %12s %12s %12s\n", "SCA", "", "triplets", "F1 differs", "F1 max", "F2 differs", "F3 differs", "F3 max");
    for (unsigned s = 0; s < 3; s++) {
        uint64_t n = 0, d1 = 0, d2 = 0, d3 = 0;
        int m1 = 0, m3 = 0;
        for (uint32_t a = 0; a < 256; a++)
            for (uint32_t c = 0; c < 256; c++)
                for (uint32_t d = 0; d < 256; d++) {
                    uint32_t r = ref(c, a, d, scas[s]), x1 = f1(c, a, d, scas[s]), x2 = f2(c, a, d, scas[s]), x3 = f3(c, a, d, scas[s]);
                    int e1 = (int)x1 - (int)r, e3 = (int)x3 - (int)r;
                    n++;
                    if (e1) { d1++; if (abs(e1) > m1) m1 = abs(e1); }
                    if (x2 != r) d2++;
                    if (e3) { d3++; if (abs(e3) > m3) m3 = abs(e3); }
                }
        printf("%-6u %-4s %12llu %12llu %12d %12llu %12llu %12d\n", scas[s], "", (unsigned long long)n, (unsigned long long)d1, m1, (unsigned long long)d2, (unsigned long long)d3, m3);
    }
    /* the pixels that matter: premultiplied and valid (c <= a), SCA 255 */
    {
        uint64_t n = 0, d1 = 0, d3 = 0;
        for (uint32_t a = 0; a < 256; a++) for (uint32_t c = 0; c <= a; c++) for (uint32_t d = 0; d < 256; d++) { uint32_t r = ref(c, a, d, 255); n++; if (f1(c, a, d, 255) != r) d1++; if (f3(c, a, d, 255) != r) d3++; }
        printf("valid premultiplied (c <= a), SCA 255: %llu triplets, F1 differs %llu, F3 differs %llu\n", (unsigned long long)n, (unsigned long long)d1, (unsigned long long)d3);
    }
    return 0;
}
