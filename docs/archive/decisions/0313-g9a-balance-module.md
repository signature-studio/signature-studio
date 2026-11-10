# 0313: G9-a — Fruity Balance を 8 つ目の実モジュールとして足す変更案(patch は dry-run 済み)、行なし import の扱い、検証計画

状態: **仮の決定(すべての選択)**。担当 mt3、依頼は PM(0313)。**実エンジンの wasm は実行していない。Chrome は起動していない。ビルドもしていない。** この文書の数字は、静的な検査(`bash -n`、`ci/test-hybrid.sh`、`clang`/`emcc -fsyntax-only`、`patch --dry-run`、`fe-lift` 1 回〔0.96 s〕、python の読み取り)の実測だけ。**「踏む・踏まない」「壁の数」は読みか未確認**で、実行の観測ではない。代役・画像の再生・事前生成は無い。

## 0. 先に結論

1. **変更案**: 10 ファイル・224 行の diff(`impl/0313-g9a-balance-module.diff`)。**main HEAD 0635cc3f〔最初の版は f0e5a2f5 で rc=0〕の `git archive` に `patch --dry-run -p1` が rc=0、当てた木で `bash -n` 6 本 ok、`ci/test-hybrid.sh` 51 ok・0 FAIL(新規 5 件を含む)、生成した 8 モジュールの表+パス表が `emcc -fsyntax-only -Wall -Wextra` rc=0・警告 0、native `clang` も同じ**。S3a/B1 の積み重ねとは独立(app/ と fe-lift/runtime は触らない)。`impl/apply-0313.sh check` で再現できる。
2. **行なし import**: Balance の import 543 個のうち行(応答)が無いのは **74 個**(0309 の表)。製品の行の集合(`ship6-imports.txt`)で Balance を lift すると、**`fle_unsupported(VA)` の thunk は 44 個(= 74 のうち lift された thunk を持つ物。残り 30 個は thunk が lift 対象に入っていない)**。ランタイムの扱いは「呼ばれたらその VA 名つきで停止(`abort`)」だけで、**黙って 0 を返す既定の応答は無い**。FLEngine の steady startup の記録にはこの種の停止は 0 件。Balance の `CreatePlugInstance` から**直接 call だけで**辿れる範囲(下限)に入る物は 44 個中 **2 個(`ScrollWindow`、`GetClassLongPtrW`)**。
3. **検証計画**: V0(静的、今回済み)→ V1(native、任意)→ **V2(wasm の冷間 build + alpha1 の node の G3 の窓表)** → V3(Chrome、G9-c)。**G9-a の合格 = 『Problems 窓の "plugin(s) could not be opened" の件数が 1 減り、Balance の行が消え、`LoadLibraryW` が 0x6800000 を返し、`GetProcAddress("CreatePlugInstance")` が 0x6abdde0 を返して呼びが戻る』**。壁の数の予想は 0309 の 2〜4(低い確度)に対し、Balance は GUI を `CreatePlugInstance` で作らないので少なめ(読み)だが、**未確認**。

## 1. 変更の一覧(0309 §4b の 9 項目との対応)

| # | 0309 §4b の項目 | この patch | ファイル |
|---|---|---|---|
| 1 | CI の抽出行 | `Fruity Balance_x64.dll` → `lift/out-plugins/fruity_balance_x64.dll`。**sha256 `a399cbe060ec3002a920f62fdcc2a2fb36bb1f3918281e01cdd5fc70724b8064` はインストーラ `flstudio_win64_26.1.7.5653.exe` の中身と一致を確認**(裸のファイル名で候補 1 件。フルパスだと 0 件=下の自己申告) | `ci/extract-dlls.tsv` |
| 2 | モジュール表への行 | 8 本目の spec(名前 `fruity_balance_x64.dll`、基底 **0x6800000**)。基底 0x6800000 は ILTools(0x6000000〜0x6394000)の上、Elastique(0xb000000)の下の空き。Balance の SizeOfImage 0x3a1000 → 終わり 0x6ba1000。**FLEngine・QFC・dsp・Blend2D・FreeType・ILTools・Balance の SizeOfImage で重なり 0 を確認**(Elastique・Oggio は基底が遥か上で、この機械に PE が無く SizeOfImage は未確認)。`table_check`(小文字・パス無し・64 KiB の倍数・0x400000 でない・一意)に合う | `lift/harness/b2d2/gen-direct.sh` |
| 3 | 基底 | 0x6800000(同上)。FLEngine 0x400000 のまま | 同上 |
| 4 | フック点 | `rva:0x17bc0`、`rva:0x31d70`、`rva:0x31d90`(alpha1 の `find-hook-points.py`。FLEngine・Wrapper と同じ Delphi RTL の prologue 形) | `lift/harness/hooks/fruity-balance-hook-points.txt`(新規) |
| 5 | `RM_MAX` | **変更なし**(8。7+1 で足りる)。`main_two.c` の `MAIN_REAL_MAX` 16 も足りる | — |
| 6 | パス表 | `fle_realmod_path_table` に `{"plugins\\fruity\\effects\\fruity balance\\fruity balance_x64.dll", "fruity_balance_x64.dll"}`。**行が 1 つも無い構成ではパス表を出さず、B1 の `realmod_table_gen.c` と 1 バイトも違わない**(`test-hybrid.sh` で確認)。awk を `realmod_tables.awk` に切り出した(`gsub` の escape が mawk と gawk で違うため使わない) | `gen-direct.sh`、`lift/harness/b2d2/realmod_tables.awk`(新規) |
| 7 | volume 上のファイル | 実 DLL を `C:\Program Files\Image-Line\FL Studio 21\Plugins\Fruity\Effects\Fruity Balance\Fruity Balance_x64.dll` へ(`fle_load.c` の `volume_file` が要る。パスはインストーラの展開木で確認) | `lift/harness/seeds/vfs-seed-idle1.tsv` |
| 8 | 組み込み | `EXTRA_MODULES="oggio elastique fruity_balance"; NEXTRA=3`(他の個数は `NEXTRA` に従う)。コメントのみ: `ci/objkeys.sh`(`m[6-9]` の loop が m8 を既に含む)、`ci/stage-lift.sh`、`ci/stage-bundle.sh` | `app/scripts/build-engine-bundle.sh` 他 |
| 9 | テスト | `ci/test-hybrid.sh` に 5 件(抽出行、種、フック点、awk が B1 の表を再現、パス表の形) | `ci/test-hybrid.sh` |

- **B-FLE の共有本体は G9-a では使わない**(PM の指示)。Balance の C は 35,335,956 B(製品の行で lift した実測。FLEngine の素の lift は 215,741,085 B なので比は 16.4%)。**wasm の増分は未測定**(C のバイト比からの読みは FLEngine 比 十数 %。B-FLE の 0312g〜i の見積りでは共有なしの 1 つ目 約 11.6〜12.3 MB〔読み〕)。
- **volume のファイルは実 DLL(+3.7 MB の seed pack)**を採る。DLL の中身を FL が読むかが**未確認**なので、stub(先頭だけ)への置換は G9-a の後の最適化(P1/転送量)。
- **フィデリティ区分**: この patch の足す処理は (b) 外部 API の契約の実装(`LoadLibraryW` のフルパス = 決定 0183 の既存の機構を Balance に使う)と、モジュール追加の手続きだけ。**新しい旗・env・クラス名/アドレスによる分岐は無い**。

## 2. 行なし import(74)の扱い — file:line

**数字(実測)**
- 出典: `docs/investigations/alpha21/tables/0309-imports-summary.tsv`・`0309-imports-none.tsv`(alpha1)。Balance の import 543 個、行あり 469、**行なし 74**。74 個はすべて FLEngine も import している(`no_row_and_not_in_FLEngine = 0`)。写しは `tables/0313-balance-74-no-row-imports.txt`。
- 内訳(dll 別): user32 24、comctl32 22(`ImageList_*`)、imm32 7、kernel32 6(`EnumCalendarInfoW`・`GetCPInfoExW`・`GlobalDeleteAtom`・`GlobalSize`・`HeapDestroy`・`RtlUnwind`)、uxtheme 4、ole32 4、shell32 3、wtsapi32 1、windowscodecs 1、msimg32 1、CRT の `memset` 1。
- **製品の行(`lift/harness/seeds/ship6-imports.txt`)で lift すると `fle_unsupported` の thunk は 44 個**(`fe-lift lift … --pdata --hook-points <Balance の 3 点> --extra-imports ship6-imports.txt`、0.96 s・111,824 KB、C 35,335,956 B。`unsupported 1`〔理由は 1 種〕)。内訳: comctl32 22、user32 9、kernel32 5、ole32 4、shell32 3、memset 1。**74 のうち 30 個(imm32 7、uxtheme 4、user32 15、wtsapi32、windowscodecs、msimg32、`RtlUnwind`)は、import 表にあるが thunk が lift の関数に入っていない**(読み: 直接の call から到達しない)。表: `tables/balance-unsupported-44-rebased.tsv`(元の VA と、**ランタイムが印字する再基底後の VA = 元 + 0x6400000**、API 名)。素の lift(extra-imports なし)だと 95 個で、差の 49 個は製品の行がある API(`CreateMenu` ほか)。**自己申告: 以前の要約の『95 個』は素の lift の数で、製品の構成の数は 44。**

**ランタイムでの扱い(file:line)**
- lift 側: import thunk `jmp [IAT]` は、行があれば `fle_imp_*` を呼ぶ `{ void f(void); f(); return; }` になる(`lift/crates/fe-lift/src/lib.rs:673-683`)。**行が無いと `indirect::jmp` に落ち、`Resolved::Import` は `Err("indirect through IAT slot (not lifted)")`**(`lift/crates/fe-lift/src/indirect.rs:16`)で、census に "Jmp: indirect through IAT slot (not lifted)" として数えられる。
- 生成 C: その命令が `fle_unsupported(UINT64_C(<VA>))` になる(`lib.rs:1309`、`:1340` はコメントつき)。
- ランタイム: `lift/runtime/fle_runtime.h:87-90` の `fle_unsupported` は `fprintf(stderr, "fle: unsupported instruction at %#llx\n")` して **`abort()`**。**停止の文面は thunk の VA だけで API 名が出ない**。名前は PE の import 表から引く: `tools/thunk-names.py`(PE と生成 C を渡す)か `tables/balance-unsupported-44-rebased.tsv`。
- **黙って成功を返す既定の応答は無い**(誤った 0 を返して先へ進むことが無い)。これは lift の忠実性の方針に合う: 壁は『その API の契約を実装する』でだけ消す。
- 応答表は**全モジュールで同一**でなければならない(`merge_tables_n.py` が検査。決定 0183 §3.1 の記述、`gen-direct.sh` の `FLE_EXTRA_IMPORTS` コメント)。**74 個はすべて FLEngine も import しているので、行を足す = FLEngine も含めて同じ表に足す**。Balance のためだけの分岐にはならない。
- 先例: Wrapper(決定 0183)は 107 個の行なしを持ち、実際に踏んだのは `CreateJobObjectW`・`SetInformationJobObject` の 2 種だけ(実測)。

**『踏む物』の予想(読み。実行していない)**
- `tools/reach-unsup.py`: lift 済み C の**直接 call/jmp だけ**を辿った下限。`CreatePlugInstance`(0x6abdde0、元の VA 0x6bdde0)から 749 関数(12,549 の 6.0%)に到達し、`fle_unsupported` は **2 つ: `0x41e7f0` = `user32!ScrollWindow`、`0x41eda0` = `user32!GetClassLongPtrW`**(再基底後 0x681e7f0、0x681eda0)。PE の entry(0x6ac13a0)からは 43 関数・0。**Delphi の仮想呼び・dispatch table・vtable は辿らないので真の到達範囲はこれより広い**(上限は不明)。表: `tables/reach-unsup-product-rows.txt`。
- 足す順序: 停止した物を 1 つずつ契約(Learn)に従って実装する。`ScrollWindow`・`GetClassLongPtrW` は GUI の窓の API で、FLEngine は既に窓のモデル(`fle_user32*` 系)を持つ。実装の先は壁ごとに決める。**事前に 74 個を一括で行を足すことはしない**(契約の裏付けの無い『それらしい』応答を足すことになるため)。
- 未確認: 74 のうち FLEngine の旧 coverage(`union-final2.cov`)で 2 件(0x5928b0、0x4235d0)が踏まれた印があるが、これが import thunk かは確かめていない。

**直接 call で届く『行あり』の import(読み。実行していない)**: `tools/reach-imps.py` が同じ下限の閉包で数えた: `CreatePlugInstance` から **異なる `fle_imp_*` 91 個**、PE entry から 12 個(`tables/reach-imps-product-rows.txt`)。すべて行がある API(実装はある)。ただし**引数の形によって名前つき停止する実装があり得る**ので、最初の実行で止まる候補を見る順: 窓・描画系(`ShowWindow`・`SetWindowPos`・`SetForegroundWindow`・`SetClassLongPtrW`・`GetSystemMenu`・`FlatSB_*`・`StretchBlt`・`CreateDIBitmap`・`CreateIcon`)、スレッドと同期(`CreateThread`〔Wrapper の W12 は native の旗の問題で、wasm の実スレッドは別経路〕・`SwitchToThread`・`WaitForSingleObject`)、ファイル・資源(`FindFirstFileW`〔Wrapper でも resource DLL の探索で失敗を返して続行〕・`LoadLibraryExW`/`LoadLibraryA`・`FindResourceW`/`LoadResource`)、レジストリ(`RegOpenKeyExW`・`RegQueryValueExW`・`RegFlushKey`)。これは**停止の予想ではなく見る順の候補**で、実行までは不明。

## 3. 検証計画(段階と合格条件)

| 段階 | 内容 | 費用・条件 | 合格条件(観測する物) |
|---|---|---|---|
| **V0** | 静的(今回済み) | CPU 軽い | `patch --dry-run` rc=0。`bash -n` ok。`test-hybrid.sh` 全 ok。生成表が `emcc`/`clang -fsyntax-only` rc=0・警告 0。**済み(上の結論 1)** |
| **V1**(任意) | native -O0 の fork プローブ(決定 0183 の wrp1 の鎖: `docs/investigations/wrp3/tools/{build-h,link-native,cycle}.sh`)に Balance を 8 つ目として足し、参照 FLP を開く | 重い(native の全 build)。**この窓の外・費用未見積り**。wasm の V2 で足りるなら省く | `LoadLibraryW(フルパス)` が 0x6800000 を返す。DllMain 完走。`CreatePlugInstance` が戻る。`fle: unsupported instruction at` が出たら §2 の表で名前を引き、1 つずつ実装 |
| **V2a** | wasm の冷間 build(B1/B2 の方式: `CI_JOBS=1`、`BINARYEN_CORES=1`、nice 19、**22〜35 分**〔B2 の実測 00:16〜00:38、link 20 分 13 秒〕) | **PM の窓の info 後(02:00 以降の想定)**。lift の key が変わるので全段が冷間 | build rc=0。wasm の大きさ(**増分は未測定**。記録して PM へ)。実モジュールの個数 8、`fle_realmod_path_count` 1(リンク map か `strings`) |
| **V2b** | alpha1 の node harness(`docs/investigations/alpha18/tools/a19-g3-run.sh ref`、中身は `lift/harness/mt2/node-bundle-run-sh.mjs` + `ref1-perf-short.g3` + 参照プロジェクト `~/ダウンロード/serina1-serum2.zip`)で参照 FLP を開く。**§3b の 2 点が前提**(volume の種、観測の手段) | node 1 本。Chrome 不要 | §3b の判定 (i)〜(v) |
| **V3** | Chrome の GUI(G9-c) | PM の Chrome の窓 | V2b と同じ判定がブラウザで再現。Problems 窓の画面(実ブラウザ)。起動時間への影響(P1)を記録 |

## 3b. V2b の仕様(alpha1 の harness に渡す。2 つの前提と、期待する出力)

**前提 1 — volume の種(実測で見つけた穴)**: `node-bundle-run-sh.mjs` は `PARITY_SEED_TSV` を与えると `FLE_VFS_SEED` をその list に**置き換える**(`:71-75`)。`a19-g3-run.sh ref` は `lift/out-ref3/tables/vfs-seed-nested.tsv`(idle1 の volume + 参照プロジェクトの files。**本 patch の Balance の行が入る前に作られた**)を渡すので、そのままだと **bundle の seed pack に Balance の DLL があっても volume に載らず**、`LoadLibraryW(フルパス)` は `volume_file` で NULL + 126(`fle_load.c:204-212`、`load_with_path` の path table の分岐 `fle_load.c:322-330`: 『file が volume に無ければ NULL / 126』)になる。→ **`tools/mk-nested-balance-seed.sh` が nested.tsv に patch の行を 1 行足した写し(289 行)を作る**(行は diff から取る。nested.tsv 自体は読むだけ)。その写しを `SEED=` にして走らせる。ブラウザ(V3)は bundle 自身の seed(idle1)を使うので不要。

**前提 2 — 観測の手段**: 製品の bundle には `LoadLibraryW` の戻り値・`GetProcAddress` の戻り値を出す trace が**無い**(`ref3-lookup` は ref3 の診断 diff で製品外)。(i)(ii) を字義どおり観測するには、**テスト専用の小さな診断 diff**(`impl/diag-g9-loadlib.diff`、`fle_load.c` と `fle_modules.c` の 2 か所・計 7 行。フルパスの `LoadLibraryW` の結果と、実モジュールへの `GetProcAddress("CreatePlugInstance")` の結果を stderr に `g9-diag:` で印字。main に `patch --dry-run` rc=0、`clang`/`emcc -fsyntax-only` rc=0・警告 0)を**テスト bundle にだけ**載せる。製品の bundle には載せない。診断 diff を載せるかは PM の判断(載せると冷間 build は 1 回で済むが、この bundle は『製品の bundle』ではなくなる。載せないと (i)(ii) は間接の観測〔(iii)(iv)〕だけになる)。

**診断 diff は挙動を変えない(PM の条件 ①。コードを読んだ確認。実行はしていない)**: `LoadLibraryW` 側は `name_has_path(buf, n)`(読むだけ)、`ascii_of(buf, shown, …)`(書き先は局所の `shown` だけ)、`fprintf(stderr, …)`、`R[0]` の読みだけ。`load_by_name` は `const uint16_t *` を受けるので `buf` は変わらず、`R`・ゲストメモリ・last error・runtime の状態への書き込みは 0 行。局所配列 `shown` は `LOAD_PATH_MAX + 1 = 353` B の C スタック(Worker の C スタック 512 KiB に対し無視できる)。`GetProcAddress` 側は `strcmp(rname, "CreatePlugInstance")` と `fprintf` と `R[0]`・`h` の読みだけで、直後の `fle_set_last_error` の前に置いたので順序も変わらない。**変わるのは stderr への出力と、その分の数マイクロ秒だけ**(印字は フルパスの LoadLibraryW ごとと CreatePlugInstance の解決ごと。数十回の見込み〔未確認〕)。→ **この bundle で first idle や性能を測らない**(PM の条件 ④。P1 の値は CI が作る本番の bundle で後に取る)。
**扱い(PM の決定 01:31)**: 診断 diff は**push しない**(テスト専用)。push する diff は `0313-g9a-balance-module.diff`(224 行)だけ。診断を載せた bundle は名前に `test` を入れる(`fl-engine-test-g9a-<hash>`。**ただし単純な改名ではブラウザで 404 になる。§6 の rename を見よ**)。V2b と V3 は同じテスト bundle。製品の bundle は main に push したときの CI が作る(ローカルの 2 回目の build は不要)。

**V2a の準備(実行なし)**: `tools/g9a-test-build.sh tree|build|rename`。`tree` は main 0635cc3f の `git archive` に本 diff と診断 diff を当てた木(`~/work/mt3w-s1/tree-g9a`)を作る — **実行済み: 両 diff とも FAILED 0・offset/fuzz 0**。`build` は `lift/out-gpu2/tools/run-ci-build.sh` の段(extract felift lift b2d objects link seed bundle)を `CI_JOBS=1`・`BINARYEN_CORES=1`・nice 19 で走らせる(**PM の info 後。未実行**)。`ci/plan.sh` の段の key(`tables/0313-plan-keys-main-vs-g9a.txt`、実測): 変わるのは extract・lift・rt・link・seed・bundle(`app` も変わって見えるが、`git` の無い木では `hashpaths` が `:(exclude)` を解さず `build-engine-bundle.sh` の変更を拾うため〔`ci/lib/common.sh:19-25`〕。git の checkout の CI では app の key は変わらない見込み〔読み〕)、**変わらないのは vst3・felift・b2d・emsdk**(= fe-lift と Blend2D の build は再利用できる見込み〔読み〕。ローカルの run-ci-build.sh は段を無条件に走らせるので、`b2d` と cargo-target は ci-b2 の写しを置いて再 build の費用を避ける〔効くかは未確認〕)。

**訂正(alpha1 の実測 03:5x。E13 の製品の bundle を node で実際に走らせた結果)**: 上の V2b の書き方は**製品形式の bundle が node の G3 で回る**前提だったが誤り。(1) 製品の wasm は診断 env を読まない(`lift/runtime/fle_test_env.h`、決定 0243。`FLE_TEST_GETENV` は null の定数)ので、`FLE_G3_SCRIPT` も `FLE_FILEDLG_PICK` も効かず、G3 の script は実行されない。(2) 製品の bundle は `-sFILESYSTEM=0` で `FS` が無く、`node-bundle-run-sh.mjs` の `FS.mkdirTree` で落ちる。**V2b は `lift/harness/parity/make-test-tree.sh PRODUCT_TREE PRODUCT_WORK TEST_TREE TEST_WORK`(wasm TEST build。製品の木の複製で `fle_test_env.h` を getenv 版にし、FORCE_FILESYSTEM=1。再コンパイル・weblink・wasm-opt・hybrid・seed・bundle で 12〜17 分〔alpha1 の読み〕)の TEST bundle で回す**(alpha1 が担当。PRODUCT_TREE=`~/work/mt3w-s1/tree-g9a`、PRODUCT_WORK=`~/work/mt3w-s1/ci-g9a/engine`)。**V3(Chrome)は私の製品形式の bundle**(診断の stderr 行は出る。`fle_test_env.h` を使わない `fprintf(stderr)` だから)。実行の道具は alpha1 の `v2b-run.sh`(MEMFS 版)・`v2b-run-hf.sh`(HostFiles 版)と `seed-nested-balance.tsv`。

**期待する出力(診断 diff あり)** — `stderr.txt` に:
- `g9-diag: LoadLibraryW "C:\Program Files\Image-Line\FL Studio 21\Plugins\Fruity\Effects\Fruity Balance\Fruity Balance_x64.dll" -> 0x6800000`(= (i)。複数回出てよい: 参照プロジェクトの Balance は複数のインスタンスの可能性〔未確認〕。いずれも同じ戻り値)
- `g9-diag: GetProcAddress(0x6800000, "CreatePlugInstance") -> 0x6abdde0`(= (ii)。0x6800000 + export RVA 0x2bdde0)
- `fle: unsupported instruction at 0x68…`、`fle: … is not supported`、`pruned function`、`onAbort` が **0 行**(出たら §2 の表で API 名を引く)。
- 同じ run の他の `LoadLibraryW "…" -> 0x0` 行(Balance 以外の plugin、VST3 など)は Balance 追加の前後で変わらない(B1/B2 に診断 diff だけを足した対照があれば比較。無ければ『Balance の行以外は 0x0』を確認)。

**(iii) Problems 窓の件数(alpha1 の返信 01:4x で確定。alpha1 の読みと実績に基づく。私は実行していない)**: 窓の header 『N plugin(s) could not be opened』は描画された画素で、wintable(class/可視/矩形の一覧)の文字列には出ない。node の G3 は **op `dump <tag>`** で、その時点の**全ての窓の面**を `FLE_G8_DUMPDIR` に `<tag>-lid<窓 id>-<幅>x<高さ>.bgrx`(BGRX の生の画素。layered でなくても出る)で書き、stdout に `g8: dump …` を出す。alpha1 の `open-problems.g3`(`/home/ubuntu/work/alpha22w/tools/`。`snap open_b` → `dump open_b` → `dump open_c`)が参照プロジェクトを開いて Problems 窓の面を出す。判定: (a) Problems 窓の面(wintable の class `TMissingPluginsForm` と矩形で絞る)を `bgrx2png.py`(同じ場所)で png にして**目で『N plugin(s)』を読む**(N: 7→6)、(b) Balance 追加前(B1/B2)の同じ窓の面との sha256 の差(必要条件)、(c) 自動読み取りが要るなら、header 領域の画素を基準の面と比較。V3 の Chrome では `sdriveg2.mjs` の `windows()` でも読める。
**(iv)(v)**: 他の 6 件の plugin、first idle、音の sha は B1/B2 から不変(node の G3 の既存の列で比較)。

**alpha1 の返信(01:4x)で確認できたこと**: (1) 前提 1 の形(nested.tsv + Balance の行の写しを `PARITY_SEED_TSV` にする)で足りる。`PARITY_SEED_TSV` の行は実行時に volume へ載り、bundle の再 build は要らない(alpha1 の tsv 3 本の実績)。(2) 面の観測は上記の `dump` で可能。(3) 診断 diff はテスト bundle のみに同意。(4) 追加の合格条件: 毎回 `grep -a '^fle:' stderr.txt` が 0 行、`g3-lines.txt` に `fruity: == stop` が無い。**注意(alpha1 の観測、E12c のテスト build)**: 実体が**モジュールでないファイル**(stub)を `…\\Fruity Balance.dll` の行に載せると、`LoadLibraryW` が『holds a file … but it is not a module』の名前つき停止になる(`fle_load.c` の path table に無い file の分岐 2.)。**本 patch は実モジュール(8 つ目)の行 + 実 DLL なので該当しない。stub 化(後の最適化)は、path table の行と実モジュールが揃っていれば volume の file の中身は読まれない〔`volume_file` は存在確認だけ〕読みだが、未確認**。

