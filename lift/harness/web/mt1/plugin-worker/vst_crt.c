/* The C runtime answers of the VST3 harness (#1338, Vital): the UCRT and vcruntime imports that a static-CRT module calls in its start-up (_initterm, the onexit
 * tables, the narrow environment) and the heap, memory and string functions of its first block. The convention of vst3a_imp.c: arguments in R[1], R[2], R[8], R[9]
 * (the fifth and later on the stack, vst3a_arg), the result in R[0], the guest return address stays on the stack. A function that is not defined here is the weak
 * stopping stub of vst3a_stubs.c (a named stop when it is called). Guest memory is read and written with fle_ld / fle_st. */
#include <errno.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#include "fle_callback.h"
#include "fle_imports.h"
#include "fle_sse.h"
#include "vst3a.h"

static void *P(uint64_t a) { return (void *)(uintptr_t)a; }

/* the onexit tables (_initialize_onexit_table, _register_onexit_function, _execute_onexit_table): keyed by the table's address; key 0 is _crt_atexit's table.
 * A table grows as the static initializers register their destructors (thousands in a large module). */
#define ONEXIT_TABLES 16
typedef struct { uint64_t key; unsigned n, cap; uint64_t *fn; } onexit_t;
static onexit_t onexit_tabs[ONEXIT_TABLES];
static unsigned onexit_used;

static onexit_t *onexit_of(uint64_t key, int create)
{
    for (unsigned i = 0; i < onexit_used; i++)
        if (onexit_tabs[i].key == key) return &onexit_tabs[i];
    if (!create) return NULL;
    if (onexit_used == ONEXIT_TABLES) vst3a_stop("onexit", "more onexit tables than the harness keeps");
    onexit_tabs[onexit_used].key = key;
    onexit_tabs[onexit_used].n = 0;
    onexit_tabs[onexit_used].cap = 0;
    onexit_tabs[onexit_used].fn = NULL;
    return &onexit_tabs[onexit_used++];
}

static void onexit_add(uint64_t key, uint64_t fn)
{
    onexit_t *t = onexit_of(key, 1);
    if (t->n == t->cap) {
        unsigned cap = t->cap ? t->cap * 2 : 64;
        uint64_t *p = realloc(t->fn, cap * sizeof *p);
        if (!p) vst3a_stop("onexit", "no memory for the onexit table");
        t->fn = p;
        t->cap = cap;
    }
    t->fn[t->n++] = fn;
}

static void onexit_run(uint64_t key)
{
    onexit_t *t = onexit_of(key, 0);
    if (!t) return;
    while (t->n > 0) {
        uint64_t fn = t->fn[--t->n];
        fle_call_guest(fn, 0, 0, 0, 0);
    }
}

void fle_imp__initialize_onexit_table(void)
{
    vst3a_note("_initialize_onexit_table");
    if (!R[1]) { R[0] = (uint64_t)-1; return; }
    onexit_of(R[1], 1)->n = 0;
    R[0] = 0;
}

void fle_imp__register_onexit_function(void)
{
    vst3a_note("_register_onexit_function");
    if (!R[1] || !R[2]) { R[0] = (uint64_t)-1; return; }
    onexit_add(R[1], R[2]);
    R[0] = 0;
}

void fle_imp__execute_onexit_table(void)
{
    vst3a_note("_execute_onexit_table");
    if (!R[1]) { R[0] = (uint64_t)-1; return; }
    onexit_run(R[1]);
    R[0] = 0;
}

void fle_imp__crt_atexit(void)
{
    vst3a_note("_crt_atexit");
    if (!R[1]) { R[0] = (uint64_t)-1; return; }
    onexit_add(0, R[1]);
    R[0] = 0;
}

/* _cexit: the process does not end here (the harness ends it); the atexit functions are the harness's to run at the end, so nothing runs now */
void fle_imp__cexit(void)
{
    vst3a_note("_cexit");
    R[0] = 0;
}

