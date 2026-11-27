// plugins/builtin-effects-fruity-filter-fruity-filter-x64: Fruity Filter (effects, DLL Fruity Filter_x64.dll (3780608 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-fruity-filter-fruity-filter-x64',
  covers: ['plugins/effects/fruity-filter-fruity-filter-x64', 'plugins/window/effects-fruity-filter-fruity-filter-x64', 'plugins/param/effects-fruity-filter-fruity-filter-x64', 'plugins/audio/effects-fruity-filter-fruity-filter-x64', 'plugins/stop/effects-fruity-filter-fruity-filter-x64'],
  expectFail: '#1535',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity Filter", kind: 'effects' });
  },
});
