// The DPR declarations of the page scenarios crisp-* and zoom-* (lib/plan.mjs fixedDpr): crisp-dpr-N is declared at DPR N, the zoom scenarios at the base DPR 1
// (a zoom changes the ratio and puts it back). A page at another DPR fails its scenario (run.mjs checkDpr, dprMatches), so the declaration is what the run checks.
import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { fixedDpr } from '../lib/plan.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
const dir = path.join(here, '..', 'scenarios', 'page');
const files = (re) => fs.readdirSync(dir).filter((f) => re.test(f)).sort();
const load = async (f) => (await import(pathToFileURL(path.join(dir, f)).href)).default;

test('page/crisp-dpr-N is declared at DPR N, and no other display', async () => {
  const list = files(/^crisp-dpr-.*\.mjs$/);
  assert.ok(list.length >= 7, 'the crisp-dpr files are there');
  for (const f of list) {
    const sc = await load(f);
    const n = Number(f.slice('crisp-dpr-'.length, -'.mjs'.length).replace('-', '.'));
    assert.deepEqual(sc.displays, [`dpr:${n}`], f);
    assert.equal(fixedDpr(sc.displays[0]), n, f);
  }
});

test('page/crisp-screen-pixels is declared at DPR 1 (the screenshot is at CSS px only there)', async () => {
  assert.deepEqual((await load('crisp-screen-pixels.mjs')).displays, ['dpr:1']);
});

test('page/zoom-N is declared at the base DPR 1 (the zoom is relative to the page\'s ratio at the start)', async () => {
  const list = files(/^zoom-\d+\.mjs$/);
  assert.ok(list.length >= 10, 'the zoom files are there');
  for (const f of list) assert.deepEqual((await load(f)).displays, ['dpr:1'], f);
});
