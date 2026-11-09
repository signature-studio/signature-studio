# 壁 B（Blend2D の DllMain(ATTACH) を通す）の測定記録

状態: **骨子（作業の途中で保存。以後の追記で埋める）**。凡例: **実測** = コマンドの出力を見た値、**読み** = 逆アセンブル・ソース・文書を読んだ結果で実行して確かめていないもの、**未確認** = 確かめていないこと。

## 1. 基準と最初の結果（実測）

| # | 項目 | 結果 |
|---|---|---|
| 1 | 木 | `lift/out-wallB/base` = `git archive HEAD`（5002d98）+ 決定 0095 の作業ツリー差分（`git diff HEAD -- lift`、SHA-256 `11eebc57f8867fd9e362c0f44d2c049f98c1c9716c27697c1885b83a097d980e`）+ `harness-int.diff`。`src` = 複製（編集用） |
| 2 | 3 モジュール構成の a4（`run-0`、`src` を変えない状態） | **init_count 159 / 942**、last unit 158（proc 0x66d1a0）、functions **1,106**、entries **27,842**、停止 `LoadLibraryW "quickfontcache_x64.dll"`（外部モジュールは M2。名前付き停止） |
| 3 | HEAD の基準（0095 なし）| init_count 150、functions 1,069、entries 27,333、停止 `indirect call to 0x489f940`（`harness-int` の文書 §4.3 と同じ） |
| 4 | Blend2D の ATTACH | 戻った。読み取り専用 gdb（`run/g0-imports.txt`）: `call_dllmain entered idx=1 reason=1` → ATTACH 内の import 188 回（通し番号 1526〜1713）→ `DLLMAIN-RETURN reason=1: call_dllmain returned 0x1` → `fle_realmod.c:144` 到達 → `LoadLibraryW` が **0x4700000** を返す → `GetProcAddress(0x4700000, "AggCreateInstance")` が **0x4759a60** を返す（通し番号 1717）→ unit 150 以降へ |

補足（この文書の最初の版を書いた時点との違い）: HEAD は `5002d98` から `a951ecc`（決定 0095 のコミット）に進み、以後の差分はこちらを基準にした（`base2` = `a951ecc` の `git archive` + `harness-int.diff`）。最初の測定（`run-0`）に使った 0095 は作業ツリーのスナップショットで、`landings_in_swapped_idioms` の条件に `if false &&` が入った途中の版だった。コミットされた版（`false &&` なし）で測り直し（`run-10`）、**同じ結果（3 モジュール 159／942、functions 1,106、entries 27,842）**。

## 2. 最終の測定（実測。`run-10`、4 構成を同じ OUT で。`EXPECT_FLE_SHA=skip`）

| 構成 | init_count | last unit | functions | entries | 停止 |
|---|---|---|---|---|---|
| 単独（FLEngine） | **145／942** | 144（proc 0x65c540） | 821 | 15,685 | `LoadLibraryW "dsp_ippv2_x64.dll"`（不変） |
| 2 モジュール（+ 縮小 dsp、0x2000000） | **150／942** | 149（proc 0x664260） | 943 | 26,585 | `LoadLibraryW "blend2d.dll"`（不変） |
| 3 モジュール（+ Blend2D、0x4700000、種 58） | **159／942** | 158（proc 0x66d1a0、in_progress） | 1,106 | 27,842 | `LoadLibraryW "quickfontcache_x64.dll"`。**HEAD 単独（0095 なし）は 150、1,069、27,333、`indirect call to 0x489f940`**。150→159 は 0095 の効果で、この作業の壁ではない |
| 4 モジュール（+ QFC 0x1d00000、+ freetype 0x4b00000、種 160） | **159／942** | 158（in_progress） | 1,806 | 76,755 | `SystemParametersInfoW: uiAction 0x200a is not answered`。**init_count は動かない** |

ログ: `log/run-10-{0,1,2,4}.stdout`、`run-10/runs/*/a4/*-native.txt`。読み取り専用の gdb（`gdb/ro_imports3.py`）の import の記録: 3 モジュールで `run/g0-imports.txt`、4 モジュール（最終）で `run/g4-imports.txt`（5,210 行）。

### 2.1 Blend2D の ATTACH（実測、`run/g0-imports.txt`）

