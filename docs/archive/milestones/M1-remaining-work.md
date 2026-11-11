# M1 残作業の計画（FLEngine_x64.dll の Rust 製 lift）

> **履歴。現在の計画は [`roadmap-to-goal.md`](roadmap-to-goal.md)**（決定 0314）。

作成日: 2026-10-20

本書は計画である。実装はしていない。数値は次の 3 種に分けて書く。

- **実測**: このリポジトリ内のファイルを読むか、コマンドで数えた値。出典を併記する。
- **推測**: 作業量や所要時間の見積もり。根拠が無いものは推測と書く。
- **既知の規則**: Intel SDM など外部仕様の記述。出典を併記する。

## 0. 前提と結論

- 「本日中にフル lift を達成する」は、本計画の結果としても達成できない。`docs/progress.md` が既に同じ判断を書いている。本計画は残作業の分割と、次に着手すべき順序を示すためのものである。
- 閉包の到達範囲は実測で関数 6,956、命令 459,902 である。間接 call 6,482 件を辿っていないため、これは下界である（`docs/investigations/closure-count.md`）。
- 生成 C のうち未対応は 57,408 件（実測、`lift/out/lift-report.txt`）。本書の単位は、この 57,408 件と、フラグ未モデルによる abort の範囲を覆う。

## 1. 実測の基礎値

| 項目 | 値 | 出典 |
|---|---|---|
| 閉包の関数 / 基本ブロック / 命令 | 6,956 / 67,310 / 459,902 | `lift/closure-run1.txt` |
| 生成命令 / 未対応 / 生成 C 行数 | 459,902 / 57,408（ニーモニック 145 種） / 494,688 行 | `lift/out/lift-report.txt` |
| jcc（条件分岐） | 32,262（すべて abort する stub） | `closure-run1.txt`、`fe-lift/src/lib.rs` の `fle_jcc_unmodelled` |
| フラグ marker を出す命令（cmp / test / add / sub / xor） | 18,524 / 13,913 / 12,083 / 15,003 / 9,425（合計 68,948） | `lift-report.txt` の handled 行 |
| 間接 call（未対応） | 6,482 | `lift-report.txt`（`Call: indirect or far call`） |
| 間接 jmp（未対応） | 418（IAT 経由 402、その他 16。閉包の計測では IAT 経由は名前で解決できる扱い。生成側は両方とも未対応） | `lift-report.txt`、`closure-count.md` |
| x87（閉包内） | 6 件（Fldcw 4、Fnclex 2） | `closure-run1.txt` |

### 未対応 57,408 件の分類（実測件数の集計、分類は手作業）

| 分類 | 件数 | 含むニーモニック（主なもの） |
|---|---|---|
| 整数 ALU・その他 | 27,334 | Movzx 6,813、Nop 6,533、Movsxd 5,127、And 1,800、Shl 1,559、Or 1,421、Imul 1,337、Shr 1,121、Neg 300、Sar 296、Not 205、Idiv 156 ほか |
| SSE/AVX/MMX（スカラー浮動小数） | 16,780 | Movss 3,220、Movsd 2,527、Cvtss2sd 1,901、Mulsd 1,373、Cvtsi2sd 1,223、Mulss 1,180、Addsd 1,027、Cvtsd2ss 906 ほか |
| SSE/AVX/MMX（パック・SIMD・その他） | 3,658 | Movdqa 1,006、Movapd 854、Movaps 687、Pand 278、Pmulhuw 18 ほか |
| 制御・setcc・cmov | 8,521 | 間接 call 6,482、間接 jmp 418、Setne 739 ほか setcc 計 1,617、cmov 4 |
| 文字列 | 1,088 | Movsq 861、Stosq 207、Stosd 17 ほか |
| x87 | 6 | Fldcw 4、Fnclex 2（閉包内） |
| その他 | 21 | Lea（32 ビットアドレス）13、Mov（high-byte 等）6 |
| system/fence | 7 | Pause 3、Sfence 2、Vzeroupper 2 |

- 合計は 57,408 件で一致することを確認した（実測）。
- SSE の内訳は、分類の境界に推測を含む。合計は一致する。

### 線形走査との関係（参考、閉包の数ではない）

- `lift/objdump-text.txt` は .text 全体の線形逆アセンブル。データ領域も含むため、x87 は 7,779 件（`fs:` 接頭辞を除く）と数えられる。閉包の x87 は 6 件である。線形の値を閉包の件数として使ってはならない。
- 前回報告の x87「2,193 件」は、今回の閉包では再現しなかった。閉包外（間接 call の先）を含めて初めて評価できる。詳細は単位 U-F。

