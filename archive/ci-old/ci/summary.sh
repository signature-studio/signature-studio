#!/bin/bash
# ci/summary.sh FILE... - the cache table for $GITHUB_STEP_SUMMARY (and the log): one row per cache operation (rows written by ci/actions/r2-cache), with the count of restore failures on top.
set -u -o pipefail
rows=$(cat "$@" 2>/dev/null)
[ -n "$rows" ] || { echo "(no cache operation)"; exit 0; }
fails=$(printf '%s\n' "$rows" | grep -c -E 'RESTORE FAILED|SAVE NOT FOUND' || true)
echo "### Stage cache (R2 cache bucket)"
echo
[ "$fails" = 0 ] && echo "Restores that failed although the lookup had found the entry: **0**" || echo "**$fails operation(s) failed although the lookup had found the entry (the stage was rebuilt or the job failed): see the rows marked FAILED**"
echo
echo "| stage | key | operation | result |"
echo "|---|---|---|---|"
printf '%s\n' "$rows" | awk -F'\t' '{k=$2; if (length(k)>34) k=substr(k,1,34) "…"; printf "| %s | `%s` | %s | %s |\n", $1, k, $3, $4}'
