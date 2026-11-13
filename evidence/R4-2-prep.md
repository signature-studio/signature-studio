写せば足りる物: 9 件(表 §1 の行 1・2・3・7・8・12・15・16・17。行 5・14 は Firefox 経路・R4-3 の掃引で、R4-2 の対象外)
新しく書く必要がある物: 5 件(表 §1 の行 4・9・10・11・13。行 6 は Firefox 用で対象外)
この機械で今すぐ測れるか: いいえ(理由: `app/test/e2e/browser.mjs` が今のリポジトリに無い。`tools/verify-kit/` の測定スクリプトは旧ツリーの絶対パスを前提にする。所定プロジェクトの配信元 `/assets/proj/` も今のリポジトリには無い。§1・§2)

# R4-2 / R4-3 測定の入口の調査(読み取りのみ。Chrome・node・vite・npm は実行していない)

調べた時点: HEAD `a33abe1`。出典は `ファイル:行`。

## 1. 依存の一覧(測定スクリプトが読む物)

| # | 依存 | 使う場所 | 今のリポジトリ | 分類 |
|---|---|---|---|---|
| 1 | `app/test/e2e/browser.mjs`(`openBrowser`) | `sdrive-a1.mjs:41`、`sdriveg2-lat.mjs:41` | 無い(`ls app/test/e2e` → No such file。`app/test/` は `helpers`・`unit`・`worker` のみ) | 写せば足りる(`archive/e2e-old/e2e/browser.mjs`) |
| 2 | `browser.mjs` が読む `./cdp.mjs`・`./netlog.mjs` | `archive/e2e-old/e2e/browser.mjs:6-7` | 無い | 写せば足りる(`archive/e2e-old/e2e/cdp.mjs`・`netlog.mjs`) |
| 3 | `app/scripts/lib/png.mjs`(`decodePng`・`toRgba`) | `sdriveg2-lat.mjs:452`、`sdrive-a1.mjs:667` ほか(`cmpShot`・`savePic` の経路) | 無い(`app/scripts/lib/` は `wasm-exports.mjs` のみ) | 写せば足りる(`archive/e2e-old/scripts-lib/png.mjs`) |
| 4 | `APP` の既定値(旧ツリーの `lift/out-git/…/app` の絶対パス) | `sdriveg2-lat.mjs:29`(`--app` で上書き可) | 旧ツリーの絶対パス | 新しく書く必要(指定の既定を今のリポジトリの `app/` へ直す) |
| 5 | `/snap/bin/geckodriver`・Firefox のパス | `sdriveg2-lat.mjs:81`(Firefox の経路のみ) | `/snap/bin/geckodriver` は在る | 写せば足りる(R4-2 は Chrome で足りる。Firefox 経路は対象外と決めれば不要) |
| 6 | `FF_PROFILE_ROOT`・`DISPLAY` | `sdriveg2-lat.mjs:81`、`env.sh` | 無い | 新しく書く必要(Firefox を使わないなら不要) |
| 7 | `scn-lat-ff.json`(`lat` シナリオ) | `sdriveg2-lat.mjs --scenarios` | 在る(`tools/verify-kit/tools/`) | 写せば足りる(移動の要否は決める) |
| 8 | `scn-s3a-song.json`・`scn-p3-300.json`・`scn-p3.json` | P3 の測定(`sdriveg2-lat.mjs`) | 在る | 写せば足りる |
| 9 | 所定プロジェクトの配信 `/assets/proj/serina1-serum2.zip` | `openZipProject`(`sdriveg2-lat.mjs:316-324`) | 配信元が無い。`verify-bundle.sh:58-63` の `proj` 行は `lift/out-gpu2/refproj/`・`lift/out-ci1/c5/`・`docs/investigations/crash1/…` を張る。いずれも今のリポジトリには無い | 新しく書く必要(zip の置き場と配信の張り方を決める) |
| 10 | `tools/verify-kit/out/`(出力先の既定) | `lat-ab.py:4`(`O = K + '/out'`)、`p3-win.py:8`、`s3a-summary.py`、`chrome-count.mjs` | 出力先が `.gitignore` に無い(`git check-ignore` → rc=1)。`K` は `tools/verify-kit/`(今のリポジトリ内)に解決するので場所は今のリポジトリ内 | 新しく書く必要(`.gitignore` に入れるか、出力を `artifacts/` 配下へ向ける) |
| 11 | `env.sh` の `K=`(旧ツリーの `lift/out-ci1/kit` の絶対パス)・`REPO=`・`TMPDIR=$K/tmp` | `tools/verify-kit/env.sh:2-3,5`、`verify-bundle.sh`、`run-s3a.sh:10`、`run-rta2.sh:10` | 旧ツリーの絶対パス | 新しく書く必要(今のリポジトリ向けに書き直す。`scripts/env.sh` の `TMPDIR` と合わせる) |
| 12 | `wait-quiet.mjs`(load ≤8・I/O 圧 0・swap 入出力 0 を待つ) | `run-s3a.sh:27`、`verify-bundle.sh` | 在る(`tools/verify-kit/tools/wait-quiet.mjs`。依存は `fs` のみ) | 写せば足りる |
| 13 | `chrome-count.mjs`(他の Chrome の数を数える観測) | `run-s3a.sh:17-21`(gate)、`p3-win.py:73` | 在る。だが `/lift/out-ci1/kit/` の絶対パスで「自分の Chrome」を判定(`chrome-count.mjs:2,12`)。今のリポジトリ向けに直す必要 | 新しく書く必要(判定の基準パスを直す) |
| 14 | `s3a-plan.py`・`run-s3a.sh`(S3a の掃引。P3 の条件比較用) | P3 の掃引(R4-3 では必須ではない) | 在る。ただし `run-s3a.sh` は `lift/out-ci1/kit/env.sh` を読む | 写せば足りる(R4-3 の単発測定には不要) |
| 15 | `p3-win.py`(P3 の集計) | `p3-win.py:2,8`(`out/p3-*`) | 在る。`K/out/` を読む | 写せば足りる(入力の置き場を決める) |
| 16 | `lat-ab.py`(P2 の T の集計) | `lat-ab.py:2,4-5` | 在る。`out/lat-*` を読む | 写せば足りる |
| 17 | `s3a-summary.py`(P3 の表) | `s3a-summary.py:8-14` | 在る | 写せば足りる |