/* _initterm(first, last): every non-null _PVFV in [first, last) is called, in order */
void fle_imp__initterm(void)
{
    uint64_t first = R[1], last = R[2];
    vst3a_note("_initterm");
    for (uint64_t p = first; p < last; p += 8) {
        uint64_t f = fle_ld(p, 8);
        if (f) fle_call_guest(f, 0, 0, 0, 0);
    }
    R[0] = 0;
}

/* _initterm_e(first, last): as _initterm; a _PIFV that returns non-zero ends the walk, and that value is the result */
void fle_imp__initterm_e(void)
{
    uint64_t first = R[1], last = R[2];
    vst3a_note("_initterm_e");
    R[0] = 0;
    for (uint64_t p = first; p < last; p += 8) {
        uint64_t f = fle_ld(p, 8);
        if (!f) continue;
        uint64_t r = fle_call_guest(f, 0, 0, 0, 0);
        if ((uint32_t)r != 0) { R[0] = r; return; }
    }
}

void fle_imp__configure_narrow_argv(void)
{
    vst3a_note("_configure_narrow_argv");
    R[0] = 0;
}

void fle_imp__initialize_narrow_environment(void)
{
    vst3a_note("_initialize_narrow_environment");
    R[0] = 0;
}

void fle_imp__callnewh(void)
{
    vst3a_note("_callnewh");
    R[0] = 0;
}

void fle_imp__fpreset(void)
{
    vst3a_note("_fpreset");
}

static int errno_slot;

void fle_imp__errno(void)
{
    vst3a_note("_errno");
    R[0] = (uint64_t)(uintptr_t)&errno_slot;
}

/* the heap: the host's malloc. A guest pointer is a host address (the identity map), so the block is used as it is */
void fle_imp_malloc(void)
{
    vst3a_note("malloc");
    R[0] = (uint64_t)(uintptr_t)malloc(R[1] ? R[1] : 1);
}

void fle_imp_calloc(void)
{
    vst3a_note("calloc");
    R[0] = (uint64_t)(uintptr_t)calloc(R[1] ? R[1] : 1, R[2] ? R[2] : 1);
}

void fle_imp_realloc(void)
{
    vst3a_note("realloc");
    R[0] = (uint64_t)(uintptr_t)realloc(P(R[1]), R[2] ? R[2] : 1);
}

void fle_imp_free(void)
{
    vst3a_note("free");
    free(P(R[1]));
}

void fle_imp__aligned_malloc(void)
{
    void *p = NULL;
    vst3a_note("_aligned_malloc");
    uint64_t al = R[2] < sizeof(void *) ? sizeof(void *) : R[2];
    if (posix_memalign(&p, al, R[1] ? R[1] : 1) != 0) p = NULL;
    R[0] = (uint64_t)(uintptr_t)p;
}

void fle_imp__aligned_free(void)
{
    vst3a_note("_aligned_free");
    free(P(R[1]));
}

void fle_imp_memcpy(void)
{
    vst3a_note("memcpy");
    if (R[8]) memcpy(P(R[1]), P(R[2]), R[8]);
    R[0] = R[1];
}

void fle_imp_memmove(void)
{
    vst3a_note("memmove");
    if (R[8]) memmove(P(R[1]), P(R[2]), R[8]);
    R[0] = R[1];
}

void fle_imp_memset(void)
{
    vst3a_note("memset");
    if (R[8]) memset(P(R[1]), (int)(R[2] & 0xff), R[8]);
    R[0] = R[1];
}

void fle_imp_memcmp(void)
{
    vst3a_note("memcmp");
    R[0] = (uint64_t)(int64_t)(R[8] ? memcmp(P(R[1]), P(R[2]), R[8]) : 0);
}

void fle_imp_memchr(void)
{
    vst3a_note("memchr");
    R[0] = R[8] ? (uint64_t)(uintptr_t)memchr(P(R[1]), (int)(R[2] & 0xff), R[8]) : 0;
}

void fle_imp_strlen(void)
{
    vst3a_note("strlen");
    R[0] = (uint64_t)strlen(P(R[1]));
}

void fle_imp_strcmp(void)
{
    vst3a_note("strcmp");
    R[0] = (uint64_t)(int64_t)strcmp(P(R[1]), P(R[2]));
}

