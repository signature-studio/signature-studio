// The shared steps of the audio scenarios (Issue #1491). A person's actions are real mouse and key events (lib/guest.mjs); the reads are the page's audio numbers
// (window.__app.audio: detail(), fill(), counters()) and its capture of the output (armCapture: the frames the worklet copies; tools/p3-audio-probe/waveform.mjs for the PCM repeat).
// The places (PLACE) are guest pixels where the controls are drawn at the 3840 x 2160 guest (4k-dpr1 and 4k-random: the guest is the same at any DPR). They are where to click,
// not what is expected: a check reads the number that changes (a counter, the output, a picture region's hash) before and after the action.
import { sleep } from './flow.mjs';
import { pixelHashRegion, openMenu, waitWindow, windowTable, picture } from './common.mjs';
import { armExpression, takeExpression, pcmReport } from '../../p3-audio-probe/waveform.mjs';

export const PLACE = {
  pattern: { x: 320, y: 19 },
  record: { x: 418, y: 19 },
  bpm: { x: 463, y: 19 },
  countIn: { x: 680, y: 19 },
  loop: { x: 713, y: 19 },
  metronome: { x: 750, y: 19 },
  kickLed: { x: 390, y: 144 },
  masterLed: { x: 461, y: 1790 },
  masterFader: { x: 461, y: 1883 },
  masterKnob: { x: 461, y: 1820 },
  insertLed: { x: 515, y: 1791 },
  exportItem: { x: 60, y: 270 },
  waveItem: { x: 262, y: 286 },
};
/** the kick's step cells of the Channel rack (the first channel of the default project): step k (1-based) in the row of the kick */
export const kickStep = (k) => ({ x: 596 + 16 * (k - 1), y: 144 });
/** the beats of one bar (steps 1, 5, 9, 13) of the kick: the sound the transport makes when the project has no other sound */
export const KICK_BEATS = [1, 5, 9, 13];
/** a square region around a place (guest px), for the picture's hash */
export const boxAt = (p, r = 14) => ({ x: p.x - r, y: p.y - r, w: 2 * r, h: 2 * r });
/** the time display (bars:beats:ticks) of the top bar: its picture changes only while the song position moves, so it tells whether the transport runs */
export const TIME_REGION = { x: 493, y: 4, w: 122, h: 31 };

export const detail = (guest) => guest.ev('window.__app.audio ? window.__app.audio.detail() : null');

/** the output's counters at one time: the quanta the worklet took (in any state), those while playing, the underrun quanta (all, warm-up included), the quanta with sound */
export async function snap(guest) {
  const d = await detail(guest);
  if (d === null) throw new Error('the page has no audio output');
  return { q: d.quantaWhileOpen + d.quantaWhilePlaying, p: d.quantaWhilePlaying, u: d.underrunQuanta, n: d.nonSilentQuanta, gap: d.engineGapFrames, skipped: d.engineSkippedFrames, d };
}

/** the differences of two snaps (the window between them). `device` counts the quanta of the output device (it is playing from the engine's start: silence when the transport stops) */
export function diff(a, b) {
  return { quanta: b.q - a.q, device: b.p - a.p, underrun: b.u - a.u, nonSilent: b.n - a.n, gapFrames: (b.gap ?? 0) - (a.gap ?? 0), skippedFrames: (b.skipped ?? 0) - (a.skipped ?? 0) };
}

/** a window of `ms` with nothing done: the counters' differences */
export async function windowOf(guest, ms) {
  const a = await snap(guest);
  await sleep(ms);
  return diff(a, await snap(guest));
}

/** whether the transport runs now over 600 ms: the time display's picture changes, or the output has sound (the metronome sounds only while the transport runs) */
export async function isPlaying(guest) {
  const a = await pixelHashRegion(guest, TIME_REGION);
  const n0 = (await snap(guest)).n;
  await sleep(600);
  const moved = (await pixelHashRegion(guest, TIME_REGION)) !== a;
  const sounded = (await snap(guest)).n - n0 >= 5;
  return moved || sounded;
}

/** waits until `pred` holds (polling every `every` ms), up to `ms`; returns whether it held */
export async function waitUntil(pred, ms, every = 200) {
  const t0 = Date.now();
  for (;;) {
    if (await pred()) return true;
    if (Date.now() - t0 >= ms) return false;
    await sleep(every);
  }
}

/** Space (the transport's key: play or stop) */
// the same key events as tools/p3-audio-probe (a keyDown with the text of the space, then the key up): the engine takes it as a key press
export async function space(guest) {
  await guest.focusDesktop();
  await guest.send('Input.dispatchKeyEvent', { type: 'keyDown', code: 'Space', key: ' ', windowsVirtualKeyCode: 32, nativeVirtualKeyCode: 32, text: ' ' });
  await sleep(40);
  await guest.send('Input.dispatchKeyEvent', { type: 'keyUp', code: 'Space', key: ' ', windowsVirtualKeyCode: 32, nativeVirtualKeyCode: 32 });
  await sleep(100);
}

/** starts the transport (Space when it is not playing) and waits until the playing quanta rise; then `warm` ms for the warm-up to pass (the first complete quantum) */
export async function playStart(guest, warm = 1500) {
  if (!(await isPlaying(guest))) await space(guest);
  const ok = await waitUntil(() => isPlaying(guest), 6000, 200);
  await sleep(warm);
  return ok;
}

/** stops the transport (Space when it is playing) and waits until the playing quanta stop */
export async function playStop(guest) {
  if (await isPlaying(guest)) await space(guest);
  return waitUntil(async () => !(await isPlaying(guest)), 6000, 200);
}

/** a click at a place (guest px) with the left button */
export const clickAt = (guest, p) => guest.click(p.x, p.y);

