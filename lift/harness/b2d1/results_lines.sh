#!/bin/bash
# the lines that decision 0120 compares between builds: RESULT, CHECK, VALUE of every test of a scan_all.sh log directory, test by test (sorted by test name), with the
# test name in front; the "attached" line (pipeline provider, differs by design in a swapped build) is not one of them.
# usage: results_lines.sh LOGDIR > file
for f in $(ls "$1"/*.out | sort); do t=$(basename "$f" .out); grep -a '^scan: \(RESULT\|CHECK\|VALUE\)' "$f" | sed "s|^|$t |"; done
