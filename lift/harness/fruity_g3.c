/* Decision 0136 (PROVISIONAL): the G3 round trip, harness side (native and wasm). Compiled and linked into the FRUITY-on harness (build_two_native.sh FRUITY=1, build_nmod_wasm.sh FRUITY=1).
 *
 *  g3_snapshot(name): prints the "g3-snap:" block that every observation of the round trip is made of: the virtual clock, one "g3-surface:" line per live drawing surface (the SHA-256 of the surface as RGBA with
 *  A = 255, the bytes a canvas read-back gives; the same function as fruity_surface.c's), the "g3-win:" lines of the visible top-level windows (class, title, enabled), and the number of WM_TIMER deliveries so far.
 *  g3_script_idle(now, next, &advance): the input SCRIPT (env FLE_G3_SCRIPT = a file) that stands for the browser's input in a native run and in a node run: one command per idle. The same events, in the same order, at the
 *  same virtual times are what the browser's real input produces in a Chrome run, so the "g3-snap:" blocks of the three can be compared. Commands (one per line, # starts a comment):
 *      snap NAME            print a snapshot (no event, no clock move)
 *      move X Y             queue a pointer move to the screen point (X, Y)
 *      down [L|R|M]         queue a button press at the pointer;  up [L|R|M] the release
 *      click X Y [L|R|M]    move, down, up
 *      key down|up VK [SCAN [EXT]]   queue a key event (VK decimal or 0x..)
 *      tap VK               key down, key up
 *      advance MS           move the virtual clock by MS ms, deadline by deadline (every window timer and Sleep that falls due is delivered at its own time)
 *      probetimer LID ID MS a timer made by the HOST for the window with logical id LID (a probe, not a call of the guest: see fle_win_host_timer)
 *      timers               print the WM_TIMER delivery log
 *      open PATH            decision 0144: send WM_COPYDATA (a file path of the virtual volume, rest of the line) to the main window (runtime/fle_appfile.c); "g8: open ... request N" or the error
 *      dump NAME            decision 0144: write the raw BGRX bytes of every live surface to $FLE_G8_DUMPDIR/NAME-lid<L>-<W>x<H>.bgrx (tools/bgrx2png.py makes the PNG); no dir = nothing
 *      openstate            print the state of the last open request (1 sent, 2 processed + the window procedure's answer)
 *      end                  the session ends (a snapshot and the timer log first)
 *      setenv NAME [VALUE]  decision 0171 (native fork-probe builds): set an environment variable of the engine process (read lazily by the runtime, e.g. FLE_SEH_TRACE); inherited by later probes
 *      probe NAME ... probe-end   decision 0171 (native only; needs -DFLE_G3_FORKPROBE and the link option -Wl,--wrap=syscall, see __wrap_syscall): the commands up to the matching probe-end run in a fork()ed child; the child's stdout and stderr go to
 *                           $FLE_G3_PROBE_DIR/NAME.out and .err (default ./probes), it takes at most $FLE_G3_PROBE_CPU CPU seconds (default 600; RLIMIT_CPU), and a snapshot "probe-NAME-end" is
 *                           printed when it reaches probe-end, then the child exits 0. The parent prints "g3-probe: NAME exit N" / "signal N", skips to after probe-end and goes on: one run
 *                           of the engine explores many independent operations from the same state.
 *  Without a script, the native hook is the clock jump of decision 0122 (FLE_FRUITY_NO_IDLE_STOP=1): the clock goes to the next deadline; with nothing pending the session ends.
 *
 *  The window table is read through fle_win_table_report (text) and the surfaces through fle_raster_surface_peek: nothing in lift/runtime is changed for it. */
#include <inttypes.h>
#include "fle_test_env.h"
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#if !defined(__EMSCRIPTEN__)
#include <sys/resource.h>
#endif

#include "fle_appfile.h"
#include "fle_runtime.h"
#include "fle_input.h"
#include "fle_layered.h"
#include "fle_raster.h"
#include "fle_sys.h"
#include "fle_thread.h"
#include "fle_win.h"
#include "fle_vfs.h"
#include "fle_env.h"
#include "fle_imports.h"

#if !defined(__EMSCRIPTEN__) && defined(FLE_G3_FORKPROBE)
#include <fcntl.h>
#include <signal.h>
#include <sys/resource.h>
#include <sys/syscall.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <unistd.h>
#endif

extern uint64_t trace_call_count(void);
void fle_win_timer_report(FILE *f);

#if !defined(__EMSCRIPTEN__) && defined(FLE_G3_FORKPROBE)
/* decision 0222 (native fork-probe builds only): the reports of the window model's trace (runtime/fle_win.c: fle_win_trace, switched by the g3 command `wintrace on`): one line on stderr for every message that reaches a
 * window procedure (phase 0 before the call, 1 after it with the result) and for every z-order move */
