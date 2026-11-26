// The durability scenarios' parts (Issue #1495): the seeded random operation columns, the repeated actions (a window opened and closed, a menu, a project replaced, a reload,
// a tab hidden) and the reads of the idle measures. A person's actions are real mouse and key events (lib/guest.mjs); the reads are the page's test aids
// (the window table, the picture, the engine's memory, the audio's numbers). The seed of a scenario is the hash of its ID, so the same ID gives the same columns.
// The pure parts (seedOf, rngOf, planOps) need no browser: the tests and --list can use them.
import path from 'node:path';
import { windowTable, picture, persist, MAIN_FORM } from './common.mjs';
import { sleep } from './flow.mjs';
import { installToastWatch } from './detect.mjs';
import { stopOf } from './stop.mjs';

export const MEM_RATIO = 1.1;
export const DUMP_RETRY_LIMIT = 200;
export const DEMO_PROJECT = path.join('Data', 'Demo projects', 'Demo songs', 'DEDZII - NIGHT TERROR.flp');
export const OP_WEIGHTS = { left: 26, right: 8, double: 8, drag: 12, wheel: 10, fkey: 10, arrow: 8, escape: 5, enter: 4, ctrl: 5, space: 4 };
export const OPS = 200;
/** the features of the input kinds (features/resilience.tsv): every seeded column covers all of them */
export const INPUT_FEATURES = [
  'resilience/input/mouse-left-click', 'resilience/input/mouse-right-click', 'resilience/input/mouse-double-click', 'resilience/input/mouse-drag',
  'resilience/input/mouse-wheel', 'resilience/input/key-f-keys', 'resilience/input/key-arrow', 'resilience/input/key-escape', 'resilience/input/key-enter',
  'resilience/input/key-ctrl', 'resilience/input/key-space',
];
const ARROWS = [['ArrowLeft', 37], ['ArrowUp', 38], ['ArrowRight', 39], ['ArrowDown', 40]];
const CTRL = [{ letter: 'Z', vk: 90 }, { letter: 'Y', vk: 89 }, { letter: 'A', vk: 65 }];

/** the seed of a scenario: FNV-1a (32 bits) of its ID */
export function seedOf(text) {
  let h = 0x811c9dc5;
  const s = String(text);
  for (let i = 0; i < s.length; i++) h = Math.imul(h ^ s.charCodeAt(i), 0x01000193) >>> 0;
  return h >>> 0;
}

