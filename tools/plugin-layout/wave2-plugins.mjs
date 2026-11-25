#!/usr/bin/env node
// Issue #1379: the plugin folders of the second wave, as plugin.tsv files, for the layout test of fe-build (plugins.rs). Usage:
//   node tools/plugin-layout/wave2-plugins.mjs <out dir>
// Input: inputs/plugin-classes.tsv (the classes of the installer's DLLs). A unit is a plugin folder (Plugins\Fruity\<kind>\<folder>\); the folders
// with a VFX DLL (VFX yes) and the four first-wave folders (Fruity Balance, Fruity Filter, Fruity Delay 2, Sytrus) are left out, as in
// pm-work/tools/gen-wave-plugins.mjs. The result is 130 folders and 159 DLLs (131 本体, 570,656,256 B; 602,251,576 B in all).
// Output: <out dir>/<folder>/plugin.tsv, one per folder, in the format of lift/plugins/<name>/plugin.tsv. The hook column is "none" and the
// omit column is "-" for every row (the hook points are not known to this test; the layout does not depend on them). The out dir must not be lift/plugins
// (the test writes its files under the cargo target directory). Nothing else is written. Exit code 0 on success, 1 on a rejected input, 2 on a usage error.
import { mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..');
const outArg = process.argv[2];
if (!outArg || process.argv.length !== 3) {
  process.stderr.write('usage: node tools/plugin-layout/wave2-plugins.mjs <out dir>\n');
  process.exit(2);
}
const out = resolve(outArg);
if (out === join(root, 'lift', 'plugins') || out.startsWith(join(root, 'lift', 'plugins') + '/')) {
  process.stderr.write('wave2-plugins: the out dir must not be lift/plugins (the generated files are not committed there)\n');
  process.exit(2);
}

const PREFIX = 'Plugins\\Fruity\\';
const FIRST_WAVE = ['Plugins\\Fruity\\Effects\\Fruity Balance', 'Plugins\\Fruity\\Effects\\Fruity Filter', 'Plugins\\Fruity\\Effects\\Fruity Delay 2', 'Plugins\\Fruity\\Generators\\Sytrus'];
const KINDS = { Effects: 'effects', Generators: 'generators' };

const text = readFileSync(join(root, 'inputs', 'plugin-classes.tsv'), 'utf8');
const rows = text.replace(/\r\n/g, '\n').split('\n').slice(1).filter((l) => l !== '').map((l) => l.split('\t'));
const fail = (msg) => {
  process.stderr.write(`wave2-plugins: ${msg}\n`);
  process.exit(1);
};

const folders = new Map();
for (const f of rows) {
  const [path, size, sha, kind, role, , , , vfx] = f;
  if (f.length < 9 || !path.startsWith(PREFIX)) fail(`a row is not a Plugins\\Fruity DLL: ${path}`);
  const folder = path.split('\\').slice(0, 4).join('\\');
  if (!(kind in KINDS)) fail(`${path}: kind ${kind} is not Effects or Generators`);
  if (role !== '本体' && role !== '補助') fail(`${path}: role ${role}`);
  if (vfx !== 'yes' && vfx !== 'no') fail(`${path}: VFX ${vfx}`);
  if (!folders.has(folder)) folders.set(folder, { kind, dlls: [], vfx: false });
  const g = folders.get(folder);
  g.dlls.push({ path, size, sha, role });
  if (vfx === 'yes') g.vfx = true;
}

const slugOf = (name) => name.toLowerCase().replace(/ /g, '-').replace(/[^a-z0-9-]/g, '').replace(/-+/g, '-').replace(/^-|-$/g, '');
const keyOf = (kind, slug, base) => `${KINDS[kind]}.${slug}.${base.toLowerCase().replace(/ /g, '_').replace(/[^a-z0-9._-]/g, '_')}`;

const selected = [...folders.entries()].filter(([folder, g]) => !g.vfx && !FIRST_WAVE.includes(folder)).sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0));
rmSync(out, { recursive: true, force: true });
let dlls = 0, main = 0, total = 0;
const seen = new Set();
for (const [folder, g] of selected) {
  if (!g.dlls.some((d) => d.role === '本体')) fail(`${folder}: no 本体 DLL`);
  const slug = slugOf(`${KINDS[g.kind]} ${folder.split("\\")[3]}`);
  if (!slug || seen.has(slug)) fail(`${folder}: the folder name ${slug} is empty or not unique`);
  seen.add(slug);
  const lines = ['key\tguest\trole\tsha256\tsize\thook\targs\tomit'];
  for (const d of g.dlls) {
    const base = d.path.split('\\').pop();
    lines.push([keyOf(g.kind, slug, base), d.path, d.role, d.sha, d.size, 'none', '', '-'].join('\t'));
    dlls++;
    total += Number(d.size);
    if (d.role === '本体') main++;
  }
  mkdirSync(join(out, slug), { recursive: true });
  writeFileSync(join(out, slug, 'plugin.tsv'), lines.join('\n') + '\n');
}
process.stdout.write(`wave2-plugins: ${selected.length} folders, ${dlls} DLLs (${main} 本体), ${total} B\n`);
