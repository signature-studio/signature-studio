import { describe, expect, it } from 'vitest';
import { EngineClient } from '../../src/engine/client.ts';
import {
  acceptFor, extensionsOfSpec, guestPathFor, isValidComponent, isValidGuestPath, MAX_FILE_BYTES, MAX_PATH_UNITS, MAX_TOTAL_BYTES, OPEN_FOLDER, parseRequest, planOpenFile, planOpenFolder, planSave, sanitizeComponent,
  SAVE_FOLDER, suggestedSaveName, type FileDlgRequest, type PickedFile,
} from '../../src/engine/filedlg.ts';
import { FruityEngine } from '../../src/engine/fruity.ts';
import type { ToWorker } from '../../src/engine/protocol.ts';
import { buildPack } from '../../src/bundle/pack.ts';
import { hostPackFromBytes } from '../../src/engine/hostfiles.ts';

// decision 0193 (fpick1): the page's side of the engine's file dialogs. The rules for the guest path are the volume's (lift/runtime/fle_vfs.c put_name_ok / put_walk); the C side has the same cases in
// lift/crates/fe-lift/tests/wall_fpick1.rs (the two lists are written separately from the rules, not from each other's code).

const FILTERS: [string, string][] = [
  ['FL Studio song file (*.flp)', '*.flp'], ['Zipped song file (*.zip)', '*.zip'], ['MIDI file (*.mid;*.midi)', '*.mid;*.midi'], ['FL Studio Mobile song file (*.flm)', '*.flm'],
  ['BeatCreator groove file (*.zgr)', '*.zgr'], ['Supported song files (*.flp;*.zip;*.mid;*.midi;*.flm;*.zgr)', '*.flp;*.zip;*.mid;*.midi;*.flm;*.zgr'],
];
const openReq = (o: Partial<FileDlgRequest> = {}): FileDlgRequest => ({ save: false, options: 0x20001000, filterIndex: 6, title: null, okLabel: null, fileName: null, defaultExt: null, folder: 'C:\\Users\\user\\Documents\\Image-Line\\FL Studio\\Projects', filters: FILTERS, ...o });
const saveReq = (o: Partial<FileDlgRequest> = {}): FileDlgRequest => ({ ...openReq(), save: true, options: 0x8802, filterIndex: 1, fileName: 'Sampler', defaultExt: 'wav', filters: [['Wave file (*.wav)', '*.wav']], ...o });
const pf = (relativePath: string, size = 10): PickedFile => ({ name: relativePath.split('/').pop() as string, size, relativePath });

describe('guest path components (the volume refuses what Windows refuses)', () => {
  it.each([
    ['Sampler.flp', true], ['SHP_Sawing Synth.wav', true], ['\u30d7\u30ed\u30b8\u30a7\u30af\u30c8.flp', true], ['a'.repeat(255), true], ['a'.repeat(256), false],
    ['', false], ['.', false], ['..', false], ['a.', false], ['a ', false], ['a<b', false], ['a>b', false], ['a:b', false], ['a"b', false], ['a|b', false], ['a?b', false], ['a*b', false], ['a/b', false], ['a\\b', false], ['a\u0001b', false],
    ['CON', false], ['con.txt', false], ['NUL', false], ['aux', false], ['COM1', false], ['com9.flp', false], ['LPT3', false], ['COM\u00b2', false], ['COM0', true], ['CONSOLE', true], ['COM10', true], ['LPT', true], ['NUL ', false],
    ['x\ud800', false], ['\udc00x', false], ['x\ud83d\ude00.flp', true],
  ])('isValidComponent(%j) = %s', (c, ok) => expect(isValidComponent(c)).toBe(ok));

  it('sanitizeComponent always gives a valid component and keeps a valid name as it is', () => {
    for (const n of ['Sampler.flp', 'a b.wav', '\u30d7\u30ed.flp']) expect(sanitizeComponent(n)).toBe(n);
    for (const n of ['a<b>c.flp', 'a:b', 'con', 'NUL.txt', 'trailing. ', '...', '', 'x\ud800y', 'q\u0000r', 'a'.repeat(400) + '.flp', 'COM3.wav', '  '])
      expect(isValidComponent(sanitizeComponent(n)), n).toBe(true);
    expect(sanitizeComponent('a<b>c.flp')).toBe('a_b_c.flp');
    expect(sanitizeComponent('con')).toBe('_con');
    expect(sanitizeComponent('trailing. ')).toBe('trailing');
    expect(sanitizeComponent('a'.repeat(400) + '.flp')).toBe('a'.repeat(251) + '.flp');
    expect(sanitizeComponent('x\ud800y')).toBe('x_y');
  });
});

