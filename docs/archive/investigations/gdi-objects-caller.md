# CreatePalette 以降の GDI/user32 呼び出しの静的調査と、仮想 GDI オブジェクト表の実装後の harness の再実行の記録（決定 0054）

実行日: 2026-10-21。作業ディレクトリ `signature-studio-develop`、HEAD b9b557b（「virtual screen DC」）。DLL は `binaries/win64/FLEngine_x64.dll`。決定は `docs/decisions/0054-virtual-gdi-objects.md`。「実測」はコマンドの出力（gdb を含む）、「読み」は逆アセンブルの静的な読み、「推測」は未検証。成果物は `lift/out-gdi/`（gitignore 済み。`TMPDIR=lift/out-gdi/tmp`、`EM_CACHE=lift/out-gdi/emcache`、`CARGO_TARGET_DIR=lift/out-gdi/target`）。逆アセンブルは `lift/objdump-text.txt`（objdump）、IAT は pefile、呼び出し関係は `lift/out-gdc/out/wasm-pdata-part-*.c`（実装前の生成 C）の `f_xxxxxx();` から取った。スクリプトは `lift/out-gdi/work/sites.py`（thunk の呼び出し点と所属関数、閉包の判定）、`dis.py`（範囲の逆アセンブル）、`thunks.py`（thunk → import 名）、`args.py`（呼び出し点の直前の引数の命令）。起点は `getdc-caller.md` §3（f_5d2680 の閉包の API の一覧）。

## 1. thunk の確認（実測: pefile の import 表と `ff 25` の thunk）

| API | thunk | 全呼び出し点 |
|---|---|---|
| gdi32!CreatePalette | 0x423ff0 | 9 |
| gdi32!GetStockObject | 0x4242e0 | 10 |
| user32!LoadIconW | 0x4250f0 | 5 |
| gdi32!SelectObject | 0x424520 | 96 |
| gdi32!GetTextMetricsW | 0x424360 | 10 |
| user32!GetSysColor | 0x424ec0 | 1 |
| gdi32!GetSystemPaletteEntries | 0x424300 | 5（全部 f_5c4dc0） |
| gdi32!DeleteObject | 0x424090 | 135 |

（隣の thunk: 0x424290 = GetPaletteEntries、0x424530 = SelectPalette、0x424480 = RealizePalette。f_5c4f30 が使う。）

## 2. 次の停止点までの実行順（読み。gdb の実測で確認したものは明記）

f_5d2680（初期化 unit。`getdc-caller.md` §3）は、`sub [0x15f2f70],1` が -1 のとき（初回）だけ本体に入り、次の順で呼ぶ。

1. `f_5d19e0`（0x5d2698）: GetDC(NULL) → GetDeviceCaps(90) → ReleaseDC → **`f_5c4dc0(0x13d8468, 15)`** → 戻り値を 0x15f2f68 に保存（0053 で実装済みの部分と、今回の CreatePalette）。
2. InitializeCriticalSection を 2 回（0x15f2f98、0x15f2fc0。既存の応答）。
3. **`GetStockObject(7)`**（0x5d26ba）→ 0x15f2f78、**`GetStockObject(5)`**（0x5d26cb）→ 0x15f2f80、**`GetStockObject(0xd)`**（0x5d26dc）→ 0x15f2f88。
4. **`LoadIconW(NULL, 0x7f00)`**（0x5d26ef。ecx = 0、edx = 0x7f00）→ 0x15f2f90。
5. `f_5d1ac0`（0x5d26fb）: `MulDiv(8, [0x15f2f74], 0x48)` → `neg` → 0x13d6a7c。続いて `f_44acd0()` が 1 のときだけ f_5d1a60（SelectObject、GetTextMetricsW）。そのあとレジストリ（`SOFTWARE\Microsoft\Windows NT\CurrentVersion\FontSubstitutes`、`Tahoma`、`MS Shell Dlg 2`。0x5d1c44 の UTF-16 文字列。読み）。
6. 以降は `f_5bfff0`、`f_5d20b0`、`f_50b8b0` などの Delphi のオブジェクト生成（読んでいない）。

