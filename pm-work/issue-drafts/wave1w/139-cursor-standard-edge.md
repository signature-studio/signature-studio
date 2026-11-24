---
draft: "139"
title: "FL のマウスカーソルを、ゲストが設定した実物のカーソルに従わせる(段 1): 標準のカーソルと、窓の端のサイズ変更の矢印"
type: Task
milestone: "M3 トップバーのコンテキストメニュー(P2)"
parent: "009"
blocked_by: ["138"]
labels: ["cursor", "input"]
size: L
priority: High
project: signature-studio/3
---

## 目的

依頼者の指示(原文): 「マウスカーソルが反映されていません。ウィンドウの端にカーソルを合わせても矢印に変化せず、Piano roll で Pencil Mode でも鉛筆カーソルになりません。**形が似ているそれっぽいカーソルに当てはめないでください。実際のマウスカーソルのみ許可します。**」

PM の決定(解釈): 「実際のマウスカーソル」とは、ゲスト(FL)が設定したカーソルと同じ意味のカーソルを、利用者の OS の実物で出すこと。似た形の別の物に置き換えない。対応する物が無い時は、置き換えず、記録に残して件数と名前を出す。

この Issue(段 1)は、Windows の標準のカーソル(`LoadCursor(NULL, IDC_*)`)と、窓の端のサイズ変更の矢印を直す。FL 自身の資源のカーソル(Piano roll の鉛筆など)は、段 2 の #{{144}} で直す。

この Issue の目標は次の 3 つ。
1. 窓の端(サイズ変更が効く窓)にポインタを置くと、8 方向それぞれで、対応するサイズ変更のカーソル(ew-resize・ns-resize・nwse-resize・nesw-resize)が出る。
2. 標準のカーソル(I ビーム・待ち・手・禁止など)は、表(下の「標準のカーソルの対応」)の 1 対 1 の対応で出る。
3. 対応の無い標準のカーソルは、似た物に置き換えず、記録に残して件数と名前を JSON に出す。

## 背景

- 事実 1(窓に枠が無い): `lift/runtime/fle_win.c` の `client_origin` の注釈(決定 0123・S-nc-zero): 窓の非クライアント領域は 0。`lift/runtime/fle_input.c` の `FLE_WM_NCHITTEST` は、窓の矩形の中なら HTCLIENT、外なら HTNOWHERE だけを返す。サイズ変更の当たり判定(HTLEFT・HTRIGHT・HTTOP・HTBOTTOM・HTTOPLEFT など)は、どこからも返されない。
- 事実 2(端の矢印の理由): `fle_input.c` の `FLE_WM_SETCURSOR`(943 行付近)は、ht が HTCLIENT なら窓のクラスのカーソル(`v.hcursor`)、それ以外なら矢印を SetCursor する。端は HTCLIENT のままなので、矢印(または窓のクラスのカーソル)のまま。
- 事実 3(カーソルの送り先): `fle_input.c` は、マウスの移動で WM_SETCURSOR を送る(611 行付近)。engine は `lift/runtime/fle_appview.h` の `cursor`・`cursor_idc`・`cursor_shared` で、SetCursor が最後に設定したハンドルを報告する。
- 事実 4(ページ): `app/src/engine/appview.ts`・`app/src/engine/os-state.ts` が読み、`app/src/main.ts` の `osstate` handler(578〜583 行付近)が `cursorCss` を通して `view.setCursor(c.css)` を呼ぶ。`app/src/ui/desktop.ts` の `setCursor` は、`#desktop` の `style.cursor` を書く。
- 事実 5(表の誤り): `app/src/ui/cursor.ts` の `IDC_CSS` は `32516`(IDC_UPARROW)を `default` に当てはめている(`S-uparrow-default`)。CSS に上向きの矢印は無いので、これは「似た物への当てはめ」で、依頼者の指示に反する。この行は取り除き、対応の無い物の扱い(下の 3)に入れる。
- 事実 6(標準のカーソルの読み): `lift/runtime/fle_cursor.c` は、`LoadCursorW(NULL, IDC_*)` の標準のカーソルの 13 個を仮想の表で持つ(`FLE_CURSOR_IDC_*`)。`fle_gdi.c` の SetCursor は、この表のハンドルを受け、その他のハンドルは `fle_stop`(3265 行付近)。
- 事実 7(窓の枠のある版との差): Windows の窓は、サイズ変更の枠(`WS_THICKFRAME`)の幅(`SM_CXSIZEFRAME`・`SM_CXPADDEDBORDER`)の帯で、端のカーソルが変わる。`lift/runtime/fle_sysmetrics.c` は、SM_CXSIZEFRAME(番号 32)の応答を持つ。今の窓は枠を描かない(決定 0123)ので、帯の幅はどの当たり判定にも使われていない。

