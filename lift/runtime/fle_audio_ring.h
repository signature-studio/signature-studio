/* The audio output of the virtual machine (decision 0139, PROVISIONAL): ONE ring of stereo float32 frames in the host's linear memory, and ONE counter of consumed frames.
 *
 * Producer: runtime/fle_dsound.c. The "device" of the DirectSound model reads the bytes of the playing buffers at the play cursor (that is what a sound card does), mixes them, converts them to float32 and appends the frames
 * here, `lead` frames ahead of the consumed ones (the write cursor of DirectSound is exactly that lead: Learn, IDirectSoundBuffer::GetCurrentPosition, "typically about 15 milliseconds' worth of audio data").
 * Consumer: the output. In a native run or a test the consumer is the virtual clock (a free-running device clock: consumed(t) = (t - t0) * rate / 10^7 from the first Play); in the browser it is the AudioWorklet, which reads the
 * frames and counts what it has read; the single source of the play position is `fle_audio_consumed()` either way. The two differ in one function (below), nothing else.
 *
 * The ring header is the interface to JavaScript: the address comes from fle_audio_ring() (exported on wasm), the layout is fixed (version 1) and listed field by field in docs/decisions/0139-audio-s0-s2.md. On wasm the Worker
 * that runs the engine owns the linear memory (a WebAssembly.Memory that is not shared), so the AudioWorklet cannot read this ring itself: the transport is the pair of imports at the end of this file, called SYNCHRONOUSLY from
 * the engine (they work while the engine computes for seconds without returning to JS).
 *
 * Plain C, no host clock, no thread, no file: standard C only, the same for native and wasm. */
#ifndef FLE_AUDIO_RING_H
#define FLE_AUDIO_RING_H

#include <stdint.h>

#define FLE_AUDIO_RING_MAGIC      0x52414C46u   /* "FLAR" little endian */
#define FLE_AUDIO_RING_VERSION    1u
#define FLE_AUDIO_RING_FRAMES     32768u        /* capacity in frames (a power of two): 0.74 s at 44.1 kHz. A limit of this environment. */
#define FLE_AUDIO_RING_CHANNELS   2u            /* interleaved L R; a mono buffer is copied to both */
#define FLE_AUDIO_BLOCK_LOG       256u          /* the last 256 appends */
#define FLE_AUDIO_LEAD_MS         60u           /* S-audio-lead (PM decision on decision 0139): the DEFAULT lead of the write cursor over the play cursor, in ms. Learn gives "typically about 15 milliseconds' worth of audio data" (GetCurrentPosition; the value 15 is still
                                                 * selectable); 60 ms rides out a 60 ms stall of the engine (0.2% of the output quanta missing in the S2 test where 15 ms loses 8.4%). Adjustable: FLE_AUDIO_LEAD_MS (native environment), leadMs of fleAudioSab (page), 1..1000 */

/* state */
#define FLE_AUDIO_CLOSED  0u   /* no device object yet */
#define FLE_AUDIO_OPEN    1u   /* a device exists and has a format; nothing plays */
#define FLE_AUDIO_PLAYING 2u   /* at least one buffer plays */

/* One append (one pull of the device). 32 bytes. */
typedef struct {
    uint64_t first_frame;    /* absolute index of the first frame of the append (the ring's write_frames before it) */
    uint32_t frames;         /* how many frames */
    uint32_t reserved;
    uint64_t virt_ticks;     /* the virtual clock (100 ns) when it was appended */
    uint64_t consumed;       /* fle_audio_consumed() when it was appended */
} fle_audio_block;

