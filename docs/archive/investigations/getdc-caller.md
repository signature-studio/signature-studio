# GetDC の呼び出し元の調査と、仮想スクリーン DC の実装後の harness の再実行の記録（決定 0053）

実行日: 2026-10-21。作業ディレクトリ `signature-studio-develop`、HEAD a44f45c。DLL は `binaries/win64/FLEngine_x64.dll`。決定は `docs/decisions/0053-virtual-screen-dc.md`。「実測」はコマンドの出力、「読み」は逆アセンブルの静的な読み、「推測」は未検証。成果物は `lift/out-gdc/`（gitignore 済み。`TMPDIR=lift/out-gdc/tmp`、`EM_CACHE=lift/out-gdc/emcache`、`CARGO_TARGET_DIR=lift/out-gdc/target`）。逆アセンブルは `lift/objdump-text.txt`（objdump）、IAT は pefile、呼び出し関係は `lift/out-adj/out/wasm-pdata-part-*.c` の `f_xxxxxx();` から取った（スクリプトは `lift/out-gdc/work/reach*.py`）。

## 1. 停止点の確認

停止は `fle: unsupported instruction at 0x424c50`。0x424c50 は `ff 25 5a bf 54 01`（`jmp [rip+0x154bf5a]`）で、IAT スロットは 0x1970bb0、pefile の import 表で `user32.dll!GetDC`（実測）。呼び出し元の鎖は f_5d19e0（0x5d19ee の `call 0x424c50`）← f_5d2680（0x5d2698）← f_4120c0 ← f_412160 ← f_41d8f0 ← エントリ（実測、停止報告のスタック）。

## 2. f_5d19e0 の読み（確定した事実）

0x5d19e0 から 0x5d1a35 まで（読み）:

```
5d19ec  xor ecx,ecx                    ; hWnd = NULL
5d19ee  call 0x424c50                  ; GetDC(NULL)
5d19f3  mov [rbp+0x38],rax             ; hdc を保存
5d19f8  mov rcx,[rbp+0x38]
5d19fc  mov edx,0x5a                   ; index 0x5a = 90
5d1a01  call 0x424220                  ; GetDeviceCaps(hdc, 90)   (0x424220 = gdi32!GetDeviceCaps)
5d1a06  mov [0x15f2f74],eax            ; 戻り値を .data のグローバルに保存
5d1a0d  xor ecx,ecx
5d1a0f  mov rdx,[rbp+0x38]
5d1a13  call 0x4252c0                  ; ReleaseDC(NULL, hdc)     (0x4252c0 = user32!ReleaseDC)
5d1a18  lea rcx,[0x13d8468]; mov edx,0xf
5d1a24  call 0x5c4dc0                  ; 16 色の静的な表（0x13d8468）から CreatePalette する関数
5d1a29  mov [0x15f2f68],rax            ; パレットのハンドルをグローバルに保存
```

