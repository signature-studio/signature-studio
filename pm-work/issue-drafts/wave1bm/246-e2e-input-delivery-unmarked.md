---
draft: "246"
title: "E2E の input と delivery: 印の無い失敗 17 本(desk のクリック・タッチ・ホイール・F5/F6/F9・ヒントバー・カーソル・lazy の配信)を、道具と製品に切り分けて直す"
type: Task
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "133"
blocked_by: []
labels: ["verify"]
size: M
priority: Medium
project: signature-studio/3
---

## 目的

`input`(121 本)の E2E で印の無い失敗が 16 件、`delivery`(47 本)で 1 件ある。どれも、製品の機能が壊れたとは決まっていない。input は、操作が画面やエンジンに届いたかを測る道具の判定(操作の記録の読み取り・測定の位置・画素の比較)が原因の可能性がある。delivery は、判定が探す配信の項目の名前が、今のマニフェストと合わない疑いがある(推定)。この Issue は、(1) 17 本を原因ごとに切り分け、(2) 道具の誤りは直し、(3) 製品の欠陥は報告に書く。製品(`app/`・`lift/`)は変えない。

## 背景

- 全シナリオの E2E(`e2e-full` の最新の run。commit 24aeee99 の版)の結果(実測):
  - input: 1/3 で印の無い失敗 5 件、2/3 で 7 件、3/3 で 4 件。合計 16 件。
  - delivery 2/2: 印の無い失敗 1 件(`delivery/lazy-module-served`)。
- input の 16 件の内訳(実測の notes の文言で分類した。ID は本の名前からの推定。run の出力で確かめる):
  - desk の左・中・右・ダブルクリック(5 件。`mouse-left-click-desk-trace`・`mouse-middle-click-desk`・`mouse-right-click-desk-trace`・`mouse-double-click-desk`・`mouse-double-click-trace` と見られる): `records down false, up false`。押した位置(desk の中央)の押す・離す が、ページの記録に出ていない(実測)。
  - タッチの長押し・タップ(2 件。`touch-long-press-desk`・`touch-tap-desk` と見られる)。
  - ホイール(1 件。`mouse-wheel-down-piano-roll`): `records wheel -120 processed 3`。記録は 3 件あるが、ピアノロールの画は変わらない(実測)。
  - F5・F6・F9 の切り替え(3 件。`key-f5-playlist-toggle`・`key-f6-channel-rack-toggle`・`key-f9-mixer-toggle` と見られる): 1 回目は画が変わり、2 回目で元に戻らない(実測)。
  - ヒントバー(3 件。`hint-bar-hides`・`hint-bar-shows`・`hint-bar-shows-right` と見られる): `top bar 3840x82: no pixel of the bar changed`(実測)。
  - フォーカス(1 件。`mouse-focus-desk` と見られる)と、テキスト欄のカーソル(I ビーム。1 件。`cursor-text-field-ibeam` と見られる)。
- 見立て(推定): desk のクリックが記録されない原因は、(a) 測定の位置(desk の中央)の下に窓や別の要素が在る、(b) 記録の読み取りの時間が足りない、(c) 製品の入力の処理、のどれか。キーの切り替えは、2 回目の判定の仕方か、画の判定の基準か、どちらか。この run では画像もエンジンのログも取っていない(`--shots`・`--engine-logs` を使っていない)ので、確認できない。
- delivery `lazy-module-served`: 判定は、マニフェストの `lazy` の項目から path `fruity_filter_x64.7000000.dll` を探したが見つからず、status が null になった(実測: `status null, bytes null (manifest null)`)。推定: マニフェストの path の付け方が変わり、判定の名前と合わない。同じ道具の Rust の試験(`lift/crates/fe-build/src/plugins.rs`)は、この項目の名前を `effects.fruity-filter.fruity_filter_x64.dll` の形で扱う。項目が無くなったのか、名前が変わっただけなのかは、確認できない。

## 読む物

