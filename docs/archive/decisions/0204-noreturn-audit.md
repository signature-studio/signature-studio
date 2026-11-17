# 決定 0204(仮): `noreturn` 宣言と『実際に返す実装』の不一致を機械で検出する(決定 0190 の再発防止)

担当 noret1。基準 HEAD 48cec69。実エンジンは実行していない(静的検査: clang で C のヘッダーと harness/runtime の C を読むだけ)。G 段階・性能(P1〜P3、S1)は動かさない。

## 冒頭三点

1. **何を実測したか**(検出器を HEAD 48cec69 に走らせた結果。代役・再生・推測は含まない)
   - `lift/harness/noreturn-audit/noreturn_audit.py`(clang のみ使う)を HEAD の `git archive` 木に走らせた。runtime のヘッダー 94 本のうち 87 本は単独で構文解析でき(残り 7 本〔`fle_errmsg_*.h`、`fle_mxcsr_cvt.h`、`fle_mxcsr_flags.h`、`fle_rcp.h`、`fle_vex.h`〕は .c の途中に取り込む断片で、`noreturn` の語を含まない)、`noreturn` と宣言された関数は **45 名**。うち **23 名は、生成単位が include するヘッダーの閉包の中**(`fle_modules.h` の 20 名ほか `fle_seh.h`・`fle_prune.h`)にある。
   - C 93 本(runtime 76、harness の該当 15、harness の wrap 生成器 3 本が書く C 3)を `clang -O0 -S -emit-llvm` で IR にし、45 名の強い定義と `__wrap_<名>` の **222 定義**を調べた。**返す経路(IR に `ret` がある)を持つのは 12 件**で、**wrp2(決定 0190)の列挙した 5 名そのもの**: GetModuleHandleExW×3、GetFileType×3、SetFilePointerEx×3、RtlPcToFileHeader×2、SleepConditionVariableSRW×1(wrap の置き場は `harness/two/wrp1_vst3_imp.c`、`harness/vst/vst_env.c`、`harness/vst3a/vst_env3a.c`)。**5 名以外の新規は 0 件**。runtime の強い定義で返す物は 0 件、clang 自身の `-Winvalid-noreturn` 警告も 0 件。検出器は壁時計で約 15 秒(user 18 秒 + sys 11 秒、この機械の load 9〜18 のとき。clang の並列は 4)。
   - 12 件のうち生成単位から宣言が見えるのは GetModuleHandleExW の 3 件だけ(`fle_modules.h`。決定 0190 の opt-in `-DFLE_GMHEX_RETURNS` で扱い済み)。残り 9 件(4 名)の宣言ヘッダー(`fle_vol.h`・`fle_proc.h`・`fle_sys.h`)は生成単位が include しない。これを、検出器の判定とは**独立に clang で確認**した: 生成器が書くものと同じ include の単位に、生成器が書くものと同じ形の直接呼び出しのブロック(`R[4] -= 8; { void fle_imp_X(void); fle_imp_X(); } R[4] += 8;`)を置いた IR で、呼び出し後を捨てる(`unreachable`)のは GetModuleHandleExW・ILClone・deferred_qfc_InitSharedPython、捨てないのは GetFileType・SetFilePointerEx・SleepConditionVariableSRW・RtlPcToFileHeader・ExitProcess(テスト `a_lifted_style_call_block_…`)。
   - 生成 C の中での直接呼び出し(`fle_imp_X();` の文): FLEngine の既定の生成 C(sha256 `cd9107ce…`、215,811,678 B、この HEAD の fe-lift で再生成して一致): GetModuleHandleExW 0、GetFileType 1、SetFilePointerEx 0、SleepConditionVariableSRW 0、RtlPcToFileHeader 0。Serum2 の m7.c(`lift/out-wrp3/serum2/gen-1/m7.c`、**やや古い fe-lift の出力で参考値**): GetModuleHandleExW 5、GetFileType 4、SetFilePointerEx 5、SleepConditionVariableSRW 3、RtlPcToFileHeader 4。他モジュール(dsp、Blend2D、QFC、FreeType、ILTools)の生成 C では数えていない。
   - 変異検査: テスト内の変異テスト 5 件(変異 6 通り)と、手動の変異 M1〜M6(`tools/mutations.py`、`logs/mutations.txt`)のすべてで検出器が落ちた。無変異の写し(M0)は通る(§4)。