`call_dllmain entered idx=1 reason=1`（行 2012）→ `BLEND2D DllMain entered (f_489e584) rcx=0x4700000 rdx=1 r8=0`（import 1,525 回の後）→ ATTACH の中の import 188 回（通し番号 1526〜1713。`__security_init_cookie` の 5 回、API セット名の `LoadLibraryExW` と `GetProcAddress`、`InitializeCriticalSectionAndSpinCount`、`TlsAlloc`／`TlsSetValue`、`GetModuleHandleA`、`GetSystemInfo`、SRW ロック、`HeapAlloc`／`HeapFree` ほか。**名前付き停止の import なし**）→ `DLLMAIN-RETURN reason=1: call_dllmain returned 0x1` → `fle_realmod.c:144` 到達 → `LoadLibraryW` が **0x4700000** → `GetProcAddress(rcx=0x4700000, "AggCreateInstance")` が **0x4759a60**（通し番号 1717）→ `RegCreateKeyExW`（unit 153 の経路）→ … → `LoadLibraryW "QuickFontCache_x64.dll"`（通し番号 1747）。4 モジュールの最終の記録では、freetype の DllMain も戻り（行 5071、0x1）、QFC の DllMain（`f_1e9c1f0`、行 2270 から）は SPI の停止まで進んだ（戻らない）。

## 3. 壁の表

`0102-blend2d-attach-walls.md` の「壁の表」に、壁ごと（停止の全文・読み・仕様・決定・撤回条件・テスト・functions／entries の前後）を書いた。要約:

| # | 壁 | 種類 | 直したもの | functions／entries（直した後の停止まで） |
|---|---|---|---|---|
| — | Blend2D の `jmp r9`（0x489f8fb） | MSVC switch | 決定 0095 | 1,106／27,842 |
| 1 | `GetModuleFileNameW`（実在モジュール） | 未応答の分岐（0082 D6） | runtime | 1,233／28,201 |
| 2 | `LoadStringW`（実在モジュール） | 未応答の分岐 | runtime | 1,337／29,676 |
| 3 | `jmp [r10+rdx*8+0x40]`（QFC 0x40c163） | 間接 jmp | fe-decode | 1,635／44,791 |
| 4 | `LoadLibraryW "freetype_x64.dll"` | 実在モジュールの追加 | 測定構成 | 1,639／44,795 |
| 5 | `GetVersionExA` | 未応答 API | runtime + 表 | 1,640／44,796 |
| 6 | `HeapCreate`／`HeapSetInformation` | 未応答 API（CRT 初期化） | runtime + 表 | 1,644／44,800 |
| 7 | `GetStartupInfoA`／`GetModuleFileNameA`（実行された）、`GetEnvironmentStrings`／`FreeEnvironmentStringsA`（実行されていない先取り） | 未応答 API（CRT 初期化） | runtime + 表 | （6 と同時） |
| 8 | `EncodePointer`／`DecodePointer` | 名前付き停止（0078）の置き換え | runtime + 表 | 1,651／44,807 |
| 9 | `ret 0` | 未対応命令 | fe-lift | 1,668／44,872 |
| 10 | `SetHandleCount` | 未応答 API（CRT 初期化） | runtime + 表 | 1,696／45,179 |
| 11 | `HeapSize` | 名前付き停止（0078）の置き換え | runtime | 1,751／45,869 |
| 12 | `ft_alloc` の間接 call（`.pdata` に無い葉関数） | 間接 call の解決（種） | 測定構成（種 160） | 1,792／46,167 |
| 13 | SPI `0x200C` | 未応答 API | runtime | 1,806／76,755 |
| — | SPI `0x200A`（最終） | 値が決まらない（止まる条件 (d)） | しない | 停止 |

分類の内訳: 未応答 API／runtime の分岐 9（1、2、5、6、7、8、10、11、13）、間接 jmp／call 2（3、12）、未対応命令 1（9）、実在モジュールの追加 1（4）。TLS の設計、JIT、SEH は**一度も必要にならなかった**（止まる条件 (a)(b)(c) に当たらない）。

## 4. FLEngine の生成 C（旧 C との差、実測）

旧 C: SHA-256 `feb5b9087f8d7a8a1282900212d7bd1f5b6071b0326e7149642f98c168e8080f`（base2 の fe-lift。functions 59,297、`bad_nodes` 0、`unsupported_total` 525）。新 C: `d83dfedb3c1370a802da173b9ae0cc7e587a63c7cbf5c6d62b37bf4a3ea9d963`（functions 59,297、`bad_nodes` 0、`unsupported_total` 524）。`diff` は 33 行:

- 応答表の追加 32 行: 宣言 9、`fle_dispatch_table` 9（擬似 VA 0x7ff9000010d0〜0x7ff900001150）、`fle_resp_table` 9、`fle_dispatch_count` 59,565→59,574、`fle_resp_count` 268→277、の合計。
- **関数本体の変化 1 行**: `L_423910`（`jmp [IAT]` の thunk = `HeapCreate`）が `fle_unsupported(0x423910)` から `fle_imp_HeapCreate()` の呼びに変わった（`HeapCreate` の行を足したため）。このゲストの経路では呼ばれていない。

