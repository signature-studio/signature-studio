#!/usr/bin/env python3
"""hyb (decision 0230): the names of the function imports of a wasm module whose module name is MODULE (default `c`: the C functions that the direct code of fe-wasmgen calls, `--mem-import c.memory`
and `--c-imports`). One name per line, in the order of the import section. This is what `wasm-objdump -x | grep '<c\\.'` printed in the earlier hybrid scripts, without needing wabt.
usage: wasm_c_imports.py MODULE.wasm [MODULE]"""
import sys


def leb(b, i):
    r = s = 0
    while True:
        x = b[i]
        i += 1
        r |= (x & 0x7F) << s
        s += 7
        if not x & 0x80:
            return r, i


def main():
    b = open(sys.argv[1], 'rb').read()
    want = sys.argv[2] if len(sys.argv) > 2 else 'c'
    if b[:4] != b'\0asm':
        sys.exit('not a wasm module')
    i = 8
    while i < len(b):
        sid = b[i]
        i += 1
        size, i = leb(b, i)
        end = i + size
        if sid == 2:
            n, j = leb(b, i)
            for _ in range(n):
                ml, j = leb(b, j)
                mod = b[j:j + ml].decode()
                j += ml
                fl, j = leb(b, j)
                name = b[j:j + fl].decode()
                j += fl
                kind = b[j]
                j += 1
                if kind == 0:
                    _, j = leb(b, j)
                    if mod == want:
                        print(name)
                elif kind == 1:
                    j += 1
                    flag = b[j]
                    j += 1
                    _, j = leb(b, j)
                    if flag & 1:
                        _, j = leb(b, j)
                elif kind == 2:
                    flag = b[j]
                    j += 1
                    _, j = leb(b, j)
                    if flag & 1:
                        _, j = leb(b, j)
                elif kind == 3:
                    j += 2
                elif kind == 4:
                    j += 1
                    _, j = leb(b, j)
                else:
                    sys.exit('unknown import kind %d' % kind)
            return
        i = end


main()