void fle_imp_strncmp(void)
{
    vst3a_note("strncmp");
    R[0] = (uint64_t)(int64_t)strncmp(P(R[1]), P(R[2]), R[8]);
}

void fle_imp_strchr(void)
{
    vst3a_note("strchr");
    R[0] = (uint64_t)(uintptr_t)strchr(P(R[1]), (int)(R[2] & 0xff));
}

void fle_imp_atoi(void)
{
    vst3a_note("atoi");
    R[0] = (uint64_t)(int64_t)atoi(P(R[1]));
}

void fle_imp__atoi64(void)
{
    vst3a_note("_atoi64");
    R[0] = (uint64_t)strtoll(P(R[1]), NULL, 10);
}

/* strtoll / strtoull (s, char **end, base): the guest's *end is written when end is not NULL */
void fle_imp_strtoll(void)
{
    char *e = NULL;
    vst3a_note("strtoll");
    R[0] = (uint64_t)strtoll(P(R[1]), &e, (int)R[8]);
    if (R[2]) fle_st(R[2], 8, (uint64_t)(uintptr_t)e);
}

void fle_imp_strtoull(void)
{
    char *e = NULL;
    vst3a_note("strtoull");
    R[0] = (uint64_t)strtoull(P(R[1]), &e, (int)R[8]);
    if (R[2]) fle_st(R[2], 8, (uint64_t)(uintptr_t)e);
}

/* the math functions: the float and double arguments are in XMM0 and XMM1 (the position of the argument decides the register: an int argument in the
 * second place is in R[2], as in ldexp and frexp), the result in XMM0 */
#include <math.h>
#define MATH_D1(name, expr) void fle_imp_##name(void) { vst3a_note(#name); X[0].f64[0] = (expr)(X[0].f64[0]); }
#define MATH_F1(name, expr) void fle_imp_##name(void) { vst3a_note(#name); X[0].f32[0] = (expr)(X[0].f32[0]); }
#define MATH_D2(name, expr) void fle_imp_##name(void) { vst3a_note(#name); X[0].f64[0] = expr(X[0].f64[0], X[1].f64[0]); }
#define MATH_F2(name, expr) void fle_imp_##name(void) { vst3a_note(#name); X[0].f32[0] = expr(X[0].f32[0], X[1].f32[0]); }
MATH_D1(sin, sin)
MATH_D1(cos, cos)
MATH_D1(tan, tan)
MATH_D1(acos, acos)
MATH_D1(exp, exp)
MATH_D1(log, log)
MATH_D1(round, round)
MATH_D2(pow, pow)
MATH_D2(fmod, fmod)
MATH_D2(atan2, atan2)
MATH_F1(sinf, sinf)
MATH_F1(cosf, cosf)
MATH_F1(tanf, tanf)
MATH_F1(asinf, asinf)
MATH_F1(atanf, atanf)
MATH_F1(expf, expf)
MATH_F1(logf, logf)
MATH_F1(log10f, log10f)
MATH_F1(log2f, log2f)
MATH_F1(roundf, roundf)
MATH_F2(powf, powf)
MATH_F2(atan2f, atan2f)
MATH_F2(fmaxf, fmaxf)
MATH_F2(fminf, fminf)
MATH_F2(copysignf, copysignf)

void fle_imp_ldexp(void)
{
    vst3a_note("ldexp");
    X[0].f64[0] = ldexp(X[0].f64[0], (int)(uint32_t)R[2]);
}

void fle_imp_frexp(void)
{
    int e = 0;
    vst3a_note("frexp");
    X[0].f64[0] = frexp(X[0].f64[0], &e);
    if (R[2]) fle_st(R[2], 4, (uint32_t)e);
}

/* __stdio_common_vsprintf / _vsprintf_s (options, buffer, count, format, locale, arglist): the format is walked here, and each conversion is printed by the
 * host's snprintf with its argument. The arguments are 8-byte slots from arglist (Win64 va_list): an integer is the low 4 or 8 bytes of its slot (8 with ll, I64,
 * j, z, t, I), a double is the slot's bits. A conversion that is not listed (n, wide strings, long double) stops by name. */
