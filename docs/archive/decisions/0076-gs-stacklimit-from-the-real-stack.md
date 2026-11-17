# 0076: `gs:[0x10]`（NT_TIB.StackLimit）をゲストが使っているスタックの下端で答える（案 A）

状態: 作業者（サブエージェント）が実装。方針（調査文書 `docs/investigations/teb-peb-model.md` §7 の案 A）は依頼者（PM）の決定。前提: 決定 0032（`gs:[0x30]`、`gs:[0x58]`）、決定 0064（`fle_call_guest`）、決定 0073。区分は調査文書 §0 と同じ（実測 / 読み / 導出 / 非公式・仮 / 未確認）。

## 決めたこと

1. `mov r64, gs:[0x10]`（64 ビットの読み。基底・添字なし）を生成器が通す（`vex.rs` の `mov_seg`。許す変位は 0x30、0x58、0x10）。書き込み（`mov gs:[0x10], r64`）、32 ビット読み、`gs:[0x8]`（StackBase）、`gs:[0x18]`、FS、ALU 形、`gs:[0x60]`（PEB）は未対応のまま（生成時に `fle_unsupported`。テスト `stacklimit_read_is_lifted_and_other_gs_forms_stay_unsupported`）。
2. 値: **ゲストが今使っているスタックの下端**。`fle_teb[0x10]` に置き、`gs:[0x10]` でも `gs:[0x30]`（Self）経由でも同じ値が読める（調査文書 §5.1 の整合）。`fle_teb[0x8]`（StackBase）に上端を置く。
3. 値の出どころ（創作なし）: harness がそのスタックのために確保した領域 [lo, hi)。native は `mmap(16 MiB)`（`harness/main.c`）、wasm は `aligned_alloc(4096, 16 MiB)`（`harness/wasm/main.c`）。harness が `fle_teb_set_stack(lo, hi)`（`runtime/fle_vex.h` に追記。既存ファイルへの追記のみで、新しい runtime ファイルは作っていない）を呼ぶ。StackLimit = lo を **ページ境界へ切り上げた値**、StackBase = hi。
4. `fle_gs_ld8(0x10)` は次のとき名前付きで止める: スタックが登録されていない（StackLimit が 0）、読んだ時点の `R[4]` が登録済みスタックの外（切り替えの更新漏れを検知する）。
5. StackBase（`gs:[0x8]`）: 調査文書 §2.1 の `__chkstk` は `gs:[0x10]` だけを読み、`gs:[0x8]` を読まない（実バイト列で確認）。**StackBase の読みは生成しない**（`fle_teb[0x8]` に値を置くのは、ゲストが Self 経由で読む場合の整合のためだけ。0x17d06f7 の範囲検査がその形。調査文書 §3.4）。

## 切り上げの理由（実測）

`__chkstk` のループは `r11 == r10` のときだけ終わる（調査文書 §2.2）。StackLimit がページ境界でないと、またぐときにループが終わらず下へ走り続ける（調査文書 §6 の「+1」「+0x800」）。native の mmap はページ境界で切り上げは何も変えない。wasm の `malloc(16 MiB)` は下位 12 ビットが 0 でない（この環境で `emcc` + node: `malloc 0x112b8`）ため、**wasm の harness は `aligned_alloc(4096, …)` に変えた**（下端がページ境界の事実になる）。`fle_teb_set_stack` は渡された lo が境界でなければ切り上げる（テスト `unaligned-registered-low-end`: 切り上げで StackLimit が lo+0x1000 になり、最初の書き込みはスタック内の lo+0 のページ。想定どおり `extra=1`）。切り上げた分（最大 4095 バイト）はスタックの下端付近の使えない領域になる。

## スタックの切り替えと追随（読み。`R[4] =` の代入を grep で洗い出した）

