#!/bin/bash
# mk-r53d-variants.sh   (R5-3d, decision 0371; no arguments; no browser, no server, no build; removes nothing)
# Makes four bundle dirs under ~/.cache/signature-studio/work-r53d/v<k>/<host dir name>/. Each has the HOST bundle's engine wasm, glue and DLL pack (symlinks, with their .br),
# the SEED PACK of the DONOR bundle (copied, with its .br), and a manifest.json that is the host's manifest with only seedPack replaced by the donor's entry
# (url renamed to the host dir, encodedSize = the byte size of the .br that was written).
#   v1: host E13 (fl-engine-ci-0363b050),  donor work-r25 (its seed pack)
#   v2: host work-r25 (fl-Oz),             donor E13 (its seed pack)
#   v3: host E13,                          donor E13 (control of the procedure)
#   v4: host work-r25,                     donor work-r25 (control of the procedure)
# The bundles are read only. Stops if work-r53d already exists.
set -eu
REPO=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)
E13=$REPO/artifacts/bundles/fl-engine-ci-0363b050
W25=$HOME/.cache/signature-studio/work-r25/bundle/fl-Oz
W=$HOME/.cache/signature-studio/work-r53d
[ ! -e "$W" ] || { echo "$W exists: stop (this script removes nothing)"; exit 2; }
for d in "$E13" "$W25"; do [ -f "$d/manifest.json" ] || { echo "no manifest in $d"; exit 2; }; done

mk() {
  local v=$1 H=$2 D=$3 hn A f
  hn=$(basename "$H"); A=$W/$v/$hn
  mkdir -p "$A"
  for f in "$H"/engine.*.wasm "$H"/engine.*.wasm.br "$H"/engine-glue.*.js "$H"/engine-glue.*.js.br "$H"/dll.*.flpack "$H"/dll.*.flpack.br; do
    [ -f "$f" ] || { echo "missing in host: $f"; exit 3; }
    ln -s "$f" "$A/$(basename "$f")"
  done
  local dseeds=( "$D"/seed.*.flpack )
  [ "${#dseeds[@]}" -eq 1 ] && [ -f "${dseeds[0]}" ] && [ -f "${dseeds[0]}.br" ] || { echo "donor seed in $D: want exactly one seed.*.flpack with its .br"; exit 3; }
  local sn; sn=$(basename "${dseeds[0]}")
  cp "${dseeds[0]}" "$A/$sn"
  cp "${dseeds[0]}.br" "$A/$sn.br"
  python3 - "$H" "$D" "$A" "$hn" "$sn" <<'PY'
import hashlib, json, os, sys
host_dir, donor_dir, out, hname, sname = sys.argv[1:6]
hm = json.load(open(os.path.join(host_dir, 'manifest.json')))
dm = json.load(open(os.path.join(donor_dir, 'manifest.json')))
if not hm['engine']['wasm']['url'].startswith('/assets/%s/' % hname):
    sys.exit('the host manifest does not name its own dir %s' % hname)
seed = os.path.join(out, sname)
data = open(seed, 'rb').read()
sp = dict(dm['seedPack'])
if len(data) != sp['size'] or hashlib.sha256(data).hexdigest() != sp['sha256']:
    sys.exit('the copied seed does not match the donor manifest (size or sha256)')
sp['url'] = '/assets/%s/%s' % (hname, sname)
sp['encodedSize'] = os.path.getsize(seed + '.br')
hm['seedPack'] = sp
with open(os.path.join(out, 'manifest.json'), 'w') as fh:
    json.dump(hm, fh, indent=2, ensure_ascii=False)
    fh.write('\n')
print('manifest written: seedPack url %s size %d sha256 %s encodedSize %d' % (sp['url'], sp['size'], sp['sha256'], sp['encodedSize']))
PY
  echo "== $v: $A"
  ls -la "$A" | tail -n +4
}

mk v1 "$E13" "$W25"
mk v2 "$W25" "$E13"
mk v3 "$E13" "$E13"
mk v4 "$W25" "$W25"
