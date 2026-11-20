#!/usr/bin/env bash
# spec/guard.test.sh : 判定の道具(guard/)が、依頼者の決まりへの違反を見つけられることを確かめるテスト。
# 一時ディレクトリはリポジトリの中の .tmp-spec.*(git の無視の対象)に作り、終わったら消す。
# git の commit を使うテストは、環境の名義で commit する(名義を上書きしない)。環境に名義が無ければ skip と出す。
here=$(cd "$(dirname "$0")" && pwd); g=$here/../guard; fail=0; T=$(mktemp -d "$here/../.tmp-spec.XXXXXX")
ok() { if [ "$2" = "$3" ]; then echo "ok   $1"; else echo "FAIL $1: expected $3, got $2"; fail=1; : > "$T/failed"; fi; }
rc() { "$@" >/dev/null 2>&1; echo $?; }
# deploy.yml の制約
printf 'on: push\njobs:\n  a:\n    steps:\n      - run: npm ci\n' > "$T/good.yml"
ok "deploy-lint: 守っている物は通る" "$(rc bash "$g/deploy-lint.sh" "$T/good.yml")" 0
{ for i in $(seq 1 101); do echo "# $i"; done; } > "$T/long.yml"; ok "deploy-lint: 101 行は見つかる" "$(rc bash "$g/deploy-lint.sh" "$T/long.yml")" 1
{ echo "# $(printf 'x%.0s' $(seq 1 81))"; } > "$T/wide.yml"; ok "deploy-lint: 81 字の行は見つかる" "$(rc bash "$g/deploy-lint.sh" "$T/wide.yml")" 1
printf 'jobs:\n  a:\n    steps:\n      - run: bash ci/x.sh\n' > "$T/sh.yml"; ok "deploy-lint: シェルスクリプトの実行は見つかる" "$(rc bash "$g/deploy-lint.sh" "$T/sh.yml")" 1
printf 'jobs:\n  a:\n    steps:\n      - uses: ./.github/actions/y\n' > "$T/local.yml"; ok "deploy-lint: ローカル action は見つかる" "$(rc bash "$g/deploy-lint.sh" "$T/local.yml")" 1
printf 'on:\n  workflow_call:\n' > "$T/reuse.yml"; ok "deploy-lint: 再利用ワークフローは見つかる" "$(rc bash "$g/deploy-lint.sh" "$T/reuse.yml")" 1
# 内容の検査(scan.sh は相対パスで規則の範囲を決めるので、一時ディレクトリの中で走らせる)
mkdir -p "$T/app"
v1=anthr; v2=opic
printf 'by %s%s\n' "$v1" "$v2" > "$T/app/a.txt"; ( cd "$T" && ok "scan: ベンダー名は見つかる" "$(rc bash "$g/scan.sh" app/a.txt)" 1 )
printf 'run: python3 x.py\n' > "$T/app/b.txt"; ( cd "$T" && ok "scan: 製品の中で python を実行する行は見つかる" "$(rc bash "$g/scan.sh" app/b.txt)" 1 )
printf 'x\n' > "$T/app/c.py"; ( cd "$T" && ok "scan: 製品の .py は見つかる" "$(rc bash "$g/scan.sh" app/c.py)" 1 )
printf 'ok text\n' > "$T/app/d.txt"; ( cd "$T" && ok "scan: 問題の無いファイルは通る" "$(rc bash "$g/scan.sh" app/d.txt)" 0 )
printf 'x.%s.dev\npython3 x.py\n' workers > "$T/app/e.txt"; ( cd "$T" && ok "scan: 2 件の違反を 2 件と数える" "$(bash "$g/scan.sh" app/e.txt 2>/dev/null | wc -l)" 2 )
# 値を隠す命令(マスク)は、ワークフローに置かない(依頼者の指示 2026-11-20・L-031)。scan.sh の mask 規則は .github/workflows/ の下だけを見る
mkdir -p "$T/.github/workflows" "$T/x/.github/workflows" "$T/app"
printf 'run: echo "::add-mask::a"\nrun: echo "::add-mask::b"\n' > "$T/.github/workflows/mk.yml"
printf 'run: echo "::add-mask::a"\n' > "$T/app/mk.yml"; printf 'run: echo "::add-mask::a"\n' > "$T/x/.github/workflows/mk.yml"
printf 'run: echo hi\n' > "$T/.github/workflows/ok.yml"
( cd "$T" && ok "scan: ワークフローの値を隠す命令は、行ごとに見つかる" "$(bash "$g/scan.sh" .github/workflows/mk.yml 2>/dev/null | grep -c '^mask')" 2 )
( cd "$T" && ok "scan: 同じ命令でも、.github/workflows/ の外(app/)は見つけない" "$(rc bash "$g/scan.sh" app/mk.yml)" 0 )
( cd "$T" && ok "scan: 名前が .github/workflows/ で始まらない場所は見つけない" "$(rc bash "$g/scan.sh" x/.github/workflows/mk.yml)" 0 )
( cd "$T" && ok "scan: 命令の無いワークフローは通る" "$(rc bash "$g/scan.sh" .github/workflows/ok.yml)" 0 )
# リポジトリの外を参照しない(依頼者の指示 2026-11-19)
ok "guard/ が、リポジトリの外を参照しない" "$(grep -nE '\$HOME|~/|systemctl|/\.local/|/\.config/|/tmp|TMPDIR|mktemp' "$g"/*.sh | wc -l)" 0
# 旗と判定(git の差分を使う)
if git var GIT_AUTHOR_IDENT >/dev/null 2>&1; then
  R=$T/repo; mkdir -p "$R"; git -C "$R" init -q .
  cm() { git -C "$R" add -A && git -C "$R" commit -q --allow-empty -m "$1"; }
  cm base
  mkdir -p "$R/lift/runtime"; printf 'int x;\n' > "$R/lift/runtime/a.c"; cm c1
  printf 'int x;\nchar *p = getenv("FOO");\n' > "$R/lift/runtime/a.c"; cm c2
  ( cd "$R" && ok "flags: 新しい getenv は見つかる" "$(rc bash "$g/flags.sh" HEAD~1 HEAD)" 1 )
  # 判定(judge.sh)
  printf 'accept: true\n' > "$T/task-ok.md"; printf 'accept: false\n' > "$T/task-ng.md"; printf '指示だけ\n' > "$T/task-none.md"
  printf 'int y;\n' > "$R/lift/runtime/b.c"; cm c3; base=$(git -C "$R" rev-parse HEAD~1)
  ( cd "$R" && ok "judge: きれいな作業ツリーで、受け入れコマンドが通れば通る" "$(rc bash "$g/judge.sh" "$base" "$T/task-ok.md")" 0 )
  ( cd "$R" && ok "judge: 受け入れコマンドの行が無いタスクも、ほかが通れば通る" "$(rc bash "$g/judge.sh" "$base" "$T/task-none.md")" 0 )
  ( cd "$R" && ok "judge: 受け入れコマンドが失敗すれば落ちる" "$(rc bash "$g/judge.sh" "$base" "$T/task-ng.md")" 1 )
  printf 'x\n' > "$R/untracked.txt"; ( cd "$R" && ok "judge: commit していない変更があれば落ちる" "$(rc bash "$g/judge.sh" "$base" "$T/task-ok.md")" 1 ); rm -f "$R/untracked.txt"
  mkdir -p "$R/app"; printf 'run: python3 x.py\n' > "$R/app/b.txt"; cm c4; base=$(git -C "$R" rev-parse HEAD~1)
  ( cd "$R" && ok "judge: 変更に違反があれば落ちる" "$(rc bash "$g/judge.sh" "$base" "$T/task-ok.md")" 1 )
  # base が読めない ref のとき、git diff の失敗を握りつぶして通さない(落とす方向だけを足す)
  ( cd "$R" && ok "judge: base の ref が読めなければ落ちる" "$(rc bash "$g/judge.sh" no-such-ref "$T/task-ok.md")" 1 )
  ( cd "$R" && ok "judge: base の ref が読めなければ「通った」を出さない" "$(bash "$g/judge.sh" no-such-ref "$T/task-ok.md" 2>&1 | grep -c '通った')" 0 )
  ( cd "$R" && ok "flags: base の ref が読めなければ落ちる" "$(rc bash "$g/flags.sh" no-such-ref HEAD)" 1 )
  ( cd "$R" && ok "judge: 変更が無い base(HEAD)なら、通った旨を出して通る" "$(bash "$g/judge.sh" HEAD "$T/task-ok.md" 2>&1 | tail -1)" "judge: 通った" )
else
  echo "skip flags・judge: 環境に git の名義が無い"
fi
[ -e "$T/failed" ] && fail=1; rm -rf "$T"; exit $fail
