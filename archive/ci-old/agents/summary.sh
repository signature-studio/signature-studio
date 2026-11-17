#!/bin/bash
# ci/agents/summary.sh <shard> - the job summary of one shard: meta.txt, the head of the report, the size of the diff. Prints Markdown on stdout.
set -u
shard=${1:?usage: ci/agents/summary.sh <shard>}
out=ci/agents/out/$shard
echo "### shard $shard"
echo
if [ -f "$out/meta.txt" ]; then echo '```'; cat "$out/meta.txt"; echo '```'; else echo "no meta.txt: the task did not finish (the job log says why)"; fi
echo
if [ -s "$out/report.md" ]; then
  echo "<details><summary>report.md (first 80 lines)</summary>"; echo
  head -80 "$out/report.md"
  echo; echo "</details>"
else
  echo "**no report.md**"
fi
if [ -s "$out/changes.patch" ]; then echo; echo "changes.patch: $(grep -c '^diff --git' "$out/changes.patch") file(s) (a proposal in the artifact; nothing was committed)"; fi
exit 0
