// plugins/builtin-effects-pitch-shifter-pitch-shifter-x64: Pitch Shifter (effects, DLL Pitch Shifter_x64.dll (4260864 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-pitch-shifter-pitch-shifter-x64',
  covers: ['plugins/effects/pitch-shifter-pitch-shifter-x64', 'plugins/window/effects-pitch-shifter-pitch-shifter-x64', 'plugins/param/effects-pitch-shifter-pitch-shifter-x64', 'plugins/audio/effects-pitch-shifter-pitch-shifter-x64', 'plugins/stop/effects-pitch-shifter-pitch-shifter-x64'],
  expectFail: '#1455',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Pitch Shifter", kind: 'effects' });
  },
});
