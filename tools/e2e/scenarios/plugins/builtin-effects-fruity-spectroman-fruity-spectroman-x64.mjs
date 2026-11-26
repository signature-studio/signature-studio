// plugins/builtin-effects-fruity-spectroman-fruity-spectroman-x64: Fruity Spectroman (effects, DLL Fruity Spectroman_x64.dll (3756544 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-fruity-spectroman-fruity-spectroman-x64',
  covers: ['plugins/effects/fruity-spectroman-fruity-spectroman-x64', 'plugins/window/effects-fruity-spectroman-fruity-spectroman-x64', 'plugins/param/effects-fruity-spectroman-fruity-spectroman-x64', 'plugins/audio/effects-fruity-spectroman-fruity-spectroman-x64', 'plugins/stop/effects-fruity-spectroman-fruity-spectroman-x64'],
  expectFail: '#1449',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity Spectroman", kind: 'effects' });
  },
});
