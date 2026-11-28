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
4. 表示(トースト)は、貼り付けの操作(メニューの項目・Ctrl+V)と、Copy の結果としてだけ出て、必ず消える。問い合わせでは出ない。

## 背景

- 事実 1(`app/src/engine/clipboard.ts` の `PageClipboard.read()`): エンジンの貼り付けの問い合わせ(`IsClipboardFormatAvailable`・`GetClipboardData` など)の度に呼ばれ、`navigator.clipboard.readText()` を呼ぶ。権限が `denied` なら、ページの状態行(`#clipboard-status`・`role="status"`)に理由を出し、次に成功するまで消さない(`say()` が、メッセージを保つ)。
- 事実 2(PM の観察。作業者が、窓の呼び出しの記録で確かめる): FL は、ノブの右クリックで文脈メニューを作る時に、「貼り付け」を有効にするかを決めるため、クリップボードの形式の有無を問い合わせる(`IsClipboardFormatAvailable`)。そのため、右クリックのたびに `readText()` が走り、許可の確認や拒否の表示が出る。依頼者の観察と合う。
- 事実 3(設計の誤り): 問い合わせ(形式の有無・個数・列挙)が、中身の読み取りと権限の要求を兼ねている。貼り付けを求めていない操作で、読みと表示が起きる。
- 事実 4(消えない表示): 表示に、消える条件が無い(次に成功するまで残る)。
- 事実 5(「空」と答えた場合の帰結): 権限が `prompt` で知っている状態が無い時に「空」と答えると、FL の「貼り付け」が無効になり、利用者は押せない。そのため読み取りの要求(許可の確認)が永久に出ない。だから、この場合は「在る」と答える(下の ①)。
- 事実 6(FL の「貼り付け」の有効・無効): FL は、右クリックの時に `GetClipboardData` の文字列を数値として解釈できるかを確かめ、数値でなければ「貼り付け」を無効に描く。測定の文字列は数字にする(下の「前回の試行で分かったこと」)。
- 参照: #1389 の方式(`tools/clipboard/measure.mjs`。Copy・Paste・拒否の扱い)、`pm-work/findings/clipboard-1389.md`。

## 読む物

