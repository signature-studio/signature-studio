# 壁 L（lift 側の穴 2 件）の調査: 関数入口でない間接 call の飛び先（unit 296）と、終了フェーズの `jmp [rcx]`（0x412752）

状態: 完成（2026-10-27）。決定は `0100-wall-lift-gaps.md`（仮の決定）。凡例: **実測** = コマンドの出力を見た値、**読み** = 逆アセンブル・生成 C・文書を読んだ結果で実行して確かめていないもの、**未確認** = 確かめていないこと。census の数（ok／stop）は **A4 の進捗ではない**（決定 0046、0060、0097、0101）。

基準: HEAD 5002d98 + census.diff（決定 0097）。ワークツリー: `lift/out-wallL/`（`base/` = `git archive HEAD` + census.diff、`src/` = 編集する複製）。FLEngine の生成 C の旧 C の SHA-256: `feb5b9087f8d7a8a1282900212d7bd1f5b6071b0326e7149642f98c168e8080f`（base の release `fe-lift lift … --pdata` で再生成して一致を確認。実測）。

測定の道具（標準ライブラリと衝突しない名前。`lift/out-wallL/py/`）: `pecommon.py`（自作の PE 読み。`.pdata` は pefile でなく 12 バイトずつ全件読む。pefile は決定 0074 の調査で 29,274 件で止まった）、`survey1〜6.py`（隙間と形の走査。capstone 5.0.7）、`callers.py`（`e8`/`e9` の rel32 の全走査）、`disasm_va.py`、`cmpc.py`（生成 C を関数単位で比べる）、`anchor_check.py`。Rust の実装（iced-x86）とは別系統で、数が一致することを確かめた（§1.4）。

## 1. 壁 1: unit 296 `indirect call to 0x477e73 (not a known function entry)`

### 1.1 事実

- 出典: census（`lift/out-census/docs-new/wall-census-2.md` の 13 行目）。スタック f_534b00 ← f_534eb0 ← f_4fa2a0、ret 0x534edf。
- **実測**: 0x477e73 のバイト列は `48 83 c1 f0 48 8b 01 ff 60 10`（10 バイト）= `add rcx,-0x10; mov rax,[rcx]; jmp qword ptr [rax+0x10]`。`.pdata` の開始でも、どの RUNTIME_FUNCTION（57,816 件。連鎖 0）の範囲の内側でもない。直前の 0x477e68 は `cc`。DIR64 のスロット 0x477ee0 が値として持つ（スロットは `.text` の中）。
- **読み**: 読み取った命令列は、多重継承で副基底の仮想関数を呼ぶときの thunk の形（this を直し、vtable ポインタを読み、スロットへ jmp）。前後の 0x477e7d 以降は決定 0051 の `add rcx,-0x10; jmp rel32` が並ぶ（同じ表の仲間）。コンパイラがこの形を実際にそう使っているかは確認していない。

### 1.2 母集団の数え方（実測。`py/survey*.py`。決定 0051 の数と一致）

| 段 | 定義 | 個数 |
|---|---|---:|
| DIR64 スロット | `.reloc` の IMAGE_REL_BASED_DIR64 | 278,531 |
| 値が実行範囲内（`.text` 0x401000〜0x13a4a10 のファイル上のバイトがある範囲）| distinct | 80,188 |
| うち `.pdata` の開始 | すでに seed（決定 0031） | 24,592 |
| うち RUNTIME_FUNCTION の範囲の内側（開始でない） | 関数の途中 | 10 |
| **範囲に入らない（`.pdata` に無い）** | 決定 0074 が見送った母集団 | **55,586** |
| うち命令境界 | `.pdata` の隙間（.text の 55,666 か所、計 5,843,778 B）を、隙間の先頭から int3 で区切りながら線形走査（capstone）して、走査の命令の先頭に当たるもの。値を含む隙間は 1,269 | 1,825 |
| うち終端（`ret`/`jmp`/`hlt`/`ud2`/`retf`）まで不正バイトなしで読める（12 命令以内、隙間の内側） | 他: 12 命令で終わらない 489、不正バイト 436、隙間の終わりに届く 19 | 881 |

881 個の形（実測）:

