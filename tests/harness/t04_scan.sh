#!/usr/bin/env bash
# t04: scripts/scan-forbidden.sh. Every forbidden sample is assembled from pieces at run time, so this file itself contains none of them.
# Checks: each rule fires, near-misses do not, the output never contains a matched value, exit codes, the allowlist, explicit paths, untracked files.
# shellcheck source=lib.sh
. "$(dirname "$0")/lib.sh"
set -o pipefail

sb=$(new_sandbox t04)
repo="$sb/repo"
make_repo_copy "$repo"
scan() { (cd "$repo" && bash scripts/scan-forbidden.sh "$@" 2>&1); }
v1="anthr"; v2="opic"; o1="signature-studio-"; o2="develop"

w() { printf '%s\n' "$2" > "$repo/$1"; }   # write one sample file
w clean.txt "an ordinary line"
w vendor.txt "uses the $v1$v2 api"
w vendor-upper.txt "uses the $(printf '%s' "$v1$v2" | tr '[:lower:]' '[:upper:]') api"
w sk.txt "KEY=sk-$(printf 'abcdefghij%.0s' 1 2 3)"
w sk-short.txt "sk-abc is too short, and sk-$(printf 'a%.0s' $(seq 1 19)) is 19 characters"
w ghp.txt "t=ghp_$(printf 'A1b2C3d4E5%.0s' 1 2 3 4)"
w gho.txt "t=gho_$(printf 'A1b2C3d4E5%.0s' 1 2 3 4)"
w ghs.txt "t=ghs_$(printf 'A1b2C3d4E5%.0s' 1 2 3 4)"
w ghpat.txt "t=github_pat_$(printf 'A1b2C3d4E5%.0s' 1 2 3 4)"
w akia.txt "id AKIA$(printf 'ABCDEFGHIJKLMNOP')"
w pem.txt "-----BEGIN $(printf 'RSA') PRIVATE KEY-----"
w pem2.txt "$(printf -- '-----%s' 'BEGIN') PRIVATE KEY-----"
w pubkey.txt "-----BEGIN PUBLIC KEY-----"
w slack.txt "xoxb-$(printf '1234567890-abcdefghij')"
w jwt.txt "jwt eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.$(printf 'abcdefghij1234567890')"
w bearer.txt "Authorization: Bearer $(printf 'abcdefghij%.0s' 1 2 3)"
w bearer-short.txt "Authorization: Bearer abc"
w akid.txt "R2_ACCESS_KEY_ID=$(printf 'abcdefgh1234')"
w sak.txt "AWS_SECRET_ACCESS_KEY: $(printf 'abcdefgh1234')"
w apitok.txt "CLOUDFLARE_API_TOKEN = $(printf 'abcdefgh1234')"
w apikey.txt "SOME_API_KEY=$(printf 'abcdefgh1234')"
w akid-empty.txt "AWS_ACCESS_KEY_ID="
w akid-var.txt "AWS_ACCESS_KEY_ID=\${R2_ID}"
w old.txt "cd ~/work/$o1$o2/lift"
w hex32.txt "id $(printf '0123456789abcdef%.0s' 1 2)"
w hex31.txt "id $(printf '0123456789abcdef%.0s' 1 2 | cut -c1-31)"
w hex33.txt "id $(printf '0123456789abcdef%.0s' 1 2 3 | cut -c1-33)"
w r2.txt "https://bucket.r2.$(printf 'cloudflare')storage.com/x"
w workers.txt "https://app.example.$(printf 'workers').dev/"
w github-ok.txt "see https://github.com/signature-studio/signature-studio and https://example.invalid/x"
mkdir -p "$repo/dir-$v1$v2"; w "dir-$v1$v2/inside.txt" "harmless"
mkdir -p "$repo/sub"; w sub/untracked.txt "untracked $v1$v2"
head -c 96000000 /dev/zero > "$repo/big.bin"
head -c 94000000 /dev/zero > "$repo/ok-size.bin"
git -C "$repo" add -A > /dev/null 2>&1
w sub/after-add.txt "created after git add: $v1$v2"

