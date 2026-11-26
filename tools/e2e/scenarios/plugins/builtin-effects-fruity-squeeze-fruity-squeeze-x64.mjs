// plugins/builtin-effects-fruity-squeeze-fruity-squeeze-x64: Fruity Squeeze (effects, DLL Fruity Squeeze_x64.dll (3826176 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-fruity-squeeze-fruity-squeeze-x64',
  covers: ['plugins/effects/fruity-squeeze-fruity-squeeze-x64', 'plugins/window/effects-fruity-squeeze-fruity-squeeze-x64', 'plugins/param/effects-fruity-squeeze-fruity-squeeze-x64', 'plugins/audio/effects-fruity-squeeze-fruity-squeeze-x64', 'plugins/stop/effects-fruity-squeeze-fruity-squeeze-x64'],
  expectFail: '#1449',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity Squeeze", kind: 'effects' });
  },
});
