import { describe, expect, it } from 'vitest';
import { stopLines, type StopFacts } from '../../src/ui/stop-message.ts';

const facts = (o: Partial<StopFacts> = {}): StopFacts => ({
  info: { reason: 'stopped', code: 134, message: 'the engine exited with code 134' },
  runtimeMessage: 'fle: indirect call to 0 is not a known function entry', stopLine: '== stop: SIGABRT (runtime abort, see runtime message)', now: 100_000,
  lastInput: { desc: 'key up vk 0xd scan 0x1c (keyup)', at: 97_600 }, pendingInput: 0,
  lastPictureChange: { at: 99_900 }, ...o,
});

describe('the message when the engine has stopped: what and where', () => {
  it('says WHAT: a wall of the engine in its own words, and the exit code', () => {
    const l = stopLines(facts());
    expect(l[0]).toBe('The engine stopped at a wall: fle: indirect call to 0 is not a known function entry');
    expect(l[1]).toBe('exit code 134 · == stop: SIGABRT (runtime abort, see runtime message)');
  });
  it('says WHERE: the last input and how long before the stop, the last time the picture changed', () => {
    const t = stopLines(facts()).join('\n');
    expect(t).toContain('Last input sent: key up vk 0xd scan 0x1c (keyup), 2.4 s before the stop; 0 input events were still pending (never processed).');
    expect(t).toContain('The desktop picture last changed 0.1 s before the stop.');
    expect(t).not.toContain('window list');
  });
  it('says that nothing answers input any more', () => {
    expect(stopLines(facts()).at(-1)).toContain('nothing answers input and the page sends none');
  });
  it('a limit of the harness is not called a wall; a Worker that was terminated has no exit code', () => {
    const l = stopLines(facts({ runtimeMessage: 'limit: wall time 60 s after CreateFruityInstance began (FLE_FRUITY_SECONDS)', info: { reason: 'stopped', code: -1, message: 'terminated' } }));
    expect(l[0]).toBe('The engine stopped: limit: wall time 60 s after CreateFruityInstance began (FLE_FRUITY_SECONDS)');
    expect(l[1]).toContain('no exit code (the Worker was terminated)');
  });
  it('no message from the engine: the page says what it knows', () => {
    expect(stopLines(facts({ runtimeMessage: '' }))[0]).toBe('The engine stopped (the engine exited with code 134).');
    expect(stopLines(facts({ runtimeMessage: '', info: { reason: 'finished', code: 0, message: 'the engine exited with code 0' } }))[0]).toBe('The engine finished (the engine exited with code 0).');
  });
  it('a session the user ended is not called a wall; the engine\'s own words are kept', () => {
    const l = stopLines(facts({ endedFromPage: true, runtimeMessage: 'fle: MessageLoop: idle: the host ended the wait (no event came; the session was closed)' }));
    expect(l[0]).toBe('The session was ended from the page (End session). The engine\'s last words: fle: MessageLoop: idle: the host ended the wait (no event came; the session was closed)');
    expect(l[0]).not.toContain('at a wall');
  });
  it('without any input it says so, and it counts the events that never got processed', () => {
    expect(stopLines(facts({ lastInput: null })).join('\n')).toContain('No input was sent before the stop.');
    expect(stopLines(facts({ pendingInput: 3 })).join('\n')).toContain('3 input events were still pending (never processed)');
  });
  it('leaves out the lines it has no facts for', () => {
    const t = stopLines(facts({ lastPictureChange: null })).join('\n');
    expect(t).not.toContain('window list');
    expect(t).not.toContain('picture changed');
  });
});
