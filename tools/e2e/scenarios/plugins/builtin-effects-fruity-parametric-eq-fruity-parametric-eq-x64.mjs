// plugins/builtin-effects-fruity-parametric-eq-fruity-parametric-eq-x64: Fruity Parametric EQ (effects, DLL Fruity Parametric EQ_x64.dll (3816960 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-fruity-parametric-eq-fruity-parametric-eq-x64',
  covers: ['plugins/effects/fruity-parametric-eq-fruity-parametric-eq-x64', 'plugins/window/effects-fruity-parametric-eq-fruity-parametric-eq-x64', 'plugins/param/effects-fruity-parametric-eq-fruity-parametric-eq-x64', 'plugins/audio/effects-fruity-parametric-eq-fruity-parametric-eq-x64', 'plugins/stop/effects-fruity-parametric-eq-fruity-parametric-eq-x64'],
  expectFail: '#1445',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity Parametric EQ", kind: 'effects' });
  },
});
