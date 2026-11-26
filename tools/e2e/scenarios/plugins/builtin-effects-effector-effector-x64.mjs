// plugins/builtin-effects-effector-effector-x64: Effector (effects, DLL Effector_x64.dll (3726336 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-effector-effector-x64',
  covers: ['plugins/effects/effector-effector-x64', 'plugins/window/effects-effector-effector-x64', 'plugins/param/effects-effector-effector-x64', 'plugins/audio/effects-effector-effector-x64', 'plugins/stop/effects-effector-effector-x64'],
  expectFail: '#1435',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Effector", kind: 'effects' });
  },
});