| 場所 | 何が起きるか | StackLimit の扱い |
|---|---|---|
| `harness/main.c`、`harness/wasm/main.c`（boot） | 唯一の「スタックを決める」場所 | **ここで `fle_teb_set_stack` を呼ぶ** |
| `fle_callback.c`（`fle_call_guest`） | `R[4]` を呼び出し点へ下げるだけ。専用のスタックを持たない（決定 0064）。同じスタックの下へ使う | 更新不要。テスト `runtime_stop_paths_and_callback_keep_the_stack_limit`（callback）: コールバックの中の `gs:[0x10]` も、Self 経由の `[0x10]`、`[0x8]` も同じ値、戻ると R[4] が元に戻る |
| `fle_win.c`（CreateWindowEx 系: `R[4] = cs` など）、`fle_monitor.c`（`R[4] = rect`） | 呼び出し元のスタックの下に構造体を置き、`fle_call_guest` を呼ぶ。同じスタック | 更新不要 |
| `fle_ole.c`（`reg_has_key`） | **C のローカル配列 `stk[8]` を一時的に `R[4]` にする（別のスタック）** | 更新していない。この間に実行されるのは runtime の `fle_imp_RegOpenKeyExW` / `RegCloseKey` だけでゲストのコードは動かず、`gs:[0x10]` を読む者がいない（読み）。もし将来この間にゲストが走れば、`R[4]` が登録範囲外なので 4. の検査で止まる |
| テスト、オラクル（`harness/oracle/lift_drv*.c` の `OR_STACK`、tests/*.rs の `R[4] = (uint64_t)stk`） | 各自のスタック | 登録しない。`gs:[0x10]` を読む前に登録していなければ 4. で止まる（fail-closed） |

- **「unit のスタック」は見つからなかった**: unit は boot のスタックの上で回ると読める（調査文書 §5.2 と同じ結論。`R[4] =` の代入は上の表で全部）。
- **未確認（網羅の漏れがありうる）**: (a) 生成 C の中で `R[4]` を直接代入する形（`mov rsp, r64`、`xchg`、`leave` 等。スタックの切り替えを guest 自身が行う場合。Fiber、`longjmp`、`RtlRestoreContext` 形）は、`R[4]` の代入の grep（`runtime/`、`harness/`）では見ていない。DLL がそれを行えば、読んだ時点の `R[4]` が範囲外になり 4. で止まる（更新するのではなく止める）。(b) 他のスレッド（`CreateThread` の答えは無い。調査文書 §5.2）。(c) wasm の C スタック（`-sSTACK_SIZE=8MB`）はゲストの `R[4]` と別物で、対象外。

## 実在のバイト列で確認した範囲

- `lift/out-ext/dsp_ippv2_x64.dll` の RVA 0x17daba0（ファイルオフセット 0x17d9fa0）から 78 バイト（0x17dabed の `ret` まで）を読み、テストの定数 `CHKSTK_HEX` と一致（テスト `the_dll_bytes_are_the_constant`）。`mov r11, gs:[0x10]`（0x17dabbc）は `65 4c 8b 1c 25 10 00 00 00`。
- そのバイト列を合成 PE に置き、生成器そのものが出した C（手編集なし）で実行。ホスト CPU では **元のバイト列**を `arch_prctl(ARCH_SET_GS)` で偽の TEB（+0x10 = 実際の下端）を指して実行し、同じメモリ領域で r10、r11、rsp、6 個のフラグ、領域のメモリ（戻り番地の 8 バイトを除く）を比較: **(a) 下が書き込める領域 10 ケース全部一致、(b) 下が PROT_NONE の領域 11 ケース全部一致**（テスト `chkstk_matches_the_host_cpu_with_the_real_stack_bottom_as_stacklimit`）。
- 結果（実測）: フレームが収まる 5 ケース（即値の最小 0x1118、DLL の最大 0xa0b0、ちょうど下端まで、確保 0、下端の近く）は **書き込みなし**。下端をまたぐ 4 ケース（+1 バイト、+1 ページ、+3 ページ、0x800 から）は **下端の下のページに 1 バイトずつゼロを書く**（1 ページ、1 ページ、3 ページ、1 ページ。最初は下端 − 0x1000）。下が PROT_NONE ならホストも生成 C も最初の書き込みで fault。確保が借りを出す巨大値は常に fault（下が書ける領域では走らせない）。
- wasm（emcc、`harness/wasm/build.sh` と同じフラグ、node）: 下が書ける 10 ケースの出力（r10、r11、rsp、フラグ、書いたページ数とアドレス）が native の生成 C と**行ごとに一致**（テスト `wasm_gives_the_same_results_as_native_for_the_generated_chkstk`）。
- これは**ループのモデルが元のバイト列と同じ**という確認で、Windows の実機の StackLimit が何かの確認ではない（調査文書 §6 と同じ限界）。

## fail-open の限界（正直な記述）

- **wasm は下端をまたぐ確保を検知できない**。線形メモリの範囲内なら何を書いても通り、ガードページの仕組みは仕様に無い（調査文書 §4。WebAssembly の load/store の trap 条件は範囲外だけ）。またぐと `__chkstk` は下端の下のページに 0 を書き、そのまま戻る。実測: 上の `straddle-*`（wasm でも `below=1..3`、fault なし）。**隣の線形メモリのデータを黙って壊しうる**。
- native でも、harness のスタックの下に保護領域（ガード）は無い（隣がマップ済みなら黙って壊す。未マップなら SIGSEGV）。案 D（`PROT_NONE` のガード）は採っていない。
- 結論: 案 A は fail-closed ではない。FLEngine は `__chkstk` の形を持たず `gs:[0x10]` も読まない（調査文書 §7）ので、今は表に出ない。dsp_ippv2_x64.dll の `__chkstk` が実際に ATTACH で呼ばれるか、何回か、確保量は**未確認**（調査文書 §9）。動的な確保 97 か所の上限も読んでいない。静的な最大は 0xa0b0（41 KiB）、スタックは 16 MiB。

## 案 C（`__chkstk` の名前付き検査）を採っていない理由と将来の条件

- 理由: (1) 生成器に VA 指定で関数を置換する仕組みが無い（調査文書 §7 の検索 0 件）。新しい仕組みで、他の subagent が `control` / `emit` を編集している最中に触る範囲ではない。(2) 置換は r10、r11、rax を保ち、フラグを `add rsp, 0x10` の結果にまで合わせる必要がある。呼び出し側がフラグを読むかは未確認。(3) DLL のバイト列を lift しない箇所ができ、元のバイト列との突合の対象が変わる。
- 採る条件（将来）: dsp_ippv2_x64.dll の実行が `__chkstk` に届き、確保がスタックの下端に近づく（またぐ）可能性が実測で出たとき。または 97 か所の動的な確保のどれかに上限が無いことが確認されたとき。案 A と排他ではない（C を入れても `gs:[0x10]` は A で答える。調査文書 §7.1）。

## PEB（`gs:[0x60]`）は今回モデル化しない

`gs:[0x60]` を読む 2 箇所（0x17d1a96、0x17d573c）はプロセス終了の経路にだけ現れる（調査文書 §3.3、§8）。**届いたら名前付きで停止する現状のまま**（生成時の `fle_unsupported`）。dsp_ippv2_x64.dll を今回の生成器で lift した出力でも、この 2 箇所は `fle_unsupported` のまま（実測: 2 件）。PEB のアドレス、`ProcessParameters`、`+8` の符号、AppPolicy の答えは決めていない（調査文書 §8.2。値は創作になる）。

## FLEngine への影響（実測）

- `fe-lift lift binaries/win64/FLEngine_x64.dll OUT.c --pdata`: functions 59,297、unsupported_total 531、bad_nodes 0。**生成 C の SHA-256 は基準と同一**（`67827424…d15584d`）。FLEngine に `gs:[0x10]` の形が無いので、生成 C は変わらない。
- a4 survey（native と WASM=1）: 停止は `LoadLibraryW "dsp_ippv2_x64.dll"`、init_count 145 / 942、関数実行数 821、native と wasm の a4 の行は同一。変化なし。
- 変わるのは runtime の `fle_vex.h`（追記）と 2 つの harness の main（登録の 1 行ずつ。wasm はスタックの確保を `malloc` から `aligned_alloc(4096, …)` に変えた）だけ。

## 未確認

- `__chkstk` が ATTACH で実際に呼ばれるか、確保量。DLL 全体の実行（`LoadLibraryW` で止まるため未到達）。
- 「StackLimit = 下端（ガードページの上）」の一次の記述（調査文書 §4。Learn の `!teb` 例と `GetCurrentThreadStackLimits` の「lower boundary」とは整合するが、定義の文は無い）。
- 実機 Windows では StackLimit は「コミット済みの下端」で、確保で下がる（非公式）。この環境の StackLimit は固定（スタック全体が確保済みのため下がらない）。この差が `__chkstk` の挙動に与える影響は、Windows 上の実行と比較していない。
- 生成 C 内の `R[4]` の直接代入によるスタック切り替え（上の (a)）。
- 0x17d06f7（C++ 例外処理系）が Self 経由で `[0x8]` と `[0x10]` を読む経路の実行（DllMain の静的な到達の外。調査文書 §3.4）。値は今は整合する（`fle_teb[0x8]` = 上端、`fle_teb[0x10]` = 下端）が、実行していない。

## 撤回条件

- dsp_ippv2_x64.dll の実行で、この値が一次の根拠のある別の値（実機の StackLimit の定義）と食い違うと分かったとき。
- `__chkstk` がまたぐ確保で実際に呼ばれるのを実測したとき（案 C か案 D の検討を始める。この決定の撤回ではなく追加）。
- スタックを切り替えるゲストの経路（Fiber、`longjmp` 形、スレッド）が実行に現れ、4. の停止に当たったとき: 切り替えの更新を足すか、この決定を見直す。

## 実装とテスト

- 生成器: `lift/crates/fe-lift/src/vex.rs`（1 行 + コメント）。runtime: `lift/runtime/fle_vex.h`（追記）。harness: `lift/harness/main.c`、`lift/harness/wasm/main.c`。
- テスト: `lift/crates/fe-lift/tests/gs_stacklimit.rs`（新規、6 件）。出力は `lift/out-w7/test`。作業物は `lift/out-w7/`（`chkstk/` に oracle・probe の複製）。
