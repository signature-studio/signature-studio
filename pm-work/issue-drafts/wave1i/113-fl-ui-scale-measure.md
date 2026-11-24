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

### 2 回目の試行(run 45198367018。約 3 時間 15 分。判定に届かず `failed`。作業は `work.diff` として artifact `agent-1353` に残った)

次の run は、この `work.diff` を作業ツリーに取り込んでから始める(取り込みの結果は `.prior-work.md`)。artifact は次の作業者が読めないので、要点を本文に書く。

- **拡大率**(DPR ごと。runner の値。nproc 4、loadavg 0.35〜1.26):

  | DPR | DPI | FL の拡大率 | round(100×DPR) | 差 |
  | --- | --- | --- | --- | --- |
  | 1 | 96 | 100 | 100 | 0 |
  | 1.25 | 120 | 125 | 125 | 0 |
  | 1.5 | 144 | 150 | 150 | 0 |
  | 1.7647 | 169 | 175 | 176 | −1 |
  | 2 | 192 | 200 | 200 | 0 |
  | 2.04(乱数。2.0 超。記録のみ) | 196 | 200 | 204 | −4 |
  | 1.13(乱数) | 108 | 100 | 113 | −13 |

- **式との違い**: 式 `min(200, 100 + 25 × round((DPI−96)/24))` は DPI 108 で 125 を返すが、FL は 100 を採った。100/125 の境界は DPI 109〜111 のどこか(観測: 108 → 100。111 → 125 は前回)。PM の見積もり(109〜114)より狭い。runtime では段を作らない。式は直していない。
- **縁**: `edgeWidth` は 2.261(DPR 1)・2.425(1.25)・2.527(1.5)・2.626(1.7647)・3.199(2)。比(`edgeWidth` ÷ (拡大率 ÷ 100)、DPR 1 の値で割る)の最大は 1.000(1.2 以下)。
- **部品**: 3 部品の座標は 7 行とも揃った。Song Position と Pattern Selector は画像から測っておらず、DPR 1 の座標を Tempo の比で求めた推定。
- **ポインタ**: `pointerSentErrPx`(ページが送った押下の座標)は 7 行とも 0 画素。`pointerErrPx`(engine が受けた画素)は、engine が応答した DPR 1.5 の 1 行だけ 0。残りの 6 行は `null`(engine が止まった)。
- **原因(確定)**: 起動の時、背景スレッド 13 が `System/Internet/Default news.rss`(`InternetOpenUrlW`。`downloadnewsrss` の周辺)を開こうとし、`HostFilesUnsupported` で trap する。`lift/harness/web/threads/fle_thread_lib.js` の `fail()` は記録して表示するだけで、待っている他のスレッドへ制御を渡さない。`app/src/engine/fruity.ts`(122 行)の `fleThreadFailed` もログに書くだけ。そのため、ページは `engine-idle` のまま押下に応じない(押下はページから engine に届く。`inputEvents` 9)。
- **頻度と切り分け**: DPR 1・1.25・1.7647・2 は、単発の測定 5 回すべてで止まった。DPR 1.5 は 5 回すべて応答した。trace は原因ではない(trace の無い構成でも 2 回中 1 回止まった)。ビューポートを上書きした構成では 4 回中 4 回止まった。同じ DPR を最大 8 回起動し直す方法は効かなかった(48 回すべて止まった)ので、道具から外した。
- **main の版**: この run の main には、`#{{116}}`(Welcome 窓を閉じると engine が trap する回帰の修正)が入っていなかった。run は 14:50Z に始まり、`#{{116}}` は 16:50Z に入った。スレッド 13 の trap は `#{{116}}` で消えた可能性が高い(推定。未確認)。
- **道具の変更**(`work.diff` に在る。次の run が取り込む): `pointerSentErrPx`・`engineFaults`・`pointerDiag`(診断)を足した。応答を待つ時間は 500 ms から最大 5 秒にした。起動し直しは外した。`pointerErrPx` の定義は変えていない(engine が受けた画素)。資料 `pm-work/findings/hidpi-fl-scaling.md` の節 9 は、今回の結果で書き換えた。
- **受け入れの式が落ちた理由**: `pointerErrPx` の `null`(6 行)。他の項目(起動・拡大率・縁・部品)は通った。合成データでは、前回の 7 行で終了コード 0、DPR 1.5 の拡大率を 180 にすると 1。
- **ソース検査の注意**: 受け入れのソース検査は `refs/judge/main` を基準にする。判定の段は main から `refs/judge/main` を取り込むので(`.github/workflows/agent.yml` の 698 行)、判定では効く。作業者の手元の checkout には `refs/judge/main` が無いので、そこでは空振りする。作業者は HEAD 基準で手で確かめ、報告に書く。
- **PM の決定(受け入れの直し)**: (b) 固定の 5 点(1・1.25・1.5・1.7647・2)は `round(100 × DPR)` の ±1。乱数の行は ±25(FL の 1 段。段の境界が DPI 109〜111 の近くにあり、±13 を超えうる)。2.0 を超える行は記録のみ。(e) `pointerErrPx` が null の行は、まず同じ DPR を再測定する(最大 3 回)。それでも null なら不合格。
- **未確認**: Windows 実機の値。100/125 の境界の正確な DPI。F10 の Display の節と Debug ログ(`#{{117}}`)。

