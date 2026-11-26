// (g) context-menu-none (Issue #1482): the browser's own context menu never shows on the page. In each region, 20 right clicks (CDP mouse events, short presses) or 20 contextmenu
// events dispatched on the element are made, and the page's own listener cancels every contextmenu (app/src/input/context-menu.ts). In the measured runs the browser sent one contextmenu
// for each right press that reached the page, even where the engine's pointer handling had cancelled the press; some of the 20 CDP presses of a real region never reached the
// page (16 to 20 did), and those are reported as presses, not as contextmenu events.
// The measure is a capture listener of the document that is added after the page's (the page's is added when its module runs, before the load event): it reads defaultPrevented after the page has cancelled the event.
// Regions: before-idle (random points of the viewport, right after the load: the engine's input is still off); canvas and desktop (random points in their boxes, when shown);
// toolbar and sidebar (aside), which the page hides (#1481): their contextmenu is dispatched on the element. Each region records the target kinds (tags, editable targets,
// targets in a frame): a 'Paste' alone is what the browser offers in an editable place, so an editable target is recorded, not hidden.
import { sleep, waitStable } from '../../lib/flow.mjs';

const N = 20;
const MEASURE = `(() => {
  const ctx = { seen: 0, prevented: 0, editable: 0, inFrame: 0, tags: {}, focus: {}, presses: 0, pressesCancelled: 0 };
  window.__e2eCtx = ctx;
  const name = (el) => (el ? el.tagName.toLowerCase() + (el.id ? '#' + el.id : '') : 'none');
  // the right presses that reach the page; a press the page cancels (its pointerdown: the engine's input) gets no contextmenu from the browser
  window.addEventListener('pointerdown', (e) => { if (e.button === 2) { ctx.presses++; if (e.defaultPrevented) ctx.pressesCancelled++; } });
  document.addEventListener('contextmenu', (e) => {
    ctx.seen++;
    if (e.defaultPrevented) ctx.prevented++;
    const t = e.target;
    const el = t instanceof Element ? t : null;
    if (el && (el.isContentEditable || el.tagName === 'INPUT' || el.tagName === 'TEXTAREA')) ctx.editable++;
    if (t && t.ownerDocument !== document) ctx.inFrame++;
    const k = el ? name(el) : String(t);
    ctx.tags[k] = (ctx.tags[k] ?? 0) + 1;
    const f = name(document.activeElement);
    ctx.focus[f] = (ctx.focus[f] ?? 0) + 1;
  }, true);
  return true;
})()`;
const READ = `(() => { const c = window.__e2eCtx; return c ? { seen: c.seen, prevented: c.prevented, editable: c.editable, inFrame: c.inFrame, tags: { ...c.tags }, focus: { ...c.focus }, presses: c.presses, pressesCancelled: c.pressesCancelled } : null; })()`;
const BOX = (sel) => `(() => { const el = document.querySelector(${JSON.stringify(sel)}); if (!el) return null; const cs = getComputedStyle(el); const r = el.getBoundingClientRect(); const shown = !el.closest('[hidden]') && cs.display !== 'none' && cs.visibility !== 'hidden' && r.width > 0 && r.height > 0; return { shown, x: r.left, y: r.top, w: r.width, h: r.height }; })()`;
const VIEW = `({ shown: true, x: 0, y: 0, w: window.innerWidth, h: window.innerHeight })`;
const SYNTH = (sel, n) => `(() => { const el = document.querySelector(${JSON.stringify(sel)}); if (!el) return 0; const r = el.getBoundingClientRect(); for (let i = 0; i < ${n}; i++) el.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, button: 2, clientX: r.left, clientY: r.top })); return ${n}; })()`;

const rnd = (a, b) => a + Math.random() * (b - a);

async function rightClickAt(guest, x, y) {
  await guest.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x, y, button: 'none', buttons: 0, pointerType: 'mouse' });
  await guest.send('Input.dispatchMouseEvent', { type: 'mousePressed', x, y, button: 'right', buttons: 2, clickCount: 1, pointerType: 'mouse' });
  await guest.send('Input.dispatchMouseEvent', { type: 'mouseReleased', x, y, button: 'right', buttons: 0, clickCount: 1, pointerType: 'mouse' });
  await sleep(120);
}

