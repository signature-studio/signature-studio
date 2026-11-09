// node run-ma.mjs [coi=1|0] [tests=...] [n=10000] [name=ma-coi]   M-A measurements in one headless Chrome (results/<name>.json)
import { withBrowser, argv } from './lib.mjs';
const coi = argv.coi !== '0';
const name = argv.name || (coi ? 'ma-coi' : 'ma-plain');
const q = new URLSearchParams();
if (argv.tests) q.set('tests', argv.tests);
if (argv.n) q.set('n', argv.n);
const out = await withBrowser({ name, page: `/${coi ? 'coi' : 'plain'}/ma.html?${q}`, timeoutS: +(argv.timeout || 600), args: argv.flags ? argv.flags.split(' ') : [] }, async ({ waitDone, console }) => {
  const r = await waitDone(+(argv.timeout || 600));
  return r;
});
console.log(JSON.stringify({ name, error: out.error, chrome_pids_left: out.chrome_pids_left, env: out.result && out.result.env }, null, 1));
