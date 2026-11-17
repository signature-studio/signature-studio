# ci1 → PM 報告 0277-E8-1（第一報）: E8 の本番 bundle `fl-engine-ci-875bfd62`（Worker 4a03e043、ページ = c74b9194）の実ブラウザ確認

## 先頭の事実（実エンジン・実ブラウザ Chrome 156 / Firefox 158。代役・再生・推測なし。Firefox は `DISPLAY=:77`、beacon の webglRenderer = llvmpipe、desk.mode = gl）
- bundle は R2 から**読み取りで取得**し、manifest の 4 ファイルの size と sha256 が**全て一致**: wasm `0a54e7f86358…` 142,586,692 B、glue `ddaf06700da6…` 744,344 B、seed `f7726113ae34…`、dll `2f494419c1ad…`（比較の相手 E7b `fl-engine-ci-d9c160aa` も一致）。同じページ（c74b9194 の app = Worker 4a03e043）で E7b と E8 を配信して比べた。
- **E8 の確認項目はここまで全て満たした**（下表）。**E8 は E7b と、P3 の underrun・書き込み率・Worker CPU に差がない**（Chrome、静かな窓、旧 E7b 新 E8 の 6 組、p = 0.24〜0.56）。Firefox の同じ比較は実行中。
- 見た目: 主窓の位置は @0,0 1920x1080 で不変。**窓の中身は E7b と違う**（左・上へ 4〜6 px、右へ約 6 px ずれる。mixer は 6 px 下）が、崩れてはいない（下）。
- 新機能: **最小化 → `#taskbar-restore` → 復元が、本番 bundle でマウスでもキーボード（Tab → Enter）でも通った**。

## 判定（E8 対 E7b。run ごとの load と I/O 圧は RESULTS-E8.md）
| 項目 | 結果 | 判定 |
|---|---|---|
| 停止なし | 15 シーン × Chrome・Firefox: 全て ok（d7 は意図した終了）。Export と P3 の 60 s 再生 12 run（Chrome）も停止 0。Firefox の d5 は最初の起動で geckodriver が `Address already in use`（ハーネスの起動失敗、エンジンとは無関係）→ 再実行して ok | **満たす** |
| Export sha256 | 22,863,828 B、sha256 `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979`（私の `sha256sum` でも同値） | **満たす** |
| File メニュー ×100 | Chrome 100/100、Firefox 100/100 | **満たす** |
| first idle ≤ E7b + 500 ms | 単独 6 組ずつ、交互、Chrome は load 6.7〜8.7（I/O 圧 avg10 は最初の 1 run だけ 1.04、他は ≤0.4）、Firefox は load 5.0〜12.1（I/O 圧 ≤0.08）: Chrome E8 5,156 / E7b 5,321 ms（−164）、Firefox E8 4,486 / E7b 4,823 ms（−338） | **満たす**（E8 の方が速い向き。P1 は 10 s 以内） |
| P3（underrun・書き込み率）が悪化していない | 下 | **悪化なし（Chrome）** |

## alpha7: 起動直後の画（Chrome、1920x1080 ゲスト）E7b と E8 を並べた
- 窓表: 主窓 `TFruityLoopsMainForm 'FL Studio 2026'` は E7b・E8 とも **@0,0 1920x1080 z0**（可視の窓は 1 つで同一）。
- 画素: 全 2,073,600 画素中 709,324（34.2 %）が違う。原因は**窓の中身が平行移動している**こと（部分ごとの最良の平行移動、残差 0〜8 画素）:
  | 部分 | E7b から E8 への移動 |
  |---|---|
  | メニュー（FILE EDIT…）・Browser のアイコン | 左 6、上 6 |
  | チャンネルラック・Sampler の行・プレイリストの Track 名・パターン一覧 | 左 4、上 6 |
  | mixer（Master と Insert 1〜9） | 左 4、**下 6** |
  | 右上の最小化/最大化/閉じる、プレイリストの閉じる | **右 5〜6**、上 6 |
- 目視（`e8/shots/cmp-*.png`、上が E7b、下が E8 の 2 倍拡大）: ツールバーが上端から始まり（E7b は上に 6 px の外枠があった）、右上の窓ボタンは右端に寄る（閉じるの右端は x≈1912 < 1920 で見える）。メニュー文字・transport・Browser・Channel rack・Sampler・mixer・プレイリストが全て見え、**欠けて読めない物はない**。下端は Browser の水平スクロールバーの下半分が 6 px 切れる（E7b では全体が見えた）。
- 解釈（私の読み）: alpha1 が予測した『FL 自身が最大化を知らされる』結果として、FL の子窓が最大化した窓の外枠ぶん（約 6 px）だけ外へ配置されている。実際の Windows の最大化窓と同じ見え方。崩れではなく外枠が画面外に出た。スクリーンショット: `e8/shots/alpha7-E7b-d9c160aa-main-1920x1080.png`、`alpha7-E8-875bfd62-main-1920x1080.png`、差分 `alpha7-E7b-vs-E8-diff.png`。

