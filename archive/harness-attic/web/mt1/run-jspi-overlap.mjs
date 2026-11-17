// node run-jspi-overlap.mjs cases="seq:4:3500,jspi:4:3500,jspi:8:3500,jspi:4:0,seq:4:0" blocks=200 reps=3 tag=jo   (case = mode:K:workUs)
import fs from 'node:fs';
import path from 'node:path';
import { withBrowser, argv, OUT } from './lib.mjs';
const cases = (argv.cases || 'seq:4:3500,jspi:4:3500').split(',').map((s) => s.split(':')), blocks = +(argv.blocks || 200), reps = +(argv.reps || 3), tag = argv.tag || 'jo';
const rows = [];
for (let rep = 0; rep < reps; rep++) for (const [mode, k, work] of cases) {
  const name = `${tag}-${mode}-k${k}-w${work}-r${rep}`;
  const o = await withBrowser({ name, page: `/coi/jspi-overlap.html?mode=${mode}&k=${k}&work=${work}&blocks=${blocks}`, timeoutS: 120 }, async ({ waitDone }) => waitDone(120));
  if (o.error) { console.error(name, 'ERROR', o.error.slice(0, 300)); continue; }
  const r = o.result;
  const row = { name, mode, k: +k, workUs: +work, blocks, load: o.loadavg_before.split(' ')[0], perBlockMs: +r.perBlockMs.toFixed(3), idealParMs: +(work / 1000).toFixed(2), seqMs: +(k * work / 1000).toFixed(2), stepsOK: r.steps === r.expectedSteps };
  rows.push(row); console.log(JSON.stringify(row));
}
fs.mkdirSync(path.join(OUT, 'results'), { recursive: true });
fs.writeFileSync(path.join(OUT, 'results', `${tag}-summary.json`), JSON.stringify(rows, null, 1));
