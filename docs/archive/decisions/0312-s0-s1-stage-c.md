# 0312(仮): S0 + S1 — E13(main d0447902)の上に 実スレッドの足場 + 16 件の修正 + D3a、その上に **段階 C**(音声リングと `wr`・`rdHi` を wasm の共有メモリへ。`fle_tp_*` を C に。page の `audio-ring`)(mt3。最終報)

状態: **仮**(最終報、使い捨ての木。製品の木・main には書いていない)。作業者 mt3、2026-11-06 JST。依頼: PM(0312)。基準: main d0447902(E13)。区分: **実測**=この作業のコマンドの出力を見た事実、**読み**、**未確認**。**Chrome は起動していない。実エンジン(wasm)のブラウザでの実行は 0 件**(B1 の bundle `fl-engine-ci-8301687b` は出来た。gpu1 がブラウザで測る)。

## 0. 冒頭の三点

1. **作った物**: (a) S0 = E13 の上に `rt-on-e11.diff` + 0305 の 16 件の修正 + D3a(5 本の積み重ねは clean)+ **17 件目の修正**(足場の `app/src/engine/fruity.ts` の診断ログ `[mt3] onExit/onAbort` が `fruity-engine.test.ts` を落としていた。E13 の形に戻した)。(b) S1 = 段階 C: 音声の出力リングを**エンジンの共有メモリの静的領域**に置き、`wr`(gap の基準)・`rdHi`(RD の 64 bit 展開)・unlock の記録・テレメトリを **C の共有状態**にし、JS には『ページへの 1 回の通知』『実時間の待ち』『2 Hz の pulse』だけを残した。page は `{type:'audio-ring', sab, hdr, data, cap}` を受けて AudioWorklet に渡す。**Worker ごとの JS 状態(`__fleAud`)も、gpu1 の測定専用 glue 補修 `patchglue-rt.py` も要らなくなった**。
2. **確かめたこと(実測)**: app の unit テスト 全体 **68 files・1,315 passed・7 skipped・0 failed**(この変更の新テストを含む)、tsc 3 構成 rc=0、**実際の `fle_audio_transport.c` を wasm にして node で動かす新しいテスト 46 checks ok + 2 本の pthread の hand-over テスト ok**(`lift/harness/audio/tp_wasm/run.sh`)、emcc での C の警告 0。**native の `cargo test --no-run` rc=0・`test.sh full` 223 results・2,430 passed・0 failed・a4 は基準と同一・B1 の ci 段 build は rc=0 で bundle `fl-engine-ci-8301687b` が出来た**(§5a)。
3. **未確認**: ブラウザでの実行(AudioWorklet が engine の共有メモリを実際に読むこと、`postMessage` で SharedArrayBuffer が page に届くこと、Worker 1 からの publish)、gpu1 の測定による T・UI Worker・重い区間・PCM の零、`lift/harness/web/audio/*`(0139 の prototype harness)と `lift/harness/mt2/*` の node harness で旧プロトコルを前提にした物(§4)。

## 1. 設計(0234l の設計を、足場の木の実際のコードに当てた形)

**リング(エンジン側、`lift/runtime/fle_audio_transport.c` の wasm の枝)**:
- `static int32_t tp_hdr[32]`(64 byte 整列。ページの `ring-layout.ts` の `W` と同じ語: RD 0、WR 1、STATE 2、RATE 3、CAP 4、MAGIC 5、VERSION 6、FILL_MIN 12、PUBLISHES 14、PUBLISHED 15、SKIPPED 16)と `static float tp_data[2 * 32768]`(interleaved L R)。`.bss` なので wasm の大きさは増えない。
- **共有の状態**: `tp_wr`(64 bit。最後の hand-over の末尾 = gap の基準)、`tp_rd_abs`(RD の 64 bit 展開。CAS で単調)。**以前の `A.wr`/`A.rdHi`/`A.rdPrev`(Worker ごとの JS 状態)の置き換え**。0305c の欠陥(Worker 0 の publish が Worker 1 の未読の窓を零で上書き)は、**状態が 1 つになったので構造上起きない**。
- `fle_tp_publish`/`fle_tp_rewrite`/`fle_tp_state`/`fle_tp_consumed` は **JS の `fle_js_publish`/`fle_js_rewrite` の意味をそのまま C にした物**(遅れた分の skip、隙間の零埋め〔容量で頭打ち〕、WR、カウンタ、rewrite の guard と WR による上限)。
- unlock の記録は C の配列(4000 件)、`waits`/`publishes`/`skipped`/`gapFrames`/`maxWaitDrift` は C の `double` の配列。ページの 1 Hz の報告用に、**最初の Worker だけ**が `globalThis.fleAudioWorkerState/Rewrite/Pace` を定義し、共有メモリを読む(`wasmMemory.buffer` を毎回取り直す。メモリが伸びても平気)。
- テレメトリ pulse(2 Hz)は、**最初の Worker のスレッド切り替え**からだけ post する(page に post できるのは最初の Worker だけ)。per-thread の合計(`sl_total` など)は共有、スライスの開始 `sl_start` は Worker ごと(`_Thread_local`)。
- 実時間の待ち `fle_tp_wait`(EM_ASYNC_JS)は状態を持たず、`waits`/`maxWaitDrift` は C 側で数える。

