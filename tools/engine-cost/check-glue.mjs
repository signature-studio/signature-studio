// Checks that sed with lift/harness/web/glue.sed makes the same engine-glue.js that fe-build's glue() wrote from the baseline's
// lift/out-app/web/flengine6.js. build-variant.mjs makes the variants' glue with that sed, so the two must be byte for byte equal.
// Usage: node tools/engine-cost/check-glue.mjs
// Exit 0 when equal; 1 when not (the first differing line is printed); 2 when an input is missing.
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

process.chdir(path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..'));

const FROM = 'lift/out-app/web/flengine6.js';
const EXPECTED = 'lift/out-app/web/engine-glue.js';
const SED_SCRIPT = 'lift/harness/web/glue.sed';

for (const file of [FROM, EXPECTED, SED_SCRIPT]) {
  if (!fs.existsSync(file)) {
    process.stderr.write(`check-glue: ${file} is missing\n`);
    process.exit(2);
  }
}
const sed = spawnSync('sed', ['-E', '-f', SED_SCRIPT, FROM], { maxBuffer: 1 << 30 });
if (sed.error || sed.status !== 0) {
  process.stderr.write(`check-glue: sed failed: ${sed.error?.message ?? `exit ${sed.status}`}\n${sed.stderr}`);
  process.exit(1);
}
const expected = fs.readFileSync(EXPECTED);
if (sed.stdout.equals(expected)) {
  process.stdout.write(`check-glue: sed output equals ${EXPECTED} (${expected.length} bytes)\n`);
  process.exit(0);
}
const got = sed.stdout.toString('utf8').split('\n');
const want = expected.toString('utf8').split('\n');
let line = 0;
while (line < got.length && line < want.length && got[line] === want[line]) line++;
process.stdout.write(`check-glue: differs: sed ${sed.stdout.length} bytes, ${EXPECTED} ${expected.length} bytes; first differing line ${line + 1}\n`);
process.stdout.write(`  sed:  ${JSON.stringify(got[line] ?? '')}\n  want: ${JSON.stringify(want[line] ?? '')}\n`);
process.exit(1);
