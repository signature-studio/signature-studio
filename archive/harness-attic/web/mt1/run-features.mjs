import { withBrowser, argv } from './lib.mjs';
const coi = argv.coi ?? '1';
const o = await withBrowser({ name: 'features-coi' + coi, page: `/${coi === '1' ? 'coi' : 'plain'}/features.html`, args: ['--autoplay-policy=no-user-gesture-required'] }, async ({ waitDone }) => waitDone(60));
console.log(JSON.stringify({ error: o.error, loadavg_before: o.loadavg_before, chrome: o.chrome.product, result: o.result }, null, 1));
