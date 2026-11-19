/* parity1 (decision 0226): link-time counters of the raster primitives (native build only; -Wl,--wrap=fle_r_blit ...). Every call of a wrapped primitive appends one record (virtual clock ms, kind, pixels returned, width*height asked) to
 * $PARITY_BLIT_LOG at exit (binary: u32 clock_ms, u32 kind, u64 px_returned). Used to tell how many pixels the guest's drawing writes per input, independent of the compiler (lift/harness/parity/blitvol.py reads it). */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "fle_raster.h"
#include "fle_runtime.h"
extern uint64_t fle_sys_clock_ticks(void);
#define TPM 10000u
typedef struct { uint32_t ms, kind; uint64_t px; } rec;
static rec *recs; static size_t nrec;
static FILE *lf; static int lfopen;
static void add(uint32_t kind, uint64_t px) {
    rec r;
    if (!lfopen) { const char *p = getenv("PARITY_BLIT_LOG"); lfopen = 1; if (p && *p) lf = fopen(p, "wb"); }
    if (!lf) return;
    r.ms = (uint32_t)(fle_sys_clock_ticks() / 10000u); r.kind = kind; r.px = px;
    fwrite(&r, sizeof r, 1, lf);
    if ((++nrec & 255u) == 0) fflush(lf);   /* the engine ends with abort(): no exit handler runs, so the file is flushed every 256 records */
}
uint64_t __real_fle_r_blit(const fle_rsurf *d, const fle_rrect *clip, int64_t dx, int64_t dy, int64_t w, int64_t h, const fle_rsurf *s, int64_t sx, int64_t sy, uint32_t idx, uint32_t pat);
uint64_t __wrap_fle_r_blit(const fle_rsurf *d, const fle_rrect *clip, int64_t dx, int64_t dy, int64_t w, int64_t h, const fle_rsurf *s, int64_t sx, int64_t sy, uint32_t idx, uint32_t pat) {
    uint64_t r = __real_fle_r_blit(d, clip, dx, dy, w, h, s, sx, sy, idx, pat); add(1, r); return r;
}
uint64_t __real_fle_r_rop_rect(const fle_rsurf *d, const fle_rrect *clip, fle_rrect r, uint32_t idx, uint32_t pat);
uint64_t __wrap_fle_r_rop_rect(const fle_rsurf *d, const fle_rrect *clip, fle_rrect r, uint32_t idx, uint32_t pat) { uint64_t x = __real_fle_r_rop_rect(d, clip, r, idx, pat); add(2, x); return x; }
uint64_t __real_fle_r_mix_rect(const fle_rsurf *d, const fle_rrect *clip, fle_rrect r, uint32_t rop2, uint32_t pen);
uint64_t __wrap_fle_r_mix_rect(const fle_rsurf *d, const fle_rrect *clip, fle_rrect r, uint32_t rop2, uint32_t pen) { uint64_t x = __real_fle_r_mix_rect(d, clip, r, rop2, pen); add(3, x); return x; }


/* API call counters (printed with the SendMessageW lines) */
static unsigned long cnt[10];
void __real_fle_imp_GetTickCount(void);
void __wrap_fle_imp_GetTickCount(void) { cnt[0]++; __real_fle_imp_GetTickCount(); }
void __real_fle_imp_PeekMessageW(void);
void __wrap_fle_imp_PeekMessageW(void) { cnt[1]++; __real_fle_imp_PeekMessageW(); }
void __real_fle_imp_GetMessageW(void);
void __wrap_fle_imp_GetMessageW(void) { cnt[2]++; __real_fle_imp_GetMessageW(); }
void __real_fle_imp_WaitMessage(void);
void __wrap_fle_imp_WaitMessage(void) { cnt[3]++; __real_fle_imp_WaitMessage(); }
void __real_fle_imp_MsgWaitForMultipleObjects(void);
void __wrap_fle_imp_MsgWaitForMultipleObjects(void) { cnt[4]++; __real_fle_imp_MsgWaitForMultipleObjects(); }
void __real_fle_imp_PostMessageW(void);
void __wrap_fle_imp_PostMessageW(void) { cnt[5]++; __real_fle_imp_PostMessageW(); }
static uint32_t dm_msg[256]; static unsigned long dm_n[256]; static uint64_t dm_hwnd[256]; static unsigned dm_k;
void __real_fle_imp_DispatchMessageW(void);
void __wrap_fle_imp_DispatchMessageW(void) {
    uint64_t p = R[1]; uint32_t m = (uint32_t)fle_ld(p + 8, 4); uint64_t h = fle_ld(p, 8); unsigned k;
    cnt[6]++;
    for (k = 0; k < dm_k; k++) if (dm_msg[k] == m && dm_hwnd[k] == h) break;
    if (k == dm_k && dm_k < 256) { dm_msg[dm_k] = m; dm_hwnd[dm_k] = h; dm_n[dm_k] = 0; dm_k++; }
    if (k < 256) dm_n[k]++;
    __real_fle_imp_DispatchMessageW();
}
void __real_fle_imp_Sleep(void);
void __wrap_fle_imp_Sleep(void) { cnt[7]++; __real_fle_imp_Sleep(); }
void __real_fle_imp_SetTimer(void);
void __wrap_fle_imp_SetTimer(void) { cnt[8]++; __real_fle_imp_SetTimer(); }
void __real_fle_imp_InvalidateRect(void);
void __wrap_fle_imp_InvalidateRect(void) { cnt[9]++; __real_fle_imp_InvalidateRect(); }
static const char *const cnt_names[] = { "GetTickCount","PeekMessageW","GetMessageW","WaitMessage","MsgWaitForMultipleObjects","PostMessageW","DispatchMessageW","Sleep","SetTimer","InvalidateRect" };
static void print_counts(void) { unsigned k; fprintf(stderr, "parity: counts"); for (k = 0; k < 10; k++) fprintf(stderr, " %s=%lu", cnt_names[k], cnt[k]); fprintf(stderr, "\n"); { unsigned a, b, best; unsigned long bn; static char used[256]; for (a = 0; a < dm_k; a++) used[a] = 0; fprintf(stderr, "parity: dispatched (top 6)"); for (b = 0; b < 6; b++) { best = 256; bn = 0; for (a = 0; a < dm_k; a++) if (!used[a] && dm_n[a] > bn) { bn = dm_n[a]; best = a; } if (best == 256) break; used[best] = 1; fprintf(stderr, " [msg %#x hwnd %#llx x%lu]", dm_msg[best], (unsigned long long)dm_hwnd[best], bn); } fprintf(stderr, "\n"); } }

