// page/startup-sequence (Issue #1520): the order of the start, watched from the page's own load. (a) The first picture is black (most of it, outside the modal) and an HTML modal
// in the middle of the window holds the start-up's progress bars (role="progressbar"). (b) The four start-up assets reach 100 %, and some of them are between. (c) The modal
// leaves the DOM before the engine starts and before its first picture. (d) The engine's window table holds the native splash's window before the first idle: the engine log
// has its window-table dump before the log length the page had when the engine reported its first idle. (e) The splash is drawn by the engine's drawing call on that window,
// before the first idle too, and the page holds no image of it (no img, no background-image, no data: image in the DOM). (f) The canvas shows the splash before the first idle:
// a shot taken while the engine has not gone idle has non-black points inside the splash window's rectangle (the first top-level window the drawing call wrote into).
import fs from 'node:fs';
import path from 'node:path';
import { defineScenario } from '../../lib/scenario.mjs';
import { sleep } from '../../lib/flow.mjs';

const START_LIMIT_MS = 180000;
const SAMPLE_MS = 200;
const BLACK_SHARE = 0.5;
const CENTRE_SHARE = 0.1;
const ASSETS = ['engine wasm', 'seed pack', 'DLL pack', 'engine glue'];
const SPLASH_DUMP = /first visible top-level window appeared/;
const SPLASH_WINDOW = /lid (\d+) class "([^"]+)" .*visible 1 /;
const DRAW_CALL = /written by (\w+) into the surface of top-level window lid (\d+)/;
const DRAW_APIS = /^(UpdateLayeredWindow|UpdateLayeredWindowIndirect|BitBlt|StretchBlt|StretchDIBits|SetDIBitsToDevice|TransparentBlt|AlphaBlend|PatBlt)$/;
const SPLASH_RECT = /lid (\d+) class "[^"]*" title "[^"]*" rect \((-?\d+), (-?\d+), (\d+) x (\d+)\)/;
// (f): the splash's share of non-black points on the canvas, inside its rectangle, before the first idle (a provisional limit: the splash is a picture, not a few pixels)
const NON_BLACK_SHARE = 0.1;
// a point is black when its three colour bytes are all at most this level: the desk's own colour is (5, 7, 10), which is black to the eye (the splash is dark too, its top colours are about (21, 39, 27))
const BLACK_LEVEL = 16;

// the page's view now: the modal (its box and bars), the engine's state and marks, and the desktop canvas: its box in CSS px and the guest screen in device px (the canvas's text)
const VIEW = `(() => {
  const m = document.getElementById('loading-modal');
  const r = m ? m.getBoundingClientRect() : null;
  const styled = m ? getComputedStyle(m).position === 'fixed' : false;
  const bars = m ? [...m.querySelectorAll('[role="progressbar"]')].map((b) => ({ label: b.closest('[data-label]')?.dataset.label ?? null, value: b.value, max: b.max })) : [];
  const a = window.__app ?? null;
  const c = document.querySelector('#desktop canvas[data-desk]');
  const cr = c ? c.getBoundingClientRect() : null;
  const screen = /guest screen (\\d+)x(\\d+)/.exec(document.getElementById('canvas-info')?.textContent ?? '');
  return { now: performance.now(), dpr: window.devicePixelRatio, inner: [window.innerWidth, window.innerHeight], modal: r ? { left: r.left, top: r.top, width: r.width, height: r.height } : null, styled, bars, state: a ? a.state : null, timeline: a ? a.timeline : {},
    canvas: cr ? { left: cr.left, top: cr.top, width: cr.width, height: cr.height } : null, screen: screen ? [Number(screen[1]), Number(screen[2])] : null };
})()`;

// read by the page at every change of the modal's bars or the engine's state (once a frame, until the scenario stops it): the moments are the page's own, not the sampler's. logLen is the engine log's length then.
// It is put into the page by evaluate right after the commit (not an init script: an init script also runs in the sandboxed frames of the Browser face, which log a blocked-script error each).
const RECORDER = `(() => {
  if (window.__startRec) return true;
  const rec = (window.__startRec = []);
  let last = '';
  const tick = () => {
    const m = document.getElementById('loading-modal');
    const bars = m ? [...m.querySelectorAll('[role="progressbar"]')].map((b) => [b.closest('[data-label]')?.dataset.label ?? '', b.value, b.max]) : null;
    const state = window.__app ? window.__app.state : null;
    const key = JSON.stringify([bars, state]);
    if (key !== last) { last = key; rec.push({ t: performance.now(), modal: bars !== null, bars, state, logLen: window.__app ? window.__app.engineLog().length : 0 }); }
  };
  const loop = () => { if (window.__startRecStop) return; tick(); requestAnimationFrame(loop); };
  loop();
  return true;
})()`;

// what the page draws with images: <img>, a CSS background-image, and a data: image in an attribute (the desktop's cursor is a CSS cursor, not an image of the splash: it is left out)
const IMAGES = `(() => {
  const bg = [...document.querySelectorAll('*')].filter((e) => { const s = getComputedStyle(e).backgroundImage; return !!s && s !== 'none'; }).length;
  const data = [...document.querySelectorAll('[src],[style]')].filter((e) => e.id !== 'desktop' && (/^data:image/.test(e.getAttribute('src') ?? '') || /data:image/.test(e.getAttribute('style') ?? ''))).length;
  return { img: document.images.length, backgroundImage: bg, dataImage: data };
})()`;

const evalOrNull = (page, expr) => page.evaluate(expr).catch(() => null);

/** a screenshot of the page at half scale (PNG, base64) */
const capture = async (cdp, v) => (await cdp.send('Page.captureScreenshot', { format: 'png', clip: { x: 0, y: 0, width: v.inner[0], height: v.inner[1], scale: 0.5 } })).data;

/** a picture of the page (a base64 PNG, read in the page). `region` (CSS px): with `rect`, the points inside that rectangle; without it, the points outside the modal's box.
 * Each point counts black (R, G, B all 0) or not. The first picture is wanted early: the engine's own picture starts about 1 s later. */
async function sampleShot(page, b64, v, region) {
  const level = region.level ?? 0;
  const box = region.rect ? null : v.modal ? { l: v.modal.left, t: v.modal.top, r: v.modal.left + v.modal.width, b: v.modal.top + v.modal.height } : null;
  const r = await page.evaluate(async ({ b64, W, H, box, rect, level }) => {
    const bytes = Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
    const bmp = await createImageBitmap(new Blob([bytes], { type: 'image/png' }));
    const cv = new OffscreenCanvas(bmp.width, bmp.height);
    const ctx = cv.getContext('2d');
    ctx.drawImage(bmp, 0, 0);
    const d = ctx.getImageData(0, 0, bmp.width, bmp.height).data;
    const kx = bmp.width / W, ky = bmp.height / H;
    const step = rect ? Math.max(1, Math.round((rect.w * kx) / 40)) : Math.max(1, Math.round(bmp.width / 96));
    let points = 0, black = 0;
    for (let y = 1; y < bmp.height; y += step) for (let x = 1; x < bmp.width; x += step) {
      const cx = x / kx, cy = y / ky;
      if (rect && !(cx >= rect.l && cx < rect.r && cy >= rect.t && cy < rect.b)) continue;
      if (box && cx >= box.l && cx < box.r && cy >= box.t && cy < box.b) continue;
      points++;
      const i = (y * bmp.width + x) * 4;
      if (d[i] <= level && d[i + 1] <= level && d[i + 2] <= level) black++;
    }
    return { points, black, size: [bmp.width, bmp.height] };
  }, { b64, W: v.inner[0], H: v.inner[1], box, rect: region.rect ?? null, level });
  return { points: r.points, black: r.black, nonBlackShare: r.points > 0 ? (r.points - r.black) / r.points : null, share: r.points > 0 ? r.black / r.points : null, size: r.size };
}

/** the share of black pixels outside the modal (the first picture) */
const blackShare = async (cdp, page, v) => sampleShot(page, await capture(cdp, v), v, {});

/** the splash window's rectangle in the engine's screen (device px): the table line of the window that the first drawing call wrote into, after that call's line. Null when there is none. */
function splashRect(lines) {
  for (let k = 0; k < lines.length; k++) {
    const m = DRAW_CALL.exec(lines[k]);
    if (!m || !DRAW_APIS.test(m[1])) continue;
    const lid = Number(m[2]);
    for (let i = k + 1; i < Math.min(lines.length, k + 200); i++) {
      const t = SPLASH_RECT.exec(lines[i]);
      if (t && Number(t[1]) === lid) return { index: k, lid, x: Number(t[2]), y: Number(t[3]), w: Number(t[4]), h: Number(t[5]) };
    }
  }
  return null;
}

/** the splash rectangle in CSS px on the page: the canvas's box scaled from the guest screen (device px) */
function cssRect(v, sr) {
  if (!v.canvas || !v.screen || !sr) return null;
  const k = v.canvas.width / v.screen[0];
  return { l: v.canvas.left + sr.x * k, t: v.canvas.top + sr.y * k, r: v.canvas.left + (sr.x + sr.w) * k, b: v.canvas.top + (sr.y + sr.h) * k, w: sr.w * k };
}

/** the splash window's dump in the engine log: the index of its line and the first visible window of the table that follows it (lid, class), or null */
function splashDump(lines) {
  const at = lines.findIndex((l) => SPLASH_DUMP.test(l));
  if (at < 0) return null;
  for (let i = at + 1; i < Math.min(lines.length, at + 12); i++) {
    const m = SPLASH_WINDOW.exec(lines[i]);
    if (m) return { index: at, lid: Number(m[1]), className: m[2] };
  }
  return { index: at, lid: null, className: null };
}

export default defineScenario({
  id: 'page/startup-sequence',
  covers: ['page/start/reaches-idle'],
  displays: ['4k-dpr1', '4k-dpr2', '4k-random'],
  async run({ guest, page, cdp, origin, log }) {
    const notes = [];
    const shots = guest.shots ? path.join(guest.shots, 'startup') : null;
    if (shots) fs.mkdirSync(shots, { recursive: true });
    await page.goto(`${origin}/`, { waitUntil: 'commit', timeout: 120000 });
    for (let i = 0; i < 100; i++) { if (await page.evaluate(RECORDER).catch(() => false)) break; await sleep(10); }

    let firstPaint = null, finalView = null, preIdle = null, preShots = 0;
    const t0 = Date.now();
    while (Date.now() - t0 < START_LIMIT_MS) {
      const v = await evalOrNull(page, VIEW);
      if (v) {
        finalView = v;
        // the first picture: the page paints nothing before its stylesheet is in (the modal is styled from then on), so the sample waits for that
        if (v.modal && v.styled && firstPaint === null) firstPaint = { view: v, black: await blackShare(cdp, page, v) };
        // (f): the canvas as it is while the engine has not gone idle. The engine's log reaches the page only at an idle (its batches are flushed by the Worker's timers), so the splash's
        // rectangle is read after the run; the last picture that was taken with no idle before it and none after it is kept
        if (v.canvas && typeof v.timeline.idle !== 'number') {
          const b64 = await capture(cdp, v);
          const after = await evalOrNull(page, VIEW);
          if (after && typeof after.timeline.idle !== 'number') { preIdle = { b64, view: v, timeline: after.timeline }; preShots++; }
        }
        if (v.state === 'engine-idle' || v.state === 'error' || v.state === 'exited') break;
      }
      await sleep(SAMPLE_MS);
    }
    await evalOrNull(page, 'window.__startRecStop = true');
    const rec = (await evalOrNull(page, 'window.__startRec ?? []')) ?? [];
    // the first picture is marked when the desktop first changes on the canvas, which can come just after the first idle: the marks are read again for a few seconds
    let tl = finalView ? finalView.timeline : {};
    for (let i = 0; i < 25 && finalView && finalView.state === 'engine-idle' && typeof tl.first_pixel !== 'number'; i++) {
      await sleep(200);
      tl = (await evalOrNull(page, 'window.__app ? window.__app.timeline : {}')) ?? tl;
    }
    const engineState = finalView ? finalView.state : null;
    const idleEntry = rec.find((e) => e.state === 'engine-idle') ?? null;
    const logLenAtIdle = idleEntry ? idleEntry.logLen : null;
    const lines = (await evalOrNull(page, 'window.__app ? window.__app.engineLog() : []')) ?? [];
    const assets = (await evalOrNull(page, "(() => { const r = window.__app && window.__app.assetsReport(); return r ? r.items.map((i) => [i.label, i.origin, i.store, i.fail, Math.round(i.totalMs ?? 0)]) : null; })()")) ?? null;
    notes.push(`(b) assets as the loader saw them (label, origin, store, fail, ms): ${JSON.stringify(assets)}`);

    // (a) the first picture: black outside the modal, the modal with its four progress bars, centred
    let a = false;
    if (firstPaint && firstPaint.view.modal) {
      const v = firstPaint.view;
      const cx = v.modal.left + v.modal.width / 2, cy = v.modal.top + v.modal.height / 2;
      const dist = Math.hypot(cx - v.inner[0] / 2, cy - v.inner[1] / 2);
      const centred = dist <= CENTRE_SHARE * v.inner[0];
      const bars = v.bars.filter((b) => ASSETS.includes(b.label));
      const black = firstPaint.black.share !== null && firstPaint.black.share >= BLACK_SHARE;
      a = black && centred && bars.length === ASSETS.length;
      notes.push(`(a) first picture: black ${firstPaint.black.black} of ${firstPaint.black.points} points outside the modal (${firstPaint.black.share === null ? 'n/a' : (100 * firstPaint.black.share).toFixed(1)} %); modal ${Math.round(v.modal.width)}x${Math.round(v.modal.height)} at (${Math.round(v.modal.left)},${Math.round(v.modal.top)}), centre ${Math.round(dist)} px from the window centre (limit ${Math.round(CENTRE_SHARE * v.inner[0])} px); progress bars ${bars.length} of ${ASSETS.length}`);
    } else {
      notes.push('(a) no modal in the page: the first picture has no HTML modal with a progress bar');
    }

    // (b) the bars move up (a bar above 0 while the modal is there), and the four start-up assets reach 100 % while the modal is still there
    const fracs = new Map(ASSETS.map((l) => [l, 0]));
    let moved = false;
    for (const e of rec) {
      if (!e.modal) continue;
      for (const [label, value, max] of e.bars) {
        if (!fracs.has(label) || !(max > 0)) continue;
        const f = value / max;
        fracs.set(label, Math.max(fracs.get(label), f));
        if (f > 0) moved = true;
      }
    }
    const b = ASSETS.every((l) => fracs.get(l) >= 0.999) && moved;
    notes.push(`(b) progress bars reached 100 % while the modal was there: ${ASSETS.map((l) => `${l} ${(100 * fracs.get(l)).toFixed(0)} %`).join(', ')}; a bar moved above 0 %: ${moved}`);
    notes.push(`(b) recorder: ${rec.length} changes; first: ${JSON.stringify(rec.map((e) => [Math.round(e.t), e.modal ? e.bars.map((x) => `${x[0]}=${Number(x[1]).toFixed(3)}`).join("|") : "-", e.state]))}; download marks: ${JSON.stringify({ download_start: tl.download_start ?? null, download_done: tl.download_done ?? null, manifest_loaded: tl.manifest_loaded ?? null })}`);

    // (c) the modal leaves the DOM before the engine starts and before its first picture
    const removed = rec.find((e, i) => !e.modal && rec.slice(0, i).some((p) => p.modal)) ?? null;
    const tRemoved = removed ? removed.t : null;
    const eStart = typeof tl.engine_start === 'number' ? tl.engine_start : null;
    const fPixel = typeof tl.first_pixel === 'number' ? tl.first_pixel : null;
    const c = tRemoved !== null && eStart !== null && fPixel !== null && tRemoved < eStart && tRemoved < fPixel;
    notes.push(`(c) modal removed at ${tRemoved === null ? 'never' : `${tRemoved.toFixed(0)} ms`}; engine_start ${eStart === null ? 'none' : `${eStart.toFixed(0)} ms`}; first_pixel ${fPixel === null ? 'none' : `${fPixel.toFixed(0)} ms`}`);

    // (d) the splash window: in the engine's window table before the first idle (the engine log's order: the dump's line is before the log length at the first idle)
    const dump = splashDump(lines);
    const d = dump !== null && dump.lid !== null && logLenAtIdle !== null && dump.index < logLenAtIdle;
    notes.push(`(d) engine window table, first visible top-level window: lid ${dump && dump.lid !== null ? dump.lid : 'none'} class "${dump && dump.className ? dump.className : 'none'}" at engine log line ${dump ? dump.index : 'none'}; the first idle at log line ${logLenAtIdle === null ? 'none' : logLenAtIdle}`);

    // (e) the splash's picture: the engine's drawing call on that window, before the first idle; the page holds no image
    const draw = lines.map((l, i) => ({ l, i, m: DRAW_CALL.exec(l) })).find((x) => x.m && dump && dump.lid !== null && Number(x.m[2]) === dump.lid && DRAW_APIS.test(x.m[1]));
    const img = await evalOrNull(page, IMAGES);
    const pageImages = img ? img.img + img.backgroundImage + img.dataImage : null;
    const e = !!draw && logLenAtIdle !== null && draw.i < logLenAtIdle && pageImages === 0;
    notes.push(`(e) engine log line ${draw ? draw.i : 'none'}: ${draw ? draw.l.trim() : 'no drawing call on the window'}; page images: ${img ? JSON.stringify(img) : 'unknown'}`);
    if (draw) log(`(e) ${draw.l.trim()}`);

    // (f) the splash is on the canvas before the first idle: the pre-idle picture has non-black points inside the splash window's rectangle (its share of the points there)
    const splash = splashRect(lines);
    let early = null;
    if (preIdle && splash) {
      const rect = cssRect(preIdle.view, splash);
      const share = await sampleShot(page, preIdle.b64, preIdle.view, { rect, level: BLACK_LEVEL });
      const firstPixelBeforeIdle = typeof preIdle.timeline.first_pixel === 'number';
      early = { splash, rect, share, shots: preShots, firstPixelBeforeIdle, pass: share.nonBlackShare !== null && share.nonBlackShare >= NON_BLACK_SHARE && firstPixelBeforeIdle, timeline: preIdle.timeline };
    }
    if (preIdle && shots) fs.writeFileSync(path.join(shots, 'pre-idle.png'), Buffer.from(preIdle.b64, 'base64'));
    const f = !!early && early.pass;
    if (early) notes.push(`(f) the splash rectangle (CSS px ${Math.round(early.rect.l)},${Math.round(early.rect.t)} to ${Math.round(early.rect.r)},${Math.round(early.rect.b)}; guest px ${early.splash.x},${early.splash.y} ${early.splash.w}x${early.splash.h} of lid ${early.splash.lid}): non-black (a colour above ${BLACK_LEVEL} in one byte) ${early.share.points - early.share.black} of ${early.share.points} points (${early.share.nonBlackShare === null ? 'n/a' : (100 * early.share.nonBlackShare).toFixed(1)} %, limit ${100 * NON_BLACK_SHARE} %); pictures taken before the idle ${preShots}; first_pixel marked before the idle: ${early.firstPixelBeforeIdle ? 'yes' : 'no'} (timeline first_pixel ${typeof early.timeline.first_pixel === 'number' ? early.timeline.first_pixel.toFixed(0) : 'none'} ms)`);
    else notes.push(`(f) no splash rectangle or no picture before the first idle (drawing call in the log: ${lines.some((l) => DRAW_CALL.test(l))}; pictures before the idle: ${preShots})`);

    const ok = engineState === 'engine-idle' && a && b && c && d && e && f;
    return {
      ok,
      engineState,
      notes,
      results: {
        firstPaint: firstPaint ? { black: firstPaint.black, modal: firstPaint.view.modal, bars: firstPaint.view.bars } : null,
        fractions: Object.fromEntries(fracs), removedAt: tRemoved, timeline: { engine_start: eStart, first_pixel: fPixel, idle: typeof tl.idle === 'number' ? tl.idle : null },
        splash: dump, idleLogLength: logLenAtIdle, drawLine: draw ? draw.l.trim() : null, pageImages: img, early, preIdleShots: preShots,
        checks: { a, b, c, d, e, f },
      },
    };
  },
});
