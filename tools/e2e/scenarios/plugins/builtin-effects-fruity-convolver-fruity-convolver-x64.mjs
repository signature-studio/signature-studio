// plugins/builtin-effects-fruity-convolver-fruity-convolver-x64: Fruity Convolver (effects, DLL Fruity Convolver_x64.dll (6156288 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-fruity-convolver-fruity-convolver-x64',
  covers: ['plugins/effects/fruity-convolver-fruity-convolver-x64', 'plugins/window/effects-fruity-convolver-fruity-convolver-x64', 'plugins/param/effects-fruity-convolver-fruity-convolver-x64', 'plugins/audio/effects-fruity-convolver-fruity-convolver-x64', 'plugins/stop/effects-fruity-convolver-fruity-convolver-x64'],
  expectFail: '#1439',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity Convolver", kind: 'effects' });
  },
});
