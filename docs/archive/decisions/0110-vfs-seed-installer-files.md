# 0110: FL Studio のインストーラに同梱される製品データ（フォント 2 ファイル）を、ハーネスが仮想ボリュームの初期状態に書き込む — 決定 0018「VFS は空で始まる」を、明示した一覧のファイルだけ緩める（仮の決定。第 1 段階の設計・実装案に、第 2 段階の統合・コンパイル・自己テスト・6 モジュール構成の a4 測定を加えた版）

状態: **仮の決定**（PM の指示（2026-10-27。番号 0110 は PM の仮の割り当て）を、作業者（サブエージェント）が設計・実装案・文書にした）。**第 2 段階（統合・検証・測定。別の作業者、2026-10-27／28）を実施した版**: HEAD `0c5000f`（ブランチ main）に F2（この決定）と G1（決定 0111）を統合し、RAM 上でコンパイルし、全体テストを失敗 0 にし、ハーネスの自己テストを**実行**し（失敗 0）、6 モジュール構成の a4 を**配置なし（M0）／配置あり（M1）で測定**した（下の「第 2 段階の結果」）。第 1 段階の本文（設計、読み、副作用の読み、解釈が分かれた点）は、数値と状態の記述以外は書き換えていない。差分は統合後の `resolved-F2G1.diff`（`patch -p1`。F2 と G1 の 4 差分と統合の修正を 1 本にしたもの）に含まれる。**実機の Windows とは照合していない。**

> **これは A4 の達成ではない。** この決定は、ILTools の `CreateCursorFromFont` が QFC に測らせた文字の矩形が空になる原因（フォントのファイルが仮想ボリュームに無い。`lift/out-git/f1-results/createbitmap-zero-cause.md`）に対する、環境の内容の追加を設計する。決定 0018 の原則（初期状態は空）を、範囲を明示して緩める。配置の後に ILTools がどこまで進むかは第 2 段階で実測した（下の「第 2 段階の結果」: 配置は効き、freetype の `FT_Stream_ReadFields` の switch を通り、次の壁は kernel32 の `HeapReAlloc`）。決定 0101 の判定構成が環境の内容の追加をどう数えるかは未決（下の「未決の点」）。

事実の出典: `lift/out-git/f1-results/createbitmap-zero-cause.md`（原因調査。§0、§2、§4 の E2、§5、§7、§8）、決定 0018（VFS の形、初期状態）、0085（`HKLM\Software` だけは初期状態で存在する例外）、0098、0103（`SHGetFolderPathW(CSIDL_FONTS)` = `C:\Windows\Fonts`）、0104（`GetModuleFileNameW` の値、`FLE_VIRTUAL_SHARED_DIR`）、0105（HOOKS 欄。a4-multi.sh の欄の手本）、0106、0108、`docs/investigations/bundled-dll-inventory.md`、`tools/extract_nsis2.py`。ILTools と FLEngine のフォント探索の読みは `iltools-font-search.md`（同じ作業の成果物。要点を下に再掲）。

凡例: **実測**＝この作業で実行して値を読んだ／**読み**＝逆アセンブル（capstone、pefile）とソースを読んだ推論／**未確認**。

---

## 決定（作業者。仮）

