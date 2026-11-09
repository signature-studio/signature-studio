#!/usr/bin/python3
"""Read-only dump of the script (instructions, sections, registry operations) of an NSIS 3 Unicode installer.

Companion of extract_nsis2.py (it imports its header reader; the installer is only read). It does not run anything.

usage:
  nsis_script_dump.py INSTALLER --info                    header fields, sections, default install directory
  nsis_script_dump.py INSTALLER --registry                every WriteReg*/ReadRegStr/DeleteReg* instruction, with the section it is in
  nsis_script_dump.py INSTALLER --range FIRST LAST        instructions FIRST..LAST (entry numbers), decoded
  nsis_script_dump.py INSTALLER --var N                   every instruction that assigns user variable N (StrCpy / ReadRegStr / Pop / IntOp ...)
  nsis_script_dump.py INSTALLER --strings PATTERN         string-table entries that contain PATTERN (case-insensitive)

Decoding (NSIS source Source/exehead/fileform.h, exehead/util.c GetNSISString, Source/build.cpp init_shellconstantvalues):
  string codes in the Unicode string table: 0x01 language string, 0x02 shell folder (next WCHAR: low byte = CSIDL for the current user, high byte
  = CSIDL for all users; a low byte with bit 0x80 is a registry-based folder: HKLM\\Software\\Microsoft\\Windows\\CurrentVersion value
  ProgramFilesDir / CommonFilesDir, bit 0x40 = 32-bit registry view), 0x03 variable (next WCHAR: DECODE_SHORT), 0x04 next WCHAR is literal.
  user variables 0..31 are the built-in ones ($0..$9 $R0..$R9 $CMDLINE $INSTDIR $OUTDIR $EXEDIR $LANGUAGE $TEMP $PLUGINSDIR $EXEPATH $EXEFILE
  $HWNDPARENT $_CLICK $_OUTDIR); the names of the script's own variables (>= 32) are not stored in the installer: they are printed as $v<N>.
Opcodes: this installer's entries are 36 bytes (8 operands) and its opcode numbers equal the enum of the current NSIS fileform.h up to
  EW_EXTRACTFILE (20) and are that enum + 2 from 23 (DELETEFILE) to 58 (FPUTS) (checked against the operand shapes of the instructions, see
  OPNAMES); the numbers above 58 are only named where the operand shapes were checked. Unknown numbers are printed as op<N>.
"""
import struct
import sys

sys.dont_write_bytecode = True
import os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import extract_nsis2 as X

BUILTIN = ["$%d" % i for i in range(10)] + ["$R%d" % i for i in range(10)] + [
    "$CMDLINE", "$INSTDIR", "$OUTDIR", "$EXEDIR", "$LANGUAGE", "$TEMP", "$PLUGINSDIR", "$EXEPATH", "$EXEFILE",
    "$HWNDPARENT", "$_CLICK", "$_OUTDIR"]

# (a, b) of a shell code -> NSIS name (Source/build.cpp). a/b are CSIDLs; 0x80 bit = registry based
CSIDL = {0x02: "PROGRAMS", 0x17: "COMMON_PROGRAMS", 0x07: "STARTUP", 0x18: "COMMON_STARTUP", 0x10: "DESKTOPDIRECTORY",
         0x19: "COMMON_DESKTOPDIRECTORY", 0x0b: "STARTMENU", 0x16: "COMMON_STARTMENU", 0x1a: "APPDATA", 0x23: "COMMON_APPDATA",
         0x05: "PERSONAL", 0x2e: "COMMON_DOCUMENTS", 0x1c: "LOCAL_APPDATA", 0x15: "TEMPLATES", 0x14: "FONTS", 0x28: "PROFILE",
         0x24: "WINDOWS", 0x25: "SYSTEM", 0x1d: "ALTSTARTUP", 0x0d: "MYMUSIC", 0x27: "MYPICTURES", 0x0e: "MYVIDEO"}
SHELL = {(0x24, 0x24): "$WINDIR", (0x25, 0x25): "$SYSDIR", (0x02, 0x17): "$SMPROGRAMS", (0x07, 0x18): "$SMSTARTUP",
         (0x10, 0x19): "$DESKTOP", (0x0b, 0x16): "$STARTMENU", (0x1a, 0x1a): "$QUICKLAUNCH", (0x05, 0x2e): "$DOCUMENTS",
         (0x14, 0x14): "$FONTS", (0x15, 0x2d): "$TEMPLATES", (0x1a, 0x23): "$APPDATA", (0x1c, 0x23): "$LOCALAPPDATA",
         (0x28, 0x28): "$PROFILE", (0x1a, 0x5a): "$USERAPPDATA", (0x23, 0x23): "$COMMONPROGRAMDATA"}

