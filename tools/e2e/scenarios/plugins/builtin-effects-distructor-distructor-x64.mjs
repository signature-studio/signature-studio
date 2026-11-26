// plugins/builtin-effects-distructor-distructor-x64: Distructor (effects, DLL Distructor_x64.dll (3656704 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-distructor-distructor-x64',
  covers: ['plugins/effects/distructor-distructor-x64', 'plugins/window/effects-distructor-distructor-x64', 'plugins/param/effects-distructor-distructor-x64', 'plugins/audio/effects-distructor-distructor-x64', 'plugins/stop/effects-distructor-distructor-x64'],
  expectFail: '#1434',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Distructor", kind: 'effects' });
  },
});