## 2. 最優先: フラグ（EFLAGS）のモデル化（単位 U-A）

### 2.1 理由

- jcc は 32,262 件あり、現状は `fle_jcc_unmodelled()` で abort する。フラグが無いと、閉包の制御フローを 1 つも実行できない。
- cmp・test・add・sub・xor の 68,948 件は、現状フラグを更新せず marker を出すだけである。このままでは jcc が読む値が無い。
- setcc（1,617 件）、cmov（4 件）、adc/sbb（48 件）、INC/DEC、シフト、乗除算も、フラグに依存するか、フラグを更新する。

### 2.2 設計（C での表現）

採用案は **eager（即時）更新** である。各命令の後で 6 個の状態ビットを直接更新する。

```c
typedef struct { uint8_t cf, pf, af, zf, sf, of; } fle_eflags; /* 各 0 or 1 */
extern fle_eflags FL;
```

- 理由: SDM の規則を 1 命令ずつ直接写せるので、検査しやすい。状態は 6 バイトで、性能は本段階の目標ではない（生成 C は既に 38 MB ある）。
- lazy（遅延）方式（演算種別・幅・オペランドを保存し、参照時に計算）は、性能が問題になった時点で検討する。今は採用しない。
- 幅（8/16/32/64）ごとに符号ビット位置と mask を持つ。関数 `fle_set_add(w, a, b, res)` のように、演算種別ごとのヘルパを `fle_flags.h`（新規、`lift/runtime/` 配下）に置く案。

### 2.3 ALU 命令ごとの更新規則（既知の規則）

出典: Intel 64 and IA-32 Architectures Software Developer's Manual（SDM）、Vol. 1 §3.4.3.1（ステータスフラグ: CF bit 0、PF bit 2、AF bit 4、ZF bit 6、SF bit 7、OF bit 11）、および Vol. 2 の各命令の "Flags Affected" 節。

| 命令 | CF | PF | AF | ZF | SF | OF | 備考 |
|---|---|---|---|---|---|---|---|
| ADD | carry out（res < a、幅で mask） | res の下位 8 bit の偶奇 | (a^b^res) の bit 4 | res==0 | 符号ビット | ((a^res)&(b^res)) の符号ビット | SDM Vol.2 ADD |
| ADC | carry in を含む | 同上 | 同上 | 同上 | 同上 | 同上 | 中間値は 128 bit で判定する |
| SUB / CMP | borrow（a < b、符号なし） | 同上 | (a^b^res) の bit 4 | res==0 | 符号ビット | ((a^b)&(a^res)) の符号ビット | SDM Vol.2 SUB、CMP |
| SBB | borrow in を含む | 同上 | 同上 | 同上 | 同上 | 同上 | |
| AND / OR / XOR / TEST | 0 | res から | 未定義 | res から | res から | 0 | SDM Vol.2 AND、OR、XOR、TEST |
| INC / DEC | **変更しない** | res から | 演算として | res から | res から | 演算として | CF は直前の値を保持する |
| NEG | (a != 0) | res から | 0 - a として | res から | res から | a が符号ビットのみのとき 1 | |
| NOT | 変更しない | | | | | | フラグを変えない |
| SHL / SHR / SAR | 最後に押し出されたビット | res から | 未定義 | res から | res から | count==1 のときだけ定義 | count は 5 bit（64 bit では 6 bit）で mask。count==0 なら全フラグ変更しない |
| IMUL（2・3 オペランド） | 切り捨て結果が全積と一致しないとき 1 | 未定義 | 未定義 | 未定義 | 未定義 | CF と同じ | SDM Vol.2 IMUL |
| MUL / IMUL（1 オペランド） | 上位半分が 0 でないとき 1 | 未定義 | 未定義 | 未定義 | 未定義 | CF と同じ | |
| DIV / IDIV | 全て未定義 | | | | | | 後続で読まれないことを要求する |
| BT | 選択ビット | 未定義 | 未定義 | 未定義 | 未定義 | 未定義 | |
| UCOMISS / UCOMISD / COMISD | 順序付け結果による | 0（無順序なら 1） | 0 | 順序付け結果による | 0 | 0 | 無順序（NaN）のときは ZF=PF=CF=1。SDM Vol.2 UCOMISS |

