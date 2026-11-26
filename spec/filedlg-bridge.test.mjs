import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';
import { test } from 'node:test';

function bridge({ host = true } = {}) {
  const texts = new Map([[1, JSON.stringify({ save: false, filters: [] })]]);
  const requests = [];
  let next = 1024, lib;
  const scope = {
    addToLibrary(value) { lib = value; },
    UTF8ToString: p => texts.get(p),
    stringToUTF8: (s, p) => texts.set(p, s),
    lengthBytesUTF8: s => Buffer.byteLength(s),
    _malloc(n) { const p = next; next += n; return p; },
    _free() {},
    HEAPU8: new Uint8Array(65536),
    setTimeout,
    Module: host ? { fleFileDlg(id, request) { requests.push({ id, request }); } } : {},
  };
  vm.runInNewContext(fs.readFileSync(new URL('../lift/harness/web/filedlg/fle_filedlg_lib.js', import.meta.url), 'utf8'), scope);
  scope.FleFileDlg = lib.$FleFileDlg;
  return { lib, scope, texts, requests };
}

test('a pick is synchronous and returns at once, a second pick while one is pending gets CANCEL, and the answer is consumed once', async () => {
  const { lib, scope, texts, requests } = bridge();
  assert.equal(lib.fle_filedlg_host_pick__async, undefined);
  assert.equal(lib.fle_filedlg_host_pick(1), 0);
  assert.equal(requests.length, 1);
  assert.equal(texts.get(lib.fle_filedlg_host_pick(1)), 'CANCEL');
  assert.equal(requests.length, 1);
  assert.equal(await lib.fle_filedlg_host_poll(), 0);
  assert.equal(scope.Module.fleFileDlgAnswer(requests[0].id + 1, { ok: false }), false);
  assert.equal(scope.Module.fleFileDlgAnswer(requests[0].id, { ok: false }), true);
  assert.equal(scope.Module.fleFileDlgAnswer(requests[0].id, { ok: false }), false);
  assert.equal(texts.get(await lib.fle_filedlg_host_poll()), 'CANCEL');
  assert.equal(await lib.fle_filedlg_host_poll(), 0);
  assert.equal(scope.Module.fleFileDlgAnswer(requests[0].id, { ok: false }), false);
  assert.equal(lib.fle_filedlg_host_pick(1), 0);
  assert.equal(requests.length, 2);
});

test('without a host nobody is asked: the answer is CANCEL at the first poll', async () => {
  const { lib, texts, requests } = bridge({ host: false });
  assert.equal(lib.fle_filedlg_host_pick(1), 0);
  assert.equal(texts.get(await lib.fle_filedlg_host_poll()), 'CANCEL');
  assert.equal(requests.length, 0);
});

test('file reading leaves the picker pending until every byte is available', async () => {
  const { lib, scope, texts, requests } = bridge();
  let complete;
  const bytes = new Uint8Array([3, 1, 4]);
  const file = { size: 3, stream() { return { getReader() {
    let first = true;
    return { read() { if (!first) return Promise.resolve({ done: true }); first = false; return new Promise(resolve => { complete = resolve; }); } };
  } }; } };
  lib.fle_filedlg_host_pick(1);
  scope.Module.fleFileDlgAnswer(requests[0].id, { ok: true, typeIndex: 1, items: [{ role: 'R', path: 'C:\\test.flp', file }] });
  assert.equal(await lib.fle_filedlg_host_poll(), 0);
  complete({ done: false, value: bytes });
  await new Promise(resolve => setImmediate(resolve));
  const answer = texts.get(await lib.fle_filedlg_host_poll());
  const match = answer.match(/^OK\nTYPE\t1\nR\t(\d+)\t3\tC:\\test.flp$/);
  assert.ok(match, answer);
  assert.deepEqual(scope.HEAPU8.slice(Number(match[1]), Number(match[1]) + 3), bytes);
  assert.equal(await lib.fle_filedlg_host_poll(), 0);
});

test('a poll yields a host task while the person has not answered', async () => {
  const { lib, scope, texts, requests } = bridge();
  lib.fle_filedlg_host_pick(1);
  let delivered = false;
  setTimeout(() => {
    delivered = true;
    scope.Module.fleFileDlgAnswer(requests[0].id, { ok: false });
  }, 0);
  const answer = await lib.fle_filedlg_host_poll();
  assert.equal(delivered, true);
  assert.equal(texts.get(answer), 'CANCEL');
  assert.equal(await lib.fle_filedlg_host_poll(), 0);
});

test('the wait loop of the C side gets the answer when guest sleeps only advance the virtual time', async () => {
  const { lib, scope, texts, requests } = bridge();
  let hostTasks = 0;
  const answerAfterThree = () => {
    hostTasks++;
    if (hostTasks < 3) setTimeout(answerAfterThree, 0);
    else scope.Module.fleFileDlgAnswer(requests[0].id, { ok: false });
  };
  setTimeout(answerAfterThree, 0);
  let ans = lib.fle_filedlg_host_pick(1);
  assert.equal(ans, 0);
  let polls = 0, virtualMs = 0;
  while (ans === 0) {
    virtualMs += 1;   // fle_thread_sleep("filedlg", 1): the virtual clock moves, no host task runs
    ans = await lib.fle_filedlg_host_poll();
    polls++;
    assert.ok(polls < 100, 'the answer never arrived');
  }
  assert.equal(texts.get(ans), 'CANCEL');
  assert.equal(hostTasks, 3);
  assert.equal(polls, 3);
  assert.equal(virtualMs, 3);
});
