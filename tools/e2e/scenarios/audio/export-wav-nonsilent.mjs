// audio/export-wav-nonsilent: File > Export > Wave file (Start) writes a WAV whose samples are not silent: the kick's beats (the Pattern) are rendered.
// Check: export-wav (lib/audio.mjs). The file is the one the engine saves (window.__app.saved); the samples are read as 16-bit PCM. The display is 4k-dpr1: the render does not depend on the DPR.
// The export is the one of audio/export-wav-length (exportWav: the page's Save box, the render form, its Start place); a render that does not save is closed, and the result
// says whether the form is still open (leftOpen). Issue #1526: no mark (the Start click saved the file).
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/export-wav-nonsilent',
  covers: ['audio/export/wav-nonsilent'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'export-wav', { mode: 'nonsilent' }),
});
