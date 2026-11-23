---
draft: "114"
title: "DPI 240 以上(DPR 2.5 以上)で FL が起動しない障害を直す: LOGPIXELSY の値で起きる再帰の閉路を特定し、runtime の応答を Windows と同じにする"
type: Task
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "005"
blocked_by: ["107", "040"]
labels: ["hidpi", "render"]
size: L
priority: High
project: signature-studio/3
---

## 目的

依頼者の要求は、100%〜300% の全ての拡大率で FL が操作できて、ぼやけないこと(`#{{108}}` が全ての拡大率を検証する)。ところが、DPI が 240 以上(DPR 2.5 以上)で、FL が engine-idle に届かず、起動しない。この Issue は、その障害を直す。

この Issue の目標は、DPR 2.5 と 3.0 と、乱数 2 つ(2.25〜3.00)の全てで engine-idle に届くこと。DPR 1・1.7647・2 の結果は変わらないこと。

## 前回までの試行で分かったこと

- 停止の表示: `[js] fle thread 2 failed: RangeError: Maximum call stack size exceeded`。DPR 2.5(DPI 240)・3.0・乱数 2.71(DPI 260)で起きる。DPR 2.25(DPI 216)と 2.09(DPI 201)は engine-idle に届く(#{{107}} の前回の表)。
- 原因の絞り込み(前回の試行の一時的な実験。`FLE_EXP_*` の切り替え。今は外されている):
  - LOGPIXELSY の値だけを 96 に戻すと、DPR 2.5 が engine-idle に届く。LOGPIXELSY が 240 のとき止まる。
  - 原因でないと確認したもの: フォントの高さ単独(`FLE_SPI_FONT_HEIGHT`。DPI から -30 になる)、`GetDpiForMonitor` 単独、物理サイズの仮説(機器の密度から求めた物理サイズでも止まる)。
- 停止は窓の作成(`CreateWindowExW`、style `0x…24`)の直後。停止時のスタックに、同じ 12 個の戻り番地の並びが繰り返し、自己呼び出しを含む。
- guest が LOGPIXELSY(90) を読む呼び出し元は 5 か所。LOGPIXELSY の値は guest の大域変数(`0x630e924`)に保存され、その読み出しは 7 か所。読み出しを含む関数の直接の呼び出しを辿ると、そこから先は 97 関数で、直接の呼び出しの閉路は無い。よって閉路は、関数ポインタか runtime を経由する呼び出しで起きている(推定)。
- 戻り番地の関数名への対応は未確定。リンクマップ(flengine6 向け)は、最終の engine.wasm の関数番号と一致しない。関数名の表は `#{{040}}` が出す(`lift/out-app/gen/wasm-func-names.tsv`)。
- 前回の一時的な診断(停止時のスタックの表示)は外されている。作業者が、同じ趣旨の記録を一時的に入れてよい。
- 前回の差分の DPI の式: `lift/runtime/fle_screen.h` の `fle_screen_dpi`(MulDiv(96, 拡大率, 100))。`SM_CXCURSOR` は DPR 2.25 で 72、DPR 2.5 で 80 と MulDiv どおり。

## 読む物

- `lift/out-app/gen/wasm-func-names.tsv`(`#{{040}}` が出す関数名の表。製品のビルドで書き出す)
- `lift/runtime/fle_win.c`(`CreateWindowExW`・窓の大きさ・`WM_GETMINMAXINFO`・`WM_SIZE`・`SetWindowPos` の応答)
- `lift/runtime/fle_sysmetrics.c`・`fle_gdc.c`・`fle_appfile.c`(`AdjustWindowRectExForDpi`・`GetSystemMetricsForDpi`・`GetDpiForMonitor`・`GetDeviceCaps` の応答)
- `lift/runtime/fle_screen.h`(`fle_screen_dpi`・`fle_screen_scale_percent`)
- `lift/runtime/fle_thread.c`・`fle_thread.h`・`lift/harness/web/threads/fle_thread_lib.js`(guest スレッドの失敗の表示)
- `tools/feat-c1-hidpi/measure-size.mjs`・`tools/feat-c1-hidpi/read-manifest.mjs`(#{{107}} の成果物)

## やること

1. 再現と記録: DPR 2.5 で起動し、停止時の guest のスタックの戻り番地を取る。戻り番地を `#{{040}}` の関数名の表で関数名に戻す。表に無い番地は、生成物の対応(`gen/` の C のソース)で近い関数を探す。閉路の入口(最初に現れる関数)を特定する。
2. 閉路を導く応答の特定: 閉路の関数が読んでいる応答(LOGPIXELSY・窓の大きさ・最小のサイズ・画面の大きさ・カーソルとアイコンの寸法)のうち、どれが再帰の深さを決めているかを、runtime の関数ごとに特定する。候補: `AdjustWindowRectExForDpi`・`GetSystemMetricsForDpi`・`GetSystemMetrics`・窓の最小サイズ・`WM_GETMINMAXINFO`・`WM_SIZE`・`SetWindowPos` の結果・`SM_CXSCREEN`。
3. Windows の仕様と照らす: 2 の応答が、Windows の公開の仕様(`GetSystemMetricsForDpi`・`AdjustWindowRectExForDpi`・`MulDiv` の文書)と同じ値・同じ順序かを確かめる。違う応答を直す。
4. 直す: 応答を Windows と同じ値・順序にする。特定の値(240・260・2.5 など)で分岐しない。値は `fle_screen_dpi`(`MulDiv`)から式で導く。
5. 閉路が FL の仕様(FL の窓の大きさの計算。Windows と同じ応答でも収束しない)によると分かった場合は、その事実を、閉路の関数名と一緒に報告に書く。その場合も、runtime の応答が Windows と同じであることを確かめる。
6. 直した後、DPR 1・1.7647・2 の結果が変わらないことを確かめる(下の受け入れ)。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/feat-c1-hidpi/measure-size.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --dprs 1,1.7647,2,2.5,3 --random 2 --random-range 2.25,3 > "$RUNNER_TEMP/size.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const R=L.filter((x)=>"dpr" in x);const eq=(x,y)=>Math.abs(x-y)<0.0001;const v=R.length?R[0].randomDprs.values:[];const want=[1,1.7647,2,2.5,3,...v];const ok=R.length===7&&v.length===2&&v.every((d)=>d>=2.25&&d<=3)&&want.every((d)=>R.some((x)=>eq(x.dpr,d)))&&R.every((x)=>x.error===null&&x.state==="engine-idle"&&x.standIn===false&&Math.abs(x.guest.w-Math.round(x.room.w*x.dpr))<=1&&Math.abs(x.guest.h-Math.round(x.room.h*x.dpr))<=1&&Math.abs(x.dpi-Math.round(96*x.dpr))<=1&&Math.abs(x.canvas.w-x.display.w)<=1&&Math.abs(x.canvas.h-x.display.h)<=1);process.exit(ok?0:1)' "$RUNNER_TEMP/size.jsonl" && ! git diff -U0 refs/judge/main -- lift/runtime lift/harness app/src | grep -E '^\+[^+]' | grep -qE '\b(125|150|168|169|175|176)\b|1\.75\b|1\.7647|0\.5667' && npm test --prefix app
```

判定の基準: 道具の 7 行(DPR ごとの行)の生の値を、上の node -e が読む。
- (a) 固定の一覧(1・1.7647・2・2.5・3)と乱数 2 つ(2.25〜3.00)が全て在る。全行が正常な起動(`error` が null・`state` が engine-idle・`standIn` が false)。DPR 2.5 と 3.0 の行が engine-idle に届く。
- (b) 全行で、guest の w と h が、部屋 × DPR の round の ±1 画素。dpi が round(96 × DPR) の ±1。canvas と display が ±1 画素で一致する(#{{107}} の受け入れの (i)〜(iii) と同じ式)。DPR 1・1.7647・2 の行も、この式を満たす(結果が変わらない)。
- (c) 乱数の DPR は、作業者の実行と判定の実行で別になる。固定の値に合わせた実装は通らない。
- ソースの検査(受け入れの最後の段): `lift/runtime`・`lift/harness`・`app/src` の、この変更で足した行に、特定の拡大率・DPI・DPR の値(`125`・`150`・`168`・`169`・`175`・`176`・`1.75`・`1.7647`・`0.5667`)が、拡大率・DPI・DPR の意味で現れないこと。一致が 0 件であること。
- 単体テスト(`npm test --prefix app`)が通ること。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 停止時の戻り番地と、関数名への対応(表で引けた物と、引けなかった物の数)、閉路の入口の関数名、閉路を導いた応答(関数と戻り値)、Windows の仕様との照合(文書の名前)、直した箇所(ファイルと行)、DPR ごとの表(状態・guest・dpi・canvas と display)、DPR 1・1.7647・2 の結果が変わらないことの確認、特定の値を用意していないことの確認(source の検査の grep の出力。一致 0 件)。閉路の入口が特定できなければ「未特定」と書く。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- 特定の DPI・DPR の値(240・260・2.5 など)で分岐しない。Windows の式で導く値だけを使う。
- 閉路を隠すだけの回避(再帰の深さの上限を置く、窓の大きさを固定の値にする、DPR を丸めて別の値にする など)で代えない。閉路の原因を直す。
- 拡大して表示する版を作らない。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- Python は製品に使わない(調査と検証にだけ使う)。

## 作業の進め方

- engine の作り直しは約 30 分。ビルドは、フォアグラウンドで、シェルツールの `timeout` 引数に大きい値(例 3600000)を渡して待つ。
- 1 回の試行で、ビルドは数回までなので、実験はビルドをまとめて計画する(前回の試行は、ビルドを 5 回使った)。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: #{{107}}(測定の道具と、DPI の式)、#{{040}}(関数名の表。`#1334` が出す)。
- `#{{113}}`(FL の UI の拡大の実測)と、窓の経路(`fle_win.c`)を変えうる。衝突したら pull して直す。
- `#{{108}}`(動的な変化と全ての拡大率)は、この Issue の後に行う。
- #1349(音が途切れる現象)は、この Issue の後の物理画素の画面で測る。
