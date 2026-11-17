/* The transport of the audio output between the engine and the page (decisions 0139 and 0175, PROVISIONAL; split from fle_audio_ring.c by decision 0241): on wasm, JavaScript embedded here (EM_JS) that works on the SharedArrayBuffer ring of the page
 * (globalThis.fleAudioSab, harness/web/audio/ring.js) - frames handed over, the read position of the AudioWorklet, the frames put over the handed-over ones (fle_tp_rewrite), the state, the real-time wait of the pacing and the telemetry to the
 * page; natively nothing (no external consumer). fle_audio_ring.c holds the ring, the lead and the pacing rules and calls this file through fle_audio_transport.h. A test that needs another consumer links its own file with the same functions
 * in place of this one (lift/harness/audio/sim_transport.c). */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fle_audio_ring.h"
#include "fle_audio_transport.h"
#include "fle_sys.h"

#ifdef __EMSCRIPTEN__
#include <emscripten.h>
#endif

#ifdef __EMSCRIPTEN__
/* init: the ring of the page, from globalThis.fleAudioSab; 0 when there is none. State lives in globalThis.__fleAud. */
EM_JS(int, fle_js_init, (void), {
  if (globalThis.__fleAud !== undefined) return globalThis.__fleAud ? 1 : 0;
  const g = globalThis.fleAudioSab;
  if (!g) { globalThis.__fleAud = null; return 0; }
  const A = { h: new Int32Array(g.sab, 0, 32), d: new Float32Array(g.sab, g.dataOffset, g.cap * 2), cap: g.cap, mask: g.cap - 1, rdHi: 0, rdPrev: 0, wr: 0, unlocks: [], publishes: 0, skipped: 0, gapFrames: 0,
    resolve: null, timer: 0, waits: 0, maxWaitDrift: 0 };
  globalThis.__fleAud = A;
  globalThis.fleAudioWorkerState = () => ({ unlocks: A.unlocks, waits: A.waits, publishes: A.publishes, skipped: A.skipped, gapFrames: A.gapFrames, maxWaitDrift: A.maxWaitDrift });
  globalThis.fleAudioRewrite = () => {
    const f = Module['_fle_audio_rewrite_stat'] || (typeof wasmExports !== 'undefined' && wasmExports['fle_audio_rewrite_stat']);
    const names = ['events', 'applied', 'lost'];
    const o = {};
    if (f) for (let i = 0; i < names.length; i++) o[names[i]] = f(i);
    return o;
  };
  globalThis.fleAudioPace = () => {
    const f = Module['_fle_audio_pace_stat'] || (typeof wasmExports !== 'undefined' && wasmExports['fle_audio_pace_stat']);
    const names = ['waits', 'waitMs', 'late', 'lateMs', 'lateMaxMs', 'stretchMaxMs', 'stretchOver20', 'stretchOver50', 'stretchOver78'];
    const o = {};
    if (f) for (let i = 0; i < names.length; i++) o[names[i]] = f(i);
    return o;
  };
  return 1;
});
/* frames read by the AudioWorklet (RD, unwrapped to 64 bits) */
EM_JS(double, fle_js_consumed, (void), {
  const A = globalThis.__fleAud;
  if (!A) return 0;
  const lo = Atomics.load(A.h, 0) >>> 0;
  if (lo < A.rdPrev) A.rdHi += 4294967296;
  A.rdPrev = lo;
  return A.rdHi + lo;
});
/* n stereo float frames at p (linear memory) to the ring at index `first`: silence for a gap since the last publish, nothing for frames the output has already passed; then WR */
EM_JS(void, fle_js_publish, (const float *p, unsigned n, double first, unsigned rate), {
  const A = globalThis.__fleAud;
  if (!A) return;
  const src = new Float32Array(HEAPU8.buffer, p, n * 2);
  const rd = A.rdHi + (Atomics.load(A.h, 0) >>> 0);
  let at = first, k0 = 0;
  if (first + n <= rd) { A.skipped += n; Atomics.add(A.h, 16, n); return; }
  if (first < rd) { const late = rd - first; k0 = late; at = rd; A.skipped += late; Atomics.add(A.h, 16, late); }
  if (at > A.wr) {
    let g = at - A.wr;
    A.gapFrames += g;
    if (g > A.cap) g = A.cap;
    for (let i = 0; i < g; i++) { const q = ((at - g + i) & A.mask) * 2; A.d[q] = 0; A.d[q + 1] = 0; }
  }
  for (let i = k0; i < n; i++) { const q = ((first + i) & A.mask) * 2; A.d[q] = src[2 * i]; A.d[q + 1] = src[2 * i + 1]; }
  A.wr = first + n;
  Atomics.store(A.h, 1, A.wr | 0);
  A.publishes++;
  Atomics.add(A.h, 14, 1); Atomics.add(A.h, 15, n - k0);
});
/* n stereo float frames at p to the ring at index `first`, only the ones the output has not read yet and that are at least `guard` frames ahead of its read position and before the last published frame (WR): a late write of the guest
 * reaches the output if it comes in time. It changes neither WR nor the counters of the page. */
