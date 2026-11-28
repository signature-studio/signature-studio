---
draft: "259"
title: "プラグインを足すと `preBootBytes` が増える: プラグインの DLL・wasm・seed を、起動の前に取らないようにする"
type: "Task"
milestone: "M5 内蔵のエフェクトとインストゥルメント"
parent: "021"
blocked_by: []
labels: ["plugin-builtin"]
size: "L"
priority: "Urgent"
project: "signature-studio/3"
---

## 目的

設計(依頼者の指示): 1 プラグイン 1 wasm。lazy(起動の後に取る)。起動の前には取らない。

いまは、プラグインを 1 本足すたびに、起動の前に取る量(`preBootBytes`)が増えている。第 2 波の 3 本(Chorus・Compressor・Convolver)を足した木では、起動の前の量が上限を 8,332,397 B 超えた。起動も 16〜17 秒に伸びている(P1 の 10 秒の目標に反する)。

この Issue は、プラグインの DLL・wasm・seed と Data を、起動の前に取らないようにする。起動の前に取るのは、目録(名前・基底・大きさ)だけにして、プラグインの数に依らない大きさにする。

## 背景

- 観測(#1439 の作業の報告。runner の値。この機械とは別枠): Chorus・Compressor・Convolver の 3 本を足した木で、起動の確認は engine-idle・elapsedMs 17,312。流れ A〜D の `preBootBytes` は 4 行とも 322,701,108 で、上限 314,368,711 を 8,332,397 B(2.65%)超えた。道具(`tools/plugin-run`)の 3 本は instantiated と processed が true、engineStop は null。
- `preBootBytes` の定義(読みで確かめた。`tools/smoke/welcome-flow.mjs` の READ_TIMES と、`tools/boot-check/boot-bundle.mjs`): 最初の engine-idle の時点で、`assetsReport()` の項目の `bytes`(復号後の大きさ)の合計。
- 項目は、`app/src/bundle/assets-report.ts` の `KIND_OF_LABEL` の 4 種だけ: 'engine wasm'(engine.wasm)・'DLL pack'・'seed pack'・'engine glue'。lazy の取得は、ラベルが各ファイルのパスになるので、この 4 種に入らない。読みでは、`preBootBytes` の増えは、この 4 種のどれかの復号後の大きさの増えになる。lazy の転送は、この値に数えられていないはず(確かめる)。
- PM の推定(未確認): 3 本の DLL(rebase 後の大きさ 3,495,628 B)と wasm(br の大きさ 5,324,703 B)の合計 8,820,331 B が、超過量 8,332,397 B に近い。ただし、この比べは br(圧縮後)と、計測の復号後の大きさを混ぜている。どの pack に何が入ったかは、まだ測っていない。
- 起動の前の取得の経路(読み。確かめる):
  - `lift/crates/fe-build/src/plugins.rs` の `pack_args`(684 行付近): 各 DLL は `--args` の値に `:/data/<名前>` として入り、`--lazy-dll` にも入る。exe 以外の seed と、各 wasm と、Data は `--lazy-file` になる。
  - `app/scripts/pack-assets.mjs`(79〜82 行): `--lazy-dll` の DLL を DLL pack から外す。122〜135 行: lazy の項目を作る。
  - `lift/harness/web/pack.args`: 固定の行。`fruity_delay2_x64.7800000.dll` は、`--args` の値と `--lazy-dll` の両方に在る(同じ形の先例)。
  - `--args` の値に lazy の DLL の名前が在ることが、起動の中で DLL を読んで待つ理由になるか(elapsedMs が伸びた候補。推定)。
  - `app/src/bundle/manifest.ts`: `lazy` は 1,536 件まで(136 行)。プラグインを足すと、DLL・seed・wasm・Data の数だけ増える。上限に近づかないかも確かめる。
- 上限の値: `pm-work/findings/wave2-round2.md` の `startup-cap-bytes:` の行(現行は 314368711)。この Issue の受け入れの式は、その行を読む。`tools/crash-hunt/hunt.mjs` の STARTUP_LIMIT(70 行)にも同じ値が在る。

## 読む物

- `lift/crates/fe-build/src/plugins.rs`: `pack_args`(684 行付近)、`generate`(756 行付近から。`pack.args` を書く段は 857 行付近)。
- `lift/crates/fe-build/src/main.rs`: `keep_lazies`(637 行付近)、`bundle`(673 行付近)。
- `lift/harness/web/pack.args`(固定の行。`--args`・`--lazy-dll`・`--lazy-file`)。
- `app/scripts/pack-assets.mjs`(`--lazy-dll`・`--lazy-file` の処理)。`app/scripts/pack-seed.mjs`(seed の形式。`--lazy`)。
- `app/src/bundle/manifest.ts`(`lazy` の項目と上限)、`app/src/bundle/assets-report.ts`(`KIND_OF_LABEL`)、`app/src/bundle/lazy.ts`、`app/src/main.ts`(`startLazy` と boot の `refs`)。
- `tools/boot-check/boot-bundle.mjs`、`tools/smoke/welcome-flow.mjs`、`tools/plugin-run/run.mjs`、`tools/crash-hunt/hunt.mjs`(70 行)。

## やること

1. 測る(製品のビルドは、この Issue の作業ジョブで行う)。作業者の木の bundle(`lift/out-app/bundle/raw/fl-Oz`)で:
   a. 起動の前の 4 種の大きさ(復号後)を、manifest の `engine.wasm.size`・`dllPack.size`・`seedPack.size`・`engine.options.glue.size` から取り、表にする。
   b. DLL pack と seed pack の中の項目(名前と大きさ)を、pack の形式(`app/src/` の pack の読み込みと `app/scripts/pack-seed.mjs`)から一覧にする。
   c. プラグインの本体・補助の DLL、wasm、seed 行、Data が、どの pack に入っているかを、項目ごとに表にする。
   d. 上の結果から、起動の前の 4 種に入っている、プラグイン由来の項目の大きさの合計を出す。その合計が、超過量(8,332,397 B。#1439 の値)と合うかを報告に書く。合わなければ、合わないと書き、残った物を書く。
   2 回の bundle(足す前と足した後)は作らない(`fe-build engine` が 1 回 40〜75 分かかり、3 時間 15 分に収まらない)。足す前の大きさは、b と c の一覧から、プラグイン由来の項目を引いて求める。
2. 直す。c で見つかった、起動の前に入るプラグイン由来の項目を、起動の前の 4 種から外し、lazy にする。起動の前に要るのは目録(名前・基底・大きさ)だけにする。
   - エンジンの `--args` の値に入れる名前が、起動の中で lazy のファイルを読ませて待たせるなら、その扱いも直す。
   - 確かめる: プラグインを要求した時(`LoadLibraryW`)に、lazy の取得が起きること。`tools/plugin-run` で確かめる。道具の出力に、その場面で取った lazy の名前を足すなら、その変更はこの Issue の範囲に入る。
   - `manifest.lazy` の件数を報告に書く(上限 1,536 を超えないこと)。
3. 上限: 直した後の bundle の `preBootBytes` を測り、新しい上限を、その値の 1.05 倍(小数点以下を切り上げ)にする。`pm-work/findings/wave2-round2.md` の `startup-cap-bytes:` の行と、`tools/crash-hunt/hunt.mjs` の STARTUP_LIMIT を、新しい値にする。報告に、直した後の値と新しい上限を書く。上限は、この Issue のあとに、ほかの Issue の式が使う値になる。
4. 試験: `lift/crates/fe-build/src/plugins.rs` の tests に、名前が `start_up_` で始まる試験を 1 件以上足す。中身は作業者が選ぶ(例: `pack_args` の `--args` の値に、プラグインの DLL の名前が入らないこと。または、起動の前の 4 種に対応する行の一覧を検査すること)。直す前の木で赤になることを、報告に書く。

## 受け入れ

```accept
test -f pm-work/findings/wave2-round2.md && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && cargo test --locked --manifest-path lift/Cargo.toml -p fe-build start_up_ 2>&1 | grep -q 'test result: ok. [1-9]' && rm -rf lift/out-app/b2d-src lift/out-app/gen && node tools/place-plugin-dlls.mjs && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=20000?0:1)' "$RUNNER_TEMP/boot.json" && node tools/plugin-run/run.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" "Effector" "Emphasis" "FL Studio Mobile Rack FX" "Fruity Balance" "Fruity Filter" --scene c > "$RUNNER_TEMP/plugin-run.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));process.exit(L.length===5&&L.every((r)=>r.instantiated===true&&r.processed===true&&r.error===null&&r.engineStop===null)?0:1)' "$RUNNER_TEMP/plugin-run.jsonl" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const cap=Number(fs.readFileSync("pm-work/findings/wave2-round2.md","utf8").match(/^startup-cap-bytes: (\d+)$/m)[1]);const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true&&r.elapsedMs<=20000&&r.preBootBytes<=cap&&(!/^[BD]$/.test(r.flow)||r.welcomeClosed===true);process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(L.find((r)=>r.flow===f)))?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && bash guard/scan.sh
```

判定の基準:
- 試験 `start_up_` が 1 件以上通る(`cargo test`)。
- 起動確認(`boot.json`): engine-idle・standIn が false・error が null・elapsedMs が 20,000 以下。
- 道具(`plugin-run`)の 5 行(Effector・Emphasis・FL Studio Mobile Rack FX・Fruity Balance・Fruity Filter): instantiated と processed が true、error と engineStop が null。
- 流れ A〜D の 4 行: 条件(error・state・standIn・trapped・hostErrors・opened・elapsedMs・welcomeClosed)が通り、preBootBytes が `startup-cap-bytes` 以下。
- 起動の前の 4 種に残る、プラグイン由来の大きさが 1,048,576 B(1 MB)以下。0 件が望ましい。残りは表で示す(「やること」の 1 の d)。
- `npm test --prefix app` が通る。`bash guard/scan.sh` の指摘が 0 件。
- 差分: 上限の値は、`pm-work/findings/wave2-round2.md` と `tools/crash-hunt/hunt.mjs` の 2 箇所だけ。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書く。
- 報告に書く: (a) 起動の前の 4 種の表(項目・pack・復号後の大きさ・lazy か)、(b) プラグイン由来の残りの大きさと、超過量(#1439 の値)との比べ、(c) 起動確認の elapsedMs、流れ A〜D の 4 行、道具の 5 行、(d) 直した後の `preBootBytes` と、新しい上限(findings と hunt.mjs に書いた値)、(e) `manifest.lazy` の件数、(f) 試験 `start_up_` の結果(直す前は赤、直した後は緑)、(g) `bash guard/scan.sh` の結果。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告の末尾に、判定が通って push されると `deploy.yml` が動く(`lift/` に触れるため)と書く。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。上限は定数にし、オプションにしない。
- Python・OCR を使わない。
- sleep・timeout のコマンド・nohup・末尾の `&` を使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- `git add` は、変えたファイルを 1 つずつ指定する(`git add -A`・`git add .` は使わない)。
- commit の名義は、環境の git の設定のまま。commit・タグ・PR に、AI の署名の行を付けない。必要の無いブランチを作らない。
- この機械で製品のビルドや測定をしない。製品の作業は Actions の作業ジョブで行う。

## 作業の進め方

- 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば最初に読む。
- ビルド(`fe-build engine`)は 1 回にする。長いコマンドはフォアグラウンドで直接、シェルツールの `timeout` 引数に 5400000 を渡す。背景で起動して待たない。
- 作業を始めてから 3 時間 15 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(判定に約 2 時間 30 分掛かる)。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。

## 他のタスクとの関係

- blocked_by: なし。
- 第 2 波の束(内蔵エフェクト・内蔵インストゥルメントの Issue)は、この Issue を blocked_by に持つ。この Issue が閉じるまで、それらは起動しない。
- #{{260}}(プラグインの再生・描画の停止)は、この Issue と並行してよい(blocked_by は無い)。#{{261}}(束から外した plugin)は、#{{260}} が閉じるまで起動しない。
- 衝突: `lift/crates/fe-build/src/plugins.rs` は、#{{260}} も触るかもしれない。衝突は、後から入る方が直す。