/** a neutral place of the guest (away from the top bar and the mixer's controls): the pointer rests here while a picture is read (a hovered control draws differently) */
export const NEUTRAL = { x: 1900, y: 2100 };
export async function rest(guest) { await guest.move(await guest.pointAt(NEUTRAL.x, NEUTRAL.y)); await sleep(200); }

/** the picture region of a place changes after the click (its hash), then the click is undone (the control is back as it was); the pointer rests away from the control at each read */
export async function clickChangesPicture(guest, p, r = 14) {
  await rest(guest);
  const h0 = await pixelHashRegion(guest, boxAt(p, r));
  await clickAt(guest, p);
  await rest(guest);
  await sleep(500);
  const h1 = await pixelHashRegion(guest, boxAt(p, r));
  await clickAt(guest, p);
  await rest(guest);
  await sleep(500);
  const h2 = await pixelHashRegion(guest, boxAt(p, r));
  return { changed: h0 !== h1, restored: h2 === h0 };
}

/** sound: the metronome is on (its button is clicked when the transport makes no sound) and the output has sound (non-silent quanta rise) */
export async function soundOn(guest, ms = 2500) {
  await playStart(guest, 0);
  const a = await snap(guest);
  await sleep(ms);
  if ((await snap(guest)).n - a.n > 20) return { sound: true, clicked: false };
  await clickAt(guest, PLACE.metronome);
  const b = await snap(guest);
  await sleep(ms);
  return { sound: (await snap(guest)).n - b.n > 20, clicked: true };
}

/** the mouse at `hz` events per second over `ms`, on a circle over the Playlist (CDP Input.dispatchMouseEvent, real moves; no button). The events are sent in
 *  batches; the achieved rate is reported (it is the rate the page received, as far as the sending goes) */
export async function mouseStorm(guest, ms, hz = 1000) {
  const g = await guest.desk();
  const cx = g.x + g.w * 0.6, cy = g.y + g.h * 0.5, rad = Math.min(g.w, g.h) * 0.2;
  const t0 = performance.now();
  let n = 0;
  while (performance.now() - t0 < ms) {
    const sends = [];
    for (let k = 0; k < 16; k++) {
      const ang = ((n + k) / hz) * 2 * Math.PI;
      sends.push(guest.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: cx + rad * Math.cos(ang), y: cy + rad * Math.sin(ang), button: 'none', buttons: 0, pointerType: 'mouse' }));
    }
    n += 16;
    await Promise.all(sends);
    const wait = t0 + (n / hz) * 1000 - performance.now();
    if (wait > 0) await sleep(wait);
  }
  const sec = (performance.now() - t0) / 1000;
  return { events: n, seconds: Math.round(sec * 100) / 100, hzAchieved: Math.round(n / sec) };
}

/** a window of `ms` while the mouse moves at `hz` (the transport plays): the counters' differences and the mouse's numbers */
export async function windowWithMouse(guest, ms, hz) {
  const a = await snap(guest);
  const m = await mouseStorm(guest, ms, hz);
  const b = await snap(guest);
  return { ...diff(a, b), mouse: m };
}

// the in-page reads of a capture: the level (peak and rms of both channels), the onsets (the mono envelope crosses 0.3 of the capture's peak, at least 100 ms after the last onset), the frames and the NaN count
const takeLevelExpression = (index) => `(() => { const a = window.__app.audio; if (!a || a.captures.length <= ${index}) return null; const c = a.captures.splice(${index}, 1)[0]; const f = c.frames; const n = f.length / 2; const env = new Float32Array(n); let pk = 0, ss = 0, cnt = 0, nan = 0; for (let i = 0; i < n; i++) { const l = f[2 * i], r = f[2 * i + 1]; if (!Number.isFinite(l) || !Number.isFinite(r)) { nan++; env[i] = 0; continue; } const al = Math.abs(l), ar = Math.abs(r); if (al > pk) pk = al; if (ar > pk) pk = ar; ss += l * l + r * r; cnt += 2; env[i] = Math.max(al, ar); } let onsets = 0, last = -1e9; const th = 0.3 * pk; for (let i = 1; i < n; i++) { if (env[i] > th && env[i - 1] <= th && i - last >= 4410) { onsets++; last = i; } } return { valid: c.valid, frames: n, validFrames: c.validFrames, peak: pk, rms: cnt ? Math.sqrt(ss / cnt) : 0, nan, onsets }; })()`;

/** a capture of `sec` seconds of the output (armed now; it starts at the first sound): { valid, frames, peak, rms, nan, onsets }, or null when it did not complete in time */
export async function captureLevel(guest, sec) {
  const frames = Math.floor(44100 * sec);
  const index = await guest.ev(`(() => { const a = window.__app.audio; const i = a.captures.length; a.armCapture(${frames}, true); return i; })()`);
  const t0 = Date.now();
  while (Date.now() - t0 < sec * 1000 + 12000) {
    const r = await guest.ev(takeLevelExpression(index)).catch(() => null);
    if (r !== null) return r;
    await sleep(250);
  }
  return null;
}

/** the PCM repeat numbers of a capture (the same as tools/p3-audio-probe measure: pcmRepeat8192 and the other lags as controls) */
export async function pcmOf(guest, ms) {
  const index = await guest.ev(armExpression());
  await sleep(ms);
  let summary = null;
  const t0 = Date.now();
  while (summary === null && Date.now() - t0 < ms + 12000) {
    summary = await guest.ev(takeExpression(index)).catch(() => null);
    if (summary === null) await sleep(250);
  }
  return { ...pcmReport(summary), summaryLags: summary ? summary.comparisons.map((c) => c.lag) : [] };
}

