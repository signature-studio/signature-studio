# 0312e: S3 の先行実装 — Worker 数と GetSystemInfo/GLPI のプロセッサ数を host の事実から決める(I15a・I15b)。S3a(env を残す)と S3b(env を除く)の 2 本の diff(mt3。PM 指示 22:14。仮。**build・test は未実施〔窓の規則〕。実エンジン・Chrome は 0。実測は『diff が S0+S1 の木に当たる』ことだけ**)

## 0. 冒頭の三点

1. **作った物**: S0+S1 の上の別 diff 2 本(`impl/0312e-s3a-host-cpus-env-kept.diff` 4 ファイル・120 行、`impl/0312e-s3b-env-removed.diff` 2 ファイル・79 行)。**S3a = host の `navigator.hardwareConcurrency` を n の既定にする(`FLE_PROCESSORS`・`FLE_WORKERS` があれば優先)**。**S3b = 2 つの環境変数を除いた最終形**(n だけから決める)。**実測**: base(S0+S1 の木の 4 ファイル)→ S3a → S3b の順に `patch --batch -p1` が通り、結果が私の編集後のファイルと byte 一致(cmp 差 0)。**コンパイル・テストは未実施**(窓の後)。
2. **B1 の env についての私の訂正(gpu1・PM に info 済み)**: B1 では `FLE_PROCESSORS` は**無視されている**(16 件の修正が `getenv("FLE_PROCESSORS")` を除いた。`fle_sys_processors()` は `return 1u;`)。効くのは `FLE_WORKERS` だけ。20:08・21:13 の私の『FLE_PROCESSORS でも n を変えられる』は誤り。B1 の測定の解釈には影響しない〔gpu1 の run-b1.sh は FLE_PROCESSORS=1 を入れており、効く値と同じ。E11 の測定条件が processors = 1 だったかは、run-b1.sh の命名からの推測で、gpu1 の run 記録で確認してほしい〕。
3. **先行点検で見つけた潜在バグ**: `fle_sysinfo.c` の `GetLogicalProcessorInformation` のレコード用バッファが `(FLE_GLPI_RECORD_COUNT + 15u)` 個 = **プロセッサ 16 個まで**(コメントの『up to 16』)。n を 32 まで許すと `count = RECORD_COUNT - 1 + np` が 38 になり**スタックのバッファを溢れる**。S3a で `(FLE_GLPI_RECORD_COUNT - 1u + FLE_SYS_MAX_PROCESSORS)` に直した(n ≤ 32 で足りる)。

## 1. 変更の中身(4 ファイル)

| ファイル | S3a | S3b(S3a からの差) |
|---|---|---|
| `lift/runtime/fle_sysinfo.h` | `FLE_SYS_MAX_PROCESSORS 32u` を定義、`fle_sys_processors()` の説明を『host の事実』に | — |
| `lift/runtime/fle_sysinfo.c` | GLPI のバッファを `FLE_SYS_MAX_PROCESSORS` まで | — |
| `lift/runtime/fle_sys.c` | 定数 1 を返す `fle_sys_processors()` を、`#ifdef FLE_GIL` で **`EM_JS fle_js_hardware_concurrency`(`navigator.hardwareConcurrency`、無ければ 1)を 1 回だけ読む**関数に(`[1, 32]` に clamp、共有の static に原子的に保存)。**位置は `fle_imp_GetSystemInfo` の後**(srw のテストが `Slim reader/writer locks … GetSystemInfo` の区間を切って検査するため。0305 の fix 14 と同じ理由)。`FLE_GIL` なしの build は 1 のまま。GetSystemInfo のコメントを実装に合わせた。S3a は弱参照 `fle_cfg_processors()`(env の優先)を持つ | 弱参照と env の優先を除く |
| `lift/runtime/fle_thread.c` | `#include "fle_sysinfo.h"`(`FLE_GIL` の中)。**`nworkers_cfg` の既定 = `clamp(n − 1, 2, 8)`**(`n < 3 ? 2 : n > 9 ? 8 : n − 1`)。`FLE_WORKERS` があれば優先(0 または 1 は協調 scheduler のまま)。`FLE_PROCESSORS` を読む強い定義 `fle_cfg_processors()`(S3a のみ) | `getenv("FLE_WORKERS")`・`fle_cfg_processors()` を除く。コメント 2 か所(`FLE_WORKERS=2`)を直す |