### 2.1 CreatePalette（0x5c4eef。f_5c4dc0）

- **引数**（読み）: `rcx = &LOGPALETTE`（`lea rcx,[rbp+0x38]`）。`[rbp+0x38] = 0x0300`（WORD、palVersion。0x5c4dd6）、`[rbp+0x3a] = 0x10`（WORD、palNumEntries。0x5c4ddc）、`[rbp+0x3c]` から 64 バイトを `f_409c80(rcx = 0x13d8468, rdx = rbp+0x3c, r8 = 0x40)` でコピー（0x5c4df3。f_409c80 は memcpy 相当）。
- **表の中身**（実測: DLL の 0x13d8468 の 64 バイト）: 16 エントリ、各 {R, G, B, 0}。`00 00 00 / 80 00 00 / 00 80 00 / 80 80 00 / 00 00 80 / 80 00 80 / 00 80 80 / 80 80 80 / c0 c0 c0 / ff 00 00 / 00 ff 00 / ff ff 00 / 00 00 ff / ff 00 ff / 00 ff ff / ff ff ff`（flags は全部 0）。標準の 16 色。
- **戻り値の使われ方**（読み）: f_5c4dc0 が `[rbp+0x440]` に保存して返し、f_5d19e0 が 0x15f2f68 に保存する（0x5d1a29）。0x15f2f68 の読み手（`# 0x15f2f68` を持つ命令）: 0x5bfa19（`cmp rcx,[0x15f2f68]`。f_5bfa10 = 「rcx が NULL でも 0x15f2f68 でもなければ DeleteObject(rcx)」）、0x5c495a、0x5c49b7（f_5c4740 内。rdx に読む）、0x5c4f44、0x5c4f71、0x5c4f8d（f_5c4f30。0x15f2f68 が 0 でなければ GetDC → GetDeviceCaps(104) → SIZEPALETTE ≥ 16 のとき GetPaletteEntries ×2）、0x5cc3a2、0x5cd97e、0x5ce91d、0x5ce926、0x5cef1f（比較と読み込み）、0x5d2622（f_5d25a0 = 終了処理: `DeleteObject([0x15f2f68])`）。つまり、**このパレットは「標準パレット」としてグローバルに保存され、他の場所で「これは標準パレットか」の比較（f_5bfa10 は削除しない判定）と、削除、SelectPalette 系の呼び出しに使われる**。個々の読み手の先は追っていない。
- **ほかの CreatePalette の呼び出し点**（8 箇所: 0x5c50c9、0x5c96ad、0x5cbca0、0x6b0358、0x6b0cab、0x6b3dfa、0x6b69a8、0x6b8d2a）: どれも f_5d2680 の閉包の外。形は同じ（palVersion 0x300 を書く。0x5c96ad、0x5cbca0 で確認）だが、palNumEntries は変数（0x5c50a1 は 0x10 のとき f_5c4f30 を呼ぶ分岐）で、**16 以外が渡る**。今回の実装は 16 のみを受け、他は停止する。
- **今回の応答**: 実装した（決定 0054）。実測（gdb）: 停止時 0x15f2f68 = 0x7fba0088（slot 33 のハンドル）。

### 2.2 GetStockObject（引数 7、5、13）

