# dsp_ippv2_x64.dll のレガシー SSE 未対応命令の実測（W8）

実行日: 2026-10-23。HEAD 6e1bc06 に、本作業の変更（決定 0075）を加えた作業ツリー。入力は調査文書 `dsp-ippv2-lift-feasibility.md` §4、§5.5、§9 W8 と同じ測定の再実行。決定は `docs/decisions/0075-legacy-sse-forms-for-dsp-ippv2.md`。作業物は `lift/out-w8/`（§6）。

## 0. 凡例

- **実測** = コマンドの出力。`lift/out-w8/` に残してある。
- 入力は `lift/out-ipp/merged-text.dll`（**実在の DLL ではない測定用の改変コピー**。`.text` と `IPPCODE` を 1 つの `.text` にしたもの。ImageBase 0x180000000 のまま）。
- 「現行 CPUID 応答の系統」= `lift/out-ipp/table-seeds-idx0.txt`（表の添字 0 のエントリ 428 個）を `--seeds` に渡して `fe-lift lift` した閉包（関数 1,891、命令 476,308、生成 C 42,866,378 B）。調査文書 §5.5 の「添字 0 のみ」と同じ。この閉包で `fle_unsupported` が 4,938 箇所、うちレガシー命令が 4,074 箇所（35 キー）。
- 「箇所」は生成 C に出た `fle_unsupported` の出現数（ブロックが複数の関数にコピーされると、それぞれ数える。`fe-lift` の統計と同じ数え方）。VA の重複を除くと 4,743。
- オペランド形とプレフィックスは、生成 C の `fle_unsupported` の VA を DLL のバイト列から capstone で読み直して得た（`lift/out-w8/w8_unsup_forms.py`）。

## 1. 結論（先に）

1. **現行 HEAD で、SSE2 以前の命令が「未対応」と数えられていたのは本当だった。** 原因は「ある形だけが欠けていた」ことではなく、**そのニーモニック自体が `fe-lift` に無かった**こと。`Orps`、`Psrldq`、`Andnps`、`Pcmpeqb`、`Pmovmskb`、`Movmskps`、`Pextrw` などは、xmm,xmm 形もメモリ形も 66 付きの形も、すべて未対応だった（表 §3.1）。近い命令は `sse_pack.rs` に `andps`、`andpd`、`orpd`、`xorps`、`xorpd`、`andnpd`、`pcmpeqd` があり、`orps` と `andnps` はそれらと**ビット演算としては同じ**だが、ニーモニックの登録が無かった。メモリ形だけが欠けている命令は無い。
2. **FLEngine（531 件）との違い**: FLEngine の未対応は IAT 経由の `jmp` 530 件と未解決の間接 `jmp` 1 件だけで、命令の未対応は 0 件（`lift/out-ipp/lift-flengine-pdata.txt`。決定 0047 の時点で命令 stub 0）。FLEngine の閉包には、これらのレガシー命令がそもそも出現しない（`fe-decode closure --mnemonics` で `Orps`、`Andnps`、`Pcmpeqb`、`Pcmpeqw`、`Pcmpgtb`、`Pcmpgtw`、`Pmovmskb`、`Movmskps`、`Movmskpd`、`Pextrw`、`Pinsrw`、`Psrldq`、`Pslldq`、`Pmuludq`、`Psubq`、`Psubb`、`Paddb`、`Pmaxub`、`Pminub`、`Pmaddwd`、`Movntps`、`Movntdq`、`Movntpd`、`Movnti`、`Rsqrtps`、`Rsqrtss` はどれも 0 件。実測）。したがって「FLEngine で実装済みの形が DLL では未実装」という関係は**無い**。FLEngine で実装済みの近い命令（`Orpd`、`Andps`、`Andnpd`、`Pcmpgtd`、`Psubw`、`Paddq`）は DLL 側でも未対応に数えられていない。
3. **未対応のレガシー 4,074 箇所（35 キー）の内訳**: CPUID が名乗る SSE/SSE2 に属する形が 3,369 箇所（25 ニーモニック）、CPUID が名乗らない SSE3 の命令が 678 箇所（7 ニーモニック）、SSE 系ではないものが 27 箇所（`Int3` 21、`Int` 5、`Fxsave` 1）。3,369 + 678 + 27 = 4,074（§3）。

## 2. 全体の数（実測）

現行 CPUID 応答の系統（`--seeds table-seeds-idx0.txt`）:

| 項目 | 実装前 | 実装後 |
|---|---:|---:|
| `unsupported_total` | 4,938 | 1,569 |
| レガシー（非 V）の命令 | 4,074（35 キー） | 705（10 キー。SSE3 の 678 と `Int3` 21、`Int` 5、`Fxsave` 1） |
| 間接 jmp の目標が未解決（`jmp [表 + 添字*8]`） | 464 | 464 |
| `V*`（VEX / EVEX） | 275（36 キー） | 275 |
| IAT 経由 `call` / `jmp` | 122 | 122 |
| `gs:` のオフセットが未モデル | 3 | 3 |

