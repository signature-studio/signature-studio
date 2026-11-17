# GetKeyboardLayout の再確認と、実装後の harness の再実行の記録（決定 0063）

実行日: 2026-10-22。作業ディレクトリ `signature-studio-develop`、HEAD b3d88bf（決定 0062）に決定 0063 の変更を載せた状態。DLL は `binaries/win64/FLEngine_x64.dll`。決定は `docs/decisions/0063-virtual-keyboard-layout.md`。「実測」はコマンドの出力、「読み」は `lift/objdump-text.txt` の静的な読み、「推測」は未検証。成果物は `lift/out-kbd/`（gitignore 済み。`TMPDIR=lift/out-kbd/tmp`、`EM_CACHE=lift/out-kbd/emcache`）。

`next-stop-getkeyboardlayout.md` と重複する事実は参照だけにする: 停止点（§1）、3 つの呼び出し点と戻り値の使われ方（§2）、Learn の GetKeyboardLayout / GetKeyboardLayoutList / GetKeyboardLayoutNameW / LoadKeyboardLayoutW の引用（§3）、既存の決定（§4）、選択肢（§5）。以下は再確認と新しい事実。

## 1. 再確認したこと

- **Learn の GetKeyboardLayout ページ**（`updated_at` 2025-07-01、`ms.date` 2018-12-05。2026-10-22 に再取得）。前の作業者の引用と一致: 引数 `[in] DWORD idThread`「The identifier of the thread to query, or 0 for the current thread.」。戻り値 `HKL`「The return value is the input locale identifier for the thread. The low word contains a Language Identifier for the input language and the high word contains a device handle to the physical layout of the keyboard.」。Remarks は「入力ロケール ID はキーボードレイアウトより広い概念」、WM_INPUTLANGCHANGE、KLID は GetKeyboardLayoutName。**失敗時の戻り値、GetLastError、HKL の具体的な値はページに無い**（本文を全部読んで確認した）。
- **3 つの呼び出し点は実測で idThread = 0**: 0x84364a、0x86521b、0xa16500 はいずれも `33 c9`（`xor ecx, ecx`）で、call の直前。thunk 0x424d60 は `jmp [0x1971120]` で、IAT スロット 0x1971120 は user32.dll の `GetKeyboardLayout`（インポートテーブルを Python で読んだ。テスト `the_thunk_jumps_to_the_slot_which_is_user32_getkeyboardlayout`）。DLL 全体で 0x424d60 を指す `e8 rel32` は 3 件だけ（バイト走査。テスト `nothing_else_in_the_file_calls_or_points_at_the_thunk`）。thunk やスロットの絶対アドレス、RVA を持つテーブルは無い。
- **戻り値の使われ方（バイト列で固定）**: 0x865226 `48 89 81 90 00 00 00`（rax 全体を `[rcx+0x90]` へ。マスクなし）、0x865e2f `48 3b 81 90 00 00 00`（`cmp rax, [rcx+0x90]`。f_865c90 の中。GetKeyboardLayoutList の要素との比較）、0xa16507〜 `mov edx, eax; and edx, 0xffff`（下位 16 ビットだけ）、0x843651〜 `mov rbx, rax; cmp rbx, [0x14075a0]`（64 ビット全体を前回値と比較）。**上位ワードの内部を見る箇所は 4 か所のどこにも無い**（読み）。テスト `the_guest_uses_the_whole_hkl_the_low_word_and_nothing_inside_the_high_word`。

## 2. f_8651a0 の 0x865222 以降の読み（新規。前の調査 §6 の 3 が「読んでいない」とした部分）

f_8651a0 は直線的なコード（0x865222〜0x865374）。条件分岐は先頭の `cmp byte [rbp+0x68], 0`（0x8651b8、0x86541e）だけ。

1. 0x865222〜0x865226: HKL を `[obj+0x90]` に保存（上記）。
2. 0x86522d〜0x8652a3: `0x40f9e0(rcx = [0x4c9af8], dl = 1)` を 5 回呼び、結果を `[obj+0xa4]`、`[obj+0xac]`、`[obj+0xb4]`、`[obj+0xbc]`、`[obj+0xf4]` に保存（生成関数。中身は追っていない）。`[obj+0xbc]` は後述のリストで、0x8652e7 で dwData に使われる。
3. 0x8652aa〜0x8652d3: `GetDC(NULL)`（thunk 0x424c50）→ `GetDeviceCaps(dc, 0x5a)`（thunk 0x424220。0x5a = 90 = LOGPIXELSY）→ `[obj+0x98]` に保存 → `ReleaseDC(NULL, dc)`（0x4252c0）。**この 3 つは決定 0053 が既に答える**。実行して通った（次の停止点が 0x8652ee なので）。
4. 0x8652da〜0x8652ee: **`0x5dca50(rcx = 0, rdx = 0, r8 = 0x85df70, r9 = [obj+0xbc])`**。thunk 0x5dca50 は `jmp [0x1971058]` で、IAT スロット 0x1971058 は user32.dll の **`EnumDisplayMonitors`**（インポートテーブルを読んだ。実測）。引数は hdc = NULL、lprcClip = NULL、lpfnEnum = 0x85df70、dwData = リスト。
5. 0x8652f3〜0x865374: `0x5c0d40(rcx = [0x5b8148], dl = 1)` を 5 回呼んで `[obj+0x104]`、`[obj+0x10c]`、`[obj+0xfc]`、`[obj+0x114]`、`[obj+0x11c]` に保存。次に `0x8663b0(obj)`。
6. 0x865379〜0x86541b: 5 つのオブジェクトに vtable（0x865fc0）を設定。

