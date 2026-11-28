// The measures of the perf area (Issue #1497): the start-up, the rAF windows (the least one-second window), the input-to-picture latency (the page's own latency records),
// the engine's busy fraction (busyPct), the memory (the engine's wasm memory and the JS heap), and the audio output under the mouse (underrun, quanta).
// A person's actions are real mouse and key events (lib/guest.mjs); the reads are the page's test aids (window.__app). The expectations are ratios against a baseline
// (the same run's reference measure, Issue #1509: the judge reads no golden table), never absolute values. Playwright is not imported: the browser comes from the page.
// CLI (the baseline is made from a run's output, and only then): node tools/e2e/lib/perf.mjs record <run.jsonl>... [--out FILE]   (default FILE: golden/perf/baseline.tsv)
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { sleep } from './flow.mjs';
import { closeNew, menuBarRect, menuLabelPlace, openLids, windowTable } from './common.mjs';

export { sleep } from './flow.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
export const GOLDEN_DIR = path.resolve(here, '..', 'golden', 'perf');
export const BASELINE = path.join(GOLDEN_DIR, 'baseline.tsv');
// the coarse limits (the runner's own spread is not known to be smaller): a time or a busy share may be 1.5 x the baseline, a memory growth 1.3 x, a fall of the rAF rate 1/1.5
export const LIMIT = { time: 1.5, fps: 1.5, busy: 1.5, memory: 1.3, latency: 1.5 };
export const STARTUP_RUNS = 3;
export const COLUMNS = ['item', 'value', 'unit', 'nproc', 'loadavg_before', 'loadavg_after', 'scenario'];

const round = (x, d = 3) => (typeof x === 'number' && Number.isFinite(x) ? Math.round(x * 10 ** d) / 10 ** d : null);
export const nprocNow = () => os.availableParallelism?.() ?? os.cpus().length;
export const loadNow = () => os.loadavg().map((x) => round(x, 2));
export const finite = (xs) => xs.filter((x) => typeof x === 'number' && Number.isFinite(x));

/** the median of the finite numbers (the mean of the two middle ones for an even count); null when there is none */
export function median(xs) {
  const s = finite(xs).sort((a, b) => a - b);
  if (s.length === 0) return null;
  const m = s.length >> 1;
  return s.length % 2 === 1 ? s[m] : (s[m - 1] + s[m]) / 2;
}

/** a TSV table (the baseline, the base of an A/B run): item -> { value, unit, nproc, loadavgBefore, loadavgAfter, scenario }; `#` lines are comments */
export function parseTable(text) {
  const rows = new Map();
  const lines = String(text).split(/\r?\n/).filter((l) => l.trim() !== '' && !l.startsWith('#'));
  if (lines.length < 2) return rows;
  const head = lines[0].split('\t');
  for (const l of lines.slice(1)) {
    const c = Object.fromEntries(l.split('\t').map((v, i) => [head[i], v]));
    rows.set(c.item, { value: Number(c.value), unit: c.unit, nproc: Number(c.nproc), loadavgBefore: c.loadavg_before, loadavgAfter: c.loadavg_after, scenario: c.scenario });
  }
  return rows;
}
export const readTable = (file) => (fs.existsSync(file) ? parseTable(fs.readFileSync(file, 'utf8')) : new Map());

/**
 * the ratio of one measured value to its base. kind 'upper': value / base must be at most the limit (a time, a busy share, a growth);
 * kind 'lower': base / value must be at most the limit (a fall: the value at least base / limit; a value of 0 is a fall too).
 * No base (or a base of 0): not judged (pass false, ratio null).
 */
export function ratioCheck(value, base, kind, limit) {
  if (!Number.isFinite(value) || !Number.isFinite(base) || !(base > 0)) return { ratio: null, pass: false };
  const ratio = round(value / base);
  if (kind === 'upper') return { ratio, pass: ratio <= limit };
  if (kind === 'lower') return { ratio, pass: value > 0 && base / value <= limit };
  throw new Error(`ratioCheck: kind ${kind} is upper or lower`);
}

