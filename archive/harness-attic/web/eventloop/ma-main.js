'use strict';
// M-A page side: SharedArrayBuffer event ring + a Worker that blocks in Atomics.wait. ?tests=env,rtt,pp,timeouts,timer,busy,mainbusy,thru (default all)
// ?n=10000 round trips. Result: window.__result, window.__done.
const Q = new URLSearchParams(location.search);
const N = +(Q.get('n') || 10000);
const WANT = (Q.get('tests') || 'env,rtt,pp,timeouts,timer,busy,mainbusy,thru').split(',');
const pause = (ms) => new Promise((r) => setTimeout(r, ms));
const log = (s) => { (window.__log = window.__log || []).push(s); };
const hasSAB = typeof SharedArrayBuffer !== 'undefined';

function startWorker(file, cfg) {
  const w = new Worker(file);
  let readyRes;
  const ready = new Promise((r) => { readyRes = r; });
  const result = new Promise((res, rej) => {
    w.onmessage = (e) => {
      const d = e.data;
      if (d && d.type === 'ready') readyRes();
      else if (d && d.type === 'result') res(d.result);
      else if (w.extra) w.extra(d);
    };
    w.onerror = (e) => rej(new Error('worker error: ' + e.message));
  });
  w.postMessage(cfg);
  return { w, ready, result };
}

async function waitReply(ring, k) {
  const i = ring.i;
  for (;;) {
    const cur = Atomics.load(i, RG.REPLY);
    if (cur === k) return;
    const r = Atomics.waitAsync(i, RG.REPLY, cur);
    if (r.async) await r.value;
  }
}

async function rttSab(n, pace, replyMode) {
  const sab = ringAlloc(256);
  const ring = new Ring(sab);
  const wk = startWorker('ma-worker.js', { mode: 'echo', sab, reply: replyMode });
  let pmWait = null;
  wk.w.extra = (d) => { if (typeof d === 'number' && pmWait && pmWait.k === d) pmWait.res(); };
  await wk.ready;
  for (let k = 1; k <= 200; k++) { // warm-up (not counted)
    ring.post(EV.PING, 1000000 + k, 0, 0, 0);
    if (replyMode === 'pm') await new Promise((res) => { pmWait = { k: 1000000 + k, res }; }); else await waitReply(ring, 1000000 + k);
  }
  const rtt = new Array(n);
  for (let k = 1; k <= n; k++) {
    if (pace) await pause(pace);
    const p = replyMode === 'pm' ? new Promise((res) => { pmWait = { k, res }; }) : null;
    const t0 = performance.now();
    ring.post(EV.PING, k, 0, 0, wallNow());
    if (replyMode === 'pm') await p; else await waitReply(ring, k);
    rtt[k - 1] = (performance.now() - t0) * 1000;
  }
  ring.post(EV.QUIT, 0, 0, 0, 0);
  const r = await wk.result;
  wk.w.terminate();
  return { unit: 'us', n, pace_ms: pace, reply: replyMode, rtt: summarize(rtt), worker: r };
}

async function rttNonBlocking(n) {
  const w = new Worker('pm-echo.js');
  let res = null;
  w.onmessage = () => res();
  for (let k = 0; k < 200; k++) { await new Promise((r) => { res = r; w.postMessage(k); }); }
  const rtt = new Array(n);
  for (let k = 0; k < n; k++) {
    const t0 = performance.now();
    await new Promise((r) => { res = r; w.postMessage(k); });
    rtt[k] = (performance.now() - t0) * 1000;
  }
  w.terminate();
  return { unit: 'us', n, rtt: summarize(rtt) };
}

async function pingpong(n) {
  const sab = new SharedArrayBuffer(64);
  const b = startWorker('ma-worker.js', { mode: 'pp', side: 'b', sab });
  const a = startWorker('ma-worker.js', { mode: 'pp', side: 'a', sab, n });
  const ra = await a.result;
  await b.result;
  a.w.terminate(); b.w.terminate();
  return ra;
}

async function timerRun(period, ticks, work, evPerBurst, cap, sendEvents) {
  const sab = ringAlloc(cap || 4096);
  const ring = new Ring(sab);
  const wk = startWorker('ma-worker.js', { mode: 'timer', sab, period, ticks, work });
  await wk.ready;
  let stop = false;
  let sent = 0;
  const prod = (async () => {
    while (!stop && sendEvents) {
      for (let j = 0; j < evPerBurst; j++) { ring.post(EV.MOVE, sent++, j, j, wallNow()); }
      await pause(16);
    }
  })();
  const r = await wk.result;
  stop = true;
  await prod;
  wk.w.terminate();
  r.sent = sent;
  return r;
}

async function busyRun(work, cap, coalesce, ms) {
  const sab = ringAlloc(cap);
  const ring = new Ring(sab);
  const wk = startWorker('ma-worker.js', { mode: 'busy', sab, work });
  await wk.ready;
  const t0 = performance.now();
  let sent = 0;
  let sentDown = 0;
  let lostDown = 0;
  let burst = 0;
  while (performance.now() - t0 < ms) {
    for (let j = 0; j < 4; j++) {
      sent++;
      if (coalesce) ring.postMove(sent, j, wallNow()); else ring.post(EV.MOVE, sent, sent, j, wallNow());
    }
    if (++burst % 30 === 0) { sentDown++; if (!ring.post(EV.DOWN, sent, 0, 0, wallNow())) lostDown++; }
    await pause(16);
  }
  const leftAtStop = ring.depth();
  Atomics.store(ring.i, RG.STOP, 1);
  ring.post(EV.QUIT, 0, 0, 0, 0);
  const r = await wk.result;
  wk.w.terminate();
  return Object.assign(r, { work_ms_per_event: work, cap, coalesce, run_ms: ms, moveSent: sent, downSent: sentDown, downLost: lostDown, leftAtStop });
}

