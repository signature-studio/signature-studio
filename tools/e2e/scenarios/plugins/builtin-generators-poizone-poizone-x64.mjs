// plugins/builtin-generators-poizone-poizone-x64: PoiZone (generators, DLL PoiZone_x64.dll (3960320 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-generators-poizone-poizone-x64',
  covers: ['plugins/generators/poizone-poizone-x64', 'plugins/window/generators-poizone-poizone-x64', 'plugins/param/generators-poizone-poizone-x64', 'plugins/audio/generators-poizone-poizone-x64', 'plugins/stop/generators-poizone-poizone-x64'],
  expectFail: '#1474',
  async run({ guest }) {
    return pluginSteps(guest, { name: "PoiZone", kind: 'generators' });
  },
});
