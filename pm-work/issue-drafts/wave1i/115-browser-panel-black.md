---
draft: "115"
title: "Browser のパネル(左の黒い領域)を FL と同じにする: 一覧の窓の WM_PAINT が描画に至らない原因を直す"
type: Task
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: []
labels: ["resources", "render"]
size: L
priority: High
project: signature-studio/3
---

## 目的

依頼者の画面(本番)で、左の Browser のパネルが、ヘッダーの下から画面の下まで黒いまま。Packs・Current project・Plugin database・Channel presets などの一覧が出ない。FL の Browser は、起動後にフォルダを列挙して一覧を作ると推定される。

この Issue の目標は、DPR 1 の画面で、Browser のパネルの黒い割合を変更の前の半分以下にし、一覧の行が描かれること。併せて、起動前に取る量を増やさない。

## 前回までの試行で分かったこと

**依頼者の実機の観察(2026-11-23。画面は物理画素 3240×1863、DPR 1.7647、部屋は 1836×1056 CSS px)**
- 左の Browser のパネルの下の領域(ブラウザの一覧)が黒いまま。Channel rack・Playlist・Mixer の配置が崩れ、Mixer が左下に小さく出る。写真では、黒い領域は guest の y 約 144〜1052、x 0〜364 に当たる。

**前回の試行(run 45119988415。約 3 時間 20 分。受け入れに届かず `failed`)**
- 製品の変更はせず、道具・基準・切り分けの記録だけを commit した。判定で落ちたため push されず、作業は artifact `agent-1355` の `work.diff` に保存された。
- 次の試行は、取り込みの段で、この 3 ファイルを作業ツリーに載せる(取り込みの結果は `.prior-work.md` に書かれる)。3 ファイルが無ければ、前回の作業は無いものとして、基準から測り直す。
  - `tools/browser-panel/measure.mjs`(新規。DPR 1・部屋 1836×1056 CSS px で起動し、1 行の JSON を出す)
  - `pm-work/findings/browser-baseline-115.json`(基準。下の値)
  - `pm-work/findings/browser-panel-1355.md`(切り分けの記録)

**基準(変更の前。製品のビルド。runner の値。nproc 4、loadavg 1.48 1.21 1.67)**

| 項目 | 1 回目 | 2 回目 |
| --- | --- | --- |
| `blackShare` | 0.929158 | 0.929158 |
| `browserComponents` | 1 | 1 |
| `elapsedMs` | 7751 | 7797 |
| `preBootBytes` | 314,326,602 | 314,326,602 |

- 矩形は、部屋の原点から x 0・y 76・364×974 CSS px(道具の定義)。Browser の窓(`TWPControl`。下の窓の構造)の位置 abs (-5, 76)・364×974 と一致する。窓の表(`summary.windows`)には Browser が無いので、道具はこの定義を使う。
- 旧の既定の矩形(x 0・y 580・400×470)は、Browser の窓と合わなかった(下半分だけで、Playlist の 36 px を含む)。#{{107}} の前回の試行(run 45089194964)の `blackShare` 0.910(DPR 1・1.25・1.5・2 で 0.903〜0.914)は、この旧の矩形による値で、今の基準とは比べない。ただし DPR 1 でも黒いことは分かる(DPI の変更の前から在る)。
- 起動の確認: `{"error":null,"state":"engine-idle","standIn":false,"elapsedMs":8125}`。

