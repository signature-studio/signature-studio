// The page's side of the engine's file dialogs (decision 0193, fpick1, PROVISIONAL). The engine's File > Open / Export call the Windows common item dialog (IFileOpenDialog / IFileSaveDialog); the runtime model
// (lift/runtime/fle_filedlg.c) suspends the engine in Show() and asks the page: the PERSON at the page chooses a file of their computer (or names a file to save), the Worker reads the
// chosen File into the engine's memory and puts it on the engine's virtual volume at a guest path decided here. Nothing is uploaded: the bytes go from the File to the Worker's memory and nowhere else.
// This module holds no DOM: the request / answer shapes, the guest-path rules (the same rules as the volume's, lift/runtime/fle_vfs.c put_name_ok) and the plans for Open and Save. src/ui/filedlg.ts is the view.

/** what the engine asked (lift/runtime/fle_filedlg.c browser_picker writes it as JSON) */
export interface FileDlgRequest {
  save: boolean;
  /** FILEOPENDIALOGOPTIONS */
  options: number;
  /** 1-based index of the selected file type */
  filterIndex: number;
  title: string | null;
  okLabel: string | null;
  fileName: string | null;
  defaultExt: string | null;
  /** the guest folder the engine set (C:\...) or null */
  folder: string | null;
  /** [display name, spec like "*.flp;*.zip"] */
  filters: [string | null, string | null][];
}

/** R: the file the dialog returns; E: another file put on the volume next to it (a sample of the project); S: the target of a Save dialog (the engine writes it); D: the folder a folder dialog returns (decision 0216) */
export type FileDlgRole = 'R' | 'E' | 'S' | 'D';
export interface FileDlgItem { role: FileDlgRole; path: string; file?: File }
/** typeIndex (decision 0216): the file type the person picked in the page's window, 1-based; an older bundle ignores it */
export type FileDlgAnswer = { ok: false; reason?: string } | { ok: true; items: FileDlgItem[]; typeIndex?: number };

/** a file the person chose, as the page knows it (File.name, File.size, webkitRelativePath for a folder) */
export interface PickedFile { name: string; size: number; relativePath?: string }

export const FOS_PICKFOLDERS = 0x20;
export const FOS_ALLOWMULTISELECT = 0x200;

/** the volume refuses a file over 256 MiB (fle_vfs.c MAX_FILE_BYTES) */
export const MAX_FILE_BYTES = 256 * 1024 * 1024;
/** S-fpick1-total: the engine's WebAssembly memory is at most 2 GiB and the engine itself uses about 1 GiB; what the person puts on the volume is limited to 384 MiB in all */
export const MAX_TOTAL_BYTES = 384 * 1024 * 1024;
export const MAX_FILES = 512;
export const MAX_PATH_UNITS = 259;
export const MAX_COMPONENT = 255;
/** S-fpick1-open-folder: where a chosen file is put (a file picker tells the page the name and not the folder). The engine finds the samples of a project next to the FLP (decision 0178 section 8) */
export const OPEN_FOLDER = 'C:\\Users\\user\\Downloads';
/** the folder the virtual profile has since decision 0188 (S-exp1-vfs-documents) */
export const SAVE_FOLDER = 'C:\\Users\\user\\Documents';

