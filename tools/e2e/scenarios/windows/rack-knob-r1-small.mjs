// windows/rack-knob-r1-small: channel 1's knob is dragged up by 10 guest px: the knob's picture changes.
import { defineScenario } from '../../lib/scenario.mjs';
import { knobDrag } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/rack-knob-r1-small',
  covers: ['windows/rack/knob'],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  async run({ guest }) {
    return knobDrag(guest, 0, 10);
  },
});
