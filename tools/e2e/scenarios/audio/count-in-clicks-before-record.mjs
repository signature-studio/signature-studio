// audio/count-in-clicks-before-record: with count-in on and record armed, Space gives sound before the recording (the count). Check: countin-clicks (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
// expectFail 'new' (found by this run, Issue not yet made): count-in on and record armed: no sound within 4 s after Space (the same as record-arm-then-play).
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/count-in-clicks-before-record',
  covers: ['audio/transport/count-in-clicks'],
  displays: ['4k-dpr1'],
  expectFail: 'new',
  run: (ctx) => runCheck(ctx, 'countin-clicks'),
});
