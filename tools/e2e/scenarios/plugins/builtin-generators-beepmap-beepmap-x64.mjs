// plugins/builtin-generators-beepmap-beepmap-x64: BeepMap (generators, DLL BeepMap_x64.dll (4286464 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-generators-beepmap-beepmap-x64',
  covers: ['plugins/generators/beepmap-beepmap-x64', 'plugins/window/generators-beepmap-beepmap-x64', 'plugins/param/generators-beepmap-beepmap-x64', 'plugins/audio/generators-beepmap-beepmap-x64', 'plugins/stop/generators-beepmap-beepmap-x64'],
  expectFail: '#1464',
  async run({ guest }) {
    return pluginSteps(guest, { name: "BeepMap", kind: 'generators' });
  },
});