| 形 | 個数 | 扱い |
|---|---:|---|
| `add reg,imm; jmp rel32`（決定 0051 の thunk） | 602 | 決定 0051 で seed |
| `cwde; ret 0x15e` | 115 | データの偶然（0051 が除外した 4 バイトの値の並び） |
| **`add rcx,-N; mov rax,[rcx]; jmp [rax+d]`** | **82** | **本決定。壁 1 の 0x477e73 を含む** |
| `push rdx; lea rdx,[rip+d]; jmp rel32` | 57 | `.didata` の遅延 import の stub と読める（0051 §2.3）。種にしない |
| 裸の `jmp [rip+d]`（IAT の thunk） | 5 | 別の扱い（`imports_table`）。種にしない |
| その他（`retf`、`hlt`、`ud2`、`ret imm16` など） | 20 | データの偶然と読める。種にしない |

境界の条件を付けずに、範囲に入らない 55,586 個すべてを先頭 4 命令まで読んで `jmp` で終わるものを数えると: `add+jmp`（直接）623（602 + 境界を満たさない 21）、**`add+mov+jmp [mem]` 82（同じ 82 個）**、`push+lea+jmp` 57、裸の直接 `jmp` 40、`jmp [rip+d]` 1、残りは `add+iretd+jmp` のようなデータの誤読。裸の直接 `jmp` 40 のうち、飛び先が `.pdata` 開始なのは 3 個（0x602af0、0x602b00、0x602b10。すべて `jmp 0x40c820`。前後が `cc`）で、残りはデータ（直前が ASCII の `xedn` など）。**この 3 個は決定 0051 が「裸の jmp は import や転送の thunk」として除外した形で、本決定も種にしない**（実行で使われた証拠が無い）。

### 1.3 82 個の内訳（実測。`logs/vcall82.txt`、`data/vcall-fle.txt`）

- 形はすべて `add rcx,-N; mov rax,[rcx]; jmp qword ptr [rax+d]`。レジスタは全部 rcx と rax。N は 0x10〜0x558（distinct 25 通り。最頻 0x40 が 14、0x38 が 13、0x28 が 11）、d は 0〜0x2a0。長さ 9 バイト 7、10 バイト 47、13 バイト 25、16 バイト 3。
- 各 thunk を指すスロットは 1 つずつ（vtable）。スロットは `.text` の中。
- 命令境界の証拠: 82 個のうち 12 個は直前が `cc`、69 個は同形の thunk（0051 の形を含む）が直前でちょうど終わる鎖、1 個（0xbed3ac）は同じ鎖が 64 段を超える長さ（Python の確認では再帰の上限に掛かった。Rust の上限は 4,096）。

### 1.4 独立した再現（実測）

- Rust の `fe-decode vcall-thunks FILE`（`vcall_thunks`）の出力 82 個と、Python の走査（境界の条件なし、先頭 3 命令が `add; mov; jmp [mem]`）の 82 個の**アドレス集合が一致**（`diff` で差なし）。

### 1.5 guest が実際に使うもの（実測）

- 82 個の入口に gdb のブレークポイント（無視回数で計数。値は書かない）を置き、census を全 942 unit で走らせた（dsp 構成）: **f_477e73 が 12 回**、**他の 81 個は 0 回**。unit 296 が停止していた壁はこの 1 個のためだった。census は 18 unit を放棄するので、81 個のうちに放棄された unit の中で使われるものがあるかは**未確認**。

### 1.6 安全な条件（規則 V）と根拠

| 条件 | 根拠 |
|---|---|
| DIR64 のスロットの値で、実行範囲内、`.pdata` の開始でも範囲の内側でもない | 決定 0051 と同じ母集団。値の無条件の種は FLEngine で `bad_nodes` 0 → 113,485（決定 0074 の調査 §7.1）なので、形で絞る |
| 形: 0〜2 個の `add`/`sub` r32/r64, imm、`mov r64,[base+disp]`、`jmp qword ptr [r64+disp]`（`mov` の書き先が jmp の base）で終わる、これだけ、16 バイト以下 | データの偶然が「参照されるスロットを持つ」と「この 3 命令の全部が一致」を同時に満たすのは考えにくい（読み。確率は測っていない）。形の否定側は `fe-decode/tests/vcall_setjmp.rs` の 14 通り（jmp の base が違う、index、32 ビット、call が混じる、など） |
| 開始が命令境界である証拠（直前が int3、`.pdata` の終端、または同形 thunk の終端で、それ自身が証拠を持つ） | REX 接頭辞を欠いて読むと同じ形に読める位置（0x477e74）が実在する（`add ecx,-0x10; mov rax,[rcx]; jmp [rax+0x10]`）。決定 0051 は DIR64 の参照で境界を保証したが、ここでは参照も別の証拠で補う |
| ガード: 他の seed（.pdata 開始、0051 の thunk、`--seeds`）の閉包の命令に、その値が入っていない | `fe-lift` の `traverse` は「自分以外の関数入口に当たったら tail call」にするので、既存の関数が流れ込む場所を入口にすると既存の関数の本体が変わる。FLEngine では 82 個すべてがガードを通る（除外 0）。thunk の本体は直接の分岐を持たないので、新しい関数は新しい入口を作らない |