**ページとの取り決め**:
- page → エンジン: `{cmd:'audio-init', cap, rate, reportMs}`(**SAB は載せない**)。Worker hook(`worker-hook.ts`)は `globalThis.fleAudioHost = {cap, rate}` を立てる。
- エンジン → page: 最初の Worker が、スレッド層の最初の init(`fle_thread.c` の `ensure_init`、他の Worker が存在する前)で `fle_audio_host_init()` → `fle_js_announce` が `self.postMessage({type:'audio-ring', sab: wasmMemory.buffer, hdr, data, cap})` を**1 回**送り、`fle_audio_external()` = 1 を**1 回だけ決めて**共有の静的変数に保存する(`globalThis.fleAudioHost` が無い build は 0 で、仮想時計の経路のまま)。他の Worker が先に聞いても 0 を返して cache しない(実際には起きない)。
- page: `PageAudio.attachRing(sab, hdr, data)`(magic・容量・整列・SAB を検査して拒否できる)。AudioWorklet の node は**ring が付いてから**作る(`start()` が待つ)。それまでは page の placeholder の ring。`SoftSink` は ring が付いたときに新しい header で作り直す。AudioWorklet の `processorOptions` に `hdr`(byte offset)を足した。

**なぜ 1 つの共有 SAB(wasm の memory)で良いか**: メモリが伸びても(`ALLOW_MEMORY_GROWTH`)、古い SAB オブジェクトは同じ領域を指し続け、ring は静的領域(低い番地)なので範囲内(0234l §2)。

## 2. 変更したファイル(`impl/0312-s1-stage-c.diff` 12 ファイル、`impl/0312-s0-fix17-fruity-diagnostic.diff` 1 ファイル)

| ファイル | 内容 |
|---|---|
| `lift/runtime/fle_audio_transport.c` | wasm の枝の全面書き直し(上記)。native の枝は `fle_audio_host_init` の空関数を足しただけ |
| `lift/runtime/fle_audio_transport.h` | `fle_audio_host_init()` の宣言 |
| `lift/runtime/fle_thread.c` | `ensure_init` から `fle_audio_host_init()`(weak。音声の transport が無い build でも link できる) |
| `lift/harness/audio/sim_transport.c` | native の scenario の transport に `fle_audio_host_init` の空関数 |
| `lift/harness/audio/tp_wasm/{tp_test.c,tp_test.mjs,tp_threads.mjs,run.sh}` | **新しいテスト**(§3。`run.sh` は実行権限を付ける: `chmod +x`) |
| `app/src/audio/worker-hook.ts` | `audio-init` を SAB なしに(`fleAudioHost`) |
| `app/src/audio/page-audio.ts` | `initMessage`(SAB なし)、`attachRing`、`audio-ring` の処理、node の作成を ring の後に、SoftSink の差し替え |
| `app/src/audio/fle-out.worklet.js` | `processorOptions.hdr`(header の byte offset) |
| `app/test/unit/audio-wiring.test.ts`・`audio-default.test.ts` | 新しい取り決めのテスト(attachRing の検査、`audio-ring` の処理、hook)。既存の `start()` のテストは『エンジンの message を模す』ために `attachRing(placeholder)` を先に呼ぶ |
| `app/src/engine/fruity.ts`(S0 の 17 件目) | 足場の診断ログ `[mt3] onExit/onAbort` を除く |

旗・環境変数: **S1 では足していない・まだ除いていない**(`FLE_WORKERS`/`FLE_PROCESSORS` は S3〜S4)。新しい旗・環境変数・class/アドレスの分岐は無い。

## 3. テスト(実測)

