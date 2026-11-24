#!/usr/bin/env bash
# spec/guard.test.sh : 判定の道具(guard/)が、依頼者の決まりへの違反を見つけられることを確かめるテスト。
# 一時ディレクトリはリポジトリの中の .tmp-spec.*(git の無視の対象)に作り、終わったら消す。
# git の commit を使うテストは、環境の名義で commit する(名義を上書きしない)。環境に名義が無ければ落ちる(skip して通さない)。
# JUDGE_IN_SPEC は、判定が spec を呼ぶときだけ立つ印。この spec の中の判定の検査は、印が無い通常の判定として行う。
unset JUDGE_IN_SPEC
unset RUNNER_TEMP   # 判定の受け入れは RUNNER_TEMP の boot.json を使う。この spec の判定の検査は、作業者のジョブの一時ディレクトリに触れない
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
mkdir -p "$T/cwd/.github/workflows"
ok "deploy-lint: 引数が無ければ、いま居る所の .github/workflows/deploy.yml を見る(101 行なら見つかる)" "$(cp "$T/long.yml" "$T/cwd/.github/workflows/deploy.yml" && cd "$T/cwd" && rc bash "$g/deploy-lint.sh")" 1
ok "deploy-lint: 引数が無く、その deploy.yml が守っていれば 0" "$(cp "$T/good.yml" "$T/cwd/.github/workflows/deploy.yml" && cd "$T/cwd" && rc bash "$g/deploy-lint.sh")" 0
# 内容の検査(scan.sh は相対パスで規則の範囲を決めるので、一時ディレクトリの中で走らせる)
mkdir -p "$T/app"
v1=anthr; v2=opic
printf 'by %s%s\n' "$v1" "$v2" > "$T/app/a.txt"; ( cd "$T" && ok "scan: ベンダー名は見つかる" "$(rc bash "$g/scan.sh" app/a.txt)" 1 )
printf 'run: python3 x.py\n' > "$T/app/b.txt"; ( cd "$T" && ok "scan: 製品の中で python を実行する行は見つかる" "$(rc bash "$g/scan.sh" app/b.txt)" 1 )
printf 'x\n' > "$T/app/c.py"; ( cd "$T" && ok "scan: 製品の .py は見つかる" "$(rc bash "$g/scan.sh" app/c.py)" 1 )
printf 'ok text\n' > "$T/app/d.txt"; ( cd "$T" && ok "scan: 問題の無いファイルは通る" "$(rc bash "$g/scan.sh" app/d.txt)" 0 )
printf 'x.%s.dev\npython3 x.py\n' workers > "$T/app/e.txt"; ( cd "$T" && ok "scan: 2 件の違反を 2 件と数える" "$(bash "$g/scan.sh" app/e.txt 2>/dev/null | wc -l)" 2 )
# 報告の伏せ字(--redact)。ベンダー名の一致だけを行ごとに伏せ字にし、伏せた後にもう一度検査する。ほかの規則の指摘は従来どおり失敗にする(報告の検査の段)
hx=0123456789abcdef; hx="$hx$hx"; hs="x.r2.cloudflare"; hs="${hs}storage.com"   # 検査の規則に当たる文字列は、組み立てて作る
printf 'by %s%s\nby %s%s\nok\n' "$v1" "$v2" "$v1" "$v2" > "$T/app/r1.txt"
printf 'by %s%s\n' "$v1" "$v2" > "$T/app/r5.txt"
printf 'by %s%s and %s%s\n' "$v1" "$v2" "$v1" "$v2" > "$T/app/r6.txt"
( cd "$T" && ok "redact: ベンダー名の行を、行ごとに出す(2 行)" "$(bash "$g/scan.sh" --redact app/r1.txt 2>/dev/null | grep -c '^redact')" 2 )
( cd "$T" && ok "redact: 伏せた後の内容に、ベンダー名が残らない(件数)" "$(grep -c "$v1$v2" app/r1.txt)" 0 )
( cd "$T" && ok "redact: 伏せ字の数は、一致の数と同じ(2 件)" "$(grep -o '\[伏せ字\]' app/r1.txt | wc -l | tr -d ' ')" 2 )
( cd "$T" && ok "redact: 伏せた後の再検査が通れば 0 で終わる" "$(rc bash "$g/scan.sh" --redact app/r5.txt)" 0 )
( cd "$T" && ok "redact: 1 行に 2 件あっても、その行は 1 行として出す" "$(bash "$g/scan.sh" --redact app/r6.txt 2>/dev/null | grep -c '^redact')/$(grep -o '\[伏せ字\]' app/r6.txt | wc -l | tr -d ' ')" "1/2" )
( cd "$T" && printf 'by %s%s\n' "$v1" "$v2" > app/r7.txt && ok "redact: 出力に、ベンダー名の文字列を含めない" "$(bash "$g/scan.sh" --redact app/r7.txt 2>&1 | grep -c "$v1$v2")" 0 )
( cd "$T" && printf 'id %s\nby %s%s\n' "$hx" "$v1" "$v2" > app/r2.txt && ok "redact: 伏せ字の後も cfhex が残れば失敗する(1)。ベンダー名は伏せる" "$(rc bash "$g/scan.sh" --redact app/r2.txt)/$(grep -c "$v1$v2" app/r2.txt)" "1/0" )
( cd "$T" && printf 'id %s\n' "$hx" > app/r2.txt && ok "redact: cfhex の指摘は伏せ字にしない(出力に cfhex、ファイルに伏せ字無し)" "$(bash "$g/scan.sh" --redact app/r2.txt 2>/dev/null | grep -c '^cfhex')/$(grep -c '伏せ字' app/r2.txt)" "1/0" )
( cd "$T" && printf 'host %s by %s%s\n' "$hs" "$v1" "$v2" > app/r3.txt && ok "redact: cfhost の指摘は伏せ字にしない(終了コード 1、ホスト名は残る)" "$(rc bash "$g/scan.sh" --redact app/r3.txt)/$(grep -cF -- "$hs" app/r3.txt)/$(grep -c '伏せ字' app/r3.txt)" "1/1/1" )
( cd "$T" && printf 'ok text\n' > app/r4.txt && ok "redact: 違反の無いファイルは 0 で終わり、redact の行を出さない" "$(rc bash "$g/scan.sh" --redact app/r4.txt)/$(bash "$g/scan.sh" --redact app/r4.txt 2>/dev/null | grep -c '^redact')" "0/0" )
( cd "$T" && ok "redact: PATH が無ければ使い方の誤り(2)" "$(rc bash "$g/scan.sh" --redact)" 2 )
( cd "$T" && printf 'by %s%s\n' "$v1" "$v2" > app/r9.txt && ok "redact: --redact 無しの検査は、従来どおり vendor で失敗し、伏せ字にしない" "$(rc bash "$g/scan.sh" app/r9.txt)/$(grep -c "$v1$v2" app/r9.txt)" "1/1" )
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
mkdir -p "$T/tools" "$T/spec" "$T/pm-work"
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
cp "$T/tools/write.txt" "$T/spec/write.txt"; cp "$T/tools/write.txt" "$T/pm-work/write.tsv"; cp "$T/tools/write.txt" "$T/notes.md"
( cd "$T" && ok "scan: wrangler.jsonc を作る・書く行は、行ごとに見つかる(15 行)" "$(bash "$g/scan.sh" tools/write.txt 2>/dev/null | grep -c '^wranglergen')" 15 )
( cd "$T" && ok "scan: wrangler.jsonc を読むだけの行(書きの関数と同じ行でも)は見つけない" "$(bash "$g/scan.sh" tools/read.txt 2>/dev/null | grep -c '^wranglergen')" 0 )
( cd "$T" && ok "scan: wrangler.noperf.jsonc・wrangler.deploy.json は見つけない" "$(bash "$g/scan.sh" tools/other.txt 2>/dev/null | grep -c '^wranglergen')" 0 )
( cd "$T" && ok "scan: spec/・pm-work/・*.md は対象外" "$(bash "$g/scan.sh" spec/write.txt pm-work/write.tsv notes.md 2>/dev/null | grep -c '^wranglergen')" 0 )
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
  # 受け入れが boot.json(起動確認)を使うとき: 判定の再実行の前に boot.json を消し、失敗したら、その結果か「無い」を 1 行で出す(受け入れの再試行は足さない)
  rt=$T/rt; mkdir -p "$rt"
  bj() { ( cd "$R" && RUNNER_TEMP="$rt" bash "$g/judge.sh" HEAD "$1" > "$T/bj.out" 2>&1; echo "$?" > "$T/bj.rc" ); }
  cat > "$T/task-b1.md" <<'EOF'