/**
 * The judge of one scenario: the measured items (each against its reference in the same run: a Map of item -> { value }), and the results the scenario returns:
 * `items` (item -> value, unit, base, ratio, limit, kind, pass, samples), `nproc`, `loadavg` (before and after the measure). notes: one line per item.
 */
export class Judge {
  constructor(base, { nproc = nprocNow(), loadavgBefore = loadNow() } = {}) {
    this.base = base;
    this.items = {};
    this.notes = [];
    this.nproc = nproc;
    this.loadavg = { before: loadavgBefore, after: null };
  }
  /** a measured item judged against the base (kind 'upper' or 'lower'); returns whether it passes */
  ratio(item, value, { kind, limit, unit, samples = [] }) {
    const b = this.base.get(item) ?? null;
    const r = b ? ratioCheck(value, b.value, kind, limit) : { ratio: null, pass: false };
    this.items[item] = { value: round(value), unit, base: b ? b.value : null, ratio: r.ratio, limit, kind, pass: r.pass, samples: samples.map((x) => round(x)) };
    const rule = kind === 'upper' ? `at most ${limit} x base` : `at least 1/${limit} of base`;
    this.notes.push(`${item}: ${round(value)} ${unit}; base ${b ? b.value : 'none (record it first)'}; ratio ${r.ratio ?? 'n/a'} (${rule})`);
    return r.pass;
  }
  /** a growth judged by itself: after / before at most the limit (no base) */
  growth(item, before, after, limit, unit = 'bytes') {
    const r = ratioCheck(after, before, 'upper', limit);
    this.items[item] = { value: round(after), unit, base: round(before), ratio: r.ratio, limit, kind: 'growth', pass: r.pass, samples: [] };
    this.notes.push(`${item}: ${round(before)} -> ${round(after)} ${unit}; ratio ${r.ratio ?? 'n/a'} (at most ${limit})`);
    return r.pass;
  }
  /** a value that is judged as a flag (a count that must be 0, a state that must hold): the pass is the caller's */
  flag(item, value, pass, unit = 'count') {
    this.items[item] = { value: round(value), unit, base: null, ratio: null, limit: null, kind: 'flag', pass: pass === true, samples: [] };
    this.notes.push(`${item}: ${round(value)} ${unit} (${pass === true ? 'as expected' : 'not as expected'})`);
    return pass === true;
  }
  results() {
    this.loadavg.after = loadNow();
    return { items: this.items, nproc: this.nproc, loadavg: this.loadavg };
  }
}

/** the page's state for the start-up poll: the state, the first idle mark (ms since the navigation started) and the page's time origin */
const STARTUP_POLL = `(() => { const a = window.__app; if (!a) return null; return { state: a.state, idle: typeof a.timeline.idle === 'number' ? a.timeline.idle : null, origin: performance.timeOrigin, error: a.error ? String(a.error.what ?? a.error.message ?? '').slice(0, 200) : null }; })()`;

/**
 * One start-up in a page of its own (a new context: its own profile, as boot-check does): from the navigation to the engine's first idle mark (firstIdleMs, the
 * same definition as tools/startup-breakdown: the page's timeline.idle) and to the state engine-idle (engineIdleMs, polled every 50 ms).
 */
