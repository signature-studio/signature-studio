# 0149: テスト全体が 45〜85 分かかる原因の実測と是正 — 時間の大半は『テストが意図的に abort する子プログラム』が core-dump ハンドラ(apport)の待ち行列で待っていたこと。素の `cargo test` 5.4 分、`full` は全部を本当に実行して 61〜84 秒、`affected` は 0〜89 秒。仮の決定

状態: **仮の決定**。依頼・番号 0149 は PM。調査・設計・実装・測定・文書は作業者(`testspeed`、2026-11-02 21:09〜 JST)。基準の測定(§2)は HEAD **cd8c6d9**、成果物の差分と受け入れ測定(§1・§4)は PM の指示で HEAD **2b3a2f2**(決定 0148 取り込み後、コミット 228)の `git archive` から作り直した。リポジトリの追跡ファイルは変更していない(差分は `testspeed.diff`)。

**依頼者の指示(原文)**: 「テストに30分以上もかかる状態は明らかに異常です。時間がかかる要因を調査し、是正してください。遅くとも3分以内に『全テストが完了』もしくは『必要な場所のみテストが実行される』状態にしてください。」

区分の凡例は決定 0121〜0146 と同じ。**実測**＝このセッションのコマンドの出力を見た事実。**読み**＝コードやログの解釈(確かめていない部分を含む)。**未確認**＝確かめていない。

**肯定的に見せる点は限られる。** (1) **すべての時間は共有マシンの上の測定**で、測定の開始時の load average は 5〜26(vclock1・vis1 の作業と、自分の直前の測定の尾を含む)。静かな時間帯の再測定はできなかった。(2) `full` が 60 秒台から 80 秒台に収まるのは、下のとおり**テストファイル 1 本の変更**(PM の許可、§3.8)のおかげで、それまでは 1 テスト 96 s が下限(122〜128 s)だった。今の下限は『全 binary の CPU 総量 約 950 s ÷ 使えるコア数』で、12 コア制限なら約 80 s、16 コアなら約 60 s。(3) 『必要な場所のみ』の選択は **strace で取った入力集合を信じる**ものなので、テストが env・時刻・ネットワークで読む値と、git 管理外の入力(DLL・種)の変更は `affected` では見えない(結果キャッシュの層は DLL・種の内容を見る)。(4) 変異検査の最初の版で 4 件が `full` でも検出されなかった(テストがその定数を固定していない。選択の偽陰性ではなく、テストの穴)。(5) `FLE_FR9T_CLEAN_RT` の SKIPPED 1 件は埋めていない。

## 1. 先に結論

| 項目 | 結果(実測。HEAD 2b3a2f2 + この差分。括弧内は開始時の load average) |
|---|---|
| **現行の全体テスト**(`cargo test` と同じ順に 1 binary ずつ、基準の通し。HEAD cd8c6d9) | 137 binary・140 results 相当、**壁時計の合計 5,065 s = 84.4 分**、CPU 1,540 s(CPU/壁時計 0.30=ほぼ待ち)、2,059 passed・0 failed。依頼文の 45〜60 分より長いのは、同じ core-dump ハンドラを他の作業の通しと取り合ったため(§2.1) |
| **要因の上位 5 つ** | ① **core-dump ハンドラ(apport)の待ち**: 1 回の通しで約 2,100〜2,500 回、ハンドラはマシン全体で毎分 57〜59 件(≒1 件/秒)で飽和。RLIMIT_CORE を 1 byte にしただけで 5,065 → 1,001 s。② **生成プログラムが自分で `setrlimit(RLIMIT_CORE,0)`** を呼ぶ 3 テスト(0 ではハンドラは止まらない): 1,001 → 792 s ③ **Rust が opt-level 0** で 22〜40 MB の DLL を解析: 792 → 421 s ④ **clang が同じ runtime 60 本(約 3.5 万行)を毎回コンパイル**(4,032 回、約 235 万行): 421 → 326 s(素の `cargo test`)⑤ **binary を 1 本ずつ直列に実行**、および**1 つのテスト(2^32 入力×6 値を 1 プロセスで直列、96 s)が下限**: 326 → 約 120 s → §3.8 の分割で 61〜84 s。ビルドは要因ではない(コールド 9.1 s=0.2 %) |
| **`affected` の 5 通り**(コールド / ウォーム、秒) | runtime/`fle_gdi.c` 1 行: **27.6 / 18.8**(64/138 binary)。`fle_win.c` 1 行: **25.0 / 27.3**(64/138)。fe-lift/src/`string.rs` 1 行: **88.6 / 39.6**(133/138、ビルド 19.8 s を含む)。テストファイル `tests/gdi.rs` 1 行: **24.5 / 8.6**(25/138、ビルド 17.3 s を含む)。docs のみ: **0 binary**(2.1 / 0.3 s、cargo のビルド確認だけ)。すべて 180 秒以内。load 18〜24 の下 |
| **`full`**(`harness/test.sh full`) | **141 results・2,068 passed・0 failed・0 ignored**(PM の基準 141 / 2,068 / 0 と同数)・SKIPPED 行 1(`fe-lift/wall_fr9t`、FLE_FR9T_CLEAN_RT 未設定=従来どおり)。壁時計: 結果キャッシュ**ウォーム 0.5 s**(138 binary すべて cached、2,068 テストを『実行していない』と別に数える) / **キャッシュ全部無効 `FLE_TEST_NOCACHE=1`(全部を本当に実行)**: 12 コア制限・6 GB で **83.7 s**(20.4)、**16 コア全部・8 GB で 75.1 s**(23.9) / clang キャッシュ空・記録なし: 12 コアで 75.5 s(21.5)、**16 コアで 61.4 s**(26.1) / キャッシュ全部空・初回の記録(strace)つき(12 コア): 105.0 s(14.8) |
| 素の `cargo test --workspace --no-fail-fast --offline`(並列化も選択も無し。cd8c6d9) | **326 s = 5.4 分**、140 results・2,059 passed・0 failed(`FLE_TEST_NOCACHE=1` で clang キャッシュ無しなら 421 s = 7.0 分) |
| 選択の健全性(変異検査) | **15 件**(runtime の .c 6・.h 2、harness の .c、種 tsv、テストファイル、fe-lift/src 2、fe-decode/src、fe-pe/src、runtime への新規ファイル): **15/15 で `affected` も `full` も同じ失敗テストを検出。偽陰性 0**(HEAD 2b3a2f2 の木で全件取り直した。うち 2 件は `FLE_TEST_NOCACHE=1` の `full` でも)。最初の版(cd8c6d9)で `affected` も `full` も検出しなかった 4 件は定数がテストで固定されていなかった(§4.3) |
| 分割したテスト(`sse_legacy::rsqrtps_all_2e32_inputs_match_host`) | 単独 96 s → 12.5〜16.6 s(FLE_TEST_THREADS=12 / 既定=コアの半分)。名前・検査内容(2^32 入力×6 値の全件照合)は同じ。範囲の被覆はテスト内のアサートで証明、不一致の仕込みは 12/12 検出(§3.8、§4.6) |
| テストの削除・弱体化 | 0。`#[ignore]` 追加 0。変更したテストファイルは **`tests/sse_legacy.rs` の 1 テスト(本体)と `tests/rsqrt/rsqrt_exhaustive.c` の 2 環境変数のみ**。`cargo test --workspace -- --list` の出力の SHA-256 は HEAD 2b3a2f2 の無変更の木と変更後の木で**同一**(`17766652…160ce2`、2,343 行) |
| SKIP | `full` の SKIPPED 行は **1**(`wall_fr9t` の FLE_FR9T_CLEAN_RT)。以前の通しで SKIP していた dsp の 2 テスト(`DSP_IPPV2_PE` を非再基底版にしていたため)は、変数を**未設定**にして実際に走らせた(§6) |

