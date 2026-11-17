# 0085: 仮想レジストリの初期状態に `HKLM\Software` を空のキーとして持たせる（0018 の「初期状態は空」への例外。仮の決定）

状態: PM の決定（2026-10-24）を、作業者（サブエージェント）が記録し、実装した。**仮の決定**（実機の Windows 10 22H2 で未確認。撤回条件は下）。事実の出典は `docs/investigations/registry-unit153.md`（以下「調査」。節番号はその文書のもの）。この記録は調査に無い主張を足さない。決定 0018 の「初期状態は空」（決定の節）と「ルート 5 つは最初から存在し、中身は空」（§6）に、この決定による例外が付く。0018 の本文は書き換えず、該当の 2 行に注記を足した（0083 が 0018 §7 の行に注記した例に倣った）。

## 決定（PM。内容を変えない）

| # | 項目 | 決定 |
|---|---|---|
| 1 | 初期状態 | 仮想レジストリは `HKLM\Software` を、値もサブキーも無い空のキーとして最初から持つ。調査 §5.1 の選択肢 (a) の最小形。Learn の Registry Hives と Structure of the Registry が支持する（調査 §4.3） |
| 2 | 作らないもの | `Microsoft` 以降は最初から作らない。一次資料に Windows 10 の記述が無く（調査 §4.3、§4.5）、作るとゲストの `RegOpenKeyExW` の結果が変わるため（調査 §5.1、§6.2 のモード f） |
| 3 | 変えない規則 | HKLM 直下（ルートの直接の子）を作ることを許さない規則（`fle_reg.c` の `cur == &roots[2] \|\| cur == &roots[3]` の名前付き停止）は変えない。HKU も同じ |
| 4 | 他のハイブ | HKCU など他のハイブの初期状態は変えない（HKCU `Software` は従来どおり存在しない） |
| 5 | 採らない選択肢 | (b) 直下を許す、(c) 失敗コードを作って返す、(d) 停止のまま、は採らない（調査 §5.2〜§5.4） |
| 6 | 状態 | 仮の決定。撤回条件は下 |

## 背景（事実。調査の節番号）

- unit 153（proc 0x665b90。`f_422ff0` ← `f_5af5e0` ← `f_664fd0` ← `f_665b90`）が `RegCreateKeyExW(HKLM, L"Software\Microsoft\Windows NT\CurrentVersion\Fonts", 0, NULL, 0, 0xF003F, NULL, &h, &disp)` を呼ぶ（調査 §2.1、§2.2。実測の引数）。目的はインストール済みフォント一覧の読み取りで、書き込み系の関数は呼ばない（調査 §2.2。読み）。
- 停止の判定は、作ろうとしている成分の親が HKLM か HKU のルートそのものであること（調査 §1.1。調査時点の `fle_reg.c` 347 行）。パスを先頭から順に探し、最初に見つからなかった成分でこの検査に当たる。空の仮想レジストリでは 1 段目の `Software` が HKLM に無く、そこで止まる（調査 §1.1）。1 段目が既にあれば、2 段目以降の親は `roots[]` ではないので判定に当たらない（実測。調査 §1.1 のモード d、e）。
- Learn の RegCreateKeyExW は「An application cannot create a key that is a direct child of HKEY_USERS or HKEY_LOCAL_MACHINE. An application can create subkeys in lower levels of the HKEY_USERS or HKEY_LOCAL_MACHINE trees.」と書き、**エラー番号は書かない**（調査 §1.2、§4.1）。「The RegCreateKeyEx function creates all missing keys in the specified path.」とも書く（調査 §4.1）。
- Learn の Registry Hives は標準ハイブの表に `HKEY_LOCAL_MACHINE\Software` を挙げ、Structure of the Registry は「The HKEY_LOCAL_MACHINE key has the following subkeys: HARDWARE, SAM, SECURITY, SOFTWARE, and SYSTEM.」と書く。「Key names are not case sensitive.」とも書く。どちらのページも版を限定しない（調査 §4.3）。`HKLM\Software` の下に何があるか、`HKLM\Software\Microsoft\Windows NT\CurrentVersion` が Windows 10 にあるかは、取得した一次資料に無い（調査 §4.3、§4.5）。
- ゲストは、create が失敗しても例外を投げず、メッセージも再試行も無く戻る。成功して空のキーのときの呼び出し列は `RegCreateKeyExW` → `RegQueryInfoKeyW`（値の個数 0）→ `RegCloseKey` の 3 回で、フォント表は空のまま作り終わる。既存のキーを開いたか新規作成したかは区別しない（disposition を読まない）（調査 §2.3。読みと、仮の値を入れた構成での実測）。
- 調査の実測（仮の値を gdb で入れた 2 モジュール構成）: HKLM に空の `Software` だけを置くと本件の create は成功し、unit 12〜158 の他の open 41 回の結果は変わらず（すべて 2）、ログ比較で一致した（調査 §5.1、§6.2 のモード d、e）。`Software\Microsoft\Windows NT\CurrentVersion` まで置くと unit 16 と 95 の open が 2 から 0 に変わり、値の読み取りが走る（すべて 2。ゲストの OS バージョンの変数は変わらなかった）（調査 §5.1、§6.2 のモード f）。

