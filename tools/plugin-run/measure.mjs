#!/usr/bin/env node
// tools/plugin-run/measure.mjs <plugin dir> <ott|serum2|vital> : starts the plugin in its own Worker (app/src/plugins/plugin-worker.ts, through PluginInstance.start)
// from the output of lift/harness/web/mt1/plugin-worker/build.sh, runs one block (4096 frames at 44,100 Hz: the OTT test signal of app/src/plugins/test-signal.ts)
// through IAudioProcessor::process, and prints one JSON line:
//   {"plugin":"<name>","instantiated":true|false,"blockFrames":<n>,"nonSilent":true|false,"error":null|"<where it stopped>"}
// instantiated is true when the Worker reports READY (setupProcessing and setActive done). blockFrames is the number of frames the Worker answered (its count of blocks
// times the frame count). nonSilent is true when one output sample is not 0. A NaN or Inf in the output is an error: "output: non-finite output: <count>".
// Then the Worker is asked to quit, and its exit (rc and the files the plugin asked for and did not get) is checked.
// Exit: 0 when error is null, 1 otherwise, 64 for a usage error (nothing on stdout). The plugin's own lines (vst: ..., RESULT ...) go to stderr.
import fs from 'node:fs';
import path from 'node:path';
import { PluginInstance } from '../../app/src/plugins/plugin-client.ts';
import { nodePluginWorker } from '../../app/src/plugins/node-worker.ts';
import { OttTestSignal } from '../../app/src/plugins/test-signal.ts';

const [dir, name] = process.argv.slice(2);
const NAMES = new Set(['ott', 'serum2', 'vital']);
if (!dir || !NAMES.has(name) || process.argv.length !== 4) {
  process.stderr.write('usage: node tools/plugin-run/measure.mjs <plugin dir> <ott|serum2|vital>\n');
  process.exit(64);
}

const FRAMES = 4096;
const RATE = 44100;
const ANSWER_TIMEOUT_MS = 120000;
const file = `${name}.vst3`;
const result = { plugin: name, instantiated: false, blockFrames: 0, nonSilent: false, error: null };
let stage = 'build output';

/** The Worker's own failure (an error message ends its done promise: stop() rejects with it). A Worker that is still busy gives no answer within 5 s. */
async function workerFailure(inst) {
  return Promise.race([
    inst.stop().then(() => 'the worker left its loop', (e) => (e instanceof Error ? e.message : String(e)).split('\n')[0]),
    new Promise((res) => setTimeout(() => res('the worker gave no answer'), 5000)),
  ]);
}

try {
  const wasm = fs.readFileSync(path.join(dir, 'vst-ext.wasm'));
  const files = {
    [`/${file}`]: new Uint8Array(fs.readFileSync(path.join(dir, file))),
    '/seed.tsv': new Uint8Array(fs.readFileSync(path.join(dir, 'seed.tsv'))),
  };
  stage = 'compile';
  const module = await WebAssembly.compile(wasm);
  stage = 'start';
  const inst = await PluginInstance.start(nodePluginWorker, { module, files, pluginPath: `/${file}`, frames: FRAMES, sampleRate: RATE, maxBlocks: 4096 });
  result.instantiated = true;
  stage = 'process';
  const sig = new OttTestSignal({ rate: RATE });
  const inL = new Float32Array(FRAMES);
  const inR = new Float32Array(FRAMES);
  const outL = new Float32Array(FRAMES);
  const outR = new Float32Array(FRAMES);
  sig.next(FRAMES, inL, inR);
  const seq = inst.begin(inL, inR);
  if (!(await inst.waitAsync(seq, ANSWER_TIMEOUT_MS))) {
    const why = await workerFailure(inst);
    throw new Error(`no answer within ${ANSWER_TIMEOUT_MS / 1000} s (${why})`);
  }
  inst.read(outL, outR);
  let nonFinite = 0;
  for (const a of [outL, outR]) {
    for (let i = 0; i < FRAMES; i++) {
      if (!Number.isFinite(a[i])) nonFinite++;
      else if (a[i] !== 0) result.nonSilent = true;
    }
  }
  stage = 'stop';
  const r = await inst.stop();
  result.blockFrames = r.served * FRAMES;
  for (const l of r.lines) process.stderr.write(`${l}\n`);
  if (r.rc !== 0) throw new Error(`the plugin's main returned ${r.rc}`);
  if (r.missing.length > 0) throw new Error(`the plugin asked for files that are not in the seed: ${r.missing.join(', ')}`);
  stage = 'output';
  if (nonFinite > 0) throw new Error(`non-finite output: ${nonFinite}`);
} catch (e) {
  const msg = e instanceof Error ? e.message.split('\n')[0] : String(e);
  result.error = `${stage}: ${msg}`;
}
process.stdout.write(`${JSON.stringify(result)}\n`);
process.exit(result.error === null ? 0 : 1);
