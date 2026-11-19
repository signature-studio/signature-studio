/* The transport of the audio output (decision 0241 split it from fle_audio_ring.c; see fle_audio_transport.c). fle_audio_ring.c calls these; fle_audio_ring.h declares fle_audio_external, fle_audio_unlock_note and fle_audio_switch_note, which the transport file defines. */
#ifndef FLE_AUDIO_TRANSPORT_H
#define FLE_AUDIO_TRANSPORT_H

#include <stdint.h>

/* frames the output has read (RD, unwrapped to 64 bits, as a double); 0 without an external consumer */
double fle_tp_consumed(void);
/* n stereo float frames to the ring at index `first`: silence for a gap since the last hand-over, nothing for the frames the output has already passed */
void fle_tp_publish(const float *lr, uint32_t n, double first, uint32_t rate);
/* the same frames again over the ones handed over before, only the ones at least `guard` frames ahead of the output and before the last handed-over frame; changes neither WR nor the counters (fle_audio_ring_rewrite) */
void fle_tp_rewrite(const float *lr, uint32_t n, double first, uint32_t guard);
/* the state (FLE_AUDIO_*) and the rate of the device, for the output */
void fle_tp_state(uint32_t state, uint32_t rate);
/* real time in ms (performance.now) */
double fle_tp_now(void);
/* waits ms of real time (2 after the time-out); fle_tp_can_wait is 0 when the build cannot wait */
int fle_tp_wait(double ms);
int fle_tp_can_wait(void);

#endif