typedef struct {
    uint32_t magic;          /* FLE_AUDIO_RING_MAGIC */
    uint32_t version;        /* FLE_AUDIO_RING_VERSION */
    uint32_t sample_rate;    /* the device's rate (the primary buffer's nSamplesPerSec); 0 while closed */
    uint32_t channels;       /* FLE_AUDIO_RING_CHANNELS */
    uint32_t capacity;       /* FLE_AUDIO_RING_FRAMES */
    uint32_t data_offset;    /* bytes from the start of this header to the first float */
    uint32_t state;          /* FLE_AUDIO_CLOSED / OPEN / PLAYING */
    uint32_t block_log_len;  /* FLE_AUDIO_BLOCK_LOG */
    volatile uint64_t write_frames;   /* total frames appended by the producer */
    volatile uint64_t read_frames;    /* total frames read by the consumer (the virtual consumer sets it from the clock) */
    volatile uint32_t underruns;      /* maintained by an EXTERNAL consumer only (0 for the virtual one: the virtual clock moves while no guest code runs, so nothing is ever missing); the AudioWorklet keeps its own counters in the shared ring */
    volatile uint32_t overruns;       /* producer: frames that could not be appended because the ring was full */
    uint64_t blocks_total;            /* appends so far */
    uint32_t lead_frames;             /* the current lead (write cursor - play cursor) in frames */
    uint32_t external_consumer;       /* 1 when the consumer is outside (wasm with the page's ring), else 0 */
    uint64_t reserved[4];
    fle_audio_block blocks[FLE_AUDIO_BLOCK_LOG];   /* block k of the log is blocks[k % FLE_AUDIO_BLOCK_LOG] */
    /* float data[FLE_AUDIO_RING_FRAMES * FLE_AUDIO_RING_CHANNELS] follows at data_offset */
} fle_audio_ring_t;

/* The ring (made at the first call, never freed). */
fle_audio_ring_t *fle_audio_ring_get(void);
/* The float frames of the ring (frame k is at [(k % capacity) * 2], left then right). */
float *fle_audio_ring_data(void);

/* The address of the ring header as a number JavaScript can use on wasm (an offset into the linear memory); on native the low 32 bits of the pointer are meaningless, use fle_audio_ring_get. */
uint32_t fle_audio_ring(void);

/* The device opened at `rate` (called by fle_dsound.c when the first device object is made and when the primary format is set). Resets nothing else. */
void fle_audio_ring_open(uint32_t rate);
void fle_audio_ring_set_state(uint32_t state);

/* ---- the single source of the play position ---- */
/* Frames consumed by the output since the device clock started (monotonic). Native: the virtual clock; in the browser: the AudioWorklet's count. 0 before fle_audio_clock_start. */
uint64_t fle_audio_consumed(void);
/* Starts the device clock at the current virtual time (the first Play after the device opened); a second call does nothing. */
void fle_audio_clock_start(void);
/* The virtual tick (100 ns) at which fle_audio_consumed() reaches `frames` (native source only; 0 when the clock is not started or the source is external). */
uint64_t fle_audio_tick_of_frame(uint64_t frames);
/* The lead in frames for the current rate (FLE_AUDIO_LEAD_MS). */
uint32_t fle_audio_lead_frames(void);

/* Appends `frames` interleaved stereo float frames (the producer). Returns how many were taken (less than `frames` only when the ring is full: overruns counts the rest). The append is logged as one block. */
uint32_t fle_audio_ring_append(const float *lr, uint32_t frames);
/* Sets read_frames to the consumed count (the virtual clock natively; the AudioWorklet in the browser). Called by the device poll. */
void fle_audio_ring_sync_consumer(void);
/* Frames the consumer could still read (write_frames - read_frames). */
uint32_t fle_audio_ring_fill(void);

/* ---- the browser transport ----
 * In a wasm build the transport is JavaScript embedded in fle_audio_ring.c (EM_JS): it works on the SharedArrayBuffer ring that the page made (harness/web/audio/ring.js), which the Worker hands over as globalThis.fleAudioSab
 * ({ sab, cap, dataOffset }) before the engine starts. Without it (any other wasm build, any native build) there is no external consumer and the virtual clock is the source. */