**窓の構造と切り分け: 描かれていない(実測)**
- Browser の窓は `TWPControl`(lid 49、0x7f9900c4)。main form(`TFruityLoopsMainForm`、lid 14、0x7f990038、1846×1068)の子。中に `TSampleListForm`(lid 68、0x7f990110、364×974)があり、その中に一覧の窓 0x7f9901ac(364×905、abs (-5, 139))と、その子 0x7f9901b0(364×905)がある。
- 表示は欠けていない: 0x7f9900c4 は `SetWindowPos` の flags=0x57(SWP_SHOWWINDOW)で表示され、wndproc の style に WS_VISIBLE が立つ。0x7f990110 は `ShowWindow` の cmd=1。0x7f9901ac と 0x7f9901b0 も SWP_SHOWWINDOW で表示される。
- 列挙は件数を返している(`enum-dir:` の行): `Data\Patches\Packs\*` 7 件、`Data\Patches\Speech\*` 71 件、`Data\Patches\Mixer presets\*` 18 件、ユーザーの `Presets\Plugin database\*` 0 件。seed には `Data\Patches\` の下の階層が在る。
- 描画: 一覧の窓 0x7f9901ac は `WM_PAINT` を 1 回受けるが、`BeginPaint` を 1 回も呼ばれない。子 0x7f9901b0 も同じ。親 0x7f990110 は `BeginPaint` を 1 回(領域 0,0,364,974)受ける。

```
probe-wmpaint: 0x7f9901ac
probe-wmpaint: 0x7f9901b0
probe-wmpaint: 0x7f990110
probe-beginpaint: 0x7f990110 vis 1 upd 0,0,364,974 pc 364,974 in_paint 0
```

- 未確認: 一覧の窓の class と、`WM_PAINT` の配送先。`lift/runtime/fle_win.c` の既定の処理の `WM_PAINT`(`case FLE_WM_PAINT`)は、`BeginPaint` を呼ばず、更新領域を消すだけ("Nothing else is drawn")。配送の経路は trace の対象外で、前回の probe では確定できなかった。
- 候補(未確認。コードの読み): `fle_win.c` の `no_redraw`(`WM_SETREDRAW` の状態)。FALSE の窓と子は、空の可視領域で描かれて捨てられる。一覧の窓が `WM_SETREDRAW`(0)を受けて、再開(1)を受けていないかを確かめる。
- 除外: 表示の欠落は無い。列挙の欠落も無い(件数が返る)。seed への階層の追加は、描画が直った後、一覧の件数が FL と違う場合だけ行う(この切り分けからは、足す根拠は出ていない)。
- 前回の probe の変更は、製品に残していない(戻した。wasm に probe の文字列が無いことを確認した)。

**前回の作業で使った trace の経緯**
- `window.__app.windowApiTrace(true)` を起動の後に送っても、前回は `CreateWindowExW` の行が 0 件だった(起動後の有効化の経路が確認できなかった)。前回は、probe 用に `api_trace` の既定を 1 にしたビルドで、窓の作成・表示の記録を取った(製品には残していない)。
- WM_PAINT の probe の表示は 400 件で上限に達する(先頭 400 件のみ)。

**読む物**
- `tools/browser-panel/measure.mjs`・`pm-work/findings/browser-panel-1355.md`(前回の道具と記録。取り込まれた後で読む)
- `lift/runtime/fle_win.c`: `call_wndproc`、既定の処理の `WM_PAINT`・`WM_SETREDRAW`(`no_redraw`)、`ShowWindow`・`upd_show_descendants`・`apply_pos`(表示と更新領域)、`DispatchMessageW` の実装。`BeginPaint` の実装は `grep -rn "BeginPaint" lift/runtime` で探す。
- `app/src/engine/render-worker.ts`・`app/src/engine/desk.ts`(窓ごとのテクスチャと合成)、`app/src/engine/fruity.ts`(`windowApiTrace`)
- `lift/harness/seeds/`(seed の表・ファイルの seed。起動前の取得と `@lazy-file:` の仕組み)
- 起動後の取得の仕組み(`grep -rn "lazy-file" lift app/src` で探す)
- `tools/boot-check/cdp.mjs`・`tools/boot-check/boot-bundle.mjs`
- `pm-work/findings/c2-split-measure.md`(過去の測定の手順)
- `git log --grep 1345`(#{{105}} の配信の変更)

## やること

0. **基準**: `pm-work/findings/browser-baseline-115.json` は前回の作業に入っている(取り込み)。無ければ、何かを変える前に `tools/browser-panel/measure.mjs` で測り、上の基準の 1 回目の値と揃えてから commit する。矩形の定義(x 0・y 76・364×974)は変えない。
1. **切り分け: 完了**(前回の結果。上の「窓の構造と切り分け」)。残りを、次の順に行う。
   - (a) 取り込まれた作業ツリーを確かめる: `git status`・`git diff HEAD`。`.prior-work.md` と上の 3 ファイルが在るかを見る。
   - (b) 一覧の窓 0x7f9901ac の class と、`WM_PAINT` の配送先を特定する。`DispatchMessageW` → `call_wndproc` の経路に、名前付きの trace を足す(窓の class 名・配送先の手続きの名前・`WM_PAINT` が既定の処理に行ったか)。`WM_SETREDRAW` の遷移(`no_redraw`)も同じ trace に入れる。
   - (c) 原因を直す。原因が描画なら、runtime(`fle_win.c` の `WM_PAINT` の配送・既定の処理・`BeginPaint` を呼ぶ経路・TWPControl の子の窓のクリップと更新領域・`WM_SETREDRAW`)か、render Worker(`app/src/engine/`)を直す。**階層の追加と起動後の取得は、描画を直した後で、一覧の件数が FL と違う場合だけ行う**(seed には `Data\Patches\` の階層が在り、件数は返っている)。
   - (d) 直した後、`tools/browser-panel/measure.mjs` で DPR 1 を測り、受け入れの判定式で確かめる。
   - (e) probe の変更(trace の既定の有効化・表示の追加など)を戻す。直しの変更だけを残し、作業ツリーを commit する(受け入れが通らなくても)。
2. 矩形は、窓の表に Browser が出るようになったら(`summary.windows`)、その位置と比べる。ずれていれば、道具の定義を直し、直した内容を報告に書く。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/browser-panel/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/browser.json" && node -e 'const fs=require("fs");const r=JSON.parse(fs.readFileSync(process.argv[1],"utf8"));const b=JSON.parse(fs.readFileSync("pm-work/findings/browser-baseline-115.json","utf8"));const ok=r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.dpr===1&&Number.isFinite(r.blackShare)&&r.blackShare<=b.blackShare/2&&r.browserComponents>=10&&r.elapsedMs<=b.elapsedMs+500&&r.preBootBytes<=b.preBootBytes+1048576;process.exit(ok?0:1)' "$RUNNER_TEMP/browser.json" && npm test --prefix app
```