static uint64_t va_next(uint64_t *ap)
{
    uint64_t v = fle_ld(*ap, 8);
    *ap += 8;
    return v;
}

/* wide (nonzero) is the wide-character format of __stdio_common_vswprintf: %s is then a wide string and %S a narrow one (as in the Microsoft printf family) */
static size_t guest_format(char *out, size_t cap, const char *fmt, uint64_t ap, int wide)
{
    size_t n = 0;
    for (const char *p = fmt; *p;) {
        if (*p != '%') {
            if (n + 1 < cap) out[n] = *p;
            n++;
            p++;
            continue;
        }
        p++;
        if (*p == '%') {
            if (n + 1 < cap) out[n] = '%';
            n++;
            p++;
            continue;
        }
        char spec[64];
        size_t k = 0;
        spec[k++] = '%';
        while (*p && strchr("-+ #0", *p)) { if (k < 16) spec[k++] = *p; p++; }
        if (*p == '*') { k += (size_t)snprintf(spec + k, sizeof spec - k, "%d", (int)va_next(&ap)); p++; }
        else while (*p >= '0' && *p <= '9') { if (k < 16) spec[k++] = *p; p++; }
        if (*p == '.') {
            spec[k++] = '.';
            p++;
            if (*p == '*') { k += (size_t)snprintf(spec + k, sizeof spec - k, "%d", (int)va_next(&ap)); p++; }
            else while (*p >= '0' && *p <= '9') { if (k < 16) spec[k++] = *p; p++; }
        }
        int lng = 0, i64 = 0;
        const char *len = "";
        if (!strncmp(p, "I64", 3) || !strncmp(p, "ll", 2)) { i64 = 1; len = "ll"; p += strncmp(p, "I64", 3) ? 2 : 3; }
        else if (!strncmp(p, "hh", 2)) { len = "hh"; p += 2; }
        else if (*p == 'h') { len = "h"; p++; }
        else if (*p == 'j' || *p == 'z' || *p == 't' || *p == 'Z') { i64 = 1; len = "ll"; p++; }
        else if (*p == 'I') { i64 = 1; len = "ll"; p++; }
        else if (*p == 'l') { lng = 1; p++; }
        else if (*p == 'L') vst3a_stop("__stdio_common_vsprintf", "long double conversion is not answered");
        char conv = *p ? *p++ : '\0';
        char host[80];
        int hn = 0;
        union { uint64_t u; double d; } slot;
        char tmp[64];
        int tn = 0;
        char *text = NULL;
        switch (conv) {
        case 'd': case 'i': case 'u': case 'x': case 'X': case 'o': case 'c': {
            uint64_t v = va_next(&ap);
            int sgn = conv == 'd' || conv == 'i';
            if (conv == 'c') { hn = snprintf(host, sizeof host, "%sc", spec); tn = snprintf(tmp, sizeof tmp, host, (int)(v & 0xff)); }
            else if (i64) { hn = snprintf(host, sizeof host, "%sll%c", spec, conv); tn = sgn ? snprintf(tmp, sizeof tmp, host, (long long)v) : snprintf(tmp, sizeof tmp, host, (unsigned long long)v); }
            else { hn = snprintf(host, sizeof host, "%s%s%c", spec, len, conv); tn = sgn ? snprintf(tmp, sizeof tmp, host, (int)(uint32_t)v) : snprintf(tmp, sizeof tmp, host, (unsigned)(uint32_t)v); }
            (void)hn;
            text = tmp;
            break;
        }
        case 's': case 'S': {
            uint64_t v = va_next(&ap);
            int wstr = conv == 's' ? (wide || lng) : !wide;
            char *narrow = NULL;
            const char *s = "(null)";
            if (v && wstr) {
                size_t units = 0;
                while (fle_ld(v + 2 * units, 2)) units++;
                narrow = malloc(units + 1);
                for (size_t i = 0; i < units; i++) {
                    uint64_t u = fle_ld(v + 2 * i, 2);
                    narrow[i] = u < 128 ? (char)u : '?';
                }
                narrow[units] = '\0';
                s = narrow;
            } else if (v) s = (const char *)(uintptr_t)v;
            hn = snprintf(host, sizeof host, "%ss", spec);
            tn = snprintf(NULL, 0, host, s);
            text = malloc((size_t)tn + 1);
            snprintf(text, (size_t)tn + 1, host, s);
            free(narrow);
            break;
        }
        case 'p': {
            uint64_t v = va_next(&ap);
            tn = snprintf(tmp, sizeof tmp, "%llx", (unsigned long long)v);
            text = tmp;
            break;
        }
        case 'f': case 'F': case 'e': case 'E': case 'g': case 'G': case 'a': case 'A': {
            slot.u = va_next(&ap);
            hn = snprintf(host, sizeof host, "%s%c", spec, conv);
            tn = snprintf(NULL, 0, host, slot.d);
            text = malloc((size_t)tn + 1);
            snprintf(text, (size_t)tn + 1, host, slot.d);
            break;
        }
        default:
            vst3a_stop("__stdio_common_vsprintf", "a conversion that is not answered");
        }
        for (int i = 0; i < tn; i++) {
            if (n + 1 < cap) out[n] = text[i];
            n++;
        }
        if (text != tmp) free(text);
    }
    if (cap) out[n < cap ? n : cap - 1] = '\0';
    return n;
}

