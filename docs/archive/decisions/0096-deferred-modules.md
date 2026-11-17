# 0096: 遅延モジュール（deferred）— `blend2d.dll` と `quickfontcache_x64.dll` を、lift するまでの間「LoadLibrary は成功、export の関数は名前付きで停止」にする（測定を unit 150 以降へ進めるための順序づけ。仮の決定）

状態: **仮の決定**。PM の決定（2026-10-25）を、作業者（サブエージェント）が記録した。**この決定は決定 0073（外部モジュールを「無い」と定義しない）、0086（同梱 DLL は実体を lift して実在モジュール `fle_realmod` に載せる）を変えない**。0086 の lift をやめるものではなく、lift を待たずに測定を unit 150 以降へ進めるための**順序づけ**（lift は別レーンで続く）。決定の節、背景、範囲、範囲外、撤回条件は実装の前に保存し、「実装した場所」「テスト」「検証」「解釈が分かれた点」「限界」は実装と検証の後に書いた（すべて記入済み）。

事実の出典（節番号は各文書のもの）:

- `docs/decisions/0086-bundled-dlls-as-additional-modules.md`（背景 §1 の LoadLibraryW の順: unit 144 dsp_ippv2、149 Blend2D、158 quickfontcache_x64。「決めないこと」）、`0073-lift-dsp-ippv2-as-second-module.md`、`0082-two-module-load-semantics.md`（D11: 参照 0 の `FreeLibrary` は名前付き停止。実在モジュールのロードの意味）、`0015-virtual-modules.md`（仮想モジュール表、ハンドル `0x7FF800000000 + 0x10000 × N`、応答表の末尾に足す運用）、`0081`、`0088`（N = 27〜32 を末尾に足した先例）、`0091`（`GetModuleHandleA`）、`0092`（応答表の末尾に 7 行を足した先例。FLEngine の生成 C の SHA `eb9d8e7f…` → `feb5b908…`）、`0094`（現在の SHA `feb5b908…`）、`0046`（M1 の受け入れ。A4 の定義。**変えない**）。
- `docs/investigations/blend2d-lift-feasibility.md` §4.1（ゲストは `LoadLibraryW("Blend2D.dll")` の直後に `GetProcAddress("AggCreateInstance")` を 1 回引き、保存先 `0x15f45f0`、ハンドル `0x15f4600`、参照カウント `0x15f45f8`）、§4.2（使い手 `f_6cb0b0` は静的には unit 614、745、880、895、898 から届く。読み）。
- `docs/investigations/quickfontcache-lift-feasibility.md` §3.3（export 10 名）、§5.1（FLEngine が `GetProcAddress` で引く 8 名）、§5.2、§5.3（使い手の静的な到達。`FontCache_AcquireFont` の使い手 `f_6685d0` は unit 245 以降から静的に届く。読み）。`docs/investigations/registry-unit153.md` §6.3（unit 158 の `LoadLibraryW "quickfontcache_x64.dll"`）。`docs/investigations/blend2d-remaining-imports.md`。
- `lift/runtime/fle_load.c`（`normalize`: ASCII の英字を小文字に畳む、`.` が無ければ `.dll` を足す、末尾の `.` は拡張子なしとして取り除く。`load_by_name` の順は (1) 実在モジュール `fle_realmod`、(2) `fle_modules[]`、(3) `fle_load_absent[]`、(4) 停止）、`fle_modules.c`、`fle_realmod.c`（`table_check` は実在モジュールの名前が仮想モジュールの名前なら停止）。

## 決定（PM。内容を変えない）

