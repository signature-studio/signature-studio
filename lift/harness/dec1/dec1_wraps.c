/* Decision 0206 (dec1, opt-in, harness only): argument traces of imports that the sample decoders reached (env FLE_DEC1_TRACE=1), linked with -Wl,--wrap=fle_imp_<API>. Prints to stderr, then calls the runtime's function. */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include "fle_runtime.h"
#include "fle_env.h"
#include "fle_imports.h"

#define WRAP(NAME, FMT, ...) \
void __real_fle_imp_##NAME(void); \
void __wrap_fle_imp_##NAME(void) \
{ \
    if (getenv("FLE_DEC1_TRACE") != NULL) fprintf(stderr, "dec1: " #NAME " " FMT "\n", __VA_ARGS__); \
    __real_fle_imp_##NAME(); \
}
#define U(x) ((unsigned long long)(x))
WRAP(GetLocaleInfoW, "lcid=%#llx type=%#llx buf=%#llx cch=%lld", U(R[1]), U(R[2]), U(R[8]), (long long)(int32_t)R[9])
WRAP(GetLocaleInfoEx, "name=%#llx type=%#llx buf=%#llx cch=%lld", U(R[1]), U(R[2]), U(R[8]), (long long)(int32_t)R[9])
WRAP(LCMapStringW, "lcid=%#llx flags=%#llx cch=%lld", U(R[1]), U(R[2]), (long long)(int32_t)R[9])

/* ReadFile trace (env FLE_DEC1_TRACE=files): the call and its result. */
void __real_fle_imp_ReadFile(void);
void __wrap_fle_imp_ReadFile(void)
{
    uint64_t h = R[1], n = R[8], pn = R[9];
    const char *t = getenv("FLE_DEC1_TRACE");
    __real_fle_imp_ReadFile();
    if (t != NULL && t[0] == 'f') fprintf(stderr, "dec1: ReadFile h=%#llx want=%llu got=%llu ret=%llu\n", U(h), U(n), U(pn ? fle_ld(pn, 4) : 0), U(R[0]));
}
void __real_fle_imp_SetFilePointer(void);
void __wrap_fle_imp_SetFilePointer(void)
{
    uint64_t h = R[1], lo = R[2], phi = R[8], m = R[9];
    const char *t = getenv("FLE_DEC1_TRACE");
    __real_fle_imp_SetFilePointer();
    if (t != NULL && t[0] == 'f') fprintf(stderr, "dec1: SetFilePointer h=%#llx lo=%#llx hi*=%#llx method=%llu -> %#llx\n", U(h), U(lo), U(phi), U(m), U(R[0]));
}
