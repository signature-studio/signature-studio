# 0315g(仮): RT-D3 の build の手順(固定。mt3、PM の判断 14:1x)と、build 前に確かめた物

状態: **仮**。区分: **実測**=この作業のコマンドの出力を見た事実、**読み**、**未確認**。**実エンジン(wasm)の実行・Chrome・Wine は 0**。

## 0. RT-D3 の内容(PM 確定、14:0x)
RT-D2(= RT-B + alpha1 の Filter 診断の第 3 版。`fl-engine-test-rtd2-297e2ac6`)の runtime に、次の TEST 専用の診断を足す。どれも**観測のみ**で、スイッチは manifest の**コピー**でだけ入れる(bundle の engine.env に FLE_DIAG_* は入れない)。

| 診断 | ファイル | スイッチ | 内容 |
|---|---|---|---|
| atom の再入 | `fle_diag_rtd3.c/.h`(新)+ `fle_glob.c` の弱い参照つき 2 行 | `FLE_DIAG_ATOM=1` | `atom_arg` の `malloc` の前後を per-thread(`_Thread_local`)に数え、最初の 200 呼びの 文字列・Worker・ゲスト thread・callback の深さ・enter/exit を stderr に出す。同じ thread で開いた malloc が残ったまま次の呼びが来たら `RE-ENTRY` と書く |
| GIL の待ち・保持 | `fle_gil.h`(取得・解放に `fle_gil_obs_on` の分岐 1 つずつ)+ `fle_gil.c`(観測器) | `FLE_DIAG_GIL=1` | Worker 別に 待ち(最初の CAS の失敗〜取得)と保持(取得〜解放)の分布 `<0.1 / <1 / <5 / <20 / <50 / ≥50 ms` の件数と最大、保持の長い API の上位 8。1 秒ごと。保持は速い経路では 16 回に 1 回だけ時計を読む |
| name section | build の仕組み(下) | (常に。診断 bundle のみ) | wasm の関数に名前が付く。stack を名前で読める |
| (alpha1 の slot 状態 / master の診断) | 受領後(14:40 の時点でテストを通っていれば) | 未定 | 通っていなければ RT-D4 |

## 1. 木とテスト(実測)
- `tree-rtd3`(build の形:FLE_DIAG_* は getenv)と `tree-rtd3t`(テストの形:FLE_TEST_GETENV。native テストはこちら)。**build の形 = テストの形 + `FLE_TEST_GETENV("FLE_DIAG_` → `getenv("FLE_DIAG_` の sed だけ**を、`rtd3-build.sh tree` が 5 ファイル(fle_diag_rtd3.c/.h、fle_gil.c/.h、fle_glob.c)の sha で照合する(OK)。link-web.sh 不変。
- native テスト(実 cargo、nice 19、JOBS 1、テストの形の木): **43 binary、passed 646、failed 0**(14:05:37〜14:08:45)。= fle_gil.h または fle_diag_rtd3.c を include する runtime ファイルを compile するテスト(PM の判断:全体の `test.sh full` は不要)。先に出した『46』は数え違い。
- native で fle_glob.c + fle_diag_rtd3.c を本物のまま驅動(Add/Find/GlobalAdd):行が出る・整数 atom は行なし・スイッチ OFF とファイル無しのリンクは無出力で動作同一。emcc `-Wall -Wextra`(GIL あり/なし)警告 0。
- 失敗から学んだ 2 点: (1) 診断を `fle_glob.c` の中に書くと、同じテストが `_exit` の語を禁じ、link に `fle_callback_depth` が無く、glob のテストを 17 件壊した → 別ファイル + 弱い参照。(2) build の形の木でテストを回すと callback の守りが `getenv` で落ちる(コードの欠陥ではない)→ テストはテストの形の木(別の target dir)で回す。