- 呼び出し点は 10 箇所。**f_5d2680 の閉包にあるのは 3 箇所**（0x5d26ba、0x5d26cb、0x5d26dc。引数 7、5、0xd）。残りは 0x7fd85c と 0x8054d9（引数 4）、0x860e05（引数 4）、0x866456、0x866544、0x866564、0x866584（引数 0xd）で、閉包の外（引数の `mov ecx,imm` は読み）。
- 数値の意味: BLACK_PEN 7、NULL_BRUSH 5、SYSTEM_FONT 13、BLACK_BRUSH 4（0x7fd85c など）。**非公式**（mingw-w64 `wingdi.h` 1441〜1462 行を読んだ）。Learn は名前だけで数値を書かない（決定 0054）。
- 戻り値の使われ方（読み）: 0x15f2f78（BLACK_PEN）、0x15f2f80（NULL_BRUSH）、0x15f2f88（SYSTEM_FONT）に保存。読み手: 0x15f2f78 は 0x5c3f7a、0x5c4255（f_5c3f50、f_5c4240 内で `rdx` に読み、続く SelectObject の第 2 引数と読める）。0x15f2f80 は 0x5c3f8a、0x5c4285。0x15f2f88 は 0x5c3f9a、0x5c4225、**0x5d1a7a**（f_5d1a60 の `SelectObject(hdc, [0x15f2f88])`）。つまり**ストックオブジェクトは「DC に選択する」ために保存される**。
- 今回の応答: 実装した（5、7、13 のみ。他は停止）。実測（gdb）: 停止時 0x15f2f78 = 0x7fba0020、0x15f2f80 = 0x7fba0018、0x15f2f88 = 0x7fba0038（slot = index なので `0x7fba0000 + 4 * (index + 1)`）。

### 2.3 LoadIconW(IDI_APPLICATION)

- 呼び出し点は 5 箇所。**閉包にあるのは 0x5d26ef だけ**（`xor ecx,ecx; mov edx,0x7f00`。0x7f00 = 32512 = IDI_APPLICATION）。ほかの 4 箇所のうち 0x86969c は同じ形（hInstance NULL、0x7f00）。0x867934（`rcx = [rax]` = モジュールのハンドルらしい、`rdx = 0x867ba0` = 名前の文字列らしい）、0x87fc44、0x9919ba は hInstance が `[rbx+0x310]`（読み）か名前が変数で、**NULL 以外の hInstance、名前での呼び出し**がありうる（今回は停止にしてある）。
- IDI_APPLICATION = 32512 は**一次資料で確認**（About Icons の表。`getdc-caller.md` §3 は非公式としていた。更新）。
- 戻り値の使われ方（読み）: 0x15f2f90 に保存。読み手: 0x5d00e8（`mov rax,[0x15f2f90]; mov [rbp+0x30],rax` = ある条件で icon として使う値を作り、`[rsi+0x20]` に保存する）、0x5d0197（`cmp rax,[0x15f2f90]` = ある値がこの既定のアイコンかを比較）。つまり**アイコンは「既定アイコン」としてグローバルに保存され、フォームのアイコンを決める処理で使われる**。その先（WM_SETICON や RegisterClass の hIcon に渡る経路）は追っていない。
- 今回の応答: 実装した（IDI_APPLICATION だけ）。実測（gdb）: 停止時 0x15f2f90 = 0x7fba0084（slot 32）。

### 2.4 SelectObject と GetTextMetricsW

- 呼び出し点: SelectObject 96 箇所、GetTextMetricsW 10 箇所。**閉包にあるのは f_5d1a60 の 0x5d1a81（SelectObject）と 0x5d1a93（GetTextMetricsW）だけ**。f_5d1a60 は `GetDC(NULL) → SelectObject(hdc, SYSTEM_FONT) → GetTextMetrics(hdc, &tm) → ReleaseDC` の形で、`tm` の `[rsp+0x5c]`（TEXTMETRICW の 1 バイトのフィールド。位置は Learn の TEXTMETRICW を読まずに知識で当てたもので、**未確認**）を返す。
- f_5d1a60 の呼び出し元は f_5d1ac0（0x5d1b06）、0x5c1227、0x6d6516。f_5d1ac0 の呼び出しは `cmp eax,1; jne` で、`eax = f_44acd0()` = 0x15ec7e8 = f_44a9a0 が GetVersionExW の構造体の `[rbp+0x54]` から入れる値。OSVERSIONINFOW の並びは Learn で確認（dwOSVersionInfoSize、Major、Minor、Build、**dwPlatformId**。構造体の先頭が rbp+0x44 なので +0x10 = dwPlatformId）。`fle_osver.c` の platformId は 2 で、**f_5d1a60 は通らない**。**実測（gdb）**: 停止時の 0x15ec7e8 = 2、f_5d1a60 へのブレークの回数 0（`out-gdi/gdb-hits-out.txt`）。前回の記録は「0 のまま、実測していない」だったので、これは新しい実測。
- 今回の応答: 作らない（実行が届かない）。

