// settings/pages-listed: the settings window's sheets are in the window table with their names (MIDI and File at least, as the inventory found them).
import { defineScenario } from '../../lib/scenario.mjs';
import { windowTable } from '../../lib/common.mjs';

export default defineScenario({
  id: 'settings/pages-listed',
  covers: ['settings/page/midi', 'settings/page/file'],
  async run({ guest }) {
    const t = await windowTable(guest);
    const names = new Set(t.children.filter((c) => c.cls === 'TVectorSheet' && c.text).map((c) => c.text));
    const ok = names.has('MIDI') && names.has('File');
    return { ok, engineState: (await guest.state())?.state ?? null, notes: [`sheets: ${[...names].join(', ')}`] };
  },
});
