# SystemParametersInfoW の呼び出し元の調査と、仮想 SPI の実装後の harness の再実行の記録（決定 0058）

実行日: 2026-10-21。作業ディレクトリ `signature-studio-develop`、HEAD fd364d4。DLL は `binaries/win64/FLEngine_x64.dll`。決定は `docs/decisions/0058-virtual-spi.md`。「実測」はコマンドの出力、「読み」は `lift/objdump-text.txt` の逆アセンブルの静的な読み、「推測」は未検証。成果物は `lift/out-spi/`（gitignore 済み。`TMPDIR=lift/out-spi/tmp`、`EM_CACHE=lift/out-spi/emcache`、`CARGO_TARGET_DIR=lift/out-spi/target`）。補助スクリプトは `lift/out-spi/work/`（`sites.py` = 呼び出し点と直前の ecx／edx／r8／r9、`reach.py` = 静的な到達範囲、`mkmut.py` = 変異、`runmut.sh`、`learn-spi-page.md` = 取得した Learn のページ全文）。

## 1. 停止点の確認（実測）

停止は `fle: unsupported instruction at 0x4255e0`。0x4255e0 は `ff 25 ea ba 54 01`（`jmp [rip+0x154baea]`）で、IAT スロットは 0x19710d0、pefile の import 表で `user32.dll!SystemParametersInfoW`（実測）。呼び出しスタックは前段の記録（`getsystemmetrics-caller.md` §8）のとおり。開始時点の基準: 関数実行数 574、入口数 13,233、深さ 10、unsupported_total 546。

## 2. 全呼び出し点（読み。静的に追える範囲）

- import 表に `SystemParametersInfoW` は 1 件（スロット 0x19710d0）、**`SystemParametersInfoA` と `SystemParametersInfoForDpi` は無い**（実測: pefile。DLL 全体の文字列検索でも `SystemParametersInfo` は 1 件、ANSI 版は 0 件、UTF-16 は 0 件）。
- スロット 0x19710d0 を指す thunk は **3 つ**（実測: objdump の `# 0x19710d0` が 3 行）: **0x4255d0**、**0x4255e0**、**0x4255f0**（いずれも `jmp [rip+disp]`）。これは依頼文の「thunk 0x4255e0」より多い。**生成器が応答に置き換えるのはスロット単位の `jmp_imp:SystemParametersInfoW` なので、3 つとも応答になる**（実測: `gen-pdata.txt` の `jmp_imp:SystemParametersInfoW 3`）。
- thunk を指す命令は **`call rel32` が 22 か所（0x4255d0 が 18、0x4255e0 が 2、0x4255f0 が 2）、`jmp`／条件ジャンプは 0**（実測: `grep`）。DLL のファイルに 3 つの thunk の VA（4／8 バイトの絶対値と 4 バイトの RVA）もスロットの VA も無い（実測。テスト `nothing_else_in_the_file_points_at_the_thunks_or_the_slot`）ので、関数ポインタの表にこれらを持つ形は無い。ただし**計算したアドレスへの間接呼び出しは追えない**。`GetProcAddress` は import されているが、`SystemParametersInfo` の文字列は import 名の 1 件だけなので、文字列で引く形は無い。
- 22 か所の**すべてが**直前に `mov ecx,imm32`（uiAction）、`xor r9,r9`（fWinIni = 0）を持つ（実測: `sites.py`。テスト `the_guest_call_sites_pass_the_uiactions_the_decision_lists` がバイト列で固定する）。uiAction が動的な呼び出しは **0 か所**。静的な uiAction は **11 種**。

### 2.1 呼び出し点の一覧（uiAction ごと。名前と数値は Learn の表）

