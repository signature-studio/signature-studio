// page/zoom-100: the page at its own zoom of 100 %: the canvas is crisp and the picture keeps drawing (the baseline of the zoom steps).
import { defineScenario } from '../../lib/scenario.mjs';
import { zoomCase } from '../../lib/page.mjs';

export default defineScenario({
  id: 'page/zoom-100',
  covers: ['page/zoom/100'],
  displays: ['dpr:1'],
  async run(ctx) {
    return zoomCase(ctx, 1);
  },
});