void fle_imp___stdio_common_vsprintf(void)
{
    vst3a_note("__stdio_common_vsprintf");
    size_t cap = R[8] == UINT64_MAX ? (size_t)-1 : (size_t)R[8];
    char *buf = P(R[2]);
    size_t n = guest_format(buf, cap, (const char *)P(R[9]), vst3a_arg(6), 0);
    R[0] = (uint64_t)(int64_t)n;
}

void fle_imp___stdio_common_vsprintf_s(void)
{
    vst3a_note("__stdio_common_vsprintf_s");
    size_t cap = (size_t)R[8];
    char *buf = P(R[2]);
    size_t n = guest_format(buf, cap, (const char *)P(R[9]), vst3a_arg(6), 0);
    R[0] = n < cap ? (uint64_t)(int64_t)n : (uint64_t)(int64_t)-1;
}

/* __stdio_common_vsscanf (options, string, count, format, locale, arglist): every conversion takes one pointer slot of arglist; the host's sscanf writes each
 * result through it, by the conversion's own type. Up to 16 conversions (more stop by name). */
void fle_imp___stdio_common_vsscanf(void)
{
    const char *s = P(R[2]);
    const char *fmt = P(R[9]);
    uint64_t ap = vst3a_arg(6);
    void *a[16] = { 0 };
    int nc = 0;
    vst3a_note("__stdio_common_vsscanf");
    for (const char *p = fmt; *p; p++) {
        if (*p != '%') continue;
        if (p[1] == '%') { p++; continue; }
        if (p[1] == '*') continue;
        if (nc == 16) vst3a_stop("__stdio_common_vsscanf", "more than 16 conversions");
        nc++;
    }
    for (int i = 0; i < nc; i++) a[i] = P(va_next(&ap));
    R[0] = (uint64_t)(int64_t)sscanf(s, fmt, a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7], a[8], a[9], a[10], a[11], a[12], a[13], a[14], a[15]);
}

/* rand / srand: the UCRT's generator (seed 1 at start, 15-bit results), so that a run gives the same numbers every time */
static unsigned crt_seed = 1;

void fle_imp_rand(void)
{
    vst3a_note("rand");
    crt_seed = crt_seed * 214013u + 2531011u;
    R[0] = (crt_seed >> 16) & 0x7fffu;
}

void fle_imp_srand(void)
{
    vst3a_note("srand");
    crt_seed = (unsigned)R[1];
}

/* __RTDynamicCast (inptr, VfDelta, SrcType, TargetType, isReference): dynamic_cast by the MSVC x64 RTTI of the image. The vfptr is at inptr + VfDelta; vfptr[-1] is the
 * _RTTICompleteObjectLocator (offset, pTypeDescriptor, pClassDescriptor are RVAs from the image base VST_RM_BASE); the complete object is the vfptr's place less the
 * locator's offset. The target is the complete type or one of its base classes (a non-virtual base, at mdisp). A virtual base stops by name. Not found: NULL, or a
 * bad_cast stop for a reference. Types are TypeDescriptors: the name is at offset 16 (pVFTable and spare come first). */
