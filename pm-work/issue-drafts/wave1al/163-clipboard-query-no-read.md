---
draft: "163"
title: "クリップボードの問い合わせが、読み取り・権限の要求・表示を起こさない。貼り付けの操作の時だけ読み、権限を求める。表示は必ず消える"
type: Task
milestone: "M3 トップバーのコンテキストメニュー(P2)"
parent: "138"
blocked_by: []
labels: ["clipboard", "verify"]
size: M
priority: Urgent
project: signature-studio/3
---

## 目的

依頼者の指示(原文): 「プラグインのノブを右クリックしたら『貼り付け』コンテキストメニューが出現した上に、権限がない旨を示すトーストが出現し、あろうことかそのトーストは非表示になりません。クリップボードの権限を求める処理がないとはどういうことですか？ ノブを右クリックしたときクリップボードから貼り付ける必要があるわけないです。トーストが消えないというのは何事ですか？」

この Issue の目標は次の 4 つ。
1. FL の画面で、ノブ(Channel Rack の Volume・Pan。内蔵プラグインのノブを含む)を右クリックしても、ページのクリップボードの API を呼ばない。読み取り・権限の要求・表示の、どれも起きない。
2. クリップボードの中身を読むのは、利用者が貼り付けを求めた時だけ(FL の貼り付けの項目の選択、または Ctrl+V)。その時、権限が `prompt` なら、ブラウザの許可の確認が出る(権限を求める処理)。`denied` なら、理由の表示を 1 回出す。
3. 表示(トースト)は、必ず消える。
4. 貼り付けの必要が無い操作(ノブの右クリックなど)では、表示を出さない。

## 背景

- 事実 1(`app/src/engine/clipboard.ts` の `PageClipboard.read()`): エンジンの貼り付けの問い合わせ(`IsClipboardFormatAvailable`・`GetClipboardData` など)の度に呼ばれ、`navigator.clipboard.readText()` を呼ぶ。権限が `denied` なら、ページの状態行(`#clipboard-status`・`role="status"`)に理由を出し、次に成功するまで消さない(`say()` が、メッセージを保つ)。
- 事実 2(PM の観察。作業者が、窓の呼び出しの記録で確かめる): FL は、ノブの右クリックで文脈メニューを作る時に、「貼り付け」を有効にするかを決めるため、クリップボードの形式の有無を問い合わせる(`IsClipboardFormatAvailable`)。そのため、右クリックのたびに `readText()` が走り、許可の確認や拒否の表示が出る。依頼者の観察と合う。
- 事実 3(設計の誤り): 問い合わせ(形式の有無・個数・列挙)が、中身の読み取りと権限の要求を兼ねている。貼り付けを求めていない操作で、読みと表示が起きる。
- 事実 4(消えない表示): 表示に、消える条件が無い(次に成功するまで残る)。
- 参照: #1389 の方式(`tools/clipboard/measure.mjs`。Copy・Paste・拒否の扱い)、`pm-work/findings/clipboard-1389.md`。

## 読む物

