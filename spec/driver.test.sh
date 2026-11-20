#!/usr/bin/env bash
# spec/driver.test.sh : 駆動(driver/)のテスト。gh は偽物(呼び出しを記録し、待ち行列をファイルで持つ)に替える。判定の起動確認の道具(tools/boot-check/)も確かめる。
# 一時ディレクトリはリポジトリの中の .tmp-spec.*(git の無視の対象)に作り、終わったら消す。
# git の commit を使うので、環境の名義で commit する(名義を上書きしない)。環境に名義が無ければ skip と出す。
here=$(cd "$(dirname "$0")" && pwd); top=$here/..; fail=0; T=$(mktemp -d "$top/.tmp-spec.XXXXXX")
ok() { if [ "$2" = "$3" ]; then echo "ok   $1"; else echo "FAIL $1: expected $3, got $2"; fail=1; : > "$T/failed"; fi; }
rc() { "$@" >/dev/null 2>&1; echo $?; }
if ! git var GIT_AUTHOR_IDENT >/dev/null 2>&1; then echo "skip driver: 環境に git の名義が無い"; rm -rf "$T"; exit 0; fi
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
  "run download") d=$(arg -D "$@"); printf 'ok\njudge: 受け入れコマンドが失敗した\n' > "$d/judge.txt" ;;
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
# アカウント ID(公開リポジトリのログに出さない。L-006)。値は表示しない: 書く段の数と、マスク無しの段の数だけを比べる
mk=$(awk '/^      - name:/{m=0} /::add-mask::/{m=1} /GITHUB_ENV/ && /CF_ACCOUNT=/{n++; if(!m) b++} END{print n+0":"b+0}' "$A")
ok "agent.yml: CF_ACCOUNT を GITHUB_ENV に書く段がある(検査が空振りしない)" "$([ "${mk%%:*}" -gt 0 ] && echo yes)" yes
ok "agent.yml: CF_ACCOUNT を書く前に ::add-mask:: でマスクする(マスク無しの段の数)" "${mk##*:}" 0
acct=$(grep '"account_id"' "$top/app/wrangler.jsonc" | cut -d'"' -f4)
ok "agent.yml・deploy.yml・driver/・spec/ に、アカウント ID の値を直書きしていない(一致の数)" "$(if [ -n "$acct" ]; then cat "$A" "$top/.github/workflows/deploy.yml" "$top"/driver/*.sh "$top"/spec/*.sh | grep -cF -- "$acct"; else echo none; fi)" 0
# driver/ はリポジトリの外を参照しない(依頼者の指示 2026-11-19)
ok "driver/ が、リポジトリの外を参照しない" "$(grep -nE '\$HOME|~/|systemctl|/\.local/|/\.config/|/tmp|TMPDIR|mktemp' "$top"/driver/*.sh | wc -l)" 0
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
[ -e "$T/failed" ] && fail=1; rm -rf "$T"; exit $fail