2. **結論**: 再発防止の機械的な検査を、新しい 3 ファイル(検出器、許可リスト、Rust テスト)だけで足した(テストは 8 件)。`lift/runtime`・生成器・既存のテストは 1 バイトも変えていない。既定の生成 C・runtime の挙動は不変。許可リストは 12 行(各行に理由)。次のどれかが起きるとテストが落ちる: (a) 許可リストにない『返す実装』が増えた(新しい wrap、新しい noreturn 宣言)、(b) 許可リストの行が何にも当たらなくなった(宣言か wrap を直した。行を消せ)、(c) 行の理由のタグの前提が崩れた(`not-gen-visible` の宣言ヘッダーが生成器の include の閉包に入った、`opt-in-macro` の宣言が条件つきでなくなった)、(d) noreturn の名前を含む C がコンパイルできず見えない。逆向き(返さない実装を返す宣言)は 60 件を表にしたが害が無いので強制しない。
3. **未確認**: §6 の『扱えない範囲』(JS・アセンブラの実装、どのビルドがどの wrap を実際にリンクするか、noreturn の呼び出しの後ろに続きを期待する harness 側の呼び出し元、IR に `ret` はあるが実行では通らない経路、**転送 wrap の helper の中身**。特に診断ビルドの `diag_stub_call` は名前つき停止を `_longjmp` で『返った』ことにする〔§6 の 7〕)。他モジュールの生成 C の直接呼び出し数。全体テストの数値は §5。

## 1. 問題(決定 0190 の要約)

`lift/runtime/fle_modules.h` が `fle_imp_GetModuleHandleExW` を `__attribute__((noreturn))`(『名前つき停止』、決定 0078)と宣言しているのに、ラッパー用ビルドのハーネスの `__wrap_fle_imp_GetModuleHandleExW` は値を返した。生成単位は `fle_modules.h` を include するので、clang -O0 は呼び出しの後ろを到達不能として**コードごと捨て**、ゲストのスタックが 8 バイトずれて `CreateThread` の `dwCreationFlags` が壊れた(壁 W12)。同種の宣言と実装の不一致が他に 4 名あると wrp2 が手で列挙した(`tables/noreturn-audit.tsv`)。機械的な検査が無かった。

## 2. 方式(選択と根拠。S- は §7)

| 段 | やること | 道具 |
|---|---|---|
| 宣言 | `lift/runtime/*.h` を 1 本ずつ単独の翻訳単位として `clang -fsyntax-only -Xclang -ast-dump=json` で読み、関数の型に `noreturn` を持つ物を (名前、ヘッダー、行) で取る。通常と `-DFLE_GMHEX_RETURNS` の 2 状態を読み、片方だけ noreturn の物は『マクロ条件つき』と記録する | clang AST |
| 生成単位から見えるか | 生成器のソース(`crates/fe-lift/src/*.rs`)の文字列リテラル `#include \"fle_*.h\"` を拾い、その `#include` の閉包を『生成単位から見えるヘッダー』とする | 文字列と正規表現 |
| 実装 | runtime と harness の C で、noreturn の名前(または `fle_imp_##` の貼り合わせ)に触れる物を `clang -O0 -S -emit-llvm -gline-tables-only -Winvalid-noreturn` で IR にし、`<名>` と `__wrap_<名>` の定義に `ret` 命令があれば『返す経路がある』とする。先頭に前置ヘッダーを `-include` し、`__real_<名>` を noreturn と宣言する(wrap が `__real_` に流すだけなら、本物が返すときだけ返す。つまり宣言と矛盾しない) | clang IR |
| 同じ翻訳単位の強い定義 | 宣言を見ている翻訳単位で noreturn の関数が返すと、clang は末尾を `unreachable` にして IR に `ret` を残さないので、警告 `-Winvalid-noreturn`(位置は閉じ括弧で名前を含まない)を、同じファイルで直前に始まる関数に割り当てて『返す定義』とする | clang 警告と DISubprogram |
| 転送 wrap | `__wrap_X` の本体が 1 ブロックで呼び出しが 1 つだけ、その引数に `@__real_X` を持つ(`diag_stub_call(i, __real_X)`、`v1_call("X", __real_X)`)物は『本物を helper に渡す転送』として所見に入れない(表 `forwarding-wrappers.tsv` に 82 件) | IR の形 |
| wrap 生成器 | `harness/idle/gen_idle_wraps.py`、`harness/diag/gen_wraps.py`、`harness/web/perf2/gen_p2_wraps.py` を noreturn の名前に対して実行し、出力した C も同じ検査に通す | 生成器の実行 |
| 逆向き | runtime の `fle_imp_*` の強い定義で IR に `ret` が無く、noreturn と宣言されていない物 | clang IR |

