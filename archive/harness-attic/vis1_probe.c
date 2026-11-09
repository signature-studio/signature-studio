/* Decision 0148 (PROVISIONAL): the API-call probe of the render-defect study. A harness-side observation: it wraps every runtime API (-Wl,--wrap=fle_imp_<name> for the names of vis1/vis1_apis.inc, flags in
 * vis1/link-flags.txt; no default build links it) and prints one "vis1-api:" line per call while a window is being painted: between user32!BeginPaint(hwnd) and the matching EndPaint, or in a clock window.
 *   VIS1_API_HWND=<hex>   the window whose paint is logged (the argument of BeginPaint, rcx)
 *   VIS1_API_SKIP=<n>     BeginPaint calls of that window to let pass before the first logged one (default 0)
 *   VIS1_API_COUNT=<n>    how many BeginPaint..EndPaint spans to log (default 1)
 *   VIS1_API_ARG1=<hex>   log every call whose first argument (rcx) is this value (a window or DC handle), whatever the thread and the time
 *   VIS1_API_NAMES=,A,B,   (with the commas) only these APIs are logged in the clock window
 *   VIS1_API_FROM / VIS1_API_TO=<virtual ms>   log every call of the thread VIS1_API_THREAD (default all threads) in the clock window (HWND is then not needed)
 * A line: the virtual ms, the host thread, the API name, rcx rdx r8 r9 on entry, the guest return address (the word at [rsp]) and rax after the call. The wrappers read R[] and change nothing the guest sees. */
#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fle_env.h"
#include "fle_runtime.h"
#include "fle_sys.h"
#include "fle_thread.h"

#define V1API(n) void __real_fle_imp_##n(void);
#include "vis1/vis1_apis.inc"
#undef V1API

static int inited;
static uint64_t want_hwnd;
static unsigned skip, count = 1, paints;
static int in_span;
static int64_t from_ms = -1, to_ms = -1;
static long only_thread = -1;
static uint64_t want_arg1;
static const char *only_names;     /* VIS1_API_NAMES=",Name1,Name2," (with the commas): log only these APIs in the clock window */

static void init(void)
{
    const char *a1e = getenv("VIS1_API_ARG1"), *h = getenv("VIS1_API_HWND"), *s = getenv("VIS1_API_SKIP"), *c = getenv("VIS1_API_COUNT"), *f = getenv("VIS1_API_FROM"), *t = getenv("VIS1_API_TO"), *th = getenv("VIS1_API_THREAD");

    inited = 1;
    only_names = getenv("VIS1_API_NAMES");
    if (a1e && a1e[0]) want_arg1 = strtoull(a1e, NULL, 0);
    if (h && h[0]) want_hwnd = strtoull(h, NULL, 0);
    if (s && s[0]) skip = (unsigned)atoi(s);
    if (c && c[0]) count = (unsigned)atoi(c);
    if (f && f[0]) from_ms = strtoll(f, NULL, 0);
    if (t && t[0]) to_ms = strtoll(t, NULL, 0);
    if (th && th[0]) only_thread = atol(th);
}

static int64_t now_ms(void) { return (int64_t)(fle_sys_clock_ticks() / FLE_TICKS_PER_MS); }

static int active(const char *name)
{
    int64_t ms;

    if (!inited) init();
    if (want_hwnd) {
        if (!strcmp(name, "BeginPaint") && R[1] == want_hwnd) {
            if (paints >= skip && paints < skip + count) in_span = 1;
            paints++;
        }
        return in_span;
    }
    if (want_arg1) return R[1] == want_arg1;
    if (from_ms < 0) return 0;
    if (only_names && only_names[0]) {
        char key[96];

        snprintf(key, sizeof key, ",%s,", name);
        if (!strstr(only_names, key)) return 0;
    }
    ms = now_ms();
    if (ms < from_ms || (to_ms >= 0 && ms > to_ms)) return 0;
    if (only_thread >= 0 && (long)fle_thread_current() != only_thread) return 0;
    return 1;
}

