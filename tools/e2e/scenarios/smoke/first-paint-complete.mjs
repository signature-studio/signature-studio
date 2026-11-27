// (d) first-paint-complete (Issue #1483): the start; 3 s after the first idle the desktop picture has no big region of pure black (R, G, B all 0, 4-connected, 100 x 100 px or more in its box).
// The picture is the compositor's as it is (deskDumpBase64(true): the picture drawn again from the textures the canvas holds, read back), and the DPR is the page's own devicePixelRatio.
import { waitStable, sleep } from '../../lib/flow.mjs';
import { blackRegions, decodePicture } from '../../lib/picture.mjs';

const SETTLE_MS = 3000;
const MIN_PX = 100;

async function readPicture(page) {
  const t0 = Date.now();
  for (;;) {
    try {
      return await page.evaluate(() => window.__app.deskDumpBase64(true));
    } catch (e) {
      if (!/ask again/.test(String(e?.message ?? e)) || Date.now() - t0 >= 60000) throw e;
      await sleep(100);
    }
  }
}

export default {
  name: 'first-paint-complete',
  expectFail: null,
  async run({ guest, page, log }) {
    const notes = [];
    const first = await waitStable(guest, 180000);
    notes.push(`first state ${first.state} after ${first.waitedMs} ms`);
    if (first.state !== 'engine-idle') return { ok: false, engineState: first.state, notes };
    await sleep(SETTLE_MS);
    const dpr = await page.evaluate(() => window.devicePixelRatio);
    const shot = await readPicture(page);
    const pic = decodePicture({ base64: shot.base64, width: shot.width, height: shot.height });
    const r = blackRegions(pic, MIN_PX);
    notes.push(`picture ${pic.w} x ${pic.h} at DPR ${dpr}: ${r.bigCount} black region(s) of ${MIN_PX} px or more, ${r.blackPixels} black pixels in ${r.groups} groups`);
    log(notes.at(-1));
    return { ok: r.bigCount === 0, engineState: first.state, notes, results: { dpr, width: pic.w, height: pic.h, bigBlack: r.bigCount, blackPixels: r.blackPixels, groups: r.groups, big: r.big } };
  },
};
