// lat1: exits when the given file exists (or after --max minutes), so that a background job's completion notice can stand for "the result is there". usage: node tools/waitfile.mjs FILE [maxMinutes]
import fs from 'node:fs';
const f = process.argv[2], max = Number(process.argv[3] ?? '120');
const t0 = Date.now();
while (!fs.existsSync(f) && Date.now() - t0 < max * 60000) await new Promise((r) => setTimeout(r, 5000));
console.log(fs.existsSync(f) ? `exists: ${f}` : `timeout waiting for ${f}`);
