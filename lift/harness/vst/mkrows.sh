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
# the methods of the host's IMessage / IAttributeList (vst_msg3a.c), IBStream (vst_stream3a.c) and the thread entry (vst_env3a.c), the rows of gen_vst3a.sh
j=0; for n in msg_qi msg_addref msg_release msg_getMessageID msg_setMessageID msg_getAttributes attr_qi attr_addref attr_release attr_setInt attr_getInt attr_setFloat attr_getFloat attr_setString attr_getString attr_setBinary attr_getBinary; do printf '0x%x vst3a_%s\n' $((0x7FFC40000000 + 0x10 * (j + 1))) $n >> "$OUT"; j=$((j + 1)); done
j=0; for n in qi addref release read write seek tell; do printf '0x%x vst3a_stream_%s\n' $((0x7FFC20000000 + 0x10 * (j + 1))) $n >> "$OUT"; j=$((j + 1)); done
printf '0x7ffc30000010 vst3a_thread_entry\n' >> "$OUT"
echo "mkrows: $k host rows, and the IMessage, IBStream and thread rows" >&2
