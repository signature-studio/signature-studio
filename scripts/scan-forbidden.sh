#!/usr/bin/env bash
# usage: scripts/scan-forbidden.sh [PATH...]
# Without arguments: every tracked file (`git ls-files`) and every untracked, not-ignored file. With arguments: those files (a directory is walked).
# Rules (printed by name): vendor secret size oldpath cfhex cfhost term spacing. Exit 0 = nothing found, 1 = findings, 2 = bad usage.
# The output never contains a matched value: only the rule name, the file and the line number (the term rule also prints the ID and the replacement word).
# Exceptions: scripts/forbidden-allowlist.txt (see its header; a path ending in "/" is a directory prefix). With arguments the paths are taken relative to the current directory
# (so scan an outside tree from inside it: `cd TREE && bash REPO/scripts/scan-forbidden.sh .`; an absolute path never matches an allowlist path).
set -o pipefail

die() { echo "scan-forbidden: $*" >&2; exit 2; }

# The vendor name and the old working tree's name are built from halves so that this file does not contain either of them.
v1=anthr
v2=opic
vendor="$v1$v2"
o1=signature-studio-
o2=develop
oldpath="$o1$o2"

declare -A RULE_PATTERN=(
  [vendor]="$vendor"
  [secret]="(^|[^A-Za-z0-9])sk-[A-Za-z0-9_-]{20,}|gh[pousr]_[A-Za-z0-9]{20,}|github_pat_[A-Za-z0-9_]{20,}|AKIA[0-9A-Z]{16}|-----BEGIN [A-Z ]*PRIVATE KEY|xox[abp]-[A-Za-z0-9-]{10,}|eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}|Bearer [A-Za-z0-9._~+/=-]{20,}|[A-Z_]*(ACCESS_KEY_ID|SECRET_ACCESS_KEY|API_TOKEN|API_KEY)[[:space:]]*[:=][[:space:]]*[^[:space:]]{12,}"
  [oldpath]="$oldpath"
  [cfhex]="(^|[^0-9A-Za-z])[0-9a-fA-F]{32}($|[^0-9A-Za-z])"
  [cfhost]="[A-Za-z0-9-]+\.r2\.cloudflarestorage\.com|[A-Za-z0-9-]+\.workers\.dev"
)
RULE_ORDER=(vendor secret oldpath cfhex cfhost)
MAX_BYTES=95000000

script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd) || die "cannot resolve the script directory"
allowlist="$script_dir/forbidden-allowlist.txt"

declare -a AL_RULE=() AL_PATH=() AL_TEXT=()
if [ -f "$allowlist" ]; then
  while IFS='|' read -r r p t || [ -n "$r" ]; do
    case "$r" in ''|'#'*) continue ;; esac
    AL_RULE+=("$r"); AL_PATH+=("$p"); AL_TEXT+=("$t")
  done < "$allowlist"
fi

# term rule: scripts/forbidden-terms.tsv (ID, fragments, replacement, exception). The fragments are joined here at run time, so no term is written in this file.
declare -a TM_ID=() TM_PAT=() TM_REP=() TM_EXC=()
terms_file="$script_dir/forbidden-terms.tsv"
if [ -f "$terms_file" ]; then
  while IFS=$'\t' read -r tid frags rep exc || [ -n "$tid" ]; do
    case "$tid" in ''|'#'*|ID) continue ;; esac
    TM_ID+=("$tid")
    TM_PAT+=("$(printf '%s' "$frags" | tr -d '|')")
    TM_REP+=("$rep")
    TM_EXC+=("$(printf '%s' "$exc" | tr -d '|')")
  done < "$terms_file"
fi

# allowed RULE FILE LINE_TEXT -> 0 when an allowlist entry covers it. An empty text allows the rule for the whole file.
# An entry path ending in "/" (and longer than "/") is a prefix: it covers every file below that directory.
allowed() {
  local i p
  for i in "${!AL_RULE[@]}"; do
    [ "${AL_RULE[$i]}" = "$1" ] || continue
    p=${AL_PATH[$i]}
    [ "$p" = "$2" ] || { [[ "$p" == ?*/ ]] && [[ "$2" == "$p"* ]]; } || continue
    [ -z "${AL_TEXT[$i]}" ] && return 0
    [[ "$3" == *"${AL_TEXT[$i]}"* ]] && return 0
  done
  return 1
}

list=$(mktemp "${TMPDIR:-/tmp}/scan-forbidden.XXXXXX") || die "cannot create a temporary file in ${TMPDIR:-/tmp}"
trap 'rm -f "$list"' EXIT

