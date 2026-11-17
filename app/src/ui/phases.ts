// The start-up of the real engine as the page shows it: download -> compile -> engine running -> first pixel -> stopped. The stages are derived from a timeline of named instants
// (milliseconds since the navigation started, i.e. performance.now() of the page), so that the same function serves the screen and the tests. Nothing here touches the DOM.
//
// Marks: manifest_loaded, download_start (first byte of any asset), download_done (every asset verified), compile_done (the wasm module is compiled; the compile overlaps the download,
// so this is the end of the compile tail), worker_ready (the Worker imported the glue), engine_start (the page told the Worker to run), main_start (the engine's main() began),
// first_pixel, stop_frame, idle, exit (the engine exited or stopped; the page was told), error.

export type StageId = 'download' | 'compile' | 'engine' | 'first-pixel' | 'end';
export type StageStatus = 'pending' | 'active' | 'done' | 'failed';

export interface StageRow { id: StageId; label: string; status: StageStatus; startedAt: number | null; endedAt: number | null; detail: string }

export interface StageContext {
  /** the current time in the same unit as the marks */
  now: number;
  /** the stage that was running when the page failed, if it did */
  failedStage?: StageId | null;
  /** the bundle's "seconds from the engine start to the first pixel" (hint only) */
  hintFirstPixelS?: number | undefined;
  /** how the engine ended, for the last row */
  endKind?: 'stopped' | 'finished' | 'trap' | 'idle' | null;
  /** decision 0220: where the assets came from, e.g. "HTTP cache ×4"; shown in the Download row */
  assetSources?: string;
}

export class Timeline {
  private readonly marks = new Map<string, number>();

  /** the first report of a mark wins: a later report of the same instant (a second event of the same kind) does not move it */
  mark(name: string, at: number): void { if (!this.marks.has(name)) this.marks.set(name, at); }
  has(name: string): boolean { return this.marks.has(name); }
  get(name: string): number | null { return this.marks.get(name) ?? null; }
  toJSON(): Record<string, number> { return Object.fromEntries(this.marks); }
}

const s = (ms: number): string => `${(ms / 1000).toFixed(1)} s`;

export function stageRows(tl: Timeline, ctx: StageContext): StageRow[] {
  const dlStart = tl.get('download_start'), dlDone = tl.get('download_done'), cDone = tl.get('compile_done');
  const eStart = tl.get('engine_start'), mainStart = tl.get('main_start'), first = tl.get('first_pixel');
  const end = tl.get('exit') ?? tl.get('idle');
  const failed = ctx.failedStage ?? null;
  const row = (id: StageId, label: string, startedAt: number | null, endedAt: number | null, detail: string, activeWhen: boolean): StageRow => {
    let status: StageStatus = 'pending';
    if (failed === id) status = 'failed';
    else if (endedAt !== null) status = 'done';
    else if (startedAt !== null && activeWhen && failed === null) status = 'active';
    return { id, label, status, startedAt, endedAt, detail };
  };

  const download = row('download', 'Download', dlStart, dlDone, dlDone !== null && dlStart !== null ? `all assets verified after ${s(dlDone - dlStart)} (at ${s(dlDone)})${ctx.assetSources ? `; from: ${ctx.assetSources}` : ''}` : dlStart !== null ? `since ${s(dlStart)}` : '', true);
  const compile = row('compile', 'Compile', dlStart, cDone, cDone !== null ? `wasm compiled at ${s(cDone)}${dlDone !== null ? ` (${s(Math.max(0, cDone - dlDone))} after the last byte; it compiles while it downloads)` : ''}` : dlStart !== null ? 'compiles while it downloads' : '', true);
  let engineDetail = '';
  if (eStart !== null) {
    const upto = end ?? ctx.now;
    const run = upto - eStart;
    const hint = ctx.hintFirstPixelS !== undefined && first === null ? ` · the bundle expects the first pixel about ${ctx.hintFirstPixelS.toFixed(0)} s after the engine starts` : '';
    engineDetail = `${end !== null ? 'ran' : 'running'} ${s(run)}${mainStart !== null ? ` (main() began at ${s(mainStart)})` : ' (glue, wasm instantiation, files)'}${hint}`;
  }
  const engine = row('engine', 'Engine running', eStart, end, engineDetail, true);
  const firstPixel = row('first-pixel', 'First pixel', eStart, first, first !== null ? `at ${s(first)}${eStart !== null ? ` (${s(first - eStart)} after the engine started)` : ''}` : '', eStart !== null && end === null);
  const endLabel = ctx.endKind === 'idle' ? 'Idle' : ctx.endKind === 'finished' ? 'Finished' : 'Stopped';
  const endDetail = end !== null ? `${ctx.endKind ?? 'ended'} at ${s(end)}` : '';
  const last = row('end', endLabel, eStart, end, endDetail, false);
  if (ctx.endKind === 'trap' && end !== null) last.status = 'failed';
  return [download, compile, engine, firstPixel, last];
}

/** MB/s over the time between the first and the latest progress report of an asset */
export function transferSpeed(first: { loaded: number; at: number } | undefined, latest: { loaded: number; at: number }): number | null {
  if (!first || latest.at <= first.at || latest.loaded <= first.loaded) return null;
  return (latest.loaded - first.loaded) / 1048576 / ((latest.at - first.at) / 1000);
}
