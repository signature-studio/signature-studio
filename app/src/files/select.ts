// The file manager as the engine's file dialog (decision 0216 stage 2, opfs1): what the engine asked (IFileOpenDialog / IFileSaveDialog, src/engine/filedlg.ts FileDlgRequest) turned into the window's
// selection mode, and what the person chose turned into the answer (FileDlgAnswer) that the existing bridge of decision 0193 already takes. No DOM and no OPFS here: the functions are pure.
import { extensionsOfSpec, FOS_ALLOWMULTISELECT, FOS_PICKFOLDERS, guestPathFor, MAX_FILES, MAX_FILE_BYTES, MAX_TOTAL_BYTES, OPEN_FOLDER, type FileDlgRequest } from '../engine/filedlg.ts';
import { baseName, extOf, normalizePath, splitPath } from './paths.ts';

export type SelectMode = 'open' | 'save' | 'folder';

export interface SelectFilter {
  /** what the engine called it ("FL Studio song file (*.flp)") */
  label: string;
  /** lower-case extensions with the dot ('.flp'); empty = every file */
  exts: string[];
}

export interface SelectConfig {
  mode: SelectMode;
  title: string;
  okLabel: string;
  filters: SelectFilter[];
  /** 0-based */
  filterIndex: number;
  /** Save: the name the engine suggested ('' = none) */
  fileName: string;
  /** Save: the extension added to a name without one ('.wav' form, '' = none) */
  defaultExt: string;
  multi: boolean;
  /** FOS_OVERWRITEPROMPT: ask before an existing file is replaced */
  overwritePrompt: boolean;
  /** one line about what the engine asked, for the person (the guest folder it named) */
  note: string;
  /** where to start: an OPFS folder */
  startDir: string;
}

export interface SelectResult {
  /** OPFS paths: Open = the files, Save = the target file, Folder = the folder */
  paths: string[];
  filterIndex: number;
}

export const FOS_OVERWRITEPROMPT = 0x2;
export const FOS_FILEMUSTEXIST = 0x1000;

export function filtersOf(req: FileDlgRequest): SelectFilter[] {
  const out = req.filters.map(([name, spec]) => ({ label: name ?? spec ?? 'Files', exts: extensionsOfSpec(spec) }));
  return out.length > 0 ? out : [{ label: 'All files (*.*)', exts: [] }];
}

/** A name passes a filter when the filter is open (no extensions) or the name's extension is one of its extensions. */
export function matchesFilter(name: string, f: SelectFilter | undefined): boolean {
  if (!f || f.exts.length === 0) return true;
  const e = extOf(name);
  return e !== '' && f.exts.includes('.' + e);
}

/** the extension a Save name without one gets: the engine's default extension, else the selected filter's first one */
export function saveExtension(defaultExt: string, f: SelectFilter | undefined): string {
  if (defaultExt) return defaultExt.startsWith('.') ? defaultExt : '.' + defaultExt;
  return f?.exts[0] ?? '';
}

// ---- the guest path of an OPFS path (PROVISIONAL, stage 2: see S-opfs1-guest-map; stage 3 decides the real assignment) ----
// Until the OPFS is the guest's file system, a chosen file is COPIED onto the engine's virtual volume at OPEN_FOLDER\<the OPFS path with backslashes>, and a Save target is written there by the engine and
// copied back to the OPFS path when the engine has finished (the existing 'saved-file' bridge of decision 0193).
export function guestPathOfOpfs(opfsPath: string): string | null {
  const p = normalizePath(opfsPath);
  return p === '/' ? OPEN_FOLDER : guestPathFor(OPEN_FOLDER, splitPath(p).join('/'));
}

/** the OPFS folder that a guest folder named by the engine stands for, or null when it is not one of the mapped ones */
export function opfsPathOfGuestFolder(guestFolder: string | null): string | null {
  if (!guestFolder) return null;
  const g = guestFolder.replace(/\\+$/, '');
  const root = OPEN_FOLDER.replace(/\\+$/, '');
  if (g.toLowerCase() === root.toLowerCase()) return '/';
  if (!g.toLowerCase().startsWith(root.toLowerCase() + '\\')) return null;
  return '/' + g.slice(root.length + 1).split('\\').filter(Boolean).join('/');
}

export function configOf(req: FileDlgRequest, startDir: string): SelectConfig {
  const filters = filtersOf(req);
  const mode: SelectMode = req.options & FOS_PICKFOLDERS ? 'folder' : req.save ? 'save' : 'open';
  const idx = Math.max(0, Math.min(filters.length - 1, (req.filterIndex || 1) - 1));
  const base = (req.fileName ?? '').split(/[\\/]/).pop()?.trim() ?? '';
  const ext = req.defaultExt ?? '';
  return {
    mode,
    title: req.title ?? (mode === 'folder' ? 'Choose a folder' : mode === 'save' ? 'Save as' : 'Open'),
    okLabel: req.okLabel ?? (mode === 'save' ? 'Save' : mode === 'folder' ? 'Choose this folder' : 'Open'),
    filters, filterIndex: idx,
    fileName: mode === 'save' ? (base === '' ? 'Untitled' + saveExtension(ext, filters[idx]) : !base.includes('.') ? base + saveExtension(ext, filters[idx]) : base) : '',
    defaultExt: ext,
    multi: mode === 'open' && !!(req.options & FOS_ALLOWMULTISELECT),
    overwritePrompt: mode === 'save' && !!(req.options & FOS_OVERWRITEPROMPT),
    note: req.folder ? `The engine started in ${req.folder}` : '',
    startDir: opfsPathOfGuestFolder(req.folder) ?? startDir,
  };
}