### 2.5 GetSysColor

- 呼び出し点は 1 箇所（0x5c07fe）。f_5c07f0 は `if (ecx < 0) return GetSysColor(ecx & 0xff); else return ecx;`（`test ecx,ecx; jge`、読み。Delphi の ColorToRGB 相当）。
- f_5c07f0 の呼び出し点は 80 箇所以上ある（`call 0x5c07f0` の grep）。さらに f_5d2680 自身が 0x5d2799 と 0x5d27c9 で `[ [0x15e2bc8] ] = 0x5c07f0`（関数ポインタの格納）を行う（読み）。つまり**GetSysColor は、色の値が負（システムカラーの印）のときに呼ばれ、実行がフォームの描画に届いたときに現れる**。実測（gdb）: 停止時までに f_5c07f0 へのブレークの回数 0。
- 引数の値（COLOR_ の番号）は、呼び出し元ごとの定数で、全体は追っていない。Learn は COLOR_ の名前と 0〜30 の番号を書き、範囲外は 0 を返すと書く（確認した）。
- 今回の応答: 作らない（届かない）。

### 2.6 GetSystemPaletteEntries

- 呼び出し点 5 箇所、全部 f_5c4dc0 の SIZEPALETTE ≥ 16 の分岐の中（0x5c4e3e、0x5c4e66、0x5c4e90、0x5c4eab、0x5c4ed7）。0053 で SIZEPALETTE = 0 にしたので通らない（`getdc-caller.md` §2）。実測: 停止時までに f_5c4dc0 は 1 回通り、CreatePalette に着いた（今回の実行で CreatePalette が応答され、先へ進んだ）。
- Learn（GetSystemPaletteEntries）は、戻り値を「取得したエントリの数」、失敗を 0、pPalEntries が NULL ならパレットの総エントリ数を返す、と書く。0053 の SIZEPALETTE = 0 とは矛盾しない。
- 今回の応答: 作らない（通らない）。

### 2.7 DeleteObject

- 呼び出し点は 135 箇所。**閉包にあるのは 0 箇所**（`sites.py` の D/E 判定。f_5d2680 の閉包に DeleteObject の thunk は無い）。見つけた経路は、f_5d25a0（0x5d2622 で `DeleteObject([0x15f2f68])`、続けて DeleteCriticalSection を 2 回。後始末の関数と読めるが推測。呼び出し元は 0x5c3c26 などで、誰が呼ぶかは追っていない）がパレットを削除するもの。f_5bfa10 は「NULL でも標準パレット（0x15f2f68）でもなければ DeleteObject」で、標準パレットを守る。
- 戻り値の使われ方: 見た 2 箇所（0x5d2622、0x5bfa22）はどちらも戻り値を使わない（`call 0x424090` の直後に使う命令が無い。読み）。残りの 133 箇所は読んでいない。stock や icon を DeleteObject に渡す経路があるかも、135 箇所のうち 2 箇所しか読んでいないので**分からない**（その場合は実行時に名前付きで停止する）。
- 今回の応答: 実装した（決定 0054）。ただし**今回の停止点までの実行では呼ばれていない**（実測: DeleteObject thunk へのブレークの回数 0）。依頼の「CreatePalette / DeleteObject / GetStockObject / LoadIconW」に従った。パレットの削除（f_5d25a0 の経路が将来届いたときの応答）のためで、生きているパレットのみ 1、それ以外は停止する。

## 3. 追えない部分

