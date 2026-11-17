import fs from 'node:fs';
import { withBrowser, OUT } from './lib.mjs';
const o = await withBrowser({ name: 'jspi-pthread', page: '/coi/jspi_pthread.html', timeoutS: 60 }, async ({ waitDone }) => waitDone(60));
const txt = `load ${o.loadavg_before} -> ${o.loadavg_after}\n` + (o.result || []).join('\n') + '\nerror: ' + (o.error || 'none') + '\n';
console.log(txt); fs.writeFileSync(OUT + '/logs/jspi-pthread.txt', txt);
