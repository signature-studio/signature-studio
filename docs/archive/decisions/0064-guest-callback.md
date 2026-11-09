# 0064: ランタイムがゲストの関数を呼ぶ（`fle_call_guest`）の規約

状態: 作業者（サブエージェント）が決定。方針（選択肢 B「ランタイムがゲストのコールバックを呼ぶ」を採用し、汎用の最小機構として作る）は依頼者（PM）。**この環境で最初の「ランタイムからゲストを呼ぶ」例**で、以降の user32（ウィンドウプロシージャ、EnumWindows、タイマー等）の基盤になる。実装は `lift/runtime/fle_callback.h`、`fle_callback.c`、テストは `lift/crates/fe-lift/tests/callback.rs`（18 件）、調査と実行記録は `docs/investigations/callback-caller.md`。最初の利用者は決定 0065（`fle_monitor.c`）。「一次」は Microsoft Learn、「読み」は生成 C（`lift/out-cb/out`）と `lift/objdump-text.txt` の静的な読み、「実測」はコマンドの出力。

## 背景

harness は `EnumDisplayMonitors`（thunk 0x5dca50）で止まっていた（`next-stop-enumdisplaymonitors.md`）。この API はゲストのコールバック（0x85df70）を呼ぶ。それまでの runtime の API は、すべて「R[] の引数を読み、R[0] に戻り値を書いて戻る」だけで、ゲストの関数を呼ぶものは無かった（`fle_winmm.c` の「callback」は `midiOutOpen` のフラグの検査だけ）。

## 規約

関数: `uint64_t fle_call_guest(uint64_t va, uint64_t rcx, uint64_t rdx, uint64_t r8, uint64_t r9)`。`va` は関数の入口の VA。引数は整数 4 つまで（MONITORENUMPROC、WNDENUMPROC、TIMERPROC、WNDPROC は 4 つ以内）。戻り値は rax の 64 ビット全部（呼び出し側が戻り値の型の分だけ使う）。標準 C だけ（インライン asm、setjmp、シグナル、スレッドなし。native と wasm で同じコードが動く）。

### (1) 呼び出し時の R[4]（実装）

1. 入口の R[4] は「実行中の API の戻り番地を指す」値（API の入口では call の直後なので 16n+8。ただし 16n+8 でなくても動く。テストは入口の R[4] の剰余 0、4、8、12 を使う）。それより下は自由で、ここから使う。
2. `call_sp = (entry_sp - 32) & ~15`。**32 バイトのシャドウ空間**（Learn: 「The caller must always allocate sufficient space to store four register parameters」）を取り、**16 バイトに切り下げる**。`call_sp` は call 命令の時点の rsp にあたる（Learn の x64 呼び出し規約に「The stack pointer must remain 16-byte aligned in any region of code that isn't part of an epilog or prolog」とあり、call の時点で 16 の倍数、callee の入口で 16n+8）。
3. `R[4] = call_sp - 8; fle_st(R[4], 8, FLE_CB_RETURN_ADDRESS);`。生成 C の `call` と同じ形。callee は 16n+8 で始まり、`[R[4]]` に戻り番地、`[R[4]+8 .. +40)` が自分のシャドウ空間。
4. 呼び出し: `fle_dispatch(va)`（`fle_dispatch.h`。表を引いて `void (*)(void)` を呼ぶ）。引数は R[1]、R[2]、R[8]、R[9] に置く。
5. 戻り: 生成 C の `ret` は `return;` だけで **R[4] を動かさない**ので、戻った時点の R[4] は戻り番地のスロットを指す。ランタイムが `R[4] == call_sp - 8` を確かめ、`R[4] = entry_sp` に戻す（生成 C の `R[4] += 8` にあたる処理に、シャドウ空間と整列の分を足したもの）。
6. 戻り値: `R[0]`。

**生成 C の `call` との照合（実測、テスト `a_generated_call_and_fle_call_guest_give_the_callee_the_same_stack`）**: 生成 C が出す `sub rsp,0x28; call f` は callee を entry より 0x30 = 48 バイト下の 16n+8 に置く。`fle_call_guest` も entry が 16n+8 のとき callee の rsp を entry - 48 に置く（シャドウ 32 + 整列 8 + 戻り番地 8）。生成 C の呼び出しの形は `{ R[4] -= 8; fle_st(R[4], 8, UINT64_C(<次の命令>)); f_<va>(); R[4] += 8; }`（実測: 生成 C の grep と、テストが生成 C にこの文字列があることを確かめる）。