## 2. 要因の実測(詳細は `profile.md`。基準の通し・要因の分解は HEAD cd8c6d9 の木)

### 2.1 core-dump ハンドラの待ち(最大の要因)

- `core_pattern` は `|/usr/share/apport/apport …`、`ulimit -c` は 0。**RLIMIT_CORE が 0 でもハンドラは起動する**(0 を『無制限』と扱う設定ではなく、パイプ先には 1 だけが『起動しない』特別値)。
- 1 行 `abort()` のプログラム: **3.0 / 4.3 / 5.1 s**(3 回)、SIGSEGV は 5.0 s。`prlimit --core=1:1` を付けると**どちらも 0.00 s**。
- `/var/log/apport.log`: マシン全体のハンドラ起動は**毎分 57〜59 件でほぼ一定**(21:06〜22:41 の毎分。測定窓 87 分で 4,917 件=56.5 件/分、最大 59)。待ち行列が飽和していて、vclock1・vis1・自分の通しが取り合う。自分の木の分は 2,108 件。過去の全体テスト 1 回あたりは verify4 の木で 2,489 件(g8a1 の木は複数回分で 4,761 件)。**1 件/秒なら 1 回 ≥ 41.5 分**で、これが 45〜60 分の正体。他の通しが同居するほど延びる(基準の通しの 84.4 分)。
- 名前に abort / stop / aborts / stops / rejects / panic / crash / segv を含むテスト **388 本**の exec_time の合計が 10,549 s(中央値 9.4 s)、残り 1,671 本は 3,048 s(中央値 0.15 s)。テスト関数の上位 30(基準の通し)のうち **27 が名前に stop / abort を含む型**(1 位 `fe-lift/sysmetrics::every_other_index_stops_by_number` 332.4 s、CPU は 0.7 s)。残り 3 は `k32_dsp::is_processor_feature_present_agrees_with_the_cpuid_answer` 109.1 s、`adjustor::adjustor_roots_add_602_functions_and_no_bad_nodes` 100.1 s、`sse_legacy::rsqrtps_all_2e32_inputs_match_host` 96.2 s。
- 同じ binary を同じ順に 1 本ずつ、RLIMIT_CORE を 1 にしただけ(他は無変更)で走らせた(V1): 5,065 → **1,001 s**(CPU は 1,540 → 1,497 s で同じ=待ちだけが消えた)。例: `fe-lift/spi` 153.3 → 2.6 s、`reg` 146.3 → 10.0、`wall_fr4` 133.2 → 2.7、`class` 114.1 → 2.8、`load` 105.5 → 2.4、`a9_a10` 86.1 → 1.4、`wall_fr13` 51.0 → 0.7。
- **例外 3 本**: 生成した C プログラムが自分で `setrlimit(RLIMIT_CORE,{0,0})` を呼ぶ(`cursor.rs` 110 行、`loadimage.rs` 139 行、`wall_fr10.rs` 887 行。『stop は abort で、core ファイルは遅くするだけ』という意図)。継承した 1 を 0 に戻してしまい、`fe-lift/cursor` は V1 でも **165.8 s**(1 テストが abort を約 25 回直列に)。→ `LD_PRELOAD` の小さな .so(`setrlimit` / `prlimit` の RLIMIT_CORE の 0 を 1 に読み替える)で解決(V2: 1,001 → 792 s。cursor 1.3 s、loadimage 1.1 s、wall_fr10 2.1 s)。テストファイルは変更なし。
- 副作用(実測): カーネルが crash ごとに `coredump: …: RLIMIT_CORE is set to 1, aborting core` を 1 行ログに出す(`journalctl -k`)。全体テスト 1 回で約 2,500 行。

### 2.2 Rust のテストが opt-level 0(CPU 律速、1 スレッド)

core 待ちを除いた後に残る長い binary はすべてこれ。V2(792 s)のうち、上位: `fe-lift/sse_legacy` 97 s(後述)、`adjustor` 82 s(CPU 97)、`vcall_thunk_dll` 51〜70 s(CPU 89)、`k32_dsp` 34〜47 s、`call_iat_dll` 47〜49 s(1 テスト)、`seh` 31〜33 s(CPU 106、RSS 1.2 GB)、`msvc_switch` 17〜21 s(CPU 66)、`jt_cpu` 17 s、fe-decode の `jumptab` 16 s・`adjustor` 14 s・`jt_cpu` 13 s・`msvc_switch` 14 s(CPU 57)・`vcall_setjmp` 12 s。DLL(`FLEngine_x64.dll` 22 MB は **54 binary**、dsp 40 MB は 8 binary)の解析と lift が、テストのプロセスの中の Rust で走る。`Cargo.toml` の `[profile.dev.package.*]` の opt-level(fe-pe・fe-decode・iced-x86 を 2、fe-lift を 1)で、V2 → V3: 792 → **421 s**(表は `profile.md` §2: fe-lift/adjustor 82.3 → 17.1 s、call_iat_dll 48.8 → 15.4、seh 31.2 → 10.6、vcall_thunk_dll 70.3 → 16.2、fe-decode の 6 本は 12〜16 → 1.8〜4.0 s)。代償はビルド時間: コールド 9.1 → 29.1 s、fe-lift/src を 1 行直したあとの再ビルド 5.7 → 20.6 s(§8)。**overflow-checks と debug-assertions は dev のまま**なので、整数オーバーフローの panic・`debug_assert!` は引き続き働く(opt-level とは別の設定)。

### 2.3 clang が同じ runtime を毎回コンパイル

- 基準の通しの記録(`wrap.tsv`): clang 3,706 回(CPU 531 s=通し全体の 34 %)。**`runtime/fle_*.c` のコンパイル 4,032 回**=60 本(35,033 行)を約 67 回ずつ、約 235 万行(コンパイルした C 440 万行の 53 %)。ほとんどが `-O0`(3,554 回)、`-g` 付きは 6 回だけ(『`-g` が付く』は要因ではない)。
- 本物の clang で runtime 60 本の compile+link: 2.5 s(CPU 2.5 s)。シム(ヒット)では 0.06 s。リンクだけは 1 回約 30 ms。
- 素の `cargo test`(runner 経由): clang キャッシュ無し 421 s → あり 326 s(CPU 825 → 661 s)。並列の `full` では壁時計に出ない(1 テストが下限)が、CPU は NOCACHE 1,022 s に対しキャッシュ空から始めて 782 s。
- 同じ入力を何度も lift / 解析すること(`FLEngine_x64.dll` を 54 binary が各自解析)は、プロセスを跨げないので共有しなかった(opt-level で CPU を減らした)。

### 2.4 直列実行

`cargo test` は 137 binary を 1 本ずつ。基準の通しの CPU/壁時計 = 0.30、V2 でも 1.8 コア。16 コアのうち平均 2 コア未満。

### 2.5 ビルドは要因ではない

`cargo test --workspace --no-run`(コールド、12 並列): **9.1 s**(user 61 s、RSS 410 MB)。基準の通し 5,065 s に対し 0.2 %。

## 3. 是正(テストの検出力を落とさない)

