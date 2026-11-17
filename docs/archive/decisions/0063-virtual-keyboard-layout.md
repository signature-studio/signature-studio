# 0063: GetKeyboardLayout(0) は仮想の入力ロケール ID 0x04090409 を返す（仮の決定。上位ワードは非公式の慣習に合わせた PM の選択）

状態: **仮の決定**。PM が B1 案（上位ワード 0x0409）を仮の決定として採用し、作業者（サブエージェント）が実装した。下位ワードは Microsoft Learn と決定 0016 から決まる。**上位ワードの値は一次資料に無い**。実装は `lift/runtime/fle_kbd.h`、`fle_kbd.c`、`lift/crates/fe-lift/src/imports_table.rs`（1 行と注釈 1 行の追記）。テストは `lift/crates/fe-lift/tests/kbd.rs`（18 件）。調査と実行記録は `docs/investigations/next-stop-getkeyboardlayout.md`（前の作業者の調査）と `docs/investigations/getkeyboardlayout-caller.md`（再確認と実行後の記録）。「一次」は Microsoft Learn（2026-10-22 取得）、「実測」はコマンドの出力と DLL のバイト列、「読み」は `lift/objdump-text.txt` の静的な読み、「非公式」は Microsoft の文書ではない出典、「仮」は根拠となる一次資料が無い選択。

## 背景

harness は `fle: unsupported instruction at 0x424d60`（user32!GetKeyboardLayout の thunk、IAT スロット 0x1971120）で止まっていた。f_8651a0 の 0x86521d（f_8659f0 のループから戻った直後）。呼び出し点は 3 か所で、3 か所とも `xor ecx, ecx`（idThread = 0）。戻り値の使い方は 3 通り（`next-stop-getkeyboardlayout.md` §2）: 64 ビット全体を `[obj+0x90]` に保存して `GetKeyboardLayoutList` の要素と等値比較（0x865226、0x865e2f）、前回値と 64 ビット全体で比較（0x84364c）、下位 16 ビット（LANGID）だけ使う（0xa16502）。ゲストは上位ワードの中身を解釈しない（実測。`kbd.rs` が該当バイト列を固定している）。

## 決定

1. `fle_imp_GetKeyboardLayout`（`user32.dll!GetKeyboardLayout`）は、引数 ecx（DWORD idThread、rcx の下位 32 ビットだけを見る）が **0** のとき、`R[0]` に 64 ビットの HKL **0x0000000004090409** を返す（上位 32 ビットは 0）。last error には触れない。戻り番地（`[rsp]`）を壊さない。rcx 以外のレジスタと rsp を変えない。状態を持たない。
2. HKL の下位ワードは `fle_emulated_locale.lcid & 0xFFFF`（`fle_kbd_hkl()`）。上位ワードは固定の定数 `FLE_KBD_HKL_HIGH_WORD = 0x0409`。**LCID から導かない**: LCID が変わると下位ワードは追従し、上位ワードは 0x0409 のまま（テスト `the_low_word_is_the_lcid_and_the_high_word_is_not_derived_from_it`。LCID 0x00020407 に差し替えて 0x04090407 を確認する）。LCID の上位 16 ビット（sort id）は下位ワードに混ぜない。
3. idThread が 0 以外のときは `fle_stop` で名前付きの停止（`fle: GetKeyboardLayout: …`、abort）。理由: この環境はスレッドが 1 つでスレッド表が無い（決定 0017、0056）ので、スレッド ID を解決できない。**GetCurrentThreadId の仮想値 0x1001 を渡された場合も停止**し、専用の理由文（「virtual GetCurrentThreadId value」「decisions 0056, 0063」）を出す。仮想 ID は鍵として解決できる表が無い。
4. ホストの実キーボード配列、入力メソッド、X11/Wayland の設定、レジストリは読まない（ホストの実環境をゲストに見せない）。値は仮想で固定し、native と wasm で同じ。
5. 追加の API は実装していない（GetKeyboardLayoutList、GetKeyboardLayoutNameW など。次の停止点は別分野の EnumDisplayMonitors だった）。したがって「GetKeyboardLayoutList の要素に同じ HKL が含まれること」の整合は、その API を実装する将来の作業の条件として残る（下の「将来の整合」）。