/** the onsets of the metronome in `sec` seconds of output (the capture's onsets) */
export async function clicksIn(guest, sec) {
  const c = await captureLevel(guest, sec);
  return c === null ? null : c.onsets;
}

/** a click with modifier keys (CDP: Alt 1, Ctrl 2, Shift 8): the Mixer's solo is Ctrl+click on the insert's LED */
export const CTRL = 2;
export async function modClick(guest, p, modifiers = 0) {
  await guest.move(p);
  await guest.send('Input.dispatchMouseEvent', { type: 'mousePressed', x: p.x, y: p.y, button: 'left', buttons: 1, clickCount: 1, modifiers });
  await sleep(60);
  await guest.send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: p.x, y: p.y, button: 'left', buttons: 0, clickCount: 1, modifiers });
  await sleep(200);
}

/** the metronome's state from its button's picture: the lit button is warm (red above 200), the unlit one grey (the Tick sound is on when lit) */
export async function metronomeOn(guest) {
  await rest(guest);
  const pic = await picture(guest);
  return pic.bytes[METRONOME_PIXEL.y * pic.stride + 4 * METRONOME_PIXEL.x] > 200;
}
/** the metronome set on or off (clicked when it is not in that state; the earlier scenarios may leave it on) */
export async function setMetronome(guest, on) {
  if ((await metronomeOn(guest)) !== on) { await clickAt(guest, PLACE.metronome); await sleep(500); }
  return metronomeOn(guest);
}
const METRONOME_PIXEL = { x: 741, y: 12 };

/** the kick's beats are on (the sound of the default project): the metronome is off (its clicks would be heard with the kick), the steps are clicked; when the kick makes no sound after that, the steps were on already and the clicks turned them off, so they are clicked again */
export async function kickSteps(guest) {
  const metronome = await setMetronome(guest, false);
  const clicks = async () => { for (const k of KICK_BEATS) { await guest.click(kickStep(k).x, kickStep(k).y); await sleep(150); } };
  await clicks();
  await rest(guest);
  await playStart(guest, WARM_MS);
  let w = await windowOf(guest, 1500);
  await playStop(guest);
  if (w.nonSilent <= SOUND_MIN) {
    await clicks();
    await rest(guest);
    await playStart(guest, WARM_MS);
    w = await windowOf(guest, 1500);
    await playStop(guest);
  }
  return { sound: w.nonSilent > SOUND_MIN, nonSilent: w.nonSilent, metronomeOn: metronome };
}

/** the output's left and right levels over `sec` seconds (the rms of each channel; the capture of the output as armCapture gives it) */
const takeStereoExpression = (index) => `(() => { const a = window.__app.audio; if (!a || a.captures.length <= ${index}) return null; const c = a.captures.splice(${index}, 1)[0]; const f = c.frames; const n = f.length / 2; let sl = 0, sr = 0, nan = 0; for (let i = 0; i < n; i++) { const l = f[2 * i], r = f[2 * i + 1]; if (!Number.isFinite(l) || !Number.isFinite(r)) { nan++; continue; } sl += l * l; sr += r * r; } return { valid: c.valid, frames: n, nan, rmsL: Math.sqrt(sl / n), rmsR: Math.sqrt(sr / n) }; })()`;
export async function captureStereo(guest, sec = 2) {
  const frames = Math.floor(44100 * sec);
  const index = await guest.ev(`(() => { const a = window.__app.audio; const i = a.captures.length; a.armCapture(${frames}, true); return i; })()`);
  const t0 = Date.now();
  while (Date.now() - t0 < sec * 1000 + 12000) {
    const r = await guest.ev(takeStereoExpression(index)).catch(() => null);
    if (r !== null) return r;
    await sleep(250);
  }
  return null;
}
/** the left-right balance of a capture: -1 (all left) to 1 (all right); 0 when the channels are equal */
export const balanceOf = (c) => (c && c.rmsL + c.rmsR > 0 ? (c.rmsL - c.rmsR) / (c.rmsL + c.rmsR) : null);

/** a WAV file's header and samples (16 or 32 bits, PCM or float): the rate, the channels, the frames, the duration, the peak and the share of non-zero samples */
export function parseWav(b) {
  if (b.length < 44 || b.toString('latin1', 0, 4) !== 'RIFF' || b.toString('latin1', 8, 12) !== 'WAVE') return { valid: false, reason: 'not a RIFF WAVE' };
  let p = 12, fmt = null, data = null;
  while (p + 8 <= b.length) {
    const id = b.toString('latin1', p, p + 4), size = b.readUInt32LE(p + 4);
    if (id === 'fmt ') fmt = { format: b.readUInt16LE(p + 8), channels: b.readUInt16LE(p + 10), rate: b.readUInt32LE(p + 12), bits: b.readUInt16LE(p + 22) };
    if (id === 'data') { data = { off: p + 8, size: Math.min(size, b.length - (p + 8)) }; break; }
    p += 8 + size + (size & 1);
  }
  if (fmt === null || data === null) return { valid: false, reason: 'no fmt or data chunk' };
  const bps = fmt.bits / 8;
  const frames = Math.floor(data.size / (fmt.channels * bps));
  let peak = 0, nonZero = 0, samples = 0;
  for (let i = 0; i + bps <= data.size; i += bps) {
    const v = fmt.bits === 16 ? b.readInt16LE(data.off + i) / 32768 : fmt.format === 3 ? b.readFloatLE(data.off + i) : NaN;
    if (!Number.isFinite(v)) return { valid: false, reason: 'unsupported sample format', ...fmt };
    const a = Math.abs(v);
    if (a > peak) peak = a;
    if (a > 0) nonZero++;
    samples++;
  }
  return { valid: true, ...fmt, frames, duration: frames / fmt.rate, peak, nonZeroFraction: samples ? nonZero / samples : 0, bytes: b.length };
}