1. **RLIMIT_CORE を 1 byte に**(`harness/testfast/runner.sh`、`prlimit --core=1:1`)+ `harness/testfast/nocore.c`(LD_PRELOAD で `setrlimit(RLIMIT_CORE,0)` を 1 に読み替え)。**`lift/.cargo/config.toml` の `runner`** にしたので、素の `cargo test` にも効く(並列と選択以外のすべてが素の `cargo test` で使える)。意図は変えていない(『core ファイルを作らない』。abort で終わること・シグナル 6 は同じ)。
2. **clang の object キャッシュ**(`harness/testfast/clangcache.c`、PATH の先頭の `clang`)。テストファイルは `Command::new("clang")` のまま。内容ハッシュ(clang の実体・フラグ・ソース・-MD の依存ヘッダ)で引き、リンクは毎回本物。失敗はキャッシュせず本物をやり直す。条件は §5.1。
3. **並列ランナー**(`harness/test.sh`、`harness/testfast/fletest.py`、標準ライブラリだけの Python): `cargo test --no-run --message-format=json` から binary を拾い、メモリ予算(cgroup の `memory.max` の 7 割。binary ごとの前回の RSS で見積もる)と CPU 数(cgroup の `cpu.max`)の範囲で、前回の壁時計の長い順に同時に走らせる。各 binary は `--nocapture` で走らせ、標準出力・標準エラーをログに取る(`SKIPPED` を数えるため)。binary 間で衝突する出力パスは無いことを実測(`profile.md` §3.4)。
4. **結果キャッシュ**(同ファイル): 『実行ファイルの SHA-256・環境・読んだ全ファイルの内容が前回の通った実行と同じ』なら走らせず『cached』と別に数える。入力集合は `strace -f` で binary ごとに自動に取る(手書きの対応表ではない)。条件・無効化は §5.2。
5. **`affected`**(同ファイル + `harness/testfast/deps.tsv`): `git diff` の変更ファイルと、strace で取った入力集合・cargo の依存から、影響する binary だけを選ぶ。選び方・限界は §5.3。`fast`(他のプログラムを起動せず 3 秒未満の 24 binary=130 テスト、約 2 秒)は『常時実行の高速層』で、コードが変わったときの `affected` に含まれる。
6. **ビルド設定**(`lift/Cargo.toml`): `[profile.dev] debug = 0`(これまでの全体テストが環境変数 `CARGO_PROFILE_DEV_DEBUG=0` で手で指定していたもの)、fe-pe・fe-decode・iced-x86 を opt-level 2、fe-lift を 1。
7. **入力の自動検出**(`harness/test-env.sh`): DLL・種を `binaries/`・`lift/out-*/` から探し、無ければ『MISSING input』と数を出す。`DSP_IPPV2_PE` は**意図して未設定**(§6)。
8. **支配していた 1 テストの並列化**(PM の許可。`tests/sse_legacy.rs` の `rsqrtps_all_2e32_inputs_match_host` と、そのテストだけが使う `tests/rsqrt/rsqrt_exhaustive.c`)。名前と検査内容(2^32 通りの float ビットパターン×6 つの MXCSR 値を、持ち上げたヘルパーとホストの RSQRTPS で全件照合し、不一致 0・状態旗の変化 0)は変えない。
   - C プログラムは環境変数 `RSQRT_FROM` / `RSQRT_TO`(4 の倍数、0 ≦ FROM ≦ TO ≦ 2^32)で範囲を絞れる。**未設定なら従来どおり 2^32 全件**(他の呼び出し方は無い)。
   - テスト本体は、2^32 を等長の 16 個の範囲に切り(`RSQRT_SLICES = 16`)、(MXCSR 値 6 × 範囲 16)= 96 回の実行を、`FLE_TEST_THREADS` 本(既定は利用可能コアの半分、最低 1)のスレッドで取り出しながら走らせる。
   - **照合する入力の集合が同一であることの証明**は、テストの中のアサート: ① 先頭の範囲が 0 から始まる ② 最後の範囲が 2^32 で終わる ③ 隣り合う範囲が接する(`end` = 次の `start`: 隙間も重なりも無い) ④ すべての境界が 4 の倍数(プログラムは 1 歩で 4 レーン) ⑤ 各実行が報告する入力数が `end - start` に等しい ⑥ MXCSR 値ごとの 16 個の合計が 4294967296 で、不一致 0・旗の変化 0 の行がちょうど 6 つ(従来のアサート)。①〜③ で和集合は [0, 2^32) の分割、⑤⑥ でプログラムが実際にその全部を見たことが言える。
   - **不一致が 1 件でもあれば失敗する**: 変異検査で、C の比較に『入力 x のとき不一致』を仕込んだ(x = 0、0x0fffffff=範囲 0 の最後、0x10000000=範囲 1 の最初、0xf0000000=最後の範囲の最初、0xffffffff=最後、0x7fffffff、0x80000000、0x12345678)8 件がすべて検出された。分割そのものの欠陥(範囲 5 が 4 パターン遅れて始まる=隙間、範囲 7 が 4 パターン遅れて終わる=重なり、最後の範囲を落とす、プログラムが各範囲の最後の 4 パターンを飛ばす)4 件もすべて検出された(§4.6)。
   - 壁時計(このテストだけを、負荷 9 前後で): 単独の直列 96.2〜96.6 s → `FLE_TEST_THREADS` 既定(コアの半分=8)16.6 s、4 本 26.1 s、8 本 15.0 s、12 本 12.5 s。
   - 他に 60 s を超える binary は、opt-level 後は無い(次点は fe-lift の `adjustor`・`vfs`・`k32_dsp`・`call_iat_dll` の 27〜38 s)。
   - ランナーの `libtest` スレッド数の環境変数は `FLE_TEST_LIBTEST_THREADS`(`--threads` の既定)に改名した(`FLE_TEST_THREADS` はテスト本体が使う)。

## 4. 受け入れ条件の実測(HEAD 2b3a2f2 + この差分)

### 4.1 `affected` の典型 5 通り(依頼 C1)

方法: `tools/accept.py`。変更は 1 行の追記(`/* … */` または `//`)。ワークツリーと、変更前の複製を `--base-tree` で比べる。**コールド** = テストのキャッシュ(clang の object・結果・記録・実行時間)を空にした `FLE_TEST_CACHE`(`deps.tsv` は git の中身なので有る。cargo の target は温かい)。**ウォーム** = 変更前の木に対し `full` を 1 回通したあとの既定のキャッシュ。cgroup 12 コア相当・6 GB。『load』は各測定の開始時の load average(**18〜24: 他の作業と直前の自分の測定を含む重い状態**)。

| 変更 | 選ばれた binary(うち高速層 24) | コールド s | ウォーム s | 実行 / cached(ウォーム) | load(コールド / ウォーム) | 結果 |
|---|---|---|---|---|---|---|
| `lift/runtime/fle_gdi.c` +1 行 | 64/138 | **27.6** | **18.8** | 40 / 24 | 19.7 / 22.4 | 950 passed・0 failed |
| `lift/runtime/fle_win.c` +1 行 | 64/138 | **25.0** | **27.3** | 40 / 24 | 24.0 / 23.6 | 885 passed・0 failed・SKIPPED 1 |
| `lift/crates/fe-lift/src/string.rs` +1 行 | 133/138(fe-lift の binary 全部 + 高速層ほか) | **88.6**(ビルド 19.8 s を含む) | **39.6**(ビルド 18.1 s) | 46 / 87 | 22.4 / 22.8 | 1,944 passed・0 failed・SKIPPED 1 |
| `lift/crates/fe-lift/tests/gdi.rs` +1 行 | 25/138(24 + `fe-lift/gdi`) | **24.5**(ビルド 17.3 s) | **8.6**(ビルド 1.9 s) | 1 / 24 | 20.3 / 18.4 | 232 passed・0 failed |
| `docs/README.md` +1 行 | **0**/138 | **2.1**(cargo のビルド確認だけ) | 0.3 | 0 / 0 | 18.4 / 18.4 | テストは走らない |

