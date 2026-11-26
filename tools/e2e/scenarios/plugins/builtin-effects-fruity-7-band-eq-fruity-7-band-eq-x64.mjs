// plugins/builtin-effects-fruity-7-band-eq-fruity-7-band-eq-x64: Fruity 7 Band EQ (effects, DLL Fruity 7 Band EQ_x64.dll (3670528 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-fruity-7-band-eq-fruity-7-band-eq-x64',
  covers: ['plugins/effects/fruity-7-band-eq-fruity-7-band-eq-x64', 'plugins/window/effects-fruity-7-band-eq-fruity-7-band-eq-x64', 'plugins/param/effects-fruity-7-band-eq-fruity-7-band-eq-x64', 'plugins/audio/effects-fruity-7-band-eq-fruity-7-band-eq-x64', 'plugins/stop/effects-fruity-7-band-eq-fruity-7-band-eq-x64'],
  expectFail: '#1437',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity 7 Band EQ", kind: 'effects' });
  },
});