## 範囲と実装した動作

- 仮想レジストリの初期状態は、HKLM のサブキーが `Software` の 1 つ（値 0、サブキー 0）。他の 4 つのルート（CLASSES_ROOT、CURRENT_USER、USERS、CURRENT_CONFIG）は従来どおり空。
- キー名は大文字小文字を区別しない（既存の規則。調査 §4.3 の Learn の記述）。`RegOpenKeyExW(HKLM, L"SOFTWARE" / L"software" / L"Software")` は 0（ハンドルを払い出す）。
- `Software` の下は空: `RegQueryInfoKeyW` はサブキー 0、値 0、`RegEnumKeyExW` と `RegEnumValueW` は index 0 で 259（ERROR_NO_MORE_ITEMS）。`RegOpenKeyExW(HKLM, L"Software\Microsoft")` と `L"SOFTWARE\Microsoft\Windows\CurrentVersion"`、`L"SOFTWARE\Classes"` は 2 のまま。HKCU、HKU、HKCR、HKCC の `Software` は 2 のまま。
- HKLM の列挙（`RegEnumKeyExW(HKLM, 0)`）は `Software`（8 文字）を返し、index 1 は 259。
- `RegCreateKeyExW(HKLM, L"Software\Microsoft\Windows NT\CurrentVersion\Fonts", 0, NULL, 0, 0xF003F, NULL, &h, &disp)` は 0 を返し、disp は REG_CREATED_NEW_KEY（1）。`Microsoft`、`Windows NT`、`CurrentVersion`、`Fonts` は、`RegCreateKeyExW` の既存の経路（`kid_add`）が作る（既存の挙動。調査 §1.1）。続く `RegQueryInfoKeyW` は 0 でサブキー 0、値 0、`RegCloseKey` は 0。同じ呼び出しの 2 回目は REG_OPENED_EXISTING_KEY（2）。大文字小文字を変えても同じ。
- `RegCreateKeyExW(HKLM, L"Software", ...)` は 0 で disp 2（`Software` は既にあるので、直下の子の作成ではない）。`Software\Foo` は作れる（disp 1）。
- 変わらない規則: `RegCreateKeyExW(HKLM, L"Foo")`、`L"HARDWARE"`、`L"Foo\Software"`、HKU の `L"Foo"`、`L"Software\Foo"`（HKU には `Software` が無い）は、従来どおり名前付き停止（`fle: RegCreateKeyExW: creating a direct child of HKEY_LOCAL_MACHINE or HKEY_USERS is not allowed (no code on the page)`、SIGABRT）。`lpdwDisposition` の値の規則、32 階層、512 階層の上限、ハンドルの領域と払い出しの式、`RegOpenKeyExW` が欠落に返す 2 は変えない。

## 仮の点