- `sdriveg2-lat.mjs` は `/proc`(`loadavg` `:247`、`pressure/io` `:641`、`task/<tid>/stat` `:133`)を読むため Linux 固有。`p3-win.py` の依存は標準ライブラリだけ。P3 の手順書 `S3A-SWEEP-PROCEDURE.md` は `docs/archive/investigations/ci1/0326/` に在る(`ls` で確認)。

## 2. 所定プロジェクトの開き方(実行はしていない)

- zip の実体: `~/ダウンロード/serina1-serum2.zip`、53,489,451 B(`ls -la` で確認。`evidence/R0-1.md` の照合済み)。
- 経路 A(`openZipProject` op。P2・P3 の全シナリオが使う。`sdriveg2-lat.mjs:316-356`): Files ボタン `#files-btn` を押す → `fetch('/assets/proj/' + zip)` を `DataTransfer` に入れて `#fx-input-files` に渡し `change` を発火 → Files 窓の zip の行 `.fx-row[data-name]` を待つ → File > Open(`click 48,85`)→ `#filedlg` の行を `dblclick` → 窓のタイトルが `serina1-serum2 …` になるまで待つ(`:344-351`)。
- 経路 B(`openProject`。複数ファイル。P2・P3 では使わない)は `:357-393`。経路 C(`openFile`。Chrome の file chooser)は `:394-424`。
- 実体: `app/src/files/explorer.ts:100-103`(zip の展開)、`app/src/engine/filedlg.ts:127-167`(`planOpenFolder`。`.flp` を優先)、`app/src/ui/filedlg.ts:131-135`(`#filedlg-input`)。
- `reference-project.md` §5 は「OPFS に置いて開く(旧検証キットの `proj` の手順)」とだけ書き、手順の本文は無い。
- 注意: `openZipProject` は `/assets/<dir>/<zip>` を読む。配信は `app/tools/assets-middleware.ts:60`(`FL_ASSETS_DIR` の直下を `/assets/` で配る。`app/vite.config.ts:4,12-13`)。

## 3. 測定の出力(コードの行から)

### P2 (`sdriveg2-lat.mjs` の `latDump` → `lat-ab.py`)
- `latDump`(`sdriveg2-lat.mjs:565-568`)は `window.__app.latency` の記録を `lat-<name>.json` に書く。各記録は `tEvent`・`tListener`・`tPage`・`tInject`・`tIdle`・`frames`・`answers`。
- `lat-ab.py` は `a-idle-menu`(idle の File クリック)と `b-play-menu`(再生中の File クリック・Esc)を読み、`T` の median / p90 / p99 / max を出す(`lat-ab.py:55,65`。判定は `percentile` = `sorted[max(0, ceil(p/100*n)-1)]`(`g2-analyze.py:17`))。
- **frame/s**: `scn-lat-ff.json` には `deskSnap`・`dragMeasure`・`dragRate` が 0 件で、`lat-ab.py` は `presentsPerS` を出さない。現行の op からは frame/s は出ない。`dragMeasure`(`sdriveg2-lat.mjs:478-497`)は `presentsPerS`(画素が変わった提示の数/s)を返すので、シナリオに足す必要がある。
- 画素が変わった frame の数は `app/src/ui/latency.ts` の `framesAreChanges`(`app/src/main.ts:685`。デスクトップは画素が変わった時だけ提示)に依る。

