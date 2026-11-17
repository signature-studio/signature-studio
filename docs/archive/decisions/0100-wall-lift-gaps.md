# 0100: lift（生成器）側の穴 2 件 — `.pdata` に無い仮想呼び出し thunk を seed にする、ゲスト自身の setjmp の戻り先（landing）を関数入口にして longjmp を C の巻き戻しで表す

状態: **仮の決定**。PM の指示（census が名前付きで示した lift 側の壁 2 件: unit 296 の `indirect call to 0x477e73`、InitUnits の後の終了フェーズの `unsupported instruction at 0x412752`）を、作業者（サブエージェント）が調査・実装・記録した（2026-10-27）。壁 1 の規則は **FLEngine で 82 個のうち実行で確認できたのは 1 個だけ**、壁 2 の機構は **native でしか確認していない（wasm 未確認）**ので、仮の決定にする。決定 0036 W1 の「`jmp [mem]` は対象外」を、この決定の範囲で解く（下の決定 3）。

記入状況: すべて記入済み。数値は 2026-10-27 の実測（作業物は `lift/out-wallL/`。基準は HEAD 5002d98 + census の差分（決定 0097）。HEAD は作業中に 0e9b703、a951ecc（決定 0095）、f1bcba7 と進んだ。「検証」の最後の行に新 HEAD への適用の確認を書く）。census の数（ok／stop）は **A4 の進捗ではない**（決定 0046、0060、0097、0101）。

事実の出典（節番号は各文書のもの）:

- `lift/out-census/docs-new/wall-census-2.md`（13 行目の壁 13、末尾の `unsupported instruction at 0x412752`）、`wall-census-2-notes.md` §1、§2。決定 0097（census）。
- 決定 0051（`.pdata` 外の this 補正 thunk を seed にする。`add rcx,imm; jmp rel32`）と `docs/investigations/adjustor-thunks.md` §2、決定 0074 と `docs/investigations/dll-nonpdata-reloc-seeds.md` §4〜§7（値が実行範囲内で `.pdata` の範囲に入らないもの 55,586 個。無条件に種にすると FLEngine で `bad_nodes` 0 → 113,485）、`blend2d-seed58.md`。
- 決定 0036（W1: `jmp r64` の tail dispatch。`jmp [mem]` は対象外、`f_4126f0` の `jmp [rcx]` は longjmp 風と書いてあった）。決定 0064（`fle_call_guest`、入れ子カウンタ）、0046／0101（A4 の定義と判定構成）。
- 調査の詳細・数え方・生の出力: `lift/out-wallL/docs-new/wall-l-investigation.md`（本決定の根拠。本書はその要約と規則）。

## 決定（作業者。仮）

