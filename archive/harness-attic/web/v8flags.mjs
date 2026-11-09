// Prints the V8 flags of the Chrome in use (--js-flags=--help), one process, killed after the output stops.
import { spawn } from 'node:child_process';
const p = spawn(process.env.CHROME_BIN || '/usr/bin/google-chrome', ['--headless=new', '--no-sandbox', '--disable-gpu', '--disable-dev-shm-usage', `--user-data-dir=${process.argv[2]}`, '--js-flags=--help', 'about:blank'], { stdio: ['ignore', 'pipe', 'pipe'] });
let out = '';
p.stdout.on('data', (d) => { out += d; });
p.stderr.on('data', (d) => { out += d; });
const done = new Promise((r) => p.on('exit', r));
const t = setTimeout(() => p.kill('SIGKILL'), 15000);
await done; clearTimeout(t);
console.log(out);