**関数の形**: 3 命令の葉。`call` を持たず、rsp を変えない。unwind 情報が無いことは Microsoft Learn「x64 exception handling」（決定 0074 の調査 §3.2 が引いた）のリーフ関数の定義に合う（読み）。runtime は `.pdata` を読まない（`runtime/` の参照 0。実測）ので、unwind 情報の無い関数が入口の表に入っても仮定に反しない。

### 1.7 副: `jmp qword ptr [mem]` が未対応だった（実測）

thunk の本体 `jmp [rax+d]` は `fe-lift` で `Jmp: indirect target unresolved`（`tailjmp::lift` はレジスタ形だけ。決定 0036 W1 が除外）だった。seed だけを足すと 82 個がそれぞれ `fle_unsupported` で止まる（`unsupported_total` 525 → 607、実測）。`jmp [mem]` を tail dispatch にすると 524 に下がり（残りは IAT の thunk 524）、変わる既存関数は `f_4126f0`（壁 2 の longjmp 側）だけ。決定 0100 の決定 3。DLL の lift への影響は同じ変更の結果（Blend2D −12、dsp −183、QFC −2 の `Jmp: indirect target unresolved`）。

## 2. 壁 2: 終了フェーズの `jmp qword ptr [rcx]`（0x412752）

### 2.1 f_4126f0 は何か（実測）

- 0x4126f0 は `.pdata` の開始（範囲 0x4126f0〜0x412755）。バイト列は `92`（`xchg edx,eax`）で始まる。逆アセンブル（capstone）:
  - `xchg edx,eax`（eax = 値 v）、`lea rdx,[rcx+0x70]` から `movapd xmm6〜xmm15,[rdx-0x20…+0x70]`（rcx+0x50〜0xe0）、`mov rbx/rbp/rsi/rdi/r12〜r15,[rcx+0x10…0x48]`、`mov rsp,[rcx+8]`、**`jmp [rcx]`（0x412752）**。
  - 直前の f_412680（0x412680〜0x4126f0）が対の保存側: `mov rax,[rsp]; lea rdx,[rsp+8]; mov [rcx],rax; mov [rcx+8],rdx; mov [rcx+0x10…0x48],rbx/rbp/rsi/rdi/r12〜r15; lea rax,[rcx+0x70]; movapd [rax-0x20…+0x70],xmm6〜15; xor eax,eax; ret`。
- つまり f_412680 = `setjmp(buf)`、f_4126f0 = `longjmp(buf, v)`（標準の setjmp/longjmp の形。Win64 の非揮発レジスタ）。f_412760 = `lea rcx,[rcx+0x38]; jmp f_412680`（context の jmp_buf への入口）、f_412770 = `longjmp` を呼ぶ側（下）。
- **読み（未確認）**: 全体は Delphi の RTL の始動・終了コード（export 名 `dbkFCallWrapperAddr`、`__dbk_fcall_wrapper`、セクション名 `.didata` から）。context は 296 バイト（0x25 qword）で、jmp_buf が context+0x38 にある。

### 2.2 jmp_buf のレイアウト

| 位置 | 内容（実測: f_412680 の保存側と f_4126f0 の復元側の命令） |
|---|---|
| +0x00 | 保存される RIP = `setjmp` の戻りアドレス（`[rsp]`） |
| +0x08 | 保存される RSP = `setjmp` から戻った後の rsp（呼び出し時の rsp + 8） |
| +0x10〜0x48 | rbx, rbp, rsi, rdi, r12, r13, r14, r15 |
| +0x50〜0xe0 | xmm6〜xmm15（16 バイト整列の `movapd`） |
| 計 | 0xf0 = 240 バイト（f_412770 の `rep movsq` ×0x1e） |

### 2.3 保存される RIP は何を指すか（実測）

