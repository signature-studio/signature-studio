---
draft: "010"
title: "右クリックの記録の仕組みと測定の道具を入れ、ミキサーの右クリックで FL のメニューが出ることを、陽性対照として測る"
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

課題 C-6(トップバーのコンテキストメニュー)の原因を調べるための、記録と測定の道具を、まず確立する。この Issue は、トップバーにメニューを出す修正を含まない。トップバーの修正は、次の Issue が扱う。

依頼者の目標: トップバー(Tempo・Song Position・Pattern Selector)の右クリックの後に、FL のメニューが画面に出て、項目を選ぶと engine に届くこと。

## 背景

- 旧の測定(pm-work/findings/survey-c6-topbar-menus.md。runner の値。変更の前の版): 右クリックは、ページの `contextmenu` を止めた上で engine に届いた。engine のログは増えなかった。メニューらしい物は見えなかった。
- 前提(読みによる推定)は、実測で成り立たなかった。`lift/runtime/fle_crash1_menu.c` の `fle_imp_TrackPopupMenu`(602 行〜)は、メニューを表示しない前提だった。実際には、FL は文脈メニューを `TrackPopupMenu` ではなく、自前のポップアップの窓で出す(下の節)。
- 関係する経路: `app/src/input/pointer.ts` 68 行(`onContext` が既定の動作を止める)と 104 行(`contextmenu` の登録)。`lift/runtime/fle_input.c` 973〜994 行(右ボタンを離した時の `WM_CONTEXTMENU` の送信。983 行: 宛先は、窓が子なら親の窓)。
- 起動直後の画面には、「使用状況の送信への同意」の確認ダイアログが出る。測定の道具の中だけで「No thanks」で閉じる。製品の既定の動作は変えない。

## 前回の試行で分かったこと

2 回の試行とも、受け入れに到達せず、push されていない。実装は失われた。main から再開する。2 回目の報告の事実を、ここに書き写す(次の作業者は、報告を読めない)。

**1. ミキサーの右クリックでは、FL 自身の文脈メニューが画面に出る(陽性の対照)。**

- 既定のプロジェクトを開き、F9 でミキサーを開く。Insert 1 の名前(ゲストの画素 (506, 174))を右クリックすると、`WM_RBUTTONDOWN` の後に、`window-api: CreateWindowExW ex=0x10000 style=0x86000000 ... parent=0x7f990038 x=230 y=220 cx=186 cy=330`、`SetWindowPos 0x7f990290 ... x=345 y=20 cx=434 cy=444`、`ShowWindow` が出る。その後 `WM_RBUTTONUP` が出るが、`WM_CONTEXTMENU` は出ない。
- 画面には、FL の文脈メニューが出ていた(撮った画像で確認)。見出し「Mixer track (Insert 1)」、Rename・Change color・Change icon・Dock to ›・Delete・Insert multiple… など。チェック・区切り・子メニューの矢印もある。
- FX スロット 1 (660, 136) の右クリックも、窓を作って表示した(`CreateWindowExW … cx=780 cy=480`)。
- **`TrackPopupMenu` は、起動から測定までの全体で 1 度も呼ばれていない(`menu-api:` 0 件)。** FL は、自前のポップアップの窓で出す。

**2. トップバーの 3 部品の右クリックでは、窓も `TrackPopupMenu` も出ない。**

- 記録は、`input:` の行だけ(`window-api:` は右クリックの間に出ない):
  - Tempo: `input: WM_RBUTTONDOWN 0x7f990124 462,20 client=161,20`、同じ形の `WM_RBUTTONUP`、`input: WM_CONTEXTMENU 0x7f9900cc 462,20`
  - Song Position: `WM_RBUTTONDOWN 0x7f990128 566,20 client=69,20`、`WM_RBUTTONUP`、`WM_CONTEXTMENU 0x7f9900cc 566,20`
  - Pattern Selector: `WM_RBUTTONDOWN 0x7f990120 616,58 client=44,17`、`WM_RBUTTONUP`、`WM_CONTEXTMENU 0x7f990114 616,58`
