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
1. FL の画面で、ノブ(Channel Rack の Volume・Pan。内蔵プラグインのノブを含む)を右クリックしても、ページのクリップボードの API の読み取りも、権限の要求も、表示も、起きない。
2. FL の「貼り付け」は、権限が `prompt` の時、有効になる(押せる)。押した時に、ブラウザの許可の確認が出る(権限を求める処理)。
3. Ctrl+V の貼り付けは、ページの `paste` イベントの内容で行い、権限の確認を出さない。
4. 表示(トースト)は、貼り付けの操作(メニューの項目・Ctrl+V)の結果としてだけ 1 回出て、必ず消える。問い合わせでは出ない。

## 背景

- 事実 1(`app/src/engine/clipboard.ts` の `PageClipboard.read()`): エンジンの貼り付けの問い合わせ(`IsClipboardFormatAvailable`・`GetClipboardData` など)の度に呼ばれ、`navigator.clipboard.readText()` を呼ぶ。権限が `denied` なら、ページの状態行(`#clipboard-status`・`role="status"`)に理由を出し、次に成功するまで消さない(`say()` が、メッセージを保つ)。
- 事実 2(PM の観察。作業者が、窓の呼び出しの記録で確かめる): FL は、ノブの右クリックで文脈メニューを作る時に、「貼り付け」を有効にするかを決めるため、クリップボードの形式の有無を問い合わせる(`IsClipboardFormatAvailable`)。そのため、右クリックのたびに `readText()` が走り、許可の確認や拒否の表示が出る。依頼者の観察と合う。
- 事実 3(設計の誤り): 問い合わせ(形式の有無・個数・列挙)が、中身の読み取りと権限の要求を兼ねている。貼り付けを求めていない操作で、読みと表示が起きる。
- 事実 4(消えない表示): 表示に、消える条件が無い(次に成功するまで残る)。
- 事実 5(「空」と答えた場合の帰結): 権限が `prompt` で知っている状態が無い時に「空」と答えると、FL の「貼り付け」が無効になり、利用者は押せない。そのため読み取りの要求(許可の確認)が永久に出ない。だから、この場合は「在る」と答える(下の ①)。
- 参照: #1389 の方式(`tools/clipboard/measure.mjs`。Copy・Paste・拒否の扱い)、`pm-work/findings/clipboard-1389.md`。

## 読む物