static void rect_str(char *o, size_t n, uint64_t p)
{
    if (!p) { snprintf(o, n, "NULL"); return; }
    snprintf(o, n, "(%d,%d,%d,%d)", (int)(int32_t)fle_ld(p, 4), (int)(int32_t)fle_ld(p + 4, 4), (int)(int32_t)fle_ld(p + 8, 4), (int)(int32_t)fle_ld(p + 12, 4));
}

static void wstr(char *o, size_t n, uint64_t p)
{
    size_t k = 0;

    if (!p) { snprintf(o, n, "NULL"); return; }
    if (p < 0x10000) { snprintf(o, n, "atom %#" PRIx64, p); return; }
    for (unsigned i = 0; i < 100 && k + 4 < n; i++) {
        uint32_t c = (uint32_t)fle_ld(p + 2u * i, 2);

        if (!c) break;
        if (c >= 0x20 && c < 0x7f) o[k++] = (char)c;
        else k += (size_t)snprintf(o + k, n - k, "\\u%04x", (unsigned)c);
    }
    o[k] = 0;
}

/* the arguments worth reading as numbers, per API (read after the call so that out-parameters are filled) */
static void detail(const char *name, uint64_t a1, uint64_t a2, uint64_t a8, uint64_t a9, uint64_t s5, uint64_t s6, uint64_t s7, uint64_t s8, uint64_t s9, char *o, size_t n)
{
    char r1[64], r2[64], r3[64];

    o[0] = 0;
    if (!strcmp(name, "IntersectRect")) {
        rect_str(r1, sizeof r1, a1); rect_str(r2, sizeof r2, a2); rect_str(r3, sizeof r3, a8);
        snprintf(o, n, "dst %s = %s ^ %s", r1, r2, r3);
    } else if (!strcmp(name, "GetClipBox")) {
        rect_str(r1, sizeof r1, a2);
        snprintf(o, n, "hdc %#" PRIx64 " box %s", a1, r1);
    } else if (!strcmp(name, "IntersectClipRect") || !strcmp(name, "ExcludeClipRect")) {
        snprintf(o, n, "hdc %#" PRIx64 " rect (%d,%d,%d,%d)", a1, (int)(int32_t)a2, (int)(int32_t)a8, (int)(int32_t)a9, (int)(int32_t)s5);
    } else if (!strcmp(name, "SetWindowOrgEx") || !strcmp(name, "SetViewportOrgEx")) {
        snprintf(o, n, "hdc %#" PRIx64 " org (%d,%d)", a1, (int)(int32_t)a2, (int)(int32_t)a8);
    } else if (!strcmp(name, "BeginPaint")) {
        rect_str(r1, sizeof r1, a2 + 12);
        snprintf(o, n, "hwnd %#" PRIx64 " rcPaint %s", a1, r1);
    } else if (!strcmp(name, "BitBlt")) {
        snprintf(o, n, "hdc %#" PRIx64 " dst (%d,%d) %dx%d src hdc %#" PRIx64 " (%d,%d) rop %#" PRIx64, a1, (int)(int32_t)a2, (int)(int32_t)a8, (int)(int32_t)a9, (int)(int32_t)s5, s6, (int)(int32_t)s7, (int)(int32_t)s8, s9);
    } else if (!strcmp(name, "CreateRectRgn")) {
        snprintf(o, n, "rect (%d,%d,%d,%d)", (int)(int32_t)a1, (int)(int32_t)a2, (int)(int32_t)a8, (int)(int32_t)a9);
    } else if (!strcmp(name, "OffsetRgn")) {
        snprintf(o, n, "rgn %#" PRIx64 " by (%d,%d)", a1, (int)(int32_t)a2, (int)(int32_t)a8);
    } else if (!strcmp(name, "SelectClipRgn") || !strcmp(name, "GetClipRgn")) {
        snprintf(o, n, "hdc %#" PRIx64 " rgn %#" PRIx64, a1, a2);
    } else if (!strcmp(name, "MoveToEx")) {
        snprintf(o, n, "hdc %#" PRIx64 " to (%d,%d)", a1, (int)(int32_t)a2, (int)(int32_t)a8);
    } else if (!strcmp(name, "LoadLibraryW") || !strcmp(name, "GetModuleHandleW") || !strcmp(name, "GetFileAttributesW") || !strcmp(name, "FindFirstFileW") || !strcmp(name, "GetEnvironmentVariableW") || !strcmp(name, "LoadLibraryExW") || !strcmp(name, "LoadImageW") || !strcmp(name, "SHGetFolderPathW") || !strcmp(name, "GetFileAttributesExW")) {
        wstr(r1, sizeof r1, a1);
        snprintf(o, n, "name \"%s\"", r1);
    } else if (!strcmp(name, "RegOpenKeyExW") || !strcmp(name, "RegCreateKeyExW") || !strcmp(name, "RegQueryValueExW") || !strcmp(name, "RegGetValueW") || !strcmp(name, "RegDeleteKeyW") || !strcmp(name, "RegSetValueExW")) {
        wstr(r1, sizeof r1, a2);
        snprintf(o, n, "hkey %#" PRIx64 " name \"%s\"", a1, r1);
    } else if (!strcmp(name, "CreateWindowExW")) {
        char c2[64];

        wstr(c2, sizeof c2, a2);
        wstr(r1, sizeof r1, a8);
        snprintf(o, n, "class \"%s\" title \"%s\"", c2, r1);
    } else if (!strcmp(name, "GetProcAddress")) {
        size_t k = 0;

        for (unsigned i = 0; i < 60 && k + 1 < sizeof r1; i++) {
            uint32_t c = (uint32_t)fle_ld(a2 + i, 1);

            if (!c) break;
            r1[k++] = (char)c;
        }
        r1[k] = 0;
        snprintf(o, n, "module %#" PRIx64 " proc \"%s\"", a1, r1);
    } else if (!strcmp(name, "RegisterClassW") || !strcmp(name, "RegisterClassExW")) {
        int ex = name[13] == 'E';
        char c2[64];

        wstr(c2, sizeof c2, fle_ld(a1 + 64, 8));
        snprintf(o, n, "class \"%s\" style %#x proc %#" PRIx64 " hbrBackground %#" PRIx64 " hCursor %#" PRIx64, c2, (unsigned)fle_ld(a1 + (ex ? 4 : 0), 4), fle_ld(a1 + 8, 8), fle_ld(a1 + 48, 8), fle_ld(a1 + 40, 8));
    } else if (!strcmp(name, "GetClientRect") || !strcmp(name, "GetWindowRect")) {
        rect_str(r1, sizeof r1, a2);
        snprintf(o, n, "hwnd %#" PRIx64 " rect %s", a1, r1);
    }
}

