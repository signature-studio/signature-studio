# 0372: ole32!CoInitializeEx の応答(GetProcAddress で取られる)
分類: (b 外部 API の契約の不足を埋める)
撤回条件: 実エンジンで台本 A(About を閉じる)を観測し、停止が同じ位置(`indirect call to 0 [guest return 0x1331c94]`)で続く、または `fle_imp_CoInitializeEx` の名前つき停止(dwCoInit が 2 以外)で別の停止が出る、または `cargo test` に新しい失敗が出るなら、この差分を外す。
日付: 2026-11-15   ROADMAP: R5-3   一覧: `evidence/R5-3f.md`・`evidence/R5-3g.md`・`evidence/R5-3h.md`   コミット: 作業はブランチ `r5-3h-coinitializeex`(main には入れていない)

## 決定
決定: `ole32.dll` の `CoInitializeEx` の応答を追加する。`lift/runtime/fle_ole.c` に `fle_imp_CoInitializeEx` を書き、`lift/runtime/fle_ole.h` に宣言を書き、`lift/crates/fe-lift/src/imports_table.rs` の `TABLE` の末尾に 1 行足す。応答は、`dwCoInit` が 2(`COINIT_APARTMENTTHREADED`)のときだけ `com_init` と同じ答え(初回 `S_OK`、以降 `S_FALSE`、`com_depth` を増やす)にする。2 以外の値は名前つき停止にする。
根拠: 停止位置の `call [rax]` は `CoInitializeEx(NULL, dwCoInit)` の形の呼び出しで、`rax` の先の値は `GetProcAddress("ole32.dll", "CoInitializeEx")` の戻り値(`evidence/R5-3f.md`・`evidence/R5-3g.md` の実測の命令列。値の確定は未確認)。応答表に `CoInitializeEx` の行が無いため、`fle_imp_GetProcAddress` は NULL を返す(`fle_ole.c` の `GetProcAddress` の応答の規則は旧記録 `docs/archive/investigations/getprocaddress-null-impact.md` の A2 の読み)。応答表の行が無いのは、API の契約(応答)の不足なので、分類は (b) である。
分類の理由: FL の挙動を補う・代行する処理は足さない(ページ・ランタイムの窓管理に触れない。`docs/ops/fidelity-and-performance.md` §6)。足すのは Win32 の API の応答だけ。

## 範囲(実装の差分。これ以外は変えない)
1. `lift/runtime/fle_ole.c`: `fle_imp_CoInitializeEx`(`dwCoInit` を読み、2 なら `com_init` と同じ答え、2 以外は名前つき停止)。`com_init` の本体は変えない。
2. `lift/runtime/fle_ole.h`: 宣言と説明の 2 行。
3. `lift/crates/fe-lift/src/imports_table.rs`: `TABLE` の末尾に 1 行(`("ole32.dll", "CoInitializeEx", "fle_imp_CoInitializeEx", "jmp_imp:CoInitializeEx")`)。
4. テスト(`lift/crates/fe-lift/tests/oleaut_ole.rs`): STA の `S_OK`・`S_FALSE`・`com_depth`・`CoUninitialize` との釣り合い、`dwCoInit` の上位 32 ビットを読まないこと、STA 以外・`pvReserved` 非 NULL で停止、行と宣言が 1 つあること。
5. 位置を固定するテストの更新(`lift/crates/fe-lift/tests/rows_0372/mod.rs` の新規 1 ファイル、`wall_fr4`・`wall_fr5`・`wall_fr13`・`wall_fr15`・`wall_fr16`・`wall_fr19`・`wall_fr20` の `mod` 行と数え方に 1 項)。既存の期待値の値は変えていない。数え方の項を足しただけ。

他の 5 つの名前(`CoCreateInstanceEx`・`CoAddRefServerProcess`・`CoReleaseServerProcess`・`CoResumeClassObjects`・`CoSuspendClassObjects`)は足さない。根拠: 旧記録は「結果が検査されない・読み手が無い」と読むが(`docs/archive/investigations/getprocaddress-null-impact.md`)、停止位置の外の検査は未確認。足すと、検査して別経路に落ちていた FL の動きが変わる。

## 契約(Microsoft Learn の読み)
Learn の `CoInitializeEx` のページ(取得済み。`pilot_webfetch` で取得した本文を読んだ):
- `pvReserved`(第 1 引数): 「reserved and must be NULL」。NULL でなければ停止する(`com_init` の既存の挙動と同じ)。
- `dwCoInit`(第 2 引数): 「Any combination of values from COINIT can be used, except that the COINIT_APARTMENTTHREADED and COINIT_MULTITHREADED flags cannot both be set. The default is COINIT_MULTITHREADED.」
- 戻り値: `S_OK`(初期化された)、`S_FALSE`(既に初期化されている)、`RPC_E_CHANGED_MODE`(以前の呼び出しと並行モデルが違う)。
- 釣り合い: 成功した呼び出し(`S_FALSE` を含む)は、それぞれ `CoUninitialize` で釣り合わせる。
- COINIT(Learn の `COINIT` のページ。取得済み): `COINIT_APARTMENTTHREADED = 0x2`、`COINIT_MULTITHREADED = 0x0`、`COINIT_DISABLE_OLE1DDE = 0x4`、`COINIT_SPEED_OVER_MEMORY = 0x8`。

