# 0112: FLEngine の環境の主張の壁 — kernel32!GetComputerNameExW（unit 830、880）と近傍（GetComputerNameW、version.dll の GetFileVersionInfoW／VerQueryValueW、LocaleNameToLCID、および GetComputerNameExA／GetUserNameW）。すべて仮の決定

状態: **仮の決定（すべての応答）**。依頼は PM（決定番号 0112 は仮の割り当て）。決定と実装は作業者（サブエージェント E1）が行い、書いた。**統合（C1。HEAD 5a39f6b = GDI 一括化（決定 0110、0111）の後の木に、E1 の差分を当てた。順序は GDI → E1）**: 応答表の末尾の衝突は手で解決し（GDI の 96 行の後に E1 の 7 行）、コンパイルし（Rust の警告・エラーなし。C は clang の `-Wall -Wextra -Werror`）、全体テストをグループに分けて走らせた（**115 test result・1,575 passed・0 failed**。内訳と C1 の他の差分は決定 0113 と MERGE-NOTES）。`wall_e1.rs` の **24 件はすべて通った**（初回から。期待値の修正は不要だった）。生成 C の統計と SHA、固定数（`vcall_thunk_dll.rs` 394 → **390**）、a4 の再測定は下の各節に反映した。**実機 Windows との照合は無い**（ホストは Linux）。ホストのコンピュータ名・ユーザー名・ファイル・レジストリ・ロケールは一切読まない。E1 自身の変異検査 M1〜M15 は C1 では**実行していない**（C1 は C1 の差分に対する 12 件を実行した。決定 0113）。census（dsp 構成）も C1 では実行していない。

**これは A4 の達成ではない。** 統合した木の 6 モジュール構成・M1（フォント配置あり）の測定は、決定 0113 の連鎖で `init_count` が **942/942** に達した（測定用の基底・種・註釈と仮の応答を含む構成。実機 Windows との照合なし）。**この連鎖の中でこの決定の API が呼ばれたかを gdb のブレークポイントで数えた（実測。最終の native）: `GetComputerNameExW` が 2 回、ほかの 5 つ（`GetComputerNameW`、`GetFileVersionInfoW`、`VerQueryValueW`、`LocaleNameToLCID`、`GetUserNameW`）は 0 回。**つまり unit 830、880 の経路の `GetComputerNameExW`（NameType 0、`DESKTOP-0000001`）は A4 の連鎖で実行され、通過した。census（dsp 構成）の壁の一覧への効果は C1 では測っていない。

区分の凡例: **実測** = 実行した出力・機械的な数え上げ（PE の逆アセンブル、K1 の TSV と pickle の再計算）。**読み** = コードの解釈。**Learn** = Microsoft Learn のページに書かれていること（取得日 2026-10-28、`updated_at` を併記）。**参考（非公式）** = Learn 以外（Microsoft Q&A、mingw-w64 のヘッダ）。**推測**。**未確認**。

事実の出典:

- K1 のバックログ: `lift/out-git/k1-results/`（`backlog-groups.md`、`unanswered-imports-by-first-unit.tsv`、`needs-by-unit.tsv`、`census-crosscheck.tsv`、`fle-imports-all.tsv`、`bundled-dll-unanswered.tsv`）。census: `lift/out-git/w6s2-results/logs/census/2mod-w6/`（`report-dsp.tsv` の 830、880）、`lift/out-git/h2-results/census-hooks/`。決定 0107（W3: shfolder と CSIDL_APPDATA。次の壁の予測）、0098（GetFileVersionInfoSizeW）、0099（遅延 import の行）、0016（ロケール）、0071（仮想窓）、0015（応答表は末尾に足す）、0018、0014。
- FLEngine の逆アセンブル（capstone、pefile。作業物は `e1-results/tools/`）。呼び出し元と引数の形は `api-table.md` §1.3。
- Microsoft Learn の本文は `e1-results/learn/` に抜き出して保存した。対象の API ごとの「Learn の文言 → 実装の規則」は **`api-table.md` §2**。

## 先に結論

