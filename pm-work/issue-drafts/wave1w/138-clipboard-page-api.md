---
draft: "138"
title: "FL のクリップボード(コピー・貼り付け)を、ページの Clipboard API(navigator.clipboard)と navigator.permissions に結ぶ"
type: Task
milestone: "M3 トップバーのコンテキストメニュー(P2)"
parent: "009"
blocked_by: []
labels: ["clipboard", "verify"]
size: M
priority: High
project: signature-studio/3
---

## 目的

依頼者の指示(原文): 「クリップボードが反映されていません。例えば、Problems Loading the project ウィンドウで項目をクリックしたときに出現する 'Copy' ツールチップを押下しても、実際のクリップボードには反映されません。`navigator.permissions` と Clipboard API を使用してください。」

この Issue の目標は次の 3 つ。
1. FL の Copy(ゲストの `SetClipboardData` で入った文字列)が、ページの `navigator.clipboard.writeText` で、利用者の OS のクリップボードに入る。文字列は一致する。
2. 利用者の OS のクリップボードの文字列を、FL の貼り付け(ゲストの `GetClipboardData`)が読める。
3. `navigator.permissions` で許可の状態を見る。許可されていない時は、FL の画面の中に偽の表示を出さず、ページの表示で理由を示す。engine は止まらない。

## 背景

- 事実 1(runtime。`lift/runtime/fle_clip.h` の 3〜4 行。決定 0171): 仮想のクリップボード。「One process, one clipboard, no host clipboard」「Nothing is read from or written to the host's clipboard.」。今の実装は、ホストのクリップボードに一切触れない。
- 事実 2(`lift/runtime/fle_clip.c`): `fle_imp_SetClipboardData` は、形式と handle(ゲストのメモリの HGLOBAL)だけを記録する。文字列の中身は、ページへ渡していない。`fle_imp_GetClipboardData` は、記録した handle を返すだけ。handle が NULL の遅延の描画は `fle_stop`(169 行付近)。合成の形式(CF_TEXT と CF_UNICODETEXT の相互変換など)は `fle_stop`(177 行付近)。
- 事実 3(HGLOBAL の中身は読める): `lift/runtime/fle_glob.c` の注釈: GMEM_MOVEABLE の handle は GlobalLock で block に変換され、block は host の malloc の領域(ゲストのメモリの外)にある。SetClipboardData の handle の中身は、runtime から読める(GlobalLock の経路を使う)。
- 事実 4(ページ): `navigator.clipboard` の呼び出しは、`app/src/` に無い。`app/src/files/explorer.ts` の Copy・Paste は、ページの中の変数(`clipboard`)で、OS のクリップボードではない。
- 事実 5(入力の経路): ページの入力は `app/src/engine/protocol.ts` の `cmd: 'input'`(RawInput)で engine に届く。Ctrl+V も、この経路で届く。engine からページへの通知は `app/src/engine/worker.ts` と `app/src/engine/client.ts`、受け口は `app/src/main.ts` の `EngineClient` の各 handler。
- 事実 6(窓): 「Problems loading the project」の窓(`TMissingPluginsForm`)は、既存の記録に在る(`pm-work/findings/p3-blocks-1335.md`・`pm-work/findings/blocks-samples-1366.md`)。'Copy' を押した時に、ゲストが呼ぶ API の並びは、まだ記録されていない(この Issue で記録する)。
- Web の仕様の要点(作業者が実測で確かめる): `navigator.clipboard.readText()` は `clipboard-read` の許可が要る。`writeText()` は、文書がフォーカスされていることが要る。`navigator.clipboard` は安全な文脈でのみ使える。`navigator.permissions.query({ name: 'clipboard-read' })` は、`granted`・`prompt`・`denied` のいずれかを返す。

## 読む物

- `lift/runtime/fle_clip.h`・`lift/runtime/fle_clip.c`(OpenClipboard・CloseClipboard・EmptyClipboard・IsClipboardFormatAvailable・CountClipboardFormats・EnumClipboardFormats・GetClipboardData・SetClipboardData と、注釈の決定 0171)
- `lift/runtime/fle_user_nowin.c`(RegisterClipboardFormatW・GetClipboardFormatNameW)・`lift/runtime/fle_glob.c`(GlobalLock・GlobalUnlock・GlobalSize)
- `app/src/engine/protocol.ts`・`app/src/engine/worker.ts`・`app/src/engine/client.ts`・`app/src/main.ts`(`EngineClient` の handler)
- `tools/smoke/welcome-flow.mjs`(起動と Welcome の閉じ方。流れ A〜D の受け入れを通す)
- `tools/p3-audio-probe/measure.mjs`(`openProject`: 既定のプロジェクトを開き、「Problems loading the project」の窓を Ok で閉じる流れ)
- `tools/feat-c6-popup-menus/measure.mjs`(窓の表と、項目の選び方)・`tools/browser-panel/measure.mjs`(CDP でブラウザを起動する型)
- `pm-work/findings/webview2-calls-1355.md`(ゲストの API の呼び出しの記録の方式)

## やること

0. 呼び出しの記録(測定を先に行う)。
   - 既定のプロジェクトを開き、「Problems loading the project」の窓を出す。項目をクリックし、'Copy' を押す。その時にゲストが呼ぶ `OpenClipboard`・`EmptyClipboard`・`SetClipboardData`(形式の番号・handle・文字列の中身)・`CloseClipboard` の並びを、既存の記録の仕組み(`webview2-calls-1355.md` の方式)で取る。
   - 貼り付け(Ctrl+V)の時の `OpenClipboard`・`IsClipboardFormatAvailable`・`GetClipboardData`(形式)の並びを、同じ方式で取る。
   - 結果を報告に書く。FL が使う形式(CF_UNICODETEXT・CF_TEXT など)は、実測で決める。
