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

## 前回の試行で分かったこと(run 45594361647)

数値は runner の値(nproc 4、loadavg 2.10 から 3.13)。実行中の rAF は 25 から 75 fps で、120 Hz には届いていない。前回の差分(commit `82a8aeaa`、未 push)は `.prior-work.md` の取り込みで作業ツリーに入る。
- 結果: 受け入れは終了コード 1(`rows 2, accepted false`)。両行とも `servedHeadersMatch` は真、`error` は null、シナリオは 18 本。
- 印の無い失敗は 2 件。
  - R03 `open-demo-project`(両表示): `opened "Song info"` の後、1,125 ms 以内に engine が exited(code 134)。engine のログ: `fle: indirect call to 0xb2844c0 is not a known function entry`(DEDZII の読み込みで止まる。#1413 と同じ文言)。R12 も、コピーの demo で同じ停止。main では、file-open-demo に印 `#1421` が付いた(bc740423)。
  - R18 `startup-no-dropout-10s`(DPR 1): `first idle, then 10 s with no input: underrun quanta 1`。この試行の全体の実行は 2 回とも 1、単独のデバッグの 1 回は 0。DPR 2 は 3 回とも 0。R18 の印は `#{{190}}`。
- R02(#1419): 通った(メニューは画面内、面と重なり、面は応答しない)。印を外す。
- R09(#1392): 通った。判定は画像カーソル(`url(...)`)の有無だけ。鉛筆の資源との照合は未確認。印を外す。
- R13(#1384): DPR 1 は窓の外が 683 px 変わる。DPR 2 は通った。前回(run 45548619833)の DPR 2 の 26 px は、今回は 0。R13 の印は `#{{191}}`。
- R12(#1385): 開いた直後は Recent に名前が在り、再読込の後は無い。印は `#1409`(Recent の保持)。
- R07(#1402): Frame Rate の表示 24 に対し、rAF は 124.8(DPR 1)・123.2(DPR 2)。Memory `100 MB`・CPU `1 %` は値が出た。近くの文字列は符号の断片で、上段の値と結び付けられない。数字は読まない。判定は、FL に渡す入力(envHz・dwmPerSec)が rAF に追随するかで見る(下の R07 の欄)。
- R10(#1382): 窓の表に、テーマの一覧が無い(頁の caption は Presets・Adjustments・Options だけ)。タイルは描画された画素で、スクロールする一覧の中にあり、1 つも押せていない。未確認。
- R11(#1395・#1419): DPR 1 は 7 言語とも、書き込み・再起動の案内・読み戻し(`kept true`)が通る。再読込のたびに engine が exited し、ページの再読込で engine-idle に戻る。Console は 23,102 件。DPR 2 は `de` の再読込の後に exited、`es` は通り、`fr` は General に届かず中断(`ja`・`ko`・`vi`・`zh` は未実施)。Console は 9,079 件。前回は 21,758 件・8,385 件で、数が違う。
- R11 の注意: `rowPasses`(main の bc740423)は、印が有っても `consoleErrors` が 0 であることを求める。#1419 が main に入るまで、R11 の行は落ちる。
- R14(#1386): 両表示で engine が exited(絵は変わる: 327,012 px・1,057,064 px)。R15(#1387): リフ マシーンの窓が出ない(engine は idle)。
- R16(#1406): 3 部品の値は変わる(DPR 1: Tempo 698 px、Song Position 659 px、Pattern Selector 293 px)。`clipboard-status` が 10 秒を超えて残り、stuck 1 で落ちた。
- R06(#1406): prompt は読み 2 回・表示が 1.5 s と 6.5 s で残る。denied は読み 0・表示が残る。granted は読み 4・表示なし。期待(読み 0・表示なし)とずれ、印の通り。
- R01: 窓の縁は、DPR 1 で 2・5・24 px、DPR 2 で 0・0・10 px。判定は「縁が 1 物理 px 以上の窓が 1 つ以上」。窓ごとの値は全て JSON に出す。
- R04: Copy value の文字列 `"0.5"` が `readText()` と一致。R05: ランダムな文字列を受け取った。R08: 枠を持つ窓が無いので `ok`(F5 から F12 の窓の style は 0x94030000 など)。R17: 1,742 Hz・1,696 Hz で underrun 0。
- main が変わった: bc740423 で、`--only`・judge.jq と `rowPasses` の印の規則(印の付いたシナリオは、pageErrors・consoleErrors・failedRequests が 0 なら通る)・file-open-demo の印 `#1421` が入った。前回の差分と衝突したら、main の規則を採る。
- 前々回の試行(run 45548619833)の記録は、下の節に残す。

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
- 開いている Issue(この本文を直した時点。印を付ける番号はこの一覧に限る): #1382(テーマの切り替えで止まる)、#1386(コード進行ツールの Start creating で止まる)、#1387(リフ マシーンが窓を出さない)、#1395(英語以外の言語での起動)、#1406(クリップボードの権限と表示)、#1409(Recent の保持)、#1419(Browser の面の iframe が出す console エラーの洪水)、#1421(DEDZII の曲で engine が止まる)、#{{190}}(起動の後の無入力 10 秒の途切れ)、#{{191}}(Settings の General のスクロールで、窓の外が変わる)。
- 閉じている Issue(#1375・#1384・#1385・#1391・#1392・#1400・#1402・#{{164}} など、#1383 の言語の保持・#1389 のクリップボード・#1390 の標準のカーソル・#1346 のトップバーのメニュー・#1365 と #1372 と #1373 の demo など)は、直ったものとして扱う。通らなければ、印を付けずに落とし、報告に書く。閉じている Issue の番号は、印に使わない。

## 読む物

- `pm-work/ledger/instructions.tsv`(依頼者の発言の全件。`text` 列)
- `tools/e2e/`(#{{164}} の成果。`run.sh`・`run.mjs`・`judge.jq`・`lib/`・`scenarios/smoke/`。main の bc740423 の判定の規則)
- `tools/clipboard/measure.mjs`・`tools/feat-c6-popup-menus/measure.mjs`・`tools/topbar-parts/`・`tools/browser-panel/`・`tools/topbar-metrics/measure.mjs`(#1402 の判定の方法)・`tools/p3-audio-probe/measure.mjs`・`tools/smoke/welcome-flow.mjs`(既存の操作と検査の型)
- `pm-work/created-issues.tsv`(下書き番号と Issue 番号の対応)

## やること

1. シナリオを作る(`tools/e2e/scenarios/regression/`。1 シナリオ 1 ファイル)。下の一覧の 18 本(クリップボードは 3 本に分ける。合計 15 本以上)。各シナリオに「期待」を、測れる形で書く。
   - R01 `hidpi-edge-and-text`: DPR 1 と DPR 2 で、窓の縁の幅が物理の画素で 1 以上、画面の文字とキャンバスの大きさが、論理の大きさ × DPR と一致(ぼやけ・拡大が無い)。
   - R02 `browser-sounds-stacking`: File メニューを開いた時、メニューが Browser の Sounds の面より前にあり、メニューの下部が隠れていない(窓の表の前後の順と、メニューの矩形が画面内)。Browser の面を開くので、`consoleErrors` は 0 を求める。印は付けない(前回は通った)。
   - R03 `open-demo-project`: File > Open で評価用のプロジェクト(demo の曲)を 1 つ開く。エンジンは止まらず、窓の題名が変わり、engine-idle のまま 20 秒続く。(`expectFail: '#1421'`。DEDZII の読み込みで止まる。main の file-open-demo にも同じ印が付いた)
   - R04 `clipboard-copy`: ノブの Copy(FL の Copy の項目)で、ページの `navigator.clipboard.readText()` が、エンジンが渡した文字列と一致する(#1389 の経路)。
   - R05 `clipboard-paste`: `navigator.clipboard.writeText('<乱数>')` の後、FL の名前の編集欄で Ctrl+V。エンジンが、その文字列を受け取る。
   - R06 `clipboard-permission-knob`: ノブを右クリックする。権限の 3 通り(`prompt`・`denied`・`granted`)。右クリックの間に読みが 0 回、表示が出ない、5 秒後に表示が残らない。(`expectFail: '#1406'`)
   - R07 `topbar-numbers`: 上部バーの Memory Usage・CPU Load・Frame Rate の文字が、0 でない実際の値。数字の読み取りは使わない(前回、近くの文字列は符号の断片だった)。判定は #1402 の道具(`tools/topbar-metrics/measure.mjs` の方法)に合わせる: FL に渡す入力(envHz・dwmPerSec)が、ページの rAF(rafPerSec)に ±15% 以内で追随する。印は付けない(#1402 は閉じている)。道具が使えない時は、R07 を `skipped` にし、`notes` に理由を書く。
   - R08 `cursor-window-edge`: FL が端でサイズ変更のカーソルを要求しない窓は、`default` のまま(FL が決める)。サイズ変更の枠(`WS_THICKFRAME`・`0x40000`)を持つ窓が在る時だけ、その窓の端にマウスを置き、ページの `#desktop` の cursor が `ew-resize`・`ns-resize` になることを確かめる。枠を持つ窓が 1 つも無い時は `ok`(`notes` に「枠を持つ窓が無い。FL が決める」)。
   - R09 `cursor-pencil-mode`: Piano roll の Pencil Mode で、カーソルが鉛筆の画素(FL の資源)になる。印は付けない(前回は通った。判定は画像カーソルの有無だけで、鉛筆の資源との照合は未確認)。
   - R10 `theme-switch-all-13`: テーマを全 13 種、順に切り替える。どの切り替えでも engine は止まらず、窓の画素が変わる。(`expectFail: '#1382'`。タイルが見つからない。道具の問題と、#1382 の再開の結果で外す)
   - R11 `language-switch-all-7`: 言語を全 7 種、順に切り替える。再読込の後も、選んだ言語が保たれる。(`expectFail: ['#1395', '#1419']`。印は 2 つ。#1395 は英語以外での起動。#1419 は Browser の面の console。再読込の後に engine が止まる問題は #1395 の範囲。7 言語を全て試し、途中で止めない)
   - R12 `recent-persists`: File > Open で開いたプロジェクトが、再読込の後も Recent に残る。(`expectFail: '#1409'`。Recent の保持は #1409 の範囲。#1385 は閉じている)
   - R13 `settings-general-scroll`: F10 の Settings の General を開き、スクロールしても描画が崩れない(画素の比較で、スクロールの前後の差が、窓の外に出ない)。(`expectFail: '#{{191}}'`。DPR 1 で窓の外が変わる。#1384 は閉じている)
   - R14 `chord-progression-start`: Piano roll のコード進行ツールの Start creating を押す。エンジンは止まらず、Piano roll にノートが作られる。(`expectFail: '#1386'`)
   - R15 `riff-machine-open`: リフ マシーンを起動する。窓が出る。(`expectFail: '#1387'`)
   - R16 `topbar-menu-value`: トップバーの Tempo・Song Position・Pattern Selector を右クリックしてメニューを出し、項目を選ぶと、部品の値が変わる。(`expectFail: '#1406'`。右クリックで出る `clipboard-status` が消えないのは #1406 の症状。値の検査は通る)
   - R17 `mouse-1000hz-no-dropout`: 再生しながら、マウスを 1000 Hz 以上で 20 秒動かす。音の途切れ(underrun)は 0。(`tools/p3-audio-probe/` の計数を使う)
   - R18 `startup-no-dropout-10s`: 起動して最初の idle の後、10 秒の間、音の途切れは 0。(`expectFail: '#{{190}}'`。DPR 1 で途切れが 1 件出た。DPR 2 は 3 回とも 0。直近 60 秒の判定は、この Issue では足さない)
   - 各シナリオの名前・期待・印は、シナリオの先頭に、1 つの定数の表として書く。
2. 印の使い方。
   - 印(`expectFail`)の値は、上の「開いている Issue」の番号に限る(`'#番号'` の形)。`ALLOWED_MARKS` に、その一覧を入れる。一覧に無い失敗は、印を付けずに落とす。報告に書く。
   - 印の付いたシナリオは、`ok` と `stuckToasts` を問わない。`pageErrors`・`consoleErrors`・`failedRequests` は 0 を求める(main の `rowPasses` と `judge.jq` と同じ規則)。
   - 印の付いたシナリオが通ったら、警告として stderr に出す(`warning: expectFail scenario passed: <name> <issue>. remove the mark`)。失敗にはしない。
   - JSON の行の `expectFail` の欄に、印の一覧(`[{"name","issue"}]`。R11 は 2 件)を出す。
3. 出力。表示ごとに 1 行の JSON(`4k-dpr1` と `4k-dpr2` の 2 行)。`scenarios` の各件に `expectFail`(印か null)を出し、行の末尾に `expectFail`(印の一覧)を出す。
4. シナリオが実行時間を超えないこと。1 表示あたり 40 分以内(18 本)。長ければ、シナリオの待ち(条件で待つ)を縮める。固定の待ち時間は使わない。
5. 受け入れの node の式を、合成データ(印の有無、`consoleErrors` が 0 と 1 の行、`servedHeadersMatch` が偽の行)で確かめてから commit する。確かめに使う一時ファイルは `.tmp-*` に置き、終わったら消す。

## 印の一覧(この本文を直した時点)

| シナリオ | 印 | 内容 |
|---|---|---|
| R03 `open-demo-project` | #1421 | DEDZII の読み込みで engine が止まる(main の file-open-demo にも付く) |
| R06 `clipboard-permission-knob` | #1406 | 権限の表示と読みの回数 |
| R10 `theme-switch-all-13` | #1382 | テーマの切り替えで止まる。タイルを押す手段が未確定 |
| R11 `language-switch-all-7` | #1395・#1419 | 英語以外での起動・Browser の面の console(2 件) |
| R12 `recent-persists` | #1409 | Recent が再読込をまたいで残らない |
| R13 `settings-general-scroll` | #{{191}} | DPR 1 で、スクロールの時に窓の外が変わる |
| R14 `chord-progression-start` | #1386 | Start creating で engine が exited |
| R15 `riff-machine-open` | #1387 | リフ マシーンの窓が出ない |
| R16 `topbar-menu-value` | #1406 | `clipboard-status` が消えない |
| R18 `startup-no-dropout-10s` | #{{190}} | 起動の後の無入力の 10 秒で、途切れが 1 件出る(DPR 1) |

印を付けないシナリオ: R01・R02・R04・R05・R07・R08・R09・R17。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && (cd tools/e2e && npm ci && npx playwright install chromium) && bash tools/e2e/run.sh --scenarios regression --displays 4k-dpr1,4k-dpr2 > "$RUNNER_TEMP/e2e-reg.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const good=L.length===2&&L.every((r)=>r.servedHeadersMatch===true&&r.error===null&&Array.isArray(r.expectFail)&&r.expectFail.every((x)=>x&&typeof x.name==="string"&&typeof x.issue==="string")&&r.scenarios.length>=15&&r.scenarios.every((s)=>s.expectFail!==null?(s.pageErrors===0&&s.consoleErrors===0&&s.failedRequests===0):(s.ok===true&&s.pageErrors===0&&s.consoleErrors===0&&s.stuckToasts===0&&s.failedRequests===0)));process.stdout.write(JSON.stringify(L.map((r)=>({display:r.name,expectFail:r.expectFail})))+"\n");process.exit(good?0:1)' "$RUNNER_TEMP/e2e-reg.jsonl"
```

判定の基準:
- `run.sh` の終了コードが 0(main の `rowPasses` の規則)。JSON の行が 2 件(`4k-dpr1`・`4k-dpr2`)。`servedHeadersMatch` が真。
- 各行で、シナリオが 15 本以上。
- 印の無いシナリオが全て `ok` が真、`stuckToasts` が 0、`pageErrors`・`consoleErrors`・`failedRequests` が 0。
- 印の付いたシナリオは、`pageErrors`・`consoleErrors`・`failedRequests` が 0(`ok` は問わない)。
- 印の一覧(行の `expectFail` の欄)は、印ごとに `{name, issue}` の 1 件。R11 は 2 件。印の番号は、上の「印の一覧」の番号に限る。
- 印の付いたシナリオは、結果を報告に書く(通った物は、印を外す提案を書く)。
- `judge.jq`(deploy の E2E の門)は、smoke の 3 本を見る。この集合の判定は、`run.sh` の終了コード(`rowPasses`)で行う。

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
- 最初に `git pull --no-rebase --no-edit` で main を取り込む(bc740423 の判定と `--only` が入っている)。前回の差分と衝突したら、main の規則を採る。
- 順序: (1) 基盤(#{{164}} の `tools/e2e/`)と main の判定を読み、smoke の 3 本が動くことを確かめる。(2) 依頼者の報告を、1 件ずつシナリオにする。(3) 2 表示で通す。
- engine の作り直しは約 35 分。ビルドは、フォアグラウンドで、シェルツールの `timeout` 引数に 5400000 を渡して待つ。背景で起動して待たない。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: #{{164}}(E2E の基盤。閉じている)。
- R11 の行は、#1419 が main に入るまで、`consoleErrors` の判定で落ちうる(`rowPasses` は印の有無に関わらず 0 を求める)。その時は、R11 が #1419 待ちだと報告に書いて終える。判定の式は緩めない。
- #1421(DEDZII の停止)の印は、main の file-open-demo と R03 の両方に付く。#1421 が直ったら、両方の印を外す。
- #1406 が直ったら、R06 と R16 の印を外す。#1419 が直ったら、R11 の #1419 の印を外す(R11 の `consoleErrors` も 0 になる)。#1395 が直ったら、R11 の #1395 の印を外す。
- #1382・#1386・#1387・#1409 が直ったら、R10・R14・R15・R12 の印を外す。
- #{{190}}(起動の後の途切れ)と #{{191}}(Settings の窓の外)は、それぞれの Issue の作業が R18 と R13 の印を外す。
- #1419(Browser の面の iframe が出す `Blocked script execution` の洪水)は、この Issue と別に直す。#1419 の受け入れに R11 の `consoleErrors` を足すのは、この Issue が main に入った後(#1419 の本文に書く)。
- 上の「開いている Issue」の番号は、作業の時点で開いている物に限る。閉じている物の印は付けない。
