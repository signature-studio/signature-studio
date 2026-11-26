// The ID sorts last (zz-): the record button stays armed once clicked (a second click does not disarm it), and an armed record stops the transport. So it runs last in its shard (README: 音声の領域).
// audio/zz-record-arm-then-play: with the record button armed, Space starts the transport. Check: record-play (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
// expectFail 'new' (found by this run, Issue not yet made): record armed: Space did not start the transport (the time display did not move and no sound in 1.5 s); the cause is not confirmed (the product or the record mode).
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/zz-record-arm-then-play',
  covers: ['audio/transport/record-play'],
  displays: ['4k-dpr1'],
  expectFail: 'new',
  run: (ctx) => runCheck(ctx, 'record-play'),
});
