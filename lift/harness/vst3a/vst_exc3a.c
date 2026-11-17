/* Decision 0177 (srm1): a diagnostic wrapper of the runtime's RaiseException (fle_seh_raise, decision 0132). With VST3A_EXC_TRACE set it prints the code and, for an MSVC C++ exception (0xE06D7363),
 * the type names in the thrown object's ThrowInfo (x64: RVAs from the image base in the 4th argument). Without the variable it only forwards. Linked with --wrap=fle_seh_raise (wraps.txt). */
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include "fle_runtime.h"

void __real_fle_seh_raise(void) __attribute__((noreturn));

void __wrap_fle_seh_raise(void)
{
    static int on = -1;
    if (on < 0) on = getenv("VST3A_EXC_TRACE") != NULL;
    if (on) {
        uint64_t code = (uint32_t)R[1], nargs = (uint32_t)R[8], args = R[9];
        printf("vst3a: RaiseException code %#llx flags %#x nargs %llu\n", (unsigned long long)code, (unsigned)R[2], (unsigned long long)nargs);
        if (code == 0xE06D7363u && nargs >= 4 && args) {
            uint64_t base = fle_ld(args + 24, 8), ti = fle_ld(args + 16, 8);
            if (ti) {
                uint64_t cta = base + fle_ld(ti + 12, 4);
                uint32_t n = (uint32_t)fle_ld(cta, 4), i;
                for (i = 0; i < n && i < 6; i++) {
                    uint64_t ct = base + fle_ld(cta + 4 + 4 * i, 4), td = base + fle_ld(ct + 4, 4);
                    char name[96];
                    unsigned k;
                    for (k = 0; k < sizeof name - 1; k++) { name[k] = (char)fle_ld(td + 16 + k, 1); if (!name[k]) break; }
                    name[k] = 0;
                    printf("vst3a:   catchable type %u: %s\n", i, name);
                }
            }
        }
        fflush(stdout);
    }
    __real_fle_seh_raise();
}
