// pw1 (decision 0186): node run-pw1.mjs serum=0,1,3,4,5,3 [ott=12 ottmod=c] [hc=4|8|16] blocks=650 reps=3 tag=name [warm=1] [mod=h123] [nice=5]
// One fresh isolated headless Chrome per repetition, started under `taskset -c <cores> nice -n <nice>` (hc=4: 0-3, hc=8: 0-7, hc=16: no taskset), the page (pw1.html) builds K Serum + M OTT Workers,
// warms them (warm=1), and an engine Worker drives all of them at 92.879 ms per block. Measured: per-block completion against the next deadline (late = the answer of block b arrives after
// (b+1)*period; "A2" = after (b+2)*period), per-plug-in process time, CPU of every thread of the Chrome tree (schedstat, ns) over the paced window, busy time of the allowed CPUs (/proc/stat),
// PSS, load average before/after. Output: results/<name>.json (raw), results/<tag>-rows.jsonl (one summary row per repetition).
import fs from 'node:fs';
import path from 'node:path';
import { withBrowser, threadNs, nsDiff, cpuStat, pssMB, rssMB, topOthers, argv, OUT, sleep, pct } from './lib.mjs';
const serum = argv.serum || '', ott = +(argv.ott || 0), ottmod = argv.ottmod || 'c', hc = +(argv.hc || 16), blocks = +(argv.blocks || 650), reps = +(argv.reps || 1), tag = argv.tag || 'pw1', warm = argv.warm ?? '1', mod = argv.mod || 'h123', nice = argv.nice ?? '5';
const warmblocks = argv.warmblocks || '150', warmms = argv.warmms || '0', settle = argv.settle || '0';
const cpus = argv.cpus ? argv.cpus : hc >= 16 ? null : `0-${hc - 1}`;   // cpus=0,2,4,6: one thread per physical core (SMT siblings are 0-1, 2-3, ...)
const prefix = [...(cpus ? ['taskset', '-c', cpus] : []), 'nice', '-n', String(nice)];
const RES = process.env.PW1_RESULTS || path.join(OUT, 'results');
fs.mkdirSync(RES, { recursive: true });
const maskList = (s) => { if (!s) return null; return s.split(',').flatMap((t) => { const [a, b] = t.split('-').map(Number); return Array.from({ length: (b ?? a) - a + 1 }, (_, i) => a + i); }); };
const mask = maskList(cpus);
const r3 = (x) => (x === undefined || x === null || Number.isNaN(x) ? null : Math.round(x * 1000) / 1000);

