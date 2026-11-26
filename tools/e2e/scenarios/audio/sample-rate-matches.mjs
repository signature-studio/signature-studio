// audio/sample-rate-matches: the context, the ring and the configured rate are equal. Check: sample-rate (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/sample-rate-matches',
  covers: ['audio/output/sample-rate'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'sample-rate'),
});