static const char *w2_msgname(uint32_t m)
{
    switch (m) {
    case 0x0001: return "WM_CREATE"; case 0x0002: return "WM_DESTROY"; case 0x0003: return "WM_MOVE"; case 0x0005: return "WM_SIZE"; case 0x0006: return "WM_ACTIVATE"; case 0x0007: return "WM_SETFOCUS";
    case 0x0008: return "WM_KILLFOCUS"; case 0x000A: return "WM_ENABLE"; case 0x000B: return "WM_SETREDRAW"; case 0x000F: return "WM_PAINT"; case 0x0010: return "WM_CLOSE"; case 0x0014: return "WM_ERASEBKGND";
    case 0x0018: return "WM_SHOWWINDOW"; case 0x001C: return "WM_ACTIVATEAPP"; case 0x001F: return "WM_CANCELMODE"; case 0x0020: return "WM_SETCURSOR"; case 0x0021: return "WM_MOUSEACTIVATE";
    case 0x0022: return "WM_CHILDACTIVATE"; case 0x0024: return "WM_GETMINMAXINFO"; case 0x0046: return "WM_WINDOWPOSCHANGING"; case 0x0047: return "WM_WINDOWPOSCHANGED"; case 0x004E: return "WM_NOTIFY";
    case 0x0081: return "WM_NCCREATE"; case 0x0082: return "WM_NCDESTROY"; case 0x0083: return "WM_NCCALCSIZE"; case 0x0084: return "WM_NCHITTEST"; case 0x0085: return "WM_NCPAINT"; case 0x0086: return "WM_NCACTIVATE";
    case 0x00A0: return "WM_NCMOUSEMOVE"; case 0x00A1: return "WM_NCLBUTTONDOWN"; case 0x00A2: return "WM_NCLBUTTONUP"; case 0x0100: return "WM_KEYDOWN"; case 0x0101: return "WM_KEYUP"; case 0x0102: return "WM_CHAR";
    case 0x0104: return "WM_SYSKEYDOWN"; case 0x0111: return "WM_COMMAND"; case 0x0112: return "WM_SYSCOMMAND"; case 0x0113: return "WM_TIMER"; case 0x0200: return "WM_MOUSEMOVE"; case 0x0201: return "WM_LBUTTONDOWN";
    case 0x0202: return "WM_LBUTTONUP"; case 0x0203: return "WM_LBUTTONDBLCLK"; case 0x0204: return "WM_RBUTTONDOWN"; case 0x0205: return "WM_RBUTTONUP"; case 0x0207: return "WM_MBUTTONDOWN"; case 0x0208: return "WM_MBUTTONUP";
    case 0x020A: return "WM_MOUSEWHEEL"; case 0x0210: return "WM_PARENTNOTIFY"; case 0x0215: return "WM_CAPTURECHANGED"; case 0x02A2: return "WM_NCMOUSELEAVE"; case 0x02A3: return "WM_MOUSELEAVE";
    case 0x0030: return "WM_SETFONT"; case 0x000C: return "WM_SETTEXT"; default: return "";
    }
}
void fle_win_trace_msg(int phase, const char *api, uint32_t lid, const char *cls, uint32_t msg, uint64_t wp, uint64_t lp, uint64_t result)
{
    if (phase == 1) {
        fprintf(stderr, "w2m: < %s lid %u %s %#x %s = %#" PRIx64 "\n", api, lid, cls, (unsigned)msg, w2_msgname(msg), result);
        return;
    }
    if ((msg == 0x46u || msg == 0x47u) && lp)
        fprintf(stderr, "w2m: > %s lid %u %s %#x %s wp %#" PRIx64 " lp %#" PRIx64 " [insertafter %#" PRIx64 " x %d y %d cx %d cy %d flags %#x]\n", api, lid, cls, (unsigned)msg, w2_msgname(msg), wp, lp, fle_ld(lp + 8, 8),
                (int)(int32_t)fle_ld(lp + 16, 4), (int)(int32_t)fle_ld(lp + 20, 4), (int)(int32_t)fle_ld(lp + 24, 4), (int)(int32_t)fle_ld(lp + 28, 4), (unsigned)fle_ld(lp + 32, 4));
    else fprintf(stderr, "w2m: > %s lid %u %s %#x %s wp %#" PRIx64 " lp %#" PRIx64 "\n", api, lid, cls, (unsigned)msg, w2_msgname(msg), wp, lp);
}
void fle_win_trace_z(const char *what, uint32_t lid, const char *cls, uint32_t after_lid, const char *after_cls)
{
    fprintf(stderr, "w2z: z-move %s lid %u %s%s%u %s\n", what, lid, cls, after_lid ? " after lid " : "", after_lid, after_cls);
}
#endif

typedef struct { uint32_t h[8]; uint8_t buf[64]; uint64_t len; uint32_t fill; } sha_t;
static const uint32_t K[64] = {
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2 };
#define ROR(x, n) (((x) >> (n)) | ((x) << (32 - (n))))
static void sha_block(sha_t *s, const uint8_t *p) {
    uint32_t w[64], a, b, c, d, e, f, g, h, t1, t2;
    for (int i = 0; i < 16; i++) w[i] = ((uint32_t)p[4 * i] << 24) | ((uint32_t)p[4 * i + 1] << 16) | ((uint32_t)p[4 * i + 2] << 8) | p[4 * i + 3];
    for (int i = 16; i < 64; i++) {
        uint32_t s0 = ROR(w[i - 15], 7) ^ ROR(w[i - 15], 18) ^ (w[i - 15] >> 3), s1 = ROR(w[i - 2], 17) ^ ROR(w[i - 2], 19) ^ (w[i - 2] >> 10);
        w[i] = w[i - 16] + s0 + w[i - 7] + s1;
    }
    a = s->h[0]; b = s->h[1]; c = s->h[2]; d = s->h[3]; e = s->h[4]; f = s->h[5]; g = s->h[6]; h = s->h[7];
    for (int i = 0; i < 64; i++) {
        t1 = h + (ROR(e, 6) ^ ROR(e, 11) ^ ROR(e, 25)) + ((e & f) ^ (~e & g)) + K[i] + w[i];
        t2 = (ROR(a, 2) ^ ROR(a, 13) ^ ROR(a, 22)) + ((a & b) ^ (a & c) ^ (b & c));
        h = g; g = f; f = e; e = d + t1; d = c; c = b; b = a; a = t1 + t2;
    }
    s->h[0] += a; s->h[1] += b; s->h[2] += c; s->h[3] += d; s->h[4] += e; s->h[5] += f; s->h[6] += g; s->h[7] += h;
}
static void sha_init(sha_t *s) {
    static const uint32_t iv[8] = { 0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19 };
    memcpy(s->h, iv, sizeof iv);
    s->len = 0;
    s->fill = 0;
}
static void sha_update(sha_t *s, const uint8_t *p, size_t n) {
    s->len += n;
    while (n) {
        size_t k = 64u - s->fill < n ? 64u - s->fill : n;
        memcpy(s->buf + s->fill, p, k);
        s->fill += (uint32_t)k;
        p += k;
        n -= k;
        if (s->fill == 64u) { sha_block(s, s->buf); s->fill = 0; }
    }
}
static void sha_final(sha_t *s, char hex[65]) {
    uint64_t bits = s->len * 8u;
    uint8_t one = 0x80, z = 0, lb[8];
    sha_update(s, &one, 1);
    while (s->fill != 56) sha_update(s, &z, 1);
    for (int i = 0; i < 8; i++) lb[i] = (uint8_t)(bits >> (56 - 8 * i));
    sha_update(s, lb, 8);
    for (int i = 0; i < 8; i++) snprintf(hex + 8 * i, 9, "%08x", s->h[i]);
}

