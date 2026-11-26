// plugins/builtin-effects-fruity-bass-boost-fruity-bass-boost-x64: Fruity Bass Boost (effects, DLL Fruity Bass Boost_x64.dll (3752448 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-fruity-bass-boost-fruity-bass-boost-x64',
  covers: ['plugins/effects/fruity-bass-boost-fruity-bass-boost-x64', 'plugins/window/effects-fruity-bass-boost-fruity-bass-boost-x64', 'plugins/param/effects-fruity-bass-boost-fruity-bass-boost-x64', 'plugins/audio/effects-fruity-bass-boost-fruity-bass-boost-x64', 'plugins/stop/effects-fruity-bass-boost-fruity-bass-boost-x64'],
  expectFail: '#1437',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity Bass Boost", kind: 'effects' });
  },
});