| uiAction | 名前 | 呼び出し点 | uiParam | pvParam | 戻り値の使われ方（読める範囲） |
|---|---|---|---|---|---|
| 0x68 | **SPI_GETWHEELSCROLLLINES** | 0x6208ac（f_620880）、0x6209f5（f_6209a0）、0xa0a57b（f_a0a4d0） | 0（`xor edx,edx`） | `lea r8,[rbx+0x14]`（2 か所。`[obj+0x14]` の DWORD）、`lea r8,[rbp+0x2c]`（f_a0a4d0） | f_620880、f_6209a0: **戻り値は見ない**（`call` の直後は `add rsp` か `jmp`）。f_a0a4d0: 戻り値は見ず、`[rbp+0x2c] == 0xffffffff`（`WHEEL_PAGESCROLL`）なら別の計算、そうでなければ `行数 * [rbx+0x514] * 定数` で量を決める（読み。`cvtsi2sd; mulsd`） |
| 0x1016 | SPI_GETTOOLTIPANIMATION | 0x61b8bb（f_61b600）、0xa11c2f（f_a11c00） | 0 | `lea r8,[rbp+0x7c]`／`[rbp+0x2c]` | 出力先 `[..] == 0` なら即座に戻る（0 を返す／何もしない）。非 0 なら 0x1018 を続けて問う |
| 0x1018 | SPI_GETTOOLTIPFADE | 0x61b8d4（f_61b600）、0xa11c4c（f_a11c00） | 0 | 同上 | f_a11c00（0xa11c51〜）: 出力先が非 0 なら 1（フェード）、0 なら 2（スライド）。続けて `[0x1659cd8] == 0` かつ結果が 1 なら 2 に直す（読み） |
| 0x101A | SPI_GETCURSORSHADOW | 0x815514（f_815050） | 0 | `lea r8,[rbp+0x1c8]`（直前に `[rbp+0x1c8] = 0` を書く） | 読み手は追っていない |
| 0x48 | SPI_GETANIMATION | 0x818c8e（f_818c70）、0x86723c（f_867220） | 8（`mov edx,8`／`mov edx,eax`＝8） | `lea r8,[rsp+0x28]`（`ANIMATIONINFO{cbSize=8, iMinAnimate}`。直前に `[rsp+0x28] = 8`） | 戻り値が 0 なら 0 を返し、非 0 なら `[rsp+0x2c]`（iMinAnimate）`!= 0` を返す（読み） |
| 0x49 | **SPI_SETANIMATION** | 0x818cd7（f_818cb0）、0x86728c（f_867260） | 8 | `lea r8,[rbp+0x28]`（`ANIMATIONINFO{cbSize=8, iMinAnimate = 引数}`） | 戻り値は見ない。fWinIni = 0 |
| 0x4B | **SPI_SETFONTSMOOTHING** | 0x7fe328（f_7fe2d0） | 0（`xor edx,edx`＝FALSE） | NULL（`xor r8,r8`） | 戻り値は見ない。呼ぶ条件は、参照カウンタ `[0x15f6e5c]` に 1 を足して 0 になり、かつ `[0x1402818] == 0`（ファイル上の初期値は 0xFFFFFFFF）のとき（読み。書き手は追っていない）。リソース解放（`f_424090` の連続）の後の関数で、フォントスムージングを戻す／切る後始末と読める（推測） |
| 0x1F | SPI_GETICONTITLELOGFONT | 0x866424（f_8663b0） | `[rbp+0x288]`（直前に 0x5C を書く。`sizeof(LOGFONTW)` は 92 = 0x5C） | `lea r8,[rbp+0x22c]` | 戻り値が非 0 なら `f_423fc0(&[rbp+0x22c])`（フォント名の取り出しと読める）を呼ぶ |
| 0x26 | SPI_GETDRAGFULLWINDOWS | 0x9f4c93（f_9f4c60） | 0 | `lea r8,[rbx+0x3c]`（直前に `[rbx+0x28] = 1`） | 戻り値は見ない。`[rbx+0x3c] == 0` なら `f_425140(0)` を呼ぶ |
| 0x29 | SPI_GETNONCLIENTMETRICS | 0x840045（f_840020）、0x866493（f_8663b0）、0x991d3d（f_991cc0） | `f_425a50()` の戻り値（0x1f8 か 0x1f4）、0x991d3d は `[rbp+0x40] = 0x1f8` | `lea r8,[rbp+0x28]`、`[rbp+0x34]`、`[rbp+0x40]`（いずれも `NONCLIENTMETRICS` の `cbSize` を先頭に置く構造体の先頭） | 戻り値が非 0 なら構造体の各フォント（`f_423fc0`）と寸法を使う |
| 0x30 | SPI_GETWORKAREA | 0x8190a6（f_819090）、0x866d1a（f_866cd0。thunk 0x4255f0）、0x8673a5（f_867390。thunk 0x4255f0）、0x11c3cd8（f_11c2d80） | 0 | `mov r8,rbx`（呼び出し元の `RECT*`）、`lea r8,[rbp+0x30]`、`lea r8,[rbp+0x220]` | f_819090 は戻り値が 0（失敗）のとき、`f_865720`（SM_CXSCREEN）と `f_865700`（SM_CYSCREEN）の値で `f_420980`（RECT を `(0, 0, cx, cy)` に設定する形と読める。推測）を呼ぶ。つまり**ゲストは SPI_GETWORKAREA が失敗したら画面全体を使う**（読み）。f_867390 は `RECT` を返す包み |

