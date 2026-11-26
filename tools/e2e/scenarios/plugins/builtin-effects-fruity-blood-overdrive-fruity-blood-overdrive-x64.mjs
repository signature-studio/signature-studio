// plugins/builtin-effects-fruity-blood-overdrive-fruity-blood-overdrive-x64: Fruity Blood Overdrive (effects, DLL Fruity Blood Overdrive_x64.dll (3727360 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-fruity-blood-overdrive-fruity-blood-overdrive-x64',
  covers: ['plugins/effects/fruity-blood-overdrive-fruity-blood-overdrive-x64', 'plugins/window/effects-fruity-blood-overdrive-fruity-blood-overdrive-x64', 'plugins/param/effects-fruity-blood-overdrive-fruity-blood-overdrive-x64', 'plugins/audio/effects-fruity-blood-overdrive-fruity-blood-overdrive-x64', 'plugins/stop/effects-fruity-blood-overdrive-fruity-blood-overdrive-x64'],
  expectFail: '#1438',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity Blood Overdrive", kind: 'effects' });
  },
});