**危険・未確認**
- FL が volume のファイルの**中身**を読むか(`volume_file` は存在確認だけの読み)。読むなら実 DLL で正しい。
- プラグインの識別に `Data\System\Plugin Databases` の `.nfo`/`.fst` が要るかは未確認(インストーラの展開木には有る)。
- Wrapper 固有の壁(W2 ジョブオブジェクト、W3 FL Cloud Plugins の探索、W4 以降の VST3)は Balance には無い見込み(読み)。**Balance が実際に踏む壁は実行するまで不明**。
- +3.7 MB の seed pack と wasm の増分は P1(起動 10 秒)の予算に効く。動的追加(LoadLibraryW の中で fetch)は 0309 §P1 の別案(未検証)。
- 参照プロジェクトの Balance は **Effects 側**(`Plugins\Fruity\Effects\Fruity Balance`)。Wrapper(Generators)とは別ファイル。

## 4. 自己申告(誤り・逸脱)

1. `extract-dlls.tsv` の最初の名前をフルパスで書いて候補 0 件になった。裸のファイル名に直し、sha256 を照合した。
2. awk の `gsub` の escape が mawk と gawk で違うので、移植できる構成(`split` と連結)に直した。**gawk はこの機械に無く、gawk での実行は未確認**(mawk 1.3.4 のみ)。
3. 以前の要約で「Balance の lift に 95 個の thunk」と書いたのは素の lift(extra-imports なし)の数。製品の行では 44。
4. 本セッションで `bash` という名の tool を誤って 5 回呼び、拒否された(実害なし)。
5. **V2b の前提の誤り(alpha1 の実測で判明)**: 『製品形式の診断入り bundle で node の G3 が回る』と書いた(§3b)が、製品の wasm は診断 env を読まず FS も無いので回らない。TEST build(make-test-tree)が要る。私は `fle_test_env.h` の仕組み(決定 0243)を確認せずに書いた。
6. 上記以外に、PM の木・製品の木への書き込みは `lift/out-git/mt3w-results/0313/` のみ。commit・push・Chrome の起動・重い job は無い。

## 5. 成果物

- `impl/0313-g9a-balance-module.diff`(10 ファイル、224 行)、`impl/apply-0313.sh`(`check`)、`impl/diag-g9-loadlib.diff`(**テスト専用**の診断。製品に載せない)
- `tables/`: `0313-apply-check.log`、`0313-test-hybrid-fresh.log`(51 ok・0 FAIL)、`0313-realmod_table_gen-8modules-example.c`、`balance-unsupported-44-rebased.tsv`、`balance-unsupported-95-plain-lift.tsv`、`reach-unsup-product-rows.txt`、`0313-balance-74-no-row-imports.txt`、`find-hook-points-balance.txt`、`balance-prod-lift.log`
- `tools/`: `thunk-names.py`、`reach-unsup.py`、`reach-imps.py`、`mk-nested-balance-seed.sh`(V2b の volume の種)、`g9a-test-build.sh`(テスト bundle の木・build・改名。build は未実行)

## 6. build の実績(04:24 完走。実測。私は実エンジンを走らせていない)

- **テスト bundle**: main 0635cc3f + `0313-g9a-balance-module.diff`(224 行)+ `diag-g9-loadlib.diff`(テスト専用)。`tools/g9a-test-build.sh build`(`CI_JOBS=1`・`BINARYEN_CORES=1`・nice 19、PM の info 03:38 後)。**03:38:23〜04:24:09 = 46 分 rc=0**(見積り 22〜35 分を超えた: ほぼ全段が冷間で、objects 22 m 49 s・link 20 m 58 s。b2d 64 s、lift 41 s、felift 6 s、extract 2 s)。ログ・段ごとの時間と最大 RSS・SHA256SUMS・manifest: `tables/build/`。
- **extract**: DLL 9 本(Balance 追加)+ seed 222 本、SHA-256 全て一致。prefix `fl-engine-ci-28f6f631`。
- **大きさ(無圧縮、E13 の製品 bundle `fl-engine-ci-0363b050` との差。E13 と同じ main の app/lift/ci かは未確認)**: wasm 150,851,575 B(E13 143,087,706 B、**+7,763,869 B = +5.4%**)、dll flpack 81,162,920 B(E13 77,444,112 B、+3,718,808 B)、seed flpack 10,967,346 B(E13 7,248,243 B、+3,719,103 B)、glue 744,946 B(不変)。**Balance の PE が dll pack(rebase 後)と seed(volume のファイル)の 2 か所に入るので、pack の増分は +7.44 MB**(§1 の『+3.7 MB』は seed だけの値で不足していた)。`.br` の大きさ(wasm 37.5 MB、dll 32.3 MB、seed 6.7 MB)は `BROTLI_QUALITY=1` で作ったため本番(11)と比べられない。P1(起動 10 秒)への影響は CI の bundle で測る。
- **メモリ**: 最大 RSS は wasm-opt(`-Oz`、13 m 40 s)と link の段で **5,340,900 kB(約 5.1 GiB)**、objects は 1,473,228 kB。MemAvailable: build の memguard が段の境界で記録した 32 点の最小は 14,779 MB、私が wasm-opt の最中(04:17)に測った値は 11,104 MB(swap 2.7 GB 使用中の機械)。**観測した範囲では 8 GB を切らなかった**(連続の監視ではない)。
- **hard link**: `w-Oz/obj` の `.o` 159 本のうちリンク数 2 以上は 0(PM も確認)。
- **rename**: 単純な `mv` は不可(`manifest.json` の url が `/assets/<ディレクトリ名>/…` で、`mk-assets.sh` は basename のまま symlink を張るため、ブラウザで 404 になる)。`tools/g9a-test-build.sh rename` は hard link の複製 `fl-engine-test-g9a-28f6f631/` を作り、`manifest.json` だけ新ファイルにして url の prefix を書き換える(manifest の sha256: 書き換え後 `e9ab1097…0d31`、元 `c4a26fa8…f735`)。4 つの url とも新ディレクトリに実在を確認。元のディレクトリも残してある。**§3b の『manifest の URL は basename なので…安全』は node の harness だけの話で、ブラウザでは誤りだった**(自己申告)。
- **trace 文面(V3・V2b が grep する行。診断 diff は挙動を変えない、§3b)**:
  - `g9-diag: LoadLibraryW "C:\Program Files\Image-Line\FL Studio 21\Plugins\Fruity\Effects\Fruity Balance\Fruity Balance_x64.dll" -> 0x6800000`
  - `g9-diag: GetProcAddress(0x6800000, "CreatePlugInstance") -> 0x6abdde0`
  - ブラウザでは `window.__app.engineLog()` の行の先頭に `[err] ` が付く。
  - `fle: unsupported instruction at 0x68…` → `tables/balance-unsupported-44-rebased.tsv` の第 2 列で API 名。
- **これからの観測(私の担当外。結果は alpha1 と gpu1 から)**: V3 = gpu1(Chrome 1 本、機能の確認のみ)、V2b = alpha1(TEST build、node の G3)。この文書の合格条件(i)〜(v)に照らして読む。

## 7. V3 の結果(gpu1 の報告 04:33。**私の測定ではない**。gpu1 の記録を引用し、私は実行していない)

実 Chrome 156 headless、テスト bundle `fl-engine-test-g9a-28f6f631`(wasm 150,851,575 B、sha `dde53a6b…`、診断入り)、page = p0(main d0447902)、参照プロジェクト `serina1-serum2.flp`、04:26:31〜04:30:03。開始前: load 0.33、MemAvailable 13.5 GB、vmstat si/so = 0・0、他の Chrome/重い job なし。run = gpu1 の `logs/rt-g9a`(report.json、engine.log 316 行、problems-pic.png)。

| 条件(§3) | 結果(gpu1 の観測) |
|---|---|
| (i) `LoadLibraryW` → `0x6800000` | **合格**: `[err] g9-diag: LoadLibraryW "C:\Program Files\Image-Line\FL Studio 21\Plugins\Fruity\Effects\Fruity Balance\Fruity Balance_x64.dll" -> 0x6800000`(1 回) |
| (ii) `GetProcAddress("CreatePlugInstance")` → `0x6abdde0` で呼びが戻る | **合格**: `[err] g9-diag: GetProcAddress(0x6800000, "CreatePlugInstance") -> 0x6abdde0`(1 回)。エンジンは終了せず、first idle まで到達(engine-idle、opErrors 空) |
| `fle: unsupported`・onAbort・pruned・not supported の行 | **0 行**(grep 済み)。他の `[err]` は act1-net〔InternetOpenUrlW 2 行〕・filedlg・window table の通常行のみ |
| (iii) Problems 窓の件数 | **『7 plugin(s) could not be opened』**、一覧は Fruity Wrapper / FLEX / Frequency Shifter / Soundgoodizer / Fruity Parametric EQ 2 / Fruity Delay 2 / Fruity Filter の 7 行で、**Fruity Balance (FL) の行が消えた**。E12 の同じ手順は 8 件で Balance を含む。窓の大きさ 691×343 → 691×327(1 行分 16 px 減)。**E13 の同じ scenario の対照は gpu1 が実行中(結果待ち)** |
| (iv) 他の plugin の行 | 残り 7 件の名前は上のとおり(E12 の 8 件から Balance だけが減った) |
| (v) first idle・音 | first idle は到達(値 4.7 s は診断と +7.4 MB が混ざるので**結論に使わない**。音は本 run で見ていない) |

**読み(私の判断)**: G9-a の合格条件のうち、実エンジン(wasm、実 Chrome)で『Balance の実モジュールを `LoadLibraryW` で開き、`CreatePlugInstance` を解決して呼び、戻る』ことが観測された。**§2 の『踏む壁』として予想した未対応 thunk(`ScrollWindow`・`GetClassLongPtrW` を含む 44 個)は、この scenario では 1 つも踏まれなかった**(`fle: unsupported` 0 行)。**限界**: gpu1 の注意のとおり、scenario には FL が Balance のプラグイン UI を開く操作も音声処理(`process`)の呼びも無い。GUI と処理は踏んでいない = 『読み込みと `CreatePlugInstance` の解決まで』の合格であり、G9-b(音が出る)・G9-c(GUI)の根拠にはならない。`CreatePlugInstance` が返した後に FL が Balance を呼ぶ経路(プロジェクトの再生・UI)は未実行。

## 8. G9-b / G9-c の scenario の設計(読み。設計だけ。実装・build・実行は PM の許可後。PM への info 04:34 の写し)

【前提の事実(読み)】参照プロジェクトの Fruity Balance は 1 インスタンス(reference-project.md:26)、オートメーション「Balance - Insert 1 - ボリューム」(同 :134)があるので Insert 1 に載っている。V3 で LoadLibraryW + CreatePlugInstance は通り、命令レベルの未対応は 0(lift で非 thunk の fle_unsupported 0、thunk 44 は主に GUI 系)。
【G9-b 音(0309 の G9-b)】
 ・scenario: 追加の操作は不要の見込み。alpha1 の export-plain.g3(File→Open、Problems 窓を閉じる、Ctrl+R、Start without Split、待つ、vfsdumpdir)を、元の FLP(Balance を含む)の seed(seed-nested-balance.tsv)で、alpha1 の TEST bundle(診断入り)に対して回す。Export の間に FL のミキサー thread が Balance を呼ぶはず(0309 §2.1-4: ミキサー本体 f_d0ece0 → plugin の vtable。ゲストどうしの呼び)。
 ・判定: (1) fle: unsupported / abort が 0 行、(2) WAV が出る(フレーム数が基準と同じ)、(3) sha が変わる(基準 381cdb5a…f979 = Balance なし)。Balance の効果が出ていれば変わるはず(未確認: Insert 1 の Balance が既定値なら無変化の可能性もある)、(4) 同じ入力の Wine で Balance だけを有効にした Export(opt1 の退避の再有効化、mp3wav 版の FLP で MP3 復号の差を除く)と比べ、sha 一致または規格化相関 ≈ 1。
 ・足りない物: (a) opt1 の Wine 側「Balance のみ有効」の Export(PM/opt1 の判断)、(b) Balance が実際に呼ばれた証拠。製品には呼び回数の trace が無い。WAV の差が主証拠で、足りなければ test-only の入口カウンタ(診断 diff 流の 7 行程度)を載せた build をもう 1 回(約 46 分、alpha1 の TEST build なら 12〜17 分)。
 ・予想される壁(読み。当たらないことも多い): 呼びの先は Delphi の DSP コード(Balance は音量とパンだけ)なので import は Sleep/CriticalSection など行ありの物が中心の見込み。ただし直接 call で届く『行あり』の import 91 種のうち、引数の形で名前つき停止し得る物は実行するまで不明(0313 §2)。
【G9-c GUI】
 ・scenario: 参照プロジェクトを開いた後、ミキサーの Insert 1 の Balance のスロットを開く操作(スロットの名前のクリック、または右のメニューから『Edit』)で Balance のプラグイン窓を出す。その窓の面を op dump で取り、Wine の同じ窓の画面と並べて読む。
 ・予想される壁(読み): comctl32 の ImageList_*(22 種。Add/Create/Draw/Read/Write …、Delphi の TImageList)が本番。user32 の MDI 系・ScrollWindow・IsDialogMessage*、ole32 の CreateStreamOnHGlobal ほか、shell32 の SHGetDesktopFolder など(0313 の 44 thunk、tables/balance-unsupported-44-rebased.tsv)。ImageList は GDI の描画(ビットマップ合成)を含み、契約の実装として重い(日数は未見積り)。G9-b の後に、止まる VA から 1 つずつ。
【順序の提案】G9-b(追加の build 不要)→ 結果次第で G9-c。G9-b の最初の 1 本は alpha1 の TEST bundle で node 2.5 分(Export)。PM の許可があれば alpha1 と詰める。返信不要。

## 9. V2b(node の G3)の結果と G9-b の現況(**alpha1 の run の log を私が読んだ物。私の測定ではない**。05:10 時点)

alpha1 の TEST bundle `fl-engine-test-g9a`(wasm `engine.26da53796d0f.wasm`)で `open-problems.g3`(参照プロジェクトを開き `dump open_b`/`dump open_c`)を回した run `runs/v2b-g9a`(04:44、wall 11.7 s)と、対照 `runs/v2b-ctl-e12c`(`fl-test-split`、Balance なしの TEST build、wall 11.8 s)を読んだ。

| 項目 | v2b-g9a(Balance 入り) | v2b-ctl-e12c(対照) |
|---|---|---|
| `g9-diag: LoadLibraryW "…Fruity Balance_x64.dll" -> 0x6800000` | **1 行** | 0 行 |
| `g9-diag: GetProcAddress(0x6800000, "CreatePlugInstance") -> 0x6abdde0` | **1 行** | 0 行 |
| `^fle:` の行 | 3 行(act1-net の `InternetOpenUrlW` 2 行 + `MessageLoop: idle: the host ended the wait (no event came; the session was closed)`) | **同じ 3 行** |
| `fle: unsupported`・abort の原因行 | 0 行(終わりの `SIGABRT` は harness が session を閉じた終わり方で、対照も同じ `fruity: == stop: SIGABRT`) | 同じ |
| Problems 窓の面(dump) | **691×327** | **691×343**(= 1 行 16 px 多い。8 件) |

読み: node の G3 でも V3(Chrome)と同じく、`LoadLibraryW` → `0x6800000`、`CreatePlugInstance` の解決、`fle: unsupported` 0、Problems 窓が 1 行(16 px)小さくなった。窓の面の件数の文字を画像で読んだか(7 か)は alpha1 の報告で確認する。主窓(1920×1080)の面の sha は対照と違う(`e2ab6400…` 対 `e4a8cb14…`)が、bundle が違う(E12c と G9-a)ので原因は未確認(874×573 の窓は同一 `da1d1697…`)。

**G9-b(音)の現況**: alpha1 の最初の Export run `runs/exp-g9b`(04:49〜04:52)は、script が `vfsdump`(native 専用の op。wasm では未知)を使ったため、WAV を回収できず sha が空だった。alpha1 は `export-g9b-v2.g3`(`vfsdumpdir`、`0310-g9b-vfsdumpdir.test-only.diff` を載せた TEST build が要る)を用意済みで、node の Export は gpu1 の計時の窓の後。PM は ci1 の Chrome の Export(04:50〜)について『Balance が Export に効いている形』と書いたが、その数字(sha・フレーム数)は私は見ていない。**G9-b の判定((1) `fle: unsupported` 0、(2) WAV のフレーム数が 5,715,911、(3) sha が `381cdb5a…f979` から変わる、(4) Wine『Balance のみ有効』と比較)は、alpha1 の node の Export と opt1 の Wine の結果が出てから**。入口カウンタ(`impl/diag-g9-count.diff`)は sha が変わらなかった場合の保険。

## 10. G9-c の設計の第 1 稿 — 次に実モジュール化する 1 つの選定(読みと静的検査のみ。build・Chrome・node の実行は 0。PM の宿題 05:15)

**結論**: 次は **Fruity Filter 1 本**(PM の判断 05:2x で確定。10.5)。理由は下記。**Fruity Delay 2 は Filter と同じ費用・同じ壁だが、増分を 1 本ずつ測るため同じ build には足さない(後の別 build)**。Frequency Shifter・PEQ2・Soundgoodizer は ildsp などの別の前提があり後に回す。Fruity Wrapper は VST3/Serum 2 を含むので最後。数字は表(`tables/g9c/`、alpha1 の `0309-plugin-sizes.tsv`・`0309-imports-summary.tsv`、私の fe-lift 1 回〔各 1.1 s・nice 19〕と python の読み)から。

### 10.1 候補の比較(6 種。参照プロジェクトの個数と位置は `reference-project.md:26`、`mt1/tables/flp-threaded.tsv`)

| | Fruity Balance(済) | **Fruity Filter** | Fruity Delay 2 | Frequency Shifter | Parametric EQ 2 | Soundgoodizer |
|---|---|---|---|---|---|---|
| 参照プロジェクトの個数・位置 | 1、insert 1 slot 0 | **1、insert 47 slot 0** | 1、insert 2 slot 3 | 1、**Master(insert 0)slot 4** | 8 | 12 |
| オートメーション(`reference-project.md` §2.7) | 『Balance - Insert 1 - ボリューム』 | **『Filter - Insert 47 - Cutoff frequency』** | なし | 『FreqShift - Master - Frequency / Dry/Wet mix』 | 不明 | 不明 |
| DLL(B) / SizeOfImage | 3,718,656 / 0x3a1000 | 3,780,608 / 0x3b2000 | 3,731,968 / 0x3a5000 | 3,876,864 / 0x3c8000 | 4,036,608 / 0x3f0000 | 6,407,680 / 0x633000 |
| 関数(.pdata)/ lift の C | 11,736 / 35.3 MB | 11,737 / **36.05 MB** | 11,760 / **35.59 MB** | 12,148 | 12,317 | 17,386 |
| 行なし import | 74 | 74(**Balance と同一の集合**) | 74(同一) | 74 | 74 | 107 |
| 製品の行で lift した `fle_unsupported` の thunk | 44 | **44(API の集合が Balance と同一。`tables/g9c/filter-unsupported-44.tsv`)** | 44(同一) | 44(alpha1 の表) | 44(同) | 多い(alpha1 の表) |
| `CreatePlugInstance` から直接 call で届く行あり import / 未対応 thunk | 91 / 2 | **91 / 2(同一の API)** | 91 / 2(同一) | 未測定 | 未測定 | 未測定 |
| フック点(`find-hook-points.py`) | 0x17bc0・0x31d70・0x31d90 | **同じ 3 つ**(`tables/g9c/hook-fruity_filter.txt`) | **同じ 3 つ** | 0x17e90・0x320b0・0x320d0(違う) | 0x17ee0・0x32100・0x32120(違う) | 0x1b7b0・0x389e0・0x38a00(違う) |
| `ildsp` の文字列(DLL 内) | 無し | **無し** | **無し** | 有り | 有り | 有り |
| 追加の DLL | なし | なし | なし | `Engine_x64.dll`(MSVC、263 KB、VEX 692 箇所) | なし | なし |
| mnemonic の集合(lift 済み、644 種) | 基準 | **同一(Balance に無い mnemonic は 0)** | **同一** | 未測定 | 未測定 | 未測定 |

- **Balance との本体の共通(fe-lift の素の出力どうしを、データ参照のリテラルと呼び先名を除いて比べた。`tables/g9c/scan-vs-balance.txt`)**: **Filter は関数の 99.5%・C のバイトの 97.0% が Balance と同じ**(Balance に無い固有: 59 関数・1,057,894 B)。**Delay 2 は 99.3%・98.1%**(固有 91 関数・667,544 B)。FLEngine と同じ本体の割合(B-FLE 方式 B の共有の基準。`tables/g9c/scan-vs-flengine.txt`)は Filter 95.5%・84.6%(固有 566 関数・5.38 MB = 15.4%)、Delay 2 95.4%・86.2%(574 関数・4.76 MB)で、Balance(95.6%・86.5%)と同じ水準。= **共有が入れば、Balance の次の Filter の増分は固有の約 1.1 MB の C 分**(読み)。今は共有なしなので増分は Balance 並み。
- **命令の差**: Filter は scalar SSE の浮動小数が Balance より多い(addss 1,001 対 591、subss 999 対 645、mulss 1,170 対 946、cvtss2sd 3,019 対 2,193、movss 4,276 対 3,158)= 実際のフィルタの係数・IIR の演算。Delay 2 は x87 が多い(fld 389 対 311、fstp 181 対 117)。**どれも既存の mnemonic の集合の中**で、lift の未対応は 0。

### 10.2 選定の理由(Filter)

1. **新しい Win32 の壁が無い見込み**(読み): 行なし import の集合・直接 call で届く import・未対応 thunk・フック点が Balance と同一なので、V3/V2b で Balance が通った道(`LoadLibraryW` → `CreatePlugInstance`)は Filter でも同じ。Frequency Shifter はフック点が違い、`Engine_x64.dll`(別の MSVC 製 DLL。VEX 692 箇所は lift の前提条件。0309 §1)と ildsp が要る。
2. **検証力が Balance より高い**: 参照プロジェクトの insert 47 には**『Filter - Insert 47 - Cutoff frequency』のオートメーション**があり、Export の WAV が**時間変化するカットオフ**を含む。Balance(音量とパン)より浮動小数の演算(scalar SSE の IIR)とホスト→プラグインのパラメータ経路(オートメーション)を強く試せる。Wine の『Filter のみ有効』の Export との比較(G9-b の方式)で、sha 一致または高い相関が出れば、DSP の演算の一致の強い証拠になる。
3. **ildsp を避けられる**: PEQ2(×8)・Soundgoodizer(×12)・Frequency Shifter は DLL に ildsp の文字列があり、ildsp は未 lift(x87 944・AVX-512 23,929 命令、`LoadLibraryW("ildsp")` を NULL 許容の分岐で呼ぶ〔決定 0113〕。0309 §1)。これらの個数が多い(PEQ2 8、SG 12)ので音への寄与は大きいが、**ildsp の扱い(NULL のまま通るか、lift するか)の決定が先**。Filter と Delay 2 はそれに依存しない。
4. **Filter を先に選ぶ**: Delay 2 はオートメーションが無く、出力の検証力が Filter より低い。固有の C は小さい(0.67 MB 対 1.06 MB)が、PM の判断で増分を 1 本ずつ測るため Delay 2 は後の別 build とする。

### 10.3 変更案(Balance と同じ手順。最初は見積りとして書き、10.5 で patch を作って静的検査した。build はしていない)

| 項目 | Filter(次の build) | Delay 2(後の別 build) |
|---|---|---|
| `ci/extract-dlls.tsv` | `Fruity Filter_x64.dll` → `lift/out-plugins/fruity_filter_x64.dll`、sha256 `f4801444224236132b61983b7b81d0d3f89395085a1dc5430653a5edb7273d6c`。裸の名前の候補は NSIS の木(`lift/out-plug1/tree-all-sections.tsv`)で 1 行(`Fruity Free Filter` は別名で当たらない) | `Fruity Delay 2_x64.dll`、sha256 `c4fed60da5034d5c04019a4992563334aa6eb89e2adc211214aaca520ea928fc`、候補 1 行 |
| `gen-direct.sh` の spec | 名前 `fruity_filter_x64.dll`、基底 **0x7000000**(Balance 0x6800000〜0x6ba1000 と Elastique 0xb000000 の間。SizeOfImage 0x3b2000 → 終わり 0x73b2000。wrp1 の Wrapper が 0x7000000 だった先例)、引数 `--pdata --hook-points <Balance と同じ 3 点の file>`、パス末尾 `plugins/fruity/effects/fruity filter/fruity filter_x64.dll` | 基底 **0x7400000**(Filter の終わり 0x73b2000 の上。SizeOfImage 0x3a5000 → 終わり 0x77a5000)、同じ hook file、`…/fruity delay 2/fruity delay 2_x64.dll` |
| フック点の file | **新規なし**: `fruity-balance-hook-points.txt` をそのまま使う(3 RVA が同一)。名前が Balance 専用なので、Filter を足すときに汎用名(例 `fruity-delphi-effect-hook-points.txt`)へ改名して Balance の spec も直すのが筋(1 行) | 同じ file |
| `vfs-seed-idle1.tsv` | 1 行(`…\Plugins\Fruity\Effects\Fruity Filter\Fruity Filter_x64.dll` → `lift/out-idle1/inputs/install/Plugins/Fruity/Effects/Fruity_Filter_x64.dll`、同じ sha) | 1 行 |
| パス表 | `realmod_tables.awk` は無変更で 2 行目が出る | 3 行目 |
| `NEXTRA` | `EXTRA_MODULES="oggio elastique fruity_balance fruity_filter"; NEXTRA=4`(DLL pack は 6+4 = 10 本) | `… fruity_delay2`、`NEXTRA=5`(11 本) |
| **`RM_MAX`(新しく要る)** | **`lift/runtime/fle_realmod.c:17` の `RM_MAX 8u` → `16u`**(Balance が 8 本目で満杯。9 本目から必須)。停止文面 `:44` の『8 real modules』、コメント `fle_realmod.h:24`・`fle_loadimage.h:49`、census の `MAX_IMAGES 9`(`census.c:41`、census harness のみ)を揃える。カーソル handle の領域は 16 本でも 1 MiB 内(stride 0x1000 × 16 + 0x7fc < 0x100000。読み)。`main_two.c` の `MAIN_REAL_MAX 16` は足りる。**runtime の変更。ただし rt の key は Balance の patch だけでも変わる〔`H_WASM` が `lift/harness` を含むため。下の 10.5 の `ci/plan.sh` の実測〕ので、Filter が新たに起こすわけではない** | 10 本目も 16 以内 |
| `test-hybrid.sh` | 5 件(Balance と同形)+ 『spec の数と NEXTRA の整合』 | +5 件 |
| 変更の規模 | **実際に作った(10.5): Balance の上の差分は 17 ファイル・313 行、main に対する合算は 17 ファイル・365 行**(最初の見積り 12 ファイル・+60〜80 行より大きい: フック点の file の改名〔33 行の削除と追加〕、`realmod.rs` のテスト、census と `main_two.c` の上限、コメントの数が増えた) | 後の別 build で +20〜30 行 |
| wasm の増分(読み。Balance の実測 +7.76 MB を C のバイト比で補正) | **約 +7.9 MB**(C 36.05 MB 対 35.34 MB) | 約 +7.7 MB |
| pack の増分(無圧縮) | dll pack・seed とも約 +3.78 MB ずつ(PE の大きさ) | 約 +3.73 MB ずつ |
| 冷間 build | 約 53 分(C が +16% 増えるため。Balance の 46 分に比例、objects と link が支配。読み) | PM の判断(10.5 の 1.)で**同じ build には足さない**。後の別 build。`ci/objkeys.sh` の `m[6-9]` は 10 本目の m10 を含まないので、Delay 2 のときに `m(6\|[7-9]\|1[0-9])` などへ直す必要がある(読み) |