## 標準のカーソルの対応(PM の決定。1 対 1 の対応だけを使う)

ブラウザの cursor キーワードは、利用者の OS の標準のカーソルを描く。ここでは意味が同じ物だけを結ぶ。

| Windows の標準のカーソル | 値 | CSS の cursor |
|---|---|---|
| IDC_ARROW | 32512 | default |
| IDC_IBEAM | 32513 | text |
| IDC_WAIT | 32514 | wait |
| IDC_CROSS | 32515 | crosshair |
| IDC_UPARROW | 32516 | (対応なし。下の 3) |
| IDC_SIZENWSE | 32642 | nwse-resize |
| IDC_SIZENESW | 32643 | nesw-resize |
| IDC_SIZEWE | 32644 | ew-resize |
| IDC_SIZENS | 32645 | ns-resize |
| IDC_SIZEALL | 32646 | move |
| IDC_NO | 32648 | not-allowed |
| IDC_HAND | 32649 | pointer |
| IDC_APPSTARTING | 32650 | progress |
| IDC_HELP | 32651 | help |

3. 対応の無い標準のカーソル(今は IDC_UPARROW): 似た物に当てはめない。既定の矢印の扱いにし、engine の記録(`unmappedCursor`: 名前と値)に残す。受け入れの JSON の `unmapped`(件数)と `unmappedNames`(名前の配列)に出す。

窓の端の対応(PM の決定): 8 方向の矢印の対応は、ew-resize(E・W)、ns-resize(N・S)、nwse-resize(NW・SE)、nesw-resize(NE・SW)。

## 読む物

- `lift/runtime/fle_input.c`(`FLE_WM_NCHITTEST`・`FLE_WM_SETCURSOR`・マウス移動と押下の WM_SETCURSOR の送り(579・611・640 行付近)・クリックの配送)
- `lift/runtime/fle_win.c`(`client_origin`・非クライアント領域の注釈)・`lift/runtime/fle_input.h`(FLE_WM_NCHITTEST・FLE_WM_SETCURSOR の値)
- `lift/runtime/fle_cursor.c`・`lift/runtime/fle_cursor.h`(標準のカーソルの仮想の表)・`lift/runtime/fle_gdi.c`(SetCursor・GetCursor・ShowCursor。3249 行付近から)
- `lift/runtime/fle_sysmetrics.c`(SM_CXSIZEFRAME・SM_CXPADDEDBORDER)
- `lift/runtime/fle_appview.h`・`lift/runtime/fle_appview.c`・`app/src/engine/appview.ts`・`app/src/engine/os-state.ts`・`app/src/main.ts`(`osstate` handler と `cursorCss`)
- `app/src/ui/cursor.ts`(`IDC_CSS`・`cursorCss`)・`app/src/ui/desktop.ts`(`setCursor`)・`app/src/style.css`(`#desktop` の cursor)
- `tools/smoke/welcome-flow.mjs`(起動と窓の表の取り方)・`tools/feat-c6-popup-menus/measure.mjs`(窓の表と、窓の位置からの操作)
- 窓のスタイル: ゲストの CreateWindowEx の WS_THICKFRAME(0x00040000)の有無を、窓の表(engine の記録)で確かめる方法を、作業者が決める

## やること

0. 現状の測定(直す前): 窓の表から、窓の端 8 方向の位置と、標準のカーソルを SetCursor した時の記録(IDC の値)を取る。どの窓に WS_THICKFRAME が在るか(サイズ変更が効く窓)を、窓の表の記録で報告する。
1. 窓の端の当たり判定: サイズ変更の枠を持つ窓(WS_THICKFRAME。最大化していない窓)について、窓の端の帯(幅は SM_CXSIZEFRAME と SM_CXPADDEDBORDER の和。`fle_sysmetrics.c` の値)の中で、`FLE_WM_NCHITTEST` が HTLEFT・HTRIGHT・HTTOP・HTBOTTOM・HTTOPLEFT・HTTOPRIGHT・HTBOTTOMLEFT・HTBOTTOMRIGHT のいずれかを返すようにする。帯の外と、枠を持たない窓は、今と同じ(HTCLIENT・HTNOWHERE)。
   - 制約: 帯の中の WM_NCHITTEST の結果は、カーソルの決定に使う。クリックの配送(WM_LBUTTONDOWN など)を変えるなら、変える理由と、FL の操作(窓の端のドラッグ、スクロールバー、枠の近くのボタン)への影響を報告に書く。変えない形を選べるなら、そちらを選ぶ。
   - 窓のレイアウト(クライアントの矩形)は変えない(決定 0123 の非クライアント領域 0 は保つ)。