## 値と出典の区別

| 項目 | 内容 | 出典 | 状態 |
|---|---|---|---|
| 引数 idThread は DWORD。0 は現在のスレッド | GetKeyboardLayout ページ「The identifier of the thread to query, or 0 for the current thread.」 | **一次**（Learn） | 確認済み |
| 戻り値の下位ワードは LANGID | 同ページ「The low word contains a Language Identifier for the input language」 | **一次** | 確認済み |
| 戻り値の上位ワードは「device handle」 | 同ページ「the high word contains a device handle to the physical layout of the keyboard」。**値は書かれていない**。不透明なハンドルとして扱う | **一次**（性質のみ。値なし） | 確認済み |
| 失敗時の戻り値と GetLastError | ページに記述が無い（Return value に失敗の文が無い） | **一次の欠落** | 確認済み。失敗を作らない（idThread ≠ 0 は値を返さず停止） |
| 下位ワード = 0x0409 | `fle_emulated_locale.lcid` = 0x0409（en-US）の下位 16 ビット | **決定 0016**（この lift 環境の設定。仕様ではない） | 確定（環境の設定） |
| 上位ワード = 0x0409 | 実 Windows で en-US の既定配列の HKL が 0x04090409 になるという慣習。サードパーティの記述（`wine-assembly.berrry.app` の設計メモ、QKeyMapper README）に現れるだけで、Microsoft の文書ではない | **非公式**（一次資料の裏付けなし） | **仮の決定**。PM（作業者の依頼元）の選択。上位ワードの値そのものに一次資料は無い |
| 戻り値の上位 32 ビット = 0 | HKL は 64 ビットのハンドル型だが、値は 0x04090409 で 32 ビットに収まる。Learn は 64 ビットの HKL の上位 32 ビットを書かない | **仮**（0054/0059/0061 の仮想ハンドルと同じ種類の選択） | 仮の決定 |
| idThread に 0x1001（仮想 TID）を渡すと停止 | スレッド表が無い（決定 0017、0056） | 本リポジトリの内部事情 | 確定 |

## 選択肢

- A. 停止のまま。**不採用**: 実行をこの先へ進められない。
- **B1. 0x04090409（採用）**。下位ワードは Learn と 0016 から決まる。上位ワードは実 Windows の慣習（非公式）に合わせる。理由: (1) 上位ワードは Learn の言葉で「device handle」、不透明なハンドルで、ゲストが中身を解釈する証拠は無い（ゲストが使うのは、64 ビット全体の保存と `GetKeyboardLayoutList` の要素との等値比較、前回値との比較、下位 16 ビット = LANGID だけ。`kbd.rs` がバイト列で固定）。0054/0059/0061 の仮想ハンドルと同じ種類の仮の決定である。(2) 下位 16 ビットは Learn と 0016 から決まる。(3) 上位ワードを 0x0409 にするのは実 Windows の慣習に合わせる選択で、一次資料の裏付けは無い。
- B2. 0x00000409（不採用）。LoadKeyboardLayoutW のページ（キーボードレイアウト名 "00000409" は「language identifier (low word) and a device identifier (high word)」と説明される）から、上位ワードを device identifier 0 と読む案。**不採用の理由**: そのページは KLID（レイアウト名）の構成を述べるもので、HKL の値とは書かない（「device identifier」と GetKeyboardLayout の「device handle」は別の言葉）。また上位ワード 0 は「ハンドルが無い」と読める値で、「device handle to the physical layout」という一次資料の文言から遠い。どちらも一次の裏付けは無いので、実機の慣習（B1）を採った。値が分岐に影響するのは、ゲストが HKL を `%.8x` でレジストリのキー名に使う経路（0x865c90 の `Keyboard Layouts\%.8x`）だが、その経路は GetKeyboardLayoutList とレジストリの仮想化が決まらないと実行されない。
- C. 仮想キーボード状態を設計する（HKL の配列、GetKeyboardLayoutNameW の KLID、LoadKeyboardLayoutW、ActivateKeyboardLayout、スレッドごとの入力ロケール、レジストリの `Keyboard Layouts`、GetKeyState/GetKeyboardState との関係）。**今回は不採用**: ゲストがこれらを使う前の段階で、GetKeyboardLayout 単体で先へ進める。