- `WM_CONTEXTMENU` の行は、右ボタンを離した後に出た。押したままでは、FL 自前のヒント「[Trial] Tempo」だけが出た(約 1357 画素が変わる。高さ約 20 px で、帯ではない)。
- 押された子の窓(0x7f990124 など)ではなく、その親の窓(0x7f9900cc など)が `WM_CONTEXTMENU` の宛先になっている。
- 候補: チャンネルラックの名前欄(522, 145)は、`menu-api:` 0 件で、画面の変化は 1260 画素(帯ではない)。プレイリストの Track 3 の名前欄 (616, 255) と、Track 4 の格子 (735, 318) は、変化が 0 画素だった。
- 1 回目の試行では、約 45 点の右クリック(Welcome の窓、確認ダイアログ、トップバーの 3 部品、ブラウザ、チャンネルラック、格子、メニューバー、トランスポート、プレイリスト)で、`WM_CONTEXTMENU` は engine に届いたが、`TrackPopupMenu` は 0 回だった。
- メニューバーの FILE を左クリックすると、FL のメニューが開く。`TrackPopupMenu` は呼ばれない(推定: FL は、このメニューも自分で描く)。

**3. 推定(未確認)。**

- トップバーの部品(子の窓)の手続きには、`WM_CONTEXTMENU` が届いていない可能性がある。`fle_input.c` 983 行の規則で、宛先が親になるため。ミキサーは `WM_RBUTTONDOWN` の時点で窓を作り、`WM_CONTEXTMENU` を経由しない。この読みは未確認。次の Issue が確かめる。

**4. 位置(ゲストの画素。±5 px。起動直後のゲストの画面は 780×480。既定のプロジェクトを開いた後は、ページの画面が 502×309 でページの (0, 36) に出る)。**

- Tempo (462, 20)、Song Position (566, 20)、Pattern Selector (616, 58)。
- 「No thanks」(確認ダイアログ)は (513, 308)。スタート画面の既定の行 Default は (124, 81)(ダブルクリックで既定のプロジェクトが開き、トップバーのある主窓が出る)。ミキサーは F9。
- 候補: チャンネルラックの名前欄「Sampler」(522, 145)、プレイリストの Track 3 の名前欄 (616, 255)、Track 4 の格子 (735, 318)、ミキサーの FX スロット 1 (660, 136)。

**5. その他。**

- `TrackPopupMenu` は FL の DLL の import 表にあり、`TrackPopupMenuEx` は無い(1 回目の確認)。
- 画素数だけでは判定できない。無入力の 1 秒の前後の差は、近傍で 164 画素ほど(揺れ)。
- ビルドは長い(前回は、`fe-build engine` のコンパイルと wasm の最適化だけで、それぞれ 8 分以上かかった)。記録は 1 回のビルドに全部入れ、1 回の起動で候補を全部試す。
- 前回の描き方(窓の描画面に帯を描く案)は、`TrackPopupMenu` が呼ばれていないので使う場面が無い。この Issue では描かない。

## 読む物

- `lift/runtime/fle_input.c`(133 行: 入力の入口。538 行: `translate_mouse`。973〜994 行: 右ボタンの離しと `WM_CONTEXTMENU` の送信)
- `lift/runtime/fle_win.c`(904 行〜: `CreateWindowExW`。`ShowWindow`・`SetWindowPos`・`MoveWindow` の入口。628 行: 窓が見えた時の記録点の注釈)、`lift/runtime/fle_win.h`
- `lift/runtime/fle_crash1_menu.c`(602 行〜: `fle_imp_TrackPopupMenu`)、`lift/runtime/fle_crash1_menu.h`(先頭の注釈)
- `lift/runtime/fle_vfs.c`(110 行〜: `fle_vfs_note_missing`。stderr に 1 行を書く記録の書き方の例)、`lift/runtime/fle_vfs.h`(99 行: `missing-path:` の形の注釈)
- `app/src/input/pointer.ts`(68・104 行)
- `app/src/engine/worker.ts`(269 行〜: `pushLog`。engine のログの経路)、`app/src/main.ts`(`engineLog`・`deskDumpBase64`・`window.__app`)
- `tools/boot-check/boot-bundle.mjs`(`bootBundle`: 受け入れと同じ起動。engine のログ全体を返す)、`tools/boot-check/cdp.mjs`
- `tools/missing-paths/measure.mjs`(`bootBundle` で engine のログを読む道具の例)
- `pm-work/findings/survey-c6-topbar-menus.md`

