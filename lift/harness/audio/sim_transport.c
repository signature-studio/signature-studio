/* The transport of the audio output for the scenarios of decision 0241 (harness/audio/ds_scenarios.c): the stand-in for the AudioWorklet and its ring. It has the functions of runtime/fle_audio_transport.h and the two notes of fle_audio_ring.h, and is
 * linked in place of runtime/fle_audio_transport.c (the product's native version has no external consumer). The play position is the count that the scenario sets (fle_sim_consumed), or the virtual clock at the device's rate after fle_sim_follow_clock; the "page's ring" is the C ring of fle_audio_ring.c itself (what the
 * device appends is what the output reads), so a rewrite is a write into that ring of the frames that JavaScript's fle_js_rewrite would write: only the ones at least `guard` frames ahead of the read position and before the last handed-over frame. */
#include <stddef.h>
#include <stdint.h>

#include "fle_audio_ring.h"
#include "fle_audio_transport.h"
#include "fle_sys.h"

static int sim_ext, sim_follow;
static uint64_t sim_consumed, sim_offered_inside_guard, sim_t0;

void fle_sim_external(int on) { sim_ext = on; }
void fle_sim_consumed(uint64_t frames) { sim_consumed = frames; }
/* the play position follows the virtual clock from tick t0 at the device's rate (what the AudioWorklet reads in real time), instead of the count that fle_sim_consumed sets */
void fle_sim_follow_clock(uint64_t t0) { sim_follow = 1; sim_t0 = t0; }
/* frames the runtime offered to the transport that are nearer to the read position than the guard (the JavaScript clips them as well; the runtime must not offer them) */
uint64_t fle_sim_offered_inside_guard(void) { return sim_offered_inside_guard; }

int fle_audio_external(void) { return sim_ext; }
double fle_tp_consumed(void)
{
    uint64_t now;

    if (!sim_follow) return (double)sim_consumed;
    now = fle_sys_clock_ticks();
    return now <= sim_t0 ? 0.0 : (double)((now - sim_t0) * fle_audio_ring_get()->sample_rate / FLE_QPF_HZ);
}
void fle_tp_publish(const float *lr, uint32_t n, double first, uint32_t rate) { (void)lr; (void)n; (void)first; (void)rate; }
void fle_tp_state(uint32_t state, uint32_t rate) { (void)state; (void)rate; }
void fle_tp_rewrite(const float *lr, uint32_t n, double first, uint32_t guard)
{
    fle_audio_ring_t *r = fle_audio_ring_get();
    float *d = fle_audio_ring_data();
    uint64_t f = (uint64_t)first, from = f > sim_consumed + guard ? f : sim_consumed + guard, end = f + n < r->write_frames ? f + n : r->write_frames, i;

    if (f < sim_consumed + guard) sim_offered_inside_guard += sim_consumed + guard - f;
    for (i = from; i < end; i++) {
        size_t at = (size_t)(i % r->capacity) * FLE_AUDIO_RING_CHANNELS;

        d[at] = lr[2u * (i - f)];
        d[at + 1] = lr[2u * (i - f) + 1u];
    }
}
double fle_tp_now(void) { return 0.0; }
int fle_tp_wait(double ms) { (void)ms; return 2; }
int fle_tp_can_wait(void) { return 0; }
void fle_audio_unlock_note(double first_frame, uint32_t frames) { (void)first_frame; (void)frames; }
void fle_audio_switch_note(unsigned from, unsigned to) { (void)from; (void)to; }
