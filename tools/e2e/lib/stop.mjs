// The engine's stop as the E2E's JSON carries it (Issue #1421): the facts of the engine's own stop report, read from the engine log of the page with the same
// rules as the measurement beacon (app/src/perf/exit-message.ts, which this file imports, so that the two cannot disagree), and the report's lines.
// The report is the harness' (lift/harness/fruity.c): "fruity: == stop:", "runtime message:", "abort kind:", the guest registers R[0..15] and the guest stack at rsp
// (the first word is the guest return address). The engine's stderr line of the stop ("fle: ...") is batched after the stdout lines of the report, so it can
// come after the report in the log: it is taken from the whole log. Pure: lines in, object out.
import { engineExitMessage, engineStopFacts } from '../../../app/src/perf/exit-message.ts';

const REPORT_MAX = 400;
const TAIL_MAX = 60;
const WINDOW = 2000;
const strip = (l) => l.replace(/^\[err\] ?/, '').replace(/^fruity: /, '');
const REG = /\b(r[a-z0-9]+|rsp|rbp|rip)=(0x[0-9a-fA-F]+|0)\b/g;   // %#llx prints 0 as 0


/** the engine's stop of one run: `lines` is the engine log (the page's engineLog(), in the order the page keeps it), `exit` is the page's engine.exit fact ({ reason, code, message }) or null */
export function stopOf(lines, exit) {
  const stopAt = lines.findLastIndex((l) => strip(l).startsWith('== stop:'));
  const fleAt = lines.findLastIndex((l) => strip(l).startsWith('fle: '));
  const info = exit && exit.reason ? { reason: exit.reason, code: exit.code, message: exit.message ?? '' } : null;
  let report, window;
  if (stopAt < 0) {
    report = lines.slice(Math.max(0, lines.length - TAIL_MAX));
    window = lines.slice(Math.max(0, lines.length - WINDOW));
  } else {
    report = lines.slice(stopAt, stopAt + REPORT_MAX);
    const body = lines.slice(stopAt, stopAt + WINDOW - 1);
    const fle = fleAt >= 0 && (fleAt < stopAt || fleAt >= stopAt + WINDOW - 1) ? [lines[fleAt]] : [];
    if (fleAt >= 0 && (fleAt < stopAt || fleAt >= stopAt + REPORT_MAX)) report = [lines[fleAt], ...report];
    window = [...body, ...fle];
  }
  const regs = {};
  let ret = null;
  let inRegs = false;
  for (const l of report) {
    const s = strip(l);
    if (s.startsWith('guest registers at the stop')) inRegs = true;
    else if (inRegs && /^\s+\w+=(0x[0-9a-fA-F]+|0)\b/.test(s)) for (const m of s.matchAll(REG)) regs[m[1]] = m[2];
    else inRegs = false;
    if (ret === null && s.startsWith('guest stack at rsp')) ret = /:\s*(0x[0-9a-fA-F]+)/.exec(s)?.[1] ?? null;
  }
  const facts = engineStopFacts(window);
  return {
    exit: info ? { reason: info.reason, code: info.code } : null,
    message: engineExitMessage(window, info),
    stop: facts.stop,
    evidence: facts.evidence,
    returnAddress: ret,
    registers: regs,
    report: report.map((l) => l.replace(/^\[err\] ?/, '')),
  };
}
