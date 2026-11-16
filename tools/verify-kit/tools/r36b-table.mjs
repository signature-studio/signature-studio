// R3-6b helper (read-only): one row per session (overall order) from the run folders, plus per-arm totals.
import fs from 'node:fs';
const WT = `${process.env.HOME}/.worktrees/github.com/signature-studio/signature-studio`;
const arms = { A: { label: 'r36arma', wt: `${WT}/r3-6-arm-a` }, B: { label: 'r36armb', wt: `${WT}/r3-6-arm-b` } };
const order = ['A', 'B', 'A', 'B', 'A', 'B', 'A', 'B', 'A', 'B', 'A', 'B'];
const rowsOut = [];
const totals = { A: { notVisible: 0, popupSeen: 0, completed: 0, seconds: 0, n: 0, runs: 0 }, B: { notVisible: 0, popupSeen: 0, completed: 0, seconds: 0, n: 0, runs: 0 } };
const seen = {};
for (const arm of ['A', 'B']) {
  const out = `${arms[arm].wt}/tools/verify-kit/out`;
  seen[arm] = fs.readdirSync(out).filter((d) => d.startsWith(`r36-${arms[arm].label}-chrome-`)).sort();
}
order.forEach((arm, i) => {
  const k = order.slice(0, i + 1).filter((x) => x === arm).length;
  const folder = `r36-${arms[arm].label}-chrome-${k}`;
  const dir = `${arms[arm].wt}/tools/verify-kit/out/${folder}`;
  const parsed = JSON.parse(fs.readFileSync(`${dir}/report.json`, 'utf8'));
  const r = Array.isArray(parsed) ? parsed[0] : parsed;
  const m = r.menuCycles;
  const nv = m.notVisible.length;
  const pms = m.popupMissingSurface.length;
  const np = `${dir}/netlog-sdrive.json`;
  const js = {};
  for (const x of fs.readFileSync(np, 'utf8').matchAll(/index-[A-Za-z0-9_-]+\.js/g)) js[x[0]] = (js[x[0]] ?? 0) + 1;
  const jsText = Object.entries(js).map(([a, b]) => `${a} ${b}件`).join(', ');
  const t = totals[arm];
  t.notVisible += nv; t.popupSeen += m.popupSeen; t.completed += m.completed; t.seconds += r.seconds; t.n += m.n; t.runs += 1;
  rowsOut.push(`| ${i + 1} | ${arm} | ${arms[arm].label} | ${folder} | ${r.seconds} | ${r.state} | ${r.stopped} | ${(r.opErrors ?? []).length} | ${m.n} | ${m.popupSeen} | ${nv} | ${pms} | ${m.completed} | ${m.stoppedAt ?? '-'} | ${m.popupSeen + nv + pms === m.completed ? '一致' : '不一致'} | ${jsText} | ${r.ok ? 'ok' : 'not ok'} |`);
});
console.log('| 順 | 腕 | label | 出力フォルダ | seconds | state | stopped | opErrors | n | popupSeen | notVisible(取りこぼし) | popupMissingSurface | completed | stoppedAt | 等式 | 配ったページ(netlog) | report.ok |');
console.log('|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|');
rowsOut.forEach((x) => console.log(x));
console.log('');
console.log(JSON.stringify(totals));
console.log(`folders A: ${seen.A.join(' ')}`);
console.log(`folders B: ${seen.B.join(' ')}`);
