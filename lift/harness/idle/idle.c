/* The stable-idle detector (idle.h; decision 0129, PROVISIONAL). Harness only. The core (SHA-256, idle_track_*) is pure C; the part under FLE_IDLE reads the runtime (window table, drawing surfaces, virtual clock,
 * the counters of the stubs) and is called by fruity.c. */
#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "idle/idle.h"

/* ---- SHA-256 (FIPS 180-4), streaming ---- */
static const uint32_t K[64] = {
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
};

static uint32_t ror(uint32_t x, unsigned r) { return (x >> r) | (x << (32u - r)); }

static void block(uint32_t h[8], const uint8_t *p)
{
    uint32_t w[64], a, b, c, d, e, f, g, hh;
    unsigned i;

    for (i = 0; i < 16; i++) w[i] = ((uint32_t)p[4 * i] << 24) | ((uint32_t)p[4 * i + 1] << 16) | ((uint32_t)p[4 * i + 2] << 8) | (uint32_t)p[4 * i + 3];
    for (i = 16; i < 64; i++) {
        uint32_t s0 = ror(w[i - 15], 7) ^ ror(w[i - 15], 18) ^ (w[i - 15] >> 3);
        uint32_t s1 = ror(w[i - 2], 17) ^ ror(w[i - 2], 19) ^ (w[i - 2] >> 10);

        w[i] = w[i - 16] + s0 + w[i - 7] + s1;
    }
    a = h[0]; b = h[1]; c = h[2]; d = h[3]; e = h[4]; f = h[5]; g = h[6]; hh = h[7];
    for (i = 0; i < 64; i++) {
        uint32_t t1 = hh + (ror(e, 6) ^ ror(e, 11) ^ ror(e, 25)) + ((e & f) ^ (~e & g)) + K[i] + w[i];
        uint32_t t2 = (ror(a, 2) ^ ror(a, 13) ^ ror(a, 22)) + ((a & b) ^ (a & c) ^ (b & c));

        hh = g; g = f; f = e; e = d + t1; d = c; c = b; b = a; a = t1 + t2;
    }
    h[0] += a; h[1] += b; h[2] += c; h[3] += d; h[4] += e; h[5] += f; h[6] += g; h[7] += hh;
}

void idle_sha_init(idle_sha *c)
{
    static const uint32_t iv[8] = { 0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19 };

    memcpy(c->h, iv, sizeof iv);
    c->len = 0;
    c->fill = 0;
}

void idle_sha_add(idle_sha *c, const void *data, size_t n)
{
    const uint8_t *p = (const uint8_t *)data;

    c->len += n;
    if (c->fill) {
        size_t take = 64u - c->fill < n ? 64u - c->fill : n;

        memcpy(c->buf + c->fill, p, take);
        c->fill += (uint32_t)take;
        p += take;
        n -= take;
        if (c->fill < 64u) return;
        block(c->h, c->buf);
        c->fill = 0;
    }
    while (n >= 64u) {
        block(c->h, p);
        p += 64;
        n -= 64;
    }
    if (n) {
        memcpy(c->buf, p, n);
        c->fill = (uint32_t)n;
    }
}

void idle_sha_final(idle_sha *c, uint8_t out[32])
{
    uint64_t bits = c->len * 8u;
    uint8_t pad[72];
    size_t padn = (c->fill < 56u) ? 56u - c->fill : 120u - c->fill;
    unsigned i;

    memset(pad, 0, sizeof pad);
    pad[0] = 0x80;
    for (i = 0; i < 8; i++) pad[padn + i] = (uint8_t)(bits >> (56 - 8 * i));
    idle_sha_add(c, pad, padn + 8);
    for (i = 0; i < 8; i++) {
        out[4 * i] = (uint8_t)(c->h[i] >> 24);
        out[4 * i + 1] = (uint8_t)(c->h[i] >> 16);
        out[4 * i + 2] = (uint8_t)(c->h[i] >> 8);
        out[4 * i + 3] = (uint8_t)c->h[i];
    }
}

void idle_hex(const uint8_t d[32], char out[65])
{
    unsigned i;

    for (i = 0; i < 32; i++) snprintf(out + 2 * i, 3, "%02x", (unsigned)d[i]);
}