/* the SHA-256 of a BGRX surface as RGBA with A = 255 (what a canvas read-back gives and what decision 0126's PNGs decode to) */
static void rgba_sha(const uint8_t *bits, uint32_t w, uint32_t h, uint32_t stride, char hex[65], uint64_t *nonzero) {
    sha_t s;
    uint8_t *row = (uint8_t *)malloc((size_t)w * 4u);
    uint64_t nz = 0;
    sha_init(&s);
    if (row) {
        for (uint32_t y = 0; y < h; y++) {
            const uint8_t *p = bits + (uint64_t)y * stride;
            for (uint32_t x = 0; x < w; x++) {
                row[4 * x] = p[4 * x + 2];
                row[4 * x + 1] = p[4 * x + 1];
                row[4 * x + 2] = p[4 * x];
                row[4 * x + 3] = 255;
                if (p[4 * x] | p[4 * x + 1] | p[4 * x + 2]) nz++;
            }
            sha_update(&s, row, (size_t)w * 4u);
        }
        free(row);
    }
    sha_final(&s, hex);
    if (nonzero) *nonzero = nz;
}

static unsigned snap_no;

/* decision 0144: the SHA-256 of a byte range as lower-case hex (for the G8 probe's per-block record of the audio ring) */
void g3_sha256_hex(const void *p, size_t n, char hex[65]) {
    sha_t s;
    sha_init(&s);
    sha_update(&s, (const uint8_t *)p, n);
    sha_final(&s, hex);
}
static int last_open_request;

/* decision 0144: the raw BGRX bytes of every live surface (a file per surface) for the PNG record of a stage */
static void g8_dump(const char *name) {
    const char *dir = FLE_TEST_GETENV("FLE_G8_DUMPDIR");
    if (!dir || !dir[0]) { printf("g8: dump %s: FLE_G8_DUMPDIR is not set\n", name); return; }
    for (uint32_t lid = 1; lid < 4096u; lid++) {
        const fle_rsurf *s = fle_raster_surface_peek(lid);
        char path[600];
        FILE *f;
        if (!s || !s->bits) continue;
        snprintf(path, sizeof path, "%s/%s-lid%u-%ux%u.bgrx", dir, name, lid, (unsigned)s->width, (unsigned)s->height);
        f = fopen(path, "wb");
        if (!f) { printf("g8: dump: cannot write %s\n", path); continue; }
        for (uint32_t y = 0; y < s->height; y++) fwrite(s->bits + (uint64_t)y * s->stride, 4, s->width, f);
        fclose(f);
        printf("g8: dump %s\n", path);
    }
    fflush(stdout);
}

/* a diagnostic: with the script commands `apitrace on|off` the API wrappers of a build that links tools/gen_apitrace.py's wrappers print the name of every runtime API the guest calls (g3_api_print). Without the wrappers the commands do nothing. */
int g3_api_trace;
void g3_api_print(const char *name) {
    fprintf(stderr, "g3-api: t%u %s ra %#llx\n", fle_thread_current ? fle_thread_current() : 0u, name, (unsigned long long)fle_ld(R[4], 8));
}

void g3_timer_log(FILE *f) {
    unsigned n = fle_win_timer_log_count();
    fprintf(f, "g3-timers: %u WM_TIMER deliveries\n", n);
    for (unsigned i = 0; i < n; i++) {
        fle_timer_log_t e;
        if (!fle_win_timer_log_get(i, &e)) break;
        fprintf(f, "g3-timer: #%u hwnd %#" PRIx64 " id %#" PRIx64 " period_ms %u due_ticks %" PRIu64 " delivered_ticks %" PRIu64 " late_ticks %" PRId64 "\n", i, e.hwnd, e.id, e.period_ms, e.due_ticks, e.delivered_ticks,
                (int64_t)(e.delivered_ticks - e.due_ticks));
    }
}