describe('guest paths', () => {
  it('isValidGuestPath: drive C only, absolute, valid components, at most 259 units', () => {
    expect(isValidGuestPath('C:\\Users\\user\\Downloads\\Sampler.flp')).toBe(true);
    expect(isValidGuestPath('c:\\a')).toBe(true);
    for (const p of ['C:', 'C:\\', 'D:\\a', 'a\\b', '\\a', 'C:a', 'C:\\a\\', 'C:\\a\\\\b', 'C:\\a\\..\\b', 'C:\\a\\con', 'C:\\a\\b?']) expect(isValidGuestPath(p), p).toBe(false);
    expect(isValidGuestPath('C:\\' + 'a'.repeat(MAX_PATH_UNITS - 3))).toBe(false);   // one component over 255
    expect(isValidGuestPath('C:\\' + 'a'.repeat(100) + '\\' + 'b'.repeat(MAX_PATH_UNITS - 3 - 101))).toBe(true);
    expect(isValidGuestPath('C:\\' + 'a'.repeat(100) + '\\' + 'b'.repeat(MAX_PATH_UNITS - 3 - 100))).toBe(false);
  });
  it('guestPathFor joins a folder and a relative path (either separator), sanitising each component; null when too long', () => {
    expect(guestPathFor(OPEN_FOLDER, 'Sampler.flp')).toBe('C:\\Users\\user\\Downloads\\Sampler.flp');
    expect(guestPathFor(OPEN_FOLDER, 'proj/sub\\x?.wav')).toBe('C:\\Users\\user\\Downloads\\proj\\sub\\x_.wav');
    expect(guestPathFor(`${OPEN_FOLDER}\\`, './a.wav')).toBe('C:\\Users\\user\\Downloads\\a.wav');
    expect(guestPathFor(OPEN_FOLDER, '')).toBeNull();
    expect(guestPathFor(OPEN_FOLDER, Array.from({ length: 30 }, () => 'dddddddddd').join('/') + '/x')).toBeNull();
  });
});

describe('file types', () => {
  it('extensionsOfSpec reads the engine\'s specs', () => {
    expect(extensionsOfSpec('*.flp;*.zip;*.mid;*.midi')).toEqual(['.flp', '.zip', '.mid', '.midi']);
    expect(extensionsOfSpec('*.FLP')).toEqual(['.flp']);
    expect(extensionsOfSpec('*.*')).toEqual([]);
    expect(extensionsOfSpec(null)).toEqual([]);
  });
  it('acceptFor gives the selected type (1-based) as an accept list; nothing when the index is out of range', () => {
    expect(acceptFor(openReq())).toBe('.flp,.zip,.mid,.midi,.flm,.zgr');
    expect(acceptFor(openReq({ filterIndex: 1 }))).toBe('.flp');
    expect(acceptFor(openReq({ filterIndex: 99 }))).toBe('');
  });
});

