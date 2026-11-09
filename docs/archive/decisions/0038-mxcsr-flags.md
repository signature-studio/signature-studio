# 0038: SSE 浮動小数演算の MXCSR 状態ビット、RC/FTZ/DAZ、PREFETCHh

状態: 作業者が決定。実行記録は `docs/investigations/mxcsr-flags-run.md`。A5 オラクル v2（0036）が見つけた lift 側の未実装 2 件（MXCSR 状態ビット 717 case / 62 関数、`Prefetcht0` 13 case）を埋める。0036 の「3. 乖離の扱い」はこの決定で解消した（run5 で `mismatch_aux` 0）。

## 1. 方式

SSE/SSE2/SSE4.1 の浮動小数命令は、命令そのものをホスト CPU で実行する（`lift/runtime/fle_mxcsr_flags.h`、ヘッダのみ、インライン asm）。命令の前後で次を行う。

1. `fle_mx_begin`: ホストの MXCSR を `(fle_mxcsr & 0xE040) | 0x1F80` にする。0xE040 は RC（bit 14:13）、FTZ（bit 15）、DAZ（bit 6）。つまり丸め・FTZ・DAZ はゲストの値、例外マスクは全マスク、状態ビットは 0。
2. 命令を実行する（`addsd %1, %0` など）。
3. `fle_mx_end`: `stmxcsr` で状態ビット（bit 5:0）を読み、`fle_mxcsr` に OR する（状態ビットは sticky。SDM Vol.1 MXCSR）。ゲストの制御ビットが既定値以外だったときだけ、ホストの MXCSR を 0x1F80 に戻す。

結果のビット列、NaN の選択、QNaN 化、非正規化数、FTZ/DAZ の扱い、RC は、すべてハードウェアのものになる（再実装していない）。NaN 規則を C で書いていた旧ヘルパ（`fle_addss` 等）は、これらの命令には使われなくなった（`sse_more.rs` が `sse_scalar.rs` より先に呼ばれる。旧ヘルパと旧ハンドラはコードに残っている。削除していない）。

ゲストの MXCSR で、立った状態ビットに対応するマスクビット（bit 7+n）が 0（例外が非マスク）のとき、実機なら #XM（または #UD）になる。この場合は `fle: SSE floating-point exception unmasked in guest MXCSR (raised flags ..., MXCSR ...); #XM is not modelled` で abort する（名前付き abort）。MXCSR の状態は、ホストの命令を実行した後にしか分からないので、abort は命令の実行後に起きる（abort 時点で XMM の書き込みはまだ行っていない: ハンドラは `X[d] = f(...)` の形で、`f` の中で abort する）。

## 2. 対象命令（実装した 60 ニーモニック）

| 区分 | ニーモニック |
|---|---|
| 四則・min・max・sqrt（各 ss/sd/ps/pd、28） | add sub mul div min max sqrt |
| 比較（ss/sd/ps/pd、4。述語 imm8 0..7） | cmpss cmpsd cmpps cmppd |
| COMIS/UCOMIS（4） | comiss comisd ucomiss ucomisd（ZF/PF/CF を設定、OF/SF/AF=0） |
| 丸め（4。imm8 0..255） | roundss roundsd roundps roundpd |
| 浮動小数どうしの変換（10） | cvtss2sd cvtsd2ss cvtps2pd cvtpd2ps cvtdq2ps cvtdq2pd cvtps2dq cvttps2dq cvtpd2dq cvttpd2dq |
| 整数との変換（6、32/64 ビット） | cvtsi2ss cvtsi2sd cvtss2si cvtsd2si cvttss2si cvttsd2si |
| PREFETCHh（4） | prefetcht0 prefetcht1 prefetcht2 prefetchnta |

メモリオペランド: スカラーは 4/8 バイトの読み出し、パックは 16 バイト整列の読み出し（`fle_ldxa`、不整列は従来どおり abort）。レジスタ形・メモリ形の両方。

実装しなかったもの（事実）: RCPPS/RSQRTPS（SDM の SIMD 例外は「None」で、結果は実装依存の近似値。CPU が違えば値が違うので突合の対象にしていない。閉包に Rcpps 8 件が残る）、MMX 形（cvtpi2ps 等）、HADDPS/DPPS、VEX 形の浮動小数演算（閉包に vcmpeqps 2、vsqrtsd 1、vminps 1、vmaxsd 1 など、`vex.rs` の範囲外のものがある）。

