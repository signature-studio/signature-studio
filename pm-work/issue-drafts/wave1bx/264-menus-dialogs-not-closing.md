---
draft: "264"
title: "製品: メニューや F8 から開いた箱(TSpeechForm・TPlugListForm・TWAVRenderForm・TTapTempoForm・TPythonForm)が、Esc・OK・Cancel で閉じない(e2e-full の最新の run で 17 本)"
type: Task
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "133"
blocked_by: []
labels: ["verify"]
size: M
priority: High
project: signature-studio/3
---

## 目的

メニューの項目や F8 で開いた箱が、Esc・OK・Cancel のどれで答えても閉じない。同じ窓の型は、どの答えでも閉じない(3 種の答えを試して、全て落ちた窓がある)。e2e-full の最新の run では、この型で 17 本が落ちた。他の窓では Esc や OK で閉じる本が多い(例: 同じ領域の `menus/add-1-choose` は通る。`menus/add-5-5-cancel` も通る)ので、窓の型ごとの閉じ方の問題と見る(推定)。

この Issue の目標は次の 3 つ。

1. 上の 5 種の箱が、答えの後に表の一覧から消え、engine が idle のままになること(製品の閉じ方を直す)。
2. 道具の判定(答えの送り方・窓の表の読み)が正しいことを確かめる。道具が正しく、製品の閉じ方だけが誤りなら、製品を直す。道具が誤りなら、道具を直す。どちらかを報告に書く。
3. 直した後も、同じ領域の他の本が落ちないこと(回帰がないこと)。

## 背景

### 落ちた本(実測。e2e-full の最新の run の `tools/e2e` の出力。dpr1 の行)

| 箱(窓のクラス) | 本 | 答え | 出力の文言(一部) |
|---|---|---|---|
| TSpeechForm[Speech properties] | `menus/add-1-esc`・`menus/add-1-ok`・`menus/add-1-cancel` | Esc・OK・Cancel | `dialog TSpeechForm[Speech properties] is still there after esc` |
| TPlugListForm[PlugList] | `menus/add-6-esc`・`menus/add-6-ok`・`menus/add-6-cancel`・`menus/view-6-cancel` | Esc・OK・Cancel | `dialog TPlugListForm[PlugList] is still there after cancel` |
| TPlugListForm(F8) | `windows/plugin-esc`(dpr1・dpr2)・`windows/plugin-reopen`(dpr1・dpr2)・`input/key-escape-closes-plugin-list` | Esc | `F8 TPlugListForm: closed by Escape false`、`F8: TPlugListForm shown true; after Escape shown true` |
| TWAVRenderForm[Render options] | `menus/file-9-8-esc`・`menus/file-9-8-ok`・`menus/file-9-8-cancel` | Esc・OK・Cancel | `dialog TWAVRenderForm[Render options] is still there after esc` |
| TTapTempoForm[Tempo tapper] | `menus/view-8-ok` | OK | `dialog TTapTempoForm[Tempo tapper] is still there after ok` |
| TPythonForm[Script output] | `menus/view-10-ok`(同時に engine が止まる。型は別の Issue の対象) | OK | `dialog TPythonForm[Script output] is still there after ok` |

- 各本の判定は、答えの後に窓の表を読み直し、その箱が表に残っていないことを見る。残っていれば `ok` は偽。
- 前回の e2e-full では、TSpeechForm の 3 本(`menus/add-1-*`)が、Issue #1527 の本文で「箱が閉じない 8 本」の一部として書かれていた(残りの 5 本は、TInAppShopForm と TMissingPluginsForm)。#1527 はその群を「別の Issue にする」と書いて閉じた。この Issue が、その別の Issue に当たる。

### 見立て(推定。確認できていない)

- 製品: 窓の閉じ方(Esc・OK・Cancel の処理)が、この窓の型では効かない。どの答えでも閉じない窓が 3 種あるので、道具の座標の誤りとは考えにくい。
- 道具: 答えを送る場所(窓の表の位置)か、窓が前面にあるか。確認できない: 答えが窓に届いたかどうか(届いた証拠は、この run の出力には無い)。

## 読む物

