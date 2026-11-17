/* The stable-idle detector of the idle continuation (decision 0129, PROVISIONAL; docs/decisions/0129-idle-continuation.md). Harness only, native only.
 *
 * Stop condition (b') of the task: with the virtual clock moved on (the scheduler of fle_thread.c moves it to the smallest deadline when every thread waits: decision 0122), the guest runs only periodic code. It is
 * STABLE when, over N consecutive virtual seconds, (1) no new lifted function was entered, (2) no API name was entered for the first time, and (3) the observable state of the guest did not change: the SHA-256 of the
 * window table (fle_win_table_report: class, title, rectangle, styles, parent, owner, visible, enabled, z position of every window) and of the drawing surfaces (every live window surface: lid, size and the pixel rows).
 * N is FLE_FRUITY_STABLE_SECONDS (a CHOICE WITHOUT A SOURCE, S-stable-n); the virtual clock is limited by FLE_FRUITY_VIRTUAL_SECONDS (a CHOICE WITHOUT A SOURCE, S-virtual-limit).
 *
 * Only a build with FLE_IDLE (lift/harness/two/build_two_native.sh IDLE=1: fruity.c compiled with -DFLE_IDLE=1, this file, and one -Wl,--wrap=fle_imp_<API> per runtime stub made by idle/gen_idle_wraps.py) contains it;
 * the wrappers only count: they never change what a stub does. A build without IDLE=1 has none of this. The core (idle_track_*) is pure C with no runtime dependency so that tests/wall_fr12.rs can drive it. */
#ifndef FLE_HARNESS_IDLE_H
#define FLE_HARNESS_IDLE_H

#include <stdint.h>
#include <stdio.h>

/* One observation of the guest. `ms` is the virtual clock in milliseconds. The two SHA-256 are of the window table and of the drawing surfaces. */
typedef struct {
    uint64_t ms;
    uint64_t distinct;         /* distinct lifted functions entered so far (trace_distinct_count) */
    uint64_t apis;             /* distinct API names entered so far */
    uint64_t timers, paints;   /* WM_TIMER and WM_PAINT messages the window model has made so far (information only: they do not decide) */
    uint64_t calls;            /* lifted entries since the call began (information only) */
    uint32_t live_timers;      /* window timers (SetTimer) that are live now */
    uint64_t max_period_ms;    /* the longest period among them (0 when none): the stable stretch must be at least this long (S-stable-timer-period) */
    uint8_t win[32], surf[32];
} idle_snap;

typedef struct {
    uint64_t need_ms;          /* N seconds in ms */
    int have;
    idle_snap base;            /* the first observation of the current stable run */
    idle_snap last;
    uint64_t n_obs, n_resets;
} idle_track;

void idle_track_init(idle_track *t, uint64_t need_ms);
/* Feeds one observation (virtual time must not go backward). Returns 1 when the guest has been stable for at least max(need_ms, the longest live timer period of `s`): the clock of `s` is that long after the first observation whose function count, API count and two
 * SHA-256 equal those of `s`, and all observations in between were equal too (a change restarts the run at the observation that changed). */
int idle_track_feed(idle_track *t, const idle_snap *s);
/* The observation that began the current stable run, and how many times the run was restarted. */
const idle_snap *idle_track_base(const idle_track *t);

/* SHA-256 (FIPS 180-4), streaming. */
typedef struct {
    uint32_t h[8];
    uint8_t buf[64];
    uint64_t len;
    uint32_t fill;
} idle_sha;
void idle_sha_init(idle_sha *c);
void idle_sha_add(idle_sha *c, const void *data, size_t n);
void idle_sha_final(idle_sha *c, uint8_t out[32]);
void idle_hex(const uint8_t d[32], char out[65]);

#ifdef FLE_IDLE
/* The harness side (fruity.c). idle_configure reads FLE_FRUITY_STABLE_SECONDS / FLE_FRUITY_VIRTUAL_SECONDS (0 = both unset, -1 = a bad value, message on stderr); idle_enabled is 1 when a stable run is asked for. */
int idle_configure(void);
int idle_enabled(void);
uint64_t idle_virtual_limit_ms(void);
/* The call of CreateFruityInstance begins: the virtual clock (ms) and the distinct-function count at that moment. */
void idle_begin(uint64_t clock_ms, uint64_t distinct0);
/* The lifted-entry hook of fruity.c calls this every time; it samples every 65,536 entries. 1 = stable (the caller prints the report and stops), 2 = the virtual-clock limit was reached, 0 = go on. */
int idle_tick(void);
/* An API name entered for the first time / a wait API entered (called from the generated wrappers). */
void idle_api_note(unsigned idx);
/* The text of the stable run for the report. */
void idle_describe(FILE *f);
#endif

#endif
