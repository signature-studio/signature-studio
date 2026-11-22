import { expect, it } from 'vitest';
import { DeskGate, newDeskControl } from '../../src/engine/desk-link.ts';

it('serves a request through shared memory before the wake message is delivered', () => {
  const control = newDeskControl();
  const engine = new DeskGate(control), render = new DeskGate(control);
  render.request();
  // The guest is running: its scheduler services the request while the message
  // queue is blocked. The delayed wake must not compose the same frame again.
  expect(engine.takeRequest()).toBe(true);
  engine.publish(1);
  expect(engine.busy).toBe(true);
  expect(engine.takeRequest()).toBe(false);
  render.ack(1);
  expect(engine.busy).toBe(false);
  render.request();
  expect(engine.takeRequest()).toBe(true);
  expect(engine.takeRequest()).toBe(false);
});
