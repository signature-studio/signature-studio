// windows/rack-knob-r5-small: channel 5's knob is dragged up by 10 guest px: the knob's picture changes.
import { defineScenario } from '../../lib/scenario.mjs';
import { knobDrag } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/rack-knob-r5-small',
  covers: ['windows/rack/knob'],
  async run({ guest }) {
    return knobDrag(guest, 4, 10);
  },
});
