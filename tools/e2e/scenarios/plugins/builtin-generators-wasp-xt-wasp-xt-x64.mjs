// plugins/builtin-generators-wasp-xt-wasp-xt-x64: Wasp XT (generators, DLL Wasp XT_x64.dll (3597312 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-generators-wasp-xt-wasp-xt-x64',
  covers: ['plugins/generators/wasp-xt-wasp-xt-x64', 'plugins/window/generators-wasp-xt-wasp-xt-x64', 'plugins/param/generators-wasp-xt-wasp-xt-x64', 'plugins/audio/generators-wasp-xt-wasp-xt-x64', 'plugins/stop/generators-wasp-xt-wasp-xt-x64'],
  expectFail: '#1477',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Wasp XT", kind: 'generators' });
  },
});
