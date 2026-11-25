// tools/ab/base.mjs (#1422, fixed for #1423-#1427): the base tree of an A/B run. The base is the tracked tree of a ref (git archive) plus the git-ignored
// inputs that the engine and bundle builds only read (lift/out-app/host, host-extra, external, and the blend2d tarball). The inputs are hard links to the
// working tree (not copies: host is 858 MB); the builds write their output into their own directories, and the links are checked after the build.
// A write into a link changes the working tree's file, so verify reports every linked file whose size, times or inode changed since the preparation.
// Usage: node tools/ab/base.mjs <ref> <dest> [path...] prepares <dest> from <ref> (only the given paths of the tracked tree, when there are some; stdout: one JSON line)
//        node tools/ab/base.mjs --verify <dest>        exit 0 when no linked input changed, 1 when one did (or there is no manifest)
// The manifest is <dest>-inputs.json (beside the tree, not in it).
import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const REQUIRED_DIRS = ['lift/out-app/host'];
const OPTIONAL_DIRS = ['lift/out-app/host-extra', 'lift/out-app/external'];
const OPTIONAL_FILE = /^blend2d-[0-9a-f]+\.tar\.gz$/;
const LINK_FALLBACK = new Set(['EXDEV', 'EPERM', 'EMLINK', 'ENOTSUP', 'EACCES']);

// the tracked files of ref (under the given paths when there are some) into dest
function gitArchive(tree, ref, dest, paths) {
  execFileSync('bash', ['-c', 'ref=$1; dest=$2; shift 2; git archive "$ref" "$@" | tar -x -C "$dest"', 'base', ref, dest, ...paths], { cwd: tree, stdio: ['ignore', 'inherit', 'inherit'] });
}

// the input paths (relative to the tree) that exist: the required ones are an error when they are missing
function inputsOf(tree) {
  const found = [];
  for (const rel of REQUIRED_DIRS) {
    if (!fs.existsSync(path.join(tree, rel))) throw new Error(`${rel} is not in the working tree: the base build needs it`);
    found.push(rel);
  }
  for (const rel of OPTIONAL_DIRS) if (fs.existsSync(path.join(tree, rel))) found.push(rel);
  const outApp = path.join(tree, 'lift/out-app');
  if (fs.existsSync(outApp)) {
    for (const name of fs.readdirSync(outApp)) if (OPTIONAL_FILE.test(name)) found.push(`lift/out-app/${name}`);
  }
  return found;
}

// every file under rel (a file or a directory), relative paths, with the stat that verify compares
function filesUnder(tree, rel) {
  const out = [];
  const walk = (p) => {
    const st = fs.lstatSync(p);
    if (st.isDirectory()) {
      for (const name of fs.readdirSync(p).sort()) walk(path.join(p, name));
    } else if (st.isFile()) {
      out.push({ rel: path.relative(tree, p), size: st.size, mtimeMs: st.mtimeMs, ctimeMs: st.ctimeMs, ino: st.ino });
    }
  };
  walk(path.join(tree, rel));
  return out;
}

// link (or copy, when the link fails) every file of src into dst; directories are made in dst, symbolic links are made again
export function linkTree(src, dst, link, counts) {
  const st = fs.lstatSync(src);
  if (st.isDirectory()) {
    fs.mkdirSync(dst, { recursive: true });
    for (const name of fs.readdirSync(src).sort()) linkTree(path.join(src, name), path.join(dst, name), link, counts);
  } else if (st.isSymbolicLink()) {
    fs.symlinkSync(fs.readlinkSync(src), dst);
    counts.symlinked += 1;
  } else if (st.isFile()) {
    try {
      link(src, dst);
      counts.linked += 1;
    } catch (e) {
      if (!LINK_FALLBACK.has(e.code)) throw e;
      fs.copyFileSync(src, dst);
      counts.copied += 1;
    }
  }
}

export function prepareBase({ tree, dest, ref, paths = [], archive, link = fs.linkSync }) {
  const inputs = inputsOf(tree);
  fs.rmSync(dest, { recursive: true, force: true });
  fs.mkdirSync(dest, { recursive: true });
  if (archive) archive(ref, dest, paths); else gitArchive(tree, ref, dest, paths);
  const counts = { linked: 0, copied: 0, symlinked: 0 };
  const entries = [];
  for (const rel of inputs) {
    linkTree(path.join(tree, rel), path.join(dest, rel), link, counts);
    entries.push(...filesUnder(tree, rel));
  }
  fs.writeFileSync(`${dest}-inputs.json`, JSON.stringify({ tree, dest, entries }) + '\n');
  return { dest, inputs, ...counts };
}

export function verifyBase({ tree, dest }) {
  const manifest = `${dest}-inputs.json`;
  if (!fs.existsSync(manifest)) return { ok: false, changed: [], reason: 'no manifest' };
  const { entries } = JSON.parse(fs.readFileSync(manifest, 'utf8'));
  const changed = [];
  for (const e of entries) {
    let st;
    try { st = fs.lstatSync(path.join(tree, e.rel)); } catch { changed.push(e.rel); continue; }
    if (st.size !== e.size || st.mtimeMs !== e.mtimeMs || st.ctimeMs !== e.ctimeMs || st.ino !== e.ino) changed.push(e.rel);
  }
  return { ok: changed.length === 0, changed };
}

function main(argv) {
  const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
  if (argv[0] === '--verify' && argv.length === 2) {
    const v = verifyBase({ tree: repo, dest: path.resolve(argv[1]) });
    process.stdout.write(JSON.stringify(v) + '\n');
    return v.ok ? 0 : 1;
  }
  if (argv.length < 2) {
    process.stderr.write('usage: node tools/ab/base.mjs <ref> <dest> [path...]  |  node tools/ab/base.mjs --verify <dest>\n');
    return 2;
  }
  const r = prepareBase({ tree: repo, dest: path.resolve(argv[1]), ref: argv[0], paths: argv.slice(2) });
  process.stdout.write(JSON.stringify(r) + '\n');
  return 0;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    process.exitCode = main(process.argv.slice(2));
  } catch (e) {
    process.stderr.write(`ab base: ${e.message}\n`);
    process.exitCode = 1;
  }
}
