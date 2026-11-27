// The rows of the plugins for the extraction (Issue #1369). Each folder lift/plugins/<name>/ holds one plugin.tsv: a header line (key, guest, role, sha256, size, hook, args, omit),
// then one row per DLL of the plugin. pluginRows() turns those rows into rows of the DLL table (extract-fl.mjs --dlls: name, host path, SHA-256)
// and of the seed table (--seed: guest path, host path, SHA-256), the same rows as fe-build's plugins stage writes (lift/crates/fe-build/src/plugins.rs). The two must agree.
// A folder without plugin.tsv is not a plugin: a missing lift/plugins gives no rows.
// Issue #1478: a folder may also hold data.tsv: the files of the plugin's Data folder (guest, installer, sha256, size). Each one is a seed row of its own,
// a @lazy-file row (the page serves it after the engine has started): the host file is plugins/ and the guest path (\ as /, a space as _).
// Its name and size are the ones of inputs/fl-installer-files.tsv (checked here).
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..', '..');
const INSTALLER_LIST = path.join(ROOT, 'inputs/fl-installer-files.tsv');
const COLUMNS = ['key', 'guest', 'role', 'sha256', 'size', 'hook', 'args', 'omit'];
const DATA_COLUMNS = ['guest', 'installer', 'sha256', 'size'];
const DATA_PREFIX = 'Plugins\\Fruity\\';
const LAZY_FILE = '@lazy-file:';
const GUEST_ROOT = 'C:\\Program Files\\Image-Line\\FL Studio 21\\';
const SEED_ROOT = 'lift/out-idle1/inputs/install';
const HOST_ROOT = 'lift/out-plugins';

function parsePlugin(plugin, text) {
  const out = [];
  let header = false;
  const lines = text.split(/\r\n|\r|\n/);
  lines.forEach((line, i) => {
    if (line.trim() === '' || line.startsWith('#')) return;
    const at = (m) => new Error(`${plugin}/plugin.tsv line ${i + 1}: ${m}`);
    const f = line.split('\t');
    if (!header) {
      if (f.join('\t') !== COLUMNS.join('\t')) throw at('the header must be key, guest, role, sha256, size, hook, args, omit (TAB separated)');
      header = true;
      return;
    }
    if (f.length !== 8) throw at(`${f.length} fields, want 8`);
    const [key, guest, role, sha256, size, hook, , omit] = f;
    if (role !== '本体' && role !== '補助') throw at(`role ${role}: want 本体 or 補助`);
    if (!/^[0-9]+$/.test(size) || Number(size) <= 0) throw at('size must be a positive integer');
    if (!['none', 'existing'].includes(hook)) {
      const rvas = /^rva:(.*)$/.exec(hook)?.[1].split(',') ?? [];
      if (rvas.length !== 3 || !rvas.every((v) => /^(0x)?[0-9a-fA-F]+$/.test(v.trim()))) throw at(`hook ${hook}: want none, existing or rva:0x..,0x..,0x..`);
    }
    if (omit !== '-' && !omit.split(',').every((v) => /^(0x)?[0-9a-fA-F]+$/.test(v))) throw at(`omit ${omit}: want - or comma separated hex RVAs`);
    out.push({ plugin, key, guest, sha256 });
  });
  if (!header) throw new Error(`${plugin}/plugin.tsv: no header`);
  if (out.length === 0) throw new Error(`${plugin}/plugin.tsv: no DLL row`);
  return out;
}

