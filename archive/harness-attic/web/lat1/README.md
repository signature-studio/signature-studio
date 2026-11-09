# lift/harness/web/lat1 — 入力→画面の遅延の測定器(決定 0194)

実エンジンを隔離ヘッドレス Chrome で動かし、CDP の模擬入力(ユーザー操作ではない)に対する「入力がページに届いてから最初の変化が描かれるまで」を段階に分けて数える。**app/ と lift/runtime は変えない**(計測は CDP の外から)。

## 前提

- `app/` の `vite build` の出力(`--outDir` 指定可)と `app/node_modules`、隔離 Chrome(`app/test/e2e/browser.mjs`)。
- 出荷バンドルを含む `FL_ASSETS_DIR`(`lift/out-websrv/assets`、読み取りのみ)。
- 環境変数: `LAT1_ROOT`(リポジトリの根)、`LAT1_APP`(`app/` 相当の木)、`LAT1_DIST`(build の出力)、`LAT1_ASSETS`。未指定は下の既定。

## ファイル

| ファイル | 役割 |
|---|---|
| `hooks.mjs` | 注入する計測(ページ側 `PAGE_HOOK` と、`Fetch.fulfillRequest` で worker-*.js の前に置く `WORKER_HOOK`) |
| `lib.mjs` | 自前の `vite preview`、隔離 Chrome、入力(CDP)、記録の回収、段階への分解(`join` / `stages`) |
| `run.mjs` | 1 本の測定(待機中と Space 再生中の 6 種類の入力 × 反復)。結果は `result-<config>-r<round>.json` |
| `series.sh` / `chain.mjs` | 構成を交互に回す(1 度に Chrome 1 つ。load ≤ 8 まで run の前に待つ) |
| `analyze.mjs` | 結果から表(`tables.md`)を作る。応答の判定はここで再計算する |
| `playwall.mjs` | 再生中の FILE メニューでエンジンが止まる件の再現(新しいセッションで N 回) |
| `trace.mjs` | Chrome のトレース(`PipelineReporter`)で、`rAF` → 合成器が出力するまで(ヘッドレス)を測る |
| `strip.mjs` / `prof-f8.mjs` | ホバー後の帯の再描画の確認(再現せず)、F8 の間の Worker の V8 CPU プロファイル(種類別の自己時間) |
| `mk-owner.py` / `mk-trace-table.py` / `firstuse.py` | 表(誰の時間か、トレース、初回の入力) |
| `waitfile.mjs` | バックグラウンドの完了通知用に、ファイルができるまで待つ |
| `rawupdate.mjs` | ポインタ移動の配信がフレームに揃えられる遅れ(`pointerrawupdate` はこの環境では来なかった) |
| `explore.mjs` / `explore2.mjs` / `probe-wheel*.mjs` | 入力の対象を探した跡 |
| `mk-nat-script.py` / `run-nat.sh` / `relink-lat1.sh` / `covwatch.mjs` / `natan.py` | ゲスト側: native -O0 の fork-probe(ia2 の D7 バイナリの読み取り用コピー)で入力ごとの関数入口数 |

## 使い方(要約)

```
cd <repo>/app && npx vite build --outDir /tmp/lat1-dist           # HEAD の app
LAT1_DIST=/tmp/lat1-dist node ../lift/harness/web/lat1/run.mjs --config SHIPH --out /tmp/lat1-run --reps 12
node ../lift/harness/web/lat1/analyze.mjs /tmp/lat1-run /tmp/lat1-tables
```
`--config` は SHIPH(既定の入口、パラメータ無し)/ F1xH(`?bundle=` 混成なし)/ hz120(`?hz=120`)/ dedupe1(`?dedupe=1`)。
