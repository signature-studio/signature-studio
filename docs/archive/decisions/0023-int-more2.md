# 0023: 整数・システム命令（bsf、bsr、bts、btr、btc、rol、ror、xchg、cmpxchg16b、jecxz、jrcxz、pause、sfence、lfence、mfence）

状態: 作業者（サブエージェント）が決定。依頼者（PM）の判断は含まない。

実装は `lift/crates/fe-lift/src/int_more2.rs`（新規）。`lib.rs` の変更は `mod int_more2;` の 1 行と、`lift_insn` の先頭に `if let` 4 行の追加のみ。runtime は新規ファイルなし（生成 C は `fle_runtime.h`、`fle_flags.h` の既存の `R`、`FL`、`fle_ld`、`fle_st`、`fle_sext` だけを使う）。テストは `lift/crates/fe-lift/tests/int_more2.rs`（新規、10 件）。

振り分けを `lift_insn` の先頭（条件分岐の判定より前）に置いた理由: iced は jecxz、jrcxz を `ConditionalBranch` とするので、先頭の判定が `cc_bits` で `no condition code (loop or jrcxz)` を返して止まる。`int_more2::lift` は既存の他の命令を横取りしない（扱う mnemonic は Bsf、Bsr、Bts、Btr、Btc、Rol、Ror、Xchg、Cmpxchg16b、Pause、Sfence、Lfence、Mfence、Jecxz、Jrcxz、Jcxz だけ）。

## 仕様の出典

**Intel SDM 公式 PDF を取得できた**（非公式の転載ではない）。

- URL: https://cdrdv2.intel.com/v1/dl/getContent/671110 （取得 2026-10-20、HTTP 200、application/pdf、11,294,447 バイト）
- 文書: 「Intel® 64 and IA-32 Architectures Software Developer's Manual, Volume 2 (2A, 2B, 2C, & 2D): Instruction Set Reference, A-Z」、Order Number 325383-093US、2574 ページ（PDF のメタデータ）。
- 保存: `lift/out-int2/spec/sdm-vol2.pdf`、`pdftotext -layout` の出力が同じ場所の `sdm-vol2.txt`。
- ページ番号は、抽出テキストの各ページ末尾の見出し（例 `BSF—Bit Scan Forward Vol. 2A 3-107`）で付けた。PDF の物理ページ番号ではない。
- 読んだ節: BSF 3-107、BSR 3-109、BT 3-111〜3-113、BTC 3-114、BTR 3-116、BTS 3-118、CMPXCHG8B/CMPXCHG16B 3-196〜3-198、Jcc（JECXZ、JRCXZ を含む）3-499〜3-503、LFENCE 3-552、MFENCE 4-15、PAUSE 4-226、RCL/RCR/ROL/ROR 4-533〜4-537、SFENCE 4-628、XCHG 6-32〜6-33、XGETBV 6-36〜6-37、Vol. 2A の「Bit(BitBase, BitOffset)」の定義（3-11）。
- 図、表の画像は見ていない。テキスト抽出で読める範囲の文面と擬似コードだけを根拠にした。

## 仕様で決まること（SDM の文面）

