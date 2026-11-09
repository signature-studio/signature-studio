// Reads what the lifted engine prints (stdout and stderr lines, in the order they were printed) into the few things the UI shows: why and how it stopped, the guest's windows,
// the guest's threads, and the surfaces it reported with their SHA-256. The line formats are those of lift/harness/fruity.c (the "fruity:" lines), lift/runtime (window and thread
// tables) and lift/harness/fruity_surface.c ("wf1-surface:"), as captured in decision 0134's Chrome runs. A line that does not match is ignored: nothing here can fail.

export interface EngineWindow {
  handle: string;
  lid: number;
  className: string;
  title: string;
  x: number;
  y: number;
  width: number;
  height: number;
  visible: boolean;
  enabled: boolean;
  /** 'parent' / 'owner' handle, or '' for a top-level window without an owner */
  relation: string;
  z: number;
}

export interface EngineThread { index: number; id: string; state: string }
export interface EngineSurfaceReport { event: string; lid: number; width: number; height: number; stride: number; nonzero: number; rgbaSha256: string }

export interface EngineLogSummary {
  /** "fruity: == stop: ..." */
  stopLine: string;
  /** the engine's own words for why it stopped ("fle: RaiseException: ... stopped here"), without a "fruity: runtime message: " prefix */
  runtimeMessage: string;
  exitCode: number | null;
  threadSummary: string;
  threads: EngineThread[];
  windows: EngineWindow[];
  surfaces: EngineSurfaceReport[];
}

const stripStream = (l: string): string => l.replace(/^\[err\] ?/, '');

const WINDOW_HEADER = /^\s*window table: (\d+) entries, (\d+) live/;
const WINDOW_LINE = /^\s*handle (0x[0-9a-f]+) lid (\d+) class "(.*?)" title "(.*?)" rect \((-?\d+), (-?\d+), (-?\d+) x (-?\d+)\) style (?:0x[0-9a-f]+|\d+) exstyle (?:0x[0-9a-f]+|\d+) (?:(parent|owner) (0x[0-9a-f]+|0) )?visible (\d) enabled (\d) z (-?\d+) thread \d+\s*$/;
const THREADS_HEADER = /^\s*threads: (\d+) \(running index (-?\d+)\)/;
const THREAD_LINE = /^\s*thread (\d+): id (0x[0-9a-f]+) handle \S+ (.+?) start \S+/;
const SURFACE_LINE = /^wf1-surface: event=(\S+) lid=(\d+) size=(\d+)x(\d+) stride=(\d+) pixels=\S+ nonzero=(\d+) rgba_sha256=([0-9a-f]{64})/;

export function summarizeEngineLog(lines: readonly string[]): EngineLogSummary {
  const out: EngineLogSummary = { stopLine: '', runtimeMessage: '', exitCode: null, threadSummary: '', threads: [], windows: [], surfaces: [] };
  let windows: EngineWindow[] | null = null;
  let threads: EngineThread[] | null = null;
  for (const raw of lines) {
    const l = stripStream(raw);
    let m: RegExpExecArray | null;
    if (l.startsWith('fruity: == stop:')) { out.stopLine = l.slice('fruity: '.length); continue; }
    if (l.startsWith('fruity: runtime message: ')) { out.runtimeMessage = l.slice('fruity: runtime message: '.length); continue; }
    // decision 0197: the wasm harness' abort path (trace_fruity.c __wrap_abort, e.g. fle_pruned()) writes these two lines without the "fruity: " prefix
    if (l.startsWith('== stop:') && !out.stopLine) { out.stopLine = l; continue; }
    if (l.startsWith('runtime message: ') && !out.runtimeMessage) { out.runtimeMessage = l.slice('runtime message: '.length); continue; }
    if ((m = /^fruity: exit code (-?\d+)/.exec(l))) { out.exitCode = Number(m[1]); continue; }
    if ((m = SURFACE_LINE.exec(l))) { out.surfaces.push({ event: m[1] as string, lid: Number(m[2]), width: Number(m[3]), height: Number(m[4]), stride: Number(m[5]), nonzero: Number(m[6]), rgbaSha256: m[7] as string }); continue; }
    if (WINDOW_HEADER.test(l)) { windows = []; out.windows = windows; continue; }
    if ((m = WINDOW_LINE.exec(l))) {
      if (!windows) { windows = []; out.windows = windows; }
      windows.push({
        handle: m[1] as string, lid: Number(m[2]), className: m[3] as string, title: m[4] as string, x: Number(m[5]), y: Number(m[6]), width: Number(m[7]), height: Number(m[8]),
        relation: m[9] ? `${m[9]} ${m[10]}` : '', visible: m[11] === '1', enabled: m[12] === '1', z: Number(m[13]),
      });
      continue;
    }
    if (THREADS_HEADER.test(l)) { out.threadSummary = l.trim(); threads = []; out.threads = threads; continue; }
    if ((m = THREAD_LINE.exec(l)) && threads) threads.push({ index: Number(m[1]), id: m[2] as string, state: m[3] as string });
  }
  return out;
}

/** the notes of the manifest whose `contains` text is part of the engine's stop message */
export function matchStopNotes(runtimeMessage: string, notes: readonly { contains: string; note: string }[] | undefined): string[] {
  if (!notes || runtimeMessage === '') return [];
  return notes.filter((n) => runtimeMessage.includes(n.contains)).map((n) => n.note);
}
