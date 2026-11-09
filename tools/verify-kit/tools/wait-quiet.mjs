// wait-quiet.mjs MAXLOAD MAXWAIT_MIN: returns (exit 0) when the 1-minute load average is <= MAXLOAD, the I/O pressure avg10 is 0 and the swap in/out counters (/proc/vmstat pswpin, pswpout) do not move over a 2 s sample (a measurement run starts only in a quiet window, decision 0277; the swap rule is the PM's of 2026-11-07 04:16); exit 1 after MAXWAIT_MIN minutes. Prints one line with the load, the I/O pressure, the time waited ("after N s") and the memory state (MemAvailable, swap used, swap in/out over the sample).
import fs from 'node:fs';
const maxLoad = Number(process.argv[2] ?? 8), maxMin = Number(process.argv[3] ?? 10); const t0 = Date.now();
const load = () => Number(fs.readFileSync('/proc/loadavg', 'utf8').split(' ')[0]);
const io = () => Number(/avg10=([\d.]+)/.exec(fs.readFileSync('/proc/pressure/io', 'utf8'))[1]);
const vm = () => { const t = fs.readFileSync('/proc/vmstat', 'utf8'); return [Number(/^pswpin (\d+)/m.exec(t)[1]), Number(/^pswpout (\d+)/m.exec(t)[1])]; };
const mem = () => { const t = fs.readFileSync('/proc/meminfo', 'utf8'); const g = (k) => Number(new RegExp('^' + k + ':\\s+(\\d+)', 'm').exec(t)[1]); return { avail: Math.round(g('MemAvailable') / 1024), swapUsed: Math.round((g('SwapTotal') - g('SwapFree')) / 1024) }; };
const nap = (ms) => new Promise((r) => setTimeout(r, ms));
for (;;) {
  const l = load(), i = io(); const a = vm(); await nap(2000); const b = vm(); const si = b[0] - a[0], so = b[1] - a[1]; const m = mem();
  const memtxt = `mem avail ${m.avail} MB, swap used ${m.swapUsed} MB, swap in/out ${si}/${so} pages (2 s)`;
  if (l <= maxLoad && i === 0 && si === 0 && so === 0) { console.log(`quiet: load ${l} io ${i} after ${Math.round((Date.now() - t0) / 1000)} s; ${memtxt}`); process.exit(0); }
  if ((Date.now() - t0) / 60000 > maxMin) { console.log(`NOT quiet after ${maxMin} min: load ${l} io ${i}; ${memtxt}`); process.exit(1); }
  await nap(3000);
}
