---
draft: "188"
title: "リフ マシーンで Accept を押すと、Piano roll にノートが作られる(`notes` ≥ 1)"
type: Task
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: ["187", "147"]
labels: ["resources", "verify", "plugin-builtin"]
size: M
priority: High
project: signature-studio/3
---

## 目的

Riff machine(リフ マシーン)の窓の主ボタン(Accept)を押すと、Piano roll にノートが作られること(`notes` が 1 以上)。

この Issue の目標は次の 2 つ。
1. Accept の位置を、窓の表と画面の変化から見つける(座標や比率を定数にしない)。
2. Accept の後に、Piano roll の窓の矩形で、後にだけ現れたノートの色の連結成分を数え、1 以上になること。

## 背景

- #{{147}}(#1387)の段階 1 で、窓は出て描かれる(run 45592009894)。ただし、Accept を押した後の画面は、まだ取れていない。窓の後に Sytrus の呼び出しで engine が止まるため(#{{187}})。
- 数え方は、#{{147}} の本文の「数え方(PM の決定)」と同じ。`countNotes(before, after, rect)` は、道具 `tools/piano-roll-tools/measure.mjs` に実装済み(#{{147}} の差分)。合成データ 15 件で確かめた(`spec/piano-roll-notes.test.mjs`)。
- 前回の観測(run 45394680999): Accept を押すと、窓は閉じ、Piano roll(「Piano roll - Riff Machine - Squelch」)に F4 付近の音符の帯が画面に出た。画素の読みによる。数えていない。
- Accept のボタンは、画面の右下の「Accept」。窓の表に文字は出ない(PNG で確かめた)。前回は、窓の矩形の比率で押した。この Issue では比率を定数にせず、見つける形にする。
- ノートの色は、実測で決める(前回の判断。定数や表に埋め込まない)。
- #1386(コード進行ツール)も、同じ `countNotes` を使う(前回の判断)。

## 前回までの試行で分かったこと

