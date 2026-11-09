import { describe, expect, it } from 'vitest';
import { configOf, filtersOf, guestPathOfOpfs, matchesFilter, opfsPathOfGuestFolder, pickProject, planOpen, saveExtension, type SiblingFile } from '../../src/files/select.ts';
import type { FileDlgRequest } from '../../src/engine/filedlg.ts';

// the two requests the real engine made (docs/investigations/fpick1/tables/engine-requests.json: File > Open, and Ctrl+R Export)
const OPEN: FileDlgRequest = {
  save: false, options: 536875008, filterIndex: 6, title: null, okLabel: null, fileName: null, defaultExt: '.flp', folder: 'C:\\Users\\user\\Documents\\Image-Line\\FL Studio\\Projects',
  filters: [['FL Studio song file (*.flp)', '*.flp'], ['Zipped song file (*.zip)', '*.zip'], ['MIDI file (*.mid;*.midi)', '*.mid;*.midi'], ['FL Studio Mobile song file (*.flm)', '*.flm'],
    ['BeatCreator groove file (*.zgr)', '*.zgr'], ['Supported song files (*.flp;*.zip;*.mid;*.midi;*.flm;*.zgr)', '*.flp;*.zip;*.mid;*.midi;*.flm;*.zgr']],
};
const SAVE: FileDlgRequest = {
  save: true, options: 536870914, filterIndex: 3, title: null, okLabel: null, fileName: 'Sampler.wav', defaultExt: '.wav', folder: 'C:\\Users\\user\\Downloads\\Sampler-project',
  filters: [['FL Studio song file (*.flp)', '*.flp'], ['Zipped song file (*.zip)', '*.zip'], ['Microsoft wave file (*.wav)', '*.wav'], ['MPEG 3 audio file (*.mp3)', '*.mp3'], ['Ogg Vorbis audio file (*.ogg)', '*.ogg']],
};

describe('filters', () => {
  it('turns COMDLG_FILTERSPEC into extension lists and keeps the selected one', () => {
    const f = filtersOf(OPEN);
    expect(f[2]).toEqual({ label: 'MIDI file (*.mid;*.midi)', exts: ['.mid', '.midi'] });
    expect(f[5]!.exts).toEqual(['.flp', '.zip', '.mid', '.midi', '.flm', '.zgr']);
    expect(configOf(OPEN, '/').filterIndex).toBe(5);
    expect(configOf(SAVE, '/').filterIndex).toBe(2);
  });
  it('matches by extension, case-insensitively; *.* and no filter match everything', () => {
    const f = filtersOf(OPEN);
    expect(matchesFilter('Song.FLP', f[0])).toBe(true);
    expect(matchesFilter('song.wav', f[0])).toBe(false);
    expect(matchesFilter('noext', f[0])).toBe(false);
    expect(matchesFilter('anything.xyz', { label: 'All', exts: [] })).toBe(true);
    expect(matchesFilter('x', undefined)).toBe(true);
    expect(filtersOf({ ...OPEN, filters: [['All files', '*.*']] })[0]!.exts).toEqual([]);
    expect(filtersOf({ ...OPEN, filters: [] })[0]!.exts).toEqual([]);
  });
  it('adds the default extension to a name without one', () => {
    expect(saveExtension('.wav', undefined)).toBe('.wav');
    expect(saveExtension('wav', undefined)).toBe('.wav');
    expect(saveExtension('', { label: 'x', exts: ['.mp3'] })).toBe('.mp3');
    expect(saveExtension('', undefined)).toBe('');
  });
});

describe('configOf', () => {
  it('Open: no file name, project filter, the engine\'s folder is only a note (it is not an OPFS folder)', () => {
    const c = configOf(OPEN, '/Last');
    expect(c).toMatchObject({ mode: 'open', title: 'Open', okLabel: 'Open', fileName: '', multi: false, overwritePrompt: false, startDir: '/Last' });
    expect(c.note).toContain('Projects');
  });
  it('Save: the engine\'s suggested name, overwrite prompt from FOS_OVERWRITEPROMPT, start in the mapped folder', () => {
    const c = configOf(SAVE, '/Last');
    expect(c).toMatchObject({ mode: 'save', okLabel: 'Save', fileName: 'Sampler.wav', overwritePrompt: true, startDir: '/Sampler-project' });
  });
  it('Save without a name gets Untitled + the default extension; FOS_PICKFOLDERS = folder; FOS_ALLOWMULTISELECT = multi', () => {
    expect(configOf({ ...SAVE, fileName: null }, '/').fileName).toBe('Untitled.wav');
    expect(configOf({ ...SAVE, fileName: 'a' }, '/').fileName).toBe('a.wav');
    expect(configOf({ ...SAVE, fileName: 'C:\\x\\b.flp' }, '/').fileName).toBe('b.flp');
    expect(configOf({ ...OPEN, options: 0x20 }, '/').mode).toBe('folder');
    expect(configOf({ ...OPEN, options: 0x200 }, '/').multi).toBe(true);
    expect(configOf({ ...OPEN, filterIndex: 99 }, '/').filterIndex).toBe(5);
  });
});

