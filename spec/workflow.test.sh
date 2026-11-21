#!/usr/bin/env bash
# spec/workflow.test.sh : 作業フロー(.github/workflows/agent.yml と tools/)が、設計(pm-work/workflow-design.md)の約束を守ることを確かめるテスト。
# 一時ディレクトリはリポジトリの中の .tmp-spec.*(git の無視の対象)に作り、終わったら消す。
# YAML の読みは検査だけに python3 と PyYAML を使う(製品と作業の経路には使わない)。旧版との比較は git show の 1306a01c の内容を使う(削除の前の版)。
# tools/select-targets.mjs は偽の gh(PATH の先頭)で試す。偽の gh は読みを fixture から返し、書きを GHLOG に残す。
here=$(cd "$(dirname "$0")" && pwd); top=$here/..; fail=0; T=$(mktemp -d "$here/../.tmp-spec.XXXXXX")
ok() { if [ "$2" = "$3" ]; then echo "ok   $1"; else echo "FAIL $1: expected $3, got $2"; fail=1; : > "$T/failed"; fi; }
A=$top/.github/workflows/agent.yml; OLD=1306a01c
git -C "$top" show "$OLD:.github/workflows/agent.yml" > "$T/old.yml" 2>/dev/null || { echo "FAIL 旧版の agent.yml を読めない($OLD)"; fail=1; }
# YAML を JSON にする。on のキーは PyYAML では真偽値になるので、on に直す
pj() { python3 -c '
import sys, yaml, json
d = yaml.safe_load(open(sys.argv[1], encoding="utf-8"))
if True in d: d["on"] = d.pop(True)
print(json.dumps(d, ensure_ascii=False))' "$1"; }
pj "$A" > "$T/new.json" 2>"$T/pj.err"; pj "$T/old.yml" > "$T/old.json" 2>>"$T/pj.err"
ok "agent.yml: YAML として読める(旧版も読める)" "$(test -s "$T/new.json" && test -s "$T/old.json" && echo yes)" yes
J=$T/new.json; O=$T/old.json
# (b) on: issues と issue_comment だけ。workflow_dispatch・schedule・push・pull_request は置かない
ok "agent.yml: on のキーは issues と issue_comment だけ" "$(jq -c '.on | keys' "$J")" '["issue_comment","issues"]'
ok "agent.yml: issues の types は、許可の一覧(opened・edited・reopened・closed・labeled・unlabeled)の部分集合" "$(jq -r '(.on.issues.types - ["opened","edited","reopened","closed","labeled","unlabeled"]) | length' "$J")" 0
ok "agent.yml: issues の types は closed を含む(ブロッカーの解除を検知する)" "$(jq -r '.on.issues.types | index("closed") != null' "$J")" true
ok "agent.yml: issue_comment の types は created だけ" "$(jq -c '.on.issue_comment.types' "$J")" '["created"]'
# (d) 旧い駆動の参照が残らない(driver・queue・MODE・inputs・ledger・inventory)
ok "agent.yml: driver/・queue/・作業モードの MODE・inputs.・ledger/・inventory を参照しない(行の数)" "$(grep -cE 'driver/|queue/|(^|[^A-Z_])MODE([^A-Z_]|$)|inputs[.]|ledger/|inventory' "$A")" 0
# (i) ジョブの順と権限
ok "agent.yml: ジョブは select・start・extract・work・report" "$(jq -r '.jobs | keys | sort | join(",")' "$J")" "extract,report,select,start,work"
ok "agent.yml: needs は select→start→extract・work→report の順(start は select の後、work は extract と start の後)" "$(jq -r '[.jobs.start.needs, .jobs.extract.needs, (.jobs.work.needs | sort | join(",")), (.jobs.report.needs | sort | join(","))] | map(tostring) | join("/")' "$J")" 'select/start/extract,start/start,work'
ok "agent.yml: work は Issue の番号ごとの matrix(fail-fast: false)" "$(jq -r '.jobs.work.strategy["fail-fast"] | tostring' "$J")/$(jq -r '.jobs.work.strategy.matrix.issue' "$J")" "false/\${{ fromJSON(needs.start.outputs.claimed || '[]') }}"
ok "agent.yml: 条件を書いたジョブは success() を持つ(if を書くと既定の success() が外れる)" "$(for j in start extract work; do jq -r --arg j "$j" '.jobs[$j].if' "$J" | grep -c 'success()'; done | paste -sd '')" 111
ok "agent.yml: work の concurrency は task-<番号>、cancel-in-progress: false" "$(jq -r '.jobs.work.concurrency.group' "$J")/$(jq -r '.jobs.work.concurrency["cancel-in-progress"] | tostring' "$J")" 'task-${{ matrix.issue }}/false'
ok "agent.yml: report は always() で動き、start が起動した対象がある時だけ(if に両方)" "$(jq -r '.jobs.report.if' "$J" | grep -cF "always() && needs.start.result == 'success' && needs.start.outputs.claimed != '[]'")" 1
ok "agent.yml: select の権限は issues: read(書き込みなし)" "$(jq -r '.jobs.select.permissions.issues' "$J")" read
ok "agent.yml: work の権限に issues: write が無い(書き込みは start と report だけ)" "$(jq -r '.jobs.work.permissions.issues' "$J")" read
ok "agent.yml: start と report は issues: write(App の token で書く)" "$(jq -r '[.jobs.start.permissions.issues, .jobs.report.permissions.issues] | join("/")' "$J")" 'write/write'
ok "agent.yml: report は成果物の名前 agent-<番号> を取る(work の保存と同じ名前)" "$(grep -cF 'name: agent-${{ matrix.issue }}' "$A")/$(grep -cF 'gh run download "$RUN_ID" -n "agent-$n"' "$A")" 1/1
# (c) opencode の導入・認証は、旧版と中身が一致(if を除く)。起動の段は、旧版の 1 行(driver/task.sh を呼ぶ行)だけを除く
for n in "opencode を導入する(版は固定)" "opencode の認証情報を取り込み、作業者のモデルを設定する"; do
  o=$(jq -S --arg n "$n" '[.jobs.run.steps[] | select(.name == $n) | del(.if)] | .[0]' "$O")
  w=$(jq -S --arg n "$n" '[.jobs.work.steps[] | select(.name == $n) | del(.if)] | .[0]' "$J")
  ok "opencode: 「$n」の run・uses・env・with は旧版と一致" "$([ "$o" = "$w" ] && [ "$o" != null ] && echo same)" same
  ok "opencode: 「$n」は if を持たない(モードの分岐を除く)" "$(jq -r --arg n "$n" '[.jobs.work.steps[] | select(.name == $n) | has("if")] | .[0]' "$J")" false
done
n="作業者を 1 回動かす"
o=$(jq -r --arg n "$n" '[.jobs.run.steps[] | select(.name == $n) | .run] | .[0]' "$O" | grep -v 'driver/task.sh')
w=$(jq -r --arg n "$n" '[.jobs.work.steps[] | select(.name == $n) | .run] | .[0]' "$J")
ok "opencode: 「作業者を 1 回動かす」は、旧版から driver/task.sh の行だけを除いた物と一致" "$([ "$o" = "$w" ] && [ -n "$w" ] && echo same)" same
ok "opencode: 「作業者を 1 回動かす」に if が無い" "$(jq -r --arg n "$n" '[.jobs.work.steps[] | select(.name == $n) | has("if")] | .[0]' "$J")" false
ok "opencode: プロンプトは、AGENTS.md と Issue の本文から作る(別の段)" "$(jq -r '[.jobs.work.steps[] | select(.name | startswith("作業者のプロンプトを作る")) | .run] | .[0]' "$J" | grep -cE 'cat AGENTS[.]md.*task[.]md')" 1
# 変えない段は、旧版と中身が一致する(run・uses・env・with・if)
KEEP=("リポジトリを取得する(資格は残さない)" "名義を設定する(token は消す)" "版の値を ci/versions.env から読む" "Node を入れる" "rust をそろえ、app の依存を入れてビルドし、fe-build を用意する" "emsdk を入れる" "アカウント ID を app/wrangler.jsonc から読む" "抽出物を R2 のキャッシュから取る(公開の artifact は使わない)" "抽出物を lift/out-app/host に置く")
diffs=""; for n in "${KEEP[@]}"; do
  o=$(jq -S --arg n "$n" '[.jobs.run.steps[] | select(.name == $n)] | .[0]' "$O"); w=$(jq -S --arg n "$n" '[.jobs.work.steps[] | select(.name == $n)] | .[0]' "$J")
  [ "$o" != null ] && [ "$o" = "$w" ] || diffs="$diffs|$n"
done
ok "作業の段: 変えない段は旧版と中身が一致する(差の一覧)" "$diffs" ""
EKEEP=("抽出物のキャッシュ(R2)" "インストーラを取り、抽出する(キャッシュが無いときだけ)")
diffs=""; for n in "${EKEEP[@]}"; do
  o=$(jq -S --arg n "$n" '[.jobs.extract.steps[] | select(.name == $n)] | .[0]' "$O"); w=$(jq -S --arg n "$n" '[.jobs.extract.steps[] | select(.name == $n)] | .[0]' "$J")
  [ "$o" != null ] && [ "$o" = "$w" ] || diffs="$diffs|$n"
done
ok "抽出の段: R2 のキャッシュの段は旧版と中身が一致する(差の一覧)" "$diffs" ""
# (e) 作業者の段に token が無い(job の env にも無い)
ok "作業者の段: env に GITHUB_TOKEN・secrets が無い" "$(jq -r --arg n "作業者を 1 回動かす" '[.jobs.work.steps[] | select(.name == $n) | (.env // {}) | tostring | test("secrets|github[.]token")] | .[0]' "$J")" false
ok "作業ジョブ: job の env に secrets・github.token が無い" "$(jq -r '(.jobs.work.env // {}) | tostring | test("secrets|github[.]token")' "$J")" false
# (f) 判定の段は pipefail を持つ。push の段は if を持たない。判定の出力は成果物に残す
ok "判定の段: 最初の行が set -o pipefail(パイプで失敗を隠さない)" "$(jq -r '.jobs.work.steps[] | select((.run // "") | test("judge[.]sh\" refs/judge/main")) | .run' "$J" | grep -m1 . )" "set -o pipefail"
ok "判定の段: judge の出力を out/judge.txt に tee で残す" "$(grep -cF 'tee "$RUNNER_TEMP/out/judge.txt"' "$A")" 1
ok "push の段: if を持たない(判定が通ったときだけ走る)" "$(jq -r '[.jobs.work.steps[] | select(.name | test("通った変更を main に push")) | has("if")] | .[0]' "$J")" false
ok "push の段: push できた SHA を out/commit.txt に残す" "$(grep -cF 'git rev-parse HEAD > "$RUNNER_TEMP/out/commit.txt"' "$A")" 1
ok "どの段にも continue-on-error が無い" "$(grep -c 'continue-on-error' "$A")" 0
ok "agent.yml: タブ文字が無い" "$(grep -c "$(printf '\t')" "$A")" 0
ok "agent.yml・deploy.yml・tools/ に、アカウント ID の値を直書きしていない" "$(acct=$(grep '"account_id"' "$top/app/wrangler.jsonc" 2>/dev/null | cut -d'"' -f4); if [ -n "$acct" ]; then cat "$A" "$top/.github/workflows/deploy.yml" "$top"/tools/*.mjs | grep -cF -- "$acct"; else echo none; fi)" 0
# (g) tools/select-targets.mjs を、偽の gh と偽のイベントで試す
FX=$T/fx; GHLOG=$T/gh.log; mkdir -p "$FX" "$T/bin"; : > "$GHLOG"
cat > "$T/bin/gh" <<'GH'
#!/usr/bin/env bash
echo "$*" >> "$GHLOG"
case "$1" in
  api)
    p=${2%%\?*}; f="$FX/$(printf '%s' "$p" | sed 's#/#__#g').json"
    [ -f "$f" ] || { echo "fake gh: 無い: $p" >&2; exit 1; }
    cat "$f" ;;
  issue) exit 0 ;;
  *) echo "fake gh: 未対応: $*" >&2; exit 1 ;;