export async function startupOnce(browser, origin, viewport, dpr) {
  const context = await browser.newContext({ viewport, deviceScaleFactor: dpr, screen: viewport });
  const out = { engineIdleMs: null, firstIdleMs: null, state: null, error: null };
  try {
    const page = await context.newPage();
    const t0 = Date.now();
    await page.goto(`${origin}/`, { waitUntil: 'commit', timeout: 120000 });
    for (;;) {
      const s = await page.evaluate(STARTUP_POLL).catch(() => null);
      if (s) {
        out.state = s.state;
        if (s.idle !== null && out.firstIdleMs === null) out.firstIdleMs = s.origin + s.idle - t0;
        if (s.state === 'engine-idle') { out.engineIdleMs = Date.now() - t0; break; }
        if (s.state === 'error' || s.state === 'exited') { out.error = `state ${s.state}${s.error ? `: ${s.error}` : ''}`; break; }
      }
      if (Date.now() - t0 >= 180000) { out.error = `no engine-idle within 180 s (state ${out.state ?? 'window.__app not set'})`; break; }
      await sleep(50);
    }
    if (out.engineIdleMs !== null) out.firstIdleMs = round(out.firstIdleMs, 1);
    out.engineIdleMs = round(out.engineIdleMs, 1);
  } catch (e) {
    out.error = String(e?.message ?? e).slice(0, 300);
  } finally {
    await context.close().catch(() => {});
  }
  return out;
}

/** the start-up runs of one scenario: STARTUP_RUNS starts in turn; each run's numbers, and the medians */
export async function startupRuns(browser, origin, viewport, dpr, runs = STARTUP_RUNS) {
  const list = [];
  for (let i = 0; i < runs; i++) list.push(await startupOnce(browser, origin, viewport, dpr));
  return { runs: list, engineIdle: list.map((r) => r.engineIdleMs), firstIdle: list.map((r) => r.firstIdleMs), error: list.find((r) => r.error)?.error ?? null };
}

const RAF = `(() => ({ n: window.__e2eRaf ? window.__e2eRaf.n : 0, t: performance.now() }))()`;
/** the page's rAF counter read every `every` ms while `active()` holds (the counter is installed by run.mjs for the shared page) */
export async function rafWhile(page, active, every = 100) {
  const out = [];
  while (active()) {
    out.push(await page.evaluate(RAF));
    await sleep(every);
  }
  out.push(await page.evaluate(RAF));
  return out;
}

/** the least rate over the one-second windows of the samples (rAF callbacks per second): for each sample, the first later sample at least 1 s on; null with less than 1 s */
export function minWindowRate(samples, windowMs = 1000) {
  let min = null;
  for (let i = 0; i < samples.length; i++) {
    const a = samples[i];
    const b = samples.slice(i + 1).find((s) => s.t - a.t >= windowMs);
    if (!b) break;
    const rate = (b.n - a.n) / ((b.t - a.t) / 1000);
    if (min === null || rate < min) min = rate;
  }
  return min;
}

/** the mean rate of the samples (callbacks per second) */
export function meanRate(samples) {
  if (samples.length < 2) return null;
  const a = samples[0], b = samples[samples.length - 1];
  return b.t > a.t ? (b.n - a.n) / ((b.t - a.t) / 1000) : null;
}

/**
 * The mouse moved by the CDP Input events at `hz` (the events are sent on the clock, catch-up: not one timer per event), on an ellipse over the desktop
 * (0.5 turn a second). Returns the events sent. The page's count of the pointer moves it received is `pointerCount` (installed once per page).
 */
export async function moveMouse(guest, { ms, hz, cx = 0.5, cy = 0.5, rx = 0.12, ry = 0.12 }) {
  const d = await guest.desk();
  let sent = 0;
  const t0 = performance.now();
  while (performance.now() - t0 < ms) {
    const due = Math.floor(((performance.now() - t0) * hz) / 1000);
    while (sent < due) {
      const th = Math.PI * (sent / hz);
      const x = d.x + (cx + rx * Math.cos(th)) * d.w;
      const y = d.y + (cy + ry * Math.sin(th)) * d.h;
      guest.cdp.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x, y, button: 'none', buttons: 0, pointerType: 'mouse' }).catch(() => {});
      sent++;
    }
    await sleep(1);
  }
  return { sent, seconds: (performance.now() - t0) / 1000 };
}

const POINTER_INSTALL = `(() => { if (!window.__perfPM) { window.__perfPM = { n: 0 }; document.addEventListener('pointermove', () => { window.__perfPM.n++; }, true); } return window.__perfPM.n; })()`;
export const pointerCount = (guest) => guest.ev(POINTER_INSTALL);

