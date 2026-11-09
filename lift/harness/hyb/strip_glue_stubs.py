#!/usr/bin/env python3
"""hyb1: removes from the emscripten glue the stubs of the undefined f_<va> (the functions that the direct back end defines; wasm-merge resolves them, so after the merge the stubs are dead code):
    function _f_<va>(...args\n  ) {\n  abort('missing function: f_<va>');\n  }\n  _f_<va>.stub = true;\n   and the entry `f_<va>: _f_<va>,` of the import object. usage: strip_glue_stubs.py IN OUT"""
import re, sys
s = open(sys.argv[1]).read()
n0 = len(s)
# decision 0234: the glue of a -pthread link has no 2-space indentation, double quotes and `/** @export */` in front of the entries; both forms are accepted (the old form matches as before)
s, a = re.subn(r"\n[ ]*function _(f_[0-9a-f]+)\(\.\.\.args\s*\) \{\s*abort\(['\"]missing function: \1['\"]\);\s*\}\s*_\1\.stub = true;\n", "\n", s)
s, b = re.subn(r"\n[ ]*(?:/\*\* @export \*/ )?(f_[0-9a-f]+): _\1,", "", s)
open(sys.argv[2], 'w').write(s)
print(f'stubs removed {a}, import entries removed {b}, {n0} -> {len(s)} bytes')
