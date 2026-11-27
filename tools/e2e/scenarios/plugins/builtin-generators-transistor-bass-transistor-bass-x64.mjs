// plugins/builtin-generators-transistor-bass-transistor-bass-x64: Transistor Bass (generators, DLL Transistor Bass_x64.dll (4056576 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-generators-transistor-bass-transistor-bass-x64',
  covers: ['plugins/generators/transistor-bass-transistor-bass-x64', 'plugins/window/generators-transistor-bass-transistor-bass-x64', 'plugins/param/generators-transistor-bass-transistor-bass-x64', 'plugins/audio/generators-transistor-bass-transistor-bass-x64', 'plugins/stop/generators-transistor-bass-transistor-bass-x64'],
  expectFail: '#1535',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Transistor Bass", kind: 'generators' });
  },
});