| 命令 | 内容 |
|---|---|
| BSF / BSR | 最下位（最上位）の 1 のビット位置を宛先へ。ソースが 0 なら宛先は変更しない。フラグ: ソースが 0 のとき ZF = 1、そうでなければ ZF = 0。PF は「ソースの 1 の数が偶数なら 1」。CF、OF、SF、AF は 0。脚注: 古いプロセッサでは 0 のソースで 32 ビット操作が 64 ビット宛先の上位 32 ビットを消すことがあり、CF、OF、SF、AF、PF が不変のことがある |
| BTS / BTR / BTC | CF = 変更前のビット。ZF は変えない。OF、SF、AF、PF は undefined。レジスタ形はオフセットを 16、32、64 で割った剰余。メモリ形は「ビット列」で、範囲はオペランド幅による。LOCK 可 |
| ROL / ROR | カウントは 5 ビット（REX.W で 6 ビット）にマスク。マスク後のカウントが 0 なら変更なし（フラグも）。0 でなければ CF を設定。OF はマスク後のカウントが 1 のときだけ定義（ROL: MSB XOR CF、ROR: 上位 2 ビットの XOR）、それ以外は undefined。SF、ZF、AF、PF は常に不変。ROL、ROR の回転量は (マスク後のカウント MOD サイズ)。8、16 ビットでマスク後のカウントがサイズの倍数なら、値は変わらず CF だけが更新される（擬似コードの「`IF (COUNT & COUNTMASK) ≠ 0 THEN CF := LSB(tempDEST)`」） |
| XCHG | メモリ オペランドがあれば、LOCK の有無にかかわらずロックのプロトコルが自動で働く。フラグは変えない。`90`（xchg eax,eax）は NOP の別名 |
| CMPXCHG16B | RDX:RAX と m128 を比較。等しければ ZF = 1 と RCX:RBX を格納。等しくなければ ZF = 0 と RDX:RAX にメモリ値を読み込み、メモリは書き戻す。CF、PF、AF、SF、OF は不変。64 ビットモードで 16 バイト境界にないと #GP(0)。CPUID.01H:ECX.CMPXCHG16B[13] = 0 でも #GP(0) |
| JECXZ / JRCXZ | ECX（RCX）が 0 ならジャンプ。フラグは見ない。アドレスサイズで決まる（67 接頭辞で JECXZ）。JCXZ は 64 ビットモードで N.E. |
| PAUSE | アーキテクチャ状態を変えない遅延 NOP |
| SFENCE / LFENCE / MFENCE | メモリ順序のみ。フラグ、レジスタは変えない |
| XGETBV | XCR[ECX] を EDX:EAX へ。ECX = 0 は XCR0。ECX = 1 は CPUID.0DH.01H:EAX.XGETBV1[2] = 1 のときだけ有効。それ以外は #GP。XCR の実装されていないビットは undefined。CPUID.01H:ECX.XSAVE[26] = 0 や CR4.OSXSAVE = 0 で #UD |

## 実装した内容と判断

1. **BSF / BSR**（16、32、64 ビット、レジスタとメモリのソース）: ソースを先に読む。ソースが 0 のときは宛先を書かず、ZF = 1、PF = 1。0 でないときは宛先へインデックスを書き（32 ビットはゼロ拡張）、ZF = 0。どちらの場合も CF、OF、SF、AF は 0。
   - **PF（ソースが 0 でないとき）は `FLE_UNDEF` にした。** SDM の文面（ソースの 1 の数の偶奇）とホストの実測が食い違うため（下の実測）。PF を読む jcc、setcc、cmov は、既存の仕組みで abort する。
   - ソースが 0 のとき PF = 1 は、SDM の文面（1 の数が 0 で偶数）とホストの実測が一致する。
   - ソースが 0 のときに宛先を書かないことは、SDM の本文どおり。ホスト（AMD）でも 32 ビット操作で宛先の上位 32 ビットが残った（実測）。SDM の脚注にある「古いプロセッサ」の挙動は実装していない。
   - 古いプロセッサで CF、OF、SF、AF、PF が不変という脚注は実装せず、本文どおり CF、OF、SF、AF を 0 にした。ホストの実測もそうだった（入力フラグ 0 と 0x8d5 の両方で確認）。
2. **BTS / BTR / BTC**（16、32、64 ビット、レジスタ、メモリ、レジスタのオフセット、imm8、LOCK 付き）: 既存の `bt`（決定 0013）と同じ規則。
   - レジスタのベース: オフセットをオペランド幅で割った剰余。
   - メモリのベースにレジスタのオフセット: オフセットは符号付き。バイト EA + floor(offset / 8) のビット (offset mod 8) を読み書きする（1 バイトのアクセス）。SDM は「オペランド幅の分をアクセスすることがある」と書くが、到達するビットは同じ。ホスト突合（オフセット -128〜255 ビット）で一致。
   - メモリのベースに imm8: オフセットをオペランド幅で割った剰余、EA からオペランド幅のアクセス。
   - フラグ: CF = 変更前のビット。ZF は変えない。PF、AF、SF、OF は `FLE_UNDEF`（SDM の undefined）。
3. **ROL / ROR**（8、16、32、64 ビット、CL、imm8、暗黙の 1、レジスタとメモリ）: SDM の擬似コードのとおり。マスク後のカウントが 0 ならフラグ不変。0 でなければ CF を設定し、OF はマスク後のカウントが 1 のときだけ値を持ち、それ以外は `FLE_UNDEF`。
   - **ホスト実測に合わせた点（SDM は明示しない）:** マスク後のカウントが 0 の 32 ビットのレジスタ形でも、宛先の上位 32 ビットはゼロ拡張される（ホストの実測。メモリ形は書き戻すだけで同じ値）。
   - AH、BH、CH、DH は未対応（`reg_num` が拒否する。既存の `lib.rs` と同じ）。
