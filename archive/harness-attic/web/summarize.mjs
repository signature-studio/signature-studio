// One TSV row per run json of $WEB_RUNS (default /tmp/opencode/web/runs): the figures that the report quotes.
// usage: node summarize.mjs [name-prefix ...] > results-summary.tsv
import fs from 'node:fs';
import path from 'node:path';
const runs = process.env.WEB_RUNS || '/tmp/opencode/web/runs';
const prefixes = process.argv.slice(2);
const cols = ['name', 'fast', 'mode', 'enc', 'rate_MBps', 'jsflags', 'completed', 'exit', 'stdout_lines', 'lines_differing_from_node', 'fetch_data_ms', 'wasm_ready_ms', 'run_ms', 'worker_total_ms', 'renderer_peak_rss_MB', 'renderer_peak_pss_MB', 'renderer_cpu_s', 'tree_peak_rss_MB', 'wasm_server_ms', 'memory_bytes_end'];
console.log(cols.join('\t'));
for (const f of fs.readdirSync(runs).filter((x) => x.endsWith('.json') && !x.endsWith('.trace.json')).sort()) {
  const name = f.slice(0, -5);
  if (prefixes.length && !prefixes.some((p) => name.startsWith(p))) continue;
  const d = JSON.parse(fs.readFileSync(path.join(runs, f), 'utf8'));
  if (!d.result || !d.result.marks) continue;
  const m = d.result.marks;
  const b = d.rss && d.rss.busiestRenderer;
  const wasm = (d.server_log || []).find((s) => /flengine6\.wasm/.test(s.url));
  const row = {
    name, fast: d.args.fast ?? '1', mode: d.args.mode ?? 'instantiate', enc: d.args.enc ?? 'identity', rate_MBps: d.args.rate ?? '', jsflags: d.args.jsflags ?? '',
    completed: d.completed, exit: d.result.exitCode, stdout_lines: d.compare_with_node && d.compare_with_node.browser_lines,
    lines_differing_from_node: d.compare_with_node && d.compare_with_node.differing_lines,
    fetch_data_ms: m.data_fetched != null ? Math.round(m.data_fetched - m.glue_loaded) : '',
    wasm_ready_ms: m.wasm_instantiated != null ? Math.round(m.wasm_instantiated - m.wasm_start) : '',
    run_ms: m.main_end != null ? Math.round(m.main_end - m.main_start) : '', worker_total_ms: Math.round(m.done),
    renderer_peak_rss_MB: b ? Math.round(b.peakRssKB / 1024) : '', renderer_peak_pss_MB: b && b.peakPssKB ? Math.round(b.peakPssKB / 1024) : '',
    renderer_cpu_s: b ? +(b.cpuMs / 1000).toFixed(1) : '', tree_peak_rss_MB: d.rss ? Math.round(d.rss.peakTreeRssKB / 1024) : '',
    wasm_server_ms: wasm ? wasm.ms : '', memory_bytes_end: d.result.memoryBytes,
  };
  console.log(cols.map((c) => row[c] ?? '').join('\t'));
}
