// audio/export-wav-length: the WAV of the Wave export is one bar long (4 beats at the New project's 130 BPM), within 0.03 s; 2 channels, 16 bits.
// Check: export-wav (lib/audio.mjs). The display is 4k-dpr1: the render does not depend on the DPR.
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/export-wav-length',
  covers: ['audio/export/wav-length'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'export-wav', { mode: 'length' }),
});