OPNAMES = {
    0: "INVALID", 1: "RET", 2: "NOP/JUMP", 3: "ABORT", 4: "QUIT", 5: "CALL", 6: "UPDATETEXT", 7: "SLEEP", 8: "BRINGTOFRONT",
    9: "CHDETAILSVIEW", 10: "SETFILEATTRIBUTES", 11: "CREATEDIR", 12: "IFFILEEXISTS", 13: "SETFLAG", 14: "IFFLAG", 15: "GETFLAG",
    16: "RENAME", 17: "GETFULLPATHNAME", 18: "SEARCHPATH", 19: "GETTEMPFILENAME", 20: "EXTRACTFILE",
    23: "DELETEFILE", 24: "MESSAGEBOX", 25: "RMDIR", 26: "STRLEN", 27: "ASSIGNVAR", 28: "STRCMP", 29: "READENVSTR",
    30: "INTCMP", 31: "INTOP", 32: "INTFMT", 33: "PUSHPOP", 34: "FINDWINDOW", 35: "SENDMESSAGE", 36: "ISWINDOW",
    37: "GETDLGITEM", 38: "SETCTLCOLORS", 39: "LOADANDSETIMAGE", 40: "CREATEFONT", 41: "SHOWWINDOW", 42: "SHELLEXEC",
    43: "EXECUTE", 44: "GETFILETIME", 45: "GETDLLVERSION", 46: "REGISTERDLL/PLUGINCALL", 47: "CREATESHORTCUT", 48: "COPYFILES",
    49: "REBOOT", 50: "WRITEINI", 51: "READINISTR", 52: "DELREG", 53: "WRITEREG", 54: "READREGSTR", 55: "REGENUM",
    56: "FCLOSE", 57: "FOPEN", 58: "FPUTS", 69: "GETOSINFO",
}
# operand roles of e[1..]: s string index, i integer, v user variable, j jump (entry+1, 0 = next), r registry root key, - unused
SHAPES = {
    2: "j", 3: "s", 5: "j", 6: "si", 7: "s", 10: "si", 11: "sii", 12: "sjj", 13: "is", 14: "jjii", 15: "vi", 16: "ssi",
    17: "vsi", 18: "vs", 19: "vs", 20: "isiiii", 23: "si", 24: "isijij", 25: "si", 26: "vs", 27: "vsss", 28: "ssjji",
    29: "vsi", 30: "ssjjji", 31: "vssi", 32: "vssi", 33: "sii", 42: "isssi", 43: "siv", 46: "sssi", 47: "sssss", 48: "ssi",
    50: "ssss", 51: "vsss", 53: "rssshii", 54: "vrssi", 55: "vrssi", 56: "v", 57: "siiv", 58: "vsi",
}