別案と採らなかった理由: (1) 全宣言を『返す』に変える opt-in マクロ(`-DFLE_NORETURN_AUDIT`)で全体をコンパイルして警告を見る — wrap は別のシンボルなので警告が出ず、検出したい対象(`__wrap_`)が見えない。(2) objdump で `ret` を数える — 最適化なしの IR のほうが関数の境界と行番号が取れ、`noreturn` の取り扱いをコンパイラ自身に任せられる。(3) grep だけ — `V1API(GetFileType)` のようにマクロが名前を作る wrap を見逃す(clang に展開させれば見える)。

許可リスト `harness/noreturn-audit/allow.tsv`: 1 所見に 1 行(種別、名前、ファイル、理由)。理由の先頭のタグは検出器が検証する。`opt-in-macro`(宣言がマクロ条件つきであること)、`not-gen-visible`(宣言ヘッダーが生成単位の include の閉包の外にあること)、`other`(自由記述)。

## 3. 検出一覧(12 件。名前、宣言、返す実装、生成 C での直接呼び出し、扱い)

`tables/findings.tsv` に全列を置いた。

| 名前 | 宣言(ヘッダー:行) | 生成単位から見えるか | 返す実装(wrap) | 直接呼び出し(FLEngine 既定の C / Serum2 m7.c〔参考〕) | 扱い(許可リストのタグ) |
|---|---|---|---|---|---|
| GetModuleHandleExW | `fle_modules.h`:79(マクロ条件つき) | **見える** | `two/wrp1_vst3_imp.c`:300、`vst/vst_env.c`:134、`vst3a/vst_env3a.c`:140 | 0 / 5 | **opt-in `-DFLE_GMHEX_RETURNS`**(決定 0190)で扱い済み(`opt-in-macro`)。ラッパー用ビルドの生成単位は、呼び出しを含む物だけマクロつきで再コンパイルされる(`out-wrp2/tools/cycle.sh`、git の外)。決定 0190 §2: vst3a の単体ハーネスは `fle_modules.h` を取り込まないので起きなかった(その読みで、ここでは再実行していない) |
| GetFileType | `fle_vol.h`:33 | 見えない | `two/wrp1_vst3_imp.c`:362、`vst/vst_env.c`:198、`vst3a/vst_env3a.c`:225 | 1 / 4 | 生成単位が include しない宣言。**直さない**(`not-gen-visible`) |
| SetFilePointerEx | `fle_vol.h`:39 | 見えない | `two/wrp1_vst3_imp.c`:375、`vst/vst_env.c`:215、`vst3a/vst_env3a.c`:242 | 0 / 5 | 同上 |
| RtlPcToFileHeader | `fle_proc.h`:84 | 見えない | `two/wrp1_vst3_imp.c`:204、`vst3a/vst_env3a.c`:337 | 0 / 4 | 同上 |
| SleepConditionVariableSRW | `fle_sys.h`:98 | 見えない | `vst3a/vst_env3a.c`:466 | 0 / 3 | 同上 |

- `not-gen-visible` の 9 件について: 生成 C は各 API を `void fle_imp_X(void);`(noreturn なし)と宣言して呼ぶので、呼び出しの後ろは捨てられない。runtime と harness の C にこれらの直接の呼び出しは 0 件(`tables/direct-callers.tsv` は ExitThread の 4 件だけで、ExitThread は本当に返らない)。もし将来、生成器が `fle_vol.h` などを include するようになれば、この行は当てはまらなくなりテストが落ちる(変異 M2)。
- GetFileType の FLEngine 既定の生成 C の中の 1 回の直接呼び出しも、`fle_vol.h` を見ない単位なので上記の IR の確認のとおり捨てられない。
- `fle_imp_ILFindLastID` ほか PIDL の 8 名、`fle_imp_deferred_*` の 11 名は生成単位から見える(`fle_modules.h`)が、返す実装は無い(harness の `vis1_probe.c`・`diag` の wrap は本物へ転送)。

## 4. 変異検査(宣言と実装の一致を意図的に壊す。落ちることを確認)

テスト(`noreturn_audit.rs`)の変異 5 件と、手動(`tools/mutations.py`、出力は `logs/mutations.txt`)の M0〜M6:

| 変異 | 期待 | 結果 |
|---|---|---|
| M0 無変異の写し | 通る | exit 0 |
| M1 harness に `__wrap_fle_imp_ILClone` を足して値を返す | 新規の所見 | `NEW FINDING [returns] fle_imp_ILClone … gen-visible=yes`、exit 1 |
| M2 生成器が `fle_vol.h` を include するようにする | GetFileType・SetFilePointerEx の許可行が無効 | 6 件が落ちる(`allow row … says not-gen-visible, but a header … is in the include closure`) |
| M3 `fle_modules.h` を決定 0190 の前の無条件 noreturn に戻す(決定 0190 の元の欠陥の再現) | GetModuleHandleExW の 3 件が落ちる | `says opt-in-macro, but the declaration is no longer conditional` |
| M4 GetFileType の宣言を noreturn なしにする(宣言を直した) | 許可行 3 件が古くなる | `STALE ALLOW ROW` 3 件 |
| M5 `fle_modules.h` に LoadLibraryA の noreturn 宣言を足す(wrap が返す) | 新規の所見(runtime の定義自身も clang 警告で出る) | 3 件 |
| M6 vst_env.c の GetFileType の wrap から早期 return を消す(返さなくなった) | 行が古くなる | `STALE ALLOW ROW` 1 件 |

検出器そのものの検査(合成の木): 早期 return、無条件の return、マクロが作る wrap、同じ翻訳単位の強い定義、マクロ条件つき宣言は見つけ、本物への直接の転送と helper への転送は見つけず、コンパイルできないファイルは報告し、逆向き(止まるのに返すと宣言された関数)を表に出す(テスト `the_detector_finds_each_kind_of_return_path_and_not_a_forwarder`)。

## 5. 既定の挙動への影響・テスト

- `lift/runtime`、`lift/crates/fe-lift/src`、既存のテスト、既存の harness は変更なし。追加は新規 3 ファイル(`harness/noreturn-audit/noreturn_audit.py`、`harness/noreturn-audit/allow.tsv`、`crates/fe-lift/tests/noreturn_audit.rs`。`noret1.diff` はこの 3 本だけ)と、文書(本ファイル、README の 1 行)。
- FLEngine の既定の生成 C: この HEAD の fe-lift で `fe-lift lift FLEngine_x64.dll out.c --pdata` を再実行して sha256 `cd9107cef4cbef14f154645749957b7067be7ed7c2b93f5bbe1f72fd47029e50`(215,811,678 B)(`logs/gen-default.sha256`)。
- a4(FRUITY 無効 942/942、895/895、functions 5,310、entries 1,009,731): **再実行していない**。fe-lift・runtime・既存の harness に触れていないので動く理由が無い(読み)。
- 全体テスト: §結果の節(本ファイル末尾)に実測値を書く。新テスト単体は 8 件 passed(約 18 秒)。`harness/testfast/deps.tsv` は触っていない(記録の無い実行ファイルは `affected` で常に対象になる)。`harness/test.sh record` で入力集合を記録し直すのは PM の未決。

## 6. 扱えない範囲(『未対応』の明記)