**ゲストの SET 系は 0x49（2 か所）と 0x4B（1 か所）の 3 か所。`fWinIni` は 22 か所すべてが 0**（プロファイルへの書き込みと `WM_SETTINGCHANGE` の送信をする呼び出しは無い）。0x68 のゲストの形は 3 か所とも `uiParam = 0`、pvParam が有効なポインタ、`fWinIni = 0`。

### 2.2 f_620880 から先の経路（読み）

f_6209a0 は `edx == 0` で f_620860 と f_620880 を、`edx == 0x68`（`sub edx,1; sub edx,0x67; je`）で `SystemParametersInfoW(0x68, 0, &[obj+0x14], 0)` を呼ぶ（`[obj+0x28]` が真で `[obj+0x11]` が非 0 のとき）。0x68 は **Learn で SPI_GETWHEELSCROLLLINES = 0x0068**（実測。§4）。pvParam は `[obj+0x14]`（DWORD。UINT の出力先）、**戻り値は見ない**。`[obj+0x14]` の読み手は追っていない。

## 3. 静的な到達範囲（読み。下限）

`lift/out-sm/out` の生成 C の直接呼び出し（`f_xxxxxx();`）だけを辿った閉包（`reach.py`、`reach.txt`）。生成 C に 3 つの thunk の呼び出しを持つ関数は 19（実測）。

| 根 | 閉包（直接／アドレス定数込み） | 間接呼び出しを含む関数 | 閉包にある SPI の使い手 |
|---|---|---|---|
| f_620640 | 107／107 | 15／15 | f_620880、f_6209a0（0x68 だけ） |
| f_623840 | 610／869 | 73／123 | 直接: f_620880、f_6209a0、**f_8663b0**（0x1F、0x29）。アドレス定数込みで f_867220（0x48）、f_867260（0x49）が加わる |

間接呼び出し（`fle_dispatch(...)`）の先は追えていないので閉包は下限。つまり、次の SPI の停止は、この範囲では **f_8663b0 の 0x1F（SPI_GETICONTITLELOGFONT）か 0x29（SPI_GETNONCLIENTMETRICS）、または f_867220／f_867260 の 0x48／0x49**。

## 4. 一次資料（Microsoft Learn。2026-10-21 取得）

| ページ | 確認した内容 |
|---|---|
| [SystemParametersInfoW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-systemparametersinfow)（`updated_at` 2025-07-01、`ms.date` 2024-03-27。**ページ全文を読んだ**（495 行）。`learn-spi-page.md`） | 書式、パラメータ、戻り値、Remarks。uiAction の表（Accessibility、Desktop、Icon、Input、Menu、Power、Screen saver、Time-out、UI effects、Window の 10 表）が各 SPI_* の**名前と 16 進の数値と pvParam の型**を書く。ゲストが使う 11 種の**名前と数値はすべて表にある**: 0x001F ICONTITLELOGFONT（`LOGFONT`、uiParam はサイズ）、0x0026 DRAGFULLWINDOWS（`BOOL`）、0x0029 NONCLIENTMETRICS（`NONCLIENTMETRICS`、cbSize と uiParam を sizeof）、0x0030 WORKAREA（`RECT`）、0x0048／0x0049 ANIMATION（`ANIMATIONINFO`、cbSize と uiParam を sizeof）、0x004B SETFONTSMOOTHING（「To disable the feature, set *uiParam* to **FALSE**」）、0x0068 WHEELSCROLLLINES（`UINT`。**「The default value is 3」**）、0x1016 TOOLTIPANIMATION（`BOOL`）、0x1018 TOOLTIPFADE（`BOOL`）、0x101A CURSORSHADOW（`BOOL`）。**既定値が書かれているのは 0x68 だけ**（ほかに 0x6C の「3」、0x2008 の 1200、0x70 の 10、0x7E／0x84／0x88 などの閾値があるが、ゲストは使わない）。uiParam と pvParam: 「If not otherwise indicated, you must specify zero／NULL」。fWinIni: 「If a system parameter is being set, specifies whether the user profile is to be updated, and if so, whether WM_SETTINGCHANGE is to be broadcast」。戻り値: 成功で非 0、失敗で 0 で「To get extended error information, call GetLastError」。**どの形が失敗かは書かれていない**。「This API is not DPI aware」。 |

