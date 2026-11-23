---
draft: "113"
title: "FL の UI の拡大を、FL 自身の情報で測る: F10 の Display の節・Debug ログ・DPI の API の全呼び出し・採用した拡大率・部品の位置・縁の遷移の幅"
type: Task
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "005"
blocked_by: ["107"]
labels: ["hidpi", "verify", "fl-exe"]
size: L
priority: Urgent
project: signature-studio/3
---

## 目的

#{{107}} で、guest の画面・DPI・canvas は物理画素で揃った。しかし、FL が**自分で UI を拡大して描いているか**は、まだ実測していない。依頼者の是正の核心は、Main GUI Scaling の System が OS の拡大率に自動で従うことである。

この Issue の目標は、FL 自身の情報(F10 の Display の節・Debug ログ・DPI の API の呼び出し・UI の部品の大きさ)から、FL が採用した拡大率・FL が見た DPI・FL の設定の読みを測る道具を入れ、その値を Windows の計算と突き合わせることである。拡大率は runtime が決めて渡すのではなく、FL が読んだ値から FL が決める。

## 前回までの試行で分かったこと

- #{{107}} の前回の試行(run 45089194964)では、`flScalingPercent`・`mainGuiScaling`・`parts`・`edgeWidth`・`debugLog` が全て null(測れていない)。
- F10 の Display の節・Debug ログの窓の操作・窓の文字を読む経路は、まだ作っていない。
- Main GUI Scaling の設定の読みは、engine の記録(trace)に出ていない。seed に GUI の拡大の値は無い(`reg-seed-app.tsv`・`reg-seed-nsis.tsv` に無い)。
- 窓の表(`fle_desk_describe`。アプリのデバッグパネルの `Guest top-level windows`)は最上位の窓だけで、トップバーの部品(Tempo・Song Position・Pattern Selector)は出ない。部品の位置は、C-6 の道具が持つ DPR 1 の CSS の座標を基準にできる。
- DPI 関係の API の記録(`window-api:` の行)は、前回の差分で起動の前から有効にできる(`app/src/main.ts`・`app/src/engine/client.ts`・`app/src/engine/fruity.ts`。既定では出ない)。
- guest が読むのは LOGPIXELSY(90) だけで、LOGPIXELSX(88) は読まれない。DPR 1・1.25・1.5・1.7647・2 で LOGPIXELSY は 96・120・144・169・192。カーソル・小アイコンの寸法は MulDiv どおり(DPR 2.25 で SM_CXCURSOR 72、SM_CXSMICON 36)。
- DPR 2.5 以上は FL が起動しない(`#{{114}}` が直す)。この Issue の測定は DPR 2.25 以下で行う。
- マニフェスト: `FL64.exe` は `dpiAware True/PM`。`FL64 (scaled).exe` は宣言が無い(推定: Windows が拡大する起動用)。

## 読む物

