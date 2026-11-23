---
draft: "117"
title: "F10 の General settings(Display の節・Debug タブ・Info タブ)を開き、FL 自身のログと設定を取る"
type: Task
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "005"
blocked_by: ["116", "113"]
labels: ["hidpi", "verify"]
size: L
priority: High
project: signature-studio/3
---

## 目的

依頼者の指示: 拡大率の測定に、Settings の Debug のログを含める。FL 自身が、何の値を見て、何を決めたかを、FL の中から取る。

この Issue の目標は、F10 の General settings を開き、(1) Display の節の Main GUI Scaling などの選択値、(2) Debug タブのログ(行数と絞った行)、(3) Info タブの要点を、測定の道具から取り、`tools/feat-c1-hidpi/measure-size.mjs` の JSON に出すこと。

## 背景

- `#{{113}}` の前回の試行: FL の拡大率は DPI から決まり、25% の段に丸まる(窓の幅 ÷ DPR 1 の窓の幅 874)。`Main GUI Scaling` の設定の読みは、起動から idle までに 0 件(FL は起動の時に設定を読まず、DPI から決めると推定)。
- `#{{116}}` は閉じた(Welcome 窓を閉じる時の trap が直った)。前回の試行で、Welcome 窓と `TMissingPluginsForm` を Escape で閉じ、F10 の窓を開けた(General 頁まで)。
- 拡大率の段が FL 自身の規則か、Windows 側の値の違いかは、Windows の実機の値が無いので判定できない。FL の Display の選択と、Debug ログの DPI の行が、その判定の材料になる。
- 窓の表に出ない文字(Debug の本文・Info の欄・Display の選択値)は、ゲストのメモリから読む(下の「やること」の 3)。

## 前回の試行で分かったこと

run 45259844145(約 3 時間。`failed`)の結果。次の作業者は artifact を読めないので、要点をここに書く。

- 作業は保存された(artifact `agent-1358` の `work.diff`。約 28 KB。main には入っていない)。次の run は、この差分を作業ツリーに載せ、`.prior-work.md` で続きから行う。
- 前回の差分の中身: `lift/runtime/fle_win.c`・`fle_desk.c`(窓の表の子窓を `child lid … text "…"` の行として足す。読み出しのみ)、`tools/feat-c1-hidpi/measure-size.mjs`(F10 の操作 `settingsPhase`: Escape で Welcome 窓と `TMissingPluginsForm` を閉じ → F10 → 帯の 10 点 → 頁と題名の確認 → Debug の本文の読み)、`pm-work/findings/hidpi-fl-scaling.md` の節 10。単体テスト 1335 件が通る。
- 落ちた理由: `debugLog.captured` が false(行数 0)・`display` が null。
- **窓の文字を読む経路は届かなかった(この経路は使わない)**:
  - 窓の表に出る文字は、頁の題名(`MIDI`・`General` など)・パネルの見出し・窓の題名。Debug の本文(`TQuickMemo`)と Info の欄(`TQuickEdit`)の文字は `text ""` で、表に載らない。
  - 文字描画の記録(`TEXT DRAWING REACHED`)は全行で 0 件。FL の文字は GDI の文字 API を通らず、`QuickFontCache`(FreeType・Agg)と Blend2D の中で字形にされる。FLEngine は Blend2D の文字 API を名前で import していないので、import の境界での記録もできない。
  - OCR は使わない。