非公式の照合（参考。値の根拠にはしない）: mingw-w64（`/usr/share/mingw-w64/include/winuser.h`、`mingw-w64-common 13.0.0-2ubuntu1`）の `SPI_*` の定義と、ゲストが使う 11 種の数値を比べた。**11 種すべてが一致、不一致 0**（実測: `grep`）。実装が使う 0x68 の数値は Learn の表で確認済みで、mingw の値は使っていない。

## 5. 実装（実測）

- `lift/runtime/fle_spi.h`、`fle_spi.c`（新規。`lift/out-spi/work/` で完成（clang `-Wall -Wextra -Werror` と emcc `-Wall -Wextra -Werror` の両方で警告なし）させてから、ヘッダ、`.c` の順に **1 回で** `lift/runtime/` に置いた。置いた後に編集していない。`cmp` で work と runtime が一致（`fle_spi.h`、`fle_spi.c`））。0x68 に答え、ゲストが使う残り 10 種（GET 8、SET 2）は名前付きで停止し、それ以外は番号で停止する。
- `lift/crates/fe-lift/src/imports_table.rs`: 末尾に 1 行（コメント付きで 2 行）追記（編集の直前に読み直した）。`lib.rs` は変更なし。
- `lift/crates/fe-lift/tests/sysmetrics.rs`: 「SystemParametersInfoW を答えない」という表明（`the_row_is_in_the_table_and_declared_in_the_header`）を外した（コメント付き。0057 のテストは「次の API はこの決定では答えない」と主張していたので、この決定で正しくなくなった）。`gdc.rs` に SPI の表明は無い（変更なし）。
- `lift/crates/fe-lift/tests/spi.rs`（新規、21 件）。

## 6. テスト（実測）

- `cargo test -p fe-lift --test spi`: **21 件すべて ok**（`lift/out-spi/test-spi2.txt`。最初の版（`test-spi.txt`）は「その他の uiAction」を 0..0x100 など約 400 個で試して 460 秒かかったので、代表 40 個余りに絞って 93 秒にした。絞った理由はテストのコメントに書いた）。内訳: 行と宣言、Learn の数値（0x68）、値 3、Learn の表に重複が無いこと、答え（4 バイトだけを書く。前後のガード DWORD は残る）、下位 DWORD、状態を持たない・last error に触れない・戻り番地を壊さない、バッファの前の値に依存しないこと、規則が書かれていない形（pvParam NULL、uiParam ≠ 0、fWinIni ≠ 0）での名前付き停止、ゲストが使う 10 種の名前付き停止、SET 系はどの引数でも止まること、ほかのホイール系の uiAction（0x69、0x6C、0x6D）が番号で止まること、それ以外の番号で止まること、ソースがホストを読まないこと、ヘッダが plain C であること、生成器が thunk を応答に置き換えること、DLL 名と API 名（`SystemParametersInfoA`、`…ForDpi`）が一致の一部であること、実 DLL（3 thunk が同じスロット、ANSI 版が import に無いこと、thunk の絶対 VA がファイルに無いこと、22 か所のバイト列、0x68 の 2 か所の完全なバイト列）、静的な uiAction の全部（11 種）が「答える」か「名前付きで停止」のどちらかに入ること。
- 変異テスト（`lift/out-spi/work/mkmut.py` が `lift/out-spi/mut/m01`〜`m22` に runtime の複製を作り、`FLE_SPI_SRC_DIR` で指す。`runmut.sh`。`lift/out-spi/mut/summary.txt`）: **22 変異すべてがテストの失敗で検出された（生き残り 0）**。失敗件数は 1〜9 件。コンパイルエラーで落ちた変異は 0（`clang_failed_mentions=0`）。内訳: m01 値が 4（6 件）、m02 値が 0（6）、m03 番号が 0x69（9）、m04 戻り値 FALSE（3）、m05 8 バイト書く（5）、m06 2 バイト書く（5）、m07 pvParam NULL を調べない（1）、m08 uiParam を調べない（2）、m09 fWinIni を調べない（2）、m10 uiParam を 64 ビットで見る（1）、m11 uiAction を 64 ビットで見る（1）、m12 fWinIni を 64 ビットで見る（1）、m13 SET を GET の理由で止める（2）、m14 SET_FONTSMOOTHING に答える（3）、m15 未使用の uiAction が FALSE（3）、m16 last error を 0 にする（2）、m17 0x1018 を名前の表から外す（2）、m18 名前の取り違え（2）、m19 0x6C に答える（3）、m20 全部を SET の理由で止める（1）、m21 戻り番地を壊す（2）、m22 GET を SET の理由で止める（1）。**同値変異は作っていない**（各変異が変えるのは、結果が観測できる値・分岐・書き込み幅・理由文のどれか。`mkmut.py` は「ソースが変わったこと」を assert し、それぞれが失敗件数 1 以上で殺されたことを `summary.txt` で確認した）。
- `cargo test --workspace --no-fail-fast`: §8。