/* the visible top-level windows of the table report, one line each */
static void win_lines(FILE *out) {
    char *buf = NULL;
    size_t len = 0;
    FILE *m = open_memstream(&buf, &len);
    if (!m) return;
    fle_win_table_report(m);
    fclose(m);
    if (buf) {
        /* decision 0142: the SHA-256 of the whole window table text of the snapshot (the table is part of the state that a visible region reads) */
        sha_t ts;
        char th[65];
        unsigned tl = 0;
        sha_init(&ts);
        sha_update(&ts, (const uint8_t *)buf, strlen(buf));
        sha_final(&ts, th);
        for (const char *q = buf; *q; q++) tl += *q == '\n';
        fprintf(out, "g3-wintable: lines %u sha256 %s\n", tl, th);
        {   /* decision 0147: FLE_G3_WINTABLE_DIR=<dir> keeps the table text of each snapshot as <dir>/wintable-<snapshot number>.txt (a diagnostic; off unless asked) */
            const char *wd = FLE_TEST_GETENV("FLE_G3_WINTABLE_DIR");
            char path[512];
            FILE *wf;
            if (wd && *wd) {
                snprintf(path, sizeof path, "%s/wintable-%u.txt", wd, snap_no);
                wf = fopen(path, "wb");
                if (wf) { fputs(buf, wf); fclose(wf); }
            }
        }
    }
    if (buf) {
        char *p = buf;
        while (*p) {
            char *e = strchr(p, '\n');
            if (e) *e = 0;
            if (strstr(p, "    handle ") == p && strstr(p, " owner ") && !strstr(p, " parent ") && strstr(p, "visible 1")) {
                const char *cl = strstr(p, "class \""), *ti = strstr(p, "title \"");
                char cls[64] = "", tit[64] = "";
                if (cl) sscanf(cl, "class \"%63[^\"]\"", cls);
                if (ti) sscanf(ti, "title \"%63[^\"]\"", tit);
                const char *en = strstr(p, "enabled ");
                const char *lid = strstr(p, " lid ");
                fprintf(out, "g3-win: lid %u class %s title \"%s\" enabled %d\n", lid ? (unsigned)strtoul(lid + 5, NULL, 10) : 0u, cls, tit, en ? atoi(en + 8) : -1);
            } else if (strstr(p, "    handle ") == p && !strstr(p, " parent ") && strstr(p, "visible 1")) {
                const char *cl = strstr(p, "class \""), *ti = strstr(p, "title \"");
                char cls[64] = "", tit[64] = "";
                if (cl) sscanf(cl, "class \"%63[^\"]\"", cls);
                if (ti) sscanf(ti, "title \"%63[^\"]\"", tit);
                const char *en = strstr(p, "enabled ");
                const char *lid = strstr(p, " lid ");
                fprintf(out, "g3-win: lid %u class %s title \"%s\" enabled %d\n", lid ? (unsigned)strtoul(lid + 5, NULL, 10) : 0u, cls, tit, en ? atoi(en + 8) : -1);
            }
            if (!e) break;
            p = e + 1;
        }
        free(buf);
    }
}

int fle_visrgn_dump_surfaces(const char *name) __attribute__((weak));
/* decision 0174: the sizes of the runtime's handle tables, one "g3-leak:" line per snapshot with FLE_G3_LEAKSTATS=1 (off otherwise: the g3 output of the earlier decisions is unchanged) */
void fle_gdi_table_stats(uint64_t o[6]) __attribute__((weak));
void fle_gdc_table_stats(uint64_t o[5]) __attribute__((weak));
void fle_event_table_stats(uint64_t o[3]) __attribute__((weak));

void g3_snapshot(const char *name) {
    if (fle_visrgn_dump_surfaces) {
        char nm[96];
        snprintf(nm, sizeof nm, "snap%u-%s", snap_no + 1u, name ? name : "-");
        fle_visrgn_dump_surfaces(nm);
    }
    printf("g3-snap: #%u %s clock_ms %" PRIu64 " entries %" PRIu64 " pending_events %u\n", ++snap_no, name ? name : "-", fle_sys_clock_ticks() / FLE_TICKS_PER_MS, trace_call_count(), fle_input_host_pending());
    if (FLE_TEST_GETENV("FLE_G3_PROGRESS") && fle_thread_preempt_stats) {   /* decision 0147: the progress counter (indirect calls) and the pseudo-preemption counters; off unless asked, so the g3 output of the earlier decisions is unchanged */
        fle_preempt_stats ps;
        fle_thread_preempt_stats(&ps);
        printf("g3-progress: #%u %s progress %" PRIu64 " armed %d slices %" PRIu64 " deferred %" PRIu64 " switches %" PRIu64 " ticks %" PRIu64 "\n", snap_no, name ? name : "-", ps.progress, ps.armed, ps.slices, ps.deferred,
               ps.switches, ps.ticks);
    }
    if (FLE_TEST_GETENV("FLE_G3_LEAKSTATS")) {
        uint64_t g[6] = {0}, d[5] = {0}, e[3] = {0};
        if (fle_gdi_table_stats) fle_gdi_table_stats(g);
        if (fle_gdc_table_stats) fle_gdc_table_stats(d);
        if (fle_event_table_stats) fle_event_table_stats(e);
        printf("g3-leak: #%u %s gdi_slots %" PRIu64 " gdi_live %" PRIu64 " gdi_cap %" PRIu64 " gdi_dead_waiting %" PRIu64 " rgn_cap %" PRIu64 " rgn_sets %" PRIu64 " dcs_n %" PRIu64 " dcs_live %" PRIu64 " dcs_cap %" PRIu64 " dcs_dead_waiting %" PRIu64 " dcs_persistent %" PRIu64 " ev_n %" PRIu64 " ev_live %" PRIu64 "\n", snap_no, name ? name : "-",
               g[0], g[1], g[2], g[3], g[4], g[5], d[0], d[1], d[2], d[3], d[4], e[0], e[1]);
    }
    for (uint32_t lid = 1; lid < 4096u; lid++) {
        const fle_rsurf *s = fle_raster_surface_peek(lid);
        char hex[65];
        uint64_t nz = 0;
        if (!s || !s->bits) continue;
        rgba_sha(s->bits, s->width, s->height, s->stride, hex, &nz);
        printf("g3-surface: lid %u %ux%u nonzero %" PRIu64 " sha256 %s\n", lid, s->width, s->height, nz, hex);
#ifndef __EMSCRIPTEN__
        {
            /* decision 0140: the layered-window attributes of the surface's window (fle_layered.c), and with FLE_G3_DUMP_DIR the raw bytes (B, G, R, X/A per pixel, top-down) of the surface as <dir>/<snapshot>-lid<N>.bgra */
            fle_layered_info li;
            const char *dir = FLE_TEST_GETENV("FLE_G3_DUMP_DIR");

            if (fle_layered_query(FLE_WIN_HANDLE_BASE + 4u * (uint64_t)lid, &li) && li.mode != FLE_LAYER_NONE)
                printf("g3-layered: lid %u mode %s alpha %u colorkey %#x per_pixel %u size %ux%u updates %" PRIu64 "\n", lid, li.mode == FLE_LAYER_ULW ? "ULW" : "SLWA", li.alpha, li.colorkey, li.per_pixel, li.width, li.height, li.updates);
            if (dir && dir[0] && name) {
                char path[1024];
                FILE *df;

                snprintf(path, sizeof path, "%s/%s-lid%u.bgra", dir, name, lid);
                df = fopen(path, "wb");
                if (df) {
                    for (uint32_t y = 0; y < s->height; y++) fwrite(s->bits + (uint64_t)y * s->stride, 4, s->width, df);
                    fclose(df);
                }
            }
        }
#endif
    }
    win_lines(stdout);
    printf("g3-timers-so-far: %u\n", fle_win_timer_log_count());
    {
        char *tb = NULL;
        size_t tl = 0;
        FILE *tm = open_memstream(&tb, &tl);
        if (tm) {
            fle_win_timer_report(tm);
            fclose(tm);
            if (tb) {
                for (char *p = tb; *p;) {
                    char *e = strchr(p, '\n');
                    if (e) *e = 0;
                    if (strstr(p, "timer slot")) printf("g3-timer-live:%s\n", p + strlen("  timer slot"));
                    if (!e) break;
                    p = e + 1;
                }
                free(tb);
            }
        }
    }
    fflush(stdout);
}

