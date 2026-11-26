// The coverage (coverage.mjs): the TSV reads, the minimums, the rule of a covered feature, and the duplicate IDs (the features of a directory, in a temporary directory of the repository).
import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { computeCoverage, parseMin, parseTsv, readFeatures } from '../coverage.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, '..', '..', '..');

test('parseTsv reads the header and skips # lines and empty lines', () => {
  const rows = parseTsv('# a comment\nid\tkind\tname\nmenus/menu/file\tmenu\tFile\n\n');
  assert.deepEqual(rows, [{ id: 'menus/menu/file', kind: 'menu', name: 'File' }]);
});

test('parseMin needs the two lines', () => {
  assert.deepEqual(parseMin('scenarios\t80\ncoverage\t0.9\n'), { scenarios: 80, coverage: 0.9 });
  assert.throws(() => parseMin('coverage\t0.9\n'), /scenarios and coverage/);
});

test('a feature is covered by a passing scenario that is not marked; a marked or failed or skipped one does not count', () => {
  const features = [{ id: 'page/module/a', area: 'page' }, { id: 'page/module/b', area: 'page' }, { id: 'menus/menu/file', area: 'menus' }];
  const rows = [{ scenarios: [
    { id: 'page/x', area: 'page', ok: true, expectFail: null, covers: ['page/module/a'] },
    { id: 'page/y', area: 'page', ok: true, expectFail: '#1', covers: ['page/module/b'] },
    { id: 'menus/z', area: 'menus', ok: false, expectFail: null, covers: ['menus/menu/file'] },
    { id: 'menus/w', area: 'menus', ok: true, expectFail: null, skipped: true, covers: ['menus/menu/file'] },
  ] }];
  const c = computeCoverage({ features, rows, mins: { page: { scenarios: 1, coverage: 0.5 } } });
  assert.equal(c.total, 3);
  assert.equal(c.covered, 1);
  assert.equal(c.perArea.page.features, 2);
  assert.equal(c.perArea.page.covered, 1);
  assert.equal(c.perArea.page.ratio, 0.5);
  assert.equal(c.perArea.page.scenarios, 2);
  assert.deepEqual(c.perArea.page.min, { scenarios: 1, coverage: 0.5 });
  assert.equal(c.perArea.menus.covered, 0);
  assert.equal(c.perArea.menus.min, null);
});

test('a cover that is not a known feature is counted apart', () => {
  const c = computeCoverage({ features: [{ id: 'page/module/a', area: 'page' }], rows: [{ scenarios: [{ id: 'p', area: 'page', ok: true, expectFail: null, covers: ['page/module/zzz'] }] }] });
  assert.equal(c.unknownCovers, 1);
  assert.equal(c.covered, 0);
});

test('readFeatures: the IDs of the directory and of generated/, and a duplicate ID is an error', () => {
  const dir = fs.mkdtempSync(path.join(repo, 'tools', 'e2e', '.tmp-test-cov-'));
  try {
    fs.mkdirSync(path.join(dir, 'generated'));
    fs.writeFileSync(path.join(dir, 'page.tsv'), 'id\tkind\npage/module/a\tmodule\n');
    fs.writeFileSync(path.join(dir, 'generated', 'page.tsv'), 'id\tkind\npage/module/b\tmodule\n');
    assert.deepEqual(readFeatures(dir).map((f) => f.id), ['page/module/a', 'page/module/b']);
    fs.writeFileSync(path.join(dir, 'generated', 'page.tsv'), 'id\tkind\npage/module/a\tmodule\n');
    assert.throws(() => readFeatures(dir), /duplicate feature id page\/module\/a/);
    fs.writeFileSync(path.join(dir, 'generated', 'page.tsv'), 'id\tkind\nnothing/x/y\tx\n');
    assert.throws(() => readFeatures(dir), /not of a known area/);
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
});

test('the features of the repository have no duplicate ID (the hand-written and the generated ones)', () => {
  const features = readFeatures(path.join(repo, 'tools', 'e2e', 'features'));
  assert.equal(new Set(features.map((f) => f.id)).size, features.length);
});