class Script:
    def __init__(self, path):
        with open(path, "rb") as f:
            self.hdr, self.info = X.read_header(f)
        h = self.hdr
        self.flags, self.blocks = X.parse_blocks(h)
        self.eoff, self.num = self.blocks[2]
        self.es = X.entry_size(self.blocks)
        self.so = self.blocks[3][0]
        self.send = self.blocks[4][0]
        self.words = [struct.unpack_from("<%di" % (self.es // 4), h, self.eoff + i * self.es) for i in range(self.num)]
        self.cf_idx = self._find_str("CommonFilesDir")
        self.pf_idx = self._find_str("ProgramFilesDir")

    def _find_str(self, s):
        raw = self.hdr[self.so:self.send]
        i = raw.find(s.encode("utf-16-le") + b"\0\0")
        return None if i < 0 else i // 2

    def raw_string(self, idx):
        return self.hdr[self.so + idx * 2:self.send].split(b"\0\0", 1)[0]  # not used for codes

    def string(self, idx):
        if idx <= 0:
            return ""
        h = self.hdr
        pos = self.so + idx * 2
        out = []
        while pos + 2 <= len(h):
            u = struct.unpack_from("<H", h, pos)[0]
            pos += 2
            if u == 0:
                break
            if u in (1, 2, 3, 4):
                w = struct.unpack_from("<H", h, pos)[0]
                pos += 2
                if u == 3:
                    n = (w & 0x7F) | (((w >> 8) & 0x7F) << 7)
                    out.append(BUILTIN[n] if n < len(BUILTIN) else "$v%d" % n)
                elif u == 2:
                    out.append(self.shell(w & 0xFF, w >> 8))
                elif u == 1:
                    out.append("$LANG(%d)" % (-(((w & 0x7F) | (((w >> 8) & 0x7F) << 7))) - 1))
                else:
                    out.append(chr(w))
            else:
                out.append(chr(u))
        return "".join(out)

    def shell(self, a, b):
        if a & 0x80:
            which = a & 0x3F
            name = "ProgramFilesDir" if which == self.pf_idx else ("CommonFilesDir" if which == self.cf_idx else "?%d" % which)
            view = "32" if a & 0x40 else "64"
            pretty = ("$PROGRAMFILES" if which == self.pf_idx else "$COMMONFILES") + view
            return "%s{HKLM\\Software\\Microsoft\\Windows\\CurrentVersion\\%s,view%s,fallback=%r}" % (pretty, name, view, self.string(b))
        n = SHELL.get((a, b))
        if n:
            return n
        return "$SH(csidl %s/%s)" % (CSIDL.get(a, "0x%02x" % a), CSIDL.get(b, "0x%02x" % b))

    def entries_of_section(self):
        off, num = self.blocks[1]
        sz = (self.blocks[2][0] - off) // num
        out = []
        for i in range(num):
            name_ptr, types, flags, code, code_size, size_kb = struct.unpack_from("<6i", self.hdr, off + i * sz)
            out.append((i, name_ptr, flags, code, code_size, self.string(name_ptr) if name_ptr else ""))
        return out

    def ctx(self, secs, n):
        for (i, np_, fl, code, cs, name) in secs:
            if code <= n < code + cs:
                return "section#%d %r" % (i, name)
        return ""

    def fmt(self, n):
        e = self.words[n]
        op = e[0]
        name = OPNAMES.get(op, "op%d" % op)
        shape = SHAPES.get(op)
        parts = []
        for k, p in enumerate(e[1:]):
            role = shape[k] if shape and k < len(shape) else "i"
            if role == "s":
                parts.append(repr(self.string(p)) if p else "''")
            elif role == "v":
                parts.append(BUILTIN[p] if 0 <= p < len(BUILTIN) else "$v%d" % p)
            elif role == "j":
                parts.append("->#%d" % (p - 1) if p > 0 else ("(next)" if p == 0 else "rel%d" % p))
            elif role == "r":
                parts.append(ROOT.get(p, str(p)))
            else:
                parts.append(str(p))
        while parts and parts[-1] in ("0", "''", "(next)"):
            parts.pop()
        return "%6d %-22s %s" % (n, name, ", ".join(parts))


ROOT = {-2147483648: "HKCR", -2147483647: "HKCU", -2147483646: "HKLM", -2147483645: "HKU", -2147483644: "HKPD", -2147483643: "HKCC",
        0: "SHCTX", 0x40000000 - 2147483648: "HKCR32"}


def main(argv):
    if len(argv) < 3:
        print(__doc__)
        return 64
    s = Script(argv[1])
    mode = argv[2]
    secs = s.entries_of_section()
    if mode == "--info":
        print("firstheader/info:", s.info)
        print("header flags: %d, blocks (offset,num): %s" % (s.flags, s.blocks))
        print("entries: %d, entry size %d bytes" % (s.num, s.es))
        # fields of header after the 8 blocks (fileform.h): install_reg_rootkey, key_ptr, value_ptr, ... install_directory_ptr, auto_append
        w = struct.unpack_from("<%di" % (len(s.hdr) // 4 if len(s.hdr) < 1000 else 250), s.hdr, 0)
        base = 1 + 16
        print("install_reg_rootkey=%d key_ptr=%d value_ptr=%d" % (w[base], w[base + 1], w[base + 2]))
        print("code_onInit=%d" % w[base + 10])
        idp, ida = w[base + 53], w[base + 54]
        print("install_directory_ptr=%d -> %r ; install_directory_auto_append=%d -> %r" % (idp, s.string(idp), ida, s.string(ida)))
        print("sections:")
        for sec in secs:
            print("  #%d name=%r flags=%d code=%d size=%d" % (sec[0], sec[5], sec[2], sec[3], sec[4]))
        return 0
    if mode == "--registry":
        for n, e in enumerate(s.words):
            if e[0] in (52, 53, 54, 55):
                print(s.fmt(n), "   ;", s.ctx(secs, n))
        return 0
    if mode == "--range":
        a, b = int(argv[3]), int(argv[4])
        for n in range(a, min(b, s.num - 1) + 1):
            print(s.fmt(n))
        return 0
    if mode == "--var":
        v = int(argv[3])
        for n, e in enumerate(s.words):
            op = e[0]
            hit = (op in (27, 26, 29, 17, 18, 19, 32, 31, 51, 54, 55, 15, 57) and e[1] == v) or (op == 33 and e[2] == 1 and e[1] == v)
            if hit:
                print(s.fmt(n), "   ;", s.ctx(secs, n))
        return 0
    if mode == "--strings":
        pat = argv[3].lower()
        raw = s.hdr[s.so:s.send]
        i = 0
        while i < len(raw):
            j = raw.find(b"\0\0", i)
            while j >= 0 and (j - i) % 2:
                j = raw.find(b"\0\0", j + 1)
            if j < 0:
                break
            t = s.string(i // 2)
            if pat in t.lower():
                print(i // 2, repr(t[:200]))
            i = j + 2
        return 0
    print(__doc__)
    return 64


if __name__ == "__main__":
    sys.exit(main(sys.argv))
