// plugins/builtin-effects-tuner-tuner-x64: Tuner (effects, DLL Tuner_x64.dll (3779584 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-tuner-tuner-x64',
  covers: ['plugins/effects/tuner-tuner-x64', 'plugins/window/effects-tuner-tuner-x64', 'plugins/param/effects-tuner-tuner-x64', 'plugins/audio/effects-tuner-tuner-x64', 'plugins/stop/effects-tuner-tuner-x64'],
  expectFail: '#1458',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Tuner", kind: 'effects' });
  },
});