out=$(scan); rc=$?
expect_eq "full scan exits 1 when there are findings" 1 "$rc"
hit() { grep -qE "^$1	$2(:|$)" <<< "$out"; }
expect_hit() { if hit "$1" "$2"; then pass "$1 fires: $2"; else fail "$1 does not fire: $2"; fi; }
expect_clean() { if grep -qE "	$1(:|$)" <<< "$out"; then fail "false positive: $1 -> $(grep -E "	$1(:|$)" <<< "$out" | head -1)"; else pass "no finding: $1"; fi; }
for f in vendor.txt vendor-upper.txt "dir-$v1$v2/inside.txt" sub/untracked.txt sub/after-add.txt; do expect_hit vendor "$f"; done
for f in sk.txt ghp.txt gho.txt ghs.txt ghpat.txt akia.txt pem.txt pem2.txt slack.txt jwt.txt bearer.txt akid.txt sak.txt apitok.txt apikey.txt; do expect_hit secret "$f"; done
expect_hit oldpath old.txt
expect_hit cfhex hex32.txt
expect_hit cfhost r2.txt
expect_hit cfhost workers.txt
expect_hit size big.bin
for f in clean.txt sk-short.txt pubkey.txt bearer-short.txt akid-empty.txt akid-var.txt hex31.txt hex33.txt github-ok.txt ok-size.bin; do expect_clean "$f"; done
expect_clean "scripts/scan-forbidden.sh"
expect_clean "scripts/preflight.sh"
expect_clean "scripts/audit-session.sh"
expect_clean ".gitignore"

# the output never shows a value
leak=0
for needle in "sk-$(printf 'abcdefghij')" "ghp_$(printf 'A1b2')" "AKIA$(printf 'ABCDEFGH')" "eyJh$(printf 'bGci')" "BEG$(printf 'IN')" "abcdefgh$(printf '1234')" "$(printf '0123456789')abcdef" "bucket.r2" "Bearer " "uses the" "untracked $v1"; do
  if grep -qF -- "$needle" <<< "$out"; then leak=1; fail "the output shows a matched value ('$needle')"; fi
done
[ "$leak" -eq 0 ] && pass "the output names rule, file and line only (no matched value; a file name may contain the word, it is not a value)"
expect_contains "the summary says that values are not shown" "$out" "Values are not shown"

# line numbers: a finding on line 3
printf 'a\nb\nuses %s\n' "$v1$v2" > "$repo/lines.txt"
expect_contains "the line number is reported" "$(scan lines.txt)" "vendor	lines.txt:3"

# explicit paths
clean_out=$(scan clean.txt); rc=$?
expect_eq "explicit clean path exits 0" 0 "$rc"
expect_contains "explicit clean path says 0 findings" "$clean_out" "0 findings in 1 file"
scan vendor.txt sk.txt > /dev/null; expect_eq "explicit dirty paths exit 1" 1 "$?"
scan nope.txt > /dev/null; expect_eq "a missing path exits 2 (not 0)" 2 "$?"
dir_out=$(scan sub); expect_contains "a directory argument is walked" "$dir_out" "sub/untracked.txt"
(cd "$repo/sub" && bash ../scripts/scan-forbidden.sh > /dev/null 2>&1); expect_eq "without arguments it scans the whole repository from a subdirectory" 1 "$?"

# allowlist: a line-level exception, a file-level exception, other lines in the same file still caught
w mixed.txt "line one has hex $(printf '0123456789abcdef%.0s' 1 2)"
printf 'second line has hex %s\n' "$(printf 'fedcba9876543210%.0s' 1 2)" >> "$repo/mixed.txt"
printf 'cfhex|mixed.txt|%s\n' "$(printf '0123456789abcdef%.0s' 1 2)" >> "$repo/scripts/forbidden-allowlist.txt"
out=$(scan mixed.txt)
expect_not_contains "allowlist: the listed line is allowed" "$out" "cfhex	mixed.txt:1"
expect_contains "allowlist: another line of the same file is still found" "$out" "cfhex	mixed.txt:2"
printf 'vendor|vendor.txt|\n' >> "$repo/scripts/forbidden-allowlist.txt"
expect_not_contains "allowlist: an empty text allows the rule for the whole file" "$(scan vendor.txt)" "vendor	vendor.txt"
printf '# a comment\n\nsecret|sk.txt|%s\n' "sk-$(printf 'abcdefghij')" >> "$repo/scripts/forbidden-allowlist.txt"
expect_not_contains "allowlist: comments and empty lines are ignored; a text entry allows its line" "$(scan sk.txt)" "secret	sk.txt"
expect_contains "allowlist: an entry for another rule does not allow this rule" "$(scan old.txt)" "oldpath	old.txt"