| 項目 | 内容 |
|---|---|
| 対象 | K1 の 10 行（IAT 9 + 遅延 1）と近傍。**census の壁は `GetComputerNameExW`（unit 830、880）だけ**。残りの 9 行は静的な閉包にあるだけ（census では ok） |
| 答える（7 行を応答表の末尾に足す） | `kernel32!GetComputerNameExW`、`kernel32!GetComputerNameW`、`version!GetFileVersionInfoW`、`version!VerQueryValueW`（必ず停止）、`kernel32!LocaleNameToLCID`（既知の名前だけ）、`advapi32!GetUserNameW`、`kernel32!GetComputerNameExA`（最後の 2 行は**どのモジュールも import していない**） |
| 答えない | `DestroyWindow`、`GetWindowLongPtrW`（窓レーン）、`PostMessageW`、`ResumeThread`、`SetClipboardData`（名前付き停止にできる）、`EnumSystemLocalesW`（コールバックと環境の主張が要る）。分類は `api-table.md` §3 |
| 実装した API の数 | **7**（答える形あり 5: GetComputerNameExW／GetComputerNameW／GetComputerNameExA／GetUserNameW／LocaleNameToLCID。失敗の形あり 1: GetFileVersionInfoW。必ず停止 1: VerQueryValueW。ただし GetComputerNameExW 系の NameType 2／3／6／7／8、LocaleNameToLCID の他の名前、GetFileVersionInfoW の既存パスなどに名前付き停止の分岐がある） |
| 追加した応答表の行 | **7**（GDI 一括化の後の 427 → **434**。統合した木の実測: 生成 C の `fle_resp_count` 427 → 434、`fle_dispatch_count` 59,807 → 59,814） |
| 新しい `.c`／`.h` | なし（`fle_env.c/.h`、`fle_vfs.c/.h`、`fle_locale.c/.h` に足した） |
| テスト | 新規 `tests/wall_e1.rs`、**24 件**（生成器 3、runtime 17、実 DLL 4）。**統合した木で 24 件とも通った**（実 DLL の 4 件は `FLENGINE_DLL` を実ファイルに向けて実行） |
| 出典なしの選択 | コンピュータ名 `DESKTOP-0000001`、DNS ホスト名 = NetBIOS 名、ほか（§出典なしの選択の一覧） |
| 変わる既存テスト | `vcall_thunk_dll.rs` の `unsupported_total` 394 → **390**（−4。統合した木。GDI 後の値 394 から）、`a4-multi.sh` の `EXPECT_FLE_SHA`（`508b76fb…` → **`f6d3a971…`**） |

## 背景

### 壁（census。実測）

`kernel32!GetComputerNameExW` は応答表に行が無く、thunk `0x423430` は `fle_unsupported`。census（w6s2 の 2 モジュール、HEAD+W6）の unit 830（proc `0x10d50d0`）と unit 880（proc `0x1263540`）が `f_423430 ← f_10c7cd0 ← f_10d50d0`（ret `0x10c89dd`）、`f_423430 ← f_125da90 ← f_1263540`（ret `0x125dcf1`）で止まった。決定 0107 W3 の読みのとおり、直前に SHGetFolderPathW(CSIDL_APPDATA) が通った後の壁（0107 は「次は GetComputerNameExW」と予測し、答えなかった）。

### FLEngine の 26 箇所（実測。詳細は `api-table.md` §1.3）

NameType はすべて 0（NetBIOS）。25 箇所は 0x2000 TCHAR のバッファに戻り値を見ずに NUL まで読み、バッファは直前の SHGetFolderPathW が書いた AppData のパスのまま（**書かずに失敗するとそれをコンピュータ名として読む**）。1 箇所（0x1159fc9）は 0x3ff で戻り値と `*nSize` を使う。GetComputerNameW は 1 箇所（0x5df888、16 TCHAR のバッファに size = 32、読み）。

### version 資源（読み）

`f_651ce0` の Size→Info→Query は、unit 880 では `f_125da90` → `f_125d9b0` → `f_652220`（`GetModuleFileNameW(hInstance)`）→ `f_6520b0` → `f_651ce0` で**モジュール自身の仮想パス**に対して呼ばれ、決定 0098 により Size は 0（エラー 3）を返し、`jbe` で Info と Query は飛ばされる。したがって **GetFileVersionInfoW と VerQueryValueW は現状のどの経路でも呼ばれない**（K1 が閉包に挙げただけ）。もう 1 組（`f_44ada0`）は `"comctl32.dll"`（ディレクトリ部なし）を渡し、Size が先に名前付きで停止する。

## 決定（仮）

