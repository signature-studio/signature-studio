## 目的

依頼者の指示: 内蔵のプラグインを、起動後にバックグラウンドで届ける(1 プラグイン 1 wasm)。@@KIND@@のうち、次の 3 フォルダ(@@NAMES@@)を、1 プラグイン 1 wasm で入れる。

## 背景

- 対象は@@KIND@@の 3 フォルダ。フォルダは、プラグインの単位(`Plugins\Fruity\<種別>\<フォルダ>\`)で、本体の DLL と、その補助の DLL(本体が使う。入口を持たない)は同じフォルダに在る。名前・大きさ・SHA-256・export・import・hook の要否は、分類の表(`inputs/plugin-classes.tsv`)の値を写した。
@@PLUGINS@@
- hook 点: hook 点の表(`inputs/plugin-hooks.tsv`)に、その DLL の行が在れば、その行の 3 つの RVA を使う。在らなければ `none`。分類の表の `hook の要否` が 要 と 既存 の物(本体と補助の両方)が対象になる。
- 型: #{{125}} の仕組み。1 プラグインは `lift/plugins/<名前>/plugin.tsv` を足すだけで入り、共有のファイル(表・引数・一覧・試験の期待値)は編集しない。生成の段が、プラグインの一覧から全てを作る。#{{126}} で、2 本目を同じ形で入れる。#{{127}} の道具で、1 本ずつ確かめる。
- 親: @@PARENT_REF@@(@@PARENT_LABEL@@)の子。
- OpenGL を使う VFX 系のフォルダ(フォルダの DLL に `VFX` が yes の物が在る。補助の DLL が OpenGL を import する場合を含む)は、この Issue に含めない(`inputs/vfx-deferred.tsv`。#{{039}} で後から入れる)。

## 読む物

- `inputs/plugin-classes.tsv`(この 3 フォルダの行)
- `inputs/plugin-hooks.tsv`(hook 点の表)
- `lift/plugins/fruity-balance/plugin.tsv` と `lift/plugins/fruity-filter/plugin.tsv`(#{{125}} と #{{126}} で足した、書式の見本)
- `tools/plugin-run/run.mjs`(#{{127}} の道具)

## 手順

1. 3 フォルダそれぞれについて、`lift/plugins/<フォルダの名前を小文字にし、空白をハイフンにしたもの>/plugin.tsv` を 1 つ足す。1 フォルダの中の本体と補助の DLL の行を、このファイルに並べる。列は見本と同じ(見出し付きの TAB 区切り。`key`・`guest`・`role`・`sha256`・`size`・`hook`・`args`)。
   - `key` は `<種別の小文字>.<フォルダの名前の小文字・ハイフン>.<DLL の名(小文字。拡張子を含む)>`(例: `effects.fruity-balance.fruity_balance_x64.dll`)。
   - `guest` は `Plugins\...\<DLL の名>`(`C:\Program Files\Image-Line\FL Studio 21\` の後)。
   - `role` は `本体` か `補助`。`sha256` と `size` は分類の表の値。
   - `hook`: hook 点の表(`inputs/plugin-hooks.tsv`)に、その DLL の行が在れば `rva:0x...,0x...,0x...`(その行の 3 つの RVA)。在らなければ `none`。
   - `args` は `--pdata`。
2. 共有のファイル(`lift/harness`・`lift/crates`・`lift/runtime`・`app`・`ci`・`tools` の既存の物)を 1 行も変えない。表と一覧と試験の期待値は、生成の段が作る。
3. 3 フォルダの本体を、`tools/plugin-run/run.mjs` で確かめる(受け入れのコマンド)。
4. 報告に、道具の出力の JSON の行をそのまま貼る。engine.wasm の大きさ、boot の `elapsedMs`、共有のファイルの変更が 0 行であることを書く。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/plugin-run/run.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" @@ARGS@@ > "$RUNNER_TEMP/plugin-run.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));process.exit(L.length>=3&&L.every((r)=>r.instantiated===true&&r.processed===true&&r.error===null&&r.elapsedMs<=12500)?0:1)' "$RUNNER_TEMP/plugin-run.jsonl" && test -z "$(git diff --name-only origin/main...HEAD -- . ':!lift/plugins')"
```

判定の基準:
- 3 フォルダの本体が、道具の出力に 1 行ずつ在る(3 行以上)。全ての行で、`instantiated` と `processed` が true、`error` が null、`elapsedMs` が 12500 以下。
- 作業の commit の変更は、`lift/plugins/` の中だけ(受け入れの最後の判定)。
- 起動の確認(boot-check)が engine-idle・standIn が false・error が null。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 追加した `plugin.tsv` の一覧(本体と補助の行の数)、道具の JSON の行、`elapsedMs` の値、engine.wasm の大きさ。

## 制約

- 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- 製品の既定の動作を、測定のためだけに変えない。
- 依頼者が言っていない制限(例: 他のプラグインの扱い)を、この作業の判断で増やさない。
- ベンダー名と Cloudflare の識別子を書かない。Python は製品に使わない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。

## 他のタスクとの関係

- blocked_by: #{{125}}(1 プラグイン = ディレクトリの仕組み)、#{{126}}(2 本目の型の確認)、#{{127}}(道具)。
- 同じ親の下の他の束の Issue と、同じ共有のファイルを編集しない(この Issue は `lift/plugins/` だけを変える)。衝突は起きない。