- fe-lift/src の『ウォーム』で 87 binary が cached なのは、1 行の追記が実行ファイルのバイト列を変えなかったため(コメントだけ)。実行ファイルの SHA-256 が同じなら結果を再利用する。
- すべて 180 秒以内。(cd8c6d9 の木では fe-lift/src のコールドが 131.9 s だった。§3.8 の分割で 88.6 s。)
- 追加の実測(他の作業の典型。cd8c6d9 の木): `imports_table.rs` の末尾に 1 行足し、`wall_fr14.rs` の複製を新しい `wall_fr20.rs` として足した木で、`affected` は 133/138 を選び(新しい binary は『crate の変更』と『記録なし』で選ばれ)、83 を実行・50 を cached、134.3 s(ビルド 18.6 s を含む。分割の前)、1,959 passed・0 failed(`logs/compat/compat-wall_fr20.txt`)。

### 4.2 `full`(依頼 C2)

`tools/final-measure2.sh`(コードを固定した最後の一連、HEAD 2b3a2f2 の木)。すべて **141 results・2,068 passed・0 failed・0 ignored・SKIPPED 行 1**(PM の基準 141 / 2,068 / 0 と同数)。12 コア制限 = `systemd-run --user --scope -p CPUQuota=1200% -p MemoryMax=6G -p MemorySwapMax=0`、16 コア = 制限なし(`-p MemoryMax=8G -p MemorySwapMax=0` のみ)。依頼者の要求は『この機械で』なので、同居する他の作業が無い場合の値は 16 コアの行が近い(ただし load 24 前後の下の値)。

| 状態 | コア | 壁時計 | CPU(user+sys) | run / cached(binary) | 開始時 load |
|---|---|---|---|---|---|
| F1 キャッシュ全部空、初回(strace で入力集合を記録しながら) | 12 | **105.0 s** | 1,114 s | 138 / 0 | 14.8 |
| F2 ウォーム(F1 の直後) | 12 | **0.5 s** | 0.6 s | 0 / 138(テスト 2,068 すべて cached) | 20.4 |
| F3 **キャッシュ全部無効**(`FLE_TEST_NOCACHE=1`、全 binary を本当に実行、clang も本物) | 12 | **83.7 s** | 957 s | 138 / 0 | 20.4 |
| F4 clang キャッシュ空・記録なし(`--no-record`) | 12 | 75.5 s | 873 s | 138 / 0 | 21.5 |
| G1 **キャッシュ全部無効**(`FLE_TEST_NOCACHE=1`) | **16** | **75.1 s** | 965 s | 138 / 0 | 23.9 |
| G2 clang キャッシュ空・記録なし | **16** | **61.4 s** | 799 s | 138 / 0 | 26.1 |

- 12 コアの行の壁時計 ≒ CPU ÷ 12(957 ÷ 12 = 80 s)で、**今の下限は CPU 総量**。一番長い binary は 29〜39 s(`sse_legacy` 31〜39 s、`adjustor`、`vfs`、`k32_dsp`、`call_iat_dll`)で、壁時計の方が長い。16 コアの行は load 24〜26 の下で 965 ÷ 16 = 60 s に対し 75 s だった(同居する他の作業が CPU を取っている)。load の無い状態での 60 s 台は**未測定**。
- 初回(F1)の strace の記録で壁時計は +21〜30 s(CPU は +160 s)。専用の `record`(全 binary を記録するだけ)は cd8c6d9 の木で 222 s かかった。
- 参考(cd8c6d9 の木、分割の前): キャッシュ全部無効 122.1 s(別の通しで 128.2 s)、空キャッシュ 118.6〜126.0 s。下限は 1 テスト 96 s だった(§2.1・§3.8)。

### 4.3 選択の健全性: 変異検査(依頼 C3)

方法: `tools/mutate.py`。ワークツリーの 1 ファイルに実際に挙動が変わる 1 箇所の変更を入れ、`affected --base-tree 無傷の複製` を走らせ、次に `full`(既定=結果キャッシュ有効。無効化が正しく働くことの検査も兼ねる)を走らせ、元に戻す。『検出』=失敗したテストが 1 件以上。偽陰性=`full` が検出して `affected` が検出しないこと。HEAD 2b3a2f2 の木で全件を取り直した(`logs/mutate2.jsonl`)。

| id | 変更 | ファイル | affected が選んだ binary(うち高速層) | affected の壁時計 s | affected の失敗(binary: テスト数) | full の失敗 | full の壁時計 s | 同じ失敗 |
|---|---|---|---|---|---|---|---|---|
| M01-runtime-header | GetDeviceCaps HORZSIZE 508 -> 509 mm | `runtime/fle_gdc.h` | 72/138 | 22.9 | fe-lift/gdc: 1; fe-lift/wall_fr7: 1 | fe-lift/gdc: 1; fe-lift/wall_fr7: 1 | 2.6 | はい |
| M02-runtime-sys | GetTickCount returns clock+1 | `runtime/fle_sys.c` | 72/138 | 1.9 | fe-lift/sys: 3; fe-lift/time: 1 | fe-lift/sys: 3; fe-lift/time: 1 | 1.9 | はい |
| M03-runtime-reg | RegQueryValueEx: ERROR_MORE_DATA returned as 235 | `runtime/fle_reg.c` | 61/138 | 28.5 | fe-lift/reg: 2 | fe-lift/reg: 2 | 16.0 | はい |
| M04-runtime-gdi | GDI invalid-parameter last error 87 -> 88 | `runtime/fle_gdi.c` | 64/138 | 18.5 | fe-lift/wall_fr11: 2 | fe-lift/wall_fr11: 2 | 33.5 | はい |
| M05-runtime-locale | code page 1252 -> 1251 | `runtime/fle_locale.c` | 65/138 | 19.4 | fe-lift/locale: 8 | fe-lift/locale: 8 | 5.6 | はい |
| M06-runtime-win | WH_MOUSE 7 -> 8 | `runtime/fle_win.c` | 64/138 | 19.5 | fe-lift/wall_fr11: 1 | fe-lift/wall_fr11: 1 | 7.8 | はい |
| M07-runtime-vol-header | max path component 255 -> 254 | `runtime/fle_vol.h` | 65/138 | 21.6 | fe-lift/vol: 1 | fe-lift/vol: 1 | 1.6 | はい |
| M08-lift-src-string | lifted STD clears DF instead of setting it | `crates/fe-lift/src/string.rs` | 133/138 | 71.9 | fe-lift/string: 1 | fe-lift/string: 1 | 73.6 | はい |
| M09-decode-src | adjustor thunk byte limit 16 -> 8 | `crates/fe-decode/src/adjustor.rs` | 138/138 | 68.2 | fe-decode/adjustor: 5; fe-decode/vcall_setjmp: 5; fe-lift/adjustor: 2; fe-lift/vcall_thunk_dll: 1 | fe-decode/adjustor: 5; fe-decode/vcall_setjmp: 5; fe-lift/adjustor: 2; fe-lift/vcall_thunk_dll: 1 | 74.8 | はい |
| M10-pe-src | rebase report counts one DIR64 slot too many | `crates/fe-pe/src/rebase.rs` | 138/138 | 69.9 | fe-pe/rebase: 4 | fe-pe/rebase: 4 | 75.8 | はい |
| M11-harness-c | g8 scenario program prints another message number | `harness/web/g8/g8_scenarios.c` | 25/138 | 0.5 | fe-lift/wall_fr19: 4 | fe-lift/wall_fr19: 4 | 20.4 | はい |
| M12-seed | registry seed value name misspelt | `harness/seeds/reg-seed-nsis.tsv` | 26/138 | 0.8 | fe-lift/wall_fr3: 1 | fe-lift/wall_fr3: 1 | 0.9 | はい |
| M13-test-file | edit of an expected value in a test file (the test must fail) | `crates/fe-lift/tests/vol.rs` | 25/138 | 1.8 | fe-lift/vol: 1 | fe-lift/vol: 1 | 5.5 | はい |
| M14-imports-table | GetTickCount row calls the GetTickCount64 helper | `crates/fe-lift/src/imports_table.rs` | 133/138 | 75.5 | fe-lift/sys: 2 | fe-lift/sys: 2 | 82.2 | はい |
| M15-new-runtime-file | a new file in runtime/ (picked up by the tests that glob the directory) | `runtime/fle_zz_mutation.c` | 128/138 | 72.6 | fe-lift/call_iat: 2; fe-lift/callback: 1; fe-lift/deferred: 12; fe-lift/hook_points: 10; fe-lift/k32_dsp: 11; fe-lift/load: 1; fe-lift/modules: 2; fe-lift/msctf: 10; fe-l … | fe-lift/call_iat: 2; fe-lift/callback: 1; fe-lift/deferred: 12; fe-lift/hook_points: 10; fe-lift/k32_dsp: 11; fe-lift/load: 1; fe-lift/modules: 2; fe-lift/msctf: 10; fe-l … | 31.7 | はい |

