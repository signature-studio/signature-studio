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
bad_entries=$(awk -F'|' '!/^#/ && NF > 0 && !(NF == 3 && $1 ~ /^(vendor|secret|size|oldpath|cfhex|cfhost|term|choice|hostval|byok|trial)$/ && $2 != "" && $2 != "/" && $2 !~ /^\// && $2 !~ /\.\./)' "$ROOT/scripts/forbidden-allowlist.txt" | wc -l)
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
real=$(cd "$ROOT" && bash scripts/scan-forbidden.sh opencode.jsonc .gitignore .opencode/plugins .githooks scripts tests 2>&1); rc=$?
expect_eq "the harness files of this repository are clean" 0 "$rc"
[ "$rc" -eq 0 ] || echo "$real" | head -5

# the scan script contains neither the vendor name nor the old tree's name in one piece
if grep -qiE "$v1$v2|$o1$o2" "$ROOT/scripts/scan-forbidden.sh"; then fail "scan-forbidden.sh contains a forbidden word in one piece"; else pass "scan-forbidden.sh builds the forbidden words from halves"; fi

# term rule (scripts/forbidden-terms.tsv): built from fragments at run time, so this file holds none of the terms as one word
tv() { local s="" p; for p in "$@"; do s+=$p; done; printf '%s' "$s"; }
t_bad=$(tv 作業 木); t_ok=$(tv ワーク ツリー)
mkdir -p "$repo/terms"
printf 'a line with %s here\n' "$t_bad" > "$repo/terms/bad.md"
printf 'a line with %s here\n' "$t_ok" > "$repo/terms/good.md"
printf 'x\n%s\n' "$(tv 試 験)" > "$repo/terms/bad2.txt"
printf 'y\n%s\n' "$(tv 試 験 的)" > "$repo/terms/exempt.txt"
printf 'use %s\n' "$(tv ユ ーザー の 判 断)" > "$repo/terms/user.md"
printf 'use %s\n' "$(tv ユ ーザー)" > "$repo/terms/user-bare.md"
printf 'use %s\n' "$(tv 依 頼 票)" > "$repo/terms/req.md"
printf '%s\n' "$(tv こ の 票)の内容" > "$repo/terms/kono-hyo.md"
printf '%s\n' "$(tv 票 の 末 尾)に貼る" > "$repo/terms/suetsu.md"
printf '%s\n' "$(tv 投 票)と$(tv 伝 票)" > "$repo/terms/tohyo-ok.md"
printf 'a %s line\n' "$(tv 台 帳)" > "$repo/terms/ledger.md"
printf 'pre %s\n' "$(tv 禁止 文字列)" > "$repo/terms/kinshi-ok.md"
printf 'pre %s\n' "$(tv 禁止 文)" > "$repo/terms/kinshi-low.md"
printf '%s\n' "依頼の$(printf '%s' Issue)を出す" > "$repo/terms/sp-bad1.md"
printf '%s\n' "$(printf '%s' Issue)を作る" > "$repo/terms/sp-bad2.md"
printf '%s\n' "依頼の $(printf '%s' Issue) を出す" > "$repo/terms/sp-ok.md"
printf '%s\n' "$(printf '%s' Issue) のとおり" > "$repo/terms/sp-ok2.md"
printf '%s\n' "\`$(printf '%s' Issue)\`を作る" > "$repo/terms/sp-code.md"
git -C "$repo" add -A > /dev/null 2>&1
tout=$(cd "$repo" && bash scripts/scan-forbidden.sh terms 2>&1); trc=$?
expect_eq "term: a file with a forbidden word exits 1" 1 "$trc"
expect_contains "term: the finding names the rule, the file and the line" "$tout" "term	terms/bad.md:1"
expect_contains "term: the output names the replacement word" "$tout" "-> ワークツリー"
expect_contains "term: a fragment-joined word in a second file is found" "$tout" "term	terms/bad2.txt:2"
expect_contains "term: the word with the 的 ending is found under its own rule (replacement 実験的)" "$tout" "-> 実験的"
expect_not_contains "term: the word with the 的 ending is not reported as the test term" "$tout" "T02 -> テスト	terms/exempt.txt"
expect_not_contains "term: the correct word is not a finding" "$tout" "term	terms/good.md"
expect_contains "term: a phrase of the high-confidence list is found (user as the owner, judgement)" "$tout" "term	terms/user.md:1"
expect_not_contains "term: a bare user word is not a finding (it is also the product's user; judged in context)" "$tout" "term	terms/user-bare.md"
expect_contains "term: the Issue-request word is found" "$tout" "term	terms/req.md:1"
expect_not_contains "term: a low-confidence coined word is not banned (only the listed terms are)" "$tout" "term	terms/ledger.md"
expect_not_contains "term: a low-confidence word is not banned (only the listed terms are)" "$tout" "term	terms/kinshi-low.md"
expect_not_contains "term: the forbidden-string word is not the prohibited-text term" "$tout" "term	terms/kinshi-ok.md"
expect_contains "term: the short form of the Issue request (rule T03b) is found" "$tout" "term	terms/kono-hyo.md:1"
expect_contains "term: the short form for the end of the Issue (rule T03c) is found" "$tout" "term	terms/suetsu.md:1"
expect_not_contains "term: 投票 and 伝票 are not findings (the 票 rule is limited to the fixed short forms)" "$tout" "term	terms/tohyo-ok.md"
expect_contains "spacing: a Japanese word directly before Issue is found" "$tout" "spacing	terms/sp-bad1.md:1"
expect_contains "spacing: a Japanese word directly after Issue is found" "$tout" "spacing	terms/sp-bad2.md:1"
expect_not_contains "spacing: 依頼の Issue (with spaces) is not a finding" "$tout" "spacing	terms/sp-ok.md"
expect_not_contains "spacing: Issue with a space to a Japanese word is not a finding" "$tout" "spacing	terms/sp-ok2.md"
expect_not_contains "spacing: a backquoted Issue is not matched (code is left alone)" "$tout" "spacing	terms/sp-code.md"
printf 'term|terms/bad.md|%s\n' "$t_bad" >> "$repo/scripts/forbidden-allowlist.txt"
tout2=$(cd "$repo" && bash scripts/scan-forbidden.sh terms/bad.md 2>&1)
expect_not_contains "term: an allowlist entry (fixed text) allows that line" "$tout2" "term	terms/bad.md:1"
(cd "$repo" && bash scripts/scan-forbidden.sh terms/good.md > /dev/null 2>&1); trc=$?
expect_eq "term: clean files exit 0" 0 "$trc"
printf 'a %s here\n' "$(tv 依 頼 者 の 判 断 が 必 要 で す)" > "$repo/terms/t10a.md"
printf 'a %s here\n' "$(tv 判 断 を 仰 ぎ ま す)" > "$repo/terms/t10b.md"
printf 'a %s here\n' "$(tv 確 認 さ せ て く だ さ い)" > "$repo/terms/t10d.md"
printf 'a %s here\n' "$(tv 許 可 を く だ さ い)" > "$repo/terms/t10e.md"
printf 'a %s here\n' "$(tv ど ち ら に し ま す か)" > "$repo/terms/t10g.md"
printf 'a %s here\n' "$(tv 依 頼 者 に 確 認 し ま す)" > "$repo/terms/t10h.md"
printf 'a %s here\n' "$(tv 依 頼 者 の 判 断 は 必 要 な い)" > "$repo/terms/t10j.md"
printf 'a %s here\n' "$(tv 決 め る ま で 待 つ)" > "$repo/terms/t10k.md"
printf 'a %s here\n' "$(tv 判 断 を 自 分 で 下 す)" > "$repo/terms/t10-ok.md"
git -C "$repo" add -A > /dev/null 2>&1
tout3=$(cd "$repo" && bash scripts/scan-forbidden.sh terms 2>&1)
expect_contains "term T10a: the T10a phrase is found" "$tout3" "term	terms/t10a.md:1"
expect_contains "term T10b: the T10b phrase is found" "$tout3" "term	terms/t10b.md:1"
expect_contains "term T10d: the T10d phrase is found" "$tout3" "term	terms/t10d.md:1"
expect_contains "term T10e: the T10e phrase is found" "$tout3" "term	terms/t10e.md:1"
expect_contains "term T10g: the T10g phrase is found" "$tout3" "term	terms/t10g.md:1"
expect_contains "term T10h: the T10h phrase is found" "$tout3" "term	terms/t10h.md:1"
expect_contains "term T10k: the T10k phrase is found" "$tout3" "term	terms/t10k.md:1"
expect_not_contains "term T10: a sentence without the phrase is not a finding" "$tout3" "terms/t10-ok.md"
printf 'a %s here\n' "$(tv 判 断 が 要 る)" > "$repo/terms/t10l.md"
printf 'a %s here\n' "$(tv 判 断 を 委 ね る)" > "$repo/terms/t10n.md"
printf 'a %s here\n' "$(tv 判 断 に 任 せ る)" > "$repo/terms/t10o.md"
printf 'a %s here\n' "$(tv P M) $(tv の)$(tv 判 断)" > "$repo/terms/t10p.md"
printf 'a %s here\n' "$(tv 依 頼 者 に 質 問)" > "$repo/terms/t10r.md"
printf 'a %s here\n' "$(tv 許 可 を 求 め る)" > "$repo/terms/t10s.md"
printf 'a %s here\n' "$(tv 許 可 が 必 要)" > "$repo/terms/t10t.md"
printf 'a %s here\n' "$(tv 承 認 が 必 要)" > "$repo/terms/t10u.md"
printf 'a %s here\n' "$(tv ご 確 認)" > "$repo/terms/t10v.md"
printf 'a %s here\n' "$(tv 依 頼 者 の 許 可)" > "$repo/terms/t10w.md"
printf 'a %s here\n' "$(tv 依 頼 者 に 判 断)" > "$repo/terms/t10y.md"
printf 'a %s here\n' "$(tv 確 認 し て く だ さ い)" > "$repo/terms/nonterm-confirm.md"
printf 'a %s here\n' "$(tv 確 か め て く だ さ い)" > "$repo/terms/nonterm-verify.md"
git -C "$repo" add -A > /dev/null 2>&1
tout4=$(cd "$repo" && bash scripts/scan-forbidden.sh terms 2>&1)
for id in t10l t10n t10o t10p t10r t10s t10t t10u t10v t10w t10y; do expect_contains "term $id: the phrase is found" "$tout4" "term	terms/$id.md:1"; done
expect_not_contains "term: a plain 'please confirm' with a different word is not a finding" "$tout4" "terms/nonterm-confirm.md"
expect_not_contains "term: 'please verify yourself' is not a finding" "$tout4" "terms/nonterm-verify.md"
expect_contains "term T10j: the T10j phrase is found" "$tout3" "term	terms/t10j.md:1"
for w in "$(tv 作業 木)" "$(tv 試 験)" "$(tv 依 頼 票)" "$(tv ユ ーザー)"; do
  if grep -qF -- "$w" "$ROOT/tests/harness/t04_scan.sh"; then fail "t04_scan.sh contains a term in one piece"; fi