/** File > Export > Wave file, then Start in the render form: the file the engine wrote (the page's saved files) as a WAV (parseWav). Its name and the form's left-open state are reported */
export async function exportWav(guest, { limit = 90000 } = {}) {
  await openMenu(guest, 'File');
  await guest.click(PLACE.exportItem.x, PLACE.exportItem.y);
  await sleep(900);
  await guest.click(PLACE.waveItem.x, PLACE.waveItem.y);
  const form = await waitWindow(guest, 'TWAVRenderForm', 6000);
  if (form === null) return { valid: false, reason: 'the render form did not open' };
  // a file of the same name replaces its entry (SavedFilesView), so the count does not move: the time of the last entry tells the new file
  const lastKey = `window.__app.saved.map((f) => f.at).join(',')`;
  const k0 = await guest.ev(lastKey);
  await guest.click(form.x + form.w - 38, form.y + form.h - 24);
  const got = await waitUntil(async () => (await guest.ev(lastKey)) !== k0, limit, 500);
  if (!got) return { valid: false, reason: `no file was saved within ${limit} ms` };
  const b64 = await guest.ev(`(() => { const f = window.__app.saved[window.__app.saved.length - 1]; const u = new Uint8Array(f.data); let s = ''; for (let i = 0; i < u.length; i += 0x8000) s += String.fromCharCode.apply(null, u.subarray(i, i + 0x8000)); return btoa(s); })()`);
  const name = await guest.ev('window.__app.saved[window.__app.saved.length - 1].name');
  await sleep(500);
  const leftOpen = (await windowTable(guest)).top.some((w) => w.cls === 'TWAVRenderForm');
  return { name, leftOpen, ...parseWav(Buffer.from(b64, 'base64')) };
}

/** the number of the window the sound check uses: the transport started, `warm` ms passed */
export const WARM_MS = 1500;

/** the latency bound (ms) from a key press to the first non-silent output: decided from the first measurements (pm-work report of #1491) */
export const LATENCY_BOUND_MS = 1000;
const FILL_MIN = 128;
const SILENT_MAX = 2;   // non-silent quanta a silent window may still show (a click that is still ringing at its start)
const SOUND_MIN = 20;   // non-silent quanta in a window that counts as sound