/** a menu label's place in the window table (the File label is the first), and a click on it */
export async function clickLabel(guest, label) {
  const bar = menuBarRect(await windowTable(guest));
  if (!bar) throw new Error('the menu bar (TNewMenu) is not in the window table');
  const p = menuLabelPlace(bar, label);
  await guest.click(p.x, p.y);
}

/** a drop-down opened by the label and closed again (the windows new since the base are closed by Escape) */
export async function openAndClose(guest, label) {
  const base = await openLids(guest);
  await clickLabel(guest, label);
  await sleep(600);
  await closeNew(guest, base);
  await sleep(200);
}

const RECORDS = `(() => { const L = window.__app.latency.records; return L.map((r) => { let at = -1; for (const f of Object.values(r.answers || {})) if (at < 0 || f.at < at) at = f.at; return [r.seq, r.tEvent ?? r.tPage, r.dropped !== null, at]; }); })()`;

/**
 * The input-to-picture latency of one input: the act (a click or a key) is sent, and the page's own records (the input's DOM time to the frame that answered it,
 * the same as tools/p2-frames) give the time to the first frame that answered one of the inputs the act sent. null when none answered within `waitMs`.
 */
export async function inputLatency(guest, act, waitMs = 3000) {
  const seen = new Set((await guest.ev(RECORDS)).map((r) => r[0]));
  await act();
  const deadline = Date.now() + waitMs;
  while (Date.now() < deadline) {
    const fresh = (await guest.ev(RECORDS)).filter((r) => !seen.has(r[0]) && !r[2] && r[3] >= 0 && r[3] >= r[1]);
    if (fresh.length > 0) return Math.min(...fresh.map((r) => r[3] - r[1]));
    await sleep(40);
  }
  return null;
}

/** the busy share of the engine over `ms` (busyPct: the sum of the busyMs of the idle reports in the window over the window's wall time, as tools/topbar-metrics) */
const IDLES = `(() => { const a = window.__app; return { abs: performance.timeOrigin + performance.now(), idles: a.idles.map((x) => [x.at, x.busyMs]) }; })()`;
export async function busyPctOver(page, ms) {
  const a = await page.evaluate(IDLES);
  await sleep(ms);
  const b = await page.evaluate(IDLES);
  const span = b.abs - a.abs;
  const inside = b.idles.filter(([at]) => at > a.abs && at <= b.abs);
  const busy = inside.reduce((s, [, m]) => s + (typeof m === 'number' ? m : 0), 0);
  return { pct: span > 0 ? (100 * busy) / span : null, idles: inside.length, seconds: round(span / 1000, 2) };
}

/** the engine's wasm memory in bytes (it only grows) and the page's JS heap in bytes */
export async function memoryNow(guest, cdp) {
  const wasm = await guest.ev('window.__app.engineMemoryBytes ?? null');
  await cdp.send('HeapProfiler.collectGarbage').catch(() => {});
  await cdp.send('Performance.enable').catch(() => {});
  const m = await cdp.send('Performance.getMetrics').catch(() => ({ metrics: [] }));
  const js = m.metrics.find((x) => x.name === 'JSHeapUsedSize')?.value ?? null;
  return { wasm: typeof wasm === 'number' ? wasm : null, js };
}

export const AUDIO = `(() => { const a = window.__app; return a && a.audio ? a.audio.detail() : null; })()`;
const tapSpace = async (guest) => {
  await guest.send('Input.dispatchKeyEvent', { type: 'keyDown', code: 'Space', key: ' ', windowsVirtualKeyCode: 32, nativeVirtualKeyCode: 32, text: ' ' });
  await sleep(40);
  await guest.send('Input.dispatchKeyEvent', { type: 'keyUp', code: 'Space', key: ' ', windowsVirtualKeyCode: 32, nativeVirtualKeyCode: 32 });
  await sleep(60);
};
const metronome = (guest, dpr) => guest.click(Math.round(750 * dpr), Math.round(19 * dpr));
async function audioWhen(guest, pred, ms) {
  const t0 = Date.now();
  for (;;) {
    const d = await guest.ev(AUDIO).catch(() => null);
    if (d && pred(d)) return d;
    if (Date.now() - t0 >= ms) return null;
    await sleep(250);
  }
}