function analyse(res, P) {
  const B = res.cfg.BLOCKS, plugins = res.plugins, K = plugins.length;
  const E = res.engine;
  const done = E.allDone;
  const late = (A) => { const l = []; for (let b = 0; b < B; b++) if (done[b] !== null) l.push(done[b] - (b + A) * P); return l; };
  const l1 = late(1), l2 = late(2);
  const cnt = (l, th) => l.filter((x) => x > th).length;
  const events = (l) => { let n = 0, inRun = false; for (const x of l) { if (x > 0) { if (!inRun) n++; inRun = true; } else inRun = false; } return n; };
  const own = []; for (let b = 0; b < B; b++) if (done[b] !== null) own.push(done[b] - E.rel[b]);
  const lateIdx = l1.map((x, b) => (x > 0 ? b : -1)).filter((b) => b >= 0);
  const per = plugins.map((p) => {
    if (!p.T) return { label: p.label, error: p.err || 'no timing' };
    const [enter, seen, start, end] = p.T;
    const proc = [], startLag = [], wake = [], harnessOverhead = [], lateK = [];
    for (let b = 0; b < B; b++) {
      if (end[b] === null || start[b] === null) continue;
      proc.push(end[b] - start[b]); startLag.push(start[b] - b * P); wake.push(seen[b] - E.rel[b]);
      if (b > 0 && enter[b] !== null && end[b - 1] !== null) harnessOverhead.push(enter[b] - end[b - 1]);
      lateK.push(end[b] - (b + 1) * P);
    }
    const ps = pct(proc, [50, 95, 99]);
    const hash = (p.lines.find((l) => l.startsWith('RESULT out_hash')) || '').slice(0, 60);
    return { label: p.label, kind: p.kind, rc: p.rc, err: p.err, done: p.done, procMin: r3(ps.min), procP50: r3(ps.p50), procP95: r3(ps.p95), procP99: r3(ps.p99), procMax: r3(ps.max), procMean: r3(ps.mean),
      procFirst3: proc.slice(0, 3).map(r3), startLagMax: r3(Math.max(...startLag)), wakeMean: r3(wake.reduce((s, x) => s + x, 0) / wake.length), wakeMax: r3(Math.max(...wake)),
      harnessOverheadMean: r3(harnessOverhead.reduce((s, x) => s + x, 0) / (harnessOverhead.length || 1)), lateA1: cnt(lateK, 0), lateMax: r3(Math.max(...lateK)), memMB: r3(p.memBytes / 1048576), hash };
  });
  const ser = per.filter((x) => x.kind === 'serum'), ot = per.filter((x) => x.kind === 'ott');
  return {
    blocksDone: l1.length, engineStuck: E.stuck,
    ownOverrun: cnt(own, P), ownMax: r3(Math.max(...own)), lateA1: cnt(l1, 0), lateA1_skip5: cnt(l1.slice(5), 0), lateA1_first5: cnt(l1.slice(0, 5), 0), lateA1_gt5ms: cnt(l1, 5), lateA1_gt20ms: cnt(l1, 20), lateA1_gtP: cnt(l1, P), lateA2: cnt(l2, 0), lateA2_gt5ms: cnt(l2, 5), eventsA1: events(l1), eventsA2: events(l2),
    lateMaxA1: r3(Math.max(...l1)), lateMaxA2: r3(Math.max(...l2)), lateIdxFirst: lateIdx.slice(0, 12), firstBlocksLateness: l1.slice(0, 6).map(r3),
    engineDelay: r3(Math.max(...E.rel.map((r, b) => (r === null ? 0 : r - b * P)))),
    serumProcMax: r3(Math.max(0, ...ser.map((x) => x.procMax || 0))), ottProcMax: r3(Math.max(0, ...ot.map((x) => x.procMax || 0))),
    hashes: ser.map((x) => x.hash), per,
  };
}

