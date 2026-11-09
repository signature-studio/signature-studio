#!/bin/bash
# stopname.sh VA : for "unsupported instruction at VA" on a jmp/call through the IAT, print the import (dll!name). Read only.
HERE=$(cd "$(dirname "$0")" && pwd)
python3 - "$1" "$HERE" <<'PY'
import subprocess, re, sys
va, here = sys.argv[1], sys.argv[2]
f = here + '/../../out-vst1/bin/OTT.vst3'
o = subprocess.check_output(['python3', here + '/pedis.py', f, va, '2']).decode().split('\n')
m = re.search(r'rip \+ (0x[0-9a-f]+)', o[0])
if not m: print(o[0]); sys.exit()
nxt = int(o[1].split(':')[0], 16)
slot = nxt + int(m.group(1), 16)
print(o[0].strip()); print(subprocess.check_output(['python3', here + '/iat.py', f, hex(slot)]).decode().strip())
PY
