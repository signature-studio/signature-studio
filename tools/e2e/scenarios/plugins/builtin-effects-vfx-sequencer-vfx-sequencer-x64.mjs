// plugins/builtin-effects-vfx-sequencer-vfx-sequencer-x64: VFX Sequencer (effects, DLL VFX Sequencer_x64.dll (4172288 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-vfx-sequencer-vfx-sequencer-x64',
  covers: ['plugins/effects/vfx-sequencer-vfx-sequencer-x64', 'plugins/window/effects-vfx-sequencer-vfx-sequencer-x64', 'plugins/param/effects-vfx-sequencer-vfx-sequencer-x64', 'plugins/audio/effects-vfx-sequencer-vfx-sequencer-x64', 'plugins/stop/effects-vfx-sequencer-vfx-sequencer-x64'],
  expectFail: '#1460',
  async run({ guest }) {
    return pluginSteps(guest, { name: "VFX Sequencer", kind: 'effects' });
  },
});
