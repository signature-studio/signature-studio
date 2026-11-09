# ILTools_x64.dll を実在モジュールとして載せたときの DllMain(ATTACH) の測定（census の中。調べるだけで、決定は 0104）

実施日: 2026-10-27（JST）。土台: `lift/out-git/integ1`（HEAD a951ecc の `git archive` に census 0097、遅延モジュール 0096、harness-int の 3 パッチを当てたもの。未コミット）のコピー `lift/out-wallI/base`。変更後は `lift/out-wallI/src`、差分は `lift/out-wallI/patches/wallI.diff`。決定は `0104-iltools-realmodule.md`。

## 0. 凡例

- **実測** = このセッションでコマンドを実行して得た値。出力は `lift/out-wallI/` の下（`census/cN/`、`lean/itNN/`、`work/`、`verify/`）。
- **読み** = 逆アセンブル（capstone）とソースを読んだ結果。実行していない。
- **未確認** = そのとおり。
- 「census」は決定 0097 の診断（unit を放棄して続行する）。**census の数は A4 の進捗ではない**。「unit N」は InitTable の 0 始まりの index。
- 構成: FLEngine（0x400000）+ 縮小 dsp_ippv2（0x2000000、種 452）+ ILTools（0x6000000）。Blend2D と QFC は実在モジュールとして載せていない（遅延モジュール 0096 が答える）。

## 1. 要約

| # | 項目 | 結果 | 区分 |
|---|---|---|---|
| 1 | 基底 | 0x6000000（測定用。決定にするなら仮）。`fe-pe rebase` の往復（0x400000 → 0x6000000 → 0x400000）は原本と SHA-256 一致（`20409dcb…0f95`）。リベース版の SHA-256 は `1ca9b793c395d94baa9d8df01d3cb803589fa4e6fb4844976bee35d33d17f04e` | 実測 |
| 2 | lift（`--pdata` のみ） | functions **12,526**、closure_instructions 400,735、**bad_nodes 0**、tail_calls 164、gotos 38,093、C 35,156,379 B、unsupported_total 352（IAT の `jmp` 350、未解決の間接 `jmp` 2）、jumptabs 0、msvc_tables 0。0.5 秒（release） | 実測 |
| 3 | lift（`--pdata` + 注釈 1 + 種 400） | functions **12,930**、closure_instructions 403,636、**bad_nodes 0**、jumptabs 1、msvc_tables 0、msvc_rejected 0、unsupported_total 866（IAT の `jmp` 350、未解決の間接 `jmp` 132、Int3 72、Int1 66、Outsb 27、`ret imm16` 22 ほか）。C 35,606,295 B | 実測 |
| 4 | strict の a4（dsp + ILTools、census なし） | init_count **159 / 942**、functions 978、entries 27,134、停止 `fle: unsupported instruction at 0x593450`（`shell32!SHGetFolderPathW` の IAT thunk。unit 158）。**ILTools には届かない**（遅延モジュールがあるので unit 149・158 を通る。決定 0096 の測定と数が同じ） | 実測 |
| 5 | census（dsp + ILTools） | 基準（ILTools 入りの土台）: ok 878 / stop 17 / fault 0 / hang 0 / empty 46 / noproc 1。**unit 280 は `LoadLibraryW "iltools_x64.dll"` に成功して ILTools の DllMain(ATTACH) に入る**。以下の壁を順に直した（§2）。最後の census（it07）も ok 878 / stop 17 | 実測 |
| 6 | unit 659 | `LoadLibraryW "iltools_x64.dll"`（unit 280 が放棄されたので状態は ST_LOADING のまま、参照 +1、DllMain は再び入らない）→ `GetProcAddress("CreateCursorFromFont")` が**実在の export**（0x6262b60）を返し、ゲストがそれを呼ぶ（スタックの caller2 が 0x6262b60）→ その中の `GetSystemMetrics(13 = SM_CXCURSOR)` で停止（決定 0057）。`GetProcAddress(NULL, …)` の abort には**当たらない** | 実測 |
| 7 | 止まった壁 | 8 件目: `fle: unsupported instruction at 0x605ddaa`。`jmp qword ptr [rax+0x10]`（Delphi のインターフェース呼び出しの thunk。`add rcx,-0x10; mov rax,[rcx]; jmp [rax+0x10]`）。`fe-lift` の `tailjmp.rs` は `jmp [mem]` を扱わない（決定 0036 W1 の記述）。この形は種 400 個の閉包に 131 か所ある（unsupported の「Jmp: indirect target unresolved」は、`--pdata` のみで 2、注釈 1 件を足して 1、種 400 個を足して 132）。**新しい形を設計しない規則に従って止めた** | 実測、形の名前は読み |