| # | 項目 | 決定 |
|---|---|---|
| 1 | 遅延モジュール（deferred） | 対象は `blend2d.dll` と `quickfontcache_x64.dll`（ゲストが `LoadLibraryW` に渡す名前を `fle_load.c` の `normalize` で畳んだ綴り）の 2 つ。DLL はインストーラに実在する（決定 0073）が、**まだ lift していない** |
| 2 | LoadLibrary | `LoadLibraryW` / `LoadLibraryA` / `LoadLibraryExW` でその名前を渡すと、**成功してハンドルを返す**。ハンドルは仮想モジュールと同じ規則 `0x7FF800000000 + 0x10000 × N` の続きで、**N = 33（`blend2d.dll`）、N = 34（`quickfontcache_x64.dll`）**。**DllMain は実行しない。メモリ像も作らない**。参照カウントは仮想モジュールと同じ（決定 0082） |
| 3 | GetProcAddress | その DLL の **export 名**（Blend2D: `AggCreateInstance`。QFC: `lift/out-qfc/quickfontcache_x64.dll` の export 全部 10 名。PE の export ディレクトリから読み、調査文書と突き合わせる）には**擬似 VA を返す**（大文字小文字は仮想モジュールと同じ規則）。その擬似 VA を呼ぶと**名前付きで停止**する。停止メッセージは `fle: <module>!<export>: the module is installed but not lifted yet (deferred, decision 0096; decision 0086 lifts it later)` に引数レジスタ（RCX、RDX、R8、R9）の値を添える。export に無い名前は従来どおり NULL と `ERROR_PROC_NOT_FOUND`（127） |
| 4 | GetModuleHandle / FreeLibrary | `GetModuleHandleW/A` は仮想モジュールと同じ規則（決定 0015、0091、0082 の DllMain 中の扱いは変えない）。`FreeLibrary` は仮想モジュールと同じ（決定 0082 D11: 参照 0 になる呼び出しは名前付き停止） |
| 5 | 実在モジュールが優先 | `fle_realmod` に**同名の実在モジュールが登録されているときは、実在モジュールが優先**（別レーンで Blend2D を lift して 3 モジュールで動かす作業を壊さない。`lift/out-rdefs/three/` の 3 モジュール構成が引き続き動くこと） |
| 6 | A4 の扱い（決定 0046 は変えない） | 遅延モジュールを使って測った a4 の数は、**「遅延構成（診断）」と別表記**にする。M1 の A4 の達成は、両 DLL を lift して実在モジュールにしてからの数で判定する。遅延モジュールは**DllMain の副作用が省かれる**ので、実機の挙動と同じと主張できない |
| 7 | 実装の形 | 応答表の行（`lift/crates/fe-lift/src/imports_table.rs` の `TABLE`。**末尾に足す**。既存の擬似 VA は動かない。決定 0015）と runtime の実装は**最小の侵襲**で。**新しい `.c`、`.h` は作らない**（`modules.rs` が全 `fle_*.c` をリンクする）。**FLEngine の生成 C の SHA は変わる**（応答表の行が増える。変わるのは応答表の箇所だけで、関数は 0 個のはず。旧 C との diff で確認して報告） |
| 8 | 状態 | **仮の決定**。撤回条件は下 |

## 背景（事実。出典の節番号）

- 現状、FLEngine の unit 149 は `LoadLibraryW("blend2d.dll")`、unit 158 は `LoadLibraryW("quickfontcache_x64.dll")` で名前付き停止し、a4 の深さ（`init_count` 150/942）がそこで止まる（決定 0086 背景 §1、`docs/investigations/two-module-after-0088.md` §3。実測）。どちらの名前も `fle_modules[]` にも `fle_load_absent[]` にも `fle_realmod` の表にも無いので、`fle_load.c` の case 3 の `fle_stop`（`module "…" is not a virtual module and not a name this environment is defined to lack`）になる。
- ゲストは Blend2D をロードした直後に `GetProcAddress` で export の関数ポインタを**保存するだけ**で、実際に呼ぶのは描画経路など遅い unit（Blend2D は静的な読みで unit 614、745、880、895、898 から `f_6cb0b0` に届く。Blend2D 調査 §4.2。読み）。QFC は 8 名を引く。うち `AcquireSharedThreadBank`、`AcquireSharedAnimTimer` の 2 名は unit 158 の `f_66d050` が続けて引く（QFC 調査 §5.1。読み）。使い手のうち `FontCache_AcquireFont` の `f_6685d0` は **unit 245 以降**から静的に届く（QFC 調査 §5.3。読み）。つまり QFC の方は「遅い unit」ではなく、unit 158 の後の比較的早い unit（245）で最初の呼び出しに届く可能性が静的な読みにある。**実行して確かめていない**（この決定の測定が確かめる）。
- 両 DLL は export の内容が違う。Blend2D は `AggCreateInstance` 1 名（`lift/out-blend2d/Blend2D.dll` を pefile で読んだ。実測）。QFC は 10 名で、`.text` のコード 9 名と、`.bss` の変数 `dbkFCallWrapperAddr` 1 名（pefile で読んだ。セクションと属性。実測。QFC 調査 §3.3 も同じ）。FLEngine が引くのは QFC の 8 名で、残り 2 名（`dbkFCallWrapperAddr`、`__dbk_fcall_wrapper`）は FLEngine も同名を export し、FLEngine は引かない（QFC 調査 §1 の 9）。
- 決定 0086 は、両 DLL を「無い」（`fle_load_absent[]`）にも仮想モジュール（`fle_modules[]` の偽の応答表）にもせず、実体を lift して実在モジュールにする方向を決めた。この決定（0096）はその**途中の順序づけ**で、0086 の却下 (b)(c) を撤回しない（下の「0086 との関係」）。

