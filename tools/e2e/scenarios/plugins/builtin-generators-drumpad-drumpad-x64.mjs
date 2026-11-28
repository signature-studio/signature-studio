// plugins/builtin-generators-drumpad-drumpad-x64: Drumpad (generators, DLL Drumpad_x64.dll (4104704 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-generators-drumpad-drumpad-x64',
  covers: ['plugins/generators/drumpad-drumpad-x64', 'plugins/window/generators-drumpad-drumpad-x64', 'plugins/param/generators-drumpad-drumpad-x64', 'plugins/audio/generators-drumpad-drumpad-x64', 'plugins/stop/generators-drumpad-drumpad-x64'],
  expectFail: '#1465',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Drumpad", kind: 'generators' });
  },
});
