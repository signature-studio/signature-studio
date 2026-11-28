---
draft: "260"
title: "プラグインの再生・描画で、間接呼び出しの呼び先が画素・文字の形になって engine が止まる: 書いた場所を特定して直す"
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

内蔵のエフェクトを再生・描画すると、engine が決定的に止まる(間欠ではない)。止まりの型が不自然である。呼び先の値(間接呼び出しの行き先)が、関数の番地ではなく、画素(BGRA の 1 画素の複製)や文字(UTF-16LE の文字)の形になっている。

関数の番地を入れる表(vtable・IAT・コールバックの表)のあるメモリに、画素や文字が書かれた疑いが強い(推定。未確認)。この Issue は、その書き込みの元を特定し、直す。

直すのは、原因が確定してから。原因を特定できたが直せなかった時は、原因の記録を報告の先頭に書いて終える(下の「報告の規則」)。

## 背景

- 止まりの文言(`lift/runtime/fle_dispatch.h` の `fle_dispatch_stop`): 標準エラーに、`fle: regs R[0]=… R[15]=…` の行と、`fle: indirect call to <値> is not a known function entry (caller <戻り番地>, near f_<関数>+<オフセット>)` の行が出て、abort する。`tools/plugin-run` の `engineStop` と `lastFle` に入る。`caller` は、間接呼び出しの戻り番地。`near` は、その番地を含む lift の関数。
- 観測の値(各束の作業者の報告の値。runner の値。この機械とは別枠):