if [ "$#" -eq 0 ]; then
  root=$(git rev-parse --show-toplevel 2>/dev/null) || die "not inside a git repository (give PATH arguments, or run it in the repository)"
  cd "$root" || die "cannot enter $root"
  { git ls-files -z; git ls-files -z --others --exclude-standard; } | sort -zu > "$list" || die "git ls-files failed in $root"
else
  for a in "$@"; do
    if [ -d "$a" ]; then
      find "$a" -path '*/.git' -prune -o -type f -print0 | sed -z 's#^\./##' >> "$list" || die "cannot walk $a"
    elif [ -e "$a" ]; then
      printf '%s\0' "${a#./}" >> "$list"
    else
      die "no such path: $a"
    fi
  done
  sort -zu "$list" -o "$list" || die "sort failed"
fi

findings=0
declare -A SEEN=()
report() { # rule file line [extra]
  local key="$1 $2:$3 ${4:-}"
  [ -n "${SEEN[$key]:-}" ] && return
  SEEN[$key]=1
  if [ -n "${4:-}" ]; then printf '%s\t%s:%s\t%s\n' "$1" "$2" "$3" "$4"; else printf '%s\t%s:%s\n' "$1" "$2" "$3"; fi
  findings=$((findings + 1))
}

# Regular files only (a symlink or a deleted tracked file is skipped); the same list feeds every rule.
files=0
regular=$(mktemp "${TMPDIR:-/tmp}/scan-forbidden.XXXXXX") || die "cannot create a temporary file"
trap 'rm -f "$list" "$regular"' EXIT
while IFS= read -r -d '' f; do
  [ -f "$f" ] && [ ! -L "$f" ] || continue
  printf '%s\0' "$f" >> "$regular"
  files=$((files + 1))
  # path rules and size
  shopt -s nocasematch
  if [[ "$f" == *"$vendor"* ]] && ! allowed vendor "$f" ""; then report vendor "$f" 0; fi
  shopt -u nocasematch
  if [[ "$f" == *"$oldpath"* ]] && ! allowed oldpath "$f" ""; then report oldpath "$f" 0; fi
  size=$(stat -c %s -- "$f" 2>/dev/null) || { echo "scan-forbidden: cannot stat $f" >&2; continue; }
  if [ "$size" -gt "$MAX_BYTES" ] && ! allowed size "$f" ""; then report size "$f" 0; fi
done < "$list"

# content rules: grep prints "file<NUL>line:text"; only the file and the line number are kept.
for rule in "${RULE_ORDER[@]}"; do
  flags=(-nIHZ -E)
  [ "$rule" = vendor ] && flags+=(-i)
  while IFS= read -r -d '' f && IFS= read -r rest; do
    n=${rest%%:*}
    text=${rest#*:}
    allowed "$rule" "$f" "$text" && continue
    report "$rule" "$f" "$n"
  done < <(xargs -0 -a "$regular" grep "${flags[@]}" -e "${RULE_PATTERN[$rule]}" 2>/dev/null)
done

# spacing: the English word Issue must have a half-width space to an adjacent Japanese character (a backquote or a URL breaks the adjacency, so code is not matched).
jpc='\x{3040}-\x{30FF}\x{3005}\x{4E00}-\x{9FFF}'
while IFS= read -r -d '' f && IFS= read -r rest; do
  n=${rest%%:*}
  text=${rest#*:}
  allowed spacing "$f" "$text" && continue
  report spacing "$f" "$n" "add a half-width space between Issue and the Japanese word"
done < <(LC_ALL=C.UTF-8 xargs -0 -a "$regular" grep -nIHZ -P -e "[${jpc}]Issue|Issue[${jpc}]" 2>/dev/null)

# term: a literal match of each term; a line that only has the term inside its exception text is not a finding.
for ((i = 0; i < ${#TM_PAT[@]}; i++)); do
  [ -n "${TM_PAT[$i]}" ] || continue
  while IFS= read -r -d '' f && IFS= read -r rest; do
    n=${rest%%:*}
    text=${rest#*:}
    if [ -n "${TM_EXC[$i]}" ]; then
      stripped=${text//"${TM_EXC[$i]}"/}
      [[ "$stripped" == *"${TM_PAT[$i]}"* ]] || continue
    fi
    allowed term "$f" "$text" && continue
    report term "$f" "$n" "${TM_ID[$i]} -> ${TM_REP[$i]}"
  done < <(xargs -0 -a "$regular" grep -nIHZ -F -e "${TM_PAT[$i]}" 2>/dev/null)
done

if [ "$findings" -gt 0 ]; then
  echo "scan-forbidden: $findings finding(s) in $files file(s). Values are not shown. Fix the file, or add a line to scripts/forbidden-allowlist.txt if it is a false positive." >&2
  exit 1
fi
echo "scan-forbidden: 0 findings in $files file(s)"