- 「未定義」のフラグを後続の jcc が読むと、結果は CPU 実装依存になる。設計上は、未定義フラグを読んだ時点で `fle_*_undefined_read` を出して abort させる。これは検査用の仕様であり、黙って 0 にしてはならない。
- この 1 行の表は、U-A のテストの期待値を書く源になる（2.5 節）。

### 2.4 jcc と setcc・cmov の条件式

出典: SDM Vol. 2「Jcc—Jump if Condition Is Met」の条件表。

| 命令 | 条件式（C） |
|---|---|
| JO / JNO | OF / !OF |
| JB（JC, JNAE） | CF |
| JAE（JNB, JNC） | !CF |
| JE（JZ） / JNE（JNZ） | ZF / !ZF |
| JBE（JNA） | CF \|\| ZF |
| JA（JNBE） | !CF && !ZF |
| JS / JNS | SF / !SF |
| JP（JPE） / JNP（JPO） | PF / !PF |
| JL（JNGE） | SF != OF |
| JGE（JNL） | SF == OF |
| JLE（JNG） | ZF \|\| (SF != OF) |
| JG（JNLE） | !ZF && (SF == OF) |

- setcc と cmov は、上の条件式をそのまま使う（SDM Vol. 2 SETcc、CMOVcc）。
- 閉包に出るニーモニックのみを実装すれば足りる（Je、Jne、Jg、Jle、Jge、Jl、Jbe、Ja、Jb、Jp、Jae、Jns、Js。実測）。ただし表全体を実装し、テストで検証する。

### 2.5 検証の方法（推測を含む）

- このマシンは x86_64 Linux（実測: `uname -m` の結果は x86_64）で、clang 21.1.8 がある（実測）。
- フラグの規則は ABI に依存しない。よって、同じ命令をホストで実行し、`pushfq` で EFLAGS を読んで、生成した C 関数の結果と突き合わせられる（推測: 実行可能と見込むが、未検証）。
- テスト対象は、幅（8/16/32/64）、境界値（0、1、符号ビット、最大値）、乱数値。ホストと比較するのは、2.3 節の表の「定義されている」フラグだけ。未定義フラグは比較しない。
- Rust 側のテストは、`fe-lift` に対して cargo test で追加する（推測: 件数は後述の見積りに従う）。

### 2.6 U-A の完了条件

- 生成 C に `fle_flags_unmodelled` と `fle_jcc_unmodelled` が現れない（grep で 0 件）。実測の現状は cmp 18,524 等の marker と jcc 32,262 件の stub。
- ホスト実機との突合テストが、2.3 節の表の全命令・全幅で成功する。件数は推測で 200 件程度（作業量の見積りは 5.2 節）。
- `cargo test --offline` が全件成功する。現状の fe-pe は 28 件。fe-lift は 0 件（実測）。

## 3. 単位の一覧

各単位は、依存が解決されれば独立したサブエージェントに渡せる。