## 5. DLL ごとの統計の前後（実測。同じリベース済み DLL と種を、base2 の fe-lift と今回の fe-lift に渡した）

| DLL | functions | closure の命令 | `msvc_tables` | `jumptabs` | `bad_nodes` | `unsupported_total` |
|---|---|---|---|---|---|---|
| FLEngine（`--pdata`） | 59,297 → 59,297 | 2,528,030 → 2,528,030 | 0 → 0 | 4 → 4 | 0 → 0 | 525 → 524 |
| dsp_ippv2（縮小、種 452） | 8,030 → 8,030 | 3,038,952 → 3,038,952 | 4 → 4 | 438 → 438 | 0 → 0 | 391,396 → 391,396 |
| Blend2D（`--pdata`、種 58） | 2,067 → 2,067 | 405,705 → 405,705 | 31 → 31 | 31 → 31 | 0 → 0 | 599 → 598（`ret 0` の 1 関数） |
| QFC（`--pdata`） | 6,712 → 6,712 | 209,259 → 209,269 | 0 → 0 | 0 → 1（`inline_tables`） | 0 → 0 | 104 → 102 |
| freetype（`--pdata`、種 160） | 1,625 → 1,625 | 99,984 → 99,984 | 17 → 17 | 17 → 17 | 0 → 0 | 176 → 75（`ret 0` が 101 か所） |

（freetype の種なしは functions 1,488、`unsupported_total` 161。種 160 で 1,625。）

## 6. テスト（正直な記録）

- 追加したテスト: `fe-lift/tests/wall_b.rs` 13 件、`fe-decode/tests/inline_table.rs` 6 件、`fe-lift/tests/realmod.rs` に 1 件追加 + 1 件更新、`k32_dsp.rs` の表の更新（0102 の「テスト」の節）。個別の実行で、`wall_b` 13／13、`inline_table` 6／6（実 QFC のテストは `QFC_PE` を指定して実行。作業ツリーの `lift/out-qfc/` には無い `git archive` の木では SKIPPED）が通った。`realmod` の 2 件（W1、W2）は**まだ実行していない**（`windres` と mingw を使う長い構築。ディスクが満杯で実行できなかった）。
- **全体テストは完走していない**。1 回目（`test-ws.txt`、949 行で切れている）も 2 回目（`test-ws-2.txt`、約 920 行で切れている）も、`locale` の途中でディスクが満杯になり（`No space left on device`、リンカの `デバイスに空き領域がありません`）、以降の binary（`mmx` から後）は実行されていない。**この結果は基準（98 binaries／1,228 passed／0 failed）との比較には使えない**。
- 2 回目で最後まで走った binary は 42 個が ok（passed の合計は約 499。私が `test result` の行から足した数で、機械で集計していない）、**2 個が失敗**（fe-decode の `jt_cpu` 2 件: `a_jump_with_a_displacement_is_not_a_stub`、`dsp_ippv2_has_434_stubs`。fe-lift の `jt_cpu` 2 件: `real_stub_selector_k_reaches_table_k_natively_and_lifted`、`real_stub_selector_outside_the_table_stops_by_name`）。**原因は私の変更**: W3 の認識が `lea B,[rip+k]; jmp [B+I*8+d]` の形のすべてに当たり、表が関数の外にある site を `jumptab_rejected` に報告した（これらのテストは実 dsp_ippv2 で `jumptab_rejected` が空であることを確かめる）。**直した**: 表が関数の外、再配置なし、`.pdata` なし、は認識せず何も報告しない（`resolve_inline` は `Option`）。直した後の確認: fe-decode の `inline_table` 6／6 と `jt_cpu` 6／6（`log/test-fix-2.txt`）。fe-lift の `jt_cpu` は**直した後の実行ができなかった**（ディスク満杯で出力が空）。
- 3 回目（`test-ws-3.txt`、`resolve_inline` を直した後、HEAD は docs だけ進んだ `f1bcba7`）は、開始直後にディスクが満杯になり、`a2a_remaining` 以降の binary の大半が 0.00 秒で失敗した（C の作業ファイルを作れない）。**無効**。fe-decode の binary（lib、adjustor、dataflow、dump_tmp、extra_seed、inline_table、jt_cpu、jumptab、msvc_switch 78）は 3 回目も通っている（passed のみ確認）。
- 固定の数を決め打ちした既存テストで更新したもの: `k32_dsp.rs`（`ANSWERED` 13→15、`STOPPED` 20→18。EncodePointer と HeapSize の 2 件が名前付き停止ではなくなった）、`realmod.rs` の GetModuleFileNameW のテスト（停止の確認 → 49 文字のパスの確認）。
- 直した後にも検証できていないテスト: fe-lift の `jt_cpu`、`realmod`、`k32_dsp`、`imports_table`、`locale` 以降のすべて。

