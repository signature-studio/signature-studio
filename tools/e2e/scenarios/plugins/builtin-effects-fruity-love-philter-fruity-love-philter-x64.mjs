// plugins/builtin-effects-fruity-love-philter-fruity-love-philter-x64: Fruity Love Philter (effects, DLL Fruity Love Philter_x64.dll (5997056 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-fruity-love-philter-fruity-love-philter-x64',
  covers: ['plugins/effects/fruity-love-philter-fruity-love-philter-x64', 'plugins/window/effects-fruity-love-philter-fruity-love-philter-x64', 'plugins/param/effects-fruity-love-philter-fruity-love-philter-x64', 'plugins/audio/effects-fruity-love-philter-fruity-love-philter-x64', 'plugins/stop/effects-fruity-love-philter-fruity-love-philter-x64'],
  expectFail: '#1443',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity Love Philter", kind: 'effects' });
  },
});