| # | API | 規則（根拠は `api-table.md` §2） |
|---|---|---|
| D1 | `GetComputerNameExW` | NameType ecx（int。上位 32 ビットは見ない）。**0、4（NetBIOS、PhysicalNetBIOS）と 1、5（DnsHostname、PhysicalDnsHostname）は同じ名前 `DESKTOP-0000001`**。2、3、6、7（DNS ドメイン、完全修飾名）、8（ComputerNameMax）、範囲外は**名前付き停止**。サイズ: `nSize` が NULL → 停止。`*nSize` ≥ 16 → 名前と NUL を書き、`*nSize` = 15、戻り 1、last error は触れない。`*nSize` < 16 → 戻り 0、last error 234（ERROR_MORE_DATA）、`*nSize` = 16、**バッファは書かない**。NULL バッファは `*nSize` = 0 のときだけ（上の失敗）、非 0 なら停止 |
| D2 | `GetComputerNameExA` | D1 と同じ規則で 8 ビット文字（名前は ASCII） |
| D3 | `GetComputerNameW` | NetBIOS 名。D1 と同じ規則。小さいときの last error は **111（ERROR_BUFFER_OVERFLOW）**。NULL バッファは `*nSize` が名前に足りないときだけ失敗、足りるなら停止（ページが定義しない） |
| D4 | `GetUserNameW`（advapi32） | 名前 `user`（決定 0107）。成功時 `*pcb` = 5（NUL を含む）。小さい: 戻り 0、last error 122（ERROR_INSUFFICIENT_BUFFER）、`*pcb` = 5、書かない |
| D5 | `GetFileVersionInfoW` | 決定 0098 の GetFileVersionInfoSizeW と**同じ失敗**: 仮想ボリュームに無いパスは戻り 0、last error = ファイルシステムの 2（親がある）または 3（親が無い）。`lpData` は触らず、`dwHandle`（ページ: ignored）と `dwLen` は見ない。NULL 名、ディレクトリ部のない名、仮想ボリュームに**ある**パスは停止 |
| D6 | `VerQueryValueW` | **必ず停止**（環境が発行した version block が存在しない。ゲストの自作の block を「valid でない」と言う根拠もページに無い） |
| D7 | `LocaleNameToLCID`（遅延 import。GetProcAddress 経由） | NULL（LOCALE_NAME_USER_DEFAULT）、`!x-sys-default-locale`、エミュレートしたロケールの名前（`en-US`。ASCII の大文字小文字を無視）は 0x409（決定 0016）、空文字列は 0x7F（LOCALE_INVARIANT）。`dwFlags` は 0 と 0x08000000 だけ。他の名前・フラグ、85 文字以上、0x7F 超の文字は停止。last error は触れない |

実装した定数: `FLE_VIRTUAL_COMPUTER_NAME "DESKTOP-0000001"`、`FLE_VIRTUAL_USER_NAME "user"`、`FLE_COMPUTERNAME_NETBIOS_MAX 15u`、`FLE_W32_ERR_BUFFER_OVERFLOW 111u`（`fle_env.h`）。名前の長さは `_Static_assert` で 15 以下を検査する。

## 選択肢

- A. 採用（上）。
- B. NameType 2、6（DNS ドメイン）に空文字列と成功、3、7 にホスト名だけの完全修飾名を返す: **不採用**。ドメイン未参加の機械が何を返すかを Learn は書かない（"If the local computer is not configured to use DNS names, GetComputerNameEx will not return DNS information" は失敗か空かを言わない）。FLEngine は NameType 0 しか使わないので、答えなくても進捗に差が無い。PM が決められる（PM の未決の点 2）。
- C. 失敗を返して「名前が無い」と言う: **不採用**。25 箇所は戻り値を見ずバッファを読み、**直前の AppData のパスをコンピュータ名として読む**ことになる。実機の Windows 10 でこの API が常に成功することとも合わない。
- D. VerQueryValueW に 0 を返す: **不採用**（Learn の "or the specified resource is not valid, the return value is zero" を使って「ゲストの自作の block」も無効と言うことになる。FLEngine の 4 箇所は GetFileVersionInfoW の成功を条件にしているので、停止は実害がない）。
- E. GetFileVersionInfoW のホストのファイルの版情報を返す: **不採用**（依頼の方針。ホストの実ファイルを見せない）。
- F. 5 つ（DestroyWindow ほか）にも名前付き停止の行を今足す: **今回は足さない**（PM の未決の点 4）。それぞれが窓・メッセージキュー・スレッド・クリップボードのレーンの前提で、census では呼ばれていない。足す場合の機械的な変更は `api-table.md` §3 の末尾。

## 範囲

