---
draft: "010"
title: "右クリックでメニューを画面に出し、項目の選択を engine に返す"
type: Task
milestone: "M3 トップバーのコンテキストメニュー(P2)"
parent: "009"
blocked_by: []
labels: ["topbar-menu"]
size: M
priority: High
project: signature-studio/3
---

## 目的

課題 C-6: トップバーのコンテキストメニューを機能させる。右クリックの後に、メニューが画面に出て、項目を選ぶと engine に届くこと。

## 背景

- 旧の測定(pm-work/findings/survey-c6-topbar-menus.md。runner の値。変更の前の版): 右クリックは、ページの `contextmenu` を止めた上で engine に届いた(入力欄に `button R up` が出て、`defaultPrevented` が true)。engine のログは増えなかった(322 → 322 行)。メニューらしい物は見えなかった。
- 前提(読みによる推定): `lift/runtime/fle_crash1_menu.c` の `fle_imp_TrackPopupMenu`(600〜618 行)は、「何も表示せず、選ばれず、窓へ何も送らない」(注釈 600〜601 行、`fle_crash1_menu.h` 5 行)。戻り値は、`TPM_RETURNCMD`(0x100)なら 0、それ以外なら 1(617 行)。**前回の実測で、この前提は成り立たなかった(次の節)。**
- 関係する経路: `app/src/input/pointer.ts` 68 行(`onContext` が既定の動作を止める)と 104 行(`contextmenu` の登録)。`lift/runtime/fle_input.c` 975〜992 行(右ボタンを離した時の `WM_CONTEXTMENU` の送信)。
- 起動直後の画面には、「使用状況の送信への同意」の確認ダイアログが出る(旧の測定。「No thanks」で閉じられる。測定の道具の中だけで「No thanks」で閉じる。製品の既定の動作は変えない)。

## 前回の試行で分かったこと

前回の作業は差し戻され、push されていない。実装は失われた。main から再開する。

- **実測: 右クリックで `TrackPopupMenu` は呼ばれなかった。** 約 45 点の右クリック(Welcome の窓、同意のダイアログ、FL の窓の中のトップバーの 3 部品・ブラウザ・チャンネルラック・格子・メニューバー・トランスポート・プレイリスト)で、`WM_CONTEXTMENU` は engine に届いたが、`TrackPopupMenu` は 0 回だった。ただし、前回の記録は「呼ばれた」と「早く戻った」を区別できなかったので、今回は記録を入れて確かめ直す(やること 1)。
- **3 部品の右クリック**: `WM_CONTEXTMENU` のログ行は、右ボタンを**離した後**に出た。押したままでは、FL 自前のヒント「[Trial] Tempo」だけが出た。
- **メニューバーの FILE を左クリックすると FL のメニューが開く。** このときも `TrackPopupMenu` の記録は無い。FL は、このメニューを自分で描いている(推定)。
- **3 部品の位置**(既定のプロジェクトを開いた後。ゲストの画素。画像からの推定で ±5 px): Tempo(140.000)約 (461, 19)、Song Position(1:01:00)約 (559, 19)、Pattern Selector 約 (606, 62)。起動直後の画面では見えない。既定のプロジェクトは、スタートの画面の Default の行をダブルクリックすると開き、トップバーのある主窓が出る。
- **import の確認(前回の結果)**: FL の DLL の import 表に `TrackPopupMenu` があり、`TrackPopupMenuEx` は無い。候補のうち `TrackPopupMenuEx` は、この確認では当たらなかった。
- **画素数だけでは判定できない**: 右クリックの後、近傍で 1357 画素が変わったが、これはヒントの変化で、メニューではなかった。無入力の 1 秒の前後の差は、近傍で 164 画素ほど(揺れ)。
- **未確認**: 項目を選んだ時の戻り値と `WM_COMMAND` の ID(選べるメニューが出なかった)。`TPM_RETURNCMD` は、呼び出しが選択より前に戻るため、同期の戻り値を返せない(前回のコードの注釈)。
- **ビルドは長い**: 前回は、`fe-build engine` のコンパイルと wasm の最適化だけで、それぞれ 8 分以上かかった。記録は 1 回のビルドに全部入れ、1 回の起動で候補を全部試す。
- **前回の描き方(失われた。必要なら再実装してよい)**: 引数の hWnd の最上位窓の描画面に、項目の帯を描く(`fle_raster_window_surface`・`fle_r_rop_rect`・`fle_raster_wrote`)。窓は新しく作らない。描いた画素は保存し、閉じる時に戻す。文字は描けない(グリフの描画器が無く、`ExtTextOutW` は止まる)。項目は文字数に比例する帯、チェックは四角、子メニューは矢印、区切りは線。デスクトップ窓の上では描かない。`TrackPopupMenu` は待たずに戻り、選択は次のボタンの押下と離す操作で取る。
- **前回の測定の道具(失われた)**: `tools/feat-c6-popup-menus/measure.mjs`。ゲストの画面は `window.__app.deskDumpBase64(true)`(peek。描画を起こさない)で読んだ。`(false)` は、描画が忙しい時に失敗しうる。

