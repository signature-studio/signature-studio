import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { prepareBase, verifyBase } from '../tools/ab/base.mjs';

const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

function scratch(t) {
  const root = fs.mkdtempSync(path.join(repo, '.tmp-ab-base-'));
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  return root;
}
function write(p, text) {
  fs.mkdirSync(path.dirname(p), { recursive: true });
  fs.writeFileSync(p, text);
}
// the tracked tree (what git archive gives) and the working tree (which also has the git-ignored inputs and the build output)
function fixture(root) {
  const tree = path.join(root, 'tree');
  const tracked = path.join(root, 'tracked');
  write(path.join(tracked, 'app/package.json'), '{}');
  write(path.join(tracked, 'lift/runtime/fle_input.c'), 'int x;');
  write(path.join(tree, 'lift/out-app/host/binaries/win64/FLEngine_x64.dll'), 'DLL-A');
  write(path.join(tree, 'lift/out-app/host/Data/plugin.dll'), 'DLL-B');
  write(path.join(tree, 'lift/out-app/host-extra/resources/System/Internet/error-offline.html'), 'HTML');
  write(path.join(tree, 'lift/out-app/external/ott.exe'), 'EXE');
  write(path.join(tree, 'lift/out-app/blend2d-aaaa.tar.gz'), 'TAR');
  write(path.join(tree, 'lift/out-app/gen/old.c'), 'stale build output');
  return { tree, tracked };
}
const archiveFrom = (dir) => (_ref, dest) => fs.cpSync(dir, dest, { recursive: true });

test('the base has the tracked tree and the git-ignored inputs, and not the build output of the working tree', (t) => {
  const root = scratch(t);
  const { tree, tracked } = fixture(root);
  const dest = path.join(root, 'base');
  prepareBase({ tree, dest, archive: archiveFrom(tracked) });
  assert.equal(fs.readFileSync(path.join(dest, 'app/package.json'), 'utf8'), '{}');
  assert.equal(fs.readFileSync(path.join(dest, 'lift/out-app/host/binaries/win64/FLEngine_x64.dll'), 'utf8'), 'DLL-A');
  assert.equal(fs.readFileSync(path.join(dest, 'lift/out-app/host/Data/plugin.dll'), 'utf8'), 'DLL-B');
  assert.ok(fs.existsSync(path.join(dest, 'lift/out-app/host-extra/resources/System/Internet/error-offline.html')));
  assert.ok(fs.existsSync(path.join(dest, 'lift/out-app/external/ott.exe')));
  assert.ok(fs.existsSync(path.join(dest, 'lift/out-app/blend2d-aaaa.tar.gz')));
  assert.equal(fs.existsSync(path.join(dest, 'lift/out-app/gen')), false);
});

test('the inputs are hard links to the working tree (the same inode), not copies', (t) => {
  const root = scratch(t);
  const { tree, tracked } = fixture(root);
  const dest = path.join(root, 'base');
  prepareBase({ tree, dest, archive: archiveFrom(tracked) });
  const src = fs.statSync(path.join(tree, 'lift/out-app/host/binaries/win64/FLEngine_x64.dll'));
  const got = fs.statSync(path.join(dest, 'lift/out-app/host/binaries/win64/FLEngine_x64.dll'));
  assert.equal(got.ino, src.ino);
  assert.equal(got.dev, src.dev);
});

test('a link that fails falls back to a copy, and the copy is still checked', (t) => {
  const root = scratch(t);
  const { tree, tracked } = fixture(root);
  const dest = path.join(root, 'base');
  const exdev = () => { const e = new Error('cross-device'); e.code = 'EXDEV'; throw e; };
  prepareBase({ tree, dest, archive: archiveFrom(tracked), link: exdev });
  const src = fs.statSync(path.join(tree, 'lift/out-app/host/Data/plugin.dll'));
  const got = fs.statSync(path.join(dest, 'lift/out-app/host/Data/plugin.dll'));
  assert.notEqual(got.ino, src.ino);
  assert.equal(fs.readFileSync(path.join(dest, 'lift/out-app/host/Data/plugin.dll'), 'utf8'), 'DLL-B');
  assert.deepEqual(verifyBase({ tree, dest }), { ok: true, changed: [] });
});

test('the tracked tree is limited to the given paths (the app-only base of an app-only change)', (t) => {
  const root = scratch(t);
  const { tree, tracked } = fixture(root);
  const dest = path.join(root, 'base');
  let seen = null;
  prepareBase({ tree, dest, paths: ['app'], archive: (_ref, d, paths) => { seen = paths; fs.cpSync(path.join(tracked, 'app'), path.join(d, 'app'), { recursive: true }); } });
  assert.deepEqual(seen, ['app']);
  assert.equal(fs.existsSync(path.join(dest, 'app/package.json')), true);
  assert.equal(fs.existsSync(path.join(dest, 'lift/runtime/fle_input.c')), false);
});

test('a missing required input (the host extraction) stops the preparation', (t) => {
  const root = scratch(t);
  const { tree, tracked } = fixture(root);
  fs.rmSync(path.join(tree, 'lift/out-app/host'), { recursive: true, force: true });
  assert.throws(() => prepareBase({ tree, dest: path.join(root, 'base'), archive: archiveFrom(tracked) }), /lift\/out-app\/host/);
});

test('nothing changed after the preparation: verify is ok', (t) => {
  const root = scratch(t);
  const { tree, tracked } = fixture(root);
  const dest = path.join(root, 'base');
  prepareBase({ tree, dest, archive: archiveFrom(tracked) });
  assert.deepEqual(verifyBase({ tree, dest }), { ok: true, changed: [] });
});

test('a write into a linked input of the base changes the working tree, and verify reports it', (t) => {
  const root = scratch(t);
  const { tree, tracked } = fixture(root);
  const dest = path.join(root, 'base');
  prepareBase({ tree, dest, archive: archiveFrom(tracked) });
  fs.writeFileSync(path.join(dest, 'lift/out-app/host/binaries/win64/FLEngine_x64.dll'), 'REWRITTEN');
  assert.equal(fs.readFileSync(path.join(tree, 'lift/out-app/host/binaries/win64/FLEngine_x64.dll'), 'utf8'), 'REWRITTEN');
  const v = verifyBase({ tree, dest });
  assert.equal(v.ok, false);
  assert.deepEqual(v.changed, ['lift/out-app/host/binaries/win64/FLEngine_x64.dll']);
});

test('a new file in the base does not reach the working tree, and verify stays ok', (t) => {
  const root = scratch(t);
  const { tree, tracked } = fixture(root);
  const dest = path.join(root, 'base');
  prepareBase({ tree, dest, archive: archiveFrom(tracked) });
  write(path.join(dest, 'lift/out-app/host/new-file.txt'), 'only in the base');
  assert.equal(fs.existsSync(path.join(tree, 'lift/out-app/host/new-file.txt')), false);
  assert.deepEqual(verifyBase({ tree, dest }), { ok: true, changed: [] });
});

test('verify without a manifest is not ok', (t) => {
  const root = scratch(t);
  const { tree } = fixture(root);
  const v = verifyBase({ tree, dest: path.join(root, 'nothing') });
  assert.equal(v.ok, false);
});
