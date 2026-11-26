// plugins/builtin-effects-fruity-soft-clipper-fruity-soft-clipper-x64: Fruity Soft Clipper (effects, DLL Fruity Soft Clipper_x64.dll (3718656 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-fruity-soft-clipper-fruity-soft-clipper-x64',
  covers: ['plugins/effects/fruity-soft-clipper-fruity-soft-clipper-x64', 'plugins/window/effects-fruity-soft-clipper-fruity-soft-clipper-x64', 'plugins/param/effects-fruity-soft-clipper-fruity-soft-clipper-x64', 'plugins/audio/effects-fruity-soft-clipper-fruity-soft-clipper-x64', 'plugins/stop/effects-fruity-soft-clipper-fruity-soft-clipper-x64'],
  expectFail: '#1448',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity Soft Clipper", kind: 'effects' });
  },
});