読み(決定の理由): ゲストは 1 スレッドの STA(決定 0037)。`dwCoInit` の 2 は STA で、`com_init` と同じ答え。0(`MULTITHREADED`、Learn の既定値)は STA と違うので、0037 の「アパートメントは常に STA・`RPC_E_CHANGED_MODE` は発生させない」に反する。よって名前つき停止にする。`2|4`・`2|8` は、Learn のページが何をするかを書いていない(フラグの意味だけ書く)ので、VM の応答を決められない。分からないときは足さない側として、2 以外を名前つき停止にする。

## `com_init` の `pvReserved` の位置(事実。直していない)
- `fle_ole.h` の 3 行目: 引数は `R[1]`(rcx)、`R[2]`(rdx)、`R[8]`(r8)、`R[9]`(r9)。
- `fle_ole.c` の `com_init` は `R[1]` を見る。`CoInitialize` の第 1 引数(`pvReserved`)は rcx なので、`R[1]` は正しい。したがって `com_init` の既存の挙動に誤りは無い。
- `CoInitializeEx` の第 1 引数(`pvReserved`)は `R[1]`、第 2 引数(`dwCoInit`)は `R[2]`(rdx)。`fle_imp_CoInitializeEx` は `R[2]` を読む。

## 実測(根拠の出典)
- `evidence/R5-3f.md`・`evidence/R5-3g.md`: 停止位置 `0x1331c80` の `call [rax]`、`xor ecx,ecx; mov edx,2`(`CoInitializeEx(NULL, 2)` の形の読み)、`0x5dfba0` の `GetProcAddress` で 6 つの `ole32.dll` の名前を取る経路。
- `lift/harness/seeds/ship6-imports.txt`(`expire_grep` の実測): `ole32.dll` の行は 4 件(`OleSetClipboard`・`OleIsCurrentClipboard`・`OleFlushClipboard`・`DoDragDrop`)。`CoInitializeEx` の行は無い。
- `imports_table.rs` の `TABLE`: `CoInitializeEx` の行は無かった(実測)。

## 未確認
- なぜ seed pack に 3 ファイルがある bundle だけがこの経路に入るか(未確認。`evidence/R5-3f.md` 5 節)。
- P(`[0x13d9f48]`)の実行時の値。`GetProcAddress` の応答の値が 0 である、とこの決定は確かめていない(読み)。
- FL が `CoInitializeEx` に渡す `dwCoInit` の値。`0x5e0dcc` は `mov edx,[0x13d9f70]` で値を読むが、その値は未確認。したがって、FL が STA(2)以外・`2|4` を渡すかは、この依頼では確かめていない。来れば名前つき停止で止まり、次の観測で分かる。
- 実エンジンでの動作。この依頼では実エンジンを実行していない。
- `lift/harness-docs-tools/alpha1/gen-wrap.py` の 24 行の `'CoInitializeEx': 'NONE'` の意味(未確認)。
- `docs/decisions/README.md` の規則 11 の現行の文面(本依頼の対象外。未読)。

## 出典
- `lift/runtime/fle_ole.h` 3 行目(引数の位置)・`lift/runtime/fle_ole.c` 36〜67 行(`com_init` と COM の釣り合いの注釈)。
- Learn: `https://learn.microsoft.com/en-us/windows/win32/api/combaseapi/nf-combaseapi-coinitializeex`(取得済み)・`https://learn.microsoft.com/en-us/windows/win32/api/objbase/ne-objbase-coinit`(取得済み)。
- `docs/decisions/0037`(実体は `docs/archive/decisions/0037-apis-oleaut-ole.md`。STA の決定)。
- `evidence/R5-3f.md`・`evidence/R5-3g.md`・`evidence/R5-3h.md`。

注(2026-11-15、R5-3i): 上の本文の「テストの更新」の新規ファイル(テスト用の rows の mod 1 件)は、ブランチ `r5-3h-coinitializeex` の `2f1915b` にだけあり、main には無い(`git ls-tree -r --name-only 2f1915b` を、そのテスト用の場所で引くと 1 件、`HEAD` で引くと 0 件)。0373 の観測は `2f1915b` から作った bundle で行った(`evidence/R5-3i.md`)。

注(2026-11-16、R5-3m): 上の 2026-11-15 の注(R5-3i)は取り込み前の状態。`d2d90a9` の取り込みで `lift/crates/fe-lift/tests/rows_0372/mod.rs` は main に入った(`git show --stat d2d90a9` に出る)。テスト `coinitex-speed-over-memory` の値は、決定の `2|8`(= 10)に合わせて 8 から 10 に直した(commit A `123deac`)。取り込み後の HEAD での cargo test は `evidence/R5-3m-cargo.txt`(commit A の HEAD で実行。終了コード 101、既知の 1 件のみ)。
