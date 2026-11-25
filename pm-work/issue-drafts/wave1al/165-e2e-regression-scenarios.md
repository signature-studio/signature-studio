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

## 前回の試行で分かったこと(run 45548619833)

数値は runner の値(nproc 4)。前回の作業の差分(`tools/e2e/` の変更)は、`.prior-work.md` の取り込みで作業ツリーに入る。

- 作ったもの: シナリオ 18 本(R01〜R18)を `tools/e2e/scenarios/regression/` に、1 報告 1 ファイルで置いた。印(`expectFail`)は 10 件。`run.mjs` に、印の一覧・印が通った時の警告・行の件数の引数化・開発用の `--only` を足した。`lib/proxy.mjs`(新規)を足した。製品(`app/`・`lift/`・`ci/`)は変えていない。
- 受け入れは終了コード 1 で落ちた。原因は、印の無い失敗が 2 件(両方の表示)。
  - R08 `cursor-window-edge`: 「FL 自身の窓に、サイズ変更の枠(`WS_THICKFRAME`・`0x40000`)を持つ窓が 1 つも無い」。見た窓の style は `0x94030000`・`0x9c030000`・`0x94000000`・`0x54000000`。Piano roll は子で、枠が無い。
  - R11 `language-switch-all-7`: 7 言語とも、再読込の後に engine が `exited`(英語の再読込で回復)。DPR 2 では de が止まり、es は止まらず、fr は Settings の General に届かず中断。
