// What the page says when the real engine has stopped (decision 0141): WHAT stopped it and WHERE it was, in the engine's own words and in the page's own facts - never a guess about what to fix.
//   what   the engine's stop message ("fle: ..." = it stopped at a wall of the lifted engine; "limit: ..." = a limit of the harness) and its exit code
//   where  the engine's stop line, the last input the page sent and how long before the stop, how many input events were never processed, the last time the desktop picture changed (decision 0223: the page knows no window)
// Pure: the facts are passed in, the text comes out, so the tests can state exactly what is said.
export interface StopFacts {
  info: { reason: 'finished' | 'stopped' | 'trap'; code: number; message: string };
  /** the engine's own message ("fle: indirect call to 0 ...") or '' */
  runtimeMessage: string;
  stopLine: string;
  /** the user (or a test) ended the session with 'End session' / stop() - the engine's own message then says only that the host ended its wait */
  endedFromPage?: boolean;
  /** ms, same clock as the times below */
  now: number;
  lastInput: { desc: string; at: number } | null;
  pendingInput: number;
  lastPictureChange: { at: number } | null;
}

export function stopLines(f: StopFacts): string[] {
  const ago = (t: number): string => `${((f.now - t) / 1000).toFixed(1)} s before the stop`;
  const wall = f.runtimeMessage.startsWith('fle:');
  const head = f.endedFromPage ? `The session was ended from the page (End session).${f.runtimeMessage !== '' ? ` The engine's last words: ${f.runtimeMessage}` : ''}` : f.runtimeMessage !== '' ? `The engine stopped${wall ? ' at a wall' : ''}: ${f.runtimeMessage}` : `The engine ${f.info.reason === 'finished' ? 'finished' : 'stopped'} (${f.info.message}).`;
  const lines = [head, `${f.info.code >= 0 ? `exit code ${f.info.code}` : 'no exit code (the Worker was terminated)'}${f.stopLine !== '' ? ` · ${f.stopLine}` : ''}`];
  if (f.lastInput) lines.push(`Last input sent: ${f.lastInput.desc}, ${ago(f.lastInput.at)}; ${f.pendingInput} input events were still pending (never processed).`);
  else lines.push('No input was sent before the stop.');
  const p = f.lastPictureChange;
  if (p) lines.push(`The desktop picture last changed ${ago(p.at)}.`);
  lines.push('The desktop shows the last picture; the engine no longer runs, so nothing answers input and the page sends none.');
  return lines;
}