SDM の例外節との対応（Vol.2 各命令の「SIMD Floating-Point Exceptions」。テキスト `lift/out-int2/spec/sdm-vol2.txt` から抽出）: add/sub/mul は Overflow, Underflow, Invalid, Precision, Denormal。div はこれに Divide-by-Zero。sqrt は Invalid, Precision, Denormal。min/max は Invalid（QNaN オペランドを含む）, Denormal。cmp は SNaN で Invalid、述語によっては QNaN でも Invalid、Denormal。comis は SNaN/QNaN で Invalid、ucomis は SNaN のみ、Denormal。cvtdq2ps は Precision、cvtsi2ss/sd は Precision（cvtsi2sd の 32 ビットは正確なので実際には立たない）。cvtps2dq/cvttps2dq/cvt[t]sd2si/ss2si は Invalid, Precision。cvtss2sd は Invalid, Denormal。cvtsd2ss は Overflow, Underflow, Invalid, Precision, Denormal。round は Invalid（SNaN のみ）, Precision（imm8 bit3 = 0 のときのみ）で Denormal は立てない。実際のフラグはホストが決めるので、この表はテストの期待値（`flags_match_sdm_expectations`、61 件）の根拠として使った。

## 3. RC / FTZ / DAZ の扱い（静的確認の結果と決定）

FLEngine_x64.dll の逆アセンブル（`lift/objdump-text.txt`）で、MXCSR を書く命令は `ldmxcsr` が 6 か所、`fxrstor`/`xrstor`/`vldmxcsr` は 0 件（`fxsave` は 2 件だがデータ領域の中で、命令ではない。`stmxcsr` は 3 か所）。インポート名にも `_controlfp`、`_control87`、`_fpreset`、`fesetround`、`SetThreadContext` などは無い（DLL 全体の文字列検索）。

| ldmxcsr の場所 | 関数 | 読み込む値 |
|---|---|---|
| 0x40a36c | 0x40a360 | `ecx & 0xFFC0` を 0x13a5054 に保存してから 0x13a5054 を読み込む |
| 0x40f530 | 0x40f530 | 0x13a5054 |
| 0x40a39d | 0x40a380 | 現在の MXCSR の bit 15:6 を保ち、bit 5:0 を引数の下位 6 ビットにする（状態ビットだけを書く） |
| 0x40eeab | 0x40eea0 | 引数 `ecx`（呼び元 0x40f139 は `(現在値 & 0xFFFFF7FF) | (保存値 & 0x800)`。UM だけを変える） |
| 0x475c8a | 0x475c60 | RC（bit 14:13）だけを `cl & 3` に変える。呼び元 3 か所（0x475ccb は `ecx=0`、0x475d43 と 0x475d7d は保存した旧 RC）。戻り値は旧 RC |
| 0x63a6d0 | 0x63a6d0 | 0x13e4d50（65 か所から call） |

データの初期値（PE の `.data` から読んだ値）: 0x13a5054 = 0x1F80、0x13ab3e0 = 0x1F80、**0x13e4d50 = 0x9F80**。0x9F80 は FTZ（bit 15）が立っている。0x63ab2b は `or [0x13e4d50], 0x40`（DAZ を立てる。0x63aaf9 以降の条件分岐の先）、0x63ab34 は `mov [0x13e4d50], 0xFFFFFFFF`（予約ビットが立つので、`ldmxcsr` が #GP になる値。lift の `fle_mxcsr_load` は予約ビットで abort する）。0x40a2c0 の呼び元は `ecx=edx=0x1F80`（全マスク）と `ecx=edx=0x800`（UM）で、マスクを「立てる」呼び出し。

静的に言えること（事実）: **FTZ は実行時に既定値（0）以外になりうる**（0x63a6d0 が FTZ 付きの 0x9F80 を読み込む。0x63a6d0 が実行される経路かどうかは今回確認していない）。DAZ は 0x63ab2b の経路で立ちうる。RC は、`ldmxcsr` に渡される値のどれからも非ゼロになる経路は見つからなかった（0x13a5054、0x13e4d50 の bit 14:13 は 0、0x475c60 は呼び元が 0 か旧値を渡す）。ただしデータフローを全部追ったのではない（0x40a360 の引数が呼び元でどう作られるか、0x40a3b0 の戻り値が書き換えられていないかは、上の呼び元を目で見た範囲）。

決定: 上のとおり FTZ/DAZ が非既定になりうるので、「非既定なら abort」とはしない。RC、FTZ、DAZ はすべて、ゲストの値のままホストの命令に反映する（§1）。0x475c60 の RC 変更も、この方式では反映される。実機突合は全 RC（4 通り）× FTZ × DAZ（計 16 通り）で行った。

## 4. PREFETCHh