EM_JS(void, fle_js_rewrite, (const float *p, unsigned n, double first, unsigned guard), {
  const A = globalThis.__fleAud;
  if (!A) return;
  const rd = A.rdHi + (Atomics.load(A.h, 0) >>> 0);
  let from = first > rd + guard ? first : rd + guard;
  const end = first + n < A.wr ? first + n : A.wr;
  if (end <= from) return;
  const src = new Float32Array(HEAPU8.buffer, p, n * 2);
  for (let i = from; i < end; i++) { const q = (i & A.mask) * 2, k = (i - first) * 2; A.d[q] = src[k]; A.d[q + 1] = src[k + 1]; }
});
EM_JS(void, fle_js_state, (unsigned state, unsigned rate), {
  const A = globalThis.__fleAud;
  if (!A) return;
  Atomics.store(A.h, 3, rate); Atomics.store(A.h, 2, state);
});
EM_JS(double, fle_js_now, (void), { return performance.now(); });
EM_JS(void, fle_js_unlock, (double first, unsigned n), {
  const A = globalThis.__fleAud;
  if (!A) return;
  if (A.unlocks.length < 4000) A.unlocks.push({ first: first, n: n, t: performance.timeOrigin + performance.now() });
});
/* telemetry to the page, from the engine's own thread (the Worker may be inside a long compute and unable to answer a query): called from a thread switch at most every 500 ms of real time */
EM_JS(void, fle_js_pulse, (double real, double virt, double consumed, double written, unsigned nthr, const double *tot, const double *mx, const double *cnt, int mixer, double uc_n, double uc_sum, double uc_max, double uc_over78), {
  const A = globalThis.__fleAud;
  if (!A) return;
  const n = Math.min(nthr, 32), t = [];
  for (let i = 0; i < n; i++) t.push([HEAPF64[(tot >> 3) + i], HEAPF64[(mx >> 3) + i], HEAPF64[(cnt >> 3) + i]]);
  self.postMessage({ type: 'audio-pulse', real: real, epoch: performance.timeOrigin + real, virt: virt, consumed: consumed, written: written, threads: t, mixer: mixer, unlockCompute: { n: uc_n, sumMs: uc_sum, maxMs: uc_max, over78: uc_over78 } });
});
#ifdef __EMSCRIPTEN__
/* ASYNC (JSPI): waits ms of real time (the thread's JSPI stack is suspended and the Worker's event loop runs). 2 after the time-out. It does not take part in the page's idle protocol of decision 0136 (fle_idle_wait / fleIdleDeliver:
 * the idle host has the first say, fle_thread.c), so an input event that the page has for the engine waits for the end of this wait or for the idle. */