/* SendMessageW / GetTickCount counters (decision 0226): used to name the message that a guest loop that never returns keeps sending (stderr, first lines and every 20000th call) */
void __real_fle_imp_SendMessageW(void);
void __wrap_fle_imp_SendMessageW(void) {
    static unsigned long n; static uint32_t seen[64]; static unsigned ns;
    uint32_t msg = (uint32_t)R[2]; uint64_t hwnd = R[1], wp = R[8], lp = R[9];
    n++;
    { unsigned k; for (k = 0; k < ns; k++) if (seen[k] == msg) break; if (k == ns && ns < 64) { seen[ns++] = msg; fprintf(stderr, "parity: SendMessageW first msg %#x hwnd %#llx wparam %#llx lparam %#llx (call %lu, clock %llu ms)\n", msg, (unsigned long long)hwnd, (unsigned long long)wp, (unsigned long long)lp, n, (unsigned long long)(fle_sys_clock_ticks() / 10000u)); } }
    if (n % 20000 == 0) fprintf(stderr, "parity: SendMessageW %lu calls, last msg %#x hwnd %#llx (clock %llu ms)\n", n, msg, (unsigned long long)hwnd, (unsigned long long)(fle_sys_clock_ticks() / 10000u)), print_counts();
    __real_fle_imp_SendMessageW();
}

/* API trace of the calls whose first argument (RCX = hwnd) is a window named in $PARITY_TRACE_HWND (comma separated hex), after the virtual clock reached $PARITY_TRACE_FROM_MS; first 80 lines (decision 0226: the WM_PAINT storm of the mixer hover) */
int parity_trace_all_on;
static int trace_on(uint64_t h) {
    static uint64_t t[8]; static int nt = -1; static uint64_t from; const char *e;
    if (nt < 0) { nt = 0; e = getenv("PARITY_TRACE_HWND"); from = getenv("PARITY_TRACE_FROM_MS") ? strtoull(getenv("PARITY_TRACE_FROM_MS"), 0, 10) : 0;
        while (e && *e && nt < 8) { t[nt++] = strtoull(e, (char **)&e, 16); if (*e == ',') e++; } }
    { int k; for (k = 0; k < nt; k++) if (t[k] == h) { if (fle_sys_clock_ticks() / 10000u >= from) { parity_trace_all_on = 1; return 1; } return 0; } }
    return 0;
}
static unsigned trace_lines;
#define TRACE_API(NAME) \
void __real_fle_imp_##NAME(void); \
void __wrap_fle_imp_##NAME(void) { \
    uint64_t h = R[1], a2 = R[2], a3 = R[8]; \
    if (trace_lines < 80 && trace_on(h)) { trace_lines++; fprintf(stderr, "parity: api " #NAME "(hwnd %#llx, %#llx, %#llx, r9 %#llx) ret %#llx clock %llu\n", (unsigned long long)h, (unsigned long long)a2, (unsigned long long)a3, (unsigned long long)R[9], (unsigned long long)fle_ld(R[4], 8), (unsigned long long)(fle_sys_clock_ticks() / 10000u)); if (0 == strcmp(#NAME, "RedrawWindow")) { unsigned q; fprintf(stderr, "parity:   stack"); for (q = 0; q < 200; q++) { uint64_t v = fle_ld(R[4] + 8u * q, 8); if (v >= 0x401000 && v < 0x1300000) fprintf(stderr, " +%u:%#llx", q * 8, (unsigned long long)v); } fprintf(stderr, "\n"); } if (a2 > 0x10000 && a2 < 0x7fffffffffffULL && (0 == strcmp(#NAME, "RedrawWindow") || 0 == strcmp(#NAME, "ValidateRect"))) fprintf(stderr, "parity:   rect %d %d %d %d\n", (int)(int32_t)fle_ld(a2, 4), (int)(int32_t)fle_ld(a2 + 4, 4), (int)(int32_t)fle_ld(a2 + 8, 4), (int)(int32_t)fle_ld(a2 + 12, 4)); } \
    __real_fle_imp_##NAME(); \
    if (trace_lines < 80 && trace_on(h)) fprintf(stderr, "parity:   -> rax %#llx\n", (unsigned long long)R[0]); \
}
TRACE_API(BeginPaint)
TRACE_API(EndPaint)
TRACE_API(ValidateRect)
TRACE_API(ValidateRgn)
TRACE_API(GetUpdateRect)
TRACE_API(GetUpdateRgn)
TRACE_API(RedrawWindow)
TRACE_API(DefWindowProcW)
TRACE_API(InvalidateRgn)
TRACE_API(UpdateWindow)
TRACE_API(CallWindowProcW)
TRACE_API(GetDC)
TRACE_API(GetDCEx)
TRACE_API(ReleaseDC)
