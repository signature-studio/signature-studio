// plugins/builtin-effects-fruity-fast-dist-fruity-fast-dist-x64: Fruity Fast Dist (effects, DLL Fruity Fast Dist_x64.dll (3703296 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-fruity-fast-dist-fruity-fast-dist-x64',
  covers: ['plugins/effects/fruity-fast-dist-fruity-fast-dist-x64', 'plugins/window/effects-fruity-fast-dist-fruity-fast-dist-x64', 'plugins/param/effects-fruity-fast-dist-fruity-fast-dist-x64', 'plugins/audio/effects-fruity-fast-dist-fruity-fast-dist-x64', 'plugins/stop/effects-fruity-fast-dist-fruity-fast-dist-x64'],
  expectFail: '#1440',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity Fast Dist", kind: 'effects' });
  },
});
