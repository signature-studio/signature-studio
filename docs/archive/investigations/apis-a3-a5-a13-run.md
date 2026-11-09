# グローバルメモリ・アトム・文字列（A3）、ボリューム・標準ハンドル（A5）、メモリ保護・ファイルマッピング（A13）の導入と計測

作業者（サブエージェント）による実行記録。実測と推測を分けて書く。判断と仕様の出典は `docs/decisions/0024-apis-a3-a5-a13.md`。対象は `docs/investigations/remaining-unsupported.md` §5.1 の A3（11 API）、A5（7 API）、A13（5 API）。HEAD は df9fa6f。

## 1. 実装（実測）

- 新規 runtime: `lift/runtime/fle_glob.h/.c`（A3）、`fle_vol.h/.c`（A5）、`fle_vmap.h/.c`（A13）。`lift/out-apis/work/` で `clang -std=gnu11 -O0 -Wall -Wextra -Werror` の警告 0 を確かめ、テストを通してから `lift/runtime/` へ置いた（ヘッダを先、.c を後。`cmp` で作業用と同一を確認）。
- 追記: `imports_table.rs` に **23 行**（A3 11、A5 7、A13 5）。編集の直前に読み直した。他の担当者の行（`fle_res`、`fle_load`、`fle_imagehlp`、`fle_user_nowin`、`fle_winmm` の分）の後ろ、`];` の直前に追記した。既存行は変更していない。`lib.rs` は変更していない。
- 既存 runtime の変更（局所）: `fle_sys.c/.h`（各ページに PAGE_* を持たせる、領域に `alloc_protect`、`fle_sys_region_at`、`fle_sys_set_protection`、`FLE_MIN_APP_ADDR`/`FLE_MAX_APP_ADDR` をヘッダへ移動）、`fle_vfs.c/.h`（`fle_vfs_add_close_hook`、`fle_vfs_file_info`、`fle_imp_CloseHandle` が登録されたフックを先に問い合わせる）。VirtualAlloc とファイル系の既存の動作は変えていない。変更後に `tests/sys.rs` 18 件、`vfs.rs` 22 件、`reg.rs` 10 件、`env.rs` 7 件が通った。
- 実装した API: **23 / 23**。A3 の 11、A5 の 7、A13 の 5 のすべて。ただし各 API の「実装した入力の範囲」は、ページが定める範囲に限った。範囲外は止める（決定 0024「実装しなかった」）。
- 応答表の行数を固定値でテストしていない（新規 3 テストは自分の行の存在だけを見る）。

## 2. テスト（実測）

- 新規テスト: `tests/glob.rs` **25 件**、`tests/vol.rs` **22 件**、`tests/vmap.rs` **23 件**、計 **70 件**。共通の補助は `tests/apis_common/mod.rs`（vol と vmap が使う）。
- 期待値は Rust 側でページから独立に書いた。MulDiv は i128 の厳密計算のモデルで、4,000 件の疑似乱数と 30 件の境界値を照合し、モデル自体も手計算の 11 件で固定した。MEMORY_BASIC_INFORMATION はテスト内の C 構造体と `_Static_assert`（サイズ 48、RegionSize 24、State 32、Type 40）で照合した。
- 検証した内容（要点）:
  - A3: 固定ブロックのハンドル = ポインタ、8 バイト境界、GHND/GPTR のゼロ初期化、GMEM_MOVEABLE のロック数（GlobalLock/Unlock の戻り値と LastError = 0 または 158）、0 バイトの MOVEABLE（GlobalLock が NULL）、ロック中の GlobalFree、確保失敗（NULL と 8）、アトム（範囲 0xC000〜0xFFFF、大文字小文字、参照カウント、`#dddd`、MAKEINTATOM、ローカルとグローバルの分離、16384 件の満杯と再利用）、MulDiv、lstrlenW（UTF-16 単位、NULL は 0）。
  - A5: GetStdHandle の NULL と LastError 不変、GetDriveTypeW（C: は 3、他は 1）、GetLogicalDrives（0x4）、GetLogicalDriveStringsW（バッファの規則と 4 / 5 の戻り値）、GetVolumeInformationW（全出力、省略可能な引数、失敗は 0 と 3）、QueryDosDeviceW（成功、小さいバッファは 122、存在しない文字は 2）、DuplicateHandle（擬似ハンドル→実ハンドル、複製、CLOSE_SOURCE、CloseHandle で閉じる、ファイルマッピングの複製）。
  - A13: VirtualQuery（予約・コミット・ラン分割・ビュー・最高アドレス超は 87）、VirtualProtect（旧保護の出力、ホストの保護でフォルトすること、ページ境界またぎ、未コミットは失敗で無変更、領域またぎ）、匿名マッピングのゼロ初期化とビュー間の一貫性、ファイルマッピング（スナップショット、サイズ規則）、FILE_MAP_* の種類と PAGE_* の対応、FILE_MAP_COPY の私有化、UnmapViewOfFile の失敗、CloseHandle とビューの寿命の順序。
  - 止める入力（SIGABRT、`fle:` のメッセージ、戻り値を返さない）: A3 は 14（メモリ）+ 8（アトム名）+ 1（満杯）= 23 ケース、A5 は 4（GetStdHandle）+ 6（ドライブ種別）+ 1（ドライブ文字列）+ 6（ボリューム情報）+ 7（QueryDosDevice）+ 12（DuplicateHandle）= 36 ケース、A13 は 7（VirtualQuery）+ 1（解放後）+ 12（VirtualProtect）+ 28（マッピング）= 48 ケース（テストの表の項目を数えた）。第 2 スレッドの個別テスト 3 件。ソースがホストのプロセス、時刻、環境、ファイル、ドライブ、`/proc` などを使わないことの検査 3 件。
