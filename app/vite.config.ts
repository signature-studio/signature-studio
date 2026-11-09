import { defineConfig, type Plugin } from 'vite';
import { assetsMiddleware } from './tools/assets-middleware.ts';
import { perfLogPath, perfMiddleware } from './tools/perf-middleware.ts';
import { CSP, ISOLATION_HEADERS, PRIVACY_HEADERS } from './shared/headers.ts';

// /assets/ is reserved for the big assets (R2 in production), so the build puts its own files under /static/.
// FL_ASSETS_DIR: the directory served at /assets/ (large, product-derived assets are not in git: wasm, packs, manifest.json)
const assetsDir = process.env.FL_ASSETS_DIR ?? '';

// FL_PERF_LOG: where POST /__perf (the page's measurement beacon, decision 0152) appends its JSONL; default: perf-log.jsonl in the parent directory of FL_ASSETS_DIR
const perfLog = perfLogPath(process.env);

function flAssets(): Plugin {
  return {
    name: 'fl-assets',
    configureServer(server) { server.middlewares.use(perfMiddleware(perfLog)); server.middlewares.use(assetsMiddleware(assetsDir)); },
    configurePreviewServer(server) { server.middlewares.use(perfMiddleware(perfLog)); server.middlewares.use(assetsMiddleware(assetsDir)); },
  };
}

const headers = { ...ISOLATION_HEADERS, ...PRIVACY_HEADERS };

// Vite answers only localhost and IP addresses by default; behind `tailscale serve` the Host header is the tailnet's MagicDNS name (<machine>.<tailnet>.ts.net)
const allowedHosts = ['.ts.net'];

export default defineConfig({
  plugins: [flAssets()],
  worker: { format: 'es' },
  build: { target: 'es2023', assetsDir: 'static', sourcemap: false, assetsInlineLimit: 0, modulePreload: { polyfill: false } },
  server: { host: '127.0.0.1', port: 5173, strictPort: false, headers, allowedHosts, fs: { strict: true } },
  // the preview server serves the production build, so it also carries the Content-Security-Policy of the Worker
  preview: { host: '127.0.0.1', port: 4173, headers: { ...headers, 'Content-Security-Policy': CSP }, allowedHosts },
});
