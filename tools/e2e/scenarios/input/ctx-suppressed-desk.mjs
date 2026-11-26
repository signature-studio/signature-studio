// input/ctx-suppressed-desk: the browser's context menu is cancelled on the desk (#desktop).
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/ctx-suppressed-desk',
  covers: ['input/menu/browser-menu-desktop'],
  run: (ctx) => I.ctxSuppressed(ctx, 'desktop'),
});