done
pass "t04_scan.sh builds the terms from pieces"

# request checks: choice (the owner is asked to choose, decide or fill in; internal words), byok (model credentials), hostval ("pattern" host values)
mkdir -p "$repo/docs/requests" "$repo/docs/ops" "$repo/app/cfg"
cw() { printf '%s\n' "$2" > "$repo/$1"; }
has() { grep -qE "^$1$(printf '\t')$2(:|$)" <<< "$3"; }
want() { if has "$1" "$2" "$3"; then pass "$4"; else fail "$4"; fi; }
nowant() { if has "$1" "$2" "$3"; then fail "$4"; else pass "$4"; fi; }
cw docs/requests/zz-choice-decide.md "依頼者が選んで実行する"
cw docs/requests/zz-choice-fill.md "<値>は依頼者が埋める"
cw docs/requests/zz-choice-way.md "どちらかの方式で作る"
cw docs/requests/zz-choice-rc.md "rc=1 で止まる"
cw docs/requests/zz-choice-cmd.md "確かめるには gh secret list を使う"
cw docs/requests/zz-choice-cmd-ok.md "エージェントは gh secret を実行しない。"
cw docs/requests/zz-choice-cmd-write.md "エージェントは gh secret を書かない。"
cw docs/requests/zz-choice-ok.md "依頼者にしてほしい操作: 読み取りのみ。依頼者の権限は変えない。"
cw docs/requests/TEMPLATE.md "依頼者が選ぶ。Bedrock の API キー。"
cw docs/ops/zz-choice-outside.md "依頼者が選んで実行する"
cw docs/requests/zz-byok-hit.md "Bedrock の API キーを作る"
cw docs/requests/zz-byok-token.md "AWS_BEARER_TOKEN を登録する"
cw docs/requests/zz-byok-profile.md "推論プロファイルの ARN を入れる"
cw docs/requests/zz-byok-cf.md "Cloudflare の API キーを登録する"
cw docs/requests/zz-byok-r2.md "R2 の API キーを作る"
cw docs/requests/zz-byok-iam.md "IAM で利用者を 1 つ作る"
cw docs/requests/zz-byok-key.md "アクセスキーを作り、登録する"
cw docs/requests/zz-byok-aws-name.md "Secret AWS_REGION を登録する"
cw docs/requests/zz-byok-generic.md "モデルの認証は、依頼者が console に登録した BYOK の API キーを使う。"
cw docs/requests/zz-byok-bedrock-plain.md "Bedrock の推論を呼ぶ"
cw docs/requests/zz-byok-cf-iam.md "Cloudflare の IAM の設定を確かめる"
cw docs/requests/zz-byok-prohibit.md "AWS の IAM は使わない。"
printf '%s\n' "# Issue zz: 取り下げ記録" "" "状態: 取り下げ(テスト)" "" "Bedrock の API キーを作る" > "$repo/docs/requests/zz-byok-withdrawn.md"
printf '%s\n' "# Issue zz" "" "a" "b" "c" "状態: 取り下げ(テスト)" "Bedrock の API キーを作る" > "$repo/docs/requests/zz-byok-late.md"
hv="chk-$(printf zone).net"
printf '{"routes": [{"pattern": "%s"}]}\n' "$hv" > "$repo/app/cfg/zz-host.jsonc"
printf '{"pattern": "%s"}\n' "fl.example.net" > "$repo/app/cfg/zz-example.jsonc"
printf '{"pattern": "%s"}\n' "<team>.chk-zone.net" > "$repo/app/cfg/zz-placeholder.jsonc"
printf '{"pattern": "%s"}\n' "your-domain.net" > "$repo/app/cfg/zz-your-domain.jsonc"
printf '%s\n' '"pattern": "[A-Za-z0-9.-]+\.[A-Za-z]{2,}"' > "$repo/app/cfg/zz-regex.txt"
ctout=$(scan docs/requests/zz-choice-decide.md docs/requests/zz-choice-fill.md docs/requests/zz-choice-way.md docs/requests/zz-choice-rc.md docs/requests/zz-choice-cmd.md docs/requests/zz-choice-cmd-ok.md docs/requests/zz-choice-cmd-write.md docs/requests/zz-choice-ok.md docs/requests/TEMPLATE.md docs/ops/zz-choice-outside.md)
want choice docs/requests/zz-choice-decide.md "$ctout" "choice: 依頼者が選んで is found in a request"
want choice docs/requests/zz-choice-fill.md "$ctout" "choice: 依頼者が埋める is found in a request"
want choice docs/requests/zz-choice-way.md "$ctout" "choice: どちらかの方式 is found in a request"
want choice docs/requests/zz-choice-rc.md "$ctout" "choice: the internal word rc= is found in a request"
want choice docs/requests/zz-choice-cmd.md "$ctout" "choice: the internal word gh secret is found in a request (no prohibition on the line)"
nowant choice docs/requests/zz-choice-cmd-ok.md "$ctout" "choice: a prohibition line with gh secret is not a finding"
nowant choice docs/requests/zz-choice-cmd-write.md "$ctout" "choice: a line that says the agent does not write gh secret is not a finding"
nowant choice docs/requests/zz-choice-ok.md "$ctout" "choice: an ordinary request line is not a finding"
nowant choice docs/requests/TEMPLATE.md "$ctout" "choice: the template is not checked"
nowant choice docs/ops/zz-choice-outside.md "$ctout" "choice: the rule covers docs/requests only"
btout=$(scan docs/requests/zz-byok-hit.md docs/requests/zz-byok-token.md docs/requests/zz-byok-profile.md docs/requests/zz-byok-cf.md docs/requests/zz-byok-r2.md docs/requests/zz-byok-iam.md docs/requests/zz-byok-key.md docs/requests/zz-byok-aws-name.md docs/requests/zz-byok-generic.md docs/requests/zz-byok-bedrock-plain.md docs/requests/zz-byok-cf-iam.md docs/requests/zz-byok-prohibit.md docs/requests/zz-byok-withdrawn.md docs/requests/zz-byok-late.md docs/requests/TEMPLATE.md)
want byok docs/requests/zz-byok-hit.md "$btout" "byok: a Bedrock API key in a request is found"
want byok docs/requests/zz-byok-iam.md "$btout" "byok: IAM in a request is found"
want byok docs/requests/zz-byok-key.md "$btout" "byok: an access key in a request is found"
want byok docs/requests/zz-byok-aws-name.md "$btout" "byok: an AWS_ name in a request is found"
nowant byok docs/requests/zz-byok-generic.md "$btout" "byok: a general API key or a model credential is not a finding (the console BYOK is asked for)"
nowant byok docs/requests/zz-byok-bedrock-plain.md "$btout" "byok: Bedrock alone is not a finding"
nowant byok docs/requests/zz-byok-cf-iam.md "$btout" "byok: a Cloudflare IAM line is not checked"
nowant byok docs/requests/zz-byok-prohibit.md "$btout" "byok: a prohibition line (the agent does not use IAM) is not a finding"

