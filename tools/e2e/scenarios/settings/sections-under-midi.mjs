// settings/sections-under-midi: the MIDI page's sections (Output and Input) are panels under the page in the window table.
import { defineScenario } from '../../lib/scenario.mjs';
import { subtree } from '../../lib/dump.mjs';
import { windowTable } from '../../lib/common.mjs';

export default defineScenario({
  id: 'settings/sections-under-midi',
  covers: ['settings/section/output', 'settings/section/input'],
  async run({ guest }) {
    const t = await windowTable(guest);
    const midi = t.children.find((c) => c.cls === 'TVectorSheet' && c.text === 'MIDI');
    const under = midi ? subtree(t, midi.lid).filter((c) => c.cls === 'TVectorPanel').map((c) => c.text) : [];
    return { ok: under.includes('Output') && under.includes('Input'), engineState: (await guest.state())?.state ?? null, notes: [`sections under MIDI: ${under.join(', ') || 'none'}`] };
  },
});