- **app の unit 全体**(`npx vitest run`): 68 files passed・1 skipped、**1,315 tests passed**・7 skipped・0 failed。tsc(`tsconfig.json`・`tsconfig.test.json`・`tsconfig.webworker.json`)rc=0。
- **新しい wasm テスト `lift/harness/audio/tp_wasm/run.sh`**: 実際の `fle_audio_transport.c` を emcc で wasm にし(`-sASYNCIFY`、node)、page の役を node が演じる: (1) `fleAudioHost` 無し → external 0、何も post しない。(2) あり → external 1 を 1 回だけ決め、`audio-ring` を 1 回 post(cap 32768、offset 整列、header の magic・cap・rate・version・fill_min)。(3) 連続した hand-over(frames が first の位置に入り、WR が進み、PUBLISHES/PUBLISHED が数え、先の block を零で潰さない)。(4) RD の 64 bit 展開(32 bit の wrap をまたいで単調、古い小さい読みで戻らない)。(5) 遅れた hand-over(読まれた分は skip して SKIPPED を数える)。(6) 隙間は零、その前のデータは無傷。(7) rewrite(guard より近い物は書かない、guard の先は書く、WR は動かない)。(8) state/rate の語。(9) unlock の記録と `fleAudioWorkerState()` の値。(10) 最初の Worker の切り替えから pulse が post される。**46 checks ok**。**二本目 `tp_threads.mjs`(pthread + JSPI の製品の形で build)**: 実際の 2 本の pthread(= 実スレッドの 2 つの Worker)が giant lock の代わりの lock の下で交互に 100 フレームずつ 800 ブロックを hand over し、リングの最後の 32,768 フレームが全部『自分の番号のフレーム』で、**gap の零埋めが 0 フレーム**(`gapFrames` 0)、WR = 総フレーム数、publishes 800: **ok(80,000 フレーム、wrong 0)**。0305c の欠陥〔Worker ごとの `A.wr` で、別の Worker が publish した窓を零で潰す〕は、状態が共有の 1 つなので起きない。
- **C の警告**: `emcc -c -Wall -Wextra`(GIL あり/なし)、clang の native、`sim_transport.c` とも 0。

## 3b. 方針との適合(必須の記載)

- **lift の忠実性**: 新しい処理は FL の挙動の補正・代行・class/アドレスの分岐ではない。S1 は **(a) 余計なロジックの除去**(Worker ごとの JS 状態 `__fleAud`・遅延 init・測定専用の glue 補修を無くす)と **(c) 変換/通信の削減**(`fle_js_publish` は 1 frame ずつ JS の loop で `HEAPU8` から SAB へコピーしていた。今は C が共有メモリのリングへ直接書く = wasm→JS のコピー 1 回/公開が無くなる。0234l の見積り: 公開あたり 6.5〜7.0 ms の一部)のどちらかに当たる。外部 API(Win32)の契約は変えていない(`fle_tp_*` の意味は JS の版と同じ)。
- **no-options**: S1 で旗・環境変数を足していない。`globalThis.fleAudioHost` は『ページが音声を持つ』という**ホストの事実**で、利用者の選択肢ではない(以前の `fleAudioSab` と同じ役割)。
- **進捗の誠実性**: ブラウザでの実行は 0 件(上)。数字は実測だけ。

## 3c. gpu1 の present の別 Worker P(0311)と衝突しうる箇所(先に info)

- 足場(S0)が触る `app/` は 3 か所だけ: `app/src/main.ts`(`deskWindowsText` の test aid 2 行。S4 で外すか決める)、`app/src/engine/fruity.ts`(`importGlueFromBlob`: pthread の Worker に glue を渡すための 8 行)、`app/test/e2e/browser.mjs`(spike 用の `FL_WORKER_CONSOLE` の環境変数。テストの道具)。S1 が触る `app/` は `app/src/audio/*` と `app/test/unit/audio-*.test.ts` だけ。**present の別 Worker が `app/src/engine/`(client/worker)と `main.ts` の `AppApi` を触るなら、`fruity.ts` の `importGlueFromBlob` と `main.ts` の 2 行が衝突しうる**(音声の側は `install.ts` が名前 'fl-engine' の Worker を包む所だけが接点で、P の Worker が別の名前なら干渉しない)。

## 4. 触っていない物(正直に)

- `lift/harness/web/audio/{audio-worker.js,scen-worker.js,page.js,run-audio.mjs}`(0139 の prototype harness)は**旧プロトコル**(`self.fleAudioSab = {sab,…}`)のまま。新しい engine では `fleAudioHost` が無いので音声が外部 consumer にならない(= 仮想時計の経路)。この harness を使う作業(`build-scen-wasm.sh`)は新プロトコルに直す必要がある(S5)。製品の経路ではない。
- `lift/harness/mt2/*`(node の harness)は `fleAudioSab` を使っていない(grep)ので影響なし。
- native の ds_scenarios(`wall_fr15`)は `sim_transport.c` の API が同じなので無変更で動くはず(§5 の `test.sh full` で確認)。

