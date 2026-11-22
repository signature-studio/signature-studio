// The items of an NSIS installer through 7-Zip (7z), which reads the NSIS script and its Deflate or LZMA streams. The installer is read, never run.
import { execFileSync } from 'node:child_process';

const MAX_BUFFER = 1 << 30;

/** whether 7-Zip reads the file `file` as an NSIS installer, and the paths of its items (7z l -slt) */
export function sevenZipList(file) {
  const out = execFileSync('7z', ['l', '-slt', '-bd', '--', file], { encoding: 'utf8', maxBuffer: MAX_BUFFER });
  return {
    nsis: out.split('\n').includes('Type = Nsis'),
    paths: out.split('\n').filter((ln) => ln.startsWith('Path = ')).map((ln) => ln.slice('Path = '.length)),
  };
}

/** the bytes of the item `item` of the archive `file` (7z x -so: written to standard output) */
export function sevenZipRead(file, item) {
  return execFileSync('7z', ['x', '-so', '-bd', '--', file, item], { maxBuffer: MAX_BUFFER, stdio: ['ignore', 'pipe', 'pipe'] });
}
