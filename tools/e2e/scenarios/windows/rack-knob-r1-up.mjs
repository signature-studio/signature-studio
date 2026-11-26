// windows/rack-knob-r1-up: channel 1's knob is dragged up by 30 guest px: the knob's picture changes.
import { defineScenario } from '../../lib/scenario.mjs';
import { knobDrag } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/rack-knob-r1-up',
  covers: ['windows/rack/knob'],
  async run({ guest }) {
    return knobDrag(guest, 0, 30);
  },
});
