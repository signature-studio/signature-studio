#!/bin/bash
# ci/compare-bundle.sh DIR_A DIR_B - which files of two engine bundles are byte-identical (SHA-256 of every file by its role: wasm, glue, seed pack, DLL pack, and their brotli forms; manifest.json differs by design:
# it carries the build time and the HEAD). Use it to compare a CI bundle with a bundle built by hand (e.g. ship2's), or two CI runs of the same inputs.
set -u -o pipefail
a=${1:?DIR_A}; b=${2:?DIR_B}
role() { case "$1" in engine.*.wasm.br) echo wasm.br ;; engine.*.wasm) echo wasm ;; engine-glue.*.js.br) echo glue.br ;; engine-glue.*.js) echo glue ;; seed.*.flpack.br) echo seed.br ;; seed.*.flpack) echo seed ;; dll.*.flpack.br) echo dll.br ;; dll.*.flpack) echo dll ;; manifest.json) echo manifest ;; *) echo other ;; esac; }
printf '%-10s %-18s %-18s %s\n' role "A (sha256 head)" "B (sha256 head)" same
for f in "$a"/*; do
  r=$(role "$(basename "$f")"); [ "$r" = other ] && continue
  g=$(ls "$b"/* 2>/dev/null | while read -r x; do [ "$(role "$(basename "$x")")" = "$r" ] && echo "$x"; done | head -1)
  sa=$(sha256sum "$f" | cut -c1-16); sb=-; [ -n "$g" ] && sb=$(sha256sum "$g" | cut -c1-16)
  printf '%-10s %-18s %-18s %s\n' "$r" "$sa" "$sb" "$([ "$sa" = "$sb" ] && echo yes || echo NO)"
done