/* 1 when the consumer is external (wasm with fleAudioSab); the play position is then the AudioWorklet's read count. */
int fle_audio_external(void);
/* A region of a secondary buffer was committed by Unlock: `first_frame` is the output frame at which the device will read its first frame (the consumed count plus the region's distance ahead of the play cursor), `frames` its
 * length. Lets the page measure the delay from the engine's write to the AudioWorklet's read. A no-op without an external consumer. */
void fle_audio_unlock_note(double first_frame, uint32_t frames);
/* S-ds-late-write (decision 0241, PROVISIONAL): a region of a playing buffer was written (Unlock) after the device had already handed the frames of that region to the output ring. Hands the frames [first, first + frames) that
 * the output has NOT read yet to the output again, over the ones handed before, without moving the write position of the ring; the frames within fle_audio_rewrite_guard_frames() of the read position (and the ones already read) are
 * left alone. External consumer only: a no-op without one (the virtual clock never moves while guest code runs, so nothing is ever published stale there). */
void fle_audio_ring_rewrite(const float *lr, uint32_t frames, uint64_t first);
/* The distance in frames from the consumed position within which a rewrite is not made (S-ds-rewrite-guard: two 128-frame quanta of the AudioWorklet, the one it may be reading and the next). */
uint32_t fle_audio_rewrite_guard_frames(void);
/* Counts one Unlock that wrote into frames already handed to the output: `applied` frames that were handed over again, `lost` frames within the guard of the read position (exported to the page as fle_audio_rewrite_stat). */
void fle_audio_rewrite_note(uint32_t applied, uint32_t lost);
double fle_audio_rewrite_stat(unsigned k);   /* 0 events, 1 frames applied, 2 frames lost */
/* native only: the transport hooks that a host program may define (weak defaults in fle_audio_ring.c: no external consumer) */
int fle_audio_host_external(void);
uint64_t fle_audio_host_consumed(void);
void fle_audio_host_rewrite(const float *lr, uint32_t n, uint64_t first, uint32_t guard);

/* ---- real-time pacing in the browser (wasm with JSPI threads and an external consumer; else fle_audio_paced() is 0) ----
 * With an external consumer the play position is real time, but the engine's virtual clock moves only when every thread waits, and then it jumps to the next deadline at the speed of the CPU: a polling mixer (Sleep(1) +
 * GetCurrentPosition) would spin. While a buffer plays and the consumer is external, the scheduler (fle_thread.c) therefore waits REAL time for the next deadline before it moves the clock there (fle_audio_wait_until), the
 * wait being an async import (the JSPI stack of the waiting thread is suspended and the Worker's event loop runs: messages, timers). The pacing epoch is set when a buffer starts to play: virtual time v corresponds to real
 * time real0 + (v - virt0) / 10^4 ms; a deadline that is already past in real time (the engine is behind) is not waited for and its lateness is recorded. */
/* 1 when the consumer is external and a buffer plays (the scheduler paces). */
int fle_audio_paced(void);
/* Waits until the real time of the virtual deadline `dl` (ticks) has come: returns 2 when it has (the caller then moves the clock), 1 when the page posted an event to a queue meanwhile (the caller evaluates again),
 * 0 when the page ended the session. */
int fle_audio_wait_until(uint64_t dl);
/* Called by the scheduler at every thread switch (wasm only measures): the thread `from` ran from the last switch to now. */
void fle_audio_switch_note(unsigned from, unsigned to);
/* Counters of the pacing, for the report: 0 waits, 1 total wait ms, 2 late deadlines, 3 total lateness ms, 4 max lateness ms, 5 max stretch ms (the longest time between two scheduler waits: the engine ran without needing to wait),
 * 6 stretches over 20 ms, 7 over 50 ms, 8 over 78 ms (block 92.9 ms - the 15 ms lead of the first S2 runs; fixed at 78 ms for all leads). */
double fle_audio_pace_stat(unsigned k);

#endif