- テスト作成中の失敗（いずれもテスト側の誤りで、実装は変えていない）:
  1. `glob.rs`: C マクロ `A((const uint16_t[]){...})` が引数の分割で失敗（括弧を足して修正）。
  2. `vmap.rs`: 補助関数 `valloc` が libc の関数名と衝突（`vm_alloc` に変更）。
  3. `vmap.rs` の `duplicate_handle...`: `fle_proc.c` をリンクしていなかった（`apis_common` に追加）。
  4. `vmap.rs` の `protect_spanning_two_regions_fails`: 隣接する 2 領域をホストが割り当てた順序に頼っていて成立しなかった（アドレスを指定する形に変更）。
- ワークスペース全体: `cargo test --offline --workspace --no-fail-fast`。
  - 1 回目（`out-apis/test-ws.txt`）: **900000 ms のタイムアウトで終了せず**。28 バイナリまでの途中経過は 205 件通過、失敗 0（reg の途中で打ち切り）。同時に他の担当者のワークスペーステストが 5 本以上走っており（`pgrep` で確認）、負荷が重かったことが原因の可能性が高い（推測）。
  - 2 回目（`out-apis/test-ws-2.txt`、shell の timeout を 3600000 に延ばして実行）: **EXIT=0、55 バイナリ、502 件通過、失敗 0、無視 0**。この回で `tests/modules.rs`（全 `fle_*.c` をリンクする）の 12 件も通った。

## 3. 生成、clang、リンク（実測）

- 生成器: `lift/out-apis/tool`（`out-vfs/tool` の複製）。896 個の seed（`out-apis/seeds-896.txt`、`out-vfs` の複製）。出力 `out-apis/out/after-apis-896.c`（42,548,949 バイト）。EXIT=0、`.err` は空。functions 8427、closure_instructions 497367（ベースと同じ）。`jmp_imp:` の行は 23 API すべてが生成物に 1 件ずつ出た。
- **unsupported_total と IAT 経由 jmp の未解決数（実測）**。「ベース」は、現在の `imports_table.rs` から本作業の 23 行だけを除いた表で、同じツールと同じ seed で生成したもの（`out-apis/base/`、`gen-base-896.txt`）。

| | unsupported_total | `Jmp: indirect through IAT slot (not lifted)` |
|---|---|---|
| ベース（23 行なし。他の担当者の追記は含む） | 309 | 268 |
| 本作業の後 | **286** | **245** |
| 差 | **−23** | **−23** |

  - 差の 23 は、実装した 23 API の thunk の数と一致する（1 API につき 1 thunk）。
  - 依頼文の「現在 541」は `proctime-run.md` の値（A1、A4 の後）。その後、他の担当者の追記（A2、A6、A9、A10、A12 の分）で、本作業を始めた時点のベースはすでに 309 だった。541 → 286 の差 255 のうち、本作業の分は 23。残りは他の担当者の分（実測の内訳は取っていない）。
- clang（-O0）とリンク: `out-apis/build.sh`、`EXTRA_RT` は `runtime/` の全 `fle_*.c`（既定 7 本を除く 20 本）。EXIT=0（`build-896.txt`）。`out-apis/bin/flengine-after-apis-896`。

## 4. 実行（実測）

`./bin/flengine-after-apis-896 ../../binaries/win64/FLEngine_x64.dll`。EXIT=134（SIGABRT）。出力 `run-apis-896.txt`、`.err`。

