import { startServer } from '../serve.mjs';
const { server, port } = await startServer({ dist: process.env.WEB_DIST });
for (const p of ['/jspi/index-jspi.html?auto=1', '/jspi/fle-worker-jspi.js', '/dist/flengine6.web.js', '/dist/data/seed/vfs-seed-fonts.tsv', '/index.html']) {
  const t = Date.now();
  const ac = new AbortController(); const to = setTimeout(() => ac.abort(), 5000);
  try { const r = await fetch(`http://127.0.0.1:${port}${p}`, { signal: ac.signal }); const b = await r.arrayBuffer(); console.log(p, r.status, b.byteLength, Date.now() - t, 'ms'); }
  catch (e) { console.log(p, 'ERR', String(e).slice(0, 100)); }
  clearTimeout(to);
}
server.close();
