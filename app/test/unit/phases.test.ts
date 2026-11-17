import { describe, expect, it } from 'vitest';
import { stageRows, Timeline, transferSpeed } from '../../src/ui/phases.ts';

const tl = (m: Record<string, number>): Timeline => { const t = new Timeline(); for (const [k, v] of Object.entries(m)) t.mark(k, v); return t; };
const st = (t: Timeline, ctx: Parameters<typeof stageRows>[1]) => stageRows(t, ctx).map((r) => `${r.id}:${r.status}`);

describe('start-up stages', () => {
  it('is all pending before anything happened', () => {
    expect(st(tl({}), { now: 100 })).toEqual(['download:pending', 'compile:pending', 'engine:pending', 'first-pixel:pending', 'end:pending']);
  });

  it('download and compile are active while bytes arrive', () => {
    expect(st(tl({ download_start: 500 }), { now: 800 })).toEqual(['download:active', 'compile:active', 'engine:pending', 'first-pixel:pending', 'end:pending']);
  });

  it('the compile tail: the download is done, the compile is not', () => {
    const rows = stageRows(tl({ download_start: 500, download_done: 9000 }), { now: 9500 });
    expect(rows.map((r) => r.status)).toEqual(['done', 'active', 'pending', 'pending', 'pending']);
    expect(rows[0]!.detail).toContain('8.5 s');
  });

  it('after the compile the engine row runs with an elapsed time and the bundle\'s hint', () => {
    const t = tl({ download_start: 500, download_done: 9000, compile_done: 9400, worker_ready: 9600, engine_start: 9700, main_start: 11000 });
    const rows = stageRows(t, { now: 20700, hintFirstPixelS: 49 });
    expect(rows.map((r) => r.status)).toEqual(['done', 'done', 'active', 'active', 'pending']);
    expect(rows[2]!.detail).toContain('running 11.0 s');
    expect(rows[2]!.detail).toContain('first pixel about 49 s');
    expect(rows[2]!.detail).toContain('main() began at 11.0 s');
  });

  it('the first pixel row is done at first_pixel and the hint disappears', () => {
    const t = tl({ download_start: 500, download_done: 9000, compile_done: 9400, engine_start: 9700, main_start: 11000, first_pixel: 60000 });
    const rows = stageRows(t, { now: 61000, hintFirstPixelS: 49 });
    expect(rows[3]).toMatchObject({ status: 'done' });
    expect(rows[3]!.detail).toContain('50.3 s after the engine started');
    expect(rows[2]!.detail).not.toContain('expects');
  });

  it('a stop ends the engine row and fills the last row', () => {
    const t = tl({ download_start: 500, download_done: 9000, compile_done: 9400, engine_start: 9700, main_start: 11000, first_pixel: 60000, exit: 64000 });
    const rows = stageRows(t, { now: 70000, endKind: 'stopped' });
    expect(rows.map((r) => r.status)).toEqual(['done', 'done', 'done', 'done', 'done']);
    expect(rows[4]!.label).toBe('Stopped');
    expect(rows[2]!.detail).toContain('ran 54.3 s');
  });

  it('an idle engine is "Idle", not stopped', () => {
    const rows = stageRows(tl({ engine_start: 1, idle: 5000 }), { now: 6000, endKind: 'idle' });
    expect(rows[4]).toMatchObject({ label: 'Idle', status: 'done' });
  });

  it('a crash marks the last row failed', () => {
    const rows = stageRows(tl({ engine_start: 1, exit: 5000 }), { now: 6000, endKind: 'trap' });
    expect(rows[4]!.status).toBe('failed');
  });

  it('an error marks the stage that was running as failed and nothing else is active', () => {
    const rows = stageRows(tl({ download_start: 500 }), { now: 900, failedStage: 'download' });
    expect(rows.map((r) => r.status)).toEqual(['failed', 'pending', 'pending', 'pending', 'pending']);
  });

  it('the first report of a mark wins', () => {
    const t = new Timeline();
    t.mark('first_pixel', 10);
    t.mark('first_pixel', 99);
    expect(t.get('first_pixel')).toBe(10);
    expect(t.toJSON()).toEqual({ first_pixel: 10 });
  });
});

describe('transferSpeed', () => {
  it('is MiB per second between the first and the latest report', () => {
    expect(transferSpeed({ loaded: 0, at: 1000 }, { loaded: 10 * 1048576, at: 3000 })).toBeCloseTo(5, 6);
  });
  it('is null when nothing has moved yet', () => {
    expect(transferSpeed(undefined, { loaded: 5, at: 5 })).toBeNull();
    expect(transferSpeed({ loaded: 5, at: 5 }, { loaded: 5, at: 9 })).toBeNull();
    expect(transferSpeed({ loaded: 5, at: 9 }, { loaded: 7, at: 9 })).toBeNull();
  });
});