| # | 項目 | 決定 |
|---|---|---|
| 1 | 位置づけ | 2 つとも生成器（`--pdata` の seed の規則、`fe-lift` の出力）と runtime の `fle_dispatch.h` の変更。**値（API の応答、仮の値）は一つも決めない**。census は診断のまま、A4 は厳格のまま。census の数は A4 の進捗として書かない |
| 2 | 壁 1: 仮想呼び出し thunk を seed にする（`fe-decode`: `adjustor::vcall_thunks` / `vcall_seeds`。`fe-lift --pdata` の既定、`--no-vcall-thunks` で外す） | 次の**すべて**を満たす値だけを seed に足す。(a) `.reloc` の DIR64 のスロットの値で、実行範囲内、`.pdata` の開始でも範囲の内側でもない（決定 0051 と同じ母集団）。(b) 0〜2 個の `add`/`sub` r32/r64, imm、続いて `mov r64,[base+disp]`（index なし、rip 相対でない、セグメントなし、8 バイト）、続いて `jmp qword ptr [r64+disp]`（`mov` が書いたレジスタが base、index なし、rip 相対でない、セグメントなし）で終わり、これだけ。全体 16 バイト以下。(c) **開始が命令境界である証拠**（デコードだけに頼らない）: 直前のバイトが int3（0xCC）、または `.pdata` の範囲がちょうどそこで終わる、または 0051 か本決定の形の thunk がちょうどそこで終わり、それ自身が同じ証拠を持つ（鎖。再帰）。(d) **ガード**: `.pdata` 開始、0051 の thunk、`--seeds` の seed の閉包の命令にその値が入っていない（既存の関数が流れ込む場所を関数入口にして、既存の関数の `goto` を tail call に変えてしまうことがない）。種は他の seed の後ろに足す |
| 3 | 壁 1 の副: `jmp qword ptr [mem]` の tail dispatch（`fe-lift/src/tailjmp.rs`） | 解決できなかった `jmp qword ptr [mem]`（64 ビットのメモリ操作数）を `{ fle_dispatch(fle_ld(ea, 8)); return; }` にする（読んだ値の関数入口へ tail dispatch。入口でない値は `fle_dispatch_miss` で停止）。決定 0036 W1 が「メモリから読む値は関数ポインタとは限らない」として除いた分を解く。理由: 壁 1 の thunk の本体が `jmp [rax+disp]` で、これを持たないと thunk が `fle_unsupported` で止まる。除いた当時の唯一の例（`f_4126f0` の `jmp [rcx]`）は壁 2 で扱う。ハンドラ名 `jmp_ind:mem_dispatch`（`register_jmps` には数えない） |
| 4 | 壁 2: ゲスト自身の setjmp の landing（`fe-decode/src/setjmp.rs`、`fe-lift`、`runtime/fle_dispatch.h`） | **setjmp の形**（`mov rax,[rsp]; lea rdx,[rsp+8]; mov [rcx],rax; mov [rcx+8],rdx; mov [rcx+0x10..0x48],rbx/rbp/rsi/rdi/r12..r15; … xor eax,eax; ret`。直接か、`lea rcx,[rcx+d]; jmp rel32` / `jmp rel32` の 2 段までの先）の関数への直接 `call` の**戻りアドレス（landing）を関数入口（seed）にする**（`fe-lift` は `--no-landings` で外す）。landing を入口にした関数 f_L への tail call（呼び出し元の関数の本体が landing で切れる）を、`fle_landing` の登録つきの形で出す: `{ fle_landing lj; fle_landing_push(&lj, L, R[4]); if (FLE_LANDING_SETJMP(&lj) == 0) { f_L(); } else { f_L(); } fle_landing_pop(&lj); return; }`。`fle_dispatch(va)` は表を引く前に、登録の中に (va, rsp = R[4]) と一致するものがあれば C の `_longjmp` でその呼び出し元のフレームへ戻る（途中の C フレームは捨てられる）。va は一致して rsp が一致する登録が無ければ名前付きで停止する。ゲストの longjmp が復元した状態（R、FL、X、ゲストのメモリ）以外にフレームの局所状態は無いので、f_L をもう一度呼べば setjmp の 2 度目の戻りになる |
| 5 | wasm | `__EMSCRIPTEN__` では `FLE_LANDING_NOJUMP`（setjmp/longjmp を使わない版）が既定: 登録は持ち、`fle_dispatch(L)` が登録に当たると名前付きで停止する（f_L を捨てたフレームの上で普通に呼ぶことはしない）。emcc の setjmp/longjmp 対応のフラグはこのツリーのスクリプトに無いので、既定を変えない。**wasm でのビルド・実行は未確認**（この作業では emcc を使わない指示）。`FLE_LANDING_FORCE` でジャンプ版を選べる |
| 6 | 作らないもの | (a) 他の形（`jmp rel32` だけの転送 thunk、`push rdx; lea; jmp` の遅延 import stub、`ret` で終わるもの）を種にしない。(b) 値を「無条件に」種にしない（決定 0074 の見送りは変えない）。(c) landing の登録を持たない `jmp [mem]` 全般を longjmp とみなさない。(d) `fle_call_guest`（決定 0064）の入れ子カウンタ、ホストのロックなど、捨てたフレームの副作用を戻さない。(e) A4 の定義（0046／0101）にこの終了フェーズが含まれるかは判断しない。事実として: entry `f_1376820` は Init `f_41d8f0` の後に無条件で `f_4128b0` を呼び、その中の `f_412770`（2 か所のうち 0x4129e8）が longjmp し、entry が `returned from entry` で戻るのはこの longjmp の後である（実測） |