4,938 − 3,369 = 1,569。

全体（`merged-text.dll` を `--pdata`、32,162 関数）: `unsupported_total` 1,011,928 → 974,004。差 37,924 箇所は、実装した 25 ニーモニックが全体の閉包に出ていた分。

FLEngine（`binaries/win64/FLEngine_x64.dll` を `--pdata`）: `unsupported_total` 531、命令 stub 0、`bad_nodes` 0、生成 C が実装前の `lift/out-ipp/flengine-pdata.c` と**バイト単位で同一**（`cmp`）。

## 3. 現行 CPUID 応答の系統の未対応レガシー 4,074 箇所（ニーモニック・形・プレフィックス別。実測）

「プレフィックス」は命令の符号化に含まれる必須プレフィックス（`66` / `F2` / `F3`）。`none` は NP（プレフィックスなし）。表は箇所数の多い順。「実装後」は本作業の後に残る箇所。

### 3.1 CPUID が名乗る SSE / SSE2 の形

| ニーモニック | オペランド形 | プレフィックス | 実装前の箇所 | 実装後 | 備考 |
|---|---|---|---:|---:|---|
| Pmuludq | xmm, xmm | 66 | 926 | 0 | SSE2 |
| Pmuludq | xmm, m128 | 66 | 26 | 0 | |
| Orps | xmm, xmm | none | 708 | 0 | SSE |
| Orps | xmm, m128 | none | 65 | 0 | |
| Psrldq | xmm, imm8 | 66 | 415 | 0 | SSE2 |
| Andnps | xmm, xmm | none | 291 | 0 | SSE |
| Andnps | xmm, m128 | none | 75 | 0 | |
| Movmskps | r32, xmm | none | 155 | 0 | SSE |
| Pmovmskb | r32, xmm | 66 | 129 | 0 | SSE2 |
| Pextrw | r32, xmm, imm8 | 66 | 111 | 0 | SSE2（`66 0F C5`） |
| Pcmpeqb | xmm, xmm | 66 | 90 | 0 | SSE2 |
| Pcmpeqb | xmm, m128 | 66 | 20 | 0 | |
| Rsqrtps | xmm, xmm | none | 75 | 0 | SSE。近似命令（決定 0075 §3） |
| Pslldq | xmm, imm8 | 66 | 49 | 0 | SSE2 |
| Movntps | m128, xmm | none | 46 | 0 | SSE |
| Movntdq | m128, xmm | 66 | 45 | 0 | SSE2 |
| Movmskpd | r32, xmm | 66 | 31 | 0 | SSE2 |
| Movntpd | m128, xmm | 66 | 26 | 0 | SSE2 |
| Pinsrw | xmm, m16, imm8 | 66 | 21 | 0 | SSE2 |
| Pinsrw | xmm, r32, imm8 | 66 | 1 | 0 | |
| Psubq | xmm, xmm | 66 | 15 | 0 | SSE2 |
| Psubq | xmm, m128 | 66 | 1 | 0 | |
| Pcmpgtw | xmm, xmm | 66 | 11 | 0 | SSE2 |
| Psubb | xmm, xmm | 66 | 8 | 0 | SSE2 |
| Pmaxub | xmm, xmm | 66 | 5 | 0 | SSE2 |
| Pmaxub | xmm, m128 | 66 | 1 | 0 | |
| Pminub | xmm, xmm | 66 | 5 | 0 | SSE2 |
| Pminub | xmm, m128 | 66 | 1 | 0 | |
| Movnti | m32, r32 | none | 4 | 0 | SSE2 |
| Pcmpeqw | xmm, xmm | 66 | 4 | 0 | SSE2 |
| Pcmpgtb | xmm, xmm | 66 | 4 | 0 | SSE2 |
| Pmaddwd | xmm, xmm | 66 | 4 | 0 | SSE2 |
| Rsqrtss | xmm, xmm | F3 | 1 | 0 | SSE。近似命令 |
| **小計** | | | **3,369** | **0** | 25 ニーモニック |

所見: レジスタ形と、メモリ形がこの閉包に出るものは、どちらも未対応だった。メモリ形が出るのは Orps、Andnps、Pmuludq、Pcmpeqb、Psubq、Pmaxub、Pminub、Pinsrw（m16）で、その箇所数は 65 + 75 + 26 + 20 + 1 + 1 + 1 + 21 = 210。それ以外の命令（Pmovmskb、Psrldq など）はこの閉包ではレジスタ形だけが出る。MMX 形（`mm` レジスタ）は 0 件。**実装前に FLEngine で実装済みの形と、DLL 側の未対応の形が重なるものは無い**（§1 の 2）。

