// Reads the JSONL the measurement beacon writes (FL_PERF_LOG, decision 0152) and prints one line per record, then the numbers against the targets P1-P3 of docs/milestones/performance-targets.md,
// then (decision 0192) a table per kind of machine / browser / screen rate: whose numbers are these, how fast did the engine start, how fast did the screen run, how many frames were slow.
// usage: node scripts/perf-log-summary.mjs <perf-log.jsonl> [--json] [--machine tailnet|loopback|lan|other|all]
// Only what was measured is printed; a stand-in's records are marked STAND-IN and are not counted in the verdict lines. A record of beacon version 1 has no screen measurement: its columns are dashes.
// Input -> update (decision 0202): a record of beacon version 3 measures it from the DOM event's own timeStamp to the next animation frame after the answer was drawn (`input.eventToRaf`): that number is the P2 verdict (p95 <= 17 ms).
// A record of version 1 or 2 measured it from the page's listener to the putImageData (10-25 ms below what a person feels: decision 0194): it is shown under its own name (listener->put), in its own column, and is NOT compared with 17 ms.
// Engine stops (decision 0209): a record of the same version 3 may carry `engine.exit.message`, the engine's last words (at most 400 characters, personal parts removed by the page). The summary counts RUNS by how they ended
// (exit reason, code and message) and prints the table, the most frequent first: that is the list of walls the people who use the page run into. A run whose exit has no message (an older page, or the engine printed none) is counted under that name.
// Whose machine: remote "tailnet" is the owner's (also behind `tailscale serve`, which the server classifies by X-Forwarded-For since decision 0192); "loopback" is this server's own machine (the automated headless Chrome runs; a record of
// an older server behind `tailscale serve` would also say loopback, so a loopback record with a real GPU / a non-headless browser is marked "loopback, not headless").

// decision 0224: this is the library (no Node import): scripts/perf-log-summary.mjs is the command line (a file, or a directory of the objects the Worker stores in R2) and worker/perf-summary.ts is GET /__perf/summary.
// Records of the Worker (remote "workers", decision 0224): the visitor reached the page through Cloudflare Workers behind Access; the class of the peer address is not kept there, only this word.

const f1 = (x) => (typeof x === 'number' ? (x < 10 ? x.toFixed(1) : x.toFixed(0)) : '-');
const f2 = (x) => (typeof x === 'number' ? x.toFixed(2) : '-');
const sec = (x) => (typeof x === 'number' ? x.toFixed(1) : '-');
const pct = (x) => (typeof x === 'number' ? `${(100 * x).toFixed(x < 0.1 ? 1 : 0)}%` : '-');
const nums = (a) => a.filter((x) => typeof x === 'number' && Number.isFinite(x));
const median = (a) => { const v = nums(a).sort((x, y) => x - y); return v.length ? (v.length % 2 ? v[v.length >> 1] : (v[(v.length >> 1) - 1] + v[v.length >> 1]) / 2) : null; };
const minOf = (a) => { const v = nums(a); return v.length ? Math.min(...v) : null; };
const maxOf = (a) => { const v = nums(a); return v.length ? Math.max(...v) : null; };
/** the common-rate class of a measured Hz (the same bands as src/perf/display.ts hzClass; below 50 Hz: throttled) */
export function hzClass(hz) {
  if (typeof hz !== 'number' || !(hz >= 50)) return null;
  let best = null, bd = Infinity;
  for (const r of [60, 75, 90, 100, 120, 144, 165, 240, 360]) { const d = Math.abs(Math.log(hz / r)); if (d < bd) { bd = d; best = r; } }
  return bd < 0.06 ? best : null;
}

/** name and major version out of a user agent (for a record without the page's own parse: beacon version 1) */
function parseUa(ua) {
  const m = /Edg(?:e|A|iOS)?\/(\d+)/.exec(ua) ?? /OPR\/(\d+)/.exec(ua) ?? /Firefox\/(\d+)/.exec(ua) ?? /(?:HeadlessChrome|Chrome|CriOS)\/(\d+)/.exec(ua) ?? /Version\/(\d+).*Safari\//.exec(ua);
  const name = /Edg(?:e|A|iOS)?\//.test(ua) ? 'Edge' : /OPR\//.test(ua) ? 'Opera' : /Firefox\//.test(ua) ? 'Firefox' : /(?:HeadlessChrome|Chrome|CriOS)\//.test(ua) ? 'Chrome' : /Version\/.*Safari\//.test(ua) ? 'Safari' : 'unknown';
  return { name, major: m ? Number(m[1]) : null, headless: /HeadlessChrome/.test(ua) };
}

const SOFTWARE_GPU = /SwiftShader|llvmpipe|Software|softpipe/i;

/** S-cbeacon1-machine: who ran it */
export function machineOf(remote, headless) {
  if (remote === 'tailnet') return 'tailnet (the owner via the tailnet)';
  if (remote === 'loopback') return headless ? 'loopback, headless (this machine, automated)' : 'loopback, not headless (this machine, or an older server behind tailscale serve)';
  if (remote === 'lan') return 'lan';
  if (remote === 'workers') return headless ? 'workers, headless (an automated browser through Cloudflare Workers)' : 'workers (through Cloudflare Workers, behind Access)';
  return 'other';
}

