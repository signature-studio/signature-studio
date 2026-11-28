// Scan of the installer's demo projects (Issue #1401): each .flp under Data\Demo projects\ in inputs/fl-installer-files.tsv is opened once, in its own launch of measure.mjs (the same method: the page, the project, Space, 10 s of audio). The project file is read from the extracted installer (lift/out-app/host/resources, the tree that the build copies); nothing is copied.
// Usage: node tools/p3-audio-probe/scan-demos.mjs <absolute path to the bundle directory raw/<name>> > scan.json
// stdout: one JSON line {"songs":[...]}, in the installer list's order. Per song: name (the file name without .flp), path (the guest path), error (null, or what measure.mjs reported; a file the tree does not have is not launched), notInInstaller (the count of the open's missing requests that the installer's list does not have; 0 when the open did not start, and then error says why), notInInstallerList (their paths), nonSilentQuanta and underrunDelta (null when not measured), elapsedMs (measure.mjs's).
// A launch that runs past SONG_LIMIT_MS is killed and the song gets an error. The launches run one after another.
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const USAGE = 'usage: node tools/p3-audio-probe/scan-demos.mjs <absolute path to the bundle directory raw/<name>> > scan.json\n';
const SONG_LIMIT_MS = 900000;
const DEMO_ROOT = 'Data\\Demo projects\\';
const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, '..', '..');
const measureFile = path.join(here, 'measure.mjs');
const TREE = path.join(repo, 'lift', 'out-app', 'host', 'resources');
const INSTALLER_LIST = path.join(repo, 'inputs', 'fl-installer-files.tsv');

if (process.argv.length !== 3) {
  process.stderr.write(USAGE);
  process.exit(2);
}
const bundleDir = path.resolve(process.argv[2]);
if (!fs.existsSync(bundleDir)) {
  process.stderr.write(`no bundle directory ${bundleDir}\n`);
  process.exit(2);
}

const candidates = fs.readFileSync(INSTALLER_LIST, 'utf8').split('\n')
  .map((l) => l.replace(/\r$/, '').split('\t')[0])
  .filter((g) => g.startsWith(DEMO_ROOT) && g.toLowerCase().endsWith('.flp'));
if (candidates.length === 0) {
  process.stderr.write('no demo projects in inputs/fl-installer-files.tsv\n');
  process.exit(1);
}

function scanSong(guest) {
  const name = guest.split('\\').pop().replace(/\.flp$/i, '');
  const base = { name, path: guest, notInInstaller: 0, notInInstallerList: [], nonSilentQuanta: null, underrunDelta: null, elapsedMs: null };
  const file = path.join(TREE, ...guest.split('\\'));
  if (!fs.existsSync(file)) return { ...base, error: `not in the tree: ${guest}` };
  const r = spawnSync(process.execPath, [measureFile, bundleDir, file], { cwd: repo, encoding: 'utf8', maxBuffer: 1 << 28, timeout: SONG_LIMIT_MS, stdio: ['ignore', 'pipe', 'inherit'] });
  let m = null;
  try {
    m = JSON.parse((r.stdout ?? '').trim().split('\n').pop());
  } catch {
    m = null;
  }
  if (m === null || typeof m !== 'object') {
    return { ...base, error: `measure.mjs gave no result (${r.error ? String(r.error.code ?? r.error.message) : `exit ${r.status}`})` };
  }
  let error = typeof m.error === 'string' ? m.error : null;
  if (error === null && typeof m.nonSilentQuanta !== 'number') error = 'the audio output gave no nonSilentQuanta';
  return {
    ...base,
    error,
    notInInstaller: typeof m.notInInstaller === 'number' ? m.notInInstaller : 0,
    notInInstallerList: Array.isArray(m.notInInstallerList) ? m.notInInstallerList : [],
    nonSilentQuanta: typeof m.nonSilentQuanta === 'number' ? m.nonSilentQuanta : null,
    underrunDelta: typeof m.underrunDelta === 'number' ? m.underrunDelta : null,
    elapsedMs: typeof m.elapsedMs === 'number' ? m.elapsedMs : null,
  };
}

const songs = [];
for (let i = 0; i < candidates.length; i++) {
  const s = scanSong(candidates[i]);
  process.stderr.write(`scan ${i + 1}/${candidates.length} ${s.name}: error=${s.error === null ? 'null' : JSON.stringify(s.error.slice(0, 160))} notInInstaller=${s.notInInstaller} nonSilentQuanta=${s.nonSilentQuanta} underrunDelta=${s.underrunDelta}\n`);
  songs.push(s);
}
process.stdout.write(JSON.stringify({ songs }) + '\n');