- **S1 の再確認(実測の基準で)**: 基準 = `performance-targets.md:141` の『wasm 対 ネイティブ合計』。E13 の wasm 143,087,706 B ÷ ネイティブ合計 77,442,944 B = 1.848。Balance 込みの test bundle は wasm 150,851,575 B ÷ (77,442,944 + 3,718,656 = 81,161,600 B) = **1.859**。Filter を足すと約 158.7 MB ÷ 84.94 MB = **約 1.87**、さらに Delay 2 で約 166.4 MB ÷ 88.67 MB = 約 1.88(推定)。**2 倍以内を満たす**(1 本ごとに 2 倍の許容枠が 7.5 MB 増え、wasm の増分が約 7.8 MB なので、枠は約 0.3 MB ずつ減る)。**0309 の『共有なしで入れてよいのは Balance 1 つだけ』は、plugin 1 本 14.6 MB の見積り(wasm 12.9 MB + データ)に基づくが、実測は wasm +7.76 MB だった**(見積りは約 1.7〜1.9 倍の過大。理由は未確認: 混成〔direct wasm〕の小ささ、`-Oz`、`7.74 × code` の係数が FLEngine 向けなど)。ただし**この比は wasm だけの比**で、dll pack と seed の +7.4 MB/本(無圧縮)は含まない。S1 の定義(『wasm と、エンジンが読むデータ』。`performance-targets.md:152`)にデータを入れるなら 1 本ごとの増分は約 15 MB で枠(約 7.5 MB)を超える。**どちらで測るかは PM の判断が要る**。brotli の転送量は q11 の bundle(CI)で測る必要がある(私の q1 の数字は比べられない)。
- **検証(G9-b と同じ形)**: V3 = Problems 窓が 7 → 6 件(Delay 2 も足せば 5 件)で Filter の行が消える。診断 diff の `g9-diag` は Balance のパスだけを出すので、Filter 用には『フルパスの LoadLibraryW』の行が出る(パスが `Fruity Filter` のものになるだけで diff は無変更)。`GetProcAddress` の診断は `CreatePlugInstance` の行が Filter でも 1 行増える。V2b は Balance と同じ。G9-b = alpha1 の `export-plain.g3` を元の FLP で回し、sha を Balance 単独の Export と比較して Filter の効果(カットオフのオートメーション)が出ているか、Wine の『Filter のみ有効』の Export(opt1)と比べる。入口カウンタ(`diag-g9-count.diff`)の対象範囲は 0x6800000..0x6c00000 なので、Filter(0x7000000〜)を数えるには範囲を広げる(1 行)。

### 10.4 未確認・危険

- Filter の `CreatePlugInstance` から先の**到達範囲は直接 call の下限のみ**(742 関数、Balance の 749 と同水準)。Delphi の仮想呼びの先は未確認。
- 参照プロジェクトで insert 47 が音を持つか、Filter が効いた Export が基準 WAV と実際に変わるかは未確認(Balance の G9-b と同じ未確認)。
- ildsp に依存する 3 種(Frequency Shifter・PEQ2・Soundgoodizer)の扱いは本稿の範囲外。ildsp が NULL のまま通るかの確認(`LoadLibraryW("ildsp")` の NULL 許容の分岐。決定 0113)は、実モジュール化の前に **PEQ2 を 1 つ読み込む実験**(Balance と同じ手順で足し、NULL 許容の分岐に入るかを `g9-diag` で見る)で分かる。

### 10.5 PM の判断(05:2x)の反映と、変更案の dry-run の結果(**実装した patch を作って静的検査まで。build・cargo test・Chrome は 0**)

**PM の判断**: (1) 次は **Fruity Filter 1 本**。Delay 2 は同じ build に足さない(増分を 1 本ずつ測る。後の別 build)。(2) `RM_MAX 8→16` は承認。ただし Split の A0(決定 0312j: close で容量を縮める)は今回の test build に**混ぜない**(増分の帰属を汚さない)。A0 は G9 のモジュール群を push する CI の build に載せる。(3) build の開始条件: (a) Balance の忠実性の判定(opt1 の Wine 参照 elog15b と alpha1 の比較 script)が出る、または (b) 判定が長引いても ci1 の counted 窓(E14 の first idle)の後で CPU が空いたとき。それまで私は設計と変更案の dry-run まで。build の窓は PM の info(`CI_JOBS=1`・`BINARYEN_CORES=1`・nice 19、メモリ確認つき)。(4) S1 は毎回 3 つを併記(下)。(5) seed の stub 化は別の宿題(今は入れない)。

**patch(`impl/0313f-filter-on-balance.diff`、Balance の diff の上。`impl/0313-g9a+filter-combined.diff` は main に対する合算)**:
- 内容: `ci/extract-dlls.tsv` に Filter(sha256 `f4801444…3d6c`)、`vfs-seed-idle1.tsv` に Filter の 1 行、`gen-direct.sh` に 9 本目の spec(基底 `0x7000000`、パス末尾 `plugins/fruity/effects/fruity filter/fruity filter_x64.dll`)、フック点の file を汎用名 `fruity-delphi-effect-hook-points.txt` に改名(Balance と Filter が同じ 3 RVA。Balance の spec も新名へ)、`build-engine-bundle.sh` を `EXTRA_MODULES="oggio elastique fruity_balance fruity_filter"; NEXTRA=4`(DLL pack は 6+4 = 10 本)、`RM_MAX 8u → 16u`(`fle_realmod.c`。停止文面とコメント〔`fle_realmod.h`・`fle_loadimage.h`〕を揃えた)、census の `MAX_IMAGES 17` と `main_two.c` 2 本の `MAIN_REAL_MAX 32`(『runtime の上限より大きく』の意図を保つ)、`realmod.rs` のテスト(『9 本で停止』を『17 本で停止、`larger than the 16 real modules`』に)、`ci/test-hybrid.sh` に 3 件(Filter の extract 行、seed の行、`NEXTRA` と `EXTRA_MODULES` の名前の数と spec の数〔5+NEXTRA〕と `RM_MAX` の整合)。
- 静的検査(実測): `patch --dry-run -p1` が **main HEAD に対する合算で rc=0**、**main + Balance の diff に対する Filter の差分で rc=0**。当てた木で `bash -n` 6 本 ok、`ci/test-hybrid.sh` **54 ok・FAIL 0**(Balance 込みの 51 + 新規 3)。9 モジュール+パス表 2 行の生成 C と `fle_realmod.c`・`fle_load.c`・`fle_loadimage.c` が `clang`/`emcc -fsyntax-only -Wall -Wextra` で rc=0・警告 0。`realmod.rs` は `rustfmt` で構文として読める(整形の差は他の file にもある既存の差)。**`cargo test`(`realmod` のテスト)は未実行**(build の窓の外で行う)。ログ: `tables/g9c/0313f-test-hybrid.log`。
- **`ci/plan.sh` の stage key(実測、git の無い木で比較。`k_app` の変化は `:(exclude)` が効かない non-git の人工物で CI では出ない見込み)**: main → Balance は extract・lift・rt・link・seed・bundle が変わる(felift・b2d は不変)。**Balance → Filter は上に加えて `k_felift` が変わる**: `H_FELIFT` が `lift/crates` 全体(`lift/crates/fe-lift/tests/realmod.rs` を含む)を hash するため、テストの更新だけで CI の fe-lift の cargo build が冷間になる(local の run-ci-build は段を無条件に走らせるので local では差が出ない。CI での cargo の冷間の所要は未測定)。避けるならテストの更新を別の push に分けるが、`RM_MAX` を上げるとテストが落ちるので同じ push が正しい。
- **S1 の 3 併記(PM の決定 4。数字は実測の test bundle と見積りを分けて書く)**: (i) wasm の全体の比 = **1.859**(Balance 込み: 150,851,575 ÷ 81,161,600)。Filter 後は**見積り**約 1.87。(ii) モジュール別の wasm 比 = Balance 単独 **7,763,869 ÷ 3,718,656 ≒ 2.09 倍**(共有なしの重複込み。PM の計算と一致)。Filter は**見積り** 約 7.9 MB ÷ 3,780,608 ≒ 2.09 倍。(iii) dll pack と seed の増分 = Balance **+7.44 MB 無圧縮**(+3,718,808 + 3,719,103 B)。Filter も**見積り**約 +7.5 MB。q11 の brotli の転送量は CI の bundle でだけ測る(私の q1 の test bundle は比べない)。(ii)(iii) が 2 倍を超えることは隠さない。
- **行なし import の増分(alpha1 の表、`lift/out-git/alpha22-results/tables/0310-g9c-extra-imports-by-plugin.tsv`・`…-summary.tsv`)**: Filter・Delay 2・PEQ2・Frequency Shifter は Balance と同じ 74(増分 0)、Soundgoodizer +33、FLEX +14。私の fe-lift の結果(Filter・Delay 2 の thunk の API 集合が Balance と同一)と一致する。

## 11. comctl32 `ImageList_*`(22 種)の契約の実装案(設計だけ。読みと静的検査のみ。実装・build・実行は 0。PM の宿題 05:3x)

**結論**: 22 種は **5 つの層**に分かれ、新規に要るのは「ImageList の表(host 側の画素配列)」が中心で、描画は既存の raster の blit に乗る。足りない物は (a) HBITMAP の画素を読む入口、(b) 宛先 DC へ surface を描く共通の入口(BitBlt の宛先の解決を再利用)、(c) アルファ合成、(d) 引数が guest の `IStream` の vtable を呼ぶ callback。**規模の目安は新規 C 約 1,200 行(関数 22 + 補助 約 25)+ 既存 file への出口 約 200 行 + テスト 約 600 行**(0210 の見積り 920〜2,300 行と整合)。最初の一手は**状態だけの 7 種(約 200 行)**で、これで次に止まる所が分かる。FL のクラス名・窓の種類での分岐は無く、全部が (b) 外部 API の契約の実装(lift-fidelity)。契約に無い・Learn に書かれない部分は『UNOFFICIAL(Wine 10.0 の `imagelist.c`)』と印を付けて根拠を分ける。

### 11.1 22 種の契約(Learn の `nf-commctrl-imagelist_*` を取得して読んだ。原文の抜粋は `tables/g9c/learn/ImageList_*.core`、取得 script は `fetch.py`)

Learn は**どの `ImageList_*` についても `GetLastError` の値を定めていない**(戻り値の失敗値だけ)。→ 実装は最後のエラーを**変えない**。数値の定数(`ILC_*`・`ILD_*`)は `commctrl.h` の値(Learn の表は名前だけ)。層: **A**=状態だけ、**B**=画素を持つ、**C**=描画、**D**=ストリーム、**E**=ドラッグ。

| 層 | API | シグネチャ(Learn)と戻り値 | 副作用・補足(Learn の記述 / UNOFFICIAL) | Balance の呼び元(元の VA。VCL のラッパー〔読み〕) |
|---|---|---|---|---|
| A | `ImageList_Create` | `HIMAGELIST (int cx, int cy, UINT flags, int cInitial, int cGrow)`。成功=ハンドル、失敗=NULL | 『`ImageList_Destroy` で壊す』。`flags` は `ILC_MASK`(1)・`ILC_COLOR*`(4/8/16/24/32=0x20、`ILC_COLORDDB`=0xFE)ほか。UNOFFICIAL(Wine `imagelist.c` の `ImageListImpl_Initialize`、読み): cx/cy が負、または 0(`ILC_COLORDDB` を除く)は NULL、cInitial 個の領域を確保 | `0x544830`(`CreateImageList`。呼び元 2) |
| A | `ImageList_Destroy` | `BOOL (HIMAGELIST)`。成功=非 0 | 『`himl` は optional』。不正なハンドルは FALSE(UNOFFICIAL: Wine の `ImageList_Destroy` は `is_valid` でなければ FALSE。確認済み) | `0x5447e0`(呼び元 5) |
| A | `ImageList_GetImageCount` | `int (HIMAGELIST)` | 画像数 | `0x544ad0`・`0x5454a0`・`0x546340`(3) |
| A | `ImageList_GetIconSize` | `BOOL (HIMAGELIST, int *cx, int *cy)` | 各画像の幅・高さを書く | `0x544580` |
| A | `ImageList_SetIconSize` | `BOOL (HIMAGELIST, int cx, int cy)` | 『全画像は同じ大きさ』。UNOFFICIAL(Wine の `ImageList_SetIconSize` で確認): 大きさを変えると画像は全部消え、overlay の index も初期化される | `0x5445e0`・`0x544640`・`0x544cb0`・`0x693670`(4) |
| A | `ImageList_GetBkColor` | `COLORREF (HIMAGELIST)` | 背景色。失敗=`CLR_NONE` | `0x544d20`・`0x693670`(2) |
| A | `ImageList_SetBkColor` | `COLORREF (HIMAGELIST, COLORREF)` | 『`CLR_NONE` ならマスクで透過』。戻り=直前の背景色、失敗=`CLR_NONE` | `0x544be0` |
| B | `ImageList_Add` | `int (HIMAGELIST, HBITMAP hbmImage, HBITMAP hbmMask)`。成功=最初の新しい画像の index、失敗=-1 | 『画像数は**ビットマップの幅から**決める』『内部構造へ**コピー**する。`hbmImage`/`hbmMask` は呼び出し側が `DeleteObject`』『マスク無しの list では `hbmMask` は無視』 | `0x544930`(2) |
| B | `ImageList_Remove` | `BOOL (HIMAGELIST, int i)` | 『`i = -1` は全画像を削除。残りの index は詰める(0〜n−1)』 | `0x544b00` |
| B | `ImageList_SetImageCount` | `BOOL (HIMAGELIST, UINT uNewCount)` | 『拡げたら `ImageList_Replace` で画像を入れる。入れない index の描画は予測できない』『縮めたら切り詰めた画像は解放』 | `0x546340`(`SetCount`) |
| B | `ImageList_GetIcon` | `HICON (HIMAGELIST, int i, UINT flags)`。失敗=NULL | 『呼び出し側が `DestroyIcon`』。`flags` は `ImageList_Draw` の `fStyle` と同じ | `0x544a60` |
| C | `ImageList_Draw` | `BOOL (HIMAGELIST, int i, HDC hdcDst, int x, int y, UINT fStyle)` | `ILD_NORMAL`(0): list の背景色で描く(`CLR_NONE` ならマスクで透過)、`ILD_TRANSPARENT`(1): 常にマスクで透過、`ILD_MASK`(0x10): マスクを描く、`ILD_IMAGE`(0x20): マスク無視、`ILD_BLEND25`(2)/`ILD_BLEND50`(4): ハイライト色と混ぜる(マスク有りのみ)、`ILD_SELECTED`/`ILD_FOCUS`、overlay(`fStyle` の 0xF00 の index、`ImageList_SetOverlayImage` で先に登録) | `0x5454a0` |
| C | `ImageList_DrawEx` | `BOOL (HIMAGELIST, int i, HDC, int x, int y, int dx, int dy, COLORREF rgbBk, COLORREF rgbFg, UINT fStyle)` | 『dx = dy = 0 なら画像全体。**引数の妥当性は保証しない**』。`rgbBk`: `CLR_NONE`=透過、`CLR_DEFAULT`=list の背景色。`rgbFg`: `CLR_NONE`=宛先 DC の色と混ぜる、`CLR_DEFAULT`=システムのハイライト色 | `0x544e70`(`DoDraw`) |
| D | `ImageList_Read` | `HIMAGELIST (IStream *pstm)` | Learn は**形式を定めない**。UNOFFICIAL(Wine `imagelist.c:88-100, 2150-2260`): 28 バイトの `ILHEAD`(`usMagic` = 'IL' 0x4C49、`usVersion` 0x101、`cCurImage`、`cMaxImage`、`cGrow`、`cx`、`cy`、`bkcolor`、`flags`、`ovls[4]`)→ 画像の `BITMAPFILEHEADER` + `BITMAPINFOHEADER`(+パレット)+ 画素 → `ILC_MASK` なら同形式のマスク。読みは `pstm->Read` | `0x546110`(`ReadData`) |
| D | `ImageList_Write` | `BOOL (HIMAGELIST, IStream *pstm)` | 同じ形式を `pstm->Write` で書く(Wine `:2905, 2999`) | `0x546410`(`WriteData`) |
| E | `ImageList_BeginDrag` | `BOOL (HIMAGELIST himlTrack, int iTrack, int dxHotspot, int dyHotspot)` | 『ドラッグ用の**一時の image list**を作る』『`WM_MOUSEMOVE` で `DragMove`、終わりは `EndDrag`』 | `0x57d820`(TDragImageList〔読み〕) |
| E | `ImageList_DragEnter` | `BOOL (HWND hwndLock, int x, int y)` | 『座標は窓の左上基準でクライアント領域基準ではない』 | `0x57d9d0` |
| E | `ImageList_DragMove` | `BOOL (int x, int y)` | 同じ座標系 | `0x57da90` |
| E | `ImageList_DragLeave` | `BOOL (HWND hwndLock)` | — | `0x57da50`(呼び元 2) |
| E | `ImageList_DragShowNolock` | `BOOL (BOOL fShow)` | ドラッグ画像の表示/非表示(窓の更新を止めない) | `0x57daf0`・`0x57db10` |
| E | `ImageList_EndDrag` | `void ()` | 『一時の image list は破棄される』 | `0x57db30`(呼び元 2) |
| E | `ImageList_GetDragImage` | `HIMAGELIST (POINT *ppt, POINT *pptHotspot)` | 位置とホットスポットを書く(どちらも NULL 可)。『一時の list は `EndDrag` で破棄』 | `0x57d8d0` |

呼び元の表(実測、`tables/g9c/imagelist-callers-balance.tsv`): **22 種はどれもちょうど 1 つのラッパー関数から呼ばれる**(`ImageList_SetIconSize` は 4、`ImageList_GetImageCount` は 3、`ImageList_GetBkColor`・`ImageList_DragShowNolock` は 2)。ラッパーは 2 つの塊にある: **`0x544580`〜`0x546410`**(`TCustomImageList` の方法 = A・B・C・D の 15 種)と **`0x57d820`〜`0x57db30`**(`TDragImageList` = E の 7 種)。名前は Delphi の VCL のソースの読みで、DLL 内の名前は確認していない。Balance の lift で `CreatePlugInstance` から直接 call で届く範囲に ImageList は**無い**(§2、実測)ので、読み込みの段階では踏まず、**GUI を作る段階**で踏む。

### 11.2 Delphi/VCL が呼ぶ順序(読み。実行していない)

- **TCustomImageList の寿命**(VCL のソースの読みとラッパーの対応): 構築 → `CreateImageList`(`0x544830` → `ImageList_Create`)→ `SetBkColor`(`0x544be0`)→ フォームの DFM を読むとき `ReadData`(`0x546110` → `ImageList_Read`。引数は Delphi の `TStreamAdapter` が guest の `IStream` の vtable を持つ)→ `GetIconSize`/`GetImageCount`(`0x544580`、`0x544ad0`)→ 描画時に `DoDraw`(`0x544e70` → `ImageList_DrawEx`)または `Draw`(`0x5454a0` → `ImageList_Draw`)→ 破棄 `0x5447e0`(`ImageList_Destroy`)。`Add`(`0x544930`)、`GetIcon`(`0x544a60`)、`Remove`(`0x544b00`)、`SetCount`(`0x546340`)、`SetWidth/Height`(`0x5445e0`・`0x544640`・`0x544cb0`)、`WriteData`(`0x546410`)はコードにあるが、**GUI の最初の構築で踏むとは限らない**。
- **TDragImageList(E の 7 種)は利用者のドラッグ操作でだけ呼ばれる**(読み)。リストやツリーの項目のドラッグを Balance の UI が持つか不明。→ 最初の GUI の run では踏まない見込み。
- **未確認(実行しないと分からない)**: Balance の `TImageList` が実際に何個あるか、`Create` の `flags`(`ILC_COLOR32` か `ILC_COLOR24`/`ILC_COLORDDB` か、`ILC_MASK` の有無)、`Read` の stream の `ILHEAD.flags`。→ 最初の GUI の run で `ImageList_Create` の引数を**テスト専用の診断で記録**すれば、必要な層の幅が決まる(診断 diff 流。製品には載せない)。

### 11.3 契約を満たす最小の実装の置き場(既存の GDI/raster の再利用)

**置き場**: 新規 `lift/runtime/fle_imagelist.c` と `fle_imagelist.h`(22 の `fle_imp_ImageList_*`)。表(ハンドルの slot)は `fle_gdi.c` と別。**画素は host 側の配列に持つ**(Windows と Wine は list の画像を GDI のビットマップ + メモリ DC に持つが、本 runtime の DDB は『最初に描くときに遅延して作る』〔`fle_gdi.c` の `ddb_view`〕で、`CreateBitmap` は 1 ビットの形だけ〔`fle_gdi.h:85-92`〕なので、ImageList を GDI のビットマップの上に載せると壁が増える)。Learn は『`ImageList_Add` は内部の構造へコピーする』と書くので、**画素の持ち方は契約の外**。

| 要る物 | 既存の再利用 | 新規 |
|---|---|---|
| list の表 | — | `{cx, cy, flags, cur, max, grow, clrBk, overlay[4], BGRA32 の画素配列(cx×cy×max)、マスク(1 ビット/画素 または アルファ)、画像ごとの 『アルファを持つ』 旗}`。ハンドルは `FLE_GDI_*` と重ならない領域から払い出し、`ImageList_Destroy` で解放 |
| `Add` の源(HBITMAP の画素を読む) | 0209 §7 の『HBITMAP から画素を読む入口が無い』は、その後の `GetBitmapBits`/`SetBitmapBits`/`GetDIBits`(`fle_gdi.c:3567, 3602, 3875`)と、DIB 用の `dib_view`・DDB 用の `ddb_view`(`:1661` の `fle_gdi_memdc_surface` が使う)で**大半が解けている** | **`fle_gdi_bitmap_view(uint64_t hbm, fle_rsurf *out)`**(`fle_gdi_memdc_surface` の『ビットマップのハンドルを直接受ける版』。約 12 行)。DIB は `dib_view`、DDB は `ddb_view`(未描画の DDB は 0 の画素を返す) |
| `Draw` の宛先 | `BitBlt` の宛先の解決: `get_target0`・`dev_rect`・`EACH_CLIP`・`blit_begin`/`blit_end`・`target_wrote`(`fle_gdi.c:2040-2133`。メモリ DC と窓/画面 DC〔クリップ、可視、原点〕の両方) | **`fle_gdi_blit_surface(api, hdcDst, x, y, cx, cy, const fle_rsurf *src, sx, sy, rop, const fle_rsurf *mask)`**(`fle_gdi.c` に約 60〜70 行。`blit_call` の『源を DC でなく surface にした版』) |
| マスクの合成 | `fle_r_maskblt`・`fle_r_blit`(三項 ROP。`SRCAND`/`SRCPAINT`/`NOTSRCAND` は `fle_r_rop3` の表にある〔ラスター演算の三項コード〕) | — |
| アルファ | **無い**(`AlphaBlend` は 74 の一つで行なし。`UpdateLayeredWindow` は `fle_layered.c` が自前) | **`fle_r_alphablend`**(`fle_raster.c` に約 60 行。事前乗算アルファの src-over。**`msimg32!AlphaBlend` も同じ関数で実装できる**〔74 の一つ、VCL の `TBitmap` の描画〕) |
| `GetIcon` の HICON | `fle_imp_CreateIconIndirect`(`fle_gdi.c:1512`)・`GetIconInfo`(`:2981`) | `fle_gdi_make_icon(color rsurf, mask rsurf)`(約 30 行。`CreateIconIndirect` の中身を C の入口に出す) |
| `Read`/`Write` の guest `IStream` | 『ランタイムから guest 関数を呼ぶ』`fle_callback`(決定 0064。`WndProc`・列挙 callback で使用) | `IStream::Read`/`Write`(vtable の slot 3/4)の呼び出し(約 40 行)+ `ILHEAD` と BMP の直列化(約 150 行) |
| 行(rows) | `lift/harness/seeds/*-imports.txt`(`<dll> <API> <関数>`、**行の位置が擬似 VA を決める**ので新しい file は最後に連結: `ship6-imports.txt` + 新 file = `ship7-imports.txt`) | 22 行 + `fle_imagelist.h` の宣言 |

**重要な副作用**: 行は**全モジュール共通の応答表**に入る(`merge_tables_n.py` の検査、0183 §3)。FLEngine も同じ 22 個を import しているので、**FLEngine の lift した C も変わる**(thunk が `fle_unsupported` から `fle_imp_ImageList_*` になる)。FLEngine は今それらを踏まないが、踏めば同じ実装が走る。→ 行を足す push は全段の冷間 build(rt・link・lift)になる。

