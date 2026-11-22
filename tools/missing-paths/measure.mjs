// Missing-path measurement (C-3, #1316). Starts the bundle 3 times with the start-up of tools/boot-check/boot-bundle.mjs (to the engine's first idle, then the state engine-idle) and reads the engine's log of each start-up: the lines that the runtime writes when a guest file request or a module load finds nothing (fle_vfs_note_missing: missing-path: <API> <code> "<path>").
// Usage: node tools/missing-paths/measure.mjs <absolute path to the bundle directory raw/<name>> > missing.jsonl   (needs app/node_modules and app/dist, as boot-bundle.mjs does)
// stdout: one JSON object per start-up, {"state","standIn","error","elapsedMs","missing"}; missing lists the guest path of every such line, in order (a path that is listed twice was asked for twice). stderr: the same lines with the API and the code, for reading.
// Exit 1 when a start-up failed (its "error" is set), 2 when no bundle directory is given.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { bootBundle } from '../boot-check/boot-bundle.mjs';

const RUNS = 3;
const LINE = /^(?:\[err\] )?missing-path: (\S+) (\d+) "(.*)"$/;

/** the missing-path lines of an engine log: [{ api, code, path }] (path as the runtime wrote it: units outside printable ASCII as \uXXXX) */
export function missingOf(engineLog) {
  const found = [];
  for (const line of engineLog ?? []) {
    const m = LINE.exec(line);
    if (m) found.push({ api: m[1], code: Number(m[2]), path: m[3] });
  }
  return found;
}

async function main() {
  if (!process.argv[2]) {
    process.stderr.write('usage: node tools/missing-paths/measure.mjs <absolute path to the bundle directory>\n');
    process.exit(2);
  }
  const bundleDir = path.resolve(process.argv[2]);
  let failed = false;
  for (let run = 1; run <= RUNS; run++) {
    const { out, engineLog } = await bootBundle(bundleDir);
    const found = missingOf(engineLog);
    for (const f of found) process.stderr.write(`run ${run}: ${f.api} ${f.code} "${f.path}"\n`);
    process.stdout.write(JSON.stringify({ state: out.state, standIn: out.standIn, error: out.error, elapsedMs: out.elapsedMs, missing: found.map((f) => f.path) }) + '\n');
    if (out.error) failed = true;
  }
  process.exitCode = failed ? 1 : 0;
}

// the measurement runs when this file is the program; a test that imports it gets missingOf only
const self = process.argv[1] ? fs.realpathSync(process.argv[1]) : null;
if (self === fs.realpathSync(fileURLToPath(import.meta.url))) await main();
