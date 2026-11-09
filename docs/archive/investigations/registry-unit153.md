# unit 153 の RegCreateKeyExW 停止（HKLM\Software\Microsoft\Windows NT\CurrentVersion\Fonts）

実行日: 2026-10-24。**調査のみ。この文書は何も決めない**（意見は §7 に別枠）。runtime、fe-lift、docs/decisions、既存のテスト、既存の docs は編集していない。作業物は `lift/out-regstop/`（ルートの `.gitignore` の `lift/out*/` に当たるので `git status` に出ない）。

背景: `stage23-two-module-real-dll.md` §5.2 と `loadlibrary-names-classification.md` §7.1 に記録された停止。DllMain の後（仮の値を gdb で入れた構成）、LoadLibrary の後の次の停止が `RegCreateKeyExW: creating a direct child of HKEY_LOCAL_MACHINE or HKEY_USERS is not allowed (no code on the page)`（`lift/runtime/fle_reg.c:348`）。

## 0. 凡例と前提

- **実測** = このセッションで実行した出力。**読み** = 逆アセンブル（capstone。`lift/out-regstop/tools/rdis.py`）とソースの読み。**未確認** = 確認していない。**一次に無い** = 取得した Microsoft Learn のページに書かれていない。
- **仮の値** = gdb が止まった所に一時的に入れた値。**決定ではなく、runtime の挙動でもない**。runtime には何も実装していない（`lift/runtime/` は編集していない。`git diff --stat HEAD -- lift/runtime/fle_reg.c fle_reg.h fle_load.c fle_modules.c` は出力なし）。
- 「unit N」は `lift/out-loadlib/units.tsv` の index（0 始まり）。ゲストの InitCount（gdb のログの `IC=`）は **index + 1**（決定 0060）。このキーの停止は **IC=154 = unit 153**（proc 0x665b90）。
- 依頼文の HEAD は 7a9af0f。作業中に別セッションが 9ae09a0（docs のみ）をコミットした。私は git の変更系操作をしていない。`docs/README.md` と `lift/harness/two/patches_stage1.py` の変更、`docs/investigations/mbtwc-high-bytes.md` は私のものではない（作業の途中で現れた。触っていない）。
- 測定に使ったバイナリは `lift/out-stage2/two/v-tbl2/native/two-native`（2026-10-23 18:15 JST のビルド。決定 0083 のコミット c68dbce の 22:29 より前）。`fle_reg.c` の最後のコミットは 2026-10-21 なので、レジストリ側のコードはこのバイナリと HEAD で同じ（読み）。
- 外部仕様は Microsoft Learn のみ（取得は 2026-10-24。生のページと平文は `lift/out-regstop/learn/`）。非公式の資料は §4.5 に別枠で置き、結論に使っていない。
- 測定の構成: `lift/harness/two/patches_chain.py` の段 1〜6（GetStartupInfoW の cb、MultiByteToWideChar、GetStringTypeW、LCMapStringW、WideCharToMultiByte、LoadLibraryW の avrt、powrprof、blend2d を NULL）を**読み取り専用で再利用**し、そこへ私の追加の段（`lift/out-regstop/tools/extra_regstop.py`。ログ出力と §6 の仮の値）を足した。基線の再現は `run/base1-*`: **init_count 154/942、関数 950（distinct）、入口 26,415、停止は同じ RegCreateKeyExW**（`stage23` §5.2 の数値と一致）。

## 1. なぜ停止するのか

### 1.1 条件の実装（事実。`fle_reg.c` 318〜359 行を読んだ）

```c
cur = base;
for (i = 0; i < kp.n; i++) {
    reg_key *k = kid_find(cur, kp.p[i], kp.l[i]);
    if (!k) {
        if (cur == &roots[2] || cur == &roots[3])
            fle_stop(api, "creating a direct child of HKEY_LOCAL_MACHINE or HKEY_USERS is not allowed (no code on the page)");
        ...
        k = kid_add(cur, kp.p[i], kp.l[i]);   disp = 1;
    }
    cur = k;
}
```

