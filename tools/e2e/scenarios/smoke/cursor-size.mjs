// (d) cursor-size (Issue #1480): the guest's own cursor (the Pencil of the Piano roll, a resource of FL) is shown at the page's device pixel ratio. The #desktop's computed cursor is
// image-set of the picture, and its declared resolution N is the ratio: |N - window.devicePixelRatio| <= 0.01, so one picture pixel is one device pixel (the CSS size is the picture's
// pixels ÷ N, and that times the ratio is the picture's size). Then the ratio is changed (CDP Emulation.setDeviceMetricsOverride) and the cursor must be given again for the new ratio.
// The picture's data is not written anywhere: its length, its declared resolutions and its size (from the PNG header) are.
import { closeWelcome, dismissProblems, noThanks, sleep, WELCOME, waitProjectOpen, waitStable } from '../../lib/flow.mjs';

const PIANO_ROLL = 'TEventEditForm';
const KEY_F7 = 118, KEY_D = 68;
/** the points of the Piano roll tried in turn (fractions of its rectangle: the note grid); a point under another window is skipped */
const GRID = [[0.5, 0.5], [0.3, 0.5], [0.7, 0.5], [0.5, 0.75], [0.3, 0.75], [0.7, 0.75], [0.5, 0.3], [0.8, 0.9]];
const TOL = 0.01;
const DUMP = 'window.__app.deskDumpBase64(false).then((d) => d.windows)';
const CURSOR = `getComputedStyle(document.getElementById('desktop')).cursor`;
const DPR = 'window.devicePixelRatio';
// the table's lines as tools/cursor/measure.mjs reads them: the drawn top-level windows (bottom first) and the child windows (their position is their parent's offset)
const TOP_RE = /^lid (\d+) at \((-?\d+), (-?\d+)\) (\d+)x(\d+) style 0x[0-9a-fA-F]+(.*)$/;
const CHILD_RE = /^child lid (\d+) parent lid (\d+) at \((-?\d+), (-?\d+)\) (\d+)x(\d+) vis (\d) class "([^"]*)" text "([^"]*)"\s*$/;

/** the window table: the top-level windows and the child windows, each child with its absolute position and its top-level window */
export function parseTable(text) {
  const top = [], child = [];
  for (const l of String(text).split('\n')) {
    let m = TOP_RE.exec(l);
    if (m) {
      const cls = /class "([^"]*)"/.exec(m[6]), title = /title "([^"]*)"\s*$/.exec(m[6]);
      top.push({ lid: Number(m[1]), x: Number(m[2]), y: Number(m[3]), w: Number(m[4]), h: Number(m[5]), cls: cls ? cls[1] : '', title: title ? title[1] : '' });
      continue;
    }
    m = CHILD_RE.exec(l);
    if (m) child.push({ lid: Number(m[1]), parent: Number(m[2]), px: Number(m[3]), py: Number(m[4]), w: Number(m[5]), h: Number(m[6]), vis: m[7] === '1', cls: m[8], text: m[9] });
  }
  const abs = new Map(top.map((t) => [t.lid, { x: t.x, y: t.y }]));
  for (let changed = true; changed;) {
    changed = false;
    for (const c of child) if (!abs.has(c.lid) && abs.has(c.parent)) { const p = abs.get(c.parent); abs.set(c.lid, { x: p.x + c.px, y: p.y + c.py }); changed = true; }
  }
  const parentOf = new Map(child.map((c) => [c.lid, c.parent]));
  const rootOf = (lid) => { let l = lid; for (let k = 0; k < 64 && parentOf.has(l); k++) l = parentOf.get(l); return l; };
  for (const c of child) { const a = abs.get(c.lid); if (a) { c.x = a.x; c.y = a.y; } c.root = rootOf(c.lid); }
  return { top, child };
}

/** the top-level window on top at (x, y): the table lists the drawn windows bottom first, so the last one holding the point is the top one; 0 when none */
export function topAt(top, x, y) {
  let r = 0;
  for (const t of top) if (x >= t.x && y >= t.y && x < t.x + t.w && y < t.y + t.h) r = t.lid;
  return r;
}

/** the reading of a computed cursor: its image-set entries (their declared resolutions: `x` or `dppx`), the picture's size from the PNG header, and the hot spot; the data is not kept */
export function readCursor(css) {
  const isImageSet = css.startsWith('image-set(');
  const shape = css.replace(/base64,[A-Za-z0-9+/=]+/g, 'base64,');
  const scales = [...shape.matchAll(/([0-9]*\.?[0-9]+)(?:dppx|x)\b/g)].map((m) => Number(m[1]));
  const b64 = /base64,([A-Za-z0-9+/=]+)/.exec(css)?.[1] ?? '';
  const head = Buffer.from(b64.slice(0, 32), 'base64');
  const png = head.length >= 24 && head.toString('latin1', 1, 4) === 'PNG';
  const hot = /\)\s+(-?\d+)\s+(-?\d+)\s*,\s*default\s*$/.exec(css);
  return {
    isImageSet, entries: (css.match(/url\(/g) ?? []).length, scales, shape: shape.slice(0, 200),
    picture: png ? { width: head.readUInt32BE(16), height: head.readUInt32BE(20) } : null,
    hot: hot ? [Number(hot[1]), Number(hot[2])] : null, length: css.length,
  };
}

const sameRatio = (read, dpr) => read.isImageSet && read.scales.length === 1 && Math.abs(read.scales[0] - dpr) <= TOL;

/** the cursor after it settles (two reads in a row agree: the engine answers a move at its next idle) */
async function settled(guest) {
  let last = null;
  for (let i = 0; i < 10; i++) {
    await sleep(i === 0 ? 400 : 250);
    const v = await guest.ev(CURSOR);
    if (v === last) return v;
    last = v;
  }
  return last;
}

async function table(guest) {
  for (let k = 0; ; k++) {
    try {
      return parseTable(await guest.ev(DUMP));
    } catch (e) {
      if (k >= 200 || !/ask again|no picture yet/.test(String(e?.message))) throw e;
      await sleep(100);
    }
  }
}

/** the Piano roll: the window of class TEventEditForm whose caption starts with "Piano roll" (top-level or child), in guest px with the lid of its top-level window */
async function pianoRoll(guest) {
  const t = await table(guest);
  const top = t.top.find((w) => w.cls === PIANO_ROLL && w.title.startsWith('Piano roll'));
  if (top) return { ...top, root: top.lid };
  const c = t.child.find((w) => w.cls === PIANO_ROLL && w.text.startsWith('Piano roll') && w.vis);
  return c ? { ...c, root: c.root } : null;
}

/** the pencil: the pointer is put over the Piano roll's grid until the page shows a picture cursor (image-set); the Draw key D is pressed when none does */
async function pencil(guest, notes) {
  await guest.focusDesktop();
  if (!(await pianoRoll(guest))) await guest.key('F7', 'F7', KEY_F7);
  const pr = await guest.waitFor(() => pianoRoll(guest), 20000, 'the Piano roll opens');
  if (!pr) return { ok: false, why: 'the Piano roll did not open' };
  for (let round = 0; round < 2; round++) {
    if (round === 1) {
      await guest.key('KeyD', 'd', KEY_D);
      notes.push('the Draw key D was pressed (no point of the grid showed a picture cursor)');
    }
    const t = await table(guest);
    for (const [fx, fy] of GRID) {
      const x = Math.round(pr.x + fx * pr.w), y = Math.round(pr.y + fy * pr.h);
      if (topAt(t.top, x, y) !== pr.root) continue;
      const c = await guest.pointAt(x, y);
      await guest.move(c);
      const css = await settled(guest);
      if (css.startsWith('image-set(')) return { ok: true, css, at: [x, y] };
    }
  }
  return { ok: false, why: 'no point of the Piano roll shows a picture cursor (image-set)' };
}

/** one reading of the cursor at the ratio: the row's facts (no data) and whether the cursor is the ratio's picture */
function row(phase, dpr, css) {
  const r = readCursor(css);
  const ok = sameRatio(r, dpr) && r.picture !== null;
  const n = r.scales[0] ?? null;
  const device = r.picture && n ? [Math.round((r.picture.width / n) * dpr * 1000) / 1000, Math.round((r.picture.height / n) * dpr * 1000) / 1000] : null;
  return { phase, dpr, ok, scales: r.scales, entries: r.entries, isImageSet: r.isImageSet, picture: r.picture, cssSize: r.picture && n ? [Math.round((r.picture.width / n) * 1000) / 1000, Math.round((r.picture.height / n) * 1000) / 1000] : null, devicePx: device, hot: r.hot, cursorLength: r.length, shape: r.shape };
}

/** the cursor reads the ratio, once the page has given it again (a change of the ratio is followed without a move of the pointer) */
async function waitRatio(guest, dpr, ms) {
  const t0 = Date.now();
  let css = await guest.ev(CURSOR);
  while (!sameRatio(readCursor(css), dpr) && Date.now() - t0 < ms) {
    await sleep(250);
    css = await guest.ev(CURSOR);
  }
  return css;
}

export default {
  name: 'cursor-size',
  expectFail: null,
  async run({ guest, cdp, log }) {
    const notes = [];
    const results = [];
    const first = await waitStable(guest, 180000);
    if (first.state !== 'engine-idle') return { ok: false, engineState: first.state, notes: [`first state ${first.state}`], results };
    if (await guest.findWindow((w) => w.cls === WELCOME)) {
      await noThanks(guest);
      await closeWelcome(guest);
    }
    const end = await waitProjectOpen(guest, first.title, 120000);
    if (!end.opened) return { ok: false, engineState: end.state, notes: [...notes, 'the default project did not open'], results };
    await dismissProblems(guest);

    const p1 = await pencil(guest, notes);
    if (!p1.ok) {
      notes.push(`the guest's cursor is not a picture: ${p1.why}`);
      return { ok: false, engineState: (await guest.state().catch(() => null))?.state ?? null, notes, results };
    }
    const dpr1 = await guest.ev(DPR);
    const r1 = row('first', dpr1, p1.css);
    results.push(r1);
    log(`cursor-size: ${r1.ok ? 'the picture is one device pixel per picture pixel' : 'not the ratio'}: declared ${JSON.stringify(r1.scales)} at ratio ${dpr1}, picture ${JSON.stringify(r1.picture)}, cursor length ${r1.cursorLength}`);

    // the ratio changes (the window moved to a screen of another density): the cursor is given again for it
    const [vw, vh] = await guest.ev('[window.innerWidth, window.innerHeight]');
    const next = Math.round((dpr1 * 1.25 <= 3 ? dpr1 * 1.25 : dpr1 / 1.25) * 100) / 100;
    await cdp.send('Emulation.setDeviceMetricsOverride', { width: vw, height: vh, deviceScaleFactor: Math.round(next * 100) / 100, mobile: false });
    const dpr2 = await guest.waitFor(async () => { const v = await guest.ev(DPR); return Math.abs(v - next) < 0.01 ? v : null; }, 20000, 'the ratio changes');
    if (dpr2 === null) {
      notes.push(`the ratio did not change to ${next} (it is ${await guest.ev(DPR)})`);
      return { ok: false, engineState: (await guest.state().catch(() => null))?.state ?? null, notes, results };
    }
    const css2 = await waitRatio(guest, dpr2, 15000);
    const r2 = row('after a change of the ratio', dpr2, css2);
    results.push(r2);
    log(`cursor-size: after the change of the ratio to ${dpr2}: declared ${JSON.stringify(r2.scales)}, picture ${JSON.stringify(r2.picture)}`);

    // back to the first ratio: the restart notice (#1348) shows only while the ratio's percent differs from the one FL started with; it is hidden again here (and not counted as stuck)
    const back = Math.round(dpr1 * 100) / 100;
    await cdp.send('Emulation.setDeviceMetricsOverride', { width: vw, height: vh, deviceScaleFactor: back, mobile: false });
    const dpr3 = await guest.waitFor(async () => { const v = await guest.ev(DPR); return Math.abs(v - back) < 0.01 ? v : null; }, 20000, 'the ratio is back');
    if (dpr3 === null) {
      notes.push(`the ratio did not come back to ${back} (it is ${await guest.ev(DPR)})`);
      return { ok: false, engineState: (await guest.state().catch(() => null))?.state ?? null, notes, results };
    }
    const r3 = row('back at the first ratio', dpr3, await waitRatio(guest, dpr3, 15000));
    results.push(r3);
    log(`cursor-size: back at the ratio ${dpr3}: declared ${JSON.stringify(r3.scales)}, picture ${JSON.stringify(r3.picture)}`);

    notes.push(`the guest's cursor is a picture (image-set) at the ratio ${dpr1} (declared ${JSON.stringify(r1.scales)}), at ${dpr2} (declared ${JSON.stringify(r2.scales)}) and back at ${dpr3}`);
    return { ok: r1.ok && r2.ok && r3.ok, engineState: (await guest.state().catch(() => null))?.state ?? null, notes, results };
  },
};
