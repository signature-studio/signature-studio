// plugins/builtin-effects-fruity-notebook-fruity-notebook-x64: Fruity NoteBook (effects, DLL Fruity NoteBook_x64.dll (3610624 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-fruity-notebook-fruity-notebook-x64',
  covers: ['plugins/effects/fruity-notebook-fruity-notebook-x64', 'plugins/window/effects-fruity-notebook-fruity-notebook-x64', 'plugins/param/effects-fruity-notebook-fruity-notebook-x64', 'plugins/audio/effects-fruity-notebook-fruity-notebook-x64', 'plugins/stop/effects-fruity-notebook-fruity-notebook-x64'],
  expectFail: '#1444',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity NoteBook", kind: 'effects' });
  },
});