accept: echo '{"state":"running","error":null,"standIn":false,"elapsedMs":1234}' > "$RUNNER_TEMP/boot.json"; false
EOF
  rm -f "$rt/boot.json"; bj "$T/task-b1.md"
  ok "判定: 受け入れが boot.json を使って失敗したら、起動確認の結果を 1 行で、失敗の直前に出す" "$(tail -n 2 "$T/bj.out" | head -n 1)" 'judge: 起動確認の結果: {"state":"running","error":null,"standIn":false,"elapsedMs":1234}'
  ok "判定: 起動確認の結果を出しても、受け入れの失敗は従来どおり最後の行で、終了コードは 1" "$(tail -n 1 "$T/bj.out")/$(cat "$T/bj.rc")" 'judge: 受け入れコマンドが失敗した/1'
  cat > "$T/task-b2.md" <<'EOF'
accept: test -s "$RUNNER_TEMP/boot.json"; false
EOF
  rm -f "$rt/boot.json"; bj "$T/task-b2.md"
  ok "判定: boot.json が無いまま受け入れが失敗したら、その旨を 1 行で出す" "$(tail -n 2 "$T/bj.out" | head -n 1)" 'judge: 起動確認の結果が無い(boot-bundle の前の段で落ちたか、boot-bundle が何も出さなかった)'
  printf '%s\n' '{"state":"engine-idle","error":null,"standIn":false,"elapsedMs":999}' > "$rt/boot.json"
  bj "$T/task-b2.md"
  ok "判定: 前の段の boot.json が残っていても、判定が消してから受け入れを走らせ、前の結果を出さない" "$(tail -n 2 "$T/bj.out" | head -n 1)/$(grep -c 'elapsedMs":999' "$T/bj.out")" 'judge: 起動確認の結果が無い(boot-bundle の前の段で落ちたか、boot-bundle が何も出さなかった)/0'
  cat > "$T/task-b4.md" <<'EOF'