// decision 0211: the audio block's detail (PageAudio.detail()) as the summary shows it; null for a record without the status (an older page, or ?audio=0)
function audioOf(a) {
  const d = a?.detail;
  if (!d || typeof d.status !== 'string') return null;
  const n = (x) => (typeof x === 'number' && Number.isFinite(x) ? x : null);
  return {
    status: d.status, ctxState: typeof d.ctxState === 'string' ? d.ctxState : null, sampleRate: n(d.sampleRate), baseLatencyMs: n(d.baseLatencyMs), outputLatencyMs: n(d.outputLatencyMs),
    autoplayPolicy: typeof d.autoplayPolicy === 'string' ? d.autoplayPolicy : null, gestures: n(d.gestures), resumeAttempts: n(d.resumeAttempts), lastResume: typeof d.lastResume?.result === 'string' ? d.lastResume.result : null,
    softSinkFrames: n(d.softSinkFrames), underrunQuanta: n(d.underrunQuanta), underrunLast60s: n(d.underrunQuantaLast60s), fillMinFrames: n(d.fillMinFrames), playingSeconds: n(d.playingSeconds), nonSilentQuanta: n(d.nonSilentQuanta), peak: n(d.peak),
  };
}
const audioText = (r) => {
  const a = r.audio;
  if (!a) return '';
  return ` [audio ${a.status}: ctx ${a.ctxState ?? '?'} ${a.sampleRate ?? '?'} Hz, latency base ${a.baseLatencyMs ?? '?'} out ${a.outputLatencyMs ?? '?'} ms, resume ${a.lastResume ?? 'none'} x${a.resumeAttempts ?? '?'} after ${a.gestures ?? '?'} gestures, ring fill min ${a.fillMinFrames ?? '?'} frames, underrun quanta ${a.underrunQuanta ?? '?'}, non-silent ${a.nonSilentQuanta ?? '?'}, peak ${a.peak ?? '?'}${a.softSinkFrames ? `, stand-in clock moved ${a.softSinkFrames} frames` : ''}]`;
};

