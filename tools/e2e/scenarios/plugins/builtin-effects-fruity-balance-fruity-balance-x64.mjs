// plugins/builtin-effects-fruity-balance-fruity-balance-x64: Fruity Balance (effects, DLL Fruity Balance_x64.dll (3718656 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-fruity-balance-fruity-balance-x64',
  covers: ['plugins/effects/fruity-balance-fruity-balance-x64', 'plugins/window/effects-fruity-balance-fruity-balance-x64', 'plugins/param/effects-fruity-balance-fruity-balance-x64', 'plugins/audio/effects-fruity-balance-fruity-balance-x64', 'plugins/stop/effects-fruity-balance-fruity-balance-x64'],
  expectFail: '#1484',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity Balance", kind: 'effects' });
  },
});
