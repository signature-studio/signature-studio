// An in-memory directory tree that answers the part of the File System Access API which files/opfs.ts uses (decision 0248, ci1). It is the root of the file manager when the browser gives the page no
// origin private file system (Firefox private windows reject navigator.storage.getDirectory() with a SecurityError; storage can also be blocked or broken): the Files window, the engine's Open / Save
// dialogs and the save-back then work exactly as with the OPFS, because everything above the handle (OpfsFs, Explorer, ops, engine-dialog) is the same code. The files live in the tab and are gone when it is closed.
// Behaviour follows the OPFS where the page can notice it: DOMException names for the failures (NotFoundError, TypeMismatchError, InvalidModificationError, NoModificationAllowedError), a file is replaced only when
// its writable is closed (abort leaves it as it was), one writable at a time per file (Firefox's rule), a removed file's handle is dead.

const domError = (name: string, message: string): DOMException => new DOMException(message, name);

const badName = (n: string): boolean => n === '' || n === '.' || n === '..' || n.includes('/') || n.includes('\\') || n.includes('\0');

class FileNode {
  blob = new Blob([]);
  modified = Date.now();
  writers = 0;
  removed = false;
}
class DirNode {
  readonly children = new Map<string, FileNode | DirNode>();
  removed = false;
}

export type MemoryWritable = {
  write(data: string | Uint8Array | ArrayBuffer | Blob): Promise<void>;
  close(): Promise<void>;
  abort(): Promise<void>;
};

export class MemoryFileHandle {
  readonly kind = 'file' as const;
  readonly name: string;
  private readonly node: FileNode;
  constructor(name: string, node: FileNode) { this.name = name; this.node = node; }

  async getFile(): Promise<File> {
    if (this.node.removed) throw domError('NotFoundError', `${this.name} is gone.`);
    return new File([this.node.blob], this.name, { lastModified: this.node.modified });
  }

  async createWritable(options?: { keepExistingData?: boolean }): Promise<MemoryWritable> {
    const node = this.node;
    if (node.removed) throw domError('NotFoundError', `${this.name} is gone.`);
    if (node.writers > 0) throw domError('NoModificationAllowedError', `${this.name} is open for writing.`);
    node.writers++;
    const parts: BlobPart[] = options?.keepExistingData ? [node.blob] : [];
    let open = true;
    return {
      write: async (data) => {
        if (!open) throw new TypeError('The writable is closed.');
        if (typeof data === 'string' || data instanceof Blob) parts.push(data);
        else if (data instanceof ArrayBuffer) parts.push(data.slice(0));
        else if (ArrayBuffer.isView(data)) parts.push(new Uint8Array(data.buffer, data.byteOffset, data.byteLength).slice());
        else throw new TypeError('Cannot write that.');
      },
      close: async () => {
        if (!open) throw new TypeError('The writable is closed.');
        open = false;
        node.writers--;
        node.blob = new Blob(parts);
        node.modified = Date.now();
      },
      abort: async () => { if (open) { open = false; node.writers--; } },
    };
  }
}

export class MemoryDirectoryHandle {
  readonly kind = 'directory' as const;
  readonly name: string;
  private readonly node: DirNode;
  constructor(name = '', node: DirNode = new DirNode()) { this.name = name; this.node = node; }

  private check(name: string): void {
    if (this.node.removed) throw domError('NotFoundError', 'The folder is gone.');
    if (badName(name)) throw new TypeError(`"${name}" is not a valid name.`);
  }

  async getDirectoryHandle(name: string, options?: { create?: boolean }): Promise<MemoryDirectoryHandle> {
    this.check(name);
    const c = this.node.children.get(name);
    if (c instanceof DirNode) return new MemoryDirectoryHandle(name, c);
    if (c) throw domError('TypeMismatchError', `${name} is a file.`);
    if (!options?.create) throw domError('NotFoundError', `${name} does not exist.`);
    const d = new DirNode();
    this.node.children.set(name, d);
    return new MemoryDirectoryHandle(name, d);
  }

  async getFileHandle(name: string, options?: { create?: boolean }): Promise<MemoryFileHandle> {
    this.check(name);
    const c = this.node.children.get(name);
    if (c instanceof FileNode) return new MemoryFileHandle(name, c);
    if (c) throw domError('TypeMismatchError', `${name} is a folder.`);
    if (!options?.create) throw domError('NotFoundError', `${name} does not exist.`);
    const f = new FileNode();
    this.node.children.set(name, f);
    return new MemoryFileHandle(name, f);
  }

  async removeEntry(name: string, options?: { recursive?: boolean }): Promise<void> {
    this.check(name);
    const c = this.node.children.get(name);
    if (!c) throw domError('NotFoundError', `${name} does not exist.`);
    if (c instanceof DirNode) {
      if (c.children.size > 0 && !options?.recursive) throw domError('InvalidModificationError', `${name} is not empty.`);
      if (busy(c)) throw domError('NoModificationAllowedError', `${name} holds a file that is open for writing.`);
      kill(c);
    } else {
      if (c.writers > 0) throw domError('NoModificationAllowedError', `${name} is open for writing.`);
      c.removed = true;
    }
    this.node.children.delete(name);
  }

  async *entries(): AsyncGenerator<[string, MemoryDirectoryHandle | MemoryFileHandle]> {
    if (this.node.removed) throw domError('NotFoundError', 'The folder is gone.');
    for (const [n, c] of [...this.node.children]) yield [n, c instanceof DirNode ? new MemoryDirectoryHandle(n, c) : new MemoryFileHandle(n, c)];
  }
}

function busy(d: DirNode): boolean {
  for (const c of d.children.values()) if (c instanceof DirNode ? busy(c) : c.writers > 0) return true;
  return false;
}
function kill(d: DirNode): void {
  d.removed = true;
  for (const c of d.children.values()) { if (c instanceof DirNode) kill(c); else c.removed = true; }
}
