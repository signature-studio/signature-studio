import fs from 'node:fs';
import path from 'node:path';
import { describe, expect, it } from 'vitest';
import { APPVIEW_AT, flScene, SyntheticEngine } from '../helpers/synthetic-engine.ts';

// The measurement workers of lift/harness/web (perf1, perf2, final0129) are classic scripts that cannot import anything, so each holds a COPY of mainLidOf (decision 0150). This reads the copies out of the files,
// checks that they are the same text, and runs one over a synthetic engine memory.
const REPO = process.env.APP_REPO_ROOT || path.resolve(import.meta.dirname, '../../..');
const FILES = ['lift/harness/web/perf1/fle-perf-worker.js', 'lift/harness/web/perf2/fle-perf2-worker.js', 'lift/harness/web/final0129/fle-final-worker.js'];
const extract = (f: string): string => {
  const text = fs.readFileSync(path.join(REPO, f), 'utf8');
  const i = text.indexOf('// ---- main-window lookup (decision 0150) ----'), j = text.indexOf('// ---- end of main-window lookup ----');
  expect(i, f).toBeGreaterThanOrEqual(0);
  expect(j, f).toBeGreaterThan(i);
  return text.slice(i, j);
};
const load = (src: string) => new Function(`${src}; return mainLidOf;`)() as (instance: unknown, memory: unknown) => number | null;

describe('mainLidOf in the measurement workers (lift/harness/web)', () => {
  it('the three copies are the same text', () => {
    const [a, b, c] = FILES.map(extract);
    expect(b).toBe(a);
    expect(c).toBe(a);
  });

  it('the workers use it for the events, the frames and the input, and no longer compare a surface event with a window number', () => {
    for (const f of FILES) {
      const t = fs.readFileSync(path.join(REPO, f), 'utf8');
      expect(t, f).toContain('mainLidOf(instance, memory)');
      expect(t, f).toContain("m.lid === 'main' ? mainLidOf(");
      expect(t.split('\n').filter((l) => /\blid\s*[=!]==?\s*13\b/.test(l)), f).toEqual([]);
    }
  });

  for (const lids of [{ main: 13, wizard: 48, logo: 18, app: 5 }, { main: 14, wizard: 49, logo: 8, app: 5 }, { main: 7777, wizard: 3, logo: 90000, app: 12 }]) {
    it(`with fle_av_poll: the lid of the window of class TFruityLoopsMainForm (here ${lids.main})`, () => {
      const e = new SyntheticEngine({ windows: flScene(lids) });
      const mainLidOf = load(extract(FILES[0] as string));
      expect(mainLidOf({ exports: { fle_av_poll: () => APPVIEW_AT } }, e.memory)).toBe(lids.main);
    });
  }

  it('without the export (the wasm builds of decisions 0128-0138): 13, the number those builds gave the main form', () => {
    const mainLidOf = load(extract(FILES[0] as string));
    const e = new SyntheticEngine({ windows: flScene({ main: 14, wizard: 49, logo: 8, app: 5 }), appView: false });
    expect(mainLidOf({ exports: {} }, e.memory)).toBe(13);
    expect(mainLidOf(null, null)).toBe(13);
  });

  it('with the export but no window of that class, or a structure that is not the window list: null (not a guess)', () => {
    const mainLidOf = load(extract(FILES[0] as string));
    const none = new SyntheticEngine({ windows: flScene({ main: 14, wizard: 49, logo: 8, app: 5 }).filter((w) => w.cls !== 'TFruityLoopsMainForm') });
    expect(mainLidOf({ exports: { fle_av_poll: () => APPVIEW_AT } }, none.memory)).toBeNull();
    const bad = new SyntheticEngine({ windows: flScene({ main: 14, wizard: 49, logo: 8, app: 5 }) });
    new DataView(bad.memory.buffer).setUint32(APPVIEW_AT, 0, true);
    expect(mainLidOf({ exports: { fle_av_poll: () => APPVIEW_AT } }, bad.memory)).toBeNull();
  });

  it('a class name that only starts like the main form\'s is not it', () => {
    const mainLidOf = load(extract(FILES[0] as string));
    const e = new SyntheticEngine({ windows: [{ lid: 5, cls: 'TFruityLoopsMainForm2', w: 10, h: 10 }, { lid: 6, cls: 'TFruityLoopsMainFor', w: 10, h: 10 }] });
    expect(mainLidOf({ exports: { fle_av_poll: () => APPVIEW_AT } }, e.memory)).toBeNull();
  });
});
