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
# 抽出の木の鍵に lift/plugins の plugin.tsv を足した(Issue #1369)。旧版との比較は、その足した 1 項目を除いた版(JN・DJN)で行う。他の変更は比較に掛かる
sed -e "s/, 'lift\/plugins\/[^']*'//" "$A" > "$T/new-noplg.yml"; pj "$T/new-noplg.yml" > "$T/new-noplg.json" 2>>"$T/pj.err"; JN=$T/new-noplg.json
# (b) on: issues と issue_comment だけ。workflow_dispatch・schedule・push・pull_request は置かない
ok "agent.yml: on のキーは issues と issue_comment だけ" "$(jq -c '.on | keys' "$J")" '["issue_comment","issues"]'
ok "agent.yml: issues の types は、許可の一覧(opened・edited・reopened・closed・labeled・unlabeled・typed・untyped)の部分集合" "$(jq -r '(.on.issues.types - ["opened","edited","reopened","closed","labeled","unlabeled","typed","untyped"]) | length' "$J")" 0
ok "agent.yml: issues の types は typed と untyped を含む(Issue の Type の変更で再評価する)" "$(jq -r '[.on.issues.types | index("typed") != null, index("untyped") != null] | map(tostring) | join("/")' "$J")" "true/true"
ok "agent.yml: issues の types は closed を含む(ブロッカーの解除を検知する)" "$(jq -r '.on.issues.types | index("closed") != null' "$J")" true
ok "agent.yml: issue_comment の types は created だけ" "$(jq -c '.on.issue_comment.types' "$J")" '["created"]'
# (d) 旧い駆動の参照が残らない(driver・queue・MODE・inputs・ledger・inventory)
ok "agent.yml: driver/・queue/・作業モードの MODE・inputs.・ledger/ を参照しない(行の数)" "$(grep -cE 'driver/|queue/|(^|[^A-Z_])MODE([^A-Z_]|$)|inputs[.]|ledger/' "$A")" 0
# (i) ジョブの順と権限
ok "agent.yml: ジョブは select・start・extract・work・report" "$(jq -r '.jobs | keys | sort | join(",")' "$J")" "extract,report,select,start,work"
ok "agent.yml: needs は select→start→extract・work→report の順(start は select の後、extract は select と start の後、work は select・start・extract の後)" "$(jq -r '[.jobs.start.needs, .jobs.extract.needs, (.jobs.work.needs | sort | join(",")), (.jobs.report.needs | sort | join(","))] | map(tostring) | join("/")' "$J")" 'select/["select","start"]/extract,select,start/start,work'
ok "agent.yml: work のジョブ名は「作業 #<番号>」(報告が「作業 #<番号>」で照合する。YAML の ' #' で切れないように引用する)" "$(jq -r '.jobs.work.name' "$J")" '作業 #${{ matrix.issue }}'
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
  ok "opencode: 「$n」の if は、成功の後で、確認の skip が偽のときだけ" "$(jq -r --arg n "$n" '[.jobs.work.steps[] | select(.name == $n) | .["if"]] | .[0]' "$J")" "success() && steps.check.outputs.skip != 'true'"
done
n="作業者を 1 回動かす"
o=$(jq -r --arg n "$n" '[.jobs.run.steps[] | select(.name == $n) | .run] | .[0]' "$O" | grep -v 'driver/task.sh')
w=$(jq -r --arg n "$n" '[.jobs.work.steps[] | select(.name == $n) | .run] | .[0]' "$J")
ok "opencode: 「作業者を 1 回動かす」は、旧版から driver/task.sh の行だけを除いた物と一致" "$([ "$o" = "$w" ] && [ -n "$w" ] && echo same)" same
ok "opencode: 「作業者を 1 回動かす」の if は、成功の後で、確認の skip が偽のときだけ" "$(jq -r --arg n "$n" '[.jobs.work.steps[] | select(.name == $n) | .["if"]] | .[0]' "$J")" "success() && steps.check.outputs.skip != 'true'"
ok "opencode: プロンプトは、AGENTS.md と Issue の本文から作る(別の段)" "$(jq -r '[.jobs.work.steps[] | select(.name | startswith("作業者のプロンプトを作る")) | .run] | .[0]' "$J" | grep -cE 'cat AGENTS[.]md.*task[.]md')" 1
# 変えない段は、旧版と中身が一致する(run・uses・env・with・if)
KEEP=("リポジトリを取得する(資格は残さない)" "名義を設定する(token は消す)" "版の値を ci/versions.env から読む" "Node を入れる" "rust をそろえ、app の依存を入れてビルドし、fe-build を用意する" "emsdk を入れる" "アカウント ID を app/wrangler.jsonc から読む" "抽出物を R2 のキャッシュから取る(公開の artifact は使わない)" "抽出物を lift/out-app/host に置く")
diffs=""; for n in "${KEEP[@]}"; do
  o=$(jq -S --arg n "$n" '[.jobs.run.steps[] | select(.name == $n) | del(.if)] | .[0]' "$O"); w=$(jq -S --arg n "$n" '[.jobs.work.steps[] | select(.name == $n) | del(.if)] | .[0]' "$JN")
  [ "$o" != null ] && [ "$o" = "$w" ] || diffs="$diffs|$n"
done
ok "作業の段: 変えない段は旧版と中身が一致する(差の一覧)" "$diffs" ""
EKEEP=("抽出物のキャッシュ(R2)" "インストーラを取り、抽出する(キャッシュが無いときだけ)")
diffs=""; for n in "${EKEEP[@]}"; do
  o=$(jq -S --arg n "$n" '[.jobs.extract.steps[] | select(.name == $n)] | .[0]' "$O"); w=$(jq -S --arg n "$n" '[.jobs.extract.steps[] | select(.name == $n)] | .[0]' "$JN")
  [ "$o" != null ] && [ "$o" = "$w" ] || diffs="$diffs|$n"