### 残りのやること(2 回目の試行の後)

(a) 作業ツリーを確かめる。`git status`・`git diff HEAD` で、`lift/runtime/fle_reg.c`・`fle_wall_ia2v.c`、`tools/feat-c1-hidpi/measure-size.mjs`・`read-manifest.mjs`、`pm-work/findings/hidpi-fl-scaling.md` の変更が載っていること。載っていなければ、`work.diff` の内容から作り直す(違いは報告に書く)。
(b) 最新の main(`#{{116}}` を含む)で測り直す。ビルドしてから、`tools/feat-c1-hidpi/measure-size.mjs` で 7 行を出す。スレッド 13 の trap が消えたかを、JSON の `engineFaults` で見る。
(c) まだ trap するなら、`Default news.rss` の読みを直す。`lift/runtime` の VFS の「無いファイル」の扱いで、ホストのファイルとして seed に無い物は `ERROR_FILE_NOT_FOUND` を返す(Windows の挙動)。`System/Internet/Default news.rss` が `inputs/fl-installer-files.tsv` に在るかを確かめる。在れば、小さい物は seed に入れ、大きい物は `@lazy-file:` にする。ページに、スレッドの失敗を画面のエラーとして出すことは、この Issue の範囲外(別の Issue)。
(d) 受け入れを 1 回通す(`pointerErrPx` の再測定も、受け入れの式が行う)。
(e) 作業ツリーを commit する(受け入れが通らなくても。未 commit の変更を残さない)。

## 読む物

