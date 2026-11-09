// node run-page.mjs page=mb.html coi=1|0 name=<result name> [q=<query string>] [timeout=<s>] [flags="--a --b"] [gpu=1] [mute=0]
// Generic case: open /coi|plain/<page>?<q> in a fresh headless Chrome, wait for window.__done, write results/<name>.json.
import { withBrowser, argv } from './lib.mjs';
const coi = argv.coi !== '0';
const page = argv.page;
const name = argv.name || page.replace(/\..*$/, '') + (coi ? '-coi' : '-plain');
const out = await withBrowser({
  name, page: `/${coi ? 'coi' : 'plain'}/${page}${argv.q ? '?' + argv.q : ''}`, timeoutS: +(argv.timeout || 300),
  args: argv.flags ? argv.flags.split(' ') : [], gpu: argv.gpu === '1', mute: argv.mute !== '0',
}, async ({ waitDone }) => waitDone(+(argv.timeout || 300)));
console.log(JSON.stringify({ name, error: out.error, chrome_pids_left: out.chrome_pids_left, env: out.result && out.result.env }, null, 1));
