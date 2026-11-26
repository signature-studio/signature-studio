// plugins/builtin-effects-transporter-transporter-x64: Transporter (effects, DLL Transporter_x64.dll (4170240 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-transporter-transporter-x64',
  covers: ['plugins/effects/transporter-transporter-x64', 'plugins/window/effects-transporter-transporter-x64', 'plugins/param/effects-transporter-transporter-x64', 'plugins/audio/effects-transporter-transporter-x64', 'plugins/stop/effects-transporter-transporter-x64'],
  expectFail: '#1458',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Transporter", kind: 'effects' });
  },
});