## 5. native・a4・B1(結果は 5a)

- native: `cargo test --workspace --no-run`、`test.sh full`(nice 19、--jobs 1)。
- B1 の ci 段 build: `~/work/mt3w-s1/b1.sh`(= gpu1 の `run-ci-build.sh`、私の木 `tree-ci`、CIWORK `~/work/mt3w-s1/ci-b1`、cache は opt1 の x20 から `cp -al`、`CI_JOBS=2`、`GIL=1`、preset `F2xH7`、段 `extract felift lift b2d objects link seed bundle`)。bundle の場所: `~/work/mt3w-s1/ci-b1/engine/bundle/fl-engine-ci-<sha>/`。page は `~/work/mt3w-s1/app-dist`(`vite build`、`audio-ring` の処理を含む)。assets は gpu1 の `mk-assets.sh NAME BUNDLE_DIR` と同じ手(`FL_ASSETS_DIR`)。


### 5a. 結果(21:15 時点。実測)

- **native**: `cargo test --workspace --no-run` **rc=0**(20:07 → 20:11、専用の target dir、fe-* を clean してから)。**`test.sh full --jobs 1`(nice 19): 219 of 219 executables + doc-tests = 223 results・2,430 passed・0 failed・ignored 1・SKIPPED 1 行**(`fe-lift/wall_fr9t` の『FLE_FR9T_CLEAN_RT が無い』= 従来の skip)。E13 の clean 確認(219 results・2,417 passed)に対し **+4 executables(足場の gil_lockfree・semaphore・x87_threads・mx_hybrid_tls)・+13 tests**。`ds_scenarios`(`sim_transport.c` を使う wall_fr15)も通った。この run は 20:26:50〜21:01 の間、窓の規則で私が SIGSTOP していた(`wall: total=3798.9 s` にはその停止時間が入っているので **所要時間としては使えない**)。ログ: `tables/0312-s0s1-test-full.log`・`tables/0312-native-summary.txt`。
- **B1 の ci build**(gpu1 の `run-ci-build.sh`、tree-ci、CI_JOBS=2、GIL=1、F2xH7、nice 19): **21:11:55 に rc=0、`SHA256SUMS ok`**。bundle = `~/work/mt3w-s1/ci-b1/engine/bundle/fl-engine-ci-8301687b/`。engine wasm **142,986,125 B**(sha256 `d396496ad64b…`)、glue `de1dc3a6a29e…`(749,586 B)、dll flpack 77,444,112 B、seed flpack 7,248,243 B。stage の実測: `objects` 53 分 19 秒(PM の SIGSTOP 20:25〜21:00 を含む wall)、`link` 11 分 05 秒、`bundle` 4 秒。**link の `wasm-opt -Oz` は 3 分 53 秒・CPU 379%・max RSS 5.0 GB**(PM の窓の規則では並列の wasm-opt は不可だった。nice 19、ci1 の non-counted の時間。事前に絞らなかったのは私の見落とし)。glue の文字列: `audio-ring` 1、`fleAudioHost` 1、旧 `fleAudioSab`・`__fleAud` 0(**文字列の確認だけ。実行は未**)。表: `tables/b1-*`。
- **a4(S1 の木、5 モジュール、nice 19、JOBS=2、taskset 8-15、fe-lift は S1 の木の物)**(21:54 → 21:58:04、gen 28 s・build 205 s): **units 942・runnable 895/895 完走・実行された関数 distinct 5,310・関数入口 1,009,737・終了コード 0 = 基準と同一**。生成 C `gen/flengine.c` の sha256 は `1d5d368d…`(E13 の a4 は `2715c3f3…`)。**差は足場の 2 つの import 行だけ**〔実測: `diff` 10 行 = `fle_imp_CreateSemaphoreW`・`fle_imp_ReleaseSemaphore` の宣言・dispatch 表・import 表と、`fle_dispatch_count` 62935→62937・`fle_resp_count` 609→611。225,541,065 B → 225,541,451 B〕。**訂正**: 0305d の第 1 回の a4 の `1d5d368d…` を『汚染された fe-lift の出力』と書いたのは不正確で、これは足場を含む fe-lift の**正しい出力**だった(失敗〔`undefined reference to fle_imp_CreateSemaphoreW`〕は、その C を E13 の runtime〔CreateSemaphoreW の無い〕に link したため)。表: `tables/0312-a4.log`・`0312-a4-gen-C-diff-vs-E13.txt`・`0312-a4-gen-C-sha256.txt`。
- **gpu1 の present Worker 案(0311)との衝突の事前確認(PM 指示 21:55)**: §6 と `0312b-p-conflict-check.md`。
- **ブラウザでの実行は 0 件**(gpu1 が測る)。