## 読む物

- `lift/runtime/fle_crash1_menu.c`(600〜618 行: `fle_imp_TrackPopupMenu`)
- `lift/runtime/fle_crash1_menu.h`(先頭の注釈。5 行)
- `lift/runtime/fle_input.c`(133 行: 入力の入口。538 行: `translate_mouse`。973〜994 行: `WM_CONTEXTMENU` の送信)
- `lift/runtime/fle_input.h`(97 行付近: `FLE_WM_RBUTTONUP`)
- `lift/runtime/fle_win.c`・`lift/runtime/fle_win.h`(窓の表。`fle_win_dc_query`)
- `lift/runtime/fle_raster.h`(85〜86 行: `fle_raster_window_surface`・`fle_raster_wrote`)、`lift/runtime/fle_desk.h`(窓を描く仕組みの注釈)
- `lift/runtime/fle_vfs.c`(110 行: `fle_vfs_note_missing`。stderr に 1 行を書く記録の書き方の例)
- `app/src/input/pointer.ts`(68・104 行)
- `app/src/engine/worker.ts`(`pushLog`。engine のログの経路)、`app/src/main.ts`(engineLog・`deskDumpBase64`・`window.__app`)
- `app/index.html`(23〜27 行: `#toolbar`)
- `tools/boot-check/boot-bundle.mjs`(`bootBundle`: 受け入れと同じ起動。engine のログ全体を返す)、`tools/boot-check/cdp.mjs`
- `tools/missing-paths/measure.mjs`(`bootBundle` で engine のログを読む道具の例)
- `pm-work/findings/survey-c6-topbar-menus.md`(測定の手順と結果)

## やること

1. **3 部品の位置を、起動後の画面で確かめる。** 既定のプロジェクトを開いた後の画面を PNG に保存し、3 部品の位置を読み直す(報告に画像の読みを書く)。PNG は `.tmp-*` に置き、終わったら消す。
2. **呼び出しの記録を入れる。** 製品の動作は変えず、次の記録を stderr に 1 行ずつ書く。`fle_vfs_note_missing` と同じ書き方。環境変数・`#ifdef` は作らず、記録は常に出す。
   - メニューの族の API の入口(`lift/runtime/fle_crash1_menu.c` の `fle_imp_*`)。行の形は `menu-api: <API> <引数の要点>`。`TrackPopupMenu` は、早く戻る分岐(メニューが無い・窓が無い・表示先が無い)でも 1 行出す。
   - 右ボタンの押下・離し(`WM_RBUTTONDOWN`・`WM_RBUTTONUP`)を入力の処理で受けた時と、`WM_CONTEXTMENU` を送った時(`lift/runtime/fle_input.c`)。行の形は `input: <メッセージ> <宛先の窓> <座標>`。
   - 窓を作る・出す API(`CreateWindowExW`・`ShowWindow`・`SetWindowPos` など。FL が自前のポップアップを作るかを見る)。行の形は `window-api: <API> <窓>`。
3. **右クリックの記録を、3 部品で取る。** 既定のプロジェクトを開いてから、3 部品それぞれに、右ボタンを押す → 待つ → 離す → 待つ、の順で送り、その前後の engine のログの増分を取る。3 部品のどれにも `menu-api:` が出なければ、候補でも取る(プレイリストの各欄、ミキサーの各欄、チャンネルラックの名前欄)。
4. **結果で分かれる。**
   - どこかで、右クリックの後に `menu-api:` の行が出た: その API を呼んだ経路を `lift/runtime` で、メニューが画面に出るように実装する。描き方は、前回の方式を再実装してよい。
   - `menu-api:` は出ず、`window-api:` に窓の作成・表示が出た: FL が自前のポップアップを出している。その窓が画面に出ない原因(描画・表示の経路)を直す。直せない時は、原因を報告に書いて終える。
   - どこでも呼び出しの行が出ない: 前提が成り立たない。実際の呼び出しに無い表示を、偽のメニューとして描かない。記録の行をそのまま報告に書いて終える。受け入れの基準は緩めない。