- `tools/feat-c1-hidpi/measure-size.mjs`・`tools/feat-c1-hidpi/read-manifest.mjs`(#{{107}} の成果物。前回の作業で直した版)
- `lift/runtime/fle_win.c`(窓の API の記録)、`lift/runtime/fle_gdc.c`・`fle_sysmetrics.c`・`fle_appfile.c`・`fle_proc.c`(DPI と寸法の応答)、`lift/runtime/fle_screen.h`(拡大率と DPI の式)
- `app/src/main.ts`・`app/src/engine/client.ts`・`app/src/engine/fruity.ts`(起動前の記録)
- `app/src/ui/screen-fit.ts`・`app/src/ui/desktop.ts`
- `lift/harness/seeds/`(seed の表・レジストリの seed・設定ファイルの seed)
- `tools/boot-check/cdp.mjs`
- `pm-work/findings/hidpi-fl-scaling.md`(#{{107}} の資料と、前回の節 9)

## やること

(1 回目の試行の手順。2 回目の試行の後は、上の「残りのやること(2 回目の試行の後)」に従う。)

1. 作業ツリーの確認。`git status`・`git diff HEAD` で、上の「前回の試行で分かったこと」の成果物(道具・記録・trace)が載っているかを見る。載っていれば、以下を続ける。無ければ、前回の作業は無いものとして、道具の拡大率・縁・部品・ポインタの測定を、上の記述どおりに作り直す。
2. 判定の式(受け入れの node -e)を、合成データで確かめる。前回の実測の 7 行(上の表の値)を入れて通ること。(b) の外れた値(例: DPR 1.5 の行で `flScalingPercent` を 180)を入れて落ちること。
3. 測定。`tools/feat-c1-hidpi/measure-size.mjs` で、現在のビルドの 7 行を出す。前回の測定と今のビルドが同じなら、再測定は要らない(報告に書く)。再測定が要る時だけ、ビルドする。
4. 受け入れが通るかを確かめる(受け入れの式)。通らなくても、作業ツリーを commit する。
5. 窓の操作(F10・タブの切り替え)・窓の文字の読み・Debug ログ・Info・Display の節の取得は、この Issue では行わない(#{{117}} に回した)。道具の `mainGuiScaling` と `debugLog` は null のままでよい。
6. ポインタの測り方。Welcome 窓が開いている間は主窓が入力を受けないため、前回は Welcome 窓の題名帯の 3 点で測った。#{{116}} の後は、主窓の部品の中心の 3 点に直してよい(直したら、報告に書く)。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/feat-c1-hidpi/measure-size.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/size.jsonl" && for i in 1 2 3; do D=$(node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l)).filter((x)=>"dpr" in x);const last=new Map();for(const x of L)last.set(String(x.dpr),x);process.stdout.write([...last.values()].filter((x)=>x.pointerErrPx===null).map((x)=>x.dpr).join(","))' "$RUNNER_TEMP/size.jsonl") && test -n "$D" || break; node tools/feat-c1-hidpi/measure-size.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --dprs "$D" --random 0 >> "$RUNNER_TEMP/size.jsonl"; done && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const M=new Map();for(const x of L.filter((x)=>"dpr" in x))M.set(String(x.dpr),x);const R=[...M.values()];const P=["tempo","song-position","pattern-selector"];const eq=(x,y)=>Math.abs(x-y)<0.0001;const F=[1,1.25,1.5,1.7647,2];const isF=(d)=>F.some((f)=>eq(d,f));const v=L[0].randomDprs.values;const want=[...F,...v];const base=R.find((x)=>eq(x.dpr,1));const ok=R.length===7&&v.length===2&&v.every((d)=>d>=1&&d<=2.25)&&want.every((d)=>R.some((x)=>eq(x.dpr,d)))&&!!base&&R.every((x)=>x.error===null&&x.state==="engine-idle"&&x.standIn===false&&Number.isFinite(x.flScalingPercent)&&(x.dpr>2.0||Math.abs(x.flScalingPercent-Math.round(100*x.dpr))<=(isF(x.dpr)?1:25))&&Number.isFinite(x.edgeWidth)&&x.edgeWidth/(x.flScalingPercent/100)<=1.2*base.edgeWidth&&Number.isFinite(x.pointerErrPx)&&x.pointerErrPx<=1&&P.every((p)=>Number.isFinite(x.parts[p].x)&&Number.isFinite(x.parts[p].y)));process.exit(ok?0:1)' "$RUNNER_TEMP/size.jsonl" && ! git diff -U0 refs/judge/main -- lift/runtime lift/harness app/src | grep -E '^\+[^+]' | grep -qE '\b(125|150|168|169|175|176)\b|1\.75\b|1\.7647|0\.5667' && npm test --prefix app
```

判定の基準: 道具の 7 行(DPR ごとの行)の生の値を、上の node -e が読む。
- (a) 固定の一覧(1・1.25・1.5・1.7647・2)と乱数 2 つ(1.00〜2.25)が全て在る。全行が正常な起動(`error` が null・`state` が engine-idle・`standIn` が false)。
- (b) DPR が 2.0 以下の行の `flScalingPercent` が、固定の 5 点(1・1.25・1.5・1.7647・2)では round(100 × DPR) の ±1、乱数の行では ±25(FL の 1 段。段の境界が DPI 109〜111 の近くにあり、±13 を超えうる)。2.0 を超える行は、値を記録するだけ(FL の 200% の頭打ちが FL の規則か、この機械では確かめられないため)。
- (c) `mainGuiScaling` は判定から外す(起動の時には読まれない。null のままでよい。値は #{{117}} が F10 で取る)。
- (d) 全ての行で、`edgeWidth` ÷ (`flScalingPercent` ÷ 100) が、DPR 1 の行の `edgeWidth` の 1.2 倍以下。拡大による縁のぼやけが、FL 自身の描き方を超えないこと。
- (e) `parts` の 3 部品が全て座標を持つ。`pointerErrPx` が 1 以下。`pointerErrPx` が null の行は、同じ DPR を再測定する(受け入れの式の中で、最大 3 回。DPR ごとに最後に測った行で判定する)。それでも null なら不合格。`pointerSentErrPx` は補助で、判定に入れない。
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
