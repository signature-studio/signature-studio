// plugins/builtin-effects-vfx-keyboard-splitter-vfx-keyboard-splitter-x64: VFX Keyboard Splitter (effects, DLL VFX Keyboard Splitter_x64.dll (5727232 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-vfx-keyboard-splitter-vfx-keyboard-splitter-x64',
  covers: ['plugins/effects/vfx-keyboard-splitter-vfx-keyboard-splitter-x64', 'plugins/window/effects-vfx-keyboard-splitter-vfx-keyboard-splitter-x64', 'plugins/param/effects-vfx-keyboard-splitter-vfx-keyboard-splitter-x64', 'plugins/audio/effects-vfx-keyboard-splitter-vfx-keyboard-splitter-x64', 'plugins/stop/effects-vfx-keyboard-splitter-vfx-keyboard-splitter-x64'],
  expectFail: '#1459',
  async run({ guest }) {
    return pluginSteps(guest, { name: "VFX Keyboard Splitter", kind: 'effects' });
  },
});