const rows = [];
for (let rep = 0; rep < reps; rep++) {
  const name = `${tag}-r${rep + +(argv.rep0 || 0)}`;
  const q = `serum=${serum}&ott=${ott}&ottmod=${ottmod}&blocks=${blocks}&warm=${warm}&warmblocks=${warmblocks}&warmms=${warmms}&settle=${settle}&mod=${mod}`;
  const o = await withBrowser({ name: `pw1-${process.pid}-${name}`, page: `/coi/pw1/pw1.html?${q}`, prefix, timeoutS: 900 }, async ({ evalJSON, waitFor, chrome, out }) => {
    await waitFor('__ready', 600, 200);
    await sleep(1500);
    const root = chrome.proc.pid;
    const sb = threadNs(root), cb = cpuStat(), lb = fs.readFileSync('/proc/loadavg', 'utf8').trim();
    out.pssMB_before = pssMB(root); out.rssMB_before = rssMB(root);
    const tB = process.hrtime.bigint();
    const sumNs = (m) => { let t = 0; for (const v of m.values()) t += v.ns; return t; };
    const ser = []; let pc = cb, pn = sumNs(sb), pt = tB;
    const iv = setInterval(() => {
      const c = cpuStat(), n = sumNs(threadNs(root)), t = process.hrtime.bigint(), dt = Number(t - pt) / 1e9;
      const busy = mask ? mask.reduce((a, k) => a + (c['cpu' + k].busy - pc['cpu' + k].busy), 0) : c.cpu.busy - pc.cpu.busy;
      ser.push([+(busy * 0.01 / dt).toFixed(2), +((n - pn) / 1e9 / dt).toFixed(2)]); pc = c; pn = n; pt = t;
    }, 1000);
    await evalJSON('window.__go = true; true');
    await waitFor('__engineDone', 600, 50);
    clearInterval(iv); out.busySeries = ser;
    const tA = process.hrtime.bigint();
    const sa = threadNs(root), ca = cpuStat();
    out.pssMB_after = pssMB(root); out.rssMB_after = rssMB(root);
    out.window = { wallMs: Number(tA - tB) / 1e6, load_in: lb, load_out: fs.readFileSync('/proc/loadavg', 'utf8').trim() };
    out.threads = nsDiff(sb, sa).slice(0, 40);
    out.cpuAll = nsDiff(sb, sa);
    out.busyMask = {}; for (const k of Object.keys(ca)) out.busyMask[k] = ca[k].busy - cb[k].busy;
    out.others = topOthers(root).map((x) => `${x.name}:${x.nice}`).slice(0, 20);
    await waitFor('__done', 120, 100);
    const r = JSON.parse(await evalJSON('JSON.stringify(window.__result)'));
    await evalJSON('window.__terminate = true; true');
    return r;
  });
  const { out, result, error } = o;
  if (error || out.error) { console.error(name, 'ERROR', String(error || out.error).slice(0, 800)); rows.push({ name, error: String(error || out.error).slice(0, 400) }); continue; }
  const P = result.cfg.PERIOD;
  const A = analyse(result, P);
  const wall = out.window.wallMs;
  const sumBy = (f) => out.cpuAll.filter(f).reduce((s, x) => s + x.ms, 0);
  const workers = out.cpuAll.filter((x) => x.name.includes('DedicatedWorker')).map((x) => r3(x.ms));
  const workerDelay = out.cpuAll.filter((x) => x.name.includes('DedicatedWorker')).map((x) => r3(x.delayMs));
  const maskBusy = mask ? mask.reduce((s, c) => s + (out.busyMask['cpu' + c] || 0), 0) * 10 : (out.busyMask.cpu || 0) * 10;
  const fore = (out.busySeries || []).map(([m, c]) => Math.max(0, m - c)).sort((a, b) => a - b);
  const row = { name, tag, serum, ott, ottmod, hc, cpus: cpus || 'all', hcReported: result.hardwareConcurrency, blocks, warm: +warm, warmms: +warmms, warmblocks: +warmblocks, settle: +settle, mod, load_in: out.window.load_in, load_out: out.window.load_out, loadavg_start: out.loadavg_before, loadavg_end: out.loadavg_after,
    windowS: r3(wall / 1000), periodMs: r3(P), tWarmMs: Math.round(result.tWarm), tInitMs: Math.round(result.tInit),
    coresTotal: r3(sumBy(() => true) / wall), coresWorkers: r3(sumBy((x) => x.name.includes('DedicatedWorker')) / wall), coresOther: r3(sumBy((x) => !x.name.includes('DedicatedWorker')) / wall),
    coresRendererMain: r3(sumBy((x) => x.proc === 'renderer' && x.key.split('/')[0] === x.key.split('/')[1]) / wall), coresBrowser: r3(sumBy((x) => x.proc === 'browser') / wall), coresGpu: r3(sumBy((x) => x.proc === 'gpu-process') / wall),
    coresMachineBusyOnMask: r3(maskBusy / wall), foreignMean: r3(fore.reduce((a, b) => a + b, 0) / (fore.length || 1)), foreignP90: r3(fore[Math.floor(fore.length * 0.9)]), foreignMax: r3(fore[fore.length - 1]), maskCpus: cpus, workerCpuMs: workers, workerRunDelayMs: workerDelay, treeRunDelayMs: r3(out.cpuAll.reduce((s, x) => s + x.delayMs, 0)), pssMB: Math.round(out.pssMB_after), rssMB: Math.round(out.rssMB_after),
    topThreads: out.threads.slice(0, 12).map((x) => `${x.name}[${x.proc}]:${Math.round(x.ms)}`), others: out.others, chrome_pids_left: out.chrome_pids_left, ...A, warmSeries: result.warm.map((w) => ({ src: w.src, first: w.blockWallMs.slice(0, 6).map(r3), mid: r3(w.blockWallMs[Math.floor(w.blockWallMs.length / 2)]), last: w.blockWallMs.slice(-3).map(r3), hash: (w.lines.find((l) => l.startsWith('RESULT out_hash')) || '').slice(0, 40) })) };
  fs.writeFileSync(path.join(RES, `${name}.json`), JSON.stringify({ row, out: { ...out, cpuAll: out.cpuAll.slice(0, 60) }, result }));
  rows.push(row);
  fs.appendFileSync(path.join(RES, `${tag}-rows.jsonl`), JSON.stringify(row) + '\n');
  const { per, ...brief } = row;
  console.log(JSON.stringify({ ...brief, per: per.map((x) => `${x.label}: p50 ${x.procP50} p99 ${x.procP99} max ${x.procMax} late ${x.lateA1} ${x.hash}`) }));
}