static int rtti_same(uint64_t a, uint64_t b)
{
    return a == b || !strcmp((const char *)P(a + 16), (const char *)P(b + 16));
}

void fle_imp___RTDynamicCast(void)
{
    uint64_t inptr = R[1];
    uint64_t vfl = inptr + (uint64_t)(int64_t)(int32_t)R[2];
    uint64_t src = R[8], dst = R[9];
    int isref = (int)(uint32_t)vst3a_arg(5);
    uint64_t col, chd, arr, complete, result = 0;
    uint32_t nb;
    vst3a_note("__RTDynamicCast");
    (void)src;
    col = fle_ld(fle_ld(vfl, 8) - 8, 8);
    complete = vfl - (uint64_t)fle_ld(col + 4, 4);
    chd = VST_RM_BASE + fle_ld(col + 16, 4);
    nb = (uint32_t)fle_ld(chd + 8, 4);
    arr = VST_RM_BASE + fle_ld(chd + 12, 4);
    if (rtti_same(VST_RM_BASE + fle_ld(col + 12, 4), dst)) result = complete;
    for (uint32_t i = 0; i < nb && !result; i++) {
        uint64_t bcd = VST_RM_BASE + fle_ld(arr + 4u * i, 4);
        if (!rtti_same(VST_RM_BASE + fle_ld(bcd, 4), dst)) continue;
        if ((int32_t)fle_ld(bcd + 12, 4) != -1) vst3a_stop("__RTDynamicCast", "a virtual base class is not answered");
        result = complete + (uint64_t)(int64_t)(int32_t)fle_ld(bcd + 8, 4);
    }
    if (!result && isref) vst3a_stop("__RTDynamicCast", "bad_cast for a reference");
    R[0] = result;
}

/* the wide character classes and case mapping (wint_t in the first argument register): the host's <wctype.h> in its C locale */
#include <wctype.h>
#define WCT_CLASS(name, expr) void fle_imp_##name(void) { vst3a_note(#name); R[0] = (expr) ? 1 : 0; }
#define WCT_MAP(name, expr) void fle_imp_##name(void) { vst3a_note(#name); R[0] = (uint64_t)(uint32_t)expr((wint_t)(uint32_t)R[1]); }
WCT_CLASS(iswalnum, iswalnum((wint_t)(uint32_t)R[1]))
WCT_CLASS(iswalpha, iswalpha((wint_t)(uint32_t)R[1]))
WCT_CLASS(iswdigit, iswdigit((wint_t)(uint32_t)R[1]))
WCT_CLASS(iswlower, iswlower((wint_t)(uint32_t)R[1]))
WCT_CLASS(iswspace, iswspace((wint_t)(uint32_t)R[1]))
WCT_CLASS(iswupper, iswupper((wint_t)(uint32_t)R[1]))
WCT_MAP(towlower, towlower)
WCT_MAP(towupper, towupper)

/* _ftime64_s (struct __timeb64 *): the clock is not modelled here; the answer is the epoch (time 0, millitm 0, timezone 0, dstflag 0), so that a run gives the same numbers */
void fle_imp__ftime64_s(void)
{
    vst3a_note("_ftime64_s");
    if (!R[1]) { R[0] = 22; return; }
    fle_st(R[1], 8, 0);
    fle_st(R[1] + 8, 2, 0);
    fle_st(R[1] + 10, 2, 0);
    fle_st(R[1] + 12, 2, 0);
    R[0] = 0;
}

/* the calendar, in UTC by arithmetic (the host's libc time functions need _tzset_js, which the plugin host does not give): a guest struct tm is nine ints
 * (tm_sec .. tm_isdst, 36 bytes) */
