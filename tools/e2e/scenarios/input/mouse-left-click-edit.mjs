// input/mouse-left-click-edit: a left click on the Edit label opens its drop-down (a new TQuickPopupMenuWindow); Escape closes it.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/mouse-left-click-edit',
  covers: ['input/mouse/left-click'],
  run: (ctx) => I.menuLabelClick(ctx, 'Edit'),
});
