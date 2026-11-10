# 0032: VEX（VMOVUPS/VMOVAPS/VMOVNTPS/VZEROUPPER）、FS/GS 付き LEA と MOV、XGETBV の扱い

状態: 作業者が決定。実行記録は `docs/investigations/vex-run.md`。

実装: `lift/crates/fe-lift/src/vex.rs`（新規）、`lift/runtime/fle_vex.h`（新規、ヘッダのみ）、テスト `lift/crates/fe-lift/tests/vex.rs`（新規、13 件）。既存ファイルの変更は 3 つだけ。

- `lift/crates/fe-lift/src/lib.rs`: `mod vex;` 1 行と、`lift_insn` の `mmx::lift` の直後に `if let` 4 行（+5 行）。
- `lift/runtime/fle_sse.h`: 末尾に `#include "fle_vex.h"` と注釈 1 行（+3 行）。
- `lift/runtime/fle_imports.c`: `fle_tls_values` の `static` を外した（1 行。GS:[0x58] がこの配列を指すため）。

## 対象（実測。`lift/out-mmx/out/after-mmx-896.c` の `fle_unsupported` 行と `lift/objdump-text.txt`）

| 命令 | オペランド形 | 件数 | 所在 |
|---|---|---|---|
| VMOVUPS | `ymm, m256`（ロード） | 8 | 関数 0x409c80（0x409da0、0x409da4、0x409de0、0x409e00、0x409e79、0x409e7d、0x409eb0、0x409ed0） |
| VMOVUPS | `m256, ymm`（ストア） | 4 | 同（0x409df0、0x409df5、0x409ec0、0x409ec4） |
| VMOVAPS | `m256, ymm`（ストア） | 2 | 同（0x409de5、0x409eb5） |
| VMOVNTPS | `m256, ymm`（ストア） | 2 | 同（0x409e05、0x409ed5） |
| VZEROUPPER | なし | 2 | 同（0x409df9、0x409ec9） |
| LEA | `lea rax, fs:[rax+riz*1+0x0]`（バイト `64 48 8d 44 20 00`） | 1 | 0x409dda（0x409c80 の中、AVX 版コピーのループ前） |
| MOV | `mov rax, gs:0x58`（`65 48 8b 04 25 58 00 00 00`） | 1 | 0x41d14b（関数 0x41d110） |
| XGETBV | `0f 01 d0` | 1 | 0x639a03（関数 0x6399e0）。**実装しない** |

VEX の 18 件はすべて VEX.256（ymm）。VEX.128 形は閉包に 0 件だが、SDM の規則（下位 128 ビットを書き、上位をゼロ化）で同じ関数が扱うので実装した。

## 判断

1. **ymm の持ち方**: 既存の `fle_xmm X[16]`（XMMn = ymm の下位 128 ビット。SSE 系ヘルパと共有）に、上位 128 ビットを持つ `fle_xmm YH[16]` を足した。旧実装の `X[16]` と `YL[16]` に当たる（今の runtime には `YL` は存在せず、`fle_sse.h` の `X[16]` だけ）。`YH` は `fle_vex.h` の weak 定義（`MM[8]` と同じ流儀）で、生成 C のヘッダ文字列（`lib.rs`）は変えていない。
   - レガシー SSE 命令は `X` だけを書き、`YH` は変わらない（SDM: 「Bits (MAXVL-1:128) ... remain unchanged」）。
   - VEX.128 の書き込み: `X := SRC`、`YH := 0`。VEX.256 の書き込み: 両半分。ストア: レジスタは変わらない。
   - 128 ビットを超える状態（ZMM、AVX-512 の上位）は持たない。EVEX 形は `Err`。XMM/YMM16 以上も `Err`。
