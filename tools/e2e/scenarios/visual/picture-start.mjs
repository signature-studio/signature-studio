// visual/picture-start: at the start, the whole picture is the golden of the start (start.tsv).
import { defineScenario } from '../../lib/scenario.mjs';
import { pictureStart } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/picture-start',
  covers: ['visual/picture/start'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => pictureStart(ctx),
});