/** the checks of the audio scenarios: each takes the guest and its parameters, and returns { ok, notes, results } */
export const CHECKS = {
  async 'worklet-loaded'(guest) {
    const d = await detail(guest);
    return { ok: d.nodeReady === true && d.error === null, results: { nodeReady: d.nodeReady, status: d.status, deviceState: d.deviceState, error: d.error }, notes: [`nodeReady ${d.nodeReady}, status ${d.status}, error ${d.error ?? 'none'}`] };
  },

  async 'context-running'(guest) {
    const d = await detail(guest);
    return { ok: d.ctxState === 'running' && d.status === 'running', results: { ctxState: d.ctxState, status: d.status, autoplay: d.autoplayPolicy }, notes: [`ctxState ${d.ctxState}, status ${d.status}`] };
  },

  async 'sample-rate'(guest) {
    const r = await guest.ev('(() => { const a = window.__app.audio; return { configured: a.opts.sampleRate, ring: a.counters().rate, context: a.ctx ? a.ctx.sampleRate : null }; })()');
    const ok = r.context === r.configured && r.ring === r.configured;
    return { ok, results: { configuredRate: r.configured, contextRate: r.context, ringRate: r.ring }, notes: [`configured ${r.configured}, context ${r.context}, ring ${r.ring}`] };
  },

  async 'device-playing'(guest, { cycles = 1 } = {}) {
    const seen = [];
    for (let i = 0; i < cycles; i++) {
      await playStart(guest, 300);
      seen.push({ transport: await isPlaying(guest), device: (await detail(guest)).deviceState });
      await playStop(guest);
    }
    return { ok: seen.every((x) => x.transport && x.device === 'playing'), results: { starts: seen }, notes: [`while the transport plays: ${seen.map((x) => `transport ${x.transport}, device ${x.device}`).join('; ')}`] };
  },

  async 'keeps-after-stop'(guest) {
    await playStart(guest, 300);
    await playStop(guest);
    await sleep(800);
    const stopped = !(await isPlaying(guest));
    const w = await windowOf(guest, 1500);
    const d = await detail(guest);
    return { ok: stopped && w.quanta > 0 && w.underrun === 0 && d.deviceState === 'playing', results: { transportStopped: stopped, quanta: w.quanta, underrun: w.underrun, deviceState: d.deviceState }, notes: [`stopped ${stopped}; the device keeps taking quanta after the stop: +${w.quanta} in 1.5 s, underrun +${w.underrun}, device ${d.deviceState}`] };
  },

  async 'quanta-rise'(guest, { ms = 2000 } = {}) {
    await playStop(guest);
    const w = await windowOf(guest, ms);
    return { ok: w.quanta > 0, results: { ms, quanta: w.quanta, device: w.device, underrun: w.underrun }, notes: [`quanta +${w.quanta} in ${ms} ms (the output device +${w.device})`] };
  },

  async 'idle'(guest, { ms = 3000 } = {}) {
    await playStop(guest);
    const w = await windowOf(guest, ms);
    return { ok: w.underrun === 0 && w.quanta > 0, results: { ms, underrun: w.underrun, quanta: w.quanta }, notes: [`idle ${ms} ms: underrun +${w.underrun}, quanta +${w.quanta}`] };
  },

  async 'play-rise'(guest, { ms = 2000 } = {}) {
    await playStart(guest, WARM_MS);
    const running = await isPlaying(guest);
    await sleep(ms);
    const still = await isPlaying(guest);
    await playStop(guest);
    return { ok: running && still, results: { ms, runningAtStart: running, runningAfter: still }, notes: [`the transport runs (time display moves): ${running} at the start, ${still} after ${ms} ms`] };
  },

  async 'play-underrun'(guest, { ms = 8000 } = {}) {
    await soundOn(guest);
    await playStart(guest, WARM_MS);
    const w = await windowOf(guest, ms);
    const running = await isPlaying(guest);
    await playStop(guest);
    return { ok: running && w.underrun === 0 && w.nonSilent > SOUND_MIN, results: { ms, transport: running, underrun: w.underrun, nonSilent: w.nonSilent }, notes: [`playing ${ms} ms (transport ${running}): underrun +${w.underrun}, non-silent +${w.nonSilent}`] };
  },

  async 'mouse-underrun'(guest, { ms = 4000, hz = 1000 } = {}) {
    await soundOn(guest);
    await playStart(guest, WARM_MS);
    const w = await windowWithMouse(guest, ms, hz);
    const running = await isPlaying(guest);
    await playStop(guest);
    const ok = running && w.underrun === 0 && w.nonSilent > SOUND_MIN && w.mouse.hzAchieved >= hz / 2;
    return { ok, results: { ms, targetHz: hz, mouseEvents: w.mouse.events, mouseHzAchieved: w.mouse.hzAchieved, transport: running, underrun: w.underrun, nonSilent: w.nonSilent }, notes: [`mouse ${w.mouse.hzAchieved} Hz (target ${hz}) for ${ms} ms, transport ${running}: underrun +${w.underrun}, non-silent +${w.nonSilent}`] };
  },

  async 'mouse-idle'(guest, { ms = 2000, hz = 1000 } = {}) {
    await playStop(guest);
    const w = await windowWithMouse(guest, ms, hz);
    const ok = w.underrun === 0 && w.quanta > 0 && w.mouse.hzAchieved >= hz / 2;
    return { ok, results: { ms, targetHz: hz, mouseHzAchieved: w.mouse.hzAchieved, underrun: w.underrun, quanta: w.quanta }, notes: [`idle, mouse ${w.mouse.hzAchieved} Hz for ${ms} ms: underrun +${w.underrun}, quanta +${w.quanta}`] };
  },

  async 'cycles'(guest, { cycles = 3, playMs = 2000 } = {}) {
    await soundOn(guest);
    const runs = [];
    for (let i = 0; i < cycles; i++) {
      await playStart(guest, WARM_MS);
      const w = await windowOf(guest, playMs);
      runs.push({ transport: await isPlaying(guest), underrun: w.underrun, nonSilent: w.nonSilent });
      await playStop(guest);
    }
    return { ok: runs.every((r) => r.transport && r.underrun === 0 && r.nonSilent > SOUND_MIN), results: { cycles, playMs, runs }, notes: [`${cycles} cycles of ${playMs} ms: underrun ${runs.map((r) => r.underrun).join('/')}, non-silent ${runs.map((r) => r.nonSilent).join('/')}`] };
  },

  async 'gap-free'(guest, { ms = 5000 } = {}) {
    await soundOn(guest);
    await playStart(guest, WARM_MS);
    const w = await windowOf(guest, ms);
    const running = await isPlaying(guest);
    await playStop(guest);
    return { ok: w.gapFrames === 0 && w.skippedFrames === 0 && running, results: { ms, transport: running, gapFrames: w.gapFrames, skippedFrames: w.skippedFrames }, notes: [`engine gap +${w.gapFrames} frames, skipped +${w.skippedFrames} in ${ms} ms (transport ${running})`] };
  },

  async 'fill-margin'(guest, { ms = 4000 } = {}) {
    await soundOn(guest);
    await playStart(guest, WARM_MS);
    const t0 = Date.now();
    let min = Infinity, n = 0;
    while (Date.now() - t0 < ms) {
      const f = await guest.ev('window.__app.audio.fill()');
      if (typeof f === 'number') { min = Math.min(min, f); n++; }
      await sleep(50);
    }
    await playStop(guest);
    return { ok: n > 0 && min >= FILL_MIN, results: { ms, samples: n, fillMinFrames: min }, notes: [`fill min ${min} frames over ${n} samples (quantum ${FILL_MIN})`] };
  },

  async 'latency'(guest) {
    await soundOn(guest);
    await playStop(guest);
    await sleep(500);
    const d = await detail(guest);
    const n0 = (await snap(guest)).n;
    const t0 = performance.now();
    await space(guest);
    let at = null;
    while (performance.now() - t0 < 4000) {
      const n = (await snap(guest)).n;
      if (n > n0) { at = performance.now() - t0; break; }
    }
    await playStop(guest);
    const ms = at === null ? null : Math.round(at);
    const ok = ms !== null && ms <= LATENCY_BOUND_MS;
    return { ok, results: { latencyMs: ms, bound: LATENCY_BOUND_MS, baseLatencyMs: d.baseLatencyMs, outputLatencyMs: d.outputLatencyMs }, notes: [`Space to the first non-silent quantum ${ms ?? 'none'} ms (bound ${LATENCY_BOUND_MS}; base ${d.baseLatencyMs} ms, output ${d.outputLatencyMs} ms)`] };
  },

  async 'resume'(guest, { cycles = 2 } = {}) {
    await soundOn(guest);
    const rises = [];
    for (let i = 0; i < cycles; i++) {
      await playStop(guest);
      await sleep(500);
      await playStart(guest, 0);
      const w = await windowOf(guest, 1500);
      rises.push(w.nonSilent);
      await playStop(guest);
    }
    return { ok: rises.every((n) => n > SOUND_MIN), results: { cycles, nonSilentPerStart: rises }, notes: [`sound after each start: non-silent ${rises.join(', ')}`] };
  },

  async 'pause-resume'(guest, { sound = false } = {}) {
    if (sound) await soundOn(guest);
    await playStart(guest, WARM_MS);
    await playStop(guest);
    await sleep(800);
    const paused = !(await isPlaying(guest));
    await playStart(guest, 0);
    const resumed = await isPlaying(guest);
    await playStop(guest);
    return { ok: paused && resumed, results: { paused, resumed }, notes: [`stopped: ${!paused ? 'still running' : 'not running'}; started again: running ${resumed}`] };
  },

  async 'stop-quiet'(guest, { ms = 1000 } = {}) {
    await soundOn(guest);
    await playStart(guest, WARM_MS);
    await playStop(guest);
    await sleep(500);
    const stopped = !(await isPlaying(guest));
    const w = await windowOf(guest, ms);
    return { ok: stopped && w.nonSilent <= SILENT_MAX, results: { ms, transportStopped: stopped, nonSilent: w.nonSilent, quanta: w.quanta }, notes: [`transport stopped ${stopped}; ${ms} ms later non-silent +${w.nonSilent}`] };
  },

  async 'record-toggle'(guest, { r = 14 } = {}) {
    const c = await clickChangesPicture(guest, PLACE.record, r);
    return { ok: c.changed, results: { changed: c.changed, restored: c.restored }, notes: [`record button picture changed on the click ${c.changed}, restored by the second ${c.restored}`] };
  },

  async 'record-play'(guest) {
    await clickAt(guest, PLACE.record);
    await rest(guest);
    await playStart(guest, 0);
    const running = await isPlaying(guest);
    await playStop(guest);
    await clickAt(guest, PLACE.record);
    await rest(guest);
    return { ok: running, results: { transport: running }, notes: [`record armed, Space: transport ${running}`] };
  },

  async 'countin-toggle'(guest) {
    const c = await clickChangesPicture(guest, PLACE.countIn);
    return { ok: c.changed, results: { changed: c.changed, restored: c.restored }, notes: [`count-in picture changed ${c.changed}, restored ${c.restored}`] };
  },

  async 'countin-clicks'(guest) {
    await soundOn(guest);
    await playStop(guest);
    await clickAt(guest, PLACE.countIn);
    await clickAt(guest, PLACE.record);
    await rest(guest);
    await sleep(500);
    const a = await snap(guest);
    await space(guest);
    const rose = await waitUntil(async () => (await snap(guest)).n - a.n > SOUND_MIN, 4000, 200);
    await playStop(guest);
    await clickAt(guest, PLACE.record);
    await clickAt(guest, PLACE.countIn);
    await rest(guest);
    return { ok: rose, results: { soundDuringCountIn: rose }, notes: [`count-in on, record armed, Space: sound ${rose ? 'within 4 s' : 'not within 4 s'}`] };
  },

  async 'pattern-song-toggle'(guest, { r = 14 } = {}) {
    const c = await clickChangesPicture(guest, PLACE.pattern, r);
    return { ok: c.changed, results: { changed: c.changed, restored: c.restored }, notes: [`pattern/song button picture changed ${c.changed}, restored ${c.restored}`] };
  },

  async 'song-play'(guest) {
    await clickAt(guest, PLACE.pattern);
    await rest(guest);
    await playStart(guest, 0);
    const running = await isPlaying(guest);
    await playStop(guest);
    await clickAt(guest, PLACE.pattern);
    await rest(guest);
    return { ok: running, results: { transport: running }, notes: [`song mode (button toggled), Space: transport ${running}`] };
  },

  async 'pattern-play'(guest) {
    await playStart(guest, 0);
    const running = await isPlaying(guest);
    await playStop(guest);
    return { ok: running, results: { transport: running }, notes: [`pattern mode, Space: transport ${running}`] };
  },

  async 'loop-toggle'(guest) {
    const c = await clickChangesPicture(guest, PLACE.loop);
    return { ok: c.changed, results: { changed: c.changed, restored: c.restored }, notes: [`loop button picture changed ${c.changed}, restored ${c.restored}`] };
  },

  async 'tempo-value'(guest, { notches = 3 } = {}) {
    await rest(guest);
    const h0 = await pixelHashRegion(guest, boxAt(PLACE.bpm, 24));
    await guest.wheel(PLACE.bpm.x, PLACE.bpm.y, notches, 120);
    await rest(guest);
    await sleep(500);
    const h1 = await pixelHashRegion(guest, boxAt(PLACE.bpm, 24));
    await guest.wheel(PLACE.bpm.x, PLACE.bpm.y, notches, -120);
    await rest(guest);
    return { ok: h0 !== h1, results: { notches, changed: h0 !== h1 }, notes: [`tempo box picture changed after ${notches} wheel notches down: ${h0 !== h1}`] };
  },

  async 'tempo-clicks'(guest, { dir = 'down', notches = 20 } = {}) {
    await soundOn(guest);
    await playStart(guest, WARM_MS);
    const before = await clicksIn(guest, 8);
    const d = dir === 'down' ? 120 : -120;
    await guest.wheel(PLACE.bpm.x, PLACE.bpm.y, notches, d);
    await sleep(800);
    const after = await clicksIn(guest, 8);
    await guest.wheel(PLACE.bpm.x, PLACE.bpm.y, notches, -d);
    await sleep(800);
    await playStop(guest);
    const ok = before !== null && after !== null && (dir === 'down' ? after < before : after > before) && Math.abs(after - before) >= 2;
    return { ok, results: { dir, notches, clicksBefore8s: before, clicksAfter8s: after }, notes: [`${notches} notches ${dir}: clicks in 8 s ${before} -> ${after}`] };
  },

  async 'metronome-sound'(guest, { ms = 2000 } = {}) {
    await soundOn(guest);
    await playStart(guest, WARM_MS);
    const w = await windowOf(guest, ms);
    const running = await isPlaying(guest);
    await playStop(guest);
    return { ok: running && w.nonSilent > SOUND_MIN && w.underrun === 0, results: { ms, transport: running, nonSilent: w.nonSilent, underrun: w.underrun }, notes: [`metronome on, transport ${running}, ${ms} ms: non-silent +${w.nonSilent}, underrun +${w.underrun}`] };
  },

  async 'metronome-off'(guest) {
    await soundOn(guest);
    await playStart(guest, WARM_MS);
    await clickAt(guest, PLACE.metronome);
    await rest(guest);
    await sleep(500);
    const off = await windowOf(guest, 2000);
    const runningOff = await isPlaying(guest);
    await clickAt(guest, PLACE.metronome);
    await rest(guest);
    await sleep(500);
    const on = await windowOf(guest, 2000);
    await playStop(guest);
    return { ok: off.nonSilent <= SILENT_MAX && runningOff && on.nonSilent > SOUND_MIN, results: { offNonSilent: off.nonSilent, transportWhileOff: runningOff, onNonSilent: on.nonSilent }, notes: [`metronome off: non-silent +${off.nonSilent} (transport ${runningOff}); on again: non-silent +${on.nonSilent}`] };
  },

  async 'metronome-button'(guest, { r = 14 } = {}) {
    const c = await clickChangesPicture(guest, PLACE.metronome, r);
    return { ok: c.changed, results: { changed: c.changed, restored: c.restored }, notes: [`metronome button picture changed ${c.changed}, restored ${c.restored}`] };
  },

  async 'pcm-repeat'(guest, { lag = 8192, ms = 8000 } = {}) {
    await soundOn(guest);
    await playStart(guest, WARM_MS);
    const p = await pcmOf(guest, ms);
    await playStop(guest);
    const at = lag === 8192 ? { equalFraction: p.pcmRepeat8192, eligible: p.pcmEligible } : (p.pcmNeighbors ?? []).find((c) => c.lag === lag) ?? null;
    const share = at ? at.equalFraction : null;
    const ok = share !== null && share < 0.01 && at.eligible >= 1000 && p.pcmCaptureValid === true;
    return { ok, results: { lag, share, eligible: at ? at.eligible : null, captureValid: p.pcmCaptureValid, captureFrames: p.pcmCaptureFrames }, notes: [`lag ${lag}: equal share ${share} over ${at ? at.eligible : 0} comparisons (capture valid ${p.pcmCaptureValid})`] };
  },

  async 'pcm-capture'(guest, { ms = 8000 } = {}) {
    await soundOn(guest);
    await playStart(guest, WARM_MS);
    const p = await pcmOf(guest, ms);
    await playStop(guest);
    return { ok: p.pcmCaptureValid === true && p.pcmCaptureFrames > 0, results: { captureValid: p.pcmCaptureValid, captureFrames: p.pcmCaptureFrames }, notes: [`capture ${p.pcmCaptureFrames} frames, valid ${p.pcmCaptureValid}`] };
  },

  async 'pcm-finite'(guest, { ms = 6000 } = {}) {
    await soundOn(guest);
    await playStart(guest, WARM_MS);
    const p = await pcmOf(guest, ms);
    await playStop(guest);
    return { ok: p.pcmNonFiniteSamples === 0 && p.pcmCaptureFrames > 0, results: { nonFiniteSamples: p.pcmNonFiniteSamples, captureFrames: p.pcmCaptureFrames }, notes: [`non-finite samples ${p.pcmNonFiniteSamples} in ${p.pcmCaptureFrames} frames`] };
  },

  async 'fader'(guest, { dir = 'down', px = 120 } = {}) {
    await soundOn(guest);
    await playStart(guest, WARM_MS);
    const before = await captureLevel(guest, 2);
    const to = dir === 'down' ? PLACE.masterFader.y + px : PLACE.masterFader.y - px;
    await guest.drag(PLACE.masterFader, { x: PLACE.masterFader.x, y: to }, 12);
    await rest(guest);
    await sleep(500);
    const after = await captureLevel(guest, 2);
    await guest.drag({ x: PLACE.masterFader.x, y: to }, PLACE.masterFader, 12);
    await rest(guest);
    await sleep(500);
    await playStop(guest);
    const ok = before !== null && after !== null && (dir === 'down' ? after.peak < before.peak * 0.8 : after.rms > before.rms);
    return { ok, results: { dir, px, peakBefore: before?.peak ?? null, peakAfter: after?.peak ?? null, rmsBefore: before?.rms ?? null, rmsAfter: after?.rms ?? null }, notes: [`master fader ${dir} ${px} px: peak ${before?.peak} -> ${after?.peak}, rms ${before?.rms} -> ${after?.rms}`] };
  },

  async 'master-mute'(guest) {
    await soundOn(guest);
    await playStart(guest, WARM_MS);
    await clickAt(guest, PLACE.masterLed);
    await rest(guest);
    await sleep(500);
    const muted = await windowOf(guest, 2000);
    const running = await isPlaying(guest);
    await clickAt(guest, PLACE.masterLed);
    await rest(guest);
    await sleep(500);
    const back = await windowOf(guest, 2000);
    await playStop(guest);
    return { ok: muted.nonSilent <= SILENT_MAX && running && back.nonSilent > SOUND_MIN, results: { mutedNonSilent: muted.nonSilent, transportWhileMuted: running, backNonSilent: back.nonSilent }, notes: [`master LED off: non-silent +${muted.nonSilent} (transport ${running}); on: non-silent +${back.nonSilent}`] };
  },

  async 'channel-led'(guest, { r = 10 } = {}) {
    const c = await clickChangesPicture(guest, PLACE.kickLed, r);
    return { ok: c.changed, results: { changed: c.changed, restored: c.restored }, notes: [`channel LED picture changed ${c.changed}, restored ${c.restored}`] };
  },

  // the export: the kick's beats are played and rendered (Pattern mode, one bar at the New project's tempo). The length is one bar: 4 beats at 130 BPM (the Default
  // template's tempo, shown in the top bar); the render's tail is kept ("Leave remainder"), so the file may be a few ms longer than the bar
  async 'export-wav'(guest, { mode = 'nonsilent', tempo = 130, tolerance = 0.03 } = {}) {
    const k = await kickSteps(guest);
    const w = await exportWav(guest);
    const expectSec = (4 * 60) / tempo;
    const formatOk = w.valid === true && w.channels === 2 && w.bits === 16 && w.format === 1;
    const nonSilent = formatOk && w.peak > 0.05 && w.nonZeroFraction > 0.2;
    const lengthOk = formatOk && Math.abs(w.duration - expectSec) <= tolerance;
    const ok = k.sound && (mode === 'length' ? lengthOk : nonSilent);
    const results = { mode, kickSound: k.sound, kickNonSilent: k.nonSilent, metronomeOn: k.metronomeOn, valid: w.valid, reason: w.reason ?? null, rate: w.rate ?? null, channels: w.channels ?? null, bits: w.bits ?? null, format: w.format ?? null, frames: w.frames ?? null, durationSec: w.duration ?? null, expectedSec: expectSec, peak: w.peak ?? null, nonZeroFraction: w.nonZeroFraction ?? null, leftOpen: w.leftOpen ?? null, fileName: w.name ?? null };
    return { ok, results, notes: [`export ${w.name ?? 'none'}: ${w.valid ? `${w.channels} ch, ${w.bits} bit, ${w.rate} Hz, ${w.duration?.toFixed(4)} s (expected ${expectSec.toFixed(4)} s), peak ${w.peak?.toFixed(3)}, non-zero ${w.nonZeroFraction?.toFixed(3)}` : `not valid: ${w.reason}`}`] };
  },

  // the master knob (the big knob of the Mixer's Master track) moves the balance: a capture of each state (before, turned, back). Turned by 4 notches, the balance moves by > 0.1; back, it is within 0.05 of the start
  async 'pan-master'(guest, { notches = 4 } = {}) {
    const k = await kickSteps(guest);
    await playStart(guest, WARM_MS);
    const c0 = await captureStereo(guest, 2);
    await guest.wheel(PLACE.masterKnob.x, PLACE.masterKnob.y, notches, 120);
    await rest(guest);
    await sleep(400);
    const c1 = await captureStereo(guest, 2);
    await guest.wheel(PLACE.masterKnob.x, PLACE.masterKnob.y, notches, -120);
    await rest(guest);
    await sleep(400);
    const c2 = await captureStereo(guest, 2);
    await playStop(guest);
    const b0 = balanceOf(c0), b1 = balanceOf(c1), b2 = balanceOf(c2);
    const ok = k.sound && [b0, b1, b2].every((b) => Number.isFinite(b)) && Math.abs(b1 - b0) > 0.1 && Math.abs(b2 - b0) < 0.05;
    return { ok, results: { notches, kickNonSilent: k.nonSilent, metronomeOn: k.metronomeOn, balanceStart: b0, balanceTurned: b1, balanceBack: b2, rmsStart: c0 && [c0.rmsL, c0.rmsR], rmsTurned: c1 && [c1.rmsL, c1.rmsR] }, notes: [`master knob ${notches} notches: balance ${b0?.toFixed(3)} -> ${b1?.toFixed(3)} -> back ${b2?.toFixed(3)}`] };
  },

  // the solo of the Mixer's insert 1 (Ctrl+click on its LED): the kick sounds (the Master carries it), the solo silences the output, the second Ctrl+click brings it back
  async 'solo-insert'(guest) {
    const k = await kickSteps(guest);
    await playStart(guest, WARM_MS);
    const before = await windowOf(guest, 2000);
    await modClick(guest, PLACE.insertLed, CTRL);
    await rest(guest);
    await sleep(500);
    const soloed = await windowOf(guest, 2000);
    await modClick(guest, PLACE.insertLed, CTRL);
    await rest(guest);
    await sleep(500);
    const back = await windowOf(guest, 2000);
    await playStop(guest);
    const ok = k.sound && before.nonSilent > SOUND_MIN && soloed.nonSilent <= SILENT_MAX && back.nonSilent > SOUND_MIN;
    return { ok, results: { metronomeOn: k.metronomeOn, beforeNonSilent: before.nonSilent, soloedNonSilent: soloed.nonSilent, backNonSilent: back.nonSilent }, notes: [`solo on insert 1: non-silent +${before.nonSilent} before, +${soloed.nonSilent} soloed, +${back.nonSilent} after the second click`] };
  },
};

/** the scenario's run: the check named by `check` with its parameters; the transport is left stopped */
export async function runCheck(ctx, check, params = {}) {
  const fn = CHECKS[check];
  if (!fn) throw new Error(`no audio check ${check}`);
  const r = await fn(ctx.guest, params);
  await playStop(ctx.guest).catch(() => false);
  return { ok: r.ok === true, engineState: (await ctx.guest.state())?.state ?? null, notes: r.notes ?? [], results: r.results };
}
