// The script of an NSIS installer (its instructions, sections and strings) and the files it puts under $INSTDIR: the same reading as
// tools/nsis_script_dump.py (Script, string, entries_of_section) and lift/harness/seeds/gen_install_tree.py (walk), for the entries of FL Studio's installer.
import { entrySize, parseBlocks, readHeader } from './nsis.mjs';

const BUILTIN = [...Array(10).keys()].map((i) => `$${i}`).concat([...Array(10).keys()].map((i) => `$R${i}`), [
  '$CMDLINE', '$INSTDIR', '$OUTDIR', '$EXEDIR', '$LANGUAGE', '$TEMP', '$PLUGINSDIR', '$EXEPATH', '$EXEFILE', '$HWNDPARENT', '$_CLICK', '$_OUTDIR']);
const SHELL = new Map([
  ['36,36', '$WINDIR'], ['37,37', '$SYSDIR'], ['2,23', '$SMPROGRAMS'], ['7,24', '$SMSTARTUP'], ['16,25', '$DESKTOP'], ['11,22', '$STARTMENU'],
  ['26,26', '$QUICKLAUNCH'], ['5,46', '$DOCUMENTS'], ['20,20', '$FONTS'], ['21,45', '$TEMPLATES'], ['26,35', '$APPDATA'], ['28,35', '$LOCALAPPDATA'],
  ['40,40', '$PROFILE'], ['26,90', '$USERAPPDATA'], ['35,35', '$COMMONPROGRAMDATA']]);
const CSIDL = new Map([
  [0x02, 'PROGRAMS'], [0x17, 'COMMON_PROGRAMS'], [0x07, 'STARTUP'], [0x18, 'COMMON_STARTUP'], [0x10, 'DESKTOPDIRECTORY'], [0x19, 'COMMON_DESKTOPDIRECTORY'],
  [0x0b, 'STARTMENU'], [0x16, 'COMMON_STARTMENU'], [0x1a, 'APPDATA'], [0x23, 'COMMON_APPDATA'], [0x05, 'PERSONAL'], [0x2e, 'COMMON_DOCUMENTS'],
  [0x1c, 'LOCAL_APPDATA'], [0x15, 'TEMPLATES'], [0x14, 'FONTS'], [0x28, 'PROFILE'], [0x24, 'WINDOWS'], [0x25, 'SYSTEM'], [0x1d, 'ALTSTARTUP'],
  [0x0d, 'MYMUSIC'], [0x27, 'MYPICTURES'], [0x0e, 'MYVIDEO']]);
const hex2 = (v) => `0x${v.toString(16).padStart(2, '0')}`;
const OP_CREATEDIR = 11, OP_EXTRACTFILE = 20, OP_ASSIGNVAR = 27;
const DEFAULT_INSTDIR = 'C:\\Program Files\\Image-Line\\FL Studio 21';

export class Script {
  constructor(inst) {
    const { hdr, info } = readHeader(inst);
    this.hdr = hdr;
    this.info = info;
    const { blocks } = parseBlocks(hdr);
    this.blocks = blocks;
    this.so = blocks[3][0];
    this.send = blocks[4][0];
    const [eoff, num] = blocks[2];
    const es = entrySize(blocks);
    this.words = [];
    for (let i = 0; i < num; i++) {
      const row = [];
      for (let k = 0; k < es / 4; k++) row.push(hdr.readInt32LE(eoff + i * es + 4 * k));
      this.words.push(row);
    }
    this.cfIdx = this.findStr('CommonFilesDir');
    this.pfIdx = this.findStr('ProgramFilesDir');
  }

  findStr(s) {
    const i = this.hdr.subarray(this.so, this.send).indexOf(Buffer.concat([Buffer.from(s, 'utf16le'), Buffer.alloc(2)]));
    return i < 0 ? null : Math.floor(i / 2);
  }

