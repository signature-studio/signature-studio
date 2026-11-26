// plugins/builtin-effects-fruity-fast-lp-fruity-fast-lp-x64: Fruity Fast LP (effects, DLL Fruity Fast LP_x64.dll (3752448 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-fruity-fast-lp-fruity-fast-lp-x64',
  covers: ['plugins/effects/fruity-fast-lp-fruity-fast-lp-x64', 'plugins/window/effects-fruity-fast-lp-fruity-fast-lp-x64', 'plugins/param/effects-fruity-fast-lp-fruity-fast-lp-x64', 'plugins/audio/effects-fruity-fast-lp-fruity-fast-lp-x64', 'plugins/stop/effects-fruity-fast-lp-fruity-fast-lp-x64'],
  expectFail: '#1441',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity Fast LP", kind: 'effects' });
  },
});