判定の基準(DPR 1 の 1 行。部屋 1836×1056 CSS px):
- 起動が正常(`error` が null・`state` が engine-idle・`standIn` が false)。
- `blackShare` が、`pm-work/findings/browser-baseline-115.json` の `blackShare` の半分以下。
- `browserComponents` が 10 以上(一覧の行が描かれている)。
- `elapsedMs` が、基準の `elapsedMs` に 500 ms を足した値以下。
- `preBootBytes` が、基準の `preBootBytes` に 1 MB(1,048,576 B)を足した値以下。
- 単体テスト(`npm test --prefix app`)が通る。

判定に使う基準の値は、`pm-work/findings/browser-baseline-115.json` の値(上の基準の 1 回目)。式の値(半分・10・500 ms・1 MB)は変えない。変更の後の値は、同じ道具で測り直す。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 基準の値(`blackShare`・`browserComponents`・`elapsedMs`・`preBootBytes`)と、変更の後の値、切り分けの結果(描かれていない/空の一覧を描いている)と、その根拠の行(`WM_PAINT`・`BeginPaint` の記録、`enum-dir:` の件数)、原因の窓の class と `WM_PAINT` の配送先、直した場所、矩形の定義を確かめた結果。耳の確認は行わない。「黒くない」とは書かず、割合と連結成分の数で書く。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- 特定の拡大率・DPI・DPR の値を入れない(この Issue は DPR 1 で測る)。
- 製品の既定の動作を、測定のためだけに変えない。probe の変更は、調査の後で戻してから commit する。
- 起動前に取る合計を、基準に 1 MB を足した値より大きくしない。大きいフォルダの中身は、起動後の取得にする。
- リポジトリの外に、ファイルを作らない。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- Python は製品に使わない(調査と検証にだけ使う)。

## 作業の進め方

- engine の作り直しは約 22〜30 分。1 回の試行で、ビルドは数回まで。**実験は、trace を複数まとめて入れた 1 回のビルドで行う**(前回は 1 つずつ試して、engine を 4 回作った)。
- ビルドは、フォアグラウンドで、シェルツールの `timeout` 引数に大きい値(例 3600000)を渡して待つ。
- blend2d の取得(GitHub)が 403 で落ちたら一時的。受け入れのコマンドは `lift/out-app/b2d-src` を消してから作り直すので、そのまま再試行する。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: なし。`#{{105}}`(閉じている)の後の main の版で行う。
- `#{{107}}`(物理画素の画面)と、`app/src` の描画の部分で、触る場所が重ならない形にする。重なれば、後に入る方が pull して直す。
- `#{{113}}`・`#{{114}}`・`#{{108}}` と、描画の経路(`app/src/engine/`)が重なりうる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
- 前回の道具(`tools/feat-c1-hidpi/measure-size.mjs`)は main に無い(`#{{107}}` の作業ツリーにある)。この Issue の道具は `tools/browser-panel/` に在る(前回の作業で作った。取り込まれる)。
