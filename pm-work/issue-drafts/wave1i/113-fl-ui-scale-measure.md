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

この Issue の目標は、FL の描いた画面から、FL が採用した拡大率・部品の位置・縁の遷移の幅・ポインタの座標の変換を測る道具を入れ、その値を Windows の計算と突き合わせることである。拡大率は runtime が決めて渡すのではなく、FL が読んだ DPI から FL が決める。FL 自身の窓(F10 の設定・Debug ログ)からの取得は、#{{117}} が行う。

## 前回の試行で分かったこと

前回の試行(run 45148253436。判定に届かず `failed`)。作業は artifact `agent-1353` の `work.diff` に在り、push されていない。取り込みで作業ツリーに載る。

- **FL の拡大率は 25% の段に丸まる(実測)**。ウェルカム窓の幅 ÷ DPR 1 の窓の幅(874 px)で求めた。

  | DPR | LOGPIXELSY | 窓の幅 | FL の拡大率 | round(100×DPR) | 差 |
  | --- | --- | --- | --- | --- | --- |
  | 1 | 96 | 874 | 100 | 100 | 0 |
  | 1.25 | 120 | 1091 | 125 | 125 | 0 |
  | 1.5 | 144 | 1310 | 150 | 150 | 0 |
  | 1.7647 | 169 | 1529 | 175 | 176 | −1 |
  | 2 | 192 | 1748 | 200 | 200 | 0 |
  | 1.64(乱数) | 157 | 1529 | 175 | 164 | +11 |
  | 2.14(乱数) | 205 | 1748 | 200 | 214 | −14 |

- 観測した段: DPI 111 → 125、132 → 150、157 → 175、169 → 175、180 → 200、192 → 200、205・211 → 200。DPI 108 と 156 の境界は、式からの推定で、観測していない。
- 丸めの式(推定): `min(200, 100 + 25 × round((DPI−96)/24))`。DPI = 96 × DPR なので、100 × DPR を 25 の段に丸めた値と同じになる。200% で頭打ち。
- Tempo の内側の幅(DPR 1: 54、1.25: 68、1.37: 81、1.5: 81、1.7647: 96、2: 106 画素)も、同じ段を示した。
- `DPI の応答` は `MulDiv(96, round(DPR×100), 100)` どおり。guest が読むのは LOGPIXELSY(90) だけで、LOGPIXELSX(88) は読まれない。
- FL が段に丸める規則が FL 自身のものか、Windows 側の値の違いかは、この機械では判定できない(Windows の実機の値が無い)。runtime は段を作らない。FL が見た DPI を Windows と同じに返すことだけを行う。
- **`Main GUI Scaling` の読みは、起動から idle までに 0 件**。名前に scaling・GUI・uiscale を含む読みは無い。読まれたのは、FL の設定の鍵(`HKCU` の `…\FL Studio 26\General`。65 回。すべて見つからない)など。FL は起動の時に設定を読まず、DPI から決めると推定する。設定の窓(F10)で読むかは、#{{117}} が確かめる。
- **`edgeWidth`(縁の遷移の幅。10%〜90% の遷移の、水平と垂直の中央値)**: DPR 1: 2.26、1.25: 2.43、1.5: 2.53、1.7647: 2.63、2: 3.20。DPR 1 でも 2 を超えるので、「2 画素以下」の閾値は FL の縁の描き方に合わない。判定は、DPR 1 の値に対する拡大の比で行う(下の受け入れ (d))。
- **`pointerErrPx`**: 全行 0 画素。ただし、Welcome 窓が開いている間は、主窓の上の押下が engine に届かない(モーダル。推定)。そのため、Welcome 窓の題名帯の 3 点で測った。
- **`parts`**: Tempo の枠は画像から測った。Song Position と Pattern Selector は、DPR 1 の座標を Tempo の比で求めた(測っていない)。
- **F10・Debug ログ・Info・Display の節は未確認**。Welcome 窓が開いたままのため、F10 が届かない。Welcome 窓を閉じる操作(「No thanks」・Escape)は engine を trap させる(解放の後の読み。`HostFilesUnsupported`)。#{{116}} が直す。
- 前回の追加(測定の道具が有効にした時だけ出す。既定では出ない): `RegQueryValueExW`・`GetPrivateProfileStringW` の trace(`lift/runtime/fle_reg.c`・`lift/runtime/fle_wall_ia2v.c`)。
- 前回の作業の成果物: `tools/feat-c1-hidpi/measure-size.mjs`(拡大率・縁・部品・ポインタ。`mainGuiScaling`・`debugLog` は null)、`pm-work/findings/hidpi-fl-scaling.md`(節 9)。