| 単位 | 名前 | 対象（件数は実測） | 触るファイル | 依存 | 完了条件 |
|---|---|---|---|---|---|
| U-A | フラグ（EFLAGS）モデル | jcc 32,262、setcc 1,617、cmov 4、adc/sbb 48、フラグ marker を出す 68,948 命令の更新規則 | `lift/crates/fe-lift/src/lib.rs`、`lift/runtime/fle_flags.h`（新規）、ホスト突合テスト（新規） | なし（最優先） | 2.6 節 |
| U-B | 整数命令の残り | movzx 6,813、movsx 218、movsxd 5,127、nop 6,533、and 1,800、or 1,421、shl 1,559、shr 1,121、sar 296、imul 1,337、mul 17、div 16、idiv 156、neg 300、not 205、cdq 128、cqo 25、inc 6、dec 42、bt 系 59、bsf 5、xchg 10、xadd 56、cmpxchg 27、rol 4、ror 3、jecxz 2 ほか | `fe-lift/src/lib.rs`、`fle_runtime.h` | flag 依存のものは U-A。movzx・movsx・nop・not は U-A に依存しない | 未対応の整数 ALU 分類（27,334 件のうち閉包内の該当分）が減る。各ハンドラに対しホスト突合テスト |
| U-C | SSE スカラー浮動小数 | movss 3,220、movsd 2,527、cvt 系 4,700、mul/add/sub/div の sd・ss 5,362、ucomi/comi 961（上記 4 群の合計 16,770。残り 10 は max/min/sqrt/and/or/xor の sd・ss、計 16,780 に含む） | | `fe-lift/src/lib.rs`、`fle_runtime.h`（XMM 16 本、MXCSR の追加） | ucomi/comi のフラグは U-A。XMM レジスタの型は本単位で定義 | 未対応の SSE スカラー分類（16,780 件）が減る。丸めモードの突合テスト |
| U-D | SSE パック・SIMD・MMX | movdqa 1,006、movapd 854、movaps 687、pand 278、パック・シャッフル系 計 3,658 | 同上 | U-C の XMM 型 | 未対応のパック分類（3,658 件）が減る |
| U-E | 間接 call / jmp の宛先解決 | 間接 call 6,482、間接 jmp 418（うち IAT 402 は閉包で扱い済み、残り 16） | `fe-decode`（閉包計測）、`fe-lift`（宛先の表） | なし。U-A〜D とは独立に進められる | 閉包の未解決数が減る。閉包の関数数・命令数を再計測し、`closure-count.md` を更新 |
| U-F | x87 | 閉包内 6（Fldcw 4、Fnclex 2）。前回報告の 2,193 件は閉包内では未確認 | `fe-lift`、`fle_runtime.h` | U-E の再計測が先（閉包が広がれば件数が変わる） | U-E 後の閉包で x87 件数を計測し、件数を本書に記録する |
| U-G | 文字列命令・系統命令 | movsq 861、stosq 207、stosd 17、movsb・stosb・cld 計 3、pause・sfence・emms 等 計 15 前後、lea（32 bit）13 | `fe-lift`、`fle_runtime.h` | 移動系は U-A に依存しない。rep 接頭辞の扱いを要確認 | 未対応の文字列分類（1,088 件）が減る |
| U-H | boot harness | — | `lift/`、新規 | U-A〜U-D の完了 | DLL の初期化を走らせて終了する（推測: 先行単位の完了後に評価） |

- U-E と U-A は、互いに待たずに並行できる。
- U-C と U-D は、U-A の完了を待たずにハンドラを書けるが、ucomi 系のフラグは U-A に依存する。
- 本計画で扱わないのは、ネイティブ照合（Win64 上での実行結果との突合）と wasm 化である。`progress.md` の未着手に従い、本書の範囲外とする。

## 4. 依存関係

```
U-A (フラグ) ──→ U-B (flag 依存の整数命令) ──┐
            └──→ U-C (ucomi のフラグ) ──→ U-D (SIMD) ──┤
U-E (間接解決) ──→ 閉包の再計測 ──→ U-F (x87) ─────┤
U-G (文字列・系統) ───────────────────────────────┤
                                           └──→ U-H (boot harness)
```

- 関係の根拠: U-B・U-C のフラグ依存、U-C→U-D の XMM 型定義、U-E の結果で閉包が変わるので U-F の件数が決まること。

## 5. 作業量の見積り（推測）

### 5.1 注意

- 以下は推測である。実測ではない。一つの単位の所要時間を示す根拠は無い。
- 件数（テスト件数と命令件数）は、実測のニーモニック件数を基に、1 命令あたり 1 から 3 件のテストを置く仮定から出している（推測）。

### 5.2 単位ごとの作業量（推測）

| 単位 | 実装の目安（推測） | テストの目安（推測） | 備考 |
|---|---|---|---|
| U-A | 中（フラグヘルパ 10 種程度、jcc 12 条件、setcc・cmov） | 200 件程度 | 最大のリスク（後続の全単位が依存する） |
| U-B | 大（約 40 ニーモニック） | 150 件程度 | 件数は大きいが、1 命令ずつは小さい |
| U-C | 大（XMM 型、丸め、約 50 ニーモニック） | 150 件程度 | 浮動小数の丸めの突合が難しい |
| U-D | 大（約 70 ニーモニック、件数は少ない） | 100 件程度 | 頻度は低い |
| U-E | 中〜大（間接 call の宛先の解析方法を決める） | 閉包の再計測で検証 | 閉包を広げると他の単位の件数が増える可能性がある |
| U-F | 小〜中（件数次第） | 件数が 6 のままなら 20 件程度 | 閉包内が 6 件なので後回しでよい |
| U-G | 小 | 30 件程度 | |
| U-H | 大（DLL 初期化の環境） | 未定 | |

### 5.3 1 単位 1 サブエージェントの粒度

- 上の表の単位は、1 サブエージェントで完結できる粒度を目標とする（推測）。U-B と U-C は、ニーモニックの群ごとにさらに分けてよい。

## 6. 検証の基準