## 6. gpu1 の present Worker 案(0311)との衝突の確認

→ `0312b-p-conflict-check.md`。textual な衝突 0(main HEAD・S0・S0+S1 の `app/` に `patch --dry-run` clean、offset は `main.ts` の 1 hunk で 2 行だけ)。P を S0+S1 の `app/` の複製に当てて tsc 3 構成 0 エラー・vitest 71 files で 1,323 passed・0 failed。重なるファイルは `main.ts` だけで、行は重ならない。実エンジンでの確認は未(S0+S1+P の bundle を 1 本測るとき)。

## 7. 0312 の最終(下書き。23:30 時点。窓の後の結果は『(追記)』の欄に入れる)

**区分**: 実測 = 出力を見た事実、読み = コードや表を読んだだけ、未 = 未実施。**実エンジン(lift した FLEngine の wasm)の実行は私の手では 0 件**(B1 は gpu1 が測定、PCM の零 3 chunk/0.036% は PM が確認済み)。

| 項目 | 状態 | 物 | 主な数字 |
|---|---|---|---|
| S0(E13 + 足場 + 16 件 + 17 件目 + D3a) | 実測 | impl/0312-s0-fix17-…diff、0305/0301 の diff | 17 件目: 足場の診断ログを除いて fruity-engine.test が通る |
| S1(段階 C。共有メモリに音声 ring) | 実測(unit・wasm テスト・native・a4・B1 の ci build) | impl/0312-s1-stage-c.diff、docs §1〜§5a | unit 1,315 passed、tp_wasm 46 checks + pthread 80,000 frames gap 0、test.sh full 223 results・2,430 passed、a4 基準同一、B1 rc=0(wasm 142,986,125 B) |
| 積み重ねの 1 本化 | 実測 | impl/0312-stack-on-main-e08b4589.diff、apply-stack-0312.sh | 80 ファイル +2,919 −281、5 本 reject 0、複製への適用が S0+S1 の木と byte 一致。**最新の main `fa0c5cc0` でも `apply-stack-0312.sh check` rc=0・単一 diff の `git apply --check` rc=0(23:57。e08b4589 から app/ と lift/〔lift/out-git を除く〕の変更は 0 ファイル)** |
| P(0311)との衝突 | 実測 + 読み | docs/0312b、0312f | textual な衝突 0、P を当てて tsc 0 エラー・vitest 1,323 passed |
| S0+S1+P の page の作り方 | `prepare` まで実測 | docs/0312f、tools/mk-s0s1p-page.sh | main eb2d0b9f で reject 0 |
| S2(T の B/C の梃子)・I15 の静的点検 | 読み | docs/0312c | B の本線は P、C1 = GIL の取り直しが第一仮説、添字の範囲外なし |
| S3a/S3b(host の hardwareConcurrency、Worker 数) | **emcc/clang/cargo/4 テスト/node テストまで実測** | impl/0312e-s3a/s3b-…diff、docs/0312e §6、impl/sysinfo_wasm/ | emcc/clang 警告 0、sysinfo 16・sys 19・srw 17・k32_dsp 14 passed、node 39 checks(S3a)・34(S3b)。GLPI のバッファ溢れ(n > 16)を発見・修正。Worker 数の式 clamp(n−1,2,8)は PM 承認(測定用の既定) |
| B-FLE の設計の読み | 読み | docs/0312g | 推薦: 静的結合 + B-FLE、dynamic linking は採らない |
| B-FLE の最小の確かめ | 実測(python、生成 C の照合)。**素の lift の基準で訂正(0312h §6)** | docs/0312h、tools/bfle_*.py、tables/0312h-plain/ | **Balance の本体の 95.6%(C の 86.5%)が FLEngine と同じ、固有 553 関数・4.63 MB(13.5%)、固有の 91.4% は Flanger と共通、共有された関数の call の 99.0% は共有先、FLEngine 自身に 12.0% の重複**(製品の C 基準の 82.2%/65.5% は、direct wasm に下げた 2,758 関数が基準に無かったための過小評価) |
| 生成器の変更の設計(方式 B)+ 書き換え script の試作 + 呼び出し時間 | 読み + python の展開テスト + emcc -c の大きさ + node の合成 bench | docs/0312i(§9 の訂正・§10 を含む)、tools/va_lit_rewrite.py、tools/bench/ | 戻りアドレス 71% は `T[0]+off`、展開テスト 3 関数一致、表経由の費用: 小さい関数 +0.1%・巨大関数 +9.2% code。**呼び出し時間: 小さい葉に近い関数 <1 ns、熱い call-heavy な `0x6ac8c0` は +57〜106%(cross call 1 回あたり約 +3〜3.7 ns)**。素の基準で Balance と共有する本体は FLEngine の実行された命令の 82.2%(葉で参照なし 38.6%・参照のみ 19.2%・call と参照 42.2%)。方式 B の見積り: 1 つ目 約 10.4 MB、2 つ目以降 約 1.1 MB、7 つで約 16.7 MB |
| (追記)test.sh full・a4 を S3a 込みで | **実測(23:41・23:46)** | tables/0312e/s3a-test-full.log、s3a-a4.log | test.sh full 223 results・2,430 passed・0 failed(wall 703.9 s)、a4 基準同一・生成 C は S0+S1 と cmp 一致 |
| (追記)bfle_plain.sh(素の FLEngine の lift で照合をやり直す) | **実測(23:47)。結果が大きく変わった** | tables/0312h-plain/、docs/0312h §6、0312i §9 | 既定の C 215,741,085 B・`6c9d92b8…`(期待どおり)。**Balance の本体 95.6%(C の 86.5%)が FLEngine と同じ、固有 4.63 MB(13.5%)、共有された本体で実行された命令は FLEngine の記録の 82.2%**(製品の C 基準の 2.3% は誤り) |
| (追記)候補関数の呼び出し時間 | **実測(合成)** | tools/bench/、tables/0312i/bench-40e800.txt | wrapper + 表の追加の 1 call は 1 ns 未満(`0x40e800`)。参照の多い関数・`call_indirect` は未測定 |
| (追記)S3a の bundle(B2)→ gpu1 の n の掃引 | **bundle は実測(00:16〜00:38、rc=0)。掃引は gpu1 の測定待ち(ブラウザ実行は 0 件)** | `ci-b2/engine/bundle/fl-engine-ci-8b2eb5d7/`、tables/0312e-b2/、tools/b2.sh | wasm 142,986,341 B(B1 +216 B)。link 20 分 13 秒(BINARYEN_CORES=1、wasm-opt 12 分 56 秒)、obj/rt の 90 個だけ再 build(15 s)。env なしの既定は n = 16・N = 8(未測定の構成) |