- **GetSystemInfo の `dwNumberOfProcessors`・`dwActiveProcessorMask`・GLPI・`GetProcessAffinityMask`/`SetProcessAffinityMask` の mask は全て `fle_sys_processors()` を通る**(`fle_sys.c:627,629`、`fle_sysinfo.c:125,153,234〜246`)ので、1 か所の変更で 4 つの API が揃う。`GetNativeSystemInfo` は `GetSystemInfo` を呼ぶ。`fle_sys_proc_mask()` は `(1 << n) − 1`(n ≤ 32 で 64 bit に収まる)。
- **`FLE_GIL` の外(native・協調の wasm・test tree)は n = 1 のまま**: 『1 つの guest スレッドしか動かない build は、プロセッサ 1 個と答える』という従来の選択が保たれ、native のテスト・協調の決定的テストの期待値は変わらない。Windows が機械のプロセッサ数を返すのと同じ向きで、実スレッドの build だけが host の事実を返す。
- **CPUID(`fle_cpuid.c`)は触らない**: leaf 1 の EBX = 0(『論理プロセッサ数を主張しない』、決定 0009/0047)。トポロジの leaf(0xB 等)は『leaf 1 のデータを返す』規則なので、GetSystemInfo の n と矛盾する値は出ない。

## 2. 区分(契約の実装 / 余計の除去 / 通信・変換の削減)と No-options の自己点検

| 変更 | 区分 | 根拠 |
|---|---|---|
| GetSystemInfo/GLPI/affinity が host の論理プロセッサ数を返す | **契約の実装(外部 API の契約)** | Windows の `dwNumberOfProcessors` は機械の論理プロセッサ数。従来の『常に 1』は、guest スレッドが 1 本しか動かない前提の選択(`fle_sys.c` のコメントに明記)で、実スレッドでは前提が無い。FL が見るスレッド数・並列度の判断材料が実機に近づく |
| GLPI のバッファを n ≤ 32 に | 契約の実装(n に応じた出力の長さ) | 上の変更で n > 16 に届き得る箇所の溢れの防止 |
| Worker 数 = `clamp(n − 1, 2, 8)` | **三つの区分(契約の実装/除去/通信・変換の削減)のどれにも当たらない。Worker 数を決める規則そのもの = 選択(PM の暫定値)。PM の承認が要る** | 下限 2 = UI Worker + 他のスレッド用 1(実スレッドが唯一の構成〔D1(a′)〕なので 1 を許さない)。上限 8 = 8 を超える Worker は未測定(I15 の測定は 2/4/8 から)。`−1` の理由は指示に書かれておらず、**私の推測**は『present 用の Worker P(0311)に 1 コア分を残す』。旗(`FLE_WORKERS`)を除く代わりに規則を 1 つ置くことになる |
| `FLE_WORKERS`・`FLE_PROCESSORS` の除去(S3b) | **余計の除去(No-options)** | 利用者・測定者が選ぶ旗の廃止。S3a が測定のために残す(B1 の n = 2/4/8 の掃引を env で行うため) |
| `#ifdef FLE_GIL` の中だけ host の事実を返す | 既存の macro の使用。**新しい旗・環境変数・class/アドレスの分岐は無い** | — |

- **新しい旗・環境変数**: 無し(S3a で既存の `FLE_PROCESSORS`・`FLE_WORKERS` を残すだけ。S3b で 0)。
- **FL の挙動の補正・代行**: 無し。値を返すだけ。
- **自己点検で気になる点(未決)**: Worker 数の式 `clamp(n − 1, 2, 8)` は選択で、`clamp(…, 2, 8)` の 8 と `−1` に根拠は PM の暫定値以上には無い。n = 1〜2 の host でも Worker 2 本(UI + 1)で動かす = 1 コアの host では GIL の競合と Worker 間の切り替えで協調より遅くなりうる〔読み、未測定〕。