- `app/src/engine/clipboard.ts`(`PageClipboard`・`permissionState`・`say`・`#clipboard-status` の要素と、文言)
- `app/src/input/keymap.ts`(`keyPolicy`。Ctrl+V の既定の動作を、止めないこと)
- `app/src/main.ts`(`clipboardRead`・`clipboardWrite` の handler。キーの入力の経路)
- `app/src/engine/client.ts`・`app/src/engine/worker.ts`・`app/src/engine/protocol.ts`(ゲストの問い合わせの経路と、ページから engine へ文字列を渡す経路)
- `lift/runtime/fle_clip.c`・`lift/runtime/fle_clip.h`・`lift/harness/web/clip/fle_clip_lib.js`(`IsClipboardFormatAvailable`・`CountClipboardFormats`・`EnumClipboardFormats`・`GetClipboardData` の、今の応答と、ページへの問い合わせの経路)
- `tools/clipboard/measure.mjs`(#1389 の道具。拡張する)
- `tools/cursor/measure.mjs`・`tools/e2e/lib/guest.mjs`(ゲストの画素から CSS px への写像と、窓の表の矩形の取り方)
- `pm-work/findings/clipboard-1389.md`・`pm-work/findings/webview2-calls-1355.md`(窓の呼び出しの記録の方式)
- `tools/smoke/welcome-flow.mjs`(起動と Welcome の閉じ方)

## やること

0. 呼び出しの記録(直す前に測る)。
   - 既定のプロジェクトを開き、Channel Rack の Volume のノブを右クリックする。その間に、ゲストが呼ぶクリップボードの API(`OpenClipboard`・`IsClipboardFormatAvailable`・`CountClipboardFormats`・`EnumClipboardFormats`・`GetClipboardData`・`CloseClipboard`)の並びと、ページの `navigator.clipboard.readText`・`read`・`navigator.permissions.query` の呼び出しを、既存の記録の方式で取る。
   - `GetClipboardData` が右クリックの間に呼ばれるかを必ず確かめ、結果を報告に書く。
   - Ctrl+V の時に、ページで `paste` イベントが出るかを確かめる。出ないなら、キーの既定の動作を止めている箇所(`keyPolicy` など)を特定し、報告に書く。

1. 直す(製品の既定の動作として入れる。オプトインを作らない)。
   - ① 問い合わせ(`IsClipboardFormatAvailable`・`CountClipboardFormats`・`EnumClipboardFormats`)は、表示を出さず、権限を要求しない。答えは次の順で決める。
     - (a) 既に知っている状態があれば、それに従う。知っている状態は、利用者の貼り付けの操作の時に得た文字列(`paste` イベントの内容、または `readText` の結果)と、Copy で書いた文字列。(a) は (b)〜(d) より先に見る。Ctrl+V の貼り付けが権限の `denied` でも動くようにするため。
     - (b) 権限が `denied`: 「無い」(読めない)。
     - (c) 権限が `granted`: `navigator.clipboard.read()` の形式の一覧(短時間のキャッシュ。権限の確認は出ない)。内容は読まない。
     - (d) 権限が `prompt` で、知っている状態が無い: 「在る」(`CF_UNICODETEXT` と `CF_TEXT`)。中身は読まず、許可の確認も出さない。
     - `CountClipboardFormats`・`EnumClipboardFormats` は、同じ答えに合わせる。
   - ② 貼り付けの操作の経路は次の 2 つ。
     - Ctrl+V(キーボード): `keyPolicy` が Ctrl+V の既定の動作を止めないようにし、ブラウザに貼り付けを実行させる。フォーカス中のデスクトップで `paste` イベントが出るので、その `clipboardData` の `text/plain` を、次の 1 回の読みに渡す(権限の確認も、読みも無い)。使い切りにし、マウスを押した時に捨てる。ゲストの `GetClipboardData` は、その文字列で答える。
     - メニューの項目(FL の「貼り付け」を選ぶ): ゲストの `GetClipboardData` で、`navigator.clipboard.readText()` を呼ぶ。権限が `prompt` なら、ブラウザの許可の確認が出る(クリックの操作の中で呼ぶ)。`granted` なら、そのまま読む。`denied` なら、読まずに、理由の表示を 1 回出し、エンジンには空を返す。エンジンは止めない(`fle_stop` にしない)。
   - ③ 表示(トースト)は、貼り付けの操作の結果と、Copy の結果としてだけ出る。問い合わせでは出さない。同時に 1 つ。5 秒で自動に消える。閉じるボタンがある。同じ理由の表示は、出ている間に重ねない(出ている間は、消える時刻を延ばすだけ)。
   - ④ ページが作る独自の「貼り付け」のメニューは作らない。メニューは FL 自身の物だけ。
   - ⑤ #1389 の Copy(`writeText` の経路)を壊さない。Copy が失敗した時の表示は残してよい(③ の規則に従う)。
   - 使い勝手の帰結(報告に正直に書く): 権限が `prompt` では、FL の「貼り付け」は有効で、押すと許可の確認が出る。`denied` では、メニューの「貼り付け」は無効になる。Ctrl+V は、`denied` でも、`paste` イベントの内容で貼り付く。知っている状態が無い時の答えは、Windows の実際の値とは違う。その差を報告に書く。

2. 測定の道具 `tools/clipboard/measure.mjs` を拡張する(既存の項目 `error`・`copied`・`match` は残す)。
   - 位置は、固定のゲスト座標と倍率で求めない。FL は DPR によって拡大率の段を選び、配置が段ごとに変わる。位置は、DPR ごとに、次のどちらかで実測する。
     - (a) 差分: 「Paste value」の行。同じ DPR で、ノブを右クリックして新しく現れたメニューの窓(窓の表の、新しく現れた最上位の窓の矩形)の画素を、権限 `denied`(項目は灰色)と `prompt`(有効)の 2 回で取る。窓の矩形の中で画素が違う行の中央を押す。
     - (b) 差分: ノブ。Channel Rack の Volume のノブを、ホイールで 1 目盛り動かす前後の画素を比べ、変化した矩形の中の、ノブの円形の部品の中心を押す。測定の後、値を元に戻す。
     - (c) 箱の「FLEX (FL)」の行・Ok・名前の欄(F2 で開く窓)は、窓の表の矩形か、同じ差分で求める。
     - (d) 上の (a)〜(c) が難しい時は、窓の表の矩形からの相対位置を、窓の大きさの比で換算する。
     - DPR ごとに 1 回求め、その DPR の 3 件で使う。求めるのに失敗したら、その DPR を 1 回やり直す(1 件の失敗で残りを止めない)。
   - DPR は、1 と、1.01〜3.00 の乱数 3 点(実行ごとに変わる)の、4 点にする。固定の 2.3 は残さない。部屋の大きさは、前回の差分の方式(1836 × 1056 CSS px)を使ってよい。
   - 権限は `prompt`・`denied`・`granted` の 3 通り。CDP の `Browser.grantPermissions`・`Browser.setPermission`・`Browser.resetPermissions` で作る(headless では、`readText` の許可の確認は実際には出ない。だから状態を作る)。
   - ページに計数の仕掛け(`navigator.clipboard.readText`・`read` の呼び出しと、`paste` イベントの数)を入れる。
   - stdout に JSON を 1 行。`runs` は 12 件(権限 3 通り × DPR 4 点)。各件は次の項目。
     - `permission`: 開始時の権限(`prompt`・`denied`・`granted` のいずれか)
     - `dpr`: DPR
     - `knobMenuShown`: ノブの右クリックで、FL の文脈メニューが出た
     - `knobReadCalls`: ノブの右クリックの間に、`readText`・`read` が呼ばれた数
     - `permissionAfterKnob`: 右クリックの後の `navigator.permissions.query` の状態
     - `knobVisibleAlerts`: 右クリックの後、可視の `role="status"`・`role="alert"`・`aria-live` の要素の数(操作の前から見えていた常設の状態表示は数えない)
     - `pasteEnabledOnKnob`: 右クリックのメニューの「貼り付け」が有効(エンジンの `IsClipboardFormatAvailable` の答えが真)。`granted` の件では、テストが事前に `writeText` で文字列を入れておく
     - `ctrlVMatch`: Ctrl+V で、テストが事前に `writeText` で入れた数字の文字列が、エンジンの `GetClipboardData` に届いて一致した
     - `ctrlVReadCalls`: Ctrl+V の間に、`readText`・`read` が呼ばれた数
     - `pasteEventOnCtrlV`: Ctrl+V の間に、ページで `paste` イベントが出た数
     - `permissionAfterCtrlV`: Ctrl+V の後の権限の状態
     - `menuPasteReadCalls`: メニューの「貼り付け」の間に、`readText` が呼ばれた数
     - `menuPasteMatch`: メニューの「貼り付け」で一致した。`granted` の件は最初の貼り付け。`prompt` の件は、`Browser.setPermission` で `granted` にした後の 2 回目の貼り付け。`denied` の件は null
     - `toastOnMenuPaste`: メニューの「貼り付け」の結果で、理由の表示が出た
     - `toastCount`: 操作の間に、表示が同時に出ていた最大数
     - `toastHiddenAfter5s`: 表示が出た後、5 秒以内に非表示になった(表示が無ければ true)
     - `toastHasClose`: 表示に閉じるボタンがある(表示が無ければ true)
     - `engineState`: 最後のエンジンの状態
   - 1 件の中の手順: ノブの右クリック → 5 秒待つ(表示と権限の状態を見る)→ Ctrl+V → 5 秒待つ → メニューの「貼り付け」→ 5 秒待つ。`prompt` の件は、メニューの貼り付けの後に `Browser.setPermission` で `granted` にし、もう一度メニューの「貼り付け」をする。
   - top-level の `copyFailure`: `clipboard-write` を `denied` にして、Problems 箱の「FLEX (FL)」の Copy を選ぶ。項目 `toastShown`・`toastHiddenAfter5s`・`toastHasClose`・`toastCount`。
   - 使い方: `node tools/clipboard/measure.mjs <bundle の絶対パス> > clip.json`。終了コードは、JSON が書けたら 0。

3. 流れ A〜D の測定: `tools/smoke/welcome-flow.mjs` を同じ道具で走らせ、4 行(A・B・C・D)が在ること、各行の判定(`error`・`state`・`standIn`・`trapped`・`hostErrors`・`opened`)が通ること、流れ B と D で `welcomeClosed` が真であること。

4. 報告に書く: 呼び出しの記録(0 の並び)、`paste` イベントの有無、採った方式と理由、使い勝手の帰結、`runs` の 12 件の値、`copyFailure`、流れ A〜D の結果、`elapsedMs` と `preBootBytes` の値、下の「既知の限界」の各点。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/clipboard/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/clip.json" && node -e 'const fs=require("fs");const r=JSON.parse(fs.readFileSync(process.argv[1],"utf8"));const runs=Array.isArray(r.runs)?r.runs:[];const by=(p)=>runs.filter((x)=>x.permission===p);const pr=by("prompt"),de=by("denied"),gr=by("granted");const dprs=[...new Set(runs.map((x)=>x.dpr))];const common=(x)=>x.knobMenuShown===true&&x.engineState==="engine-idle"&&x.permissionAfterKnob===x.permission&&x.knobVisibleAlerts===0&&x.ctrlVMatch===true&&x.ctrlVReadCalls===0&&x.pasteEventOnCtrlV===1&&x.permissionAfterCtrlV===x.permission&&x.toastHiddenAfter5s===true&&x.toastHasClose===true&&x.toastCount<=1;const cf=r.copyFailure||{};process.exit(r.error===null&&r.copied===true&&r.match===true&&runs.length===12&&pr.length===4&&de.length===4&&gr.length===4&&dprs.length===4&&dprs.includes(1)&&dprs.filter((d)=>d!==1).every((d)=>d>=1.01&&d<=3.00)&&runs.every(common)&&runs.filter((x)=>x.permission!=="granted").every((x)=>x.knobReadCalls===0)&&gr.every((x)=>x.knobReadCalls<=1)&&pr.every((x)=>x.pasteEnabledOnKnob===true&&x.menuPasteReadCalls>=1&&x.menuPasteMatch===true&&x.toastOnMenuPaste===false)&&de.every((x)=>x.menuPasteReadCalls===0&&x.toastOnMenuPaste===true)&&gr.every((x)=>x.pasteEnabledOnKnob===true&&x.menuPasteMatch===true&&x.toastOnMenuPaste===false)&&cf.toastShown===true&&cf.toastHiddenAfter5s===true&&cf.toastHasClose===true&&cf.toastCount<=1?0:1)' "$RUNNER_TEMP/clip.json" && node tools/clipboard/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --calibration > "$RUNNER_TEMP/cal.json" && node -e 'const fs=require("fs");const r=JSON.parse(fs.readFileSync(process.argv[1],"utf8"));const c=Array.isArray(r.calibrations)?[...r.calibrations].sort((a,b)=>a.dpr-b.dpr):[];const e=Array.from({length:51},(_,i)=>1+0.04*i);process.exit(c.length===51&&c.every((x,i)=>Math.abs(x.dpr-e[i])<1e-6&&x.ok===true)?0:1)' "$RUNNER_TEMP/cal.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;const pb=Math.max(...L.map((r)=>r.preBootBytes));const el=Math.max(...L.map((r)=>r.elapsedMs));process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true&&el<=12500&&pb<=314368711?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app
```

## 判定の基準(受け入れのコマンドの各段の結果)

- `tools/clipboard/measure.mjs` の JSON: `error` が null・`copied` と `match` が真・`runs` が 12 件・DPR の点が 4 つ(1 と、1.01〜3.00 の 3 点)・権限の 3 通りが各 4 件。
- 全ての件で、`knobMenuShown` が真・`engineState` が engine-idle・`permissionAfterKnob` が `permission` と同じ・`knobVisibleAlerts` が 0・`ctrlVMatch` が真・`ctrlVReadCalls` が 0・`pasteEventOnCtrlV` が 1・`permissionAfterCtrlV` が `permission` と同じ・`toastHiddenAfter5s` が真・`toastHasClose` が真・`toastCount` が 1 以下。
- `prompt` の件: `knobReadCalls` が 0・`pasteEnabledOnKnob` が真・`menuPasteReadCalls` が 1 以上・`menuPasteMatch` が真(`Browser.setPermission` の後の貼り付け)・`toastOnMenuPaste` が偽。
- `denied` の件: `knobReadCalls` が 0・`menuPasteReadCalls` が 0・`toastOnMenuPaste` が真。
- `granted` の件: `knobReadCalls` が 1 以下・`pasteEnabledOnKnob` が真・`menuPasteMatch` が真・`toastOnMenuPaste` が偽。
- `copyFailure`: `toastShown` が真・`toastHiddenAfter5s` が真・`toastHasClose` が真・`toastCount` が 1 以下。
- 流れ A〜D の 4 行が在り、各行が `error` が null・`state` が engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が真。流れ B と D は `welcomeClosed` が真。4 流れの `elapsedMs` の最大が 12,500 以下。`preBootBytes` の最大が 314,368,711 以下。
- 校正の走査(`--calibration`)の JSON: `calibrations` が 51 件。DPR を並べ直すと 1.00・1.04・…・3.00 で、全件の `ok` が真。通る組と、`ok` が偽の 1 件を含む組は、判定で落ちる(合成データで確かめた)。
- 単体テスト(`npm test --prefix app`)が通る。

## 前回の試行で分かったこと(2 回目)

前回の試行(run 45524225325。判定の段で落ちた)の結果。次の作業者は artifact を読めないので、要点をここに書く。

- 作業者の受け入れは、DPR 1 と 2.3 の 6 件が全て通った。判定の段で、乱数の DPR が 1.56 の回だけ落ちた。commit は push されていない(判定で落ちたため)。前回の差分は作業者の artifact(`agent-1406`・`work.diff`)に在る。作業ツリーに取り込まれていれば、土台にしてよい。
- 落ちた内容: DPR 1.56 の回で、`menuPasteMatch` が prompt と granted の両方で偽、denied で `toastOnMenuPaste` が偽。「貼り付け」の項目を押せていなかった。
- 原因: 位置を、固定のゲスト座標 × 箱の幅から求めた倍率で換算していた。FL は DPR によって拡大率の段を選び、配置が段ごとに変わる。DPR 1.56 は別の段に入る。だから、位置は実測で求める(上の「やること」の 2)。DPR 1 と 2.3 は偶然通った。
- 前回の `paste` イベントが出なかった原因: `app/src/input/keymap.ts` の `keyPolicy` が、Ctrl+V の keydown を既定の動作ごと止めていた。前回の差分では、Ctrl+V を転送し、既定の動作を止めない形(`prevent: false`)に直した。直した後は、`pasteEventOnCtrlV` が 1 で、中身は一致した。
- 右クリックの `GetClipboardData`: FL は、右クリックの時に文字列を数値として解釈できるかで「貼り付け」の有効・無効を決める。英数字の文字列では、`IsClipboardFormatAvailable` が 1 でも無効に描かれた。だから、測定の文字列は数字にする。知っている状態が無い時は、数値の `"0"` を返す(読まない)。
- `granted` の右クリックでは、形式の一覧のための `navigator.clipboard.read()` が 1 回走る(内容は読まず、確認も出ない)。
- headless の Chrome では、`readText()` の許可の確認は実際には出ず、失敗して空が返った。許可の状態は `Browser.setPermission` で作る。
- 測定の道具の直した点: 部屋の大きさを 1836 × 1056 CSS px のまま保つ。ただし、拡大率の段が変わる DPR では、位置の換算が外れる(上の原因)。
- ページ常設の状態表示(`audio-status`・`status`・`taskbar`)は、操作の前から見えているので、表示の数に入れない。
- 前回の差分は、Copy が失敗した時の表示を残している。この表示も、5 秒で消え、閉じるボタンが在り、重ねないことを、受け入れに入れた。
- 前回の差分(artifact の `work.diff`)には、`pm-work/findings/clipboard-1406.md`(直す前の記録と、6 件の表)が含まれる。この Issue の報告は、同じ形で、12 件と `copyFailure` を書く。

## 前回の試行で分かったこと(3 回目。run 45569813770)

数値は runner の値(nproc 4。測定の終わりの loadavg 2.02・2.22・2.13)。前回の差分(16 ファイル。`pm-work/findings/clipboard-1406.md` を含む)は、`.prior-work.md` の取り込みで作業ツリーに入る。

- 判定は失敗。失敗の段は、受け入れの `clip.json` の判定。乱数の DPR 1.44 の回だけ落ちた。前の段は全て通った: `npm run build`・`fe-build engine`・`fe-build bundle`・`boot-bundle`(engine-idle・standIn false・elapsedMs 11,231)・`npm test --prefix app`。
- 落ちた内容: DPR 1.44 の 3 件(prompt・denied・granted)が `error: "no calibration at this DPR"`。校正の行は `calibration {"dpr":1.44,"ok":false}`。ノブは見つかった(`knob` の wheelBox 49x60・ホイールの位置 626,234)が、校正の出力に `row`(メニューの行の差分)が無い。
- 通った DPR: DPR 1(校正 ok・prompt・denied・granted の全件 ok)、乱数の 1.77(校正 ok・wheelBox 57x61)、1.81(校正 ok・wheelBox 57x61)。`copyFailure` は DPR 1 で通った(`toastShown` true・`toastHiddenAfter5s` true・`toastHasClose` true・`toastCount` 1・`toastEpisodes` [5000])。
- 作業者の手元の判定(乱数の DPR は 1・1.92・2.53・1.13)は全て通っていた。落ちたのは、この run の乱数の 1.44 だけ。乱数の DPR は実行ごとに変わるので、どの DPR で落ちるかは、run の前には分からない。
- 1.44 の位置: DPI に換算すると約 138(96 × 1.44 = 138.24)。FL の拡大率の段の境界の近く(推定)。ノブの大きさ(wheelBox)は段で変わる(1.44 で 49x60、1.77 と 1.81 で 57x61)。
- 今の差分の方式: 問い合わせは新しい非同期の import `fle_clip_host_probe`(知っている文字列 → denied は無 → granted は形式の一覧 → prompt で知らなければ在る)。FL の `GetClipboardData` は、読まずに答える(知っている文字列。無ければ数値の `"0"`)。Ctrl+V は `app/src/input/keymap.ts` の `keyPolicy` が既定の動作を止めないように直した(直す前は `paste` が 0 回)。
- 位置: ノブとメニューの行は差分で求める。Ok は子窓の表に無いので、幅の比の式で求め、閉じる操作で窓が消えることを確かめる。名前の欄(F2)は、式の候補で F2 が名前の窓を開けば採る。
- 表示: 同時に 1 つ・5 秒で消える・閉じるボタン・貼り付けと Copy の結果でだけ出る(`copyFailure` の表示も同じ規則で消える)。
- 作業者の commit は `75168a44`(ローカルのみ。push されていない)。作業者の判定の記録は、`pm-work/findings/clipboard-1406.md` の差分にある。
- 既知の限界(依頼者の判断): キーボードでメニューを開いた直後の貼り付けは、右クリックの問い合わせと誤認しうる。`granted` の右クリックでは、形式の一覧のための `read()` が 1 回走る(内容は読まず、確認も出ない)。headless では `readText` の許可の確認が出ないので、許可の確認は実機で確かめる。

## 3 回目の試行でやること(PM の決定)

1. **校正が落ちる理由を特定する**: DPR を 1.00 から 3.00 まで 0.04 刻み(51 点)で、校正だけ(起動と校正。権限の 3 通りは測らない。1 点約 40 秒)走らせ、どの DPR で `ok` が false になるかを表にして、`pm-work/findings/clipboard-1406.md` に書く。走らせる道具は、`tools/clipboard/measure.mjs` に校正だけの走査(`--calibration`)として足す。出力の JSON は `calibrations` の配列で、1 点ごとに `dpr` と `ok` を持つ(51 件)。
2. **直す**: 失敗する DPR で、ノブの位置とメニューの行を求められるようにする。位置の求め方を、FL の拡大率の段に追随させる(窓の表の窓の矩形の比から)。段の境界では、段ごとの候補を試し、ノブの差分とメニューの窓の矩形の両方で確かめる。
3. **受け入れ**: 乱数の DPR 3 点と DPR 1 の 12 件が、元のまま全て通る。加えて、校正の表(`--calibration`)で、全 51 点が `ok`。
4. この Issue は、ほぼ完了している。再開で、校正を直し、受け入れを通して commit して終える。
5. 受け入れの鎖は、エンジンの作り直し(約 35 分)・12 件の測定(約 16 分)・校正の走査(約 34 分)を含み、約 90 分ある。作業者は、段ごとに分けて実行してよい。判定は、鎖の全体を 1 回で見る(ジョブの上限は 5 時間 50 分)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 右クリックの呼び出しの記録、`paste` イベントの有無、採った方式と理由、使い勝手の帰結、`runs` の 12 件の値、`copyFailure`、流れ A〜D の JSON の行。
- 既知の限界(依頼者の判断に残す。報告に明記し、直さない):
  - キーボードでメニューを開いた直後の貼り付けは、右クリックの問い合わせと誤認しうる(右ボタンを離してから 1.5 秒以内、という手がかりによる)。誤認の結果は、その時だけ形式の問い合わせの答えが知っている状態になるだけで、害は小さい。
  - `granted` の右クリックでは、形式の一覧のための `read()` が 1 回走る(内容は読まず、確認も出ない)。
  - headless では `readText` の許可の確認が出ないので、許可の確認は、実機での確認が要る。この Issue では確かめていない。
  - Copy が失敗した時の表示は残す(依頼者の指示は「貼り付けの結果としてだけ」だが、Copy の表示は同じ規則で消える)。広げすぎかどうかは、依頼者が判断する。
  - 位置は、拡大率の段の境界では外れうる。乱数の 3 点は、その境界を必ず踏むとは限らない。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。テストのための分岐を製品に入れない。
- Python を使わない(調査と検証にだけ使う)。OCR を使わない(FL の画面の文字は、ゲストのメモリの走査か、API の呼び出しの記録で読む)。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。待つ時は、条件(要素・イベント)で待つ。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- 開始の時刻を、最初に `date -u` で記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える。
- 作業ツリーに前回の差分(`.prior-work.md` の取り込み)が在れば、最初に読み、土台にする。前回の製品の変更(Ctrl+V の `keyPolicy`、問い合わせの非同期の import、ページ側の `clipboard.ts`)は残してよい。直すのは、位置の求め方と、測定の道具の項目(上の「やること」の 2)。
- engine の作り直しは約 35 分。1 回の試行で、ビルドは数回まで。実験は、記録と trace をまとめた 1 回のビルドで行う。
- ビルドは、フォアグラウンドで、シェルツールの `timeout` 引数に 5400000 を渡して待つ。背景で起動して待たない。
- blend2d の取得(GitHub)が 403 で落ちたら一時的。受け入れのコマンドは `lift/out-app/b2d-src` を消してから作り直すので、そのまま再試行する。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: なし。
- #1389(親)の方式を、直す。`app/src/engine/protocol.ts`・`app/src/engine/worker.ts` は、他の Issue(#1390・#1392 など)もメッセージ型を足す。衝突したら、main の版を取り、この Issue の変更を取り直し、両方を残す(削除しない)。
- #{{164}}(E2E の基盤)が、この Issue の結果を、本番と同じ配信の上で検査する。この Issue は、それを待たずに入る。