- `roots[]` の添字 2、3 は `root_low[]` の `0x80000002`（HKLM）、`0x80000003`（HKU）。
- 判定されるのは「**いま作ろうとしている成分の親が、HKLM か HKU のルートそのものか**」だけ。パスを `\` で分け、**先頭から順に探し、最初に見つからなかった成分**でこの検査に当たる。
- この lpSubKey（`Software\Microsoft\Windows NT\CurrentVersion\Fonts`）では、仮想レジストリが空なので **1 段目の `Software` が HKLM に無く、その時点で停止**する。`Microsoft` 以降は見られない。停止メッセージの「direct child」は、パス全体ではなく **1 段目の成分（`Software`）が HKLM の直下の子**だという意味。
- 1 段目が既にあれば、2 段目以降の親は `roots[]` ではないので判定に当たらない。**実測**（§6.2 のモード d、e）: HKLM に空の `Software` を 1 つ置くと、同じ呼び出しが 0 を返して成功した（`Microsoft`、`Windows NT`、`CurrentVersion`、`Fonts` は runtime 自身の `kid_add` が作った）。
- HKCU（`roots[1]`）と HKCR は判定に当たらない。HKLM の下のキーを開いたハンドル（`hKey` が `roots[2]` ではない）も当たらない。
- 他の引数の扱い（同じ関数。実測の入力はすべてこの範囲）: `Reserved` 0、`dwOptions` 0 か 1、`lpSecurityAttributes` NULL、`samDesired` が `0x000F003F` の範囲内、`phkResult` 非 NULL。`lpClass` は無視。`lpdwDisposition` が非 NULL なら 1（作った）か 2（既存）を書く。

### 1.2 ページの記述（Learn。§4 に引用）

`RegCreateKeyExW` のページ Remarks に「An application cannot create a key that is a direct child of HKEY_USERS or HKEY_LOCAL_MACHINE. An application can create subkeys in lower levels of the HKEY_USERS or HKEY_LOCAL_MACHINE trees.」とある。**エラー番号は書かれていない**（Return value は「a nonzero error code defined in Winerror.h」だけ）。runtime のコメント（`fle_reg.c` 315 行）も同じ事情を書いている。

### 1.3 決定 0018 の該当記述（事実）

- 0018 の本文に `HKLM`、`HKU`、`直下`、`HKEY_LOCAL` の語は**無い**（grep 0 件）。この停止条件は 0018 の本文ではなく、`fle_reg.c` のコメントと `import-vfs-spec.md` の表（「HKLM／HKU 直下の作成禁止（番号なし、abort）」）、テスト `lift/crates/fe-lift/tests/reg.rs` の `create-under-hklm`（HKLM に `Q` を作ると停止）に書かれている。
- 0018 から導かれる根拠は 3 つ（読み）: (i)「レジストリは、メモリ内の仮想レジストリ。初期状態は空」（決定の節）、(ii)「仕様が定める範囲を超える入力は abort で止める（黙って成功にしない）」（決定の節）、(iii) §6「定義済みの 5 つのルートが最初から存在し、中身は空」。
- つまり停止は「ページが禁止すると書く」＋「番号を書かない」→ abort、と「ルートが空」の組み合わせで出ている。**実機では `Software` が最初からあるので、この規則には当たらない**（ページの根拠は §4.3）。テスト `registry_starts_empty` は HKLM の `SOFTWARE\Microsoft\Windows\CurrentVersion` と HKCU の `Software` が 2（ERROR_FILE_NOT_FOUND）で失敗することを固定している。

## 2. ゲストの使い方（読み。`lift/out-regstop/f_*.dis`、実測で裏取り）

### 2.1 呼び出しの連鎖

| 関数 | 役割（読み） |
|---|---|
| `f_665b90`（unit 153 の proc） | `sub [0x15f4614],1; cmp [0x15f4614],-1; jne; call f_664fd0`。参照カウント 0 → −1 のとき 1 回だけ本体を呼ぶ形（Delphi の unit 初期化の定型と読める） |
| `f_664fd0` | 本体。レジストリのオブジェクト（`f_5af120`）を作り、ルートを HKLM にし、`Fonts` キーを開き、値を列挙して、フォント表（グローバル 0x15f4618）を作る |
| `f_5af120` | オブジェクトの生成。`[obj+0x2c]`（アクセス権）= `0xF003F`（KEY_ALL_ACCESS）、`[obj+0x18]` = 1、ルート = `0xFFFFFFFF80000001`（HKCU）を設定（0x5af151〜0x5af16c） |
| `f_5af3d0` | ルートの設定（`0xFFFFFFFF80000002` = HKLM を 0x6650af で渡す）。以前のルートが所有物なら `RegCloseKey` |
| `f_5af5e0(obj, path, CanCreate)` | キーを開く。`CanCreate`（r8b）= 1 かつパスが非空 → `RegCreateKeyExW`、そうでなければ `RegOpenKeyExW`。`f_5af380` で結果を `[obj+0x30]` に保存し、0 なら真を返す |
| `f_422ff0` | `RegCreateKeyExW` の IAT thunk（`jmp [0x1971238]`） |

ライブラリ名の対応は読み。バイナリには `TRegistry`、`System.Win.Registry` の文字列があり、`f_5af7c0`（読み取り用の open）が `0x20019`、`0x20009`、`0x1` の順にアクセス権を落として 3 回開く形（実測。§3）が Delphi の `OpenKeyReadOnly` と合う。メソッド名は私の推定で、関数の同定そのものは逆アセンブルの内容から。

### 2.2 この RegCreateKeyExW の目的と引数（実測。`run/base1-gdb.log`）

目的は**書き込みではなく、インストール済みフォント一覧の読み取り**（読み）。根拠: `f_664fd0` は書き込み系のラッパー（`f_5b0b30` = RegSetValueExW、`f_5af470` = CreateKey、`f_5afa50`/`f_5afd50` = 削除）を 1 つも呼ばない。`CanCreate` = 1 と KEY_ALL_ACCESS は、Delphi の `OpenKey(key, True)` と `TRegistry` の既定のアクセス権（コンストラクタが設定）の組で、一覧を読むだけの用途でも作成呼び出しになる。

| 引数 | 値（実測） |
|---|---|
| hKey | `0xffffffff80000002`（HKLM） |
| lpSubKey | `Software\Microsoft\Windows NT\CurrentVersion\Fonts`（UTF-16、0x665720） |
| Reserved | 0 |
| lpClass | NULL |
| dwOptions | **0**（REG_OPTION_NON_VOLATILE） |
| samDesired | **0xF003F**（KEY_ALL_ACCESS） |
| lpSecurityAttributes | NULL |
| phkResult | ゲストのスタック上（`[rbp+0x88]`） |
| lpdwDisposition | **非 NULL**（スタック上の `[rbp+0x7c]`。`f_5af5e0` は 0x5af6f6 で `lea` するだけで**この値を読まない**＝読み） |

（初版の harness は 5 番目以降の引数を 1 スロットずらして読んでいた。`run/base0-*` はその版の出力で、引数 5 以降が誤り。`base1` 以降が正しい版。`base0` は結論に使っていない。）

### 2.3 呼び出し後の分岐（読み）

- **失敗したとき**（非 0）: `f_5af5e0` は偽を返す。例外は**投げない**（`f_5af470`＝CreateKey のラッパーは失敗時に例外を投げる形だが、ここは `OpenKey` 経路）。`f_664fd0` は `test al,al; je 0x665567` でクリーンアップへ飛び、2 つの一時オブジェクトを解放して `f_5af390`（閉じる）して戻る。**メッセージも再試行も無い**。フォント表は作られない。エラー番号は `[obj+0x30]` に保存されるが、`f_664fd0` 内では 0／非 0 の判定にしか使われない（`+0x30` を別のレジスタ経由で読む命令は `f_664fd0` に無い。他の関数が読むかは未確認）。
- **既存のキーを開いたとき／新規作成したとき**: **区別しない**。disposition を読まないので、同じ経路に進む。
- 成功後の順序（読み）: (1) `f_5b0150`（値名の列挙）: `RegQueryInfoKeyW`（`f_5afd80`）で値の個数を取り、0 から個数−1 まで `RegEnumValueW(h, i, name, &len, NULL, NULL, NULL, NULL)`（**名前だけ**。lpType、lpData、lpcbData は NULL）。(2) 名前ごとに `f_5b0cb0`（`RegQueryValueExW(h, name, NULL, &type, NULL, &size)` で存在確認）→ 存在すれば `f_5b03a0`（サイズを取り、データを読む。型が REG_SZ か REG_EXPAND_SZ でなければ `f_5aee80` で例外を投げる形）。(3) 名前が `(TrueType)` か `(OpenType)` に当たり、かつデータの拡張子がグローバルの表（0x13e83a0 の 3 ポインタ: `.ttf`、`.ttc`、`.otf`。実測）に当たるときだけ、` Bold`、` Italic`、` Underline`、` Strikeout` の有無を調べて、20 バイトの項目を 0x15f4618 の配列へ追加する。(4) 最後に `f_5af390`（`[obj+0x18]` が 1 なので `RegFlushKey` は呼ばず `RegCloseKey` だけ）。
- **空のキーのとき**（§6 の仮の値）: `RegQueryInfoKeyW` が個数 0 を返し、列挙のループもフォント表のループも 0 回。**実測の IC=154 の呼び出し列は `RegCreateKeyExW` → `RegQueryInfoKeyW` → `RegCloseKey` の 3 回だけ**で、`RegEnumValueW` も `RegQueryValueExW` も無い。

## 3. ゲストが触るレジストリのキー（静的な一覧。FLEngine_x64.dll）

### 3.1 数え方（実測。`lift/out-regstop/callsites.tsv`、`regusers.tsv`、`regfuncs.tsv`、`regtable.txt`）

- advapi32 の import 14 名のうち Reg* は 12（IAT 0x19711d0〜0x1971238。`tools/regimports.py`）。IAT の thunk は 15 個（0x4071f0、0x407200、0x407210 と 0x422fe0〜0x423110）。`call thunk` の呼び出し点は **184 件**: RegCloseKey 95、RegQueryValueExW 42、RegOpenKeyExA 21、RegOpenKeyExW 14、RegEnumKeyExW 3、RegEnumValueW 2、RegCreateKeyExW 2、RegSetValueExW 1、RegQueryInfoKeyW 1、RegFlushKey 1、RegDeleteValueW 1、RegDeleteKeyW 1。`call [IAT スロット]` の直接形は .text に **0 件**。遅延 import の advapi32 は `RegDeleteKeyExW` 1 名（名前だけ確認。呼び出し点は調べていない）。
- Delphi の `TRegistry` 相当の薄いラッパー（`f_5af120` 生成、`f_5af3d0` ルート設定、`f_5af5e0` open（CanCreate 付き）、`f_5af470` create、`f_5af7c0` 読み取り用 open、`f_5b03a0` 文字列の読み取り、`f_5b0cb0` 値の存在確認、`f_5b0150` 値名の列挙、`f_5afa50` 削除、`f_5b0b30` 値の書き込み）の呼び出し点は別に数えた: 生成 63、ReadString 39、読み取り用 open 38、ValueExists 15、OpenKey 13、CreateKey 13、削除 5、値名の列挙 4。**ラッパーのメソッド名は私の推定**（関数の中身は逆アセンブルで確認。名前は Delphi の `System.Win.Registry` の文字列が実在することからの対応づけ）。
- レジストリの呼び出しを含む関数は **163**。うち IAT thunk を直接呼ぶ関数は 103（`callsites-reach.tsv` の関数の distinct 数）、残りの 60 はラッパー経由だけ（`regfuncs.tsv` との差。直接呼ぶだけで `regfuncs.tsv` に無い関数は 0）。
- 静的に届く unit は `lift/out-loadlib/` の `units.tsv`、`edges-direct.tsv`（D: 直接の call/jmp のみ）、`edges-ptr.tsv`（P: D に `lea reg,[rip+関数]` を足す。過大見積もり）を読み取り専用で再利用して逆向きに辿った（`tools/regreach.py`、`regfuncs.py`）。**vtable とレジスタ経由の間接 call はどちらのグラフにも無い**ので、「届かない」は「実行されない」ではない。
- hKey の渡し方（読み）: 直接呼びの open は**定義済みハンドルの即値**。`f_419990` は `mov ecx,0x80000001`（ゼロ拡張。実測の rcx = `0x80000001`）、`f_44a9a0`、`f_454090`、`f_865c90` と RegOpenKeyExA の 21 件は `mov rcx,0xffffffff80000002`（符号拡張。実測の rcx = `0xffffffff80000002`）。ラッパー経由は、`f_5af120` が既定でルートを HKCU にし、`f_5af3d0` で変える。open の基点は「現在のキーのハンドル（`[obj+8]`、保存されたハンドル）か、無ければルート」（`f_5af440`）。HKU（`0x80000003`）の即値は**レジストリを使う関数の中に 0 件**（`regtable.txt`）。
- `RegSetValueExW` と `RegCreateKeyExW` のうち、**unit 145〜200 から静的に届くのは `f_5af5e0` の RegCreateKeyExW（unit 153 だけ）**。`f_5af470`（CreateKey）と `f_5b0b30`（RegSetValueExW）は P で unit 614、D で unit 895 以降。削除（`f_5afa50`、`f_5afd50`）と列挙のうち `f_5afe30`（unit 830）、`f_5b11d0`、`f_5b0fb0` は 145〜200 に届かない。

### 3.2 キーの表（データを持つ関数。`regtable.txt`、`createsites.tsv`）

「静的 unit」は P の最小 unit と、145〜200 に入る unit。「実測」は §6.2 のモード d の run（`run/moded3-gdb.log`。IC = unit + 1）で、unit 153 の呼び出しだけが成功（0）、他の open はすべて 2。

| 関数 | 呼び出し | hKey | サブキー（UTF-16 のリテラル） | 値名 | 用途（読み） | 静的 unit | 実測 |
|---|---|---|---|---|---|---|---|
| `f_419990` | RegOpenKeyExW ×6、RegQueryValueExW ×4、RegCloseKey | HKCU ×4、HKLM ×2（即値） | `Software\Embarcadero\Locales`（HKCU、HKLM）、`Software\CodeGear\Locales`（HKCU、HKLM）、`Software\Borland\Locales`（HKCU）、`Software\Borland\Delphi\Locales`（HKCU） | 計算した名前と、無名（既定）値（0x419dc8 のファイル上の先頭 2 バイトが 0 なので空文字列） | Delphi の RTL のロケール選択 | 最小 12。145〜200: 149、153、158、160、189、190 | unit 12 と 17 で 6 回ずつ（samDesired 0x20019）。すべて 2 |
| `f_454090` | RegOpenKeyExW、RegQueryValueExW ×9、RegCloseKey | HKLM | `SOFTWARE\Microsoft\Windows NT\CurrentVersion` | `CurrentMajorVersionNumber`、`CurrentMinorVersionNumber`、`CurrentVersion`、`CurrentBuild`、`CurrentBuildNumber` | GetVersionExW の答えが 6.1 超のときの OS バージョンの上書き（`versioninfo-size0-spec.md`） | 最小 16（1 unit） | unit 16 で open → 2（以降の query は走らない） |
| `f_44a9a0` | RegOpenKeyExW、RegQueryValueExW ×5、RegCloseKey | HKLM | `SOFTWARE\Microsoft\Windows NT\CurrentVersion` | `DisplayVersion`、`ReleaseId`、`UBR`（`UBR` は 4 バイトで 0x15ec800 へ） | OS の表示バージョン | 最小 95。145〜200: 153 | unit 95 で open → 2 |
| `f_5d1ac0` | ラッパー（生成、ルート設定、読み取り用 open、ReadString） | HKLM | `SOFTWARE\Microsoft\Windows NT\CurrentVersion\FontSubstitutes` | `MS Shell Dlg 2`（ReadString の第 3 引数を値名として渡す形。読み）。近くに `Tahoma` | フォント名の置換表 | 最小 95（1 unit） | unit 95 で 3 回（0x20019、0x20009、0x1）。すべて 2 |
| `f_664fd0` | ラッパー（CanCreate=1 の open、値名の列挙、ValueExists、ReadString） | HKLM（`f_5af3d0` に即値） | `Software\Microsoft\Windows NT\CurrentVersion\Fonts` | 列挙で得た名前（`(TrueType)`、`(OpenType)` で終わるもの。` Bold`、` Italic`、` Underline`、` Strikeout` を後置判定） | インストール済みフォントの一覧（§2） | 153 のみ | **unit 153: create、QueryInfoKey、Close の 3 回。仮の値のときだけ成功** |
| `f_652830` | ラッパー（読み取り用 open、ReadString） | 不明（即値なし。既定の HKCU かループで変える。未確認） | `Software\Image-Line\Shared\Paths` | `FL Studio engine`（のち `FLEngine_x64.dll` を連結） | FL Studio のエンジン DLL の場所 | 最小 144。145〜200: 149、158、190 | unit 144 で 12 回、149 と 158 で 6 回ずつ。すべて 2 |
| `f_652e60`、`f_652ff0` | ラッパー（読み取り用 open、CanCreate=1 の open） | HKCU | `Software\Image-Line\Shared\Paths` | `Shared data` | 共有データの場所（`f_652ff0` は作る側） | P: 614（D: 895） | 未実測 |
| `f_7a62d0`、`f_7a6bc0`、`f_7a6c80` | ラッパー | HKLM、HKCU | `software\microsoft\Windows NT\CurrentVersion`、`Software\Wow6432Node`、`Software\Classes\http\shell\open\command`、`SOFTWARE\Microsoft\Windows\Shell\Associations\UrlAssociations\https\UserChoice` | `CurrentVersion`、`ProductName`、`ProgId` | OS 名の表示、既定ブラウザの判別 | 265 | 未実測 |
| `f_8e6150` ほか 10 関数 | ラッパー | HKCU、HKLM | `Software\Image-Line\Registrations\`、同 `FL Studio 26.1`、`FL Data`、`Token`、`FL Studio user name`、`FL Studio webpass`、`FLS Info` | `ILRegKey`、`Key`、`Date`、`FL Studio license plate` ほか | 登録・ライセンス | 830 以降、またはなし | 未実測 |
| 20 関数（`f_bf94e0`、`f_f40840`、`f_10c7cd0` ほか） | RegOpenKeyExA（ANSI のサブキー）+ RegQueryValueExW | HKLM（`mov rcx,0xffffffff80000002`） | `SOFTWARE\Microsoft\Cryptography` | `MachineGuid` | マシン ID | `f_f40840` が P:614（D:895）、`f_10c7cd0` が 830、他はなし | 未実測 |
| `f_865c90`、`f_843390` | RegOpenKeyExW、ラッパー | HKLM | `System\CurrentControlSet\Control\Keyboard Layouts\%.8x`、`\SYSTEM\CurrentControlSet\Control\Keyboard Layouts\` | `layout text`、`Layout File`、`KbdLayerDescriptor` | キーボードレイアウト名（`next-stop-getkeyboardlayout.md`） | なし | 未実測 |
| 設定 | ラッパー（読み、一部は CreateKey の書き込み側 13 件） | 主に HKCU（既定のルート） | `Software\Image-Line\Shared\General`、`\Shared\Language`、`\Shared\Browser\`、`Software\Image-Line\FL Studio 26\General`、`...\Favorite dirs`、`Software\Image-Line\FL Studio ` | `High Visibility`、`Input device type`、`Program Language`、`TabWidth`、`SplashImage`、`LastUpdateDate`、`LogStatisticsEnabled` ほか | ユーザー設定 | 245、272、614 以降、またはなし | 未実測 |
| その他 | ラッパー、RegOpenKeyExW | HKCU、HKLM、HKCR | `Software\Microsoft\Windows\CurrentVersion\Internet settings`（`ProxyEnable`、`ProxyServer`）、`Software\VST`（`VSTPluginsPath`）、`Software\Microsoft\Windows\CurrentVersion\App Paths\Cubase.exe`、`clsid\…\InprocServer32`、`software\asio` | 左記 | プロキシ、VST の場所、ASIO ドライバの列挙 | なし | 未実測 |

- **unit 145〜200 の起動で実際に走ったレジストリ呼び出し**（実測、仮の値つき。unit 153 を除く）は、`Software\Image-Line\Shared\Paths`（unit 144、149、158）だけ。他の unit 12、16、17、95 の呼び出しは unit 145 より前で、`f_419990`（Locales）と OS バージョン、FontSubstitutes。
- 書き込み系で unit 145〜200 に静的に届くのは §3.1 のとおり unit 153 の create だけ。**HKLM の下に作る呼び出しで、root が静的に読み取れたのは `f_664fd0` の 1 件だけ**（`CanCreate=1` の open 13 件のうち root が即値で分かるのは `f_652ff0`（HKCU）、`f_664fd0`（HKLM）、`f_8e6083`（HKCU）の 3 件。他は呼び出し元がルートを設定する）。
- 作成呼び出しの `lpSubKey` が**絶対（先頭が `\`）か相対か**は `f_5aeec0` が判定する（先頭が `\` なら偽）。今回の `Software\...` は相対で、基点は「現在のキーが無ければルート」なので HKLM（実測）。

## 4. Learn（Microsoft Learn）が言うこと

取得は 2026-10-24（`lift/out-regstop/learn/*.html` と `*.txt`）。各ページの `updated_at`（ページのメタ）と本文末の "Last updated on" を併記した。**引用は取得した本文のとおり**。

### 4.1 RegCreateKeyExW（`api/winreg/nf-winreg-regcreatekeyexw`。updated_at 2025-07-01）

**ページに書いてあること**:

- 「Creates the specified registry key. If the key already exists, the function opens it.」
- hKey: 「A handle to an open registry key. The calling process must have KEY_CREATE_SUB_KEY access to the key.」「Access for key creation is checked against the security descriptor of the registry key, not the access mask specified when the handle was obtained.」「it can be one of the following predefined keys: HKEY_CLASSES_ROOT、HKEY_CURRENT_CONFIG、HKEY_CURRENT_USER、HKEY_LOCAL_MACHINE、HKEY_USERS」。
- lpSubKey: 「The subkey specified must be a subkey of the key identified by the hKey parameter; it can be up to 32 levels deep in the registry tree.」「This parameter cannot be NULL.」
- 中間キー: 「The RegCreateKeyEx function creates all missing keys in the specified path. An application can take advantage of this behavior to create several keys at once.」
- **直下の子**: 「An application cannot create a key that is a direct child of HKEY_USERS or HKEY_LOCAL_MACHINE. An application can create subkeys in lower levels of the HKEY_USERS or HKEY_LOCAL_MACHINE trees.」
- dwOptions: 「REG_OPTION_NON_VOLATILE 0x00000000L: This key is not volatile; this is the default. The information is stored in a file and is preserved when the system is restarted.」
- samDesired: 「A mask that specifies the access rights for the key to be created.」
- lpdwDisposition: 「REG_CREATED_NEW_KEY 0x00000001L: The key did not exist and was created.」「REG_OPENED_EXISTING_KEY 0x00000002L: The key existed and was simply opened without being changed.」「If lpdwDisposition is NULL, no disposition information is returned.」
- Return value: 「If the function fails, the return value is a nonzero error code defined in Winerror.h.」
- Requirements: Minimum supported client は Windows 2000 Professional [desktop apps only]。

**ページに書いていないこと**:

- 「direct child」を作ろうとしたときの**エラー番号**、呼び出し全体が失敗するのか、途中まで作るのか。
- 既存のキーを開く場合に、samDesired が要求する権限がセキュリティ記述子で許されないときの結果（Create のページは作成の権限を書く。Open のページが「The function fails if the security descriptor of the key does not permit the requested access」と書く）。
- 標準ハイブにどのキーがあるか（HKLM の直下に何があるか）。

**読み**: runtime の実装（§1.1）はこの記述を字義どおりに実装したもの。パスの 1 段目が HKLM の直下の子として新規になるときだけ止める。**停止の原因は実装の誤りではなく、仮想レジストリの初期状態が空であること**。

### 4.2 RegOpenKeyExW と Registry Key Security and Access Rights

- RegOpenKeyExW（updated_at 2025-07-01、"Last updated on 2023-02-09"）: 「Unlike the RegCreateKeyEx function, the RegOpenKeyEx function does not create the specified key if the key does not exist in the registry.」「The function fails if the security descriptor of the key does not permit the requested access for the calling process.」例のコードは `ERROR_FILE_NOT_FOUND` を「Key not found.」として扱う（欠落のキーは 2、ページの例）。
- Registry Key Security and Access Rights（updated_at 2025-03-11、"Last updated on 2021-01-07"）: 「When you call the RegOpenKeyEx function, the system checks the requested access rights against the key's security descriptor. If the user does not have the correct access to the registry key, the open operation fails.」KEY_ALL_ACCESS（0xF003F）、KEY_CREATE_SUB_KEY（0x0004: 「Required to create a subkey of a registry key.」）。「If you specify NULL, the key gets a default security descriptor. The ACLs in a default security descriptor for a key are inherited from its direct parent key.」
- 書いていないこと: 失敗の番号（`ERROR_ACCESS_DENIED` を書かない）。

### 4.3 HKLM\Software の存在（Predefined Keys、Registry Hives、Structure of the Registry）

**書いてあること**:

- Predefined Keys（updated_at 2025-03-11）: HKEY_CURRENT_CONFIG の行に「Information about the standard hardware configuration is stored under the Software and System keys of HKEY_LOCAL_MACHINE.」HKEY_LOCAL_MACHINE の行に「software-related information (such as server names and the location of the server)」。HKEY_CURRENT_USER の行に「Microsoft, for example, creates the HKEY_CURRENT_USER\Software\Microsoft key for its applications to use」（例示）。
- Registry Hives（updated_at 2025-03-11、"Last updated on 2021-01-07"）: 標準ハイブの表に `HKEY_LOCAL_MACHINE\Software`（Software、Software.log、Software.sav）、`\System`、`\SAM`、`\Security`、`HKEY_USERS\.DEFAULT`、`HKEY_CURRENT_USER`、`HKEY_CURRENT_CONFIG`。
- Structure of the Registry（updated_at 2025-04-15、"Last updated on 2021-06-02"）: 「The HKEY_LOCAL_MACHINE key has the following subkeys: HARDWARE, SAM, SECURITY, SOFTWARE, and SYSTEM.」（Registry Editor の図の説明）「A registry tree can be 512 levels deep. You can create up to 32 levels at a time through a single registry API call.」「Key names are not case sensitive.」
- Registry Virtualization（updated_at 2025-04-15、"Last updated on 2022-04-18"）: 「registry operations to the global store (HKEY_LOCAL_MACHINE\Software) are redirected to a per-user location」（Windows Vista 以降）。

**書いていないこと**: これらが **Windows 10 22H2 について**のものだという明記（上のページはどれも版を限定しない。Registry Hives の記述には Windows 2000、XP の注記がある）。HKLM\Software の下に何があるか（`Microsoft`、`Windows NT` など）。**一次に無い**: 「Windows 10 に `HKLM\Software\Microsoft\Windows NT\CurrentVersion` がある」。

HKLM のページ（`sysinfo/hkey-local-machine-key`）は 404。archived の `previous-versions/windows/it-pro/windows-server-2003/cc784983(v=ws.10)`（Windows Server 2003 の HKLM: Core Services）は 49 語の短いページで、キー名を挙げない。

### 4.4 一般ユーザー権限での HKLM への書き込み

**書いてあること**（Registry Virtualization）:

- 「Prior to Windows Vista, applications were typically run by administrators. … If these applications were run by a standard user, they would fail due to insufficient access rights. Windows Vista and later versions of Windows improve application compatibility for these applications by automatically redirecting these operations.」
- 「If the caller does not have write access to a key and attempts to write a value to it or create a subkey, the value is written to the virtual store.」
- **適用範囲**: 「Registry virtualization is enabled only for the following: 32-bit interactive processes. Keys in HKEY_LOCAL_MACHINE\Software. …」「Registry virtualization is disabled for the following: 64-bit processes. … Keys and subkeys of HKEY_LOCAL_MACHINE\Software\Classes, HKEY_LOCAL_MACHINE\Software\Microsoft\Windows, and HKEY_LOCAL_MACHINE\Software\Microsoft\Windows NT.」
- 「Applications designed for Windows Vista and later versions of Windows should not write to sensitive system areas, nor should they rely on virtualization to remedy any problems.」

**読み**: `FLEngine_x64.dll` は 64 ビット（PE の Machine = 0x8664。実測）で、キーは `Software\Microsoft\Windows NT` の下なので、**どちらの理由でも仮想化されない**。

**書いていないこと**: 標準ユーザーが `HKLM\Software\Microsoft\Windows NT\CurrentVersion\Fonts` に KEY_ALL_ACCESS で作成呼び出しをしたときの結果とエラー番号。そのキーの ACL。ゲストを動かす利用者が管理者か標準ユーザーか（決定 0014 も定義しない。`docs/decisions` に「管理者」「standard user」「elevat」の記述は無い）。

### 4.5 `Fonts` キーそのもの（Windows 10 に存在するか）

- 「Font Installation and Deletion」（`gdi/font-installation-and-deletion`。updated_at 2025-03-11、"Last updated on 2021-01-07"）が、このキーのパスを書く唯一の取得済み一次: 「Permanent fonts remain installed after reboot and are loaded by all created sessions. Call RemoveFontResource to force the current session to unload the font. Then, in the font registry key (HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts) find and remove the registry value associated with the font.」
- **このページは版を限定しない**。「Windows 10 に存在する」とは書かない。ページはこのキーに font ごとの値があるとして手順を書く。
- 「Windows 10 font list」（`typography/fonts/windows_10_font_list`）はフォント名とファイル名の表で、レジストリへの言及は **0 件**（取得したページを grep）。
- **結論**: 「Windows 10 22H2 に `HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts` が存在する」ことを明言する一次は**無い**。上の 1 ページは、現行の Win32 のドキュメントがこのキーを前提にしていることまで（導出）。

### 4.6 非公式の資料（別枠。**根拠にしない**）

Web 検索の抜粋で見かけただけで、ページ本文は取得していない。

- Microsoft Q&A（コミュニティの質問）の投稿が、Windows 10 の話として `[HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts] "Segoe UI (TrueType)"="segoeui.ttf"` を載せる。Learn のドメイン内だが、Microsoft の公式文書ではない。
- 第三者のサイト（renenyffenegger.ch、woshub.com）が、このキーを「インストール済みフォントの一覧」「Windows 10 の既定フォントの .reg」として説明する。matplotlib の issue が、per-user のフォントを `HKCU\Software\Microsoft\Windows NT\CurrentVersion\Fonts` に置くと書く（Windows 1809）。
- これらは、このキーが実機の Windows 10 にあることとゲストの使い方（§2）が合っていることを**補強するだけ**で、決定の根拠には使わない。

## 5. 選択肢（決めない）

共通の前提（実測・読み）: このゲストの `Fonts` 読み取りは、キーが**存在して空**でも、**開けない**（番号 5 などで失敗）でも、フォント表を空のまま作り終える（§2.3）。違いは「どこまで同じ起動経路で進むか」と「実機との差」。

### 5.1 (a) 仮想レジストリの初期内容に標準ハイブの骨組み（キー名のみ）を持たせる

- **内容**: 起動時に、少なくとも `HKLM\Software`（ページ: Registry Hives、Structure of the Registry が `SOFTWARE` を HKLM の子として挙げる）を作る。必要なら `Microsoft`、`Windows NT`、`CurrentVersion`、`Fonts` まで。**値は作らず、キー名だけ**。
- **決定 0014（Windows 10 22H2）との整合**: `HKLM\Software` までは Learn の記述（§4.3）で支持される（版を限定しないページ）。`Microsoft\Windows NT\CurrentVersion` と `Fonts` は**一次に Windows 10 の記述が無い**（§4.5。`Fonts` はパスだけが一次にある）。`CurrentVersion` を作ると 0014 の `GetVersionExW` の答え（10.0、build 19045）と食い違う可能性が出る（下の帰結）。
- **ゲストの後続の動作への帰結（実測。モード d、e、f。§6.2）**:
  - `Software` だけ（モード d、e）: 本件の create は成功し、`Fonts` が空で作られる。**他の呼び出しの結果は基線と同じ**（unit 12〜158 の open 41 回はすべて 2 のまま。ログ比較で一致）。到達は IC=159 へ進む。
  - `Software\Microsoft\Windows NT\CurrentVersion` まで作る（モード f）: **unit 16 と unit 95 の open が 2 から 0 に変わる**。`CurrentMajorVersionNumber`、`CurrentVersion`、`CurrentBuild`（unit 16）と `DisplayVersion`、`ReleaseId`、`UBR`（unit 95）の読み取りが走り、すべて 2（値が無い）。open は成功するのに値が全部無い、という実機にない組み合わせになる。ゲストの変数 `0x15ec580/84/88/800` は**変わらなかった**（build = 0x4a65 = 19045、major = 10、minor = 0、UBR = 0）ので、ゲストの OS バージョンの値には影響しなかった（実測）。ただし `CurrentVersion` を作ると値の読み取りの経路が増える。
  - 値を足す（`CurrentBuildNumber` など）かどうかは別の話で、**0014 の再定義が要る**（この選択肢の範囲外。未測定）。
- **決定 0018 の改訂**: **要る**。0018 は「初期状態は空」と書く。「中身は空」は §6 でルート 5 つについても言っている。改訂するのは決定の節と §6。`import-vfs-spec.md` の表の備考と、テスト `registry_starts_empty`（HKLM の `SOFTWARE\Microsoft\Windows\CurrentVersion` が 2、HKCU の `Software` が 2）の期待値も同時に変わる。
- **Learn が支持する範囲**: 「HKLM に Software がある」まで（§4.3）。`Microsoft`、`Windows NT` 以降は支持しない（一次に無い）。

### 5.2 (b) RegCreateKeyExW が HKLM / HKU の直下の子を作ることを許す（ページの記述の範囲で）

- **内容**: 判定（`fle_reg.c:347`）を外す。
- **ページとの整合**: **整合しない**。ページは「cannot create a key that is a direct child of HKEY_USERS or HKEY_LOCAL_MACHINE」と書く。許すと、ページが禁じる動作を runtime が起こす。ページは「下位レベルのサブキーは作れる」と書くので、**パスの 2 段目以降を作る**のはページの範囲内（現行の実装もそう）。**1 段目を作る**のは範囲外。
- **帰結**: モード d と同じ結果（§6.2）。ただし、ゲストが `HKLM\Foo` のような直下のキーを作るコードを将来実行したとき、実機では失敗する動作が成功になる（黙って成功にしない、という 0018 の方針に反する）。
- **0018 の改訂**: 「仕様が定める範囲を超える入力は abort」と矛盾するので、**例外の追記が要る**。
- **Learn が支持する範囲**: 中間キー（2 段目以降）の作成は支持。直下の作成は**支持しない**（禁止と書く）。

### 5.3 (c) 失敗を返す

- **内容**: 1 段目を作ろうとするとき、停止の代わりにエラー番号を返し、phkResult と lpdwDisposition には触れない。
- **どのエラー番号か**: **ページは書かない**（§4.1）。候補は 5（ERROR_ACCESS_DENIED。通常は権限不足の番号。Registry Key Security のページが「the open operation fails」と書くが番号は無い）。`syserr-0-499` のページに 5 の名前（「Access is denied.」）はある。2（FILE_NOT_FOUND）は create には不自然（読み）。**どれも一次に無い**（0018 の §5「ページが番号を書かない失敗」の扱いと同じ種類の推測）。
- **ゲストの分岐は許容するか**（実測。モード c、`run/fin-c-*`）: **許容する**。5 を返すと `f_5af5e0` が偽を返し、`f_664fd0` がクリーンアップして戻る（§2.3）。unit 153 の呼び出しは create の 1 回だけになり（runtime を飛ばすので create は `[reg-create]` 行にだけ出る。`seq-finc.tsv` には open 41 回だけが残り、`RegQueryInfoKeyW` と `RegCloseKey` は無い）、IC=159 まで同じ経路で進んで、モード d と同じ次の停止（§6.3）に着く。**クラッシュも例外も出ない**。フォント表は空のまま（モード d と同じ）。
- **0014 との整合**: 「HKLM に書き込めない」は、標準ユーザーで動く Windows 10 22H2 として自然（読み。§4.4 は番号を書かない）。ただしこのゲストは直下ではなく 2 段目以降を作る呼び出しで、**実機で失敗する根拠は `Software` の存在ではなく ACL**。ページは ACL を書かない。
- **0018 の改訂**: 「番号が書かれていないのでまだ abort」を「番号を決めて返す」に変える。0018 の「仕様が定める範囲を超える入力は abort」の例外か、番号の推測を §5 に追加する形（既存の「推測で決めたエラー番号」の一覧に加える）。
- **Learn が支持する範囲**: 失敗を返すこと自体は「If the function fails, the return value is a nonzero error code」で支持。**番号は支持しない**。

### 5.4 (d) 停止のまま

- **内容**: 今のまま。
- **帰結**: unit 153 より先が測れない（§6 の測定は仮の値の上でしか出ない）。ゲストの動作は何も決まらない。
- **0014、0018**: 整合（改訂不要）。**ただし 0018 の「初期状態は空」が、実機の Windows 10 には無いはずの状態を前提にしている**（`Software` が無い HKLM）ことは残る。
- **Learn**: 停止の理由（直下の作成禁止）はページが支持する。実機と違う初期状態が原因という点は、ページからは出ない。

### 5.5 比較表

| | (a) 骨組み | (b) 直下を許す | (c) 失敗（5 など） | (d) 停止 |
|---|---|---|---|---|
| 本件の create | 成功（空の `Fonts`） | 成功 | 失敗（番号は仮） | 停止 |
| ページとの整合 | `Software` まではページが支持。以降は一次に無い | **ページの禁止に反する** | 番号は一次に無い | 整合 |
| 0018 の改訂 | 要る（初期状態、ルートの中身、テスト） | 要る（例外） | 要る（番号の推測を追記） | 不要 |
| 0014 との整合 | `Software` まで問題なし。`CurrentVersion` は §5.1 の注意 | 影響なし | 影響なし | 影響なし |
| 他の呼び出しへの影響（実測） | `Software` だけなら無し。`CurrentVersion` まで作ると unit 16、95 の open が成功に変わる | `Software` だけ作られる（無し） | 無し | — |
| unit 153 の呼び出し列（実測） | create、QueryInfoKey、Close | 同左 | create のみ | — |
| 次の停止（実測） | §6.3 | 同左 | 同左 | 本件 |

## 6. このキーの先で続く停止の見込み（unit 153〜160 付近）

**この節の「仮の値」はすべて gdb が一時的に入れた測定の道具で、決定でもruntime の挙動でもない**（runtime には何も実装していない）。

### 6.1 方法

`lift/out-regstop/tools/chain_regstop.sh NAME`（`REGSTOP_MODE`、`REGSTOP_MSGBOX`、`GDBI_SYMS`、`CHAIN_ABSENT` を環境変数で渡す）。`patches_chain.py` の段 1〜6 の上に、私の段 7（RegCreateKeyExW のログと仮の値）と、段 8 以降（他の Reg* 全 API のログのみ。値は変えない）を載せる。**3 段目までの仮の値**（依頼の上限）は次のとおり。

| 段 | 止まった所 | 仮の値（**決定ではない**） |
|---|---|---|
| 1 | 本件 `RegCreateKeyExW` | モード d、e: HKLM に空の `Software` を runtime 自身の `kid_add` で 1 つ作ってから runtime に任せる。モード b: hKey を HKCU に差し替える。モード c: 5 を返して runtime をスキップ。モード f: 空の `Software\Microsoft\Windows NT\CurrentVersion` を作る |
| 2 | `LoadLibraryW("quickfontcache_x64.dll")`（unit 158。`fle_load.c:174`） | NULL（`CHAIN_ABSENT` に追加。**`loadlibrary-names-classification.md` §8 の 3 にあるとおり、同梱 DLL を「無い」とは言えない**。測定の道具としてだけ） |
| 3 | `f_425180`（user32 の MessageBoxW の thunk）が `fle_unsupported` | 戻り値 1、何も表示しない（`REGSTOP_MSGBOX=1`。`f_425180` に gdb の breakpoint を置くのは `gdb_regstop.py`） |

数値（`run/fin-*` と `modeX3`。いずれも `stop:` は gdb の `-gdb.err`）:

| run | 構成 | 停止 | init_count | 関数（distinct） | 入口 |
|---|---|---|---:|---:|---:|
| `fin-log`、`base1` | 仮の値なし（段 1〜6 のみ） | `RegCreateKeyExW: creating a direct child …` | 154/942 | 950 | 26,415 |
| `modeb1`、`moded1`、`modec1` | モード b、d、c のみ | `LoadLibraryW: module "quickfontcache_x64.dll" is not a virtual module …` | 159 | 974、974、967 | 26,784、26,784、26,755 |
| `modec2`、`moded2` | 上 + quickfontcache を NULL | `unsupported instruction at 0x425180` | 159 | 968、975 | 26,760、26,789 |
| `fin-b`、`fin-c`、`fin-d` | 上 + MessageBoxW に 1 | `GetProcAddress with hModule 0 that is not a virtual module handle is not supported`（`fle_modules.c:204`） | 159 | 975、968、975 | 26,804、26,775、26,804 |
| `modee3`、`modef3` | モード e、f + MessageBoxW | 同上 | 159 | 975、976 | 26,804、26,812 |

**停止が移るだけで、init_count は 159（unit 158、proc 0x66d1a0）から先に進んでいない**。3 段で足りる範囲の測定。

### 6.2 レジストリの呼び出しの実測（`seq-moded3.tsv`。IC = unit + 1。モード d）

| unit | 呼び出し（キー） | 結果 |
|---|---|---|
| 12 | open ×6（`Software\Embarcadero\Locales`、`Software\CodeGear\Locales`、`Software\Borland\Locales`、`Software\Borland\Delphi\Locales`。HKCU と HKLM） | 全部 2 |
| 16 | open `HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion` | 2 |
| 17 | open ×6（Locales と同じ） | 全部 2 |
| 95 | open `HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion`、open `...\FontSubstitutes` ×3（0x20019、0x20009、0x1） | 全部 2 |
| 144 | open `Software\Image-Line\Shared\Paths`（HKCU、HKLM。アクセス権 3 通り ×2 ルート）を 2 回 | 全部 2 |
| 149 | 同上 6 回 | 全部 2 |
| **153** | **create `HKLM\Software\Microsoft\Windows NT\CurrentVersion\Fonts`、QueryInfoKey、Close** | **0（仮の値のとき）** |
| 158 | open `Software\Image-Line\Shared\Paths` 6 回 | 全部 2 |

- 記録した 44 回のレジストリ呼び出し（unit 153 を除く 41 回の open）は **すべて `RegOpenKeyExW` で、結果は 2**（`ERROR_FILE_NOT_FOUND`。実測）。`RegQueryValueExW`、`RegEnumValueW`、`RegSetValueExW`、`RegEnumKeyExW` は **1 回も呼ばれていない**（open が失敗するため）。
- モード b（HKCU に差し替え）、c、d、e の間で、unit 153 より前の呼び出しの列と結果は同じ（`base0` と `moded3` の最初の 35 回はログ比較で一致）。
- モード f だけ、unit 16 と 95 の open が 0 になり、値の `RegQueryValueExW` が出る（`seq-modef4.tsv`。すべて 2）。

### 6.3 次の停止の読み（unit 158、159 付近）

実測のとおり、unit 153 の次は **unit 158（proc 0x66d1a0）= `f_66d1a0` → `f_66d050`** で、`QuickFontCache` の読み込みに失敗する経路:

1. `f_66d050` が `f_665bd0` を呼ぶ。`f_665bd0` は `f_6539e0(L"QuickFontCache.dll", dl=1)` を呼ぶ（ゲストの LoadLibrary のラッパー。`loadlibrary-names-classification.md` §3 の #5。結果が 0 なら偽を返す）。
2. **`LoadLibraryW("quickfontcache_x64.dll")` が停止**（`fle_load.c:174`。仮想モジュールにも `fle_load_absent[]` にも無い。`f_6539e0` が名前に `_x64` を付ける。実測）。`loadlibrary-names-classification.md` §7.2 の「静的に届く最小 unit 158」と一致する。
3. 仮の値 NULL を入れると、`f_665bd0` が偽 → `f_66d050` が `MessageBoxW(0, L"QuickFontCache DLL not found!", L"Failed to load QuickFontCache", 0)`（0x66d06c〜0x66d07d。実測の引数）を呼ぶ → `f_425180` が `fle_unsupported`（`MessageBoxW` は応答表に無い。`seq_frontier.tsv` の unit 158 の行にも `user32.dll!MessageBoxW` と `shell32.dll!SHGetFolderPathW`）。
4. MessageBoxW に 1 を入れると、`f_66d050` は `f_66cfa0` を呼び、そこで **`GetProcAddress(*[0x15e0a50], L"AcquireSharedThreadBank")`**（0x66cfae。実測の停止で hModule（rcx）= 0、名前 = 0x66cfec の `AcquireSharedThreadBank`）。hModule は `mov rax,[0x15e0a50]; mov rcx,[rax]` で読まれ、イメージ上の `[0x15e0a50]` の 8 バイトは `0x15f4620`（読み。ファイルのバイト列）で、これは `f_665bd0` が `f_6539e0` の戻り値（LoadLibrary の結果）を保存する先（0x665bf2）と同じ。つまり NULL にした LoadLibrary の結果がそのまま `GetProcAddress` の hModule に渡っている）。`fle_modules.c:204` が「hModule NULL は仮想モジュールのハンドルでない」として **abort**。続く `f_668d30`（2,525 行）は読んでいない。
5. Learn（GetProcAddress）: 「A handle to the DLL module that contains the function or variable. The LoadLibrary, LoadLibraryEx, LoadPackagedLibrary, or GetModuleHandle function returns this handle.」「If the function fails, the return value is NULL.」**hModule が NULL のときの動作は書かれていない**（一次に無い）。

つまり、**unit 153 の次の停止は、レジストリではなく `quickfontcache_x64.dll`（同梱 DLL、`LoadLibraryW`）で、これは決定 0073、0029 の外部モジュールの扱い**。その先は、同梱 DLL が無いことへのゲストの対応（MessageBoxW とエラー後の `GetProcAddress(NULL, …)`）が続く。**ゲスト側のこの経路は、`quickfontcache_x64` を NULL にするという仮の値が作った**（実機では DLL があるので通らない。§7 の意見 5）。

### 6.4 unit 153〜160 付近のレジストリの読み（静的）

- unit 153〜160 で静的に届くレジストリ関数は `f_419990`（Locales。149、153、158、160 に届く）、`f_44a9a0`（153 だけ。DisplayVersion など）、`f_5af*`／`f_5b0*` のラッパー（153 は `f_664fd0` 経由）、`f_652830`（149、158、190）。`f_44a9a0` は unit 153 で届くと静的には出るが、**実測の IC=154 では呼ばれなかった**（IC=96 で呼ばれた。IC=154 の列は create、QueryInfoKey、Close の 3 回）。つまり「静的な到達」は「その unit で実行される」を意味しない（§3.1 のとおり）。
- **レジストリが原因で、unit 153〜160 のうちで次に止まる見込み**: **実測では無かった**（上の 41 回の open はすべて 2 で、ゲストは止まらず進んだ）。静的には、160 と 189、190 に `f_419990`（Locales）が届くが、unit 12、17 で同じ関数が 2 のまま通った（実測）ので、読み取りが同じ形で失敗するなら止まらない（読み）。
- 書き込み系（`RegSetValueExW`、`RegDeleteKeyW`、`f_5af470` の create）は、145〜200 に静的に届くものは本件だけ（§3.1）。

## 7. 意見（別枠。決めない。PM の判断の材料）

この節は作業者の意見で、事実の節ではない。

1. **停止の原因は実装の誤りではなく、0018 の「初期状態は空」が実機の Windows 10 にない状態（`Software` の無い HKLM）を作っていること**。ページ（§4.1）の「直下の子は作れない」は実機でも真で、runtime はそれを守っている。実機でこの呼び出しが成功するのは、`HKLM\Software` が最初からあるから（Registry Hives、Structure of the Registry が `SOFTWARE` を標準ハイブとして挙げる）。
2. **(a) の最小形（`HKLM\Software` を 1 つだけ作る。実測のモード d、e）が、一次に最も近く、他への影響が最も小さい**（実測: unit 12〜158 の他の呼び出しが変わらない）。決定 0014 との整合も保てる。これより深く（`Microsoft`、`Windows NT`、`CurrentVersion`、`Fonts`）は、一次に Windows 10 の記述が無い（§4.5）うえ、`CurrentVersion` まで作ると unit 16 と 95 の読み取り経路が変わる（実測のモード f。ゲストの OS バージョンの値には影響しなかったが、open が成功して値が全部無い組み合わせになる）。
3. **(c) の失敗は、番号が一次に無い**。ゲストの分岐は許容する（実測）ので機能上の問題は小さいが、番号の推測（5 など）を 0018 に足すことになる。「標準ユーザーで動く」という環境の定義が 0014 に無いことが前提の不確かさ。
4. **(b) は勧めない**。ページの禁止に反する。
5. **この停止を直しても、次は同梱 DLL の `quickfontcache_x64.dll`（unit 158）で止まる**（実測。`loadlibrary-names-classification.md` §8 の 3 と同じ種類。決定 0073、0029 の外部モジュールの扱い）。レジストリ側だけを決めても unit 159 を越えない。`Fonts` を読む側（unit 153）は、キーが空でも開けなくても同じ形で終わる（実測のモード c と d）ので、**Fonts の中身（値）は今は要らない**。
6. 仮のフォント一覧（値）を入れるかは別の判断。入れる場合は、決定 0014 の「Windows 10 22H2」に合う一次が必要で、**Learn の Windows 10 font list はフォント名とファイル名の表でレジストリの値ではない**（§4.5）。`(TrueType)` を後置した値名と拡張子の対応は、一次のどこにも書かれていない。
7. テスト `registry_starts_empty` と `create-under-hklm` は、0018 の現行の記述を固定している。(a) を採るなら、後者は 1 段目が既にある場合のケースを足す形になる。

## 8. 未確認

- ゲストが `Fonts` の値（フォント表）を**後で使う**経路（0x15f4618 の読み手）。`f_665860` などが読むことまでは見たが、この表が空のときにゲストが何をするかは追っていない。
- `f_668d30`（`f_66d050` から呼ばれる 2,525 行の関数）の中身。
- `f_652830` ほか、hKey が呼び出し元で決まるラッパーの root（即値なし）。
- ラッパーのメソッド名（私の推定）。`f_5af7c0`、`f_5afa50` の名前の「?」。
- 実機の Windows 10 22H2 での `HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts` の存在と、標準ユーザーでの create の結果（番号）。一次に無い。
- 決定 0083 の後の再ビルドで同じ停止になるか（測定に使ったバイナリは 0083 の前）。`fle_reg.c` は 0083 の影響を受けない（読み）が、DllMain を通る仮の値の構成が 0083 の後で同じかは未測定。
- `RegDeleteKeyExW`（遅延 import）の呼び出し点。
- `dwOptions`、`samDesired` が他の呼び出し元（`f_5af470` など）でどの値で渡されるか（`f_5af470` は `samDesired = (obj+0x2c & 0x300) | 0xF003F` と読めるが、実行していない）。
- 項目 6 の「最大 3 段」は守った（`Software` の仮置き、quickfontcache の NULL、MessageBoxW の 1）。ただし、モード b、c、d、e、f を比較のために分けて走らせている（どれも「3 段」の範囲内の組み合わせで、4 段目以降は入れていない）。

## 9. 作成物と限界

作業物は `lift/out-regstop/`（`.gitignore` の `lift/out*/` で追跡されない）:

- `callsites.tsv`（IAT thunk の呼び出し点 184 件）、`callsites-reach.tsv`（各点の関数と静的な到達 unit）、`regusers.tsv`（ラッパーの呼び出し点）、`regfuncs.tsv`（163 関数）、`regtable.txt`、`createsites.tsv`、`seq-moded3.tsv`、`seq-modef4.tsv`、`seq-finc.tsv`。
- `f_*.dis`: 読んだ関数の逆アセンブル（`f_664fd0`、`f_5af5e0`、`f_5af470`、`f_665b90`、`f_5af120` ほか）。
- `learn/`: 取得した Learn のページ（`*.html` と、本文だけの `*.txt`）。`hklm-key.html` は 404（本文なし）。
- `run/`: gdb の run 16 個分（`base0`、`base1`、`modeb1`、`modec1`、`modec2`、`moded1`、`moded2`、`moded3`、`modee3`、`modee4`、`modef3`、`modef4`、`fin-b`、`fin-c`、`fin-d`、`fin-log`。各 `-gdb.log`、`-gdb.out`、`-gdb.err`、`-imports.txt`）。`base0` は引数のずれた初版（§2.2）。
- `tools/`: `rdis.py`、`callsites.py`、`regimports.py`、`regreach.py`、`regusers.py`、`regtable.py`、`regfuncs.py`、`createsites.py`、`seqsummary.py`、`html2txt.py`、`chain_regstop.sh`、`gdb_regstop.py`、`extra_regstop.py`（既存のハーネス `lift/harness/two/` と `lift/out-stage2/`、`lift/out-loadlib/` は**読み取り専用で使い、編集していない**）。

限界:

- 実機の Windows ではなく、Linux 上の native ビルドと gdb（実測）、Learn のページ（引用）、逆アセンブルの読み。ゲストが実際に何を表示するか、フォントが読めるかは実機で確かめていない。
- 実測の「仮の値」は、その run の範囲で次の停止を見るためだけのもの。**値の正しさは何も主張しない**。
- 静的な到達（§3）は、vtable、レジスタ経由の間接 call を含まない。「静的に届かない」は「実行されない」ではない。
- 作業中の事故: 一時ファイルを誤って `/tmp/x` に 1 つ作った（6,000 バイトの出力。規則の `/tmp` 禁止に反する。削除は `rm` 禁止なので**していない**。内容は run のログの正規化で、結論に使っていない）。`lift/out-regstop/tools/__pycache__` は私の `import rdis` で生成された（削除していない）。

