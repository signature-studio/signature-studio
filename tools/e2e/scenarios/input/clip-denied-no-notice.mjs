// input/clip-denied-no-notice: under 'denied', a paste and a copy show no notice (no visible live region with text, no #clipboard-status element).
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/clip-denied-no-notice',
  covers: ['input/clipboard/notice-none'],
  run: (ctx) => I.clipNotice(ctx),
});
