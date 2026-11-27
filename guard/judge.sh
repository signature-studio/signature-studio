#!/usr/bin/env bash
# guard/judge.sh BASE TASKFILE : 作業ジョブの判定。通れば 0 で終わる。作業ジョブの最後の段で、取り直した main の物を呼ぶ。
# 0) BASE の ref が読めない(git diff が失敗する)なら落ちる。失敗を握りつぶして、差分が無いものとして通さない
# 1) 作業ツリーに、commit していない変更が無い(判定するのは push される物)。commit の題名と本文に、deploy を止める印(skip ci など)が無い
# 2) タスクの受け入れコマンド(タスクのファイルの「accept: 」の行)が 0 で終わる。行が無いタスクは、この段を飛ばす。受け入れが boot.json を使うときは、失敗の原因の手がかり(起動確認の結果の 4 項目)を、失敗の直前に 1 行で出す
# 3) BASE からの変更ファイルに、依頼者の決まりへの違反が無い(scan.sh、flags.sh)
# 4) ハーネス(tools/・guard/・spec/・.github/・opencode.jsonc)を変えたなら、BASE に在る spec/guard.test.sh・spec/workflow.test.sh・spec/driver.test.sh を実行する。判定の中の spec は、判定を呼び直さない
# 2) の前に、木に無い内蔵のプラグインの DLL を置く(tools/place-plugin-dlls.mjs。失敗しても止めず、行を判定の出力に残す)
# 既知の限界: 4) の spec は作業者の作業ツリーの版で走る。作業者が spec を弱めれば、それを通す。
base=${1:?usage: guard/judge.sh BASE TASKFILE}; task=${2:?usage: guard/judge.sh BASE TASKFILE}
here=$(cd "$(dirname "$0")" && pwd)
changed=$(git diff --name-only --diff-filter=AM "$base"...HEAD) || { echo "judge: base の ref を読めない: $base"; exit 1; }
if [ -n "$(git status --porcelain)" ]; then echo "judge: commit していない変更がある"; git status --short | head -20; exit 1; fi
if git log --format=%B "$base..HEAD" | grep -qiE 'skip ci|ci skip|no ci|skip actions|actions skip|NO_CI'; then echo "judge: commit の題名・本文に、deploy を止める印(skip ci など)がある"; exit 1; fi
cmd=$(grep -m1 '^accept: ' "$task" | cut -c9-)
# 受け入れが boot.json(起動確認)を使うときの手がかり。受け入れの前に前の段の boot.json を消し、失敗したら、その結果か「無い」を 1 行で出す。受け入れの再試行はしない
accept_report() {
  case "$1" in *boot.json*) ;; *) return 0 ;; esac
  if [ -z "${RUNNER_TEMP:-}" ] || [ ! -s "$RUNNER_TEMP/boot.json" ]; then echo "judge: 起動確認の結果が無い(boot-bundle の前の段で落ちたか、boot-bundle が何も出さなかった)"; return 0; fi
  local r
  if r=$(node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));const e=typeof r.error==="string"?r.error.slice(0,300):r.error;console.log(JSON.stringify({state:r.state,error:e,standIn:r.standIn,elapsedMs:r.elapsedMs}))' "$RUNNER_TEMP/boot.json" 2>/dev/null); then
    echo "judge: 起動確認の結果: $r"
  else
    echo "judge: 起動確認の結果が読めない: $(head -c 300 "$RUNNER_TEMP/boot.json" | tr '\n\r' '  ' | sed 's/ *$//')"
  fi
}
# 受け入れの前に、取った内蔵のプラグインの DLL を木に置く(tools/place-plugin-dlls.mjs があるときだけ)。失敗しても判定は続き、受け入れの結果で決まる。行は判定の出力に残す
if [ -f tools/place-plugin-dlls.mjs ]; then node tools/place-plugin-dlls.mjs || echo "judge: 木に無い DLL が在る、または置けなかった(上の行。missing は源が無い物、mismatch は sha256 が違う物)"; fi
if [ -n "$cmd" ]; then
  if [ -n "${RUNNER_TEMP:-}" ]; then rm -f "$RUNNER_TEMP/boot.json"; fi
  bash -c "$cmd" || { accept_report "$cmd"; echo "judge: 受け入れコマンドが失敗した"; exit 1; }
fi
files=()
if [ -n "$changed" ]; then mapfile -t files <<<"$changed"; fi
if [ "${#files[@]}" -gt 0 ]; then bash "$here/scan.sh" "${files[@]}" || { echo "judge: 違反がある"; exit 1; }; fi
bash "$here/flags.sh" "$base" HEAD || { echo "judge: 製品に新しい旗がある"; exit 1; }
harness=$(git diff --name-only --no-renames "$base"...HEAD | grep -E '^(tools|guard|spec|[.]github)/|^opencode[.]jsonc$' || true)
if [ -n "$harness" ] && [ -z "${JUDGE_IN_SPEC:-}" ]; then
  for s in spec/guard.test.sh spec/workflow.test.sh spec/driver.test.sh; do
    git cat-file -e "$base:$s" 2>/dev/null || continue
    out=$(JUDGE_IN_SPEC=1 bash "$s" 2>&1) || { grep -v '^ok' <<<"$out" | tail -n 20; echo "judge: ハーネスの変更がテストに通らない($s)"; exit 1; }
  done
fi
echo "judge: 通った"
