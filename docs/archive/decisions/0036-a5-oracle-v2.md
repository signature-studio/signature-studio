# 0036: A5 オラクル v2（ポインタ入力、EFLAGS/MXCSR/x87 の比較、実行バイトのネガコン、規模拡大）

状態: 作業者が決定。手順と結果は `docs/investigations/a5-oracle-run2.md`、不一致は `docs/investigations/a5-oracle-mismatches.md`（追記）。0035 を置き換えず、差分だけをここに書く（0035 の判定基準、選別、ドライバの骨格は継続）。

実装は `lift/harness/oracle/` の新規ファイルのみ（`oracle2_common.{h,c}`、`ms_call2.S`、`native_drv2.c`、`lift_drv2.c`、`argptr.py`、`exclude_stats.py`、`run_diff2.py`、`build2.sh`、`run2.sh`）。v1 のファイルは変更していない。lifter、runtime、`imports_table.rs` は変更していない。

## 1. 入力: ポインタ引数

- `argptr.py` が、純計算 9,901 関数の逆アセンブル（capstone）を前方一方向に走査し、引数 4 つ（RCX/RDX/R8/R9）とスタック引数 4 つ（`[rsp+0x28]` 以降）が「ポインタとして逆参照されるか」を判定する。
  - レジスタごとに由来のタグを持つ: 引数そのもの（mov/lea/add/sub/and/or/cmov で伝播）、「引数が指す構造体から読んだ値」（二重間接）、「引数の 32 ビット整数値（添字）」。rsp/rbp 相対のスタックスロットの保存/読み戻しも追う（rbp フレームは `mov rbp, rsp` / `lea rbp, [rsp+d]` から追う）。
  - `[reg+...]` のベースレジスタがタグ付きならポインタ、添字レジスタにタグが付いていれば整数（小さい値を渡す）。`rep movs/stos`、直接 call/jmp 先への引数の受け渡し（呼び先のポインタ位置を呼び元へ伝播、不動点）も判定に使う。
  - 前方分岐の状態は和集合で合流。後方分岐（ループ）は辿らない。これは近似で、取りこぼしも過検出もある（取りこぼしは native_fault の原因別内訳に現れる）。
- ポインタとして使われる引数には scratch（0x20000000、1 MiB）内の 16 バイト整列のアドレス（0x20020000 以降の 256 KiB 内）を渡す。native と lift は同じ入力ファイルと同じ scratch 内容（同じ seed から生成）を使う。
- 二重間接: 「引数が指す構造体から読んだ値」がポインタとして使われる関数は、scratch の fill モード 5（8 語中 6 語が scratch 内の 16 バイト整列ポインタ、2 語が 0..32 の整数）を多用する。`[arg+d]` が二重間接に使われる位置は、そのアドレス（引数+d）に有効なポインタを case ごとに直接書き込む（`pokes`、最大 24 個、scratch 内の 8 バイト書き込み。両側で fill の後に適用）。添字として使われる位置は 0..16 の整数を書く。
- 整数引数: 小さい値（0..64）、境界値、32 ビット乱数、64 ビット乱数、0。他の GPR は乱数（case 14/15 は一部のレジスタに scratch 内のポインタ）。
- 各 case に `ptr_input`（関数が 1 個以上のポインタ引数を持つ）/ `no_ptr` を付け、別集計する。

## 2. 比較対象の拡張

| 状態 | 取得 | 比較 |
|---|---|---|
| EFLAGS 状態フラグ CF/PF/AF/ZF/SF/OF | native: 戻り直後に `pushfq`（`ms_call2.S`）。lift: runtime の `FL`（6 バイト）。入力 EFLAGS も両側に同じ値を与える（case ごとに乱数、DF=0） | lift の `FL` が `FLE_UNDEF` のフラグは「未定義」としてカウントだけ（比較しない）。それ以外は比較（「定義」）。件数は別々に報告 |
| DF | native/lift とも入力 0。出力は比較に含めた（lift は `fle_df`） | 比較する |
| MXCSR | native: `stmxcsr`。lift: `fle_mxcsr`。入力は両側 0x1F80（native は `ldmxcsr` で設定） | 制御ビット（15:6）と、状態ビット（5:0、IE DE ZE OE UE PE）を別に判定（`mxcsr_ctrl`、`mxcsr_sticky`） |
| x87 制御/状態/タグ | native: `fninit` の後、戻りで `fnstenv`（cw、sw、tw）。lift: `fle_x87.cw/sw/tw` | cw、sw（TOP 含む）は完全一致。tw は「空のレジスタの集合」で比較（決定 0028 の判断 2。lift の tw は valid を 0000 で持ち、native の fnstenv は内容から再計算した値になるため）。生の tw の差は参考値として数える |
| MMX | x87 とエイリアスしない設計（0028）なので、MM[] の値は比較しない。x87 の tw/sw のみ | 純計算プールで MMX 命令を持つ関数は 4 件（実測）。選ばれたのは 1 件 |

`FL` と未定義の扱い: SDM で未定義のフラグを比較から外す。除外は命令単位ではなく「lift が `FLE_UNDEF` を持つフラグ」で決めた（lifter が命令ごとの未定義フラグを `FLE_UNDEF` にしているので、その表が除外の根拠になる）。除外した件数は、フラグごとに「未定義（lift）」「そのとき native が 1 だった件数」で報告する。未定義とされたフラグを関数戻り後に別の関数が読む場合、lift は abort する（`fle_rd`）ので、結果が未定義フラグに依存することはない。

