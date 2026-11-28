// windows/zz-f12-plugin: F12 closes the plugin list (the table keeps only the main form). Known defect (#1510, read on 4k-dpr1): with the plugin list open, F12 leaves it open and hides nothing; with no window open, F12 hides the Channel rack, the Playlist and the Mixer, and a second F12 does not show them. It runs last in its shard (the zz- prefix). After it, the Channel rack is shown again (the cleanup, Issue #1507).
import { defineScenario } from '../../lib/scenario.mjs';
import { f12CloseAll, rackBack } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/zz-f12-plugin',
  covers: ['windows/op/f12'],
  expectFail: 'new',
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  async run({ guest }) {
    const r = await f12CloseAll(guest, 'F8');
    const rack = await rackBack(guest);
    return { ok: r.ok && rack, results: { ...r.results, rackVisible: rack }, notes: [...r.notes, `the Channel rack shown after the cleanup: ${rack}`] };
  },
});
