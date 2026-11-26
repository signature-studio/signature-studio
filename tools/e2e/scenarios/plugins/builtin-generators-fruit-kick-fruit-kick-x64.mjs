// plugins/builtin-generators-fruit-kick-fruit-kick-x64: Fruit Kick (generators, DLL Fruit Kick_x64.dll (4268544 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-generators-fruit-kick-fruit-kick-x64',
  covers: ['plugins/generators/fruit-kick-fruit-kick-x64', 'plugins/window/generators-fruit-kick-fruit-kick-x64', 'plugins/param/generators-fruit-kick-fruit-kick-x64', 'plugins/audio/generators-fruit-kick-fruit-kick-x64', 'plugins/stop/generators-fruit-kick-fruit-kick-x64'],
  expectFail: '#1467',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruit Kick", kind: 'generators' });
  },
});