- 実行時に gdb で読んだ値（census、dsp 構成）: `f_412760` が 1 回呼ばれ（rcx = 0x7ffff7bffe78、guest-return **0x1376840**）、`f_412680` が rcx = +0x38、guest-return 0x1376840。longjmp 側: `f_412770` が rcx = 0x15eac80（グローバルの context）で呼ばれ（guest-return 0x4129ed = f_4128b0 の `call` 0x4129e8 の次）、ctx+0x38 の jmp_buf は **saved rip 0x1376840、saved rsp 0x7ffff7bffe30**。`f_4126f0` が edx = 1 で呼ばれる。
- 0x1376840 は entry `f_1376820` の `call 0x412760`（0x137683b）の**次の命令**（`cmp eax,1`）で、関数の途中。生成 C では f_1376820 の C フレームの中のラベル `L_1376840` に当たる。
- 修正前の census の停止を `fle_dispatch(fle_ld(R[1],8))` 化（決定 3）した直後は `fle: indirect call to 0x1376840 is not a known function entry` で、この値が実測で確認できた。

### 2.4 呼び元と対の setjmp 側の数（実測。`py/callers.py` の `e8`/`e9` の全走査 + DIR64 の値の走査）

| 関数 | 直接の呼び元 | 備考 |
|---|---:|---|
| f_412680（setjmp） | 1（f_412760 の `jmp`） | データの参照 0 |
| f_412760（setjmp の入口） | 1（`f_1376820` の 0x137683b） | データの参照 0 |
| f_4126f0（longjmp） | 1（f_412770 の 0x4127bf） | データの参照 0 |
| f_412770 | 2（f_4128b0 の 0x4128ff と 0x4129e8） | 実行で通るのは 0x4129e8 側（[0x15eacb0] ≠ 0 のとき） |
| f_4128b0 | 5（f_412160 の 0x41229c、f_412a40 の 0x412a4a、f_868c00 の 0x8691a9、f_1262880 の 0x1262884、**entry の 0x137687f**） | f_412a40 は `ExitCode` を保存してから呼ぶ形（読み。Halt 風） |

setjmp の形の関数は FLEngine に 2 つ（f_412680 と入口 f_412760）、呼び出しは 1 か所。QFC に 1 か所（決定 0100 の検証。QFC は実行していない）。

### 2.5 entry が戻るまでの順序（実測と読み）

`f_1376820`: `call f_412760`（jmp_buf = ローカル rbp+0x80）→ `cmp eax,1; setle cl; mov [rbp+0x17c],ecx` → `test eax,eax; jne 0x1376884` → `call f_41d8f0`（Init。InitUnits を含む）→ **無条件で `call f_4128b0`**（0x137687f）→ `mov eax,[rbp+0x17c]; lea rsp,[rbp+0x180]; pop rbp; ret`。`f_4128b0` は `[0x15eacb0] ≠ 0` のとき `lea rcx,[0x15eac80]; call f_412770`（0x4129e8）→ context の jmp_buf を局所にコピーして `longjmp(コピー, edx)`。著者の読みでは Delphi の `Halt`／終了の経路。**事実として、entry が `returned from entry` で戻るには `f_4128b0` を通り、census の実行ではその longjmp を必ず通る**（0046／0101 の A4 に含まれるかは判断しない）。longjmp の値は 1 なので、着地した entry は `eax = 1`（`setle cl` = 1）で戻る（実測: 修正後の census の終わりは `returned from entry`、rax = 0x1）。

### 2.6 非局所ジャンプを生成 C でどう表せるか（選択肢）

前提: longjmp の `jmp [rcx]` の飛び先 L = 0x1376840 は entry の途中。C の呼び出し鎖は entry → … → f_4128b0 → f_412770 → f_4126f0 で、**entry の C フレームは生きている**（longjmp は祖先の関数の途中への復帰）。ゲストの状態はすべて大域（`R[]`、`FL`、`X[]`）とゲストのメモリにあり、C の局所変数にゲスト状態は無い（実測: 生成 C に命令をまたぐ局所は無い）。

