'use strict';
// The page of the audio prototype (decision 0139, PROVISIONAL). Makes the AudioContext and the AudioWorkletNode (the worklet needs a Window), the ring (ring.js), and the engine Worker (audio-worker.js); collects the counters of the ring,
// the worklet's samples, the context's timing values and the Worker's report; window.__audio is the result when window.__audioDone is true.
// Query: rate (default 44100), cap (ring frames, default 32768), ms (how long to measure after the first PLAYING, default 20000), engine=0 (no engine: a synthetic producer, mode=sine|stall), env=<JSON>, wait (ms to wait for PLAYING, default 240000).
const q = new URLSearchParams(location.search);
const RATE = +(q.get('rate') || 44100), CAP = +(q.get('cap') || 32768), MEASURE_MS = +(q.get('ms') || 20000), WAIT_MS = +(q.get('wait') || 240000);
const ENGINE = q.get('engine') !== '0';
const LEAD_MS = +(q.get('lead') || 0);   // 0: the runtime's default (60 ms, S-audio-lead)
const SCEN = q.get('scen');   // a scenario of harness/audio/ds_scenarios.c built by build-scen-wasm.sh instead of the engine: scen=rt_mixer&args=40,0,0 (comma separated)
const logEl = document.getElementById('log');
const log = (s) => { logEl.textContent += s + '\n'; };
const t0 = performance.now();
const live = { seq: 0, state: 0, rd: 0, wr: 0, marks: [], jserrors: [], engineMessages: 0, stdout: 0 };
window.__live = live;
window.__audioDone = false;
const samples = [];       // worklet samples with the page's clocks at their arrival
const stamps = [];        // getOutputTimestamp pairs
let workerState = [];
const pulses = [];       // telemetry posted by the engine itself at thread switches (every 500 ms of real time)     // answers of the Worker
let ctx, node, sab, h, w;

const sha = async (u8) => { const d = await crypto.subtle.digest('SHA-256', u8); return [...new Uint8Array(d)].map((b) => b.toString(16).padStart(2, '0')).join(''); };

