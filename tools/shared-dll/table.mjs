// The table of the shared DLLs that the engine asks for (Issue #1511): inputs/shared-dll-requests.tsv, built from the measurement's output (tools/shared-dll/measure.mjs) and the installer's file list.
// Usage: node tools/shared-dll/table.mjs <measure.jsonl> <inputs/fl-installer-files.tsv> > inputs/shared-dll-requests.tsv
// A request is a missing-path line (lift/runtime/fle_vfs.c) of a file whose name is a DLL of the installer outside Plugins\ (the 79 rows of the installer list). The line's API is not a column:
// the LoadLibrary requests are counted apart in pm-work/findings/shared-dll-inventory.md. Names that are not in the list (Windows DLLs, plug-in DLLs, empty names) are left out here and counted on stderr.
// Columns (TAB): dll (the file name), guest (the installer's spelling of its path), trigger (the first operation that asked for it, group/step), count (the requests in all the operations, the starts' boot included), stoppedAfter (true when the engine stopped in that same operation: the stop is after the request in it; a stop in a later operation is not counted).
import fs from 'node:fs';

const INSTALL_ROOT = /^C:\\Program Files\\Image-Line\\FL Studio 21\\/i;
const DLL = /\.dll$/i;

/** the installer's DLL rows outside Plugins\: { guest, dll } */
export function installerDlls(tsvText) {
  return String(tsvText).split(/\r?\n/).slice(2).map((l) => l.split('\t')).filter((f) => f.length >= 4 && f[3] === 'dll' && !/^Plugins\\/i.test(f[0]))
    .map((f) => ({ guest: f[0], dll: f[0].split('\\').pop() }));
}

/** the guest spelling of a requested path (relative to the install root, or a bare name), or null when the installer lists no such DLL */
export function guestOf(request, list) {
  const rel = request.replace(/\//g, '\\').replace(INSTALL_ROOT, '').toLowerCase();
  const exact = list.find((d) => d.guest.toLowerCase() === rel);
  if (exact) return exact.guest;
  if (!rel.includes('\\')) return list.find((d) => d.dll.toLowerCase() === rel)?.guest ?? null;
  return list.find((d) => `\\${d.guest.toLowerCase()}`.endsWith(`\\${rel}`))?.guest ?? null;
}

/** the table's rows from the measurement's lines (one JSON object per operation, and one per start of the bundle) */
export function tableRows(jsonlText, list) {
  const lines = String(jsonlText).split(/\r?\n/).filter((l) => l.trim() !== '').map((l) => JSON.parse(l));
  const ends = new Map();   // group/session -> stoppedAt
  for (const r of lines) if (r.sessionEnd) ends.set(`${r.group}#${r.session}`, r.stoppedAt);
  const rows = new Map();
  for (const r of lines) {
    if (r.sessionEnd) continue;
    for (const m of r.missing ?? []) {
      if (!DLL.test(m.path) || m.path === '') continue;
      const guest = guestOf(m.path, list);
      if (guest === null) continue;
      const dll = guest.split('\\').pop();
      if (!rows.has(guest)) rows.set(guest, { dll, guest, trigger: `${r.group}/${r.step}`, index: r.index, session: r.session, group: r.group, count: 0 });
      rows.get(guest).count++;
    }
  }
  return [...rows.values()].sort((a, b) => a.index - b.index).map((row) => {
    const stopped = ends.get(`${row.group}#${row.session}`);
    return { dll: row.dll, guest: row.guest, trigger: row.trigger, count: row.count, stoppedAfter: stopped === row.index };
  });
}

async function main() {
  const [jsonlPath, tsvPath] = process.argv.slice(2);
  if (!jsonlPath || !tsvPath) {
    process.stderr.write('usage: node tools/shared-dll/table.mjs <measure.jsonl> <inputs/fl-installer-files.tsv>\n');
    process.exit(2);
  }
  const list = installerDlls(fs.readFileSync(tsvPath, 'utf8'));
  const rows = tableRows(fs.readFileSync(jsonlPath, 'utf8'), list);
  process.stdout.write(['dll', 'guest', 'trigger', 'count', 'stoppedAfter'].join('\t') + '\n');
  for (const r of rows) process.stdout.write([r.dll, r.guest, r.trigger, r.count, r.stoppedAfter ? 'true' : 'false'].join('\t') + '\n');
  process.stderr.write(`rows ${rows.length} (installer DLLs outside Plugins\\: ${list.length})\n`);
}

const self = process.argv[1] ? fs.realpathSync(process.argv[1]) : null;
if (self === fs.realpathSync(new URL(import.meta.url).pathname)) await main();
