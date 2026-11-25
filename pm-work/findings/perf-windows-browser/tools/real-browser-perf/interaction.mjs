#!/usr/bin/env node
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { aggregateInteraction } from './interaction-aggregate.mjs';

const execFileAsync = promisify(execFile);
const fail = (error) => {
  const message = error?.cmd ? String(error.stderr?.trim() || 'browse command failed').slice(0, 4096) : String(error?.message ?? error);
  process.stdout.write(`${JSON.stringify({ ok: false, error: message, ...(error?.code !== undefined ? { code: error.code } : {}), ...(error?.signal ? { signal: error.signal } : {}) })}\n`);
  process.exitCode = 1;
};

function args(argv) {
  const values = argv.map(Number);
  const [tab, fromX, fromY, toX, toY, reps, steps, ...extra] = argv;
  if (extra.length || !tab || values.slice(1).some((n) => !Number.isFinite(n)) ||
      [fromX, fromY, toX, toY].some((n) => !Number.isFinite(Number(n))) ||
      !Number.isSafeInteger(Number(reps)) || Number(reps) < 1 || Number(reps) > 1000 ||
      !Number.isSafeInteger(Number(steps)) || Number(steps) < 1 || Number(steps) > 1000) {
    throw new Error('usage: interaction.mjs tab fromX fromY toX toY reps steps; reps and steps: 1..1000');
  }
  return { tab, fromX: Number(fromX), fromY: Number(fromY), toX: Number(toX), toY: Number(toY), reps: Number(reps), steps: Number(steps) };
}

function beginExpression() {
  if (window.__interactionCapture) return { ok: false, failure: 'capture-active', message: 'an interaction capture is already active' };
  const a = window.__app;
  const detail = a?.audio?.detail?.();
  const state = { app: Boolean(a), engine: a?.state, fileDialog: a?.fileDlg?.open, audio: detail?.deviceState, context: detail?.ctxState, visibility: document.visibilityState };
  const mismatches = Object.entries({ app: state.app, engine: state.engine === 'engine-idle', fileDialog: state.fileDialog === false, audio: state.audio === 'playing' && state.context === 'running', visibility: state.visibility === 'visible' }).filter(([, ok]) => !ok).map(([name]) => name);
  if (mismatches.length) return { ok: false, failure: 'precondition', state, mismatches };

  const records = a.latency.records;
  const startAt = performance.timeOrigin + performance.now();
  const startSeq = records.at(-1)?.seq ?? -1;
  const totals = a.latency.totals();
  const drags = [];
  const canvasCounter = () => typeof a.deskStats?.displayFrames === 'number' && Number.isFinite(a.deskStats.displayFrames) ? a.deskStats.displayFrames : null;
  const capture = { a, state, startAt, startSeq, totals, drags, listeners: null, active: null };
  const finish = (completed) => {
    const d = capture.active;
    if (!d) return;
    const end = performance.now();
    const canvasEnd = canvasCounter();
    const canvasFrames = d.canvasStart !== null && canvasEnd !== null && canvasEnd >= d.canvasStart ? canvasEnd - d.canvasStart : null;
    drags.push({ elapsedMs: end - d.start, canvasFrames, completed });
    capture.active = null;
  };
  const down = (event) => {
    if (capture.active || event.button !== 0) return;
    capture.active = { pointerId: event.pointerId, start: performance.now(), canvasStart: canvasCounter() };
  };
  const up = (event) => {
    if (capture.active?.pointerId === event.pointerId) finish(true);
  };
  const cancel = (event) => {
    if (capture.active?.pointerId === event.pointerId) finish(false);
  };
  capture.listeners = { down, up, cancel };
  try {
    document.addEventListener('pointerdown', down, true);
    document.addEventListener('pointerup', up, true);
    document.addEventListener('pointercancel', cancel, true);
    window.__interactionCapture = capture;
  } catch (error) {
    document.removeEventListener('pointerdown', down, true);
    document.removeEventListener('pointerup', up, true);
    document.removeEventListener('pointercancel', cancel, true);
    throw error;
  }
  return { ok: true, state, startAt, startSeq, retentionLimit: 400 };
}

