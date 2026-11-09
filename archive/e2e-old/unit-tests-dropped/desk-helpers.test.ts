import { describe, expect, it } from 'vitest';
// @ts-expect-error a plain .mjs test helper
import { CLS, classify, dialogsOf, isKind, isWizard, mainOf, parseDeskWindows } from '../e2e/desk-helpers.mjs';

// Decision 0223 / desk2: the real-engine scripts read the engine's windows from the compositor's text. These cases are the text of D8 (no class, no title) and the text of an engine built with desk2's fle_desk.c (class and title).
const D8_TEXT = 'lid 14 at (-5, -6) 1930x1092 alpha 0 key 000000\nlid 144 at (4, 36) 206x59 alpha 0 key 000000\n';
const NAMED_TEXT = [
  'lid 14 at (-5, -6) 1930x1092 alpha 0 key 000000 class "TFruityLoopsMainForm" title "Sampler.flp - FL Studio 2026"',
  'lid 144 at (4, 36) 206x59 alpha 0 key 000000 class "TQuickPopupMenuWindow" title ""',
  'lid 150 at (743, 224) 432x631 alpha 0 key 000000 class "TWAVRenderForm" title "Rendering to Sampler.wav"',
  'lid 151 at (600, 400) 420x180 alpha 0 key 000000 class "TMsgForm" title "Warning"',
  '',
].join('\n');

describe('parseDeskWindows', () => {
  it('reads the lines of a bundle without class and title (D8)', () => {
    const ws = parseDeskWindows(D8_TEXT);
    expect(ws.map((w: any) => [w.lid, w.x, w.y, w.w, w.h])).toEqual([[14, -5, -6, 1930, 1092], [144, 4, 36, 206, 59]]);
    expect(ws.every((w: any) => w.cls === undefined && w.title === undefined)).toBe(true);
  });
  it('reads class and title when the line has them', () => {
    const ws = parseDeskWindows(NAMED_TEXT);
    expect(ws.map((w: any) => [w.lid, w.cls, w.title])).toEqual([[14, 'TFruityLoopsMainForm', 'Sampler.flp - FL Studio 2026'], [144, 'TQuickPopupMenuWindow', ''], [150, 'TWAVRenderForm', 'Rendering to Sampler.wav'], [151, 'TMsgForm', 'Warning']]);
  });
});

describe('classify: with class and title (named)', () => {
  const ws = classify(parseDeskWindows(NAMED_TEXT), 'Warning');
  it('the kind is the class and the title the window\'s own', () => {
    expect(ws.map((w: any) => w.kind)).toEqual(['main', 'popup', 'render', 'msg']);
    expect(ws.every((w: any) => w.named)).toBe(true);
    expect(ws[2].title).toBe('Rendering to Sampler.wav');
  });
  it('isKind is exact: a render window is not a message box', () => {
    expect(isKind(ws[2], 'render')).toBe(true);
    expect(isKind(ws[2], 'msg')).toBe(false);
    expect(isKind(ws[3], 'msg')).toBe(true);
    expect(isKind(ws[1], 'problems')).toBe(false);
  });
  it('dialogsOf leaves out the main window, the wizard and the popup', () => {
    expect(dialogsOf(ws).map((w: any) => w.lid)).toEqual([150, 151]);
  });
  it('an unknown class is "other"', () => {
    const o = classify(parseDeskWindows('lid 9 at (1, 2) 30x40 alpha 0 key 000000 class "TSomething" title "x"\n'), '');
    expect(o[0].kind).toBe('other');
  });
});

describe('classify: without class and title (D8, the weaker way)', () => {
  const MAIN = 'lid 14 at (-5, -6) 1930x1092 alpha 0 key 000000\n';
  it('a window narrower than 300 is a popup, whatever the foreground title says', () => {
    const ws = classify(parseDeskWindows(D8_TEXT), 'Sampler.flp - FL Studio 2026');
    expect(ws.map((w: any) => w.kind)).toEqual(['main', 'popup']);
    expect(ws.some((w: any) => w.named)).toBe(false);
    expect(dialogsOf(ws)).toEqual([]);
    expect(classify(parseDeskWindows(D8_TEXT), 'Rendering to Sampler.wav').map((w: any) => w.kind)).toEqual(['main', 'popup']);   // the title came before the window: nothing to click yet
  });
  it('a window at least 300 x 100 is a dialog, and the foreground title is the topmost dialog\'s, not a stale popup\'s drawn after it', () => {
    const ws = classify(parseDeskWindows(MAIN + 'lid 150 at (743, 224) 433x632 alpha 0 key 000000\nlid 144 at (4, 36) 206x59 alpha 0 key 000000\n'), 'Rendering to Sampler.wav');
    expect(ws.map((w: any) => w.kind)).toEqual(['main', 'dialog', 'popup']);
    expect(ws.map((w: any) => w.title)).toEqual(['', 'Rendering to Sampler.wav', '']);
    expect(isKind(ws[1], 'render') && isKind(ws[1], 'msg') && isKind(ws[1], 'problems')).toBe(true);   // the class is not known: any dialog stands for them
    expect(isKind(ws[2], 'render')).toBe(false);
    expect(dialogsOf(ws).map((w: any) => w.lid)).toEqual([150]);
  });
  it('the 206 x 104 "Buy FL Studio" window is a dialog only when it is drawn last and the foreground title says so', () => {
    const t = MAIN + 'lid 160 at (700, 400) 206x104 alpha 0 key 000000\n';
    expect(classify(parseDeskWindows(t), 'Buy FL Studio').map((w: any) => w.kind)).toEqual(['main', 'dialog']);
    expect(classify(parseDeskWindows(t), 'Buy FL Studio')[1].title).toBe('Buy FL Studio');
    expect(classify(parseDeskWindows(t), 'FL Studio 2026').map((w: any) => w.kind)).toEqual(['main', 'popup']);
  });
  it('the wizard is the 874-wide window', () => {
    const ws = classify(parseDeskWindows(MAIN + 'lid 48 at (523, 253) 874x573 alpha 0 key 000000\n'), 'Welcome to FL Studio - in ?trial mode?');
    expect(ws.map((w: any) => w.kind)).toEqual(['main', 'wizard']);
  });
});

describe('small helpers', () => {
  it('mainOf is the largest, isWizard the 874-wide one, CLS names the classes the scripts use', () => {
    const ws = parseDeskWindows(D8_TEXT + 'lid 48 at (523, 253) 874x573 alpha 0 key 000000\n');
    expect(mainOf(ws).lid).toBe(14);
    expect(ws.filter(isWizard).map((w: any) => w.lid)).toEqual([48]);
    expect(Object.values(CLS)).toContain('TWAVRenderForm');
  });
});
