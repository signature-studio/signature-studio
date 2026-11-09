// Reads the JSONL the measurement beacon writes (FL_PERF_LOG, decision 0152) and prints the summary of shared/perf-summary.mjs (the library: the tables, what they mean and how the records are read are described there).
// usage: node scripts/perf-log-summary.mjs <perf-log.jsonl> [--json] [--machine tailnet|loopback|lan|other|workers|all]
//        node scripts/perf-log-summary.mjs --dir <directory> [--json] [--machine ...]   (decision 0224: the objects the Worker stores in R2, copied to a directory, e.g.
//          aws s3 sync s3://fl-web-cache/perf/ <directory> --endpoint-url https://<account>.r2.cloudflarestorage.com ; every *.json under it is one beacon record)
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { format, joinStoredObjects, summarize } from '../shared/perf-summary.mjs';

export * from '../shared/perf-summary.mjs';

/** every *.json file under a directory, as text, in a stable order (the order of the names; summarize() sorts by the records' time) */
export function readStoredObjects(dir) {
  const out = [];
  const walk = (d) => {
    for (const e of fs.readdirSync(d, { withFileTypes: true }).sort((a, b) => (a.name < b.name ? -1 : 1))) {
      const p = path.join(d, e.name);
      if (e.isDirectory()) walk(p);
      else if (e.isFile() && e.name.endsWith('.json')) out.push(fs.readFileSync(p, 'utf8'));
    }
  };
  walk(dir);
  return out;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const usage = 'usage: node scripts/perf-log-summary.mjs <perf-log.jsonl> | --dir <directory of stored beacons> [--json] [--machine tailnet|loopback|lan|other|workers|all]';
  const di = process.argv.indexOf('--dir');
  const file = di > 0 ? null : process.argv[2];
  if (di > 0 ? !process.argv[di + 1] || process.argv[di + 1].startsWith('--') : !file || file.startsWith('--')) { console.error(usage); process.exit(2); }
  const mi = process.argv.indexOf('--machine');
  let text;
  if (di > 0) {
    const dir = process.argv[di + 1];
    if (!fs.existsSync(dir) || !fs.statSync(dir).isDirectory()) { console.error(`not a directory: ${dir}`); process.exit(2); }
    text = joinStoredObjects(readStoredObjects(dir));
  } else text = fs.readFileSync(file, 'utf8');
  const s = summarize(text);
  console.log(process.argv.includes('--json') ? JSON.stringify(s, null, 2) : format(s, { machine: mi > 0 ? process.argv[mi + 1] : 'all' }));
}
