// input/ctx-suppressed-sidebar: the browser's context menu is cancelled on the sidebar (a contextmenu event on the hidden element).
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/ctx-suppressed-sidebar',
  covers: ['input/menu/browser-menu-sidebar'],
  run: (ctx) => I.ctxSuppressed(ctx, 'sidebar'),
});