## 将来の整合（今は実装しない）

- `GetKeyboardLayoutList` を実装するときは、配列に `fle_kbd_hkl()` と同じ値を含める（ゲストは 0x865e2f で `[obj+0x90]`（GetKeyboardLayout の保存値）と要素を 64 ビットで等値比較する。含めないと表が作られない経路に入る）。値を二重に書かず、`fle_kbd_hkl()` を呼ぶ。
- `GetKeyboardLayoutNameW` は KLID を返す。LoadKeyboardLayoutW のページは、レイアウト名が「language identifier (low word) と device identifier (high word)」で、米国英語の主レイアウトは "00000409" と書く。HKL の上位ワード 0x0409 と KLID の上位ワード 0x0000 は**別の量**で、一致させる規則は一次資料に無い。実装するときの決定事項。
- `GetKeyState(0xa5)`（0x84364c の前の判定）の設計次第で、0x84364c の GetKeyboardLayout 呼び出しは実行されない（読み。`next-stop-getkeyboardlayout.md` §7）。

## 撤回条件

- Learn または他の一次資料が、en-US の HKL の値、または device handle の値の決め方を書いたとき: 上位ワードをその値にする（`FLE_KBD_HKL_HIGH_WORD` の 1 行とテストの `HIGH_WORD`）。
- ゲストが上位ワードを解釈して分岐すると分かったとき（今は 3 か所すべて、保存・等値比較・下位ワードだけ）: 分岐の条件に合わせて値を決め直す。
- 実 Windows で GetKeyboardLayout の実測ができ、0x04090409 と異なると分かったとき: 実測値にする。
- スレッドを増やす将来（決定 0056 の拡張点）: idThread ≠ 0 の解決とスレッドごとの入力ロケールを別の決定にする。
- PM が B2（0x00000409）に変えるとき: 定数 1 行、テストの `HIGH_WORD`、`kbd.rs` の 0x04090409 の期待値。

## 結果（実測）

- 生成 C は変わった（thunk が応答になった）。`unsupported_total` **541 → 540**（`jmp_imp:GetKeyboardLayout 1`）。runtime、生成 C の 17 パーツ、harness は `errors=0 warnings=0`（`fle_proc.c` の既存の警告 1 件のみ。`fle_kbd.c` は clang `-Wall -Wextra -Werror` と emcc で 0 件）。
- harness: GetKeyboardLayout は 1 回呼ばれ（gdb。rcx = 0）、0x424d60 を通り、停止点は **0x5dca50（user32!EnumDisplayMonitors の thunk）** に進んだ。native と wasm は `calling entry … rsp=` の 1 行を除いて完全一致。詳細は `getkeyboardlayout-caller.md`、次の停止点の判断材料は `next-stop-enumdisplaymonitors.md`。
- テスト: `tests/kbd.rs` 18 件。変異 21 件を全て検出（`lift/out-kbd/mut-summary2.txt`）。

## 限界

- 実 Windows の GetKeyboardLayout(0) の値は実測していない（ホストは Linux）。上位ワードは非公式の慣習への合わせ込み。
- GetKeyboardLayoutList、GetKeyboardLayoutNameW、GetKeyState、レジストリの `Keyboard Layouts` は未実装。HKL の値を使う経路は、それらが答えられるまで実行されない。