describe('Open plans', () => {
  it('one file goes to Downloads under its own name and is the dialog result', () => {
    expect(planOpenFile(openReq(), { name: 'Sampler.flp', size: 1000 })).toEqual({ ok: true, value: { items: [{ role: 'R', path: 'C:\\Users\\user\\Downloads\\Sampler.flp' }] } });
  });
  it('a file over 256 MiB, a folder request and a Save request are refused with a reason', () => {
    const big = planOpenFile(openReq(), { name: 'big.flp', size: MAX_FILE_BYTES + 1 });
    expect(big.ok).toBe(false);
    expect(planOpenFile(openReq(), { name: 'edge.flp', size: MAX_FILE_BYTES }).ok).toBe(true);
    expect(planOpenFile(openReq({ options: 0x20 }), { name: 'a.flp', size: 1 }).ok).toBe(false);
    expect(planOpenFile(saveReq(), { name: 'a.flp', size: 1 }).ok).toBe(false);
  });
  it('a project folder: the FLP is the result, every other file is an extra next to it', () => {
    const files = [pf('proj/a.wav'), pf('proj/song.flp'), pf('proj/b.mp3'), pf('proj/sub/c.ogg'), pf('proj/z.mid')];
    const p = planOpenFolder(openReq(), files);
    expect(p.ok).toBe(true);
    if (!p.ok) return;
    expect(p.value.resultIndex).toBe(1);
    expect(p.value.items.map((i) => `${i.role} ${i.path}`)).toEqual([
      'E C:\\Users\\user\\Downloads\\proj\\a.wav', 'R C:\\Users\\user\\Downloads\\proj\\song.flp', 'E C:\\Users\\user\\Downloads\\proj\\b.mp3', 'E C:\\Users\\user\\Downloads\\proj\\sub\\c.ogg', 'E C:\\Users\\user\\Downloads\\proj\\z.mid',
    ]);
  });
  it('several FLPs: the one nearest the top, then by name; without any FLP the first file of a type the engine offers; none: refused', () => {
    expect((planOpenFolder(openReq(), [pf('p/sub/a.flp'), pf('p/z.flp'), pf('p/b.flp')]) as { value: { resultIndex: number } }).value.resultIndex).toBe(2);
    expect((planOpenFolder(openReq(), [pf('p/x.wav'), pf('p/m.mid')]) as { value: { resultIndex: number } }).value.resultIndex).toBe(1);
    expect((planOpenFolder(openReq(), [pf('p/a.mid'), pf('p/b.flp'), pf('p/c.zip')]) as { value: { resultIndex: number } }).value.resultIndex).toBe(1);   // an FLP wins over a MIDI file that sorts first
    expect(planOpenFolder(openReq(), [pf('p/x.wav')]).ok).toBe(false);
    expect(planOpenFolder(openReq(), []).ok).toBe(false);
  });
  it('limits: 256 MiB per file, 384 MiB in all, 512 files', () => {
    expect(planOpenFolder(openReq(), [pf('p/a.flp', MAX_FILE_BYTES + 1)]).ok).toBe(false);
    expect(planOpenFolder(openReq(), [pf('p/a.flp', MAX_FILE_BYTES), pf('p/b.wav', MAX_TOTAL_BYTES - MAX_FILE_BYTES)]).ok).toBe(true);
    expect(planOpenFolder(openReq(), [pf('p/a.flp', MAX_FILE_BYTES), pf('p/b.wav', MAX_TOTAL_BYTES - MAX_FILE_BYTES + 1)]).ok).toBe(false);
    expect(planOpenFolder(openReq(), Array.from({ length: 513 }, (_, i) => pf(`p/f${i}.wav`, 1))).ok).toBe(false);
  });
  it('a sample whose path does not fit is left out; the project file itself must fit', () => {
    const long = 'l'.repeat(200);
    const p = planOpenFolder(openReq(), [pf('p/a.flp'), pf(`p/${long}/${long}.wav`)]);
    expect(p.ok && p.value.items.length).toBe(1);
    expect(planOpenFolder(openReq(), [pf(`p/${long}/${long}.flp`)]).ok).toBe(false);
  });
});