## やること

1. **呼び出しの記録を入れる。** 製品の動作は変えない(戻り値・描画・入力の扱いは変えない)。記録は stderr に 1 行ずつ書く。書き方は `fle_vfs_note_missing`(`lift/runtime/fle_vfs.c`)と同じにする。
   - `lift/runtime/fle_crash1_menu.c` のメニューの族の入口(`fle_imp_*`)。行の形は `menu-api: <API> <引数の要点>`。`TrackPopupMenu` は、早く戻る分岐(メニューが無い・窓が無い)でも 1 行出す。
   - `lift/runtime/fle_input.c` の右ボタンの押下・離し(`WM_RBUTTONDOWN`・`WM_RBUTTONUP`)を、ゲストのメッセージ取り出しで受けた時と、`WM_CONTEXTMENU` を送った時(983〜992 行付近)。行の形は `input: <メッセージ> <宛先の窓> <座標>`。押下・離しは、クライアントの座標を `client=<x>,<y>` で付ける。`WM_CONTEXTMENU` は、押された窓(wParam)を `child=<窓>` で付ける。前回の形: `input: WM_RBUTTONDOWN 0x7f990124 462,20 client=161,20`、`input: WM_CONTEXTMENU 0x7f9900cc 462,20`。
   - `lift/runtime/fle_win.c` の窓を作る・出す API(`CreateWindowExW`・`ShowWindow`・`SetWindowPos`・`MoveWindow`)の入口。行の形は `window-api: <API> <窓> <引数の要点>`。
   - `menu-api:` と `input:` は常に出す(量は少ない)。`window-api:` は起動の間に約 7,000 行出るため、既定では出さず、測定の道具が有効にした時だけ出す(P1 を守る)。有効にする方法は、既存の仕組み(guest の VFS・レジストリ上の既存の設定など)から作業者が選ぶ。新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数は作らない。選んだ方法と理由を報告に書く。
2. **`tools/boot-check/boot-bundle.mjs` に任意のフック `afterIdle` を足す。** engine-idle の後に呼ばれる関数(省略時は何もしない)。CDP のページを渡す。既存の呼び出し(`tools/missing-paths/measure.mjs` など)の結果は変えない。
3. **測定の道具 `tools/feat-c6-popup-menus/measure.mjs` を作る。** `tools/missing-paths/measure.mjs` と同じく `bootBundle` で起動し、`afterIdle` で次を行う。
   - (a) 測定の道具の中だけで、確認ダイアログを「No thanks」(513, 308)で閉じる。
   - (b) スタート画面の Default(124, 81)をダブルクリックして既定のプロジェクトを開く。タイトルと engine-idle を確かめてから次へ進む(前回は、待つ時間が短すぎた)。
   - (c) 次の順に 1 部品ずつ測る: `tempo` (462, 20)、`song-position` (566, 20)、`pattern-selector` (616, 58)。そのあとに F9 でミキサーを開き、`mixer-insert1` (506, 174)。各部品で、右ボタンの押下 → 1 秒待つ → 離す → 1 秒待つ → 画面を読む。画面は `window.__app.deskDumpBase64(true)`(描画を起こさない)で読む。次の部品の前に、出たメニューを閉じる(閉じ方は作業者が選び、報告に書く)。
   - (d) 対照: 右クリックと同じ 1 秒の間隔で、無入力の 1 秒の前後の差を、同じクリック点の近傍で数える(`controlNearClick`)。
   - (e) 右クリックの前後の画面の差から、対照でも変わった画素を除き、残りを `changedNearClick` として数える。近傍は、クリック点の周りの、ゲストの画素で 200 px の正方形。
   - (f) `menuBandChanged`: 対照で変わった画素を除いた残りの画素(画面全体)を 4 連結の領域に分け、外接の箱が **幅 120 px 以上・高さ 40 px 以上・充填率 0.8 以上** のものが 1 つでもあれば true。ヒント(高さ約 20 px)は帯にならない。閾値は測定の道具の冒頭に書く。
   - (g) 項目の選択は、この道具では行わない。
   - 部品の位置が読めない時は、その部品の `note` に「未確認」と書き、`menuBandChanged` は false にする。
   - stdout には JSON の 1 行だけを出す: `{"state","standIn","error","nproc","loadavg","parts":[{"name","x","y","inputLines":[...],"windowApiCalls":[...],"menuApiCalls":[...],"controlNearClick","changedNearClick","menuBandChanged","note"}],"logAdded":[...]}`。`name` は `tempo`・`song-position`・`pattern-selector`・`mixer-insert1`。`inputLines` は、その右クリックの間(押下から、`WM_CONTEXTMENU` まで)に増えた `input:` の行。`windowApiCalls`・`menuApiCalls` は、同じ区間に増えた `window-api:`・`menu-api:` の行。`x`・`y` はゲストの画素。`logAdded` は、既定のプロジェクトを開いた後に増えた `input:`・`menu-api:`・`window-api:` の行。