## 読む物

- `tools/feat-c1-hidpi/measure-size.mjs`・`tools/feat-c1-hidpi/read-manifest.mjs`(#{{107}} の成果物。前回の作業で直した版)
- `lift/runtime/fle_win.c`(窓の API の記録)、`lift/runtime/fle_gdc.c`・`fle_sysmetrics.c`・`fle_appfile.c`・`fle_proc.c`(DPI と寸法の応答)、`lift/runtime/fle_screen.h`(拡大率と DPI の式)
- `app/src/main.ts`・`app/src/engine/client.ts`・`app/src/engine/fruity.ts`(起動前の記録)
- `app/src/ui/screen-fit.ts`・`app/src/ui/desktop.ts`
- `lift/harness/seeds/`(seed の表・レジストリの seed・設定ファイルの seed)
- `tools/boot-check/cdp.mjs`
- `pm-work/findings/hidpi-fl-scaling.md`(#{{107}} の資料と、前回の節 9)

## やること

1. 作業ツリーの確認。`git status`・`git diff HEAD` で、上の「前回の試行で分かったこと」の成果物(道具・記録・trace)が載っているかを見る。載っていれば、以下を続ける。無ければ、前回の作業は無いものとして、道具の拡大率・縁・部品・ポインタの測定を、上の記述どおりに作り直す。
2. 判定の式(受け入れの node -e)を、合成データで確かめる。前回の実測の 7 行(上の表の値)を入れて通ること。(b) の外れた値(例: DPR 1.5 の行で `flScalingPercent` を 180)を入れて落ちること。
3. 測定。`tools/feat-c1-hidpi/measure-size.mjs` で、現在のビルドの 7 行を出す。前回の測定と今のビルドが同じなら、再測定は要らない(報告に書く)。再測定が要る時だけ、ビルドする。
4. 受け入れが通るかを確かめる(受け入れの式)。通らなくても、作業ツリーを commit する。
5. 窓の操作(F10・タブの切り替え)・窓の文字の読み・Debug ログ・Info・Display の節の取得は、この Issue では行わない(#{{117}} に回した)。道具の `mainGuiScaling` と `debugLog` は null のままでよい。
6. ポインタの測り方。Welcome 窓が開いている間は主窓が入力を受けないため、前回は Welcome 窓の題名帯の 3 点で測った。#{{116}} の後は、主窓の部品の中心の 3 点に直してよい(直したら、報告に書く)。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/feat-c1-hidpi/measure-size.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/size.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const R=L.filter((x)=>"dpr" in x);const P=["tempo","song-position","pattern-selector"];const eq=(x,y)=>Math.abs(x-y)<0.0001;const v=R.length?R[0].randomDprs.values:[];const want=[1,1.25,1.5,1.7647,2,...v];const base=R.find((x)=>eq(x.dpr,1));const ok=R.length===7&&v.length===2&&v.every((d)=>d>=1&&d<=2.25)&&want.every((d)=>R.some((x)=>eq(x.dpr,d)))&&!!base&&R.every((x)=>x.error===null&&x.state==="engine-idle"&&x.standIn===false&&Number.isFinite(x.flScalingPercent)&&(x.dpr>2.0||Math.abs(x.flScalingPercent-Math.round(100*x.dpr))<=13)&&Number.isFinite(x.edgeWidth)&&x.edgeWidth/(x.flScalingPercent/100)<=1.2*base.edgeWidth&&Number.isFinite(x.pointerErrPx)&&x.pointerErrPx<=1&&P.every((p)=>Number.isFinite(x.parts[p].x)&&Number.isFinite(x.parts[p].y)));process.exit(ok?0:1)' "$RUNNER_TEMP/size.jsonl" && ! git diff -U0 refs/judge/main -- lift/runtime lift/harness app/src | grep -E '^\+[^+]' | grep -qE '\b(125|150|168|169|175|176)\b|1\.75\b|1\.7647|0\.5667' && npm test --prefix app
```

判定の基準: 道具の 7 行(DPR ごとの行)の生の値を、上の node -e が読む。
- (a) 固定の一覧(1・1.25・1.5・1.7647・2)と乱数 2 つ(1.00〜2.25)が全て在る。全行が正常な起動(`error` が null・`state` が engine-idle・`standIn` が false)。
- (b) DPR が 2.0 以下の行の `flScalingPercent` が、round(100 × DPR) の ±13(FL の 25% の段の半分)。2.0 を超える行は、値を記録するだけ(FL の 200% の頭打ちが FL の規則か、この機械では確かめられないため)。
- (c) `mainGuiScaling` は判定から外す(起動の時には読まれない。null のままでよい。値は #{{117}} が F10 で取る)。
- (d) 全ての行で、`edgeWidth` ÷ (`flScalingPercent` ÷ 100) が、DPR 1 の行の `edgeWidth` の 1.2 倍以下。拡大による縁のぼやけが、FL 自身の描き方を超えないこと。
- (e) `parts` の 3 部品が全て座標を持つ。`pointerErrPx` が 1 以下。
- (f) `debugLog` は判定から外す(#{{117}})。
- (g) 乱数の DPR は、作業者の実行と判定の実行で別になる。期待値は、各行の DPR から式で計算する。固定の値に合わせた実装は通らない。
- ソースの検査(受け入れの最後の段): 足した行に、特定の拡大率・DPI・DPR の値(`125`・`150`・`168`・`169`・`175`・`176`・`1.75`・`1.7647`・`0.5667`)が、拡大率・DPI・DPR の意味で現れないこと。一致が 0 件であること。
- 単体テスト(`npm test --prefix app`)が通ること。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: DPR ごとの表(`flScalingPercent`・丸め方・DPI・部品の座標・`edgeWidth`・`pointerErrPx`)、(b)(d) の判定の値、合成データの確認の結果、特定の値を用意していないことの確認(source の検査の grep の出力。一致 0 件)、ポインタの測り方を変えたかどうか。
- 拡大率が (b) の基準から外れた場合は、FL が何を見ているかの特定と、直した内容を報告に書く。runtime で段を作って合わせない。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- 特定の拡大率・DPI・DPR の値を、runtime・app・seed・設定・テストに、定数・表・分岐・seed の値として入れない。
- 拡大して表示する版を作らない。
- 測定の道具の trace は、既定の動作(利用者の画面)を変えない。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- Python は製品に使わない(調査と検証にだけ使う)。

## 作業の進め方

- engine の作り直しは約 35 分。ビルドは、フォアグラウンドで、シェルツールの `timeout` 引数に大きい値(例 3600000)を渡して待つ。
- 1 回の試行(最大約 5.5 時間)で、ビルドは数回までなので、実験はビルドをまとめて計画する。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: #{{107}}(物理画素の画面と、DPI の式。測定の道具の土台)。
- #{{116}}(Welcome 窓を閉じる時の trap)と #{{117}}(F10 の Display の節・Debug・Info の取得)とは別。F10 の取得は #{{117}} が行う。
- #{{114}}(DPI 240 以上の停止)とは別。この Issue の測定は DPR 2.25 以下で行う。
- #{{115}}(Browser のパネル)と、描画の経路(`app/src/engine/`)が重なりうる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
- #{{108}}(動的な変化と全ての拡大率)は、この Issue と #{{114}} の後に行う。
