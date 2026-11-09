import { describe, expect, it } from 'vitest';
import { PROTECTED_PATHS, protectedHit, baseName, extOf, formatBytes, isInside, joinPath, nameProblem, normalizePath, parentOf, safeRelative, sanitizeName, splitPath, uniqueName } from '../../src/files/paths.ts';

describe('paths', () => {
  it('normalises, joins and splits', () => {
    expect(normalizePath('a//b/./c/')).toBe('/a/b/c');
    expect(normalizePath('')).toBe('/');
    expect(() => normalizePath('/a/../b')).toThrow();
    expect(joinPath('/', 'x')).toBe('/x');
    expect(joinPath('/a', 'x')).toBe('/a/x');
    expect(parentOf('/a/b')).toBe('/a');
    expect(parentOf('/a')).toBe('/');
    expect(baseName('/a/b.wav')).toBe('b.wav');
    expect(splitPath('/a/b')).toEqual(['a', 'b']);
    expect(isInside('/a/b/c', '/a/b')).toBe(true);
    expect(isInside('/a/bc', '/a/b')).toBe(false);
    expect(isInside('/a', '/a')).toBe(true);
  });
  it('rejects names that Windows would not keep', () => {
    for (const n of ['', '.', '..', 'a/b', 'a\\b', 'a:b', 'a*b', 'a?b', 'a"b', 'a<b', 'a>b', 'a|b', 'x\u0001', 'name ', 'name.', 'CON', 'nul.txt', 'com1', 'LPT9.wav', 'a'.repeat(256)]) expect(nameProblem(n), JSON.stringify(n)).not.toBeNull();
    for (const n of ['a', 'a b.wav', '日本語 サンプル.wav', 'x (2).flp', 'CONSOLE', 'com10', '.hidden', 'a.b.c', '[fadr.com] Vocals - x.wav', 'a'.repeat(255)]) expect(nameProblem(n), JSON.stringify(n)).toBeNull();
  });
  it('makes outside names acceptable and says whether it changed them', () => {
    expect(sanitizeName('a:b*c.wav')).toEqual(['a_b_c.wav', true]);
    expect(sanitizeName('ok.wav')).toEqual(['ok.wav', false]);
    expect(sanitizeName('trail. ')).toEqual(['trail', true]);
    expect(sanitizeName('a\ud800b')[0]).toBe('a_b');
    expect(sanitizeName('CON')).toEqual(['_CON', true]);
    expect(sanitizeName('a'.repeat(300) + '.wav')[0].length).toBe(255);
    expect(sanitizeName('a'.repeat(300) + '.wav')[0].endsWith('.wav')).toBe(true);
  });
  it('refuses a path that leaves the destination', () => {
    expect(safeRelative('../x')).toBeNull();
    expect(safeRelative('a/../../x')).toBeNull();
    expect(safeRelative('C:\\x\\y.txt')).toBeNull();
    expect(safeRelative('//server/share/x')).toBeNull();
    expect(safeRelative('')).toBeNull();
    expect(safeRelative('/')).toBeNull();
    expect(safeRelative('/abs/x.txt')).toEqual({ parts: ['abs', 'x.txt'], changed: true });
    expect(safeRelative('a\\b\\c.txt')).toEqual({ parts: ['a', 'b', 'c.txt'], changed: true });
    expect(safeRelative('p/Samples/a.wav')).toEqual({ parts: ['p', 'Samples', 'a.wav'], changed: false });
    expect(safeRelative('p/a:b.wav')).toEqual({ parts: ['p', 'a_b.wav'], changed: true });
  });
  it('numbers clashing names, ignoring case', () => {
    expect(uniqueName('a.wav', new Set())).toBe('a.wav');
    expect(uniqueName('a.wav', new Set(['a.wav']))).toBe('a (2).wav');
    expect(uniqueName('A.WAV', new Set(['a.wav', 'a (2).wav']))).toBe('A (3).WAV');
    expect(uniqueName('dir', new Set(['dir']))).toBe('dir (2)');
    expect(uniqueName('.hidden', new Set(['.hidden']))).toBe('.hidden (2)');
  });
  it('formats', () => {
    expect(extOf('A.ZIP')).toBe('zip');
    expect(extOf('noext')).toBe('');
    expect(formatBytes(0)).toBe('0 B');
    expect(formatBytes(1536)).toBe('1.50 KiB');
    expect(formatBytes(62747002)).toBe('59.8 MiB');
  });
});

describe('the folders the file manager asks about (act1 R5)', () => {
  it('knows the two folders and everything around them', () => {
    expect(PROTECTED_PATHS).toEqual(['/fl/state', '/fl/license']);
    expect(protectedHit('/fl/state')).toBe('/fl/state');
    expect(protectedHit('/fl/state/registrations.bin')).toBe('/fl/state');
    expect(protectedHit('/fl')).toBe('/fl/state');
    expect(protectedHit('/')).toBe('/fl/state');
    expect(protectedHit('/fl/license/x')).toBe('/fl/license');
    expect(protectedHit('/fl/other')).toBeNull();
    expect(protectedHit('/flstate')).toBeNull();
    expect(protectedHit('/Projects/fl/state')).toBeNull();
  });
});
