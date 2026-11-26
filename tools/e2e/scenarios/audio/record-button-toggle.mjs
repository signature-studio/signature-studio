// audio/record-button-toggle: the record button changes its picture when clicked, and changes back on the second click. Check: record-toggle (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
// expectFail 'new' (found by this run, Issue not yet made): the record button: the click gave no change of its picture in this run (the state before it is not known: it may be armed).
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/record-button-toggle',
  covers: ['audio/transport/record-toggle'],
  displays: ['4k-dpr1'],
  expectFail: 'new',
  run: (ctx) => runCheck(ctx, 'record-toggle'),
});