export function summarize(text) {
  const recs = [];
  const bad = [];
  text.split('\n').forEach((line, i) => {
    if (!line.trim()) return;
    try { recs.push(JSON.parse(line)); } catch { bad.push(i + 1); }
  });
  const rows = recs.map((r) => {
    const p = r.perf ?? {};
    const b = p.browser ?? {};
    const ua = b.userAgent ?? '';
    const c = p.client?.browser ?? null;
    const parsed = c ? { name: c.name, major: c.major, headless: c.headless === true } : parseUa(ua);
    const d = p.display ?? null;
    // the steady-state window: since the first idle when it has enough intervals, else the whole run
    const rafWin = d?.raf?.sinceIdle && d.raf.sinceIdle.n >= 30 ? d.raf.sinceIdle : d?.raf?.total ?? null;
    const canvasWin = d?.canvas ? (d.canvas.sinceIdle && d.canvas.sinceIdle.ticks >= 30 ? d.canvas.sinceIdle : d.canvas.total) : null;
    const gpu = b.webglRenderer ?? null;
    // decision 0202: version 3 has both numbers under their names; a record of version 1 or 2 has the flat listener -> put number only
    const inp = p.input ?? null;
    const v3 = typeof p.v === 'number' && p.v >= 3 && inp && typeof inp.eventToRaf === 'object' && inp.eventToRaf !== null;
    const answeredOnly = v3 ? inp.eventToRaf : null;
    // decision 0205: with ?dedupe=1 the verdict is over the answered inputs and the late visible changes (eventToRafWithLate); the answered-only number is kept beside it
    const withLate = v3 && inp.eventToRafWithLate && typeof inp.eventToRafWithLate === 'object' && inp.eventToRafWithLate.n > 0 ? inp.eventToRafWithLate : null;
    const eventToRaf = withLate ?? answeredOnly;
    const listenerToPut = v3 ? (inp.listenerToPut ?? null) : inp && typeof inp.p95Ms === 'number' ? inp : null;
    const headless = parsed.headless || (typeof gpu === 'string' && SOFTWARE_GPU.test(gpu));
    return {
      t: r.t, remote: r.remote, reason: p.reason, seq: p.seq, sid: typeof p.sid === 'string' ? p.sid : null, v: p.v ?? 1, kind: p.bundle?.engineKind ?? null, bundle: p.bundle?.name ?? null, standIn: p.bundle?.standIn === true,
      firstIdleSeconds: p.startup?.firstIdleSeconds ?? null, updatesPerSecond: p.ui?.updates?.recent10s?.perSecond ?? null, updateP95Ms: p.ui?.updates?.recent10s?.intervalP95Ms ?? null,
      inputP95Ms: eventToRaf?.p95Ms ?? null, inputP50Ms: eventToRaf?.p50Ms ?? null, inputN: eventToRaf?.n ?? 0, inputStages: v3 ? (inp.stages?.p50 ?? null) : null,
      // decision 0205: the processed inputs by how they were answered (a record from before it has none of these)
      inputBasis: withLate ? 'answered+late' : 'answered', inputAnsweredP95Ms: answeredOnly?.p95Ms ?? null, inputAnsweredP50Ms: answeredOnly?.p50Ms ?? null, inputAnsweredN: answeredOnly?.n ?? 0,
      inputLate: v3 && inp.lateVisible ? { n: inp.lateVisible.n, p50Ms: inp.lateVisible.p50Ms, p95Ms: inp.lateVisible.p95Ms } : null,
      inputCounts: v3 && typeof inp.noAnswer === 'number' ? { processed: inp.processed ?? null, answered: inp.answered ?? null, late: inp.late ?? null, noAnswer: inp.noAnswer } : null,
      listenerToPutP95Ms: listenerToPut?.p95Ms ?? null, listenerToPutN: listenerToPut?.n ?? 0, inputDefinition: v3 ? 'event->rAF' : 'listener->put',
      underruns: p.audio?.underruns ?? null,
      audio: audioOf(p.audio),
      cores: b.hardwareConcurrency ?? null, memGB: b.deviceMemoryGB ?? null, gpu, ua, screen: b.screen ? `${b.screen.width}x${b.screen.height}@${b.devicePixelRatio ?? '?'}` : null, error: p.error?.what?.split('\n')[0] ?? null,
      browser: parsed.name === 'unknown' ? 'unknown' : `${parsed.name}${parsed.major !== null ? ` ${parsed.major}` : ''}`, headless,
      os: p.client?.browser?.os ?? null, hostKind: p.client?.page?.hostKind ?? null,
      hasDisplay: d !== null,
      rafHz: rafWin?.hz ?? null, rafPeakHz: rafWin?.peakHz ?? null, rafP95Ms: rafWin?.p95Ms ?? null, rafMaxMs: rafWin?.maxMs ?? null, rafSlow: rafWin?.slow ?? null, rafN: rafWin?.n ?? 0, rafPerSecond: rafWin?.perSecond ?? null,
      rafOver12p5: rafWin?.over12p5 ?? null,
      canvasFreshRatio: canvasWin?.freshRatio ?? null, canvasFreshPerSecond: canvasWin?.freshPerSecond ?? null, canvasDrawsPerSecond: canvasWin?.drawsPerSecond ?? null, drawP95Ms: canvasWin?.drawMs?.p95 ?? null,
      longTasksSinceIdle: d?.longTasks?.sinceIdle ?? null, slowEvents: d?.events?.slow ?? null, hiddenMs: d?.visibility?.hiddenMs ?? null, visibleMs: d?.visibility?.visibleMs ?? null,
      engineHz: p.hz?.engineHz ?? null, hzParam: p.hz?.param ?? null, hzFit: p.hz?.fit ?? null,
      exitReason: typeof p.engine?.exit?.reason === 'string' ? p.engine.exit.reason : null, exitCode: typeof p.engine?.exit?.code === 'number' ? p.engine.exit.code : null,
      exitMessage: typeof p.engine?.exit?.message === 'string' ? p.engine.exit.message : null,
      // decision 0216: the optional `files` block (counts of the file manager's use; no names); absent in older records
      files: p.files && typeof p.files === 'object' && !Array.isArray(p.files) ? p.files : null,
      // decision 0220: the optional `assets` block (where the four start-up assets came from, and the times) and the download window of the timeline
      assets: p.assets && typeof p.assets === 'object' && !Array.isArray(p.assets) && Array.isArray(p.assets.items) ? p.assets : null,
      downloadMs: typeof p.timeline?.download_done === 'number' && typeof p.timeline?.download_start === 'number' ? p.timeline.download_done - p.timeline.download_start : null,
      compileDoneMs: typeof p.timeline?.compile_done === 'number' ? p.timeline.compile_done : null,
      softStops: Array.isArray(p.engine?.softStops) ? p.engine.softStops.filter((e) => e && typeof e.api === 'string' && typeof e.n === 'number') : [],
    };
  });
  // runs: the records of one page load. A record with a session id is grouped by it; a version 1 record has none, so a run starts at its seq 0 (per remote and user agent)
  const runs = new Map();
  const open = new Map();
  let counter = 0;
  for (const r of rows) {
    let key;
    if (r.sid) key = `${r.remote}|sid:${r.sid}`;
    else {
      const k = `${r.remote}|${r.ua}`;
      if (r.seq === 0 || !open.has(k)) open.set(k, ++counter);
      key = `${k}|n${open.get(k)}`;
    }
    if (!runs.has(key)) runs.set(key, []);
    runs.get(key).push(r);
  }
  const runList = [...runs.values()].map((rs) => {
    const real = rs.filter((r) => !r.standIn && r.kind === 'emscripten-fruity-v1');
    const last = (field) => { for (let i = rs.length - 1; i >= 0; i--) if (rs[i][field] !== null && rs[i][field] !== undefined && !rs[i].standIn) return rs[i][field]; return null; };
    const lastReal = real.length ? real[real.length - 1] : null;
    return {
      records: rs.length, real: real.length, standIn: rs.every((r) => r.standIn), remote: rs[0].remote, browser: rs[0].browser, headless: rs.some((r) => r.headless), os: rs[0].os, screen: rs[0].screen, gpu: rs[0].gpu,
      machine: machineOf(rs[0].remote, rs.some((r) => r.headless)),
      firstIdleSeconds: minOf(rs.map((r) => r.firstIdleSeconds)), updatesPerSecond: last('updatesPerSecond'), rafHz: last('rafHz'), rafPeakHz: last('rafPeakHz'), rafP95Ms: last('rafP95Ms'), rafSlow: last('rafSlow'), rafN: last('rafN') ?? 0,
      canvasFreshRatio: last('canvasFreshRatio'), canvasFreshPerSecond: last('canvasFreshPerSecond'), drawP95Ms: last('drawP95Ms'), inputP95Ms: last('inputP95Ms'), inputStages: last('inputStages'), listenerToPutP95Ms: last('listenerToPutP95Ms'), longTasksSinceIdle: last('longTasksSinceIdle'), slowEvents: last('slowEvents'),
      hzFit: last('hzFit'), hzParam: lastReal?.hzParam ?? null, hasDisplay: rs.some((r) => r.hasDisplay), lastT: rs[rs.length - 1].t,
      exitReason: last('exitReason'), exitCode: last('exitCode'), exitMessage: last('exitMessage'), ended: rs.some((r) => r.exitReason !== null),
      files: last('files'),
      assets: last('assets'), downloadMs: last('downloadMs'), compileDoneMs: last('compileDoneMs'),
      softStops: rs.map((r) => r.softStops).filter((x) => x.length).at(-1) ?? [],
    };
  });
  const real = rows.filter((r) => !r.standIn && r.kind === 'emscripten-fruity-v1');
  const firstIdles = real.map((r) => r.firstIdleSeconds).filter((x) => typeof x === 'number');
  const rates = real.map((r) => r.updatesPerSecond).filter((x) => typeof x === 'number');
  const inputs = real.map((r) => r.inputP95Ms).filter((x) => typeof x === 'number');
  const oldInputs = real.map((r) => r.listenerToPutP95Ms).filter((x) => typeof x === 'number');
  return {
    records: rows.length, unreadableLines: bad, real: real.length, standIn: rows.filter((r) => r.standIn).length, rows,
    p1: { seconds: firstIdles, target: 10, bestSeconds: firstIdles.length ? Math.min(...firstIdles) : null },
    p2: { updatesPerSecond: rates, target: 120, bestUpdatesPerSecond: rates.length ? Math.max(...rates) : null, inputP95Ms: inputs, inputTargetMs: 17, bestInputP95Ms: inputs.length ? Math.min(...inputs) : null, listenerToPutP95Ms: oldInputs },
    p3: { underruns: real.map((r) => r.underruns).filter((x) => typeof x === 'number'), statuses: countBy(real.map((r) => r.audio?.status).filter((x) => typeof x === 'string')) },
    stops: stopTable(runList.filter((r) => !r.standIn && r.real > 0)),
    files: filesTable(runList.filter((r) => !r.standIn && r.real > 0)),
    assets: assetsTable(runList.filter((r) => !r.standIn && r.real > 0)),
    softStops: softTable(runList.filter((r) => !r.standIn && r.real > 0)),
    groups: groupRuns(runList.filter((r) => !r.standIn && r.real > 0)),
    runs: runList,
  };
}

