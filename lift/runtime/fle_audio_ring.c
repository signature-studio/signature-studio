/* The audio output ring and the single source of the play position (decision 0139, PROVISIONAL). See fle_audio_ring.h. Standard C only (no host clock, no thread, no file): the virtual clock is fle_sys_clock_ticks(). */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fle_audio_ring.h"
#include "fle_audio_transport.h"
#include "fle_test_env.h"
#include "fle_sys.h"

#ifdef __EMSCRIPTEN__
#include <emscripten.h>
#define FLE_AUDIO_EXPORT EMSCRIPTEN_KEEPALIVE
#else
#define FLE_AUDIO_EXPORT
#endif

#define DATA_BYTES ((size_t)FLE_AUDIO_RING_FRAMES * FLE_AUDIO_RING_CHANNELS * sizeof(float))

static fle_audio_ring_t *ring;
static void pace_epoch_reset(void);

/* the transport (fle_audio_transport.c: wasm, JavaScript embedded there; native: nothing) */
static double js_consumed(void) { return fle_tp_consumed(); }
static void js_publish(const float *lr, uint32_t n, double first, uint32_t rate) { if (fle_audio_external()) fle_tp_publish(lr, n, first, rate); }
static void js_state(uint32_t state, uint32_t rate) { if (fle_audio_external()) fle_tp_state(state, rate); }
static void js_rewrite(const float *lr, uint32_t n, double first, uint32_t guard) { if (fle_audio_external()) fle_tp_rewrite(lr, n, first, guard); }
static double js_now(void) { return fle_tp_now(); }
static int js_wait(double ms) { return fle_tp_wait(ms); }
static int js_can_wait(void) { return fle_tp_can_wait(); }

/* The device clock (native source): frames = acc + (now - seg_t0) * rate / 10^7 for the current segment; a change of rate starts a new segment. */
static int clock_started;
static uint64_t seg_t0, acc_frames;

fle_audio_ring_t *fle_audio_ring_get(void)
{
    if (!ring) {
        ring = calloc(1, sizeof *ring + DATA_BYTES);
        if (!ring) {
            fputs("fle: audio ring: out of memory\n", stderr);
            abort();
        }
        ring->magic = FLE_AUDIO_RING_MAGIC;
        ring->version = FLE_AUDIO_RING_VERSION;
        ring->channels = FLE_AUDIO_RING_CHANNELS;
        ring->capacity = FLE_AUDIO_RING_FRAMES;
        ring->data_offset = (uint32_t)sizeof *ring;
        ring->state = FLE_AUDIO_CLOSED;
        ring->block_log_len = FLE_AUDIO_BLOCK_LOG;
        ring->external_consumer = fle_audio_external() ? 1u : 0u;
    }
    return ring;
}

float *fle_audio_ring_data(void) { return (float *)((char *)fle_audio_ring_get() + sizeof(fle_audio_ring_t)); }

FLE_AUDIO_EXPORT uint32_t fle_audio_ring(void) { return (uint32_t)(uintptr_t)fle_audio_ring_get(); }

void fle_audio_ring_set_state(uint32_t state)
{
    fle_audio_ring_t *r = fle_audio_ring_get();

    if (r->state == state) return;
    if (state == FLE_AUDIO_PLAYING) pace_epoch_reset();
    r->state = state;
    js_state(state, r->sample_rate);
}

void fle_audio_ring_open(uint32_t rate)
{
    fle_audio_ring_t *r = fle_audio_ring_get();

    if (clock_started && r->sample_rate != 0 && r->sample_rate != rate) {   /* new segment of the device clock at the new rate */
        acc_frames = fle_audio_consumed();
        seg_t0 = fle_sys_clock_ticks();
    }
    r->sample_rate = rate;
    if (r->state == FLE_AUDIO_CLOSED) fle_audio_ring_set_state(FLE_AUDIO_OPEN);
    else js_state(r->state, rate);
}

