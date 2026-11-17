# ci1 → PM 報告 0277-1（第一報）: 新本番 bundle `fl-engine-ci-d9c160aa`（Worker 51f102d8、CI 42137885173、HEAD 0b228624）の実ブラウザ確認

## 先頭の事実（実エンジン・実ブラウザ Chrome 156 / Firefox 158。代役・再生・推測なし）
- bundle は配信バケット `fl-web-assets` から**読み取りで取得**（`wrangler r2 object get`、局所の再ビルドなし）。manifest の 4 ファイルの size と sha256 が**全て一致**: wasm `d6e3b018accb…` 142,587,221 B、glue `8dc2abf01efc…` 743,276 B、seed `f7726113ae34…`、dll `2f494419c1ad…`。比較の相手 `fl-engine-ci-ff5ee429` も R2 に残っていて、同様に全て一致。局所の `.br` は私が品質 6 で作った物（転送サイズは本番と違う。内容は同じ）。
- ページは CI の headSha **0b228624** の `app`（= Worker 51f102d8 のビルド）。新旧の bundle を同じページで配信して比べた。
- Firefox は `DISPLAY=:77`（Xvfb）。preflight の beacon: `webglRenderer` = `llvmpipe, or similar`、`desk.mode` = `gl`（2D 表示ではない）。Chrome の beacon の `webglRenderer` は SwiftShader（ヘッドレス）、`desk.mode` = gl。
- **この報告に含まれない物（未実施）: Export の sha256、60 s 再生、Mixer ドラッグ A/B。** 実行中 / これから（最終 11:30、全表 11:15）。

## 判定（第一報の時点）
| 判定基準 | 結果 |
|---|---|
| 停止なし | **11 シーン + plus-more-plugins + Add メニュー fade + F8 picker 2 種: Chrome 15/15 ok、Firefox 15/15 ok（停止 0。d7 は意図した終了）**。ただし下の『自己申告の逸脱』の 2 件（d1・d10・d11 の入力欠落による無効 run、Firefox d7 の WebDriver 切断 1 回）を読むこと |
| File メニュー ×100 | **Chrome 100/100、Firefox 100/100**（取りこぼし 0、popup 欠落 0、不可視 0） |
| first idle が ff5ee429 に対し +500 ms 以内 | 下表。**Firefox: 新が 376 ms 速い（全 6 組の中央値）、Chrome: 新が 1,432 ms 速い（全 6 組）**。どちらも +500 ms 以内。ただし Chrome は run 3〜5 で load が 12〜20 に上がった（参考値）。静かな run だけ（load ≤12・I/O 圧 <1）では Chrome 新 6,132 / 旧 6,574（n=2/2）、Firefox 新 4,945 / 旧 5,297（n=4/3） |
| Export の sha256 | **未実施（実行中）** |

## first idle（単独 run・冷間・毎回新しいプロファイル。旧 ff5ee429 と新 d9c160aa を交互。全 run の load・I/O 圧は `data/idle-*.tsv`、`data/RESULTS-partial-1030.md`）
| ブラウザ | 旧 ff5ee429 の中央値 (n) | 新 d9c160aa の中央値 (n) | 差 | 備考 |
|---|---|---|---|---|
| Firefox（WebGL、gl） | 5,276 ms (6) | **4,900 ms (6)** | −376 ms | 全 12 run が load 8〜15、I/O 圧 avg10 ≤0.1。全て gl |
| Chrome（gl） | 7,946 ms (6) | **6,514 ms (6)** | −1,432 ms | run 1・2 は load 6〜9（静か）。run 3〜6 は load 9〜20（他の作業者の job）で、新 10,723 / 9,682 / 6,269 / 6,411 ms、旧 9,349 / 9,766 / 9,122 / 6,414 ms。静かな window の取り直しは再生・Mixer の後に入れる |
- Firefox の旧新の個々の値（ms、前後交互）: 旧 5,786 / 5,254 / 4,721 / 4,454 / 5,297 / 5,934、新 4,890 / 4,778 / 4,981 / 4,909 / 4,701 / 5,718。
- P1（起動 10 s 以内）: この環境の値としては、単独 run（Chrome 最大 10,723 ms、load 12.6〜15.4 の run）も 15 シーンの first idle（最大 9,743 ms）も 11 s を超えない。**オーナーのマシンの値は 0（未測定）**。

