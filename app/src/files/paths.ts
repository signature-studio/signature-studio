// Path and name rules of the page's file manager over the browser's private file system (OPFS; decision 0216, opfs1).
// A path is '/a/b/c' (always absolute, '/' is the root; no '.' or '..' components). The names are the ones the guest (Windows) will see later, so the rules are Windows' (S-opfs1-names):
// no < > : " | ? * \ / or control characters, no trailing space or dot, no device names (CON, NUL, COM1 ...), at most 255 UTF-16 units; and two names that differ only by case collide (Windows is case-insensitive, OPFS is not).

import { isValidComponent, MAX_COMPONENT, sanitizeComponent } from '../engine/filedlg.ts';

export const MAX_NAME = MAX_COMPONENT;
const DEVICE = /^(con|prn|aux|nul|com[1-9]|lpt[1-9])(\..*)?$/i;

export function splitPath(p: string): string[] {
  return p.split('/').filter((s) => s !== '' && s !== '.');
}

/** normalise: always starts with '/', no empty or '.' parts; a '..' part is an error (it throws) */
export function normalizePath(p: string): string {
  const parts = splitPath(p);
  if (parts.includes('..')) throw new Error(`path with '..': ${p}`);
  return '/' + parts.join('/');
}

export function joinPath(dir: string, name: string): string {
  return dir === '/' ? '/' + name : dir + '/' + name;
}

export function parentOf(p: string): string {
  const parts = splitPath(p);
  parts.pop();
  return '/' + parts.join('/');
}

export function baseName(p: string): string {
  return splitPath(p).pop() ?? '';
}

export function isInside(path: string, ancestor: string): boolean {
  const a = splitPath(ancestor), p = splitPath(path);
  return p.length >= a.length && a.every((s, i) => s === p[i]);
}

/** null if the name is acceptable, else the reason (in words for the person) */
export function nameProblem(name: string): string | null {
  if (name === '') return 'The name is empty.';
  if (name === '.' || name === '..') return `"${name}" is not a name.`;
  if (name.length > MAX_NAME) return `The name is longer than ${MAX_NAME} characters.`;
  const m = name.match(/[<>:"|?*\\/\u0000-\u001f]/);
  if (m) return `A name cannot contain ${m[0] < ' ' ? 'a control character' : `"${m[0]}"`}. (The engine's Windows rules apply here.)`;
  if (/[ .]$/.test(name)) return 'A name cannot end with a space or a dot (Windows would drop it).';
  if (DEVICE.test(name)) return `"${name}" is a reserved device name on Windows.`;
  if (!isValidComponent(name)) return 'The engine\'s virtual volume would not accept this name.';
  return null;
}

/** make a name acceptable (for names that come from outside: a zip, a dropped folder) with the engine volume's own rule (src/engine/filedlg.ts, the same as lift/runtime/fle_vfs.c put_name_ok): returns [name, changed] */
export function sanitizeName(name: string): [string, boolean] {
  const n = sanitizeComponent(name);
  return [n, n !== name];
}

/** the relative path parts of something that came from outside (a zip entry, a dropped file): '..' / absolute / drive letters are refused (null), the rest is made acceptable */
export function safeRelative(raw: string): { parts: string[]; changed: boolean } | null {
  const slashed = raw.replace(/\\/g, '/');
  if (/^[A-Za-z]:/.test(slashed) || slashed.startsWith('//')) return null;
  const parts: string[] = [];
  let changed = slashed.startsWith('/');
  for (const s of slashed.split('/')) {
    if (s === '' || s === '.') { changed = changed || s === '.'; continue; }
    if (s === '..') return null;
    const [n, c] = sanitizeName(s);
    if (c) changed = true;
    parts.push(n);
  }
  if (parts.length === 0) return null;
  return { parts, changed: changed || slashed !== raw };
}

/** 'a.wav' -> 'a (2).wav' */
export function numbered(name: string, n: number): string {
  const dot = name.lastIndexOf('.');
  return dot > 0 ? `${name.slice(0, dot)} (${n})${name.slice(dot)}` : `${name} (${n})`;
}

/** the first of name, 'name (2)', 'name (3)' ... whose lower case is not in `taken` (a Set of lower-cased names) */
export function uniqueName(name: string, taken: ReadonlySet<string>): string {
  if (!taken.has(name.toLowerCase())) return name;
  for (let i = 2; ; i++) {
    const c = numbered(name, i);
    if (!taken.has(c.toLowerCase())) return c;
  }
}

export function extOf(name: string): string {
  const dot = name.lastIndexOf('.');
  return dot > 0 ? name.slice(dot + 1).toLowerCase() : '';
}

export function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  const u = ['KiB', 'MiB', 'GiB', 'TiB'];
  let v = n / 1024, i = 0;
  while (v >= 1024 && i < u.length - 1) { v /= 1024; i++; }
  return `${v >= 100 ? v.toFixed(0) : v >= 10 ? v.toFixed(1) : v.toFixed(2)} ${u[i]}`;
}

/** decision 0216 (R5 of act1's requirements): folders whose loss makes the engine forget its registration or licence; the file manager asks before it deletes, renames or moves them or a folder that holds them. The folders do not exist yet (act1/stage 3 create them). */
export const PROTECTED_PATHS: readonly string[] = ['/fl/state', '/fl/license'];

/** the first protected folder that `path` is, is inside, or contains; null when none */
export function protectedHit(path: string): string | null {
  const p = normalizePath(path);
  for (const q of PROTECTED_PATHS) if (isInside(p, q) || isInside(q, p)) return q;
  return null;
}