**自己申告(0312 全体。⑯⑰は 23:50 以降に追加)**: ①s0 の基準木で `tar` に `.` と `-T` を併用して repo 全体 86 GB をコピー(すぐ消した)。②`sleep 0` の拒否、`bash` という名前のツールの呼び出しの拒否が 3 回(いずれも実行されていない)。③20:07〜20:26 に B1 と native test を ci1 の extras と重ねた(『pure end が窓の合図』と誤読)。④B1 の link の `wasm-opt` が 21:04〜21:08 に CPU 379%。⑤『FLE_PROCESSORS でも n を変えられる』という誤った info(20:08・21:13)→ 22:21 に訂正。⑥0305d の a4 の『汚染された fe-lift』の表現の訂正。⑦P の複製の最初の vitest が 10 件落ちたのは `../lift` の欠如(環境の誤り)。⑧17 件目の修正の原因の推測を事実のように書いた(訂正済み)。⑨積み重ねの 1 本化の最初の `emit` に scratch の `.apply.log` が混入(作り直し)。⑩docs/0312c の行番号のずれ(直した)。⑪`link.map` の最初の集計で uppercase の symbol 名で section を切り替えて CODE が 131 KB に見えた(直した)。⑫**0312h §3 の B-FLE の数を A 案の値のまま方式 B の表に書いた**(0312i で訂正)。⑬S3a の node テストの最初の run は私のテストの誤りで落ちた(直した)。⑭書き換え script の最初の計測は同じ TU・static で LLVM が表を畳み込み無意味だった(作り直し)。⑮python の照合を gpu1 の Chrome 窓の最中に走らせた(nice 19、PM の『CPU 軽い作業のみ』『今から可』の範囲と読んだ)。
⑯**0312h・0312i の『65.5%』『2.3%』は、基準に製品の C を使った誤り**。0312h §2 に基準の限界を書きながら、0312i の結論(『共有部分はホットでない』)にその限界を反映せず断定的に書いた。素の lift の再実行で正した(0312h §6、0312i §9)。⑰0312i の最初の呼び出し時間の bench(`0x40e800`)は呼び出し規約を取り違えて trap した(直した)。


## PM の決定と取り込み(2026-11-06 22:32、docs のみ。製品の差分なし)