- 件数の基準: 未対応の件数（`lift/out/lift-report.txt` の `unsupported_total`）を、単位ごとに減らす。再生成は `fe-lift lift FILE OUT.c` で行う。
- 動作の基準: 単位ごとにホスト実機との突合テストを置く（推測: 2.5 節の方法）。
- 閉包の基準: U-E の後に `fe-decode closure ... --mnemonics` を再実行し、`closure-count.md` の値を更新する。

## 7. 既存の残置物（削除は禁止。報告のみ）

依頼文の記載によるもので、本計画では確認していない。`/tmp` を使わない制約のため、確認コマンドを実行していない。

- `/tmp/x`、`/tmp/y`（禁止違反で作成されたもの）
- `lift/objdump-text.txt`（依頼文は 233 MB、`closure-count.md` は 243 MB と記す。MiB と MB の表記差の可能性があり、実サイズは未確認。`.gitignore` 対象外なので注意）。
- `tools/__pycache__/`（`.gitignore` 対象外）

## 8. 本計画の範囲外

- 削除、git 操作、2 GB 級の中間ファイル作成は、本計画の作業で行っていない。
- `docs/progress.md` の既存の記述は変更していない（本書へのリンクを 1 行追記のみ）。

## 9. 追記（2026-10-21）: 受け入れ基準 0046 と現在の優先課題

本書 §0〜§8 は 2026-10-20 の最初の計画で、数値（未対応 57,408 件、jcc の stub など）は当時のものである。その後 U-A〜U-G は実装され、all-pdata の `unsupported_total` は **601**（2026-10-21、HEAD 31d0134 の生成で実測）になった。現在の判定は次の文書を基準にする。

- 受け入れ基準 A1〜A6 と現在の状態: [`docs/decisions/0046-m1-acceptance.md`](../decisions/0046-m1-acceptance.md)。M1 完了 = A2 + A4 + A5 + A6。現状は A1 達成、A3 は定義の範囲で達成、A2、A4 は未達、A5、A6 は部分。

### 現在の優先課題（推測を含む。順序は PM が決める）

1. **停止点の先（A4）**: 停止は `GetLocaleInfoW` LCType 0x1c（INEGCURR）。InitTable の runnable 895 件のうち index 17（proc 0x455790）を実行中。この先に `GetDateFormatW`、`EnumCalendarInfoW` などが続く見込み（`locale-enus-run.md` §5）。unit ごとの実行回数を数える harness の計数器が無いので、A4 の判定のために先に足す。
2. **ロケール 6 種の出典（A4）**: 0xc（SLIST）、0x1c（INEGCURR）、0x1f（SSHORTDATE）、0x20（SLONGDATE）、0x1003（STIMEFORMAT）、0x79（SSHORTTIME）は、Wine と CLDR で値が食い違い、abort のまま（0042）。実機 Windows の値か Microsoft の公式資料が要る。決まらない間は harness がここで止まる。あわせて、0040、0041、0042 の仮の値（非公式の出典）を実機で確認する。
3. **IAT thunk の応答（A2b）**: 応答の無い IAT thunk が 557（536 API、23 DLL。user32 221、gdi32 129、kernel32 58、comctl32 31、shell32 24、ole32 19 ほか）。実行で踏まれるものから、仕様確認済みの応答を足す。ゲスト関数の呼び返し機構（W10）と仮想 DC の設計が先に要る。遅延 import の 27 API（9 DLL）は、仮想モジュール表に無い DLL で、外部モジュール（M2）の判断が先。
4. **命令と制御フローの残り 44 か所（A2a）**: Psubw 10、Pcmpgtd 8、Rcpps 8、Bswap 7、Test high-byte 7、And high-byte 1、Andnpd 1、Xgetbv 1、`jmp [rcx]`（f_4126f0、W3）1。
5. **A5 の拡大**: 純計算 9,901 件のうち 1,000 件だけを比較している。import や間接 call を持つ関数は対象外（選別から外れた 47,915 件）。native_fault 15.4% の削減、ネガコンの検出率の向上（run5 は 123/203 = 60.6%）、0042〜0044 の後の再実行、x87 の超越関数の Intel 実機との比較が課題。
6. **wasm の残課題（A3）**: `fle_x87.c` の `long` を `int64_t` にする（fscale の native と wasm の不一致の原因。別担当の編集中だった）、`fle_ldx` 系と runtime 内の直接変換を上位ビット検査に通す、停止点より先での native と wasm の一致確認、ブラウザ対応（`NODERAWFS` を使わない構成）、`-O1` 以上でのリンクと実行、ページ保護の扱い、アリーナ 512 MiB の上限の確認。