esac
GH
chmod +x "$T/bin/gh"
fx() { printf '%s' "$2" > "$FX/$(printf '%s' "$1" | sed 's#/#__#g').json"; }
iss() { local l=""; for x in ${3//,/ }; do l="$l{\"name\":\"$x\"},"; done; printf '{"number":%s,"state":"%s","labels":[%s]}' "$1" "$2" "${l%,}"; }
R=repos/o/r
fx $R/issues/5 "$(iss 5 closed task)"
fx $R/issues/5/dependencies/blocking '[{"number":10,"state":"open"},{"number":11,"state":"open"}]'
fx $R/issues/10 "$(iss 10 open task)"; fx $R/issues/10/dependencies/blocked_by '[{"number":5,"state":"closed"}]'; fx $R/issues/10/sub_issues '[]'
fx $R/issues/11 "$(iss 11 open task)"; fx $R/issues/11/dependencies/blocked_by '[{"number":5,"state":"closed"},{"number":12,"state":"open"}]'; fx $R/issues/11/sub_issues '[]'
fx $R/issues/20 "$(iss 20 open task,pm-hold)"; fx $R/issues/20/dependencies/blocked_by '[]'; fx $R/issues/20/sub_issues '[]'
fx $R/issues/21 "$(iss 21 open task,epic)"; fx $R/issues/21/dependencies/blocked_by '[]'; fx $R/issues/21/sub_issues '[]'
fx $R/issues/22 "$(iss 22 open task)"; fx $R/issues/22/dependencies/blocked_by '[]'; fx $R/issues/22/sub_issues '[{"number":30,"state":"open"}]'
fx $R/issues/23 "$(iss 23 open task,failed)"; fx $R/issues/23/dependencies/blocked_by '[]'; fx $R/issues/23/sub_issues '[]'
fx $R/issues/24 "$(iss 24 open)"; fx $R/issues/24/dependencies/blocked_by '[]'; fx $R/issues/24/sub_issues '[]'
fx $R/issues/25 "$(iss 25 open task,running)"; fx $R/issues/25/dependencies/blocked_by '[]'; fx $R/issues/25/sub_issues '[]'
fx $R/issues/28 "$(iss 28 open task)"; fx $R/issues/28/dependencies/blocked_by '[{"number":29,"state":"open"}]'; fx $R/issues/28/sub_issues '[]'
fx $R/issues/26 "$(iss 26 open task,needs-owner)"; fx $R/issues/26/dependencies/blocked_by '[]'; fx $R/issues/26/sub_issues '[]'
fx $R/issues/31 "$(iss 31 open task)"; fx $R/issues/31/dependencies/blocked_by '[]'; fx $R/issues/31/sub_issues '[]'
fx $R/collaborators/w/permission '{"permission":"write","role_name":"write"}'
fx $R/collaborators/r/permission '{"permission":"read","role_name":"read"}'
# x の権限は fixture を置かない(gh api が失敗する = 権限の無い人。実際の API でも非コラボレーターは 404)
sel() { # 引数: イベント名、イベントの JSON、(任意)gh の追加の準備。標準出力に select の出力
  printf '%s' "$2" > "$T/ev.json"
  ( GITHUB_EVENT_NAME=$1 GITHUB_EVENT_PATH=$T/ev.json GH_REPO=o/r FX=$FX GHLOG=$GHLOG PATH=$T/bin:$PATH node "$top/tools/select-targets.mjs" select 2>"$T/sel.err" )
}
W='{"login":"w","type":"User"}'; RD='{"login":"r","type":"User"}'; X='{"login":"x","type":"User"}'; BOT='{"login":"agent-app[bot]","type":"Bot"}'
ok "select: closed(5)の解除で、ブロッカーが残る 11 を外し、解除された 10 だけを選ぶ" "$(sel issues '{"action":"closed","issue":{"number":5},"sender":'"$W"'}' | jq -c .targets 2>/dev/null)" "[10]"
ok "select: opened で pm-hold が付いた 20 は選ばない" "$(sel issues '{"action":"opened","issue":{"number":20},"sender":'"$W"'}' | jq -c .targets 2>/dev/null)" "[]"
ok "select: labeled で epic の 21 は選ばない" "$(sel issues '{"action":"labeled","issue":{"number":21},"sender":'"$W"'}' | jq -c .targets 2>/dev/null)" "[]"
ok "select: sub-issue を持つ 22 は選ばない(親)" "$(sel issues '{"action":"edited","issue":{"number":22},"sender":'"$W"'}' | jq -c .targets 2>/dev/null)" "[]"
ok "select: 開いたブロッカー 29 を持つ 28 は選ばない" "$(sel issues '{"action":"edited","issue":{"number":28},"sender":'"$W"'}' | jq -c .targets 2>/dev/null)" "[]"
ok "select: running の付いた 25 は選ばない" "$(sel issues '{"action":"edited","issue":{"number":25},"sender":'"$W"'}' | jq -c .targets 2>/dev/null)" "[]"
ok "select: needs-owner の 26 は選ばない" "$(sel issues '{"action":"edited","issue":{"number":26},"sender":'"$W"'}' | jq -c .targets 2>/dev/null)" "[]"
ok "select: task ラベルの無い 24 は選ばない" "$(sel issues '{"action":"edited","issue":{"number":24},"sender":'"$W"'}' | jq -c .targets 2>/dev/null)" "[]"
ok "select: 条件を満たす 10 を、書き込み権限のある人の edited で選ぶ" "$(sel issues '{"action":"edited","issue":{"number":10},"sender":'"$W"'}' | jq -c .targets 2>/dev/null)" "[10]"
ok "select: failed の 23 は、普通のコメントでは選ばない" "$(sel issue_comment '{"action":"created","issue":{"number":23},"comment":{"body":"了解","user":{"type":"User"}},"sender":'"$W"'}' | jq -c .targets 2>/dev/null)" "[]"
ok "select: /retry のコメントなら failed の 23 を選び、retry を true にする" "$(sel issue_comment '{"action":"created","issue":{"number":23},"comment":{"body":"/retry\n","user":{"type":"User"}},"sender":'"$W"'}' | jq -c '[.targets, .retry]' 2>/dev/null)" "[[23],true]"
ok "select: Bot のコメントは対象にしない(/retry でも)" "$(sel issue_comment '{"action":"created","issue":{"number":23},"comment":{"body":"/retry","user":{"type":"Bot"}},"sender":'"$BOT"'}' | jq -c .targets 2>/dev/null)" "[]"
ok "select: PR のコメントは対象にしない" "$(sel issue_comment '{"action":"created","issue":{"number":23,"pull_request":{"url":"x"}},"comment":{"body":"/retry","user":{"type":"User"}},"sender":'"$W"'}' | jq -c .targets 2>/dev/null)" "[]"
ok "select: 権限の無い人(x)の /retry では、failed の 23 を選ばない" "$(sel issue_comment '{"action":"created","issue":{"number":23},"comment":{"body":"/retry","user":{"type":"User"}},"sender":'"$X"'}' | jq -c .targets 2>/dev/null)" "[]"
ok "select: 読むだけの人(r)の /retry では、failed の 23 を選ばない" "$(sel issue_comment '{"action":"created","issue":{"number":23},"comment":{"body":"/retry","user":{"type":"User"}},"sender":'"$RD"'}' | jq -c .targets 2>/dev/null)" "[]"
ok "select: 権限の無い人(x)が開いた Issue(task ラベル無し)は選ばない" "$(sel issues '{"action":"opened","issue":{"number":24},"sender":'"$X"'}' | jq -c .targets 2>/dev/null)" "[]"
ok "select: 権限の無い人(x)による edited では、条件を満たす 10 でも選ばない(作者による本文の書き換え)" "$(sel issues '{"action":"edited","issue":{"number":10},"sender":'"$X"'}' | jq -c .targets 2>/dev/null)" "[]"
ok "select: 権限の無い人(x)が閉じたブロッカー 5 では、後続の 10 を選ばない" "$(sel issues '{"action":"closed","issue":{"number":5},"sender":'"$X"'}' | jq -c .targets 2>/dev/null)" "[]"
ok "select: sender の無いイベントは選ばない(権限を確かめられない)" "$(sel issues '{"action":"edited","issue":{"number":10}}' | jq -c .targets 2>/dev/null)" "[]"
ok "select: App(Bot)が閉じた 5 は、解除された 10 を選ぶ(報告の close の経路)" "$(sel issues '{"action":"closed","issue":{"number":5},"sender":'"$BOT"'}' | jq -c .targets 2>/dev/null)" "[10]"
ok "select: App(Bot)が外した pm-hold の 31 は選ぶ" "$(sel issues '{"action":"unlabeled","issue":{"number":31},"label":{"name":"pm-hold"},"sender":'"$BOT"'}' | jq -c .targets 2>/dev/null)" "[31]"
ok "select: 権限の無い人(x)が外した pm-hold の 31 は選ばない" "$(sel issues '{"action":"unlabeled","issue":{"number":31},"label":{"name":"pm-hold"},"sender":'"$X"'}' | jq -c .targets 2>/dev/null)" "[]"
: > "$GHLOG"; out=$(sel issues '{"action":"closed","issue":{"number":5},"sender":'"$W"'}' >/dev/null; cat "$GHLOG")
ok "select: 読みだけ行う(gh の書き込みが無い)" "$(grep -c 'issue \(edit\|close\|comment\)' <<<"$out")" 0
cl() { ( GH_REPO=o/r FX=$FX GHLOG=$GHLOG PATH=$T/bin:$PATH node "$top/tools/select-targets.mjs" claim "$1" "$2" 2>"$T/cl.err" ); }
: > "$GHLOG"; ok "claim: 条件を満たす 10 に running を付け、claimed に入れる" "$(cl '[10]' false | jq -c .claimed 2>/dev/null)/$(grep -c 'issue edit 10 .*--add-label running' "$GHLOG")" "[10]/1"
fx $R/issues/23 "$(iss 23 open task,failed)"
: > "$GHLOG"; ok "claim: /retry の 23 は running を付けてから failed を外す(外す → 付けの順だと二重に起動する)" "$(cl '[23]' true | jq -c .claimed 2>/dev/null)/$(a=$(grep -n 'issue edit 23 .*--add-label running' "$GHLOG" | cut -d: -f1); r=$(grep -n 'issue edit 23 .*--remove-label failed' "$GHLOG" | cut -d: -f1); [ -n "$a" ] && [ -n "$r" ] && [ "$a" -lt "$r" ] && echo add-first)" "[23]/add-first"
: > "$GHLOG"; ok "claim: 普通の再起動(retry でない)は failed の 23 を選ばない" "$(cl '[23]' false | jq -c .claimed 2>/dev/null)/$(grep -c 'issue edit' "$GHLOG")" "[]/0"
: > "$GHLOG"; ok "claim: pm-hold の 20 は付けない" "$(cl '[20]' false | jq -c .claimed 2>/dev/null)/$(grep -c 'issue edit' "$GHLOG")" "[]/0"
# (h) tools/task-accept.mjs: 本文から、accept ブロックの 1 行を取り出す
acc() { printf '%b' "$1" | ( cd "$top" && node tools/task-accept.mjs 2>"$T/acc.err" ); echo "rc=$?"; }
ok "task-accept: accept ブロックの 1 行を、accept: の行として出す" "$(acc 'タスク\n\n```accept\nbash .tmp-x.sh\n```\n' | head -n 1)" "accept: bash .tmp-x.sh"
ok "task-accept: 終了コードは 0" "$(acc '```accept\nfalse\n```\n' | tail -n 1)" "rc=0"
ok "task-accept: CRLF の本文でも取り出す" "$(acc '```accept\r\nfalse\r\n```\r\n' | head -n 1)" "accept: false"
ok "task-accept: accept ブロックが無ければ 1 で終わり、理由を標準エラーに出す" "$(acc '本文だけ\n' | tail -n 1)/$(grep -c 'accept' "$T/acc.err")" "rc=1/1"
ok "task-accept: accept ブロックが 2 つなら 1 で終わる" "$(acc '```accept\nfalse\n```\n\n```accept\ntrue\n```\n' | tail -n 1)" "rc=1"
ok "task-accept: 中身が 2 行なら 1 で終わる" "$(acc '```accept\nfalse\ntrue\n```\n' | tail -n 1)" "rc=1"
ok "task-accept: 閉じていない accept ブロックは 1 で終わる" "$(acc '```accept\nfalse\n' | tail -n 1)" "rc=1"
ok "task-accept: 中身が空なら 1 で終わる" "$(acc '```accept\n\n```\n' | tail -n 1)" "rc=1"
[ -e "$T/failed" ] && fail=1; rm -rf "$T"; exit $fail
