#!/usr/bin/env node
// Extracts, from the FL Studio NSIS installer, the product files that the engine bundle build reads, into HOST_ROOT, and checks every file's SHA-256.
// The port of ci/lib/extract_fl.py (the same rows, the same checks, the same manifest); the NSIS reading is in lib/nsis.mjs and lib/nsis-script.mjs.
//
//   node app/scripts/extract-fl.mjs INSTALLER HOST_ROOT --dlls ci/extract-dlls.tsv --seed lift/harness/seeds/vfs-seed-idle1.tsv [--manifest OUT.sha256]
//
//   * --dlls: the engine DLL and the bundled DLLs, found by name in the installer's EXTRACTFILE list, each with the SHA-256 it must have.
//   * --seed: every row of the VFS seed list that has a host file (guest path TAB host path TAB SHA-256): the guest path is looked up in the install tree of the NSIS
//             script (decision 0129), the stream is extracted to HOST_ROOT/<host path>.
//             Exception (decision 0236): a row whose host path is under lift/harness/seeds/os-fonts/ names a file of THIS repository: it is copied to HOST_ROOT/<host path>
//             and checked against the row's SHA-256 like the others.
// The installer is only read. Nothing is written when a SHA-256 differs: the run stops with exit 2.
// --manifest writes `SHA-256  relative path` for every written file (sorted): its SHA-256 is the identity of the extraction.
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { Installer, extractFiles, parseBlocks, readHeader, streamFile } from './lib/nsis.mjs';
import { Script, installTree } from './lib/nsis-script.mjs';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const REPO_ROWS = 'lift/harness/seeds/os-fonts/';
const sha256 = (data) => crypto.createHash('sha256').update(data).digest('hex');

function rows(file) {
  return fs.readFileSync(file, 'utf8').split(/\r\n|\r|\n/)
    .filter((ln) => ln.trim() !== '' && !ln.startsWith('#'))
    .map((ln) => ln.split('\t'));
}

function place(dest, data) {
  fs.mkdirSync(path.dirname(dest), { recursive: true });
  fs.writeFileSync(`${dest}.part`, data);
  fs.renameSync(`${dest}.part`, dest);
}

function main(argv) {
  if (argv.length < 3 || !argv.includes('--dlls') || !argv.includes('--seed')) {
    console.log('usage: extract-fl.mjs INSTALLER HOST_ROOT --dlls TSV --seed TSV [--manifest OUT]');
    return 64;
  }
  const [instFile, rootArg] = argv;
  const opt = (k) => (argv.includes(k) ? argv[argv.indexOf(k) + 1] : undefined);
  const dlls = opt('--dlls'), seed = opt('--seed'), manifest = opt('--manifest');
  const root = path.resolve(rootArg);
  const written = new Map();
  const inst = new Installer(instFile);
  try {
    const { hdr, info } = readHeader(inst);
    const entries = extractFiles(hdr, parseBlocks(hdr).blocks);
    const ds = info.dataStart;
    for (const [name, rel, want] of rows(dlls)) {
      const cands = [...new Set(entries.filter(([, n]) => n.toLowerCase().endsWith(name.toLowerCase())).map(([, , off]) => off))].sort((a, b) => a - b);
      let ok = false;
      for (const off of cands) {
        const data = streamFile(inst, ds, off);
        if (sha256(data) === want) {
          place(path.join(root, rel), data);
          ok = true;
          break;
        }
      }
      if (!ok) {
        console.error(`extract_fl: ${name}: ${cands.length} candidate stream(s), none has SHA-256 ${want}`);
        return 2;
      }
      written.set(rel, want);
    }
    const tree = new Map();
    for (const [, p, off] of installTree(new Script(inst))) tree.set(p.toLowerCase(), off);
    let n = 0;
    for (const r of rows(seed)) {
      if (r.length < 3 || r[1] === '-') continue;
      const [guest, rel, want] = r;
      let data;
      if (rel.startsWith(REPO_ROWS)) {
        data = fs.readFileSync(path.join(ROOT, rel));
      } else {
        const off = tree.get(guest.toLowerCase());
        if (off === undefined) {
          console.error(`extract_fl: seed row ${guest} is not in the install tree of the NSIS script`);
          return 2;
        }
        data = streamFile(inst, ds, off);
      }
      const got = sha256(data);
      if (got !== want) {
        console.error(`extract_fl: ${guest} has SHA-256 ${got}, the seed list says ${want}`);
        return 2;
      }
      place(path.join(root, rel), data);
      written.set(rel, want);
      n++;
    }
    if (manifest) {
      const lines = [...written.keys()].sort().map((rel) => `${written.get(rel)}  ${rel}\n`);
      fs.writeFileSync(manifest, lines.join(''));
    }
    console.log(`extract_fl: ${written.size - n} DLL(s) + ${n} seed file(s) written under ${root}, all SHA-256 verified`);
    return 0;
  } finally {
    inst.close();
  }
}

process.exitCode = main(process.argv.slice(2));
