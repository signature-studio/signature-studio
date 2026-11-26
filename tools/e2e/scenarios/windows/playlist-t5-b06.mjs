// windows/playlist-t5-b06: a click on the Playlist grid (track 5, bar 6) with the default tool: the grid's picture changes.
import { defineScenario } from '../../lib/scenario.mjs';
import { playlistCell } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/playlist-t5-b06',
  covers: ['windows/playlist/clip-place'],
  async run({ guest }) {
    return playlistCell(guest, 4, 5);
  },
});
