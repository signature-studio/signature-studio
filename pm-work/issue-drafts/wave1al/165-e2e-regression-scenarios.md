---
draft: "165"
title: "E2E のシナリオ集: 依頼者が報告した全ての操作を、回帰として E2E に入れる(4K・DPR 2 通り)"
type: Task
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "133"
blocked_by: ["164"]
labels: ["verify"]
size: L
priority: Urgent
project: signature-studio/3
---

## 目的

依頼者が報告した操作を、1 件ずつ E2E のシナリオにする。直した後に同じ操作を繰り返し、壊れていないことを、本番と同じ配信・4K・120 Hz の条件で確かめられるようにする。

この Issue の目標は次の 3 つ。
1. 依頼者の報告(`pm-work/ledger/instructions.tsv` の、依頼者の発言の全件)のうち、操作で確かめられる物を、シナリオにする。1 報告 = 1 シナリオを基本にする。
2. 各シナリオの「期待」(合格の条件)を、測れる形で書く(ゲストのメモリの走査・エンジンのログ・DOM・窓の表・画素の比較のいずれか)。
3. まだ直っていない物は、印(`expectFail: '#番号'`)を付け、直ったら印を外す。

## 背景

- #{{164}} が、E2E の基盤(4K・120 Hz の画面、本番と同じ配信、操作の部品、停止と消えない表示の検出)を作る。この Issue は、その上にシナリオを足す(段階 2)。
- 依頼者の報告の一覧(依頼者の発言の原文は台帳にある。読むこと)。依頼者が報告したのは、次の操作。
  - HiDPI の表示(ぼやけ・拡大の無さ。窓の縁の幅)
  - Browser の Sounds タブの前後関係(File メニューを開いて、下部が隠れない)
  - 評価用のプロジェクト(demo の曲)を開く
  - クリップボード(Copy・Paste・権限と表示)
  - 上部バーの数値(Memory・CPU・Frame Rate が、実際の値)
  - マウスカーソル(窓の端の矢印・Piano roll の鉛筆)
  - テーマの切り替え(全 13)
  - 言語の切り替え(全 7。再読込の後も保持)
  - Recent の保持(再読込の後も残る)
  - Settings の General のスクロール
  - コード進行ツール(Start creating)
  - リフ マシーン
  - トップバーのメニュー(Tempo・Song Position・Pattern Selector の右クリック)
  - マウスを 1000 Hz で動かしても、音が途切れない
  - 起動して 10 秒後の途切れが 0