| 案 | 内容 | 制約 | 実装量 | 正しさ・結果 |
|---|---|---|---|---|
| **A（採用）** | landing L を関数入口（seed）にする。呼び出し元の関数は setjmp の呼び出しの後で f_L へ tail call するが、その tail call を「登録して `_setjmp`、0 なら f_L()、非 0 でも f_L()（もう一度）、戻ったら登録を外して return」にする。`fle_dispatch(va)` は表を引く前に、登録の (va, rsp) が一致するものに `_longjmp` する | landing を持つ関数の本体が landing で切れる（FLEngine で entry の 1 関数）。setjmp の形の認識が要る。捨てたフレームの副作用（トレースの深さ、`fle_call_guest` の入れ子カウンタ、ホストのロック）は戻らない。wasm は setjmp/longjmp のフラグが要る | 実装 約 510 行（Rust 約 440、C 約 76。決定 0100 の「実装した場所」）、テスト 約 970 行 | **native で、生成 C とホスト CPU の実行が一致**（`setjmp_landing.rs`）。実 DLL で entry が戻る |
| B | 関数の先頭に「再開 VA」の `switch` を足し、landing のラベルへ `goto`。C のフレームの巻き戻しは別に要る | landing を持つ関数ごとに本体が変わる。結局 C の巻き戻しの機構も要る（B 単独では C スタックが深いまま） | A より大きい | 不採用 |
| C | すべての `call` の後で「巻き戻し中」フラグを検査して `return` を繰り返す | 既存の全関数（FLEngine で 226,186 の `call`）の本体が変わる。実行も遅くなる | 大 | 不採用（「既存の関数の本体を変えない」に反する） |
| D | landing を関数入口にして、`fle_dispatch(L)` が f_L を**入れ子で**呼ぶだけ | なし（実装は最小） | 最小 | **誤り**: f_L が捨てたはずのフレームの上で走り、f_L が戻った後で f_4126f0 以降の残りの C コードが走る（ゲストでは起きない）。不採用 |
| E | ホスト側（`fle_call_guest` の外）の setjmp で最外のフレームだけ | 目的のフレームが最外のときだけ正しい | 小 | FLEngine の entry は最外だが、Delphi の `Halt` は初期化の深い所からも longjmp する（f_4128b0 の呼び元 5 か所）ので、一般には誤る。不採用 |
| F | 終了フェーズを「プログラムの終わり」とみなして entry を戻したことにする | 値と意味（eax = 1）を推測で決める | 最小 | 不採用 |

### 2.7 採用案の検証（実測）

- 合成 PE（x86 の機械語を `clang -c` で組んだ 10 関数）の 4 プログラムを、生成 C とホスト CPU（同じバイト列を実行可能ページで実行）で動かし、出力（rax、マーク 10 個、rsp の一致）が**一致**。値は手で導いた期待と一致: main = `rax=7 marks=1,0,7,1111,0,0,1,0,1,0 rsp_eq=1`（`[1]`、`[7]`、`[9]` が 0 = 捨てたフレームの後続が走らない。`[3]` = 0x1111 = 復元された rbx）、main2（longjmp なし）= `rax=0`、main3（trampoline 経由・同じフレームへ 2 度着地）= `rax=9 marks=1,0,2,9,…`。
- 変異検査: `fle_landing_try` の呼び出しを外す／rsp の一致の検査を外す／ラッパーを出さない — それぞれテストが落ちる（4 件／1 件／5 件失敗）。
- 実 DLL: census（dsp 構成・単独構成）の終わりが `unsupported instruction at 0x412752`（修正前）→ `returned from entry`、rax = 0x1、終了コード 0。

## 3. 未確認・限界

- 壁 1 の 82 個のうち使われたのは 1 個。残りの実行は未確認。census が放棄した 18 unit の中の使用は未確認。
- 壁 2 は native のみ。wasm（emcc）は使っていない。`FLE_LANDING_NOJUMP`（`__EMSCRIPTEN__` の既定）の停止は native の `-DFLE_LANDING_NOJUMP` で確認しただけ。
- Delphi の RTL であることは export 名と形からの読み。jmp_buf の xmm の部分（+0x50〜）の使い方は実行で確認していない（復元側は `movapd` の実行を伴うが、値の比較はしていない）。
- 捨てたフレームの副作用: トレースの深さ（`call depth at stop: 4` が戻った後も残る。診断の表示のみ）、`fle_call_guest` の入れ子、ホストのロック。
- 実機 Windows との照合は無い。

## 4. 作ったファイル

`lift/out-wallL/`: `base/`、`src/`、`patches/wallL.diff`、`docs-new/`（本書、`0100-wall-lift-gaps.md`、`README-0100-line.txt`）、`py/`、`asm/`（合成テストの組み立てソース `sj4.s`）、`gen/`（生成 C。大きい）、`two-s2/`、`two-s3/`、`two-s4/`、`single-s3/`、`single-s4/`（ビルドと実行用）、`runs/`、`logs/`、`dllstat/`、`gdb/`、`tmp/`。