async function main() {
  ctx = new AudioContext({ sampleRate: RATE, latencyHint: 'interactive' });
  const ctxStart = { state: ctx.state, sampleRate: ctx.sampleRate, baseLatency: ctx.baseLatency, outputLatency: ctx.outputLatency, currentTime: ctx.currentTime, sinkId: String(ctx.sinkId || '') };
  if (ctx.state !== 'running') await ctx.resume();
  await ctx.audioWorklet.addModule('/audio-worklet.js');
  sab = FLE_AUDIO_SAB.make(CAP, RATE);
  h = new Int32Array(sab, 0, 32);
  node = new AudioWorkletNode(ctx, 'fle-out', { numberOfInputs: 0, numberOfOutputs: 1, outputChannelCount: [2], processorOptions: { sab, cap: CAP, dataOffset: FLE_AUDIO_SAB.DATA_OFFSET } });
  node.connect(ctx.destination);
  node.port.onmessage = (e) => {
    const m = e.data;
    m.pagePerf = performance.now(); m.pageCtxTime = ctx.currentTime;
    if (samples.length < 100000) samples.push(m);
    live.state = m.state; live.rd = m.rd; live.wr = m.wr;
  };
  setInterval(() => { try { const ts = ctx.getOutputTimestamp(); stamps.push({ perf: performance.now(), contextTime: ts.contextTime, performanceTime: ts.performanceTime, ctxCurrentTime: ctx.currentTime, outputLatency: ctx.outputLatency }); } catch (e) {} }, 250);
  setInterval(() => { live.seq++; }, 1000);   // heartbeat of the page's own event loop
  let maxGap = 0, last = performance.now();
  setInterval(() => { const n = performance.now(); maxGap = Math.max(maxGap, n - last); last = n; }, 20);
  const startedPerf = performance.now();
  if (ENGINE) {
    w = new Worker(SCEN ? '/scen-worker.js' : '/audio-worker.js');
    w.onerror = (ev) => { live.jserrors.push('worker.onerror: ' + ev.message); };
    w.onmessage = (ev) => {
      const m = ev.data;
      live.engineMessages++;
      if (m.type === 'ready' && SCEN) {
        w.postMessage({ cmd: 'run-scen', dist: '/dist/', args: [SCEN].concat(q.get('args') ? q.get('args').split(',') : []) });
      } else if (m.type === 'ready') {
        w.postMessage({ cmd: 'run', glue: new URL('/dist/flengine6.js', location.href).href, wasm: new URL('/dist/flengine6.wasm', location.href).href, dataBase: new URL('/dist/data/', location.href).href,
          streaming: false, cacheapi: false, cache: false, par: false, seedmode: 'pack', pack: 'seedpack.bin', root: '/data', env: q.get('env') ? JSON.parse(q.get('env')) : {} });
      } else if (m.type === 'mark') { live.marks.push([m.name, m.t]); }
      else if (m.type === 'stdout' || m.type === 'stderr') { live.stdout++; if (/^(fruity|fle)/.test(m.line)) (live.special = live.special || []).push(m.line.slice(0, 400)); (live.tail = live.tail || []).push((m.type === 'stderr' ? '[err] ' : '') + m.line.slice(0, 220)); if (live.tail.length > 40) live.tail.shift(); }
      else if (m.type === 'jserror' || m.type === 'error') { live.jserrors.push(m.type + ': ' + (m.message || '').slice(0, 300)); }
      else if (m.type === 'done') { live.engineDone = { exitCode: m.exitCode, error: m.error }; }
      else if (m.type === 'audio-state') { workerState.push(m); }
      else if (m.type === 'audio-pulse') { m.pagePerf = performance.now(); if (pulses.length < 4000) pulses.push(m); live.lastPulse = { real: m.real, virt: m.virt, consumed: m.consumed, written: m.written }; }
      else if (m.type === 'idle') { live.idles = (live.idles || 0) + 1; }
    };
    w.postMessage({ cmd: 'audio-init', sab, cap: CAP, dataOffset: FLE_AUDIO_SAB.DATA_OFFSET, leadMs: LEAD_MS });
    const oc = new OffscreenCanvas(1930, 1092);
    w.postMessage({ cmd: 'init', canvasA: oc }, [oc]);
  } else {
    // a synthetic producer in a Worker would be needed; engine=0 is the transport-only case, driven by window.P.synth (see the runner)
  }
  // wait for the first PLAYING (the engine opened the device and started the secondary buffer)
  const tw = performance.now();
  while (Atomics.load(h, 2) !== 2 && performance.now() - tw < WAIT_MS) await new Promise((r) => setTimeout(r, 100));
  const playing = Atomics.load(h, 2) === 2;
  const tPlay = performance.now();
  const snap0 = counters();
  // measure
  if (playing) await new Promise((r) => setTimeout(r, MEASURE_MS));
  const tEnd = performance.now();
  const snap1 = counters();
  // ask the Worker for its report (answered when its event loop runs); wait up to 15 s
  if (ENGINE && w) { w.postMessage({ cmd: 'audio-state', seq: 1 }); const tq = performance.now(); while (!workerState.length && performance.now() - tq < 15000) await new Promise((r) => setTimeout(r, 100)); }
  const ctxEnd = { state: ctx.state, currentTime: ctx.currentTime, baseLatency: ctx.baseLatency, outputLatency: ctx.outputLatency };
  window.__audio = { pageTimeOrigin: performance.timeOrigin, rate: RATE, cap: CAP, ctxStart, ctxEnd, startedPerf, tPlayPerf: tPlay, tPlayAfterStartMs: tPlay - startedPerf, playing, measureMs: tEnd - tPlay, snap0, snap1, maxPageGapMs: maxGap,
    pulses: pulses, samplesN: samples.length, samples: downsample(samples, 4000), stamps: stamps.slice(-400), worker: workerState, live: JSON.parse(JSON.stringify(live)) };
  window.__audioDone = true;
}
function counters() {
  const g = (i) => Atomics.load(h, i);
  return { perf: performance.now(), rd: g(0) >>> 0, wr: g(1) >>> 0, state: g(2), qTotal: g(8), qPlaying: g(9), urQuanta: g(10), urFrames: g(11), fillMin: g(12), fillMax: g(13), publishes: g(14), published: g(15), skipped: g(16), ctxTime: ctx.currentTime };
}
function downsample(a, n) { if (a.length <= n) return a; const out = []; const step = a.length / n; for (let i = 0; i < n; i++) out.push(a[Math.floor(i * step)]); return out; }
window.P = { counters, samples: () => samples.length, stamps: () => stamps.length };
main().catch((e) => { live.jserrors.push('main: ' + String((e && e.stack) || e)); window.__audio = { error: String((e && e.stack) || e), live }; window.__audioDone = true; });
