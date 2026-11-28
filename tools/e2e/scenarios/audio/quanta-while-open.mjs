// audio/quanta-while-open: once the output is open the page keeps taking its quanta (the worklet's counter rises over 2 s). The count is the total of the quanta while open and while playing
// (the page's quantaWhileOpen counts only the quanta while the device is not playing, so it stays flat while the transport plays). Check: quanta-rise (lib/audio.mjs).
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/quanta-while-open',
  covers: ['page/module/audio-start', 'audio/output/quanta-while-open'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'quanta-rise', { ms: 2000 }),
});
