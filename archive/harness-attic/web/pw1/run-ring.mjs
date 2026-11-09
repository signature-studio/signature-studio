// pw1 (decision 0186): node run-ring.mjs serum=0,1,3,4,5,3 ott=12 hc=8 blocks=650 reps=3 tag=ring-heavy-hc8 [stagger=1|0] [lead=1] [ahead=0] [warmms=25000 settle=3000]
// The decoupled-ring variant of run-pw1.mjs: every instance renders on its own schedule (phase-staggered when stagger=1) into an output ring and a reader takes block b at T0 + (b+1+lead)*P; no lock step.
// Measured per run: engine-side under-runs (instances that had not produced block b when the reader took it), blocks with at least one under-run, the same count for other read times (lead 0, 0.5, 1, 2 computed from the
// instances' completion times), own-deadline misses (an instance finished its block later than its own start + one period), CPU of the Chrome tree, foreign CPU, load. Output: results/<name>.json, results/<tag>-rows.jsonl.
import fs from 'node:fs';
import path from 'node:path';
import { withBrowser, threadNs, nsDiff, cpuStat, pssMB, rssMB, argv, OUT, sleep, pct } from './lib.mjs';
const serum = argv.serum || '', ott = +(argv.ott || 0), ottmod = argv.ottmod || 'c', hc = +(argv.hc || 16), blocks = +(argv.blocks || 650), reps = +(argv.reps || 1), tag = argv.tag || 'ring', mod = argv.mod || 'h123', nice = argv.nice ?? '5';
const warmms = argv.warmms || '25000', settle = argv.settle || '3000', stagger = argv.stagger ?? '1', lead = argv.lead || '1', ahead = argv.ahead || '0';
const cpus = argv.cpus ? argv.cpus : hc >= 16 ? null : `0-${hc - 1}`;
const prefix = [...(cpus ? ['taskset', '-c', cpus] : []), 'nice', '-n', String(nice)];
const RES = process.env.PW1_RESULTS || path.join(OUT, 'results');
fs.mkdirSync(RES, { recursive: true });
const maskList = (s) => (!s ? null : s.split(',').flatMap((t) => { const [a, b] = t.split('-').map(Number); return Array.from({ length: (b ?? a) - a + 1 }, (_, i) => a + i); }));
const mask = maskList(cpus);
const r3 = (x) => (x === undefined || x === null || Number.isNaN(x) ? null : Math.round(x * 1000) / 1000);

function analyse(res) {
  const P = res.cfg.PERIOD, B = res.cfg.BLOCKS, K = res.plugins.length, R = res.ring;
  const C = res.plugins.map((p) => p.T[3]), S = res.plugins.map((p) => p.T[2]);
  const missBlocks = (L) => { let n = 0; for (let b = 0; b < B; b++) { let m = false; for (let k = 0; k < K; k++) if (C[k][b] > (b + 1 + L) * P) m = true; n += m; } return n; };
  const engMiss = new Set(); let engMissPairs = 0;
  for (let b = 0; b < B; b++) for (let k = 0; k < K; k++) if (R.miss[b * K + k]) { engMiss.add(b); engMissPairs++; }
  let own = 0, ownBlocks = new Set(), maxOver = -1e9, maxL1 = -1e9;
  for (let b = 0; b < B; b++) for (let k = 0; k < K; k++) {
    const sched = b * P + res.plugins[k].phi;
    if (C[k][b] > sched + P) { own++; ownBlocks.add(b); }
    maxOver = Math.max(maxOver, C[k][b] - (sched + P)); maxL1 = Math.max(maxL1, C[k][b] - (b + 2) * P);
  }
  const per = res.plugins.map((p, k) => {
    const proc = []; for (let b = 0; b < B; b++) proc.push(C[k][b] - S[k][b]);
    const ps = pct(proc, [50, 95, 99]);
    const hash = (p.lines.find((l) => l.startsWith('RESULT out_hash')) || '').slice(0, 60);
    return { label: p.label, kind: p.kind, phi: r3(p.phi), procP50: r3(ps.p50), procP99: r3(ps.p99), procMax: r3(ps.max), procMean: r3(ps.mean), ownLate: C[k].filter((c, b) => c > b * P + p.phi + P).length, hash, err: p.err, done: p.done };
  });
  return { periodMs: r3(P), readerMissBlocks: engMiss.size, readerMissPairs: engMissPairs, lateRead0: missBlocks(0), lateRead05: missBlocks(0.5), lateRead1: missBlocks(1), lateRead2: missBlocks(2), ownLatePairs: own, ownLateBlocks: ownBlocks.size,
    maxLatenessRead1: r3(maxL1), maxOverOwnDeadline: r3(maxOver), per, hashes: per.map((x) => x.hash) };
}