/* ---- the script ---- */
static char **lines;
static unsigned nlines, cur_line;
static int loaded;
static uint64_t pending_target;

#if !defined(__EMSCRIPTEN__) && defined(FLE_G3_FORKPROBE)
/* The runtime's single-thread rule compares fle_host_tid() = syscall(SYS_gettid) with the first thread that called it; a fork()ed child has another tid and would stop at the first window API.
 * The child therefore keeps answering the tid of the process that forked (the link wraps syscall). */
long __real_syscall(long n, ...);
static long probe_parent_tid;
long __wrap_syscall(long n, long a, long b, long c, long d, long e, long f)
{
    if (n == SYS_gettid && probe_parent_tid) return probe_parent_tid;
    return __real_syscall(n, a, b, c, d, e, f);
}
static FILE *follow_f;
static int more_lines(void);
static int in_probe;
static char probe_name[96];

extern void trace_cov_probe_begin(const char *name) __attribute__((weak));   /* decision 0184 (harness/wasm/trace_native.c); weak since decision 0188: a link without trace_native.c still works */
extern void trace_cov_probe_end(void) __attribute__((weak));
/* decision 0215 (sweep1): `probejobs N` (N > 1) lets the parent keep N probe children running; `waitprobes` (and `end`, and the end of a probe that forked) reaps them all. N = 1 (the default) is the old behaviour. */
static unsigned probe_jobs = 1, probe_inflight;
static struct { pid_t pid; char name[128]; } probe_pend[64];
static void probe_reap(int blocking) {
    int st = 0;
    pid_t p = waitpid(-1, &st, blocking ? 0 : WNOHANG);
    if (p < 0) { probe_inflight = 0; return; }   /* no child is left (a recorded pid that was never reaped must not make the caller spin) */
    if (p == 0) return;
    for (unsigned i = 0; i < probe_inflight; i++) {
        if (probe_pend[i].pid != p) continue;
        if (WIFEXITED(st)) printf("g3-probe: %s exit %d\n", probe_pend[i].name, WEXITSTATUS(st));
        else printf("g3-probe: %s signal %d\n", probe_pend[i].name, WTERMSIG(st));
        fflush(stdout);
        probe_pend[i] = probe_pend[--probe_inflight];
        return;
    }
}
static void probe_wait_all(void) { while (probe_inflight) probe_reap(1); }
/* returns 1 in the child (the script goes on there), 0 in the parent (the child has ended; the caller skips to after probe-end) */
static int probe_begin(const char *name) {
    unsigned k;
    const char *dir = FLE_TEST_GETENV("FLE_G3_PROBE_DIR");
    const char *cpu = FLE_TEST_GETENV("FLE_G3_PROBE_CPU");
    char path[512];
    pid_t pid;
    int st = 0;
    /* the whole block is in memory before the fork: the child must not read the FIFO the parent goes on reading */
    for (k = cur_line;; k++) {
        if (k >= nlines && !more_lines()) { fprintf(stderr, "g3: probe %s has no probe-end\n", name); exit(2); }
        if (!strncmp(lines[k], "probe-end", 9)) break;
    }
    fflush(stdout);
    fflush(stderr);
    while (probe_jobs > 1 && probe_inflight >= probe_jobs) probe_reap(1);   /* decision 0215: make room before the fork, so that the new child cannot be reaped before it is recorded */
    if (!probe_parent_tid) probe_parent_tid = __real_syscall(SYS_gettid);
    pid = fork();
    if (pid < 0) { perror("g3: fork"); exit(2); }
    if (pid == 0) {
        struct rlimit rl;
        in_probe = 1;
        probe_inflight = 0;
        probe_jobs = 1;
        follow_f = NULL;
        snprintf(probe_name, sizeof probe_name, "%s", name);
        if (trace_cov_probe_begin) trace_cov_probe_begin(name);
        snprintf(path, sizeof path, "%s/%s.out", dir && dir[0] ? dir : "probes", name);
        if (!freopen(path, "w", stdout)) _exit(97);
        snprintf(path, sizeof path, "%s/%s.err", dir && dir[0] ? dir : "probes", name);
        if (!freopen(path, "w", stderr)) _exit(97);
        setvbuf(stderr, NULL, _IONBF, 0);
        rl.rlim_cur = rl.rlim_max = cpu && cpu[0] ? (rlim_t)strtoul(cpu, NULL, 10) : 600;
        setrlimit(RLIMIT_CPU, &rl);
        return 1;
    }
    if (probe_jobs > 1) {   /* decision 0215 (sweep1, opt-in `probejobs N`): up to N probe children run at once; the result lines are printed when a child is reaped */
        probe_pend[probe_inflight].pid = pid;
        snprintf(probe_pend[probe_inflight].name, sizeof probe_pend[0].name, "%s", name);
        probe_inflight++;
        return 0;
    }
    if (waitpid(pid, &st, 0) < 0) { perror("g3: waitpid"); exit(2); }
    if (WIFEXITED(st)) printf("g3-probe: %s exit %d\n", name, WEXITSTATUS(st));
    else printf("g3-probe: %s signal %d\n", name, WTERMSIG(st));
    fflush(stdout);
    return 0;
}
#endif