### 3.2 CPUID が名乗らない命令（SSE3 以降。実装しなかった。決定 0075 §5）

| ニーモニック | オペランド形 | プレフィックス | 箇所 | 世代（SDM の CPUID Feature Flag 列） |
|---|---|---|---:|---|
| Lddqu | xmm, m128 | F2 | 196 | SSE3 |
| Movddup | xmm, m64 | F2 | 135 | SSE3 |
| Movddup | xmm, xmm | F2 | 35 | SSE3 |
| Addsubps | xmm, xmm | F2 | 78 | SSE3 |
| Addsubpd | xmm, xmm | 66 | 76 | SSE3 |
| Movshdup | xmm, xmm / m128 | F3 | 46 + 32 | SSE3 |
| Movsldup | xmm, xmm / m128 | F3 | 46 + 32 | SSE3 |
| Haddpd | xmm, xmm | 66 | 2 | SSE3 |
| **小計** | | | **678** | 7 ニーモニック |

（Movddup は 135 + 35 = 170、Movshdup と Movsldup は各 46 + 32 = 78。）符号化と世代（SDM の CPUID Feature Flag 列）は felixcloutier の転載（非公式）で読んだ。

注: プレフィックスは DLL のバイト列の先頭の 66 / F2 / F3 を読んだ実測（`w8_unsup_forms.py`。最初の版は capstone の `prefix` 欄を使い、F2/F3 を落として `none` と出したので、バイト列を直接読む形に直して再集計した）。
その他（SSE 系ではない。27 箇所）: `Int3` 21（パディングまたは致命エラー）、`Int` 5（`int 0x29`。致命エラー）、`Fxsave m64` 1（FXSAVE。決定 0075 §5）。`Mov r64, m64` 3 箇所は `gs:` のオフセット（W7 の範囲）で、レガシー命令の 4,074 に含まれない。

### 3.3 レガシーではない未対応（参考。この作業の対象外）

| 理由 | 箇所 |
|---|---:|
| 間接 jmp の目標が未解決（`Jmp m64`。`jmp [表 + 添字*8]`） | 464 + 6（表では `Jmp m64` 470） |
| IAT 経由の間接 call / jmp（`Call m64` 116 を含む） | 122 |
| `V*`（VEX / EVEX） | 275 |
| `gs:` のオフセットが未モデル | 3 |

## 4. `V*` が残る 275 箇所（参考）

実装の対象外（CPUID が AVX を名乗らない。決定 0047 の XGETBV の条件）。添字 0 の閉包に残る VEX 命令。主なもの: Vmulsd 40、Vfmadd213sd 39、Vsubsd 26、Vaddsd 19、Vmovdqu 19、Vmovq 19、Vmovsd 15、Vmovapd 14、Vfmadd231sd 12、Vmovdqa 13、Vmovntdq 8、Vfnmadd231sd 6 ほか（全 36 キー。`lift/out-w8/base/forms-idx0.tsv`）。

## 5. 全体の閉包（`--pdata`）での残り（実測。SSE3 以降を含む）

実装後の全体の閉包（`merged-text.dll` の 32,162 関数）で、レガシー（非 V）の未対応の上位: Movddup 9,167、Addsubpd 4,916、Lddqu 3,760、Pshufb 3,573（SSSE3）、Insertps 1,489（SSE4.1）、Pmovzxbd 1,413（SSE4.1）、Shlx 1,239（BMI2）、Haddpd 1,177、Palignr 1,174（SSSE3）、… 実装した 25 ニーモニックの分はどれも 0 になった（PEXTRW の `66 0F 3A 15`（SSE4.1）形が全体の閉包に 1 箇所あり、「SSE4.1 のため対象外」の理由付きで残る）。

## 6. 作ったファイル（`lift/out-w8/`）

- `w8_unsup_forms.py`: 生成 C の `fle_unsupported` を、ニーモニック・形・プレフィックス別に集計する（読み取り専用）。
- `base/`: 実装前の生成物（`ipp-idx0.c`、`lift-idx0.txt`、`forms-idx0.tsv`、`forms-merged.tsv`、`lift.txt`）。`after/`: 実装後。`test-all.txt`: 全体テストの出力。
- `probe_align.c`、`probe_align.txt`: ホストでのアライメント測定。`rsqrt_probe.c`、`rsqrt_special.c`、`rsqrt_dump.txt`、`rsqrt_exhaustive`: RSQRTPS の測定。`spec_fetch.py`、`spec/`: SDM の転載の取得（非公式）。