/**
 * The output plays: the audio output is started by a key press of the page (F24: a gesture, as tools/p2-frames load b), then Space starts the transport and the
 * metronome button of the top bar is pressed (its clicks make sound with or without a project; the default project is silent without it). Returns the
 * output's numbers and whether the non-silent quanta rose (the sound is on). stopSound() puts both back.
 */
export async function startSound(guest, dpr) {
  const running = (d) => d.ctxState === 'running' && d.nodeReady === true;
  await guest.key('F24', 'F24', 0x87);
  let d = await audioWhen(guest, running, 5000);
  if (!d) {
    await guest.key('ShiftLeft', 'Shift', 16);
    d = await audioWhen(guest, running, 5000);
  }
  if (!d) return { playing: false, why: 'the audio output did not start (no gesture took)', start: null };
  await guest.focusDesktop();
  await tapSpace(guest);
  await sleep(500);
  const n0 = (await guest.ev(AUDIO))?.nonSilentQuanta ?? 0;
  await metronome(guest, dpr);
  const rose = await audioWhen(guest, (x) => (x.nonSilentQuanta ?? 0) > n0 + 20, 6000);
  return { playing: rose !== null, why: rose ? null : `no sound: nonSilentQuanta stayed at ${n0}`, start: n0 };
}
export async function stopSound(guest, dpr) {
  await guest.focusDesktop();
  await tapSpace(guest);
  await metronome(guest, dpr);
  await sleep(300);
}

/** the pointer moves received by the page over the send (pointer events the page took: per second) */
export async function receivedHz(guest, before, ms) {
  return ms > 0 ? round(((await pointerCount(guest)) - before) / (ms / 1000), 1) : null;
}

/** the base of a ratio measured in the same run (Issue #1509): the reference is the same measure under the reference condition, taken in this run (no table is read) */
export const sameRunBase = (item, value) => new Map([[item, { value }]]);
const dprOf = (page) => page.evaluate(() => window.devicePixelRatio);

/**
 * The start-up medians of the scenario's display (its page's viewport and device pixel ratio). Two sets of start-ups in this run: the first set is the reference
 * (the same operation on the same machine, the sets' medians), and the second set is judged against it as the items startup.engineIdle.ms and startup.firstIdle.ms
 * (the ratio at most the limit). No golden table is read (Issue #1509).
 */
export async function measureStartup(ctx) {
  const viewport = ctx.page.viewportSize();
  const dpr = await dprOf(ctx.page);
  const browser = ctx.page.context().browser();
  const ref = await startupRuns(browser, ctx.origin, viewport, dpr);
  const s = await startupRuns(browser, ctx.origin, viewport, dpr);
  const eng = median(s.engineIdle), first = median(s.firstIdle);
  const j = new Judge(new Map([...sameRunBase('startup.engineIdle.ms', median(ref.engineIdle)), ...sameRunBase('startup.firstIdle.ms', median(ref.firstIdle))]));
  const ok1 = s.error === null && ref.error === null && eng !== null && j.ratio('startup.engineIdle.ms', eng, { kind: 'upper', limit: LIMIT.time, unit: 'ms', samples: s.engineIdle });
  const ok2 = s.error === null && ref.error === null && first !== null && j.ratio('startup.firstIdle.ms', first, { kind: 'upper', limit: LIMIT.time, unit: 'ms', samples: s.firstIdle });
  j.notes.push(`reference: the same run's first ${ref.runs.length} starts: engine-idle ${ref.engineIdle.join(', ')} ms; first idle ${ref.firstIdle.join(', ')} ms`);
  j.notes.push(`runs ${s.runs.length} at ${viewport.width}x${viewport.height} DPR ${dpr}: engine-idle ${s.engineIdle.join(', ')} ms; first idle ${s.firstIdle.join(', ')} ms${s.error ? `; error ${s.error}` : ''}${ref.error ? `; reference error ${ref.error}` : ''}`);
  return { ok: ok1 && ok2, results: j.results(), notes: j.notes, state: (await ctx.guest.state())?.state ?? null };
}

