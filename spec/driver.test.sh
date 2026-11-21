#!/usr/bin/env bash
# spec/driver.test.sh : 駆動(driver/)のテスト。gh は偽物(呼び出しを記録し、待ち行列をファイルで持つ)に替える。判定の起動確認の道具(tools/boot-check/)も確かめる。
# 一時ディレクトリはリポジトリの中の .tmp-spec.*(git の無視の対象)に作り、終わったら消す。
# git の commit を使うので、環境の名義で commit する(名義を上書きしない)。環境に名義が無ければ skip と出す。
here=$(cd "$(dirname "$0")" && pwd); top=$here/..; fail=0; T=$(mktemp -d "$top/.tmp-spec.XXXXXX")
ok() { if [ "$2" = "$3" ]; then echo "ok   $1"; else echo "FAIL $1: expected $3, got $2"; fail=1; : > "$T/failed"; fi; }
rc() { "$@" >/dev/null 2>&1; echo $?; }
if ! git var GIT_AUTHOR_IDENT >/dev/null 2>&1; then echo "FAIL driver: 環境に git の名義が無い(判定の spec は、名義が無いと検査を行わない。skip して通さない)"; rm -rf "$T"; exit 1; fi
# 小さなリポジトリ: 関数の表 4 行(0xa1 は直接 wasm 済み)、型、閾値 3、ci のタスク 1 件
R=$T/repo; mkdir -p "$R/queue/inputs" "$R/queue/kinds" "$R/ci/tasks" "$R/lift/harness/hyb/lists"
cp -r "$top/driver" "$R/"; cp "$top/queue/kinds/lower.md" "$R/queue/kinds/"; printf '3\n' > "$R/queue/threshold"
printf 'va\tmodule\tinstructions\n0xa1\tfle\t40\n0xb2\tfle\t30\n0xc3\tdsp\t20\n0xd4\tfle\t10\n0xe5\tfle\t5\n' > "$R/queue/inputs/lift-function-classes.tsv"
printf '0xa1\n' > "$R/lift/harness/hyb/lowered-all.txt"; printf '手で書いたタスク\n' > "$R/ci/tasks/hand.md"
# atomic の表(本物と同じ形: 注釈の行、見出しの行)。0X00C3 は表の書式が違っても飛ばすことを確かめるための行
mkdir -p "$R/lift/harness/seeds"; printf '# 注釈\nva\tsites\tmnemonics\n0X00C3\t2\tCmpxchg:2\n' > "$R/lift/harness/seeds/mt4-atomic-functions.tsv"
git -C "$R" init -q . && git -C "$R" add -A && git -C "$R" commit -q -m base
# task.sh
( cd "$R" && ok "task: ci/tasks のタスクを出す" "$(bash driver/task.sh hand)" "手で書いたタスク" )
( cd "$R" && ok "task: 型 lower は、表の module を入れて出す" "$(bash driver/task.sh lower-0xc3 | grep -c 'lowered-dsp.txt')" 2 )
( cd "$R" && ok "task: 型 lower には受け入れコマンドがある" "$(bash driver/task.sh lower-0xc3 | grep -c '^accept: ')" 1 )
( cd "$R" && ok "task: 表に無い VA は落ちる" "$(rc bash driver/task.sh lower-0xff)" 1 )
( cd "$R" && ok "task: 不正な名前は落ちる" "$(rc bash driver/task.sh ../x)" 1 )
# generate.sh
( cd "$R" && ok "generate: 直接 wasm 済み・除く名前・atomic を飛ばし、表の順に出す" "$(printf 'lower-0xb2\n' | bash driver/generate.sh 5 | tr '\n' ' ')" "lower-0xd4 lower-0xe5 " )
( cd "$R" && ok "generate: atomic の関数(表の書式が違っても)は生成されない" "$(bash driver/generate.sh 5 < /dev/null | grep -c 'lower-0xc3')" 0 )
( cd "$R" && ok "generate: 件数で止まる" "$(bash driver/generate.sh 1 < /dev/null)" "lower-0xb2" )
# 偽の gh: 待ち行列は $Q/ready・$Q/running(番号<TAB>題名)、落ちた回数は $Q/fails、Issue ごとのコメントは $Q/comments-<番号>.json(--jq は実際の jq で通す)
# 報告の成果物は、run download が(落ちた判定の出力として)judge.txt を作る。コメントは $Q/comment-<番号>.txt に残す
Q=$T/q; mkdir -p "$Q" "$T/bin"
cat > "$T/bin/gh" <<'EOF'
#!/usr/bin/env bash
echo "$*" >> "$Q/log"
arg() { local w=$1; shift; while [ $# -gt 0 ]; do if [ "$1" = "$w" ]; then echo "$2"; return; fi; shift; done; }
case "$1 $2" in
  "issue list") for a in "$@"; do case $a in ready|running) cat "$Q/$a" 2>/dev/null;; failed) ;; esac; done ;;
  "issue create") n=$(( $(cat "$Q/n" 2>/dev/null || echo 100) + 1 )); echo "$n" > "$Q/n"; printf '%s\t%s\n' "$n" "$4" >> "$Q/ready" ;;
  "issue view") n=$3
    if [ "$(arg --json "$@")" = title ]; then awk -F'\t' -v n="$n" '$1 == n { print $2 }' "$Q/ready" "$Q/running" 2>/dev/null | head -1
    elif [ -f "$Q/comments-$n.json" ]; then jq -r "$(arg --jq "$@")" "$Q/comments-$n.json"
    elif [ -f "$Q/fails" ]; then cat "$Q/fails"
    else echo '{"comments":[]}' | jq -r "$(arg --jq "$@")"; fi ;;
  "issue comment") printf '%s\n' "$(arg --body "$@")" > "$Q/comment-$3.txt" ;;
  "run download") d=$(arg -D "$@"); if [ -f "$Q/judge.txt" ]; then cp "$Q/judge.txt" "$d/judge.txt"; else printf 'ok\njudge: 受け入れコマンドが失敗した\n' > "$d/judge.txt"; fi ;;
  "run view") echo "判定(テスト)" ;;