## 最小化 → `#taskbar-restore` → 復元（Chrome、本番 bundle E8、FL 自身の最小化ボタンをマウスでクリック）
| 時点 | 窓表（可視） | `#taskbar-restore` の DOM | フォーカス |
|---|---|---|---|
| 起動直後 | 主窓 @0,0 1920x1080 の 1 窓 | `hidden` = true、display none | `desktop` |
| 最小化の 3 s 後 | **0 窓** | `hidden` = false、display inline-block、位置 x12 y1071 幅 182 高さ 33（ページ座標、desktop は y36 から 1920x1080 なので左下）、数は 1 | `desktop` |
| Tab を押した後（キーボードの経路） | 0 窓 | 同じ（x12 y1071） | **`taskbar-restore`** |
| クリック（DOM の click）/ Enter の 3 s 後 | **主窓 @0,0 1920x1080 の 1 窓（元に戻る）** | `hidden` = true | `desktop`（フォーカスが desktop に戻る） |
- エンジンは停止せず（`engine-idle`）、2 経路とも成功。最小化後の画面は暗い背景に左下のボタンだけ。復元後の画（`e8/shots/minrestore-3-after-restore.png`）: 窓は戻り、ツールバーが右へ約 15 px ずれる（既知の AlignControls）。**それ以外に気づいた差**: 復元後も最小化ボタンが青くハイライト（ポインタがそこに残っているため）、ヒント欄に『Minimize』、プレイリストの題が『Pattern 1』ではなく『(none)』になった（原因未確認。FL の復元時の再描画か、選択パターンの表示が空になったのかは区別していない）。
- 注意: 復元のクリックは DOM の `.click()`（ページのボタンの実クリックではない）。キーボード経路の Enter は実際のキー入力。マウスでボタンを押す経路の実クリックは未確認。

## P3: E7b 対 E8（Chrome、参照 zip を開き Space で 60 s、旧新交互 6 組、開始時 load ≤8 かつ I/O 圧 0 の run だけ数える）
- 数えた run: E7b 5、E8 4。外れた run: E7b の 1 run（開始時 load 8.6）、E8 の 2 run（I/O 圧 avg10 = 0.04、load 10.8）。E7b 1 と E8 1 の最初の 2 run は I/O 圧が記録されていない（ドライバの記録漏れ。E8 の 1 は load 6.2 で数えた）。全 12 run の感度も併記。
| 指標（60 s） | E7b 中央値 (n) | E8 中央値 (n) | p（数えた run） | 全 run の p |
|---|---|---|---|---|
| underrun quanta | 785 (5) | 725 (4) | 0.24 | 0.41 |
| 重い区間 30〜60 s の underrun quanta | 687 | 653 | 0.26 | 0.47 |
| episodes | 85 | 77 | 0.28 | 0.38 |
| エンジンが自分で書いた gap フレーム | 92,735 | 86,073 | 0.24 | 0.41 |
| 書き込み率（Unlock、10〜60 s の平均。1.0 = 実時間） | 0.401 | 0.397 | 0.33 | 0.56 |
| Worker CPU（60 s、秒） | 45.6 | 46.3 | 0.29 | 0.40 |
- 窓別の表は `RESULTS-E8.md`（0〜10 s は書き込み率 0.72、10〜20 s は 0.48 で underrun 0、20〜50 s で 0.33〜0.41 に落ちて underrun が出る、50〜60 s は 0.45）。**E7b も E8 も同じ形**で、underrun は曲の 20〜50 s に集中する。
- 差は検出できない（p ≥ 0.24、n=4〜6）。**E8 で P3/P1 の悪化は出ていない**。underrun の量（60 s で約 700〜900 quanta、約 85 episodes）は E7b・E8 に共通。
- 旧 ff5ee429 と E7b の比較（以前の私の beacon 差分で『新が多い向き』だった件）は、この表とは別。PM の項目 3 の指示どおり別の窓で 6 組以上流す。

## 未確認・できなかったこと
- Firefox の P3（E7b 対 E8、3 組）は実行中。ページのボタンの実マウスクリック。復元後のプレイリスト題『(none)』の原因。オーナーの機械は 0 件。
- 旧 ff5ee429 対 E7b の P3 決着（項目 3）はまだ流していない。E9 のデプロイ（13:00〜）までの空きで流す予定。

## 自己申告の逸脱
1. Firefox の d5 の最初の起動失敗（geckodriver のポート衝突）を、最初の集計で見落とした（『全て ok』は結果のある 29 行だけを数えていた）。報告の前に気づき、集計に『結果なし』の行を足し、再実行した。
2. P3 の最初の 1 run は I/O 圧の記録が無い（私のドライバの片方に I/O 圧の記録を足し忘れていた）。以降の run は記録あり。
3. `bash` ツール・`sleep`・`&`・`pkill -f` は今回は使っていない。

## 成果物（`lift/out-git/ci1-results/0277/e8/`）
`REPORT-0277-E8-1-to-PM.md`、`RESULTS-E8.md`、`shots/`、`data/`。キットは `../kit/`（`p3`・`minrestore` モードと `p3-win.py`・`pngdiff.py` を追加）。
