// node run-draw.mjs [name=draw-coi] [gpu=1] [n=200] [present=raf,timeout,blocking,bitmap,mainthread_put]
// 1) the autonomous page measurements (draw-main.js), 2) presentation tests driven over CDP: while the Worker draws in a given loop style the driver
// takes tiny screenshots of the displayed canvas (the frame number is encoded in pixel 2 of row 0) and counts Page.screencastFrame events.
import { withBrowser, argv } from './lib.mjs';
import { sleep } from '../cdp.mjs';
const name = argv.name || 'draw-coi';
const styles = (argv.present || 'raf,timeout,blocking,bitmap,mainthread_put').split(',');
const MS = 3500;
const out = await withBrowser({ name, page: `/coi/draw.html?n=${argv.n || 200}`, timeoutS: 600, gpu: argv.gpu === '1', args: argv.flags ? argv.flags.split(' ') : [] }, async ({ waitDone, S, evalJSON, cdp, sessionId }) => {
  const R = await waitDone(500);
  R.present = {};
  for (const style of styles) {
    // (a) screenshots of the displayed canvas
    await evalJSON(`window.presentStart(${JSON.stringify(style)}, ${MS})`);
    const t0 = Date.now();
    const shots = [];
    while (Date.now() - t0 < MS - 300) {
      await sleep(400);
      try {
        const shot = await S('Page.captureScreenshot', { format: 'png', clip: { x: 0, y: 0, width: 16, height: 2, scale: 1 } });
        const px = await evalJSON(`window.decodeShot(${JSON.stringify(shot.data)})`);
        shots.push({ t_ms: Date.now() - t0, rgba: px.slice(0, 4) });
      } catch (e) { shots.push({ t_ms: Date.now() - t0, error: String(e).slice(0, 100) }); }
    }
    const info = JSON.parse(await evalJSON(`new Promise((res) => { const f = () => window.__present.done ? res(JSON.stringify({info: window.__present.info, bitmaps: window.__present.bitmaps})) : setTimeout(f, 50); f(); })`));
    // (b) screencast frame count during a second run of the same style
    let frames = 0;
    const onEv = (m) => { if (m.method === 'Page.screencastFrame' && m.sessionId === sessionId) { frames++; cdp.send('Page.screencastFrameAck', { sessionId: m.params.sessionId }, sessionId).catch(() => {}); } };
    cdp.on(onEv);
    await evalJSON(`window.presentStart(${JSON.stringify(style)}, ${MS})`);
    await S('Page.startScreencast', { format: 'jpeg', quality: 10, maxWidth: 160, maxHeight: 90, everyNthFrame: 1 });
    const t1 = Date.now();
    await evalJSON(`new Promise((res) => { const f = () => window.__present.done ? res(1) : setTimeout(f, 50); f(); })`);
    const el = Date.now() - t1;
    await S('Page.stopScreencast');
    R.present[style] = { workerInfo: info.info, pageBitmaps: info.bitmaps, shots, screencast_frames: frames, screencast_window_ms: el, screencast_fps: +(frames * 1000 / el).toFixed(1) };
    console.error(`present ${style}: shots=${shots.map((s) => s.rgba ? s.rgba[0] : 'x').join(',')} screencast=${frames} frames in ${el} ms`);
  }
  return R;
});
console.log(JSON.stringify({ name, error: out.error, pids_left: out.chrome_pids_left }));
