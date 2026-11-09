import { describe, expect, it } from 'vitest';
import { matchStopNotes, summarizeEngineLog } from '../../src/engine/fruity-log.ts';

const SHA = 'b12c8eceb4e86199fe70ccd1d64a8c2e68e23a3e78999616b9832e836bee51db';
// lines in the formats of a real stop (decision 0134's Chrome runs); the titles and numbers are shortened
const STOP = [
  'image: base=0x400000 size=0x18c0000 entry=0x1376820 imports=738 exports=3',
  `wf1-surface: event=first-non-background-pixel lid=13 size=1930x1092 stride=7720 pixels=0x339a0008 nonzero=201380 rgba_sha256=${'4'.repeat(64)} entries_since_program_start=0`,
  '[err] fle: RaiseException: exception code 0xeedfade, flags 0x1, 8 argument(s) raised; no structured exception handling is modelled, stopped here',
  `wf1-surface: event=stop lid=13 size=1930x1092 stride=7720 pixels=0x339a0008 nonzero=1917816 rgba_sha256=${SHA} entries_since_program_start=0`,
  'fruity: == stop: SIGABRT (runtime abort, see runtime message)',
  'fruity: runtime message: fle: RaiseException: exception code 0xeedfade, flags 0x1, 8 argument(s) raised; no structured exception handling is modelled, stopped here',
  '[err]   window table: 2 entries, 2 live; top-level windows in z-order, top first, each followed by its children (z = position in its group, top first):',
  '[err]     handle 0x7f990034 lid 13 class "TFLMain" title "FL Studio" rect (-5, -6, 1930 x 1092) style 0x94030000 exstyle 0x50000 owner 0 visible 1 enabled 1 z 0 thread 0',
  '[err]     handle 0x7f9900c8 lid 50 class "TQuickControlBar" title "" rect (5, 6, 1920 x 82) style 0x56000000 exstyle 0x10000 parent 0x7f990034 visible 1 enabled 1 z 5 thread 0',
  '[err]     handle 0x7f9900b0 lid 44 class "TQuickTabSelector" title "" rect (0, 0, 196 x 24) style 0x46000000 exstyle 0 parent 0x7f9900ac visible 0 enabled 1 z 0 thread 0',
  '[err]     handle 0x7f9900d8 lid 54 class "TPluginForm" title "Plug-in \\"x\\"" rect (34, 118, 568 x 466) style 0x84010000 exstyle 0x10000 owner 0x7f990034 visible 0 enabled 1 z 10 thread 0',
  'threads: 3 (running index 0), virtual clock 36000088530 ticks (3600008 ms)',
  '  thread 0: id 0x1001 handle 0 running start 0 param 0 exit 0 suspend 0 rsp 0x1399bec8; queue 0 posted 0 sent',
  '  thread 1: id 0x1002 handle 0x7ffaa0000008 done (handle closed) start 0x1d0fdb0 param 0x13c01130 exit 0 suspend 0 rsp 0x1410ffc0; queue none 0 posted 0 sent',
  '  thread 2: id 0x1003 handle 0x7ffaa000000c waiting start 0x412ae0 param 0x13ab2dd0 exit 0 suspend 0 rsp 0x1420fe38; queue 0 posted 0 sent',
  'fruity: exit code 134 (the codes of harness/wasm/trace_native.c: 134 SIGABRT)',
];

describe('summarizeEngineLog', () => {
  const s = summarizeEngineLog(STOP);

  it('finds the stop line, the engine\'s own words and the exit code', () => {
    expect(s.stopLine).toBe('== stop: SIGABRT (runtime abort, see runtime message)');
    expect(s.runtimeMessage).toContain('RaiseException: exception code 0xeedfade');
    expect(s.runtimeMessage).toContain('stopped here');
    expect(s.exitCode).toBe(134);
  });

  it('reads the surface reports with their SHA-256', () => {
    expect(s.surfaces.map((x) => x.event)).toEqual(['first-non-background-pixel', 'stop']);
    expect(s.surfaces[1]).toMatchObject({ lid: 13, width: 1930, height: 1092, stride: 7720, nonzero: 1917816, rgbaSha256: SHA });
  });

  it('reads the window table: lid, class, title, position, size, visibility, relation', () => {
    expect(s.windows).toHaveLength(4);
    expect(s.windows[0]).toMatchObject({ lid: 13, className: 'TFLMain', title: 'FL Studio', x: -5, y: -6, width: 1930, height: 1092, visible: true, relation: 'owner 0' });
    expect(s.windows[1]).toMatchObject({ lid: 50, className: 'TQuickControlBar', title: '', x: 5, y: 6, width: 1920, height: 82, relation: 'parent 0x7f990034' });
    expect(s.windows[2]).toMatchObject({ lid: 44, className: 'TQuickTabSelector', width: 196, height: 24, visible: false, relation: 'parent 0x7f9900ac' }); // "exstyle 0" has no 0x
    expect(s.windows[3]).toMatchObject({ lid: 54, visible: false, relation: 'owner 0x7f990034' });
  });

  it('reads the thread table', () => {
    expect(s.threadSummary).toContain('threads: 3');
    expect(s.threads.map((t) => [t.index, t.state])).toEqual([[0, 'running'], [1, 'done (handle closed)'], [2, 'waiting']]);
  });

  it('uses the LAST table when the engine printed the table several times', () => {
    const two = [
      'window table: 1 entries, 1 live', '    handle 0x1 lid 1 class "A" title "" rect (0, 0, 1 x 1) style 0x0 exstyle 0x0 owner 0 visible 1 enabled 1 z 0 thread 0',
      'window table: 2 entries, 2 live', '    handle 0x2 lid 2 class "B" title "" rect (0, 0, 2 x 2) style 0x0 exstyle 0x0 owner 0 visible 1 enabled 1 z 0 thread 0',
      '    handle 0x3 lid 3 class "C" title "" rect (0, 0, 3 x 3) style 0x0 exstyle 0x0 owner 0 visible 1 enabled 1 z 1 thread 0',
    ];
    expect(summarizeEngineLog(two).windows.map((w) => w.lid)).toEqual([2, 3]);
  });

  it('ignores lines it does not know and survives an empty log', () => {
    expect(summarizeEngineLog(['hello', '', 'handle but not a table row'])).toMatchObject({ windows: [], threads: [], surfaces: [], exitCode: null, runtimeMessage: '' });
    expect(summarizeEngineLog([])).toMatchObject({ stopLine: '' });
  });
});

describe('matchStopNotes', () => {
  const notes = [{ contains: 'RaiseException: exception code 0xeedfade', note: 'wall 16' }, { contains: 'something else', note: 'other' }];
  it('returns the notes whose text is in the message', () => {
    expect(matchStopNotes('fle: RaiseException: exception code 0xeedfade, flags 0x1', notes)).toEqual(['wall 16']);
  });
  it('returns nothing for another stop, for no message and for no notes', () => {
    expect(matchStopNotes('fle: indirect call to 0x33d6e70', notes)).toEqual([]);
    expect(matchStopNotes('', notes)).toEqual([]);
    expect(matchStopNotes('x', undefined)).toEqual([]);
  });
});
