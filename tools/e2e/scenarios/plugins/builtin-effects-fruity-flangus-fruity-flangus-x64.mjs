// plugins/builtin-effects-fruity-flangus-fruity-flangus-x64: Fruity Flangus (effects, DLL Fruity Flangus_x64.dll (3686912 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-fruity-flangus-fruity-flangus-x64',
  covers: ['plugins/effects/fruity-flangus-fruity-flangus-x64', 'plugins/window/effects-fruity-flangus-fruity-flangus-x64', 'plugins/param/effects-fruity-flangus-fruity-flangus-x64', 'plugins/audio/effects-fruity-flangus-fruity-flangus-x64', 'plugins/stop/effects-fruity-flangus-fruity-flangus-x64'],
  expectFail: '#1441',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity Flangus", kind: 'effects' });
  },
});
