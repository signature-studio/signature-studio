# 0368: P3 の条件 2 で、旧の道具(old)と今の道具(new)を交互・直列に測る(R4-3 と R4-3c の値の違いの切り分け)

分類: 測定(lift の処理を足さない。(a)(b)(c) の対象外。`app/`・`lift/`・`tools/` の中身を変えない)。撤回条件: この比較の結果を、R4-3 の値と R3-1d・R4-3c の値の違いの切り分けにだけ使う。別の窓の結果と混ぜて判断しない。

日付: 2026-11-15   依頼: R4-3d(main が付けた名。ROADMAP の行は変えない)   コミット: 前 `6fc4db5`(R4-3c の記録)

## 決定
決定: 測定は本書の定義どおりに行う。判断の規則は置かない(観測の記録)。根拠: 0367 と同じ。測る前に結果の使い方を固定せず、結果を見て選ぶと後から基準を動かす余地が生まれるため。判断を伴う選択は、結果の後に別の決定文書で、測定の前に規則を書いてから行う。

## 問い
- R4-3 の条件 2(E13、n=3、道具 `serve-once-p3-mixmove.mjs`)の underrun は 909 / 566 / 588(`evidence/R4-3.md` §4)。
- R4-3c の e13(E13、n=5、道具 `serve-once-p3-bundle.mjs`)は 39 / 0 / 104 / 30 / 6(`evidence/R4-3c.md` §4 の表 B)。
- 両者は同じ bundle(E13。manifest の wasm sha256 の頭 `58ad4a6aa720375a`)。値の桁が違う。違いが道具の違いで説明できるかを、道具だけを違えて、bundle・台本・page・窓を同じにして交互・直列に測って比べる。

## 比較の定義(測定の前に固定)
- old = `node tools/verify-kit/serve-once-p3-mixmove.mjs`(引数なし。E13 固定)。出力 `tools/verify-kit/out/p3-mixmove-chrome-<N>`。既存は N = 1〜3 と `p3-mixmove-chrome-2-reference-swapin`(参考。N の数に入らない)。次の N は 4 以降。
- new = `node tools/verify-kit/serve-once-p3-bundle.mjs "$HOME/work/signature-studio/artifacts/bundles/fl-engine-ci-0363b050" e13b`。出力 `tools/verify-kit/out/p3-mixmove-e13b-chrome-<N>`(既存なし。N は 1 から)。label `e13b` は R4-3c の `e13` と区別するため。
- 同じ bundle: `artifacts/bundles/fl-engine-ci-0363b050`(E13)。manifest の wasm sha256 の頭は、測定の worker が `expire_grep` で読んで測定の記録に書く(この文書の時点では未読)。
- 同じ台本: `tools/verify-kit/tools/scn-p3-mixmove-300.json`(条件 2)。
- 同じ page: `app/dist`。測定の開始時に `stat -c '%y' app/dist/index.html` を記録する(前回の値は 2026-11-14 18:45:44 +0900)。
- 順序: 交互・直列。位置 1〜10 = old・new・old・new … (位置 1 が old)。各 n=5(計 10 回)。
- run の保存: 各位置の run のディレクトリ名を測定の記録に書く。計算は `tools/verify-kit/tools/p3-recompute-dirs.py <run...>`。label は old が `p3-mixmove`、new が `p3-mixmove-e13b` と出る(`evidence/R4-3c-recompute.txt` で、label はフォルダ名から `-chrome-<N>` を除いた形)。

## 静かさの基準(測定の前に固定。0367 と同じ)
- 各回の直前に `node tools/verify-kit/tools/wait-quiet.mjs 8 10` を通し(rc を記録)、`/proc/loadavg`・`/proc/pressure/io` の 1 行目(some)の avg10・`vmstat 1 3` を読む。
- 数えない回 = `wait-quiet` の 2 秒の swap の入出力(pswpin/pswpout の差)が 0 でない、または 1 分 loadavg が 8 を超える、または `/proc/pressure/io` の some の avg10 が 0 でない。
- `vmstat` の si/so は参考として表に残す(判定には使わない)。
- 数えない回は参考として残し、同じ位置で撮り直す。1 つの位置の試行は最大 3 回。3 回とも基準に当たったら、その位置を数えないとして残して次へ進む。
- 数えた回が old・new のどちらかで 5 に満たない場合、語の定義は適用せず「n 不足」と書く(追加の測定は別の依頼)。