void fle_audio_clock_start(void)
{
    if (clock_started) return;
    clock_started = 1;
    seg_t0 = fle_sys_clock_ticks();
    acc_frames = 0;
}

uint64_t fle_audio_consumed(void)
{
    fle_audio_ring_t *r = fle_audio_ring_get();
    uint64_t now;

    if (fle_audio_external()) {
        double d = js_consumed();

        return d <= 0 ? 0 : (uint64_t)d;
    }
    if (!clock_started) return 0;
    now = fle_sys_clock_ticks();
    if (now <= seg_t0) return acc_frames;
    return acc_frames + (now - seg_t0) * (uint64_t)r->sample_rate / FLE_QPF_HZ;
}

uint64_t fle_audio_tick_of_frame(uint64_t frames)
{
    fle_audio_ring_t *r = fle_audio_ring_get();
    uint64_t rel, num;

    if (fle_audio_external() || !clock_started || r->sample_rate == 0) return 0;
    if (frames <= acc_frames) return seg_t0;
    rel = frames - acc_frames;
    num = rel * FLE_QPF_HZ;
    return seg_t0 + (num + r->sample_rate - 1u) / r->sample_rate;   /* the first tick at which (now - seg_t0) * rate / 10^7 >= rel */
}

/* The lead in ms: FLE_AUDIO_LEAD_MS (60, S-audio-lead; Learn's "typically about 15 milliseconds" is the other candidate). Decision 0243: the page no longer passes another value; the native scenarios of decision 0139 give 15 through the environment (FLE_TEST_GETENV: the wasm product reads none).
 * A longer lead lets the output ride out a longer stall of the engine, and costs the engine time to rewrite the half it has just left: the output reads that far ahead of the play cursor, and a region read ahead cannot be changed any more. */
static uint32_t lead_ms_cached;
static uint32_t lead_ms(void)
{
    if (lead_ms_cached == 0) {
        const char *e = FLE_TEST_GETENV("FLE_AUDIO_LEAD_MS");
        uint32_t ms = e ? (uint32_t)atoi(e) : 0;

        lead_ms_cached = ms >= 1 && ms <= 1000 ? ms : FLE_AUDIO_LEAD_MS;
    }
    return lead_ms_cached;
}

uint32_t fle_audio_lead_frames(void)
{
    fle_audio_ring_t *r = fle_audio_ring_get();

    return (uint32_t)((uint64_t)r->sample_rate * lead_ms() / 1000u);
}

uint32_t fle_audio_ring_fill(void)
{
    fle_audio_ring_t *r = fle_audio_ring_get();

    return r->write_frames > r->read_frames ? (uint32_t)(r->write_frames - r->read_frames) : 0u;
}

uint32_t fle_audio_ring_append(const float *lr, uint32_t frames)
{
    fle_audio_ring_t *r = fle_audio_ring_get();
    float *d = fle_audio_ring_data();
    uint64_t first = r->write_frames;
    uint64_t used = first > r->read_frames ? first - r->read_frames : 0u;
    uint32_t room, take, k;
    fle_audio_block *b;

    if (frames == 0) return 0;
    room = used >= r->capacity ? 0u : (uint32_t)(r->capacity - used);
    take = frames < room ? frames : room;
    if (take < frames) r->overruns += frames - take;
    for (k = 0; k < take; k++) {
        size_t at = (size_t)((first + k) % r->capacity) * FLE_AUDIO_RING_CHANNELS;

        d[at] = lr[2u * k];
        d[at + 1] = lr[2u * k + 1u];
    }
    b = &r->blocks[r->blocks_total % FLE_AUDIO_BLOCK_LOG];
    b->first_frame = first;
    b->frames = take;
    b->virt_ticks = fle_sys_clock_ticks();
    b->consumed = fle_audio_consumed();
    r->blocks_total++;
    r->write_frames = first + take;
    if (take) js_publish(lr, take, (double)first, r->sample_rate);
    return take;
}