describe('Save plans', () => {
  it('the engine\'s file name and default extension; the folder the engine set if the volume takes it, else Documents', () => {
    expect(suggestedSaveName(saveReq())).toBe('Sampler.wav');
    expect(suggestedSaveName(saveReq({ fileName: 'C:\\x\\Song.flp' }))).toBe('Song.flp');
    expect(suggestedSaveName(saveReq({ fileName: null, defaultExt: null }))).toBe('Untitled');
    expect(planSave(saveReq({ folder: null }), 'Sampler.wav')).toEqual({ ok: true, value: { path: `${SAVE_FOLDER}\\Sampler.wav`, name: 'Sampler.wav' } });
    expect(planSave(saveReq({ folder: 'C:\\Users\\user\\Documents\\Image-Line\\FL Studio\\Projects\\' }), 'Mix')).toEqual({ ok: true, value: { path: 'C:\\Users\\user\\Documents\\Image-Line\\FL Studio\\Projects\\Mix.wav', name: 'Mix.wav' } });
    expect(planSave(saveReq({ folder: 'D:\\nope' }), 'Mix')).toEqual({ ok: true, value: { path: `${SAVE_FOLDER}\\Mix.wav`, name: 'Mix.wav' } });
    expect(planSave(saveReq({ folder: null }), '')).toEqual({ ok: true, value: { path: `${SAVE_FOLDER}\\Untitled.wav`, name: 'Untitled.wav' } });
  });
  it('a name with characters Windows refuses is sanitised, not refused; an Open request is refused', () => {
    expect(planSave(saveReq({ folder: null }), 'a/b:c?.wav')).toEqual({ ok: true, value: { path: `${SAVE_FOLDER}\\a_b_c_.wav`, name: 'a_b_c_.wav' } });
    expect(planSave(openReq(), 'x').ok).toBe(false);
  });
});

describe('parseRequest: the Worker\'s message is checked before the page uses it', () => {
  const good = { save: 0, options: 4096, filterIndex: 1, title: 'Open', okLabel: null, fileName: null, defaultExt: null, folder: 'C:\\x', filters: [['A', '*.a'], [null, null]] };
  it('accepts the shape the C side writes (save as 0/1)', () => {
    expect(parseRequest(good)).toEqual({ save: false, options: 4096, filterIndex: 1, title: 'Open', okLabel: null, fileName: null, defaultExt: null, folder: 'C:\\x', filters: [['A', '*.a'], [null, null]] });
    expect(parseRequest({ ...good, save: 1 })?.save).toBe(true);
  });
  it('rejects the rest', () => {
    for (const x of [null, 3, 'x', {}, { ...good, options: 'a' }, { ...good, filters: 3 }, { ...good, filters: [3] }, { ...good, save: 'yes' }]) expect(parseRequest(x), JSON.stringify(x)).toBeNull();
    expect(parseRequest({ ...good, title: 5 })?.title).toBeNull();
  });
});