## 2. 壁の一覧（census の中で ILTools の ATTACH が止まった順）

unit 280（proc 0x816010）の停止。各行は「census を走らせる → 最初の壁 → 最小の修正 → 再実行」。`it*` は `lift/out-wallI/lean/` の反復ビルド、`c*` は `lift/out-wallI/census/` の census 出力。entries は unit 280 の関数入口数（ATTACH の到達の目安）。

| # | 反復 | 停止メッセージ（要約） | top（ILTools の関数） | entries | 原因（読み） | 修正 |
|--:|---|---|---|--:|---|---|
| 1 | it00（c2、c1 と同じ） | `GetModuleFileNameW: only the lifted DLL has a path (hModule is not NULL or 0x400000)` | 0x6004c80（IAT thunk） | 336 | Delphi の資源モジュール探索（`f_6013d70` → `f_60155c0`）が自分の hInstance で呼ぶ | `fle_vfs.c`、`fle_realmod.c/h`: 仮の決定 P1 |
| 2 | it01（c3） | `LoadStringW: hModule 0x6000000 is not a module handle of this runtime` | 0x6004c60 | — | 資源 API が 0x400000 の像だけを読む（決定 0025） | `fle_res.c`: 実在モジュールの像の資源を読む（決定、仮ではない） |
| 3 | it02（c4） | `unsupported instruction at 0x600a5c3` | 0x600a580 | — | 塗りつぶしルーチンの `jmp [r10+rdx*8+0x40]`。FLEngine の 0x40d210（同じバイト列。違いは rip 相対と表の中身だけ）には注釈（決定 0036）がある | `jumptab.rs` に `ILTOOLS_6000000`（同じ形の注釈 1 件）、`fe-lift/main.rs` が連結して渡す |
| 4 | it03（c5） | `LoadImageW: hInst is neither NULL nor the lifted DLL: not answered (decision 0061)` | 0x601e570 | 7,459 | VCL の `TScreen` のカーソル読み込み（`f_6295290`: `LoadImageW(HInstance, name, IMAGE_CURSOR, 32, 32, LR_SHARED)`） | `fle_loadimage.c/h`、`fle_res.c/h`: 実在モジュールの像の `RT_GROUP_CURSOR` を調べ、ハンドルは仮の決定 P2 |
| 5 | it04（c6） | `GetClassInfoW: hInstance is not the lifted DLL (0x400000): not answered` | 0x601e0e0 | — | ウィンドウクラスの所有者 hInstance | `fle_class.c/h`: 所有者に実在モジュールの基底を認める |
| 6 | it05（c7） | `CreateWindowExW: hInstance is not the lifted DLL (0x400000): not answered (decision 0071)` | 0x601eb20 | — | 同上 | `fle_win.c`: 同じ判定 |
| 7 | it06（c8） | `indirect call to 0x605dda3 is not a known function entry` | 0x610b460（呼び元） | 9,695 | `.pdata` に無く、`.pdata` の閉包が覆わない reloc 先の thunk（`add rcx,-0x10; mov rax,[rcx]; jmp [rax+8]` の一族） | 種 400 個（`lift/harness/seeds/iltools-6000000-seeds400.txt`）。規則は決定 0104 の「種」 |
| 8 | it07（c9） | `unsupported instruction at 0x605ddaa` | 0x605dda3 | 9,696 | 上の thunk の `jmp [rax+0x10]`。`tailjmp.rs` は `jmp [mem]` を扱わない | **直さない**（新しい形。FLEngine に同じ未解決の `jmp` が 1 か所あり、扱えば FLEngine の C が変わる）。記録して止めた |

