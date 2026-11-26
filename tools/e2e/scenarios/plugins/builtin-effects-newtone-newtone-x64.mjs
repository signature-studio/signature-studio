// plugins/builtin-effects-newtone-newtone-x64: Newtone (effects, DLL Newtone_x64.dll (5077504 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-newtone-newtone-x64',
  covers: ['plugins/effects/newtone-newtone-x64', 'plugins/window/effects-newtone-newtone-x64', 'plugins/param/effects-newtone-newtone-x64', 'plugins/audio/effects-newtone-newtone-x64', 'plugins/stop/effects-newtone-newtone-x64'],
  expectFail: '#1455',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Newtone", kind: 'effects' });
  },
});