FLEngine の `GetComputerNameExW`（26 箇所）、`GetComputerNameW`（1 箇所）、version.dll の 2 関数（Size の続き）、遅延 import の `LocaleNameToLCID`、依頼にあった `GetComputerNameExA`／`GetUserNameW`（FLEngine にも同梱 DLL にも無い）。ILTools も `GetFileVersionInfoW`／`VerQueryValueW` を import する（K1。ILTools の unit 93、vtable 候補）ので、応答表の共有により同じ答えになる（ILTools がどのパスを渡すかは**未確認**）。

## 範囲外

- `DestroyWindow`、`GetWindowLongPtrW`、`PostMessageW`、`ResumeThread`、`SetClipboardData`、`EnumSystemLocalesW`（`api-table.md` §3）。
- NameType 2、3、6、7（ドメイン）。ドメイン参加・クラスタの主張。
- `GetUserNameExW`、`GetComputerNameExW` の SetComputerName 系、`WTSQuerySessionInformation`。
- version 資源の中身（`VS_FIXEDFILEINFO`、StringFileInfo）。仮想ボリュームのファイルの版情報。
- 他の「環境の主張」（K1 の env-claims の 74 種のうちこの決定に無いもの）、census の測定、A4。
- `fle_win.c`、`fle_gdi.c`、`tests/gdi.rs`、`tests/gdc.rs`、`harness/`、`vcall_thunk_dll.rs` の期待値（式だけ MERGE-NOTES に書いた）。

## 撤回条件

(1) 実機 Windows 10 の `GetComputerNameExW`／`GetComputerNameW`／`GetUserNameW` の値と失敗の仕様（小さいバッファでバッファを書くか、`*nSize` の単位）が示され、食い違う。(2) PM が環境の定義にコンピュータ名・ユーザー名を入れる（そのとき `FLE_VIRTUAL_COMPUTER_NAME`、`FLE_VIRTUAL_USER_NAME`、`FLE_VIRTUAL_USER_PROFILE_DIR`（決定 0107）の 3 か所を一緒に整合させる）。(3) ドメイン参加・クラスタの主張が入る（NameType 2／3／6／7、Physical の分離）。(4) 仮想ボリュームに版情報のあるファイルを置く決定（GetFileVersionInfoW と VerQueryValueW の設計が変わる。そのとき GetFileVersionInfoSizeW の停止「パスが仮想ボリュームにある」も変わる）。(5) ゲストが GetFileVersionInfoW を Size 無しで呼ぶことが分かった（D5 の失敗の形を見直す）。(6) 「サポートするロケールはエミュレートした 1 つ」という既存の選択（`IsValidLocale`）の変更（D7 と EnumSystemLocalesW）。(7) GetUserNameW／GetComputerNameExA のいずれかを import するモジュールが現れない限り、最後の 2 行は外してよい（外しても他の行の擬似 VA は動かない）。

## 実装した場所

新しい `.c`、`.h` は作っていない（`tests/modules.rs` が全 `fle_*.c` をリンクするため）。差分は `e1-env-claims.diff`（HEAD の木に対する `diff -ru`。`patch -p1` で当たることを HEAD の木で `--dry-run` して確認）。

- `lift/runtime/fle_env.h`／`.c`: `FLE_W32_ERR_BUFFER_OVERFLOW`、`FLE_VIRTUAL_USER_NAME`、`FLE_VIRTUAL_COMPUTER_NAME`、`FLE_COMPUTERNAME_NETBIOS_MAX`、`fle_imp_GetComputerNameExW`／`…ExA`／`GetComputerNameW`／`GetUserNameW` と共通の `computer_name_for_format`、`put_name`、`name_answer`。既存の関数の本体と値は**変えていない**。
- `lift/runtime/fle_vfs.h`／`.c`: `fle_imp_GetFileVersionInfoW`（Size の本体は変えず、パスの手順を繰り返した）、`fle_imp_VerQueryValueW`。
- `lift/runtime/fle_locale.h`／`.c`: `fle_imp_LocaleNameToLCID`。
- `lift/crates/fe-lift/src/imports_table.rs`: `TABLE` の**末尾に 7 行**（コメント付き）。既存の擬似 VA は動かない（0015）。
- `lift/crates/fe-lift/tests/wall_e1.rs`（新規）。

## テスト（`tests/wall_e1.rs`。24 件。統合した木で実行して全件通過）

