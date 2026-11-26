// plugins/builtin-effects-razer-chroma-razer-chroma-x64: Razer Chroma (effects, DLL Razer Chroma_x64.dll (4046848 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-razer-chroma-razer-chroma-x64',
  covers: ['plugins/effects/razer-chroma-razer-chroma-x64', 'plugins/window/effects-razer-chroma-razer-chroma-x64', 'plugins/param/effects-razer-chroma-razer-chroma-x64', 'plugins/audio/effects-razer-chroma-razer-chroma-x64', 'plugins/stop/effects-razer-chroma-razer-chroma-x64'],
  expectFail: '#1456',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Razer Chroma", kind: 'effects' });
  },
});
