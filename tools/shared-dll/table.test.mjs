// Tests of tools/shared-dll/table.mjs (Issue #1511): the guest spelling of a request, and the table's stoppedAfter rule. Run: node --test tools/shared-dll/table.test.mjs
import test from 'node:test';
import assert from 'node:assert/strict';
import { guestOf, installerDlls, tableRows } from './table.mjs';

const LIST = installerDlls([
  'guest\tbytes\tsha256\tkind\timports',
  '$0\t\t\tother\t',
  'Shared\\Reverb_x64.dll\t492544\tab\tdll\tkernel32.dll',
  'Shared\\ffmpeg\\x64\\ILVideo_x64.dll\t1329152\tcd\tdll\tavutil-56.dll',
  'Shared\\Blend2D.dll\t2065920\t66\tdll\tkernel32.dll',
  'Plugins\\Fruity\\Effects\\Balance\\Fruity Balance.dll\t1\tef\tdll\tkernel32.dll',
  'FLEngine_x64.dll\t22227016\t01\tdll\tkernel32.dll',
].join('\n'));

test('a request maps to the installer spelling of its file', () => {
  assert.equal(guestOf('C:\\Program Files\\Image-Line\\FL Studio 21\\Shared\\Reverb_x64.dll', LIST), 'Shared\\Reverb_x64.dll');
  assert.equal(guestOf('Reverb_x64.dll', LIST), 'Shared\\Reverb_x64.dll');
  assert.equal(guestOf('ffmpeg\\x64\\ilvideo_x64.dll', LIST), 'Shared\\ffmpeg\\x64\\ILVideo_x64.dll');
  assert.equal(guestOf('C:\\Program Files\\Image-Line\\FL Studio 21\\FLEngine_x64.dll', LIST), 'FLEngine_x64.dll');
});

test('a name the installer does not list is not a row', () => {
  assert.equal(guestOf('imm32.dll', LIST), null);
  assert.equal(guestOf('C:\\Program Files\\Image-Line\\FL Studio 21\\Plugins\\Fruity\\Effects\\Balance\\Fruity Balance.dll', LIST), null);
});

test('stoppedAfter is true only in the operation where the engine stopped', () => {
  const line = (o) => JSON.stringify(o);
  const text = [
    line({ index: 0, session: 1, group: 'tools', step: 'tools-3', missing: [{ api: 'LoadLibraryW', code: 126, path: 'Reverb_x64.dll' }] }),
    line({ index: 1, session: 1, group: 'tools', step: 'tools-4', state: 'exited', missing: [{ api: 'GetFileAttributesW', code: 2, path: 'C:\\Program Files\\Image-Line\\FL Studio 21\\Shared\\Blend2D.dll' }] }),
    line({ group: 'tools', session: 1, sessionEnd: true, stoppedAt: 1 }),
    line({ index: 2, session: 2, group: 'tools', step: 'tools-5', missing: [{ api: 'GetFileAttributesW', code: 2, path: 'Shared\\Blend2D.dll' }] }),
    line({ group: 'tools', session: 2, sessionEnd: true, stoppedAt: null }),
  ].join('\n');
  const rows = tableRows(text, LIST);
  assert.deepEqual(rows.map((r) => [r.dll, r.trigger, r.count, r.stoppedAfter]), [
    ['Reverb_x64.dll', 'tools/tools-3', 1, false],
    ['Blend2D.dll', 'tools/tools-4', 2, true],
  ]);
});