- 間接呼び出し（`fle_dispatch(...)`）の先: f_5d2680 の閉包の 34 関数は追えていない（`getdc-caller.md` §3 のまま）。
- 閉包の外の呼び出し点（CreatePalette 8、GetStockObject 7、LoadIconW 4、SelectObject 95、GetTextMetricsW 9、DeleteObject 135）は、引数と所属の関数だけ。呼び出し元の鎖と、実行が届くかは見ていない。
- 0x15f2f68、0x15f2f78〜0x15f2f90 の読み手の先（どの API の引数になるか）は追っていない。
- f_5d1a60 の `[rsp+0x5c]` が TEXTMETRICW のどのフィールドか。
- f_5d1ac0 以降の Delphi のオブジェクト生成（f_5bfff0、f_5d20b0、f_50b8b0）の中身。

## 4. 実装

- `lift/runtime/fle_gdi.h`、`fle_gdi.c`（新規。`lift/out-gdi/work/` で完成（clang `-Wall -Wextra -Werror` と emcc `-Wall -Wextra -Werror` の両方で警告なしを確認）させてから、ヘッダ → .c の順に 1 回で `lift/runtime/` に置いた。置いたあとのファイルは work と `cmp` で一致）。値と選択肢は決定 0054。
- `lift/crates/fe-lift/src/imports_table.rs`: 5 行を末尾に追記（コメント 1 行 + 4 行。編集の直前に読み直した。`git diff --stat`: 5 行追加）。`lib.rs` は変更なし。
- `lift/crates/fe-lift/tests/gdi.rs`（新規、21 件）。
- `lift/crates/fe-lift/tests/gdc.rs`: 既存の 1 件（`rows_are_in_the_table_…`）が「GetStockObject、CreatePalette、LoadIconW、…に行が無いこと」を表明していたので、その 3 つを一覧から外し、コメントを 1 つ足した（3 行追加・2 行削除）。0053 の決定の「APIs that need a window or a GDI object table は行が無い」のうち、本決定で答えるようになった 3 つを外しただけで、他の表明は同じ。
- `fe-lift` 本体（`lib.rs`）は変更なし。

## 5. テスト（実測）

- `cargo test -p fe-lift --test gdi`: **21 件すべて ok**（`out-gdi/test-gdi.txt`。70 秒。1 件が 60 秒を超える `stock_indexes_the_guest_does_not_use_stop_by_number` は 24 通りの停止を 1 つずつ clang でビルドするため）。内訳: 表と定数 3（行の存在と重複なし・未実装 API に行が無いこと、決定の値、ハンドル領域が 0053 の DC 領域と重ならないことと 32 ビットの切り詰め）、CreatePalette 4（16 エントリの内容、コピーであること、毎回新しい値・last error、不正な LOGPALETTE の 10 通りの停止とメッセージ）、DeleteObject 2、GetStockObject 3、LoadIconW 2、DC との同居 2、スレッド 1、ソースの検査 2、生成器 2。
- 変異テスト（`lift/out-gdi/work/mkmut.py` が `lift/out-gdi/mut2/g1`〜`g21` を作り、`FLE_GDI_SRC_DIR` で変異したソースを指す。各変異は `-Werror` でコンパイルできることを先に確認）: **21 変異すべてがテストの失敗で検出された（生き残り 0）**。検出した件数（`out-gdi/mut2/summary.txt`）: g1 palVersion の検査を外す（1 件失敗）、g2 16 未満の個数を受ける（1）、g3 stock の DeleteObject が 1 を返す（1）、g4 2 回目の DeleteObject が 1（1）、g5 未知の stock index が黙って NULL（1）、g6 BLACK_PEN が NULL_BRUSH と同じハンドル（2）、g7 LoadIconW が hInstance を無視（1）、g8 LoadIconW が任意の名前を受ける（1）、g9 パレットの内容を保存しない（2）、g10 領域が DC 領域と重なる（2）、g11 パレットの生存数を減らさない（2）、g12 LoadIconW のスレッド検査を外す（1）、g13 未発行のハンドルの DeleteObject が 0（1）、g14 すべてのパレットが同じ slot（1）、g15 rcx の上位 32 ビットを見る（1）、g16 icon の DeleteObject が 1（1）、g17 CreatePalette が last error を 0 にする（2）、g18 NULL の plpal を通す（1）、g19 DeleteObject(NULL) が停止（1）、g20 IDI_APPLICATION の定数が違う（5）、g21 SYSTEM_FONT を pen として記録（1）。最初の g1 は `-Werror` の未使用変数でコンパイルできなかったので、`(void)ver;` を足して作り直し、出力先を `mut2/` に変えた（`mut/` は最初の 21 個の生成物が残っているだけで、実行していない。削除は禁止なので残してある）。
- `cargo test --workspace --no-fail-fast`（`lift/out-gdi/test-ws.txt`、shell のタイムアウト 3,600,000 ms、フォアグラウンド）: awk の集計は `test result` 行 **72**、**696 passed、0 failed、0 ignored**（cargo の終了コード 0）。直前の基準は 71 行、675 件。差は +1 行、+21 件（新規 `gdi.rs` の 21 件）。