/** decision 0209: the real-engine runs by how the engine ended: [{reason, code, message, runs, browsers}], most frequent first. A run that did not end (no exit record: the page was closed or the engine still runs) is not in the table; `ended` of the total is returned beside it. */
export const NO_STOP_MESSAGE = '(no message: the engine printed none, or the page is from before decision 0209)';
function stopTable(runs) {
  const by = new Map();
  for (const r of runs) {
    if (r.exitReason === null) continue;
    const message = r.exitMessage ?? NO_STOP_MESSAGE;
    const key = `${r.exitReason}\u0000${r.exitCode}\u0000${message}`;
    if (!by.has(key)) by.set(key, { reason: r.exitReason, code: r.exitCode, message, runs: 0, browsers: new Set() });
    const e = by.get(key);
    e.runs++;
    e.browsers.add(r.browser);
  }
  return [...by.values()].map((e) => ({ reason: e.reason, code: e.code, message: e.message, runs: e.runs, browsers: [...e.browsers].sort() })).sort((a, b) => b.runs - a.runs || a.message.localeCompare(b.message));
}

/**
 * decision 0216: the file manager's use by the real-engine runs: how many runs sent a `files` block, the sums of its counters, the failure codes and, per browser, the runs and the things a browser can fail at
 * (the real folder chooser, a real folder drop, persist()). A run's block is the one in its LAST record (the counters only grow within a page load).
 */
export function filesTable(runs) {
  const withFiles = runs.filter((r) => r.files);
  const sum = (path) => withFiles.reduce((n, r) => n + (path.reduce((o, k) => (o && typeof o === 'object' ? o[k] : undefined), r.files) ?? 0), 0);
  const fail = {};
  for (const r of withFiles) for (const [k, v] of Object.entries(r.files.fail ?? {})) fail[k] = (fail[k] ?? 0) + (typeof v === 'number' ? v : 0);
  const byBrowser = new Map();
  for (const r of withFiles) {
    const b = byBrowser.get(r.browser) ?? { browser: r.browser, runs: 0, opens: 0, pickFolder: 0, dropFolders: 0, persistAsked: 0, persistGranted: 0, persistDenied: 0, persistPending: 0, engineOpen: 0, engineSave: 0, failed: 0 };
    const f = r.files;
    b.runs++; b.opens += f.opens ?? 0; b.pickFolder += f.pick?.folder ?? 0; b.dropFolders += f.drop?.folders ?? 0;
    b.persistAsked += f.persist?.asked ?? 0; b.persistGranted += f.persist?.granted ?? 0; b.persistDenied += f.persist?.denied ?? 0; b.persistPending += f.persist?.pending ?? 0;
    b.engineOpen += f.engine?.open ?? 0; b.engineSave += f.engine?.save ?? 0; b.failed += f.failed ?? 0;
    byBrowser.set(r.browser, b);
  }
  return {
    runs: runs.length, runsWithFiles: withFiles.length,
    totals: { opens: sum(['opens']), pickFiles: sum(['pick', 'files']), pickFolder: sum(['pick', 'folder']), pickZip: sum(['pick', 'zip']), dropFiles: sum(['drop', 'files']), dropFolders: sum(['drop', 'folders']), dropZips: sum(['drop', 'zips']),
      written: sum(['written']), failed: sum(['failed']), extractZips: sum(['extract', 'zips']), engineOpen: sum(['engine', 'open']), engineSave: sum(['engine', 'save']), engineFolder: sum(['engine', 'folder']), engineAnswered: sum(['engine', 'answered']), engineCancelled: sum(['engine', 'cancelled']) },
    fail, byBrowser: [...byBrowser.values()].sort((a, b) => b.runs - a.runs || a.browser.localeCompare(b.browser)),
  };
}

