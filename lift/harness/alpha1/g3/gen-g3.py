#!/usr/bin/env python3
"""alpha1: generate a G3 script (THE INPUT IS A HARNESS SIMULATION). usage: gen-g3.py NAME > NAME.g3"""
import sys
PRE = """# alpha1 %s -- THE INPUT IS A HARNESS SIMULATION (G3 script), NOT A USER OPERATION
cpu wiz0
advance 3000
tap 13
advance 3000
open C:\\Program Files\\Image-Line\\FL Studio 21\\Data\\Templates\\Minimal\\Basic\\Basic.flp
advance 1500
cpu opened
snap opened
"""
def frames(tag, n, step=16):
    s = []
    for i in range(n):
        s.append("advance %d" % step)
        s.append("cpu %s%02d" % (tag, i))
    return "\n".join(s) + "\n"
name = sys.argv[1]
out = PRE % name
if name == 'pull1':        # File menu: choose the recent project (the row 'Basic.flp'); Add menu: choose the radio row 'Simple'
    out += "click 18 20 L\nadvance 800\ncpu fm_open\nsnap fm_open\n"
    out += "click 44 305 L\ncpu fm_sel\n" + frames('fm', 40)
    out += "snap fm_after\nadvance 1500\nclick 73 20 L\nadvance 800\ncpu am_open\nsnap am_open\n"
    out += "click 99 133 L\ncpu am_sel\n" + frames('am', 40)
    out += "snap am_after\nadvance 500\n"
elif name == 'pick1s':      # Add menu -> 'Plugin picker', then Esc
    out += "click 73 20 L\nadvance 800\ncpu am_open\nsnap am_open\n"
    out += "click 241 83 L\ncpu pk_sel\n" + frames('pk', 50)
    out += "snap pk_open\nadvance 1000\nsnap pk_open2\ntap 27\ncpu pk_esc\n" + frames('pc', 30)
    out += "snap pk_closed\nadvance 500\n"
elif name == 'f8':          # F8 picker
    out += "tap 0x77\ncpu f8_sel\n" + frames('f8', 50)
    out += "snap f8_open\nadvance 1000\nsnap f8_open2\ntap 27\ncpu f8_esc\n" + frames('f8c', 30)
    out += "snap f8_closed\nadvance 500\n"
else:
    raise SystemExit("unknown scenario")
out += "cpu end\nend\n"
sys.stdout.write(out)
