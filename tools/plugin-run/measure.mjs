#!/usr/bin/env node
// tools/plugin-run/measure.mjs <plugin dir> <ott|serum2|vital> : starts the plugin in its own Worker (app/src/plugins/plugin-worker.ts, through PluginInstance.start)
// from the output of lift/harness/web/mt1/plugin-worker/build.sh, and prints one JSON line:
//   {"plugin":"<name>","instantiated":true|false,"blockFrames":<n>,"nonSilent":true|false,"error":null|"<where it stopped>"}
// instantiated is true when the Worker reports READY (setupProcessing and setActive done, the first block wait reached). This stage processes no audio:
// blockFrames is 0 and nonSilent is false. Then the Worker is asked to quit, and its exit (rc and the files the plugin asked for and did not get) is checked.
// Exit: 0 when error is null, 1 otherwise, 64 for a usage error (nothing on stdout). The plugin's own lines (vst: ..., RESULT ...) go to stderr.
import fs from 'node:fs';
import path from 'node:path';
import { PluginInstance } from '../../app/src/plugins/plugin-client.ts';
import { nodePluginWorker } from '../../app/src/plugins/node-worker.ts';

const [dir, name] = process.argv.slice(2);
const NAMES = new Set(['ott', 'serum2', 'vital']);
if (!dir || !NAMES.has(name) || process.argv.length !== 4) {
  process.stderr.write('usage: node tools/plugin-run/measure.mjs <plugin dir> <ott|serum2|vital>\n');
  process.exit(64);
}

const file = `${name}.vst3`;
const result = { plugin: name, instantiated: false, blockFrames: 0, nonSilent: false, error: null };
let stage = 'build output';
try {
  const wasm = fs.readFileSync(path.join(dir, 'vst-ext.wasm'));
  const files = {
    [`/${file}`]: new Uint8Array(fs.readFileSync(path.join(dir, file))),
    '/seed.tsv': new Uint8Array(fs.readFileSync(path.join(dir, 'seed.tsv'))),
  };
  stage = 'compile';
  const module = await WebAssembly.compile(wasm);
  stage = 'start';
  const inst = await PluginInstance.start(nodePluginWorker, { module, files, pluginPath: `/${file}`, frames: 4096, sampleRate: 44100, maxBlocks: 4096 });
  result.instantiated = true;
  stage = 'stop';
  const r = await inst.stop();
  for (const l of r.lines) process.stderr.write(`${l}\n`);
  if (r.rc !== 0) throw new Error(`the plugin's main returned ${r.rc}`);
  if (r.missing.length > 0) throw new Error(`the plugin asked for files that are not in the seed: ${r.missing.join(', ')}`);
} catch (e) {
  const msg = e instanceof Error ? e.message.split('\n')[0] : String(e);
  result.error = `${stage}: ${msg}`;
}
process.stdout.write(`${JSON.stringify(result)}\n`);
process.exit(result.error === null ? 0 : 1);