- **GetDC の引数**: hWnd は **NULL**（`xor ecx,ecx`、読み）。GetDC の全呼び出し点は 36 箇所で、直前に `xor ecx,ecx` があるのは 31 箇所（残り 5 箇所 0x605098、0x60af5f、0x61758c、0xa26060、0xa28627 は `rcx` を別の経路で作る。読んでいない）。
- **戻り値の使われ方**: `[rbp+0x38]` に保存し、GetDeviceCaps の `rcx` と、ReleaseDC の `rdx` に渡すだけ。
- **後続の API と index**: GetDeviceCaps(hdc, **0x5a = 90**) 1 回、ReleaseDC(NULL, hdc) 1 回（f_5d19e0 の中。読み）。f_5c4dc0（呼び出し先）で GetDC(NULL)、GetDeviceCaps(hdc, **0x68 = 104**)、（104 の戻り値が 16 以上のときだけ）GetSystemPaletteEntries（先頭 8 エントリの読み出しの後、その内容が 0xc0c0c0 なら続けて 3 回、そうでなければ 1 回。呼び出し点は 0x5c4e3e、0x5c4e66、0x5c4e90、0x5c4eab、0x5c4ed7 の 5 箇所）、ReleaseDC(NULL, hdc)、CreatePalette（読み）。CreateCompatibleDC は f_5d19e0・f_5c4dc0 では呼ばれない。
- **index の数値の意味**: Learn の GetDeviceCaps は名前だけを書き、数値を書かない。数値は mingw-w64 の `wingdi.h` と docs.rs の windows-sys の定数ページ（どちらも非公式）で確認した: LOGPIXELSY = 90、SIZEPALETTE = 104（決定 0053 の表）。つまり 90 = **LOGPIXELSY**、104 = **SIZEPALETTE**。
- **値の用途**:
  - LOGPIXELSY（96）→ グローバル 0x15f2f74。読み: f_5d1ac0（0x5d1ae3）が `MulDiv(8, [0x15f2f74], 0x48 = 72)` を呼び、`neg` して 0x13d6a7c に保存する。「8 ポイントの文字の高さを論理単位（負の値）にする」式で、フォントの高さの計算に使う。名前（`PixelsPerInch` 相当）は推測。f_5d1ac0 は停止点より後で、まだ実行されていない（gdb で停止時の 0x13d6a7c が 0 であることを実測）。
  - SIZEPALETTE → f_5c4dc0 の `cmp [rbp+0x43c],0x10; jl 0x5c4edc`。16 未満なら GetSystemPaletteEntries の呼び出しを全部飛ばして、そのまま ReleaseDC（0x5c4ee6）と CreatePalette（0x5c4eef）へ進む（読み）。16 以上なら、システムパレットの先頭 8 エントリと末尾 8 エントリを読み、その内容（`cmp [rbp+0x58],0xc0c0c0`）に応じて、ローカルの 16 エントリの表（コピーした 0x13d8468 の内容）の一部を書き換える（読み。書き換えの意味は追っていない）。つまり、システムパレットがある（8 ビット以下の）画面でだけ、表を画面のパレットに合わせる処理と読める（推測）。SIZEPALETTE が 0 なら表は静的な 16 色のまま CreatePalette に渡る。
  - GetDC・ReleaseDC の戻り値自体は、色深度や DPI の判定には使われない。
- **f_5d19e0 の GetDeviceCaps に使われない index**: HORZRES、VERTRES、BITSPIXEL、PLANES、LOGPIXELSX は f_5d2680 から静的に追える範囲では呼ばれない（下の §3）。

## 3. 初期化 unit（f_5d2680）から先の GDI／ウィンドウ系の到達範囲（静的）

方法: f_5d2680 から、直接の `call f_xxxxxx();` を辿って閉包を作った（341 関数）。さらに、関数本体の中の `UINT64_C(0x…)` が別の関数の開始に当たる場合（関数ポインタの格納）も辿ると 357 関数。`fle_dispatch(...)`（間接呼び出し）を含む関数が 34 あり、その先は**追えていない**。したがって以下は到達の**下限**で、間接呼び出しの先に別の API があるかは分からない（追えない部分）。

閉包の import thunk は 52（直接）／53（アドレス定数込み）。このうち user32／gdi32:

| API | 呼ばれる場所（読み） | 応答 |
|---|---|---|
| user32!GetDC | f_5d19e0、f_5c4dc0、f_5d1a60 | **今回実装**（NULL のみ） |
| user32!ReleaseDC | 同上 | **今回実装** |
| gdi32!GetDeviceCaps | f_5d19e0（90）、f_5c4dc0（104） | **今回実装**（7 index） |
| gdi32!CreatePalette | f_5c4dc0（0x5c4eef。**今回の新しい停止点**） | 無し |
| gdi32!GetSystemPaletteEntries | f_5c4dc0（SIZEPALETTE ≥ 16 のときだけ） | 無し。SIZEPALETTE = 0 の決定で通らない |
| gdi32!GetStockObject | f_5d2680 の 0x5d26b5、0x5d26c6、0x5d26d7（引数 7、5、0xd。数値の意味は mingw の `wingdi.h`: BLACK_PEN 7、NULL_BRUSH 5、SYSTEM_FONT 13。非公式） | 無し |
| user32!LoadIconW | f_5d2680 の 0x5d26ef（`hInstance` NULL、`lpIconName` 0x7f00。0x7f00 = 32512 = IDI_APPLICATION、mingw の `winuser.h`。非公式） | 無し |
| gdi32!SelectObject、GetTextMetricsW | f_5d1a60（フォントの文字セットの確認） | 無し。下を参照 |
| user32!GetSysColor | アドレス定数の経路のみ | 無し |
| user32!LoadStringW、CharNextW | | 既存の応答 |

