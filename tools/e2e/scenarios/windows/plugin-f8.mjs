// windows/plugin-f8: F8 opens the plugin list (TPlugListForm, top-level). After it, the Channel rack is shown again (the cleanup of the chain, Issue #1507).
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginListOpen, rackBack } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/plugin-f8',
  covers: ['windows/window/tpluglistform', 'windows/plugin/open'],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  async run({ guest }) {
    const r = await pluginListOpen(guest, 'F8');
    const rack = await rackBack(guest);
    return { ok: r.ok && rack, results: { ...r.results, rackVisible: rack }, notes: [...r.notes, `the Channel rack shown after the cleanup: ${rack}`] };
  },
});
