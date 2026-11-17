#!/bin/bash
# Wrapper so that lift/harness/a4-survey.sh (which runs "$NATIVE_BIN" "$DLL") can run the two-module native binary:
#   NATIVE_BIN=lift/harness/two/two-native-run.sh TWO_NATIVE=<two-native> TWO_REAL=<rebased real module file> bash lift/harness/a4-survey.sh
# N real modules: TWO_REAL may hold several files separated by ':' (main_two.c of this directory splits it). The command line and the
# behaviour with one file are unchanged. a4-survey.sh itself needs no change: it passes "$DLL" only.
exec "${TWO_NATIVE:?TWO_NATIVE is not set}" "$1" "${TWO_REAL:?TWO_REAL is not set}" "${@:2}"
