// windows/playlist-t4-b10: a click on the Playlist grid (track 4, bar 10) with the default tool: the grid's picture changes.
import { defineScenario } from '../../lib/scenario.mjs';
import { playlistCell } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/playlist-t4-b10',
  covers: ['windows/playlist/clip-place'],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  async run({ guest }) {
    return playlistCell(guest, 3, 9);
  },
});
