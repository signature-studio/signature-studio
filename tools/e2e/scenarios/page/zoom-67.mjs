// page/zoom-67: the page at a browser zoom of 67 %: the canvas is crisp at the new ratio and the picture keeps drawing.
import { defineScenario } from '../../lib/scenario.mjs';
import { zoomCase } from '../../lib/page.mjs';

export default defineScenario({
  id: 'page/zoom-67',
  covers: ['page/zoom/67'],
  displays: ['dpr:1'],
  async run(ctx) {
    return zoomCase(ctx, 0.67);
  },
});
