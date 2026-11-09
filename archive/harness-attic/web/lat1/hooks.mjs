// lat1 (decision 0194): measurement hooks that are injected FROM OUTSIDE the app (CDP). Nothing in app/ is changed.
//   PAGE_HOOK   Page.addScriptToEvaluateOnNewDocument: records the capture-phase DOM input events (event.timeStamp), the arrival of each Worker message
//               of type 'frame' (before the app's own handler), every putImageData onto a window canvas (end time, rectangle, whether it changed any pixel
//               of what the canvas already held: a per-window shadow of the pixels) and the next requestAnimationFrame callback after a CHANGING draw.
//   WORKER_HOOK Fetch.fulfillRequest: the text below is put IN FRONT OF the served worker-*.js (the served file on disk is not touched). It records when a
//               'input' message reaches the Worker (a 'message' listener registered before the app's onmessage) and when self.postMessage is called for a
//               frame / input-done. A message {cmd:'lat1-dump'} makes it post the records back ({t:'lat1-dump', ...}); the app ignores unknown message types.
// All times are epoch milliseconds: performance.timeOrigin + performance.now() (the same scale in the page and in the Worker).
export const WORKER_HOOK = `(() => {
  const O = performance.timeOrigin; const now = () => O + performance.now();
  const L = { recv: [], posts: [] };
  const origPost = self.postMessage.bind(self);
  self.addEventListener('message', (e) => {
    const m = e.data;
    if (!m || typeof m !== 'object') return;
    if (m.cmd === 'input') L.recv.push({ seq: m.seq, t: now(), tPage: m.tPage });
    else if (m.cmd === 'lat1-dump') { origPost({ t: 'lat1-dump', recv: L.recv, posts: L.posts }); L.recv = []; L.posts = []; }
  });
  self.postMessage = function (m, tr) {
    const m0 = (m && typeof m === 'object') ? m : null;
    const a = now();
    origPost(m, tr);
    const b = now();
    if (m0 && (m0.t === 'frame' || m0.t === 'input-done')) {
      if (m0.t === 'frame') L.posts.push({ t: 'frame', a, b, lid: m0.lid, rect: m0.rect, postedAt: m0.postedAt, layer: m0.layerAlpha !== undefined });
      else L.posts.push({ t: 'input-done', a, b, seqs: m0.items.map((x) => x.seq) });
    }
  };
})();
`;

export const PAGE_HOOK = `(() => {
  const O = performance.timeOrigin; const now = () => O + performance.now();
  const L = window.__lat1 = { dom: [], msgs: [], puts: [], workerDumps: [], hashMs: 0, nputs: 0 };
  // DOM events in the capture phase (before the app's listeners): the event's own timeStamp (when the browser made it) and the instant this listener ran
  for (const type of ['pointerdown', 'pointerup', 'pointermove', 'mousedown', 'mouseup', 'mousemove', 'wheel', 'keydown', 'keyup', 'contextmenu']) {
    window.addEventListener(type, (e) => { L.dom.push({ type, ts: O + e.timeStamp, t: now(), x: e.clientX, y: e.clientY, key: e.key }); }, true);
  }
  const RealWorker = window.Worker;
  let curMsg = null;
  window.Worker = class extends RealWorker {
    constructor(...a) {
      super(...a);
      this.addEventListener('message', (ev) => {
        const m = ev.data;
        if (m && m.t === 'frame') { curMsg = { tRecv: now(), postedAt: m.postedAt, lid: m.lid, rect: m.rect }; L.msgs.push(curMsg); }
        else if (m && m.t === 'lat1-dump') L.workerDumps.push(m);
      });
      window.__lat1.worker = this;
    }
  };
  const P = CanvasRenderingContext2D.prototype.putImageData;
  const shadows = new Map();
  CanvasRenderingContext2D.prototype.putImageData = function (img, dx, dy, ...rest) {
    const c = this.canvas;
    const lid = c && c.dataset ? c.dataset.lid : undefined;
    const r = P.call(this, img, dx, dy, ...rest);
    const tEnd = now();
    if (lid === undefined) return r;
    const t1 = performance.now();
    const cw = c.width, ch = c.height;
    let sh = shadows.get(c);
    if (!sh || sh.w !== cw || sh.h !== ch) { sh = { w: cw, h: ch, px: new Uint32Array(cw * ch) }; shadows.set(c, sh); }
    const u = new Uint32Array(img.data.buffer, img.data.byteOffset, img.data.byteLength >> 2);
    let changed = 0;
    for (let y = 0; y < img.height; y++) {
      const yy = dy + y; if (yy < 0 || yy >= ch) continue;
      let o = yy * cw + dx, s = y * img.width;
      for (let x = 0; x < img.width; x++, o++, s++) { if (dx + x < 0 || dx + x >= cw) continue; const v = u[s]; if (sh.px[o] !== v) { changed++; sh.px[o] = v; } }
    }
    L.hashMs += performance.now() - t1; L.nputs++;
    const rec = { tEnd, lid: +lid, x: dx, y: dy, w: img.width, h: img.height, changed, msg: curMsg, raf: null, rafDone: null };
    curMsg = null;
    L.puts.push(rec);
    if (changed) requestAnimationFrame((ts) => { rec.raf = now(); rec.rafTs = O + ts; const mc = new MessageChannel(); mc.port1.onmessage = () => { rec.rafDone = now(); mc.port1.close(); }; mc.port2.postMessage(0); });
    return r;
  };
})();
`;

export async function installWorkerHook(page, browser) {
  // Fetch.enable on the page session with a Response-stage pattern for the worker script; the body is read, the hook put in front, the response fulfilled.
  await page.S('Fetch.enable', { patterns: [{ urlPattern: '*/static/worker-*.js', requestStage: 'Response' }] });
  const state = { patched: 0 };
  browser.cdp.on(async (m) => {
    if (m.method !== 'Fetch.requestPaused' || !m.sessionId) return;
    const p = m.params;
    try {
      if (!/\/static\/worker-[^/]*\.js(\?|$)/.test(p.request.url) || p.responseStatusCode !== 200) { await browser.cdp.send('Fetch.continueRequest', { requestId: p.requestId }, m.sessionId); return; }
      const body = await browser.cdp.send('Fetch.getResponseBody', { requestId: p.requestId }, m.sessionId);
      const text = body.base64Encoded ? Buffer.from(body.body, 'base64').toString('utf8') : body.body;
      const headers = p.responseHeaders.filter((h) => !/^content-length$|^content-encoding$/i.test(h.name));
      await browser.cdp.send('Fetch.fulfillRequest', { requestId: p.requestId, responseCode: 200, responseHeaders: headers, body: Buffer.from(WORKER_HOOK + text, 'utf8').toString('base64') }, m.sessionId);
      state.patched++;
    } catch (e) { state.error = String(e); try { await browser.cdp.send('Fetch.continueRequest', { requestId: p.requestId }, m.sessionId); } catch { /* ignore */ } }
  });
  return state;
}