- **`HKLM\Software` が Windows 10 22H2 にあること**: 取得した Learn のページが版を限定せず書くだけ（調査 §4.3）。実機で確認していない。
- **標準ユーザーでの `Software\Microsoft\Windows NT\CurrentVersion\Fonts` の create の結果**: 一次資料に無い（Registry Virtualization のページは 64 ビットのプロセスと `HKLM\Software\Microsoft\Windows NT` 以下では仮想化しないと書くが、結果の番号と ACL は書かない。調査 §4.4）。この仮想レジストリは ACL を持たない（0018。セキュリティ記述子は未実装）ので、create は成功する。実機で同じ呼び出しが成功するかは、確認していない（撤回条件の対象）。
- **`Software` の write time**: `RegQueryInfoKeyW` が返す `Software` の最終書き込み時刻は、仮想クロックの初期値（`FLE_VFS_CLOCK_START`）。作業者の選択で、どの資料にもない値。HKLM ルートの時刻と仮想クロックは、`Software` を足す前の値に戻してあり、後から作るキーの時刻は 0085 の前と変わらない（テストで確認）。
- **`Microsoft` 以降が無いこと**: 実機にはあるはずのキーが仮想レジストリに無い状態は残る（調査 §4.5 は `Fonts` のパスだけを Learn の別ページから拾い、Windows 10 の存在を明言する一次は無いと書く）。
- **`Fonts` が空であること**: 上の `Fonts` キーは値を持たないので、ゲストのフォント表は空のまま作られる（調査 §2.3）。仮のフォント値は入れない。
- **ゲストの create が副作用を残すこと**: 下の限界を参照。

## 撤回条件

実機の Windows 10 22H2 で、標準ユーザーの `RegCreateKeyExW(HKLM, L"Software\\...")` が `Software` の存在以外の理由で失敗する、または HKLM に `Software` が無いと確認されたとき。撤回するときは `fle_reg.c` の `reg_boot` を外して初期状態を空に戻す（その場合、unit 153 の create は再び名前付き停止に戻る）。

## 実装した場所

- `lift/runtime/fle_reg.c`: `reg_booted`、`reg_boot()`（HKLM に `Software` を 1 つ `kid_add` し、仮想クロックと HKLM ルートの時刻を元に戻す）。`predefined()` の先頭で最初の 1 回だけ呼ぶ。冒頭と `fle_imp_RegCreateKeyExW` のコメントを更新。新しい .c、.h は作っていない。
- ルートの初期状態を作る専用の関数は、この runtime に無かった（`roots[5]` は static のゼロ初期化）。キーは `calloc` で作るため、静的な初期化子では足せず、最初にハンドルを解決する呼び出しの時に足す。`RegDeleteKeyW` が `free` するのは `kid_add` が作ったキーだけ、という既存の前提を保つため、`Software` も他のキーと同じ heap のキーにした。入口は全部 `predefined()`（`key_arg` 経由を含む）を通るので、最初に呼ばれる Reg* 関数がどれでも同じ初期状態になる。
- 固定の数への影響（数えた）: キーのハンドルは `Software` に払い出さない（`kh_n` は 0 のまま。最初に払い出されるハンドルは `FLE_REG_KEY_BASE + 4` のまま。テストで確認）。ハンドル領域の上限（`FLE_HANDLE_REGION_SIZE / 4`）、`KEY_NAME_MAX`、`CREATE_LEVELS_MAX`（32）、`TREE_DEPTH_MAX`（512）、値のサイズの上限は変えていない。`Software` の depth は 1（`kid_add` の既存の式）。HKLM の `kids` 配列は 1 要素（容量 8。`kid_add` の既存の初期容量）。

## テスト（`lift/crates/fe-lift/tests/reg.rs`）