done
ok "抽出の段: R2 のキャッシュの段は旧版と中身が一致する(差の一覧)" "$diffs" ""
# (e) 作業者の段に token が無い(job の env にも無い)
ok "作業者の段: env に GITHUB_TOKEN・secrets が無い" "$(jq -r --arg n "作業者を 1 回動かす" '[.jobs.work.steps[] | select(.name == $n) | (.env // {}) | tostring | test("secrets|github[.]token")] | .[0]' "$J")" false
ok "作業ジョブ: job の env に secrets・github.token が無い" "$(jq -r '(.jobs.work.env // {}) | tostring | test("secrets|github[.]token")' "$J")" false
# (f) 判定の段は pipefail を持つ。push の段は if を持たない。判定の出力は成果物に残す
ok "判定の段: 最初の行が set -o pipefail(パイプで失敗を隠さない)" "$(jq -r '.jobs.work.steps[] | select((.run // "") | test("judge[.]sh\" refs/judge/main")) | .run' "$J" | grep -m1 . )" "set -o pipefail"
ok "判定の段: judge の出力を out/judge.txt に tee で残す" "$(grep -cF 'tee "$RUNNER_TEMP/out/judge.txt"' "$A")" 1
ok "push の段: if は成功の後で、確認の skip が偽のときだけ(判定が通ったときだけ走る)" "$(jq -r '[.jobs.work.steps[] | select(.name | test("通った変更を main に push")) | .["if"]] | .[0]' "$J")" "success() && steps.check.outputs.skip != 'true'"
ok "push の段: push できた SHA を out/commit.txt に残す" "$(grep -cF 'git rev-parse HEAD > "$RUNNER_TEMP/out/commit.txt"' "$A")" 1
ok "どの段にも continue-on-error が無い" "$(grep -c 'continue-on-error' "$A")" 0
ok "agent.yml: タブ文字が無い" "$(grep -c "$(printf '\t')" "$A")" 0
# (j) start と report の書き込みは GITHUB_TOKEN で行う(App の token は使わない)。App の鍵は、名義の段と push の段だけ。Issue の close は、push する commit の Closes #N で行う
ok "start: botid・GH_APP_ の secrets・GH_CONFIG_DIR を使わない(報告の App の段は「(u)」で確かめる)" "$(jq -r '[.jobs.start] | map(tostring | test("botid|GH_APP_|GH_CONFIG_DIR")) | join("/")' "$J")" "false"
ok "work: GH_APP_ の secrets を持つ段は、名義の段と push の段だけ" "$(jq -r '[.jobs.work.steps[] | select(tostring | test("secrets[.]GH_APP_")) | .name] | join("|")' "$J")" "名義を設定する(token は消す)|通った変更を main に push する(token はこの段で発行する)"
ok "start: running を付ける段の GH_TOKEN は github.token" "$(jq -r '[.jobs.start.steps[] | select(.name | test("running を付ける")) | .env.GH_TOKEN][0]' "$J")" '${{ github.token }}'
ok "report: 結果を Issue に書く段の GH_TOKEN は github.token" "$(jq -r '[.jobs.report.steps[] | select(.name | test("結果を Issue に書き")) | .env.GH_TOKEN][0]' "$J")" '${{ github.token }}'
ok "work: タスクの段が、題名を RUNNER_TEMP/title.txt に残す(push の段の commit に使う)" "$(jq -r '[.jobs.work.steps[] | select(.name | test("タスク")) | .run] | .[0]' "$J" | grep -q 'title[.]txt' && echo yes)" yes
P=$(jq -r '[.jobs.work.steps[] | select(.name | test("通った変更を main に push")) | .run] | .[0]' "$J")
ok "push: 判定の後に、題名と Closes #\$ISSUE の空の commit を作り、その後に push する" "$(c=$(grep -n 'git commit --allow-empty' <<<"$P" | cut -d: -f1); p=$(grep -n 'git push origin HEAD:main' <<<"$P" | cut -d: -f1); [ -n "$c" ] && [ -n "$p" ] && [ "$c" -lt "$p" ] && grep -qF 'Closes #$ISSUE' <<<"$P" && grep -qF 'title.txt' <<<"$P" && echo yes)" yes
SEDX='s/skip ci|ci skip|no ci|skip actions|actions skip|NO_CI//gI'
ok "push: 題名の空の commit は、deploy を止める印を落としてから作る(Issue の題名に印が在っても、main の deploy は止まらない)" "$(grep -cF "sed -E '$SEDX'" <<<"$P")/$(printf '%s' '題名 [skip ci] [ci skip] [No CI] NO_CI x' | sed -E "$SEDX" | grep -ciE 'skip ci|ci skip|no ci|skip actions|actions skip|NO_CI')" "1/0"
ok "push: 変更が無くても止めない(「変更なし」で終わる経路が無い)" "$(grep -c '変更なし' <<<"$P")" 0
ok "push: commit の名義を書かない(git -c・--author・GIT_AUTHOR・GIT_COMMITTER・git config user を使わない)" "$(grep -cE 'git -c |--author|GIT_AUTHOR|GIT_COMMITTER|git config user' "$A")" 0
ok "push の段は、判定の段より後にあり、if は成功の後で、確認の skip が偽のとき" "$(jq -r '[.jobs.work.steps | to_entries[] | select(.value.name | test("^判定[(]")) | .key] as $j | [.jobs.work.steps | to_entries[] | select(.value.name | test("通った変更を main に push")) | .key] as $p | "\($j[0] < $p[0])/\(.jobs.work.steps[$p[0]].if == "success() && steps.check.outputs.skip != '"'"'true'"'"'")"' "$J")" "true/true"
R=$(jq -r '[.jobs.report.steps[] | select(.name | test("結果を Issue に書き")) | .run] | .[0]' "$J")
ok "report: gh issue close は 1 回だけ、Issue が open のときの if の中にある" "$(grep -c 'gh issue close' <<<"$R")/$(awk '/= open \]; then/{f=1} /else/{f=0} /gh issue close/{print f}' <<<"$R")" "1/1"
ok "report: 閉じられなかったときの注記を、コメントに書く" "$(grep -cF 'close を commit のキーワードで行えなかったので、後続の Issue は起動しない。PM が再評価する' <<<"$R")" 1
ok "report: 成功の段は、running を close の後に外す" "$(awk '/gh issue close/{c=NR} /--remove-label running/{if (!r) r=NR} END{print (c && r && c < r) ? "yes" : "no"}' <<<"$R")" yes
ok "agent.yml・deploy.yml・tools/ に、アカウント ID の値を直書きしていない" "$(acct=$(grep '"account_id"' "$top/app/wrangler.jsonc" 2>/dev/null | cut -d'"' -f4); if [ -n "$acct" ]; then cat "$A" "$top/.github/workflows/deploy.yml" "$top"/tools/*.mjs | grep -cF -- "$acct"; else echo none; fi)" 0
ok "agent.yml・deploy.yml に、マスクの命令(add-mask)が無い(値は隠さず、出さない形に直す)" "$(cat "$A" "$top/.github/workflows/deploy.yml" | grep -c 'add-mask')" 0
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
  if (/^repos\/o\/r\/issues\/\d+$/.test(p) && m === 'GET') { const n = Number(p.split('/').pop()); return out({ number: n, id: n * 10, node_id: `I_${n}` }); }
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
# (i2) 増分の起票(--existing): 既に作った下書き番号 → Issue 番号の表(pm-work/created-issues.tsv の形)で、第 2 波の参照(parent・blocked_by・#{{NNN}})を既存の Issue に解決する
EX=$top/pm-work/created-issues.tsv; EXT=$T/w2-existing.tsv; WD=$T/w2d; WS=$T/w2state.json; EXF=$EXT
cp "$EX" "$EXT"; mkdir -p "$WD"
w2() { printf -- '---\n%s\n---\n\n%s\n' "$2" "$3" > "$WD/$1"; }
w2 041-ext.md 'draft: "041"
title: "W2 1: 既存の親と既存の blocked_by"
type: "Task"
milestone: "M1 テスト(P3)"
parent: "018"
blocked_by: ["015"]
labels: []
size: "S"
priority: "Medium"
project: "signature-studio/3"' '既存の 015 の型(#{{015}})を使う。親は #{{018}}。'
w2 042-batch.md 'draft: "042"
title: "W2 2: 同じ束の 041 の後"
type: "Task"
milestone: "M1 テスト(P3)"
parent: "018"
blocked_by: ["041"]
labels: []
size: "S"
priority: "Medium"
project: "signature-studio/3"' '先行は #{{041}}。'
cr2() { ( cd "$top" && GH_REPO=o/r GHLOG=$CG FX=$CFX FAIL_ON="${FAIL_ON:-}" FAIL_NTH="${FAIL_NTH:-0}" PATH=$T/cbin:$PATH node "$CI" "$@" --drafts "${WDIR:-$WD}" --milestones "$CM" --existing "$EXF" --state "$WS" 2>"$T/cr.err" ); }
: > "$CG"; EXF=$EX cr2 > "$T/w2dry.out"; rc=$?
ok "増分: dry-run は既存の表(実物)を読んで 0 で終わり、gh を呼ばず、状態ファイルも作らない" "$rc/$(wc -l < "$CG" | tr -d ' ')/$([ -e "$WS" ] && echo made || echo none)" "0/0/none"
ok "増分: dry-run の計画に、既存への参照の対応(018 -> #1318・015 -> #1315)を出す" "$(grep -c '018 -> #1318' "$T/w2dry.out")/$(grep -c '015 -> #1315' "$T/w2dry.out")" "1/1"
rm -f "$WS" "$CFX/seq"; : > "$CG"; cr2 --apply > "$T/w2apply.out"; rc=$?
ok "増分: --apply は 0 で終わり、第 2 波の 2 件を作る" "$rc/$(jq -r 'select(.m=="POST" and (.p|endswith("/issues"))) | .body.title' "$CG" | wc -l | tr -d ' ')" "0/2"
ok "増分: parent 018 は既存の #1318 に付く(子の id を付ける)" "$(jq -r 'select(.p|endswith("/sub_issues")) | "\(.p) \(.body.sub_issue_id)"' "$CG" | paste -sd, -)" "repos/o/r/issues/1318/sub_issues 40210,repos/o/r/issues/1318/sub_issues 40240"
ok "増分: blocked_by 015 は既存の #1315 の id(13150)を、同じ束の 041 は新しい id(40210)を付ける" "$(jq -r 'select(.p|endswith("/dependencies/blocked_by")) | "\(.p) \(.body.issue_id)"' "$CG" | paste -sd, -)" "repos/o/r/issues/4021/dependencies/blocked_by 13150,repos/o/r/issues/4024/dependencies/blocked_by 40210"
ok "増分: 本文の #{{015}}・#{{018}} は既存の番号に直り、#{{041}} は新しい番号に直る" "$(jq -r 'select(.m=="PATCH" and .p=="repos/o/r/issues/4021") | .body.body' "$CG")/$(jq -r 'select(.m=="PATCH" and .p=="repos/o/r/issues/4024") | .body.body' "$CG")" "既存の 015 の型(#1315)を使う。親は #1318。/先行は #4021。"
ok "増分: 作った 2 件を、既存の表に追記する(下書き番号・Issue 番号・Type)" "$(awk -F'\t' '$1=="041"||$1=="042"{print $1" "$2" "$4}' "$EXT" | paste -sd, -)" "041 #4021 Task,042 #4024 Task"
ok "増分: 既存の表の行数は、元の行数に 2 を足した数" "$(( $(wc -l < "$EXT") - $(wc -l < "$EX") ))" 2
: > "$CG"; cr2 --apply --resume >/dev/null 2>&1; rc=$?
ok "増分: 完了の後の --resume は、作り直さず、表も増やさずに 1 で止まる(表に在る番号は重複)" "$rc/$(jq -r 'select(.m=="POST" and (.p|endswith("/issues"))) | .body.title' "$CG" | wc -l | tr -d ' ')/$(( $(wc -l < "$EXT") - $(wc -l < "$EX") ))/$(grep -c '作成済み。重複' "$T/cr.err")" "1/0/2/2"
EXF=$T/w2-existing2.tsv; cp "$EX" "$EXF"; rm -f "$WS" "$CFX/seq"; : > "$CG"; FAIL_ON='POST repos/o/r/issues' FAIL_NTH=2 cr2 --apply >/dev/null 2>&1; rc=$?
ok "増分: 途中で落ちた(2 件目の作成)ときは 1 で止まり、表には 1 行も追記しない" "$rc/$(jq -r 'select(.m=="POST" and (.p|endswith("/issues"))) | .body.title' "$CG" | wc -l | tr -d ' ')/$(( $(wc -l < "$EXF") - $(wc -l < "$EX") ))/$(jq '.drafts | keys | join(",")' "$WS")" '1/2/0/"041"'
: > "$CG"; cr2 --apply --resume > /dev/null 2>&1; rc=$?
ok "増分: --resume は作った 041 を作り直さず、残りの 042 だけを作り、表に 2 行追記する" "$rc/$(jq -r 'select(.m=="POST" and (.p|endswith("/issues"))) | .body.title' "$CG" | paste -sd, -)/$(( $(wc -l < "$EXF") - $(wc -l < "$EX") ))" "0/W2 2: 同じ束の 041 の後/2"
UD=$T/w2u; rm -rf "$UD"; cp -r "$WD" "$UD"
printf -- '---\n%s\n---\n\n%s\n' 'draft: "043"
title: "W2 3: 未解決の blocked_by"
type: "Task"
milestone: "M1 テスト(P3)"
parent: "018"
blocked_by: ["099"]
labels: []
size: "S"
priority: "Medium"
project: "signature-studio/3"' '未解決の先行。' > "$UD/043-bad.md"
rm -f "$WS"; : > "$CG"; WDIR=$UD cr2 --apply >/dev/null 2>&1; rc=$?
ok "増分: 未解決の blocked_by(099)は、書く前に 1 で止まり、状態ファイルも作らない" "$rc/$(writes)/$([ -e "$WS" ] && echo made || echo none)" "1/0/none"
ok "増分: 未解決の理由は、下書き番号 099 を挙げる" "$(grep -c '099' "$T/cr.err")" 1
UB=$T/w2b; rm -rf "$UB"; cp -r "$WD" "$UB"
printf -- '---\n%s\n---\n\n%s\n' 'draft: "043"
title: "W2 3: 未解決の本文の参照"
type: "Task"
milestone: "M1 テスト(P3)"
parent: "018"
blocked_by: []
labels: []
size: "S"
priority: "Medium"
project: "signature-studio/3"' '未解決の参照 #{{998}}。' > "$UB/043-bad.md"
rm -f "$WS"; : > "$CG"; WDIR=$UB cr2 --apply >/dev/null 2>&1; rc=$?
ok "増分: 未解決の本文の #{{998}} は、書く前に 1 で止まる" "$rc/$(writes)/$([ -e "$WS" ] && echo made || echo none)" "1/0/none"
UP=$T/w2dup; rm -rf "$UP"; cp -r "$WD" "$UP"
printf -- '---\n%s\n---\n\n%s\n' 'draft: "018"
title: "W2 重複"
type: "Task"
milestone: "M1 テスト(P3)"
parent: null
blocked_by: []
labels: []
size: "S"
priority: "Medium"
project: "signature-studio/3"' '既存の 018 と重複。' > "$UP/018-dup.md"
rm -f "$WS"; : > "$CG"; EXF=$EX WDIR=$UP cr2 --apply >/dev/null 2>&1; rc=$?
ok "増分: 既存の表に在る下書き番号(018)と重複すると、書く前に 1 で止まる" "$rc/$(writes)/$(grep -c '重複' "$T/cr.err")" "1/0/1"
rm -f "$WS"; : > "$CG"; EXF=$T/none.tsv cr2 --apply >/dev/null 2>&1; rc=$?
ok "増分: --existing の表が無ければ、書く前に 1 で止まる" "$rc/$(writes)" "1/0"
# (k) インストーラの一覧(inventory)と外部プラグイン(plugin-external)の段(pm-work/workflow-design.md の「3」「10」)。変更前の agent.yml(BASE)と比べる。
# 新しい段は、対象の Issue のラベルで条件を付けた段だけ。既存の段とジョブの設定は、変更前と一致する
BASE=551a4e59
git -C "$top" show "$BASE:.github/workflows/agent.yml" > "$T/base.yml" 2>/dev/null || echo "FAIL 変更前の agent.yml を読めない($BASE)"
pj "$T/base.yml" > "$T/base.json" 2>>"$T/pj.err"; B=$T/base.json
ok "select: 出力は targets・retry・labels・issue_labels" "$(jq -c '.jobs.select.outputs | keys' "$J")" '["issue_labels","labels","retry","targets"]'
ok "select: labels と issue_labels は pick の段の出力を渡す" "$(jq -r '[.jobs.select.outputs.labels, .jobs.select.outputs.issue_labels] | join(" ")' "$J")" '${{ steps.pick.outputs.labels }} ${{ steps.pick.outputs.issue_labels }}'
ok "select: pick の段が、labels と issue_labels を GITHUB_OUTPUT に書く" "$(jq -r '[.jobs.select.steps[] | select(.id == "pick") | .run] | .[0]' "$J" | grep -cE 'labels=.*jq -r .labels|issue_labels=.*jq -c .issue_labels')" 2
ok "extract: needs は select と start(labels を読む)" "$(jq -c '.jobs.extract.needs' "$J")" '["select","start"]'
ok "work: needs は select・start・extract(issue_labels を読む)" "$(jq -c '.jobs.work.needs | sort' "$J")" '["extract","select","start"]'
NEWS=$(jq -n -S --slurpfile b "$B" --slurpfile n "$J" '
  ([$b[0].jobs.extract.steps[].name] + [$b[0].jobs.work.steps[].name]) as $old
  | [ ($n[0].jobs.extract.steps[] | . + {job: "extract"}), ($n[0].jobs.work.steps[] | . + {job: "work"}) ]
  | map(select((.name) as $x | ($old | any(. == $x)) | not)) | map(select(.name | test("^作業の前に Issue|^作業を飛ばした旨|^前回の作業を取り込む$|^押せなかった作業を保存する$|^判定が落ちた作業を保存する$") | not))')
ok "新しい段: 551a4e59 より後の段は、extract に 17 件、work に 12 件(抽出物の追加の段と、起動用の実行ファイルの段と、実機のビーコンの段を含む)" "$(jq -r 'group_by(.job) | map("\(.[0].job)=\(length)") | join(" ")' <<<"$NEWS")" "extract=17 work=12"
ok "新しい段: すべて、対象の Issue のラベルの判定を if に持つ" "$(jq -r 'map(.["if"] // "" | test("needs[.]select[.]outputs[.](labels|issue_labels)")) | all' <<<"$NEWS")" true
ok "新しい段: extract は labels、work は issue_labels で判定する" "$(jq -r 'map(select(.job == "extract") | .["if"] | test("needs[.]select[.]outputs[.]labels")) | all' <<<"$NEWS")/$(jq -r 'map(select(.job == "work") | .["if"] | test("needs[.]select[.]outputs[.]issue_labels")) | all' <<<"$NEWS")" "true/true"
ok "新しい段: ラベルの判定は、カンマで囲んで行う(部分一致を避ける)" "$(grep -cF "format(',{0},'" "$A")" 29
ok "新しい段: R2 の鍵を使うのは、extract の 5 つ(一覧の作成・外部の取得・言語等の取り出し・内蔵のプラグインの取り出し・起動用の実行ファイルの取り出し)だけ" "$(jq -r '[.[] | select(tostring | contains("R2_ARTIFACT")) | .job] | join(",")' <<<"$NEWS")" "extract,extract,extract,extract,extract"
ok "work: R2_ARTIFACT の鍵が無い(作業ジョブは R2 の鍵を持たない)" "$(jq -r '.jobs.work | tostring | contains("R2_ARTIFACT")' "$J")" false
ok "既存の段: extract・work の既存の段は、name・if・id・uses・with・env・run が変更前と一致する" "$(jq -n -S --slurpfile b "$B" --slurpfile n "$JN" '
  def unskip: if . == null then null else (sub(" && steps[.]check[.]outputs[.]skip != .true."; "") | sub("^success[(][)] && "; "") | if . == "success()" or . == "" then null else . end) end;
  def pick($steps; $names): [$steps[] | select(.name as $x | $names | any(. == $x)) | {name: .name, cond: (.["if"] | unskip), id: .id, uses: .uses, with: .with, env: .env, run: .run}];
  ([$b[0].jobs.extract.steps[].name] + [$b[0].jobs.work.steps[].name] - ["報告を検査して、成果物とサマリーに出す", "通った変更を main に push する(token はこの段で発行する)"]) as $names
  | (pick($b[0].jobs.extract.steps; $names) == pick($n[0].jobs.extract.steps; $names)) and (pick($b[0].jobs.work.steps; $names) == pick($n[0].jobs.work.steps; $names))')" true
ok "既存のジョブ: start・report と、extract・work の needs 以外の設定は、変更前と一致する" "$(jq -n -S --slurpfile b "$B" --slurpfile n "$J" '[($b[0].jobs.extract | del(.steps, .needs)) == ($n[0].jobs.extract | del(.steps, .needs)), ($b[0].jobs.work | del(.steps, .needs, .name, .permissions)) == ($n[0].jobs.work | del(.steps, .needs, .name, .permissions)), $b[0].jobs.start == $n[0].jobs.start, ($b[0].jobs.report | del(.steps)) == ($n[0].jobs.report | del(.steps))] | all')" true
ok "一覧の鍵: 保存と復元の 3 つ(extract の 2 つ・work の 1 つ)が、同じ式" "$(jq -r '[.jobs.extract.steps[], .jobs.work.steps[]] | map(.with.key? // empty) | map(select(startswith("inventory-"))) | "\(length) \(unique | length) \(unique[0])"' "$J")" "3 1 inventory-"'${{ hashFiles('"'ci/artifacts.lock', 'tools/fl-installer-files.mjs', 'app/scripts/lib/*.mjs'"') }}'
ok "外部の鍵: 保存と復元の 3 つ(extract の 2 つ・work の 1 つ)が、同じ式" "$(jq -r '[.jobs.extract.steps[], .jobs.work.steps[]] | map(.with.key? // empty) | map(select(startswith("external-"))) | "\(length) \(unique | length) \(unique[0])"' "$J")" "3 1 external-"'${{ hashFiles('"'ci/artifacts.lock', 'tools/external-inputs/check.mjs'"') }}'
ok "保存の段: runs-on/cache/save が 6 つ。一覧・外部・言語等・内蔵のプラグイン・起動用の実行ファイルの 5 つは取り直したときだけ動き、実機のビーコンの 1 つはラベルの作業のときだけ動く" "$(jq -r '[.jobs.extract.steps[] | select(.uses == "runs-on/cache/save@v5") | .["if"]] | "\(length)/\(map(select(test("cache-hit != .true."))) | length)/\(map(select(test("perf-beacons"))) | length)"' "$J")" "6/5/1"
ok "復元の段: work の R2 の復元は 7 つ(既存の 1、一覧・外部・言語等・内蔵のプラグイン・起動用の実行ファイル・実機のビーコン)、すべてが fail-on-cache-miss を持つ" "$(jq -r '[.jobs.work.steps[] | select(.uses == "runs-on/cache/restore@v5") | .with["fail-on-cache-miss"]] | map(tostring) | join(",")' "$J")" "true,true,true,true,true,true,true"
ok "作業ツリーへの配置は commit しない(inputs/ に置く段に git が無い)" "$(jq -r '[.jobs.work.steps[] | select(.name | test("inputs/ に置く")) | .run] | .[0] // "" | contains("git ")' "$J")" false
ok "node の道具は、一覧と外部の検査で呼ぶ(fl-installer-files と external-inputs/check)" "$(grep -cF 'node tools/fl-installer-files.mjs' "$A")/$(grep -cF 'node tools/external-inputs/check.mjs' "$A")" "1/2"
ok "agent.yml: python を使わない(製品の検査の規則)" "$(grep -ciE 'python' "$A")" 0
AWKX="awk -F'\\t' '\$1 ~ /^installers\\// && \$1 !~ /^installers\\/flstudio_/ {print \$1}' ci/artifacts.lock"
ok "外部の 3 件: ci/artifacts.lock の installers/ の下で FL 以外の行は 3 件(extract の取得と同じ式)" "$(grep -cF "$AWKX" "$A")/$(cd "$top" && eval "$AWKX" | wc -l | tr -d ' ')" "1/3"
ok "外部の 3 件の名前: Serum 2・OTT・Vital を含む(lock の説明の列)" "$(cd "$top" && awk -F'\t' '$1 ~ /^installers\// && $1 !~ /^installers\/flstudio_/ {print $5}' ci/artifacts.lock | paste -sd'|' -)" "Serum 2 2.1.5 (VST3)|OTT 1.37 (VST3)|Vital (VST3)"
fx $R/issues/40 "$(iss 40 open inventory Task)"; fx $R/issues/40/dependencies/blocked_by '[]'; fx $R/issues/40/sub_issues '[]'
fx $R/issues/41 "$(iss 41 open inventory,plugin-external Task)"; fx $R/issues/41/dependencies/blocked_by '[]'; fx $R/issues/41/sub_issues '[]'
fx $R/issues/50 "$(iss 50 closed "" Task)"; fx $R/issues/50/dependencies/blocking '[{"number":40,"state":"open"},{"number":41,"state":"open"}]'
ok "select: 2 件の対象の labels は、重複なしのカンマ区切り。issue_labels は番号ごと(偽のイベントで)" "$(sel issues '{"action":"closed","issue":{"number":50},"sender":'"$W"'}' | jq -c '[.targets, .labels, .issue_labels]' 2>/dev/null)" '[[40,41],"inventory,plugin-external",{"40":"inventory","41":"inventory,plugin-external"}]'
ok "select: ラベルの無い対象は、labels が空の文字列、issue_labels は空の値" "$(sel issues '{"action":"opened","issue":{"number":33},"sender":'"$W"'}' | jq -c '[.labels, .issue_labels]' 2>/dev/null)" '["",{"33":""}]'
ok "select: 起動しない対象だけなら、labels は空(対象が空)" "$(sel issues '{"action":"opened","issue":{"number":20},"sender":'"$W"'}' | jq -c '[.targets, .labels]' 2>/dev/null)" '[[],""]'
# (l) 抽出物の追加(インストーラの言語・テーマ・テンプレート・プロジェクト(resources)と、内蔵のプラグインの DLL(plugin-builtin))。変更前の agent.yml(B3)と比べる。
# 新しい段は、対象の Issue のラベルで条件を付けた段だけ。既存の段とジョブの設定は変更前と一致する。ラベルの無い run では、判定の文字列は ',,' で、どのラベルも含まない(新しい段は skip される)
B3=a7f153b4
git -C "$top" show "$B3:.github/workflows/agent.yml" > "$T/base3.yml" 2>/dev/null || echo "FAIL 変更前の agent.yml を読めない($B3)"
pj "$T/base3.yml" > "$T/base3.json" 2>>"$T/pj.err"; B3J=$T/base3.json
NEWS3=$(jq -n -S --slurpfile b "$B3J" --slurpfile n "$J" '
  ([$b[0].jobs.extract.steps[].name] + [$b[0].jobs.work.steps[].name]) as $old
  | [ ($n[0].jobs.extract.steps[] | . + {job: "extract"}), ($n[0].jobs.work.steps[] | . + {job: "work"}) ]
  | map(select((.name) as $x | ($old | any(. == $x)) | not)) | map(select(.name | test("^作業の前に Issue|^作業を飛ばした旨|^前回の作業を取り込む$|^押せなかった作業を保存する$|^判定が落ちた作業を保存する$") | not))')
ok "抽出物の追加: 新しい段は extract に 11 件、work に 8 件(resources・plugin-builtin・fl-exe の段に、実機のビーコンの 2 と 2 を足した)" "$(jq -r 'group_by(.job) | map("\(.[0].job)=\(length)") | join(" ")' <<<"$NEWS3")" "extract=11 work=8"
ok "抽出物の追加: 新しい段は全て、対象の Issue のラベルの判定を if に持つ" "$(jq -r 'map(.["if"] // "" | test("needs[.]select[.]outputs[.](labels|issue_labels)")) | all' <<<"$NEWS3")" true
ok "抽出物の追加: extract の新しい段は labels、work の新しい段は issue_labels で判定する" "$(jq -r 'map(select(.job == "extract") | .["if"] | test("needs[.]select[.]outputs[.]labels")) | all' <<<"$NEWS3")/$(jq -r 'map(select(.job == "work") | .["if"] | test("needs[.]select[.]outputs[.]issue_labels")) | all' <<<"$NEWS3")" "true/true"
ok "抽出物の追加: resources の段は 5 件、plugin-builtin の段は 5 件(判定に、そのラベルを含む)" "$(jq -r '[(map(.["if"] // "" | select(test(",resources,"))) | length), (map(.["if"] // "" | select(test(",plugin-builtin,"))) | length)] | join("/")' <<<"$NEWS3")" "5/5"
ok "抽出物の追加: 新しい段に App の鍵・token を使う段が無い" "$(jq -r 'map(tostring | test("GH_APP_|GITHUB_TOKEN|github[.]token|GH_TOKEN")) | any' <<<"$NEWS3")" false
cat > "$T/want-res.txt" <<'WANT'
mapfile -t prefixes < ci/resources-prefixes.txt
node tools/fl-installer-extract.mjs "$RUNNER_TEMP/in/flstudio_win64_26.1.7.5653.exe" "$RUNNER_TEMP/extra-resources" "${prefixes[@]}"
WANT
cat > "$T/want-plg.txt" <<'WANT'
mapfile -t prefixes < ci/plugin-builtin-prefixes.txt
node tools/fl-installer-extract.mjs "$RUNNER_TEMP/in/flstudio_win64_26.1.7.5653.exe" "$RUNNER_TEMP/extra-plugins" "${prefixes[@]}"
WANT
cat > "$T/want-key-res.txt" <<'WANT'
extra-resources-${{ hashFiles('ci/artifacts.lock', 'tools/fl-installer-extract.mjs', 'tools/fl-installer-files.mjs', 'app/scripts/lib/*.mjs', 'ci/resources-prefixes.txt') }}
WANT
cat > "$T/want-key-plg.txt" <<'WANT'
extra-plugins-${{ hashFiles('ci/artifacts.lock', 'tools/fl-installer-extract.mjs', 'tools/fl-installer-files.mjs', 'app/scripts/lib/*.mjs', 'ci/plugin-builtin-prefixes.txt') }}
WANT
ok "抽出物の追加: 言語等の取り出しは、接頭辞の一覧(ci/resources-prefixes.txt)を配列で読み、取り出しの道具に渡す(run の行の完全一致 2 行)" "$(jq -r '[.jobs.extract.steps[] | select(.name == "インストーラの言語・テーマ・テンプレート・プロジェクトを取り出す(キャッシュが無いときだけ)") | .run][0]' "$J" | grep -cxF -f "$T/want-res.txt")" 2
ok "抽出物の追加: 内蔵のプラグインの取り出しは、接頭辞の一覧(ci/plugin-builtin-prefixes.txt)を配列で読み、取り出しの道具に渡す(run の行の完全一致 2 行)" "$(jq -r '[.jobs.extract.steps[] | select(.name == "内蔵のプラグインの DLL を取り出す(キャッシュが無いときだけ)") | .run][0]' "$J" | grep -cxF -f "$T/want-plg.txt")" 2
ok "抽出物の追加: 言語等の鍵は、保存・復元の 3 つ(extract の 2 つと work の 1 つ)で同じ式" "$(jq -r '[.jobs.extract.steps[], .jobs.work.steps[]] | map(.with.key? // empty | select(startswith("extra-resources-"))) | "\(length) \(unique | length) \(unique[0])"' "$J")" "3 1 $(cat "$T/want-key-res.txt")"
ok "抽出物の追加: 内蔵のプラグインの鍵は、保存・復元の 3 つ(extract の 2 つと work の 1 つ)で同じ式" "$(jq -r '[.jobs.extract.steps[], .jobs.work.steps[]] | map(.with.key? // empty | select(startswith("extra-plugins-"))) | "\(length) \(unique | length) \(unique[0])"' "$J")" "3 1 $(cat "$T/want-key-plg.txt")"
ok "抽出物の追加: 作業の場所は、RUNNER_TEMP の extra-exe・extra-plugins・extra-resources(取り出し・保存・復元で同じ)" "$(jq -r '[.jobs.extract.steps[], .jobs.work.steps[]] | map(.with.path? // empty | select(test("extra-"))) | unique | join(" ")' "$J")" '${{ runner.temp }}/extra-exe ${{ runner.temp }}/extra-plugins ${{ runner.temp }}/extra-resources'
ok "抽出物の追加: 言語等の復元の段の id は resources、内蔵のプラグインの復元の段の id は plugins" "$(jq -r '[(.jobs.extract.steps[] | select(.name == "インストーラの言語・テーマ・テンプレート・プロジェクトのキャッシュ(R2)") | .id), (.jobs.extract.steps[] | select(.name == "内蔵のプラグインのキャッシュ(R2)") | .id)] | join(" ")' "$J")" "resources plugins"
ok "抽出物の追加: 取り出しと保存の 4 つの段は、cache-hit が偽のときだけ動く(言語等は resources、プラグインは plugins の復元を見る)" "$(jq -r '[.jobs.extract.steps[] | select(.name | test("^インストーラの言語・テーマ・テンプレート・プロジェクト(を取り出す|をキャッシュに保存)|^内蔵のプラグインの DLL を取り出す|^内蔵のプラグインをキャッシュに保存")) | .["if"] | (contains("cache-hit != ") and contains(",resources,") or contains("cache-hit != ") and contains(",plugin-builtin,"))] | "\(length) \(all)"' "$J")" "4 true"
ok "抽出物の追加: 作業の段は、取った物を lift/out-app/host-extra/resources と lift/out-app/host-extra/plugins に置く(mv 2 件)" "$(jq -r '[.jobs.work.steps[] | select(.name | test("^取った物を lift/out-app/host-extra/")) | .run] | map(select(test("mv .*lift/out-app/host-extra/(resources|plugins)"))) | length' "$J")" 2
ok "抽出物の追加: 作業の段の配置は、git を呼ばない(commit は作業者が行う。ここでは置くだけ)" "$(jq -r '[.jobs.work.steps[] | select(.name | test("^取った物を lift/out-app/host-extra/")) | .run] | map(select(contains("git "))) | length' "$J")" 0
ok "抽出物の追加: 作業ツリーの lift/out-app/host-extra は git の対象外(判定の git status を汚さない)" "$(git -C "$top" check-ignore -q lift/out-app/host-extra/resources/x && git -C "$top" check-ignore -q lift/out-app/host-extra/plugins/x && echo yes)" yes
# (p) 起動用の実行ファイル(fl-exe: FL64.exe と FL64 (scaled).exe。根の直下なので、接頭辞でなく完全なパスを --file で取る)。ラベルの Issue だけ、extract で R2 のキャッシュに置き、work で lift/out-app/host-extra/exe に置く(commit はしない)
cat > "$T/want-exe-run.txt" <<'WANT'
node tools/fl-installer-extract.mjs "$RUNNER_TEMP/in/flstudio_win64_26.1.7.5653.exe" "$RUNNER_TEMP/extra-exe" --file 'FL64.exe' --file 'FL64 (scaled).exe'
WANT
cat > "$T/want-key-exe.txt" <<'WANT'
extra-fl-exe-FL64.exe-FL64-scaled.exe-${{ hashFiles('ci/artifacts.lock', 'tools/fl-installer-extract.mjs', 'tools/fl-installer-files.mjs', 'app/scripts/lib/*.mjs') }}
WANT
EXR='起動用の実行ファイルを取り出す(キャッシュが無いときだけ)'; EXRE='起動用の実行ファイルのキャッシュ(R2)'; EXS='起動用の実行ファイルをキャッシュに保存する(取ったときだけ)'
WKR='起動用の実行ファイルを R2 のキャッシュから取る(fl-exe の作業だけ)'; WKP='取った物を lift/out-app/host-extra/exe に置く(fl-exe の作業だけ。commit はしない)'
ok "fl-exe: 2 つの完全なパスは一覧の guest の列に在り、接頭辞 FL64 に当たる物は 2 件だけ" "$(cut -f1 "$top/inputs/fl-installer-files.tsv" | grep -cxF -e 'FL64.exe' -e 'FL64 (scaled).exe')/$(cut -f1 "$top/inputs/fl-installer-files.tsv" | grep -c '^FL64')" "2/2"
ok "fl-exe: 取り出しは、2 つの完全なパスを --file で、取り出しの道具に渡し、extra-exe に書く(行の完全一致)" "$(jq -r --arg n "$EXR" '[.jobs.extract.steps[] | select(.name == $n) | .run][0]' "$J" | grep -cxF -f "$T/want-exe-run.txt")" 1
ok "fl-exe: 取り出しの段の入力は、R2_ARTIFACT の鍵だけ(キャッシュの鍵は使わない)" "$(jq -r --arg n "$EXR" '[.jobs.extract.steps[] | select(.name == $n) | .env.AWS_ACCESS_KEY_ID] | .[0]' "$J")" '${{ secrets.R2_ARTIFACT_ACCESS_KEY_ID }}'
ok "fl-exe: 鍵は、保存・復元の 3 つ(extract の 2 つと work の 1 つ)で同じ式" "$(jq -r '[.jobs.extract.steps[], .jobs.work.steps[]] | map(.with.key? // empty | select(startswith("extra-fl-exe-"))) | "\(length) \(unique | length) \(unique[0])"' "$J")" "3 1 $(cat "$T/want-key-exe.txt")"
ok "fl-exe: 作業の場所は、RUNNER_TEMP の extra-exe(取り出し・保存・復元で同じ)" "$(jq -r '[.jobs.extract.steps[], .jobs.work.steps[]] | map(.with.path? // empty | select(test("extra-exe"))) | unique | join(" ")' "$J")" '${{ runner.temp }}/extra-exe'
ok "fl-exe: 取り出し・保存・復元の 3 段の判定は、fl-exe のラベルを labels で見る(作業の 2 段は issue_labels)" "$(jq -r '[(.jobs.extract.steps[] | select(.name | test("^起動用の実行ファイル")) | .["if"] | select(contains(",fl-exe,") and contains("needs.select.outputs.labels"))), (.jobs.work.steps[] | select(.name | test("fl-exe の作業だけ")) | .["if"] | select(contains(",fl-exe,") and contains("needs.select.outputs.issue_labels")))] | length' "$J")" 5
ok "fl-exe: 取り出しと保存の 2 段は、cache-hit が偽のときだけ動く(復元の段の id は exe)" "$(jq -r --arg e "$EXRE" '[(.jobs.extract.steps[] | select(.name == $e) | .id), ([.jobs.extract.steps[] | select(.name | test("^起動用の実行ファイル(を取り出す|をキャッシュに保存)")) | .["if"] | select(contains("steps.exe.outputs.cache-hit"))] | length | tostring)] | join("/")' "$J")" "exe/2"
ok "fl-exe: 復元の段は、作業の復元で fail-on-cache-miss: true、uses は runs-on/cache/restore" "$(jq -r --arg w "$WKR" '[.jobs.work.steps[] | select(.name == $w) | "\(.uses)/\(.with["fail-on-cache-miss"])"] | .[0]' "$J")" 'runs-on/cache/restore@v5/true'
WRUN=$(jq -r --arg w "$WKP" '[.jobs.work.steps[] | select(.name == $w) | .run][0]' "$J")
ok "fl-exe: 作業の段は mv で extra-exe を lift/out-app/host-extra/exe に置き、2 つのファイルを確かめる(git を呼ばない)" "$(echo "$WRUN" | grep -cF 'mv "$RUNNER_TEMP/extra-exe" lift/out-app/host-extra/exe')/$(echo "$WRUN" | grep -cF 'test -f lift/out-app/host-extra/exe/FL64.exe')/$(echo "$WRUN" | grep -cF 'test -f "lift/out-app/host-extra/exe/FL64 (scaled).exe"')/$(echo "$WRUN" | grep -c 'git ')" "1/1/1/0"
ok "fl-exe: 作業の段の順は、plugins の置く段の後、fl-exe の復元、fl-exe の置く段、取り込みの段の前" "$(jq -r --arg w "$WKR" --arg p "$WKP" '[.jobs.work.steps[].name] as $n | ($n | index("取った物を lift/out-app/host-extra/plugins に置く(plugin-builtin の作業だけ。commit はしない)")) as $q | ($n | index($w)) as $r | ($n | index($p)) as $e | ($n | index("前回の作業を取り込む")) as $t | ($q != null and $r != null and $e != null and $t != null and $q < $r and $r < $e and $e < $t)' "$J")" true
ok "fl-exe: extract の段の順は、内蔵のプラグインの保存の後で、fl-exe の 3 段(復元・取り出し・保存)" "$(jq -r --arg e "$EXRE" --arg x "$EXR" --arg s "$EXS" '[.jobs.extract.steps[].name] as $n | ($n | index("内蔵のプラグインをキャッシュに保存する(取ったときだけ)")) as $p | ($n | index($e)) as $a | ($n | index($x)) as $b | ($n | index($s)) as $c | ($p != null and $a != null and $b != null and $c != null and $p < $a and $a < $b and $b < $c)' "$J")" true
ok "fl-exe: 作業ジョブに R2_ARTIFACT の鍵を使う段が無い(取り出しは extract だけ)" "$(jq -r '[.jobs.work.steps[] | tostring | test("R2_ARTIFACT")] | any' "$J")" false
ok "fl-exe: 作業ツリーの lift/out-app/host-extra/exe は git の対象外" "$(git -C "$top" check-ignore -q lift/out-app/host-extra/exe/FL64.exe && echo yes)" yes
# (r) 接頭辞の一覧は、ci/ の 1 行 1 接頭辞のファイル(resources・plugin-builtin)。extract の取り出しは一覧を読み、鍵の hashFiles に一覧のファイルを入れる(一覧を変えると鍵が変わり、古いキャッシュは使われない)。fl-exe の完全なパスは鍵の文字列に入れる
RPF=$top/ci/resources-prefixes.txt; PPF=$top/ci/plugin-builtin-prefixes.txt
ok "接頭辞(r): ci/resources-prefixes.txt は、この 14 件を、この順で、1 行 1 接頭辞で持つ(Internet の接頭辞と、Blocks の DNC_*.wav の接頭辞を含む。末尾の 6 件は #1386 の Chord progression・Python の実行環境と、#1387 の Riff machine)" "$(paste -sd'|' "$RPF" 2>/dev/null)" 'System\Languages\|Artwork\|Data\Templates\|Data\Demo projects\|Data\System\|Data\Projects\|System\Internet\|Data\Patches\Packs\Legacy\Drums\Dance\|System\Tools\Chord progression\|Shared\Python\|Data\Patches\Scores\Arpeggiator\Riff machine\|Data\Patches\Scores\Quantization\Riff machine\|Data\Patches\Scores\Chopping\Riff machine step 1\|Data\Patches\Scores\Chopping\Riff machine step 2\'
ok "接頭辞(r): ci/plugin-builtin-prefixes.txt は、この 2 件を、この順で、1 行 1 接頭辞で持つ" "$(paste -sd'|' "$PPF" 2>/dev/null)" 'Plugins\Fruity\Effects\|Plugins\Fruity\Generators\'
ok "接頭辞(r): 2 つのファイルは、合わせて 16 行で、\\ で終わる行だけ(空の行と CR が無い)" "$(cat "$RPF" "$PPF" 2>/dev/null | wc -l | tr -d ' ')/$(cat "$RPF" "$PPF" 2>/dev/null | grep -cvE '\\$')/$(cat "$RPF" "$PPF" 2>/dev/null | grep -c $'\r')" "16/0/0"
ok "接頭辞(r): 一覧は、Blocks が使う DNC_*.wav(Data\Patches\Packs\Legacy\Drums\Dance\ の下)を取り出す接頭辞を 1 つ持つ(\\ で終わる行が 1 つ)" "$(grep -cxF 'Data\Patches\Packs\Legacy\Drums\Dance\' "$RPF" 2>/dev/null)" 1
ok "接頭辞(r): その接頭辞の下の DNC_*.wav は、インストーラの一覧で 21 件(接頭辞の取り出しで全件が入る)" "$(grep -cF 'Data\Patches\Packs\Legacy\Drums\Dance\DNC_' "$top/inputs/fl-installer-files.tsv")" 21
# (r2) #1386・#1387 の 6 件: Chord progression(FL 内蔵の Python のスクリプト)・Python の実行環境・Riff machine の資料。件数と大きさは一覧で数える(大きさは dll・exe と Data\ の物だけ)
NEWP=('System\Tools\Chord progression\' 'Shared\Python\' 'Data\Patches\Scores\Arpeggiator\Riff machine\' 'Data\Patches\Scores\Quantization\Riff machine\' 'Data\Patches\Scores\Chopping\Riff machine step 1\' 'Data\Patches\Scores\Chopping\Riff machine step 2\')
INST=$top/inputs/fl-installer-files.tsv
ok "接頭辞(r2): 追加の 6 件は、インストーラの一覧で件数が 59・37・40・9・24・19 件(接頭辞の取り出しで全件が入る)" "$(for p in "${NEWP[@]}"; do P="$p" awk -F'\t' 'NR>2 && index(tolower($1), tolower(ENVIRON["P"]))==1 {n++} END{print n+0}' "$INST"; done | paste -sd/ -)" "59/37/40/9/24/19"
ok "接頭辞(r2): Shared\\Python\\ の dll・exe の大きさの合計は 14,939,080 B(他の 27 件は一覧に大きさが無い。30 MB 以下の根拠)" "$(P='Shared\Python\' awk -F'\t' 'NR>2 && index(tolower($1), tolower(ENVIRON["P"]))==1 && $2!="" {s+=$2} END{print s+0}' "$INST")" 14939080
ok "接頭辞(r2): 追加の Riff machine の 4 件の大きさは、一覧の bytes の合計で 21,119 B" "$(for p in "${NEWP[@]:2}"; do P="$p" awk -F'\t' 'NR>2 && index(tolower($1), tolower(ENVIRON["P"]))==1 && $2!="" {s+=$2} END{print s+0}' "$INST"; done | awk '{s+=$1} END{print s+0}')" 21119
ok "接頭辞(r2): 一覧は、System\\Tools\\・Shared\\・Data\\Patches\\Scores\\ の全体を持たない(必要最小。全体は System\\Tools\\ 419 件・Shared\\ 123 件・Data\\Patches\\Scores\\ 2,334 件)" "$(grep -cxF -e 'System\Tools\' -e 'Shared\' -e 'Data\Patches\Scores\' "$RPF")" 0
ok "接頭辞(r2): 設計の文書の接頭辞の節に、追加の 6 件をそれぞれ 1 行で書いた" "$(for p in "${NEWP[@]}"; do P="$p" awk 'index($0, ENVIRON["P"]) > 0 {n++} END{print (n >= 1)}' "$top/pm-work/workflow-design.md"; done | paste -sd/ -)" "1/1/1/1/1/1"
# (r3) #1387 の Riff machine の実物(一覧の Data\System\Riff machine\ の 8 件)は、接頭辞の一覧の Data\System\ の 1 行で取り出される。Riff machine の接頭辞は足さない(必要最小は 0 件。Data\System\ を消すと、この検査が落ちる)
RIF='Data\System\Riff machine\'
ok "接頭辞(r3): 一覧の Data\System\Riff machine\ は 8 件で、大きさの合計は 17,346 B" "$(P="$RIF" awk -F'\t' 'NR>2 && index($1, ENVIRON["P"])==1 {n++; s+=$2} END{print n+0"/"s+0}' "$INST")" "8/17346"
ok "接頭辞(r3): その 8 件は全て、接頭辞の一覧のいずれかで取り出される(一覧の Data\System\ が、この 8 件を覆う)" "$(RIF="$RIF" awk -F'\t' 'FNR==NR {if ($0 != "") p[++n]=$0; next} index($1, ENVIRON["RIF"])==1 {t++; for (i=1; i<=n; i++) if (index($1, p[i])==1) {c++; break}} END{print c+0"/"t+0}' "$RPF" "$INST")" "8/8"
RRUN=$(jq -r '[.jobs.extract.steps[] | select(.name == "インストーラの言語・テーマ・テンプレート・プロジェクトを取り出す(キャッシュが無いときだけ)") | .run][0] // ""' "$J")
PRUN=$(jq -r '[.jobs.extract.steps[] | select(.name == "内蔵のプラグインの DLL を取り出す(キャッシュが無いときだけ)") | .run][0] // ""' "$J")
ok "接頭辞(r): 言語等の取り出しは、一覧のファイルを読む 1 行と、配列を渡す 1 行で、接頭辞の直書きが無い" "$(grep -cxF 'mapfile -t prefixes < ci/resources-prefixes.txt' <<<"$RRUN")/$(grep -cF '"${prefixes[@]}"' <<<"$RRUN")/$(grep -cE "'[A-Za-z]+[\\]" <<<"$RRUN")" "1/1/0"
ok "接頭辞(r): 内蔵のプラグインの取り出しは、一覧のファイルを読む 1 行と、配列を渡す 1 行で、接頭辞の直書きが無い" "$(grep -cxF 'mapfile -t prefixes < ci/plugin-builtin-prefixes.txt' <<<"$PRUN")/$(grep -cF '"${prefixes[@]}"' <<<"$PRUN")/$(grep -cE "'[A-Za-z]+[\\]" <<<"$PRUN")" "1/1/0"
KR=$(jq -r '[.jobs.extract.steps[], .jobs.work.steps[]] | map(.with.key? // empty | select(startswith("extra-resources-"))) | .[]' "$J")
KP=$(jq -r '[.jobs.extract.steps[], .jobs.work.steps[]] | map(.with.key? // empty | select(startswith("extra-plugins-"))) | .[]' "$J")
KE=$(jq -r '[.jobs.extract.steps[], .jobs.work.steps[]] | map(.with.key? // empty | select(startswith("extra-fl-exe-"))) | .[]' "$J")
ok "接頭辞(r): 言語等の鍵(保存・復元の 3 つ)の hashFiles に、一覧のファイル ci/resources-prefixes.txt が入る" "$(grep -cE "hashFiles[(].*'ci/resources-prefixes[.]txt'[)]" <<<"$KR")/$(wc -l <<<"$KR" | tr -d ' ')" "3/3"
ok "接頭辞(r): 内蔵のプラグインの鍵(保存・復元の 3 つ)の hashFiles に、一覧のファイル ci/plugin-builtin-prefixes.txt が入る" "$(grep -cE "hashFiles[(].*'ci/plugin-builtin-prefixes[.]txt'[)]" <<<"$KP")/$(wc -l <<<"$KP" | tr -d ' ')" "3/3"
EXRUN=$(jq -r --arg n "$EXR" '[.jobs.extract.steps[] | select(.name == $n) | .run][0]' "$J")
EXNAMES=$(grep -oE "[-]-file '[^']*'" <<<"$EXRUN" | sed -E "s/^[-]-file '//; s/'\$//" | tr -d '()' | tr ' ' '-' | paste -sd- -)
ok "接頭辞(r): fl-exe の鍵(保存・復元の 3 つ)の文字列は、--file の 2 つの完全なパスを空白と括弧を除いて - でつないだ物を含む($EXNAMES)" "$(grep -cF "extra-fl-exe-$EXNAMES-" <<<"$KE")/$(wc -l <<<"$KE" | tr -d ' ')" "3/3"
ok "接頭辞(r): 設計の文書に、一覧のファイルと鍵の関係を書いた" "$(grep -c 'ci/resources-prefixes[.]txt' "$top/pm-work/workflow-design.md" | awk '{print ($1 >= 1)}')/$(grep -c 'ci/plugin-builtin-prefixes[.]txt' "$top/pm-work/workflow-design.md" | awk '{print ($1 >= 1)}')" "1/1"
EORD='["外部プラグインのインストーラをキャッシュに保存する(取ったときだけ)","インストーラの言語・テーマ・テンプレート・プロジェクトのキャッシュ(R2)","インストーラの言語・テーマ・テンプレート・プロジェクトを取り出す(キャッシュが無いときだけ)","インストーラの言語・テーマ・テンプレート・プロジェクトをキャッシュに保存する(取ったときだけ)","内蔵のプラグインのキャッシュ(R2)","内蔵のプラグインの DLL を取り出す(キャッシュが無いときだけ)","内蔵のプラグインをキャッシュに保存する(取ったときだけ)"]'
WORD='["抽出物を lift/out-app/host に置く","言語・テーマ・テンプレート・プロジェクトを R2 のキャッシュから取る(resources の作業だけ)","取った物を lift/out-app/host-extra/resources に置く(resources の作業だけ。commit はしない)","内蔵のプラグインを R2 のキャッシュから取る(plugin-builtin の作業だけ)","取った物を lift/out-app/host-extra/plugins に置く(plugin-builtin の作業だけ。commit はしない)","タスク(Issue の本文と題名)を取り、受け入れコマンドを取り出す"]'
ok "抽出物の追加: extract の段の順は、外部の保存の後に、言語等、内蔵のプラグイン" "$(jq -r --argjson o "$EORD" '[.jobs.extract.steps[].name] as $n | [$o[] as $x | $n | index($x)] | (all(. != null)) and (. == (sort))' "$J")" true
ok "抽出物の追加: work の段の順は、lift/out-app/host の後に、言語等と内蔵のプラグインを置き、タスクの前" "$(jq -r --argjson o "$WORD" '[.jobs.work.steps[].name] as $n | [$o[] as $x | $n | index($x)] | (all(. != null)) and (. == (sort))' "$J")" true
ok "既存の段: extract・work の既存の段は、name・if・id・uses・with・env・run が a7f153b4 と一致する" "$(jq -n -S --slurpfile b "$B3J" --slurpfile n "$JN" '
  def unskip: if . == null then null else (sub(" && steps[.]check[.]outputs[.]skip != .true."; "") | sub("^success[(][)] && "; "") | if . == "success()" or . == "" then null else . end) end;
  def pick($steps; $names): [$steps[] | select(.name as $x | $names | any(. == $x)) | {name: .name, cond: (.["if"] | unskip), id: .id, uses: .uses, with: .with, env: .env, run: .run}];
  ([$b[0].jobs.extract.steps[].name] + [$b[0].jobs.work.steps[].name] - ["報告を検査して、成果物とサマリーに出す", "通った変更を main に push する(token はこの段で発行する)"]) as $names
  | (pick($b[0].jobs.extract.steps; $names) == pick($n[0].jobs.extract.steps; $names)) and (pick($b[0].jobs.work.steps; $names) == pick($n[0].jobs.work.steps; $names))')" true
ok "既存のジョブ: select・start・report と、extract・work の needs 以外の設定は a7f153b4 と一致する" "$(jq -n -S --slurpfile b "$B3J" --slurpfile n "$J" '[($b[0].jobs.extract | del(.steps, .needs)) == ($n[0].jobs.extract | del(.steps, .needs)), ($b[0].jobs.work | del(.steps, .needs, .name, .permissions)) == ($n[0].jobs.work | del(.steps, .needs, .name, .permissions)), $b[0].jobs.select == $n[0].jobs.select, $b[0].jobs.start == $n[0].jobs.start, ($b[0].jobs.report | del(.steps)) == ($n[0].jobs.report | del(.steps))] | all')" true
for n in fl-installer-files external-inputs gen-wave-plugins fl-installer-extract plugin-rows auto-retry; do node "$here/$n.test.mjs" > "$T/$n.out" 2>&1; ok "試験: spec/$n.test.mjs が通る" "$?" 0; done
# (n) 同じ Issue の直列化、作業の前の Issue の状態の確認、報告の伏せ字、報告の判定(閉じて main に入った作業は、失敗の印を付けない)。偽の gh と fixture で、段の実際の動きを確かめる
v1=anthr; v2=opic
ok "concurrency: workflow の最上位が、Issue の番号ごとの group で、cancel-in-progress は false" "$(jq -r '.concurrency | "\(.group)/\(.["cancel-in-progress"] | tostring)"' "$J")" 'agent-${{ github.event.issue.number }}/false'
ok "作業の前の確認: work の 2 番目の段(リポジトリ取得の次)が id check" "$(jq -r '.jobs.work.steps[1].id' "$J")" check
ok "作業の前の確認: token は github.token だけ(secrets を使わない)" "$(jq -r '.jobs.work.steps[1].env | tostring | ((test("secrets") | not) and test("github[.]token"))' "$J")" true
ok "作業の前の確認: skip は GITHUB_OUTPUT に書く(GITHUB_ENV は使わない)" "$(jq -r '.jobs.work.steps[1].run' "$J" | grep -cF 'GITHUB_OUTPUT')/$(jq -r '.jobs.work.steps[1].run' "$J" | grep -c 'GITHUB_ENV')" "1/0"
NGT="steps.check.outputs.skip != 'true'"
ok "作業の段: 確認より後の段(飛ばした旨を除く)は、全て success() か always() で、skip でないときの条件を持つ(例外 0)" "$(jq -r --arg g "$NGT" '[.jobs.work.steps[2:][] | select(.name | test("飛ばした旨|押せなかった作業を保存する|判定が落ちた作業を保存する") | not) | (.["if"] // "") | select((contains($g) and (startswith("success() && ") or startswith("always() && "))) | not)] | length' "$J")" 0
ok "作業の段: 確認より後の段は 20 以上ある(空振りでない)" "$(jq -r '[.jobs.work.steps[2:][] | select(.name | test("飛ばした旨") | not)] | length > 20' "$J")" true
ok "作業の段: always() の段は 2 つ(報告の検査・成果物の保存)で、どちらも skip でないときの条件を持つ" "$(jq -r --arg g "$NGT" '[.jobs.work.steps[] | select((.["if"] // "") | startswith("always()")) | (.["if"] | contains($g))] | "\(length)/\(all)"' "$J")" "2/true"
ok "作業の段: 飛ばした旨の段は、成功の後で、確認の skip が真のときだけ動く" "$(jq -r '[.jobs.work.steps[] | select(.name | test("飛ばした旨")) | .["if"]] | .[0]' "$J")" "success() && steps.check.outputs.skip == 'true'"
RP=$(jq -r '[.jobs.work.steps[] | select(.name == "報告を検査して、成果物とサマリーに出す") | .run][0] // ""' "$J")
ok "報告の検査: --redact で伏せ字にし、伏せ字の件数を ::warning:: で出す(値は出さない)" "$(grep -cF -- '--redact "$RUNNER_TEMP/out/report.md"' <<<"$RP")/$(grep -c '::warning::' <<<"$RP")" "1/1"
ok "報告の検査: pipefail を持つ(伏せ字の後の失敗を隠さない)" "$(grep -c 'set -o pipefail' <<<"$RP")" 1
NFX=$T/nfx; NFB=$T/nfbin; NGL=$T/ngh.log; NGB=$T/ngh-bodies.txt; mkdir -p "$NFX" "$NFB"; : > "$NGL"; : > "$NGB"
cat > "$NFB/gh" <<'NGH'
#!/usr/bin/env bash
echo "$*" >> "$NGL"
case "$1" in
  api)
    shift; p=""; jqf=""
    while [ $# -gt 0 ]; do
      case "$1" in
        --jq) jqf=$2; shift ;;
        -*) ;;
        *) [ -z "$p" ] && p=$1 ;;
      esac
      shift
    done
    f="$NFX/$(printf '%s' "${p%%\?*}" | sed 's#/#__#g').json"
    [ -f "$f" ] || { echo "fake gh: 無い: $p" >&2; exit 1; }
    if [ -n "$jqf" ]; then jq -r "$jqf" "$f"; else cat "$f"; fi
    exit $? ;;
  run)
    n=""; d=""; shift 2
    while [ $# -gt 0 ]; do case "$1" in -n) n=$2; shift ;; -D) d=$2; shift ;; esac; shift; done
    src="$NFX/art-${n#agent-}"; [ -d "$src" ] || exit 1
    cp -r "$src"/. "$d"/; exit $? ;;
  issue)
    sub=$2; num=$3; shift 3; bf=""
    while [ $# -gt 0 ]; do case "$1" in --body-file) bf=$2; shift ;; esac; shift; done
    if [ -n "$bf" ]; then echo "BODY $num $sub" >> "$NGB"; cat "$bf" >> "$NGB"; fi
    exit 0 ;;
  *) echo "fake gh: 未対応: $*" >&2; exit 1 ;;
esac
NGH
chmod +x "$NFB/gh"
nfx() { printf '%s' "$2" > "$NFX/repos__o__r__issues__$1.json"; }
nfx 11 '{"number":11,"state":"closed","labels":[]}'
nfx 12 '{"number":12,"state":"open","labels":[]}'
nfx 13 '{"number":13,"state":"open","labels":[]}'
nfx 14 '{"number":14,"state":"open","labels":[]}'
nfx 15 '{"number":15,"state":"closed","labels":[]}'
nfx 21 '{"number":21,"state":"open","labels":[]}'
nfx 22 '{"number":22,"state":"closed","labels":[]}'
nfx 23 '{"number":23,"state":"open","labels":[{"name":"failed"}]}'
nfx 24 '{"number":24,"state":"open","labels":[{"name":"render"},{"name":"pm-hold"}]}'
CHK=$(jq -r '[.jobs.work.steps[] | select(.id == "check") | .run][0] // ""' "$J"); printf '%s\n' "$CHK" > "$T/nchk.sh"
nchk() { rm -f "$T/nchk-out"; ( cd "$T" && env PATH="$NFB:$PATH" NFX="$NFX" NGL="$NGL" NGB="$NGB" GH_TOKEN=x GH_REPO=o/r ISSUE=$1 RUNNER_TEMP="$T" GITHUB_OUTPUT="$T/nchk-out" bash -e "$T/nchk.sh" > "$T/nchk.log" 2>&1; echo "$? $(grep '^skip=' "$T/nchk-out" 2>/dev/null)" ); }
ok "作業の前の確認: 開いていて、failed・pm-hold が無ければ、作業する(skip=false)" "$(nchk 21)" "0 skip=false"
ok "作業の前の確認: 閉じていれば、作業を飛ばす(skip=true)" "$(nchk 22)" "0 skip=true"
ok "作業の前の確認: failed が付いていれば、作業を飛ばす(skip=true)" "$(nchk 23)" "0 skip=true"
ok "作業の前の確認: pm-hold が付いていれば、作業を飛ばす(skip=true)" "$(nchk 24)" "0 skip=true"
ok "作業の前の確認: Issue を読めなければ失敗する(作業の判定を出さない)" "$(nchk 25)" "1 "
R1=$(jq -r '[.jobs.report.steps[] | select(.name | test("結果を集める")) | .run][0] // ""' "$J")
R2=$(jq -r '[.jobs.report.steps[] | select(.name | test("結果を Issue に書き")) | .run][0] // ""' "$J")
SKN=$(jq -r '[.jobs.work.steps[] | select(.name | test("飛ばした旨")) | .name][0] // "無し"' "$J")
CKN=$(jq -r '[.jobs.work.steps[] | select(.id == "check") | .name][0] // "無し"' "$J")
SCN="報告を検査して、成果物とサマリーに出す"
jq -n --arg sk "$SKN" --arg ck "$CKN" --arg sc "$SCN" '{jobs: [
  {name: "起動する Issue を決める", conclusion: "success", steps: []},
  {name: "作業 #11", conclusion: "failure", steps: [{name: $ck, conclusion: "success"}, {name: $sc, conclusion: "failure"}]},
  {name: "作業 #12", conclusion: "failure", steps: [{name: $ck, conclusion: "success"}, {name: "作業者を 1 回動かす", conclusion: "failure"}]},
  {name: "作業 #13", conclusion: "success", steps: [{name: $ck, conclusion: "success"}, {name: $sc, conclusion: "success"}]},
  {name: "作業 #14", conclusion: "success", steps: [{name: $ck, conclusion: "success"}, {name: $sk, conclusion: "success"}]},
  {name: "作業 #15", conclusion: "failure", steps: [{name: $ck, conclusion: "success"}, {name: $sc, conclusion: "failure"}]}
]}' > "$NFX/repos__o__r__actions__runs__77__jobs.json"
mkdir -p "$NFX/art-11" "$NFX/art-13" "$NFX/art-15"; printf '0123abc\n' > "$NFX/art-11/commit.txt"; printf '4567def\n' > "$NFX/art-13/commit.txt"; printf 'diff\n' > "$NFX/art-15/work.diff"
printf '%s\n' "$R1" > "$T/nrep1.sh"; printf '%s\n' "$R2" > "$T/nrep2.sh"
nrun() { ( cd "$top" && env PATH="$NFB:$PATH" NFX="$NFX" NGL="$NGL" NGB="$NGB" GH_TOKEN=x GH_REPO=o/r RUN_ID=77 RUN_URL=https://example.test/run/77 SERVER=https://example.test CLAIMED='[11,12,13,14,15]' RUNNER_TEMP="$T/nrt" GITHUB_OUTPUT="$T/ngho" bash -e "$T/$1" > "$T/$1.log" 2>&1; echo $? ); }
mkdir -p "$T/nrt"
ok "報告の 1 段目(結果を集める)が、0 で終わる" "$(nrun nrep1.sh)" 0
ok "報告の 2 段目(結果を書き、状態を変える)が、0 で終わる" "$(nrun nrep2.sh)" 0
nb() { awk -v n="$1" '$0 ~ ("^BODY " n " ") {f=1; next} /^BODY /{f=0} f' "$NGB"; }
ok "報告(閉じて main に入った失敗): 失敗の印 failed を付けない" "$(grep -c '^issue edit 11 .*add-label failed' "$NGL")" 0
ok "報告(閉じて main に入った失敗): 作業が main に入ったと書き、失敗した段の名前を出す" "$(nb 11 | grep -cF "作業は完了し main に入った。後続の段($SCN)が失敗した")" 1
ok "報告(閉じて main に入った失敗): 閉じているので close はせず、running は外す" "$(grep -c '^issue close 11' "$NGL")/$(grep -c '^issue edit 11 .*remove-label running' "$NGL")" "0/1"
ok "報告(open のまま失敗): failed を付け、running を外す" "$(grep -c '^issue edit 12 .*add-label failed' "$NGL")/$(grep -c '^issue edit 12 .*remove-label running' "$NGL")" "1/1"
ok "報告(open のまま失敗): 作業が main に入ったとは書かない" "$(nb 12 | grep -cF '作業は完了し')" 0
ok "報告(成功): 通った旨を書き、開いていれば close し、running を外す" "$(grep -c '^issue close 13' "$NGL")/$(nb 13 | grep -cF '通った')/$(grep -c '^issue edit 13 .*remove-label running' "$NGL")" "1/1/1"
ok "報告(飛ばした): 何も書かず、failed も付けず、running だけ外す" "$(grep -c '^BODY 14 ' "$NGB")/$(grep -c '^issue edit 14 .*add-label failed' "$NGL")/$(grep -c '^issue edit 14 .*remove-label running' "$NGL")" "0/0/1"
ok "報告(閉じていて、main に入った記録が無い失敗): failed を付け、作業が入ったとは書かない" "$(grep -c '^issue edit 15 .*add-label failed' "$NGL")/$(nb 15 | grep -cF '作業は完了し')" "1/0"
ok "報告(落ちた・保存した作業あり): 作業を保存したと書く" "$(nb 15 | grep -cF '作業を保存した')" 1
ok "報告(落ちた・保存した作業なし): 保存したとは書かない" "$(nb 12 | grep -cF '作業を保存した')" 0
SCD=$T/nscan; SRT=$T/nscan-rt; mkdir -p "$SCD/guard" "$SRT/out"; cp "$top/guard/scan.sh" "$SCD/guard/scan.sh"; printf '%s\n' "$RP" > "$T/nscan.sh"
printf 'by %s%s\n' "$v1" "$v2" > "$SRT/out/report.md"
( cd "$SCD" && env RUNNER_TEMP="$SRT" ISSUE=5 GITHUB_STEP_SUMMARY="$SRT/summary.md" bash -e "$T/nscan.sh" > "$T/nscan.log" 2>&1; echo $? > "$T/nscan.rc" )
ok "報告の検査(実行): ベンダー名の文字列だけなら通り、報告を伏せ字にし、サマリーに出す" "$(cat "$T/nscan.rc")/$(grep -c "$v1$v2" "$SRT/out/report.md")/$(grep -c '^### 作業 #5$' "$SRT/summary.md")/$(grep -c '::warning::' "$T/nscan.log")" "0/0/1/1"
hx=0123456789abcdef; hx="$hx$hx"; printf 'id %s\nby %s%s\n' "$hx" "$v1" "$v2" > "$SRT/out/report.md"; rm -f "$SRT/summary.md"
( cd "$SCD" && env RUNNER_TEMP="$SRT" ISSUE=5 GITHUB_STEP_SUMMARY="$SRT/summary.md" bash -e "$T/nscan.sh" > "$T/nscan.log" 2>&1; echo $? > "$T/nscan.rc" )
ok "報告の検査(実行): ベンダー名以外(cfhex)が残れば失敗し、サマリーには出さない" "$(cat "$T/nscan.rc")/$(grep -c "$v1$v2" "$SRT/out/report.md")/$(test -e "$SRT/summary.md" && echo made || echo none)" "1/0/none"
# (o) 押せなかった作業の保存(push の段が落ちたとき)と、再試行での前回の作業の取り込み。段の順・条件・権限は JSON で、動きは偽の gh とローカルの git で確かめる
SVN="押せなかった作業を保存する"; PRN="前回の作業を取り込む"
ok "保存の段: 名前が 1 つ。push の段(id push)の後で、報告の検査の前に在る" "$(jq -r --arg s "$SVN" '[.jobs.work.steps[] | .id // ""] as $ids | [.jobs.work.steps[] | .name] as $n | ($n | index($s)) as $s0 | ($ids | index("push")) as $p | ($n | index("報告を検査して、成果物とサマリーに出す")) as $r | "\($n | map(select(. == $s)) | length) \($p != null and $s0 != null and $r != null and $p < $s0 and $s0 < $r)"' "$J")" "1 true"
ok "保存の段の if: 判定が通り push が落ちたとき、または時間切れで取り消されたときだけ(判定の失敗では保存しない。確認の skip も偽)" "$(jq -r --arg s "$SVN" '[.jobs.work.steps[] | select(.name == $s) | .["if"]] | .[0]' "$J")" "(failure() || cancelled()) && steps.push.conclusion == 'failure' && steps.check.outputs.skip != 'true'"
ok "push の段: id は push" "$(jq -r '[.jobs.work.steps[] | select(.name | test("通った変更を main に push")) | .id] | .[0]' "$J")" push
PSH=$(jq -r '[.jobs.work.steps[] | select(.name | test("通った変更を main に push")) | .run] | .[0]' "$J")
ok "push の段: 変更前(551a4e59)の run の行は、残る。削除されるのは題名の印を落とす commit の行の書き換えだけ(それ以外の行の削除・書き換えが無い)" "$(diff <(jq -r '[.jobs.work.steps[] | select(.name | test("通った変更を main に push")) | .run] | .[0]' "$B") <(printf '%s\n' "$PSH") | awk '/^</ && !/git commit --allow-empty/ && NF > 1' | wc -l | tr -d ' ')" 0
ok "push の段: env は変更前と一致する(App の鍵は、この段と名義の段だけ)" "$(jq -n -S --slurpfile b "$B" --slurpfile n "$J" '[$b[0].jobs.work.steps[] | select(.name | test("通った変更を main に push")) | .env] == [$n[0].jobs.work.steps[] | select(.name | test("通った変更を main に push")) | .env]')" true
ok "push の段: push 前の HEAD を RUNNER_TEMP/work-tip.txt に、空の commit を作る前に残す(pull の後の HEAD は保存の基点にしない)" "$(t=$(grep -n 'git rev-parse HEAD > "$RUNNER_TEMP/work-tip.txt"' <<<"$PSH" | cut -d: -f1); c=$(grep -n 'git commit --allow-empty' <<<"$PSH" | cut -d: -f1); [ -n "$t" ] && [ -n "$c" ] && [ "$t" -lt "$c" ] && echo yes)" yes
ok "取り込みの段: 名前が 1 つ。名義の段の後で、作業者の段の前に在る" "$(jq -r --arg p "$PRN" '[.jobs.work.steps[] | .name] as $n | ($n | index($p)) as $i | ($n | index("名義を設定する(token は消す)")) as $g | ($n | index("作業者を 1 回動かす")) as $w | "\($n | map(select(. == $p)) | length) \($g != null and $i != null and $w != null and $g < $i and $i < $w)"' "$J")" "1 true"
ok "取り込みの段の if: 作業の前の確認が通った(skip が偽)ときだけ" "$(jq -r --arg p "$PRN" '[.jobs.work.steps[] | select(.name == $p) | .["if"]] | .[0]' "$J")" "success() && steps.check.outputs.skip != 'true'"
ok "取り込みの段の env: GH_TOKEN は github.token で、secrets を使わない" "$(jq -r --arg p "$PRN" '[.jobs.work.steps[] | select(.name == $p) | .env] | .[0] | "\(.GH_TOKEN)/\(tostring | test("secrets"))"' "$J")" '${{ github.token }}/false'
ok "取り込みの段の位置: キャッシュの復元(runs-on/cache/restore の全て)と、取った物の置く段の全てより後で、作業者の段の前に在る(鍵は取り込み前の main のファイルで計算される)" "$(jq -r --arg p "$PRN" '
  [.jobs.work.steps[].name] as $n
  | ($n | index($p)) as $i
  | ($n | index("作業者を 1 回動かす")) as $w
  | [.jobs.work.steps | to_entries[] | select((.value.uses // "") | startswith("runs-on/cache/restore")) | .key] as $r
  | [.jobs.work.steps | to_entries[] | select((.value.name // "") | test("^(抽出物を lift/out-app/host に置く|一覧を作業ツリーの inputs/ に置く|外部プラグインを lift/out-app/external に置き|取った物を lift/out-app/host-extra/)")) | .key] as $p
  | "\($r | length)/\($p | length)/\(($i != null) and ($w != null) and (((($r + $p) | max) < $i)) and ($i < $w))"' "$J")" "7/7/true"
ok "取り込みの段の位置: 取った物の置く段(plugin-builtin)の後で、タスクの段の前に在る" "$(jq -r --arg p "$PRN" '[.jobs.work.steps[].name] as $n | ($n | index($p)) as $i | ($n | index("取った物を lift/out-app/host-extra/plugins に置く(plugin-builtin の作業だけ。commit はしない)")) as $q | ($n | index("タスク(Issue の本文と題名)を取り、受け入れコマンドを取り出す")) as $t | ($i != null and $q != null and $t != null and $q < $i and $i < $t)' "$J")" true
ok "作業者の段に env が無い(取り込みの token を渡さない)" "$(jq -r --arg n "作業者を 1 回動かす" '[.jobs.work.steps[] | select(.name == $n) | has("env")] | .[0] | tostring' "$J")" false
ok "work の権限: actions が read(取り込みの段が、保存された artifact を読む)。contents と issues は read のまま" "$(jq -r '.jobs.work.permissions | "\(.actions)/\(.contents)/\(.issues)"' "$J")" "read/read/read"
ok ".gitignore に .prior-work.md が在る(取り込みの記述は commit しない)" "$(grep -cxF '.prior-work.md' "$top/.gitignore")" 1
ok "AGENTS.md: 作業者の手順に、.prior-work.md を最初に読む旨が 1 行在る" "$(awk '/^## 作業者の手順/{f=1;next} /^## /{f=0} f' "$top/AGENTS.md" | grep -c '[.]prior-work[.]md')" 1
ok "AGENTS.md: 作業者の手順に、ターンを終えない規則が 1 行在る" "$(awk '/^## 作業者の手順/{f=1;next} /^## /{f=0} f' "$top/AGENTS.md" | grep -c 'ターンを終えるのは、受け入れが通って commit した時か、続けられない理由を報告に書く時だけ')" 1
ok "AGENTS.md: 作業者の手順に、長いコマンドを前景で行い timeout 引数を渡す旨が 1 行在る" "$(awk '/^## 作業者の手順/{f=1;next} /^## /{f=0} f' "$top/AGENTS.md" | grep -c 'フォアグラウンドで実行し、シェルツールの')" 1
ok "AGENTS.md: 作業者の手順に、受け入れが通らなくても作業の最後に作業ツリーを commit する旨が 1 行在る" "$(awk '/^## 作業者の手順/{f=1;next} /^## /{f=0} f' "$top/AGENTS.md" | grep -c '受け入れが通らなくても、作業の最後に、作業ツリーを commit する')" 1
SVS=$T/save.sh; printf '%s\n' "$(jq -r --arg s "$SVN" '[.jobs.work.steps[] | select(.name == $s) | .run] | .[0] // ""' "$J")" > "$SVS"
SVD=$T/save; mkdir -p "$SVD"
mkrepo() { rm -rf "$1"; git init -q -b main "$1"; printf '[user]\n\tname = spec\n\temail = spec@example.invalid\n' >> "$1/.git/config"; printf 'l1\nl2\nl3\nl4\nl5\n' > "$1/a.txt"; git -C "$1" add a.txt; git -C "$1" commit -qm base; git -C "$1" update-ref refs/judge/main main; }
sv() { # 引数: リポジトリ、RUNNER_TEMP。標準出力に終了コード
  mkdir -p "$2"; ( cd "$1" && env RUNNER_TEMP="$2" bash -eo pipefail "$SVS" > "$2.log" 2>&1; echo $? )
}
S1=$SVD/s1; mkrepo "$S1"; git -C "$S1" checkout -q -b agent; sed -i '1s/.*/T1/' "$S1/a.txt"; printf 'new\n' > "$S1/b.txt"; git -C "$S1" add -A; git -C "$S1" commit -qm agent-work; A1=$(git -C "$S1" rev-parse HEAD)
git -C "$S1" checkout -q -b m main; sed -i '3s/.*/M3/' "$S1/a.txt"; git -C "$S1" commit -qam main-moved; git -C "$S1" checkout -q agent; git -C "$S1" merge -q --no-edit m >/dev/null
R1=$SVD/rt1; mkdir -p "$R1"; printf '%s\n' "$A1" > "$R1/work-tip.txt"
ok "保存: push 前の HEAD から、pull で取り込んだ main の変更(3 行目の M3)を除いた差分を work.diff に残す" "$(rc=$(sv "$S1" "$R1"); echo "$rc/$(cat "$R1/out/work.diff" 2>/dev/null | grep -c '^+M3')/$(cat "$R1/out/work.diff" 2>/dev/null | grep -c '^+T1')/$(cat "$R1/out/work.diff" 2>/dev/null | grep -c '^+new')")" "0/0/1/1"
S2=$SVD/s2; mkrepo "$S2"; R2=$SVD/rt2; mkdir -p "$R2"; git -C "$S2" rev-parse main > "$R2/work-tip.txt"
ok "保存: 差分が空(作業が無い)なら work.diff を作らず、その旨を出す" "$(rc=$(sv "$S2" "$R2"); echo "$rc/$(test -e "$R2/out/work.diff" && echo made || echo none)/$(grep -c '空' "$R2.log")")" "0/none/1"
R3=$SVD/rt3; S3=$SVD/s3; mkrepo "$S3"; mkdir -p "$R3"
ok "保存: push の段が始まる前に落ちた(work-tip.txt が無い)なら、何もせずに 0 で終わる" "$(rc=$(sv "$S3" "$R3"); echo "$rc/$(test -e "$R3/out/work.diff" && echo made || echo none)")" "0/none"
S4=$SVD/s4; mkrepo "$S4"; git -C "$S4" checkout -q -b agent; head -c 44000000 /dev/urandom > "$S4/big.bin"; git -C "$S4" add -A; git -C "$S4" commit -qm big; R4=$SVD/rt4; mkdir -p "$R4"; git -C "$S4" rev-parse HEAD > "$R4/work-tip.txt"
ok "保存: 差分が 50 MB を超えるなら作らず、警告を出す" "$(rc=$(sv "$S4" "$R4"); echo "$rc/$(test -e "$R4/out/work.diff" && echo made || echo none)/$(grep -c '::warning::' "$R4.log")")" "0/none/1"
PRS=$T/prior.sh; printf '%s\n' "$(jq -r --arg p "$PRN" '[.jobs.work.steps[] | select(.name == $p) | .run] | .[0] // ""' "$J")" > "$PRS"
PBIN=$T/pbin; PLOG=$T/pgh.log; mkdir -p "$PBIN"
cat > "$PBIN/gh" <<'PGH'
#!/usr/bin/env bash
echo "$*" >> "$PLOG"
case "$1" in
  api) shift; jqf=""; while [ $# -gt 0 ]; do case "$1" in --jq) jqf=$2; shift ;; esac; shift; done
       if [ -n "$jqf" ]; then jq -r "$jqf" "$FIX/list.json"; else cat "$FIX/list.json"; fi ;;
  run) rid=$3; d=""; shift 3; while [ $# -gt 0 ]; do case "$1" in -D) d=$2; shift ;; esac; shift; done
       [ -d "$FIX/dl-$rid" ] || exit 1; cp -r "$FIX/dl-$rid"/. "$d"/ ;;
  *) echo "fake gh: 未対応: $*" >&2; exit 1 ;;
esac
PGH
chmod +x "$PBIN/gh"
PV=$T/pv; mkrepo "$PV"; git -C "$PV" checkout -q -b agent; sed -i '1s/.*/T1/' "$PV/a.txt"; printf 'new\n' > "$PV/b.txt"; git -C "$PV" add -A; git -C "$PV" commit -qm prior-work; git -C "$PV" diff --binary main agent > "$T/diff901"
pf() { mkdir -p "$1"; printf '%s' "$2" > "$1/list.json"; }
prun() { # 引数: リポジトリ、fixture の dir、RUNNER_TEMP。標準出力に終了コード
  mkdir -p "$3"; : > "$PLOG"; ( cd "$1" && env PATH="$PBIN:$PATH" FIX="$2" PLOG="$PLOG" GH_TOKEN=x GH_REPO=o/r ISSUE=5 RUNNER_TEMP="$3" bash -eo pipefail "$PRS" > "$3.log" 2>&1; echo $? )
}
LIST901='{"artifacts":[{"id":1,"expired":false,"created_at":"2026-11-20T00:00:00Z","workflow_run":{"id":901}}]}'
FC=$T/fc; pf "$FC" "$LIST901"; mkdir -p "$FC/dl-901"; cp "$T/diff901" "$FC/dl-901/work.diff"; printf 'judge: 通った\n' > "$FC/dl-901/judge.txt"
CR=$T/pc; mkrepo "$CR"; sed -i '5s/.*/R5/' "$CR/a.txt"; git -C "$CR" commit -qam main-moved; n0=$(git -C "$CR" rev-list --count HEAD)
rc=$(prun "$CR" "$FC" "$T/rc1")
ok "取り込み(衝突なし): 段は 0 で終わり、前回の作業を commit 1 つで載せ、.prior-work.md に「衝突なく」と書く" "$rc/$(( $(git -C "$CR" rev-list --count HEAD) - n0 ))/$(grep -c '衝突なく' "$CR/.prior-work.md" 2>/dev/null)" "0/1/1"
ok "取り込み(衝突なし): 前回の変更(1 行目 T1・b.txt)と、main の変更(5 行目 R5)が両方在る" "$(sed -n 1p "$CR/a.txt")/$(sed -n 5p "$CR/a.txt")/$(test -f "$CR/b.txt" && echo yes)" "T1/R5/yes"
ok "取り込み(衝突なし): commit の題名は「前回の作業を取り込む(run 901)」で、期限内の artifact(run 901)を取る" "$(git -C "$CR" log -1 --format=%s)/$(grep -c 'run download 901' "$PLOG")" "前回の作業を取り込む(run 901)/1"
CX=$T/px; mkrepo "$CX"; sed -i '1s/.*/X1/' "$CX/a.txt"; git -C "$CX" commit -qam main-moved; n0=$(git -C "$CX" rev-list --count HEAD)
rc=$(prun "$CX" "$FC" "$T/rx1")
ok "取り込み(衝突): 段は 0 で終わり、commit は増えず、衝突の印が a.txt に残る" "$rc/$(( $(git -C "$CX" rev-list --count HEAD) - n0 ))/$(grep -c '^<<<<<<<' "$CX/a.txt")" "0/0/1"
ok "取り込み(衝突): .prior-work.md に衝突したファイル(a.txt)と、解消して commit する旨を書く" "$(grep -c 'a[.]txt' "$CX/.prior-work.md")/$(grep -c '衝突' "$CX/.prior-work.md")/$(grep -c '解消' "$CX/.prior-work.md")" "1/1/1"
ok "取り込み(衝突): 未解決のファイルは a.txt だけ(git の unmerged)" "$(git -C "$CX" diff --name-only --diff-filter=U | paste -sd, -)" "a.txt"
CN=$T/pn; mkrepo "$CN"; FN=$T/fn; pf "$FN" '{"artifacts":[]}'
rc=$(prun "$CN" "$FN" "$T/rn1")
ok "取り込み(保存なし): artifact が無ければ、何もせずに 0 で終わる(.prior-work.md も作らず、download もしない)" "$rc/$(test -e "$CN/.prior-work.md" && echo made || echo none)/$(grep -c 'run download' "$PLOG")" "0/none/0"
CE=$T/pe; mkrepo "$CE"; sed -i '5s/.*/R5/' "$CE/a.txt"; git -C "$CE" commit -qam main-moved; FE=$T/fe; pf "$FE" '{"artifacts":[{"id":2,"expired":true,"created_at":"2026-11-21T00:00:00Z","workflow_run":{"id":902}}]}'; mkdir -p "$FE/dl-902"; cp "$T/diff901" "$FE/dl-902/work.diff"
rc=$(prun "$CE" "$FE" "$T/re1")
ok "取り込み(期限切れ): 期限切れの artifact しか無ければ、取らずに 0 で終わる" "$rc/$(test -e "$CE/.prior-work.md" && echo made || echo none)/$(grep -c 'run download' "$PLOG")" "0/none/0"
CF=$T/pf; mkrepo "$CF"; sed -i '5s/.*/R5/' "$CF/a.txt"; git -C "$CF" commit -qam main-moved; FF=$T/ff; pf "$FF" '{"artifacts":[{"id":1,"expired":false,"created_at":"2026-11-20T00:00:00Z","workflow_run":{"id":901}},{"id":3,"expired":false,"created_at":"2026-11-22T00:00:00Z","workflow_run":{"id":903}}]}'; mkdir -p "$FF/dl-901" "$FF/dl-903"; cp "$T/diff901" "$FF/dl-901/work.diff"; printf 'judge: 通った\n' > "$FF/dl-901/judge.txt"; printf 'x\n' > "$FF/dl-903/judge.txt"
rc=$(prun "$CF" "$FF" "$T/rf1")
ok "取り込み(新しい方に差分が無い): 新しい順に見て、差分が在る 901 を取る(903 を先に試す)" "$rc/$(grep -c 'run 901' "$CF/.prior-work.md" 2>/dev/null)/$(awk '/^run download/{print $3}' "$PLOG" | paste -sd, -)" "0/1/903,901"
# (p) 判定が落ちた作業の保存(判定の段が落ちて push が飛んだとき)と、判定を通らなかった作業の続きからの取り込み(commit せず作業ツリーに載せる)
JSN="判定が落ちた作業を保存する"; JS=$T/jsave.sh; printf '%s\n' "$(jq -r --arg s "$JSN" '[.jobs.work.steps[] | select(.name == $s) | .run] | .[0] // ""' "$J")" > "$JS"
ok "判定が落ちた作業の保存: 名前が 1 つ。押せなかった作業の保存の後で、報告の検査の前に在る" "$(jq -r --arg s "$JSN" '[.jobs.work.steps[] | .name] as $n | ($n | index("押せなかった作業を保存する")) as $a | ($n | index($s)) as $b | ($n | index("報告を検査して、成果物とサマリーに出す")) as $c | "\($n | map(select(. == $s)) | length) \($a != null and $b != null and $c != null and $a < $b and $b < $c)"' "$J")" "1 true"
ok "判定が落ちた作業の保存の if: 判定の段で落ちた、または時間切れで取り消されて push が飛んだときだけ(確認の skip も偽)" "$(jq -r --arg s "$JSN" '[.jobs.work.steps[] | select(.name == $s) | .["if"]] | .[0]' "$J")" "(failure() || cancelled()) && steps.push.conclusion == 'skipped' && steps.check.outputs.skip != 'true'"
ok "判定が落ちた作業の保存: env を持たない(token を渡さない)" "$(jq -r --arg s "$JSN" '[.jobs.work.steps[] | select(.name == $s) | has("env")] | .[0] | tostring' "$J")" false
JR=$SVD/jr; jrepo() { rm -rf "$1"; git init -q -b main "$1"; printf '[user]\n\tname = spec\n\temail = spec@example.invalid\n' >> "$1/.git/config"; cp "$top/.gitignore" "$1/.gitignore"; printf 'l1\nl2\nl3\nl4\nl5\n' > "$1/a.txt"; git -C "$1" add -A; git -C "$1" commit -qm base; git -C "$1" update-ref refs/judge/main main; }
J1=$JR/a; jrepo "$J1"; git -C "$J1" checkout -q -b agent; sed -i '1s/.*/T1/' "$J1/a.txt"; git -C "$J1" commit -qam committed-work
git -C "$J1" checkout -q main; sed -i '5s/.*/R5/' "$J1/a.txt"; git -C "$J1" commit -qam main-moved; git -C "$J1" update-ref refs/judge/main main; git -C "$J1" checkout -q agent
sed -i '2s/.*/U2/' "$J1/a.txt"; printf 'new\n' > "$J1/b.txt"; mkdir -p "$J1/lift/out-app" "$J1/target"; printf 'x\n' > "$J1/lift/out-app/big.bin"; printf 'x\n' > "$J1/target/t.o"; printf 'p\n' > "$J1/.prior-work.md"; printf 'x\n' > "$J1/.tmp-x.txt"
JA=$JR/rt-a; mkdir -p "$JA/out"; : > "$JA/out/report.md"
jsv() { ( cd "$1" && env RUNNER_TEMP="$2" bash -eo pipefail "$JS" > "$2.log" 2>&1; echo $? ); }
rc=$(jsv "$J1" "$JA"); D=$JA/out/work.diff
ok "判定が落ちた作業の保存: 終了コード 0 で、work.diff を残す" "$rc/$(test -s "$D" && echo made)" "0/made"
ok "判定が落ちた作業の保存: commit 済みの変更(T1)・未 commit の変更(U2)・未追跡(b.txt)を含む" "$(grep -c '^+T1$' "$D")/$(grep -c '^+U2$' "$D")/$(grep -c '^+new$' "$D")" "1/1/1"
ok "判定が落ちた作業の保存: .gitignore の物(lift/out-app・target・.prior-work.md・.tmp-*)と .gitignore 自体を含まない(差分の見出しは 2 件)" "$(grep -c '^diff --git' "$D")/$(grep -cE 'big[.]bin|t[.]o|prior-work|tmp-x|a/[.]gitignore' "$D")" "2/0"
ok "判定が落ちた作業の保存: 基点は refs/judge/main との merge-base(main が進んだ後の R5 を含まない)" "$(grep -c 'R5' "$D")" 0
JE=$JR/e; jrepo "$JE"; JEA=$JR/rt-e; mkdir -p "$JEA/out"; : > "$JEA/out/report.md"
rc=$(jsv "$JE" "$JEA"); ok "判定が落ちた作業の保存: 差分が空なら作らず、その旨を出す" "$rc/$(test -e "$JEA/out/work.diff" && echo made || echo none)/$(grep -c '空' "$JEA.log")" "0/none/1"
JN=$JR/rt-n; mkdir -p "$JN/out"; rc=$(jsv "$J1" "$JN"); ok "判定が落ちた作業の保存: 作業者が動いていなければ(report.md が無ければ)作らない" "$rc/$(test -e "$JN/out/work.diff" && echo made || echo none)" "0/none"
FJ=$JR/fj; pf "$FJ" '{"artifacts":[{"id":950,"expired":false,"created_at":"2026-11-23T00:00:00Z","workflow_run":{"id":950}}]}'; mkdir -p "$FJ/dl-950"; cp "$D" "$FJ/dl-950/work.diff"; { printf 'judge: 受け入れコマンドが失敗した\n'; printf '%300s\n' '' | tr ' ' x; } > "$FJ/dl-950/judge.txt"
CJ=$JR/cj; mkrepo "$CJ"; sed -i '5s/.*/R5/' "$CJ/a.txt"; git -C "$CJ" commit -qam main-moved; n0=$(git -C "$CJ" rev-list --count HEAD); h0=$(git -C "$CJ" rev-parse HEAD)
rc=$(prun "$CJ" "$FJ" "$JR/rj1")
ok "取り込み(判定が落ちた): 段は 0 で終わり、commit は増えず、HEAD も変わらない" "$rc/$(( $(git -C "$CJ" rev-list --count HEAD) - n0 ))/$(test "$(git -C "$CJ" rev-parse HEAD)" = "$h0" && echo same)" "0/0/same"
ok "取り込み(判定が落ちた): 前回の変更(1 行目 T1・2 行目 U2・b.txt)と、main の変更(5 行目 R5)が作業ツリーに在る" "$(sed -n 1p "$CJ/a.txt")/$(sed -n 2p "$CJ/a.txt")/$(sed -n 5p "$CJ/a.txt")/$(test -f "$CJ/b.txt" && echo yes)" "T1/U2/R5/yes"
ok "取り込み(判定が落ちた): 取り込んだ変更は未 commit で、作業ツリーに載っている(git status に在る)" "$(test -n "$(git -C "$CJ" status --porcelain)" && echo yes)" yes
ok "取り込み(判定が落ちた): .prior-work.md に、判定を通らなかった旨・理由・続きから行う旨を書き、「衝突なく」は書かない" "$(grep -c '判定を通らなかった' "$CJ/.prior-work.md")/$(grep -c '受け入れコマンドが失敗した' "$CJ/.prior-work.md")/$(grep -c '続きから' "$CJ/.prior-work.md")/$(grep -c '衝突なく' "$CJ/.prior-work.md")" "1/1/1/0"
ok "取り込み(判定が落ちた): 理由の行は 200 字で切る(300 字の行が 200 字になる)" "$(grep -c '^x\{200\}$' "$CJ/.prior-work.md")/$(grep -c 'x\{201\}' "$CJ/.prior-work.md")" "1/0"
CX=$JR/cx; mkrepo "$CX"; sed -i '1s/.*/X1/' "$CX/a.txt"; git -C "$CX" commit -qam main-moved; n0=$(git -C "$CX" rev-list --count HEAD)
rc=$(prun "$CX" "$FJ" "$JR/rj2")
ok "取り込み(判定が落ちた・衝突): 段は 0 で終わり、commit は増えず、衝突の印が a.txt に残る" "$rc/$(( $(git -C "$CX" rev-list --count HEAD) - n0 ))/$(grep -c '^<<<<<<<' "$CX/a.txt")" "0/0/1"
ok "取り込み(判定が落ちた・衝突): .prior-work.md に衝突したファイル(a.txt)と、解消する旨を書く" "$(grep -c 'a[.]txt' "$CX/.prior-work.md")/$(grep -c '解消' "$CX/.prior-work.md")" "1/1"
FJ2=$JR/fj2; pf "$FJ2" "$(cat "$FJ/list.json")"; mkdir -p "$FJ2/dl-950"; cp "$D" "$FJ2/dl-950/work.diff"
CN=$JR/cn; mkrepo "$CN"; sed -i '5s/.*/R5/' "$CN/a.txt"; git -C "$CN" commit -qam main-moved; n0=$(git -C "$CN" rev-list --count HEAD)
rc=$(prun "$CN" "$FJ2" "$JR/rj3")
ok "取り込み(判定の出力が無い): 判定に至らなかった作業として、commit せずに取り込み、その旨を書く" "$rc/$(( $(git -C "$CN" rev-list --count HEAD) - n0 ))/$(grep -c '判定に至らなかった' "$CN/.prior-work.md")" "0/0/1"
# (k) 本番のデプロイ(deploy.yml)の抽出の木の鍵は、作業ジョブの抽出の木の鍵(agent.yml の extract-tree-)が読む入力を全て覆う。覆わないと、表を変えても古い木が使われる
pj "$top/.github/workflows/deploy.yml" > "$T/deploy.json" 2>>"$T/pj.err"; DJ=$T/deploy.json
sed -e "s/, 'lift\/plugins\/[^']*'//" "$top/.github/workflows/deploy.yml" > "$T/deploy-noplg.yml"; pj "$T/deploy-noplg.yml" > "$T/deploy-noplg.json" 2>>"$T/pj.err"; DJN=$T/deploy-noplg.json
DK=$(jq -r '[.jobs.deploy.steps[] | select(.id == "cache") | .with.key][0]' "$DJ")
AK=$(jq -r '[.jobs.extract.steps[], .jobs.work.steps[]] | map(.with.key? // empty) | map(select(startswith("extract-tree-"))) | unique[0]' "$J")
cover() { python3 - "$top" "$1" "$2" <<'PY'
import glob, os, re, sys
os.chdir(sys.argv[1])
def files(key):
    s = set()
    for args in re.findall(r"hashFiles\(([^)]*)\)", key):
        for pat in re.findall(r"'([^']*)'", args):
            s |= {os.path.normpath(f) for f in glob.glob(pat, recursive=True) if os.path.isfile(f)}
    return s
a = files(sys.argv[2]); d = files(sys.argv[3])
miss = sorted(a - d)
print(" ".join(miss) if miss else ("covered" if a else "empty"))
sys.exit(0 if a and not miss else 1)
PY
}
ok "deploy.yml: 抽出の木の鍵(id: cache)は host- で、seed 表(lift/harness/seeds/vfs-seed-idle1.tsv)を入力に含む" "$(case "$DK" in host-*"'lift/harness/seeds/vfs-seed-idle1.tsv'"*) echo yes;; *) echo no;; esac)" yes
ok "deploy.yml: 抽出の木の鍵は、agent.yml の extract-tree- の鍵が読む入力(ファイルの集合)を全て覆う(覆われていない物の一覧)" "$(cover "$AK" "$DK")" covered
ok "plugins: agent.yml の抽出の木の鍵(保存・復元の 2 つ)と deploy.yml の host の鍵が、lift/plugins の plugin.tsv を入力に含む" "$(grep -cF "'lift/plugins/*/plugin.tsv'" "$A")/$(grep -cF "'lift/plugins/**'" "$top/.github/workflows/deploy.yml")" "2/1"
ok "plugins: 旧版との比較の前に、鍵から lift/plugins の 1 項目を除くと、lift/plugins の文字が残らない" "$(grep -c 'lift/plugins' "$T/new-noplg.yml")/$(grep -c 'lift/plugins' "$T/deploy-noplg.yml")" "0/0"
ok "plugins: extract-fl.mjs は lift/plugins の行を lib/plugin-rows.mjs で読む(import と呼び出しの 2 行)" "$(grep -c 'pluginRows' "$top/app/scripts/extract-fl.mjs")" 2
ok "deploy.yml: 抽出の木の鍵は、fe-build extract が読む表(ci/extract-dlls.tsv・seed 表)と extract-fl.mjs を全て覆う" "$(cover "hashFiles('ci/extract-dlls.tsv', 'lift/harness/seeds/vfs-seed-idle1.tsv', 'app/scripts/extract-fl.mjs')" "$DK")" covered
ok "deploy.yml: 抽出の木の鍵は、インストーラの名前と抽出の引数(fe-build の main.rs)と、extract-fl.mjs が repo から読む os-fonts の 4 件を覆う" "$(cover "hashFiles('lift/crates/fe-build/src/main.rs', 'lift/harness/seeds/os-fonts/**')" "$DK")" covered
# (m) 本番へ上げる前のスモーク(deploy.yml): 起動の確認と、Welcome 窓の 4 つの流れ。上げる段(upload・wrangler)より前に在り、既定の success() で、落ちれば後を飛ばす
D=$top/.github/workflows/deploy.yml; DOLD=cf3e7dc9
git -C "$top" show "$DOLD:.github/workflows/deploy.yml" > "$T/deploy-old.yml" 2>/dev/null || echo "FAIL 変更前の deploy.yml を読めない($DOLD)"
pj "$T/deploy-old.yml" > "$T/deploy-old.json" 2>>"$T/pj.err"; DO=$T/deploy-old.json
SMOKE='boot-bundle|welcome-flow|jq -e'
KINDS='[.jobs.deploy.steps[] | (.run // "") as $r | if .uses then (if (.uses | test("wrangler")) then "wrangler" else "uses" end) elif ($r | test("fe-build bundle$")) then "bundle" elif ($r | test("boot-bundle[.]mjs")) then "boot" elif ($r | test("jq -e") and test("boot[.]json")) then "bootgate" elif ($r | test("welcome-flow[.]mjs")) then "flow" elif ($r | test("jq -e") and test("flow[.]jsonl")) then "flowgate" elif ($r | test("cmp ")) then "readback" elif ($r | test("fe-build upload$")) then "upload" else "other" end] | map(select(. != "other" and . != "uses")) | join(" ")'
ok "deploy.yml(m): 順は fe-build bundle → 起動の確認 → 起動の判定 → Welcome 窓の流れ → 流れの判定 → fe-build upload → 本番の manifest の確認 → wrangler" "$(jq -r "$KINDS" "$DJ")" "bundle boot bootgate flow flowgate upload readback wrangler"
ok "deploy.yml(m): 起動の確認と Welcome 窓の流れの段は、それぞれ 1 つ" "$(jq '[.jobs.deploy.steps[] | select((.run // "") | test("boot-bundle[.]mjs"))] | length' "$DJ")/$(jq '[.jobs.deploy.steps[] | select((.run // "") | test("welcome-flow[.]mjs"))] | length' "$DJ")" "1/1"
ok "deploy.yml(m): スモークの段に if が無い(既定の success(). 落ちれば後の段を飛ばす)" "$(jq "[.jobs.deploy.steps[] | select(((.run // \"\") | test(\"$SMOKE\"))) | has(\"if\")] | any" "$DJ")" false
ok "deploy.yml(m): 起動の確認と流れの段は時間の上限(timeout-minutes)を持つ" "$(jq '[.jobs.deploy.steps[] | select(((.run // "") | test("boot-bundle|welcome-flow"))) | has("timeout-minutes")] | length == 2 and all' "$DJ")" true
ok "deploy.yml(m): スモークの段は env を持たない(鍵と token を渡さない)" "$(jq "[.jobs.deploy.steps[] | select(((.run // \"\") | test(\"$SMOKE\"))) | has(\"env\")] | any" "$DJ")" false
ok "deploy.yml(m): どの段にも continue-on-error が無い" "$(jq '[.jobs.deploy.steps[] | has("continue-on-error")] | any' "$DJ")" false
ok "deploy.yml(m): スモークと本番の manifest の確認の段以外の段と job の設定と on は、変更前(cf3e7dc9)と一致する(抽出の木の鍵の式と、resources・bundle の段は除く)" "$(jq -n -S --slurpfile n "$DJN" --slurpfile o "$DO" --arg re '^(本番の manifest を確かめる|resources を取り出す|bundle の中身を確かめる)$' --arg sm "$SMOKE" 'def nk: map(if .id == "cache" then del(.with.key) else . end); ([$n[0].jobs.deploy.steps[] | select(((.run // "") | test($sm)) | not) | select((.name // "") | test($re) | not)] | nk) == ($o[0].jobs.deploy.steps | nk) and ($n[0].jobs.deploy | del(.steps)) == ($o[0].jobs.deploy | del(.steps)) and $n[0].on == $o[0].on')" true
ok "deploy.yml(m): guard/deploy-lint.sh が deploy.yml を通す" "$(bash "$top/guard/deploy-lint.sh" "$D" > /dev/null 2>&1; echo $?)" 0
ok "deploy.yml(m): スモークが読む bundle は、fe-build の upload が読む場所と同じ(raw/fl-Oz)" "$(grep -cF '"$PWD/lift/out-app/bundle/raw/fl-Oz"' "$D")/$(grep -cF 'OUT_APP_DIR}/bundle/raw/fl-Oz/manifest.json' "$top/lift/crates/fe-build/src/main.rs")" "2/1"
BG=$(jq -r '[.jobs.deploy.steps[] | select(((.run // "") | test("jq -e")) and ((.run // "") | test("boot[.]json"))) | .run] | .[0] // ""' "$DJ")
bg() { mkdir -p "$T/rt"; printf '%s\n' "$1" > "$T/rt/boot.json"; ( cd "$T" && RUNNER_TEMP="$T/rt" bash -c "$BG" > /dev/null 2>&1 ); echo $?; }
ok "deploy.yml(m): 起動の判定: idle・standIn 偽・error 無しで通る" "$(bg '{"error":null,"state":"engine-idle","standIn":false}')" 0
ok "deploy.yml(m): 起動の判定: error が有れば落ちる" "$(bg '{"error":"x","state":"engine-idle","standIn":false}')" 1
ok "deploy.yml(m): 起動の判定: error が null でも idle でなければ落ちる(boot-bundle の error は idle の証拠ではない)" "$(bg '{"error":null,"state":"error","standIn":false}')" 1
ok "deploy.yml(m): 起動の判定: standIn が真なら落ちる" "$(bg '{"error":null,"state":"engine-idle","standIn":true}')" 1
FG=$(jq -r '[.jobs.deploy.steps[] | select(((.run // "") | test("jq -e")) and ((.run // "") | test("flow[.]jsonl"))) | .run] | .[0] // ""' "$DJ")
fg() { mkdir -p "$T/rt"; printf '%s\n' "$1" > "$T/rt/flow.jsonl"; ( cd "$T" && RUNNER_TEMP="$T/rt" bash -c "$FG" > /dev/null 2>&1 ); echo $?; }
FL=$(for f in A B C D; do jq -c -n --arg f $f '{flow:$f,error:null,state:"engine-idle",standIn:false,trapped:false,hostErrors:0,welcomeClosed:true,opened:true}'; done)
ok "deploy.yml(m): 流れの判定: 4 つ(A B C D)が全て正常なら通る" "$(fg "$FL")" 0
ok "deploy.yml(m): 流れの判定: 1 つでも trap していれば落ちる" "$(fg "$(jq -c 'if .flow == "B" then .trapped = true else . end' <<<"$FL")")" 1
ok "deploy.yml(m): 流れの判定: 1 つに error が有れば落ちる" "$(fg "$(jq -c 'if .flow == "C" then .error = "x" else . end' <<<"$FL")")" 1
ok "deploy.yml(m): 流れの判定: 1 つで開かなければ落ちる(開いたか)" "$(fg "$(jq -c 'if .flow == "D" then .opened = false else . end' <<<"$FL")")" 1
ok "deploy.yml(m): 流れの判定: 1 つで窓が閉じなければ落ちる" "$(fg "$(jq -c 'if .flow == "A" then .welcomeClosed = false else . end' <<<"$FL")")" 1
ok "deploy.yml(m): 流れの判定: 1 つで idle でなければ落ちる" "$(fg "$(jq -c 'if .flow == "A" then .state = "error" else . end' <<<"$FL")")" 1
ok "deploy.yml(m): 流れの判定: 流れが 3 つだけなら落ちる" "$(fg "$(head -n 3 <<<"$FL")")" 1
ok "deploy.yml(m): 流れの判定: 流れの名前が重複すれば落ちる(D が無い)" "$(fg "$(jq -c 'if .flow == "D" then .flow = "A" else . end' <<<"$FL")")" 1
ok "deploy.yml(m): 流れの判定: 出力が空なら落ちる" "$(fg "")" 1
ok "deploy.yml(m): 流れの判定: HostFilesUnsupported の件数は判定に入れない(documented: 正常時の件数は未確認)" "$(fg "$(jq -c 'if .flow == "A" then .hostErrors = 3 else . end' <<<"$FL")")" 0
# (q) 時間切れ(取り消し)でも作業を保存する。保存の段は、通信をせず、取り消しで残った index.lock を外してから差分を作る。報告は、取り消された作業を failed にし、保存の有無を書く。取り込みは、判定の出力が無い作業を、判定の前の取り消しとして続きから扱う。AGENTS.md に、長いコマンドと 5 時間の規則が在る
ok "取り消し: 保存の 2 段は、通信をしない(git fetch・gh を呼ばない。猶予の約 5 分で終わる)" "$(for s in "$SVN" "$JSN"; do jq -r --arg s "$s" '[.jobs.work.steps[] | select(.name == $s) | .run] | .[0]' "$J" | grep -cE 'git fetch|gh '; done | paste -sd/)" "0/0"
ok "取り消し: 判定が落ちた作業の保存は、取り消しで残った index.lock を外してから git add -A する" "$(jq -r --arg s "$JSN" '[.jobs.work.steps[] | select(.name == $s) | .run] | .[0]' "$J" | grep -n 'rm -f .git/index.lock\|git add -A' | cut -d: -f2 | paste -sd' ' -)" "rm -f .git/index.lock git add -A"
RPT=$(jq -r '[.jobs.report.steps[] | select(.name | test("結果を集める")) | .run][0] // ""' "$J")
ok "取り消し: 報告の結果を集める段は、取り消された作業を、時間切れで取り消された旨として書く" "$(grep -c '時間切れで取り消された' <<<"$RPT")" 1
NCF=$T/ncfx; NCL=$T/ncgh.log; NCB=$T/ncgh-bodies.txt; NCRT=$T/ncrt; mkdir -p "$NCF" "$NCRT"; : > "$NCL"; : > "$NCB"
printf '%s' '{"number":16,"state":"open","labels":[]}' > "$NCF/repos__o__r__issues__16.json"
printf '%s' '{"number":17,"state":"open","labels":[]}' > "$NCF/repos__o__r__issues__17.json"
jq -n --arg ck "$CKN" '{jobs: [
  {name: "起動する Issue を決める", conclusion: "success", steps: []},
  {name: "作業 #16", conclusion: "cancelled", steps: [{name: $ck, conclusion: "success"}, {name: "作業者を 1 回動かす", conclusion: "cancelled"}]},
  {name: "作業 #17", conclusion: "cancelled", steps: [{name: $ck, conclusion: "success"}, {name: "作業者を 1 回動かす", conclusion: "cancelled"}]}
]}' > "$NCF/repos__o__r__actions__runs__78__jobs.json"
mkdir -p "$NCF/art-16"; printf 'diff\n' > "$NCF/art-16/work.diff"
ncrun() { ( cd "$top" && env PATH="$NFB:$PATH" NFX="$NCF" NGL="$NCL" NGB="$NCB" GH_TOKEN=x GH_REPO=o/r RUN_ID=78 RUN_URL=https://example.test/run/78 SERVER=https://example.test CLAIMED='[16,17]' RUNNER_TEMP="$NCRT" GITHUB_OUTPUT="$T/ncgho" bash -e "$T/$1" > "$T/$1.nc.log" 2>&1; echo $? ); }
ncr=$(ncrun nrep1.sh)/$(ncrun nrep2.sh)
ncb() { awk -v n="$1" '$0 ~ ("^BODY " n " ") {f=1; next} /^BODY /{f=0} f' "$NCB"; }
ok "取り消し(報告): 2 段とも 0 で終わる" "$ncr" "0/0"
ok "取り消し(報告・保存あり): 失敗の印 failed を付け、running を外し、close はしない" "$(grep -c '^issue edit 16 .*add-label failed' "$NCL")/$(grep -c '^issue edit 16 .*remove-label running' "$NCL")/$(grep -c '^issue close 16' "$NCL")" "1/1/0"
ok "取り消し(報告・保存あり): 時間切れで取り消された旨と、作業を保存した旨を書く(保存できなかった旨は書かない)" "$(ncb 16 | grep -cF '時間切れで取り消された')/$(ncb 16 | grep -cF '作業を保存した')/$(ncb 16 | grep -cF '作業を保存できなかった')" "1/1/0"
ok "取り消し(報告・保存なし): 失敗の印 failed を付け、running を外す" "$(grep -c '^issue edit 17 .*add-label failed' "$NCL")/$(grep -c '^issue edit 17 .*remove-label running' "$NCL")" "1/1"
ok "取り消し(報告・保存なし): 時間切れで取り消された旨と、作業を保存できなかった旨を書く(保存した旨は書かない)" "$(ncb 17 | grep -cF '時間切れで取り消された')/$(ncb 17 | grep -cF '作業を保存できなかった')/$(ncb 17 | grep -cF '作業を保存した')" "1/1/0"
ok "取り消し(取り込み): 判定の出力が無い作業は、判定の前に取り消された(時間切れ)旨を、続きから行う理由に書く" "$(grep -c '判定の前に取り消された(時間切れ)' "$CN/.prior-work.md")" 1
A2=$(awk '/^## 作業者の手順/{f=1;next} /^## /{f=0} f' "$top/AGENTS.md")
ok "取り消し(AGENTS): 作業者の手順に、長いコマンドを背景で起動して終了を待たない旨が 1 行在る" "$(grep -c 'プロセスの終了を待つ' <<<"$A2")/$(grep -c 'フォアグラウンドで、直接実行する' <<<"$A2")" "1/1"
ok "取り消し(AGENTS): timeout 引数の上限は 最大 5400000(90 分)で、0 や、それ以上にしない" "$(grep -c '最大 5400000(90 分)' <<<"$A2")" 1
ok "取り消し(AGENTS): 作業は開始から 5 時間以内に終え、4 時間 30 分を過ぎたら commit して終える" "$(grep -c '5 時間以内に終える' <<<"$A2")/$(grep -c '4 時間 30 分を過ぎたら' <<<"$A2")/$(grep -c 'date -u' <<<"$A2")" "1/1/1"
ok "取り消し(AGENTS): 旧い規則(timeout の 0 で無効・背景の完了を同じターンで待つ)を残さない" "$(grep -c '0 で無効\|同じターンの中で、出力のファイルを読んで確かめる' <<<"$A2")" 0
ok "取り消し(設計): 時間切れの保存と、待ちの固まりの規則を、設計の文書に書いた" "$(grep -cE '時間切れ.*保存|待ちの固まり' "$top/pm-work/workflow-design.md" | awk '{print ($1 >= 2) ? "yes" : "no"}')" yes
# (r) 実機のビーコン(ラベル perf-beacons)。extract が R2 の fl-web-cache から直近 3 日の perf/ を取り、run ごとの鍵のキャッシュに保存する。work は同じ鍵で復元し、lift/out-app/host-extra/perf に置く。公開の artifact は使わない。ラベルの無い run では、新しい段は動かない
ge1() { if [ "$1" -ge 1 ]; then echo yes; else echo no; fi; }
PX='実機のビーコンを R2 から取る(perf-beacons の作業だけ。直近 3 日)'; PS='実機のビーコンをキャッシュに保存する(perf-beacons の作業だけ。run ごとの鍵)'
PR='実機のビーコンを R2 のキャッシュから取る(perf-beacons の作業だけ。run ごとの鍵)'; PP='取った物を lift/out-app/host-extra/perf に置く(perf-beacons の作業だけ。commit はしない)'
IFE="success() && contains(format(',{0},', needs.select.outputs.labels), ',perf-beacons,')"
IFW="success() && steps.check.outputs.skip != 'true' && contains(format(',{0},', fromJSON(needs.select.outputs.issue_labels)[format('{0}', matrix.issue)]), ',perf-beacons,')"
PXR=$(jq -r --arg x "$PX" '[.jobs.extract.steps[] | select(.name == $x) | .run][0] // ""' "$J")
ok "ビーコン: 段は extract に 2 つ(取り出し・保存)、work に 2 つ(復元・配置)" "$(jq -r --arg x "$PX" --arg s "$PS" --arg r "$PR" --arg p "$PP" '"\([.jobs.extract.steps[].name] | map(select(. == $x or . == $s)) | length)/\([.jobs.work.steps[].name] | map(select(. == $r or . == $p)) | length)"' "$J")" "2/2"
ok "ビーコン: extract の 2 段の if は、labels の判定を持つ(success() を付け、カンマで囲む)" "$(jq -r --arg x "$PX" --arg s "$PS" --arg c "$IFE" '[.jobs.extract.steps[] | select(.name == $x or .name == $s) | .["if"] == $c] | all' "$J")" true
ok "ビーコン: work の 2 段の if は、issue_labels の判定と、確認の skip の判定を持つ" "$(jq -r --arg r "$PR" --arg p "$PP" --arg c "$IFW" '[.jobs.work.steps[] | select(.name == $r or .name == $p) | .["if"] == $c] | all' "$J")" true
ok "ビーコン: 鍵は perf-beacons-<run_id>(保存と復元が同じ式。run ごとに変わる)" "$(jq -r '[.jobs.extract.steps[], .jobs.work.steps[]] | map(.with.key? // empty) | map(select(startswith("perf-beacons-"))) | "\(length)/\(unique | join(" "))"' "$J")" '2/perf-beacons-${{ github.run_id }}'
ok "ビーコン: 保存は runs-on/cache/save@v5 で、path は runner.temp の perf" "$(jq -r --arg x "$PS" '[.jobs.extract.steps[] | select(.name == $x) | "\(.uses)|\(.with.path)"][0]' "$J")" 'runs-on/cache/save@v5|${{ runner.temp }}/perf'
ok "ビーコン: 復元は runs-on/cache/restore@v5 で、path は runner.temp の perf、fail-on-cache-miss: true" "$(jq -r --arg r "$PR" '[.jobs.work.steps[] | select(.name == $r) | "\(.uses)|\(.with.path)|\(.with["fail-on-cache-miss"])"][0]' "$J")" 'runs-on/cache/restore@v5|${{ runner.temp }}/perf|true'
ok "ビーコン: 取り出しの鍵は R2_CACHE_ASSETS(fl-web-cache の鍵)で、R2_ARTIFACT を使わない" "$(jq -r --arg x "$PX" '[.jobs.extract.steps[] | select(.name == $x) | "\(.env.AWS_ACCESS_KEY_ID)|\(.env.AWS_SECRET_ACCESS_KEY)"][0]' "$J")" '${{ secrets.R2_CACHE_ASSETS_ACCESS_KEY_ID }}|${{ secrets.R2_CACHE_ASSETS_SECRET_ACCESS_KEY }}'
ok "ビーコン: 公開の artifact を使わない(ビーコンの 4 段に、upload・download・gh run download・agent- が無い)" "$(jq -r --arg x "$PX" --arg s "$PS" --arg r "$PR" --arg p "$PP" '[.jobs.extract.steps[], .jobs.work.steps[] | select(.name == $x or .name == $s or .name == $r or .name == $p) | tostring | test("upload-artifact|download-artifact|gh run download|agent-")] | any' "$J")" false
ok "ビーコン: 作業者の段に、R2 の鍵もビーコンの段の名前も無い" "$(jq -r '[.jobs.work.steps[] | select(.name == "作業者を 1 回動かす") | tostring | (test("R2_") or test("perf"))] | any' "$J")" false
ok "ビーコン: 取り出しは fl-web-cache の perf/ を読み、件数(1000)と合計(104857600 バイト)で止める" "$(ge1 "$(grep -cF -- '--bucket fl-web-cache' <<<"$PXR")")/$(ge1 "$(grep -cF 'perf/' <<<"$PXR")")/$(ge1 "$(grep -cF '1000' <<<"$PXR")")/$(ge1 "$(grep -cF '104857600' <<<"$PXR")")" "yes/yes/yes/yes"
ok "ビーコン: 取り出しの run は、鍵・中身・アカウントを出さない(set -x・printenv・鍵や入口の echo が無い)" "$(grep -cE 'set -x|printenv|echo .*[$](key|ep)' <<<"$PXR")" 0
ok "ビーコン: 取り出し・保存は、fl-exe の保存の後に在り、取り出しの後に保存が在る" "$(jq -r --arg x "$PX" --arg s "$PS" --arg e "$EXS" '[.jobs.extract.steps[].name] as $n | ($n | index($e)) as $a | ($n | index($x)) as $b | ($n | index($s)) as $c | ($a != null and $b != null and $c != null and $a < $b and $b < $c)' "$J")" true
ok "ビーコン: 配置は fl-exe の配置の後・取り込みの段より前で、復元の後に在る" "$(jq -r --arg r "$PR" --arg p "$PP" --arg e "$WKP" --arg q "前回の作業を取り込む" '[.jobs.work.steps[].name] as $n | ($n | index($e)) as $a | ($n | index($r)) as $b | ($n | index($p)) as $c | ($n | index($q)) as $d | ($a != null and $b != null and $c != null and $d != null and $a < $b and $b < $c and $c < $d)' "$J")" true
ok "ビーコン: 設計の文書に、run ごとの鍵と、公開の artifact を使わない理由を書いた" "$(ge1 "$(grep -c 'perf-beacons' "$top/pm-work/workflow-design.md")")/$(ge1 "$(grep -c 'run ごと' "$top/pm-work/workflow-design.md")")/$(ge1 "$(grep -c '公開の artifact' "$top/pm-work/workflow-design.md")")" "yes/yes/yes"
# 取り出しの run を、偽の aws で動かす。一覧は鍵と大きさの表、取得は鍵を記録する
PXS=$T/pxs.sh; printf '%s\n' "$PXR" > "$PXS"
PXB=$T/pxbin; PXLOG=$T/pxaws.log; PXGOT=$T/pxgot; mkdir -p "$PXB"
cat > "$PXB/aws" <<'PXAWS'
#!/usr/bin/env bash
# 偽の aws(試験用)。一覧は PXLIST(鍵と大きさの表)を --prefix で絞って返す(空なら None)。取得は鍵を PXGOT に残し、空の物を作る。PXFAIL=list なら一覧で失敗する
echo "$*" >> "$PXLOG"
case "$1 $2" in
  "s3api list-objects-v2")
    shift 2; p=""; while [ $# -gt 0 ]; do case "$1" in --prefix) p=$2; shift ;; esac; shift; done
    [ "$PXFAIL" = list ] && { echo "fake aws: 失敗" >&2; exit 255; }
    out=$(awk -F'\t' -v p="$p" 'index($1, p) == 1' "$PXLIST"); if [ -z "$out" ]; then echo None; else printf '%s\n' "$out"; fi ;;
  "s3api get-object")
    shift 2; key=""; file=""; while [ $# -gt 0 ]; do case "$1" in --key) key=$2; shift ;; --endpoint-url|--bucket) shift ;; *) file=$1 ;; esac; shift; done
    echo "$key" >> "$PXGOT"; printf '{}\n' > "$file" ;;
  *) echo "fake aws: 未対応: $*" >&2; exit 1 ;;
esac
PXAWS
chmod +x "$PXB/aws"
D0=$(date -u +%Y-%m-%d); D1=$(date -u -d '-1 day' +%Y-%m-%d); D2=$(date -u -d '-2 day' +%Y-%m-%d); D3=$(date -u -d '-3 day' +%Y-%m-%d)
mkl() { for p in "$@"; do printf '%s\t%s\n' "${p% *}" "${p##* }"; done; }
pxrun() { # 引数: 一覧の表、失敗の種類(無ければ空)、RUNNER_TEMP。標準出力に終了コード
  mkdir -p "$3"; : > "$PXGOT"; : > "$PXLOG"; printf '%s\n' "$1" > "$T/pxlist.tsv"
  ( cd "$T" && env PATH="$PXB:$PATH" PXLIST="$T/pxlist.tsv" PXGOT="$PXGOT" PXLOG="$PXLOG" PXFAIL="$2" CF_ACCOUNT=0123456789 AWS_ACCESS_KEY_ID=x AWS_SECRET_ACCESS_KEY=y RUNNER_TEMP="$3" bash -eo pipefail "$PXS" > "$3.log" 2>&1; echo $? )
}
PXA=$(mkl "perf/$D0/a-1.json 1000" "perf/$D0/a-2.json 2000" "perf/$D0/x.txt 10" "perf/$D1/b-1.json 300" "perf/$D2/c-1.json 400" "perf/$D3/d-1.json 500")
RA=$T/pa; rc=$(pxrun "$PXA" "" "$RA")
ok "ビーコン(取り出し): 直近 3 日(今日・前日・前々日)の .json だけを取る(3 日より前と、.json でない物は取らない)" "$rc/$(sort "$PXGOT" | paste -sd' ' -)" "0/$(printf '%s\n' "perf/$D0/a-1.json" "perf/$D0/a-2.json" "perf/$D1/b-1.json" "perf/$D2/c-1.json" | sort | paste -sd' ' -)"
ok "ビーコン(取り出し): 日付ごとの置き場に分け、MANIFEST.tsv に件数と合計を書く(4 件・3700 バイト)" "$(test -f "$RA/perf/$D0/a-1.json" && test ! -e "$RA/perf/$D3" && echo placed)/$(sed -n 1p "$RA/perf/MANIFEST.tsv")/$(sed -n 2p "$RA/perf/MANIFEST.tsv")" "placed/files	4/bytes	3700"
PXB1=$(for i in $(seq 1 1005); do printf 'perf/%s/n-%s.json\t1\n' "$D0" "$i"; done)
RB=$T/pb; rc=$(pxrun "$PXB1" "" "$RB")
ok "ビーコン(取り出し): 件数は 1000 で止まる(1005 件のうち 1000 件を取る)" "$rc/$(wc -l < "$PXGOT" | tr -d ' ')/$(sed -n 1p "$RB/perf/MANIFEST.tsv")" "0/1000/files	1000"
PXC=$(mkl "perf/$D0/e-1.json 40000000" "perf/$D0/e-2.json 40000000" "perf/$D0/e-3.json 40000000")
RC=$T/pc; rc=$(pxrun "$PXC" "" "$RC")
ok "ビーコン(取り出し): 合計 100 MB で止まる(40 MB が 3 件なら 2 件まで。3 件目は 120 MB で超える)" "$rc/$(sort "$PXGOT" | paste -sd' ' -)/$(sed -n 2p "$RC/perf/MANIFEST.tsv")" "0/perf/$D0/e-1.json perf/$D0/e-2.json/bytes	80000000"
RD=$T/pd; rc=$(pxrun "$PXA" "list" "$RD")
ok "ビーコン(取り出し): 一覧の取得が失敗すれば、段は失敗する(取得の鍵は 1 つも取らない)" "$rc/$(wc -l < "$PXGOT" | tr -d ' ')" "1/0"
RE=$T/pe; rc=$(pxrun "" "" "$RE")
ok "ビーコン(取り出し): 0 件でも段は通り、MANIFEST.tsv(0 件)を作る(キャッシュの保存が空の木にならない)" "$rc/$(sed -n 1p "$RE/perf/MANIFEST.tsv")" "0/files	0"
# (s) 本番の engine の門の廃止(#1378)。deploy に門の段は無く、upload は manifest を常に差し替える。upload の後に、本番の manifest がこの build の物と一致するかを確かめる
DPRE=384b1e40; git -C "$top" show "$DPRE:.github/workflows/deploy.yml" > "$T/deploy-pre.yml" 2>/dev/null || echo "FAIL 変更前の deploy.yml を読めない($DPRE)"
pj "$T/deploy-pre.yml" > "$T/deploy-pre.json" 2>>"$T/pj.err"; DP=$T/deploy-pre.json
ok "deploy.yml(s): 門の段は無い(「検証済みにする」の段も、lift/harness/web を読み書きする段も無い)" "$(jq -r '[.jobs.deploy.steps[] | select(.name == "検証済みにする" or ((.run // "") | test("harness/web")))] | length' "$DJ")" 0
ok "deploy.yml(s): 「本番の manifest を確かめる」の段は 1 つで、fe-build upload の後・wrangler の前に在る" "$(jq -r '
  ([.jobs.deploy.steps | to_entries[] | select((.value.run // "") | test("fe-build upload$")) | .key] | .[0]) as $u
  | ([.jobs.deploy.steps | to_entries[] | select(.value.name == "本番の manifest を確かめる") | .key] | .[0]) as $r
  | ([.jobs.deploy.steps | to_entries[] | select((.value.uses // "") | test("wrangler")) | .key] | .[0]) as $w
  | "\([.jobs.deploy.steps[] | select(.name == "本番の manifest を確かめる")] | length) \($u != null and $r != null and $w != null and $u < $r and $r < $w)"' "$DJ")" "1 true"
ok "deploy.yml(s): 「本番の manifest を確かめる」の段に if・env・uses・continue-on-error が無い(既定の success()。鍵を渡さない)" "$(jq -r '[.jobs.deploy.steps[] | select(.name == "本番の manifest を確かめる") | (has("if") or has("env") or has("uses") or has("continue-on-error"))] | any' "$DJ")" false
RSN=$(jq -r '[.jobs.deploy.steps[] | select(.name == "本番の manifest を確かめる") | .run] | .[0] // ""' "$DJ")
ok "deploy.yml(s): 「本番の manifest を確かめる」は読むだけ(commit・push・gh・wrangler・書き込みの記号は無い)" "$(grep -cE 'git |gh |wrangler|>|tee' <<<"$RSN")" 0
ok "deploy.yml(s): 「本番の manifest を確かめる」を除き、job の設定と on と、既存の段の中身は、変更前(384b1e40)と一致する(抽出の木の鍵の式と、resources・bundle の段は除く)" "$(jq -n -S --slurpfile n "$DJN" --slurpfile o "$DP" --arg re '^(本番の manifest を確かめる|resources を取り出す|bundle の中身を確かめる)$' 'def nk: map(if .id == "cache" then del(.with.key) else . end); ([$n[0].jobs.deploy.steps[] | select((.name // "") | test($re) | not)] | nk) == ($o[0].jobs.deploy.steps | nk) and ($n[0].jobs.deploy | del(.steps)) == ($o[0].jobs.deploy | del(.steps)) and $n[0].on == $o[0].on')" true
ok "fe-build upload は門を読まない(main.rs に、門の語 verified・gated と、root の一覧の読みが無い)" "$(grep -ciE 'verified|root[.]txt|gated' "$top/lift/crates/fe-build/src/main.rs")" 0
OLDSHA=b440e13a623aa949ccce21db53566658425743bb2b21263bcf773d278396e043; NEWSHA=$(printf 'c%.0s' $(seq 1 64))
vm() { printf '{"engine":{"wasm":{"sha256":%s}}}\n' "$1"; }
RS=$T/rs.sh; printf '%s\n' "$RSN" > "$RS"; RBIN=$T/rbbin; RBLOG=$T/rb-aws.log; mkdir -p "$RBIN"
cat > "$RBIN/aws" <<'RAWS'
#!/usr/bin/env bash
echo "$*" >> "$RBLOG"
[ "$RBFAIL" = 1 ] && { echo "fake aws: 失敗" >&2; exit 1; }
[ "$1 $2 $3" = "s3 cp s3://fl-web-assets/manifest.json" ] && [ -n "$4" ] && cp "$RBLIVE" "$4" && exit 0
echo "fake aws: 未対応: $*" >&2; exit 1
RAWS
chmod +x "$RBIN/aws"
vm "\"$NEWSHA\"" > "$T/live-same.json"; vm "\"$OLDSHA\"" > "$T/live-old.json"
rbrun() { # 引数: 本番の manifest のファイル、取得の失敗(1 で失敗)。標準出力に終了コード
  rm -rf "$T/rb"; mkdir -p "$T/rb/lift/out-app/bundle/raw/fl-Oz" "$T/rb/rt"; vm "\"$NEWSHA\"" > "$T/rb/lift/out-app/bundle/raw/fl-Oz/manifest.json"; : > "$RBLOG"
  ( cd "$T/rb" && env PATH="$RBIN:$PATH" RBLOG="$RBLOG" RBLIVE="$1" RBFAIL="$2" RUNNER_TEMP="$T/rb/rt" bash -e "$RS" > "$T/rb.log" 2>&1; echo $? )
}
rc=$(rbrun "$T/live-same.json" 0)
ok "本番の確認(差し替え済み): 本番の manifest がこの build と一致すれば 0 で終わり、取得は 1 回" "$rc/$(grep -c '^s3 cp s3://fl-web-assets/manifest.json ' "$RBLOG")/$(grep -c '^manifest: ' "$T/rb.log")" "0/1/1"
rc=$(rbrun "$T/live-old.json" 0)
ok "本番の確認(差し替わらず): 本番の manifest が古いままなら 0 にならず、deploy を失敗にする" "$( [ "$rc" != 0 ] && echo fails)" fails
rc=$(rbrun "$T/live-same.json" 1)
ok "本番の確認(取得できず): 取れなければ 0 にならない" "$( [ "$rc" != 0 ] && echo fails)" fails
S14=$(awk '/^## 14[.]/{f=1;next} /^## /{f=0} f' "$top/pm-work/workflow-design.md")
ok "deploy.yml(s): 設計の 14 節に、門を廃止した事実(静かに本番に出なかったこと)と理由を書いた" "$(ge1 "$(grep -c '廃止' <<<"$S14")")/$(ge1 "$(grep -c '静かに' <<<"$S14")")" "yes/yes"
# (t) 本番の bundle の入力は、作業者の受け入れの入力と同じ(#1378 の後)。resources の木(error-offline.html など)を deploy が取り出し、bundle の前に lift/out-app/host-extra/resources に置く(fe-build bundle の OFFLINE_PAGE_SRC が読む)。bundle の中身は、manifest の一覧で確かめる
RN='resources を取り出す'; BN='bundle の中身を確かめる'
ok "deploy.yml(t): resources の段は 1 つで、fe-build extract の後・fe-build bundle の前に在る" "$(jq -r --arg r "$RN" '
  ([.jobs.deploy.steps | to_entries[] | select(.value.name == $r) | .key] | length) as $n
  | ([.jobs.deploy.steps | to_entries[] | select((.value.run // "") | test("fe-build extract$")) | .key] | .[0]) as $e
  | ([.jobs.deploy.steps | to_entries[] | select((.value.run // "") | test("fe-build bundle$")) | .key] | .[0]) as $b
  | ([.jobs.deploy.steps | to_entries[] | select(.value.name == $r) | .key] | .[0]) as $r0
  | "\($n) \($e != null and $b != null and $e < $r0 and $r0 < $b)"' "$DJ")" "1 true"
RSN=$(jq -r --arg r "$RN" '[.jobs.deploy.steps[] | select(.name == $r) | .run][0] // ""' "$DJ")
ok "deploy.yml(t): resources の段は、作業者と同じ fl-installer-extract を、ci/resources-prefixes.txt の接頭辞で呼び、lift/out-app/host-extra/resources に置く" "$(grep -cF 'tools/fl-installer-extract.mjs' <<<"$RSN")/$(grep -cF 'ci/resources-prefixes.txt' <<<"$RSN")/$(grep -cF 'lift/out-app/host-extra/resources' <<<"$RSN")" "1/1/1"
ok "deploy.yml(t): resources の段のインストーラの URL は、fe-build の INSTALLER と同じ" "$(jq -r --arg r "$RN" '[.jobs.deploy.steps[] | select(.name == $r) | .env.SRC][0] // ""' "$DJ" | xargs -I{} grep -cF {} "$top/lift/crates/fe-build/src/main.rs")" 1
ok "deploy.yml(t): resources の段は、R2_ARTIFACT の鍵だけを渡し、公開の artifact と gh を使わない" "$(jq -r --arg r "$RN" '[.jobs.deploy.steps[] | select(.name == $r) | "\(.env.AWS_ACCESS_KEY_ID)|\(.env.AWS_SECRET_ACCESS_KEY)|\(tostring | test("upload-artifact|download-artifact|gh "))"][0]' "$DJ")" '${{ secrets.R2_ARTIFACT_ACCESS_KEY_ID }}|${{ secrets.R2_ARTIFACT_SECRET_ACCESS_KEY }}|false'
ok "deploy.yml(t): bundle の中身の段は、流れの判定の後・fe-build upload の前に在り、既定の success() で、if・env・uses・continue-on-error が無い" "$(jq -r --arg b "$BN" '
  ([.jobs.deploy.steps | to_entries[] | select((.value.run // "") | test("flow[.]jsonl") and test("jq -e")) | .key] | .[0]) as $f
  | ([.jobs.deploy.steps | to_entries[] | select(.value.name == $b) | .key] | .[0]) as $c
  | ([.jobs.deploy.steps | to_entries[] | select((.value.run // "") | test("fe-build upload$")) | .key] | .[0]) as $u
  | ([.jobs.deploy.steps[] | select(.name == $b) | (has("if") or has("env") or has("uses") or has("continue-on-error"))] | any) as $x
  | "\([.jobs.deploy.steps[] | select(.name == $b)] | length) \($f != null and $c != null and $u != null and $f < $c and $c < $u) \($x)"' "$DJ")" "1 true false"
BCS=$T/bc.sh; jq -r --arg b "$BN" '[.jobs.deploy.steps[] | select(.name == $b) | .run][0] // ""' "$DJ" > "$BCS"
BCM=$T/bc-m.json; BCQ=$T/bc-s.bin
sf() { printf 'FLSEEDP1....{"version":1,"files":[%s]}' "$(printf '{"path":"seed/resources/%s"},' "$@" | sed 's/,$//')"; }
printf '%s\n' '{"lazy":[{"name":"error-offline.html","path":"resources/System/Internet/error-offline.html"}]}' > "$BCM"
sf System/Languages/de.moe Artwork/Fonts/Cuprum/Cuprum.ttf Data/Templates/Other/Blocks/Blocks.flp > "$BCQ"
bcrun() { # 引数: 2 つ(bundle の manifest・seed.bin)。一時の木で bundle の中身の段を走らせ、終了コードを出す
  rm -rf "$T/bc"; mkdir -p "$T/bc/lift/out-app/bundle/raw/fl-Oz" "$T/bc/lift/out-app/seed"
  cp "$1" "$T/bc/lift/out-app/bundle/raw/fl-Oz/manifest.json"; cp "$2" "$T/bc/lift/out-app/seed/seed.bin"
  ( cd "$T/bc" && bash -e "$BCS" > /dev/null 2>&1; echo $? )
}
ok "deploy.yml(t): bundle の中身: 代表 4 件(error-offline.html・言語・テーマ・テンプレート)が在れば 0 で通る" "$(test -s "$BCS" && bcrun "$BCM" "$BCQ")" 0
printf '%s\n' '{"lazy":[]}' > "$T/bc-m0.json"
ok "deploy.yml(t): bundle の中身: error-offline.html が無ければ落ちる(本番に出ない物を見逃さない)" "$(bcrun "$T/bc-m0.json" "$BCQ")" 1
sf Artwork/Fonts/Cuprum/Cuprum.ttf Data/Templates/Other/Blocks/Blocks.flp > "$T/bc-s1.bin"
ok "deploy.yml(t): bundle の中身: 言語パックの代表が無ければ落ちる" "$(bcrun "$BCM" "$T/bc-s1.bin")" 1
sf System/Languages/de.moe Data/Templates/Other/Blocks/Blocks.flp > "$T/bc-s2.bin"
ok "deploy.yml(t): bundle の中身: テーマの代表が無ければ落ちる" "$(bcrun "$BCM" "$T/bc-s2.bin")" 1
sf System/Languages/de.moe Artwork/Fonts/Cuprum/Cuprum.ttf > "$T/bc-s3.bin"
ok "deploy.yml(t): bundle の中身: テンプレートの代表が無ければ落ちる" "$(bcrun "$BCM" "$T/bc-s3.bin")" 1
printf '%s\n' seed/resources/System/Languages/de.moe seed/resources/Artwork/Fonts/Cuprum/Cuprum.ttf seed/resources/Data/Templates/Other/Blocks/Blocks.flp > "$T/bc-s4.bin"
ok "deploy.yml(t): bundle の中身: 引用符の無い語句だけでは、ヘッダの path と見なさない(3 件そろっても落ちる)" "$(bcrun "$BCM" "$T/bc-s4.bin")" 1
# (u) main との衝突による push の失敗の自動の再試行(設計「4」「7」「8」)。push の段は衝突の印を残す。報告は印の数で retry か fail を決め、retry は failed を付けず、App の token で running を外して次の run を起こす
AGPRE=f3abbf3c
PREU=$T/agent-pre.yml; git -C "$top" show "$AGPRE:.github/workflows/agent.yml" > "$PREU" 2>/dev/null
pj "$PREU" > "$T/agent-pre.json" 2>>"$T/pj.err"; PREJ=$T/agent-pre.json
PREP=$(jq -r '[.jobs.work.steps[] | select(.name | test("通った変更を main に push")) | .run] | .[0] // ""' "$PREJ")
PSHS=$T/push-u.sh; printf '%s\n' "$PSH" > "$PSHS"
ok "push(u1): 変更前(f3abbf3c)の push の段の行は、全て今の段に残る(削除・書き換えが無い。追加だけ)" "$(diff <(printf '%s\n' "$PREP") "$PSHS" | grep -c '^<')" 0
ok "push(u2): 衝突の印を GITHUB_OUTPUT に書く行が 1 つ在る" "$(grep -cF 'echo push_conflict=true >> "$GITHUB_OUTPUT"' <<<"$PSH")" 1
ok "push(u3): 衝突の一覧は pm-work/ の外の名前だけを out/push-conflict.txt に残す(同じ行で絞って書く)" "$(grep -F "grep -v '^pm-work/'" <<<"$PSH" | grep -cF 'out/push-conflict.txt')" 1
PH=$T/ph; mkdir -p "$PH/bin"
cat > "$PH/bin/git" <<'FGIT'
#!/usr/bin/env bash
# 偽の git(試験用)。pull と push の試行ごとの結果を、FG_PULL・FG_PUSH の 1 文字ずつで決める(c = 衝突、o = 成功、r = 拒否)。衝突の一覧は FG_FILES(空白区切り)。計数は FG_D の下
step() { local i; i=$(( $(cat "$2" 2>/dev/null || echo 0) + 1 )); echo "$i" > "$2"; printf '%s' "${1:$((i - 1)):1}"; }
case "$1" in
  rev-parse) echo 0123456789abcdef0123456789abcdef01234567 ;;
  commit|merge) exit 0 ;;
  pull) [ "$(step "$FG_PULL" "$FG_D/pull")" = c ] && { echo 'Automatic merge failed' >&2; exit 1; }; exit 0 ;;
  push) [ "$(step "$FG_PUSH" "$FG_D/push")" = o ] && exit 0; echo ' ! [remote rejected] HEAD -> main' >&2; exit 1 ;;
  diff) if [ -n "$FG_FILES" ]; then printf '%s\n' $FG_FILES; fi ;;
  *) echo "fake git: 未対応: $*" >&2; exit 1 ;;
esac
FGIT
chmod +x "$PH/bin/git"
pscn() { # 引数: 名前、pull の列、push の列、衝突の一覧。標準出力に終了コード
  local d=$PH/$1; rm -rf "$d"; mkdir -p "$d/rt/out" "$d/rt/botid-bin" "$d/home" "$d/repo"
  printf 'タイトル\n' > "$d/rt/title.txt"; printf '{}\n' > "$d/rt/botid-config.json"
  printf '#!/usr/bin/env bash\nexit 0\n' > "$d/rt/botid-bin/botid"; chmod +x "$d/rt/botid-bin/botid"; : > "$d/gho"
  ( cd "$d/repo" && env PATH="$PH/bin:$PATH" HOME="$d/home" RUNNER_TEMP="$d/rt" GITHUB_OUTPUT="$d/gho" GITHUB_REPOSITORY=o/r ISSUE=5 BOTID_GH="$d/ghcfg" FG_PULL="$2" FG_PUSH="$3" FG_FILES="$4" FG_D="$d" bash -eo pipefail "$PSHS" > "$d/log" 2>&1; echo $? )
}
r=$(pscn p1 ccc "" "a.txt pm-work/x.md lift/y.rs")
ok "push(u4): 衝突で 3 回とも落ちたら 1 で終わり、衝突の印(push_conflict=true)を残す" "$r/$(grep -cx 'push_conflict=true' "$PH/p1/gho")" "1/1"
ok "push(u5): 衝突の一覧は pm-work/ の外の名前だけ(a.txt と lift/y.rs)を残す" "$(paste -sd, - < "$PH/p1/rt/out/push-conflict.txt")" "a.txt,lift/y.rs"
ok "push(u6): 衝突のたびに、既存の「衝突: 一覧」の行を出す(3 回)" "$(grep -cF '衝突: a.txt pm-work/x.md lift/y.rs' "$PH/p1/log")" 3
r=$(pscn p2 ooo rrr "")
ok "push(u7): push の拒否(衝突でない)で 3 回とも落ちたら 1 で終わるが、衝突の印は残さない(remote rejected は再試行の対象外)" "$r/$(grep -c 'push_conflict' "$PH/p2/gho")/$(test -e "$PH/p2/rt/out/push-conflict.txt" && echo made || echo none)" "1/0/none"
r=$(pscn p3 ccc "" "")
ok "push(u8): pull が失敗しても衝突の一覧が空なら、衝突の印を残さない" "$r/$(grep -c 'push_conflict' "$PH/p3/gho")/$(test -e "$PH/p3/rt/out/push-conflict.txt" && echo made || echo none)" "1/0/none"
r=$(pscn p4 oco ro "a.txt")
ok "push(u9): 途中で衝突しても、最後の試行が成功すれば 0 で終わり、印を残さず、commit.txt を残す" "$r/$(grep -c 'push_conflict' "$PH/p4/gho")/$(test -e "$PH/p4/rt/out/push-conflict.txt" && echo made || echo none)/$(test -s "$PH/p4/rt/out/commit.txt" && echo sha)" "0/0/none/sha"
r=$(pscn p5 cco rrr "a.txt")
ok "push(u10): 最後の試行が拒否なら(前の試行が衝突でも)、衝突の印は残さない(最後の試行の原因で判定する)" "$r/$(grep -c 'push_conflict' "$PH/p5/gho")/$(test -e "$PH/p5/rt/out/push-conflict.txt" && echo made || echo none)" "1/0/none"
r=$(pscn p6 ooc rr "a.txt lift/q.rs")
ok "push(u11): 最後の試行だけ衝突で落ちたら、印と一覧を残す(拒否の後の衝突)" "$r/$(grep -cx 'push_conflict=true' "$PH/p6/gho")/$(paste -sd, - < "$PH/p6/rt/out/push-conflict.txt")" "1/1/a.txt,lift/q.rs"
r=$(pscn p7 ooo ooo "")
ok "push(u12): 成功なら 0 で終わり、印も一覧も残さず、commit.txt を残す" "$r/$(test -e "$PH/p7/rt/out/push-conflict.txt" && echo made || echo none)/$(test -s "$PH/p7/rt/out/commit.txt" && echo sha)" "0/none/sha"
# (u) 報告: 印の数で retry か fail を決める。retry は failed を付けず、印の行を書く。App の段は、再試行の Issue があるときだけ
MK='<!-- auto-retry:push-conflict -->'
NV=$T/nv; NVO=$T/nv-gho; NVL=$T/nvgh.log; NVBD=$T/nvgh-bodies.txt; NVRT=$T/nvrt; mkdir -p "$NV"; : > "$NVL"; : > "$NVBD"; : > "$NVO"
nvfx() { printf '%s' "$2" > "$NV/repos__o__r__issues__$1.json"; }
nvfx 18 '{"number":18,"state":"open","labels":[{"name":"running"}]}'
nvfx 19 '{"number":19,"state":"open","labels":[{"name":"running"}]}'
nvfx 20 '{"number":20,"state":"open","labels":[{"name":"running"}]}'
nvfx 21 '{"number":21,"state":"open","labels":[{"name":"running"}]}'
nvfx 22 '{"number":22,"state":"closed","labels":[{"name":"running"}]}'
printf '%s' '[]' > "$NV/repos__o__r__issues__18__comments.json"
printf '%s' "[{\"body\":\"$MK\\n(1/2)\"},{\"body\":\"$MK\\n(2/2)\"}]" > "$NV/repos__o__r__issues__19__comments.json"
printf '%s' '[]' > "$NV/repos__o__r__issues__20__comments.json"
printf '%s' "[{\"body\":\"$MK\\n(1/2)\"}]" > "$NV/repos__o__r__issues__21__comments.json"
printf '%s' '[]' > "$NV/repos__o__r__issues__22__comments.json"
PUSHN='通った変更を main に push する(token はこの段で発行する)'; CKN=$(jq -r '[.jobs.work.steps[] | select(.id == "check") | .name][0]' "$J")
jq -n --arg p "$PUSHN" --arg ck "$CKN" '{jobs: [
  {name: "起動する Issue を決める", conclusion: "success", steps: []},
  {name: "作業 #18", conclusion: "failure", steps: [{name: $ck, conclusion: "success"}, {name: $p, conclusion: "failure"}]},
  {name: "作業 #19", conclusion: "failure", steps: [{name: $ck, conclusion: "success"}, {name: $p, conclusion: "failure"}]},
  {name: "作業 #20", conclusion: "failure", steps: [{name: $ck, conclusion: "success"}, {name: "作業者を 1 回動かす", conclusion: "failure"}]},
  {name: "作業 #21", conclusion: "failure", steps: [{name: $ck, conclusion: "success"}, {name: $p, conclusion: "failure"}]},
  {name: "作業 #22", conclusion: "failure", steps: [{name: $ck, conclusion: "success"}, {name: $p, conclusion: "failure"}]}
]}' > "$NV/repos__o__r__actions__runs__79__jobs.json"
for n in 18 19 21 22; do mkdir -p "$NV/art-$n"; printf 'lift/a.rs\n' > "$NV/art-$n/push-conflict.txt"; printf 'diff\n' > "$NV/art-$n/work.diff"; printf 'judge: 通った\n' > "$NV/art-$n/judge.txt"; done
mkdir -p "$NV/art-20"; printf 'diff\n' > "$NV/art-20/work.diff"; printf 'judge: 通った\n' > "$NV/art-20/judge.txt"
nvrun() { # 引数: 報告のスクリプト名。標準出力に終了コード
  ( cd "$top" && env PATH="$NFB:$PATH" NFX="$NV" NGL="$NVL" NGB="$NVBD" GH_TOKEN=x GH_REPO=o/r RUN_ID=79 RUN_URL=https://example.test/run/79 SERVER=https://example.test CLAIMED='[18,19,20,21,22]' RUNNER_TEMP="$NVRT" GITHUB_OUTPUT="$NVO" bash -e "$T/$1" > "$T/$1.nv.log" 2>&1; echo $? )
}
rm -rf "$NVRT"; mkdir -p "$NVRT"
ok "報告(u13): 結果を集める段は、5 件とも 0 で終わる" "$(nvrun nrep1.sh)" 0
ok "報告(u14): 結果を書く段は、5 件とも 0 で終わる" "$(nvrun nrep2.sh)" 0
ok "報告(u15): 衝突があり印が 0 の 18 は retry、印が 1 の 21 も retry、印が 2 の 19 は failure、衝突の印が無い 20 と閉じた 22 は failure" "$(cat "$NVRT/report/18/status")/$(cat "$NVRT/report/21/status")/$(cat "$NVRT/report/19/status")/$(cat "$NVRT/report/20/status")/$(cat "$NVRT/report/22/status")" "retry/retry/failure/failure/failure"
ok "報告(u16): 再試行が在るので、起動の段の if のための出力 retry=true を残す" "$(grep -cx 'retry=true' "$NVO")" 1
ok "報告(u17): 再試行の印の行は、18 が 1/2、21 が 2/2 と書く(印の行を含むコメント)" "$(grep -cF "$MK" "$NVRT/report/18/comment.md")/$(grep -cF '(1/2)' "$NVRT/report/18/comment.md")/$(grep -cF '(2/2)' "$NVRT/report/21/comment.md")" "1/1/1"
nvb() { awk -v n="$1" '$0 ~ ("^BODY " n " ") {f=1; next} /^BODY /{f=0} f' "$NVBD"; }
ok "報告(u18): retry の 18 と 21 は、印の行を含むコメントを書く(GITHUB_TOKEN で)" "$(nvb 18 | grep -cF "$MK")/$(nvb 21 | grep -cF "$MK")" "1/1"
ok "報告(u19): retry の 18・21 は failed を付けず、running も GITHUB_TOKEN では外さない(外すのは App の段)" "$(grep -c '^issue edit 18 .*add-label failed' "$NVL")/$(grep -c '^issue edit 18 .*remove-label running' "$NVL")/$(grep -c '^issue edit 21 .*add-label failed' "$NVL")/$(grep -c '^issue edit 21 .*remove-label running' "$NVL")" "0/0/0/0"
ok "報告(u20): 印が 2 の 19 は failed を付け、running を外し、再試行の上限の旨を書く(印の行は書かない)" "$(grep -c '^issue edit 19 .*add-label failed' "$NVL")/$(grep -c '^issue edit 19 .*remove-label running' "$NVL")/$(nvb 19 | grep -cF '自動の再試行を 2 回行ったが、衝突が続く。人手(PM)が見る')/$(nvb 19 | grep -cF "$MK")" "1/1/1/0"
ok "報告(u21): 衝突の印が無い 20(拒否)は failed を付け、自動の再試行の旨を書かない" "$(grep -c '^issue edit 20 .*add-label failed' "$NVL")/$(nvb 20 | grep -c '自動の再試行')" "1/0"
ok "報告(u22): 閉じている 22 は、衝突でも自動の再試行をせず、failed を付ける" "$(grep -c '^issue edit 22 .*add-label failed' "$NVL")/$(nvb 22 | grep -cF "$MK")" "1/0"
ok "報告(u23): 再試行の印のコメントに、保存した作業の旨を書く" "$(nvb 18 | grep -cF '作業を保存した')" 1
APPN='自動の再試行を起動する(App の token で running を外す。再試行の Issue だけ)'
ok "報告(u24): botid(App の名義)を使う段は 1 つだけで、その名前は再試行の起動の段" "$(jq -r '[.jobs.report.steps[] | select(tostring | test("botid")) | .name] | join("|")' "$J")" "$APPN"
ok "報告(u25): App の鍵を使う段は、報告の中でも、その起動の段だけ" "$(jq -r '[.jobs.report.steps[] | select(tostring | test("secrets[.]GH_APP_")) | .name] | join("|")' "$J")" "$APPN"
ok "報告(u26): 起動の段は、結果を集める段の出力 retry が真のときだけ動く" "$(jq -r --arg a "$APPN" '[.jobs.report.steps[] | select(.name == $a) | .["if"]] | .[0]' "$J")" "steps.collect.outputs.retry == 'true'"
ok "報告(u27): 結果を集める段に id collect がある" "$(jq -r '[.jobs.report.steps[] | select(.name | test("結果を集める")) | .id] | .[0]' "$J")" collect
ok "報告(u28): 起動の段は GH_TOKEN を持たない(gh は App の設定だけを使う)" "$(jq -r --arg a "$APPN" '[.jobs.report.steps[] | select(.name == $a) | (.env // {} | has("GH_TOKEN"))] | .[0] | tostring' "$J")" false
ok "報告(u29): 起動の段は、結果を書く段の後に在る" "$(jq -r --arg a "$APPN" '[.jobs.report.steps[].name] as $n | ($n | index("結果を Issue に書き、状態を変える(GITHUB_TOKEN で。通った Issue が開いたままなら、ここで閉じる)")) as $w | ($n | index($a)) as $p | "\($w != null and $p != null and $p > $w)"' "$J")" true
APPR=$(jq -r --arg a "$APPN" '[.jobs.report.steps[] | select(.name == $a) | .run] | .[0] // ""' "$J")
ok "報告(u30): 起動の段は、道具の trigger を呼び、botid の設定で GH_CONFIG_DIR を得る" "$(grep -cF 'node tools/auto-retry.mjs trigger' <<<"$APPR")/$(grep -cF 'GH_CONFIG_DIR' <<<"$APPR")" "1/2"
ok "報告(u31): 起動の段は、使い終える行で、鍵の一時ファイル(botid-config.json)と hosts.yml を消す" "$(grep -cE 'rm -f .*botid-config[.]json.*hosts[.]yml' <<<"$APPR")" 1
ok "報告(u32): 起動の段の botid の取得は、push の段と同じ版と sha256(botid 0.2.3・fffa02…)" "$(grep -cF 'v=0.2.3' <<<"$APPR")/$(grep -cF 'fffa02853f88cf20637970f219f682c7a7768427169d4dc883491995d019fc50' <<<"$APPR")" "1/1"
R1U=$(jq -r '[.jobs.report.steps[] | select(.name | test("結果を集める")) | .run][0] // ""' "$J")
ok "報告(u33): 結果を集める段は、印の数を道具の count で数え、decide で決め、retry を GITHUB_OUTPUT に書く" "$(grep -cF 'node tools/auto-retry.mjs count' <<<"$R1U")/$(grep -cF 'node tools/auto-retry.mjs decide' <<<"$R1U")/$(grep -cF 'echo "retry=' <<<"$R1U")" "1/1/1"
RW=$(jq -r '[.jobs.report.steps[] | select(.name | test("結果を Issue に書き")) | .run][0] // ""' "$J")
ok "報告(u34): 結果を書く段は、retry の Issue に GITHUB_TOKEN で running を外さない(除去は if の外で、retry を除く)" "$(grep -cF 'if [ "$s" != retry ]' <<<"$RW")/$(grep -cF 'remove-label running' <<<"$RW")" "1/1"
ok "報告(u35): 再試行の印の文字列は、報告の段と道具で同じ" "$(grep -cF "$MK" <<<"$R1U")/$(grep -cF "$MK" "$top/tools/auto-retry.mjs")" "1/1"
ok "報告(u36): 再試行の上限は、道具の定数 2 と、報告の表示の 2 で、同じ(2/2 と 2 回)" "$(grep -cF 'MAX = 2' "$top/tools/auto-retry.mjs")/$(grep -cF '/2)' <<<"$R1U")" "1/1"
ok "(u) 道具: tools/auto-retry.mjs と spec/auto-retry.test.mjs が在る" "$(test -f "$top/tools/auto-retry.mjs" && test -f "$top/spec/auto-retry.test.mjs" && echo yes)" yes
DSG=$top/pm-work/workflow-design.md
DSS=$(awk '/^## .*衝突の自動の再試行/{f=1;next} /^## /{f=0} f' "$DSG")
ok "設計(u): 「衝突の自動の再試行」の節が在り、印・上限 2・remote rejected の対象外・起動の方式(unlabeled)を書く" "$(for w in 'auto-retry:push-conflict' '上限' 'remote rejected' 'unlabeled' 'App'; do grep -cF "$w" <<<"$DSS"; done | awk '{s+=($1>0)} END{print s}')" 5
ok "設計(u): 「8」のトークンの節は、報告の起動の段に App の鍵を渡すことを書く" "$(awk '/^## 8[.]/{f=1;next} /^## /{f=0} f' "$DSG" | grep -cF '自動の再試行の起動の段')" 1
# (v) 本番の連鎖(deploy.yml → fe-build)が作る物(#1337 の欠陥の再発防止)。pack.args が読む lift/out-app/ の元は、作る段が本番の連鎖(plan の記録か deploy.yml)に在る。作業者の受け入れの連鎖にしか無い物は、本番で作られず落ちる
DYML=$top/.github/workflows/deploy.yml; PA=$top/lift/harness/web/pack.args; PLAN=$top/lift/crates/fe-build/tests/plan
LZ=$(awk 'prev == "--lazy-file" { sub(/=.*/, ""); print } { prev = $0 }' "$PA" | grep '^lift/out-app/')
TOPS=$(printf '%s\n' "$LZ" | while IFS= read -r p; do printf '%s\n' "$p" | cut -d/ -f1-3; done | sort -u)
MISS=$(for d in $TOPS; do
  grep -vh 'pack-assets' "$PLAN/bundle.txt" "$PLAN/engine.txt" "$PLAN/extract.txt" "$PLAN/external.txt" 2> /dev/null | grep -qF "$d" || grep -qF "$d" "$DYML" || echo "$d"
done | paste -sd' ' -)
ok "本番(v): pack.args の lift/out-app/ の元の dir(先頭 3 段)は、全て作る段が本番の連鎖に在る(在らない dir の一覧)" "$MISS" ""
ok "本番(v): 外部のインストーラは、fe-build bundle より前に fe-build external で取る(deploy.yml の行の順)" "$(a=$(grep -n 'fe-build external' "$DYML" | cut -d: -f1 | head -1); b=$(grep -n 'fe-build bundle' "$DYML" | cut -d: -f1 | head -1); [ -n "$a" ] && [ -n "$b" ] && [ "$a" -lt "$b" ] && echo yes)" yes
OTTP=$(grep -n 'plugin-worker/build.sh ott' "$PLAN/bundle.txt" | cut -d: -f1 | head -1); PKP=$(grep -n 'pack-assets' "$PLAN/bundle.txt" | cut -d: -f1 | head -1)
ok "本番(v): bundle の計画は、OTT の wasm の作る段(build.sh ott)を pack-assets より前に持つ" "$([ -n "$OTTP" ] && [ -n "$PKP" ] && [ "$OTTP" -lt "$PKP" ] && echo yes)" yes
ok "本番(v): OTT の作る段の出力(build.sh の \$OUT/vst-ext.wasm)は、pack.args の --lazy-file の元(plugins/ott/vst-ext.wasm)と同じ名前" "$(grep -qF '"$OUT/vst-ext.wasm"' "$top/lift/harness/web/mt1/plugin-worker/build.sh" && grep -qF 'lift/out-app/plugins/ott/vst-ext.wasm' "$PA" && echo yes)" yes
ok "本番(v): OTT の作る段の入力(インストーラの dir)は、fe-build external が作る dir と同じ" "$(grep -qF 'lift/out-app/external' "$PLAN/bundle.txt" && grep -qF 'lift/out-app/external' "$PLAN/external.txt" && echo yes)" yes
ok "本番(v): fe-build external は、ci/artifacts.lock の外部のインストーラを全て取り、照合する(check.mjs)" "$(grep -cF 'tools/external-inputs/check.mjs lift/out-app/external' "$PLAN/external.txt")" 1
ok "本番(v): deploy.yml は fe-build の副命令を呼ぶだけ(OTT の作る段は、deploy.yml の run に書かない)" "$(grep -cE 'plugin-worker|build[.]sh' "$DYML")" 0
[ -e "$T/failed" ] && fail=1; rm -rf "$T"; exit $fail