### 11.4 描画系は既存の BitBlt の経路に乗るか

**乗る**(読み)。理由: (1) マスク有りの list の `ILD_NORMAL`/`ILD_TRANSPARENT` は、Windows も Wine も『マスクを `SRCAND`、画像を `SRCPAINT`』の 2 回の三項 ROP(Wine `imagelist.c:1403, 1433-1434, 1495-1498`)で、これは既存の `fle_r_blit` の ROP3 そのもの。(2) 背景色を使う `ILD_NORMAL`(`CLR_NONE` でない)は先に背景色で塗る → `fle_r_rop_rect`(PatBlt 相当)。(3) 宛先の解決(窓/メモリ DC、クリップ、更新領域の記録)は `blit_call` の物をそのまま使う(新しい経路を作らない)。**乗らない部分**: (a) 32 ビット + アルファの list(VCL の `ILC_COLOR32` は一般的)は『アルファを持つ画像』を `AlphaBlend` の意味で合成する(Wine は `GdiAlphaBlend`、`imagelist.c:1279, 1365-1390`)= `fle_r_alphablend` が新規。(b) `ILD_BLEND25`/`ILD_BLEND50` は Wine がディザのブラシ(`hbrBlend25/50`)と `PatBlt` で作る(`:1450`)= パターンブラシの経路が要る。Learn は『ハイライト色と 25% 混ぜる』としか書かず濃淡の作り方は規定しないので、**最初は名前つき停止**(`ILD_BLEND*`・`ILD_SELECTED`・`ILD_FOCUS`・overlay・`ILD_SCALE`・`ILD_PRESERVEALPHA`)にして、踏んだら決める。(c) `ILD_MASK` は『マスクを描く』= マスクの surface を `SRCCOPY` で描く。

### 11.5 工数の見積り(日ではなく関数数・行数の目安。実装していない読み。0210 の 920〜2,300 行の範囲)

| 段 | 内容 | 関数(guest 向け / 補助) | 行(目安) |
|---|---|---:|---:|
| 1 | 状態だけ: `Create`・`Destroy`・`GetImageCount`・`GetIconSize`・`SetIconSize`・`GetBkColor`・`SetBkColor` + 表の管理 | 7 / 約 6 | 約 200 |
| 2 | 画素: `Add`・`Remove`・`SetImageCount` + `fle_gdi_bitmap_view` | 3 / 約 5 | 約 220 |
| 3 | ストリーム: `Read`・`Write`(`ILHEAD`、BMP の直列化、guest の `IStream` の callback) | 2 / 約 5 | 約 280 |
| 4 | 描画(マスク、背景色、透過): `Draw`・`DrawEx` + `fle_gdi_blit_surface`(`fle_gdi.c`) | 2 / 約 5 | 約 330(うち `fle_gdi.c` 約 70) |
| 5 | アルファ: `fle_r_alphablend`(`fle_raster.c`)+ アルファ付きの描画 + `msimg32!AlphaBlend` の行 | 0(+1 の別 API)/ 約 2 | 約 120 |
| 6 | `GetIcon`(`fle_gdi_make_icon`)と overlay の状態 | 1 / 約 3 | 約 100 |
| 7 | ドラッグ 7 種: 状態 + 窓の面への描画/消去(宛先の窓の画素を保存して戻す)。**最初は名前つき停止でよい**(利用者のドラッグ操作でだけ呼ばれる読み) | 7 / 約 6 | 約 150(状態だけなら約 60) |
| | **合計(1〜6)** | **15 / 約 26** | **約 1,250**(+ 7 を入れて約 1,400) |
| | テスト(`lift/crates/fe-lift/tests/imagelist.rs`、native。API ごとの契約のテスト + Wine 10.0 との差分) | — | 約 600 |
| | 行の file(`ship7-imports.txt`)+ 宣言 | — | 約 30 |

- **順序の提案**: 段 1 を先に入れ(約 200 行、1 回の build の窓で足りる)、GUI の run で次の停止(= 実際に踏む層)と `Create` の引数を見てから段 2 以降を決める。段 1 の『状態だけ』は画素を持たないので、`Add`/`Draw` は**名前つき停止**のまま(止まる場所が変わるだけ、と 0209 §7 が退けた形だが、**今回は run の観測で次の層を決めるために使う**ので意味がある。PM の判断が要る)。
- **工数の不確かさ**: (a) 32 ビットのアルファ(段 5)の要否は Balance の `Create` の `flags` 次第。(b) `Read` の stream が v6 の形(32 ビット)か v5(24 ビット + マスク)かは DLL を作った Delphi の版で決まる(未確認)。(c) `fle_gdi.c` の宛先の解決の再利用(`fle_gdi_blit_surface`)は `blit_call` の内部の構造に依存し、他担当の `fle_gdi.c` の編集と衝突し得る(0215 の注意と同じ)。

### 11.6 方針の点検(lift-fidelity)

- **新しい処理の区分**: 全部が (b) 外部 API の契約の実装。新しい旗・環境変数・クラス名/窓の種類/アドレスによる分岐は無い。
- **Learn に無い挙動**(`Read`/`Write` の形式、大きさ変更で画像が消える、ブレンドの濃淡)は『UNOFFICIAL(Wine 10.0)』と書き、最初は名前つき停止を優先する。
- **Wine との差分テスト**: 0210 が『headless Wine で窓・更新領域・メッセージ列まで比べられる』と書いた差分テストの道を、API ごとの契約のテストに使う(`ImageList_Read` は Wine が生成した stream と本実装が書いた stream の相互読みを含める)。
- **FL 側の補正・代行は無い**: ImageList の画素を FL の描画に合わせて直す処理は書かない。

## 12. GUI を踏む scenario の設計(Balance の窓を開く)— sdriveg2 の JSON と G3 の下書き(設計だけ。**実行 0**、座標は他者の run の窓表と画像からの読み。PM の宿題 05:4x)

**目的**: 現行の G9-a の test bundle(`fl-engine-test-g9a-28f6f631`)で、参照プロジェクトを開いたあと**ミキサーの Insert 1 の最初のスロット(Fruity Balance)をクリックして Balance の窓を開き**、(i) 最初に止まる VA(= 踏んだ API。`balance-unsupported-44-rebased.tsv` で API 名)と、(ii) 止まらなければ窓が出るか、を観測する。ImageList の層(§11)の要否もここで決まる。**新しい build は要らない**(診断 diff 入りの bundle をそのまま使う。Chrome は gpu1 の非計時の 1 本、約 6〜8 分の見込み〔読み〕)。

### 12.1 座標の求め方(窓の面と窓表から)

**使える情報の差(重要)**: sdriveg2 の `windows()`/`describe` はコンポジタの窓の文字(`lid N at (x, y) WxH … class … title …`)で、**面を持つ最上位の窓だけ**を列挙する(実測: gpu1 の `rt-ptv-pv74s-25/report.json` の `describe` は `TFruityLoopsMainForm`〔lid 14〕と `TMissingPluginsForm` の 2 つだけ)。ミキサー(`TFXForm`)・チャンネルラック・プレイリストは**主窓の子**で独立の面を持たないため、`clickIn cls:'TFXForm'` は使えない。→ **絶対座標の `click` を使い、座標はエンジン自身の窓表から求める**。

窓表は alpha1 の node の G3 の stdout(`handle 0x… lid N class "…" title "…" rect (x, y, w x h) … parent 0x… … visible 1 … z N`)にある。**窓の絶対位置 = 親の rect の x, y を最上位まで足す**。alpha1 の run `runs/v2b-g9a`(G9-a の test bundle、プロジェクトを開いて Problems 窓を出した状態。画像 `tables/g9c/gui-ref-main-1920x1080-after-project-open.png`)の窓表から計算した値:

| 窓(lid / class / title) | 親 | 親内の rect | **絶対位置と大きさ(ゲスト画面)** | 備考 |
|---|---|---|---|---|
| 14 `TFruityLoopsMainForm` 『FL Studio 2026』 | — | (0,0,1920×1080) | (0,0) 1920×1080 | 主窓。面を持つ |
| 16 `TWPControl` | 主窓 | (372,82,1548×998) | (372,82) 1548×998 | パネルの置き場 |
| 17 `TStepSeqForm` 『Channel rack』 | lid 16 | (0,0,499×126) | (372,82) 499×126 | z 0 |
| 53 `TEventEditForm` 『Playlist -』 | lid 16 | (499,0,1049×998) | (871,82) 1049×998 | z 1(Mixer より**手前**) |
| **64 `TFXForm` 『Mixer - Master』** | lid 16 | (16,466,1516×500) | **(388,548) 1516×500** | z 2。右側 x ≥ 871 はプレイリストの下 |
| 88 `TVectorPanel`(左端の 3 本: C・Master・Insert 1) | lid 64 | (7,28,132×465) | (395,576) 132×465 | 1 本 44 px: C 395–439、M 439–483、**Insert 1 483–527** |
| 86 `TVectorPanel`(Insert 2 以降のスクロール) | lid 64 | (149,28,1125×465) | (537,576) 1125×465 | |
| 83 `TVectorPanel`(右のスロット欄) | lid 64 | (1289,28,220×465) | **(1677,576) 220×465** | プレイリストの下に隠れている |
| 84 `TVectorPanel`(スロットの一覧) | lid 83 | (0,32,220×191) | **(1677,608) 220×191** | 10 スロット × 約 19.1 px |
| 143 `TMissingPluginsForm`(G9-a の test bundle) | 主窓の owner | — | (614,376) 691×327 | 面を持つ。E13 は 691×343 で (614,368) |

**クリック先の計算**:

| 操作 | 式 | 座標 | 根拠と確度 |
|---|---|---|---|
| Problems 窓を閉じる(OK ボタン) | 窓の中央の x、下から 28 px 上: x = 614+370 = 984、y = 376+327−28 = 675(`clickIn TMissingPluginsForm dx 370 dy 299`) | (984, 675) | gpu1 の既存の scenario は E13 の 691×343 の窓で `(984, 683)` = 窓の相対 (370, 315)。窓が 16 px 低くなった分だけ上へ(Balance の行が消えたため)。**代案**: Return キー(alpha1 の `export-plain.g3` は `tap 13` で閉じている)。ボタンの高さ内かは**未確認** |
| Insert 1 を選ぶ | Insert 1 の帯 483–527 の中央付近、名前の縦書きの位置(画像で『Insert 1』の文字が x≈495、y≈690) | (495, 690) | 画像から読んだ。帯のどこでも選択されるはず(FL の挙動の読み。**未確認**) |
| スロット 0(Balance)を開く | x = 1677 + 73 = 1750(スロット欄の中ほど、名前の位置)、y = 608 + 19.1/2 ≈ 617 | (1750, 617) | 窓表からの計算。**スロット欄はプレイリストの下にあるので、先に Insert 1 のクリックで Mixer が手前に来ている必要がある**(FL が自分で z 順を決める。我々は介入しない)。来ていなければ 4 の `pixel` の検査が落ちる |

### 12.2 手順と、各段で期待する状態・観測する物

| 段 | 操作(JSON の op) | 期待(読み) | 観測する物(どこで) |
|---|---|---|---|
| 0 | `closeWizard`、`openProject`(flp と filesFile は既存 scn のまま)、`waitWindow TMissingPluginsForm`、`describe 00-problems` | Problems 窓が 7 件で出る(G9-a の test bundle) | `describe`(窓の一覧)、`engineLog` に `g9-diag` の 2 行 |
| 1 | `clickIn TMissingPluginsForm dx 370 dy 299`、`waitWindow … visible:false`、`wait 10000`、`savePic 01-project-open`、`describe windows-1` | Problems 窓が消え、主窓にミキサー(Master 選択)が出る | PNG(画素で Mixer の位置を確認)、窓一覧が主窓のみ |
| 2 | `logMark before-insert1`、`click (495,690)`、`wait 2500`、`savePic 02-insert1-selected`(raw)、`pixel`/`expectPixel`(スロット欄がプレイリストと違う色か)、`saveLog log-insert1` | ミキサーが手前に来て『Mixer - Insert 1』になり、スロット欄の最初に『Fruity Balance』が見える | PNG で Mixer のスロット欄とスロット名を**目で確認**(ここで座標を直す)、`expectPixel`、engine log(この段の増分が空か) |
| 3 | `logMark before-slot0`、`click (1750,617)`、`wait 4000`、`savePic 03-after-slot0-click`(raw)、`describe windows-2`、`saveLog log-slot0` | **A**: Balance の窓が出る(新しい最上位または子の窓)。**B**: `fle: unsupported instruction at 0x69…`(止まる。ImageList なら再基底後 `0x691b3e0`〜)。**C**: 何も起きない(クリックの当たり違い) | engine log の `[err] fle:`/`g9-diag`/`onAbort`、`describe`(新しい `cls`)、PNG(新しい窓の画素)、`expectRunning`(エンジンが終了していないか) |
| 4 | `wait 10000`、`savePic 04-slot0-plus-14s`、`saveLog log-slot0-14s`、`expectRunning` | A の場合: 窓が描かれ続ける(`ImageList_Draw` などで止まれば B) | 同上 + 窓の画素の変化 |

**止まった VA の読み方**: `fle: unsupported instruction at 0x68xxxxx|0x69xxxxx` は再基底後(base 0x6800000)。`tables/balance-unsupported-44-rebased.tsv` の第 2 列で API 名を引く。`ImageList_*` の再基底後の thunk は `0x691b3e0`(Create)〜`0x691b530`(SetIconSize)、全 22 個は `tables/g9c/imagelist-callers-balance.tsv`。止まらなければ、`ImageList_Create` の引数(flags・大きさ)を知るためにテスト専用の診断が要る(§11.2。診断 diff は push しない)。

### 12.3 観測する物の一覧

| 観測 | 取り方 | 出る場所 | 見るもの |
|---|---|---|---|
| engine の stderr(`fle:`・`g9-diag:`・abort) | `logMark`/`saveLog`(`window.__app.engineLog()`、行頭 `[err] `) | `OUT/log-*.log` | `fle: unsupported instruction at 0x…`(1 行 = 止まった所)、`fle: … is not supported`、onAbort |
| エンジンの状態 | `expectRunning`/report の `state`・`exit` | report.json | `engine-idle` のままか、`exited`/`error` か |
| 窓の一覧(面を持つ窓) | `describe` | report.json の `describes` | 新しい最上位の窓の `cls`/`title`/位置(Balance の窓が独立の面なら出る。子なら出ない) |
| 窓の画素 | `savePic`(PNG。`raw:true` で RGBA も) | `OUT/*.png` | ミキサーの手前化、スロット欄、開いた窓の中身(目で読む)、段 2 と 3 の差 |
| 窓表(エンジンの全窓、子を含む) | **Chrome では取れない**(コンポジタの文字は最上位だけ)。node の G3 の stdout(`g3-wintable`・`handle …`)で取る | alpha1 の TEST bundle の run | 新しい class(`TPluginForm` 系)の有無と rect。Chrome の run で窓が出たら、node の `impl/g9c-balance-gui.g3` でも同じ手順を回して窓表を得る |
| 特定の画素 | `pixel`/`expectPixel` | report.json の `expects` | 段 2 でスロット欄が見えているか(誤クリック防止) |

### 12.4 scenario の下書き(実行 0)

- **`impl/scn-g9c-balance-gui.json`**: gpu1 の sdriveg2 用(JSON として検証済み、29 op)。op は既存のもの(`closeWizard`・`openProject`・`waitWindow`・`describe`・`clickIn`・`click`・`savePic`・`pixel`・`expectPixel`・`logMark`・`saveLog`・`expectRunning`)だけで、**sdriveg2 に新しい op は足さない**。`listWindows` は最後の 1 つしか残らないので `describe` を使った。
- **`impl/g9c-balance-gui.g3`**: alpha1 の node の G3(TEST bundle)用。`open-problems.g3` の op(`click`・`tap`・`advance`・`snap`・`dump`)だけ。窓表を stdout に得るための道。

### 12.5 危険・未確認

1. **Mixer が手前に来るか**: FL が自分でパネルを前面に出す(`BringWindowToTop` 等)ことに頼る。我々は z 順に介入しない。来なければ段 3 のクリックはプレイリストに当たる(トラック領域のクリックで**プロジェクトを編集してしまう**恐れ)。段 2 の `expectPixel` の失敗で止める運用にし、段 3 の前に PNG を目で確認する(初回は 2 段に分けて走らせる案: 段 0〜2 を先に走らせ、PNG からスロットの座標を確定してから段 3)。
2. **Chrome の幾何が node と同じか**: 座標は node の run の窓表から。Chrome のゲスト画面 1920×1080 と同じ初期配置になる見込み(同じ project・同じ FL の初期配置)だが未確認。差があれば段 2 の PNG で直す。
3. **スロットのクリックの意味**: スロット名のクリックでプラグインの窓が開く(FL の挙動の読み)。LED(左)やミックスのつまみ(右)、右クリックのメニューに当たらない位置を選んだが、窓の幅が 220 でスロット内の部品位置は画像で確認していない。
4. **Balance の窓が最上位かどうか**: 独立の最上位の窓なら Chrome の `describe` に出る。FL の子(パネル)として出るなら出ない → node の G3 の窓表で確認する。
5. **踏む層の想定**: 窓を作る段階で `ImageList_Create`(`TCustomImageList`)の他に、`ScrollWindow`・`GetClassLongPtrW`(直接 call の下限〔§2〕で CreatePlugInstance から届いていた 2 つ)や `FlatSB_*`(行あり)も踏み得る。**止まった物から 1 つずつ契約で実装する**。
6. **Esc/Enter**: Problems 窓の閉じ方は座標かキー。どちらも 1 回の run では確かめていない。
7. **診断の追加は不要で始められる**: 既存の診断(LoadLibraryW/GetProcAddress)と『止まった VA』で最初の 1 手は決まる。`ImageList_Create` の引数の診断は、止まらずに窓が出た場合にだけ必要。

## 13. 段 3 でスロット 0 の左クリックが効かないときの別経路(座標案を 1 ページで。設計だけ、**実行 0**、FL の UI の挙動は読みで未確認)

**前提**: 段 0〜2(プロジェクトを開く、Problems を閉じる、Insert 1 を選ぶ、PNG と `pixel` で Mixer が手前に来たことを確認)は主 scenario と同じ。以下は**1 本の run に 1 経路**(別の経路の状態が次に漏れないよう run を分ける)。`impl/scn-g9c-balance-gui-alt.json` は sdriveg2 の scenario 3 本(段 0〜2 + 各経路。JSON 検証済み、新しい op なし)。座標の根拠は §12.1 と同じ窓表(スロット欄 lid 84 = (1677,608) 220×191、10 行 × 約 19.1 px、スロット 0 の行 = y 608〜627)。**スロット内の部品(左の LED、名前、右の混合つまみ)の x の位置は窓表に無い**(スロット欄は 1 枚の `TVectorPanel` に描かれた絵で、子の窓が無い)ので、**段 2 の PNG(Mixer が手前に来た状態)から目で読む**。以下の x は 220 幅の比率からの仮の値。

| 順 | 経路 | 操作(JSON の route) | 座標(ゲスト画面) | 副作用の危険 | 効いたかの見分け | 次に進む条件 |
|---|---|---|---|---|---|---|
| R1 | 名前の**ダブルクリック** | `click count:2` | (1750, 617) | 名前の編集(リネーム)に入る恐れ。Esc で戻る(Esc の効果は未確認) | `describe` に新しい最上位の窓、または PNG に Balance の窓 | R1 で窓が出ず、PNG に変化なし |
| R2 | 名前の**別の x を 1 回ずつ** | `click` を 2 点 | (1720, 617)、(1820, 617) | **左端(x ≈ 1685〜1695)の LED は有効/無効の切替、右端(x ≈ 1880〜1895)のつまみは混合量**。これらは避ける(Balance を無効にすると Export の音が変わる〔G9-b の比較を汚す〕)。1720 と 1820 はどちらも名前の領域内の見込み(PNG で要確認) | 同上 | 同上 |
| R3 | **右クリックのメニュー**の『Edit』項目(PM の言う『Edit メニュー』) | `click button:right` → `waitWindow TQuickPopupMenuWindow`(soft)→ `savePic menu`・`describe menu-windows` → `clickIn TQuickPopupMenuWindow dx dy` | 右クリックは (1750, 617)。**メニューは最上位の窓なので `describe` に `TQuickPopupMenuWindow` の位置・大きさが出る**(File メニューは既存の scenario が `waitWindow TQuickPopupMenuWindow` で待っている)。項目の `dx,dy` は `menu.png` から読む(項目の高さは未確認。File メニューの『Open...』は (48,85) をクリックしている先例) | 別の項目(削除・置換)に当たる恐れ。**メニューが出たら PNG を PM が見てから項目の座標を決める**(2 段運用)。`dx,dy` は JSON に (0,0) のまま入れてある | メニューの項目名を PNG で確認してから | 3 経路のどれでも出ない |

- **メニューの候補の項目**(FL の挙動の読み。**未確認**): 『Edit』(プラグインの窓を開く)、『Replace』、『Rename』、『Move up/down』、『Delete』。項目名は PNG で確認してから押す。
- **やらない物**: 左端の LED、右端のつまみ、スロット欄の下の空きスロット(クリックでプラグイン選択の窓が開く)、Mixer の他の帯。
- **どの経路でも窓が出ない場合**: クリックが当たっていない(Mixer が手前に来ていない、または座標違い)のか、窓の生成で静かに失敗したのかを `engineLog` の増分(`log-*.log`)と `describe` の差で切り分ける。`fle: unsupported`・abort があれば止まった所が分かる(§12.2)。何も出なければ、PNG の差(段 2 と各経路の後)で FL 側の反応(スロットの強調など)を見る。
- **別経路の限界**: 3 経路とも FL の UI の挙動(クリック、メニュー)の読みに基づく。FL が窓を開く条件(例: プラグインの `EditorShow` に相当する呼びが成功するか)は run で初めて分かる。座標・項目は主 scenario の段 2 の PNG を見たあとで直すのが前提。

## 14. Filter の test build の手順書(開始の直前に流す。**PM の info の後にだけ実行**。この節の `$ …` の行は『実行する手順』で、準備の段階では実行していない物は『(未実行)』と書いた)

**状態(準備で実行済み)**: `impl/apply-0313f.sh check` を main 最新(5b49dcb3、06:08)で実行 → **`patch --dry-run` rc=0(合算 diff)、診断 diff も rc=0、`bash -n` 6 本 ok、`ci/test-hybrid.sh` 全 ok**(`tables/g9c/0313f-apply-check.log`)。`app`・`lift`・`ci` の差は 0635cc3f 以降 0。木 `~/work/mt3w-s1/tree-g9f-build` は `tools/g9f-test-build.sh tree` で作成済み(合算 diff と診断 diff が FAILED 0・offset/fuzz 0)。

### 14.1 手順

| 段 | 実行する物 | 合格・止める条件 |
|---|---|---|
| 0 記録 | `date; uptime; free -m \| sed -n 2,3p; vmstat 1 3 \| tail -2; ps -eo pid,args \| awk '/sdriveg2\|chrome\|firefox\|wine\|clang\|rustc\|wasm-opt\|emcc\|cargo/ && !/awk/'; git -C <repo> log --oneline -1; git -C <repo> diff --stat 0635cc3f HEAD -- app lift ci ':(exclude)lift/out-git' \| tail -1` | load ≤ 3、vmstat の si/so = 0、MemAvailable ≥ 8 GB、他の重い job が無い(ci1 の窓が返却済み)。`diff --stat` が空でなければ段 1 を必ずやり直す |
| 1 dry-run | `$ bash lift/out-git/mt3w-results/0313/impl/apply-0313f.sh check`(約 15 s、`/tmp/opencode/g9f-check` を作って自動で消す) | `dry-run: rc=0`、`bash -n ok` 6 本、`test-hybrid: all checks hold`。落ちたら build しない |
| 2 木 | `$ bash lift/out-git/mt3w-results/0313/tools/g9f-test-build.sh tree`(最新の main から作り直す。『tree from <commit>』を記録) | `FAILED/reject lines … : 0` |
| 3 build | `$ bash …/tools/g9f-test-build.sh build`(**background**、nice 19、`CI_JOBS=1`・`BINARYEN_CORES=1`・`BROTLI_QUALITY=1`。ログ `~/work/mt3w-s1/g9f-build.log`) | `rc=0`。所要は Balance の test build の 46 分(objects 22 m 49 s・link 20 m 58 s が支配)に、Filter 分の生成 C 約 36 MB(FLEngine 約 215 MB と他のモジュールは不変)の追加を足して **約 50〜55 分**(読み。§10.3 の『約 53 分』と同じ見込み) |
| 4 rename | `$ bash …/tools/g9f-test-build.sh rename`(hard link の複製 + `manifest.json` の url の prefix 書き換え。**単純な `mv` は 404**〔§6〕) | `fl-engine-test-g9f-<hash>/` の 4 つの url が実在、2 つの manifest の sha256 を記録 |
| 5 事後 | `find ~/work/mt3w-s1/ci-g9f/engine/w-Oz/obj -name '*.o' -links +1 \| wc -l`(= 0)、`sha256sum -c SHA256SUMS`、`ls -l` で wasm・dll pack・seed の大きさ、`~/work/mt3w-s1/ci-g9f/timing/*.time` の段別所要と最大 RSS | `.o` のリンク数 0、SHA256SUMS ok |

**memguard**: `build-engine-bundle.sh` の `MEMGUARD_MIN_MB`(既定 4096)は `JOBS > 1` のときに JOBS を 1 に落とすだけで、`CI_JOBS=1` では**何も止めない**(`:79-80`)。PM の規則(MemAvailable < 8 GB で止めて info)は**人が見る**: 段の境界で記録される `memguard: <時刻> stage=… MemAvailable=…MB`(ログ内)に加えて、最大 RSS が出る link と wasm-opt(Balance では約 5.1 GiB、約 13〜21 分続く)の最中に `free -m | sed -n 2,3p` を数回見る。8 GB を切ったら build を止めて PM に info(止めるのは自分の job だけ)。

