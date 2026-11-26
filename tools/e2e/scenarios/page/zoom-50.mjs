// page/zoom-50: the page at a browser zoom of 50 %: the ratio halves and the window's physical size stays; the canvas is crisp and the picture keeps drawing.
import { defineScenario } from '../../lib/scenario.mjs';
import { zoomCase } from '../../lib/page.mjs';

export default defineScenario({
  id: 'page/zoom-50',
  covers: ['page/zoom/50'],
  displays: ['dpr:1'],
  async run(ctx) {
    return zoomCase(ctx, 0.5);
  },
});