## 壁ごとの表（読み／規則／撤回条件／テスト）

| 壁 | 読み（区分） | 規則 | 撤回条件 | テスト |
|---|---|---|---|---|
| 1: unit 296 `indirect call to 0x477e73`（f_534b00 ← f_534eb0 ← f_4fa2a0、ret 0x534edf） | **実測**: 0x477e73 は `48 83 c1 f0 | 48 8b 01 | ff 60 10` = `add rcx,-0x10; mov rax,[rcx]; jmp [rax+0x10]`（10 バイト）。`.pdata` の開始でも範囲の内側でもなく、DIR64 のスロット 0x477ee0 が値として持つ。決定 0051 の thunk（直接 `jmp` で終わる）と同族で、末尾が vtable のスロットを通る間接 `jmp`。**読み（未確認）**: 多重継承の仮想関数を副基底から呼ぶ thunk。82 個すべて同じ形（`add rcx,-N; mov rax,[rcx]; jmp [rax+d]`。N は 0x10〜0x558、d は 0〜0x2a0）で、各 1 つのスロットだけが指す | 規則 V = 決定 2。FLEngine で 82 個、全部がガードを通る（閉包に入る場所の 0）。関数 +82、命令 +246（82 × 3）、`bad_nodes` 0、`unsupported_total` は増えない（決定 3 のため）。既存関数の本体の変化 0 | 82 個のうち実行で確認できたのは **0x477e73 だけ**（census 全体で 12 回）。残り 81 個が関数入口として使われることの確認は無い。(1) seed にした関数が実在の thunk でないと分かった（例: 実行で同じ番地が別の命令列として使われる）。(2) 他のイメージ（QFC、将来の DLL）で同じ規則がデータの偶然を関数にしてしまう。(3) ガードが働かずに既存関数の本体が変わった | `fe-decode/tests/vcall_setjmp.rs`（バイト列の肯定 4 + 否定 14、実 DLL の 82 個・昇順・長さの内訳 7/47/25/3・0x477e73・REX を欠いた 0x477e74 は `NotAnchored`・閉包の増分と既存関数の命令列が不変・すでに入口なら新しい seed でない）、`fe-lift/tests/vcall_thunk_dll.rs`（生成 C を関数ごとにハッシュ: 既存関数の本体の変化 0、追加 82、f_477e73 の本文）、`fe-lift/tests/tailjmp.rs`（メモリ形が slot の関数に着く／入口でない値で停止） |
| 2: 終了フェーズ `unsupported instruction at 0x412752`（f_4126f0 ← f_412770 ← f_4128b0） | **実測**: f_4126f0 = ゲスト自身の `longjmp(buf, v)`（`xchg edx,eax`、xmm6〜15、rbx/rbp/rsi/rdi/r12〜r15、`mov rsp,[rcx+8]`、`jmp [rcx]`）。f_412680 = 対の `setjmp`（戻りアドレスを `[buf]`、呼び出し後の rsp を `[buf+8]`、rbx…r15 を `+0x10〜0x48`、xmm6〜15 を `+0x50〜0xe0`、0 を返す。jmp_buf は 240 バイト）。f_412760 = `lea rcx,[rcx+0x38]; jmp f_412680`（context の jmp_buf への入口）。f_412770 = context（296 バイト）の jmp_buf を局所にコピーして context を外側のものに戻し、`longjmp(局所のコピー, 1 か 2)` を呼ぶ。**保存される RIP は 0x1376840**（entry `f_1376820` の `call 0x412760`（0x137683b）の次の命令。実行時に gdb で読んだ値: saved rip 0x1376840、saved rsp 0x7ffff7bffe30、edx = 1）。**読み（未確認）**: Delphi の RTL の始動コード（export 名 `dbkFCallWrapperAddr`、`__dbk_fcall_wrapper`）。**FLEngine 全体で数えた数（実測）**: setjmp の呼び元 1（entry の 0x137683b。f_412680 への直接の参照は f_412760 の `jmp` 1）、longjmp（f_4126f0）の呼び元 1（f_412770 の 0x4127bf）、f_412770 の呼び元 2（f_4128b0 の 0x4128ff と 0x4129e8）、f_4128b0 の呼び元 5（entry の 0x137687f を含む）。これらの番地を値に持つ DIR64 のスロットは 0 | 決定 3、4。FLEngine で landing は 1 つ（0x1376840）。entry は `call f_412760; { 登録つきの f_1376840 呼び出し }`、f_1376840 は元の関数の残り。census（dsp 構成・単独構成）の終わりは `unsupported instruction at 0x412752` から **`returned from entry`（終了コード 0、rax = 1）**になった（unit の ok/stop は 296 が stop → ok の 1 件だけ変化） | (1) setjmp の形が他にも出て、規則の形に合わない（形の拡張が要る）。(2) landing と longjmp の間に `fle_call_guest` のフレームがあり、入れ子カウンタが戻らない（census の `cb_depth_leak` で見える）。(3) wasm で setjmp/longjmp が使えず、`FLE_LANDING_NOJUMP` の停止では足りない。(4) 実機 Windows の `longjmp` が jmp_buf の読み方を変える（このツリーは実機未確認） | `fe-lift/tests/setjmp_landing.rs` 8 件: x86 の機械語を組んだ 4 本（setjmp → mid → mid2 → longjmp、setjmp なし、trampoline 経由の 2 度の着地、rsp を壊した longjmp）。**生成 C とホスト CPU（同じバイト列を実行可能ページで実行）の出力が一致**（rax、マーク 10 個、rsp の一致）。捨てたフレームの後続が走らないこと、restore した rbx、`FLE_LANDING_NOJUMP` の停止。変異検査 3 件（`fle_landing_try` の呼び出しを外す／rsp の一致の検査を外す／ラッパーを出さない）で失敗を確認 |

