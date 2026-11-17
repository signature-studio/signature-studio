// perf1 (decision 0130, PROVISIONAL): one TSV line per Chrome run (and per navigation of a multi-run file) from the runner's <name>.json files.
// usage: node summarize.mjs OUT.tsv LOGDIR/NAME.json...
// Columns (all times in seconds since the page's navigation start = performance.now() of the page, unless said otherwise): wasm bytes on the wire, data bytes on the wire, wasm download+compile end (wasm_compiled),
// engine start (main_start), first non-background pixel of lid 13 drawn on canvas A/B (frame kind 1: the Worker's event time, the page's putImageData, the next requestAnimationFrame), idle stop (frame kind 2),
// the SHA checks (first, stop: module / canvas A / canvas B equal the PNG's RGBA SHA-256), the final linear memory, the renderer's peak RSS and the tree's, the net-log verdict.
import fs from 'node:fs';
const [out, ...files] = process.argv.slice(2);
const s = (x) => (x === null || x === undefined ? '' : (x / 1000).toFixed(2));
const rows = [];
const hdr = ['file', 'run', 'variant_args', 'wire_wasm_B', 'wire_data_B', 'wasm_compiled_s', 'main_start_s', 'first_px_worker_s', 'first_px_putImageData_s', 'first_px_raf_s', 'idle_worker_s', 'idle_raf_s', 'done_s', 'engine_s(main_start->exit)',
  'sha_first(mod/A/B)', 'sha_stop(mod/A/B)', 'memory_B', 'renderer_peak_MB', 'tree_peak_MB', 'tcp_nonloop', 'udp_nonloop_bytes_sent'];
for (const f of files) {
  const j = JSON.parse(fs.readFileSync(f, 'utf8'));
  const runs = j.runs && j.runs.length ? j.runs : [{ run: 1, result: j.result, requests: [] }];
  for (const r of runs) {
    const res = r.result;
    if (!res) { rows.push([f, r.run, JSON.stringify(j.args), 'NO RESULT']); continue; }
    const mk = Object.fromEntries((res.markNav || []).map((m) => [m[0], m[1]]));
    const isMain = (x) => x.main ?? x.lid === 13;   // decision 0150: the Worker flags the main window's frames; a result saved before it has no flag and its main form was lid 13 (those builds only)
    const fr1 = (res.frames || []).find((x) => x.kind === 1 && isMain(x)), fr2 = (res.frames || []).find((x) => x.kind === 2 && isMain(x));
    const reqs = r.requests || [];
    const wire = (re) => reqs.filter((q) => re.test(q.path)).reduce((t, q) => t + (q.bytes || 0), 0);
    const exp = j.compare || {};
    const chk = (k) => exp[k] ? [exp[k].equal_module, exp[k].equal_canvasA, exp[k].equal_canvasB].map((b) => (b ? 'Y' : 'N')).join('/') : '?';
    const nl = j.netlog && j.netlog.summary || '';
    const m1 = /non_loopback_tcp_connects=(\d+)/.exec(nl), m2 = /udp_bytes_sent_on_them=(\d+)/.exec(nl);
    rows.push([f.replace(/.*\//, ''), r.run, (j.args.enc ? 'enc=' + j.args.enc : '') + (j.args.rate ? ' rate=' + j.args.rate : '') + (j.args.cache ? ' cache=' + j.args.cache : '') + (j.args.streaming ? ' streaming=' + j.args.streaming : ''),
      wire(/flengine6\.wasm$/), wire(/\/data\//), s(mk.wasm_compiled), s(mk.main_start), s(fr1 && fr1.nav.navWorkerEvent), s(fr1 && fr1.nav.navDrawn), s(fr1 && fr1.nav.navRaf), s(fr2 && fr2.nav.navWorkerEvent), s(fr2 && fr2.nav.navRaf), s(res.doneNav),
      (mk.exit !== undefined && mk.main_start !== undefined) ? s(mk.exit - mk.main_start) : '', chk('first'), chk('stop'), res.memoryBytes, r.run === runs.length ? Math.round((j.rss.busiestRenderer || {}).peakRssKB / 1024) : '', r.run === runs.length ? Math.round(j.rss.peakTreeRssKB / 1024) : '',
      m1 ? m1[1] : '?', m2 ? m2[1] : '?']);
  }
}
fs.writeFileSync(out, [hdr.join('\t'), ...rows.map((r) => r.join('\t'))].join('\n') + '\n');
console.log(fs.readFileSync(out, 'utf8'));