1. 設計(下の案は PM の案で、実測で確かめて選ぶ):
   - Copy: ゲストの `SetClipboardData` が形式を受けた時、handle の中身(GlobalLock 相当)を読み、engine の Worker からページのメインスレッドへ文字列を渡す。ページは `navigator.clipboard.writeText` を呼ぶ。利用者の操作の中で書く必要があれば、ページ側でポインタ・キーのイベントを受けた時に、その処理の中で書く。非同期の遅れで操作の権限が切れないことを実測で確かめる。
   - Paste: ゲストの `GetClipboardData` は同期で返す。ページの `readText()` は非同期。貼り付けの操作(Ctrl+V・メニューの貼り付け)の前に、ページが読んで engine に渡しておく形を取る。どの時点で読むかは実測で決める(入力を保留する、操作の前の段で読む、など)。入力から貼り付けまでの遅れを測り、報告に書く。入力の遅れを増やさない方式を選ぶ。
   - 権限: `navigator.permissions.query` で `clipboard-read`・`clipboard-write` の状態を見る。`prompt` の時は、利用者の操作の中で要求する。`denied` の時は、FL の画面の中に偽の表示を出さない。ページの小さな表示(`role="status"` の要素)で理由を示す。Clipboard API が無い時も同じ扱い。engine は止めない(`fle_stop` にしない)。
   - ホストのクリップボードに触れない、という今の決定 0171 の注釈は、新しい方式に合わせて直す。
2. 製品の既定の動作として入れる。オプトイン(環境変数・URL パラメータ・ビルド変数)は作らない。
3. 測定の道具 `tools/clipboard/measure.mjs`(新規)。CDP の `Browser.grantPermissions`(`clipboardReadWrite`・`clipboardSanitizedWrite`)で許可して開く。許可を `denied` にした版も開き、`deniedHandled` を測る。stdout に JSON を 1 行。項目:
   - `error`: null、または失敗の理由。
   - `copied`: 'Copy' の後に、ページの `navigator.clipboard.readText()` が値を返した。
   - `match`: その値が、エンジンが `SetClipboardData` に渡した文字列と完全に一致する。
   - `text`: 文字列(200 文字以内に切る)。`textLength`: 文字列の長さ。
   - `pasted`: `navigator.clipboard.writeText('<乱数の文字列>')` の後、FL のテキストの欄で Ctrl+V し、ゲストの `GetClipboardData` が、その文字列を返した(エンジンのログか記録で確かめる)。
   - `pasteDelayMs`: Ctrl+V の入力から、ゲストの `GetClipboardData` までの時間。
   - `deniedHandled`: 許可が `denied` の時、engine が止まらず(state が engine-idle)、ページに理由の表示が出た。
4. 報告に書く: 呼び出しの記録(0 の並び)、採った方式と理由、`pasteDelayMs`、`tools/clipboard/measure.mjs` の JSON の行、流れ A〜D の結果。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/clipboard/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/clip.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error===null&&r.copied===true&&r.match===true&&r.pasted===true&&r.deniedHandled===true?0:1)' "$RUNNER_TEMP/clip.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;const pb=Math.max(...L.map((r)=>r.preBootBytes));const el=Math.max(...L.map((r)=>r.elapsedMs));process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true&&el<=12500&&pb<=314385095?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app
```

判定の基準:
- `tools/clipboard/measure.mjs` の JSON が、`error` が null、`copied`・`match`・`pasted`・`deniedHandled` が全て true。
- `tools/smoke/welcome-flow.mjs` の 4 行(流れ A・B・C・D)が在り、各行が `error` が null・`state` が engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。流れ B と D は `welcomeClosed` が true。
- 4 流れの `elapsedMs` の最大が 12,500 以下。`preBootBytes` の最大が 314,385,095 以下(#1376 の基準 314,368,711 B に、余裕 16,384 B を足した値)。
- 単体テスト(`npm test --prefix app`)が通る。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 「直った」と書かない。上の判定の基準の結果で書く。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。
- Python は製品に使わない(調査と検証にだけ使う)。OCR は使わない(FL の画面の文字は、ゲストのメモリの走査か、API の呼び出しの記録で読む)。
- ベンダー名と Cloudflare の識別子を、ファイルに書かない。
- sleep・timeout コマンド・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- 開始の時刻を、最初に `date -u` で記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える。
- engine の作り直しは約 35 分。1 回の試行で、ビルドは数回まで。実験は、記録と trace をまとめた 1 回のビルドで行う。
- ビルドは、フォアグラウンドで、シェルツールの `timeout` 引数に 5400000 を渡して待つ。背景で起動して待たない。
- blend2d の取得(GitHub)が 403 で落ちたら一時的。受け入れのコマンドは `lift/out-app/b2d-src` を消してから作り直すので、そのまま再試行する。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: なし。
- `#{{139}}`(マウスカーソル。同じ engine から page への通知の経路 `protocol.ts`・`worker.ts` を使う)は、この Issue の後に行う。
- 既存の `app/src/files/explorer.ts` の Copy・Paste は、この Issue の範囲外。触らない。