生成器 3: `the_seven_rows_are_appended_in_order_after_the_older_rows`、`the_four_imported_flengine_thunks_are_answered_under_their_own_dll`、`the_functions_of_the_rows_are_declared_in_the_runtime_headers`。runtime 17（模擬 PE の生成 C と全 `fle_*.c` をリンクしたドライバ。GetProcAddress の擬似 VA を `fle_dispatch` で呼ぶ）: 定数と名前の形、ExW の成功・形式 0／1／4／5・小さい／NULL／サイズ 0・停止する形式、26 箇所の呼び出しの形（古い AppData の文字列が NUL で切れる）、GetComputerNameW、GetUserNameW、ANSI、GetProcAddress の行の DLL 別の有無（遅延 import の LocaleNameToLCID を含む）、GetFileVersionInfoW の失敗と停止、モジュール自身のパスで Size が 0（unit 880 の経路）、VerQueryValueW、LocaleNameToLCID の答えと停止。実 DLL 4（無ければ飛ばす）: 26 箇所の形（25 + 1）、GetComputerNameW の 1 箇所、version の 3 組と `GetModuleFileNameW` の出所と `"comctl32.dll"`、遅延 thunk。期待値は決定と Learn から書き、C のコードから読み戻していない。

### 変異検査の案（C1 では**未実行**。`sed` で runtime の複製を壊して `wall_e1` のどのテストが落ちるかを見る）

| # | 変異 | 落ちるはずのテスト |
|---|---|---|
| M1 | 名前を 16 文字にする | `_Static_assert`（コンパイル不能）／`the_computer_name_is_a_netbios_name…` |
| M2 | NameType 4、5 を停止にする | `the_netbios_and_dns_host_name_formats…` |
| M3 | NameType 2 を空文字列＋成功にする | `the_formats_without_a_derivable_answer…` |
| M4 | NameType を 64 ビットで読む | `get_computer_name_ex_w_writes_the_name…`（上位が汚れた rcx） |
| M5 | 成功時の `*nSize` に NUL を数える（W）／数えない（GetUserNameW） | 同上、`get_user_name_w…` |
| M6 | 失敗の last error を ExW で 111 に／W で 234 に | `a_buffer_that_is_too_small…`、`get_computer_name_w…` |
| M7 | 小さいときに途中まで書く／`*nSize` を書かない | `a_buffer_that_is_too_small…`（first aaaa、size 16） |
| M8 | NUL を書かない | `get_computer_name_ex_w_writes_the_name…`（next）、`the_call_shape_of_the_26…` |
| M9 | NULL バッファ＋非 0 サイズを停止にしない | `a_null_buffer_with_size_zero…` |
| M10 | GetUserNameW の名前を `User` に | `get_user_name_w…` |
| M11 | GetFileVersionInfoW を成功にする／エラーを固定の 2 にする | `get_file_version_info_w_fails_like…` |
| M12 | VerQueryValueW が 0 を返す | `ver_query_value_w_stops_in_every_case` |
| M13 | LocaleNameToLCID が任意の名前に 0x409 を返す | `locale_name_to_lcid_stops_for…` |
| M14 | 行を別の DLL の下に置く／順序を入れ替える／1 行外す | `the_seven_rows…`、`the_rows_are_found_by_get_proc_address…`、`the_four_imported_flengine_thunks…` |
| M15 | FLEngine で `unsupported_total` が 390 にならない（行を 1 つ外す） | `vcall_thunk_dll.rs` |

## 固定数テストへの影響（統合した木で実測）