static int64_t days_from_civil(int64_t y, int64_t m, int64_t d)
{
    y -= m <= 2;
    int64_t era = (y >= 0 ? y : y - 399) / 400;
    int64_t yoe = y - era * 400;
    int64_t doy = (153 * (m + (m > 2 ? -3 : 9)) + 2) / 5 + d - 1;
    int64_t doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    return era * 146097 + doe - 719468;
}

static void civil_from_days(int64_t z, int64_t *y, int64_t *m, int64_t *d)
{
    z += 719468;
    int64_t era = (z >= 0 ? z : z - 146096) / 146097;
    int64_t doe = z - era * 146097;
    int64_t yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    int64_t doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    int64_t mp = (5 * doy + 2) / 153;
    *d = doy - (153 * mp + 2) / 5 + 1;
    *m = mp < 10 ? mp + 3 : mp - 9;
    *y = yoe + era * 400 + (*m <= 2);
}

static void tm_store(uint64_t dst, int64_t t)
{
    int64_t days = t >= 0 ? t / 86400 : -((-t + 86399) / 86400);
    int64_t rem = t - days * 86400;
    int64_t y, m, d;
    civil_from_days(days, &y, &m, &d);
    int64_t wday = ((days + 4) % 7 + 7) % 7;
    int64_t yday = days - days_from_civil(y, 1, 1);
    int f[9] = { (int)(rem % 60), (int)(rem / 60 % 60), (int)(rem / 3600), (int)d, (int)(m - 1), (int)(y - 1900), (int)wday, (int)yday, 0 };
    for (int i = 0; i < 9; i++) fle_st(dst + 4u * i, 4, (uint32_t)f[i]);
}

/* _gmtime64_s / _localtime64_s (struct tm *, const __time64_t *): EINVAL (22) for a NULL argument; both are UTC (the host's zone is UTC) */
static uint64_t time_to_tm(void)
{
    if (!R[1] || !R[2]) return 22;
    tm_store(R[1], (int64_t)fle_ld(R[2], 8));
    return 0;
}

void fle_imp__gmtime64_s(void)
{
    vst3a_note("_gmtime64_s");
    R[0] = time_to_tm();
}

void fle_imp__localtime64_s(void)
{
    vst3a_note("_localtime64_s");
    R[0] = time_to_tm();
}

/* _mktime64 (struct tm *): the time of the fields (months and days out of range are carried), the fields are written back normalized */
void fle_imp__mktime64(void)
{
    vst3a_note("_mktime64");
    int64_t sec = (int32_t)fle_ld(R[1], 4), min = (int32_t)fle_ld(R[1] + 4, 4), hour = (int32_t)fle_ld(R[1] + 8, 4);
    int64_t mday = (int32_t)fle_ld(R[1] + 12, 4), mon = (int32_t)fle_ld(R[1] + 16, 4), year = (int32_t)fle_ld(R[1] + 20, 4) + 1900;
    int64_t y = year + (mon >= 0 ? mon / 12 : -((-mon + 11) / 12)), mm = ((mon % 12) + 12) % 12;
    int64_t t = (days_from_civil(y, mm + 1, 1) + mday - 1) * 86400 + hour * 3600 + min * 60 + sec;
    tm_store(R[1], t);
    R[0] = (uint64_t)t;
}

/* __stdio_common_vswprintf (options, wchar_t *buffer, count, const wchar_t *format, locale, arglist): the wide format is read as ASCII (a unit above 127 is
 * '?'), the output is formatted narrow and widened into the buffer. A count of -1 is no limit; the result is the number of characters. */
void fle_imp___stdio_common_vswprintf(void)
{
    uint64_t f = R[9], ap = vst3a_arg(6);
    size_t fl = 0, n, cap = R[8] == UINT64_MAX ? (size_t)-1 : (size_t)R[8];
    uint16_t *out = (uint16_t *)P(R[2]);
    char *nf, *nb;
    vst3a_note("__stdio_common_vswprintf");
    while (fle_ld(f + 2 * fl, 2)) fl++;
    nf = malloc(fl + 1);
    for (size_t i = 0; i < fl; i++) {
        uint64_t u = fle_ld(f + 2 * i, 2);
        nf[i] = u < 128 ? (char)u : '?';
    }
    nf[fl] = '\0';
    n = guest_format(NULL, 0, nf, ap, 1);
    nb = malloc(n + 1);
    guest_format(nb, n + 1, nf, ap, 1);
    if (cap) {
        for (size_t i = 0; i < n && i + 1 < cap; i++) out[i] = (uint8_t)nb[i];
        out[n < cap ? n : cap - 1] = 0;
    }
    R[0] = (uint64_t)(int64_t)n;
    free(nf);
    free(nb);
}

