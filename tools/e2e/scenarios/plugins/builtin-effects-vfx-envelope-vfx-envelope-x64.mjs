// plugins/builtin-effects-vfx-envelope-vfx-envelope-x64: VFX Envelope (effects, DLL VFX Envelope_x64.dll (5758976 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-vfx-envelope-vfx-envelope-x64',
  covers: ['plugins/effects/vfx-envelope-vfx-envelope-x64', 'plugins/window/effects-vfx-envelope-vfx-envelope-x64', 'plugins/param/effects-vfx-envelope-vfx-envelope-x64', 'plugins/audio/effects-vfx-envelope-vfx-envelope-x64', 'plugins/stop/effects-vfx-envelope-vfx-envelope-x64'],
  expectFail: '#1459',
  async run({ guest }) {
    return pluginSteps(guest, { name: "VFX Envelope", kind: 'effects' });
  },
});
