---
draft: "192"
title: "内蔵プラグインの Data(551 件・22 フォルダ)を、プラグインごとの `data.tsv` で宣言し、共有の seed 表に行を足さずに、起動の後の取得で配信する"
type: Task
milestone: "M5 内蔵のエフェクトとインストゥルメント"
parent: "018"
blocked_by: []
labels: ["plugin-builtin", "verify"]
size: M
priority: High
project: signature-studio/3
---

## 目的

依頼者の指示: 内蔵のプラグインを、起動の後にバックグラウンドで届ける(1 プラグイン 1 wasm)。プラグインの Data(`Plugins\Fruity\<種別>\<フォルダ>\Data\` の下のファイル)も、プラグインと同じく起動の後に届ける。

いまの仕組みでは、Data の行は共有のファイル(`lift/harness/seeds/vfs-seed-idle1.tsv` と `lift/harness/web/pack.args`)に書く。第 2 波の束が書くと、並行の束が衝突する。この Issue は、Data の宣言をプラグインのフォルダの中(`lift/plugins/<名前>/data.tsv`)に置き、生成の段がその行から seed の行と `pack.args` の組を作るようにする。

この Issue の目標は次の 3 つ。
1. `lift/plugins/<名前>/data.tsv` の形を決め、取り出しの道具(`app/scripts/lib/plugin-rows.mjs`)と生成の段(`lift/crates/fe-build/src/plugins.rs`)が、その行から配信の行を作ること。新しい Data の行は、共有の表と `pack.args` に 1 行も足さないこと。
2. Sytrus の Data(109 件。#{{171}} が共有の表に足した行)を `lift/plugins/sytrus/data.tsv` に移し、共有の表と `pack.args` から外すこと。移す前後で、bundle の lazy の sha256 の集合が同じであること。
3. 第 2 波の Data(551 件)を入れた時の件数が、上限(下の「やること」の 4)に収まること。第 2 波の束の `data.tsv` は、この Issue では書かない。

## 背景

- 第 2 波の束(`pm-work/issue-drafts/wave2/`)のうち、Data を持つ束は 19 件(下書き 042・046・050・057・059・061・063・068・070・072・074・075・076・077・078・079・080・083・084)。Data は 22 フォルダ・551 件・計 約 9.4 MB。Harmor(`Plugins\Fruity\Generators\Harmor\Data\`。179 件・約 8.2 MB)を含む。
- 事実(#{{171}} の経緯): Sytrus の Data の 109 件は、共有の表に `@lazy-file:` の行として足して配信した。この方法は、束ごとの衝突を生むので、第 2 波には使えない。
- 取り出しの経路: `ci/plugin-builtin-prefixes.txt` の `Plugins\Fruity\Effects\` と `Plugins\Fruity\Generators\` の接頭辞で、Data は既に取り出しの木に入る。ただし配信の行が指す実物は、`app/scripts/extract-fl.mjs` の `--seed` が作る(行の host の綴りで置く)。だから `ci/` は変えない。
- 既存の部品: `app/scripts/lib/plugin-rows.mjs` の `pluginRows()` は、`lift/plugins/<名前>/plugin.tsv` から、取り出しの行と seed の行を返す(`extract-fl.mjs` が読む)。`lift/crates/fe-build/src/plugins.rs` の `generate()` は、生成の seed 表と `seed.args` を書き、`pack_args()` は `pack.args` の `--lazy-file` の組を足す。2 つは同じ行を作らなければならない(両方の注釈が定める)。`spec/plugin-rows.test.mjs` は、JS の行と、`plugins.rs` の試験の期待値が一致するかを確かめる。
- `@lazy-file:` の行は、`app/scripts/pack-seed.mjs` が pack に入れず、sha256 を検査する。`--lazy-file SRC=PATH` の組は、`app/scripts/pack-assets.mjs` が manifest の `lazy` に 1 項目ずつ入れる。
- 現在の上限(既定の値): `app/src/bundle/manifest.ts` の lazy の `maxItems` は 256。`lift/runtime/fle_vfs.c` の `LAZY_MAX_STREAMS` と `lift/harness/vfs_lazy.h` の `VL_REG_MAX` は 1024(seed 表の行は 1 行 1 stream と書かれている)。`lift/harness/vfs_seed.h` の `more than %u seed files` の判定は `VL_REG_MAX` で、`VS_MAX_ENTRIES` は 2,048。seed 表は今 675 行(うち `@lazy-file:` が 224 行)。
- 件数の見積もり(目安。作業者が数えて報告する): 生成の段は 1 DLL につき lazy を 3 件足す(`--lazy-dll` の本体・seed の `--lazy-file`・wasm の `--lazy-file`)。第 2 波の 159 DLL で 477 件。Data は 551 件。いまの `pack.args` の `--lazy-file` は 229 行。合計は 1,250 件前後になり、256 も 1024 も超える。
- lazy の仕組み: `manifest.lazy` の各項目は、ファイルごとの URL・大きさ・sha256 を持つ。`app/src/main.ts` は、エンジンが起動した時に、manifest の順に全件の取得を始める。`app/src/engine/lazy-file.ts` は、ファイルごとに、guest の読みの時に待つ。もう 1 つの型は `app/src/engine/lazy-tree.ts`(decision 0314)で、1 つの index と、その下の stream をチャンクで取る。件数が多すぎる時は、この 2 つを比べる(下の「やること」の 5)。

## 読む物

- `app/scripts/lib/plugin-rows.mjs`(`pluginRows()`・`parsePlugin()`・`validate()`)、`app/scripts/extract-fl.mjs`(`--seed` と `@lazy-file:` の扱い)、`spec/plugin-rows.test.mjs`
- `lift/crates/fe-build/src/plugins.rs`(`generate()`・`pack_args()`・`seed_row()`・`seed_file()`・`guest_path()`・試験)、`lift/crates/fe-build/src/main.rs`(`seed_step()`・`pack_args()`・`keep_lazies()`)
- `lift/plugins/sytrus/plugin.tsv`(書式の見本)、`lift/plugins/fruity-balance/plugin.tsv`
- `lift/harness/seeds/vfs-seed-idle1.tsv`(`@lazy-file:` の行と、先頭の注釈。Sytrus の Data の行)、`lift/harness/web/pack.args`(`--lazy-file` の組)
- `app/scripts/pack-seed.mjs`・`app/scripts/pack-assets.mjs`(`--lazy-file`)
- `app/src/bundle/manifest.ts`(`lazy` の `maxItems`・`lazySchema`。name は 100 字まで、path は 256 字まで)
- `app/src/main.ts`(lazy の取得の開始)、`app/src/engine/lazy-file.ts`、`app/src/engine/lazy-tree.ts`、decision 0314 の記録
- `lift/runtime/fle_vfs.c`(`LAZY_MAX_STREAMS`)、`lift/harness/vfs_lazy.h`(`VL_REG_MAX`)、`lift/harness/vfs_seed.h`(`VS_MAX_ENTRIES`・`more than %u seed files`)
- `inputs/fl-installer-files.tsv`(一覧。`Plugins\Fruity\` の下の `\Data\` の行)
- `ci/plugin-builtin-prefixes.txt`・`ci/resources-prefixes.txt`(変えない。`ci/resources-prefixes.txt` の完全なパスの行の合計は 30 MB 以下。`spec/workflow.test.sh` の (r5) が検査する)
- `tools/boot-check/boot-bundle.mjs`・`tools/smoke/welcome-flow.mjs`・`tools/p3-audio-probe/measure.mjs`
- `gh api repos/{owner}/{repo}/issues/<#{{171}} の番号>` の body と、最後のコメント(Sytrus の Data の経緯)

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
1. `data.tsv` の形を決める。`lift/plugins/<名前>/data.tsv` は、1 プラグインの Data。無いフォルダは Data が無い。見出し行と TAB 区切り。列は 4 つ: `guest`(ゲストのパス。`Plugins\Fruity\<種別>\<フォルダ>\Data\…`)・`installer`(`inputs/fl-installer-files.tsv` の行と同じ綴り。取り出しは、この一覧で照合する)・`sha256`(一覧の値)・`size`(一覧の値。正の整数)。`#` で始まる行は注釈。検査は `plugin.tsv` と同じく、行の形・一意性・一覧との照合を行い、`pluginRows()` と `plugins.rs` の両方で同じ行を作る。
2. 配信の行を作る。
   - 取り出し: `pluginRows()` が Data の行を `seed` に足す。host の綴りは `@lazy-file:plugins/` と guest のパスを連ねたもの。空白は `_` に換える(seed の `--lazy` の行は空白で区切って読まれる。#{{171}} の観測)。guest の綴りは元のまま。`extract-fl.mjs --seed` がこの行で実物を置く。
   - 生成: `plugins.rs` の `generate()` が、Data の行を生成の seed 表に `@lazy-file:` の行として足す。`pack_args()` は、同じ件数の `--lazy-file SRC=PATH` の組を足す(SRC は取り出しの実物、PATH は `seed/` の下)。
   - 共有のファイルは、この 2 つ(`plugin-rows.mjs` と `plugins.rs` の生成)を除き、新しい行を足さない。
3. Sytrus の Data を移す。`lift/harness/seeds/vfs-seed-idle1.tsv` と `lift/harness/web/pack.args` から、`Plugins\Fruity\Generators\Sytrus\Data\` の 109 行(と、それぞれの `--lazy-file` の組)を外し、`lift/plugins/sytrus/data.tsv`(109 行)に移す。移す前後で、bundle の lazy の sha256 の集合が同じであること(順序は問わない)。行の数(109)と、合計の大きさ(1,229,971 B)が一致すること。
4. 上限を、第 2 波の全件を入れた数まで上げる(既定の値。オプトインを作らない)。
   - `app/src/bundle/manifest.ts` の lazy の `maxItems`
   - `lift/runtime/fle_vfs.c` の `LAZY_MAX_STREAMS` と `lift/harness/vfs_lazy.h` の `VL_REG_MAX`(同じ数を覆う)
   - `lift/harness/vfs_seed.h` の `VS_MAX_ENTRIES`(今の 2,048 で、seed 表の行数を覆うかを確かめる)
   - `app/scripts/pack-seed.mjs` に件数の上限があれば、それも上げる(無ければ報告に無いと書く)
   - 件数は、`plugins.rs` の試験で数える: tempdir に第 2 波の Data 551 件を宣言する `data.tsv` を作り、生成の段の関数で、seed の行と `--lazy-file` の組の件数を求め、上限と比べる。実物の bundle は、Sytrus の 109 件だけで測る。
5. manifest の大きさを測る。`manifest.json` は起動の前に取られる。今の値(Sytrus の 109 件だけ)と、第 2 波の件数を入れた試験の値の大きさを比べて、報告に書く。`preBootBytes` が上限を超えるなら、次の 2 つを比べた設計の比較を `pm-work/findings/plugin-data-delivery.md` に書き、commit に含める: (a) いまの型(manifest の項目を件数分増やす)、(b) decision 0314 の型(`lazy-tree.ts` の index 1 つと、その下の stream)。どちらを既定にするかは、測った値で決め、理由を書く。どちらも製品の既定の動作にする。
6. 測る。`tools/boot-check/boot-bundle.mjs`(起動)・`tools/smoke/welcome-flow.mjs`(流れ A〜D)・`tools/p3-audio-probe/measure.mjs`(Sytrus の demo 2 件)。判定の基準を通すこと。

## 受け入れ

```accept
npm run build --prefix app && npm test --prefix app && node spec/plugin-rows.test.mjs && cargo test --locked --manifest-path lift/Cargo.toml -p fe-build && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=12500?0:1)' "$RUNNER_TEMP/boot.json" && node -e 'const fs=require("fs");const m=JSON.parse(fs.readFileSync(process.argv[1],"utf8"));const want=[];for(const d of fs.readdirSync("lift/plugins")){const p="lift/plugins/"+d+"/data.tsv";if(!fs.existsSync(p))continue;for(const l of fs.readFileSync(p,"utf8").split(/\r?\n/)){if(l.trim()===""||l.startsWith("#")||l.startsWith("guest\t"))continue;want.push(l.split("\t")[2]);}}const have=new Set((m.lazy??[]).map((l)=>l.sha256));const table=fs.readFileSync("lift/harness/seeds/vfs-seed-idle1.tsv","utf8");process.exit(want.length===109&&want.every((s)=>have.has(s))&&!/Sytrus\\Data\\/.test(table)?0:1)' "$PWD/lift/out-app/bundle/raw/fl-Oz/manifest.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true&&r.elapsedMs<=12500&&r.preBootBytes<=314368711;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && mkdir -p lift/out-app/host/resources && cp -a lift/out-app/host-extra/resources/. lift/out-app/host/resources/ && mkdir -p .tmp-sytrus-data && cp "lift/out-app/host/resources/Data/Demo projects/Product demos/Synth_dfr - Overcome - Sytrus.flp" .tmp-sytrus-data/synth.flp && cp "lift/out-app/host/resources/Data/Demo projects/Product demos/Olbaid - Sytrus Essentials.flp" .tmp-sytrus-data/olbaid.flp && node tools/p3-audio-probe/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" "$PWD/.tmp-sytrus-data/synth.flp" > "$RUNNER_TEMP/open-synth.json" && node tools/p3-audio-probe/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" "$PWD/.tmp-sytrus-data/olbaid.flp" > "$RUNNER_TEMP/open-olbaid.json" && node -e 'const fs=require("fs");const L=["synth","olbaid"].map((f)=>JSON.parse(fs.readFileSync(process.env.RUNNER_TEMP+"/open-"+f+".json","utf8").trim()));process.exit(L.every((r)=>!JSON.stringify(r).includes("empty path")&&!JSON.stringify(r.inventoryList??[]).includes("Sytrus"))?0:1)'
```

判定の基準:
- 起動の確認(`tools/boot-check/boot-bundle.mjs`): `state` が engine-idle、`standIn` が false、`error` が null、`elapsedMs` が 12,500 以下(DPR 1)。
- 宣言の配信: `lift/plugins/*/data.tsv` の全行の sha256 が、`manifest.json` の `lazy` に在る(Sytrus の 109 件)。共有の seed 表に `Plugins\Fruity\Generators\Sytrus\Data\` の行が 0 件。
- 流れ A〜D(`tools/smoke/welcome-flow.mjs`)の 4 行: 各行で `error` が null、`state` が engine-idle、`standIn` が false、`trapped` が false、`hostErrors` が 0、`opened` が true、`elapsedMs` が 12,500 以下、`preBootBytes` が 314,368,711 以下。B と D は `welcomeClosed` が true。
- Sytrus の demo(synth・olbaid)の測定: 出力に `empty path` が無い。`inventoryList` に Sytrus の行が無い。
- 単体と試験: `npm test --prefix app`・`spec/plugin-rows.test.mjs`・`cargo test -p fe-build`(第 2 波の件数の試験を含む)が通る。
- 差分: `lift/plugins/sytrus/data.tsv` を除き、新しい `lift/plugins/*/data.tsv` は足さない(第 2 波の宣言は、各束が書く)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: `data.tsv` の形(列と検査)、配信の行の数(Sytrus の 109 件)、`pack.args` の `--lazy-file` の数、上げた上限の値(`maxItems`・`LAZY_MAX_STREAMS`・`VL_REG_MAX`・`VS_MAX_ENTRIES`・pack-seed の上限の有無)と、数えた件数(第 2 波の件数を入れた試験の値)、`manifest.json` の大きさ(今の値と、第 2 波の件数を入れた値)、`preBootBytes` と `elapsedMs` の値(流れ A〜D の 4 行をそのまま)、Sytrus の移しの前後の sha256 の集合が同じかどうか、比較した設計(5 を使ったなら)。
- `.github/` の鍵(`extract-tree-` と `extra-plugins-` の hashFiles)には、`lift/plugins/*/data.tsv` が入っていない。この作業者は `.github/` を変えない。報告に「PM が鍵に足す必要がある」と書く。
- `lift/`・`app/`・`ci/`・`spec/` を変えたことは、push で `deploy.yml` を動かすので、報告に書く。

## 制約

- 製品に、新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。上限の変更は既定の値として行い、オプトインを作らない。
- Data の行を、共有の表(`vfs-seed-idle1.tsv`・`pack.args`)に新しく足さない(Sytrus の移しで外すだけ)。
- `ci/`(接頭辞の一覧)と `.github/` を変えない。
- 空の代替(空ファイル・0 埋め)を置かない。大きさと sha256 は、一覧(インストーラの実物から取った値)と照合したものだけを置く。
- Python と OCR を使わない。AI のベンダー名と、Cloudflare の識別子を、ファイルに書かない(`guard/scan.sh` が検査する)。URL を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通っていなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- エンジンのビルドは約 35 分。フォアグラウンドで直接実行し、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。背景で起動して待たない。`sleep`・`timeout` をコマンドの先頭に書かない。
- 先に、`plugin-rows.mjs` と `plugins.rs` の試験で、Data の行の形と件数を確かめてから、bundle のビルドに進む。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。

## 他のタスクとの関係

- blocked_by: なし。この Issue は、第 2 波のうち Data を持つ束の前提。第 2 波の束の本文は、PM が Data の範囲を直す(この作業者は、第 2 波の束の本文と `data.tsv` に触れない)。
- #{{171}}(Sytrus の Data の配信)。この Issue は、その配信の仕組みを宣言の形に置き換える。
- #{{193}}(`plugins.rs` の試験の直し)と、同じファイルを変える。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
