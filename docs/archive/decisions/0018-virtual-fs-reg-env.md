# 0018: 仮想ファイルシステム・仮想レジストリ・仮想環境（ファイル、advapi32 レジストリ、環境変数、コマンドライン）

状態: PM が決定（方針）。仕様の読みと実装は作業者。API ごとの出典と「実装した／しなかった」は `docs/investigations/import-vfs-spec.md`。実行記録は `docs/investigations/vfs-run.md`。

## 決定（PM。変更しない）

- ファイル系は、メモリ内の仮想ファイルシステム。ホストの実ファイルシステムは見せない。初期状態は空で、FLEngine が作ったものだけが見える。存在しないパスは仕様どおり失敗（INVALID_HANDLE_VALUE、ERROR_FILE_NOT_FOUND=2、ERROR_PATH_NOT_FOUND=3）。
- GetModuleFileNameW は DLL 自身の仮想パスを返す。値は仮の判断で、**Windows の仕様ではない**。
- レジストリは、メモリ内の仮想レジストリ。初期状態は空。存在しないキーは ERROR_FILE_NOT_FOUND(2)。（例外: `HKLM\Software` だけは、値もサブキーも無い空のキーとして最初から存在する。0085、仮の決定。）
- 環境変数は、メモリ内の仮想環境ブロック。初期状態は空（ホストの環境変数は見せない）。存在しない変数は 0 と ERROR_ENVVAR_NOT_FOUND(203)。
- 仕様が定める範囲を超える入力は abort で止める（黙って成功にしない）。

## 作業者の判断（PM の決定の範囲内の細部）

### 1. 実装ファイル

`lift/runtime/fle_env.h/.c`（環境、コマンドライン、GetStartupInfoW、および 3 ファイル共通の Win32 エラー番号、`fle_stop`、UTF-16 の補助関数、スタック引数の読み出し `fle_arg`）、`fle_vfs.h/.c`（ファイルとハンドル）、`fle_reg.h/.c`（レジストリ）。既存の `fle_imports.c`、`fle_modules.c` などは変更していない（`fle_set_last_error` を呼ぶだけ）。`imports_table.rs` には 42 行を末尾に追加しただけ（他のサブエージェントの行は触っていない）。

### 2. 仮想パスの値（すべて仕様ではない。PM の GetModuleFileNameW の値を除き、作業者の仮の判断）

