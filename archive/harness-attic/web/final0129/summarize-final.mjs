// final0129 (decision 0134, PROVISIONAL): one TSV line per Chrome navigation from the runner's <name>.json files (run-chrome-final.mjs).
// usage: node summarize-final.mjs OUT.tsv LOGDIR/NAME.json...
// Times are SECONDS SINCE THE PAGE'S NAVIGATION START (performance.timeOrigin of the page). Columns: (i) pack_fetched = the seed pack's bytes in hand in the Worker; data_fetched = pack + the six DLLs; (ii) wasm_ready = WebAssembly.instantiate done
// (mark wasm_instantiated; wasm_compiled = compile promise settled); memfs_ready; main_start; (iii) first_px_worker = the Worker's event for the first non-background pixel, first_px_raf = the first rAF after the page drew it on canvas B;
// (iv) stop_worker / stop_raf = the same for the stop surface (wall 16); exit = the exit mark; then wire bytes (what the server sent, from the request log), linear memory at the end, renderer / tree peak RSS, SHA checks (module / canvas A / canvas B),
// the net-log verdict (TCP non-loopback, UDP bytes sent).
import fs from 'node:fs';
const [out, ...files] = process.argv.slice(2);
const s = (x) => (x === null || x === undefined ? '' : (x / 1000).toFixed(2));
const hdr = ['file', 'run', 'args', 'pack_fetched_s', 'data_fetched_s', 'wasm_compiled_s', 'wasm_ready_s', 'memfs_ready_s', 'main_start_s', 'first_px_worker_s', 'first_px_raf_s', 'stop_worker_s', 'stop_raf_s', 'exit_s', 'done_s',
  'wire_wasm_B', 'wire_pack_B', 'wire_dlls_B', 'memory_B', 'renderer_peak_MB', 'tree_peak_MB', 'sha_first(mod/A/B)', 'sha_stop(mod/A/B)', 'tcp_nonloop', 'udp_bytes_sent'];
const rows = [];
for (const f of files) {
  const j = JSON.parse(fs.readFileSync(f, 'utf8'));
  const runs = j.runs && j.runs.length ? j.runs : [];
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
    const nl = (j.netlog && j.netlog.summary) || '';
    const m1 = /non_loopback_tcp_connects=(\d+)/.exec(nl), m2 = /udp_bytes_sent_on_them=(\d+)/.exec(nl);
    const last = r.run === runs.length;
    rows.push([f.replace(/.*\//, ''), r.run, ['enc', 'rate', 'cache', 'cacheapi', 'par', 'streaming'].filter((k) => j.args[k]).map((k) => k + '=' + j.args[k]).join(' '),
      s(mk['fetched:seedpack.bin']), s(mk.data_fetched), s(mk.wasm_compiled), s(mk.wasm_instantiated), s(mk.memfs_ready), s(mk.main_start),
      s(fr1 && fr1.nav.navWorkerEvent), s(fr1 && fr1.nav.navRaf), s(fr2 && fr2.nav.navWorkerEvent), s(fr2 && fr2.nav.navRaf), s(mk.exit), s(res.doneNav),
      wire(/flengine6\.wasm(\.gz)?$/), wire(/seedpack\.bin(\.gz)?$/), wire(/\.dll(\.gz)?$/), res.memoryBytes, last ? Math.round((j.rss.busiestRenderer || {}).peakRssKB / 1024) : '', last ? Math.round(j.rss.peakTreeRssKB / 1024) : '',
      chk('first'), chk('stop'), m1 ? m1[1] : '?', m2 ? m2[1] : '?']);
  }
}
fs.writeFileSync(out, [hdr.join('\t'), ...rows.map((r) => r.join('\t'))].join('\n') + '\n');
console.log(fs.readFileSync(out, 'utf8'));
