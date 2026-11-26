// plugins/builtin-effects-fruity-waveshaper-fruity-waveshaper-x64: Fruity WaveShaper (effects, DLL Fruity WaveShaper_x64.dll (5742592 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-fruity-waveshaper-fruity-waveshaper-x64',
  covers: ['plugins/effects/fruity-waveshaper-fruity-waveshaper-x64', 'plugins/window/effects-fruity-waveshaper-fruity-waveshaper-x64', 'plugins/param/effects-fruity-waveshaper-fruity-waveshaper-x64', 'plugins/audio/effects-fruity-waveshaper-fruity-waveshaper-x64', 'plugins/stop/effects-fruity-waveshaper-fruity-waveshaper-x64'],
  expectFail: '#1450',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity WaveShaper", kind: 'effects' });
  },
});
