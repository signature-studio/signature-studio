// (d) page-chrome (Issue #1481): the page is the guest's screen and nothing else. At the first idle: (i) the canvas fills the window (its width and height differ from the window's by at most 1 CSS px);
// (ii) no other element of the body is shown: an element counts when it is displayed with a box, except the canvas and the containers that hold it (they count only when they paint: a background, a border, an outline or text),
// and except the Browser face (WebView2: the iframe data-fl-webview2 and its box), which is the content of an FL window (#1376 / #1419 keep it as it is; it is reported apart);
// (iii) the text of the page has no data: picture (a cursor's picture is never written out as text: Issue #1481).
import { waitStable } from '../../lib/flow.mjs';

const CHECK = `(() => {
  const shown = (el) => {
    const cs = getComputedStyle(el);
    if (cs.display === 'none' || cs.visibility === 'hidden' || Number(cs.opacity) === 0) return false;
    const r = el.getBoundingClientRect();
    return r.width > 0 && r.height > 0;
  };
  const paints = (el) => {
    const cs = getComputedStyle(el);
    const bg = cs.backgroundColor !== 'rgba(0, 0, 0, 0)' && cs.backgroundColor !== 'transparent';
    const border = ['Top', 'Right', 'Bottom', 'Left'].some((s) => parseFloat(cs['border' + s + 'Width']) > 0 && cs['border' + s + 'Style'] !== 'none');
    const outline = parseFloat(cs.outlineWidth) > 0 && cs.outlineStyle !== 'none';
    const text = [...el.childNodes].some((n) => n.nodeType === 3 && n.textContent.trim() !== '');
    return bg || border || outline || text;
  };
  const canvas = document.querySelector('#desktop canvas[data-desk="1"]');
  const holders = new Set();
  for (let e = canvas ? canvas.parentElement : null; e && e !== document.body; e = e.parentElement) holders.add(e);
  // the Browser face (WebView2, #1376 / #1419) is the content of an FL window: its iframe and the box that holds it are counted apart
  const faceParts = new Set();
  for (const f of document.querySelectorAll('iframe[data-fl-webview2]')) { faceParts.add(f); if (f.parentElement) faceParts.add(f.parentElement); }
  const others = [];
  const faces = [];
  for (const el of document.body.querySelectorAll('*')) {
    if (el === canvas || el.tagName === 'SCRIPT' || el.tagName === 'STYLE') continue;
    if (!shown(el)) continue;
    if (faceParts.has(el)) { faces.push(el.tagName.toLowerCase()); continue; }
    if (holders.has(el) && !paints(el)) continue;
    others.push(el.id ? '#' + el.id : el.tagName.toLowerCase());
  }
  const r = canvas ? canvas.getBoundingClientRect() : null;
  return {
    canvas: r ? { w: r.width, h: r.height, left: r.left, top: r.top } : null,
    window: { w: window.innerWidth, h: window.innerHeight },
    others,
    faces,
    dataImage: document.body.textContent.includes('data:image'),
  };
})()`;

export default {
  name: 'page-chrome',
  expectFail: null,
  async run({ guest }) {
    const first = await waitStable(guest, 180000);
    const notes = [`first state ${first.state} after ${first.waitedMs} ms`];
    if (first.state !== 'engine-idle') return { ok: false, engineState: first.state, notes };
    const c = await guest.ev(CHECK);
    const fills = c.canvas !== null && Math.abs(c.canvas.w - c.window.w) <= 1 && Math.abs(c.canvas.h - c.window.h) <= 1 && Math.abs(c.canvas.left) <= 1 && Math.abs(c.canvas.top) <= 1;
    const alone = c.others.length === 0;
    const noData = !c.dataImage;
    notes.push(c.canvas
      ? `canvas ${c.canvas.w.toFixed(1)} x ${c.canvas.h.toFixed(1)} CSS px in a window of ${c.window.w} x ${c.window.h}: ${fills ? 'it fills the window' : 'it does not fill the window'}`
      : 'no canvas of the desktop in the page');
    notes.push(`shown elements besides the canvas: ${c.others.length}${alone ? '' : ` (${c.others.slice(0, 12).join(', ')})`}; the Browser face of an FL window (WebView2, counted apart): ${c.faces.length}`);
    notes.push(`data:image text in the page: ${noData ? 'no' : 'yes'}`);
    return { ok: fills && alone && noData, engineState: first.state, notes, check: { fills, others: c.others, dataImage: c.dataImage } };
  },
};