## 語の定義(測定の前に固定)
- 主指標 = underrun quanta(0〜300 s)。
- 「new が少ない側に分離した」= new の n=5 の最大 < old の n=5 の最小(厳密な不等号)。
- 「old が少ない側に分離した」= old の n=5 の最大 < new の n=5 の最小(厳密な不等号)。
- どちらでもなければ「範囲が重なる」。
- 事実だけを書き、有意性は測っていない(n=5)と必ず併記する。
- 副指標(判定に使わない): 書き込み率の最小・0.95 未満の窓数・presents/s・lagCuts・engine gap frames・Worker CPU。

## 参考(旧の別の測定。判定に使わない)
- `evidence/R4-3.md` の条件 2(E13、n=3、道具 `serve-once-p3-mixmove.mjs`、別の窓): 909 / 566 / 588。
- `evidence/R4-3c.md` の e13(E13、n=5、道具 `serve-once-p3-bundle.mjs`、別の窓): 39 / 0 / 104 / 30 / 6。同じ測定の wr25: 14 / 5 / 18 / 43 / 62。
- `evidence/R3-1d.md` の前(work-r25、n=5、同じ道具 `serve-once-p3-bundle.mjs`、別の窓): 読み B で 10 / 18 / 51 / 75 / 73、読み A で 10 / 18 / 51 / 75 / 13。
- 数字は「参考」と書いて並べるだけで、この測定の基準にしない。

## 事実(手順 2 の読み。2026-11-15 時点。出典は各項)
- (a) 道具の差(`diff tools/verify-kit/serve-once-p3-mixmove.mjs tools/verify-kit/serve-once-p3-bundle.mjs`。差は 6 箇所だけ):
  - bundle の指定: old は `BUNDLE = 'fl-engine-ci-0363b050'` の固定で、`path.join(repo, 'artifacts', 'bundles', BUNDLE)`。new は絶対パスの引数を `path.normalize` し、引数の数・label の形(`[a-z0-9]+`)・`manifest.json` の有無を検査して、不正なら exit 2。`BUNDLE` は basename。
  - 出力名: `p3-mixmove-chrome-<N>` 対 `p3-mixmove-<label>-chrome-<N>`。
  - 一時ディレクトリの接頭辞: `serve-once-p3m-` 対 `serve-once-p3b-`。
  - 同じ点: diff が挙げた行以外は全部同じ。台本・viewport・serve・`sdriveg2-lat.mjs` の呼び方・`vite preview` の配り方は、差に含まれない。
  - 同じ bundle の指す先: old の `repo/artifacts/bundles/fl-engine-ci-0363b050` と、new に `$HOME/work/signature-studio/artifacts/bundles/fl-engine-ci-0363b050` を渡したときの先は、リポジトリが `$HOME/work/signature-studio` にあるとき同じ実パス。
- (b) 履歴(`git log --format='%h %ad %s' --date=iso -- <道具・台本・sdriveg2-lat.mjs・lib>`):
  - `serve-once-p3-mixmove.mjs`・`sdriveg2-lat.mjs`・`scn-p3-mixmove-300.json`・`lib` の最新の変更は `854a410`(2026-11-14 00:42:43 +0900。R4-3 の測定スクリプトの追加)。`git log 854a410..HEAD -- <同じパス>` は空。
  - `854a410` の時刻は、R4-3 の出力フォルダの更新時刻(`ls -ld --time-style=full-iso`: `p3-mixmove-chrome-1` 00:14:53、`-2` 00:29:03、`-3` 00:35:26。いずれも 2026-11-14 +0900)より後。よって、run の時点の mixmove スクリプトが `854a410` と同じ内容かは未確認。
  - `serve-once-p3-bundle.mjs` の履歴は `0e1ded8`(2026-11-15 07:01:12 +0900。R3-1c)の 1 件。R4-3 の run より後に git に入った。
