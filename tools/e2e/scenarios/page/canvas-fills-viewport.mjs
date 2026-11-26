// page/canvas-fills-viewport: the #desktop box lies inside the page's viewport, keeps the guest picture's aspect ratio, and covers most of the viewport (the picture follows the window).
import { defineScenario } from '../../lib/scenario.mjs';

export default defineScenario({
  id: 'page/canvas-fills-viewport',
  covers: ['page/module/resize', 'page/canvas/fills-window'],
  async run({ guest, page }) {
    const v = await page.evaluate(() => ({ w: window.innerWidth, h: window.innerHeight, dpr: window.devicePixelRatio }));
    const d = await guest.desk();
    const inside = d.w > 0 && d.h > 0 && d.x >= 0 && d.y >= 0 && d.x + d.w <= v.w + 1 && d.y + d.h <= v.h + 1;
    const aspect = Math.abs(d.gw / d.gh - d.w / d.h) < 0.01;
    const share = (d.w * d.h) / (v.w * v.h);
    const ok = inside && aspect && share >= 0.85;
    return { ok, engineState: (await guest.state())?.state ?? null, notes: [`viewport ${v.w}x${v.h} DPR ${v.dpr}; #desktop ${d.w}x${d.h} at ${d.x},${d.y}; guest ${d.gw}x${d.gh}; share ${share.toFixed(3)}`, `inside ${inside}, aspect kept ${aspect}`] };
  },
});
