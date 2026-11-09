// The engine's Open / Save dialogs answered by the file manager window (decision 0216 stage 2, opfs1). The engine is suspended in IFileOpenDialog::Show / IFileSaveDialog::Show (decision 0193); the page
// shows the window in selection mode (files/explorer.ts), the person chooses a file of the browser's storage (OPFS) instead of one of the computer, and the answer is the same FileDlgAnswer as before:
//   Open  -> the chosen File objects (read from the OPFS) with the guest paths they get on the engine's virtual volume; a project (.flp) brings the files next to it
//   Save  -> the guest path of the target; when the engine has finished writing it, the bytes are copied back to the chosen OPFS path (the 'saved-file' message of decision 0193)
// A transient user activation is not needed anywhere here: nothing opens a browser chooser.
import { OpfsFs, describeError, memoryNotice } from './opfs.ts';
import { baseName, joinPath, normalizePath, parentOf, uniqueName } from './paths.ts';
import { Explorer } from './explorer.ts';
import { failCode, filesUsage } from './usage.ts';
import { configOf, guestPathOfOpfs, isZipName, pickProject, planFolder, planOpen, type SiblingFile } from './select.ts';
import { extractZip, planZip } from './ops.ts';
import type { FileDlgAnswer, FileDlgItem, FileDlgRequest } from '../engine/filedlg.ts';

export interface EngineDialogRecord {
  id: number;
  mode: string;
  /** what was answered: [role, guest path, bytes] */
  items: [string, string, number][];
  note: string;
  unpackedZip: string | null;
  at: number;
}

export class EngineFiles {
  private fsP: Promise<OpfsFs> | null = null;
  private lastDir = '/';
  private notedMemory = false;
  /** the guest path (lower case) of every Save target the person chose -> the OPFS path to copy the finished file to */
  readonly pendingSaves = new Map<string, string>();
  /** for the tests: what the dialogs were answered with, and the files copied back to the OPFS */
  readonly answers: EngineDialogRecord[] = [];
  readonly savedBack: { guest: string; opfs: string; size: number; at: number; error?: string }[] = [];
  explorer: Explorer | null = null;
  private readonly log: (kind: 'info' | 'warn', text: string) => void;

  constructor(log: (kind: 'info' | 'warn', text: string) => void) { this.log = log; }

  private fs(): Promise<OpfsFs> {
    if (!this.fsP) { this.fsP = OpfsFs.open(); this.fsP.catch(() => { this.fsP = null; }); }
    return this.fsP;
  }

  /** the hook FileDialogView calls instead of building its box: shows the selection window in `root` and calls `done` once with the engine's answer; returns what undoes it */
  select = (id: number, request: FileDlgRequest, root: HTMLElement, done: (a: FileDlgAnswer) => void): (() => void) => {
    let disposed = false;
    let fs: OpfsFs;
    const cancel = (why: string): void => { if (!disposed) { filesUsage.c.engine.cancelled++; done({ ok: false, reason: why }); } };
    void (async () => {
      try { fs = await this.fs(); } catch (e) { cancel(`the browser's file storage is not available (${describeError(e)})`); return; }
      if (!fs.persistent && !this.notedMemory) { this.notedMemory = true; this.log('warn', memoryNotice(fs.why)); }
      if (disposed) return;
      const config = configOf(request, this.lastDir);
      filesUsage.c.engine[config.mode]++;
      const ex = new Explorer(root, fs, { close: () => cancel('cancelled by the person') }, {
        config,
        done: (r) => {
          if (r === null) { cancel('cancelled by the person'); return; }
          void this.answer(id, ex, fs, config.mode, r.paths, r.filterIndex + 1).then((a) => { if (!disposed) { if (a) { filesUsage.c.engine.answered++; done(a); } else { filesUsage.fail(ex.log[ex.log.length - 1] ?? ''); ex.reopen(ex.log[ex.log.length - 1] ?? 'The engine could not be given that.'); } } });
        },
      });
      let start = config.startDir;
      if (!(await fs.stat(start))) start = '/';
      await ex.navigate(start);
      if (!disposed) this.explorer = ex;   // set when the window has its first listing: the tests (and the glue) take it as ready
      root.querySelector<HTMLElement>('#fx-filename')?.focus({ preventScroll: true });
    })();
    return () => { disposed = true; if (this.explorer && this.explorer.root === root) this.explorer = null; };
  };

