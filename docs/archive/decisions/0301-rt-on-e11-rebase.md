# 0301(仮): 実スレッド版の足場(rt-on-e9.diff)を E11(main 7373cd55)に載せ直す rebase — 機械的な 5 hunk と、意味の衝突 1(`fle_landing_top` の置き場)の解き方(mt3)

状態: **仮**(第 1 報。使い捨ての木での作業で、製品の木には出さない)。作業者 mt3、2026-11-06 JST。依頼: PM(0301、go/no-go #2 の準備)。入力: gpu1 の `lift/out-git/gpu7-results/0289/rt-on-e9.diff`(59 ファイル、E9 基準)と同 §8・§11。基準: main 7373cd55(E11)。区分: **実測**=この作業の出力を見た値、**読み**、**未確認**。

## 0. 冒頭の三点(正直さ)

1. **作った物**: `rt-on-e11.diff`(`impl/`。60 ファイル = rt-on-e9.diff の 59 + 試験 1 本、約 4,270 行)。**`git archive 7373cd55` の木にそのまま当たる(`patch --batch -p1 --dry-run` rc=0、offset も fuzz も無し。`tables/dry-run-on-7373cd55.log`)**。rt-on-e9.diff を 7373cd55 に当てると、55 ファイルはそのまま、`lower.rs` は offset のみ、3 ファイル 5 hunk が不成立(gpu1 の 0289 §11.1 の dry-run と同じ結果を再現)。不成立の 5 hunk は機械的に手で直し、意味の衝突 1 は設計して直した(§1〜§2)。
2. **実エンジンの実行は無い**: この作業はコードの載せ直しと試験(fe-wasmgen の node 試験、ci/test-hybrid、test.sh full、a4 の生成・build 段階)だけで、実エンジンの音・画面の結果は 0 件。冷間の CI ステージビルド(約 20 分)と実ブラウザの確認は gpu1(bundle を作って測る側)の領分で、**未実施・未確認**。
3. **変更表(『No-options の唯一の構成』にするための変更)は gpu1 の 0289 §8 の表を使う**(PM の指示)。この文書は rebase の差分と衝突の解き方だけを扱い、旗の除去(`GIL=1` を既定に等)は **やっていない**。

## 1. rt-on-e9.diff を 7373cd55 に当てた結果と、5 hunk の手当て(実測)

| ファイル | 不成立 | 原因 | 手当て(`impl/rt-on-e11.diff` の該当箇所) |
|---|---|---|---|
| `fe-wasmgen/src/lib.rs` | 3 hunk(Default、`dry` の `Ctx`、`cx` の `Ctx`) | E11(0285)が `Config.hybrid_dispatch` と `Ctx.ti_helper` を足し、リテラル 3 行が変わった | `Config` の Default に `hybrid_mx_tls: false`、2 つの `Ctx` に `mx_tls: conf.hybrid_mx_tls`(各 1 語の挿入) |
| `fe-wasmgen/src/main.rs` | 1 hunk(usage 行) | E11 が usage 行に `[--hybrid-dispatch LIMIT LANDING HOOK]` を足した | usage 行に `[--hybrid-mx-tls]` を足す(引数の解析の hunk は成立) |
| `app/scripts/build-engine-bundle.sh` | 1 hunk(#5) | 0285b: `hyb_guard cpart` が 7 番地を検査する。E11 の行は `LIM`/`LAND`/`HOOK` を含む | `MXTLS` の宣言と `GIL=1` の枝(MXA・BUFA に加えて **LAND**)を `echo "addresses:"` の前に入れ、`fle_ctx` の size の正規表現を `(300|540)` に(rt-on-e9 の hunk と同じ)、`hyb-gen` に `$MXTLS`。`hyb_guard cpart` は §2 |

## 2. 意味の衝突 1(patch には出ない): `fle_landing_top` は TLS_MORE ではデータの symbol でなくなる — 解き方(設計 1 ページ)

**事実(コードの読み)**:
- E11(0285)の型付き間接呼びの共有ヘルパー(`fe-wasmgen/src/lib.rs` の `ti_helper`)は、`landing`(`fle_landing_top`、32 bit の pointer)と `hook`(`fle_dispatch_hook`)が 0 でないとき C の dispatch に回す。その 2 語を **絶対番地**(`i32.const 0; i32.load <addr>`)で読む。番地は `build-engine-bundle.sh` が link map の `fle_landing_top` の行から取り、`--hybrid-dispatch $LIM $LAND $HOOK` で埋め込む。
- 実スレッド版(`-DFLE_TLS_MORE`)では `fle_ctx_more.h:14` が `#define fle_landing_top (*(fle_landing **)(fle_ctx + FLE_CTX_LANDTOP))`(0x4D0)にする。**thread-local の `fle_ctx` の欄になり、link map に `fle_landing_top` のデータの行が無い** → `LAND` が空で 'an address is missing' で止まる。仮に番地を与えても、読むのが共有メモリの 1 語になり、**Worker ごとの landing とは別物**(誤り)。
- MXCSR とスクラッチ buffer は、足場の `--hybrid-mx-tls` で同じ問題を解いてある(アドレスは register page 空間の `RB + 欄の offset`、直 wasm は `__tls_base` 経由 `reg_base` で読む)。

**解き方(新しい旗は足さない)**: `--hybrid-mx-tls` の意味を『MXCSR・スクラッチ buffer・landing の head が thread-local の欄』に広げる。
1. `fe-wasmgen`(`ti_helper`): `landing` の読みだけ `reg_base(landing)` + `i32.load`(`__tls_base` + `landing − hybrid_r + tls_ctx`)にする(`hybrid_mx_tls` のとき)。`hook`・`fle_progress_limit`(u64)・進捗の count は大域のまま(下)。
2. `build-engine-bundle.sh`: `GIL=1` のとき `LAND=$(printf '%x' $((RB + 0x4D0)))`(`FLE_CTX_LANDTOP`)。
3. `hyb_guard cpart`: MXCSR・buffer・landing は C の部分に絶対番地が無いので `0x0`(rt-on-e9 の `MXA`/`BUFA` と同じ扱い)。**呼び出しは 1 か所のまま**(`GMX`/`GBUF`/`GLAND` を先に決める)。if/else で 2 か所に分けると `ci/test-hybrid.sh` の『stage_hybrid の `hyb_guard` の数 = 4』が落ちる(実測: 5 になって FAIL)ので、こうした。(**gpu1 へ**: rt-on-e9.diff の if/else は同じ理由で `ci/test-hybrid.sh` を落とすはず。)
4. 試験(`typed_indirect.rs` に 1 本と、既存の試験を TLS 版でも走らせるための引数の追加。+63 / −11 行。`tables/typed_indirect-test.diff`): `in_the_thread_local_build_the_typed_call_reads_the_landing_through_tls_base`。TLS 版を `__tls_base = 0x30000`(block は 0x30040)でコンパイルし、(a) landing が自分の block の中に立つ → dispatch(`dispatches` = 1)、(b) 絶対番地 0x24D0 に landing の語が立っていても型付きのまま(`dispatches` = 0。ヘルパーが絶対番地を読んでいない証明)、(c) census hook(大域)が立つ → dispatch、(d) 型付き・dispatch・stray の 3 通りで rax・callee-saved・X[6..15]・flag・count・slice・戻り番地が同一。**対照実験**: ヘルパーを元の絶対番地の読みに戻すと (a) の case が落ちる(`dispatches` 0 ≠ 1)ことを確認した。

**E11 のアダプタ(型付き呼びの表の各 entry)が TLS 相対になっているか**: gpu1 の 0289 §11 の指摘(アダプタは `c.reg_base(…)` で register page に触る)を、同じ試験で確かめた。試験は `__tls_base = 0x30000` で register file が 0x30040 にあり、絶対番地 0x2000 には何も無い。アダプタが絶対番地で R を読み書きしていれば rax・callee-saved・戻り番地が合わないが、型付き・dispatch・stray の 3 通りで全て一致して通った(**実測**。アダプタ 2 本〔T1、T2〕の範囲)。

**変えないもの(読み、確認が要る)**: `fle_progress_limit`(共有の u64)と `fle_dispatch_hook` は TLS_MORE でも大域のまま(足場の `--hybrid-progress` が大域の `fle_progress_count` を使うのと同じ)。型付き呼びが読む `limit`・`count` は E9 まで直 wasm が読まなかった値で、2 つの Worker が同時に読み・加算する。count の `+1` は直 wasm の既存の非原子的な加算(足場のまま)で、**読み・書きの競合は heuristic の範囲**(slice の判定が数回ずれる程度)— gpu1 の 0289 §11 の指摘どおり。これを厳密にするには count を TLS にするか atomic にする必要があり、この rebase では**やっていない**(判断は PM)。

**0285 の追加 C(`fle_hyb_dispatch` の変更)と GIL の包み**: `gen_wrap.py` が `fle_imp_*` を拾う方式は E11 で新しい import が増えていない(`fle_hyb_dispatch` は既に包みの対象)。今回の差分は import を足さない。**読みで、この木で `check_i1.py` はまだ走らせていない**(下の未実施)。

## 3. 検証(使い捨ての木 = 7373cd55 + rt-on-e11.diff)

| 検証 | 結果(実測) |
|---|---|
| 適用 | `git archive 7373cd55` に `patch --batch -p1 --dry-run` rc=0、offset も fuzz も無し |
| `cargo test --no-run` | rc=0(214 test executables) |
| fe-wasmgen の試験(`cargo test -p fe-wasmgen`) | 33 binaries、73 passed、0 failed、1 ignored。新しい試験とヘルパーの既存 4 本(typed_indirect 5 本)、`mx_hybrid_tls` が通る |
| `ci/test-hybrid.sh` | all checks hold(FAIL 0。上の `hyb_guard` の数を直す前は FAIL 1) |
| `test.sh full`(`FLE_TEST_NOCACHE=1 --jobs 2`) | 222 results(基準 218 + 足場の新しい 4 binaries〔gil_lockfree、semaphore、x87_threads、mx_hybrid_tls〕、これらは通る)、2,405 passed、**16 failed**(13 binaries)。**対照実験: E9(38efb398)に rt-on-e9.diff を当てた木〔offset 無しでクリーンに適用〕で同じ 13 binaries・同じ 16 試験が同じ形で失敗する**(§3b)= rebase が原因ではなく、足場(rt-on-e9.diff)自身が試験の期待を壊している。`tables/testfull-on-rt-on-e11.log`、`tables/e9-control-failing-tests.txt` |
| a4(生成・build 段階) | units 942/942、runnable 895/895、distinct 5,310、entries 1,009,737、終了コード 0(**基準の 7373cd55 と同一**。実スレッドの変更は `-DFLE_TLS_MORE` / `--hybrid-mx-tls` の下だけで、旗なしの生成物は変わらない。`tables/a4-on-rt-on-e11.log`) |
| 冷間の CI ステージビルド(約 20 分、`GIL=1`) | **未実施**。PM が gpu1 に依頼済み(0297 の Chrome の後、16:35 ごろから、nice 19)。gpu1 の手順は 0289 の `tools/mk-rt.sh`(20 分)→ `patchglue-rt.py` |
| `check_i1.py`(包みの検査) | 未実施 |

## 3b. test.sh full の失敗 16 本の内訳(足場の変更が原因。rebase の原因ではない)

E9 + rt-on-e9.diff の対照木でも同一(`tables/e9-control-failing-tests.txt`)。原因は失敗メッセージから(**読み**。足場の変更箇所の特定まではしていない):

| 原因 | 試験(binary: 数) |
|---|---|
| 足場が import 表に 2 行足した(`CreateSemaphoreW`、`ReleaseSemaphore`。`imports_table.rs`)ので、『表の行数・最後の行の位置・擬似 VA が動かない』を固定した試験が 2 行ずれる(例: wall_fr10 の 546 ≠ 544、wall_fr9 の 544 ≠ 542、GetModuleHandleA の擬似 VA が 32 ずれる) | wall_fr9、wall_fr10、wall_fr11、wall_fr12、wall_fr13、wall_fr16、wall_fr15(dsound の行)、deferred、wall_c1(HeapReAlloc の位置 259 ≠ 257)、srw(GetModuleHandleA)= 10 試験 |
| 足場の新しいコードの源の検査: `srw`『新しいコードは guest memory も host も読まない』が `getenv(` を見つけた | srw: 1 |
| `wall_fr15` の『hook が決定の言う所にある』が `fle_com_try_slot` の文字列の近傍(400 文字)を探して外れた(足場がその近傍を編集) | wall_fr15: 1 |
| `callback`『fle_callback.c は plain C』が `long` を見つけた | callback: 1 |
| `gpu7_ring_poll`『hook は wasm ビルドまたは試験の define のときだけ』が『使用は guard の中』で外れた | gpu7_ring_poll: 1 |
| `wall_wrp1`: 試験の単体 link が `fle_imp_InitializeCriticalSection` / `Enter…` / `Leave…` を解決できない(足場が CS を別の物に置き換えた) | wall_wrp1: 2 |
| 合計 | **16** |

これらは gpu1 の 0289 §8『試験: 8〜9 日』の中身に当たる(期待値の更新か足場の修正)。**この rebase ではやっていない**(PM の依頼は載せ直しと検証の報告。期待値を直す前に、どれが足場の誤りでどれが期待値の更新かの判断が要る)。

## 4. 方針との適合(必須の記載)

- **lift の忠実性**: 新しい処理は (b) 外部 API ではなく**実スレッド版のコード生成の整合**(TLS_MORE で欄になった `fle_landing_top` を、thread-local として読む)。FL の挙動の補正・代行・class/アドレスの分岐は無い。`GIL=1` の環境変数は rt-on-e9.diff の足場のまま(旗の除去は gpu1 の表に従って PM の判断後)。
- **no-options**: 新しい旗・環境変数は足していない(`--hybrid-mx-tls` の意味を広げただけ)。足場の `GIL=1` は残っている(『唯一の構成』にする変更は未実施)。
- **進捗の誠実性**: 実エンジンの結果は無い(§0)。数字は全て実測。

## 5. 自己申告(0301 の区間)

- 使い捨ての木で `git init` を誤って実行し、すぐ `.git` を消した(自分の木、相対パス)。
- 最初に `hyb_guard cpart` を if/else の 2 か所にして、`ci/test-hybrid.sh` の数の検査を落とした。落ちてから 1 か所に直した(上)。
- 対照実験(ヘルパーを絶対番地に戻す)で一時的に `lib.rs` を書き換えた。コピーから戻し、`if false &&` が残っていないことを確認した。
- `test.sh full` の 16 失敗を見て、原因が rebase か足場かを分けるために E9 の対照木(38efb398 + rt-on-e9.diff)を作って同じ 13 binaries を走らせた(結果: 同一)。対照木の最初の作成は `set -e` と `grep -c`(0 件で終了コード 1)で途中で止まり、やり直した。
- 存在しない `bash` ツールをまた 1 回呼んだ(通算の自己申告に追加)。

## PM の判断と取り込み(2026-11-06 16:19、docs のみ。使い捨ての木、製品の差分なし)

取り込み: mt3 の第 1 報の文書・表(.rej を含む)と `rt-on-e11.diff`(237,588 B、60 ファイル)を `docs/investigations/mt3w/` に置いた。**実エンジンの音・画面の結果は 0 件**(コードの載せ直しと試験だけ。冷間 CI ステージビルドと実ブラウザは gpu1 が 16:35 ごろから)。依頼者の機械は 0 件。

事実として受け取る(PM は dry-run を再現していない。mt3 の測定):
1. `rt-on-e11.diff` は `git archive 7373cd55` に `patch --dry-run` rc=0・offset/fuzz なし。rt-on-e9.diff をそのまま当てると 3 ファイル 5 hunk が不成立(gpu1 の 0289 §11 と一致)で、すべて機械的。
2. **意味の衝突(patch に出ない)**: `fle_landing_top` は TLS_MORE では fle_ctx の欄(FLE_CTX_LANDTOP 0x4D0)で link map にデータの行が無く、E11 の型付き間接呼びの共有ヘルパーは landing を絶対番地で読む。解き方(新しい旗なし): `--hybrid-mx-tls` のとき `ti_helper` が landing を `reg_base(landing)`(`__tls_base` 経由)で読み、bundle スクリプトは `GIL=1` で `LAND = RB + 0x4D0`。試験を 1 本足し、ヘルパーを絶対番地の読みに戻すと落ちることを確認(対照実験)。census hook・`fle_progress_limit`・count は大域のまま(競合は heuristic の範囲、厳密にするなら count を TLS/atomic に — 未実施)。
3. 検証: cargo test --no-run rc=0、fe-wasmgen 73 passed/0 failed/1 ignored、ci/test-hybrid all checks hold(**hyb_guard cpart を if/else の 2 か所にすると失敗する**ので 1 か所にした。gpu1 の rt-on-e9.diff の if/else も同じ理由で落ちるはず)、a4 942/942・895/895・5,310・1,009,737(基準と同一)、`test.sh full` 222 results/2,405 passed/**16 failed**(13 binaries)。**16 失敗は足場自身**(E9 に rt-on-e9.diff をクリーンに当てた対照木で同じ 16 試験が同じ形で失敗)。原因の読み: import 表に CreateSemaphoreW/ReleaseSemaphore の 2 行が増えて「行数・位置・擬似 VA が動かない」を固定した試験が 10 本ずれる、ほか srw・callback・wall_wrp1 の単体 link など。= 0289 §8 の「試験 8〜9 日」の中身で、この rebase ではやっていない。

判断:
1. **`rt-on-e11.diff` を go/no-go #2 の足場として採る**(使い捨て、製品の木に入れない)。冷間ビルドは gpu1 が 0297 の Chrome の後(16:35 ごろから)、nice 19・CI_JOBS 3 以下で行い、続けて 5 分再生 + matched-music の窓で測る(成功/失敗の数値条件は決定 0289 の PM 節)。
2. 16 失敗の仕分け(期待値の更新か足場の修正か)・count/limit の競合の厳密化・`check_i1.py` は、go/no-go #2 が「続ける」になってから着手する(今は見積りだけ)。
3. 未確認: 冷間ビルド、実ブラウザ、`check_i1.py`、count/limit の競合の影響。