  /** the string at index idx of the string table, with the codes of NSIS (language, shell folder, variable, literal) decoded */
  string(idx) {
    if (idx <= 0) return '';
    const h = this.hdr;
    let pos = this.so + idx * 2;
    let out = '';
    while (pos + 2 <= h.length) {
      const u = h.readUInt16LE(pos);
      pos += 2;
      if (u === 0) break;
      if (u === 1 || u === 2 || u === 3 || u === 4) {
        const w = h.readUInt16LE(pos);
        pos += 2;
        if (u === 3) {
          const n = (w & 0x7f) | (((w >> 8) & 0x7f) << 7);
          out += n < BUILTIN.length ? BUILTIN[n] : `$v${n}`;
        } else if (u === 2) {
          out += this.shell(w & 0xff, w >> 8);
        } else if (u === 1) {
          out += `$LANG(${-((w & 0x7f) | (((w >> 8) & 0x7f) << 7)) - 1})`;
        } else {
          out += String.fromCharCode(w);
        }
      } else {
        out += String.fromCharCode(u);
      }
    }
    return out;
  }

  shell(a, b) {
    if (a & 0x80) {
      const which = a & 0x3f;
      const name = which === this.pfIdx ? 'ProgramFilesDir' : which === this.cfIdx ? 'CommonFilesDir' : `?${which}`;
      const view = a & 0x40 ? '32' : '64';
      const pretty = (which === this.pfIdx ? '$PROGRAMFILES' : '$COMMONFILES') + view;
      return `${pretty}{HKLM\\Software\\Microsoft\\Windows\\CurrentVersion\\${name},view${view},fallback='${this.string(b)}'}`;
    }
    return SHELL.get(`${a},${b}`) ?? `$SH(csidl ${CSIDL.get(a) ?? hex2(a)}/${CSIDL.get(b) ?? hex2(b)})`;
  }

  /** the sections: { i, name, flags, code, codeSize } */
  sections() {
    const [off, num] = this.blocks[1];
    const sz = Math.floor((this.blocks[2][0] - off) / num);
    const out = [];
    for (let i = 0; i < num; i++) {
      const at = off + i * sz;
      const namePtr = this.hdr.readInt32LE(at);
      out.push({ i, name: namePtr ? this.string(namePtr) : '', flags: this.hdr.readInt32LE(at + 8), code: this.hdr.readInt32LE(at + 12), codeSize: this.hdr.readInt32LE(at + 16) });
    }
    return out;
  }
}

/** [entry number, guest path, stream offset] of the EXTRACTFILE entries of the section 0 that the script puts under instdir (decision 0129) */
export function installTree(script, instdir = DEFAULT_INSTDIR) {
  const sec0 = script.sections().find((s) => s.i === 0);
  const first = sec0.code, size = sec0.codeSize;
  let outdir = instdir, varOutdir = instdir;
  const sub = (s) => s.split('$_OUTDIR').join(varOutdir).split('$INSTDIR').join(instdir).split('$OUTDIR').join(outdir);
  const rows = [];
  for (let n = first; n < first + size; n++) {
    const e = script.words[n];
    const op = e[0];
    if (op === OP_CREATEDIR) {
      const p = sub(script.string(e[1]));
      if (e[2] === 1) outdir = p;
    } else if (op === OP_ASSIGNVAR && e[1] === 31) {
      if (script.string(e[2]) === '$OUTDIR') varOutdir = outdir;
    } else if (op === OP_EXTRACTFILE) {
      const name = sub(script.string(e[2]));
      if (name.includes('$PLUGINSDIR') || name.includes('$TEMP')) continue;
      const p = name.includes('\\') && name.includes(':') ? name : `${outdir.replace(/\\+$/, '')}\\${name}`;
      if (p.toLowerCase().startsWith(instdir.toLowerCase() + '\\')) rows.push([n, p, e[3] >>> 0]);
    }
  }
  return rows;
}
