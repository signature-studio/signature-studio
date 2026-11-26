// page/zoom-75: the page at a browser zoom of 75 %: the canvas is crisp at the new ratio and the picture keeps drawing.
import { defineScenario } from '../../lib/scenario.mjs';
import { zoomCase } from '../../lib/page.mjs';

export default defineScenario({
  id: 'page/zoom-75',
  covers: ['page/zoom/75'],
  displays: ['dpr:1'],
  async run(ctx) {
    return zoomCase(ctx, 0.75);
  },
});
