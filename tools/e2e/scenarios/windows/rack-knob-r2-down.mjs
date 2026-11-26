// windows/rack-knob-r2-down: channel 2's knob is dragged down by 30 guest px: the knob's picture changes.
import { defineScenario } from '../../lib/scenario.mjs';
import { knobDrag } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/rack-knob-r2-down',
  covers: ['windows/rack/knob'],
  async run({ guest }) {
    return knobDrag(guest, 1, -30);
  },
});