4. **XCHG**（`86`、`87`、短い形 `90+r`、REX.W、16 ビット接頭辞）: 両方の値を読んでから書く。メモリのアドレスは 1 回だけ計算する。レジスタ同士の同一レジスタ（`87 c0`、`87 db`）でも 32 ビットはゼロ拡張（**ホスト実測に合わせた点**。SDM は同一レジスタ時の上位 32 ビットを書いていない）。フラグは変えない。LOCK と暗黙のロックは無視（下の「LOCK」）。
5. **CMPXCHG16B**: SDM の擬似コードどおり。ZF だけを設定する。16 バイト境界にないときは「#GP(0)」の代わりに `fle_stop` 相当（stderr に "fle: cmpxchg16b operand is not 16-byte aligned (#GP)" を出して abort）。メモリの 2 つの 8 バイトは、書き込みが起きる前に両方読む。
6. **JECXZ / JRCXZ**: `if (cond) goto L_<target>;` または、ターゲットが関数の入口なら `if (cond) { f_<target>(); return; }`。条件は `(uint32_t)R[1] == 0`（jecxz）、`R[1] == 0`（jrcxz）。Ctl は `Seq` のままで、`lib.rs` に新しい制御の種類を足していない。分岐先と次の命令は、iced の `ConditionalBranch` として既存の `traverse` が関数に含める。JCXZ（16 ビットのアドレスサイズ）は SDM が 64 ビットモードで N.E. としているので、未対応のまま（`Err`）。
7. **PAUSE**: 何もしない。**SFENCE、LFENCE、MFENCE**: `__atomic_thread_fence(__ATOMIC_SEQ_CST);`（ゲストは 1 スレッドなので観測できる差はない。SEQ_CST は最も強い順序で、仕様の範囲内）。

## LOCK

xadd、cmpxchg と同じく LOCK は無視する（決定 0013 の項目 9）。FLEngine が別スレッドを使わないことは、ここでも未確認の前提である。CreateThread は応答表になく（`remaining-unsupported.md` §5.2 D6）、今回の停止点より前にスレッドが作られる経路は確認していない（確認していない、であって、無いとは言っていない）。

## 実装しなかったもの（fail-closed）

- **XGETBV**: 未対応のまま（`Xgetbv: mnemonic outside minimal set` が 1 件残る）。SDM は XCR0 の値を決めていない（プロセッサと OS の状態で決まる）。この `fle_cpuid.c` は CPUID 葉 1 の ECX を 0 にして XSAVE（ビット 26）、OSXSAVE（ビット 27）、AVX（ビット 28）を主張しないので、SDM の #UD 条件（XSAVE = 0）と整合する値を作る根拠はあるが、「どの値を返すか」を決める仕様はない。値を選ぶ判断は別に要る。0x639a03 の直前は `and ecx, 0x18000000; cmp ecx, 0x18000000; jne` で、`fle_cpuid.c` の ECX = 0 ならこの `jne` で飛び越える（`remaining-unsupported.md` §4 の命令列の読み。実行では確認していない）。
- Cmpxchg8b、Rcl、Rcr、Shld、Shrd（今回の対象外）。

## ホスト実測（AMD Ryzen 9 9950X、Linux x86_64。Intel の CPU ではない）

`lift/out-int2/work/probe.c`、`probe2.c`、`probe3.c` と出力 `probe-out.txt`、`probe2-out.txt`、`probe3-out.txt`。

- **BSF / BSR の PF（ソースが 0 でない）:** ランダムなソース各約 20 万件（32 ビット、64 ビット、BSF、BSR の 4 通り、有効な件数は 146,916〜196,913）で、PF は「宛先に書いたインデックスの下位 8 ビットの偶数パリティ」と全件一致（不一致 0）。SDM の文面（ソースの 1 の数の偶数パリティ）と一致したのは約半分（例: bsf32 は 146,916 件中 73,620 件）。つまり**ホストは SDM の文面と違う**（Intel の CPU では確認していない。AMD の実装の違いか、SDM の文面が実際と違うのかは、この測定では分からない）。CF、OF、SF、AF は全件 0。
- **BSF / BSR（ソースが 0）:** 宛先は不変（32、64、16 ビットすべて。32 ビットでも上位 32 ビットが残る）。ZF = 1、PF = 1、CF = OF = SF = AF = 0。宛先の下位バイトが奇数パリティ（0x01、0x02、0x07、0x80 など）でも PF = 1。入力フラグ 0 と 0x8d5 で同じ。
- **ROL / ROR:** マスク後のカウント 0 のときフラグは不変、32 ビットの宛先の上位はゼロ拡張。8 ビットの ROL を 8 回、16 回、16 ビットの ROL を 16 回すると値は同じで CF = LSB になる（SDM の擬似コードどおり）。