5. **測定の道具** `tools/feat-c6-popup-menus/measure.mjs` を作る。`tools/missing-paths/measure.mjs` と同じく `tools/boot-check/boot-bundle.mjs` の `bootBundle` で起動し、engine-idle の後、次を行う。
   - (a) 測定の道具の中だけで、同意ダイアログを「No thanks」で閉じる。
   - (b) スタートの画面の Default の行をダブルクリックして、既定のプロジェクトを開き、engine-idle を待つ。
   - (c) 3 部品(Tempo・Song Position・Pattern Selector の順)のそれぞれで、右クリック前の画面を読む。右ボタンの押下と離しを送る(ゲストの画素の位置を、`window.__app` のゲストの画面の大きさと `#desktop` の位置から、ページの座標に換算する)。1 秒待ち、画面を読み、engine のログの増分を取る。
   - (d) 対照: 無入力の 1 秒の前後の差を、同じ近傍で数える。
   - (e) クリック点の周り(半径 200 px の正方形)で、対照を超えて変わった画素を数える(`changedNearClick`)。
   - (f) `menuBandChanged`: 変わった画素が、メニューの帯(連続した長方形)に入るかを判定する。閾値はエージェントが決め、測定の道具に書き、報告に書く。ヒントの変化(前回の「[Trial] Tempo」のような小さい帯の変化)では true にならないこと。
   - (g) 項目の選択は、この道具では行わない。
   - 部品の位置が読めない時は、その部品に `note` として「未確認」と書き、`menuBandChanged` は false にする。
   - stdout には JSON の 1 行だけを出す: `{"state","standIn","error","nproc","loadavg","parts":[{"name","x","y","menuApiCalls":[...],"controlNearClick","changedNearClick","menuBandChanged","note"}],"logAdded":[...]}`。`menuApiCalls` は、右クリックの後に engine のログに増えた行のうち `menu-api:` で始まる行。`x`・`y` はゲストの画素。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/feat-c6-popup-menus/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/feat-c6.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8").trim());const ok=r.state==="engine-idle"&&r.standIn===false&&r.error===null&&r.parts.some((p)=>p.menuApiCalls.length>0&&p.menuBandChanged===true);process.exit(ok?0:1)' "$RUNNER_TEMP/feat-c6.json"
```

判定の基準: 道具の出力の生の値を、上の node -e が読む。3 部品のうち 1 つ以上で、右クリックの後に、メニュー関係の API の記録(`menu-api:`)があり、かつ `menuBandChanged` が true。項目の選択(`WM_COMMAND`・戻り値)は、判定に入れない。報告に書き、後続の #{{011}} が扱う。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。
- 報告に書く: 変えたパス(lift/・app/ を含む)、3 部品と候補ごとの記録の行(`menu-api:`・`input:`・`window-api:` の一覧)、描き方の選択と理由(呼ばれた API に応じて)、`menuBandChanged` の閾値、3 部品の位置の読み、項目を選んだ時の戻り値と `WM_COMMAND` の ID(試せた時だけ)。
- 確認ダイアログを「No thanks」で閉じたのは測定の道具の中だけであることを書く。製品の既定の動作は変えていない。
- 前提が成り立たなかった時は、その事実(記録の行)をそのまま書く。

## 制約

- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない。記録の行は、常に出す。
- 製品の既定の動作を、測定のためだけに変えない(起動時の確認ダイアログを含む)。
- 実際の呼び出しに無い表示を、偽のメニューとして描かない。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 依頼者の資格情報・設定に触れない。

## 他のタスクとの関係

- #{{011}}(3 部品の値の変化)が、この Issue を待つ(blocked_by)。
- #{{006}}(HiDPI)が描画の座標を変える場合、この Issue の測定の座標を、#{{006}} の結果に合わせる。両方を変えるときは、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、受け入れをもう一度通す。
- #{{002}}(描画の分離)の Worker を通る経路を、描画の Worker が変える場合がある。その場合も、同じ受け入れで確かめる。
