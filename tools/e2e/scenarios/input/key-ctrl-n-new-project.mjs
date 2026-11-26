// input/key-ctrl-n-new-project: Ctrl+N (File > New) on the empty project: the key reaches the engine, no box comes up, the desk stays the same. Not met: Ctrl+N shows the Save as box (TNewProjForm) and the desk changes behind it.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/key-ctrl-n-new-project',
  covers: ['input/key/ctrl-n'],
  expectFail: 'new',
  run: (ctx) => I.ctrlNCase(ctx),
});