- `app/src/engine/clipboard.ts`(`PageClipboard`・`permissionState`・`say`・`#clipboard-status` の要素と、文言)
- `app/src/main.ts`(`clipboardRead`・`clipboardWrite` の handler。キーの入力の経路)
- `app/src/engine/client.ts`・`app/src/engine/worker.ts`・`app/src/engine/protocol.ts`(ゲストの問い合わせの経路と、ページから engine へ文字列を渡す経路)
- `lift/runtime/fle_clip.c`・`lift/runtime/fle_clip.h`(`IsClipboardFormatAvailable`・`CountClipboardFormats`・`EnumClipboardFormats`・`GetClipboardData` の、今の応答と、ページへの問い合わせの経路)
- `tools/clipboard/measure.mjs`(#1389 の道具。拡張する)
- `pm-work/findings/clipboard-1389.md`・`pm-work/findings/webview2-calls-1355.md`(窓の呼び出しの記録の方式)
- `tools/smoke/welcome-flow.mjs`(起動と Welcome の閉じ方)
- `tools/feat-c6-popup-menus/measure.mjs`(窓の表と、ノブ・項目の位置の取り方。固定の座標は書かない)

## やること

0. 呼び出しの記録(直す前に測る)。
   - 既定のプロジェクトを開き、Channel Rack の Volume のノブを右クリックする。その間に、ゲストが呼ぶクリップボードの API(`OpenClipboard`・`IsClipboardFormatAvailable`・`CountClipboardFormats`・`EnumClipboardFormats`・`GetClipboardData`・`CloseClipboard`)の並びと、ページの `navigator.clipboard.readText`・`read`・`navigator.permissions.query` の呼び出しを、既存の記録の方式で取る。
   - `GetClipboardData` が右クリックの間に呼ばれるかを必ず確かめ、結果を報告に書く。
   - Ctrl+V の時に、ページで `paste` イベントが出るかを確かめる(キーの処理が既定の動作を止めていないか)。出ないなら、その原因を報告に書く。

1. 直す(製品の既定の動作として入れる。オプトインを作らない)。
   - ① 問い合わせ(`IsClipboardFormatAvailable`・`CountClipboardFormats`・`EnumClipboardFormats`)は、表示を出さず、権限を要求しない。答えは次の順で決める。
     - (a) 既に知っている状態があれば、それに従う。知っている状態は、利用者の貼り付けの操作の時に得た文字列(`paste` イベントの内容、または `readText` の結果)と、Copy で書いた文字列。(a) は (b)〜(d) より先に見る。Ctrl+V の貼り付けが権限の `denied` でも動くようにするため。
     - (b) 権限が `denied`: 「無い」(読めない)。
     - (c) 権限が `granted`: `navigator.clipboard.read()` の形式の一覧(短時間のキャッシュ。権限の確認は出ない)。
     - (d) 権限が `prompt` で、知っている状態が無い: 「在る」(`CF_UNICODETEXT` と `CF_TEXT`)。中身は読まず、許可の確認も出さない。
     - `CountClipboardFormats`・`EnumClipboardFormats` は、同じ答えに合わせる。
   - ② 貼り付けの操作の経路は次の 2 つ。
     - Ctrl+V(キーボード): ページは、`paste` イベントの `clipboardData` の `text/plain` を、先にエンジンへ渡す(既知の状態として。権限の確認は要らない)。その後に、Ctrl+V のキーをゲストに送る。ゲストの `GetClipboardData` は、渡した文字列で答える。`readText()` は呼ばない。`paste` イベントが出ない時の扱い(キーを送る時点を、どう決めるか)は、実測で決める。
     - メニューの項目(FL の「貼り付け」を選ぶ): ゲストの `GetClipboardData` で、`navigator.clipboard.readText()` を呼ぶ。権限が `prompt` なら、ブラウザの許可の確認が出る(クリックの操作の中で呼ぶ)。`granted` なら、そのまま読む。`denied` なら、読まずに、理由の表示を 1 回出し、エンジンには空を返す。エンジンは止めない(`fle_stop` にしない)。
   - ③ 表示(トースト)は、貼り付けの操作の結果としてだけ 1 回出る(メニューの項目・Ctrl+V)。問い合わせでは出さない。5 秒で自動に消える。閉じるボタンがある。同じ理由の表示は、出ている間に重ねない(出ている間は、消える時刻を延ばすだけ)。
   - ④ ページが作る独自の「貼り付け」のメニューは作らない。メニューは FL 自身の物だけ。
   - #1389 の Copy(`writeText` の経路)を壊さない。
   - 使い勝手の帰結(報告に正直に書く): 権限が `prompt` では、FL の「貼り付け」は有効で、押すと許可の確認が出る。`denied` では、メニューの「貼り付け」は無効になる。Ctrl+V は、`denied` でも、`paste` イベントの内容で貼り付く。Windows の実際の値とは、知っている状態が無い時に違う。その差を報告に書く。

2. 測定の道具 `tools/clipboard/measure.mjs` を拡張する(既存の項目 `error`・`copied`・`match` は残す)。CDP の `Browser.grantPermissions`・`Browser.setPermission`・`Browser.resetPermissions` で、権限を `prompt`・`denied`・`granted` の 3 通りにする。各通りを、DPR 1 と、乱数の DPR(1.01〜3.00 の 1 点)の 2 通りで開く。ページに計数の仕掛け(`navigator.clipboard.readText`・`read` の呼び出しを数える)を入れる。stdout に JSON を 1 行。`runs` は 6 件(通り 3 × DPR 2)。各件は次の項目。
   - `permission`: 開始時の権限(`prompt`・`denied`・`granted` のいずれか)
   - `dpr`: DPR
   - `knobMenuShown`: ノブの右クリックで、FL の文脈メニューが出た
   - `knobReadCalls`: ノブの右クリックの間に、`readText`・`read` が呼ばれた数
   - `permissionAfterKnob`: 右クリックの後の `navigator.permissions.query` の状態
   - `knobVisibleAlerts`: 右クリックの後、可視の `role="status"`・`role="alert"`・`aria-live` の要素の数
   - `pasteEnabledOnKnob`: 右クリックのメニューの「貼り付け」が有効(エンジンの `IsClipboardFormatAvailable` の答えが真)。`granted` の件では、テストが事前に `writeText` で文字列を入れておく
   - `ctrlVMatch`: Ctrl+V で、テストが事前に `writeText` で入れた乱数の文字列が、エンジンの `GetClipboardData` に届いて一致した
   - `ctrlVReadCalls`: Ctrl+V の間に、`readText`・`read` が呼ばれた数
   - `permissionAfterCtrlV`: Ctrl+V の後の権限の状態
   - `menuPasteReadCalls`: メニューの「貼り付け」の間に、`readText` が呼ばれた数
   - `menuPasteMatch`: メニューの「貼り付け」で一致した。`granted` の件は最初の貼り付け。`prompt` の件は、`Browser.setPermission` で `granted` にした後の 2 回目の貼り付け。`denied` の件は null
   - `toastOnMenuPaste`: メニューの「貼り付け」の結果で、理由の表示が出た
   - `toastCount`: 操作の間に、表示が同時に出ていた最大数
   - `toastHiddenAfter5s`: 表示が出た後、5 秒以内に非表示になった(表示が無ければ true)
   - `engineState`: 最後のエンジンの状態
   - 1 件の中の手順: ノブの右クリック → 5 秒待つ(表示と権限の状態を見る)→ Ctrl+V → 5 秒待つ → メニューの「貼り付け」→ 5 秒待つ。`prompt` の件は、メニューの貼り付けの後に `Browser.setPermission` で `granted` にし、もう一度メニューの「貼り付け」をする。
   - 使い方: `node tools/clipboard/measure.mjs <bundle の絶対パス> > clip.json`。終了コードは、JSON が書けたら 0。

3. 流れ A〜D の測定: `tools/smoke/welcome-flow.mjs` を同じ道具で走らせ、4 行(A・B・C・D)が在ること、各行の判定(`error`・`state`・`standIn`・`trapped`・`hostErrors`・`opened`)が通ること、流れ B と D で `welcomeClosed` が真であること。

4. 報告に書く: 呼び出しの記録(0 の並び)、`paste` イベントの有無、採った方式と理由、使い勝手の帰結、`runs` の 6 件の値、流れ A〜D の結果、`elapsedMs` と `preBootBytes` の値。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/clipboard/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/clip.json" && node -e 'const fs=require("fs");const r=JSON.parse(fs.readFileSync(process.argv[1],"utf8"));const runs=Array.isArray(r.runs)?r.runs:[];const common=(x)=>x.knobMenuShown===true&&x.engineState==="engine-idle"&&x.permissionAfterKnob===x.permission&&x.knobVisibleAlerts===0&&x.ctrlVMatch===true&&x.ctrlVReadCalls===0&&x.permissionAfterCtrlV===x.permission&&x.toastHiddenAfter5s===true&&x.toastCount<=1;const by=(p)=>runs.filter((x)=>x.permission===p);const pr=by("prompt"),de=by("denied"),gr=by("granted");process.exit(r.error===null&&r.copied===true&&r.match===true&&runs.length===6&&runs.every(common)&&pr.length===2&&de.length===2&&gr.length===2&&pr.every((x)=>x.knobReadCalls===0&&x.pasteEnabledOnKnob===true&&x.menuPasteReadCalls>=1&&x.menuPasteMatch===true)&&de.every((x)=>x.knobReadCalls===0&&x.menuPasteReadCalls===0&&x.toastOnMenuPaste===true)&&gr.every((x)=>x.pasteEnabledOnKnob===true&&x.menuPasteMatch===true&&x.toastOnMenuPaste===false)?0:1)' "$RUNNER_TEMP/clip.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;const pb=Math.max(...L.map((r)=>r.preBootBytes));const el=Math.max(...L.map((r)=>r.elapsedMs));process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true&&el<=12500&&pb<=314368711?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app
```

判定の基準(受け入れのコマンドの各段の結果):
- `tools/clipboard/measure.mjs` の JSON: `error` が null・`copied` と `match` が真・`runs` が 6 件。全ての件で、`knobMenuShown` が真・`engineState` が engine-idle・`permissionAfterKnob` が `permission` と同じ・`knobVisibleAlerts` が 0・`ctrlVMatch` が真・`ctrlVReadCalls` が 0・`permissionAfterCtrlV` が `permission` と同じ・`toastHiddenAfter5s` が真・`toastCount` が 1 以下。
- `prompt` の件: `knobReadCalls` が 0・`pasteEnabledOnKnob` が真・`menuPasteReadCalls` が 1 以上・`menuPasteMatch` が真(`Browser.setPermission` の後の貼り付け)。
- `denied` の件: `knobReadCalls` が 0・`menuPasteReadCalls` が 0・`toastOnMenuPaste` が真。
- `granted` の件: `pasteEnabledOnKnob` が真・`menuPasteMatch` が真・`toastOnMenuPaste` が偽。
- 流れ A〜D の 4 行が在り、各行が `error` が null・`state` が engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が真。流れ B と D は `welcomeClosed` が真。4 流れの `elapsedMs` の最大が 12,500 以下。`preBootBytes` の最大が 314,368,711 以下。
- 単体テスト(`npm test --prefix app`)が通る。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 右クリックの呼び出しの記録、`paste` イベントの有無、採った方式と理由、使い勝手の帰結、`runs` の 6 件の値、流れ A〜D の JSON の行。

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