| 型 | 値 | 観測した木 |
| --- | --- | --- |
| 画素の形(B・G・R・A の 1 画素の複製) | `0xd5ffccd1d5ffccd1` | Control Surface・Distructor(#1434) |
| 同 | `0x53ff444b51ff4d54`・`0x37ff2b3337ff2c33` | Fruity LSD(#1443) |
| 同 | `0x36ff2a3136ff2a31` | Fruity NoteBook(#1444) |
| 同 | `0x59ff676259ff6762` | FL Studio Mobile Rack FX だけの木(#1436) |
| 文字の形(UTF-16LE の文字の並び) | `0x5300464f5400464e`・`0x5400464f5400464f` | Fruity HTML NoteBook(#1442) |
| 値が 0 | `indirect call to 0` | Effector・Emphasis(#1435 の観測)。ほかに #1436・#1438・#1443・#1444・#1446 の束の報告に出た(どの plugin かは、各束の報告による。この表では確認できない) |
| 像の中の番地(入口が `.pdata` に無い) | `0x13447d38`(#1436。17 回出た。ildsp の像の中) | #1436 の、ildsp を使う木(Emphasizer・Frequency Shifter) |
| 同 | `0xf7e6c88` | Frequency Splitter(#1437) |
| 同 | `0x11933b40` | Fruity Parametric EQ 2(#1446) |

- 読みの仮説(推定。確かめる):
  - 画素・文字の値: 表のあるメモリへの書き込み。描画のバッファの溢れ(書く量が確保した量より大きい)、画素の幅・stride・ビット深さの計算の誤り、文字列を書く API の長さの誤り、が候補。描画の実装は `lift/runtime/fle_gdi.c`(DIB 系の関数。`grep -rn 'DIBSection\|StretchDIBits\|SetDIBits\|GetDIBits' lift/runtime` で探す)。
  - 値が 0: `lift/harness/web/pack.args` の `--stop-note`(`indirect call to 0`)は、「GetProcAddress が求められた関数に 0 を返した」型だと書く。古い注記で、今の原因とは限らない。直前の GetProcAddress の名前を読む。
  - 像の中の番地: 関数の入口が `.pdata` に無い。#1395・#1429・#1518 の仕組み(`--reloc-seeds`・`--rva-tables`・`seeds.tsv`)で解く型。`ildsp_x64.dll` は #1539 で `--reloc-seeds` を足した(commit `3deea975`)が、`0x13447d38` が今も出るかは確かめていない。
- 再現の入力: main に既に `lift/plugins/` の行が在る Effector・Emphasis・FL Studio Mobile Rack FX(と、既存の Fruity Balance・Fruity Filter を対照に使う)。

## 再現

1. `node tools/place-plugin-dlls.mjs`、`lift/target/release/fe-build engine`、`lift/target/release/fe-build bundle` で bundle を作る(`cargo build --release --locked --manifest-path lift/Cargo.toml` の後)。
2. `node tools/plugin-run/run.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" "Effector" "Emphasis" "FL Studio Mobile Rack FX" --scene d` を実行する。`--scene c` も同じ名前で実行する。
3. 出力の `engineStop`(stop・message・returnAddress・registers)と `lastFle` を、報告に貼る。

## やること

1. 再現する(上の 2〜3)。どの型で止まるか(画素・文字・0・像の番地)を、木ごとに報告に書く。再現しなければ、出力を報告して止まる。
2. 呼び先の読み元を特定する(診断の差分。commit しない):
   a. 停止の `caller`(戻り番地)の直前の `call` 命令を、そのプラグインの DLL の逆アセンブルで読み、被演算子 `[reg+off]` を出す。`near f_<関数>+<オフセット>` の関数を、lift の出力(`lift/out-app/gen/` の C)で探す。
   b. その番地(表の番地)が、どのメモリのブロックに属するかを、停止のときに出す(確保器のブロック、スタック、像、画素のバッファの、どれか)。その番地の周りの 64 バイトも出す。
   c. 書き込みの元を特定する: 表の番地への書き込みを、書いた命令の番地と共に記録する(診断版のみ)。画素の形なら、どの描画の API のバッファか(幅・高さ・stride・ビット深さ・確保した量と、書いた量)。文字の形なら、どの文字列の API か(長さの計算)。
3. 直す(原因が確定してから)。試験を先に赤にする: 再現の入力を使う試験を、`spec/` か `lift/crates/fe-lift/tests/` に 1 件書き、直す前の木で赤になることを報告に書く。試験の作り方は作業者が選ぶ。
4. 値が 0 の型: GetProcAddress が 0 を返した名前を特定する。runtime に無い名前なら、その名前を実装するか、名前を言う停止にする(黙って 0 を返さない)。
5. 像の中の番地の型: `ildsp_x64.dll` の `0x13447d38` が、`--reloc-seeds` で解けたかを確かめる。解けなければ、`lift/plugins/lameenc/seeds.tsv` を見本に、`lift/plugins/ildsp/seeds.tsv`(RVA の列)を足す。RVA は、ildsp の像の基底からの差で求める。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && node tools/place-plugin-dlls.mjs && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=20000?0:1)' "$RUNNER_TEMP/boot.json" && node tools/plugin-run/run.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" "Effector" "Emphasis" "FL Studio Mobile Rack FX" "Fruity Balance" "Fruity Filter" > "$RUNNER_TEMP/plugin-run.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));process.exit(L.length===5&&L.every((r)=>r.instantiated===true&&r.processed===true&&r.error===null&&r.engineStop===null)?0:1)' "$RUNNER_TEMP/plugin-run.jsonl" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const cap=Number(fs.readFileSync("pm-work/findings/wave2-round2.md","utf8").match(/^startup-cap-bytes: (\d+)$/m)[1]);const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true&&r.elapsedMs<=20000&&r.preBootBytes<=cap&&(!/^[BD]$/.test(r.flow)||r.welcomeClosed===true);process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(L.find((r)=>r.flow===f)))?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && bash guard/scan.sh
```

判定の基準:
- 起動確認(`boot.json`): engine-idle・standIn が false・error が null・elapsedMs が 20,000 以下。
- 道具(`plugin-run`、場面は既定の auto)の 5 行(Effector・Emphasis・FL Studio Mobile Rack FX・Fruity Balance・Fruity Filter): instantiated と processed が true、error と engineStop が null。
- 流れ A〜D の 4 行: 条件が通り、preBootBytes が `pm-work/findings/wave2-round2.md` の `startup-cap-bytes` 以下。
- `npm test --prefix app` が通る。`bash guard/scan.sh` の指摘が 0 件。
- 上の受け入れは、直した後の木で通ること。診断の差分は、commit に入れない。
- 上の 5 行は、再現で落ちた 3 本と、対照の 2 本を見る。他の型(Fruity Big Clock など)は #{{261}} で扱う。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書く。
- 報告に書く: (a) 再現の出力(`plugin-run` の `engineStop`・`lastFle`。木ごと・場面ごと)、(b) 読み元の表(値・caller・near・番地・属するブロック・書いた命令の番地・書いた API)、(c) 直し方と、その理由(原因が確定した根拠)、(d) 試験の赤と緑の出力、(e) 直した後の `plugin-run` の 5 行、(f) 起動確認の elapsedMs、流れ A〜D の 4 行、preBootBytes、(g) 値が 0 の型と、像の番地の型の結果(解けたか、どの行を足したか)、(h) `bash guard/scan.sh` の結果。
- 原因が確定できず直せなかった時は、(b) の表と、次に確かめることを、報告の先頭に書いて終える。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告の末尾に、判定が通って push されると `deploy.yml` が動く(`lift/` に触れるため)と書く。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。診断の差分は、commit しない。
- 製品の既定の動作を、測定のためだけに変えない。
- Python・OCR を使わない。
- sleep・timeout のコマンド・nohup・末尾の `&` を使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- `git add` は、変えたファイルを 1 つずつ指定する(`git add -A`・`git add .` は使わない)。
- commit の名義は、環境の git の設定のまま。commit・タグ・PR に、AI の署名の行を付けない。必要の無いブランチを作らない。
- この機械で製品のビルドや測定をしない。製品の作業は Actions の作業ジョブで行う。

## 作業の進め方

- 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば最初に読む。
- ビルド(`fe-build engine`)は 1 回にする。再現と読み元の特定は、その 1 回の bundle で行う。直した後の確認は、作業の中で 2 回目のビルドをせず、判定(受け入れ)に任せる。長いコマンドはフォアグラウンドで直接、シェルツールの `timeout` 引数に 5400000 を渡す。背景で起動して待たない。
- 作業を始めてから 3 時間 15 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(判定に約 2 時間 30 分掛かる)。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。

## 他のタスクとの関係

- blocked_by: なし。
- #{{259}}(起動の前に取る量)と並行してよい。両方が `lift/crates/fe-build/src/plugins.rs` を触るかもしれないので、commit の前に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直す。
- 第 2 波の #1435・#1436 は、この Issue を blocked_by に持つ(起票の後に設定する)。この Issue が閉じるまで起動しない。
- #{{261}} は、この Issue が閉じるまで起動しない(#{{261}} の blocked_by)。