### 14.2 報告に入れる項目(Balance の報告と同じ形)

開始時刻・終了時刻・所要(段別: extract・felift・lift・b2d・objects・link・seed・bundle)・開始時の load / MemAvailable / swap の入出力・最小の MemAvailable(memguard のログと私の観測)・最大 RSS・`.o` のリンク数・SHA256SUMS の可否・manifest の sha256(元とテスト名)・**増分**(Balance の test bundle `fl-engine-test-g9a-28f6f631` との差: wasm 150,851,575 B、dll pack 81,162,920 B、seed 10,967,346 B を基準に Filter 分の無圧縮の増分と、wasm の比 = 増分 ÷ 3,780,608 B)・S1 の 3 併記(PM の決定 4)。**q1 の brotli の大きさは比べない**(本番は q11)。

### 14.3 Filter の V3 で期待する値(gpu1 に渡す。Balance と同じ形)

- `[err] g9-diag: LoadLibraryW "C:\Program Files\Image-Line\FL Studio 21\Plugins\Fruity\Effects\Fruity Filter\Fruity Filter_x64.dll" -> 0x7000000`(1 回)。
- `[err] g9-diag: GetProcAddress(0x7000000, "CreatePlugInstance") -> 0x72cb670`(export RVA 0x2cb670 + base 0x7000000。実測の PE から)。`__dbk_fcall_wrapper` は 0x7018670、`dbkFCallWrapperAddr` は 0x73232b0。image は 0x7000000〜0x73b2000。
- `fle: unsupported instruction at 0x7…` が 0 行。出たら `tables/g9c/filter-unsupported-44.tsv` の再基底後の VA(元 + 0x6c00000 = base 0x7000000 − 0x400000)で API 名。
- Problems 窓の件数: Balance 込みの test bundle の 7 件から **6 件**(Filter の行が消える)。E13 の対照は 8 件。

### 14.4 `realmod` の cargo test を build と一緒に回すかの提案(**重いので勝手に走らせない**。PM の許可後)

- **何を試すか**: `RM_MAX 8→16` で書き換えた `lift/crates/fe-lift/tests/realmod.rs`(『17 本で停止』のテストを含む)。他に `fle_realmod` を使うテスト(`wall_wrp1`・`wall_dec1`・`deferred` ほか 10 前後)は 8 を前提にした文面を持たない(`larger than the 8 real modules` の文字列は `realmod.rs` だけ。grep 済み)が、runtime が変わるので影響の有無は走らせて確かめる。
- **道**: `bash lift/harness/test.sh full --only 'realmod' --jobs 1 --threads 2`(木 `tree-g9f-build` の中。`full` が mingw と clang で合成 PE を作り runtime をリンクする。所要は**未測定**〔test.sh full 全体は S3a で 223 results・704 s〕)。範囲を広げるなら `affected --base-tree ~/work/mt3w-s1/tree-g9-base`(Balance の main 基準の木と比べた差の影響を受ける実行体だけ)。
- **提案**: build の `objects` 段(CI_JOBS=1 で 1 コアだけ使う約 23 分)の間に、**nice 19・`--jobs 1`・`CARGO_BUILD_JOBS=2`** で `--only realmod` だけを 1 回(CPU は空いており、メモリは build の objects が約 1.5 GB なので足りる〔読み〕)。gpu1 の Chrome は非計時なので重なってよい。**ci1 の計時の窓とは重ねない**。全体の `full` は push 前の 1 回(窓の外)に回す案。結果が落ちたらテストの更新の誤りか runtime の副作用かを切り分ける。

### 12.6 追記(gpu1 の窓の間の確認 06:12。**gpu1 の報告の引用。私の測定ではない**)

- scenario 2 本(`scn-g9c-balance-gui.json`、`-alt.json` の R1〜R3)を `sdriveg2.mjs` の op 一覧と突き合わせた: 未知の op は 0、使った引数は全て driver が読む。
- **driver の不具合を gpu1 が見つけて直した**: `pixel` op は窓ごとの 2D canvas(`canvas[data-lid=…]`)を探すが、display method D(1 枚の desktop canvas)にはそれが無く null になり、`expectPixel` が絵に関わらず落ちる。gpu1 が `deskDumpBase64(true)` の RGBA から guest 画面座標の点を読む fallback を足した(`gpu8-results/tools/sdriveg2.mjs`、元は `/tmp/opencode/sdriveg2.pre-pixelfallback.mjs`)。**実機の Chrome ではまだ未実行**(最初の Balance GUI の run が初回)。scenario の変更は不要。
- 座標の突き合わせ(gpu1 の `problems-pic.png`): Problems 窓 691×327・(614,376)、OK の中心は画像で (983,675) = 窓から (+369,+299) → `clickIn dx 370 dy 299` と合う。Insert 1 の縦文字は (495,690) 付近で合う。(1750,617) は現状の画像ではプレイリストの clip の上 = §12.5 の 1. のとおり『Insert 1 のクリックで Mixer が前に来る』ことが前提。
- 実行は窓の後、port 4969・`assets-g9a`・page p0、`tools/pre-run-note.sh` を開始前に。非計時。

### 14.5 realmod の cargo test の結果(実測。06:35〜06:38、build の objects 段の間、nice 19・`--jobs 1`・`CARGO_BUILD_JOBS=2`)

`bash lift/harness/test.sh full --only realmod --jobs 1 --threads 2`(木 `tree-g9f-build`、`FLE_INPUT_ROOT` = PM の checkout〔読むだけ〕): **`fe-lift/realmod` 16 tests passed・failed 0・skipped 0**(『17 本で停止、`larger than the 16 real modules`』の更新後のテストを含む)。`cargo test --no-run` は 128.7 s(215 のテスト実行体を build)、実行体 19.9 s、全体 148.9 s。doc-tests を含め results=5・SKIPPED-lines=0。ログ: `tables/g9c/0313f-realmod-cargo-test.log`。**範囲の限界**: `--only realmod` の 1 実行体だけ。`fle_realmod` を使う他のテスト(`wall_wrp1`・`wall_dec1`・`deferred` ほか)と全体の `full` は**未実行**(push 前に窓の外で 1 回の提案のまま)。

## 15. Balance の編集窓で『枠だけ出て client が描かれない』原因の切り分け(静的な読みと観測・診断の設計。**実行 0**。PM の宿題 06:4x、期限 07:30)

**事実(gpu1 の stage 3、PM が画像で確認)**: 実 Chrome・test bundle `fl-engine-test-g9a-28f6f631` で Insert 1 のスロット名をクリックすると、『Fruity Balance (Insert 1)』の窓の枠が (482,233)〜(713,406) に出るが、client 領域は描かれず(後ろのチャンネルラックが透ける)、engine log の増分は 0 行(`fle: unsupported`・abort なし)、`describe` の窓一覧は主窓のみ(画像: `gpu8-results/logs/rt-g9c-s3/`。私も `03c-10s.png` を読んだ: 枠は歯車・タイトル・最小化・閉じる、中は透明)。以下の『実測』は Balance の lift 済み C(製品の行で lift、`tables/g9c/` に同種)に対する私の静的な解析、『読み』は FL SDK/VCL の知識による推測。

### 15.1 FL が編集窓を作る手順と Balance 側の呼び(静的に読んだ)

1. **FL 側(読み)**: FL が自分の枠(`TNewCaption` の歯車・タイトル・ボタンを持つ形。画像の枠)を作り、プラグイン(`TFruityPlug`)の **`Dispatcher(ID = FPD_ShowEditor = 0, Index, Value = 親の HWND)`** を呼ぶ(FL SDK の `FP_Def` の規約〔読み〕。FL の窓の多くは `TUnpaintedWPForm` 系 = 背景を自分で塗らず子が描く〔`detach-window/form-classes.tsv:57` の継承鎖〕ので、**plugin の editor 窓(子)が無いか見えないと client は塗られず透ける**)。
2. **Balance 側(実測)**: `CreatePlugInstance`(lift VA `0x6bdde0`、再基底後 `0x6abdde0`)は `f_6b8a60` の判定ののち `f_6bd210` を呼び、`[0x6bdbf8]` のクラス参照(= VMT `0x6bdcc0`、再基底後 `0x6abdcc0`。VMT の直後に文字列 `TestPlug` = FL SDK のサンプルの雛形のクラス名)で `f_6be800`(構築)を呼ぶ。**vtable の slot 1 = `0x6bde30`(再基底後 `0x6abde30`)が `Dispatcher`**(`Self`・ID・Index・Value を退避し、まず基底の `f_6b97f0` を呼び、0 でなければ ID で分岐:`ID == 0xb`・`ID − 1 == −1`〔= ID 0〕・1・2・0x15・0x1e… の比較を実測)。vtable は slot 0〜7 = 0x6b92f0・0x6bde30・0x6ba140・0x6be740・0x6bee90・0x6b97d0・0x6beaa0・0x6befc0、slot 8〜19 = 同一のスタブ 0x409ec0(12 個)、slot 20〜25 = 0x6ba150・0x6ba1e0・0x6b9ce0・0x6b9c30・0x6b9cc0・0x6be6e0。
3. **ID 0(ShowEditor)の枝(実測)**: `Self` の `+0xc4` が指す物の `+0x6f0` がエディタのフォーム(`TForm` の物体)。`Value`(R9)が 0 なら `f_670630(form, 0, …)`(隠す側〔読み〕)、0 でなければ **`f_6705f0(form, Value)`**(親窓を設定)→ **`f_579670(form)`**(2 関数・import 0 の小さな物。フォームの `Handle` を返す読み)→ 結果を `Self+0x18`(`EditorHandle`)に保存して返る。`f_6705f0` の直接 call の到達範囲に **`SetParent`** がある(300 関数・import 40)。= **ShowEditor 自体は『フォームの作成』ではなく『親を設定してハンドルを返す』**で、**フォーム自身は `[[Self+0xc4]+0x6f0]` に既にあるはず**(構築時または基底の `f_6b97f0` が作る読み。**未確認**)。窓の実体(`CreateWindowExW`)は Delphi の VCL の仮想呼び(`TWinControl.CreateWnd`)経由で**直接 call の到達範囲に無い**(`CreateWindowExW` は `Dispatcher` の 1,025 関数・123 import の閉包に出ない。実測)。
4. **`Dispatcher` の直接 call の閉包(実測、`tools/g9c-vslots.py`)**: 1,025 関数・import 123 種。GUI 系: `SetParent`・`ShowWindow`・`SetWindowPos`・`SetWindowLongPtrW`・`SendMessageW`・`InvalidateRect`・`BeginPaint`/`EndPaint`・`BitBlt`・`CreateCompatibleDC`・`CreateDIBSection`・`ReleaseDC`・`GetSystemMetrics`・`FindResourceW`/`LoadResource`。**予想した 44 個の未対応 thunk の到達は 0**。他の slot の閉包: slot 4 の `0x6bee90` は 286 関数、slot 6 の `0x6beaa0` は 297 関数、slot 22/23/25 は 56〜75 関数、他は 1〜4 関数。いずれも未対応 thunk は 0。
5. **44 個の thunk と `ImageList_*`(22)との関係**: (i) `ImageList_*` の呼び元は `TCustomImageList` のラッパー(`0x544580`〜`0x546410`)と `TDragImageList`(`0x57d820`〜`0x57db30`)で、**`Dispatcher` からも `CreatePlugInstance` からも直接 call では届かない**(実測)。VCL では DFM の読み込み(`TReader` が `TImageList` を名前で作る)など**仮想呼び**を経由する(読み)。(ii) **未対応 thunk は実行されれば `fle_unsupported` で abort する**(§2)。stage 3 で engine log に abort が無くエンジンが走り続けた = **44 個のうち、この run で実行された物は 0**(確かな推論)。= `ImageList_*`・`ScrollWindow`・`GetClassLongPtrW` は**まだ踏まれていない**。よって『client が描かれない』は ImageList の不足ではなく、その手前。

### 15.2 『client が描かれない』原因の候補(a)〜(d)と、区別する観測

| | 候補 | この候補なら見える物 | どこで見えるか |
|---|---|---|---|
| (a) | FL が Balance の `Dispatcher(FPD_ShowEditor)` を呼んでいない(枠だけ) | Balance の image(再基底後 `0x6800000`〜`0x6c00000`)への間接呼びで `0x6abde30`(Dispatcher)の **ID = 0 の呼びが 0 回**。タイトルの名前は別の slot(`GetName`)から来る | 診断(15.3 の 1)。**窓一覧・画像・engine log からは区別できない**(今の log は 0 行) |
| (b) | `Dispatcher` は呼ばれ、フォームの親付け・可視化が効かない(`CreateWindowExW`・`SetParent`・`ShowWindow`・`SetWindowPos` の契約の不足、または親が可視でない) | `Dispatcher(0, …, Value ≠ 0)` の呼びがある。`SetParent(form, Value)` の引数が不正でない。窓表に新しい窓(class `T…Form`)が **parent = 枠の窓**で出るが、`visible 0`、または rect が 0×0 | 診断の API 列(15.3 の 2)、node の G3 の窓表(`handle … class … parent … visible … rect`。Chrome の `describe` は子を列挙しない) |
| (c) | 描画は実行されるが面に出ない、または可視でない窓への描画が破棄される(決定 0126 の `S-draw-hidden`。`fle_raster.h:91`) | Balance の `BeginPaint`/`EndPaint`/`BitBlt` の呼びがあるのに、`fle_raster_discarded_count()`(`fle_raster.c:612`、**停止時の report でしか出ない**)が増える、または窓の面の非ゼロ画素が増えない | 診断(15.3 の 3:discarded の差を段ごとに印字)、`g3-surface` の非ゼロ画素数(node)、PNG の差 |
| (d) | 作成中に Delphi の例外が起きて**握りつぶされる**(静かに戻る)、または `FindResourceW`/`LoadResource` が NULL を返して `EResNotFound` | `RaiseException` の呼び(コード 0x0EEDFADE など)が Balance から出る。その後 `SetParent` に到達しない。`RaiseException` は `fle_seh_raise` が guest の frame handler を探し、**見つかれば except 節で握りつぶされ、無ければ名前つき停止**(`fle_proc.c:380`)。停止は log に出るので、**log が 0 行 = handler が見つかって握りつぶされた**可能性が残る | 診断の API 列(`RaiseException`・`FindResourceW` の戻り) |

**今ある証拠での絞り込み(読み)**: (a)(b)(c)(d) のどれも否定も肯定もできない。エンジンは走り続け、未対応の thunk は踏まれていない。枠のタイトル『Fruity Balance (Insert 1)』は FL が plugin の名前を得て描いた物で、FL が Balance の vtable のどこかを呼んだ証拠にはなるが ShowEditor の呼びの証拠ではない。

### 15.3 test 専用の診断案(push しない。build は PM の許可後)と、Chrome 1 本で足りる観測の手順

| # | 診断 | 変更量(目安) | 何が分かるか |
|---|---|---|---|
| 1 | **`diag-g9-count.diff` の流用・拡張**: `fle_dispatch_hook` で Balance の image への間接呼びを数える既存案(31 行)に、**最初の N(= 64)回の呼びで対象 VA と `RDX`/`R8`/`R9`(= ID・Index・Value)を印字**し、`0x6abde30`(Dispatcher)の呼びは毎回 ID と Value を出す | +約 10 行(`R[2]`・`R[8]`・`R[9]` を読むだけ) | (a) か否か(Dispatcher の ID 0 の有無、Value = 親の HWND)。他の slot(`Idle_Public`・`GetName`・`Eff_Render` ほか)の呼びの様子 |
| 2 | **GUI 系 API の呼びの列**: Balance の image から呼ばれた時だけ(ゲストの戻り先 `fle_ld(R[4], 8)` が `0x6800000`〜`0x6c00000`)、`CreateWindowExW`(class・親・style・rect)・`SetParent`・`ShowWindow`・`SetWindowPos`・`RegisterClassW`/`ExW`・`SetWindowLongPtrW`・`InvalidateRect`・`UpdateWindow`・`BeginPaint`/`EndPaint`・`BitBlt`・`StretchBlt`・`RaiseException`・`FindResourceW` の名前と引数・戻りを印字(**最初の 200 行まで**) | `fle_win.c`・`fle_gdi.c`・`fle_proc.c`・`fle_res.c` の各 API の先頭に 1 行(共通の補助 `g9_from_balance()` を fle_callback.c に約 15 行)。計 約 40〜60 行 | (b)(d)。thunk の名前は不要(これは runtime が API 名を知っている側で出す) |
| 3 | **raster の discarded の差**: `fle_raster_discarded_count()` と各 surface の非ゼロ画素数を、段(クリック後 1 s・3 s・10 s)ごとに印字(`fle_callback.c` の hook に『N 回目に印字』を足すか、`describe` の呼びで) | +約 10 行 | (c)。Chrome の `describe` では見えない面の違い |

**Chrome 1 本の手順**(gpu1 の `scn-g9c-balance-gui.json` の段 0〜3 を、診断入りの新しい test bundle で。**build が 1 回要る**〔Balance 込みの test bundle の上に診断を足す。局所の再 build は `fle_callback.c`・`fle_win.c` 等の数 file + link + wasm-opt で Balance の 46 分より短い見込み。未測定〕): 段 3 のクリックの直前に `logMark`、直後に `saveLog`(`[err] g9-diag:` の行が `window.__app.engineLog()` に出る〔§3b、V3 の実測〕)。出力を (a)〜(d) の表で読む。**node の G3**(alpha1 の TEST bundle)を並行すると、窓表(子の窓の class・parent・visible・rect)が得られ (b) を直接見られる(`impl/g9c-balance-gui.g3`)。

### 15.4 契約の実装が要りそうな API(Learn と照らす。実装は PM の許可後。『成功を返すだけ』の既定値は入れない)

行は全部あるので**未実装ではなく、契約の網羅を run で確かめる**対象:

| API | Learn の契約(要点) | Balance での使い道(実測の閉包) | 確かめる点(読み) |
|---|---|---|---|
| `SetParent` | 子の親を替える。『`WS_CHILD`/`WS_POPUP` は変えない』『子が可視なら再描画』(`fle_win.c:3711` のコメントは R6 を引き、位置は保つ choice) | `f_6705f0` の閉包(ShowEditor) | FL の枠の窓(新しい親)が窓表の窓か。枠が FL の別の仕組み(自前の合成)の窓で `live_index` に無いと『表に無い親』になる可能性(未確認) |
| `ShowWindow` | 可視化は `WS_VISIBLE`。子は親が可視のときだけ実際に見える(`IsWindowVisible` は祖先の `WS_VISIBLE` を全て要求) | `f_6b97f0` の閉包 | 親(枠)の `visible` と、Balance のフォームの `visible` の連鎖 |
| `SetWindowPos` | 位置・大きさ・z 順・`SWP_SHOWWINDOW` | 同上 | フォームの rect が 0×0 になっていないか |
| `CreateWindowExW`(VCL の `CreateWnd`) | `WS_CHILD` なら親が要る。`CW_USEDEFAULT`。`WM_CREATE` | 仮想呼び(閉包に無い) | 親 HWND の有効性、`WS_VISIBLE` の付与 |
| `BeginPaint`/`EndPaint`/`InvalidateRect`/`UpdateWindow` | `WM_PAINT` と更新領域、`BeginPaint` の DC のクリップ = 更新領域 ∩ 可視領域 | `f_6b97f0` の閉包 | 子の更新領域が親の更新に伝播するか(0210 が『親→子の伝播が無い』を指摘した点〔`0210-api-layer-survey.md:13`〕。現状は未確認) |
| `RaiseException`(+ `fle_seh_raise`) | SEH。ハンドラが無ければ既定動作 | 例外時 | plugin モジュール(FLEngine 以外)の `.pdata`/unwind を `fle_seh.c` が扱うか(FLEngine 用に `FLE_SEH_HANDLERS` を渡している。Balance には渡していない) |
| `FindResourceW`/`LoadResource`/`LockResource`/`SizeofResource` | PE の資源 | DFM の読み込み(RT_RCDATA) | 実モジュールの hModule(`0x6800000`)で動くか(`check_module(api, h, 1)`〔`fle_res.c:247`〕が実モジュールを受けるか確認が要る) |
| `ImageList_*`(22)と `ScrollWindow`・`GetClassLongPtrW`ほか 44 | §11 | **まだ踏まれていない**(15.1 の 5) | 踏んだら(停止が出たら)その時に |

**今は新しい実装を足さない**: どれも run の観測の後に、止まった所(または食い違った契約)から 1 つずつ。診断を先に入れて (a)〜(d) を切り分ける。

### 15.5 未確認

- 15.1 の 3 の『フォームが既に `[[Self+0xc4]+0x6f0]` にある』の作成元(構築時か基底の `f_6b97f0`)。
- FL が `FPD_ShowEditor` を呼ぶ時の引数(Value が FL のどの窓の HWND か)。診断の 1 で分かる。
- FL の枠の窓が窓表の窓か。node の G3 の窓表で見える(`impl/g9c-balance-gui.g3`)。
- 『client が透ける』のは枠の窓の背景を FL が塗らないためか(`TUnpaintedWPForm`)、塗るはずなのに描画が破棄されているためか(c)。

## 16. 音声経路の点検(依頼者の観察 O13。**実エンジンの実行は 0(読みだけ)**。PM の宿題 (A)、期限 07:40)

読んだ物: `app/src/audio/{fle-out.worklet.js, page-audio.ts, ring-layout.ts, soft-sink.ts, worker-hook.ts, install.ts, underrun.ts}`(644 行、全部)、`lift/runtime/{fle_audio_ring.c, fle_audio_transport.c, fle_dsound.c の audio_sync / Play / Lock / Unlock / cursors / mix_chunk / gains 周辺, fle_winmm.c}`、`app/src` 全体の `gain`/`mute`/`volume`/`suspend`/`silence` の grep。方針は『(a) 我々が足した余計な処理』と『(b) 契約の不足』のどちらかだけ(`docs/policies/lift-fidelity.md`)。

### 16.0 訂正(07:3x。決定 0241〔aud1、`docs/decisions/0241-first-beat.md`〕を読んでいなかった私の誤り)

**§16.1 の 3・4 と H1(『Play の順序の競合が 1 拍目の最有力』)は、transport の開始には当たらない**。決定 0241 の実エンジンの実測: **transport の開始(Space)では DirectSound の `Play()` は呼ばれない**(DS バッファは起動時に Stop/Play を 2 回した後ずっと再生中で、FL は無音を書き続ける)。実測の `skipped = 0`・`gapFrames = 0`(PM の最初の仮説『late_frames/skipped で捨てる』も外れ)。私の入れ替え案(`audio-first-beat-reorder-proposal.diff`)が効くのは**装置の最初の `Play`(起動時の 1 回)**だけで、**1 拍目の問題の対策にはならない**。→ **診断 build に同梱するのをやめてよい**(害は無いが理由が無い)。

**0241 が測った本当の機構(要約)**: FL は『再生カーソルが半分を越えたら反対側の半分(4096 frame)を書く』ので、通常は Unlock の時点で領域の先頭が再生カーソルの 83〜92 ms 先にあるが、エンジンは再生カーソルの `lead` = 60 ms 先まで**その時点の中身を先に出力リングへ公開**する。Space の直後は 1 本の Worker で UI とミキサーが協調実行されるため Unlock が 93 ms 間隔から 134〜175 ms に伸び、領域の先頭までの距離が 60 ms を切ると**頭の 171〜1152 frame が古い内容(無音)で公開済み**になる。**F2 = `S-ds-late-write`(`rewrite_unread`、`fle_dsound.c:331-365`)**は公開済みで出力がまだ読んでいない frame を、読み位置 + 256 frame(guard)以降について混ぜ直す。F2 は main に入っている(Chrome 156 の最初の再生で 1 拍目の欠け/消失 10/13 → 0/11)。**F2 の効かない範囲**: (i) Unlock の遅れが約 180 ms を超える Worker の停止(0241 §8: 188/340 ms の 2 件)、(ii) 書き込みが guard 内(`rewrite.lost`)。**根本の原因 = 協調実行のミキサーの応答の遅れ**で、除去は『ミキサーを別 Worker に』= 私の S0+S1/S3a(実スレッド)の領域。

**依頼者の『1 拍目が鳴らない問題が未解決』(O13)への読み(未確認)**: F2 が main にあっても残る機構は (i)(ii) と、依頼者の機械(Windows・Firefox 159?)で Space 直後の停止が F2 の範囲を超えること。**次に見る物**: (1) 依頼者のビーコンの `rewrite`(events/applied/lost)と `skipped`/`gapFrames`/`UR_*`(gpu1 が 0241 §9 で表にした形)、(2) 実スレッド版の bundle(B1/B2)で Space 直後の Unlock の間隔が伸びないか(最初の再生の `workerUnlocks` の `first`/`t` の間隔。0241 の表 3)、(3) 0241 の分類(完全/欠け/消失)で `armCapture(…, true)` を取る。**§16.4 の probe のうち H1 用の項目(`SKIPPED_FRAMES` が再生開始直後に増えるか)は、transport の開始では 0 のはず(0241 の実測)で、増えたら新しい事実**。

### 16.1 結論

