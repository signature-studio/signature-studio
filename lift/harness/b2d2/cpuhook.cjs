// hle1 (decision 0151): node --require preload. Before every write to stdout (fd 1) that contains a g3 snapshot line, prints a line `hle1-maincpu: <seconds> s` with the CPU time of the MAIN thread
// (process.threadCpuUsage(): user + system), which is the thread the wasm engine runs on (JSPI coroutines). The wall clock of a loaded machine is not a measurement of the engine; the main thread's CPU time is much less sensitive to it.
const fs = require('fs');
const ws = fs.writeSync;
fs.writeSync = function (fd, buf, ...rest) {
  if (fd === 1) {
    try {
      const s = typeof buf === 'string' ? buf : Buffer.from(buf.buffer, buf.byteOffset, buf.byteLength).toString('latin1');
      if (s.indexOf('g3-snap:') >= 0) {
        const u = process.threadCpuUsage();
        ws.call(fs, 1, 'hle1-maincpu: ' + ((u.user + u.system) / 1e6).toFixed(3) + ' s\n');
        ws.call(fs, 1, 'hle2-hrtime-us: ' + (process.hrtime.bigint() / 1000n).toString() + '\n');
      }
    } catch (e) { /* never disturb the engine */ }
  }
  return ws.call(fs, fd, buf, ...rest);
};