/**
 * decision 0220: the real-engine runs by where their start-up assets came from. A run is 'cold' when every asset came over the network, 'HTTP cache' when none did but some came from the browser's HTTP cache,
 * 'own stored copy' when every asset came from the page's OPFS copy, and 'mixed' otherwise; a run without the block is 'no assets block' (a page from before decision 0220). Per browser and class: the runs, the median first
 * idle, the median download window of the timeline (download_start -> download_done, which includes decoding and hashing), and the medians of the sums over the four assets of the read / hash / copy times and of the transferred bytes.
 * The log has no identity of a machine, so a second load of the same person is recognised only by this class, not paired with the first.
 */
export function assetsTable(runs) {
  const classOf = (a) => {
    if (!a) return 'no assets block';
    const o = a.items.map((i) => i.origin);
    if (o.length === 0) return 'no assets recorded';
    if (o.every((x) => x === 'network')) return 'cold (network)';
    if (o.every((x) => x === 'storage-cache')) return 'own stored copy';
    if (o.every((x) => x !== 'network')) return 'HTTP cache (and own stored copy)';
    return 'mixed';
  };
  const sumMs = (a, k) => (a && a.items.length ? a.items.reduce((n, i) => n + (i.ms?.[k] ?? 0), 0) : null);
  const by = new Map();
  for (const r of runs) {
    const cls = classOf(r.assets);
    const key = `${r.browser}|${cls}`;
    if (!by.has(key)) by.set(key, { browser: r.browser, cls, runs: [] });
    by.get(key).runs.push(r);
  }
  const rows = [...by.values()].map((g) => ({
    browser: g.browser, class: g.cls, runs: g.runs.length,
    firstIdleSecondsMedian: median(g.runs.map((r) => r.firstIdleSeconds)), downloadMsMedian: median(g.runs.map((r) => r.downloadMs)),
    readMsMedian: median(g.runs.map((r) => sumMs(r.assets, 'read'))), hashMsMedian: median(g.runs.map((r) => sumMs(r.assets, 'hash'))), copyMsMedian: median(g.runs.map((r) => sumMs(r.assets, 'copy'))), parseMsMedian: median(g.runs.map((r) => sumMs(r.assets, 'parse'))),
    transferMiBMedian: median(g.runs.map((r) => (r.assets && r.assets.items.length && r.assets.items.every((i) => typeof i.transfer === 'number') ? r.assets.items.reduce((n, i) => n + i.transfer, 0) / 1048576 : null))),
    hash: [...new Set(g.runs.map((r) => r.assets?.hash).filter(Boolean))].sort().join('+') || '-',
    storeFailures: g.runs.reduce((n, r) => n + (r.assets ? r.assets.items.filter((i) => i.store === 'failed').length : 0), 0),
  })).sort((a, b) => a.browser.localeCompare(b.browser) || a.class.localeCompare(b.class));
  const fails = {};
  for (const r of runs) if (r.assets) for (const i of r.assets.items) if (i.fail) fails[i.fail] = (fails[i.fail] ?? 0) + 1;
  const storage = runs.map((r) => r.assets?.storage).filter(Boolean);
  return { runs: runs.length, withBlock: runs.filter((r) => r.assets).length, rows, fails, persisted: storage.filter((x) => x.persisted === true).length, notPersisted: storage.filter((x) => x.persisted === false).length };
}

function countBy(list) {
  const o = {};
  for (const k of list) o[k] = (o[k] ?? 0) + 1;
  return o;
}

/** decision 0209: the soft stops of the real-engine runs (a -DFLE_SOFT_STOP build): per API, in how many runs it answered a documented failure and the largest count of one run; most runs first. A run's list is that of its last record that had one. */
function softTable(runs) {
  const by = new Map();
  for (const r of runs) {
    for (const e of r.softStops ?? []) {
      if (!by.has(e.api)) by.set(e.api, { api: e.api, runs: 0, maxCount: 0, total: 0 });
      const x = by.get(e.api);
      x.runs++; x.total += e.n; x.maxCount = Math.max(x.maxCount, e.n);
    }
  }
  return [...by.values()].sort((a, b) => b.runs - a.runs || b.total - a.total || a.api.localeCompare(b.api));
}

