// Stages the assets that the app serves at /assets/ (app/tools/assets-middleware.ts) for one bundle, from the two directories that app/scripts/pack-assets.mjs writes:
// raw/<name>/ holds manifest.json, and br/<name>/ holds each file brotli-compressed under its name without .br (the bucket's key; the Worker serves it with Content-Encoding br).
// Each br file is linked as <file>.br, the name the middleware looks for, so the bytes are the ones the Worker serves from R2. The links point at the bundle; nothing is copied.
import fs from 'node:fs';
import path from 'node:path';

export function stageAssets({ rawDir, brDir, assets, bundle }) {
  if (!fs.existsSync(brDir)) throw new Error(`no ${brDir}: the brotli directory of the bundle must be next to raw/`);
  const dir = path.join(assets, bundle);
  fs.mkdirSync(dir, { recursive: true });
  const files = (d) => fs.readdirSync(d, { withFileTypes: true }).filter((e) => e.isFile()).map((e) => e.name);
  for (const f of files(rawDir)) fs.symlinkSync(path.join(rawDir, f), path.join(dir, f));
  for (const f of files(brDir)) fs.symlinkSync(path.join(brDir, f), path.join(dir, `${f}.br`));
  fs.copyFileSync(path.join(rawDir, 'manifest.json'), path.join(assets, 'manifest.json'));
  return dir;
}
