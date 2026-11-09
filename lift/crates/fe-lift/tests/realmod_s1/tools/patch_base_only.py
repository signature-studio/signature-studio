"""negative control: copy a PE and change ONLY the ImageBase field (no DIR64 is applied). usage: patch_base_only.py IN OUT 0xBASE
OUT must not exist (mode 'x')."""
import struct, sys
src, dst, base = sys.argv[1], sys.argv[2], int(sys.argv[3], 16)
b = bytearray(open(src, 'rb').read())
lfanew = struct.unpack_from('<I', b, 0x3c)[0]
opt = lfanew + 24
old = struct.unpack_from('<Q', b, opt + 24)[0]
struct.pack_into('<Q', b, opt + 24, base)
open(dst, 'xb').write(b)
print('old_base %#x new_base %#x (only the header field changed)' % (old, base))