/* S-ds-rewrite-guard: 256 frames = two quanta of the AudioWorklet (128 each: the one it may be reading now and the next), 5.8 ms at 44.1 kHz. Alternatives: 128 (one quantum), 512. */
#define REWRITE_GUARD_FRAMES 256u
uint32_t fle_audio_rewrite_guard_frames(void) { return REWRITE_GUARD_FRAMES; }

void fle_audio_ring_rewrite(const float *lr, uint32_t frames, uint64_t first)
{
    if (frames) js_rewrite(lr, frames, (double)first, REWRITE_GUARD_FRAMES);
}

/* what the late writes did, for the report of the page (0 events: Unlocks that wrote into frames the device had already handed to the output, 1 frames of them that were still in time and were handed over again, 2 frames of them that
 * were within the guard of the read position and could not be changed any more) */
static double rw_events, rw_applied, rw_lost;
void fle_audio_rewrite_note(uint32_t applied, uint32_t lost)
{
    rw_events += 1.0;
    rw_applied += applied;
    rw_lost += lost;
}
FLE_AUDIO_EXPORT double fle_audio_rewrite_stat(unsigned k) { return k == 0 ? rw_events : k == 1 ? rw_applied : k == 2 ? rw_lost : 0.0; }

void fle_audio_ring_sync_consumer(void)
{
    fle_audio_ring_t *r = fle_audio_ring_get();
    uint64_t c = fle_audio_consumed();

    r->external_consumer = fle_audio_external() ? 1u : 0u;
    if (c > r->read_frames) r->read_frames = c;
}

/* ---- pacing ---- */
static int pace_init;
static double pace_real0;
static uint64_t pace_virt0;
static double st_waits, st_wait_ms, st_late, st_late_ms, st_late_max, st_stretch_max, st_over20, st_over50, st_over78;
static double last_return_ms = -1.0;

int fle_audio_paced(void)
{
    fle_audio_ring_t *r = fle_audio_ring_get();

    return js_can_wait() && fle_audio_external() && r->state == FLE_AUDIO_PLAYING;
}

FLE_AUDIO_EXPORT double fle_audio_pace_stat(unsigned k)
{
    switch (k) {
    case 0: return st_waits;
    case 1: return st_wait_ms;
    case 2: return st_late;
    case 3: return st_late_ms;
    case 4: return st_late_max;
    case 5: return st_stretch_max;
    case 6: return st_over20;
    case 7: return st_over50;
    case 8: return st_over78;
    default: return 0.0;
    }
}

int fle_audio_wait_until(uint64_t dl)
{
    double now = js_now(), due, wait;
    uint64_t v = fle_sys_clock_ticks();

    if (!pace_init) {   /* the epoch: this moment is virtual time v */
        pace_init = 1;
        pace_real0 = now;
        pace_virt0 = v;
        last_return_ms = now;
    }
    if (last_return_ms >= 0.0) {   /* how long the engine ran since it last returned from a wait: a stretch that the mixer's deadline could not interrupt */
        double stretch = now - last_return_ms;

        if (stretch > st_stretch_max) st_stretch_max = stretch;
        if (stretch > 20.0) st_over20++;
        if (stretch > 50.0) st_over50++;
        if (stretch > 78.0) st_over78++;
    }
    due = pace_real0 + (double)(dl - pace_virt0) / 10000.0;
    wait = due - now;
    if (wait <= 0.0) {   /* the engine is behind real time: no wait; the lateness is the measure of how far */
        st_late++;
        st_late_ms += -wait;
        if (-wait > st_late_max) st_late_max = -wait;
        last_return_ms = now;
        return 2;
    }
    if (wait < 0.05) {
        last_return_ms = now;
        return 2;
    }
    {
        int rc = js_wait(wait);
        double after = js_now();

        st_waits++;
        st_wait_ms += after - now;
        last_return_ms = after;
        return rc;
    }
}

static void pace_epoch_reset(void) { pace_init = 0; last_return_ms = -1.0; }
