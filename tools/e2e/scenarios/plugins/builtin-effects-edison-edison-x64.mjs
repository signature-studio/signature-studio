// plugins/builtin-effects-edison-edison-x64: Edison (effects, DLL Edison_x64.dll (6223360 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-edison-edison-x64',
  covers: ['plugins/effects/edison-edison-x64', 'plugins/window/effects-edison-edison-x64', 'plugins/param/effects-edison-edison-x64', 'plugins/audio/effects-edison-edison-x64', 'plugins/stop/effects-edison-edison-x64'],
  expectFail: '#1435',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Edison", kind: 'effects' });
  },
});
