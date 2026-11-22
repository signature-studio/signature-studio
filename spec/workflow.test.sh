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
ok "agent.yml: issues の types は、許可の一覧(opened・edited・reopened・closed・labeled・unlabeled・typed・untyped)の部分集合" "$(jq -r '(.on.issues.types - ["opened","edited","reopened","closed","labeled","unlabeled","typed","untyped"]) | length' "$J")" 0
ok "agent.yml: issues の types は typed と untyped を含む(Issue の Type の変更で再評価する)" "$(jq -r '[.on.issues.types | index("typed") != null, index("untyped") != null] | map(tostring) | join("/")' "$J")" "true/true"
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
ok "agent.yml: start と report は issues: write(GITHUB_TOKEN で書く)" "$(jq -r '[.jobs.start.permissions.issues, .jobs.report.permissions.issues] | join("/")' "$J")" 'write/write'
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
# (j) start と report の書き込みは GITHUB_TOKEN で行う(App の token は使わない)。App の鍵は、名義の段と push の段だけ。Issue の close は、push する commit の Closes #N で行う
ok "start・report: botid・GH_APP_ の secrets・GH_CONFIG_DIR を使わない" "$(jq -r '[.jobs.start, .jobs.report] | map(tostring | test("botid|GH_APP_|GH_CONFIG_DIR")) | join("/")' "$J")" "false/false"
ok "work: GH_APP_ の secrets を持つ段は、名義の段と push の段だけ" "$(jq -r '[.jobs.work.steps[] | select(tostring | test("secrets[.]GH_APP_")) | .name] | join("|")' "$J")" "名義を設定する(token は消す)|通った変更を main に push する(token はこの段で発行する)"
ok "start: running を付ける段の GH_TOKEN は github.token" "$(jq -r '[.jobs.start.steps[] | select(.name | test("running を付ける")) | .env.GH_TOKEN][0]' "$J")" '${{ github.token }}'
ok "report: 結果を Issue に書く段の GH_TOKEN は github.token" "$(jq -r '[.jobs.report.steps[] | select(.name | test("結果を Issue に書き")) | .env.GH_TOKEN][0]' "$J")" '${{ github.token }}'
ok "work: タスクの段が、題名を RUNNER_TEMP/title.txt に残す(push の段の commit に使う)" "$(jq -r '[.jobs.work.steps[] | select(.name | test("タスク")) | .run] | .[0]' "$J" | grep -q 'title[.]txt' && echo yes)" yes
P=$(jq -r '[.jobs.work.steps[] | select(.name | test("通った変更を main に push")) | .run] | .[0]' "$J")
ok "push: 判定の後に、題名と Closes #\$ISSUE の空の commit を作り、その後に push する" "$(c=$(grep -n 'git commit --allow-empty' <<<"$P" | cut -d: -f1); p=$(grep -n 'git push origin HEAD:main' <<<"$P" | cut -d: -f1); [ -n "$c" ] && [ -n "$p" ] && [ "$c" -lt "$p" ] && grep -qF 'Closes #$ISSUE' <<<"$P" && grep -qF 'title.txt' <<<"$P" && echo yes)" yes
ok "push: 変更が無くても止めない(「変更なし」で終わる経路が無い)" "$(grep -c '変更なし' <<<"$P")" 0
ok "push: commit の名義を書かない(git -c・--author・GIT_AUTHOR・GIT_COMMITTER・git config user を使わない)" "$(grep -cE 'git -c |--author|GIT_AUTHOR|GIT_COMMITTER|git config user' "$A")" 0
ok "push の段は、判定の段より後にあり、if を持たない" "$(jq -r '[.jobs.work.steps | to_entries[] | select(.value.name | test("^判定[(]")) | .key] as $j | [.jobs.work.steps | to_entries[] | select(.value.name | test("通った変更を main に push")) | .key] as $p | "\($j[0] < $p[0])/\(.jobs.work.steps[$p[0]] | has("if") | not)"' "$J")" "true/true"
R=$(jq -r '[.jobs.report.steps[] | select(.name | test("結果を Issue に書き")) | .run] | .[0]' "$J")
ok "report: gh issue close は 1 回だけ、Issue が open のときの if の中にある" "$(grep -c 'gh issue close' <<<"$R")/$(awk '/= open \]; then/{f=1} /else/{f=0} /gh issue close/{print f}' <<<"$R")" "1/1"
ok "report: 閉じられなかったときの注記を、コメントに書く" "$(grep -cF 'close を commit のキーワードで行えなかったので、後続の Issue は起動しない。PM が再評価する' <<<"$R")" 1
ok "report: 成功の段は、running を close の後に外す" "$(awk '/gh issue close/{c=NR} /--remove-label running/{if (!r) r=NR} END{print (c && r && c < r) ? "yes" : "no"}' <<<"$R")" yes
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
iss() { local l=""; for x in ${3//,/ }; do l="$l{\"name\":\"$x\"},"; done; local t=null; [ -n "${4:-}" ] && t="{\"name\":\"$4\"}"; printf '{"number":%s,"state":"%s","labels":[%s],"type":%s}' "$1" "$2" "${l%,}" "$t"; }
R=repos/o/r
fx $R/issues/5 "$(iss 5 closed "" Task)"
fx $R/issues/5/dependencies/blocking '[{"number":10,"state":"open"},{"number":11,"state":"open"}]'
fx $R/issues/10 "$(iss 10 open "" Task)"; fx $R/issues/10/dependencies/blocked_by '[{"number":5,"state":"closed"}]'; fx $R/issues/10/sub_issues '[]'
fx $R/issues/11 "$(iss 11 open "" Task)"; fx $R/issues/11/dependencies/blocked_by '[{"number":5,"state":"closed"},{"number":12,"state":"open"}]'; fx $R/issues/11/sub_issues '[]'
fx $R/issues/20 "$(iss 20 open pm-hold Task)"; fx $R/issues/20/dependencies/blocked_by '[]'; fx $R/issues/20/sub_issues '[]'
fx $R/issues/21 "$(iss 21 open "" Feature)"; fx $R/issues/21/dependencies/blocked_by '[]'; fx $R/issues/21/sub_issues '[]'
fx $R/issues/22 "$(iss 22 open "" Task)"; fx $R/issues/22/dependencies/blocked_by '[]'; fx $R/issues/22/sub_issues '[{"number":30,"state":"open"}]'
fx $R/issues/23 "$(iss 23 open failed Task)"; fx $R/issues/23/dependencies/blocked_by '[]'; fx $R/issues/23/sub_issues '[]'
fx $R/issues/24 "$(iss 24 open)"; fx $R/issues/24/dependencies/blocked_by '[]'; fx $R/issues/24/sub_issues '[]'
fx $R/issues/25 "$(iss 25 open running Task)"; fx $R/issues/25/dependencies/blocked_by '[]'; fx $R/issues/25/sub_issues '[]'
fx $R/issues/27 "$(iss 27 open task)"; fx $R/issues/27/dependencies/blocked_by '[]'; fx $R/issues/27/sub_issues '[]'
fx $R/issues/28 "$(iss 28 open "" Task)"; fx $R/issues/28/dependencies/blocked_by '[{"number":29,"state":"open"}]'; fx $R/issues/28/sub_issues '[]'
fx $R/issues/26 "$(iss 26 open needs-owner Task)"; fx $R/issues/26/dependencies/blocked_by '[]'; fx $R/issues/26/sub_issues '[]'
fx $R/issues/31 "$(iss 31 open "" Task)"; fx $R/issues/31/dependencies/blocked_by '[]'; fx $R/issues/31/sub_issues '[]'
fx $R/issues/32 "$(iss 32 open "" Bug)"; fx $R/issues/32/dependencies/blocked_by '[]'; fx $R/issues/32/sub_issues '[]'
fx $R/issues/33 "$(iss 33 open "" Task)"; fx $R/issues/33/dependencies/blocked_by '[]'; fx $R/issues/33/sub_issues '[]'
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
ok "select: Type が Feature の 21(親)は、labeled でも選ばない" "$(sel issues '{"action":"labeled","issue":{"number":21},"sender":'"$W"'}' | jq -c .targets 2>/dev/null)" "[]"
ok "select: Type が Bug の 32 は選ばない" "$(sel issues '{"action":"opened","issue":{"number":32},"sender":'"$W"'}' | jq -c .targets 2>/dev/null)" "[]"
ok "select: Type が Task なら、ラベル無しの 33 を選ぶ(ラベルは要らない)" "$(sel issues '{"action":"opened","issue":{"number":33},"sender":'"$W"'}' | jq -c .targets 2>/dev/null)" "[33]"
ok "select: sub-issue を持つ 22 は選ばない(親)" "$(sel issues '{"action":"edited","issue":{"number":22},"sender":'"$W"'}' | jq -c .targets 2>/dev/null)" "[]"
ok "select: 開いたブロッカー 29 を持つ 28 は選ばない" "$(sel issues '{"action":"edited","issue":{"number":28},"sender":'"$W"'}' | jq -c .targets 2>/dev/null)" "[]"
ok "select: running の付いた 25 は選ばない" "$(sel issues '{"action":"edited","issue":{"number":25},"sender":'"$W"'}' | jq -c .targets 2>/dev/null)" "[]"
ok "select: needs-owner の 26 は選ばない" "$(sel issues '{"action":"edited","issue":{"number":26},"sender":'"$W"'}' | jq -c .targets 2>/dev/null)" "[]"
ok "select: Type の無い 24 は選ばない" "$(sel issues '{"action":"edited","issue":{"number":24},"sender":'"$W"'}' | jq -c .targets 2>/dev/null)" "[]"
ok "select: ラベル task があっても、Type の無い 27 は選ばない" "$(sel issues '{"action":"edited","issue":{"number":27},"sender":'"$W"'}' | jq -c .targets 2>/dev/null)" "[]"
ok "select: task・epic のラベルを判定に使わない(ソースに 'task'・'epic' の文字列が無い)" "$(grep -cE "'(task|epic)'" "$top/tools/select-targets.mjs")" 0
ok "select: 条件を満たす 10 を、書き込み権限のある人の edited で選ぶ" "$(sel issues '{"action":"edited","issue":{"number":10},"sender":'"$W"'}' | jq -c .targets 2>/dev/null)" "[10]"
ok "select: failed の 23 は、普通のコメントでは選ばない" "$(sel issue_comment '{"action":"created","issue":{"number":23},"comment":{"body":"了解","user":{"type":"User"}},"sender":'"$W"'}' | jq -c .targets 2>/dev/null)" "[]"
ok "select: /retry のコメントなら failed の 23 を選び、retry を true にする" "$(sel issue_comment '{"action":"created","issue":{"number":23},"comment":{"body":"/retry\n","user":{"type":"User"}},"sender":'"$W"'}' | jq -c '[.targets, .retry]' 2>/dev/null)" "[[23],true]"
ok "select: Bot のコメントは対象にしない(/retry でも)" "$(sel issue_comment '{"action":"created","issue":{"number":23},"comment":{"body":"/retry","user":{"type":"Bot"}},"sender":'"$BOT"'}' | jq -c .targets 2>/dev/null)" "[]"
ok "select: PR のコメントは対象にしない" "$(sel issue_comment '{"action":"created","issue":{"number":23,"pull_request":{"url":"x"}},"comment":{"body":"/retry","user":{"type":"User"}},"sender":'"$W"'}' | jq -c .targets 2>/dev/null)" "[]"
ok "select: 権限の無い人(x)の /retry では、failed の 23 を選ばない" "$(sel issue_comment '{"action":"created","issue":{"number":23},"comment":{"body":"/retry","user":{"type":"User"}},"sender":'"$X"'}' | jq -c .targets 2>/dev/null)" "[]"
ok "select: 読むだけの人(r)の /retry では、failed の 23 を選ばない" "$(sel issue_comment '{"action":"created","issue":{"number":23},"comment":{"body":"/retry","user":{"type":"User"}},"sender":'"$RD"'}' | jq -c .targets 2>/dev/null)" "[]"
ok "select: 権限の無い人(x)が開いた Issue(Type 無し)は選ばない" "$(sel issues '{"action":"opened","issue":{"number":24},"sender":'"$X"'}' | jq -c .targets 2>/dev/null)" "[]"
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
fx $R/issues/23 "$(iss 23 open failed Task)"
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
# (i) pm-work/tools/create-issues.mjs: 下書きから Issue を作る道具。偽の gh(node。呼び出しを CG に残し、番号は飛び飛びに返す)で試す。実際の GitHub には書かない
CI=$top/pm-work/tools/create-issues.mjs; CD=$T/cdrafts; CM=$T/milestones.md; CS=$T/cstate.json; CG=$T/cgh.log; CFX=$T/cfx; CFN=$T/cfx-none
mkdir -p "$CD" "$CFX" "$CFN" "$T/cbin"; : > "$CG"
cat > "$T/cbin/gh" <<'FAKEGH'
#!/usr/bin/env node
import { appendFileSync, existsSync, readFileSync, writeFileSync } from 'node:fs';
const { FX, GHLOG, FAIL_ON = '', FAIL_NTH = '0', DROP_TYPE = '' } = process.env;
const argv = process.argv.slice(2);
const fail = (msg) => { process.stderr.write(`fake gh: ${msg}\n`); process.exit(1); };
if (argv[0] !== 'api') fail(`未対応: ${argv.join(' ')}`);
let method = null; let path = null; let input = false;
for (let i = 1; i < argv.length; i++) {
  const a = argv[i];
  if (a === '-X') method = argv[++i];
  else if (a === '--input') { input = true; i++; }
  else if (!a.startsWith('-') && path === null) path = a;
}
const p = path.split('?')[0];
const m = method ?? (input ? 'POST' : 'GET');
const body = input ? JSON.parse(readFileSync(0, 'utf8')) : null;
appendFileSync(GHLOG, `${JSON.stringify({ m, p, body })}\n`);
const lines = readFileSync(GHLOG, 'utf8').split('\n').filter(Boolean).map((l) => JSON.parse(l));
if (FAIL_ON && lines.filter((x) => `${x.m} ${x.p}` === FAIL_ON).length === Number(FAIL_NTH)) fail('失敗させた');
const out = (v) => process.stdout.write(`${JSON.stringify(v)}\n`);
const R = 'repos/o/r';
function route() {
  if (p === 'graphql') {
    const q = body.query;
    if (q.includes('addProjectV2ItemById')) return out({ data: { addProjectV2ItemById: { item: { id: `PVTI_${body.variables.content}` } } } });
    if (q.includes('updateProjectV2ItemFieldValue')) return out({ data: { updateProjectV2ItemFieldValue: { projectV2Item: { id: body.variables.item } } } });
    if (q.includes('projectV2(')) return out(existsSync(`${FX}/project.json`) ? JSON.parse(readFileSync(`${FX}/project.json`, 'utf8')) : { data: { organization: { projectV2: null } } });
    return fail('未対応の GraphQL');
  }
  if (p.startsWith('orgs/') && p.endsWith('/issue-types')) return out([{ id: 1, name: 'Task' }, { id: 2, name: 'Feature' }, { id: 3, name: 'Bug' }]);
  if (p.startsWith('orgs/') && p.endsWith('/issue-fields') && m === 'GET') {
    if (existsSync(`${FX}/issue-fields.err`)) return fail(readFileSync(`${FX}/issue-fields.err`, 'utf8'));
    if (!existsSync(`${FX}/issue-fields.json`)) return fail(`無い: ${p}`);
    return out(JSON.parse(readFileSync(`${FX}/issue-fields.json`, 'utf8')));
  }
  if (p === `${R}/labels` && m === 'GET') return out([]);
  if (p === `${R}/labels` && m === 'POST') return out({ name: body.name });
  if (p === `${R}/milestones` && m === 'GET') return out([]);
  if (p === `${R}/milestones` && m === 'POST') return out({ number: 99 + lines.filter((x) => x.p === `${R}/milestones` && x.m === 'POST').length, title: body.title });
  if (/^repos\/o\/r\/milestones\/\d+$/.test(p) && m === 'PATCH') return out({});
  if (p === `${R}/issues` && m === 'POST') {
    const seq = Number(existsSync(`${FX}/seq`) ? readFileSync(`${FX}/seq`, 'utf8') : 4021);
    writeFileSync(`${FX}/seq`, String(seq + 3));
    return out({ number: seq, id: seq * 10, node_id: `I_${seq}`, title: body.title, type: body.type && !DROP_TYPE ? { name: body.type } : null });
  }
  if (/^repos\/o\/r\/issues\/\d+\/sub_issues$/.test(p) && m === 'POST') return out({});
  if (/^repos\/o\/r\/issues\/\d+\/issue-field-values$/.test(p) && m === 'POST') return out(body.issue_field_values);
  if (/^repos\/o\/r\/issues\/\d+\/dependencies\/blocked_by$/.test(p) && m === 'POST') return out({});
  if (/^repos\/o\/r\/issues\/\d+$/.test(p) && m === 'PATCH') return out({});
  if (/^repos\/o\/r\/issues\/\d+\/labels\/pm-hold$/.test(p) && m === 'DELETE') return out([]);
  return fail(`未対応: ${m} ${p}`);
}
route();
FAKEGH
chmod +x "$T/cbin/gh"
cat > "$CFX/project.json" <<'PJ'
{"data":{"organization":{"projectV2":{"id":"PVT_1","fields":{"nodes":[{"id":"F_size","name":"Size","options":[{"id":"O_XS","name":"XS"},{"id":"O_S","name":"S"},{"id":"O_M","name":"M"},{"id":"O_L","name":"L"},{"id":"O_XL","name":"XL"}]},{"id":"F_prio","name":"Priority","options":[]}]}}}}}
PJ
printf '%s' '[{"id":777,"name":"Priority","data_type":"single_select","options":[{"id":1,"name":"Urgent"},{"id":2,"name":"High"},{"id":3,"name":"Medium"},{"id":4,"name":"Low"}]}]' > "$CFX/issue-fields.json"
cdraft() { printf -- '---\n%s\n---\n\n%s\n' "$2" "$3" > "$CD/$1"; }
cdraft 001-parent.md 'draft: "001"
title: "T001 親課題"
type: "Feature"
milestone: "M1 テスト(P3)"
parent: null
blocked_by: []
labels: ["render"]
size: "L"
priority: "Urgent"
project: "signature-studio/3"' '子課題をまとめる親の課題。'
cdraft 002-after.md 'draft: "002"
title: "T002 後続課題"
type: "Task"
milestone: "M1 テスト(P3)"
parent: "001"
blocked_by: ["003"]
labels: []
size: "S"
priority: "High"
project: "signature-studio/3"' '先行課題 #{{003}} が閉じたら始める。親は #{{001}}。'
cdraft 003-before.md 'draft: "003"
title: "T003 先行課題"
type: "Task"
milestone: "M1 テスト(P3)"
parent: "001"
blocked_by: []
labels: []
size: "M"
priority: "Medium"
project: "signature-studio/3"' '先に終える課題。'
cdraft 004-child.md 'draft: "004"
title: "T004 子課題"
type: "Task"
milestone: "M1 テスト(P3)"
parent: "001"
blocked_by: []
labels: ["render"]
size: "XS"
priority: "Low"
project: "signature-studio/3"' '通常の子課題。'
cat > "$CM" <<'MS'
# マイルストーン(テスト)

| 番号 | タイトル | 対応 |
| --- | --- | --- |
| M1 | M1 テスト(P3) | - |

### M1 テスト(P3)

- 説明: テスト用。下位は #{{004}}。
- 完了の条件: 無し
MS
cr() { ( cd "$top" && GH_REPO=o/r GHLOG=$CG FX=${FXD:-$CFX} FAIL_ON="${FAIL_ON:-}" FAIL_NTH="${FAIL_NTH:-0}" DROP_TYPE="${DROP_TYPE:-}" PATH=$T/cbin:$PATH node "$CI" "$@" --drafts "${DR:-$CD}" --milestones "$CM" --state "$CS" 2>"$T/cr.err" ); }
writes() { jq -c 'select(.m != "GET" and (.p != "graphql" or ((.body.query // "") | contains("mutation"))))' "$CG" | wc -l | tr -d ' '; }
: > "$CG"; rm -f "$CS"; out=$(cr); rc=$?
ok "create: dry-run は 0 で終わり、gh を 1 回も呼ばず、状態ファイルも作らない" "$rc/$(wc -l < "$CG" | tr -d ' ')/$([ -e "$CS" ] && echo made || echo none)" "0/0/none"
ok "create: dry-run は、実行しない旨と、作る Issue の行(4 件)を出す" "$(head -n 1 <<<"$out" | cut -c1-7)/$(grep -c '^   作る ' <<<"$out")" "dry-run/4"
ok "create: dry-run の事前の検査は、プロジェクトの Size と、組織の Issue field の Priority を挙げる" "$(grep -c 'プロジェクト signature-studio/3 の Size の選択肢、組織の Issue field の Priority' <<<"$out")" 1
: > "$CG"; FXD=$CFN cr --apply >/dev/null; rc=$?
ok "create: 事前の検査(プロジェクトが読めない)で、書き込みを 1 件も行わず 1 で終わる" "$rc/$(writes)/$([ -e "$CS" ] && echo made || echo none)" "1/0/none"
ok "create: 事前の検査の失敗は、プロジェクト名を含む理由を標準エラーに出す" "$(grep -c 'signature-studio/3' "$T/cr.err")" 1
ok "create: --resume だけ(--apply 無し)は使い方の誤りで 2" "$(cr --resume >/dev/null 2>&1; echo $?)" 2
rm -f "$CS" "$CFX/seq"; : > "$CG"; cr --apply > "$T/capply.out"; rc=$?
ok "create: --apply は 0 で終わる" "$rc" 0
ok "create: 作成の順は親 → 子、blocked by の先 → 後(下書きの番号の順ではない)" "$(jq -r 'select(.m=="POST" and (.p|endswith("/issues"))) | .body.title' "$CG" | paste -sd, -)" "T001 親課題,T003 先行課題,T002 後続課題,T004 子課題"
ok "create: 番号は API の応答から取り(4021 から飛び飛び)、対応表を標準出力に出す" "$(grep -c '^001 -> #4021 Feature' "$T/capply.out")/$(grep -c '^[0-9][0-9][0-9] -> #' "$T/capply.out")" "1/4"
ok "create: Issue Type は下書きの type で作る" "$(jq -r 'select(.m=="POST" and (.p|endswith("/issues"))) | .body.type' "$CG" | paste -sd, -)" "Feature,Task,Task,Task"
ok "create: ラベルは pm-hold と領域のラベルだけ(Type・Size・Priority を入れない)" "$(jq -r 'select(.m=="POST" and (.p|endswith("/issues"))) | (.body.labels | join("+"))' "$CG" | paste -sd, -)" "pm-hold+render,pm-hold,pm-hold,pm-hold+render"
ok "create: タイトルと本文に Type・Size・Priority の語が無い" "$(jq -r 'select(.m=="POST" and (.p|endswith("/issues"))) | [.body.title, .body.body] | join("\n")' "$CG" | grep -ciE 'task|epic|feature|bug|size|priority|urgent|high|medium|low')" 0
ok "create: 作成の時の本文は #{{NNN}} のまま(置換は後の PATCH)" "$(jq -r 'select(.m=="POST" and .body.title=="T002 後続課題") | .body.body' "$CG")" "先行課題 #{{003}} が閉じたら始める。親は #{{001}}。"
ok "create: 本文の PATCH は 1 件(#{{NNN}} が残らず、実番号に直る)" "$(jq -r 'select(.m=="PATCH" and (.p|startswith("repos/o/r/issues/"))) | .body.body' "$CG")/$(jq -c 'select(.m=="PATCH" and (.p|startswith("repos/o/r/issues/")))' "$CG" | wc -l | tr -d ' ')" "先行課題 #4024 が閉じたら始める。親は #4021。/1"
ok "create: マイルストーンの説明の #{{NNN}} も実番号に直す" "$(jq -r 'select(.m=="PATCH" and (.p|startswith("repos/o/r/milestones/"))) | .body.description' "$CG")" "テスト用。下位は #4030。"
ok "create: 親子は、親の番号に、子の id(番号ではない)を付ける(子 3 件)" "$(jq -r 'select(.p|endswith("/sub_issues")) | "\(.p) \(.body.sub_issue_id)"' "$CG" | paste -sd, -)" "repos/o/r/issues/4021/sub_issues 40240,repos/o/r/issues/4021/sub_issues 40270,repos/o/r/issues/4021/sub_issues 40300"
ok "create: blocked by は、後続(002)の番号に、先行(003)の id を付ける" "$(jq -r 'select(.p|endswith("/dependencies/blocked_by")) | "\(.p) \(.body.issue_id)"' "$CG" | paste -sd, -)" "repos/o/r/issues/4027/dependencies/blocked_by 40240"
ok "create: 全ての Issue をプロジェクトに追加する(4 件)" "$(grep -c 'addProjectV2ItemById' "$CG")" 4
ok "create: Size はプロジェクトの単一選択で、全ての Issue に設定する(4 件。Priority の選択肢は使わない)" "$(jq -r 'select(.body.variables.option != null) | "\(.body.variables.field) \(.body.variables.option)"' "$CG" | sort | paste -sd, -)" "F_size O_L,F_size O_M,F_size O_S,F_size O_XS"
ok "create: Priority は組織の Issue field の値として、Issue ごとに 1 回ずつ設定する(4 件)" "$(jq -r 'select(.m=="POST" and (.p|endswith("/issue-field-values"))) | "\(.p) \(.body.issue_field_values[0].field_id) \(.body.issue_field_values[0].value)"' "$CG" | paste -sd, -)" "repos/o/r/issues/4021/issue-field-values 777 Urgent,repos/o/r/issues/4024/issue-field-values 777 Medium,repos/o/r/issues/4027/issue-field-values 777 High,repos/o/r/issues/4030/issue-field-values 777 Low"
ok "create: pm-hold を、Priority・Size・関係の設定の後に、4 件から外す" "$(jq -s -r '[to_entries[] | select(.value.m=="DELETE") | .key] as $d | [to_entries[] | select(.value.m!="DELETE" and .value.m!="GET") | .key] as $o | "\($d | length)/\(($d | min) > ($o | max))"' "$CG")/$(jq -r 'select(.m=="DELETE") | .p' "$CG" | grep -c 'labels/pm-hold$')" "4/true/4"
ok "create: 状態ファイルに 4 件を、別々の番号で残す" "$(jq '.drafts | length' "$CS")/$(jq -r '[.drafts[].number] | unique | length' "$CS")/$(jq -r '.drafts["002"].number' "$CS")" "4/4/4027"
: > "$CG"; cr --apply >/dev/null 2>&1; rc=$?
ok "create: 状態ファイルが在るのに --resume が無ければ、gh を呼ばずに 1 で終わる" "$rc/$(wc -l < "$CG" | tr -d ' ')" "1/0"
rm -f "$CS" "$CFX/seq"; : > "$CG"; FAIL_ON='POST repos/o/r/issues' FAIL_NTH=3 cr --apply >/dev/null 2>&1; rc=$?
ok "create: 途中で落ちると 1 で終わり、作れた分(001・003)だけを状態ファイルに残す" "$rc/$(jq '.drafts | length' "$CS")/$(jq -r '.drafts | keys | join(",")' "$CS")" "1/2/001,003"
: > "$CG"; cr --apply --resume > "$T/cres.out"; rc=$?
ok "create: --resume は作った分を作り直さず、残り(002・004)だけを作る" "$rc/$(jq -r 'select(.m=="POST" and (.p|endswith("/issues"))) | .body.title' "$CG" | paste -sd, -)" "0/T002 後続課題,T004 子課題"
ok "create: --resume の後、4 件とも別の番号で、重複が無い" "$(jq '.drafts | length' "$CS")/$(jq -r '[.drafts[].number] | unique | length' "$CS")/$(jq -r '.drafts["004"].number' "$CS")" "4/4/4030"
vz() { local dir=$T/cv-$1; rm -rf "$dir"; cp -r "$CD" "$dir"; sed -i "s/$2/$3/" "$dir/$4"; : > "$CG"; DR=$dir cr >/dev/null 2>&1; a=$?; DR=$dir cr --apply >/dev/null 2>&1; b=$?; ok "create: $5" "$a/$b/$(wc -l < "$CG" | tr -d ' ')" "1/1/0"; }
vz ref '#{{003}}' '#{{999}}' 002-after.md "下書きの無い #{{999}} があると、dry-run と --apply の両方が、書く前に 1 で止まる"
vz kisoku '先に終える課題。' '規模: 大。先に終える課題。' 003-before.md "本文に 規模 を書くと、dry-run と --apply の両方が、書く前に 1 で止まる"
vz type 'type: "Task"' 'type: "epic"' 004-child.md "Type が Task・Feature・Bug 以外なら、dry-run と --apply の両方が、書く前に 1 で止まる"
vz label 'labels: \["render"\]' 'labels: ["task"]' 004-child.md "ラベルに task を書くと、dry-run と --apply の両方が、書く前に 1 で止まる"
for c in 403 three nopriority text; do mkdir -p "$T/cfx-$c"; cp "$CFX/project.json" "$T/cfx-$c/project.json"; done
printf 'Resource not accessible by integration (HTTP 403)' > "$T/cfx-403/issue-fields.err"
printf '%s' '[{"id":777,"name":"Priority","data_type":"single_select","options":[{"id":1,"name":"Urgent"},{"id":2,"name":"High"},{"id":3,"name":"Medium"}]}]' > "$T/cfx-three/issue-fields.json"
printf '%s' '[]' > "$T/cfx-nopriority/issue-fields.json"
printf '%s' '[{"id":777,"name":"Priority","data_type":"text","options":[]}]' > "$T/cfx-text/issue-fields.json"
pn() { rm -f "$CS" "$CFX/seq"; : > "$CG"; FXD=$1 cr --apply >/dev/null 2>&1; rc=$?; echo "$rc/$(writes)/$([ -e "$CS" ] && echo made || echo none)"; }
ok "create: Issue fields が 403 なら、書き込みを 1 件も行わず 1 で終わる" "$(pn "$T/cfx-403")" "1/0/none"
ok "create: 403 の理由は、組織の Issue Fields の読み取りの権限が要る、と言う" "$(grep -c 'App に、組織の Issue Fields の読み取りの権限が要る' "$T/cr.err")" 1
ok "create: Priority の選択肢が 3 つ(Low が無い)なら、書き込みを 1 件も行わず 1 で終わり、Low が無いと言う" "$(pn "$T/cfx-three")/$(grep -c 'Priority の選択肢が無い: Low' "$T/cr.err")" "1/0/none/1"
ok "create: 組織に Issue field の Priority が無ければ、書き込みを 1 件も行わず 1 で終わる" "$(pn "$T/cfx-nopriority")" "1/0/none"
ok "create: Priority が単一選択でなければ、書き込みを 1 件も行わず 1 で終わる" "$(pn "$T/cfx-text")/$(grep -c '単一選択' "$T/cr.err")" "1/0/none/1"
rm -f "$CS" "$CFX/seq"; : > "$CG"; DROP_TYPE=1 cr --apply >/dev/null 2>&1; rc=$?
ok "create: 作成の応答に Issue Type が無い(捨てられた)なら、その場で 1 で止まり、次の Issue を作らない" "$rc/$(jq -r 'select(.m=="POST" and (.p|endswith("/issues"))) | .body.title' "$CG" | wc -l | tr -d ' ')/$(jq '.drafts | length' "$CS")" "1/1/1"
ok "create: Type が違うと、標準エラーに Issue Type が Feature でない、と出す" "$(grep -c 'Issue Type が Feature でない' "$T/cr.err")" 1
[ -e "$T/failed" ] && fail=1; rm -rf "$T"; exit $fail