## 0086 との関係（変えないもの、変えるもの）

- 変えない: 0086 の方向（実体を lift して実在モジュールに載せる）。0073。0082。`fle_load_absent[]` の中身（0015、0029）。`fle_realmod` の仕組みと制約（`RM_MAX` 4、基底の検査）。
- 足すもの: LoadLibrary 系、`GetModuleHandle` 系、`GetProcAddress` の応答で、この 2 名だけが「ロードできて、export 名だけ引けて、呼ぶと停止する」モジュールとして見える。**実在モジュールが登録されればそれが優先する**ので、lift が済んだ名前から遅延モジュールは使われなくなる（撤回条件 (b)）。
- 0086 は「仮想モジュールにしない」と決めたが、その却下の理由は「実機の DLL の有無について偽の主張になる」「関数の意味を runtime が創作する」だった。遅延モジュールは**ロードの成功**と**export 名の存在**（どちらも PE の実体から読んだ事実）だけを答え、**関数の意味は創作せず、呼ばれたら停止する**。それでも「DllMain を実行せずロードが成功する」点は実機と同じではない（決定の 6）。この決定が仮なのはそのため。

## 範囲（今回扱う）

- 遅延モジュール 2 名（N = 33、34）の LoadLibraryW／A／ExW、`GetModuleHandleW／A`、`GetProcAddress`、`FreeLibrary`。
- 応答表の末尾 11 行（Blend2D 1、QFC 10。**export の全部**）と、それを呼んだときの名前付き停止（11 個の runtime 関数）。
- 実在モジュールとの優先順位（登録されていれば実在が優先）。
- 既存テストの固定数（`fle_modules[]` の数 32 など）の更新（決定 0088、0092 と同じ流儀）。新しいテスト。

## 範囲外（今回扱わない）

- 両 DLL の lift、リベース、実在モジュール化（0086。別レーン）。
- DllMain の実行、メモリ像、export の関数の実装（呼ばれたら停止するだけ）。
- 遅延モジュールで測った数を A4 の進捗として数えること（決定の 6。決定 0046 は変えない）。
- 他の同梱 DLL 11 名（0086 の範囲の節。実行が届いて調査した時点で決める）。
- `GetProcAddress` の序数、export のフォワーダ（仮想モジュールと同じ扱いのまま）。
- 遅延構成で測って現れる新しい停止（直さない。報告するだけ）。

## 撤回条件（仮の決定）

- (a) ゲストが両 DLL のメモリ（DllMain が書くはずのもの）を直接読むと分かったとき。
- (b) 両 DLL を lift して実在モジュールにしたとき（遅延モジュールを外す）。外すときは `fle_modules[]` の末尾 2 行（N = 33、34）と応答表の末尾 11 行を、実在モジュールの登録と同じ変更で取り除く。N = 33、34 は表の末尾なので他のハンドルは変わらない（決定 0088 の撤回条件と同じ形）。応答表の行を取り除くと、その後ろに足された行の擬似 VA が動く点は、その時に扱う（決定 0015 の「末尾に足す」運用の限界）。
- (c) 実機で `LoadLibraryW` が失敗すると分かったとき。

