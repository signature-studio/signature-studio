// plugins/builtin-effects-low-lifter-low-lifter-x64: Low Lifter (effects, DLL Low Lifter_x64.dll (3793408 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-low-lifter-low-lifter-x64',
  covers: ['plugins/effects/low-lifter-low-lifter-x64', 'plugins/window/effects-low-lifter-low-lifter-x64', 'plugins/param/effects-low-lifter-low-lifter-x64', 'plugins/audio/effects-low-lifter-low-lifter-x64', 'plugins/stop/effects-low-lifter-low-lifter-x64'],
  expectFail: '#1453',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Low Lifter", kind: 'effects' });
  },
});
