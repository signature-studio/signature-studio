実エンジン: 実行なし(既存の道具・旧記録・コードの read だけ)。Chrome・ビルド・cargo は実行していない。
結果: Tempo の数字を機械で読む道具は既存に無い。動いた・動かなかったは画素の差分で読める。数字は PNG を開いて目視で読む(目視と書く)。右クリックの結果は窓表(describe・listWindows)で読める。engine log には WM_CONTEXTMENU の行が無い。
決定: R2-4c の手順は「製品のページ(パラメータ無し)+ 既存の drag・click・shot・pixel・describe・latDump・saveLog の op + 取り込み前後の bundle の直列・交互」に絞る。新しい URL パラメータ・環境変数・旗・診断は要らない。
※ 完了ではない: P2 の 120 fps は未確認、P3 は音切れが残る、O9 は未特定、依頼者機の値は 0 件。この記録は測定手順の設計だけで、O20 の確認ではない。

## 0 開始時

- `git rev-parse HEAD`: f058835f91b957b1a78aa683fe244bb232a7f27f
- `git status --short`: `?? evidence/R0-6-failfast.txt` のみ(別の作業の未追跡。触れていない)
- 変更したファイル: `evidence/R2-4b-design.md` の追加のみ。`lift/`・`app/`・`ci/`・`.github/`・`tools/` は読むだけ。

## 1 事実(旧記録と今回の確認)

| 項目 | 結果 | 出典 |
|---|---|---|
| 旧: 右クリックは 6 要素で窓が出ない | 旧 E13 と RT-B の再現、RT-E でも出ない | `docs/archive/feedback/2026-11-07-owner-observations.md:158`、`:313` |
| 旧: Tempo の値(開始 200.000、3 px で 522.000) | RT-B(診断入り page)の値。旧 RT-E では 200.000 → 180.000 | 同 `:158`、`:247`、`:306` |
| 旧: 読み方(数字をどう読んだか) | 読んだ範囲(`:244-251`、`:301-314`)には記述が無い。同じ記録の他の行にあるかは未確認 | 同 `:244`、`:303` の実行の節 |
| 旧: シナリオ `scn-o20-tempo-ptr.json` | 本リポジトリに無い(`tools/`・`docs/` の grep で 0 件) | 同 `:244` が参照 |
| 旧: 画・数値の置き場 | `docs/investigations/gpu8/0331/logs/` は無い。`lift/out-git/gpu8-results/0331/` と `lift/out-gpu2/runs/rt-o20-*` も無い | `ls` の結果 |
| 旧: 測定に使った page | page-rt1 のコピー(worker が `mark('pointer')` を出し、main が `pointerTrace` を残す)。診断入りの改造ページ | 同 `:244`、`:303` |
| ROADMAP R2-4 | 「3 px ドラッグ→値、80 px ドラッグ→値、6 要素の右クリック→窓表」。完了条件は `evidence/R2-4.md` | `docs/ROADMAP.md:32` |
| 今回の bundle | `~/.cache/signature-studio/work-r24/` はあるが `bundle/` は無い(別の worker がビルド中) | `ls` の結果 |

## 2 (a) Tempo の値の読み方

| 方法 | 既存の道具で可能か | 根拠 |
|---|---|---|
| 窓表から値を読む | 不可。窓表は窓の lid・位置・大きさ・class・title だけ | `lift/runtime/fle_desk.c:422-431`(`build_desc`) |
| `window.__app` の API から値を読む | 不可。state・engineLog・latency・deskDumpBase64・readbackDesk・deskStats・screen はコントロールの値を返さない | `app/src/main.ts:161-181` |
| engine log から値を読む | 不可(確認できた範囲)。Tempo の数字を出す行は旧記録にも `tools/` にも無い。522 の出所は記録に無い | `tools/` の grep で 522 と Tempo の該当 0 件 |
| 画像から数字を機械で読む | 不可。OCR の道具は無い | `tools/` の grep(tesseract・ocr)で 0 件 |
| 画素の差分で「動いた・張り付いた」を見る | 可能 | `tools/verify-kit/tools/pngdiff.py`(画素比較)。`sdriveg2-lat.mjs` の `pixel`・`shot`・`cmpShot`(`tools/verify-kit/tools/sdriveg2-lat.mjs` の case) |
| 1 点の色を時系列で見る | 可能(`sdrive-a1.mjs` の `series` op。`sdriveg2-lat.mjs` には無い) | `tools/verify-kit/tools/sdrive-a1.mjs:758-775` |
| PNG を開いて数字を目視で読む | 可能。結果は「目視」と書いて記録する | 目視の読みは機械の値ではない |