`FLE_TEST_NOCACHE=1` の full(結果キャッシュ無し・clang キャッシュ無し・全 binary を実行)でも確かめた 3 件:

| id | affected の失敗 | full(NOCACHE)の失敗 | full(NOCACHE)の壁時計 s | 実行した binary | 同じ失敗 |
|---|---|---|---|---|---|
| M01-runtime-header | fe-lift/gdc: 1; fe-lift/wall_fr7: 1 | fe-lift/gdc: 1; fe-lift/wall_fr7: 1 | 81.5 | 138/138 | はい |
| M12-seed | fe-lift/wall_fr3: 1 | fe-lift/wall_fr3: 1 | 84.3 | 138/138 | はい |

- **15 件すべてで `affected` と `full` が同じ失敗テストを検出した。偽陰性 0。** 失敗の数え方は『binary: 失敗したテストの数』(ランナーは 1 binary あたり 12 件まで表示するので、12 は『12 件以上』の意味。M15 だけが該当)。
- `FLE_TEST_NOCACHE=1` の `full`(全 138 binary を実行)でも確かめたのは M01・M12(第 2 表。cd8c6d9 の木では M01・M08・M12 の 3 件)。
- **cd8c6d9 の木の最初の版で検出されなかった 4 件**(定数がテストで固定されていなかった。`affected` と `full` が**両方**検出しなかったので選択の偽陰性ではない。別の変更に差し替えた): `fle_reg.c` の `KEY_NAME_MAX` 255 → 254、`fle_gdi.c` の `GDI_CLIP_MAX` 64 → 63、`fle_reg.c` の `CREATE_LEVELS_MAX` 32 → 31(コンソールにだけ残した)、`fe-decode/src/adjustor.rs` の `MAX_PRE` 3 → 2(`logs/baseline-cd8c6d9/mutate-first-pass-survivors.jsonl`、前 2 件は `mutate-batch1-first.jsonl`)。これらはテストの穴の候補。
- 新規ファイル(M15)は『ディレクトリ一覧』の依存で選ばれる: `runtime/` を一覧する binary は `affected` で高速層を含め 128 binary が選ばれ、リンクの重複定義で落ちた。`full` も同じ失敗。

### 4.4 `cargo test --workspace -- --list`(依頼 C4)

前: HEAD 2b3a2f2 の無変更の木(`clean2`)で `cargo test --workspace --offline -- --list` → SHA-256 `17766652e072be0d0306dcf283880fa203a3ba72747c6b75a83481c5f4160ce2`(2,343 行)。後: 変更後の木で同じ → **同一**(`logs/list-before-2b3a2f2.txt` / `list-after-2b3a2f2.txt`)。追加・削除・`#[ignore]` は 0。テストファイルの変更は §3.8 の 2 ファイルだけ(新しいテスト関数は足していない)。cd8c6d9 の木でも同一だった(`cee4315f…`、2,332 行)。

### 4.5 進行中の他の作業との両立(依頼 C5)

`tests/etxt_common` など共有ヘルパーの API、テストが出力する固定パス(`lift/out-*/test/…`)は変更していない。確認(cd8c6d9 の木): 新しいテストファイル(`wall_fr14.rs` の複製を `wall_fr20.rs`)を足し、`imports_table.rs` の末尾に 1 行追記した木で、(a) 素の `cargo test --test wall_fr20`(runner 経由)が 24 passed、(b) `affected` がその binary を選んで走らせた(`logs/compat/compat-wall_fr20.txt`)。2b3a2f2 で入った `wall_fr21.rs`(9 件)は、そのまま `full` で通る(138 binary に含まれる)。差分と重なる既存ファイルは `lift/Cargo.toml`(末尾に追記)と `tests/sse_legacy.rs`・`tests/rsqrt/rsqrt_exhaustive.c` の 3 つだけ。vclock1(決定 0147)がこれらを触るなら、MERGE-NOTES の取り方に従う。

### 4.6 分割したテストの変異検査(PM の条件)

`tools/rsqrt-mutate.sh`(`logs/rsqrt-mutations.txt`)。コンソールに残した 12 件のうち、仕込んだ不一致 8 件(x = 0x00000000、0x0fffffff、0x10000000、0xf0000000、0xffffffff、0x12345678、0x7fffffff、0x80000000)・分割の欠陥 3 件(隙間、重なり、最後の範囲の欠落)・プログラムが各範囲の最後の 4 パターンを飛ばす 1 件が、**12/12 検出**された。x = 0xffffffff の 1 件は、実際の不一致行(`mismatch mxcsr 0x1f80 x=0xffffffff host=… lifted=…`、6 つの MXCSR で 1 件ずつ、合計 `mismatches 1`)で失敗することを確認した(clang のコンパイル失敗による偽の検出ではない)。ログの中の `sed: -e expression #1 …` の 1 行は、スクリプトに残った不要な 1 行が出したエラー(何も変更していない)。


## 5. キャッシュの条件と無効化(『通った』と数えるとき)

三つの層がある。どれも**テストの中身は変えない**(テストファイルは 1 本も編集していない)。

### 5.1 clang の object キャッシュ(`harness/testfast/clangcache.c`、PATH の先頭の `clang`)

テストは `Command::new("clang")` で `-O0 … runtime/fle_*.c 60 本 + 生成した C + main.c -o exe` を毎回実行する。シムはこれを『1 本ずつ `clang -c` → リンク』に分け、object を内容で引く。

