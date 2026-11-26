// plugins/builtin-effects-fruity-scratcher-fruity-scratcher-x64: Fruity Scratcher (effects, DLL Fruity Scratcher_x64.dll (4272128 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-fruity-scratcher-fruity-scratcher-x64',
  covers: ['plugins/effects/fruity-scratcher-fruity-scratcher-x64', 'plugins/window/effects-fruity-scratcher-fruity-scratcher-x64', 'plugins/param/effects-fruity-scratcher-fruity-scratcher-x64', 'plugins/audio/effects-fruity-scratcher-fruity-scratcher-x64', 'plugins/stop/effects-fruity-scratcher-fruity-scratcher-x64'],
  expectFail: '#1448',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity Scratcher", kind: 'effects' });
  },
});