| 項目 | GDI 一括化の後（HEAD 5a39f6b） | E1 を当てた後（実測） |
|---|---|---|
| 応答表の行数（生成 C の `fle_resp_count`） | 427 | **434**（+7） |
| `fle_dispatch_count` | 59,807 | **59,814** |
| 新しい 7 行の擬似 VA（`0x7FF900000000 + 0x10 × (k + 1)`） | — | k = 427〜433 → `0x7FF900001AC0`〜`0x7FF900001B20`（算術。`wall_e1.rs` は行の順序・連続・0x10 刻みだけを固定し、擬似 VA の値は固定していない） |
| `vcall_thunk_dll.rs` の `unsupported_total` と `unsupported["Jmp: indirect through IAT slot (not lifted)"]`（2 か所） | 394 | **390**（−4）。**実測で 390**（389 ではない）= `LocaleNameToLCID` の遅延 thunk は数えられていない（決定の読みどおり） |
| 内訳（−4） | — | GetComputerNameExW `0x423430`、GetComputerNameW `0x423420`、GetFileVersionInfoW `0x423df0`、VerQueryValueW `0x423e10`（各 1 thunk） |
| FLEngine の生成 C（註釈なし、`fe-lift lift FLE OUT.c --pdata` が書いたファイルの SHA-256） | `508b76fb15153de73e03dd9f7813b7f77cc0122b105a2f5d728b93737939263b`（215,771,656 B、functions 59,380、bad_nodes 0、unsupported_total 394、c_lines 2,886,657） | **`f6d3a9715ddb92d734577fc1e87a381142c722aa17562a22a72c565599cbb08c`**（215,773,171 B、functions 59,380、bad_nodes 0、unsupported_total **390**、c_lines 2,886,678 = +21 = 7 行 × 3 節） |
| FLEngine の生成 C（註釈あり、`--hook-points flengine-hook-points.txt`。a4 の測定で `EXPECT_FLE_SHA` に使う） | `6fd3a82e88c46e8f40ed5479a3298a225f71fa9ecc6a7eb4e7f41036874c0426`（215,772,503 B） | **`e047beeb44f16aa406a6ec254cc6603be192044206195c62ea2d459ec88bfdd2`**（215,774,018 B） |
| 本体が変わった FLEngine の関数 | — | **4**（`f_423420`、`f_423430`、`f_423df0`、`f_423e10`。関数単位のダイジェストの比較。ほかは `fle_dispatch_table`／`fle_resp_table` の各 7 行、個数 2 行、プロトタイプ 7 行） |
| 他のモジュールの生成 C への効果（HEAD の fe-lift と C1 の fe-lift の比較。C1 の fe-lift には LDDQU と CPU 振り分けスタブの変更も入っているので、E1 だけの効果ではない） | — | ILTools の thunk 2 つ（`f_601d580`、`f_601d5a0`。version.dll の 2 API）が変わり、`unsupported_total` 613 → 611。Blend2D、QFC、freetype は 0 関数 |
| `harness/a4-multi.sh` の `EXPECT_FLE_SHA`（既定） | `508b76fb…` | **`f6d3a9715ddb92d734577fc1e87a381142c722aa17562a22a72c565599cbb08c`** |
| `harness/two/gen_two.sh`、`harness/census/gen_two.sh` の `EXPECT_FLE_SHA`（`feb5b908…`／`43985d62…`） | 元から古い値（HEAD の SHA と違う） | 変更なし |
| 実 DLL のテスト（`FLENGINE_DLL`） | — | `wall_e1` の実 DLL 4 件を実行して通過 |

## 検証（統合した木。C1）

- コンパイル: 警告 0・エラー 0（`cargo build`、dev）。C は `clang -fsyntax-only -std=gnu11 -Wall -Wextra -Werror`。
- テスト: 全体を 12 グループに分けて実行（1 回の通しではない）。**115 test result・1,575 passed・0 failed**。`wall_e1` 24 件は全件通過。隣のテスト（`wall_r`、`wall_r2`、`wall_s`、`locale`、`vfs`、`ver`、`versioninfo`、`env`、`modules`、`sysinfo`、`vcall_thunk_dll`）も通過。`modules.rs` が全 `fle_*.c` をリンクするので、重複シンボルの有無もこれで確かめた（なし）。
- 生成 C: 上の表（4 関数の本体が変わった）。
- a4（6 モジュール構成・M1・測定用の基底と種）: 統合した木では決定 0113 の連鎖で 942/942（0113 に記録）。この決定の API の呼び出し回数（gdb、最終の native）: `GetComputerNameExW` 2 回、ほか 5 つは 0 回（ログ `logs/gdb-e1.txt`）。
- **やっていないこと**: E1 の変異検査（M1〜M15）、census（dsp 構成）、wasm、実機との比較。

## census での次の壁の見込み（読み。**C1 でも実測していない**）

unit 830（proc `f_10d50d0`）と unit 880（proc `f_1263540`）の**静的な直接 call の閉包**（K1 の `fle-clos.pkl` から再計算）:

- unit 830: 83 関数、import 呼びは 12 種（RegCloseKey、RegEnumKeyExW、RegOpenKeyExA、RegQueryValueExW、FileTimeToSystemTime、FreeLibrary、GetComputerNameExW、GetDateFormatW、GetFileAttributesExW、GetLocalTime、GetThreadLocale、LoadLibraryW）。**GetComputerNameExW を除いて全部に行がある**（実測）。
- unit 880: 23 関数、10 種（RegCloseKey、RegOpenKeyExA、RegQueryValueExW、FreeLibrary、GetComputerNameExW、GetModuleFileNameW、LoadLibraryW、GetFileVersionInfoSizeW、GetFileVersionInfoW、VerQueryValueW）。**この決定ですべてに行がある**。

