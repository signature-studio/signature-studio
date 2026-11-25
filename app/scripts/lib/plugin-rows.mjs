// The rows of the plugins for the extraction (Issue #1369). Each folder lift/plugins/<name>/ holds one plugin.tsv: a header line (key, guest, role, sha256, size, hook, args, omit),
// then one row per DLL of the plugin. pluginRows() turns those rows into rows of the DLL table (extract-fl.mjs --dlls: name, host path, SHA-256)
// and of the seed table (--seed: guest path, host path, SHA-256), the same rows as fe-build's plugins stage writes (lift/crates/fe-build/src/plugins.rs). The two must agree.
// A folder without plugin.tsv is not a plugin: a missing lift/plugins gives no rows.
import fs from 'node:fs';
import path from 'node:path';

const COLUMNS = ['key', 'guest', 'role', 'sha256', 'size', 'hook', 'args', 'omit'];
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

function validate(dlls) {
  const keys = new Set(), guests = new Set(), names = new Set();
  for (const d of dlls) {
    const at = (m) => new Error(`plugin ${d.plugin} (${d.key}): ${m}`);
    const base = d.guest.split('\\').pop();
    const name = base.toLowerCase().replaceAll(' ', '_');
    if (!/^[a-z0-9._-]+$/.test(d.key)) throw at('the key is lower case letters, digits, \'.\', \'-\' and \'_\' only');
    if (keys.has(d.key)) throw at('the key is not unique');
    keys.add(d.key);
    if (d.guest.split('\\').length < 3 || d.guest.includes('/')) throw at('the guest path is Plugins\\...\\file (backslashes, at least three parts)');
    if (guests.has(d.guest.toLowerCase())) throw at('the guest path is not unique');
    guests.add(d.guest.toLowerCase());
    if (!base.toLowerCase().endsWith('.dll')) throw at('the file is not a DLL');
    if (names.has(name)) throw at(`the file name ${name} is not unique (the realmod name must be unique)`);
    names.add(name);
    if (!/^[0-9a-f]{64}$/.test(d.sha256)) throw at('sha256 is 64 lower case hex digits');
  }
}

// { extract: [[name, host path, SHA-256]], seed: [[guest path, host path, SHA-256]] }, in the order of the keys
export function pluginRows(dir) {
  if (!fs.existsSync(dir)) return { extract: [], seed: [] };
  const folders = fs.readdirSync(dir).filter((f) => fs.statSync(path.join(dir, f)).isDirectory()).sort();
  const dlls = [];
  for (const folder of folders) {
    if (!/^[a-z0-9-]+$/.test(folder)) throw new Error(`${dir}/${folder}: the folder name is lower case letters, digits and '-' only`);
    dlls.push(...parsePlugin(folder, fs.readFileSync(path.join(dir, folder, 'plugin.tsv'), 'utf8')));
  }
  dlls.sort((a, b) => (a.key < b.key ? -1 : a.key > b.key ? 1 : 0));
  validate(dlls);
  return {
    extract: dlls.map((d) => {
      const base = d.guest.split('\\').pop();
      return [base.toLowerCase(), `${HOST_ROOT}/${base.toLowerCase().replaceAll(' ', '_')}`, d.sha256];
    }),
    seed: dlls.map((d) => {
      const parts = d.guest.split('\\');
      return [GUEST_ROOT + d.guest, `${SEED_ROOT}/${parts.slice(0, -2).join('/')}/${parts[parts.length - 1].replaceAll(' ', '_')}`, d.sha256];
    }),
  };
}