function groupRuns(runs) {
  const by = new Map();
  for (const r of runs) {
    const cls = hzClass(r.rafHz);
    const screenHz = r.rafHz === null ? 'no screen measurement' : cls !== null ? `${cls} Hz class` : `${f1(r.rafHz)} Hz (no class)`;
    const key = [r.machine, r.browser, screenHz].join('|');
    if (!by.has(key)) by.set(key, { machine: r.machine, browser: r.browser, screen: screenHz, runs: [] });
    by.get(key).runs.push(r);
  }
  return [...by.values()].map((g) => {
    const rs = g.runs;
    const slowRatio = rs.map((r) => (typeof r.rafSlow === 'number' && r.rafN > 0 ? r.rafSlow / r.rafN : null));
    return {
      machine: g.machine, browser: g.browser, screen: g.screen, runs: rs.length,
      p1: { minSeconds: minOf(rs.map((r) => r.firstIdleSeconds)), medianSeconds: median(rs.map((r) => r.firstIdleSeconds)), maxSeconds: maxOf(rs.map((r) => r.firstIdleSeconds)), metRuns: rs.filter((r) => typeof r.firstIdleSeconds === 'number' && r.firstIdleSeconds <= 10).length },
      p2: {
        updatesPerSecondMedian: median(rs.map((r) => r.updatesPerSecond)), updatesPerSecondMax: maxOf(rs.map((r) => r.updatesPerSecond)),
        rafHzMedian: median(rs.map((r) => r.rafHz)), canvasFreshPerSecondMedian: median(rs.map((r) => r.canvasFreshPerSecond)), canvasFreshRatioMedian: median(rs.map((r) => r.canvasFreshRatio)),
        slowFrameRatioMedian: median(slowRatio), rafP95MsMedian: median(rs.map((r) => r.rafP95Ms)), drawP95MsMedian: median(rs.map((r) => r.drawP95Ms)),
        inputP95MsMin: minOf(rs.map((r) => r.inputP95Ms)), listenerToPutP95MsMin: minOf(rs.map((r) => r.listenerToPutP95Ms)), inputStages: rs.map((r) => r.inputStages).filter(Boolean).at(-1) ?? null,
        inputMetRuns: rs.filter((r) => typeof r.inputP95Ms === 'number' && r.inputP95Ms <= 17).length, inputRuns: rs.filter((r) => typeof r.inputP95Ms === 'number').length, longTasksMax: maxOf(rs.map((r) => r.longTasksSinceIdle)), slowEventsMax: maxOf(rs.map((r) => r.slowEvents)),
        metRuns: rs.filter((r) => typeof r.canvasFreshPerSecond === 'number' && r.canvasFreshPerSecond >= 114).length,
      },
      hzFits: Object.fromEntries([...new Set(rs.map((r) => r.hzFit).filter(Boolean))].map((k) => [k, rs.filter((r) => r.hzFit === k).length])),
    };
  }).sort((a, b) => a.machine.localeCompare(b.machine) || a.browser.localeCompare(b.browser) || a.screen.localeCompare(b.screen));
}

/** one record's input numbers with their definition named (decision 0202) */
function inputText(r) {
  if (r.inputDefinition === 'event->rAF') {
    const st = r.inputStages, c = r.inputCounts;
    const withLate = r.inputBasis === 'answered+late';
    // decision 0205: with ?dedupe=1 the p95 includes the late inputs whose later frame changed pixels, and the answered-only number stands beside it; without it the late inputs are not in the p95
    const basis = withLate ? `; p95 over the answered inputs and the late visible changes, answered only p95 ${f1(r.inputAnsweredP95Ms)} ms p50 ${f1(r.inputAnsweredP50Ms)} ms (n=${r.inputAnsweredN})` : '';
    const none = c ? `; of ${c.processed} processed ${c.answered} have a value, ${c.late + c.noAnswer} (${c.processed ? (100 * (c.late + c.noAnswer) / c.processed).toFixed(0) : 0}%) have none (${c.late} late${r.inputLate && r.inputLate.n > 0 ? `, ${f1(r.inputLate.p50Ms)} / ${f1(r.inputLate.p95Ms)} ms p50 / p95 after the event` : ''}; ${c.noAnswer} with no frame soon enough), ${withLate ? 'the late ones are in the p95' : 'not in the p95'}` : '';
    return `input event->rAF p95 ${f1(r.inputP95Ms)} ms p50 ${f1(r.inputP50Ms)} ms (n=${r.inputN}; old listener->put p95 ${f1(r.listenerToPutP95Ms)} ms${basis}${none})${st ? ` stages p50 S0 ${f1(st.s0)} + page ${f1(st.s0b)} + toWorker ${f1(st.toWorker)} + worker ${f1(st.worker)} + fromWorker ${f1(st.fromWorker)} + put ${f1(st.put)} + rAF ${f1(st.raf)} ms` : ''}`;
  }
  return `input listener->put p95 ${f1(r.listenerToPutP95Ms)} ms (n=${r.listenerToPutN}; the old definition, v${r.v}: not event->rAF)`;
}