- Display の選択値: 未確認。General 頁の見出し(窓の表): `System`・`Display (requires a restart to take effect)`・`Advanced`・`Miscellaneous`・`Input`・`Updates and privacy`・`Language (requires a restart to take effect)`・`Hints and warnings`・`Undo history`。設定の読み: 名前に `scaling`・`GUI`・`uiscale`・`PPI` を含む `RegQueryValueExW` は 0 件(FL が読んだのは `LogStatisticsEnabled`(not found)と `Shared\Paths` だけ)。
- Debug ログ: DPR 2 と 1.93 では Debug 頁を開けたが、本文は `text ""` で行数 0。他の DPR では帯に Debug 頁が出ず、開けなかった。File と Info の頁は、どの DPR でも帯に出なかった。依頼者の本番の画面(100% の UI)では、全ての頁(MIDI・Audio・General・File・Theme・Project・Info・Debug・Account・About)が見えている。窓の表に出ないだけで、頁は在ると推定する(未確認)。
- 帯(設定窓の頁の並び)の幅: DPR 1 → 658、1.25 → 824、1.5 → 986、1.7647 → 1152、2 → 1314。FL の拡大率は 100・125・150・175・200%(`#{{113}}` と同じ)。
- 縁・ポインタ(`pointerErrPx` 0)は `#{{113}}` のとおり。

## 読む物

- `tools/feat-c1-hidpi/measure-size.mjs`(`#{{113}}` の成果物。JSON の欄を直す)
- `tools/feat-c6-popup-menus/measure.mjs`(窓の操作と、窓の表から座標を取る方法)、`tools/boot-check/cdp.mjs`
- `lift/runtime/fle_win.c`(窓の API の記録・`call_wndproc`・`fle_win_trace_msg`)
- `app/src/engine/worker.ts`(要求の `case`(`desk-dump`・`desk-peek`)と `curMemory()`(ゲストの wasm のメモリ))、`app/src/engine/client.ts`(`deskDump`)、`app/src/main.ts`(`window.__app` の `deskDumpBase64`・`engineLog`)
- `app/src/engine/fruity.ts`(`windowApiTrace`)、`window.__app` の窓の表(`summary`)
- `pm-work/findings/hidpi-fl-scaling.md`(`#{{113}}` の記録。節 9)

## やること

1. 作業ツリーの確認。`.prior-work.md` を最初に読む。`git status`・`git diff HEAD` で、前回の変更(上の「前回の試行で分かったこと」の 2 つ目)が載っているかを見る。載っている物は取り除かず、残りを行う。載っていない物は、この節の方式で作る。
2. 窓の操作。Escape で Welcome 窓と `TMissingPluginsForm`(載っていれば)を閉じ、F10 で General settings を開く。頁(MIDI・Audio・General・File・Theme・Project・Info・Debug・Account・About の 10 頁)は、帯の位置をクリックして切り替える。帯の位置は、窓の表の帯の矩形、または頁の題名の文字の位置から取る。取れなければ、設定窓の矩形を頁の数(10)で等分する。切り替わったことは、窓の題名と本文の変化で確かめる。固定の座標を使わない。
3. 文字の読み(ゲストのメモリから)。窓の表と文字描画の記録では、Debug の本文・Info の欄・Display の選択値の文字は取れなかった。FL は、ログの行を、起動の時から、ゲストの線形メモリの中に文字列として持つ。その文字列を、測定の時だけ読む。
   - 経路: `window.__app` に、読み出しを頼む関数を 1 つ足す(測定の道具だけが呼ぶ)。既存の `deskDumpBase64`・`engineLog` と同じ経路を通す: `app/src/main.ts` → `app/src/engine/client.ts` → `app/src/engine/worker.ts` の要求(`desk-dump`・`desk-peek` と並べる `case` の 1 つ)。Worker は `curMemory()`(ゲストの wasm のメモリ)を読む。既定では呼ばれず、利用者の画面の動作は変わらない。
   - 探索は Worker の側で行う。メモリは数百 MB なので、全体は返さない。目印の文字列の位置の前後で、印字可能な文字の連続を取り、改行(`\r\n`・`\n`)で行に分けて、行の配列だけを返す。
   - 目印: FL が起動の時に書く行(例: `Browser caching took 0 milliseconds.`・`There were no updates for MIDI scripts.`・`There were no updates for themes.`・`Mixer track map: 1,2,3,4,…`)の先頭の文字列。ASCII・UTF-16LE・UTF-8 の 3 通りで探す。見つからなければ、`There were no updates for`・`Browser caching took` まで短くして試す。
   - 時刻: 起動の idle の後、Debug タブを開く前と後の 2 回読む。どちらで見つかったかを、報告と JSON に書く。
   - 結果は JSON の `debugLog` に入れる: `captured`(行が 1 以上で true)・`lines`(行数)・`method`(`"guest-memory-scan"`)・`scalingLines`(絞った行の配列)。絞った行は、`dpi`・`scal`・`screen`・`monitor`・`display`・`resol`・`zoom`・`window`・`error`・`warn` を含む行(大文字と小文字を区別しない)。