### 残置物と更新が必要な文書（報告のみ）

- `docs/progress.md` は古く（「lift 本体は未着手」と書く）、現状を反映していない。更新は PM が判断する。
- README.md に ADR の索引は無い。`docs/decisions/` には 0036 が 2 本ある（`0036-a5-oracle-v2.md`、`0036-indirect-jmp.md`）。

## 10. 2026-10-22 の現状

§0〜§9 は書いた時点の記録として残す。本節は、§9 の「現在の優先課題」と、`docs/investigations/unsupported-inventory-pdata.md` の実装単位 W0〜W12（2026-10-20 の提案）を、HEAD 92b9e50（2026-10-22）の実測に合わせて整理し直したもの。**判定基準は 0046 のまま**（`decisions/0046-m1-acceptance.md` §9 に現在の判定）。数は実測のあるものだけ書く。実測でないものは「推測」と書く。

### 10.1 実測の基礎値（2026-10-22）

| 項目 | 値 | 出典 |
|---|---|---|
| `unsupported_total` | **534**（IAT 経由 jmp 533 + 解決不能な間接 jmp 1）。命令 stub は 0 | `lift/out-cl/gen/gen-pdata.txt`（2026-10-22 15:21 生成）。生成 C の `fle_unsupported` 行も 534（`lift/out-cl/out/wasm-pdata-part-*.c`） |
| 応答の無い IAT thunk | **533 thunk、514 API、23 DLL** | `lift/out-doc/remaining-iat-by-dll.txt`（生成 C の 533 行を、DLL の `jmp [rip+rel32]` と `lift/out-imports/imports.txt` で DLL と API に引いた。スクリプト `remaining-iat-by-dll.py`） |
| 生成 C | 関数 59,297、命令 2,528,830、C 2,885,304 行、`bad_nodes` 0 | 同 `gen-pdata.txt` |
| import 応答表（TABLE） | 224 行 | `lift/crates/fe-lift/src/imports_table.rs`（DLL 別: kernel32 140、user32 25、oleaut32 18、advapi32 12、winmm 11、ole32 10、gdi32 6、msctf 1、imagehlp 1） |
| 遅延 import の `jmp reg`（tail dispatch） | 57 occurrence（20 か所）、`reg_dispatch` 57、表 jmp 3、解決不能 0 | `gen-pdata.txt` の `register_jmps` |
| A4 | init_count 115 / 942、completed 110 / 895、停止は CreateWindowExW（thunk 0x425780） | `lift/out-cl/a4/20261022T101528Z-native.txt` |
| テスト | 84 バイナリ、958 件成功、失敗 0 | `lift/out-cl/test-ws.txt`（2026-10-22 20:19） |

0046（2026-10-21）の時点との差: `unsupported_total` 601 → 534（−67）、そのうち命令 43 → 0、IAT 経由 jmp 557 → 533（−24）。DLL ごと: user32 221 → 207、gdi32 129 → 123、kernel32 58 → 54（thunk 数）。

### 10.2 W0〜W12 の現状（unsupported-inventory-pdata.md §6 の提案に対して）

「API 数」は、`remaining-iat-by-dll.txt` に残る API のうち、§6 が各単位に挙げた名前に一致するものの数（名前による分類。§6 の分類と同じく**推測を含む**。境界は動く）。

