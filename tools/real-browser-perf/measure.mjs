#!/usr/bin/env node
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { waveformSummary } from './waveform.mjs';

const execFileAsync = promisify(execFile);
const fail = (error) => {
  // execFile's message repeats the entire injected script. Keep failed runs compact.
  const message = error?.cmd ? String(error.stderr?.trim() || 'browse eval failed').slice(0, 4096) : String(error?.message ?? error);
  process.stdout.write(`${JSON.stringify({ ok: false, error: message, ...(error?.code !== undefined ? { code: error.code } : {}), ...(error?.signal ? { signal: error.signal } : {}) })}\n`);
  process.exitCode = 1;
};

function args(argv) {
  const [tab, condition, durationText, hzText, ...extra] = argv;
  const durationMs = Number(durationText), inputHz = Number(hzText);
  if (extra.length || !tab || !['none', 'center', 'toolbar'].includes(condition) || !Number.isSafeInteger(durationMs) || durationMs < 1 || durationMs > 3_600_000 || !Number.isFinite(inputHz) || inputHz <= 0 || inputHz > 10_000 || (condition === 'none' && hzText === undefined)) {
    throw new Error('usage: measure.mjs tab condition(none|center|toolbar) durationMs inputHz; duration 1..3600000, inputHz >0..10000');
  }
  return { tab, condition, durationMs, inputHz };
}