accept: echo '{broken' > "$RUNNER_TEMP/boot.json"; false
EOF
  rm -f "$rt/boot.json"; bj "$T/task-b4.md"
  ok "判定: boot.json が JSON として読めないときは、先頭だけを 1 行で出す" "$(tail -n 2 "$T/bj.out" | head -n 1)" 'judge: 起動確認の結果が読めない: {broken'
  cat > "$T/task-b5.md" <<'EOF'
accept: false
EOF
  rm -f "$rt/boot.json"; bj "$T/task-b5.md"
  ok "判定: boot.json を使わない受け入れの失敗には、起動確認の行を足さない" "$(grep -c '起動確認' "$T/bj.out")" 0
  cat > "$T/task-b7.md" <<'EOF'
accept: node -e 'require("fs").writeFileSync(process.env.RUNNER_TEMP + "/boot.json", JSON.stringify({state: "error", error: "x".repeat(400), standIn: false, elapsedMs: 5}))'; false
EOF
  rm -f "$rt/boot.json"; bj "$T/task-b7.md"
  ok "判定: 起動確認の error は 300 文字で切って出す" "$(tail -n 2 "$T/bj.out" | head -n 1 | grep -o 'x*' | awk '{print length}' | sort -n | tail -n 1)" 300
  cat > "$T/task-b6.md" <<'EOF'
accept: echo '{}' > "$RUNNER_TEMP/boot.json"; true
EOF
  rm -f "$rt/boot.json"; bj "$T/task-b6.md"
  ok "判定: 受け入れが通るときは、起動確認の行を足さず、通った旨を出す" "$(tail -n 1 "$T/bj.out")/$(grep -c '起動確認' "$T/bj.out")/$(cat "$T/bj.rc")" 'judge: 通った/0/0'
  # ハーネス(tools/・guard/・spec/・.github/・opencode.jsonc)を変えたら、BASE に在る spec を判定の中で実行する。偽の spec で確かめる
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
  # driver/ と queue/kinds/ は削除された物なので、ハーネスではない。pm-work/(PM 資料)もハーネスではない。spec/workflow.test.sh は、BASE に在れば判定の中で走る
  git -C "$H" checkout -q -B s8 "$HB"; mkdir -p "$H/driver" "$H/queue/kinds" "$H/pm-work"; printf 'echo\n' > "$H/driver/y.sh"; printf 'x\n' > "$H/queue/kinds/z.md"; printf 'x\n' > "$H/pm-work/w.md"; git -C "$H" add -A; git -C "$H" commit -qm s8
  rcset 1 1; jrun
  ok "判定: driver/・queue/kinds/・pm-work/ の変更は、ハーネスとして spec を走らせず、通る" "$(cat "$T/jrc")/$(grep -c '^judge: 通った' "$T/jout")/$(cat "$T/spec-runs.log" 2>/dev/null | wc -l)" 0/1/0
  H3=$T/harness3; rm -rf "$H3"; mkdir -p "$H3/spec" "$H3/tools"; cp "$fp/fake-spec.sh" "$H3/spec/workflow.test.sh"; printf '1\n' > "$H3/tools/x.mjs"; git -C "$H3" init -q && git -C "$H3" add -A && git -C "$H3" commit -q -m base3; H3B=$(git -C "$H3" rev-parse HEAD)
  printf '2\n' > "$H3/tools/x.mjs"; git -C "$H3" commit -qam s9; printf '1\n' > "$T/rc-workflow.test.sh"; rm -f "$T/spec-runs.log"
  ( cd "$H3" && out=$(FAKE_T="$T" FAKE_G="$g" bash "$g/judge.sh" "$H3B" "$T/task-ok.md" 2>&1); echo "$?" > "$T/jrc"; printf '%s\n' "$out" > "$T/jout" )
  ok "判定: spec/workflow.test.sh が BASE に在るとき、ハーネスの変更で走り、失敗すれば落ちて名指しする" "$(cat "$T/jrc")/$(grep -c 'judge: ハーネスの変更がテストに通らない(spec/workflow.test.sh)' "$T/jout")/$(grep -c 'workflow.test.sh' "$T/spec-runs.log" 2>/dev/null)" 1/1/1
else
  echo "FAIL flags・judge: 環境に git の名義が無い(判定の検査は名義が要る。skip して通さない)"; fail=1
fi
[ -e "$T/failed" ] && fail=1; rm -rf "$T"; exit $fail
