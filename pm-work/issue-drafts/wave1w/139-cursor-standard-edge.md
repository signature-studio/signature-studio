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

## 前回の試行で分かったこと

- 前回: run 45453233663(判定は `failed`。差し戻し)。作業の commit は `5d86d73c`。main には無い。差分は作業ツリーに載る。
- 受け入れの結果: `boot-bundle` は engine-idle・standIn false・elapsedMs 11264。`measure` は error null だが、`edgeWindows` が 0 で、判定の式(`edgeWindows >= 1`)で落ちた。`welcome-flow` は 4 流れすべて error null・engine-idle・opened true(elapsedMs 10292〜10498、preBootBytes 311120188)。`npm test --prefix app` は 1382 件通過。
- 測れた窓は 0。約 150 の窓の作成時の style を読んだが、サイズ変更の枠(`WS_THICKFRAME` = 0x00040000)を持つ窓は 1 つも無い。主窓 `0x94030000`(最小化・最大化の箱はあるが枠は無い)、Welcome `0x94000000`、名前の変更 `TNameEditForm` `0x94000000`、Piano roll(子の窓)`0x54000000`。
- Piano roll(子。枠は無い)を `--force-sizable` で測ると、直す前も直した後も、8 方向すべて `default`。枠の無い窓は今と同じで、設計どおり。
- 前回の差分の帯の作り(`lift/runtime/fle_input.c`): 枠を持つ窓の 8 px の帯(`SM_CXSIZEFRAME` + `SM_CXPADDEDBORDER` = 4 + 4。96 DPI)で `WM_NCHITTEST` がサイズ変更のコードを返す。ただし、runtime が窓の端を決める形なので、PM の決定(下の「やること」)で作り直す。
- 標準のカーソル: `unresolved` 0、`unmapped` 0(`unmappedNames` は空)。`textOk` は true(名前の変更の窓の `TQuickEdit` の (107, 92) で `text`)。
- ゲストが作ったカーソル(`CreateIconIndirect`)の報告は 51 件(`madeByGuest`)。前回の `unresolved` の定義では、これを数えて落ちる。このため、ゲストの作った物は `unresolved` に数えない(段 2 の #{{144}} の範囲)。
- IDC_HAND(32649)は観測されない。この engine の LoadImageW は DLL から読んで NULL を返す(決定 0062)。
- 未確認: FL の窓手続きが、端の `WM_NCHITTEST` にどう答えるか。前回は、ゲストの窓手続きへの送りを観測していない。FL が端で `SetCursor(IDC_SIZE*)` を呼んだかも、観測していない。
- 前回の表の直し: `app/src/ui/cursor.ts` の `IDC_CSS` を、表の 13 行(`32516` 以外)に揃えた。`app/test/unit/cursor.test.ts` は、表の全行を検査する形に直した(旧版は `32516` を `default` に当てはめていた)。
- 道具: `tools/cursor/measure.mjs` は、窓の表の style を読む。`--force-sizable` は、枠の無い窓を測定の時だけ測る(判定には入らない)。部屋の大きさは、welcome-flow と同じ 1836×1056 に、先に合わせる。

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

0. 前回の差分(commit `5d86d73c` の内容)が作業ツリーに在るかを、`git status` と `git log` で確かめる。在れば、その上で続ける。
1. **観測(直す前に、窓の端の動きを記録する)**: ポインタを、主窓・Playlist・Channel Rack などの端の ±8 px に動かし、engine のログに次の 3 つを出す。製品の利用者向けの表示は増やさない。
   - (a) ゲストの窓手続きが受けた `WM_NCHITTEST` と、その答え(HTLEFT など。ゲストが答えなければ `DefWindowProc` の答え)。
   - (b) `SetCursor` の呼び出し(ハンドルが、どの `IDC_*` か)。
   - (c) `SetWindowLongPtr(GWL_STYLE)` と `SetWindowPos` の呼び出し(`WS_THICKFRAME` が後から足されるか)。
2. **runtime を、ゲストが決める形に直す(PM の決定)**:
   - ポインタの下の窓の `WM_NCHITTEST` を、ゲストの窓手続きに送り、その答えに従う。ゲストが処理しない時だけ、`DefWindowProc` の既定(枠の style から決める)を使う。
   - `WM_SETCURSOR`(`wParam` = 窓、`lParam` の下位 = hit-test の値、上位 = マウスのメッセージ)・`WM_NCMOUSEMOVE`・`WM_NCLBUTTONDOWN` の送りも、この答えに従う。`DefWindowProc` が `SC_SIZE` を始めるのは、答えが `HTLEFT` などの時だけ。
   - 今の runtime が帯の中の当たり判定を自分で決めている部分は、`DefWindowProc` の既定にだけ残す。
3. **runtime が FL の代わりに、サイズ変更の矢印を決めない**。FL のコードが `IDC_SIZE*` を要求した時だけ、そのカーソルが出る。窓の枠の有無(製品の窓の性質)を runtime が変えて矢印を出す、という作業はしない。
4. 標準のカーソルの表(`app/src/ui/cursor.ts` の `IDC_CSS`・13 行)・対応の無い物の記録(`unmappedCursor`)・ゲストが作ったカーソルの印(`cursorMade`。`fle_appview` の reserved の 1 語)・道具 `tools/cursor/measure.mjs` は、前回の差分のとおり残す。
5. **測定**: 直す前(1 の観測)と直した後(2 の後)で、同じ道具で、窓の端 8 方向の値(`edgeDetail`)と、(a)〜(c) の記録を取り、報告に書く。
6. **受け入れ**: 下の「受け入れ」のコマンドを、そのまま通す。通らなければ、出力を報告に貼り、作業ツリーを commit する。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/cursor/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/cursor.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error===null&&Number.isInteger(r.edgeWindows)&&r.edgeWindows>=0&&r.edgeSamples===8*r.edgeWindows&&r.edgeMatches===r.edgeSamples&&r.textOk===true&&r.unresolved===0&&Array.isArray(r.unmappedNames)&&r.unmapped===r.unmappedNames.length?0:1)' "$RUNNER_TEMP/cursor.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;const pb=Math.max(...L.map((r)=>r.preBootBytes));const el=Math.max(...L.map((r)=>r.elapsedMs));process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true&&el<=12500&&pb<=314385095?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app
```

判定の基準(`tools/cursor/measure.mjs` の JSON の項目):
- `error`: null。
- `edgeWindows`: 端の 8 方向の点のうち、ゲストの `SetCursor` がサイズ変更の IDC(`IDC_SIZENS`・`IDC_SIZEWE`・`IDC_SIZENWSE`・`IDC_SIZENESW`)を設定した点を持つ窓の数。0 でもよい。数を作らない。
- `edgeSamples`: 8 × `edgeWindows`。窓の端の 8 方向の点(CDP の `Input.dispatchMouseEvent` の mouseMoved)。
- `edgeMatches`: `edgeSamples` のうち、ページの `#desktop` の `style.cursor`(または `window.__app` が返す cursor の css)が、ゲストが設定した IDC に対応する値(`IDC_SIZENS`: ns-resize、`IDC_SIZEWE`: ew-resize、`IDC_SIZENWSE`: nwse-resize、`IDC_SIZENESW`: nesw-resize)と一致した数。`edgeDetail` に、窓・方向・ゲストの IDC・実際の値を全て出す。
- `edgeWindows` が 0 の時: 報告に、観測した窓の名前・端の位置・`WM_NCHITTEST` の答え・`SetCursor` の記録を書く。0 は、FL が端でサイズ変更のカーソルを呼ばなかった事実で、runtime が代わりに出したものではない。
- `textOk`: テキストの欄の上で、cursor が `text`。
- `unresolved`: SetCursor の handle が解決できなかった回数。ゲストが作ったカーソル(`CreateIconIndirect`)は数えず、`madeByGuest` に数える。0 が正しい。
- `unmapped`: 対応の無い標準のカーソルの件数。`unmappedNames`: その名前の配列(件数と一致)。0 件でもよい。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 直す前の窓の端 8 方向の値(`edgeDetail`)、直した後の値、サイズ変更の枠を持つ窓の一覧(窓の種類とスタイル)、クリックの配送を変えたかどうか、`unmappedNames` の中身、`cursor.ts` の表の変更の差分。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。既存の FLE_* の環境変数は読むだけで、増やさない。
- 「似た形のカーソルに当てはめる」ことをしない。対応の無い物は、表示を変えず、記録に残す。
- runtime が、FL の窓の枠の有無を変えて、サイズ変更の矢印を出さない。FL のコードの要求に従う。
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

## 他の Issue との重なり

#{{138}}(クリップボード)と #{{140}}(上部バー)が、同じ時期に、`app/src/engine/protocol.ts` と `app/src/engine/worker.ts` に、engine から page への通知のメッセージ型を足す。**足す変更どうしなので、`main` への push で衝突したら、両方の型・handler を残して解く**(片方を消さない)。解いた後、`npm test --prefix app` と受け入れを通す。