## 背景（事実）

### 壁 1（census 13 行目）

- 母集団の数え方は `wall-l-investigation.md` §1。DIR64 のスロット 278,531、実行範囲内の distinct な値 80,188、`.pdata` 開始 24,592、範囲の内側 10、**範囲に入らない 55,586**（決定 0051 の §2.1 と一致）。このうち命令境界の証拠（.pdata の隙間を int3 で区切って線形走査）を持つもの 1,825、終端（`ret`/`jmp` など）まで不正バイトなしで読めるもの 881。881 の形: 決定 0051 の `add+jmp` 602、`cwde+ret` 115（データの偶然。0051 が除外）、**`add+mov+jmp [mem]` 82（本決定）**、`push+lea+jmp` 57（`.didata` の遅延 import stub）、裸の `jmp` 5、その他 20（`retf`、`hlt` などデータの偶然）。
- 82 個を Python の独立した走査（`py/survey5.py`。pefile でなく自作の PE 読み、capstone）と Rust の実装で別々に数え、**同じ 82 個**だった（アドレス集合が一致）。
- 82 個を指すスロットは各 1 つ（vtable のスロット）。82 個のうち使われるのを実測したのは 0x477e73 だけ（census 全体で 12 回、gdb のブレークポイントの計数）。

### 壁 2（終了フェーズ）

