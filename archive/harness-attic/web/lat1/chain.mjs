// lat1: waits (inside this process) until no playwall.mjs / trace.mjs run of this tree is alive, then runs series.sh with the given arguments. usage: node tools/chain.mjs <series-name> <order1> <order2> ...
import fs from 'node:fs';
import { spawn } from 'node:child_process';
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const alive = () => fs.readdirSync('/proc').filter((d) => /^\d+$/.test(d)).some((d) => { try { return /tools\/(playwall|trace)\.mjs/.test(fs.readFileSync(`/proc/${d}/cmdline`, 'utf8').replace(/\0/g, ' ')); } catch { return false; } });
while (alive()) await sleep(5000);
const p = spawn('bash', ['tools/series.sh', ...process.argv.slice(2)], { cwd: '/home/ubuntu/work/signature-studio-develop/lift/out-lat1', stdio: 'inherit' });
p.on('exit', (c) => process.exit(c ?? 0));