/** one region: the events it made, how many the page cancelled, and what the targets were */
async function region(guest, page, name, box, sel) {
  const before = await page.evaluate(READ);
  let kind;
  if (box && box.shown) {
    kind = 'real';
    for (let i = 0; i < N; i++) await rightClickAt(guest, box.x + rnd(0.05, 0.95) * box.w, box.y + rnd(0.05, 0.95) * box.h);
  } else {
    kind = 'synthetic';
    await page.evaluate(SYNTH(sel, N));
  }
  const after = await page.evaluate(READ);
  const tags = diff(before.tags, after.tags), focus = diff(before.focus, after.focus);
  return {
    region: name, kind, events: after.seen - before.seen, prevented: after.prevented - before.prevented,
    presses: after.presses - before.presses, pressesCancelled: after.pressesCancelled - before.pressesCancelled,
    editable: after.editable - before.editable, inFrame: after.inFrame - before.inFrame, tags, focus,
  };
}

/** the counts that grew between two reads of the page's counts */
function diff(before, after) {
  const out = {};
  for (const [k, v] of Object.entries(after)) { const d = v - (before[k] ?? 0); if (d > 0) out[k] = d; }
  return out;
}

/** the judge: in every region the browser's contextmenu events are all cancelled by the page (so no browser menu shows) and there are some (a synthetic region: all N); none is in a frame. The presses are not judged (notes only) */
export function judge(regions) {
  return regions.every((r) => r.events > 0 && r.prevented === r.events && r.inFrame === 0 && (r.kind === 'real' || r.events === N));
}

export default {
  name: 'context-menu-none',
  expectFail: null,
  async run({ guest, page, log }) {
    const notes = [];
    await page.evaluate(MEASURE);
    const atStart = await guest.state().catch(() => null);
    const regions = [];
    regions.push(await region(guest, page, 'before-idle', await page.evaluate(VIEW), null));
    notes.push(`the first region is made at engine state ${atStart?.state ?? 'unreadable'} (the input is off until the first idle)`);
    const first = await waitStable(guest, 180000);
    if (first.state !== 'engine-idle') return { ok: false, engineState: first.state, notes: [...notes, `first state ${first.state}`], results: regions };
    const canvas = await page.evaluate(BOX('#desktop canvas[data-desk="1"]'));
    const desk = await page.evaluate(BOX('#desktop'));
    regions.push(await region(guest, page, 'canvas', canvas, '#desktop canvas[data-desk="1"]'));
    regions.push(await region(guest, page, 'desktop', desk, '#desktop'));
    regions.push(await region(guest, page, 'toolbar', await page.evaluate(BOX('#toolbar')), '#toolbar'));
    regions.push(await region(guest, page, 'sidebar', await page.evaluate(BOX('aside')), 'aside'));
    for (const r of regions) log(`${r.region} (${r.kind}): ${r.prevented}/${r.events} cancelled, editable ${r.editable}, in a frame ${r.inFrame}`);
    // the judge (judge()): every contextmenu the browser sends is cancelled (so no browser menu shows), the events are there (a synthetic region: all N), none is in a frame.
    // The presses are reported in the notes only: a real region's presses (the window's pointerdown count) can read 0 while its contextmenu events are all cancelled.
    const ok = judge(regions);
    notes.push(...regions.map((r) => `${r.region} (${r.kind}): ${N} right clicks, ${r.presses} presses reached the page (${r.pressesCancelled} cancelled by its pointer handling), ${r.events} contextmenu events, ${r.prevented} cancelled; editable targets ${r.editable}, in a frame ${r.inFrame}, targets ${JSON.stringify(r.tags)}, focused ${JSON.stringify(r.focus)}`));
    return { ok, engineState: first.state, notes, results: regions };
  },
};
