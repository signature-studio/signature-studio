#!/usr/bin/env bash
# spec/guard.test.sh : 判定の道具(guard/)が、依頼者の決まりへの違反を見つけられることを確かめるテスト。
# 一時ディレクトリはリポジトリの中の .tmp-spec.*(git の無視の対象)に作り、終わったら消す。
# git の commit を使うテストは、環境の名義で commit する(名義を上書きしない)。環境に名義が無ければ落ちる(skip して通さない)。
# JUDGE_IN_SPEC は、判定が spec を呼ぶときだけ立つ印。この spec の中の判定の検査は、印が無い通常の判定として行う。
unset JUDGE_IN_SPEC
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
ok "ワークフローに、値を隠す命令が無い(件数)" "$(cat "$here"/../.github/workflows/*.yml | grep -cF -- 'add-mask')" 0
mkdir -p "$T/.github/workflows" "$T/x/.github/workflows" "$T/app"
printf 'run: echo "::add-mask::a"\nrun: echo "::add-mask::b"\n' > "$T/.github/workflows/mk.yml"
printf 'run: echo "::add-mask::a"\n' > "$T/app/mk.yml"; printf 'run: echo "::add-mask::a"\n' > "$T/x/.github/workflows/mk.yml"
printf 'run: echo hi\n' > "$T/.github/workflows/ok.yml"
( cd "$T" && ok "scan: ワークフローの値を隠す命令は、行ごとに見つかる" "$(bash "$g/scan.sh" .github/workflows/mk.yml 2>/dev/null | grep -c '^mask')" 2 )
( cd "$T" && ok "scan: 同じ命令でも、.github/workflows/ の外(app/)は見つけない" "$(rc bash "$g/scan.sh" app/mk.yml)" 0 )
( cd "$T" && ok "scan: 名前が .github/workflows/ で始まらない場所は見つけない" "$(rc bash "$g/scan.sh" x/.github/workflows/mk.yml)" 0 )
( cd "$T" && ok "scan: 命令の無いワークフローは通る" "$(rc bash "$g/scan.sh" .github/workflows/ok.yml)" 0 )
# wrangler.jsonc は、その場で作り直さず、デプロイに使える形でコミットする(依頼者の指示 2026-11-20・L-032)
ok "app/wrangler.jsonc は追跡されている(ls-files)" "$(git -C "$here/.." ls-files --error-unmatch app/wrangler.jsonc >/dev/null 2>&1 && echo yes)" yes
ok "app/wrangler.jsonc は無視されていない(check-ignore)" "$(git -C "$here/.." check-ignore -q app/wrangler.jsonc && echo ignored || echo kept)" kept
mkdir -p "$T/tools" "$T/spec" "$T/ledger"
cat > "$T/tools/write.txt" <<'EOF'
cat <<EOT > app/wrangler.jsonc
echo x >> app/wrangler.jsonc
jq '.a' in.json | tee app/wrangler.jsonc
sed -i 's/a/b/' app/wrangler.jsonc
cp new.jsonc app/wrangler.jsonc
mv new.jsonc app/wrangler.jsonc
envsubst < t.jsonc > app/wrangler.jsonc
jq '.x' a.json >| app/wrangler.jsonc
ln -sf other.jsonc app/wrangler.jsonc
dd if=x of=app/wrangler.jsonc
perl -pi -e 's/a/b/' app/wrangler.jsonc
touch app/wrangler.jsonc
fs.writeFileSync('app/wrangler.jsonc', text);
fs::write("app/wrangler.jsonc", s)?;
fs.copyFileSync(src, 'app/wrangler.jsonc');
EOF
cat > "$T/tools/read.txt" <<'EOF'
grep '"account_id"' app/wrangler.jsonc | cut -d'"' -f4
cat app/wrangler.jsonc
jq '.x' app/wrangler.jsonc > out.json
diff app/wrangler.jsonc new.json > log.txt
cp app/wrangler.jsonc backup/out.json
let t = fs::read_to_string("app/wrangler.jsonc")?;
const t = fs.readFileSync(path.join(APP, 'wrangler.jsonc'), 'utf8');
fs.writeFileSync(noperf, readFileSync('app/wrangler.jsonc'));
EOF
cat > "$T/tools/other.txt" <<'EOF'
cat > wrangler.deploy.json <<EOT
cp x app/wrangler.noperf.jsonc
fs.writeFileSync(path.join(work, 'wrangler.noperf.jsonc'), text);
EOF
cp "$T/tools/write.txt" "$T/spec/write.txt"; cp "$T/tools/write.txt" "$T/ledger/write.tsv"; cp "$T/tools/write.txt" "$T/notes.md"
( cd "$T" && ok "scan: wrangler.jsonc を作る・書く行は、行ごとに見つかる(15 行)" "$(bash "$g/scan.sh" tools/write.txt 2>/dev/null | grep -c '^wranglergen')" 15 )
( cd "$T" && ok "scan: wrangler.jsonc を読むだけの行(書きの関数と同じ行でも)は見つけない" "$(bash "$g/scan.sh" tools/read.txt 2>/dev/null | grep -c '^wranglergen')" 0 )
( cd "$T" && ok "scan: wrangler.noperf.jsonc・wrangler.deploy.json は見つけない" "$(bash "$g/scan.sh" tools/other.txt 2>/dev/null | grep -c '^wranglergen')" 0 )
( cd "$T" && ok "scan: spec/・ledger/・*.md は対象外" "$(bash "$g/scan.sh" spec/write.txt ledger/write.tsv notes.md 2>/dev/null | grep -c '^wranglergen')" 0 )
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
  # ハーネス(tools/・guard/・driver/・spec/・.github/・queue/kinds/・opencode.jsonc)を変えたら、BASE に在る spec を判定の中で実行する。偽の spec で確かめる
  fp=$T/fake; mkdir -p "$fp"
  cat > "$fp/fake-spec.sh" <<'FAKE'