static unsigned lines_cap;

static int add_line(const char *buf0) {
    char buf[512];
    size_t n;
    snprintf(buf, sizeof buf, "%s", buf0);
    n = strlen(buf);
    while (n && (buf[n - 1] == '\n' || buf[n - 1] == '\r')) buf[--n] = 0;
    if (!n || buf[0] == '#') return 0;
    if (nlines == lines_cap) { lines_cap = lines_cap ? lines_cap * 2 : 64; lines = (char **)realloc(lines, lines_cap * sizeof *lines); }
    lines[nlines++] = strdup(buf);
    return 1;
}

/* follow mode (FLE_G3_FOLLOW=1, native): FLE_G3_SCRIPT is a FIFO (opened read-write, so a writer closing does not end the script); the next command is read when the script has no more lines, and the
 * session waits there (no CPU) until one arrives. Returns 1 when line cur_line... exists after the call. */
static int more_lines(void) {
#if !defined(__EMSCRIPTEN__) && defined(FLE_G3_FORKPROBE)
    char buf[512];
    if (!follow_f) return 0;
    while (fgets(buf, sizeof buf, follow_f)) if (add_line(buf)) return 1;
#endif
    return 0;
}

static void load_script(void) {
    const char *path = FLE_TEST_GETENV("FLE_G3_SCRIPT");
    FILE *f;
    char buf[512];
    loaded = 1;
    if (!path || !path[0]) return;
#if !defined(__EMSCRIPTEN__) && defined(FLE_G3_FORKPROBE)
    if (FLE_TEST_GETENV("FLE_G3_FOLLOW") && FLE_TEST_GETENV("FLE_G3_FOLLOW")[0] == '1') {
        int fd = open(path, O_RDWR);
        if (fd < 0 || !(follow_f = fdopen(fd, "r"))) { fprintf(stderr, "g3: cannot open FLE_G3_SCRIPT=%s\n", path); exit(2); }
        printf("g3: script %s in follow mode\n", path);
        fflush(stdout);
        /* the caller asks for lines: nlines stays 0 here, g3_script_idle reads the first one */
        return;
    }
#endif
    f = fopen(path, "r");
    if (!f) { fprintf(stderr, "g3: cannot read FLE_G3_SCRIPT=%s\n", path); exit(2); }
    while (fgets(buf, sizeof buf, f)) add_line(buf);
    fclose(f);
    printf("g3: script %s, %u commands\n", path, nlines);
}

static unsigned parse_btn(const char *s) { return s && (s[0] == 'R' || s[0] == 'r') ? FLE_IN_BTN_R : s && (s[0] == 'M' || s[0] == 'm') ? FLE_IN_BTN_M : FLE_IN_BTN_L; }