### (2) callee-saved レジスタと xmm（読み、実装）

- Learn（x64 calling convention）: rbx、rbp、rdi、rsi、rsp、r12-r15、xmm6-xmm15 は不揮発。「They must be saved and restored by a function that uses them」。**関数が自分で保存・復元する**（生成 C の関数は push/pop で R[3]、R[5]-R[7]、R[12]-R[15] を保存する。実測: callback 0x85df70 は `push rsi; push rbx` と `pop rbx; pop rsi`）。したがってランタイムは何も保存しなくてよい。
- それでも**検査**する（実装）: 呼び出し前の rbx(3)、rbp(5)、rsi(6)、rdi(7)、r12-r15(12-15) を控え、戻った後に比べる。違えば `fle: fle_call_guest: the callback changed the callee-saved register <name>` で停止する（callee が規約を破ったか、リフトが間違っている印）。**xmm6-15 は検査しない**（`X[]` は `fle_sse.h` にあり、この機構は `fle_sse.h` に依存させない。リフトされた関数が xmm6-15 を壊す箇所が見つかれば、検査を足す）。
- 揮発レジスタ（rax、rcx、rdx、r8-r11、xmm0-5、フラグ）は callee が自由に変える。テスト `each_callee_saved_register_is_checked_and_each_volatile_one_is_free` が R[0..2、8..11] は自由、R[3、5、6、7、12-15] は停止することを固定する。
- **API の戻り値 R[0] をコールバックの後で上書きしてよいか**: runtime の `fle_imp_*` は呼び出し前後で R[] を保存しない（「R[1] 等の引数を読む → R[0] に戻り値を書く」だけ。実測: `fle_kbd.c`、`fle_cursor.c`、`fle_gdc.c`、`fle_spi.c` の読み）。呼び出し元（ゲスト）は API の呼び出しの前後で揮発レジスタを信じない（Learn: 揮発レジスタは呼び出しで壊れうる）。したがって、コールバックの後で R[0] に API の戻り値を書いてよく、R[1]、R[2]、R[8]、R[9]、R[10]、R[11] が壊れていてもよい。`fle_imp_EnumDisplayMonitors` はそうしている。**一方、API 自身が使う値（引数）は、コールバックの後で R[] から読まない**。コールバックの前に C のローカル変数へ取る（rcx 等は callee が壊すため）。

### (3) 戻り番地に積む値（読み、実測）

- 値: `FLE_CB_RETURN_ADDRESS = 0xCB000000`。イメージ（0x400000〜0x18C0000）の外で、関数の入口にもならない値。ゲストの `ret` が読まないなら何でもよいが、読まれたときに「どの命令でもない」ことが分かる値にした。
- 生成 C の `ret` は `return;` で、戻り番地を読まない（実測: 生成 C に `fle_ld(R[4]` を使う `Ret` は無い）。callee が `[rsp]` を読む箇所（戻り番地を使うトリック）は、**コールバックの閉包に無い**: 動的に実行された関数 16 個（gdb で `__cyg_profile_func_enter` を 0x85df70 の入口から出口まで記録。`lift/out-cb/closure-dyn-85df70.txt`: f_85df70 f_40f9e0 f_410320 f_40f960 f_409410 f_407a50 f_40fb40 f_40d210 f_410360 f_410070 f_50a830 f_50aac0 f_50ade0 f_417fc0 f_417b70 f_409470）の生成 C を読み、`R[4]` を使う行は push/pop/`sub rsp`/`add rsp`/`mov rbp, rsp` だけ（`[rsp]` 直読みも rsp 相対の戻り番地参照も無い。実測: 正規表現による全行の走査、出力は空）。静的な直接呼び出しの閉包は 5 関数（f_85df70、f_40f9e0、f_410320、f_410360、f_50a830）で、間接呼び出し 4 か所（`fle_dispatch(t)`）の先は動的な実測で補った。
- 戻った後に、スロットが元の値のままかを検査する（`the callback changed its return address slot` で停止）。
- **別の callee（今後のコールバック）が `[rsp]` を読む場合**は、この規約の外。その callee を実行で見つけたら、戻り番地に何を積むかをその時点で決める（撤回条件）。

### (4) 再入の深さ（実装）

