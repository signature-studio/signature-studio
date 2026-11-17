// Decision 0220: how does createWritable().write() of blocks cut from one big buffer end? (A bug hunt: the stored dll pack came out as a multiple of 4 MiB.) Chrome only.
import fs from 'node:fs';
import { Cdp, launchChrome, shutdown, sleep } from './cdp.mjs';
const port = Number(process.argv[2] ?? 8801);
const profile = '/home/ubuntu/work/signature-studio-develop/lift/out-cache1/work/opfs-probe-profile';
fs.rmSync(profile, { recursive: true, force: true });
const chrome = await launchChrome({ profile, extraArgs: ['--no-proxy-server'] });
const cdp = new Cdp(chrome.version.webSocketDebuggerUrl);
await cdp.opened;
try {
  const { targetId } = await cdp.send('Target.createTarget', { url: 'about:blank' });
  const { sessionId } = await cdp.send('Target.attachToTarget', { targetId, flatten: true });
  await cdp.send('Page.enable', {}, sessionId); await cdp.send('Runtime.enable', {}, sessionId);
  await cdp.send('Page.navigate', { url: `http://127.0.0.1:${port}/p.html` }, sessionId);
  await sleep(500);
  const expr = `(async () => {
    const total = 70573808, buf = new Uint8Array(total); for (let i = 0; i < total; i += 4099) buf[i] = i & 255;
    const root = await navigator.storage.getDirectory();
    const out = [];
    for (const mode of ['one write', 'blocks 4MiB chained', 'blocks 4MiB awaited', 'chunks 64KiB awaited']) {
      const fh = await root.getFileHandle('probe-' + mode.replace(/\\W/g, ''), { create: true });
      const w = await fh.createWritable();
      let n = 0;
      if (mode === 'one write') { await w.write(buf); n = total; }
      else if (mode === 'blocks 4MiB chained') { let chain = Promise.resolve(), at = 0; while (at < total) { const end = Math.min(total, at + 4194304 + 1000); const part = buf.subarray(at, end); chain = chain.then(() => w.write(part)); at = end; } await chain; n = total; }
      else if (mode === 'blocks 4MiB awaited') { let at = 0; while (at < total) { const end = Math.min(total, at + 4194304 + 1000); await w.write(buf.subarray(at, end)); at = end; } n = total; }
      else { let at = 0; while (at < total) { const end = Math.min(total, at + 65536); await w.write(buf.subarray(at, end)); at = end; } n = total; }
      await w.close();
      const f = await fh.getFile();
      out.push(mode + ': wrote ' + n + ' -> file ' + f.size + (f.size === n ? ' OK' : ' SHORT by ' + (n - f.size)));
    }
    return out;
  })()`;
  const r = await cdp.send('Runtime.evaluate', { expression: expr, awaitPromise: true, returnByValue: true }, sessionId);
  console.log(JSON.stringify(r.result.value ?? r.exceptionDetails, null, 1));
} finally { await shutdown(chrome, cdp); }