- R11 の `console.error`: DPR 1 で 21,758 件・DPR 2 で 8,385 件。全て `Blocked script execution in 'about:srcdoc'`(Browser の面の iframe。サンドボックス)。
- R11 の保存は成立: 7 言語とも、書き込み(`Program language`)・再起動の案内・読み戻し(保たれた値)が通った。再読込の後の停止は、製品の動作。#1395 は開いている(閉じた物として扱わない)。
- 音声の worklet が HTTP 403 で落ちた(ページの追加ヘッダーが worklet の取得に届かない)。そのため R17・R18 が `status: error` になった。harness の `lib/proxy.mjs` が、トークンの無い要求にトークンを付けて解決した(本番の Cloudflare Access の端がページの全要求に付けるのと同じ動き。Worker の検査は変えていない)。その後、R17 は 7,070 quanta・1,786 Hz のマウスで underrun 0。
- R01: DPR 2 で、子窓の縁が 0・0・10 px。判定は「窓ごとに 1 px 以上」ではなく「窓の縁が 1 物理 px 以上の窓が、最大で 1 つ以上」。窓ごとの値は全て出す。
- R04: Copy value の文字列 `"0.5"` が `readText()` と一致。メニューの Copy はキャプションが無いので、ノブのメニューの左上から +60・+253 CSS px(DPR 1)の位置を押す。FL の文言は「Copy value」。
- R06(#1406): 権限 3 通りの右クリックで、prompt は読み 2 回・表示 1 件、denied は表示 1 件、granted は読み 4 回。期待(読み 0・表示なし)とずれ、印の通りに落ちた。
- R07(#1391・#1402): MB・% の文字列が見つからない。FPS の表示 24 と rAF の 123 が一致しない。FPS の文字列の特定は未確定。
- R09(#1392): Piano roll の Pencil Mode で、カーソルは `default`。
- R10(#1382): テーマの一覧は窓の表に出ない(頁の caption は Presets・Adjustments・Options だけ)。タイルは描画された画素で、スクロールする一覧の中にあり、タイルを 1 つも押せていない。未確認。
- R12(#1385): 開いた直後は Recent に名前が在り、再読込の後に無い(保たれていない)。
- R13(#1384): 下へのスクロールで 696 px 動き、DPR 2 では 26 px が窓の外に出た。
- R14(#1386・#1394): 両表示で engine が `exited`。
- R15(#1387): リフ マシーンの窓が出ない。engine は idle。
- R16(#1406): 3 部品の値は変わった(通る)。右クリックで `clipboard-status` の表示が 10 秒を超えて残る(#1406 の症状)。stuck の件数が 1 で、印を #1406 にした。
- R18(#1375): 最初の idle から 10 秒、underrun 0・output running。直近 60 秒は測っていない。#1375 は閉じているので、印は外す。
- 時間: 両表示の全件の実行は約 39 分。受け入れの全体は、エンジンの作り直し(約 26 分)と E2E(約 40 分)で、90 分のコマンドの上限を超えうる。段ごとに分けて実行してよい(判定は全体を 1 回で見る)。
- 残りの論点: R10 のタイルを押す仕組み、R07 の数の読み方(文字列の検索は符号の断片に当たる)、R11 の再読込の停止(製品の仕事)、Browser の面の洪水(#1419)。

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
- 開いている Issue(作成時点。印を付ける番号はこの一覧に限る): #1382(テーマの切り替えで止まる)、#1384(Settings の General のスクロール)、#1385(Recent・Presets などの保持)、#1386(コード進行ツール)、#1394(コード進行ツールの Start creating で、ノートが作られない)、#1387(リフ マシーン)、#1400(リフ マシーンの Sytrus のプリセット)、#1392(FL 自身のカーソル)、#1395(英語以外の言語での起動)、#1402(上部バーの CPU Load と Frame Rate の段階 2)、#1406(クリップボードの権限と表示)、#1419(Browser の面の iframe が出す console エラーの洪水)。この本文を直した時点で、#1391・#1375 は閉じている。
- 閉じている Issue(#1383 の言語の保持・#1389 のクリップボード・#1390 の標準のカーソル・#1346 のトップバーのメニュー・#1365 と #1372 と #1373 の demo など)は、直ったものとして扱う。通らなければ、印を付けずに落とし、報告に書く。

## 読む物

- `pm-work/ledger/instructions.tsv`(依頼者の発言の全件。`text` 列)
- `tools/e2e/`(#{{164}} の成果。`run.sh`・`lib/`・`scenarios/smoke/`)
- `tools/clipboard/measure.mjs`・`tools/feat-c6-popup-menus/measure.mjs`・`tools/topbar-parts/`・`tools/browser-panel/`・`tools/p3-audio-probe/measure.mjs`・`tools/smoke/welcome-flow.mjs`(既存の操作と検査の型)
- `pm-work/created-issues.tsv`(下書き番号と Issue 番号の対応)

## やること

1. シナリオを作る(`tools/e2e/scenarios/regression/`。1 シナリオ 1 ファイル)。下の一覧の 18 本(クリップボードは 3 本に分ける。合計 15 本以上)。各シナリオに「期待」を、測れる形で書く。
   - R01 `hidpi-edge-and-text`: DPR 1 と DPR 2 で、窓の縁の幅が物理の画素で 1 以上、画面の文字とキャンバスの大きさが、論理の大きさ × DPR と一致(ぼやけ・拡大が無い)。
   - R02 `browser-sounds-stacking`: File メニューを開いた時、メニューが Browser の Sounds の面より前にあり、メニューの下部が隠れていない(窓の表の前後の順と、メニューの矩形が画面内)。Browser の面を開くので、`consoleErrors` の判定は `expectFail: '#1419'` で緩める(#1419 が直ったら印を外す)。
   - R03 `open-demo-project`: File > Open で評価用のプロジェクト(demo の曲)を 1 つ開く。エンジンは止まらず、窓の題名が変わり、engine-idle のまま 20 秒続く。
   - R04 `clipboard-copy`: ノブの Copy(FL の Copy の項目)で、ページの `navigator.clipboard.readText()` が、エンジンが渡した文字列と一致する(#1389 の経路)。
   - R05 `clipboard-paste`: `navigator.clipboard.writeText('<乱数>')` の後、FL の名前の編集欄で Ctrl+V。エンジンが、その文字列を受け取る。
   - R06 `clipboard-permission-knob`: ノブを右クリックする。権限の 3 通り(`prompt`・`denied`・`granted`)。右クリックの間に読みが 0 回、表示が出ない、5 秒後に表示が残らない。(`expectFail: '#1406'`)
   - R07 `topbar-numbers`: 上部バーの Memory Usage・CPU Load・Frame Rate の文字が、0 でない実際の値。Frame Rate は、ページの rAF の実測と 10% 以内で一致する(`expectFail: '#1402'`。#1391 は閉じている)。
   - R08 `cursor-window-edge`: FL が端でサイズ変更のカーソルを要求しない窓は、`default` のまま(FL が決める)。サイズ変更の枠(`WS_THICKFRAME`・`0x40000`)を持つ窓が在る時だけ、その窓の端にマウスを置き、ページの `#desktop` の cursor が `ew-resize`・`ns-resize` になることを確かめる。枠を持つ窓が 1 つも無い時は `ok`(`notes` に「枠を持つ窓が無い。FL が決める」)。
   - R09 `cursor-pencil-mode`: Piano roll の Pencil Mode で、カーソルが鉛筆の画素(FL の資源)になる。(`expectFail: '#1392'`)
   - R10 `theme-switch-all-13`: テーマを全 13 種、順に切り替える。どの切り替えでも engine は止まらず、窓の画素が変わる。(`expectFail: '#1382'`)
   - R11 `language-switch-all-7`: 言語を全 7 種、順に切り替える。再読込の後も、選んだ言語が保たれる。(`expectFail: ['#1395', '#1419']`。印は 2 つ。#1395 は英語以外での起動。#1419 は Browser の面の console。再読込の後に engine が止まる問題は #1395 の範囲。7 言語を全て試し、途中で止めない)
   - R12 `recent-persists`: File > Open で開いたプロジェクトが、再読込の後も Recent に残る。(`expectFail: '#1385'`)
   - R13 `settings-general-scroll`: F10 の Settings の General を開き、スクロールしても描画が崩れない(画素の比較で、スクロールの前後の差が、窓の外に出ない)。(`expectFail: '#1384'`)
   - R14 `chord-progression-start`: Piano roll のコード進行ツールの Start creating を押す。エンジンは止まらず、Piano roll にノートが作られる。(`expectFail: '#1386'`・`'#1394'` のうち、直っていない方)
   - R15 `riff-machine-open`: リフ マシーンを起動する。窓が出る。(`expectFail: '#1387'`・`'#1400'` のうち、直っていない方)
   - R16 `topbar-menu-value`: トップバーの Tempo・Song Position・Pattern Selector を右クリックしてメニューを出し、項目を選ぶと、部品の値が変わる。(`expectFail: '#1406'`。右クリックで出る `clipboard-status` が消えないのは #1406 の症状。値の検査は通る)
   - R17 `mouse-1000hz-no-dropout`: 再生しながら、マウスを 1000 Hz 以上で 20 秒動かす。音の途切れ(underrun)は 0。(`tools/p3-audio-probe/` の計数を使う)
   - R18 `startup-no-dropout-10s`: 起動して最初の idle の後、10 秒の間、音の途切れは 0(印は付けない。#1375 は閉じている。直近 60 秒の判定は、この Issue では足さない)。
   - 各シナリオの名前・期待・印は、シナリオの先頭に、1 つの定数の表として書く。
2. 印の使い方。
   - 印(`expectFail`)の値は、上の「開いている Issue」の番号に限る(`'#番号'` の形)。一覧に無い失敗は、印を付けずに落とす。報告に書く。
   - 印の付いたシナリオが通ったら、警告として stderr に出す(`warning: expectFail scenario passed: <name> <issue>. remove the mark`)。失敗にはしない。
   - JSON の `expectFail` の欄に、印の一覧(`[{"name","issue"}]`)を出す。
3. 出力。表示ごとに 1 行の JSON(`4k-dpr1` と `4k-dpr2` の 2 行)。形は #{{164}} と同じで、`scenarios` の各件に `expectFail`(印か null)を出し、行の末尾に `expectFail`(印の一覧)を出す。
4. シナリオが実行時間を超えないこと。1 表示あたり 40 分以内(18 本)。長ければ、シナリオの待ち(条件で待つ)を縮める。固定の sleep は使わない。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && (cd tools/e2e && npm ci && npx playwright install chromium) && bash tools/e2e/run.sh --scenarios regression --displays 4k-dpr1,4k-dpr2 > "$RUNNER_TEMP/e2e-reg.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));process.exit(L.length===2&&L.every((r)=>r.servedHeadersMatch===true&&Array.isArray(r.expectFail)&&r.expectFail.every((x)=>x&&typeof x.name==="string")&&r.scenarios.length>=15&&r.scenarios.every((s)=>s.expectFail!==null||s.ok===true))?0:1)' "$RUNNER_TEMP/e2e-reg.jsonl"
```

判定の基準:
- `run.sh` の終了コードが 0。JSON の行が 2 件(`4k-dpr1`・`4k-dpr2`)。
- 各行で、シナリオが 15 本以上。印の無いシナリオが全て `ok` が真。`expectFail` の一覧を出している。`servedHeadersMatch` が真。
- 印の付いたシナリオは、結果を報告に書く(通った物は、印を外す提案を書く)。
- 印の一覧(`expectFail` の欄)は、印ごとに `{name, issue}` の 1 件。1 つのシナリオに印が 2 つあるとき(R11)は 2 件になる。印の番号は、回帰の集合の `index.mjs` の `ALLOWED_MARKS` に在る物だけ(#1419 を足す)。
- 印の無い失敗は、全て落ちる。印の付いた失敗は、判定の式では落ちない(run.mjs の行の判定の規則)。
- deploy の E2E の門(`tools/e2e/judge.jq`)は、smoke の 3 本を見て、印の有無に関わらず `consoleErrors` が 0 を求める。smoke が #1419 の洪水を出す間、その門は落ちる。この集合の判定には、その門を使わない。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 報告に書く: シナリオの一覧(名前・期待・印)、印の付いたシナリオの結果、印の付いていない失敗の全文(何が起きたか。既存の Issue の番号があれば書く)。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。

## 制約

- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
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
- #1406(クリップボードの権限と表示)が直ったら、R06 と R16 の印を外す。#1419(Browser の面の console。下の「前回の試行」の洪水)が直ったら、R02 と R11 の #1419 の印を外す。#1395 が直ったら、R11 の #1395 の印を外す。
- #1419(Browser の面の iframe が出す `Blocked script execution` の洪水)は、この Issue と別に直す。R02・R11 の `consoleErrors` の判定は、その Issue が直るまで印で緩める。
- 上の「開いている Issue」の番号は、作業の時点で開いている物に限る。閉じている物の印は付けない。
