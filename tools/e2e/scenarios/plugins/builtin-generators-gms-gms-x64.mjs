// plugins/builtin-generators-gms-gms-x64: GMS (generators, DLL GMS_x64.dll (1552896 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-generators-gms-gms-x64',
  covers: ['plugins/generators/gms-gms-x64', 'plugins/window/generators-gms-gms-x64', 'plugins/param/generators-gms-gms-x64', 'plugins/audio/generators-gms-gms-x64', 'plugins/stop/generators-gms-gms-x64'],
  expectFail: '#1471',
  async run({ guest }) {
    return pluginSteps(guest, { name: "GMS", kind: 'generators' });
  },
});
