// plugins/builtin-effects-fruity-flanger-fruity-flanger-x64: Fruity Flanger (effects, DLL Fruity Flanger_x64.dll (3716608 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-fruity-flanger-fruity-flanger-x64',
  covers: ['plugins/effects/fruity-flanger-fruity-flanger-x64', 'plugins/window/effects-fruity-flanger-fruity-flanger-x64', 'plugins/param/effects-fruity-flanger-fruity-flanger-x64', 'plugins/audio/effects-fruity-flanger-fruity-flanger-x64', 'plugins/stop/effects-fruity-flanger-fruity-flanger-x64'],
  expectFail: '#1441',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity Flanger", kind: 'effects' });
  },
});