#!/usr/bin/env bash
n=$(basename "$0")
echo "$n" >> "$FAKE_T/spec-runs.log"
if [ -n "${FAKE_NEST:-}" ] && [ -z "${FAKE_DONE:-}" ] && [ "$n" = guard.test.sh ]; then FAKE_DONE=1 bash "$FAKE_G/judge.sh" "$FAKE_BASE" "$FAKE_T/task-ok.md" >/dev/null 2>&1; fi
exit "$(cat "$FAKE_T/rc-$n")"
FAKE
  mkh() { rm -rf "$1"; mkdir -p "$1/spec" "$1/tools" "$1/guard" "$1/lift/harness/hyb"; cp "$fp/fake-spec.sh" "$1/spec/guard.test.sh"; cp "$fp/fake-spec.sh" "$1/spec/driver.test.sh"; printf '1\n' > "$1/tools/x.mjs"; printf 'echo\n' > "$1/guard/x.sh"; printf '0xa1\n' > "$1/lift/harness/hyb/lowered-all.txt"; git -C "$1" init -q && git -C "$1" add -A && git -C "$1" commit -q -m base; }
  H=$T/harness; mkh "$H"; HB=$(git -C "$H" rev-parse HEAD)
  rcset() { printf '%s\n' "$1" > "$T/rc-guard.test.sh"; printf '%s\n' "$2" > "$T/rc-driver.test.sh"; }
  jrun() { rm -f "$T/spec-runs.log"; ( cd "$H" && out=$(FAKE_T="$T" FAKE_G="$g" FAKE_BASE="$HB" bash "$g/judge.sh" "$HB" "$T/task-ok.md" 2>&1); echo "$?" > "$T/jrc"; printf '%s\n' "$out" > "$T/jout" ); }
  git -C "$H" checkout -q -B s1 "$HB"; printf '2\n' > "$H/tools/x.mjs"; git -C "$H" commit -qam s1
  rcset 0 0; jrun
  ok "判定: ハーネス(tools/)の変更で、BASE の spec 2 本が走り、通れば通る" "$(cat "$T/jrc")/$(grep -c '^judge: 通った' "$T/jout")/$(cat "$T/spec-runs.log" 2>/dev/null | wc -l)" 0/1/2
  rcset 1 0; jrun
  ok "判定: spec が 0 以外なら落ちて、その spec を名指しする" "$(cat "$T/jrc")/$(grep -c 'judge: ハーネスの変更がテストに通らない(spec/guard.test.sh)' "$T/jout")" 1/1
  git -C "$H" checkout -q -B s3 "$HB"; printf '0xa1\n0xb2\n' > "$H/lift/harness/hyb/lowered-all.txt"; git -C "$H" commit -qam s3
  rcset 1 1; jrun
  ok "判定: 製品のデータ(lift/harness/hyb)だけの変更では、spec を走らせず通る" "$(cat "$T/jrc")/$(grep -c '^judge: 通った' "$T/jout")/$(cat "$T/spec-runs.log" 2>/dev/null | wc -l)" 0/1/0
  git -C "$H" checkout -q -B s4 "$HB"; git -C "$H" rm -q guard/x.sh; git -C "$H" commit -qm s4
  rcset 0 1; jrun
  ok "判定: ファイルの削除だけ(guard/x.sh)の変更でも、ハーネスとして spec を走らせる" "$(cat "$T/jrc")/$(grep -c 'judge: ハーネスの変更がテストに通らない(spec/driver.test.sh)' "$T/jout")/$(grep -c 'driver.test.sh' "$T/spec-runs.log" 2>/dev/null)" 1/1/1
  H2=$T/harness2; rm -rf "$H2"; mkdir -p "$H2/tools"; printf '1\n' > "$H2/tools/x.mjs"; git -C "$H2" init -q && git -C "$H2" add -A && git -C "$H2" commit -q -m base2; H2B=$(git -C "$H2" rev-parse HEAD)
  printf '2\n' > "$H2/tools/x.mjs"; git -C "$H2" commit -qam s5; rcset 1 1; rm -f "$T/spec-runs.log"
  ( cd "$H2" && out=$(FAKE_T="$T" FAKE_G="$g" bash "$g/judge.sh" "$H2B" "$T/task-ok.md" 2>&1); echo "$?" > "$T/jrc"; printf '%s\n' "$out" > "$T/jout" )
  ok "判定: BASE に spec が無いリポジトリでは、ハーネスの変更でも spec を走らせず通る" "$(cat "$T/jrc")/$(grep -c '^judge: 通った' "$T/jout")/$(cat "$T/spec-runs.log" 2>/dev/null | wc -l)" 0/1/0
  git -C "$H" checkout -q -B s6 "$HB"; git -C "$H" rm -q spec/guard.test.sh; git -C "$H" commit -qm s6
  rcset 0 0; jrun
  ok "判定: BASE に在る spec を消す変更は、実行に失敗して落ちる" "$(cat "$T/jrc")/$(grep -c 'judge: ハーネスの変更がテストに通らない(spec/guard.test.sh)' "$T/jout")" 1/1
  git -C "$H" checkout -q -B s7 "$HB"; printf '3\n' > "$H/tools/x.mjs"; git -C "$H" commit -qam s7
  rcset 0 0; rm -f "$T/spec-runs.log"; ( cd "$H" && out=$(FAKE_NEST=1 FAKE_T="$T" FAKE_G="$g" FAKE_BASE="$HB" bash "$g/judge.sh" "$HB" "$T/task-ok.md" 2>&1); echo "$?" > "$T/jrc"; printf '%s\n' "$out" > "$T/jout" )
  ok "判定: spec の中で判定を呼び直しても、ハーネスの spec は再び走らない(再帰しない。走るのは 2 本だけ)" "$(cat "$T/jrc")/$(cat "$T/spec-runs.log" 2>/dev/null | wc -l)" 0/2
else
  echo "FAIL flags・judge: 環境に git の名義が無い(判定の検査は名義が要る。skip して通さない)"; fail=1
fi
[ -e "$T/failed" ] && fail=1; rm -rf "$T"; exit $fail
