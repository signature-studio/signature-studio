// plugins/builtin-effects-fruity-html-notebook-fruity-html-notebook-x64: Fruity HTML NoteBook (effects, DLL Fruity HTML NoteBook_x64.dll (3927040 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-fruity-html-notebook-fruity-html-notebook-x64',
  covers: ['plugins/effects/fruity-html-notebook-fruity-html-notebook-x64', 'plugins/window/effects-fruity-html-notebook-fruity-html-notebook-x64', 'plugins/param/effects-fruity-html-notebook-fruity-html-notebook-x64', 'plugins/audio/effects-fruity-html-notebook-fruity-html-notebook-x64', 'plugins/stop/effects-fruity-html-notebook-fruity-html-notebook-x64'],
  expectFail: '#1442',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity HTML NoteBook", kind: 'effects' });
  },
});