## 11 シーン + 追加（新 bundle、Chrome / Firefox。各行の load と I/O 圧は `data/scenes.json`）
- ok: d1 sampler-envelope、d2 browser-folder、d3 settings-all-tabs、d4 playlist-middle-drag、d5 mixer-front、d6 playlist-close、d7 main-close（意図した終了）、d8 file-menu-100（100/100）、d9 keys、d10 pianoroll-f12、d11 menu-hover-escape、plus-more-plugins、add-menu-simple-fade、picker-f8-fade、picker-cost。**Chrome も Firefox も全て ok、停止 0**。
- 2 ジョブ並行で流したので、シーン表の first idle は参考値（load 8〜28）。

## alpha7（Chrome、1920x1080 のゲスト）
- 主窓 `TFruityLoopsMainForm 'FL Studio 2026'` は **@0,0 1920x1080**（旧 df0512b0 は @−5,−6 1930x1092）。スクリーンショット `shots/alpha7-d9c160aa-main-1920x1080.png`: ツールバーが上端から始まり、左のブラウザ枠・チャンネルラック・ミキサー（Master と Insert 1〜9）・プレイリスト（Track 1〜19）・右上の最小化/最大化/閉じるが全て見える。ff5ee429 の時の見え方と同じ。ブラウザ枠の中の黒は旧新とも同じ。

## 自己申告の逸脱
1. **d1・d10・d11 の最初の run は無効だった（私の誤り）。** キットの `proj`（配信する /assets/proj/）に `Sampler.flp` と `SHP_Sawing Synth.wav` が無く、404 の本文（35 バイト）が『Sampler.flp』として開かれた。d11 は Chrome で新旧 bundle とも、Firefox で新 bundle が、`CompareStringW` の名前つき停止（exit 134）になった。**d1・d10 は『ok』と出たが、Sampler を操作していない**無効 run。本物の Sampler.flp（git 管理の `docs/investigations/crash1/tools/Sampler-project/` と同一）を配信して**再実行し、全て ok（上の表）**。d11 は 207 s → 27 s で ok（Chrome・Firefox）。無効だった run は `out/invalid-noproj/` に残し、結果表には数えない。再発防止: `verify-bundle.sh proj` と、preflight の『4 ファイルが 200 とサイズで返る』検査（違えば rc 3 で停止）。
2. **Firefox の d7 で 1 回だけ WebDriver が切れた**（`invalid session id`、load 17.2、2 ジョブ並行）。エンジンの終了は読めていない。同じ条件で 3 回再実行して**全て ok（意図した終了）**。再現しなかった（4 回中 1 回）。原因は不明。結果表は最新の run。判定は『意図した終了後の切断』と確認できた物ではないので、『ok、ただし 1/4 で WebDriver 切断（原因不明）』と書く。
3. 局所の `.br`（品質 6）は私の作成物。他の作業者の job で load が上下した（run ごとの値は表に付記）。
4. 前の stretch の再発: なし（`sleep` / `&` / `pkill -f` / `bash` ツールは今回使っていない）。

## 次
Export（Chrome、参照プロジェクト、Ctrl+R、sha256）を今実行中。続けて再生 60 s（Chrome・Firefox、underrun と Worker busy ms/s）、Mixer ドラッグ A/B（前後交互 2 往復、Chrome・Firefox）、静かな window での Chrome first idle の取り直し。全表 `out/RESULTS.md` を 11:15 までに。

## 成果物（`lift/out-git/ci1-results/0277/`）
`REPORT-0277-1-to-PM.md`、`kit/`（検証キット一式。README に順番と判定基準）、`shots/`、`data/`（`RESULTS-partial-1030.md`、`scenes.json`、`idle-*.tsv`、`steps.log`、manifest 照合 `fetch-*.txt`、`preflight.txt`）。