- 停止点: `fle: indirect call to 0x40f960 is not a known function entry`（`fle_dispatch_miss`）。呼び出しの深さ 8、実行した関数（distinct）125、関数の呼び出し 658。
- 前回（`proctime-run.md`）は、distinct 97、呼び出し 150、停止点 0x4193c4。つまり実行は進んだ。ただし、この差は他の担当者の追加（GetProcAddress の応答表の行、`fle_load`、`fle_user_nowin` など）と同時の結果で、**本作業の寄与かどうかは分からない**。
- **本作業の 23 API は、この実行で 1 回も呼ばれていない**（23 個の thunk のアドレスを entry log の `callee=` と照合して 0 件）。したがって、23 API は実行では未検証で、テストでのみ検証している。
- 停止点 0x40f960 の読み（objdump）: `push rbx; sub rsp,0x20; mov rbx,rcx; movsxd rcx,[rbx-0x80]; call 0x409410; ...; ret` という関数の先頭。関数の入口（`f_40f960`）が生成物にない（`grep -c f_40f960` は 0）。呼び出し元の 0x410320 は `call [rax-0x30]`（vtable の間接呼び出し）。**この関数の入口が seed に入っていないために dispatch が失敗した**と読める（推測。vtable を作る箇所は調べていない）。本作業の API とは無関係で、担当外。

## 5. 実装しなかったもの（止める）と、その理由

詳細は決定 0024。要点:

- ページが数値や失敗コードを書かない入力は、実装せず止める（例: 数値のない GlobalAlloc の廃止フラグ、GetStdHandle の無効な値、GetVolumeInformationW の小さいバッファ、VirtualQuery の追跡外アドレス、MapViewOfFile の保護の不一致）。
- 設計が要るものは実装せず止める: 名前つきのファイルマッピング、PAGE_READWRITE の仮想ファイルのマッピング（書き戻しの設計）、仮想ファイルのハンドルの DuplicateHandle（`fle_vfs.c` のハンドル表を触る必要）、他プロセス、QueryDosDeviceW の全デバイス列挙、ビューの VirtualProtect、FILE_MAP_EXECUTE。
- 閉包の外の関連 API は、この 23 件に入っていないので触っていない（GlobalReAlloc、GlobalSize、GlobalFlags、GlobalDeleteAtom、GetAtomName、OpenFileMapping、FlushViewOfFile、SetStdHandle など）。

## 6. 公式で確認できなかった点（決定 0024 に明記）

- `FILE_MAP_*` の数値（Learn の MapViewOfFile ページに数値がない）と `GMEM_DDESHARE`/`GMEM_SHARE` = 0x2000: 非公式（windows-sys、mingw-w64）で裏付けた。
- GetLogicalDriveStringsW の成功時の戻り値（「最後の追加の NUL を除いた長さ」の文の読み。例なし）: 4 とした（読み）。
- GlobalFree(NULL) の戻り値、GMEM_FIXED で 0 バイト、各 API の失敗時のエラー番号: ページにないので選択（決定 0024 の「ページにない選択」）。

## 7. 失敗・拒否された操作

- 権限の拒否は 0 件。
- `fle_sys.c` の編集を最初に誤った形で 2 回行い（FLE_PG_COMMITTED の扱い）、同じファイル内で元の文面に戻してから、PAGE_* を持たせる形で入れ直した。最終の差分は `git diff` で確認した（`fle_sys.c` 48 行、`.h` 19 行、`fle_vfs.c` 29 行、`.h` 12 行）。
- シェルの 1 回目の Python 実行は、`cd` のパスの誤りで失敗した（副作用なし）。
- ワークスペーステストの 1 回目が 900000 ms で時間切れ（上記）。2 回目は指示の 900000 ms を超える 3600000 ms の timeout を指定した。
- Microsoft Learn のページ取得で 404 が 2 件（`processenv/nf-processenv-getstdhandle`、`winuser/nf-winuser-makeintatom`）。GetStdHandle は `windows/console/getstdhandle`、MAKEINTATOM は `winbase` 版で取り直せた。

## 8. 残置物

`lift/out-apis/`: `work/`（作業用の複製と .o）、`test/`（テストが生成した C と実行ファイル）、`tool/`、`base/`（ベース計測用のクレートの複製と `Cargo.toml`）、`target-tool`、`target-base`、`out/after-apis-896.c`（約 41 MB）、`out/base-apis-896.c`（約 41 MB）、`bin/`、`build.sh`、各 .o、`gen-*.txt`、`run-apis-896.*`、`test-ws.txt`（1 回目、時間切れの途中経過）、`test-ws-2.txt`（2 回目、全体）、`tmp/`。削除禁止のため残す。
