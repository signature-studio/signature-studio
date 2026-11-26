// plugins/builtin-effects-fruity-vocoder-fruity-vocoder-x64: Fruity Vocoder (effects, DLL Fruity Vocoder_x64.dll (3814912 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-fruity-vocoder-fruity-vocoder-x64',
  covers: ['plugins/effects/fruity-vocoder-fruity-vocoder-x64', 'plugins/window/effects-fruity-vocoder-fruity-vocoder-x64', 'plugins/param/effects-fruity-vocoder-fruity-vocoder-x64', 'plugins/audio/effects-fruity-vocoder-fruity-vocoder-x64', 'plugins/stop/effects-fruity-vocoder-fruity-vocoder-x64'],
  expectFail: '#1450',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity Vocoder", kind: 'effects' });
  },
});
