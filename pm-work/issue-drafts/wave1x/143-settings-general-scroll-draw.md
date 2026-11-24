---
draft: "143"
title: "F10 の Settings の General タブをスクロールすると、描画が崩壊する(症状の再現と修正)"
type: Task
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "005"
blocked_by: ["149"]
labels: ["render", "hidpi", "verify"]
size: M
priority: High
project: signature-studio/3
---

## 目的

依頼者の指摘(2026-11-24。原文): 「Settings の General タブは、スクロールすると描画が崩壊する症状を維持しています。」(以前から指摘されている症状が、直っていないという指摘)

この Issue の目標は、F10 の設定窓(Settings)の General 頁をスクロールした時に、描画が崩れない(同じ位置の画面が、どの経路で来ても同じ。再描画しても変わらない)こと。

## 背景

- 症状の記録: 以前の観察(`lift/runtime/fle_win.c` の、スクロールの模型の注釈。decision 0319)でも、同じ症状が記録されている: スクロールで絵が崩れ、マウスが乗った要素だけが直る。その模型は入った。依頼者の指摘は、それでも直っていないということ。
- 設定窓: F10 で開く窓(クラス `TMIDIForm`)。頁は帯(`TQuickSheetSelector`)で切り替える。頁の一覧は MIDI・Audio・General・Theme・Project(・Debug・Account・About。帯の幅で出る頁が変わる)(`pm-work/findings/hidpi-fl-scaling.md` の 10.1)。General 頁の見出しは窓の表に出る(10.1)。General 頁の項目は多く、スクロールで全体を見る。
- スクロールの経路(runtime。今の実装):
  - `lift/runtime/fle_win.c` 3629〜3800 行: `fle_win_scroll`(ビットと無効な領域の模型。`SW_SCROLLCHILDREN`・`SW_INVALIDATE`・`SW_ERASE` の旗を扱う)。
  - 同じ 3644〜3689 行: `scroll_copy_region`。ビットを移す。**面が無い・面が直接でない場合は、何もせずに返る**(3653 行と 3655 行)。その場合も、呼び出し元は動かなかった領域を無効にする(3710 行より後)。
  - `fle_win_scroll` は、次の場合に 0 を返す(呼び出し元の後備えに落ちる): 窓が msgonly または iconic(3716 行)、窓の可視領域が `fle_visrgn` の保持する矩形の数を超える(3747 行)。
  - 呼び出し元 `lift/runtime/fle_wall_ia2w.c`: `ScrollWindowEx`(185 行付近)は、0 の時に `scroll_window_ex_simple`(56 行〜)に落ち、そこで `SW_SCROLLCHILDREN` の要求は `fle_stop` になる(67 行)。`ScrollWindow`(221 行〜)は 0 の時に `fle_stop`(231 行)。
  - `ScrollDC`: `lift/runtime/fle_gdi.c` 2335〜2350 行。
  - 更新領域は、窓ごとに 1 つの外接矩形で持つ(`fle_win.c` の注釈「The update area of a window is one bounding rectangle」)。
  - 窓の面は、最上位の窓ごとに 1 つ(`fle_win.c` 2997 行付近の注釈)。ブラウザが、最上位の窓の面を合成する。
- 推定(未確認。下の再現で確かめる):
  - H1: 3653 行か 3655 行で、ビットが動かずに返る(面が直接でない、または道具の関数が無い)。すると、無効な領域だけが描き直され、残りは古い位置のまま残る(「マウスが乗った要素だけが直る」と合う)。
  - H2: 0 を返して後備えに落ち、後備えの経路で、子の窓の移動(`WM_MOVE`)か、ビットの移動が一部だけ行われる。
  - H3: 子の窓(項目)が、親の面と別に描かれ、親のスクロールで移動した子の領域が、次の描画で再び描かれない。
- 推測しない。どの H が当たるかは、再現の測定で決める。

## 前回の試行で分かったこと(2 回目)

