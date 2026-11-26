// plugins/builtin-effects-transmitter-transmitter-x64: Transmitter (effects, DLL Transmitter_x64.dll (4210176 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-transmitter-transmitter-x64',
  covers: ['plugins/effects/transmitter-transmitter-x64', 'plugins/window/effects-transmitter-transmitter-x64', 'plugins/param/effects-transmitter-transmitter-x64', 'plugins/audio/effects-transmitter-transmitter-x64', 'plugins/stop/effects-transmitter-transmitter-x64'],
  expectFail: '#1457',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Transmitter", kind: 'effects' });
  },
});