- 上の表の数え方と読み。entry の中の呼び出しの順（実測の `f_1376820` の逆アセンブル）: `call f_412760`（jmp_buf = ローカル rbp+0x80）、`cmp eax,1; setle cl`（`[rbp+0x17c]` に保存）、`test eax,eax; jne 0x1376884`、`call f_41d8f0`（Init）、`call f_4128b0`、`mov eax,[rbp+0x17c]; … ret`。`f_4128b0` の中で `[0x15eacb0] != 0` なら `lea rcx,[0x15eac80]; call f_412770`（0x4129e8）で longjmp する。**longjmp は例外ではなく、通常の終了の経路**（実測: census と同じ実行で必ず通る）。longjmp の値は 1（edx = 1）なので、着地した entry は `eax = 1` で戻る。
- C に直すと、longjmp の `jmp [rcx]` の飛び先は「別の関数（entry）の途中」で、その関数の C フレームは生きている（C の呼び出し鎖は entry → f_4128b0 → f_412770 → f_4126f0）。

## 検討した代替（壁 2）

詳細（制約・実装量・正しさ）は `wall-l-investigation.md` §3。要点:

| 案 | 内容 | 判定 |
|---|---|---|
| A | landing を関数入口にして、呼び出し元の C フレームに `_setjmp` を置く（**採用**） | 関数途中への復帰を作らない。生成 C の変更は landing の tail call 1 か所。捨てたフレームの副作用は戻らない（限界） |
| B | 関数の先頭に「再開 VA」の `switch` を足して途中ラベルへ `goto` | 関数の本体が変わる（landing を持つ関数ごと）。C のフレームの巻き戻しは別に要る。A より大きい |
| C | 全 call の後で「巻き戻し中」フラグを検査 | 既存の全関数が変わる。不採用 |
| D | landing を普通の関数入口にしただけで `fle_dispatch(L)` を入れ子で呼ぶ | 実測で確認できる誤り: f_L が捨てたはずのフレームの上で動き、戻った後に f_4126f0 以降の残りが走る。不採用 |
| E | ホスト側（`fle_call_guest` の外）の setjmp で最外のフレームだけ | 目的のフレーム（entry）が最外のときだけ正しい。一般の setjmp には使えない |
| F | 終了フェーズを「プログラムの終わり」と決め打ち | 値と意味を推測で決める。不採用 |

## 範囲外（今回扱わない）

- jumptab.rs の MSVC switch 認識（決定 0095 の担当）。触っていない。
- 担当外の壁（未応答 import、RaiseException、序数 GetProcAddress、VirtualProtect、レジストリ、Blend2D／QFC／iltools のロード）。
- 壁 1 の 81 個が実行で使われるかの確認、wasm の確認、実機 Windows との照合。

## 実装した場所

- `lift/crates/fe-decode/src/adjustor.rs`（追加のみ。0051 の関数と型は 1 行も変えていない。`Reject::NotAnchored` を足した）: `VcallThunk`、`decode_vcall_thunk`、`vcall_thunks`、`classify_vcall`、`vcall_seeds(pe, &base)`。
- `lift/crates/fe-decode/src/setjmp.rs`（新規）: `is_setjmp_entry`、`landings(text, insns)`。`lib.rs` は `pub mod setjmp;` の 1 行。`main.rs` は `fe-decode vcall-thunks FILE`（候補と、ガードを通ったか）。
- `lift/crates/fe-lift/src/tailjmp.rs`: メモリ形（決定 3）。`src/lib.rs`: `Cx.landings`、`succ` の landing の tail call、`Stats::setjmp_landings`。`src/main.rs`: seed の集め方（他の seed の閉包を 1 回作り、それをガードと landing に使う。2 回目が本番）、`--no-vcall-thunks`、`--no-landings`、出力に `setjmp_landings`。
- `lift/runtime/fle_dispatch.h`: `fle_landing`、`FLE_LANDING_SETJMP`、`fle_landing_push/pop/try`、`fle_dispatch` の先頭の 1 行。runtime の .c は増やしていない。生成 C は FLEngine で 2 関数が変わり 83 関数が増える（検証）。
- 新規テスト: `fe-decode/tests/vcall_setjmp.rs`、`fe-lift/tests/setjmp_landing.rs`、`fe-lift/tests/vcall_thunk_dll.rs`。更新したテスト: `fe-lift/tests/tailjmp.rs`（メモリ形が未対応だった期待を、決定 3 の期待に直した）。