## 7. harness の再実行（実測）

生成 C は変わった（3 つの thunk が応答になり、応答表が 1 行増えた）ので `lift/out-spi/out` に新規生成した（`OUT=lift/out-spi bash lift/harness/wasm/build.sh gen split`、次いで `rt gen-c harness link`、native は `build_native.sh`）。`functions 59297`、`bad_nodes 0`、`unsupported_total` **546 → 543**（`jmp_imp:SystemParametersInfoW 3`）。runtime、生成 C の 17 パーツ、harness がすべて `errors=0 warnings=0`（`build-wasm.txt` の 57 行）。ただし `fle_proc.c` に**既存の**警告 1 件（前段の記録と同じ）。`fle_spi.c` は 0 件。

| | native | wasm（node） |
|---|---|---|
| 停止 | `fle: unsupported instruction at 0x4250e0`（SIGABRT、`abort kind: fle_unsupported`） | 同じ |
| 関数実行数（distinct） | **578**（前 574） | 578 |
| 入口数（calls） | **13,245**（前 13,233） | 13,245 |
| 呼び出し深さ | 9（前 10） | 9 |
| unsupported_total | **543**（前 546） | 543 |

- `diff <(grep -v '^calling' run-native.txt) <(grep -v '^calling' run-node.txt)`: **差なし**。stderr も差なし。全体の diff は `calling entry … rsp=` の 1 行だけ（native `0x747baadfffc0`、wasm `0x11c4f420`）。
- 0x4250e0 は **`user32!LoadCursorW` の thunk**（実測: IAT スロット 0x1970e70）。呼び出しスタック（native と wasm で同じ）: `f_4250e0`（guest-return 0x865a06）← `f_8659f0` ← `f_8651a0`（0x86521b）← `f_620cd0`（0x620dea）← `f_623840` ← `f_4120c0` ← `f_412160` ← `f_41d8f0` ← エントリ。f_8659f0 は `LoadCursorW(NULL, 0x7f00)`（`xor ecx,ecx; mov edx,0x7f00`。0x7F00 = IDC_ARROW と読める。**Learn では未確認**）を呼び、結果を `[rbx+0x154]` に保存する。この thunk を呼ぶ箇所は 7 か所（0x6117b5、0x865a01、0x865ae6、0x86618f、0x9f2927、0x9f2959、0xa0e3db）。
- 前の停止点の SPI（0x68）は通過した（呼び出し 0x6208ac は停止点が先に進んだことから、`[obj+0x28]` が真で 0x68 に答えたと読める。gdb では確かめていない）。増分（関数 +4、入口 +12）は、f_620880 の後の初期化（f_6209a0 の残り、f_8651a0、f_8659f0、f_4250e0 など）と整合する。

## 8. 未確認、限界

- 実 Windows との比較はしていない。SystemParametersInfoW が実機で返す値は見ていない。
- 間接呼び出しの先（§2、§3）、`[obj+0x14]`（スクロール行数）の読み手は追えていない。f_a0a4d0 の呼び出し元は追っていない。
- ゲストが使う GET 系 7 種と SET 系 2 種は値を決めていない（一次資料が既定値を書かない。決定 0058）。実行がそれらに届いたときに止まる。
- 0x7F00 が IDC_ARROW であること（LoadCursorW の次の停止点）は Learn で確認していない。

## 9. 触ったファイルと制約

- 変更: `lift/crates/fe-lift/src/imports_table.rs`（2 行追記）、`lift/crates/fe-lift/tests/sysmetrics.rs`（表明の更新）。
- 新規: `lift/runtime/fle_spi.h`、`fle_spi.c`、`lift/crates/fe-lift/tests/spi.rs`、`docs/decisions/0058-virtual-spi.md`、本書、`lift/out-spi/`（gitignore 済み）。
- `rm`、既存ファイルの `>` 上書き、git の変更操作、`/tmp`、コマンド文字列の `sleep`/`timeout`/`nohup` は使っていない。`lift/iat-unsup-closure.txt` には触っていない。emcc は `JOBS=2` 以下。