予測:

1. 両 unit は `GetComputerNameExW` を通る（NameType 0、`DESKTOP-0000001`）。FLEngine 側の「未応答 import」の壁（分類 a）は 1 種から 0 種になる。dsp 構成の census の放棄 unit は 5 → 最大 3（ILTools の `LoadLibraryW "iltools_x64.dll"` の 245、280、659 は dsp 構成では残る）。
2. 次に止まるとすれば、**未応答 import ではなく**、答え済みの API の中の名前付き停止か、lift の穴。名前を挙げられるのは次の 3 つ:
   - `RegOpenKeyExA(HKLM, "SOFTWARE\Microsoft\Cryptography", …)`（決定 0107 の読み）: 仮想レジストリは `HKLM\Software` だけが空のキーとして在る（0085）ので、`Microsoft` が無く ERROR_FILE_NOT_FOUND のはずで、ゲストは MachineGuid の読み（RegQueryValueExW）を飛ばす（読み）。
   - `GetDateFormatW`（`f_443110`、`f_443300`。`GetThreadLocale()`、**flags = 4**、書式の文字列あり）: `fle_time.c` は「書式の文字列があるのに flags が 0 でない」で停止する。ただし直前の `GetFileAttributesExW`（unit 830 の proc の 0x10d5224）が仮想ボリュームに無いパスで失敗するはずなので、日付の整形は飛ばされる（読み）。census が 830 でこの前を通って GetComputerNameExW まで届いたことと合う。
   - `GetFileAttributesExW`／`RegEnumKeyExW` の引数の形（読んでいない）。
3. unit 880 では GetFileVersionInfoW と VerQueryValueW は**呼ばれない**（Size が 0）。呼ばれたら D5／D6 の停止になる。
4. A4 の 6 モジュール構成: 830、880 は unit 245 の壁（GDI レーン）の後なので、この決定の効果はまだ見えない。見えるのは dsp 構成の census だけ。
5. 計画外の副作用: `comctl32.dll`（`f_44ada0`）の GetFileVersionInfoSizeW の停止（ディレクトリ部のない名前）が、unit 895 側の UI の立ち上げで先に出る。

## 出典なしの選択の一覧

| # | 選択 | 値 | 根拠・区分 |
|---|---|---|---|
| S1 | **コンピュータ名** | `DESKTOP-0000001`（15 文字、大文字・数字・ハイフン） | **値は出典なしの選択**。形の制約は Learn（15 文字以内、大文字、ハイフンは名前の中、数字だけは不可）。"DESKTOP-" + 7 文字の形は Microsoft Q&A（**参考（非公式）**）と、Windows 10 の既定の形としての一般知識。ホストの名前は使わない。`DESKTOP-FLENGIN` のような FLEngine 由来の語は避けた（依頼文の読み。「解釈が分かれた点」3） |
| S2 | DNS ホスト名 | NetBIOS 名と同じ文字列（大文字のまま） | Learn: NetBIOS 名は DNS ホスト名の OEM 表現を 15 文字まで、規約では大文字。DNS ホスト名の大文字小文字は選択（ドメイン名は大文字小文字を区別しない: Computer Names） |
| S3 | ユーザー名 | `user` | 決定 0107 の選択を再利用。値は出典なし |
| S4 | Physical 形式 = 非 Physical | 同じ名前 | Learn: クラスタのノードでのみ異なる。この環境がクラスタのノードでないという主張は環境の選択 |
| S5 | 失敗時にバッファを書かない | — | **推測**（"the function fails" から。ページに無い） |
| S6 | `*nSize` の単位は TCHAR | W は文字、A はバイト | ページ内の食い違い（本文は TCHARs、エラー表は "bytes"）を本文に寄せた |
| S7 | GetFileVersionInfoW の失敗コード | 2 または 3 | **推測**（Learn はコードを書かない。決定 0098 のファイルシステムの規則の延長） |
| S8 | LocaleNameToLCID の名前の比較 | ASCII の大文字小文字を無視 | **推測**（ロケール名は言語タグ。Learn のこのページは大文字小文字を言わない） |
| S9 | 空の名前 → 0x7F | LOCALE_INVARIANT | winnls.h の定義値（**参考（非公式）**）と `fle_locale.c` の既存の定数 |
| S10 | 成功時に last error を触れない | — | 既存の決定と同じ慣習（ページは書かない） |
| S11 | 最後の 2 行（GetUserNameW、GetComputerNameExA） | import するモジュールが無いまま行を足す | PM の依頼どおり |

