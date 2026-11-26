// plugins/builtin-effects-fruity-send-fruity-send-x64: Fruity Send (effects, DLL Fruity Send_x64.dll (3708416 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-fruity-send-fruity-send-x64',
  covers: ['plugins/effects/fruity-send-fruity-send-x64', 'plugins/window/effects-fruity-send-fruity-send-x64', 'plugins/param/effects-fruity-send-fruity-send-x64', 'plugins/audio/effects-fruity-send-fruity-send-x64', 'plugins/stop/effects-fruity-send-fruity-send-x64'],
  expectFail: '#1448',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity Send", kind: 'effects' });
  },
});