2. **アラインメント**（SDM の Other Exceptions）: VMOVAPS と VMOVNTPS のメモリは、xmm 形で 16 バイト、ymm 形で 32 バイト境界でなければ #GP（Table 2-18 の「VEX.256: Memory operand is not 32-byte aligned. VEX.128: ... 16-byte aligned」）。生成 C は abort する（既存の `fle_misaligned` と同じ流儀。32 バイト用は `fle_misaligned32`）。VMOVUPS は境界の規則がなく、チェックしない（ホストでも、非整列の VMOVUPS が fault しないことを突合で確認）。
3. **フラグ**: どれも変えない。
4. **VZEROUPPER**: 64 ビットモードなので YMM0-15 の上位 128 ビットを 0 に（SDM: `limit := 15`）。下位は変えない。SSE/AVX 遷移ペナルティは架構上の状態ではないのでモデル化しない。
5. **LEA の FS/GS 接頭辞**: アドレスのオフセット部分だけを計算し、セグメントベースは足さない（SDM LEA: 「the source operand is a memory address (offset part)」）。したがって `lea rax, fs:[rax+riz*1+0]` は `R[0] = R[0]`。ホスト実機で 8 形（FS/GS、64/32 ビットのデスティネーション、SIB、disp32）を突合した。`riz`（インデックスなし）は iced でインデックス `None` になる。32 ビットアドレス（0x67）は既存の LEA と同じく `Err`（lib.rs の `ea` の規則を踏襲）。
6. **GS（TEB）**: 既存の runtime に GS ベースの実装は**なかった**（`FLE_TEB`、`0x1D00000`、`gs` を `lift/` 全体で検索して 0 件）。そこで `fle_vex.h` に、ホスト側のバッファ `fle_teb[0x2000]`（weak、0 初期化）を置き、次の 2 つだけをモデルにした。それ以外の GS オフセットは `Err`（生成時に `fle_unsupported`）、実行時も `fle_gs_ld8` が abort する。
   - `mov r64, gs:[0x58]`（閉包の 1 件）: 戻り値は `fle_tls_values` の先頭アドレス。`fle_tls_values` は `fle_imports.c` の TlsAlloc/TlsSetValue/TlsGetValue が使う 1 スレッド用スロット表（索引は TlsAlloc が払い出す 0 始まり）。0x41d14b の後続は `ecx = [0x13a9388]`（TlsAlloc で得た索引）、`rax = [rax + rcx*8]` なので、`TlsGetValue(索引)` と同じ値になる。**これは作業者の判断**。Windows の TEB の 0x58 は ThreadLocalStoragePointer（PE の `.tls` 用の配列）で、動的な TlsAlloc の索引が指すのは別の場所（TEB の TlsSlots）である（Vergilius Project の `_TEB`: ThreadLocalStoragePointer 0x58、TlsSlots 0x1480。Microsoft の公式ページでは TEB の配置は決まらない）。ゲストが 0x58 の配列を索引で読む以上、この環境ではその配列を TlsGetValue の表にした。
   - `mov r64, gs:[0x30]`（閉包に 0 件。TEB の自己参照）: `fle_teb` のアドレス（mingw の `winnt.h` の `NT_TIB64.Self` が 0x30。二次資料）。2 つの値は `fle_teb` の同じオフセットにも書くので、Self を読んでからそこ経由で 0x30/0x58 を読んでも同じ値になる。必要最小限を超える追加（閉包にない）なので、ここに記録する。
   - 他の形（32 ビットロード、ストア、ベース/インデックスあり、RIP 相対、ALU 命令の GS オペランド、VEX の GS オペランド）は `Err`。テスト `other_fs_gs_forms_stay_unsupported`。
7. **FS**: `fs:` を持つメモリオペランドは、LEA 以外はすべて `Err`（fail-closed）。0 ベースとして通す案は取らない。理由: FS のベースを 0 とみなすと、ゲストが FS 経由で読んだ値が本物のスレッドデータでないのに通ってしまう。x64 の Windows ユーザーモードで FS を使う場面を、この閉包は含まない（閉包の FS は 0x409dda の LEA の 1 件だけで、これは実質 nop 形。x64 Windows が FS を使わないのは推測で、Microsoft の資料では確認していない）。
8. **XGETBV: 実装しない**（判断は下）。

## XGETBV の記録（実装しない。静的に確認した事実）

- SDM は XCR0 の値を決めない（プロセッサと OS の状態で決まる）。値を作る根拠がないので、`fle_unsupported` のまま（決定 0023 と同じ）。
- 0x639a03 は関数 0x6399e0（呼び出し元は 0x639beb の直接 call 1 件）の中。命令列（`objdump-text.txt`）:
  `cpuid(0)`、`eax < 1` なら 0x639a15（戻り値 0）、`cpuid(1)`、`ecx & 0x18000000 != 0x18000000` なら 0x639a15（`jne` 0x6399ff）、`xor ecx,ecx`、**`xgetbv`**、`(eax & 6) == 6` なら戻り値 1、それ以外は 0。
  ビット 27 は OSXSAVE、ビット 28 は AVX（CPUID 葉 1 の ECX）。
- `lift/runtime/fle_cpuid.c` は葉 1 の ECX を 0 で返す（読んで確認）ので、0x6399ff の `jne` が必ず取られ、**XGETBV は実行されない**。葉 0 の EAX は 1 を返すので、1 つ目の `jb` は取られず、葉 1 の検査まで進む。この結論は静的な読みで、この関数の実行では確認していない（実行ログの先頭 256 件に f_6399e0 は出ていない）。
- 今後 CPUID が OSXSAVE と AVX を主張する場合は、XGETBV の戻り値（XCR0 の値）の選択が先に要る。これは別の判断。