- f_5d1a60（SelectObject、GetTextMetricsW を呼ぶ）は f_5d1ac0 の 0x5d1b06 から、`f_44acd0` の戻り（グローバル 0x15ec7e8）が 1 のときだけ呼ばれる。0x15ec7e8 は f_44a9a0 が `GetVersionExW` に渡した構造体の `[rbp+0x54]`（構造体は `rbp+0x44` から。差 0x10 は OSVERSIONINFOW の dwPlatformId の位置）から入れる値（読み。構造体のオフセットは Learn の OSVERSIONINFOW を読まずに知識で当てた）。`fle_osver.c` の platform_id は 2（VER_PLATFORM_WIN32_NT）で、比較は 1 との一致なので、この呼び出しは通らないと読める（実測はしていない。0x15ec7e8 は停止時にまだ 0）。f_5d1a60 には 0x5c1227、0x6d6516 の別の呼び出し元もあり、そちらは見ていない。
- ウィンドウ系（CreateWindowExW、RegisterClassW、メッセージ関係）は、閉包の import thunk に**無い**（52／53 の thunk の中に user32 は上の 5 つ + GetSysColor だけ）。ただし間接呼び出しの先（34 関数）は追えていない。
- GetDeviceCaps の呼び出し点は DLL 全体で 31 箇所。f_5d2680 の閉包（直接の call のみ）にあるのは 2 箇所（0x5d1a01、0x5c4e13）だけ。31 箇所全部の index（直前の `mov edx,imm` の読み）は 4（HORZSIZE）、6（VERTSIZE）、8（HORZRES）、10（VERTRES）、12（BITSPIXEL）、14（PLANES）、0x58 = 88（LOGPIXELSX）、0x5a = 90（LOGPIXELSY）、0x68 = 104（SIZEPALETTE）の **9 通り**。このうち 4 と 6 は決定 0053 で名前付きの停止にしている（物理サイズ。値の根拠が無い）。今後の実行で、この初期化 unit の外の関数が呼ばれると、4 と 6 に当たる可能性がある。

## 4. 実装

- `lift/runtime/fle_gdc.h`、`fle_gdc.c`（新規。`lift/out-gdc/work/` で完成（clang `-Wall -Wextra -Werror` と emcc の両方で警告なしを確認）させてから 1 回で `lift/runtime/` に置いた）。`GetDC`（hWnd NULL のみ）、`ReleaseDC`（hWnd NULL のみ）、`GetDeviceCaps`（7 index）。値と選択肢は決定 0053。
- `lift/crates/fe-lift/src/imports_table.rs`: 3 行を末尾に追記（編集の直前に読み直した。`git diff --stat`: 4 行追加）。`lib.rs` は変更なし。
- `lift/crates/fe-lift/tests/gdc.rs`（新規、17 件）。

## 5. テスト（実測）

- `cargo test -p fe-lift --test gdc`: 17 件すべて ok。
- 変異テスト（`FLE_GDC_SRC_DIR` で変異したソースを指す。`lift/out-gdc/mut/`）: 9 変異すべてがテストの失敗で検出された（生き残り 0）。HORZRES を 1919 に変える（3 件失敗）、GetDC が hWnd 非 NULL を通す（1 件）、解放済みの ReleaseDC が 1 を返す（1 件）、未使用 index が黙って 0 を返す（1 件）、HORZSIZE が値を返す（1 件）、2 つ目のスレッドの検査を外す（1 件）、LOGPIXELSY を 120 にする（2 件）、SIZEPALETTE を 256 にする（2 件）、hdc の検査を外す（1 件）。最初に作った 2 つ（index の変異と hdc の変異）は `-Werror` の未使用変数でコンパイルできず、10 件の失敗はテストの検出ではなかったので、作り直して再実行した結果が上の数字（`m4b`、`m9b`）。
- `cargo test --workspace --no-fail-fast`（`lift/out-gdc/test-ws.txt`、shell のタイムアウト 3,600,000 ms、フォアグラウンド）: awk の集計は `test result` 行 **71**、**675 passed、0 failed、0 ignored**。直前の基準は 70 行、658 件。差は +1 行、+17 件（新規 `gdc.rs` の 17 件）。

## 6. harness の再実行（実測）

