// input/mouse-left-click-file: a left click on the File label opens its drop-down (a new TQuickPopupMenuWindow); Escape closes it.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/mouse-left-click-file',
  covers: ['input/mouse/left-click'],
  run: (ctx) => I.menuLabelClick(ctx, 'File'),
});