describe('guest paths (provisional mapping of stage 2)', () => {
  it('OPFS path <-> guest path under Downloads', () => {
    expect(guestPathOfOpfs('/proj/a b.flp')).toBe('C:\\Users\\user\\Downloads\\proj\\a b.flp');
    expect(guestPathOfOpfs('/')).toBe('C:\\Users\\user\\Downloads');
    expect(opfsPathOfGuestFolder('C:\\Users\\user\\Downloads\\Sampler-project')).toBe('/Sampler-project');
    expect(opfsPathOfGuestFolder('C:\\Users\\user\\Downloads\\a\\b\\')).toBe('/a/b');
    expect(opfsPathOfGuestFolder('c:\\users\\USER\\downloads')).toBe('/');
    expect(opfsPathOfGuestFolder('C:\\Users\\user\\Documents\\Image-Line')).toBeNull();
    expect(opfsPathOfGuestFolder(null)).toBeNull();
  });
});

const sib = (dir: string, name: string, size = 10): SiblingFile => ({ name, path: (dir === '/' ? '' : dir) + '/' + name, size });

describe('planOpen', () => {
  const dir = '/serina';
  const files = [sib(dir, 'song.flp', 100), sib(dir, 'other.flp'), sib(dir, 'kick.wav', 50), sib(dir, 'snare.WAV', 60)];
  it('a project brings its neighbours (not other projects); the project is R, the neighbours are E', () => {
    const r = planOpen([sib(dir, 'song.flp', 100)], () => files);
    expect(r.ok).toBe(true);
    if (!r.ok) return;
    expect(r.value.items.map((i) => [i.role, i.guest])).toEqual([
      ['R', 'C:\\Users\\user\\Downloads\\serina\\song.flp'], ['E', 'C:\\Users\\user\\Downloads\\serina\\kick.wav'], ['E', 'C:\\Users\\user\\Downloads\\serina\\snare.WAV']]);
    expect(r.value.note).toBe('');
  });
  it('a MIDI file or a wav is alone', () => {
    const r = planOpen([sib(dir, 'kick.wav', 50)], () => files);
    expect(r.ok && r.value.items.length).toBe(1);
  });
  it('too many or too big neighbours: the project alone, with a note that the samples will be missing', () => {
    const many = Array.from({ length: 600 }, (_, i) => sib(dir, `s${i}.wav`));
    const r = planOpen([sib(dir, 'song.flp')], () => many);
    expect(r.ok).toBe(true);
    if (!r.ok) return;
    expect(r.value.items.length).toBe(1);
    expect(r.value.note).toMatch(/only the project file/);
    const big = planOpen([sib(dir, 'song.flp')], () => [sib(dir, 'huge.wav', 300 * 1048576)]);
    expect(big.ok && big.value.items.length).toBe(1);
  });
  it('refuses a chosen file over the volume\'s limit', () => {
    const r = planOpen([sib(dir, 'big.flp', 300 * 1048576)], () => []);
    expect(r.ok).toBe(false);
  });
  it('several chosen files (multi-select) keep their own folders', () => {
    const r = planOpen([sib('/a', 'x.mid'), sib('/b', 'y.mid')], () => []);
    expect(r.ok && r.value.items.map((i) => i.guest)).toEqual(['C:\\Users\\user\\Downloads\\a\\x.mid', 'C:\\Users\\user\\Downloads\\b\\y.mid']);
  });
});

describe('pickProject', () => {
  it('takes the shallowest .flp, ties by path; null when there is none', () => {
    expect(pickProject(['/z/a/deep.flp', '/z/top.flp', '/z/b.flp', '/z/s.wav'])).toBe('/z/b.flp');
    expect(pickProject(['/z/s.wav'])).toBeNull();
    expect(pickProject([])).toBeNull();
  });
});
