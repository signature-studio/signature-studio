// audio/tempo-value-changes: the tempo box changes when the wheel is turned, and it is turned back. Check: tempo-value (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/tempo-value-changes',
  covers: ['audio/transport/tempo-change'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'tempo-value'),
});