### P3 (`sdriveg2-lat.mjs` の `audioSnap`・`audioDump` → `p3-win.py`・`s3a-summary.py`)
- `audioSnap`(`:570-573`)は 2 s ごとに `underrunQuanta`・`episodes`・`engineGapFrames`・`lagCuts`・`presents` を読む。`audioDump`(`:576-581`)は 2 Hz のパルス・Unlock の時刻・1 Hz のレポートを `audio-p3.json` に書く。
- `p3-win.py:9-31`: 10 s 窓の**書き込み率**(Unlock のフレーム ÷ (10 × 44100))、**消費率**、**underrun quanta**・**episodes**・**engine gap frames** の差分。
- `s3a-summary.py:46,58`: 書き込み率の平均・最小・「0.95 未満の窓数」(`wr_lt95`)。**最長の連続**は `longestRunQuanta` を `s3a-summary.py:47,58` が ms に換算する(1 quantum = 128/44100 s)。`p3-win.py` には longest の列が無い(`grep -c longest` = 0)。
- `scn-s3a-song.json`・`scn-p3-300.json`: `wait` の合計は 314,500 ms(再生 300 s と起動待ちを含む)。Space は 2 回(開始・停止)。`move` は 0。
- **R4-3 の 2 条件**: 「放置」は `scn-s3a-song.json`(`move` 0、5 分)。「Mixer 表示・マウス移動」は `scn-o19.json` の `o19-song-mixmove`(`move` 600、**60 秒**の窓。`shown` は操作なしの表示、`hidden` は F9 で Mixer を閉じる)。5 分の条件は無く、R4-3 で作る必要がある。

### headless の rAF(60 Hz)
- 文書: `docs/STATE.md:32`・`docs/core/performance.md:3`・`docs/ROADMAP.md:44,73` が「headless の rAF は 60 Hz、120 fps は確認できない」と書く。
- コード: `app/src/perf/display.ts:134`(`hz` = 1000 / 中央値の間隔)が実測する。**この機械での実測は未確認**(本作業は実行していない)。

## 4. 完了条件への回答と未確認

- 完了条件 3(`grep -nE 'import|require|e2e-old|rAF|60 Hz'`)は本ファイルに含まれる(§1・§3)。
- 「60 Hz」の実測: **未確認**(§3。文書の記述のみ)。
- 所定プロジェクトの開く経路の実行: **未確認**(読み取りのみ)。
- `app/test/e2e` の復元・測定用ファイルの作成: 未実施(調査のみ)。

## 5. R4-2 の worker への入力案(全て未確認。実行していない)

1. 起動: `tools/boot-check/boot.mjs`・`cdp.mjs` の部品を使い、隔離の引数は `archive/e2e-old/e2e/browser.mjs` から写す(旧ツリーからは複製しない)。bundle は `artifacts/bundles/fl-engine-ci-0363b050/`、ページは `app/` の `vite build`(R0-2-prep §3 S1〜S4)。
2. 所定プロジェクトの zip を `FL_ASSETS_DIR/proj/serina1-serum2.zip` に置く(置き場は `artifacts/` 配下。git に入れない)。
3. 測定スクリプトを今のリポジトリ向けに写す: `sdriveg2-lat.mjs` の `APP` 既定を `app/`、`import` を `archive/` の写しへ、出力を `artifacts/` へ。`env.sh` は旧ツリーの絶対パスを外す。`chrome-count.mjs` の判定パスを直す。出力先を `.gitignore` に入れる(§6)。
4. P2 n=5: `scn-lat-ff.json` を 5 回回す。frame/s のため `dragMeasure` を足す(現行の `lat` シナリオは `presentsPerS` を出さない)。T は `lat-ab.py`、frame/s は `dragMeasure` の `presentsPerS`。画面の Hz は `display.ts:134` の `hz`。60 Hz 台なら「120 fps は確認できない」と明記。
5. P3(R4-3 の内容): `scn-s3a-song.json`(放置・5 分)を 1 回。Mixer 表示・マウス移動の 5 分版は無いので、`scn-o19.json` の `o19-song-mixmove`(60 秒)を元に作る。集計は `s3a-summary.py`(掃引はしない)。
6. 負荷の条件: `wait-quiet.mjs` を gate に使う(load ≤8・I/O 圧 0・swap 入出力 0)。測定の前後で `/proc/loadavg` と `chrome-count` を記録する。
7. 測定の前に、この機械の headless の rAF が 60 Hz 台かを `hz` で確かめる。

提案: `tools/verify-kit/out/` を `.gitignore` に入れるか、出力を `artifacts/measure/` へ向ける(R4-2 の前に決める)。