## AVX 版コピーへ入る条件（静的に確定。実行で確認した部分と区別して書く）

- 前任の記録（`remaining-unsupported.md`）は、`0x15e69c0` の出所を未確認としていた。今回、確定した:
  - 関数 0x40f4b0 は、`ebx = 0..7` で関数 0x40f490（内部で `cpuid`、`eax = ebx`、`ecx = 0`）を呼び、結果の 16 バイト（eax、ebx、ecx、edx）を `0x15e69a8 + 16*ebx` に `movsq` 2 回で書く。葉 1（`ebx = 1`）の ECX は `0x15e69b8 + 8 = 0x15e69c0`。
  - 関数 0x409bf0 は、`0x15e69c0 & 0x10000000`（CPUID 葉 1 の ECX の AVX ビット）から `0x15e60b0` を作る（0x409c0f）。`0x15e60b0` の参照は 0x409c0f（書き込み）、0x409d1d と 0x409e1a（`test [..],1`）だけ（`objdump-text.txt` の RIP 相対参照の検索）。0x409bf0 の入口の `call 0x409bd0` の結果（`al`）が 0 なら、`0x15e60b0` は書かれず 0 のまま（`.bss`）。
- 現在の `fle_cpuid.c` は葉 1 の ECX が 0 なので、どちらの場合も `0x15e60b0 = 0` で、AVX 版（0x409da0 以降）に入らない。**今回 VEX を実装しても、この実行では AVX の命令は実行されない**（0x40f4b0 と 0x409bf0 が実行ログに出ていること、f_409c80 が 11 回出ていることは確認済み。SSE 版が通っている）。
- 決定 0009 の「AVX を立てない根拠は lift が VEX を持たないこと」は、今回で根拠が消えた。ただし CPUID を変えることは、上の XGETBV の判断を伴うので、今回は `fle_cpuid.c` を触っていない。

## 仕様の根拠（Intel SDM Vol. 2、`lift/out-int2/spec/sdm-vol2.txt`。他の決定と同じ抽出テキスト）

- VMOVUPS（4-126〜4-128）: 「VMOVUPS (VEX.256 Encoded Version, Load - and Register Copy) DEST[255:0] := SRC[255:0]; DEST[MAXVL-1:256] := 0」、ストアは `DEST[255:0] := SRC[255:0]`、「VMOVUPS (VEX.128 Encoded Version) DEST[127:0] := SRC[127:0]; DEST[MAXVL-1:128] := 0」。「the operand may be unaligned without causing a general-protection exception」。
- VMOVAPS（4-41〜4-44）: 同じ擬似コード。メモリは 16/32 バイト境界でなければ #GP。
- VMOVNTPS（4-96、4-97）: `DEST := SRC`（VEX 形は `DEST[VL-1:0] := SRC[VL-1:0]`）。「The memory operand must be aligned on a 16-byte (128-bit version), 32-byte (VEX.256 encoded version) ... boundary otherwise a general-protection exception (#GP)」。非一時ヒントは、このモデルでは通常のストアと同じ（キャッシュの挙動はモデルにない）。
- VZEROUPPER（5-776）: 「In 64-bit mode, the instruction zeroes the bits in positions 128 and higher in YMM0-YMM15」。擬似コード `limit := 15; FOR i in 0..limit: simd_reg_file[i][MAXVL-1:128] := 0`。
- LEA（3-547）: 上記のとおり。

## 確認できていないこと・モデルにないもの

- #UD / #NM（CR4.OSXSAVE、XCR0、CPUID の AVX ビットが 0 のとき VEX で #UD。Table 2-18、2-25）はモデルにない。ゲストの CPUID が AVX を立てないので、AVX 版に入るゲストは現れない、というのが現状の前提。
- VEX.vvvv が 1111B でない場合の #UD（SDM の各命令の Other Exceptions）は、`vex.rs` では検査していない。`mov` はオペランド数が 2 でなければ `Err` にするが、iced が vvvv != 1111B の VMOVUPS をどうデコードするかは確認していない（テストにもない）。閉包の 18 件は、すべて vvvv = 1111B のバイト列（`objdump-text.txt`）。
- #AC（アラインメントチェック）、非正規アドレス、ページフォルト: 既存の SSE と同じく、モデルにない。
- VMOVAPS/VMOVNTPS の非整列で abort するのは C 側だけをテスト（ホストは #GP で SIGSEGV になるので突合しない）。

## テスト

`cargo test --offline -p fe-lift --test vex`: 13 件、すべて通過（`lift/out-vex/test-vex-1.txt`）。ホスト突合 9,860 ケース、失敗 0。内訳と変異テストは `docs/investigations/vex-run.md`。
