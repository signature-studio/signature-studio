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

依頼者の指示: 拡大率の測定に、Settings の Debug のログを含める。FL 自身が、何の値を見て、何を決めたかを、FL の窓から取る。

この Issue の目標は、F10 の General settings を開き、(1) Display の節の Main GUI Scaling などの選択値、(2) Debug タブのログ(行数と絞った行)、(3) Info タブの要点を、測定の道具から取り、`tools/feat-c1-hidpi/measure-size.mjs` の JSON に出すこと。

## 背景

- `#{{113}}`(#1353)の前回の試行: FL の拡大率は DPI から決まり、25% の段に丸まる(窓の幅 ÷ DPR 1 の窓の幅 874)。`Main GUI Scaling` の設定の読みは、起動から idle までに 0 件(FL は起動の時に設定を読まず、DPI から決めると推定)。F10 の窓は、Welcome 窓が開いたままのため届かなかった。
- `#{{116}}` が、Welcome 窓を閉じる時の trap を直す。その後に、F10 の窓が届く。
- Display の節・Debug タブ・Info タブは、まだ一度も開けていない。
- 拡大率の段が FL 自身の規則か、Windows 側の値の違いかは、Windows の実機の値が無いので判定できない。FL の Display の選択と、Debug ログの DPI の行が、その判定の材料になる。

## 読む物

- `tools/feat-c1-hidpi/measure-size.mjs`(`#{{113}}` の成果物。JSON の欄を直す)
- `tools/feat-c6-popup-menus/measure.mjs`(窓の操作と、窓の表から座標を取る方法)、`tools/boot-check/cdp.mjs`
- `lift/runtime/fle_win.c`(窓の API の記録・`call_wndproc`・`fle_win_trace_msg`)、`lift/runtime/fle_gdc.c`(`ExtTextOutW`・`DrawTextW`・`TextOutW` の実装)
- `app/src/engine/fruity.ts`(`windowApiTrace`)、`window.__app` の窓の表(`summary`)
- `pm-work/findings/hidpi-fl-scaling.md`(`#{{113}}` の記録)

## やること

1. 窓の操作。Welcome 窓を閉じ(`#{{116}}` の直した後の経路)、F10 で General settings を開き、タブ(MIDI・Audio・General・File・Theme・Project・Info・Debug・Account・About)を切り替えられるようにする。座標は、窓の表と部品の位置から取る。固定の座標を使わない。
2. 窓の文字を読む経路を 1 つ選び、実測で確かめる。
   - (i) 標準のテキストの部品(EDIT・RichEdit)なら、`WM_GETTEXT`・`GetWindowTextW`・`EM_GETLINE` の読み。
   - (ii) FL 自作の描画なら、`ExtTextOutW`・`DrawTextW`・`TextOutW` の文字列の記録(測定の道具が有効にした時だけ記録する)。
   - (iii) OCR は使わない。
   - どの方式で取れたかを、理由と一緒に報告に書く。
3. 取る物。
   - General タブの Display の節: Main GUI Scaling・Pop-ups・Toolbars・Cursors・Legacy scaling の選択値を、読める形で。
   - Debug タブ(ログ)の全文の行数。絞った行(`dpi`・`scal`・`screen`・`monitor`・`display`・`resol`・`zoom`・`window`・`error`・`warn` を含む行。大文字と小文字を区別しない)の全行を、`pm-work/findings/hidpi-fl-scaling.md` に載せる。長い全文は載せない。
   - Info タブ: FL・OS・画面の情報の要点。
4. 道具の出力。`tools/feat-c1-hidpi/measure-size.mjs` の JSON の `mainGuiScaling` の欄は `display` に改め、Display の節の Main GUI Scaling の選択値(文字列。取れなければ null)を入れる。`debugLog`(`{"captured","lines","method","scalingLines"}`)を、F10 の Debug タブの読みで埋める。取れない項目は null のままにし、理由を報告に書く。
5. 測定。DPR 1 と 1.7647 を含む全ての DPR で、道具を 1 回ずつ動かす。`#{{113}}` の判定(拡大率・縁・部品・ポインタ)は、そのまま残す。
6. 報告に、取れた方式、Display の選択値、Debug ログの行数と絞った行、Info の要点、Main GUI Scaling が System か(読めれば)を書く。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/feat-c1-hidpi/measure-size.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/size.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const R=L.filter((x)=>"dpr" in x);const pick=(d)=>R.find((x)=>Math.abs(x.dpr-d)<0.0001);const ok=(x)=>!!x&&x.error===null&&x.state==="engine-idle"&&x.standIn===false&&!!x.debugLog&&x.debugLog.captured===true&&x.debugLog.lines>=1&&typeof x.display==="string"&&x.display.length>0;process.exit(ok(pick(1))&&ok(pick(1.7647))?0:1)' "$RUNNER_TEMP/size.jsonl" && npm test --prefix app
```

判定の基準:
- 道具の DPR 1 の行と DPR 1.7647 の行の両方で、`error` が null・`state` が engine-idle・`standIn` が false。
- 両方の行で、`debugLog.captured` が true で、`debugLog.lines` が 1 以上。
- 両方の行で、`display`(Main GUI Scaling の選択値)が文字列で出ている。
- 単体テスト(`npm test --prefix app`)が通る。

値そのもの(System か否か)は判定に入れない。報告に書く。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 窓の文字を読んだ方式と理由、F10 の Display の選択値(全て)、Debug ログの行数と絞った行(`pm-work/findings/hidpi-fl-scaling.md` の節)、Info の要点、DPR ごとの `display`・`debugLog`、Main GUI Scaling が System か(読めれば)。
- 読めなかった項目は「未確認」と書き、理由を書く。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- F10 の操作・窓の文字を読む経路・Debug の記録は、測定の道具だけが有効にする。既定の動作(利用者の画面)を変えない。
- 特定の拡大率・DPI・DPR の値を、runtime・app・seed・設定・テストに、定数・表・分岐として入れない。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- Python は製品に使わない(調査と検証にだけ使う)。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- engine の作り直しは約 35 分。ビルドは、フォアグラウンドで、シェルツールの `timeout` 引数に大きい値(例 3600000)を渡して待つ。
- 1 回の試行で、ビルドは数回まで。窓の操作と文字の読みは、記録をまとめた 1 回のビルドで行う。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: `#{{116}}`(Welcome 窓を閉じる時の trap)と `#{{113}}`(拡大率の測定の道具。土台)。
- `#{{114}}`(DPI 240 以上の停止)とは別。この Issue の測定は、停止しない DPR で行う。
- `#{{108}}`(動的な変化と全ての拡大率)は、この Issue の後に行う。