// The checks of plugins.rs validate(): the keys (the realmod names, and the stems of the rebased files) and the guest paths are unique;
// the file names need not be (Issue #1379: 18 DLLs are engine_x64.dll).
function validate(dlls) {
  const keys = new Set(), guests = new Set();
  for (const d of dlls) {
    const at = (m) => new Error(`plugin ${d.plugin} (${d.key}): ${m}`);
    const base = d.guest.split('\\').pop();
    if (!/^[a-z0-9._-]+$/.test(d.key)) throw at('the key is lower case letters, digits, \'.\', \'-\' and \'_\' only');
    if (!d.key.endsWith('.dll')) throw at('the key ends with .dll (it is the realmod name and the stem of the rebased file)');
    if (keys.has(d.key)) throw at('the key is not unique');
    keys.add(d.key);
    const parts = d.guest.split('\\').length;
    if ((parts < 3 && !(parts === 2 && d.guest.startsWith('Shared\\'))) || d.guest.includes('/')) throw at('the guest path is Plugins\\...\\file (backslashes, at least three parts) or Shared\\file (Issue #1429)');
    if (guests.has(d.guest.toLowerCase())) throw at('the guest path is not unique');
    guests.add(d.guest.toLowerCase());
    if (!/\.(dll|exe)$/.test(base.toLowerCase())) throw at('the file is not a DLL or an .exe');
    if (!/^[0-9a-f]{64}$/.test(d.sha256)) throw at('sha256 is 64 lower case hex digits');
  }
}

// data.tsv: the header, then one row per file of the plugin's Data folder (`#` lines are notes). Rows: guest, installer, sha256, size.
function parseData(plugin, text) {
  const out = [];
  let header = false;
  text.split(/\r\n|\r|\n/).forEach((line, i) => {
    if (line.trim() === '' || line.startsWith('#')) return;
    const at = (m) => new Error(`${plugin}/data.tsv line ${i + 1}: ${m}`);
    const f = line.split('\t');
    if (!header) {
      if (f.join('\t') !== DATA_COLUMNS.join('\t')) throw at('the header must be guest, installer, sha256, size (TAB separated)');
      header = true;
      return;
    }
    if (f.length !== 4) throw at(`${f.length} fields, want 4`);
    const [guest, installer, sha256, size] = f;
    if (!/^[0-9]+$/.test(size) || Number(size) <= 0) throw at('size must be a positive integer');
    out.push({ plugin, guest, installer, sha256, size: Number(size) });
  });
  if (!header) throw new Error(`${plugin}/data.tsv: no header`);
  return out;
}

// inputs/fl-installer-files.tsv: installer path → { size, sha256 }
function installerRows() {
  const out = new Map();
  for (const line of fs.readFileSync(INSTALLER_LIST, 'utf8').split(/\r\n|\r|\n/)) {
    if (line.trim() === '' || line.startsWith('#') || line.startsWith('guest\t')) continue;
    const f = line.split('\t');
    out.set(f[0], { size: Number(f[1]), sha256: f[2] });
  }
  return out;
}

// the host file of a Data row, relative to the host tree: the guest path with / for \ and _ for a space (the names of #1414)
function dataHost(guest) {
  return `plugins/${guest.replaceAll('\\', '/').replaceAll(' ', '_')}`;
}

// The checks of the Data rows (plugins.rs validate_data): the guest path is the folder's own Data (Plugins\Fruity\<kind>\<folder>\Data\...), it is not a DLL
// row's, and its installer row is in the list with the same SHA-256 and size. The guest paths and the host files are unique.
function validateData(data, dlls) {
  if (data.length === 0) return;
  const list = installerRows();
  const guests = new Set(dlls.map((d) => d.guest.toLowerCase())), hosts = new Set();
  for (const d of data) {
    const at = (m) => new Error(`plugin ${d.plugin} (${d.guest}): ${m}`);
    const parts = d.guest.split('\\');
    if (!d.guest.startsWith(DATA_PREFIX) || !/^Plugins\\Fruity\\(Effects|Generators)\\/.test(d.guest) || parts.length < 6 || parts[4] !== 'Data' || d.guest.includes('/')) {
      throw at('the guest path is Plugins\\Fruity\\<kind>\\<folder>\\Data\\... (the folder\'s own Data)');
    }
    if (d.guest.toLowerCase().endsWith('.dll')) throw at('a DLL is a row of plugin.tsv, not of data.tsv');
    const row = list.get(d.installer);
    if (!row) throw at(`installer ${d.installer}: not a row of inputs/fl-installer-files.tsv`);
    if (d.installer.toLowerCase() !== d.guest.toLowerCase()) throw at('the installer path is the guest path (the same spelling, case-insensitive)');
    if (!/^[0-9a-f]{64}$/.test(d.sha256)) throw at('sha256 is 64 lower case hex digits');
    if (row.sha256 !== d.sha256) throw at('sha256 is not the one of the installer list');
    if (row.size !== d.size) throw at(`size ${d.size}: the installer list says ${row.size}`);
    if (guests.has(d.guest.toLowerCase())) throw at('the guest path is not unique (a DLL or another Data row has it)');
    guests.add(d.guest.toLowerCase());
    const host = dataHost(d.guest);
    if (hosts.has(host)) throw at(`the host file ${host} is not unique (a space is _)`);
    hosts.add(host);
  }
}