- **キー**: clang の実体(realpath・サイズ・mtime)、`-c` に渡す全フラグ(`-I` 等のパスは realpath に正規化)、ソースの SHA-256 と**ファイル名**(ELF の FILE 記号に入るのは名前だけ)。ヒットの条件は、前回の `-MD` が出した依存ヘッダ**すべて**の SHA-256 が同じこと(`/usr`・`/opt` のヘッダは mtime とサイズが同じなら同じとみなす)。ソースかヘッダに `__FILE__`・`__DATE__`・`__TIME__`・`__COUNTER__`、またはフラグに `-g`・`-finstrument-functions` がある、あるいは clang が警告を出した場合は、ソースの綴りと cwd が同じことも要る(object がパスに依存し得る)。
- **使わないもの**(本物の clang をそのまま実行): 知らないフラグ(`-E -P -x -S -fsyntax-only -shared -nostdlib …` や `-s*` など)、`.c` / `.o` 以外の入力、`-o` が無い、`-c` で複数ソース、24 MiB を超えるソース、`lift/out-*/` の下の**テストが自分で書いた**ソース(実行ごとに名前が変わるので保存しても二度と使われない。コンパイルしてリンクに使い、消す)。
- **失敗はキャッシュしない。** コンパイルが 1 本でも失敗したら、元のコマンドをそのまま本物の clang で実行し直す(診断・終了コードは本物)。警告は保存した標準エラーを再生する。`-Werror` で落ちるテストはいつでも本物の clang に当たる。
- **リンクは毎回本物**(`-Qunused-arguments` を足す)。実測 1 回約 30 ms。
- **無効化**: ソース・依存ヘッダ・フラグ・clang の実体のどれかが変われば別のキー(古い entry は残る)。`FLE_TEST_NOCACHE=1` で全部を本物にする。置き場は `$FLE_TEST_CACHE/cc/{b,m,l,t}` で、`test.sh gc` か各実行の最後に 1500 MB 以内に刈る(最近使った blob を残す)。同じキーの同時コンパイルは `flock` で 1 回にする。
- **保証しない点**: (a) `#include "x.h"` が、前回は無かった『先に探す場所』の同名ファイルに隠される場合は、ヘッダの内容が変わらないと気づかない(ccache と同じ。runtime の中では起きない。**テスト binary の結果キャッシュ側は**、探して無かったヘッダの置き場の一覧を記録するので、そこでは気づく)。(b) libc・ld・crt の更新は clang の実体が変わらないと気づかない(リンクは本物なので、リンク結果は新しい)。

### 5.2 テスト binary の結果キャッシュ(`harness/testfast/fletest.py`、`$FLE_TEST_CACHE/{results,records}`)

『ある binary が前回通ったので、今回は走らせない』ための記録。**走らせなかった binary は『cached』と数え、実行した件数と別に表示する**(`executables: run=… cached=…`、`tests: run=… cached=…`)。

- **入力集合**は `strace -f` の実測(手書きの対応表ではない): その binary が開いて読んだファイル(`stat`・`access` で存在を確かめたものを含む)、ディレクトリの一覧(`getdents64`、および存在しなかった `#include "…h"` の探し先)、存在しなかったパス。除くもの: その実行が最初に書いた・作った・消したパス(出力)、`/usr` `/etc` `/proc` `/tmp` など、cargo の target。記録は binary の SHA-256 ごと。
- **ヒットの条件(全部)**: ① 実行ファイルの SHA-256 が同じ(コメントだけの編集のように、再ビルドしても同じバイトなら同じ) ② 環境が同じ(`FLE_* FLENGINE_* ILTOOLS_* BLEND2D_* DSP_* FREETYPE_* FL64_* X87_* MXCSR_* RUST_TEST* RUST_BACKTRACE` の値、`clang python3 node nm` の実体のサイズと mtime) ③ 読んだ全ファイルの SHA-256、一覧したディレクトリのファイル名一覧、存在しなかったパスの不在が、前回の通った実行の**開始前**の値と同じ ④ 前回が rc=0・failed=0。DLL のような 1 MiB を超えるファイルは (サイズ, mtime, inode) で覚えて再計算を省く(同じプロセスの中だけ)。
- **無効化**: 上のどれかが違えば走らせる。通らなかった実行は保存しない。`--fresh` と `FLE_TEST_NOCACHE=1` は使わない。記録が無い binary は走らせる(初回の `full` は strace で記録し、`--no-record` で省ける)。
- **保証しない点**: テストが時刻・乱数・ネットワーク・env で読む値(`FLE_*` 以外の変数、現在時刻)に依存する場合は、その変化に気づかない(strace で見えるのはファイルだけ)。テストの読み集合が実行ごとに変わる場合(非決定的)は、記録した 1 回の集合が正しいとして扱う。`binaries/` や `lift/out-*/` の git 管理外の入力(DLL・種)の変更は、この層は**内容のハッシュで気づく**(`affected` の選択は git の差分しか見ないので気づかない: §5.3)。

### 5.3 `affected` の選び方(`harness/testfast/deps.tsv`、git に入れる生成ファイル)

- **変更の取り方**: `git diff --name-status HEAD`(`--base REV` で変える)+ 未追跡で ignore されていないファイル。git の木でなければ `--base-tree` / `--files`。`deps.tsv` 自身の変更は数えない。
- **選ぶ規則**(どれか): (1) 変更ファイルが `lift/crates/<c>/` の `src`・`Cargo.toml`・`build.rs` なら、c を(path 依存を含めて)ビルドに含む binary 全部(`cargo metadata` から。fe-pe を変えれば全 binary、fe-lift/src なら fe-lift の 113)。`lift/Cargo.toml`・`Cargo.lock`・`.cargo/` なら全部。(2) 変更ファイルが、ある test crate の rustc 依存情報(`.d`)にあるか、そのクレートの `tests/NAME.rs` なら、その binary(`tests/etxt_common/mod.rs` のような共有 mod なら、それを `mod` で取り込む全 binary)。(3) 変更ファイルを**読んだ・存在を確かめた**と `deps.tsv` の `R`/`A` 行にある binary。(4) 追加・削除されたファイルがあるディレクトリを**一覧した**(`D` 行。`#include` の探し先を含む)binary。(5) `deps.tsv` に行が無い binary(新しいテスト)。(6) コード(`docs/` `worklog/` `*.md` `app/` `tools/` 以外)が 1 つでも変わったとき、高速層(`tier=fast`)。
- **『未知のファイルが変わったら全部』**は、(5) と、`deps.tsv` が無いときの全選択、(1) の `Cargo.*` だけ。それ以外の『誰も読まないファイル』の変更は 0 件を選ぶ(その場合、`docs/` のみの変更は 0 binary になる。**docs を読むテストがあれば、記録に入っていて選ばれる**: 実測では `deps.tsv` に `docs/` の行は無い。`tools/authenticode_check.py` だけは `fe-lift/authenticode_check` が読むので、その変更はその binary を選ぶ)。
- **選ばれない場合がある条件(限界)**: ① git 管理外の入力(DLL・種)の変更 ② テストが env や時刻で読む値 ③ 記録した後で、あるテストが新しいファイルを読むように変わった場合(そのテストのソースが変わったので (2) で選ばれ、次の `record` で直る) ④ `deps.tsv` が古い(`record` を忘れた)場合: 新しい読み先は (2) の経路でしか選ばれない。

## 6. 使い方

`lift/` で(どこからでも可。スクリプトは自分の場所から木を見つける。パッチを当てた直後は実行ビットが無いので `chmod +x harness/test.sh` か `bash harness/test.sh`。cargo の runner は `bash` で起動するので実行ビット不要):

