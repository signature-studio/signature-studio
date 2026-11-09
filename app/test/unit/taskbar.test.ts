import fs from 'node:fs';
import path from 'node:path';
import { describe, expect, it } from 'vitest';
import { describeRaw, IN, SC, type RawInput } from '../../src/input/events.ts';
import { IdleDriver, type EngineInput } from '../../src/engine/idle-driver.ts';
import { TaskbarRestore, type RestoreButton } from '../../src/ui/taskbar.ts';
import { TaskbarReport } from '../../src/engine/taskbar-report.ts';

// Decision 0272: the page's restore button for a minimized window (the engine's taskbar entry, decision 0267). The Worker reads fle_g3_taskbar_iconic() and posts the window's number when it changes
// (src/engine/worker.ts); this file tests the page's side: the button, the press -> raw event, the road of the event through the IdleDriver, and the markup (a <button>, hidden until the engine reports a window).

class FakeButton extends EventTarget implements RestoreButton {
  hidden: boolean | string = true;
  click(): void { if (!this.hidden) this.dispatchEvent(new Event('click')); }   // a hidden button gets no click (display: none)
}
function make() {
  const button = new FakeButton();
  const sent: RawInput[] = [];
  let focused = 0;
  const t = new TaskbarRestore(button, { send: (ev) => { sent.push(ev); }, focusDesktop: () => { focused++; } });
  return { button, t, sent, focused: () => focused };
}

describe('TaskbarRestore', () => {
  it('shows nothing until the engine reports a minimized window, and a press then sends SC_RESTORE for that window', () => {
    const { button, t, sent, focused } = make();
    expect(button.hidden).toBe(true);
    expect(t.shown).toBe(false);
    button.click();
    expect(sent).toEqual([]);
    t.update(41);
    expect(button.hidden).toBe(false);
    expect(t.shown).toBe(true);
    expect(t.window).toBe(41);
    button.click();
    expect(sent).toEqual([{ k: 'syscmd', kind: IN.SYSCMD, lid: 41, cmd: 0xf120 }]);
    expect(SC.RESTORE).toBe(0xf120);
    expect(t.presses).toBe(1);
    expect(focused()).toBe(1);
  });
  it('is one button: a repeated report changes nothing, a report of another window moves it, 0 hides it again (not shown twice)', () => {
    const { button, t, sent } = make();
    t.update(5); t.update(5); t.update(5);
    expect(t.window).toBe(5);
    expect(button.hidden).toBe(false);
    t.update(9);
    expect(button.hidden).toBe(false);
    button.click();
    expect(sent.map((e) => (e.k === 'syscmd' ? e.lid : -1))).toEqual([9]);
    t.update(0);
    expect(button.hidden).toBe(true);
    expect(t.shown).toBe(false);
    button.click();
    expect(sent.length).toBe(1);
  });
  it('a press while the engine has not reported the restore yet may be repeated: each press is one SC_RESTORE (the engine restores a restored window to itself)', () => {
    const { button, t, sent } = make();
    t.update(3);
    button.click(); button.click();
    expect(sent.length).toBe(2);
    expect(t.presses).toBe(2);
  });
  it('clear() (the engine ended) hides the button and nothing can be sent any more', () => {
    const { button, t, sent } = make();
    t.update(3);
    t.clear();
    expect(button.hidden).toBe(true);
    expect(t.window).toBe(0);
    // a Worker message that was already on its way when the engine ended would show the button again: the page clears it at the exit, and the Worker posts no more after it
    button.hidden = true;
    button.click();
    expect(sent).toEqual([]);
  });
  it('the event says what the engine told: it names no window by a number of its own', () => {
    const { t, sent, button } = make();
    t.update(1234);
    button.click();
    expect(describeRaw(sent[0] as RawInput)).toBe('system command 0xf120 for window 1234');
  });
});

describe('TaskbarReport (what the Worker posts)', () => {
  it('a build without the taskbar entry (null) is never heard of, however often it is asked', () => {
    const r = new TaskbarReport();
    expect([r.next(null), r.next(null), r.next(null)]).toEqual([null, null, null]);
  });
  it('posts only a change: the first 0 is what the page assumes already, a window is posted once, its restore (0) is posted once', () => {
    const r = new TaskbarReport();
    expect([r.next(0), r.next(0), r.next(7), r.next(7), r.next(7), r.next(0), r.next(0), r.next(9), r.next(4)]).toEqual([null, null, 7, null, null, 0, null, 9, 4]);
  });
  it('a tick of a build whose answer goes null in between does not forget what was said', () => {
    const r = new TaskbarReport();
    expect([r.next(5), r.next(null), r.next(5), r.next(0)]).toEqual([5, null, null, 0]);
  });
});