- コールバックの中でランタイムが再びゲストを呼ぶ形（例: ウィンドウプロシージャの中の `SendMessage`）に対応する。入れ子は **FLE_CB_MAX_DEPTH = 16** まで。それを超えると `fle: fle_call_guest: callback nesting is deeper than FLE_CB_MAX_DEPTH (16): decision 0064` で停止する。16 は**この環境の選択**（Windows の制限はスタックの大きさ）。根拠のある値ではない。
- 内側の呼び出しの callee は、外側の callee の rsp より下の 16n+8 に置かれる（外側の戻り番地、内側のシャドウ空間と戻り番地の分の隙間を持つ。実測: テスト `a_callback_may_make_the_runtime_call_another_callback`）。内側が戻った後、外側の R[4] は元の値に戻る。
- ゲストのスタックの大きさ（harness は 16 MiB）に対する深さの余裕: 1 段あたりの消費は callee のフレーム + 48 バイト程度。16 段で足りなくなることはない（読み。callee が巨大なローカル配列を持てば別）。

### (5) コールバックの中の停止（実装）

- `fle_stop`、`fle_unsupported`、`fle_dispatch_miss` は `abort()` する。**longjmp は使わない**。したがって、コールバックの中で停止したら run 全体が止まる（既存の動作）。テスト `a_stop_inside_a_callback_stops_the_whole_run`。
- `va` が 0、または関数の入口でない値は `fle_dispatch_miss`（`indirect call to <va> is not a known function entry`）または `the callback address is 0` で停止する。

## 選択肢

- **A. ランタイムが生成 C と同じ `call` の形を再現する（採用）**: `fle_dispatch` で関数を引く。生成器に手を入れず、既存の `fle_dispatch_table`（全関数入口を持つ）だけを使う。native と wasm で同じ。欠点: callee の `ret` の整合は検査でしか確かめられない（実際の x86 の ret は戻り番地を読む）。
- B. 生成器が「コールバック用の入口」（引数を取って `R[]` を設定し、戻り値を返す C の関数）を関数ごとに吐く。型付きの C の呼び出しにでき、整合は生成器が保証する。**不採用**: 全 59,297 関数に入口を足すとコードが倍になる。コールバックになりうる関数は実行まで分からない（`lpfnEnum` はゲストの値）。
- C. 実 x86 の `ret` を再現するために、戻り番地をスタックから読んで分岐する「トランポリン」を生成する。**不採用**: 生成 C は関数呼び出しを C の呼び出しで実現しており、戻り番地を読まない前提で動いている。
- D. setjmp/longjmp で停止を巻き戻す。**不採用**: 既存の停止は `abort()` で、巻き戻す利用者がいない。wasm では setjmp が別の仕組み（例外処理）を要する。
- シャドウ空間: 32 バイトを取る（採用。Learn の規約）。取らない案は、callee がシャドウ空間に書くと API の呼び出し元のフレームを壊すので不採用（テスト `the_callee_may_write_its_shadow_space...`）。
- 整列: 切り下げて 16 の倍数にする（採用）。API の入口の R[4] が 16n+8 でない場合でも callee は 16n+8 で始まる。

## 撤回条件

- コールバックの callee が `[rsp]` を戻り番地として読む、または rsp 相対でシャドウ空間の外（戻り番地より上）を読む形が見つかった（戻り番地の値と、積む内容を決め直す）。
- リフトされた関数が xmm6-15 を壊して戻る形が見つかった（検査を足す）。または `fle_call_guest` を使う API が 5 引数以上のコールバック（SetTimer の TIMERPROC は 4、WNDPROC は 4、EnumFontFamiliesEx の FONTENUMPROC は 4、SetWindowsHookEx の HOOKPROC は 3）を要する（引数を増やす）。
- 16 段の上限に実行が届いた（値の見直し）。
- コールバックの中のゲストが別スレッドを作り、そこから API が呼ばれる（決定 0017 の単一スレッド規則を見直す）。
- 実 Windows の callee が戻り番地やシャドウ空間の内容に依存することが示された。

## 確認できていないこと

- 実 Windows の実機でのコールバックの挙動（比較していない）。
- `fle_dispatch` を通る間接呼び出しが、wasm の関数テーブルでネイティブより遅い点の影響（性能は測っていない）。
- xmm6-15 の保存（検査していない）。
- x87 スタックと MXCSR の扱い（callee がそれらを変えて戻る場合、Learn は FPCSR/MXCSR の不揮発部分を callee が復元するとする。検査していない）。