```
harness/test.sh affected            # 変更(git diff HEAD + 未追跡)を読む binary だけ。コード変更があれば高速層も
harness/test.sh affected --base HEAD~3          # 基準を変える
harness/test.sh fast                # 高速層(他のプログラムを起動せず 3 秒未満の 24 binary・130 テスト、約 2 秒)
harness/test.sh full                # 全 binary + doc-test。結果キャッシュが有効な binary は『cached』と別に数えて表示
FLE_TEST_NOCACHE=1 harness/test.sh full        # 全部を本当に実行(clang キャッシュも結果キャッシュも使わない)= 最終ゲート
harness/test.sh full --no-record    # 初回に strace で入力集合を取らない(12 コアで実測 105.0 → 75.5 s。結果キャッシュは貯まらない)
harness/test.sh record              # 全 binary を strace で走らせ入力集合を取り直し、harness/testfast/deps.tsv を作り直す(約 3.7 分=実測 222 s)
harness/test.sh record --collate-only          # 手元の記録から deps.tsv だけ作り直す
harness/test.sh names               # 全テスト名の一覧と SHA-256
harness/test.sh gc                  # clang の object 置き場を FLE_TEST_CACHE_MAX_MB(既定 1500)以内に刈る(各実行の最後にも自動)
harness/test-env.sh                 # 入力(DLL・種)の自動検出の結果を見る。--link で git 管理外の入力への symlink を作る(FLE_INPUT_ROOT=別の checkout)
```

- 選び方の指定: `--only REGEX`(binary 名 `fe-lift/modules` など、繰り返し可)、`--jobs N`(同時に走らせる binary 数、既定は使えるコア数の 2/3)、`--threads N`(各 binary のテストスレッド数、既定 4)、`--fresh`(結果キャッシュを使わない)、`-- 引数`(各 binary にそのまま渡す。テスト名の絞り込みなど)。
- git の木でないとき(アーカイブの展開など): `--base-tree 無傷の複製` または `--files 変更ファイル一覧`(行頭 `A:` / `D:` は追加・削除)。
- メモリ: cgroup の `memory.max`(無ければ MemAvailable)の 7 割(6 割)を予算にし、binary ごとの最大 RSS(前回の実測、無ければ `deps.tsv` の桁)を足して超えるなら待つ。CPU 数は cgroup の `cpu.max` を見る。依頼どおり 12 コア相当に抑えるには `systemd-run --user --scope -p CPUQuota=1200% -p MemoryMax=6G -p MemorySwapMax=0 harness/test.sh full`。
- 素の `cargo test --workspace --no-fail-fast --offline`(`lift/` で)も、`lift/.cargo/config.toml` の `runner` のおかげで同じ core 制限・clang キャッシュを使う(並列化と選択だけが無い)。`cargo run` で動かす道具には何も足さない(runner は `target/*/deps/` の実行ファイルにだけ作用)。
- 環境変数: `FLE_TEST_NOCACHE=1`(上記)、`FLE_TEST_CACHE=DIR`(既定 `$CARGO_TARGET_DIR/fle-test-cache`)、`FLE_INPUT_ROOT`、`FLE_TEST_JOBS` / `FLE_TEST_LIBTEST_THREADS`(ランナーが各 binary に渡す `--test-threads` の既定)、`FLE_TEST_THREADS`(テストの中で並列に走るテスト=`rsqrtps_all_2e32_inputs_match_host` が同時に走らせるプロセス数。既定は利用可能コアの半分)、`FLE_TEST_CACHE_MAX_MB`、`FLE_CC_STATS`(clang シムの判断を TSV で追記)。
- 出力: 実行ごとの標準出力・標準エラーは `$FLE_TEST_CACHE/logs/<日時>-<モード>/`(binary ごとの `.out` / `.err`、`cc-stats.tsv`、`doctests.out`)。標準エラーは `--nocapture` で取るので、`SKIPPED` 行が数えられる。
- 終了コード: 失敗が 1 つでもあれば 1。

**SKIP の見え方。** 末尾の要約に `SKIPPED-lines=N` と、その binary 名と最初の 1 行を出す。見つからない入力があれば `MISSING INPUT FILES`(`harness/test-env.sh` が数える)。静かに戻るだけで `SKIPPED` を出さないテスト(`FL64_EXE` を使う `authenticode_check` の 14 本)は数えられない: その入力が `test-env.sh` で見つかっていることだけが保証。

**`DSP_IPPV2_PE` は意図して未設定にする。** 同じ変数を、非再基底版(`out-ext/dsp_ippv2_x64.dll`、image base 0x180000000)を要求する `fe-pe/tests/rebase.rs`(2 本)・`fe-decode/tests/jt_cpu.rs`(1 本)と、再基底版(0x2000000)を要求する `fe-decode/tests/msvc_switch.rs`(2 本)が読む。再基底版を渡すと前者の 3 本が落ちる(実測、§4.5)、非再基底版を渡すと後者の 2 本は `SKIPPED: the image base is 0x180000000` で何も確かめずに通る(verify4 が見つけたとおり)。未設定なら、各テストの既定パスが正しい版になる(後者は `out-jtb2d90/three/dll/dsp_ippv2_x64.2000000.dll`)。この `harness/test-env.sh` の形で、dsp の 2 本は実際に走る(SKIPPED 行 0)。

## 7. 限界と未確認

- **測定はすべて共有マシンの上**(開始時の load average 5〜26)。静かな時間帯の再測定はしていない。各表に開始時の load を付けた。同じ構成の再測定のばらつき(cd8c6d9 の木): `full` NOCACHE 122.1 / 128.2 s。HEAD 2b3a2f2 の木の一連(`logs/final2`)は、最初の一連が自分の環境の不備(後述 §9)で無効になり、取り直した 2 回目の値。
- `full` の**下限は CPU 総量**になった(約 950 s ÷ コア数)。load の無い状態での 60 s 台は未測定。16 コアの行(61〜75 s)は load 24〜26 の下の値。
- `affected` で fe-lift/src・fe-decode・fe-pe が変わる場合は 133〜138 binary が影響を受け、いちばん長い 89 s(コールド、ビルド 20 s 込み)。180 秒に対し余裕は約 90 秒。
- 結果キャッシュ・`affected` は **strace の記録に依存**する。`strace` が無い環境では記録を取らず(走らせて通すだけ)、結果は貯まらず、`deps.tsv` が無ければ `affected` は全部を選ぶ。記録は 1 回の実行の読み集合(非決定的に読み先が変わるテストがあれば取りこぼす)。
- **見えないもの**: テストが環境変数(`FLE_*` 等の接頭辞以外)・時刻・乱数・ネットワークで読む値。git 管理外の入力の変更は `affected` では選ばれない(`full` の結果キャッシュは内容のハッシュで見る)。
- 変異検査の 4 件は、cd8c6d9 の木の最初の版で `affected` も `full` も検出しなかった(テストの穴)。『その定数をテストが固定していない』事実は残る。
- `FLE_FR9T_CLEAN_RT`(`wall_fr9t` の『pre-0124 + 0123 の fle_thread.c との object 比較』)は自動検出できない(その版のファイルを git の履歴から再構成する必要がある)。**SKIPPED 1 件のまま**。`FL64_EXE` を使う `authenticode_check` 14 本は、入力が見つかっていることを `test-env.sh` が保証するだけで、静かに戻る枝は数えられない。
- `wall_fr21.rs`(決定 0148)は `lift/out-vis1/inputs/fonts` の名前を文字列として検査するが、そのディレクトリを読まずに通った(読む必要は無かった)。`out-vis1` には触れていない。
- `harness/test.sh record` は約 3.7 分(strace、cd8c6d9 の木で 222 s)。`deps.tsv` は生成物で、他の決定がテストを足すたびに `record` で作り直すか、その binary が『記録なし』で常に選ばれる(正しいが遅い)。
- clang キャッシュの object は、clang の実体・フラグ・ソース・ヘッダの内容が同じ間は再利用される(§5.1)。libc・ld の更新、`-I` の先頭に後から同名のヘッダが現れる場合は clang の実体が変わらないと気づかない(リンクは毎回本物)。
- メモリ: 6 GB の cgroup で OOM は出なかった(最大の binary RSS 1.5 GB、同時に 8 binary まで)。16 コアの測定は 8 GB の cgroup で、同じく OOM なし。
- `affected --quick`(選ばれた binary のうち 30 s 以内だけ)は作っていない(PM: 無理に作らなくてよい)。高速層だけなら `harness/test.sh fast`(24 binary・130 テスト・約 2 秒)。
- wasm・ブラウザ・実機 Windows は対象外(今回の対象はホストの `cargo test` だけ)。app/ の TypeScript のテストは対象外。
- 未実施: 静かな時間帯の再測定。`test.sh` を実リポジトリ(`/home/ubuntu/work/signature-studio-develop` 本体)の木で走らせること(作業は archive の複製だけ。git の差分取得と `git archive HEAD` への `patch --dry-run` だけ、本体の木で読み取りで確かめた)。パッチを当てたあとの実リポジトリでの `record`。

