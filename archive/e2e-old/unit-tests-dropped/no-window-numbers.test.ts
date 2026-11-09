import fs from 'node:fs';
import path from 'node:path';
import { describe, expect, it } from 'vitest';

// Decision 0150: the app, the bundle tools and the real-engine tests find the main window (and the wizard, the logo, Settings) by what they are, never by the number a window happens to have in one build.
// This reads the SOURCE and refuses a number written where a window is named (a text check: it cannot see a number computed at run time, the unit tests of main-window.ts do that).
const APP = path.resolve(import.meta.dirname, '../..');
const read = (f: string): string => fs.readFileSync(path.join(APP, f), 'utf8');
const walk = (d: string): string[] => fs.readdirSync(path.join(APP, d), { withFileTypes: true }).flatMap((e) => (e.isDirectory() ? walk(path.join(d, e.name)) : [path.join(d, e.name)]));
const lines = (text: string): { n: number; l: string }[] => text.split('\n').map((l, i) => ({ n: i + 1, l })).filter((x) => !/^\s*(\/\/|\*|\/\*)/.test(x.l));

/** a window named by a number: `lid === 13`, `lid: 13`, `.lid == 48`, `data-lid="18"`, `lids = [13, 48]` */
const NUMBERED = [
  /\blid\s*[=!]==?\s*\d/, /\b\d+\s*[=!]==?\s*\w*\.?lid\b/, /\blid\s*:\s*\d/, /data-lid=\\?"\d/, /\blid\s*\)\s*===\s*\d/, /\blids?\s*(=|:)\s*\[\s*\d/,
];
/** the manifest's field of the old kind: only manifest.ts may name it */
const MAIN_LID = /\bmainLid\b|\bmain-lid\b/;

describe('no window number is written where a window is named', () => {
  const src = walk('src').filter((f) => f.endsWith('.ts'));
  it('the sources of the app (src/**/*.ts) name no window by a number; no source mentions mainLid', () => {
    const hits: string[] = [];
    for (const f of src) for (const { n, l } of lines(read(f))) if (NUMBERED.some((re) => re.test(l)) || MAIN_LID.test(l)) hits.push(`${f}:${n}: ${l.trim().slice(0, 140)}`);
    expect(hits).toEqual([]);
  });

  it('manifest.ts has no mainLid and no expect block (decision 0243: the page names no window)', () => {
    const m = read('src/bundle/manifest.ts');
    expect(m).not.toMatch(MAIN_LID);
    expect(m).not.toMatch(/\bexpect\b/);
  });

  it('pack-assets.mjs writes no mainLid and has no default window number for it', () => {
    const p = lines(read('scripts/pack-assets.mjs')).filter((x) => !/console\.error\('pack-assets: --main-lid/.test(x.l) && !/args\['main-lid'\] !== undefined/.test(x.l));
    expect(p.filter((x) => MAIN_LID.test(x.l))).toEqual([]);
  });

  it('build-engine-bundle.sh passes no --main-lid and no --expect (the page names no window)', () => {
    const b = lines(read('scripts/build-engine-bundle.sh'));
    expect(b.filter((x) => /--main-lid/.test(x.l))).toEqual([]);
    expect(b.filter((x) => /--expect\b/.test(x.l))).toEqual([]);
  });

  it('the real-engine e2e scripts name no window by a number (desk2: they read the compositor\'s list, the foreground title and the picture; the page has no window list and the helpers no class table)', () => {
    for (const f of ['test/e2e/engine-desk.mjs', 'test/e2e/desk-helpers.mjs', 'test/e2e/input-helpers.mjs', 'test/e2e/files-engine-dialog.mjs']) {
      const hits = lines(read(f)).filter((x) => NUMBERED.some((re) => re.test(x.l)) || /\.lid === \d/.test(x.l) || /for lid \d+ /.test(x.l));
      expect(hits.map((h) => `${f}:${h.n}: ${h.l.trim().slice(0, 120)}`), f).toEqual([]);
    }
    const h = read('test/e2e/input-helpers.mjs');
    for (const gone of ['CLASSES', 'windowOfClass', 'lidOfClass', 'canvasSha', 'waitIdleWithWindows', 'appView']) expect(h, gone).not.toContain(gone);
  });
});

describe('the guard itself', () => {
  it('sees the patterns it is meant to refuse', () => {
    for (const bad of ['if (ev.lid === 13) {', 'x.lid !== 14', 'lid: 13,', "document.querySelector('#desktop canvas[data-lid=\"18\"]')", "data-lid=\"13\"", 'const lids = [13, 48];', '13 === s.lid']) {
      expect(NUMBERED.some((re) => re.test(bad)), bad).toBe(true);
    }
    expect(MAIN_LID.test('const lid = fruityOptions.mainLid;')).toBe(true);
    for (const fine of ['if (ev.lid === mainLid)', 'lid: e.lid,', 'data-lid="${logoLid}"', 'const lid = 0;']) {
      expect(NUMBERED.some((re) => re.test(fine)), fine).toBe(false);
    }
  });
});
