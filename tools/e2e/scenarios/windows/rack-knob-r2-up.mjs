// windows/rack-knob-r2-up: channel 2's knob is dragged up by 30 guest px: the knob's picture changes.
import { defineScenario } from '../../lib/scenario.mjs';
import { knobDrag } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/rack-knob-r2-up',
  covers: ['windows/rack/knob'],
  async run({ guest }) {
    return knobDrag(guest, 1, 30);
  },
});