| # | 項目 | 決定 |
|--:|---|---|
| 1 | 方向 | 決定 0018 の「ファイル系の初期状態は空」を、**インストーラ由来のファイルを、一覧に明示したものだけ**、ハーネスが実行前に書き込めるようにして緩める。**Windows 自体のシステムフォント（Segoe UI など）は含めない**（インストーラに無く、再配布できない）。**ホストの実ファイル（`/usr/share/fonts` など）をゲストに見せない**。ゲストが見るのは一覧のファイルとその親ディレクトリだけ。**未設定なら従来と完全に同じ（空のボリューム）** |
| 2 | 方式（f1 の M3） | ハーネスが、runtime のゲスト向け API の C 実装（`fle_imp_GetFileAttributesW`、`CreateDirectoryW`、`CreateFileW`、`WriteFile`、`SetFilePointer`、`ReadFile`、`CloseHandle`）を、レジスタ（`R[1]`、`R[2]`、`R[8]`、`R[9]`）と偽のスタック（引数 5 以降を `[rsp+0x28+8*(n-5)]`、`[rsp]` に戻り番地）に積んで、**エントリ（DllMain 相当）の前に**呼ぶ。`tests/fsx_common` が使う呼び方と同じ。**runtime に新しいコードは足さない**（足りない箇所は無い。ただし下の「未決の点」の 4 を見よ） |
| 3 | ハーネス側の実装 | **新しい `.c`／`.h` は runtime に作らない**。`lift/harness/vfs_seed.h`（ヘッダのみ、`static`。SHA-256、一覧の解析、書き込み。2 つの `main_two.c` が共有する）を新設し、`lift/harness/two/main_two.c` と `lift/harness/census/main_two.c` が `vfs_seed_from_env()` を 1 回呼ぶ（4 行ずつ）。`lift/harness/a4-multi.sh` に `VFS_SEED` 欄を足す |
| 4 | 環境変数 | `FLE_VFS_SEED=<一覧のファイル>`（未設定または空なら何もしない）。`FLE_VFS_SEED_ROOT=<絶対ディレクトリ>`（一覧のホストのファイルが相対パスのときの起点。相対パスがあるのに未設定なら名前付きで停止） |
| 5 | 一覧の書式 | 1 行 1 件、TAB 区切り 3 欄: `ゲストパス<TAB>ホストのファイル<TAB>SHA-256`。空行と `#` で始まる行は無視。ゲストパスは `C:\…` のバックスラッシュ区切り、ASCII、`.`／`..`／空の成分なし、259 文字以内。ホストのファイルは絶対パスか `FLE_VFS_SEED_ROOT` からの相対（`..` を含む相対は拒否）。SHA-256 は 64 桁の 16 進（大文字小文字不問）。最大 256 件、1 ファイル 64 MiB まで。**リポジトリにホストの絶対パスを固定しない** |
| 6 | 完全性 | **全件を先に読んで SHA-256 を検査し、1 件でも不一致（読めない、64 MiB 超、ゲストパスの重複、書式不正を含む）なら名前付きで停止し、仮想ボリュームには何も書かない**（`vfs-seed: <一覧>:<行>: SHA-256 mismatch for <ゲストパス> (host file <パス>): expected …, got …; nothing was written`）。書いた後は、同じハンドルで先頭から読み戻し、ホストのバイト列と比べる。停止は標準エラーへの 1 行と終了コード 2（ゲストを走らせる前） |
| 7 | 既定の一覧 | `lift/harness/seeds/vfs-seed-fonts.tsv`。**`ILCursors.ilfont`（7,892 B）と、マーカー `ILGlyphsEx.ilfont`（102,712 B）の 2 ファイル**を `C:\Program Files\Image-Line\FL Studio 21\Shared\Artwork\Fonts\` に置く（下の表）。ホストのファイルは `lift/out-inventory/extracted/fonts/`（git の管理外。`lift/out*/` は無視される）に、`tools/extract_nsis2.py` で書き出す |
| 8 | a4-multi.sh | 環境変数 `VFS_SEED=<一覧の絶対パス>`、`VFS_SEED_ROOT=<絶対ディレクトリ>`（既定は `$LIFT` の親＝リポジトリのルート）。呼び出し側の `FLE_VFS_SEED`／`FLE_VFS_SEED_ROOT` は**取り除く**（意図せず配置されないため）。未設定ならスクリプトの出力は従来と同じ。設定すると `a4-multi: VFS_SEED=…` の行、`$RUN/vfs-seed.tsv` への一覧の複製、報告の `---- vfs seed` の節（ハーネスの `vfs seed:` 行）を足す。実在モジュールが 0 個の構成（`harness/wasm/build_native.sh`。この 2 つの `main_two.c` を使わない）では停止する。**stamp と TAG には入れない**（実行時の入力で、ビルドは配置あり／なしで共有する） |
| 9 | 状態 | 仮の決定。撤回条件は下 |

### 配置するファイル（実測）

| ゲストパス | サイズ | SHA-256 | 取得 |
|---|--:|---|---|
| `C:\Program Files\Image-Line\FL Studio 21\Shared\Artwork\Fonts\ILCursors.ilfont` | 7,892 B | `588c4bf43070ebe92e97ef7a317677575ae344f71ef75020f0b13a308adb880b` | インストーラのエントリ 56532 |
| `C:\Program Files\Image-Line\FL Studio 21\Shared\Artwork\Fonts\ILGlyphsEx.ilfont` | 102,712 B | `578b46df16da9e98e9fe837b5786ec037b84a91478a46990b0d251d86f58aeb1` | 同 56534 |

- インストーラ: `/home/ubuntu/ダウンロード/flstudio_win64_26.1.7.5653.exe`（1,106,033,456 B、SHA-256 `04393e79…fc02`。記録 `lift/out-inventory/installer-sha256.txt` の値。**この作業では再計算していない**。`stat` のサイズは記録と一致）。
- 書き出し（実測）: `python3 tools/extract_nsis2.py INSTALLER --write ILCursors.ilfont lift/out-inventory/extracted/fonts/ILCursors.ilfont`、同 `--write ILGlyphsEx.ilfont …`（終了コード 0、書き出し先は `xb` で新規）。`--hash` の出力と `sha256sum` の値が一致。インストーラは読み取りのみ（他の場所に展開していない）。両ファイルとも sfnt `0x00010000`（TrueType、`glyf`）。`ILGlyphsEx.ilfont` は `cvt `／`fpgm`／`prep`（ヒンティングのバイトコード）を持つ。
- 書き出した 2 ファイルの合計は 110,604 B（`/` の書き込みの許容内）。

### ゲストパスの根拠（読み。詳細は `iltools-font-search.md`）

| 項目 | 内容 |
|---|---|
| ILTools が見る親ディレクトリ | `GetModuleFileNameW(HInstance)`（決定 0104: `C:\Program Files\Image-Line\FL Studio 21\Shared\ILTools_x64.dll`）の `ExtractFilePath` ＋ `Artwork\Fonts\ILGlyphsEx.ilfont`（マーカー）が**存在するときだけ**、そのディレクトリ（`…\Shared\Artwork\Fonts\`）が探索表の [1] になる。[0] は `C:\Windows\Fonts\`。`ILCursors.ilfont` は表の**後ろから前へ**探される（`f_65ef30`） |
| 第 2 候補（`f_5a5250`。f1 は未読） | 第 1 候補が見つからなかったときだけ。`QuickFontCache_x64.dll` を実行ファイルのディレクトリから親へ登って探す処理の結果で、仮想ボリュームにその DLL が無いので失敗し、**相対パス `Shared\` に落ちる**＝`C:\Shared\Artwork\Fonts\ILGlyphsEx.ilfont`（現在のディレクトリ `C:\`）。実在するディレクトリを指さない。**配置しない** |
| 作るディレクトリ | `C:\Program Files`、`…\Image-Line`、`…\FL Studio 21`、`…\Shared`、`…\Artwork`、`…\Fonts`（6 つ） |
| 落とし穴（読み） | 第 1 候補の探索は、`GetModuleFileNameW` の直後に **`GetLastError() == 0`** を要求する（ILTools `f_5c9c70` の 0x5c9cbf。FLEngine `f_668b50` も同じ）。この runtime の `GetModuleFileNameW` は成功時に最終エラーを 0 にしない。その時点の最終エラーが 0 でなければ、**配置しても第 1 候補が黙って飛ばされる**。第 2 段階で確認する（`MERGE-NOTES.md`）。Wine の `GetModuleFileNameW` は成功時に `SetLastError(0)` 相当（ソースを読んだ。実機ではない） |


---

## 第 2 段階の結果（実測。凡例: 実測＝この作業でコマンドの出力を見た事実／読み＝そこからの解釈／未確認）

基準: HEAD `0c5000f`。F2（`f2-vfs-seed.diff`）と G1（決定 0111 の 4 差分）を `patch -p1` で順に当てた木（`/tmp/opencode/s3/src/`）。全体は `resolved-F2G1.diff`（16 ファイル。行数は MERGE-NOTES の「成果物」）。

### 1. 統合・コンパイル・自己テスト（実測）

| 項目 | 結果 |
|---|---|
| `patch -p1 --dry-run` と適用 | F2 の 6 ファイルは**ずれ・拒否・fuzz なし**。G1 の 4 差分とも重ならない（`a4-multi.sh` は F2 が 4 か所、G1 の統合が既定の `EXPECT_FLE_SHA` の 1 行。行が別なので衝突なし）。`resolved-F2G1.diff` を別の clean な `git archive 0c5000f` に `patch -p1` で当て、適用後の木がテストした木と `diff -r` で一致 |
| `bash -n lift/harness/a4-multi.sh` | OK |
| `clang -fsyntax-only -Wall -Wextra -Werror -std=gnu11 -O1`: `two/main_two.c`、`census/main_two.c`、`seeds/vfs_seed_selftest.c`（統合した木） | 警告 0・エラー 0 |
| **自己テスト** `vfs_seed_selftest.c` のビルド（`clang -std=gnu11 -O0 -g -Wall -Wextra -fPIE`、runtime の 6 ファイルとリンク） | 警告 0・エラー 0・**リンク不足なし** |
| **自己テストの実行** | **終了コード 0、最後の行 `0 failure(s)`、`ok` の行 41、`FAIL` の行 0**。標準エラーの 12 行は拒否ケースの `vfs-seed: …`（正常）。SHA-256 の C 実装（標準ベクトル 5 件ほか）は C を実行して一致した（第 1 段階は Python の転記だけだった） |
| 直した誤り | F2 のコード（`vfs_seed.h`、`main_two.c` ×2、`a4-multi.sh`、一覧、自己テスト）は**1 文字も直していない**（構文・型のエラー 0） |
| `cargo build --offline`（dev） | 警告 0・エラー 0（F2 は Rust に触れない） |

### 2. 6 モジュール構成の a4（native。註釈あり。実測）

構成: dsp_ippv2（0x2000000）、Blend2D（0x4700000）、QFC（0x1d00000）、freetype（0x4b00000）、ILTools（0x6000000）。測定用の基底と種・註釈（`lift/harness/seeds`、`hooks`）。決定 0106 の S6 と同じ引数。素材の SHA-256 は W6 統合時の記録と先頭が一致（dsp 497dd4f7…、Blend2D 66634d24…、QFC afc5a0ed…、freetype 6ea6c106…）。`a4-multi.sh`（`a4-multi-trim.sh`: 差は `HERE` の固定と、gen／build の後に生成 C と分割ソースを消す行、`REUSE=1`＝ビルド済みの native を使い gen／build を飛ばす行だけ）。**M0 と M1 は同じビルド（同じ OUT の同じ native）を使い、実行だけ変えた**（`VFS_SEED` は stamp にも TAG にも入れていない設計どおり）。M0 は `FLE_VFS_SEED`／`FLE_VFS_SEED_ROOT` を環境から除いて実行。M1 は `VFS_SEED=<木の lift/harness/seeds/vfs-seed-fonts.tsv>`、`VFS_SEED_ROOT=/home/ubuntu/work/signature-studio-develop`（リポジトリのルート。**一覧のホストのパスが `lift/out-inventory/extracted/fonts/…` の相対なので、起点はルートである**。依頼文の「フォントのあるディレクトリ」は「フォントを含む木のルート」と読んだ。フォントだけのディレクトリを起点にするなら一覧の相対パスを `ILCursors.ilfont` に直す必要がある）。

| | M0（配置なし） | M1（配置あり） | 基準（W6 統合時） |
|---|---|---|---|
| init_count | **246 / 942** | **246 / 942** | 246 / 942 |
| 停止 | `fle: StretchDIBits: this pixel transfer or read-back is not one of FillRect, PatBlt and BitBlt(SRCCOPY) between DIB sections: the conversions and the bytes GDI writes are not on the pages, and a device-dependent bitmap keeps no pixels (decision 0111)` | `fle: HeapReAlloc: the requested size of a process-heap block is not recorded by HeapAlloc (not answered)` | `fle: unsupported instruction at 0x601da30`（`gdi32!MoveToEx`） |
| 分類 | **名前付き停止**（決定 0111 の区分 C、gdi32!StretchDIBits。ILTools の thunk は 0x601dc90） | **名前付き停止**（kernel32!HeapReAlloc、決定 0078。freetype の `ft_mem_qrealloc` の中） | 未応答 import |
| functions／entries | 3,411 ／ 134,085 | 3,386 ／ 134,274 | 3,377 ／ 133,965 |
| call depth | 32 | 45 | 36 |
| 終了コード | 134（SIGABRT） | 134（SIGABRT） | 134 |
| 再現性 | 同じ native の再実行 2 回（計 3 回）で同じ。**別ディレクトリで一からビルドして再測しても同一**（native の SHA-256 先頭 `08f6bc15…` も同じ） | 同じ | — |

- M0 の C スタック（gdb `bt`）: `fle_imp_StretchDIBits ← f_601dc90 ← f_61eb000 ← f_6130190 ← f_625ee30 ← f_625ef30 ← f_6262b60（CreateCursorFromFont）← f_625ed40 ← … ← f_62b35f0（ILTools DllMain）← fle_realmod_load ← fle_imp_LoadLibraryW ← f_423990 …`。
- M1 の C スタック: `fle_imp_HeapReAlloc ← f_4b570b0 ← f_4b24830 ← f_4b169f0（ft_mem_qrealloc）← f_4b18930（ft_mem_realloc）← f_4b19040（FT_CMap_New）← f_4b3c570 ← f_4b3d430 ← f_4b4a980 ← f_4b173c0 ← f_4b1a070（FT_Open_Face）← f_4b1a4d0（FT_New_Memory_Face）← f_1e74f10 …（QFC）… ← f_61c9710 ← f_625ef30 ← f_6262b60（CreateCursorFromFont）← … ← f_62b35f0（ILTools DllMain）`。
- **`FT_Stream_ReadFields`（`f_4b166e0`。決定 0109 が受理した switch 0x4b1674d を含む）は、M1 で 37 回呼ばれ、戻った**（gdb の関数の入口の計数。M0 では 0 回）。`FT_Open_Face` は 1 回。switch の各 case のどれが踏まれたかは**未計測**（読み）。0109 が受理した switch を**ゲストが初めて実行した**。
- 配置は効いた。`vfs seed:` の行（ハーネス）: `done, 2 files, 6 directories created`。ILCursors.ilfont は ILTools の探索表の [1] で見つかり、QFC の `CreateFileW`（小文字化されたパス）で開かれた。
- 予測との差（第 1 段階の読み）: 「`fle: indirect call to 0x4b1674f is not a known function entry`」の停止は**起きなかった**（0109 で解決済みのため）。代わりに、その先の `FT_CMap_New` の realloc で kernel32 の名前付き停止に当たった。「MoveToEx の停止より手前」は当たった（M1 は `CreateCursorFromFont` の前半、M0 は後半）。「init_count は 246 / 942 のまま」も当たった。
- **配置ありの停止（HeapReAlloc）は freetype／kernel32 側の壁**で、配置が作ったものではない（配置なしでは freetype に入らないので見えなかった）。この壁を越えた先は未測定（新しい停止は直さない）。

### 3. 最終エラーの門（F2 の懸念。実測）

- ILTools の探索（caller 0x61c9cbf。基底 0x6000000 の測定用のアドレス。F2 の読みの `f_5c9c70` の 0x5c9cbf と同じコードと読む）の `GetModuleFileNameW(hModule=0x6000000)` の直後の `GetLastError` は、**M0 でも M1 でも 0** で、直後に `…\Shared\Artwork\Fonts\ILGlyphsEx.ilfont` への `GetFileAttributesW` が出た（M0: 0xffffffff、M1: 0x80）。**門は飛ばされていない**。したがって `fle_set_last_error(0)` を足した変種は**作っていない**（依頼の条件「飛ばされているなら」に当たらない）。
- 最終エラーが 0 になる理由（実測）: `fle_last_error` へのウォッチポイントで、門の直前の `GetLastError → 2`（`QuickFontCache_x64.dll` の探索の失敗）のあと、**`fle_imp_TlsGetValue`（ILTools の `f_6017d40`。Delphi の threadvar へのアクセスと読む）が最終エラーを 0 に書いていた**（`TlsGetValue` は成功時に最終エラーを ERROR_SUCCESS にする、という Learn の記述どおりの実装）。`GetModuleFileNameW` 自身は成功時に最終エラーを触らない（`fle_vfs.c` のとおり）。つまり門は**`TlsGetValue` が間に入る限り通る**（読み。別の経路では通らない可能性がある: 同じ `GetModuleFileNameW(0x6000000)` でも caller 0x61a4732 の呼び出しの後の `GetLastError` は 0x7f だった）。
- よって「未決の点」の 4（`GetModuleFileNameW` の runtime 変更）は、**この構成では必要なかった**（配置が効いた）。

### 4. 配置あり／なしのファイル系の呼び出しの差（gdb。`logs/gdb/m0-fs-trace.txt`、`m1-fs-trace.txt`、`fs-diff-m0-m1.txt`）

呼び出し数（`GetFileAttributesW`／`CreateDirectoryW`／`CreateFileW`／`FindFirstFileW`／`GetModuleFileNameW`／`GetLastError`）: M0 113／0／0／12／36／263、M1 133／6／3／12／36／267。

1. **M1 の最初の 20 行はハーネスの配置**（`GetFileAttributesW` 12、`CreateDirectoryW` 6、`CreateFileW` 2。エントリの前。F2 の設計どおり）。`CreateFileW` の戻りは `0x7ffa00000004`、`…08`。ゲストの最初のファイルハンドルは `0x7ffa0000000c`（F2 の副作用 5 の予測どおり）。
2. **FLEngine 側の呼び出しの順序と結果は変わらなかった**: FLEngine のフォント探索（`FileExists` `f_43ba70`、caller 0x43ba94）の 26 回の `GetFileAttributesW`（`…\FL Studio 21\Artwork\Fonts\ILGlyphsEx.ilfont`、`Shared\Artwork\Fonts\ILGlyphsEx.ilfont`、`C:\Windows\Fonts\…`）は M0 と M1 で**同じ順序・同じ結果（すべて 0xffffffff）**。F2 の「FLEngine のフォント探索は配置では直らない」の予測どおり。
3. **ゲストから見えた差**: (a) **`GetLastError` が 3（PATH_NOT_FOUND）→ 2（FILE_NOT_FOUND）に変わった箇所が 12**（`C:\Program Files\Image-Line\FL Studio 21\` と `…\Shared\` の直下の存在しない DLL（`dsp_ippv2_x64.dll`、`Blend2D.dll`、`QuickFontCache_x64.dll`、`ILTools_x64.dll`）の探索の失敗 6 と、その直後に `GetModuleFileNameW(NULL)` を挟んで同じ値を読む `GetLastError` 6。`GetFileAttributesW` の戻り値自体は 0xffffffff のまま。F2 の副作用 2 の予測どおり）。(b) ILTools の `FileExists`（caller 0x6032124）の探索が**増えた**: マーカー `…\Shared\Artwork\Fonts\ILGlyphsEx.ilfont` が見つかる（0x80）ので、UI フォントの探索で `…\Shared\Artwork\Fonts\{OpenSans-Regular.ttf（5 回）、tahoma.ttf、OpenSans-CondLight.ttf、lucon.ttf、WebSymbols-Regular.otf、PT_Sans-Web-Regular.ttf}` が 10 回加わった（すべて 0xffffffff、`GetLastError` 2。一覧に無いので見つからない）。(c) M0 の `C:\Windows\Fonts\ILGlyphsEx.ilfont`、`ILCursors.ilfont`（小文字の形を含む）への失敗した探索が、M1 では `…\Shared\Artwork\Fonts\` の同名ファイルの探索（0x80）に置き換わり、続いて `CreateFileW c:\program files\image-line\fl studio 21\shared\artwork\fonts\ilcursors.ilfont`（QFC。caller 0x1d30fce）が出た。
4. 上以外の呼び出し元（F2 が「追っていない」とした約 40 か所）が M1 で新しい呼び出しをした形跡は、**この停止までの範囲では無い**（`FindFirstFileW` の 12 回は M0 と同じパス）。停止より先は未測定。
5. 仮想クロック（10 秒進む）、`FindFirstFileW("C:\*")` の差などの F2 の副作用 3、4 は**この測定では見ていない**（未確認）。

### 5. 第 1 段階の予測と実測の対照

| 第 1 段階の読み | 実測 |
|---|---|
| 第 1 候補の門で飛ばされる懸念 | 飛ばされなかった（上の 3） |
| `ILCursors.ilfont` が見つかり QFC に実在のパスが渡る | 当たり |
| `ILGlyphsEx.ilfont` が QFC に渡され、広い穴に早く当たる可能性（解釈が分かれた点 2 の 2） | **渡された形跡は無い**（`CreateFileW` は `ilcursors.ilfont` の 1 回だけ。`ILGlyphsEx.ilfont` は `GetFileAttributesW` で存在を確かめるだけ）。この停止までの範囲 |
| freetype の停止は `FT_Stream_ReadFields` の switch | 0109 で解決済み。switch は通過（37 回）。実際の停止は後段の `HeapReAlloc` |
| init_count 246 / 942 のまま | 当たり |

---

## 背景（事実と読み）

1. **原因**（f1 の結論。強い証拠）: ILTools の DllMain(ATTACH) の中の `CreateCursorFromFont('C')` が QFC に文字 "C" の矩形を測らせ、矩形が (0, 0, 0, 0) になり、`CreateBitmap(0, 0)` が呼ばれる。QFC がフォントの実体を作れなかった直接の原因は、`ILCursors.ilfont` が仮想ボリュームに無いこと（ボリュームは空で始まる。決定 0018）。実測（旧ビルドの 5 モジュール、ILTools 抜き、gdb）で、ファイルが無ければ矩形はすべて 0、有れば経路が `CreateFileW`→`ReadFile`（7,892 B）→`FT_New_Memory_Face` に変わる。
2. **そのファイルの出自**: `ILCursors.ilfont` は Image-Line が `Shared\Artwork\Fonts\` に同梱する**製品データ**で、Windows のシステムフォントではない。インストーラ（NSIS）に 36 ファイル（合計 5,163,436 B）が入る。Segoe UI／Arial／Tahoma などは**入っていない**（`nsis-list-all.txt` の名前検索で 0 件）。
3. **この決定を取る理由**（f1 の 7.1 の A、7.2 の M3）: 製品の実ファイルを（ホストの実ファイルシステムからでなく）インストーラから取り、ゲストが実機で見るはずの場所に置けば、ゲストの経路は実機に近づく（読み）。D（置かない）のままでは、0×0 → 1×1 のビットマップで W6 以降（GDI の塊）へ進み、そこで積む応答が実機の経路と一致する保証がない（f1 §6 の 3、決定 0106 の P6）。
4. **配置の後の壁**（f1 の 6 節の 1）: freetype の `FT_Stream_ReadFields`（`f_4b166e0`）の switch（0x4b1674d）が `msvc_rejected`（決定 0093 の上限の外の形）で、配置すると face の作成（`FT_New_Memory_Face`）がそこで止まる見込み（読み。旧ビルドの 5 モジュールで実測。HEAD の 6 モジュールでは未測定）。別の作業者が直している最中。

---

## 副作用（ゲストに見える変化）

### 仮想ボリューム（runtime の実装から。読み）

1. 存在するようになるもの: 6 ディレクトリと 2 ファイル（上）。`GetFileAttributesW` は、ディレクトリに 0x10、ファイルに 0x80（`FILE_ATTRIBUTE_NORMAL`。インストール直後の実ファイルの属性とは限らない）を返す。
2. **以前は ERROR_PATH_NOT_FOUND（3）で失敗したパスが、ERROR_FILE_NOT_FOUND（2）で失敗する**: 上の 6 ディレクトリの下の存在しないファイル（例: `…\FL Studio 21\QuickFontCache_x64.dll`、`…\Shared\QuickFontCache_x64.dll`、`…\FL Studio 21\<DLL 名>_x64.dll`）。
3. **以前は 3 で失敗した書き込み系が成功しうる**: `CreateFileW(CREATE_ALWAYS)` が `…\FL Studio 21\`、`…\Shared\` などの直下で成功する。`CreateDirectoryW` が既存のディレクトリに 183 を返す（以前は成功か 3）。`FindFirstFileW("C:\*")` が `Program Files` を返す（以前は 2）。
4. **仮想クロック**が進む（変更ごとに 1 秒）: 6 ディレクトリ ＋ 2 ファイルの作成で 8、2 回の `WriteFile` で 2、合計 **10 秒**（読み）。ゲストが後で作るファイルの時刻が 10 秒遅れる。
5. **ファイル ハンドルの番号**: 書き込みに 2 つ使う（値は再利用されない。1 ファイル 1 ハンドル。読み戻しは同じハンドル）。ゲストの最初のハンドルは `0x7FFA00000004` ではなく `0x7FFA0000000C`（決定 0018 の割り当て規則から。ゲストがハンドルの値そのものに依存すると分かったときは決定 0104 の P4 の撤回条件に近い）。
6. ハーネスは配置の前後で `R[]` を保存・復元し、最終エラーを 0 に戻す（`GetFileAttributesW` が失敗で 2／3 を残すため）。エントリに渡るレジスタの状態は配置の有無で同じ。

### ILTools（読み。詳細は `iltools-font-search.md`）

1. 探索表に [1] `C:\Program Files\Image-Line\FL Studio 21\Shared\Artwork\Fonts\` が入る（落とし穴の条件が満たされれば）。`ILCursors.ilfont` が見つかり、QFC に実在するパスが渡る → `FT_New_Memory_Face` に入る。
2. **`ILGlyphsEx.ilfont` の記述子が実在するパスになる**（`f_5c9e50` の 0x5cd04e 付近）。ILTools が QFC にこのフォントを渡す時期は**未確認**。渡すと、102,712 B・ヒンティング付きの TrueType の FreeType の経路（`cvt `／`fpgm`／`prep`）に入る。`ILCursors.ilfont`（ヒンティングなし）より**広い**範囲の freetype の穴に当たる可能性がある。
3. UI フォントの探索（`segoeui.ttf` が無い → `OpenSans-Regular`）は、表が 2 要素になるので `GetFileAttributesW` が 1 回増えるが、**`OpenSans-Regular.ttf` は一覧に無い**ので結果（見つからない）は同じ。UI のテキストの測定は、配置後も劣化したまま（読み。`OpenSans-Regular.ttf` は 217,360 B、SHA-256 `e64e508b2aa2880f907e470c4550980ec4c0694d103a43f36150ac3f93189bee`。`--hash` だけ。書き出していない）。

### FLEngine（読み。詳細は `iltools-font-search.md` の 5）

1. FLEngine の同じコード（`f_668b50`）は `C:\Program Files\Image-Line\FL Studio 21\Artwork\Fonts\ILGlyphsEx.ilfont`（FLEngine の HInstance のディレクトリ。**配置先 `…\Shared\` とは別**）と相対の `Shared\` を見る → **配置後も見つからない**。FLEngine のフォント表（`f_668d30`）、FLEngine 自身が持つカーソルの参照（`f_815050`）の結果は**変わらない**。FLEngine の UI フォントは配置では直らない。
2. FLEngine の DLL の存在確認（`f_653660` の 4 候補、`f_653490` の登り）は偽のまま。失敗の最終エラーが 3→2 になるだけで、`FileExists`（`f_43ba70`）は同じに扱う。
3. 上以外の `GetFileAttributesW`（12）、`CreateFileW`（15）、`CreateDirectoryW`（2）、`FindFirstFileW`（6）、`GetFileAttributesExW`（8）の使用者は**追っていない（未確認）**。第 2 段階で呼び出し順の差分を取る（`gdb_fs_trace.py`（`f2-results/tools/`））。

---

## 配置の後に ILTools がどこまで進むか（第 1 段階の読み。実測は上の「第 2 段階の結果」で、この節の予測のうち停止の位置だけが違った）

- 停止の位置: 最初の `CreateCursorFromFont('C', 0xc8, -1)`（`f_65ef30`）の中で、`f_5c9710`（0x65f64e）が `FontCache_AcquireFont` を呼ぶ → QFC の `Loader.SetFont` → `QFC.Load` → `FileExists` → `CreateFileW`／`ReadFile` → `FT_New_Memory_Face` → `FT_Open_Face` → … → `FT_Stream_ReadFields` の switch（0x4b1674d）で `fle: indirect call to 0x4b1674f is not a known function entry`（f1 の E2。**旧ビルドの 5 モジュールで実測**。HEAD の 6 モジュールでは**未測定**）。
- **「配置なし」の `MoveToEx` の停止（決定 0108。`gdi32!MoveToEx` の thunk 0x601da30）との前後**: `MoveToEx` は、同じ `CreateCursorFromFont('C')` の呼び出しの**後半**（測定 → `CreateBitmap(0,0→1×1)` → `CreateCompatibleDC` → `MoveToEx`）にある。freetype の停止は `f_5c9710`（0x65f64e）の**中**で、測定（0x65f6a1）より前。したがって**freetype の停止は `MoveToEx` の停止より手前（同じ呼び出しの前半）**（読み）。配置ありでは `CreateBitmap`、`CreateCompatibleDC`、`MoveToEx` には**届かない**。
- init_count: どちらも FLEngine の unit 245（`LoadLibraryW "iltools_x64.dll"`）の中なので、**246 / 942 のまま**の見込み（読み。A4 の進捗ではない）。functions／entries と停止メッセージが変わる。
- 例外（配置の効果が出ない、または手前で止まる場合。**未確認**）: (a) `GetLastError` の門で第 1 候補が飛ばされ、配置の効果が出ない（結果は配置なしと同じ: `MoveToEx`）。(b) `ILGlyphsEx.ilfont` が `ILCursors.ilfont` より早く QFC に渡され、さらに手前で止まる。(c) freetype が直った後は、`CreateBitmap` の引数が 0×0 から（f1 の推測で）32×32 に変わる見込みで、W6 以降（GDI）の経路が実機に近づく。測定矩形の値は未確認。

---

## 範囲

- この決定が扱う: 一覧のファイル（既定は 2 ファイル）を、ハーネスが実行前に仮想ボリュームに書く仕組み、その書式、検査、a4-multi.sh の欄、`iltools-font-search.md`。
- 扱う環境: native のハーネス（`two/main_two.c`、`census/main_two.c`）。wasm のハーネスは、`FLE_VFS_SEED` が設定されていたら名前付きで停止する（ホストのファイルを読む設定が無い。ビルドも実行もしていない）。

## 範囲外

- Windows のシステムフォント（Segoe UI、Arial、Tahoma など）。ホストの実ファイルの配置。`Shared\Artwork\Fonts` の 36 ファイル全部（案 B）や `OpenSans-Regular.ttf`（UI フォント。候補）。
- FLEngine のフォント探索を満たす配置（`…\FL Studio 21\Artwork\Fonts\`）。実在しない第 2 候補（`C:\Shared\…`）の配置。QFC の DLL などを仮想ボリュームのファイルにすること。
- freetype の lift の穴（`FT_Stream_ReadFields` の switch ほか）の修正。runtime の変更（`GetModuleFileNameW` の最終エラーを含む。下の「未決の点」の 4）。応答表の行。
- 配置後の実測（6 モジュール構成の a4、gdb、census）。実機との照合。`run_census.sh` の変更（`FLE_VFS_SEED` を `unset` しない。`MERGE-NOTES.md`）。単一モジュールのハーネス（`harness/main.c`、`harness/wasm/main.c`）への実装。

## 撤回条件（仮の決定）

1. PM が決定 0018 の緩和を取り消したとき（一覧を空にする、または `FLE_VFS_SEED` を使わない。コードは環境変数が無ければ何もしないので、撤回は運用で足りる）。
2. 実測で、配置が実機と違う経路を作る、または配置なしより遠くへ進めない（例: 第 1 候補が最終エラーの門で飛ばされる、`ILGlyphsEx.ilfont` の配置が FreeType のより広い穴に早く当たる）と分かり、PM が配置を採らないと決めたとき。
3. 決定 0104 の `FLE_VIRTUAL_SHARED_DIR`（`C:\Program Files\Image-Line\FL Studio 21\Shared\`）が変わったとき、または実機の `Shared` の位置が分かったとき（一覧のパスを更新）。
4. インストーラ由来の製品データをローカルの検証に使うこと（git に入れず、`lift/out-inventory/extracted/` に保管）に、権利上の問題が指摘されたとき。
5. 配置の範囲を 2 ファイルより広げるとき（この決定を更新するか、新しい決定にする）。
6. 決定 0101 の A4 の判定が、環境の内容の追加を「仮の応答」と同様に数えると決めたとき（記録の扱いを変える）。

## 実装した場所（`f2-vfs-seed.diff`。6 ファイル、+約 700 行）

| ファイル | 変更 |
|---|---|
| `lift/harness/vfs_seed.h`（新規。450 行） | SHA-256、一覧の解析、検査、書き込み、`vfs_seed_from_env()` |
| `lift/harness/two/main_two.c` | `#include "vfs_seed.h"` と、エントリの存在確認の後、スタックの確保の前の呼び出し 1 回（+4） |
| `lift/harness/census/main_two.c` | 同じ（+4） |
| `lift/harness/a4-multi.sh` | `VFS_SEED`／`VFS_SEED_ROOT` の欄（説明、検証、表示、複製、報告）（+33） |
| `lift/harness/seeds/vfs-seed-fonts.tsv`（新規） | 既定の一覧（2 件） |
| `lift/harness/seeds/vfs_seed_selftest.c`（新規。194 行） | `vfs_seed.h` の自己テスト（第 2 段階でビルドして実行する） |

