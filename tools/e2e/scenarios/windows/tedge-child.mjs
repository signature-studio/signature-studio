// windows/tedge-child: the Browser panel has its web content (a TEdgeBrowser child) shown in the table.
import { defineScenario } from '../../lib/scenario.mjs';
import { browserWebChild } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/tedge-child',
  covers: ['windows/child/tedgebrowser'],
  async run({ guest }) {
    return browserWebChild(guest);
  },
});