# trial: the "自分で試した記録" section of a live request shows a help output, or cites a path or a URL
printf '%s\n' '# Issue zz' '' '## 自分で試した記録' '$ opencode --help' 'usage: opencode [options]' > "$repo/docs/requests/zz-trial-help.md"
printf '%s\n' '# Issue zz' '' '## 自分で試した記録' "\`docs/ops/environment.md\` の確認を読んだ。" > "$repo/docs/requests/zz-trial-cite.md"
printf '%s\n' '# Issue zz' '' '## 自分で試した記録' 'https://example.com/doc を開いた。' > "$repo/docs/requests/zz-trial-url.md"
printf '%s\n' '# Issue zz' '' '## 自分で試した記録' '実行した。結果は良かった。' > "$repo/docs/requests/zz-trial-none.md"
printf '%s\n' '# Issue zz' '' '## 自分で試した記録' '' '## 完了の確認コマンド' 'x' > "$repo/docs/requests/zz-trial-empty.md"
printf '%s\n' '# Issue zz' '' '## 依頼 ID・日付' '0005 / 2026-11-16' > "$repo/docs/requests/zz-trial-missing.md"
printf '%s\n' '# Issue zz' '状態: 取り下げ(テスト)' '' '## 依頼 ID・日付' > "$repo/docs/requests/zz-trial-withdrawn.md"
printf '%s\n' '# Issue zz' '状態: 完了(テスト)' '' '## 依頼 ID・日付' > "$repo/docs/requests/zz-trial-done.md"
tr_out=$(scan docs/requests/zz-trial-help.md docs/requests/zz-trial-cite.md docs/requests/zz-trial-url.md docs/requests/zz-trial-none.md docs/requests/zz-trial-empty.md docs/requests/zz-trial-missing.md docs/requests/zz-trial-withdrawn.md docs/requests/zz-trial-done.md docs/requests/TEMPLATE.md)
want trial docs/requests/zz-trial-missing.md "$tr_out" "trial: a request without the section is a finding"
want trial docs/requests/zz-trial-none.md "$tr_out" "trial: a section without a help output, a path or a URL is a finding"
want trial docs/requests/zz-trial-empty.md "$tr_out" "trial: an empty section is a finding"
nowant trial docs/requests/zz-trial-help.md "$tr_out" "trial: a help output in the section is not a finding"
nowant trial docs/requests/zz-trial-cite.md "$tr_out" "trial: a path in the section is not a finding"
nowant trial docs/requests/zz-trial-url.md "$tr_out" "trial: a URL in the section is not a finding"
nowant trial docs/requests/zz-trial-withdrawn.md "$tr_out" "trial: a withdrawn record is not checked"
nowant trial docs/requests/zz-trial-done.md "$tr_out" "trial: a completed record is not checked"
nowant trial docs/requests/TEMPLATE.md "$tr_out" "trial: the template is not checked"
want byok docs/requests/zz-byok-token.md "$btout" "byok: AWS_BEARER_TOKEN is found"
want byok docs/requests/zz-byok-profile.md "$btout" "byok: an inference profile is found"
nowant byok docs/requests/zz-byok-cf.md "$btout" "byok: a Cloudflare line is not checked"
nowant byok docs/requests/zz-byok-r2.md "$btout" "byok: an R2 line is not checked"
nowant byok docs/requests/zz-byok-withdrawn.md "$btout" "byok: a withdrawn record (status in the first 5 lines) is not checked"
want byok docs/requests/zz-byok-late.md "$btout" "byok: a status line after line 5 does not exempt the file"
nowant byok docs/requests/TEMPLATE.md "$btout" "byok: the template is not checked"
htout=$(scan app/cfg)
want hostval app/cfg/zz-host.jsonc "$htout" "hostval: a dotted host name in a pattern value is found"
nowant hostval app/cfg/zz-example.jsonc "$htout" "hostval: an example host is not a finding"
nowant hostval app/cfg/zz-placeholder.jsonc "$htout" "hostval: a placeholder host is not a finding"
nowant hostval app/cfg/zz-your-domain.jsonc "$htout" "hostval: a your-domain host is not a finding"
nowant hostval app/cfg/zz-regex.txt "$htout" "hostval: a regular expression in a pattern value is not a finding"
if grep -qF "$hv" <<< "$htout$btout$ctout"; then fail "the request checks show a matched value"; else pass "the request checks show no matched value (file and line only)"; fi

finish
