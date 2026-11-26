// windows/playlist-menu: a right-click on the Playlist grid opens one drop-down.
import { defineScenario } from '../../lib/scenario.mjs';
import { playlistMenu } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/playlist-menu',
  covers: ['windows/playlist/menu', 'windows/popup/right-click-playlist'],
  async run({ guest }) {
    return playlistMenu(guest);
  },
});
