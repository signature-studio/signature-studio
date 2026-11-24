#!/bin/bash
# The dispatch rows of the host methods (vst_host.h, vst_hm.def): one "<0x pseudo-VA> <function>" line per row, the VA of row k = VST_HM_BASE + 0x10 * (k + 1).
# usage: bash lift/harness/vst/mkrows.sh OUT.txt
set -u -o pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
OUT=${1:?usage: mkrows.sh OUT.txt}
BASE=$((0x7FFC10000000))
k=0
: > "$OUT"
while read -r line; do
    case $line in
        HM\(*\)) n=${line#HM(}; n=${n%)}; printf '0x%x vst_hm_%s\n' $((BASE + 0x10 * (k + 1))) "$n" >> "$OUT"; k=$((k + 1)) ;;
    esac
done < "$HERE/vst_hm.def"
echo "mkrows: $k host rows" >&2