async function browserMeasure(c, summarizeWaveform) {
    const a = window.__app;
    const read = () => {
      const s = a.perf.snapshot();
      const d = a.audio?.detail?.() ?? null;
      const r = a.audio?.reports?.at(-1);
      const worker = r ? { waits: r.waits, maxWaitDrift: r.maxWaitDrift, pace: r.pace ? { ...r.pace } : null } : null;
      return { bundleVersion: s.bundle?.version ?? a.manifest?.bundle?.version ?? null, guestScreen: s.guestScreen ?? a.screen?.guest ?? null, hz: s.hz ?? null, at: performance.now(), state: a.state, audio: d, worker, pulses: a.audio?.pulses?.slice() ?? [], desk: a.deskStats ? { ...a.deskStats } : null, driver: a.driverStats ? { ...a.driverStats } : null, perf: { input: s.input, engineRun: s.engineRun } };
    };
    const detail = a?.audio?.detail?.();
    const state = { app: Boolean(a), engine: a?.state, fileDialog: a?.fileDlg?.open, audio: detail?.deviceState, context: detail?.ctxState, visibility: document.visibilityState };
    const wrong = Object.entries({ app: state.app, engine: state.engine === 'engine-idle', fileDialog: state.fileDialog === false, audio: state.audio === 'playing' && state.context === 'running', visibility: state.visibility === 'visible' }).filter(([, ok]) => !ok).map(([name]) => name);
    if (wrong.length) return { ok: false, failure: 'precondition', state, mismatches: wrong };
    const before = read();
    const captureIndex = a.audio.captures.length;
    if (c.durationMs >= 10_000) a.audio.armCapture(Math.floor(Atomics.load(a.audio.header, 3) * 8), true);
    let sent = 0, lastSent = null, maxIntervalMs = null;
    let worker = null, url = null, timer = null, listener = null;
    const canvasWindows = [];
    let canvasWindowCount = 0, minOneSecondCanvasFps = null;
    let canvasSampleAt = before.at, canvasFrames = before.desk?.displayFrames;
    const sampleCanvas = () => {
      const at = performance.now(), frames = a.deskStats?.displayFrames;
      const elapsedMs = at - canvasSampleAt;
      const frameDelta = Number.isFinite(canvasFrames) && Number.isFinite(frames) && frames >= canvasFrames ? frames - canvasFrames : null;
      const fps = frameDelta === null || elapsedMs <= 0 ? null : frameDelta * 1000 / elapsedMs;
      const window = { elapsedMs, frameDelta, fps };
      canvasWindows.push(window);
      if (canvasWindows.length > 60) canvasWindows.shift();
      canvasWindowCount++;
      if (elapsedMs >= 1000 && fps !== null) minOneSecondCanvasFps = Math.min(minOneSecondCanvasFps ?? fps, fps);
      canvasSampleAt = at;
      canvasFrames = frames;
    };
    let canvasTimer = null;
    const sampleNextWindow = () => {
      const remaining = 1000 - (performance.now() - canvasSampleAt);
      if (remaining <= 0) sampleCanvas();
      canvasTimer = setTimeout(sampleNextWindow, Math.max(1, remaining > 0 ? remaining : 1000));
    };
    canvasTimer = setTimeout(sampleNextWindow, 1000);
    try {
      if (c.condition === 'none') {
        await new Promise((resolve) => { timer = setTimeout(resolve, c.durationMs); });
      } else {
        const source = 'onmessage = ({data}) => { const end = performance.now() + data.duration; const step = 1000 / data.hz; let due = performance.now() + step; while (performance.now() < due) {} while (due < end) { postMessage(due); due += step; while (performance.now() < due) {} } postMessage({done:true}); };';
        url = URL.createObjectURL(new Blob([source], { type: 'text/javascript' }));
        worker = new Worker(url);
        listener = (event) => {
          if (event.data?.done) return;
          const received = performance.now();
          const { width: w, height: h } = a.screen.guest;
          const phase = (received - before.at) * 2 * Math.PI / 1000;
          const x = c.condition === 'center' ? w / 2 + Math.min(300, w / 10) * Math.sin(phase) : w / 4 + w / 5 * Math.sin(phase);
          const y = c.condition === 'center' ? h / 3 + Math.min(200, h / 8) * Math.cos(phase) : 30;
          a.sendRaw({ k: 'mouse', kind: 1, x: Math.round(x), y: Math.round(y), data: 0 });
          if (lastSent !== null) maxIntervalMs = Math.max(maxIntervalMs ?? 0, received - lastSent);
          lastSent = received;
          sent++;
        };
        worker.addEventListener('message', listener);
        const done = new Promise((resolve, reject) => {
          worker.addEventListener('message', (e) => { if (e.data?.done) resolve(); });
          worker.addEventListener('error', reject, { once: true });
          timer = setTimeout(() => reject(new Error('worker timed out')), c.durationMs + 10_000);
        });
        worker.postMessage({ duration: c.durationMs, hz: c.inputHz });
        await done;
      }
      sampleCanvas();
      const after = read();
      const delta = (x, y) => typeof x === 'number' && typeof y === 'number' ? y - x : null;
      const metricNames = ['underrunQuanta', 'engineGapFrames', 'engineSkippedFrames', 'engineLateWriteLost', 'nonSilentQuanta'];
      const audio = Object.fromEntries(metricNames.map((k) => [k, delta(before.audio?.[k], after.audio?.[k])]));
      const audioWait = {
        waits: delta(before.worker?.waits, after.worker?.waits),
        waitMs: delta(before.worker?.pace?.waitMs, after.worker?.pace?.waitMs),
        late: delta(before.worker?.pace?.late, after.worker?.pace?.late),
        lateMs: delta(before.worker?.pace?.lateMs, after.worker?.pace?.lateMs),
        lifetimeMaxWaitDriftMs: after.worker?.maxWaitDrift ?? null,
      };
      const deskKeys = ['displayFrames', 'presents', 'composeMs', 'putMs', 'drawMs', 'uploadBytes', 'uploads', 'draws'];
      const desk = Object.fromEntries(deskKeys.map((k) => [k, delta(before.desk?.[k], after.desk?.[k])]));
      const meanCanvasFps = Number.isFinite(desk.displayFrames) && desk.displayFrames >= 0 && after.at > before.at ? desk.displayFrames * 1000 / (after.at - before.at) : null;
      const driver = Object.fromEntries([...new Set([...Object.keys(before.driver ?? {}), ...Object.keys(after.driver ?? {})])].map((k) => [k, delta(before.driver?.[k], after.driver?.[k])]));
      const recentPulses = after.pulses.filter((p) => p.page >= before.at && p.page <= after.at);
      const firstPulse = before.pulses.at(-1), lastPulse = after.pulses.at(-1);
      const pulseWallMs = delta(firstPulse?.real, lastPulse?.real);
      const pulseVirtualTicks = delta(firstPulse?.virt, lastPulse?.virt);
      const clock = {
        pulseWallMs,
        virtualMs: pulseVirtualTicks === null ? null : pulseVirtualTicks / 10000,
        virtualPerWall: pulseWallMs > 0 && pulseVirtualTicks !== null ? pulseVirtualTicks / 10000 / pulseWallMs : null,
      };
      const unlockCompute = Object.fromEntries(['n', 'frames', 'sumMs', 'over78'].map(k => [k, delta(firstPulse?.unlockCompute?.[k], lastPulse?.unlockCompute?.[k])]));
      unlockCompute.lifetimeMaxMs = lastPulse?.unlockCompute?.maxMs ?? null;
      const unlocks = (a.audio?.reports ?? []).flatMap(r => r.unlocks ?? []).filter(u => u.t > firstPulse?.epoch && u.t <= lastPulse?.epoch);
      const outputConsumedFrames = delta(firstPulse?.consumed, lastPulse?.consumed);
      const recordedWriteFrames = unlocks.reduce((sum, u) => sum + u.n, 0);
      const fullyLogged = unlockCompute.n !== null && unlocks.length === unlockCompute.n;
      const audioProducer = {
        registeredUnlocks: unlockCompute.n, recordedUnlocks: unlocks.length,
        registeredWriteFrames: unlockCompute.frames,
        recordedWriteFrames, outputConsumedFrames, fullyLogged,
        writeToReadFrameRatio: outputConsumedFrames > 0 ? unlockCompute.frames !== null ? unlockCompute.frames / outputConsumedFrames : fullyLogged ? recordedWriteFrames / outputConsumedFrames : null : null,
        bufferFrames: [...new Set(unlocks.map(u => u.bufferFrames).filter(Number.isFinite))],
        offsetFrames: [...new Set(unlocks.map(u => u.offsetFrames).filter(Number.isFinite))],
        note: '書き込み量と消費量の比。同一サンプルレートで比較する。ループ音源は再利用できるため、単独で音声の正しさ・欠落を判定しない。累積フレーム数がない旧ビルドではログ欠落時の比率は未測定。',
      };
      // These counters attribute wall time between guest switches, including
      // waits; they are not a CPU sample profile. Keep output bounded even for
      // a long soak rather than returning every cumulative telemetry sample.
      const threadPulses = Array.from({ length: lastPulse?.threads?.length ?? 0 }, (_, thread) => ({
        thread,
        attributedWallMs: delta(firstPulse?.threads?.[thread]?.[0], lastPulse?.threads?.[thread]?.[0]),
        switches: delta(firstPulse?.threads?.[thread]?.[2], lastPulse?.threads?.[thread]?.[2]),
        lifetimeMaxMs: lastPulse.threads[thread][1],
      }));
      const elapsedMs = after.at - before.at;
      const captured = c.durationMs >= 10_000 ? a.audio.captures[captureIndex] : null;
      const waveform = summarizeWaveform(captured);
      if (captured) a.audio.captures.splice(captureIndex, 1);
      return { ok: true, condition: c.condition, requested: { durationMs: c.durationMs, inputHz: c.condition === 'none' ? null : c.inputHz }, interval: { start: before.at, end: after.at, elapsedMs }, audio, audioWait, audioProducer, clock, waveform, desk, meanCanvasFps, minOneSecondCanvasFps, canvasWindows: { recent: canvasWindows, total: canvasWindowCount }, driver, realAudioObserved: typeof audio.nonSilentQuanta === 'number' && audio.nonSilentQuanta > 0, measurementPrerequisites: { bundleVersion: before.bundleVersion, guestScreen: before.guestScreen, hz: before.hz }, displayFramesMeaning: 'render Worker rAF周期で重複排除したcanvas更新。GPU完了や物理表示の証明ではない。未測定はnull。', audioPulse: { n: recentPulses.length, mixer: lastPulse?.mixer ?? null, threads: threadPulses, unlockCompute }, input: { sent, achievedHz: sent * 1000 / elapsedMs, maxIntervalMs, targetHz: c.condition === 'none' ? null : c.inputHz, target1250AchievedAtLeast1000Hz: c.condition === 'none' ? null : c.inputHz === 1250 ? sent * 1000 / elapsedMs >= 1000 : null, intervalStart: before.at, intervalEnd: after.at }, perf: after.perf, physicalMouseMeasured: false, note: '入力は page main の __app.sendRaw による合成 raw injection。physical mouse は測定していない。', state };
    } finally {
      if (timer !== null) clearTimeout(timer);
      clearTimeout(canvasTimer);
      if (worker && listener) worker.removeEventListener('message', listener);
      worker?.terminate();
      if (url) URL.revokeObjectURL(url);
    }
}

function expression({ condition, durationMs, inputHz }) {
  return `(${browserMeasure.toString()})(${JSON.stringify({ condition, durationMs, inputHz })},(${waveformSummary.toString()}))`;
}

try {
  const input = args(process.argv.slice(2));
  const timeout = input.durationMs + 30_000;
  const { stdout } = await execFileAsync('browse', ['eval', '--json', JSON.stringify({ tab: input.tab, world: 'main', expression: expression(input), timeout })], { timeout: timeout + 5_000, maxBuffer: 8 * 1024 * 1024 });
  const result = JSON.parse(stdout);
  process.stdout.write(`${JSON.stringify(result)}\n`);
  if (result.ok === false) process.exitCode = 1;
} catch (error) {
  fail(error);
}