## 検証

数値は `lift/out-wallL/` の `logs/`、`runs/`、`dllstat/`。

| 項目 | 結果 |
|---|---|
| FLEngine の生成 C（`fe-lift lift FLEngine_x64.dll OUT.c --pdata`） | SHA-256 **`1bed5071891bb5e2161c87443edf81357d0e5d39501abe0cfd7b9022c366df45`**（旧 `feb5b908…` から変わる）。functions 59,297 → **59,380**（+83 = thunk 82 + landing 1）、closure_instructions 2,528,030 → 2,528,276（+246）、`bad_nodes` 0、`unsupported_total` 525 → **524**（`Jmp: indirect target unresolved` の 1 件が無くなった。残りは IAT thunk 524）、`tail_calls` 610 → 611、`gotos` 同じ。`--no-vcall-thunks --no-landings` を付けると SHA は `b08dd03e…`（旧 `feb5b908…` と違う）で、変わるのは `f_4126f0` の 1 関数だけ、追加は 0（決定 3 の分。段階の分解は次の行） |
| 旧 C との関数単位の比較（`py/cmpc.py`。自作の分割） | **既存 59,297 関数のうち本体が変わったのは 2 つだけ**: `f_4126f0`（決定 3 の `jmp [rcx]` が `fle_unsupported` から dispatch になった）と `f_1376820`（entry。決定 4 で landing の tail call になった）。**追加 83 関数**（thunk 82、`f_1376840`）。削除 0。関数の外の行は、追加した 83 個のプロトタイプ・83 行の表の行・件数の行を除いて**同一（順序も同じ）** |
| 段階の分解（既存関数の本体の変化 0 の確認） | 旧 → 壁 1 の seed だけ（`jmp [mem]` 未対応のまま）: **変化 0、追加 82**。→ `jmp [mem]` の対応: 変化 1（`f_4126f0`）。→ landing: 変化 1（`f_1376820`）、追加 1。実 DLL の Rust テストでも、seed の有無で既存関数の本体のハッシュが全部一致する（変化 0、追加 82）ことを確認 |
| 壁 1 の追加関数と pdata／応答表の仮定 | 82 個は `.pdata` に無く（unwind 情報なし）、ランタイムは `.pdata` を読まない（`runtime/` に参照 0）。`fle_dispatch_table` は関数入口だけを持ち、行が 83 増えるだけ。応答表（import 応答の `fle_imp_*` の行）には触れない（本体に `call [IAT]` も `jmp [IAT]` も無い） |
| census（dsp 構成。FLEngine + 縮小 dsp_ippv2、基底 0x2000000） | ok 876 → **877**、stop 19 → **18**。変わった unit は **296 だけ**（`indirect call to 0x477e73` → ok。708 入口）。次の壁は出ない（unit は戻った）。終わり: `unsupported instruction at 0x412752`（unit の外）→ **`returned from entry`、終了コード 0、rax = 0x1**。残りの 18 の stop は他の担当の壁 |
| census（単独構成） | ok 873 → **874**、stop 22 → **21**。変わった unit は 296 だけ。終わりは dsp 構成と同じく `returned from entry`、rax = 0x1。**census の数は A4 の進捗ではない** |
| census 無効の a4（厳格） | 単独 **145/942**、停止 `LoadLibraryW "dsp_ippv2_x64.dll"`（不変）。実在モジュール構成（縮小 dsp_ippv2）**150/942**、停止 `LoadLibraryW "blend2d.dll"`（不変）。functions／entries だけ +1（単独 821 → 822、15,685 → 15,686。dsp 構成 943 → 944、26,585 → 26,586）。理由: entry の tail call 先 `f_1376840` が実行されるため（スタックの出力に `#8 0x1376840 f_1376840` の 1 行が増える。stdout のそれ以外の差は入口ログの番号のずれだけ、stderr は完全に一致） |
| 壁 1 の実行の確認 | gdb（読み取りのブレークポイントの計数）で 82 個の入口を数えた census 全体で、実行されたのは **f_477e73 だけ（12 回）**。残り 81 個は 0 回 |
| DLL の lift への影響（`fe-lift lift FILE OUT.c --pdata` の前後。`dllstat/`） | **Blend2D**（rebased）: functions 2,019 → 2,019、closure_instructions 同じ、`unsupported_total` 596 → **584**（`Jmp: indirect target unresolved` 12 が dispatch に）、`bad_nodes` 0。**dsp_ippv2**（`--pdata`、フル）: functions 32,648 → 32,648、`unsupported_total` 974,527 → **974,344**（183 が dispatch に）。**QFC**: functions 6,712 → **6,723**（+11 = thunk 10 + landing 1）、closure_instructions 209,259 → 209,289（+30）、`unsupported_total` 104 → **102**、`setjmp_landings` 1（QFC は FLEngine と同じ RTL の setjmp を 1 回呼ぶと読める。QFC は実行していない）。縮小 dsp_ippv2（census の構成。`--seeds` で lift）: functions 8,030、closure_instructions 3,038,921 とも同じ、C の SHA が変わる（`jmp [mem]` の分。c_bytes +5,699）。**どの DLL も `bad_nodes` は増えない（0）**。DLL の C は全部 SHA が変わる（`jmp [mem]` の対応のため） |
| 全体テスト（`cargo test --workspace --no-fail-fast`、`FLENGINE_PE` 指定、`CARGO_TARGET_DIR=lift/out-wallL/target`。census 適用後の木 + 本差分） | **102 binaries / 1,257 passed / 0 failed**。基準（HEAD 単体 98 binaries / 1,228 passed、census.diff の追加 1 binary・9 件）に、本差分の追加 3 binaries（`vcall_setjmp` 9 件、`setjmp_landing` 8 件、`vcall_thunk_dll` 2 件）と `tailjmp` の 4 → 5 件を足した 98 + 1 + 3 = 102、1,228 + 9 + 19 + 1 = 1,257 に**一致**。既存のテストの期待値を変えたのは `tailjmp.rs`（メモリ形が未対応だった期待）だけで、ほかの既存テストは無変更で通った。**注意**: 1 回の実行ではない。機械のディスク（/、1004G）が全体で 100% になり、最初の実行が `sse_scalar` から ENOSPC で失敗して cargo が止まった（`test-ws.txt` の失敗はすべて「デバイスに空き領域がありません」）。そこで、同じ木のまま、通った 67 binaries（888 件）+ 残りを 4 回に分けて実行した（22 binaries・284 件、1 binary・2 件、fe-pe 10 binaries・83 件、doc-test 2 件）。内訳と根拠は `test-ws-summary.txt`。ENOSPC の文言は後の実行には 0 件。全体テストは**決定 0095 を含まない木**（HEAD 5002d98 + census）で実行した |
| 新しい HEAD への適用 | `lift/out-wallL/patches/wallL.diff`（`diff -u -r -N base src`、`patch -p1` で当たる形。新規 4 ファイルを含む。`lift/out-*` と `target` は除く）。HEAD f1bcba7（決定 0095 を含む。`lift/` は a951ecc から変更なし）の `git archive` に census.diff を当てた木へ、`patch -p1` が**エラーなしで当たる**（`--dry-run` と実適用の両方）。その木で release の `fe-lift` をビルドし直し、FLEngine の生成 C の SHA-256 が `1bed5071891bb5e2161c87443edf81357d0e5d39501abe0cfd7b9022c366df45` で**同じ**（functions 59,380、`bad_nodes` 0）であることを確認。**0095 を含む木では、全体テスト、census、a4 は再実行していない** |

