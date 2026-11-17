import fs from 'node:fs';
import http from 'node:http';
import os from 'node:os';
import path from 'node:path';
import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { assetsMiddleware } from '../../tools/assets-middleware.ts';
import { assetSuite, rawGet } from '../helpers/asset-suite.ts';
import { FIXTURE, MANIFEST_REAL, PRE_BR, PRE_GZ } from '../helpers/fixtures.ts';

let dir: string;
let server: http.Server;
let base = '';
let noDirServer: http.Server;
let noDirBase = '';

const listen = (s: http.Server): Promise<string> => new Promise((res) => s.listen(0, '127.0.0.1', () => res(`http://127.0.0.1:${(s.address() as { port: number }).port}`)));

beforeAll(async () => {
  dir = fs.mkdtempSync(path.join(os.tmpdir(), 'app0-assets-'));
  fs.mkdirSync(path.join(dir, 'v1-0123456789ab'));
  fs.writeFileSync(path.join(dir, 'blob.bin'), FIXTURE.blob);
  fs.writeFileSync(path.join(dir, 'engine.wasm'), FIXTURE.wasm);
  fs.writeFileSync(path.join(dir, 'pre.wasm'), FIXTURE.pre);
  fs.writeFileSync(path.join(dir, 'pre.wasm.br'), PRE_BR);
  fs.writeFileSync(path.join(dir, 'pre.wasm.gz'), PRE_GZ);
  fs.writeFileSync(path.join(dir, 'v1-0123456789ab', 'pinned.bin'), 'x');
  fs.writeFileSync(path.join(dir, 'manifest.json'), MANIFEST_REAL); // the default manifest is for the real engine only (decision 0152)
  const mw = assetsMiddleware(dir);
  server = http.createServer((req, res) => mw(req, res, () => { res.writeHead(404); res.end('next'); }));
  base = await listen(server);
  const mw2 = assetsMiddleware('');
  noDirServer = http.createServer((req, res) => mw2(req, res, () => { res.writeHead(404); res.end('next'); }));
  noDirBase = await listen(noDirServer);
});
afterAll(() => { server.close(); noDirServer.close(); fs.rmSync(dir, { recursive: true, force: true }); });

assetSuite('dev/preview middleware (files)', () => base, { brVariantIsSibling: true });

describe('assets middleware: what is specific to it', () => {
  it('passes everything outside /assets/ on to the next handler', async () => {
    const r = await rawGet(`${base}/index.html`);
    expect(r.body.toString()).toBe('next');
  });
  it('without FL_ASSETS_DIR it says so instead of failing silently', async () => {
    const r = await rawGet(`${noDirBase}/assets/x`);
    expect(r.status).toBe(404);
    expect(r.body.toString()).toContain('FL_ASSETS_DIR');
  });
  it('a symlink-free traversal attempt with a NUL byte is refused', async () => {
    expect((await rawGet(`${base}/assets/blob.bin%00.png`)).status).toBeGreaterThanOrEqual(400);
  });
});
