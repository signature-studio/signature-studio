// plugins/builtin-effects-fruity-parametric-eq-2-fruity-parametric-eq-2-x64: Fruity Parametric EQ 2 (effects, DLL Fruity Parametric EQ 2_x64.dll (4036608 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-fruity-parametric-eq-2-fruity-parametric-eq-2-x64',
  covers: ['plugins/effects/fruity-parametric-eq-2-fruity-parametric-eq-2-x64', 'plugins/window/effects-fruity-parametric-eq-2-fruity-parametric-eq-2-x64', 'plugins/param/effects-fruity-parametric-eq-2-fruity-parametric-eq-2-x64', 'plugins/audio/effects-fruity-parametric-eq-2-fruity-parametric-eq-2-x64', 'plugins/stop/effects-fruity-parametric-eq-2-fruity-parametric-eq-2-x64'],
  expectFail: '#1446',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity Parametric EQ 2", kind: 'effects' });
  },
});