for (let rep = 0; rep < reps; rep++) {
  const name = `${tag}-r${rep + +(argv.rep0 || 0)}`;
  const q = `serum=${serum}&ott=${ott}&ottmod=${ottmod}&blocks=${blocks}&warm=1&warmms=${warmms}&settle=${settle}&mod=${mod}&stagger=${stagger}&lead=${lead}&ahead=${ahead}`;
  const o = await withBrowser({ name: `ring-${process.pid}-${name}`, page: `/coi/pw1/ring.html?${q}`, prefix, timeoutS: 900 }, async ({ evalJSON, waitFor, chrome, out }) => {
    await waitFor('__ready', 600, 200);
    await sleep(1500);
    const root = chrome.proc.pid;
    const sb = threadNs(root), cb = cpuStat(), lb = fs.readFileSync('/proc/loadavg', 'utf8').trim();
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
    out.cpuAll = nsDiff(sb, sa);
    out.busyMask = {}; for (const k of Object.keys(ca)) out.busyMask[k] = ca[k].busy - cb[k].busy;
    await waitFor('__done', 120, 100);
    const r = JSON.parse(await evalJSON('JSON.stringify(window.__result)'));
    await evalJSON('window.__terminate = true; true');
    return r;
  });
  const { out, result, error } = o;
  if (error || out.error) { console.error(name, 'ERROR', String(error || out.error).slice(0, 800)); continue; }
  const A = analyse(result);
  const wall = out.window.wallMs, sumBy = (f) => out.cpuAll.filter(f).reduce((s, x) => s + x.ms, 0);
  const maskBusy = mask ? mask.reduce((s, c) => s + (out.busyMask['cpu' + c] || 0), 0) * 10 : (out.busyMask.cpu || 0) * 10;
  const fore = (out.busySeries || []).map(([m, c]) => Math.max(0, m - c)).sort((a, b) => a - b);
  const row = { name, tag, mode: 'ring', stagger: +stagger, lead: +lead, ahead: +ahead, serum, ott, hc, cpus: cpus || 'all', hcReported: result.hardwareConcurrency, blocks, warmms: +warmms, load_in: out.window.load_in, load_out: out.window.load_out,
    windowS: r3(wall / 1000), tWarmMs: Math.round(result.tWarm), tInitMs: Math.round(result.tInit),
    coresTotal: r3(sumBy(() => true) / wall), coresWorkers: r3(sumBy((x) => x.name.includes('DedicatedWorker')) / wall), coresMachineBusyOnMask: r3(maskBusy / wall),
    foreignMean: r3(fore.reduce((a, b) => a + b, 0) / (fore.length || 1)), foreignP90: r3(fore[Math.floor(fore.length * 0.9)]), foreignMax: r3(fore[fore.length - 1]),
    workerCpuMs: out.cpuAll.filter((x) => x.name.includes('DedicatedWorker')).map((x) => r3(x.ms)), workerRunDelayMs: out.cpuAll.filter((x) => x.name.includes('DedicatedWorker')).map((x) => r3(x.delayMs)),
    pssMB: Math.round(out.pssMB_after), chrome_pids_left: out.chrome_pids_left, ...A };
  fs.writeFileSync(path.join(RES, `${name}.json`), JSON.stringify({ row, out: { ...out, cpuAll: out.cpuAll.slice(0, 60) }, result }));
  fs.appendFileSync(path.join(RES, `${tag}-rows.jsonl`), JSON.stringify(row) + '\n');
  const { per, ...brief } = row;
  console.log(JSON.stringify({ ...brief, per: per.map((x) => `${x.label}: phi ${x.phi} p50 ${x.procP50} max ${x.procMax} ownLate ${x.ownLate} ${x.hash}`) }));
}
