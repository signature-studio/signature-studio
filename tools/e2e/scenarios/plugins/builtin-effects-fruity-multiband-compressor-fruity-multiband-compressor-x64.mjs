// plugins/builtin-effects-fruity-multiband-compressor-fruity-multiband-compressor-x64: Fruity Multiband Compressor (effects, DLL Fruity Multiband Compressor_x64.dll (3852800 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-fruity-multiband-compressor-fruity-multiband-compressor-x64',
  covers: ['plugins/effects/fruity-multiband-compressor-fruity-multiband-compressor-x64', 'plugins/window/effects-fruity-multiband-compressor-fruity-multiband-compressor-x64', 'plugins/param/effects-fruity-multiband-compressor-fruity-multiband-compressor-x64', 'plugins/audio/effects-fruity-multiband-compressor-fruity-multiband-compressor-x64', 'plugins/stop/effects-fruity-multiband-compressor-fruity-multiband-compressor-x64'],
  expectFail: '#1444',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity Multiband Compressor", kind: 'effects' });
  },
});