- **PM の判断(22:23)**: Worker 数の式 `clamp(n − 1, 2, 8)` は『**区分に当たらない選択**』と明記したうえで**測定用の既定として採る**。根拠は 3 つとも暫定値(`n − 1` = ブラウザの main/compositor に 1 コアを残す〔P の Worker が入ればさらに減る見込み〕。下限 2 = 実スレッド化の意味〔UI とミキサーが別 Worker〕が成立する最小。上限 8 = GIL 競合と V8 isolate のメモリの見込み)。**確定は S3a の bundle の n の掃引(`FLE_WORKERS` = 2/4/8 と hc 既定)の実測で行う。この式は『測定で変えうる選択』**。1〜2 コアの host で 2 本が協調より遅くなりうる点は、測れる機械が無いので『未測定・依頼者の機械は 0 件』のまま残す(読み)。E11 の測定が processors = 1 だった確認は PM が gpu1 の run の manifest で行う。

## 3. 製品の挙動への影響(読み。測定は S3a の bundle の後)

1. **FL が見るプロセッサ数が 1 から n(≤ 32)になる**: FL 本体・同梱 DLL(IPP の `dsp_ippv2_x64`、Blend2D、ILTools 等)のスレッド数・並列度の決め方が変わりうる。**起動時に作るスレッドの数が増えると、`pick_worker` が Worker を起こす数(`wk_start`)が増え、起動 P1(10 s 以内)に響きうる**。測るのは first idle の時刻と、作られた guest スレッドの数(`fleThreadReport`/`FLE_DBG_REPORT_S` の thread table)。
2. **Blend2D**: `BLContext` のスレッド数が機械のプロセッサ数に依存する版なら、描画が worker スレッドに出る(速くなる/GIL 競合が増える)〔読み。FL が `threadCount` を指定しているかは未確認〕。
3. **決定的な出力**: 従来『host の core 数は機械ごとに出力を変える』ので 1 に固定していた(`fle_sys.c` の旧コメント)。**Export の WAV の sha256 が n に依存するか**(FL のミキサーのスレッド分割)は、0308 の Export の非決定性の調査と合わせて確認が要る(n = 1 の E13 の sha `381cdb5a…f979` との比較)。製品の Export は『同じ入力に同じ出力』が望ましいので、**n に依らない出力か否かを S3a の bundle の最初の確認にする**。

## 4. 窓の後に行う確認(順)

1. `emcc -c -Wall -Wextra` の GIL あり/なし(`fle_sys.c`・`fle_sysinfo.c`・`fle_thread.c`)、native の clang(`-DFLE_GIL` なし)。警告 0。
2. native: `cargo test --workspace --no-run`、`sysinfo`・`sys`・`srw`・`k32_dsp` の binary を個別に、次に `test.sh full`(n = 1 のまま = 期待値は変わらない見込み)。a4。
3. wasm の node テスト: 小さなテストを `lift/harness/audio/tp_wasm/` の形で追加(実際の `fle_sys_processors` を emcc で wasm にし、node の `navigator.hardwareConcurrency` を差し替えて clamp〔0 → 1、64 → 32〕と『1 回だけ読む』を確認)。**未作成**(窓の後に、軽い範囲で)。
4. S3a の bundle(使い捨ての ci build、冷間 約 25〜35 分〔B1 の実測の目安〕)、gpu1 の測定: first idle(P1)、guest スレッド数、n = 2/4/8(`FLE_PROCESSORS` で guest の n、`FLE_WORKERS` で Worker 数)、Export の sha、PCM の零。
5. S3b は『S3a の測定が済んだ後』に積む。

## 5. 触っていない物・残り

- `lift/harness/mt2/node-bundle-run-rt.mjs` の冒頭コメントの `FLE_WORKERS=2`(S5 で直す。manifest の env を使う driver で、S3b の後は無視される)。
- `fle_thread.c` の `if (nworkers_cfg > FLE_MAXW)` は S3b の後は到達しない(上限 8 ≤ 32)。S4 の掃除で除く。
- 協調 scheduler と `rt_mode` の分岐は S4。

## 6. S3a の確認の結果(23:20〜23:25、バッチ窓。nice 19・JOBS 2。S0+S1 の木に S3a を当てた状態。実測)