## テスト（ホスト突合）

`cargo test --offline -p fe-lift --test int_more2`: 10 件、すべて通過（`lift/out-int2/test-int_more2-2.txt`）。方式は `tests/int_more.rs` と同じ（ホストはインラインの機械語を mmap して実行、C は `lift_body` の本文を clang -O0 でコンパイル）。違いは、入力を 1 ベクタあたり数百〜数千行（乱数 + 境界値 + ベクタ固有の値）にして、レジスタ、64 バイトのバッファ、**定義されたフラグ**を比べる点。

- 比較: レジスタ（rdi がメモリ形のときを除く）、バッファ 64 バイト、C が `FLE_UNDEF` にしていない各フラグ（入力フラグはランダム）。
- **「undefined のセットが SDM と一致する」検査:** C が `FLE_UNDEF` にするフラグの集合が、命令ごとに期待する集合と完全に一致することを検査する（BSF/BSR: ソース 0 なら空、そうでなければ PF。BT*: PF、AF、SF、OF。ROL/ROR: マスク後のカウントが 2 以上なら OF。他は空）。これで、「未定義だから比べない」が定義済みのフラグを隠すことを防ぐ。
- 規模（`lift/out-int2/test/summary-*.txt`）:

| 群 | ベクタ | 行（入力） | レジスタ比較 | フラグ比較（C が定義） | C が undefined にしたフラグ |
|---|---|---|---|---|---|
| bsf/bsr | 36 | 14,400 | 79,200 | 75,441 | 10,959 |
| bts/btr/btc | 477 | 57,240 | 300,240 | 114,480 | 228,960 |
| rol/ror | 248 | 39,680 | 222,720 | 214,091 | 23,989 |
| xchg | 20 | 8,000 | 44,000 | 48,000 | 0 |
| pause と fence | 4 | 1,600 | 9,600 | 9,600 | 0 |
| cmpxchg16b | 2 | 1,600 | 8,000 | 9,600 | 0 |
| jecxz/jrcxz | 2 | 420（rcx の値 210 × 2） | — | — | — |

  - bsf/bsr のソース 0 の行は 3,441、ROL/ROR のマスク後のカウント 0 の行は 7,293、カウント 1 の行は 8,398、cmpxchg16b の ZF = 1 の行は 823、jecxz/jrcxz は分岐あり 58、なし 362（各テストで下限を assert している）。
- 他: `cmpxchg16b_misaligned_aborts`（境界にないアドレスで abort）、`jcxz_to_function_entry_is_a_tail_call`（合成 PE で、jecxz、jrcxz が関数の入口への末尾呼び出しになること、`unsupported_total` が 0 であること、rcx = 2^32 で jecxz だけが分岐すること）、`unsupported_stay_unsupported`（xgetbv、AH 形、GS セグメントが未対応のまま）。

## 確認できていないこと

- Intel の CPU での挙動（ホストは AMD）。特に BSF/BSR の PF（ソースが 0 でないとき）。そのため PF を未定義にした。
- メモリ形の BTS/BTR/BTC を、ページ境界で SDM の「オペランド幅のアクセス」がフォールトする場合（モデルは 1 バイトだけ触る）。
- LOCK 付きの命令が単一スレッドで足りること（上の「LOCK」）。
- **CMPXCHG16B と CPUID の整合:** SDM は CPUID.01H:ECX[13] = 0 のとき #GP(0) としている。`fle_cpuid.c` は葉 1 の ECX を 0 で返すので、ゲストの CPUID の自己申告は「CMPXCHG16B なし」になる。今回は命令を実装したが、この食い違いは決めていない（ゲストが CPUID を見てから使うのかは未確認）。0x41baa1 と 0x4a5bb7 の 2 か所がある。
