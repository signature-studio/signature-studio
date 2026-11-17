'use strict';
// ?matrix=1 runs the stall/ring matrix; the default is one short smoke run. Real-time AudioContext (not Offline).
const Q = new URLSearchParams(location.search);
const log = (s) => { (window.__log = window.__log || []).push(s); };
const hasSAB = typeof SharedArrayBuffer !== 'undefined';
const pause = (ms) => new Promise((r) => setTimeout(r, ms));

async function oneRun(cfg) {
  // cfg: {sampleRate, cap (frames), chunk, ms, stallMs, stallEveryMs, loadMs, mainBusy: {ms, every}, latencyHint}
  const R = { cfg };
  const t0 = performance.now();
  const ctx = new AudioContext({ sampleRate: cfg.sampleRate, latencyHint: cfg.latencyHint || 'interactive' });
  R.created = { state: ctx.state, sampleRate: ctx.sampleRate, baseLatency: ctx.baseLatency, outputLatency: ctx.outputLatency, hasWorklet: !!ctx.audioWorklet };
  if (ctx.state !== 'running') { try { await Promise.race([ctx.resume(), pause(3000)]); } catch (e) { R.resumeError = String(e); } }
  R.afterResume = { state: ctx.state };
  await ctx.audioWorklet.addModule('audio-worklet.js');
  const sab = new SharedArrayBuffer(64 + cfg.cap * 2 * 4);
  const i = new Int32Array(sab);
  const node = new AudioWorkletNode(ctx, 'ring-reader', { numberOfInputs: 0, numberOfOutputs: 1, outputChannelCount: [2], processorOptions: { sab, cap: cfg.cap } });
  node.connect(ctx.destination);
  const wk = new Worker('audio-worker.js');
  const done = new Promise((res, rej) => { wk.onmessage = (e) => { if (e.data.type === 'result') res(e.data); }; wk.onerror = (e) => rej(new Error('worker: ' + e.message)); });
  const ct0 = ctx.currentTime;
  const w0 = performance.now();
  wk.postMessage({ sab, cap: cfg.cap, chunk: cfg.chunk, ms: cfg.ms, sampleRate: cfg.sampleRate, stallMs: cfg.stallMs || 0, stallEveryMs: cfg.stallEveryMs || 0, loadMs: cfg.loadMs || 0 });
  let stop = false;
  if (cfg.mainBusy) (async () => { while (!stop) { await pause(cfg.mainBusy.every); const e = performance.now() + cfg.mainBusy.ms; while (performance.now() < e); } })();
  const pr = await done;
  stop = true;
  const wallMs = performance.now() - w0;
  const ctDelta = ctx.currentTime - ct0;
  wk.terminate();
  R.producer = pr;
  R.timing = { wall_ms: +wallMs.toFixed(1), audioContext_currentTime_delta_s: +ctDelta.toFixed(3), realtime_factor: +(ctDelta * 1000 / wallMs).toFixed(4) };
  R.worklet = {
    blocks: Atomics.load(i, 3), expectedBlocks: Math.round(ctDelta * cfg.sampleRate / 128),
    underrunBlocksTotal: Atomics.load(i, 2), underrunBlocksAfterStart: Atomics.load(i, 9), startedFlag: Atomics.load(i, 8), underrunFrames: Atomics.load(i, 7), minFill: i[5], maxFill: i[6],
    blocksPerSecondWall: +(Atomics.load(i, 3) * 1000 / wallMs).toFixed(1), expectedBlocksPerSecond: +(cfg.sampleRate / 128).toFixed(1),
  };
  R.after = { state: ctx.state, outputLatency: ctx.outputLatency };
  node.disconnect();
  await ctx.close();
  R.setup_ms = +(w0 - t0).toFixed(1);
  return R;
}

async function main() {
  const R = { env: {
    crossOriginIsolated: self.crossOriginIsolated, AudioContext: typeof AudioContext, AudioWorklet: typeof AudioWorklet,
    OfflineAudioContext: typeof OfflineAudioContext, SharedArrayBuffer: typeof SharedArrayBuffer, userAgent: navigator.userAgent,
    mediaDevices: !!(navigator.mediaDevices && navigator.mediaDevices.enumerateDevices),
    outputDevices: null,
  } };
  try { const d = await navigator.mediaDevices.enumerateDevices(); R.env.outputDevices = d.filter((x) => x.kind === 'audiooutput').map((x) => ({ id: x.deviceId.slice(0, 8), label: x.label })); R.env.allDevices = d.length; } catch (e) { R.env.enumerateError = String(e); }
  log('env ' + JSON.stringify(R.env));
  R.runs = [];
  const cfgs = [];
  if (Q.get('matrix') === '1') {
    for (const sr of [48000, 44100]) cfgs.push({ sampleRate: sr, cap: 2048, chunk: 512, ms: 5000 });
    for (const cap of [256, 1024, 4096]) for (const stall of [0, 5, 20, 50]) cfgs.push({ sampleRate: 48000, cap, chunk: Math.min(512, cap / 2), ms: 3000, stallMs: stall, stallEveryMs: 500 });
    cfgs.push({ sampleRate: 48000, cap: 1024, chunk: 256, ms: 4000, mainBusy: { ms: 100, every: 250 } });
    cfgs.push({ sampleRate: 48000, cap: 1024, chunk: 256, ms: 4000, loadMs: 3 });   // render cost 3 ms per 256 frames (5.3 ms of audio): 56% load
    cfgs.push({ sampleRate: 48000, cap: 1024, chunk: 256, ms: 4000, loadMs: 6 });   // 113% load: cannot keep up
  } else cfgs.push({ sampleRate: 48000, cap: 2048, chunk: 512, ms: 2000 });
  for (const cfg of cfgs) {
    try { R.runs.push(await oneRun(cfg)); } catch (e) { R.runs.push({ cfg, error: String(e && e.stack || e) }); }
    log('run ' + JSON.stringify(cfg) + ' done');
  }
  window.__result = R;
  window.__done = true;
}
if (!hasSAB) { window.__result = { env: { SharedArrayBuffer: 'undefined' } }; window.__done = true; } else main().catch((e) => { window.__error = String(e && e.stack || e); });
