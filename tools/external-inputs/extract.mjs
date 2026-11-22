#!/usr/bin/env node
// tools/external-inputs/extract.mjs DIR OUT [--lock FILE] [--tsv FILE] : takes the VST3 binaries out of the external plugin installers in DIR, and writes each one to OUT/<plugin>/.
// DIR's installers are first checked against the lock (as check.mjs does); nothing is taken out when a check fails. NSIS installers are read by 7-Zip (sevenzip.mjs),
// Inno Setup installers by inno.mjs. No installer is run. --tsv FILE writes the list (plugin, path, bytes, sha256, imports; TAB separated).
// Prints one JSON line per binary. Exit: 0 when every plugin gives a binary, 1 otherwise (or when a check fails), 64 for a usage error.
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { checkDir, externalEntries } from './check.mjs';
import { innoFiles } from './inno.mjs';
import { isPe, peImports } from './pe.mjs';
import { sevenZipList, sevenZipRead } from './sevenzip.mjs';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const VST3 = /\.vst3$/i;

/** the plugin name of a lock description: "Serum 2 2.1.5 (VST3)" -> "Serum 2" */
export function pluginName(used) {
  return used.replace(/ \(VST3\)$/, '').replace(/ \d[\d.]*$/, '');
}

/** the VST3 binaries of one installer: [{ path, data }], with the path under the VST3 folder (the install prefix left out) */
function binariesOf(file) {
  const list = sevenZipList(file);
  if (list.nsis) {
    return list.paths.filter((p) => VST3.test(p)).flatMap((p) => {
      const data = sevenZipRead(file, p);
      const i = p.toLowerCase().indexOf('/vst3/');
      return isPe(data) && i >= 0 ? [{ path: p.slice(i + '/vst3/'.length), data }] : [];
    });
  }
  return innoFiles(fs.readFileSync(file))
    .filter((f) => VST3.test(f.dest) && isPe(f.data))
    .map((f) => ({ path: f.dest.replace(/^\{[^}]*\}[\\/]/, '').replace(/\\/g, '/'), data: f.data }));
}

function main(argv) {
  const args = [...argv];
  let lock = path.join(ROOT, 'ci/artifacts.lock');
  let tsv = null;
  for (const [flag, set] of [['--lock', (v) => (lock = v)], ['--tsv', (v) => (tsv = v)]]) {
    const at = args.indexOf(flag);
    if (at >= 0) {
      set(args[at + 1]);
      args.splice(at, 2);
    }
  }
  const [dir, out] = args;
  if (!dir || !out) {
    process.stderr.write('usage: tools/external-inputs/extract.mjs DIR OUT [--lock FILE] [--tsv FILE]\n');
    return 64;
  }
  const entries = externalEntries(fs.readFileSync(lock, 'utf8'));
  const checked = checkDir(entries, dir);
  if (!checked.ok) {
    process.stdout.write(`${JSON.stringify(checked)}\n`);
    return 1;
  }
  const rows = [];
  const plugins = new Set();
  for (const e of entries) {
    const plugin = pluginName(e.used);
    for (const b of binariesOf(path.join(dir, e.name))) {
      const target = path.join(out, plugin.toLowerCase().replace(/[^a-z0-9]+/g, '-'), path.posix.basename(b.path));
      fs.mkdirSync(path.dirname(target), { recursive: true });
      fs.writeFileSync(target, b.data);
      const row = {
        plugin, path: b.path, bytes: b.data.length,
        sha256: crypto.createHash('sha256').update(b.data).digest('hex'),
        imports: peImports(b.data).join(';'),
      };
      rows.push(row);
      plugins.add(plugin);
      process.stdout.write(`${JSON.stringify({ plugin, path: row.path, bytes: row.bytes, sha256: row.sha256 })}\n`);
    }
  }
  if (tsv) {
    const lines = ['plugin\tpath\tbytes\tsha256\timports', ...rows.map((r) => [r.plugin, r.path, r.bytes, r.sha256, r.imports].join('\t'))];
    fs.mkdirSync(path.dirname(tsv), { recursive: true });
    fs.writeFileSync(tsv, `${lines.join('\n')}\n`);
  }
  return entries.every((e) => plugins.has(pluginName(e.used))) ? 0 : 1;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  process.exitCode = main(process.argv.slice(2));
}
