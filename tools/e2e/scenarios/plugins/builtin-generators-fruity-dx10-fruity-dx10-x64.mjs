// plugins/builtin-generators-fruity-dx10-fruity-dx10-x64: Fruity DX10 (generators, DLL Fruity DX10_x64.dll (4332032 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-generators-fruity-dx10-fruity-dx10-x64',
  covers: ['plugins/generators/fruity-dx10-fruity-dx10-x64', 'plugins/window/generators-fruity-dx10-fruity-dx10-x64', 'plugins/param/generators-fruity-dx10-fruity-dx10-x64', 'plugins/audio/generators-fruity-dx10-fruity-dx10-x64', 'plugins/stop/generators-fruity-dx10-fruity-dx10-x64'],
  expectFail: '#1468',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity DX10", kind: 'generators' });
  },
});
