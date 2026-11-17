# ci1 → PM 途中報 0277-E9-1: E9 の本番 bundle `fl-engine-ci-62098b85`（Worker 2f390ad3、ページ = 38efb398）の実ブラウザ確認（manifest・scenes・Export・first idle・File ×100・alpha7・logo・title まで。pure・lat は alpha1 の窓待ちの後）

## 先頭の事実（実エンジン・実ブラウザ Chrome 156 / Firefox 158。代役・再生・推測なし。Firefox は `DISPLAY=:77`、beacon の webglRenderer = llvmpipe、desk.mode = gl）
- bundle は R2 から**読み取りで取得**し、manifest の 4 ファイルの size と sha256 が**全て一致**: wasm `55db9bdc3363…`（先頭 16 桁 `55db9bdc336329a6`）142,587,278 B、glue `def59af002e3…` 744,670 B、seed `f7726113ae34…`、dll `2f494419c1ad…`（E8 `fl-engine-ci-875bfd62` と同じ手順で E8 も使用）。ページは 38efb398 の app（= Worker 2f390ad3）。
- **E9 の判定項目のうち、ここまでの 4 つは満たした**: 停止なし（15 シーン × Chrome・Firefox = 30/30 ok、結果なしの行 0）、Export の WAV が参照と一致（22,863,828 B、sha256 `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979`、私の `sha256sum` でも同値）、File ×100 は Chrome・Firefox とも 100/100、first idle は E8 比 +500 ms 以内（Chrome +92 ms、Firefox +167 ms）。
- **まだ取っていない（この報告に含まれない）**: `pure`（純再生 60 s、E8 対 E9、UI idles/s・presents/s・仮想 s/実 s・lagCuts/s・書き込み率・underrun・Worker CPU/wall）、`lat`（再生中の File クリック/Esc の true T）、Firefox の pure、ff5ee429 対 E7b の P3。PM の指示（alpha1 の Chrome A/B のため最大 12 分待つ）に従い、開始待ち。

## 判定（E9 対 E8。run ごとの load と I/O 圧は RESULTS-E9-mid.md）
| 項目 | 結果 | 判定 |
|---|---|---|
| 停止なし | 15 シーン × 2 ブラウザ = 30/30 ok（d7 は意図した終了）。logo 6 run・title 6 変種・Export も停止 0 | **満たす** |
| Export sha256 | 22,863,828 B、`381cdb5a…f979` | **満たす** |
| File ×100 | Chrome 100/100、Firefox 100/100 | **満たす** |
| first idle ≤ E8 + 500 ms（単独 6 組ずつ、交互） | Chrome E9 5,128 / E8 5,037 ms（+92、静かな run だけ 5,096 / 4,982 = +114）、Firefox E9 4,456 / E8 4,289 ms（+167）。Chrome は load 5.9〜7.3（I/O 圧 avg10 は 1 run の開始だけ 1.39、他 0）、Firefox は load 4.5〜6.5（I/O 圧 0） | **満たす**（E8 比で遅い向き、+500 に届かない） |

## alpha7: 起動直後の画（Chrome、1920x1080 ゲスト）E8 と E9
- 主窓 `TFruityLoopsMainForm 'FL Studio 2026'` は E8・E9 とも **@0,0 1920x1080**（可視の窓は 1 つで同一）。
- 画素: 2,073,600 画素中 **65 画素だけ違う**（x 1037..1119、y 6..18 = 時計の数字の欄、時間で変わる）。窓の中身の位置は E8 と同じ。スクリーンショット: `e9/shots/alpha7-E9-62098b85-main-1920x1080.png`、差分 `alpha7-E8-vs-E9-diff.png`。

## lid 8（ロゴ窓）の起動直後（Chrome、各 3 run）
- first idle の後の窓表（`deskDumpBase64` の peek、250 ms ごと、first idle から +8 s まで）: **E9・E8 とも 2 窓だけ**（lid 14 `TFruityLoopsMainForm` @0,0 1920x1080、lid 48 `TWelcomeWizard` @523,253 874x573）。**lid 8 は一度も窓表に出なかった**（E9 の first idle 1.7〜1.9 s、E8 1.4〜1.9 s、各 7〜9 サンプル）。wiz0（ウィザードが開いた状態）、その 2.5 s 後、closeWizard の後の窓表も同じ（`LOGO.md`）。つまり『326x460 になりうる』は、**この観測の時間帯（first idle の後）には現れない**。first idle より前の窓（エンジンの起動中）は、窓表が使えるのが first idle の頃からなので**見えていない**（未確認）。

## 最小化 → `#taskbar-restore` → 復元と題 '(none)'（Chrome、本番 bundle E9、6 変種）
- 6 変種（DOM click の待ち 0.1 / 0.5 / 3 / 15 s、Tab → Enter、**CDP の実マウスクリック**）**全て ok**: 最小化後にボタンが出る（x12 y1071 幅 182 高さ 33）、復元後にボタンが hidden になり主窓が @0,0 1920x1080 に戻る、フォーカスは desktop。
- **実マウスクリック（`mouse-3s`）は E9 でも通った**（E8 と同じ結果）。入力 ring との併用で壊れていない。
- 復元後のプレイリスト見出しは **6 変種すべてで『(none)』**（既知の『(none)』の画と 0 画素差、起動直後の『Pattern 1』と 408 画素差）。復元後と起動直後の画の違いは 41,691〜41,698 画素で全て y 0〜102 の行（E8 と同じ: ツールバーの 15 px 右ずれ、ヒント欄、見出し）。**最小化から復元までを 0.1 s にした変種も同じ**（PM の追加依頼: 最小化だけの面は暗い背景で題が見えないので、最も短い 0.1 s と 0.5 s の変種で代える。最小化の直後の復元でも題は『(none)』）。表: `data/TITLE.md`。ブラウザでも FL 自身の再評価と同じ契機（WM_SIZE）と読めるかは、画の側からは区別できない。

## 自己申告の逸脱・未確認
1. 並列に起動したため、サーバー起動が page のビルドより先に走って 1 回失敗した（page ビルド完了後に起動し直した。結果に影響なし）。
2. lid 8 は first idle より前は見ていない（窓表が使えない）。『326x460』は観測できていない。
3. オーナーの機械は 0 件。`pure`・`lat`・Firefox の pure・ff5ee429 対 E7b の P3 は未実施。
4. `sleep`・`&`・`pkill -f`・`bash` ツールは今回は使っていない。

## 成果物（`lift/out-git/ci1-results/0277/e9/`）
`REPORT-0277-E9-1-to-PM.md`、`RESULTS-E9-mid.md`、`shots/`、`data/`（`scenes.json`、`idle-*.tsv`、`title-report.json`、`TITLE.md`、`LOGO.md`、`startup-windows-*.json`、`export.json`、`steps.log`）。キットは `../kit/`（`logo`・`lat`・`pure` モード、`startup-windows.mjs`・`logocheck.py`・`titlecheck.py` を追加）。
