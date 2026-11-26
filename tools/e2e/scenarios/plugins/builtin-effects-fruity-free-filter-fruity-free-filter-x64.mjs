// plugins/builtin-effects-fruity-free-filter-fruity-free-filter-x64: Fruity Free Filter (effects, DLL Fruity Free Filter_x64.dll (3777536 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-fruity-free-filter-fruity-free-filter-x64',
  covers: ['plugins/effects/fruity-free-filter-fruity-free-filter-x64', 'plugins/window/effects-fruity-free-filter-fruity-free-filter-x64', 'plugins/param/effects-fruity-free-filter-fruity-free-filter-x64', 'plugins/audio/effects-fruity-free-filter-fruity-free-filter-x64', 'plugins/stop/effects-fruity-free-filter-fruity-free-filter-x64'],
  expectFail: '#1442',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity Free Filter", kind: 'effects' });
  },
});
