// plugins/builtin-effects-emphasizer-emphasizer-x64: Emphasizer (effects, DLL Emphasizer_x64.dll (3776512 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-emphasizer-emphasizer-x64',
  covers: ['plugins/effects/emphasizer-emphasizer-x64', 'plugins/window/effects-emphasizer-emphasizer-x64', 'plugins/param/effects-emphasizer-emphasizer-x64', 'plugins/audio/effects-emphasizer-emphasizer-x64', 'plugins/stop/effects-emphasizer-emphasizer-x64'],
  expectFail: '#1436',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Emphasizer", kind: 'effects' });
  },
});
