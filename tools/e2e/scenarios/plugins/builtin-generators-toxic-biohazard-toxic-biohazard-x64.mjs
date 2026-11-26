// plugins/builtin-generators-toxic-biohazard-toxic-biohazard-x64: Toxic Biohazard (generators, DLL Toxic Biohazard_x64.dll (755200 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-generators-toxic-biohazard-toxic-biohazard-x64',
  covers: ['plugins/generators/toxic-biohazard-toxic-biohazard-x64', 'plugins/window/generators-toxic-biohazard-toxic-biohazard-x64', 'plugins/param/generators-toxic-biohazard-toxic-biohazard-x64', 'plugins/audio/generators-toxic-biohazard-toxic-biohazard-x64', 'plugins/stop/generators-toxic-biohazard-toxic-biohazard-x64'],
  expectFail: '#1476',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Toxic Biohazard", kind: 'generators' });
  },
});
