// A closed Issue is never a mark (Issue #1534): no file of tools/e2e/scenarios or tools/e2e/lib names one as a quoted '#<番号>' (comments included).
// The list is the closed Issues checked on 2026-11-28 (gh issue view). The test sees only the numbers listed here: a newly closed Issue must be added.
import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const CLOSED = [1315, 1319, 1336, 1337, 1338, 1382, 1400, 1406, 1409, 1411, 1415, 1416, 1419, 1421, 1429, 1484, 1499, 1502, 1503, 1504, 1505];
const MARK = new RegExp(`'#(${CLOSED.join('|')})'`, 'g');
const here = path.dirname(fileURLToPath(import.meta.url));
const e2e = path.resolve(here, '..');

/** the .mjs files under a directory, in any depth */
function mjsFiles(dir) {
  return fs.readdirSync(dir, { withFileTypes: true }).flatMap((e) => {
    const p = path.join(dir, e.name);
    if (e.isDirectory()) return mjsFiles(p);
    return e.name.endsWith('.mjs') ? [p] : [];
  });
}

/** the closed marks of a text, one line each: `'#<番号>'` with a number of the list */
function closedMarksIn(text) {
  return text.split('\n').flatMap((line, i) => [...line.matchAll(MARK)].map((m) => `${i + 1}: '#${m[1]}'`));
}

test('the closed-mark detector takes a quoted closed number and ignores an open one, a menu item and a bare number', () => {
  assert.deepEqual(closedMarksIn("  expectFail: '#1499',"), ["1: '#1499'"]);
  assert.deepEqual(closedMarksIn("expectFail: ['#1395', '#1419']"), ["1: '#1419'"]);
  assert.deepEqual(closedMarksIn("expectFail: '#1386'"), []);
  assert.deepEqual(closedMarksIn('chooseItem(ctx, ["Add","#5","#5"])'), []);
  assert.deepEqual(closedMarksIn("// #1429 is closed\nconst x = '#14290';"), []);
});

test('no file of tools/e2e/scenarios or tools/e2e/lib names a closed Issue as a mark (quoted, comments included)', () => {
  const files = [...mjsFiles(path.join(e2e, 'scenarios')), ...mjsFiles(path.join(e2e, 'lib'))];
  assert.ok(files.length > 300, `${files.length} files scanned`);
  const hits = files.flatMap((f) => closedMarksIn(fs.readFileSync(f, 'utf8')).map((h) => `${path.relative(e2e, f)}:${h}`));
  assert.deepEqual(hits, []);
});
