// input/ctx-suppressed-canvas: the browser's context menu is cancelled on the canvas: every contextmenu event (from right presses) is cancelled.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/ctx-suppressed-canvas',
  covers: ['input/menu/browser-menu-canvas'],
  run: (ctx) => I.ctxSuppressed(ctx, 'canvas'),
});
