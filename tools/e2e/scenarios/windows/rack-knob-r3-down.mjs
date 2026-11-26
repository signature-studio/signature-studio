// windows/rack-knob-r3-down: channel 3's knob is dragged down by 30 guest px: the knob's picture changes.
import { defineScenario } from '../../lib/scenario.mjs';
import { knobDrag } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/rack-knob-r3-down',
  covers: ['windows/rack/knob'],
  async run({ guest }) {
    return knobDrag(guest, 2, -30);
  },
});