| 用途 | 値 | 根拠 |
|---|---|---|
| GetModuleFileNameW（NULL、0x400000） | `C:\Program Files\Image-Line\FL Studio 21\FLEngine_x64.dll`（57 文字） | PM の指示の例どおり |
| GetCommandLineW | `"C:\Program Files\Image-Line\FL Studio 21\FLEngine_x64.dll"`（引用符つき、引数なし） | 実行ファイルがないので DLL のパス。ページの「OS が完全修飾パスを付けることがある」に合わせた |
| GetWindowsDirectoryW | `C:\Windows` | 作業者の仮の値（ページの例は `C:\Windows`） |
| GetSystemDirectoryW | `C:\Windows\System32` | 同上（ページの例） |
| 現在のディレクトリ（初期） | `C:\` | 作業者の判断。ファイル系の唯一の存在物がボリュームルートなので |
| GetTempPathW | 仮想環境の TMP、TEMP、USERPROFILE の順、なければ `C:\Windows`、末尾に `\` | ページの探索順どおり。環境が空なので初期は `C:\Windows\` |

これらのディレクトリは仮想ボリュームには作らない（`GetFileAttributesW("C:\Windows")` は 2 で失敗する）。ホスト側に対応する実体はない。

### 3. ハンドルの領域（既存の定義を確認して決めた）

既存の値（実測: `grep 0x7FF` と `imports_table.rs`、`fle_modules.h`、`fle_sys.h`）:

- プロセスヒープ: `0x7FF700000000`（`fle_sys.h`）
- モジュールのハンドル: `0x7FF800000000 + 0x10000 × N`（N ≤ 25、0015）
- GetProcAddress の擬似 VA: `0x7FF900000000 + 0x10 × (k + 1)`（0015、応答表の行数分。現在の行数でも 0x7FF9000010xx 程度までで、`0x7FFA00000000` に届かない。テスト `handle_regions_do_not_overlap_anything_else` は 1000 行分まで見込んで確認）
- GetModuleHandleW(NULL): `0x400000`（イメージ内）

追加した領域（各 `0x10000000` バイト分、`FLE_HANDLE_REGION_SIZE`）:

| 種類 | 基点 | 払い出し |
|---|---|---|
| ファイルハンドル | `0x7FFA00000000`（`FLE_VFS_FILE_BASE`） | `BASE + 4 × (通し番号 + 1)`、再利用しない |
| 検索ハンドル（FindFirstFileW） | `0x7FFA10000000`（`FLE_VFS_FIND_BASE`） | 同上 |
| レジストリキーのハンドル | `0x7FFA20000000`（`FLE_REG_KEY_BASE`） | 同上 |

- 4 の倍数にしたのは Windows のハンドルに合わせた判断（仕様の記述は確認していない）。再利用しないのは、閉じたハンドルの誤用を無効として検出するため。
- レジストリの定義済みキー（HKEY_CURRENT_USER など）の値は winnt.h の値（`0xFFFFFFFF80000001` など）で、これは**ページから確認できていない**（`predefined-keys` のページは名前だけで数値がない）。FLEngine 自身は `mov ecx, 0x80000001`（ゼロ拡張）で渡す（`0x419a12`）ので、ゼロ拡張と符号拡張の両方を受ける。これは advapi32 の実際の挙動の推測。
- ファイルハンドルと検索ハンドルを別領域にしたのは、`CloseHandle` に検索ハンドル（ページが誤用と書く）が渡されたら失敗にするため。
- CloseHandle が受けるのはファイルハンドルのみ。他の領域の値（モジュール、ヒープ、擬似 VA を含む）、どの領域にもない値は abort（未実装の種類のハンドルを黙って閉じない）。NULL と INVALID_HANDLE_VALUE は失敗（0、ERROR 6）。

### 4. 仮想ファイルシステムの形

- 1 つのボリューム `C:`。ルートは最初から存在し、中身は空。他のドライブ、UNC、`\\?\`、`\\.\`、ドライブ相対（`C:name`）は abort。
- 名前は大文字小文字を区別せず（ASCII のみ折りたたむ。ページ: Naming a File）、`/` と `\` の両方を区切りとする（ページ: CreateFileW）。`.` と `..` を解決する。
- ページの命名規則に反する名前（予約文字、制御文字、末尾の空白とピリオド、デバイス名、ストリーム `:`、255 文字を超える成分、259 文字を超えるパス）は、ページがエラー番号を定めないので abort。
- ファイルの最大は 256 MiB（この runtime の制限。超えたら abort）。時刻は仮想クロック（2021-01-01 から、変更ごとに 1 秒進む。ホストの時計は読まない）。
- 共有モード、DeleteFile の「閉じるまで削除待ち」、FILE_FLAG_DELETE_ON_CLOSE は、ページの記述どおり実装した。

### 5. 仕様で決まらず、推測で決めたエラー番号（コードのコメントに「inference」と書いた）

ページが番号を書かない失敗: 無効ハンドルは ERROR_INVALID_HANDLE(6)、書き込み権なし／読み取り権なしは 5、RemoveDirectory の「空でない」は 145、ファイルに RemoveDirectory は 267、CreateFile の CREATE_NEW 以外の READONLY 衝突は 5、CopyFile の bFailIfExists は 80、MoveFile の宛先存在は 183、SetFilePointer の 32 ビット溢れは 87、FindFirstFile の末尾バックスラッシュは 123、レジストリの無効ハンドルは 6、アクセス権不足は 5。これらは実機で確認していない。

### 6. 仮想レジストリの形

- 定義済みの 5 つのルート（CLASSES_ROOT、CURRENT_USER、LOCAL_MACHINE、USERS、CURRENT_CONFIG）が最初から存在し、中身は空。キー名と値名は大文字小文字を区別しない（ページ）。（例外: LOCAL_MACHINE の中身は、空のキー `Software` 1 つだけ。0085、仮の決定。他の 4 つのルートは空のまま。）
- 値の型は NONE、SZ、EXPAND_SZ、BINARY、DWORD、MULTI_SZ、QWORD。型番号はページになく（winnt.h の値。`fle_reg.h`）、`REG_DWORD_BIG_ENDIAN`、`REG_LINK` は abort。
- 1 回の RegCreateKeyExW で作れるのは 32 階層まで（ページ）、キー名成分は 255 文字、値名は 16,383 文字、値は 1 MiB まで（ページ: Registry Element Size Limits）。超えたら abort。
- 列挙は作成順（ページは順序を定めない）。セキュリティ記述子、シンボリックリンク、WOW64 ビュー、トランザクション、HKEY_PERFORMANCE_* は未実装で abort。

### 7. 仮想環境の形

- 環境ブロックは空で始まる。名前は大文字小文字を区別しない比較（Windows の既知の挙動で、ページ Environment Variables には書かれていないと確認した。未確認の推測として扱う）。名前が空、NULL、`=` を含む、は abort。値は 32,767 文字まで（ページ）。
- ExpandEnvironmentStringsW は、`%name%` を展開し、見つからなければそのまま残す（ページ）。閉じない `%`、空の `%%`、lpSrc と lpDst が同じ、は abort。見つからない名前の後は閉じ `%` の次から走査を続ける（ページに記述なし。推測）。
- GetStartupInfoW は、`cb`（0x68）以外の全メンバーを 0 にする。作成元プロセスがないので、フラグ、位置、標準ハンドルはない。入力 cb の検査は 0083 で撤回。

### 8. 実装しなかったもの（理由は import-vfs-spec.md）

CreateFileMappingW、MapViewOfFile、UnmapViewOfFile（メモリマップは別の設計が要る）、SetDllDirectoryW（担当の例にない。閉包の 1 件）。ディレクトリのハンドル（FILE_FLAG_BACKUP_SEMANTICS で開く）、非同期 I/O、ストリーム、デバイス、UNC、他のドライブ、セキュリティ属性。

## 限界

- Windows 本体との照合はしていない（ホストが Linux）。テストの期待値はページの記述から Rust 側で独立に書いたが、ページが書かない箇所（上の 5）は推測。
- ゲストが実機のパス（`C:\Users\...`、`C:\ProgramData` など）を前提にしても、それらは存在しない。FLEngine がそれらを作る動作をするかどうかは、実行で初めて分かる。
- 閉包の中で、これらの API が呼ばれる引数（パス、キー名）は静的には調べていない。実行して出た分だけが分かる。