1. JS・アセンブラの実装: `fle_wasm_thread_exit`(`fle_thread_wasm.c` の noreturn 宣言)の実体は `harness/web/threads/fle_thread_lib.js` の `throw` のみで、返らない(手で読んだ)。ほかに N 名の JS 実装は grep で 0 件。
2. **どのビルドがどの wrap を実際にリンクするか**: `-Wl,--wrap=` を渡すのはビルドスクリプトの側(`harness/vst*/build_*.sh`、`two/build_nmod_wasm.sh`、`vst3a/wraps.txt`、`vis1/link-flags.txt` ほか)で、ここでは『返す wrap のソースがある』ことだけを見ている。表 `findings.tsv` の列『-Wl,--wrap= を持つスクリプト』は grep の結果。
3. noreturn と宣言された名前を harness が**呼んで**、続きを期待している場合(clang が続きを黙って捨てる): 表 `direct-callers.tsv` に C の呼び出し元を出すが(ExitThread の 4 件のみで問題なし)、`-Wunreachable-code` による検出はしていない。
4. 生成 C の中の呼び出し数: FLEngine の既定の C(再生成)と Serum2 の m7.c(古い fe-lift)だけ。他モジュールは未実施。
5. 逆向きの不一致(返さない実装を返す宣言)は表にするだけ: `reverse-direction.tsv` に 60 件(`fle_gdi.c` 43、`fle_filedlg.c` 15、`fle_wall_ia2.c` 1、`fle_msctf.c` 1。すべて『止まる』スタブ)。害は無い(呼び出しの後ろが死んだコードになるだけ)ので強制しない。`harness` の `__wrap_` が返さない物は 0 件。
6. 構造的な検査: IR に `ret` があっても実行では通らない経路は『返す』と数える(許可リストの理由で扱う)。`longjmp` など非局所の脱出、`noreturn` を推論できない helper を呼んで終わる関数は『返す』側に出る(今の木では 0 件)。
7. **転送 wrap の helper `diag_stub_call`(`harness/diag/diag.c`、DIAG=1 の診断ビルド専用)は、名前つき停止(abort)を `_longjmp` で捕まえてスタブが『返った』ことにし、仮の値(rax = `FLE_DIAG_RET`)で実行を続ける**(同ファイルの冒頭に明記。『進捗ではない』と書かれた診断)。つまり、生成単位から見て noreturn と宣言された名前(`fle_modules.h` の 20 名)のスタブが止まる場合、diag ビルドでは返ることになり、本決定の IR 検査は『転送』として所見に入れない。この影響は、その名前を直接呼ぶ生成 C があるときだけ出る: FLEngine の既定の C では GetModuleHandleExW・PIDL の 8 名・deferred_* の 11 名の直接呼び出しは 0 件(`tables/declared-noreturn.tsv`)なので今は起きない。DIAG=1 でほかのモジュールをビルドするときは要確認。表 `forwarding-wrappers.tsv` に 41 件(`generated by harness/diag/gen_wraps.py`)として出る。
8. コンパイルできない C: `harness/wasm/jspi_hooks.c`(EM_JS、`emscripten.h`)は noreturn の名前を書いていない(`WRAP(fle_imp_EnterCriticalSection, …)` など 6 名)ので所見ではなく表に出すだけ。名前を書いたファイルがコンパイルできなくなったら所見(テストが落ちる)。

## 7. 出典のない選択(S-)

| 番号 | 選択 | 代替・撤回条件 |
|---|---|---|
| S-noret-1 | 『返す』の判定を `-O0` の IR の `ret` の有無(構造)にする | 代替: マクロ `-DFLE_NORETURN_AUDIT` で警告を見る(wrap が別シンボルなので見えない)。撤回: 偽陰性が見つかったとき |
| S-noret-2 | `__real_<名>` を noreturn と仮定する | 代替: 返すと仮定(転送 wrap 82+ 件が所見になり許可リストが膨らむ)。`__real_` の実体が返す実装なら、その実体自身が所見になるので穴にならない |
| S-noret-3 | 転送 wrap(1 ブロック、呼び出し 1、引数に `@__real_X`)を所見にしない | 代替: `diag`・`vis1_probe` の 82 件を許可リストに載せる。**判定は形だけ**で、helper の中身は見ていない。読んだ結果は §6 の 7: `v1_call` は本物を呼ぶだけだが、`diag_stub_call` は名前つき停止を `_longjmp` で『返った』ことにする(診断用の DIAG=1 ビルド)。撤回: 他の helper が本物を呼ばずに返す harness が出たとき |
| S-noret-4 | 『生成単位から見える』を、生成器のソースの `#include \"fle_*.h\"` リテラルの閉包とする | 代替: 生成器で単位を作って `clang -H`(215 MB の lift が必要)。補強: 独立の IR の確認テストがある |
| S-noret-5 | 許可リストの行の理由にタグを付けて検証する | 代替: 自由記述だけ(理由の前提が崩れても気づけない) |
| S-noret-6 | 4 名の宣言ヘッダーを直さない(許可リストのまま)。PM 判断(第 1 回 report への返信): 直さない | 代替: 宣言を noreturn なしにする(runtime の属性変更。呼び出しの無い宣言で、挙動は変わらないが本決定の範囲外) |
| S-noret-7 | 逆向きは表だけで強制しない | 代替: 件数を固定して増減で落とす(停止スタブの追加で毎回更新が要る) |
| S-noret-8 | 単独で構文解析できないヘッダーは、`noreturn` の語を含まないときだけ読み飛ばす | 含むのに解析できなければ tool-error(テストが落ちる) |
| S-noret-9 | 合成の木と変異の一時ファイルは `std::env::temp_dir()`(`/tmp`)に作り、終了時に消す | 数 MB |

