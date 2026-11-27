// windows/rack-knob-r5-down: channel 5's knob is dragged down by 30 guest px: the knob's picture changes.
import { defineScenario } from '../../lib/scenario.mjs';
import { knobDrag } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/rack-knob-r5-down',
  covers: ['windows/rack/knob'],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  async run({ guest }) {
    return knobDrag(guest, 4, -30);
  },
});