static void v1_call(const char *name, void (*real)(void))
{
    int on = active(name);
    uint64_t a1 = R[1], a2 = R[2], a8 = R[8], a9 = R[9], ra = fle_ld(R[4], 8);
    uint64_t s5 = 0, s6 = 0, s7 = 0, s8 = 0, s9 = 0;
    char det[256];

    if (on) {
        s5 = fle_arg(5); s6 = fle_arg(6); s7 = fle_arg(7); s8 = fle_arg(8); s9 = fle_arg(9);
    }
    real();
    if (on) {
        detail(name, a1, a2, a8, a9, s5, s6, s7, s8, s9, det, sizeof det);
        fprintf(stderr, "vis1-api: ms %" PRId64 " t%u %s %#" PRIx64 " %#" PRIx64 " %#" PRIx64 " %#" PRIx64 " ra %#" PRIx64 " -> %#" PRIx64 " %s\n", now_ms(), fle_thread_current(), name, a1, a2, a8, a9, ra, R[0], det);
    }
    if (want_hwnd && !strcmp(name, "EndPaint") && a1 == want_hwnd) in_span = 0;
}

#define V1API(n) void __wrap_fle_imp_##n(void) { v1_call(#n, __real_fle_imp_##n); }
#include "vis1/vis1_apis.inc"
#undef V1API
