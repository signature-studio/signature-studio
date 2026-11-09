// The REAL lifted OTT VST3 (decision 0156/0164) in plugin Workers behind the SAB protocol (decision 0173, S1), in node worker_threads. Needs the assets that are not in git (skipped without them):
//   MT1_OTT_DIR (default <repo>/lift/out-mt1/data) containing  vst-ext.wasm  (lift/harness/web/mt1/plugin-worker/build.sh),  ott.vst3  (the rebased PE),  seed.tsv  (lift/harness/vst/vfs-seed-vst.tsv)
// Expected hashes are the out_hash of the unchanged harness in decision 0164 (default / input x0.5 / silence / --param-first 0.25, 40 blocks of 4096 frames).
// MT1_RT_STRICT=0 turns the real-time deadline assertion into a report (a very loaded machine).
import fs from 'node:fs';
import path from 'node:path';
import { describe, expect, it } from 'vitest';
import { drive } from '../../src/plugins/driver.ts';
import { nodePluginWorker } from '../../src/plugins/node-worker.ts';
import { PluginInstance, type PluginConfig } from '../../src/plugins/plugin-client.ts';
import { OttTestSignal } from '../../src/plugins/test-signal.ts';

const here = path.dirname(new URL(import.meta.url).pathname);
const dir = process.env['MT1_OTT_DIR'] ?? path.resolve(here, '../../../../out-mt1/data');
const have = ['vst-ext.wasm', 'ott.vst3', 'seed.tsv'].every((f) => fs.existsSync(path.join(dir, f)));
const PERIOD = (4096 / 44100) * 1000;

async function config(extra: Partial<PluginConfig> = {}): Promise<PluginConfig> {
  const module = await WebAssembly.compile(fs.readFileSync(path.join(dir, 'vst-ext.wasm')));
  const files = { '/ott.vst3': new Uint8Array(fs.readFileSync(path.join(dir, 'ott.vst3'))), '/seed.tsv': new Uint8Array(fs.readFileSync(path.join(dir, 'seed.tsv'))) };
  return { module, files, pluginPath: '/ott.vst3', maxBlocks: 4096, ...extra };
}

async function runScenario(cfg: PluginConfig, opts: { scale?: number; silence?: boolean }, blocks = 40): Promise<string | undefined> {
  const p = await PluginInstance.start(nodePluginWorker, cfg);
  const sig = new OttTestSignal(opts);
  const l = new Float32Array(4096);
  const r = new Float32Array(4096);
  const ol = new Float32Array(4096);
  const or = new Float32Array(4096);
  for (let b = 0; b < blocks; b++) {
    sig.next(4096, l, r);
    p.processSync(l, r, ol, or);
  }
  const res = await p.stop();
  expect(res.rc).toBe(0);
  expect(res.served).toBe(blocks);
  expect(res.missing).toEqual([]);
  return res.result['out_hash'];
}

describe.skipIf(!have)('OTT in a plugin Worker: same output as the single-process harness', () => {
  it('default input: out_hash 8104f50e938ccf36', async () => {
    expect(await runScenario(await config(), {})).toBe('8104f50e938ccf36');
  });
  it('input x0.5: 0c435d2acaa363ed', async () => {
    expect(await runScenario(await config(), { scale: 0.5 })).toBe('0c435d2acaa363ed');
  });
  it('silence: 96deca0f23b22325', async () => {
    expect(await runScenario(await config(), { silence: true })).toBe('96deca0f23b22325');
  });
  it('--param-first 0.25: 07f081139e0d6448', async () => {
    expect(await runScenario(await config({ extraArgs: ['--param-first', '0.25'] }), {})).toBe('07f081139e0d6448');
  });
});

describe.skipIf(!have)('four instances at once', () => {
  it('fan-out on one compiled module: every instance reproduces the default hash', async () => {
    const cfg = await config();
    const ps = await Promise.all([0, 1, 2, 3].map(() => PluginInstance.start(nodePluginWorker, cfg)));
    const sigs = ps.map(() => new OttTestSignal());
    const res = drive(ps, { blocks: 40, periodMs: 0, topology: 'fanout', signal: (k, n, l, r) => sigs[k]!.next(n, l, r) });
    expect(res.latencyMs.length).toBe(40);
    const out = await Promise.all(ps.map((p) => p.stop()));
    expect(out.map((o) => o.result['out_hash'])).toEqual(Array(4).fill('8104f50e938ccf36'));
  }, 60000);

  it('real-time paced (92.88 ms): no block misses its deadline after the pre-roll', async () => {
    const cfg = await config({ preroll: 6, prerollReset: 0 });
    const ps = await Promise.all([0, 1, 2, 3].map(() => PluginInstance.start(nodePluginWorker, cfg)));
    const sigs = ps.map(() => new OttTestSignal());
    const blocks = 120;
    const res = drive(ps, { blocks, periodMs: PERIOD, topology: 'fanout', signal: (k, n, l, r) => sigs[k]!.next(n, l, r) });
    await Promise.all(ps.map((p) => p.stop()));
    const sorted = [...res.latencyMs].sort((a, b) => a - b);
    console.log(`paced x4: p50 ${sorted[blocks >> 1]!.toFixed(2)} ms, p99 ${sorted[Math.floor(blocks * 0.99)]!.toFixed(2)} ms, max ${sorted[blocks - 1]!.toFixed(2)} ms, late blocks ${res.late.length}`);
    if (process.env['MT1_RT_STRICT'] !== '0') expect(res.late).toEqual([]);
  }, 60000);
});

describe.skipIf(!have)('pre-roll', () => {
  it('moves the first-block cost out of the first real block, and what it does to the output is measured, not assumed', async () => {
    const first: Record<string, number> = {};
    const hashes: Record<string, string | undefined> = {};
    for (const [name, extra] of [['none', {}], ['preroll8-keep', { preroll: 8, prerollReset: 0 }], ['preroll8-resetProc', { preroll: 8, prerollReset: 1 }], ['preroll8-resetActive', { preroll: 8, prerollReset: 2 }]] as const) {
      const p = await PluginInstance.start(nodePluginWorker, await config(extra));
      const sig = new OttTestSignal();
      const l = new Float32Array(4096);
      const r = new Float32Array(4096);
      const o = new Float32Array(4096);
      sig.next(4096, l, r);
      first[name] = p.processSync(l, r, o, new Float32Array(4096));
      for (let b = 1; b < 40; b++) {
        sig.next(4096, l, r);
        p.processSync(l, r, o, new Float32Array(4096));
      }
      const res = await p.stop();
      hashes[name] = res.result['out_hash'];
    }
    console.log('pre-roll first block ms', JSON.stringify(first), 'out_hash', JSON.stringify(hashes));
    expect(hashes['none']).toBe('8104f50e938ccf36');
    for (const v of Object.values(first)) expect(v).toBeGreaterThan(0);
  }, 60000);
});