- `registry_starts_empty`（既存を拡張）: 既存の 4 行（HKLM `SOFTWARE\Microsoft\Windows\CurrentVersion`、HKCU `Software`、HKCR `CLSID`、HKLM `HARDWARE` がすべて 2）の期待値は変えず、末尾に HKLM `Software` を開ける（0）、`RegQueryInfoKeyW` が 0 でサブキー 0・値 0、HKCU `Software` は 2、の行を足した。
- `hklm_software_exists_and_is_empty_and_nothing_else_was_added`（新規）: `Software` が開け、サブキー・値とも 0、`RegEnumKeyExW` と `RegEnumValueW` が 259、`SOFTWARE` と `software`（`RegOpenKeyExA`）でも開ける、`Software\Microsoft` と `SOFTWARE\Classes` は 2、HKCU、HKU、HKCR、HKCC の `Software` は 2。5 つのルートそれぞれの `RegEnumKeyExW` と `RegQueryInfoKeyW` で、キーを持つのは HKLM の `Software`（8 文字）1 つだけ。
- `creating_the_fonts_key_like_unit_153_works_on_the_initial_registry`（新規）: unit 153 と同じ形の呼び出し（引数は調査 §2.2）が 0、disp 1、最初のハンドルが `FLE_REG_KEY_BASE + 4`、続く `RegQueryInfoKeyW` が 0 でサブキー 0・値 0、`RegCloseKey` が 0。2 回目は disp 2。大文字小文字を変えた 3 回目も disp 2。作られた `Software\Microsoft`、`...\Windows NT`、`...\CurrentVersion` が開ける（各サブキー 1）こと、`Software` のサブキーが 1 であること、HKCU の同じパスが 2 であること。
- `creating_software_directly_under_hklm_opens_the_existing_key`（新規）: `RegCreateKeyExW(HKLM, L"Software")` が 0 で disp 2、そのハンドルの下の `Foo` が disp 1、`Software\Foo\Bar` が disp 1（`Foo` は既にあり `Bar` だけ新規）、`Software\Baz` が disp 1、`SOFTWARE\foo\bar` が開ける。
- `the_initial_software_key_does_not_move_the_virtual_clock`（新規）: `Software` の時刻が仮想クロックの初期値（132539328000000000）で、0085 の後に最初に作るキーの時刻が初期値 + 1 秒（0085 の前と同じ。`reg_boot` が仮想クロックを戻す行を外すと失敗することを確認した）。
- `creating_a_direct_child_of_hklm_or_hku_still_stops_with_the_named_message`（新規）: HKLM の `Foo`、`Foo\Software`、`HARDWARE`、HKU の `Foo`、`Software\Foo` が SIGABRT で、メッセージに `RegCreateKeyExW` と `direct child of HKEY_LOCAL_MACHINE or HKEY_USERS` を含む（返らない）。
- 既存の `undefined_input_stops_the_run` の行 `create-under-hklm`（HKLM に `Q` を作ると停止）は変えていない。0085 の後も同じ期待値で通る。
- `oleaut_ole.rs` の `prog_id_from_clsid_*`（HKLM `SOFTWARE\Classes\CLSID\{...}` を開く）ほか、レジストリの初期状態に依存する既存のテストは、変更なしで通った（`SOFTWARE\Classes` が無いので 2 のまま）。

## 限界