// ---- Open: which files go onto the volume with the chosen one ----
export interface SiblingFile { name: string; path: string; size: number }

export interface OpenPlan {
  /** the chosen files and, for a project file, its neighbours in the same folder (the engine looks for a project's samples next to it, decision 0178 section 8), as [opfs path, guest path, role] */
  items: { opfs: string; guest: string; role: 'R' | 'E'; size: number }[];
  /** the neighbours that were left out and why ('' = none) */
  note: string;
  totalBytes: number;
}

const PROJECT_EXT = new Set(['.flp']);

/**
 * chosen: the files the person picked; folderFiles(dir): the files directly in a folder (not the sub-folders: the engine finds a sample by its base name next to the project).
 * Neighbours are added only for a project file (.flp); for a MIDI file or an audio file the chosen file is alone. Limits are the volume's (filedlg.ts): 256 MiB a file, 384 MiB and 512 files in all;
 * when the neighbours do not fit, the project is still opened alone and the note says so (the engine then reports its samples as missing, as it would for deleted ones).
 */
export function planOpen(chosen: SiblingFile[], folderFiles: (dir: string) => SiblingFile[]): { ok: true; value: OpenPlan } | { ok: false; reason: string } {
  const items: OpenPlan['items'] = [];
  const seen = new Set<string>();
  let total = 0, note = '';
  const add = (f: SiblingFile, role: 'R' | 'E'): string | null => {
    const key = f.path.toLowerCase();
    if (seen.has(key)) return null;
    const g = guestPathOfOpfs(f.path);
    if (g === null) return `${f.path}: the name cannot be given to the engine's volume`;
    if (f.size > MAX_FILE_BYTES) return `${f.name} is ${(f.size / 1048576).toFixed(0)} MiB; the engine's virtual volume takes files up to ${MAX_FILE_BYTES / 1048576} MiB`;
    seen.add(key);
    items.push({ opfs: f.path, guest: g, role, size: f.size });
    total += f.size;
    return null;
  };
  for (const f of chosen) {
    const err = add(f, 'R');
    if (err) return { ok: false, reason: err };
  }
  if (total > MAX_TOTAL_BYTES) return { ok: false, reason: `the chosen files are ${(total / 1048576).toFixed(0)} MiB; at most ${MAX_TOTAL_BYTES / 1048576} MiB are taken` };
  for (const f of chosen) {
    if (!PROJECT_EXT.has('.' + extOf(f.name))) continue;
    const dir = f.path.slice(0, f.path.length - f.name.length - 1) || '/';
    const sibs = folderFiles(dir).filter((s) => !seen.has(s.path.toLowerCase()) && !PROJECT_EXT.has('.' + extOf(s.name)));
    const sumNew = sibs.reduce((n, s) => n + s.size, 0);
    if (items.length + sibs.length > MAX_FILES || total + sumNew > MAX_TOTAL_BYTES || sibs.some((s) => s.size > MAX_FILE_BYTES)) {
      note = `The folder next to ${f.name} holds ${sibs.length} other files (${(sumNew / 1048576).toFixed(0)} MiB), more than the engine's volume takes (${MAX_FILES} files, ${MAX_TOTAL_BYTES / 1048576} MiB in all): only the project file is given to the engine, so the samples next to it will be reported missing.`;
      continue;
    }
    for (const s of sibs) { const err = add(s, 'E'); if (err) note = (note ? note + ' ' : '') + `Left out: ${err}.`; }
  }
  return { ok: true, value: { items, note, totalBytes: total } };
}

/** the project file of an extracted zip: the shallowest .flp, ties by name; null when there is none */
export function pickProject(paths: string[]): string | null {
  let best: string | null = null;
  for (const p of paths) {
    if (!PROJECT_EXT.has('.' + extOf(baseName(p)))) continue;
    if (best === null) { best = p; continue; }
    const d = splitPath(p).length, bd = splitPath(best).length;
    if (d < bd || (d === bd && p < best)) best = p;
  }
  return best;
}

export const isZipName = (name: string): boolean => extOf(name) === 'zip';

/**
 * A folder dialog (FOS_PICKFOLDERS): the answer is the folder (role D) and the files in it and below it (role E) that fit the volume's limits (512 files, 384 MiB, 256 MiB a file), in the order given;
 * the rest is left out and the note says how many. The engine sees the folder and these files only (stage 3 maps the OPFS itself and needs no copy).
 */
export function planFolder(dir: string, files: SiblingFile[]): { ok: true; value: { folder: string; items: { opfs: string; guest: string; size: number }[]; note: string } } | { ok: false; reason: string } {
  const folder = guestPathOfOpfs(dir);
  if (folder === null) return { ok: false, reason: `${dir}: the name cannot be given to the engine's volume` };
  const items: { opfs: string; guest: string; size: number }[] = [];
  let total = 0, left = 0;
  for (const f of files) {
    const g = guestPathOfOpfs(f.path);
    if (g === null || f.size > MAX_FILE_BYTES || items.length >= MAX_FILES - 1 || total + f.size > MAX_TOTAL_BYTES) { left++; continue; }
    items.push({ opfs: f.path, guest: g, size: f.size });
    total += f.size;
  }
  return { ok: true, value: { folder, items, note: left > 0 ? `${left} of the ${files.length} files in the folder were left out (the engine's volume takes ${MAX_FILES} files, ${MAX_TOTAL_BYTES / 1048576} MiB in all, ${MAX_FILE_BYTES / 1048576} MiB a file).` : '' } };
}