## 解釈が分かれた点

1. **`jmp [mem]` を一般化するか、thunk の形に限るか**: 一般化した。限ると、壁 1 の thunk の本体だけを特別扱いする分岐が生成器に要り、同じ命令が別の関数では未対応のままになる。一般化の影響は DLL の lift の `unsupported_total` が減る（Blend2D −12、dsp −183、QFC −2）ことと、FLEngine の `f_4126f0` の停止の文言が変わることで、飛び先が入口でない場合は従来どおり名前付きで停止する（`fle_dispatch_miss`）。決定 0036 の除外の理由は longjmp だったので、決定 4 を同時に決めた。
2. **landing の登録の持ち方**: 呼び出し元の関数の C フレームに登録する案（採用）と、`fle_call_guest` の外で最外のフレームにだけ持つ案（E）。後者は目的のフレームが最外のときだけ正しい。FLEngine の entry は最外だが、Delphi の RTL の `Halt` は初期化の深いところからも longjmp するので（f_4128b0 の呼び元 5 か所）、前者にした。
3. **setjmp の認識を VA の注釈にするか、形にするか**: 形にした（決定 0036 W2 の jump table は注釈。あちらは範囲の導出が人の読みだったが、setjmp の形は命令列そのもので、誤認識の余地が小さい）。FLEngine で形に合う関数は 2 つ（f_412680 とその入口 f_412760）、呼び出しは 1 か所。他の DLL で合うのは QFC の 1 か所だけ。
4. **thunk の鎖による命令境界の証拠**: 母集団の走査（隙間の線形走査）だけでも 82 個は全部境界だった。生成器の規則には、隙間の走査でなく「直前が int3、`.pdata` の終端、または同形の thunk の終端」を選んだ（データの途中を誤って走査した結果の境界を信用しないため）。長い鎖（鎖の長さが 4,096 を超える thunk 群）は見つからない扱いにする上限を持つ。
5. **ガードを 2 回の閉包で作る**: 「他の seed の閉包」を 1 回作る分だけ lift が遅くなる（FLEngine で約 1.1 秒。全体の lift は約 3 秒）。

