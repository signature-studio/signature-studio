// page/picture-matches-canvas: the engine's picture has the canvas's size (the canvas shows every pixel of it, none is cut or added).
import { defineScenario } from '../../lib/scenario.mjs';
import { picture } from '../../lib/common.mjs';
import { deskInfo } from '../../lib/page.mjs';

export default defineScenario({
  id: 'page/picture-matches-canvas',
  covers: ['page/canvas/picture-size'],
  async run({ guest, page }) {
    const d = await deskInfo(page);
    const p = await picture(guest);
    const c = d.canvas;
    const ok = c !== null && p.w === c.w && p.h === c.h && p.stride >= p.w * 4;
    return { ok, engineState: d.state, notes: [`picture ${p.w}x${p.h} (row ${p.stride} bytes); canvas ${c ? `${c.w}x${c.h}` : 'none'}`], results: { picture: [p.w, p.h], canvas: c ? [c.w, c.h] : null, stride: p.stride } };
  },
});