1. **音は常にストリーム**: `STATE == PLAYING`(DirectSound の再生中のバッファが 1 つでもある間。再生中のバッファが 0 になった時だけ OPEN に戻る〔`fle_dsound.c:468`、`:645`〕)の間、worklet は**量子(128 frame)ごとに必ず読み**、engine が出していないフレームは 0 を出す(`fle-out.worklet.js:36-87`)。**ページ側に無音ゲート・フェード・音量・mute・suspend は無い**(`app/src` 全体の grep で `GainNode`・`gain`・`mute`・`ctx.suspend`・`visibilitychange` による音の制御は 0 件。`page-audio.ts:159-165` は `AudioWorkletNode` を `ctx.destination` に直結)。runtime の `mix_chunk`(`fle_dsound.c:296-323`)は『0 で初期化 → 再生中のバッファの標本を足す』だけで、無音の判定・間引き・閾値は無い。
2. **『無音で切る』処理は見つからない**。見つかったのは『音を捨てる』処理 3 種(下表の 4・5・8)と、『engine が出していない区間を 0 にする』処理(worklet の timeline、`fle_js_publish` の gap 埋め)。
3. **(訂正前の記述。16.0 を読むこと)1 拍目が落ちる最有力の機構(読み)は、再生開始の順序の競合**(16.3 の H1)。`m_b_play`(`fle_dsound.c:1197-1201`)は **`fle_audio_ring_set_state(PLAYING)` を先に呼び、その後に `audio_sync()` で最初の `lead`(60 ms = 2,646 frame)を渡す**。state が PLAYING になった瞬間から worklet は読み位置 RD を量子ごとに 128 進める(timeline)が、最初の hand-over はまだ無い。hand-over が worklet の次の量子より遅れると、`fle_js_publish`(`fle_audio_transport.c:60-61`)が『出力が既に通り過ぎた frame』を**捨てて**(`skipped`)、**最初のブロック(= 1 拍目の頭)の前の部分が欠ける**。worklet 自身のコメントも『the first block is written `lead` ahead only after the state became PLAYING』(`fle-out.worklet.js:7`)と、起動時の underrun(`UR_STARTUP`)を既知の物として数えている。
4. **削除(順序の入れ替え)案**を `impl/audio-first-beat-reorder-proposal.diff`(`fle_dsound.c` の 2 行の入れ替え。**適用しない**。`patch --dry-run` rc=0、`clang -fsyntax-only` 警告 0)に出した。state を PLAYING にする**前**に最初の `audio_sync()`(hand-over)を済ませる。state が OPEN の間は worklet は RD を動かさないので、hand-over の frame は RD より前に落ちない。DirectSound の契約(Learn: Play の後は play cursor が進み、Play までに書いたデータが聞こえる)に沿う。(b) ではなく **(a) 我々の順序の除去**。

### 16.2 『我々が足した』処理と FL の契約が要る処理の表(ファイル:行)

区分: **契約** = Learn の記述が根拠 / **選択** = ページに無く我々が決めた(`S-…` の名前つき)/ **除去候補** = 1 拍目に効き得る我々の処理。

| # | 場所 | 処理 | 区分 | 1 拍目・音への効き | 案 |
|---|---|---|---|---|---|
| 1 | `fle_dsound.c:283, 843, 1196` | `silent_until_coop`: 協調レベルを設定するまで利得 0 | **契約**(Learn Play: 『no sound will be produced until a cooperative level has been set』) | 通常は `SetCooperativeLevel` が先 | 残す |
| 2 | `fle_dsound.c:1197-1201` | **Play: 先に state = PLAYING、後で最初の hand-over** | **選択(順序)= 除去候補** | **大**(H1) | `audio-first-beat-reorder-proposal.diff`(順序の入れ替え) |
| 3 | `fle_dsound.c:1189-1194` | 装置が idle だった間の write_frames の繰り上げ(`S-ds-idle-gap`) | 選択 | 無し(止まっていた間の出力時計の分を飛ばす) | 残す |
| 4 | `fle_dsound.c:429-437` | `audio_sync`: 外部 consumer で write_frames < consumed のとき、consumed まで飛ばして `late_frames` に数える(frame を**捨てる**) | 選択(timeline) | engine が遅れた区間は元から 0 で再生済み(捨てても聞こえ方は変わらない)。**開始直後の競合では 1 拍目を巻き込み得る**(H2) | timeline の仕組み。順序の入れ替えで開始時の競合は無くなる。他は残す(実機でも遅れた区間は聞こえない) |
| 5 | `fle_audio_transport.c:60-61` | `fle_js_publish`: 出力が既に通り過ぎた frame は書かない(`skipped`、header 語 16) | 選択(timeline) | 同上(H1 の直接の欠落点) | 同上 |
| 6 | `fle_audio_transport.c:62-67` | publish 間の穴を 0 で埋める(`gapFrames`) | 選択 | 古い内容を再生しないための 0。**必要** | 残す |
| 7 | `fle_audio_ring.c:158-160` | リングが満杯なら新しい frame を捨てる(`overruns`)。容量 32,768 frame = 743 ms | 選択(容量) | 通常は起きない | 残す(観測だけ: `overruns`) |
| 8 | `app/src/audio/soft-sink.ts:27-42`、`page-audio.ts:146` | worklet が量子を処理していない間(AudioContext が suspended)、10 ms ごとに RD をページ時計で進める(`GRACE_MS` 100 ms の猶予後)。**その間 engine の frame は上書きされ聞こえない** | 選択(決定 0211: 凍結を避ける) | **AudioContext が suspended の間に再生を始めると、最初の拍が聞こえずに過ぎる**(H3) | 判断が要る(下) |
| 9 | `fle_audio_ring.c:120-139`、`fle_dsound.c:903` | hand-over の先読み `lead` 60 ms(`S-audio-lead`)と、guest に見せる write cursor の先行 15 ms(`S-ds-write-cursor`)を**別の値**にしている | 選択(Learn は『write cursor は play cursor の約 15 ms 先』) | 先読みの 60 ms は late write の `rewrite` で救う(下の 10) | 残す |
| 10 | `fle_audio_ring.c:179`、`fle_audio_transport.c:76-85` | 遅い書き込み(`rewrite`): 読み位置 + 256 frame(5.8 ms)より前は変えられず `lost` に数える | 選択(`S-ds-rewrite-guard`) | 開始直後の Unlock が 256 frame 以内なら失われる(H4) | 数だけ観測 |
| 11 | `fle-out.worklet.js:36-87` | timeline: PLAYING の間は engine が出していなくても RD を 128 ずつ進め、無い frame は 0 | 選択(声カードの時間軸と同じ) | 聞こえ方は実機と同型。ただし開始時の競合と組むと 1 拍目を失う | 残す(順序の入れ替えで足りる) |
| 12 | `fle_audio_ring.c:237-278` | `fle_audio_wait_until`: 仮想時計を実時間に合わせる待ち。engine が遅れていれば待たない | 選択(ペーシング) | 音の欠落は作らない | 残す |
| 13 | `page-audio.ts:148` | `AudioContext({ latencyHint: 'interactive', sampleRate: 44100 })` | 選択 | 出力遅延のみ | 残す |

**FL の契約として必要な処理(残す)**: 1 のほか、DirectSound の `Lock`/`Unlock`/`GetCurrentPosition` の意味(`cursors`、`mix_chunk`、`gains`)、primary buffer の規則(Learn: 『primary buffer は looping で Play』)、`DSBPLAY_LOOPING` の循環、通知(`fire_notifications`)。**波形そのものに触る処理は無い**(`mix_chunk` は利得の掛け算 + 加算のみ、`gains` は SetVolume/SetPan の Learn の式)。

### 16.3 1 拍目の欠落が起きる機構の仮説(読み。実行していない)

| | 仮説 | 機構 | この仮説なら見える物 |
|---|---|---|---|
| **H1** | 開始の競合(最有力) | `m_b_play` が `set_state(PLAYING)` → `audio_sync()` の順。state 変更から hand-over までの間(wasm では最初の `mix_chunk` の 2,646 frame の混合と最初の JIT を含む。数 ms〜数十 ms)に worklet が RD を進め、最初のブロックの頭が `skipped` で捨てられる | ring header の語 16(`SKIPPED_FRAMES`)が再生開始直後に増える。`UR_STARTUP`(語 21)> 0。最初の非零 frame の位置が期待より後ろ |
| H2 | engine が開始直後に遅れる | 最初の hand-over の後も混合が遅れ(初回のコード生成、`fle_audio_wait_until` の `late`)、`audio_sync` が consumed まで飛ばす(`late_frames`) | `late_frames`(`stats`。今は export が無い)、`gapFrames`(語 は worker の `A.gapFrames`)、`UR_STEADY` が開始直後に立つ |
| H3 | 開始時に AudioContext が suspended | 再生開始の gesture と `ctx.resume()` が並行し、resume 完了まで SoftSink が RD を進める(frame は上書き) | `softSinkFrames`(`AudioDetail`)> 0、`ctxState` が再生開始時に `suspended`、`lastResume.ms` |
| H4 | 再生開始直後の Unlock が guard 内 | FL が Play の後に最初のブロックを書く場合、既に hand-over された 0 の区間のうち RD + 256 frame より近い部分は `rewrite` で変えられず `lost` | `rewrite.lost`(`fleAudioRewrite()` の `lost`)、`events` |
| H5 | 先頭が FL 自身の無音(FL の挙動) | 1 拍目に音が無いプロジェクト・量子化・プリロール。**我々の処理ではない** | 実機(Wine)の WAV の先頭と比べる(opt1 の Export)。(この一致は Export 経路〔FL が file に直接書く〕の結果で、DirectSound → worklet の再生経路とは別) |

**H5 の確認**: Export(`vfsdumpdir`)は WAV ファイルへ直接書き、DirectSound/worklet を通らない。`opt1` の Wine と wasm の Export は全体で一致している(PM の判定: 5,715,911 フレーム中 5 フレームだけが ±1 LSB。先頭の局所の比較は私は見ていない)ので、**FL のミキサー自体は音を正しく出す**と読める。依頼者の『1 拍目が鳴らない』は**再生経路(DirectSound → worklet)に限った問題**と読める。

### 16.4 gpu1 が非計時の Chrome で再現する観測点(新しい build は不要。`window.__app.audio` はページ側で読める)

再生を押す**前**に、ページで次の polling を仕込み(例: CDP の evaluate)、Space で再生を始めて 2 秒後に結果を読む。`audio.sab` は SharedArrayBuffer なのでページから ring が直接読める。

```js
// 参考(実行は gpu1。sdriveg2 の ev で流す形)
(() => {
  const A = window.__app.audio, h = A.header, cap = A.opts.capFrames, d = new Float32Array(A.sab, 256, cap * 2);
  const P = window.__probe = { t0: performance.now(), s: [], first: null };
  const tick = () => {
    const st = Atomics.load(h, 2);
    if (st === 2 || P.s.length) P.s.push({ t: +(performance.now() - P.t0).toFixed(1), rd: Atomics.load(h, 0) >>> 0, wr: Atomics.load(h, 1) >>> 0, st,
      skipped: Atomics.load(h, 16), urStartup: Atomics.load(h, 21), warm: Atomics.load(h, 22), qPlaying: Atomics.load(h, 9) });
    if (st === 2 && P.first === null) {   // ring content at the moment the state became PLAYING
      let mx = 0, firstNz = -1; for (let i = 0; i < 4096; i++) { const a = Math.max(Math.abs(d[2 * i]), Math.abs(d[2 * i + 1])); if (a > mx) mx = a; if (firstNz < 0 && a > 0) firstNz = i; }
      P.first = { t: +(performance.now() - P.t0).toFixed(1), maxFirst4096: mx, firstNonZero: firstNz };
    }
    if (P.s.length < 600) setTimeout(tick, 2);
  };
  tick();
})();
```

観測する物と判定:

| 観測 | 取り方 | 読み方 |
|---|---|---|
| ring の最初の 4096 frame の最大値と最初の非零 frame の位置 | `__probe.first`(state が PLAYING になった瞬間。**この時点では hand-over がまだ無い**=最初は 0、のはず) | 少し後(+50 ms)にもう一度同じ走査をして、最初の非零 frame の位置を得る。期待する 1 拍目の位置(project の最初の発音の frame)と比べる |
| `SKIPPED_FRAMES`(語 16)、`UR_STARTUP`(語 21)、`WARM`(語 22) | `__probe.s` の各時刻 | 再生開始から 100 ms 以内に `skipped` > 0 なら H1(ほぼ確定)。0 なら H1 ではない |
| `softSinkFrames`、`ctxState`、`lastResume` | `window.__app.audio.detail()`(再生開始の直前と 2 秒後) | `softSinkFrames` > 0 なら H3 |
| `engineGapFrames`、`engineSkippedFrames`、`engineLateWrites`/`Applied`/`Lost` | `detail()` の同名の欄(1 Hz の worker 報告) | H2・H4 |
| worklet の capture(timeline) | `audio.armCapture(8192, true)`(`page-audio.ts:179`)を再生の前に | `firstFrame` と、先頭の波形(1 拍目の頭が欠けているか)を Wine の Export の先頭と比べる |

### 16.5 判断してほしい点

1. **順序の入れ替え**(`audio-first-beat-reorder-proposal.diff`、2 行)を採るか。H1 の観測(`SKIPPED_FRAMES` > 0 が再生開始直後に出る)で裏付けてからを推奨。裏付け前に入れると、他の仮説(H3 など)を隠す。
2. **SoftSink**(表の 8):『AudioContext が suspended の間も凍結を避ける』という決定 0211 の便宜。契約ではない。『engine が PLAYING と言っているのに worklet が処理していない間は RD を動かさない(transport は止まる)』に戻すと、suspended の間の最初の拍が失われない代わりに、gesture 無しでは song が進まない(決定 0211 が測った凍結)。依頼者の環境(gesture 後に再生)では resume が速いはずで、H3 の観測で影響の大きさを見てから決める。
3. 『無音で切る』処理は**見つからなかった**(上の 1・2)。依頼者の記憶にある処理が app/ や runtime のどこかにあれば、grep の対象語を教えてほしい。

## 17. 診断入りテスト build の準備(PM の許可 07:0x。patch と dry-run まで。**build は PM の許可後**。テスト専用・push しない)

**積み重ねる diff(main の上、順に)**: `impl/0313-g9a+filter-combined.diff`(Balance + Filter。push する物)→ `impl/diag-g9-loadlib.diff`(LoadLibraryW/GetProcAddress の戻り)→ **`impl/diag-g9-gui.diff`**(新規。下)→ `impl/audio-first-beat-reorder-proposal.diff`(音声の順序の入れ替え 2 行。(a) 我々の順序の除去)。**実測**: 4 本とも新しい `git archive HEAD` に順に `patch --batch -p1` で FAILED 0・offset/fuzz 0、`fle_callback.c`・`fle_dsound.c`・`fle_load.c`・`fle_modules.c` は `emcc -fsyntax-only -Wall -Wextra` rc=0・警告 0、`link-web.sh` は `bash -n` ok。

**`diag-g9-gui.diff`(92 行、2 file: `lift/runtime/fle_callback.c` の末尾に約 75 行、`lift/harness/web/fruity/link-web.sh` に `--wrap` の 1 行)**。`diag-g9-count.diff` の上位互換(count を含む)。観察だけ(guest から見えるレジスタ・メモリ・戻り値を書かない。本物の関数をそのまま呼ぶ):
1. **間接呼びの観測**: `fle_dispatch_hook`(決定 0097 の census の hook)で、Balance/Filter の image(`0x6800000`〜`0x7400000`)への間接呼びを、**最初の 64 回は対象 VA と rcx/rdx/r8/r9 つきで**(plug-in の `Dispatcher(Self, ID, Index, Value)` では rdx = ID、r9 = Value = 親窓。ID 0 = ShowEditor)、最初の 16 の異なる対象、2 のべき乗ごとの累計を印字(§15.3 の 1)。
2. **GUI 系 API の列**: `CreateWindowExW`(style・class・位置・大きさ・親)・`DestroyWindow`・`SetParent`・`ShowWindow`・`SetWindowPos`・`MoveWindow`・`RegisterClassW`・`SetWindowLongPtrW`・`InvalidateRect`・`UpdateWindow`・`BeginPaint`・`EndPaint`・`BitBlt`・`StretchBlt`・`RaiseException`・`FindResourceW`・`LoadResource` の 17 個を、**ゲストの戻り先(`fle_ld(R[4], 8)`)が Balance/Filter の image の中にある呼びだけ**、引数と戻り値と『これまでに破棄された描画呼び出しの数』(`fle_raster_discarded_count()`。**単位は画素ではなく呼び出し**〔`fle_raster.c:810` の `discarded_total++`〕)を最大 600 行まで印字(§15.3 の 2・3)。仕組みはリンカの `--wrap`(`link-web.sh` の既存の `--wrap=fprintf` 等と同じ。他の呼び元は見えず、`fle_win.c` 等の他担当の file は一切触らない)。
3. 出力先は stderr(`[err] g9-diag:`)。ブラウザでは `window.__app.engineLog()`(§3b、V3 で実測済み)。node の G3 の TEST bundle でも同じ行が stderr に出る。

**区分**: 診断 = 観測だけ(契約の実装でも余計な処理でもなく、テスト専用)。`audio-first-beat-reorder-proposal.diff` = **(a) 我々が足した順序の除去**(再生開始: 先に state = PLAYING、後で最初の hand-over → 先に hand-over)。直す仕組み = 再生開始の境界(timeline 方式の開始点)。画面・症状ごとの対症ではない。**push しない**(音声の順序の入れ替えを push するのは後の G9 の push)。

**使い方(PM の許可後)**: `tools/g9f-test-build.sh` の `tree` の段で 3 本目・4 本目を足す(`$D` の diag を並べる 2 行)。Chrome 1 本(gpu1 の scenario の段 0〜3、段 3 の前後に `logMark`/`saveLog`)で §15.2 の (a)〜(d) を区別し、同じ run で §16.4 の音声の観測(再生を押す前の polling)も取れる(診断 build は 1 回で足りる)。**実行は 0**。

## 18. 遅延ロードの install tree 配布の設計メモ(依頼者の観察 O10〜O12。**実エンジンの実行は 0(読みだけ)**。PM の宿題 (B)、期限 08:15。実装は PM の許可後)

数字の出典: opt1 の棚卸し `lift/out-git/opt2-results/0320/docs/0320-opt1-resource-inventory.md`(原本の install tree を `lstat` で数えた実測。PM が 27,822 行を確認)と、PM の info(圧縮率の標本)。コードの出典は `app/src/engine/hostfiles.ts`・`app/src/bundle/assetstore.ts`・`lift/runtime/fle_vfs.c`・`fle_load.c`・`lift/harness/web/fruity/link-web.sh`・§3b/§6/§14 の bundle の実測。『読み』は未測定の見積り。区分は **(b) 外部 API の契約の実装**(guest に『インストール先の全ファイルが在る』ように見せる = ファイル系 API の契約を満たす)。FL のクラス名・窓・パスによる分岐は足さない。

### 18.1 対象と現状

