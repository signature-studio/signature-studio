// plugins/builtin-effects-luxeverb-luxeverb-x64: Luxeverb (effects, DLL LuxeVerb_x64.dll (4341248 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-luxeverb-luxeverb-x64',
  covers: ['plugins/effects/luxeverb-luxeverb-x64', 'plugins/window/effects-luxeverb-luxeverb-x64', 'plugins/param/effects-luxeverb-luxeverb-x64', 'plugins/audio/effects-luxeverb-luxeverb-x64', 'plugins/stop/effects-luxeverb-luxeverb-x64'],
  expectFail: '#1453',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Luxeverb", kind: 'effects' });
  },
});