`a4-multi.sh` の stamp は `harness/*.h` と `harness/two/main_two.c` を含むので、**この差分より前に作った OUT は拒否される**（新しい OUT を使う。`MERGE-NOTES.md`）。FLEngine の生成 C には影響しない（`EXPECT_FLE_SHA` は変わらない）。

## テスト

ハーネスには自動テストが無い（`main_two.c` は native の実行で確かめてきた）。

1. **自己テスト** `lift/harness/seeds/vfs_seed_selftest.c`（新規。第 1 段階では構文確認だけ。**第 2 段階で実行: 失敗 0、`ok` 41 行**）。runtime の `fle_vfs.c`、`fle_reg.c`、`fle_env.c`、`fle_imports.c`、`fle_modules.c`、`fle_string.c` だけをリンクし（生成 C なし）、数秒でビルド・実行できる。検査: SHA-256 の標準ベクトル 5 件（"abc"、空、56 バイト、100 万個の 'a'、パターン 100,000 B）、未設定／空で何も起きない（`R[]` 不変、ボリュームは空）、良い一覧（相対のホスト名、大文字の SHA-256、CRLF、末尾改行なし、空ファイル、100,000 B、ゲストパスの大文字小文字違い）で戻り値 0・`R[]` 復元・親ディレクトリ・読み戻し・最終エラー 0・ハンドル番号の消費（4 番目）・一覧外が増えない、同じ一覧の 2 回目が拒否される（CREATE_NEW）、拒否（SHA 不一致、16 進でない、ホストのファイルなし、`..`、重複、`.` 成分、別ドライブ、欄の数）が**ボリュームに何も書かない**こと、`FLE_VFS_SEED_ROOT` なしの相対パスの拒否、一覧のファイルなしの拒否。出力は `ok`／`FAIL` の行と失敗数（終了コード）。
2. **手動の検証手順**（第 2 段階。`MERGE-NOTES.md` に詳細）: (a) 差分を当てて構文確認、(b) 自己テストをビルドして実行（失敗 0）、(c) `sha256sum -c` で書き出した 2 ファイルの SHA-256 を確認、(d) 6 モジュール構成の a4 を配置なし／ありで 1 回ずつ（同じ OUT。配置なしは従来の出力と同じことを確認）、(e) `gdb_fs_trace.py`（`f2-results/tools/`） で、最終エラーの門と、ファイル系の呼び出し順の差分を取る。