esac
EOF
chmod +x "$T/bin/gh"; export Q PATH="$T/bin:$PATH"
reset() { rm -f "$Q"/*; { [ -n "$1" ] && printf '%s\n' "$1"; } > "$Q/ready"; { [ -n "$2" ] && printf '%s\n' "$2"; } > "$Q/running"; }
# fill.sh
reset "" ""
( cd "$R" && bash driver/fill.sh >/dev/null 2>&1 )
ok "fill: 待ちが少なければ、生成器のタスクを Issue にする" "$(grep -c '^issue create' "$Q/log")" 3
ok "fill: 閾値まで起動する(実行中 0、閾値 3)" "$(grep -c '^workflow run agent.yml' "$Q/log")" 3
reset "" "$(printf '1\tlower-0xb2\n2\tlower-0xc3\n')"
( cd "$R" && bash driver/fill.sh >/dev/null 2>&1 )
ok "fill: 不足分だけ起動する(実行中 2、閾値 3)" "$(grep -c '^workflow run agent.yml' "$Q/log")" 1
reset "$(printf '7\thand\n')" ""
( cd "$R" && bash driver/fill.sh >/dev/null 2>&1 )
ok "fill: 受け入れコマンドの無いタスクは起動せず、閉じる" "$(grep -c -- '-f task=hand' "$Q/log")/$(grep -c '^issue close 7' "$Q/log")" 0/1
printf '0\n' > "$R/queue/threshold"; reset "$(printf '8\tlower-0xb2\n')" ""; ( cd "$R" && bash driver/fill.sh >/dev/null 2>&1 )
ok "fill: 閾値が 0 なら、待ちが有っても何も起動しない" "$(grep -c '^workflow run agent.yml' "$Q/log")" 0
printf '3\n' > "$R/queue/threshold"
# 落ちた時の状態(state:)と今の状態が同じ Issue は起動しない。違えば起動する(driver/state.sh)
cur=$(cd "$R" && source driver/state.sh && state)
reset "$(printf '1\tlower-0xb2\n')" ""; jq -n --arg b "$(printf '落ちた: x\nstate: %s' "$cur")" '{comments: [{body: $b}]}' > "$Q/comments-1.json"
( cd "$R" && bash driver/fill.sh > "$T/fill-same.out" 2>&1 )
ok "fill: 落ちた時と状態が同じ Issue は起動しない(飛ばした件数をログに出す)" "$(grep -c -- '-f task=lower-0xb2' "$Q/log")/$(grep -c '状態が同じ.*1 件' "$T/fill-same.out")" 0/1
reset "$(printf '1\tlower-0xb2\n')" ""; jq -n '{comments: [{body: "落ちた: x\nstate: 0000"}]}' > "$Q/comments-1.json"
( cd "$R" && bash driver/fill.sh >/dev/null 2>&1 )
ok "fill: 落ちた時と状態が違えば、同じ Issue を起動する" "$(grep -c -- '-f task=lower-0xb2' "$Q/log")" 1
# 非 lower(survey・計画の課題など)を、lower より先に起動する(台帳 L-036)。どちらも番号の昇順
mkdir -p "$R/queue/tasks"; printf '# survey-x\n\nmode: agent\naccept: true\n' > "$R/queue/tasks/survey-x.md"; printf '# other-y\n\nmode: agent\naccept: true\n' > "$R/queue/tasks/other-y.md"
# 型(mode)の行: script と agent は読める。行の無い物・値が不正な物・本文の無い名前は、起動の検査で落ちる(台帳 L-037)
printf '# m-script\n\nmode: script\naccept: true\n' > "$R/queue/tasks/m-script.md"; printf '# m-agent\n\nmode: agent\naccept: true\n' > "$R/queue/tasks/m-agent.md"
printf '# m-none\n\naccept: true\n' > "$R/queue/tasks/m-none.md"; printf '# m-bad\n\nmode: other\naccept: true\n' > "$R/queue/tasks/m-bad.md"
git -C "$R" add -A && git -C "$R" commit -q -m tasks
started() { grep '^workflow run agent.yml' "$Q/log" | sed 's/.*-f task=\([^ ]*\).*/\1/' | tr '\n' ' '; }
printf '2\n' > "$R/queue/threshold"; reset "$(printf '1\tlower-0xb2\n2\tlower-0xd4\n3\tsurvey-x\n4\tother-y\n')" ""
( cd "$R" && bash driver/fill.sh >/dev/null 2>&1 )
ok "fill: 閾値 2 なら、lower が番号の先でも、非 lower の survey-x と other-y を起動する" "$(started)" "survey-x other-y "
printf '3\n' > "$R/queue/threshold"; reset "$(printf '1\tlower-0xb2\n2\tlower-0xd4\n3\tsurvey-x\n4\tother-y\n')" ""
( cd "$R" && bash driver/fill.sh >/dev/null 2>&1 )
ok "fill: 閾値 3 なら、非 lower の 2 件の後に、lower の番号の一番小さい物" "$(started)" "survey-x other-y lower-0xb2 "
printf '2\n' > "$R/queue/threshold"; reset "$(printf '5\tlower-0xd4\n2\tlower-0xb2\n')" ""
( cd "$R" && bash driver/fill.sh >/dev/null 2>&1 )
ok "fill: 非 lower が無ければ、番号の昇順(従来どおり)" "$(started)" "lower-0xb2 lower-0xd4 "
cur=$(cd "$R" && source driver/state.sh && state)
reset "$(printf '3\tsurvey-x\n4\tother-y\n1\tlower-0xb2\n')" ""; jq -n --arg b "$(printf 'state: %s' "$cur")" '{comments: [{body: $b}]}' > "$Q/comments-3.json"
( cd "$R" && bash driver/fill.sh > "$T/fill-order.out" 2>&1 )
ok "fill: 非 lower で状態が同じ物は飛ばし、その枠は次の候補(other-y、lower-0xb2)に回る" "$(started)" "other-y lower-0xb2 "
ok "fill: 飛ばした件数はログに出る(非 lower の 1 件)" "$(grep -c '状態が同じ.*1 件' "$T/fill-order.out")" 1
# mode.sh: 本文の mode: の値(script・agent)を出す。無い・不正・本文の無い名前は、何も出さずに非 0 で終わる
ok "mode: script のタスクは script を出す" "$(cd "$R" && bash driver/mode.sh m-script)" script
ok "mode: agent のタスクは agent を出す" "$(cd "$R" && bash driver/mode.sh m-agent)" agent
ok "mode: 行の無いタスクは非 0 で終わる" "$(cd "$R" && rc bash driver/mode.sh m-none)" 1
ok "mode: 行の無いタスクは何も出さない" "$(cd "$R" && bash driver/mode.sh m-none 2>/dev/null)" ""
ok "mode: 値が script・agent 以外なら非 0 で終わる" "$(cd "$R" && rc bash driver/mode.sh m-bad)" 1
ok "mode: 本文の無い名前は非 0 で終わる" "$(cd "$R" && rc bash driver/mode.sh no-such-task)" 1
# fill: mode の無いタスクは起動せず閉じる。mode: script と agent は起動する。lower- は古い型の在庫なので、mode の検査を免除する
reset "$(printf '7\tm-none\n')" ""; ( cd "$R" && bash driver/fill.sh >/dev/null 2>&1 )
ok "fill: mode の無いタスクは起動せず、閉じる" "$(grep -c -- '-f task=m-none' "$Q/log")/$(grep -c '^issue close 7' "$Q/log")" 0/1
reset "$(printf '9\tm-script\n')" ""; ( cd "$R" && bash driver/fill.sh >/dev/null 2>&1 )
ok "fill: mode: script のタスクは起動する" "$(grep -c -- '-f task=m-script' "$Q/log")" 1
reset "$(printf '9\tm-agent\n')" ""; ( cd "$R" && bash driver/fill.sh >/dev/null 2>&1 )
ok "fill: mode: agent のタスクは起動する" "$(grep -c -- '-f task=m-agent' "$Q/log")" 1
reset "$(printf '9\tm-bad\n')" ""; ( cd "$R" && bash driver/fill.sh >/dev/null 2>&1 )
ok "fill: mode の値が不正なタスクは起動しない" "$(grep -c -- '-f task=m-bad' "$Q/log")" 0
reset "$(printf '9\tlower-0xd4\n')" ""; ( cd "$R" && bash driver/fill.sh >/dev/null 2>&1 )
ok "fill: lower- の古い型は mode が無くても起動する(A6 で閉じるまでの在庫)" "$(grep -c -- '-f task=lower-0xd4' "$Q/log")" 1
ok "fill: 起動の順は、題名が ^lower で始まる物(lowerb- を含む)を後に並べる" "$(grep -c "\$2 ~ /^lower/" "$R/driver/fill.sh")" 1
# close.sh
reset "" ""; ( cd "$R" && bash driver/close.sh 5 success https://x >/dev/null 2>&1 )
ok "close: 通ったら閉じる" "$(grep -c '^issue close 5 --reason completed' "$Q/log")" 1
reset "" ""; echo 0 > "$Q/fails"; ( cd "$R" && bash driver/close.sh 5 failure https://x >/dev/null 2>&1 )
ok "close: 落ちたら待ちに戻す" "$(grep -c -- '--add-label ready' "$Q/log")" 1
reset "" ""; echo 2 > "$Q/fails"; ( cd "$R" && bash driver/close.sh 5 failure https://x >/dev/null 2>&1 )
ok "close: 3 回落ちたら failed にして閉じる" "$(grep -c -- '--add-label failed' "$Q/log")/$(grep -c '^issue close 5' "$Q/log")" 1/1
reset "" ""; echo 2 > "$Q/fails"; ( cd "$R" && bash driver/close.sh 5 cancelled https://x >/dev/null 2>&1 )
ok "close: 取り消された run は落ちた数に入れず、コメントせず待ちに戻す(2 回の後でも failed にしない)" "$(grep -c -- '--add-label failed' "$Q/log")/$(grep -c -- '--add-label ready' "$Q/log")/$(grep -c '^issue comment' "$Q/log")" 0/1/0
reset "" ""; echo 0 > "$Q/fails"; ( cd "$R" && bash driver/close.sh 5 failure https://x/actions/runs/77 >/dev/null 2>&1 )
ok "close: 落ちた時は、落ちた回数と原因の行(判定の出力)をコメントに残す" "$(grep -c '^落ちた: https://x/actions/runs/77(1 回目' "$Q/comment-5.txt")/$(grep -c 'judge: 受け入れコマンドが失敗した' "$Q/comment-5.txt")" 1/1
ok "close: 落ちた時の main の SHA と状態を、コメントに 1 行ずつ残す" "$(grep -c '^main-sha: [0-9a-f]\{40\}$' "$Q/comment-5.txt")/$(grep -c '^state: [0-9a-f]\{40\}$' "$Q/comment-5.txt")" 1/1
ok "close: 判定の出力を使い終えたら、一時のディレクトリは消す" "$([ -e "$R/.tmp-close-77" ] && echo kept || echo gone)" gone
reset "" ""; echo 0 > "$Q/fails"; printf 'ok\njudge: 起動確認の結果: {"state":"running","error":null,"standIn":false,"elapsedMs":1234}\njudge: 受け入れコマンドが失敗した\n' > "$Q/judge.txt"
( cd "$R" && bash driver/close.sh 5 failure https://x/actions/runs/91 >/dev/null 2>&1 )
ok "close: 判定の末尾の起動確認の行と、失敗の行が、落ちたコメントに残る" "$(grep -c '^judge: 起動確認の結果: {"state":"running"' "$Q/comment-5.txt" 2>/dev/null)/$(grep -c '^judge: 受け入れコマンドが失敗した$' "$Q/comment-5.txt" 2>/dev/null)" 1/1
# 失敗の数え方: 「落ちた:」で始まり、state: の行がある(今の形の)コメントだけを数える。古い形のコメントは数えない
st=0000000000000000000000000000000000000000
reset "" ""; jq -n '{comments: [{body: "落ちた: https://x/old1(1 回目。待ちに戻した)"}, {body: "落ちた: https://x/old2(2 回目。待ちに戻した)"}]}' > "$Q/comments-5.json"
( cd "$R" && bash driver/close.sh 5 failure https://x/actions/runs/88 >/dev/null 2>&1 )
ok "close: 古い形の落ちたが 2 つ有っても、今の形の 1 回目は 1 回目で待ちに戻る" "$(grep -c '^落ちた: https://x/actions/runs/88(1 回目。待ちに戻した)' "$Q/comment-5.txt" 2>/dev/null)/$(grep -c -- '--add-label failed' "$Q/log")" 1/0
reset "" ""; jq -n --arg s "$st" '{comments: [{body: ("落ちた: https://x/n1(1 回目。待ちに戻した)\nmain-sha: a\nstate: " + $s)}, {body: ("落ちた: https://x/n2(2 回目。待ちに戻した)\nmain-sha: a\nstate: " + $s)}]}' > "$Q/comments-5.json"
( cd "$R" && bash driver/close.sh 5 failure https://x/actions/runs/89 >/dev/null 2>&1 )
ok "close: 今の形の落ちたが 2 つ有れば、3 回目で failed にして閉じる" "$(grep -c -- '--add-label failed' "$Q/log")/$(grep -c '^issue close 5' "$Q/log")" 1/1
reset "" ""; jq -n --arg s "$st" '{comments: [{body: "落ちた: https://x/old1(1 回目。待ちに戻した)"}, {body: "落ちた: https://x/old2(2 回目。待ちに戻した)"}, {body: ("落ちた: https://x/n1(1 回目。待ちに戻した)\nmain-sha: a\nstate: " + $s)}]}' > "$Q/comments-5.json"
( cd "$R" && bash driver/close.sh 5 failure https://x/actions/runs/90 >/dev/null 2>&1 )
ok "close: 古い形の 2 つと今の形の 1 つが有れば、今回は 2 回目で待ちに戻る(failed にしない)" "$(grep -c '^落ちた: https://x/actions/runs/90(2 回目。待ちに戻した)' "$Q/comment-5.txt" 2>/dev/null)/$(grep -c -- '--add-label failed' "$Q/log")" 1/0
# state: 作業が読む木(製品・判定・駆動・型)が変わったときだけ変わる(台帳の変更では変わらない)
s1=$(cd "$R" && source driver/state.sh && state)
mkdir -p "$R/ledger"; echo x > "$R/ledger/x.tsv"; git -C "$R" add -A && git -C "$R" commit -q -m ledger
s2=$(cd "$R" && source driver/state.sh && state)
mkdir -p "$R/app"; echo x > "$R/app/a.txt"; git -C "$R" add -A && git -C "$R" commit -q -m app
s3=$(cd "$R" && source driver/state.sh && state)
ok "state: 台帳だけの変更では変わらない" "$([ "$s1" = "$s2" ] && echo same)" same
ok "state: 製品(app/)の変更では変わる" "$([ "$s2" != "$s3" ] && echo changed)" changed
# agent.yml: 判定の出力は報告と同じ成果物に入る。待ち行列の段は、落ちた run の成果物とジョブを読める
A=$top/.github/workflows/agent.yml
ok "agent.yml: 判定の出力を out/judge.txt に残し、成果物(out)に入れる" "$(grep -c 'tee "$RUNNER_TEMP/out/judge.txt"' "$A")/$(grep -c '^ *path: [$]{{ runner.temp }}/out$' "$A")" 1/1
ok "agent.yml: 待ち行列の段(close)は actions を読める" "$(awk '/^  close:/{f=1} /^  fill:/{f=0} f && /actions: read/{print "yes"}' "$A")" yes
# agent.yml の判定の段: 判定(judge.sh)の失敗が、パイプ(| tee)で隠れないこと。shell 未指定の run は bash -e で動き、pipefail は無い
J=$(awk '/^      - name: 判定/{f=1} /^      - name: 通った変更/{f=0} f' "$A")
jl=$(grep -n 'judge.sh" refs/judge/main' <<<"$J" | cut -d: -f1); pl=$(grep -n '^ *set -o pipefail$' <<<"$J" | head -1 | cut -d: -f1)
ok "agent.yml: 判定の段は judge.sh の行より前に set -o pipefail を置く" "$([ -n "$pl" ] && [ -n "$jl" ] && [ "$pl" -lt "$jl" ] && echo yes)" yes
# 同じ形で動かす: 判定の段の judge の行(と pipefail の行)をそのまま使い、judge.sh は落ちる偽物にする
jr=$T/judge-rt; mkdir -p "$jr/guard" "$jr/out"; printf 'echo "judge: 違反がある"\nexit 1\n' > "$jr/guard/judge.sh"
jline=$(sed -n "${jl}p" <<<"$J" | sed 's/^ *//'); pline=""; [ -n "$pl" ] && pline=$(sed -n "${pl}p" <<<"$J" | sed 's/^ *//')
ok "判定の段の形で、judge.sh が落ちれば段も落ちる(パイプで隠れない)" "$(RUNNER_TEMP=$jr bash -e -c "$pline"$'\n'"$jline" >/dev/null 2>&1 && echo passes || echo fails)" fails
ok "対照: pipefail が無い bash -e では、同じ判定の行の失敗は 0 になる(偽物が本物の形を再現すること)" "$(RUNNER_TEMP=$jr bash -e -c "$jline" >/dev/null 2>&1; echo $?)" 0
# push の段は判定の段が成功したときだけ走る: if を持たず、どの段にも continue-on-error が無い
P=$(awk '/^      - name: 通った変更を main に push/{f=1} /^      - name: 報告を検査/{f=0} f' "$A")
ok "agent.yml: push の段は if を持たない(判定が成功したときだけ走る)" "$(grep -c '^ *if:' <<<"$P")" 0
ok "agent.yml: どの段にも continue-on-error が無い(判定の失敗を飛ばさない)" "$(grep -c 'continue-on-error' "$A")" 0
# アカウント ID の値を、ファイルに直書きしていない(一致の件数だけを見る。値は表示しない。L-006)
acct=$(grep '"account_id"' "$top/app/wrangler.jsonc" | cut -d'"' -f4)
ok "agent.yml・deploy.yml・driver/・spec/ に、アカウント ID の値を直書きしていない(一致の数)" "$(if [ -n "$acct" ]; then cat "$A" "$top/.github/workflows/deploy.yml" "$top"/driver/*.sh "$top"/spec/*.sh | grep -cF -- "$acct"; else echo none; fi)" 0
# push の段の再試行(agent.yml の for 〜 done を、実際の URL の代わりに手元の bare リポジトリで動かす)。衝突は中断して、衝突したファイル名だけを出す。拒否されたら次の試行へ
PB=$(awk '/for i in 1 2 3; do/{f=1} f{print} f && /^ *exit 1$/{exit}' "$A" | sed 's#"https://github.com/$GITHUB_REPOSITORY.git"#"$BARE"#')
pr=$T/push; mkdir -p "$pr"; printf '%s\n' "$PB" > "$pr/loop.sh"
mkbase() { git init -q --bare "$pr/$1.git" && git -C "$pr/$1.git" symbolic-ref HEAD refs/heads/main && git init -q "$pr/$1.seed" && git -C "$pr/$1.seed" symbolic-ref HEAD refs/heads/main && printf 'x\n' > "$pr/$1.seed/f" && git -C "$pr/$1.seed" add f && git -C "$pr/$1.seed" commit -q -m base && git -C "$pr/$1.seed" push -q "$pr/$1.git" main && git clone -q "$pr/$1.git" "$pr/$1.work"; }
mkbase c1; git clone -q "$pr/c1.git" "$pr/c1.other"
( cd "$pr/c1.work" && printf 'mine\n' > f && git commit -q -am mine )
( cd "$pr/c1.other" && printf 'theirs\n' > f && git commit -q -am theirs && git push -q origin HEAD:main )
out=$(cd "$pr/c1.work" && BARE="$pr/c1.git" bash -e "$pr/loop.sh" 2>&1); rc=$?
ok "push の段: 衝突したら、試行ごとに衝突したファイル名だけを出し、マージを中断する(未解決の印を残さない)" "$(grep -c '^衝突: f$' <<<"$out")/$(git -C "$pr/c1.work" status --porcelain | grep -c '^UU')" 3/0
ok "push の段: 衝突で 3 回とも失敗したら、非 0 で終わる" "$([ "$rc" -ne 0 ] && echo nonzero)" nonzero
ok "push の段: 2 回目の試行は、未解決の印のせいで落ちない(Pulling is not possible が出ない)" "$(grep -c 'Pulling is not possible' <<<"$out")" 0
mkbase c2
( cd "$pr/c2.work" && printf 'g\n' > g.txt && git add g.txt && git commit -q -m mine2 )
printf '#!/bin/sh\nif [ ! -e "%s" ]; then : > "%s"; echo "reject once" >&2; exit 1; fi\nexit 0\n' "$pr/rejected-once" "$pr/rejected-once" > "$pr/c2.git/hooks/pre-receive"; chmod +x "$pr/c2.git/hooks/pre-receive"
out2=$(cd "$pr/c2.work" && BARE="$pr/c2.git" bash -e "$pr/loop.sh" 2>&1); rc2=$?
ok "push の段: push が拒否されたら、次の試行で成功する(終了コード 0、リモートに手元の commit が入る)" "$rc2/$(git -C "$pr/c2.git" log -1 --format=%s main)" "0/mine2"
# driver/ はリポジトリの外を参照しない(依頼者の指示 2026-11-19)
ok "driver/ が、リポジトリの外を参照しない" "$(grep -nE '\$HOME|~/|systemctl|/\.local/|/\.config/|/tmp|TMPDIR|mktemp' "$top"/driver/*.sh | wc -l)" 0
ok "名義の無い環境では、driver の spec は skip して通らず、落ちる" "$(env -i HOME="$T/nohome" PATH="$PATH" bash "$here/driver.test.sh" >/dev/null 2>&1; echo $?)" 1
ok "判定の spec の中(JUDGE_IN_SPEC が有る)でも、guard の spec は判定の検査を省かずに通る" "$(JUDGE_IN_SPEC=1 bash "$here/guard.test.sh" >/dev/null 2>&1; echo $?)" 0
# agent.yml の作業段: npm ci の直後に app のビルドを実行する(tools/boot-check が app/dist を前提にする)
A=$top/.github/workflows/agent.yml; ci_line=$(grep -n '^ *npm ci --prefix app$' "$A" | cut -d: -f1)
ok "agent.yml: 作業段は npm ci の直後に npm run build --prefix app を実行する" "$(awk -v c="$ci_line" 'NR==c+1' "$A" | sed 's/^ *//')" "npm run build --prefix app"
# 型 lower の判定(boot.json を読む部分)。standIn は true/false で出るので、null と比べると false で落ちてしまう(false==null は偽)
acc=$(grep -m1 '^accept: ' "$top/queue/kinds/lower.md" | cut -c9-); node_cmd=$(grep -o 'node -e .*' <<<"$acc")
boot() { mkdir -p "$T/rt"; printf '%s' "$1" > "$T/rt/boot.json"; RUNNER_TEMP=$T/rt bash -c "$node_cmd" >/dev/null 2>&1; echo $?; }
ok "判定(lower): 正常な起動(engine-idle、error 無し、standIn false)は通る" "$(boot '{"error":null,"state":"engine-idle","standIn":false}')" 0
ok "判定(lower): STAND-IN の画面(standIn true)は落ちる" "$(boot '{"error":null,"state":"engine-idle","standIn":true}')" 1
ok "判定(lower): standIn が無い(読めなかった)のは落ちる" "$(boot '{"error":null,"state":"engine-idle"}')" 1
ok "判定(lower): error が有れば落ちる" "$(boot '{"error":"x","state":"engine-idle","standIn":false}')" 1
ok "判定(lower): engine-idle で止まっていなければ落ちる" "$(boot '{"error":null,"state":"exited","standIn":false}')" 1
ok "判定(lower): 型の受け入れ行は standIn===false を見て、standIn==null を持たない" "$(grep -c 'r.standIn===false' <<<"$acc")/$(grep -c 'standIn==null' <<<"$acc")" 1/0
# 判定(lower)は同じ作業ツリーで 2 回走る(作業者の受け入れ、次に判定)。fe-build engine の clone 先と gen の出力先を、走る前に消す(2 回目も通すため)
rmcmd=$(grep -o 'rm -rf [^&]*' <<<"$acc" | sed 's/ *$//')
ok "判定(lower): fe-build engine の前に、clone 先と gen の出力先を消す" "$([ -n "$rmcmd" ] && case $acc in *"$rmcmd"*"lift/target/release/fe-build engine"*) echo yes;; esac)" yes
ib=$T/idem; mkdir -p "$ib/origin.git"; git init -q --bare "$ib/origin.git"
mkfix() { rm -rf "$ib/work"; mkdir -p "$ib/work/lift/out-app/b2d-src" "$ib/work/lift/out-app/gen/src" "$ib/work/lift/out-app/host"; echo stale > "$ib/work/lift/out-app/b2d-src/old.txt"; echo stale > "$ib/work/lift/out-app/gen/src/fle-tables.txt"; echo keep > "$ib/work/lift/out-app/host/keep.txt"; }
mkfix; ok "判定(lower): 対照: 消さないと、2 回目の git clone は失敗する(この検査が 2 回目の失敗を再現すること)" "$(cd "$ib/work" && git clone -q "$ib/origin.git" lift/out-app/b2d-src >/dev/null 2>&1; echo $?)" 128
mkfix; ok "判定(lower): accept の rm の後は、既に在る clone 先でも git clone できる" "$(cd "$ib/work" && eval "$rmcmd" && git clone -q "$ib/origin.git" lift/out-app/b2d-src >/dev/null 2>&1; echo $?)" 0
mkfix; ok "判定(lower): accept の rm の後は、既に在る gen の出力(排他で作る)を作り直せる" "$(cd "$ib/work" && eval "$rmcmd"; mkdir -p lift/out-app/gen/src && (set -C; : > lift/out-app/gen/src/fle-tables.txt) 2>/dev/null; echo $?)" 0
mkfix; ok "判定(lower): 前の段の入力(lift/out-app/host)は、rm で消えない" "$(cd "$ib/work" && eval "$rmcmd"; test -f lift/out-app/host/keep.txt; echo $?)" 0
# 起動確認の道具(tools/boot-check): raw/<名前> の隣の br/<名前>/ を <ファイル>.br として assets に置くと、app の配信(assets-middleware.ts)が Content-Encoding br で返す
mv=$(node --input-type=module -e '
import fs from "node:fs"; import http from "node:http"; import zlib from "node:zlib"; import path from "node:path"; import { pathToFileURL } from "node:url";
const [T, top] = process.argv.slice(1);
const { stageAssets } = await import(pathToFileURL(path.join(top, "tools/boot-check/stage-assets.mjs")).href);
const { assetsMiddleware } = await import(pathToFileURL(path.join(top, "app/tools/assets-middleware.ts")).href);
const b = path.join(T, "bt"); const raw = path.join(b, "bundle/raw/fl-Oz"), br = path.join(b, "bundle/br/fl-Oz"), assets = path.join(b, "assets");
fs.mkdirSync(raw, { recursive: true }); fs.mkdirSync(br, { recursive: true });
fs.writeFileSync(path.join(raw, "manifest.json"), "{}\n"); fs.writeFileSync(path.join(br, "engine.abc.wasm"), zlib.brotliCompressSync(Buffer.from("WASMDATA")));
stageAssets({ rawDir: raw, brDir: br, assets, bundle: "fl-Oz" });
let threw = "ran"; try { stageAssets({ rawDir: raw, brDir: path.join(b, "none"), assets: path.join(b, "a2"), bundle: "fl-Oz" }); } catch { threw = "threw"; }
const srv = http.createServer(assetsMiddleware(assets)); await new Promise((r) => srv.listen(0, "127.0.0.1", r));
const get = (p, ae) => new Promise((res, rej) => http.get({ host: "127.0.0.1", port: srv.address().port, path: p, headers: ae ? { "accept-encoding": ae } : {} }, (r) => { const c = []; r.on("data", (d) => c.push(d)); r.on("end", () => res({ status: r.statusCode, enc: r.headers["content-encoding"] ?? "", body: Buffer.concat(c) })); }).on("error", rej));
const hit = await get("/assets/fl-Oz/engine.abc.wasm", "br"); const miss = await get("/assets/fl-Oz/engine.abc.wasm", "");
srv.close();
fs.rmSync(assets, { recursive: true, force: true });
console.log([hit.status, hit.enc, zlib.brotliDecompressSync(hit.body).toString(), miss.status, fs.existsSync(path.join(br, "engine.abc.wasm")) ? "kept" : "gone", threw].join(" "));
' "$T" "$top" 2>/dev/null)
ok "boot-check: br の .br は Content-Encoding br で配信され、掃除で元のファイルは消えない。br が無ければ落ちる" "$mv" "200 br WASMDATA 404 kept threw"
# cdp.mjs: DevTools が 12 秒後に答える Chrome でも、起動に失敗しない(起動待ちを時間で切る)
F=$T/fake; mkdir -p "$F/bin" "$F/profile"
cat > "$F/bin/google-chrome" <<'EOF'
#!/usr/bin/env node
const port = Number(process.argv.find((a) => a.startsWith('--remote-debugging-port=')).split('=')[1]);
setTimeout(() => {
  require('node:http').createServer((q, s) => s.end(JSON.stringify({ Browser: 'fake/1', webSocketDebuggerUrl: 'ws://127.0.0.1:1/' }))).listen(port, '127.0.0.1');
}, 12000);
EOF
chmod +x "$F/bin/google-chrome"
slow=$(PATH="$F/bin:$PATH" node --input-type=module -e '
import path from "node:path"; import { pathToFileURL } from "node:url";
const [T, top] = process.argv.slice(1);
const { launchChrome } = await import(pathToFileURL(path.join(top, "tools/boot-check/cdp.mjs")).href);
try { const c = await launchChrome({ profile: path.join(T, "fake/profile") }); console.log(c.version.Browser); c.proc.kill("SIGKILL"); } catch (e) { console.log("threw"); }
' "$T" "$top" 2>/dev/null)
ok "cdp: DevTools が 12 秒後に答える Chrome でも起動できる" "$slow" "fake/1"
# 完成までの計画(台帳 L-033): 待ち行列の survey 6 件、計画の見出し、受け入れの行、台帳の行
Q_TASKS=$top/queue/tasks
ok "queue: survey の本文が 6 件ある" "$(ls "$Q_TASKS"/survey-c*.md 2>/dev/null | wc -l)" 6
ok "queue: 本文のすべてが、accept: の行をちょうど 1 つ持つ" "$(n=0; for f in "$Q_TASKS"/*.md; do [ -e "$f" ] || continue; [ "$(grep -c '^accept: ' "$f")" = 1 ] || n=$((n+1)); done; echo "$n")" 0
ok "plan: C-1 から C-6 の見出しが在る" "$(for n in 1 2 3 4 5 6; do grep -q "^### C-$n " "$top/queue/plan.md" 2>/dev/null && echo x; done | wc -l)" 6
ok "plan: 各 C に対応する survey の本文が在る" "$(for n in 1 2 3 4 5 6; do ls "$Q_TASKS"/survey-c$n-*.md >/dev/null 2>&1 && echo x; done | wc -l)" 6
ok "task.sh: survey の 6 件は終了コード 0 で、accept: の行を含む本文を出す" "$(cd "$top" && for f in "$Q_TASKS"/survey-c*.md; do n=$(basename "$f" .md); out=$(bash driver/task.sh "$n" 2>/dev/null) && grep -q '^accept: ' <<<"$out" && echo x; done | wc -l)" 6
ok "survey: 受け入れの行は、出力の先と findings の 3 つの見出しを検査する(6 件とも)" "$(for f in "$Q_TASKS"/survey-c*.md; do n=$(basename "$f" .md); a=$(grep '^accept: ' "$f"); grep -q "queue/findings/$n.md" <<<"$a" && grep -q '## 実測' <<<"$a" && grep -q '## 未確認' <<<"$a" && grep -q '## 次の手順の案' <<<"$a" && echo x; done | wc -l)" 6
ok "台帳: L-033 が在り、5 列" "$(awk -F'\t' '$1 == "L-033" && NF == 5 { print "yes" }' "$top/ledger/instructions.tsv")" yes
# 内蔵の目標(台帳 L-034): 約 141(効果 約 90 + インストゥルメント 約 51)。共通の前提に inventory。survey-c4 の本文の整え
ok "台帳: L-034 が在り、5 列" "$(awk -F'\t' '$1 == "L-034" && NF == 5 { print "yes" }' "$top/ledger/instructions.tsv")" yes
C4SEC=$(awk '/^### C-4 /{f=1; next} /^### |^## /{f=0} f' "$top/queue/plan.md")
ok "plan: C-4 の節に、目標の約 141 が在る" "$(grep -q '約 141' <<<"$C4SEC" && echo yes || echo no)" yes
ok "plan: C-4 の節に、後回しの VFX 系と OpenGL が在る" "$(grep -q 'VFX' <<<"$C4SEC" && grep -q 'OpenGL' <<<"$C4SEC" && echo yes || echo no)" yes
ok "plan: 共通の前提の節に、インストーラの一覧(inventory)が在る" "$(grep -c '^## 共通の前提.*inventory' "$top/queue/plan.md")" 1
ok "plan: 更新の記録(④)に L-034 の行が在る" "$(awk '/^## ④ 更新の記録/{f=1} f' "$top/queue/plan.md" | grep -c 'L-034')" 1
ok "plan: エンジン本体の wasm の実測(165848536)と出典が、同じ行に在る" "$(grep -q '165848536.*出典' "$top/queue/plan.md" && echo yes || echo no)" yes
# 配信の方針(台帳 L-035・依頼者の提案): 起動前と起動後の区分、1 プラグイン 1 wasm。古い方式の文(survey で決める)は残さない
ok "台帳: L-035 が在り、5 列" "$(awk -F'\t' '$1 == "L-035" && NF == 5 { print "yes" }' "$top/ledger/instructions.tsv")" yes
ok "plan: 配信の方針の節が在り、1 プラグイン 1 wasm・起動前・バックグラウンドの語が在る" "$(P=$top/queue/plan.md; if [ "$(grep -c '^## 配信の方針' "$P")" = 1 ] && grep -q '1 プラグイン 1 wasm' "$P" && grep -q '起動前' "$P" && grep -q 'バックグラウンド' "$P"; then echo yes; else echo no; fi)" yes
ok "plan: 「必要になった時に読み込むか」の方式の文が残っていない" "$(grep -c '必要になった時に読み込むかは survey の結果で決める' "$top/queue/plan.md")" 0
S4=$top/queue/tasks/survey-c4-builtin-plugins.md
ok "survey-c4: accept: の行はちょうど 1 つ" "$(grep -c '^accept: ' "$S4")" 1
ok "survey-c4: 目的に依頼者の申告(約 90・約 51・±2・VFX 後回し)が在る" "$(for w in '約 90' '約 51' '±2' 'VFX'; do grep -q -- "$w" "$S4" || { echo no; exit 0; }; done; echo yes)" yes
ok "survey-c4: 手順の行は、句点か閉じ括弧で終わる(途中で切れていない)" "$(grep -E '^[0-9]+\. ' "$S4" | grep -vcE '(。|）|\))$')" 0
ok "survey-c4: インストゥルメントの件数が 0 件だと、手順 2 に書く" "$(grep -E '^2\. ' "$S4" | grep -c '0 件')" 1
# タスクの粒度(台帳 L-037): 機械的な作業は mode: script(スクリプト)で行い、エージェントは規模が大のタスクだけ
ok "台帳: L-037 が在り、5 列で、日付 2026-11-21 と出所(依頼者)が在る" "$(awk -F'\t' '$1 == "L-037" && NF == 5 && $2 == "2026-11-21" && $3 == "依頼者" { print "yes" }' "$top/ledger/instructions.tsv")" yes
ok "plan: 「タスクの粒度」の節が在り、3 時間と 30 件を書く" "$(P=$top/queue/plan.md; S=$(awk '/^## タスクの粒度/{f=1; next} /^## /{f=0} f' "$P"); if [ "$(grep -c '^## タスクの粒度' "$P")" = 1 ] && grep -q '3 時間' <<<"$S" && grep -q '30 件' <<<"$S"; then echo yes; else echo no; fi)" yes
ok "AGENTS.md: 作業者の手順に、mode: script の行が在る" "$(grep -c '^- タスク本文の `mode: script` は、スクリプトが作業を行う' "$top/AGENTS.md")" 1
# agent.yml(台帳 L-037): 作業の型(MODE)を読み、opencode に関わる段は agent のときだけ動く。script の段は、タスクの run: をスクリプトとして実行する
RUNJ=$(awk '/^  run:$/{f=1} /^  close:$/{f=0} f' "$A")
EXPECT_NAMES='リポジトリを取得する(資格は残さない)|作業の型を読む(script はスクリプト、agent は作業者)|名義を設定する(token は消す)|版の値を ci/versions.env から読む|Node を入れる|rust をそろえ、app の依存を入れてビルドし、fe-build を用意する|emsdk を入れる|アカウント ID を app/wrangler.jsonc から読む|抽出物を R2 のキャッシュから取る(公開の artifact は使わない)|抽出物を lift/out-app/host に置く|opencode を導入する(版は固定)|opencode の認証情報を取り込み、作業者のモデルを設定する|作業者を 1 回動かす|スクリプトで作業する(script。エージェントは動かさない)|判定(取り直した main の guard/judge.sh と、main のタスクで)|通った変更を main に push する(token はこの段で発行する)|報告を検査して、成果物とサマリーに出す|報告と判定の出力を成果物に保存する(7 日)'
ok "agent.yml: run ジョブの段の名前と順が、型の 2 段を足した物と一致する" "$(awk '/^      - name: /{ sub(/^      - name: /, ""); print }' <<<"$RUNJ" | paste -sd'|')" "$EXPECT_NAMES"
ok "agent.yml: opencode の 3 段は、if: env.MODE == 'agent' を名前の次の行に持つ" "$(awk '/^      - name: /{ name = $0; if ((getline nxt) > 0 && nxt == "        if: env.MODE == '\''agent'\''") print name }' <<<"$RUNJ" | grep -c 'opencode を導入する\|opencode の認証情報\|作業者を 1 回動かす')" 3
ok "agent.yml: if: env.MODE == 'agent' は、ちょうど 3 つ(opencode の段だけ)" "$(grep -cF -- "        if: env.MODE == 'agent'" <<<"$RUNJ")" 3
ok "agent.yml: opencode の段の主要な行が、変更前のまま在る(認証・導入・実行)" "$(for l in 'install -D -m 755 "$RUNNER_TEMP/package/bin/opencode" "$HOME/.opencode/bin/opencode"' 'printf '\''%s'\'' "$OPENCODE_AUTH_JSON" | opencode auth import --standalone' 'opencode run --agent worker "$(cat "$RUNNER_TEMP/prompt.md")"'; do grep -cF -- "$l" <<<"$RUNJ"; done | paste -sd' ')" "1 1 1"
MODESTEP=$(awk '/^      - name: 作業の型を読む/{f=1; next} /^      - name: /{f=0} f' <<<"$RUNJ")
ok "agent.yml: 型の段は mode.sh で TASK の型を読み、値は script か agent の 2 つだけを GITHUB_ENV に書く" "$(grep -cF -- 'm=$(bash driver/mode.sh "$TASK" || true)' <<<"$MODESTEP")/$(grep -cF -- 'case "$m" in script|agent) echo "MODE=$m" >> "$GITHUB_ENV" ;;' <<<"$MODESTEP")" 1/1
ok "agent.yml: 型の段は、GITHUB_ENV への書き込みと、失敗時の >&2 以外に echo しない" "$(grep 'echo' <<<"$MODESTEP" | grep -v 'GITHUB_ENV' | grep -vc '>&2')" 0
SCRIPTSTEP=$(awk '/^      - name: スクリプトで作業する/{f=1; next} /^      - name: /{f=0} f' <<<"$RUNJ")
ok "agent.yml: スクリプトの段は if: env.MODE == 'script' を持つ" "$(head -n 1 <<<"$SCRIPTSTEP" | sed 's/^ *//')" "if: env.MODE == 'script'"
ok "agent.yml: スクリプトの段は pipefail を置き、task.sh の run: 行を bash -c で実行し、出力を script.txt に残す" "$(grep -cF -- '          set -o pipefail' <<<"$SCRIPTSTEP")/$(grep -cF -- 'cmd=$(awk' <<<"$SCRIPTSTEP")/$(grep -cF -- 'bash -c "$cmd" 2>&1 | tee "$RUNNER_TEMP/out/script.txt"' <<<"$SCRIPTSTEP")" 1/1/1
ok "agent.yml: スクリプトの段は opencode を動かさない" "$(grep -c 'opencode' <<<"$SCRIPTSTEP")" 0
ok "agent.yml: スクリプトの段の失敗は止めず、判定(judge)が決める(agent の段と同じ)" "$(grep -cF -- 'script.txt" || true' <<<"$SCRIPTSTEP")" 1
[ -e "$T/failed" ] && fail=1; rm -rf "$T"; exit $fail