SDM Vol.2（PREFETCHh）: 「merely a hint and does not affect program behavior」、メモリオペランドは m8 でアドレスの計算だけ。実装は `prefetch.rs`: 実効アドレス式を評価して捨てる（`(void)(R[8] + UINT64_C(0x10));`）。メモリの読み書きをしない。レジスタ、フラグ、MXCSR を変えない。rip 相対（`UINT64_C(アドレス)`）、base+index*scale、REX.B の base、base なし（disp のみ）に対応する。FS/GS セグメントは既存の `ea` が拒否する。

既存の実装はなかった（`crates/fe-lift/src` に Prefetch の処理なし。`docs/` を `prefetch`、`no-op` で検索した範囲では、旧調査に「prefetch は no-op」と書いた箇所は見つからず、未実装の一覧にだけ載っていた）。

## 5. 検証

- `lift/crates/fe-lift/tests/mxcsr_flags.rs`（9 件）: ホストの機械語（mmap したスタブ。intrinsics ではない）と lift の生成 C を同じ状態で走らせ、XMM（case が設定する 5 本と、触ってはいけない他のレジスタ・メモリ）、GPR、COMIS の ZF/PF/CF/OF/SF/AF、MXCSR の全ビット（状態ビットと制御ビット）を比較する。MXCSR は case ごとにスタブの中で設定し、命令の後に読み戻し、スタブの先頭で読んだ元の値を戻す（テストスレッドの MXCSR は変わらない）。入力: QNaN/SNaN（符号、ペイロード付き）、±0、±inf、非正規化数、最大/最小の正規数、オーバーフロー/アンダーフローの境界、丸めの境界（tie）、不正確になる値、整数の端、レーンごとの乱数の組み合わせ。全 RC × FTZ × DAZ、事前に立っている sticky ビット。スカラー算術のレジスタ形は、プールの全ペアも既定の MXCSR で追加。
- `flags_match_sdm_expectations`: 手書きの 61 件（SDM の節に従い、どの入力でどのフラグが立つか）を C 側だけで確認。
- `unmasked_raised_flag_is_a_named_abort`: マスクを外したときの abort（ZE、IE、PE）、マスク時は続行して sticky、フラグが立たなければ全マスク解除でも続行。
- `prefetch_never_faults_and_changes_nothing`: 4 種 × 8 通りのアドレス指定 × 9 個の不正アドレス（0、1、0x10、非正規、カーネル側、0xFFFF…）。C 側は fault せず、レジスタ・XMM・フラグ・MXCSR が不変。ホストの機械語でも同じ不正アドレスで fault しないことを確認（同じ 288 通り）。
- MXCSR を各ホスト実行の前後で既知値に初期化し、元の値を復元している（スタブ内）。

## 6. 性能（実測、`mxcsr-flags-run.md` §4）

状態ビットの更新は 1 命令につき `ldmxcsr` 1 回と `stmxcsr` 1 回（と数命令）を足す。-O0 の関数呼び出し構造で、1 演算あたり約 +11〜13 ns（addsd 5.1→16.4 ns、cvtsi2sd 1.3→14.0 ns）。生成 C のサイズは、all-pdata の `c_bytes` が 217,968,523 → 215,539,533（−2.4 MB、−1.1%）で、増えていない。ただし `fenv`（`fetestexcept` は `stmxcsr` と x87 の `fnstsw` の両方を読む）や `__builtin_ia32_stmxcsr` による取得は、同じ命令列になるか重くなるので、別の方式として実装・計測はしていない（`ldmxcsr` を条件付きにする案は §6 の予備ベンチ `out-mxcsr/bench/bench.c` で 4 方式が 12.1〜12.8 ns/op の差に収まり、差が出なかったので採用していない。ただしこのベンチは -O0 の呼び出し自体が支配的で、方式の差を判別できた確証はない）。

## 7. 限界（事実）

- 状態ビットはホストの命令が立てたものを写している。ホストと違う CPU の挙動（たとえば AMD と Intel の非正規化数の扱いの差）は、この環境（AMD Ryzen 9 9950X）でのみ確認した。
- 非マスクの例外は #XM を再現せず abort する。ハンドラが SEH で処理するようなゲストは動かない（FLEngine で、マスクが外れる経路があるかは追っていない）。
- オラクル（run5）は native が 0x1F80 で入るので、RC/FTZ/DAZ が非既定の関数突合はオラクルには含まれない。単体テストで全組合せを確認している。
- ホストの MXCSR は命令ごとに書き換える。`SIGFPE` は全マスクのため起きない。ゲストの命令の途中でシグナルが入ると、そのハンドラは一時的にゲストの制御ビットで動く（シグナルハンドラは使っていない。確認していない）。