生成 C は変わった（3 つの thunk が応答になり、応答表が 3 行増える）ので `lift/out-gdc/out` に新規生成した（`build.sh gen split`）。`functions 59297`、`bad_nodes 0`、`unsupported_total` **554 → 551**（`jmp_imp:GetDC 1`、`jmp_imp:ReleaseDC 1`、`jmp_imp:GetDeviceCaps 1`）。17 パーツすべてを再コンパイル（`JOBS=2`、emcc は同時 2 本。gen 17 件、rt 34 件、すべて `errors=0`）。

| | native | wasm（node） |
|---|---|---|
| 停止 | `fle: unsupported instruction at 0x423ff0`（`abort kind: fle_unsupported`） | 同じ |
| 関数実行数（distinct） | **472**（前 468） | 472 |
| 入口数（calls） | **12,479**（前 12,471） | 12,479 |
| 呼び出し深さ | 8（前 7） | 8 |

- `diff <(grep -v '^calling' run-native.txt) <(grep -v '^calling' run-node.txt)` は差なし（`calling … rsp=` の行だけが違う: native `0x74aad5ffffc0`、wasm `0x11c4e900`）。
- 呼び出しスタック（native と wasm で同じ）: `f_423ff0`（0x423ff0 = `gdi32!CreatePalette` の thunk）← `f_5c4dc0`（guest-return 0x5c4ef4）← `f_5d19e0`（0x5d1a29）← `f_5d2680` ← `f_4120c0` ← `f_412160` ← `f_41d8f0` ← エントリ。
- 増分（関数 +4、入口 +8）の内訳は**実測していない**（入口ログは先頭 256 件だけ）。読みからの推測: 入口は f_424220 ×2（0x5d1a01、0x5c4e13）、f_4252c0 ×2（0x5d1a13、0x5c4ee6）、f_424c50 の 2 回目（0x5c4dfa）、f_5c4dc0、f_409c80（0x5c4df3 の表のコピー。memcpy 相当）、f_423ff0 で、前の停止時に入口に数えられていた f_424c50 の 1 回を除くと +8 になり、実測の +8 と合う。関数（distinct）は f_424220、f_4252c0、f_5c4dc0、f_423ff0 の 4 つが新しく、f_409c80 は前から実行済みだったと考えると +4 に合う（f_409c80 が前に実行済みだったかは確かめていない）。
- gdb（native）で停止時のグローバルを実測: 0x15f2f74 = **96**（LOGPIXELSY の保存先）、0x15f2f68 = 0（CreatePalette の戻りがまだ入っていない）、0x13d6a7c = 0、0x15ec7e8 = 0（まだ f_5d1ac0 に着いていない）。

## 7. 未確認、限界

- 実 Windows との比較はしていない。GetDC・GetDeviceCaps が実機で返す値は見ていない。
- 間接呼び出しの先（f_5d2680 の閉包の 34 関数）は追えていない。§3 の一覧は下限。
- f_5d1ac0 以降（0x5d1ac0 の MulDiv、レジストリの `FontSubstitutes`、`Tahoma`、`MS Shell Dlg 2` 等）は、実行がまだそこまで進んでいないので読みだけ。
- LOGPIXELSY の値が 96 のとき、MulDiv(8, 96, 72) = 11（`fle_imp_MulDiv` の丸め規則の結果）。実際に 0x13d6a7c に -11 が入るかは、実行が f_5d1ac0 に着いたら確かめる。
- BITSPIXEL、PLANES、LOGPIXELSX、HORZRES、VERTRES はこの初期化 unit では使われない。値は決定 0053 の選択で、ゲストの挙動の裏付けは無い。

## 8. 触ったファイルと制約

- 変更: `lift/crates/fe-lift/src/imports_table.rs`（3 行 + コメント 1 行）。
- 新規: `lift/runtime/fle_gdc.h`、`lift/runtime/fle_gdc.c`、`lift/crates/fe-lift/tests/gdc.rs`、`docs/decisions/0053-virtual-screen-dc.md`、本書、`lift/out-gdc/`（gitignore 済み）。
- `rm`、既存ファイルの `>` 上書き、git の変更操作、`/tmp`、コマンド文字列の `sleep`/`timeout`/`nohup` は使っていない。権限拒否はなし。`lift/iat-unsup-closure.txt` には触っていない。emcc は `JOBS=2` 以下。