- `tools/e2e/lib/menus.mjs`(`chooseItem`・`dialogAnswer`・`observe`・`closeNewWindows`)と `tools/e2e/scenarios/menus/add-1-esc.mjs`・`add-6-cancel.mjs`(答えの送り方)。
- `tools/e2e/lib/guest.mjs`(`escape`・`windows`・`press`)。
- `tools/e2e/scenarios/windows/plugin-esc.mjs`・`plugin-reopen.mjs`・`input/key-escape-closes-plugin-list.mjs`(F8 の箱)。
- `app/src/` の窓の閉じ方(TSpeechForm・TPlugListForm・TWAVRenderForm・TTapTempoForm・TPythonForm の閉じる処理)。`lift/` の窓の処理(TPlugListForm の Esc の扱い)。

## やること

1. 再現: 上の 14 本を、`--displays 4k-dpr1` で 1 回ずつ走らせる(`--only` に ID を並べる)。窓の表と答えの送信を、出力の `details` で確かめる。
2. 道具の確認: 答えが窓に届いたかを、キーの送信の記録と、窓の前面の表で確かめる。同じ窓を、他の答えや他の入口(F8 と Add メニュー)から開いて確かめる。
3. 切り分け: 道具の誤りなら道具を直す(本の座標・答えの順番など)。製品の誤りなら、閉じる処理を直す。どちらか分からなければ、窓の型ごとに、確かめた事実を報告に書く。
4. 直した後、印(`expectFail`)が残っている本の扱いを報告する。通った本から印を外す。通らない本には、落ちた理由を書いた印の Issue を付ける。
5. 受け入れを通す。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && (cd tools/e2e && npm ci && npx playwright install chromium) && node --test tools/e2e/test/*.test.mjs && node tools/e2e/run.mjs --only menus/add-1-esc,menus/add-1-ok,menus/add-1-cancel,menus/add-6-esc,menus/add-6-ok,menus/add-6-cancel,menus/view-6-cancel,menus/file-9-8-esc,menus/file-9-8-ok,menus/file-9-8-cancel,menus/view-8-ok,windows/plugin-esc,windows/plugin-reopen,input/key-escape-closes-plugin-list --displays 4k-dpr1,4k-dpr2 --budget 40m > "$RUNNER_TEMP/e2e.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((x)=>JSON.parse(x));process.exit(L.length>=1&&L.every((r)=>r.error===null&&r.scenarios.length>=1&&r.scenarios.every((s)=>s.ok===true&&s.skipped!==true&&s.error===null&&s.engineStops===0))?0:1)' "$RUNNER_TEMP/e2e.jsonl"
```

判定の基準:
- 受け入れの式が終了コード 0 で終わる(単体の `node --test` が通り、起動確認が engine-idle、14 本の E2E が全て合格する)。
- 報告に: (a) 窓の型ごとの原因の切り分け(道具・製品・確認できない)と、確かめた方法、(b) 直した箇所(ファイルと行。道具と製品を分けて書く)、(c) 印の扱い(外した本・残した本と、その理由)、(d) 14 本の出力の行。
- 製品に新しい環境変数・旗を足していないこと(`flags.sh` が通る)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 確認できなかった物は「確認できない」と書く。
- 段ごとに、やること 1〜5 のどこまで進んだか(完了・途中・未着手)を書く。

## 制約

- 製品(`app/`・`lift/`)は、原因が製品にあるときだけ直す。`tools/e2e/` は、道具の誤りを直すときに直す。
- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。テストのための分岐を製品に入れない。
- Python を使わない(調査と検証にだけ使う)。OCR を使わない。文字は、ゲストのメモリの文字列・窓の表・画素のハッシュで読む。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。
- sleep・nohup・末尾の `&`・先頭の `timeout` は使わない。待つ時は条件で待つ。
- 一時ファイルは `.tmp-*` に置き、終わったら消す。
- commit・PR に、AI が書いたことを示す行を付けない。commit の名義は、環境の設定のまま。
- GitHub に、Issue・ラベル・コメントを書かない。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- ビルドと E2E は、この job の中で行う。長いコマンドは、フォアグラウンドで直接実行し、シェルツールの `timeout` 引数に 5400000 を渡す。背景で起動して待たない。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。`git add` は、自分が変えたパスだけを指定する。

## 他のタスクとの関係

- 起動の getSize の連鎖(#{{263}})とは別。この Issue の本の一部は、その連鎖で落ちることがあるので、受け入れの前に #{{263}} の修正の有無を確かめる。
- 一覧の読み(#{{265}})とは別。
