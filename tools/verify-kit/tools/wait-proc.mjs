// wait-proc.mjs PID [MAXMIN]: exits 0 when the process PID is gone (a run of a chain started earlier has ended), exit 1 after MAXMIN minutes (default 30). Prints one line.
import fs from 'node:fs';
const pid = process.argv[2], maxMin = Number(process.argv[3] ?? 30), t0 = Date.now();
const alive = () => fs.existsSync('/proc/' + pid) && !/^State:\s+Z/m.test(fs.readFileSync('/proc/' + pid + '/status', 'utf8'));
for (;;) {
  if (!alive()) { console.log(`process ${pid} is gone after ${Math.round((Date.now() - t0) / 1000)} s`); process.exit(0); }
  if (Date.now() - t0 > maxMin * 60000) { console.log(`process ${pid} still runs after ${maxMin} min`); process.exit(1); }
  await new Promise((r) => setTimeout(r, 2000));
}
