// wait-until.mjs HH:MM [next]: exits at that local time today (a scheduled start of a measurement chain; the PM asked ci1 to hold its Chrome runs until then unless another agent returns the window). A time already past today exits at once, unless the 2nd argument is `next` (then it waits until that time tomorrow: use it across midnight).
const [h, m] = (process.argv[2] ?? '0:0').split(':').map(Number); const t = new Date(); t.setHours(h, m, 0, 0);
if (process.argv[3] === 'next' && t.getTime() <= Date.now()) t.setDate(t.getDate() + 1);
const ms = t.getTime() - Date.now(); console.log(`waiting ${Math.max(0, Math.round(ms / 1000))} s until ${process.argv[2]}`);
if (ms > 0) await new Promise((r) => setTimeout(r, ms));
console.log('go');
