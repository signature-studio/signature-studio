// plugins/builtin-effects-fruity-delay-3-fruity-delay-3-x64: Fruity Delay 3 (effects, DLL Fruity Delay 3_x64.dll (3939328 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-fruity-delay-3-fruity-delay-3-x64',
  covers: ['plugins/effects/fruity-delay-3-fruity-delay-3-x64', 'plugins/window/effects-fruity-delay-3-fruity-delay-3-x64', 'plugins/param/effects-fruity-delay-3-fruity-delay-3-x64', 'plugins/audio/effects-fruity-delay-3-fruity-delay-3-x64', 'plugins/stop/effects-fruity-delay-3-fruity-delay-3-x64'],
  expectFail: 'new',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity Delay 3", kind: 'effects' });
  },
});