- **実機の Windows では未確認**。ホストは Linux。`Software` の存在も、標準ユーザーでの create の結果も、Learn の記述と調査の読みであって、実機の出力ではない（調査 §8）。
- **足したのは `HKLM\Software` だけ**。`Microsoft` 以降、HKCU の `Software`、`HARDWARE`、`SAM`、`SECURITY`、`SYSTEM` は足していない（Structure of the Registry は HKLM の子としてこれらを挙げるが、この決定は `Software` だけ）。
- **ゲストの create が成功すると、副作用で `Software\Microsoft\Windows NT\CurrentVersion\Fonts` の 4 つのキーが仮想レジストリに作られる**。以降に実行される unit が同じパスを `RegOpenKeyExW` で開くと、2 から 0 に変わりうる。調査 §3.2 の表と、調査 §3.1 と同じ静的な到達の方法（`lift/out-loadlib/` の `units.tsv`、`edges-direct.tsv`、`edges-ptr.tsv`）を再実行して確認した（`lift/out-hklm-sw/reach_after153.py`、`chain_literals.py`。作業物。vtable とレジスタ経由の間接 call は含まないので、「無い」は「実行されない」ではない。パスを実行時に組み立てる呼び出しもリテラルには出ない）:
  - `...\Windows NT\CurrentVersion\Fonts` を開く関数は `f_664fd0`（unit 153）だけ。
  - `...\Windows NT\CurrentVersion` を開く関数は 3 つ。`f_454090`（`SOFTWARE\Microsoft\Windows NT\CurrentVersion`。unit 16 だけで、unit 153 より前）、`f_44a9a0`（同じパス。`DisplayVersion`、`ReleaseId`、`UBR`。unit 95 に加え、unit 153 より後の unit 265、314、584、830、895、898（D）、これに 304、614、745、880 を足したのが P）、`f_7a62d0`（`software\microsoft\Windows NT\CurrentVersion`。`CurrentVersion`、`ProductName`。unit 265）。この 2 つ（`f_44a9a0`、`f_7a62d0`）は unit 153 より後で、open が 2 から 0 に変わりうる。値の読み取りは、値が無いので 2 になる。unit 16 と 95 の同じ変化は調査 §5.1 のモード f で実測済みで、ゲストの OS バージョンの変数は変わらなかった（調査 §5.1）。**unit 265 以降の呼び出しの実測は無い**。
  - `Software`、`Software\Microsoft`、`Software\Microsoft\Windows NT` そのものを開くリテラルは、調査の関数一覧（163 関数）に無かった。`SOFTWARE\Microsoft\Windows NT\CurrentVersion\FontSubstitutes`（`f_5d1ac0`。unit 95 だけ）は `Fonts` の兄弟で作られないので、2 のまま。
  - 調査 §3.2 の表の `SOFTWARE\Microsoft\Cryptography`（`MachineGuid`）、`SOFTWARE\Microsoft\Windows\...`（`Windows\Shell\Associations\...`）、`Software\Microsoft\Windows\CurrentVersion\...`（`Internet settings`、`App Paths\Cubase.exe`）は、`Cryptography` と `Windows`（`Windows NT` とは別のキー）が作られないので、`Fonts` の create の後も 2 のまま（キー名の比較で読んだ。実行はしていない）。
- **`RegDeleteKeyW(HKLM, L"Software")` を、ゲストが呼べば消える**（この仮想レジストリは ACL を持たない）。消えた後は `Software` を作り直さない（`reg_boot` は 1 回だけ）。FLEngine の削除の呼び出しは、調査 §3.1 では unit 145〜200 に静的には届かない（`f_5afa50`、`f_5afd50`）。実際にそのパスを削除するかは未確認。
- **unit 153 の実行は今回していない**。単独構成の a4 は unit 144 の `LoadLibraryW "dsp_ippv2_x64.dll"` で止まる（145/942 のまま）。2 モジュール構成は DllMain が `GetStringTypeW` で止まる（0084 の後。`docs/investigations/two-module-after-0084.md`）。unit 153 と同じ形の呼び出しはテストで示した。
- **この停止の次は同梱 DLL の `quickfontcache_x64.dll`（unit 158）**（調査 §6.1、§6.3。仮の値を入れた構成の実測）。この決定だけでは unit 159 を越えない。
- **古い記述が残るファイル**（この作業では編集の範囲外なので触っていない）: `lift/runtime/fle_reg.h` の冒頭のコメント（「the five predefined roots exist and are empty」）、`lift/runtime/fle_ole.c` の `ProgIDFromCLSID` のコメント（「The virtual registry (fle_reg.c) is empty at start」。動作は変わらない。`SOFTWARE\Classes\CLSID\...` は 2 のまま）、`docs/investigations/import-vfs-spec.md` の RegCreateKeyExW の行（HKLM／HKU 直下の作成禁止。規則は残るので記述は今も正しい）、調査 `registry-unit153.md` §1.3（0085 の前の状態の記述）。