  /** the paths the person chose -> the answer, or null when it cannot be given (the window then says why and stays open) */
  private async answer(id: number, ex: Explorer, fs: OpfsFs, mode: string, paths: string[], typeIndex: number): Promise<FileDlgAnswer | null> {
    try {
      if (mode === 'save') {
        const target = paths[0]!;
        const guest = guestPathOfOpfs(target);
        if (guest === null) { ex.say(`${target}: the name cannot be given to the engine's volume.`, true); return null; }
        this.pendingSaves.set(guest.toLowerCase(), target);
        this.lastDir = parentOf(target);
        this.answers.push({ id, mode, items: [['S', guest, 0]], note: '', unpackedZip: null, at: Date.now() });
        return { ok: true, typeIndex, items: [{ role: 'S', path: guest }] };
      }
      if (mode === 'folder') {
        const dir = paths[0]!;
        const all: SiblingFile[] = [];
        const walk = async (p: string): Promise<void> => { for (const e of await fs.list(p, true)) { if (e.kind === 'directory') await walk(e.path); else all.push({ name: e.name, path: e.path, size: e.size }); } };
        await walk(dir);
        const pf = planFolder(dir, all);
        if (!pf.ok) { ex.say(pf.reason, true); return null; }
        const items: FileDlgItem[] = [{ role: 'D', path: pf.value.folder }];
        for (const it of pf.value.items) items.push({ role: 'E', path: it.guest, file: await fs.read(it.opfs) });
        if (pf.value.note) this.log('warn', pf.value.note);
        this.lastDir = parentOf(dir);
        this.answers.push({ id, mode, items: [['D', pf.value.folder, 0], ...pf.value.items.map((i): [string, string, number] => ['E', i.guest, i.size])], note: pf.value.note, unpackedZip: null, at: Date.now() });
        return { ok: true, typeIndex, items };
      }
      // Open: a zip is unpacked here first and its project is what the engine opens (the engine's own unpacking, DelZip64, is another route: decision 0206)
      let chosen = paths;
      let unpacked: string | null = null;
      if (paths.length === 1 && isZipName(baseName(paths[0]!))) {
        const zipPath = paths[0]!;
        const blob = await fs.read(zipPath);
        const dir = parentOf(zipPath);
        const stem = baseName(zipPath).replace(/\.zip$/i, '');
        const dest = joinPath(dir, uniqueName(stem, await fs.names(dir)));
        const plan = await planZip(blob);
        const res = await ex.withProgress((ctx) => extractZip(ctx, dest, blob, plan), 'unpack zip');
        if (!res || res.cancelled) { ex.say('Unpacking was cancelled.', true); return null; }
        if (res.failed.length > 0) this.log('warn', `the zip had ${res.failed.length} entries that could not be unpacked (${res.failed[0]!.reason})`);
        const all: string[] = [];
        const walk = async (p: string): Promise<void> => { for (const e of await fs.list(p, false)) { if (e.kind === 'directory') await walk(e.path); else all.push(e.path); } };
        await walk(dest);
        const flp = pickProject(all);
        if (flp === null) { ex.say(`${baseName(zipPath)} holds no .flp project (unpacked into ${dest}).`, true); return null; }
        chosen = [flp];
        unpacked = dest;
        filesUsage.c.engine.zipUnpacked++;
      }
      const stats = await Promise.all(chosen.map((p) => fs.stat(p)));
      if (stats.some((s) => s === null || s.kind !== 'file')) { ex.say('A chosen file is gone.', true); return null; }
      const sibCache = new Map<string, SiblingFile[]>();
      for (const p of chosen) {
        const dir = parentOf(p);
        if (!sibCache.has(dir)) sibCache.set(dir, (await fs.list(dir, true)).filter((e) => e.kind === 'file').map((e) => ({ name: e.name, path: e.path, size: e.size })));
      }
      const plan = planOpen(chosen.map((p, i) => ({ name: baseName(p), path: normalizePath(p), size: stats[i]!.size })), (dir) => sibCache.get(dir) ?? []);
      if (!plan.ok) { ex.say(plan.reason, true); return null; }
      const items: FileDlgItem[] = [];
      for (const it of plan.value.items) items.push({ role: it.role, path: it.guest, file: await fs.read(it.opfs) });
      if (plan.value.note) this.log('warn', plan.value.note);
      if (chosen.length === 1) this.lastDir = parentOf(chosen[0]!);
      this.answers.push({ id, mode, items: plan.value.items.map((i) => [i.role, i.guest, i.size]), note: plan.value.note, unpackedZip: unpacked, at: Date.now() });
      return { ok: true, typeIndex, items };
    } catch (e) {
      ex.say(`The engine could not be given that: ${describeError(e)}`, true);
      return null;
    }
  }

  /** the engine finished writing a Save target: when the person chose it in the window, copy it to that OPFS path; returns the path or null */
  async onSavedFile(f: { guestPath: string; size: number; data: ArrayBuffer }): Promise<string | null> {
    const target = this.pendingSaves.get(f.guestPath.toLowerCase());
    if (target === undefined) return null;
    const rec: { guest: string; opfs: string; size: number; at: number; error?: string } = { guest: f.guestPath, opfs: target, size: f.size, at: Date.now() };
    try {
      const fs = await this.fs();
      await fs.write(target, new Uint8Array(f.data));
      this.log('info', `the file the engine saved (${f.size} bytes) was copied to ${fs.persistent ? "the browser's storage" : "this tab's memory"} (the name is not written here)`);
      this.savedBack.push(rec);
      filesUsage.c.engine.savedBack++;
      return target;
    } catch (e) {
      rec.error = describeError(e);
      this.savedBack.push(rec);
      filesUsage.c.engine.savedBackFailed++;
      filesUsage.fail(e);
      this.log('warn', `the file the engine saved could not be copied to the browser's storage: ${failCode(e)}. It is still under "Files the engine wrote" for download.`);
      return null;
    }
  }
}