- **対象は install tree 全体**: 27,822 ファイル・2,053.3 MiB(Data 785.0 MiB/20,272、Plugins 867.7 MiB/6,414、Shared 205.8 MiB/123、System 128.9 MiB/864、直下 43.4 MiB/14、Artwork 19.7 MiB/80、Help 2.8 MiB/55)、ディレクトリ 1,354、パスの平均 72 B。テーマ(`Artwork\Themes` 39 ファイル)・言語(`System\Languages` 15)・テンプレート(`Data\Templates` 70)は Data/Plugins の外にもあるので**索引は全域**。ファイルの大きさの中央値は約 2.8 KiB、4 KiB 未満が全体の 15,223 個(10.3 MiB)、10 MiB 以上は 12 個(304.9 MiB。Data では 3 個)。VST/VST3/Serum 2 は install tree に無い(別経路)。
- **guest が見る根**は `C:\Program Files\Image-Line\FL Studio 21\`(seed の名前。原本の実フォルダは `FL Studio 2026`)。同じ相対パスで置く。**今の seed** は 225 エントリ・7,205,523 B(brotli 3.93 MB)で、うち 218 ファイルが原本とバイト一致(opt1 が sha256 照合)。install tree に対し 0.78%(ファイル数)・0.26%(バイト)。
- **経路(読み)**: guest の `CreateFileW`/`ReadFile`/`FindFirstFileW` 等は `lift/runtime/fle_vfs.c` の**メモリ上の木**(`fle_node`: 名前・親・属性・`data`/`size`/`cap`・時刻 3 つ・子の配列。`fle_vfs.c:59-72`)に当たる。seed は起動時に `FLE_VFS_SEED` の TSV を読み、host ファイル(`HostFiles` の POSIX 系 syscall = `__syscall_openat`/`fd_pread` 等。`hostfiles.ts:1-14, 56-58`。`-sFILESYSTEM=0`)から**線形メモリの heap へコピー**する。`HostFiles` は `ByteSource.readInto(dst, at)` を**同期**で呼ぶ(`hostfiles.ts:18-23`)。実モジュール(lifted plugin)の `LoadLibraryW` は volume 上に『ファイルが在る』ことだけを確認し(`fle_load.c:204-212 volume_file`)、image は別の `dll.flpack` から読む(§6 の『pack に 2 か所入る』の原因)。
- **制約(実測・設定値)**: 線形メモリは初期 640 MiB・上限 2 GiB(`link-web.sh:23`)。node の G3 の run では使用 1.1〜1.2 GB(alpha1 の `summary.json`)。→ **2.05 GB は heap に載らない**。したがって『seed の延長で全部 heap へコピー』は不可能で、**中身は heap の外(host 側)に置いて必要な範囲だけを読む**設計が要る。`0312j`(大きなファイルの退避)の A1 = 『閉じたファイルの中身を host の OPFS に置く』は、**同じ『node の中身を host 側が持つ』仕組みの書き込み側**なので、一つの記憶の抽象にまとめられる。
- **既存の host 側の部品**: `assetstore.ts` = OPFS の `/fl/cache/assets/<sha256>.<ext>`(内容アドレス、SHA-256 で再検証、Web Locks で他ページの掃除から守る、`navigator.storage.persist()` は要求しない)。R2 の bundle は brotli で**符号化したまま**置き、Worker が `Content-Encoding: br` を付けて返す(`upload-commands.txt`)ので、ブラウザが復号して JS は復号済みの bytes を受ける。

### 18.2 提案する構成(『Lazy Volume』)

**L1 索引(全域、起動時に読む)**: 全ファイルの `{相対パス, 大きさ, 属性, 時刻 3 つ, pack の chunk 列}` を 1 つの `index.bin`(+ ディレクトリ木)に。opt1 の見積り = パス 2.0 MB + 1 件約 12 B で未圧縮 約 2.3 MB。**実測(opt1 の 27,822 行の一覧から python で試算、`tools` 無し・読みのみ)**: パス(改行区切り)2,031,806 B → zlib-9 166,455 B・xz-9 133,516 B・zstd-19 137,422 B。前方一致で符号化(front coding)したパス 374,187 B → xz 114,876 B。ファイルごとの記録(大きさ 8 B・属性・chunk 参照の模型 17 B × 27,822 = 472,974 B)→ xz 61,788 B・zlib 113,725 B。合わせて**約 0.18〜0.25 MB(zlib/xz/zstd。brotli はこの機械に無く未測定だが同程度の見込み)**。時刻(各 4〜8 B)と chunk の offset はこの模型に入っておらず**約 +0.1〜0.15 MB の見込み** → **索引は圧縮して約 0.3〜0.4 MB**。起動時に `fle_node` の骨格(`data = NULL`)を 28,000 個作る: 1 node 約 120 B + 名前 UTF-16(平均 約 60 B)+ 子の配列 → **約 6〜8 MB の heap**(読み)。`FindFirstFileW`/`GetFileAttributesW`/`FindNextFileW`/`GetFileSizeEx` は**索引だけで答えられる**(中身を取らない)。
**L2 中身の裏付け**: node に `backing = {chunk 列, 先頭の offset, 大きさ, sha256}` を持たせ、`data == NULL` の間は『host 側が持つ』とする。`ReadFile`/`SetFilePointerEx` は backing から `pread` で読む。**書き込み**(`CreateFileW` の GENERIC_WRITE、`WriteFile`、`DeleteFile`、`MoveFile`)は node を**写し取って heap(または 0312j の OPFS 退避)の通常の node にする**(copy-on-write の overlay)。install tree の既存ファイルを guest が書き換える契約を満たすため。
**L3 読み出しの経路(同期のまま)**: `fle_vfs.c` の backing の読みは `fd_pread` に出る(既存の `HostFiles` の境界)。新しい `ByteSource`(`ChunkedSource`)が chunk を **OPFS のキャッシュから `FileSystemSyncAccessHandle` で同期に読む**(Worker 内で同期。ヒットなら待ちが無い)。**ミス(未取得)**の chunk は、**I/O 用の別 Worker** が `fetch` → 検証(SHA-256)→ OPFS に書く間、エンジンの Worker は `Atomics.wait` で待つ(`ByteSource.readInto` が同期の契約なので変更が要らない)。取得中は guest の thread が止まる = 実機のディスク待ちと同じ。**GIL は待ちの間は放す**(runtime の他の blocking 呼びと同じ型。実スレッド版のミキサー thread が別の thread の I/O 待ちに巻き込まれないため)。
**L4 先読み(性能のみ。契約には影響しない)**: (i) 起動に必要な物(今の seed の 218 ファイル)は pack に入れず**現行の seed のまま eager**; (ii) ディレクトリを列挙した直後の同じ chunk 群(同じディレクトリのファイルは隣に並べる); (iii) 起動後の idle に、優先度つきの一覧(Templates・Themes・Languages・Packs の Drums 等)を背景で取得(帯域と CPU を制限、再生中は止める — P3 を守るため); (iv) FLP を開く時にページが FLP の参照を見て hint を出す(**hint であって正しさに要らない**)。
**L5 キャッシュ(OPFS)**: `fl/cache/vfs/<sha256>.chk`(chunk 単位、内容アドレス)。`assetstore.ts` と同じ Web Locks・検証・掃除。置き換えは chunk の LRU、起動に必要な chunk は pin。`QuotaExceededError` で冷たい chunk から落とす。**容量**: 2.05 GB の無圧縮が最大(OPFS は圧縮せず保存)。Chrome は origin の quota が大きい(ディスクの大きな割合。読み)が、Firefox・Safari は小さい(未確認)ので、**必ず『ミス → 再取得』で動く**(キャッシュ容量が小さくても正しさは保つ)。

### 18.3 pack の置き方の選択肢(PM の (4)(5))

事実: 中央値 2.8 KiB(ファイル 1 本ごとの HTTP range は 27,822 回 — 往復の遅延が支配して不可)。圧縮率(PM の標本): PCM `.wav` 0.87、`.wv`/`.flac`/`.ogg` 0.98(ほぼ縮まない)、DLL 0.34、`.fst` 0.14。種類別の量(opt1): Data = `.wav` 197.3 MiB・`.fst` 164.5・`.wv` 117.8・`.flp` 83.6・`.flexpack` 83.1・`.mrp` 35.0・`.ogg` 30.6・`.png` 27.6・`.flac` 20.6 …、Plugins = `.dll` 676.9 MiB(78%)・`.tga` 53.7・`.wav` 47.6・`.jpg` 14.3 …

| 案 | 内容 | 長所 | 短所 |
|---|---|---|---|
| **P-A** 1 ファイル 1 range | 原本の木をそのまま置く | 実装が最も単純 | 27,822 回の往復。中央値 2.8 KiB では遅延だけで数十分(読み)。**不採用** |
| **P-B** 最上位ディレクトリごとの 1 pack + range | Data・Plugins・Shared・System… の 5〜6 本の大きな pack、offset で range | 管理が少ない | 小さなファイルが多数でも range の往復が要る。巨大 pack は CDN のキャッシュ・並列に不利 |
| **P-C(推奨)** 固定長 chunk の連結 + chunk 単位の取得 | ファイルを**(種別の組, パス)の順**に並べて連結し、**種別の組ごと**に chunk(小ファイルの流れは 1 MiB、大きなファイルの流れは 4 MiB)へ切る。chunk を**1 つの R2 オブジェクト**(`vfs/<sha256>.chk`、不変、CDN にキャッシュ可)として置く。索引が『ファイル → chunk 列 + offset』を持つ | 1 往復で隣のファイルが付いてくる(同じディレクトリが隣り合う)。chunk が取得・検証・キャッシュ・圧縮の単位。range 不要で CDN に素直。大きなファイル(10 MiB 以上 12 個)は複数 chunk を並列に | 索引が chunk 列を持つ(索引の大きさ 約 +0.05〜0.1 MB)。更新時は chunk 単位で差し替え |
| P-D ディレクトリごとの pack | 1 ディレクトリ = 1 pack | FL のブラウザの歩き方(ディレクトリを列挙して開く)に合う | 巨大ディレクトリ(`Packs\Drums` 1,916 ファイル、`Plugin presets` 8,992 ファイル)や多数の小ディレクトリ(1,354)で大きさがばらつく。P-C をパス順にすれば同じ局所性が得られる |

**圧縮の分け方**: 『縮まない物(`.wv`/`.flac`/`.ogg`/`.jpg`/`.png`、`.wav` は 0.87 で効きが薄い)』と『縮む物(DLL 0.34、`.fst` 0.14、`.tga`/`.flp`/`.flexpack` ほか、テキスト)』を**別の chunk の流れ**に分け、前者は **identity**(取得後に復号の CPU が要らない)、後者は **brotli を掛けて `Content-Encoding: br` のまま置く**(既存の bundle と同じ仕組み。ブラウザが復号)。種別が混ざった chunk は作らない(縮まない物に brotli を掛けても無駄で、復号の CPU を増やすだけ)。**転送量の見積り(読み。PM の率を種類別の量に掛けた概算、実測ではない)**: Data 約 560〜600 MiB、Plugins 約 400 MiB(DLL 230 + 他)、Shared 約 85、System 約 55、他 約 20 → 合計 **約 1.1〜1.2 GB**(無圧縮 2.05 GB の約 55%)。全部を取るとは限らない(必要な物だけ)。

### 18.4 起動時間(P1 ≤ 10 s)への影響

- 起動に増えるのは **索引の取得(圧縮して約 0.3〜0.4 MB。上の試算)と骨格 node の構築(約 28,000 個、数十 ms。読み)** だけ。chunk は起動に要る物だけが seed に残り、**新しい取得は起動の経路に入らない**(今の seed が起動に必要な 218 ファイル)。予算(PM: 前の bundle 比 +100 ms)に入る見込みだが**未測定**。索引が大きすぎる場合は、ディレクトリ単位に分割して初回の列挙で取る(同じ blocking 経路。起動はさらに軽い)。
- **seed の重複の解消**: 今は Balance の PE が `dll.flpack` と seed の 2 か所にある(+7.4 MB/本、§6)。lazy node は『volume に見えるファイル』の裏付けを **`dll.flpack` の同じ bytes に向ける**(sha256 が同じ)ので seed のコピーは不要になる(index のエントリだけ)。→ 本件の『stub 化』の宿題(§10.5 の 5)を含む。
- メモリ: 骨格 約 6〜8 MB(読み)+ 取得中の chunk の一時 buffer(数 MB)。再生中の読み(ストリームの `.wv`/`.wav`)は chunk 単位の逐次読みで、heap に溜めない。

### 18.5 契約の点検(Learn と照らす。API ごと。実装は許可後)

| API | 契約の要点 | lazy node での扱い |
|---|---|---|
| `FindFirstFileW`/`FindNextFileW`/`FindClose` | 列挙の順序は規定されない(NTFS は名前順が普通)。`WIN32_FIND_DATAW` に属性・時刻・大きさ | 索引から。順序は『名前の昇順(大文字小文字を畳む)』に固定(決定論のため。**選択**として書く) |
| `GetFileAttributesW`/`Ex`、`GetFileSizeEx`、`GetFileTime` | 属性・大きさ・時刻 | 索引から(中身を取らない)。時刻は原本の mtime を索引に持つ(原本は Wine の prefix の写しなので、元の mtime は installer の時刻か取得時刻か〔未確認〕) |
| `CreateFileW`(読み) | `OPEN_EXISTING`・共有モード・`FILE_FLAG_*` | 存在は索引で判定。開くだけでは取らない |
| `CreateFileW`(書き)/`WriteFile`/`DeleteFile`/`MoveFileW` | install tree のファイルの変更は guest の私的な複製に | copy-on-write(L2) |
| `ReadFile`/`SetFilePointerEx` | 逐次・ランダムな読み。EOF での 0 バイト | backing の `pread`(L3)。部分読みを返してよい(Learn: 要求より少なく返ることがある) |
| `CreateFileMappingW`/`MapViewOfFile`(`fle_vmap.c:721`) | 読み取りの view | 範囲を guest メモリへ**コピーして**渡す(wasm にページフォルトは無い)。view の大きさだけ一時的に増える(最大の `.flexpack` 40.3 MiB)。**契約は満たす**が、view が大きいと瞬間的にメモリが増える点を記録 |
| `LoadLibraryW`(`fle_load.c`) | フルパスの存在 | index だけで存在を答える(§3b の `volume_file`)。lifted module なら実 image は `dll.flpack` から。非 lifted の DLL は今までどおり名前つきの停止 |
| `GetDiskFreeSpaceW`/`GetVolumeInformationW` | 空き容量・種別・シリアル | 今の仮想ボリュームの値のまま |

### 18.6 Plugins の DLL(182 個・676.9 MiB・78%)の遅延取得と、lifted wasm module のオンデマンド化

- **DLL を『ファイル』として見せる**のは上の L1〜L3 で済む(中身は読まれるまで取らない)。問題は **lift した code を wasm に持つ**こと。今は 1 つの wasm に静的に結合(§3b/§6/§10): Balance 1 つで wasm +7.76 MB(E13 比)・dll pack +3.72 MB・seed +3.72 MB(無圧縮)。Plugins の `*_x64.dll` は 175 個・640.0 MiB(native)。**全部を静的に載せると wasm は数百 MB 増え、S1(wasm ≤ native の 2 倍)の分母の native も 2 倍近くになるが、取得量・起動時の compile が P1 を壊す**(0309 の見積り 9.4 ms/MB の compile 時間が全 wasm に掛かる)。
- **オンデマンド化の道**:
  - **O-1(静的のまま、選ぶ)**: G9 で実際に使う少数(参照プロジェクトの 7 種)だけを静的に載せ、残りは**名前つきの停止**のまま(今の挙動)。S1・P1 の定義が変わらない。plugin を増やす度に build が重くなる。
  - **O-2(side module)**: plugin ごとの wasm を別 module にして、`LoadLibraryW` の中で取得・compile・instantiate。**必要な物**: (1) module が runtime の関数(`fle_imp_*` ほか 約 1,000)とグローバル(`R`・`X`・`FL`・TEB 等)を**エンジンの instance の export から import**する、(2) 間接呼びの表 `fle_dispatch_table`(VA → 関数)へ module の関数を**登録**する API(今は C の静的配列)、(3) 線形メモリ・関数表を共有する(`-sMAIN_MODULE`/`SIDE_MODULE` の PIC 化は『主 module の code +10〜25%』と 0312g が読んだが**未測定**、pthread + JSPI との組み合わせの実績なし)。取得中の待ちは、**I/O Worker が fetch して `WebAssembly.compile` し、`Module` をエンジンの Worker へ渡し、エンジンの Worker は `Atomics.wait` で待って同期の `new WebAssembly.Instance` で組み立てる**(JSPI を使わない道。同期 instantiate は Worker で大きさに制限が無い)。JSPI で `LoadLibraryW` 全体を suspend する道もあるが、**どの guest thread の深い C スタックでも suspend できる保証が無い**(エンジンの JSPI は今 `fle_js_wait` の pacing だけ〔`fle_audio_transport.c:108`〕)ので推奨しない。
  - **O-3(インタプリタ・JIT)**: 検討しない。
- **S1 の定義が変わる点**: 静的結合では『wasm 全体 ÷ native 全体』(現在 1.859)。O-2 では **(i) 核の wasm(FLEngine + 常駐 module)の比**と**(ii) plugin module ごとの比**(Balance 単独 ≒ 2.09、共有なしの重複込み)と**(iii) 起動時の取得量**に分かれ、plugin は『使うときだけ』取得・compile する(P1 にも効く: 起動の wasm が小さくなる)。**PM の判断が要る**: S1 を『起動に取る量』と『module ごと』のどちらで測るか(§10.5 の 3 併記の延長)。B-FLE の共有(方式 B、§10 以前の 0312g〜i)は module 間の重複を減らすので O-2 と併用できる。
- **取得中に FL を止める時**: guest の `LoadLibraryW` は同期の API(実機でもディスク読みと DllMain の実行の間は戻らない)なので、止めて待つ形は契約に合う。長さは plugin wasm 約 1.4 MB(brotli、読み)の取得 + compile(読み: 約 75 ms/MB × 8 MB wasm = 0.6 s)で、ネットが遅いと数秒。**FLP を開く時にページが FLP の plugin 名を見て先に取得を始める hint**(正しさに要らない)で隠す。
- **VST/VST3/Serum 2(Fruity Wrapper の内側)**は install tree に無く別経路(本メモの範囲外)。

### 18.7 段階案と工数の目安(実装は許可後。行は読みの見積り)

| 段 | 内容 | 規模の目安 |
|---|---|---|
| 1 | 索引と chunk の生成ツール(原本の木 → `index.bin` + `vfs/<sha>.chk` + 公開用の一覧。種別の組で分け、brotli は縮む組だけ) | python 約 300 行 |
| 2 | runtime: `fle_node` に backing を足す(`->data` 15 か所を accessor 経由に: 0312j と共通)+ 起動時の索引読み込み + copy-on-write | C 約 400 行 + テスト |
| 3 | host: `ChunkedSource`(OPFS の同期読み、I/O Worker、`Atomics.wait`、検証、LRU、Web Locks)+ `HostFiles` への登録 | TS 約 500 行 + テスト |
| 4 | CI: chunk と索引を R2 へ(`wrangler` の既存の仕組みに追加)+ manifest に索引の sha | 約 150 行 |
| 5 | 先読みの方針(idle の背景取得、再生中の抑制)| 約 150 行 |
| 6 | lifted module のオンデマンド化(O-2)| **別の設計**(link の変更と dispatch 表の動的登録を含む。1,500 行以上の見込み、未設計) |

### 18.8 測定計画(実装の前に測れる物)と PM の判断

測る物: (1) 索引の実際の大きさと brotli 後(opt1 の `0320-install-tree-files.tsv.gz` から python で作れる。実行は私の許可後ではなく読みのみなので、次の宿題で可)、(2) chunk の取得の往復(R2 + Worker + CDN、1 MiB・4 MiB)、(3) OPFS の `FileSystemSyncAccessHandle` の同期読みの速度と、chunk 数 2,000 個の open の費用、(4) `Atomics.wait` で待つ guest thread と GIL の挙動(実スレッド版)、(5) Firefox・Safari の quota、(6) 骨格 node 28,000 個の起動費用と heap。

**PM の判断が要る点**: (a) P-C(chunk を R2 の不変オブジェクト)で進めるか。(b) 待ちは `Atomics.wait` + I/O Worker(JSPI に依存しない)で進めるか。(c) 全 tree を索引で見せ、中身は要求時 + 背景先読みにするか(起動に新しい取得を入れない方針)。(d) S1 の定義(O-2 を選ぶ場合)。(e) install tree の既存ファイルへの書き込みを overlay にする(copy-on-write)か。

## 19. 遅延 install tree の索引・chunk の形式(V0 の生成器 = opt1、V1 の読み手 = mt3 の runtime(C)と host(TS)。**規範**。PM の判断 07:2x を反映。細部は opt1 と直接詰めた)

opt1 の既定(info 07:24)に **3 点の変更と 5 点の追加**を求める。以下を V1 の読み手が実装する形式とする(opt1 の生成器が異なる細部を使うなら、この節を直してから実装する)。

**変更(opt1 の既定との差)**:
1. **chunk の SHA-256 と名前は『復号後(論理)の bytes』のもの**にする(opt1 の既定は『保存する bytes』)。理由: brotli の chunk は R2 に符号化したまま置き Worker が `Content-Encoding: br` を付ける(既存の bundle と同じ)ので、ブラウザの `fetch` が**自動で復号した後の bytes**しか JS は見られない。検証できるのは復号後の bytes の SHA-256(`assetstore.ts` も『manifest の sha256 = 復号後』)。名前 = `vfs/<論理 bytes の SHA-256 hex>.chk`。符号化は流れごとに一定(`coding` 欄)なので、同じ論理 bytes は同じ符号化で 1 つのオブジェクトにまとまる。
2. **ファイルごとの SHA-256 は付けない**(索引が約 0.9 MB 太り縮まない。chunk の SHA-256 で検証は足りる。`dll.flpack` と同じ bytes への裏付けの付け替えはパスで行う)。形式には欄を残す(`flags` の bit0)が、V0 は 0 で出す。
3. **索引は『ディレクトリの記録』も持つ**(空のディレクトリ 19 個を含む。guest は `GetFileAttributesW`/`FindFirstFileW` でディレクトリ自体を見る。決定 0129)。

**追加(読み手が必要とする物)**:
4. **マウント点を索引の頭に文字列で持つ**(`C:\Program Files\Image-Line\FL Studio 21`)。runtime にパスを埋めない。
5. **ファイル名は UTF-8・区切りは `/`・マウント点からの相対**。runtime が UTF-16・`\` に直す。
6. **流れの表の `host_name`**: 各流れを guest の runtime は**host のファイル**として開く(`HostFiles` の POSIX 境界)。名前は `vfs/<流れの名前>`(例 `vfs/small-bin`)。V1 の読み手はこの名前で `open`/`pread` する(論理 bytes の連結としての『1 つの大きなファイル』に見える)。
7. **時刻**: `base_filetime`(FILETIME、100 ns、1601 起点)と、各エントリの `mtime` を『秒単位の差の zigzag varint』で。作成・アクセス時刻は mtime と同じ値とする(生成器は原本の mtime だけを持つ)。
8. **属性**: bit0 = ARCHIVE(0x20)、bit1 = READONLY(0x01)、bit2 = HIDDEN(0x02)、bit3 = SYSTEM(0x04)。DIRECTORY は `kind` から。

**形式(リトルエンディアン。`varint` = LEB128 の無符号。`zz` = zigzag)**

```
index.bin :=
  magic        char[8] = "FLVFSIX1"
  version      u32     = 1
  flags        u32     ; bit0 = file_sha32 present
  small_max    u32     ; informative (the size threshold between the small and the large streams)
  base_ft      u64     ; FILETIME
  n_streams    varint
  n_dirs       varint
  n_files      varint
  mount_len    varint ; mount UTF-8 (backslashes), e.g. "C:\Program Files\Image-Line\FL Studio 21"
  streams[n_streams]:
    name_len   varint ; name UTF-8 (the host file is "vfs/" + name)
    coding     u8     ; 0 = identity, 1 = brotli (the object is stored encoded and served with Content-Encoding: br)
    chunk_size varint ; logical bytes per chunk (the last chunk of the stream is shorter)
    n_chunks   varint
    logical    varint ; total logical bytes of the stream (= the sum of the sizes of its files)
    chunks[n_chunks]: sha256 u8[32] of the LOGICAL (decoded) bytes ; the object name is vfs/<hex>.chk
  entries[n_dirs + n_files] in depth-first NTFS order, a directory BEFORE its children:
    kind       u8     ; 0 = file, 1 = directory
    common     varint ; bytes shared with the previous entry's path
    suffix_len varint ; suffix bytes follow
    mtime_zz   varint ; zz((mtime_ft - base_ft) / 10^7) seconds
    attrs      u8
    -- file only:
    stream     u8     ; index into streams[]
    size       varint
    [flags bit0] file_sha32 u8[32]
```
**ファイルの流れの中の offset は書かない**: 同じ `stream` のファイルを entries の順に並べた時の直前の終わり(連続)。読み手は entries を読みながら流れごとの累計を持つ。論理 bytes の `[off, off + size)` の chunk は `off / chunk_size … (off + size − 1) / chunk_size`。サイズ 0 のファイルは chunk を持たない。
**objects の名前と検証**: `vfs/<SHA-256 hex>.chk`。取得した bytes(`fetch` が復号した後)の SHA-256 が名前と一致することを確かめてから OPFS に置く。
**索引の配布**: `index.<sha12>.br`(brotli q11、`Content-Encoding: br`)を取得し、復号後の `index.bin` の SHA-256 は manifest に書く。
**読み手の上限**: V1 の runtime は n_streams ≤ 16、n_files + n_dirs ≤ 65,536、パス ≤ 259 UTF-16 単位、chunk_size ≤ 8 MiB を前提にする(超えたら名前つきの停止)。

**種を持つ 218 ファイルの扱い**: 索引にも全ファイルを載せる(重複)。**seed が勝つ**(V1 の実装は逆順: harness は list の eager な行を先に置き、その後で索引を読み、`fle_vfs_put_lazy_file` は既にファイルがあるパスに 7 を返して読み飛ばす。結果は同じで、索引の 29,000 件それぞれを seed の 218 件と突き合わせる探索が要らない)。生成器は seed を知らなくてよい。
**書き込み**: lazy node への書き込み・削除・名前の変更は copy-on-write(runtime の仕事。形式には現れない)。


## 20. 遅延 install tree V1(runtime と host)の実装の現状(mt3。**一時停止中**〔PM の 08:0x の判断: P3 が優先〕。実行したのは host の C の自己テストと tsc だけで、**engine(wasm)の実行は 0、ブラウザの実行は 0、vitest の新しいテストは未作成**)

差分(**適用していない・push しない**): `impl/0314v1-lazy-vfs-runtime.diff`(5 ファイル、1,082 行: `lift/runtime/fle_vfs.c`・`fle_vfs.h`、`lift/harness/vfs_lazy.h`〔新規〕、`vfs_seed.h`、`harness/seeds/vfs_lazy_selftest.c`〔新規〕)、`impl/0314v1-lazy-vfs-host.diff`(782 行: `app/src/engine/{vfs-index,chunk-store,chunk-io,chunk-io-worker,chunked-source,lazy-tree}.ts`〔新規〕、`hostfiles.ts`、`app/test/unit/helpers/{lazy-tree-gen,chunk-io-node-worker}.ts`〔新規〕)。HEAD のコピーに `patch --dry-run -p1` rc 0。木: `~/work/mt3w-s1/tree-lazy`。

- **runtime(C)**: `fle_node` に `lazy`(stream+1)と `lazy_off`。runtime は host のファイルに触れず、harness が渡す読み関数 1 本(`fle_vfs_lazy_set_reader`)と stream の長さ(`fle_vfs_lazy_add_stream`)だけを持つ。ReadFile は guest の buffer へ直接読む(volume にコピーしない)。WriteFile・SetEndOfFile・CreateFileMapping(`fle_vfs_file_info`)・host 側の peek(`fle_vfs_peek_file`・`fle_vfs_file_state`)は先に**ファイル全体を一度読んで ordinary にする(copy-on-write)**。CopyFileW は lazy のまま同じ stream の同じ範囲を指す。CREATE_ALWAYS/TRUNCATE は lazy を捨てる。`fle_vfs_put_lazy_file`・`fle_vfs_put_dir` は仮想時計を進めない(索引の件数でゲストの作るファイルの時刻が変わらない)。**読みの失敗は `fle_stop`(名前つきの停止)**。ERROR_READ_FAULT 等への写像は V2 の判断(読み。出典の無い選択を避けた)。
- **harness**: seed list の 1 行 `<mount>\<TAB>@lazy:vfs/index.bin<TAB><index.bin の SHA-256>`(新しい旗・環境変数は無い)。索引 FLVFSIX1(§19)を境界つきで読み、**まず dry で全部検査**(上限 stream 16・entries 65,536・chunk 1 B〜8 MiB・stream 名 `[A-Za-z0-9._-]`・パス 259 UTF-16 単位・ファイル群の合計 = stream の論理長・件数の一致・末尾の余りなし)してから volume に置く。stream の host ファイルは「索引の dir/<stream 名>」を open + pread。
- **host(TS)**: `parseIndex`(stream 表だけ/全 entries)、`FetchChunkStore`(`vfs/<sha>.chk`、復号後の SHA-256 と長さを検証、3 回まで再試行)、`ChunkedSource`(同期の ByteSource、LRU の chunk cache)、`SabChunkClient`+`serveChunkIo`(SharedArrayBuffer の 1 スロット + `Atomics.wait`。I/O Worker は idle の間 Atomics.wait でブロックし、fetch の間は event loop が空く。`Atomics.waitAsync` は使わない〔Firefox〕)、`HostFiles.addPersistent`(first idle の `release()` を生き延びる stream。source の例外は EIO)、`mountLazyTree`。**engine の worker.ts/fruity.ts への配線は未**(V2)。
- **実測(host の道具)**: C の自己テスト `vfs_lazy_selftest.c`(gcc `-fsanitize=address,undefined`、`-Wall -Wextra` 警告 0): **58 ok・FAIL 0**(索引の全ての prefix が拒否される、1 byte ずつ反転しても読み手が落ちない、COW・Copy・SetEndOfFile・Move・Delete・peek・時刻・属性・seed が勝つ、など)。既存の `vfs_seed_selftest` も 0 failure。`clang -Wall -Wextra` 警告 0。TS: tsc(tsconfig・webworker)rc 0。
- **vitest(再開後、実測)**: `app/test/unit/lazy-tree.test.ts` **20 tests passed**(3.5 s)。索引 parser(全 prefix の拒否、1 byte 反転で VfsIndexError 以外が出ない、上限、flags bit0)、FetchChunkStore(再試行、hash/長さの検証)、ChunkedSource と LRU、**SAB の client/server を実 worker_threads で**(`Atomics.wait` を両側で、chunk の再訪、store の拒否からの復帰、I/O Worker 不在の timeout、長さの違反)、HostFiles(pread、`release()` 後も stream は読める・index は消える、source の例外は EIO)、**opt1 のテスト木(`testtree-out/`、21 ファイル + 日本語名 + 6 MiB の wav + 0 byte)を全ファイル byte 一致**、**opt1 の実物の索引(`run-a/index.9fde21c39f5d.br`、27,822 ファイル・1,522 dir・16 stream)から 202 ファイル(均等 181 + 大きい 20 + 0 byte)を chunk から再構成して原本(Wine prefix の FL Studio 2026)と byte 比較: 202 件一致・398,256,591 B・サイズ違い 0・欠落 0・chunk fetch 210 回**。既存の hostfiles/fruity-engine/loader/pack のテスト 72 件も通る。tsc rc 0。**このテストが見つけた parser の誤り**: magic の検査に fatal な TextDecoder を使っていて、壊れた byte で TypeError(VfsIndexError でない)。byte 比較に直した。テスト自身の誤り: I/O サーバを main thread で起動して自分を Atomics.wait で止めた(worker に移した)。
- **C の reader を opt1 の索引に通した(実測、native gcc -O2、`harness/seeds/vfs_lazy_conformance.c` + `lazy-conformance-prep.mjs`)**: prep が chunk(SHA-256 検証つき)から stream ファイルを復元 → seed list の `@lazy:` 行で `vfs_seed_from_env()` → guest 側の `CreateFileW/GetFileSize/ReadFile` で原本と比較。(1) opt1 のテスト木: 21 ファイル 8,076,557 B **不一致 0**、読み込み 0.001 s。(2) **実物の索引(27,822 ファイル + 1,522 dir、16 stream、論理 2,153,002,077 B)**: **読み込み 0.038 s(native)**、lazy ファイル 27,822、サンプル 271 ファイル(均等 250 + 大きい 20 + 0 byte)398,841,645 B **不一致 0・原本の欠落 0**、読んだ後も lazy 27,822 のまま(読みは具体化しない)、読み 0.708 s。**wasm の値ではない**(P1 への寄与は wasm で測る。native の数倍までは見込めるが読み)。
- **未**: emcc での `vfs_lazy.h` の構文確認(seed header 経由)、engine への配線(worker.ts/fruity.ts)と engine での実行、V2(OPFS・R2・Chrome)。

## 21. 診断 build #1 の失敗(--wrap と hybrid の export)と cand1(mt3。08:00〜)

- **Filter の V3(gpu1、実 Chrome)が File→Open の途中で exit 134**: `DestroyCursor: hIcon was not issued by this runtime's GDI table`(fle_gdi.c:2954)。**原因は私の Filter 差分の見落とし**: `fle_cursor.c` の `fle_cursor_is_shared_handle` が module の cursor の index を `off / 0x1000 > 8u` で『1..8』に限っていた(RM_MAX の写し)。realmod の表は 9 行で Filter は idx 8 → k = 9。区分 (b)、機構 = 『real module の数』の定数が 2 か所に別々。修正 `impl/0313f2-cursor-realmod-cap.diff`(65 行)。**red/green(実測、host の cargo test)**: テストを 0..16 に直して修正前の runtime で FAILED(`all module forms 0`)、修正後 ok。wall_c1 12・cursor 25・loadimage 33 passed。PM が HEAD に実適用 rc 0。
- **診断 build #1(diag-g9-gui、27 分)は hybrid の merge で失敗**: `build-engine-bundle.sh` が『merged module still imports functions of the module c』。`wasm_c_imports.py merged.wasm c` の出力 = `fle_imp_BitBlt`・`BeginPaint`・`EndPaint`・`SetWindowPos` の 4 個。原因(実測): `--wrap=fle_imp_X` は export の名前を `__wrap_fle_imp_X` に変え(c.opt.wasm の export を比較: 基準にある 4 名が消え、`__wrap_` の 4 名が増えた)、`hyb/exports.rsp` が `--export-if-defined=fle_imp_X` で出す 4 関数だけが重なった(ほかの 13 は export 表に無い)。direct wasm は `c.fle_imp_X` を import するので解決できない。**教訓**: 診断の `--wrap` は export 表(hyb/exports.rsp)と突き合わせてから。代替: runtime の関数の中に直接(alpha1 の diag-g9-fl)、または間接呼びの hook だけ(`impl/diag-g9-hook.diff`、リンカ非依存)。in-source で 4 関数を包む変種 `impl/diag-g9-gui-v2-inline4-unbuilt.diff` は build していない。
- **cand1**(`tools/g9f-cand1-build.sh`): Filter の木 + 0323-contract-fixes + 0319-scrollwindow-model + 0319-time-env-run-values + 0319-fle-stop-context + d1 + d2 + diag-g9-fl + diag-g9-fl-scroll(getenv に置換、manifest に FLE_DIAG_*=1)+ diag-g9-hook。**見落としを 1 つ直した**: lift 段は gen を作り直す(`onat/gen/flengine.c` の sha が 569989b9… → 2ff75d02…、`ship6-imports.txt` に ScrollWindow の行)が、`resplit` は wsrc/src が在ると『exists, skipped』で古い fle-NN.c を残す。wsrc を消して再 lift し、新旧の fle-NN/mK-NN.c を cmp して **61 ファイルが変わった**ので、その .o と rt/h-fruity 群を消した(残り 2 個)= objects は冷間に近い。build の結果は報告に書く。
- 自己申告: PM の木で `git pull` を 1 回(HEAD・ワークツリー・reflog は不変、動いたのは refs/remotes/origin/main だけ)。tool 名 `bash` を 1 回(6 回目)。diag-g9-gui の --wrap を export 表と突き合わせずに build #1 に入れた(27 分)。