## 8. 実測値(全体テスト)

この機械(16 コア、他の worker と共有。load 8〜19)の値。`harness/test.sh full --jobs 2 --fresh`(キャッシュを使わず全実行ファイルを実際に走らせた。`executables: run=174 cached=0`)、木は HEAD 48cec69 の `git archive` + `noret1.diff` + 製品入力の symlink(`harness/test-env.sh --link`):

- **results=178 passed=2274 failed=0 ignored=0 SKIPPED-lines=1**(SKIPPED は `fe-lift/wall_fr9t` の 1 行のみ。基準の 177 results・2,266 passed・SKIPPED 1 に、新しい実行ファイル `fe-lift/noreturn_audit` の 1 結果・8 件が加わった値)。壁時計 475 秒(`logs/fulltest-1.txt` = `logs/fulltest-final.txt`)。`fe-lift/noreturn_audit` は passed=8、38.0 秒(他の実行ファイルと並走したので単体の 18 秒より長い)。
- FLEngine の既定の生成 C: sha256 `cd9107cef4cbef14f154645749957b7067be7ed7c2b93f5bbe1f72fd47029e50`、215,811,678 B(HEAD の fe-lift、`fe-lift lift binaries/win64/FLEngine_x64.dll out.c --pdata`、`logs/gen-default.sha256`)。
- `patch -p1 --dry-run`: 最新の HEAD(48cec69)の `git archive` に当たる(`logs/dryrun.txt`)。`-` の行は 0。
- a4: 再実行していない(§5)。
- 検出器単体: 実木に対して `logs/audit-final.txt`(12 所見、許可リスト外 0、古い行 0、exit 0)。表は `tables/`。

途中の記録: 最初の全体テストは 7 件版のテストファイルで始め(8 件目の『生成単位の呼び出しブロックの IR』を後から足したため)、36 実行ファイルの時点で自分のプロセスグループごと止めて 8 件版で最初からやり直した(`logs/fulltest-0-killed-7test-version.txt`。最終の値は 2 回目のみ)。

## PM の決定と取り込み(2026-11-04)

取り込み: `noret1.diff` は新規3ファイルだけ(`lift/harness/noreturn-audit/noreturn_audit.py`、`allow.tsv`、`lift/crates/fe-lift/tests/noreturn_audit.rs`)で、`-` の行は0。HEAD 48cec69 に当たる。PM の測定: 新テスト `noreturn_audit` 8 passed(単体 13.3 秒)。全体テストは作業者の値(178 results・2,274 passed・0 failed)を採り、PM は dd1・cbeacon2 の統合の後にまとめて通しで再走する。runtime・fe-lift・既存テストは変更していないので、FLEngine の既定の生成 C の sha256 と a4 は変わらない(作業者の確認)。

判断:

1. 検出された12件は決定 0190(wrp2)が手で列挙した5名そのもので、新規は0。許可リスト12行(`opt-in-macro` 3、`not-gen-visible` 9)の理由を採用する。4名の宣言(`fle_vol.h`・`fle_proc.h`・`fle_sys.h`)は直さない(生成単位が include しない宣言であり、この決定は検査だけを入れる)。許可リストのタグの前提が崩れる(宣言ヘッダーが生成器の include に入る等)とテストが落ちる(変異 M2 で確認)。
2. 逆向き(返さない実装を、返す宣言で呼ぶ)60件は表にするだけで強制しない。
3. **診断ビルド(`diag/diag.c` の `diag_stub_call`)は名前つき停止を `_longjmp` で「返った」ことにして実行を続ける。** FLEngine の既定の C では該当する直接呼び出しが0件で今は問題ないが、他のモジュールを DIAG=1 でビルドするときは、生成単位から見える noreturn 名のスタブが止まった後の実行が実際には続く点を確認すること(S-noret-3)。
4. `deps.tsv` は触っていない。新テストは依存の記録が無いので `test.sh affected` では常に対象になる。記録し直すかは必要になった時に判断する。
5. 作業者の自己申告(存在しない bash ツール1回、同時の重い処理が2本超、7件版の全体テストを途中で止めて取り直し、`tail --pid=… -f /dev/null` の待ち1回、作業ディレクトリ一時 3.2 GiB〔上限 3 GiB〕、自分の ft コピーの symlink を `rm -rf`)は記録する。
