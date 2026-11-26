// plugins/builtin-effects-gross-beat-gross-beat-x64: Gross Beat (effects, DLL Gross Beat_x64.dll (5850112 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-gross-beat-gross-beat-x64',
  covers: ['plugins/effects/gross-beat-gross-beat-x64', 'plugins/window/effects-gross-beat-gross-beat-x64', 'plugins/param/effects-gross-beat-gross-beat-x64', 'plugins/audio/effects-gross-beat-gross-beat-x64', 'plugins/stop/effects-gross-beat-gross-beat-x64'],
  expectFail: '#1452',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Gross Beat", kind: 'effects' });
  },
});