async function mainBusyRun(busyMs, everyMs, ticks) {
  const w = new Worker('ma-worker.js');
  const arrivals = [];
  let done;
  const fin = new Promise((r) => { done = r; });
  w.onmessage = (e) => {
    const d = e.data;
    if (d.type === 'tick') arrivals.push(performance.timeOrigin + performance.now());
    else if (d.type === 'result') done(d.result);
  };
  w.postMessage({ mode: 'ticker', period: 10, ticks });
  const t0 = performance.now();
  (async () => { while (!window.__stopBusy) { await pause(everyMs); if (busyMs) spin(busyMs); } })();
  const r = await fin;
  window.__stopBusy = true;
  await pause(300);   // let the queued ticks arrive
  w.terminate();
  window.__stopBusy = false;
  const gaps = [];
  for (let k = 1; k < arrivals.length; k++) gaps.push(arrivals[k] - arrivals[k - 1]);
  return { busy_ms: busyMs, every_ms: everyMs, worker: r, arrivalsAtPage: arrivals.length, gapAtPage: summarize(gaps), gapMax: Math.max.apply(null, gaps) };
}

async function throughput(ms) {
  const sab = ringAlloc(65536);
  const ring = new Ring(sab);
  const wk = startWorker('ma-worker.js', { mode: 'busy', sab, work: 0 });
  await wk.ready;
  const t0 = performance.now();
  let sent = 0;
  let ok = 0;
  while (performance.now() - t0 < ms) {
    for (let j = 0; j < 1000; j++) { sent++; if (ring.post(EV.KEY, sent, 0, 0, 0)) ok++; }
  }
  const el = performance.now() - t0;
  await pause(200);
  Atomics.store(ring.i, RG.STOP, 1);
  ring.post(EV.QUIT, 0, 0, 0, 0);
  const r = await wk.result;
  wk.w.terminate();
  return { run_ms: el, attempted: sent, accepted: ok, post_us_each: (el * 1000) / sent, consumed: r.consumed, dropped: r.dropped };
}

async function main() {
  const R = {};
  R.env = {
    crossOriginIsolated: self.crossOriginIsolated, SharedArrayBuffer: typeof SharedArrayBuffer, waitAsync: typeof (self.Atomics && Atomics.waitAsync),
    hardwareConcurrency: navigator.hardwareConcurrency, userAgent: navigator.userAgent, timeResolution_us: (() => { let m = 1e9; let p = performance.now(); for (let k = 0; k < 20000; k++) { const q = performance.now(); if (q > p) { m = Math.min(m, q - p); p = q; } } return +(m * 1000).toFixed(1); })(),
  };
  log('env ' + JSON.stringify(R.env));
  if (WANT.includes('rtt')) {
    R.rtt_nonblocking_pm = await rttNonBlocking(N); log('rtt_nonblocking_pm done');
    if (hasSAB) {
      R.rtt_sab_sab = await rttSab(N, 0, 'sab'); log('rtt_sab_sab done');
      R.rtt_sab_pm = await rttSab(N, 0, 'pm'); log('rtt_sab_pm done');
      R.rtt_sab_sab_paced4ms = await rttSab(Math.min(N, 2500), 4, 'sab'); log('rtt paced done');
    }
  }
  if (hasSAB && WANT.includes('pp')) { R.pingpong_worker_worker = await pingpong(N); log('pp done'); }
  if (hasSAB && WANT.includes('timeouts')) {
    const w = startWorker('ma-worker.js', { mode: 'timeouts', list: [0.5, 1, 2, 5, 10, 15, 16, 20, 33, 50, 100] });
    R.atomics_wait_timeout = await w.result; w.w.terminate(); log('timeouts done');
  }
  if (hasSAB && WANT.includes('timer')) {
    R.timer = {};
    R.timer.p10_idle = await timerRun(10, 500, 0, 0, 4096, false);
    R.timer.p1_idle = await timerRun(1, 2000, 0, 0, 4096, false);
    R.timer.p10_with_events = await timerRun(10, 500, 0, 4, 4096, true);
    R.timer.p10_events_work2ms = await timerRun(10, 500, 2, 4, 4096, true);
    log('timer done');
  }
  if (hasSAB && WANT.includes('busy')) {
    R.busy = [];
    for (const work of [0, 5, 20, 100]) for (const cap of [64, 4096]) for (const coalesce of [false, true]) {
      R.busy.push(await busyRun(work, cap, coalesce, 3000));
      log(`busy ${work} ${cap} ${coalesce} done`);
    }
  }
  if (hasSAB && WANT.includes('mainbusy')) {
    R.mainbusy = [await mainBusyRun(0, 250, 300), await mainBusyRun(100, 250, 300), await mainBusyRun(400, 600, 300)];
    log('mainbusy done');
  }
  if (hasSAB && WANT.includes('thru')) { R.throughput = await throughput(2000); log('thru done'); }
  window.__result = R;
  window.__done = true;
}
main().catch((e) => { window.__error = String(e && e.stack || e); });
