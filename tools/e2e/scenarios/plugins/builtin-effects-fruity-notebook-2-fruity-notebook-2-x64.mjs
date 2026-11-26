// plugins/builtin-effects-fruity-notebook-2-fruity-notebook-2-x64: Fruity NoteBook 2 (effects, DLL Fruity NoteBook 2_x64.dll (3612672 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-fruity-notebook-2-fruity-notebook-2-x64',
  covers: ['plugins/effects/fruity-notebook-2-fruity-notebook-2-x64', 'plugins/window/effects-fruity-notebook-2-fruity-notebook-2-x64', 'plugins/param/effects-fruity-notebook-2-fruity-notebook-2-x64', 'plugins/audio/effects-fruity-notebook-2-fruity-notebook-2-x64', 'plugins/stop/effects-fruity-notebook-2-fruity-notebook-2-x64'],
  expectFail: '#1445',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity NoteBook 2", kind: 'effects' });
  },
});