- `tools/e2e/lib/input.mjs`: `clickTrace`(desk の中央の点・押す・離す・記録の読み取り)・`wheelCase`・`hintCase`・ファンクションキーの判定(`mode` が `toggle` の関数)・タッチの関数・`deskNow`
- `tools/e2e/lib/guest.mjs`(`desk()`・`rightClick`)、`tools/e2e/lib/common.mjs`(`windowTable`)
- `tools/e2e/scenarios/input/`(16 件の本の宣言と `covers`)
- `tools/e2e/scenarios/delivery/lazy-module-served.mjs`・`tools/e2e/features/generated/delivery.tsv`(`delivery/lazy/fruity-filter-x64-7000000-dll` の行)・`tools/e2e/lib/delivery-tests.mjs`
- 製品: `app/src/bundle/manifest.ts`(マニフェストの `lazy` の形。読むだけ)、`lift/crates/fe-build/src/plugins.rs`(読むだけ)

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
1. 再現(input): 16 件を、本ごとに `--only` で、`--shots` と `--engine-logs` を付けて走らせる(例: `node tools/e2e/run.mjs --only input/mouse-left-click-desk-trace --displays 4k-dpr1 --shots <DIR> --engine-logs <DIR> --budget 20m`。`<DIR>` は `.tmp-*`)。フォアグラウンドで走らせる。
2. desk のクリック: 押した点に、どの窓が在るかを窓の表(`windowTable`)で確かめる。同じ shard の窓の中の本(`mouse-left-click-*`・`mouse-right-click-*`)が、同じ方法で記録されるかを比べる。記録されるなら、点か位置の道具の誤り。記録されないなら、製品の入力の処理として報告に書く。
3. タッチ・ホイール・F5/F6/F9・ヒントバー・フォーカス・カーソル: 同じ手順で、道具か製品か、確認できないかを切り分ける。切り分けは本ごとに 1 行、根拠の出力の行つきで報告に書く。
4. delivery: bundle の manifest(E2E の `fetch` が読む `/assets/manifest.json` と同じ物)の `lazy` の一覧から、`fruity_filter` を含む項目の path を書き出す。判定の path が一覧と違うなら、道具の誤り(判定の名前)として直す。一覧に項目が無いなら、無くなった理由を `app/src/bundle/manifest.ts` と `lift/crates/fe-build/src/plugins.rs` の読みで確かめ、報告に書く。
5. 道具の誤りを直した後は、同じ bundle で、その本をもう一度走らせて、結果を確かめる。
6. 受け入れを通す。通らない時は、落ちた ID・表示・出力の 1 行を報告に書く。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs!=null&&r.elapsedMs<=20000?0:1)' "$RUNNER_TEMP/boot.json" && (cd tools/e2e && npm ci && npx playwright install chromium) && node --test tools/e2e/test/*.test.mjs && node tools/e2e/run.mjs --area input --shard 1/3 --budget 35m --displays 4k-dpr1 --report-only > "$RUNNER_TEMP/i13.jsonl" && node tools/e2e/run.mjs --area input --shard 2/3 --budget 35m --displays 4k-dpr1 --report-only > "$RUNNER_TEMP/i23.jsonl" && node tools/e2e/run.mjs --area input --shard 3/3 --budget 35m --displays 4k-dpr1 --report-only > "$RUNNER_TEMP/i33.jsonl" && node tools/e2e/run.mjs --area delivery --shard 2/2 --budget 20m --displays 4k-dpr1 --report-only > "$RUNNER_TEMP/d22.jsonl" && node -e 'const fs=require("fs");let n=0,rows=0,bad=0;for(const p of process.argv.slice(1)){for(const r of fs.readFileSync(p,"utf8").trim().split("\n").map((l)=>JSON.parse(l))){rows++;if(r.error!==null)bad++;for(const s of r.scenarios){if(s.error!==null)bad++;if(s.expectFail===null&&s.ok!==true)n++}}}process.exit(bad===0&&rows>0&&n<=4?0:1)' "$RUNNER_TEMP/i13.jsonl" "$RUNNER_TEMP/i23.jsonl" "$RUNNER_TEMP/i33.jsonl" "$RUNNER_TEMP/d22.jsonl"
```

判定の基準:
- 受け入れの式が、終了コード 0 で終わる。
- input の 3 つの shard と delivery 2/2 の run で、row と scenario に error が無く、印の無い失敗(skipped を含む)が合計 4 本以下。
- 残った印の無い失敗は、製品の欠陥と分かった物だけ。印 `new` を付け、理由(根拠の出力の行)を報告に書く。
- 報告に: (a) 17 本の切り分けの一覧(道具・製品・確認できない の別と、根拠の出力の行)、(b) desk のクリックの切り分けの結果(押した点と、その点の窓の表の結果、同じ shard の窓の中の本との比較)、(c) delivery の lazy の path の今の値(manifest の `lazy` の該当の項目の path の行)、(d) 4 本の run の出力の行(各 run の未印の失敗の ID・ok・notes)、(e) 印 `new` を付けた本と理由。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には、runner の値であることと、nproc と loadavg を添える。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 確認できなかった物は「確認できない」と書く。
- 段ごとに、やること 0〜6 のどこまで進んだか(完了・途中・未着手)を書く。

## 制約

- 製品(`app/`・`lift/`)は変えない(読むだけ)。製品の欠陥と分かった場合は、報告に書く。製品の Issue は、この Issue では起票しない。`.github/`・`spec/` は、直す物がある時だけ変える。`tools/e2e/` は直してよい。
- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。テストのための分岐を製品に入れない。
- Python を使わない(調査と検証にだけ使う)。OCR を使わない。文字は、ゲストのメモリの文字列・窓の表・画素のハッシュで読む。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
- sleep・nohup・末尾の `&`・先頭の `timeout` は使わない。待つ時は条件で待つ。
- 一時ファイルは `.tmp-*` に置き、終わったら消す。
- commit・PR に、AI が書いたことを示す行を付けない。commit の名義は、環境の設定のまま。
- GitHub に、Issue・ラベル・コメントを書かない。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- この作業の job(GitHub Actions の runner)で、受け入れの式の全てを実行する。ビルドと E2E は、この job の中で行う。
- ビルドや E2E の長いコマンドは、フォアグラウンドで直接実行し、シェルツールの `timeout` 引数に 5400000 を渡す。背景で起動して待たない。
- ビルドは 1 回にする。tools だけを直した後は、同じ bundle(`lift/out-app/bundle/raw/fl-Oz`)で E2E を走らせて直す。
- 1 回で終わらない前提で、段ごとに commit する。再試行は前回の差分の上で続け、作業ツリーを消さない。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する(未 commit の変更が残ると、判定は失敗する)。`git add` は 1 つずつ、自分が変えたパスだけを指定する。

## 他のタスクとの関係

- #1509(topbar・page・resilience・perf)・#1507(windows・settings)・#1527(menus)・#1528(visual)・#1530(file)・#1531(settings)とは、本のファイルが重ならない。
- #1423(連続入力の間に描画の要求と仮想時計が止まる。製品の Issue)は、入力の処理に関わり得る。製品の欠陥が #1423 の症状と同じだと分かった時は、その番号を印にしてよい(症状が同じ物だけ)。
- 製品の欠陥は、この Issue では直さない。