## 実装した場所

- `lift/runtime/fle_modules.c`（+44）: `fle_modules[]` の末尾に `"blend2d.dll"`（N = 33、index 32）、`"quickfontcache_x64.dll"`（N = 34、index 33）と決定 0096 を指すコメント。`FLE_DEFERRED_FIRST`（32）、`FLE_DEFERRED_N`（2）、`fle_module_is_deferred(index)`。`get_module_handle`（GetModuleHandleW／A の共通本体）は、表に当たった名前が遅延モジュールで同名の実在モジュールが登録されていれば表の答えを返さず実在モジュールの経路へ進む。11 個の `fle_imp_deferred_*`（Blend2D 1、QFC 10）は、`fle: <module>!<export>: the module is installed but not lifted yet (deferred, decision 0096; decision 0086 lifts it later) RCX=0x… RDX=0x… R8=0x… R9=0x…` を stderr に出して `abort()`。`GetProcAddress` 本体は変えていない（応答表の `dll` 欄がモジュール名の行を引く既存の経路）。
- `lift/runtime/fle_modules.h`（+16）: `fle_module_is_deferred` と 11 個の宣言（`noreturn`）。
- `lift/runtime/fle_realmod.c`（+2 −1）: `table_check` の「実在モジュールの名前が仮想モジュールの名前なら停止」から遅延モジュールの名前を除く。`fle_realmod.h`（+3 −2）: その注記。
- `lift/crates/fe-lift/src/imports_table.rs`（+15）: `TABLE` の**末尾**に 11 行（`blend2d.dll!AggCreateInstance`、`quickfontcache_x64.dll!` の 10 名を PE の export ディレクトリの序数順に）。擬似 VA は `0x7ff9000010d0`〜`0x7ff900001170`（行 269〜279）。既存の擬似 VA は動かない。
- `lift/runtime/fle_load.c`: **変更なし**。LoadLibraryW／A／ExW は `load_by_name` が (1) 実在モジュール、(2) `fle_modules[]` の順に引くので、表に 2 名を足すだけで、成功・ハンドル・参照カウント（初期 1、LoadLibrary で +1）・`FreeLibrary`（参照 1 で名前付き停止、決定 0082 D11）が仮想モジュールと同じになり、実在モジュールが優先される。
- 新しい `.c`、`.h` は作っていない。決定 0073、0082、0086、0015 の本文は書き換えていない。

## テスト

- 新規 `lift/crates/fe-lift/tests/deferred.rs`（15 件）: 応答表の 11 行（既存の 268 行の直後、擬似 VA が動かない、export 名が実 DLL の export ディレクトリと一致）、表の末尾が N = 33、34、LoadLibraryW／A／ExW の成功とハンドルの規則・last error 不変・**DllMain などゲストの呼び出し 0 回**・何もマップされない、参照カウントと `FreeLibrary`、`GetModuleHandleW／A`、`GetProcAddress` が 11 名すべてで擬似 VA（一意）・export にない名前は NULL と 127、**11 個すべての擬似 VA を `fle_dispatch` で呼ぶと名前付きで停止（stderr 全文と引数 4 つ）**、ゲストの手順（保存して後で呼ぶ）、近い名前（`blend2d2.dll`、32 ビット版 `quickfontcache.dll` ほか）は従来の名前付き停止、実在の `blend2d.dll` を登録したドライバで実在が優先（GetModuleHandle、LoadLibrary で DllMain が 1 回、GetProcAddress が実イメージの export ディレクトリ、FreeLibrary は実在モジュールの規則）かつ QFC は遅延のまま。
- 既存テストの固定数の更新（表が 32 件 → 34 件）: `modules.rs`（`* 32` → `* 34` が 2 行、`count 32` → `34` と範囲、名前の検査に 2 名を足す、`handle-past-table` の `h(32)` → `h(34)`）、`msctf.rs`（`count 32` → `34`、`fle_module_name(32)` → `(34)`、シナリオ `handle-33` → `handle-35`）、`load.rs`（`fle_load_refcount(32)` → `(34)` が 2 行、`fle_module_handle(32)` → `(34)`）、`event.rs`、`glob.rs`、`vol.rs`、`vfs.rs`（`mods_end` の `* 32` → `* 34`）。理由は表の長さが変わったことだけで、各テストの意図（「表の外の最初のハンドルで停止」など）は保っている。詳細は `docs/investigations/deferred-measurement.md` §3.2。
- 変異検査（自己検査。決定の根拠ではない）: 8 種（遅延が実在に勝つ、テーブル検査が実在を拒む、R8 と R9 の取り違え、行の欠落、行の取り違え、N の入れ替え、停止せず 0 を返す、LoadLibrary が表で見つけない）がすべて 1 件以上のテストを落とした。元に戻して `cmp` で一致を確認。詳細と取り直しの記録は測定記録 §3.3。