- (c) page: `git log -n 12 -- app/src app/index.html app/vite.config.ts app/package.json` は `6d95158`(2026-11-14 18:51:42 +0900)と `14268c2`(2026-11-10 04:48:45 +0900)の 2 件。`stat -c '%y' app/dist/index.html` は 2026-11-14 18:45:44.878 +0900(`app/dist/index.html` の更新時刻は `6d95158` より前)。`evidence/R4-3.md` に `dist`・`vite preview`・`app/dist` の記述は無い(`expire_grep` 0 件)。よって R4-3 の時点の page の更新時刻は未確認。
- (d) `report.json` の `loadStart`(`expire_grep` の `"loadStart"` と `evidence/R4-3c-recompute.txt`):
  - R4-3(条件 2、n=3): 3.00 / 0.66 / 2.31(`p3-mixmove-chrome-1`・`-2`・`-3`)。
  - R3-1d の前(r25、n=6): 0.07 / 1.75 / 2.37 / 1.27 / 2.08 / 1.99(範囲 0.07〜2.37)。後(r31、n=7): 2.00 / 1.44 / 1.24 / 2.42 / 2.20 / 3.84 / 1.56(範囲 1.24〜3.84)。
  - R4-3c(e13 n=5、wr25 n=5): e13 0.17 / 2.37 / 1.92 / 2.27 / 2.43(範囲 0.17〜2.43)。wr25 3.10 / 1.88 / 3.19 / 2.31 / 2.71(範囲 1.88〜3.19)。
- (e) 既存の run の `report.json`(`p3-mixmove-chrome-1`〜`-3`): bundle の識別(manifest の HEAD・wasm の sha・bundle 名)を書いたキーは無い。`expire_grep`(`bundle|sha|manifest|fl-engine|HEAD|engine\.`)の一致は `manifest_loaded` の時刻だけ。bundle の中身の更新時刻(`ls -la --time-style=full-iso artifacts/bundles/fl-engine-ci-0363b050`)は 2026-11-06 20:51〜20:52。R4-3 の run より前。mtime の読みでは中身は変わっていないが、内容の同一性は hash を取っていないので未確認。
- (f) R4-3 の時刻: 依頼の「2026-11-14 の約 01:00〜03:00 JST」と、出力フォルダの更新時刻(00:14〜00:35)は一致しない。`git log -- evidence/R4-3.md` で、最初の commit は `da50414`(2026-11-14 00:42:44 +0900)。依頼の `369a70e`(01:13)は 3 番目の commit だった。

## 未確認
- R4-3 の値(566〜909)と R4-3c の e13(0〜104)の違いの理由(道具・窓・時刻・負荷・page・bundle の状態のどれか)。この比較の結果でも、理由は断定しない。
- R4-3 の時点の mixmove スクリプト・page(`app/dist`)・bundle の内容の同一性(hash は取っていない)。
- R4-3 の測定時刻(依頼の 01:00〜03:00 と、出力フォルダの 00:14〜00:35 の差)。
- manifest の wasm sha256 の頭(測定の worker が測定の記録に書く)。
- BitBlt CPU は未測定(`report.json` に項目が無い)。P2 は測らない。耳での確認はしない。有意性は測っていない(n=5)。

## 出典
- `evidence/R4-3.md`(§2・§3・§4・§10)、`evidence/R4-3c.md`(§3・§4・表 B)、`evidence/R4-3c-recompute.txt`、`evidence/R3-1d.md`(§2・§3・§4)、`docs/decisions/0367-p3-e13-vs-r25.md`。