/* ---- the tracker ---- */
void idle_track_init(idle_track *t, uint64_t need_ms)
{
    memset(t, 0, sizeof *t);
    t->need_ms = need_ms;
}

static int same(const idle_snap *a, const idle_snap *b)
{
    return a->distinct == b->distinct && a->apis == b->apis && memcmp(a->win, b->win, 32) == 0 && memcmp(a->surf, b->surf, 32) == 0;
}

int idle_track_feed(idle_track *t, const idle_snap *s)
{
    uint64_t need;

    t->n_obs++;
    if (!t->have) {
        t->base = *s;
        t->have = 1;
    } else if (!same(&t->base, s)) {
        t->base = *s;
        t->n_resets++;
    }
    t->last = *s;
    /* S-stable-timer-period (decision 0129): a window timer that is live must have had its period run out inside the stable stretch, otherwise WM_TIMER was not delivered to a state that could change: the stretch is at least N
     * seconds and at least the longest period of a live timer. */
    need = t->need_ms;
    if (s->max_period_ms > need) need = s->max_period_ms;
    return s->ms >= t->base.ms && s->ms - t->base.ms >= need;
}

const idle_snap *idle_track_base(const idle_track *t) { return t->have ? &t->base : NULL; }

#ifdef FLE_IDLE
#include <errno.h>
#include <signal.h>
#include <unistd.h>

#include "fle_gdi.h"
#include "fle_runtime.h"
#include "fle_raster.h"
#include "fle_sys.h"
#include "fle_thread.h"
#include "fle_win.h"
#include "a4.h"

extern const char *const idle_wrap_names[];
extern const unsigned idle_wrap_count;

#define ULL(x) ((unsigned long long)(x))
#define SAMPLE_EVERY UINT64_C(65536)
#define MAX_API 4096

static uint64_t cfg_stable_ms, cfg_virtual_ms;
static int on;
static idle_track trk;
static uint64_t clock0_ms, distinct0, calls0, next_sample, samples;
static uint8_t seen_api[MAX_API];
static uint64_t n_apis;
static char first_new_api[64][48];   /* the first API names seen (for the report: what is seen for the first time during the run) */
static unsigned n_first_new;
static uint64_t calls_since;
static uint8_t trace_api[MAX_API];   /* FLE_FRUITY_API_TRACE=<API>[,<API>...]: the entries into these stubs are printed to stderr with the virtual clock and the four register arguments (information only) */

static int parse_ms(const char *name, uint64_t *out)
{
    const char *s = getenv(name);
    char *end;
    unsigned long long v;

    if (!s || !*s) return 0;
    errno = 0;
    v = strtoull(s, &end, 10);
    if (errno != 0 || end == s || *end != 0 || v < 1 || v > 1000000ull) {
        fprintf(stderr, "fruity: %s=\"%s\" is not a decimal number of seconds from 1 to 1000000\n", name, s);
        return -1;
    }
    *out = (uint64_t)v * 1000u;
    return 1;
}

int idle_configure(void)
{
    int a = parse_ms("FLE_FRUITY_STABLE_SECONDS", &cfg_stable_ms);
    int b = parse_ms("FLE_FRUITY_VIRTUAL_SECONDS", &cfg_virtual_ms);

    if (a < 0 || b < 0) return -1;
    if (b > 0 && a == 0) {
        fprintf(stderr, "fruity: FLE_FRUITY_VIRTUAL_SECONDS is set without FLE_FRUITY_STABLE_SECONDS: refused\n");
        return -1;
    }
    on = a > 0;
    if (on && cfg_virtual_ms == 0) cfg_virtual_ms = 3600u * 1000u;   /* S-virtual-limit: one virtual hour when unset */
    if (on) {
        const char *t = getenv("FLE_FRUITY_API_TRACE");
        unsigned k;

        if (t && *t) {
            char buf[512], *tok, *save = NULL;

            snprintf(buf, sizeof buf, "%s", t);
            for (tok = strtok_r(buf, ",", &save); tok; tok = strtok_r(NULL, ",", &save)) {
                int hit = 0;

                for (k = 0; k < idle_wrap_count && k < MAX_API; k++)
                    if (strcmp(idle_wrap_names[k], tok) == 0) {
                        trace_api[k] = 1;
                        hit = 1;
                    }
                if (!hit) {
                    fprintf(stderr, "fruity: FLE_FRUITY_API_TRACE names \"%s\", which is not a stub of the runtime\n", tok);
                    return -1;
                }
            }
        }
    }
    return on;
}