**API を直接呼ぶ箇所**: 3 の 3 つ（答え済み）、4 の EnumDisplayMonitors。`0x8663b0` の中（0x866424 `SystemParametersInfoW(0x1f, ...)`、0x866493 `SystemParametersInfoW(0x29, ...)`、`CreateFontIndirectW`、`GetStockObject`）は、条件分岐（0x8663d5、0x8663f5）を評価していないので実行されるかは不明（推測。0058 は 0x1f と 0x29 を名前付きで停止にしている）。

## 3. 実装

- `lift/runtime/fle_kbd.h`、`fle_kbd.c`（新規）: `fle_imp_GetKeyboardLayout` と `fle_kbd_hkl()`（検査用の純粋関数）。作成は `lift/out-kbd/work/` で、clang `-Wall -Wextra -Werror` と emcc `-Wall -Wextra -Werror` の両方で警告なしを確認してから、ヘッダ、`.c` の順に `lift/runtime/` に 1 回で置いた。`cmp` で work と一致を確認。置いた後の編集なし。
- `lift/crates/fe-lift/src/imports_table.rs`: 末尾に注釈 1 行と表の 1 行を追記（編集の直前に読み直した）。`lib.rs` は変更なし（生成 C は thunk のプロトタイプを自分で宣言する）。
- `lift/crates/fe-lift/tests/spi.rs`: `the_row_is_in_the_table_and_declared_in_the_header` の「答えない」一覧から `"GetKeyboardLayout"` を外し、理由をコメントに書いた（決定 0059 で LoadCursorW を外したときと同じ流儀）。
- `lift/crates/fe-lift/tests/kbd.rs`（新規）18 件。変異 21 件を全て検出（`out-kbd/mkmut.py`、`runmut.sh`、`mut-summary2.txt`）。最初の実行で生き残った変異は 2 件: k03（LCID のマスクを外す）は、LCID の上位ワードの bit が `0x0409 << 16` と重なる値（0x00010407）で試していたため同値だった。テストの LCID を 0x00020407 に変えて検出するようにした。k19（rdx を書く）は、rdx は Win64 の揮発レジスタで書いてもよいので、**同値変異だった**。作り直して rbx（callee-saved）を書く変異にし、テストを「callee-saved の保存と rsp」を見る形にして検出した（k14 の「id が理由文から消える」変異は clang の失敗で検出されていた偽の検出だったので、`%u` の位置を動かす変異に作り直した）。

## 4. 実行（実測。native と wasm が一致）

生成 C は変わったので `lift/out-kbd/out` に新規生成（`OUT=$PWD/lift/out-kbd bash lift/harness/wasm/build.sh gen split` → `rt gen-c harness link`、native は `build_native.sh`）。`functions 59297`、`unsupported_total` **541 → 540**（`jmp_imp:GetKeyboardLayout 1`）。

`OUT=$PWD/lift/out-kbd WASM=1 bash lift/harness/a4-survey.sh`（ログ `lift/out-kbd/a4/20261021T200747Z-*`、`lift/out-kbd/survey1.txt`）:

```
stop   : fle: unsupported instruction at 0x5dca50        (user32!EnumDisplayMonitors の thunk、IAT スロット 0x1971058)
depth  : init_count=115 / 942   last unit: index=114 run_rank=111/895 proc=0x623840 in_progress
         started=111 completed=110 in_progress=1 of 895 runnable   all_units_done=no
funcs  : executed (distinct)=582  entries=13379
```

- 呼び出しスタック（深さ 8）: `f_5dca50` ← `f_8651a0`（return 0x8652f3）← `f_620cd0` ← `f_623840`（unit 114）← `f_4120c0` ← `f_412160` ← `f_41d8f0` ← `f_1376820`。
- 前回（GetKeyboardLayout で停止）から 関数実行数 581 → 582、入口数 13,330 → 13,379（+49）。a4 の値は同じ（停止は同じ unit 114 の中）。
- gdb（native）で `fle_imp_GetKeyboardLayout` を数えた: **1 回**、rcx = 0。他の 2 つの呼び出し点は、この停止点までに実行されていない。
- native と wasm: `diff <(grep -v '^calling' native) <(grep -v '^calling' node)` は差なし、stderr も差なし。終了コードはどちらも 134（SIGABRT）。a4 の行 6 行とも同じ（a4-survey が確認し、exit 0）。

## 5. 未確認、限界

- 実 Windows の GetKeyboardLayout(0) の値は未測定。上位ワードは非公式の慣習への合わせ込み（決定 0063）。
- 0x84364c（GetKeyState(0xa5) の後）と 0xa16502 は、まだ実行されていない。0xa16502 の下流（`f_a16460` → `GetLocaleInfoW(0x409, 0x1004)`）が既存の応答で通る読みは、実行で確かめていない。
- `0x8663b0` の条件分岐、`0x40f9e0`、`0x5c0d40` の中身は追っていない。
