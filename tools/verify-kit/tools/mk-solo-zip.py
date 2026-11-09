#!/usr/bin/env python3
"""mk-solo-zip.py ORIG.zip SOLO12.flp OUT.zip  (S3a, ci1 0326): OUT.zip = ORIG.zip with the .flp member replaced by SOLO12.flp (the member keeps its name, serina1-serum2.flp, which the sample paths of the project are relative to); every other member is copied as it is (bytes, names, order).
The reference project is opened as a zip by the page (decision 0248); solo12.flp is made by opt1 (the reference project with Track 1-5,7,13,15-17,19,20 soloed)."""
import sys, zipfile
src, flp, out = sys.argv[1:4]
zi = zipfile.ZipFile(src); names = [i.filename for i in zi.infolist() if i.filename.lower().endswith('.flp')]
assert len(names) == 1, f'the zip must hold exactly one .flp: {names}'
with zipfile.ZipFile(out, 'w', zipfile.ZIP_DEFLATED) as zo:
    for i in zi.infolist():
        data = open(flp, 'rb').read() if i.filename == names[0] else zi.read(i.filename)
        zo.writestr(zipfile.ZipInfo(i.filename, i.date_time), data, compress_type=i.compress_type)
print('replaced', names[0], 'in', src, '->', out, sum(1 for _ in zipfile.ZipFile(out).infolist()), 'members')