- `tools/feat-c1-hidpi/measure-size.mjs`・`tools/feat-c1-hidpi/read-manifest.mjs`(#{{107}} の成果物)
- `lift/runtime/fle_win.c`(窓の API の記録)、`lift/runtime/fle_gdc.c`・`fle_sysmetrics.c`・`fle_appfile.c`・`fle_proc.c`(DPI と寸法の応答)、`lift/runtime/fle_screen.h`(拡大率と DPI の式)
- `app/src/main.ts`・`app/src/engine/client.ts`・`app/src/engine/fruity.ts`(起動前の記録)
- `app/src/ui/screen-fit.ts`・`app/src/ui/desktop.ts`
- `lift/harness/seeds/`(seed の表・レジストリの seed・設定ファイルの seed)
- `tools/boot-check/cdp.mjs`
- `pm-work/findings/hidpi-fl-scaling.md`(#{{107}} の資料)

## やること

1. 窓の操作と、窓の文字を読む経路を作る。
   - F10 で General settings を開き、タブ(MIDI・Audio・General・File・Theme・Project・Info・Debug・Account・About)を切り替えられるようにする。座標は、窓の表(`window.__app` の窓の表)と、部品の位置から取る。
   - 窓の文字を読む経路を 1 つ選び、実測で確かめる: (i) 標準のテキストの部品(EDIT・RichEdit)なら `WM_GETTEXT`・`GetWindowTextW`・`EM_GETLINE` の読み。(ii) FL 自作の描画なら `ExtTextOutW`・`DrawTextW`・`TextOutW` の文字列の記録(測定の道具が有効にした時だけ記録する)。(iii) スクリーンショットの OCR は使わない。どの方式で取れたかを、理由と一緒に報告に書く。
2. F10 の Display の節・Info タブ・Debug タブを取る。
   - General タブの Display の節: Main GUI Scaling・Pop-ups・Toolbars・Cursors・Legacy scaling の選択値を、読める形で報告に書く。
   - Debug タブ(`Settings - Debugging log`): 全文の行数を数え、先頭と末尾の数十行と、絞った行(`grep -i 'dpi\|scal\|screen\|monitor\|display\|resol\|zoom\|window\|error\|warn'` で絞った全行)を `pm-work/findings/hidpi-fl-scaling.md` に載せる。長い全文は載せない。
   - Info タブ: FL・OS・画面の情報が出るなら、その全文の要点を載せる。
3. DPI・画面・システムメトリクスに関係する呼び出しを、起動から idle まで記録する。前回の差分の窓の API の記録(起動の前から有効にする経路)を使い、既定では出ない形にする。FL の Main GUI Scaling の読み(レジストリ・設定ファイル)が記録に出るかを確かめる。出なければ、FL がどこから読むかを API の記録から探し、探した経路を報告に書く。
4. FL が採用した拡大率(`flScalingPercent`)を、DPR 1・1.25・1.5・1.7647・2 と乱数 2 つ(1.00〜2.25)で測る。
   - 求め方: UI の部品(Tempo の部品)の幅の画素数 ÷ DPI 96 の時の幅の画素数。FL が段に丸める場合は、段の値と境界を特定して書く。
   - FL が採用する拡大率が round(100 × DPR) に従うかを確かめる。
   - 従わない場合: FL が何を見ているかを、API の記録・Debug ログ・設定の読みから特定し、runtime の戻り値を Windows と同じにして直す。特定の値を用意しない。拡大率は、FL が読んだ値から FL が決める。
5. 部品の位置(`parts`)・縁の遷移の幅(`edgeWidth`)・ポインタの座標の変換(`pointerErrPx`)を測る。
   - 部品の位置: 窓の表に出ない部品は、FL の窓の手続きの呼び出し、または画像から求め、DPR 1 の CSS 座標の基準と突き合わせる。
   - 縁の遷移の幅: スクリーンショットの、Tempo の部品の枠の縁の遷移の幅(画素。10%〜90% の遷移の、水平と垂直の中央値)。FL が自分で描いた縁は 2 画素以下。
   - ポインタの座標の変換: 主窓の部品の中心を 3 点押し、engine が受けた位置(物理画素)と、期待する物理画素の位置(`floor(clientX × DPR)` と同じ)との差の最大値(画素)を `pointerErrPx` に出す。
6. 測定の道具 `tools/feat-c1-hidpi/measure-size.mjs` に、`flScalingPercent`・`mainGuiScaling`(FL の設定の読みの値。記録が無ければ null)・`parts`・`edgeWidth`・`debugLog`(`{"captured","lines","method","scalingLines"}`)・`pointerErrPx` を出す。取れない項目は null のままにし、理由を報告に書く。
7. 推測を事実として書かない。ログに出た行と、記録された API の戻り値だけを根拠にする。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/feat-c1-hidpi/measure-size.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/size.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const R=L.filter((x)=>"dpr" in x);const P=["tempo","song-position","pattern-selector"];const eq=(x,y)=>Math.abs(x-y)<0.0001;const v=R.length?R[0].randomDprs.values:[];const want=[1,1.25,1.5,1.7647,2,...v];const ok=R.length===7&&v.length===2&&v.every((d)=>d>=1&&d<=2.25)&&want.every((d)=>R.some((x)=>eq(x.dpr,d)))&&R.every((x)=>x.error===null&&x.state==="engine-idle"&&x.standIn===false&&Number.isFinite(x.flScalingPercent)&&Math.abs(x.flScalingPercent-Math.round(100*x.dpr))<=3&&x.mainGuiScaling!==null&&Number.isFinite(x.edgeWidth)&&x.edgeWidth<=2&&x.debugLog.captured===true&&x.debugLog.lines>=1&&Number.isFinite(x.pointerErrPx)&&x.pointerErrPx<=1&&P.every((p)=>Number.isFinite(x.parts[p].x)&&Number.isFinite(x.parts[p].y)));process.exit(ok?0:1)' "$RUNNER_TEMP/size.jsonl" && ! git diff -U0 refs/judge/main -- lift/runtime lift/harness app/src | grep -E '^\+[^+]' | grep -qE '\b(125|150|168|169|175|176)\b|1\.75\b|1\.7647|0\.5667' && npm test --prefix app
```

判定の基準: 道具の 7 行(DPR ごとの行)の生の値を、上の node -e が読む。
- (a) 固定の一覧(1・1.25・1.5・1.7647・2)と乱数 2 つ(1.00〜2.25)が全て在る。全行が正常な起動(`error` が null・`state` が engine-idle・`standIn` が false)。
- (b) `flScalingPercent` が round(100 × DPR) の ±3。FL が段に丸める場合は、丸めた後の値を入れ、丸め方を報告に書く。
- (c) `mainGuiScaling` が null でない(FL の設定の読みが記録に出た)。値そのもの(System か否か)は報告に書く。
- (d) `edgeWidth` が 2 以下。`debugLog.captured` が true で、`lines` が 1 以上。
- (e) `parts` の 3 部品が全て座標を持つ。`pointerErrPx` が 1 以下。
- (f) 乱数の DPR は、作業者の実行と判定の実行で別になる。固定の値に合わせた実装は通らない。期待値は、各行の DPR から式で計算する。
- ソースの検査(受け入れの最後の段): `lift/runtime`・`lift/harness`・`app/src` の、この変更で足した行に、特定の拡大率・DPI・DPR の値(`125`・`150`・`168`・`169`・`175`・`176`・`1.75`・`1.7647`・`0.5667`)が、拡大率・DPI・DPR の意味で現れないこと。一致が 0 件であること。
- 単体テスト(`npm test --prefix app`)が通ること。
- 拡大率が round(100 × DPR) と違った場合、この受け入れは通らない。その時は、FL が何を見ているかの特定と、直した内容を報告に書く(直せなければ、通らない)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: F10 の Display の選択値、窓の文字を読んだ方式と理由、Debug ログの行数と絞った行、Info の要点、DPI 関係の API の記録(呼び出し元と戻り値)、Main GUI Scaling の読みの経路、DPR ごとの表(`flScalingPercent`・丸め方・DPI・部品の座標・`edgeWidth`・`pointerErrPx`・Debug ログの captured と lines)、拡大率が round(100 × DPR) と違った場合の原因と直した内容、特定の値を用意していないことの確認(source の検査の grep の出力。一致 0 件)。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- 特定の拡大率・DPI・DPR の値を、runtime・app・seed・設定・テストに、定数・表・分岐・seed の値として入れない。
- 拡大して表示する版を作らない。
- 窓の操作・窓の文字を読む経路・DPI の記録は、測定の道具だけが有効にする。既定の動作(利用者の画面)を変えない。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- Python は製品に使わない(調査と検証にだけ使う)。

## 作業の進め方

- engine の作り直しは約 30 分。ビルドは、フォアグラウンドで、シェルツールの `timeout` 引数に大きい値(例 3600000)を渡して待つ。
- 1 回の試行(最大約 5.5 時間)で、ビルドは数回までなので、実験はビルドをまとめて計画する。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: #{{107}}(物理画素の画面と、DPI の式。測定の道具の土台)。
- `#{{114}}`(DPI 240 以上の停止)とは別。この Issue の測定は DPR 2.25 以下で行う。
- `#{{115}}`(Browser のパネル)と、描画の経路(`app/src/engine/`)が重なりうる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
- `#{{108}}`(動的な変化と全ての拡大率)は、この Issue と `#{{114}}` の後に行う。
