// node run-rtt.mjs cases="worker:park:park:1:0,worker:spin:spin:1:0,main-waitAsync:park:park:1:0,postMessage:park:park:1:0" rounds=5000 reps=3 tag=rtt
// case = producer:mode:pluginMode:K:workUs
import fs from 'node:fs';
import path from 'node:path';
import { withBrowser, argv, OUT } from './lib.mjs';
const cases = (argv.cases || 'worker:park:park:1:0').split(',').map((s) => s.split(':'));
const rounds = +(argv.rounds || 5000), reps = +(argv.reps || 3), tag = argv.tag || 'rtt';
const rows = [];
for (let rep = 0; rep < reps; rep++) for (const [prod, mode, pmode, k, work] of cases) {
  const name = `${tag}-${prod}-${mode}-${pmode}-k${k}-w${work}-r${rep}`;
  const o = await withBrowser({ name, page: `/coi/rtt.html?prod=${prod}&mode=${mode}&pmode=${pmode}&k=${k}&work=${work}&rounds=${rounds}`, timeoutS: 120 }, async ({ waitDone }) => waitDone(120));
  if (o.error) { console.error(name, 'ERROR', o.error.slice(0, 300)); continue; }
  const r = o.result;
  const row = { name, prod, mode, pmode, k: +k, workUs: +work, rounds, load_start: o.loadavg_before.split(' ')[0], load_end: o.loadavg_after.split(' ')[0], hc: r.hardwareConcurrency, p50: +r.us.p50.toFixed(1), p90: +r.us.p90.toFixed(1), p99: +r.us.p99.toFixed(1), p999: +r.us.p99.toFixed(1) && +r.us['p99.9'].toFixed(1), max: +r.us.max.toFixed(1), mean: +r.us.mean.toFixed(1), over1ms: r.over1ms, over5ms: r.over5ms };
  rows.push(row); console.log(JSON.stringify(row));
}
fs.mkdirSync(path.join(OUT, 'results'), { recursive: true });
fs.writeFileSync(path.join(OUT, 'results', `${tag}-summary.json`), JSON.stringify(rows, null, 1));