/**
 * The rAF's least one-second window (and the mean) while `during` runs (a mouse for `ms`, the menus, New); with no `during`, the page is only watched for `ms`.
 * Judged as the item against the base (kind lower: the fall is at most the limit).
 */
export async function measureFps(ctx, item, ms, during) {
  const quiet = await rafSpan(ctx.page, 8000, null);
  const j = new Judge(sameRunBase(item, quiet.least));
  const span = await rafSpan(ctx.page, ms, during);
  const { least, mean, samples } = span;
  const ok = least !== null && quiet.least !== null && j.ratio(item, least, { kind: 'lower', limit: LIMIT.fps, unit: 'rAF/s (least 1 s window)', samples: [least] });
  j.notes.push(`reference: the same run with no input, 8 s: least ${quiet.least === null ? 'none' : round(quiet.least, 2)} rAF/s; mean ${round(mean, 2)} rAF/s over ${round((samples[samples.length - 1].t - samples[0].t) / 1000, 1)} s; ${samples.length} samples`);
  return { ok, results: j.results(), notes: j.notes, state: (await ctx.guest.state())?.state ?? null };
}

/** the rAF samples while `during` runs (or for `ms` with no input): the least one-second window, the mean and the samples */
async function rafSpan(page, ms, during) {
  let done = false;
  const work = (during ? during() : sleep(ms)).finally(() => { done = true; });
  const samples = await rafWhile(page, () => !done);
  await work;
  return { least: minWindowRate(samples), mean: meanRate(samples), samples };
}

/**
 * The input-to-picture latency of the act, `n` times (each act, then the close by Escape): the median is the item's value. The act is a click on a label (the
 * drop-down opens), or a key with the drop-down open. Returns the latencies and their median.
 */
export async function measureLatency(guest, { n, act, close = true, prepare = null }) {
  const out = [];
  for (let i = 0; i < n; i++) {
    const base = await openLids(guest);
    if (prepare) await prepare();
    out.push(await inputLatency(guest, act));
    if (close) { await closeNew(guest, base); await sleep(250); }
  }
  return { values: out, median: median(out) };
}

/** a click on a menu label, `n` times: the click's input-to-picture latency */
export const menuLatency = (guest, label, n = 5, prepare = null) => measureLatency(guest, { n, act: () => clickLabel(guest, label), prepare });

/** the key latency: the drop-down of File is open and ArrowDown is pressed `n` times (each press is one input) */
export async function keyLatency(guest, n = 10, prepare = null) {
  const base = await openLids(guest);
  await clickLabel(guest, 'File');
  await sleep(800);
  const out = [];
  for (let i = 0; i < n; i++) {
    if (prepare) await prepare();
    out.push(await inputLatency(guest, () => guest.key('ArrowDown', 'ArrowDown', 40)));
    await sleep(150);
  }
  await closeNew(guest, base);
  await sleep(250);
  return { values: out, median: median(out) };
}

/**
 * The sound of the page, for a time, with or without the mouse at `hz` (CDP mouse moves; 0 for none): the underrun and the quanta of the output over the time.
 * Returns the sound's start (startSound) and the output's counts over the time.
 */
