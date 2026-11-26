// R17 (#1408) mouse-1000hz-no-dropout. Report (L-049, L-051, #1349): while the transport plays, a mouse moved at 1000 Hz or more does not cut the sound. Expectation:
// the audio output starts (a press on the status line), the transport plays (Space); then the mouse is moved over the desktop for 20 s at 1000 Hz or more (the
// sends are counted); the underrun quanta of the output in those 20 s are 0, and the quanta played in them are more than 0.
import { audioNow, geometry, sleep, startDefault, result } from './common.mjs';

const SCENARIO = {
  name: 'mouse-1000hz-no-dropout',
  report: 'R17',
  expect: 'playing (Space) and the mouse moved over the desktop at 1000 Hz or more for 20 s: 0 underrun quanta, the output played',
  expectFail: null,
};
const MOVE_MS = 20000;
const MIN_HZ = 1000;
const MOVES_PER_TICK = 2;

const waitFor = async (guest, pred, ms) => {
  const t0 = Date.now();
  for (;;) {
    const a = await audioNow(guest);
    if (a && pred(a)) return a;
    if (Date.now() - t0 >= ms) return a;
    await sleep(250);
  }
};

export default {
  ...SCENARIO,
  async run({ guest, log }) {
    const start = await startDefault(guest, log);
    if (!start.ok) return result(false, start.state, start.notes);
    const st = await guest.ev("(() => { const r = document.getElementById('status').getBoundingClientRect(); return { x: r.x + r.width / 2, y: r.y + r.height / 2 }; })()");
    await guest.clickCss(st.x, st.y);
    const running = await waitFor(guest, (a) => a.ctx === 'running', 15000);
    await guest.focusDesktop();
    await guest.key('Space', 'Space', 32);
    const playing = await waitFor(guest, (a) => a.playing > 0, 15000);
    const g = await geometry(guest);
    const c = { x: g.x + g.w / 2, y: g.y + g.h / 2 };
    const a0 = await audioNow(guest);
    const cdp = guest.cdp;
    let sent = 0;
    const t0 = performance.now();
    await new Promise((resolve) => {
      const timer = setInterval(() => {
        if (performance.now() - t0 >= MOVE_MS) { clearInterval(timer); resolve(); return; }
        for (let k = 0; k < MOVES_PER_TICK; k++) {
          const n = sent + k;
          const x = c.x + (n % 2 === 0 ? -60 : 60), y = c.y + ((n >> 1) % 2 === 0 ? -40 : 40);
          cdp.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x, y, button: 'none', buttons: 0, pointerType: 'mouse' }).catch(() => {});
        }
        sent += MOVES_PER_TICK;
      }, 1);
    });
    const secs = (performance.now() - t0) / 1000;
    await sleep(500);
    const a1 = await audioNow(guest);
    const hz = sent / secs;
    const under = a1 && a0 ? a1.ur - a0.ur : null;
    const played = a1 && a0 ? a1.playing - a0.playing : null;
    const ok = under === 0 && played > 0 && hz >= MIN_HZ && (await guest.state())?.state === 'engine-idle';
    const notes = [...start.notes, `audio: status ${a1?.status}, context ${running?.ctx}, error ${a1?.err ?? 'none'}; playing before the move ${playing?.playing ?? null} quanta; mouse moves ${sent} in ${secs.toFixed(1)} s (${hz.toFixed(0)} Hz); underrun quanta ${under}; quanta played ${played}`];
    log(notes.at(-1));
    return result(ok, (await guest.state())?.state ?? null, notes, { sent, hz: Math.round(hz), underrunQuanta: under, played });
  },
};