読み方の結論: 数字は機械で読めない。R2-4c は、Tempo の箱を切り出した PNG を各段で保存し、(1) 画素差分で「変わったか」を判定し、(2) 数字は目視で読んで「目視」と書く。

Tempo の箱の位置: 保存画から測っていない(未実施)。座標の候補は旧記録の (450,20)(`docs/archive/feedback/2026-11-07-owner-observations.md:313`)だけ。R2-4c で `o20-start` の PNG を保存してから、そのファイルのパスを書いて目視で確かめる。

## 3 (b) ポインタ・右ボタンの操作

| 操作 | 既存の道具 | 根拠(読みのみ。実行は未確認) |
|---|---|---|
| 左右のボタンの down・up | `sdriveg2-lat.mjs` の `down`・`up`(`button` に `right` を指定可) | `tools/verify-kit/tools/sdriveg2-lat.mjs:220-221`。ボタンの対応は `:176`(`right: 2`) |
| 右クリック(press と release) | `click` に `button: "right"` | `sdriveg2-lat.mjs:222-230` |
| ドラッグ(3 px・80 px) | `drag`(`x,y → x2,y2`、`steps` 既定 10、`stepMs` 既定 40) | `sdriveg2-lat.mjs:231-239` |
| 座標の系 | guest の screen 座標。CSS 座標へ `desktop の rect.x + (sx+0.5)·(rect.w/gw)` | `tools/verify-kit/tools/sdrive-a1.mjs:183`(同じ式を `sdriveg2-lat.mjs` が使うかは未確認。本体を読んでいない) |
| guest の大きさ | `window.__app.screen.guest`(既定は SCREEN_DEFAULT) | `app/src/main.ts:164` |
| 送られた入力の記録 | `latDump`(ページ自身の入力の遅延の記録。tEvent・outcome など) | `sdriveg2-lat.mjs:566`、`app/src/ui/latency.ts:17` |

6 要素の座標(guest 座標。旧 RT-E の記録):

| 要素 | 座標 | 出典 | 照合 |
|---|---|---|---|
| Tempo | (450,20) | `owner-observations.md:313` | 未実施(R2-4c で `o20-start` の PNG を開いて確かめる) |
| Song Position | (485,20) | 同 | 未実施(R2-4c で `o20-start` の PNG を開いて確かめる) |
| Pattern Selector | (615,61) | 同 | 未実施(R2-4c で `o20-start` の PNG を開いて確かめる) |
| Channel rack のボタン | (520,144) | 同 | 未実施(R2-4c で `o20-start` の PNG を開いて確かめる) |
| Playlist の空き | (1500,900) | 同 | 未実施(R2-4c で `o20-start` の PNG を開いて確かめる) |
| Mixer の insert 名 | (505,690) | 同 | 未実施(R2-4c で `o20-start` の PNG を開いて確かめる) |

3 px と 80 px の正確な形は、旧記録に「3 px の小さな上下 1 回」「80 px 下」とあるだけで、旧のシナリオは無い。R2-4c の案は次のとおり(案。実行は R2-4c):
- 3 px: `drag` (450,20) → (450,23)、`steps` 3。
- 80 px: `drag` (450,20) → (450,100)、`steps` 20、`stepMs` 40。
- 20 px 上(旧にあった): `drag` (450,100) → (450,80)。