export async function soundRun(ctx, { ms, hz = 0 }) {
  const guest = ctx.guest;
  const dpr = await dprOf(ctx.page);
  const s = await startSound(guest, dpr);
  if (!s.playing) return { s, playing: false };
  const d0 = await guest.ev(AUDIO);
  const pm0 = await pointerCount(guest);
  const mv = hz > 0 ? await moveMouse(guest, { ms, hz }) : (await sleep(ms), null);
  const d1 = await guest.ev(AUDIO);
  const pmHz = await receivedHz(guest, pm0, ms);
  await stopSound(guest, dpr);
  const delta = (k) => (typeof d1?.[k] === 'number' && typeof d0?.[k] === 'number' ? d1[k] - d0[k] : null);
  return { s, playing: true, underrun: delta('underrunQuanta'), quanta: delta('quantaWhilePlaying'), nonSilent: delta('nonSilentQuanta'), mouseSent: mv?.sent ?? 0, mouseSeconds: mv ? round(mv.seconds, 2) : null, receivedHz: pmHz, status: d1?.status ?? null, error: d1?.error ?? null };
}

/** the table of the baseline as TSV lines: one row per measured item of the results (the value is the item's value; the load and the nproc are the scenario's) */
export function tableRows(rows) {
  const out = [COLUMNS.join('\t')];
  for (const r of rows) {
    out.push([r.item, r.value, r.unit, r.nproc, (r.loadavgBefore ?? []).join(','), (r.loadavgAfter ?? []).join(','), r.scenario].join('\t'));
  }
  return out;
}

/** the rows of the items of the runs' scenarios (JSON lines of run.mjs: each row's scenarios with their results.items), the last one of an item wins */
export function rowsOfRuns(lines) {
  const byItem = new Map();
  for (const line of lines) {
    if (!line.trim()) continue;
    const row = JSON.parse(line);
    for (const s of row.scenarios ?? []) {
      const r = s.results;
      if (!r || !r.items) continue;
      for (const [item, v] of Object.entries(r.items)) {
        if (typeof v.value !== 'number' || (v.kind !== 'upper' && v.kind !== 'lower')) continue;
        byItem.set(item, { item, value: v.value, unit: v.unit, nproc: r.nproc, loadavgBefore: r.loadavg?.before ?? [], loadavgAfter: r.loadavg?.after ?? [], scenario: s.id });
      }
    }
  }
  return [...byItem.values()].sort((a, b) => (a.item < b.item ? -1 : a.item > b.item ? 1 : 0));
}

/** the record: a table written from the runs' output (the baseline, or the A/B base). Writes the file; returns its lines */
export function recordTable(lines, outFile, note) {
  const rows = rowsOfRuns(lines);
  if (rows.length === 0) throw new Error('record: the runs have no measured item (results.items)');
  const head = [`# ${note}`, `# recorded from the runs' output (tools/e2e/lib/perf.mjs record); the values are the runner's (Actions), not this machine's`];
  const text = [...head, ...tableRows(rows)].join('\n') + '\n';
  fs.mkdirSync(path.dirname(outFile), { recursive: true });
  fs.writeFileSync(outFile, text);
  return rows.length;
}

const isMain = process.argv[1] && pathToFileURL(path.resolve(process.argv[1])).href === import.meta.url;
if (isMain) {
  const argv = process.argv.slice(2);
  const oi = argv.indexOf('--out');
  const out = oi >= 0 ? path.resolve(argv[oi + 1]) : BASELINE;
  const files = argv.filter((a, i) => a !== 'record' && a !== '--out' && (oi < 0 || i !== oi + 1));
  if (argv[0] !== 'record' || files.length === 0) {
    process.stderr.write('usage: node tools/e2e/lib/perf.mjs record <run.jsonl>... [--out FILE]\n');
    process.exit(2);
  }
  const lines = files.flatMap((f) => fs.readFileSync(f, 'utf8').split('\n'));
  const n = recordTable(lines, out, `the baseline of the perf area (Issue #1497). Each value is the median of the scenario's runs; the nproc and loadavg are the scenario's before and after.`);
  process.stdout.write(`${out}: ${n} items\n`);
}