4. **記録が動いていることを確かめる。** 測定の間に出た `window-api:` の行数を、報告に書く(前回は 6,997 行だった)。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/feat-c6-popup-menus/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/feat-c6.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8").trim());const m=r.parts.find((p)=>p.name==="mixer-insert1");const top=r.parts.filter((p)=>["tempo","song-position","pattern-selector"].includes(p.name));const ok=r.state==="engine-idle"&&r.standIn===false&&r.error===null&&m!=null&&m.windowApiCalls.some((l)=>/CreateWindowExW/.test(l))&&m.menuBandChanged===true&&top.length===3&&top.every((p)=>Array.isArray(p.windowApiCalls)&&["WM_RBUTTONDOWN","WM_RBUTTONUP","WM_CONTEXTMENU"].every((k)=>p.inputLines.some((l)=>l.includes(k))));process.exit(ok?0:1)' "$RUNNER_TEMP/feat-c6.json"
```

判定の基準: 道具の出力の生の値を、上の node -e が読む。

- (1) 起動: `state` が engine-idle、`standIn` が false、`error` が null。
- (2) 陽性の対照: `mixer-insert1` の `windowApiCalls` に `CreateWindowExW` の行があり、`menuBandChanged` が true。
- (3) トップバーの 3 部品(`tempo`・`song-position`・`pattern-selector`)それぞれの `inputLines` に、`WM_RBUTTONDOWN`・`WM_RBUTTONUP`・`WM_CONTEXTMENU` の行があり、`windowApiCalls` が配列で在る(空でもよい)。
- 3 部品の `menuBandChanged` と `windowApiCalls` の中身は、判定に入れない。報告に書き、次の Issue が扱う。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。
- 報告に書く: 変えたパス(lift/・tools/ を含む)、記録の行の形(実物の行を 1 つずつ)、`window-api:` を有効にした方法と理由、4 部品それぞれの `inputLines`・`windowApiCalls`・`menuApiCalls` の件数と、`menuBandChanged` の値、閾値、`window-api:` の行数(記録が動いていることの確認)。
- 確認ダイアログを「No thanks」で閉じたのは、測定の道具の中だけであることを書く。製品の既定の動作(戻り値・描画・入力・起動時の確認ダイアログ)は変えていないことを書く。

## 制約

- 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- 起動の経路に、重い出力を足さない(P1)。`window-api:` は測定の道具だけが有効にする。
- 製品の既定の動作を、測定のためだけに変えない。
- 実際の呼び出しに無い表示を、偽のメニューとして描かない。この Issue は、メニューを描かない。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 依頼者の資格情報・設定に触れない。

## 他のタスクとの関係

- #{{011}}(3 部品の値の変化)は、この Issue ではなく、トップバーのメニューを出す次の Issue を待つ。
- #{{006}}(HiDPI)が描画の座標を変える場合、この Issue の測定の座標を、#{{006}} の結果に合わせる。両方を変えるときは、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、受け入れをもう一度通す。
- #{{002}}(描画の分離)の Worker を通る経路を、描画の Worker が変える場合がある。その場合も、同じ受け入れで確かめる。