## 解釈が分かれた点

1. **依頼の前提「GetComputerNameExA／GetUserNameW が 26 箇所の周辺で呼ばれる」**: 事実と違う（FLEngine も同梱 DLL も import しない。実測）。依頼の本文は両方の設計を求めているので、実装して最後の 2 行にした。
2. **「9 件」**: K1 の表は 10 行（遅延 import の `LocaleNameToLCID` を含む）。本文の「9 件」は IAT だけの数。
3. **「`DESKTOP-FLENGIN` ではなく」**: 「この例のような FLEngine 由来の名前ではなく、15 文字以内の大文字の決定的な固定値」と読んだ。別の読み（その例を使う）も成り立つが、名前の語に製品名を入れると、ゲストが名前から識別子を作る場合に環境の都合が漏れる、と判断した。
4. **ERROR_MORE_DATA の単位**: 本文は TCHAR、エラー表は bytes。TCHAR（S6）。
5. **GetComputerNameW の「15 文字制限」**: 名前の長さの制約として扱い（15 文字以内の名前）、バッファが小さければ失敗する（切り詰めない）。
6. **「サイズ 0・負・不正な形式」**: DWORD に負は無い（0xFFFFFFFF は大きな値）。不正な形式は NameType の範囲外、NULL のサイズ、NULL のバッファに読んだ。
7. **version 資源「実機と同じ失敗応答になるか」**: Learn は失敗のコードを書かない。実機の挙動は**未確認**。決定 0098 の規則に揃えた。
8. **365／492／745 の「実装できるか名前付き停止か」**: 分類だけ書き、行は足していない。

## 未決の点(PM)

| # | 点 | 実装した最小の案 | 他の選択肢 |
|---|---|---|---|
| 1 | コンピュータ名の値・形・大文字小文字 | `DESKTOP-0000001`、DNS ホスト名と同じ大文字 | 別の固定値（形は 15 文字以内、大文字・数字・ハイフン、数字だけは不可）、DNS ホスト名を小文字にする。名前から機械識別子を作るゲストの出力が決定的になる（毎回同じ）ことは副作用 |
| 2 | NameType 2、3、6、7（ドメイン、完全修飾名） | 名前付き停止 | 空文字列＋成功（ドメイン無し）、完全修飾名＝ホスト名。実機の挙動の確認が先（Learn は書かない）。FLEngine は NameType 0 しか使わない |
| 3 | GetUserNameW と GetComputerNameExA の行 | 足した（最後の 2 行。どのモジュールも import しない） | 外す（−2 行。`wall_e1.rs` の `E1_ROWS` と 2 つのテストを直す） |
| 4 | 5 つ（DestroyWindow、GetWindowLongPtrW、PostMessageW、ResumeThread、SetClipboardData）と EnumSystemLocalesW | 行を足さない（レーンに任せる） | 名前付き停止の行を今足す（−1 thunk／行）。EnumSystemLocalesW は「サポートするロケールは 1 つ」と答えるか（IsValidLocale と整合）は未決 |
| 5 | GetFileVersionInfoW の失敗コード、VerQueryValueW を必ず停止 | 決定 0098 と同じ 2／3、停止 | VerQueryValueW に 0 を返す（不採用の理由は選択肢 D） |
| 6 | 実機の測定 | しない | Windows 10 の実機で NameType 0〜8 の戻り値・小さいバッファ・GetFileVersionInfoW の存在しないパスの GetLastError を測れれば、S5、S7、点 2 が閉じる |

## 限界

- C1 でコンパイルとテスト（`wall_e1` 24 件）を実行した（上の検証）。census と E1 の変異検査は未実行。
- 実機の Windows と照合していない。S1〜S11 は出典なしか推測。
- unit 830、880 の先は census 前の読み。K1 の静的な閉包は上界で、動的な経路は見えない。
- `GetFileVersionInfoW`／`VerQueryValueW` が ILTools（6 モジュール構成）でどのパスを受けるかは**未確認**。
- `f_a5fce0` の version 呼び出しのファイル名の出所は未確認。`f_44ada0` の `comctl32.dll` は読み。
- `LOCALE_NAME_MAX_LENGTH` = 85 は mingw-w64 の `winnt.h`（非公式）の値。
- 遅延 thunk（`LocaleNameToLCID`）は `unsupported_total` に**数えられていない**ことを C1 で実測で確認した（390。389 ではない）。
- wasm 版はビルドしていない。