/* returns -1 when no script is configured; else the hook answer (0 end, 1 events queued, 2 advance) */
int g3_script_idle(uint64_t now, uint64_t next, uint64_t *adv) {
    (void)next;
    if (!loaded) load_script();
#if !defined(__EMSCRIPTEN__) && defined(FLE_G3_FORKPROBE)
    if (!nlines && follow_f) more_lines();
#endif
    if (!nlines) return -1;
    for (;;) {
        char cmd[32] = "", a1[64] = "", a2[64] = "", a3[64] = "", a4[64] = "";
        int nf;
        const char *ln;
        if (pending_target) {
            if (now >= pending_target) pending_target = 0;
            else { *adv = pending_target; return 2; }
        }
        if (cur_line >= nlines && !more_lines()) { g3_snapshot("script-end"); g3_timer_log(stdout); fflush(stdout); return 0; }
        ln = lines[cur_line++];
        nf = sscanf(ln, "%31s %63s %63s %63s %63s", cmd, a1, a2, a3, a4);
        if (!strcmp(cmd, "snap")) { g3_snapshot(nf > 1 ? a1 : "-"); continue; }
#if !defined(__EMSCRIPTEN__)
        if (!strcmp(cmd, "cpu")) {   /* decision 0188: `cpu LABEL` prints the process's CPU time (getrusage, user+system), the virtual clock and the entry count: brackets a phase of a native run */
            struct rusage ru;
            getrusage(RUSAGE_SELF, &ru);
            printf("g3-cpu: %s user_ms %lld sys_ms %lld clock_ms %" PRIu64 " entries %" PRIu64 "\n", nf > 1 ? a1 : "-", (long long)ru.ru_utime.tv_sec * 1000 + ru.ru_utime.tv_usec / 1000,
                   (long long)ru.ru_stime.tv_sec * 1000 + ru.ru_stime.tv_usec / 1000, now / FLE_TICKS_PER_MS, trace_call_count());
            fflush(stdout);
            continue;
        }
        if (!strcmp(cmd, "vfsdump") && nf >= 3) {   /* decision 0188: vfsdump GUESTPATH HOSTFILE (no spaces): write the bytes of a file the guest wrote on the virtual volume to a host file, print size and SHA-256 */
            uint16_t *w = fle_w_from_ascii(a1);
            const uint8_t *d = NULL;
            uint64_t n = 0;
            int ok = fle_vfs_peek_file(w, &d, &n);
            sha_t sh;
            char hex[65];
            FILE *of;

            free(w);
            if (ok) {
                sha_init(&sh); sha_update(&sh, d, (size_t)n); sha_final(&sh, hex);
                of = fopen(a2, "wb");
                if (of) { if (n) fwrite(d, 1, (size_t)n, of); fclose(of); }
                printf("g3-vfsdump: %s -> %s: %" PRIu64 " bytes sha256 %s%s\n", a1, a2, n, hex, of ? "" : " (HOST FILE NOT WRITTEN)");
            } else printf("g3-vfsdump: %s: not a file on the virtual volume\n", a1);
            fflush(stdout);
            continue;
        }
#endif
        if (!strcmp(cmd, "probetimer") && nf >= 4) {
            int ok = fle_win_host_timer(FLE_WIN_HANDLE_BASE + 4u * (uint64_t)strtoul(a1, NULL, 0), strtoull(a2, NULL, 0), (uint32_t)strtoul(a3, NULL, 0));
            printf("g3: probetimer lid %s id %s %s ms: %s\n", a1, a2, a3, ok ? "set" : "NOT set");
            fflush(stdout);
            continue;
        }
        if (!strcmp(cmd, "open") && nf >= 2) {
            const char *p = ln + 4;
            int rq;
            while (*p == ' ') p++;
            rq = fle_appfile_open(p);
            printf("g8: open %s clock_ms %" PRIu64 " entries %" PRIu64 ": request %d\n", p, now / FLE_TICKS_PER_MS, trace_call_count(), rq);
            fflush(stdout);
            if (rq > 0) last_open_request = rq;
            return 1;
        }
#if !defined(__EMSCRIPTEN__) && defined(FLE_G3_FORKPROBE)
        if (!strcmp(cmd, "probe") && nf >= 2) {
            if (probe_begin(a1)) continue;
            {   /* decision 0215: skip to the MATCHING probe-end, so that a probe block may hold probe blocks (a state child that forks its probes); without nesting this is the old "next probe-end" */
                unsigned depth = 1;
                while (cur_line < nlines) {
                    const char *q = lines[cur_line++];
                    if (!strncmp(q, "probe-end", 9)) { if (--depth == 0) break; }
                    else if (!strncmp(q, "probe ", 6)) depth++;
                }
            }
            continue;
        }
        if (!strcmp(cmd, "probejobs") && nf >= 2) { probe_wait_all(); probe_jobs = (unsigned)atoi(a1); if (probe_jobs < 1) probe_jobs = 1; if (probe_jobs > 64) probe_jobs = 64; continue; }   /* decision 0215 */
        if (!strcmp(cmd, "waitprobes")) { probe_wait_all(); continue; }   /* decision 0215 */
        if (!strcmp(cmd, "setenv") && nf >= 2) { setenv(a1, nf >= 3 ? a2 : "1", 1); continue; }   /* decision 0171: e.g. "setenv FLE_SEH_TRACE 1" before a probe */
#if !defined(__EMSCRIPTEN__) && defined(FLE_G3_FORKPROBE)
        if (!strcmp(cmd, "callapi")) {   /* decision 0222 (native fork-probe builds only, a TEST FIXTURE, never a call of the guest): callapi NAME A1 .. A8 calls the runtime's answer of one user32 function with the numbers given (decimal or 0x..); prints "g3-callapi: NAME = result" */
            extern uint64_t fle_api2_call(void (*f)(void), uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t);
            static const struct { const char *n; void (*f)(void); } T[] = { { "SetWindowPos", fle_imp_SetWindowPos }, { "BringWindowToTop", fle_imp_BringWindowToTop }, { "ShowWindow", fle_imp_ShowWindow },
                { "SetActiveWindow", fle_imp_SetActiveWindow }, { "SetFocus", fle_imp_SetFocus }, { "DestroyWindow", fle_imp_DestroyWindow }, { "SetForegroundWindow", fle_imp_SetForegroundWindow } };
            char *save = NULL, *tok, buf[256];
            uint64_t v[8] = { 0 };
            unsigned k, nv = 0;
            const char *name = NULL;
            strncpy(buf, ln, sizeof buf - 1);
            buf[sizeof buf - 1] = 0;
            tok = strtok_r(buf, " \t\r\n", &save);   /* the command word */
            while ((tok = strtok_r(NULL, " \t\r\n", &save))) {
                if (!name) name = tok;
                else if (nv < 8) v[nv++] = strtoull(tok, NULL, 0);
            }
            for (k = 0; name && k < sizeof T / sizeof *T; k++)
                if (!strcmp(T[k].n, name)) {
                    uint64_t r = fle_api2_call(T[k].f, v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7]);
                    printf("g3-callapi: %s = %#llx\n", name, (unsigned long long)r);
                    fflush(stdout);
                    break;
                }
            if (!name || k == sizeof T / sizeof *T) printf("g3-callapi: unknown function\n");
            continue;
        }
#endif
        if (!strcmp(cmd, "wintrace")) { extern int fle_win_trace; fle_win_trace = nf > 1 && !strcmp(a1, "on"); continue; }   /* decision 0222: messages to window procedures and z-order moves on stderr */
        if (!strcmp(cmd, "importlog")) { extern int fruity_import_log; fruity_import_log = nf > 1 && !strcmp(a1, "on"); continue; }   /* decision 0171 */
        if (!strcmp(cmd, "probe-end")) {
            if (in_probe) {
                char nm[128];
                probe_wait_all();
                snprintf(nm, sizeof nm, "probe-%s-end", probe_name);
                g3_snapshot(nm);
                fflush(stdout);
                if (trace_cov_probe_end) trace_cov_probe_end();
                _exit(0);
            }
            continue;
        }
#endif
        if (!strcmp(cmd, "dump") && nf >= 2) { g8_dump(a1); continue; }
        if (!strcmp(cmd, "openstate")) {
            uint64_t res = 0;
            int st = fle_appfile_state(last_open_request, &res);
            printf("g8: openstate request %d state %d result %" PRIu64 " clock_ms %" PRIu64 "\n", last_open_request, st, res, fle_sys_clock_ticks() / FLE_TICKS_PER_MS);
            fflush(stdout);
            continue;
        }
        if (!strcmp(cmd, "apitrace")) { g3_api_trace = nf > 1 && !strcmp(a1, "on"); continue; }
        if (!strcmp(cmd, "timers")) { g3_timer_log(stdout); fflush(stdout); continue; }
        if (!strcmp(cmd, "end")) {
#if !defined(__EMSCRIPTEN__) && defined(FLE_G3_FORKPROBE)
            probe_wait_all();
#endif
            g3_snapshot("end"); g3_timer_log(stdout); fflush(stdout); return 0; }
        if (!strcmp(cmd, "advance") && nf >= 2) {
            pending_target = now + (uint64_t)strtoull(a1, NULL, 10) * FLE_TICKS_PER_MS;
            printf("g3: advance %s ms from clock_ms %" PRIu64 "\n", a1, now / FLE_TICKS_PER_MS);
            now = fle_sys_clock_ticks();
            continue;
        }
        if (!strcmp(cmd, "display") && nf >= 3) { printf("g3: display %s x %s -> queued %d\n", a1, a2, fle_input_host_display((uint32_t)atoi(a1), (uint32_t)atoi(a2))); return 1; }   /* decision 0212: the display changed to W H */
        if (!strcmp(cmd, "move") && nf >= 3) { fle_input_host_mouse(FLE_IN_MOVE, atoi(a1), atoi(a2), 0); return 1; }
        if (!strcmp(cmd, "down")) { int32_t x, y; fle_input_pointer(&x, &y); fle_input_host_mouse(FLE_IN_BDOWN, x, y, (int32_t)parse_btn(a1)); return 1; }
        if (!strcmp(cmd, "up")) { int32_t x, y; fle_input_pointer(&x, &y); fle_input_host_mouse(FLE_IN_BUP, x, y, (int32_t)parse_btn(a1)); return 1; }
        if (!strcmp(cmd, "click") && nf >= 3) {
            unsigned b = parse_btn(nf >= 4 ? a3 : NULL);
            int32_t x = atoi(a1), y = atoi(a2);
            fle_input_host_mouse(FLE_IN_MOVE, x, y, 0);
            fle_input_host_mouse(FLE_IN_BDOWN, x, y, (int32_t)b);
            fle_input_host_mouse(FLE_IN_BUP, x, y, (int32_t)b);
            return 1;
        }
        if (!strcmp(cmd, "wheel") && nf >= 4) {   /* decision 0191: wheel X Y DELTA (a multiple of 120; positive = away from the user) at the screen point; the host pointer moves there first */
            int32_t x = atoi(a1), y = atoi(a2);
            fle_input_host_mouse(FLE_IN_MOVE, x, y, 0);
            fle_input_host_mouse(FLE_IN_WHEEL, x, y, atoi(a3));
            return 1;
        }
        if (!strcmp(cmd, "key") && nf >= 3) {
            fle_input_host_key(!strcmp(a1, "down") ? FLE_IN_KDOWN : FLE_IN_KUP, (uint32_t)strtoul(a2, NULL, 0), nf >= 4 ? (uint32_t)strtoul(a3, NULL, 0) : 0u, nf >= 5 ? atoi(a4) : 0);
            return 1;
        }
        if (!strcmp(cmd, "taskbar") && nf >= 2) {   /* decision 0267: taskbar restore|minimize|maximize|close: the task bar presses the first button whose window is in the state the command needs (restore: a minimized one) */
            fle_taskbar_button tb[16];
            unsigned n = fle_win_taskbar_buttons(tb, 16), k;
            uint32_t sc = !strcmp(a1, "restore") ? 0xF120u : !strcmp(a1, "minimize") ? 0xF020u : !strcmp(a1, "maximize") ? 0xF030u : 0xF060u;

            for (k = 0; k < n; k++) {
                if (sc == 0xF120u && !tb[k].iconic) continue;
                if (sc == 0xF020u && tb[k].iconic) continue;
                printf("g3: taskbar %s: button lid %u hwnd %#llx iconic %u zoomed %u -> queued %d\n", a1, tb[k].lid, (unsigned long long)tb[k].hwnd, tb[k].iconic, tb[k].zoomed, fle_input_host_syscommand(tb[k].hwnd, sc));
                break;
            }
            if (k == n) printf("g3: taskbar %s: no button\n", a1);
            return 1;
        }
        if (!strcmp(cmd, "tap") && nf >= 2) {
            uint32_t vk = (uint32_t)strtoul(a1, NULL, 0);
            fle_input_host_key(FLE_IN_KDOWN, vk, 0, 0);
            fle_input_host_key(FLE_IN_KUP, vk, 0, 0);
            return 1;
        }
        fprintf(stderr, "g3: unknown script command '%s'\n", ln);
        exit(2);
    }
}

#ifndef __EMSCRIPTEN__
/* The native idle: the script, or (no script) the clock jump of decision 0122/0129 with FLE_FRUITY_NO_IDLE_STOP=1. */
int fle_thread_idle_hook(uint64_t now, uint64_t next, uint64_t *advance_to) {
    int r = g3_script_idle(now, next, advance_to);
    if (r >= 0) return r;
    if (!next) return 0;
    *advance_to = next;
    return 2;
}
#endif