- **emcc -c -O1 -Wall -Wextra**(`fle_sys.c`・`fle_sysinfo.c`・`fle_thread.c` を GIL なし/`-DFLE_GIL -DFLE_TLS_MORE -pthread` で 6 回): rc=0、**警告 0・エラー 0**。GIL ありの `.o` に `fle_js_hardware_concurrency`(`fle_sys`)と `fle_cfg_processors`(`fle_thread`)があり、GIL なしには無い(文字列で確認)。
- **native clang -c -O1 -Wall -Wextra**(同じ 3 ファイル): 出力なし(警告・エラー 0)。
- **`cargo test --workspace --no-run`: rc=0**。個別: **sysinfo 16 passed・sys 19 passed・srw 17 passed(`Slim reader/writer locks … GetSystemInfo` の切り出しのテストを含む)・k32_dsp 14 passed**、失敗 0。
- **`bash lift/harness/sysinfo_wasm/run.sh`: 39 checks ok**(S3a。`FLE_PROCESSORS` の優先を含む)。S3b のテキストでも **34 checks ok**。**最初の run は落ちた — 私のテストの誤り**(`instance a` は最初の呼び出しで値を決めるのに、`navigator` を変えた後で最初に呼んでいた)。`hc_test.mjs` を直した(「最初の呼び出しの値が保たれる」を、呼ぶ前に navigator を変えない形に)。
- 表: `tables/0312e/`(`s3a-results.txt`、emcc の log、cargo と 4 テストの出力)。
- **`test.sh full --jobs 1`(nice 19、23:30〜23:41:50、S0+S1+S3a の木): 219 of 219 executables + doc-tests = 223 results・2,430 passed・0 failed**(ignored 1、SKIPPED 1 行は従来の `wall_fr9t`)。S3a の前(2,430 passed)と同じ。`wall: total=703.9 s`(止めていない。以前の 3,798.9 s は私が止めていた時間を含んでいた)。ログ: `tables/0312e/s3a-test-full.log`。
- **a4(S3a 込み、23:43〜23:46:18、nice 19、JOBS=2)**: units 942・runnable 895/895・distinct 5,310・関数入口 1,009,737・exit 0 = **基準と同一**(gen 29 s・build 201 s)。生成 C `flengine.c` は sha256 `1d5d368d…` で **S0+S1 の a4 と `cmp` で完全一致**(native の `fle_sys_processors` は 1 のまま)。`tables/0312e/s3a-a4.log`。
- **`bfle_plain.sh`**: 既定の FLEngine の C = **215,741,085 B・sha256 `6c9d92b8…`**(S0+S1 の木の期待値と一致。fe-lift 9.07 s)。
- **S3a の bundle(B2)**: `fl-engine-ci-8b2eb5d7`(00:16〜00:38、rc=0、`SHA256SUMS ok`)。wasm 142,986,341 B(B1 の 142,986,125 B + 216 B)、sha256 `e1e2bf8a072d50b4…`。`CI_JOBS=1`・`BINARYEN_CORES=1`・nice 19: `objects` 15 s(`obj/rt` の 90 個だけ)、`link` 20 分 13 秒(`wasm-opt -Oz` 12 分 56 秒・CPU 99%)、`b2d` 54 s、`lift` 26 s。glue に `hardwareConcurrency` と `audio-ring`。表: `tables/0312e-b2/`。
- **未実施**: n の掃引(gpu1 の測定)。env なしの既定は n = 16・N = 8(この機械)で、**一度も走っていない構成**。


## PM の判断と取り込み(2026-11-06 22:32、docs のみ)

取り込み: mt3 の最終(0312 の一部)として `docs/investigations/mt3w/0312/` の表・道具・差分とともに置いた。判断は決定 0312 の PM の節にまとめた(段階 C は採る、T の基準が通るまで実スレッドは main に出さない、P を先に、S0+S1+P の bundle の測定、Worker 数の式は「区分に当たらない選択」として測定用の既定)。**ブラウザでの実行は 0 件、依頼者の機械は 0 件**。


### 追記(2026-11-06 23:29、§6 S3a のコンパイル・テストの結果の取り込み、docs のみ)

最新版(§6)を取り込んだ。表は `docs/investigations/mt3w/0312/tables/0312e/`(原本の `*.log` は `.txt` に改名)。**mt3 の実測(ビルド・native・node のみ、Chrome 0 件)**: emcc -c -O1 -Wall -Wextra(fle_sys.c・fle_sysinfo.c・fle_thread.c、GIL あり/なし)6 回で警告・エラー 0、native clang も 0、`cargo test --workspace --no-run` rc=0、sysinfo 16・sys 19・srw 17・k32_dsp 14 件通過、`bash lift/harness/sysinfo_wasm/run.sh` は S3a 39 checks ok(S3b の文面で 34 ok)。`test.sh full`・a4・bundle・n の掃引は窓の後(PM の info)。
