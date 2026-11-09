// The engine's last words, as the measurement beacon carries them (decision 0209): when the engine stops, the beacon's `engine.exit.message` says WHY in the engine's own words ("fle: <api>: <why>", a limit of the
// harness, an abort's VA name), so that the log of the server can tell which wall the people who use the page run into. The text comes from the lines the engine printed; it is cut to EXIT_MESSAGE_MAX characters and
// stripped of anything that could name a person or a file of theirs (home directories, user names in paths, file names, e-mail addresses, URLs, IP addresses) BEFORE it leaves the page. Pure: lines in, text out.
export const EXIT_MESSAGE_MAX = 400;

/** paths that belong to the virtual machine of the engine (lift/runtime/fle_env.h: C:\Windows, C:\Program Files\Image-Line\...) or to the host's own pseudo files: kept, because they say which module / file the engine asked for */
const KEEP_WINDOWS_ROOTS = /^[A-Za-z]:\\(?:Windows|Program Files(?: \(x86\))?|ProgramData)(?:\\|$)/i;
const KEEP_POSIX_ROOTS = /^\/(?:proc|dev|sys)(?:\/|$)/;
/** files a person owns (a project, a sample, an archive): the name is dropped wherever it stands */
const USER_FILE = /([^\s\\/:"'<>|*?]+)\.(flp|zip|rar|7z|wav|wave|mp3|ogg|flac|aif|aiff|mid|midi|fsc|fst|nfp|fpc|wv|m4a)\b/gi;

// A Windows path (drive letter or UNC) or an absolute POSIX path of at least two components. In a Windows path a directory name may contain spaces ("Program Files", "Jane Doe"), the last component may not, so that
// "C:\Users\Jane Doe\x.flp failed" ends after x.flp; a colon, a comma, a semicolon or a quote ends a path (none of them can be inside a Windows file name except the comma and the semicolon, which are rare).
const WIN_PATH = /(?:[A-Za-z]:\\|\\\\[^\s\\/:]+\\)(?:[^\\"'<>|*?;,:]+\\)*[^\s\\"'<>|*?;,:]*/g;
const POSIX_PATH = /(?<![\w.:\\<])\/(?:[^\s/"'<>|*?;,:]+\/)+[^\s/"'<>|*?;,:]*/g;
const EMAIL = /[A-Za-z0-9._%+-]+@[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)+/g;
const URL_RE = /\b[a-z][a-z0-9+.-]*:\/\/[^\s"'<>]+/gi;
const IPV4 = /\b(?:\d{1,3}\.){3}\d{1,3}\b/g;

const pathReplacement = (p: string, keep: boolean): string => {
  if (keep) return p;
  // the trailing sentence punctuation is not part of the path
  const m = /^(.*?)([.,;:)\]]*)$/.exec(p) as RegExpExecArray;
  const body = m[1] as string, tail = m[2] as string;
  const ext = /\.([A-Za-z0-9]{1,5})$/.exec(body)?.[1];
  return `<path>${ext ? `.${ext}` : ''}${tail}`;
};

/** the text with the personal parts removed (idempotent) and cut to `max` characters; control characters and line breaks become single spaces */
export function sanitizeEngineText(text: string, max = EXIT_MESSAGE_MAX): string {
  let s = text.replace(/[\u0000-\u001f\u007f-\u009f\u2028\u2029]+/g, ' ');
  s = s.replace(URL_RE, '<url>').replace(EMAIL, '<email>');
  s = s.replace(WIN_PATH, (p) => pathReplacement(p, KEEP_WINDOWS_ROOTS.test(p) && !/^[A-Za-z]:\\Users(?:\\|$)/i.test(p)));
  s = s.replace(POSIX_PATH, (p) => pathReplacement(p, KEEP_POSIX_ROOTS.test(p)));
  s = s.replace(USER_FILE, '<file>.$2').replace(IPV4, '<ip>');
  s = s.replace(/ {2,}/g, ' ').trim();
  return s.length > max ? `${s.slice(0, max - 1)}…` : s;
}

const stripStream = (l: string): string => l.replace(/^\[err\] ?/, '');
const stripFruity = (l: string): string => l.replace(/^fruity: /, '');

export interface ExitFacts { reason: string; code: number; message: string }

const IMPORT_LINE = /^fruity: the stop is in the lifted thunk f_[0-9a-f]+ \([^)]*\) = `jmp \[IAT slot\]` of import (\S+)/;
const STACK_LINE = /^fruity: guest stack at rsp \(return address[^)]*\):\s*(0x[0-9a-f]+)/;

/**
 * Which of the engine's lines says why it stopped: the last "runtime message: " (the harness' own copy of the last "fle: ..." line; with or without the "fruity: " prefix, decision 0197), else the last "fle: " / "limit: " line,
 * else the stop line ("== stop: ..."), else the first line of the trap's message (a wasm trap prints nothing). null: none of them (the engine finished, or the Worker was terminated). Never throws.
 * Two facts of the harness' stop report are added in brackets, so that the server's log can tell the walls apart without the engine log: "[import dll!Name]" (the import whose thunk the stop is in; a stop that says only
 * "unsupported instruction at 0x..." names no API itself, decision 0209) unless the message already names that API, and "[guest return 0x...]" (the first word at the guest stack pointer at the stop = the return address into the caller).
 */
export function engineExitMessage(lines: readonly string[], info: ExitFacts | null): string | null {
  const from = Math.max(0, lines.length - 2000);
  let runtime: string | null = null, wall: string | null = null, stop: string | null = null;
  let imp: string | null = null, ret: string | null = null;
  for (let i = lines.length - 1; i >= from && runtime === null; i--) {
    const l = stripFruity(stripStream(lines[i] as string));
    if (l.startsWith('runtime message: ')) runtime = l.slice('runtime message: '.length);
    else if (wall === null && (l.startsWith('fle: ') || l.startsWith('limit: '))) wall = l;
    else if (stop === null && l.startsWith('== stop:')) stop = l;
    else {
      const raw = stripStream(lines[i] as string);
      let m: RegExpExecArray | null;
      if (imp === null && (m = IMPORT_LINE.exec(raw))) imp = m[1] as string;
      else if (ret === null && (m = STACK_LINE.exec(raw))) ret = m[1] as string;
    }
  }
  const first = runtime ?? wall ?? stop ?? (info && info.reason === 'trap' ? (info.message.split('\n')[0] ?? '') : null);
  if (first === null) return null;
  const main = sanitizeEngineText(first, 10000);
  const api = imp ? (imp.split('!')[1] ?? imp) : null;
  const suffix = [
    imp && api && !main.toLowerCase().includes(api.toLowerCase()) ? ` [import ${imp}]` : '',
    ret ? ` [guest return ${ret}]` : '',
  ].join('');
  if (main === '') return null;
  // the suffix is what tells walls apart: when the whole is too long the message is cut, not the suffix
  const room = EXIT_MESSAGE_MAX - suffix.length;
  return `${main.length > room ? `${main.slice(0, Math.max(0, room - 1))}…` : main}${suffix}`;
}