### 21.1 cand1 の build の結果と、+8.45 MB の原因(私の手順の誤り)

- **cand1 `fl-engine-ci-f7ebcca1`(テスト版は `fl-engine-test-g9f-cand1-f7ebcca1`)**: 08:08:46 開始、**rc=0 09:09:14(60 分 28 秒)**。extract 2 s、lift 31 s、b2d 62 s、objects 1681 s(rt 16 s + units 1662 s)、link 1846 s(wasm-opt 1298 s、hyb-merge2 19 s)、seed 0 s、bundle 4 s。SHA256SUMS ok=9。wasm 166,973,591 B(br 41,697,087)、dll pack 84,943,680(Filter と同一)、seed 14,748,394(同一)、glue 741,071。.o の link 数 0/163。free log(2 分おき 31 sample)の MemAvailable の最小 13,084 MB。manifest の engine.env に FLE_DIAG_WIN/SCROLL/VFS_MISS=1。
- **wasm が Filter(158,438,681 B)より +8,534,910 B(+5.4%)、S1 の比 1.866 → 1.966**: 原因は診断でも差分でもなく、**hybrid strip の印 `$HYB/stripped` が古いまま skip された**こと。私の invalidate 段が wsrc を作り直したのに印を消さず、strip 前の units が compile された(build log:『hybrid strip: done for list ec345f4a4af9d034, skipped』。Filter の build は実際に strip: 『dropped 3875 functions, kept 122279』)。wasm の function section +3,850 ≒ lowered 関数 3,875。確定の根拠(実測): 新しい wsrc に同じ strip を当てると、Filter の strip 済み units との差は FLEngine が +790 B・25 行、module が +1,344 B・44 行だけ。lift に効くのは ship6-imports.txt の ScrollWindow の 1 行だけ(その行なしで lift をやり直すと flengine.c の sha が Filter と一致)。objects の増加は fle 群 +7.55 MB・mod 群 +3.99 MB に対し rt 群は +14.8 KB。**strip した版の S1 は約 1.866(読み)**。影響: 機能の確認には使えるが、C 側が direct 版でなく C 版の lowered 関数を呼ぶので性能・first idle の測定には使えない。RT-A は新しい work dir で strip 済み。是正は `tools/g9f-cand1-build.sh` の invalidate 段(印を消す)。


## PM の判断と取り込み(2026-11-07 01:35、docs のみ。製品の差分は push していない)

取り込み: mt3 の 0313(G9-a = Fruity Balance を 6 本目の実モジュールとして足す変更案の準備)の文書・差分・表・道具を `docs/investigations/mt3w/0313/` に置いた(生ログは `.txt` に改名)。**実エンジンの実行・wasm の build・Chrome は 0 件。検査は静的だけ**(patch --dry-run、bash -n、ci/test-hybrid.sh 51 ok、clang/emcc -fsyntax-only 警告 0、fe-lift 1 回)。『踏む壁』は読みで、実行の観測ではない。

事実として受け取る: 差分 `impl/0313-g9a-balance-module.diff`(10 ファイル・224 行)は main に dry-run rc 0。内容は抽出行(sha256 a399cbe0…8064)、8 本目の spec(fruity_balance_x64.dll、基底 0x6800000、他モジュールの SizeOfImage との重なり 0)、フック点 3、パス表 1 行、volume に実 DLL(+3.7 MB の seed pack)、NEXTRA=3。RM_MAX は 8 のまま。B-FLE の共有は使わない。行なし import 74(全て FLEngine も import)は、行が無いと fle_unsupported の abort で止まる(黙って成功を返す既定は無い)ので、壁ごとに契約として実装する(事前の一括は足さない)。

判断:
1. **方式**: 本 diff を main に入れる(S3a/S0+S1 の積み重ねの上には載せない)。実 DLL を seed pack に入れる(stub 化は後の最適化。+3.7 MB の first idle への影響は CI の bundle で測る。予算は前の bundle 比 +100 ms)。区分は (b) 外部契約の実装(LoadLibraryW フルパス = 決定 0183 の既存機構)とモジュール追加の手続きで、旗・環境変数・名前/アドレスの分岐は無い。
2. **検証**: テスト専用の診断 diff `impl/diag-g9-loadlib.diff`(2 か所・7 行。LoadLibraryW と GetProcAddress の戻り値を stderr に出すだけ。挙動を変えない確認は §3b)を載せたテスト bundle(名前に test)で V2b(alpha1 の node harness)と V3(Chrome)を行う。**push するのは 0313-g9a-balance-module.diff だけ**で、診断は push しない。製品の bundle は push 時の CI が作る。V2b は PARITY_SEED_TSV が FLE_VFS_SEED を置き換えるので Balance の行を足した写し(tools/mk-nested-balance-seed.sh)を使う。
3. **合格条件(V2b)**: LoadLibraryW が 0x6800000、GetProcAddress("CreatePlugInstance") が 0x6abdde0 で呼びが戻る、Problems 窓の N が 1 減り Balance の行が消える、他の 6 件は不変、first idle・音の sha など既存の測定値は B1/B2 から不変(P1 は診断の無い CI の bundle で測る)。不合格なら fle: unsupported の VA を表で API 名にして 1 つずつ契約で実装する。
4. **窓**: 冷間 build(CI_JOBS=1、BINARYEN_CORES=1、nice 19、22〜35 分)は、gpu1 の窓と ci1 の窓、S3a の掃引の後の約 03:10 以降に PM の info。
5. 未確認: FL が volume のファイルの中身を読むか、プラグインの識別に .nfo/.fst が要るか、+3.7 MB の seed と wasm の増分、Balance が実際に踏む壁(実行するまで不明)、gawk での再現(mawk のみ確認)。


### 追記(2026-11-07 04:39、mt3 の build 記録・V3・G9-b/c 設計の取り込みと PM の V3 の確認、docs のみ。製品の差分なし)

取り込み: mt3 の 0313 の現行版(§6 build の実績、§7 V3、§8 G9-b/c の scenario の設計)、表・道具・差分(`impl/diag-g9-count.diff` を含む。test 専用で push しない)、gpu1 の V3 の記録(`docs/investigations/gpu8/0313-V3-chrome-g9a.md`)。生ログは `.txt` に改名、400 KB 超は無し。

**PM が自分で確認した事実**(実エンジン、実 Chrome 156、この機械、headless、非計時の機能の確認。依頼者の機械は 0 件): gpu1 の `rt-g9a`(G9-a の test bundle `fl-engine-test-g9a-28f6f631`、wasm 150,851,575 B)と対照 `rt-g9a-ctl`(E13、同じ scenario・同じ page)の `problems-pic.png` を画像で読んだ。対照は『8 plugin(s) could not be opened』で Fruity Balance (FL) を含む 8 行、G9-a は『7』で Balance の行が消えた。`engine.log`(`rt-g9a`)に g9-diag の LoadLibraryW(Balance のフルパス)→ 0x6800000 と GetProcAddress("CreatePlugInstance") → 0x6abdde0 が各 1 回、`fle: unsupported instruction` は 0 行。対照に g9-diag は 0 行。**限界**: n = 1 ずつ、first idle は測っていない、Balance の process / GUI は未踏。= 『読み込みと CreatePlugInstance の解決』まで。**G9 は未到達**。

**大きさ(S1)の実測**(mt3 の §6): test bundle の wasm は E13 より +7,763,869 B(+5.4%)、dll pack +3,718,808 B、seed pack +3,719,103 B(Balance の PE が dll pack と seed の 2 か所に入るため pack の増分は +7.44 MB)。比は E13 の 1.848 倍に対し 1.859 倍(9 モジュールの native 合計 81,161,600 B に対する計算)。**0309 の『Balance 1 つで 1.848 → 1.94』と『wasm の増分 14.6 MB』は読みの見積もりで、実測の wasm の増分は 7.76 MB(pack を含めた非圧縮の増分は約 15.2 MB)**。この差は、プラグインの C が FLEngine と共有されない前提の見積もりと、Balance の C が小さい実測の差で、2 つ目以降の増分と B-FLE の判断(0312g〜i)は G9 で再測定する。brotli の大きさは test bundle が品質 1 なので使えない。seed の volume の実 DLL を stub にする最適化(約 3.7 MB/個)は後。

判断:
1. **G9-b を承認する**(§8 の設計のとおり。alpha1 の TEST bundle で V2b の後に node の Export を 1 本、判定は (1) `fle: unsupported` / abort が 0 行 (2) WAV のフレーム数が 5,715,911 (3) sha256 が `381cdb5a…f979` から変わるか)。sha が変わらなければ test 専用の入口カウンタ `impl/diag-g9-count.diff`(fle_callback.c の末尾に約 20 行、決定 0097 の census 用の dispatch hook の流用、動作は変えない)を載せた再 build(12〜17 分)を私の許可後に行う。性能は測らない。**push するのは `0313-g9a-balance-module.diff` だけ**(診断 2 本は push しない)。
2. **G9-c(GUI)は G9-b の結果を見てから**。ImageList_* 22 種の契約の実装は重い(日数は未見積り)ので、止まる VA を 1 つずつ。新しい runtime の実装は私の許可後。
3. G9-a の Wine 側の参照(Balance のみ有効の Export、opt1 の `elog15`)は Chrome の窓が全部終わった後。

### 追記(2026-11-07 05:15、mt3 の 0313 最終〔§9 V2b と G9-b の現状〕の取り込みと G9-b の Export の PM の確認、docs のみ。製品の差分なし)

取り込み: mt3 の 0313 の最終版(§9 を含む)と表・道具(`docs/investigations/mt3w/0313/`。`*.log` は `.txt` に改名、400 KB 超なし)。**実エンジン・実 Chrome 156・この機械・headless の実測(代役・再生なし)。依頼者の機械は 0 件。**

**G9-b の Export(ci1、Chrome、テスト bundle `fl-engine-test-g9a-28f6f631`、実行 04:53〜04:58、PM が WAV を自分で比べた事実)**: Balance を実モジュールにした bundle での Export は 22,863,828 B・5,715,911 フレーム(Balance なしと同長)、sha256 `94c9366687076a410f48c6fbe1485e8e22f6b528592fcc5ceba2d665d1554090`(Balance なし = `381cdb5a…f979`)。unsupported/abort の行は 0。`a22-wavcmp2.py` で比べると、一致フレーム 88.8%、差の RMS 3,986.6(−14.2 dB)、最大差 65,534。10 s 区間別では 0〜20 s・90〜100 s・120 s 以降は差 0.0、20〜90 s と 100〜120 s に差。alpha1 の FLP の解析(読み)では、差が出るフレーム 640,995 のうち 626,143 は Balance の自動化 clip の中、残り 14,852 は clip の終わりから 3.11 ms 以内で、clip の無い区間(81.5〜103.1 s など)は差 0。= **Balance の実モジュールが自動化を受けて出力を変えた(処理が走った)ことを示すが、変え方が正しいか(FL の Balance の実際の挙動と一致するか)は Wine の参照なしでは判定できない**。**G9 は未到達のまま**(7 種のうち 1 種の実モジュール化、他 6 種と Fruity Wrapper は未実装、Balance の GUI の観測もない)。

判断: (1) 診断を載せた bundle の作り直し(vfsdumpdir・entry counter)は今は行わない(Chrome の経路で WAV が取れる)。(2) 判定は opt1 の Wine の参照(Balance だけ有効、mp3wav-all、SSE2 cpuid の構成)の WAV との比較で行う(窓の後)。(3) 32-bit float の Export・Split mixer tracks の Export は、Wine の比較で説明がつかない場合に限り、私の許可の後。(4) 06:15 までに mt3 が G9-c(次の 1 モジュール)の設計を §10 に出す。


### 追記(2026-11-07 05:30、mt3 の §10〔G9-c の設計の最終版: 次の実モジュールは Fruity Filter 1 本〕の取り込みと PM の判断、docs のみ。製品の差分は push していない)

取り込み: §10.1〜10.5、`impl/0313f-filter-on-balance.diff`(Balance の上、17 ファイル・313 行)と `impl/0313-g9a+filter-combined.diff`(main に対する合算、365 行)、`tables/g9c/`(`*.log` は `.txt` に改名)。**静的検査のみ(`patch --dry-run` rc 0、`bash -n`、`test-hybrid.sh` 54 ok・FAIL 0、clang/emcc `-fsyntax-only` 警告 0)。build・実エンジンの実行は 0。依頼者の機械は 0 件。**

PM の判断(05:22 に mt3 へ送った内容の記録): (1) 次は Fruity Filter 1 本(Delay 2 は後の別 build。増分を 1 本ずつ測る)。(2) RM_MAX 8→16 は承認(自前の module 表の容量。FL の挙動・契約に触れない)。Split の A0 は今回の test build に混ぜない。(3) build は Balance の忠実性の判定(opt1 の Wine 参照 elog15b と alpha1 の比較)が出るか、ci1 の counted 窓の後で CPU が空いたときに、PM の info で開始。(4) S1 は wasm 全体の比(正式な基準)・モジュール別の wasm 比・dll pack と seed を含む配信量と q11 の brotli の 3 つを併記し、2 倍を超える物を隠さない。(5) stub 化は別の宿題。

**mt3 の訂正の記録**: 「RM_MAX で rt の key が変わる」は誤りで、Balance の patch だけで k_rt は変わる(H_WASM が lift/harness を含む)。Filter の追加では **k_felift も変わる**(H_FELIFT が lift/crates を hash し、realmod.rs のテスト更新が入る)ので、push すれば CI の fe-lift の cargo build も冷間になる(所要は未測定)。変更規模は最初の見積り(約 12 ファイル・+60〜80 行)より大きい(17 ファイル・313 行)。**押さえる点**: これらは見積り・静的検査で、Filter の実エンジンでの動作は未確認(Filter の到達範囲は直接 call の下限のみ、insert 47 が音を持つか、ildsp が NULL のまま通るかは未確認)。G9 は未到達のまま。


### 追記(2026-11-07 06:12、mt3 の §11〔ImageList_* 22 種の契約の実装案〕・§12〔Balance の窓を開く scenario〕・§13〔別経路〕・§14〔Filter の test build の手順書〕の取り込みと PM の判断、docs のみ。製品の差分なし)

取り込み: mt3 の 0313 の現行版(§11〜§14)と表・道具・差分(`docs/investigations/mt3w/0313/`。`*.log` は `.txt` に改名、PNG と 400 KB 超は含めない)。**§11〜§14 は読みと静的検査(patch --dry-run、bash -n、test-hybrid)だけで、実エンジンの実行は 0。**

**Balance の忠実性(PM の実測。0310 §9 の PM 追記と 0307 §2p の PM 追記を参照)**: 実エンジン同士で、Wine 上の元の x86 FL(Balance のみ有効、MP3→WAV 入力、cpuid SSE2)の Export と wasm の G9-a(Chrome の Export、sha256 `94c93666…4090`)は、5,715,911 フレーム中 5 フレームだけが ±1 LSB 違い(RMS 0.0007、−149.8 dB)。Balance なしの wasm との差(RMS 3,986.6)は Wine と wasm で 4 桁一致。= Balance の実モジュールの Export の出力は元の FL と一致(lift の欠陥は見えない)。**G9 は未到達**(7 種のうち 1 種、GUI 未観測、他 6 種と Fruity Wrapper は未着手、test bundle は診断入りで push しない)。

判断:
1. **§11(ImageList_*)**: 設計の方向を承認(状態だけの 7 関数 → GUI の観測 → 以降の段)。実装は私の許可後。行表への追加は全モジュール共通で FLEngine の C も変わるので、push すれば全段の冷間 build。ほかの G9 の変更と同じ push に載せる(単独では出さない)。契約にない値(Learn が定義しない last error など)は、名前つきの停止にし、成功を既定にしない。
2. **§12・§13(GUI の scenario)**: gpu1 が非計時の Chrome で段 0〜2 を実行し、私が PNG を読んでから段 3 以降(§13 の別経路は 1 run に 1 経路)。
3. **§14(Filter の test build)**: 手順書を承認。開始は ci1 の計時の窓が返った後の私の info(Balance と Wine の判定の条件は満たした)。`CI_JOBS=1`、`BINARYEN_CORES=1`、nice 19、MemAvailable < 8 GB で止めて報告。realmod の cargo test は objects の段に重ねてよいが、ci1 の e2e の時間とは重ねない。Delay 2 は別 build。


### 追記(2026-11-07 06:53、Balance の GUI の観察〔gpu1 の stage 3〕の記録、docs のみ。製品の差分なし)

**実エンジン・実 Chrome 156・この機械・headless・非計時の観察(代役・再生なし。依頼者の機械は 0 件)**: gpu1 が G9-a の test bundle(`fl-engine-test-g9a-28f6f631`、診断入り)をページ p0(ポート 4969)で、06:38:38〜06:42:42 に走らせた。参照プロジェクトを開き、Problems 窓を閉じ、Mixer の Insert 1 を選択し(Mixer は FL 自身が前面に出した)、スロット 0 の名前(1750,617)を**1 回だけ**クリックした。PM が `03a-1s.png` と `02-insert1-selected.png` を読んで確認した事実: (1) 『Fruity Balance (Insert 1)』と題した窓の**枠**が (482,233)〜(713,406) に出た(最小化・閉じるボタン、『Effect slot 1』の注記)。(2) **client 領域は描かれていない**(背後のチャンネルラックのボタンが透けて見える。02 との差は client 領域で 146 画素、1 s → 3 s で 13 画素、3 s → 10 s で 0)。(3) engine log の増分は 1・3・10 s とも 0 行。`fle: unsupported instruction`・abort・stop の行なし。(4) 窓の一覧は主窓のみのまま(Balance の枠は主窓の面の中に描かれている)。(5) engine-idle のまま、opErrors 空。`pixel` op の fallback(`deskDumpBase64(true)`)は実 Chrome で初めて通った(dist 156)。コピー: `lift/out-git/gpu8-results/logs/rt-g9c-s3/`。

**読み(未確認)**: プラグインの GUI の生成(client 領域の作成・配置・描画)が行われていないか、行われても表示されていない。候補と切り分けは mt3 の §15(07:30 まで)と alpha1 の node G3 の窓の表(07:20 まで)が担当する。**これは G9 ではない**: 7 種のうち 1 種(Balance)の Export の一致のみ、GUI は枠だけ、他の 6 種と Fruity Wrapper は未着手。新しい runtime の実装(ImageList_* ほか)は PM の許可の後で、他の G9 の変更と同じ push に載せる(共有の行は FLEngine の C も変えるため全段の冷間 build になる)。


### 追記(2026-11-07 09:48、cand1・RT-A・Filter の判別の結果の取り込みと PM の確認、docs のみ。製品の差分なし。依頼者の機械は 0 件)

**実エンジン・実 Chrome 156・この機械・headless・非計時(代役・再生なし)の結果。PM が実ファイルと numpy で確認した物だけを書く。**

- **cand1**(`fl-engine-test-g9f-cand1-f7ebcca1`、rc=0 09:09:14、wasm 166,973,591 B): **hybrid strip の印が古いまま skip された**(Filter の build log `hybrid strip: 61 units, dropped 3875 functions`、cand1 は `hybrid strip: done ... skipped`)ため、direct wasm に下げたはずの 3,875 関数が C 側にも重複して入った(C 部分 146,444,280 → 154,898,590 B、+8.45 MB)。**cand1 は機能の確認と Export の比較にだけ使い、性能・first idle・S1 の測定には使わない**。cand1 の S1 1.966 は build の artifact。
- **RT-A**(実スレッド版、strip 済み、`fl-engine-test-rt1-91bf78f6`、rc=0 09:37:10): PM が確認: SHA256SUMS 非 OK 0、**wasm 158,362,507 B、S1 全体 = 158,362,507 ÷ 84,942,208 = 1.8644**(E13 143,087,706 B から +15,274,801 B、Filter bundle より 76,174 B 小さい)。dll pack 84,943,680 B、seed 14,748,394 B。build log 67 行に `hybrid strip: 61 units, dropped 3875 functions`。
- **V3 Filter(gpu1、cand1)**: 合格。`LoadLibraryW "…Fruity Filter_x64.dll" -> 0x7000000`、`GetProcAddress(0x7000000,"CreatePlugInstance") -> 0x72cb670`、`fle: unsupported` 0、Problems 窓は 6 plugin(Wrapper / FLEX / Frequency Shifter / Soundgoodizer / Parametric EQ 2 / Delay 2。PM が画像で確認)。直前の Filter bundle の DestroyCursor の停止は 0313f2 入りで再現しない。**実モジュールは 7 種のうち 2 種(Balance・Filter)**。
- **Export(ci1、cand1)**: 原 FLP は sha256 `94c9366687076a41…4090` = Balance のみの G9-a の Export と**全バイト一致**(PM の numpy でも 0 フレーム差)、Wine の `elog16`(Balance + Filter)とは 5,715,911 フレーム中 2 フレームが ±1 LSB(RMS 0.0004)。原 FLP では Filter が音に効かない(Insert 47 に Agogo の信号しか来ない)ので**判別できない**。
- **Filter の判別(ins1-lp。Insert 1 のスロット 1 に Filter を足した変種 FLP)**: wasm `export-cand1-ins1lp-r2`(5,729,830 フレーム、sha256 `cf57f124d57264a3…1e39`)対 Wine `elog16-ins1-lp`(5,729,968 フレーム)。PM が numpy で確認: 長さの伸びは wasm +13,919・Wine +14,057(差 138 フレーム)。効果量(変種 − 原)は Wine RMS 12,697.66、wasm RMS 12,314.05(97%)。効果の始まりは Wine frame 211,681(4.800 s)、wasm frame 210,723(4.778 s)。**wasm 変種 − Wine 変種の差 RMS 5,325.57**(4,600,668 フレームが非零、最大 52,591)。窓別 0–5 s 649.02、5–20 s 6146.74、20–60 s 4876.45、60–120 s 5725.46。効果信号どうしの最良 lag は 0、利得 1.0074。帯域別の効果のパワー比(wasm/Wine、20–30 s 左ch): 0–200 Hz 0.721、200–1k 0.935、1k–4k 0.998、4k–22k 0.986。**= Filter は Wine と一致していない。高域・中域はほぼ一致し、低域だけ wasm の効果が約 28% 小さく、効果の始まりが 22 ms 早く、尾が 138 フレーム短い。lift の欠陥か、Wine の変種 run の非決定か、FLP の状態かは未特定**。切り分けの手配: alpha1(host 側の値・x87・MXCSR の読み)、opt1(Wine の ins1-lp をもう 2 本、ins1-bp を 1 本)、ci1(wasm の ins1-bp の Export)。**G9 は未到達**。
- **Balance の GUI(cand1)**: client は描かれない(0323① の背景消去の修正で変化なし)。gpu1 は『プラグイン DLL 自身が作る窓の create は 0 行』と読んだが、PM の訂正: FLE_DIAG_WIN の `win create lid 171 class "TWPControlForm" … parent_lid 170 caller 0x6973abe` の caller 0x6973abe は Balance の image の中(0x6800000 + RVA 0x173abe)。**Balance の VCL が自分の form(lid 171、227×148、style 0x46000000 = WS_VISIBLE なし)を作っている**。この form が表示され WM_PAINT を受けるかを、alpha1 の `diag-g9-fl-paint`(窓ごとの vis・show・paint-msg・BeginPaint・draw-discard)で次の bundle で見る。
- 欠落資材(cand1 の vfs-miss): BigFruit.png、AboutTxt.txt、Fruity microfont.ttf、Windows\\Fonts の各種、Settings\\Skins\\Default\\ScopeGradient.bmp ほか。O3(About)と O16(日本語)の資材仮説は seed 実験用 bundle(0315c)で確認する。
