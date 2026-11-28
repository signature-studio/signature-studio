// R02 (#1408) browser-sounds-stacking. Report (L-056): the Browser's Sounds tab (a page face, an iframe) must not be shown in front of FL's windows; when the File
// menu is open, the menu is in front and its lower part is not hidden. Expectation: after File is opened, the menu lies on the screen, the menu overlaps a face
// (the page's iframe) on the page, and no sample point of the menu is answered by that iframe (elementFromPoint); the overlap is not empty (the test is not void).
import { openMenuLabel, startDefault, table, newTops, sleep, result, geometry } from './common.mjs';

const SCENARIO = {
  name: 'browser-sounds-stacking',
  report: 'R02',
  expect: 'File menu open: the menu is on screen; it overlaps the Sounds face (iframe) and elementFromPoint at its sample points is never the iframe; 0 console errors',
  expectFail: null,
};
const FILE_LABEL = { fx: 0.057, fy: 0.45 };

export default {
  ...SCENARIO,
  async run({ guest, log }) {
    const start = await startDefault(guest, log);
    if (!start.ok) return result(false, start.state, start.notes);
    const before = (await table(guest)).map((w) => w.lid);
    const pop = await openMenuLabel(guest, FILE_LABEL.fx, FILE_LABEL.fy);
    if (!pop) return result(false, (await guest.state())?.state, [...start.notes, 'the File menu did not open']);
    await sleep(800);
    const t = await table(guest);
    const menu = newTops(t, before).find((w) => w.cls === pop.cls && w.lid === pop.lid) ?? pop;
    const g = await geometry(guest);
    const inScreen = menu.ay >= 0 && menu.ay + menu.h <= g.gh && menu.ax >= 0 && menu.ax + menu.w <= g.gw;
    const r = await guest.ev(`(() => {
      const d = document.getElementById('desktop').getBoundingClientRect();
      const s = window.__app.screen.guest;
      const kx = d.width / s.width, ky = d.height / s.height;
      const m = ${JSON.stringify({ x: menu.ax, y: menu.ay, w: menu.w, h: menu.h })};
      const faces = [...document.querySelectorAll('iframe')];
      const fr = faces.map((f) => f.getBoundingClientRect());
      let samples = 0, overFace = 0, answeredByFace = 0;
      for (let gy = m.y + 3; gy < m.y + m.h - 3; gy += 6) for (let gx = m.x + 3; gx < m.x + m.w - 3; gx += 6) {
        samples++;
        const x = d.x + (gx + 0.5) * kx, y = d.y + (gy + 0.5) * ky;
        if (fr.some((b) => x >= b.left && x < b.right && y >= b.top && y < b.bottom)) overFace++;
        const el = document.elementFromPoint(x, y);
        if (el && faces.some((f) => f === el || f.contains(el))) answeredByFace++;
      }
      return { faces: faces.length, samples, overFace, answeredByFace };
    })()`);
    const overlap = r.overFace > 0;
    const stacked = r.answeredByFace === 0;
    const notes = [
      `File menu: "${menu.text || menu.cls}" at guest (${menu.ax}, ${menu.ay}) ${menu.w}x${menu.h} (screen ${g.gw}x${g.gh}); on screen: ${inScreen}`,
      `Sounds faces (iframes): ${r.faces}; menu samples ${r.samples}, over a face ${r.overFace}, answered by a face ${r.answeredByFace}`,
      `menu in front of the face: ${stacked}; the menu overlaps a face: ${overlap}`,
    ];
    log(notes.join(' | '));
    await guest.escape();
    await sleep(500);
    const ok = inScreen && overlap && stacked;
    return result(ok, (await guest.state())?.state ?? null, [...start.notes, ...notes], { menu: { x: menu.ax, y: menu.ay, w: menu.w, h: menu.h }, ...r });
  },
};
