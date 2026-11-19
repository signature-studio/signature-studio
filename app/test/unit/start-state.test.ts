import { describe, expect, it } from 'vitest';
import { stateAfterFirstPixel } from '../../src/engine/start-state.ts';

describe('stateAfterFirstPixel', () => {
  it('the first pixel before the first idle: the page is running', () => {
    expect(stateAfterFirstPixel('running')).toBe('running');
    expect(stateAfterFirstPixel('loaded')).toBe('running');
  });
  it('the first pixel after the first idle does not take the page back from engine-idle', () => {
    expect(stateAfterFirstPixel('engine-idle')).toBe('engine-idle');
  });
  it('an ended or failed engine stays ended or failed', () => {
    expect(stateAfterFirstPixel('exited')).toBe('exited');
    expect(stateAfterFirstPixel('error')).toBe('error');
  });
});