## 検証

詳細は `docs/investigations/deferred-measurement.md`。作業物は `lift/out-deferred/`。

| 項目 | 結果 |
|---|---|
| `cargo test --workspace`（`CARGO_TARGET_DIR=lift/out-deferred/target`、`FLENGINE_PE` 指定、`ulimit -c 0`） | **99 binaries / 1,243 passed / 0 failed / 0 ignored**（基準 98 / 1,228 / 0 / 0。**+1 binary、+15 件**は新規の `deferred.rs`。`SKIPPED` 0 件。既存のテストは増減していない。出力は `lift/out-deferred/test-ws3.txt`。最初の全体テスト `test-ws.txt` も同じ 99 / 1,243 / 0 / 0。その後に `deferred.rs` の総行数の検査を緩めたので取り直した） |
| **FLEngine の生成 C の SHA-256** | **変わった**: `feb5b9087f8d7a8a1282900212d7bd1f5b6071b0326e7149642f98c168e8080f` → **`170bd18fe2a365de47068e85ffe1c2e1a3f04f912db7e3335f40f6eecaecca43`**（215,709,232 → 215,712,039 B）。旧 C との `diff` は応答表の 5 か所（プロトタイプ 11 行、`fle_dispatch_table` の末尾 11 行、`fle_dispatch_count` 59565 → 59576、`fle_resp_table` の末尾 11 行、`fle_resp_count` 268 → 279。追加 35 行、変更 2 行）だけで、**変わった関数 0 個**。functions 59,297、bad_nodes 0、unsupported_total 525 は不変。今後の基準は `170bd18f…` |
| 単独構成の a4（runtime を最初から作り直した） | **145/942**、functions 821、entries 15,685、停止 `LoadLibraryW "dsp_ippv2_x64.dll"`。基準と一致。2 回同一 |
| **遅延構成（診断）の a4**（FLEngine + 縮小 dsp_ippv2 + 遅延モジュール。gdb の仮の値なし、runtime の応答追加なし）。**この数は A4 の進捗ではない** | init_count **159/942**、last unit 158（proc `0x66d1a0`）、started 146 / completed 145、functions 978、entries 27,134。2 回同一。**次の停止**: `fle: unsupported instruction at 0x593450`（`f_593450` = `shell32.dll!SHGetFolderPathW` の thunk。`SHGetFolderPathW(NULL, 0x14 = CSIDL_FONTS, NULL, 0, buf)`。呼び元は QFC のフォント設定 `f_668d30` ← `f_66d050`（unit 158））。分類: 未応答の import。遅延モジュールの export は停止まで 1 回も呼ばれていない。直していない |
| 実在モジュール優先（3 モジュール: Blend2D を実在モジュール、基底 0x4700000、種 58 個） | 150/942、停止 `0x489f940`（`f_489f7d0`）、functions 1,069、entries 27,333。決定 0094 の測定と同一。2 回同一 |

## 解釈が分かれた点

決定の内容は変えていない。**実装できなかった項目は無い**。