- 開いている Issue(作成時点。印を付ける番号はこの一覧に限る): #1382(テーマの切り替えで止まる)、#1384(Settings の General のスクロール)、#1385(Recent・Presets などの保持)、#1386(コード進行ツール)、#1394(コード進行ツールの Start creating で、ノートが作られない)、#1387(リフ マシーン)、#1400(リフ マシーンの Sytrus のプリセット)、#1391(上部バーの Memory・CPU・Frame Rate)、#1402(上部バーの CPU Load と Frame Rate の段階 2)、#1392(FL 自身のカーソル)、#1375(起動直後の途切れの段 B)、#{{163}}(クリップボードの権限と表示)。
- 閉じている Issue(#1383 の言語の保持・#1389 のクリップボード・#1390 の標準のカーソル・#1346 のトップバーのメニュー・#1365 と #1372 と #1373 の demo など)は、直ったものとして扱う。通らなければ、印を付けずに落とし、報告に書く。

## 読む物

- `pm-work/ledger/instructions.tsv`(依頼者の発言の全件。`text` 列)
- `tools/e2e/`(#{{164}} の成果。`run.sh`・`lib/`・`scenarios/smoke/`)
- `tools/clipboard/measure.mjs`・`tools/feat-c6-popup-menus/measure.mjs`・`tools/topbar-parts/`・`tools/browser-panel/`・`tools/p3-audio-probe/measure.mjs`・`tools/smoke/welcome-flow.mjs`(既存の操作と検査の型)
- `pm-work/created-issues.tsv`(下書き番号と Issue 番号の対応)

## やること

1. シナリオを作る(`tools/e2e/scenarios/regression/`。1 シナリオ 1 ファイル)。下の一覧の 18 本(クリップボードは 3 本に分ける。合計 15 本以上)。各シナリオに「期待」を、測れる形で書く。
   - R01 `hidpi-edge-and-text`: DPR 1 と DPR 2 で、窓の縁の幅が物理の画素で 1 以上、画面の文字とキャンバスの大きさが、論理の大きさ × DPR と一致(ぼやけ・拡大が無い)。
   - R02 `browser-sounds-stacking`: File メニューを開いた時、メニューが Browser の Sounds の面より前にあり、メニューの下部が隠れていない(窓の表の前後の順と、メニューの矩形が画面内)。
   - R03 `open-demo-project`: File > Open で評価用のプロジェクト(demo の曲)を 1 つ開く。エンジンは止まらず、窓の題名が変わり、engine-idle のまま 20 秒続く。
   - R04 `clipboard-copy`: ノブの Copy(FL の Copy の項目)で、ページの `navigator.clipboard.readText()` が、エンジンが渡した文字列と一致する(#1389 の経路)。
   - R05 `clipboard-paste`: `navigator.clipboard.writeText('<乱数>')` の後、FL の名前の編集欄で Ctrl+V。エンジンが、その文字列を受け取る。
   - R06 `clipboard-permission-knob`: ノブを右クリックする。権限の 3 通り(`prompt`・`denied`・`granted`)。右クリックの間に読みが 0 回、表示が出ない、5 秒後に表示が残らない。(`expectFail: '#{{163}}'`)
   - R07 `topbar-numbers`: 上部バーの Memory Usage・CPU Load・Frame Rate の文字が、0 でない実際の値。Frame Rate は、ページの rAF の実測と 10% 以内で一致する(`expectFail: '#1391'`・`'#1402'` のうち、直っていない方)。
   - R08 `cursor-window-edge`: 窓の端にマウスを置くと、ページの `#desktop` の cursor が、サイズ変更の矢印の値(`ew-resize`・`ns-resize` など)になる。
   - R09 `cursor-pencil-mode`: Piano roll の Pencil Mode で、カーソルが鉛筆の画素(FL の資源)になる。(`expectFail: '#1392'`)
   - R10 `theme-switch-all-13`: テーマを全 13 種、順に切り替える。どの切り替えでも engine は止まらず、窓の画素が変わる。(`expectFail: '#1382'`)
   - R11 `language-switch-all-7`: 言語を全 7 種、順に切り替える。再読込の後も、選んだ言語が保たれる。
   - R12 `recent-persists`: File > Open で開いたプロジェクトが、再読込の後も Recent に残る。(`expectFail: '#1385'`)
   - R13 `settings-general-scroll`: F10 の Settings の General を開き、スクロールしても描画が崩れない(画素の比較で、スクロールの前後の差が、窓の外に出ない)。(`expectFail: '#1384'`)
   - R14 `chord-progression-start`: Piano roll のコード進行ツールの Start creating を押す。エンジンは止まらず、Piano roll にノートが作られる。(`expectFail: '#1386'`・`'#1394'` のうち、直っていない方)
   - R15 `riff-machine-open`: リフ マシーンを起動する。窓が出る。(`expectFail: '#1387'`・`'#1400'` のうち、直っていない方)
   - R16 `topbar-menu-value`: トップバーの Tempo・Song Position・Pattern Selector を右クリックしてメニューを出し、項目を選ぶと、部品の値が変わる。
   - R17 `mouse-1000hz-no-dropout`: 再生しながら、マウスを 1000 Hz 以上で 20 秒動かす。音の途切れ(underrun)は 0。(`tools/p3-audio-probe/` の計数を使う)
   - R18 `startup-no-dropout-10s`: 起動して最初の idle の後、10 秒の間、音の途切れは 0(`expectFail: '#1375'` は、直っていない時だけ)。
   - 各シナリオの名前・期待・印は、シナリオの先頭に、1 つの定数の表として書く。
2. 印の使い方。
   - 印(`expectFail`)の値は、上の「開いている Issue」の番号に限る(`'#番号'` の形)。一覧に無い失敗は、印を付けずに落とす。報告に書く。
   - 印の付いたシナリオが通ったら、警告として stderr に出す(`warning: expectFail scenario passed: <name> <issue>. remove the mark`)。失敗にはしない。
   - JSON の `expectFail` の欄に、印の一覧(`[{"name","issue"}]`)を出す。
3. 出力。表示ごとに 1 行の JSON(`4k-dpr1` と `4k-dpr2` の 2 行)。形は #{{164}} と同じで、`scenarios` の各件に `expectFail`(印か null)を出し、行の末尾に `expectFail`(印の一覧)を出す。
4. シナリオが実行時間を超えないこと。1 表示あたり 40 分以内(18 本)。長ければ、シナリオの待ち(条件で待つ)を縮める。固定の sleep は使わない。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && (cd tools/e2e && npm ci && npx playwright install chromium) && bash tools/e2e/run.sh --scenarios regression --displays 4k-dpr1,4k-dpr2 > "$RUNNER_TEMP/e2e-reg.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));process.exit(L.length===2&&L.every((r)=>r.servedHeadersMatch===true&&Array.isArray(r.expectFail)&&r.scenarios.length>=15&&r.scenarios.every((s)=>s.expectFail!==null||s.ok===true))?0:1)' "$RUNNER_TEMP/e2e-reg.jsonl"
```

判定の基準:
- `run.sh` の終了コードが 0。JSON の行が 2 件(`4k-dpr1`・`4k-dpr2`)。
- 各行で、シナリオが 15 本以上。印の無いシナリオが全て `ok` が真。`expectFail` の一覧を出している。`servedHeadersMatch` が真。
- 印の付いたシナリオは、結果を報告に書く(通った物は、印を外す提案を書く)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 報告に書く: シナリオの一覧(名前・期待・印)、印の付いたシナリオの結果、印の付いていない失敗の全文(何が起きたか。既存の Issue の番号があれば書く)。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。

## 制約

- 製品(`app/`・`lift/`)は変えない(読むだけ)。
- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。テストのための分岐を製品に入れない。
- 印を付けるために、判定を緩めない。期待を変えない。印の番号は、上の一覧に限る。
- Python を使わない。OCR を使わない(FL の画面の文字は、窓の表とゲストのメモリの走査で読む)。
- ベンダー名、Cloudflare の識別子を、ファイルに書かない。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。待つ時は、条件で待つ。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- 開始の時刻を、最初に `date -u` で記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える。
- 順序: (1) 基盤(#{{164}} の `tools/e2e/`)を読み、smoke の 3 本が動くことを確かめる。(2) 依頼者の報告を、1 件ずつシナリオにする。(3) 2 表示で通す。
- engine の作り直しは約 35 分。ビルドは、フォアグラウンドで、シェルツールの `timeout` 引数に 5400000 を渡して待つ。背景で起動して待たない。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: #{{164}}(E2E の基盤)。
- #{{163}}(クリップボードの権限と表示)が直ったら、R06 の印を外す。
- 上の「開いている Issue」の番号は、作業の時点で開いている物に限る。閉じている物の印は付けない。