取り込み: mt3 の最終(0312・0312b・0312c・0312d・0312e)の文書・表・道具・差分を `docs/investigations/mt3w/0312/`(生ログ `*.log` は入れない。原本は `lift/out-git/mt3w-results/0312/`)に置いた。mt3 自身の実行は cargo/native/node/ビルドだけで**ブラウザでの実行は 0 件**。ブラウザでの測定は gpu1(決定 0311 の B1 の節)。**依頼者の機械は 0 件、本番 R2 の `perf/` に beacon は 0 件。FL 内蔵エフェクト 7 種 25 インスタンスと Fruity Wrapper が無視された負荷(G9 未到達)での値**。

事実として受け取る:
1. S0(足場 rt-on-e11.diff + 0305 の 16 件 + 17 件目〔fruity.ts の診断ログの除去〕+ D3a〔計測用の旗の除去〕)+ S1(段階 C: 音声リング 2×32768 フレームと wr/rdHi/unlock 記録を wasm の共有メモリの静的領域に置く。Worker ごとの JS 状態なし)。native `test.sh full` 223 results・2,430 passed・0 failed・SKIPPED 1(wall_fr9t)、a4 は units 942・runnable 895/895・distinct 5,310・entries 1,009,737(基準と同一。生成 C の差は足場の 2 import 行と dispatch/resp の count だけ)、cargo test --no-run rc=0、wasm の pthread テスト 46 checks(2 pthread + JSPI で 80,000 フレーム、gap 零埋め 0)。
2. B1 bundle `fl-engine-ci-8301687b`(wasm 142,986,125 B、CI 段 rc=0)。**gpu1 の実 Chrome 測定(決定 0311 の B1)**: 5 分再生で PCM の零 3 区間・790 フレーム(0.036%、測定 glue なし。協調 4 区間・0.038%、修正前の実スレッド 212 区間・19.63%)、distinct 書き込み 0.978〜1.000、12288 の割合 0〜2%、CPU/Unlock 最大 66 ms(30 s)・74 ms(10 s)、UI Worker 30 s 区間 0.49〜0.64・平均 0.583(PM が表から計算)、2 Worker 合計 最大 1.34 コア。**数の基準は全て満たす**(単発、effect なしの負荷、guest のプロセッサ数 1)。入力の真の T(ABBA 途中): File クリック B1 28.4 / 27.7 ms 対 協調 18.1 ms、Esc 14.1 / 12.1 対 4.8 ms = **「協調 +2 ms 以内」は満たさない**(E11 の実スレッドと同じ退行。B が +5、C が +5)。
3. 修正の発見: GetLogicalProcessorInformation のバッファが 16 プロセッサ分で、n を 32 まで許すと溢れる(S3a で修正)。0305b の「shadow stack 8 MiB × n」は誤りで、Worker の C スタックは 512 KiB(`fle_thread.c:1193`)。B1 では `FLE_PROCESSORS` は無視され(16 件の修正が getenv を除去)、効くのは `FLE_WORKERS` だけ。
4. P(決定 0311)との textual な衝突は 0: P の 3 本の差分は main・S0・S0+S1 の app/ に patch --dry-run で全部 clean(main.ts の 1 hunk が offset 2 行)。P を S0+S1 の app/ に当てて tsc 0 エラー・vitest 1,323 passed。
5. 積み重ねの 1 本化: `impl/apply-stack-0312.sh check|apply|emit` と `impl/0312-stack-on-main-e08b4589.diff`(80 ファイル +2,919 −281)。5 本とも main e08b4589 に `patch --dry-run` rc=0・reject 0、1 本の diff は PM の木に `git apply --check` rc=0、当てた結果が S0+S1 の木と byte 一致。
6. S2(T の B/C): 読み。B の本線は P(present を Worker 0 から出す)。「page が Atomics.notify」案はほとんど効かない(Worker 0 の待ちは既に waitAsync + JSPI)。B2(メッセージキューの問い合わせ入口で ring を読む)は P の後に B > 1.5 ms の時だけ。C の第一仮説は UI スレッドの GIL の取り直し(`fle_gil.c:71` の 2,000 回 spin → `:90` の futex)。計器は D3a の `patch -R`(S0+S1 の木に clean)。
7. S3(0312e、build/test 未実施): S3a = host の `navigator.hardwareConcurrency` を [1,32] に clamp して GetSystemInfo/GLPI/affinity に返し、Worker 数の既定を `clamp(n−1, 2, 8)`(env があれば優先)、S3b = env 除去。