/* localeconv: the C locale. struct lconv starts with ten char* fields (decimal_point ".", the others ""), then eight char fields that the C locale sets to CHAR_MAX
 * (the layout of the UCRT's lconv; the wide fields after them are NULL). */
void fle_imp_localeconv(void)
{
    static uint64_t lconv[32];
    static char dot[] = ".", empty[] = "", *chars = (char *)&lconv[10];
    vst3a_note("localeconv");
    if (!lconv[0]) {
        lconv[0] = (uint64_t)(uintptr_t)dot;
        for (int i = 1; i < 10; i++) lconv[i] = (uint64_t)(uintptr_t)empty;
        for (int i = 0; i < 8; i++) chars[i] = 127;
    }
    R[0] = (uint64_t)(uintptr_t)lconv;
}

/* kernel32!ReplaceFileW (replaced rcx, replacement rdx, backup r8, flags r9, ...): the replaced file is removed and the replacement is moved in its place (the
 * runtime's file system: DeleteFileW, then MoveFileW). A backup, and the flags, are not answered: a backup stops by name. */
void fle_imp_DeleteFileW(void);
void fle_imp_MoveFileW(void);
void fle_imp_ReplaceFileW(void)
{
    uint64_t replaced = R[1], replacement = R[2], backup = R[8];
    vst3a_note("ReplaceFileW");
    if (backup) vst3a_stop("ReplaceFileW", "a backup file is not answered");
    R[1] = replaced;
    fle_imp_DeleteFileW();
    R[1] = replacement;
    R[2] = replaced;
    fle_imp_MoveFileW();
}

/* the std::exception data of the library (__std_exception_data { char *_What; bool _DoFree; }): a copy owns its text (_DoFree), a destroy frees an owned text */
void fle_imp___std_exception_copy(void)
{
    uint64_t src = R[1], dst = R[2];
    char *what;
    vst3a_note("__std_exception_copy");
    if (!src || !dst) { R[0] = 22; return; }
    what = strdup(P(fle_ld(src, 8)));
    if (!what) { R[0] = 12; return; }
    fle_st(dst, 8, (uint64_t)(uintptr_t)what);
    fle_st(dst + 8, 1, 1);
    R[0] = 0;
}

void fle_imp___std_exception_destroy(void)
{
    uint64_t d = R[1];
    vst3a_note("__std_exception_destroy");
    if (!d) return;
    if (fle_ld(d + 8, 1)) free(P(fle_ld(d, 8)));
    fle_st(d, 8, 0);
    fle_st(d + 8, 1, 0);
}

/* _CxxThrowException (object, ThrowInfo): a C++ exception has no unwinding in this runtime, so the throw stops, and the stop names the type that was thrown:
 * ThrowInfo's pCatchableTypeArray (RVA, at +12) gives the first CatchableType (RVA at +4 of the array), whose pType (RVA at +4) is the TypeDescriptor (name at +16). */
void fle_imp__CxxThrowException(void)
{
    uint64_t ti = R[2], arr, ct, td;
    char why[256];
    vst3a_note("_CxxThrowException");
    if (!ti) vst3a_stop("_CxxThrowException", "a rethrow (no ThrowInfo) is not answered");
    arr = VST_RM_BASE + fle_ld(ti + 12, 4);
    ct = VST_RM_BASE + fle_ld(arr + 4, 4);
    td = VST_RM_BASE + fle_ld(ct + 4, 4);
    snprintf(why, sizeof why, "the guest threw %s (no unwinding is modelled)", (const char *)P(td + 16));
    vst3a_stop("_CxxThrowException", why);
}