EM_ASYNC_JS(int, fle_js_wait, (double ms), {
  const A = globalThis.__fleAud;
  A.waits++;
  return await new Promise((resolve) => {
    const t0 = performance.now();
    setTimeout(() => { const drift = performance.now() - t0 - ms; if (drift > A.maxWaitDrift) A.maxWaitDrift = drift; resolve(2); }, ms);
  });
});
int fle_tp_can_wait(void) { return 1; }
#else
static int js_wait_unavailable(double ms) { (void)ms; return 2; }
#define fle_js_wait js_wait_unavailable
int fle_tp_can_wait(void) { return 0; }
#endif
static int ext_cached = -1;
int fle_audio_external(void)
{
    if (ext_cached < 0) ext_cached = fle_js_init();
    return ext_cached;
}
double fle_tp_consumed(void) { return fle_js_consumed(); }
void fle_tp_publish(const float *lr, uint32_t n, double first, uint32_t rate) { fle_js_publish(lr, n, first, rate); }
void fle_tp_state(uint32_t state, uint32_t rate) { fle_js_state(state, rate); }
void fle_tp_rewrite(const float *lr, uint32_t n, double first, uint32_t guard) { fle_js_rewrite(lr, n, first, guard); }
double fle_tp_now(void) { return fle_js_now(); }
int fle_tp_wait(double ms) { return fle_js_wait(ms); }
/* per-thread run time between switches (wall ms), the compute time of the mixer's committed blocks, and the telemetry */
#define SL_MAX 64u
static double sl_total[SL_MAX], sl_max[SL_MAX], sl_n[SL_MAX], sl_start = -1.0, last_pulse = -1e9, uc_n, uc_sum, uc_max, uc_over78;
static int mixer_thr = -1;
unsigned fle_thread_current(void) __attribute__((weak));
void fle_audio_switch_note(unsigned from, unsigned to)
{
    double now;

    (void)to;
    if (!fle_audio_external()) return;
    now = fle_tp_now();
    if (sl_start >= 0.0 && from < SL_MAX) {
        double d = now - sl_start;

        sl_total[from] += d;
        sl_n[from] += 1.0;
        if (d > sl_max[from]) sl_max[from] = d;
    }
    sl_start = now;
    if (now - last_pulse >= 500.0) {
        last_pulse = now;
        fle_js_pulse(now, (double)fle_sys_clock_ticks(), (double)fle_audio_consumed(), (double)fle_audio_ring_get()->write_frames, SL_MAX, sl_total, sl_max, sl_n, mixer_thr, uc_n, uc_sum, uc_max, uc_over78);
    }
}
void fle_audio_unlock_note(double first_frame, uint32_t frames)
{
    if (!fle_audio_external()) return;
    fle_js_unlock(first_frame, frames);
    if (fle_thread_current) mixer_thr = (int)fle_thread_current();
    if (sl_start >= 0.0) {   /* the mixer ran since its slice began: this is the time it took to produce the block it just committed (and whatever else the slice did before) */
        double d = fle_tp_now() - sl_start;

        uc_n += 1.0;
        uc_sum += d;
        if (d > uc_max) uc_max = d;
        if (d > 78.0) uc_over78 += 1.0;
    }
}
#else
/* Native: no transport. The play position is the virtual clock (fle_audio_ring.c), nothing is handed to a page. The scenarios of decision 0241 (lift/harness/audio/sim_transport.c) link their own transport instead of this file
 * to stand for the AudioWorklet. */
int fle_audio_external(void) { return 0; }
double fle_tp_consumed(void) { return 0.0; }
void fle_tp_publish(const float *lr, uint32_t n, double first, uint32_t rate) { (void)lr; (void)n; (void)first; (void)rate; }
void fle_tp_state(uint32_t state, uint32_t rate) { (void)state; (void)rate; }
void fle_tp_rewrite(const float *lr, uint32_t n, double first, uint32_t guard) { (void)lr; (void)n; (void)first; (void)guard; }
double fle_tp_now(void) { return 0.0; }
int fle_tp_wait(double ms) { (void)ms; return 2; }
int fle_tp_can_wait(void) { return 0; }
void fle_audio_unlock_note(double first_frame, uint32_t frames) { (void)first_frame; (void)frames; }
void fle_audio_switch_note(unsigned from, unsigned to) { (void)from; (void)to; }
#endif