/** mulberry32: a generator of numbers in [0, 1) from a 32-bit seed (the same seed, the same numbers) */
export function rngOf(seed) {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/** n operations from the seed: each is { kind, x, y, x2, y2, pick } with the places as fractions of the guest's picture (so no size is fixed here) */
export function planOps(seed, n, weights = OP_WEIGHTS) {
  const rng = rngOf(seed);
  const kinds = Object.keys(weights);
  const total = kinds.reduce((s, k) => s + weights[k], 0);
  const out = [];
  for (let i = 0; i < n; i++) {
    const r = rng() * total;
    let acc = 0, kind = kinds[kinds.length - 1];
    for (const k of kinds) {
      acc += weights[k];
      if (r < acc) { kind = k; break; }
    }
    out.push({ kind, x: rng(), y: rng(), x2: rng(), y2: rng(), pick: rng() });
  }
  return out;
}

/** a key by its name: [the key's code, its Windows virtual key] (the F keys as the engine reads them: F5 is 116) */
export const KEYS = {
  Escape: ['Escape', 27], Enter: ['Enter', 13], Space: ['Space', 32],
  ArrowLeft: ['ArrowLeft', 37], ArrowUp: ['ArrowUp', 38], ArrowRight: ['ArrowRight', 39], ArrowDown: ['ArrowDown', 40],
  F5: ['F5', 116], F6: ['F6', 117], F7: ['F7', 118], F8: ['F8', 119], F9: ['F9', 120], F10: ['F10', 121], F11: ['F11', 122], F12: ['F12', 123],
};

/** one key press as a person's: the key's name is its code, except Space (the space character) */
export async function press(guest, name) {
  const [code, vk] = KEYS[name];
  return guest.key(code, name === 'Space' ? ' ' : code, vk);
}

/** one operation of the plan, on the guest's picture as it is now (the fractions are scaled to its size); returns what it did, in guest px */
export async function runOp(guest, op) {
  const d = await guest.desk();
  const px = (f, n) => Math.min(n - 1, Math.floor(f * n));
  const gx = px(op.x, d.gw), gy = px(op.y, d.gh);
  switch (op.kind) {
    case 'left': await guest.click(gx, gy); return `left click at ${gx},${gy}`;
    case 'right': await guest.rightClick(gx, gy, 300); return `right click at ${gx},${gy}`;
    case 'double': await guest.dblclick(gx, gy); return `double click at ${gx},${gy}`;
    case 'drag': { const to = { x: px(op.x2, d.gw), y: px(op.y2, d.gh) }; await guest.drag({ x: gx, y: gy }, to, 8); return `drag ${gx},${gy} to ${to.x},${to.y}`; }
    case 'wheel': { const n = 1 + Math.floor(op.pick * 3); await guest.wheel(gx, gy, n, op.pick < 0.5 ? 120 : -120); return `wheel ${n} at ${gx},${gy}`; }
    case 'fkey': { const k = `F${5 + Math.floor(op.pick * 8)}`; await press(guest, k); return `key ${k}`; }
    case 'arrow': { const [name, vk] = ARROWS[Math.floor(op.pick * 4)]; await guest.key(name, name, vk); return `key ${name}`; }
    case 'escape': await press(guest, 'Escape'); return 'key Escape';
    case 'enter': await press(guest, 'Enter'); return 'key Enter';
    case 'ctrl': { const { letter, vk } = CTRL[Math.floor(op.pick * 3)]; await guest.ctrl(`Key${letter}`, letter.toLowerCase(), vk); return `key Ctrl+${letter}`; }
    case 'space': await press(guest, 'Space'); return 'key Space';
    default: throw new Error(`runOp: no kind ${op.kind}`);
  }
}

/** the window table reads (the page answers "ask again" while it is drawing; the table is asked again for up to 60 s) */
export async function readable(guest) {
  return windowTable(guest).then(() => true, () => false);
}

/** the windows of the table, top-level and child, each with its name (the title, or the child's text) */
export function allWindows(table) {
  return [...table.top.map((w) => ({ ...w, name: w.title })), ...table.children.map((w) => ({ ...w, name: w.text }))];
}

/** whether a window of the class is shown now: in the table, and for a child window its visible flag is set (a top-level window is shown when it is listed) */
export async function hasWindow(guest, cls) {
  return allWindows(await windowTable(guest)).some((w) => w.cls === cls && (w.vis === undefined || w.vis === true));
}

/** whether the main form is in the table now */
export async function mainFormHere(guest) {
  return (await windowTable(guest)).top.some((w) => w.cls === MAIN_FORM);
}

/** polls `pred` every `every` ms until it holds (true) or `ms` has passed (false) */
export async function waitUntil(pred, ms, every = 150) {
  const t0 = Date.now();
  for (;;) {
    if (await pred().catch(() => false)) return true;
    if (Date.now() - t0 >= ms) return false;
    await sleep(every);
  }
}

/** the engine's state name now ('unreadable' when the page does not answer) */
export async function stateName(guest) {
  return (await guest.state().catch(() => null))?.state ?? 'unreadable';
}

/** the engine's wasm memory in bytes as of its last idle report (null before the first one) */
export const memoryBytes = (guest) => guest.ev('window.__app.engineMemoryBytes ?? null');

/** the audio output's numbers (null when the page has no audio output) */
export const audioNumbers = (guest) => guest.ev('window.__app.audio ? window.__app.audio.detail() : null');

/** the JS heap of the page (Chromium's own figure; null when the browser does not give it) */
export const heapBytes = (guest) => guest.ev('(performance.memory ? performance.memory.usedJSHeapSize : null)');

/** runs the operations in order. The engine's state is read after every operation (a stop is taken to the operation that came before it: `stoppedAt`), and the window table
 *  every `checkEvery` operations and at the end; a stopped engine ends the run */
export async function runOps(guest, ops, { checkEvery = 20 } = {}) {
  const counts = {};
  const reads = { ok: 0, fail: 0 };
  const retries0 = guest.dumpRetries;
  let done = 0, stops = 0, finalState = null, stoppedAt = null;
  for (let i = 0; i < ops.length; i++) {
    const what = await runOp(guest, ops[i]);
    counts[ops[i].kind] = (counts[ops[i].kind] ?? 0) + 1;
    done++;
    finalState = await stateName(guest);
    if (finalState === 'error' || finalState === 'exited') { stops++; stoppedAt = { index: i, what }; break; }
    if (done % checkEvery === 0 || done === ops.length) {
      if (await readable(guest)) reads.ok++; else reads.fail++;
    }
  }
  return { done, counts, reads, stops, finalState, stoppedAt, dumpRetries: guest.dumpRetries - retries0 };
}

/** a scenario starts on a running engine: when an earlier scenario of this page stopped it (the page's engine is then exited or error), the page is loaded again
 *  (the start runs again to the idle engine and the default project). The load waits 45 s first: a load soon after a stop found the stopped engine's files still held
 *  (`getSize failed` on a sync access handle, the start is an error), and then the load is tried again after 45 s, 3 times at most. Returns null when nothing had to be done */
export async function startIfStopped(guest, page, log = (_m) => {}) {
  const s = await stateName(guest);
  if (s !== 'error' && s !== 'exited') return null;
  const tries = [];
  let last = { state: s, opened: false };
  for (let k = 0; k < 3; k++) {
    await sleep(45000);
    last = await persist(page, guest);
    await installToastWatch(page);
    tries.push(last.state);
    if (last.state === 'engine-idle') break;
  }
  log(`the engine was ${s} (an earlier scenario of this page stopped it); the page was loaded ${tries.length} time(s): states ${tries.join(', ')}, project open ${last.opened}`);
  return { found: s, tries, state: last.state, opened: last.opened };
}

/** a window opened and closed `n` times: each cycle presses `open` until the class is in the table, then presses the close keys (the first that closes it) */
export async function cycleWindow(guest, { cls, open, close }, n) {
  const out = { cycles: 0, opened: 0, closed: 0, fails: [], memory: [] };
  for (let i = 1; i <= n; i++) {
    if (!(await hasWindow(guest, cls))) await press(guest, open);
    const shown = await waitUntil(() => hasWindow(guest, cls), 3000);
    out.cycles = i;
    if (!shown) { out.fails.push({ cycle: i, step: 'open' }); continue; }
    out.opened++;
    let gone = false;
    for (const k of close) {
      await press(guest, k);
      gone = await waitUntil(async () => !(await hasWindow(guest, cls)), 2500);
      if (gone) break;
    }
    if (gone) out.closed++; else out.fails.push({ cycle: i, step: 'close' });
    if (i === 1 || i === n) out.memory.push({ cycle: i, bytes: await memoryBytes(guest) });
  }
  return out;
}

/** the engine's stop as lib/stop.mjs reads it from the page's log (taken before any load of the page, which clears the log); null when the engine is not stopped */
export async function stopReport(guest) {
  const st = await guest.state().catch(() => null);
  if (!st || (st.state !== 'error' && st.state !== 'exited')) return null;
  const lines = await guest.ev('window.__app.engineLog()').catch(() => []);
  return stopOf(lines, st.exit ?? null);
}

/** a column of seeded operations (the ID's seed; `weights` picks the kinds): the table is read every 20 operations, and the engine must be idle at the end.
 *  A stop is reported (results.engineStop) and the engine is started again (startIfStopped), so the scenarios after it start on a running engine */
export async function opColumn(guest, id, { page, log = (_m) => {}, weights = OP_WEIGHTS, n = OPS, why = '' } = {}) {
  const seed = seedOf(id);
  const mem0 = await memoryBytes(guest);
  const r = await runOps(guest, planOps(seed, n, weights));
  let engineStop = null, recovery = null;
  if (r.stops > 0) {
    engineStop = await stopReport(guest);
    recovery = await startIfStopped(guest, page, log);
  }
  const idle = await waitUntil(async () => (await stateName(guest)) === 'engine-idle', 60000, 250);
  const mem1 = r.stops === 0 ? await memoryBytes(guest) : null;
  const engineState = await stateName(guest);
  const ok = r.stops === 0 && r.reads.fail === 0 && r.dumpRetries <= DUMP_RETRY_LIMIT && idle;
  const stop = r.stoppedAt ? `; the engine stopped after operation ${r.stoppedAt.index + 1} (${r.stoppedAt.what}); started again: ${recovery ? recovery.state : 'not needed'}` : '';
  const notes = [`seed 0x${seed.toString(16)} (FNV-1a of ${id}): ${r.done} operations, stops ${r.stops}, table reads ok ${r.reads.ok} failed ${r.reads.fail}, dumpRetries ${r.dumpRetries} (limit ${DUMP_RETRY_LIMIT}), idle after ${idle}${stop}${why ? `; ${why}` : ''}`];
  return {
    ok, engineState, notes,
    results: { seed, ops: n, done: r.done, counts: r.counts, stops: r.stops, stoppedAt: r.stoppedAt, engineStop, recovery, readsOk: r.reads.ok, readsFailed: r.reads.fail, dumpRetries: r.dumpRetries, finalState: r.finalState, idleAfter: idle, memBytes: { start: mem0, end: mem1 } },
  };
}

/** the memory's growth: the last value over the first, both from the engine's report (null when either is unknown) */
export function memoryRatio(samples) {
  const known = samples.filter((s) => s.bytes !== null && s.bytes !== undefined);
  if (known.length < 2) return null;
  return Math.round((known[known.length - 1].bytes / known[0].bytes) * 1000) / 1000;
}

/** the page's tab hidden: a second tab of the same browser context is brought to the front, the page is then not visible */
export async function hideTab(page) {
  const other = await page.context().newPage();
  await other.bringToFront();
  const hidden = await waitUntil(async () => (await page.evaluate(() => document.visibilityState)) === 'hidden', 5000, 200);
  return { other, hidden };
}

/** the page shown again: the page is brought to the front and the second tab is closed */
export async function showTab(page, other) {
  await page.bringToFront();
  const visible = await waitUntil(async () => (await page.evaluate(() => document.visibilityState)) === 'visible', 5000, 200);
  await other.close().catch(() => {});
  return visible;
}

/** the desktop's picture hash now (the picture as the engine draws it) */
export const deskHash = (guest) => guest.ev('window.__app.deskDumpHash()').then((d) => d.sha256);

/** the picture's size is the guest's size (the page reads it at the same time; the scenario's own check) */
export async function pictureMatchesGuest(guest) {
  const d = await guest.desk();
  const p = await picture(guest);
  return p.w === d.gw && p.h === d.gh;
}