4. Display の選択値。General 頁の Display の節の選択値(Main GUI Scaling・Pop-ups・Toolbars・Cursors・Legacy scaling)は、メモリの中の選択肢の文字列(`System`・`100%`・`125%` など)と、選ばれている添字の値を、読める範囲で読む。Main GUI Scaling の選択値を JSON の `display`(文字列。取れなければ null)に入れ、他の項目は `displayChoices`(項目名 → 選択値)に入れる。
5. Info タブ。Info の欄の文字を、同じ方式で読む。FL の版・OS・画面の情報の要点を、JSON の `info`(文字列の配列。取れなければ null)に入れる。
6. 道具の出力。`tools/feat-c1-hidpi/measure-size.mjs` の `mainGuiScaling` の欄(レジストリの読み)は、`display` に改める。既存の canvas の寸法の欄 `display`(`{w, h}`)は `canvasDisplay` に改める(名前の衝突を避ける)。`debugLog.why` の「Debug タブに届かない」という固定の文は、読めた時は null にする。取れない項目は null のままにし、理由を報告に書く。
7. 測定。DPR 1 と 1.7647 を含む全ての DPR(固定の 5 と乱数の 2)で、道具を 1 回ずつ動かす。`#{{113}}` の判定(拡大率・縁・部品・ポインタ)は、そのまま残す。`pointerErrPx` が null の行は、受け入れのループで測り直す。
8. 記録。`pm-work/findings/hidpi-fl-scaling.md` の節 10 に、取れた結果を書く。Debug ログの行数・先頭 30 行・絞った行、Display の選択値、Info の要点。長い全文は載せない。
9. 受け入れを通し、作業ツリーを commit する。受け入れが通らなくても、作業ツリーを commit する。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/feat-c1-hidpi/measure-size.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/size.jsonl" && for i in 1 2 3; do D=$(node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l)).filter((x)=>"dpr" in x);const last=new Map();for(const x of L)last.set(String(x.dpr),x);process.stdout.write([...last.values()].filter((x)=>x.pointerErrPx===null).map((x)=>x.dpr).join(","))' "$RUNNER_TEMP/size.jsonl") && test -n "$D" || break; node tools/feat-c1-hidpi/measure-size.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --dprs "$D" --random 0 >> "$RUNNER_TEMP/size.jsonl"; done && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const M=new Map();for(const x of L.filter((x)=>"dpr" in x))M.set(String(x.dpr),x);const R=[...M.values()];const P=["tempo","song-position","pattern-selector"];const eq=(x,y)=>Math.abs(x-y)<0.0001;const F=[1,1.25,1.5,1.7647,2];const isF=(d)=>F.some((f)=>eq(d,f));const v=L[0].randomDprs.values;const want=[...F,...v];const base=R.find((x)=>eq(x.dpr,1));const dl=(x)=>!!x&&!!x.debugLog&&x.debugLog.captured===true&&x.debugLog.lines>=1&&x.debugLog.method==="guest-memory-scan";const ok=R.length===7&&v.length===2&&v.every((d)=>d>=1&&d<=2.25)&&want.every((d)=>R.some((x)=>eq(x.dpr,d)))&&!!base&&R.every((x)=>x.error===null&&x.state==="engine-idle"&&x.standIn===false&&Number.isFinite(x.flScalingPercent)&&(x.dpr>2.0||Math.abs(x.flScalingPercent-Math.round(100*x.dpr))<=(isF(x.dpr)?1:25))&&Number.isFinite(x.edgeWidth)&&x.edgeWidth/(x.flScalingPercent/100)<=1.2*base.edgeWidth&&Number.isFinite(x.pointerErrPx)&&x.pointerErrPx<=1&&P.every((p)=>Number.isFinite(x.parts[p].x)&&Number.isFinite(x.parts[p].y)))&&[1,1.7647].every((d)=>dl(R.find((x)=>eq(x.dpr,d))));process.exit(ok?0:1)' "$RUNNER_TEMP/size.jsonl" && ! git diff -U0 refs/judge/main -- lift/runtime lift/harness app/src | grep -E '^\+[^+]' | grep -qE '\b(125|150|168|169|175|176)\b|1\.75\b|1\.7647|0\.5667' && npm test --prefix app
```

## 判定の基準

- 全ての行(固定の 5 と乱数の 2 を含む 7 行): `error` が null・`state` が engine-idle・`standIn` が false・`flScalingPercent`・`edgeWidth`・`pointerErrPx`・`parts`(`#{{113}}` のまま)。
- DPR 1 と DPR 1.7647 の行: `debugLog.captured` が true・`debugLog.lines` が 1 以上・`debugLog.method` が `"guest-memory-scan"`。
- ソースの検査: 足した行に、拡大率・DPI・DPR の値(125・150・168・169・175・176・1.75・1.7647・0.5667)を書かない。文字の範囲(ASCII・UTF-16 の値)は 16 進で書く。
- 単体テスト(`npm test --prefix app`)が通る。
- 判定から外す(JSON と報告には出す): `display`・`displayChoices`・`info`・`debugLog.scalingLines`・`canvasDisplay`。DPR 2.5・3(`#{{114}}`)。Main GUI Scaling が System か否か(値そのもの)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 窓の文字をメモリから読んだ経路(目印・見つかった時刻・ASCII か UTF-16LE か)、読めなかった項目とその理由、F10 の Display の選択値(読めた物は全て)、Debug ログの行数と絞った行(節 10 の要点)、Info の要点、DPR ごとの `display`・`debugLog.captured`・`lines`・`method`、Main GUI Scaling が System か(読めれば)。
- 読めなかった項目は「未確認」と書き、理由を書く。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- 窓の操作・メモリの読み出し・Debug の記録は、測定の道具だけが有効にする。既定の動作(利用者の画面)を変えない。ゲストのメモリには書き込まない(読むだけ)。
- 特定の拡大率・DPI・DPR の値を、runtime・app・seed・設定・テストに、定数・表・分岐として入れない。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- Python は製品に使わない(調査と検証にだけ使う)。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- 最初に `date -u` で開始の時刻を記録する。4 時間 30 分を過ぎたら、受け入れが通っていなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- engine の作り直しは約 35 分。ビルドは、フォアグラウンドで、シェルツールの `timeout` 引数に大きい値(例 3600000。上限 5400000)を渡して待つ。
- 1 回の試行で、ビルドは数回まで。窓の操作・メモリの読み・記録は、まとめて 1 回のビルドで行う。

## 他のタスクとの関係

- blocked_by: `#{{116}}`(Welcome 窓を閉じる時の trap。閉じた)と `#{{113}}`(拡大率の測定の道具。閉じた)。
- `#{{114}}`(DPI 240 以上の停止)とは別。この Issue の測定は、停止しない DPR で行う(DPR 2.5・3 は判定に入れない)。
- `#{{108}}`(動的な変化と全ての拡大率)は、この Issue の後に行う。