## 4 (c) 右クリックの結果の読み方

| 読み方 | 可能か | 根拠 |
|---|---|---|
| 窓表で「メニューの窓が出たか」 | 可能 | `describe` op(`sdriveg2-lat.mjs:589`)→ `window.__app.deskDumpBase64(true)`(`app/src/main.ts:175`)→ 窓の行(`fle_desk.c:422-431`)。`listWindows`(`sdriveg2-lat.mjs:428`)と `waitWindow`・`expectWindow` が class 名で判定する |
| メニューの class 名 | `TQuickPopupMenuWindow` | `sdriveg2-lat.mjs:301`(menuCycles の判定)。旧 RT-E の対照: 左クリックの File メニュー(18,20)で出る(`owner-observations.md:313`) |
| 画で見る | 可能 | `shot`(PNG。クリップは `geometry` の rect)と `pixel` |
| engine log に WM_CONTEXTMENU の行が出るか | 無い | `lift/runtime/fle_input.c:973-985` の WM_RBUTTONUP の処理に log の出力が無い(`fle_input.c` に `fprintf` が 0 件) |
| 入力が届いたか | `latDump` で送られた入力の記録は読める | `sdriveg2-lat.mjs:566`。WM_CONTEXTMENU そのものは見えない |

`fle_desk_describe` の窓表は、コンポジタが描いた窓の一覧(`cur[]`)。メニューが出れば `TQuickPopupMenuWindow` の行が出る。

## 5 (d) R2-4c の手順(案)

前提(R2-4c が始める前に満たす):
- P0. `~/.cache/signature-studio/work-r24/bundle` ができてから始める(今は無い)。`app/` の build は別の worker が行っているので、R2-4c は build しない。
- P1. bundle の所在: `tools/verify-kit/serve-once-o9.mjs:13` は bundle を `e13` と `head` の 2 つに固定している。R2-4c は bundle の場所を 1 つ足す必要がある(`tools/` の 1 行。製品に触れない。URL パラメータ・環境変数は不要)。
- P2. 負荷の確認: `tools/verify-kit/tools/wait-quiet.mjs`(load 8 以下、I/O 圧 0 を待つ)を各 run の前に使う。`free -m` と `vmstat 1 3` を記録する。

手順:
1. 起動: `serve-once-o9.mjs` と同じ形(`sdriveg2-lat.mjs --browser chrome --url http://127.0.0.1:PORT/`)。URL は `/`(パラメータ無し)。
2. 開始: `closeWizard`(ウィザードを閉じる。`scn-o9.json` と同じ)→ `shot` `o20-start` → `describe` `o20-start`。PNG と窓表を保存する。
3. Tempo の確認: 保存した PNG を目視で開き、(450,20) が Tempo の箱の中にあることを確かめる。結果を「目視」と書く。
4. 3 px: `logMark` → `drag` (450,20)→(450,23) → `waitIdle` 相当の `wait 800` → `shot` `o20-3px` → 箱の切り出しを画素差分で比べ、目視で数字を読む。
5. 80 px: `drag` (450,20)→(450,100) → `wait 800` → `shot` `o20-80px` → 同じ判定。
6. 6 要素の右クリック(各要素ごと): `describe` `rc-before-N` → `click` `button: right` の座標 → `wait 1000` → `describe` `rc-after-N` → `expectWindow` `TQuickPopupMenuWindow` `visible: true`(失敗しても止めない扱い)→ `shot` → `key Escape` → `wait 800`。
7. 対照: 左クリック File メニュー (18,20) で `TQuickPopupMenuWindow` が出ることを確かめる(出なければ窓表の判定が効いていないので、その run は使わない)。
8. 保存: `saveLog`(`logMark` から後のengine log)、`latDump`、PNG、窓表(`describe` の行)、`manifest.json` の sha256、開始時の load と I/O 圧。
9. 比較: 取り込み後(`work-r24`)と取り込み前(`~/.cache/signature-studio/work/bundle`。版と中身は未確認なので manifest の sha256 を記録する)を、1 本ずつ直列・交互に各 n=3。headless Chrome は同時に 1 本だけ。判定は R2-4c で書く。