describe('the protocol between the page and the Worker', () => {
  class FakeWorker {
    onmessage: ((ev: { data: unknown }) => void) | null = null;
    onerror: unknown = null;
    onmessageerror: unknown = null;
    static sent: ToWorker[] = [];
    postMessage(m: ToWorker): void { FakeWorker.sent.push(m); }
    terminate(): void { /* nothing */ }
  }
  const g = globalThis as unknown as { Worker?: unknown };
  const manifest = { engine: { kind: 'emscripten-fruity-v1', env: {} } } as never;
  const assets = { module: {} as WebAssembly.Module, seed: new Uint8Array(0), glue: new Uint8Array(0), dll: new Uint8Array(0) };
  const base = { deskCanvas: () => ({ transferControlToOffscreen: () => ({}) }) as unknown as HTMLCanvasElement, exit: () => undefined, error: () => undefined };

  it('a request from the Worker reaches fileDlg() parsed; the answer goes back as a filedlg-answer message with the same id', () => {
    const saved = g.Worker;
    g.Worker = FakeWorker;
    FakeWorker.sent.length = 0;
    try {
      const got: { id: number; req: FileDlgRequest }[] = [];
      const c = new EngineClient({ ...base, log: () => undefined, fileDlg: (id, req) => got.push({ id, req }) });
      void c.init(manifest, assets);
      const w = (c as unknown as { worker: FakeWorker }).worker;
      w.onmessage?.({ data: { t: 'filedlg-request', id: 7, request: { save: 0, options: 1, filterIndex: 1, title: null, okLabel: null, fileName: null, defaultExt: null, folder: null, filters: [] } } });
      expect(got).toHaveLength(1);
      expect(got[0]?.id).toBe(7);
      c.answerFileDlg(7, { ok: false, reason: 'x' });
      expect(FakeWorker.sent.at(-1)).toEqual({ cmd: 'filedlg-answer', id: 7, answer: { ok: false, reason: 'x' } });
    } finally { g.Worker = saved; }
  });
  it('a request the page cannot parse, or one with nobody to ask, is answered with a cancel at once (the engine is never left waiting)', () => {
    const saved = g.Worker;
    g.Worker = FakeWorker;
    FakeWorker.sent.length = 0;
    const logs: string[] = [];
    try {
      const c = new EngineClient({ ...base, log: (_l, t) => logs.push(t) });   // no fileDlg handler
      void c.init(manifest, assets);
      const w = (c as unknown as { worker: FakeWorker }).worker;
      w.onmessage?.({ data: { t: 'filedlg-request', id: 1, request: { save: 0, options: 1, filterIndex: 1, title: null, okLabel: null, fileName: null, defaultExt: null, folder: null, filters: [] } } });
      expect(FakeWorker.sent.at(-1)).toMatchObject({ cmd: 'filedlg-answer', id: 1, answer: { ok: false } });
      w.onmessage?.({ data: { t: 'filedlg-request', id: 2, request: 5 } });
      expect(FakeWorker.sent.at(-1)).toMatchObject({ cmd: 'filedlg-answer', id: 2, answer: { ok: false } });
      expect(logs.some((l) => /file dialog 2/.test(l))).toBe(true);
    } finally { g.Worker = saved; }
  });
  it('FruityEngine: the hooks exist only when the host has a file dialog; pollSaved is null for an engine without the export', async () => {
    const mk = (withDlg: boolean) => {
      const host = { mark: () => undefined, stdout: () => undefined, stderr: () => undefined, log: () => undefined, surfaceEvent: () => undefined, idle: () => undefined, ...(withDlg ? { fileDlg: () => undefined } : {}) };
      const m = { schema: 1, bundle: { name: 'f', version: '1' }, engine: { kind: 'emscripten-fruity-v1', wasm: { url: '/e', size: 1, sha256: 'a'.repeat(64), compression: 'identity' }, args: [], env: {}, options: { glue: { url: '/g', size: 1, sha256: 'a'.repeat(64), compression: 'identity' }, root: '/data', hostFilesAbi: 'emscripten-6.0.12' } }, seedPack: null, dllPack: null };
      return WebAssembly.compile(new Uint8Array([0, 0x61, 0x73, 0x6d, 1, 0, 0, 0])).then((module) => FruityEngine.create(m as never, { module, glue: new Uint8Array(0), seed: hostPackFromBytes(buildPack([])), dll: hostPackFromBytes(buildPack([])) }, host, async () => (async () => ({}))));
    };
    const a = await mk(true), b = await mk(false);
    expect(typeof a.opts.fleFileDlg).toBe('function');
    expect(b.opts.fleFileDlg).toBeUndefined();
    expect(a.pollSaved()).toBeNull();
    expect(a.answerFileDlg(1, { ok: false })).toBe(false);
  });
});