1. **N の数え方**: 決定の「N は 33 と 34」は 1 始まり（決定 0015、0088）。表の index では 32 と 33。ハンドルは `0x7FF800210000`、`0x7FF800220000`。
2. **停止メッセージの `<module>` と引数の書式**: 決定の文面は `<module>!<export>` とだけある。`<module>` は `fle_modules[]` の綴り（小文字、拡張子つき）、`<export>` は export 名のまま、引数は `RCX=0x… RDX=0x… R8=0x… R9=0x…` を空白で並べて文の後に足した。1 行（改行は末尾の 1 つ）。
3. **QFC の export 全部 10 名に `dbkFCallWrapperAddr`（`.bss` の変数）を含める**: 決定は「export 全部」。この名前の `GetProcAddress` は実機ならコードではなく変数のアドレスを返す。ここでは他の 9 名と同じ停止関数の擬似 VA を返す。FLEngine は引かない（QFC 調査 §1 の 9）ので、実行では起きない見込み（未確認）。
4. **実在モジュール優先の実装箇所**: LoadLibrary 系と `GetProcAddress`（実在モジュールのハンドルは仮想ハンドルの検査より先に引く）は既存の順序で既に実在が優先だったので変更不要。必要だったのは (a) `GetModuleHandleW／A`（表を先に引くため、遅延モジュールの名前で実在に回す分岐）と (b) `fle_realmod.c` のテーブル検査（仮想モジュールの名前を拒否するため、遅延モジュールを除く）の 2 か所。決定が挙げた `fle_load.c` は変更していない。
5. **`GetModuleHandle` は LoadLibrary の前から遅延モジュールを「ある」と答える**: 決定の 4（仮想モジュールと同じ規則）に従った。実機では未ロードの DLL は見つからない（限界）。
6. **実在モジュールが登録されたとき、遅延モジュールのハンドル（N = 33）の行き先**: LoadLibrary／GetModuleHandle は実在の基底を返すので、遅延ハンドルは誰にも渡されない。`GetProcAddress(0x7FF800210000, …)` を直接呼べば遅延モジュールの行が当たるが、通常の経路では起きない。実在モジュールを登録したら、遅延モジュールの表の 2 名と応答表の 11 行を外す（撤回条件 (b)）。
7. **FLEngine の生成 C の SHA の基準の更新**: 決定の 7 のとおり変わる。基準の SHA を固定するテストは無い（決定 0091、0092 のときと同じ）。`gen_two.sh` の期待値 `EXPECT_FLE_SHA` は呼び出しで渡した（スクリプトは無変更）。

## 限界

- **遅延構成の a4 は A4 の進捗ではない**。DllMain の副作用（Blend2D と QFC の初期化）が省かれ、実機と同じ挙動とは主張できない。撤回条件 (a)（ゲストが両 DLL のメモリを直接読む）は調べていない。
- 遅延モジュールの export を FLEngine の実行で呼ぶ段階には届いていない（停止は `SHGetFolderPathW`）。停止メッセージの発火は単体テストと合成ドライバで確認した。QFC の `FontCache_AcquireFont` の使い手は静的な読みで unit 245 以降から届く（QFC 調査 §5.3。読み）ので、測定を進めると最初の遅延モジュールの停止として現れる可能性がある（未確認）。
- 実機の Windows での挙動は未確認（ホストは Linux）。実機で `LoadLibraryW("blend2d.dll")` が成功するか（撤回条件 (c)）は未確認。`GetModuleHandle` が LoadLibrary 前から成功する点は実機と違う。
- `GetModuleFileNameW`、`LoadResource` 系などで遅延モジュールのハンドルを渡すと、既存の「仮想モジュールのハンドルは対象外」の名前付き停止になる（読み。今回は届いていない）。
- 応答表の行を末尾に足したので、**別の決定が先に末尾へ行を足していると、この 11 行の擬似 VA が動く**。`deferred.rs` の `ROWS_BEFORE`（268）がそれを検出する。
- 母集団は `blend2d.dll` と `quickfontcache_x64.dll` の 2 つ。他の同梱 DLL 11 名（決定 0086）は扱っていない。
- 並行して走る他の作業（census の停止経路のフックなど）と `fle_modules.c` で重ならないことは、この作業では確認していない（スナップショットは `0ddcbf1` から）。
