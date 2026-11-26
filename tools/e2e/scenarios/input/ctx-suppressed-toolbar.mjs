// input/ctx-suppressed-toolbar: the browser's context menu is cancelled on the toolbar (a contextmenu event on the hidden element).
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/ctx-suppressed-toolbar',
  covers: ['input/menu/browser-menu-toolbar'],
  run: (ctx) => I.ctxSuppressed(ctx, 'toolbar'),
});
