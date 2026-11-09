"""Decision 0165 (vst3a): extracts the saved states of the Serum 2 instances from the reference project's FLP (the state files are NOT in the results: they are the project's data).
usage: extract_states.py serina1-serum2.flp OUTDIR    (needs docs/investigations/reference-project/tools/flp_model.py of the repository)
Writes OUTDIR/s<N>_c3.bin (processor state, 'XferJson' + zstd; IComponent::setState), s<N>_c2.bin (controller state; IEditController::setState), s<N>_c4.bin (parameter-id list), s<N>_c1.bin, for the
Serum 2 instances N = 0.. in the order of the FLP's PluginParams events (event 213): N = 0..5 are the channels 1, 2, 3, 28, 33, 37 (plugins.tsv of the reference-project analysis)."""
import os, sys
sys.dont_write_bytecode = True
here = os.path.dirname(os.path.abspath(__file__))
root = os.path.abspath(os.path.join(here, '../../../..'))   # lift/harness/vst3a/tools -> repository root
sys.path.insert(0, os.path.join(root, 'docs/investigations/reference-project/tools'))
import flp_model as M
buf = open(sys.argv[1], 'rb').read()
out = sys.argv[2]
os.makedirs(out, exist_ok=True)
P = M.build(buf)
n = 0
for e in P['events']:
    if e.id != 213: continue
    r = M.parse_plugin_params(e.d)
    if not r or r.get('name') != 'Serum2': continue
    recs, ok = M.records(r['state'], 4)
    for cid, off, sz in recs:
        open(os.path.join(out, 's%d_c%d.bin' % (n, cid)), 'wb').write(r['state'][off:off + sz])
    n += 1
print(n, 'Serum 2 states written to', out)