(run 45394680999 と run 45592009894。#1387 の観測。この Issue の元)

1. 窓の矩形(run 45592009894・DPR 1): (751,275)・334x506。DPR 1.32: (1045,444)・334x506。
2. run 45592009894 では、Accept を押した後の画面を取れていない(窓の後の止まりのため)。`notes` 0 は数えた値ではない。
3. run 45394680999: Accept を押すと、窓は窓の表から消える(閉じる)。Piano roll に、F4 付近の音符の帯が出た。数えていない。
4. 保存(Ctrl+S)では、保存のダイアログも保存の記録も出なかった(タイトルに Trial。試用版では保存が無効と見られる。推定)。保存で数える経路は使えない。
5. 画素の変化の総数では、ノートの数の判定に使えない(前回の判断)。
6. 合成データ(`spec/piano-roll-notes.test.mjs`・15 件): 10x6 の新しい色の塊 1 つ → notes 1。変化なし → 0。3x3 の点は数えない(4 px 未満)。角だけで接する 2 塊 → 2。
7. 合成データ: 最頻の新しい色(黄)の 10x6 は数え、緑の 3x9 は数えない。矩形で 3x3 に切られた塊は数えない。
8. `dominantShare`(合成データ): 一様な矩形は 100%。3x3 の点は背景が 95% 以上(描かれていない扱い)。12x40 の塊は背景 80%(描かれている)。
9. `warnings`: 道具が出した警告の全部を記録する(道具が閉じた物も含む)。前回の run では空だった。
10. 流れ A〜D(run 45592009894): 4 行とも合格。`elapsedMs` の中央値 11,698 ms。`preBootBytes` 312,304,864 B。
11. 押す前に窓の表(`windowNew`)を取る。前回は、Accept の後の画面で `windowNew` を取っていた不具合があり、直した(前回の差分に入っている)。
12. `visible` は、表に vis の項目が無いため常に false だった。前回直した(表に在れば visible とした)。
13. JSON に出る項目: `notes`・`noteRects`(x・y・幅・高さ)・`noteColour`・`noteWindow`(Piano roll の窓の矩形)。
14. 画面の題名: 窓は「Piano roll - riff machine」。Accept の後は「Piano roll - Riff Machine - Squelch」(前回の観測)。
15. Accept の後に窓が閉じる。閉じ方が Accept か Cancel かは、画面の差(ノート)で確かめる。Cancel の時は `notes` が 0 になる。
16. 直す前(HEAD の seed 表)の回は窓が無いので、Accept の操作は届かなかった。
17. 前回の観測の画面(PNG)は、run の成果物に在る。前回は目で読んだ(OCR は使わない)。
18. 道具の DPR: 前回の測定は DPR 1 と乱数の DPR(1.01〜3.00)の 2 点。
19. 数え方の注意(前回の判断): 連結成分の幅・高さが 4 px 未満の物は数えない。細い線(幅 1〜3 px)は、ノートとして数えない。
20. 数え方の注意(前回の判断): 窓の矩形の外の変化は数えない。Piano roll の窓の矩形は、`noteWindow` から取る。
21. run 45394680999 の判定は「差し戻し」。理由は、Sytrus の止まり・`missingPaths`・`notes` の数え方が無いこと(#1387 の記録の 13)。
22. 前回の 2 回の run で、`notes` は一度も数えていない。0 と書いた値は、数えた値ではない(run 45592009894 は、止まった後の画面)。
23. Accept の押下の見た目(ボタンの画素の変化)が、位置の目印として使えるかは、未確認。前回は PNG を目で読んだだけ。
24. 窓の子の数は 8(run 45592009894 の JSON の `children`)。窓の表の子の一覧は、Accept の位置を見つける時の手がかりにできる(未確認)。
25. 流れ A〜D の道具 `tools/smoke/welcome-flow.mjs` は、前回の run で 4 行を出した(main の版)。判定の式は #{{147}} と同じ。
26. `dominantShare` の判定(一色の割合が 95% 未満で描かれている)は、窓の `painted` に使う。前回の run の窓の値は 0.371(DPR 1)と 0.372(DPR 1.32)。
27. 前回の直し: `windowNew` は、Accept を押す前に取る(押した後の画面で取っていた不具合を直した)。
28. 前回の観測で、Accept を押した後に窓が閉じた(窓の表から消えた)。Accept と Cancel のどちらで閉じたかは、区別していない(未確認)。
29. 前回の PNG(Accept の前後の画面)は run の成果物にしか無い。この Issue の作業者は、Accept の前後の画像の要点(窓の矩形・色・画素の数字)を、`pm-work/findings/` の記録に残す。

## 読む物

- `tools/piano-roll-tools/measure.mjs`(`countNotes`・`dominantShare`・`noteWindow`・窓の表・Accept の段)
- `spec/piano-roll-notes.test.mjs`(合成データの試験。15 件)
- #{{147}} の本文の「数え方(PM の決定)」と「前回までの試行で分かったこと」の 14〜32
- `pm-work/findings/riff-machine-no-response.md`(#1387 の記録。前回の差分で main に入る)

## やること

1. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
2. Accept の位置を見つける: 窓(`TPRScoreCreatorForm`)の矩形の中で、候補の点を窓の幅と高さの比率で並べ、1 点ずつ押す。押す前後で、窓の矩形の画素のハッシュが変わる(押下の見た目)か、窓が窓の表から消える(閉じる)かを、候補ごとに記録する。定数の座標や比率を、製品にも道具にも書かない。
3. 押す前に窓の表(`windowNew`)を取る。押した後に、Piano roll の窓の矩形(`noteWindow`)で画面を取る。
4. `countNotes(before, after, rect)` で、ノートの連結成分を数える。ノートの色は、実測(後にだけ現れた色の最頻値)。定数にしない。
5. Cancel の確かめ: 閉じた後の `notes` が 0 なら、その候補は Accept ではない。外して、次の候補を試す。
6. 道具 `tools/piano-roll-tools/measure.mjs --tool riff-machine` の JSON に、Accept の段の結果を出す(`notes`・`noteRects`・`noteColour`)。DPR 1 と乱数の DPR の 2 点で測る。
7. 流れ A〜D を取る。`npm test --prefix app` を通す。
8. 受け入れを通す。通らなければ、作業ツリーを commit して終える。

## 受け入れ

```accept
mkdir -p lift/out-app/host/resources && cp -a lift/out-app/host-extra/resources/. lift/out-app/host/resources/ && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && D=$(node -e 'process.stdout.write((require("crypto").randomInt(101,301)/100).toFixed(2))') && node tools/piano-roll-tools/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --tool riff-machine --dprs 1,$D > "$RUNNER_TEMP/riff.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const ok=(r)=>r.error===null&&r.stopped===false&&r.engine==="engine-idle"&&Array.isArray(r.warnings)&&r.warnings.length===0&&Number.isInteger(r.notes)&&r.notes>=1&&Array.isArray(r.noteRects)&&r.noteRects.length===r.notes&&r.noteRects.every((b)=>b.w>=4&&b.h>=4);process.exit(L.length===2&&L[0].dpr===1&&L[1].dpr>=1.01&&L[1].dpr<=3&&L.every(ok)?0:1)' "$RUNNER_TEMP/riff.jsonl" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;const ms=L.map((r)=>r.elapsedMs).sort((a,b)=>a-b);const med=(ms[1]+ms[2])/2;const pb=Math.max(...L.map((r)=>r.preBootBytes));process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true&&med<=12500&&pb<=314368711?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app
```

判定の基準:
- 道具の 2 行(DPR 1 と乱数の DPR): `error` が null、`stopped` が false、`engine` が engine-idle、`warnings` が空の配列、`notes` が 1 以上の整数、`noteRects` の数が `notes` と同じで、各 `w`・`h` が 4 以上。
- Accept の位置は、見つけた物(固定の座標・比率を、製品にも道具にも書いていない)。報告に、見つけた方法と、試した候補の数を書く。
- 流れ A〜D(4 行)。判定は #{{147}} と同じ(各行の `error`・`state`・`standIn`・`trapped`・`hostErrors`・`opened`、流れ B と D の `welcomeClosed`、4 流れの `elapsedMs` の中央値が 12,500 以下、`preBootBytes` の最大が 314,368,711 以下)。
- 単体テスト(`npm test --prefix app`)が通る。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 押す前後の窓の表、Accept の位置を見つけた方法と候補の数、閉じた後の `notes` と `noteRects`(Cancel の確かめを含む)、ノートの色(`noteColour`)、道具の JSON の 2 行、流れ A〜D の 4 行。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。
- Python を使わない。OCR を使わない。FL の画面の文字は、ゲストのメモリの走査か、エンジンの記録で読む。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
- ぼやけた・拡大した画面を出さない。特定の拡大率や DPI の値を、定数・表として埋め込まない(DPR は、道具の引数で渡す値と、乱数の値だけ)。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- 作業の開始時刻を `date -u` で記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- engine の作り直しは約 35〜40 分。ビルドはフォアグラウンドで、シェルツールの `timeout` 引数に 5400000 を渡して待つ(背景で起動して待たない)。
- blend2d の取得(GitHub)が 403 で落ちたら一時的。受け入れは `lift/out-app/b2d-src` を消してから作り直すので、そのまま再試行する。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: #{{187}}(窓の後の Sytrus の呼び出しの止まり)と #{{147}}(窓が出る段階 1。`countNotes` の道具の差分が main に入る)。
- #1386(コード進行ツール)は、同じ `countNotes` を使う。衝突したら両方を残し、報告に書く。
