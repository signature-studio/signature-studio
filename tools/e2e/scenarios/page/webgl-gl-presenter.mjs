// page/webgl-gl-presenter: the picture is drawn with WebGL2 (the presenter's mode is gl in this runner): a change of the picture shows in the canvas.
import { defineScenario } from '../../lib/scenario.mjs';
import { deskInfo, menuChangesPicture, pictureHash } from '../../lib/page.mjs';

export default defineScenario({
  id: 'page/webgl-gl-presenter',
  covers: ['page/webgl/gl-presenter'],
  async run({ guest, page }) {
    const d = await deskInfo(page);
    const f = await menuChangesPicture(guest, () => pictureHash(guest));
    const drawn = f.pops > 0 && f.shown !== f.after;
    return { ok: d.mode === 'gl' && drawn, engineState: d.state, notes: [`presenter mode ${d.mode}; presents ${d.presents}; the File menu drew ${drawn}`], results: { mode: d.mode, presents: d.presents, drawn } };
  },
});
