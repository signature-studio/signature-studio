// windows/rack-knob-r5-up: channel 5's knob is dragged up by 30 guest px: the knob's picture changes.
import { defineScenario } from '../../lib/scenario.mjs';
import { knobDrag } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/rack-knob-r5-up',
  covers: ['windows/rack/knob'],
  async run({ guest }) {
    return knobDrag(guest, 4, 30);
  },
});