- `app/src/engine/clipboard.ts`(`PageClipboard`・`permissionState`・`say`・`#clipboard-status` の要素と、文言)
- `app/src/main.ts`(`clipboardRead`・`clipboardWrite` の handler)
- `app/src/engine/client.ts`・`app/src/engine/worker.ts`・`app/src/engine/protocol.ts`(ゲストの問い合わせの経路)
- `lift/runtime/fle_clip.c`・`lift/runtime/fle_clip.h`(`IsClipboardFormatAvailable`・`CountClipboardFormats`・`EnumClipboardFormats`・`GetClipboardData` の、今の応答と、ページへの問い合わせの経路)
- `tools/clipboard/measure.mjs`(#1389 の道具。拡張する)
- `pm-work/findings/clipboard-1389.md`・`pm-work/findings/webview2-calls-1355.md`(窓の呼び出しの記録の方式)
- `tools/smoke/welcome-flow.mjs`(起動と Welcome の閉じ方)
- `tools/feat-c6-popup-menus/measure.mjs`(窓の表と、ノブ・項目の位置の取り方。固定の座標は書かない)

## やること

0. 呼び出しの記録(直す前に測る)。
   - 既定のプロジェクトを開き、Channel Rack の Volume のノブを右クリックする。その間に、ゲストが呼ぶクリップボードの API(`OpenClipboard`・`IsClipboardFormatAvailable`・`CountClipboardFormats`・`EnumClipboardFormats`・`GetClipboardData`・`CloseClipboard`)の並びと、ページの `navigator.clipboard.readText`・`read`・`navigator.permissions.query` の呼び出しを、既存の記録の方式で取る。
   - 貼り付けの時(FL の名前の編集欄で Ctrl+V。貼り付けの項目の選択)の並びも取る。
   - `GetClipboardData` が右クリックの間に呼ばれるかを必ず確かめ、結果を報告に書く。呼ばれるなら、読みを `GetClipboardData` に結ぶ方式は使えない。その時は別の切り分けを取り、理由を報告に書く。

1. 直す(製品の既定の動作として入れる。オプトインを作らない)。
   - ① 問い合わせは、中身を読まない。`IsClipboardFormatAvailable`・`CountClipboardFormats`・`EnumClipboardFormats` は、クリップボードを読まず、権限を要求せず、表示を出さない。答えは、ページが既に知っている状態から出す。知っている状態の源は次の通り。
     - エンジン自身が最後に書いた文字列(`SetClipboardData` の後に、`navigator.clipboard.writeText` が成功した物)。
     - 利用者の貼り付けの操作の時に得た文字列(`paste` イベントの `clipboardData`。権限が要らない。または `readText` の結果)。
     - 権限が既に `granted` で、キャッシュがある時の、直近の読みの結果。キャッシュは、貼り付けの操作の時にだけ作る。右クリックでは作らない。
     - 知っている状態が無い時(`prompt`・`denied` のいずれでも)は、「テキストがある」と答えず(空)、表示も出さない。
   - ② 中身を読むのは、利用者が貼り付けを求めた時だけ。`GetClipboardData`(貼り付けの項目の選択・Ctrl+V の後)の時に、権限が `prompt` なら `navigator.clipboard.readText()` を呼び、ブラウザの許可の確認を出す(権限を求める処理)。権限が `denied` なら、理由の表示を 1 回出し、エンジンには空を返す。エンジンは止めない(`fle_stop` にしない)。
   - ③ 表示は必ず消える。5 秒で自動に消える・閉じるボタンがある・同じ理由の表示は、出ている間に重ねない(出ている間は、消える時刻を延ばすだけ)・貼り付けを求めていない操作(ノブの右クリックなど)では出さない。
   - ④ ページが作る独自の「貼り付け」のメニューは作らない。メニューは FL 自身の物だけ。
   - 注意: 右クリックの答えを「空」にすると、FL の「貼り付け」の項目は無効になる。これは、知っている状態が無い時の仕様の結果。使い勝手(どの状態で貼り付けの項目が有効になるか)を、報告に正直に書く。Windows の実際の値とは違う場合は、その差も書く(依頼者の判断が要る)。
   - #1389 の Copy(`writeText` の経路)と、Paste の結果(`pasted`)を壊さない。

2. 測定の道具 `tools/clipboard/measure.mjs` を拡張する(既存の項目 `error`・`copied`・`match`・`pasted`・`pasteDelayMs`・`deniedHandled` は残す)。CDP の `Browser.grantPermissions`・`Browser.setPermission`・`Browser.resetPermissions` で、権限を `prompt`・`denied`・`granted` の 3 通りにする。各通りを、DPR 1 と、乱数の DPR(1.01〜3.00 の 1 点)の 2 通りで開く。ページに計数の仕掛け(`navigator.clipboard.readText`・`read` の呼び出しを数える)を入れる。stdout に JSON を 1 行。`runs` は 6 件(通り 3 × DPR 2)。各件は次の項目。
   - `permission`: 開始時の権限(`prompt`・`denied`・`granted` のいずれか)
   - `dpr`: DPR
   - `knobMenuShown`: ノブの右クリックで、FL の文脈メニューが出た
   - `readCallsOnKnob`: ノブの右クリックの間に、`readText`・`read` が呼ばれた数(0 が必要)
   - `permissionAfterKnob`: 右クリックの後の `navigator.permissions.query` の状態(`permission` と同じが必要)
   - `visibleAlertsOnKnob`: 右クリックの後、可視の `role="status"`・`role="alert"`・`aria-live` の要素の数(0 が必要)
   - `pasteAnswered`: 貼り付けの後、エンジンが答え(文字列か空)を受けて戻った(true が必要)
   - `pasteMatch`: `granted` の時だけ、貼り付けた文字列と一致した(`granted` の件で true が必要。他の件は null)
   - `toastOnPaste`: 貼り付けで理由の表示が出た(`denied` の件で true が必要)
   - `toastHiddenAfter5s`: 表示が出た後、5 秒以内に非表示になった(表示が無い時は true。true が必要)
   - `duplicateToasts`: 表示が出ている間に、同じ理由の表示が重なった数(0 か 1 が必要)
   - `engineState`: 最後のエンジンの状態(`engine-idle` が必要)
   - 1 件の中の手順: ノブの右クリック → 5 秒待ち(表示と権限の状態を見る)→ 貼り付け(Ctrl+V)→ 5 秒待つ。
   - 使い方: `node tools/clipboard/measure.mjs <bundle の絶対パス> > clip.json`。終了コードは、JSON が書けたら 0。

3. 流れ A〜D の測定: `tools/smoke/welcome-flow.mjs` を同じ道具で走らせ、4 行(A・B・C・D)が在ること、各行の判定(`error`・`state`・`standIn`・`trapped`・`hostErrors`・`opened`)が通ること、流れ B と D で `welcomeClosed` が真であること。

4. 報告に書く: 呼び出しの記録(0 の並び)、採った方式と理由、貼り付けの項目が有効になる条件、`tools/clipboard/measure.mjs` の JSON の `runs` の 6 件、流れ A〜D の結果、`elapsedMs` と `preBootBytes` の値。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/clipboard/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/clip.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));const runs=Array.isArray(r.runs)?r.runs:[];const ok=(x)=>x.engineState==="engine-idle"&&x.readCallsOnKnob===0&&x.permissionAfterKnob===x.permission&&x.visibleAlertsOnKnob===0&&x.pasteAnswered===true&&x.toastHiddenAfter5s===true&&x.duplicateToasts<=1;process.exit(r.error===null&&r.copied===true&&r.match===true&&runs.length===6&&runs.every(ok)&&runs.filter((x)=>x.permission==="granted").every((x)=>x.pasteMatch===true)&&runs.filter((x)=>x.permission==="denied").every((x)=>x.toastOnPaste===true)?0:1)' "$RUNNER_TEMP/clip.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;const pb=Math.max(...L.map((r)=>r.preBootBytes));const el=Math.max(...L.map((r)=>r.elapsedMs));process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true&&el<=12500&&pb<=314368711?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app
```

判定の基準(受け入れのコマンドの各段の結果):
- `tools/clipboard/measure.mjs` の JSON: `error` が null・`copied` と `match` が真・`runs` が 6 件。各件で、`engineState` が engine-idle・`readCallsOnKnob` が 0・`permissionAfterKnob` が `permission` と同じ・`visibleAlertsOnKnob` が 0・`pasteAnswered` が真・`toastHiddenAfter5s` が真・`duplicateToasts` が 1 以下。`granted` の件は `pasteMatch` が真。`denied` の件は `toastOnPaste` が真。
- 流れ A〜D の 4 行が在り、各行が `error` が null・`state` が engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が真。流れ B と D は `welcomeClosed` が真。4 流れの `elapsedMs` の最大が 12,500 以下。`preBootBytes` の最大が 314,368,711 以下。
- 単体テスト(`npm test --prefix app`)が通る。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 右クリックの呼び出しの記録、採った方式、貼り付けの項目が有効になる条件、`runs` の 6 件の値、流れ A〜D の JSON の行。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。テストのための分岐を製品に入れない。
- Python を使わない(調査と検証にだけ使う)。OCR を使わない(FL の画面の文字は、ゲストのメモリの走査か、API の呼び出しの記録で読む)。
- ベンダー名と Cloudflare の識別子を、ファイルに書かない。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。待つ時は、条件(要素・イベント)で待つ。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- 開始の時刻を、最初に `date -u` で記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える。
- engine の作り直しは約 35 分。1 回の試行で、ビルドは数回まで。実験は、記録と trace をまとめた 1 回のビルドで行う。
- ビルドは、フォアグラウンドで、シェルツールの `timeout` 引数に 5400000 を渡して待つ。背景で起動して待たない。
- blend2d の取得(GitHub)が 403 で落ちたら一時的。受け入れのコマンドは `lift/out-app/b2d-src` を消してから作り直すので、そのまま再試行する。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: なし。
- #1389(親)の方式を、直す。`app/src/engine/protocol.ts`・`app/src/engine/worker.ts` は、他の Issue(#1390・#1391・#1392 など)もメッセージ型を足す。衝突したら、main の版を取り、この Issue の変更を取り直し、両方を残す(削除しない)。
- #{{164}}(E2E の基盤)が、この Issue の結果を、本番と同じ配信の上で検査する。この Issue は、それを待たずに入る。