## 限界

- **壁 1 は 82 個のうち 1 個しか実行で確認していない**。残りが関数入口である確認は形（命令列）と vtable のスロットから参照されていることだけ。実機 Windows との照合は無い。
- **wasm は未確認**。`FLE_LANDING_NOJUMP`（`__EMSCRIPTEN__` の既定）はネイティブ（`-DFLE_LANDING_NOJUMP`）でだけ試した。wasm ではこの landing を通ると名前付きで停止する（終了フェーズに届く構成では entry が戻らない）。ジャンプ版を wasm で使うには emcc の setjmp/longjmp 対応のフラグが要り、確認していない。
- **捨てたフレームの副作用は戻らない**: `-finstrument-functions` のトレース（`call depth at stop` が 4 のままになる。診断の表示だけ）、`fle_call_guest` の入れ子カウンタ、ホストのロック。FLEngine の終了フェーズでは間に `fle_call_guest` のフレームは無い（実測: 停止時の C のスタックは `f_4126f0` ← `f_412770` ← `f_4128b0` ← `f_1376820` の 4 つだけ。census の `cb_depth_leak` 列も 942 unit で全部 0）。
- **着地する先が死んだフレームのときは検出しない**: landing の登録はフレームが戻ると消えるので、ゲストが戻った関数の jmp_buf へ longjmp すると `fle_dispatch(L)` は普通の関数呼び出しになる（ゲストでも未定義の動作）。rsp が合わない場合だけ名前付きで停止する。
- 壁 1 の規則は FLEngine と QFC で働く。同じ形の thunk が別の DLL にあっても、鎖の証拠（int3／pdata の終端）が無ければ種にならない。
- census の数は A4 の進捗ではない。上の「census」の行は、この 2 件の壁を直した効果の記録。
