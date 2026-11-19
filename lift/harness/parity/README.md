# lift/harness/parity — ネイティブ対 wasm の全操作比較と退行ゲート(決定 0226)

**入力はすべてハーネスの模擬(G3 スクリプトまたは CDP)であり、ユーザー操作ではない。** 実エンジン(FLEngine_x64.dll と同梱 5 DLL の lift 結果)が実行されたときの CPU 秒だけを測る。代役・再生・期待値の貼り付けは無い。

## 何を比べるか
同じ入力列(`ops.tsv` → `mk-scripts.py` が作る G3 スクリプト、仮想時計)を
- **N1**: 同じ生成 C を native に `clang -O2`(計装なし、runtime も -O2、上流 Blend2D)でビルドした物(`native-build.sh`)
- **W**: wasm の bundle(node の JSPI、`node-bundle-run.mjs`。`parity1-maincpu:` 行 = wasm が走る主スレッドの CPU)
で再生し、操作ごとの CPU 秒(と仮想 1 秒あたり)、ゲストの進捗カウンタ、16 面の面 SHA を比べる。面 SHA が違えば**画素が違う**ので FAIL。
- **N2**(実物 FL64.exe を Wine で): `wine-run.py` と `tools/wine-install.sh` を作ったが、FL64.exe は Wine 10.0 で「The validity of the program could not be verified」で止まり測れなかった(検証の偽装はしない)。
- Chrome(実時間、`chrome-ops.mjs`): CDP の模擬入力。Worker スレッドの CPU(/proc)、主スレッドの CPU、canvas の更新回数/s(`putImageData` のうち画素が変わった物)。

## 使い方(退行ゲート)
```
PARITY_REPO=<binaries/ と lift/out-g8a1/inputs/ があるチェックアウト> \
lift/harness/parity/run.sh --bundle <bundle のディレクトリ> --native <N1 のバイナリ> [--quick] [--sessions "boot ui-pl ..."] [--out DIR] [--chrome <assets 内の bundle 名>]
```
- 所要(この機械、16 コア、load 15〜40、1 本ずつ): `--quick`(boot + ui-pl)約 3 分。既定の全 10 セッション(56 行)は load 10〜15 で約 6〜8 分、load 40〜65 では 16 分(wasm 0.3〜1.2 分 + native 0.1〜0.5 分 / セッション)。実行中の 1 分平均 load が 20 を超えたら、CPU 秒の判定は信頼できないという WARN を出す(同じ bundle の CPU が load だけで 0.59〜1.81 倍動く)。`--chrome` は約 4 分(要 Chrome・`app/node_modules`・vite build)。
- 終了コード 0 = 通過、1 = `expected/thresholds.tsv` の閾値超過または画素差、2 = 実行の失敗。`WARN` は目標(W/N1 ≤ 1.2、連続操作で wasm 1.0 CPU 秒/仮想秒以下)に届かない行(失敗ではない)。
- `expected/thresholds.tsv` は「受け入れた状態(V1H)× 2.0」(`expected/thresholds-nonhybrid.tsv` は V1 × 2.0)の退行ガード(目標ではない。1.2 倍程度の退行は見えない)。`mk-thresholds.py MASTER W N1 OUT 2.0` で受け入れ後の master 表から作り直す。
- N1 は取り込みごとに作り直す(生成 C が変わるため): `build-engine-bundle.sh gen resplit` の後に `native-build.sh <WORK> <OUT>`(約 20 分)。

## ファイル
| ファイル | 役割 |
|---|---|
| `ops.tsv` | 操作の行列(セッション、操作、区分、説明、G3 コマンド、種別 D=単発 / C=連続) |
| `mk-scripts.py` | `ops.tsv` → `<session>.native.g3`(`cpu` 行つき)と `<session>.wasm.g3` |
| `run.sh` | ゲート本体(上の使い方) |
| `summarize.py` / `parity_lib.py` | native と wasm の stdout から操作ごとの表(CPU、比、進捗、面の一致) |
| `check.py` / `mk-thresholds.py` / `expected/thresholds.tsv` | 閾値の検査と生成 |
| `mk-master.py` / `summ-master.py` | 複数の構成(N1, N1fc, V0, V1 ...)を並べた master 表と比の要約 |
| `native-build.sh` | N1 のビルド(生成 C を -O2 で、`WRAPS=1` でカウンタ・サンプラを付ける) |
| `parity_wrap.c` / `blitvol.py` | native 専用: ラスタ関数(`fle_r_blit` など)の画素量、SendMessageW の記録 |
| `parity_prof.c` / `profsym.py` | native 専用: SIGPROF のサンプラ(perf が使えないため)と記号化 |
| `chrome-ops.mjs` / `prof-top.mjs` / `prof-diff.mjs` / `prof-callers.mjs` / `prof-units.py` | Chrome の操作の測定と V8 CPU プロファイルの集計(関数、呼び出し元、単位) |
| `node-bundle-run.mjs` | `tools/prune2/node-bundle-run.mjs` の写し + 主スレッド CPU の行 + 追加 seed(`PARITY_SEED_TSV`) |
| `wine-run.py` | N2: Wine + Xvfb + XTest で同じ G3 スクリプトを実時間で再生(FL64.exe が起動しないので未使用) |

## 注意
- seed は `lift/harness/seeds/vfs-seed-g8.tsv`(Basic.flp と Sampler.flp が入る)。`vfs-seed-idle1.tsv` だと Basic.flp が無く「Error」ダイアログ(TMsgForm)が主窓を disabled にして操作が空振りする。
- 仮想時計の下で測るので CPU 秒/仮想秒は壁時計に依存しない。ただし load が高いとキャッシュ競合で CPU 秒も 1〜2 割動く(各表に load を併記)。
- ミキサーの停止(決定 0226 §6)の再現列 `ui-hang1` はゲートの既定セッションに入れていない。
