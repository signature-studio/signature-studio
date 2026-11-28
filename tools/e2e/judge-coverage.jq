# judge-coverage.jq (Issue #1485). Input: coverage.json (tools/e2e/coverage.mjs, from the runs of all shards together). Run with -e: exit 0 only when every area that has a .min meets it.
# An area's minimum (tools/e2e/features/<領域>.min): `scenarios` (the passing scenarios of the area) and `coverage` (the covered share of its features, 0 to 1).
# An area with no .min is not judged. Each area below its minimum is named on stderr (debug). The judge is for the merged coverage only: a single shard is not judged.
(.perArea | to_entries[] | select(.value.min != null)
  | select(.value.scenarios < .value.min.scenarios or .value.ratio < .value.min.coverage)
  | debug("E2E 網羅: \(.key) は最小 \(.value.min.scenarios) シナリオ・網羅率 \(.value.min.coverage) を下回る (シナリオ \(.value.scenarios)、網羅 \(.value.covered)/\(.value.features) = \(.value.ratio))")
  | empty),
(.perArea | to_entries | all(.[]; .value.min == null or (.value.scenarios >= .value.min.scenarios and .value.ratio >= .value.min.coverage)))