## 7. 止まった条件と選択肢

**条件 (d)**: `SystemParametersInfoW(SPI_GETFONTSMOOTHINGTYPE = 0x200A)`。ページ（Learn、2026-10-27）は値の候補 `FE_FONTSMOOTHINGSTANDARD`（1）と `FE_FONTSMOOTHINGCLEARTYPE`（2）を挙げるだけで、既定を書かない。機械の設定なので、仕様からは決まらない。

選択肢:
1. **フォント平滑化の設定一式をまとめて仮の値で決める**（有効か（`0x4A`）、種類（`0x200A`）、コントラスト（`0x200C` = 1400、決定済み）、向き（`0x2012`））。決定 0058／0066 の「仮の仮想マシン設定」と同じ種類。QFC がどれを読むかは、停止の度に 1 つずつ分かる。作業量: 小（行を足すだけ）だが、値の選択が PM の判断。
2. 答えずに止めたままにして、QFC を別レーン（census／遅延モジュール）の扱いにする。
3. QFC を「無い」と定義する案は、0086 と矛盾する（同梱されている）。

その先に見える壁（読み）: QFC は `dsp_ippv2.dll`（`dsp_ippv2_x64.dll` との名前の違いがある可能性。未確認）、Elastique、Python の読み込みも持つとされる（`quickfontcache-lift-feasibility.md`）。`fle_realmod` の上限は 4 で、dsp、Blend2D、QFC、freetype で使い切っている。

## 8. 限界

- init_count の変化だけを進捗と数える: **単独 145、2 モジュール 150、3 モジュール 159（0095 の効果）、4 モジュール 159**。QFC と freetype を載せた 4 モジュール構成は、unit 158 の中で進んだだけ。
- 4 モジュール構成は測定用の基底と種（QFC 0x1d00000、freetype 0x4b00000、freetype の種 160）の上で、決定ではない。
- 先取りの 2 API（`GetEnvironmentStrings`、`FreeEnvironmentStringsA`）は実行されなかった。`LockResource` の実在モジュール対応も実行されていない。
- `inline_table` の run の長さは再配置の連続で、ガードから導いていない（過大側）。
- native のみ。wasm、ブラウザは未測定。ホストの実ファイル・レジストリ・ロケールはゲストに見せていない（freetype の抽出はインストーラの読み取りで、結果の DLL は lift の入力にしただけ）。
- **後始末が必要なもの**: ディスクが満杯になったため、私の作業物の一部を `/dev/shm` に移した（`/dev/shm/wallB-spill`（約 1.2 GB。`fast/w1`、`fast/w2` の移動先）、`/dev/shm/wallB-tests`（約 158 MB。テストの作業ディレクトリ `src/lift/out-*` の一部。移しきれなかったものは作業ディレクトリに残り、`/dev/shm` に途中のコピーがある）。`src/lift/out-*` の一部は `/dev/shm` への symlink）。`rm` は禁止なので削除していない。`lift/out-wallB/` の `run-0`〜`run-10`（各 3〜7 GB。`run-10` が最終）、`run-1/v-t3956c8cbce29`（再起動で壊れたビルド）は不要になった中間物。

## 9. 作ったファイル

`lift/out-wallB/`: `base/`（5002d98 + 0095 のスナップショット + harness）、`base2/`（a951ecc + harness。差分の基準）、`src/`（編集した木）、`patches/wallB-v2.diff`（最終、1,967 行、27 ファイル）、`patches/wallB.diff`（古い版）、`patches/0095-worktree.diff`／`0095-committed.diff`、`docs-new/0102-blend2d-attach-walls.md`、`docs-new/README-0102-line.txt`、`docs-new/wall-b-measurement.md`（これ）、`fast.sh`、`gdb/`、`analysis/lea_orphans.py`、`seeds/`（freetype の種）、`ext/freetype_x64.dll`（インストーラから抽出）、`stats/`（前後の統計と生成 C）、`log/`、`run/`（gdb の記録）、`run-0`〜`run-10`、`test-ws.txt`／`test-ws-2.txt`（切れた全体テスト）。
