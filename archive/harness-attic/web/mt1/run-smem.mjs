import fs from 'node:fs';
import { withBrowser, OUT } from './lib.mjs';
const o = await withBrowser({ name: 'smem', page: '/coi/smem.html', timeoutS: 200 }, async ({ waitDone }) => waitDone(200));
console.log(o.error || '', o.loadavg_before, o.loadavg_after);
console.log((o.result || []).join('\n'));
fs.writeFileSync(OUT + '/logs/smem.txt', `load ${o.loadavg_before} -> ${o.loadavg_after}\n` + (o.result || []).join('\n') + '\n' + (o.error || '') + '\n');