判断:
1. **段階 C(S1)は採る**: 製品の形で零の欠陥が出ず、数の基準を満たす。ただし**入力の T が「協調 +2 ms 以内」を満たさない間は、実スレッドを main に出さない**(旗の除去 S4 にも進まない)。T の本線は P(決定 0311)で、P の穴(IdleDriver が idle で処理した入力の答え)の修正と 43/s の状態の Esc を直したうえで、**S0+S1+P の bundle(mt3 が `apply-stack-0312.sh` の上に P を当てて CI 段 build、冷間 約 45〜60 分)を gpu1 が測る**。順序: E14 = P(単独、協調で基準を満たしたら main へ)→ S0+S1+P の測定 → 実スレッドの main 化。
2. **Worker 数の式 `clamp(n−1, 2, 8)` は「契約の実装・余計の除去・通信/変換の削減」のどれにも当たらない選択**として docs に明記し、測定用の既定として採る。定数(−1: ブラウザに 1 コアを残す、下限 2: UI とミキサーが別 Worker、上限 8: GIL 競合とメモリ)は暫定で、S3a の bundle の n の掃引(FLE_WORKERS = 2/4/8 と hardwareConcurrency の既定)の実測で変えうる。1〜2 コアの host で 2 Worker が協調より遅くなるかは測る機械が無く、未測定のまま。
3. D1(a')・D2・D3 は仮のまま(旗の除去 S4 は T の基準が通ってから)。S3a/S3b の build と test は窓の後。
4. 未確認: B1 の T の最後の 1 本、P と組み合わせた T、n=2/4/8、Firefox、実スレッド版の 120 Hz、依頼者の機械。


### 追記(2026-11-07 00:42、mt3 の 0312 最終の取り込み、docs のみ。製品の差分なし)

取り込み: mt3 の 0312 最終(本文を差し替え、上の PM の判断の節は残した)と `0312e` の最新版、表・道具(`docs/investigations/mt3w/0312/`。原本の `*.log` は `.txt` に改名)。**実エンジン(lift した FLEngine の wasm)のブラウザでの実行は mt3 の手では 0 件**(B1 の PCM の零・T は gpu1 の Chrome の測定)。

PM の検証(実測):
1. **積み重ね**: `apply-stack-0312.sh check`(main `fa0c5cc0` の PM の木、81 ファイル): 5 本すべて rc=0、rejects: 0(offset は 2 本目の 1 hunk〔19 行〕のみ)。mt3 の報告と一致。
2. **S3a の bundle**(`fl-engine-ci-8b2eb5d7`、wasm 142,986,341 B = B1 の 142,986,125 B + 216 B): `SHA256SUMS` の manifest・seed・.br は OK。manifest の `engine.env` に `FLE_WORKERS`/`FLE_PROCESSORS` は無く(env なし = 実スレッド、n = hardwareConcurrency〔1〜32 に clamp〕、N = clamp(n−1, 2, 8)。この機械は hc = 16 で n = 16・N = 8)、B1 とも E13 とも違う**未測定の構成**。
3. S3a の確認(mt3 の実測): emcc/native clang の `-Wall -Wextra` 警告 0、`cargo test --no-run` rc=0、sysinfo 16・sys 19・srw 17・k32_dsp 14 件、node の clamp テスト 39 checks、`test.sh full` 223 results・2,430 passed・0 failed、a4 基準同一(942/895/5,310/1,009,737、生成 C は S0+S1 と cmp 一致)。

判断:
1. **S0+S1+S3a は main に入れない**(実スレッド版は T が協調 +2 ms 以内になるまで main に出さない。B1 + P の Esc は 11〜24 ms、V5 は効かなかった。決定 0311)。bundle は gpu1 の n の掃引と P1(first idle)の測定用の局所物。
2. **gpu1 の測定順**: 先に E14 の V4(協調 + P、次の Chrome 窓)。その次の窓で S3a の対照(`FLE_WORKERS=2 FLE_PROCESSORS=1` = B1 と同じ n=1・N=2)→ n=4 → n=8 → 既定(n=16, N=8)。既定は起動で止まる可能性があるので最後。記録には n と N を必ず書く。Export の sha(381cdb5a…f979)が n に依らないかも見る。
3. mt3 の自己申告 17 項目(86 GB のコピー、窓の誤読、vitest の環境の誤り、0312h/0312i の数字の誤りなど)はすべて訂正済みで、行為の規則に反する物は無いことを確認した(窓の合図の誤読は私の側の誤りが元で、`performance-targets.md` §4e に開示済み)。
4. 次の担当: 0312j(Split mixer tracks の `WriteFile: out of memory` の設計の読み。実装はしない)。S3b(env の除去)は n の掃引の結果の後。