int idle_enabled(void) { return on; }
uint64_t idle_virtual_limit_ms(void) { return cfg_virtual_ms; }

void idle_api_note(unsigned idx)
{
    if (!on || idx >= MAX_API) return;
    if (trace_api[idx]) {
        const char *nm = idle_wrap_names[idx];
        char txt[200];

        txt[0] = 0;
        /* the first string argument, when the API is known to take one (a UTF-16 path or name in rcx; a byte buffer in rdx for WriteFile (length r8)); information only */
        if (strcmp(nm, "CreateFileW") == 0 || strcmp(nm, "GetFileAttributesW") == 0 || strcmp(nm, "FindFirstFileW") == 0 || strcmp(nm, "CreateDirectoryW") == 0 || strcmp(nm, "DeleteFileW") == 0 || strcmp(nm, "OutputDebugStringW") == 0 || strcmp(nm, "GetFileVersionInfoSizeW") == 0 || strcmp(nm, "GetFileVersionInfoW") == 0 || strcmp(nm, "GetModuleFileNameW") == 0 ||
            strcmp(nm, "SetWindowTextW") == 0 || strcmp(nm, "CreateWindowExW") == 0 || strcmp(nm, "RegOpenKeyExW") == 0 || strcmp(nm, "RegQueryValueExW") == 0 || strcmp(nm, "LoadLibraryW") == 0 || strcmp(nm, "MessageBoxW") == 0) {
            /* the text argument: rcx for most, rdx for SetWindowTextW / RegQueryValueExW, r8 for CreateWindowExW (lpWindowName; the class name in rdx is printed after it when it is a pointer) */
            uint64_t p = strcmp(nm, "SetWindowTextW") == 0 || strcmp(nm, "RegQueryValueExW") == 0 ? R[2] : strcmp(nm, "CreateWindowExW") == 0 ? R[8] : R[1];
            size_t k;

            for (k = 0; p > 0xFFFF && k + 1 < sizeof txt && k < 190; k++) {
                uint16_t u = (uint16_t)fle_ld(p + 2u * k, 2);

                if (!u) break;
                txt[k] = (u >= 0x20 && u < 0x7f) ? (char)u : '?';
            }
            txt[k] = 0;
        } else if (strcmp(nm, "WriteFile") == 0) {
            uint64_t p = R[2], n = (uint32_t)R[8];
            size_t k;

            for (k = 0; k + 1 < sizeof txt && k < n && k < 190; k++) {
                uint8_t b = (uint8_t)fle_ld(p + k, 1);

                txt[k] = (b >= 0x20 && b < 0x7f) ? (char)b : '.';
            }
            txt[k] = 0;
        }
        if (strcmp(nm, "CreateWindowExW") == 0 && R[2] > 0xFFFF) {
            char cls[80];
            size_t k;

            for (k = 0; k + 1 < sizeof cls && k < 70; k++) {
                uint16_t u = (uint16_t)fle_ld(R[2] + 2u * k, 2);

                if (!u) break;
                cls[k] = (u >= 0x20 && u < 0x7f) ? (char)u : '?';
            }
            cls[k] = 0;
            fprintf(stderr, "idle-trace: (CreateWindowExW class name \"%s\")\n", cls);
        }
        fprintf(stderr, "idle-trace: %s clock=%llu ms thread=%u args=%#llx %#llx %#llx %#llx entries=%llu%s%s%s\n", nm, ULL(fle_sys_clock_ticks() / FLE_TICKS_PER_MS), fle_thread_current(), ULL(R[1]), ULL(R[2]), ULL(R[8]),
                ULL(R[9]), ULL(trace_call_count() - calls0), txt[0] ? " text=\"" : "", txt, txt[0] ? "\"" : "");
    }
    if (!seen_api[idx]) {
        seen_api[idx] = 1;
        n_apis++;
        if (calls_since && n_first_new < 64u && idx < idle_wrap_count) {
            snprintf(first_new_api[n_first_new], sizeof first_new_api[0], "%s", idle_wrap_names[idx]);
            n_first_new++;
        }
    }
}