describe('the syscmd event on the IdleDriver road (decision 0272: the same road as a display change)', () => {
  function rig(withSyscommand: boolean) {
    const calls: string[] = [];
    let waiting = true;
    const delivered: number[] = [];
    const engine: EngineInput = {
      mouse: () => 1, key: () => 1, clockTo: (t) => { calls.push(`clock ${t}`); },
      ...(withSyscommand ? { syscommand: (lid: number, cmd: number) => { calls.push(`syscmd ${lid} ${cmd.toString(16)}`); return 1; } } : {}),
    };
    const dropped: string[] = [];
    const d = new IdleDriver(engine, {
      deliver: (code) => { delivered.push(code); const w = waiting; waiting = false; return w; },
      now: () => 1000, setTimer: () => 0, clearTimer: () => undefined,
    }, { dropped: (_s, r) => { dropped.push(r); } });
    const idle = (n: number): void => { waiting = true; d.onIdle(n, n * 10000, 0); };
    return { d, calls, delivered, dropped, idle };
  }
  const press: RawInput = { k: 'syscmd', kind: IN.SYSCMD, lid: 6, cmd: SC.RESTORE };
  it('is given to the engine at its idle, after the clock was moved, and the engine is resumed', () => {
    const r = rig(true);
    r.idle(1);
    expect(r.d.submit(press, 1, 0)).toBe('applied');
    expect(r.calls).toContain('syscmd 6 f120');
    expect(r.delivered).toEqual([0]);
  });
  it('is queued while the engine is busy and given at the next idle', () => {
    const r = rig(true);
    r.idle(1);
    r.d.submit({ k: 'mouse', kind: IN.MOVE, x: 1, y: 1, data: 0 }, 1, 0);   // the engine is resumed and busy now
    expect(r.d.submit(press, 2, 0)).toBe('queued');
    expect(r.calls.filter((c) => c.startsWith('syscmd'))).toEqual([]);
    r.idle(2);
    expect(r.calls.filter((c) => c.startsWith('syscmd'))).toEqual(['syscmd 6 f120']);
  });
  it('is dropped before the first idle, like every input', () => {
    const r = rig(true);
    expect(r.d.submit(press, 1, 0)).toBe('dropped');
    expect(r.calls).toEqual([]);
  });
  it('a build without the export refuses it (the engine did not take it) and the engine is not resumed for it', () => {
    const r = rig(false);
    r.idle(1);
    r.d.submit(press, 1, 0);
    expect(r.delivered).toEqual([]);
    expect(r.calls.filter((c) => c.startsWith('syscmd'))).toEqual([]);
    expect(r.dropped.length).toBe(1);
  });
});

describe('the markup (index.html, style.css)', () => {
  const APP = path.resolve(import.meta.dirname, '../..');
  const html = fs.readFileSync(path.join(APP, 'index.html'), 'utf8');
  const css = fs.readFileSync(path.join(APP, 'src/style.css'), 'utf8');
  it('the restore control is a <button type="button"> (reachable by Tab, Enter and Space), hidden until the engine reports a window, in a status region that announces it', () => {
    const m = /<div id="taskbar"([^>]*)>\s*<button id="taskbar-restore"([^>]*)>([^<]+)<\/button>\s*<\/div>/.exec(html);
    expect(m, 'the #taskbar region with the #taskbar-restore button').not.toBeNull();
    expect(m?.[1]).toContain('role="status"');
    expect(m?.[2]).toContain('type="button"');
    expect(m?.[2]).toMatch(/\bhidden\b/);
    expect((m?.[3] ?? '').trim().length).toBeGreaterThan(0);
  });
  it('the button floats over the desktop and takes no room: it cannot change the room the screen is fitted into (no feedback into the guest\'s display size)', () => {
    expect(css).toMatch(/body\.screen-fit #taskbar \{[^}]*position: absolute;/);
    // no rule gives the button a display value: [hidden] keeps working
    expect(/#taskbar-restore[^{]*\{[^}]*display\s*:/.test(css)).toBe(false);
  });
});
