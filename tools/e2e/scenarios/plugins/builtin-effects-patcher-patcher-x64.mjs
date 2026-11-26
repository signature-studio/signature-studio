// plugins/builtin-effects-patcher-patcher-x64: Patcher (effects, DLL Patcher_x64.dll (7056384 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-patcher-patcher-x64',
  covers: ['plugins/effects/patcher-patcher-x64', 'plugins/window/effects-patcher-patcher-x64', 'plugins/param/effects-patcher-patcher-x64', 'plugins/audio/effects-patcher-patcher-x64', 'plugins/stop/effects-patcher-patcher-x64'],
  expectFail: '#1455',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Patcher", kind: 'effects' });
  },
});
