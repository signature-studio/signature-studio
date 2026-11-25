---
draft: "144"
title: "FL のマウスカーソルを、ゲストが設定した実物のカーソルに従わせる(段 2): FL 自身のカーソル(Piano roll の鉛筆など)を、資源の画素のまま出す"
type: Task
milestone: "M3 トップバーのコンテキストメニュー(P2)"
parent: "009"
blocked_by: ["139"]
labels: ["cursor", "fl-exe"]
size: L
priority: High
project: signature-studio/3
---

## 目的

依頼者の指示(原文): 「マウスカーソルが反映されていません。ウィンドウの端にカーソルを合わせても矢印に変化せず、Piano roll で Pencil Mode でも鉛筆カーソルになりません。**形が似ているそれっぽいカーソルに当てはめないでください。実際のマウスカーソルのみ許可します。**」

この Issue(段 2)は、FL 自身のカーソル(FL の資源の RT_CURSOR と、FL が CreateIconIndirect で作るカーソル。Piano roll の鉛筆など)を直す。段 1(#{{139}})は、Windows の標準のカーソルと窓の端を直す。

PM の決定(画素): FL 自身のカーソルは、資源の実物のビットマップ(AND・XOR のマスクと、カラーの画像)とホットスポットから画像を作り、CSS の `cursor: url(...) x y, <フォールバック>` で出す。画素は資源と同一にする。スケールしない・補間しない・ぼやけさせない。似た形の別のカーソル(十字・ペンの絵など)に置き換えない。

この Issue の目標は次の 3 つ。
1. Piano roll を開き、鉛筆(Draw)の道具にして、ノートの欄にポインタを置くと、ページの cursor が、FL の鉛筆のカーソル(FL の画素。資源の RT_CURSOR か、CreateIconIndirect で作られた物)から作った画像(`url(...)`)になる。
2. その画像の画素とホットスポットは、資源と一致する(PNG の画素の比較)。
3. 窓の端・標準のカーソル(段 1)の結果は、今のまま(回帰しない)。

## 前回の試行で分かったこと

- 前回: run 45516103909(判定は `failed`)。作業の commit は `1d6ef2cd`(20 ファイル)で、main には無い。差分は次の試行の作業ツリーに `.prior-work.md` と一緒に載るので、それを読んで続きから行う。
- 受け入れの結果: 起動確認は engine-idle・standIn false・elapsedMs 9150。`measure --pencil` は error null・pencilOk true・pencilPixelsMatch true・pencilMismatch 0・pencilHotspotMatch true・pencilInvertPixels 0。`welcome-flow` は 4 流れすべて opened true・elapsedMs 8925。`npm test --prefix app` は 1420 件通過。落ちたのは、判定式の `edgeWindows >= 1` だけ(観測は 0)。
- 直す前(HEAD `75aa9fe0`)の Piano roll のグリッドは 16 点すべて `default`(矢印)。直した後は `url(...)`。ノートの欄の SetCursor の handle は、直す前と後で同じ(GDI の領域・idc 0)。D キーを押しても変わらなかった。Draw の道具に切り替えたかは確かめていない。
- 鉛筆は、資源の RT_CURSOR ではなく、FL が `CreateIconIndirect` で作ったカーソル(資源の id が無い)。画素は 32×32。AND は全 0。XOR の黒(0,0,0)が 854 画素、非 0 が 170 画素。XOR の反転の画素は 0。
- 黒い箱の中に白い鉛筆が出た(PNG で確認)。不透明の画素は 1024 すべて。前回の実装は AND だけで透明を決めたので、背景が不透明になった。32bpp のアルファ(4 番目のバイト)を使う経路を足す(「資源のカーソルの画素の作り方」のアルファの規則)。
- 資源の RT_CURSOR は FL64.exe には無く、FLEngine_x64.dll(ImageBase 0x400000)に在った: RT_CURSOR 60 件、RT_GROUP_CURSOR 57 件。全てのグループは項目が 1 つ。RT_CURSOR は全て 32×32。名前が近い EEPEN(id 26)は、この run の grid では使われていない。
- 窓の端: `edgeWindows` 0 は HEAD でも同じ(同じ手順で測って 0)。FL の `WM_NCHITTEST` は端でも HTCLIENT を返すため。直した後の trace では、主窓の端の点(HTCLIENT)でも `SetCursor 0x7fba0580 idc=0` が設定され、ページの cursor は `url(...)` になった。直す前の同じ点は測っていない。作業者は、窓の中央の既定の cursor を直す前と後で比べ、報告に書く(段 1 の回帰の確認)。
- `textOk` と探索の食い違い: 途中の探索で I ビームが見つかった(F2 で開く名前の変更の窓の `TQuickEdit`、F11 で開いた窓の `TVectorSheet`)ので、`textOk` が true になった。最後の探索(663 点・25 子)は I ビームを見つけなかった。`measure.mjs` は途中の探索と最後の探索を OR で使い、最後の行だけ見ると矛盾して見える。
- DPR は 1 だけ測った(`setRoom` が deviceScaleFactor 1 に固定)。DPR 2 と乱数の DPR は、まだ測っていない。
- 未確認: Chrome の実際の描画(測ったのは style の値と PNG の画素だけ)。

## 背景

- 事実 1(今の経路): ゲストは FL 自身のカーソルを、`LoadCursorW(hInstance, name)` で読む。`lift/runtime/fle_cursor.c` の `fle_imp_LoadCursorW`(90〜100 行付近)は、hInstance が NULL でない時、弱い参照の `fle_cursor_load_named` に渡す。その実装は `lift/runtime/fle_res.c` の決定 0125 の経路(548 行付近。`lift/runtime/fle_res.h` の 50〜53 行の宣言)。読めたハンドルは「共有のハンドル」(`fle_cursor_is_shared_handle`)。
- 事実 2(表示の結果): `lift/runtime/fle_appview.h` の `cursor_shared` が 1 になる。`app/src/ui/cursor.ts` の `cursorCss` は、共有のハンドルを `default`(矢印)として返す(理由の文は「shown as the arrow」)。だから、Piano roll の鉛筆も、矢印で出ている。
- 事実 3(画素が page に届いていない): engine から page へ送るのは、ハンドルの値だけ(`fle_appview.h` の `cursor`・`cursor_idc`・`cursor_shared`)。資源の画素・ホットスポットは、page に届かない。
- 事実 4(資源の在り処): 起動用の実行ファイル(FL64.exe と FL64 (scaled).exe。ラベル fl-exe)は、`.github/workflows/agent.yml` の抽出(`tools/fl-installer-extract.mjs`)で `lift/out-app/host-extra/exe/` に置かれる(commit はしない)。この機械には、無いことがある。作業者は、Actions の作業の段で読む。前回の run では、RT_CURSOR は FL64.exe には無く、FLEngine_x64.dll に在った(前回の試行の節)。
- 事実 5(PE の資源を読む道具): `lift/crates/fe-pe/src/lib.rs` の `resources()`(616 行付近)と `resource_data()`(654 行付近)が、資源の木を読む。`lift/runtime/fle_res.c` は、RT_GROUP_CURSOR の NEWHEADER と RESDIR(404〜466 行付近)を読む。
- 事実 6(大きさの選び方): 1354 の調査(`pm-work/findings/dpi240-stop-1354.md`)で、SM_CXCURSOR は DPI で拡大する(96 DPI で 32、240 DPI で 80)。グループに複数の大きさが在る時、どの項目を選ぶかは `fle_res.c` の LoadCursorW の経路が決める。その選び方を、作業者が確かめる。
- 事実 7(Piano roll を開く): verify-kit の scenario(`tools/verify-kit/tools/scn-client.json` の `d10-pianoroll`)は、Sampler.flp を開いて F7 で Piano roll を開く。鉛筆(Draw)の道具を選ぶ手順は、作業者が Piano roll の表で確かめる。
- Chrome の cursor の仕様(作業者が実測で確かめる): cursor の画像は大きすぎると無視される(上限の寸法は実測)。画像の寸法の上限は、実測で確かめる(DPR の扱いは「資源のカーソルの画素の作り方」の最後の項)。

## 資源のカーソルの画素の作り方(PM の決定。作業者が Windows の仕様を Learn で確かめる)

- RT_GROUP_CURSOR の項目(RESDIR)が、RT_CURSOR の資源の id を指す。
- RT_CURSOR の資源は、先頭の 2 つの WORD がホットスポットの x・y。続きが DIB(BITMAPINFOHEADER。biHeight は画像の高さの 2 倍で、XOR のマスクと AND のマスクが続く)。
- FL が `CreateIconIndirect` で作ったカーソルは、id が無い。ICONINFO の hbmMask(AND のマスク)と hbmColor(カラーの画素)とホットスポットから、同じ規則で画素を作る。作られた時点の画素を、ゲストがビットマップを消す前に写し取る。
- 最初に、カラーのビットマップ(RT_CURSOR の XOR の DIB、または hbmColor)が 32bpp で、アルファ(4 番目のバイト)が 1 つでも非 0 なら、アルファを使い、AND のマスクは無視する(Windows の CreateIconIndirect の意味)。
- そうでなければ、次の AND と XOR の規則。AND = 0 の画素は、XOR の色で不透明。AND = 1 かつ XOR = 0 は透明。AND = 1 かつ XOR = 1 は画面の色を反転する画素で、CSS の画像では表せない。その画素の数を報告し、表し方を決める(例: 既定の色で描く)。表し方は、比較の参照と同じ規則で作る。
- 画像は、ホットスポットの位置を `cursor: url(...) x y` で渡す。DPR が 1 より大きい時は、`image-set` で、1x に資源の画素(原寸の 32×32)、2x に画素を最近傍で 2 倍に拡大した版(64×64。補間しない)を渡す。3x 以上は、最近傍で整数倍の版を同じ形で足す。

## 読む物

- 前回の差分: `lift/runtime/fle_cursor.h`(`fle_cursor_pic`・`FLE_CURSOR_PIC_MAX`)・`lift/runtime/fle_gdi.c` の CreateIconIndirect(作られた時点の画素の写し取り)。
- `lift/runtime/fle_cursor.c`(`fle_imp_LoadCursorW`・`fle_cursor_is_shared_handle`)・`lift/runtime/fle_cursor.h`
- `lift/runtime/fle_res.c`(404〜466 行の RT_GROUP_CURSOR の読み、548 行付近の LoadCursorW の経路)・`lift/runtime/fle_res.h`(50〜53 行)
- `lift/runtime/fle_appview.h`・`lift/runtime/fle_appview.c`(`cursor_shared`)・`app/src/engine/appview.ts`・`app/src/engine/os-state.ts`・`app/src/engine/protocol.ts`(page へ送る型)・`app/src/engine/worker.ts`・`app/src/engine/client.ts`・`app/src/main.ts`
- `app/src/ui/cursor.ts`(`cursorCss` の共有のハンドルの分岐)
- `lift/crates/fe-pe/src/lib.rs`(`resources()`・`resource_data()`)・`tools/fl-installer-extract.mjs`・`.github/workflows/agent.yml`(fl-exe の抽出。10 行付近の説明と、320 行付近から)
- `pm-work/findings/dpi240-stop-1354.md`(SM_CXCURSOR の拡大)
- `tools/verify-kit/tools/scn-client.json`(`d10-pianoroll`: F7 で Piano roll)・`tools/smoke/welcome-flow.mjs`
- 段 1 の道具 `tools/cursor/measure.mjs`(#{{139}} で作る。この Issue で、その道具を拡張する)

## やること

0. 現状の測定(直す前): Piano roll を開き、鉛筆の道具にして、ノートの欄にポインタを置く。その時の `LoadCursorW` の名前(または序数)と、ハンドル、ページの cursor の値を取る。FL の資源の一覧(RT_CURSOR の id と名前)を、Actions の作業の段で読んで、表にする(報告に書く)。
1. 資源の読み: `fle_res.c`(runtime)か、`lift/crates/fe-pe` か、既存の道具の、どちらかで RT_CURSOR を読み、ホットスポットと画素を取り出す。Chrome に渡す画像は、そこから作る(手で書いた絵・似た形の絵を使わない)。
2. engine から page へ、共有のハンドルの画像(ホットスポット・寸法・RGBA)を、一度だけ送る(ハンドルごと。送る経路は既存の `osstate` と並ぶ新しい通知。製品の既定の動作)。資源の RT_CURSOR の画像と、CreateIconIndirect で作られたカーソルの画像を、同じ経路で送る。page は、その画像から `cursor: url(...)` を作る。
3. `app/src/ui/cursor.ts` の `cursorCss`: 共有のハンドルの画像が届いている時は `url(...) x y, default` を返す。届いていない時は、理由を出す(`default` を、資源の画像の代わりに使ったと分かる形で)。
4. 測定の道具 `tools/cursor/measure.mjs`(#{{139}} で作った物)を拡張する。段 1 の項目は、そのまま出す。鉛筆の項目を足す(下の「受け入れ」)。DPR 2 と乱数の DPR は、`Emulation.setDeviceMetricsOverride` の deviceScaleFactor を変えて測る(前回の `setRoom` は 1 に固定していた)。
5. 報告に書く: 資源の一覧(id・名前・大きさ・ホットスポット)、鉛筆に使われた資源の id と名前、XOR の反転の画素の数と表し方、直す前と後の `pencil*` の値、`LoadCursorW` の大きさの選び方、`pencilDprs` の 3 点、`textFoundAt` と `textEnd`、窓の中央の既定の cursor の直す前と後。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/cursor/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --pencil > "$RUNNER_TEMP/cursor.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error===null&&Number.isInteger(r.edgeWindows)&&r.edgeWindows>=0&&r.edgeSamples===8*r.edgeWindows&&r.edgeMatches===r.edgeSamples&&r.textOk===true&&r.unresolved===0&&Array.isArray(r.unmappedNames)&&r.unmapped===r.unmappedNames.length&&r.pencilOk===true&&r.pencilPixelsMatch===true&&r.pencilHotspotMatch===true&&Array.isArray(r.pencilDprs)&&r.pencilDprs.length===3&&r.pencilDprs[0].dpr===1&&r.pencilDprs[0].ok===true&&r.pencilDprs[1].dpr===2&&r.pencilDprs[2].dpr>=1.01&&r.pencilDprs[2].dpr<=3&&r.pencilDprs[2].ok===true?0:1)' "$RUNNER_TEMP/cursor.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;const pb=Math.max(...L.map((r)=>r.preBootBytes));const el=Math.max(...L.map((r)=>r.elapsedMs));process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true&&el<=12500&&pb<=314385095?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app
```

判定の基準(`tools/cursor/measure.mjs --pencil` の JSON の項目。段 1 の項目は #{{139}} の判定の基準と同じ):
- `pencilOk`: true。Piano roll のノートの欄の上で、ページの cursor が `url(` で始まる(矢印の `default` ではない)。
- `pencilResource`: 資源の名前か id(文字列)。FL が CreateIconIndirect で作ったカーソルの時は `made by CreateIconIndirect`。
- `pencilSize`: [幅, 高さ]。資源の画像の寸法。
- `pencilHotspotMatch`: true。ページの cursor のホットスポットが、資源のホットスポットと一致する。
- `pencilPixelsMatch`: true。ページの画像を PNG として読んだ画素が、資源の画素(XOR の反転の表し方を含む)から作った参照と、完全に一致する。`pencilMismatch`: 一致しない画素の数(0 が正しい)。
- `pencilInvertPixels`: XOR の反転の画素の数(0 でもよい。数を出す)。
- `pencilDprs`: 3 点の配列。[0] は DPR 1、[1] は DPR 2、[2] は 1.01〜3.00 の乱数(小数第 2 位。JSON の `dpr` に出す)。各点は `dpr`・`ok`(Piano roll のノートの欄で、ページの cursor が `url(` で始まる)・`pixelsMatch`・`hotspotMatch`・`mismatch`。判定は [0] と [2] の `ok` が true。[1] は dpr が 2 であることだけを判定し、ok の値は報告に書く(判定には入れない)。
- 段 1 の項目(`edgeWindows`・`edgeSamples`・`edgeMatches`・`textOk`・`unresolved`・`unmapped`・`unmappedNames`)は、回帰しない(今のまま通る)。`edgeWindows` は 0 でもよい(数を作らない)。判定は `edgeMatches` と `edgeSamples` が等しいこと(0 と 0 でも通る)。
- `textOk`: テキストの欄の上で cursor が `text`。道具は、探索で I ビームが見つかった点(`textFoundAt`)があった時だけ true にする。最後の探索の結果は `textEnd`(真偽)として別に出す。`textOk` と `textEnd` が食い違う時は、その原因を報告に書く(前回は、途中の探索の結果を OR で使い、最後の行だけ「no I-beam」と出た)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 「直った」と書かない。上の判定の基準の結果で書く。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。
- 似た形のカーソルに当てはめない。画像は、FL の資源から作る。手で描いた絵・既存の CSS の絵を使わない。
- 画素を拡大・補間・ぼかしをしない。DPR に合わせて表示を変える時も、資源の画素を整数倍か原寸のままにする。
- Python は製品に使わない(調査と検証にだけ使う)。OCR は使わない。
- ベンダー名と Cloudflare の識別子を、ファイルに書かない。書いてよいもの: FL の製品名、資源の名前(FL64.exe・FLEngine_x64.dll・RT_CURSOR)、Image-Line のインストール先、変数名。禁止は、AI のベンダー名と Cloudflare の識別子だけ。
- sleep・timeout コマンド・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- 開始の時刻を、最初に `date -u` で記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える。
- engine の作り直しは約 35 分。1 回の試行で、ビルドは数回まで。資源の一覧と、鉛筆の呼び出しの記録は、1 回のビルドにまとめて取る。
- ビルドは、フォアグラウンドで、シェルツールの `timeout` 引数に 5400000 を渡して待つ。背景で起動して待たない。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: #{{139}}(段 1。同じ道具 `tools/cursor/measure.mjs` と、同じ `cursor.ts` の表を使う)。
- #{{140}}(上部バー)とは、直列にしない(別の経路)。ただし、page への通知の型(`protocol.ts`)が重なるので、衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
- 表示の大きさ(DPI で拡大した時の資源の選び方)は、#1354 と #1347 の拡大の作業と関係する。影響を報告に書く。