## 6. harness の再実行（実測）

生成 C は変わった（4 つの thunk が応答になり、応答表が 4 行増える）ので `lift/out-gdi/out` に新規生成した（`OUT=lift/out-gdi bash harness/wasm/build.sh gen split`）。`functions 59297`、`bad_nodes 0`、`unsupported_total` **551 → 547**（`jmp_imp:CreatePalette 1`、`jmp_imp:DeleteObject 1`、`jmp_imp:GetStockObject 1`、`jmp_imp:LoadIconW 1`）。17 パーツすべてを再コンパイル（`JOBS=2`、emcc は同時 2 本。gen 17 件、rt 35 件（`fle_gdi.c` が増えた分）。すべて `errors=0`。`warnings=0` でないのは既存の `fle_proc.c` が警告 1 件。今回の変更とは無関係で、`fle_gdi.c` は 0 件）。native は `build_native.sh`（clang）で同じパーツをビルド。

| | native | wasm（node） |
|---|---|---|
| 停止 | `fle: LoadLibraryW: module "msctf.dll" is not a virtual module and not a name this environment is defined to lack (external modules are M2; no handle and no failure is invented for them)`（`SIGABRT`、`abort kind: (not a runtime message)`） | 同じ |
| 関数実行数（distinct） | **535**（前 472） | 535 |
| 入口数（calls） | **12,913**（前 12,479） | 12,913 |
| 呼び出し深さ | 8（前 8） | 8 |

- `diff <(grep -v '^calling' run-native.txt) <(grep -v '^calling' run-node.txt)` は差なし（`calling … rsp=` の行だけが違う: native `0x7630ba3fffc0`、wasm `0x11c4ece0`）。stderr も同じ。
- 呼び出しスタック（native と wasm で同じ）: `f_423990`（0x423990 = `kernel32!LoadLibraryW` の thunk）← `f_44db40`（guest-return 0x44db70）← `f_5de9e0`（0x5de9ff）← `f_5deae0`（0x412129）← `f_4120c0` ← `f_412160` ← `f_41d8f0` ← エントリ。f_5d2680 はスタックに無い（= 0x5d2698 から始まった GDI 初期化 unit は最後まで戻った。読み: 戻りはスタックに無いことだけから）。
- 停止点の読み: f_5de9e0 は `[0x13d9ef8] == 0` のとき `f_44db40(L"Msctf.dll", 0x8000)` を呼び、戻り値を 0x13d9ef8 に保存する（0x5de9ee〜0x5de9ff。文字列は 0x5dea18 の UTF-16 を実測で読んだ: `Msctf.dll`）。f_44db40 は `SetErrorMode(0x8000)` → `LoadLibraryW` → `SetErrorMode(元)` の形（0x44db56、0x44db6b、0x44db78。0x8000 = SEM_NOOPENFILEERRORBOX は Learn の SetErrorMode で確認）。f_5dea70（読み）は `[0x13d9ef8] == 0` のとき「0 を返すスタブ」、非 0 のとき「1 を返すスタブ」を関数ポインタ `[0x13d9f00]` に入れて呼ぶ。**ゲストは NULL のハンドルを許容する形**に読める（推測。実行はしていない）。
- gdb（native）で停止時のグローバルを実測（`out-gdi/gdb-native.txt`）: 0x15f2f68 = 0x7fba0088（CreatePalette の戻り）、0x15f2f78 = 0x7fba0020（BLACK_PEN）、0x15f2f80 = 0x7fba0018（NULL_BRUSH）、0x15f2f88 = 0x7fba0038（SYSTEM_FONT）、0x15f2f90 = 0x7fba0084（icon）、0x15f2f74 = 96（LOGPIXELSY）、**0x13d6a7c = -11**（`MulDiv(8, 96, 72)` の `neg`。前回の記録で「実行が着いたら確かめる」としていた項目が確認できた）、0x15ec7e8 = 2、0x13d9ef8 = 0。
- ブレーク（`out-gdi/gdb-hits-out.txt`）: f_5d1ac0 の入口 1 回。f_5c07f0（GetSysColor の関数）、f_5c4f30（SelectPalette 系）、f_5bfa10（DeleteObject の関数）、f_5d1a60（SelectObject、GetTextMetricsW）、DeleteObject の thunk の入口は 0 回。
- 増分（関数 +63、入口 +434）の内訳は**実測していない**（入口ログは先頭 256 件だけ）。f_5d2680 の残り（f_5d1ac0 以降のレジストリ読み、Delphi のオブジェクト生成）と、f_4120c0 の次の unit の途中までを含む（推測）。

