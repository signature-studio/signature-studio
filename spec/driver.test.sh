#!/usr/bin/env bash
# spec/driver.test.sh : 駆動(driver/)のテスト。gh は偽物(呼び出しを記録し、待ち行列をファイルで持つ)に替える。判定の起動確認の道具(tools/boot-check/)も確かめる。
# 一時ディレクトリはリポジトリの中の .tmp-spec.*(git の無視の対象)に作り、終わったら消す。
# git の commit を使うので、環境の名義で commit する(名義を上書きしない)。環境に名義が無ければ skip と出す。
here=$(cd "$(dirname "$0")" && pwd); top=$here/..; fail=0; T=$(mktemp -d "$top/.tmp-spec.XXXXXX")
ok() { if [ "$2" = "$3" ]; then echo "ok   $1"; else echo "FAIL $1: expected $3, got $2"; fail=1; fi; }
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
# 偽の gh: 待ち行列は $Q/ready・$Q/running(番号<TAB>題名)、落ちた回数は $Q/fails
Q=$T/q; mkdir -p "$Q" "$T/bin"
cat > "$T/bin/gh" <<'EOF'
#!/usr/bin/env bash
echo "$*" >> "$Q/log"
case "$1 $2" in
  "issue list") for a in "$@"; do case $a in ready|running) cat "$Q/$a" 2>/dev/null;; failed) ;; esac; done ;;
  "issue create") n=$(( $(cat "$Q/n" 2>/dev/null || echo 100) + 1 )); echo "$n" > "$Q/n"; printf '%s\t%s\n' "$n" "$4" >> "$Q/ready" ;;
  "issue view") cat "$Q/fails" ;;
esac
EOF
chmod +x "$T/bin/gh"; export Q PATH="$T/bin:$PATH"
reset() { rm -f "$Q"/*; printf '%s' "$1" > "$Q/ready"; printf '%s' "$2" > "$Q/running"; }
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
# close.sh
reset "" ""; ( cd "$R" && bash driver/close.sh 5 success https://x >/dev/null 2>&1 )
ok "close: 通ったら閉じる" "$(grep -c '^issue close 5 --reason completed' "$Q/log")" 1
reset "" ""; echo 0 > "$Q/fails"; ( cd "$R" && bash driver/close.sh 5 failure https://x >/dev/null 2>&1 )
ok "close: 落ちたら待ちに戻す" "$(grep -c -- '--add-label ready' "$Q/log")" 1
reset "" ""; echo 2 > "$Q/fails"; ( cd "$R" && bash driver/close.sh 5 failure https://x >/dev/null 2>&1 )
ok "close: 3 回落ちたら failed にして閉じる" "$(grep -c -- '--add-label failed' "$Q/log")/$(grep -c '^issue close 5' "$Q/log")" 1/1
reset "" ""; echo 2 > "$Q/fails"; ( cd "$R" && bash driver/close.sh 5 cancelled https://x >/dev/null 2>&1 )
ok "close: 取り消された run は落ちた数に入れず、コメントせず待ちに戻す(2 回の後でも failed にしない)" "$(grep -c -- '--add-label failed' "$Q/log")/$(grep -c -- '--add-label ready' "$Q/log")/$(grep -c '^issue comment' "$Q/log")" 0/1/0
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
rm -rf "$T"; exit $fail