| 単位 | 提案の内容 | 現状 | 残り（実測。名前の一致） |
|---|---|---|---|
| W0 | TABLE を単位ごとのファイルに分ける | 提案の形では実装されていない（`imports_table.rs` は 1 ファイルのまま、224 行）。構造の変更が行われたかは確認していない | — |
| W1 | `jmp reg` の tail dispatch | **実装済み**（決定 0036 `indirect-jmp`）。57 occurrence が `reg_dispatch` | 0 |
| W2 | 表を引く jmp（jump table） | **実装済み**（0036 `indirect-jmp`。f_40d210 と switch 3 か所）。表 jmp 3 | 0 |
| W3 | setjmp/longjmp 風（`f_4126f0` の `jmp [rcx]`） | **未実装**。解決不能な間接 jmp として 1 か所 | 1 か所（0x412752）。実際に踏まれていない |
| W4 | oleaut32／ole32 の BSTR、Variant、SafeArray、CoTaskMem、CoInitialize | **ほぼ実装済み**（決定 0037） | oleaut32 `GetActiveObject` 1（W4 の列挙外） |
| W5 | kernel32 の純粋・状態表なし（FormatMessageW ほか） | **IAT 経由で挙げられた 15 名はすべて未実装**（§6 は「16」と書くが、うち GlobalMemoryStatus は §6 の時点で対応済み。遅延 import の 5 名は本表の対象外）。GetComputerNameW 系などは未対応のまま。W5 の周辺の別 API は 0020、0034 などで実装された | 15 API: CompareFileTime、CompareStringA、EnumCalendarInfoW\*、EnumSystemLocalesW\*、FormatMessageW、FreeResource、GetCPInfoExW、GetComputerNameExW、GetComputerNameW、GetConsoleCP、GetConsoleOutputCP、GetOEMCP、GetUserDefaultLCID、IsDebuggerPresent、TzSpecificLocalTimeToSystemTime（\* はゲストのコールバックを呼ぶ。コールバック機構は 0064 で入った） |
| W6 | kernel32 の同期・スレッド・プロセス・ファイル状態 | **イベントだけ実装**（0050: CreateEventW、SetEvent、ResetEvent、WaitForSingleObject、CloseHandle フック。単一スレッドの状態機械。スレッド生成は名前付き abort の方針）。スレッド、待機、ファイル状態、タイマーキューは未実装 | kernel32 のうち W5 以外の 36 API: CreateThread、ResumeThread、SuspendThread、ExitThread、GetExitCodeThread、GetThreadPriority、WaitForMultipleObjects(Ex)、SignalObjectAndWait、HeapCreate、HeapDestroy、TlsFree、GetFileType、SetEndOfFile、GetFileInformationByHandle、GetDiskFreeSpaceW、GetTempFileNameW、Get/WritePrivateProfileStringW、タイマーキュー 3、OpenProcess、CreateProcessW、AddVectoredExceptionHandler、RtlUnwindEx、UnhandledExceptionFilter ほか |
| W7 | user32 の「ウィンドウ無し」環境値と表示 | **一部実装**（0057 GetSystemMetrics 14 index、0058 SPI 3 種、0059 カーソル、0063 GetKeyboardLayout、0065 モニタ。**ほぼすべて仮の決定**） | 23 API（名前の一致）: GetSysColor、GetDoubleClickTime、GetMessageExtraInfo、MessageBeep、MapVirtualKeyW、GetKeyNameTextW、GetKeyboardLayoutList、GetKeyboardLayoutNameW、EnumDisplaySettingsW、MonitorFromPoint/Rect/Window、GetCursorPos、GetCursorInfo、GetClipCursor、GetDesktopWindow、EqualRect、PtInRect、InflateRect、SubtractRect、GetGuiResources、CharNextExA、CharToOemBuffW |
| W8 | 「デバイス／サービス無し」型 | **未実装**（環境の主張の決定が要る） | 61 API（wininet 15、msacm32 14、winmm 13、crypt32 7、winspool 5、mpr 4、dsound 2、wintrust 1） |
| W9 | shell32／version／shfolder／imagehlp／shlwapi／crt | **未実装** | 30 API（shell32 22、version 3、imagehlp 2、shfolder 1、shlwapi 1、crt `memset` 1）。imagehlp の ImageNtHeader は 0029 で実装済み（TABLE に 1 行あり） |
| W10 | user32／gdi32／comctl32 の実体と、ゲスト関数の呼び返し機構 | **呼び返し機構は実装済み**（0064 `fle_call_guest`）。仮想 DC（0053）、GDI オブジェクト表（0054。パレット、stock 3 種）、フォント（0066、0068）、カーソル・画像（0059、0061）、ウィンドウクラス表（0070）は**入口だけ**。ウィンドウ、メッセージ、DC 描画は**未実装**。ウィンドウ系の設計メモは `docs/investigations/window-design.md`（別の作業者が作成中。本節は未確認） | user32 174 API（W7 の名前を除く）、gdi32 120、comctl32 30、opengl32 2。**現在の停止点 CreateWindowExW を含む** |
| W11 | COM/OLE | **未実装**（W10 の呼び返しは入った） | ole32 19 API（CoCreateInstance、CoGetClassObject、CreateStreamOnHGlobal、RegisterDragDrop、OleSetClipboard、ReleaseStgMedium ほか） |
| W12 | 外部 DLL（遅延 import 27 API、9 DLL） | 状況は**再計測していない**。msctf.dll は 0055 で仮想モジュールになったが、遅延 import とは別の経路（LoadLibraryW）。M2 の判断が先 | 不明（再計測なし） |