2. 標準のカーソルの表を直す: `app/src/ui/cursor.ts` の `IDC_CSS` を、上の表の 14 行に揃える。`32516`(IDC_UPARROW)の行は取り除き、対応の無い物の扱い(既定の矢印・記録・件数と名前の JSON)に入れる。表の全行を単体テストで検査する(`npm test --prefix app` に入る。表と、各 IDC の CSS の値の対応を全行確かめる)。
3. 対応の無い標準のカーソルを、engine の記録に残す(`unmappedCursor`)。ページの `osstate` の情報だけで足りるなら、その情報で作る。利用者向けの新しい表示は作らない。
4. 測定の道具 `tools/cursor/measure.mjs`(新規)。stdout に JSON を 1 行(下の「受け入れ」の項目)。
5. 直した後に、同じ道具で測り、直す前と後の値を報告に書く。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/cursor/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/cursor.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error===null&&r.edgeWindows>=1&&r.edgeSamples===8*r.edgeWindows&&r.edgeMatches===r.edgeSamples&&r.textOk===true&&r.unresolved===0&&Array.isArray(r.unmappedNames)&&r.unmapped===r.unmappedNames.length?0:1)' "$RUNNER_TEMP/cursor.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;const pb=Math.max(...L.map((r)=>r.preBootBytes));const el=Math.max(...L.map((r)=>r.elapsedMs));process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true&&el<=12500&&pb<=314385095?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app
```

判定の基準(`tools/cursor/measure.mjs` の JSON の項目):
- `error`: null。
- `edgeWindows`: サイズ変更の枠を持つ窓の数(1 以上)。窓ごとに、8 方向の帯の中の点にポインタを置く(CDP の `Input.dispatchMouseEvent` の mouseMoved)。
- `edgeSamples`: 8 × `edgeWindows`。
- `edgeMatches`: `edgeSamples` のうち、ページの `#desktop` の `style.cursor`(または `window.__app` が返す cursor の css)が、対応する値(N・S: ns-resize、E・W: ew-resize、NE・SW: nesw-resize、NW・SE: nwse-resize)と一致した数。`edgeDetail` に、窓・方向・期待・実際の値を全て出す。
- `textOk`: テキストの欄の上で、cursor が `text`。
- `unresolved`: SetCursor の handle が解決できなかった回数(engine の記録。0 が正しい)。
- `unmapped`: 対応の無い標準のカーソルの件数。`unmappedNames`: その名前の配列(件数と一致)。0 件でもよい。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 直す前の窓の端 8 方向の値(`edgeDetail`)、直した後の値、サイズ変更の枠を持つ窓の一覧(窓の種類とスタイル)、クリックの配送を変えたかどうか、`unmappedNames` の中身、`cursor.ts` の表の変更の差分。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。既存の FLE_* の環境変数は読むだけで、増やさない。
- 「似た形のカーソルに当てはめる」ことをしない。対応の無い物は、表示を変えず、記録に残す。
- カーソルの画像は、ぼやけさせない・拡大しない(段 2 の #{{144}} で、資源の画素をそのまま使う。この段は CSS キーワードだけを使う)。
- Python は製品に使わない(調査と検証にだけ使う)。OCR は使わない(FL の画面の文字は、ゲストのメモリの走査か、API の呼び出しの記録で読む)。
- ベンダー名と Cloudflare の識別子を、ファイルに書かない。
- sleep・timeout コマンド・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- 開始の時刻を、最初に `date -u` で記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える。
- engine の作り直しは約 35 分。1 回の試行で、ビルドは数回まで。実験は、記録と trace をまとめた 1 回のビルドで行う。
- ビルドは、フォアグラウンドで、シェルツールの `timeout` 引数に 5400000 を渡して待つ。背景で起動して待たない。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: #{{138}}(クリップボード。同じ engine から page への通知の経路 `protocol.ts`・`worker.ts` を使うので、直列にする)。
- #{{144}}(段 2: FL 自身のカーソル)は、この Issue の後に行う。
- #{{140}}(上部バー)は、この Issue の後に行う。
- 窓の端の帯で、クリックの配送が変わると、他の窓の操作に影響しうる。影響は報告に書く。
