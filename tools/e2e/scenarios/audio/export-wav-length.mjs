// audio/export-wav-length: the WAV of the Wave export is one bar long (4 beats at the New project's 130 BPM), within 0.03 s; 2 channels, 16 bits.
// Check: export-wav (lib/audio.mjs). The display is 4k-dpr1: the render does not depend on the DPR.
// The export: Wave opens the page's "Save as" box; its Save shows the render form (TWAVRenderForm); the form's Start place starts the render (exportWav). A render that does not
// save is closed (the box by Cancel, the form by its title box), and the result says whether the form is still open (leftOpen). Issue #1526: no mark (the Start click saved the file).
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/export-wav-length',
  covers: ['audio/export/wav-length'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'export-wav', { mode: 'length' }),
});