## 検証（実施したこと。実施していないこと）

| 項目 | 結果 |
|---|---|
| `clang -fsyntax-only -Wall -Wextra -Werror -std=gnu11 -O1 -I lift/runtime -I lift/harness`: `two/main_two.c`、`census/main_two.c`（`-I lift/harness/census` を追加）、`seeds/vfs_seed_selftest.c`、`-D__EMSCRIPTEN__=1` でヘッダだけ | **警告 0・エラー 0**（実測。clang 21.1.8。ヘッダは `-Wshadow -Wconversion` も警告 0）。HEAD の元の `two/main_two.c` も同じ警告 0 |
| `bash -n lift/harness/a4-multi.sh` | OK（実測） |
| SHA-256 の C 実装の**アルゴリズム**: ヘッダのコード（`vs_block`、`vs_sha256_hex`）を 1 行ずつ Python に転記（定数はヘッダから正規表現で読み出し）し、`hashlib` と比較 | 標準ベクトル "abc"・空、長さ 1／54〜57／63〜65／119〜121／127〜129／1000／4096 のランダム 16 件、書き出した 2 ファイルが**すべて一致**（実測。**C の実行ではない**: 符号なし 32 ビットの算術は C の規則で書いたが、コンパイルして動かしてはいない） |
| ゲストパスの規則 `vs_guest_problem` の Python 転記 | 17 件の期待どおり、既定の一覧の 2 件は受理（実測。転記） |
| 差分: `patch -p1 --dry-run`（HEAD の木のコピー）、適用後の木とワークツリーが `diff -r` で一致、リポジトリでの `git apply --check -p1` | OK（実測。リポジトリは変更していない） |
| **第 2 段階で実施** | コンパイル、リンク、自己テストの実行（失敗 0）、6 モジュール構成の a4（配置なし／あり）、gdb（ファイル系の呼び出しの差分、最終エラーの門、`fle_last_error` のウォッチ）、全体テスト（1,526 passed・0 failed。F2 は Rust のテストに触れない）。`a4-multi.sh` の `VFS_SEED` の実際の動作（`---- vfs seed` の節、`$RUN/vfs-seed.tsv`、M0 に `VFS_SEED` の行が出ないこと）を確認（`a4-multi-trim.sh` 経由。差は上の 2 の注記のとおり）。runtime の挙動（`CreateFileW`／`WriteFile`／読み戻し）は配置の成功（`done, 2 files, 6 directories created`）で実行された |
| **未実施** | census（`run_census.sh` の `FLE_VFS_SEED` の扱いを含む）、wasm、実機 Windows との照合（0 件）、配置ありの停止（`HeapReAlloc`）より先 |