## 2. name section(小さい実験で実測。実エンジンの大きさは build して測る)
名前が最後の wasm に残るには**3 か所の指定**が要る(emcc の実物と Binaryen の実物で確認):
1. link に `--profiling-funcs`:`EMCC_CFLAGS` は emcc の compile+link の両方に効く。build script の既存の hook `SPIKE_CFLAGS`(work tree 専用、`wasm_env` が `EMCC_CFLAGS` に足す)で入れる = **script は変えない**。compile では『linker flag ignored』の警告が各 rt object の log に 1 行出るだけ(object はバイト同一、実測)。link-web.sh は触れない。
2. `wasm-opt -Oz` に `-g`:無いと名前を捨てる。
3. `wasm-merge` に `-g`:無いと捨てる(名前の無い module と合成しても、名前のある側は残る)。
2・3 のために、`run-ci-build.sh` が毎回 `$CIW/wrap/wasm-opt`・`wasm-merge` を **time -v の wrapper で上書きする**ことに合わせ、**wrapper の printf の 1 行だけ `-g` つきにしたコピー `ci-rtd3/run-ci-build-g.sh`**(sed 1 本。他の行が同一なことを `rtd3-build.sh all` が確かめてから走る)を使う。gpu2 の共有の道具は編集しない。生成された wrapper(`time -v ... wasm-opt -g "$@"`)を実際に動かして、wasm-opt も wasm-merge も名前を残すことを確認した(実測)。
実験の結果(小さい program):名前を付けても**コード部分はバイト同一**で、増えるのは name section だけ。実エンジンの増分(サイズ・link と wasm-opt の時間)は **build して実測する**(見積りは出さない)。

## 3. 手順(14:40 に 1 コマンドずつ)
1. (alpha1 の差分が来て、PM の許可があれば)`bash rtd3-build.sh addpatch DIFF`:テストの形と build の形の両方の木に当てる(dry-run が両方通らなければ止まる)。差分が触るファイルの native テストを `d3-native.sh` の形で回す(nice 19、JOBS 1)。通らなければ**足さない**(RT-D4)。
2. `bash rtd3-build.sh tree`:2 つの形の照合。
3. 開始前に free / vmstat を記録(MemAvailable ≥ 8 GB、swap OUT 0)。`free -m -s 120 -c 40 > rtd3-free.log` を別に(timeout 0)。
4. `bash rtd3-build.sh prep`(`ci-rtd3` = `ci-rtd2` のコピー − bundle・w-Oz/web・w-Oz/obj/rt。**済み**:14:11)→ `bash rtd3-build.sh all`(background、timeout 0。guard < 7 GB、nice 19、JOBS 1、GIL=1)。
5. 完了後 `bash rtd3-build.sh rename`(`fl-engine-test-rtd3-<hash>`、README 入り、fix-test-sums)→ `bundle-report.sh`。報告:rc、SHA256SUMS、wasm の大きさ(RT-D2 165,697,223 B との差 = name section)、**link と wasm-opt の時間の増(RT-D2:link 1478 s、wasm-opt 998 s)**、最小 MemAvailable、wasm の中の `FLE_DIAG_ATOM`・`FLE_DIAG_GIL` の文字列、dll/seed/glue が RT-B と同一か、engine.env に診断なし、name section があること(`wasm-objdump -h`)。
6. RT-B・RT-D・RT-D2 の bundle(`ci-rtb`・`ci-rtd`・`ci-rtd2`)は**読むだけ**。

## 4. 未確認
- 実エンジンでの name section の大きさと時間の増(build 後に実測)。
- ~~`fe-wasmgen` が出す direct module の関数名が `wasm-merge -g` で残るか~~ → **実測で答えが出た(14:14)**:RT-B の `hyb/dir/n-fle.wasm`(fe-wasmgen の出力、6,341,173 B)・`n-fle.oz.wasm`・`direct.wasm`(10,582,930 B)には **name section が無い**。= **hybrid で lowered された関数(direct module の側)は名前なしのまま**で、名前が付くのは C 部分(runtime・libc・dlmalloc・lifted の C の関数)だけ。今回の mutex の stack の 8 frame(dlmalloc・malloc・`fle_w_read_alloc`・`fle_glob.c`)はすべて C 部分なので、name section で読める。direct 側の関数の名前が要る調査になったら、`fe-wasmgen` が name section を出す形に直す別の仕事(対応の取り方は未確認:`hyb/dir/n-*.tsv` は VA ごとの行〔列: va, insns, native_bytes, wasm_code_bytes, sig_in, sig_out, elided, status, indirect〕で、**関数 index の列は無い**ことだけを確認した。direct module の関数 index と VA の対応は、export 名か function の並びから別に引く必要がある)。