- 2 回目の試行(run 45364914105。作業 #1384)は差し戻し。commit は作ったが、判定が通らず push していない。判定を通らなかった作業の差分は、次の試行の作業ツリーに載る(`.prior-work.md` が書かれる)。
- 受け入れの値(runner の値。この機械とは別枠。nproc 4): DPR 1 で、直す前の木の (a) `mismatchPath` は 0.5116、最後の木は 0.5484。(b) `mismatchRedraw` は最後の木で 0.6804(直す前は、強制の再描画の口が無いので測れなかった)。(c) `blankMax` は 0 で通る。`elapsedMs` 7229・`preBootBytes` 311110408 は通る。
- 見立て(実測): H1(ビットが動かずに返る)は外れた。`scroll_copy_region` は毎フレーム動く(`ScrollWindowEx bits: 1 rects, 2070360 bytes, moved by (0,-19)`)。H2(後備えに落ちる)も外れた。`fle_win_scroll` は毎フレーム答えている。H3 は一部当たり: 1 ノッチで、頁の窓(lid 21)に `ScrollWindowEx(dy, flags=0)` が約 11 回入る。その後、項目 9 件が `EndDeferWindowPos` で `flags=0x1d`(`SWP_NOCOPYBITS` が無い)で動く。項目の文字と枠は、スクロールの後に描き直されない。強制の再描画の画だけが正しく、これが基準(`pm-work/findings/settings-scroll-1384-dpr1-p50-redraw.png`)。
- 窓の作成の trace: 親の窓(頁)は `WS_CLIPCHILDREN` だが、スクロールされる窓 lid 21 は `cf=0x2`(`WS_CLIPCHILDREN` が無い)。
- 未確定: 子の `WM_PAINT` が届いているか、届いた時の clip が正しいか。trace に `WM_PAINT` の行が出ていない。
- 試して効かなかった物(戻した): (1) 動いた子を、親の枠に関わらず全体を無効にして描き直す → (a) 0.50 のまま。(2) 露出した帯を、フラグに関わらず消去付きで無効にする → (a) 0.50 のまま。(3) 一緒に運ばれた子の後の移動で、ビットを二重に運ばない + 全体を描き直す → (a) 0.52、(b) 0.65。(4) 入力を 1 つ入れてから画を取る → 数値は同じ(入力待ちで止まっているのではない)。
- 製品に入ったもの(測定だけが呼ぶ。既定の動作は変わらない): `window.__app.settingsRedraw(lid)` → Worker の `settings-redraw` → engine の `fle_g3_redraw_tree` → `fle_win_redraw_tree`。engine のログの一時の記録は戻した。
- 測定の道具 `tools/settings-scroll/measure.mjs`: (a) はホイールでは位置が揃わない(頁の範囲 M が 96 の倍数にならない)ため、つまみのドラッグの位置で揃える。ホイールは (c) の材料だけに使う。(b) は下から来た時の画で計る。乱数の DPR は 1.01〜2.25 にする(2.5 以上は起動が止まる。#{{149}} で扱う)。
- ホイールは DPR ≠ 1 で頁を動かさない(M = 0)。下の次にやること 5 で扱う。
- 起動の間欠の停止(#{{149}}): 同じ bundle で、流れ A〜D のうち 2 本が `no engine-idle within 180000 ms (state running)` で失敗した(失敗する起動は毎回違った)。前回の試行の別の回では 4 本とも通った。乱数 DPR の boot も、180 秒で engine-idle に届かない回が 2 回あった。

## 読む物

- `lift/runtime/fle_win.c`(3629〜3800 行: 模型と `scroll_copy_region`。3200 行付近の類似の経路。`fle_imp_InvalidateRect`・`fle_imp_UpdateWindow`・`fle_imp_RedrawWindow`)
- `lift/runtime/fle_wall_ia2w.c`(56〜240 行: `ScrollWindowEx`・`ScrollWindow` の呼び出し元と後備え)
- `lift/runtime/fle_gdi.c`(2335〜2350 行: `ScrollDC`)、`lift/runtime/fle_raster.c`(818 行: `fle_raster_wrote`。面の書いた範囲の記録)
- `app/src/engine/desk.ts`・`app/src/engine/desk-gl.ts`(最上位の窓の面を合成して提示する経路。`stats.presents`)、`app/src/engine/worker.ts`(`desk-peek`・`guest-text` の要求の `case`)
- `tools/feat-c1-hidpi/measure-size.mjs` の `settingsPhase`(F10 で設定窓を開き、頁を押す手順。#{{117}})、`tools/smoke/welcome-flow.mjs`(起動と、Welcome 窓を閉じる手順)
- `pm-work/findings/hidpi-fl-scaling.md` の 10.1〜10.2(設定窓の頁・Debug の頁)

## やること

0. 作業ツリーの確認。`.prior-work.md` があれば最初に読む。
1. 再現の道具 `tools/settings-scroll/measure.mjs` を使う(前回の試行で作った。作業ツリーに差分として載る。無ければ新規に作る)。
   - 起動し、Welcome 窓と確認ダイアログを閉じる(`tools/smoke/welcome-flow.mjs` の手順)。F10 で設定窓を開き、General 頁を押す(頁の位置は窓の表の帯から取る。固定の座標を使わない)。
   - スクロールする経路は 2 つ: (a) 頁の上でホイール(窓の表の頁の矩形の中心)、(b) 右のスクロールバーのつまみのドラッグ。どちらも、上端から下端まで、下端から上端まで行う。
   - 同じスクロール位置の画面を取る。位置は、スクロールの範囲(上端と下端の差)の 0%・25%・50%・75%・100% の 5 点。画面は `window.__app.deskDumpBase64(true)` の RGBA を、設定窓の矩形で切り出す。
   - 比較 (a): 上端から来た時と、下端から来た時の、同じ位置の画面の画素の不一致の割合。
   - 比較 (b): 各位置で、スクロールのまま取った画面と、窓の全体を強制的に描き直した後の画面の不一致の割合。強制の再描画は、測定の道具だけが呼ぶ要求(`window.__app` の関数 1 つと、Worker の `case` 1 つ。`guest-text` と同じ経路。中身は、設定窓の全ての窓を `RedrawWindow` で全域を無効にして描き直す)。既定の動作では呼ばれない。
   - (c): スクロールの途中で、窓の面積の中で、黒い・未描画の領域(画素が既定の背景色ではない・全部黒の矩形)の割合の最大。
   - 1 行の JSON(`dpr`・`mismatchPath`・`mismatchRedraw`・`blankMax`・`error`・`state`・`standIn`・`elapsedMs`・`preBootBytes`)を stdout に出す。
   - 画像は、`pm-work/findings/settings-scroll-<番号>.png` に、ずれの大きい位置の物だけを残す(数枚。小さいもの)。
2. 原因の特定。H1〜H3 のどれかを、道具の出力と、engine のログ(`fle_win_scroll`・`scroll_copy_region` の早い戻りの記録。測定の時だけ出す一時の記録。報告の後で消す)で確かめる。どの経路が、どの位置の画面を作ったかを報告に書く。
3. 直す。既定の動作として直す(オプトインを作らない)。模型の早い戻りを、面が直接でない時にも正しく動かす。後備えに落ちる時は、後備えの答えが、模型と同じ移動になるように直す。
4. 道具の結果が、受け入れの判定を満たすことを確かめる(DPR 1 と、乱数の DPR)。

## 次にやること(2 回目の試行の後)

目標(受け入れの (a)(b)(c))は変えない。次の順に直す。

1. `SetWindowPos`・`DeferWindowPos`・`EndDeferWindowPos` で `SWP_NOCOPYBITS` が無い時の Windows の挙動を、`lift/runtime/fle_win.c` に実装する: 子の窓のクライアントのビット(その窓の面)を、新しい位置へ複写し、露出した部分と、複写の元の位置に重なる親の領域を無効にする。
2. `ScrollWindowEx` が `flags=0` の時は、露出した領域を無効にしない(Windows と同じ。無効にするのは呼び出し側の FL)。`SW_INVALIDATE`・`SW_SCROLLCHILDREN`・`SW_ERASE` の旗ごとの動作を、Windows の公開の文書に合わせる。
3. 子の `WM_PAINT` が届くか、届いた時の clip が正しいかを、trace で確かめる。trace は engine のログに出す(測定の時だけ。環境変数・`#ifdef` を足さない。報告の後で消す)。
4. 窓 lid 21 の `WS_CLIPCHILDREN` の有無が、描画に効くかを確かめる。
5. DPR ≠ 1 でホイールが頁を動かさない件を確かめる。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/settings-scroll/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/scroll.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const ok=(r)=>r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.mismatchPath<=0.001&&r.mismatchRedraw<=0.001&&r.blankMax<=0.01&&r.elapsedMs<=12500&&r.preBootBytes<=314368711;process.exit(L.length===2&&L.every(ok)?0:1)' "$RUNNER_TEMP/scroll.jsonl" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app
```

判定の基準(道具の出力は 2 行: DPR 1 と乱数の DPR 1.01〜2.25。2.5 以上は #{{149}} が直るまで使わない):
- (a) `mismatchPath` が 0.001(0.1%)以下。上端から来た時と下端から来た時の、同じ 5 点の画面の画素の不一致の割合。
- (b) `mismatchRedraw` が 0.001(0.1%)以下。スクロールのまま取った画面と、強制の再描画の後の画面の不一致の割合。
- (c) `blankMax` が 0.01(1%)以下。スクロール中に、黒い・未描画の領域が窓の面積に占める割合の最大。
- `error` が null、`state` が engine-idle、`standIn` が false。`elapsedMs` が 12,500 以下。`preBootBytes` が 314,368,711 以下。
- 流れ A〜D(`tools/smoke/welcome-flow.mjs`)が通る。単体テスト(`npm test --prefix app`)が通る。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 直す前後の (a)(b)(c) の値、H1〜H3 のどれが当たったか(根拠の出力)、直した箇所と理由、ずれの大きい位置の画像の場所。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。強制の再描画の要求は、測定の道具だけが呼ぶ(既定の動作を変えない)。
- 特定の拡大率・DPI・DPR の値(125・150・168・169・175・176 など)を、runtime・app・テストに、定数・表・分岐として入れない。
- ゲストのメモリに書き込まない(読むだけ)。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- Python は製品に使わない(調査と検証にだけ使う)。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- 最初に `date -u` で開始の時刻を記録する。4 時間 30 分を過ぎたら、受け入れが通っていなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- ビルドは約 35 分。フォアグラウンドで直接、シェルツールの `timeout` 引数は最大 5400000 を渡して待つ。背景で起動して待たない。4 時間 30 分で、受け入れが通らなくても、commit して終える。
- 1 回の試行で、ビルドは数回まで。道具・記録・直しは、まとめて 1 回のビルドで行う。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- #{{117}}(F10 の設定窓の読み)の道具の窓の操作を使う。#{{117}} の判定は変えない。
- #{{110}}(P2 の測定の道具)と #{{111}}(描画の所要が変化した面積に比例する)は、描画の経路が重なりうる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
- #{{108}}(動的な変化と全ての拡大率)は、この Issue の後に行う。
- #{{149}}(起動が first idle に届かない。間欠の停止)を `blocked_by` に足した。この Issue の受け入れの起動確認・流れ A〜D・乱数 DPR の起動は、#{{149}} の間欠の停止で落ちるため。#{{149}} が閉じるまで、この Issue は起動しない。