// { extract: [[installer name, host path, SHA-256]], seed: [[guest path, host path, SHA-256]] }, in the order of the keys (the DLLs), then the Data rows by guest path.
// The host path is named by the key (plugins.rs host_file, seed_file); the installer name is the file's name in lower case (installer_name).
// A row whose file is an .exe (Issue #1499) is extracted, but it has no seed row here: its VFS row is the fixed seed table's (plugins.rs seed_table).
function readPlugins(dir) {
  const folders = fs.readdirSync(dir).filter((f) => fs.statSync(path.join(dir, f)).isDirectory()).sort();
  const dlls = [], data = [];
  for (const folder of folders) {
    if (!/^[a-z0-9-]+$/.test(folder)) throw new Error(`${dir}/${folder}: the folder name is lower case letters, digits and '-' only`);
    dlls.push(...parsePlugin(folder, fs.readFileSync(path.join(dir, folder, 'plugin.tsv'), 'utf8')));
    const file = path.join(dir, folder, 'data.tsv');
    if (fs.existsSync(file)) data.push(...parseData(folder, fs.readFileSync(file, 'utf8')));
  }
  dlls.sort((a, b) => (a.key < b.key ? -1 : a.key > b.key ? 1 : 0));
  validate(dlls);
  data.sort((a, b) => (a.guest < b.guest ? -1 : a.guest > b.guest ? 1 : 0));
  validateData(data, dlls);
  return { dlls, data };
}

const isExe = (guest) => /\.exe$/i.test(guest.split('\\').pop());

export function pluginRows(dir) {
  if (!fs.existsSync(dir)) return { extract: [], seed: [] };
  const { dlls, data } = readPlugins(dir);
  return {
    extract: dlls.map((d) => [d.guest.split('\\').pop().toLowerCase(), `${HOST_ROOT}/${d.key}`, d.sha256]),
    seed: [
      ...dlls.filter((d) => !isExe(d.guest)).map((d) => {
        const parts = d.guest.split('\\');
        return [GUEST_ROOT + d.guest, [SEED_ROOT, ...parts.slice(0, -2), d.key].join('/'), d.sha256];
      }),
      ...data.map((d) => [GUEST_ROOT + d.guest, `${LAZY_FILE}${dataHost(d.guest)}`, d.sha256]),
    ],
  };
}

// The .exe rows (Issue #1499): { host, guest, sha256 } of each, host being its extraction (lift/out-plugins/<key>). The file is the installer's, and it is
// the resources' file too (its VFS row is the fixed seed table's), so a missing extraction is placed from the resources (tools/place-plugin-dlls.mjs).
export function pluginExes(dir) {
  if (!fs.existsSync(dir)) return [];
  return readPlugins(dir).dlls.filter((d) => isExe(d.guest)).map((d) => ({ host: `${HOST_ROOT}/${d.key}`, guest: d.guest, sha256: d.sha256 }));
}
