// wait-file.mjs PATH [MAXMIN]: exits when PATH exists (a marker written by a chain), or after MAXMIN minutes (exit 1)
import fs from 'node:fs';
const p = process.argv[2], maxMin = Number(process.argv[3] ?? 120); const t0 = Date.now();
while (!fs.existsSync(p)) { if ((Date.now() - t0) / 60000 > maxMin) { console.log('timeout'); process.exit(1); } await new Promise((r) => setTimeout(r, 3000)); }
console.log('found', p);