## 7. その先に必要になる API の見通し（読みと推測。実装していない）

1. **`kernel32!LoadLibraryW("Msctf.dll")`**（今回の停止点）。`fle_load.c` は「仮想モジュール」と「欠けていると定義した 9 つの DLL」（dwmapi、imm32、msimg32、propsys、rpcrt4、shcore、uxtheme、windowscodecs、wtsapi32）以外を停止する。msctf.dll をどちらにするかは、決定 0015 の範囲で未決。ゲストが NULL を許容する形に読める（§6）ので、「欠けている」にするなら変更は小さいが、**決定は PM**。実行はしていない。
2. 「欠けている」にした場合の次: f_5dea70 → NULL なら 0 を返すスタブ。続く処理は未読。
3. ゲストが LoadIconW の戻り値（0x15f2f90）、stock のハンドル（0x15f2f78〜0x15f2f88）、標準パレット（0x15f2f68）を使う経路: SelectObject、SelectPalette、RealizePalette、GetPaletteEntries（f_5c4f30）、GetTextMetricsW（f_5d1a60。今回は通らない）、CreateFontIndirectW、CreateSolidBrush、CreatePen、GetObjectW、DeleteDC、CreateCompatibleDC、GetSysColor、DestroyIcon など。いつ届くかは未確認。届いたとき、SelectObject は DC の選択状態を持つ設計（0053 の DC 表の拡張）が要る。DeleteObject の「選択中は 0」（決定 0054 の撤回条件）はそのとき一緒に決める。
4. ウィンドウ系（RegisterClassW、CreateWindowExW、メッセージ）は、`getdc-caller.md` §3 の閉包の thunk に無い。ただし間接呼び出しの先は追えていない。

## 8. 触ったファイルと制約

- 変更: `lift/crates/fe-lift/src/imports_table.rs`（5 行追記）、`lift/crates/fe-lift/tests/gdc.rs`（1 つのテストの一覧から 3 つを外す）。
- 新規: `lift/runtime/fle_gdi.h`、`lift/runtime/fle_gdi.c`、`lift/crates/fe-lift/tests/gdi.rs`、`docs/decisions/0054-virtual-gdi-objects.md`、本書、`lift/out-gdi/`（gitignore 済み）。
- `rm`、既存ファイルの `>` 上書き、git の変更操作、`/tmp`、コマンド文字列の `sleep`/`timeout`/`nohup` は使っていない。権限拒否はなし。`lift/iat-unsup-closure.txt` には触っていない。emcc は `JOBS=2` 以下。`lift/out-gdi/work/mkmut.py`（自作の補助スクリプト）には 1 回 `sed -i` を当てた（出力先を `mut/` → `mut2/` に変えるため）。他の既存ファイルに `sed -i` は使っていない。
