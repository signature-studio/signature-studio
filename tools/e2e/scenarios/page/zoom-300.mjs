// page/zoom-300: the page at a browser zoom of 300 % (the largest zoom step): the canvas is crisp at the new ratio and the picture keeps drawing.
import { defineScenario } from '../../lib/scenario.mjs';
import { zoomCase } from '../../lib/page.mjs';

export default defineScenario({
  id: 'page/zoom-300',
  covers: ['page/zoom/300'],
  displays: ['dpr:1'],
  async run(ctx) {
    return zoomCase(ctx, 3);
  },
});
