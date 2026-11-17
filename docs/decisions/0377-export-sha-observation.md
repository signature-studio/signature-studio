# 0377: 参照プロジェクトの Export の WAV のサイズと sha256 を、E13 と work-r53k で再取得する(R5-4)

分類: 測定(lift の処理を足さない。(a)(b)(c) の対象外。道具は `tools/` の追加だけで、製品(`app/`・`lift/`)は変えない)
撤回条件: この観測の結果は `evidence/R5-4.md` の記録にだけ使い、P2・P3 の判断や G 段階の「達成」の根拠に使わない。

日付: 2026-11-15   ROADMAP: R5-4   一覧: R5-4(ROADMAP の行)   コミット: この決定文書と道具を commit してから、道具の確認の run を行う(`evidence/R5-4a.md`)

## 決定

決定: R5-4 の Export の観測を、E13(対照。label `r54e13`)と work-r53k(label `r54r53k`)の各 n=2 で行う。順は E13・r53k・E13・r53k の直列。R5-4a(この依頼)の道具の確認の run は数えない。根拠: ROADMAP R5-4 の「Export の WAV のサイズと sha256(旧: 22,863,828 B、`381cdb5a…f979`)を再取得」を、測定の定義を先に固定して行うため。

## 定義(測定の前に固定)

- 「一致」= Export が WAV を出し、size が 22,863,828 B かつ sha256 の全 64 桁が `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979` と同じ。値は `tools/verify-kit/README.md` の 22 行から転記した。WAV は `sha256sum` で照合する。
- 「不一致」= WAV が出て、size か sha256 のどちらかが違う。両方の値を書く。
- 「停止」= `report.json` の `state` が `exited` か `error` で、WAV が出る前に止まった。`beaconExit.message` と `beaconExit.code` をそのまま書く。
- 「出なかった」= 上限 1,500 s(`exportSha` の `ms`: 1500000)を超えても `window.__app.saved` が空。
- 「数えない回」= 道具かページのエラーで、WAV の有無が読めなかった回(`opErrors` が非空など)。原因を書き、同じ位置で最大 3 回まで撮り直す。撮り直しの回も、結果を全部書く。
- sha256 は負荷に依らない量なので `wait-quiet` は要求しない。ただし各回の直前に `free -m` の 2 行と `/proc/loadavg` を記録する。

## 組(各 n=2・直列)

- E13(対照。ビルドの出力): `artifacts/bundles/fl-engine-ci-0363b050`、label `r54e13`。
- work-r53k: `~/.cache/signature-studio/work-r53k/bundle/fl-Oz`、label `r54r53k`。
- 順: E13 → work-r53k → E13 → work-r53k。各回は `node tools/verify-kit/serve-once-r54-bundle.mjs <bundle の絶対パス> <label>`(foreground、`timeout` 1800000)。
- 台本: `tools/verify-kit/tools/scn-r54-export.json`。入力は `~/ダウンロード/serina1-serum2.zip`(リポジトリには入れない)。

## 予測(観測の前に書く)

- E13: 「一致」と予測する。参考(旧の別の測定): `docs/archive/progress.md` が E8〜E13 で同じ WAV(22,863,828 B・`381cdb5a…f979`)と記す。
- work-r53k: 「一致」と予測する。外れ得る。work-r53k は E13 より後の変更を多く含む。音声の経路に触れたかは未確認。「不一致」になった場合も、結果をそのまま書く。

## 未確認

- 依頼者の機械・Wine との比較は 0 件。この sha が正しい音かは判定しない。一致は「E13 と同じ WAV が出た」ことだけを表す。
- 本観測は P2・P3・G 段階の判断に使わない(撤回条件)。

## 出典

- `docs/ROADMAP.md` の R5-4 の行。
- `tools/verify-kit/README.md` 22 行(期待値)。
- `tools/verify-kit/tools/sdriveg2-export.mjs` の `exportSha`(657〜664 行の元の位置)と、この commit で追加した `openZipProject`(`sdriveg2-lat.mjs` 317〜357 行の複製)。
- `tools/verify-kit/serve-once-r54-bundle.mjs`(`serve-once-r53-bundle.mjs` の複製。駆動するスクリプトと台本と出力の接頭辞の 3 点だけ変えた)。
- 旧の台本 `tools/verify-kit/tools/scn-export.json` の Export の手順(`ctrlKey KeyR` → `#filedlg #fx-ok` → Export 開始の click → `exportSha`)。Export 開始の位置 `1135,832` は旧の 2260x1116 での位置で、今の画面で合うかは未確認(道具の確認の run で `export-dialog.png` を見て決める)。

## 測定前の補足(2026-11-16)

(a) 前の作業用サブエージェントが、E13 の 1 回目(出力フォルダ `tools/verify-kit/out/r54-r54e13-chrome-1`、更新時刻 2026-11-15 16:55〜16:56)を実行した。依頼文の記述によれば、その後レート制限のエラーで止まった。直前の記録(`free -m`・`/proc/loadavg`・`ps`)は残っていない。同じフォルダに `export.json`・`export.wav`・`report.json` があるが、直前の記録がないため、0377 の「各回の直前に `free -m` の 2 行と `/proc/loadavg` を記録する」を満たさない。

(b) 決定: この 1 回は組に数えない。根拠: 各回の直前の記録は 0377 の要件であり、この 1 回はその要件を満たさない。この決定は、組の結果を見る前に行う。組の 4 回(E13 → work-r53k → E13 → work-r53k)は、新しく実行する。この 1 回の結果は、組の結果が出た後に「数えない回(直前の記録なし)」として別に書く。

(c) 出力フォルダの番号は、道具が label ごとに既存の最大の番号 + 1 で付ける(`serve-once-r54-bundle.mjs` の `nextOut`)。このため、新しい回は次の名前になる見込み(道具の定義からの導出。実行の前に書いた): E13 は `r54-r54e13-chrome-2` と `r54-r54e13-chrome-3`、work-r53k は `r54-r54r53k-chrome-1` と `r54-r54r53k-chrome-2`。

## 結果の追記(2026-11-16、`evidence/R5-4.md`)

組の 4 回(出力フォルダは `tools/verify-kit/out/` の下。各回の直前に `free -m` の 2 行・`/proc/loadavg`・`ps` を記録した)。

| 順 | label | 出力フォルダ | 判定 | size | sha256 の先頭 8 桁 |
|---|---|---|---|---|---|
| 1 | r54e13 | `r54-r54e13-chrome-2` | 一致 | 22,863,828 B | 381cdb5a |
| 2 | r54r53k | `r54-r54r53k-chrome-1` | 一致 | 22,863,828 B | 381cdb5a |
| 3 | r54e13 | `r54-r54e13-chrome-3` | 一致 | 22,863,828 B | 381cdb5a |
| 4 | r54r53k | `r54-r54r53k-chrome-2` | 一致 | 22,863,828 B | 381cdb5a |

- 組の中の判定の件数: 一致 4、不一致 0、停止 0、出なかった 0、数えない回 0。
- 数えない回 1 件: `r54-r54e13-chrome-1`(直前の記録がない。定義上の判定は一致だが、組に数えない。上の「測定前の補足」(b))。
- 予測と結果: E13 の予測「一致」→ 結果は 2 回中 2 回が「一致」。work-r53k の予測「一致」→ 結果は 2 回中 2 回が「一致」。
- E13 と work-r53k の WAV は同じバイト(`cmp`)。両方とも期待値と違わない。
- 詳細と未確認は `evidence/R5-4.md`。この結果は P2・P3・G 段階の判断に使わない(撤回条件)。