## 8. PM の判断点

1. **`lift/.cargo/config.toml` の `runner`**: 素の `cargo test` にも効く(RLIMIT_CORE=1・LD_PRELOAD・clang キャッシュ)。他のレーンの通しも自動で速くなり、同時に core-dump 待ちの行列を詰まらせなくなる。反面、全員のテストが**自作のシム**(clang の前置)を通る。外すにはそのファイルを消す。キャッシュを使いたくない人は `FLE_TEST_NOCACHE=1`(core 制限は残る)。
2. **`lift/Cargo.toml` の opt-level と `debug = 0`**: **PM が採用を了承済み**(fe-pe・fe-decode・iced-x86 を 2、fe-lift を 1、overflow-checks・debug-assertions は dev のまま)。代償は fe-lift/src 編集後のビルド 5.7 → 20 s(コールド 9 → 29 s)。**fe-lift を 2 にするとさらに延びて約 60 s**(テスト: コールド 60.4 s、fe-lift/adjustor など速さは 1 と同じ以下)。debug=0 は Rust のバックトレースの行番号が出なくなる(`CARGO_PROFILE_DEV_DEBUG=2` で戻せる)。
3. **`sse_legacy` の 1 テストの分割**: **PM の許可を得て実施済み**(§3.8)。確認してほしい点は、`tests/sse_legacy.rs` の 1 テストの本体と `tests/rsqrt/rsqrt_exhaustive.c` の変更が MERGE-NOTES の取り方で当たること、`FLE_TEST_THREADS` の既定(コアの半分)が他の作業との同居で多すぎないか。
4. **`deps.tsv` を git に入れるか**: 生成ファイルで約 500 行・約 0.1 MB。他の決定がテストを足すたびに行が増え、マージで衝突し得る。入れない場合は各自 `record` を 1 回(約 3.7 分)。入れた場合も、新しいテストは `record` までは常に選ばれる(正しいが遅い)。
5. **apport そのものを止めるか**: 根本の原因はこのマシンの `core_pattern` が apport にパイプしていること(root の設定。`sysctl kernel.core_pattern` / `systemctl disable apport`)。今回は触っていない(他の利用者に影響する)。止めれば、テスト以外の道具(`a4-fruity.sh` などが走らせるプログラムの abort)も速くなる。止めなくても、テストは runner 経由なら影響されない。
6. **カーネルログ**: RLIMIT_CORE=1 は crash ごとに 1 行をカーネルログに出す(全体テスト 1 回で約 2,500 行、`journalctl -k`)。害は見つけていない。
7. **`FLE_FR9T_CLEAN_RT` の SKIPPED 1 件**を、決定 0126 の clean 木の再構成で埋めるか。
8. **変異検査で見つかったテストの穴**(4 件、§4.3)を埋めるか。

## 9. 作業者の自己申告(規則との関係)

- **作業ディレクトリの容量**: `lift/out-testspeed/` が一時 **51 GB**(上限 15 GiB)になった(cd8c6d9 の測定中)。原因はテストが `lift/out-*/test` に残す生成物(`dev` の木だけで 36 GB)。気づいてすぐ `out-*/test` と不要な target を削除し 11 GB → 3.2 GB にした。以後は各測定のあとに削除。2b3a2f2 の測定中の最大は約 2.5 GB。終了時は 500 MB 以下に片付ける。
- **他の作業の木**: 最初に `lift/out-*` の一覧と大きさ(`du -sh`)を取った。その中には `out-vclock1`(11 GB)と `out-vis1`(24 GB)が入っていて、**中身は読んでいない**が、名前と大きさは見た。`/var/log/apport.log`(システムのログ、adm グループで読める)から全体の起動件数と、vclock1 / vis1 / 自分の木の件数を数えた(パスの文字列を集計。内容は読んでいない)。
- 存在しない `bash` ツールを 1 回呼んだ(エラーで返った)。
- 最初の基準の通し(cd8c6d9)は、ログの出力先を相対パスにしてしまい clang の記録が取れず、約 3 分で自分で止めて(`pkill -f` で自分のプロセスだけ)取り直した。
- **自分の環境の不備で無効になった測定**: 2b3a2f2 の最初の最終一連は、`TMPDIR` が指す作業ディレクトリの `tmp` を片付けで消していたため、clang と rustdoc が一時ファイルを作れず(`unable to make temporary file`)、NOCACHE の 2 本で 28 テストが失敗、doc-test が失敗した。`tmp` を作り直して一連を取り直した。報告する値は取り直した 2 回目だけ。
- **サーバの再起動で測定が 2 回中断された**。変異検査の途中の中断で、2 回、ワークツリーのファイルが変異した状態で残った(`fle_sys.c` の M02、`fe-pe/src/rebase.rs` の M10)。どちらも中断のあとの最初の確認(`diff -rq 無傷の複製 ワークツリー`)で見つけ、無傷の複製から戻して差が 0 になったことを確認してから続きを走らせた。中断した 2 件(M02、M10)は最初から取り直した。結果キャッシュに変異した状態の結果が残るが、キャッシュのキーに入力のハッシュがあるので戻したあとは使われない。
- 背景実行(`background: true`)を、基準の通し・`fix1`・`fix2` の 3 本に使った(cd8c6d9 の測定。長い測定を並べるため)。完了通知を待って終了することはしていない。他のプロセスの終了を待つ `pgrep` / `tail --pid` / while ループは使っていない(自分の測定の様子を `ps` で見ただけ)。`sleep`・`timeout`(コマンド)・`nohup`・末尾の `&` は使っていない。
- 途中報告: 1 回目は期限(開始 + 40 分=21:49)に 1 分遅れ(21:50)、2 回目は 30 分ごとの予定(22:20)に対し 22:50、3 回目(最終)は 00:15。新しい指示(HEAD 2b3a2f2 への作り直し、分割の許可、01:30 目安)への返信は 00:17 に出した。PM の 24:30 目安は、指示が届いたとき(00:15)にはすでに過ぎていた。
- `fix2` の測定中に `src` の木の `out-*/test` を削除した(自分の作業ディレクトリの中、容量対策)。そのとき動いていた binary に失敗は出ていない。
- 動かした `rm` はすべて `lift/out-testspeed/` の中。git の変更系操作は 0。`/tmp` は `/tmp/opencode/testspeed/`(約 15 MB)だけ。`python3` は `PYTHONDONTWRITEBYTECODE=1`。
- 本体のリポジトリの木では `git archive`・`git diff --name-status`・`git ls-files -o`・`git log`(読み取り)と、`ls` / `du` / `cat` だけを実行。テストは archive の複製で走らせた。