- 直したのは 7 件（上限 12 件の内）。応答表（`imports_table.rs`）の行は**足していない**（FLEngine の生成 C は土台と 1 バイトも違わない。下の §4）。
- 反復 it00〜it07 のうち、行を足さなかったので FLEngine と dsp_ippv2 の生成 C とオブジェクトは土台のものをそのまま使えた（`lean.sh`。実測: it00 の census は土台の census と時間の列を除いて同一）。ILTools の C だけを作り直した。
- 壁の 2〜6 は、どれも「ゲストが 0x400000（FLEngine の hInstance）だけを想定していた API に、ILTools が自分の hInstance（基底）を渡した」もの。ILTools は FLEngine と同じ Delphi の RTL と VCL を持つ（読み）ので、同じ型の壁が続くと考えられる。

## 2b. 静的な読みで ILTools に届く他の unit（実測。c9）

`loadlibrary-names-classification.md` は ILTools の `LoadLibraryW` に unit 245、279、280、647、659 が静的に届くと読んでいた（`bundled-dll-inventory.md` §6.3 は 245、279、647 で止まらない理由を未確認としていた）。census（c9）の実測: unit 280 と 659 だけが ILTools を使う（上）。**unit 279 と 647 は ok で、関数入口 2 回（1 関数）で戻る**（ILTools に入らない）。unit 245 は ILTools の前に `RaiseException`（例外コード 0xc06d007e。135 入口）で止まる。これらの unit が `LoadLibraryW "iltools_x64.dll"` を呼ばない理由は**未確認**（読み: 呼び出しに届く前の条件分岐。調べていない）。

## 3. 8 件目の先（未測定）

8 件目より先は見えていない（ATTACH はそこで放棄される）。静的に分かること（読み）:

- ILTools の起動表は unit 283 個、runnable 231（`fe-pe pe-init-table`。実測、`bundled-dll-inventory.md` §7.1）。ATTACH はこの Delphi の InitUnits を自分で回す。**FLEngine の A4 と同じ形の問題（数百の unit）が ILTools の中にもある**。census の 7 件で通ったのは、その最初の部分（資源、カーソル、ウィンドウクラス）にすぎない。
- 応答表に行が無い import は 346 名（`bundled-dll-inventory.md` §7.2）。今回の 7 件では、行の無い import には**1 度も当たらなかった**（壁の原因が 8 件とも runtime の hInstance の判定か lift の形か種だった）。
- unit 280 が完了しないと、`CreateCursorFromFont` を unit 659 が呼ぶ前提（ILTools が初期化済み）は成り立たない。unit 659 で見えたのは**初期化の済んでいない ILTools** の export の呼び出し（上の §1 の 6）。

## 4. FLEngine の生成 C（実測）

- 変更後の `fe-lift`（release）で `lift FLEngine_x64.dll --pdata`: functions 59,297、closure_instructions 2,528,030、bad_nodes 0、unsupported_total 525、jumptabs 4。生成 C の SHA-256 **`170bd18fe2a365de47068e85ffe1c2e1a3f04f912db7e3335f40f6eecaecca43`**（215,712,039 B）。土台（遅延モジュールの行を足した木）と同じ値。表の部分（`fle-tables.txt`）も `cmp` 一致。
- 土台の値は HEAD a951ecc の `feb5b908…`（215,709,232 B）と違う。差は遅延モジュール（決定 0096）の応答表の行だけ（`diff` の 37 行: `fle_imp_deferred_*` の宣言 11、表の行 11、名前の行 11、`fle_dispatch_count` 59,565 → 59,576、`fle_resp_count` 268 → 279。関数本体の差は 0）。
- つまり**今回の変更は FLEngine の生成 C を 1 バイトも変えていない**（応答表の行も足していない）。

## 5. 作ったもの

- `lift/out-wallI/lean.sh`（反復ビルド: ILTools の C を作り直し、FLEngine と dsp_ippv2 は表だけ作り直して統合、実行時ライブラリを作り直し、`--strip-debug` でリンク）、`census.sh`（census の実行）、`work/py/`（`tables_only.py`: 標準入力の `fe-lift` の C から表の部分を出す。`classify_reloc.py`、`refine_seeds.py`: 種の絞り込み、`disasm_va.py`）。測定の道具で、パッチには入れていない（`refine_seeds.py` と種のファイルだけは `lift/harness/seeds/` としてパッチに入れた）。
- 反復ごとのビルドは `lean/it00`〜`it07`、census の出力は `census/c1`〜`c9`、strict の a4 は `run2/`（`a4-multi.sh` の出力）。