## 解釈が分かれた点

1. **「ILTools がどの親ディレクトリを基準にフォントを探すか」**: 依頼文の `list[0]+"ILCursors.ilfont"` の `list[0]` は `C:\Windows\Fonts\`（`SHGetFolderPathW(CSIDL_FONTS)`）で、ILTools は最初にそれを試す。ファイルが見つかるのは、続けて**表の後ろから前へ**探す（`f_65ef30`。f1 の §2 の 6 も同じ読み）ときの、マーカーが追加する **`list[1]`（`…\Shared\Artwork\Fonts\`）**である（読み）。依頼文の「`list[0]` から何が組み立てられるか」を、この読みで答えた（`list[0]` の組み立ては `GetModuleFileNameW` ではなく `SHGetFolderPathW`、`GetModuleFileNameW` の答えから組み立てられるのは `list[1]`）。
2. **第 2 候補は「実在するディレクトリ」ではなく相対パス `Shared\`**: 依頼文は「第 2 候補（`f_5a5250` が返す）を特定して、必要なディレクトリをすべて作る」。実際の第 2 候補は `C:\Shared\Artwork\Fonts\`（`C:\` からの相対）で、**第 1 候補が見つかれば評価されない**ので、作らなかった。作るべきだと読むなら、`C:\Shared\Artwork\Fonts\ILGlyphsEx.ilfont` を一覧に足せば効くが、実機にない場所への配置になる。
3. **ホストのファイルの供給**: 依頼文は「リポジトリ相対パスまたは環境変数展開」。`FLE_VFS_SEED_ROOT` 相対を採った（環境変数展開は `${…}` を実装していない）。`lift/out*/` が git の管理外なので、一覧には `lift/out-inventory/extracted/fonts/…` の相対パスだけが入り、起点（リポジトリのルート）は a4-multi.sh が与える。
4. **失敗時の扱い**: 全件検査してから書く（all-or-nothing）、書式不正も SHA の不一致と同じ停止、終了コード 2。依頼文の「不一致なら名前付きで停止」を、一覧全体の停止に広げた。
5. **読み戻しの検証**: 依頼文に無い。書いた内容が仮想ボリュームから同じに読めることを、同じハンドルで確かめる（ハンドル番号を余計に使わない）。runtime の `WriteFile`／`ReadFile` の実装の誤りを配置の時点で検出するため。
6. **`census/main_two.c` にも入れた**: 依頼に従った。`run_census.sh` は `FLE_VFS_SEED` を `unset` しない（`FLE_CENSUS` は unset している）。環境に `FLE_VFS_SEED` が残っていると、census の基準の実行も配置される。
7. **新しいヘッダを `lift/harness/` に作った**: 「runtime に新しい .c／.h を作らない」の対象外と読んだ（ハーネスの共有ヘッダ）。2 つの `main_two.c` に同じ約 400 行を複製しないため。a4-multi.sh の stamp は `harness/*.h` を含むので、変更は検出される。
8. **ファイルの属性を `FILE_ATTRIBUTE_NORMAL`（0x80）**にした: 実機のインストール直後の属性（アーカイブ属性など）とは限らない（未確認）。ゲストがファイルの属性に依存するかは未確認。
9. **ホストの実ファイルを見せない**の解釈: 一覧のホストのファイルは、利用者が `tools/extract_nsis2.py` でインストーラから書き出したもの（`lift/out-inventory/extracted/fonts/`）。ホストが元から持っているファイル（`/usr/share/fonts`）ではない。ゲストに見えるのは書き込んだバイト列のコピーだけで、ホストのパスはゲストに見えない（ゲストの API はホストのパスを返さない）。

## 未決の点(PM)

1. **決定 0018 の緩和の承認**（範囲: 一覧に明示した、インストーラ由来のファイル）。撤回条件は上。決定 0101 の A4 の判定が、環境の内容の追加（この 2 ファイル）を「仮の応答」と同様に数えるか／別の欄にするか。`docs/investigations/provisional-decisions.md` と README の「仮」の数（37）に 1 を足すか。
2. **配置先のパス（`…\FL Studio 21\Shared\Artwork\Fonts\`）は決定 0104 の創作値に依存する**。NSIS のヘッダの読みでは、`Shared` の起点の変数が FLEngine の出力先の変数と別（読み。変数名の対応は未確認）で、実機の `Shared` は `FL Studio 21` の子でなく兄弟（`…\Image-Line\Shared`）の可能性がある。`FLE_VIRTUAL_SHARED_DIR` を変えるなら一覧も変える。変えない場合、この仮のパスのまま進めてよいか。
3. **FLEngine 自身のフォント探索は満たされない**: FLEngine の同じコードは `…\FL Studio 21\Artwork\Fonts\ILGlyphsEx.ilfont` を見る。同じ 2 ファイルを `…\FL Studio 21\Artwork\Fonts\` にも置く（一覧に 2 行足す。実機の配置とは別の仮の置き方）かどうか。置くと FLEngine の UI フォント（`OpenSans-Regular.ttf` など）の経路も FreeType に入り、freetype の穴に当たる場所が増える。FLEngine 自身の `ILCursors.ilfont` の参照（`f_815050`）が実行されるかは未確認。
4. **`GetModuleFileNameW`／`A` が成功時に最終エラーを 0 にするか**（runtime の変更）。第 2 段階で、第 1 候補の探索の前の `GetLastError` が 0 でないと分かったとき: `fle_vfs.c` の成功の分岐（`fle_imp_GetModuleFileNameW` の `len < nsize` の枝と、`A` の同じ枝）に `fle_set_last_error(0)` を足す案（2 行）がある。Wine は成功時に `SetLastError(0)` 相当だが、**実機の Windows は未確認**。`tests/vfs.rs`（761〜778 行）と `wall_b.rs`（312〜330 行）の期待値を確認する必要がある。この作業では**入れていない**（runtime に足さない方針）。入れるかは未決。**第 2 段階の実測: 門は通った（`TlsGetValue` が最終エラーを 0 にするため。上の 3）ので、この構成では不要**。
5. **`OpenSans-Regular.ttf`（217,360 B）など、UI フォントの追加**。ILTools の UI フォントの測定が劣化したまま残る。追加すると freetype の穴に当たる範囲が広がる。まず 2 ファイルで freetype の穴を直す順序でよいか。
6. **順序**: 配置を入れて freetype の穴を直す（f1 §8 の 2 の (ii)）か、配置なしで W6 以降へ進む（D）か。この決定は (ii) の準備。
7. 製品データを、git に入れず、ローカルの検証の入力として使うことの可否（権利の解釈はこの作業では行っていない）。

## 限界・未確認

- **第 2 段階で確認できたこと／できなかったこと**: 上の「第 2 段階の結果」を参照。確認できなかったのは、census（`FLE_VFS_SEED` が残ったシェルでの挙動）、wasm、`HeapReAlloc` より先、`ILGlyphsEx.ilfont` を QFC に渡す時期（この停止までは渡されていない）、仮想クロックと `FindFirstFileW("C:\*")` の副作用、実機 Windows との照合（0 件）。以下は第 1 段階の記述のまま。

- コンパイルもテストの実行もしていない。ヘッダは構文確認と、SHA-256 の Python 転記による確認だけ。自己テストは未実行。a4-multi.sh は `bash -n` だけ。
- ILTools も FLEngine も実行していない。上の「配置の後に進む位置」と「副作用」は、逆アセンブルと runtime のソースの読み。**配置後に ILTools が何をするか（`GetLastError` の門、`ILGlyphsEx.ilfont` を QFC に渡す時期、freetype の停止の位置）は未測定**。
- f1 の動的確認（E2）は旧ビルドの 5 モジュール（ILTools 抜き）。HEAD の 6 モジュールでの `FT_New_Memory_Face` への到達は未確認。
- NSIS の変数番号と変数名の対応、実機の `Shared` の位置、実機での属性、実機の `GetModuleFileNameW` の最終エラー、実機での ILTools の探索順は未確認。実機の Windows との照合は 0 件。
- 非 RTL の `GetFileAttributesW`／`CreateFileW`／`CreateDirectoryW`／`FindFirstFileW` の呼び出し元（FLEngine に約 40 か所、ILTools に 11 か所）に渡るパスは追っていない。
- wasm（`__EMSCRIPTEN__`）のハーネスは、ホストのファイルを読む設定が無いので `FLE_VFS_SEED` があれば停止する。ビルドしていない。

## 規則に関する自己申告

- ビルド・リンク・テストの実行はしていない。実行したのは、`clang -fsyntax-only`、`bash -n`、`patch --dry-run`（コピー）、`git apply --check`（リポジトリを変更しない）、`tools/extract_nsis2.py`（`--list`、`--hash`、`--write` 2 件）、capstone／pefile による逆アセンブル（読み取り）、Python による SHA-256 の転記の検査。
- 書いた場所: `/tmp/opencode/f2/`（約 25 MB。上限 100 MB 以内）、`lift/out-inventory/extracted/fonts/`（フォント 2 ファイル 110,604 B）、`lift/out-git/f2-results/`（成果物のテキスト）。リポジトリのソース、`lift/out-*` の既存物、他の `/tmp/opencode` のディレクトリは読み取りのみ。git の変更系操作なし、commit なし。
- 存在しない `bash` ツールを 1 回呼んだ（無効。副作用なし）。Wine のソース（`kernelbase/loader.c`）を `webfetch` で 1 回取得して読んだ（外部の参照。実機ではない）。