# allowlist, shipped list: a new file is still scanned. The sandbox holds a copy of the real list (plus the test entries added above, which name other files);
# the paths below are in no entry and under no prefix entry.
h32=$(printf '0123456789abcdef%.0s' 1 2)
for d in lift/crates/fe-lift/tests evidence/windows-replay-new tools/win-replay ci archive/other zz-new; do mkdir -p "$repo/$d"; done
for f in lift/crates/fe-lift/tests/zz_new_file.rs evidence/windows-replay-new/x.tsv tools/win-replay/zz_new.tsv ci/zz_new.lock archive/other/x.txt zz-new/x.txt zz_root.txt; do w "$f" "id $h32"; done
out=$(scan lift evidence tools ci archive zz-new zz_root.txt)
for f in lift/crates/fe-lift/tests/zz_new_file.rs evidence/windows-replay-new/x.tsv tools/win-replay/zz_new.tsv ci/zz_new.lock archive/other/x.txt zz-new/x.txt zz_root.txt; do
  expect_contains "shipped allowlist: a new file with 32 hex digits is found ($f)" "$out" "cfhex	$f:1"
done
bad_entries=$(awk -F'|' '!/^#/ && NF > 0 && !(NF == 3 && $1 ~ /^(vendor|secret|size|oldpath|cfhex|cfhost)$/ && $2 != "" && $2 != "/" && $2 !~ /^\// && $2 !~ /\.\./)' "$ROOT/scripts/forbidden-allowlist.txt" | wc -l)
expect_eq "shipped allowlist: every entry has 3 fields, a known rule name and a relative path (no absolute path, no .., no bare /)" 0 "$bad_entries"

# allowlist: a path ending in "/" is a directory prefix
mkdir -p "$repo/pfx/deep/er" "$repo/pfx-sibling" "$repo/tpfx"
w pfx/a.txt "id $h32"
w pfx/deep/er/b.txt "id $h32"
w pfx/v.txt "uses the $v1$v2 api"
w pfx-sibling/c.txt "id $h32"
w pfx.txt "id $h32"
w tpfx/t.txt "first $h32"
printf 'second %s\n' "$(printf 'fedcba9876543210%.0s' 1 2)" >> "$repo/tpfx/t.txt"
printf 'cfhex|pfx/|\ncfhex|tpfx/|first %s\ncfhex|/|\n' "$h32" >> "$repo/scripts/forbidden-allowlist.txt"
out=$(scan pfx pfx-sibling pfx.txt tpfx)
expect_not_contains "prefix: a file directly under the prefix is allowed" "$out" "cfhex	pfx/a.txt"
expect_not_contains "prefix: a file in a nested directory under the prefix is allowed" "$out" "cfhex	pfx/deep/er/b.txt"
expect_contains "prefix: another rule in a file under the prefix is still found" "$out" "vendor	pfx/v.txt:1"
expect_contains "prefix: a sibling directory that only starts with the same letters is found" "$out" "cfhex	pfx-sibling/c.txt:1"
expect_contains "prefix: a file named like the prefix without the slash is found" "$out" "cfhex	pfx.txt:1"
expect_contains "prefix: an entry of '/' alone allows nothing, even for a scan given an absolute path" "$(scan "$repo/zz-new")" "cfhex	$repo/zz-new/x.txt:1"
expect_not_contains "prefix with a text: the listed line under the prefix is allowed" "$out" "cfhex	tpfx/t.txt:1"
expect_contains "prefix with a text: another line under the prefix is found" "$out" "cfhex	tpfx/t.txt:2"

# a repository with no findings
clean_repo="$sb/clean-repo"; make_repo_copy "$clean_repo"
printf 'hello\n' > "$clean_repo/a.txt"; git -C "$clean_repo" add -A > /dev/null 2>&1
out=$(cd "$clean_repo" && bash scripts/scan-forbidden.sh 2>&1); rc=$?
expect_eq "a clean repository exits 0 (the harness files themselves are clean)" 0 "$rc"
expect_contains "and says so" "$out" "0 findings"

# the real harness files are clean (this repository, read-only: no git command is run in it; files are given explicitly)
real=$(cd "$ROOT" && bash scripts/scan-forbidden.sh opencode.jsonc .gitignore .opencode .githooks scripts tests 2>&1); rc=$?
expect_eq "the harness files of this repository are clean" 0 "$rc"
[ "$rc" -eq 0 ] || echo "$real" | head -5

# the scan script contains neither the vendor name nor the old tree's name in one piece
if grep -qiE "$v1$v2|$o1$o2" "$ROOT/scripts/scan-forbidden.sh"; then fail "scan-forbidden.sh contains a forbidden word in one piece"; else pass "scan-forbidden.sh builds the forbidden words from halves"; fi

finish