void idle_begin(uint64_t clock_ms, uint64_t d0)
{
    clock0_ms = clock_ms;
    distinct0 = d0;
    calls0 = trace_call_count();
    idle_track_init(&trk, cfg_stable_ms);
    next_sample = SAMPLE_EVERY;
    calls_since = 0;
}

static void win_hash(uint8_t out[32])
{
    char *buf = NULL;
    size_t len = 0;
    FILE *m = open_memstream(&buf, &len);
    idle_sha c;

    idle_sha_init(&c);
    if (m) {
        fle_win_table_report(m);
        fclose(m);
        idle_sha_add(&c, buf, len);
        free(buf);
    }
    idle_sha_final(&c, out);
}

static void surf_hash(uint8_t out[32])
{
    uint32_t lids[256], n = fle_raster_live_lids(lids, 256), i, y;
    idle_sha c;

    idle_sha_init(&c);
    for (i = 0; i < n; i++) {
        const fle_rsurf *s = fle_raster_surface_peek(lids[i]);
        uint32_t hdr[3];

        if (!s) continue;
        hdr[0] = lids[i];
        hdr[1] = s->width;
        hdr[2] = s->height;
        idle_sha_add(&c, hdr, sizeof hdr);
        for (y = 0; y < s->height; y++) idle_sha_add(&c, fle_r_row(s, y), (size_t)s->width * (s->bpp / 8u));
    }
    idle_sha_final(&c, out);
}

static void take(idle_snap *s)
{
    memset(s, 0, sizeof *s);
    s->ms = fle_sys_clock_ticks() / FLE_TICKS_PER_MS;
    s->distinct = trace_distinct_count();
    s->apis = n_apis;
    s->calls = trace_call_count() - calls0;
    s->live_timers = fle_win_timer_live_count();
    s->max_period_ms = fle_win_timer_max_period_ms();
    fle_win_synth_counts(&s->timers, &s->paints);
    win_hash(s->win);
    surf_hash(s->surf);
}

int idle_tick(void)
{
    idle_snap s;
    uint64_t c;

    if (!on) return 0;
    c = trace_call_count();
    calls_since++;
    if (calls_since < next_sample) return 0;
    next_sample = calls_since + SAMPLE_EVERY;
    (void)c;
    take(&s);
    samples++;
    if (idle_track_feed(&trk, &s)) return 1;
    if (s.ms - clock0_ms >= cfg_virtual_ms) return 2;
    return 0;
}

void idle_describe(FILE *f)
{
    const idle_snap *b = idle_track_base(&trk);
    char wh[65], sh[65];
    unsigned i;

    fprintf(f, "idle: N = %llu virtual s (FLE_FRUITY_STABLE_SECONDS), virtual limit %llu s (FLE_FRUITY_VIRTUAL_SECONDS); %llu samples (one every %llu lifted entries), the run was restarted %llu times\n", ULL(cfg_stable_ms / 1000),
            ULL(cfg_virtual_ms / 1000), ULL(samples), ULL(SAMPLE_EVERY), ULL(trk.n_resets));
    if (!b) return;
    idle_hex(trk.last.win, wh);
    idle_hex(trk.last.surf, sh);
    fprintf(f, "idle: the stable run began at virtual %llu ms (%llu ms after the call began, at entry %llu) and the last sample is at %llu ms (entry %llu): %llu ms stable; new functions since the call began %llu, API names %llu, WM_TIMER made %llu, WM_PAINT made %llu, live window timers %u (longest period %llu ms)\n",
            ULL(b->ms), ULL(b->ms - clock0_ms), ULL(b->calls), ULL(trk.last.ms), ULL(trk.last.calls), ULL(trk.last.ms - b->ms), ULL(trk.last.distinct - distinct0), ULL(trk.last.apis), ULL(trk.last.timers), ULL(trk.last.paints),
            (unsigned)trk.last.live_timers, ULL(trk.last.max_period_ms));
    fprintf(f, "idle: window table SHA-256 %s; drawing surfaces SHA-256 %s\n", wh, sh);
    fprintf(f, "idle: API names entered for the first time after the call began (up to 64):");
    for (i = 0; i < n_first_new; i++) fprintf(f, " %s", first_new_api[i]);
    fprintf(f, "%s\n", n_first_new ? "" : " (none)");
}
#endif
