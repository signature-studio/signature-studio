// audio/metronome-button-toggle-wide: the metronome button, the region read wider. Check: metronome-button (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/metronome-button-toggle-wide',
  covers: ['audio/metronome/button-changes'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'metronome-button', { r: 20 }),
});
