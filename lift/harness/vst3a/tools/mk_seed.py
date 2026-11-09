"""usage: mk_seed.py CONTENT_DIR OUT.tsv : the FLE_VFS_SEED list for the extracted content: Tables/ and Samples/ under C:\\SerumData\\, Multisamples/ under C:\\SerumData\\Presets\\ (the plugin looks for
Multisamples under PresetsDir and for Tables/Samples under its parent; the registry value PresetsDir = C:\\SerumData\\Presets is written by main_vst3a --presets-dir). Append this list to
lift/harness/vst/vfs-seed-vst.tsv (the profile folders) to get the seed file for FLE_VFS_SEED."""
import hashlib, os, sys
root, out = sys.argv[1:3]
n = 0
with open(out, 'w') as f:
    for dp, dn, fn in sorted(os.walk(root)):
        for x in sorted(fn):
            p = os.path.join(dp, x)
            rel = os.path.relpath(p, root).replace('/', '\\')
            g = ('C:\\SerumData\\Presets\\' if rel.startswith('Multisamples\\') else 'C:\\SerumData\\') + rel
            f.write('%s\t%s\t%s\n' % (g, os.path.abspath(p), hashlib.sha256(open(p, 'rb').read()).hexdigest()))
            n += 1
print(n, 'entries')