function endExpression(aggregate) {
  const c = window.__interactionCapture;
  if (!c) return { ok: false, error: 'interaction capture was not started' };
  try {
    document.removeEventListener('pointerdown', c.listeners.down, true);
    document.removeEventListener('pointerup', c.listeners.up, true);
    document.removeEventListener('pointercancel', c.listeners.cancel, true);
    if (c.active) {
      const canvasEnd = typeof c.a.deskStats?.displayFrames === 'number' && Number.isFinite(c.a.deskStats.displayFrames) ? c.a.deskStats.displayFrames : null;
      c.drags.push({ elapsedMs: performance.now() - c.active.start, canvasFrames: c.active.canvasStart !== null && canvasEnd !== null && canvasEnd >= c.active.canvasStart ? canvasEnd - c.active.canvasStart : null, completed: false });
      c.active = null;
    }
    const endAt = performance.timeOrigin + performance.now();
    const records = c.a.latency.records.filter((r) => r.seq > c.startSeq && r.tPage >= c.startAt && r.tPage <= endAt);
    const sentDelta = c.a.latency.totals().sent - c.totals.sent;
    const result = aggregate({ records, startSeq: c.startSeq, startAt: c.startAt, endAt, sentDelta, drags: c.drags, retentionLimit: 400 });
    return { ok: true, state: c.state, interval: { start: c.startAt, end: endAt, elapsedMs: endAt - c.startAt }, ...result,
      inputMethod: 'browse drag uses CDP-driven DOM pointer input; it is not a physical mouse measurement.',
      canvasFramesMeaning: 'Per-drag deltas of __app.deskStats.displayFrames (render Worker canvas updates), not physical display FPS; null when the counter is unavailable.',
    };
  } finally {
    document.removeEventListener('pointerdown', c.listeners.down, true);
    document.removeEventListener('pointerup', c.listeners.up, true);
    document.removeEventListener('pointercancel', c.listeners.cancel, true);
    c.active = null;
    delete window.__interactionCapture;
  }
}

function cleanupExpression() {
  const c = window.__interactionCapture;
  if (!c) return true;
  for (const type of ['pointerdown', 'pointerup', 'pointercancel']) document.removeEventListener(type, c.listeners[type.slice(7)], true);
  c.active = null;
  delete window.__interactionCapture;
  return true;
}

const evalCommand = async (tab, expression) => {
  const { stdout } = await execFileAsync('browse', ['eval', '--json', JSON.stringify({ tab, world: 'main', expression })], { timeout: 35_000, maxBuffer: 8 * 1024 * 1024 });
  return JSON.parse(stdout);
};

let input;
let beginAttempted = false;
try {
  input = args(process.argv.slice(2));
  beginAttempted = true;
  const started = await evalCommand(input.tab, `(${beginExpression.toString()})()`);
  if (!started.ok) {
    beginAttempted = false;
    process.stdout.write(`${JSON.stringify(started)}\n`);
    process.exitCode = 1;
  } else {
    let error = null;
    try {
      for (let i = 0; i < input.reps; i++) {
        const forward = i % 2 === 0;
        const from = forward ? { x: input.fromX, y: input.fromY } : { x: input.toX, y: input.toY };
        const to = forward ? { x: input.toX, y: input.toY } : { x: input.fromX, y: input.fromY };
        await execFileAsync('browse', ['drag', '--json', JSON.stringify({ tab: input.tab, from, to, steps: input.steps })], { timeout: 60_000, maxBuffer: 2 * 1024 * 1024 });
      }
    } catch (cause) {
      error = String(cause?.stderr?.trim() || cause?.message || cause).slice(0, 4096);
    }
    let ended;
    try {
      ended = await evalCommand(input.tab, `(${endExpression.toString()})(${aggregateInteraction.toString()})`);
    } catch (cause) {
      try { await evalCommand(input.tab, `(${cleanupExpression.toString()})()`); } catch { /* the browser may have disconnected */ }
      beginAttempted = false;
      throw cause;
    }
    beginAttempted = false;
    process.stdout.write(`${JSON.stringify({ ...ended, request: { reps: input.reps, steps: input.steps }, ...(error ? { dragError: error } : {}) })}\n`);
    if (ended.ok === false || error) process.exitCode = 1;
  }
} catch (error) {
  if (beginAttempted && input?.tab) {
    try { await evalCommand(input.tab, `(${cleanupExpression.toString()})()`); } catch { /* the browser may have disconnected */ }
  }
  fail(error);
}