- 合計の確認: 上の「残り」は、`remaining-iat-by-dll.txt` の 533 thunk の内訳と整合する（W5 の 15 + W6 の 36 は kernel32 の 51 API（54 thunk）、W7 の 23 + W10 の user32 174 は user32 197 API、など。名前の一致による分類で、thunk 数と API 数は異なる）。W1、W2 の実装後、IAT 経由 jmp 以外の `unsupported_total` は 1（W3）。
- §9 の優先課題 3「IAT thunk の応答（A2b）」は、**ゲスト関数の呼び返し機構（W10 の前提）と仮想 DC は入った**。しかし A2b の判定は「仕様確認済みの応答」で、W7、W10 の入口の応答はほぼ仮の決定である（§10.4）。

### 10.3 現在の次の壁（実測。停止点）

- 停止点は user32!CreateWindowExW（thunk 0x425780、`f_52a7f0` の 0x52a8be、`CreateWindowExW(WS_EX_TOOLWINDOW 0x80, L"TPUtilWindow", L"", WS_POPUP 0x80000000, 0,0,0,0, NULL, NULL, HInstance, NULL)`。`getclassinfo-caller.md` §3、§7）。0070 の実装範囲（ウィンドウクラス表）の外。
- ウィンドウの実体設計（ウィンドウ表、メッセージキュー、`DefWindowProcW`、`DispatchMessageW`、`SendMessageW`、`GetMessageW` など）は、W10 に属する。**設計の決定がまだ無い**（`decisions/` に 0071 以降は、本節を書いた時点で存在しない）。user32 の CreateWindowExW 以降の呼び出しは、**何個のウィンドウ系 API が同じ unit 114 の中で踏まれるかを測っていない**。
- init_count は 115 のまま（unit 114、GDI・入力・ウィンドウの初期化）。停止点が進んでも、A4 の指標は動いていない（0046 §9.5）。unit 114 を戻るまでに必要なウィンドウ系 API の数は、**測っていない**。

### 10.4 優先課題の更新（§9 の置き換えではなく、現在の見立て。推測を含む。順序は PM が決める）

1. **A4（停止点の先）**: 次の壁は CreateWindowExW。user32 のウィンドウ・メッセージ、gdi32 の DC・描画の実体設計が先に要る（W10。設計メモは別の作業者が作成中）。**設計の決定が出るまで、A4 の進捗の指標（init_count）は動かない見込み**（推測。unit 114 の内側で、完走まで未知の数のウィンドウ系 API がある）。
2. **仮の決定の確認（A2b、A4）**: 仮の決定 18 件が応答の根拠になっている（`docs/investigations/provisional-decisions.md`）。実機 Windows または公式資料で確認できるものを、一括で確認する手順を同ファイルの §2 にまとめた（計画。未実施）。確認しないまま完走しても、A4 の「仕様確認済みのみ」は満たさない可能性が高い（0046 §5、§9.5 の扱い）。
3. **A2a（残り 1 か所）**: `f_4126f0` の `jmp [rcx]`（W3）。Xgetbv の除外が A2a の定義に合うかは PM が決める事項（未決。0046 §9.3）。
4. **A2b の厚み**: 533 thunk（W5〜W12）。実行で踏まれる数を測るのが先（静的閉包は過大見積もり）。A4 の停止点で見つかるものから順に、が現在の運用。
5. **A5**: 再実行していない（run5 のまま）。0042〜0049 以降の変更の後で再実行が要る。
6. **A3 の残課題**: ブラウザ対応（`NODERAWFS` を使わない構成）、`-O1` 以上、ページ保護、アリーナ 512 MiB の上限（0045、0049）。
7. **A6**: push は remote 未設定のため保留。

### 10.5 この節の限界

- 名前による分類（W4〜W11 の「残り」）は、§6 の列挙と `remaining-iat-by-dll.txt` の名前を突き合わせたもの。§6 自体が「API 名からの判断」（推測）で、境界（特に W7 と W10 の user32）は動く。
- 生成 C の統計（534、533 thunk）は 2026-10-22 15:21 の生成。0069、0070 の実装を含むかは `gen-pdata.txt` の `jmp_imp:GetClassInfoW`、`jmp_imp:RegisterClassW` の行から含むと読んだ。HEAD 92b9e50 の後に再生成していない。
- 作業量・時期の見積もりは書いていない（計測の根拠が無い）。