R2-4c が保存する物: PNG(Tempo 箱の切り出し、各段の全画面)、窓表の行、`latDump` の JSON、engine log の範囲、manifest の sha256、load と I/O 圧、目視の読みの記録(「目視」と明記)。

## 6 できない手順と理由

| 手順 | できるか | 理由 | 代替 |
|---|---|---|---|
| Tempo の数字を機械で読む | できない | 値を返す API が無い(`main.ts:161-181`、`fle_desk.c:422-431`)。OCR の道具が無い | 画素差分(動いた・動かない)と目視(「目視」と明記) |
| 旧の 522 を同じ手順で再現する | 未確認 | 旧のシナリオが無い。旧の値の読み方が記録に無い | R2-4c は上記の手順で測る。旧の数値と比べるのは参考 |
| WM_CONTEXTMENU が guest に届いたことを engine log で見る | できない | `fle_input.c` に log の出力が無い | 窓表・`latDump`・画 |
| 新しい URL パラメータ・環境変数・旗・診断を足す | 要らない | 上の手順は既存の op だけで足りる | 足さない(AGENTS §6) |

旧の RT-E で使われた `FLE_DIAG_ATOM=1 FLE_DIAG_GIL=1` は、R2-4c では使わない。製品の既定の bundle と条件が変わるため。

## 7 未確認

- 右ボタンの down・up を `sdriveg2-lat.mjs` で実行したこと: コードの読みだけ。
- `sdriveg2-lat.mjs` の `mouse()` が `sdrive-a1.mjs:183` と同じ座標の式を使うこと: 未確認。
- Song Position (485,20) と Pattern Selector (615,61) が画のどの部品か: 未確認。
- `~/.cache/signature-studio/work/bundle` の版と中身: 未確認。
- 旧の page-rt1 のコピーの所在: 未確認。
- 要素の座標(§3)と画の対応: 未確認。旧記録の座標(旧 RT-E)を引いただけで、本リポジトリの画では確かめていない。
- P2 の 120 fps は未確認、P3 は音切れが残る、O9 は未特定、依頼者機の値は 0 件。
- O20 の原因: 本記録では断定しない。

## 追記 2026-11-14(出典のない目視の除去)

直す前の記述(出典不明の観測。使わない):
- §2 の段落(直す前の `:39`): 「依頼に添付された FL Studio の画面(画面 1920×1080。出典の保存先は無く、証跡に使わない)では、`140.000` の表示が x 約 440〜490、y 約 12〜28 にある。この画の Tempo は 140.000 で、…」
- §3 の表の列(直す前の `:54-61`): 「目視の照合(添付画・出典不明)」の列で、Tempo「一致」、Channel rack「一致(Sampler のボタン、目視)」、Playlist「一致(Playlist の格子の中、目視)」、Mixer「一致(Insert 1 の縦書き、目視)」、Song Position・Pattern Selector「画の部品を特定できない(未確認)」。
- 末尾の未確認の行(直す前の `:118`): 「添付の画(140.000 の画面)の出典: 不明。証跡に使わない。」

除去の理由: この依頼に画像の添付は無かった。記録自身が「出典の保存先は無い」と書いていた。開いたファイルを示せない観測は、AGENTS §5 の誠実性の規則に反するので書かない。

今回の確認(ファイル名の `ls` と `find` だけ。中身は開いていない): `evidence/R1-6b/` には `*-o9-*.png` が 6 枚(`e13-run1-` と `head-run1-` の各 3 枚)、`evidence/R2-2h/` には `r23-run1-o9-*.png` が 3 枚ある。`tools/verify-kit/out/` には名前に `start` を含む `o9-start.png` が 20 枚あり、`evidence/` 直下には該当が無い。これらを開いた目視は、この依頼では未実施。座標の目視は R2-4c で行う。
