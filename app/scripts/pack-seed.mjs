#!/usr/bin/env node
// Seed pack of the browser build (decision 0134): ONE file that holds every host file the harness reads before the guest runs, so that a Worker can fetch it once.
// The port of lift/harness/web/final0129/pack_seed.py (the same layout and the same bytes; the header is the JSON that Python writes, with non-ASCII escaped).
//
//   node app/scripts/pack-seed.mjs --root <repository root> --tsv <vfs seed list> --extra <file> [--extra <file> ...] --out <pack file>
//
// What goes in: the VFS seed list and, for every line of it that names a host file, that host file (path relative to --root). The SHA-256 of every host file is checked
// against the list's third column before it is packed; the registry seeds and the Authenticode claims come in as --extra.
// Layout (little endian): 'FLSEEDP1' (8 bytes) | u32 header_length | header (UTF-8 JSON) | the data region.
// A JSON manifest is written next to the pack (<pack file>.manifest.json).
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';

const sha256 = (data) => crypto.createHash('sha256').update(data).digest('hex');
// Python's json.dumps (ensure_ascii): every character outside printable ASCII is written as \uXXXX (the line breaks of an indented form stay)
const pyJson = (v, indent) => (indent === undefined ? JSON.stringify(v) : JSON.stringify(v, null, indent))
  .replace(/[^\x20-\x7e\n]/g, (c) => `\\u${c.charCodeAt(0).toString(16).padStart(4, '0')}`);

function parseArgs(argv) {
  const a = { extra: [] };
  for (let i = 0; i < argv.length; i += 2) {
    const k = argv[i].replace(/^--/, '');
    if (k === 'extra') a.extra.push(argv[i + 1]);
    else a[k] = argv[i + 1];
  }
  return a;
}

class PackError extends Error {}

function main(argv) {
  const a = parseArgs(argv);
  if (!a.root || !a.tsv || !a.out) {
    console.log('usage: pack-seed.mjs --root ROOT --tsv TSV [--extra FILE]... --out PACK');
    return 2;
  }
  const root = path.resolve(a.root);
  const entries = [];
  const seen = new Map();
  const add = (packPath, host) => {
    if (!seen.has(packPath)) {
      seen.set(packPath, host);
      entries.push([packPath, host]);
    } else if (seen.get(packPath) !== host) {
      throw new PackError(`pack path ${packPath} from two different files`);
    }
  };
  const tsv = path.resolve(a.tsv);
  add(`seed/${path.basename(tsv)}`, tsv);
  const want = new Map();
  const guestSeen = new Map();
  const lines = fs.readFileSync(tsv, 'utf8').split(/\r\n|\r|\n/);
  for (let ln = 1; ln <= lines.length; ln++) {
    const line = lines[ln - 1];
    if (!line || line.startsWith('#')) continue;
    const f = line.split('\t');
    if (f.length !== 3) throw new PackError(`${tsv}:${ln}: a line of the seed list needs three fields`);
    const [g, h, s] = f;
    // the rule of vfs_seed.h (vs_parse): a guest path may be listed once, without regard to ASCII case; a directory line is compared without its trailing backslash
    const gk = (h === '-' && s === '-' && g.endsWith('\\') ? g.slice(0, -1) : g).replace(/[A-Z]/g, (c) => c.toLowerCase());
    if (guestSeen.has(gk)) throw new PackError(`vfs-seed: ${tsv}:${ln}: the guest path ${g} is listed twice (line ${guestSeen.get(gk)})`);
    guestSeen.set(gk, ln);
    if (h === '-' && s === '-') continue;
    if (h.startsWith('/') || h.split('/').includes('..')) throw new PackError(`${tsv}:${ln}: host path must be relative without '..': ${h}`);
    add(`seed/${h}`, path.join(root, h));
    want.set(`seed/${h}`, s.toLowerCase());
  }
  for (const x of a.extra) {
    add(`seed/${path.basename(path.resolve(x))}`, path.resolve(x));
  }
  const files = [], blobs = [];
  let off = 0;
  for (const [packPath, host] of entries) {
    const data = fs.readFileSync(host);
    const sha = sha256(data);
    if (want.has(packPath) && want.get(packPath) !== sha) throw new PackError(`SHA-256 mismatch for ${host}: list ${want.get(packPath)}, file ${sha}`);
    files.push({ path: packPath, size: data.length, offset: off, sha256: sha });
    blobs.push(data);
    off += data.length;
  }
  const header = Buffer.from(pyJson({ version: 1, root: 'seed', count: files.length, data_bytes: off, files }), 'utf8');
  const len = Buffer.alloc(4);
  len.writeUInt32LE(header.length);
  const pack = Buffer.concat([Buffer.from('FLSEEDP1', 'ascii'), len, header, ...blobs]);
  fs.writeFileSync(a.out, pack);
  const largest = files.map((f) => [f.size, f.path]).sort((x, y) => (x[0] !== y[0] ? y[0] - x[0] : x[1] < y[1] ? 1 : x[1] > y[1] ? -1 : 0)).slice(0, 5);
  const man = {
    pack: path.basename(a.out), bytes: pack.length, sha256: sha256(pack), header_bytes: header.length, data_bytes: off, files: files.length,
    files_checked_against_list_sha256: want.size, largest,
  };
  fs.writeFileSync(`${a.out}.manifest.json`, pyJson(man, 1));
  console.log(pyJson(man, 1));
  return 0;
}

try {
  process.exitCode = main(process.argv.slice(2));
} catch (e) {
  console.error(e instanceof PackError ? e.message : e);
  process.exitCode = 1;
}