const DEVICE = /^(con|prn|aux|nul|com[1-9\u00b9\u00b2\u00b3]|lpt[1-9\u00b9\u00b2\u00b3])$/i;
// eslint-disable-next-line no-control-regex
const BAD_CHARS = /[\u0000-\u001f<>:"|*?/\\]/g;

const wellFormed = (s: string): boolean => (typeof (s as unknown as { isWellFormed?: () => boolean }).isWellFormed === 'function' ? (s as unknown as { isWellFormed: () => boolean }).isWellFormed() : !/[\ud800-\udfff]/.test(s.replace(/[\ud800-\udbff][\udc00-\udfff]/g, '')));

/** the volume's rule for one path component (put_name_ok): non-empty, at most 255 UTF-16 units, none of < > : " | * ? / \ or a control character, no trailing space or dot, not . or .., not a device name */
export function isValidComponent(c: string): boolean {
  if (c.length === 0 || c.length > MAX_COMPONENT || !wellFormed(c)) return false;
  if (/[\u0000-\u001f<>:"|*?/\\]/.test(c)) return false;
  if (/[ .]$/.test(c)) return false;
  const stem = c.split('.')[0]?.replace(/ +$/, '') ?? '';
  return !DEVICE.test(stem);
}

/** turns what a file system calls a name into a component the volume takes; the extension survives */
export function sanitizeComponent(name: string): string {
  let s = wellFormed(name) ? name : name.replace(/[\ud800-\udbff](?![\udc00-\udfff])|(?<![\ud800-\udbff])[\udc00-\udfff]/g, '_');
  s = s.replace(BAD_CHARS, '_').replace(/[ .]+$/, '');
  if (s === '' || s === '.' || s === '..') s = '_';
  if (DEVICE.test(s.split('.')[0]?.replace(/ +$/, '') ?? '')) s = `_${s}`;
  if (s.length > MAX_COMPONENT) {
    const dot = s.lastIndexOf('.');
    const ext = dot > 0 && s.length - dot <= 16 ? s.slice(dot) : '';
    let head = s.slice(0, MAX_COMPONENT - ext.length);
    if (/[\ud800-\udbff]$/.test(head)) head = head.slice(0, -1);
    s = (head + ext).replace(/[ .]+$/, '') || '_';
  }
  return s;
}

/** the volume's rule for a whole guest path (put_walk): C:\ then valid components, at most 259 UTF-16 units, no trailing separator */
export function isValidGuestPath(p: string): boolean {
  if (p.length < 4 || p.length > MAX_PATH_UNITS || !/^[Cc]:\\/.test(p)) return false;
  return p.slice(3).split('\\').every(isValidComponent);
}

/** folder + relative path ('a/b/c.wav' or 'c.wav') -> a guest path with every component sanitised; null when the result is too long */
export function guestPathFor(folder: string, relative: string): string | null {
  const parts = relative.split(/[\\/]/).filter((x) => x !== '' && x !== '.').map(sanitizeComponent);
  if (parts.length === 0) return null;
  const p = `${folder.replace(/\\+$/, '')}\\${parts.join('\\')}`;
  return isValidGuestPath(p) ? p : null;
}

/** "*.flp;*.zip" -> ['.flp', '.zip'] ('*.*' and '*' -> []) */
export function extensionsOfSpec(spec: string | null): string[] {
  if (!spec) return [];
  const out: string[] = [];
  for (const part of spec.split(';')) {
    const m = /^\s*\*?(\.[A-Za-z0-9_-]+)\s*$/.exec(part);
    if (m) out.push((m[1] as string).toLowerCase());
  }
  return out;
}

/** the value of the file input's `accept` attribute for the file type the engine selected ('' = no restriction) */
export function acceptFor(req: FileDlgRequest): string {
  const f = req.filters[req.filterIndex - 1];
  return extensionsOfSpec(f?.[1] ?? null).join(',');
}

const extOf = (name: string): string => { const i = name.lastIndexOf('.'); return i > 0 ? name.slice(i).toLowerCase() : ''; };

export type Plan<T> = { ok: true; value: T } | { ok: false; reason: string };

/** Open, one file: put at OPEN_FOLDER\<name>. */
export function planOpenFile(req: FileDlgRequest, file: PickedFile): Plan<{ items: { role: FileDlgRole; path: string }[] }> {
  if (req.save) return { ok: false, reason: 'this is a Save dialog' };
  if (req.options & FOS_PICKFOLDERS) return { ok: false, reason: 'the engine asked for a folder of its virtual volume: there is none to choose from on this computer' };
  if (file.size > MAX_FILE_BYTES) return { ok: false, reason: `${file.name} is ${(file.size / 1048576).toFixed(0)} MiB; the engine's virtual volume takes files up to ${MAX_FILE_BYTES / 1048576} MiB` };
  const path = guestPathFor(OPEN_FOLDER, file.name);
  if (path === null) return { ok: false, reason: `${file.name}: the name cannot be given to the engine's volume` };
  return { ok: true, value: { items: [{ role: 'R', path }] } };
}

/**
 * Open, a project folder (showDirectoryPicker / <input webkitdirectory>): every file is put at OPEN_FOLDER\<folder>\<relative path>; the dialog's result is the project file (the first file whose extension is in the
 * engine's selected type, else in any type the engine offers; ties by name), the others are extra files (the samples of the project, found next to it).
 */
export function planOpenFolder(req: FileDlgRequest, files: PickedFile[]): Plan<{ items: { role: FileDlgRole; path: string; index: number }[]; resultIndex: number }> {
  if (req.save) return { ok: false, reason: 'this is a Save dialog' };
  if (files.length === 0) return { ok: false, reason: 'the folder holds no files' };
  if (files.length > MAX_FILES) return { ok: false, reason: `the folder holds ${files.length} files; at most ${MAX_FILES} are taken` };
  let total = 0;
  for (const f of files) {
    if (f.size > MAX_FILE_BYTES) return { ok: false, reason: `${f.relativePath ?? f.name} is ${(f.size / 1048576).toFixed(0)} MiB; the engine's virtual volume takes files up to ${MAX_FILE_BYTES / 1048576} MiB` };
    total += f.size;
  }
  if (total > MAX_TOTAL_BYTES) return { ok: false, reason: `the folder holds ${(total / 1048576).toFixed(0)} MiB; at most ${MAX_TOTAL_BYTES / 1048576} MiB are taken` };
  const selected = extensionsOfSpec(req.filters[req.filterIndex - 1]?.[1] ?? null);
  const any = req.filters.flatMap((f) => extensionsOfSpec(f[1]));
  const keyOf = (f: PickedFile): string => f.relativePath ?? f.name;
  const depthOf = (f: PickedFile): number => keyOf(f).split('/').length;
  const pick = (exts: string[]): number => {
    if (exts.length === 0) return -1;
    let best = -1;
    files.forEach((f, i) => {
      if (!exts.includes(extOf(f.name))) return;
      const b = files[best];
      if (b === undefined || depthOf(f) < depthOf(b) || (depthOf(f) === depthOf(b) && keyOf(f) < keyOf(b))) best = i;
    });
    return best;
  };
  // the FLP itself first (a folder of a project holds the FLP and its samples; an .mid among the samples must not win over it)
  let resultIndex = pick(['.flp']);
  if (resultIndex < 0) resultIndex = pick(selected);
  if (resultIndex < 0) resultIndex = pick(any);
  if (resultIndex < 0) return { ok: false, reason: 'the folder holds no file of a type the engine opens here' };
  const items: { role: FileDlgRole; path: string; index: number }[] = [];
  for (let i = 0; i < files.length; i++) {
    const f = files[i] as PickedFile;
    const path = guestPathFor(OPEN_FOLDER, f.relativePath ?? f.name);
    if (path === null) {
      if (i === resultIndex) return { ok: false, reason: `${f.relativePath ?? f.name}: the name cannot be given to the engine's volume` };
      continue;   // a sample whose path does not fit is left out; the engine then reports the sample as missing, as it would for a deleted one
    }
    items.push({ role: i === resultIndex ? 'R' : 'E', path, index: i });
  }
  return { ok: true, value: { items, resultIndex } };
}

/** Save: the name the person typed (or the engine's suggestion) -> the guest path of the target and the name the download gets */
export function planSave(req: FileDlgRequest, typed: string): Plan<{ path: string; name: string }> {
  if (!req.save) return { ok: false, reason: 'this is an Open dialog' };
  let name = sanitizeComponent(typed.trim() === '' ? 'Untitled' : typed.trim());
  if (!name.includes('.') && req.defaultExt) {
    const e = req.defaultExt.replace(/^\./, '');
    if (e !== '') name = sanitizeComponent(`${name}.${e}`);
  }
  const folder = req.folder !== null && /^[Cc]:\\/.test(req.folder) && isValidGuestPath(req.folder.replace(/\\+$/, '') + '\\x') ? req.folder.replace(/\\+$/, '') : SAVE_FOLDER;
  const path = guestPathFor(folder, name);
  if (path === null) return { ok: false, reason: `${name}: the name is too long for the engine's volume` };
  return { ok: true, value: { path, name } };
}

/** the suggestion shown in the Save box: the engine's file name (without a folder), else 'Untitled' */
export function suggestedSaveName(req: FileDlgRequest): string {
  const raw = (req.fileName ?? '').split(/[\\/]/).pop() ?? '';
  const base = raw.trim() === '' ? 'Untitled' : raw.trim();
  return !base.includes('.') && req.defaultExt ? `${base}.${req.defaultExt.replace(/^\./, '')}` : base;
}

/** one line for the page: what the engine offers */
export function describeFilters(req: FileDlgRequest): string {
  return req.filters.map(([n, s], i) => `${i + 1 === req.filterIndex ? '\u25B6 ' : ''}${n ?? ''}${s ? ` (${s})` : ''}`).join(' | ');
}

/** a request from the Worker is data of the engine's strings: check its shape before the page uses it */
export function parseRequest(x: unknown): FileDlgRequest | null {
  if (typeof x !== 'object' || x === null) return null;
  const r = x as Record<string, unknown>;
  const str = (v: unknown): string | null => (typeof v === 'string' ? v : null);
  if (typeof r.save !== 'boolean' && typeof r.save !== 'number') return null;
  if (typeof r.options !== 'number' || typeof r.filterIndex !== 'number' || !Array.isArray(r.filters)) return null;
  const filters: [string | null, string | null][] = [];
  for (const f of r.filters.slice(0, 64)) {
    if (!Array.isArray(f)) return null;
    filters.push([str(f[0]), str(f[1])]);
  }
  return { save: !!r.save, options: r.options, filterIndex: r.filterIndex, title: str(r.title), okLabel: str(r.okLabel), fileName: str(r.fileName), defaultExt: str(r.defaultExt), folder: str(r.folder), filters };
}