lift 側の口: `FL`（`fle_flags.h`）、`fle_df`（`fle_string.c`）、`fle_mxcsr`（`fle_mxcsr.c`）、`fle_x87`（`fle_x87ctl.c`）はグローバル変数としてリンクできた。runtime の変更は不要だった。

## 3. 乖離の扱い（MXCSR の状態ビット）

native 側は SSE 演算が MXCSR の状態ビット（IE DE ZE OE UE PE）を立てる。lift の runtime は `fle_mxcsr` を stmxcsr/ldmxcsr 以外で更新しない（`fle_sse.h` に `fle_mxcsr` の参照がない。決定 0005 の「仕様の外」に既記）。このため、浮動小数点演算を持つ関数は、浮動小数点の結果が bit-exact でも MXCSR の状態ビットで必ず差が出る。これは既知の未実装（lifter の設計外）であって、命令ハンドラの誤りではない。

判断: 状態ビットの差は別カテゴリ `mismatch_aux`（フィールド `mxcsr_sticky`）にし、`mismatch_core`（GPR、XMM、メモリ）とは混ぜない。`mismatch_aux` は bit_exact に数えない。比較から外さない（除外ではなく別集計）。ネガコンの検出判定では、`mxcsr_sticky` だけの差は「検出」に数えない（native のバイト反転で、元々 lift と常に違う状態ビットの差が出るので、検出の証拠にならない）。

## 4. native_fault の原因分類

fault した case ごとに、native の fault 命令アドレス、シグナル、`si_code`、fault 時のレジスタを記録し（`oracle2_common.c`）、`run_diff2.py` が命令をデコードして次のどれかに分類する: `null_deref`（実効アドレス < 0x10000）、`wild_pointer`、`gp_noncanonical_address`（`si_code`=0x80 で非正規アドレス）、`scratch_range_exceeded`（scratch 付近で範囲外）、`stack_range_exceeded`、`sigfpe(divide)`、`timeout(loop)`、`write_to_readonly_image`、`gp_other`。ベースレジスタの出所（引数と同じ値か、入力構造体から読んだ値か、関数内で計算/ロードした値か）も記録する（後方 40 命令の走査による推定）。

## 5. ネガティブコントロール

- 反転対象: native の単一ステップ実行（`ORACLE_TRACE`。`TF` を立てて `SIGTRAP` で rip を記録）が実行した命令のバイトのうち、opcode、displacement、immediate のバイト（capstone の `modrm_offset`/`disp_offset`/`imm_offset` による）。接頭辞（REX を含む）と ModRM/SIB のバイトは候補に入れていない（依頼が opcode/immediate/displacement に限っているため。「opcode」は接頭辞・REX の後から ModRM の手前までのバイト）。
- 元の関数が bit_exact だった case のトレースから、実行された命令を集め、そこから無作為に 1 バイトを選ぶ。native のみ `ORACLE_PATCH=va:01` でメモリ上の 1 バイトを反転して走らせる（PE のコピーは作らない）。lift は元のまま。
- ケースは関数の bit_exact だった case だけ（native が fault しない入力）。
- 判定（優先順）: `detected_core`（GPR/XMM/メモリが lift と違う）、`detected_aux_only`（EFLAGS/MXCSR 制御/x87 だけ違う。`mxcsr_sticky` は除く）、`detected_by_native_fault_only`（反転後に native が fault/タイムアウトで戻らない。出力の差ではなく、挙動の変化の検出）、`undetected_state_changed_but_masked`（実行された命令列のハッシュ（rip、GPR、XMM、状態フラグの各ステップ）が変わったが、最終出力は変わらない）、`undetected_equivalent_no_state_change`（命令列のハッシュも同じ。そのバイトを変えても各ステップで観測できる状態が全く変わらない。等価な変異か、トレースの観測範囲外（メモリ値、ステップ間のメモリ書き込み値）の変化）。
- 3 件の定数返却関数（`mov eax, imm32; ret`）の imm は、健全性の確認として別に反転して走らせた（検出されなければならない）。

## 6. 規模、タイムアウト

- 1000 関数 × 16 case = 16,000 case。選別は v1 と同じ `Random(seed).sample(200)` を最初の 200 とし、残り 800 を残りの母集団から無作為に追加した。v1 と同じ 200 関数の結果は別集計（ただし case の生成は v2 の方法なので、v1 の case とは別）。
- 並列 16 の下で 2 秒のアラームが誤って発火することがあるので、タイムアウトした case は両側を 8 並列、10 秒で再実行する（結果は再実行後を使う）。
- 純計算以外の除外の分類（IAT 呼び出し、間接 call/jmp、x87、GS）は `exclude_stats.py` が、各関数の自身の理由と、呼び出し先を経由した理由を集計する（`select_pure.py` は最初の理由だけだった）。

## 7. 限界（事実）

- ポインタ判定は静的な近似。入力構造体の内容は関数ごとの型を知らないので、ランダムなポインタ/整数。構造体を深く辿る関数（3 段以上）、ベクタ長や文字列長を引数が決める関数は、依然として fault する。
- native の単一ステップの `trace` は、rip と GPR/XMM/状態フラグの各ステップのハッシュ。メモリの値は含まない。
- MXCSR の状態ビットは lift が更新しない（上記）。x87 の演算は lift していない（選別で除外）ので、x87 の状態の検査は「x87 命令を持たない関数で不変」の確認だけ。
- 比較していないもの: GS/TEB、MM レジスタの値、x87 の 80 ビットレジスタの内容、MXCSR の丸め制御を変えた状態。