export function format(s, opts = {}) {
  const out = [];
  // decision 0224: `rows` limits the one-line-per-record part to the LAST n records (the Worker shows 30; the command line shows all)
  const shown = typeof opts.rows === 'number' && opts.rows >= 0 && opts.rows < s.rows.length ? s.rows.slice(s.rows.length - opts.rows) : s.rows;
  if (shown.length < s.rows.length) out.push(`(the last ${shown.length} of ${s.rows.length} record lines; the tables below count all records)`);
  for (const r of shown) {
    const screen = r.hasDisplay ? ` | rAF ${f1(r.rafHz)} Hz (${hzClass(r.rafHz) ?? '-'} class; ${f1(r.rafPerSecond)} /s; gap p95 ${f1(r.rafP95Ms)} ms max ${f1(r.rafMaxMs)} ms; slow ${r.rafSlow ?? '-'}/${r.rafN}) canvas fresh ${pct(r.canvasFreshRatio)} (${f1(r.canvasFreshPerSecond)} /s, draw p95 ${f2(r.drawP95Ms)} ms) long tasks ${r.longTasksSinceIdle ?? '-'} slow events ${r.slowEvents ?? '-'}${r.hzFit ? ` | ?hz=${r.hzParam ?? '-'} ${r.hzFit}` : ''}` : '';
    out.push(`${r.t} ${r.remote.padEnd(8)} #${r.seq} ${String(r.reason).padEnd(10)} ${r.standIn ? 'STAND-IN ' : ''}${r.kind ?? 'no-engine'} first idle ${sec(r.firstIdleSeconds)} s | updates ${f1(r.updatesPerSecond)} /s (p95 gap ${f1(r.updateP95Ms)} ms) | ${inputText(r)}${screen} | underruns ${r.underruns ?? 'not measured'}${audioText(r)} | ${r.cores ?? '?'} cores ${r.memGB ?? '?'} GB ${r.screen ?? '?'} | ${r.gpu ?? 'gpu ?'} | ${r.browser}${r.headless ? ' headless' : ''} | ${(r.ua ?? '').slice(0, 80)}${r.error ? ` | ERROR ${r.error}` : ''}${r.exitReason !== null ? ` | EXIT ${r.exitReason} code ${r.exitCode}${r.exitMessage !== null ? `: ${r.exitMessage}` : ' (no message)'}` : ''}`);
  }
  out.push(`${s.records} records (${s.real} real engine, ${s.standIn} stand-in)${s.unreadableLines.length ? `, unreadable lines: ${s.unreadableLines.join(',')}` : ''}`);
  out.push(`P1 first idle (target <= ${s.p1.target} s), real engine: ${s.p1.seconds.length ? s.p1.seconds.map(sec).join(', ') + ' s' : 'no record'}`);
  out.push(`P2 updates per second (target >= ${s.p2.target}), real engine: ${s.p2.updatesPerSecond.length ? s.p2.updatesPerSecond.map(f1).join(', ') : 'no record'}; input event->rAF p95 (version 3 records; target <= ${s.p2.inputTargetMs} ms): ${s.p2.inputP95Ms.length ? s.p2.inputP95Ms.map(f1).join(', ') + ' ms' : 'no version 3 record'}`);
  out.push(`  input listener->put p95 (the old definition of versions 1 and 2, and version 3's second number; 10-25 ms below what a person feels, decision 0194; NOT compared with the target): ${s.p2.listenerToPutP95Ms.length ? s.p2.listenerToPutP95Ms.map(f1).join(', ') + ' ms' : 'no record'}`);
  out.push(`P3 audio under-runs: ${s.p3.underruns.length ? s.p3.underruns.join(', ') : 'not measured'}`);
  out.push(`  audio output status of the real-engine records (decision 0211): ${Object.keys(s.p3.statuses).length ? Object.entries(s.p3.statuses).map(([k, v]) => `${k} x${v}`).join(', ') : 'no record carries it'}`);
  out.push('');
  const realRuns = s.runs.filter((r) => !r.standIn && r.real > 0);
  const endedRuns = realRuns.filter((r) => r.exitReason !== null);
  out.push(`Engine stops (real engine; one run = one page load; ${endedRuns.length} of ${realRuns.length} runs ended with an exit record; the engine's last words are the beacon's engine.exit.message)`);
  if (s.stops.length === 0) out.push('  no run ended');
  for (const e of s.stops) out.push(`  ${String(e.runs).padStart(3)} x ${e.reason} code ${e.code} | ${e.message} | ${e.browsers.join(', ')}`);
  if (s.softStops.length) {
    out.push('', 'Soft stops (APIs that answered a documented failure instead of stopping the engine; a -DFLE_SOFT_STOP build; per API: runs, largest count of one run, total)');
    for (const e of s.softStops) out.push(`  ${e.api.padEnd(32)} ${String(e.runs).padStart(3)} run(s)  max ${e.maxCount}  total ${e.total}`);
  }
  out.push('');
  out.push(`File manager (decision 0216; real engine; ${s.files.runsWithFiles} of ${s.files.runs} runs sent a files block: counts only, no names)`);
  if (s.files.runsWithFiles === 0) out.push('  no run used Files');
  else {
    const t = s.files.totals;
    out.push(`  opened ${t.opens}x; chooser used: files ${t.pickFiles}, folder ${t.pickFolder}, zip ${t.pickZip}; dropped: files ${t.dropFiles}, folders ${t.dropFolders}, zips ${t.dropZips}; files written ${t.written}, failed ${t.failed}; zips unpacked ${t.extractZips}`);
    out.push(`  engine dialogs: open ${t.engineOpen}, save ${t.engineSave}, folder ${t.engineFolder}; answered ${t.engineAnswered}, cancelled ${t.engineCancelled}`);
    out.push(`  failure codes: ${Object.keys(s.files.fail).length ? Object.entries(s.files.fail).sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0])).map(([k, v]) => `${k} ${v}`).join(', ') : 'none'}`);
    for (const b of s.files.byBrowser) out.push(`  ${b.browser}: ${b.runs} run(s); opened ${b.opens}; real folder chooser ${b.pickFolder}; real folder drops ${b.dropFolders}; persist() asked ${b.persistAsked} -> granted ${b.persistGranted}, denied ${b.persistDenied}, no answer in 3 s ${b.persistPending}; engine dialogs ${b.engineOpen + b.engineSave}; failed ${b.failed}`);
  }
  out.push('');
  out.push(`Start-up assets (decision 0220; real engine; ${s.assets.withBlock} of ${s.assets.runs} runs sent an assets block; one row = one browser and one way the page got its bytes; the log has no identity, so a repeat visit shows up as 'HTTP cache' or 'own stored copy', not paired with the first)`);
  if (s.assets.withBlock === 0) out.push('  no run sent it');
  for (const a of s.assets.rows) out.push(`  ${a.browser} | ${a.class} | ${a.runs} run(s) | first idle median ${sec(a.firstIdleSecondsMedian)} s | download window median ${f1(a.downloadMsMedian)} ms | sum over the 4 assets, median: read ${f1(a.readMsMedian)} ms, hash ${f1(a.hashMsMedian)} ms (${a.hash}), copy ${f1(a.copyMsMedian)} ms, pack index ${f1(a.parseMsMedian)} ms | transferred ${f1(a.transferMiBMedian)} MiB | stored-copy failures ${a.storeFailures}`);
  if (s.assets.withBlock > 0) out.push(`  stored-copy / clean-up failure codes: ${Object.keys(s.assets.fails).length ? Object.entries(s.assets.fails).sort((x, y) => y[1] - x[1] || x[0].localeCompare(y[0])).map(([k, v]) => `${k} ${v}`).join(', ') : 'none'}; browser storage protected from eviction in ${s.assets.persisted} run(s), not protected in ${s.assets.notPersisted}`);
  out.push('');
  const only = opts.machine && opts.machine !== 'all' ? s.groups.filter((g) => g.machine.startsWith(opts.machine)) : s.groups;
  out.push(`By machine / browser / screen rate (real engine; one run = one page load; per run P1 is its first idle and the P2 values are those of its LAST record; stand-in excluded; ${only.length} group(s))`);
  if (only.length === 0) out.push('  no real-engine run' + (opts.machine && opts.machine !== 'all' ? ` from machine "${opts.machine}"` : ''));
  for (const g of only) {
    out.push(`  ${g.machine} | ${g.browser} | screen: ${g.screen} | ${g.runs} run(s)`);
    out.push(`    P1 first idle s min/median/max ${sec(g.p1.minSeconds)} / ${sec(g.p1.medianSeconds)} / ${sec(g.p1.maxSeconds)} (target <= 10 s met in ${g.p1.metRuns}/${g.runs} runs)`);
    out.push(`    P2 window updates/s median ${f1(g.p2.updatesPerSecondMedian)} (max ${f1(g.p2.updatesPerSecondMax)}); rAF Hz median ${f1(g.p2.rafHzMedian)}; canvas-updating screen frames/s median ${f1(g.p2.canvasFreshPerSecondMedian)} (target >= 120, met in ${g.p2.metRuns}/${g.runs} runs), fresh ${pct(g.p2.canvasFreshRatioMedian)}`);
    out.push(`    slow frames (> 1.5 x median interval) median ${pct(g.p2.slowFrameRatioMedian)}, rAF gap p95 median ${f1(g.p2.rafP95MsMedian)} ms, draw p95 median ${f2(g.p2.drawP95MsMedian)} ms, input event->rAF p95 best ${f1(g.p2.inputP95MsMin)} ms (target <= 17, met in ${g.p2.inputMetRuns}/${g.p2.inputRuns} version 3 runs) [old listener->put p95 best ${f1(g.p2.listenerToPutP95MsMin)} ms, not judged], long tasks max ${g.p2.longTasksMax ?? '-'}, slow events max ${g.p2.slowEventsMax ?? '-'}${Object.keys(g.hzFits).length ? `; ?hz= vs screen: ${Object.entries(g.hzFits).map(([k, v]) => `${k} ${v}`).join(', ')}` : ''}`);
  }
  return out.join('\n');
}

/**
 * decision 0224: the Worker stores one beacon per R2 object (`perf/<UTC date>/<sid>-<seq>.json`, the body is one line of the same JSONL that the dev server writes). The objects of a directory (aws s3 sync) or of an R2 listing
 * are put back into one log: every object's text, in the order of the records' own time `t` and then `seq`, so that summarize() sees them as the dev server's log would show them. An object that is not a record stays in the text
 * (summarize() lists it under unreadableLines); an empty object is dropped.
 */
export function joinStoredObjects(texts) {
  const keyed = texts.map((text, i) => {
    const line = text.split('\n').find((l) => l.trim() !== '') ?? '';
    let t = '', seq = 0;
    try { const r = JSON.parse(line); t = typeof r.t === 'string' ? r.t : ''; seq = typeof r.perf?.seq === 'number' ? r.perf.seq : 0; } catch { /* kept, sorted first */ }
    return { line: text.trim(), t, seq, i };
  });
  keyed.sort((a, b) => (a.t < b.t ? -1 : a.t > b.t ? 1 : a.seq - b.seq || a.i - b.i));
  return keyed.filter((k) => k.line !== '').map((k) => k.line).join('\n') + '\n';
}
