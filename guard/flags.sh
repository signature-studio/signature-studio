#!/usr/bin/env bash
# guard/flags.sh BASE [HEAD] : 依頼者の指示「オプトイン・オプトアウトを作らない」(2026-11-05)。追加した行に、製品の新しい旗を作る形が無いことを検査する。
# 対象: app/src と lift の runtime・crates・harness の追加行。検出: getenv( / #ifdef / #ifndef / #if defined / URLSearchParams / searchParams.get / process.env.
# 終了コード 0 = 指摘なし、1 = 指摘あり。
base=${1:?usage: guard/flags.sh BASE [HEAD]}; head=${2:-HEAD}
re='getenv[[:space:]]*\(|^[[:space:]]*#[[:space:]]*(ifdef|ifndef)|^[[:space:]]*#[[:space:]]*if[[:space:]]+(!?defined|[A-Z_]+)|URLSearchParams|searchParams\.get|process\.env\.'
diff=$(git diff --unified=0 --diff-filter=AM "$base" "$head" -- app/src lift/runtime lift/crates lift/harness)
printf '%s\n' "$diff" | awk '/^\+\+\+ /{f=substr($2,3)} /^@@/{split($3,a,","); l=substr(a[1],2)+0; next} /^\+[^+]/{print f ":" l ":" substr($0,2); l++}' \
  | grep -E -- "$re" | cut -d: -f1,2 | sed 's/^/newflag\t/'
c=$(printf '%s\n' "$diff" | grep -E '^\+[^+]' | grep -cE -- "$re")
[ "${c:-0}" -eq 0 ]
